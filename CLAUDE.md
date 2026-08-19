# ClutchFactor

Desktop CS2 coaching app: parses match demo files (`.dem`) and produces coaching insights backed by a 2D replay viewer — not a stats tracker. **`PROMPT.md` is the approved spec and engineering charter — read it before doing anything else.** `docs/spec/death-taxonomy.md` (PROMPT.md §5A) governs death/duel analysis: 15-class taxonomy, H1–H16 rule families, cross-demo habit tracking. This file is session context; keep it ≤120 lines and always correct (§11.1).

Tracked player (owner): SteamID64 `76561199228328773`, in-game name `misosoupy3` — present in all five `fixtures/own/` demos.

## Stack

Tauri 2 (2.10.x) shell · Rust core (demoparser2 git dep, detectors, rusqlite/SQLite) · React + TypeScript + Vite frontend · Canvas 2D replay rendering · GitHub Actions CI.

## Dev commands

Prereqs: Rust stable via rustup (installed: 1.97.x), Node 22 LTS, pnpm 10. Fresh shells may need `source "$HOME/.cargo/env"`.

```sh
pnpm install                 # setup (frontend deps)
pnpm tauri dev               # run the desktop app (compiles Rust + Vite dev server)
pnpm typecheck && pnpm lint && pnpm test:run          # frontend checks (tsc, eslint, vitest)
cargo fmt --manifest-path src-tauri/Cargo.toml --all --check
cargo clippy --manifest-path src-tauri/Cargo.toml --workspace --all-targets -- -D warnings
cargo test --manifest-path src-tauri/Cargo.toml --workspace          # add --release to speed up golden demo parse
# M0 parser proof — kill feed + round scores from a real demo:
cargo run -p cf-parser --release --example print_match --manifest-path src-tauri/Cargo.toml -- fixtures/public/<demo>.dem
```

## Architecture map

```
src-tauri/crates/cf-parser/    demoparser2 wrapper → normalized MatchData (ONLY interface cf-analysis sees)
src-tauri/crates/cf-analysis/  Detector trait impls: pure fns MatchData → Vec<Insight>, no I/O
src-tauri/crates/cf-store/     SQLite (rusqlite bundled), embedded versioned migrations
src-tauri/crates/cf-narrator/  CoachingNarrator trait + TemplateNarrator (v1)
src-tauri/src/                 Tauri app: snake_case commands, Progress{stage,pct,detail} event
src/                           React: screens/, replay/ (canvas + coord math), components/
assets/maps/                   radar images + per-map calibration (awpy data, vendored w/ attribution)
fixtures/                      real .dem files, gitignored; see fixtures/README.md
```

**Boundary rule:** no demoparser2 types leak past `cf-parser` — this keeps risk R1's fallback (C# demofile-net sidecar) survivable.

## Conventions

- Conventional commits (`feat(analysis): …`); push after every unit of work; milestone tags `m0`, `m1`, …
- ADRs in `docs/adr/ADR-NNNN-*.md` for every significant decision — half a page max.
- **Evidence contract:** every `Insight` carries `EvidenceRef { round, tick_start, tick_end, focus_players, camera_hint }` the replay viewer can jump to. No evidence → detector gets redesigned, not shipped.
- All detector thresholds live in `DetectorConfig` with documented defaults (PROMPT.md §6.4) — never scatter magic numbers; thresholds in seconds/world units, never ticks.
- Detectors are pure functions over `MatchData`; TDD with synthetic scenario builders is mandatory.
- Rule engine (§5A): rule ids (`H2_ISOLATED_DEATH`, …) are load-bearing — never rename/renumber; rules are data (YAML), each emits `confidence`; approximations bias toward silence (false negative ≫ false positive); every rule gets a hand-verified golden clip test; class-13 share is a CI regression metric.
- Real demos only — no fake match data, no placeholder insights.
- Verify external APIs (demoparser2, Tauri) against docs/real output before coding against them.

Current state: see docs/PROGRESS.md
