import { useNavigate } from "react-router-dom";
import type { RoundStat } from "../lib/ipc";
import { roundResult } from "../lib/report";

interface Props {
  matchId: number;
  rounds: RoundStat[];
}

/** One cell per round: won/lost for the tracked side, tracked K-D inside. */
export function RoundStripReport({ matchId, rounds }: Props) {
  const navigate = useNavigate();
  return (
    <div className="report-round-strip" role="list" aria-label="Rounds">
      {rounds.map((r) => {
        const result = roundResult(r);
        return (
          <button
            key={r.number}
            role="listitem"
            className={`rr-cell rr-${result} side-cell-${(r.tracked_side ?? "none").toLowerCase()}`}
            title={`Round ${r.number} — ${result === "unknown" ? r.winner + " won" : "you " + result}; your K-D ${r.kills}-${r.deaths}. Open in replay.`}
            onClick={() => navigate(`/replay/${matchId}?round=${r.number}`)}
          >
            <span className="rr-num">{r.number}</span>
            <span className="rr-kd">
              {r.kills}-{r.deaths}
            </span>
          </button>
        );
      })}
    </div>
  );
}
