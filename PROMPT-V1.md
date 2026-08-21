# ClutchFactor v1 — Engineering Charter

**Status: approved charter for v1. This document supersedes `PROMPT.md` wherever
they conflict; everything PROMPT.md specifies that this file does not restate
(the death taxonomy, detector definitions D1–D6, §5A rule families, thresholds,
the evidence contract, risk register) remains binding. Read both. Read
`docs/spec/death-taxonomy.md`. Then read GitHub issue #9 — fully READ EVERYTHING REGARDING THE FEATURE AND LOOK AT THE MOCKUP TOO**

---

## 1. Where we are, and the verdict on v0

v0 shipped (tag `v0.1.0`): parser, library, 60 fps 2D replay, the §5A rule
engine with hand-verified detectors, match reports with narrated insights,
cross-demo habits, a reference-corpus positioning detector, trends, settings,
and a working two-platform release pipeline. The bones are good and the honesty
discipline is real.

The owner's verdict, which this charter exists to fix: **v0 is a skeleton. It
looks generic, feels generic, and the coaching is superficial.** A player reads
"you died isolated 7 times" and still doesn't know what happened, why it
mattered, or what to do on Thursday night when they queue again. The UI is
clean but anonymous — it could be any dashboard. v1 turns the skeleton into a
product that feels like sitting next to a paid coach who watched your demos
before you arrived.

v1 has four pillars, in this order of importance:

