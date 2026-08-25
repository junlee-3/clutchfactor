# Play Ledger & The Coach — spec for V1.2b and V1.3

> Owner course-correction, 2026-08-25, adopted as spec (supersedes issue #9 §6's
> "only rounds above the threshold are surfaced" and the V1.1 "Film Room"
> palette/typography; everything else in issue #9, PROMPT-V1.md, and ADR-0008
> still binds). Owner intent, verbatim in substance:
>
> 1. "For half the rounds it's just not coaching at all — it needs to be doing a
>    live commentary of each play you made: was it a good smoke, did you trade,
>    is this good positioning, did you rush in."
> 2. "The AI shouldn't just be writing text — it should BE the coach."
> 3. "Go back to the old UI: old colors, old fonts, keep the more premium
>    changes, just clean it up. Show the map image thumbnail for the games."

## 1. UI revert (V1.2b-A) — old navy, old system fonts, keep the structure

**Palette.** Token *names* stay (no churn) but are re-valued to the v0 palette,
and the chalk-era names are renamed to neutral ink names in one mechanical pass:

| token | value | role |
|---|---|---|
| `--bg0` / `--bg1` / `--bg2` | `#0e1116` / `#151a21` / `#1d242e` | canvas / surface / hover-inset |
| `--bg-tape` | `#0b0d11` | radar & heatmap wells (a touch darker than bg0) |
| `--line` / `--line-strong` | `#232b36` / `#2f3944` | hairlines / emphasized |
| `--ink` / `--ink-bright` / `--ink-dim` / `--ink-faint` | `#dfe5ec` / `#f2f5f8` / `#8a94a3` / `#5c6672` | primary / interactive / secondary / tertiary ink (renamed from `--chalk*`) |
| `--accent` | `#4aa3ff` | the app accent — buttons, focus ring, progress fills (v0 used CT blue for this; restored; the app chrome has no links) |
| `--ct` / `--t` / `--win` / `--loss` / `--tie` | `#4aa3ff` / `#f5b83d` / `#5dbb7a` / `#d16a5f` / `#8a94a3` | unchanged game/outcome hues |

`--accent` and `--ct` share a value but stay separate tokens so side identity
remains explicit in code. Derived surface/border tokens keep their formulas.
Canvas/TS reads through `theme.ts` unchanged (token names updated).

**Type.** The bundled Fraunces/Inter/JetBrains Mono files, their OFL texts, and
the `@font-face` blocks are removed. Stacks return to v0 verbatim:
`--font-sans: -apple-system, "Inter", "Segoe UI", system-ui, sans-serif`;
`--font-mono: ui-monospace, "SF Mono", "JetBrains Mono", Menlo, Consolas, monospace`;
`--font-display` = the sans stack (no serif). The eight-role scale keeps its
sizes; display/title roles use sans at weight 600. ADR-0009 records the
revert (owner preference) and marks ADR-0007 superseded.

**Structure kept.** Sidebar shell, `ui/` components, MatchHeader, skeletons,
toasts, one focus treatment, window floor 1200×760, the dashed-underline
evidence chips (structural, not chalk-specific).

**Rail per the mockup.** Active moment = a 2px *severity* stripe on the left
edge (`--loss` mixed by the moment's |Δp| or rule severity, `--win` for
positive plays), replacing the dashed-chalk stripe (design-system §5 updated:
dashed = evidence chips only; solid severity edge = the current moment).
Verdict chip: `cost_you` = loss outline + loss text, `won_it` = win outline,
`not_on_you`/`traded`/`quiet` = neutral outline. The rail header (round
number, verdict chip) is sans as in the mockup; the context line and the
timestamps are mono (what shipped).

**Library thumbnails.** Each match row shows the map's radar image
(`radarImageUrl(map, "upper")`, the vendored `assets/maps/<map>.png`) as a
56×56 thumbnail: `object-fit: cover`, `--r-sm`, 1px `--line`, 80% opacity
(100% on hover/focus); unknown map → a mono two-letter fallback tile.

**Cleanup.** Chalk-era comments/names swept, the V1.2 deferred minors that are
one-liners (double `annotationGeometry` compute, eyebrow idiom, `.tnum`
orphan), walkthrough recaptured to `docs/design/walkthrough-v1.2b/`, README
screenshots refreshed, design-system.md re-issued as "v2: the studio, navy"
with the new values (no aesthetic essay — a reference).

