# ClutchFactor — Build Prompt

You are Claude Fable 5, acting as the sole senior engineer building **ClutchFactor**: a desktop coaching application for Counter-Strike 2 players. This document is the approved product spec and engineering charter. It is the output of a completed brainstorming/design session with the product owner — treat every decision marked **DECIDED** as settled. Do not re-litigate them; do not re-run brainstorming on the overall product. Your first act is to read this file end to end, then follow **§14 First Actions**.

---

## 1. Mission

CS2 players who want to improve today have stat trackers (Leetify, csstats.gg) that tell them *what* happened — K/D, ADR, HLTV rating. ClutchFactor tells them *why* they are losing rounds and *what to change*. It ingests a demo file (`.dem` — the full match recording), reconstructs the match tick by tick, and produces **coaching insights backed by replayable evidence**, e.g.:

- "You died isolated 7 times this match — no teammate close enough to trade you. Here are the deaths." *(click → 2D replay jumps to that moment)*
- "4 of your 11 flashes blinded a teammate longer than any enemy."
- "On Mirage A-site holds, you set up in positions reference players almost never use at this phase of the round."

The quality bar for every insight: **would a human coach say this, and can the player click through to see the proof?** A stat with no evidence and no action attached is not an insight and does not ship.

The product owner is the primary user: a competitive CS2 player, developing on an Apple Silicon Mac, playing on Windows. They will use the app on real demos from their own matches.

## 2. Ground Rules (non-negotiable, apply to every session)

1. **Commit and push constantly.** Small, coherent commits with clear messages; push to `origin main` after every completed unit of work. Never leave more than ~an hour of work uncommitted. Any session may be interrupted at any time — the repo plus its docs must always be sufficient for a fresh session to resume with zero conversation history. (Git identity and remote are already configured; do not rewrite pushed history.)
2. **Maintain the context-survival docs** exactly as specified in §11. `CLAUDE.md`, `docs/PROGRESS.md`, and `docs/adr/` are load-bearing infrastructure, not paperwork.
3. **Verify, don't guess, on external APIs.** demoparser2's exact Rust API, Tauri 2 command/event APIs, current crate versions — look them up (web search, docs.rs, the library's GitHub) before writing code against them. Wrong guessed field names in a parser integration waste entire sessions. Pin versions in lockfiles.
4. **Real data only.** Every parser and detector feature is developed and verified against real `.dem` files (§10.1). No hardcoded fake match data in the UI, no placeholder insights, no "TODO: implement" stubs left behind in committed code.
5. **The evidence contract is sacred.** Every `Insight` must reference concrete ticks/positions/players that the replay viewer can jump to. If a detector can't produce evidence, redesign the detector.
6. **Autonomy policy.** Make technical decisions yourself and record them as ADRs. Ask the product owner only when you need something from them (a demo file, a product-taste call, an account/asset decision) — and batch such questions. Never block on questions you can answer with research.

## 3. Tech Stack — DECIDED

Evaluated independently; the owner's original C# WPF/MAUI idea was rejected because development happens on macOS (WPF cannot build or run there; MAUI's Windows target cannot be built from a Mac). The Python+Rust alternative was rejected for packaging pain (PyInstaller) and sidecar IPC complexity.

| Layer | Choice | Notes |
|---|---|---|
| Shell | **Tauri 2** | Small binary, cross-platform: develop on macOS, ship to Windows. |
| Core / backend | **Rust** | All parsing, analysis, storage. |
| Demo parsing | **demoparser2** (LaihoE/demoparser, Rust core) | The fastest, most battle-tested CS2 parser. Its Rust core may not be published on crates.io — a git dependency on the repo is acceptable; verify current state before scaffolding. |
| Frontend | **React + TypeScript + Vite** | UI, replay viewer, charts. |
| Replay rendering | **Canvas 2D** (WebGL only if profiling demands it) | Player dots, utility, kill markers on radar images. |
| Storage | **SQLite** via `rusqlite` (bundled) | Single file DB in app data dir. Migrations from day one (embedded, versioned). |
| State/data (UI) | TanStack Query (wrapping Tauri `invoke`) + Zustand or equivalent | Keep it boring. |
| Charts | Lightweight (e.g. visx/recharts/uPlot) — pick once, record ADR | Used for trends, timelines. |
| CI | GitHub Actions | See §10.3. |

