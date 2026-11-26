---
name: code-diagram
description: Analyzes the repository as it is (crates, services, ports, Tauri commands, HTTP routes, UI modules) and writes Mermaid architecture and sequence diagrams to docs/diagrams/, in English. Use when asked for code diagrams, an up-to-date architecture overview, or to check the code against ARCHITECTURE.md.
---

# Generate code diagrams

Optional argument: a scope (a crate, `ui/tv`, a flow name). Default: the whole repository.

**Output in English**, even though the rest of the docs are in French (user choice for this skill). Diagrams first, text minimal: the reader should understand the code from the diagrams alone. Every node and arrow comes from a file you read: describe the code as it is, not the target in `ARCHITECTURE.md`.

## 1. Collect facts

Skip `node_modules`, `target`, `dist`, `spikes/`, `ui/shared/src/generated`.

- Crates: internal `path = ...` deps in each `Cargo.toml`; UI packages: `workspace:*` deps.
- Services: `pub struct *Service` in `crates/core/src` and their `new(...)` parameters.
- Ports: `pub trait` injected as `Arc<dyn ...>`, their real `impl` and the fake in `crates/core/src/testing.rs`.
- Wiring: `src-tauri/src/bootstrap.rs` (the source of truth).
- Boundary: `generate_handler![...]`, `invoke("...")` in `ui/*/src`, `UiEvent` / `emit(`, `.route(` in `crates/remote-api`.
- Frontend: `ui/*/src` folders, screens rendered by `App.tsx`, `@om/shared` imports.

Note divergences with `ARCHITECTURE.md` §3 and §8 (missing or undocumented services, ports, routes).

## 2. `docs/diagrams/architecture.md`

```markdown
# Code architecture
<sub>/code-diagram · YYYY-MM-DD · branch @ short-hash</sub>

## Crates
<mermaid>
## Services and ports
<mermaid>
| Port | Real | Fake |
## Boundary
<mermaid>
## Frontend
<mermaid>
## Gaps vs ARCHITECTURE.md
- ...
```

- Crates: `flowchart LR`; flag arrows breaking `src-tauri → {infra, player-mpv, remote-api} → core` in red (`linkStyle`).
- Services `[Rect]`, ports `{{Hexagon}}`, like `ARCHITECTURE.md` §3.
- Boundary: UI client method → command/route → service; `UiEvent` emitter → UI listener.
- Mark empty or unused things in the node label (`remote-api (empty)`), not in prose.
- At most one short sentence under a diagram, only when it shows something the diagram cannot. The only bullets allowed are in the gaps list.
- About 25 nodes per diagram; split beyond that.

## 3. `docs/diagrams/sequences.md`

Same header. One `sequenceDiagram` per flow **that exists in the code** (startup, a read and a write command, input from keyboard and `UiBridge`, anything reaching an HTTP route or an external system). Real participant names, `alt` for error branches present in the code. Under each diagram, one line: `Files: path:line, path:line`. End with a `Planned, not implemented` list of one-liners pointing to `ARCHITECTURE.md` sections.

## 4. Finish

- Mermaid: quote labels with `()`, `:`, `/`, `<`, `>`; no `;` in labels; no raw `<` `>` in sequence messages; never reference a composite-state child from outside it.
- Validate with `npx --no-install mmdc` if available; otherwise say it was not machine-checked.
- Overwrite the files; do not commit.
- Reply in at most 5 lines: files written, divergences found, what could not be determined.
