import { useCallback, useEffect, useRef, type RefObject } from "react";
import type { Direction } from "./spatial";
import { createSpatialNavigation, type SpatialNavigation } from "./spatialNavigation";

/**
 * Mounts spatial navigation on `root` for the component's lifetime. Arrow keys are handled
 * directly; the returned `move` is the entry point for other input sources (phone remote).
 */
export function useSpatialNavigation(
  root: RefObject<HTMLElement | null>,
): (dir: Direction) => boolean {
  const nav = useRef<SpatialNavigation | null>(null);

  useEffect(() => {
    if (!root.current) return;
    const created = createSpatialNavigation(root.current);
    nav.current = created;
    return () => {
      created.destroy();
      nav.current = null;
    };
  }, [root]);

  return useCallback((dir: Direction) => nav.current?.move(dir) ?? false, []);
}
