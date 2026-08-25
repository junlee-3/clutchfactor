-- Config fingerprint on stored round reviews (V1.2 final-review fix wave,
-- finding #5): a review computed under an old RbrCfg/engine version must be
-- recomputed, not served stale. Empty-string default backfills existing
-- rows (they predate this column) so they always mismatch a nonempty
-- current fingerprint and get recomputed on next read.
ALTER TABLE round_review ADD COLUMN cfg_fingerprint TEXT NOT NULL DEFAULT '';
