-- Migration 4 (M5): corpus demos share the matches table but must stay
-- invisible to tracked-player analytics (library, habits, identity).

ALTER TABLE matches ADD COLUMN kind TEXT NOT NULL DEFAULT 'own';
