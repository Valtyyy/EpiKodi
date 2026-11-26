// @vitest-environment jsdom
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { Direction } from "../focus/spatial";
import { createInputHandler, type InputHandler } from "./inputHandler";

const key = (k: string, target: Element = document.activeElement ?? document.body) =>
  !target.dispatchEvent(new KeyboardEvent("keydown", { key: k, bubbles: true, cancelable: true }));

let moves: Direction[];
let handler: InputHandler;

beforeEach(() => {
  moves = [];
  handler = createInputHandler(document, {
    move: (dir) => {
      moves.push(dir);
      return true;
    },
  });
  document.body.innerHTML = `<button id="b">b</button><input id="t"><button id="w">w</button>`;
});

afterEach(() => {
  handler.destroy();
  document.body.innerHTML = "";
});

const byId = (id: string) => document.getElementById(id)!;

describe("input handler", () => {
  it("turns arrows into focus moves and consumes them", () => {
    byId("b").focus();
    expect(key("ArrowUp")).toBe(true);
    key("ArrowDown");
    key("ArrowLeft");
    key("ArrowRight");
    expect(moves).toEqual(["up", "down", "left", "right"]);
  });

  it("activates the focused element once on ok", () => {
    const click = vi.fn();
    byId("b").addEventListener("click", click);
    byId("b").focus();
    expect(key("Enter")).toBe(true);
    expect(click).toHaveBeenCalledTimes(1);
  });

  it("leaves keys a widget already used", () => {
    byId("w").addEventListener("keydown", (e) => e.preventDefault());
    byId("w").focus();
    key("ArrowRight");
    expect(moves).toEqual([]);
  });

  it("ignores keys with modifiers", () => {
    byId("b").focus();
    byId("b").dispatchEvent(
      new KeyboardEvent("keydown", { key: "ArrowDown", altKey: true, bubbles: true }),
    );
    expect(moves).toEqual([]);
  });

  it("leaves left/right and Enter to a text field, not up/down", () => {
    byId("t").focus();
    expect(key("ArrowLeft")).toBe(false);
    expect(key("Enter")).toBe(false);
    key("ArrowDown");
    expect(moves).toEqual(["down"]);
  });
});

describe("dispatch (actions from the UiBridge)", () => {
  it("produces the same moves and activation as the keyboard", () => {
    const click = vi.fn();
    byId("b").addEventListener("click", click);
    byId("b").focus();
    for (const a of ["up", "down", "left", "right", "ok"] as const) handler.dispatch(a);
    expect(moves).toEqual(["up", "down", "left", "right"]);
    expect(click).toHaveBeenCalledTimes(1);
  });

  it("goes through the focused widget first, as its key", () => {
    const seen: string[] = [];
    byId("w").addEventListener("keydown", (e) => {
      seen.push(e.key);
      e.preventDefault();
    });
    byId("w").focus();
    handler.dispatch("left");
    handler.dispatch("back");
    expect(seen).toEqual(["ArrowLeft", "Escape"]);
    expect(moves).toEqual([]);
  });

  it("enters the navigation when nothing is focused", () => {
    handler.dispatch("down");
    expect(moves).toEqual(["down"]);
  });
});
