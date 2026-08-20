> **Historical record** of a completed milestone. The "push origin main" / "push per task" steps below predate ADR-0005 - `main` is PR-only now (branch, `gh pr create`, auto-merge). Do not copy the push flow from this file.

# M0 — Skeleton & Parser Proof Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Scaffolded Tauri 2 + React + Rust workspace with green CI, demoparser2 proven on a real demo: kill feed + round scores printed and hand-validated (PROMPT.md §13 M0).

**Architecture:** create-tauri-app scaffold merged into the existing repo; `src-tauri` becomes a Cargo workspace hosting the Tauri app plus `cf-parser`/`cf-analysis`/`cf-store`/`cf-narrator` library crates. `cf-parser` pulls demoparser2's Rust core as a pinned git dependency and exposes a proof API (`parse_proof_summary`) plus a `print_match` example. Golden snapshot committed; CI never needs a real demo.

**Tech Stack:** Tauri 2.10.x, React 18+ / TypeScript / Vite, pnpm, Rust 1.97 stable, demoparser2 (git rev `266a831f08b0264dd722b017a5c05d765206a7ed`), GitHub Actions (macos-14 for Rust, ubuntu for web checks).

**Spec:** `PROMPT.md` (§3 stack, §4 architecture, §10 testing/CI, §13 M0 DoD, §14 first actions). Research findings: `docs/adr/ADR-0001-demoparser2-viability.md`.

## Global Constraints

- Conventional commits; push after every completed task (PROMPT.md §2.1, §11.5).
- No demoparser2 types leak past `cf-parser` (§4). For M0 the proof example lives *inside* cf-parser, so this holds trivially — its public fn returns only cf-parser types.
- No fake data, no TODO stubs in committed code (§2.4).
- CI checks (§10.3): `cargo fmt --check`, `clippy -D warnings`, `cargo test`, `tsc --noEmit`, eslint, vitest. CI must be green on every push.
- Pin demoparser2 to rev `266a831f08b0264dd722b017a5c05d765206a7ed` with `package = "parser"` (ADR-0001).
- Rust on PATH via `source "$HOME/.cargo/env"`.
- Verified demoparser2 API (do not re-guess; if compile fails, read the vendored source under `~/.cargo/git/checkouts/`):
  - `parser::first_pass::parser_settings::{ParserInputs, create_mmap, FirstPassParser}`
  - `parser::second_pass::parser_settings::create_huffman_lookup_table`
  - `parser::parse_demo::{Parser, ParsingMode, DemoOutput}` — `Parser::new(inputs, ParsingMode::Normal)`, `.parse_demo(&mmap) -> Result<DemoOutput, DemoParserError>`
  - `DemoOutput { game_events: Vec<GameEvent>, header: Option<AHashMap<String,String>>, game_events_counter, .. }`
  - `GameEvent { name: String, fields: Vec<EventField>, tick: i32 }`; `EventField { name: String, data: Option<Variant> }`
  - `Variant::{Bool, U32, I32, F32, U64, String, ..}` (in `parser::second_pass::variants`)
  - `ParserInputs` fields (all required): `real_name_to_og_name: AHashMap<String,String>`, `wanted_players: Vec<u64>`, `wanted_player_props: Vec<String>`, `wanted_other_props: Vec<String>`, `wanted_prop_states: AHashMap<String, Variant>`, `wanted_ticks: Vec<i32>`, `wanted_events: Vec<String>`, `parse_ents: bool`, `parse_projectiles: bool`, `parse_grenades: bool`, `only_header: bool`, `only_convars: bool`, `huffman_lookup_table: &Vec<(u8,u8)>`, `order_by_steamid: bool`, `list_props: bool`, `fallback_bytes: Option<Vec<u8>>`.

---

### Task 1: Scaffold Tauri 2 + React + TypeScript app

**Files:**
- Create: `package.json`, `pnpm-lock.yaml`, `index.html`, `vite.config.ts`, `tsconfig.json`, `src/**` (template), `src-tauri/**` (template app crate), `public/**`
- Modify: `.gitignore` (merge template entries if any are missing)

**Interfaces:**
- Produces: working `pnpm install`, `pnpm tauri dev`-capable scaffold; `src-tauri/` app crate named `clutchfactor` with identifier `com.clutchfactor.app`.

