---
paths:
  - "crates/player-mpv/**"
  - "src-tauri/**"
  - "spikes/video-overlay/**"
---

# Video playback, GTK and threads

Read ARCHITECTURE.md §5, ADR-0002 and `docs/spikes/2.1-composition-video.md` before any change.

- Composition: transparent WebKitGTK webview on top of a `GtkGLArea` rendered by libmpv (plan A, validated on AMD, Wayland and XWayland; plan B as fallback). Do not change the composition mode without updating ADR-0002.
- Require `LC_NUMERIC=C` before initializing libmpv. Follow the creation order described in the spike (render deadlock risk).
- Threads:
  - **Main GTK**: no blocking call, no libmpv call outside rendering.
  - **Tokio runtime**: no long computation or synchronous call.
  - **Dedicated `mpv` thread**: the only one talking to the mpv client API; commands in via channel, events out via channel.
  - **`spawn_blocking`**: audio tags, local disk walking, bounded by a semaphore.
- SMB files are read through the local stream gateway (127.0.0.1 only, per-session token, HTTP Range), never through a system mount (ADR-0004).
- The window is created in `setup` (fullscreen, undecorated), not by `tauri.conf.json`. Strict CSP. Minimal Tauri capabilities: no `fs` or `shell` plugin.
- Do not assume a single GPU or a Wayland session: Intel, NVIDIA and native X11 are not measured yet.
