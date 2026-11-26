# Main flows

<sub>/code-diagram · 2026-11-26 · feat/epic-04-4-input-action @ 820619a + uncommitted #30 work</sub>

## Startup

```mermaid
sequenceDiagram
    participant T as Tauri setup
    participant B as bootstrap
    participant DB as SqliteSettingsRepository
    participant S as SettingsService
    participant W as Window
    participant UI as ui/tv main.tsx

    T->>B: build(data_dir, TauriUiBridge)
    B->>DB: open(open-mosaik.db)
    alt database unavailable
        DB-->>B: AppError
        B->>B: in-memory SQLite + "storage" warning
    else opened
        DB-->>B: repository
    end
    B->>S: load_with_warnings()
    S->>DB: load_all()
    B-->>T: Services
    T->>T: register(): manage() each service
    T->>W: build window (decorations from settings)
    T->>T: task: view.changed() applies decorations live
    W->>UI: load index.html
    UI->>UI: parseTheme(theme.json)
    alt contrast below 4.5:1
        UI-->>UI: throw, nothing rendered
    else valid
        UI->>UI: applyTheme(), render App
    end
```

Files: src-tauri/src/lib.rs:15, src-tauri/src/bootstrap.rs:27, src-tauri/src/bootstrap.rs:87, ui/tv/src/main.tsx:10, ui/shared/src/theme/theme.ts:54

## Read settings

```mermaid
sequenceDiagram
    participant Scr as SettingsScreen
    participant C as TauriApiClient
    participant Cmd as get_settings
    participant S as SettingsService

    Scr->>C: getSettings()
    C->>Cmd: invoke("get_settings")
    Cmd->>S: snapshot()
    S-->>Scr: SettingsSnapshot (entries, secret states, warnings)
    Scr->>Scr: fields from descriptor.kind, focus first
```

Files: ui/tv/src/settings/SettingsScreen.tsx:32, ui/tv/src/api/tauri.ts:10, src-tauri/src/commands/settings.rs:8

## Update a setting

```mermaid
sequenceDiagram
    participant F as SettingField
    participant Scr as SettingsScreen
    participant C as TauriApiClient
    participant Cmd as update_settings
    participant S as SettingsService
    participant DB as SettingsRepository
    participant V as SettingsView subscribers

    F->>Scr: onCommit(value)
    Scr->>C: updateSettings(patch)
    C->>Cmd: invoke("update_settings")
    Cmd->>S: update(patch)
    alt env-locked, verified-rollback, out of bounds or wrong type
        S-->>Cmd: AppError validation (field, suggestion)
    else valid
        S->>DB: save_many(rows), atomic
        alt write fails
            DB-->>S: AppError, memory unchanged
        else written
            S->>V: send_replace(settings)
            Cmd->>S: snapshot()
        end
    end
    alt error
        Cmd-->>Scr: AppError
        Scr->>F: error under the field
    else ok
        Cmd-->>Scr: SettingsSnapshot
        Scr->>Scr: aria-live "label saved"
    end
```

Files: ui/tv/src/settings/SettingsScreen.tsx:41, src-tauri/src/commands/settings.rs:13, crates/core/src/settings/service.rs:311, crates/infra/src/settings_repository.rs:39

## Input: keyboard and UiBridge

```mermaid
sequenceDiagram
    actor U as User
    participant In as InputService
    participant Br as TauriUiBridge
    participant TV as TvInput
    participant El as Focused element
    participant W as Widget handler
    participant H as Input handler
    participant N as Spatial navigation

    alt living-room keyboard
        U->>El: keydown
    else phone remote (caller: route #80)
        In->>Br: send(UiEvent::Input)
        Br->>TV: emit("ui-event")
        TV->>H: dispatch(action)
        H->>El: synthetic keydown (ACTION_KEY)
    end
    El->>W: React propagation
    alt widget uses the key
        W->>W: preventDefault(), own action
    else key free
        El->>H: keydown on document
        alt direction
            H->>N: move(dir)
            N->>El: focus() nearest data-nav item
        else ok
            H->>El: click()
        else back
            H-->>H: no default yet
        end
    end
```

Files: crates/core/src/input.rs:35, src-tauri/src/ui_bridge.rs:20, ui/tv/src/input/TvInput.tsx:16, ui/shared/src/input/inputHandler.ts:58, ui/shared/src/focus/spatialNavigation.ts:69

## Planned, not implemented

- Phone pairing, `POST /api/v1/pair`: ARCHITECTURE.md §8.
- Play a movie from the phone, `POST /player/load` → PlaybackService → PlayerEngine: §8.
- SMB playback through the local stream gateway: §5.3.
- Source scan and metadata (TMDB, MusicBrainz): §4, §5.
- Live state, `WS /api/v1/events`: §8.
