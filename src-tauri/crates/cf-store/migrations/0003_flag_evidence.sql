-- Migration 3 (M4): rule flags carry their evidence so cross-demo habit
-- reports can deep-link into the replay without rebuilding windows.
-- Old rows stay NULL → habit evidence falls back to (round, tick ± 5 s).

ALTER TABLE rule_flags ADD COLUMN evidence_json TEXT;
