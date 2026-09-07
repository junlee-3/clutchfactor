# ADR-0012 — Marketing site: `site/` in this repo, vanilla Vite, Vercel

**Status:** accepted (2026-08-29)

## Context

v1.0.0 shipped with installers on GitHub releases and no public page. CS2
players find tools through links, not repos; the README is not a landing
page. The owner chose the site's visual direction from three mocks
(`docs/spec/marketing-site.md`): the app's own design system extended to
the web, with the owner's gameplay clips as the hero and the play ledger as
the first explanation.

## Decision

1. **In this repo, under `site/`.** The page shows the app's real
   screenshots and quotes its real output; keeping it next to the product
   means a release PR can update both, and the tokens drift test can read
   the app's `tokens.css` directly. A separate repo would copy screenshots
   by hand every release.
2. **Vanilla TypeScript + CSS on Vite, no React.** The page has four
   behaviours (clip rotation, a timed ledger, OS-detected download button,
   scroll reveals). A framework buys nothing here and costs bundle, a
   hydration step, and a second React version to keep in line with the app.
3. **Standalone package, no pnpm workspace.** `site/` has its own
   `package.json` and lockfile so the Tauri app's install, `tauri build` and
   its three CI jobs are untouched; CI gains one cheap `site` job.
4. **Vercel, root directory `site`, previews per PR.** The Vercel CLI and
   plugin are already installed; previews give the owner a URL to review
   every change. GitHub Pages was the fallback (no previews, slower loop).
5. **Owner's footage only.** No third-party gameplay video is ever
   committed or hot-linked; the hero degrades to a poster, then to a real
   radar image.

## Consequences

- Releases get one more step: update `site/src/release.ts` (version, asset
  names, sizes). It is in CLAUDE.md's checklist.
- Design tokens exist in two files; the vitest drift test fails CI when
  they disagree.
- `vercel login`/`link` are interactive and the owner's to run; until then
  the site builds locally and in CI but is not deployed.
- The site inherits the app's single dark theme; a light theme would be a
  new decision.
