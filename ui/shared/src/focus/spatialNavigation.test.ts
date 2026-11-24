// @vitest-environment jsdom
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { createSpatialNavigation, type SpatialNavigation } from "./spatialNavigation";

/** jsdom has no layout: each item declares its box as `data-box="x y w h"`. */
function mount(html: string): HTMLElement {
  const root = document.createElement("div");
  root.innerHTML = html;
  document.body.append(root);
  for (const el of root.querySelectorAll<HTMLElement>("[data-box]")) {
    const [x, y, w, h] = el.dataset.box!.split(" ").map(Number);
    el.getBoundingClientRect = () => new DOMRect(x, y, w, h);
  }
  return root;
}

const byId = (id: string) => document.getElementById(id)!;
const press = (key: string, target: Element = document.activeElement ?? document.body) =>
  target.dispatchEvent(new KeyboardEvent("keydown", { key, bubbles: true, cancelable: true }));
const flush = () => new Promise((r) => setTimeout(r, 0));

let nav: SpatialNavigation;
let root: HTMLElement;

afterEach(() => {
  nav.destroy();
  root.remove();
});

describe("spatial navigation on a grid", () => {
  beforeEach(() => {
    // 2x3 grid of buttons, no zones.
    root = mount(
      ["a", "b", "c", "d", "e", "f"]
        .map(
          (id, i) =>
            `<button id="${id}" data-nav data-box="${(i % 3) * 120} ${Math.floor(i / 3) * 120} 100 100">${id}</button>`,
        )
        .join(""),
    );
    nav = createSpatialNavigation(root);
  });

  it("moves document.activeElement with the arrow keys", () => {
    byId("a").focus();
    press("ArrowRight");
    expect(document.activeElement).toBe(byId("b"));
    press("ArrowDown");
    expect(document.activeElement).toBe(byId("e"));
    press("ArrowLeft");
    press("ArrowUp");
    expect(document.activeElement).toBe(byId("a"));
  });

  it("keeps a single tabbable item that follows focus (roving tabindex)", () => {
    const tabbable = () => [...root.querySelectorAll("button")].filter((b) => b.tabIndex === 0);
    expect(tabbable().map((b) => b.id)).toEqual(["a"]);
    byId("a").focus();
    press("ArrowRight");
    expect(tabbable().map((b) => b.id)).toEqual(["b"]);
  });

  it("stays put at the edge and leaves the event alone", () => {
    byId("c").focus();
    const handled = !press("ArrowRight");
    expect(handled).toBe(false);
    expect(document.activeElement).toBe(byId("c"));
  });

  it("ignores arrows a widget already handled", () => {
    byId("a").focus();
    byId("a").addEventListener("keydown", (e) => e.preventDefault());
    press("ArrowRight");
    expect(document.activeElement).toBe(byId("a"));
  });

  it("skips disabled and hidden items", () => {
    (byId("b") as HTMLButtonElement).disabled = true;
    byId("c").hidden = true;
    byId("a").focus();
    // Nothing usable left in the row: the closest item ahead is in the next row.
    expect(nav.move("right")).toBe(true);
    expect(document.activeElement).toBe(byId("e"));
  });

  it("brings focus back from <body> on the next arrow", () => {
    byId("e").focus();
    byId("e").blur();
    expect(document.activeElement).toBe(document.body);
    press("ArrowDown", document.body);
    expect(document.activeElement).toBe(byId("e"));
  });

  it("refocuses an item when the focused one is removed", async () => {
    byId("e").focus();
    byId("e").remove();
    await flush();
    expect(document.activeElement).toBe(byId("a"));
  });
});

describe("zones", () => {
  beforeEach(() => {
    // A menu column on the left, a 2x2 content grid on the right.
    root = mount(`
      <nav data-nav-zone>
        <button id="m1" data-nav data-box="0 0 100 100">m1</button>
        <button id="m2" data-nav data-box="0 120 100 100">m2</button>
      </nav>
      <section data-nav-zone>
        <button id="c1" data-nav data-box="200 0 100 100">c1</button>
        <button id="c2" data-nav data-box="320 0 100 100">c2</button>
        <button id="c3" data-nav data-box="200 120 100 100">c3</button>
        <button id="c4" data-nav data-box="320 120 100 100">c4</button>
      </section>`);
    nav = createSpatialNavigation(root);
  });

  it("has one tabbable item per zone", () => {
    const tabbable = [...root.querySelectorAll("button")].filter((b) => b.tabIndex === 0);
    expect(tabbable.map((b) => b.id)).toEqual(["m1", "c1"]);
  });

  it("returns to the last focused item of a zone", () => {
    byId("c4").focus();
    press("ArrowLeft");
    press("ArrowLeft");
    expect(document.activeElement).toBe(byId("m2"));
    press("ArrowUp");
    // Geometry would pick c1 from m1; the zone remembers c3.
    press("ArrowRight");
    expect(document.activeElement).toBe(byId("c3"));
  });

  it("keeps the remembered item as the zone's tab stop", () => {
    byId("c4").focus();
    byId("m1").focus();
    const tabbable = [...root.querySelectorAll("button")].filter((b) => b.tabIndex === 0);
    expect(tabbable.map((b) => b.id)).toEqual(["m1", "c4"]);
  });
});

describe("guards", () => {
  it("does not leave an open modal", () => {
    root = mount(`
      <button id="behind" data-nav data-box="0 300 100 100">behind</button>
      <div role="dialog" aria-modal="true">
        <button id="ok" data-nav data-box="0 0 100 100">ok</button>
      </div>`);
    nav = createSpatialNavigation(root);
    byId("ok").focus();
    expect(nav.move("down")).toBe(false);
    expect(document.activeElement).toBe(byId("ok"));
  });

  it("leaves left/right to a text field's caret", () => {
    root = mount(`
      <input id="field" data-nav data-box="0 0 100 100">
      <button id="next" data-nav data-box="120 0 100 100">next</button>
      <button id="below" data-nav data-box="0 120 100 100">below</button>`);
    nav = createSpatialNavigation(root);
    byId("field").focus();
    press("ArrowRight");
    expect(document.activeElement).toBe(byId("field"));
    press("ArrowDown");
    expect(document.activeElement).toBe(byId("below"));
  });
});
