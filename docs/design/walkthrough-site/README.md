# Marketing site — sign-off (v1.0.0 site, 2026-08-29)

Spec: `docs/spec/marketing-site.md` §8. Renders from `pnpm -C site preview`
with Brave headless; the hero shows the radar fallback because no clips were
committed yet.

**Render method note:** the brief's literal `--window-size=W,H --screenshot=…`
Brave command produced correct, full-width renders at 768×1024, 1440×900,
and 1440×7400 (reduced motion), but at 375×812 it consistently laid the page
out at a narrower internal width (~500px) and cropped the PNG to the
requested pixel box — nav text ("Do…"), body copy, and a ledger timestamp
("R12 ·") were cut off mid-word. This is the same headless-Brave quirk noted
in `.superpowers/sdd/site-marketing/task-8-report.md`. `375.png` was instead
produced with a small CDP script (`Emulation.setDeviceMetricsOverride` to
force the true viewport, then `Page.captureScreenshot`) driving the same
headless Brave binary against the same preview server — no `chrome-remote-interface`
package was needed, just Node 22's built-in `WebSocket` against the
`/json/new` + `webSocketDebuggerUrl` endpoints. `768.png`, `1440.png`, and
`reduced-motion.png` are from the brief's literal command as-is.

| check | evidence |
|---|---|
| 375×812: no horizontal scroll, h1 ≥ 40 px, primary button above the fold | `375.png` (CDP render) |
| 768×1024 | `768.png` (Brave `--window-size` render) |
| 1440×900: primary button above the fold, nav legible | `1440.png` (Brave `--window-size` render) |
| reduced motion: poster only, all ledger rows visible | `reduced-motion.png` (Brave `--window-size=1440,7400` render) |
| Lighthouse desktop | `lighthouse.md` — performance 100, accessibility 96, best-practices 100, seo 100 (first run: 93/96/100/92; fixed a render-blocking Google Fonts stylesheet and a missing `robots.txt`, see `lighthouse.md`) |
| download URLs answer 200 | `.dmg`, `.exe`, `.msi` — curl -sIL, 2026-08-29 |
| `pnpm -C site typecheck && lint && test:run && build` | green, 2026-08-29 |
