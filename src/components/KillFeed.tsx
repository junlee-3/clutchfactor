import type { KillInfo } from "../lib/ipc";
import { Card } from "./ui/Card";

interface Props {
  kills: KillInfo[]; // this round's kills, sorted by tick
  tick: number;
  tickrate: number;
  names: Map<string, string>;
  sides: Map<string, "CT" | "T">;
  onJump: (tick: number) => void;
}

// The tape's other side-rail Card (design-system.md §9). Each row is a
// single button that seeks the canvas straight to that kill — the click
// jumps to the tape, same contract as an evidence chip, just not dashed
// (§5 reserves the dashed grammar for chips/rail/the teammate line
// specifically, not every clickable row in the app).
export function KillFeed({ kills, tick, tickrate, names, sides, onJump }: Props) {
  const shown = kills.filter((k) => k.tick <= tick).slice(-6);
  const sideClass = (sid: string | null) =>
    sid ? (sides.get(sid) === "T" ? "rpl-side-t" : "rpl-side-ct") : "rpl-side-none";
  const name = (sid: string | null) =>
    (sid && names.get(sid)) || (sid ? sid : "world");

  return (
    <Card>
      <h4 className="type-micro rpl-panel-label">Kill feed</h4>
      {shown.length === 0 && <p className="rpl-kf-empty type-body">No kills yet</p>}
      {shown.map((k, i) => (
        <button
          key={`${k.tick}-${i}`}
          className={`rpl-kf-row type-data${tick - k.tick < 3 * tickrate ? " rpl-kf-row-recent" : ""}`}
          title="Jump to this kill"
          onClick={() => onJump(k.tick - 2 * tickrate)}
        >
          <span className={sideClass(k.attacker)}>{name(k.attacker)}</span>
          <span className="rpl-kf-weapon">
            {k.weapon}
            {k.headshot ? " ⌖" : ""}
          </span>
          <span className={sideClass(k.victim)}>{name(k.victim)}</span>
        </button>
      ))}
    </Card>
  );
}
