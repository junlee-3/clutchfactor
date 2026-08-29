# ADR-0012 — Clip export: record the canvas, negotiate the container, write to a fixed folder

**Status:** accepted (V1.6, 2026-08-29)

## Context

The 2D replay is the app's evidence surface, and until now evidence could only be looked at inside the app — a moment worth keeping (or sending to a teammate) had no way out. The app had never written a byte to disk either, so "export" needed a path from canvas pixels to a file that doesn't turn into a second rendering pipeline or a general filesystem grant.

## Decision

1. **Record the live canvas in the WebView, in real time at 1×.** `MediaRecorder` over `captureStream(30)` of an offscreen 1024² canvas that mirrors the on-screen one each `requestAnimationFrame`. The replay's rAF loop already runs continuously and `draw(ctx, scene)` is pure, so the clip is the tape as it played — there is no second renderer to keep in sync with `Renderer.ts`, no ffmpeg dependency, and no offline/faster-than-realtime path to maintain. The mirror also fixes the export at 1024² regardless of the display's dpr (the on-screen canvas is up to 2048²) and is where the "Map · Rn · player" caption band is stamped.
2. **The container is negotiated at runtime**, not assumed: `video/mp4;codecs=avc1` → `video/webm;codecs=vp9` → `…vp8` → `video/webm`, first supported wins (`pickMimeType`). WKWebView writes mp4, WebView2 writes webm; the file name's extension follows. When a WebView supports none of them — or has no `MediaRecorder`/`captureStream` at all — the button is disabled with a title that says so, never a failure under the click.
3. **A fixed folder, no save dialog.** Clips land in the platform's Videos directory under `ClutchFactor/` (`~/Movies/ClutchFactor` on macOS), and the toast names the absolute path. One click → one file, like a screenshot: a dialog per clip is a tax on a loop the coach runs repeatedly, and a fixed, named folder is more discoverable than a path the user has to remember choosing. Existing files are never overwritten — `-2`, `-3`, … is appended.
4. **Raw-body IPC, not the fs plugin.** The video crosses as `InvokeBody::Raw` (`callRaw` on the JS side) with the file name in an `x-clip-name` header. Adding `tauri-plugin-fs` would grant the WebView a general write capability for one command's sake; this way the Rust `save_clip` owns the whole contract — it sanitises the name to `[A-Za-z0-9._-]` with an mp4/webm extension, refuses a name carrying a path rather than flattening it, picks the directory, and caps a clip at 200 MB. Base64-in-JSON was the alternative: a third more bytes and another copy of a file that runs to tens of megabytes.

## Consequences

- Windows clips are webm. Editors and browsers read it; converting is out of scope.
- The clip inherits the live playback: a frame the replay drops is a frame the file misses, and a recording costs one extra full-canvas draw per frame while it runs. Playback is locked to 1× and the transport is disabled during a recording so the file is honestly what played.
- No audio, and no round- or match-length export — both would need the offline renderer this deliberately does not build.
- The app now owns a second on-disk location besides its data directory; anything that later needs to list or clean up clips has one folder to look in.