- [x] **Step 1: Scaffold into a temp dir, merge into repo**

```bash
cd /private/tmp/claude-501/-Users-junlee-Documents-programming-clutchfactor/6cd645f7-ec15-4640-9fa2-a031eb067f90/scratchpad
pnpm create tauri-app@latest clutchfactor-scaffold --template react-ts --manager pnpm --identifier com.clutchfactor.app --yes
# Merge (template has no README.md collision risk with ours; verify before copying):
ls clutchfactor-scaffold
rsync -a clutchfactor-scaffold/ /Users/junlee/Documents/programming/clutchfactor/ --exclude .git
```

If the template writes a `README.md`, keep ours (`git checkout README.md` after rsync — check `git status`).

- [x] **Step 2: Install and verify the scaffold compiles**

```bash
cd /Users/junlee/Documents/programming/clutchfactor
pnpm install
pnpm tsc --noEmit || pnpm exec tsc --noEmit
source "$HOME/.cargo/env" && cargo check --manifest-path src-tauri/Cargo.toml
```

Expected: both succeed. (Full `pnpm tauri dev` GUI launch is verified in Task 8; `cargo check` proves the Rust side builds.)

- [x] **Step 3: Commit**

```bash
git add -A && git commit -m "feat: scaffold Tauri 2 + React + TS app (create-tauri-app, pnpm)" && git push origin main
```

### Task 2: Cargo workspace with cf-* crates

**Files:**
- Modify: `src-tauri/Cargo.toml` (add `[workspace]`)
- Create: `src-tauri/crates/cf-parser/{Cargo.toml,src/lib.rs}`, same for `cf-analysis`, `cf-store`, `cf-narrator`

**Interfaces:**
- Produces: workspace members `cf-parser`, `cf-analysis`, `cf-store`, `cf-narrator` (lib crates, edition 2021). Task 5 adds real content to `cf-parser`.

- [x] **Step 1: Add workspace section to src-tauri/Cargo.toml**

Append to `src-tauri/Cargo.toml`:

```toml
[workspace]
members = ["crates/cf-parser", "crates/cf-analysis", "crates/cf-store", "crates/cf-narrator"]
resolver = "2"
```

- [x] **Step 2: Create the four lib crates**

Each `src-tauri/crates/cf-<name>/Cargo.toml`:

```toml
[package]
name = "cf-parser"   # cf-analysis / cf-store / cf-narrator respectively
version = "0.1.0"
edition = "2021"

[dependencies]
```

Each `src/lib.rs` gets only a crate-level doc comment stating its §4 responsibility (no stub functions), e.g. for cf-parser:

```rust
//! demoparser2 wrapper producing normalized match data.
//!
//! Boundary rule (PROMPT.md §4): types from this crate are the ONLY interface
//! downstream crates (cf-analysis, cf-store) see — no demoparser2 types leak out.
```

cf-analysis: `//! Detectors: pure functions over MatchData -> Vec<Insight>. No I/O.`
cf-store: `//! SQLite persistence (rusqlite, bundled), embedded versioned migrations.`
cf-narrator: `//! CoachingNarrator trait + TemplateNarrator (PROMPT.md §8).`

- [x] **Step 3: Verify workspace builds clean**

```bash
source "$HOME/.cargo/env"
cargo fmt --manifest-path src-tauri/Cargo.toml --all
cargo clippy --manifest-path src-tauri/Cargo.toml --workspace --all-targets -- -D warnings
cargo test --manifest-path src-tauri/Cargo.toml --workspace
```

Expected: all pass (no tests yet is fine — exit 0).

- [x] **Step 4: Commit**

```bash
git add -A && git commit -m "feat: cargo workspace with cf-parser/cf-analysis/cf-store/cf-narrator crates" && git push origin main
```

### Task 3: Frontend tooling — eslint, vitest, typecheck scripts

**Files:**
- Create: `eslint.config.js`, `vitest.config.ts`
- Modify: `package.json` (scripts + devDependencies)

**Interfaces:**
- Produces: `pnpm typecheck`, `pnpm lint`, `pnpm test:run` — the exact commands CI (Task 4) runs.

- [x] **Step 1: Install dev deps**

```bash
pnpm add -D eslint @eslint/js typescript-eslint eslint-plugin-react-hooks vitest
```

- [x] **Step 2: Create eslint flat config**

