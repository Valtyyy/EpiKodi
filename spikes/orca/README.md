# Spike 2.5 - Orca / AT-SPI (issue #14)

Rapport complet : `docs/spikes/2.5-orca.md`.

Contenu :

| Chemin | Role |
|---|---|
| `src/main.rs`, `tauri.conf.json`, `Cargo.toml` | Hote Tauri 2 minimal (fenetre "Orca spike", `frontendDist` = `dist/`). Crate autonome (`[workspace]` vide). |
| `dist/index.html` | Page de test statique : titre, 4 boutons en roving tabindex, regions `aria-live` polite/assertive, champ de saisie, image avec `alt`, barre de progression (`aria-valuenow`), dialogue (`role="dialog"` + `aria-labelledby`). Un script joue un scenario sans intervention humaine (cycle de 16 s, premier cycle a 6 s). |
| `scripts/atspi_probe.py` | Sonde AT-SPI (Python, `gi` + Atspi 2.0) : trouve la fenetre, dump l'arbre (role, nom, etats, attributs, Value/Text), ecoute N secondes, deregistre ses listeners, quitte seule. Option `--all-object-events` : enregistre toute la famille `object`. |
| `announcer/` | Prototype Rust du port `Announcer` : `NoopAnnouncer`, `SpeechDispatcherAnnouncer` (sous-processus `spd-say`), et un bench de latence (`src/bin/bench.rs`). |
| `target/logs/` | Journaux de toutes les executions (ignores par git, via `target/`). |

## Rejouer

Pre-requis : Ubuntu 24.04, `webkit2gtk-4.1`, `orca`, `speech-dispatcher` / `spd-say`, Python `gi` avec Atspi 2.0.

```bash
cd spikes/orca
cargo build                              # app Tauri (target/debug/orca-spike)

# Une seule instance a la fois, toujours sous timeout, journaux hors terminal
timeout 60 ./target/debug/orca-spike > target/logs/app.log 2>&1 &
APP_PID=$!
sleep 3
timeout 50 python3 -I scripts/atspi_probe.py --seconds 35 > target/logs/probe.log 2>&1
wait $APP_PID

# Sonde "tous les evenements object" (regions live)
timeout 50 python3 -I scripts/atspi_probe.py --seconds 35 --all-object-events > target/logs/probe-all.log 2>&1

# Latence (silencieux) et un seul appel audible
cd announcer && cargo test && cargo build --release --bins
./target/release/bench                         # aucun son
./target/release/bench --speak-once "Test"     # UN son (test manuel volontaire)
```

Ne pas lancer Orca depuis ces scripts : l'ecoute est manuelle (voir la checklist dans le rapport, section 7).

## Etat systeme

Le spike ne modifie pas la configuration. `org.gnome.desktop.interface toolkit-accessibility` vaut `false` avant et apres ; WebKitGTK expose l'arbre sans le changer (verifie dans les journaux). Aucun processus `orca-spike` ou `atspi_probe` ne doit rester : verifier avec `ps -eo pid,comm,args | grep -E 'orca-spike|atspi_probe'`.
