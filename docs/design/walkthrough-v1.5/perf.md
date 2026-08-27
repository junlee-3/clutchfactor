# V1.5 perf pass — measurements

Measured against `docs/spec/polish-and-release.md` §1 on the owner's real
library, from `pnpm tauri dev` (debug build) run in this worktree
(`feat/v1.5-perf`, at commit `6c7d0b4` — Task 1's instrumentation commit
`3da2936` plus Task 2's O(n) debt-fix commit `6c7d0b4`, both already on this
branch before this task started). All timings are the `perf: <command> <ms>
ms` stderr lines the debug build already prints (`timed(...)` in
`src-tauri/src/commands.rs`); flows exercised by driving the running app via
the accessibility API (System Events), never synthetic input.

## Machine + library

- Mac: Apple M4, 16 GB RAM, macOS 26.5.1 (25F80), arm64.
- DB: `~/Library/Application Support/com.clutchfactor.app/clutchfactor.db`,
  **189.8 MB** (198,979,584 bytes).
- `matches`: **6** rows (5 `kind='own'` + 1 `kind='corpus'`). The owner's five
  real demos are the whole `own` library (PROMPT-V1.md's 30-match target is
  explicitly out of scope for this pass — see spec §1 "Out of scope"; no
  DB-path override exists to load a synthetic library).
- `tick_samples`: **2,108,740** rows across all 6 matches.
- Match under test for Report/Replay flows: id 8, `de_mirage`, 12–12 (TIE),
  24 rounds — the "Mirage, TIE…" Library row named in the task.

## Results

Every number is ms from the `perf:` log; "median of 3" is noted where the
flow was repeated. Verdicts are against §1's budgets: Report open ≤ 300 ms
total; Replay round switch ≤ 150 ms; rail click ≤ 50 ms; Library ≤ 100 ms;
Trends ≤ 200 ms. Flows with no budget named in §1 (round-strip clicks, Replay
*open* as a whole) are recorded for context and marked informational.

