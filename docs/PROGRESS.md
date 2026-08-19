# PROGRESS

The resume file. A fresh session reads CLAUDE.md → this file → the active plan in `docs/plans/` and continues within minutes. Update at the end of every work chunk and before context runs long.

## Now

M0 — Skeleton & parser proof. Research done (see ADR-0001), context docs created. Next in-flight step: write `docs/plans/M0-skeleton.md` via superpowers:writing-plans, then execute it (scaffold Tauri 2 + React workspace, CI, parser proof against a real demo).

## Next

1. M0 plan → scaffold app (create-tauri-app react-ts, pnpm) → restructure into cargo workspace with cf-* crates
2. CI (GitHub Actions): fmt/clippy/test/tsc/eslint/vitest on push
3. Pull demoparser2 as git dep (pin rev 266a831), parse a real demo, print kill feed + round scores, hand-validate
4. ADR-0001 finalize with parse proof; commit golden fixture summary
5. Ask owner for 2–3 of their own demos (batched ask, §14.6)

## Done

- 2026-08-19: Rust toolchain installed (rustup, 1.97.1 + clippy + rustfmt). Node 22.19 / pnpm 10.33 already present.
- 2026-08-19: M0 research closed: demoparser2 Rust core viable as git dep (ADR-0001 draft); Tauri 2 stable 2.10.x; create-tauri-app react-ts is current practice.
- 2026-08-19: CLAUDE.md, docs/PROGRESS.md, docs/adr/ADR-0001, fixtures/README.md created.

## Decisions

- ADR-0001: use demoparser2 Rust core (`parser` crate) as pinned git dependency; R1 fallback = C# demofile-net sidecar. Status: draft-accepted, proof pending first real parse.

## Gotchas

- `cargo`/`rustc` need `source "$HOME/.cargo/env"` in fresh non-login shells.
- demoparser2's Rust core crate is named just `parser` — depend on it as `demoparser = { git = "https://github.com/LaihoE/demoparser", rev = "…", package = "parser" }`. It path-depends on sibling crate `csgoproto` (same repo — resolves automatically in a git dep).
- demoparser2 Rust API: `ParserInputs` (wanted_events, wanted_player_props, …) → `Parser::new(inputs, ParsingMode::Normal)` → `parser.parse_demo(&mmap)`; mmap via `first_pass::parser_settings::create_mmap(path)`. Python binding (`src/python/src/lib.rs` in their repo) is the reference for correct usage.
