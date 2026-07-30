import { useState, useEffect } from "react";
import { Link, useLocation } from "react-router-dom";
import { History, Zap, Settings, Github, Scissors, Film } from "lucide-react";
import { SettingsModal } from "./SettingsModal";

export default function Nav() {
  const [settingsOpen, setSettingsOpen] = useState(false);
  const location = useLocation();
  const [isStudio, setIsStudio] = useState(location.pathname === "/studio");

  useEffect(() => {
    if (location.pathname === "/studio") {
      setIsStudio(true);
    } else if (location.pathname.startsWith("/task/")) {
      setIsStudio(sessionStorage.getItem("nova_last_task_type") === "studio");
    } else {
      setIsStudio(false);
    }
  }, [location.pathname]);

  return (
    <>
      <nav className="nav">
        <div className="nav-inner">
          <div style={{ display: "flex", alignItems: "center", gap: "1.25rem" }}>
            <Link to="/" className="nav-logo" style={{ display: "flex", alignItems: "center", gap: "0.6rem" }}>
              <img
                src="/logo.jpg?v=2"
                alt="NovaClip Logo"
                style={{
                  width: 38,
                  height: 38,
                  borderRadius: "10px",
                  objectFit: "cover",
                  boxShadow: "0 0 14px rgba(255, 224, 0, 0.25)",
                }}
              />
              <span>Nova<span>Clip</span></span>
            </Link>

            {/* Mode Switcher Pill */}
            <div style={{ display: "flex", background: "#0c0c0f", padding: "0.2rem", borderRadius: "10px", border: "1px solid rgba(255,255,255,0.08)" }}>
              <Link
                to="/"
                style={{
                  padding: "0.35rem 0.75rem",
                  borderRadius: "8px",
                  fontSize: "0.78rem",
                  fontWeight: 800,
                  textDecoration: "none",
                  display: "flex",
                  alignItems: "center",
                  gap: "0.4rem",
                  background: !isStudio ? "var(--accent)" : "transparent",
                  color: !isStudio ? "#000" : "#aaa",
                  transition: "all 0.15s",
                }}
              >
                <Scissors size={14} /> Nova Clipper
              </Link>
              <Link
                to="/studio"
                style={{
                  padding: "0.35rem 0.75rem",
                  borderRadius: "8px",
                  fontSize: "0.78rem",
                  fontWeight: 800,
                  textDecoration: "none",
                  display: "flex",
                  alignItems: "center",
                  gap: "0.4rem",
                  background: isStudio ? "var(--accent)" : "transparent",
                  color: isStudio ? "#000" : "#aaa",
                  transition: "all 0.15s",
                }}
              >
                <Film size={14} /> Nova Studio
              </Link>
            </div>
          </div>

          <div style={{ display: "flex", gap: "0.5rem", alignItems: "center" }}>
            <button
              className="btn btn-ghost btn-sm"
              onClick={() => setSettingsOpen(true)}
              title="Settings & API Keys"
            >
              <Settings size={15} /> Settings
            </button>
            <Link to="/history" className="btn btn-ghost btn-sm">
              <History size={15} /> History
            </Link>
            <a href="https://github.com/samolubukun/NovaClip" target="_blank" rel="noopener noreferrer" className="btn btn-secondary btn-sm" style={{ display: "flex", alignItems: "center", gap: "0.4rem" }}>
              <Github size={15} /> GitHub
            </a>
          </div>
        </div>
      </nav>

      <SettingsModal isOpen={settingsOpen} onClose={() => setSettingsOpen(false)} />
    </>
  );
}
