# ClutchFactor

ClutchFactor is a coach, not a stats tracker: import your own CS2 demo
(`.dem`) and it narrates what happened, round by round. Every insight and
every number it shows links to the exact rounds and seconds in a 2D replay,
so you can watch the play instead of taking a stat's word for it.

![Match report](docs/screenshots/report.png)

## What it does

- **Round-by-round coaching** — every round is narrated from a play ledger:
  setup, utility, trades, deaths, rotations, outcome. Each round gets a
  verdict, and when one play decided it, the report names that play.
- **The AI coach** — optional. With a Gemini key set, the coach reads the
  measured facts for a round and writes its own read on what happened, per
  round and per match — a judgment call, not a template fill-in. Every
  number, name and callout it cites is checked against those facts before
  you see it. No key means no network call and no coach — just the
  plain-language templates.
- **Stats that link to their coaching** — K/D, ADR (damage counted as
  health actually removed, not the game's uncapped damage log), HS%, KAST,
  entry attempts and wins, trade rate, clutch attempts and wins. Every stat
  is a chip that opens the rules behind it.
- **What your coach watches** — a dedicated screen listing every detection
  rule in plain language with its live thresholds, which of the 15 death
  classes aren't built yet and why, and what the engine flatly cannot see
  (economy, utility lineups, comms, aim mechanics, line of sight).
- **2D replay** — radar playback of any round at 60 fps: positions, health,
  weapons, kill feed, deaths, callout labels on the map. Deep-linked from
  every insight and every stat.
- **Cross-match habits** — patterns promoted only when they repeat ("Left
  trades on the table in 5 of your last 10 matches"), including repeat death
  hotspots per map, with evidence into each contributing demo.
- **Trends** — per-habit sparklines across your imported matches, the share
  of deaths that were pure aim duels, and streak callouts ("Good news:
  isolated deaths trending down 4 matches straight").
- **Reference corpus** — drop pro demos (freely downloadable from HLTV match
  pages) into the corpus and the app builds positional occupancy heatmaps
  per map/side/round-phase. With 8+ pro demos on a map, it flags spots you
  hold that reference players rarely do. Statistically honest by design:
  the wording is "unusual, not wrong", and below 8 demos it stays silent.

![Replay](docs/screenshots/replay.png)
![Trends](docs/screenshots/trends.png)
![Reference corpus](docs/screenshots/corpus.png)
![Library](docs/screenshots/library.png)

## The analysis, honestly

Deaths are classified into 15 classes by a rule engine (families H1–H16
over parsed demo events: positions, trades, utility, timing), and every
threshold behind them is documented config, not a magic number buried in
code. Three classes (8, 10, 12) are not built: they need peek geometry and
angle-of-exposure data the parser doesn't provide, and the "What your coach
watches" screen says so rather than folding them into a false "fair duel."
Rules bias toward silence: a missed detection is fine, a wrong accusation is
not, and every rule carries a confidence. Deaths the engine can't attribute
stay "Unclassified" and the report says so. Line-of-sight raycasts and
crosshair placement are out of scope for v1 — the engine has positions and
events, not sightlines or aim.

The AI coach can judge and prioritize from its own CS2 knowledge, but it
cannot invent a number: every figure, player name, callout and round it
cites is checked against the measured facts before it's shown, and anything
that doesn't check out is rejected in favor of the template. The only
network call this app ever makes is to Google's Gemini API, and only when
you've set a key.

## Install

Grab **v1.0.0** from the
[releases page](../../releases):

- **Windows** — the `.exe` NSIS installer (or `.msi`). SmartScreen will warn
  because the build is unsigned: "More info → Run anyway".
- **macOS** (Apple silicon) — the `.dmg`. Unsigned: right-click the app →
  Open on first launch.

First run: **Import demo** → pick a `.dem` from your own matches (CS2 →
Watch → Your Matches → Download, or a FACEIT match room). The app
auto-detects which player you are (the most-seen account across your
imports) — override it in Settings if it guesses wrong.

The AI coach is optional. Add a Gemini API key under Settings → Coach; it's
stored in the app's local database and never leaves your machine except in
requests to Google's API when the coach runs.

## Development

Prereqs: Rust stable (rustup), Node 22, pnpm 10. See `CLAUDE.md` for the
full command list.

```sh
pnpm install
pnpm tauri dev                                  # run the app
pnpm typecheck && pnpm lint && pnpm test:run    # frontend checks
cargo test --manifest-path src-tauri/Cargo.toml --workspace
```

Architecture: `cf-parser` (demoparser2 wrapper → normalized match data) →
`cf-analysis` (detectors, play ledger, stats, catalog) → `cf-store`
(SQLite) → Tauri commands (+ the coach) → React/canvas. Real demos live in
`fixtures/` (gitignored — see `fixtures/README.md`). Radar images vendored
from
[awpy](https://github.com/pnxenopoulos/awpy) (see
`assets/maps/ATTRIBUTION.md`).
