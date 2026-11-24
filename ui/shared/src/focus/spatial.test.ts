import { describe, expect, it } from "vitest";
import { findNext, type Direction, type Rect } from "./spatial";

const box = (x: number, y: number, w = 100, h = 100): Rect => ({
  left: x,
  top: y,
  right: x + w,
  bottom: y + h,
});

/** rows x cols cells of 100x100 with a 20px gap; index = row * cols + col. */
function grid(rows: number, cols: number): Rect[] {
  return Array.from({ length: rows * cols }, (_, i) =>
    box((i % cols) * 120, Math.floor(i / cols) * 120),
  );
}

const step = (rects: Rect[], from: number, dir: Direction) => findNext(rects[from], rects, dir);

describe("findNext", () => {
  it("moves to the direct neighbor in a regular grid", () => {
    const g = grid(3, 3);
    expect(step(g, 4, "up")).toBe(1);
    expect(step(g, 4, "down")).toBe(7);
    expect(step(g, 4, "left")).toBe(3);
    expect(step(g, 4, "right")).toBe(5);
  });

  it("returns -1 at the edges (no wrap)", () => {
    const g = grid(3, 3);
    expect(step(g, 0, "up")).toBe(-1);
    expect(step(g, 0, "left")).toBe(-1);
    expect(step(g, 8, "down")).toBe(-1);
    expect(step(g, 8, "right")).toBe(-1);
  });

  it("prefers the aligned neighbor over a closer diagonal one", () => {
    // 1 is diagonal but only 10px away, 2 is aligned and 200px away.
    const rects = [box(0, 0), box(110, 110), box(300, 0)];
    expect(findNext(rects[0], rects, "right")).toBe(2);
  });

  it("skips a gap in a row and reaches the next item in that row", () => {
    const g = grid(2, 4).filter((_, i) => i !== 1);
    expect(step(g, 0, "right")).toBe(1); // former index 2
  });

  it("goes down from a wide item to the most centered item below", () => {
    const rects = [
      box(0, 0, 460, 100),
      ...grid(1, 4).map((r) => ({ ...r, top: 120, bottom: 220 })),
    ];
    expect(findNext(rects[0], rects, "down")).toBe(2);
  });

  it("reaches a shorter row below from the end of a long row", () => {
    const rects = [...grid(1, 5), box(0, 120), box(120, 120)];
    expect(step(rects, 4, "down")).toBe(6);
  });

  it("ignores items behind the origin", () => {
    const rects = [box(0, 0), box(-200, 0), box(0, -200)];
    expect(findNext(rects[0], rects, "right")).toBe(-1);
    expect(findNext(rects[0], rects, "down")).toBe(-1);
  });
});
