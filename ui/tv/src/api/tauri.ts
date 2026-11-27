import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { TvApiClient, UiEvent } from "@om/shared";

/** Same name as `UI_EVENT` in src-tauri/src/ui_bridge.rs. */
const UI_EVENT = "ui-event";

/** The only module allowed to import `@tauri-apps/*` (ESLint `no-restricted-imports`). */
export function createTauriApiClient(): TvApiClient {
  return {
    sendInput: (action) => invoke("send_input", { action }),
    getSettings: () => invoke("get_settings"),
    updateSettings: (patch) => invoke("update_settings", { patch }),
    setSecret: (key, value) => invoke("set_secret", { key, value }),
    clearSecret: (key) => invoke("clear_secret", { key }),
    onUiEvent: (listener) => {
      const unlisten = listen<UiEvent>(UI_EVENT, (e) => listener(e.payload));
      return () => void unlisten.then((stop) => stop());
    },
  };
}
