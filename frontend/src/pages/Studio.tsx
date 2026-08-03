import { useState, useRef } from "react";
import { useNavigate } from "react-router-dom";
import { motion } from "framer-motion";
import {
  Film, Sparkles, Wand2, Sliders, Play, RotateCcw, Upload, Image as ImageIcon,
  Check, Cpu, Volume2, Globe, Layers, Video, Zap, MessageSquare
} from "lucide-react";
import { toast } from "sonner";
import { api } from "../lib/api";

const ASPECT_RATIOS = [
  { id: "9:16", label: "9:16 Vertical", sublabel: "TikTok / Shorts / Reels", width: "190px", height: "340px", borderRadius: "24px" },
  { id: "1:1", label: "1:1 Square", sublabel: "Instagram Post", width: "240px", height: "240px", borderRadius: "16px" },
  { id: "16:9", label: "16:9 Widescreen", sublabel: "YouTube Video", width: "320px", height: "180px", borderRadius: "14px" },
];

const LLM_PROVIDERS = [
  { id: "gemini-3.1-flash-lite", label: "Gemini 3.1 Flash-Lite (Recommended Default)", icon: Sparkles },
  { id: "gemini-3.1-pro", label: "Gemini 3.1 Pro (Deep Reasoning)", icon: Sparkles },
  { id: "openrouter/free", label: "OpenRouter: Free Models Router (Auto-Select)", icon: Cpu },
  { id: "nvidia/nemotron-3-nano-30b-a3b:free", label: "OpenRouter: Nvidia Nemotron 3 Nano (:free)", icon: Cpu },
  { id: "google/gemma-4-31b:free", label: "OpenRouter: Google Gemma 4 31B (:free)", icon: Cpu },
  { id: "openai/gpt-oss-20b:free", label: "OpenRouter: OpenAI GPT-OSS 20B (:free)", icon: Cpu },
  { id: "poolside/laguna-s-2.1:free", label: "OpenRouter: Poolside Laguna S 2.1 (:free)", icon: Cpu },
  { id: "cohere/north-mini-code-20260617:free", label: "OpenRouter: Cohere North Mini Code (:free)", icon: Cpu },
  { id: "meta-llama/llama-3.3-70b-instruct:free", label: "OpenRouter: Llama 3.3 70B (:free)", icon: Cpu },
  { id: "deepseek/deepseek-r1:free", label: "OpenRouter: DeepSeek R1 (:free)", icon: Cpu },
  { id: "qwen/qwen-2.5-72b-instruct:free", label: "OpenRouter: Qwen 2.5 72B (:free)", icon: Cpu },
  { id: "custom", label: "OpenRouter: Custom Specified Model", icon: Cpu },
];

