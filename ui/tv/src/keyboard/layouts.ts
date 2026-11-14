export type LayoutId = "letters" | "digits" | "symbols";

export type KeyDef =
  | { kind: "char"; char: string }
  | { kind: "space" }
  | { kind: "backspace" }
  | { kind: "shift" }
  | { kind: "layout"; target: LayoutId }
  | { kind: "submit" }
  | { kind: "cancel" };

const chars = (s: string): KeyDef[] => [...s].map((char) => ({ kind: "char", char }));

const letterRows = ["qwertyuiop", "asdfghjkl"];

// The bottom row is shared: layout switches, space, then the two exits.
function bottomRow(current: LayoutId): KeyDef[] {
  const others = (["letters", "digits", "symbols"] as const).filter((l) => l !== current);
  return [
    ...others.map((target): KeyDef => ({ kind: "layout", target })),
    { kind: "space" },
    { kind: "cancel" },
    { kind: "submit" },
  ];
}

export const LAYOUTS: Record<LayoutId, KeyDef[][]> = {
  letters: [
    chars(letterRows[0]),
    chars(letterRows[1]),
    [{ kind: "shift" }, ...chars("zxcvbnm"), { kind: "backspace" }],
    bottomRow("letters"),
  ],
  digits: [chars("1234567890"), chars(".,-_@"), [{ kind: "backspace" }], bottomRow("digits")],
  symbols: [
    chars(".-_@:/?!#"),
    chars("%&+=()[]~"),
    [...chars(",;'\"*\\|"), { kind: "backspace" }],
    bottomRow("symbols"),
  ],
};

export const LAYOUT_LABEL: Record<LayoutId, { text: string; name: string }> = {
  letters: { text: "ABC", name: "Disposition lettres" },
  digits: { text: "123", name: "Disposition chiffres" },
  symbols: { text: "#+=", name: "Disposition symboles" },
};

const CHAR_NAMES: Record<string, string> = {
  ".": "point",
  ",": "virgule",
  "-": "tiret",
  _: "tiret bas",
  "@": "arobase",
  ":": "deux-points",
  ";": "point-virgule",
  "/": "barre oblique",
  "\\": "barre oblique inverse",
  "|": "barre verticale",
  "?": "point d'interrogation",
  "!": "point d'exclamation",
  "#": "dièse",
  "%": "pour cent",
  "&": "esperluette",
  "+": "plus",
  "=": "égal",
  "(": "parenthèse ouvrante",
  ")": "parenthèse fermante",
  "[": "crochet ouvrant",
  "]": "crochet fermant",
  "~": "tilde",
  "'": "apostrophe",
  '"': "guillemet",
  "*": "astérisque",
  " ": "espace",
};

/** Spoken name of a character (punctuation is unreadable for a screen reader). */
export function charName(char: string): string {
  return CHAR_NAMES[char] ?? char;
}
