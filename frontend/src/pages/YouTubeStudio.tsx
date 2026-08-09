import { useState } from "react";
import { motion } from "framer-motion";
import {
  Youtube, Image, Wand2, FileText, Sparkles, Check, Copy,
  MessageSquare, RefreshCw, ThumbsUp, Download, Upload, Film, Layers
} from "lucide-react";
import { toast } from "sonner";
import { GEMINI_MODELS, OPENROUTER_MODELS, type LlmProvider } from "../lib/llmModels";

type Tab = "thumbnails" | "titles" | "descriptions";

type ThumbResult = { index?: number; image_url: string; prompt?: string; design_notes?: string };
type TitleRecommended = { index: number; reason: string };

export default function YouTubeStudio() {
  const [tab, setTab] = useState<Tab>("thumbnails");

  // Thumbnail state
  const [thumbTopic, setThumbTopic] = useState("");
  const [thumbStyle, setThumbStyle] = useState("viral");
  const [thumbCount, setThumbCount] = useState(3);
  const [thumbExtra, setThumbExtra] = useState("");
  const [thumbContext, setThumbContext] = useState("");
  const [faceFile, setFaceFile] = useState<File | null>(null);
  const [bgFile, setBgFile] = useState<File | null>(null);
  const [thumbLoading, setThumbLoading] = useState(false);
  const [thumbResults, setThumbResults] = useState<ThumbResult[]>([]);
  const [selectedThumb, setSelectedThumb] = useState(0);

  // Title state
  const [titleTopic, setTitleTopic] = useState("");
  const [titleTranscript, setTitleTranscript] = useState("");
  const [titleVideo, setTitleVideo] = useState<File | null>(null);
  const [titleTone, setTitleTone] = useState("viral");
  const [titleCount, setTitleCount] = useState(10);
  const [titleLoading, setTitleLoading] = useState(false);
  const [titles, setTitles] = useState<string[]>([]);
  const [titleSummary, setTitleSummary] = useState("");
  const [titleRecommended, setTitleRecommended] = useState<TitleRecommended[]>([]);
  const [titleChat, setTitleChat] = useState<{ role: string; text: string }[]>([]);
  const [titleChatInput, setTitleChatInput] = useState("");
  const [titleRefining, setTitleRefining] = useState(false);

  // Description state
  const [descVideoUrl, setDescVideoUrl] = useState("");
  const [descVideoFile, setDescVideoFile] = useState<File | null>(null);
  const [descLoading, setDescLoading] = useState(false);
  const [description, setDescription] = useState("");
  const [chapters, setChapters] = useState<{ time: string; title: string }[]>([]);

  // LLM
  const [llmProvider, setLlmProvider] = useState<LlmProvider>("gemini");
  const [llmModel, setLlmModel] = useState("gemini-3.1-flash-lite");
  const [customLlmModel, setCustomLlmModel] = useState("");

  // Thumbnail image model — thumbnails are rendered by a Gemini IMAGE model
  // (text LLMs like flash-lite and OpenRouter models cannot generate images).
  const [thumbImageModel, setThumbImageModel] = useState("gemini-3.1-flash-image-preview");

  const selectedModel = llmProvider === "gemini" ? llmModel : llmModel === "custom" ? customLlmModel : llmModel;
  const geminiKey = localStorage.getItem("novaclip_gemini_key") || "";

  const handleGenerateThumbnail = async () => {
    if (!thumbTopic.trim()) { toast.error("Enter a topic or title for the thumbnail"); return; }
    setThumbLoading(true);
    try {
      const fd = new FormData();
      fd.append("title", thumbTopic.trim());
      fd.append("style", thumbStyle);
      fd.append("count", String(thumbCount));
      if (thumbExtra.trim()) fd.append("extra_prompt", thumbExtra.trim());
      if (thumbContext.trim()) fd.append("video_context", thumbContext.trim());
      if (faceFile) fd.append("face_image", faceFile);
      if (bgFile) fd.append("bg_image", bgFile);
      fd.append("image_model", thumbImageModel);
      if (geminiKey) fd.append("api_key", geminiKey);
      const res = await fetch("/youtube/generate-thumbnail", { method: "POST", body: fd });
      const data = await res.json();
      if (!res.ok) throw new Error(data.error || "Thumbnail generation failed");
      setThumbResults(data.thumbnails || []);
      setSelectedThumb(0);
      toast.success(`${data.thumbnails?.length || 0} thumbnails generated!`);
    } catch (e: any) {
      toast.error(e.message || "Failed to generate thumbnail");
    } finally {
      setThumbLoading(false);
    }
  };

  const handleDownloadThumb = (url: string) => {
    const a = document.createElement("a");
    a.href = url;
    a.download = `novaclip_thumb_${selectedThumb + 1}.png`;
    a.click();
  };

  const handleGenerateTitles = async () => {
    if (!titleTopic.trim() && !titleTranscript.trim() && !titleVideo) {
      toast.error("Enter a topic, paste a transcript, or upload a video");
      return;
    }
    setTitleLoading(true);
    setTitles([]);
    try {
      const fd = new FormData();
      if (titleTopic.trim()) fd.append("topic", titleTopic.trim());
      if (titleTranscript.trim()) fd.append("transcript", titleTranscript.trim());
      if (titleVideo) fd.append("video_file", titleVideo);
      fd.append("tone", titleTone);
      fd.append("count", String(titleCount));
      fd.append("llm_provider", selectedModel);
      if (geminiKey) fd.append("api_key", geminiKey);
      const res = await fetch("/youtube/generate-titles", { method: "POST", body: fd });
      const data = await res.json();
      if (!res.ok) throw new Error(data.error || "Title generation failed");
      setTitles(data.titles || []);
      setTitleSummary(data.transcript_summary || "");
      setTitleRecommended(data.recommended || []);
      toast.success(`${data.titles?.length || 0} titles generated!`);
    } catch (e: any) {
      toast.error(e.message || "Failed to generate titles");
    } finally {
      setTitleLoading(false);
    }
  };

  const handleRefineTitle = async (direction: string) => {
    if (!titleChatInput.trim() && !direction) return;
    setTitleRefining(true);
    const input = direction || titleChatInput.trim();
    setTitleChatInput("");
    setTitleChat(prev => [...prev, { role: "user", text: input }]);
    try {
      const res = await fetch("/youtube/refine-titles", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          topic: titleTopic.trim(),
          current_titles: titles,
          instruction: input,
          llm_provider: selectedModel,
          api_key: geminiKey,
        }),
      });
      const data = await res.json();
      if (!res.ok) throw new Error(data.error || "Refinement failed");
      setTitles(data.titles || []);
      setTitleChat(prev => [...prev, { role: "assistant", text: data.response || "Titles refined!" }]);
    } catch (e: any) {
      toast.error(e.message || "Refinement failed");
    } finally {
      setTitleRefining(false);
    }
  };

  const handleGenerateDescription = async () => {
    if (!descVideoUrl.trim() && !descVideoFile) {
      toast.error("Enter a YouTube video URL, paste a transcript, or upload a video");
      return;
    }
    setDescLoading(true);
    try {
      const fd = new FormData();
      if (descVideoUrl.trim()) fd.append("video_url", descVideoUrl.trim());
      if (descVideoFile) fd.append("video_file", descVideoFile);
      fd.append("llm_provider", selectedModel);
      if (geminiKey) fd.append("api_key", geminiKey);
      const res = await fetch("/youtube/generate-description", { method: "POST", body: fd });
      const data = await res.json();
      if (!res.ok) throw new Error(data.error || "Description generation failed");
      setDescription(data.description || "");
      setChapters(data.chapters || []);
      toast.success("Description generated!");
    } catch (e: any) {
      toast.error(e.message || "Failed to generate description");
    } finally {
      setDescLoading(false);
    }
  };

  const accentColor = "#ef4444";
  const accentRgb = "239,68,68";

  const fileChip = (file: File | null, label: string) => (
    <div style={{ display: "flex", alignItems: "center", gap: "0.5rem", marginTop: "0.5rem", padding: "0.4rem 0.6rem", background: "rgba(255,255,255,0.05)", borderRadius: "8px", border: "1px solid rgba(255,255,255,0.1)" }}>
      <Film size={13} color={accentColor} />
      <span style={{ flex: 1, fontSize: "0.74rem", color: "#bbb", overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>{file ? file.name : label}</span>
      {file && (
        <button type="button" onClick={() => { if (faceFile === file) setFaceFile(null); if (bgFile === file) setBgFile(null); if (titleVideo === file) setTitleVideo(null); if (descVideoFile === file) setDescVideoFile(null); }}
          style={{ background: "none", border: "none", color: "#888", cursor: "pointer", fontSize: "0.72rem", fontWeight: 700 }}>
          x
        </button>
      )}
    </div>
  );

  return (
    <div style={{ maxWidth: "1100px", margin: "0 auto", padding: "1.5rem 1rem", ["--accent" as any]: accentColor, ["--accent-rgb" as any]: accentRgb }}>
      {/* Header */}
      <div style={{ textAlign: "center", marginBottom: "2.5rem" }}>
        <div style={{ display: "inline-flex", alignItems: "center", gap: "0.5rem", background: "rgba(239,68,68,0.1)", border: "1px solid rgba(239,68,68,0.3)", padding: "0.4rem 1rem", borderRadius: "20px", color: "#ef4444", fontSize: "0.78rem", fontWeight: 800, textTransform: "uppercase", letterSpacing: "0.08em", marginBottom: "2.3rem" }}>
          <Youtube size={14} /> YouTube Studio
        </div>
        <h1 style={{ fontSize: "clamp(1.8rem, 3.8vw, 3.2rem)", fontWeight: 900, lineHeight: 1.1, marginBottom: "0.75rem", letterSpacing: "-0.03em", color: "#fff" }}>
          Complete <span style={{ color: "#ef4444", textShadow: "0 0 35px rgba(239,68,68,0.3)" }}>YouTube Toolkit</span>
        </h1>
        <p style={{ fontSize: "1.05rem", color: "#a1a1aa", maxWidth: "640px", margin: "0 auto" }}>
          AI thumbnails with face &amp; background uploads, video-aware viral titles with refinement chat, and auto-descriptions with chapter timestamps.
        </p>
      </div>

      {/* Tab Switcher */}
      <div style={{ display: "flex", justifyContent: "center", marginBottom: "1.75rem" }}>
        <div style={{ display: "inline-flex", background: "#0c0c0f", border: "1px solid rgba(255,255,255,0.1)", borderRadius: "14px", padding: "0.35rem", gap: "0.3rem" }}>
          {([
            { id: "thumbnails" as Tab, icon: Image, label: "Thumbnails" },
            { id: "titles" as Tab, icon: Wand2, label: "Title Studio" },
            { id: "descriptions" as Tab, icon: FileText, label: "Descriptions" },
          ]).map(({ id, icon: Icon, label }) => (
            <button
              key={id}
              type="button"
              onClick={() => setTab(id)}
              style={{
                display: "flex", alignItems: "center", gap: "0.45rem", padding: "0.55rem 1.1rem", borderRadius: "10px",
                border: "none", cursor: "pointer", fontWeight: 800, fontSize: "0.85rem",
                background: tab === id ? accentColor : "transparent",
                color: tab === id ? "#fff" : "#888",
                boxShadow: tab === id ? "0 0 18px rgba(239,68,68,0.4)" : "none",
              }}
            >
              <Icon size={16} /> {label}
            </button>
          ))}
        </div>
      </div>

      {/* LLM Settings Row */}
      <div style={{ display: "flex", justifyContent: "center", gap: "0.75rem", marginBottom: "1.5rem" }}>
        <select value={llmProvider} onChange={e => { setLlmProvider(e.target.value as LlmProvider); setLlmModel(e.target.value === "gemini" ? "gemini-3.1-flash-lite" : "openrouter/free"); }}
          style={{ background: "#131318", color: "#fff", border: "1px solid rgba(255,255,255,0.15)", borderRadius: "8px", padding: "0.4rem 0.75rem", fontSize: "0.78rem", fontWeight: 600 }}>
          <option value="gemini">Gemini</option>
          <option value="openrouter">OpenRouter</option>
        </select>
        <select value={llmModel} onChange={e => setLlmModel(e.target.value)}
          style={{ background: "#131318", color: "#fff", border: "1px solid rgba(255,255,255,0.15)", borderRadius: "8px", padding: "0.4rem 0.75rem", fontSize: "0.78rem", fontWeight: 600 }}>
          {(llmProvider === "gemini" ? GEMINI_MODELS : OPENROUTER_MODELS).map(p => <option key={p.id} value={p.id}>{p.label}</option>)}
        </select>
        {llmProvider === "openrouter" && llmModel === "custom" && (
          <input type="text" placeholder="custom model ID" value={customLlmModel} onChange={e => setCustomLlmModel(e.target.value)}
            style={{ background: "#131318", color: "#fff", border: "1px solid rgba(255,255,255,0.15)", borderRadius: "8px", padding: "0.4rem 0.75rem", fontSize: "0.78rem", width: "200px" }} />
        )}
      </div>

      {/* ============ THUMBNAILS TAB ============ */}
      {tab === "thumbnails" && (
        <div style={{ display: "grid", gridTemplateColumns: "1fr 460px", gap: "1.75rem", alignItems: "start" }}>
          <motion.div initial={{ opacity: 0, y: 12 }} animate={{ opacity: 1, y: 0 }}>
            <div style={{ background: "#0c0c0f", border: "1px solid rgba(255,255,255,0.08)", borderRadius: "20px", padding: "1.5rem" }}>
              <h3 style={{ fontSize: "0.95rem", fontWeight: 800, color: "#fff", marginBottom: "1.25rem", display: "flex", alignItems: "center", gap: "0.5rem" }}>
                <Image size={16} color={accentColor} /> Generate AI Thumbnails
              </h3>
              <div style={{ marginBottom: "1rem" }}>
                <label style={{ display: "block", fontSize: "0.8rem", color: "#aaa", fontWeight: 700, marginBottom: "0.4rem" }}>Video Title</label>
                <input type="text" className="input" placeholder="e.g., I Tried Dropshipping for 30 Days..." value={thumbTopic} onChange={e => setThumbTopic(e.target.value)}
                  style={{ width: "100%", fontSize: "0.88rem" }} />
              </div>
              <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: "1rem", marginBottom: "1rem" }}>
                <div>
                  <label style={{ display: "block", fontSize: "0.78rem", color: "#aaa", fontWeight: 700, marginBottom: "0.3rem" }}>Thumbnail Style</label>
                  <select value={thumbStyle} onChange={e => setThumbStyle(e.target.value)}
                    style={{ width: "100%", background: "#131318", color: "#fff", border: "1px solid rgba(255,255,255,0.15)", borderRadius: "10px", padding: "0.55rem 0.75rem", fontSize: "0.82rem", fontWeight: 600 }}>
                    <option value="viral">Viral Reaction Face</option>
                    <option value="comparison">Before/After Comparison</option>
                    <option value="text">Bold Text Overlay</option>
                    <option value="minimal">Clean Minimal</option>
                    <option value="step">Step-by-Step Numbers</option>
                  </select>
                </div>
                <div>
                  <label style={{ display: "block", fontSize: "0.78rem", color: "#aaa", fontWeight: 700, marginBottom: "0.3rem" }}>Number of Variants</label>
                  <input type="range" min={1} max={4} value={thumbCount} onChange={e => setThumbCount(+e.target.value)}
                    style={{ width: "100%", accentColor }} />
                  <span style={{ fontSize: "0.72rem", color: "#888" }}>{thumbCount} thumbnail{thumbCount > 1 ? "s" : ""}</span>
                </div>
              </div>
              <div style={{ marginBottom: "0.75rem", padding: "0.6rem 0.75rem", borderRadius: "8px", background: "rgba(239,68,68,0.06)", border: "1px solid rgba(239,68,68,0.2)", color: "#fca5a5", fontSize: "0.72rem", lineHeight: 1.45 }}>
                Thumbnails are rendered by a Gemini <b>image</b> model (text LLMs like Flash-Lite and OpenRouter models cannot generate images). The LLM provider above only applies to titles &amp; descriptions.
              </div>
              <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: "1rem", marginBottom: "1rem" }}>
                <div>
                  <label style={{ display: "block", fontSize: "0.78rem", color: "#aaa", fontWeight: 700, marginBottom: "0.3rem" }}>Thumbnail Image Model</label>
                  <select value={thumbImageModel === "gemini-3.1-flash-image-preview" ? "gemini-3.1-flash-image-preview" : "custom"} onChange={e => setThumbImageModel(e.target.value)}
                    style={{ width: "100%", background: "#131318", color: "#fff", border: "1px solid rgba(255,255,255,0.15)", borderRadius: "10px", padding: "0.55rem 0.75rem", fontSize: "0.82rem", fontWeight: 600 }}>
                    <option value="gemini-3.1-flash-image-preview">Gemini 3.1 Flash Image (Recommended)</option>
                    <option value="custom">Custom Gemini image model...</option>
                  </select>
                  {thumbImageModel !== "gemini-3.1-flash-image-preview" && (
                    <input type="text" className="input" placeholder="e.g. gemini-3.1-pro-image-preview" value={thumbImageModel} onChange={e => setThumbImageModel(e.target.value)}
                      style={{ width: "100%", marginTop: "0.4rem", fontSize: "0.8rem" }} />
                  )}
                </div>
              </div>
              <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: "1rem", marginBottom: "1rem" }}>
                <div>
                  <label style={{ display: "flex", alignItems: "center", gap: "0.4rem", fontSize: "0.78rem", color: "#aaa", fontWeight: 700, marginBottom: "0.4rem" }}>
                    <Upload size={13} /> Face Image
                  </label>
                  <label style={{ display: "block", padding: "0.55rem", textAlign: "center", background: "rgba(255,255,255,0.03)", border: "1px dashed rgba(255,255,255,0.18)", borderRadius: "10px", color: "#888", fontSize: "0.78rem", fontWeight: 700, cursor: "pointer" }}>
                    {faceFile ? "Change face" : "+ Add your face"}
                    <input type="file" accept="image/*" style={{ display: "none" }} onChange={e => setFaceFile(e.target.files?.[0] || null)} />
                  </label>
                  {faceFile && (
                    <img src={URL.createObjectURL(faceFile)} alt="face" style={{ width: "100%", marginTop: "0.5rem", borderRadius: "8px", border: "1px solid rgba(255,255,255,0.1)" }} />
                  )}
                </div>
                <div>
                  <label style={{ display: "flex", alignItems: "center", gap: "0.4rem", fontSize: "0.78rem", color: "#aaa", fontWeight: 700, marginBottom: "0.4rem" }}>
                    <Layers size={13} /> Background
                  </label>
                  <label style={{ display: "block", padding: "0.55rem", textAlign: "center", background: "rgba(255,255,255,0.03)", border: "1px dashed rgba(255,255,255,0.18)", borderRadius: "10px", color: "#888", fontSize: "0.78rem", fontWeight: 700, cursor: "pointer" }}>
                    {bgFile ? "Change background" : "+ Add background"}
                    <input type="file" accept="image/*" style={{ display: "none" }} onChange={e => setBgFile(e.target.files?.[0] || null)} />
                  </label>
                  {bgFile && (
                    <img src={URL.createObjectURL(bgFile)} alt="bg" style={{ width: "100%", marginTop: "0.5rem", borderRadius: "8px", border: "1px solid rgba(255,255,255,0.1)" }} />
                  )}
                </div>
              </div>
              <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: "1rem", marginBottom: "1.25rem" }}>
                <div>
                  <label style={{ display: "block", fontSize: "0.78rem", color: "#aaa", fontWeight: 700, marginBottom: "0.3rem" }}>Video Context (transcript / summary)</label>
                  <textarea rows={3} className="input" placeholder="Optional: paste the transcript or key context so the thumbnail matches the actual content..." value={thumbContext} onChange={e => setThumbContext(e.target.value)}
                    style={{ width: "100%", fontSize: "0.8rem", resize: "vertical" }} />
                </div>
                <div>
                  <label style={{ display: "block", fontSize: "0.78rem", color: "#aaa", fontWeight: 700, marginBottom: "0.3rem" }}>Extra Instructions</label>
                  <textarea rows={3} className="input" placeholder="Optional: e.g. 'make the face shocked', 'use red arrow'..." value={thumbExtra} onChange={e => setThumbExtra(e.target.value)}
                    style={{ width: "100%", fontSize: "0.8rem", resize: "vertical" }} />
                </div>
              </div>
              <button type="button" onClick={handleGenerateThumbnail} disabled={thumbLoading}
                style={{ width: "100%", background: accentColor, color: "#fff", fontWeight: 900, fontSize: "1rem", borderRadius: "14px", border: "none", padding: "0.85rem", boxShadow: "0 0 25px rgba(239,68,68,0.35)", cursor: "pointer", display: "flex", alignItems: "center", justifyContent: "center", gap: "0.5rem" }}>
                {thumbLoading ? <><div className="spinner" style={{ borderColor: "#000", borderTopColor: "transparent" }} /><span>Generating...</span></> : <><Sparkles size={18} /><span>Generate {thumbCount} Thumbnails</span></>}
              </button>
            </div>
          </motion.div>

          <motion.div initial={{ opacity: 0, x: 20 }} animate={{ opacity: 1, x: 0 }} style={{ position: "sticky", top: "84px" }}>
            <div style={{ background: "linear-gradient(180deg, #1a0a0a 0%, #0d0b12 100%)", border: "1px solid rgba(239,68,68,0.28)", borderRadius: "24px", padding: "1.5rem" }}>
              <span style={{ fontSize: "0.75rem", color: accentColor, fontWeight: 800, textTransform: "uppercase", display: "flex", alignItems: "center", gap: "0.4rem", marginBottom: "1rem" }}>
                <Sparkles size={14} /> Preview
              </span>
              {thumbResults.length > 0 ? (
                <>
                  <div style={{ display: "flex", gap: "0.5rem", marginBottom: "0.75rem", flexWrap: "wrap" }}>
                    {thumbResults.map((t, i) => (
                      <button key={i} type="button" onClick={() => setSelectedThumb(i)}
                        style={{ padding: 0, border: selectedThumb === i ? `2px solid ${accentColor}` : "2px solid rgba(255,255,255,0.15)", borderRadius: "8px", overflow: "hidden", cursor: "pointer", background: "none" }}>
                        <img src={t.image_url} alt={`variant ${i + 1}`} style={{ width: "72px", height: "41px", objectFit: "cover", display: "block" }} />
                      </button>
                    ))}
                  </div>
                  <img src={thumbResults[selectedThumb]?.image_url} alt="AI Thumbnail" style={{ width: "100%", borderRadius: "12px", border: "1px solid rgba(255,255,255,0.1)" }} />
                  <div style={{ display: "flex", gap: "0.5rem", marginTop: "0.75rem" }}>
                    <button type="button" onClick={() => handleDownloadThumb(thumbResults[selectedThumb].image_url)}
                      style={{ flex: 1, display: "flex", alignItems: "center", justifyContent: "center", gap: "0.4rem", background: "rgba(239,68,68,0.15)", border: "1px solid rgba(239,68,68,0.35)", color: "#fca5a5", borderRadius: "8px", padding: "0.5rem", fontSize: "0.78rem", fontWeight: 800, cursor: "pointer" }}>
                      <Download size={14} /> Download
                    </button>
                    <button type="button" onClick={() => { navigator.clipboard.writeText(thumbResults[selectedThumb].image_url); toast.success("Image data URL copied!"); }}
                      style={{ flex: 1, display: "flex", alignItems: "center", justifyContent: "center", gap: "0.4rem", background: "rgba(255,255,255,0.06)", border: "1px solid rgba(255,255,255,0.12)", color: "#aaa", borderRadius: "8px", padding: "0.5rem", fontSize: "0.78rem", fontWeight: 800, cursor: "pointer" }}>
                      <Copy size={14} /> Copy
                    </button>
                  </div>
                  {thumbResults[selectedThumb]?.prompt && (
                    <div style={{ marginTop: "0.75rem", padding: "0.6rem 0.75rem", background: "rgba(0,0,0,0.3)", borderRadius: "8px", fontSize: "0.72rem", color: "#888", lineHeight: 1.4 }}>
                      <span style={{ color: "#aaa", fontWeight: 600 }}>Prompt: </span>{thumbResults[selectedThumb].prompt}
                    </div>
                  )}
                </>
              ) : (
                <div style={{ aspectRatio: "16/9", background: "#0c0c0f", borderRadius: "12px", border: "2px dashed rgba(255,255,255,0.1)", display: "flex", alignItems: "center", justifyContent: "center", color: "#555", fontSize: "0.85rem", fontWeight: 600 }}>
                  AI thumbnails will appear here
                </div>
              )}
            </div>
          </motion.div>
        </div>
      )}

      {/* ============ TITLES TAB ============ */}
      {tab === "titles" && (
        <div style={{ display: "grid", gridTemplateColumns: "1fr 400px", gap: "1.75rem", alignItems: "start" }}>
          <motion.div initial={{ opacity: 0, y: 12 }} animate={{ opacity: 1, y: 0 }}>
            <div style={{ background: "#0c0c0f", border: "1px solid rgba(255,255,255,0.08)", borderRadius: "20px", padding: "1.5rem", marginBottom: "1.25rem" }}>
              <h3 style={{ fontSize: "0.95rem", fontWeight: 800, color: "#fff", marginBottom: "1.25rem", display: "flex", alignItems: "center", gap: "0.5rem" }}>
                <Wand2 size={16} color={accentColor} /> Generate Viral Titles
              </h3>
              <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: "1rem", marginBottom: "1rem" }}>
                <div>
                  <label style={{ display: "block", fontSize: "0.78rem", color: "#aaa", fontWeight: 700, marginBottom: "0.3rem" }}>Video Topic</label>
                  <input type="text" className="input" placeholder="e.g., How I Built a SaaS in 7 Days" value={titleTopic} onChange={e => setTitleTopic(e.target.value)}
                    style={{ width: "100%", fontSize: "0.85rem" }} />
                </div>
                <div>
                  <label style={{ display: "block", fontSize: "0.78rem", color: "#aaa", fontWeight: 700, marginBottom: "0.3rem" }}>Tone & Style</label>
                  <select value={titleTone} onChange={e => setTitleTone(e.target.value)}
                    style={{ width: "100%", background: "#131318", color: "#fff", border: "1px solid rgba(255,255,255,0.15)", borderRadius: "8px", padding: "0.45rem 0.6rem", fontSize: "0.8rem" }}>
                    <option value="viral">Viral / Clickbaity</option>
                    <option value="educational">Educational / How-To</option>
                    <option value="story">Story-Driven</option>
                    <option value="controversial">Controversial / Debate</option>
                    <option value="listicle">Listicle / Top 10</option>
                  </select>
                </div>
              </div>
              <div style={{ marginBottom: "1rem" }}>
                <label style={{ display: "block", fontSize: "0.78rem", color: "#aaa", fontWeight: 700, marginBottom: "0.3rem" }}>Number of Titles</label>
                <input type="range" min={5} max={20} value={titleCount} onChange={e => setTitleCount(+e.target.value)}
                  style={{ width: "100%", accentColor }} />
                <span style={{ fontSize: "0.72rem", color: "#888" }}>{titleCount} titles</span>
              </div>
              <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: "1rem", marginBottom: "1.25rem" }}>
                <div>
                  <label style={{ display: "block", fontSize: "0.78rem", color: "#aaa", fontWeight: 700, marginBottom: "0.3rem" }}>Upload Video (optional)</label>
                  <label style={{ display: "block", padding: "0.55rem", textAlign: "center", background: "rgba(255,255,255,0.03)", border: "1px dashed rgba(255,255,255,0.18)", borderRadius: "10px", color: "#888", fontSize: "0.78rem", fontWeight: 700, cursor: "pointer" }}>
                    <Upload size={13} style={{ verticalAlign: "middle", marginRight: "0.3rem" }} /> {titleVideo ? titleVideo.name : "+ Upload video for real content analysis"}
                    <input type="file" accept="video/*" style={{ display: "none" }} onChange={e => setTitleVideo(e.target.files?.[0] || null)} />
                  </label>
                </div>
                <div>
                  <label style={{ display: "block", fontSize: "0.78rem", color: "#aaa", fontWeight: 700, marginBottom: "0.3rem" }}>Paste Transcript (optional)</label>
                  <textarea rows={4} className="input" placeholder="Paste the video transcript for content-aware titles..." value={titleTranscript} onChange={e => setTitleTranscript(e.target.value)}
                    style={{ width: "100%", fontSize: "0.78rem", resize: "vertical" }} />
                </div>
              </div>
              <button type="button" onClick={handleGenerateTitles} disabled={titleLoading}
                style={{ width: "100%", background: accentColor, color: "#fff", fontWeight: 900, fontSize: "1rem", borderRadius: "14px", border: "none", padding: "0.85rem", boxShadow: "0 0 25px rgba(239,68,68,0.35)", cursor: "pointer", display: "flex", alignItems: "center", justifyContent: "center", gap: "0.5rem" }}>
                {titleLoading ? <><div className="spinner" style={{ borderColor: "#000", borderTopColor: "transparent" }} /><span>Generating...</span></> : <><Sparkles size={18} /><span>Generate {titleCount} Titles</span></>}
              </button>
            </div>

            {titleSummary && (
              <div style={{ background: "rgba(239,68,68,0.08)", border: "1px solid rgba(239,68,68,0.25)", borderRadius: "14px", padding: "1rem", marginBottom: "1.25rem" }}>
                <span style={{ fontSize: "0.72rem", color: accentColor, fontWeight: 800, textTransform: "uppercase", marginBottom: "0.3rem", display: "block" }}>Video Summary</span>
                <p style={{ fontSize: "0.82rem", color: "#ddd", lineHeight: 1.5, margin: 0 }}>{titleSummary}</p>
              </div>
            )}

            {/* Generated titles list */}
            {titles.length > 0 && (
              <div style={{ background: "#0c0c0f", border: "1px solid rgba(255,255,255,0.08)", borderRadius: "20px", padding: "1.5rem" }}>
                <h3 style={{ fontSize: "0.9rem", fontWeight: 800, color: "#fff", marginBottom: "1rem" }}>
                  {titles.length} Title Ideas
                </h3>
                <div style={{ display: "flex", flexDirection: "column", gap: "0.5rem" }}>
                  {titles.map((t, i) => {
                    const rec = titleRecommended.find(r => r.index === i);
                    return (
                      <div key={i} style={{ display: "flex", alignItems: "center", gap: "0.6rem", padding: "0.6rem 0.75rem", background: rec ? "rgba(239,68,68,0.08)" : "rgba(255,255,255,0.03)", borderRadius: "10px", border: rec ? "1px solid rgba(239,68,68,0.35)" : "1px solid rgba(255,255,255,0.06)" }}>
                        <span style={{ color: accentColor, fontSize: "0.72rem", fontWeight: 900, minWidth: "24px" }}>{i + 1}</span>
                        <div style={{ flex: 1 }}>
                          <span style={{ fontSize: "0.85rem", color: "#ddd", lineHeight: 1.3 }}>{t}</span>
                          {rec && (
                            <div style={{ display: "flex", alignItems: "center", gap: "0.3rem", marginTop: "0.2rem", fontSize: "0.68rem", color: "#fca5a5" }}>
                              <Check size={11} /> {rec.reason}
                            </div>
                          )}
                        </div>
                        <button type="button" onClick={() => { navigator.clipboard.writeText(t); toast.success("Copied!"); }}
                          style={{ background: "none", border: "none", color: "#888", cursor: "pointer", padding: "0.25rem" }}>
                          <Copy size={14} />
                        </button>
                      </div>
                    );
                  })}
                </div>
              </div>
            )}
          </motion.div>

          {/* Title refinement chat */}
          <motion.div initial={{ opacity: 0, x: 20 }} animate={{ opacity: 1, x: 0 }} style={{ position: "sticky", top: "84px" }}>
            <div style={{ background: "linear-gradient(180deg, #1a0a0a 0%, #0d0b12 100%)", border: "1px solid rgba(239,68,68,0.28)", borderRadius: "24px", padding: "1.5rem" }}>
              <span style={{ fontSize: "0.75rem", color: accentColor, fontWeight: 800, textTransform: "uppercase", display: "flex", alignItems: "center", gap: "0.4rem", marginBottom: "1rem" }}>
                <MessageSquare size={14} /> Refine Titles
              </span>
              {titles.length === 0 ? (
                <p style={{ fontSize: "0.78rem", color: "#666", lineHeight: 1.5 }}>
                  Generate titles first, then refine them here with instructions like "make them shorter", "add more urgency", or "focus on the money angle".
                </p>
              ) : (
                <>
                  <div style={{ display: "flex", flexDirection: "column", gap: "0.5rem", maxHeight: "200px", overflowY: "auto", marginBottom: "0.75rem" }}>
                    {titleChat.map((m, i) => (
                      <div key={i} style={{ padding: "0.4rem 0.6rem", borderRadius: "8px", fontSize: "0.78rem", background: m.role === "user" ? "rgba(239,68,68,0.15)" : "rgba(255,255,255,0.04)", color: m.role === "user" ? "#fca5a5" : "#aaa" }}>
                        {m.text}
                      </div>
                    ))}
                  </div>
                  <div style={{ display: "flex", gap: "0.4rem", marginBottom: "0.75rem", flexWrap: "wrap" }}>
                    {["Make them shorter", "Add more urgency", "Focus on results", "Make them funnier"].map(q => (
                      <button key={q} type="button" onClick={() => handleRefineTitle(q)} disabled={titleRefining}
                        style={{ background: "rgba(239,68,68,0.1)", border: "1px solid rgba(239,68,68,0.25)", color: "#fca5a5", borderRadius: "999px", padding: "0.3rem 0.7rem", fontSize: "0.7rem", fontWeight: 700, cursor: "pointer" }}>
                        {q}
                      </button>
                    ))}
                  </div>
                  <div style={{ display: "flex", gap: "0.5rem" }}>
                    <input type="text" placeholder="Custom instruction..." value={titleChatInput} onChange={e => setTitleChatInput(e.target.value)}
                      onKeyDown={e => e.key === "Enter" && handleRefineTitle("")}
                      style={{ flex: 1, background: "#131318", color: "#fff", border: "1px solid rgba(255,255,255,0.15)", borderRadius: "8px", padding: "0.5rem 0.75rem", fontSize: "0.8rem" }} />
                    <button type="button" onClick={() => handleRefineTitle("")} disabled={titleRefining || !titleChatInput.trim()}
                      style={{ background: accentColor, border: "none", borderRadius: "8px", padding: "0.5rem 0.75rem", cursor: "pointer", display: "flex", alignItems: "center" }}>
                      <RefreshCw size={16} color="#fff" />
                    </button>
                  </div>
                </>
              )}
            </div>
          </motion.div>
        </div>
      )}

      {/* ============ DESCRIPTIONS TAB ============ */}
      {tab === "descriptions" && (
        <div style={{ display: "grid", gridTemplateColumns: "1fr 400px", gap: "1.75rem", alignItems: "start" }}>
          <motion.div initial={{ opacity: 0, y: 12 }} animate={{ opacity: 1, y: 0 }}>
            <div style={{ background: "#0c0c0f", border: "1px solid rgba(255,255,255,0.08)", borderRadius: "20px", padding: "1.5rem", marginBottom: "1.25rem" }}>
              <h3 style={{ fontSize: "0.95rem", fontWeight: 800, color: "#fff", marginBottom: "1.25rem", display: "flex", alignItems: "center", gap: "0.5rem" }}>
                <FileText size={16} color={accentColor} /> Generate Video Description
              </h3>
              <div style={{ marginBottom: "1rem" }}>
                <label style={{ display: "block", fontSize: "0.8rem", color: "#aaa", fontWeight: 700, marginBottom: "0.4rem" }}>YouTube Video URL or Transcript</label>
                <textarea className="input" rows={4} placeholder="Paste a YouTube URL to auto-extract the transcript, or paste your transcript text directly..." value={descVideoUrl} onChange={e => setDescVideoUrl(e.target.value)}
                  style={{ width: "100%", fontSize: "0.85rem", resize: "vertical" }} />
              </div>
              <div style={{ marginBottom: "1.25rem" }}>
                <label style={{ display: "block", fontSize: "0.78rem", color: "#aaa", fontWeight: 700, marginBottom: "0.4rem" }}>Upload Video (optional)</label>
                <label style={{ display: "block", padding: "0.55rem", textAlign: "center", background: "rgba(255,255,255,0.03)", border: "1px dashed rgba(255,255,255,0.18)", borderRadius: "10px", color: "#888", fontSize: "0.78rem", fontWeight: 700, cursor: "pointer" }}>
                  <Upload size={13} style={{ verticalAlign: "middle", marginRight: "0.3rem" }} /> {descVideoFile ? descVideoFile.name : "+ Upload video for real chapter timestamps"}
                  <input type="file" accept="video/*" style={{ display: "none" }} onChange={e => setDescVideoFile(e.target.files?.[0] || null)} />
                </label>
                {descVideoFile && fileChip(descVideoFile, "video selected")}
              </div>
              <button type="button" onClick={handleGenerateDescription} disabled={descLoading}
                style={{ width: "100%", background: accentColor, color: "#fff", fontWeight: 900, fontSize: "1rem", borderRadius: "14px", border: "none", padding: "0.85rem", boxShadow: "0 0 25px rgba(239,68,68,0.35)", cursor: "pointer", display: "flex", alignItems: "center", justifyContent: "center", gap: "0.5rem" }}>
                {descLoading ? <><div className="spinner" style={{ borderColor: "#000", borderTopColor: "transparent" }} /><span>Generating...</span></> : <><Sparkles size={18} /><span>Generate Description</span></>}
              </button>
            </div>

            {description && (
              <div style={{ background: "#0c0c0f", border: "1px solid rgba(255,255,255,0.08)", borderRadius: "20px", padding: "1.5rem" }}>
                <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: "0.75rem" }}>
                  <h3 style={{ fontSize: "0.9rem", fontWeight: 800, color: "#fff" }}>Generated Description</h3>
                  <button type="button" onClick={() => { navigator.clipboard.writeText(description); toast.success("Copied!"); }}
                    style={{ display: "flex", alignItems: "center", gap: "0.3rem", background: "rgba(255,255,255,0.06)", border: "1px solid rgba(255,255,255,0.1)", color: "#aaa", borderRadius: "6px", padding: "0.3rem 0.6rem", fontSize: "0.72rem", fontWeight: 700, cursor: "pointer" }}>
                    <Copy size={13} /> Copy
                  </button>
                </div>
                <pre style={{ whiteSpace: "pre-wrap", fontSize: "0.8rem", color: "#ccc", lineHeight: 1.5, fontFamily: "inherit", margin: 0 }}>{description}</pre>
              </div>
            )}
          </motion.div>

          {/* Chapters sidebar */}
          <motion.div initial={{ opacity: 0, x: 20 }} animate={{ opacity: 1, x: 0 }} style={{ position: "sticky", top: "84px" }}>
            <div style={{ background: "linear-gradient(180deg, #1a0a0a 0%, #0d0b12 100%)", border: "1px solid rgba(239,68,68,0.28)", borderRadius: "24px", padding: "1.5rem" }}>
              <span style={{ fontSize: "0.75rem", color: accentColor, fontWeight: 800, textTransform: "uppercase", display: "flex", alignItems: "center", gap: "0.4rem", marginBottom: "1rem" }}>
                <ThumbsUp size={14} /> Chapter Timestamps
              </span>
              {chapters.length === 0 ? (
                <p style={{ fontSize: "0.78rem", color: "#666", lineHeight: 1.5 }}>
                  Chapters with timestamps will appear here after generating a description from a YouTube video.
                </p>
              ) : (
                <div style={{ display: "flex", flexDirection: "column", gap: "0.4rem" }}>
                  {chapters.map((ch, i) => (
                    <div key={i} style={{ display: "flex", gap: "0.6rem", padding: "0.4rem 0.5rem", background: "rgba(255,255,255,0.03)", borderRadius: "6px", alignItems: "baseline" }}>
                      <span style={{ color: accentColor, fontSize: "0.72rem", fontWeight: 800, fontFamily: "monospace", minWidth: "48px" }}>{ch.time}</span>
                      <span style={{ fontSize: "0.78rem", color: "#ccc" }}>{ch.title}</span>
                    </div>
                  ))}
                </div>
              )}
            </div>
          </motion.div>
        </div>
      )}
    </div>
  );
}