| Flow | Command | ms (median) | Budget | Verdict | Scales with |
|---|---|---:|---|---|---|
| Library open (fresh launch) | `list_matches` | 2 | ≤ 100 ms | **PASS** | per own-match (~4 queries/match: rounds subquery + played/kills/deaths lookups); 2 ms @ 5 matches |
| Report open (match 8) | `get_match_report` | 4 | — | — | per match (insights + per-round stats for one match) |
| Report open (match 8) | `get_habits` | 10 | — | — | rule_ids × habit window (capped lookback across recent matches, not full library) |
| Report open (match 8) | `get_match_stats` | 0 | — | — | constant (PK lookup by match_id) |
| Report open (match 8) | `get_round_scoreboard` (round 1) | 0 | — | — | per match/round (fix #1 landed: one `player_names` query, not a full `MatchDetail`) |
| Report open (match 8) | `list_matches` (Report's own `useMatches()`, prev/next-match nav) | 0 | — | — | per own-match, same query as Library open |
| **Report open, total** | sum of the above | **14** | ≤ 300 ms | **PASS** | — |
| Round strip click R1 | `get_round_scoreboard` | 0 (n/a — same round, no refetch) | *(no budget in §1)* | informational | per round |
| Round strip click R8 | `get_round_scoreboard` | 0 | *(no budget in §1)* | informational | per round |
| Round strip click R20 | `get_round_scoreboard` | 0 | *(no budget in §1)* | informational | per round |
| Replay open (Watch round 8 →) | `get_match_detail` | 0 | — | — | per match |
| Replay open (Watch round 8 →) | `list_matches` | 0 | — | — | per own-match, same query as Library open |
| Replay open (Watch round 8 →) | `get_round_ticks` (round 8) | 53 (2nd call; 121 on 1st — see note) | — | — | per round: tick range scan bounded by `match_id`, not yet index-narrowed by `tick` |
| Replay open (Watch round 8 →) | `get_round_review` | 5 | — | — | per match (all rounds computed/cached together) |
| Replay open (Watch round 8 →) | `get_match_stats` | 0 | — | — | constant |
| Replay open (Watch round 8 →) | `get_map_callouts` | 0 | — | — | per map (cached after first backfill) |
| **Replay open, total** | sum, single-fetch reading | **~58** | *(no explicit budget in §1)* | informational | — |
| Replay round switch R8→R9 | `get_round_ticks` (round 9) | 69 | ≤ 150 ms | **PASS** | per round |
| Replay round switch R9→R10 | `get_round_ticks` (round 10) | 53 | ≤ 150 ms | **PASS** | per round |
| Replay round switch (extra samples R11, R12, R13) | `get_round_ticks` | 62, 55, 45 | ≤ 150 ms | **PASS** | per round |
| **Replay round switch, median of 5 fresh switches** | `get_round_ticks` | **55** | ≤ 150 ms | **PASS** | — |
| Rail moment click | (none) | no IPC | ≤ 50 ms | **PASS** | constant — render only |
| Trends open (sample 1/2/3) | `get_trends` | 0 / 0 / 0 | ≤ 200 ms | **PASS** | per tracked match in trend window (`trend_matches` + `rule_trend_counts` + `match_stats_for_matches` IN-query, fix #2 landed) |
| Trends open (sample 1/2/3) | `list_matches` | 0 / 0 / 0 | — | — | Trends also calls `useMatches()`; same per-own-match query |

Revisiting an already-fetched round (clicking back to R8, R9 or R10 after
the first visit) issues **no** `get_round_ticks` call at all — React Query's
cache serves it, 0 ms of command time. That is the common case once a match
has been scrubbed once; the "fresh switch" numbers above are the worst case
(cold cache), and they still clear the budget by a wide margin (55 ms median
against 150 ms).

**Note on the duplicate `get_round_ticks` at Replay open (121 ms then 53
ms):** `src/main.tsx` wraps the app in `React.StrictMode`, which double-
invokes mount effects in dev only; this is the only place two identical
`get_round_ticks(8)` calls appeared back to back, immediately after the
Replay screen mounted, and every subsequent round switch (R9, R10, R11, R12,
R13) fired exactly one call each. Release builds don't double-invoke, so
production Replay-open cost is the single-call figure (53 ms), not the
apparent 174 ms sum. Recorded here rather than filtered out so the artifact
is visible instead of silently smoothed over.

## What was not measured, and why

- **`get_coach_synthesis` (Report) and `get_coach_rounds` (Replay), the
  "coach cache read" the task asked to include.** Neither is wrapped in
  `timed(...)` in `src-tauri/src/commands.rs`, so no `perf:` line exists for
  either. This is intentional, not a gap: Task 1's report
  (`.superpowers/sdd/V1.5-release/task-1-report.md`) documents a controller
  ruling that `get_coach_rounds` (and the other three coach commands, which
  share the same shape) be left unwrapped because their `#[tauri::command]`
  bodies are a single line — `crate::coach::round_commentary(&state,
  match_id, &[]).await` — with no synchronous section before the `.await`
  to time; wrapping would either time across the network-bound `.await`
  (explicitly out of scope for `timed`) or measure nothing. Fixing that is a
  Task 1 instrumentation change, not one of this task's Step 4 fixes (tick
  index / rail memo / trends index), so it's out of scope here too. Read
  instead of measured: `coach_cache` (migration `0009_coach_cache.sql`) has
  `PRIMARY KEY (match_id, kind, round) WITHOUT ROWID` — the exact key
  `get_coach_cache` queries by — so a cache hit is a single indexed point
  lookup, not a scan; `crate::coach::round_commentary` additionally
  re-derives the same `match_context`/round-review/ledger bundle
  `get_round_review` builds (5 ms measured here) before checking the cache.
  Best-effort estimate from reading the code: comparable to or a little
  above `get_round_review`'s 5 ms per match on a full cache hit, nowhere
  near the 300 ms / 150 ms Report and Replay-open figures have headroom
  for. Not a confirmed number — flagged here rather than guessed into the
  table.
- **React Profiler on the rail moment click.** The click produced no `perf:`
  line and no visible stutter when driven via the accessibility API, so per
  the brief ("measure the render with `performance.now()` ... only if the UI
  feels slow") the profiler wasn't opened. Recorded as "no IPC" per the
  task's own fallback for this case.
- **A synthetic 30-match library.** Explicitly out of scope (spec §1,
  "Out of scope" and the owner-scoping note at the top of the spec) — no
  DB-path override exists, and the owner ruled out a demo-import batch. The
  "scales with" column above is the substitute: every command that isn't
  already a constant-time PK lookup is annotated with what it scales with,
  so a 30-match projection can be reasoned about instead of measured.

## Outcome

Every measured budget passes with wide margin — Report open at 14 ms against
a 300 ms budget (21×), Replay round switch at a 55 ms median against 150 ms
(2.7×), Trends at 0 ms against 200 ms, Library at 2 ms against 100 ms, and
the rail click issuing no IPC at all against its 50 ms budget. **No budget
was missed, so none of Step 4's fixes (the `tick_samples` covering index,
the `CoachRail` row memo, or a Trends index) were applied — no code change
in this pass.** The O(n) debt items Task 2 fixed regardless of measurement
(`get_round_scoreboard` reading `player_names` instead of a full
`MatchDetail`, `match_stats_for_matches`'s single `IN (…)` query, and the
per-match `PlacesCache` for `distinct_places`; commit `6c7d0b4`) are already
on this branch and are reflected in the numbers above — `get_round_
scoreboard` at 0 ms and `get_trends`'s `match_stats_for_matches`-backed stat
series at 0 ms are those fixes working, not headroom that happened to
already exist. (`get_round_ticks`'s columnar payload — `RoundTicks` as
parallel arrays, reshaped client-side by `src/replay/interp.ts`'s
`buildTracks` — predates V1.5 entirely, from the original replay read model;
it was not a Task 1/2 change, just an existing shape this pass measured and
found still within budget without the `tick_samples(match_id, tick)` index
Step 4 would have added on a miss.)
