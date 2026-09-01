/**
 * Tauri helpers - safe to import in web build (no-ops when not in Tauri)
 */
export const isTauri = (): boolean => {
  return typeof window !== "undefined" && "__TAURI__" in window;
};

// Detect desktop and expose backend URL override
export const getDesktopBackendUrl = (): string => {
  if (isTauri()) {
    // In packaged Tauri app, backend sidecar runs on 127.0.0.1:8000
    // VITE_API_URL is already set to http://127.0.0.1:8000 at build time for Tauri
    return import.meta.env.VITE_API_URL || "http://127.0.0.1:8000";
  }
  return import.meta.env.VITE_API_URL || "";
};

// Optionally ping Tauri backend to check health
export const checkBackendHealth = async (url?: string): Promise<boolean> => {
  const base = url ?? getDesktopBackendUrl();
  const healthUrl = `${base}/health`.replace(/\/+health/, "/health");
  try {
    const r = await fetch(healthUrl, { signal: AbortSignal.timeout(2000) });
    return r.ok;
  } catch {
    return false;
  }
};
