# ClutchFactor Design System — "The Film Room"

**Status: v1 reference (V1.1, 2026-08-22).** This is the document SDD reviewers
check screens against (charter Pillar 3). Every visual decision in the app
derives from here; a screen that needs a value this file doesn't define is a
reason to extend this file, not to invent a one-off.

---

## 1. Position

The field (verified live, 2026-08: FACEIT, Leetify, csstats, Scope, Refrag)
shares one look: cool navy/black canvas, one loud signature hue (orange /
pink / blue / lime), condensed poster sans, HLTV-scoreboard density, ring and
donut gauges, green/red deltas on everything, zero written sentences.

ClutchFactor is **the coach's studio, not a stats casino**. The identity is
built from the product's own world — the film room where the tape is playing
and the coach draws on it:

1. **The room is warm and dark; the tape is the light source.** Warm graphite
   chrome (no navy anywhere), radar/replay panels sit slightly darker and
   cooler so the map reads as the lit screen in a dark room.
2. **The only saturated color on screen is the game itself.** CT blue, T
   amber, win green, loss red — all four carry game meaning. The brand has no
   hue; the "accent" is chalk (warm light). If something glows a color, it's
   the game talking, never the app decorating.
3. **The coach speaks in sentences; the instrument speaks in numbers.**
   Editorial serif display for the coach's voice, mono for every numeral.
   No ring gauges, no donuts, no rainbow. Data-ink restraint per the dataviz
   rules.
4. **Dashed means evidence.** The chalk annotation — see §5. Structure is
   information: a dashed stroke is always a claim you can click through to
   watch; a solid hairline is just furniture.
5. **Premium = what we leave out.** One accent system, one table style, one
   card surface, four transition durations. When in doubt, remove.

## 2. Color tokens

All UI color comes from these custom properties (`src/styles/tokens.css`).
Canvas/TS code reads the same values via `src/lib/theme.ts` (which resolves
the CSS variables at runtime) — no second color system.

```css
:root {
  color-scheme: dark; /* native controls render dark */

  /* Neutrals — warm graphite ("the studio"). No blue cast anywhere. */
  --bg0:   #131110;   /* app canvas */
  --bg1:   #1a1715;   /* card / row surface */
  --bg2:   #221e1a;   /* hover / inset / track */
  --bg-tape: #0d0c0b; /* radar & heatmap wells — darker, lets the map glow */
  --line:  #2a251f;   /* hairline borders, dividers */
  --line-strong: #3a332a; /* emphasized borders (active card edge) */

  /* Chalk — ink AND interaction. Brightness is the accent, not hue. */
  --chalk:      #eae4d6;  /* primary ink */
  --chalk-bright: #f7f2e6; /* interactive/hover ink, focus ring, active nav */
  --chalk-dim:  #a89f90;  /* secondary ink */
  --chalk-faint:#6e675c;  /* tertiary: timestamps, placeholders, disabled */

  /* Game hues — reserved. Never used for chrome, brand, or emphasis. */
  --ct:   #4aa3ff;    /* CT side identity only */
  --t:    #f5b83d;    /* T side identity only */
  --win:  #5dbb7a;    /* round/match won, good-news class 13 */
  --loss: #d16a5f;    /* round/match lost, severity, errors */
  --tie:  #a89f90;    /* = chalk-dim */

  /* Derived, defined once (no ad-hoc color-mix in component CSS) */
  --surface-win:  color-mix(in srgb, var(--win) 12%, var(--bg1));
  --surface-loss: color-mix(in srgb, var(--loss) 10%, var(--bg1));
  --border-win:   color-mix(in srgb, var(--win) 35%, var(--line));
  --border-loss:  color-mix(in srgb, var(--loss) 35%, var(--line));
}
```

Rules:
- **`--ct` is no longer the app accent.** Buttons, focus, progress, links,
  active states all use chalk. `--ct`/`--t` appear only where the thing IS a
  side: dots, rosters, side chips, round-winner marks, kill feed names.
- Severity encodes via `--loss`-mixed spines/edges (existing `--sev` pattern),
  never via new hues.
- Charts use chalk for the line/mark by default; side-split series may use
  CT/T. Never green/red pairs for non-outcome data. (Full rules: invoke the
  `dataviz` skill per chart — binding, per charter.)

## 3. Type

Three faces, three jobs, all SIL OFL, **bundled locally** in `assets/fonts/`
(desktop app, no CDN; license files vendored beside the woff2s — ADR-0007):

