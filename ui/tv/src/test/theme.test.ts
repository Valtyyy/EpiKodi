/// <reference types="node" />
// @vitest-environment node
import { compile } from "@tailwindcss/node";
import { readFile } from "node:fs/promises";
import { describe, expect, it } from "vitest";

// Compiles the real TV stylesheet, so the guarantees below hold for what ships,
// not for a copy of the theme. Read from disk: Vitest stubs CSS imports (even ?raw).
async function build(candidates: string[]): Promise<string> {
  const entry = new URL("../index.css", import.meta.url);
  const css = await readFile(entry, "utf8");
  expect(css).toContain('@import "@om/shared/theme.css"');
  const base = new URL(".", entry).pathname;
  const compiler = await compile(css, { base, onDependency: () => {} });
  return compiler.build(candidates);
}

describe("TV theme", () => {
  it("emits no style for Tailwind's default palette", async () => {
    const out = await build(["bg-blue-500", "text-red-600", "border-gray-200"]);
    expect(out).not.toMatch(/\.bg-blue-500|\.text-red-600|\.border-gray-200/);
  });

  it("emits semantic colors backed by the theme variables", async () => {
    const out = await build(["bg-surface", "text-accent"]);
    expect(out).toMatch(/\.bg-surface\s*\{\s*background-color: var\(--om-surface\)/);
    expect(out).toMatch(/\.text-accent\s*\{\s*color: var\(--om-accent\)/);
  });

  it("only offers the 10-foot text scale", async () => {
    const out = await build(["text-xs", "text-sm", "text-tv-sm"]);
    expect(out).not.toMatch(/\.text-xs|\.text-sm\b/);
    expect(out).toMatch(/\.text-tv-sm\s*\{\s*font-size: var\(--text-tv-sm\)/);
  });

  it("scales spacing for a 3 meter viewing distance", async () => {
    const out = await build(["p-8"]);
    expect(out).toMatch(/--spacing: 0\.375rem/);
    expect(out).toMatch(/\.p-8\s*\{\s*padding: calc\(var\(--spacing\) \* 8\)/);
  });
});
