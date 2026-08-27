# V1.5 walkthrough — Polish & release v1.0.0

Charter (PROMPT-V1.md §7 V1.5) as scoped by the owner on 2026-08-27: no screenshot refresh, no 30-demo library. The DoD's "owner installs the Windows build and reviews a fresh match unassisted" is the owner's step and is listed in PROGRESS.md.

## 1. Perf pass

`perf.md` in this directory: debug-only `perf: <command> <ms> ms` timings on the heavy commands, measured on the owner's real library (5 own matches + 1 corpus, 2.1 M tick samples, 190 MB) by driving the app through the accessibility API. Every budget from the spec passes — Library 2 ms (≤ 100), Report open 14 ms (≤ 300), Replay round switch 55 ms median cold (≤ 150), rail click no IPC (≤ 50), Trends 0 ms (≤ 200) — so the conditional fixes (tick index, rail memo, Trends index) were not applied. The four known O(n) shapes were fixed regardless (scoreboard names by one query; `match_stats` IN-query; per-match places cache; `RoundTicks` was already columnar).

## 2. Error / empty / loading audit

`audit.md` in this directory: screen × state × what renders. Code: `ErrorBoundary` at the route level (keyed by pathname), `errorMessage()` for §7-voice reasons, an `EmptyState` with **Retry** on every query-backed screen, and the dev-only `VITE_FAIL_IPC=<command>` switch that provokes a screen's error state. In-app checks (captures in `audit/`): the Library error state, and the loading states of Report, Trends and Replay (first round and after a round switch); the audit also found that TanStack's default three retries hid every error behind ~7 s of skeleton — fixed with `retry: false`.

## 3. README

Rewritten for v1.0.0, text only (images untouched): round-by-round coaching, the AI coach and its grounding, stats that link to their coaching, Watches, replay callouts, habits, trends, corpus; the honesty section names the three unbuilt classes and the only network call; install notes for unsigned builds and the optional key.

## 4. Release v1.0.0

- Versions bumped to 1.0.0 (`package.json`, `src-tauri/tauri.conf.json`, the `clutchfactor` app crate; workspace crates stay 0.1.0) in PR #45; tag `v1.0.0` pushed on main `0edee36` on 2026-08-27.
- `release.yml` run 33074331469: `build (macos-14)` and `build (windows-latest)` both succeeded; the GitHub release "ClutchFactor v1.0.0" carries `ClutchFactor_1.0.0_aarch64.dmg` (10.5 MB), `ClutchFactor_1.0.0_x64-setup.exe` (7.9 MB), `ClutchFactor_1.0.0_x64_en-US.msi` (10.2 MB) and `ClutchFactor_aarch64.app.tar.gz`; notes set with `gh release edit --notes-file`.
- Windows: the green `windows-latest` job producing the NSIS/MSI installers is the automated smoke; installing it and reviewing a fresh match unassisted is the owner's DoD step (PROGRESS.md → owner asks).

## 5. macOS smoke test (from the release artifact)

`ClutchFactor_1.0.0_aarch64.dmg` downloaded from the release (sha256 `3e7bef92beba8438f40f7c902bf246e9fd4c3b7a65d7161533d6364cb28c9153`), mounted with `hdiutil`, the `.app` copied to a scratch directory and its quarantine flag removed there (the local copy only — the artifact is untouched; a user sees the unsigned-app dialog and uses right-click → Open). `Info.plist` reports 1.0.0, ad-hoc signature. Launched under the owner-activity guard (Mac idle ≥ 5 min): the process came up, the window title is "ClutchFactor", and the Library rendered the owner's five matches from the existing database — `smoke-library-from-dmg.png`. The accessibility tree of the release build's WKWebView did not expose the row buttons in time, so the scripted click into a report did not fire; opening a report and playing a replay in the installed build were not exercised by the script and are part of the owner's review. The app quit cleanly on request (no process left).
