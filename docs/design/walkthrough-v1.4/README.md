# V1.4 walkthrough — Stats & understanding (hand-verification evidence)

Charter DoD (PROMPT-V1.md §7 V1.4): every stat cross-checked against raw SQL for one real match; every stat links to its coaching. Verified 2026-08-26 on the dev database, match id 8 (de_mirage, tracked `misosoupy3` = `76561199228328773`), re-analyzed through the app (Library → Re-analyze) so `match_stats` / `round_player_stats` / `map_callouts` were written by the V1.4 engine.

## 1. SQL cross-check (spec §4)

`scripts/stats-crosscheck.sql` recomputes every §1 stat from the raw tables (`rounds`, `round_sides`, `kills`, `hurts`, `round_plays.plays_json`) and prints it beside the stored `match_stats` value:

```sh
sqlite3 -cmd ".param set :mid 8" -cmd ".param set :sid '76561199228328773'" -header -column \
  "$HOME/Library/Application Support/com.clutchfactor.app/clutchfactor.db" < scripts/stats-crosscheck.sql
```

Output (verbatim):

```
stat                             raw
-------------------------------  ---
rounds_played                    24 
match_stats.rounds_played        24 
kills                            7  
match_stats.kills                7  
deaths                           19 
match_stats.deaths               19 
assists                          5  
match_stats.assists              5  
headshots                        0  
match_stats.headshots            0  
damage                           923
match_stats.damage               923
rps.kast (tracked rows)          14 
match_stats.kast_rounds          14 
rps.entry attempts               2  
match_stats.entry_attempts       2  
rps.traded                       2  
match_stats.traded_deaths        2  
ledger trade plays               1  
match_stats.trade_kills          1  
ledger trade + missed_trade      11 
match_stats.trade_opportunities  11
```

All eleven raw/stored pairs agree. Damage is the health actually removed: CS2's `player_hurt.dmg_health` is uncapped (an AWP headshot logs 446), so both the engine and the script replay each round's hurts in tick order with 100 HP per player and credit `min(dmg, hp_left)` — 923 for the tracked player (the raw column sums to 967; the whole-branch review caught the overcount on the scoreboard, where one opponent's ADR read 170.6 before the cap and 108.1 after). The raw `kills` table holds 8 kills by the tracked player; the eighth is a teamkill (victim on the same side) and is excluded by the contract, so `kills = 7`. The 0 headshots are real: the match-wide headshot share is 41.5 %, so the field is populated — the tracked player simply had none this match.

## 2. Hand-verified rounds (independent replay over the raw tables)

`docs/design/walkthrough-v1.4/handverify-match8.txt` is the output of a standalone Python replay (rosters from `round_sides`, kills in tick order) written without reference to the engine:

- **Entry (2 attempts, 0 wins):** R7 — first kill of the round at tick 40511, 795 ticks (12.4 s) after freeze end, tracked player is the victim; R20 — tick 120688, 624 ticks (9.8 s) after freeze end, victim. Both inside `entry.opening_window_s` (15 s), both opposite-side duels. Rows carry `entry = 'loss'`; the killers' rows carry `'win'`.
- **Traded deaths (2 of 19):** R13 — killed at tick 79440, killer died at 79542 (+102 ticks = 1.6 s); R21 — killed at 127465, killer died at 127552 (+87 ticks = 1.4 s). Both inside `trade.commit_window_s` (2 s = 128 ticks).
- **Clutch (3 attempts, 0 wins):** R15 1v3, R18 1v4, R23 1v4 — tracked player last alive on T with enemies alive; all three rounds won by CT.
- **KAST 14/24:** the per-round rows' `kills>0 OR assists>0 OR survived OR traded` count equals `kast_rounds`.
- **Trades:** 1 `trade` play and 10 `missed_trade` plays in the ledger → `trade_kills 1 / trade_opportunities 11`.

## 3. Every stat links to its coaching

Each chip in the match-header strip is a link to `/watches?stat=<key>`; the Watches screen filters the catalog to the rules whose `stat_links` name that key. Click-through confirmed for all seven keys (K/D → the death rules; ADR → D2/H3/H6/H16 utility rules; HS% → no rule — the screen says so and points at "Aim mechanics" under what the engine cannot see; KAST → H2 isolated/baited, H14 unsupported entry, H11 early death; Entry → D4/D5/H6 push/H11/H14; Trades → H2; Clutch → D5/H11 slow rotation). Trends' "Your numbers" titles and the Watches sidebar entry reach the same screen.

## 4. Screenshots

| file | what it shows |
|---|---|
| `01-stats-strip.png` | Report header for match 8 with the seven outlined chips (captured before the damage cap landed, so it reads ADR 40.3; the stored row now says 923 damage → 38.5, per §1 — K/D 0.37 · HS% 0% · KAST 58% · Entry 0/2 · Trades 2/19 · Clutch 0/3 are unchanged) |
| `02-scoreboard.png` | Report, round 8 selected in the strip, per-round scoreboard grouped CT/T with the tracked row's tone edge |
| `02b-scoreboard-match.png` | The same scoreboard's Match tab (client-side aggregate: ADR, HS%, KAST, entries, traded) — also pre-cap; the top opponent's ADR reads 170.6 here and is 108.1 after the re-analyze |
| `03-trends-numbers.png` | Trends "Your numbers" — one real point per series with its "why it matters" line (captured with one re-analyzed match and before the why-lines moved to sentence case; all five own matches now carry stats) |
| `04-watches.png` | What your coach watches — families, live thresholds, class chips |
| `05-watches-kast.png` | The same screen filtered by `?stat=kast` — the rules behind KAST |
| `06-replay-callouts.png` | Replay of match 8 with callout labels from `map_callouts` (23 de_mirage places) |
| `06b-replay-callouts-off.png` | The Callouts toggle off |
| `07-replay-callouts-nuke.png` | Nuke (match 2) with the lower layer shown — its labels (Observation, B site, Decon, Tunnels, Secret) carry z and draw on their own layer; the map's callouts were filled lazily on first open |
| `07b-replay-callouts-nuke-upper.png` | The same match on the upper layer — none of the lower-level names appear |

## 5. Not verified here / known limits

- Three screenshots (01, 02b, 03) predate the damage cap / sentence-case commits; a re-capture attempt landed while the owner was using the Mac and was discarded — re-take them from a quiet desktop (Library → Mirage → header; Match tab; Trends).
- Callout labels draw only when the radar is ≥ 560 CSS px wide (this Mac's maximum windowed radar is 596 px).
- Settings' threshold table now lists config-path names with blank units on count rows.
