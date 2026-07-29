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
  const watermarkInputRef = useRef<HTMLInputElement>(null);
  const [tab, setTab] = useState<"url" | "upload">("url");
  const [url, setUrl] = useState("");
  const [file, setFile] = useState<File | null>(null);
  const [dragOver, setDragOver] = useState(false);
  const [uploadPct, setUploadPct] = useState(0);
  const [aspectRatio, setAspectRatio] = useState("9:16");
  const [numClips, setNumClips] = useState(5);
  const [clipInputStr, setClipInputStr] = useState("5");
  const [captionTemplate, setCaptionTemplate] = useState("default");
  const [fontFamily, setFontFamily] = useState("THEBOLDFONT");
  const [fontSize, setFontSize] = useState(24);
  const [primaryTextColor, setPrimaryTextColor] = useState("#FFFFFF");
  const [highlightAccentColor, setHighlightAccentColor] = useState("#FFE000");
  const [captionAnimation, setCaptionAnimation] = useState("pop");
  const [autoEmojis, setAutoEmojis] = useState(true);
  const [watermarkFile, setWatermarkFile] = useState<File | null>(null);
  const [watermarkPreviewUrl, setWatermarkPreviewUrl] = useState<string | null>(null);
  const [watermarkPosition, setWatermarkPosition] = useState("top_right");
  const [watermarkOpacity, setWatermarkOpacity] = useState(80);
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
        font_family: fontFamily,
        font_size: fontSize,
        font_color: primaryTextColor,
        highlight_color: highlightAccentColor,
        caption_animation: captionAnimation,
        auto_emojis: autoEmojis,
        watermark_position: watermarkPosition,
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

  const activeT = CAPTION_TEMPLATES.find(t => t.id === captionTemplate) || CAPTION_TEMPLATES[0];

  const mockupStyle: Record<string, { width: string; height: string; borderRadius: string; frameStyle: any }> = {
    "9:16": {
      width: "220px",
      height: "410px",
      borderRadius: "38px",
      frameStyle: {
        border: "9px solid #1a1a24",
        boxShadow: "0 25px 50px rgba(0,0,0,0.85), inset 0 0 0 2px rgba(255,255,255,0.12)",
      }
    },
    "1:1": {
      width: "300px",
      height: "300px",
      borderRadius: "22px",
      frameStyle: {
        border: "7px solid #1a1a24",
        boxShadow: "0 20px 40px rgba(0,0,0,0.8), inset 0 0 0 2px rgba(255,255,255,0.12)",
      }
    },
    "16:9": {
      width: "360px",
      height: "205px",
      borderRadius: "18px",
      frameStyle: {
        border: "7px solid #1a1a24",
        boxShadow: "0 20px 40px rgba(0,0,0,0.8), inset 0 0 0 2px rgba(255,255,255,0.12)",
      }
    },
    "original": {
      width: "340px",
      height: "220px",
      borderRadius: "16px",
      frameStyle: {
        border: "4px dashed rgba(255,224,0,0.4)",
        boxShadow: "0 15px 35px rgba(0,0,0,0.7)",
      }
    }
  };

  const currentMockup = mockupStyle[aspectRatio] || mockupStyle["9:16"];

  return (
    <div style={{ minHeight: "100vh", paddingTop: "64px", background: "#09090b", color: "#fff" }}>
      {/* Hero Control Panel Section */}
      <section style={{ padding: "2.5rem 0 4rem" }}>
        <div className="container" style={{ maxWidth: "1280px", margin: "0 auto", padding: "0 1.5rem" }}>
          
          {/* Header Title */}
          <motion.div
            initial={{ opacity: 0, y: 16 }}
            animate={{ opacity: 1, y: 0 }}
            transition={{ duration: 0.4 }}
            style={{ textAlign: "center", marginBottom: "2rem" }}
          >
            <h1 style={{ fontSize: "clamp(1.8rem, 3.8vw, 3.2rem)", fontWeight: 900, lineHeight: 1.1, marginBottom: "0.75rem", letterSpacing: "-0.03em" }}>
              Turn Long Videos Into <span style={{ color: "var(--accent)", textShadow: "0 0 35px rgba(255,224,0,0.3)" }}>Viral Short Clips</span>
            </h1>
            <p style={{ fontSize: "1.05rem", color: "#a1a1aa", maxWidth: "600px", margin: "0 auto" }}>
              Automatic AI hook extraction, virality scoring, and multi-color animated karaoke captions engineered for max engagement.
            </p>
          </motion.div>

          {/* 2-Column Grid Layout: Controls (Left) & Live Preview Mockup (Right) */}
          <div style={{ display: "grid", gridTemplateColumns: "1fr 420px", gap: "2rem", alignItems: "start" }}>
            
            {/* LEFT COLUMN: Controls & Options */}
            <motion.div
              initial={{ opacity: 0, x: -20 }}
              animate={{ opacity: 1, x: 0 }}
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
                <div style={{ marginBottom: "1.5rem" }}>
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
                <div style={{ marginBottom: "1.5rem" }}>
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
                      padding: "1.75rem",
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
                <div style={{ marginBottom: "1.5rem" }}>
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
              {inputMode === "form" && (
                <>
                  <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: "1.25rem", marginBottom: "1.25rem" }}>
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
                                gap: "0.5rem",
                                transition: "all 0.15s",
                              }}
                            >
                              <IconComponent size={16} color={isSelected ? "var(--accent)" : "#aaa"} />
                              <div style={{ textAlign: "left" }}>
                                <div style={{ fontSize: "0.8rem", fontWeight: 800, color: isSelected ? "var(--accent)" : "#fff" }}>{ar.label}</div>
                                <div style={{ fontSize: "0.65rem", color: "#888" }}>{ar.sublabel}</div>
                              </div>
                            </button>
                          );
                        })}
                      </div>
                    </div>

                    {/* Clips to Extract & AI Vertical Reframe */}
                    <div>
                      <label style={{ display: "flex", justifyContent: "space-between", fontSize: "0.82rem", color: "#a1a1aa", fontWeight: 700, marginBottom: "0.6rem" }}>
                        <span>Clips to Extract</span>
                        <span style={{ color: "var(--accent)", fontWeight: 800 }}>Max 30</span>
                      </label>
                      <div style={{ display: "flex", alignItems: "center", gap: "0.75rem", background: "#08080a", padding: "0.55rem 0.75rem", borderRadius: "12px", border: "1px solid rgba(255,255,255,0.08)", marginBottom: "0.65rem" }}>
                        <input
                          type="range" min={1} max={30} value={numClips}
                          onChange={e => updateClipCount(+e.target.value)}
                          style={{ flex: 1, accentColor: "var(--accent)", cursor: "pointer" }}
                        />
                        <div style={{ display: "flex", alignItems: "center", background: "#131318", border: "1px solid rgba(255,255,255,0.15)", borderRadius: "8px", overflow: "hidden" }}>
                          <button
                            type="button"
                            onClick={() => updateClipCount(numClips - 1)}
                            style={{ padding: "0.35rem 0.45rem", background: "transparent", border: "none", color: "#aaa", cursor: "pointer", display: "flex", alignItems: "center" }}
                          >
                            <Minus size={14} />
                          </button>
                          <input
                            type="text"
                            value={clipInputStr}
                            onChange={handleInputChange}
                            onBlur={handleInputBlur}
                            style={{
                              width: "32px", textAlign: "center", background: "transparent",
                              border: "none", color: "var(--accent)", fontWeight: 900,
                              fontSize: "0.9rem", outline: "none",
                            }}
                          />
                          <button
                            type="button"
                            onClick={() => updateClipCount(numClips + 1)}
                            style={{ padding: "0.35rem 0.45rem", background: "transparent", border: "none", color: "#aaa", cursor: "pointer", display: "flex", alignItems: "center" }}
                          >
                            <Plus size={14} />
                          </button>
                        </div>
                      </div>

                      {/* AI Vertical Reframe Button with Checkmark & Subtitle explanation */}
                      <div>
                        <button
                          type="button"
                          disabled={aspectRatio !== "9:16"}
                          onClick={() => aspectRatio === "9:16" && setAutoVerticalReframe(!autoVerticalReframe)}
                          style={{
                            width: "100%",
                            padding: "0.6rem 0.75rem",
                            borderRadius: "10px",
                            border: `1px solid ${aspectRatio !== "9:16" ? "rgba(255,255,255,0.05)" : autoVerticalReframe ? "var(--accent)" : "rgba(255,255,255,0.12)"}`,
                            background: aspectRatio !== "9:16" ? "rgba(255,255,255,0.02)" : autoVerticalReframe ? "rgba(255, 224, 0, 0.15)" : "#08080a",
                            color: aspectRatio !== "9:16" ? "#555" : autoVerticalReframe ? "var(--accent)" : "#aaa",
                            cursor: aspectRatio === "9:16" ? "pointer" : "not-allowed",
                            fontSize: "0.82rem",
                            fontWeight: 800,
                            display: "flex",
                            alignItems: "center",
                            justifyContent: "center",
                            gap: "0.5rem",
                            transition: "all 0.2s",
                            opacity: aspectRatio === "9:16" ? 1 : 0.45,
                          }}
                        >
                          {/* Checkmark icon indicator */}
                          <div style={{
                            width: "16px", height: "16px", borderRadius: "4px",
                            border: `1.5px solid ${autoVerticalReframe && aspectRatio === "9:16" ? "var(--accent)" : "#666"}`,
                            background: autoVerticalReframe && aspectRatio === "9:16" ? "var(--accent)" : "transparent",
                            display: "flex", alignItems: "center", justifyContent: "center", transition: "all 0.15s"
                          }}>
                            {autoVerticalReframe && aspectRatio === "9:16" && <Check size={12} color="#000" strokeWidth={3} />}
                          </div>
                          <Cpu size={15} color={aspectRatio !== "9:16" ? "#555" : autoVerticalReframe ? "var(--accent)" : "#aaa"} />
                          <span>AI Vertical Reframe</span>
                          {aspectRatio !== "9:16" && <span style={{ fontSize: "0.68rem", opacity: 0.7 }}>(9:16 Only)</span>}
                        </button>

                        {/* Explanatory Subtitle Text */}
                        <span style={{ display: "block", fontSize: "0.7rem", color: "#888", marginTop: "0.35rem", textAlign: "center", lineHeight: 1.3 }}>
                          Auto-detects speaker faces & tracks active subject into 9:16 frame.
                        </span>
                      </div>
                    </div>
                  </div>

                  {/* Font Customization & Caption Style Controls */}
                  {addSubtitles && (
                    <div style={{ background: "#08080a", borderRadius: "16px", padding: "1.25rem", border: "1px solid rgba(255,255,255,0.08)", marginBottom: "1.25rem" }}>
                      <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: "0.85rem" }}>
                        <label style={{ fontSize: "0.85rem", color: "#fff", fontWeight: 800, display: "flex", alignItems: "center", gap: "0.5rem" }}>
                          <Sliders size={16} color="var(--accent)" />
                          Font & Style Customization
                        </label>
                        <span style={{ fontSize: "0.72rem", color: "#888" }}>Full Studio Control</span>
                      </div>

                      {/* Caption Presets */}
                      <div style={{ marginBottom: "1rem" }}>
                        <span style={{ display: "block", fontSize: "0.75rem", color: "#aaa", marginBottom: "0.4rem", fontWeight: 600 }}>Caption Preset</span>
                        <div style={{ display: "flex", gap: "0.4rem", flexWrap: "wrap" }}>
                          {CAPTION_TEMPLATES.map(t => (
                            <button key={t.id} onClick={() => {
                              setCaptionTemplate(t.id);
                              setPrimaryTextColor(t.textColor);
                              setHighlightAccentColor(t.highlightColor);
                            }}
                              style={{
                                padding: "0.3rem 0.65rem", borderRadius: "999px",
                                border: `1px solid ${captionTemplate === t.id ? t.color : "rgba(255,255,255,0.1)"}`,
                                background: captionTemplate === t.id ? `${t.color}25` : "#131318",
                                color: captionTemplate === t.id ? t.color : "#aaa",
                                cursor: "pointer", fontSize: "0.75rem", fontWeight: 700, transition: "all 0.15s",
                              }}
                            >{t.label}</button>
                          ))}
                        </div>
                      </div>

                      {/* AI Caption Color Palette Picker */}
                      <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: "1rem", marginBottom: "1rem", background: "#131318", padding: "0.75rem", borderRadius: "10px", border: "1px solid rgba(255,255,255,0.06)" }}>
                        <div>
                          <label style={{ display: "block", fontSize: "0.75rem", color: "#aaa", marginBottom: "0.3rem", fontWeight: 600 }}>Primary Text Color</label>
                          <div style={{ display: "flex", alignItems: "center", gap: "0.5rem" }}>
                            <input
                              type="color" value={primaryTextColor}
                              onChange={e => setPrimaryTextColor(e.target.value)}
                              style={{ width: "32px", height: "32px", border: "none", background: "transparent", cursor: "pointer", borderRadius: "6px" }}
                            />
                            <input
                              type="text" value={primaryTextColor}
                              onChange={e => setPrimaryTextColor(e.target.value)}
                              style={{ flex: 1, background: "#08080a", border: "1px solid rgba(255,255,255,0.15)", borderRadius: "6px", padding: "0.3rem 0.5rem", color: "#fff", fontSize: "0.78rem", textTransform: "uppercase", fontWeight: 700 }}
                            />
                          </div>
                        </div>

                        <div>
                          <label style={{ display: "block", fontSize: "0.75rem", color: "#aaa", marginBottom: "0.3rem", fontWeight: 600 }}>Highlight Accent Color</label>
                          <div style={{ display: "flex", alignItems: "center", gap: "0.5rem" }}>
                            <input
                              type="color" value={highlightAccentColor}
                              onChange={e => setHighlightAccentColor(e.target.value)}
                              style={{ width: "32px", height: "32px", border: "none", background: "transparent", cursor: "pointer", borderRadius: "6px" }}
                            />
                            <input
                              type="text" value={highlightAccentColor}
                              onChange={e => setHighlightAccentColor(e.target.value)}
                              style={{ flex: 1, background: "#08080a", border: "1px solid rgba(255,255,255,0.15)", borderRadius: "6px", padding: "0.3rem 0.5rem", color: "#fff", fontSize: "0.78rem", textTransform: "uppercase", fontWeight: 700 }}
                            />
                          </div>
                        </div>
                      </div>

                      {/* Font Family & Size Grid */}
                      <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: "1rem", marginBottom: "1rem" }}>
                        <div>
                          <label style={{ display: "block", fontSize: "0.75rem", color: "#aaa", marginBottom: "0.3rem", fontWeight: 600 }}>Font Family (12 Fonts)</label>
                          <select
                            value={fontFamily}
                            onChange={e => setFontFamily(e.target.value)}
                            style={{
                              width: "100%", background: "#131318", color: "#fff", border: "1px solid rgba(255,255,255,0.15)",
                              borderRadius: "8px", padding: "0.45rem 0.6rem", fontSize: "0.8rem", fontWeight: 600, cursor: "pointer",
                            }}
                          >
                            <option value="THEBOLDFONT">The Bold Font (Viral Heavy)</option>
                            <option value="TiktokSans">TikTok Sans</option>
                            <option value="Montserrat">Montserrat Black</option>
                            <option value="Impact">Impact Heavy</option>
                            <option value="Bebas Neue">Bebas Neue</option>
                            <option value="Inter">Inter Clean</option>
                            <option value="Roboto">Roboto Condensed</option>
                            <option value="Oswald">Oswald Bold</option>
                            <option value="Poppins">Poppins SemiBold</option>
                            <option value="Anton">Anton Display</option>
                            <option value="Syne">Syne ExtraBold</option>
                            <option value="Courier Prime">Courier Prime Code</option>
                          </select>
                        </div>

                        <div>
                          <label style={{ display: "block", fontSize: "0.75rem", color: "#aaa", marginBottom: "0.3rem", fontWeight: 600 }}>Font Size ({fontSize}px)</label>
                          <input
                            type="range" min={16} max={56} value={fontSize}
                            onChange={e => setFontSize(parseInt(e.target.value, 10))}
                            style={{ width: "100%", accentColor: "var(--accent)", cursor: "pointer", marginTop: "0.5rem" }}
                          />
                        </div>
                      </div>

                      {/* Preset Caption Animations & Auto Emojis */}
                      <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: "1rem" }}>
                        <div>
                          <label style={{ display: "block", fontSize: "0.75rem", color: "#aaa", marginBottom: "0.3rem", fontWeight: 600 }}>Caption Animation</label>
                          <select
                            value={captionAnimation}
                            onChange={e => setCaptionAnimation(e.target.value)}
                            style={{
                              width: "100%", background: "#131318", color: "#fff", border: "1px solid rgba(255,255,255,0.15)",
                              borderRadius: "8px", padding: "0.45rem 0.6rem", fontSize: "0.8rem", fontWeight: 600, cursor: "pointer",
                            }}
                          >
                            <option value="pop">Bouncy Word-by-Word Pop</option>
                            <option value="typewriter">Typewriter Reveal</option>
                            <option value="fade">Smooth Fade-In</option>
                            <option value="slide">Slide Up Reveal</option>
                          </select>
                        </div>

                        <div style={{ display: "flex", alignItems: "center", marginTop: "1.2rem" }}>
                          <label style={{ display: "flex", alignItems: "center", gap: "0.5rem", cursor: "pointer", fontSize: "0.8rem", color: "#ddd", fontWeight: 600 }}>
                            <input
                              type="checkbox"
                              checked={autoEmojis}
                              onChange={e => setAutoEmojis(e.target.checked)}
                              style={{ accentColor: "var(--accent)", width: 16, height: 16 }}
                            />
                            <span>Emoji Auto-Insertion 🔥</span>
                          </label>
                        </div>
                      </div>
                    </div>
                  )}

                  {/* Watermark / Brand Logo Overlay Section */}
                  <div style={{ background: "#08080a", borderRadius: "16px", padding: "1.25rem", border: "1px solid rgba(255,255,255,0.08)", marginBottom: "1.25rem" }}>
                    <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: "0.85rem" }}>
                      <label style={{ fontSize: "0.85rem", color: "#fff", fontWeight: 800, display: "flex", alignItems: "center", gap: "0.5rem" }}>
                        <Sparkles size={16} color="var(--accent)" />
                        Brand Logo & Watermark Overlay
                      </label>
                      <span style={{ fontSize: "0.72rem", color: "#888" }}>PNG / Transparent Logo</span>
                    </div>

                    {/* Dedicated Upload Area */}
                    <div style={{ marginBottom: "1rem" }}>
                      <input
                        ref={watermarkInputRef}
                        type="file" accept="image/png,image/jpeg,image/webp"
                        hidden
                        onChange={e => {
                          const f = e.target.files?.[0] || null;
                          setWatermarkFile(f);
                          if (f) setWatermarkPreviewUrl(URL.createObjectURL(f));
                          else setWatermarkPreviewUrl(null);
                        }}
                      />
                      
                      {watermarkFile && watermarkPreviewUrl ? (
                        <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between", background: "#131318", border: "1px solid rgba(255,224,0,0.3)", borderRadius: "12px", padding: "0.75rem 1rem" }}>
                          <div style={{ display: "flex", alignItems: "center", gap: "0.85rem" }}>
                            <img src={watermarkPreviewUrl} alt="Logo" style={{ width: "36px", height: "36px", objectFit: "contain", borderRadius: "6px", background: "#000", padding: "4px" }} />
                            <div>
                              <div style={{ fontSize: "0.85rem", fontWeight: 700, color: "#fff" }}>{watermarkFile.name}</div>
                              <div style={{ fontSize: "0.7rem", color: "#22c55e", fontWeight: 600 }}>Ready to overlay</div>
                            </div>
                          </div>
                          <button
                            type="button"
                            onClick={() => { setWatermarkFile(null); setWatermarkPreviewUrl(null); if (watermarkInputRef.current) watermarkInputRef.current.value = ""; }}
                            style={{ background: "rgba(239, 68, 68, 0.15)", border: "1px solid rgba(239, 68, 68, 0.3)", color: "#ef4444", borderRadius: "8px", padding: "0.35rem 0.75rem", fontSize: "0.75rem", fontWeight: 700, cursor: "pointer" }}
                          >
                            Remove Logo
                          </button>
                        </div>
                      ) : (
                        <div
                          onClick={() => watermarkInputRef.current?.click()}
                          style={{
                            background: "#131318",
                            border: "2px dashed rgba(255,255,255,0.15)",
                            borderRadius: "12px",
                            padding: "1rem",
                            textAlign: "center",
                            cursor: "pointer",
                            transition: "all 0.2s",
                          }}
                        >
                          <Upload size={24} style={{ color: "var(--accent)", marginBottom: "0.35rem" }} />
                          <div style={{ fontSize: "0.85rem", fontWeight: 800, color: "#fff" }}>Click to Upload Brand / Watermark Logo</div>
                          <div style={{ fontSize: "0.72rem", color: "#888", marginTop: "0.2rem" }}>Supports PNG, WEBP, JPEG up to 10MB</div>
                        </div>
                      )}
                    </div>

                    {/* Position & Opacity Settings */}
                    <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: "1rem" }}>
                      <div>
                        <label style={{ display: "block", fontSize: "0.75rem", color: "#aaa", marginBottom: "0.3rem", fontWeight: 600 }}>Watermark Position</label>
                        <select
                          value={watermarkPosition}
                          onChange={e => setWatermarkPosition(e.target.value)}
                          style={{
                            width: "100%", background: "#131318", color: "#fff", border: "1px solid rgba(255,255,255,0.15)",
                            borderRadius: "8px", padding: "0.45rem 0.6rem", fontSize: "0.78rem", fontWeight: 600, cursor: "pointer",
                          }}
                        >
                          <option value="top_right">Top Right</option>
                          <option value="top_center">Top Center</option>
                          <option value="top_left">Top Left</option>
                          <option value="center">Dead Center</option>
                          <option value="bottom_right">Bottom Right</option>
                          <option value="bottom_center">Bottom Center</option>
                          <option value="bottom_left">Bottom Left</option>
                        </select>
                      </div>
                      <div>
                        <label style={{ display: "block", fontSize: "0.75rem", color: "#aaa", marginBottom: "0.3rem", fontWeight: 600 }}>Opacity ({watermarkOpacity}%)</label>
                        <input
                          type="range" min={10} max={100} value={watermarkOpacity}
                          onChange={e => setWatermarkOpacity(parseInt(e.target.value, 10))}
                          style={{ width: "100%", accentColor: "var(--accent)", cursor: "pointer", marginTop: "0.5rem" }}
                        />
                      </div>
                    </div>
                  </div>

                  {/* Toggles Bar */}
                  <div style={{ display: "flex", gap: "1.25rem", alignItems: "center", flexWrap: "wrap", marginBottom: "1.25rem", background: "#08080a", padding: "0.75rem 1rem", borderRadius: "12px", border: "1px solid rgba(255,255,255,0.06)" }}>
                    <label style={{ display: "flex", alignItems: "center", gap: "0.5rem", cursor: "pointer", fontSize: "0.85rem", color: "#ddd", fontWeight: 600 }}>
                      <input type="checkbox" checked={addSubtitles} onChange={e => setAddSubtitles(e.target.checked)} style={{ accentColor: "var(--accent)", width: 16, height: 16 }} />
                      <span>Burn Karaoke Captions</span>
                    </label>
                    {addSubtitles && (
                      <label style={{ display: "flex", alignItems: "center", gap: "0.5rem", cursor: "pointer", fontSize: "0.85rem", color: "#ddd", fontWeight: 600 }}>
                        <input type="checkbox" checked={showHookTitle} onChange={e => setShowHookTitle(e.target.checked)} style={{ accentColor: "var(--accent)", width: 16, height: 16 }} />
                        <span>Show Hook Title</span>
                      </label>
                    )}
                  </div>

                  {/* Originality Boost & Translation */}
                  <div style={{ display: "flex", gap: "1.25rem", alignItems: "center", flexWrap: "wrap", marginBottom: "1.5rem", background: "#08080a", padding: "0.75rem 1rem", borderRadius: "12px", border: "1px solid rgba(255,255,255,0.06)" }}>
                    <label style={{ display: "flex", alignItems: "center", gap: "0.4rem", fontSize: "0.85rem", color: "#ddd", fontWeight: 600 }}>
                      <Wand2 size={14} /> Boost
                    </label>
                    <select
                      value={originalityBoost === "custom" ? "custom" : originalityBoost}
                      onChange={e => setOriginalityBoost(e.target.value)}
                      style={{
                        background: "#131318", color: "#fff", border: "1px solid rgba(255,255,255,0.15)",
                        borderRadius: "8px", padding: "0.3rem 0.5rem", fontSize: "0.78rem", fontWeight: 600, cursor: "pointer",
                      }}
                    >
                      <option value="none">Off</option>
                      <option value="light">Light</option>
                      <option value="balanced">Balanced</option>
                      <option value="strong">Strong</option>
                    </select>

                    <div style={{ width: "1px", height: "20px", background: "rgba(255,255,255,0.1)" }} />
                    
                    <label style={{ display: "flex", alignItems: "center", gap: "0.4rem", fontSize: "0.85rem", color: "#ddd", fontWeight: 600 }}>
                      <Languages size={14} /> Translate
                    </label>
                    <select
                      value={translateLanguage}
                      onChange={e => setTranslateLanguage(e.target.value)}
                      style={{
                        background: "#131318", color: "#fff", border: "1px solid rgba(255,255,255,0.15)",
                        borderRadius: "8px", padding: "0.3rem 0.5rem", fontSize: "0.78rem", fontWeight: 600, cursor: "pointer",
                      }}
                    >
                      <option value="">Original Language (No Translate)</option>
                      <option value="en">English</option>
                      <option value="es">Spanish (Español)</option>
                      <option value="fr">French (Français)</option>
                      <option value="de">German (Deutsch)</option>
                      <option value="it">Italian (Italiano)</option>
                      <option value="pt">Portuguese (Português)</option>
                      <option value="nl">Dutch (Nederlands)</option>
                      <option value="ru">Russian (Русский)</option>
                      <option value="zh">Chinese (Mandarin Simplified)</option>
                      <option value="zh-TW">Chinese (Traditional)</option>
                      <option value="ja">Japanese (日本語)</option>
                      <option value="ko">Korean (한국어)</option>
                      <option value="ar">Arabic (العربية)</option>
                      <option value="hi">Hindi (हिन्दी)</option>
                      <option value="bn">Bengali (বাংলা)</option>
                      <option value="tr">Turkish (Türkçe)</option>
                      <option value="vi">Vietnamese (Tiếng Việt)</option>
                      <option value="th">Thai (ไทย)</option>
                      <option value="id">Indonesian (Bahasa Indonesia)</option>
                      <option value="pl">Polish (Polski)</option>
                      <option value="uk">Ukrainian (Українська)</option>
                      <option value="sv">Swedish (Svenska)</option>
                    </select>
                  </div>

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
                </>
              )}
            </motion.div>

            {/* RIGHT COLUMN: Live Device Mockup Preview (Fixed & Sticky) */}
            <motion.div
              initial={{ opacity: 0, x: 20 }}
              animate={{ opacity: 1, x: 0 }}
              transition={{ duration: 0.5, delay: 0.2 }}
              style={{
                position: "sticky",
                top: "84px",
                background: "#131318",
                border: "1px solid rgba(255, 255, 255, 0.12)",
                borderRadius: "24px",
                padding: "1.5rem",
                boxShadow: "0 30px 60px -15px rgba(0,0,0,0.8)",
                display: "flex",
                flexDirection: "column",
                alignItems: "center",
              }}
            >
              <div style={{ display: "flex", justifyContent: "space-between", width: "100%", marginBottom: "1.25rem", alignItems: "center" }}>
                <span style={{ fontSize: "0.75rem", color: "var(--accent)", fontWeight: 800, letterSpacing: "0.08em", textTransform: "uppercase", display: "flex", alignItems: "center", gap: "0.4rem" }}>
                  <Sparkles size={14} /> Live Preview ({aspectRatio})
                </span>
                <span style={{ fontSize: "0.68rem", color: "#888", background: "#08080a", padding: "0.2rem 0.6rem", borderRadius: "6px", border: "1px solid rgba(255,255,255,0.06)" }}>
                  {aspectRatio === "9:16" ? "Mobile Phone" : aspectRatio === "1:1" ? "Instagram Square" : aspectRatio === "16:9" ? "YouTube Frame" : "Original"}
                </span>
              </div>

              {/* Mockup Frame Container */}
              <div
                style={{
                  width: "100%",
                  minHeight: "440px",
                  background: "#050507",
                  border: "1px solid rgba(255,255,255,0.08)",
                  borderRadius: "18px",
                  display: "flex",
                  alignItems: "center",
                  justifyContent: "center",
                  padding: "1rem",
                  position: "relative",
                  overflow: "hidden",
                }}
              >
                {/* Framed Mockup Display */}
                <div
                  style={{
                    width: currentMockup.width,
                    height: currentMockup.height,
                    borderRadius: currentMockup.borderRadius,
                    ...currentMockup.frameStyle,
                    background: "linear-gradient(180deg, #111118 0%, #08080c 100%)",
                    position: "relative",
                    display: "flex",
                    flexDirection: "column",
                    alignItems: "center",
                    justifyContent: "center",
                    padding: "1rem",
                    transition: "all 0.3s cubic-bezier(0.4, 0, 0.2, 1)",
                  }}
                >
                  {/* Phone Notch for 9:16 */}
                  {aspectRatio === "9:16" && (
                    <div
                      style={{
                        position: "absolute",
                        top: "8px",
                        width: "60px",
                        height: "12px",
                        background: "#000",
                        borderRadius: "10px",
                        boxShadow: "inset 0 0 2px rgba(255,255,255,0.2)",
                      }}
                    />
                  )}

                  {/* Mockup Video Ambient Backdrop */}
                  <div style={{ opacity: 0.25, position: "absolute", inset: 0, background: "radial-gradient(circle at center, rgba(255,224,0,0.15) 0%, transparent 70%)", pointerEvents: "none" }} />

                  {/* Live Watermark Overlay Mockup */}
                  {watermarkPreviewUrl && (() => {
                    const posStyles: Record<string, React.CSSProperties> = {
                      top_left: { top: "16px", left: "16px" },
                      top_center: { top: "16px", left: "50%", transform: "translateX(-50%)" },
                      top_right: { top: "16px", right: "16px" },
                      center: { top: "50%", left: "50%", transform: "translate(-50%, -50%)" },
                      bottom_left: { bottom: "16px", left: "16px" },
                      bottom_center: { bottom: "16px", left: "50%", transform: "translateX(-50%)" },
                      bottom_right: { bottom: "16px", right: "16px" },
                    };
                    return (
                      <img
                        src={watermarkPreviewUrl}
                        alt="Watermark Preview"
                        style={{
                          position: "absolute",
                          maxHeight: "36px",
                          maxWidth: "80px",
                          objectFit: "contain",
                          opacity: watermarkOpacity / 100,
                          zIndex: 3,
                          pointerEvents: "none",
                          ...(posStyles[watermarkPosition] || posStyles.top_right),
                        }}
                      />
                    );
                  })()}

                  {/* Live Hook Title Header Banner in Mockup */}
                  {showHookTitle && (
                    <div
                      style={{
                        position: "absolute",
                        top: aspectRatio === "9:16" ? "28px" : "12px",
                        left: "50%",
                        transform: "translateX(-50%)",
                        maxWidth: "85%",
                        background: "rgba(0, 0, 0, 0.85)",
                        border: "1px solid var(--accent)",
                        borderRadius: "8px",
                        padding: "0.2rem 0.5rem",
                        color: "var(--accent)",
                        fontSize: "0.62rem",
                        fontWeight: 900,
                        textTransform: "uppercase",
                        letterSpacing: "0.04em",
                        textAlign: "center",
                        overflow: "hidden",
                        textOverflow: "ellipsis",
                        whiteSpace: "nowrap",
                        zIndex: 4,
                        boxShadow: "0 4px 12px rgba(0,0,0,0.6)",
                      }}
                    >
                      ⚡ HOOK: VIRAL SECRET
                    </div>
                  )}

                  {/* Subtitle Caption Text */}
                  {addSubtitles && (
                    <div
                      style={{
                        maxWidth: "90%",
                        background: activeT.bg,
                        padding: activeT.bg !== "transparent" ? "0.35rem 0.75rem" : "0",
                        borderRadius: "8px",
                        fontFamily: fontFamily === "THEBOLDFONT" ? "'THEBOLDFONT', sans-serif" : fontFamily,
                        fontSize: `${Math.min(fontSize, aspectRatio === "16:9" ? 18 : 22)}px`,
                        fontWeight: 900,
                        letterSpacing: "0.04em",
                        textAlign: "center",
                        wordBreak: "break-word",
                        zIndex: 2,
                        textShadow: "-2px -2px 0 #000, 2px -2px 0 #000, -2px 2px 0 #000, 2px 2px 0 #000, 0 4px 12px rgba(0,0,0,0.9)",
                      }}
                    >
                      <span style={{ color: primaryTextColor }}>VIRAL CLIPS </span>
                      <span style={{ color: highlightAccentColor }}>IN SECONDS {autoEmojis ? "🔥" : ""}</span>
                    </div>
                  )}
                </div>
              </div>
            </motion.div>
          </div>

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
