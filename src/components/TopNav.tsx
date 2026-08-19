import { Link, useLocation } from "react-router-dom";

const LINKS = [
  { to: "/", label: "Library" },
  { to: "/trends", label: "Trends" },
  { to: "/corpus", label: "Reference corpus" },
  { to: "/settings", label: "Settings" },
];

/** Shared topbar navigation. The current screen's link renders as plain
 *  text ink so the reader always knows where they are. */
export function TopNav() {
  const { pathname } = useLocation();
  return (
    <nav className="topnav">
      {LINKS.map((l) => (
        <Link
          key={l.to}
          className={`topnav-link${pathname === l.to ? " topnav-link-here" : ""}`}
          to={l.to}
        >
          {l.label}
        </Link>
      ))}
    </nav>
  );
}
