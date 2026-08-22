import { NavLink } from "react-router-dom";
import { useTrackedPlayer } from "../../lib/queries";
import type { ShellMode } from "../../lib/shellMode";
import { Chip } from "./Chip";

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
  const tracked = useTrackedPlayer();
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
      {!rail && tracked.data && (
        <div className="sidebar-footer">
          <Chip title="Tracked player (auto-detected)">tracking {tracked.data}</Chip>
        </div>
      )}
    </nav>
  );
}
