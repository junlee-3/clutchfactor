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
