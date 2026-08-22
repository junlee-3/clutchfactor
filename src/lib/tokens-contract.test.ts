// Regression guards for the design system's foundation layer: tokens.css
// (the token contract) and base.css (the reset + base typography that
// depends on those tokens). Not tokens-only — base.css is covered too.
import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";

const css = readFileSync(new URL("../styles/tokens.css", import.meta.url), "utf8");
const baseCss = readFileSync(new URL("../styles/base.css", import.meta.url), "utf8");

const REQUIRED = [
  "--bg0", "--bg1", "--bg2", "--bg-tape", "--line", "--line-strong",
  "--chalk", "--chalk-bright", "--chalk-dim", "--chalk-faint",
  "--ct", "--t", "--win", "--loss", "--tie",
  "--surface-win", "--surface-loss", "--border-win", "--border-loss",
  "--font-display", "--font-sans", "--font-mono",
  "--text-display", "--text-title", "--text-heading", "--text-body",
  "--text-ui", "--text-data", "--text-micro", "--text-stat",
  "--s1", "--s2", "--s3", "--s4", "--s5", "--s6", "--s7", "--s8",
  "--r-sm", "--r-md", "--r-lg", "--r-full",
  "--shadow-float", "--dur-fast", "--dur", "--ease",
];

describe("tokens.css contract", () => {
  it("defines every token the design system depends on", () => {
    for (const t of REQUIRED) expect(css, `missing ${t}`).toContain(t);
  });
  it("keeps game hues at their calibrated values", () => {
    expect(css).toContain("--ct:   #4aa3ff");
    expect(css).toContain("--t:    #f5b83d");
  });
  it("declares dark color-scheme", () => {
    expect(css).toContain("color-scheme: dark");
  });
});

describe("base.css contract", () => {
  it("keeps the universal reset (margin/padding zeroed, border-box sizing)", () => {
    expect(baseCss).toMatch(
      /\*,\s*\n\s*\*::before,\s*\n\s*\*::after\s*\{\s*\n\s*margin:\s*0;\s*\n\s*padding:\s*0;\s*\n\s*box-sizing:\s*border-box;/,
    );
  });
});
