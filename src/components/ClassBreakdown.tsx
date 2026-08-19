import type { DeathClassRow } from "../lib/ipc";
import { classLabel, GOOD_NEWS_CLASS, HYGIENE_CLASSES } from "../lib/report";

interface Props {
  rows: DeathClassRow[];
  class13SharePct: number;
  classesNotBuilt: number[];
}

/** Horizontal bars: one muted hue for preventable classes, win-green for the
 *  "fair duel" class (good news), hollow for hygiene/unclassified. Labels
 *  carry identity — color is never the only encoding. */
export function ClassBreakdown({ rows, class13SharePct, classesNotBuilt }: Props) {
  const counts = new Map<number, number>();
  for (const r of rows) counts.set(r.class_id, (counts.get(r.class_id) ?? 0) + 1);
  const entries = [...counts.entries()].sort((a, b) => b[1] - a[1]);
  const max = entries[0]?.[1] ?? 1;

  return (
    <section className="class-breakdown" aria-label="How you died">
      <h3>How you died</h3>
      {entries.map(([id, count]) => {
        const kind =
          id === GOOD_NEWS_CLASS
            ? "good"
            : HYGIENE_CLASSES.includes(id)
              ? "hygiene"
              : "preventable";
        return (
          <div key={id} className="cb-row" title={`${classLabel(id)}: ${count} of ${rows.length} deaths`}>
            <span className="cb-label">{classLabel(id)}</span>
            <span className="cb-count">{count}</span>
            <div className="cb-track" aria-hidden="true">
              <div
                className={`cb-fill cb-${kind}`}
                style={{ width: `${(count / max) * 100}%` }}
              />
            </div>
          </div>
        );
      })}
      <p className="cb-note">
        {class13SharePct}% were fair duels you lost on mechanics — good news:
        the rest had a fixable cause.
      </p>
      {classesNotBuilt.length > 0 && (
        <p className="cb-honesty">
          Not yet detected: {classesNotBuilt.map(classLabel).join(", ")}. Some
          deaths above may belong there.
        </p>
      )}
    </section>
  );
}
