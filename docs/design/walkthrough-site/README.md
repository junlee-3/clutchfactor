# Marketing site — sign-off (v1.0.0 site, 2026-08-29)

All four PNGs and the Lighthouse run below were regenerated after the final
fix wave (download-button fallback + short card labels, hero rotation,
scroll offset, chip labels, footer ink).

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
| 375×812: no horizontal scroll, h1 ≥ 40 px, primary button above the fold, its label on one line with the size hint beneath | `375.png` (CDP render) |
| 768×1024 | `768.png` (Brave `--window-size` render) |
| 1440×900: primary button above the fold, nav legible; exactly one accent button in the hero | `1440.png` (Brave `--window-size` render) |
| reduced motion: poster only, all 6 ledger rows visible; download cards read "Download .dmg" / "Apple silicon · 10 MB" (no repeated extension), exactly one accent button in the section | `reduced-motion.png` (Brave `--window-size=1440,7400` render) |
| Lighthouse desktop | `lighthouse.md` — performance 100, accessibility 100, best-practices 100, seo 100, no failing audit (first run: 93/96/100/92 — fixed a render-blocking Google Fonts stylesheet and a missing `robots.txt`; the last accessibility point was `color-contrast` on the footer legal line, fixed in the fix wave) |
| download URLs answer 200 | `.dmg`, `.exe`, `.msi` — curl -sIL, 2026-08-29 |
| `pnpm -C site typecheck && lint && test:run && build` | green, 2026-08-29 (43 tests) |
| CLAUDE.md: `site/` map line (L44), `pnpm -C site dev` (L21), `release.ts` + shots + Honest-limits release step (L58), ≤ 120 lines | 62 lines, 2026-08-29 |
