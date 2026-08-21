import { Outlet, useLocation } from "react-router-dom";
import { Sidebar } from "./Sidebar";
import { shellMode } from "../../lib/shellMode";

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
