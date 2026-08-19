# PROGRESS

The resume file. A fresh session reads CLAUDE.md → this file → the active plan in `docs/plans/` and continues within minutes. Update at the end of every work chunk and before context runs long.

## Now

M0 complete (tagged `m0`). Spec expanded 2026-08-19: owner supplied `docs/spec/death-taxonomy.md` (15-class death taxonomy, H1–H16 rule families, cross-demo habit tracking — see PROMPT.md §5A) + five of their own demos (all verified parsing) + tracked identity (76561199228328773 / misosoupy3). Next: M1 plan via superpowers:writing-plans → `docs/plans/M1-ingest.md`.

## Next

1. M1 plan: MatchData model, round normalization (§6.2 quirks — warmup/knife rounds, round_officially_ended), SQLite schema v1 + migrations (ADR needed — must carry `death_class`, per-rule flags w/ rule_id+confidence+secondary_tags, cross-demo keys per §5A), import command with Progress events, Library screen, player-identity detection (default tracked: 76561199228328773)
2. M1 per-tick needs driven by §5A: `spotted` (bool only — no spotted_by_mask, see spec §5.1), `active_weapon` (derive weapon_switch by diffing at cache-write, spec §5.3), filter `is_alive` before any per-tick field (spec §5.2), position/yaw/health
3. Golden-test round normalization against fixture demos (own demos: 17/16/20/24/24 raw round_ends incl. any warmup quirks — verify)
4. ADRs due during M1: DB schema v1; position downsampling rate (measure first). M2: radar/nav assets — evaluate awpy nav+radar bundle (spec §5.5: no de_cache, no callouts; `last_place_name` prop may cover captions)

## Done

- 2026-08-19: Owner demos verified — all five `fixtures/own/*.dem` parse (de_dust2 17r/127k, de_inferno 16r/118k, de_inferno 20r/153k, de_mirage 24r/183k, de_nuke 24r/192k; both "tie" demos are exactly 12–12); misosoupy3 (76561199228328773) in every roster; proof API extended with players (steamid/name/team), golden regenerated.
- 2026-08-19: Spec addendum integrated: `docs/spec/death-taxonomy.md` + PROMPT.md §5A + CLAUDE.md conventions (rule ids load-bearing, rules-as-data, confidence, silence bias, class-13 CI metric).
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
- demoparser's `csgoproto` crate needs `protoc` at build time (prost-build) — CI installs it via `arduino/setup-protoc@v3`; locally it was already present.
- Release-mode demo parse of a 222 MB demo ≈ 0.2 s warm (rayon); debug mode is much slower — run demo-touching tests with `--release`.
- demofile-net's committed snapshots (`src/DemoFile.Test/Snapshots/` on GitHub) are independent ground truth for our fixture demos — great for validating round/kill extraction without hand-counting.
- Shell cwd drifts between Bash calls — use absolute paths or explicit `cd` (a `git add .github` once failed from `src-tauri/`).
