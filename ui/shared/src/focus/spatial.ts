/** Same names as the future `InputAction` directions (ARCHITECTURE.md section 6). */
export type Direction = "up" | "down" | "left" | "right";

/** The subset of `DOMRect` the algorithm needs, so tests can describe layouts as plain data. */
export interface Rect {
  left: number;
  top: number;
  right: number;
  bottom: number;
}

interface Axis {
  /** Signed gap from the origin's far edge to the candidate's near edge. */
  gap: (o: Rect, c: Rect) => number;
  /** Candidate center past the origin's far edge: it really lies in that direction. */
  ahead: (o: Rect, c: Rect) => boolean;
  /** Gap between the two rects on the orthogonal axis, 0 when they overlap. */
  offset: (o: Rect, c: Rect) => number;
  /** Distance between centers on the orthogonal axis, to break ties. */
  drift: (o: Rect, c: Rect) => number;
}

const cx = (r: Rect) => (r.left + r.right) / 2;
const cy = (r: Rect) => (r.top + r.bottom) / 2;
const spanGap = (a0: number, a1: number, b0: number, b1: number) => Math.max(0, b0 - a1, a0 - b1);

const horizontal = {
  offset: (o: Rect, c: Rect) => spanGap(o.top, o.bottom, c.top, c.bottom),
  drift: (o: Rect, c: Rect) => Math.abs(cy(o) - cy(c)),
};
const vertical = {
  offset: (o: Rect, c: Rect) => spanGap(o.left, o.right, c.left, c.right),
  drift: (o: Rect, c: Rect) => Math.abs(cx(o) - cx(c)),
};

const AXES: Record<Direction, Axis> = {
  right: { ...horizontal, gap: (o, c) => c.left - o.right, ahead: (o, c) => cx(c) > o.right },
  left: { ...horizontal, gap: (o, c) => o.left - c.right, ahead: (o, c) => cx(c) < o.left },
  down: { ...vertical, gap: (o, c) => c.top - o.bottom, ahead: (o, c) => cy(c) > o.bottom },
  up: { ...vertical, gap: (o, c) => o.top - c.bottom, ahead: (o, c) => cy(c) < o.top },
};

/** Among misaligned candidates, misalignment weighs more than distance. */
const OFFSET_WEIGHT = 2;

/** Lexicographic: aligned first, then score, then drift. */
function better(a: readonly number[], b: readonly number[]): boolean {
  for (let i = 0; i < a.length; i++) if (a[i] !== b[i]) return a[i] < b[i];
  return false;
}

/**
 * Index of the candidate to reach from `origin` in direction `dir`, or -1 when there is
 * none (edge of the screen: no wrap). A candidate aligned with the origin (same row for
 * left/right, same column for up/down) always wins over a misaligned one, even a closer
 * one, so a hole in a row is skipped instead of jumping to the next row. Pure geometry,
 * so it is tested on plain grids.
 */
export function findNext(origin: Rect, candidates: readonly Rect[], dir: Direction): number {
  const axis = AXES[dir];
  let best = -1;
  let bestKey: number[] = [];
  candidates.forEach((c, i) => {
    if (!axis.ahead(origin, c)) return;
    const offset = axis.offset(origin, c);
    const key = [
      offset > 0 ? 1 : 0,
      Math.max(0, axis.gap(origin, c)) + OFFSET_WEIGHT * offset,
      axis.drift(origin, c),
    ];
    if (best < 0 || better(key, bestKey)) {
      best = i;
      bestKey = key;
    }
  });
  return best;
}
