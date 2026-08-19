# PROGRESS

The resume file. A fresh session reads CLAUDE.md → this file → the active plan in `docs/plans/` and continues within minutes. Update at the end of every work chunk and before context runs long.

## Now

M3 in flight (plan: docs/plans/M3-detectors.md). DONE: Tasks 0–2 (parser rule data incl. targeted inventory pass; schema migration 2 + save_analysis/readers; cf-analysis foundation — types/config/context/scenario/classifier, 15 tests) + Task 8 partials (import pipeline runs analyze, print_insights tool). IN FLIGHT: Tasks 3–7 dispatched to five parallel subagents in worktrees under `.claude/worktrees/agent-*` (families h2, h3, h16, h4, flash_util — each creates `cf-analysis/src/families/<x>.rs` + registers in mod.rs in its own worktree). MERGE PROCEDURE when they report: copy each family file into main tree, hand-merge the five mod.rs registrations, run full test suite + clippy, code-review each family against its plan task + spec §2, then Task 9 (analysis goldens for mirage-tie+navi with class-13 share, wipe dev DB, UI re-import, hand-verify ≥3 flags/family via evidence deep links in the replay viewer, docs, tag m3).

## Next

1. Merge + review family subagent outputs (above).
2. Task 9 verification (see plan).
3. M6 debt: Settings UI for tracked-player override.
4. Perf budget (§10.4) nightly integration still unbuilt; import now includes an analyze stage — timing worth a look at Task 9.

## Done

- 2026-08-19: **M2 complete** — awpy radar assets vendored (ADR-0004, build 17595823, served via vite publicDir=assets); MatchDetail/RoundTicks read models + commands; replay math TDD (coords/interp/utility/timeline, 24 tests); canvas replay viewer (rAF, 16 Hz interpolation, utility lifetimes w/ smoke_expired pairing, bomb state, death markers, focus dimming, kill feed, roster HP, scrubber + pips, keyboard transport, evidence deep links); Windows CI build job green. E2E on real demos via AX scripting: mirage round played at steady 60 fps, kill-feed jump verified against DB ground truth (NCZ RG 0:23 / nekoo鸭 0:25 first deaths, live HP 91/87/86 mid-fight), round switch to 12/24, de_nuke (lower-layer map) loads & plays. Tag `m2`.
- 2026-08-19: **M1 complete** — MatchData model + round normalization (7 unit tests, MM+GOTV encodings); two-pass extraction (events + ticks) with side assignment, roster-following score derivation, goldens for mirage-tie (12–12) and navi (13–10); ADR-0002 sample_every=4; cf-store SQLite schema v1 + migrations (ADR-0003, 6 tests); import_demo with Channel progress + sha256 dedup; Library screen (TanStack Query, 7 vitest tests). E2E through the real UI via accessibility scripting: 3 own demos imported by clicking Import → native dialog, live progress, correct rows (Dust2 L 4–13 6/17, Inferno W 13–7 16/14, Mirage T 12–12 7/19), duplicate re-import rejected with visible error, relaunch persistence confirmed, owner identity set via settings override. Tag `m1`.
- 2026-08-19: Owner demos verified (5/5 parse, misosoupy3 in every roster); spec addendum integrated (docs/spec/death-taxonomy.md, PROMPT §5A).
- 2026-08-19: **M0 complete** — scaffold, workspace, CI, demoparser2 proven (tag `m0`).

## Decisions

- ADR-0001: demoparser2 pinned git dep — proven. ADR-0002: tick sampling every 4th tick (16 Hz). ADR-0003: SQLite schema v1 (steamids as TEXT everywhere incl. IPC; death_class/rule tables deferred to migration 2 at M3).
- tauri-specta still RC → TS types hand-mirrored in `src/lib/ipc.ts` under MIRROR CHECKLIST.

## Gotchas

- react-hooks v7 lint forbids ref writes during render and setState-in-effect: replay playback uses key-based remount per round + a `getScene()` closure read inside rAF; images via useMemo'd `Image` polled with `img.complete` (no state).
- AX-tree reads race the 10 Hz React panel updates while playing — pause first (Space keystroke), then read; wrap element reads in `try`.
- Round chips (role=tab buttons) expose as AX "radio button"; kill-feed rows are buttons (invisible to static-text dumps); react-router `<Link>` clicks via AX are unreliable — relaunch to navigate back in E2E scripts.
- `weapon_name` values are display names ("USP-S", "Bayonet"), not `weapon_*` codes.

- **demoparser2 skips per-tick prop collection when `wanted_events` is non-empty** (collect_entities gate) → cf-parser does two passes per demo, like upstream's Python bindings. Two passes ≈ 1 s release on a 250 MB demo.
- `active_weapon` prop = raw entity handle (U32); the weapon-name string prop is **`weapon_name`**. Small int props arrive as I32 *or* U32 per netvar — `int_col` handles both.
- **Numeric round_end reason codes are winner-relative** (9 = Terrorists_Win ⇒ CTs eliminated); MM string reasons name the eliminated side ("t_killed" = CT win). Decoder maps both to eliminated-side enums; cross-checked vs #SFUI messages.
- MM vs GOTV round events differ (String vs I32 winner; GOTV round 1 may lack round_start; MM duplicates round_officially_ended ×2). normalize_rounds handles all; don't touch without running both goldens.
- **Identity modal fallback can pick a constant queue-mate** (it did: xnopyt appears in all 3 imported demos and has a lower steamid). Owner's DB has `settings.tracked_steamid = 76561199228328773`; M6 Settings UI must expose this. Kill counts exclude self-kills (`victim != attacker`) — self/fall deaths show as attacker=self, weapon "world" (class-14 material at M3).
- E2E technique: the WKWebView exposes DOM via macOS accessibility — `osascript` System Events can click app buttons and read rendered text; native open dialog driven with Cmd+Shift+G + typed path. Screen capture is permission-blocked; AX text dump is the reliable check.
- Port 1420 leftovers: killing tauri dev can orphan vite — `lsof -ti :1420 | xargs kill` before relaunching.
- demoparser's `csgoproto` needs `protoc` (CI: arduino/setup-protoc@v3; local via Homebrew).
- `cargo`/`rustc` need `source "$HOME/.cargo/env"` in fresh shells. Shell cwd drifts between tool calls — use absolute paths.
