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

## V1.3 verification (2026-08-26)

The coach (spec `docs/spec/play-ledger-and-coach.md` §3, ADR-0010). Run from
the `docs/v1.3-verification` worktree against the owner's real dev DB
(`~/Library/Application Support/com.clutchfactor.app/clutchfactor.db`, backed
up first with `sqlite3 .backup`; 5 own matches, all with a V1.2b ledger).
Tracked `76561199228328773` (misosoupy3). The app was driven through the
accessibility tree (AppleScript), the window at a true 1440×900 (Dock
auto-hidden for the captures, restored after). Timings are wall-clock from
the button press to the row landing in `coach_cache` (polled every second)
unless stated. The owner's key stayed in the gitignored repo-root `env.local`
throughout; it was never printed, copied or put on a command line — to
disable it the file was renamed, never deleted. Gotcha for the next person:
the debug loader resolves `<crate>/../env.local` = the *worktree* root, so a
worktree run needs `ln -s <main checkout>/env.local <worktree>/env.local`
(gitignored, removed afterwards).

**Style-version note (fix round 1, 2026-08-26):** every coach read quoted or
counted below was generated and cached under `STYLE_VERSION = "coach-v2"` —
the fix that shipped `coach-v3` (tick labels confined to the `plays` array,
place names never a map slug; `docs/adr/ADR-0010-coach-architecture.md`)
landed in a follow-up commit after this verification session. The `v2`→`v3`
bump changes the cache key, so every row here will regenerate fresh on the
next open; the validator/grounding evidence below stands (it is about what
the validator catches, not the prose style), but the exact wording quoted in
§C — including the `[tick N]` labels and raw map ids called out as voice
notes — will read differently once `coach-v3` rows replace them. See
"Deviations" below for the R8/R10 vs. brief round swap.

### A. Byte-identical without a key (DoD)

`mv env.local env.local.off`; `CLUTCHFACTOR_GEMINI_KEY` unset in the launching
shell (`env | grep -c CLUTCHFACTOR_GEMINI_KEY` → 0); `DELETE FROM settings
WHERE key='gemini_api_key'` → `changes()` = 0 (no row had ever existed). First
launch of this build applied migration 0009 (`coach_cache` created, 0 rows).
The startup log had no `coach: dev key loaded` line.

| Screen | What rendered | Coach strings in the AX tree | `lsof -i -P -n \| grep -i clutchfactor` |
|---|---|---|---|
| Settings → Coach card | "STATUS off — no key", empty key field + "Save key", "Test connection" disabled, model fields at `gemini-3.7-flash` | — | none |
| Report, mirage-tie (id 8) | the V1.2b surface exactly: template lead "Mirage, 12-12 draw", round strip, insight cards, sidebar — no "Coach's read", no "Ask the coach" (compared against `docs/design/walkthrough-v1.2b/report.png`) | 0 | none |
| Replay id 8 R8 (Won it) and R10 (Cost you) | the V1.2b rail exactly: header, verdict chip, the play list, template "Why it mattered" — no "Coach's read", no "Regenerate", no "Ask the coach" (compared against `walkthrough-v1.2b/replay-rail.png`) | 0 | none |

`SELECT COUNT(*) FROM coach_cache` → 0 before and after browsing. Capture:
`docs/design/walkthrough-v1.3/settings-nokey.png`. Then `mv env.local.off
env.local`, kill + relaunch (the env var is read at startup).

### B. With the key

**Settings.** Status "on · key …1Y4A from the environment" (4 characters,
never more); the key field disabled with "The key comes from the
CLUTCHFACTOR_GEMINI_KEY environment variable and can't be edited here."

