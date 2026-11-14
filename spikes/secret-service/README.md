# Spike 2.6 : Secret Service (trousseau)

Petit binaire Rust qui valide l'accès au trousseau Linux (GNOME Keyring, Secret Service D-Bus)
pour la clé TMDB, avec le repli par variable d'environnement. Résultats et conclusions :
[`docs/spikes/2.6-secret-service.md`](../../docs/spikes/2.6-secret-service.md).

## Commandes

```
secret-service-spike [--timeout-secs N] <commande>

set <valeur>   enregistre la valeur de test (open-mosaik-spike / tmdb-api-key-spike)
get            affiche la valeur de test
delete         supprime l'entrée de test
status         état de la collection par défaut, puis accès à l'entrée
resolve        ordre : OPEN_MOSAIK_TMDB_API_KEY (verrouillé par l'environnement) > trousseau > non configurée
raw-get        DIAGNOSTIC : get_password sans la garde de verrouillage
```

Codes de sortie : 0 succès, 1 trousseau indisponible ou erreur, 2 usage, 3 non configurée (`resolve`), 4 verrouillé.
`--timeout-secs` (défaut 8) borne chaque appel au trousseau.

## Garde de verrouillage

Chaque commande qui touche une entrée lit d'abord `Locked` sur la collection par défaut (lecture seule,
sans invite). Un trousseau verrouillé renvoie `Trousseau verrouillé` (code 4). Sans cette garde,
`get_password` déclenche une invite de déverrouillage.

## Backend

`keyring` 3.6.3 avec `default-features = false` et `sync-secret-service`. Voir le rapport, section 3.

## Scénarios

- `scripts/run-all.sh` : lance tous les scénarios et affiche les sorties brutes.
  Le scénario 1 touche le bus réel, uniquement avec l'entrée de test, et la supprime.
  Les autres scénarios tournent sur des bus privés.
- `scripts/iso-session.sh <nom> -- <commande>` : exécute une commande sur un bus D-Bus privé.
  HOME, XDG_* et le répertoire de services sont isolés sous `target/i/<nom>/`. Le bus disparaît avec la commande.
- `scripts/scenario-locked.sh`, `scripts/scenario-nosecrets.sh`, `scripts/scenario-hang.sh` : scénarios 2a, 3 et 4.
- `scripts/fake-secrets.py {silent|locked-prompt}` : faux service `org.freedesktop.secrets` pour simuler un service muet ou une invite sans réponse.

Le trousseau réel n'est jamais verrouillé, déverrouillé ou modifié. Aucun démon n'est lancé hors d'un bus isolé.

## Build

```
cargo build
cargo clippy --all-targets -- -D warnings
```

Le crate est autonome : `[workspace]` vide dans `Cargo.toml`, exclu du workspace racine.
