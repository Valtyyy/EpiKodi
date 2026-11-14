import type { ApplyMoment, SecretStatus } from "@om/shared";

export const APPLY_LABEL: Record<ApplyMoment, string> = {
  immediate: "Appliqué immédiatement",
  next_operation: "À la prochaine opération",
  next_playback: "À la prochaine lecture",
  verified_rollback: "Après vérification",
};

export const SECRET_LABEL: Record<string, string> = {
  tmdb_api_key: "Clé API TMDB",
};

export function secretLabel(key: string): string {
  return SECRET_LABEL[key] ?? key;
}

/** Text describing a secret's state. Never contains the secret itself. */
export function secretStateText(s: SecretStatus): string {
  switch (s.state) {
    case "env_locked":
      return `Imposée par la variable OPEN_MOSAIK_${s.key.toUpperCase()}`;
    case "keyring_locked":
      return "Trousseau verrouillé";
    case "unavailable":
      return s.message ? `Trousseau indisponible : ${s.message}` : "Trousseau indisponible";
    case "ok":
      return s.configured ? "Configurée" : "Non configurée";
  }
}
