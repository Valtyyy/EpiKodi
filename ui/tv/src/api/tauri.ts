import { invoke } from "@tauri-apps/api/core";
import type { ApiClient } from "@om/shared";

export function createTauriApiClient(): ApiClient {
  return {
    getSettings: () => invoke("get_settings"),
    updateSettings: (patch) => invoke("update_settings", { patch }),
    setSecret: (key, value) => invoke("set_secret", { key, value }),
    clearSecret: (key) => invoke("clear_secret", { key }),
  };
}
