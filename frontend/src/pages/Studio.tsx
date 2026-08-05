import { useState, useRef } from "react";
import { useNavigate } from "react-router-dom";
import { motion } from "framer-motion";
import {
  Film, Sparkles, Wand2, Sliders, Play, RotateCcw, Upload, Image as ImageIcon,
  Check, Volume2, Globe, Layers, Video, Zap, MessageSquare, Search
} from "lucide-react";
import { toast } from "sonner";
import { api } from "../lib/api";
import { GEMINI_MODELS, OPENROUTER_MODELS, type LlmProvider } from "../lib/llmModels";

const TTS_PROVIDERS = [
  { id: "edge-tts", label: "Edge-TTS (Free Neural Voices)", desc: "10+ Languages, Zero API cost" },
  { id: "elevenlabs", label: "ElevenLabs API", desc: "Cloned & custom voices (Requires API Key)" },
  { id: "deepgram-aura", label: "Deepgram TTS", desc: "Low latency streaming AI voices" },
];

const ELEVENLABS_VOICES = [
  { id: "EXAVITQu4vr4xnSDxMaL", name: "Bella", gender: "Female", tone: "Soft, warm, friendly" },
  { id: "ErXwobaYiN019PkySvjV", name: "Antoni", gender: "Male", tone: "Well-rounded, engaging" },
  { id: "VR6AewLTigWG4xSOukaG", name: "Arnold", gender: "Male", tone: "Crisp, articulate" },
  { id: "pNInz6obpgDQGcFmaJgB", name: "Adam", gender: "Male", tone: "Deep, smooth narration" },
  { id: "JBFqnCBsd6RMkjVDRZzb", name: "George", gender: "Male", tone: "Warm, British storyteller" },
  { id: "cgSgspJ2msm6clMCkdW9", name: "Jessica", gender: "Female", tone: "Playful, trendy, bright" },
];

const DEEPGRAM_VOICES = [
  { id: "aura-2-asteria-en", label: "Aura 2: Asteria (Female) ★" },
  { id: "aura-2-athena-en", label: "Aura 2: Athena (Female) ★" },
  { id: "aura-2-luna-en", label: "Aura 2: Luna (Female) ★" },
  { id: "aura-2-stella-en", label: "Aura 2: Stella (Female) ★" },
  { id: "aura-2-hera-en", label: "Aura 2: Hera (Female) ★" },
  { id: "aura-2-orion-en", label: "Aura 2: Orion (Male) ★" },
  { id: "aura-2-arcas-en", label: "Aura 2: Arcas (Male) ★" },
  { id: "aura-2-perseus-en", label: "Aura 2: Perseus (Male) ★" },
  { id: "aura-2-angus-en", label: "Aura 2: Angus (Male) ★" },
  { id: "aura-2-orpheus-en", label: "Aura 2: Orpheus (Male) ★" },
  { id: "aura-asteria-en", label: "Aura 1: Asteria (Female)" },
  { id: "aura-athena-en", label: "Aura 1: Athena (Female)" },
  { id: "aura-luna-en", label: "Aura 1: Luna (Female)" },
  { id: "aura-stella-en", label: "Aura 1: Stella (Female)" },
  { id: "aura-hera-en", label: "Aura 1: Hera (Female)" },
  { id: "aura-orion-en", label: "Aura 1: Orion (Male)" },
  { id: "aura-arcas-en", label: "Aura 1: Arcas (Male)" },
  { id: "aura-perseus-en", label: "Aura 1: Perseus (Male)" },
  { id: "aura-angus-en", label: "Aura 1: Angus (Male)" },
  { id: "aura-orpheus-en", label: "Aura 1: Orpheus (Male)" },
];

