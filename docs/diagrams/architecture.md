# Code architecture

<sub>/code-diagram · 2026-11-26 · feat/epic-04-4-input-action @ 820619a + uncommitted #30 work</sub>

## Crates

```mermaid
flowchart LR
    subgraph rust["Cargo workspace"]
        tauri["src-tauri"]
        infra["infra"]
        mpv["player-mpv (empty)"]
        api["remote-api (empty)"]
        core["core (om-core)"]
        proto["ext-protocol (empty)"]
        host["ext-host (empty binary)"]
    end
    subgraph front["pnpm workspace"]
        tv["ui/tv"]
        remote["ui/remote"]
        shared["ui/shared"]
    end
    tauri --> infra --> core
    tauri --> mpv --> core
    tauri --> api --> core
    tauri --> core
    core --> proto
    host --> proto
    tv --> shared
    remote --> shared
    core -. "ts-rs types" .-> shared
    tauri -. "serves build" .-> tv
    linkStyle 2,3,4,5,7,8 stroke-dasharray:4
```

Dashed: declared but unused (cargo `unused_dependencies` warning). No arrow breaks the expected direction.

## Services and ports

```mermaid
flowchart LR
    Settings[SettingsService] --> Repo{{SettingsRepository}}
    Settings --> Env{{Environment}}
    Settings --> Secrets{{SecretStore}}
    Settings -- "view()" --> View["SettingsView (read-only)"]
    Input[InputService] --> Bridge{{UiBridge}}
    Ping[PingService] --> Clock{{Clock}}
    Ping --> Bus["EventBus (no subscriber)"]
    Repo -. "Sqlite impl" .-> Clock
```

| Port | Real | Fake |
|---|---|---|
| `SettingsRepository` | `SqliteSettingsRepository` (infra), in-memory SQLite fallback | `MemorySettingsRepository` |
| `Environment` | `ProcessEnvironment` (infra) | `MemoryEnvironment` |
| `SecretStore` | `KeyringSecretStore` (infra) | `MemorySecretStore` |
| `Clock` | `SystemClock` (infra) | `ManualClock` |
| `UiBridge` | `TauriUiBridge` (src-tauri, event `ui-event`) | `RecordingUiBridge` |

## Boundary

```mermaid
flowchart LR
    subgraph client["ui/tv TauriApiClient"]
        gs["getSettings()"]
        us["updateSettings()"]
        ss["setSecret()"]
        cs["clearSecret()"]
        oe["onUiEvent()"]
    end
    subgraph cmds["Tauri commands"]
        c1["get_settings"]
        c2["update_settings"]
        c3["set_secret"]
        c4["clear_secret"]
        c5["ping (no UI caller)"]
    end
    gs --> c1 --> s1["SettingsService.snapshot()"]
    us --> c2 --> s2["SettingsService.update()"]
    ss --> c3 --> s3["SettingsService.set_secret()"]
    cs --> c4 --> s4["SettingsService.clear_secret()"]
    c5 --> p["PingService.ping()"]
    caller["(no caller yet, route #80)"] -.-> i["InputService.send()"] --> b{{UiBridge}} -- "UiEvent::Input" --> oe
    routes["remote-api: no routes"]
```

## Frontend

```mermaid
flowchart TB
    subgraph shared["ui/shared"]
        apiS["api: ApiClient, ApiProvider, useApi, fake"]
        focus["focus: useRovingGrid, spatial nav, focusRing"]
        input["input: createInputHandler"]
        theme["theme: parseTheme, applyTheme, theme.css"]
        gen["generated: 18 ts-rs types"]
    end
    subgraph tv["ui/tv"]
        main["main.tsx"]
        app["App: Root picks the screen"]
        tvin["TvInput"]
        wizard["FirstRunWizard (3 steps)"]
        settings["SettingsScreen, SettingField, SecretField"]
        kb["KeyboardDialog, VirtualKeyboard"]
        tauriApi["api/tauri.ts"]
    end
    remoteApp["ui/remote: placeholder page"]
    main --> theme
    main --> app
    app --> tauriApi --> apiS
    app --> tvin --> focus
    tvin --> input
    app --> wizard --> settings
    app --> settings --> kb
    wizard --> focus
    settings --> focus
    kb --> focus
    settings --> apiS
    wizard --> apiS
    apiS --> gen
```

## Gaps vs ARCHITECTURE.md

- Missing services (§3): Source, Scan, Library, Metadata, Playback, Extension, Pairing, RemoteServer.
- Undocumented service: `PingService` (wiring example) and its `ping` command.
- Missing ports (§3): Media/Source/Progress/Device/Extension repositories, `SourceProvider`, `MetadataProvider`, `ArtworkStore`, `PlayerEngine`, `ExtensionRuntime`.
- Undocumented port: `Environment`.
- Control API (§8): no route, no `PairedDevice`, no WebSocket, no stream gateway.
- UI (§6): no `HttpApiClient` (#31), no TanStack Query.
