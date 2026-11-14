---
paths:
  - "ui/**"
  - "eslint.config.js"
  - "*.json"
---

# Frontend rules

- Three pnpm packages: `ui/tv` (10-foot interface, fullscreen, remote/keyboard navigation), `ui/remote` (mobile, embedded in the binary), `ui/shared` (generated types, API client, common components). `ui/shared` depends on neither `tv` nor `remote`.
- `ui/shared/src/generated` is produced by ts-rs: read-only.
- The remote is served over **plain HTTP**: do not use `crypto.subtle`, Service Workers or the Clipboard API.
- No `<video>`/`<audio>`: video is drawn by libmpv under the webview; the TV UI must stay transparent on top during playback.
- Escaped React rendering only; no `dangerouslySetInnerHTML` for external or extension metadata (strict CSP, no `unsafe-inline` for scripts).
- Styles: Tailwind 4 + theme CSS variables (the theme stays independent of the code).
- Accessibility: large touch targets, visible focus, sufficient contrast, screen-reader labels (Orca is part of the final demo). Component tests with Vitest + Testing Library + axe.
- No external network call from the UI: everything goes through Tauri IPC (TV) or the control API (remote).
- Build/test a package only through its scripts: `pnpm --filter <tv|remote|shared> <script>`. The `ui/remote` build must precede any Rust compilation.
- Prettier (`pnpm fmt`), ESLint (`pnpm lint`) and `pnpm typecheck` must pass.
