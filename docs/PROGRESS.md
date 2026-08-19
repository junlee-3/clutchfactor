# PROGRESS

The resume file. A fresh session reads CLAUDE.md → this file → the active plan in `docs/plans/` and continues within minutes. Update at the end of every work chunk and before context runs long.

## Now

M4 complete (tag `m4` pending owner DoD sign-off — the milestone's DoD is literally "the owner reviews one of their own matches and agrees ≥1 insight is real and actionable"; the ask is out). Next: M5 — Reference corpus & D6 (PROMPT.md §13): Corpus screen + ingestion, occupancy grids per map/side/phase, D6 with honesty rules + minimum-corpus gate (default 8 demos/map), heatmap rendering (invoke dataviz first). Start with superpowers:writing-plans → docs/plans/M5-corpus.md.

## Next

1. M5 plan. CARRY-INS flagged at M4 final review: corpus demos will land in the same `matches` table — habit windows/death_positions must exclude non-tracked-player matches (filter on players-contains-tracked or a match kind column); H2 insight should carry the non-following teammate so baited captions can name them (ticketed M5).
2. M6 debt: Settings UI (tracked-player override), re-analyze command for old imports, deferred minors list in `.superpowers/sdd/M4-report/progress.md`-style records (hardcoded TICKRATE in Report.tsx, per-round kills include teamkills, habits loading-state mislabel, ad-hoc hotspot score).
3. Perf budgets (§10.4) nightly integration still unbuilt.

## Done

- 2026-08-20: **M4 complete** — Match Report screen (narrated insight feed grouped/ranked, death-class breakdown w/ honesty note, round strip → replay, evidence chips → replay deep links); cf-narrator TemplateNarrator v1 (§8 voice, deterministic variants, 46 exact-string tests); D4 entry-structure + D5 timing families (class 11 live); cross-demo habit promotion + H4_REPEAT_HOTSPOT clustering (store migration 3: flag evidence). Executed via superpowers:subagent-driven-development (3 worktree implementers + per-task reviews + 1 narrator fix round + final whole-branch review + 1 fix wave, all clean; ledger at .superpowers/sdd/M4-report/). E2E: 5 UI imports, report verified via AX (real coaching text incl. "left trades on the table in 5 of your last 10 matches — 35 times"), chip → replay at exact evidence tick.
- 2026-08-19: **M3 complete** (tag `m3`) — §5A rule engine, taxonomy classes 1–7/9/13–15, hand-verified. **M2 complete** (`m2`) — replay viewer. **M1 complete** (`m1`) — ingest + Library. **M0 complete** (`m0`) — skeleton + parser proof.

## Decisions

- ADR-0001 demoparser2 git dep · ADR-0002 16 Hz sampling · ADR-0003 schema v1 (+m2 analysis, +m3 flag evidence) · ADR-0004 awpy radar assets.
- Narrator: deterministic template variants via content hash (no RNG); ClaudeNarrator seam intact (§8).
- Habit scoring: severity × confidence × (matches_hit/window) × ln(1+total); baited never promoted alone; one hotspot card per map.
- SDD process notes: worktree-isolated parallel implementers + coordinator merge works well; reviewers scoped to diff packages; models sonnet(families)/opus(taste)/fable(final review).

## Gotchas

- **Corpus-demo dilution risk (M5 blocker-aware):** habit windows + death_positions query ALL matches; corpus imports must be excluded from tracked-player analytics.
- Death-tick inventory always empty (pre-death sampling); steamids as strings at JS boundaries (EvidenceRef serializer); demoparser two-pass (events gate ticks) + targeted inventory pass; numeric round reasons winner-relative; MM vs GOTV dialects.
- AX E2E: pause before reading; kill-feed rows/round chips are buttons/radio buttons; canvas invisible to AX; `.claude/` excluded from git/eslint/vitest.
- Rust changes under tauri dev auto-rebuild+relaunch the app; frontend hot-reloads. Port 1420 orphans: `lsof -ti :1420 | xargs kill`.
- CI: protoc via arduino/setup-protoc; pnpm pinned via packageManager; `source "$HOME/.cargo/env"`.