export default function Studio() {
  const navigate = useNavigate();

  const [script, setScript] = useState("");
  const [topic, setTopic] = useState("");
  const [scriptStatus, setScriptStatus] = useState<"idle" | "loading" | "done" | "error">("idle");
  const [scriptError, setScriptError] = useState("");
  const [aspectRatio, setAspectRatio] = useState("9:16");
  const [llmProvider, setLlmProvider] = useState<LlmProvider>("gemini");
  const [llmModel, setLlmModel] = useState("gemini-3.1-flash-lite");
  const [customLlmModel, setCustomLlmModel] = useState("");
  const [ttsProvider, setTtsProvider] = useState("edge-tts");
  const [voiceName, setVoiceName] = useState("en-US-ChristopherNeural");
  const [elevenVoiceId, setElevenVoiceId] = useState("");
  const [deepgramVoice, setDeepgramVoice] = useState("aura-asteria-en");

  const [duration, setDuration] = useState("60");
  const [source, setSource] = useState("all");
  const [mediaType, setMediaType] = useState<"video" | "photo">("video");
  const [vibe, setVibe] = useState("aesthetic");
  const [subtitleStyle, setSubtitleStyle] = useState("high_retention");
  const [bgMusic, setBgMusic] = useState("none");
  const [loading, setLoading] = useState(false);
  const [mode, setMode] = useState<"stock" | "ai">(() => (sessionStorage.getItem("nova_studio_mode") === "ai" ? "ai" : "stock"));

  const handleModeChange = (m: "stock" | "ai") => {
    setMode(m);
    if (m === "ai" && ttsProvider === "edge-tts") {
      setTtsProvider("deepgram-aura");
    }
    sessionStorage.setItem("nova_studio_mode", m);
    window.dispatchEvent(new Event("nova-studio-mode-change"));
  };

  const [watermarkFile, setWatermarkFile] = useState<File | null>(null);
  const [watermarkPreviewUrl, setWatermarkPreviewUrl] = useState<string | null>(null);
  const [watermarkPosition, setWatermarkPosition] = useState("top_right");
  const [watermarkOpacity, setWatermarkOpacity] = useState(80);
  const watermarkInputRef = useRef<HTMLInputElement>(null);

  const handleGenerateScriptWithAI = async () => {
    if (!topic.trim()) {
      setScriptStatus("error");
      setScriptError("Please enter a topic first");
      return;
    }
    setScriptStatus("loading");
    setScriptError("");
    try {
      const apiKey = localStorage.getItem("novaclip_gemini_key") || localStorage.getItem("novaclip_openrouter_key") || "";
      const res = await fetch("/studio/generate_script", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          topic: topic.trim(),
          vibe,
          duration: Number(duration),
         llm_provider: llmProvider === "gemini" ? llmModel : llmModel === "custom" ? customLlmModel : llmModel,
          api_key: apiKey,
        }),
      });
      const data = await res.json();
      if (!res.ok) throw new Error(data.error || "Script generation failed");
      setScript(data.script || "");
      setScriptStatus("done");
    } catch (e: any) {
      setScriptStatus("error");
      setScriptError(e.message || "Failed to generate script");
    }
  };

  const handleCreateStudioVideo = async () => {
    if (!script.trim()) {
      toast.error("Please enter or generate a video script first");
      return;
    }
    if (mode === "ai" && !localStorage.getItem("novaclip_wavespeed_key")) {
      toast.error("AI B-Roll requires a WaveSpeed API key. Add it in Settings first.");
      return;
    }
    setLoading(true);
    try {
      const videoTitle = topic.trim()
        ? topic.trim().slice(0, 80)
        : script.trim().split(/\s+/).slice(0, 10).join(" ").slice(0, 80);

      const payload = {
        script: script.trim(),
        mode,
        aspect_ratio: aspectRatio,
        llm_provider: llmProvider === "gemini" ? llmModel : llmModel === "custom" ? customLlmModel : llmModel,
        tts_provider: ttsProvider,
        voice: ttsProvider === "elevenlabs" ? elevenVoiceId : ttsProvider === "deepgram-aura" ? deepgramVoice : voiceName,
        duration: Number(duration),
        source,
        media_type: mediaType,
        vibe,
        subtitle_style: subtitleStyle,
        bg_music: bgMusic,
        api_keys: {
          gemini_key: localStorage.getItem("novaclip_gemini_key") || "",
          openrouter_key: localStorage.getItem("novaclip_openrouter_key") || "",
          deepgram_key: localStorage.getItem("novaclip_deepgram_key") || "",
          elevenlabs_key: localStorage.getItem("novaclip_elevenlabs_key") || "",
          pexels_key: localStorage.getItem("novaclip_pexels_key") || "",
          pixabay_key: localStorage.getItem("novaclip_pixabay_key") || "",
          wavespeed_key: localStorage.getItem("novaclip_wavespeed_key") || "",
        }
      };

      const task = await api.createTask({
        url: "studio://faceless",
        source_title: videoTitle,
        aspect_ratio: aspectRatio,
        num_clips: 1,
        llm_provider: llmProvider === "gemini" ? llmModel : llmModel === "custom" ? customLlmModel : llmModel,
        font_family: "THEBOLDFONT",
        font_size: 28,
        font_color: "#ffffff",
        highlight_color: "#ffe000",
        caption_animation: "word_pop",
        auto_emojis: true,
        watermark_position: watermarkPosition,
        watermark_opacity: watermarkOpacity,
        studio_payload: payload,
      });

      if (watermarkFile) {
        api.uploadWatermark(task.task_id, watermarkFile).catch(() => {});
      }

      sessionStorage.setItem("nova_last_task_type", "studio");
      toast.success("Nova Studio task created! Check the task page for progress.");
      navigate(`/task/${task.task_id}`);
    } catch (e: any) {
      toast.error(e.message || "Failed to start studio task");
    } finally {
      setLoading(false);
    }
  };

  return (
    <div
      style={{
        maxWidth: "1280px",
        margin: "0 auto",
        padding: "1.5rem 1rem",
        ["--accent" as any]: mode === "ai" ? "#d946ef" : "#8b5cf6",
        ["--accent-rgb" as any]: mode === "ai" ? "217,70,239" : "139,92,246",
      }}
    >
      {/* Header Banner */}
      <div style={{ textAlign: "center", marginBottom: "2.5rem" }}>
        <div style={{ display: "inline-flex", alignItems: "center", gap: "0.5rem", background: "rgba(var(--accent-rgb),0.1)", border: "1px solid rgba(var(--accent-rgb),0.3)", padding: "0.4rem 1rem", borderRadius: "20px", color: "var(--accent)", fontSize: "0.78rem", fontWeight: 800, textTransform: "uppercase", letterSpacing: "0.08em", marginBottom: "2.3rem" }}>
          <Film size={14} /> Nova Studio: Faceless AI Creator
        </div>
        <h1 style={{ fontSize: "clamp(1.8rem, 3.8vw, 3.2rem)", fontWeight: 900, lineHeight: 1.1, marginBottom: "0.75rem", letterSpacing: "-0.03em", color: "#fff" }}>
          Generate Viral <span style={{ color: "var(--accent)", textShadow: "0 0 35px rgba(var(--accent-rgb),0.3)" }}>Faceless AI Videos</span>
        </h1>
        <p style={{ fontSize: "1.05rem", color: "#a1a1aa", maxWidth: "680px", margin: "0 auto" }}>
          {mode === "stock"
            ? "Turn scripts or AI topics into complete short-form clips with automated stock media scraping, multi-provider neural voiceovers, and animated karaoke captions."
            : "Turn scripts or AI topics into short-form clips with AI-generated scenes, neural voiceovers, and word-synced animated captions."}
        </p>
      </div>

      {/* Mode Tabs */}
      <div style={{ display: "flex", justifyContent: "center", marginBottom: "1.75rem" }}>
        <div style={{ display: "inline-flex", background: "#0c0c0f", border: "1px solid rgba(255,255,255,0.1)", borderRadius: "14px", padding: "0.35rem", gap: "0.3rem" }}>
          <button
            type="button"
            onClick={() => handleModeChange("stock")}
            style={{
              display: "flex", alignItems: "center", gap: "0.45rem", padding: "0.55rem 1.1rem", borderRadius: "10px",
              border: "none", cursor: "pointer", fontWeight: 800, fontSize: "0.85rem",
              background: mode === "stock" ? "#8b5cf6" : "transparent",
              color: mode === "stock" ? "#fff" : "#888",
              boxShadow: mode === "stock" ? "0 0 18px rgba(139,92,246,0.4)" : "none",
            }}
          >
            <Film size={16} /> Stock B-Roll
          </button>
          <button
            type="button"
            onClick={() => handleModeChange("ai")}
            style={{
              display: "flex", alignItems: "center", gap: "0.45rem", padding: "0.55rem 1.1rem", borderRadius: "10px",
              border: "none", cursor: "pointer", fontWeight: 800, fontSize: "0.85rem",
              background: mode === "ai" ? "#d946ef" : "transparent",
              color: mode === "ai" ? "#fff" : "#888",
              boxShadow: mode === "ai" ? "0 0 18px rgba(217,70,239,0.4)" : "none",
            }}
          >
            <Wand2 size={16} /> AI B-Roll
          </button>
        </div>
      </div>

      {mode === "ai" && (
        <div style={{ margin: "0 auto 1.5rem", maxWidth: "720px", padding: "0.8rem 1rem", borderRadius: "12px", background: "rgba(var(--accent-rgb),0.08)", border: "1px solid rgba(var(--accent-rgb),0.25)", color: "#c4b5fd", fontSize: "0.82rem", fontWeight: 600, display: "flex", alignItems: "center", gap: "0.6rem", lineHeight: 1.4 }}>
          <Wand2 size={16} style={{ flexShrink: 0 }} />
          <span>
            AI B-Roll generates a unique AI video clip for every sentence using the bytedance/seedance-v1-pro-fast model on WaveSpeed, plus AI background music (Lyria). Requires a WaveSpeed API key in{" "}
            <span onClick={() => document.querySelector<HTMLButtonElement>("[data-open-settings]")?.click()} style={{ color: "#fff", textDecoration: "underline", cursor: "pointer" }}>Settings</span>.
          </span>
        </div>
      )}

      {/* 2-Column Grid */}
      <div style={{ display: "grid", gridTemplateColumns: "1fr 380px", gap: "1.75rem", alignItems: "start" }}>
        
        {/* LEFT COLUMN: Controls */}
        <motion.div initial={{ opacity: 0, y: 15 }} animate={{ opacity: 1, y: 0 }} transition={{ duration: 0.4 }}>
          <div style={{ background: "#0c0c0f", border: "1px solid rgba(255,255,255,0.08)", borderRadius: "20px", padding: "1.5rem", marginBottom: "1.5rem" }}>
            
            {/* AI Topic Prompt */}
            <div style={{ marginBottom: "1.25rem" }}>
              <label style={{ display: "block", fontSize: "0.82rem", color: "#aaa", fontWeight: 700, marginBottom: "0.4rem" }}>Video Topic / Idea</label>
              <div style={{ display: "flex", gap: "0.5rem" }}>
                <input
                  type="text"
                  className="input"
                  placeholder="e.g., 5 Mind-Blowing Secrets About Space Exploration...  (or type directly in the script box below)"
                  value={topic}
                  onChange={e => setTopic(e.target.value)}
                  style={{ flex: 1, fontSize: "0.88rem" }}
                />
                <button
                  type="button"
                  onClick={handleGenerateScriptWithAI}
                  disabled={scriptStatus === "loading"}
                  style={{ background: "var(--accent)", color: "#fff", fontWeight: 900, border: "none", borderRadius: "10px", padding: "0.6rem 1rem", cursor: "pointer", display: "flex", alignItems: "center", gap: "0.4rem" }}
                >
                  {scriptStatus === "loading" ? <div className="spinner" style={{ borderColor: "#000", borderTopColor: "transparent" }} /> : <Sparkles size={16} />}
                  <span>Generate</span>
                </button>
              </div>
            </div>

            {/* Inline generation status */}
            {scriptStatus === "loading" && (
              <div style={{ marginBottom: "0.75rem", padding: "0.5rem 0.75rem", borderRadius: "8px", background: "rgba(var(--accent-rgb),0.08)", border: "1px solid rgba(var(--accent-rgb),0.2)", color: "var(--accent)", fontSize: "0.82rem", fontWeight: 600, display: "flex", alignItems: "center", gap: "0.5rem" }}>
                <div className="spinner" style={{ width: "14px", height: "14px", borderWidth: "2px", borderColor: "var(--accent)", borderTopColor: "transparent", flexShrink: 0 }} /> Generating script from topic...
              </div>
            )}
            {scriptStatus === "done" && (
              <div style={{ marginBottom: "0.75rem", padding: "0.5rem 0.75rem", borderRadius: "8px", background: "rgba(0,255,100,0.08)", border: "1px solid rgba(0,255,100,0.2)", color: "#00ff64", fontSize: "0.82rem", fontWeight: 600, display: "flex", alignItems: "center", gap: "0.4rem" }}>
                <Check size={14} /> Script generated. Review and edit below, then configure settings and generate video.
              </div>
            )}
            {scriptStatus === "error" && (
              <div style={{ marginBottom: "0.75rem", padding: "0.5rem 0.75rem", borderRadius: "8px", background: "rgba(255,50,50,0.08)", border: "1px solid rgba(255,50,50,0.2)", color: "#ff5050", fontSize: "0.82rem", fontWeight: 600, display: "flex", alignItems: "center", gap: "0.4rem" }}>
                <span style={{ fontWeight: 900 }}>!</span> {scriptError}
              </div>
            )}

            {/* Script Textarea */}
            <div>
              <label style={{ display: "block", fontSize: "0.82rem", color: "#aaa", fontWeight: 700, marginBottom: "0.4rem" }}>
                Video Script ({script.split(/\s+/).filter(Boolean).length} words)
              </label>
              <textarea
                className="input"
                rows={5}
                placeholder="Enter your script here. Each sentence will automatically match HD stock footage and voiceovers..."
                value={script}
                onChange={e => setScript(e.target.value)}
                style={{ width: "100%", fontSize: "0.88rem", lineHeight: 1.4, resize: "vertical" }}
              />
            </div>
          </div>

          {/* Grid Settings */}
          <div style={{ background: "#0c0c0f", border: "1px solid rgba(255,255,255,0.08)", borderRadius: "20px", padding: "1.5rem" }}>
            <h3 style={{ fontSize: "0.95rem", fontWeight: 800, color: "#fff", marginBottom: "1.25rem", display: "flex", alignItems: "center", gap: "0.5rem" }}>
              <Sliders size={16} color="var(--accent)" /> Studio Pipeline Settings
            </h3>

            <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: "1.25rem", marginBottom: "1.25rem" }}>
              
              {/* Aspect Ratio */}
              <div>
                <label style={{ display: "block", fontSize: "0.78rem", color: "#aaa", fontWeight: 700, marginBottom: "0.4rem" }}>Aspect Ratio</label>
                <select
                  value={aspectRatio}
                  onChange={e => setAspectRatio(e.target.value)}
                  style={{ width: "100%", background: "#131318", color: "#fff", border: "1px solid rgba(255,255,255,0.15)", borderRadius: "10px", padding: "0.55rem 0.75rem", fontSize: "0.82rem", fontWeight: 600 }}
                >
                  <option value="9:16">9:16 Vertical (TikTok / Shorts / Reels)</option>
                  <option value="1:1">1:1 Square (Instagram Feed)</option>
                  <option value="16:9">16:9 Widescreen (YouTube Video)</option>
                </select>
              </div>

            </div>

            <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: "1.25rem", marginBottom: "1.25rem" }}>
              
              {/* AI Brain / LLM Provider */}
              <div>
                <label style={{ display: "block", fontSize: "0.78rem", color: "#aaa", fontWeight: 700, marginBottom: "0.4rem" }}>AI Brain / LLM Engine</label>
                <select
                  value={llmProvider}
                  onChange={e => { const provider = e.target.value as LlmProvider; setLlmProvider(provider); setLlmModel(provider === "gemini" ? "gemini-3.1-flash-lite" : "openrouter/free"); }}
                  style={{ width: "100%", background: "#131318", color: "#fff", border: "1px solid rgba(255,255,255,0.15)", borderRadius: "10px", padding: "0.55rem 0.75rem", fontSize: "0.82rem", fontWeight: 600 }}
                >
                  <option value="gemini">Gemini</option>
                  <option value="openrouter">OpenRouter</option>
                </select>
                <select value={llmModel} onChange={e => setLlmModel(e.target.value)} style={{ width: "100%", marginTop: "0.4rem", background: "#131318", color: "#fff", border: "1px solid rgba(255,255,255,0.15)", borderRadius: "8px", padding: "0.4rem", fontSize: "0.76rem", fontWeight: 600 }}>
                  {(llmProvider === "gemini" ? GEMINI_MODELS : OPENROUTER_MODELS).map(p => <option key={p.id} value={p.id}>{p.label}</option>)}
                </select>
                {llmProvider === "openrouter" && llmModel === "custom" && (
                  <input
                    type="text"
                    placeholder="e.g., anthropic/claude-3.5-sonnet"
                    value={customLlmModel}
                    onChange={e => setCustomLlmModel(e.target.value)}
                    style={{ width: "100%", marginTop: "0.4rem", background: "#131318", color: "#fff", border: "1px solid rgba(255,255,255,0.15)", borderRadius: "8px", padding: "0.4rem", fontSize: "0.78rem" }}
                  />
                )}
              </div>

              {/* TTS Voice Provider */}
              <div>
                <label style={{ display: "block", fontSize: "0.78rem", color: "#aaa", fontWeight: 700, marginBottom: "0.4rem" }}>Text-to-Speech (TTS) Voice</label>
                <select
                  value={ttsProvider}
                  onChange={e => { const v = e.target.value; setTtsProvider(v); if (v === "edge-tts") setVoiceName("en-US-ChristopherNeural"); }}
                  style={{ width: "100%", background: "#131318", color: "#fff", border: "1px solid rgba(255,255,255,0.15)", borderRadius: "10px", padding: "0.55rem 0.75rem", fontSize: "0.82rem", fontWeight: 600 }}
                >
                  {TTS_PROVIDERS.filter(t => mode !== "ai" || t.id === "elevenlabs" || t.id === "deepgram-aura").map(t => (
                    <option key={t.id} value={t.id}>{t.label}</option>
                  ))}
                </select>
                {ttsProvider === "elevenlabs" && (
                  <select
                    value={elevenVoiceId}
                    onChange={e => setElevenVoiceId(e.target.value)}
                    style={{ width: "100%", marginTop: "0.4rem", background: "#131318", color: "#fff", border: "1px solid rgba(255,255,255,0.15)", borderRadius: "8px", padding: "0.4rem", fontSize: "0.78rem" }}
                  >
                    {ELEVENLABS_VOICES.map(v => (
                      <option key={v.id} value={v.name}>{v.name} ({v.gender}) — {v.tone}</option>
                    ))}
                  </select>
                )}
                {ttsProvider === "deepgram-aura" && (
                  <select
                    value={deepgramVoice}
                    onChange={e => setDeepgramVoice(e.target.value)}
                    style={{ width: "100%", marginTop: "0.4rem", background: "#131318", color: "#fff", border: "1px solid rgba(255,255,255,0.15)", borderRadius: "8px", padding: "0.4rem", fontSize: "0.78rem" }}
                  >
                    {DEEPGRAM_VOICES.map(v => (
                      <option key={v.id} value={v.id}>{v.label}</option>
                    ))}
                  </select>
                )}
              </div>
            </div>

            <div style={{ display: "grid", gridTemplateColumns: mode === "ai" ? "1fr 1fr" : "1fr 1fr 1fr 1fr 1fr", gap: "1rem", marginBottom: "1.5rem" }}>
              {/* Target Duration */}
              <div>
                <label style={{ display: "block", fontSize: "0.75rem", color: "#888", fontWeight: 600, marginBottom: "0.3rem" }}>Target Duration</label>
                <select
                  value={duration}
                  onChange={e => setDuration(e.target.value)}
                  style={{ width: "100%", background: "#131318", color: "#fff", border: "1px solid rgba(255,255,255,0.15)", borderRadius: "8px", padding: "0.4rem", fontSize: "0.78rem" }}
                >
                  <option value="30">30s Short</option>
                  <option value="45">45s</option>
                  <option value="60">60s Standard</option>
                  <option value="90">90s</option>
                  <option value="120">120s Long</option>
                </select>
              </div>

              {/* Media Type */}
              {mode === "stock" && (
                <div>
                  <label style={{ display: "block", fontSize: "0.75rem", color: "#888", fontWeight: 600, marginBottom: "0.3rem" }}>Media Mode</label>
                  <select
                    value={mediaType}
                    onChange={e => setMediaType(e.target.value as any)}
                    style={{ width: "100%", background: "#131318", color: "#fff", border: "1px solid rgba(255,255,255,0.15)", borderRadius: "8px", padding: "0.4rem", fontSize: "0.78rem" }}
                  >
                    <option value="video">HD Videos</option>
                    <option value="photo">Photos (Ken Burns)</option>
                  </select>
                </div>
              )}

              {/* Media Source */}
              {mode === "stock" && (
                <div>
                  <label style={{ display: "block", fontSize: "0.75rem", color: "#888", fontWeight: 600, marginBottom: "0.3rem" }}>Media Source</label>
                  <select
                    value={source}
                    onChange={e => setSource(e.target.value)}
                    style={{ width: "100%", background: "#131318", color: "#fff", border: "1px solid rgba(255,255,255,0.15)", borderRadius: "8px", padding: "0.4rem", fontSize: "0.78rem" }}
                  >
                    <option value="pinterest">Pinterest</option>
                    <option value="stock_api">Pexels &amp; Pixabay</option>
                    <option value="all">All Sources</option>
                  </select>
                </div>
              )}

              {/* Vibe Mode */}
              {mode === "stock" && (
                <div>
                  <label style={{ display: "block", fontSize: "0.75rem", color: "#888", fontWeight: 600, marginBottom: "0.3rem" }}>Vibe Aesthetic</label>
                  <select
                    value={vibe}
                    onChange={e => setVibe(e.target.value)}
                    style={{ width: "100%", background: "#131318", color: "#fff", border: "1px solid rgba(255,255,255,0.15)", borderRadius: "8px", padding: "0.4rem", fontSize: "0.78rem" }}
                  >
                    <option value="aesthetic">Aesthetic / Chill</option>
                    <option value="lofi">LoFi Art</option>
                    <option value="futuristic">Futuristic Cyber</option>
                    <option value="black_and_white">Monochrome B&W</option>
                    <option value="general">General Stock</option>
                  </select>
                </div>
              )}

              {/* Subtitle Style */}
              {mode === "stock" && (
                <div>
                  <label style={{ display: "block", fontSize: "0.75rem", color: "#888", fontWeight: 600, marginBottom: "0.3rem" }}>Subtitle Style</label>
                  <select
                    value={subtitleStyle}
                    onChange={e => setSubtitleStyle(e.target.value)}
                    style={{ width: "100%", background: "#131318", color: "#fff", border: "1px solid rgba(255,255,255,0.15)", borderRadius: "8px", padding: "0.4rem", fontSize: "0.78rem" }}
                  >
                    <option value="high_retention">Hormozi High-Retention</option>
                    <option value="yellow_box">Yellow Highlight Box</option>
                    <option value="bold_outline">Bold Black Outline</option>
                    <option value="minimal">Minimal White</option>
                  </select>
                </div>
              )}
            </div>
            {mode === "stock" && (
              <div style={{ display: "flex", gap: "1rem", marginBottom: "1.5rem", alignItems: "center" }}>
                <label style={{ display: "block", fontSize: "0.75rem", color: "#888", fontWeight: 600, whiteSpace: "nowrap" }}>Background Music</label>
                <select
                  value={bgMusic}
                  onChange={e => setBgMusic(e.target.value)}
                  style={{ flex: 1, maxWidth: "200px", background: "#131318", color: "#fff", border: "1px solid rgba(255,255,255,0.15)", borderRadius: "8px", padding: "0.4rem", fontSize: "0.78rem" }}
                >
                  <option value="none">None</option>
                  <option value="upbeat">Upbeat</option>
                  <option value="chill">Chill</option>
                  <option value="cinematic">Cinematic</option>
                </select>
              </div>
            )}

            {/* Watermark / Brand Logo Overlay */}
            {mode === "stock" && (
            <div style={{ background: "#08080a", borderRadius: "12px", padding: "1rem", border: "1px solid rgba(255,255,255,0.08)", marginBottom: "1.25rem" }}>
              <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: "0.75rem" }}>
                <label style={{ fontSize: "0.8rem", color: "#fff", fontWeight: 800, display: "flex", alignItems: "center", gap: "0.4rem" }}>
                  <Upload size={14} color="var(--accent)" />
                  Brand Logo / Watermark
                </label>
                <span style={{ fontSize: "0.7rem", color: "#888" }}>PNG / Transparent</span>
              </div>
              <input
                ref={watermarkInputRef}
                type="file" accept="image/png,image/jpeg,image/webp"
                hidden
                onChange={e => {
                  const f = e.target.files?.[0] || null;
                  setWatermarkFile(f);
                  setWatermarkPreviewUrl(f ? URL.createObjectURL(f) : null);
                }}
              />
              {watermarkFile && watermarkPreviewUrl ? (
                <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between", background: "#131318", border: "1px solid rgba(var(--accent-rgb),0.3)", borderRadius: "10px", padding: "0.5rem 0.75rem", marginBottom: "0.75rem" }}>
                  <div style={{ display: "flex", alignItems: "center", gap: "0.6rem" }}>
                    <img src={watermarkPreviewUrl} alt="Logo" style={{ width: "30px", height: "30px", objectFit: "contain", borderRadius: "4px", background: "#000", padding: "2px" }} />
                    <span style={{ fontSize: "0.78rem", fontWeight: 600, color: "#fff" }}>{watermarkFile.name}</span>
                  </div>
                  <button type="button" onClick={() => { setWatermarkFile(null); setWatermarkPreviewUrl(null); }} style={{ background: "rgba(239,68,68,0.15)", border: "1px solid rgba(239,68,68,0.3)", color: "#ef4444", borderRadius: "6px", padding: "0.25rem 0.6rem", fontSize: "0.7rem", fontWeight: 700, cursor: "pointer" }}>Remove</button>
                </div>
              ) : (
                <div onClick={() => watermarkInputRef.current?.click()} style={{ background: "#131318", border: "2px dashed rgba(255,255,255,0.15)", borderRadius: "10px", padding: "0.75rem", textAlign: "center", cursor: "pointer", marginBottom: "0.75rem" }}>
                  <Upload size={20} style={{ color: "var(--accent)", marginBottom: "0.2rem" }} />
                  <div style={{ fontSize: "0.78rem", fontWeight: 700, color: "#fff" }}>Click to Upload Logo</div>
                </div>
              )}
              <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: "0.75rem" }}>
                <div>
                  <label style={{ display: "block", fontSize: "0.7rem", color: "#aaa", marginBottom: "0.2rem", fontWeight: 600 }}>Position</label>
                  <select value={watermarkPosition} onChange={e => setWatermarkPosition(e.target.value)} style={{ width: "100%", background: "#131318", color: "#fff", border: "1px solid rgba(255,255,255,0.15)", borderRadius: "6px", padding: "0.35rem 0.5rem", fontSize: "0.72rem", fontWeight: 600 }}>
                    <option value="top_right">Top Right</option>
                    <option value="top_left">Top Left</option>
                    <option value="bottom_right">Bottom Right</option>
                    <option value="bottom_left">Bottom Left</option>
                  </select>
                </div>
                <div>
                  <label style={{ display: "block", fontSize: "0.7rem", color: "#aaa", marginBottom: "0.2rem", fontWeight: 600 }}>Opacity ({watermarkOpacity}%)</label>
                  <input type="range" min={10} max={100} value={watermarkOpacity} onChange={e => setWatermarkOpacity(parseInt(e.target.value, 10))} style={{ width: "100%", accentColor: "var(--accent)", cursor: "pointer", marginTop: "0.35rem" }} />
                </div>
              </div>
            </div>
            )}

            {/* Submit Button */}
            <button
              type="button"
              onClick={handleCreateStudioVideo}
              disabled={loading}
              style={{
                width: "100%", background: "var(--accent)", color: "#fff", fontWeight: 900,
                fontSize: "1.05rem", borderRadius: "14px", border: "none", padding: "0.9rem",
                boxShadow: "0 0 25px rgba(var(--accent-rgb),0.35)", cursor: "pointer", display: "flex",
                alignItems: "center", justifyContent: "center", gap: "0.5rem"
              }}
            >
              {loading ? (
                <><div className="spinner" style={{ borderColor: "#000", borderTopColor: "transparent" }} /><span>Generating Video Engine...</span></>
              ) : (
                <>{mode === "ai" ? <Wand2 size={20} /> : <Film size={20} />}<span>{mode === "ai" ? "Generate AI B-Roll Video" : "Generate Faceless AI Video"}</span></>
              )}
            </button>

          </div>
        </motion.div>

        {/* RIGHT COLUMN: Studio Production Process */}
        <motion.div initial={{ opacity: 0, x: 20 }} animate={{ opacity: 1, x: 0 }} transition={{ duration: 0.4, delay: 0.1 }} style={{ position: "sticky", top: "84px" }}>
          <div style={{ background: "linear-gradient(180deg, #171322 0%, #0d0b12 100%)", border: "1px solid rgba(var(--accent-rgb),0.28)", borderRadius: "24px", padding: "1.5rem", display: "flex", flexDirection: "column" }}>
            <div style={{ display: "flex", justifyContent: "space-between", width: "100%", marginBottom: "0.5rem", alignItems: "center" }}>
              <span style={{ fontSize: "0.75rem", color: "var(--accent)", fontWeight: 800, textTransform: "uppercase", display: "flex", alignItems: "center", gap: "0.4rem" }}>
                <Sparkles size={14} /> How Nova Studio Builds Your Video
              </span>
            </div>

            <p style={{ color: "#aaa", fontSize: "0.78rem", lineHeight: 1.5, margin: "0 0 1.25rem" }}>
              {mode === "stock"
                ? "From an idea to a finished faceless video, each stage prepares the next one automatically."
                : "AI B-Roll replaces stock footage with AI-generated clips and music tailored to your script."}
            </p>

            <div style={{ display: "flex", flexDirection: "column", gap: "0.65rem", marginBottom: "1.25rem" }}>
              {(mode === "stock" ? [
                { number: "01", icon: MessageSquare, title: "Shape the story", text: "Gemini turns your topic into a structured script with the right length and tone." },
                { number: "02", icon: Volume2, title: "Create the voiceover", text: "Your selected neural voice reads the script as one continuous narration." },
                { number: "03", icon: Search, title: "Gather supporting visuals", text: "Nova Studio searches your selected stock sources for footage or photos in the chosen vibe." },
                { number: "04", icon: Layers, title: "Build the visual timeline", text: "Media is trimmed, looped, and sequenced to follow the narration." },
                { number: "05", icon: Sparkles, title: "Finish captions and branding", text: "Word-level karaoke captions, music, aspect ratio, and watermark are composited." },
              ] : [
                { number: "01", icon: MessageSquare, title: "Shape the story", text: "Gemini turns your topic into a structured script with the right length and tone." },
                { number: "02", icon: Volume2, title: "Create the voiceover", text: "Your selected neural voice reads the script as one continuous narration." },
                { number: "03", icon: Wand2, title: "Direct each scene", text: "Gemini writes a detailed visual prompt for every sentence as your AI camera director." },
                { number: "04", icon: Zap, title: "Generate AI B-roll", text: "The bytedance/seedance-v1-pro-fast model on WaveSpeed renders a unique clip per scene (up to 3 in parallel), with Pexels as fallback." },
                { number: "05", icon: Layers, title: "Compose music & captions", text: "Lyria composes matching AI music while word-level karaoke captions are burned in." },
              ]).map(({ number, icon: Icon, title, text }) => (
                <div key={number} style={{ display: "grid", gridTemplateColumns: "34px 1fr", gap: "0.7rem", alignItems: "start", padding: "0.7rem", borderRadius: "12px", background: "rgba(var(--accent-rgb),0.07)", border: "1px solid rgba(var(--accent-rgb),0.14)" }}>
                  <div style={{ width: 30, height: 30, borderRadius: "9px", background: "var(--accent)", color: "#fff", display: "flex", alignItems: "center", justifyContent: "center", fontSize: "0.65rem", fontWeight: 900, boxShadow: "0 0 14px rgba(var(--accent-rgb),0.3)" }}>
                    {number}
                  </div>
                  <div>
                    <div style={{ display: "flex", alignItems: "center", gap: "0.4rem", color: "#fff", fontSize: "0.78rem", fontWeight: 800 }}>
                      <Icon size={14} color="#a78bfa" /> {title}
                    </div>
                    <div style={{ color: "#999", fontSize: "0.7rem", lineHeight: 1.45, marginTop: "0.25rem" }}>{text}</div>
                  </div>
                </div>
              ))}
            </div>

            <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: "0.6rem", borderTop: "1px solid rgba(255,255,255,0.08)", paddingTop: "1rem" }}>
              {[
                ["Output", aspectRatio],
                ["Duration", `${duration}s`],
                ["Voice", ttsProvider === "edge-tts" ? "Edge-TTS" : ttsProvider === "elevenlabs" ? "ElevenLabs" : "Deepgram Aura"],
                ["Visuals", mode === "ai" ? "AI clips" : mediaType === "video" ? "HD video" : "Photos"],
              ].map(([label, value]) => (
                <div key={label} style={{ background: "rgba(0,0,0,0.25)", borderRadius: "8px", padding: "0.55rem 0.65rem" }}>
                  <div style={{ color: "#777", fontSize: "0.62rem", textTransform: "uppercase", letterSpacing: "0.06em" }}>{label}</div>
                  <div style={{ color: "#c4b5fd", fontSize: "0.75rem", fontWeight: 800, marginTop: "0.15rem" }}>{value}</div>
                </div>
              ))}
            </div>
          </div>
        </motion.div>

      </div>

      {/* Nova Studio Footer */}
      <section style={{ borderTop: "1px solid rgba(var(--accent-rgb),0.18)", background: "#0b0b0e", padding: "2.2rem 0", marginTop: "2.5rem" }}>
        <div className="container" style={{ maxWidth: "1200px", margin: "0 auto", padding: "0 1.5rem" }}>
          <div style={{ display: "grid", gridTemplateColumns: "repeat(auto-fit, minmax(210px, 1fr))", gap: "1.5rem", textAlign: "center" }}>
            {[
              { key: "SCRIPT", color: "var(--accent)", text: "AI-shaped storytelling" },
              { key: "VOICE", color: mode === "ai" ? "var(--accent)" : "#a78bfa", text: "Natural neural narration" },
              { key: "VISUALS", color: mode === "ai" ? "var(--accent)" : "#c4b5fd", text: mode === "ai" ? "AI-generated scenes" : "Stock media matched to your vibe" },
              { key: "FINISH", color: mode === "ai" ? "var(--accent)" : "#ddd6fe", text: mode === "ai" ? "AI music, captions, branding" : "Captions, music, and branding" },
            ].map(({ key, color, text }) => (
              <div key={key}>
                <h4 style={{ fontSize: "1.35rem", fontWeight: 900, color, margin: 0 }}>{key}</h4>
                <p style={{ fontSize: "0.78rem", color: "#888", margin: "0.35rem 0 0" }}>{text}</p>
              </div>
            ))}
          </div>
        </div>
      </section>
    </div>
  );
}
