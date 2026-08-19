# PROGRESS

The resume file. A fresh session reads CLAUDE.md → this file → the active plan in `docs/plans/` and continues within minutes. Update at the end of every work chunk and before context runs long.

## Now

M0 complete (tagged `m0`). Next up: M1 — Ingest pipeline & Library (PROMPT.md §13). No M1 plan written yet; start with superpowers:writing-plans → `docs/plans/M1-ingest.md`. Blocked-but-not-blocking: waiting on owner's own demos for detector-tuning realism (asked 2026-08-19; public pro demos unblock all M1 work meanwhile).

## Next

1. M1 plan: MatchData model, round normalization (§6.2 quirks — warmup/knife rounds, round_officially_ended), SQLite schema v1 + migrations (ADR needed), import command with Progress events, Library screen, player-identity detection
2. Golden-test round normalization against the fixture demo (extend the M0 golden)
3. ADRs due during M1: DB schema v1; position downsampling rate (measure first)

## Done

- 2026-08-19: **M0 complete** — Tauri 2 + React + TS scaffold; cargo workspace (cf-parser/cf-analysis/cf-store/cf-narrator); CI green (macos-14 rust job, ubuntu web job); demoparser2 pinned git dep proven: 222 MB pro demo parsed, kill feed + 23 round ends validated against demofile-net's independent snapshot (exact tick match) + real match result (13–10 NAVI Javelins); golden snapshot + fixture-gated golden test; app window launches. Tag `m0`.
- 2026-08-19: Fixtures: demofile-net public bundle downloaded (3 full pro demos incl. de_mirage, de_ancient, de_vertigo + small test demos) into fixtures/public/.
- 2026-08-19: Rust toolchain installed (rustup 1.97.1); context docs + ADR-0001 created.

## Decisions

- ADR-0001: demoparser2 Rust core as pinned git dep (`package = "parser"`, rev 266a831) — **proven at M0**; R1 fallback (C# sidecar) not needed.

## Gotchas

- `cargo`/`rustc` need `source "$HOME/.cargo/env"` in fresh non-login shells.
- demoparser2 Rust API: `ParserInputs` (16 required fields, incl. `huffman_lookup_table: &create_huffman_lookup_table()`) → `Parser::new(inputs, ParsingMode::Normal)` → `parse_demo(&create_mmap(path_string)?)`. `create_mmap` takes `String`, not `&Path`.
- Event enrichment: `userid`-bearing events get `user_name`/`user_steamid` fields (prefix map: attacker/user/assister/victim in `game_events.rs`); `winner` on `round_end` is `Variant::I32` (2 = T side, 3 = CT side). Set `parse_ents: true` for name enrichment.
- `pnpm/action-setup@v4` in CI requires `"packageManager"` in package.json — pinned `pnpm@10.33.2`.
- Release-mode demo parse of a 222 MB demo ≈ 0.2 s warm (rayon); debug mode is much slower — run demo-touching tests with `--release`.
- demofile-net's committed snapshots (`src/DemoFile.Test/Snapshots/` on GitHub) are independent ground truth for our fixture demos — great for validating round/kill extraction without hand-counting.
- Shell cwd drifts between Bash calls — use absolute paths or explicit `cd` (a `git add .github` once failed from `src-tauri/`).
