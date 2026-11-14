# Spike 2.1 - video-overlay

Minimal Tauri 2 app that composes a transparent WebKitGTK webview over a libmpv `GtkGLArea`
(ARCHITECTURE.md section 5.2). Standalone crate (empty `[workspace]` table, excluded from the
root workspace). Report: `docs/spikes/2.1-composition-video.md`.

## Prerequisites (Ubuntu 24.04)

```bash
sudo apt install libmpv-dev libwebkit2gtk-4.1-dev libgtk-3-dev ffmpeg
```

## Test video (git-ignored)

```bash
mkdir -p media
ffmpeg -y -f lavfi -i testsrc2=size=1920x1080:rate=30 -t 60 \
  -c:v libx264 -pix_fmt yuv420p -preset veryfast -profile:v high -level 4.1 media/test-1080p30.mp4
```

## Build

```bash
cargo build            # debug binary: target/debug/video-overlay-spike
cargo clippy
```

## Run

| Mode | Command |
|---|---|
| Plan A (overlay), Wayland | `cargo run` |
| Plan A, X11 (XWayland) | `GDK_BACKEND=x11 cargo run` |
| Plan A, DMABUF renderer off | `WEBKIT_DISABLE_DMABUF_RENDERER=1 cargo run` |
| Plan B (stack + OSD), Wayland | `cargo run -- --plan b` (click "Play"; Space, arrows, f, Esc relayed to mpv) |
| Plan A without webview (baseline) | `cargo run -- --no-overlay` |

Options: `--video <path>`, `--measure [--duration N]`, `--out <json>`, `--screenshots <dir>`.

## Measurement

`--measure` plays the video with `hwdec=auto-safe`, samples once per second
(`frame-drop-count`, `decoder-frame-drop-count`, `estimated-frame-number`, `hwdec-current`,
CPU from `/proc/self/stat`, RSS), injects synthetic key events into the GTK main loop, does
10 fullscreen/windowed cycles (20 toggles, 750 ms apart, starting 6 s into playback), prints a
JSON summary and exits on its own. Use a duration of at least 25 s (default 30) so all toggles complete.

```bash
./run-measure.sh a-wayland --                                   # logs in target/logs/a-wayland.{json,log}
./run-measure.sh a-x11 GDK_BACKEND=x11 --
./run-measure.sh a-wayland-nodmabuf WEBKIT_DISABLE_DMABUF_RENDERER=1 --
./run-measure.sh b-wayland -- --plan b
python3 summarize.py                                            # per-phase table from all JSON files
```

The runs are under `timeout 120` and never write to the terminal.

## Notes

- Screenshots are produced by drawing the GTK widget tree (GLArea texture + webview) into a
  cairo surface. They prove that both layers composite inside GTK, not what the compositor
  finally shows on screen (no grim / portal available in the test session).
- `GDK_BACKEND=x11` under a Wayland session runs through XWayland, not a native X11 session.
