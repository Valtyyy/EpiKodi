import { useEffect, useId, useRef, useState, type KeyboardEvent } from "react";
import { useRovingGrid } from "@om/shared";
import { LAYOUTS, LAYOUT_LABEL, charName, type KeyDef, type LayoutId } from "./layouts";

export interface VirtualKeyboardProps {
  label: string;
  value: string;
  onChange: (value: string) => void;
  onSubmit: () => void;
  onCancel: () => void;
  maxLength?: number;
  /** Masks the display and never announces typed characters (API keys, passwords). */
  secret?: boolean;
}

const keyClass =
  "min-h-16 min-w-16 flex-1 rounded-lg bg-surface-raised px-4 text-tv-md text-content " +
  "focus:outline-4 focus:outline-offset-2 focus:outline-focus focus:bg-accent focus:text-accent-content " +
  "aria-pressed:bg-content-muted aria-pressed:text-surface";

export function VirtualKeyboard({
  label,
  value,
  onChange,
  onSubmit,
  onCancel,
  maxLength,
  secret = false,
}: VirtualKeyboardProps) {
  const [layout, setLayout] = useState<LayoutId>("letters");
  const [shift, setShift] = useState(false);
  const [announcement, setAnnouncement] = useState({ text: "", n: 0 });
  const labelId = useId();

  const rows = LAYOUTS[layout];
  const grid = useRovingGrid({ rowLengths: rows.map((r) => r.length) });
  const { focusActive } = grid;

  // Mount: the keyboard takes focus from whatever opened it. After a layout switch, which
  // can remove the focused key and leave focus on <body>, only recover if focus was lost.
  const mounted = useRef(false);
  useEffect(() => {
    const lost = !document.activeElement || document.activeElement === document.body;
    if (!mounted.current || lost) focusActive();
    mounted.current = true;
    // focusActive changes on every focus move; only the layout should retrigger this.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [layout]);

  // aria-live may not reach Orca under WebKitGTK (spike 2.5); a later Announcer port adds a
  // fallback. Keying the node by `n` remounts it so identical consecutive messages are re-read.
  const announce = (text: string) => setAnnouncement((a) => ({ text, n: a.n + 1 }));

  function insert(char: string) {
    if (maxLength !== undefined && value.length >= maxLength) {
      announce("Longueur maximale atteinte");
      return;
    }
    onChange(value + char);
    announce(secret ? "caractère saisi" : charName(char));
    if (shift) setShift(false);
  }

  function backspace() {
    if (value.length === 0) return;
    onChange(value.slice(0, -1));
    announce("Caractère effacé");
  }

  function activate(key: KeyDef) {
    switch (key.kind) {
      case "char":
        insert(shift ? key.char.toUpperCase() : key.char);
        break;
      case "space":
        insert(" ");
        break;
      case "backspace":
        backspace();
        break;
      case "shift":
        setShift(!shift);
        announce(shift ? "Majuscules désactivées" : "Majuscules activées");
        break;
      case "layout":
        setLayout(key.target);
        announce(LAYOUT_LABEL[key.target].name);
        break;
      case "submit":
        onSubmit();
        break;
      case "cancel":
        onCancel();
        break;
    }
  }

  // The physical keyboard stays usable at any time. Enter and Space are left to the focused
  // button (native click), everything else printable goes straight into the value.
  function onPhysicalKey(e: KeyboardEvent<HTMLElement>) {
    if (e.defaultPrevented || e.ctrlKey || e.metaKey || e.altKey) return;
    if (e.key === "Escape") {
      e.preventDefault();
      onCancel();
    } else if (e.key === "Backspace") {
      e.preventDefault();
      backspace();
    } else if (e.key.length === 1 && e.key !== " ") {
      e.preventDefault();
      insert(e.key);
    }
  }

  function keyLabel(key: KeyDef): { text: string; name: string } {
    switch (key.kind) {
      case "char": {
        const text = shift ? key.char.toUpperCase() : key.char;
        return { text, name: charName(text) };
      }
      case "space":
        return { text: "␣", name: "Espace" };
      case "backspace":
        return { text: "⌫", name: "Effacer" };
      case "shift":
        return { text: "⇧", name: "Majuscules" };
      case "layout":
        return LAYOUT_LABEL[key.target];
      case "submit":
        return { text: "Valider", name: "Valider" };
      case "cancel":
        return { text: "Annuler", name: "Annuler" };
    }
  }

  const shown = secret ? "•".repeat(value.length) : value;

  return (
    <section
      aria-labelledby={labelId}
      className="flex flex-col gap-6 bg-surface p-8 text-content"
      onKeyDown={onPhysicalKey}
    >
      <p id={labelId} className="text-tv-lg">
        {label}
      </p>
      {/* Read-only on purpose: input comes from the keys, never from a native text field. */}
      <div
        role="textbox"
        aria-readonly="true"
        aria-labelledby={labelId}
        className="min-h-20 rounded-lg bg-surface-raised px-4 py-3 font-mono text-tv-lg break-all"
      >
        {shown}
        <span aria-hidden="true" className="text-accent">
          |
        </span>
      </div>
      <div className="flex flex-col gap-3">
        {rows.map((row, r) => (
          <div key={r} className="flex gap-3">
            {row.map((key, c) => {
              const { text, name } = keyLabel(key);
              return (
                <button
                  key={c}
                  type="button"
                  aria-label={name}
                  aria-pressed={key.kind === "shift" ? shift : undefined}
                  className={`${keyClass} ${key.kind === "space" ? "flex-[4]" : ""}`}
                  onClick={() => activate(key)}
                  {...grid.getItemProps(r, c)}
                >
                  {text}
                </button>
              );
            })}
          </div>
        ))}
      </div>
      <p aria-live="polite" role="status" className="sr-only">
        <span key={announcement.n}>{announcement.text}</span>
      </p>
    </section>
  );
}
