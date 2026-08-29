# ClutchFactor

Desktop CS2 coaching app: parses match demo files (`.dem`) and produces coaching insights backed by a 2D replay viewer — not a stats tracker. **`PROMPT-V1.md` is the approved v1 engineering charter — read it first; it supersedes `PROMPT.md` where they conflict, and PROMPT.md remains binding everywhere else.** v1 pillars: round-by-round coaching (GitHub issue #9 IS that spec — read it fully), Gemini AI layer, premium design system, coaching depth + stats. `docs/spec/death-taxonomy.md` (PROMPT.md §5A) governs death/duel analysis: 15-class taxonomy, H1–H16 rule families, cross-demo habit tracking. This file is session context; keep it ≤120 lines and always correct (§11.1).
`docs/spec/play-ledger-and-coach.md` governs V1.2b's play ledger (every round narrated) and V1.3's coach contract (facts validated, judgment free).
`docs/spec/stats-and-understanding.md` governs V1.4's stats (every number cross-checkable by SQL — `scripts/stats-crosscheck.sql` — and linked to its coaching on `/watches`).

Tracked player (owner): SteamID64 `76561199228328773`, in-game name `misosoupy3` — present in all five `fixtures/own/` demos.

## Stack

Tauri 2 (2.10.x) shell · Rust core (demoparser2 git dep, detectors, rusqlite/SQLite) · React + TypeScript + Vite frontend · Canvas 2D replay rendering · GitHub Actions CI · marketing site in `site/` (vanilla TS + CSS on Vite, Vercel — `docs/spec/marketing-site.md`, ADR-0012).

## Dev commands

Prereqs: Rust stable via rustup (installed: 1.97.x), Node 22 LTS, pnpm 10. Fresh shells may need `source "$HOME/.cargo/env"`.

```sh
pnpm install                 # setup (frontend deps)
pnpm tauri dev               # run the desktop app (compiles Rust + Vite dev server)
pnpm typecheck && pnpm lint && pnpm test:run          # frontend checks (tsc, eslint, vitest)
pnpm -C site dev                                     # marketing site; checks: pnpm -C site typecheck && pnpm -C site lint && pnpm -C site test:run && pnpm -C site build
cargo fmt --manifest-path src-tauri/Cargo.toml --all --check
cargo clippy --manifest-path src-tauri/Cargo.toml --workspace --all-targets -- -D warnings
cargo test --manifest-path src-tauri/Cargo.toml --workspace          # add --release to speed up golden demo parse
# Parser proof — kill feed + round scores from a real demo:
cargo run -p cf-parser --release --example print_match --manifest-path src-tauri/Cargo.toml -- fixtures/public/<demo>.dem
# Detector output — insights, rule flags, death-class table + class-13 share:
cargo run -p cf-analysis --release --example print_insights --manifest-path src-tauri/Cargo.toml -- fixtures/own/<demo>.dem <steamid64>
```

Coach (V1.3): key via Settings → Coach or `CLUTCHFACTOR_GEMINI_KEY`; debug builds read the gitignored repo-root `env.local` (`GEMINI_API_KEY=…`) — never commit it; CI's `secrets` job greps for key shapes.

## Architecture map

```
src-tauri/crates/cf-parser/    demoparser2 wrapper → normalized MatchData (ONLY interface cf-analysis sees)
src-tauri/crates/cf-analysis/  Detector trait impls: pure fns MatchData → Vec<Insight>, no I/O; stats (coaching-first, health-capped damage) + catalog (what the coach watches)
src-tauri/crates/cf-store/     SQLite (rusqlite bundled), embedded versioned migrations
src-tauri/crates/cf-narrator/  CoachingNarrator trait + TemplateNarrator (v1)
src-tauri/src/                 Tauri app: snake_case commands, Progress{stage,pct,detail} event
src/                           React: screens/, replay/ (canvas + coord math), components/
assets/maps/                   radar images + per-map calibration (awpy data, vendored w/ attribution)
fixtures/                      real .dem files, gitignored; see fixtures/README.md
site/                          marketing site, standalone package (own lockfile): public/shots via `pnpm -C site shots`, release links in src/release.ts
```

**Boundary rule:** no demoparser2 types leak past `cf-parser` — this keeps risk R1's fallback (C# demofile-net sidecar) survivable.

## Conventions

- Conventional commits (`feat(analysis): …`); milestone tags `m0`, `m1`, …
- `main` is ruleset-protected (ADR-0005) — branch, `gh pr create`, `gh pr merge --auto --squash`, then **verify it landed**: `gh pr view <n> --json state,mergeStateStatus` (auto-merge stalls silently on a `BEHIND` branch — rebase and re-arm). PRs are the default path to main; the admin bypass exists for mid-release emergencies only and skips every required check — if you must use it, say so in the commit message.
- ADRs in `docs/adr/ADR-NNNN-*.md` for every significant decision — half a page max.
- **Evidence contract:** every `Insight` carries `EvidenceRef { round, tick_start, tick_end, focus_players, camera_hint }` the replay viewer can jump to. No evidence → detector gets redesigned, not shipped.
- All detector thresholds live in `DetectorConfig` with documented defaults (PROMPT.md §6.4) — never scatter magic numbers; thresholds in seconds/world units, never ticks.
- Detectors are pure functions over `MatchData`; TDD with synthetic scenario builders is mandatory.
- Rule engine (§5A): rule ids (`H2_ISOLATED_DEATH`, …) are load-bearing — never rename/renumber; rules are data (YAML), each emits `confidence`; approximations bias toward silence (false negative ≫ false positive); every rule gets a hand-verified golden clip test; class-13 share is a golden-test regression metric (needs `fixtures/` demos, so it runs locally — CI runners only get the synthetic suites).
- Releases: bump `package.json` + `src-tauri/tauri.conf.json` + `src-tauri/Cargo.toml` (workspace crates stay 0.1.0), tag `vX.Y.Z` on main → `release.yml` builds both installers; then `gh release edit vX.Y.Z --notes-file …`; then update `site/src/release.ts` (version, asset names, byte sizes from `gh release view vX.Y.Z --json assets`) and re-run `pnpm -C site shots` if screenshots changed and re-check the site's Honest-limits copy (which death classes are unbuilt) against the release. Debug builds print `perf: <command> <ms> ms`; `VITE_FAIL_IPC=<command> pnpm tauri dev` provokes a screen's error state.
- Real demos only — no fake match data, no placeholder insights.
- Verify external APIs (demoparser2, Tauri) against docs/real output before coding against them.

Current state: see docs/PROGRESS.md
