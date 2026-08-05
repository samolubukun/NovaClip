import { useState, useEffect, useRef } from "react";
import { useParams, Link } from "react-router-dom";
import { ArrowLeft, RefreshCw, XCircle, Play, Pause, Download, ExternalLink, Trash2, Copy, Zap, Flame, Radio, Mic, Brain, Crop, Sparkles, CheckCircle2, MessageCircle, Send, X, Film, Wand2, ListChecks, ThumbsUp, AlertTriangle } from "lucide-react";
import { toast } from "sonner";
import { motion, AnimatePresence } from "framer-motion";
import { api } from "@/lib/api";
import { createTaskSSE } from "@/lib/sse";

interface Clip {
  id: string;
  task_id: string;
  clip_order: number;
  filename: string;
  start_time: string;
  end_time: string;
  duration: number;
  transcript_text: string;
  virality_score: number;
  hook_score: number;
  engagement_score: number;
  value_score: number;
  shareability_score: number;
  hook_type: string;
  hook_title: string;
  reasoning: string;
}

interface Task {
  id: string;
  status: string;
  progress: number;
  progress_message: string;
  source_url: string;
  source_title: string;
  source_type: string;
  aspect_ratio: string;
  num_clips: number;
  clips: Clip[];
  error_message?: string;
  studio_payload?: any;
  novaedit_payload?: any;
  edit_plan?: {
    rationale?: string;
    total_duration?: number;
    entries?: {
      shot_id: string;
      start_trim: number;
      end_trim: number;
      position: number;
      text_overlay?: string | null;
    }[];
  };
  review_score?: {
    adherence?: number;
    pacing?: number;
    visual_quality?: number;
    watchability?: number;
    overall?: number;
    feedback?: string;
  };
  repurpose_result?: {
    campaign_name?: string;
    source_title?: string;
    audience?: string;
    goal?: string;
    tone?: string;
    core_message?: string;
    cta?: string;
    platform_copy?: Record<string, Record<string, unknown>>;
    videos?: { platform: string; filename: string; aspect_ratio: string; duration: number }[];
  };
}

interface ReviewBarProps { label: string; value?: number }
function ReviewBar({ label, value = 0 }: ReviewBarProps) {
  const pct = Math.round(Math.min(1, Math.max(0, value)) * 100);
  return (
    <div>
      <div style={{ display: "flex", justifyContent: "space-between", marginBottom: "4px" }}>
        <span style={{ fontSize: "0.72rem", color: "#888" }}>{label}</span>
        <span style={{ fontSize: "0.72rem", fontWeight: 700, color: "#22d3ee" }}>{value.toFixed(2)}</span>
      </div>
      <div style={{ background: "rgba(255,255,255,0.08)", height: "4px", borderRadius: "2px", overflow: "hidden" }}>
        <div style={{ width: `${pct}%`, background: pct >= 70 ? "#22c55e" : pct >= 55 ? "#22d3ee" : "#ef4444", height: "100%" }} />
      </div>
    </div>
  );
}

function ScoreBar({ label, score }: { label: string; score: number }) {
  return (
    <div>
      <div style={{ display: "flex", justifyContent: "space-between", marginBottom: "4px" }}>
        <span style={{ fontSize: "0.72rem", color: "#888" }}>{label}</span>
        <span style={{ fontSize: "0.72rem", fontWeight: 700, color: "var(--accent)" }}>{score}</span>
      </div>
      <div className="score-bar-track" style={{ background: "rgba(255,255,255,0.08)", height: "4px", borderRadius: "2px", overflow: "hidden" }}>
        <div className="score-bar-fill" style={{ width: `${(score / 25) * 100}%`, background: "var(--accent)", height: "100%" }} />
      </div>
    </div>
  );
}

function HookTypeBadge({ hookType }: { hookType: string }) {
  const colors: Record<string, string> = {
    question: "#3b82f6",
    statement: "#8b5cf6",
    statistic: "#06b6d4",
    story: "#f59e0b",
    contrast: "#ef4444",
    none: "#6b7280",
  };
  return (
    <span style={{
      fontSize: "0.68rem", fontWeight: 700, padding: "0.2rem 0.6rem",
      borderRadius: "9999px", textTransform: "uppercase", letterSpacing: "0.06em",
      background: `${colors[hookType] || "#6b7280"}20`,
      color: colors[hookType] || "#6b7280",
      border: `1px solid ${colors[hookType] || "#6b7280"}40`,
    }}>
      {hookType}
    </span>
  );
}

