export interface ProgressEvent {
  task_id: string;
  percent: number;
  message: string;
  status: string;
  event_type: string;
  clip_index?: number;
  total_clips?: number;
  hook_title?: string;
  virality_score?: number;
}

export function createTaskSSE(
  taskId: string,
  onProgress: (e: ProgressEvent) => void,
  onClipReady: (e: ProgressEvent) => void,
  onComplete: () => void,
  onError: (msg: string) => void,
): EventSource {
  const url = `/tasks/${taskId}/progress`;
  const es = new EventSource(url);

  es.addEventListener("progress", (e: MessageEvent) => {
    try {
      const data: ProgressEvent = JSON.parse(e.data);
      onProgress(data);
      if (data.status === "completed") { onComplete(); es.close(); }
      if (data.status === "error") { onError(data.message); es.close(); }
      if (data.status === "cancelled") { es.close(); }
    } catch {}
  });

  es.addEventListener("clip_ready", (e: MessageEvent) => {
    try { onClipReady(JSON.parse(e.data)); } catch {}
  });

  es.addEventListener("completed", () => { onComplete(); es.close(); });
  es.addEventListener("ping", () => {});

  es.onerror = () => {
    // SSE connection dropped; the task is likely done, so just close.
    es.close();
  };

  return es;
}
