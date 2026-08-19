import type { PlayerTrack } from "../replay/interp";
import { stateAt } from "../replay/interp";

interface Props {
  tracks: PlayerTrack[];
  names: Map<string, string>;
  sides: Map<string, "CT" | "T">;
  tick: number;
}

function weaponLabel(w: string | null): string {
  if (!w) return "";
  return w.replace(/^weapon_/, "");
}

export function RosterPanel({ tracks, names, sides, tick }: Props) {
  const bySide = (side: "CT" | "T") =>
    tracks
      .filter((t) => sides.get(t.steamid) === side)
      .sort((a, b) =>
        (names.get(a.steamid) ?? "").localeCompare(names.get(b.steamid) ?? ""),
      );

  const renderSide = (side: "CT" | "T") => (
    <div className={`roster-side roster-${side.toLowerCase()}`}>
      <h3>{side}</h3>
      {bySide(side).map((t) => {
        const s = stateAt(t, tick);
        const hp = s?.isAlive ? s.health : 0;
        return (
          <div
            key={t.steamid}
            className={`roster-row ${hp === 0 ? "dead" : ""}`}
          >
            <span className="roster-name">{names.get(t.steamid)}</span>
            <span className="roster-weapon">
              {hp > 0 ? weaponLabel(s?.weapon ?? null) : ""}
            </span>
            <span className="roster-hp-num">{hp > 0 ? hp : ""}</span>
            <div className="hp-track" aria-hidden="true">
              <div
                className="hp-fill"
                style={{ width: `${Math.max(0, Math.min(100, hp))}%` }}
              />
            </div>
          </div>
        );
      })}
    </div>
  );

  return (
    <div className="roster-panel">
      {renderSide("CT")}
      {renderSide("T")}
    </div>
  );
}
