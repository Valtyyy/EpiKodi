# ADR 0008 : Configuration à chaud, secrets en écriture seule et invariants non configurables

- **Statut :** Proposé
- **Date :** 2026-11-14
- **Références :** `ARCHITECTURE.md` §1 (C5, C6, C7), §3 (ports, `SettingsView`), §4 (références de trousseau, clé TMDB), §5.1 et §5.3 (configuration de mpv, passerelle de flux), §7 (bac à sable), §8 (adresse d'écoute, appairage), §9 (configuration), §12 (sécurité), §14 (ADR-008), §16 ; [ADR-0003](0003-injection-de-dependances.md) ; [spike 2.6](../spikes/2.6-secret-service.md) ; issue #25 ; EPIC 09 (télécommande)

## Contexte

Le sujet impose que tout soit configurable depuis l'application, sans fichier à éditer (C7). Les réglages doivent donc être typés, validés, appliqués à chaud et persistés. Trois exigences en découlent.

- **Un réglage invalide ne doit pas bloquer l'utilisateur.** Une valeur corrompue, par exemple après une mise à jour, ne doit pas empêcher l'application de démarrer, puisque l'utilisateur doit pouvoir la corriger depuis l'écran Paramètres (`ARCHITECTURE.md` §9).
- **Les secrets ne doivent jamais sortir du processus en clair.** La clé TMDB et les mots de passe SMB ne doivent ni être renvoyés à l'interface, ni être stockés en clair sur le disque, ni apparaître dans les journaux (§4, §9, §12 ; C6 pour les chemins de lecture).
- **Certaines valeurs sont des garanties de sécurité.** Les rendre réglables reviendrait à les supprimer (§9, « Ce qui reste volontairement non configurable »).

Hypothèses encore non validées, à ne pas confondre avec des acquis : le chiffrement du transport D-Bus (`crypto-rust`), la garde de verrouillage et le délai de 8 secondes ne sont pas testés contre un Secret Service réel. L'invite de déverrouillage, le comportement sous connexion automatique et KWallet ne sont pas observés sur une session réelle (`ARCHITECTURE.md` §16 ; [spike 2.6](../spikes/2.6-secret-service.md), section 8).

## Décision

### 1. Un seul catalogue

Chaque réglage simple est déclaré une fois, dans la macro `settings_catalog!` (`crates/core/src/settings/catalog.rs`). Cette déclaration génère :

- la struct `Settings`, avec un champ typé par réglage et une valeur par défaut ;
- `Settings::descriptors()`, qui renvoie des `SettingDescriptor { key, section, label, description, kind, default, unit, apply, visible }` ;
- `Settings::get(key)` et `Settings::set(key, value)`, où `set` valide la valeur avant de l'affecter.

Les types suivants sont exportés en TypeScript par `ts-rs` sous la feature `ts` : `SettingValue`, `SettingChange`, `SettingsPatch`, `SettingEntry`, `SettingsSnapshot`, `SettingKind`, `ApplyMoment`, `SettingDescriptor`, `SettingSource`, `SettingWarning`, `SecretKey`, `SecretSource`, `SecretState`, `SecretStatus`. Les fichiers générés vont dans `ui/shared/src/generated` et ne sont jamais écrits à la main (règle 7 de `AGENTS.md`).

Ajouter un réglage consiste à ajouter une entrée dans la macro. Si son type Rust n'existe pas encore, il faut aussi implémenter `FieldType` pour ce type. Le test `descriptors_cover_every_key_with_matching_defaults` compte les réglages en dur et doit être mis à jour dans le même changement.

**Bornes.** `SettingKind` porte les contraintes : `Bool`, `Int { min, max, step }`, `Choice { options }`, `Text { max_len }`. `SettingKind::validate` les vérifie **uniquement côté Rust**, à chaque `set` et donc à chaque `update`. L'écran ne fait qu'afficher `kind` et le message de suggestion renvoyé par Rust.

**Écran généré.** `SettingsService::snapshot()` renvoie tout l'écran Paramètres en une fois : les entrées (descripteur, valeur, source, raison de verrouillage), l'état des secrets et les avertissements. Les réglages dont `visible` vaut `false` restent dans le snapshot et doivent être filtrés par l'interface.

Catalogue actuel (5 réglages) :

| Clé | Variable d'environnement | Type | Défaut | Bornes | Unité | Moment | Visible |
|---|---|---|---|---|---|---|---|
| `interface.window_decorations` | `OPEN_MOSAIK_INTERFACE_WINDOW_DECORATIONS` | booléen | `true` | | | Immédiat | oui |
| `network.timeout_ms` | `OPEN_MOSAIK_NETWORK_TIMEOUT_MS` | entier | `10000` | 1000 à 60000, pas de 500 | ms | Prochaine opération | oui |
| `library.scan_parallelism` | `OPEN_MOSAIK_LIBRARY_SCAN_PARALLELISM` | entier | `2` | 1 à 8, pas de 1 | | Prochaine opération | oui |
| `playback.hardware_decoding` | `OPEN_MOSAIK_PLAYBACK_HARDWARE_DECODING` | booléen | `true` | | | Prochaine lecture | oui |
| `setup.first_run_completed` | `OPEN_MOSAIK_SETUP_FIRST_RUN_COMPLETED` | booléen | `false` | | | Immédiat | non |

Les autres réglages de la section 9 de `ARCHITECTURE.md` (télécommande, métadonnées, lecture, extensions) ne sont pas encore au catalogue. Ils seront ajoutés avec leur service.

### 2. Moments d'application

`ApplyMoment` prend quatre valeurs :

- **`Immediate`** : le service s'abonne à `SettingsView::changed()` et applique la valeur sans attendre. C'est le cas de `interface.window_decorations`, appliqué par `set_decorations` dans `src-tauri/src/lib.rs`.
- **`NextOperation`** : le service lit `view.current()` au début de la prochaine opération (scan, requête réseau). C'est le cas de `network.timeout_ms` et `library.scan_parallelism`.
- **`NextPlayback`** : même principe, au prochain lancement de lecture (`playback.hardware_decoding`).
- **`VerifiedRollback`** : la valeur n'est appliquée qu'après un essai réussi, avec retour arrière en cas d'échec. Aucun réglage du catalogue ne l'utilise aujourd'hui.

**Refus dans `update()`.** `update()` refuse toute clé dont le moment est `VerifiedRollback` (fonction `ensure_plain_update`), avec une erreur de validation sur le champ concerné et la suggestion « Il est appliqué puis vérifié par un service dédié ». Le test `verified_rollback_settings_are_not_directly_updatable` le vérifie. La règle est en place avant l'arrivée du premier réglage de ce moment.

**Adresse d'écoute de la télécommande (EPIC 09).** Ce réglage aura le moment `VerifiedRollback`. Il ne passera **pas** par `SettingsService::update()`, car un `bind` peut échouer et il ne faut enregistrer que ce qui fonctionne (§9). Il sera écrit par `RemoteServerService::reconfigure(host, port)`, qui enregistre la valeur après un `bind` réussi.

### 3. Priorité : environnement, puis base, puis défaut

Pour chaque clé, `SettingsService::load` applique la priorité suivante :

1. **Variable d'environnement** `OPEN_MOSAIK_<CLÉ>`, produite par `env_var_name` : la clé est mise en majuscules et les points deviennent des underscores, par exemple `network.timeout_ms` donne `OPEN_MOSAIK_NETWORK_TIMEOUT_MS`. Les underscores déjà présents dans la clé sont conservés.
2. **Valeur enregistrée** dans la table `setting`.
3. **Valeur par défaut** du catalogue.

La valeur d'environnement est lue par `SettingKind::parse_plain` (booléens `true`, `false`, `1`, `0`, sans tenir compte de la casse ; entiers après `trim`), puis validée comme une valeur normale.

**Verrouillage.** Une variable d'environnement valide a deux effets :

- la source devient `Env` et `SettingsMeta.locks` reçoit la raison « Imposé par la variable d'environnement OPEN_MOSAIK_… », affichée à l'écran ;
- `update()` refuse toute clé verrouillée, avec le champ concerné et la suggestion de retirer la variable. La requête est refusée en entier : aucun autre réglage du même lot n'est persisté. Le test `env_locked_key_rejects_update_and_persists_nothing` le vérifie.

La valeur enregistrée reste dans la base, mais elle est ignorée tant que la variable est définie.

### 4. Valeurs invalides et démarrage

- Une **variable d'environnement invalide** est ignorée avec un `SettingWarning`. La valeur enregistrée prend alors le relais, sans verrouillage.
- Une **valeur enregistrée invalide** (JSON incorrect, hors bornes, mauvais type) est ignorée avec un avertissement. La valeur par défaut est utilisée.
- Une **table illisible** (`load_all` en erreur) donne les valeurs par défaut et un avertissement de clé `storage`.
- `SettingsService::load` renvoie toujours un `Self`, jamais une erreur. `SettingsService::load_with_warnings` accepte en plus des avertissements déjà connus de l'appelant. Un avertissement disparaît dès que la clé est mise à jour par `update()`.

**Base de réglages inaccessible.** `bootstrap::build` ne renvoie plus d'erreur. Si la base ne s'ouvre pas (création du dossier de données, ouverture SQLite ou migrations), la racine de composition :

1. ouvre une base SQLite `:memory:` (`infra::db::open_in_memory`) ;
2. si cela échoue aussi, utilise un `MemorySettingsRepository` ;
3. ajoute l'avertissement de clé `storage` : « Base de réglages inaccessible : <cause>. Les modifications ne seront pas conservées. »

Les réglages fonctionnent alors en mémoire, pour la session seulement. L'application démarre, et l'utilisateur voit l'avertissement dans l'écran Paramètres.

**Limite restante.** `src-tauri/src/lib.rs` résout le répertoire de données par `app.path().app_data_dir()?`. Si Tauri ne peut pas fournir ce chemin, `setup` échoue et l'application ne démarre pas. Ce cas n'est pas couvert par la décision ci-dessus.

### 5. Un seul écrivain

- `SettingsService` est le **seul** service qui écrit : `update()`, `set_secret()`, `clear_secret()`. Lui seul détient le port `SettingsRepository` et le port `SecretStore` pour l'écriture.
- Les autres services reçoivent un `SettingsView`, obtenu par `SettingsService::view()`. Cette vue n'offre que `current()` et `changed()`. Elle est clonable et ne permet pas d'écrire. En test, `SettingsView::fixed(Settings { … })` suffit.
- `src-tauri/src/commands/settings.rs` ne fait que traduire : `get_settings`, `update_settings`, `set_secret`, `clear_secret`. Aucune de ces commandes ne contient de logique.

**Atomicité de `update()`.** `update()` valide tout le lot en mémoire avant toute écriture : clés verrouillées, réglages `VerifiedRollback`, clés inconnues, bornes, types. Il construit ensuite la liste des lignes JSON et appelle `SettingsRepository::save_many`, qui écrit toutes les lignes dans **une seule transaction sqlx**. La mémoire, les métadonnées et la diffusion `watch` ne changent qu'après ce succès. Si l'écriture échoue, rien n'est modifié : ni la base, ni la mémoire, ni les abonnés (test `failed_write_leaves_memory_and_subscribers_unchanged` ; test `save_many_is_atomic` côté SQLite). Un `write_lock` sérialise les mises à jour concurrentes.

### 6. Secrets

- **Port.** `SecretStore` (`crates/core/src/secrets.rs`) expose `status`, `set`, `clear` et `get`. `get` est réservé aux services consommateurs, par exemple un fournisseur de métadonnées. Il ne doit jamais être exposé par une commande Tauri ni par une route HTTP.
- **Implémentation.** `KeyringSecretStore` (`crates/infra/src/secret_store.rs`) utilise la crate `keyring` 3, avec les features `sync-secret-service` et `crypto-rust` (`Cargo.toml` racine), et la crate `dbus-secret-service` 4.1 pour lire la propriété `Locked`. L'entrée porte le service `open-mosaik` et l'identifiant `tmdb_api_key`.
- **Écriture seule.** L'interface ne reçoit que `SecretStatus { key, configured, source, state, message? }`. `source` vaut `env`, `keyring` ou `none`. `state` vaut `ok`, `env_locked`, `keyring_locked` ou `unavailable`. Aucun champ ne contient la valeur. La valeur entre une seule fois, par `set_secret(key, value: String)`, puis est enveloppée immédiatement dans `SecretValue`.
- **Variable d'environnement.** `OPEN_MOSAIK_TMDB_API_KEY` (`SecretKey::env_var`). Si sa valeur n'est pas vide après `trim`, l'état est `env_locked`, `set` et `clear` sont refusés, et le trousseau n'est **jamais** interrogé (test `env_secret_locks_and_never_touches_keyring`, qui compte les appels au magasin). Une variable vide ou composée d'espaces est ignorée.
- **Trousseau verrouillé.** Avant tout `get_password`, `set_password` ou `delete_credential`, `guarded()` lit la propriété `Locked` de la collection par défaut. Si elle est verrouillée, l'appel renvoie `SecretError::Locked` sans déclencher d'invite (spike 2.6, recommandation 3). Cette lecture utilise `EncryptionType::Plain`, car seule la propriété est lue, aucun secret ne circule.
- **Budget.** Chaque appel, garde comprise, s'exécute sur un thread dédié, sous `tokio::time::timeout` de 8 secondes. Au-delà, l'erreur est `Unavailable("délai dépassé (8 s)…")`. Le thread expiré reste bloqué jusqu'à la fin de l'appel D-Bus, car l'appel ne peut pas être annulé.
- **Échec explicite, sans repli.** Si le trousseau est indisponible, `set` renvoie une erreur (`secret_keyring_unavailable` ou `secret_keyring_locked`) et rien n'est écrit. Il n'existe aucun repli vers un fichier, la base ou la mémoire.
- **Valeur masquée.** `SecretValue` n'implémente ni `Serialize`, ni `Display`. Son `Debug` affiche `SecretValue(***)`. Seul `expose()` donne accès à la valeur, pour le code qui l'utilise. `map_error` ne formate jamais les variantes qui peuvent contenir des octets du secret (`BadEncoding`). Le test `bad_encoding_never_leaks_bytes` le vérifie. Le code actuel ne contient aucun appel de journalisation sur une valeur de secret.
- **Résolution pour les consommateurs.** `SettingsService::secret(key)` renvoie `Result<Option<SecretValue>, AppError>` : la valeur d'environnement d'abord, puis le trousseau. `Ok(None)` signifie « non configurée ». Une erreur du trousseau est renvoyée avec son code (`secret_keyring_locked` ou `secret_keyring_unavailable`), sans se confondre avec l'absence de valeur (test `secret_reports_keyring_failure_instead_of_none`).

### 7. Invariants non configurables

Ces valeurs ne sont pas au catalogue. Les rendre réglables supprimerait la garantie de sécurité. Les raisons viennent de `ARCHITECTURE.md`.

| Invariant | Raison | État |
|---|---|---|
| Passerelle de flux liée à `127.0.0.1`, jeton aléatoire à usage limité par session de lecture | Elle ne doit jamais être exposée au LAN (§5.3) | À implémenter (`remote-api`) |
| Appairage obligatoire sur toutes les routes sauf `GET /` et `POST /pair` | Exigence du sujet (§8) | À implémenter (EPIC 09) |
| Contrôle des permissions et bac à sable des extensions | Exigence du sujet (C2, §7) | À implémenter (EPIC 08) |
| Configuration fermée de mpv : `config=no`, `load-scripts=no`, `ytdl=no`, `input-default-bindings=no`, `input-vo-keyboard=no`, `osc=no` | mpv ne lit pas la configuration utilisateur, n'exécute aucun script et ne résout jamais d'URL venue d'une extension (§5.1, ADR-0002 règle 2) | À implémenter (`player-mpv`) |
| Code d'appairage d'au moins 6 chiffres, au plus 10 essais | Protection contre la force brute (§9, §12) | Pas encore de réglage. Les valeurs de référence de §8 et §12 sont 6 chiffres, 2 minutes, usage unique et 5 essais. Les 10 essais sont le plafond, et 5 la valeur retenue par défaut. |
| Limites d'une extension (mémoire, délai, essais de redémarrage) bornées par un plafond | Raison déduite de §7 : ces limites sont une défense en profondeur. Un réglage sans plafond permettrait de les lever. | Aucun plafond chiffré n'existe dans les documents. Il faut le fixer avant d'ajouter le réglage. |

La règle de vérification est une revue : ajouter au catalogue une clé correspondant à l'un de ces invariants exige une nouvelle ADR. Un test de non-régression qui vérifie l'absence de ces clés dans `Settings::descriptors()` est proposé, mais il n'est pas écrit.

### 8. Persistance

- **Base.** SQLite via `sqlx` 0.9 (contrainte `0.9` dans le `Cargo.toml` racine ; la version exacte est fixée par `Cargo.lock`), avec les features `sqlite`, `runtime-tokio`, `migrate` et `macros`.
- **Ouverture.** `crates/infra/src/db.rs` ouvre `open-mosaik.db` dans le répertoire de données de l'application, en mode WAL, avec les clés étrangères activées et un pool de 5 connexions. Les tests utilisent `:memory:` avec une seule connexion.
- **Port.** `SettingsRepository` expose `load_all` et `save_many`. `save_many` écrit tout le lot dans une transaction. `save`, pour une seule clé, en est une méthode par défaut.
- **Migrations.** `crates/infra/migrations/0001_setting.sql` crée la table `setting(key TEXT PRIMARY KEY NOT NULL, value TEXT NOT NULL, updated_at INTEGER NOT NULL)`.
- **Format.** `value` contient le JSON de `SettingValue`, par exemple `{"type":"bool","value":false}`. L'écriture est un `INSERT … ON CONFLICT(key) DO UPDATE`, dans la transaction de `save_many`. `updated_at` vient du port `Clock`.

**Fichier `.env`.** `.gitignore` ignore `.env` et `.env.*`, sauf `.env.example`, conformément à `ARCHITECTURE.md` §12. Le fichier `.env.example` (racine du dépôt) liste les variables `OPEN_MOSAIK_*` ci-dessus, commentées.

## Alternatives écartées

- **Fichier de configuration à éditer (TOML ou JSON).** Écarté : C7 et §9 imposent « sans fichier à éditer ». Une erreur de syntaxe bloquerait le démarrage, et un fichier ne permet pas de verrouiller un réglage avec sa raison affichée.
- **Réglages modifiables par n'importe quel service.** Écarté : contraire à la règle 8 de l'ADR-0003. Un service qui écrit peut former un cycle, et les tests ne peuvent plus fournir une vue figée (§3, §9). Seul `SettingsService` écrit.
- **Secrets dans la table `setting`.** Écarté : la base contiendrait le secret en clair, et `SettingValue` pourrait sortir vers l'interface. §4 et §9 interdisent les secrets dans la base.
- **Secrets en clair quand le trousseau est absent.** Écarté : §4 et §9 imposent un échec explicite. L'utilisateur peut utiliser la variable d'environnement.
- **Chiffrement dans un fichier, avec clé à stocker quelque part.** Écarté : §11 pose la question « où mettre la clé ? » sans y répondre. Le Secret Service est le standard Linux (§11).
- **`keyutils` comme stockage.** Écarté : le cache keyutils ne survit pas au redémarrage (spike 2.6, section 3).
- **Clé et valeur non typées (`HashMap<String, String>`).** Écarté : aucune borne ni type ne peut alors être vérifié, ni généré pour l'écran. C'est l'objet même du catalogue.
- **Écrire l'adresse de la télécommande par `update()`.** Écarté : un `bind` peut échouer, et §9 impose de n'enregistrer que ce qui fonctionne. Le réglage passe donc par `reconfigure`.
- **Variable d'environnement qui écrase la valeur sans verrouiller.** Écarté : l'utilisateur ne saurait pas pourquoi le réglage ne change pas. Le verrou et sa raison sont affichés.
- **Arrêt au démarrage si la base de réglages est inaccessible.** Écarté : §9 impose que l'application démarre, et l'utilisateur doit pouvoir voir et corriger ses réglages. Le repli en mémoire avec avertissement remplace cette option.

## Conséquences

### Positives

- Un réglage se déclare à un seul endroit. Le type, la valeur par défaut, les bornes, le moment d'application et l'export TypeScript suivent de cette déclaration.
- Les bornes sont vérifiées en Rust, avant toute persistance. L'interface ne peut pas les contourner.
- Une écriture de réglages est atomique : la base, la mémoire et l'écran restent cohérents, même en cas d'échec.
- Aucun secret ne revient vers l'interface ou vers un `Debug`, et l'environnement permet le développement et la CI sans trousseau.
- Une valeur invalide enregistrée ne bloque pas le démarrage. Une base inaccessible non plus : l'application démarre en mémoire et l'avertissement est affiché.
- Un échec du trousseau est distinct de l'absence de secret pour les consommateurs.

### Négatives et compromis

- La liste des réglages est codée en dur dans un test (`descriptors.len() == 5`). Ajouter un réglage exige de modifier ce test.
- Un réglage verrouillé par l'environnement affiche la valeur effective, mais la valeur enregistrée reste invisible tant que la variable est définie.
- Sans base, les modifications ne survivent pas à la fermeture de l'application. L'avertissement l'indique.
- La valeur de secret transite une fois par l'IPC Tauri, en entrée (`value: String` dans `set_secret`). Elle ne revient jamais en sortie.
- Le trousseau est interrogé à chaque appel, avec un thread par appel. Ce coût est accepté, car les appels sont rares (spike 2.6, recommandation 4).

### Risques et suivi

- **Trousseau, à valider sur un Secret Service réel.** Hypothèses non testées :
  - le chiffrement du transport D-Bus (`crypto-rust`) n'a pas été testé (spike 2.6, recommandation 8) ;
  - la garde de verrouillage (lecture de `Locked` avant tout accès) n'a pas été testée contre un vrai Secret Service : les tests unitaires utilisent un faux magasin ;
  - le délai de 8 secondes n'a pas été mesuré sur une invite réelle (spike 2.6, scénario 4, testé seulement sur un faux service) ;
  - l'invite de déverrouillage réelle (M1) et le démarrage avec connexion automatique (M2) ne sont pas observés ;
  - KWallet n'est pas testé (M3) ;
  - une machine sans trousseau n'est pas testée (M4).
  Responsable : binôme ; échéance : avant la clôture de l'issue #25.
- **Trousseau réel.** Le test `real_keyring_roundtrip` est marqué `#[ignore]`. Il écrase l'entrée réelle et ne doit être lancé qu'à la main, sur une session de test.
- **Repli de démarrage.** Le test `unopenable_database_falls_back_with_a_storage_warning` (`src-tauri/src/bootstrap.rs`) ne couvre que l'échec de création du dossier de données. L'échec d'ouverture SQLite et l'échec des migrations ne sont pas testés. Action : ajouter ces deux cas. Responsable : binôme ; échéance : avant la clôture de l'issue #25.
- **Résolution du répertoire de données.** Si `app.path().app_data_dir()` échoue, `setup` échoue (voir §4). Décision à prendre, ou à accepter et documenter.
- **Vérification de cette ADR.** Elle a été rédigée à partir de la lecture du code. Les tests existants n'ont pas été exécutés pour cette rédaction. Avant acceptation, lancer `cargo test --workspace --all-features`, puis `pnpm gen:types` pour vérifier les types exportés.
- Réviser cette ADR si le moment `VerifiedRollback` change, si un invariant devient configurable, ou si le trousseau réel contredit les hypothèses ci-dessus.
