# ClutchFactor Design System — v2 (the navy studio)

> v2 (2026-08-25, ADR-0009): v0 palette + system fonts on the V1.1 structure. v1 "The Film Room" is history; nothing below refers to it except this line.

**Status: v2 reference (V1.2b, 2026-08-25).** This is the document SDD reviewers
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
built from the product's own world — the tape playing in a dark room, with
the coach drawing on it:

1. **The room is cool navy and dark; the tape is the light source.** Navy
   chrome, radar/replay panels sit darker still (`--bg-tape`) so the map reads
   as the lit screen in a dark room.
2. **The only saturated color on screen is the game itself.** CT blue, T
   amber, win green, loss red — all four carry game meaning. `--accent` is
   the one interaction hue (primary buttons, focus ring, progress fills); it
   is not a game hue and never decorates chrome beyond that job.
3. **The coach speaks in sentences; the instrument speaks in numbers.**
   System sans for prose and titles, mono for every numeral. No ring gauges,
   no donuts, no rainbow. Data-ink restraint per the dataviz rules.
4. **Dashed means evidence.** The ink annotation — see §5. Structure is
   information: a dashed stroke is always a claim you can click through to
   watch; a solid hairline is just furniture; the rail's active row is a
   solid tone edge, not a dash — a state, not evidence.
5. **Premium = what we leave out.** One accent system, one table style, one
   card surface, four transition durations. When in doubt, remove.

## 2. Color tokens

All UI color comes from these custom properties (`src/styles/tokens.css`).
Canvas/TS code reads the same values via `src/lib/theme.ts` (which resolves
the CSS variables at runtime) — no second color system.

```css
:root {
  color-scheme: dark; /* native controls render dark */

  /* Neutrals — cool navy (the v0 palette). */
  --bg0:   #0e1116;   /* app canvas */
  --bg1:   #151a21;   /* card / row surface */
  --bg2:   #1d242e;   /* hover / inset / track */
  --bg-tape: #0b0d11; /* radar & heatmap wells — a touch darker than bg0 */
  --line:  #232b36;   /* hairline borders, dividers */
  --line-strong: #2f3944; /* emphasized borders (active card edge) */

  /* Ink — text. Brightness carries hierarchy; --accent carries interaction. */
  --ink:      #dfe5ec;  /* primary ink */
  --ink-bright: #f2f5f8; /* interactive/hover ink, active nav */
  --ink-dim:  #8a94a3;  /* secondary ink */
  --ink-faint:#5c6672;  /* tertiary: timestamps, placeholders, disabled */

  /* Accent — the app's ONE interaction hue: primary buttons, focus ring,
     progress fills. Shares --ct's value on purpose (v0 did too); a separate
     token so side identity stays explicit in code. */
  --accent:   #4aa3ff;
  --accent-bright: #6db6ff; /* hover on accent fills */

  /* Game hues — reserved. Never used for chrome, brand, or emphasis. */
  --ct:   #4aa3ff;    /* CT side identity only */
  --t:    #f5b83d;    /* T side identity only */
  --win:  #5dbb7a;    /* round/match won, good-news class 13, good plays */
  --loss: #d16a5f;    /* round/match lost, severity, errors, bad plays */
  --tie:  #8a94a3;    /* = ink-dim */

  /* Derived, defined once (no ad-hoc color-mix in component CSS) */
  --surface-win:  color-mix(in srgb, var(--win) 12%, var(--bg1));
  --surface-loss: color-mix(in srgb, var(--loss) 10%, var(--bg1));
  --border-win:   color-mix(in srgb, var(--win) 35%, var(--line));
  --border-loss:  color-mix(in srgb, var(--loss) 35%, var(--line));
}
```

Rules:
- `--accent` is the app accent — primary buttons, focus ring, progress
  fills, nothing else. `--ct` shares its value but appears only where the
  thing IS a side: dots, rosters, side chips, round-winner marks, kill feed
  names.
- Ink brightness carries hierarchy (`--ink-bright` for active nav/hover
  text, `--ink` prose, `--ink-dim` secondary, `--ink-faint` tertiary).
- Severity encodes via `--loss`/`--win` edges and outlines (the rail's tone
  edge, verdict chips, Card edges), never via new hues.
- Charts use ink for the line/mark by default; side-split series may use
  CT/T. Never green/red pairs for non-outcome data. (Full rules: invoke the
  `dataviz` skill per chart — binding, per charter.)

## 3. Type

Two stacks, both system-native (nothing bundled — ADR-0009):

| Role | Stack | Usage |
|---|---|---|
| **UI / body / display** | `--font-sans` | Screen titles (weight 600), card titles (600), body, buttons, nav, form labels. `--font-display` is an alias of this stack. |
| **Data / instrument** | `--font-mono` | Every numeral, timestamp, stat, table cell, kill feed row, callout label, micro-caps eyebrow. Tabular numerals on. |

```css
--font-sans:    -apple-system, "Inter", "Segoe UI", system-ui, sans-serif;
--font-mono:    ui-monospace, "SF Mono", "JetBrains Mono", Menlo, Consolas, monospace;
--font-display: var(--font-sans);
```

Scale (tokens, px @ base 14 — keep the whole app on these eight):

