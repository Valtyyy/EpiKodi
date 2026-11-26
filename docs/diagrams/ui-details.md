# TV UI details

<sub>/code-diagram · 2026-11-26 · feat/epic-04-4-input-action @ 820619a + uncommitted #30 work</sub>

Colors and sizes: [visual-identity.md](visual-identity.md). `ui/remote` only shows a placeholder page.

## Window

```mermaid
flowchart LR
    win["Tauri window 'main'"] --> size["1280 × 720 at launch<br/>min 960 × 540"]
    win --> fs["not fullscreen yet"]
    win --> deco["title bar = interface.window_decorations<br/>applied live"]
    win --> body["html, body: surface fill, content text 24 px"]
```

## Screen flow

```mermaid
stateDiagram-v2
    [*] --> Theme: main.tsx
    Theme --> Rejected: contrast below 4.5
    Rejected --> [*]
    Theme --> Loading: theme applied
    Loading --> Error: getSettings() fails
    Loading --> Wizard: setup.first_run_completed false
    Loading --> Settings: wizard done

    state Wizard {
        [*] --> Source
        Source --> TMDB: Next or Skip step
        TMDB --> Remote: Next or Skip step
        Remote --> [*]: Finish
    }
    Wizard --> Settings: completion saved (Finish or Skip wizard)
    Wizard --> SaveFailed: write refused
    SaveFailed --> Settings: Continue

    Settings --> Keyboard: Enter, Replace or text field
    Keyboard --> Settings: Submit or Cancel (Esc)

    Loading: Loading
    Error: Error message (danger)
    Source: Add a source (placeholder)
    TMDB: TMDB key (optional)
    Remote: Remote address (placeholder)
    SaveFailed: Save failed
    Keyboard: Virtual keyboard dialog
```

## Layouts

```mermaid
flowchart TB
    subgraph settings["Settings · centered, max 1024 px, padding 48 px"]
        s1["h1 Settings · 48 px"]
        s2["warnings panel (if any) · surface-raised, danger 20 px"]
        s3["section per category · h2 32 px + bottom rule"]
        s4["fields"]
        s5["Secrets section · secret fields"]
        s6["hidden aria-live region"]
        s1 --> s2 --> s3 --> s4 --> s5 --> s6
    end
    subgraph wizard["Wizard · centered, max 896 px, padding 48 px"]
        w1["Step n of 3 · 20 px muted"]
        w2["h1 step title · 48 px, takes focus, no outline"]
        w3["step content"]
        w4["Skip step · Next / Finish"]
        w5["Skip wizard"]
        w1 --> w2 --> w3 --> w4 --> w5
    end
    subgraph dialog["Keyboard dialog · fullscreen overlay, surface, z-50"]
        k1["prompt · 32 px"]
        k2["input box · surface-raised, min 120 px, mono 32 px<br/>accent caret, dots for secrets"]
        k3["key grid · gap 18 px"]
        k1 --> k2 --> k3
    end
```

## Setting field

```mermaid
flowchart TB
    l["label · 24 px"] --> d["description · 20 px muted (optional)"]
    d --> c["control (by descriptor.kind)"]
    c --> a["apply moment · 20 px muted"]
    a --> k["Locked badge + reason (if env-locked)"]
    k --> e["error · 20 px danger (if refused)"]
    c --> bool["bool · switch On / Off<br/>OK toggles"]
    c --> int["int · ◀ 30000 ms ▶ (range, step)<br/>Left/Right −/+ one step"]
    c --> choice["choice · ◀ info ▶<br/>Left/Right cycle, OK next"]
    c --> txt["text · value or Empty<br/>OK opens keyboard"]
```

Apply moments shown: Applied immediately, Next operation, Next playback, After verification (French labels in the UI).

## Control and key states

```mermaid
stateDiagram-v2
    [*] --> Rest
    Rest --> Focused: focus
    Focused --> Rest: blur
    Rest --> Locked: aria-disabled
    Rest --> Invalid: aria-invalid
    Rest --> Pressed: aria-pressed (Shift)
    Rest: Rest · surface-raised, content 24 px, radius 8 px, min height 96 px
    Focused: Focused · accent fill, accent-content text, 4 px focus outline
    Locked: Locked · 70 percent opacity, action ignored
    Invalid: Invalid · danger error under the field
    Pressed: Pressed · content-muted fill, surface text
```

## Secret field (write-only)

```mermaid
flowchart LR
    lbl["TMDB API key"] --> st["state text only<br/>Configured · Not configured · Set by env var<br/>Keyring locked · Keyring unavailable"]
    st --> btn["Enter or Replace · Clear<br/>disabled unless state is ok"]
    btn --> err["error · danger 20 px"]
```

## Virtual keyboard

```mermaid
flowchart TB
    subgraph letters["Letters"]
        direction TB
        l1["q w e r t y u i o p"] --- l2["a s d f g h j k l"] --- l3["⇧ z x c v b n m ⌫"] --- l4["123 · #+= · space ×4 · Cancel · Submit"]
    end
    subgraph digits["Digits"]
        direction TB
        d1["1 2 3 4 5 6 7 8 9 0"] --- d2[". , - _ @"] --- d3["⌫"] --- d4["ABC · #+= · space · Cancel · Submit"]
    end
    subgraph symbols["Symbols"]
        direction TB
        y1[". - _ @ : / ? ! #"] --- y2["% & + = ( ) [ ] ~"] --- y3[", ; ' &quot; * \ | ⌫"] --- y4["ABC · 123 · space · Cancel · Submit"]
    end
    letters ~~~ digits ~~~ symbols
```

Keys are at least 96 × 96 px. Each keystroke is announced (only "character typed" for secrets); Shift stays pressed until the next character.

## Focus and navigation

```mermaid
flowchart LR
    key["arrows, Enter, Esc<br/>or remote action"] --> w{"focused widget<br/>uses the key?"}
    w -- "yes" --> own["useRovingGrid: next row/col, no wrap<br/>stepper/choice: next value<br/>keyboard: Esc cancels"]
    w -- "no, direction" --> sp["spatial navigation<br/>nearest data-nav item"]
    w -- "no, Enter" --> click["click focused element"]
    own --> vis["focus always visible<br/>except wizard step title"]
    sp --> vis
    dlg["keyboard dialog"] --> trap["Tab trapped inside<br/>focus returns to opener"]
```
