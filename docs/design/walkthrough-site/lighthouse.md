# Lighthouse — marketing site (2026-08-29)

Run against the local `pnpm -C site preview` server (`http://localhost:4173/`),
desktop preset, headless Brave.

```bash
CHROME_PATH="/Applications/Brave Browser.app/Contents/MacOS/Brave Browser" \
  npx --yes lighthouse@12 http://localhost:4173/ --preset=desktop --quiet --chrome-flags="--headless=new" \
  --output=json --output-path=/tmp/lh.json
node -e 'const r=require("/tmp/lh.json").categories; for (const k in r) console.log(k, Math.round(r[k].score*100))'
```

## Final scores

| category | score |
|---|---|
| performance | 100 |
| accessibility | 96 |
| best-practices | 100 |
| seo | 100 |

All four ≥ 95.

## First run (before fixes)

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

accessibility was 96 on both runs (already ≥ 95, no fix needed).
