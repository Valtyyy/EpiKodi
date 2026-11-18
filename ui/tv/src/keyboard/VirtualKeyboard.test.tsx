import { useState } from "react";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { axeViolations } from "../test/axe";
import { VirtualKeyboard } from "./VirtualKeyboard";

const TMDB_KEY = "0123456789abcdef0123456789abcdef";

function Harness(props: {
  secret?: boolean;
  maxLength?: number;
  initial?: string;
  onSubmit?: () => void;
  onCancel?: () => void;
  onValue?: (v: string) => void;
}) {
  const [value, setValue] = useState(props.initial ?? "");
  return (
    <VirtualKeyboard
      label="Clé API TMDB"
      value={value}
      onChange={(v) => {
        setValue(v);
        props.onValue?.(v);
      }}
      onSubmit={props.onSubmit ?? (() => {})}
      onCancel={props.onCancel ?? (() => {})}
      maxLength={props.maxLength}
      secret={props.secret}
    />
  );
}

const textbox = () => screen.getByRole("textbox", { name: "Clé API TMDB" });
const status = () => screen.getByRole("status");

/** Accessible names of every key, as a grid (row -> column), read from the DOM. */
function grid(): string[][] {
  const rows = new Map<Element, string[]>();
  for (const b of screen.getAllByRole("button")) {
    const row = b.parentElement!;
    rows.set(row, [...(rows.get(row) ?? []), b.getAttribute("aria-label")!]);
  }
  return [...rows.values()];
}

function find(name: string): [number, number] {
  const g = grid();
  for (let r = 0; r < g.length; r++) {
    const c = g[r].indexOf(name);
    if (c >= 0) return [r, c];
  }
  throw new Error(`no key named ${name}`);
}

function currentPosition(): [number, number] {
  return find(document.activeElement!.getAttribute("aria-label")!);
}

/** Navigates by arrows only (as a TV remote would) to the key, then presses Enter. */
async function pressKey(user: ReturnType<typeof userEvent.setup>, name: string) {
  const [tr, tc] = find(name);
  for (;;) {
    const [r, c] = currentPosition();
    if (r === tr && c === tc) break;
    // Rows have different lengths, so move vertically first, then horizontally (clamping
    // may change the column, hence the loop re-reads the position every step).
    if (r < tr) await user.keyboard("{ArrowDown}");
    else if (r > tr) await user.keyboard("{ArrowUp}");
    else if (c < tc) await user.keyboard("{ArrowRight}");
    else await user.keyboard("{ArrowLeft}");
  }
  await user.keyboard("{Enter}");
}

async function typeWithArrows(user: ReturnType<typeof userEvent.setup>, text: string) {
  for (const ch of text) {
    if (!grid().flat().includes(ch)) {
      await pressKey(user, /[0-9]/.test(ch) ? "Disposition chiffres" : "Disposition lettres");
    }
    await pressKey(user, ch);
  }
}

describe("VirtualKeyboard", () => {
  it("focuses a key on mount and exposes French accessible names", () => {
    render(<Harness />);
    expect(document.activeElement).toBe(screen.getByRole("button", { name: "q" }));
    for (const name of ["Effacer", "Majuscules", "Valider", "Annuler", "Disposition chiffres"]) {
      expect(screen.getByRole("button", { name })).toBeInTheDocument();
    }
    expect(screen.getByRole("button", { name: "Espace" })).toBeInTheDocument();
  });

  it("types a 32-char hex TMDB key using only arrows and Enter", async () => {
    const user = userEvent.setup();
    const onValue = vi.fn();
    render(<Harness onValue={onValue} />);
    await typeWithArrows(user, TMDB_KEY);
    expect(onValue).toHaveBeenLastCalledWith(TMDB_KEY);
    expect(textbox()).toHaveTextContent(TMDB_KEY);
  });

  it("accepts the physical keyboard at any time, Backspace and Escape included", async () => {
    const user = userEvent.setup();
    const onCancel = vi.fn();
    render(<Harness onCancel={onCancel} />);
    await user.keyboard("ab-c");
    expect(textbox()).toHaveTextContent("ab-c");
    await user.keyboard("{Backspace}");
    expect(textbox()).toHaveTextContent("ab-");
    await user.keyboard("{Escape}");
    expect(onCancel).toHaveBeenCalledTimes(1);
  });

  it("deletes with the Effacer key and announces it", async () => {
    const user = userEvent.setup();
    render(<Harness initial="abc" />);
    await pressKey(user, "Effacer");
    expect(textbox()).toHaveTextContent("ab");
    expect(status()).toHaveTextContent("Caractère effacé");
  });

  it("switches layouts and announces the new one", async () => {
    const user = userEvent.setup();
    render(<Harness />);
    await pressKey(user, "Disposition symboles");
    expect(status()).toHaveTextContent("Disposition symboles");
    expect(screen.getByRole("button", { name: "arobase" })).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "q" })).toBeNull();
    // Focus survived the switch and still lives inside the keyboard.
    expect(document.activeElement).not.toBe(document.body);
    await pressKey(user, "Disposition chiffres");
    expect(screen.getByRole("button", { name: "7" })).toBeInTheDocument();
  });

  it("shift is one-shot and aria-pressed", async () => {
    const user = userEvent.setup();
    render(<Harness />);
    await pressKey(user, "Majuscules");
    expect(screen.getByRole("button", { name: "Majuscules" })).toHaveAttribute(
      "aria-pressed",
      "true",
    );
    await pressKey(user, "A");
    expect(screen.getByRole("button", { name: "Majuscules" })).toHaveAttribute(
      "aria-pressed",
      "false",
    );
    await pressKey(user, "a");
    expect(textbox()).toHaveTextContent("Aa");
  });

  it("Valider and Annuler call their handlers", async () => {
    const user = userEvent.setup();
    const onSubmit = vi.fn();
    const onCancel = vi.fn();
    render(<Harness onSubmit={onSubmit} onCancel={onCancel} />);
    await pressKey(user, "Valider");
    expect(onSubmit).toHaveBeenCalledTimes(1);
    await pressKey(user, "Annuler");
    expect(onCancel).toHaveBeenCalledTimes(1);
  });

  it("respects maxLength", async () => {
    const user = userEvent.setup();
    render(<Harness maxLength={2} />);
    await user.keyboard("abc");
    expect(textbox()).toHaveTextContent("ab");
    expect(status()).toHaveTextContent("Longueur maximale atteinte");
  });

  it("never renders or announces a secret in clear", async () => {
    const user = userEvent.setup();
    const { container } = render(<Harness secret />);
    await user.keyboard("zq");
    expect(textbox()).toHaveTextContent("••");
    expect(status()).toHaveTextContent("caractère saisi");
    expect(textbox().textContent).not.toMatch(/[zq]/);
    expect(status().textContent).not.toMatch(/[zq]/);
    await pressKey(user, "a");
    expect(status().textContent).not.toContain("a ");
    expect(container.textContent).not.toContain("zqa");
    expect(textbox()).toHaveTextContent("•••");
  });

  it("has no axe violations", async () => {
    const { container } = render(<Harness initial="abc" />);
    expect(await axeViolations(container)).toEqual([]);
  });
});