`eslint.config.js`:

```js
import js from "@eslint/js";
import tseslint from "typescript-eslint";
import reactHooks from "eslint-plugin-react-hooks";

export default tseslint.config(
  { ignores: ["dist/", "src-tauri/"] },
  js.configs.recommended,
  ...tseslint.configs.recommended,
  {
    files: ["src/**/*.{ts,tsx}"],
    plugins: { "react-hooks": reactHooks },
    rules: { ...reactHooks.configs.recommended.rules },
  }
);
```

- [x] **Step 3: Create vitest config and scripts**

`vitest.config.ts`:

```ts
import { defineConfig } from "vitest/config";

export default defineConfig({
  test: {
    environment: "node",
    passWithNoTests: true, // first logic-bearing components land in M1
  },
});
```

`package.json` scripts (add):

```json
"typecheck": "tsc --noEmit",
"lint": "eslint .",
"test:run": "vitest run"
```

- [x] **Step 4: Verify all three commands pass**

```bash
pnpm typecheck && pnpm lint && pnpm test:run
```

Expected: exit 0 each (vitest reports "no test files found" but passes).

- [x] **Step 5: Commit**

```bash
git add -A && git commit -m "chore: eslint flat config, vitest, typecheck scripts" && git push origin main
```

### Task 4: CI — GitHub Actions on every push

**Files:**
- Create: `.github/workflows/ci.yml`

**Interfaces:**
- Consumes: `pnpm typecheck|lint|test:run` (Task 3), cargo workspace (Task 2).
- Produces: green check on every push; the gate every later task keeps green.

- [x] **Step 1: Write the workflow**

`.github/workflows/ci.yml`:

```yaml
name: ci
on:
  push:
  pull_request:

jobs:
  rust:
    runs-on: macos-14
    defaults:
      run:
        working-directory: src-tauri
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          components: rustfmt, clippy
      - uses: Swatinem/rust-cache@v2
        with:
          workspaces: src-tauri
      - run: cargo fmt --all --check
      - run: cargo clippy --workspace --all-targets -- -D warnings
      - run: cargo test --workspace

  web:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: pnpm/action-setup@v4
      - uses: actions/setup-node@v4
        with:
          node-version: 22
          cache: pnpm
      - run: pnpm install --frozen-lockfile
      - run: pnpm typecheck
      - run: pnpm lint
      - run: pnpm test:run
```

(macOS runner primary per §10.3 for the Rust/Tauri side; web checks are platform-neutral so ubuntu is fine. Windows build job comes at M2 per spec.)

- [x] **Step 2: Push and verify green**

```bash
git add .github && git commit -m "ci: fmt/clippy/test + tsc/eslint/vitest on push" && git push origin main
gh run watch --exit-status || gh run list --limit 1
```

Expected: run completes with success. If it fails, fix before proceeding (every commit leaves CI green).

### Task 5: Fixture acquisition — real public demo

**Files:**
- Create: `fixtures/public/` (gitignored content), `fixtures/goldens/README.md` (committed provenance/validation notes)

**Interfaces:**
- Produces: at least one full-match CS2 `.dem` at a known path under `fixtures/public/`, plus its expected final score from a public match page (for Task 6 hand-validation).

- [x] **Step 1: Download the demofile-net test-demo bundle (public, used by their MIT test suite)**

```bash
cd /Users/junlee/Documents/programming/clutchfactor/fixtures
mkdir -p public && cd public
curl -C - --retry 5 --fail 'https://pub-df0163da89b24187b28fd37c8dc7c8a1.r2.dev/demofile-net-demos-9.zip' -o demos.zip
unzip -o demos.zip && rm demos.zip && ls -la
```

Expected: several `.dem` files including at least one full HLTV/pro match demo (e.g. a `navi-javelins-vs-9-pandas-*.dem` style name). If the URL is dead, fallback: check `demos/download.sh` on saul/demofile-net main for the current URL.

- [x] **Step 2: Identify the full match demo + its real-world score**

Pick the largest `.dem` (full matches are 50–400 MB). Find the match on HLTV (search team names from the filename) and record map + final score. Write `fixtures/goldens/README.md`:

