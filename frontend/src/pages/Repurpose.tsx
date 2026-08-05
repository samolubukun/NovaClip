import { useEffect, useRef, useState } from "react";
import { useNavigate } from "react-router-dom";
import { motion } from "framer-motion";
import { Check, ChevronDown, ChevronUp, FileText, Link2, Megaphone, Sparkles, Upload, Video, Wand2 } from "lucide-react";
import { toast } from "sonner";
import { api } from "../lib/api";
import { GEMINI_MODELS, OPENROUTER_MODELS, type LlmProvider } from "../lib/llmModels";

const PLATFORMS = [
  { id: "tiktok", label: "TikTok", ratio: "9:16", duration: 30, video: true },
  { id: "instagram", label: "Instagram", ratio: "9:16", duration: 60, video: true },
  { id: "youtube", label: "YouTube", ratio: "16:9", duration: 60, video: true },
  { id: "linkedin", label: "LinkedIn", ratio: "1:1", duration: 60, video: true },
  { id: "x", label: "X", ratio: "16:9", duration: 45, video: true },
  { id: "newsletter", label: "Newsletter", ratio: "16:9", duration: 0, video: false },
  { id: "blog", label: "Blog", ratio: "16:9", duration: 0, video: false },
];

type PlatformState = Record<string, { selected: boolean; video: boolean; written: boolean; aspect_ratio: string; duration_seconds?: number }>;

