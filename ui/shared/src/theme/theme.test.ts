// @vitest-environment jsdom
import Ajv2020 from "ajv/dist/2020";
import { describe, expect, it } from "vitest";
import defaultTheme from "../../../../themes/default/theme.json";
import schema from "../../../../themes/theme.schema.json";
import { contrastRatio } from "./contrast";
import { applyTheme, parseTheme, THEME_COLORS, type Theme } from "./theme";

const validateSchema = new Ajv2020({ allErrors: true }).compile(schema);

function withColors(colors: Partial<Theme["colors"]>): unknown {
  return { ...defaultTheme, colors: { ...defaultTheme.colors, ...colors } };
}

describe("contrastRatio", () => {
  it("matches the WCAG reference values", () => {
    expect(contrastRatio("#000000", "#ffffff")).toBeCloseTo(21, 5);
    expect(contrastRatio("#777777", "#ffffff")).toBeCloseTo(4.48, 2);
    expect(contrastRatio("#ffffff", "#000000")).toBe(contrastRatio("#000000", "#ffffff"));
  });
});

describe("default theme", () => {
  it("is valid against the published schema and the loader", () => {
    expect(validateSchema(defaultTheme), JSON.stringify(validateSchema.errors)).toBe(true);
    expect(parseTheme(defaultTheme)).toEqual({
      ok: true,
      theme: { name: defaultTheme.name, colors: defaultTheme.colors },
    });
  });
});

describe("parseTheme", () => {
  it("rejects a theme under 4.5:1 with a message naming the pair", () => {
    const result = parseTheme(withColors({ "content-muted": "#5a6270" }));
    expect(result.ok).toBe(false);
    if (result.ok) return;
    expect(result.errors).toContain(
      "Contraste insuffisant entre « content-muted » et « surface-raised » : 2.62:1, minimum 4.5:1.",
    );
  });

  it("floors the displayed ratio so it never rounds up to the minimum", () => {
    // #767676 on #ffffff is 4.54:1 (passes); #777777 is 4.48:1 (fails).
    const result = parseTheme(withColors({ content: "#777777", surface: "#ffffff" }));
    expect(result.ok).toBe(false);
    if (result.ok) return;
    expect(result.errors.join("\n")).toMatch(/« content » et « surface » : 4\.47:1/);
  });

  it.each([
    ["not an object", [], "Le thème doit être un objet JSON."],
    ["missing color", withColors({ focus: undefined }), "Couleur « focus » manquante."],
    ["short hex", withColors({ accent: "#fff" }), "Couleur « accent » invalide : attendu #rrggbb."],
    [
      "alpha",
      withColors({ accent: "#6cb4ff80" }),
      "Couleur « accent » invalide : attendu #rrggbb.",
    ],
    [
      "named color",
      withColors({ accent: "red" }),
      "Couleur « accent » invalide : attendu #rrggbb.",
    ],
    ["unknown color", withColors({ extra: "#000000" } as never), "Couleur inconnue « extra »."],
    ["unknown key", { ...defaultTheme, font: "x" }, "Propriété inconnue « font »."],
    ["empty name", { ...defaultTheme, name: "" }, "« name » doit être un texte non vide."],
    ["no colors", { name: "x" }, "« colors » doit être un objet."],
  ])("rejects a malformed theme (%s), like the schema", (_, raw, message) => {
    const result = parseTheme(raw);
    expect(result.ok).toBe(false);
    if (!result.ok) expect(result.errors).toContain(message);
    expect(validateSchema(raw)).toBe(false);
  });

  it("knows the same color roles as the schema", () => {
    expect([...THEME_COLORS].sort()).toEqual([...schema.properties.colors.required].sort());
  });
});

describe("applyTheme", () => {
  it("sets one --om-* variable per color role", () => {
    const root = document.createElement("div");
    applyTheme(defaultTheme, root);
    for (const role of THEME_COLORS) {
      expect(root.style.getPropertyValue(`--om-${role}`)).toBe(defaultTheme.colors[role]);
    }
  });
});
