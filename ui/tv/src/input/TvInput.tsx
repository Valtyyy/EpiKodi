import { useEffect, useRef, type ReactNode } from "react";
import { createInputHandler, createSpatialNavigation, useApi } from "@om/shared";

/**
 * Mounts the TV's single input path on its subtree: living-room keys and actions from
 * the backend (`UiBridge`, phone remote) go through the same handler into the spatial
 * navigation.
 */
export function TvInput({ children }: { children: ReactNode }) {
  const api = useApi();
  const root = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const el = root.current;
    if (!el) return;
    const nav = createSpatialNavigation(el);
    const input = createInputHandler(el.ownerDocument, nav);
    const unsubscribe = api.onUiEvent((event) => {
      if (event.type === "input") input.dispatch(event.action);
    });
    return () => {
      unsubscribe();
      input.destroy();
      nav.destroy();
    };
  }, [api]);

  return <div ref={root}>{children}</div>;
}
