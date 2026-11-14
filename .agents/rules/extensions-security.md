---
paths:
  - "crates/ext-host/**"
  - "crates/ext-protocol/**"
  - "crates/remote-api/**"
  - "extensions/**"
  - "spikes/ext-host-sandbox/**"
---

# Extensions and remote API

Read ARCHITECTURE.md §7, §8, §12 before changing these areas.

## Extensions
- **One `ext-host` process per extension** (QuickJS via `rquickjs`), communicating over JSON-RPC on stdio only. No `dlopen`, no Tauri plugin, no binary reload (C2, C3).
- Three lines of defense: process sandbox (landlock, seccomp), permission broker in the main process, manifest validation. Every capability an extension requests goes through the broker; deny by default.
- Manifest, permission and message types live in `ext-protocol` (types only, no logic).
- When installing an archive: guard against zip slip and decompression bombs.
- Extension widgets are **declarative** (no HTML/JS supplied by the extension).
- Tests use real `ext-host` processes and hostile fixtures (infinite loop, memory leak, forbidden file/network access).

## Control API (axum)
- Same service instances as the Tauri commands (`State<Service>` on the axum side).
- Default listen address `0.0.0.0:8420`, changeable hot. Port in use: never silently pick another one.
- Pairing: 6-digit code, 2 min, single use, 5 attempts; 256-bit token, only the fingerprint is stored; `SameSite=Strict` cookie and `Origin` check.
- Plain HTTP on the LAN is an accepted limitation (ADR to come). Never present it as secure.
- Route tests through `tower::oneshot`, no open port.
- No secret in logs or API responses (write-only fields).