function ClipCard({ clip, taskId, aspectRatio, isSelected, onSelect }: { clip: Clip; taskId: string; aspectRatio?: string; isSelected: boolean; onSelect: () => void }) {
  const [playing, setPlaying] = useState(false);
  const [expanded, setExpanded] = useState(false);
  const videoRef = useRef<HTMLVideoElement>(null);
  const fileUrl = api.clipFileUrl(taskId, clip.id);

  const togglePlay = (e: React.MouseEvent) => {
    e.stopPropagation();
    if (!videoRef.current) return;
    if (playing) { videoRef.current.pause(); setPlaying(false); }
    else { videoRef.current.play(); setPlaying(true); }
  };

  const deleteClip = async (e: React.MouseEvent) => {
    e.stopPropagation();
    if (!confirm("Delete this clip?")) return;
    await api.deleteClip(taskId, clip.id);
    toast.success("Clip deleted");
    window.location.reload();
  };

  const copyUrl = (e: React.MouseEvent) => {
    e.stopPropagation();
    navigator.clipboard.writeText(window.location.origin + fileUrl);
    toast.success("URL copied!");
  };

  const viralityColor = clip.virality_score >= 80 ? "#22c55e" : clip.virality_score >= 60 ? "var(--accent)" : clip.virality_score >= 40 ? "#f59e0b" : "#ef4444";
  const cssRatio = aspectRatio && aspectRatio !== "original" ? aspectRatio.replace(":", "/") : "9/16";

  return (
    <motion.div
      className="clip-card"
      initial={{ opacity: 0, y: 20 }}
      animate={{ opacity: 1, y: 0 }}
      transition={{ duration: 0.35 }}
      style={{
        background: "#131318",
        border: `1px solid ${isSelected ? "var(--accent)" : "rgba(255,255,255,0.08)"}`,
        borderRadius: "14px",
        overflow: "hidden",
        boxShadow: isSelected ? "0 0 20px rgba(255,224,0,0.15)" : "none",
        cursor: "pointer",
      }}
      onClick={onSelect}
    >
      {/* Video Container (No Zooming, Full Frame) */}
      <div style={{ position: "relative", background: "#050507", height: "260px", width: "100%", overflow: "hidden", display: "flex", justifyContent: "center", alignItems: "center" }}>
        <video
          ref={videoRef}
          src={`${fileUrl}#t=0.5`}
          style={{ height: "100%", width: "100%", objectFit: "contain" }}
          onEnded={() => setPlaying(false)}
          preload="metadata"
          playsInline
        />
        <button
          onClick={togglePlay}
          style={{
            position: "absolute", inset: 0, display: "flex", alignItems: "center", justifyContent: "center",
            background: playing ? "transparent" : "rgba(0,0,0,0.3)", transition: "background 0.2s", border: "none", cursor: "pointer",
          }}
        >
          {!playing && (
            <div style={{ width: 48, height: 48, borderRadius: "50%", background: "var(--accent)", display: "flex", alignItems: "center", justifyContent: "center", boxShadow: "0 0 16px rgba(255,224,0,0.4)" }}>
              <Play size={20} fill="#000" color="#000" />
            </div>
          )}
        </button>

        {/* Virality score badge (Solid High-Contrast Backdrop) */}
        <div style={{
          position: "absolute", top: "0.75rem", right: "0.75rem",
          background: "#09090d",
          borderRadius: "999px", padding: "0.3rem 0.65rem",
          display: "flex", alignItems: "center", gap: "0.35rem",
          border: `1px solid ${viralityColor}`,
          boxShadow: "0 2px 8px rgba(0,0,0,0.85)",
        }}>
          <Zap size={13} fill={viralityColor} color={viralityColor} />
          <span style={{ fontSize: "0.8rem", fontWeight: 900, color: viralityColor }}>{clip.virality_score}</span>
        </div>

        {/* Timestamp (Solid High-Contrast Backdrop) */}
        <div style={{
          position: "absolute", bottom: "0.75rem", left: "0.75rem",
          background: "#09090d", border: "1px solid rgba(255,255,255,0.15)", borderRadius: "6px",
          padding: "0.25rem 0.6rem", fontSize: "0.72rem", color: "#fff", fontWeight: 700,
          boxShadow: "0 2px 8px rgba(0,0,0,0.85)",
        }}>
          {clip.start_time} – {clip.end_time}
        </div>
      </div>

      {/* Details Body */}
      <div style={{ padding: "1rem" }}>
        {clip.hook_title && (
          <h3 style={{ fontSize: "0.9rem", fontWeight: 700, marginBottom: "0.5rem", lineHeight: 1.3, color: "#fff" }}>
            {clip.hook_title}
          </h3>
        )}

        <div style={{ display: "flex", alignItems: "center", gap: "0.5rem", marginBottom: "0.75rem" }}>
          <HookTypeBadge hookType={clip.hook_type || "none"} />
          <span style={{ fontSize: "0.72rem", color: "#888" }}>{clip.duration.toFixed(1)}s</span>
        </div>

        {/* Score Bars */}
        <div style={{ display: "grid", gap: "0.4rem", marginBottom: "0.75rem" }}>
          <ScoreBar label="Hook" score={clip.hook_score} />
          <ScoreBar label="Engagement" score={clip.engagement_score} />
          <ScoreBar label="Value" score={clip.value_score} />
          <ScoreBar label="Shareability" score={clip.shareability_score} />
        </div>

        {/* Reasoning */}
        {clip.reasoning && (
          <button
            onClick={(e) => { e.stopPropagation(); setExpanded(!expanded); }}
            style={{ fontSize: "0.75rem", color: "#aaa", background: "none", border: "none", cursor: "pointer", marginBottom: expanded ? "0.5rem" : 0 }}
          >
            {expanded ? "Hide" : "Why this clip?"} ↗
          </button>
        )}
        {expanded && clip.reasoning && (
          <p style={{ fontSize: "0.78rem", color: "#aaa", lineHeight: 1.5, borderLeft: "2px solid var(--accent)", paddingLeft: "0.75rem", marginBottom: "0.75rem" }}>
            {clip.reasoning}
          </p>
        )}

        {/* Action Buttons */}
        <div style={{ display: "flex", gap: "0.4rem", flexWrap: "wrap", marginTop: "0.75rem" }}>
          <a href={fileUrl} download={clip.filename} onClick={(e) => e.stopPropagation()} className="btn btn-primary btn-sm" style={{ flex: 1, background: "var(--accent)", color: "#000", fontWeight: 700 }}>
            <Download size={13} /> Download
          </a>
          <button className="btn btn-ghost btn-icon btn-sm" onClick={deleteClip} title="Delete clip" style={{ color: "#ef4444" }}>
            <Trash2 size={13} />
          </button>
        </div>
      </div>
    </motion.div>
  );
}

function DisplayValue({ value, label }: { value: unknown; label: string }) {
  // Render a string
  if (typeof value === "string") {
    return <p style={{ color: "#ccc", fontSize: ".8rem", lineHeight: 1.6, whiteSpace: "pre-wrap", margin: 0 }}>{value || "—"}</p>;
  }
  // Render an array of primitives as a bulleted list
  if (Array.isArray(value)) {
    const allPrimitive = value.every(v => typeof v === "string" || typeof v === "number");
    if (allPrimitive) {
      return (
        <ul style={{ margin: 0, paddingLeft: "1.1rem", color: "#ccc", fontSize: ".8rem", lineHeight: 1.6 }}>
          {value.map((item, i) => <li key={i}>{String(item)}</li>)}
        </ul>
      );
    }
  }
  // Fallback: nested objects/arrays render their sub-fields
  return <ObjectFields value={value as any} />;
}

function ObjectFields({ value }: { value: Record<string, unknown> | unknown[] }) {
  if (Array.isArray(value)) {
    return (
      <div style={{ display: "grid", gap: ".5rem" }}>
        {value.map((item, i) => <CopyValue key={i} label={`Item ${i + 1}`} value={item} />)}
      </div>
    );
  }
  return (
    <div style={{ display: "grid", gap: ".75rem" }}>
      {Object.entries(value as Record<string, unknown>).map(([key, nested]) => (
        <CopyValue key={key} label={key} value={nested} />
      ))}
    </div>
  );
}

function CopyValue({ label, value }: { label: string; value: unknown }) {
  const text = Array.isArray(value) ? value.join("\n") : typeof value === "object" ? JSON.stringify(value, null, 2) : String(value ?? "");
  return (
    <div style={{ background: "#0b0b0e", border: "1px solid rgba(255,255,255,.08)", borderRadius: 9, padding: ".7rem" }}>
      <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between", marginBottom: ".45rem" }}>
        <span style={{ color: "#fb7185", fontSize: ".68rem", fontWeight: 900, textTransform: "uppercase", letterSpacing: ".04em" }}>{label.replace(/_/g, " ")}</span>
        <button onClick={() => { navigator.clipboard.writeText(text); toast.success(`${label.replace(/_/g, " ")} copied`); }} style={{ background: "rgba(244,63,94,.12)", border: "1px solid rgba(244,63,94,.25)", color: "#fb7185", cursor: "pointer", fontSize: ".68rem", fontWeight: 800, padding: ".25rem .6rem", borderRadius: 6 }}>Copy</button>
      </div>
      <DisplayValue value={value} label={label} />
    </div>
  );
}

