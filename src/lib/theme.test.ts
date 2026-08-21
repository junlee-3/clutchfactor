import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";
import { FALLBACK, getToken, rgba, type TokenName } from "./theme";

// tokens.css is the contract (docs/design/design-system.md §2); FALLBACK is
// a literal copy for the no-DOM path (node tests, SSR-less safety). This
// suite is what keeps the copy honest — it fails the moment the two drift.
const css = readFileSync(new URL("../styles/tokens.css", import.meta.url), "utf8");

describe("FALLBACK stays in sync with tokens.css", () => {
  for (const name of Object.keys(FALLBACK) as TokenName[]) {
    it(`${name} matches its tokens.css value`, () => {
      const match = new RegExp(`${name}:\\s*(#[0-9a-f]{6})`, "i").exec(css);
      expect(match, `${name} not found in tokens.css`).not.toBeNull();
      expect(FALLBACK[name].toLowerCase()).toBe(match![1].toLowerCase());
    });
  }
});

describe("rgba", () => {
  it("converts a token's hex value + alpha into a canvas rgba() string", () => {
    expect(rgba("--ct", 0.5)).toBe("rgba(74, 163, 255, 0.5)");
  });

  it("handles alpha 0 and 1", () => {
    expect(rgba("--loss", 0)).toBe("rgba(209, 106, 95, 0)");
    expect(rgba("--win", 1)).toBe("rgba(93, 187, 122, 1)");
  });
});

describe("getToken", () => {
  it("has no DOM in this suite (node environment)", () => {
    expect(typeof document).toBe("undefined");
  });

  it("returns the FALLBACK value when there is no document", () => {
    for (const name of Object.keys(FALLBACK) as TokenName[]) {
      expect(getToken(name)).toBe(FALLBACK[name]);
    }
  });
});
