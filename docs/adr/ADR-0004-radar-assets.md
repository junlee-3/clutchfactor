# ADR-0004: Radar asset sourcing & licensing

Status: accepted
Date: 2026-08-19

## Context

The replay viewer needs per-map radar images + world→image calibration (PROMPT.md §6.3).
Candidate sources: awpy's maintained artifact bundle, SimpleRadar community pack
(unclear redistribution terms), or extracting from local game VPKs (impossible on macOS, §5A spec §5.5).

## Decision

Vendor the **awpy maps artifact** (build 17595823) into `assets/maps/`: 1024×1024 radar
PNGs (incl. `_lower` variants for nuke/train/vertigo/baggage) + `map-data.json`
(`pos_x`, `pos_y`, `scale`, `lower_level_max_units` per map). 2.4 MB total — committed
to git. Attribution in `assets/maps/ATTRIBUTION.md`. Mapping formula (verified):
`img_x = (world_x - pos_x)/scale`, `img_y = (pos_y - world_y)/scale`; lower layer when
`z < lower_level_max_units`.

Serving: Vite `publicDir` is pointed at `assets/`, so images load at `/maps/<map>.png`.
This deviates from a separate `public/` dir (§4 names `assets/maps/` as the location;
one directory serves both purposes).

## Consequences

- Asset source is swappable (§6.3): everything goes through `map-data.json` +
  `radarImageUrl()`; replacing the pack is a file swap.
- Radar imagery is Valve-derived (community-standard usage in free local tools; awpy
  tooling itself is MIT). Risk accepted for a non-commercial local app; revisit if the
  product ever ships commercially.
- Map pool updates require re-vendoring on CS2 patches that change layouts; the update
  procedure is documented in ATTRIBUTION.md. `de_cache` is absent from the current bundle.
