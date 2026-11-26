# Visual identity

<sub>/code-diagram · 2026-11-26 · feat/epic-04-4-input-action @ 820619a + uncommitted #30 work</sub>

## Theme pipeline

```mermaid
flowchart LR
    json["themes/default/theme.json<br/>8 roles, #rrggbb"] --> parse["parseTheme()<br/>schema + contrast"]
    parse -- "one pair below 4.5:1" --> ko["whole theme rejected"]
    parse -- "valid" --> apply["applyTheme()<br/>--om-* on :root"]
    apply --> inline["theme.css @theme inline<br/>--color-surface: var(--om-surface)"]
    inline --> classes["bg-surface, text-content,<br/>outline-focus..."]
    classes --> tv["ui/tv"]
    classes --> remote["ui/remote"]
    tw["Tailwind default palette"] -. "--color-*: initial" .-x classes
```

## Palette (default theme)

```mermaid
flowchart LR
    subgraph bg["Backgrounds"]
        surface["surface<br/>#0e1116<br/>screen"]
        raised["surface-raised<br/>#1b212b<br/>fields, keys, panels"]
    end
    subgraph text["Text"]
        content["content<br/>#f3f5f8<br/>main text"]
        muted["content-muted<br/>#aab2c0<br/>secondary text, rules, badge"]
    end
    subgraph act["Accent and focus"]
        accent["accent<br/>#6cb4ff<br/>focused fill"]
        accentc["accent-content<br/>#08111d<br/>text on accent"]
        focus["focus<br/>#ffd24a<br/>focus outline"]
    end
    subgraph st["States"]
        danger["danger<br/>#ff8a80<br/>errors"]
    end
    style surface fill:#0e1116,color:#f3f5f8,stroke:#aab2c0
    style raised fill:#1b212b,color:#f3f5f8,stroke:#aab2c0
    style content fill:#f3f5f8,color:#0e1116,stroke:#0e1116
    style muted fill:#aab2c0,color:#0e1116,stroke:#0e1116
    style accent fill:#6cb4ff,color:#08111d,stroke:#08111d
    style accentc fill:#08111d,color:#6cb4ff,stroke:#6cb4ff
    style focus fill:#ffd24a,color:#0e1116,stroke:#0e1116
    style danger fill:#ff8a80,color:#0e1116,stroke:#0e1116
```

## Contrast (minimum 4.5:1)

```mermaid
flowchart LR
    content["content"] -- "17.32" --> surface["surface"]
    content -- "14.80" --> raised["surface-raised"]
    muted["content-muted"] -- "8.86" --> surface
    muted -- "7.57" --> raised
    accent["accent"] -- "8.65" --> surface
    accentc["accent-content"] -- "8.67" --> accentbg["accent"]
    focus["focus"] -- "13.12" --> surface
    focus -- "11.22" --> raised
    danger["danger"] -- "8.28" --> surface
    danger -- "7.08" --> raised
    accent2["accent<br/>(keyboard caret)"] -. "7.40 unchecked" .-> raised2["surface-raised"]
    surface2["surface<br/>(Locked badge, Shift pressed)"] -. "8.86 unchecked" .-> muted2["content-muted"]
    linkStyle 10,11 stroke:#ff8a80
    style surface fill:#0e1116,color:#f3f5f8
    style surface2 fill:#0e1116,color:#f3f5f8
    style raised fill:#1b212b,color:#f3f5f8
    style raised2 fill:#1b212b,color:#f3f5f8
    style content fill:#f3f5f8,color:#0e1116
    style muted fill:#aab2c0,color:#0e1116
    style muted2 fill:#aab2c0,color:#0e1116
    style accent fill:#6cb4ff,color:#08111d
    style accent2 fill:#6cb4ff,color:#08111d
    style accentbg fill:#6cb4ff,color:#08111d
    style accentc fill:#08111d,color:#6cb4ff
    style focus fill:#ffd24a,color:#0e1116
    style danger fill:#ff8a80,color:#0e1116
```

Solid: `CONTRAST_PAIRS`, checked on every load. Red dashed: used by the UI but missing from `CONTRAST_PAIRS`, so another theme would not be checked on them.

## Type scale (TV)

```mermaid
flowchart TB
    xl["text-tv-xl · 48 px · screen title h1"]
    lg["text-tv-lg · 32 px · section h2, input box, keyboard prompt"]
    md["text-tv-md · 24 px · body default, labels, controls, keys"]
    sm["text-tv-sm · 20 px · descriptions, apply moment, field errors, progress"]
    none["text-xs...9xl removed · nothing under 20 px"]
    xl --> lg --> md --> sm
    sm -. "floor" .- none
```

Font: WebView default sans-serif; `font-mono` for the virtual keyboard input box.

## Spacing and shapes

```mermaid
flowchart LR
    unit["--spacing 6 px<br/>(Tailwind default x1.5)"] --> p8["p-8 · 48 px<br/>screen padding"]
    unit --> g6["gap-6 / mb-6 · 36 px<br/>between blocks"]
    unit --> g3["gap-3 · 18 px<br/>between buttons, keys"]
    unit --> h16["min-h-16 · 96 px<br/>control and key height"]
    radius["radius"] --> lg8["rounded-lg · 8 px<br/>controls, keys, panels"]
    radius --> r4["rounded · 4 px<br/>badge"]
    width["max width"] --> w5["max-w-5xl · 1024 px<br/>settings, keyboard"]
    width --> w4["max-w-4xl · 896 px<br/>wizard"]
```

## Focus indicator (`focusRing`)

```mermaid
flowchart LR
    rest["Rest<br/>surface-raised fill, content text"] -- "focus:" --> foc["Focused<br/>4 px focus outline, 2 px offset<br/>accent fill, accent-content text"]
    style rest fill:#1b212b,color:#f3f5f8,stroke:#1b212b
    style foc fill:#6cb4ff,color:#08111d,stroke:#ffd24a,stroke-width:4px
```

`focus:`, never `focus-visible:`: focus moved by script or by the remote may not match `:focus-visible`.

## Enforcement

```mermaid
flowchart LR
    r1["semantic colors only"] --> t1["--color-*: initial<br/>+ ESLint no-unknown-classes"]
    r2["no text under 20 px"] --> t2["--text-*: initial<br/>+ ESLint no-unknown-classes"]
    r3["focus: not focus-visible:"] --> t3["ESLint no-restricted-classes"]
    r4["contrast 4.5:1"] --> t4["parseTheme() at load<br/>+ theme tests in CI"]
    r5["stable class order"] --> t5["prettier-plugin-tailwindcss"]
```
