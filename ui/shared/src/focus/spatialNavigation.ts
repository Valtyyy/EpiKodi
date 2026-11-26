import { findNext, type Direction } from "./spatial";

/** Marks an element as a spatial navigation target. Its `tabindex` belongs to the manager. */
export const NAV_ITEM_ATTR = "data-nav";
/** Groups items: each zone keeps one tabbable item and remembers the last focused one. */
export const NAV_ZONE_ATTR = "data-nav-zone";

export interface SpatialNavigation {
  /** Moves real DOM focus one step; false when there is nothing in that direction. */
  move: (dir: Direction) => boolean;
  destroy: () => void;
}

function isUsable(el: HTMLElement): boolean {
  return !(el as HTMLButtonElement).disabled && el.closest("[hidden],[inert]") === null;
}

/**
 * Spatial navigation over `[data-nav]` items under `root` (ARCHITECTURE.md section 6).
 * It moves the real DOM focus, because a screen reader follows DOM focus, not React
 * state. Roving tabindex is per zone: the zone's remembered item is the only tabbable
 * one, so Tab jumps between zones and coming back to a zone lands where the user left.
 *
 * It does not listen to keys: the input handler (`createInputHandler`) calls `move`, so
 * the living-room keyboard and the phone remote share one path.
 */
export function createSpatialNavigation(root: HTMLElement): SpatialNavigation {
  const doc = root.ownerDocument;
  const memory = new WeakMap<Element, HTMLElement>();
  let last: HTMLElement | null = null;
  let lastZone: Element = root;

  const zoneOf = (el: Element): Element => {
    const zone = el.closest(`[${NAV_ZONE_ATTR}]`);
    return zone && root.contains(zone) ? zone : root;
  };

  const items = (scope: Element = root): HTMLElement[] =>
    [...scope.querySelectorAll<HTMLElement>(`[${NAV_ITEM_ATTR}]`)].filter(isUsable);

  const activeOf = (zone: Element, all: HTMLElement[]): HTMLElement | undefined => {
    const remembered = memory.get(zone);
    if (remembered && all.includes(remembered)) return remembered;
    return all.find((el) => zoneOf(el) === zone);
  };

  function sync() {
    const all = items();
    const zones = new Map<Element, HTMLElement[]>();
    for (const el of all) {
      const zone = zoneOf(el);
      zones.set(zone, [...(zones.get(zone) ?? []), el]);
    }
    for (const [zone, members] of zones) {
      const active = activeOf(zone, members);
      for (const el of members) {
        const tabIndex = el === active ? 0 : -1;
        if (el.tabIndex !== tabIndex) el.tabIndex = tabIndex;
      }
    }
  }

  /** Where focus goes when there is no focused item: back where it was, else the first item. */
  function entryPoint(): HTMLElement | undefined {
    const all = items();
    return (lastZone.isConnected ? activeOf(lastZone, all) : undefined) ?? all[0];
  }

  function move(dir: Direction): boolean {
    const current = doc.activeElement;
    if (!(current instanceof HTMLElement) || current === doc.body || current === root) {
      const entry = entryPoint();
      entry?.focus();
      return entry !== undefined;
    }
    // Focus is in a layer outside the root (a portal dialog): it owns its navigation.
    if (!root.contains(current)) return false;
    // Never leave an open modal by arrows.
    const modal = current.closest('[aria-modal="true"]');
    const scope = modal && root.contains(modal) ? modal : root;
    const candidates = items(scope)
      .filter((el) => el !== current)
      .map((el) => ({ el, rect: el.getBoundingClientRect() }))
      // display: none and detached layouts have an empty box.
      .filter(({ rect }) => rect.width > 0 || rect.height > 0);
    const i = findNext(
      current.getBoundingClientRect(),
      candidates.map((c) => c.rect),
      dir,
    );
    if (i < 0) return false;
    let target = candidates[i].el;
    const zone = zoneOf(target);
    if (zone !== root && zone !== zoneOf(current)) {
      const remembered = memory.get(zone);
      if (remembered && candidates.some((c) => c.el === remembered)) target = remembered;
    }
    target.focus();
    return true;
  }

  const onFocusIn = (e: FocusEvent) => {
    const t = e.target;
    if (!(t instanceof HTMLElement) || !t.hasAttribute(NAV_ITEM_ATTR)) return;
    last = t;
    lastZone = zoneOf(t);
    memory.set(lastZone, t);
    sync();
  };

  // The focused item can be unmounted (list refresh, screen change): focus would fall on
  // <body> with no indicator, so it is put back on the zone's remembered item.
  const observer = new MutationObserver(() => {
    sync();
    const focused = doc.activeElement;
    if (last && !last.isConnected && (focused === null || focused === doc.body)) {
      last = null;
      entryPoint()?.focus();
    }
  });

  sync();
  root.addEventListener("focusin", onFocusIn);
  observer.observe(root, {
    childList: true,
    subtree: true,
    attributes: true,
    attributeFilter: ["hidden", "inert", "disabled", NAV_ITEM_ATTR],
  });

  return {
    move,
    destroy() {
      root.removeEventListener("focusin", onFocusIn);
      observer.disconnect();
    },
  };
}
