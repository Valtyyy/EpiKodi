# ADR 0003 : Injection de dépendances par constructeur et racine de composition unique

- **Statut :** Accepté
- **Date :** 2026-11-11
- **Références :** `ARCHITECTURE.md` §1 (C4, C5), §3 (« Injection de dépendances »), §9 (`SettingsView`), §13, §14 (ADR-003)

## Contexte

Le cœur métier doit se tester sans réseau, sans Tauri, sans GTK et sans libmpv (C5). La même logique doit être accessible depuis deux transports : les commandes Tauri pour la télé, et l'API HTTP axum pour la télécommande, qui doit être branchée sur les mêmes instances de services (C4).

Tauri résout un service par son type, au moyen de `State<'_, T>`. Deux points demandent une attention particulière, tirés de `ARCHITECTURE.md` §3 :

- un `State` non enregistré n'est détecté qu'à l'appel de la commande, donc à l'exécution ;
- la résolution se fait par type : deux valeurs du même type ne peuvent pas cohabiter.

Enfin, certains services doivent réagir à d'autres sans en dépendre, et le graphe de dépendances doit rester acyclique.

## Décision

1. **Injection par constructeur.** Un service reçoit ses dépendances dans `new()`. Il ne connaît ni `AppHandle` ni `State`.

2. **Services concrets, ports en `trait`.**
   - Un service est une `struct` concrète, annotée `#[derive(Clone)]`, dont les champs sont des `Arc<dyn Port>`. Il n'y a pas de trait par service : on ne remplace que les bords du système.
   - Un port est un `trait`, avec `async-trait` lorsqu'il est utilisé en `dyn` et possède des méthodes `async`. Les ports sont listés en §3 : `MediaRepository`, `PlayerEngine`, `SourceProvider`, `SecretStore`, `Clock`, etc.

   ```rust
   #[derive(Clone)]
   pub struct LibraryService {
       media: Arc<dyn MediaRepository>,
       progress: Arc<dyn ProgressRepository>,
       events: EventBus,
   }
   ```

3. **Un service clonable, partagé.** Parce que le clone est peu coûteux, la même instance logique est partagée entre les commandes Tauri, les handlers axum et les tâches de fond. Un état mutable reste privé au service (`Arc<RwLock<_>>`, `watch`). Aucune signature de commande ne contient de `Mutex` ni d'`Arc`.