function RepurposeResult({ task, taskId }: { task: Task; taskId: string }) {
  const stages = ["Source Analysis", "Campaign Strategy", "Video Adaptation", "Platform Copy", "Package Finalization"];
  if (task.status !== "completed" || !task.repurpose_result) return <motion.div initial={{opacity:0}} animate={{opacity:1}} style={{ background: "linear-gradient(180deg,#181016,#0d0b0e)", border: "1px solid rgba(244,63,94,.3)", borderRadius: 20, padding: "1.75rem", marginBottom: "2rem" }}><div style={{ display: "flex", justifyContent: "space-between", marginBottom: "1.3rem" }}><div><h3 style={{ margin: 0, color: "#fff" }}>Building your repurpose campaign</h3><span style={{ color: "#888", fontSize: ".75rem" }}>{task.progress_message}</span></div><strong style={{color:"#fb7185"}}>{task.progress}%</strong></div><div style={{ display:"grid",gridTemplateColumns:"repeat(5,1fr)",gap:".5rem" }}>{stages.map((stage,i)=><div key={stage} style={{textAlign:"center"}}><div style={{width:34,height:34,borderRadius:"50%",margin:"0 auto .5rem",background:task.progress >= (i+1)*20?"#f43f5e":"#18181d",display:"grid",placeItems:"center",fontSize:".7rem",fontWeight:900}}>{i+1}</div><span style={{fontSize:".65rem",color:task.progress >= i*20?"#fff":"#666"}}>{stage}</span></div>)}</div><div style={{height:6,background:"#08080a",borderRadius:5,marginTop:"1.2rem",overflow:"hidden"}}><div style={{height:"100%",width:`${task.progress}%`,background:"linear-gradient(90deg,#f43f5e,#fb7185)"}} /></div></motion.div>;
  const result = task.repurpose_result;
  let copy: any = result.platform_copy || {};
  // Defensive normalization: unwrap a "platforms" wrapper and drop metadata keys.
  if (copy.platforms && typeof copy.platforms === "object") copy = copy.platforms;
  (["audience", "campaign", "goal", "tone", "source", "core_message", "cta"] as const)
    .forEach(k => { delete copy[k]; });
  return <motion.div initial={{opacity:0,y:10}} animate={{opacity:1,y:0}} style={{ marginBottom: "2rem" }}>
    <div style={{ background: "linear-gradient(180deg,#181016,#0d0b0e)", border: "1px solid rgba(244,63,94,.3)", borderRadius: 20, padding: "1.6rem", marginBottom: "1rem" }}>
      <div style={{display:"flex",justifyContent:"space-between",gap:"1rem",alignItems:"center",flexWrap:"wrap"}}>
        <div>
          <span style={{color:"#fb7185",fontSize:".7rem",fontWeight:900}}>NOVA REPURPOSE CAMPAIGN</span>
          <h2 style={{margin:".3rem 0",color:"#fff"}}>{result.campaign_name}</h2>
          <p style={{margin:0,color:"#999",fontSize:".8rem"}}>{[result.audience, result.goal, result.tone].filter(Boolean).join(" · ")}</p>
        </div>
        <a href={`/tasks/${taskId}/repurpose-pdf`} download className="btn" style={{background:"#f43f5e",color:"#fff",fontWeight:900}}>Download Campaign PDF</a>
      </div>
      {(result.core_message || result.cta) && <div style={{marginTop:"1rem",padding:".8rem",borderLeft:"3px solid #f43f5e",background:"rgba(244,63,94,.06)",color:"#ddd",fontSize:".8rem"}}>{result.core_message || ""}{result.cta && <strong style={{display:"block",color:"#fb7185",marginTop:".3rem"}}>CTA: {result.cta}</strong>}</div>}
      {(result.videos || []).length > 0 && (
        <div style={{ marginTop: "1rem", display: "flex", gap: ".5rem", flexWrap: "wrap" }}>
          {(result.videos as any[]).map(v => (
            <span key={v.platform} style={{ background: "rgba(244,63,94,.1)", border: "1px solid rgba(244,63,94,.25)", padding: ".3rem .7rem", borderRadius: 999, fontSize: ".72rem", fontWeight: 800, color: "#fb7185", textTransform: "capitalize" }}>{v.platform} · {v.aspect_ratio} · {v.duration}s</span>
          ))}
        </div>
      )}
    </div>
    {Object.keys(copy).length > 0 && (
      <h3 style={{ color:"#fff", fontSize:"1rem", margin:"0 0 .9rem" }}>Platform Content</h3>
    )}
    <div style={{display:"grid",gap:"1rem"}}>{Object.entries(copy).map(([platform, content])=>
      <section key={platform} style={{background:"#121217",border:"1px solid rgba(255,255,255,.09)",borderRadius:16,padding:"1.2rem"}}>
        <h3 style={{margin:"0 0 .8rem",color:"#fff",textTransform:"capitalize",display:"flex",alignItems:"center",gap:".5rem"}}>
          {platform}
          <span style={{fontSize:".62rem",color:"#fb7185",background:"rgba(244,63,94,.12)",padding:".15rem .5rem",borderRadius:999,textTransform:"uppercase",letterSpacing:".05em"}}>Copy-ready</span>
        </h3>
        <div style={{display:"grid",gridTemplateColumns:"repeat(auto-fit,minmax(280px,1fr))",gap:".7rem",alignItems:"start"}}>{Object.entries((content as any)||{}).map(([key,value])=><CopyValue key={key} label={key} value={value} />)}</div>
      </section>
    )}</div>
  </motion.div>;
}

