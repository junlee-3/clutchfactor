import { useEffect } from "react";
import type { RailMomentDto, RoundReviewDto } from "../lib/ipc";
import { activeMomentIndex, nextFlagged, prevFlagged } from "../replay/rail";
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
  onActiveMomentChange: (m: RailMomentDto | null) => void;
}

/** Verdict -> extra Chip class. Only the two measured-impact verdicts get a
 * surface tint (design-system.md §2 derived tokens); "not_on_you"/"traded"/
 * "quiet" stay neutral — the verdict is named, not color-coded, for anything
 * that isn't a measured swing. Side hues (--ct/--t) never appear here. */
function verdictChipClass(verdict: string): string | undefined {
  if (verdict === "won_it") return "rpl-rail-verdict-won";
  if (verdict === "cost_you") return "rpl-rail-verdict-loss";
  return undefined;
}

/** The one-liner for a round the rail doesn't elaborate on. Verdict-agnostic
 * on purpose: an unselected round can still carry any verdict (e.g. a low-
 * impact "Traded") — this restates the round's own numbers, never assumes
 * "Quiet" specifically. */
function quietSummary(r: RoundReviewDto): string {
  const outcome = r.won ? "won" : "lost";
  const manContext = r.man_context ? `, ${r.man_context}` : "";
  return `Nothing here needed the coach — you ${outcome} it, ${r.kills}-${r.deaths}${manContext}.`;
}

// The feature's face (issue #9 mockup; design-system.md §5/§6/§9): a coach's
// note beside the tape, not a stats panel. The picture makes the argument
// (canvas overlay, Task 9) — this only names it: round header, a moment list
// where the timestamp is mono and the numbers are the content, why/practise
// as micro-eyebrow sections, prev/next flagged-round nav. The active moment
// carries the §5 dashed-chalk left stripe — the one signature device, spent
// here, nowhere else in this component.
export function CoachRail({
  reviews,
  round,
  spec,
  tickrate,
  displayTick,
  onJump,
  onRound,
  onActiveMomentChange,
}: Props) {
  const review = reviews.find((r) => r.round === round) ?? null;
  const moments = review?.moments ?? [];
  const activeIdx = activeMomentIndex(moments, displayTick);
  const activeMoment = moments[activeIdx] ?? null;
  const prev = prevFlagged(reviews, round);
  const next = nextFlagged(reviews, round);

  // Fires whenever playback crosses into a new moment (or leaves them all,
  // -1 -> null) — Task 9 drives the canvas annotation + focus dimming off
  // this; wired here regardless so the contract is live from day one.
  useEffect(() => {
    onActiveMomentChange(activeMoment);
  }, [activeMoment, onActiveMomentChange]);

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

        {review.selected ? (
          <>
            <div className="rpl-rail-moments">
              {moments.map((m, i) => (
                <button
                  key={`${m.tick}-${i}`}
                  className={`rpl-rail-moment${i === activeIdx ? " rpl-rail-moment-active" : ""}`}
                  title="Jump to this moment"
                  onClick={() => onJump(m.tick)}
                >
                  <span className="rpl-rail-moment-time type-data">
                    {fmtClock(spec, m.tick, tickrate)}
                  </span>
                  <span className="rpl-rail-moment-body">
                    <span className="rpl-rail-moment-headline type-ui">{m.headline}</span>
                    {m.facts.map((f, fi) => (
                      <span key={fi} className="rpl-rail-moment-fact type-data">
                        {f}
                      </span>
                    ))}
                  </span>
                </button>
              ))}
            </div>

            {review.why_it_mattered && (
              <div className="rpl-rail-note">
                <p className="type-micro rpl-rail-note-label">Why it mattered</p>
                <p className="type-body">{review.why_it_mattered}</p>
              </div>
            )}
            {review.what_to_practise && (
              <div className="rpl-rail-note">
                <p className="type-micro rpl-rail-note-label">What to practise</p>
                <p className="type-body">{review.what_to_practise}</p>
              </div>
            )}
          </>
        ) : (
          <p className="type-body rpl-rail-quiet">{quietSummary(review)}</p>
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
