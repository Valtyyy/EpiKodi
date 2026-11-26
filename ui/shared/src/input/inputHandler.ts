import type { InputAction } from "../generated/InputAction";
import type { Direction } from "../focus/spatial";

/**
 * The living-room key of each action. An action from another device (phone remote) is
 * replayed as this key on the focused element, so widgets with their own key handling
 * (stepper, virtual keyboard) behave the same whatever the source.
 */
export const ACTION_KEY: Record<InputAction, string> = {
  up: "ArrowUp",
  down: "ArrowDown",
  left: "ArrowLeft",
  right: "ArrowRight",
  ok: "Enter",
  back: "Escape",
};

const KEY_ACTION: Partial<Record<string, InputAction>> = Object.fromEntries(
  Object.entries(ACTION_KEY).map(([action, key]) => [key, action as InputAction]),
);

/** What the default handling drives: the spatial navigation. */
export interface FocusMover {
  move: (dir: Direction) => boolean;
}

export interface InputHandler {
  /** Applies an action from another device as if its key had been pressed. */
  dispatch: (action: InputAction) => void;
  destroy: () => void;
}

/** Input types without a caret: their arrows and Enter are free for navigation. */
const NO_CARET = new Set([
  "button",
  "checkbox",
  "radio",
  "submit",
  "reset",
  "image",
  "file",
  "color",
]);

function isTextField(t: EventTarget | null): boolean {
  if (t instanceof HTMLTextAreaElement) return true;
  if (t instanceof HTMLInputElement) return !NO_CARET.has(t.type);
  return t instanceof HTMLElement && t.isContentEditable;
}

/**
 * The single `InputAction` handler of the TV (ARCHITECTURE.md section 6). It runs after
 * the focused widget: a widget that used the key calls `preventDefault` and keeps it.
 * Otherwise directions move the focus and `ok` activates the focused element. `back`
 * has no default yet: screens and dialogs handle Escape themselves.
 */
export function createInputHandler(doc: Document, focus: FocusMover): InputHandler {
  function perform(action: InputAction, target: EventTarget | null): boolean {
    // A text field keeps its caret and Enter.
    if (action !== "up" && action !== "down" && action !== "back" && isTextField(target)) {
      return false;
    }
    switch (action) {
      case "up":
      case "down":
      case "left":
      case "right":
        return focus.move(action);
      case "ok": {
        const el = doc.activeElement;
        if (!(el instanceof HTMLElement) || el === doc.body) return false;
        // Also replaces the native Enter activation (prevented below), so a button is
        // clicked once, by the same code for the keyboard and the remote.
        el.click();
        return true;
      }
      case "back":
        return false;
    }
  }

  // On the document: React handles events at its root container, so a listener below it
  // would run before the widgets had a chance to call preventDefault.
  const onKeyDown = (e: KeyboardEvent) => {
    const action = KEY_ACTION[e.key];
    if (!action || e.defaultPrevented || e.altKey || e.ctrlKey || e.metaKey || e.shiftKey) return;
    if (perform(action, e.target)) e.preventDefault();
  };
  doc.addEventListener("keydown", onKeyDown);

  return {
    dispatch(action) {
      const target = doc.activeElement ?? doc.body;
      target.dispatchEvent(
        new KeyboardEvent("keydown", { key: ACTION_KEY[action], bubbles: true, cancelable: true }),
      );
    },
    destroy() {
      doc.removeEventListener("keydown", onKeyDown);
    },
  };
}