```css
--text-display: 26px/1.15 var(--font-display);   /* screen title; weight 600 */
--text-stat:    24px/1.2  var(--font-mono);      /* headline stat (display-data role) */
--text-title:   19px/1.2  var(--font-display);   /* card/section feature moments; weight 600 */
--text-heading: 15px/1.35 var(--font-sans);      /* section heads; weight 600 */
--text-body:    14px/1.55 var(--font-sans);      /* prose */
--text-ui:      13px/1.4  var(--font-sans);      /* buttons, nav, inputs */
--text-data:    12.5px/1.4 var(--font-mono);     /* stats, tables, chips */
--text-micro:   10.5px/1.3 var(--font-mono);     /* eyebrows: uppercase, letter-spacing .14em, ink-dim */
```

The micro eyebrow (mono, tracked caps, ink-dim) is the ONE label style —
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

## 5. The signature: the dashed grammar

- **Dashed stroke (`[4,3]`) = evidence.** Evidence chips get a dashed
  underline; the replay's teammate-distance line is dashed ink with a mono
  distance tag. Everything dashed is interactive and jumps to the tape.
  Nothing decorative may be dashed.
- **Solid hairline (`--line`) = furniture.** Dividers, card edges.
- **The rail's active row = a solid 2px tone edge** (`--loss` bad / `--win`
  good / `--ink-dim` neutral) — a state, not evidence. It is never dashed;
  dashing is reserved for evidence you can click through to watch.
- Hover on a dashed element brightens it to `--ink-bright` — the coach
  presses the line down.

## 6. Components (`src/components/ui/`)

One canonical implementation each; screens compose these, never re-declare
surfaces. (Existing bespoke variants — 6 button styles, 8 hand-copied card
surfaces, 3 focus treatments, 6 badge one-offs — all collapse into these.)

- **Button** — variants `primary` (accent fill, bg0 ink), `secondary` (bg2 +
  line border), `ghost` (borderless), `danger` (loss border/ink, two-step
  confirm pattern stays); sizes `md`/`sm`. Accent-bright focus ring, 2px
  offset — the app's ONE focus treatment, on every interactive element.
- **Card** — `--bg1`, `--line`, `--r-md`, padding `--s4`; optional eyebrow
  slot; optional `edge` prop (win/loss/severity left edge, 2px).
- **Chip / Badge** — mono `--text-data`, `--r-sm`; variants: default,
  evidence (dashed underline — see §5), outlined (verdict chips), side-ct/
  side-t, count.
- **Table** — the one table style (mono data cells, sans header eyebrows,
  hairline rows); replaces `.grid-table` + match-list grid.
- **Input / Select** — dark, `--r-sm`, ink caret, accent-bright focus ring.
- **Tabs / Segmented** — segmented control (replaces speed buttons, side
  chips, phase chips, map chips: one component, four call sites).
- **EmptyState** — display title + body + one action button ("an invitation
  with a next action"). **Skeleton** — shimmer-free (motion restraint):
  static `--bg2` blocks at final layout size; every screen's loading state
  uses skeletons, never a bare sentence. **Toast** — bottom-right,
  `--shadow-float`, auto-dismiss, `role="status"`/`alert`.
- **Sidebar** (§7), **MatchHeader** (§8), **ImportQueuePanel** (extracted
  from the duplicated Library/Corpus block).

## 7. Shell & navigation

One shell for every screen (`<AppShell>`): fixed left sidebar + content.

- **Sidebar, 216px:** wordmark (display-sans, type-title) → nav (Library,
  Trends, Corpus, Settings — text-first, no icon library; active item
  ink-bright with a 2px accent left edge) → footer: tracked-player chip
  (mono).
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
[Mirage]  (display-sans)     13 : 7  WON   ·  2026-08-18   [stats strip: K-D 18-14 · HS 52%]
```

Map name in display-sans; score + result mark (win/loss/tie in game hues);
date mono; stats strip mono (V1.1 shows K-D/HS% from existing data; V1.4
extends with ADR/KAST etc. without redesign). Back-navigation lives here
("← Library") plus the Report↔Replay cross-link.

## 9. Screen application notes (what "rebuilt" means per screen)

Every screen: shell + tokens only (no raw px/hex outside tokens.css), skeleton
loading, §7-voice errors via Toast or inline, focus visible, holds at 1200×760.
**Process rule (charter): invoke `frontend-design:frontend-design` with a
screen-specific brief AND `dataviz` (for any chart) before writing each
screen's code; screenshot + self-critique against this doc before done.**

- **Library** — the shelf of tapes. Match rows become Table rows with a 2px
  win/loss edge and a radar map thumbnail; import queue uses
  ImportQueuePanel + Toast for completion.
- **Report** — the coach's write-up. Coach-note becomes the editorial lead;
  insight cards on Card with severity edge + evidence chips (dashed); class
  breakdown keeps single-hue bars (dataviz pass).
- **Replay** — the tape. Radar well on `--bg-tape` with `--r-lg`; transport
  bar tokens; roster/kill feed on Card; canvas colors move to `theme.ts`
  (Renderer/Heatmap drift dies). The round-by-round rail's active moment is a
  solid 2px tone edge, not a dash (§5).
- **Trends** — the instrument panel. Chart line becomes ink (not CT blue);
  axes/labels per dataviz skill; ribbon/sparks on tokens.
- **Corpus** — the archive. Gate meter cells ink-filled; heatmap stays
  single-hue (CT blue is correct there: corpus grids are per-side data).
- **Settings** — the desk drawer. Cards, one Input, Table for thresholds.

## 10. Accessibility & quality floor

WCAG AA contrast on every token pair used (ink on bg0 ≈ 12.9:1, ink-dim on
bg1 ≥ 4.6:1 — verify pairs when finalizing values; ink-faint is decorative
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
```

`styles.css` is deleted at the end of the milestone; nothing may import it.
