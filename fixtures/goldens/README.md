# Golden snapshots — provenance & hand-validation

Golden JSONs here are committed; the source demos are gitignored (see `fixtures/README.md`).

| Golden | Source demo (gitignored) | Origin | Validated against |
|---|---|---|---|
| `navi-javelins-mirage.match.json` | `fixtures/public/navi-javelins-vs-9-pandas-fearless-m1-mirage.dem` | demofile-net public test bundle (`demofile-net-demos-9.zip`, R2 bucket used by their MIT-licensed CI) | (1) demofile-net's committed, maintainer-verified snapshot `RoundStartEnd([v13963…]).txt`: **23 round_end events — 15 × winner side 2 (T), 8 × winner side 3 (CT)**, ticks matching exactly; (2) HLTV/egamersworld: NAVI Javelins beat 9 Pandas Fearless 2:0 on 2023-10-19 (ESL Impact S4 Europe); our derived team score is **13–10 with the NAVI Javelins roster (Liina, Hanka-, LETi-, Angelka, vicu) on 13** ✓. Numeric reason codes cross-checked against `#SFUI_Notice_*` messages (9 = CTs eliminated, 8 = Ts eliminated, 7 = defused, 1 = exploded). |
| `mirage-tie.match.json` | `fixtures/own/mirage-tie-18-8-2026.dem` | Owner matchmaking demo (2026-08-18) | Known match reality: 24 rounds, **12–12 tie** ✓ (derived score follows rosters through the halftime swap); every round 5v5 side assignment; 183 kills matching the M0 event count; misosoupy3 (76561199228328773) on the roster; round winners/ticks match the round-event probe output. |

Validation performed 2026-08-19 (M0):

- Round ends: our 23 `round_end` events match demofile-net's verified snapshot exactly — same count, same winner split (15 T / 8 CT), and identical ticks spot-checked for the first 10 rounds (5478, 10805, 18154, 24965, 33042, 41063, 49461, 60084, 71624, 80156).
- Team accounting across the halftime swap yields 13–10, consistent with NAVI Javelins' 2:0 series win on 2023-10-19.
- Kill feed sanity: `map: de_mirage`; 170 kills across exactly the 10 real rosters' players (vicu 20, victoria 20, Liina 19, t4tty 18, LETi- 18, Angelka 17, f6tal 16, Elizabeth- 16, Hanka- 13, Ksu 11) + 2 `planted_c4` world deaths; round 1 weapons are pistol-round-correct (glock/elite).
- Golden test controls: corrupting the golden makes the test FAIL (negative control); restoring it passes (positive control); test skips cleanly when the demo file is absent (CI path).

## Analysis goldens (M3)

`mirage-tie.analysis.json` (tracked: misosoupy3) and `navi-javelins-mirage.analysis.json`
(tracked: vicu) snapshot per-rule flag counts, the death-class distribution, and the
**class-13 share** — the spec's CI regression metric.

Hand-verification performed 2026-08-19 (M3, §12 — ≥3 flagged instances per family,
independently cross-checked against raw DB tables, plus a replay-viewer spot check):

- `H2_ISOLATED_DEATH` ×3 (mirage r3/r11/r12): teammate distances recomputed by SQL from
  tick_samples — nearest living teammate 1813 u (BackAlley vs SideAlley), 943 u (Stairs vs
  TopofMid), 1560 u (Apartments vs Catwalk); all > 900 u, different places, untraded ✓.
- `H4_KILLED_WITHOUT_CONTACT` ×6 (mirage): both "smoke" variants have `thru_smoke=1` in the
  kills table (p250 r14, AWP r20); no-contact variants have no victim shots/damage ✓.
- `H3_WASTED_UTILITY` ×3 (nuke): held-item lists match raw pre-death inventory samples
  exactly (incl. r12: died holding Flash+Smoke+HE+Incendiary) ✓.
- `H16_FIRE_LINGER` ×1 (inferno-loss r9): flagged 75 dmg / 2.8 s = exact sum of raw inferno
  hurts (3+4+4+8×8 over 179 ticks); the 19-dmg and 4-dmg burn episodes correctly stayed
  under threshold ✓.
- `H2_BAITED_TRADE` ×2 (inferno-loss): details name the dead teammate, killer, and
  non-following teammate at 1575 u / 1135 u (> 700 u trade range) ✓; team_pattern marker
  fired (failed×4 + baited×2).
- `H5_DIED_FLASHED` zero across all demos — verified truthful: the tracked player died inside
  a blind window exactly once in 3 matches, and the blinder was a *teammate* (rule requires
  an enemy flash) ✓.
- Replay spot check: mirage r3 flagged death visible in the viewer's kill feed at 0:32
  ("doctorwu2021 ak47 ⌖ misosoupy3") at the evidence tick ✓.
- Spec sanity: class 14 non-zero across demos (4 total), class 2 fired once in 4 matchmaking
  demos (~matches the spec's 0.8 % volume), class-13 share 21–47 % per match, class 15 at
  0–2 per match (deaths with no engagement evidence and no rule match — expected to shrink
  when H1/H5-audio/H8 land).
