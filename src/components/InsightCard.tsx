import { useNavigate } from "react-router-dom";
import { evidenceUrl } from "../lib/evidence";
import type { NarratedInsight, RoundStat } from "../lib/ipc";
import { chipLabel } from "../lib/report";
import { Card } from "./ui/Card";
import { Chip } from "./ui/Chip";

interface Props {
  matchId: number;
  insight: NarratedInsight;
  rounds: RoundStat[];
  tickrate: number;
}

// One insight = one Card with a severity edge (design-system.md §6, §9): the
// left spine mixes --loss by the insight's own 0-1 severity, so the coach's
// most urgent points read heavier without a second color. Evidence chips
// are the dashed chalk-annotation grammar (§5) — always interactive, always
// jumping straight to the tape (evidenceUrl, untouched).
export function InsightCard({ matchId, insight, rounds, tickrate }: Props) {
  const navigate = useNavigate();
  return (
    <Card edge="severity" sevValue={insight.severity}>
      <header className="rpt-head">
        <h4 className="type-heading">{insight.title}</h4>
        <Chip
          variant="count"
          title={`Severity ${insight.severity.toFixed(2)} · confidence ${insight.confidence.toFixed(2)}`}
        >
          {Math.round(insight.confidence * 100)}%
        </Chip>
      </header>
      <p className="rpt-body type-body">{insight.body}</p>
      {insight.evidence.length > 0 && (
        <div className="rpt-chips">
          {insight.evidence.map((ev, i) => (
            <Chip
              key={i}
              variant="evidence"
              title="Watch this moment in the replay"
              onClick={() =>
                navigate(
                  evidenceUrl(matchId, {
                    round: ev.round,
                    tick_start: ev.tick_start,
                    tick_end: ev.tick_end,
                    focus_players: ev.focus_players,
                    camera_hint: ev.camera_hint ?? undefined,
                  }),
                )
              }
            >
              {chipLabel(ev, rounds, tickrate)}
            </Chip>
          ))}
        </div>
      )}
    </Card>
  );
}
