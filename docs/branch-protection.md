# Protection de la branche `main`

Ce document liste les vérifications CI requises avant un merge sur `main` et la façon de les imposer. La configuration se fait côté GitHub, elle n'est pas appliquée automatiquement par le dépôt.

## Vérifications requises

Les noms ci-dessous sont ceux des jobs de `.github/workflows/ci.yml` (champ `name:`). Ne pas les renommer sans mettre à jour la protection.

| Check                | Contenu                                                           |
| -------------------- | ----------------------------------------------------------------- |
| `Build frontend`     | `pnpm build` de `ui/tv`, `ui/remote`, `ui/shared`                 |
| `Lint`               | `cargo fmt --check`, Prettier, ESLint, `tsc`                      |
| `Clippy`             | `cargo clippy --workspace --all-targets --all-features -D warnings` |
| `Tests + types`      | `pnpm test` (Rust + Vitest, hors ligne) puis `scripts/check-types.sh` (bindings ts-rs à jour) |
| `Commitlint`         | Conventional Commits sur tous les commits de la PR                |

`Clippy` et `Tests + types` dépendent de `Build frontend` (artefact `frontend-dist`). Les tests et la vérification des types partagent un seul job pour éviter un second `apt-get` et une seconde compilation.

## Via l'interface

1. Dépôt GitHub, Settings, Branches, Add branch ruleset (ou Add classic branch protection rule).
2. Cible : `main`.
3. Cocher : Require a pull request before merging, Require status checks to pass, Require branches to be up to date before merging.
4. Ajouter les cinq checks du tableau ci-dessus.
5. Optionnel : Block force pushes, Do not allow bypassing the above settings.

Un check n'apparaît dans la recherche qu'après avoir tourné au moins une fois sur le dépôt.

## Via la CLI (à lancer manuellement, ne l'est pas par le projet)

Remplacer `OWNER/REPO`. Nécessite les droits d'administration.

```bash
gh api -X PUT repos/OWNER/REPO/branches/main/protection --input - <<'JSON'
{
  "required_status_checks": {
    "strict": true,
    "contexts": [
      "Build frontend",
      "Lint",
      "Clippy",
      "Tests + types",
      "Commitlint"
    ]
  },
  "enforce_admins": true,
  "required_pull_request_reviews": {
    "required_approving_review_count": 1
  },
  "restrictions": null,
  "allow_force_pushes": false,
  "allow_deletions": false
}
JSON
```

## Hooks locaux

`pnpm install` installe lefthook (script `prepare`) : `pre-commit` (Prettier, ESLint, `cargo fmt --check`) et `commit-msg` (commitlint). La CI refait les mêmes contrôles, les hooks ne sont qu'un confort.
