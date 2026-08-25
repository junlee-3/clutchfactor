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
| 12 | Quiet rounds summary-only | PASS (corrected 2026-08-25) | Original 2026-08-22 check only looked at inferno-loss R1 (single sentence, no moments/why/practise) and missed a real defect: inferno-loss R2 (+0.3767 impact, round lost, no exculpatory rule → `quiet` per the verdict precedence — see the §12 impact cross-check table above) was selected anyway, rendering a bright dot and full moments — `select_rounds` thresholded on `\|impact\|` alone and never checked verdict. Fixed in the V1.2 final-review fix wave (finding #1; ADR-0008 "Final-review fixes" amendment): `Quiet`-verdict rounds are now excluded from selection candidacy regardless of impact magnitude. Re-verified live post-fix: inferno-loss R2 now shows NO attention dot and renders the one-line quiet summary only, same disposition as R1 — see the "V1.2 final-review fix wave" section below for the full re-verification. |
| 13 | Zero economy/buy text anywhere | PASS | Grepped `cf-narrator::rail` source (the entire narration surface) for econ/buy/$/price — no matches besides the doc-comment's own negative statement; visually confirmed on every screenshot. |
| 14 | Attention dots use no color channel | PASS | Zoomed round-strip screenshot: dots render in chalk white/grey only (`--chalk-faint`/`--chalk-bright`), distinguished by size, never hue — the blue/orange winner underline is a separate, pre-existing element. |

**14/14 PASS.**

**Threshold calibration:** `attention_threshold_p` raised from 0.18 to 0.25
(ADR-0008's Calibration section has the full before/after numbers and
rationale) — 0.18 saturated the 6-round cap on all 5 owner matches
(candidates 15/11/13/12/11); 0.25 leaves only the closest match (nuke-tie,
a 12-12 tie) at the cap, with the other four now selecting fewer than 6
rounds on the threshold alone.

## V1.2 final-review fix wave (2026-08-25)

The §12 impact cross-check above verified `round_review.impact`/`verdict`
numerically but never cross-checked *selection* against verdict — a gap
the final-review pass caught. Live case: inferno-loss (id 3) round 2
(impact **+0.3767**, round lost, no exculpatory rule → `quiet` per the
verdict precedence — see the round-2 row in the §12 impact cross-check
table above) was selected anyway: bright attention dot, full moments, a
`why_it_mattered` line — everything acceptance criterion #12 says a
`quiet` round must never show. Root cause: `select_rounds` thresholded on
`|impact| ≥ attention_threshold_p` alone and never checked `verdict`, so a
large-magnitude `quiet` round cleared the bar on magnitude and got
selected. Fixed (`round_review.rs`'s `select_rounds`; ADR-0008's
"Final-review fixes" amendment): a `Quiet`-verdict round is now excluded
from selection candidacy regardless of `|impact|`.

**Re-verified live, post-fix:** `pkill -f clutchfactor`; cleared the dev
DB's `round_review` table (pre-migration-0007 rows have no fingerprint to
compare, so a manual clear was still needed this one time — future engine
changes recompute automatically via the new `cfg_fingerprint` column);
fresh `pnpm tauri dev`; opened inferno-loss (match id 3).

