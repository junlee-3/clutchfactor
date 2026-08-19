# fixtures/ — real demo files

Demos (`.dem`, 50–400 MB) are gitignored; only this README and small golden snapshot JSONs derived from them are committed.

## What to put here

- `fixtures/own/` — **2–3 of your own matchmaking or FACEIT demos** (the primary tuning data — detector judgments are calibrated on these).
- `fixtures/pro/` — by M5, a handful of pro demos on one map (for the reference corpus feature).
- `fixtures/public/` — any publicly downloadable match demo (unblocks parser work when own demos aren't available yet).

Filenames: keep them descriptive, e.g. `mirage-2026-08-12-faceit.dem`.

## How to download your own demos

- **Matchmaking / Premier:** in CS2 → Watch → Your Matches → Download. Files land in `…/Steam/steamapps/common/Counter-Strike Global Offensive/game/csgo/replays/` (extension `.dem`, sometimes `.dem.bz2` — decompress first).
- **FACEIT:** match room page → Demo download (`.dem.gz` — decompress first).
- **Pro demos:** HLTV match page → Rewatch/Demo link (GOTV demos, often zipped per map).

## Golden snapshots

`fixtures/goldens/` (committed) holds compact JSON summaries (rounds, scores, kill feed, event counts, spot-checked positions) parsed from the demos above. CI runs against goldens + synthetic scenarios only — it never needs a real demo. Each golden's provenance and hand-validation note lives in `fixtures/goldens/README.md`.