| Role | Face | Usage |
|---|---|---|
| **Display / coach's voice** | **Fraunces** (variable, incl. italic) | Screen titles, match-header map name, coach-note pull quotes, big verdict words, empty-state titles, the wordmark. Used with restraint — if a screen has more than two Fraunces moments, it has too many. |
| **UI / body** | **Inter** (variable) | Everything conversational: body text, buttons, nav, form labels, card prose. |
| **Data / instrument** | **JetBrains Mono** | Every numeral, timestamp, stat, table cell, kill feed row, callout label, micro-caps eyebrow. Tabular numerals on. |

```css
--font-display: "Fraunces", ui-serif, Georgia, serif;
--font-sans:    "Inter", -apple-system, "Segoe UI", system-ui, sans-serif;
--font-mono:    "JetBrains Mono", ui-monospace, "SF Mono", Menlo, monospace;
```

Scale (tokens, px @ base 14 — keep the whole app on these eight):

```css
--text-display: 26px/1.15 var(--font-display);   /* screen title; weight 560, optical size high */
--text-stat:    24px/1.2  var(--font-mono);      /* headline stat (display-data role) */
--text-title:   19px/1.2  var(--font-display);   /* card/section feature moments; weight 540 */
--text-heading: 15px/1.35 var(--font-sans);      /* section heads; weight 600 */
--text-body:    14px/1.55 var(--font-sans);      /* prose */
--text-ui:      13px/1.4  var(--font-sans);      /* buttons, nav, inputs */
--text-data:    12.5px/1.4 var(--font-mono);     /* stats, tables, chips */
--text-micro:   10.5px/1.3 var(--font-mono);     /* eyebrows: uppercase, letter-spacing .14em, chalk-dim */
```

The micro eyebrow (mono, tracked caps, chalk-dim) is the ONE label style —
the current four near-duplicates collapse into it.

## 4. Space, radius, elevation, motion

```css
/* 4px base — the only spacing values allowed */
--s1: 4px; --s2: 8px; --s3: 12px; --s4: 16px; --s5: 20px;
--s6: 24px; --s7: 32px; --s8: 48px;

--r-sm: 4px;  /* chips, pips, inputs */
--r-md: 8px;  /* cards, buttons, rows — THE surface radius */
--r-lg: 12px; /* modals/toasts, the radar well */
--r-full: 999px;

/* Elevation: borders first, shadow only where something floats */
--shadow-float: 0 8px 24px rgb(0 0 0 / 0.45); /* toasts, menus */

--dur-fast: 120ms; --dur: 200ms; --ease: cubic-bezier(0.2, 0, 0, 1);
```

Motion: hover/active transitions at `--dur-fast`; reveals at `--dur`; nothing
animates position except the playhead and progress fills; every transition
respects `prefers-reduced-motion` (existing pattern, kept).

## 5. The signature: chalk annotation

Today the system's one memorable device lives in exactly one place: **evidence
chips get a dashed underline** (chalk, 4-3 dash). V1.2 (issue #9) extends the
same grammar onto two more surfaces — a **dashed line from the tracked player
to the nearest teammate with the distance labelled**, drawn on the replay
canvas, and a **dashed left stripe marking the active moment in the RBR
rail**. The grammar is one system, phased across surfaces as it ships:

- **Dashed stroke (chalk, 4-3 dash) = evidence.** Evidence chips get a dashed
  underline today. V1.2 adds the replay's teammate-distance line (dashed
  chalk with a mono distance tag) and the RBR rail's active-moment left
  stripe. Everything dashed is interactive and jumps to the tape. Nothing
  decorative may be dashed.
- **Solid hairline (`--line`) = furniture.** Dividers, card edges.
- Hover on a dashed element brightens it to `--chalk-bright` — the coach
  presses the chalk down.

## 6. Components (`src/components/ui/`)

One canonical implementation each; screens compose these, never re-declare
surfaces. (Existing bespoke variants — 6 button styles, 8 hand-copied card
surfaces, 3 focus treatments, 6 badge one-offs — all collapse into these.)

- **Button** — variants `primary` (chalk fill, bg0 ink), `secondary` (bg2 +
  line border), `ghost` (borderless), `danger` (loss border/ink, two-step
  confirm pattern stays); sizes `md`/`sm`. Chalk-bright focus ring, 2px offset
  — the app's ONE focus treatment, on every interactive element.
- **Card** — `--bg1`, `--line`, `--r-md`, padding `--s4`; optional eyebrow
  slot; optional `edge` prop (win/loss/severity left edge, 2px).
- **Chip / Badge** — mono `--text-data`, `--r-sm`; variants: default,
  evidence (dashed underline — see §5), side-ct/side-t, count.
- **Table** — the one table style (mono data cells, sans header eyebrows,
  hairline rows); replaces `.grid-table` + match-list grid.
- **Input / Select** — dark, `--r-sm`, chalk caret, chalk-bright focus ring.
- **Tabs / Segmented** — segmented control (replaces speed buttons, side
  chips, phase chips, map chips: one component, four call sites).
