import type { PlayDto, RoundReviewDto } from "../lib/ipc";
import { activeMomentIndex, nextFlagged, prevFlagged, stripeTone } from "../replay/rail";
import { fmtClock } from "../replay/timeline";
import type { TimelineSpec } from "../replay/timeline";
import { Button } from "./ui/Button";
import { Card } from "./ui/Card";
import { Chip } from "./ui/Chip";

interface Props {
  reviews: RoundReviewDto[];
  round: number;
  spec: TimelineSpec;
  tickrate: number;
  displayTick: number;
  onJump: (tick: number) => void;
  onRound: (round: number) => void;
}

/** Verdict -> Chip class. Outlined, never filled (spec §1): the two
 * measured-impact verdicts get their outcome hue on the outline and text,
 * everything else a neutral outline. Side hues (--ct/--t) never appear here. */
function verdictChipClass(verdict: string): string {
  if (verdict === "won_it") return "rpl-rail-verdict-won";
  if (verdict === "cost_you") return "rpl-rail-verdict-loss";
  return "rpl-rail-verdict-neutral";
}

// The feature's face (issue #9 mockup; design-system.md §5/§6/§9): a coach's
// note beside the tape, not a stats panel. The picture makes the argument
// (canvas overlay, Task 9) — this only names it: round header, then the
// play ledger — every round narrated (spec §2), setup through outcome, not
// just the flagged ones. `selected` still gates the why/practise prose and
// the timeline dots; it no longer gates whether the round gets a list at
// all. A match imported before the ledger existed (no plays at all — the
// ledger always writes setup + outcome for a round the player was in) falls
// back to its review moments with a re-analyze hint, hint included when
// there are no moments to show either. The active row carries a solid 2px
// tone edge (loss/win/neutral — spec §1), not a dashed stripe; dashed stays
// reserved for evidence.
export function CoachRail({
  reviews,
  round,
  spec,
  tickrate,
  displayTick,
  onJump,
  onRound,
}: Props) {
  const review = reviews.find((r) => r.round === round) ?? null;
  const moments = review?.moments ?? [];
  // Every round narrated (spec §2): the ledger's plays are the list. A match
  // imported before V1.2b has no ledger yet — no plays at all is that case,
  // since the ledger always writes setup + outcome — so fall back to its
  // review moments (rbr-v2 builds them for every round) and say how to get
  // the rest, whether or not there are moments to show.
  const plays: PlayDto[] = review?.plays ?? [];
  const preLedger = plays.length === 0;
  type RailRow = {
    tick: number;
    headline: string;
    facts: string[];
    delta_p: number | null;
    rule_id: string | null;
    quality?: string | null;
  };
  const rows: RailRow[] = preLedger ? moments : plays;
  const activeIdx = activeMomentIndex(rows, displayTick);
  const prev = prevFlagged(reviews, round);
  const next = nextFlagged(reviews, round);

  if (!review) return null;

  return (
    <aside className="rpl-coach-rail">
      <Card>
        <div className="rpl-rail-header">
          <div className="rpl-rail-heading-row">
            <h3 className="type-title rpl-rail-round-num">Round {review.round}</h3>
            <Chip variant="count" className={verdictChipClass(review.verdict)}>
              {review.verdict_label}
            </Chip>
          </div>
          <p className="type-data rpl-rail-context">
            {review.side} ·{" "}
            <span className={review.won ? "rpl-rail-result-won" : "rpl-rail-result-lost"}>
              {review.won ? "won" : "lost"}
            </span>{" "}
            · you {review.kills}-{review.deaths}
            {review.man_context ? ` · ${review.man_context}` : ""}
          </p>
        </div>

        <div className="rpl-rail-moments">
          {rows.map((r, i) => (
            <button
              key={`${r.tick}-${i}`}
              className={`rpl-rail-moment${
                i === activeIdx ? ` rpl-rail-moment-active rpl-rail-tone-${stripeTone(r)}` : ""
              }`}
              title="Jump to this play"
              onClick={() => onJump(r.tick)}
            >
              <span className="rpl-rail-moment-time type-data">
                {fmtClock(spec, r.tick, tickrate)}
              </span>
              <span className="rpl-rail-moment-body">
                <span className="rpl-rail-moment-headline type-ui">{r.headline}</span>
                {r.facts.map((f, fi) => (
                  <span key={fi} className="rpl-rail-moment-fact type-data">
                    {f}
                  </span>
                ))}
              </span>
            </button>
          ))}
          {rows.length === 0 && (
            <p className="type-body rpl-rail-quiet">Nothing recorded for you this round.</p>
          )}
        </div>

        {preLedger && (
          <p className="type-data rpl-rail-hint">
            Showing the key moments only — Re-analyze this match from the Library for the full play-by-play.
          </p>
        )}

        {review.selected && review.why_it_mattered && (
          <div className="rpl-rail-note">
            <p className="type-micro rpl-rail-note-label">Why it mattered</p>
            <p className="type-body">{review.why_it_mattered}</p>
          </div>
        )}
        {review.selected && review.what_to_practise && (
          <div className="rpl-rail-note">
            <p className="type-micro rpl-rail-note-label">What to practise</p>
            <p className="type-body">{review.what_to_practise}</p>
          </div>
        )}
      </Card>

      {(prev !== null || next !== null) && (
        <div className="rpl-rail-footer">
          {prev !== null ? (
            <Button variant="secondary" size="sm" onClick={() => onRound(prev)}>
              ← R{prev}
            </Button>
          ) : (
            <span />
          )}
          {next !== null ? (
            <Button variant="secondary" size="sm" onClick={() => onRound(next)}>
              R{next} →
            </Button>
          ) : (
            <span />
          )}
        </div>
      )}
    </aside>
  );
}
