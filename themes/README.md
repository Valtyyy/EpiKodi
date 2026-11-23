# Thèmes

Un thème décrit les couleurs de l'interface (télé et télécommande) dans un fichier JSON, sans toucher
au code. Le thème par défaut est `default/theme.json`.

## Format

Le format est défini par le JSON Schema [`theme.schema.json`](theme.schema.json). La clé `$schema`
active la complétion et la validation dans les éditeurs.

```json
{
  "$schema": "../theme.schema.json",
  "name": "Par défaut",
  "colors": {
    "surface": "#0e1116",
    "surface-raised": "#1b212b",
    "content": "#f3f5f8",
    "content-muted": "#aab2c0",
    "accent": "#6cb4ff",
    "accent-content": "#08111d",
    "focus": "#ffd24a",
    "danger": "#ff8a80"
  }
}
```

| Rôle | Usage |
|---|---|
| `surface` | Fond de l'écran |
| `surface-raised` | Fond des champs, touches et panneaux |
| `content` | Texte principal |
| `content-muted` | Texte secondaire, bordures, fond d'un badge |
| `accent` | Élément focalisé, texte d'accent |
| `accent-content` | Texte posé sur `accent` |
| `focus` | Contour de l'élément focalisé |
| `danger` | Erreurs |

Règles :

- les huit rôles sont obligatoires, aucune autre clé n'est acceptée ;
- une couleur s'écrit `#rrggbb` : pas de transparence, pas de nom (`red`), pas de forme courte
  (`#fff`), sinon le contraste ne peut pas être calculé.

## Contraste

Au chargement, chaque paire texte / fond utilisée par l'interface doit atteindre **4,5:1**
(WCAG 2.2, critère 1.4.3). Un thème qui échoue sur une seule paire est refusé en entier, avec un
message par paire, par exemple :

```text
Contraste insuffisant entre « content-muted » et « surface-raised » : 2.62:1, minimum 4.5:1.
```

Paires vérifiées (texte, fond) :

| Texte | Fond |
|---|---|
| `content` | `surface`, `surface-raised` |
| `content-muted` | `surface`, `surface-raised` |
| `accent` | `surface` |
| `accent-content` | `accent` |
| `focus` | `surface`, `surface-raised` |
| `danger` | `surface`, `surface-raised` |

La liste fait foi dans `ui/shared/src/theme/theme.ts` (`CONTRAST_PAIRS`). Une nouvelle combinaison
de couleurs dans l'interface doit y être ajoutée.

## Application

`parseTheme()` valide le fichier, `applyTheme()` écrit les couleurs dans les variables CSS `--om-*`
de la page. Tailwind les lit par `@theme inline` (`ui/shared/src/theme.css`) : `bg-surface` vaut
`var(--om-surface)`. Changer de thème revient à réécrire ces variables, sans recompiler.
