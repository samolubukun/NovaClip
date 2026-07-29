import { useState, useRef } from "react";
import { useNavigate } from "react-router-dom";
import { motion } from "framer-motion";
import {
  Film, Sparkles, Wand2, Sliders, Play, RotateCcw, Upload, Image as ImageIcon,
  Check, Cpu, Volume2, Globe, FileText, Layers, Video, Zap, MessageSquare
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
  { id: "meta-llama/llama-3.3-70b-instruct:free", label: "OpenRouter — Llama 3.3 70B (:free)", icon: Cpu },
  { id: "deepseek/deepseek-r1:free", label: "OpenRouter — DeepSeek R1 (:free)", icon: Cpu },
  { id: "qwen/qwen-2.5-72b-instruct:free", label: "OpenRouter — Qwen 2.5 72B (:free)", icon: Cpu },
  { id: "custom", label: "OpenRouter — Custom Specified Model", icon: Cpu },
];

const TTS_PROVIDERS = [
  { id: "edge-tts", label: "Edge-TTS (Free Neural Voices)", desc: "10+ Languages, Zero API cost" },
  { id: "elevenlabs", label: "ElevenLabs API", desc: "Cloned & custom voices (Requires API Key)" },
  { id: "deepgram-aura", label: "Deepgram Aura TTS", desc: "Low latency streaming AI voices" },
];

const SCRAPER_SOURCES = [
  { id: "pinterest", label: "Pinterest Video & Photo Scraper (World's First!)", badge: "Free" },
  { id: "pexels", label: "Pexels Stock API", badge: "API" },
  { id: "pixabay", label: "Pixabay Stock API", badge: "API" },
];

