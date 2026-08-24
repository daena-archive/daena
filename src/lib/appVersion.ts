let cached: string | null = null;
let inflight: Promise<string> | null = null;

declare const __APP_VERSION__: string | undefined;

export async function appVersion(): Promise<string> {
  if (cached !== null) return cached;
  if (inflight !== null) return inflight;
  inflight = (async () => {
    // 1. Dedicated Tauri command — single runtime source (CARGO_PKG_VERSION / tauri.conf.json)
    try {
      const { invoke } = await import("@tauri-apps/api/core");
      const v = await invoke<string>("app_version");
      if (v) {
        cached = v;
        return cached;
      }
    } catch {
      // not in Tauri (vite dev, tests)
    }
    // 2. Stock Tauri app plugin (fallback for older builds)
    try {
      const { getVersion } = await import("@tauri-apps/api/app");
      const v = await getVersion();
      if (v) {
        cached = v;
        return cached;
      }
    } catch {
      // ignore
    }
    // 3. Vite build-time define — web preview
    try {
      const fallback = typeof __APP_VERSION__ !== "undefined" ? __APP_VERSION__ : undefined;
      if (fallback) {
        cached = fallback;
        return cached;
      }
    } catch {
      // ignore
    }
    cached = "0.0.0";
    return cached;
  })();
  const result = await inflight;
  inflight = null;
  return result;
}

export function appVersionSyncFallback(): string {
  try {
    if (typeof __APP_VERSION__ !== "undefined" && __APP_VERSION__) return __APP_VERSION__;
  } catch {
    // ignore
  }
  return cached ?? "0.0.0";
}
