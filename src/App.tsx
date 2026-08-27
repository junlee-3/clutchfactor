import type { ReactNode } from "react";
import { Route, Routes, useLocation, useNavigate } from "react-router-dom";
import { AppShell } from "./components/ui/AppShell";
import { EmptyState } from "./components/ui/EmptyState";
import { ErrorBoundary } from "./components/ui/ErrorBoundary";
import { Corpus } from "./screens/Corpus";
import { Library } from "./screens/Library";
import { Replay } from "./screens/Replay";
import { Report } from "./screens/Report";
import { Settings } from "./screens/Settings";
import { Trends } from "./screens/Trends";
import { Watches } from "./screens/Watches";

function NotFound() {
  const navigate = useNavigate();
  return (
    <div className="route-not-found">
      <EmptyState
        title="That page doesn't exist"
        body="The link you followed doesn't match a screen in ClutchFactor."
        action={{ label: "Back to library", onClick: () => navigate("/") }}
      />
    </div>
  );
}

// Route-level safety net (spec §2, ErrorBoundary.tsx): keyed on the
// pathname so a crash on one screen never survives a navigation to
// another — the boundary clears itself as soon as the route changes.
function RouteBoundary({ children }: { children: ReactNode }) {
  const { pathname } = useLocation();
  return <ErrorBoundary resetKey={pathname}>{children}</ErrorBoundary>;
}

export default function App() {
  return (
    <Routes>
      <Route element={<AppShell />}>
        <Route path="/" element={<RouteBoundary><Library /></RouteBoundary>} />
        <Route path="/trends" element={<RouteBoundary><Trends /></RouteBoundary>} />
        <Route path="/watches" element={<RouteBoundary><Watches /></RouteBoundary>} />
        <Route path="/settings" element={<RouteBoundary><Settings /></RouteBoundary>} />
        <Route path="/corpus" element={<RouteBoundary><Corpus /></RouteBoundary>} />
        <Route path="/report/:matchId" element={<RouteBoundary><Report /></RouteBoundary>} />
        <Route path="/replay/:matchId" element={<RouteBoundary><Replay /></RouteBoundary>} />
        <Route path="*" element={<RouteBoundary><NotFound /></RouteBoundary>} />
      </Route>
    </Routes>
  );
}
