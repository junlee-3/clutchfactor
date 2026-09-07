# Lighthouse — marketing site (2026-08-29, re-run after the fix wave)

Run against the local `pnpm -C site preview` server (`http://localhost:4173/`),
desktop preset, headless Brave.

```bash
CHROME_PATH="/Applications/Brave Browser.app/Contents/MacOS/Brave Browser" \
  npx --yes lighthouse@12 http://localhost:4173/ --preset=desktop --quiet --chrome-flags="--headless=new" \
  --output=json --output-path=/tmp/lh.json
node -e 'const r=require("/tmp/lh.json").categories; for (const k in r) console.log(k, Math.round(r[k].score*100))'
```

## Final scores (re-run after the fix wave, 2026-08-29)

| category | score |
|---|---|
| performance | 100 |
| accessibility | 100 |
| best-practices | 100 |
| seo | 100 |

All four 100; no failing audit in accessibility, best-practices or seo.

Accessibility moved 96 → 100 in this run. Confirmed by putting the old colour
back in the built CSS and re-running `--only-categories=accessibility`: 96,
one failing audit, `color-contrast` on
`body > footer.footer > div.container > p.footer__legal` — *"insufficient
color contrast of 3.24 (foreground #5c6672, background #0e1116, font size
8.3pt (11px)). Expected contrast ratio of 4.5:1"*. That line used
`--ink-faint`; the fix wave moved it to `--ink-dim` (`#8a94a3`, 6.16:1), per
spec §4 — `--ink-faint` is not for text the reader needs. Nothing else in
`site.css` uses `--ink-faint` now.

Performance stays 100 with several diagnostic-only insights unscored-but-red
(`uses-responsive-images`, `prioritize-lcp-image`, `render-blocking-insight`
and friends) — they are opportunities, not category failures, and the LCP
image is the radar poster that only ships until the clips are recorded (§5).

## First run (before the task-11 fixes)

| category | score |
|---|---|
| performance | 93 |
| accessibility | 96 |
| best-practices | 100 |
| seo | 92 |

Two fixes applied, then rebuilt and re-run:

- **seo (robots-txt is not valid, 332 errors)** — `site/public/robots.txt` did not
  exist, so `vite preview`'s SPA fallback served `index.html` for the
  `/robots.txt` request and Lighthouse tried to parse the HTML as robots
  syntax. Added `site/public/robots.txt` (`User-agent: *` / `Allow: /`).
- **performance (first-contentful-paint 0.69, largest-contentful-paint 0.87,
  render-blocking-resources flagged the Google Fonts stylesheet, ~376 ms
  wasted)** — the `<link rel="stylesheet" href="https://fonts.googleapis.com/...">`
  in `site/index.html` was render-blocking. Switched it to the standard
  non-blocking pattern: `media="print" onload="this.media='all'"` with a
  `<noscript>` fallback for when JS is disabled. `preconnect` hints for
  `fonts.googleapis.com`/`fonts.gstatic.com` were already present.

accessibility was 96 on both of those runs (already ≥ 95, no fix needed
for the DoD); the fix wave later took it to 100, see above.
