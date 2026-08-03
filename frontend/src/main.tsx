import React, { useEffect } from "react";
import ReactDOM from "react-dom/client";
import { MemoryRouter, Route, Routes, useLocation } from "react-router-dom";
import { Toaster } from "sonner";
import "./styles/globals.css";
import Nav from "./components/Nav";
import Home from "./pages/Home";
import Studio from "./pages/Studio";
import NovaEdit from "./pages/NovaEdit";
import TaskPage from "./pages/Task";
import History from "./pages/History";

const INTERNAL_ROUTE_KEY = "novaclip_internal_route";

function RoutePersistence() {
  const location = useLocation();

  useEffect(() => {
    sessionStorage.setItem(INTERNAL_ROUTE_KEY, `${location.pathname}${location.search}`);
  }, [location.pathname, location.search]);

  return null;
}

const browserRoute = `${window.location.pathname}${window.location.search}`;
const initialRoute = browserRoute !== "/"
  ? browserRoute
  : sessionStorage.getItem(INTERNAL_ROUTE_KEY) || "/";

if (window.location.pathname !== "/" || window.location.search) {
  window.history.replaceState(null, "", "/");
}

ReactDOM.createRoot(document.getElementById("root")!).render(
  <React.StrictMode>
    <MemoryRouter initialEntries={[initialRoute]}>
      <RoutePersistence />
      <Nav />
      <Routes>
        <Route path="/" element={<Home />} />
        <Route path="/studio" element={<Studio />} />
        <Route path="/novaedit" element={<NovaEdit />} />
        <Route path="/task/:id" element={<TaskPage />} />
        <Route path="/history" element={<History />} />
      </Routes>
      <Toaster
        theme="dark"
        position="bottom-right"
        toastOptions={{
          style: {
            background: "var(--bg-elevated)",
            border: "1px solid var(--border)",
            color: "var(--text-primary)",
          },
        }}
      />
    </MemoryRouter>
  </React.StrictMode>
);
