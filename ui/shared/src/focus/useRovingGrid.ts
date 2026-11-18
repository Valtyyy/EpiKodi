import { useCallback, useRef, useState, type KeyboardEvent } from "react";

export interface GridPosition {
  row: number;
  col: number;
}

export interface UseRovingGridOptions {
  /** Number of items in each row; rows may have different lengths. */
  rowLengths: readonly number[];
  initial?: GridPosition;
}

export interface GridItemProps {
  ref: (el: HTMLElement | null) => void;
  tabIndex: 0 | -1;
  onFocus: () => void;
  onKeyDown: (e: KeyboardEvent<HTMLElement>) => void;
}

export interface RovingGrid {
  active: GridPosition;
  getItemProps: (row: number, col: number) => GridItemProps;
  /** Moves real DOM focus to the active item (e.g. after its element was remounted). */
  focusActive: () => void;
}

function clamp(pos: GridPosition, rowLengths: readonly number[]): GridPosition {
  const row = Math.max(0, Math.min(pos.row, rowLengths.length - 1));
  const col = Math.max(0, Math.min(pos.col, (rowLengths[row] ?? 1) - 1));
  return { row, col };
}

/**
 * Roving tabindex over a 2D grid with real DOM focus (no aria-activedescendant,
 * which WebKitGTK/Orca handle poorly). Only the active item is tabbable. Arrows
 * move focus; changing row clamps the column to the target row's length. No wrap.
 */
export function useRovingGrid({ rowLengths, initial }: UseRovingGridOptions): RovingGrid {
  const [pos, setPos] = useState<GridPosition>(initial ?? { row: 0, col: 0 });
  const refs = useRef(new Map<string, HTMLElement>());
  // rowLengths may shrink between renders (layout change): derive instead of syncing state.
  const active = clamp(pos, rowLengths);

  const focusAt = useCallback(
    (p: GridPosition) => refs.current.get(`${p.row}:${p.col}`)?.focus(),
    [],
  );

  const focusActive = useCallback(() => focusAt(active), [focusAt, active]);

  const getItemProps = (row: number, col: number): GridItemProps => ({
    ref: (el) => {
      const key = `${row}:${col}`;
      if (el) refs.current.set(key, el);
      else refs.current.delete(key);
    },
    tabIndex: row === active.row && col === active.col ? 0 : -1,
    onFocus: () => setPos({ row, col }),
    onKeyDown: (e) => {
      if (e.defaultPrevented || e.altKey || e.ctrlKey || e.metaKey) return;
      let next: GridPosition;
      switch (e.key) {
        case "ArrowLeft":
          next = { row, col: col - 1 };
          break;
        case "ArrowRight":
          next = { row, col: col + 1 };
          break;
        case "ArrowUp":
          next = { row: row - 1, col };
          break;
        case "ArrowDown":
          next = { row: row + 1, col };
          break;
        default:
          return;
      }
      e.preventDefault();
      if (next.row < 0 || next.row >= rowLengths.length) return;
      if (next.col < 0 || next.col >= rowLengths[next.row]) {
        if (next.row === row) return;
        next = clamp(next, rowLengths);
      }
      focusAt(next);
    },
  });

  return { active, getItemProps, focusActive };
}
