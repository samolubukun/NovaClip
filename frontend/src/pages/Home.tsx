import { useState, useCallback, useRef } from "react";
import { useNavigate } from "react-router-dom";
import { Link2, Upload, Zap, Sparkles, Smartphone, Square, Monitor, Film, Plus, Minus, Check, Sliders, Cpu, Wand2, Languages, MessageSquareText, FormInput } from "lucide-react";
import { toast } from "sonner";
import { api } from "@/lib/api";
import { motion } from "framer-motion";

const ASPECT_RATIOS = [
  { id: "9:16", label: "9:16", sublabel: "Vertical 9:16", Icon: Smartphone },
  { id: "1:1",  label: "1:1",  sublabel: "Instagram",      Icon: Square },
  { id: "16:9", label: "16:9", sublabel: "YouTube",         Icon: Monitor },
  { id: "original", label: "Original", sublabel: "No crop",Icon: Film },
];

const CAPTION_TEMPLATES = [
  { id: "default", label: "Default", color: "#FFE000", bg: "transparent", textColor: "#FFFFFF", highlightColor: "#FFE000" },
  { id: "bold", label: "Bold Accent", color: "#00FF66", bg: "transparent", textColor: "#FFFFFF", highlightColor: "#00FF66" },
  { id: "vibrant", label: "High Energy", color: "#FFFF00", bg: "transparent", textColor: "#FFFF00", highlightColor: "#FF2D2D" },
  { id: "tiktok",  label: "TikTok",  color: "#FE2C55", bg: "transparent", textColor: "#FFFFFF", highlightColor: "#FE2C55" },
  { id: "neon",    label: "Neon Cyber",color: "#FF00FF", bg: "transparent", textColor: "#00FFFF", highlightColor: "#FF00FF" },
  { id: "podcast", label: "Podcast", color: "#FFB800", bg: "rgba(0,0,0,0.85)", textColor: "#FFFFFF", highlightColor: "#FFB800" },
  { id: "minimal", label: "Minimal Pill", color: "#888888", bg: "rgba(0,0,0,0.85)", textColor: "#FFFFFF", highlightColor: "#FFFFFF" },
  { id: "cinematic", label: "Cinematic Gold", color: "#FFD700", bg: "transparent", textColor: "#FFD700", highlightColor: "#FFFFFF" },
  { id: "cyber",   label: "Cyber Lime", color: "#39FF14", bg: "transparent", textColor: "#39FF14", highlightColor: "#00FFFF" },
  { id: "clean",   label: "Clean Dark", color: "#FFE000", bg: "rgba(20,20,22,0.9)", textColor: "#FFFFFF", highlightColor: "#FFE000" },
];

