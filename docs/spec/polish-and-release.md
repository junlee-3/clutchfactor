# Polish & release v1.0.0 — V1.5 spec

> Charter: PROMPT-V1.md §7 "V1.5 — Polish & release v1.0.0. Perf pass (report +
> replay + rail with a 30-match library), error/empty audit, README + screenshots
> refresh, tagged release, both installers smoke-tested. DoD: the owner installs
> the Windows build and reviews a fresh match using RBR + AI coaching unassisted."
> Owner scoping (2026-08-27): **no screenshot refresh, no 30-demo library.** The
> perf pass runs on the real library (5 own matches + corpus) with measured
> numbers; the README refresh is text only; the Windows install review stays the
> owner's step of the DoD.

## 1. Perf pass — measure, then fix what is measured

**Instrumentation (ships, debug builds only).** Every heavy command is wrapped
in `timed("get_match_report", || …)` which prints `perf: get_match_report 123 ms`
to stderr in debug builds and is a no-op in release. Heavy = `list_matches`,
`get_match_report`, `get_habits`, `get_trends`, `get_round_ticks`,
`get_round_review`, `get_coach_rounds`, `get_match_stats`, `get_round_scoreboard`,
`get_map_callouts`, `get_match_detail`. The frontend's single `invoke` wrapper in
`src/lib/ipc.ts` records `performance.measure("ipc:<cmd>")` in dev builds so the
DevTools Performance tab shows IPC time per command; nothing in production.

**Budgets** (on this Mac, the owner's real library): Report open ≤ 300 ms of
command time end to end; Replay round switch ≤ 150 ms from click to first frame
of the new round; rail scrub (moment click) ≤ 50 ms; Library open ≤ 100 ms;
Trends open ≤ 200 ms. Each budget is measured before and after and the numbers
go in the walkthrough.

**Fixes in scope** (each done only if the measurement shows it matters, except
the four debt items, which are done regardless because they are the known
O(n) shapes):
1. `get_round_scoreboard` fetches player names with one light query instead of
   the whole `MatchDetail` per click.
2. `match_stats_for_matches` is a single `WHERE match_id IN (…)` query.
3. `distinct_places` (coach known-callouts) is memoised per match in `AppState`
   (invalidated on re-analyze / delete).
4. `get_round_ticks` payload: if a round switch exceeds its budget, the command
   returns columnar arrays (`steamids`, `ticks`, `x`, `y`, `z`, … one array each)
   and `useRoundTicks` reshapes them into the existing row model client-side, so
   the renderer and coordinate code are untouched; a test proves the reshape is
   lossless.
5. Rail: if a moment click re-renders every play row, memoise the row component;
   measured with React Profiler in dev.
6. Trends: if `rule_trend_counts`/ledger scans exceed budget, add the missing
   index or precomputed column — decided by the measurement, recorded in the
   walkthrough.

**Out of scope:** a synthetic 30-match library (no DB-path override exists and
the owner ruled the demo import out); the numbers are reported for the real
library with a note on which paths scale linearly with matches.

## 2. Error / empty / loading audit — every screen, three states

Inventory today: `Replay` and `Watches` handle `isError`; `Corpus`, `Library`,
`Report`, `Settings`, `Trends` do not (a failed query renders nothing or a stale
skeleton); no `ErrorBoundary` exists (a render exception blanks the window).

**Contract per screen** (Library, Report, Replay, Trends, Watches, Corpus,
Settings): loading = skeleton at final size; empty = `EmptyState` with the one
next action; error = `EmptyState` in the §7 voice ("Couldn't load the report —
<reason>") with a **Retry** action that refetches; mutations keep the toast
pattern. A route-level `ErrorBoundary` (class component, `src/components/ui/
ErrorBoundary.tsx`) wraps the routed screen with a "Something in this screen
broke — Reload the screen / Back to Library" fallback; it never swallows the
error (logged to console). The audit is a table in the walkthrough: screen ×
state × what renders, each verified in the running app (errors provoked by
pointing the command at a missing id, e.g. `/report/999`, `/replay/999`, and by
a forced rejection in dev).

## 3. README refresh — text only

Rewrite `README.md` for v1.0.0 without touching the existing images: what it
does now (round-by-round coaching from the play ledger, the AI coach and how it
is grounded, stats that link to their coaching, Watches, callouts, habits,
trends, the reference corpus), the honesty section (what the engine cannot see,
silence bias, no key ever leaves the machine except to Google's API when the
owner sets one), install for v1.0.0 (unsigned builds, first-run steps, where
the Gemini key goes and that it is optional), development (unchanged commands,
pointer to CLAUDE.md). No exclamation marks.

## 4. Release v1.0.0

- Versions: `package.json`, `src-tauri/tauri.conf.json`, `src-tauri/Cargo.toml`
  → `1.0.0` (workspace crates stay `0.1.0`; they are not published).
- `git tag v1.0.0` on main after the PRs land → `release.yml` builds the macOS
  `.dmg` and the Windows NSIS/MSI and attaches them to the GitHub release.
- Release notes: `gh release edit v1.0.0 --notes-file` with the v1 highlights
  (RBR coaching, AI coach, stats & Watches, replay callouts, corpus, honesty).
- Smoke test: **macOS** — download the release `.dmg` with `gh release download`,
  mount it, launch the bundled `.app` from the mount, confirm the window opens
  on the Library with the owner's matches and that a replay plays; **Windows** —
  the CI job producing the installer is the automated smoke; installing and
  reviewing a match unassisted is the owner's DoD step and is listed as such.

## 5. Milestones / PRs

- **PR A — perf** (`feat/v1.5-perf`): instrumentation, the four debt fixes,
  measured fixes 4–6 if warranted, before/after numbers in the report.
- **PR B — audit + README** (`feat/v1.5-audit`): ErrorBoundary, per-screen
  error/empty/loading states, README rewrite.
- **PR C — release** (`docs/v1.5-release`): version bump, PROGRESS/CLAUDE
  updates, walkthrough (`docs/design/walkthrough-v1.5/`: perf table, audit
  table, smoke-test evidence), then tag `v1.0.0` and edit the release notes.

## 6. Definition of done

Perf budgets met or the miss documented with the reason; every screen's three
states verified in the app; README reads true for v1.0.0; `v1.0.0` release
exists with both installers attached; macOS installer launched from the DMG;
PROGRESS.md says what remains for the owner (Windows install review, coach
model switch-back, `secrets` as a required check).
