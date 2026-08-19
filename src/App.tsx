import { Route, Routes } from "react-router-dom";
import { Library } from "./screens/Library";
import { Replay } from "./screens/Replay";

export default function App() {
  return (
    <Routes>
      <Route path="/" element={<Library />} />
      <Route path="/replay/:matchId" element={<Replay />} />
    </Routes>
  );
}
