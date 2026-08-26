-- V1.4 DoD: every stat cross-checked against the raw tables (spec §4).
-- Rosters live in round_sides(match_id, number, steamid, side); plays live in
-- round_plays.plays_json (one JSON array per round); "enemy" =
-- victim whose side that round differs from the tracked player's.
WITH r AS (
  SELECT r.number, r.start_tick, COALESCE(r.officially_ended_tick, r.end_tick) AS end_tick, r.winner, me.side AS my_side
  FROM rounds r LEFT JOIN round_sides me ON me.match_id = r.match_id AND me.number = r.number AND me.steamid = :sid
  WHERE r.match_id = :mid
),
k AS (
  SELECT k.round, k.tick, k.attacker, k.victim, k.assister, k.headshot, r.my_side, vs.side AS victim_side
  FROM kills k JOIN r ON r.number = k.round
  LEFT JOIN round_sides vs ON vs.match_id = k.match_id AND vs.number = k.round AND vs.steamid = k.victim
  WHERE k.match_id = :mid
),
h AS (
  -- Health replay: dmg_health is uncapped in CS2 (an awp headshot can log 446), so
  -- credit only the health actually removed: min(dmg, 100 - damage already taken
  -- by that victim earlier in the round, from anyone).
  SELECT MIN(MAX(h.dmg_health, 0), MAX(0, 100 - COALESCE(SUM(MAX(h.dmg_health, 0)) OVER (
           PARTITION BY r.number, h.victim ORDER BY h.tick, h.rowid
           ROWS BETWEEN UNBOUNDED PRECEDING AND 1 PRECEDING), 0))) AS dmg_health,
         r.my_side, vs.side AS victim_side, h.attacker, h.victim
  FROM hurts h JOIN r ON h.tick BETWEEN r.start_tick AND r.end_tick
  JOIN round_sides vs ON vs.match_id = h.match_id AND vs.number = r.number AND vs.steamid = h.victim
  WHERE h.match_id = :mid
)
SELECT 'rounds_played' AS stat, COUNT(*) AS raw FROM r WHERE my_side IS NOT NULL
UNION ALL SELECT 'match_stats.rounds_played', rounds_played FROM match_stats WHERE match_id = :mid
UNION ALL SELECT 'kills',     COUNT(*) FROM k WHERE attacker = :sid AND victim != :sid AND victim_side != my_side
UNION ALL SELECT 'match_stats.kills', kills FROM match_stats WHERE match_id = :mid
UNION ALL SELECT 'deaths',    COUNT(*) FROM k WHERE victim = :sid
UNION ALL SELECT 'match_stats.deaths', deaths FROM match_stats WHERE match_id = :mid
UNION ALL SELECT 'assists',   COUNT(*) FROM k WHERE assister = :sid AND victim_side != my_side
UNION ALL SELECT 'match_stats.assists', assists FROM match_stats WHERE match_id = :mid
UNION ALL SELECT 'headshots', COUNT(*) FROM k WHERE attacker = :sid AND victim != :sid AND victim_side != my_side AND headshot = 1
UNION ALL SELECT 'match_stats.headshots', headshots FROM match_stats WHERE match_id = :mid
UNION ALL SELECT 'damage',    COALESCE(SUM(dmg_health), 0) FROM h WHERE attacker = :sid AND victim != :sid AND victim_side != my_side
UNION ALL SELECT 'match_stats.damage', damage FROM match_stats WHERE match_id = :mid
UNION ALL SELECT 'rps.kast (tracked rows)', SUM(CASE WHEN kills > 0 OR assists > 0 OR survived = 1 OR traded = 1 THEN 1 ELSE 0 END) FROM round_player_stats WHERE match_id = :mid AND steamid = :sid
UNION ALL SELECT 'match_stats.kast_rounds', kast_rounds FROM match_stats WHERE match_id = :mid
UNION ALL SELECT 'rps.entry attempts', COUNT(*) FROM round_player_stats WHERE match_id = :mid AND steamid = :sid AND entry IS NOT NULL
UNION ALL SELECT 'match_stats.entry_attempts', entry_attempts FROM match_stats WHERE match_id = :mid
UNION ALL SELECT 'rps.traded', COUNT(*) FROM round_player_stats WHERE match_id = :mid AND steamid = :sid AND traded = 1
UNION ALL SELECT 'match_stats.traded_deaths', traded_deaths FROM match_stats WHERE match_id = :mid
UNION ALL SELECT 'ledger trade plays', COUNT(*) FROM round_plays rp, json_each(rp.plays_json) p WHERE rp.match_id = :mid AND json_extract(p.value, '$.kind') = 'trade'
UNION ALL SELECT 'match_stats.trade_kills', trade_kills FROM match_stats WHERE match_id = :mid
UNION ALL SELECT 'ledger trade + missed_trade', COUNT(*) FROM round_plays rp, json_each(rp.plays_json) p WHERE rp.match_id = :mid AND json_extract(p.value, '$.kind') IN ('trade', 'missed_trade')
UNION ALL SELECT 'match_stats.trade_opportunities', trade_opportunities FROM match_stats WHERE match_id = :mid;
