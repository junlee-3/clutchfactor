# Hero clips

The hero loops the owner's own CS2 gameplay. Nothing third-party is ever
committed here.

## Spec (per clip)

- H.264 MP4, no audio track, 1280×720 (or 1920×1080), 24–30 fps
- 8–15 s, ≤ 5 MB each; 3–5 clips total
- Name `clip-01.mp4`, `clip-02.mp4`, … (lowercase, digits, hyphens only)
- `poster.jpg`: the first frame of `clip-01.mp4`, 1920×1080, ≤ 300 KB

## Make one from a recording

```sh
ffmpeg -ss 00:01:23 -t 12 -i recording.mp4 -an -vf "scale=1280:-2" \
  -c:v libx264 -crf 26 -preset slow -movflags +faststart clip-01.mp4
ffmpeg -i clip-01.mp4 -frames:v 1 -q:v 3 poster.jpg
```

## Register it

Add the file to `clips.json`, in play order:

```json
[{ "file": "clip-01.mp4" }, { "file": "clip-02.mp4" }]
```

An empty manifest shows the poster; no poster shows a dimmed Inferno radar.
Files here are cached for a year (`vercel.json`) — rename to change.