export default function Repurpose() {
  const navigate = useNavigate();
  const fileRef = useRef<HTMLInputElement>(null);
  const [sourceMode, setSourceMode] = useState<"task" | "upload" | "youtube">("upload");
  const [tasks, setTasks] = useState<any[]>([]);
  const [sourceTaskId, setSourceTaskId] = useState("");
  const [file, setFile] = useState<File | null>(null);
  const [url, setUrl] = useState("");
  const [campaignName, setCampaignName] = useState("");
  const [audience, setAudience] = useState("");
  const [goal, setGoal] = useState("awareness");
  const [tone, setTone] = useState("engaging");
  const [coreMessage, setCoreMessage] = useState("");
  const [cta, setCta] = useState("");
  const [instructions, setInstructions] = useState("");
  const [showAdvanced, setShowAdvanced] = useState(false);
  const [provider, setProvider] = useState<LlmProvider>("gemini");
  const [model, setModel] = useState("gemini-3.1-flash-lite");
  const [customModel, setCustomModel] = useState("");
  const [loading, setLoading] = useState(false);
  const [platforms, setPlatforms] = useState<PlatformState>(() => Object.fromEntries(PLATFORMS.map(p => [p.id, { selected: p.id === "tiktok" || p.id === "instagram", video: p.video, written: true, aspect_ratio: p.ratio, duration_seconds: p.duration || undefined }])));

  useEffect(() => {
    api.listTasks().then(r => {
      const completed = (r.tasks || []).filter((t: any) => t.status === "completed" && t.source_type !== "repurpose");
      setTasks(completed);
      setSourceTaskId(completed[0]?.id || "");
    }).catch(() => {});
  }, []);

  const updatePlatform = (id: string, patch: Partial<PlatformState[string]>) => setPlatforms(current => ({ ...current, [id]: { ...current[id], ...patch } }));
  const selectedCount = Object.values(platforms).filter(platform => platform.selected).length;

  const submit = async () => {
    const selected = PLATFORMS.filter(p => platforms[p.id].selected).map(p => ({ id: p.id, ...platforms[p.id] }));
    if (!campaignName.trim()) return toast.error("Enter a campaign name");
    if (!selected.length) return toast.error("Select at least one platform");
    if (sourceMode === "task" && !sourceTaskId) return toast.error("Select a completed task");
    if (sourceMode === "upload" && !file) return toast.error("Choose a video file");
    if (sourceMode === "youtube" && !url.trim()) return toast.error("Enter a YouTube URL");
    setLoading(true);
    try {
      let sourceUrl = sourceMode === "youtube" ? url.trim() : "repurpose://task";
      if (sourceMode === "upload") sourceUrl = (await api.uploadVideo(file!)).video_path;
      const selectedModel = provider === "gemini" ? model : model === "custom" ? customModel : model;
      const task = await api.createTask({
        url: "repurpose://campaign",
        source_title: campaignName.trim(),
        source_type: "repurpose",
        aspect_ratio: selected.find(p => p.video)?.aspect_ratio || "16:9",
        num_clips: selected.filter(p => p.video).length || 1,
        llm_provider: selectedModel,
        repurpose_payload: {
          campaign_name: campaignName.trim(), audience, goal, tone, core_message: coreMessage, cta, instructions,
          source_task_id: sourceMode === "task" ? sourceTaskId : null,
          source_url: sourceMode === "task" ? null : sourceUrl,
          platforms: selected,
        },
      });
      sessionStorage.setItem("nova_last_task_type", "repurpose");
      navigate(`/task/${task.task_id}`);
    } catch (error: any) {
      toast.error(error.message || "Failed to create repurpose campaign");
    } finally { setLoading(false); }
  };

  return (
    <div style={{ minHeight: "100vh", display: "flex", flexDirection: "column", justifyContent: "space-between" }}>
      <div style={{ maxWidth: "1280px", width: "100%", margin: "0 auto", padding: "1.5rem 1rem 3rem", flex: 1, boxSizing: "border-box" }}>
        {/* Header Banner */}
        <div style={{ textAlign: "center", marginBottom: "2.5rem" }}>
          <div style={{ display: "inline-flex", alignItems: "center", gap: "0.5rem", color: "#fb7185", background: "rgba(244,63,94,.1)", border: "1px solid rgba(244,63,94,.3)", borderRadius: "20px", padding: "0.4rem 1rem", fontSize: "0.78rem", fontWeight: 800, textTransform: "uppercase", letterSpacing: "0.08em", marginBottom: "2.3rem" }}>
            <Megaphone size={14} /> Nova Repurpose
          </div>
          <h1 style={{ fontSize: "clamp(1.8rem, 3.8vw, 3.2rem)", fontWeight: 900, lineHeight: 1.1, margin: "0 0 0.75rem", letterSpacing: "-0.03em", color: "#fff" }}>
            One video. <span style={{ color: "#f43f5e", textShadow: "0 0 35px rgba(244,63,94,0.3)" }}>Every platform.</span>
          </h1>
          <p style={{ fontSize: "1.05rem", color: "#a1a1aa", maxWidth: "700px", margin: "0 auto" }}>
            Generate platform-formatted videos and substantial, editable campaign copy for every selected channel, then export the complete written package as a designed PDF.
          </p>
        </div>

        {/* 2-Column Main Layout */}
        <div style={{ display: "grid", gridTemplateColumns: "1fr 380px", gap: "1.75rem", alignItems: "start" }}>
          {/* LEFT COLUMN: Controls */}
          <motion.div initial={{ opacity: 0, y: 12 }} animate={{ opacity: 1, y: 0 }} style={{ display: "grid", gap: "1.25rem" }}>
            {/* 1. Choose source content */}
            <section style={cardStyle}>
              <h3 style={headingStyle}><Video size={17} color="#fb7185" /> Choose source content</h3>
              <div style={{ display: "flex", gap: ".5rem", flexWrap: "wrap", marginBottom: "1rem" }}>{(["upload","youtube","task"] as const).map(mode => <button key={mode} onClick={() => setSourceMode(mode)} style={pill(sourceMode === mode)}>{mode === "task" ? "Completed Task" : mode === "upload" ? "Upload Video" : "YouTube URL"}</button>)}</div>
              {sourceMode === "task" && <select value={sourceTaskId} onChange={e => setSourceTaskId(e.target.value)} style={inputStyle}>{tasks.map(t => <option key={t.id} value={t.id}>{t.source_title || t.source_url}</option>)}</select>}
              {sourceMode === "upload" && <><input ref={fileRef} type="file" accept="video/*" hidden onChange={e => setFile(e.target.files?.[0] || null)} /><button onClick={() => fileRef.current?.click()} style={{ ...inputStyle, cursor: "pointer" }}><Upload size={15} /> {file?.name || "Choose video file"}</button></>}
              {sourceMode === "youtube" && <div style={{ position: "relative" }}><Link2 size={15} style={{ position: "absolute", left: 12, top: 13, color: "#777" }} /><input value={url} onChange={e => setUrl(e.target.value)} placeholder="https://youtube.com/watch?v=..." style={{ ...inputStyle, paddingLeft: 36 }} /></div>}
            </section>

            {/* 2. Campaign brief */}
            <section style={cardStyle}>
              <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: "1rem" }}>
                <h3 style={{ ...headingStyle, margin: 0 }}><FileText size={17} color="#fb7185" /> Campaign brief</h3>
                <span style={{ color: "#71717a", fontSize: ".7rem", fontWeight: 700 }}>Core message, CTA & notes optional</span>
              </div>
              <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: "1rem" }}>
                <input value={campaignName} onChange={e => setCampaignName(e.target.value)} placeholder="Campaign name *" style={inputStyle} />
                <input value={audience} onChange={e => setAudience(e.target.value)} placeholder="Target audience" style={inputStyle} />
                <select value={goal} onChange={e => setGoal(e.target.value)} style={inputStyle}><option value="awareness">Brand awareness</option><option value="engagement">Engagement</option><option value="sales">Product sales</option><option value="leads">Lead generation</option><option value="education">Education</option></select>
                <select value={tone} onChange={e => setTone(e.target.value)} style={inputStyle}><option>engaging</option><option>professional</option><option>energetic</option><option>educational</option><option>conversational</option></select>
              </div>
              <button onClick={() => setShowAdvanced(!showAdvanced)} style={{ display: "flex", alignItems: "center", gap: ".35rem", marginTop: "1rem", background: "none", border: 0, color: "#fb7185", fontSize: ".72rem", fontWeight: 800, cursor: "pointer", padding: 0 }}>
                {showAdvanced ? <ChevronUp size={14} /> : <ChevronDown size={14} />} Advanced options
              </button>
              {showAdvanced && (
                <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: "1rem", marginTop: ".75rem" }}>
                  <input value={coreMessage} onChange={e => setCoreMessage(e.target.value)} placeholder="Core message (AI will derive it from the video if empty)" style={inputStyle} />
                  <input value={cta} onChange={e => setCta(e.target.value)} placeholder="Primary CTA" style={inputStyle} />
                  <textarea value={instructions} onChange={e => setInstructions(e.target.value)} placeholder="Additional campaign instructions" rows={3} style={{ ...inputStyle, gridColumn: "1 / -1", resize: "vertical" }} />
                </div>
              )}
            </section>

            {/* 3. Platform package */}
            <section style={cardStyle}>
              <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: "1rem" }}>
                <h3 style={{ ...headingStyle, margin: 0 }}><Megaphone size={17} color="#fb7185" /> Platform package</h3>
                <span style={{ color: "#fb7185", fontSize: ".7rem", fontWeight: 900 }}>{selectedCount} of {PLATFORMS.length}</span>
              </div>
              <div style={{ display: "grid", gridTemplateColumns: "repeat(auto-fit, minmax(200px, 1fr))", gap: ".6rem" }}>
                {PLATFORMS.map(platform => {
                  const state = platforms[platform.id];
                  const toggle = () => updatePlatform(platform.id, { selected: !state.selected });
                  return (
                    <div
                      key={platform.id}
                      role="button"
                      tabIndex={0}
                      aria-pressed={state.selected}
                      onClick={toggle}
                      onKeyDown={event => { if (event.key === "Enter" || event.key === " ") { event.preventDefault(); toggle(); } }}
                      style={{ padding: ".8rem", background: state.selected ? "rgba(244,63,94,.1)" : "#0b0b0e", border: `1px solid ${state.selected ? "rgba(244,63,94,.45)" : "rgba(255,255,255,.09)"}`, borderRadius: 10, cursor: "pointer", boxShadow: state.selected ? "0 0 14px rgba(244,63,94,.08)" : "none", transition: "all .15s" }}
                    >
                      <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", gap: ".75rem" }}>
                        <div style={{ display: "flex", alignItems: "center", gap: ".6rem", color: "#fff", fontWeight: 800 }}>
                          <span style={{ width: 22, height: 22, borderRadius: 6, border: `1px solid ${state.selected ? "#f43f5e" : "#555"}`, background: state.selected ? "#f43f5e" : "transparent", display: "grid", placeItems: "center", flexShrink: 0 }}>
                            {state.selected && <Check size={14} color="#fff" strokeWidth={3} />}
                          </span>
                          {platform.label}
                        </div>
                        {state.selected && (
                          <div style={{ display: "flex", gap: ".65rem", fontSize: ".72rem", color: "#ddd" }} onClick={event => event.stopPropagation()}>
                            {platform.video && <label style={{ cursor: "pointer" }}><input type="checkbox" checked={state.video} onChange={e => updatePlatform(platform.id, { video: e.target.checked })} /> Video</label>}
                            <label style={{ cursor: "pointer" }}><input type="checkbox" checked={state.written} onChange={e => updatePlatform(platform.id, { written: e.target.checked })} /> Copy</label>
                          </div>
                        )}
                      </div>
                      {state.selected && state.video && (
                        <div style={{ display: "flex", gap: ".5rem", marginTop: ".65rem" }} onClick={event => event.stopPropagation()}>
                          <select value={state.aspect_ratio} onChange={e => updatePlatform(platform.id, { aspect_ratio: e.target.value })} style={miniInput}><option>9:16</option><option>1:1</option><option>16:9</option></select>
                          <select value={state.duration_seconds} onChange={e => updatePlatform(platform.id, { duration_seconds: Number(e.target.value) })} style={miniInput}><option value={15}>15s</option><option value={30}>30s</option><option value={45}>45s</option><option value={60}>60s</option><option value={90}>90s</option></select>
                        </div>
                      )}
                    </div>
                  );
                })}
              </div>
            </section>
          </motion.div>

          {/* RIGHT COLUMN: AI Brain & Sidebar */}
          <motion.div initial={{ opacity: 0, y: 15 }} animate={{ opacity: 1, y: 0 }} transition={{ duration: 0.4, delay: 0.1 }} style={{ display: "grid", gap: "1.25rem", position: "sticky", top: "2rem" }}>
            {/* Sidebar "How Repurpose Works" */}
            <div style={{ background: "#0c0c0f", border: "1px solid rgba(255,255,255,0.08)", borderRadius: "20px", padding: "1.25rem" }}>
              <div style={{ fontSize: "0.82rem", fontWeight: 800, color: "#fff", marginBottom: "0.85rem", textTransform: "uppercase", letterSpacing: "0.05em", display: "flex", alignItems: "center", gap: "0.4rem" }}>
                <Sparkles size={14} color="#fb7185" /> How Repurpose Works
              </div>
              <div style={{ display: "grid", gap: "0.75rem" }}>
                {[
                  { number: "01", title: "Source Ingestion", text: "Transcribes audio and extracts core topics from your video or past task." },
                  { number: "02", title: "Multi-Format Edit", text: "Cuts platform-native 9:16, 1:1, or 16:9 video clips tailored to each channel." },
                  { number: "03", title: "Copy & Captions", text: "Writes platform posts, hashtags, email copy, and blog summaries." },
                  { number: "04", title: "PDF Export", text: "Bundles all formatted campaign copy into a downloadable PDF package." },
                ].map((step) => (
                  <div key={step.number} style={{ display: "flex", gap: "0.75rem", alignItems: "start" }}>
                    <div style={{ background: "rgba(244,63,94,0.12)", border: "1px solid rgba(244,63,94,0.3)", borderRadius: "8px", padding: "0.2rem 0.45rem", fontSize: "0.68rem", fontWeight: 900, color: "#fb7185", flexShrink: 0 }}>
                      {step.number}
                    </div>
                    <div>
                      <div style={{ fontSize: "0.78rem", fontWeight: 700, color: "#fff", lineHeight: 1.2 }}>{step.title}</div>
                      <div style={{ fontSize: "0.72rem", color: "#888", marginTop: "0.15rem", lineHeight: 1.35 }}>{step.text}</div>
                    </div>
                  </div>
                ))}
              </div>
            </div>

            {/* AI Engine & Generate Button */}
            <section style={cardStyle}>
              <h3 style={headingStyle}><Wand2 size={17} color="#fb7185" /> AI Engine & Action</h3>
              <div style={{ display: "grid", gap: "0.85rem", marginBottom: "1rem" }}>
                <div>
                  <div style={labelStyle}>AI provider</div>
                  <select value={provider} onChange={e => { const p=e.target.value as LlmProvider; setProvider(p); setModel(p==="gemini"?"gemini-3.1-flash-lite":"openrouter/free"); }} style={inputStyle}>
                    <option value="gemini">Gemini</option>
                    <option value="openrouter">OpenRouter</option>
                  </select>
                </div>
                <div>
                  <div style={labelStyle}>Model</div>
                  <select value={model} onChange={e => setModel(e.target.value)} style={inputStyle}>
                    {(provider==="gemini"?GEMINI_MODELS:OPENROUTER_MODELS).map(m=><option key={m.id} value={m.id}>{m.label}</option>)}
                  </select>
                  {model === "custom" && <input value={customModel} onChange={e=>setCustomModel(e.target.value)} placeholder="provider/model-id" style={{...inputStyle,marginTop:".6rem"}} />}
                </div>
              </div>

              <button disabled={loading} onClick={submit} style={{ width: "100%", display: "flex", alignItems: "center", justifyContent: "center", gap: ".5rem", padding: ".9rem 1.5rem", border: 0, borderRadius: 12, background: "linear-gradient(90deg,#f43f5e,#fb7185)", color: "#fff", fontWeight: 900, fontSize: ".95rem", cursor: "pointer", boxShadow: "0 0 24px rgba(244,63,94,.25)", transition: "opacity .15s", opacity: loading ? .7 : 1 }}>
                {loading ? <><div className="spinner" style={{ borderColor: "#fff", borderTopColor: "transparent" }} /><span>Creating campaign...</span></> : <><Megaphone size={20} /><span>Generate Repurpose Campaign</span></>}
              </button>
            </section>
          </motion.div>
        </div>
      </div>

      {/* Full-Bleed Nova Repurpose Footer */}
      <section style={{ borderTop: "1px solid rgba(244,63,94,0.18)", background: "#0b0b0e", padding: "2.2rem 0", width: "100%", marginTop: "2.5rem" }}>
        <div style={{ maxWidth: "1280px", margin: "0 auto", padding: "0 1rem" }}>
          <div style={{ display: "grid", gridTemplateColumns: "repeat(auto-fit, minmax(210px, 1fr))", gap: "1.5rem", textAlign: "center" }}>
            <div>
              <h4 style={{ fontSize: "1.35rem", fontWeight: 900, color: "#f43f5e", margin: 0 }}>ONE SOURCE</h4>
              <p style={{ fontSize: "0.78rem", color: "#888", margin: "0.35rem 0 0" }}>Upload, past job, YouTube, or link</p>
            </div>
            <div>
              <h4 style={{ fontSize: "1.35rem", fontWeight: 900, color: "#fb7185", margin: 0 }}>MULTI PLATFORM</h4>
              <p style={{ fontSize: "0.78rem", color: "#888", margin: "0.35rem 0 0" }}>YouTube, TikTok, X, LinkedIn & IG</p>
            </div>
            <div>
              <h4 style={{ fontSize: "1.35rem", fontWeight: 900, color: "#fda4af", margin: 0 }}>VIDEO + COPY</h4>
              <p style={{ fontSize: "0.78rem", color: "#888", margin: "0.35rem 0 0" }}>Aspect cuts plus platform posts</p>
            </div>
            <div>
              <h4 style={{ fontSize: "1.35rem", fontWeight: 900, color: "#fecdd3", margin: 0 }}>PDF CAMPAIGN</h4>
              <p style={{ fontSize: "0.78rem", color: "#888", margin: "0.35rem 0 0" }}>Export full marketing packet</p>
            </div>
          </div>
        </div>
      </section>
    </div>
  );
}