4. **`State` s'arrête à la frontière.** Seules les commandes Tauri résolvent un service par `State<'_, T>`, et les handlers axum par l'extracteur `State<T>`. Une commande ne fait que traduire : elle n'a aucune logique. Toutes les commandes renvoient `Result<_, AppError>`.

5. **Une seule racine de composition.** `src-tauri/src/bootstrap.rs` est le seul fichier qui connaît à la fois les services et leurs implémentations concrètes. La fonction `register` déstructure `Services` **sans `..`** et enregistre chaque champ. Ajouter un service sans l'enregistrer fait échouer la compilation.

6. **Test de démarrage.** Un test appelle `try_state` pour chaque service, afin de détecter à l'exécution, dans la CI, un `State` manquant avant qu'une commande ne l'appelle. *Ce test est à écrire : rien n'est encore implémenté.*

7. **Un type par valeur gérée.** Chaque service a son propre type. On n'enregistre jamais directement un `Arc<dyn Port>` dans Tauri : les ports restent internes aux services.

8. **Graphe acyclique.** La construction dans `bootstrap.rs` empêche d'écrire un cycle. Lorsqu'un service doit réagir à un autre sans en dépendre, il passe par l'`EventBus`. Exemple : la progression et l'état de lecture sont diffusés à l'interface par le port `UiBridge`. Les services qui ont besoin des réglages reçoivent une `SettingsView` en lecture seule, et non le `SettingsService` (§9). Une vue ne permet ni d'écrire ni de former un cycle.

9. **Ordre de démarrage.** La fenêtre est créée avec `create: false`, puis construite dans `setup`, **après** `register`. Une commande ne peut donc pas être appelée avant que ses services soient enregistrés.

## Alternatives écartées

- **Conteneur d'injection de dépendances ou framework DI.** Écarté : un conteneur déplace la vérification du graphe à l'exécution. La déstructuration exhaustive de `Services` fait au contraire échouer la compilation en cas d'oubli, ce qui correspond à l'objectif du §3. Le document privilégie un câblage explicite, lisible dans un seul fichier.
- **Un trait par service.** Écarté : le document indique qu'un trait par service n'apporterait rien, puisque les services ne sont pas remplacés en test. Seuls les ports, aux bords du système, sont substitués par des faux en mémoire.
- **Singletons globaux (`static`, `lazy`).** Écarté : un état global rend les tests dépendants les uns des autres et ne permet pas d'injecter des faux pour les ports (§3, colonne « En test »). Il rend aussi le graphe invisible.
- **Passer `AppHandle` ou `State` dans un service.** Écarté : la règle n° 1 l'interdit, et un service qui dépend de Tauri ne se teste plus sans lui, ce qui viole C5.
- **Un `Mutex` ou un `Arc` dans la signature des commandes.** Écarté : le verrouillage relève du service, qui le garde privé. La signature d'une commande reste une simple traduction.
- **Enregistrer directement un `Arc<dyn Port>` dans Tauri.** Écarté : la résolution se fait par type, donc deux ports de même type entreraient en collision (§3, pièges).
- **Localisation de service à la demande (service locator).** Écarté pour la même raison que le conteneur : les dépendances d'un service ne sont plus visibles dans sa signature, et le graphe ne se vérifie plus à la compilation. *Raison déduite des principes ci-dessus.*

## Conséquences

### Positives

- **Tests sans réseau.** Les tests utilisent le vrai service et des ports en mémoire : SQLite `:memory:`, `PlayerEngine` scripté, `Clock` manuelle, `SourceProvider` avec pannes programmables (§3, §13).
- **Un état unique.** Le scan, la lecture et la télécommande partagent les mêmes instances. Une action depuis le téléphone se répercute à l'écran sans code particulier (§8).
- **Graphe vérifié à la compilation.** Un champ oublié dans `Services` ne compile pas, et un cycle ne peut pas s'écrire dans `bootstrap.rs`.
- **Commandes triviales.** Une commande se réduit à une traduction, ce qui limite les endroits où la logique peut se cacher.

### Négatives et compromis

- **Coût de déclaration.** Chaque service demande une `struct`, un `new()`, un champ dans `Services` et une ligne de déstructuration. Ce coût est accepté pour la vérification à la compilation.
- **`bootstrap.rs` est un point de convergence.** Il connaît tous les services et toutes les implémentations, ce qui en fait le fichier le plus modifié lors d'un ajout. C'est le prix de la règle n° 3.
- **Cloner doit rester bon marché.** Tout état partagé doit être derrière un `Arc`. Un service qui copierait ses données à chaque clone irait à l'encontre de la décision.
- **Les erreurs de `State` restent à l'exécution.** Le test de démarrage avec `try_state` est indispensable : sans lui, la déstructuration ne couvre pas l'enregistrement effectif dans Tauri.
- **Les services ne voient pas les réglages en direct.** Une `SettingsView` est une vue en lecture seule : un service lit la valeur au moment de s'en servir, ou s'abonne à un changement. Il faut donc penser à s'abonner lorsqu'un réglage doit être appliqué à chaud (§9).
- **Un type par service.** Il n'est pas possible de gérer deux instances d'un même service avec des configurations différentes dans Tauri. Si ce besoin apparaît, il faudra un type distinct ou une ADR qui le justifie.

### Risques et suivi

- Écrire le test de démarrage (`try_state` pour chaque service) avant l'arrivée du premier service métier.
- Vérifier en revue que chaque `async fn` d'un port passe par `async-trait` et qu'aucune commande ne renvoie autre chose que `Result<_, AppError>`.
- Si un service doit un jour dépendre d'un autre dans les deux sens, revoir la décision avec une ADR plutôt que de contourner le graphe.
