import { Route, Routes } from "react-router-dom";
import { Library } from "./screens/Library";

export default function App() {
  return (
    <Routes>
      <Route path="/" element={<Library />} />
    </Routes>
  );
}