**Risk R1 (document in ADR-0001, monitor at M0):** if demoparser2's Rust core proves unusable as a library (unstable git API, missing events), the fallback is a **C# sidecar using demofile-net** exposed to Tauri as a subprocess emitting JSON/MessagePack. Prove demoparser2 viable in M0 before building anything on top of it.

**Ship targets:** Windows installer (primary, via CI), macOS app (dev/dogfood). Linux: not a goal, don't break it gratuitously.

## 4. Architecture

Monorepo layout:

```
clutchfactor/
├── PROMPT.md                  # this file — the spec
├── CLAUDE.md                  # short, always-current session context (§11)
├── docs/
│   ├── PROGRESS.md            # living state: now / next / done / decisions / gotchas
│   ├── adr/                   # ADR-NNNN-*.md, one per significant decision
│   └── plans/                 # implementation plans (superpowers:writing-plans output)
├── src-tauri/                 # Rust workspace
│   ├── crates/
│   │   ├── cf-parser/         # demoparser2 wrapper → normalized MatchData
│   │   ├── cf-analysis/       # detectors: MatchData → Vec<Insight>
│   │   ├── cf-store/          # SQLite persistence, migrations, queries
│   │   └── cf-narrator/       # CoachingNarrator trait + TemplateNarrator
│   └── src/                   # Tauri app: commands, events, wiring
├── src/                       # React frontend
│   ├── screens/               # Library, MatchReport, Replay, Trends, Corpus, Settings
│   ├── replay/                # canvas renderer, timeline scrubber, coord math
│   └── components/
├── assets/maps/               # radar images + per-map calibration data (§6.3)
└── fixtures/                  # real .dem files, gitignored; README explains how to add
```

**Data flow:** UI issues `import_demo(path)` → Tauri command spawns async task → `cf-parser` streams the demo into a normalized `MatchData` (players, rounds, kill/blind/grenade/bomb events, position samples) → `cf-analysis` runs every registered detector → `cf-store` persists match summary, per-round data, downsampled position tracks, and insights → progress events stream to the UI throughout → UI reads via query commands.

**Design rules:**
- `cf-parser`'s output types are the **only** interface `cf-analysis` sees — no demoparser2 types leak past `cf-parser`. This is what makes risk R1's fallback survivable.
- Each detector is one module implementing a common `Detector` trait: `fn detect(&self, match_data: &MatchData, cfg: &DetectorConfig) -> Vec<Insight>`. Detectors are pure functions over `MatchData` — no I/O — so they are trivially unit-testable with synthetic scenario builders.
- `Insight` (shape to refine): `{ id, detector, category, severity, round, player, title_data, evidence: Vec<EvidenceRef>, metrics: serde_json::Value }` where `EvidenceRef = { round, tick_start, tick_end, focus_players, camera_hint }`. The replay viewer consumes `EvidenceRef` directly — this is the evidence contract.
- All tunable thresholds (§6.4) live in one `DetectorConfig` with documented defaults; never scatter magic numbers.
- Position tracks are downsampled for storage/replay (e.g. every 4th tick ≈ 16 Hz — decide via ADR after measuring); events keep exact ticks.
- IPC: typed commands, snake_case names, one `Progress { stage, pct, detail }` event channel for long operations. Generate or mirror TS types for every command payload (e.g. via specta/tauri-specta if current — verify; hand-mirrored types with a checklist are an acceptable fallback).

## 5. Feature Spec — Detectors (the product core)

Build in this order within M3–M5. Every detector: definition, defaults (see §6.4), evidence, and the coaching angle its template text must express.

