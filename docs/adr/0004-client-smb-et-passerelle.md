# ADR 0004 : Client SMB (crate `smb`) et passerelle de flux locale

- **Statut :** Accepté
- **Date :** 2026-11-12
- **Références :** `ARCHITECTURE.md` §1 (C6), §4 (« Sources »), §5.3, §11, §12, §14 (ADR-004), §16 ; [spike 2.3](../spikes/2.3-smb.md) (issue #12) ; issues #39 (SourceProvider SMB), #40 (SourceRegistry et coupe-circuit), #63 (passerelle de flux)

## Contexte

Un partage SMB lent ou injoignable ne doit jamais figer les autres sources ni l'interface (C6). Les données de la médiathèque viennent toujours de SQLite, et chaque appel distant doit avoir un délai maximal (§4).

`ARCHITECTURE.md` §5.3 retient la crate `smb` plutôt que `pavao`, dont la documentation décrit un verrou global au processus, et il demande de valider `smb` sur Samba. Ces hypothèses (§16) sont maintenant testées par le [spike 2.3](../spikes/2.3-smb.md) : `smb` 0.12.1 contre Samba 4.17.12 sous Docker, 182 fichiers répartis dans 54 dossiers, dont un fichier de 50 MiB.

Résultats utiles à la décision :

- Le listage récursif, `read_at` (50 lectures sur 50 identiques, comparaison octet à octet et SHA-256), NTLM et les erreurs d'authentification explicites fonctionnent.
- L'accès invité fonctionne, mais seulement avec l'utilisateur littéral `/GUEST` et `allow_unsigned_guest_access: true`. Les noms `Guest`, `guest`, `nobody` et `randomuser` échouent avec une `InvalidState` peu parlante.
- `share_connect` vers un hôte gelé (`docker pause`) ne retourne **jamais** (plus de 120 s observées), même avec `ConnectionConfig::timeout` à 2 s. Les requêtes en vol ont, elles, un délai interne de 10 s.
- Après `docker stop` puis `start`, l'ancien `Client` renvoie des erreurs à vie. Un nouveau `Client` se connecte dès que Samba redémarre.
- Chaque aller-retour coûte environ 40 ms, même en local : 1.6 MB/s avec des blocs de 64 KiB, 24.5 MB/s avec des blocs de 1 MiB.
- Un serveur à +300 ms (`netem`) ralentit le listage de 9.15 s à 66.19 s et le débit à 3.1 MB/s.

Limites de la validation : un seul serveur (Samba), pas de test Windows ou NAS, pas de signature ni de chiffrement obligatoires, pas de test de concurrence réelle.

## Décision

### 1. Client SMB : la crate `smb` 0.12 dans `infra`

`infra` implémente le port `SourceProvider` pour SMB avec `smb` 0.12. Le port ne change pas (§4). Les défauts constatés se contournent dans `infra`, par les règles suivantes, qui sont obligatoires.

1. **Délais externes.** Chaque appel est enveloppé dans `tokio::time::timeout` : connexion et authentification 5 s ; ouverture ou listage d'un dossier 5 s ; `read_at` 5 s ; `close()` 2 s ou en tâche détachée, car il bloque 10 s sur un serveur gelé. `ConnectionConfig::timeout` est fixé à 5 s pour les requêtes en vol. Le délai externe est indispensable à la connexion, que la crate ne borne pas.
2. **Client recréé.** À la première erreur réseau (`ConnectionStopped`, `Failed to send message to worker`, `OperationTimeout`, `Connection refused`), le `Client` est jeté et reconstruit. Un client en erreur n'est jamais réutilisé. `Logon Failure` et `Access Denied` ne sont pas des pannes réseau : elles ne comptent pas pour le coupe-circuit et remontent « identifiants refusés ».
3. **Gros blocs.** La lecture se fait par blocs d'au moins 1 MiB, à cause des 40 ms par aller-retour. Le listage en parallèle est borné.
4. **Accès invité.** Pour un partage sans identifiants, `infra` utilise l'utilisateur littéral `/GUEST` et `allow_unsigned_guest_access: true`. Un test d'intégration contre Samba (CI, #44) fige ce comportement. *Cette règle repose sur une valeur trouvée dans les tests de la crate, pas dans sa documentation : ce test est le seul garde-fou en cas de changement de version.*
5. **Coupe-circuit** (état diffusé par le `SourceRegistry`, #40), seuils recommandés par le spike :
   - `Unreachable` après 3 échecs réseau consécutifs ou 2 timeouts de 5 s. Dans cet état, aucune requête distante ; une sonde de santé toutes les 15 à 30 s (connexion et listage de la racine, avec le même délai de 5 s). Une sonde réussie ramène à `Online`.
   - `Slow` si la latence médiane d'une opération courante (ouverture et petite lecture) dépasse 1 s, ou si l'ouverture ou l'authentification dépasse 2 s sur 3 mesures glissantes. Retour à `Online` sous 500 ms. Références mesurées : 0.18 s de connexion et environ 125 ms par ouverture sans délai ; 1.8 s et environ 660 ms à +300 ms.
6. **Identifiants.** Un mot de passe n'est jamais stocké dans la base ni dans un fichier. `sources.credential_ref` ne contient qu'une référence vers le `SecretStore` (Secret Service, crate `keyring`). Sans trousseau, l'ajout d'une source authentifiée échoue avec un message explicite : il n'y a **pas de repli en clair**. Un partage invité reste possible (§4, §12).

### 2. Passerelle de flux locale

mpv ne reçoit jamais d'URL `smb://` ni d'identifiants. Pour `PlaybackLocator::Gateway`, il lit `http://127.0.0.1:<port>/stream/<jeton>`.

- **Écouteur distinct.** La passerelle est un second écouteur axum, dans `remote-api`, lié à `127.0.0.1` uniquement, sur un port éphémère. Elle n'est jamais montée sur l'écouteur du LAN. Un test d'intégration vérifie l'adresse liée.
- **Jeton.** Aléatoire, d'au moins 128 bits (générateur cryptographique), associé à une seule session de lecture et révoqué à l'arrêt de la lecture.
- **Route.** `GET` et `HEAD /stream/<jeton>`, avec `Range` : `206 Partial Content` et `Content-Range` pour une plage valide, `Accept-Ranges: bytes` sur toutes les réponses, `416 Range Not Satisfiable` pour une plage invalide. Un jeton inconnu ou révoqué reçoit `404`.
- **Lecture.** La passerelle lit via `SourceProvider::read_at`, par blocs d'au moins 1 MiB, et ne connaît ni SMB ni les identifiants.
- **Journaux.** Ni jeton ni identifiant n'apparaissent dans les journaux. Les chemins de requête sont masqués. Cette règle n'est pas contrôlée par le compilateur : elle est vérifiée par un test sur les journaux et par la revue.

## Alternatives écartées

- **`pavao` 0.2.16 comme client principal.** Le spike l'a exécuté (listage et lecture corrects) mais a relevé trois points : le contexte `libsmbclient` est un `static` unique pour tout le processus, et ses options ne sont appliquées qu'à sa création ; contre un serveur gelé, le test paniquait avec `BadFileDescriptor` après environ 20 s, contre 3 s avec `smb` et un délai externe ; il impose la dépendance système `libsmbclient` et une API synchrone (`spawn_blocking`). Le blocage d'un partage par un autre n'a pas été reproduit en concurrence, mais le risque contredit C6.
  - **Conservé en repli**, dans un processus de travail dédié par source (binaire `smb-worker` lancé par `infra`, un `SmbClient` par processus, protocole JSON-lines ou `postcard` sur stdin et stdout). Un worker bloqué est tué et relancé sans toucher aux autres sources. On bascule si, sur un serveur réel, `smb` échoue de façon répétée à : une authentification NTLM, SMB signé ou chiffré obligatoire, ou le listage et la lecture sur Windows ou NAS, sans correctif possible en amont ni dans `infra`. Coût : un processus par source, une copie mémoire des blocs, une dépendance `libsmbclient` à packager.
- **Donner `smb://` à mpv.** Écarté (§5.3) : la prise en charge dépend de la compilation de mpv, les identifiants risquent d'apparaître dans les journaux, et le coupe-circuit et les délais ne s'appliqueraient plus.
- **Monter le partage avec gvfs ou `cifs`.** Écarté : le montage `cifs` demande des privilèges ou une configuration système (`fstab`, `sudo`), et gvfs dépend de l'environnement de bureau. Cela casse la portabilité, la configuration depuis l'application (C7) et le contrôle fin des pannes.

## Conséquences

### Positives

- Un partage gelé coûte au plus 5 s par appel, puis le coupe-circuit le déclare `Unreachable` sans nouvelle attente. Les autres sources et l'interface ne sont pas touchés (C6).
- Pas de dépendance système `libsmbclient` : Rust pur et asynchrone.
- Les identifiants restent dans le trousseau, hors de mpv, des journaux et du LAN.
- Le port `SourceProvider` ne change pas, et le repli `pavao` reste possible sans toucher aux services.

### Négatives et compromis

- Les contournements (délais, recréation du client, blocs de 1 MiB) vivent dans `infra` et doivent être testés avec un `Client` factice et des pannes programmables.
- La passerelle ajoute un écouteur et un état (les jetons) à gérer.
- La latence fixe de 40 ms par aller-retour (cause non vérifiée : Nagle ou ACK retardé) limite le scan : 9 s pour 54 dossiers en local, 66 s à +300 ms. Le scan doit être incrémental et annulable.
- Le `Client` en erreur étant irrécupérable, chaque panne coûte une reconnexion complète (0.18 s sans délai, 1.8 s à +300 ms).

### Risques et suivi

- **Non testé :** serveurs Windows et NAS, signature ou chiffrement obligatoires, concurrence réelle entre plusieurs partages, lecture courte de `read_at` (non observée). Responsable : EPIC 05, via #39 (SourceProvider SMB) et #44 (Samba sous Docker en CI), avant la fin de l'EPIC.
- Si l'un de ces tests échoue de façon durable, appliquer les conditions de bascule vers `pavao` décrites ci-dessus et réviser cet ADR.
- Le test d'accès invité `/GUEST` doit rester en CI : il protège contre une mise à jour de `smb`.
- Les seuils `Slow` et `Unreachable` sont issus d'un banc local : à réajuster avec des mesures sur un réseau réel.
- Mesurer l'effet de `set_nodelay` si la crate l'expose : l'hypothèse sur la latence de 40 ms n'est pas vérifiée.
