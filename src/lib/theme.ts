// Single color source for canvas/TS. UI CSS reads tokens.css directly; this
// module resolves the SAME custom properties at runtime for code that can't
// use CSS (Canvas 2D drawing, SVG attributes built in TS) — see
// docs/design/design-system.md §2 ("no second color system").
//
// FALLBACK is a literal copy of tokens.css's hex values for contexts with no
// `document` (vitest's node environment, any future SSR-less path).
// theme.test.ts parses tokens.css and asserts FALLBACK never drifts from it.

export type TokenName =
  | "--bg0"
  | "--bg-tape"
  | "--line"
  | "--ink"
  | "--ink-bright"
  | "--ink-dim"
  | "--accent"
  | "--ct"
  | "--t"
  | "--win"
  | "--loss";

export const FALLBACK: Record<TokenName, string> = {
  "--bg0": "#0e1116",
  "--bg-tape": "#0b0d11",
  "--line": "#232b36",
  "--ink": "#dfe5ec",
  "--ink-bright": "#f2f5f8",
  "--ink-dim": "#8a94a3",
  "--accent": "#4aa3ff",
  "--ct": "#4aa3ff",
  "--t": "#f5b83d",
  "--win": "#5dbb7a",
  "--loss": "#d16a5f",
};

// getComputedStyle returns a live view bound to the element — one lookup
// reflects current styles on every property read, so caching it (rather
// than re-querying per call) is safe and cheap.
let rootStyle: CSSStyleDeclaration | null = null;

/** Resolves a color token to its live CSS value. Falls back to the literal
 *  FALLBACK copy when there is no `document` (node tests) or the property
 *  isn't set on `:root` for some reason. */
export function getToken(name: TokenName): string {
  if (typeof document === "undefined") return FALLBACK[name];
  rootStyle ??= getComputedStyle(document.documentElement);
  return rootStyle.getPropertyValue(name).trim() || FALLBACK[name];
}

function hexToRgb(hex: string): [number, number, number] {
  const m = /^#?([0-9a-f]{2})([0-9a-f]{2})([0-9a-f]{2})$/i.exec(hex.trim());
  if (!m) return [0, 0, 0];
  return [parseInt(m[1], 16), parseInt(m[2], 16), parseInt(m[3], 16)];
}

/** Canvas-ready `rgba(r, g, b, a)` string for a token at the given alpha. */
export function rgba(name: TokenName, alpha: number): string {
  const [r, g, b] = hexToRgb(getToken(name));
  return `rgba(${r}, ${g}, ${b}, ${alpha})`;
}
