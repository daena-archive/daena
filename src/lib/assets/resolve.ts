import { project } from "$lib/project/client";

const cache = new Map<string, string>(); // path -> blob (LRU, bounded)
const reverse = new Map<string, string>(); // blob -> path (persistent for denormalize)
const pending = new Map<string, Promise<string | null>>();
const activeRefs = new Map<string, number>(); // blob -> retain count
const negative = new Map<string, number>(); // path -> expiry ms
const MAX_BLOB_CACHE = 80;
const NEGATIVE_TTL_MS = 30_000;

function touch(path: string) {
  const url = cache.get(path);
  if (url !== undefined) {
    cache.delete(path);
    cache.set(path, url);
  }
}

function isActive(url: string): boolean {
  return (activeRefs.get(url) ?? 0) > 0;
}

export function retainAssetUrl(url: string): void {
  if (!url || !url.startsWith("blob:")) return;
  activeRefs.set(url, (activeRefs.get(url) ?? 0) + 1);
}

export function releaseAssetUrl(url: string): void {
  if (!url || !url.startsWith("blob:")) return;
  const cur = activeRefs.get(url) ?? 0;
  if (cur <= 1) activeRefs.delete(url);
  else activeRefs.set(url, cur - 1);
}

function evictIfNeeded() {
  while (cache.size > MAX_BLOB_CACHE) {
    let evicted = false;
    for (const [path, blob] of cache) {
      if (isActive(blob)) continue;
      cache.delete(path);
      try {
        URL.revokeObjectURL(blob);
      } catch {}
      // Keep reverse entry for denormalizeAssetHtml so existing editor HTML
      // that still contains this blob: URL can be rewritten to its assets/... path
      // on save even after eviction. The mapping is cleared only on revokeAll.
      evicted = true;
      break;
    }
    if (!evicted) break; // all remaining blobs are actively retained
  }
  if (negative.size > 200) {
    const now = Date.now();
    for (const [k, exp] of [...negative.entries()]) if (exp <= now) negative.delete(k);
  }
}

function decodePath(path: string): string {
  try {
    return decodeURIComponent(path);
  } catch {
    return path;
  }
}

/**
 * Resolve a portable `assets/...` path to an object URL backed by runtime bytes.
 * Returns null if not an asset path or fetch fails. Caches result for session.
 */
export async function resolveAssetSrc(path: string): Promise<string | null> {
  const trimmed = path.trim();
  if (!trimmed.startsWith("assets/")) return null;
  if (trimmed.length > 1024 || trimmed.includes("\0") || trimmed.includes("..")) return null;
  const decoded = decodePath(trimmed);
  if (decoded.includes("\0") || decoded.includes("..")) return null;
  const key = decoded;
  const negExp = negative.get(key);
  if (negExp !== undefined) {
    if (Date.now() < negExp) return null;
    negative.delete(key);
  }
  if (cache.has(key)) {
    touch(key);
    return cache.get(key)!;
  }
  if (pending.has(key)) return pending.get(key)!;

  const p = (async () => {
    // try decoded and original
    const candidates = [decoded, trimmed];
    // dedupe
    const uniq = [...new Set(candidates)];
    for (const cand of uniq) {
      try {
        const bytes = await project.readAssetBytesByPath(cand);
        if (!bytes || bytes.length === 0) continue;
        // need mime for blob — fetch asset meta first to get mime
        // but we can infer or fetch via getAssetByPath
        let mime = "application/octet-stream";
        try {
          const meta = await project.getAssetByPath(cand);
          if (meta?.mime_type) mime = meta.mime_type;
        } catch {}
        const blob = new Blob([Uint8Array.from(bytes)], { type: mime });
        const url = URL.createObjectURL(blob);
        cache.set(key, url);
        reverse.set(url, key);
        negative.delete(key);
        evictIfNeeded();
        return url;
      } catch {}
    }
    negative.set(key, Date.now() + NEGATIVE_TTL_MS);
    return null;
  })();
  pending.set(key, p);
  const result = await p;
  pending.delete(key);
  return result;
}

export function revokeAllResolvedAssetUrls() {
  for (const url of cache.values()) {
    try {
      URL.revokeObjectURL(url);
    } catch {}
  }
  for (const url of reverse.keys()) {
    try {
      URL.revokeObjectURL(url);
    } catch {}
  }
  cache.clear();
  reverse.clear();
  pending.clear();
  activeRefs.clear();
  negative.clear();
}

export function denormalizeAssetHtml(html: string): string {
  if (!html || !html.includes("blob:")) return html;
  let out = html;
  // reverse is the persistent superset (includes evicted blobs) — iterate first
  for (const [blob, path] of reverse.entries()) {
    if (out.includes(blob)) out = out.split(blob).join(path);
  }
  for (const [path, blob] of cache.entries()) {
    if (out.includes(blob)) out = out.split(blob).join(path);
  }
  return out;
}

export function isAssetPath(value: string): boolean {
  return value.trim().startsWith("assets/");
}
