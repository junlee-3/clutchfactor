import { Route, Routes } from "react-router-dom";
import { Corpus } from "./screens/Corpus";
import { Library } from "./screens/Library";
import { Replay } from "./screens/Replay";
import { Report } from "./screens/Report";
import { Settings } from "./screens/Settings";
import { Trends } from "./screens/Trends";

export default function App() {
  return (
    <Routes>
      <Route path="/" element={<Library />} />
      <Route path="/trends" element={<Trends />} />
      <Route path="/settings" element={<Settings />} />
      <Route path="/corpus" element={<Corpus />} />
      <Route path="/report/:matchId" element={<Report />} />
      <Route path="/replay/:matchId" element={<Replay />} />
    </Routes>
  );
}
