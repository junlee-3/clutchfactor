# Radar & map-preview assets attribution

- Radar images (`*.png`, incl. `_lower` variants) and `map-data.json` calibration
  constants come from the **awpy** project's map artifact bundle
  (https://github.com/pnxenopoulos/awpy, MIT license), artifact build **17595823**
  (CS2 patch 2025-03-04), downloaded from `https://awpycs.com/17595823/maps.zip`.
- Library map-preview images (`previews/*.png`) are the CS2 competitive-queue /
  map-selection scenic screenshots (panorama `map_icons/screenshots`), extracted
  from the official game depot and vendored from
  [MurkyYT/cs2-map-icons](https://github.com/MurkyYT/cs2-map-icons)
  (`images/thumbs/<map>_png.png`, center-cropped to 256×256). They replace the
  flat radar tiles on Library rows only — the replay viewer still uses the awpy
  radar PNGs.
- Radar and preview imagery are derived from Counter-Strike 2 game files,
  © Valve Corporation. Used here as community-standard practice in a free,
  local analysis tool; not affiliated with or endorsed by Valve.
- To update radars for a new CS2 patch: check `awpy/data/__init__.py` for the
  current build id, download the new `maps.zip`, diff `map-data.json`, replace
  files, and update the build id here and in ADR-0004. To refresh previews:
  pull `images/thumbs/<map>_png.png` from `MurkyYT/cs2-map-icons`, center-crop
  to square, resize to 256×256, and write into `previews/`.
