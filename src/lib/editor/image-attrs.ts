export function clampDim(v: string): string {
  const t = v.trim();
  if (t === "") return "";
  const n = Number(t);
  if (!Number.isFinite(n)) return "";
  return String(Math.max(16, Math.min(2000, Math.round(n))));
}

export function mimeTypeFor(filename: string): string {
  const ext = filename.split(".").pop()?.toLowerCase() ?? "";
  if (ext === "png") return "image/png";
  if (ext === "jpg" || ext === "jpeg") return "image/jpeg";
  if (ext === "gif") return "image/gif";
  if (ext === "webp") return "image/webp";
  if (ext === "svg") return "image/svg+xml";
  if (ext === "mp4") return "video/mp4";
  if (ext === "webm") return "video/webm";
  if (ext === "pdf") return "application/pdf";
  if (ext === "mp3") return "audio/mpeg";
  if (ext === "wav") return "audio/wav";
  return "application/octet-stream";
}
export function isImage(mime: string) {
  return mime.startsWith("image/");
}
export function isVideo(mime: string) {
  return mime.startsWith("video/");
}
export function formatSize(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  const kb = Math.round(bytes / 1024);
  if (kb < 1024) return `${kb} KB`;
  return `${(kb / 1024).toFixed(1)} MB`;
}
