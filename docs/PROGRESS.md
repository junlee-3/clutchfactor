# PROGRESS

The resume file. A fresh session reads CLAUDE.md → this file → the active plan in `docs/plans/` and continues within minutes. Update at the end of every work chunk and before context runs long.

## Now

M5 complete (tag `m5`). Next: **M6 — Trends, Settings, polish, ship v0** (PROMPT.md §13): Trends screen (per-rule trajectory across matches), Settings (tracked-player override UI, threshold view), empty/error states, app icon, README with screenshots, tagged release (Windows installer + mac app). Owner directive: ship v0 first, improvement list comes after. Start with superpowers:writing-plans → docs/plans/M6-ship.md, then superpowers:subagent-driven-development.

## Next

1. M6 plan (above). Batched owner ask outstanding: ~8 pro Mirage demos into the corpus (Corpus screen → Add pro demos) to clear the honest D6 gate; dev verification used a documented `CLUTCHFACTOR_CONFIG` override (gate 1) — shipped default stays 8.
2. M6 debt: Settings UI (tracked-player override), re-analyze command for old imports, hardcoded TICKRATE in Report.tsx, per-round kills include teamkills, habits loading-state mislabel, ad-hoc hotspot score, grid `built_at` shown as unlabeled UTC, report summary "Positioning are" pluralization, D6 severity/confidence as local consts (brief-mandated), calibration_for re-parses embedded JSON per call (cache if corpus ingestion ever calls per-sample).
3. Perf budgets (§10.4) nightly integration still unbuilt.

## Done

- 2026-08-20: **M5 complete** (tag `m5`) — reference corpus + D6 positioning. Corpus-blind analytics (matches.kind own|corpus, migration 4); baited insights name non-followers (M4 carry-in); cf-analysis corpus.rs (occupancy grids 128², world→radar via embedded map-data.json, pooled-density + nearest-rank percentile, silence gate <8 demos, recurrence ≥3, phase_moments sampling); grid cache (migration 5, LE-u32 blobs); commands import_corpus_demo/build_corpus (re-runs D6 for own matches on rebuilt maps)/corpus_status/get_grid/analyze_positioning; import_demo auto-runs D6 when grids exist; Corpus screen (gate meter one-cell-per-demo, chronology phase strip, HeatmapCanvas single-hue sqrt-alpha); D6 narrator honesty template ("unusual, not wrong"). SDD: 2 worktree implementers + reviews + 1 fix round (heatmap stale-image race) + tickrate follow-up; ledger .superpowers/sdd/M5-corpus/. E2E: pro mirage demo imported via UI dialog → 8 grids (115 samples = 23 rounds × 5 alive ✓) → 4 D6 insights matching an independent SQL/python recomputation exactly (§12) → report cards with honesty wording → evidence chip → replay round 5; corpus-blindness verified (own list + rule counts byte-identical pre/post).
- 2026-08-20: **M4 complete** (tag `m4`) — Match Report screen, TemplateNarrator v1, D4/D5, cross-demo habits + hotspots. E2E via AX; ledger .superpowers/sdd/M4-report/.
- 2026-08-19: **M3 complete** (tag `m3`) — §5A rule engine, classes 1–7/9/13–15, hand-verified. **M2** (`m2`) replay viewer · **M1** (`m1`) ingest + Library · **M0** (`m0`) skeleton + parser proof.

## Decisions

- ADR-0001 demoparser2 git dep · ADR-0002 16 Hz sampling · ADR-0003 schema v1 (+m2 analysis, +m3 flag evidence, +m4 kind, +m5 corpus_grids) · ADR-0004 awpy radar assets.
- D6: severity 0.5 / confidence 0.6 fixed (unusualness cap); d6_insights takes the match tickrate (no TICKRATE consts); build_corpus re-runs positioning for own matches (fresh grids invalidate old D6); `CLUTCHFACTOR_CONFIG` env = explicit dev threshold override, shipped defaults untouched.
- Narrator: deterministic template variants via content hash; ClaudeNarrator seam intact (§8).
- Habit scoring: severity × confidence × (matches_hit/window) × ln(1+total); baited never promoted alone; one hotspot card per map.
- SDD process notes: worktree-isolated parallel implementers + coordinator cherry-pick works well; reviewers scoped to diff packages; models sonnet(implement+review)/fable(final review).

## Gotchas

- Corpus rows live in `matches` with kind='corpus' — every tracked-player query filters kind='own' (store tests enforce).
- Death-tick inventory always empty (pre-death sampling); steamids as strings at JS boundaries; demoparser two-pass + targeted inventory pass; numeric round reasons winner-relative; MM vs GOTV dialects.
- AX E2E: pause before reading; **a stale dev binary can serve clicks after an edit — kill + relaunch `tauri dev` before trusting E2E results**; round chips are radio buttons; canvas invisible to AX; `.claude/` excluded from git/eslint/vitest.
- Bash tool cwd persists — a `cd` into a worktree leaks into later commands; use absolute paths or cd back.
- CI: protoc via arduino/setup-protoc; pnpm pinned via packageManager; `source "$HOME/.cargo/env"`.