const TTS_PROVIDERS = [
  { id: "edge-tts", label: "Edge-TTS (Free Neural Voices)", desc: "10+ Languages, Zero API cost" },
  { id: "elevenlabs", label: "ElevenLabs API", desc: "Cloned & custom voices (Requires API Key)" },
  { id: "deepgram-aura", label: "Deepgram Aura TTS", desc: "Low latency streaming AI voices" },
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
  const [llmProvider, setLlmProvider] = useState("gemini-3.1-flash-lite");
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

  const [watermarkFile, setWatermarkFile] = useState<File | null>(null);
  const [watermarkPreviewUrl, setWatermarkPreviewUrl] = useState<string | null>(null);
  const [watermarkPosition, setWatermarkPosition] = useState("top_right");
  const [watermarkOpacity, setWatermarkOpacity] = useState(80);
  const watermarkInputRef = useRef<HTMLInputElement>(null);

  const currentMockup = ASPECT_RATIOS.find(ar => ar.id === aspectRatio) || ASPECT_RATIOS[0];

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
          llm_provider: llmProvider === "custom" ? customLlmModel : llmProvider,
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
    setLoading(true);
    try {
      const videoTitle = topic.trim()
        ? topic.trim().slice(0, 80)
        : script.trim().split(/\s+/).slice(0, 10).join(" ").slice(0, 80);

      const payload = {
        script: script.trim(),
        aspect_ratio: aspectRatio,
        llm_provider: llmProvider === "custom" ? customLlmModel : llmProvider,
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
        }
      };

      const task = await api.createTask({
        url: "studio://faceless",
        source_title: videoTitle,
        aspect_ratio: aspectRatio,
        num_clips: 1,
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
    <div style={{ maxWidth: "1280px", margin: "0 auto", padding: "1.5rem 1rem" }}>
      {/* Header Banner */}
      <div style={{ textAlign: "center", marginBottom: "2.5rem" }}>
        <div style={{ display: "inline-flex", alignItems: "center", gap: "0.5rem", background: "rgba(139,92,246,0.1)", border: "1px solid rgba(139,92,246,0.3)", padding: "0.4rem 1rem", borderRadius: "20px", color: "#8b5cf6", fontSize: "0.78rem", fontWeight: 800, textTransform: "uppercase", letterSpacing: "0.08em", marginBottom: "2.3rem" }}>
          <Film size={14} /> Nova Studio: Faceless AI Creator
        </div>
        <h1 style={{ fontSize: "clamp(1.8rem, 3.8vw, 3.2rem)", fontWeight: 900, lineHeight: 1.1, marginBottom: "0.75rem", letterSpacing: "-0.03em", color: "#fff" }}>
          Generate Viral <span style={{ color: "#8b5cf6", textShadow: "0 0 35px rgba(139,92,246,0.3)" }}>Faceless AI Videos</span>
        </h1>
        <p style={{ fontSize: "1.05rem", color: "#a1a1aa", maxWidth: "680px", margin: "0 auto" }}>
          Turn scripts or AI topics into complete short-form clips with automated stock media scraping, multi-provider neural voiceovers, and animated karaoke captions.
        </p>
      </div>

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
                  style={{ background: "#8b5cf6", color: "#fff", fontWeight: 900, border: "none", borderRadius: "10px", padding: "0.6rem 1rem", cursor: "pointer", display: "flex", alignItems: "center", gap: "0.4rem" }}
                >
                  {scriptStatus === "loading" ? <div className="spinner" style={{ borderColor: "#000", borderTopColor: "transparent" }} /> : <Sparkles size={16} />}
                  <span>Generate</span>
                </button>
              </div>
            </div>

            {/* Inline generation status */}
            {scriptStatus === "loading" && (
              <div style={{ marginBottom: "0.75rem", padding: "0.5rem 0.75rem", borderRadius: "8px", background: "rgba(139,92,246,0.08)", border: "1px solid rgba(139,92,246,0.2)", color: "#8b5cf6", fontSize: "0.82rem", fontWeight: 600, display: "flex", alignItems: "center", gap: "0.5rem" }}>
                <div className="spinner" style={{ width: "14px", height: "14px", borderWidth: "2px", borderColor: "#8b5cf6", borderTopColor: "transparent", flexShrink: 0 }} /> Generating script from topic...
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
              <Sliders size={16} color="#8b5cf6" /> Studio Pipeline Settings
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
                  onChange={e => setLlmProvider(e.target.value)}
                  style={{ width: "100%", background: "#131318", color: "#fff", border: "1px solid rgba(255,255,255,0.15)", borderRadius: "10px", padding: "0.55rem 0.75rem", fontSize: "0.82rem", fontWeight: 600 }}
                >
                  {LLM_PROVIDERS.map(p => (
                    <option key={p.id} value={p.id}>{p.label}</option>
                  ))}
                </select>
                {llmProvider === "custom" && (
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
                  onChange={e => setTtsProvider(e.target.value)}
                  style={{ width: "100%", background: "#131318", color: "#fff", border: "1px solid rgba(255,255,255,0.15)", borderRadius: "10px", padding: "0.55rem 0.75rem", fontSize: "0.82rem", fontWeight: 600 }}
                >
                  {TTS_PROVIDERS.map(t => (
                    <option key={t.id} value={t.id}>{t.label}</option>
                  ))}
                </select>
                {ttsProvider === "elevenlabs" && (
                  <input
                    type="text"
                    placeholder="Voice ID or name (e.g., Bella, Adam, EXAVITQu4vr4xnSDxMaL)"
                    value={elevenVoiceId}
                    onChange={e => setElevenVoiceId(e.target.value)}
                    style={{ width: "100%", marginTop: "0.4rem", background: "#131318", color: "#fff", border: "1px solid rgba(255,255,255,0.15)", borderRadius: "8px", padding: "0.4rem", fontSize: "0.78rem" }}
                  />
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

            <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr 1fr 1fr 1fr", gap: "1rem", marginBottom: "1.5rem" }}>
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

              {/* Media Source */}
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

              {/* Vibe Mode */}
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

              {/* Subtitle Style */}
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
            </div>
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

            {/* Watermark / Brand Logo Overlay */}
            <div style={{ background: "#08080a", borderRadius: "12px", padding: "1rem", border: "1px solid rgba(255,255,255,0.08)", marginBottom: "1.25rem" }}>
              <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: "0.75rem" }}>
                <label style={{ fontSize: "0.8rem", color: "#fff", fontWeight: 800, display: "flex", alignItems: "center", gap: "0.4rem" }}>
                  <Upload size={14} color="#8b5cf6" />
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
                <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between", background: "#131318", border: "1px solid rgba(139,92,246,0.3)", borderRadius: "10px", padding: "0.5rem 0.75rem", marginBottom: "0.75rem" }}>
                  <div style={{ display: "flex", alignItems: "center", gap: "0.6rem" }}>
                    <img src={watermarkPreviewUrl} alt="Logo" style={{ width: "30px", height: "30px", objectFit: "contain", borderRadius: "4px", background: "#000", padding: "2px" }} />
                    <span style={{ fontSize: "0.78rem", fontWeight: 600, color: "#fff" }}>{watermarkFile.name}</span>
                  </div>
                  <button type="button" onClick={() => { setWatermarkFile(null); setWatermarkPreviewUrl(null); }} style={{ background: "rgba(239,68,68,0.15)", border: "1px solid rgba(239,68,68,0.3)", color: "#ef4444", borderRadius: "6px", padding: "0.25rem 0.6rem", fontSize: "0.7rem", fontWeight: 700, cursor: "pointer" }}>Remove</button>
                </div>
              ) : (
                <div onClick={() => watermarkInputRef.current?.click()} style={{ background: "#131318", border: "2px dashed rgba(255,255,255,0.15)", borderRadius: "10px", padding: "0.75rem", textAlign: "center", cursor: "pointer", marginBottom: "0.75rem" }}>
                  <Upload size={20} style={{ color: "#8b5cf6", marginBottom: "0.2rem" }} />
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
                  <input type="range" min={10} max={100} value={watermarkOpacity} onChange={e => setWatermarkOpacity(parseInt(e.target.value, 10))} style={{ width: "100%", accentColor: "#8b5cf6", cursor: "pointer", marginTop: "0.35rem" }} />
                </div>
              </div>
            </div>

            {/* Submit Button */}
            <button
              type="button"
              onClick={handleCreateStudioVideo}
              disabled={loading}
              style={{
                width: "100%", background: "#8b5cf6", color: "#fff", fontWeight: 900,
                fontSize: "1.05rem", borderRadius: "14px", border: "none", padding: "0.9rem",
                boxShadow: "0 0 25px rgba(139,92,246,0.35)", cursor: "pointer", display: "flex",
                alignItems: "center", justifyContent: "center", gap: "0.5rem"
              }}
            >
              {loading ? (
                <><div className="spinner" style={{ borderColor: "#000", borderTopColor: "transparent" }} /><span>Generating Video Engine...</span></>
              ) : (
                <><Film size={20} /><span>Generate Faceless AI Video</span></>
              )}
            </button>

          </div>
        </motion.div>

        {/* RIGHT COLUMN: Live Mockup */}
        <motion.div initial={{ opacity: 0, x: 20 }} animate={{ opacity: 1, x: 0 }} transition={{ duration: 0.4, delay: 0.1 }} style={{ position: "sticky", top: "84px" }}>
          <div style={{ background: "#131318", border: "1px solid rgba(255, 255, 255, 0.12)", borderRadius: "24px", padding: "1.5rem", display: "flex", flexDirection: "column", alignItems: "center" }}>
            <div style={{ display: "flex", justifyContent: "space-between", width: "100%", marginBottom: "1.25rem", alignItems: "center" }}>
              <span style={{ fontSize: "0.75rem", color: "#8b5cf6", fontWeight: 800, textTransform: "uppercase", display: "flex", alignItems: "center", gap: "0.4rem" }}>
                <Sparkles size={14} /> Live Studio Preview
              </span>
              <span style={{ fontSize: "0.68rem", color: "#888", background: "#08080a", padding: "0.2rem 0.6rem", borderRadius: "6px" }}>{aspectRatio}</span>
            </div>

            {/* Device Mockup */}
            <div style={{ width: "100%", minHeight: "440px", background: "#050507", borderRadius: "18px", display: "flex", alignItems: "center", justifyContent: "center", position: "relative", overflow: "hidden", border: "1px solid rgba(255,255,255,0.08)" }}>
              <div style={{ width: currentMockup.width, height: currentMockup.height, borderRadius: currentMockup.borderRadius, background: "linear-gradient(180deg, #111118 0%, #08080c 100%)", position: "relative", display: "flex", flexDirection: "column", alignItems: "center", justifyContent: "center", padding: "1rem", border: "2px solid rgba(255,255,255,0.12)" }}>
                
                {/* Notch for 9:16 */}
                {aspectRatio === "9:16" && <div style={{ position: "absolute", top: "8px", width: "60px", height: "12px", background: "#000", borderRadius: "10px" }} />}

                {/* Subtitle Preview */}
                <div style={{ background: "rgba(0,0,0,0.85)", border: "1px solid #8b5cf6", borderRadius: "8px", padding: "0.4rem 0.8rem", color: "#fff", fontSize: "0.75rem", fontWeight: 900, textAlign: "center", maxWidth: "85%" }}>
                  <span style={{ color: "#8b5cf6" }}>FACELESS AI </span>
                  <span>VIDEO IN SECONDS 🔥</span>
                </div>
              </div>
            </div>
          </div>
        </motion.div>

      </div>
    </div>
  );
}
