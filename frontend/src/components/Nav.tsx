import { useState } from "react";
import { Link } from "react-router-dom";
import { History, Zap, Settings, Github } from "lucide-react";
import { SettingsModal } from "./SettingsModal";

export default function Nav() {
  const [settingsOpen, setSettingsOpen] = useState(false);

  return (
    <>
      <nav className="nav">
        <div className="nav-inner">
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
