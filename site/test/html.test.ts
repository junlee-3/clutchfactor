import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";
import { release } from "../src/release";

const html = readFileSync(new URL("../index.html", import.meta.url), "utf8");

describe("index.html structure", () => {
  it("has exactly one h1 and the section anchors the nav links to", () => {
    expect(html.match(/<h1\b/g)?.length).toBe(1);
    for (const id of ["top", "ledger", "catches", "coach", "limits", "habits", "download"]) {
      expect(html, id).toContain(`id="${id}"`);
    }
  });

  it("links every release asset by its real file name (no-JS fallback)", () => {
    for (const file of [release.mac.file, release.win.file, release.msi.file]) expect(html).toContain(file);
  });

  it("has the hooks main.ts binds to", () => {
    for (const hook of [
      "data-nav-download",
      'data-video="a"',
      'data-video="b"',
      "data-download-buttons",
      "data-ledger",
      "data-target=",
      "data-reveal",
    ]) {
      expect(html, hook).toContain(hook);
    }
  });

  it("gives every image descriptive alt text", () => {
    const imgs = html.match(/<img\b[^>]*>/g) ?? [];
    expect(imgs.length).toBeGreaterThanOrEqual(5); // replay, coach, watches, trends, corpus
    for (const img of imgs) expect(img).toMatch(/\balt="[^"]{30,}"/);
  });

  it("contains no placeholder copy", () => {
    expect(html).not.toMatch(/lorem|TODO|TBD|placeholder/i);
  });
});
