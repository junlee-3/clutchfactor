# ADR-0010: The coach — architecture, key handling, models

**Status:** accepted · 2026-08-25

## Context
Spec §3 (`docs/spec/play-ledger-and-coach.md`): the AI layer must *be* the
coach — interpret, weigh, advise — while never citing a fact the engine did
not supply, and the app must be byte-identical to template mode without a
key. PROMPT.md §8 designed the `CoachingNarrator` seam as a synchronous
trait; the coach needs the network, batching and a cache.

## Decision
- **Pure/impure split.** `cf_narrator::coach` holds everything the model
  sees and everything we check (types, prompt rendering, JSON schemas, the
  grounding validator, parsing) with no I/O; `src-tauri/src/coach/` owns the
  key, the HTTP call, caching and fallback. The synchronous
  `CoachingNarrator` trait stays as the template seam; the coach is an
  async orchestration beside it, not an impl of it.
- **Grounding.** The validator builds its allowed sets from exactly the text
  the model was shown (`render_round_block`), so anything citable must be
  rendered and nothing rendered may be a guess. Numbers, roster names, known
  callouts, round numbers, ticks; opinions are free. The known-callout set
  is every callout anyone stood in during the match — the ledger's places ∪
  the position samples' `last_place` — not a per-map list. Names and
  callouts match at word boundaries only ("CT spawn" never grounds "T
  spawn"). Reject → one retry with the violations listed → template
  fallback for that round.
- **Key.** `CLUTCHFACTOR_GEMINI_KEY` env var overrides the Settings value
  (`gemini_api_key` in the SQLite settings table — the charter's choice; the
  DB lives in the user's app-data dir). Debug builds seed the env var from
  the gitignored repo-root `env.local`; release builds never read it. The
  value never appears in logs, errors, DTOs (only a 4-char hint) or the repo
  (CI `secrets` job greps every PR; to be added to the required checks).
- **Transport.** REST `v1beta/models/{model}:generateContent` via `reqwest`
  (rustls), key in `x-goog-api-key`, JSON-schema structured output,
  temperature 0.4, 45 s timeout. Default model `gemini-3.7-flash` for both
  per-round batches (6 rounds per call) and synthesis; `gemini-3.5-flash-lite`
  is the documented cheap alternative; both editable in Settings.
- **Cache.** `coach_cache` (migration 0009) keyed by (match, kind, round)
  storing the facts hash (sha256 of the rendered block + model + style
  version), status (`ok` | `fallback`) and the validated response. A changed
  ledger, model or style regenerates; Regenerate busts the row explicitly.
  Fallback rows are cached too so a failing round is not re-billed on every
  open.

## Consequences
- Cost ≈ one call per six rounds plus one synthesis call per match; a
  24-round match is ~5 calls, a few cents at 2026 Flash pricing.
- Word numerals and entities outside the roster/callout sets are not
  validated — the prompt forbids them; the validator catches every numeric,
  roster-name, callout and tick invention.
- What the validator provably does not catch: mis-attribution of a real
  name or number to the wrong event ("Kit awped you" when Sam did, with
  both in the facts), word numerals ("three teammates"), and callouts
  outside the match's visited set (nobody stood there, so it isn't in the
  known set). The prompt forbids all three; the owner's voice review of
  real output (the milestone hand-verification) is what covers them.
  Earlier rounds are citable by number because the "Earlier this match"
  digest spells them "Round N" — that grounds every earlier round number
  as a bare integer (so "7" passes in any field of a round-8 read, whatever
  it actually refers to); the per-round grounding set is the round block
  only, so the batch header's map, score and roster line do not ground a
  round read (a cite of the final score is rejected).
- A future `ClaudeNarrator` reuses the pure half unchanged; only
  `gemini.rs` is provider-specific.
- Tick labels are confined to the `plays` array's `tick` field — the prose
  fields (`read`, `why_it_mattered`, `what_to_practise`, `focus`) refer to
  moments by clock time or event, never `[tick N]`; place names are used
  exactly as the facts write them, never a map slug or internal id. Style
  `coach-v3`.