- **Round 2** (the fixed case): the round chip shows NO attention dot
  (round 6/12's dots are visible for comparison in the same screenshot);
  the rail reads "Round 2 · Quiet · CT · lost · you 2-1 · 3v0" and renders
  only "Nothing here needed the coach — you lost it, 2-1, 3v0." — no
  moments list, no why-it-mattered, no what-to-practise. Screenshot:
  `t-finalfix-r2-quiet.png`.
- **Round 6** (`not_on_you`, spot-checked as a previously-selected non-quiet
  round): dot still present above its chip; full rail renders unchanged
  from the 2026-08-22 pass — "Round 6 · Not on you", the `H2_ISOLATED_DEATH`
  moment ("Nearest: Mashed Potato", "1,575 u away at Banana", "Mashed
  Potato 1,575 u back when SirEggsAlot went down — never in trade range",
  "Not traded — round lost 62 s later") and its what-to-practise line
  ("Before you take a fight at Banana, know who is close enough to trade
  you.") — confirming the fix only removed `Quiet` rounds from candidacy
  and didn't disturb any other verdict's selection or rendering.
  Screenshot: `t-finalfix-r6-notonyou.png`.

Screenshots (plus the Library and pre-fix Match Report/Replay-launch shots)
live in
`/private/tmp/claude-501/-Users-junlee-Documents-programming-clutchfactor/ed626285-4a44-4fc2-9c1f-bff2273fbf21/scratchpad/`,
prefixed `t-finalfix-`.

## V1.2b hand-verification (2026-08-25)

Play ledger + re-analyze (spec `docs/spec/play-ledger-and-coach.md` §2, §4
DoD). Run from the `docs/v1.2b-verification` worktree against the owner's
real dev DB (`~/Library/Application Support/com.clutchfactor.app/clutchfactor.db`,
backed up first with `sqlite3 .backup`; 5 `fixtures/own/` matches imported
before the ledger existed + 1 corpus demo). Tracked `76561199228328773`
(misosoupy3). All distances below are the engine's metric — nearest sample at
or before the tick, `sqrt(dx² + dy² + (2·dz)²)` (z-weight 2.0), rounded to
whole units; seconds are ticks/64 to 1 dp; "window" = 128 ticks (2 s).

### A. Backfill through the real picker flow

Before launch the DB was at migration 0007: no `matches.source_path`, no
`round_plays`. First launch applied 0008 (column added, table created, all
six `source_path` NULL, 0 ledger rows). Every own match was then re-analyzed
from the Library's **Re-analyze** button: the first call returned
`needs_file` (no `source_path`) and opened the native panel titled
"Locate <file_name>", which was driven with AppleScript (⌘⇧G → the absolute
fixture path → Return → Return). Parse + analysis + ledger per match:
mirage-tie < 30 s, inferno-loss 14 s, dust2-loss 14 s, inferno-win 18 s,
nuke-tie 20 s (debug build).

```sql
SELECT match_id, COUNT(*) FROM round_plays GROUP BY match_id;
-- 2|24  3|16  4|20  7|17  8|24
SELECT match_id, COUNT(*) FROM rounds GROUP BY match_id;
-- 2|24  3|16  4|20  6|23  7|17  8|24        (6 = the corpus demo: no tracked player, no ledger, not in the Library)
SELECT id, source_path FROM matches WHERE kind='own';
-- every row now carries its absolute fixtures/own/<file>.dem path
```

Play kinds written (per match, from `plays_json`): nuke-tie 24 setup / 24
outcome / 19 death / 13 kill / 10 assist / 23 missed_trade / 4 he / 3 molotov
/ 4 smoke / 2 rotation / 2 plant / 1 defuse; inferno-loss 16/16, 15 death, 6
kill, 7 missed_trade, 1 trade, 1 rush, 1 rotation, 1 flash, 4 he, 3 smoke, 2
plant, 2 bare `flag`; inferno-win 20/20, 14 death, 16 kill, 4 trade, 6
missed_trade, 4 flash, 7 molotov, 5 `flag`; dust2-loss 17/17, 17 death, 6 kill,
13 missed_trade, 5 flash, 2 rotation; mirage-tie 24/24, 19 death, 7 kill, 10
missed_trade, 1 trade, 4 smoke, 1 plant. Quality tags only where a measure
backs them (death Bad/Neutral, flash Good/Bad/Neutral, he/molotov Bad on team
damage, trade Good, missed_trade Bad/Neutral, rush Neutral, kill Bad on a
teamkill).

### B. Three rounds recomputed by hand from raw SQL

Each row: the raw query → the hand number → the stored ledger fact → the rail
line the app rendered (read back from the Replay screen's accessibility tree,
verbatim). **Every fact and every line matched; no fix commit was needed.**

**inferno-loss (id 3) R6 — CT, lost (`ct_killed`); `start 25475 · freeze_end
26435 · end 32226 · officially_ended 32674`.** Flags on the tracked death tick
28287: `H2_ISOLATED_DEATH` (sev 0.8) + `H2_BAITED_TRADE` (sev 0.35, the
exculpatory rule).

| Check | SQL (match_id=3) | Hand | Ledger fact | Rail line |
|---|---|---|---|---|
| Setup checkpoint 26435+320 = **26755** | `tick_samples` latest ≤ 26755 for tracked + the 4 CT teammates (all sampled at 26752) | me (1677.5, 2769.7, 124.0) CTSpawn → SirEggsAlot **159.50** (f32 159.4955 → 159), Mashed Potato 323.9, Crunchy Potato 1527.8, Roland 1813.3 → nearest SirEggsAlot 159; within 900 u: 2 of 4 | `nearest_teammate` SirEggsAlot, `nearest_teammate_dist` 159, `teammates_within_isolation` 2, `teammates_alive` 4, place CTSpawn | "0:05 Setup at CT spawn · Nearest teammate SirEggsAlot, 159 u · 2 of 4 teammates within 900 u" |
| Flashes | `blinds WHERE attacker=tracked AND tick BETWEEN 25475 AND 32674`; `grenades WHERE thrower=tracked …` | 0 rows / 0 rows — the tracked player threw nothing in R6 | no flash/smoke/he/molotov play (silence) | (none) |
| Missed trade @ **28215** (SirEggsAlot ← MyUnit) | samples at 28212; `shots`/`hurts` by tracked in [28215, 28343]; `kills WHERE victim=MyUnit AND tick ≤ 28343` | me↔SirEggsAlot **304.53** ≤ 700; 5 shots (28258–28284) + 30 dmg on MyUnit at 28284 ⇒ committed; MyUnit not killed in the window ⇒ not traded by team | `missed_trade` distance 305, `committed` true, `traded_by_me` false, `traded_by_team` false, quality neutral, no rule (H2_FAILED_TRADE correctly did not fire) | "0:27 Trade on SirEggsAlot missed · You fired, but MyUnit lived 2 s" |
| Death @ **28287** by MyUnit (galilar, HS) | samples at 28284 for tracked, MyUnit, living CT; `kills WHERE victim=MyUnit AND tick BETWEEN 28287 AND 28415` | me↔MyUnit **666.61** → 667; nearest living teammate Mashed Potato **1575.18** → 1575 (Crunchy 1666.4, Roland 1966.6, SirEggsAlot dead); MyUnit not killed in the window ⇒ not traded; (32226−28287)/64 = **61.55** → 61.5; before the kill CT 4 (SirEggsAlot dead) v T 4 (Ismoothy dead) | `killer_distance` 667, `nearest_teammate` Mashed Potato 1575, `traded` false, `round_end_delta_s` 61.5, `dead_time` false, `man_context` 4v4, `rule_id` H2_ISOLATED_DEATH, **`exculpatory` true, `quality` neutral**, merged `their_distance` 1575.18 / `non_following_teammate` Mashed Potato / `dead_teammate` SirEggsAlot | chip **"Not on you"**; "0:28 Died to MyUnit · 667 u, galilar, headshot · Nearest: Mashed Potato · 1,575 u away at Banana · Mashed Potato 1,575 u back when SirEggsAlot went down — never in trade range · Not traded — round ended 62 s later · 4v4 before" |
| Outcome @ 32226 | roster minus `kills` at or before 32674; `hurts` by tracked on T in the span | CT 5 − 5 = 0; T 5 − 5 = 0 (MyUnit died to the bomb at 32530, after the round ended) → 0v0; kills 0; damage 30 (m4a1 on MyUnit) | `won` false, `my_alive` 0, `their_alive` 0, `kills` 0, `damage` 30, `reason` ct_killed | "1:30 Round lost — ct killed · 0v0 at the end · 0 kills, 30 damage" |

The only tracked flash in this demo is R15 @ **89265** (grenade event tick =
blind tick), checked in its place: 7 `blinds` rows — enemies ≥ 1.1 s: MyUnit
5.10, Ismoothy 4.81 (CT, tracked is T after the swap) → **2**; teammates ≥ 1.1 s:
Roland 4.84, Mashed Potato 4.58, SirEggsAlot 5.16, Crunchy Potato 3.56 → **4**;
self 1.01 s < 1.1 → not self-blind; no kill in [89265, 89393] → not converted.
Ledger: `enemies_blinded` 2, `teammates_blinded` 4, `self_blind` false,
`converted` false, quality **bad** ✓. All four tracked HEs also match their
`hurts` (`weapon='hegrenade'`, 0.5 s window): R10 @56438 17 + 8 = **25** on Konky
+ MyUnit → `enemy_damage` 25 / `victims` both ✓; R11 @67372 **22** on Ismoothy ✓;
R15 @88846 no hurt rows → **0** ✓; R16 @94017 **25** on MyUnit ✓; team/self damage
0 everywhere ✓.

**inferno-loss (id 3) R2 — CT, lost (`bomb_exploded`); `start 3379 ·
freeze_end 4339 · end 8952 · officially_ended 9400`.** No tracked flags.

| Check | SQL (match_id=3) | Hand | Ledger fact | Rail line |
|---|---|---|---|---|
| Setup @ **4659** | samples at 4656 | me (2153.7, 2543.1, 124.0) CTSpawn → Roland **431.56** → 432, Mashed Potato 575.2, SirEggsAlot 677.7, Crunchy Potato 1393.1 → 3 of 4 within 900 | nearest Roland 432, within 3, alive 4 | "0:05 Setup at CT spawn · Nearest teammate Roland Pryzbylewski, 432 u · 3 of 4 teammates within 900 u" |
| Rotation (plant @ **6328** by Chet at BombsiteA) | samples for Chet at 6328, tracked at 6328 / 6968 / 7032; `rotate_radius_u` 800 | me at Banana **2320.13** → 2320 from the plant; 10 s later 997.7 (> 800), 11 s later **793.2** (≤ 800) → arrived at 11 s | `distance_at_plant` 2320, `at_site` false, `arrived_s` 11.0, `died_before_arrival` false | "0:31 Rotated to the plant in 11 s · 2,320 u from the plant when it went down" |
| Kill @ **7887** Konky (mp9) | samples at 7884; `kills` before 7887 | **1436.27** → 1436; before: CT 3 (Crunchy 6010, SirEggsAlot 6838 dead) v T 5 | `killer_distance` 1436, `man_context` 3v5 | "0:55 Killed Konky · 1,436 u, mp9 · Konky down · +2% win probability · 3v5 before" |
| Kill @ **8249** MyUnit (mp9) | samples at 8248 | **775.34** → 775; before: T 2 (Konky, Chet 7904, Logical 8192 dead) | `killer_distance` 775, `man_context` 3v2 | "1:01 Killed MyUnit · 775 u, mp9 · MyUnit down · +35% win probability · 3v2 before" |
| **Tail death @ 8971** (`planted_c4`, no attacker — 19 ticks after `end_tick`) | `kills` row 8971; samples at 8968 | (8952 − 8971) = −19 ticks → clamped **0.0**, `dead_time`; no killer ⇒ not traded; nearest Roland **375.69** → 376 (Mashed Potato 1145.5); before: CT 3 v T 0 | `round_end_delta_s` **0.0**, `dead_time` **true**, `traded` false, `killer` null, `nearest_teammate` Roland 376, `man_context` 3v0, place Apartments, no quality | "1:12 Death · Nearest: Roland Pryzbylewski · At Apartments · **Not traded — after the round was decided** · 3v0 before" |
| Outcome @ 8952 | roster minus kills ≤ 9400; `hurts` by tracked on T | CT 5 − 4 (Crunchy, SirEggsAlot, misosoupy3 @8971, Roland @8975) = **1**; T 5 − 5 = **0**; kills 2; damage 21+17+21+21+15+26 = **121** | `my_alive` 1, `their_alive` 0, `kills` 2, `damage` 121, `survived` false | "1:12 Round lost — bomb exploded · 1v0 at the end · 2 kills, 121 damage" |

**mirage-tie (id 8) R13 — T, won (`ct_killed`); `start 76557 · freeze_end
77933 · end 80183 · officially_ended 80631`.** Flags: `H2_FAILED_TRADE` @78958
(distance 466.40), `H2_FAILED_TRADE` @79424 (427.21), `H4_KILLED_WITHOUT_CONTACT`
@79440 (`no_contact`, p250).

| Check | SQL (match_id=8) | Hand | Ledger fact | Rail line |
|---|---|---|---|---|
| Setup @ **78253** | samples at 78252 | me (1128.3, −1017.7, −259.7) TSpawn → xnopyt **221.7** → 222, Bebita 302.5, Roland 389.3, lyra 449.9 → 4 of 4 | nearest xnopyt 222, within 4, alive 4 | "0:05 Setup at T spawn · Nearest teammate xnopyt, 222 u · 4 of 4 teammates within 900 u" |
| Missed trade @ **78958** (Bebita ← tttttssssss) | samples at 78956; `shots WHERE player=tracked AND tick BETWEEN 78958 AND 79086` → 0; `hurts` by tracked in that window → 0 (the tracked glock shots are at 78626–78694 and 79131–79179); tttttssssss died at 79150 (192 ticks > 128) | me↔Bebita **466.4** → 466 ≤ 700; no commit; not traded by team | `distance` 466, `committed` false, `traded_by_me` false, `traded_by_team` false, quality **bad**, `rule_id` **H2_FAILED_TRADE** | "0:16 Didn't trade Bebita · tttttssssss killed them 466 u from you; no shot from you in 2 s" |
| Missed trade @ **79424** (Roland ← NCZ RG) | samples at 79424; shots/hurts in [79424, 79552] → 0; NCZ RG died at 79670 (246 > 128) | **427.2** → 427; no commit; not traded | `distance` 427, `committed` false, quality **bad**, `rule_id` **H2_FAILED_TRADE** (the second same-rule flag now merges — final-review fix #2) | "0:23 Didn't trade Roland Pryzbylewski · NCZ RG killed them 427 u from you; no shot from you in 2 s" |
| Death @ **79440** by doctorwu2021 (p250) | samples at 79440; `kills WHERE victim=doctorwu2021 AND tick BETWEEN 79440 AND 79568` → 79542 | me↔doctorwu2021 **291.7** → 292; nearest lyra **371.7** → 372 (xnopyt 567.5); killer died 102 ticks later ⇒ traded; (80183−79440)/64 = **11.6**; before: T 3 v CT 4 | `killer_distance` 292, `nearest_teammate` lyra 372, `traded` true, `round_end_delta_s` 11.6, `dead_time` false, `man_context` 3v4, `rule_id` H4_KILLED_WITHOUT_CONTACT, quality bad | chip "Traded"; "0:23 Died to doctorwu2021 · 292 u, p250 · Nearest: lyra · At A site · Traded — round continued 12 s after · 3v4 before" |
| Utility | `grenades`/`blinds` by tracked in the span; `hurts WHERE weapon IN ('hegrenade','inferno','molotov','incgrenade')` by anyone in the span | nothing thrown by the tracked player; no HE/fire damage by anyone in R13 | no utility play (silence) | (none) |
| Outcome @ 80183 | roster minus kills ≤ 80631; `hurts` by tracked by weapon/victim side | T 5 − 3 = **2**; CT 5 − 5 = **0**; kills 0; damage glock **11** on CT | `won` true, `my_alive` 2, `their_alive` 0, `kills` 0, `damage` 11 | "0:35 Round won — ct killed · 2v0 at the end · 0 kills, 11 damage" |

Observations (truthful per the spec's definitions, logged for the V1.3 coach
rather than fixed here): the outcome's alive counts are taken at
`officially_ended_tick`, so post-decision bomb deaths count (R6 reads "0v0 at
the end"; R2 "1v0" while its tail death reads "3v0 before"); a death with no
rule renders "Nearest: <name>" without the stored `nearest_teammate_dist`
(R2: 376 u is in the facts, not in the caption); `H4_KILLED_WITHOUT_CONTACT`'s
`variant` isn't narrated in the ledger caption; two same-clock rows in R2
("1:12 Round lost" before "1:12 Death") are tick-ordered truth (8952 < 8971).

### C. Re-analyze acceptance (Library → Re-analyze, dust2-loss id 7)

| # | Case | What happened | Evidence |
|---|---|---|---|
| a | Pre-V1.2b import, `source_path` NULL → picker → correct file | Every one of the five matches in Part A: `needs_file` → "Locate <file>" panel → fixture → progress ("Checking the demo file 0%" → parsing → analysis) → toast "Re-analyzed <Map> — play-by-play is ready for every round." → ledger rows for every round, `source_path` recorded | `round_plays` counts above; toast text read from the AX tree |
| b | Wrong file → hash error, ledger unchanged | `UPDATE matches SET source_path='…/nuke-tie-18-8-2026.dem' WHERE id=7` (a stale stored path) → Re-analyze → the picker opened (the stored-path hash mismatch is `needs_file`, not an error — final-review fix #4) → picked the nuke demo → error toast **"That file isn't dust2-loss-18-8-2026.dem — its contents don't match the imported demo. Pick the original file."** 4 s after the pick; `round_plays` for id 7 md5 `3a09d7b6…` before and after (unchanged), `source_path` unchanged | toast text from the AX tree; md5 over `round, plays_json, timeline_json` |
| c | Cancelled picker → no change, no error | Re-analyze → picker → Escape → no toast, no progress, md5 unchanged, `source_path` unchanged | AX dump after cancel had no status/error text |
| a′ | Then the correct file | picker → dust2 fixture → toast "Re-analyzed Dust2 — play-by-play is ready for every round.", `source_path` back to the dust2 path, 17 rows, ledger md5 identical to the earlier run (the ledger is deterministic) | SQL |

Picker flow driven end to end through the real native panel (no SQL
fallback needed); `commands.rs`'s `resolve_candidate…` / `hash_mismatch…`
unit tests cover the same branches off-screen.

### D. Walkthrough (`docs/design/walkthrough-v1.2b/`, window 1440×900)

Captured from the running app at a true 1440×900 (the Dock was auto-hidden
for the capture — macOS otherwise clamps the window to 1440×888 on the
owner's display; restored afterwards). Graded against
`docs/design/design-system.md` v2:

| Screen | §2 color | §3 type | §4 space/radius/motion | §5 dashed grammar | Verdict |
|---|---|---|---|---|---|
| `library.png` | navy canvas/surfaces; win/loss row edges the only outcome hue; accent only on "Import demos" and focus; radar thumbnails at 80% on `--bg-tape` | sans map names (600), mono score/K-D/HS/rounds/dates | `--r-md` rows, `--s4` padding, 56×56 `--r-sm` thumbnails, 1px `--line` | no dashes (nothing here is evidence) | PASS |
| `replay-rail.png` (inferno-loss R2, playhead 0:55) | radar well on `--bg-tape`; CT/T only on rosters, round chips, kill feed; active row's **solid 2px win-tone edge** (+2% play); verdict chip "Quiet" **outlined neutral** | rail header sans, context line + timestamps + facts mono; headlines sans | `--r-lg` radar well, `--r-md` cards, 4px-grid gaps | dashed only on evidence (the death annotation's teammate line appears only inside a death moment); the active row is a solid edge, never dashed | PASS |
| `report.png` | coach-note lead + insight cards with `--loss` severity edges; class-13 bar in `--win` (good news), other bars ink; no ring/donut | display sans titles, mono percentages/evidence chips | Card/eyebrow/chip tokens | evidence chips dashed-underlined and clickable — the only dashes on screen | PASS |
| `settings.png` | cards, one accent primary ("Track this player"), secondary "Clear override" | mono threshold table incl. the new `ledger.*` rows; eyebrows in micro caps | Table hairlines, Input `--r-sm` | none | PASS (copy nit fixed in this PR: the tracked-player note still told the user to delete and re-import a match — it now points at Re-analyze) |

`docs/screenshots/{library,replay,report,corpus,trends}.png` recaptured at
1440×900 (1×) from the same session; README copy unchanged (`grep -n
"Fraunces\|serif" README.md` prints nothing).
