import { Outlet, useLocation } from "react-router-dom";
import { Sidebar, type ShellMode } from "./Sidebar";

// Immersive screens collapse the sidebar to a 56px rail so the tape stays
// hero (design-system.md §7) — everything else about the shell is identical.
const IMMERSIVE_PREFIXES = ["/replay", "/report"];

function shellMode(pathname: string): ShellMode {
  return IMMERSIVE_PREFIXES.some((p) => pathname.startsWith(p)) ? "rail" : "full";
}

// The one shell every screen renders through (design-system.md §7): fixed
// left sidebar + content, chosen by route. Screens compose only their inner
// content via <Outlet/> — nav, wordmark, and the tracked-player chip live
// here exactly once.
export function AppShell() {
  const { pathname } = useLocation();
  const mode = shellMode(pathname);
  const rail = mode === "rail";

  return (
    <div className={`shell${rail ? " shell-rail" : ""}`}>
      <Sidebar mode={mode} />
      <main className={rail ? "shell-content-immersive" : "shell-content"}>
        <Outlet />
      </main>
    </div>
  );
}
