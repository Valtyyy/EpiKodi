import type {
  InputAction,
  SecretKey,
  SecretStatus,
  SettingsPatch,
  SettingsSnapshot,
  UiEvent,
} from "../index";

/**
 * What both the TV and the phone remote can ask of the backend, so a component written
 * against it runs on both (ARCHITECTURE.md section 6). Rejections are `AppError` objects.
 */
export interface ApiClient {
  /** Same effect as a living-room key, from any device (`InputService`). */
  sendInput(action: InputAction): Promise<void>;
}

/**
 * TV-only operations. The control API exposes no settings route (section 8), so the
 * remote's client cannot implement them: a phone component calling them does not compile.
 */
export interface TvApiClient extends ApiClient {
  getSettings(): Promise<SettingsSnapshot>;
  updateSettings(patch: SettingsPatch): Promise<SettingsSnapshot>;
  /** Write-only: the value goes in, only a status comes out (C7). */
  setSecret(key: SecretKey, value: string): Promise<SecretStatus>;
  clearSecret(key: SecretKey): Promise<SecretStatus>;
  /** Events pushed by the backend (`UiBridge`). Returns the unsubscribe function. */
  onUiEvent(listener: (event: UiEvent) => void): () => void;
}

export function isTvApiClient(client: ApiClient): client is TvApiClient {
  return "getSettings" in client;
}