export default function TaskPage() {
  const { id } = useParams<{ id: string }>();
  const [task, setTask] = useState<Task | null>(null);
  const [progress, setProgress] = useState(0);
  const [message, setMessage] = useState("Loading...");
  const [status, setStatus] = useState("queued");
  const [loading, setLoading] = useState(true);
  const [selectedClipIndex, setSelectedClipIndex] = useState(0);
  const [chatOpen, setChatOpen] = useState(false);
  const [chatInput, setChatInput] = useState("");
  const [chatMessages, setChatMessages] = useState<{role: string; text: string}[]>([]);
  const [chatLoading, setChatLoading] = useState(false);
  const chatRef = useRef<HTMLDivElement>(null);
  const sseRef = useRef<EventSource | null>(null);
  const [approving, setApproving] = useState(false);
  const [replanOpen, setReplanOpen] = useState(false);
  const [replanMsg, setReplanMsg] = useState("");
  const [replanLoading, setReplanLoading] = useState(false);

  useEffect(() => {
    if (!id) return;
    let mounted = true;

    const loadTask = async () => {
      try {
        const t = await api.getTask(id);
        if (!mounted) return;
        setTask(t);
        if (t.source_type === "studio" || t.source_url?.startsWith("studio://")) {
          sessionStorage.setItem("nova_last_task_type", "studio");
        } else if (t.source_type === "agentic" || t.source_url?.startsWith("novaedit://")) {
          sessionStorage.setItem("nova_last_task_type", "agentic");
        } else if (t.source_type === "repurpose" || t.source_url?.startsWith("repurpose://")) {
          sessionStorage.setItem("nova_last_task_type", "repurpose");
        } else {
          sessionStorage.setItem("nova_last_task_type", "clipper");
        }
        window.dispatchEvent(new Event("nova-task-type-change"));
        setStatus(t.status);
        setProgress(t.progress || 0);
        setMessage(t.progress_message || "");
      } catch (e) {
        // SSE will retry; suppress toast on first load
      } finally {
        setLoading(false);
      }
    };

    loadTask();

    // Subscribe to SSE
    sseRef.current = createTaskSSE(
      id,
      (e) => {
        if (!mounted) return;
        setProgress(e.percent);
        setMessage(e.message);
         setStatus(e.status);
         if (e.status === "completed") {
           api.getTask(id).then(fresh => { if (mounted) setTask(fresh); }).catch(() => {});
         }
      },
      (_e) => {
        if (mounted) loadTask();
      },
      () => {
        if (mounted) loadTask();
      },
      (err) => {
        if (mounted) {
          setStatus("error");
          setMessage(err || "Processing failed");
        }
      }
    );

    return () => {
      mounted = false;
      sseRef.current?.close();
    };
  }, [id]);

  const activeClip = task?.clips?.[selectedClipIndex] || task?.clips?.[0];

  const handleChatSend = async () => {
    const msg = chatInput.trim();
    if (!msg || !id || !task?.clips?.length) return;
    setChatInput("");
    setChatMessages(prev => [...prev, { role: "user", text: msg }]);
    setChatLoading(true);
    try {
      const clipIds = task.clips.map(c => c.id);
      const result = await api.aiEdit(id, clipIds, msg);
      const actions = result.actions_applied?.map((a: any) =>
        `${a.action} on ${a.clip_id?.slice(0,8) || "all"}: ${a.status}`
      ).join("\n") || "Done";
      setChatMessages(prev => [...prev, { role: "assistant", text: `Applied: ${actions}` }]);
    } catch (e: any) {
      setChatMessages(prev => [...prev, { role: "assistant", text: `Error: ${e.message}` }]);
    } finally {
      setChatLoading(false);
    }
  };

  const isAgentic = task?.source_type === "agentic" || task?.source_url?.startsWith("novaedit://");
  const isRepurpose = task?.source_type === "repurpose" || task?.source_url?.startsWith("repurpose://");

  const handleApprovePlan = async () => {
    if (!id) return;
    setApproving(true);
    try {
      await api.approveEditPlan(id);
      toast.success("Edit plan approved. Rendering started.");
      setStatus("queued");
      setProgress(50);
      setMessage("Edit approved. Rendering...");
    } catch (e: any) {
      toast.error(e.message || "Failed to approve plan");
    } finally {
      setApproving(false);
    }
  };

  const handleReplan = async () => {
    const msg = replanMsg.trim();
    if (!msg || !id) return;
    setReplanLoading(true);
    try {
      await api.replan(id, msg);
      toast.success("Re-planning with your feedback...");
      setReplanOpen(false);
      setReplanMsg("");
      setStatus("queued");
      setProgress(50);
      setMessage("Re-planning with your feedback...");
    } catch (e: any) {
      toast.error(e.message || "Failed to re-plan");
    } finally {
      setReplanLoading(false);
    }
  };

  return (
    <div style={{ paddingTop: "64px", minHeight: "100vh", background: "#0b0b0e", color: "#fff" }}>
      <div className="container" style={{ maxWidth: "1200px", margin: "0 auto", padding: "2rem 1.5rem 4rem" }}>
        
        {/* Top Nav Bar */}
        <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between", marginBottom: "1.5rem" }}>
          <div style={{ display: "flex", alignItems: "center", gap: "1rem" }}>
            <Link to="/history" className="btn btn-ghost btn-icon" style={{ background: "#16161c" }}><ArrowLeft size={18} /></Link>
            <div>
              <h1 style={{ fontSize: "1.25rem", fontWeight: 800, margin: 0, overflow: "hidden", textOverflow: "ellipsis", maxWidth: "600px" }}>
                {task?.source_title || (task?.source_url?.startsWith("studio://") ? "Nova Studio AI Video" : task?.source_url || "Processing Video")}
              </h1>
              <span style={{ fontSize: "0.78rem", color: "#888" }}>Task ID: {id?.slice(0, 12)}...</span>
              {(task?.source_url?.startsWith("studio://") || task?.source_type === "studio") ? (
                <span style={{ marginLeft: "0.5rem", fontSize: "0.68rem", fontWeight: 900, padding: "0.15rem 0.55rem", borderRadius: "999px", background: "rgba(139,92,246,0.2)", color: "#8b5cf6", border: "1px solid rgba(139,92,246,0.3)", textTransform: "uppercase", letterSpacing: "0.04em" }}>
                  Faceless AI
                </span>
              ) : isAgentic ? (
                <span style={{ marginLeft: "0.5rem", fontSize: "0.68rem", fontWeight: 900, padding: "0.15rem 0.55rem", borderRadius: "999px", background: "rgba(34,211,238,0.15)", color: "#22d3ee", border: "1px solid rgba(34,211,238,0.3)", textTransform: "uppercase", letterSpacing: "0.04em" }}>
                  Agentic AI
                </span>
              ) : isRepurpose ? (
                <span style={{ marginLeft: "0.5rem", fontSize: "0.68rem", fontWeight: 900, padding: "0.15rem 0.55rem", borderRadius: "999px", background: "rgba(244,63,94,0.15)", color: "#fb7185", border: "1px solid rgba(244,63,94,0.3)", textTransform: "uppercase", letterSpacing: "0.04em" }}>Repurpose</span>
              ) : (
                <span style={{ marginLeft: "0.5rem", fontSize: "0.68rem", fontWeight: 900, padding: "0.15rem 0.55rem", borderRadius: "999px", background: "rgba(255,224,0,0.15)", color: "var(--accent)", border: "1px solid rgba(255,224,0,0.25)", textTransform: "uppercase", letterSpacing: "0.04em" }}>
                  Clip
                </span>
              )}
            </div>
          </div>

          <span
            style={{
              fontSize: "0.78rem", fontWeight: 900, textTransform: "uppercase", padding: "0.3rem 0.85rem", borderRadius: "999px",
              background: status === "completed" ? "#047857" : "#854d0e",
              color: "#ffffff",
              border: `1px solid ${status === "completed" ? "#10b981" : "#eab308"}`,
              boxShadow: "0 4px 12px rgba(0,0,0,0.9)",
              letterSpacing: "0.05em",
            }}
          >
            {status}
          </span>
        </div>

        {/* High-Tech Visual Pipeline Progression Stepper */}
        {loading ? (
          <div style={{ textAlign: "center", padding: "3rem 0", color: "#666" }}>
            <div className="spinner" style={{ width: 32, height: 32, border: "3px solid rgba(255,255,255,0.08)", borderTopColor: "var(--accent)", borderRadius: "50%", animation: "spin 0.8s linear infinite", margin: "0 auto 1rem" }} />
            <span>Loading task...</span>
          </div>
        ) : status !== "completed" && status !== "error" && task && (() => {
          const isStudio = task.source_url?.startsWith("studio://") || task.source_type === "studio";
          if (isRepurpose) {
            return <RepurposeResult task={task} taskId={id!} />;
          }
          const accent = isAgentic ? "rgba(34,211,238,1)" : isStudio ? "#8b5cf6" : "var(--accent)";
          const accentSoft = isAgentic ? "rgba(34,211,238,0.15)" : isStudio ? "rgba(139,92,246,0.15)" : "rgba(255, 224, 0, 0.15)";
          const PIPELINE_STAGES = isStudio ? [
            { id: "decompose", label: "AI Script Decompose", threshold: 10, Icon: Brain },
            { id: "tts", label: "Voice Synthesis", threshold: 30, Icon: Mic },
            { id: "scrape", label: "Stock Media Scrape", threshold: 50, Icon: Download },
            { id: "stitch", label: "Scene Stitching", threshold: 75, Icon: Film },
            { id: "render", label: "Captions & Finalize", threshold: 100, Icon: Sparkles },
          ] : isAgentic ? [
            { id: "preprocess", label: "Ingest & Analyze", threshold: 20, Icon: Download },
            { id: "director", label: "Director Plans", threshold: 45, Icon: Brain },
            { id: "approval", label: "Your Approval", threshold: 50, Icon: ThumbsUp },
            { id: "editor", label: "Editor Renders", threshold: 90, Icon: Crop },
            { id: "review", label: "Reviewer Scores", threshold: 100, Icon: Wand2 },
          ] : [
            { id: "download", label: "Download Stream", threshold: 20, Icon: Download },
            { id: "transcribe", label: "Speech Recognition", threshold: 45, Icon: Mic },
            { id: "analyze", label: "Virality Scoring", threshold: 70, Icon: Brain },
            { id: "crop", label: "Smart 9:16 Crop", threshold: 88, Icon: Crop },
            { id: "render", label: "Karaoke Burn", threshold: 100, Icon: Sparkles },
          ];

          return (
            <motion.div
              initial={{ opacity: 0, scale: 0.98 }}
              animate={{ opacity: 1, scale: 1 }}
              style={{
                background: "#131318",
                border: `1px solid ${isAgentic ? "rgba(34,211,238,0.25)" : isStudio ? "rgba(139,92,246,0.2)" : "rgba(255,255,255,0.12)"}`,
                borderRadius: "20px",
                padding: "2rem 1.75rem",
                marginBottom: "2rem",
                boxShadow: "0 20px 50px rgba(0,0,0,0.6)",
              }}
            >
              <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between", marginBottom: "1.75rem" }}>
                <div style={{ display: "flex", alignItems: "center", gap: "0.75rem" }}>
                  <div style={{ width: 40, height: 40, borderRadius: "50%", background: accentSoft, display: "flex", alignItems: "center", justifyContent: "center" }}>
                    <Radio size={20} color={isAgentic ? "#22d3ee" : isStudio ? "#8b5cf6" : "var(--accent)"} className="pulse" />
                  </div>
                  <div>
                    <h3 style={{ fontSize: "1.1rem", fontWeight: 800, margin: 0, color: "#fff" }}>{message || (isAgentic ? "Agentic Editing..." : isStudio ? "Generating AI Video..." : "Processing Video...")}</h3>
                    <span style={{ fontSize: "0.78rem", color: "#888" }}>{isAgentic ? "NovaEdit Agent Pipeline" : isStudio ? "Nova Studio Pipeline" : "Clipper Pipeline"}</span>
                  </div>
                </div>
                <div style={{ background: `${accent}1f`, border: `1px solid ${accent}4d`, borderRadius: "999px", padding: "0.35rem 0.9rem" }}>
                  <span style={{ fontSize: "1rem", fontWeight: 900, color: isAgentic ? "#22d3ee" : isStudio ? "#8b5cf6" : "var(--accent)" }}>{progress}%</span>
                </div>
              </div>

              {/* Step Progression Bar */}
              <div style={{ display: "grid", gridTemplateColumns: `repeat(${PIPELINE_STAGES.length}, 1fr)`, gap: "0.5rem", position: "relative", marginBottom: "1.5rem" }}>
                {PIPELINE_STAGES.map((stg, idx) => {
                  const IconComponent = stg.Icon;
                  const isDone = progress >= stg.threshold;
                  const isActive = progress < stg.threshold && (idx === 0 || progress >= PIPELINE_STAGES[idx - 1].threshold);

                  return (
                    <div key={stg.id} style={{ textAlign: "center", position: "relative", zIndex: 2 }}>
                      <div
                        style={{
                          width: 44,
                          height: 44,
                          borderRadius: "50%",
                          margin: "0 auto 0.6rem",
                          display: "flex",
                          alignItems: "center",
                          justifyContent: "center",
                          background: isDone ? "#22c55e" : isActive ? accent : "#09090c",
                          color: isDone || isActive ? "#000" : "#666",
                          border: `2px solid ${isDone ? "#22c55e" : isActive ? accent : "rgba(255,255,255,0.1)"}`,
                          boxShadow: isActive ? `0 0 20px ${accent}` : isDone ? "0 0 15px rgba(34, 197, 94, 0.3)" : "none",
                          transition: "all 0.3s",
                        }}
                      >
                        {isDone ? <CheckCircle2 size={20} color="#000" /> : <IconComponent size={20} color={isActive ? "#000" : "#666"} />}
                      </div>
                      <span style={{ fontSize: "0.75rem", fontWeight: isActive ? 800 : 600, color: isActive ? (isAgentic ? "#22d3ee" : isStudio ? "#8b5cf6" : "var(--accent)") : isDone ? "#fff" : "#666", display: "block" }}>
                        {stg.label}
                      </span>
                    </div>
                  );
                })}
              </div>

              {/* Progress Track */}
              <div style={{ background: "#09090c", height: "8px", borderRadius: "4px", overflow: "hidden" }}>
                <div style={{ width: `${progress}%`, background: isStudio ? "linear-gradient(90deg, #8b5cf6 0%, #22c55e 100%)" : "linear-gradient(90deg, var(--accent) 0%, #22c55e 100%)", height: "100%", transition: "width 0.4s ease-out" }} />
              </div>
            </motion.div>
          );
        })()}

        {!loading && isRepurpose && status === "completed" && task && (
          <RepurposeResult task={task} taskId={id!} />
        )}

        {/* NovaEdit: Edit Plan Approval Gate */}
        {isAgentic && status === "awaiting_approval" && task?.edit_plan && (
          <motion.div
            initial={{ opacity: 0, y: 12 }}
            animate={{ opacity: 1, y: 0 }}
            style={{
              background: "linear-gradient(180deg, #10161c 0%, #0b0b0e 100%)",
              border: "1px solid rgba(34,211,238,0.35)",
              borderRadius: "20px",
              padding: "2rem 1.75rem",
              marginBottom: "2rem",
              boxShadow: "0 0 40px rgba(34,211,238,0.08), 0 20px 50px rgba(0,0,0,0.6)",
            }}
          >
            <div style={{ display: "flex", alignItems: "center", gap: "0.75rem", marginBottom: "1.5rem" }}>
              <div style={{ width: 42, height: 42, borderRadius: "50%", background: "rgba(34,211,238,0.15)", display: "flex", alignItems: "center", justifyContent: "center" }}>
                <ListChecks size={20} color="#22d3ee" />
              </div>
              <div>
                <h3 style={{ fontSize: "1.1rem", fontWeight: 800, margin: 0, color: "#fff" }}>Director's Edit Plan: Ready for Approval</h3>
                <span style={{ fontSize: "0.78rem", color: "#888" }}>The Director agent analyzed your footage and proposed this cut. Review & approve to start rendering.</span>
              </div>
            </div>

            {task.edit_plan.rationale && (
              <div style={{ background: "rgba(34,211,238,0.07)", borderLeft: "3px solid #22d3ee", borderRadius: "8px", padding: "0.9rem 1.1rem", marginBottom: "1.25rem" }}>
                <span style={{ fontSize: "0.8rem", color: "#9fe8f6", lineHeight: 1.5, display: "block" }}>{task.edit_plan.rationale}</span>
              </div>
            )}

            {task.edit_plan.entries && task.edit_plan.entries.length > 0 && (
              <div style={{ display: "flex", flexDirection: "column", gap: "0.6rem", marginBottom: "1.5rem" }}>
                {task.edit_plan.entries.map((entry, i) => (
                  <div key={entry.shot_id + i} style={{ display: "grid", gridTemplateColumns: "28px 1fr auto", gap: "0.75rem", alignItems: "center", background: "rgba(255,255,255,0.03)", border: "1px solid rgba(255,255,255,0.08)", borderRadius: "12px", padding: "0.7rem 0.9rem" }}>
                    <div style={{ width: 26, height: 26, borderRadius: "50%", background: "rgba(34,211,238,0.15)", color: "#22d3ee", fontSize: "0.72rem", fontWeight: 800, display: "flex", alignItems: "center", justifyContent: "center" }}>{i + 1}</div>
                    <div style={{ minWidth: 0 }}>
                      <div style={{ fontSize: "0.8rem", fontWeight: 700, color: "#fff", overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>{entry.shot_id}</div>
                      <div style={{ fontSize: "0.72rem", color: "#888", marginTop: "2px" }}>
                        {entry.start_trim.toFixed(2)}s → {entry.end_trim.toFixed(2)}s · position {entry.position}
                      </div>
                      {entry.text_overlay && (
                        <div style={{ fontSize: "0.72rem", color: "#22d3ee", marginTop: "2px", fontStyle: "italic" }}>"{entry.text_overlay}"</div>
                      )}
                    </div>
                    <span style={{ fontSize: "0.68rem", fontWeight: 800, color: "#22d3ee", background: "rgba(34,211,238,0.1)", borderRadius: "999px", padding: "0.2rem 0.6rem", whiteSpace: "nowrap" }}>
                      {(entry.end_trim - entry.start_trim).toFixed(1)}s
                    </span>
                  </div>
                ))}
              </div>
            )}

            <div style={{ display: "flex", gap: "0.75rem", flexWrap: "wrap" }}>
              <button
                onClick={handleApprovePlan}
                disabled={approving}
                style={{ display: "flex", alignItems: "center", gap: "0.5rem", padding: "0.7rem 1.5rem", borderRadius: "10px", border: "none", background: "linear-gradient(90deg, #22d3ee 0%, #06b6d4 100%)", color: "#000", fontSize: "0.85rem", fontWeight: 900, cursor: "pointer", transition: "all 0.2s" }}
                onMouseEnter={e => (e.currentTarget.style.opacity = "0.85")}
                onMouseLeave={e => (e.currentTarget.style.opacity = "1")}
              >
                {approving ? "Approving..." : <> <ThumbsUp size={16} /> Approve & Start Rendering</>}
              </button>
              <button
                onClick={() => setReplanOpen(true)}
                style={{ display: "flex", alignItems: "center", gap: "0.5rem", padding: "0.7rem 1.5rem", borderRadius: "10px", border: "1px solid rgba(255,255,255,0.15)", background: "transparent", color: "#fff", fontSize: "0.85rem", fontWeight: 700, cursor: "pointer", transition: "all 0.2s" }}
              >
                <MessageCircle size={16} /> Re-plan with feedback
              </button>
            </div>
          </motion.div>
        )}

        {/* NovaEdit: Reviewer Score Card */}
        {isAgentic && status === "completed" && task?.review_score && (
          <motion.div
            initial={{ opacity: 0, y: 12 }}
            animate={{ opacity: 1, y: 0 }}
            style={{
              background: "linear-gradient(180deg, #10161c 0%, #0b0b0e 100%)",
              border: "1px solid rgba(34,211,238,0.3)",
              borderRadius: "20px",
              padding: "1.75rem",
              marginBottom: "2rem",
            }}
          >
            <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between", marginBottom: "1.25rem", flexWrap: "wrap", gap: "0.75rem" }}>
              <div style={{ display: "flex", alignItems: "center", gap: "0.6rem" }}>
                <Wand2 size={18} color="#22d3ee" />
                <h3 style={{ fontSize: "1rem", fontWeight: 800, margin: 0, color: "#fff" }}>Reviewer Agent Scorecard</h3>
              </div>
              <div style={{ background: "rgba(34,211,238,0.12)", border: "1px solid rgba(34,211,238,0.3)", borderRadius: "999px", padding: "0.3rem 0.9rem" }}>
                <span style={{ fontSize: "0.85rem", fontWeight: 900, color: "#22d3ee" }}>Overall {(task.review_score.overall ?? 0).toFixed(2)}</span>
              </div>
            </div>
            <div style={{ display: "grid", gridTemplateColumns: "repeat(auto-fit, minmax(160px, 1fr))", gap: "1rem" }}>
              <ReviewBar label="Adherence" value={task.review_score.adherence} />
              <ReviewBar label="Pacing" value={task.review_score.pacing} />
              <ReviewBar label="Visual Quality" value={task.review_score.visual_quality} />
              <ReviewBar label="Watchability" value={task.review_score.watchability} />
            </div>
            {task.review_score.feedback && (
              <div style={{ marginTop: "1.25rem", background: "rgba(255,255,255,0.03)", border: "1px solid rgba(255,255,255,0.08)", borderRadius: "10px", padding: "0.9rem 1.1rem" }}>
                <span style={{ fontSize: "0.8rem", color: "#bbb", lineHeight: 1.6, display: "block" }}>{task.review_score.feedback}</span>
              </div>
            )}
            <button
              onClick={() => setReplanOpen(true)}
              style={{ display: "flex", alignItems: "center", gap: "0.5rem", marginTop: "1.25rem", padding: "0.6rem 1.2rem", borderRadius: "10px", border: "1px solid rgba(34,211,238,0.4)", background: "rgba(34,211,238,0.08)", color: "#22d3ee", fontSize: "0.8rem", fontWeight: 800, cursor: "pointer" }}
            >
              <RefreshCw size={15} /> Re-edit with feedback
            </button>
          </motion.div>
        )}

        {/* XPLAY Inspired Featured Broadcast Studio Player (Compact & Sleek) */}
        {task?.clips && task.clips.length > 0 && activeClip && (
          <div
            style={{
              background: "linear-gradient(180deg, #14141a 0%, #0b0b0e 100%)",
              border: "1px solid rgba(255, 255, 255, 0.12)",
              borderRadius: "16px",
              padding: "1.25rem",
              marginBottom: "1.75rem",
              display: "grid",
              gridTemplateColumns: "260px 1fr",
              gap: "1.75rem",
              alignItems: "center",
              boxShadow: "0 15px 35px rgba(0,0,0,0.5)",
            }}
          >
            {/* Player Left: Featured Video Container (Wider & Prominent) */}
            <div style={{ background: "#000", borderRadius: "12px", overflow: "hidden", position: "relative", height: "340px", width: "100%", display: "flex", justifyContent: "center", alignItems: "center" }}>
              <video
                key={activeClip.id}
                src={api.clipFileUrl(id!, activeClip.id)}
                controls
                autoPlay={false}
                style={{ height: "100%", width: "100%", objectFit: "contain" }}
              />
            </div>

            {/* Player Right: Studio Metadata & Actions */}
            <div>
              <div style={{ display: "flex", alignItems: "center", gap: "0.5rem", marginBottom: "0.4rem" }}>
                <span style={{ background: "var(--accent)", color: "#000", fontSize: "0.68rem", fontWeight: 900, padding: "0.15rem 0.5rem", borderRadius: "4px", textTransform: "uppercase" }}>
                  CLIP #{selectedClipIndex + 1}
                </span>
                <HookTypeBadge hookType={activeClip.hook_type || "viral"} />
              </div>

              <h2 style={{ fontSize: "1.2rem", fontWeight: 800, marginBottom: "0.5rem", lineHeight: 1.25, color: "#fff" }}>
                {activeClip.hook_title || "Viral Clip Highlight"}
              </h2>

              <p style={{ fontSize: "0.8rem", color: "#aaa", lineHeight: 1.5, marginBottom: "1rem", borderLeft: "2px solid var(--accent)", paddingLeft: "0.75rem" }}>
                {activeClip.reasoning || "AI scoring detected high engagement and strong hook retention."}
              </p>

              <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: "0.6rem", marginBottom: "1rem", background: "#060608", padding: "0.75rem", borderRadius: "10px" }}>
                <div>
                  <span style={{ fontSize: "0.7rem", color: "#888" }}>Virality Score</span>
                  <div style={{ fontSize: "1.15rem", fontWeight: 900, color: "#22c55e" }}>{activeClip.virality_score} / 100</div>
                </div>
                <div>
                  <span style={{ fontSize: "0.7rem", color: "#888" }}>Duration</span>
                  <div style={{ fontSize: "1.15rem", fontWeight: 900, color: "#fff" }}>{activeClip.duration.toFixed(1)}s</div>
                </div>
              </div>

              <div style={{ display: "flex", gap: "0.5rem" }}>
                <a href={api.clipFileUrl(id!, activeClip.id)} download={activeClip.filename} className="btn btn-primary btn-md" style={{ flex: 1, background: "var(--accent)", color: "#000", fontWeight: 800, padding: "0.45rem" }}>
                  <Download size={15} /> Download MP4
                </a>
                <a href={`/api/tasks/${id}/download-all`} download={`novaclip_${id?.slice(0, 8)}.zip`} className="btn btn-secondary btn-md" style={{ background: "#1f1f28", color: "#fff", fontWeight: 700, padding: "0.45rem 0.85rem" }}>
                  <Download size={15} /> Download All (.zip)
                </a>
              </div>
            </div>
          </div>
        )}

        {/* Clips Grid (XPLAY Inspired) */}
        {task?.clips && task.clips.length > 0 && (
          <div>
            <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between", marginBottom: "1.25rem", flexWrap: "wrap", gap: "0.5rem" }}>
              <h2 style={{ fontSize: "1.2rem", fontWeight: 800 }}>
                Generated Clips ({task.clips.length})
              </h2>
              <div style={{ display: "flex", alignItems: "center", gap: "1rem" }}>
                <span style={{ fontSize: "0.8rem", color: "#888" }}>{task.aspect_ratio} • Sorted by virality</span>
                <a href={`/api/tasks/${id}/download-all`} download={`novaclip_${id?.slice(0, 8)}.zip`} className="btn btn-primary btn-sm" style={{ background: "var(--accent)", color: "#000", fontWeight: 800 }}>
                  <Download size={14} /> Download All (.zip)
                </a>
              </div>
            </div>

            <div style={{ display: "grid", gridTemplateColumns: "repeat(auto-fill, minmax(280px, 1fr))", gap: "1.25rem" }}>
              {task.clips.map((clip, idx) => (
                <ClipCard
                  key={clip.id}
                  clip={clip}
                  taskId={id!}
                  aspectRatio={task.aspect_ratio}
                  isSelected={selectedClipIndex === idx}
                  onSelect={() => setSelectedClipIndex(idx)}
                />
              ))}
            </div>
          </div>
        )}
      </div>

      {/* Floating AI Chat Button */}
      <button
        onClick={() => setChatOpen(!chatOpen)}
        style={{
          position: "fixed", bottom: "1.5rem", right: "1.5rem", zIndex: 1000,
          width: 52, height: 52, borderRadius: "50%",
          background: "var(--accent)", border: "none", cursor: "pointer",
          display: "flex", alignItems: "center", justifyContent: "center",
          boxShadow: "0 4px 20px rgba(255,224,0,0.4)",
        }}
      >
        {chatOpen ? <X size={22} color="#000" /> : <MessageCircle size={22} color="#000" />}
      </button>

      {/* AI Chat Panel */}
      {chatOpen && (
        <div
          ref={chatRef}
          style={{
            position: "fixed", bottom: "5.5rem", right: "1.5rem", zIndex: 1000,
            width: "360px", maxHeight: "480px",
            background: "#131318", border: "1px solid rgba(255,255,255,0.12)",
            borderRadius: "16px", display: "flex", flexDirection: "column",
            boxShadow: "0 20px 60px rgba(0,0,0,0.6)", overflow: "hidden",
          }}
        >
          <div style={{ padding: "0.85rem 1rem", borderBottom: "1px solid rgba(255,255,255,0.08)", display: "flex", alignItems: "center", gap: "0.5rem" }}>
            <Brain size={16} color="var(--accent)" />
            <span style={{ fontWeight: 800, fontSize: "0.85rem" }}>Edit with AI</span>
            <span style={{ marginLeft: "auto", fontSize: "0.68rem", color: "#888" }}>
              {task?.clips?.length || 0} clips
            </span>
          </div>
          <div style={{ flex: 1, overflowY: "auto", padding: "0.75rem", display: "flex", flexDirection: "column", gap: "0.5rem", minHeight: "200px" }}>
            {chatMessages.length === 0 && (
              <div style={{ fontSize: "0.78rem", color: "#888", textAlign: "center", padding: "2rem 0" }}>
                Tell AI what to do with your clips.<br />
                Try: "trim the first 2 seconds off clip 1" or "add captions to all clips"
              </div>
            )}
            {chatMessages.map((m, i) => (
              <div key={i} style={{
                maxWidth: "85%", padding: "0.5rem 0.75rem", borderRadius: "10px",
                fontSize: "0.78rem", lineHeight: 1.4,
                alignSelf: m.role === "user" ? "flex-end" : "flex-start",
                background: m.role === "user" ? "var(--accent)" : "#08080a",
                color: m.role === "user" ? "#000" : "#ddd",
                fontWeight: m.role === "user" ? 700 : 400, whiteSpace: "pre-wrap",
              }}>
                {m.text}
              </div>
            ))}
            {chatLoading && (
              <div style={{ alignSelf: "flex-start", background: "#08080a", padding: "0.5rem 0.75rem", borderRadius: "10px", fontSize: "0.78rem", color: "#888" }}>
                Thinking...
              </div>
            )}
          </div>
          <div style={{ padding: "0.6rem", borderTop: "1px solid rgba(255,255,255,0.08)", display: "flex", gap: "0.5rem" }}>
            <input
              value={chatInput}
              onChange={e => setChatInput(e.target.value)}
              onKeyDown={e => e.key === "Enter" && !chatLoading && handleChatSend()}
              placeholder="Ask AI to edit clips..."
              style={{
                flex: 1, background: "#08080a", border: "1px solid rgba(255,255,255,0.1)",
                borderRadius: "8px", padding: "0.5rem 0.75rem", fontSize: "0.8rem",
                color: "#fff", outline: "none",
              }}
            />
            <button
              onClick={handleChatSend}
              disabled={chatLoading || !chatInput.trim()}
              style={{
                background: "var(--accent)", border: "none", borderRadius: "8px",
                padding: "0.5rem", cursor: "pointer", display: "flex",
                alignItems: "center", justifyContent: "center",
                opacity: chatLoading || !chatInput.trim() ? 0.5 : 1,
              }}
            >
              <Send size={16} color="#000" />
            </button>
          </div>
        </div>
      )}

      {/* NovaEdit: Replan Feedback Modal */}
      {replanOpen && (
        <div style={{ position: "fixed", inset: 0, background: "rgba(0,0,0,0.75)", zIndex: 1000, display: "flex", alignItems: "center", justifyContent: "center", padding: "1rem" }} onClick={() => setReplanOpen(false)}>
          <div style={{ background: "#131318", border: "1px solid rgba(34,211,238,0.35)", borderRadius: "16px", padding: "1.75rem", maxWidth: "520px", width: "100%", boxShadow: "0 30px 80px rgba(0,0,0,0.7)" }} onClick={e => e.stopPropagation()}>
            <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between", marginBottom: "1rem" }}>
              <h3 style={{ fontSize: "1.05rem", fontWeight: 800, margin: 0, color: "#fff", display: "flex", alignItems: "center", gap: "0.5rem" }}>
                <MessageCircle size={18} color="#22d3ee" /> Re-plan with Feedback
              </h3>
              <button onClick={() => setReplanOpen(false)} style={{ background: "none", border: "none", color: "#888", cursor: "pointer", padding: "0.25rem" }}>
                <X size={18} />
              </button>
            </div>
            <p style={{ fontSize: "0.8rem", color: "#888", margin: "0 0 1rem", lineHeight: 1.5 }}>
              The Director agent will re-plan your edit using your notes. Examples: "Cut out the intro slower part", "Add a text hook at the start", "Make pacing faster".
            </p>
            <textarea
              value={replanMsg}
              onChange={e => setReplanMsg(e.target.value)}
              rows={4}
              maxLength={4000}
              placeholder="What should change in the next version?"
              style={{ width: "100%", background: "#0b0b0e", border: "1px solid rgba(255,255,255,0.12)", borderRadius: "10px", padding: "0.75rem", fontSize: "0.85rem", color: "#fff", outline: "none", resize: "vertical", fontFamily: "inherit", marginBottom: "1.25rem" }}
            />
            <div style={{ display: "flex", gap: "0.75rem", justifyContent: "flex-end" }}>
              <button onClick={() => setReplanOpen(false)} style={{ padding: "0.6rem 1.2rem", borderRadius: "10px", border: "1px solid rgba(255,255,255,0.15)", background: "transparent", color: "#ccc", fontSize: "0.82rem", fontWeight: 700, cursor: "pointer" }}>
                Cancel
              </button>
              <button
                onClick={handleReplan}
                disabled={replanLoading || !replanMsg.trim()}
                style={{ display: "flex", alignItems: "center", gap: "0.5rem", padding: "0.6rem 1.4rem", borderRadius: "10px", border: "none", background: "linear-gradient(90deg, #22d3ee 0%, #06b6d4 100%)", color: "#000", fontSize: "0.85rem", fontWeight: 900, cursor: "pointer", opacity: replanLoading || !replanMsg.trim() ? 0.5 : 1 }}
              >
                {replanLoading ? "Re-planning..." : <> <RefreshCw size={15} /> Re-plan</>}
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
