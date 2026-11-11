# Open Mosaik

Media center de salon pour Linux, piloté depuis le navigateur de votre téléphone.

> **Statut :** projet en cours. Le dépôt contient pour l'instant le scaffold Tauri + React + TypeScript. Voir [ARCHITECTURE.md](ARCHITECTURE.md) pour la conception.

## Fonctionnalités prévues

- Lecture vidéo et audio via libmpv
- Interface de salon affichée sur la télévision
- Télécommande accessible depuis le navigateur du téléphone, sans installation
- Extensions installables et désinstallables à chaud
- Réglages modifiables depuis l'application (clé TMDB, sources SMB, etc.)

## Stack

- **Frontend :** React 19, TypeScript, Vite
- **Application :** Tauri 2 (Rust)

## Prérequis

- [Node.js](https://nodejs.org/) 22 et [pnpm](https://pnpm.io/) 10
- [Rust](https://www.rust-lang.org/tools/install) (stable)
### Dépendances système (Linux)

Sur Debian / Ubuntu (testé sur Debian 13) :

```bash
sudo apt-get update
sudo apt-get install -y build-essential pkg-config libssl-dev libgtk-3-dev \
  libwebkit2gtk-4.1-dev libjavascriptcoregtk-4.1-dev libsoup-3.0-dev \
  librsvg2-dev libayatana-appindicator3-dev libxdo-dev
```

Pour les autres distributions, voir la [documentation Tauri](https://tauri.app/start/prerequisites/#linux).

## Installation

```bash
pnpm install
```

## Lancer en développement

```bash
pnpm tauri dev
```

## Build

```bash
pnpm build          # frontend uniquement
pnpm tauri build    # application complète
```

## Structure

```
src/           Frontend React
src-tauri/     Backend Rust (Tauri)
public/        Ressources statiques
ARCHITECTURE.md  Conception du projet
```

## Licence

À définir.
