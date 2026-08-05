import { useState, useRef } from "react";
import { useNavigate } from "react-router-dom";
import { motion } from "framer-motion";
import {
  Wand2, Sparkles, Upload, Film, Check, X, Target, MessageSquare, Sliders, Video, Zap, Repeat, Monitor
} from "lucide-react";
import { toast } from "sonner";
import { api } from "../lib/api";
import { GEMINI_MODELS, OPENROUTER_VISION_MODEL_OPTIONS, OPENROUTER_VISION_MODELS, type LlmProvider } from "../lib/llmModels";

const TONES = [
  "authentic", "energetic", "calm", "professional", "funny", "inspirational", "edgy", "corporate",
];

export default function NovaEdit() {
  const navigate = useNavigate();

  const [files, setFiles] = useState<File[]>([]);
  const [uploading, setUploading] = useState(false);
  const [uploadProgress, setUploadProgress] = useState<Record<number, number>>({});
  const inputRef = useRef<HTMLInputElement>(null);

  const [product, setProduct] = useState("");
  const [audience, setAudience] = useState("");
  const [tone, setTone] = useState("authentic");
  const [contentType, setContentType] = useState("short_form");
  const [duration, setDuration] = useState("30");
  const [aspectRatio, setAspectRatio] = useState("9:16");
  const [instruction, setInstruction] = useState("");

  const [llmProvider, setLlmProvider] = useState<LlmProvider>("gemini");
  const [llmModel, setLlmModel] = useState("gemini-3.1-flash-lite");
  const [reviewThreshold, setReviewThreshold] = useState(0.6);
  const [maxRetries, setMaxRetries] = useState(2);

  const [loading, setLoading] = useState(false);
  const isVisionModel = llmProvider === "gemini" || OPENROUTER_VISION_MODELS.has(llmModel);

  const addFiles = (list: FileList | null) => {
    if (!list) return;
    const accepted = Array.from(list).filter((f) =>
      f.type.startsWith("video/") || /\.(mp4|mov|mkv|avi|m4v)$/i.test(f.name)
    );
    setFiles((prev) => [...prev, ...accepted]);
  };

  const removeFile = (idx: number) => {
    setFiles((prev) => prev.filter((_, i) => i !== idx));
  };

  const handleUploadAll = async (): Promise<string[]> => {
    const paths: string[] = [];
    setUploading(true);
    try {
      for (let i = 0; i < files.length; i++) {
        const res = await api.uploadVideo(files[i], (pct) =>
          setUploadProgress((prev) => ({ ...prev, [i]: pct }))
        );
        paths.push(res.video_path);
      }
    } finally {
      setUploading(false);
    }
    return paths;
  };

  const handleCreate = async () => {
    if (files.length === 0) {
      toast.error("Upload at least one footage clip first");
      return;
    }
    if (!product.trim() && !instruction.trim()) {
      toast.error("Describe your product/service or add a creative instruction");
      return;
    }
    setLoading(true);
    try {
      const footage = await handleUploadAll();
      if (footage.length === 0) {
        toast.error("No footage could be uploaded");
        return;
      }

      const title = product.trim() || instruction.trim().slice(0, 80) || "Agentic Edit";

      const payload = {
        brief: {
          product: product.trim(),
          audience: audience.trim(),
           tone,
           content_type: contentType,
           duration_seconds: Number(duration),
          instruction: instruction.trim(),
        },
        footage,
        api_keys: {
          gemini_key: localStorage.getItem("novaclip_gemini_key") || "",
          openrouter_key: localStorage.getItem("novaclip_openrouter_key") || "",
          deepgram_key: localStorage.getItem("novaclip_deepgram_key") || "",
        },
         llm_provider: llmModel,
         visual_analysis: isVisionModel,
        stage: "director",
        retries_used: 0,
        max_retries: maxRetries,
        review_threshold: reviewThreshold,
        feedback_history: [],
      };

      const task = await api.createTask({
        url: "novaedit://raw",
        source_title: title,
        aspect_ratio: aspectRatio,
        num_clips: 1,
        llm_provider: llmModel,
        novaedit_payload: payload,
      });

      sessionStorage.setItem("nova_last_task_type", "agentic");
      toast.success("NovaEdit task created. Director AI is planning your cut!");
      navigate(`/task/${task.task_id}`);
    } catch (e: any) {
      toast.error(e.message || "Failed to start NovaEdit task");
    } finally {
      setLoading(false);
    }
  };

  return (
    <div style={{ maxWidth: "1280px", margin: "0 auto", padding: "1.5rem 1rem" }}>
      {/* Header Banner */}
      <div style={{ textAlign: "center", marginBottom: "2.5rem" }}>
        <div style={{ display: "inline-flex", alignItems: "center", gap: "0.5rem", background: "rgba(34,211,238,0.1)", border: "1px solid rgba(34,211,238,0.3)", padding: "0.4rem 1rem", borderRadius: "20px", color: "#22d3ee", fontSize: "0.78rem", fontWeight: 800, textTransform: "uppercase", letterSpacing: "0.08em", marginBottom: "2.3rem" }}>
          <Wand2 size={14} /> NovaEdit: Agentic AI Video Editor
        </div>
        <h1 style={{ fontSize: "clamp(1.8rem, 3.8vw, 3.2rem)", fontWeight: 900, lineHeight: 1.1, marginBottom: "0.75rem", letterSpacing: "-0.03em", color: "#fff" }}>
          Raw Footage In, <span style={{ color: "#22d3ee", textShadow: "0 0 35px rgba(34,211,238,0.3)" }}>Polished Edit Out</span>
        </h1>
        <p style={{ fontSize: "1.05rem", color: "#a1a1aa", maxWidth: "700px", margin: "0 auto" }}>
          AI agents (<strong style={{ color: "#fff" }}>Director</strong>, <strong style={{ color: "#fff" }}>Editor</strong>, <strong style={{ color: "#fff" }}>Reviewer</strong>) turn raw clips and a brief into a finished video. Review the plan, approve, and watch them render, score, and improve.
        </p>
      </div>

      <div style={{ display: "grid", gridTemplateColumns: "1fr 380px", gap: "1.75rem", alignItems: "start" }}>
        {/* LEFT COLUMN: Controls */}
        <motion.div initial={{ opacity: 0, y: 15 }} animate={{ opacity: 1, y: 0 }} transition={{ duration: 0.4 }}>
          {/* Footage Upload */}
          <div style={{ background: "#0c0c0f", border: "1px solid rgba(255,255,255,0.08)", borderRadius: "20px", padding: "1.5rem", marginBottom: "1.5rem" }}>
            <label style={{ display: "flex", alignItems: "center", gap: "0.4rem", fontSize: "0.82rem", color: "#aaa", fontWeight: 700, marginBottom: "0.4rem" }}>
              <Video size={15} color="#22d3ee" /> Raw Footage Clips
            </label>
            <input
              ref={inputRef}
              type="file" accept="video/*,.mp4,.mov,.mkv,.avi,.m4v" multiple hidden
              onChange={(e) => { addFiles(e.target.files); e.target.value = ""; }}
            />
            <div onClick={() => inputRef.current?.click()} style={{ background: "#131318", border: "2px dashed rgba(34,211,238,0.3)", borderRadius: "12px", padding: "1.25rem", textAlign: "center", cursor: "pointer", marginBottom: "0.75rem" }}>
              <Upload size={22} style={{ color: "#22d3ee", marginBottom: "0.25rem" }} />
              <div style={{ fontSize: "0.85rem", fontWeight: 800, color: "#fff" }}>Click to upload footage</div>
              <div style={{ fontSize: "0.72rem", color: "#888", marginTop: "0.2rem" }}>A-Roll, B-Roll, or raw takes. MP4 / MOV / MKV</div>
            </div>

            {files.length > 0 && (
              <div style={{ display: "flex", flexDirection: "column", gap: "0.4rem" }}>
                {files.map((f, i) => (
                  <div key={i} style={{ display: "flex", alignItems: "center", gap: "0.5rem", background: "#131318", border: "1px solid rgba(255,255,255,0.08)", borderRadius: "8px", padding: "0.4rem 0.6rem" }}>
                    <Film size={14} color="#22d3ee" style={{ flexShrink: 0 }} />
                    <span style={{ fontSize: "0.75rem", fontWeight: 600, color: "#fff", overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap", flex: 1 }}>{f.name}</span>
                    {uploadProgress[i] != null && uploadProgress[i] < 100 && (
                      <span style={{ fontSize: "0.68rem", color: "#22d3ee", fontWeight: 700 }}>{Math.round(uploadProgress[i])}%</span>
                    )}
                    <button type="button" onClick={() => removeFile(i)} style={{ background: "rgba(239,68,68,0.15)", border: "1px solid rgba(239,68,68,0.3)", color: "#ef4444", borderRadius: "6px", padding: "0.15rem 0.5rem", fontSize: "0.68rem", fontWeight: 700, cursor: "pointer" }}>
                      <X size={12} />
                    </button>
                  </div>
                ))}
              </div>
            )}
          </div>

          {/* Creative Brief */}
          <div style={{ background: "#0c0c0f", border: "1px solid rgba(255,255,255,0.08)", borderRadius: "20px", padding: "1.5rem", marginBottom: "1.5rem" }}>
            <h3 style={{ fontSize: "0.95rem", fontWeight: 800, color: "#fff", marginBottom: "1.25rem", display: "flex", alignItems: "center", gap: "0.5rem" }}>
              <Target size={16} color="#22d3ee" /> Creative Brief
            </h3>
            <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: "1rem", marginBottom: "1rem" }}>
              <div>
                <label style={{ display: "block", fontSize: "0.78rem", color: "#aaa", fontWeight: 700, marginBottom: "0.4rem" }}>Product / Service</label>
                <input className="input" placeholder="e.g., NovaClip, an AI video app" value={product} onChange={e => setProduct(e.target.value)} style={{ width: "100%", fontSize: "0.85rem" }} />
              </div>
              <div>
                <label style={{ display: "block", fontSize: "0.78rem", color: "#aaa", fontWeight: 700, marginBottom: "0.4rem" }}>Target Audience</label>
                <input className="input" placeholder="e.g., Creators 18-35" value={audience} onChange={e => setAudience(e.target.value)} style={{ width: "100%", fontSize: "0.85rem" }} />
              </div>
            </div>
            <div style={{ display: "grid", gridTemplateColumns: "repeat(auto-fit, minmax(150px, 1fr))", gap: "1rem", marginBottom: "1rem" }}>
              <div>
                <label style={{ display: "block", fontSize: "0.78rem", color: "#aaa", fontWeight: 700, marginBottom: "0.4rem" }}>Content Type</label>
                <select value={contentType} onChange={e => { const type = e.target.value; setContentType(type); setDuration(type === "long_form" ? "600" : "30"); setAspectRatio(type === "long_form" ? "16:9" : "9:16"); }} style={{ width: "100%", background: "#131318", color: "#fff", border: "1px solid rgba(255,255,255,0.15)", borderRadius: "10px", padding: "0.55rem 0.75rem", fontSize: "0.82rem", fontWeight: 600 }}>
                  <option value="short_form">Short form</option>
                  <option value="long_form">Long form</option>
                </select>
              </div>
              <div>
                <label style={{ display: "block", fontSize: "0.78rem", color: "#aaa", fontWeight: 700, marginBottom: "0.4rem" }}>Tone</label>
                <select value={tone} onChange={e => setTone(e.target.value)} style={{ width: "100%", background: "#131318", color: "#fff", border: "1px solid rgba(255,255,255,0.15)", borderRadius: "10px", padding: "0.55rem 0.75rem", fontSize: "0.82rem", fontWeight: 600 }}>
                  {TONES.map(t => <option key={t} value={t}>{t}</option>)}
                </select>
              </div>
              <div>
                <label style={{ display: "block", fontSize: "0.78rem", color: "#aaa", fontWeight: 700, marginBottom: "0.4rem" }}>Target Duration</label>
                <select value={duration} onChange={e => setDuration(e.target.value)} style={{ width: "100%", background: "#131318", color: "#fff", border: "1px solid rgba(255,255,255,0.15)", borderRadius: "10px", padding: "0.55rem 0.75rem", fontSize: "0.82rem", fontWeight: 600 }}>
                  {contentType === "short_form" ? (
                    <>
                      <option value="20">20s Short</option>
                      <option value="30">30s Short</option>
                      <option value="45">45s</option>
                      <option value="60">60s Standard</option>
                      <option value="90">90s</option>
                    </>
                  ) : (
                    <>
                      <option value="120">2 minutes</option>
                      <option value="300">5 minutes</option>
                      <option value="600">10 minutes</option>
                      <option value="900">15 minutes</option>
                      <option value="1800">30 minutes</option>
                      <option value="3600">60 minutes</option>
                    </>
                  )}
                </select>
              </div>
              <div>
                <label style={{ display: "flex", alignItems: "center", gap: "0.4rem", fontSize: "0.78rem", color: "#aaa", fontWeight: 700, marginBottom: "0.4rem" }}>
                  <Monitor size={13} color="#22d3ee" /> Format
                </label>
                <select value={aspectRatio} onChange={e => setAspectRatio(e.target.value)} style={{ width: "100%", background: "#131318", color: "#fff", border: "1px solid rgba(255,255,255,0.15)", borderRadius: "10px", padding: "0.55rem 0.75rem", fontSize: "0.82rem", fontWeight: 600 }}>
                  <option value="9:16">9:16 Vertical (TikTok / Shorts / Reels)</option>
                  <option value="1:1">1:1 Square (Instagram Feed)</option>
                  <option value="16:9">16:9 Widescreen (YouTube)</option>
                  <option value="4:3">4:3 Classic</option>
                </select>
              </div>
            </div>
            <div>
              <label style={{ display: "flex", alignItems: "center", gap: "0.4rem", fontSize: "0.78rem", color: "#aaa", fontWeight: 700, marginBottom: "0.4rem" }}>
                <MessageSquare size={13} color="#22d3ee" /> Creative Instruction (optional)
              </label>
              <textarea
                className="input" rows={3}
                placeholder="e.g., Start with a bold hook, emphasize the transformation story, keep cuts tight and energetic, add a clear call to action..."
                value={instruction} onChange={e => setInstruction(e.target.value)}
                style={{ width: "100%", fontSize: "0.85rem", lineHeight: 1.4, resize: "vertical" }}
              />
            </div>
          </div>

          {/* Agent Settings */}
          <div style={{ background: "#0c0c0f", border: "1px solid rgba(255,255,255,0.08)", borderRadius: "20px", padding: "1.5rem" }}>
            <h3 style={{ fontSize: "0.95rem", fontWeight: 800, color: "#fff", marginBottom: "1.25rem", display: "flex", alignItems: "center", gap: "0.5rem" }}>
              <Sliders size={16} color="#22d3ee" /> Agent Pipeline Settings
            </h3>
            <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr 1fr", gap: "1rem", marginBottom: "1.25rem" }}>
              <div>
                <label style={{ display: "block", fontSize: "0.78rem", color: "#aaa", fontWeight: 700, marginBottom: "0.4rem" }}>AI Brain / LLM Engine</label>
                <select value={llmProvider} onChange={e => { const provider = e.target.value as LlmProvider; setLlmProvider(provider); setLlmModel(provider === "gemini" ? "gemini-3.1-flash-lite" : OPENROUTER_VISION_MODEL_OPTIONS[0].id); }} style={{ width: "100%", background: "#131318", color: "#fff", border: "1px solid rgba(255,255,255,0.15)", borderRadius: "10px", padding: "0.55rem 0.75rem", fontSize: "0.78rem", fontWeight: 600 }}>
                  <option value="gemini">Gemini</option>
                  <option value="openrouter">OpenRouter</option>
                </select>
                <select value={llmModel} onChange={e => setLlmModel(e.target.value)} style={{ width: "100%", marginTop: "0.4rem", background: "#131318", color: "#fff", border: "1px solid rgba(255,255,255,0.15)", borderRadius: "8px", padding: "0.4rem", fontSize: "0.76rem", fontWeight: 600 }}>
                  {(llmProvider === "gemini" ? GEMINI_MODELS : OPENROUTER_VISION_MODEL_OPTIONS).map(p => <option key={p.id} value={p.id}>{p.label}</option>)}
                </select>
              </div>
              <div>
                <label style={{ display: "block", fontSize: "0.78rem", color: "#aaa", fontWeight: 700, marginBottom: "0.4rem" }}>Review Threshold</label>
                <div style={{ display: "flex", alignItems: "center", gap: "0.5rem" }}>
                  <input type="range" min={0.4} max={0.9} step={0.05} value={reviewThreshold} onChange={e => setReviewThreshold(parseFloat(e.target.value))} style={{ flex: 1, accentColor: "#22d3ee" }} />
                  <span style={{ fontSize: "0.8rem", fontWeight: 800, color: "#22d3ee", minWidth: "2.6rem", textAlign: "right" }}>{reviewThreshold.toFixed(2)}</span>
                </div>
                <div style={{ fontSize: "0.68rem", color: "#888" }}>Editor auto-retries if Reviewer scores below this</div>
              </div>
              <div>
                <label style={{ display: "block", fontSize: "0.78rem", color: "#aaa", fontWeight: 700, marginBottom: "0.4rem" }}>Max Auto-Retries</label>
                <select value={maxRetries} onChange={e => setMaxRetries(Number(e.target.value))} style={{ width: "100%", background: "#131318", color: "#fff", border: "1px solid rgba(255,255,255,0.15)", borderRadius: "10px", padding: "0.55rem 0.75rem", fontSize: "0.8rem", fontWeight: 600 }}>
                  <option value={0}>0: no review loop</option>
                  <option value={1}>1</option>
                  <option value={2}>2 (Recommended)</option>
                  <option value={3}>3</option>
                </select>
              </div>
            </div>

            {/* Flow preview */}
            <div style={{ display: "flex", alignItems: "center", gap: "0.5rem", flexWrap: "wrap", marginBottom: "1.25rem", background: "#08080a", border: "1px solid rgba(255,255,255,0.06)", borderRadius: "10px", padding: "0.75rem 1rem" }}>
              {["Preprocess", "Director", "Approval", "Editor", "Reviewer"].map((s, i, arr) => (
                <div key={s} style={{ display: "flex", alignItems: "center", gap: "0.5rem" }}>
                  <span style={{ fontSize: "0.72rem", fontWeight: 800, color: "#22d3ee", background: "rgba(34,211,238,0.1)", border: "1px solid rgba(34,211,238,0.25)", padding: "0.2rem 0.6rem", borderRadius: "999px" }}>{s}</span>
                  {i < arr.length - 1 && <Zap size={12} color="#555" />}
                </div>
              ))}
            </div>

            <button
              type="button"
              onClick={handleCreate}
              disabled={loading}
              style={{
                width: "100%", background: "linear-gradient(90deg, #06b6d4, #22d3ee)", color: "#000", fontWeight: 900,
                fontSize: "1.05rem", borderRadius: "14px", border: "none", padding: "0.9rem",
                boxShadow: "0 0 25px rgba(34,211,238,0.3)", cursor: "pointer", display: "flex",
                alignItems: "center", justifyContent: "center", gap: "0.5rem"
              }}
            >
              {loading ? (
                <><div className="spinner" style={{ borderColor: "#000", borderTopColor: "transparent" }} /><span>Uploading &amp; starting agents...</span></>
              ) : (
                <><Wand2 size={20} /><span>Start Agentic Edit</span></>
              )}
            </button>
          </div>
        </motion.div>

        {/* RIGHT COLUMN: How it works */}
        <motion.div initial={{ opacity: 0, x: 20 }} animate={{ opacity: 1, x: 0 }} transition={{ duration: 0.4, delay: 0.1 }} style={{ position: "sticky", top: "84px" }}>
          <div style={{ background: "#131318", border: "1px solid rgba(34,211,238,0.15)", borderRadius: "24px", padding: "1.5rem" }}>
            <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: "1rem" }}>
              <span style={{ fontSize: "0.75rem", color: "#22d3ee", fontWeight: 800, textTransform: "uppercase", display: "flex", alignItems: "center", gap: "0.4rem" }}>
                <Repeat size={14} /> How the agents work
              </span>
            </div>
            <div style={{ display: "flex", flexDirection: "column", gap: "0.75rem" }}>
              <div style={{ background: "#0c0c0f", borderRadius: "12px", padding: "0.9rem", border: "1px solid rgba(255,255,255,0.07)" }}>
                <div style={{ fontWeight: 800, fontSize: "0.85rem", color: "#fff", marginBottom: "0.25rem" }}>1 · Preprocess</div>
                <div style={{ fontSize: "0.75rem", color: "#aaa", lineHeight: 1.45 }}>Scene detection + word-level transcription of every clip (Deepgram / Vosk / Whisper), packed into a shot-level transcript index.</div>
              </div>
              <div style={{ background: "#0c0c0f", borderRadius: "12px", padding: "0.9rem", border: "1px solid rgba(255,255,255,0.07)" }}>
                <div style={{ fontWeight: 800, fontSize: "0.85rem", color: "#fff", marginBottom: "0.25rem" }}>2 · Director agent</div>
                <div style={{ fontSize: "0.75rem", color: "#aaa", lineHeight: 1.45 }}>Reasons over the transcript + your brief, picks shots and trim points, and proposes an EditPlan with a narrative arc.</div>
              </div>
              <div style={{ background: "#0c0c0f", borderRadius: "12px", padding: "0.9rem", border: "1px solid rgba(255,255,255,0.07)" }}>
                <div style={{ fontWeight: 800, fontSize: "0.85rem", color: "#fff", marginBottom: "0.25rem" }}>3 · Your approval</div>
                <div style={{ fontSize: "0.75rem", color: "#aaa", lineHeight: 1.45 }}>Review the EditPlan on the task page. Approve it as-is or edit trim points first. You stay in control.</div>
              </div>
              <div style={{ background: "#0c0c0f", borderRadius: "12px", padding: "0.9rem", border: "1px solid rgba(255,255,255,0.07)" }}>
                <div style={{ fontWeight: 800, fontSize: "0.85rem", color: "#fff", marginBottom: "0.25rem" }}>4 · Editor agent</div>
                <div style={{ fontSize: "0.75rem", color: "#aaa", lineHeight: 1.45 }}>Deterministic FFmpeg render: word-boundary cuts, 30ms audio fades, vertical normalization, lossless concat.</div>
              </div>
              <div style={{ background: "#0c0c0f", borderRadius: "12px", padding: "0.9rem", border: "1px solid rgba(255,255,255,0.07)" }}>
                <div style={{ fontWeight: 800, fontSize: "0.85rem", color: "#fff", marginBottom: "0.25rem" }}>5 · Reviewer agent</div>
                <div style={{ fontSize: "0.75rem", color: "#aaa", lineHeight: 1.45 }}>Scores adherence, pacing, visual quality, and watchability. If below threshold, feedback loops back to the Director and it retries up to your maximum.</div>
              </div>
            </div>
            <div style={{ marginTop: "1rem", padding: "0.75rem", borderRadius: "10px", background: "rgba(34,211,238,0.08)", border: "1px solid rgba(34,211,238,0.2)", fontSize: "0.72rem", color: "#7dd3fc", lineHeight: 1.5 }}>
              <Check size={13} style={{ verticalAlign: "-2px", marginRight: "0.3rem" }} />
              Needs Gemini or OpenRouter plus Deepgram keys (or local Vosk). Set them in <strong>Settings</strong>.
            </div>
          </div>
        </motion.div>
      </div>

      {/* Nova Edit Footer */}
      <section style={{ borderTop: "1px solid rgba(34,211,238,0.18)", background: "#0b0b0e", padding: "2.2rem 0", marginTop: "2.5rem" }}>
        <div style={{ maxWidth: "1280px", margin: "0 auto", padding: "0 1rem" }}>
          <div style={{ display: "grid", gridTemplateColumns: "repeat(auto-fit, minmax(210px, 1fr))", gap: "1.5rem", textAlign: "center" }}>
            <div>
              <h4 style={{ fontSize: "1.35rem", fontWeight: 900, color: "#22d3ee", margin: 0 }}>DIRECTOR</h4>
              <p style={{ fontSize: "0.78rem", color: "#888", margin: "0.35rem 0 0" }}>Plans the story from your footage</p>
            </div>
            <div>
              <h4 style={{ fontSize: "1.35rem", fontWeight: 900, color: "#67e8f9", margin: 0 }}>APPROVE</h4>
              <p style={{ fontSize: "0.78rem", color: "#888", margin: "0.35rem 0 0" }}>You stay in control of the EditPlan</p>
            </div>
            <div>
              <h4 style={{ fontSize: "1.35rem", fontWeight: 900, color: "#a5f3fc", margin: 0 }}>EDITOR</h4>
              <p style={{ fontSize: "0.78rem", color: "#888", margin: "0.35rem 0 0" }}>Renders precise transcript-aware cuts</p>
            </div>
            <div>
              <h4 style={{ fontSize: "1.35rem", fontWeight: 900, color: "#cffafe", margin: 0 }}>REVIEWER</h4>
              <p style={{ fontSize: "0.78rem", color: "#888", margin: "0.35rem 0 0" }}>Scores and improves every version</p>
            </div>
          </div>
        </div>
      </section>
    </div>
  );
}
