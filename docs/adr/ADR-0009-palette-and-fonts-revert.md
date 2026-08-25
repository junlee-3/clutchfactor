# ADR-0009: Revert to the v0 palette and system fonts

**Status:** accepted · 2026-08-25 · supersedes ADR-0007

## Context
V1.1 shipped "The Film Room" — a warm graphite palette, chalk-as-accent, and
three bundled OFL faces (Fraunces/Inter/JetBrains Mono, ADR-0007). After
using V1.2 the owner asked for the old look back: the v0 cool-navy palette
and the v0 system font stacks, keeping the V1.1 structure (sidebar shell,
`ui/` components, MatchHeader, skeletons, toasts, one focus treatment).

## Decision
- `tokens.css` is re-valued to the v0 palette; `--chalk*` tokens are renamed
  `--ink*` (text only); `--accent` (= v0's CT blue) returns as the single
  interaction hue for primary buttons, the focus ring and progress fills.
  `--ct` keeps the same value as a separate token so side identity stays
  explicit.
- Fonts are system stacks (`-apple-system … sans-serif`, `ui-monospace …
  monospace`); `--font-display` aliases the sans stack. `assets/fonts/` and
  all `@font-face` rules are deleted; ADR-0007's licensing obligations no
  longer apply (nothing is vendored).
- The dashed grammar stays for evidence only; the rail's active row uses a
  solid tone edge (loss/win/neutral); verdict chips are outlined.

## Consequences
- No font licensing surface; ~700 KB smaller bundle; platform-native text.
- `docs/design/design-system.md` is re-issued as v2 (a reference, not an
  essay). Screenshots and walkthrough are recaptured in V1.2b's final task.
- Token names are neutral (`--ink*`), so a future palette change is values
  only.
