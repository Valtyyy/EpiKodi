import type { SecretKey, SecretStatus, SettingsPatch, SettingsSnapshot, UiEvent } from "../index";

/** Everything the UIs may ask of the backend. Rejections are `AppError` objects. */
export interface ApiClient {
  getSettings(): Promise<SettingsSnapshot>;
  updateSettings(patch: SettingsPatch): Promise<SettingsSnapshot>;
  /** Write-only: the value goes in, only a status comes out (C7). */
  setSecret(key: SecretKey, value: string): Promise<SecretStatus>;
  clearSecret(key: SecretKey): Promise<SecretStatus>;
  /** Events pushed by the backend (`UiBridge`). Returns the unsubscribe function. */
  onUiEvent(listener: (event: UiEvent) => void): () => void;
}