## 2. Play ledger (V1.2b-B) — every round, every play, with a number

**Scope change.** Every round gets coaching content. `round_review`'s impact,
verdicts, selection, and attention dots are unchanged (ADR-0008) — they now
only decide which rounds to read *first*. The rail shows the play ledger for
the selected round whether or not it is "selected" for attention.

**Model.** A new pure module `cf-analysis::play_ledger` computed inside
`analyze()` from `AnalysisContext` (it needs tick samples, blinds, hurts,
grenades — everything the DB post-pass lacks):

```
RoundLedger { round, plays: Vec<Play>, timeline: Vec<TimelineEvent> }

Play {
  tick, phase: RoundPhase,
  kind: setup | flash | smoke | he | molotov | rush | rotation |
        kill | death | assist | trade | missed_trade | plant | defuse | flag | outcome,
  facts: Value,            // numbers + RAW callouts + steamid strings (as today)
  quality: Option<Quality>,// Good | Bad | Neutral — ONLY when a measure backs it
  rule_id: Option<String>, // when an existing rule fired on this tick
  delta_p: Option<f32>,    // state-changing plays only (same engine as ADR-0008)
}

TimelineEvent { tick, kind: kill | plant | defuse | explode, actor, subject, side }
  // everyone's kills/bomb events — the situation, for the coach
```

Play definitions (thresholds in `DetectorConfig::ledger`, seconds/units):

| kind | when | facts | quality (measure-backed only) |
|---|---|---|---|
| setup | `freeze_end + setup_s` (5.0) | place, nearest teammate (id, dist), teammates within `trade.isolation_u` | none — positioning judgement is the coach's |
| flash | each tracked flashbang detonate (grenade event left-joined to the blind group at that tick; a flash that blinded nobody is a dud, not invisible) | enemies blinded ≥ `flash.effective_s`, teammates blinded, self, converted within `flash.conversion_window_s` | Good: ≥1 enemy & 0 team; Bad: any team/self blind; Neutral: 0/0 |
| smoke | each tracked smoke detonate | place, phase, dead_time (H6 logic) | Bad if dead-time; else none |
| he / molotov | each tracked detonate/inferno | enemy damage, team damage, victims | Bad if team damage; else none (damage stands alone) |
| rush | tracked beyond `timing.min_spawn_distance_u` within `timing.early_aggression_s` with no teammate within `trade.distance_u` | distance, seconds, nearest teammate | Bad if died in that window (H11); else Neutral, labeled "no support" |
| rotation | bomb planted | tracked at planted site? time-to-site if arrived | Bad if H11_SLOW_ROTATION fired; else none |
| kill / death | each tracked kill / death | victim/killer, place, distance, headshot, traded, isolated, man context before; death adds `round_end_delta_s` (clamped at 0) + `dead_time` (died after the round was decided); merged flag details, plus the additive `exculpatory: true` marker when any `rbr.exculpatory_rules` flag merged in (no seen-first metric — the parser's `spotted` flag is per-player, not pairwise, so it stays silent) | death, in precedence order: (1) ANY exculpatory flag merged in (`exculpatory: true`) → Neutral, whichever rule won `rule_id` (ADR-0008 "Not on you"); (2) else any non-exculpatory rule fired (`rule_id` set — H2/H3/H4/H6/H11/H16) → Bad, regardless of `traded`; (3) else `traded` → Neutral; (4) else none (a fair duel). `traded` only matters when no rule fired |
| assist | tracked assist | victim, teammate | none |
| trade / missed_trade | a teammate died within `trade.distance_u` of tracked | teammate, killer, tracked committed within `trade.commit_window_s`? | Good (trade) / Bad (missed, H2_FAILED_TRADE) |
| plant / defuse | tracked plants/defuses | delta_p | none |
| flag | a tracked-player rule fired on a tick that carries no play of the tracked player's (a bare flag; a flag on a play's tick merges into that play instead) | the rule's own `details` | Bad; Neutral when the rule is in `rbr.exculpatory_rules` |
| outcome | round end | won, survived, my-vs-their alive at end, reason | none |

