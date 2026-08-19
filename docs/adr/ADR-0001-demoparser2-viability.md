# ADR-0001: demoparser2 Rust core as the demo parsing library

Status: accepted (draft — final proof is the M0 fixture parse; update this line when done)
Date: 2026-08-19

## Context

ClutchFactor needs a CS2 `.dem` parser in Rust (PROMPT.md §3). demoparser2 (LaihoE/demoparser) is the fastest, most battle-tested CS2 parser, but its Rust core is not published on crates.io — risk R1 is that it proves unusable as a library.

## Research findings (2026-08-19)

- Repo layout: Rust core is crate `parser` v0.1.1 at `src/parser`, with a path dependency on sibling crate `csgoproto` (`src/csgoproto`). Python (`src/python`), Node (`src/node`), and WASM (`src/wasm`) bindings are thin wrappers over it.
- Public Rust API (as used by the Python binding): `first_pass::parser_settings::{ParserInputs, create_mmap}` → `parse_demo::Parser::new(inputs, ParsingMode::Normal)` → `parser.parse_demo(&mmap)` → output containing game events (`second_pass::game_events::GameEvent`) and per-tick prop data (`second_pass::variants::{Variant, VarVec}`).
- Actively maintained: latest commit 266a831 (2026-08-10). No semver discipline on the core (v0.1.1, generic name) — API may move between revs.

## Options

1. **Git dependency on the Rust core** — `package = "parser"` rename, pinned `rev`. Full-speed native, no IPC, single binary.
2. C# sidecar with demofile-net, JSON/MessagePack over subprocess IPC — the R1 fallback. Adds .NET runtime packaging + IPC complexity.
3. Other Rust parsers (e.g. source2-demo) — less battle-tested for CS2 match analytics; not evaluated further unless 1 fails.

## Decision

Option 1: `demoparser = { git = "https://github.com/LaihoE/demoparser", rev = "266a831f08b0264dd722b017a5c05d765206a7ed", package = "parser" }`, pinned rev, upgraded deliberately (diff against golden snapshots per §10.2).

## Consequences

- All demoparser2 types are confined to `cf-parser`; its normalized `MatchData` is the only interface downstream crates see. This keeps the fallback (option 2) survivable at the cost of a mapping layer.
- Pinned rev means we don't get upstream fixes automatically; upgrades are explicit commits validated by golden tests.
- If the git API breaks irrecoverably: switch to option 2 behind the same `cf-parser` output types.
