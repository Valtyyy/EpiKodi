import axe from "axe-core";

/** Runs axe on a rendered subtree and returns violations (empty array = accessible). */
export async function axeViolations(root: Element) {
  // jsdom has no layout/canvas: color-contrast is checked on the theme tokens instead.
  const results = await axe.run(root, { rules: { "color-contrast": { enabled: false } } });
  return results.violations.map(
    (v) => `${v.id}: ${v.help} (${v.nodes.map((n) => n.target).join(", ")})`,
  );
}