Silence bias holds: a play with no computable number is not emitted; a quality
tag without a backing measure is never emitted. Plays reuse the detectors'
per-event logic (flash effectiveness, dead-time smoke, trade windows,
early-aggression, rotation) lifted into shared per-event functions — the
match-level detectors and the ledger must not drift.

**Storage & serving.** Migration 0008 `round_plays(match_id, round, plays_json,
timeline_json)` written by `save_analysis`; `get_round_review` gains `plays`
and `timeline` per round. Template captions per kind (numbers first, the
existing rail voice) are the offline/fallback narration.

**Backfill.** Pre-V1.2b imports have no tick-level ledger. The store records
only `file_name` + `file_hash` today, so migration 0008 also adds
`matches.source_path` (written at import). A `re_analyze_match(match_id,
path?)` command re-parses the demo — from `source_path` when it still exists,
else from a file the user picks, whose hash must equal the stored `file_hash` —
and replaces the match's rows in place (`replace_match_data`, same match id,
so URLs and reviews survive), then re-runs analysis, review, and ledger. The
Library row gets a "Re-analyze" action with progress; a missing file gets a
§7-voice error naming the file. Owner fixtures cover the dev DB.

## 3. The coach (V1.3) — facts grounded, judgment free

`GeminiNarrator` is not a text renderer for algorithm output; it is the coach.
The engine supplies what happened; the model decides what mattered and what to
do about it.

**Inputs per round:** the round's `plays` + `timeline`, header (side, result,
K-D, man context), verdict + impact, match context (map, score, names), and a
compact digest of the last N rounds' ledgers in this match (so it can say "the
third time this half you pushed Connector without info").

**Prompt.** System: the coach persona under PROMPT-V1.md's voice rules (the §8 seam in PROMPT.md; the rules text is in PROMPT-V1.md) (numbers
first, then the fix; no exclamation marks; never scold; "unusual, not wrong"
for corpus positioning; specific — name the callout, the teammate, the
timestamp). It may judge, prioritize, and advise from its own CS2 knowledge.
It must cite only facts present in the input and must not describe events the
input does not contain. Output is JSON:

```
RoundCommentary {
  read: string,                       // 2–4 sentences of live commentary on the round
  plays: [{ tick, comment: string }], // one line per play it chose to comment on
  why_it_mattered: string | null,
  what_to_practise: string | null,
  focus: string | null,               // the one thing to take from this round
}
```

**Grounding validator (hard requirement, ship-blocking on failure).** From the
response: every number (after unit normalization), every player name, every
callout, every round number, and every `tick` must appear in the input facts;
otherwise the response is rejected, retried once with the violations listed,
and on second failure the template captions render. Opinions and advice are
not validated — they are the coach's, and the UI marks the block "Coach's
read" with a regenerate action. Adversarial tests feed facts and assert
invented numbers, names, callouts, and ticks are caught.

**Match synthesis.** Coach's opening statement on the report and "what to work
on next" across the last N matches (habits + trends + verdicts), same contract.

**Key & guard.** Settings field (SQLite settings table) → `CLUTCHFACTOR_GEMINI_KEY`
env override → in dev only, the repo-root `env.local` file is loaded at startup
(the owner keeps the key there; the file is gitignored). Never logged, never
in error text. CI grep guard fails on `AQ\.[A-Za-z0-9_-]{30,}` and
`AIza[0-9A-Za-z_-]{30,}` in tracked files.

**Models, batching, cache.** Verify current Gemini model names/endpoints at
build time (ground rule 6). Default a fast tier for per-round commentary,
batched per match into as few calls as the schema allows, and a stronger tier
for synthesis; both configurable in Settings. Cache by (facts hash, model,
style version); regenerate busts the row. No key / offline / rate-limited /
invalid → templates, seamlessly.

## 4. Milestones

- **V1.2b** — A (UI revert + thumbnails + cleanup), then B (play ledger on
  every round + re-analyze). DoD: walkthrough graded against design-system v2;
  every round of an owner demo shows a numbered play ledger; three rounds'
  ledgers hand-checked against raw SQL/replay; re-analyze verified on a
  pre-V1.2b import.
- **V1.3** — C. DoD (charter): key removed → byte-identical template mode;
  adversarial grounding tests pass; CI guard green; owner judges the coach's
  voice on their own report; the rail reads like a coach on every round.
