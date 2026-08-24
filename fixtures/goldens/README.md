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

M4 additions (2026-08-20): analysis goldens include D4/D5 rule counts (H14_UNSUPPORTED_ENTRY,
H11_EARLY_AGGRESSIVE_DEATH, H11_SLOW_ROTATION, H6_PUSH_WITHOUT_INFO — class 11 live). Volumes on
refresh: mirage-tie +1 early-aggressive death; navi +1 slow rotation — precision-first, no spam.

## M5 hand-verification (2026-08-20)

D6 corpus pipeline cross-checked end-to-end on the real app DB: the navi
mirage pro demo produced 8 occupancy grids (freeze_end/early = 115 samples
= 23 rounds × 5 alive players); an independent Python/SQL recomputation of
cells, pooled densities, nearest-rank threshold and recurrence gating
predicted exactly the 4 insights the app wrote ((CT,early) 6 rounds,
(CT,mid) 5, (T,early) 11 — evidence capped at 8, (T,mid) 7; threshold 1).
Verified under the documented dev-only `CLUTCHFACTOR_CONFIG` gate override
(min_demos_per_map 1); dev D6 rows were removed afterwards — the shipped
gate (8) keeps D6 silent until the owner supplies a real corpus.

## V1.2 hand-verification (2026-08-22)

