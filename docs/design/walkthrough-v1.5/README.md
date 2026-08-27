# V1.5 walkthrough — Polish & release v1.0.0

Charter (PROMPT-V1.md §7 V1.5) as scoped by the owner on 2026-08-27: no screenshot refresh, no 30-demo library. The DoD's "owner installs the Windows build and reviews a fresh match unassisted" is the owner's step and is listed in PROGRESS.md.

## 1. Perf pass

`perf.md` in this directory: debug-only `perf: <command> <ms> ms` timings on the heavy commands, measured on the owner's real library (5 own matches + 1 corpus, 2.1 M tick samples, 190 MB) by driving the app through the accessibility API. Every budget from the spec passes — Library 2 ms (≤ 100), Report open 14 ms (≤ 300), Replay round switch 55 ms median cold (≤ 150), rail click no IPC (≤ 50), Trends 0 ms (≤ 200) — so the conditional fixes (tick index, rail memo, Trends index) were not applied. The four known O(n) shapes were fixed regardless (scoreboard names by one query; `match_stats` IN-query; per-match places cache; `RoundTicks` was already columnar).

## 2. Error / empty / loading audit

`audit.md` in this directory: screen × state × what renders. Code: `ErrorBoundary` at the route level (keyed by pathname), `errorMessage()` for §7-voice reasons, an `EmptyState` with **Retry** on every query-backed screen, and the dev-only `VITE_FAIL_IPC=<command>` switch that provokes a screen's error state. In-app checks (captures in `audit/`): the Library error state, and the loading states of Report, Trends and Replay (first round and after a round switch); the audit also found that TanStack's default three retries hid every error behind ~7 s of skeleton — fixed with `retry: false`.

## 3. README

Rewritten for v1.0.0, text only (images untouched): round-by-round coaching, the AI coach and its grounding, stats that link to their coaching, Watches, replay callouts, habits, trends, corpus; the honesty section names the three unbuilt classes and the only network call; install notes for unsigned builds and the optional key.

## 4. Release v1.0.0 and the macOS smoke test

Recorded in this file by the post-tag docs PR once `release.yml` has built the installers (the tag is pushed after this PR lands).