**D1 — Untraded/isolated deaths.** For each death of the tracked player: distance and named place of nearest living teammate at time of death; whether any teammate had a realistic trade opportunity (alive, within trade-distance or same/adjacent place, within trade window); whether the death was actually traded (killer killed within window). Classify: `traded / tradeable-untraded / isolated`. Coaching angle: isolated deaths are positioning errors before they are aim errors.

**D2 — Flash effectiveness.** From `player_blind` + `flashbang_detonate`: per flash — enemies effectively blinded (duration ≥ threshold), teammates blinded, self-blind, whether a blinded enemy died within the follow-up window (flash converted), flashes thrown with no teammate or self positioned to play off them. Match-level: effective-flash rate, team-flash count with worst offenders replayable. `player_death.assistedflash` cross-checks conversions.

**D3 — Utility usage & waste.** HE/molotov damage per throw; smokes thrown after the round is functionally over vs. in execute windows; utility still in inventory on death ("died with $1900 of nades"); molly/HE damage vs. teammates. Coaching angle: utility hoarding and dead-time throws.

**D4 — Trade discipline & entry structure (team-context for the tracked player).** Entry attempts (first engagement of the round on T side): who took them, was the entry supported (teammate within trade parameters), success rates. For the tracked player: how often they were the unsupported entry vs. the non-trading teammate.

**D5 — Timing & rotation.** Site presence at bomb plant; time-to-rotate after plant/first-contact info; early aggressive deaths (died in first N seconds beyond mid-map depth without team support). Uses named places (`last_place_name`) heavily.

**D6 — Positional baseline comparison (M5).** The **reference corpus**: user drops pro/high-level demos (freely downloadable from HLTV match pages, FACEIT, etc. — the app never scrapes; it ingests local files) into a corpus library. Per map/side/round-phase (freeze-end, early, mid, post-plant) build positional occupancy grids from corpus players. For the tracked player at sampled key moments (e.g. at first contact, at plant), report where they stood vs. corpus density for the same phase/context, surfacing consistent low-density (unusual) positioning. Be statistically honest: this measures *unusualness*, not *wrongness* — template text must say "reference players rarely hold this position here; common alternatives are X/Y (click to view heatmap)". Corpus must be ≥ N demos on that map before the detector activates (default 8, configurable), otherwise it stays silent.

**Cross-cutting: recurrence.** Single events are noise; patterns are coaching. The insight feed ranks by (severity × recurrence within match), and the Trends screen (M6) tracks detector metrics across matches ("isolated deaths trending down 4 matches straight").

**Deliberate non-detectors for v1:** aim/crosshair mechanics (duels are visible in the replay; scoring micro-aim is a different product), economy strategy beyond D3's waste angle, and anything requiring visibility raycasts against map geometry (smoke LOS-blocking quality, precise crossfire geometry). Approximate with distance + named places + event data; record a `docs/adr/` note on what a geometry pass would add (awpy's nav-mesh/geometry data is prior art) for v2.

**§5A — Addendum (owner, 2026-08-19): death taxonomy, rule families, cross-demo habits.** `docs/spec/death-taxonomy.md` is part of the product spec and governs death/duel analysis in detail: a 15-class death taxonomy (priority-ordered, one primary class per death of the tracked player + secondary tags), rule families H1–H16 (rule ids are load-bearing; rules are data/YAML, every rule emits `confidence`, silence-biased, thresholds in seconds/world-units never ticks, one hand-verified golden clip per rule in CI), and **cross-demo habit tracking as a core requirement** — patterns promoted across matches ("you repeat X in N of your last M games, here's the fix") with evidence links into each contributing demo. D1–D5 remain the milestone deliverables and map onto the families (mapping table in that doc). Insight ranking becomes severity × confidence × recurrence-across-demos. Milestone impact: M1 schema carries `death_class` + rule flags + cross-demo keys; M3 = taxonomy MVP (geometry-free classes) via the rule engine; M4 adds cross-demo pattern promotion to the report; M6 Trends renders class shares and habit trends. Tier-3 geometry (raycasts) stays a non-goal for v1 per above.

## 6. CS2 Domain Cheat-Sheet

Verify anything load-bearing against the parser's actual output on a real demo; this section saves you research hours but is not a substitute for checking.

