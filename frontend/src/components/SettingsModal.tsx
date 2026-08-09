import { useState, useEffect } from "react";
import { Settings, Key, Check, X, ShieldAlert } from "lucide-react";

interface SettingsModalProps {
  isOpen: boolean;
  onClose: () => void;
}

export function SettingsModal({ isOpen, onClose }: SettingsModalProps) {
  const [geminiKey, setGeminiKey] = useState("");
  const [deepgramKey, setDeepgramKey] = useState("");
  const [openrouterKey, setOpenrouterKey] = useState("");
  const [openrouterModel, setOpenrouterModel] = useState("openrouter/free");
  const [elevenlabsKey, setElevenlabsKey] = useState("");
  const [pexelsKey, setPexelsKey] = useState("");
  const [pixabayKey, setPixabayKey] = useState("");
  const [wavespeedKey, setWavespeedKey] = useState("");
  const [uploadpostKey, setUploadpostKey] = useState("");
  const [saved, setSaved] = useState(false);

  useEffect(() => {
    if (isOpen) {
      setGeminiKey(localStorage.getItem("novaclip_gemini_key") || "");
      setDeepgramKey(localStorage.getItem("novaclip_deepgram_key") || "");
      setOpenrouterKey(localStorage.getItem("novaclip_openrouter_key") || "");
      setOpenrouterModel(localStorage.getItem("novaclip_openrouter_model") || "openrouter/free");
      setElevenlabsKey(localStorage.getItem("novaclip_elevenlabs_key") || "");
      setPexelsKey(localStorage.getItem("novaclip_pexels_key") || "");
      setPixabayKey(localStorage.getItem("novaclip_pixabay_key") || "");
      setWavespeedKey(localStorage.getItem("novaclip_wavespeed_key") || "");
      setUploadpostKey(localStorage.getItem("novaclip_uploadpost_key") || "");
      setSaved(false);
    }
  }, [isOpen]);

  if (!isOpen) return null;

  const handleSave = () => {
    localStorage.setItem("novaclip_gemini_key", geminiKey.trim());
    localStorage.setItem("novaclip_deepgram_key", deepgramKey.trim());
    localStorage.setItem("novaclip_openrouter_key", openrouterKey.trim());
    localStorage.setItem("novaclip_openrouter_model", openrouterModel.trim());
    localStorage.setItem("novaclip_elevenlabs_key", elevenlabsKey.trim());
    localStorage.setItem("novaclip_pexels_key", pexelsKey.trim());
    localStorage.setItem("novaclip_pixabay_key", pixabayKey.trim());
    localStorage.setItem("novaclip_wavespeed_key", wavespeedKey.trim());
    localStorage.setItem("novaclip_uploadpost_key", uploadpostKey.trim());
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
          maxWidth: "520px",
          maxHeight: "90vh",
          overflowY: "auto",
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
          Enter your API keys below. Keys are stored safely in local browser storage and used directly for video processing, AI models, voiceovers, and media scrapers.
        </p>

        <div style={{ display: "flex", flexDirection: "column", gap: "1.1rem" }}>
          {/* Gemini API Key */}
          <div>
            <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: "0.3rem" }}>
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
              style={{ width: "100%", fontFamily: "monospace", fontSize: "0.82rem" }}
            />
            <span style={{ fontSize: "0.7rem", color: "#888", marginTop: "0.2rem", display: "block" }}>Used for Gemini 3.1 Flash-Lite AI script analysis & virality scoring</span>
          </div>

          {/* OpenRouter API Key */}
          <div>
            <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: "0.3rem" }}>
              <label style={{ display: "flex", alignItems: "center", gap: "0.4rem", fontSize: "0.85rem", fontWeight: 600 }}>
                <Key size={14} color="var(--accent, #FFE000)" /> OpenRouter API Key & Free Models
              </label>
              <a href="https://openrouter.ai" target="_blank" rel="noopener noreferrer" style={{ fontSize: "0.72rem", color: "var(--accent)", textDecoration: "underline" }}>Get Free Key ↗</a>
            </div>
            <input
              type="password"
              className="input"
              placeholder="sk-or-v1-..."
              value={openrouterKey}
              onChange={(e) => setOpenrouterKey(e.target.value)}
              style={{ width: "100%", fontFamily: "monospace", fontSize: "0.82rem", marginBottom: "0.4rem" }}
            />
            <input
              type="text"
              className="input"
              placeholder="Default model: openrouter/free"
              value={openrouterModel}
              onChange={(e) => setOpenrouterModel(e.target.value)}
              style={{ width: "100%", fontFamily: "monospace", fontSize: "0.78rem" }}
            />
            <span style={{ fontSize: "0.7rem", color: "#888", marginTop: "0.2rem", display: "block" }}>Allows using free OpenRouter models (e.g. openrouter/free router, Gemma 4, Nemotron 3, GPT-OSS 20B, DeepSeek R1)</span>
          </div>

          {/* Deepgram API Key */}
          <div>
            <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: "0.3rem" }}>
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
              style={{ width: "100%", fontFamily: "monospace", fontSize: "0.82rem" }}
            />
            <span style={{ fontSize: "0.7rem", color: "#888", marginTop: "0.2rem", display: "block" }}>Used for Nova-3 word-level transcription & Deepgram Aura TTS voiceover</span>
          </div>

          {/* ElevenLabs API Key */}
          <div>
            <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: "0.3rem" }}>
              <label style={{ display: "flex", alignItems: "center", gap: "0.4rem", fontSize: "0.85rem", fontWeight: 600 }}>
                <Key size={14} color="var(--accent, #FFE000)" /> ElevenLabs API Key
              </label>
              <a href="https://elevenlabs.io" target="_blank" rel="noopener noreferrer" style={{ fontSize: "0.72rem", color: "var(--accent)", textDecoration: "underline" }}>Get Key ↗</a>
            </div>
            <input
              type="password"
              className="input"
              placeholder="Enter ElevenLabs API key..."
              value={elevenlabsKey}
              onChange={(e) => setElevenlabsKey(e.target.value)}
              style={{ width: "100%", fontFamily: "monospace", fontSize: "0.82rem" }}
            />
            <span style={{ fontSize: "0.7rem", color: "#888", marginTop: "0.2rem", display: "block" }}>Used for ElevenLabs custom cloned AI voices</span>
          </div>

          {/* Pexels & Pixabay Stock Keys */}
          <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: "0.75rem" }}>
            <div>
              <label style={{ display: "block", fontSize: "0.78rem", fontWeight: 600, color: "#ccc", marginBottom: "0.2rem" }}>Pexels API Key</label>
              <input
                type="password"
                className="input"
                placeholder="Pexels key..."
                value={pexelsKey}
                onChange={(e) => setPexelsKey(e.target.value)}
                style={{ width: "100%", fontFamily: "monospace", fontSize: "0.78rem" }}
              />
            </div>
            <div>
              <label style={{ display: "block", fontSize: "0.78rem", fontWeight: 600, color: "#ccc", marginBottom: "0.2rem" }}>Pixabay API Key</label>
              <input
                type="password"
                className="input"
                placeholder="Pixabay key..."
                value={pixabayKey}
                onChange={(e) => setPixabayKey(e.target.value)}
                style={{ width: "100%", fontFamily: "monospace", fontSize: "0.78rem" }}
              />
            </div>
          </div>

          {/* WaveSpeed API Key */}
          <div>
            <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: "0.3rem" }}>
              <label style={{ display: "flex", alignItems: "center", gap: "0.4rem", fontSize: "0.85rem", fontWeight: 600 }}>
                <Key size={14} color="var(--accent, #FFE000)" /> WaveSpeed API Key
              </label>
              <a href="https://www.wavespeed.ai" target="_blank" rel="noopener noreferrer" style={{ fontSize: "0.72rem", color: "var(--accent)", textDecoration: "underline" }}>Get Key ↗</a>
            </div>
            <input
              type="password"
              className="input"
              placeholder="WaveSpeed key..."
              value={wavespeedKey}
              onChange={(e) => setWavespeedKey(e.target.value)}
              style={{ width: "100%", fontFamily: "monospace", fontSize: "0.82rem" }}
            />
            <span style={{ fontSize: "0.7rem", color: "#888", marginTop: "0.2rem", display: "block" }}>Used for AI B-Roll clips (Seedance), AI Shorts (Flux 2 Pro, AI Talking Photos, InfiniteTalk) & Lyria music</span>
          </div>

          {/* Upload-Post API Key */}
          <div>
            <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: "0.3rem" }}>
              <label style={{ display: "flex", alignItems: "center", gap: "0.4rem", fontSize: "0.85rem", fontWeight: 600 }}>
                <Key size={14} color="var(--accent, #FFE000)" /> Upload-Post API Key
              </label>
              <a href="https://app.upload-post.com/api-keys" target="_blank" rel="noopener noreferrer" style={{ fontSize: "0.72rem", color: "var(--accent)", textDecoration: "underline" }}>Get Key ↗</a>
            </div>
            <input
              type="password"
              className="input"
              placeholder="Upload-Post key..."
              value={uploadpostKey}
              onChange={(e) => setUploadpostKey(e.target.value)}
              style={{ width: "100%", fontFamily: "monospace", fontSize: "0.82rem" }}
            />
            <span style={{ fontSize: "0.7rem", color: "#888", marginTop: "0.2rem", display: "block" }}>Used for one-click publishing to TikTok, Instagram Reels, and YouTube Shorts (10 free uploads/month)</span>
          </div>
        </div>

        <div style={{ marginTop: "1.25rem", display: "flex", justifyContent: "flex-end", gap: "0.5rem" }}>
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