- **EmptyState** — Fraunces title + body + one action button ("an invitation
  with a next action"). **Skeleton** — shimmer-free (motion restraint):
  static `--bg2` blocks at final layout size; every screen's loading state
  uses skeletons, never a bare sentence. **Toast** — bottom-right,
  `--shadow-float`, auto-dismiss, `role="status"`/`alert`.
- **Sidebar** (§7), **MatchHeader** (§8), **ImportQueuePanel** (extracted
  from the duplicated Library/Corpus block).

## 7. Shell & navigation

One shell for every screen (`<AppShell>`): fixed left sidebar + content.

- **Sidebar, 216px:** wordmark (Fraunces, type-title) → nav (Library, Trends,
  Corpus, Settings — text-first, no icon library; active item chalk-bright
  with a 2px chalk left edge) → footer: tracked-player chip (mono).
- **Immersive screens (Replay, Report):** the sidebar collapses to a 56px
  rail (wordmark glyph "CF" + initials nav) so the tape stays hero; the
  MatchHeader carries context. Everything else about the shell is identical —
  the two-shell split dies.
- **Content:** editorial screens center at `max-width: 960px`, padding
  `--s7`; immersive screens go full-bleed with `--s4` gutters.
- **Window:** min 1200×760 (charter floor), default 1440×900, both set in
  `tauri.conf.json`. At exactly 1200px wide every screen must hold —
  the replay side panel keeps `min-width: 250px` and the radar shrinks first.
- `index.html`: title "ClutchFactor", real favicon (app icon), remove
  scaffold leftovers.

## 8. MatchHeader

Reused by Report and Replay (charter-mandated component):

```
[Mirage]  (Fraunces display)     13 : 7  WON   ·  2026-08-18   [stats strip: K-D 18-14 · HS 52%]
```

Map name in Fraunces; score + result mark (win/loss/tie in game hues); date
mono; stats strip mono (V1.1 shows K-D/HS% from existing data; V1.4 extends
with ADR/KAST etc. without redesign). Back-navigation lives here ("← Library")
plus the Report↔Replay cross-link.

## 9. Screen application notes (what "rebuilt" means per screen)

Every screen: shell + tokens only (no raw px/hex outside tokens.css), skeleton
loading, §7-voice errors via Toast or inline, focus visible, holds at 1200×760.
**Process rule (charter): invoke `frontend-design:frontend-design` with a
screen-specific brief AND `dataviz` (for any chart) before writing each
screen's code; screenshot + self-critique against this doc before done.**

- **Library** — the shelf of tapes. Match rows become Table rows with a 2px
  win/loss edge; map name gets a small Fraunces moment; import queue uses
  ImportQueuePanel + Toast for completion.
- **Report** — the coach's write-up. Coach-note becomes the editorial lead
  (Fraunces pull-quote); insight cards on Card with severity edge + evidence
  chips (dashed); class breakdown keeps single-hue bars (dataviz pass).
- **Replay** — the tape. Radar well on `--bg-tape` with `--r-lg`; transport
  bar tokens; roster/kill feed on Card; canvas colors move to `theme.ts`
  (Renderer/Heatmap drift dies).
- **Trends** — the instrument panel. Chart line becomes chalk (not CT blue);
  axes/labels per dataviz skill; ribbon/sparks on tokens.
- **Corpus** — the archive. Gate meter cells chalk-filled; heatmap stays
  single-hue (CT blue is correct there: corpus grids are per-side data).
- **Settings** — the desk drawer. Cards, one Input, Table for thresholds.

## 10. Accessibility & quality floor

WCAG AA contrast on every token pair used (chalk on bg0 ≈ 12.9:1, chalk-dim
on bg1 ≥ 4.6:1 — verify pairs when finalizing values; chalk-faint is decorative
only, never for essential text). One focus treatment everywhere. Keyboard:
existing replay bindings stay; sidebar and all controls tabbable. No layout
shift on data arrival (skeletons at final size). Dark only, done impeccably
(`color-scheme: dark`).

## 11. File architecture

```
src/styles/tokens.css      the contract (this doc, §2-§5, as code)
src/styles/base.css        reset, html/body, typography classes, focus
src/styles/components.css  ui/ component styles
src/styles/screens.css     screen-specific composition (thin)
src/lib/theme.ts           CSS-var reader for canvas/TS consumers
src/components/ui/*        Button, Card, Chip, Table, Input, Segmented,
                           EmptyState, Skeleton, Toast, Sidebar, AppShell,
                           MatchHeader, ImportQueuePanel
assets/fonts/*             Fraunces, Inter, JetBrains Mono (woff2 + OFL texts)
```

`styles.css` is deleted at the end of the milestone; nothing may import it.
