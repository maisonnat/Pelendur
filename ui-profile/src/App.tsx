import { BrowserRouter, Routes, Route } from "react-router-dom";
import { lazy, Suspense } from "react";
import Layout from "./components/Layout";
import Dashboard from "./components/Dashboard";
import Skills from "./components/Skills";
import Experiences from "./components/Experiences";
import StarStories from "./components/StarStories";
import Companies from "./components/Companies";
import Settings from "./components/Settings";
import { DebriefPanel as Debrief } from "./components/Debrief";
import Practice from "./components/Practice";

const Constellation = lazy(() => import("./components/Constellation"));

export default function App() {
  return (
    <BrowserRouter>
      <Routes>
        <Route element={<Layout />}>
          <Route index element={<Dashboard />} />
          <Route path="skills" element={<Skills />} />
          <Route path="experiences" element={<Experiences />} />
          <Route path="star-stories" element={<StarStories />} />
          <Route path="companies" element={<Companies />} />
          <Route
            path="constellation"
            element={
              <Suspense fallback={<div className="flex items-center justify-center h-full text-white/20 text-sm">Loading…</div>}>
                <Constellation />
              </Suspense>
            }
          />
          <Route path="settings" element={<Settings />} />
          <Route path="debrief" element={<Debrief />} />
          <Route path="practice" element={<Practice />} />
        </Route>
      </Routes>
    </BrowserRouter>
  );
}
