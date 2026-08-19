# PROGRESS

The resume file. A fresh session reads CLAUDE.md → this file → the active plan in `docs/plans/` and continues within minutes. Update at the end of every work chunk and before context runs long.

## Now

M3 complete (tagged `m3`). Next: M4 — Match Report (PROMPT.md §13): insight feed UI (grouping by category, severity × confidence ranking + recurrence, evidence chips → replay deep links), round timeline strip, TemplateNarrator v1 (§8 quality bar — cf-narrator crate is still empty), D4 + D5 detectors (H1 man-count + H6-info + H11 rotation ⇒ classes 8/11), **plus §5A cross-demo habit promotion** (patterns across matches — H4_REPEAT_HOTSPOT is natively cross-demo). DoD: owner reviews one of their own matches end-to-end and finds ≥1 insight they agree is real. Start with superpowers:writing-plans → `docs/plans/M4-report.md`; invoke frontend-design + dataviz for the feed/timeline UI.

## Next

1. M4 plan (above). Insights are already persisted with EvidenceRefs; the UI reads insights_for_match/death_classes_for_match (cf-store readers exist).
2. M4 narration inputs: rule `details` JSONs carry steamids as strings + distances/places — TemplateNarrator resolves names Rust-side.
3. M6 debt: Settings UI for tracked-player override; re-analyze command for old imports (analysis runs only at import today).
4. Perf: import incl. analysis ≈ well under budget (~3 s parse+analyze release); nightly §10.4 integration test still unbuilt.

## Done

- 2026-08-19: **M3 complete** — §5A rule engine: cf-analysis foundation (types/config-YAML/context/scenario-builder/classifier, priority order + class-14 pre-emption + 13-vs-15 fair-duel split); five families built by parallel subagents in worktrees, reviewed & merged (H2 trade spacing, H3 utility vulnerability, H4 Tier-1 exposure, H16 utility damage, flash+utility economy) — 99 cf-analysis tests; parser gained shots/hurts/reloads/is_scoped + targeted pre-death inventory pass; schema migration 2 (death_class/rule_flags/insights + inputs); import pipeline analyzes + persists; print_insights tool; analysis goldens w/ class-13 share as CI metric; §12 hand-verification via independent SQL cross-checks + replay spot check (see fixtures/goldens/README.md). Tag `m3`.
- 2026-08-19: **M2 complete** — replay viewer (radar assets ADR-0004, 60 fps canvas, scrubber, evidence deep links, Windows CI). Tag `m2`.
- 2026-08-19: **M1 complete** — ingest pipeline & Library (tag `m1`). **M0 complete** — skeleton + parser proof (tag `m0`).

## Decisions

- ADR-0001 demoparser2 git dep (proven). ADR-0002 sample_every=4. ADR-0003 schema v1 (+migration 2 at M3). ADR-0004 awpy radar assets.
- Rules-as-data: YAML-driven DetectorConfig thresholds/severities (predicate DSL deliberately deferred — spec principle 3 partially satisfied, revisit if rule authoring by non-devs ever matters).
- tauri-specta still not adopted (RC) — hand-mirrored TS types under MIRROR CHECKLIST.
- Subagent-driven development works well for rule families: isolated worktrees, disjoint module files, coordinator merges + reviews. Registration lines merged by coordinator, not agents.

## Gotchas

- **Death-tick inventory is always empty** (items drop at death; a living player always holds ≥ a knife) → parser samples 0.25 s pre-death; `inventory_at` skips empty samples as death artifacts.
- Steamids as JSON numbers lose precision in JS (2^53) — `EvidenceRef.focus_players` serializes as strings; rule `details` steamids are stringified by convention.
- demoparser2 skips per-tick props when events are wanted (two passes; third targeted pass for inventory via `wanted_ticks`). `active_weapon` = entity handle; the string prop is `weapon_name`. Numeric round_end reasons are winner-relative (9 = CTs eliminated).
- MM vs GOTV round-event dialects differ; normalize_rounds handles both (goldens for each).
- Identity modal fallback can pick a constant queue-mate — owner's DB has settings.tracked_steamid override; M6 Settings UI must expose it. Tracked setting must exist BEFORE import for analysis to use it (set via sqlite3 in dev).
- E2E via macOS AX: pause before reading (10 Hz updates race the tree); round chips are AX "radio buttons"; kill-feed rows are buttons; canvas content is invisible to AX — verify via SQL cross-checks + kill feed/roster text.
- Keyboard transport must be window-scoped (round chips live outside the player subtree).
- `.claude/` (agent worktrees) must stay excluded from git/eslint/vitest — vitest silently picked up worktree test copies (33→198 tests).
- CI: protoc via arduino/setup-protoc; `packageManager` pinned for pnpm/action-setup; `cargo`/`rustc` need `source "$HOME/.cargo/env"`.
