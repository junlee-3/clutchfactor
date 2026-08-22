import type { PlayerTrack } from "../replay/interp";
import { stateAt } from "../replay/interp";
import { Card } from "./ui/Card";

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

// The tape's side-rail Card (design-system.md §9): one Card, two side
// groups. Group labels stay plain type-micro — never CT/T colored — because
// §9 reserves side hues for names and HP fills only, so color never becomes
// the only way a header conveys which side it is.
export function RosterPanel({ tracks, names, sides, tick }: Props) {
  const bySide = (side: "CT" | "T") =>
    tracks
      .filter((t) => sides.get(t.steamid) === side)
      .sort((a, b) =>
        (names.get(a.steamid) ?? "").localeCompare(names.get(b.steamid) ?? ""),
      );

  const renderSide = (side: "CT" | "T") => {
    const sideClass = side === "T" ? "rpl-side-t" : "rpl-side-ct";
    return (
      <div className="rpl-roster-side">
        <h4 className="type-micro rpl-roster-side-label">{side}</h4>
        {bySide(side).map((t) => {
          const s = stateAt(t, tick);
          const hp = s?.isAlive ? s.health : 0;
          return (
            <div
              key={t.steamid}
              className={`rpl-roster-row type-data${hp === 0 ? " rpl-roster-row-dead" : ""}`}
            >
              <span className={`rpl-roster-name ${sideClass}`}>
                {names.get(t.steamid)}
              </span>
              <span className="rpl-roster-weapon">
                {hp > 0 ? weaponLabel(s?.weapon ?? null) : ""}
              </span>
              <span className="rpl-roster-hp">{hp > 0 ? hp : ""}</span>
              <div className="rpl-hp-track" aria-hidden="true">
                <div
                  className={`rpl-hp-fill rpl-hp-fill-${side.toLowerCase()}`}
                  style={{ width: `${Math.max(0, Math.min(100, hp))}%` }}
                />
              </div>
            </div>
          );
        })}
      </div>
    );
  };

  return (
    <Card>
      {renderSide("CT")}
      {renderSide("T")}
    </Card>
  );
}