**Test connection, default model `gemini-3.7-flash`: FAILED — Google side.**
Three presses, each ending 45 s later (the reqwest timeout) in the toast
**"Couldn't reach Gemini (error sending request for url
(https://generativelanguage.googleapis.com/v1beta/models/gemini-3.7-flash:generateContent)).
The template captions are shown meanwhile."** Diagnosis, so the owner does
not chase the key: `lsof` on the app showed the TCP connection to
172.217.119.4:443 go SYN_SENT → ESTABLISHED within 1 s and stay established
for the full 45 s; `nettop` showed ~2.4 KB out / ~5.7 KB in in the first
second (TLS handshake + request sent) and nothing after — the app was
waiting for Google. A throwaway out-of-process replica of the exact request
(same reqwest features, same body, key read from `env.local` by the probe
itself, nothing key-derived printed) got: `gemini-3.7-flash` + schema →
timeout at 60 s; `gemini-3.7-flash` without schema → **HTTP 503 after 30.9 s,
"This model is currently experiencing high demand. Spikes in demand are
usually temporary. Please try again later."**; `gemini-3.5-flash-lite` +
schema → **200 in 909 ms, `{"ok": true}`**. Re-run ~15 minutes later:
3.7-flash timed out at 60 s with and without the schema, flash-lite 200 in
1.03 s. The key and the transport are fine; the default model was overloaded
for the whole session. Two debts logged
in PROGRESS: the Offline toast drops the cause (reqwest's outer message
only; "operation timed out" is in the source chain), and there is no
per-request model fallback.

**Switched both models to `gemini-3.5-flash-lite` through the Settings UI**
(the ADR-0010 documented alternative): typed into "Per-round model" and
"Match synthesis model", "Save models" → `settings` rows
`coach_round_model` / `coach_synthesis_model` = `gemini-3.5-flash-lite`.
**Test connection → "Connected — gemini-3.5-flash-lite answered in 1009 ms."**
(1009 ms is the app's own self-reported request latency, printed verbatim in
the toast text — not this section's polling stopwatch; the toast itself
appeared on screen ~1.2 s after the press, timed by that stopwatch). Capture:
`walkthrough-v1.3/settings-coach.png` (the card + that toast). The dev DB is
left on `gemini-3.5-flash-lite` so the owner sees exactly the reads checked
below; switch back in Settings when 3.7-flash is healthy.

**Report, mirage-tie (id 8), first open.** `coach_cache` rows for match 8
(all `t+` figures are this section's polling stopwatch — wall-clock from the
Report's open to the row landing, `coach_cache` polled every second):
6 at t+5.2 s, 12 at t+10.3 s, 18 at t+20.5 s, 24 at t+27.7 s, synthesis at
**t+29.8 s** — 4 round batches of 6 + 1 synthesis = **5 requests** for a
24-round match, exactly ⌈24/6⌉ + 1. The Report's "Coach's read ·
gemini-3.5-flash-lite" lead rendered with the opening + 3 work-on items
(text in C). Capture: `walkthrough-v1.3/report-coach.png`.

**Replay id 8.** R8 (Won it): "Coach's read" + focus line, a comment under
the kill play, coach "Why it mattered" replacing the template line;
**Regenerate on R8: the request took 2.6 s**, timed the same way as every
other figure in this section — wall-clock from the button press to the new
row landing in `coach_cache` (polled every second) — one request, a
different read. Separately, the row's `created_at` moved from 15:53:04 to
15:54:27 UTC, an 83 s gap; that gap is *not* the request latency and is not
offered as one — R8's row at 15:53:04 was written during the match's initial
5-request open (the "6 at t+5.2 s" batch above), well before Regenerate was
pressed, so the 83 s mostly covers the idle time between that first open and
whenever Regenerate was actually clicked, plus the 2.6 s request at the end
of it. R10 (Cost you): read + a comment under the death play; `why_it_mattered` /
`what_to_practise` / `focus` came back null so the template "Why it
mattered" stayed. Capture: `walkthrough-v1.3/rail-coach.png` (R10).

**Report, inferno-loss (id 3).** Rows (same stopwatch as mirage-tie above —
wall-clock from this Report's open, `coach_cache` polled every second): 6 at
t+25.8 s (this batch carried the one rejected round, so it was one call + one
retry), 12 at t+41.3 s, 16 at t+45.4 s, synthesis at **t+47.5 s** — 3 batches
(6/6/4) + 1 retry + 1 synthesis = 5 requests for 16 rounds. Opening + 2
work-on items rendered.

**Cache / request accounting** (`SELECT match_id, kind, round, status, model,
substr(violations_json,1,120) FROM coach_cache ORDER BY match_id, kind,
round`):

| match | kind | rows | ok | fallback | model |
|---|---|---|---|---|---|
| 3 inferno-loss | round | 16 (= rounds) | 15 | 1 | gemini-3.5-flash-lite |
| 3 | synthesis | 1 | 1 | 0 | gemini-3.5-flash-lite |
| 8 mirage-tie | round | 24 (= rounds) | 24 | 0 | gemini-3.5-flash-lite |
| 8 | synthesis | 1 | 1 | 0 | gemini-3.5-flash-lite |

Every `ok` row has `violations_json = []`. The one fallback row, verbatim:
`3|round|3|fallback|["why_it_mattered:Number:13"]` — the coach's
`why_it_mattered` for inferno-loss R3 cited "13", which is not in that
round's block (R3 facts: setup Ruins 185 u / 2 of 4, missed trade 331 u, kill
739 u mp9 4v5, death 840 u ak47 HS at B site nearest 503 u 28 s before the
end 4v4, outcome 0v3 / 1 kill / 72 damage; impact −7%), retried once with the
violation listed, rejected again, cached as `fallback` so R3 is not
re-billed. **This is the validator doing its job**, not a defect: the only
"13" the model ever saw is the match's final score 3-13 in the *batch*
header, and the per-round grounding set is deliberately the round block
alone. Two things to improve, logged as debt: the rejected text is not kept
(`response_json` is null on a fallback row), so the exact sentence cannot be
shown here; and a read citing the match score is a plausible legitimate cite
the header-blind grounding rejects.

### C. Grounding in the wild — every token, by hand

Method: the coach's text is copied verbatim from `coach_cache.response_json`;
every number, roster name and callout in it is looked up in that round's
`round_plays` facts (`plays_json` / `timeline_json`) as the rail narrates
them, or in the synthesis prompt's sources. Steamids resolved via `players`
(76561199011427752 = xnopyt, 76561198826400404 = NCZ RG, 76561199210928680 =
Bebita, 76561198988858765 = tttttssssss). Clocks: the coach block prints
`+N s` rounded from `(tick − freeze_end)/64`; the rail floors to `m:ss`
(R8 kill: 27.7 s → block "+28 s", rail "0:27" — same tick).

**mirage-tie R8 (CT, won, Won it, +32%; `freeze_end 44277`) — the read now in
the DB (after Regenerate):**

> "At +5 s, you setup at Shop near xnopyt. Following the early trades, you secured a critical kill on NCZ RG at +28 s with the m4a1_silencer, shifting the odds in a 2v3 situation. xnopyt closed out the round at +82 s by eliminating tttttssssss."

| Token | Kind | Where it is in R8's facts |
|---|---|---|
| +5 s | clock | setup play tick 44597 → (44597−44277)/64 = 5.0; rail "0:05 Setup at Shop" |
| Shop | callout | setup `place: Shop`; rail "Setup at Shop" |
| xnopyt | name | setup `nearest_teammate` 76561199011427752; rail "Nearest teammate xnopyt, 458 u" |
| NCZ RG | name | kill `victim` 76561198826400404; rail "Killed NCZ RG" |
| +28 s | clock | kill tick 46048 → 27.7; rail "0:27" |
| m4a1_silencer | fact | kill `weapon`; rail "1,246 u, m4a1_silencer" |
| 2v3 | number | kill `man_context: 2v3`; rail "2v3 before" |
| +82 s | clock | outcome tick 49538 → 82.2; rail "1:22 Round won" |
| tttttssssss | name | `timeline_json` tick 49538: actor xnopyt killed 76561198988858765 (ak47) — the block's "+82 s xnopyt killed tttttssssss (ak47)" |
| "early trades", "critical", "shifting the odds", "near" | judgment | not facts — free |

Per-play comments: tick **44597** (a real play) "At +5 s, you set up at Shop
with xnopyt nearby." — all tokens above. Tick **46048** "At +28 s, you
eliminated NCZ RG with the m4a1_silencer from 1,246 u, converting a 2v3 into
a 2v2." — 1,246 u = kill `killer_distance` 1246; 2v2 = the header's "2v2 at
the pivotal moment" (`round_review.header_json.man_context`; and true: after
the kill CT misosoupy3 + xnopyt v T doctorwu2021 + tttttssssss). Why it
mattered: "…swung the win probability by 32 percent…" — 32 = impact +32%.
What to practise / focus: no facts. **Every token grounded; no violation
missed.** The pre-Regenerate read ("At +5 s, you set up at Shop near xnopyt.
At +28 s, you secured a crucial kill on NCZ RG with the m4a1_silencer, adding
+32% win probability in a 2v3 situation. The round ended in a win at +82 s."
+ comment "Clean elimination on NCZ RG at +28 s to swing the round in a 2v3."
+ why "Securing the entry at +28 s broke open the 2v3 disadvantage…") was
grounded token for token too; "entry" is a judgment word, and a loose one —
the +28 s kill was the round's sixth — a voice note for the owner, not a
validator matter.

**mirage-tie R10 (CT, won, Cost you, −27%; `freeze_end 58436`):**

> "At +5 s, you spawned near Bebita at CT spawn. At +44 s, you were eliminated by NCZ RG with an ak47 at Connector in a 4v3 setup. The team recovered to secure the round win later."

| Token | Kind | Where it is in R10's facts |
|---|---|---|
| +5 s | clock | setup tick 58756 → 5.0; rail "0:05 Setup at CT spawn" |
| Bebita | name | setup `nearest_teammate` 76561199210928680; rail "Nearest teammate Bebita, 97 u" |
| CT spawn | callout | setup `place: CTSpawn`; rail "Setup at CT spawn" |
| +44 s | clock | death tick 61278 → 44.4; rail "0:44 Died to NCZ RG" |
| NCZ RG | name | death `killer` 76561198826400404 |
| ak47 | fact | death `weapon`; rail "112 u, ak47" |
| Connector | callout | death `place: Connector`; rail "At Connector" |
| 4v3 | number | death `man_context: 4v3`; rail "4v3 before" |
| "round win" | fact | header `won: true`; rail "1:19 Round won — t killed" |
| "recovered", "setup" | judgment | free |

Per-play comment at tick **61278** (the death): "Died at Connector to NCZ RG
at +44 s; hold safer crossfires rather than challenging individual duels
alone." — Connector / NCZ RG / +44 s as above; "alone" is judgment (the
facts carry nearest Bebita 500 u, which neither the rail caption nor the
block prints for this death, so nothing shown contradicts it).
`why_it_mattered` / `what_to_practise` / `focus`: null. **Every token
grounded.**