Round-by-round coaching (issue #9; ADR-0008). Stale `round_review` rows
(computed before an engine fix) were deleted from the dev DB first; the
lazy backfill on `get_round_review` recomputed all five `fixtures/own/`
matches under the current engine.

**§12 impact cross-check — two owner matches, three rounds each, hand
recomputed from raw SQL (`kills`/`bomb_events`/`round_sides`) against
`win_prob_v1.yaml` cell values, replaying the ADR-0008 8-point model by
hand (state replay, per-event ΔP, the terminal defuse/explode latch, and
the `ct_alive == 0` / `t_alive == 0 && !planted` rule-clamps in
`winprob.rs::p_ct_win`, which are *not* raw table rows). All six matched
the app's stored `round_review.impact` exactly:**

| Match | Round | Hand-computed impact | App `impact` | Verdict | Note |
|---|---|---|---|---|---|
| inferno-loss (id 3) | 2 | 0.376732 → **0.3767** | 0.3767 | quiet | Positive impact, round lost anyway (won 2 duels post-plant, then a post-explosion "world" death — both players killed by the bomb itself after the terminal latch — correctly scored `delta_p: None`). |
| inferno-loss (id 3) | 6 | −0.275453 → **−0.2755** | −0.2755 | not_on_you | Tracked death backed by a real `H2_BAITED_TRADE` flag (non-following teammate 1,575 u back) — verdict precedence confirmed: NotOnYou beats what would otherwise be CostYou at this impact. |
| inferno-loss (id 3) | 12 | −0.270862 → **−0.2709** | −0.2709 | cost_you | Single unsupported death, no exculpatory flag. |
| mirage-tie (id 8) | 8 | 0.315918 → **0.3159** | 0.3159 | won_it | Single entry kill, comfortably won. |
| mirage-tie (id 8) | 19 | −0.399428 → **−0.3994** | −0.3994 | cost_you | Tracked **self-kill** (`attacker == victim`, weapon `world`) on real data — confirms the self-kill-counts-once-as-a-death path outside its unit test. |
| mirage-tie (id 8) | 24 | 0.273721 + 0.115170 + 0.039448 = **0.4283** | 0.4283 | won_it | Kill, then plant, then the round-winning kill that drops CT to 0 alive — that last event only scores because `p_ct_win` rule-clamps `ct_alive == 0 → Some(0.0)` instead of falling through to an absent table row; live app screen showed the identical per-moment deltas (+27%, +12%, +4%) and the +43% summary. |

Exact match on 6/6, spanning all four non-quiet-adjacent verdicts, the
terminal latch, the `ct_alive == 0` rule-clamp, and a real self-kill.

**Hard rules, verified on real data:**
- `not_on_you` presence iff an `H2_BAITED_TRADE` flag exists: the flag
  exists (inferno-loss R6, ×1 in the library) and `not_on_you` is surfaced
  both there and (unselected, low-impact) at inferno-loss R9 and dust2-loss
  R13 — rule is satisfiable and satisfied, not vacuously true.
- No fault language in any `why_it_mattered`: read all 21 selected rounds'
  narration across the library (18 dumped live via
  `get_round_review`/devtools console, the remaining 3 already visible
  on-screen from other checks). Every line is one of the fixed factual
  templates in `cf-narrator::rail` ("You closed it out on X: +N% win
  probability, and it held.", "You were the last event that mattered: the
  round tipped N s after your death.", etc.) — no blame-toned text is
  reachable from the code at all (`rail.rs` narration is fully
  deterministic; grepped for fault/blame/econ/buy/`$` language — none).
- `won_it` guarantee verified against the real candidate list, under both
  thresholds: at the pre-calibration default (0.18) it fired for real on
  this data — nuke-tie R12 (impact 0.2166) and inferno-win R11 (impact
  0.2084) were each the single highest-impact **cut** `won_it` candidate in
  their match and were swapped in for the weakest non-`won_it` selection,
  exactly per the single-swap spec, while sibling cut `won_it` rounds
  (inferno-win R5, R9, impact 0.2057 each) correctly stayed cut. At the
  shipped 0.25 default, re-checked the full post-calibration candidate list
  across all 5 matches: zero `won_it`-verdict rounds are currently cut by
  the cap anywhere in the library (every `won_it` round that clears the
  raised bar is small enough in count to fit inside the cap on its own), so
  the guarantee is presently inactive on this data — not broken, just
  unexercised; the mechanism's correctness on real (not just synthetic)
  numbers is the 0.18-era evidence above, and `round_review.rs`'s
  `won_it_guarantee_swaps_weakest` unit test still covers the swap path
  directly.

**14 acceptance criteria — walked live, fresh `pnpm tauri dev`, real
matches (inferno-win id 4, inferno-loss id 3, mirage-tie id 8):**

| # | Criterion | Result | Evidence |
|---|---|---|---|
| 1 | Rail shows only algorithm-selected rounds' coaching | PASS | Unselected rounds render the one-line quiet summary only (e.g. inferno-loss R1: "Nothing here needed the coach — you won it, 0-0, 5v5"); selected rounds render header/moments/why/practise. |
| 2 | Never exceeds `max_rounds` | PASS | SQL across all 5 matches post-tune: selected counts 6, 4, 4, 2, 5 — never above the cap of 6. |
| 3 | ≥1 Won it when one qualifies | PASS | won_it surfaced and selected in 3 of 5 matches (nuke-tie R1/R15, inferno-win R15/R20, mirage-tie R8/R24); the two matches without one (inferno-loss, dust2-loss) have no round clearing the WonIt bar. |
| 4 | Moment click jumps playback | PASS | Clicked "0:50 Kill kuangzhitian down" (inferno-win R15) — scrubber jumped to exactly 0:50, roster/kill-feed state updated to match. |
| 5 | Playback highlights moments as it passes | PASS | Pressed Play from 0:50; on autoplay reaching 0:55 the third moment row's active-highlight advanced automatically, unprompted. |
| 6 | Playback never auto-pauses | PASS | Started playback, then clicked a moment row mid-play (inferno-win R2) — the button stayed "Pause" (still playing) the whole time; switching rounds only resets state because each round is a fresh player instance, never an interrupt of the current one. |
| 7 | Prev/next skips unflagged | PASS | From R15 (selected), nav showed "← R7 / R20 →", skipping unflagged 16-19; from inferno-loss R6, nav showed "← R2 / R12 →", skipping unflagged 3-5 and 7-11. |
| 8 | Every moment carries numeric evidence | PASS | Every moment line seen carried a number: win-prob deltas (+28%, +43%...), distances ("1,575 u away"), or seconds ("62 s later"). |
| 9 | Replay highlights only EvidenceRef/moment focus players | PASS | Live death moment (inferno-win R2) drew a dashed line + "118 u" tag to exactly the named teammate/killer pair; no other player annotated. |
| 10 | Not on you only rule-established | PASS | Inferno-loss R6 shows "Not on you" backed by the real `H2_BAITED_TRADE` flag's own facts (teammate name, distance) rendered in the moment — never inferred from absent flags. |
| 11 | No unsupported causal claims | PASS | All narration is numbers-first and factual (see the why-it-mattered dump above) — no line asserts an unevidenced cause. |
| 12 | Quiet rounds summary-only | PASS | Inferno-loss R1: single sentence, no moments/why/practise sections rendered. |
| 13 | Zero economy/buy text anywhere | PASS | Grepped `cf-narrator::rail` source (the entire narration surface) for econ/buy/$/price — no matches besides the doc-comment's own negative statement; visually confirmed on every screenshot. |
| 14 | Attention dots use no color channel | PASS | Zoomed round-strip screenshot: dots render in chalk white/grey only (`--chalk-faint`/`--chalk-bright`), distinguished by size, never hue — the blue/orange winner underline is a separate, pre-existing element. |

**14/14 PASS.**

**Threshold calibration:** `attention_threshold_p` raised from 0.18 to 0.25
(ADR-0008's Calibration section has the full before/after numbers and
rationale) — 0.18 saturated the 6-round cap on all 5 owner matches
(candidates 15/11/13/12/11); 0.25 leaves only the closest match (nuke-tie,
a 12-12 tie) at the cap, with the other four now selecting fewer than 6
rounds on the threshold alone.
