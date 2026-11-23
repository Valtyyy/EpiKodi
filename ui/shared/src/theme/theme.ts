import { contrastRatio } from "./contrast";

/** Semantic color roles; each one becomes `--om-<role>` then the Tailwind color `<role>`. */
export const THEME_COLORS = [
  "surface",
  "surface-raised",
  "content",
  "content-muted",
  "accent",
  "accent-content",
  "focus",
  "danger",
] as const;

export type ThemeColor = (typeof THEME_COLORS)[number];

export interface Theme {
  name: string;
  colors: Record<ThemeColor, string>;
}

export const MIN_CONTRAST = 4.5;

/**
 * Foreground/background pairs the UI actually combines. A pair added to the UI
 * must be added here, otherwise a theme could make it unreadable unnoticed.
 */
export const CONTRAST_PAIRS: readonly (readonly [ThemeColor, ThemeColor])[] = [
  ["content", "surface"],
  ["content", "surface-raised"],
  ["content-muted", "surface"],
  ["content-muted", "surface-raised"],
  ["accent", "surface"],
  ["accent-content", "accent"],
  ["focus", "surface"],
  ["focus", "surface-raised"],
  ["danger", "surface"],
  ["danger", "surface-raised"],
];

export type ThemeResult = { ok: true; theme: Theme } | { ok: false; errors: string[] };

const HEX_COLOR = /^#[0-9a-fA-F]{6}$/;

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

/**
 * Validates an untrusted theme (same rules as themes/theme.schema.json) then checks
 * every pair of CONTRAST_PAIRS against MIN_CONTRAST. A theme is never partially applied:
 * any error rejects it whole, with one message per problem.
 */
export function parseTheme(raw: unknown): ThemeResult {
  if (!isRecord(raw)) return { ok: false, errors: ["Le thème doit être un objet JSON."] };

  const errors: string[] = [];
  for (const key of Object.keys(raw)) {
    if (key !== "$schema" && key !== "name" && key !== "colors") {
      errors.push(`Propriété inconnue « ${key} ».`);
    }
  }
  if (typeof raw.name !== "string" || raw.name.length === 0) {
    errors.push("« name » doit être un texte non vide.");
  }
  if (!isRecord(raw.colors)) {
    errors.push("« colors » doit être un objet.");
    return { ok: false, errors };
  }

  const colors = raw.colors;
  for (const key of Object.keys(colors)) {
    if (!(THEME_COLORS as readonly string[]).includes(key)) {
      errors.push(`Couleur inconnue « ${key} ».`);
    }
  }
  for (const role of THEME_COLORS) {
    const value = colors[role];
    if (value === undefined) errors.push(`Couleur « ${role} » manquante.`);
    else if (typeof value !== "string" || !HEX_COLOR.test(value)) {
      errors.push(`Couleur « ${role} » invalide : attendu #rrggbb.`);
    }
  }
  // Contrast is only computable once every color is well formed.
  if (errors.length > 0) return { ok: false, errors };

  const theme = { name: raw.name as string, colors: colors as Theme["colors"] };
  for (const [fg, bg] of CONTRAST_PAIRS) {
    const ratio = contrastRatio(theme.colors[fg], theme.colors[bg]);
    if (ratio < MIN_CONTRAST) {
      // Floored so a 4.497 ratio never reads as "4.50:1" next to a 4.5 minimum.
      const shown = (Math.floor(ratio * 100) / 100).toFixed(2);
      errors.push(
        `Contraste insuffisant entre « ${fg} » et « ${bg} » : ${shown}:1, minimum ${MIN_CONTRAST}:1.`,
      );
    }
  }
  return errors.length > 0 ? { ok: false, errors } : { ok: true, theme };
}

/** Exposes the theme as `--om-*` variables, read by `@theme inline` in theme.css. */
export function applyTheme(theme: Theme, root: HTMLElement = document.documentElement): void {
  for (const role of THEME_COLORS) {
    root.style.setProperty(`--om-${role}`, theme.colors[role]);
  }
}
