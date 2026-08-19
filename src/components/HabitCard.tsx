import { useNavigate } from "react-router-dom";
import { evidenceUrl } from "../lib/evidence";
import type { HabitReport } from "../lib/ipc";

interface Props {
  habit: HabitReport;
}

export function HabitCard({ habit }: Props) {
  const navigate = useNavigate();
  return (
    <article className="habit-card">
      <header className="hc-head">
        <h4>{habit.title}</h4>
        <span
          className="hc-recurrence"
          title={`Recurred in ${habit.matches_hit} of your last ${habit.window} matches (${habit.total} times total)`}
        >
          {habit.matches_hit} matches
        </span>
      </header>
      <p className="ic-body">{habit.body}</p>
      {habit.evidence.length > 0 && (
        <div className="ic-chips">
          {habit.evidence.map((he, i) => (
            <button
              key={i}
              className="chip"
              title={`Watch on ${he.map}`}
              onClick={() =>
                navigate(
                  evidenceUrl(he.match_id, {
                    round: he.evidence.round,
                    tick_start: he.evidence.tick_start,
                    tick_end: he.evidence.tick_end,
                    focus_players: he.evidence.focus_players,
                    camera_hint: he.evidence.camera_hint ?? undefined,
                  }),
                )
              }
            >
              {he.map.replace(/^de_/, "")} R{he.evidence.round}
            </button>
          ))}
        </div>
      )}
    </article>
  );
}
