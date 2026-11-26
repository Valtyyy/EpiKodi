import type {
  AppError,
  SecretKey,
  SecretStatus,
  SettingEntry,
  SettingValue,
  SettingsSnapshot,
  UiEvent,
} from "../index";
import type { ApiClient } from "./client";

function invalid(field: string, message: string): AppError {
  return { code: "invalid", message, field };
}

function validate(entry: SettingEntry, value: SettingValue): AppError | null {
  const { key, kind } = entry.descriptor;
  if (entry.locked_reason !== undefined || entry.source === "env") {
    return invalid(key, entry.locked_reason ?? "Réglage imposé par l'environnement");
  }
  if (kind.type !== value.type) return invalid(key, "Type de valeur incorrect");
  if (kind.type === "int" && value.type === "int") {
    if (!Number.isInteger(value.value) || value.value < kind.min || value.value > kind.max) {
      return invalid(key, `La valeur doit être comprise entre ${kind.min} et ${kind.max}`);
    }
  }
  if (kind.type === "choice" && value.type === "choice" && !kind.options.includes(value.value)) {
    return invalid(key, "Choix inconnu");
  }
  if (kind.type === "text" && value.type === "text" && value.value.length > kind.max_len) {
    return invalid(key, `Au plus ${kind.max_len} caractères`);
  }
  return null;
}

/**
 * In-memory ApiClient mimicking the Rust semantics (bounds, env locks, write-only
 * secrets) closely enough for component tests. Secret values are kept in a private
 * map for inspection by tests only and are never part of any returned object.
 */
export function createFakeApiClient(initial: SettingsSnapshot) {
  const snapshot: SettingsSnapshot = structuredClone(initial);
  const secretValues = new Map<SecretKey, string>();
  const uiListeners = new Set<(event: UiEvent) => void>();

  const snap = () => structuredClone(snapshot);
  const setStatus = (key: SecretKey, patch: Partial<SecretStatus>): SecretStatus => {
    const current = snapshot.secrets.find((s) => s.key === key);
    if (!current) throw { code: "not_found", message: "Secret inconnu", field: key } as AppError;
    Object.assign(current, patch);
    return structuredClone(current);
  };
  const lockError = (key: SecretKey): AppError | null => {
    const s = snapshot.secrets.find((x) => x.key === key);
    if (s?.state === "env_locked") return invalid(key, "Secret imposé par l'environnement");
    return null;
  };

  const client: ApiClient & {
    _secretValue(key: SecretKey): string | undefined;
    /** Plays the backend's part: sends an event as `UiBridge` would. */
    _emitUiEvent(event: UiEvent): void;
  } = {
    getSettings: async () => snap(),
    updateSettings: async (patch) => {
      for (const change of patch.changes) {
        const entry = snapshot.entries.find((e) => e.descriptor.key === change.key);
        if (!entry) throw invalid(change.key, "Réglage inconnu");
        const err = validate(entry, change.value);
        if (err) throw err;
      }
      // Validated as a whole first: a patch is applied atomically.
      for (const change of patch.changes) {
        const entry = snapshot.entries.find((e) => e.descriptor.key === change.key)!;
        entry.value = structuredClone(change.value);
        entry.source = "stored";
      }
      return snap();
    },
    setSecret: async (key, value) => {
      const err = lockError(key);
      if (err) throw err;
      if (value.length === 0) throw invalid(key, "La valeur est vide");
      secretValues.set(key, value);
      return setStatus(key, {
        configured: true,
        source: "keyring",
        state: "ok",
        message: undefined,
      });
    },
    clearSecret: async (key) => {
      const err = lockError(key);
      if (err) throw err;
      secretValues.delete(key);
      return setStatus(key, { configured: false, source: "none", state: "ok", message: undefined });
    },
    onUiEvent: (listener) => {
      uiListeners.add(listener);
      return () => uiListeners.delete(listener);
    },
    _secretValue: (key) => secretValues.get(key),
    _emitUiEvent: (event) => uiListeners.forEach((l) => l(event)),
  };
  return client;
}