### 6.1 Demos
- `.dem` files: recorded by GOTV or the client. Matchmaking demos download in-game (Watch → Your Matches); files land under `.../Counter-Strike Global Offensive/game/csgo/replays/`. FACEIT/HLTV provide demo downloads per match. Typical size 50–400 MB compressed; matches are MR12 (first to 13, max 24 regulation rounds + OT).
- CS2 uses subtick input on a 64-tick simulation; demos carry ~64 ticks/sec. A full match ≈ 150k–250k ticks.

### 6.2 Events & properties (demoparser2 vocabulary)
- Key events: `player_death` (attacker, assister, `assistedflash`, headshot, weapon), `player_blind` (userid, attacker, `blind_duration`), `flashbang_detonate`, `smokegrenade_detonate`, `smokegrenade_expired`, `hegrenade_detonate`, `inferno_startburn`/`inferno_expire`, `weapon_fire`, `bomb_planted`/`bomb_defused`/`bomb_exploded`, `round_start`, `round_freeze_end`, `round_end`, `round_officially_ended`.
- Key per-tick player props: position (`X`, `Y`, `Z`), yaw/pitch, `health`, `armor`, is_alive, `last_place_name` (named map area like "BombsiteA", "TSpawn" — free from the demo, no nav-mesh needed), active weapon, money, team.
- Round boundaries in CS2 demos have known quirks (warmup rounds, knife rounds in scrim demos, `round_officially_ended` vs `round_end`). Normalize to a clean `rounds[]` early in `cf-parser` and golden-test it — round misalignment silently corrupts every detector downstream.

### 6.3 Radar coordinate mapping
Per-map calibration constants (`pos_x`, `pos_y`, `scale`) come from the game's overview data; the awpy project (MIT) maintains machine-readable map data — vendor with attribution. For a 1024×1024 radar image:
`img_x = (world_x - pos_x) / scale` ; `img_y = (pos_y - world_y) / scale`.
Maps with verticality (Nuke, Vertigo, Train) need a Z threshold to switch upper/lower radar layers — awpy's data includes these. Radar images: research current best source at M2 (community radar packs such as SimpleRadar with credit, awpy's images, or extraction from local game VPKs); keep the asset source swappable and record the licensing decision in an ADR.

### 6.4 Default thresholds (all in `DetectorConfig`, all documented as tunable approximations)
- Trade window: 3.0 s; trade distance: teammate within ~700 units or same/adjacent named place.
- Isolation: no living teammate within ~900 units at death and none tradeable.
- Effective flash: `blind_duration ≥ 1.1 s` (community-standard cutoff — below that a player recovers before it matters); team-flash flagged at the same bar; flash-conversion window: 2.0 s.
- Early aggressive death: within 20 s of `round_freeze_end`.
- Positional grid: 128×128 cells per map for corpus density.

## 7. UI Spec

Screens (React Router or Tauri multiwindow — keep it single-window, routed):

