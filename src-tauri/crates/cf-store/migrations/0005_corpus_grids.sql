-- Corpus occupancy-grid cache (PROMPT.md §5 D6). One row per (map, side, phase);
-- counts is little-endian u32, row-major [y][x], size*size cells.
CREATE TABLE corpus_grids (
    map      TEXT NOT NULL,
    side     TEXT NOT NULL,    -- 'CT' | 'T'
    phase    TEXT NOT NULL,    -- 'freeze_end' | 'early' | 'mid' | 'post_plant'
    size     INTEGER NOT NULL,
    counts   BLOB NOT NULL,
    demos    INTEGER NOT NULL,
    samples  INTEGER NOT NULL,
    built_at TEXT NOT NULL,
    PRIMARY KEY (map, side, phase)
) WITHOUT ROWID;