**mirage-tie synthesis opening** (grounded against the synthesis prompt:
`# Match: de_mirage · final score 12-12`, the per-round digests "Round N ·
verdict · won/lost: <read>", the template insights, the habits):

> "We finished a tight match on de_mirage with a 12-12 scoreline where spacing and trade discipline decided our rounds. In rounds like round 24, you showed great impact by trading xnopyt and planting the bomb to secure the win. However, across the match, you suffered from dying isolated 7 times and leaving 8 trade opportunities on the table. Fixing your trade timing and staying connected with teammates will turn those close losses into round wins."

| Token | Where it is |
|---|---|
| de_mirage | prompt header `# Match: de_mirage` (the raw map id — voice note below) |
| 12-12 | `final score 12-12` |
| round 24 | digest line "Round 24 · Won it · won: …" |
| trading xnopyt | R24's validated read in the digest ("traded xnopyt by eliminating tttttssssss through smoke with a glock at [tick 146418] and [tick 146446]") ← R24 `trade` play: teammate 76561199011427752 = xnopyt, `traded_by_me: true`; kill 146446 on tttttssssss thru_smoke, glock |
| planting the bomb | R24's read ("planted the bomb at [tick 148877]") ← R24 `plant` play 148877 at BombsiteA |
| 7 times | insight "Died isolated 7 times: You died isolated 7 times…" |
| 8 trade opportunities | insight "8 trades you were in range for: A teammate died inside trade range of you 8 times…" |

Work-on items: "Re-peek within two seconds of a teammate's death…" (word
numeral, not validated; it is the insight body's "The two seconds after his
death"), the other two cite nothing. **Every token grounded.** inferno-loss
spot-check: the opening's `de_inferno`, `3-13`, `round 2` are in its prompt;
R6's read ("At tick 26755, you set up at CT spawn with SirEggsAlot 159 u
away. At tick 28215, you missed your trade on SirEggsAlot against MyUnit. At
tick 28287, you died to MyUnit with a galilar headshot.") cites only the
V1.2b-hand-verified R6 facts (159 u, ticks 26755/28215/28287, galilar,
headshot, SirEggsAlot, MyUnit).

**No violation the validator let through was found; no `fix(narrator)`
commit was needed.** Voice notes for the owner (judgment, not grounding):
the coach sometimes speaks as a teammate ("We finished…our rounds"); it
echoes raw ids the prompt shows it ("de_mirage", "de_inferno") and the
`[tick N]` play labels leak into prose in some reads (id 8 R24, id 3 R6:
"At tick 26755, …"); "setup" as a verb; "entry" for a mid-round kill.

**Two deliberate probes** through the pure harness (scratchpad
`coach-check`, `cargo run --release --offline`, real mirage-tie ledger →
`render_round_block` → `Grounding::for_round` → `validate_round`, no
network; known-callout set = ledger places ∪ 23 visited places, 23 names):

| Probe | Response text | Result |
|---|---|---|
| R8 — prior-round cite | "Round 7 went the same way: you won it 2v0." | **0 violations** (the block's "Earlier this match: … Round 7 · Quiet · won" grounds the bare 7; 2v0 is in R8's outcome) |
| R10 — substring callout | "You died at T spawn to NCZ RG from 112 u." (R10's block says "CT spawn", never "T spawn"; both are known callouts) | **rejected — `read:Callout:T spawn`** (word-boundary match) |

The same run's adversarial rows still catch an invented distance (1500 /
250), a known-but-absent callout (Connector in R8, Jungle in both), a
non-play tick (46000 / 61000) and an exclamation mark.

### D. CI + branch protection

`gh run list --workflow ci --limit 6` + `gh run view <id> --json jobs`: the
`secrets` job (grep for `AIza[0-9A-Za-z_-]{30,}|AQ\.[A-Za-z0-9_-]{30,}` over
tracked files) ran **green on PR #32** (run 32853229797, 6 s) **and PR #33**
(run 32858959424, 5 s), and on both main pushes. Branch protection was not
changed here — `secrets` still has to be added to the required checks
(ADR-0005), which the controller does after the docs PR lands. Before every
commit in this session: `git status --porcelain | grep -c env.local` → 0 and
the key-shape grep over the worktree → empty.

### E. Walkthrough (`docs/design/walkthrough-v1.3/`, window 1440×900)

Graded against `docs/design/design-system.md` v2:

| Screen | §2 color | §3 type | §4 space/radius/motion | §5 dashed grammar | Verdict |
|---|---|---|---|---|---|
| `settings-nokey.png` | cards on navy; one accent primary per card ("Save key" disabled-looking only because it is disabled); status text ink | status line + model fields mono; eyebrows micro caps; hint body | `--r-md` cards, `--r-sm` inputs, 4px-grid gaps | none (nothing is evidence) | PASS |
| `settings-coach.png` | same; the toast floats (`--shadow-float`, `--r-lg`) in ink on `--bg1` — no new hue for "connected" | "on · key …1Y4A from the environment" mono; toast body sans | as above | none | PASS |
| `report-coach.png` | "Coach's read · gemini-3.5-flash-lite" eyebrow ink-dim; the read as the editorial lead with a solid left edge (furniture), template lead below it; severity edges unchanged | eyebrow micro caps mono, read italic body, work-on list body, "Regenerate" ghost button `--text-ui` | lead + list on the 4px grid | evidence chips remain the only dashes; the coach block has none (it is not evidence) | PASS |
| `rail-coach.png` (R10) | "Coach's read" block with a solid 2px ink edge, per-play comment in body ink under the mono facts, verdict chip outlined `--loss`; no new hues | read body sans, comment body sans, facts mono, "Regenerate" ghost | block padding `--s3`/`--s4`, `--r-md` card | active row solid edge (playhead at 0:00 → none); nothing dashed but evidence | PASS |

Model label note: the Report eyebrow prints the model id in the micro-caps
style ("GEMINI-3.5-FLASH-LITE") — consistent with the one label style, kept.

### F. Deviations

**Round swap, undisclosed at the time.** The dispatch (Task 11) named
mirage-tie R6 (quiet) and R13 as the pair to hand-check in Replay; §B–§C
above hand-check **R8 (Won it) and R10 (Cost you)** instead. The original
task-11 report did not record why the swap was made. The reason is supplied
here, in fix round 1, not carried over from that report: R8 (Won it) and R10
(Cost you) give verdict diversity — a won round and a cost-you round — where
the brief's R6/R13 pair (quiet and flagged) would not have. Mirage-tie R6
and R13 were not hand-checked under the coach in this session; if the owner
specifically wants those two, they can be regenerated and grounded the same
way §C does for R8/R10.

## V1.4 verification (2026-08-26)

Charter DoD: every stat cross-checked against raw SQL for one real match; every stat links to its coaching. Evidence lives in `docs/design/walkthrough-v1.4/` (README + eight screenshots + `handverify-match8.txt`); the reusable cross-check is `scripts/stats-crosscheck.sql`.

- **Match:** dev DB id 8, mirage-tie (`fixtures/own/`), tracked `misosoupy3` `76561199228328773`, re-analyzed through the app so the V1.4 engine wrote `match_stats`, `round_player_stats`, `map_callouts`.
- **SQL cross-check:** all eleven raw/stored pairs agree — rounds 24, kills 7 (8 raw, one teamkill excluded), deaths 19, assists 5, headshots 0 (match-wide HS share 41.5 %, so the field is live), damage 923 (health-capped: `dmg_health` is uncapped in CS2, so engine and script both replay each round's hurts with 100 HP per player — the raw column sums to 967), KAST rounds 14, entry attempts 2, traded deaths 2, trade kills 1, trade opportunities 11.
- **Hand-verified by an independent replay:** entries R7 (+12.4 s after freeze end) and R20 (+9.8 s), both losses; traded deaths R13 (killer died +1.6 s) and R21 (+1.4 s); clutch situations R15 1v3, R18 1v4, R23 1v4, all lost.
- **Links:** every strip chip → `/watches?stat=<key>` → the rules whose `stat_links` name it; confirmed for all seven keys.
- **Rulings recorded during execution** are in the milestone summary and ADR-0011 (stats computed in `analyze()` with the detectors' own helpers; typed tables; static catalog with a coverage test; callouts from `last_place` medians with z so labels draw on their own radar layer; 560 px label floor; damage capped at the health removed).
