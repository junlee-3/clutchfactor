// Route -> Sidebar width decision, used by AppShell (design-system.md §7):
// immersive screens (Replay, Report) collapse the sidebar to a 56px rail so
// the tape stays hero; every other route gets the full 216px sidebar.
// Extracted as a pure function (no React/router imports) so it's directly
// testable in vitest's node environment — see shellMode.test.ts.

export type ShellMode = "full" | "rail";

const IMMERSIVE_PREFIXES = ["/replay", "/report"];

// Matches on a path-segment boundary, not a raw string prefix: a plain
// `startsWith(p)` check would also match a future sibling route whose name
// merely starts with the same characters (e.g. "/reports".startsWith(
// "/report") is true), silently collapsing it to the rail. Requiring an
// exact match or a "/" boundary after the prefix rules that out.
export function shellMode(pathname: string): ShellMode {
  const immersive = IMMERSIVE_PREFIXES.some(
    (p) => pathname === p || pathname.startsWith(`${p}/`),
  );
  return immersive ? "rail" : "full";
}
