const API_BASE = import.meta.env.VITE_API_URL || "";

export const api = {
  async createTask(payload: object) {
    const geminiKey = localStorage.getItem("novaclip_gemini_key");
    const deepgramKey = localStorage.getItem("novaclip_deepgram_key");
    const openrouterKey = localStorage.getItem("novaclip_openrouter_key");

    const fullPayload = {
      ...payload,
      ...(geminiKey ? { gemini_api_key: geminiKey } : {}),
      ...(deepgramKey ? { deepgram_api_key: deepgramKey } : {}),
      ...(openrouterKey ? { openrouter_api_key: openrouterKey } : {}),
    };

    const r = await fetch(`${API_BASE}/tasks`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(fullPayload),
    });
    if (!r.ok) throw new Error((await r.json()).error || r.statusText);
    return r.json();
  },

  async getTask(id: string) {
    const r = await fetch(`${API_BASE}/tasks/${id}`);
    if (!r.ok) throw new Error("Task not found");
    return r.json();
  },

  async listTasks() {
    const r = await fetch(`${API_BASE}/tasks`);
    if (!r.ok) throw new Error("Failed to fetch tasks");
    return r.json();
  },

  async deleteTask(id: string) {
    const r = await fetch(`${API_BASE}/tasks/${id}`, { method: "DELETE" });
    return r.ok;
  },

  async cancelTask(id: string) {
    const r = await fetch(`${API_BASE}/tasks/${id}/cancel`, { method: "POST" });
    return r.ok;
  },

  async resumeTask(id: string) {
    const r = await fetch(`${API_BASE}/tasks/${id}/resume`, { method: "POST" });
    if (!r.ok) throw new Error("Resume failed");
    return r.json();
  },

  async updateTask(id: string, payload: object) {
    const r = await fetch(`${API_BASE}/tasks/${id}`, {
      method: "PATCH",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(payload),
    });
    return r.json();
  },

  async applySettings(id: string, payload: object) {
    const r = await fetch(`${API_BASE}/tasks/${id}/settings`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(payload),
    });
    return r.json();
  },

  async deleteClip(taskId: string, clipId: string) {
    const r = await fetch(`${API_BASE}/tasks/${taskId}/clips/${clipId}`, { method: "DELETE" });
    return r.ok;
  },

  async trimClip(taskId: string, clipId: string, startOffset: number, endOffset: number) {
    const r = await fetch(`${API_BASE}/tasks/${taskId}/clips/${clipId}`, {
      method: "PATCH",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ start_offset: startOffset, end_offset: endOffset }),
    });
    return r.json();
  },

  async uploadVideo(file: File, onProgress?: (pct: number) => void) {
    return new Promise<{ video_path: string }>((resolve, reject) => {
      const xhr = new XMLHttpRequest();
      const fd = new FormData();
      fd.append("video", file);
      xhr.open("POST", `${API_BASE}/media/upload`);
      if (onProgress) xhr.upload.onprogress = (e) => onProgress(e.loaded / e.total * 100);
      xhr.onload = () => {
        if (xhr.status < 300) resolve(JSON.parse(xhr.responseText));
        else reject(new Error(JSON.parse(xhr.responseText).error));
      };
      xhr.onerror = () => reject(new Error("Upload failed"));
      xhr.send(fd);
    });
  },

  clipFileUrl(taskId: string, clipId: string) {
    return `${API_BASE}/tasks/${taskId}/clips/${clipId}/file`;
  },

  clipExportUrl(taskId: string, clipId: string, preset: string) {
    return `${API_BASE}/tasks/${taskId}/clips/${clipId}/export?preset=${preset}`;
  },

  async getCaptionTemplates() {
    const r = await fetch(`${API_BASE}/media/caption-templates`);
    return r.json();
  },

  async getBrollStatus() {
    const r = await fetch(`${API_BASE}/media/broll/status`);
    return r.json();
  },

  async aiEdit(taskId: string, clipIds: string[], instruction: string) {
    const geminiKey = localStorage.getItem("novaclip_gemini_key");
    const r = await fetch(`${API_BASE}/tasks/${taskId}/ai-edit`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        clip_ids: clipIds,
        instruction,
        ...(geminiKey ? { gemini_api_key: geminiKey } : {}),
      }),
    });
    if (!r.ok) throw new Error((await r.json()).error || r.statusText);
    return r.json();
  },

  async translateCaptions(taskId: string, clipId: string, language: string) {
    return this.aiEdit(taskId, [clipId], `translate captions to ${language}`);
  },

  async uploadWatermark(taskId: string, file: File) {
    const fd = new FormData();
    fd.append("watermark", file);
    const r = await fetch(`${API_BASE}/tasks/${taskId}/watermark`, {
      method: "POST",
      body: fd,
    });
    if (!r.ok) throw new Error((await r.json()).error || "Watermark upload failed");
    return r.json();
  },

  async aiPrompt(url: string, instruction: string) {
    const geminiKey = localStorage.getItem("novaclip_gemini_key");
    const r = await fetch(`${API_BASE}/tasks/ai-prompt`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ url, instruction, ...(geminiKey ? { gemini_api_key: geminiKey } : {}) }),
    });
    if (!r.ok) throw new Error((await r.json()).error || r.statusText);
    return r.json();
  },

  async aiChat(url: string, params: object, messages: { role: string; content: string }[]) {
    const geminiKey = localStorage.getItem("novaclip_gemini_key");
    const r = await fetch(`${API_BASE}/tasks/ai-prompt/chat`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ url, params, messages, ...(geminiKey ? { gemini_api_key: geminiKey } : {}) }),
    });
    if (!r.ok) throw new Error((await r.json()).error || r.statusText);
    return r.json();
  },

  async approveEditPlan(taskId: string, editPlan?: object) {
    const r = await fetch(`${API_BASE}/tasks/${taskId}/approve-edit-plan`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(editPlan ? { edit_plan: editPlan } : {}),
    });
    if (!r.ok) throw new Error((await r.json()).error || r.statusText);
    return r.json();
  },

  async replan(taskId: string, message: string) {
    const r = await fetch(`${API_BASE}/tasks/${taskId}/replan`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ message }),
    });
    if (!r.ok) throw new Error((await r.json()).error || r.statusText);
    return r.json();
  },

  async publishVideo(taskId: string, payload: { clip_id?: string; platforms: string[]; title?: string; description?: string }) {
    const uploadpostKey = localStorage.getItem("novaclip_uploadpost_key") || "";
    const r = await fetch(`${API_BASE}/tasks/${taskId}/publish`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ ...payload, ...(uploadpostKey ? { uploadpost_key: uploadpostKey } : {}) }),
    });
    if (!r.ok) throw new Error((await r.json()).error || r.statusText);
    return r.json();
  },
};
