import React from "react";
import ReactDOM from "react-dom/client";
import { BrowserRouter, Route, Routes } from "react-router-dom";
import { Toaster } from "sonner";
import "./styles/globals.css";
import Nav from "./components/Nav";
import Home from "./pages/Home";
import Studio from "./pages/Studio";
import TaskPage from "./pages/Task";
import History from "./pages/History";

ReactDOM.createRoot(document.getElementById("root")!).render(
  <React.StrictMode>
    <BrowserRouter>
      <Nav />
      <Routes>
        <Route path="/" element={<Home />} />
        <Route path="/studio" element={<Studio />} />
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
    </BrowserRouter>
  </React.StrictMode>
);
