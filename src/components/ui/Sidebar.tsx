import { NavLink } from "react-router-dom";
import type { TrackedPlayer } from "../../lib/ipc";
import { useTrackedPlayer, useTrackedProfile } from "../../lib/queries";
import type { ShellMode } from "../../lib/shellMode";
import { trackedInitials, trackedLabel } from "../../lib/trackedPlayer";

interface NavItem {
  to: string;
  label: string;
  short: string;
  end?: boolean;
}

// Text-first, no icon library (design-system.md §7). `short` is the
// two-letter glyph shown in rail mode; `label` is always the accessible
// name (visible text in full mode, `title` tooltip in rail mode).
const NAV_ITEMS: NavItem[] = [
  { to: "/", label: "Library", short: "Li", end: true },
  { to: "/trends", label: "Trends", short: "Tr" },
  { to: "/watches", label: "Watches", short: "Wa" },
  { to: "/corpus", label: "Corpus", short: "Co" },
  { to: "/settings", label: "Settings", short: "Se" },
];

interface SidebarProps {
  mode: ShellMode;
}

// One shell for every screen (design-system.md §7). `full` (216px):
// wordmark + nav + footer tracked-player chip. `rail` (56px, immersive
// screens): "CF" glyph + two-letter nav, no footer — the tape stays hero.
export function Sidebar({ mode }: SidebarProps) {
  // The store answers instantly; the Steam profile lands later, or not at
  // all, and only ever adds the avatar and a fresher name.
  const tracked = useTrackedPlayer();
  const profile = useTrackedProfile();
  const player = profile.data ?? tracked.data;
  const rail = mode === "rail";

  return (
    <nav className={`sidebar${rail ? " sidebar-rail" : ""}`} aria-label="Primary">
      <div className="sidebar-wordmark type-title">{rail ? "CF" : "ClutchFactor"}</div>
      <ul className="sidebar-nav">
        {NAV_ITEMS.map((item) => (
          <li key={item.to}>
            <NavLink
              to={item.to}
              end={item.end}
              className={({ isActive }) =>
                `sidebar-link${isActive ? " sidebar-link-active" : ""}`
              }
              title={rail ? item.label : undefined}
            >
              {rail ? item.short : item.label}
            </NavLink>
          </li>
        ))}
      </ul>
      {!rail && player && (
        <div className="sidebar-footer">
          <TrackedPlayerChip player={player} />
        </div>
      )}
    </nav>
  );
}

// The footer profile: avatar + name, never a raw SteamID64 (issue #39). The
// steamid stays reachable in the tooltip — it is still the thing you paste
// into a bug report. `avatar` arrives as an inlined data: URI, so there is no
// network request here and no layout shift when it resolves; when it is
// missing (offline, private profile) the initials placeholder holds the same
// box. The eyebrow says "Tracking" because the detection is automatic and the
// user may need to correct it in Settings.
function TrackedPlayerChip({ player }: { player: TrackedPlayer }) {
  const label = trackedLabel(player);
  return (
    <div
      className="tracked-player"
      title={`Tracked player (auto-detected) — ${player.steamid}`}
    >
      {player.avatar ? (
        <img className="tracked-avatar" src={player.avatar} alt="" />
      ) : (
        <span className="tracked-avatar tracked-avatar-empty" aria-hidden="true">
          {trackedInitials(label)}
        </span>
      )}
      <span className="tracked-id">
        <span className="type-micro">Tracking</span>
        <span className="tracked-name">{label}</span>
      </span>
    </div>
  );
}