```markdown
# Golden snapshots — provenance & hand-validation

| Golden | Source demo (gitignored) | Origin | Hand-validated against |
|---|---|---|---|
| (filled by Task 7) | fixtures/public/<name>.dem | demofile-net test bundle (demofile-net-demos-9.zip, public R2 bucket) | HLTV match page <url>: <team A> <X>–<Y> <team B> on <map> |
```

- [x] **Step 3: Commit the README (demos stay gitignored)**

```bash
git add fixtures/goldens/README.md && git commit -m "docs: golden provenance README, public fixture source recorded" && git push origin main
```

### Task 6: cf-parser — demoparser2 integration + proof API (TDD)

**Files:**
- Modify: `src-tauri/crates/cf-parser/Cargo.toml`, `src-tauri/crates/cf-parser/src/lib.rs`
- Create: `src-tauri/crates/cf-parser/src/proof.rs`, `src-tauri/crates/cf-parser/examples/print_match.rs`

**Interfaces:**
- Produces:
  - `cf_parser::proof::ProofSummary { map: String, kills: Vec<KillEntry>, round_ends: Vec<RoundEnd> }`
  - `cf_parser::proof::KillEntry { tick: i32, attacker: String, victim: String, weapon: String, headshot: bool }`
  - `cf_parser::proof::RoundEnd { tick: i32, winner: String }` (winner: `"CT"`/`"T"`/raw number as string)
  - `cf_parser::proof::parse_proof_summary(path: &Path) -> Result<ProofSummary, String>`
  - Pure helpers (unit-tested): `field_str(ev: &GameEvent, name: &str) -> Option<String>`, `field_bool`, `field_i32`, `winner_label(v: &Variant) -> String`

- [x] **Step 1: Add dependencies to cf-parser**

`src-tauri/crates/cf-parser/Cargo.toml` `[dependencies]`:

```toml
demoparser = { git = "https://github.com/LaihoE/demoparser", rev = "266a831f08b0264dd722b017a5c05d765206a7ed", package = "parser" }
ahash = "0.8"
```

Run `cargo check -p cf-parser --manifest-path src-tauri/Cargo.toml` — expect the git dep to vendor and compile. **If compile errors reference missing/renamed items, read the vendored source at `~/.cargo/git/checkouts/demoparser-*/266a831/src/parser/src/` and fix names — never guess.**

- [x] **Step 2: Write failing unit tests for the pure helpers**

`src-tauri/crates/cf-parser/src/proof.rs` (tests first, at bottom of new file with `use` stubs so it fails to compile on missing fns — that is the red state):

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use demoparser::second_pass::game_events::{EventField, GameEvent};
    use demoparser::second_pass::variants::Variant;

    fn ev(fields: Vec<(&str, Option<Variant>)>) -> GameEvent {
        GameEvent {
            name: "player_death".to_string(),
            tick: 100,
            fields: fields
                .into_iter()
                .map(|(n, d)| EventField { name: n.to_string(), data: d })
                .collect(),
        }
    }

    #[test]
    fn field_str_extracts_string_variant() {
        let e = ev(vec![("attacker_name", Some(Variant::String("dev1ce".into())))]);
        assert_eq!(field_str(&e, "attacker_name").as_deref(), Some("dev1ce"));
    }

    #[test]
    fn field_str_none_when_missing_or_wrong_type() {
        let e = ev(vec![("headshot", Some(Variant::Bool(true)))]);
        assert_eq!(field_str(&e, "attacker_name"), None);
        assert_eq!(field_str(&e, "headshot"), None);
    }

    #[test]
    fn field_bool_extracts() {
        let e = ev(vec![("headshot", Some(Variant::Bool(true)))]);
        assert_eq!(field_bool(&e, "headshot"), Some(true));
    }

    #[test]
    fn winner_label_maps_team_numbers() {
        assert_eq!(winner_label(&Variant::U32(3)), "CT");
        assert_eq!(winner_label(&Variant::U32(2)), "T");
        assert_eq!(winner_label(&Variant::I32(3)), "CT");
        assert_eq!(winner_label(&Variant::String("CT".into())), "CT");
    }
}
```

Run: `cargo test -p cf-parser --manifest-path src-tauri/Cargo.toml` → expect compile FAIL (functions not defined).

- [x] **Step 3: Implement helpers + proof API**

Top of `src-tauri/crates/cf-parser/src/proof.rs`:

```rust
//! M0 parser proof: kill feed + round ends from a real demo.
//! Throwaway-adjacent — M1 replaces this with the full MatchData model.

