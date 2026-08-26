import type { RoundStat } from "../lib/ipc";
import { roundResult } from "../lib/report";

interface Props {
  rounds: RoundStat[];
  selected: number | null;
  onSelect: (round: number) => void;
}

/** One cell per round: won/lost for the tracked side, tracked K-D inside.
 *  A compact mono grid (frontend-design pass) — win/loss surface tints come
 *  from the §2 derived tokens, and every cell shares the app's one focus
 *  treatment (base.css `:focus-visible`) rather than a bespoke ring.
 *
 *  V1.4: this strip is a selector, not a navigation control — a click picks
 *  the round the Scoreboard below shows; the old click-to-replay behaviour
 *  moved to the Scoreboard's own "Watch round N" link, one click away. */
export function RoundStripReport({ rounds, selected, onSelect }: Props) {
  return (
    <ul className="rpt-round-strip" aria-label="Rounds">
      {rounds.map((r) => {
        const result = roundResult(r);
        const isSelected = r.number === selected;
        return (
          <li key={r.number}>
            <button
              className={`report-round-cell${result === "unknown" ? "" : ` report-round-cell-${result}`}${isSelected ? " report-round-cell-selected" : ""}`}
              title={`Round ${r.number} — ${result === "unknown" ? r.winner + " won" : "you " + result}; your K-D ${r.kills}-${r.deaths}.`}
              aria-pressed={isSelected}
              onClick={() => onSelect(r.number)}
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
