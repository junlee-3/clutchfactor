import { useNavigate } from "react-router-dom";
import type { RoundStat } from "../lib/ipc";
import { roundResult } from "../lib/report";

interface Props {
  matchId: number;
  rounds: RoundStat[];
}

/** One cell per round: won/lost for the tracked side, tracked K-D inside.
 *  A compact mono grid (frontend-design pass) — win/loss surface tints come
 *  from the §2 derived tokens, and every cell shares the app's one focus
 *  treatment (base.css `:focus-visible`) rather than a bespoke ring. */
export function RoundStripReport({ matchId, rounds }: Props) {
  const navigate = useNavigate();
  return (
    <ul className="rpt-round-strip" aria-label="Rounds">
      {rounds.map((r) => {
        const result = roundResult(r);
        return (
          <li key={r.number}>
            <button
              className={`report-round-cell${result === "unknown" ? "" : ` report-round-cell-${result}`}`}
              title={`Round ${r.number} — ${result === "unknown" ? r.winner + " won" : "you " + result}; your K-D ${r.kills}-${r.deaths}. Open in replay.`}
              onClick={() => navigate(`/replay/${matchId}?round=${r.number}`)}
            >
              <span className="type-micro">{r.number}</span>
              <span className="type-data">
                {r.kills}-{r.deaths}
              </span>
            </button>
          </li>
        );
      })}
    </ul>
  );
}
