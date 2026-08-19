# Radar assets attribution

- Radar images (`*.png`, incl. `_lower` variants) and `map-data.json` calibration
  constants come from the **awpy** project's map artifact bundle
  (https://github.com/pnxenopoulos/awpy, MIT license), artifact build **17595823**
  (CS2 patch 2025-03-04), downloaded from `https://awpycs.com/17595823/maps.zip`.
- The radar imagery itself is derived from Counter-Strike 2 game files,
  © Valve Corporation. Used here as community-standard practice in a free,
  local analysis tool; not affiliated with or endorsed by Valve.
- To update for a new CS2 patch: check `awpy/data/__init__.py` for the current
  build id, download the new `maps.zip`, diff `map-data.json`, replace files,
  and update the build id here and in ADR-0004.
