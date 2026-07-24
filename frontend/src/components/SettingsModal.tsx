import { useState, useEffect } from "react";
import { Settings, Key, Check, X, ShieldAlert } from "lucide-react";

interface SettingsModalProps {
  isOpen: boolean;
  onClose: () => void;
}

export function SettingsModal({ isOpen, onClose }: SettingsModalProps) {
  const [geminiKey, setGeminiKey] = useState("");
  const [deepgramKey, setDeepgramKey] = useState("");
  const [saved, setSaved] = useState(false);

  useEffect(() => {
    if (isOpen) {
      setGeminiKey(localStorage.getItem("novaclip_gemini_key") || "");
      setDeepgramKey(localStorage.getItem("novaclip_deepgram_key") || "");
      setSaved(false);
    }
  }, [isOpen]);

  if (!isOpen) return null;

  const handleSave = () => {
    localStorage.setItem("novaclip_gemini_key", geminiKey.trim());
    localStorage.setItem("novaclip_deepgram_key", deepgramKey.trim());
    setSaved(true);
    setTimeout(() => {
      onClose();
    }, 600);
  };

  return (
    <div
      style={{
        position: "fixed",
        inset: 0,
        zIndex: 9999,
        background: "rgba(0, 0, 0, 0.75)",
        backdropFilter: "blur(6px)",
        display: "flex",
        alignItems: "center",
        justifyContent: "center",
        padding: "1rem",
      }}
      onClick={onClose}
    >
      <div
        className="card"
        style={{
          width: "100%",
          maxWidth: "480px",
          padding: "1.5rem",
          position: "relative",
          background: "#121214",
          border: "1px solid rgba(255, 255, 255, 0.12)",
          borderRadius: "var(--radius-lg, 12px)",
          boxShadow: "0 20px 40px rgba(0, 0, 0, 0.6)",
        }}
        onClick={(e) => e.stopPropagation()}
      >
        <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between", marginBottom: "1.25rem" }}>
          <div style={{ display: "flex", alignItems: "center", gap: "0.6rem" }}>
            <Settings size={20} color="var(--accent, #FFE000)" />
            <h2 style={{ fontSize: "1.1rem", fontWeight: 700, margin: 0 }}>API Settings (BYOK)</h2>
          </div>
          <button
            onClick={onClose}
            className="btn btn-ghost btn-icon btn-sm"
            style={{ color: "#aaa" }}
          >
            <X size={18} />
          </button>
        </div>

        <p style={{ fontSize: "0.82rem", color: "var(--text-muted, #aaa)", marginBottom: "1.25rem", lineHeight: 1.4 }}>
          Enter your API keys below. Keys are stored safely in your local device storage and used directly for video transcription and AI analysis.
        </p>

        <div style={{ display: "flex", flexDirection: "column", gap: "1.25rem" }}>
          <div>
            <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: "0.4rem" }}>
              <label style={{ display: "flex", alignItems: "center", gap: "0.4rem", fontSize: "0.85rem", fontWeight: 600 }}>
                <Key size={14} color="var(--accent, #FFE000)" /> Google Gemini API Key
              </label>
              <a href="https://ai.google.dev" target="_blank" rel="noopener noreferrer" style={{ fontSize: "0.72rem", color: "var(--accent)", textDecoration: "underline" }}>Get Key ↗</a>
            </div>
            <input
              type="password"
              className="input"
              placeholder="AIzaSy..."
              value={geminiKey}
              onChange={(e) => setGeminiKey(e.target.value)}
              style={{ width: "100%", fontFamily: "monospace", fontSize: "0.85rem" }}
            />
            <span style={{ fontSize: "0.72rem", color: "#888", marginTop: "0.25rem", display: "block" }}>Used for AI video analysis, virality scoring, and clip reasoning</span>
          </div>

          <div>
            <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: "0.4rem" }}>
              <label style={{ display: "flex", alignItems: "center", gap: "0.4rem", fontSize: "0.85rem", fontWeight: 600 }}>
                <Key size={14} color="var(--accent, #FFE000)" /> Deepgram API Key
              </label>
              <a href="https://console.deepgram.com" target="_blank" rel="noopener noreferrer" style={{ fontSize: "0.72rem", color: "var(--accent)", textDecoration: "underline" }}>Get Key ↗</a>
            </div>
            <input
              type="password"
              className="input"
              placeholder="Enter Deepgram key..."
              value={deepgramKey}
              onChange={(e) => setDeepgramKey(e.target.value)}
              style={{ width: "100%", fontFamily: "monospace", fontSize: "0.85rem" }}
            />
            <span style={{ fontSize: "0.72rem", color: "#888", marginTop: "0.25rem", display: "block" }}>Used for Nova-3 word-level audio transcription and karaoke timing</span>
          </div>
        </div>

        <div style={{ marginTop: "1.5rem", display: "flex", justifyContent: "flex-end", gap: "0.5rem" }}>
          <button className="btn btn-secondary btn-sm" onClick={onClose}>
            Cancel
          </button>
          <button
            className="btn btn-primary btn-sm"
            onClick={handleSave}
            style={{ display: "flex", alignItems: "center", gap: "0.4rem" }}
          >
            {saved ? <Check size={16} /> : null}
            {saved ? "Saved!" : "Save Keys"}
          </button>
        </div>
      </div>
    </div>
  );
}
