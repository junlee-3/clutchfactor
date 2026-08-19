import { useNavigate } from "react-router-dom";
import { evidenceUrl } from "../lib/evidence";
import type { NarratedInsight, RoundStat } from "../lib/ipc";
import { chipLabel } from "../lib/report";

interface Props {
  matchId: number;
  insight: NarratedInsight;
  rounds: RoundStat[];
  tickrate: number;
}

export function InsightCard({ matchId, insight, rounds, tickrate }: Props) {
  const navigate = useNavigate();
  return (
    <article
      className="insight-card"
      style={{ "--sev": insight.severity } as React.CSSProperties}
    >
      <div className="ic-spine" aria-hidden="true" />
      <div className="ic-content">
        <header className="ic-head">
          <h4>{insight.title}</h4>
          <span
            className="ic-conf"
            title={`Severity ${insight.severity.toFixed(2)} · confidence ${insight.confidence.toFixed(2)}`}
          >
            {Math.round(insight.confidence * 100)}%
          </span>
        </header>
        <p className="ic-body">{insight.body}</p>
        {insight.evidence.length > 0 && (
          <div className="ic-chips">
            {insight.evidence.map((ev, i) => (
              <button
                key={i}
                className="chip"
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
              </button>
            ))}
          </div>
        )}
      </div>
    </article>
  );
}
