-- V1.2b play ledger (docs/spec/play-ledger-and-coach.md §2): one row per
-- round with the tracked player's plays and the all-players timeline, both
-- STRUCTURED JSON (facts as data, raw callouts) — narrated at serve time in
-- cf-narrator::plays, so V1.3's grounding contract feeds on facts.
CREATE TABLE round_plays (
    match_id      INTEGER NOT NULL REFERENCES matches(id) ON DELETE CASCADE,
    round         INTEGER NOT NULL,
    plays_json    TEXT NOT NULL,
    timeline_json TEXT NOT NULL,
    PRIMARY KEY (match_id, round)
) WITHOUT ROWID;

-- Where the demo was imported from, so `re_analyze_match` can re-parse it
-- without asking. NULL for pre-V1.2b imports (the command then asks for
-- the file and verifies its hash against file_hash).
ALTER TABLE matches ADD COLUMN source_path TEXT;