use std::path::Path;

use ahash::AHashMap;
use demoparser::first_pass::parser_settings::{create_mmap, ParserInputs};
use demoparser::parse_demo::{Parser, ParsingMode};
use demoparser::second_pass::game_events::GameEvent;
use demoparser::second_pass::parser_settings::create_huffman_lookup_table;
use demoparser::second_pass::variants::Variant;

#[derive(Debug, Clone, serde::Serialize)]
pub struct KillEntry {
    pub tick: i32,
    pub attacker: String,
    pub victim: String,
    pub weapon: String,
    pub headshot: bool,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct RoundEnd {
    pub tick: i32,
    pub winner: String,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct ProofSummary {
    pub map: String,
    pub kills: Vec<KillEntry>,
    pub round_ends: Vec<RoundEnd>,
}

pub fn field(ev: &GameEvent, name: &str) -> Option<Variant> {
    ev.fields.iter().find(|f| f.name == name).and_then(|f| f.data.clone())
}

pub fn field_str(ev: &GameEvent, name: &str) -> Option<String> {
    match field(ev, name) {
        Some(Variant::String(s)) => Some(s),
        _ => None,
    }
}

pub fn field_bool(ev: &GameEvent, name: &str) -> Option<bool> {
    match field(ev, name) {
        Some(Variant::Bool(b)) => Some(b),
        _ => None,
    }
}

pub fn field_i32(ev: &GameEvent, name: &str) -> Option<i32> {
    match field(ev, name) {
        Some(Variant::I32(v)) => Some(v),
        Some(Variant::U32(v)) => Some(v as i32),
        _ => None,
    }
}

pub fn winner_label(v: &Variant) -> String {
    match v {
        Variant::U32(2) | Variant::I32(2) => "T".to_string(),
        Variant::U32(3) | Variant::I32(3) => "CT".to_string(),
        Variant::String(s) => s.clone(),
        other => format!("{other:?}"),
    }
}

pub fn parse_proof_summary(path: &Path) -> Result<ProofSummary, String> {
    let huf = create_huffman_lookup_table();
    let inputs = ParserInputs {
        real_name_to_og_name: AHashMap::default(),
        wanted_players: vec![],
        wanted_player_props: vec![],
        wanted_other_props: vec![],
        wanted_prop_states: AHashMap::default(),
        wanted_ticks: vec![],
        wanted_events: vec!["player_death".to_string(), "round_end".to_string()],
        parse_ents: true,
        parse_projectiles: false,
        parse_grenades: false,
        only_header: false,
        only_convars: false,
        huffman_lookup_table: &huf,
        order_by_steamid: false,
        list_props: false,
        fallback_bytes: None,
    };
    let mmap = create_mmap(path.to_string_lossy().as_ref().to_string())
        .map_err(|e| format!("mmap failed: {e:?}"))?;
    let mut parser = Parser::new(inputs, ParsingMode::Normal);
    let output = parser.parse_demo(&mmap).map_err(|e| format!("parse failed: {e:?}"))?;

    let map = output
        .header
        .as_ref()
        .and_then(|h| h.get("map_name").cloned())
        .unwrap_or_else(|| "<unknown>".to_string());

    let mut kills = vec![];
    let mut round_ends = vec![];
    for ev in &output.game_events {
        match ev.name.as_str() {
            "player_death" => kills.push(KillEntry {
                tick: ev.tick,
                attacker: field_str(ev, "attacker_name").unwrap_or_else(|| "<world>".into()),
                victim: field_str(ev, "user_name").unwrap_or_else(|| "<unknown>".into()),
                weapon: field_str(ev, "weapon").unwrap_or_else(|| "<unknown>".into()),
                headshot: field_bool(ev, "headshot").unwrap_or(false),
            }),
            "round_end" => round_ends.push(RoundEnd {
                tick: ev.tick,
                winner: field(ev, "winner").map(|v| winner_label(&v)).unwrap_or_else(|| "<unknown>".into()),
            }),
            _ => {}
        }
    }
    Ok(ProofSummary { map, kills, round_ends })
}
```

(`create_mmap`'s exact signature — `String` vs `&Path` — and header key `"map_name"`: verify in vendored source; adjust. Add `serde = { version = "1", features = ["derive"] }` and `serde_json = "1"` to cf-parser deps for the golden output.)

`src/lib.rs` — add `pub mod proof;` below the crate doc.

- [x] **Step 4: Unit tests green**

`cargo test -p cf-parser --manifest-path src-tauri/Cargo.toml` → PASS (4 tests).

- [x] **Step 5: Example binary**

`src-tauri/crates/cf-parser/examples/print_match.rs`:

```rust
//! M0 proof: print kill feed + round scores from a real demo.
//! Usage: cargo run -p cf-parser --release --example print_match -- <demo.dem> [--golden <out.json>]

use std::path::PathBuf;

fn main() {
    let mut args = std::env::args().skip(1);
    let demo = PathBuf::from(args.next().expect("usage: print_match <demo.dem> [--golden out.json]"));
    let golden_out = match (args.next().as_deref(), args.next()) {
        (Some("--golden"), Some(p)) => Some(PathBuf::from(p)),
        _ => None,
    };

    let summary = cf_parser::proof::parse_proof_summary(&demo).expect("parse failed");

    println!("map: {}", summary.map);
    println!("-- kill feed ({} kills) --", summary.kills.len());
    for k in &summary.kills {
        let hs = if k.headshot { " (HS)" } else { "" };
        println!("[tick {:>7}] {} -> {} [{}]{}", k.tick, k.attacker, k.victim, k.weapon, hs);
    }
    println!("-- rounds ({}) --", summary.round_ends.len());
    let (mut ct, mut t) = (0u32, 0u32);
    for (i, r) in summary.round_ends.iter().enumerate() {
        match r.winner.as_str() {
            "CT" => ct += 1,
            "T" => t += 1,
            _ => {}
        }
        println!("round {:>2}: winner {}  (CT {} - {} T)  [tick {}]", i + 1, r.winner, ct, t, r.tick);
    }
    println!("final (by side, no half-swap accounting): CT-side wins {ct} - T-side wins {t}");

    if let Some(p) = golden_out {
        std::fs::write(&p, serde_json::to_string_pretty(&summary).unwrap()).unwrap();
        println!("golden written: {}", p.display());
    }
}
```

Note: side-vs-team accounting (halftime swap) is deliberately out of M0 scope — validation compares total rounds won per *team* by mapping halves manually against the HLTV page.

- [x] **Step 6: Run on the real fixture demo**

```bash
source "$HOME/.cargo/env"
cargo run -p cf-parser --release --example print_match --manifest-path src-tauri/Cargo.toml -- fixtures/public/<the-demo>.dem
```

Expected: real player names in the kill feed, plausible weapons, round count matching a real match (13–24+). **Hand-validate:** total rounds and winner tally consistent with the HLTV page score recorded in Task 5; spot-check 3 kill-feed lines against the HLTV match-page scoreboard (kill totals per player should be plausible). Record validation in `fixtures/goldens/README.md`.

- [x] **Step 7: Commit**

```bash
git add -A && git commit -m "feat(parser): demoparser2 integration + M0 proof (kill feed, round ends)" && git push origin main
```

### Task 7: Golden snapshot + gated integration test

**Files:**
- Create: `fixtures/goldens/<demo-stem>.proof.json` (committed), `src-tauri/crates/cf-parser/tests/golden_proof.rs`
- Modify: `fixtures/goldens/README.md` (validation row filled in)

**Interfaces:**
- Consumes: `parse_proof_summary` (Task 6).
- Produces: committed golden JSON; `cargo test` diff-checks it when the fixture demo exists locally, skips cleanly otherwise (CI never needs a demo, §10.3).

- [x] **Step 1: Generate the golden**

```bash
cargo run -p cf-parser --release --example print_match --manifest-path src-tauri/Cargo.toml -- \
  fixtures/public/<the-demo>.dem --golden fixtures/goldens/<demo-stem>.proof.json
```

If the full JSON is large (>1 MB with all kills), keep it — kills+rounds only is compact (a few hundred KB max).

- [x] **Step 2: Write the gated integration test**

`src-tauri/crates/cf-parser/tests/golden_proof.rs`:

```rust
//! Golden test: parse the fixture demo and diff against the committed snapshot.
//! Skips (passes) when the demo isn't present — CI never needs real demos (PROMPT.md §10.3).

use std::path::Path;

#[test]
fn proof_summary_matches_golden() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
    let demo = root.join("fixtures/public/DEMO_FILE_NAME.dem");
    let golden_path = root.join("fixtures/goldens/DEMO_STEM.proof.json");
    if !demo.exists() {
        eprintln!("fixture demo not present — skipping golden test");
        return;
    }
    let summary = cf_parser::proof::parse_proof_summary(&demo).expect("parse failed");
    let actual = serde_json::to_string_pretty(&summary).unwrap();
    let expected = std::fs::read_to_string(&golden_path).expect("golden missing");
    assert_eq!(actual.trim(), expected.trim(), "parser output diverged from golden");
}
```

(Replace `DEMO_FILE_NAME`/`DEMO_STEM` with the real names. Add `serde_json` to cf-parser `[dev-dependencies]` if not already a dependency.)

- [x] **Step 3: Run tests — golden passes locally**

```bash
cargo test -p cf-parser --manifest-path src-tauri/Cargo.toml
```

Expected: unit tests + golden test PASS (golden runs since demo exists locally).

- [x] **Step 4: Fill the validation row in fixtures/goldens/README.md, commit**

```bash
git add -A && git commit -m "test(parser): golden snapshot of proof summary, gated on fixture presence" && git push origin main
```

### Task 8: Verify, finalize docs, tag m0

**Files:**
- Modify: `CLAUDE.md` (dev commands now real), `docs/PROGRESS.md`, `docs/adr/ADR-0001-demoparser2-viability.md` (status → accepted/proven), `PROMPT.md` (§13 M0 checkbox)

**Interfaces:**
- Consumes: everything above.

- [x] **Step 1: Full local verification (superpowers:verification-before-completion)**

```bash
source "$HOME/.cargo/env"
cargo fmt --manifest-path src-tauri/Cargo.toml --all --check
cargo clippy --manifest-path src-tauri/Cargo.toml --workspace --all-targets -- -D warnings
cargo test --manifest-path src-tauri/Cargo.toml --workspace
pnpm typecheck && pnpm lint && pnpm test:run
pnpm tauri dev   # launch the actual app window once, confirm it opens, close it
gh run list --limit 1   # latest CI run green
```

- [x] **Step 2: Update docs**

- CLAUDE.md dev commands section: setup (`pnpm install`), run (`pnpm tauri dev`), tests (`cargo test --manifest-path src-tauri/Cargo.toml --workspace`, `pnpm test:run`), lint (`cargo clippy … -D warnings`, `pnpm lint`, `pnpm typecheck`), parser proof example command.
- ADR-0001: Status → `accepted (proven: parsed <demo> on 2026-08-19, kill feed + rounds hand-validated vs HLTV)`.
- PROGRESS.md: move M0 to Done, set Now = M1 planning; add gotchas learned.
- PROMPT.md §13: check the M0 box.

- [x] **Step 3: Commit, tag, push**

```bash
git add -A && git commit -m "docs: M0 complete — dev commands, ADR-0001 proven, progress updated"
git tag -a m0 -m "M0: skeleton + parser proof"
git push origin main --tags
```

---

## Self-review notes

- Spec coverage vs §13 M0 DoD: scaffold (T1–2) ✓, CI green (T4) ✓, ADR-0001 (pre-existing + T8 finalize) ✓, demoparser2 pulled + fixture parsed + kill feed & round scores printed and hand-validated (T5–7) ✓, context docs (pre-existing + T8) ✓, owner asked for demos (final report to owner — §14.6, after T8) ✓.
- "Hand-validated against the in-game scoreboard": owner's own demos aren't available yet; §10.1 explicitly allows a public demo for parser work — validation is against the public match page (HLTV) for that demo, recorded in goldens README. Owner-demo validation lands when they supply demos.
- Type consistency: `parse_proof_summary(&Path) -> Result<ProofSummary, String>` used identically in example (T6) and golden test (T7). Helper names `field_str/field_bool/field_i32/winner_label` consistent between tests (T6.2) and impl (T6.3).
- Known uncertainty, flagged not guessed: `create_mmap` arg type, header key `"map_name"`, event field names (`user_name` vs `victim_name`), winner variant type (U32 vs I32/U8) — all verified against vendored source / real output at T6, with explicit instructions.
