# ADR-0007: Bundled fonts — sourcing and licensing

**Status:** superseded by ADR-0009 · 2026-08-25 (was: accepted · 2026-08-22)

## Context
The design system (`docs/design/design-system.md` §3) gives the app three
faces with three distinct jobs — Fraunces for the coach's editorial voice,
Inter for UI/body, JetBrains Mono for every numeral — and the charter
requires them bundled locally (`assets/fonts/`, no CDN) since ClutchFactor is
an offline desktop app. All three are SIL Open Font License families; OFL
requires the license text travel with the font binary, not just be linked.

## Decision
Vendor one file per family/style directly from each family's own upstream
release (not the google/fonts mirror, which ships TTF only and would require
a conversion step this task is not permitted to add):

- **Fraunces** — variable (opsz, wght, plus SOFT/WONK axes at default 0),
  upright + italic, from `undercasetype/Fraunces` release `1.000`
  (`Fonts - Web/Fraunces[SOFT,WONK,opsz,wght].woff2` and the `-Italic`
  sibling).
- **Inter** — variable (wght, opsz), from `rsms/inter` release `v4.1`
  (`web/InterVariable.woff2`).
- **JetBrains Mono** — static Regular only (spec does not mark this face
  variable; UI never bolds mono data), from `JetBrains/JetBrainsMono`
  release `v2.304` (`fonts/webfonts/JetBrainsMono-Regular.woff2`).

Every file confirmed `Web Open Font Format (Version 2)` via `file` and under
1 MB. Each family's `OFL.txt` is vendored beside its woff2 as
`assets/fonts/OFL-<family>.txt`; the Fraunces license (absent from the
release zip) was pulled from the canonical `undercasetype/Fraunces` repo and
diffed byte-identical against the google/fonts mirror copy before being
committed, so its provenance is verified independent of a single source.

## Consequences
≈880 KB total font payload (well inside the ADR's own 1 MB/file ceiling and
the ≈2-3 MB budget implied by three families). OFL 1.1 permits bundling and
redistribution with the required attribution, which the vendored `OFL-*.txt`
files satisfy. Swapping a family later is a two-file change: replace the
woff2(s) under `assets/fonts/` and update the matching `@font-face src` in
`src/styles/base.css` — `tokens.css`'s `--font-*` values don't need to move.
