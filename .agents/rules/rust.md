---
paths:
  - "crates/**/*.rs"
  - "src-tauri/**/*.rs"
  - "**/Cargo.toml"
---

# Rust rules

## Crate dependencies
- Among application crates, `core` only depends on `ext-protocol` (types). Never add Tauri, gtk, libmpv, sqlx, reqwest or smb to `crates/core/Cargo.toml`: the isolation is enforced by the compiler.
- Shared dependencies go in `[workspace.dependencies]` of the root `Cargo.toml`; crates reference them with `workspace = true`.
- `spikes/` is excluded from the workspace: do not attach it, do not import its code.

## Services and ports
- Service = concrete `struct`, `Clone` (a handle on `Arc` shared state), built by `new()` with its dependencies. No trait per service.
- Port = `trait` for an external boundary (SQLite, SMB, TMDB, mpv, clock, keyring, extension runtime). Injected as `Arc<dyn Port>`. Each port has an in-memory fake in `testing.rs` for tests.
- Clock: always through the `Clock` port, never `SystemTime::now()` in a service (pairing codes, resume).
- No global state (`static`, `lazy`) for services.
- Only in `bootstrap.rs`: instantiate concrete implementations and wire them.

## Errors and robustness
- Typed errors in `core` (`thiserror`); no `unwrap`/`expect` outside tests and documented invariants.
- An async Tauri command returns `Result<T, E>` with `E: Serialize`.
- Events to the UI go through the `UiBridge` port, not `AppHandle::emit` directly inside a service.

## Types exported to TypeScript
- Derive `ts_rs::TS` behind the `ts` feature (`#[cfg_attr(feature = "ts", derive(ts_rs::TS))]`). Output: `ui/shared/src/generated`.
- Regenerate with `pnpm gen:types` and commit.

## Tests
- Unit tests in the module, integration tests in `tests/`. No network, no files outside a temp directory.
- Scan, resume and expiry tests use fake ports (in-memory tree with programmable failures, manual clock).
