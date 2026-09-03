import type { Asset } from "$lib/project/client";

export function mimeTypeFor(filename: string) {
  const extension = filename.split(".").pop()?.toLowerCase();
  return extension === "png"
    ? "image/png"
    : extension === "jpg" || extension === "jpeg"
      ? "image/jpeg"
      : extension === "gif"
        ? "image/gif"
        : extension === "webp"
          ? "image/webp"
          : extension === "mp4"
            ? "video/mp4"
            : extension === "webm"
              ? "video/webm"
              : "application/octet-stream";
}

export function canUseAsProfile(asset: Asset) {
  return ["image/png", "image/jpeg", "image/gif", "image/webp"].includes(asset.mime_type);
}