1. **Round-by-round coaching** (GitHub issue #9) — the flagship. Non-negotiable
   and must be *perfect*.
2. **The AI coaching layer** — Gemini-powered natural coaching voice and
   cross-match synthesis, strictly grounded in detector facts.
3. **A premium design system** — the app must look and feel like the most
   elegant product in the CS2 space, not a template.
4. **Coaching depth and a stats surface** — fix what the data already
   diagnosed, make every insight actionable, and show the numbers players
   expect to see.

---

## 2. Ground rules (non-negotiable, every session)

1. **`main` is PR-only** (ADR-0005). Branch → `gh pr create` →
   `gh pr merge --auto --squash` → verify the merge actually fired
   (`gh pr view --json state,mergeStateStatus`). Required checks: `rust`,
   `windows-build`, `web`. Small coherent commits, conventional messages,
   nothing left unmerged for more than ~an hour of work.
2. **INVOKE THE SKILLS — ACTUALLY INVOKE THEM.** This is standing owner
   feedback and it is absolute. Use the Skill tool to invoke, every single
   time, never "following the skill from context" or from memory:
   - `superpowers:brainstorming` before any design decision that isn't already
     settled here or in an issue;
   - `superpowers:writing-plans` before every milestone's implementation;
   - `superpowers:subagent-driven-development` to execute every plan
     (worktree-isolated implementers, per-task reviews, whole-branch final
     review on the most capable model, ledgers that survive compaction);
   - `frontend-design:frontend-design` **and** `dataviz` before writing ANY
     screen, component, or chart code — once per screen/feature with a brief
     specific to it, not once per project.
   A milestone where a skill was "followed" but not invoked is a process
   failure even if the code is good.
3. **Real data only.** Every feature is built and verified against real `.dem`
   files in `fixtures/`. No fake match data, no placeholder insights, no
   demo-reel screenshots of invented numbers.
4. **The evidence contract is sacred.** Every coaching claim — template or
   AI-written — carries an `EvidenceRef` the replay can jump to. No evidence →
   the feature gets redesigned, not shipped.
5. **Bias to silence, never to blame.** False negative ≫ false positive. The
   AI layer does not get to relax this: a hallucinated coaching claim is a
   ship-blocking bug, not a tone problem.
6. **Verify external APIs before coding against them** — Gemini API endpoints
   and current model names, Tauri, demoparser2. Look them up; do not guess.
7. **Secrets never touch the repo.** The Gemini API key is BYO: entered in
   Settings (stored in the local SQLite settings table), overridable via the
   `CLUTCHFACTOR_GEMINI_KEY` env var for dev. It must never appear in any
   committed file, log output, or error message. Add a CI grep guard. The
   owner holds the key and will supply/rotate it — ask when the Settings field
   exists, as a batched ask.
8. **Docs discipline**: `CLAUDE.md` (≤120 lines, always correct),
   `docs/PROGRESS.md` after every work chunk, half-page ADRs for significant
   decisions, milestone tags. Update `CLAUDE.md`'s pointer line to name this
   charter in the first working session.
9. **Autonomy with batched asks.** Work without waiting; collect owner
   questions into batched asks at milestone boundaries. Stop only for
   destructive/irreversible actions or genuine scope decisions.

---

## 3. Pillar 1 — Round-by-round coaching (GitHub issue #9)

**THE most important feature of v1. It must be perfect.**

**FIRST ACTION for whoever builds this: run `gh issue view 9` and read the
entire issue — every section, every table, every acceptance criterion — until
you could re-derive its decisions from memory. The issue IS the spec. Do not
plan, design, or write a line of code for this feature before you have fully
read and understood issue #9.** Its decisions are binding:

- Coach rail beside the 2D replay; rounds selected by **impact threshold with
  a cap**, never fixed top-N; attention dots on the round strip that spend no
  color channel.
- **Win-probability table** for impact scoring — the man-count heuristic is
  explicitly rejected; do not build it "as an interim step".
- The five-verdict vocabulary (`Won it` / `Cost you` / `Not on you` / `Traded`
  / `Quiet`) with its two hard rules: `Cost you` measures position change and
  never asserts fault; `Not on you` must be positively established by a rule
  (e.g. `H2_BAITED_TRADE`), never inferred from the absence of flags.
- Moment list where **the number is the finding** ("Nearest 1,223 u at
  Catwalk") and the prose is only its label; flag-to-moment join on
  `(match_id, round, tick)`; `round_phase` added to `AnalysisContext` first.
- Narration for the rail lives in a **new module**, not `templates.rs`.
- **No economy/buy coaching in v1** (no buy data is parsed); at least one
  `Won it` round surfaced whenever one qualifies; rail text generated from the
  round's own event stream, never a fixed template with counts substituted in.
- Replay integration: focus players bright, everyone else dimmed; the dashed
  line to the nearest teammate with its distance labelled, solid line to the
  killer. *The picture makes the argument; the rail only names it.*

**Definition of perfect:** all 14 acceptance boxes in issue #9 check green;
every rail claim hand-verified against raw SQL and the replay on the owner's
real demos (§12-style cross-checks, recorded in the goldens README); the
open questions in issue #9 §10 (responsive behavior at minimum) resolved by
decision, not default; and an owner sign-off ask sent with specific rounds to
review. The AI layer (Pillar 2) then narrates the rail's moments — build the
rail's data honestly first, voice second.

---

## 4. Pillar 2 — the AI coaching layer (Gemini)

**Architecture: grounded narrator + synthesis.** The detectors remain the only
source of truth. Gemini turns their structured facts into a natural coaching
voice and composes cross-match priorities. It never generates a claim of its
own.

- **`GeminiNarrator`** implements the existing `CoachingNarrator` seam
  (PROMPT.md §8 reserved exactly this). Input: the same structured facts the
  template narrator gets (insight JSON, round moments, habit data, match
  context) plus a compact style guide. Output: narration text.
- **Match synthesis**: one new surface — a coach's opening statement on the
  match report and a "what to work on next" synthesis across the last N
  matches (feeds from habits + trends + RBR verdicts). Same grounding rules.
- **The grounding contract (hard requirement):** the prompt to Gemini contains
  the facts and forbids new claims; the response is validated before display —
  every number, callout, round reference, and rule claim in the output must
  appear in the input facts, enforced by a validator (numbers/rounds/callouts
  extracted and checked; response rejected and retried once, then fall back to
  template text). Write adversarial tests: feed facts, assert invented
  numbers/claims are caught.
- **Graceful degradation**: no key, offline, rate-limited, or validation
  failure → `TemplateNarrator` output, seamlessly. The app must remain fully
  functional and honest with zero network access. AI text is visibly marked
  (a subtle "coach" affordance) with a regenerate action.
- **Caching**: narrations cached in SQLite keyed by (content hash of input
  facts, model, style version) — a report re-open costs zero tokens; a
  regenerate busts the cache row.
- **Model selection**: verify current Gemini model names/endpoints at build
  time (Ground Rule 6). Default to a fast/cheap tier for per-insight and
  per-moment narration and a stronger tier for match synthesis; make both
  configurable in Settings. Respect token budgets — batch per-report narration
  into as few calls as the grounding contract allows.
- **Voice**: the §8 coach voice rules still bind (numbers first, then the fix;
  no exclamation marks; never scold; "unusual, not wrong" honesty for D6).
  The AI's advantage is fluency and specificity — "that Catwalk re-peek in
  round 12" — not enthusiasm.

---

## 5. Pillar 3 — premium design system and UX

The current UI is clean but generic. v1 rebuilds the visual identity from
tokens up, then rebuilds navigation and every screen on it.

- **Study the field first, then differentiate.** Look at FACEIT, Leetify, and
  CSStats (current live product screenshots — verify, don't recall). They are
  dense, gamer-loud, stat-forward dashboards. ClutchFactor's position:
  **the elegant one** — a coach's studio, not a stats casino. Editorial
  typography with real personality, generous space, restrained motion, one
  disciplined accent system, data-ink restraint. Premium = what we leave out.
- **Design system milestone**: tokens (color, type scale, spacing, radii,
  elevation), a characterful display face paired with a workhorse body/mono
  (bundled locally — desktop app, no CDN), core components (cards, chips,
  buttons, inputs, tables, empty/loading/error states, toasts), sidebar
  navigation replacing the topbar, a match-header component (map, score,
  result, date, stats strip) reused across report/replay, and dark as the
  only v1 theme done impeccably. Document it in `docs/design/` with a live
  component reference the SDD reviewers can check screens against.
- **Every screen rebuilt** on the system: Library (richer match cards),
  Report, Replay (+ RBR rail), Trends, Corpus, Settings. CT `#4aa3ff` /
  T `#f5b83d` stay reserved for side identity; dataviz rules continue to bind
  all charts.
- **Process, mandatory**: invoke `frontend-design:frontend-design` and
  `dataviz` (Skill tool) before each screen; screenshot every screen after
  building (the AX + `screencapture` loop from M6) and self-critique against
  the design doc before calling it done. The final milestone review must
  include the screenshots.
- **UX quality floor**: keyboard focus visible everywhere, skeleton loading
  states, every empty state an invitation with a next action, every error in
  §7 voice (what happened + what to do), no layout shift on data arrival,
  windows resizable down to 1200×760 without breakage (RBR rail behavior at
  small widths per issue #9 §10 — decided, not defaulted).

---

## 6. Pillar 4 — coaching depth, correctness debt, and stats

**Correctness first (GitHub issue #6 — land before anything builds on top):**
the fixes were diagnosed on a retired branch and must be re-implemented on
main from the issue's diagnosis: `H3_WASTED_UTILITY` wording ("unused utility
in inventory", never "holding"); hotspot clustering by shared callout +
pairwise diameter (kills the false Ladder/Underpass/Catwalk cluster — port the
regression test); habits gain "most often at Catwalk (5) and Underpass (3)"
location clauses from the already-stored `last_place`; the 10 s lookback bound
on `death_positions`; hotspot dedup keyed `(map, place)`.

**Callouts everywhere (issue #2):** one prettifier (`BombsiteA` → "A site")
used by narrator, habits, RBR moments, and rendered on the replay map at
appropriate zoom. Positions in coaching text always name the callout.

**Actionability bar — the fix for "superficial":** every insight and habit
card must answer three questions or it doesn't ship: *what happened* (with the
number and the callout), *why it mattered* (round/impact consequence), *what
to practise* (one concrete, doable instruction — the issue #9 "only rendered
when a rule can back it" standard applies product-wide). Audit every existing
template against this bar during the AI-layer milestone; the ones that fail
get rewritten, not decorated.

**"What your coach watches" screen:** a legible page listing every detector —
what it looks for, its thresholds in plain language, what the engine cannot
see (economy, utility lineups, comms, aim mechanics beyond outcomes) — and
which classes are not yet built. The honesty that lives in the code becomes a
product surface users can trust.

**Coaching-first stats (not a stats-tracker pivot):** a stats strip on the
match header (K/D, ADR — computable from parsed `hurts` — HS%, KAST-style
round contribution, entry attempts/success, trade rate, clutch attempts/wins);
a per-round scoreboard view; Trends extended with these series. Every stat
links to the coaching that explains it (entry success → D4 insights, trade
rate → H2). Multi-demo import (issue #3): multi-select dialog, sequential
queue with per-file progress and per-file error reporting (the M6 review
already flagged last-error-only overwriting — fix it here).

**Shared infrastructure:** the win-probability table built for RBR is a
first-class module (`cf-analysis`), versioned and documented, sourced/derived
transparently — RBR impact, stats context, and any future leak board all read
from it.

---

## 7. Milestones

Each milestone: brainstorm (if open questions) → `superpowers:writing-plans` →
`superpowers:subagent-driven-development` → whole-branch final review (most
capable model) → docs + tag → batched owner ask. All via PRs.

- **V1.0 — Foundations.** Issue #6 fixes re-landed with regression tests;
  callout prettifier + plumbing (#2 groundwork); multi-demo import (#3);
  `round_phase` in `AnalysisContext`; win-probability table with tests.
  *DoD: every fix hand-verified against the real DB exactly as issue #6 did.*
- **V1.1 — Design system.** Tokens/type/components/sidebar/match-header;
  all existing screens rebuilt; `docs/design/` reference.
  *DoD: full screenshot walkthrough reviewed against the design doc; zero
  legacy-styled surfaces left.*
- **V1.2 — Round-by-round coaching.** Issue #9, complete.
  *DoD: all 14 acceptance criteria green; §12 hand-verification on owner
  demos recorded; owner sign-off ask sent naming specific rounds to review.*
- **V1.3 — AI layer.** `GeminiNarrator` + match synthesis + grounding
  validator + caching + Settings key UX + fallback + template actionability
  audit. *DoD: with the key removed the app is byte-identical to template
  mode; adversarial grounding tests pass; no secret in repo (CI guard);
  owner ask: supply/rotate key, judge the voice on their own report.*
- **V1.4 — Stats & understanding.** Match-header stats strip, per-round
  scoreboard, Trends v2, "What your coach watches" screen, callouts on the
  replay map. *DoD: every stat cross-checked against raw SQL for one real
  match; every stat links to its coaching.*
- **V1.5 — Polish & release v1.0.0.** Perf pass (report + replay + rail with
  a 30-match library), error/empty audit, README + screenshots refresh,
  tagged release, both installers smoke-tested.
  *DoD: the owner installs the Windows build and reviews a fresh match using
  RBR + AI coaching unassisted.*

---

## 8. First actions (first v1 session, in order)

1. Read `PROMPT.md`, this file, `docs/spec/death-taxonomy.md`,
   `docs/PROGRESS.md`. Run `gh issue view 9` and read it completely; then
   issues #6, #2, #3.
2. Update `CLAUDE.md` to point at this charter (keep ≤120 lines).
3. Confirm fixtures still present (`fixtures/own/`, `fixtures/public/`);
   note the standing corpus ask (~8 pro Mirage demos) stays open.
4. Invoke `superpowers:writing-plans` for V1.0 and begin. Batched owner asks
   only at milestone boundaries — the Gemini key ask lands with V1.3.

Commit and push changes as you go on a new branch.