# ADR 0001 : Application Tauri 2 et découpage en crates Cargo

- **Statut :** Accepté
- **Date :** 2026-11-11
- **Références :** `ARCHITECTURE.md` §1 (C1, C2, C4, C5), §2 (« Découpage du dépôt »), §5.2, §7, §11, §13, §14 (ADR-001)

## Contexte

Open Mosaik est un media center de salon pour Linux, piloté depuis le navigateur d'un smartphone. Plusieurs exigences du sujet pèsent sur la structure du code :

- **C1** : la lecture passe par une vraie bibliothèque (libmpv), jamais par le lecteur intégré de Tauri ou d'Electron, et aucune balise `<video>` ou `<audio>`.
- **C2 et C3** : les extensions tournent dans un processus séparé et se chargent à chaud.
- **C4** : la télécommande est une page web servie par le media center, indépendante de l'IPC Tauri, branchée sur les mêmes services.
- **C5** : le cœur métier se teste sans réseau, sur une seule commande, sans dépendance à Tauri, GTK ou libmpv.

Le dépôt ne contient aujourd'hui que le scaffold Tauri (`src/`, `src-tauri/`). Le découpage décrit ici est une cible (`ARCHITECTURE.md` §15).

Le sujet impose Tauri comme shell d'application (`ARCHITECTURE.md` §11). La fenêtre doit en outre composer une surface GTK (`GtkGLArea`) pour le rendu mpv, sous le webview (§5.2).

## Décision

### Shell

L'application est une application **Tauri 2.12**. La racine de composition `src-tauri` construit la fenêtre GTK, la surface vidéo et le webview, expose les commandes Tauri et démarre les services.

### Workspace Cargo

Le dépôt est un workspace Cargo dont les crates sont les suivantes :

| Crate | Contenu |
|---|---|
| `crates/core` | Domaine, services, ports. Aucune dépendance native. |
| `crates/infra` | SQLite, SMB, TMDB, MusicBrainz, secrets, images. |
| `crates/player-mpv` | `PlayerEngine` via libmpv. |
| `crates/remote-api` | Routeur axum et passerelle de flux. |
| `crates/ext-host` | Binaire séparé : QuickJS et bac à sable. Sans lien avec GTK ni WebKit. |
| `crates/ext-protocol` | Messages JSON-RPC, manifeste, permissions. Types uniquement. |
| `src-tauri` | Racine de composition, commandes, surface GTK. |

### Sens des dépendances

```text
src-tauri  →  { infra, player-mpv, remote-api }  →  core
```

- `core` ne dépend d'aucune autre crate applicative, à l'exception de `ext-protocol`, qui ne contient que des types.
- `core` n'a ni Tauri, ni GTK, ni libmpv dans son `Cargo.toml`. Un `use tauri::…` dans `core` ne compile donc pas. La règle est vérifiée par le compilateur, et non par convention.
- Tauri et GTK n'apparaissent que dans `src-tauri`. L'`EventBus` de l'interface (port `UiBridge`, implémenté par `AppHandle::emit`) y est donc aussi.

## Alternatives écartées

- **Electron.** Écarté : il n'offre pas d'accès natif équivalent pour composer la vidéo, et il est plus lourd (`ARCHITECTURE.md` §11). Le sujet retient Tauri.
- **Application GTK native, sans webview.** Raison déduite des autres décisions du document : l'interface de salon et la télécommande sont en React 19 avec des composants partagés (§6), et le thème passe par Tailwind et des variables CSS. Une interface GTK native dupliquerait ces composants et ferait perdre le partage avec `ui/shared`.
- **Un seul crate avec des modules et des feature flags.** Une séparation par modules n'est qu'une convention : rien n'empêche un module métier d'importer Tauri. Les feature flags ne suffisent pas non plus, car une simple importation dans le build par défaut passe. Le document fait reposer C5 sur la compilation (§2), et la stratégie de test (§13) suppose que `core` et `remote-api` se testent sans GTK, WebKit ni libmpv.
- **`tauri-plugin-libmpv` pour intégrer mpv.** Écarté : son README indique que l'embarquement sous Linux ne fonctionne pas (`ARCHITECTURE.md` §5.2). Le moteur est donc une crate propre, `player-mpv`, qui parle directement à libmpv.
- **Embarquer mpv avec `--wid`.** Écarté : cette option ne fonctionne que sous X11, alors que la session de développement est sous Wayland (§5.2).

## Conséquences

### Positives

- La règle de dépendance est contrôlée par le compilateur. Une violation est une erreur de compilation, et non une dérive découverte en revue.
- `core` et `remote-api` se testent en CI sans dépendance native. `remote-api` est testé avec `tower::oneshot`, sans port ouvert (§13).
- `ext-host` est un binaire séparé, sans GTK ni WebKit. Il est empaqueté à part (`bundle.externalBin`, §7).
- Changer une implémentation (par exemple le client SMB) ne touche que `infra` : le port `SourceProvider` limite le changement à cette crate (§5.3).

### Négatives et compromis

- Le workspace comporte plus de fichiers `Cargo.toml` à maintenir. Les versions doivent rester cohérentes entre crates.
- Toute la composition se concentre dans `src-tauri`, qui devient le point de passage obligé pour brancher une nouvelle implémentation. C'est voulu, mais le fichier grossit avec le projet.
- Le passage du scaffold à la cible demande un travail de migration, listé en §15 : retirer `panic = "abort"`, supprimer `UserService` et `greet`, déplacer l'application modèle vers `ui/tv`, et retirer `tauri-plugin-opener` s'il est inutile.

### Risques et suivi

- La superposition du webview sur la `GtkGLArea` (plan A, §5.2) reste une **hypothèse non validée**. Elle ne remet pas en cause le découpage, mais `player-mpv` et `src-tauri` en dépendent. Si le test échoue, le plan B réutilise la même surface GL et le même code de rendu, sans changer le découpage. Le résultat donnera lieu à l'ADR 0002.
- Vérifier, par revue ou par un test de CI, qu'aucune crate autre que `src-tauri` ne déclare Tauri, GTK ou libmpv dans son `Cargo.toml`.
