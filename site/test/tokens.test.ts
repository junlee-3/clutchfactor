import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";

const read = (rel: string) => readFileSync(new URL(rel, import.meta.url), "utf8");

/** `--name: value;` pairs, comments stripped, ignoring the web-only type tokens. */
function tokens(css: string): Map<string, string> {
  const noComments = css.replace(/\/\*[\s\S]*?\*\//g, "");
  const out = new Map<string, string>();
  for (const m of noComments.matchAll(/(--[a-z0-9-]+)\s*:\s*([^;]+);/g)) {
    const name = m[1];
    if (name.startsWith("--font-") || name.startsWith("--text-")) continue;
    out.set(name, m[2].replace(/\s+/g, " ").trim());
  }
  return out;
}

describe("site tokens mirror the app's", () => {
  const app = tokens(read("../../src/styles/tokens.css"));
  const site = tokens(read("../src/styles/tokens.css"));

  it("has the same token names", () => {
    expect([...site.keys()].sort()).toEqual([...app.keys()].sort());
  });

  it("has the same values", () => {
    for (const [name, value] of app) expect(site.get(name), name).toBe(value);
  });
});
