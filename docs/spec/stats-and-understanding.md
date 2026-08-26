# Stats & understanding — spec for V1.4

> Charter (PROMPT-V1.md §6 "Coaching-first stats", "What your coach watches";
> §7 V1.4): a stats strip on the match header (K/D, ADR, HS%, KAST-style
> round contribution, entry attempts/success, trade rate, clutch
> attempts/wins), a per-round scoreboard view, Trends extended with these
> series, a screen listing every detector and what the engine cannot see,
> callouts rendered on the replay map. **DoD: every stat cross-checked
> against raw SQL for one real match; every stat links to its coaching.**
> Not a stats-tracker pivot: a stat exists only because a coaching surface
> explains it.

## 1. Stat definitions (the contract)

Computed in a new pure module `cf-analysis::stats` inside `analyze()` from
`AnalysisContext` — the same event stream the detectors and the play ledger
read, reusing their per-event helpers (`h14_entry::round_entries`,
`h2::{killed_in, committed}`) so a stat and its coaching rule never disagree.
Rounds "played" = rounds where the tracked player is on a roster. Kills on
teammates never count as kills; a teamkill death counts as a death.

| stat | definition | coaching link |
|---|---|---|
| **K / D** | enemy kills / deaths | Report death breakdown (taxonomy classes) |
| **ADR** | Σ health actually removed from enemies / rounds played, 1 dp. CS2's `player_hurt.dmg_health` is uncapped (an AWP headshot logs 446), so each round is replayed with 100 HP per rostered player and every hurt credits `min(dmg_health, hp_left)` to an opposite-side attacker; team and self damage reduce hp but credit nobody | catalog: H16 / D2 utility damage |
| **HS%** | headshot enemy kills / enemy kills, whole percent; `null` with 0 kills | catalog entry "aim outcomes" (what the engine cannot see) |
| **KAST%** | rounds with a Kill, an Assist, Survival, or a Traded death (killer died to anyone within `trade.commit_window_s`) / rounds played | H2 trade rules |
| **Entry** | attempts = rounds whose opening duel (first kill with both sides known, within `entry.opening_window_s` of freeze end — the H14 definition, applied to BOTH sides) involves the tracked player; wins = tracked was the attacker | D4_ENTRY_PROFILE / H14_UNSUPPORTED_ENTRY |
| **Trade rate** | traded deaths / deaths; and trade kills = tracked killed a teammate's killer within the commit window (the ledger's `trade` plays) / opportunities (`trade` + `missed_trade` plays) | H2_FAILED_TRADE / H2_ISOLATED_DEATH |
| **Clutch** | attempts = rounds where the tracked player was at some point the last alive on their side with ≥ 1 enemy alive (state replay from kills, as `round_review` does), recorded as 1vN with N at that moment; wins = those rounds won | RBR verdicts (Won it) |

Per-round, per-player rows (all ten players — the scoreboard): `kills`,
`deaths` (0/1), `assists`, `damage`, `headshots`, `survived`, `traded`
(death traded within the window), `entry` (`"win"` / `"loss"` / `null`),
`side`. The tracked player's match totals are the sum of their rows.

Silence bias: a stat whose inputs are missing (no hurts for an old import,
no `freeze_end_tick`) is `null`, never 0; the UI renders "—".

## 2. Storage & serving

Migration 0010: `match_stats` (typed columns per stat above + `rounds_played`,
`kills`, `deaths`, `assists`, `damage`, `headshots`, `kast_rounds`,
`entry_attempts`, `entry_wins`, `traded_deaths`, `trade_kills`,
`trade_opportunities`, `clutch_attempts`, `clutch_wins`) and
`round_player_stats(match_id, round, steamid, side, kills, deaths, assists,
damage, headshots, survived, traded, entry)`, both written by
`save_analysis` (DELETE+INSERT in the same transaction) and both in
`MATCH_ANALYSIS_TABLES` (re-analyze replaces them). Typed columns so the DoD
cross-check is plain SQL and Trends is one query.

Commands: `get_match_stats(match_id) -> Option<MatchStatsDto>` (null fields
render "—"; pre-V1.4 imports have no row → the header shows the Re-analyze
hint), `get_round_scoreboard(match_id, round) -> Vec<PlayerRoundStatsDto>`
(names resolved), `get_trends` gains `stats: StatSeries[]` (one series per
stat in §1, values aligned with `matches`, `null` where absent),
`get_detector_catalog() -> CatalogDto`, `get_map_callouts(map) ->
Vec<CalloutDto>`.

## 3. Surfaces

**Match header stats strip** (Report + Replay, `MatchHeader`): K/D · ADR ·
HS% · KAST · Entry (wins/attempts) · Trades (traded/deaths) · Clutch
(wins/attempts). Every chip is a link: to the Report's death breakdown, the
relevant insight group when present, or the catalog entry that explains the
stat. Missing row → the existing placeholder slot + "Re-analyze for stats".

**Per-round scoreboard** (Report): selecting a round in the round strip
opens a table of all ten players for that round (side · K · D · A · DMG · HS
· survived/traded · entry), tracked player highlighted; a "Match" tab shows
per-player match totals. Side hues only in the side column. The table is the
existing `DataTable`.

**Trends v2**: a "Your numbers" section — one sparkline per stat series (ink,
last value emphasized, map filter shared with the rule sparklines), each with
a "why it matters" line linking to the catalog entry or rule series. dataviz
rules bind (no green/red for non-outcome data).

**"What your coach watches"** (new route `/watches`, sidebar "Watches"): from
a static catalog in `cf-analysis::catalog` (tested to cover every rule id
`families::all()` emits plus D2/D4/D5/D6 and every taxonomy class): per
detector — family, what it looks for, thresholds in plain language rendered
from the live `DetectorConfig` values (no magic numbers in prose), the
taxonomy class it sources, an example of the wording it produces; a "what the
engine cannot see" section (economy, utility lineups, comms, aim mechanics
beyond outcomes, visibility raycasts against map geometry); "not built yet":
classes 8, 10, 12 with the reason (each needs geometry/LOS the parser does
not provide). Every stat in §1 links here.

**Callouts on the replay map**: per map, the median world position of every
`last_place` value across all imported matches on that map (own + corpus),
cached in `map_callouts(map, place, x, y, samples)` (migration 0010) and
refreshed after any import/re-analyze of that map; rendered by the replay
`Renderer` as small mono labels (prettified via `callout_name`) in `--ink-dim`
when the radar is drawn at ≥ 560 CSS px on screen (ruled down from 600: this Mac's
maximum windowed radar is 596 px), under the player dots; a
"Callouts" toggle in the transport (default on, remembered per session).

## 4. DoD (charter)

- For match 8 (mirage-tie): every §1 stat and one full round's scoreboard
  recomputed by hand from raw `kills`/`hurts`/`rounds`/`round_sides` SQL and
  matched to the stored rows and the rendered strip — recorded in
  `fixtures/goldens/README.md`.
- Every stat chip, sparkline and scoreboard column links to a coaching
  surface (walked in the walkthrough).
- Catalog coverage test green; walkthrough-v1.4 captures graded against
  design-system v2; frontend-design + dataviz skills invoked per screen.