export default function Studio() {
  const navigate = useNavigate();
  const [scriptMode, setScriptMode] = useState<"write" | "ai">("ai");
  const [script, setScript] = useState("");
  const [topic, setTopic] = useState("");
  const [aspectRatio, setAspectRatio] = useState("9:16");
  const [llmProvider, setLlmProvider] = useState("gemini-3.1-flash-lite");
  const [customLlmModel, setCustomLlmModel] = useState("");
  const [ttsProvider, setTtsProvider] = useState("edge-tts");
  const [voiceName, setVoiceName] = useState("en-US-ChristopherNeural");
  const [elevenVoiceId, setElevenVoiceId] = useState("");
  const [scraperSource, setScraperSource] = useState("pinterest");
  const [mediaType, setMediaType] = useState<"video" | "photo">("video");
  const [vibe, setVibe] = useState("aesthetic");
  const [subtitleStyle, setSubtitleStyle] = useState("high_retention");
  const [bgMusic, setBgMusic] = useState("none");
  const [loading, setLoading] = useState(false);
  const [aiScriptLoading, setAiScriptLoading] = useState(false);

  const currentMockup = ASPECT_RATIOS.find(ar => ar.id === aspectRatio) || ASPECT_RATIOS[0];

  const handleGenerateScriptWithAI = async () => {
    if (!topic.trim()) {
      toast.error("Please enter a topic for AI script generation");
      return;
    }
    setAiScriptLoading(true);
    try {
      const apiKey = localStorage.getItem("novaclip_gemini_key") || localStorage.getItem("novaclip_openrouter_key") || "";
      const res = await fetch("/api/studio/generate_script", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          topic: topic.trim(),
          vibe,
          llm_provider: llmProvider === "custom" ? customLlmModel : llmProvider,
          api_key: apiKey,
        }),
      });
      if (!res.ok) throw new Error("Script generation failed");
      const data = await res.json();
      setScript(data.script || "");
      setScriptMode("write");
      toast.success("AI script generated successfully!");
    } catch (e: any) {
      toast.error(e.message || "Failed to generate script");
    } finally {
      setAiScriptLoading(false);
    }
  };

  const handleCreateStudioVideo = async () => {
    if (!script.trim()) {
      toast.error("Please enter or generate a video script first");
      return;
    }
    setLoading(true);
    try {
      const payload = {
        script: script.trim(),
        aspect_ratio: aspectRatio,
        llm_provider: llmProvider === "custom" ? customLlmModel : llmProvider,
        tts_provider: ttsProvider,
        voice: ttsProvider === "elevenlabs" ? elevenVoiceId : voiceName,
        source: scraperSource,
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
        video_url: "studio://faceless",
        aspect_ratio: aspectRatio,
        num_clips: 1,
        font_family: "THEBOLDFONT",
        font_size: 28,
        font_color: "#ffffff",
        highlight_color: "#ffe000",
        caption_animation: "word_pop",
        auto_emojis: true,
        watermark_position: "top_right",
        watermark_opacity: 80,
        studio_payload: payload,
      });

      toast.success("Nova Studio task created! Processing video...");
      navigate(`/task/${task.id}`);
    } catch (e: any) {
      toast.error(e.message || "Failed to start studio task");
    } finally {
      setLoading(false);
    }
  };

  return (
    <div style={{ maxWidth: "1280px", margin: "0 auto", padding: "1.5rem 1rem" }}>
      {/* Header Banner */}
      <div style={{ textAlign: "center", marginBottom: "2rem" }}>
        <div style={{ display: "inline-flex", alignItems: "center", gap: "0.5rem", background: "rgba(255,224,0,0.1)", border: "1px solid rgba(255,224,0,0.3)", padding: "0.4rem 1rem", borderRadius: "20px", color: "var(--accent)", fontSize: "0.78rem", fontWeight: 800, textTransform: "uppercase", letterSpacing: "0.08em", marginBottom: "1.15rem" }}>
          <Film size={14} /> Nova Studio — Faceless AI Creator
        </div>
        <h1 style={{ fontSize: "2.25rem", fontWeight: 900, color: "#fff", marginBottom: "0.6rem" }}>
          Generate Viral Faceless Videos in Seconds
        </h1>
        <p style={{ color: "#aaa", fontSize: "0.92rem", maxWidth: "680px", margin: "0 auto" }}>
          Turn scripts or AI topics into complete short-form clips with automated stock media scraping, multi-provider neural voiceovers, and animated karaoke captions.
        </p>
      </div>

      {/* 2-Column Grid */}
      <div style={{ display: "grid", gridTemplateColumns: "1fr 380px", gap: "1.75rem", alignItems: "start" }}>
        
        {/* LEFT COLUMN: Controls */}
        <motion.div initial={{ opacity: 0, y: 15 }} animate={{ opacity: 1, y: 0 }} transition={{ duration: 0.4 }}>
          <div style={{ background: "#0c0c0f", border: "1px solid rgba(255,255,255,0.08)", borderRadius: "20px", padding: "1.5rem", marginBottom: "1.5rem" }}>
            
            {/* Script Input Mode Switcher */}
            <div style={{ display: "flex", gap: "0.75rem", marginBottom: "1.25rem" }}>
              <button
                type="button"
                onClick={() => setScriptMode("ai")}
                style={{
                  flex: 1, padding: "0.65rem", borderRadius: "12px",
                  border: `1px solid ${scriptMode === "ai" ? "var(--accent)" : "rgba(255,255,255,0.08)"}`,
                  background: scriptMode === "ai" ? "rgba(255,224,0,0.12)" : "#131318",
                  color: scriptMode === "ai" ? "var(--accent)" : "#aaa",
                  fontWeight: 800, fontSize: "0.85rem", cursor: "pointer", display: "flex", alignItems: "center", justifyContent: "center", gap: "0.5rem"
                }}
              >
                <Wand2 size={16} /> AI Topic Script Writer
              </button>
              <button
                type="button"
                onClick={() => setScriptMode("write")}
                style={{
                  flex: 1, padding: "0.65rem", borderRadius: "12px",
                  border: `1px solid ${scriptMode === "write" ? "var(--accent)" : "rgba(255,255,255,0.08)"}`,
                  background: scriptMode === "write" ? "rgba(255,224,0,0.12)" : "#131318",
                  color: scriptMode === "write" ? "var(--accent)" : "#aaa",
                  fontWeight: 800, fontSize: "0.85rem", cursor: "pointer", display: "flex", alignItems: "center", justifyContent: "center", gap: "0.5rem"
                }}
              >
                <FileText size={16} /> Paste Custom Script
              </button>
            </div>

            {/* AI Topic Prompt */}
            {scriptMode === "ai" ? (
              <div style={{ marginBottom: "1.25rem" }}>
                <label style={{ display: "block", fontSize: "0.82rem", color: "#aaa", fontWeight: 700, marginBottom: "0.4rem" }}>Video Topic / Idea</label>
                <div style={{ display: "flex", gap: "0.5rem" }}>
                  <input
                    type="text"
                    className="input"
                    placeholder="e.g., 5 Mind-Blowing Secrets About Space Exploration..."
                    value={topic}
                    onChange={e => setTopic(e.target.value)}
                    style={{ flex: 1, fontSize: "0.88rem" }}
                  />
                  <button
                    type="button"
                    onClick={handleGenerateScriptWithAI}
                    disabled={aiScriptLoading}
                    style={{ background: "var(--accent)", color: "#000", fontWeight: 900, border: "none", borderRadius: "10px", padding: "0.6rem 1rem", cursor: "pointer", display: "flex", alignItems: "center", gap: "0.4rem" }}
                  >
                    {aiScriptLoading ? <div className="spinner" style={{ borderColor: "#000", borderTopColor: "transparent" }} /> : <Sparkles size={16} />}
                    <span>Generate</span>
                  </button>
                </div>
              </div>
            ) : null}

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

              {/* Media Scraper Source */}
              <div>
                <label style={{ display: "block", fontSize: "0.78rem", color: "#aaa", fontWeight: 700, marginBottom: "0.4rem" }}>Media Scraper Source</label>
                <select
                  value={scraperSource}
                  onChange={e => setScraperSource(e.target.value)}
                  style={{ width: "100%", background: "#131318", color: "#fff", border: "1px solid rgba(255,255,255,0.15)", borderRadius: "10px", padding: "0.55rem 0.75rem", fontSize: "0.82rem", fontWeight: 600 }}
                >
                  {SCRAPER_SOURCES.map(src => (
                    <option key={src.id} value={src.id}>{src.label}</option>
                  ))}
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
                    placeholder="ElevenLabs Voice ID (e.g., 21m00Tcm4TlvDq8ikWAM)"
                    value={elevenVoiceId}
                    onChange={e => setElevenVoiceId(e.target.value)}
                    style={{ width: "100%", marginTop: "0.4rem", background: "#131318", color: "#fff", border: "1px solid rgba(255,255,255,0.15)", borderRadius: "8px", padding: "0.4rem", fontSize: "0.78rem" }}
                  />
                )}
              </div>
            </div>

            <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr 1fr", gap: "1rem", marginBottom: "1.5rem" }}>
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

            {/* Submit Button */}
            <button
              type="button"
              onClick={handleCreateStudioVideo}
              disabled={loading}
              style={{
                width: "100%", background: "var(--accent)", color: "#000", fontWeight: 900,
                fontSize: "1.05rem", borderRadius: "14px", border: "none", padding: "0.9rem",
                boxShadow: "0 0 25px rgba(255,224,0,0.25)", cursor: "pointer", display: "flex",
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
              <span style={{ fontSize: "0.75rem", color: "var(--accent)", fontWeight: 800, textTransform: "uppercase", display: "flex", alignItems: "center", gap: "0.4rem" }}>
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
                <div style={{ background: "rgba(0,0,0,0.85)", border: "1px solid var(--accent)", borderRadius: "8px", padding: "0.4rem 0.8rem", color: "#fff", fontSize: "0.75rem", fontWeight: 900, textAlign: "center", maxWidth: "85%" }}>
                  <span style={{ color: "var(--accent)" }}>FACELESS AI </span>
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