export default function Home() {
  const nav = useNavigate();
  const fileRef = useRef<HTMLInputElement>(null);
  const [tab, setTab] = useState<"url" | "upload">("url");
  const [url, setUrl] = useState("");
  const [file, setFile] = useState<File | null>(null);
  const [dragOver, setDragOver] = useState(false);
  const [uploadPct, setUploadPct] = useState(0);
  const [aspectRatio, setAspectRatio] = useState("9:16");
  const [numClips, setNumClips] = useState(5);
  const [clipInputStr, setClipInputStr] = useState("5");
  const [captionTemplate, setCaptionTemplate] = useState("default");
  const [addSubtitles, setAddSubtitles] = useState(true);
  const [showHookTitle, setShowHookTitle] = useState(false);
  const [mode, setMode] = useState<"fast" | "quality">("fast");
  const [autoVerticalReframe, setAutoVerticalReframe] = useState(false);
  const [reframePreset, setReframePreset] = useState("talking_head");
  const [originalityBoost, setOriginalityBoost] = useState("none");
  const [customBrightness, setCustomBrightness] = useState(0.05);
  const [customContrast, setCustomContrast] = useState(1.08);
  const [customSaturation, setCustomSaturation] = useState(1.10);
  const [translateLanguage, setTranslateLanguage] = useState("");
  const [loading, setLoading] = useState(false);
  const [inputMode, setInputMode] = useState<"form" | "ai">("form");
  const [aiInstruction, setAiInstruction] = useState("");

  const updateClipCount = (val: number) => {
    const clamped = Math.min(30, Math.max(1, val));
    setNumClips(clamped);
    setClipInputStr(clamped.toString());
  };

  const handleInputChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    const val = e.target.value;
    setClipInputStr(val);
    if (val === "") return;
    const parsed = parseInt(val, 10);
    if (!isNaN(parsed)) {
      setNumClips(Math.min(30, Math.max(1, parsed)));
    }
  };

  const handleInputBlur = () => {
    if (clipInputStr === "" || isNaN(parseInt(clipInputStr, 10))) {
      updateClipCount(5);
    } else {
      updateClipCount(parseInt(clipInputStr, 10));
    }
  };

  const handleDrop = useCallback((e: React.DragEvent) => {
    e.preventDefault();
    setDragOver(false);
    const f = e.dataTransfer.files[0];
    if (f && f.type.startsWith("video/")) setFile(f);
    else toast.error("Please drop a video file");
  }, []);

  const submit = async () => {
    if (loading) return;
    setLoading(true);
    try {
      let sourceUrl = "";
      if (tab === "url") {
        if (!url.trim()) { toast.error("Enter a YouTube URL"); setLoading(false); return; }
        sourceUrl = url.trim();
      } else {
        if (!file) { toast.error("Select a video file"); setLoading(false); return; }
        const uploadResult = await api.uploadVideo(file, setUploadPct);
        sourceUrl = uploadResult.video_path;
      }

      const result = await api.createTask({
        url: sourceUrl,
        aspect_ratio: aspectRatio,
        num_clips: numClips,
        caption_template: captionTemplate,
        add_subtitles: addSubtitles,
        processing_mode: mode,
        auto_vertical_reframe: autoVerticalReframe,
        reframe_preset: reframePreset,
        originality_boost: originalityBoost === "custom"
          ? `custom:${customBrightness}:${customContrast}:${customSaturation}`
          : originalityBoost,
        translate_language: translateLanguage,
      });

      toast.success("Task created! Processing started.");
      nav(`/task/${result.task_id}`);
    } catch (e: any) {
      toast.error(e.message || "Failed to create task");
    } finally {
      setLoading(false);
      setUploadPct(0);
    }
  };

  const aiSubmit = async () => {
    if (loading || !aiInstruction.trim()) return;
    setLoading(true);
    try {
      let sourceUrl = "";
      if (tab === "url") {
        if (!url.trim()) { toast.error("Enter a YouTube URL"); setLoading(false); return; }
        sourceUrl = url.trim();
      } else {
        if (!file) { toast.error("Select a video file"); setLoading(false); return; }
        const uploadResult = await api.uploadVideo(file, setUploadPct);
        sourceUrl = uploadResult.video_path;
      }
      const result = await api.aiPrompt(sourceUrl, aiInstruction.trim());
      toast.success("AI understood! Creating clips...");
      nav(`/task/${result.task_id}`);
    } catch (e: any) {
      toast.error(e.message || "Failed to create task");
    } finally {
      setLoading(false);
      setUploadPct(0);
    }
  };

  return (
    <div style={{ minHeight: "100vh", paddingTop: "64px", background: "#09090b", color: "#fff" }}>
      {/* Hero Control Panel Section (Bespoke Full-Width Workspace) */}
      <section style={{ padding: "3rem 0 4rem" }}>
        <div className="container" style={{ maxWidth: "980px", margin: "0 auto", padding: "0 1.5rem" }}>
          
          {/* Header Title */}
          <motion.div
            initial={{ opacity: 0, y: 16 }}
            animate={{ opacity: 1, y: 0 }}
            transition={{ duration: 0.4 }}
            style={{ textAlign: "center", marginBottom: "2.5rem" }}
          >
            <h1 style={{ fontSize: "clamp(1.8rem, 3.8vw, 3.2rem)", fontWeight: 900, lineHeight: 1.1, marginBottom: "0.85rem", letterSpacing: "-0.03em", whiteSpace: "nowrap" }}>
              Turn Long Videos Into <span style={{ color: "var(--accent)", textShadow: "0 0 35px rgba(255,224,0,0.3)" }}>Viral Short Clips</span>
            </h1>
            <p style={{ fontSize: "1.05rem", color: "#a1a1aa", maxWidth: "600px", margin: "0 auto" }}>
              Automatic AI hook extraction, virality scoring, and multi-color animated karaoke captions engineered for max engagement.
            </p>
          </motion.div>

          {/* Bespoke Studio Control Panel Card */}
          <motion.div
            initial={{ opacity: 0, y: 24 }}
            animate={{ opacity: 1, y: 0 }}
            transition={{ duration: 0.5, delay: 0.1 }}
            style={{
              background: "#131318",
              border: "1px solid rgba(255, 255, 255, 0.12)",
              borderRadius: "24px",
              padding: "2rem",
              boxShadow: "0 30px 60px -15px rgba(0,0,0,0.8)",
            }}
          >
            {/* Input Method Tabs */}
            <div style={{ display: "flex", gap: "0.5rem", marginBottom: "1.5rem", background: "#08080a", borderRadius: "14px", padding: "5px", border: "1px solid rgba(255,255,255,0.06)" }}>
              {(["url", "upload"] as const).map(t => (
                <button
                  key={t}
                  className={`tab-btn ${tab === t ? "active" : ""}`}
                  style={{
                    flex: 1, padding: "0.7rem", fontSize: "0.9rem", fontWeight: 700, borderRadius: "10px", transition: "all 0.2s",
                    background: tab === t ? "var(--accent)" : "transparent",
                    color: tab === t ? "#000" : "#aaa",
                    border: "none", cursor: "pointer",
                    display: "flex", alignItems: "center", justifyContent: "center", gap: "0.5rem",
                  }}
                  onClick={() => setTab(t)}
                >
                  {t === "url" ? <><Link2 size={16} />YouTube URL</> : <><Upload size={16} />Upload Video File</>}
                </button>
              ))}
            </div>

            {/* URL or Upload Input */}
            {tab === "url" ? (
              <div style={{ marginBottom: "1.75rem" }}>
                <input
                  className="input input-xl"
                  value={url}
                  onChange={e => setUrl(e.target.value)}
                  placeholder="Paste YouTube video link (e.g. https://youtube.com/watch?v=...)"
                  onKeyDown={e => e.key === "Enter" && submit()}
                  id="youtube-url-input"
                  style={{
                    background: "#08080a",
                    border: "1px solid rgba(255, 255, 255, 0.16)",
                    color: "#fff",
                    width: "100%",
                    borderRadius: "12px",
                    padding: "1rem 1.25rem",
                    fontSize: "0.95rem",
                  }}
                />
              </div>
            ) : (
              <div style={{ marginBottom: "1.75rem" }}>
                <div
                  className={`upload-zone ${dragOver ? "drag-over" : ""}`}
                  onClick={() => fileRef.current?.click()}
                  onDragOver={e => { e.preventDefault(); setDragOver(true); }}
                  onDragLeave={() => setDragOver(false)}
                  onDrop={handleDrop}
                  id="video-upload-zone"
                  style={{
                    background: "#08080a",
                    border: "2px dashed rgba(255, 255, 255, 0.2)",
                    borderRadius: "14px",
                    padding: "2rem",
                    textAlign: "center",
                    cursor: "pointer",
                  }}
                >
                  <Upload size={32} style={{ marginBottom: "0.6rem", color: "var(--accent)" }} />
                  {file ? (
                    <p style={{ fontWeight: 700, color: "#fff", fontSize: "1rem" }}>{file.name}</p>
                  ) : (
                    <>
                      <p style={{ fontWeight: 700, marginBottom: "0.3rem", fontSize: "0.95rem" }}>Drop your video file here</p>
                      <p style={{ fontSize: "0.8rem", color: "#888" }}>Supports MP4, MOV, WebM up to 1GB</p>
                    </>
                  )}
                </div>
                <input ref={fileRef} type="file" accept="video/*" hidden onChange={e => e.target.files?.[0] && setFile(e.target.files[0])} />
              </div>
            )}

            {/* Input Mode Toggle: Form vs AI */}
            <div style={{ display: "flex", gap: "0.5rem", marginBottom: "1.5rem", background: "#08080a", borderRadius: "10px", padding: "4px", border: "1px solid rgba(255,255,255,0.06)" }}>
              <button
                onClick={() => setInputMode("form")}
                style={{
                  flex: 1, padding: "0.5rem", fontSize: "0.82rem", fontWeight: 700, borderRadius: "8px",
                  background: inputMode === "form" ? "var(--accent)" : "transparent",
                  color: inputMode === "form" ? "#000" : "#aaa",
                  border: "none", cursor: "pointer",
                  display: "flex", alignItems: "center", justifyContent: "center", gap: "0.4rem",
                }}
              >
                <FormInput size={15} /> Form
              </button>
              <button
                onClick={() => setInputMode("ai")}
                style={{
                  flex: 1, padding: "0.5rem", fontSize: "0.82rem", fontWeight: 700, borderRadius: "8px",
                  background: inputMode === "ai" ? "var(--accent)" : "transparent",
                  color: inputMode === "ai" ? "#000" : "#aaa",
                  border: "none", cursor: "pointer",
                  display: "flex", alignItems: "center", justifyContent: "center", gap: "0.4rem",
                }}
              >
                <MessageSquareText size={15} /> AI Chat
              </button>
            </div>

            {/* AI Chat Mode */}
            {inputMode === "ai" && (
              <div style={{ marginBottom: "1.75rem" }}>
                <div style={{
                  background: "#08080a", border: "1px solid rgba(255,255,255,0.1)",
                  borderRadius: "14px", padding: "1rem",
                }}>
                  <textarea
                    value={aiInstruction}
                    onChange={e => setAiInstruction(e.target.value)}
                    placeholder='Describe what you want — e.g. "Find the 5 most viral moments, make them vertical with captions, apply a balanced originality boost, and add reaction emojis"'
                    rows={4}
                    style={{
                      width: "100%", background: "transparent", border: "none",
                      color: "#fff", fontSize: "0.9rem", resize: "none",
                      outline: "none", fontFamily: "inherit", lineHeight: 1.5,
                    }}
                  />
                </div>
                <button
                  className="btn btn-primary btn-lg"
                  style={{
                    width: "100%", marginTop: "0.75rem",
                    background: "var(--accent)", color: "#000", fontWeight: 900,
                    fontSize: "1rem", borderRadius: "12px", border: "none",
                    padding: "0.85rem", boxShadow: "0 0 25px rgba(255,224,0,0.25)",
                    cursor: "pointer", opacity: loading || !aiInstruction.trim() ? 0.5 : 1,
                  }}
                  onClick={aiSubmit}
                  disabled={loading || !aiInstruction.trim()}
                >
                  {loading ? <><div className="spinner" style={{ borderColor: "#000", borderTopColor: "transparent" }} /><span>AI is thinking...</span></>
                    : <><Sparkles size={18} /> Let AI Create Clips</>}
                </button>
              </div>
            )}

            {/* Grid of Control Settings */}
            {inputMode === "form" && <><div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: "1.5rem", marginBottom: "1.75rem" }}>
              {/* Aspect Ratio */}
              <div>
                <label style={{ display: "block", fontSize: "0.82rem", color: "#a1a1aa", marginBottom: "0.6rem", fontWeight: 700 }}>Aspect Ratio</label>
                <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: "0.5rem" }}>
                  {ASPECT_RATIOS.map(ar => {
                    const IconComponent = ar.Icon;
                    const isSelected = aspectRatio === ar.id;
                    return (
                      <button
                        key={ar.id}
                        onClick={() => setAspectRatio(ar.id)}
                        style={{
                          padding: "0.65rem 0.5rem",
                          borderRadius: "10px",
                          border: `1px solid ${isSelected ? "var(--accent)" : "rgba(255,255,255,0.08)"}`,
                          background: isSelected ? "rgba(255, 224, 0, 0.15)" : "#08080a",
                          cursor: "pointer",
                          display: "flex",
                          alignItems: "center",
                          gap: "0.6rem",
                          transition: "all 0.15s",
                        }}
                      >
                        <IconComponent size={16} color={isSelected ? "var(--accent)" : "#aaa"} />
                        <div style={{ textAlign: "left" }}>
                          <div style={{ fontSize: "0.82rem", fontWeight: 800, color: isSelected ? "var(--accent)" : "#fff" }}>{ar.label}</div>
                          <div style={{ fontSize: "0.68rem", color: "#888" }}>{ar.sublabel}</div>
                        </div>
                      </button>
                    );
                  })}
                </div>
              </div>

              {/* Custom Bespoke Number of Clips Controls */}
              <div>
                <label style={{ display: "flex", justifyContent: "space-between", fontSize: "0.82rem", color: "#a1a1aa", fontWeight: 700, marginBottom: "0.6rem" }}>
                  <span>Clips to Extract</span>
                  <span style={{ color: "var(--accent)", fontWeight: 800 }}>Max 30</span>
                </label>
                
                <div style={{ display: "flex", flexDirection: "column", gap: "0.75rem" }}>
                  {/* Slider + Custom Stepper Box */}
                  <div style={{ display: "flex", alignItems: "center", gap: "0.85rem", background: "#08080a", padding: "0.5rem 0.75rem", borderRadius: "12px", border: "1px solid rgba(255,255,255,0.08)" }}>
                    <input
                      type="range" min={1} max={30} value={numClips}
                      onChange={e => updateClipCount(+e.target.value)}
                      style={{ flex: 1, accentColor: "var(--accent)", cursor: "pointer" }}
                    />
                    
                    {/* Bespoke Numeric Stepper Buttons */}
                    <div style={{ display: "flex", alignItems: "center", background: "#131318", border: "1px solid rgba(255,255,255,0.15)", borderRadius: "8px", overflow: "hidden" }}>
                      <button
                        type="button"
                        onClick={() => updateClipCount(numClips - 1)}
                        style={{ padding: "0.4rem 0.5rem", background: "transparent", border: "none", color: "#aaa", cursor: "pointer", display: "flex", alignItems: "center" }}
                        title="Decrease"
                      >
                        <Minus size={14} />
                      </button>
                      <input
                        type="text"
                        value={clipInputStr}
                        onChange={handleInputChange}
                        onBlur={handleInputBlur}
                        style={{
                          width: "36px",
                          textAlign: "center",
                          background: "transparent",
                          border: "none",
                          color: "var(--accent)",
                          fontWeight: 900,
                          fontSize: "0.95rem",
                          outline: "none",
                        }}
                      />
                      <button
                        type="button"
                        onClick={() => updateClipCount(numClips + 1)}
                        style={{ padding: "0.4rem 0.5rem", background: "transparent", border: "none", color: "#aaa", cursor: "pointer", display: "flex", alignItems: "center" }}
                        title="Increase"
                      >
                        <Plus size={14} />
                      </button>
                    </div>
                  </div>
                </div>
              </div>
            </div>

            {/* Toggles Bar */}
            <div style={{ display: "flex", gap: "1.5rem", alignItems: "center", flexWrap: "wrap", marginBottom: "1.5rem", background: "#08080a", padding: "0.75rem 1rem", borderRadius: "12px", border: "1px solid rgba(255,255,255,0.06)" }}>
              <label style={{ display: "flex", alignItems: "center", gap: "0.6rem", cursor: "pointer", fontSize: "0.85rem", color: "#ddd", fontWeight: 600 }}>
                <input type="checkbox" checked={addSubtitles} onChange={e => setAddSubtitles(e.target.checked)} style={{ accentColor: "var(--accent)", width: 16, height: 16 }} />
                <span>Burn Karaoke Captions</span>
              </label>
              {addSubtitles && (
                <label style={{ display: "flex", alignItems: "center", gap: "0.6rem", cursor: "pointer", fontSize: "0.85rem", color: "#ddd", fontWeight: 600 }}>
                  <input type="checkbox" checked={showHookTitle} onChange={e => setShowHookTitle(e.target.checked)} style={{ accentColor: "var(--accent)", width: 16, height: 16 }} />
                  <span>Show Hook Title Header</span>
                </label>
              )}
              <div style={{ width: "1px", height: "24px", background: "rgba(255,255,255,0.1)" }} />
              <label style={{ display: "flex", alignItems: "center", gap: "0.6rem", cursor: "pointer", fontSize: "0.85rem", color: "#ddd", fontWeight: 600 }}>
                <input type="checkbox" checked={autoVerticalReframe} onChange={e => setAutoVerticalReframe(e.target.checked)} style={{ accentColor: "var(--accent)", width: 16, height: 16 }} />
                <Cpu size={14} />
                <span>AI Vertical Reframe</span>
              </label>
              {autoVerticalReframe && (
                <select
                  value={reframePreset}
                  onChange={e => setReframePreset(e.target.value)}
                  style={{
                    background: "#131318", color: "#fff", border: "1px solid rgba(255,255,255,0.15)",
                    borderRadius: "8px", padding: "0.3rem 0.6rem", fontSize: "0.78rem", fontWeight: 600,
                    cursor: "pointer",
                  }}
                >
                  <option value="talking_head">Talking Head</option>
                  <option value="sports">Sports</option>
                  <option value="pets">Pets</option>
                  <option value="cars">Cars</option>
                </select>
              )}
            </div>

            {/* Originality Boost + Translate Section */}
            <div style={{ display: "flex", gap: "1.5rem", alignItems: "center", flexWrap: "wrap", marginBottom: "1.5rem", background: "#08080a", padding: "0.75rem 1rem", borderRadius: "12px", border: "1px solid rgba(255,255,255,0.06)" }}>
              <label style={{ display: "flex", alignItems: "center", gap: "0.5rem", fontSize: "0.85rem", color: "#ddd", fontWeight: 600 }}>
                <Wand2 size={14} />
                <span>Originality Boost</span>
              </label>
              <select
                value={originalityBoost === "custom" ? "custom" : originalityBoost}
                onChange={e => setOriginalityBoost(e.target.value)}
                style={{
                  background: "#131318", color: "#fff", border: "1px solid rgba(255,255,255,0.15)",
                  borderRadius: "8px", padding: "0.3rem 0.6rem", fontSize: "0.78rem", fontWeight: 600,
                  cursor: "pointer",
                }}
              >
                <option value="none">Off</option>
                <option value="light">Light</option>
                <option value="balanced">Balanced</option>
                <option value="strong">Strong</option>
                <option value="custom">Custom</option>
              </select>
              {originalityBoost === "custom" && (
                <div style={{ display: "flex", gap: "1rem", alignItems: "center" }}>
                  <label style={{ fontSize: "0.75rem", color: "#aaa" }}>B <input type="range" min="-0.1" max="0.2" step="0.01" value={customBrightness} onChange={e => setCustomBrightness(parseFloat(e.target.value))} style={{ width: 60, verticalAlign: "middle" }} /></label>
                  <label style={{ fontSize: "0.75rem", color: "#aaa" }}>C <input type="range" min="0.8" max="1.5" step="0.01" value={customContrast} onChange={e => setCustomContrast(parseFloat(e.target.value))} style={{ width: 60, verticalAlign: "middle" }} /></label>
                  <label style={{ fontSize: "0.75rem", color: "#aaa" }}>S <input type="range" min="0.8" max="2.0" step="0.01" value={customSaturation} onChange={e => setCustomSaturation(parseFloat(e.target.value))} style={{ width: 60, verticalAlign: "middle" }} /></label>
                </div>
              )}
              <div style={{ width: "1px", height: "24px", background: "rgba(255,255,255,0.1)" }} />
              <label style={{ display: "flex", alignItems: "center", gap: "0.5rem", fontSize: "0.85rem", color: "#ddd", fontWeight: 600 }}>
                <Languages size={14} />
                <span>Translate Captions</span>
              </label>
              <select
                value={translateLanguage}
                onChange={e => setTranslateLanguage(e.target.value)}
                style={{
                  background: "#131318", color: "#fff", border: "1px solid rgba(255,255,255,0.15)",
                  borderRadius: "8px", padding: "0.3rem 0.6rem", fontSize: "0.78rem", fontWeight: 600,
                  cursor: "pointer",
                }}
              >
                <option value="">Original (no translate)</option>
                <option value="ko">Korean</option>
                <option value="ja">Japanese</option>
                <option value="zh">Chinese</option>
                <option value="es">Spanish</option>
                <option value="fr">French</option>
                <option value="de">German</option>
                <option value="pt">Portuguese</option>
              </select>
            </div>

            {/* Subtitles & Live Preview */}
            {addSubtitles && (() => {
              const activeT = CAPTION_TEMPLATES.find(t => t.id === captionTemplate) || CAPTION_TEMPLATES[0];
              return (
                <div style={{ marginBottom: "1.75rem" }}>
                  <label style={{ display: "block", fontSize: "0.82rem", color: "#a1a1aa", fontWeight: 700, marginBottom: "0.6rem" }}>Caption Preset Style</label>
                  <div style={{ display: "flex", gap: "0.4rem", flexWrap: "wrap", marginBottom: "1rem" }}>
                    {CAPTION_TEMPLATES.map(t => (
                      <button key={t.id} onClick={() => setCaptionTemplate(t.id)}
                        style={{
                          padding: "0.4rem 0.85rem", borderRadius: "999px",
                          border: `1px solid ${captionTemplate === t.id ? t.color : "rgba(255,255,255,0.1)"}`,
                          background: captionTemplate === t.id ? `${t.color}25` : "#08080a",
                          color: captionTemplate === t.id ? t.color : "#aaa",
                          cursor: "pointer", fontSize: "0.78rem", fontWeight: 700, transition: "all 0.15s",
                        }}
                      >{t.label}</button>
                    ))}
                  </div>

                  {/* Live Caption Preview Box */}
                  <div
                    style={{
                      background: "#050507",
                      border: "1px solid rgba(255,255,255,0.1)",
                      borderRadius: "14px",
                      padding: "1rem",
                      textAlign: "center",
                    }}
                  >
                    <span style={{ fontSize: "0.68rem", color: "#666", marginBottom: "0.35rem", display: "block", textTransform: "uppercase", letterSpacing: "0.08em" }}>Live Preview</span>
                    <div
                      style={{
                        background: activeT.bg,
                        padding: activeT.bg !== "transparent" ? "0.35rem 0.9rem" : "0",
                        borderRadius: "6px",
                        display: "inline-block",
                        fontFamily: "'THEBOLDFONT', sans-serif",
                        fontSize: "1.25rem",
                        fontWeight: 900,
                        letterSpacing: "0.05em",
                        textShadow: "-2px -2px 0 #000, 2px -2px 0 #000, -2px 2px 0 #000, 2px 2px 0 #000",
                      }}
                    >
                      <span style={{ color: activeT.textColor }}>VIRAL CLIPS </span>
                      <span style={{ color: activeT.highlightColor }}>IN SECONDS</span>
                    </div>
                  </div>
                </div>
              );
            })()}

            {/* Submit Button */}
            <button
              className="btn btn-primary btn-lg"
              style={{ width: "100%", background: "var(--accent)", color: "#000", fontWeight: 900, fontSize: "1.05rem", borderRadius: "14px", border: "none", padding: "0.9rem", boxShadow: "0 0 25px rgba(255,224,0,0.25)", cursor: "pointer" }}
              onClick={submit}
              disabled={loading}
              id="generate-btn"
            >
              {loading ? (
                <><div className="spinner" style={{ borderColor: "#000", borderTopColor: "transparent" }} /><span>Processing...</span></>
              ) : (
                <><Sparkles size={20} /><span>Generate Viral Clips Now</span></>
              )}
            </button>
          </>}
        </motion.div>
        </div>
      </section>

      {/* Bottom Features Strip */}
      <section style={{ borderTop: "1px solid rgba(255, 255, 255, 0.08)", background: "#0b0b0e", padding: "2.2rem 0" }}>
        <div className="container" style={{ maxWidth: "1200px", margin: "0 auto", padding: "0 1.5rem" }}>
          <div style={{ display: "grid", gridTemplateColumns: "repeat(auto-fit, minmax(220px, 1fr))", gap: "2rem", textAlign: "center" }}>
            <div>
              <h4 style={{ fontSize: "1.8rem", fontWeight: 900, color: "var(--accent)", margin: 0 }}>30X</h4>
              <p style={{ fontSize: "0.82rem", color: "#888", marginTop: "0.3rem", margin: 0 }}>Faster Clip Generation</p>
            </div>
            <div>
              <h4 style={{ fontSize: "1.8rem", fontWeight: 900, color: "#22c55e", margin: 0 }}>100%</h4>
              <p style={{ fontSize: "0.82rem", color: "#888", marginTop: "0.3rem", margin: 0 }}>Automated Karaoke Subtitles</p>
            </div>
            <div>
              <h4 style={{ fontSize: "1.8rem", fontWeight: 900, color: "#3b82f6", margin: 0 }}>Up to 30</h4>
              <p style={{ fontSize: "0.82rem", color: "#888", marginTop: "0.3rem", margin: 0 }}>Viral Clips Per Batch</p>
            </div>
            <div>
              <h4 style={{ fontSize: "1.8rem", fontWeight: 900, color: "#a855f7", margin: 0 }}>100% BYOK</h4>
              <p style={{ fontSize: "0.82rem", color: "#888", marginTop: "0.3rem", margin: 0 }}>Offline & Desktop Ready</p>
            </div>
          </div>
        </div>
      </section>
    </div>
  );
}
