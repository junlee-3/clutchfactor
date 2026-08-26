-- V1.4 fix wave: callout labels carry the median z of their samples, so the
-- replay renderer can draw each label on the radar layer it belongs to
-- (nuke's B site sits directly under A site in x/y — without z, "B site" is
-- labelled on the upper radar, on top of A). Rows written before this
-- migration default to 0, which is above every map's lower_level_max_units
-- and therefore the upper layer: exactly where they were already drawn.
ALTER TABLE map_callouts ADD COLUMN z REAL NOT NULL DEFAULT 0;