1. **Library** — imported matches: map, date, score, player's headline stats, insight-severity summary. Import via file picker + drag-drop; parse progress inline.
2. **Match Report** — the money screen. Insight feed grouped by category (Deaths, Utility, Positioning, Timing), ranked by severity × recurrence, each card: template narration, key metrics, evidence chips → click jumps into the Replay viewer at that moment. Scoreboard and round timeline strip along the top (round results, player's K/D/impact per round, clickable).
3. **Replay Viewer** — 2D radar playback: radar image, colored player dots with view-direction wedges and name labels, death markers, active utility (smoke circles with lifetime, molly areas, flash pops), bomb state, HP bars in a side roster, kill feed. Timeline scrubber with round boundaries and event pips; play/pause, 1×/2×/4×; jump-to-evidence deep links (`round`, `tick`, focus players highlighted, others dimmed). 60 fps target.
4. **Trends** — detector metrics across the player's imported matches: line/spark charts, per-map splits, streak callouts.
5. **Reference Corpus** — manage the pro-demo library: add demos, per-map counts, corpus build status, heatmap previews per map/side/phase.
6. **Settings** — player identity (SteamID(s) to track, auto-detected from demos where possible), detector threshold overrides, data location, (later) narrator/API-key section.

**Design direction:** dark, information-dense but calm — closer to a professional analytics tool (Linear/Grafana energy) than an esports-hype site. Team colors CT #4aa3ff-ish / T #f5b83d-ish as the only loud hues; severity encoded by a restrained scale, not rainbow alerts. Radar view is the hero; chrome recedes. **At the start of each UI milestone, check the available-skills listing and invoke the relevant design skills** — expected available: `frontend-design:frontend-design` (aesthetic direction) and `dataviz` (before ANY chart/heatmap/timeline work — it is mandatory before writing chart code). Use them; if a named skill is missing, honor its intent. Accessibility floor: keyboard scrubbing in the replay viewer, visible focus, WCAG AA contrast.

## 8. AI Narrator (designed now, built later)

`cf-narrator` defines the seam so the AI layer is a drop-in, not a rewrite:

```rust
pub trait CoachingNarrator {
    /// Turn one insight (+ match context) into user-facing coaching text.
    fn narrate(&self, insight: &Insight, ctx: &MatchContext) -> Narration;
    /// Optional match-level summary from the full insight set.
    fn summarize(&self, insights: &[Insight], ctx: &MatchContext) -> Option<Narration>;
}
```

- **v1 ships `TemplateNarrator` only**: deterministic, parameterized templates per detector with enough variants to not feel robotic. Zero cost, offline, testable. Write the templates like a good coach talks: specific, actionable, no filler ("You died isolated at Connector 3 rounds running — mid-round, no teammate closer than Jungle. Either arrive with the Connector player or hold one step deeper.").
- **`ClaudeNarrator` (future, feature-flagged off):** user-supplied API key, batched per-match (one request summarizing the insight set, not one per insight), responses cached in SQLite keyed by insight-set hash, hard per-match token budget, cheapest capable model tier. When you build it, invoke the `claude-api` skill for current models/pricing/SDK — do not code it from memory. Cost concern is explicit: the app must be fully useful with the AI off.

## 9. Non-Goals (v1) — do not build

- No interaction with the running game client, no overlays, no injection — nothing that could ever look like a cheat or trip VAC.
- No HLTV/FACEIT scraping or auto-downloading; corpus demos are user-supplied local files.
- No accounts, no backend service, no telemetry. Fully local.
- No live/in-progress match analysis; demos only.
- No macOS/Linux distribution polish; Windows installer + mac dev build.

## 10. Testing, Fixtures, CI

### 10.1 Fixtures
`fixtures/` is gitignored (demos are 50–400 MB) with a committed `fixtures/README.md` explaining what to place there. **First user request of the project (M0): ask the owner to drop 2–3 of their own matchmaking/FACEIT demos plus (by M5) a handful of pro demos into `fixtures/`.** If blocked before they respond, any publicly downloadable match demo unblocks parser work — but detector *tuning* judgments wait for the owner's own demos.

### 10.2 Test strategy
- **Golden snapshots:** parse a fixture demo → serialize a compact summary (rounds, scores, kill feed, event counts, spot-checked positions) → commit the JSON snapshot (small) to git. Parser refactors and demoparser2 upgrades diff against it. Validate once against the in-game scoreboard by hand and record that validation in the snapshot's README.
- **Detector unit tests:** synthetic `MatchData` scenario builders (`Scenario::new().kill(a, b, tick, pos)...`) exercising each classification edge (traded at 2.9 s vs 3.1 s, flash at 1.09 s vs 1.11 s...). Detectors are pure — test them hard here; this is where correctness lives. Follow `superpowers:test-driven-development` for every detector.
- **Frontend:** vitest + React Testing Library for logic-bearing components; unit-test replay coordinate/interpolation math exhaustively (it will otherwise eat debugging sessions). Playwright smoke later only if it earns its keep.
- **Integration (local/nightly, not CI-blocking):** full parse+analyze of fixture demos with performance assertions (§10.4).

### 10.3 CI (GitHub Actions, from M0)
- Every push: `cargo fmt --check`, `clippy -D warnings`, `cargo test`; `tsc --noEmit`, eslint, vitest. macOS runner primary; add a Windows build job by M2 so Windows breakage surfaces early, not at ship time. Tagged releases: `tauri build` artifacts (Windows NSIS installer + mac .app).
- CI never needs a real demo (golden snapshots + synthetic scenarios cover it).

### 10.4 Performance budgets (assert in nightly integration tests)
- Parse + analyze a full MR12 demo: **≤ 30 s** on Apple Silicon (stretch 15 s), streaming progress the whole way; peak memory ≤ ~1.5 GB.
- Replay playback 60 fps; scrub-seek to any tick < 100 ms; screens interactive < 1 s on a library of 100 matches.

## 11. Process & Context Management — as important as the code

You will burn through many context windows building this. The repo is your long-term memory; conversation is scratch space. **A fresh session reading only `CLAUDE.md` → `docs/PROGRESS.md` → the active plan must be able to continue within minutes.**

### 11.1 CLAUDE.md (create at M0, keep ≤ ~120 lines)
Contents, in order: one-paragraph project summary + pointer to `PROMPT.md` as the spec; stack summary; exact dev commands (setup, run, test, lint, build) kept **always correct** — update in the same commit that changes them; architecture map (5–10 lines, where things live, the `cf-parser`-output-is-the-boundary rule); conventions (commit style, ADR policy, threshold-config rule, evidence contract); and a final line: "Current state: see docs/PROGRESS.md". Never let CLAUDE.md rot or bloat — it is loaded into every session's context; stale commands in it are worse than none.

### 11.2 docs/PROGRESS.md (the resume file)
Sections: **Now** (the in-flight task, precisely — "D2 flash detector: conversion window logic done, team-flash aggregation not started, see failing test X"), **Next** (ordered short queue), **Done** (one line per completed chunk, newest first), **Decisions** (one-liners linking ADRs), **Gotchas** (hard-won facts: weird demo edge cases, API surprises, things a future session would otherwise rediscover). Update it and commit **at the end of every work chunk and whenever context is running long** — before compaction, not after. When you complete a milestone, also update the checklist in §13.

### 11.3 ADRs (docs/adr/ADR-NNNN-title.md)
One per significant decision: context, options, decision, consequences — half a page max. Mandatory ADRs already known: 0001 demoparser2 viability & R1 fallback status; radar asset sourcing/licensing; chart library; position downsampling rate; DB schema v1.

### 11.4 Skill mapping (check the available-skills listing each session; use these when present)
- Per milestone: `superpowers:writing-plans` to produce `docs/plans/M<N>-*.md`, then `superpowers:executing-plans` (or `superpowers:subagent-driven-development` when tasks are independent) to execute with checkpoints.
- `superpowers:test-driven-development` for every detector and all replay math. `superpowers:systematic-debugging` before proposing fixes for any bug. `superpowers:verification-before-completion` before claiming any milestone done. `superpowers:requesting-code-review` / `/code-review` at milestone boundaries — act on findings via `superpowers:receiving-code-review`.
- UI work: `frontend-design:frontend-design` + `dataviz` (see §7). The `run` skill to actually launch the app when verifying.
- Do **not** re-run `superpowers:brainstorming` for the overall product (this spec is the brainstorm output); use it only if the owner requests a genuinely new feature area mid-build.

### 11.5 Git discipline
Conventional-commit style messages (`feat(analysis): ...`, `fix(replay): ...`); every commit leaves CI green; push after each unit of work (Ground Rule 1); milestone completions get an annotated tag (`m0`, `m1`, ...). Branches optional while solo on `main`; if you branch, use `superpowers:using-git-worktrees` and merge promptly via `superpowers:finishing-a-development-branch`.

## 12. Verification bar

"Done" for any user-visible feature means: tests green **and** you launched the actual app, exercised the feature on a real fixture demo, and confirmed the output is *sane against the demo's reality* (spot-check: does the death you flagged "isolated" actually look isolated in the replay viewer?). For detector work, hand-verify at least 3 flagged instances per detector against the replay before calling it done. Never report a milestone complete without having run the app that day.

## 13. Milestones

Work strictly in order; each has a definition of done (DoD). Check them off here (edit this file) as they complete.

- [x] **M0 — Skeleton & parser proof.** *(done 2026-08-19)* Tauri 2 + React + Rust workspace scaffolded; CI green on push; ADR-0001 written: demoparser2 pulled in, a fixture demo parsed, kill feed + round scores printed and hand-validated against the in-game scoreboard. `CLAUDE.md`, `docs/PROGRESS.md`, `fixtures/README.md` created. Owner asked for demos. **DoD: `git clone` → documented setup → see a real match's kill feed.**
- [x] **M1 — Ingest pipeline & Library.** *(done 2026-08-19)* `MatchData` model finalized; normalized rounds (§6.2 quirks handled + golden-tested); SQLite schema v1 + migrations; import command with streaming progress; Library screen listing real imported matches; player-identity detection. **DoD: import 3 real demos through the UI; relaunch persistence; golden snapshots committed.**
- [x] **M2 — Replay viewer.** *(done 2026-08-19)* Radar assets + calibration data wired (ADR on sourcing/licensing); playback with interpolated player dots, deaths, utility lifetimes, bomb state, kill feed, roster HP; scrubber with round/event pips; deep-link API (`show_evidence(EvidenceRef)`) working end to end; Windows CI build job added. **DoD: watch a full real round smoothly at 60 fps; jump to any kill from the timeline.**
- [x] **M3 — Core detectors.** *(done 2026-08-19)* D1, D2, D3 built TDD-style with scenario builders; `DetectorConfig` with §6.4 defaults; insights persisted with evidence refs; hand-verification per §12 done. **DoD: real demo produces correct, evidence-backed insights for deaths/flashes/utility.**
- [x] **M4 — Match Report.** Insight feed UI (grouping, severity × recurrence ranking, evidence chips → replay deep links); round timeline strip; `TemplateNarrator` v1 with quality bar per §8; D4 + D5 added. **DoD: the owner reviews one of their own matches end-to-end and gets at least one insight they agree is real and actionable.**
- [ ] **M5 — Reference corpus & D6.** Corpus screen + ingestion; occupancy grids per map/side/phase; D6 with honesty rules + minimum-corpus gate; heatmap rendering (invoke `dataviz` first). **DoD: with ~8 pro demos on one map, positioning comparison produces sane, replay-backed output on the owner's demo.**
- [ ] **M6 — Trends, polish, ship.** Trends screen; Settings (thresholds, identity); empty/error states everywhere (corrupt demo, wrong game, mid-parse crash recovery); app icon; tagged release with Windows installer + mac app; README with screenshots. **DoD: owner installs the Windows build from CI artifacts on their gaming PC and analyzes a fresh match unassisted.**

After M6: revisit §9 non-goals and §8 `ClaudeNarrator` with the owner before choosing v2 scope.

## 14. First Actions (this session, in order)

1. Read this entire file. Skim the repo (near-empty: README + git remote).
2. Check the available-skills listing; note which §11.4 skills exist in this environment.
3. Research to close M0 unknowns (web search + GitHub): demoparser2 Rust-core usability as a dependency (R1), current Tauri 2 scaffolding practice, current stable toolchain versions. Timebox it; write findings into ADR-0001 draft.
4. Create `CLAUDE.md`, `docs/PROGRESS.md`, `docs/adr/`, `fixtures/README.md` per §11. Commit and push.
5. Invoke `superpowers:writing-plans` to write `docs/plans/M0-skeleton.md` from §13 M0. Then execute it per `superpowers:executing-plans`.
6. In your first message to the owner, batch the asks: 2–3 of their own demos into `fixtures/` (with the §6.1 instructions for downloading them), plus anything R1 research surfaced that needs a product call.

Build it like it's going to be used every day — because it is.
