import { useState, useEffect, useRef } from "react";
import { useParams, Link } from "react-router-dom";
import { ArrowLeft, RefreshCw, XCircle, Play, Pause, Download, ExternalLink, Trash2, Copy, Zap, Flame, Radio, Mic, Brain, Crop, Sparkles, CheckCircle2, MessageCircle, Send, X, Film } from "lucide-react";
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
        }
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
          const PIPELINE_STAGES = isStudio ? [
            { id: "decompose", label: "AI Script Decompose", threshold: 10, Icon: Brain },
            { id: "tts", label: "Voice Synthesis", threshold: 30, Icon: Mic },
            { id: "scrape", label: "Stock Media Scrape", threshold: 50, Icon: Download },
            { id: "stitch", label: "Scene Stitching", threshold: 75, Icon: Film },
            { id: "render", label: "Captions & Finalize", threshold: 100, Icon: Sparkles },
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
                border: `1px solid ${isStudio ? "rgba(139,92,246,0.2)" : "rgba(255,255,255,0.12)"}`,
                borderRadius: "20px",
                padding: "2rem 1.75rem",
                marginBottom: "2rem",
                boxShadow: "0 20px 50px rgba(0,0,0,0.6)",
              }}
            >
              <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between", marginBottom: "1.75rem" }}>
                <div style={{ display: "flex", alignItems: "center", gap: "0.75rem" }}>
                  <div style={{ width: 40, height: 40, borderRadius: "50%", background: isStudio ? "rgba(139,92,246,0.15)" : "rgba(255, 224, 0, 0.15)", display: "flex", alignItems: "center", justifyContent: "center" }}>
                    <Radio size={20} color={isStudio ? "#8b5cf6" : "var(--accent)"} className="pulse" />
                  </div>
                  <div>
                    <h3 style={{ fontSize: "1.1rem", fontWeight: 800, margin: 0, color: "#fff" }}>{message || (isStudio ? "Generating AI Video..." : "Processing Video...")}</h3>
                    <span style={{ fontSize: "0.78rem", color: "#888" }}>{isStudio ? "Nova Studio Pipeline" : "Clipper Pipeline"}</span>
                  </div>
                </div>
                <div style={{ background: "rgba(255, 224, 0, 0.12)", border: "1px solid rgba(255, 224, 0, 0.3)", borderRadius: "999px", padding: "0.35rem 0.9rem" }}>
                  <span style={{ fontSize: "1rem", fontWeight: 900, color: "var(--accent)" }}>{progress}%</span>
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
                          background: isDone ? "#22c55e" : isActive ? (isStudio ? "#8b5cf6" : "var(--accent)") : "#09090c",
                          color: isDone || isActive ? "#000" : "#666",
                          border: `2px solid ${isDone ? "#22c55e" : isActive ? (isStudio ? "#8b5cf6" : "var(--accent)") : "rgba(255,255,255,0.1)"}`,
                          boxShadow: isActive ? `0 0 20px ${isStudio ? "rgba(139,92,246,0.5)" : "rgba(255, 224, 0, 0.5)"}` : isDone ? "0 0 15px rgba(34, 197, 94, 0.3)" : "none",
                          transition: "all 0.3s",
                        }}
                      >
                        {isDone ? <CheckCircle2 size={20} color="#000" /> : <IconComponent size={20} color={isActive ? "#000" : "#666"} />}
                      </div>
                      <span style={{ fontSize: "0.75rem", fontWeight: isActive ? 800 : 600, color: isActive ? (isStudio ? "#8b5cf6" : "var(--accent)") : isDone ? "#fff" : "#666", display: "block" }}>
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
    </div>
  );
}
