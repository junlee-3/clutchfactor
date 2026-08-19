import { Route, Routes } from "react-router-dom";
import { Corpus } from "./screens/Corpus";
import { Library } from "./screens/Library";
import { Replay } from "./screens/Replay";
import { Report } from "./screens/Report";
import { Trends } from "./screens/Trends";

export default function App() {
  return (
    <Routes>
      <Route path="/" element={<Library />} />
      <Route path="/trends" element={<Trends />} />
      <Route path="/corpus" element={<Corpus />} />
      <Route path="/report/:matchId" element={<Report />} />
      <Route path="/replay/:matchId" element={<Replay />} />
    </Routes>
  );
}
