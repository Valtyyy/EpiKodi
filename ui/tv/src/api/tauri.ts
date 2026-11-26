import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { ApiClient, UiEvent } from "@om/shared";

/** Same name as `UI_EVENT` in src-tauri/src/ui_bridge.rs. */
const UI_EVENT = "ui-event";

export function createTauriApiClient(): ApiClient {
  return {
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