const cardStyle: React.CSSProperties = { background: "#101014", border: "1px solid rgba(255,255,255,.09)", borderRadius: 18, padding: "1.35rem" };
const headingStyle: React.CSSProperties = { display: "flex", alignItems: "center", gap: ".5rem", color: "#fff", fontSize: ".95rem", margin: "0 0 1rem" };
const labelStyle: React.CSSProperties = { color: "#888", fontSize: ".68rem", fontWeight: 800, textTransform: "uppercase", letterSpacing: ".05em", marginBottom: ".4rem" };
const inputStyle: React.CSSProperties = { width: "100%", boxSizing: "border-box", display: "flex", alignItems: "center", gap: ".5rem", background: "#17171d", color: "#fff", border: "1px solid rgba(255,255,255,.13)", borderRadius: 9, padding: ".65rem .75rem", fontFamily: "inherit" };
const miniInput: React.CSSProperties = { flex: 1, background: "#17171d", color: "#fff", border: "1px solid rgba(255,255,255,.12)", borderRadius: 6, padding: ".3rem", fontSize: ".68rem" };
const pill = (active:boolean): React.CSSProperties => ({ border: `1px solid ${active?"#f43f5e":"rgba(255,255,255,.1)"}`, background: active?"rgba(244,63,94,.14)":"#15151a", color: active?"#fb7185":"#aaa", borderRadius: 8, padding: ".45rem .75rem", fontWeight: 800, cursor: "pointer" });
