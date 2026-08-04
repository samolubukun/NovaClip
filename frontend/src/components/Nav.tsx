import { useState, useEffect } from "react";
import { Link, useLocation } from "react-router-dom";
import { History, Settings, Github, Scissors, Film, Wand2, Megaphone } from "lucide-react";
import { SettingsModal } from "./SettingsModal";

type Mode = "clipper" | "studio" | "novaedit" | "repurpose";

export default function Nav() {
  const [settingsOpen, setSettingsOpen] = useState(false);
  const location = useLocation();
  const historyActive = location.pathname === "/history";
  const [mode, setMode] = useState<Mode>(() => {
    if (location.pathname === "/studio") return "studio";
    if (location.pathname === "/novaedit") return "novaedit";
    if (location.pathname === "/repurpose") return "repurpose";
    return "clipper";
  });

  useEffect(() => {
    const syncMode = () => {
      if (location.pathname === "/studio") {
      setMode("studio");
      } else if (location.pathname === "/novaedit") {
      setMode("novaedit");
      } else if (location.pathname === "/repurpose") {
      setMode("repurpose");
      } else if (location.pathname.startsWith("/task/")) {
      const t = sessionStorage.getItem("nova_last_task_type");
      setMode(t === "studio" ? "studio" : t === "agentic" ? "novaedit" : t === "repurpose" ? "repurpose" : "clipper");
      } else if (location.pathname !== "/history") {
      setMode("clipper");
      }
    };

    syncMode();
    window.addEventListener("nova-task-type-change", syncMode);
    return () => window.removeEventListener("nova-task-type-change", syncMode);
  }, [location.pathname]);

  const tabStyle = (tab: Mode) => {
    const active = !historyActive && mode === tab;
    const colors = {
      clipper: { background: "var(--accent)", color: "#000", glow: "rgba(255,224,0,0.22)" },
      novaedit: { background: "#22d3ee", color: "#001014", glow: "rgba(34,211,238,0.22)" },
      studio: { background: "#8b5cf6", color: "#fff", glow: "rgba(139,92,246,0.28)" },
      repurpose: { background: "#f43f5e", color: "#fff", glow: "rgba(244,63,94,0.28)" },
    }[tab];

    return {
      padding: "0.35rem 0.75rem",
      borderRadius: "8px",
      fontSize: "0.78rem",
      fontWeight: 800,
      textDecoration: "none",
      display: "flex",
      alignItems: "center",
      gap: "0.4rem",
      background: active ? colors.background : "transparent",
      color: active ? colors.color : "#aaa",
      boxShadow: active ? `0 0 14px ${colors.glow}` : "none",
      transition: "all 0.15s",
    } as const;
  };

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
              <Link to="/" style={tabStyle("clipper")}>
                <Scissors size={14} /> Nova Clipper
              </Link>
              <Link to="/studio" style={tabStyle("studio")}>
                <Film size={14} /> Nova Studio
              </Link>
              <Link to="/novaedit" style={tabStyle("novaedit")}>
                <Wand2 size={14} /> Nova Edit
              </Link>
              <Link to="/repurpose" style={tabStyle("repurpose")}>
                <Megaphone size={14} /> Nova Repurpose
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
            <Link
              to="/history"
              className="btn btn-ghost btn-sm"
              style={{
                background: historyActive ? "rgba(255,255,255,0.12)" : undefined,
                color: historyActive ? "#fff" : undefined,
                border: historyActive ? "1px solid rgba(255,255,255,0.28)" : undefined,
                boxShadow: historyActive ? "0 0 14px rgba(255,255,255,0.16)" : "none",
              }}
            >
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
