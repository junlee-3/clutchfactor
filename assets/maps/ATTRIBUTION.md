# Radar & map-preview assets attribution

- Radar images (`*.png`, incl. `_lower` variants) and `map-data.json` calibration
  constants come from the **awpy** project's map artifact bundle
  (https://github.com/pnxenopoulos/awpy, MIT license), artifact build **17595823**
  (CS2 patch 2025-03-04), downloaded from `https://awpycs.com/17595823/maps.zip`.
- Library map-preview images (`previews/*.png`) are CS2 competitive-queue style
  composites: blurred scenic screenshot behind the map badge icon. Both layers
  come from the official game depot via
  [MurkyYT/cs2-map-icons](https://github.com/MurkyYT/cs2-map-icons)
  (`images/thumbs/<map>_png.png` + `images/<map>.png`), center-cropped /
  composited to 256×256. They replace the flat radar tiles on Library rows
  only — the replay viewer still uses the awpy radar PNGs.
- Radar and preview imagery are derived from Counter-Strike 2 game files,
  © Valve Corporation. Used here as community-standard practice in a free,
  local analysis tool; not affiliated with or endorsed by Valve.
- To update radars for a new CS2 patch: check `awpy/data/__init__.py` for the
  current build id, download the new `maps.zip`, diff `map-data.json`, replace
  files, and update the build id here and in ADR-0004. To refresh previews:
  pull `images/thumbs/<map>_png.png` (scene) and `images/<map>.png` (badge)
  from `MurkyYT/cs2-map-icons`, center-crop the scene to square, Gaussian-blur
  + dim it, overlay the badge at ~72% scale, resize to 256×256, write into
  `previews/`.
