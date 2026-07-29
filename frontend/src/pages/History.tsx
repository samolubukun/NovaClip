import { useState, useEffect } from "react";
import { Link } from "react-router-dom";
import { ArrowLeft, ExternalLink, Trash2, Search, Play, X, Video, Film } from "lucide-react";
import { toast } from "sonner";
import { motion, AnimatePresence } from "framer-motion";
import { api } from "@/lib/api";

interface TaskSummary {
  id: string;
  status: string;
  progress: number;
  source_url: string;
  source_title?: string;
  clips_count: number;
  created_at: string;
  completed_at?: string;
}

interface Clip {
  id: string;
  filename: string;
  hook_title?: string;
  duration: number;
  virality_score: number;
  start_time: string;
  end_time: string;
}

function timeAgo(iso: string) {
  const diff = Date.now() - new Date(iso).getTime();
  const mins = Math.floor(diff / 60000);
  if (mins < 1) return "just now";
  if (mins < 60) return `${mins}m ago`;
  const hours = Math.floor(mins / 60);
  if (hours < 24) return `${hours}h ago`;
  return `${Math.floor(hours / 24)}d ago`;
}

export default function History() {
  const [tasks, setTasks] = useState<TaskSummary[]>([]);
  const [firstClipMap, setFirstClipMap] = useState<Record<string, string>>({});
  const [loading, setLoading] = useState(true);
  const [search, setSearch] = useState("");
  const [selectedTask, setSelectedTask] = useState<{ id: string; title: string; clips: Clip[] } | null>(null);

  useEffect(() => {
    api.listTasks().then(async r => {
      const taskList = r.tasks || [];
      setTasks(taskList);
      setLoading(false);

      // Fetch first clip ID for completed tasks to render real video poster frame
      const map: Record<string, string> = {};
      for (const t of taskList) {
        if (t.status === "completed") {
          try {
            const details = await api.getTask(t.id);
            if (details.clips?.[0]?.id) {
              map[t.id] = details.clips[0].id;
            }
          } catch {}
        }
      }
      setFirstClipMap(map);
    }).catch(() => setLoading(false));
  }, []);

  const deleteTask = async (id: string, e: React.MouseEvent) => {
    e.stopPropagation();
    if (!confirm("Delete task and all its clips?")) return;
    await api.deleteTask(id);
    setTasks(t => t.filter(x => x.id !== id));
    toast.success("Task deleted");
  };

  const openPreview = async (taskId: string, title: string) => {
    try {
      const fullTask = await api.getTask(taskId);
      setSelectedTask({
        id: taskId,
        title: title || fullTask.source_title || fullTask.source_url,
        clips: fullTask.clips || [],
      });
    } catch (e) {
      toast.error("Failed to load task clips");
    }
  };

  const filteredTasks = tasks.filter(t =>
    (t.source_title || t.source_url).toLowerCase().includes(search.toLowerCase()) ||
    t.id.toLowerCase().includes(search.toLowerCase())
  );

  return (
    <div style={{ paddingTop: "64px", minHeight: "100vh", background: "#0b0b0e", color: "#fff" }}>
      <div className="container" style={{ maxWidth: "1200px", margin: "0 auto", padding: "2.5rem 1.5rem 4rem" }}>
        
        {/* Header Bar */}
        <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between", marginBottom: "2rem", flexWrap: "wrap", gap: "1rem" }}>
          <div style={{ display: "flex", alignItems: "center", gap: "1rem" }}>
            <Link to="/" className="btn btn-ghost btn-icon" style={{ background: "#16161c" }}><ArrowLeft size={18} /></Link>
            <div>
              <h1 style={{ fontSize: "1.6rem", fontWeight: 800, margin: 0 }}>Task & Clip History</h1>
              <p style={{ fontSize: "0.82rem", color: "#888", margin: 0 }}>View and preview all generated clip tasks</p>
            </div>
          </div>

          <div style={{ display: "flex", alignItems: "center", gap: "0.75rem" }}>
            {/* Daily Audio Overview (TTS Dashboard Briefing Button) */}
            <button
              onClick={() => {
                if ('speechSynthesis' in window) {
                  window.speechSynthesis.cancel();
                  const completed = tasks.filter(t => t.status === "completed").length;
                  const totalClips = tasks.reduce((sum, t) => sum + (t.clips_count || 0), 0);
                  const text = `NovaClip Daily Audio Briefing. You currently have ${tasks.length} total video tasks in your history dashboard. ${completed} tasks are fully completed, producing a total of ${totalClips} viral video clips ready for download. Have a productive day generating clips!`;
                  const msg = new SpeechSynthesisUtterance(text);
                  msg.rate = 1.0;
                  msg.pitch = 1.0;
                  window.speechSynthesis.speak(msg);
                  toast.success("Playing Daily Audio Briefing 🎙️");
                } else {
                  toast.error("Speech synthesis not supported in browser");
                }
              }}
              className="btn btn-secondary btn-sm"
              style={{
                background: "rgba(255, 224, 0, 0.12)",
                border: "1px solid rgba(255, 224, 0, 0.3)",
                color: "var(--accent)",
                fontWeight: 800,
                fontSize: "0.78rem",
                display: "flex",
                alignItems: "center",
                gap: "0.4rem",
                borderRadius: "10px",
                padding: "0.5rem 0.85rem",
                cursor: "pointer",
              }}
            >
              <Play size={14} /> Daily Audio Overview
            </button>

            {/* Search Input (Serivia Inspired) */}
            <div style={{ position: "relative", minWidth: "240px" }}>
            <Search size={16} style={{ position: "absolute", left: "0.85rem", top: "50%", transform: "translateY(-50%)", color: "#666" }} />
            <input
              type="text"
              placeholder="Search tasks or clips..."
              value={search}
              onChange={e => setSearch(e.target.value)}
              style={{
                width: "100%",
                padding: "0.55rem 0.85rem 0.55rem 2.4rem",
                background: "#16161c",
                border: "1px solid rgba(255,255,255,0.1)",
                borderRadius: "10px",
                color: "#fff",
                fontSize: "0.85rem",
              }}
            />
          </div>
        </div>

        {loading && (
          <div style={{ display: "flex", justifyContent: "center", paddingTop: "4rem" }}>
            <div className="spinner" style={{ width: 32, height: 32, borderColor: "var(--accent)" }} />
          </div>
        )}

        {!loading && filteredTasks.length === 0 && (
          <div style={{ textAlign: "center", paddingTop: "4rem", color: "#888" }}>
            <Film size={48} style={{ opacity: 0.3, marginBottom: "1rem" }} />
            <p style={{ fontSize: "1rem" }}>No tasks found.</p>
            <Link to="/" className="btn btn-primary" style={{ marginTop: "1rem", background: "var(--accent)", color: "#000", fontWeight: 700 }}>Create your first clip</Link>
          </div>
        )}

        {/* Serivia-inspired Task Grid */}
        <div style={{ display: "grid", gridTemplateColumns: "repeat(auto-fill, minmax(320px, 1fr))", gap: "1.25rem" }}>
          {filteredTasks.map((task, i) => (
            <motion.div
              key={task.id}
              initial={{ opacity: 0, y: 16 }}
              animate={{ opacity: 1, y: 0 }}
              transition={{ delay: i * 0.05 }}
              style={{
                background: "#131318",
                border: "1px solid rgba(255, 255, 255, 0.08)",
                borderRadius: "14px",
                overflow: "hidden",
                cursor: "pointer",
                transition: "all 0.2s",
                display: "flex",
                flexDirection: "column",
              }}
              onClick={() => openPreview(task.id, task.source_title || task.source_url)}
            >
              {/* Card Banner / Real Video Frame Preview */}
              <div style={{ position: "relative", height: "160px", background: "#050507", overflow: "hidden", display: "flex", alignItems: "center", justifyContent: "center" }}>
                {task.status === "completed" && firstClipMap[task.id] ? (
                  <video
                    src={`${api.clipFileUrl(task.id, firstClipMap[task.id])}#t=0.5`}
                    preload="metadata"
                    style={{ width: "100%", height: "100%", objectFit: "contain", pointerEvents: "none" }}
                  />
                ) : (
                  <Video size={36} color="var(--accent)" style={{ opacity: 0.4 }} />
                )}

                <div
                  style={{
                    position: "absolute", inset: 0, background: "rgba(0,0,0,0.3)",
                    display: "flex", alignItems: "center", justifyContent: "center",
                  }}
                >
                  <div style={{ width: 44, height: 44, borderRadius: "50%", background: "var(--accent)", display: "flex", alignItems: "center", justifyContent: "center", boxShadow: "0 0 16px rgba(255,224,0,0.4)" }}>
                    <Play size={20} fill="#000" color="#000" />
                  </div>
                </div>

                {/* Deep Solid Emerald Badge */}
                <span
                  style={{
                    position: "absolute", top: "0.75rem", right: "0.75rem",
                    fontSize: "0.7rem", fontWeight: 900, textTransform: "uppercase", padding: "0.25rem 0.75rem", borderRadius: "999px",
                    background: task.status === "completed" ? "#047857" : "#854d0e",
                    color: "#ffffff",
                    border: `1px solid ${task.status === "completed" ? "#10b981" : "#eab308"}`,
                    boxShadow: "0 4px 12px rgba(0,0,0,0.9)",
                    letterSpacing: "0.05em",
                  }}
                >
                  {task.status}
                </span>
              </div>

              {/* Card Body */}
              <div style={{ padding: "1.1rem", flex: 1, display: "flex", flexDirection: "column", justifyContent: "space-between" }}>
                <div>
                  <h3 style={{ fontSize: "0.95rem", fontWeight: 700, marginBottom: "0.4rem", overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap", color: "#fff" }}>
                    {task.source_title || task.source_url}
                  </h3>
                  <div style={{ display: "flex", gap: "0.75rem", fontSize: "0.75rem", color: "#888", marginBottom: "1rem" }}>
                    <span>{timeAgo(task.created_at)}</span>
                    <span>•</span>
                    <span style={{ color: "var(--accent)", fontWeight: 700 }}>{task.clips_count} clips</span>
                  </div>
                </div>

                <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between", borderTop: "1px solid rgba(255,255,255,0.06)", paddingTop: "0.75rem" }}>
                  <Link
                    to={`/task/${task.id}`}
                    onClick={(e) => e.stopPropagation()}
                    className="btn btn-secondary btn-sm"
                    style={{ fontSize: "0.75rem", display: "flex", alignItems: "center", gap: "0.3rem" }}
                  >
                    <ExternalLink size={12} /> Open Task Page
                  </Link>

                  <button
                    className="btn btn-ghost btn-icon btn-sm"
                    onClick={(e) => deleteTask(task.id, e)}
                    style={{ color: "#ef4444" }}
                    title="Delete task"
                  >
                    <Trash2 size={14} />
                  </button>
                </div>
              </div>
            </motion.div>
          ))}
        </div>
      </div>

      {/* Instant Video Clip Preview Modal (Serivia Inspired) */}
      <AnimatePresence>
        {selectedTask && (
          <div
            style={{
              position: "fixed", inset: 0, zIndex: 9999,
              background: "rgba(0,0,0,0.85)", backdropFilter: "blur(8px)",
              display: "flex", alignItems: "center", justifyContent: "center", padding: "1.5rem",
            }}
            onClick={() => setSelectedTask(null)}
          >
            <motion.div
              initial={{ opacity: 0, scale: 0.95 }}
              animate={{ opacity: 1, scale: 1 }}
              exit={{ opacity: 0, scale: 0.95 }}
              style={{
                background: "#121218",
                border: "1px solid rgba(255,255,255,0.12)",
                borderRadius: "16px",
                width: "100%",
                maxWidth: "750px",
                maxHeight: "85vh",
                overflowY: "auto",
                padding: "1.5rem",
                position: "relative",
              }}
              onClick={(e) => e.stopPropagation()}
            >
              <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between", marginBottom: "1.25rem" }}>
                <div>
                  <h2 style={{ fontSize: "1.1rem", fontWeight: 700, margin: 0, color: "#fff" }}>{selectedTask.title}</h2>
                  <span style={{ fontSize: "0.78rem", color: "var(--accent)" }}>{selectedTask.clips.length} Clips Generated</span>
                </div>
                <button className="btn btn-ghost btn-icon" onClick={() => setSelectedTask(null)}><X size={18} /></button>
              </div>

              {selectedTask.clips.length === 0 ? (
                <p style={{ color: "#888", fontSize: "0.9rem", textAlign: "center", padding: "2rem 0" }}>No clips available for this task.</p>
              ) : (
                <div style={{ display: "grid", gridTemplateColumns: "repeat(auto-fit, minmax(220px, 1fr))", gap: "1rem" }}>
                  {selectedTask.clips.map(clip => {
                    const fileUrl = api.clipFileUrl(selectedTask.id, clip.id);
                    return (
                      <div key={clip.id} style={{ background: "#09090c", borderRadius: "10px", overflow: "hidden", border: "1px solid rgba(255,255,255,0.08)" }}>
                        <video src={fileUrl} controls style={{ width: "100%", maxHeight: "200px", background: "#000", objectFit: "cover" }} />
                        <div style={{ padding: "0.75rem" }}>
                          <div style={{ fontSize: "0.8rem", fontWeight: 700, color: "#fff", marginBottom: "0.3rem" }}>{clip.hook_title || clip.filename}</div>
                          <div style={{ display: "flex", justifyContent: "space-between", fontSize: "0.72rem", color: "#aaa" }}>
                            <span>{clip.duration.toFixed(1)}s</span>
                            <span style={{ color: "var(--accent)", fontWeight: 700 }}>Score: {clip.virality_score}</span>
                          </div>
                        </div>
                      </div>
                    );
                  })}
                </div>
              )}
            </motion.div>
          </div>
        )}
      </AnimatePresence>
    </div>
  );
}
