import { Route, Routes, useNavigate } from "react-router-dom";
import { AppShell } from "./components/ui/AppShell";
import { EmptyState } from "./components/ui/EmptyState";
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

export default function App() {
  return (
    <Routes>
      <Route element={<AppShell />}>
        <Route path="/" element={<Library />} />
        <Route path="/trends" element={<Trends />} />
        <Route path="/watches" element={<Watches />} />
        <Route path="/settings" element={<Settings />} />
        <Route path="/corpus" element={<Corpus />} />
        <Route path="/report/:matchId" element={<Report />} />
        <Route path="/replay/:matchId" element={<Replay />} />
        <Route path="*" element={<NotFound />} />
      </Route>
    </Routes>
  );
}
