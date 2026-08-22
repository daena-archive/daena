<script lang="ts">
import { onDestroy } from "svelte";
import { safeSrc } from "./urls.ts";
import { resolveAssetSrc, retainAssetUrl, releaseAssetUrl } from "$lib/assets/resolve";

let {
  src,
  alt = "",
  width = "",
  height = "",
  title = "",
}: { src: string; alt?: string; width?: string; height?: string; title?: string } = $props();

let resolvedSrc = $state<string | null>(null);
let loading = $state(true);
let error = $state(false);
// svelte-ignore state_referenced_locally
let currentSrc = $state(src);
let retainedBlob: string | null = null;

function setRetained(url: string | null) {
  if (retainedBlob && retainedBlob !== url) {
    releaseAssetUrl(retainedBlob);
    retainedBlob = null;
  }
  if (url && url.startsWith("blob:") && url !== retainedBlob) {
    retainAssetUrl(url);
    retainedBlob = url;
  } else if (!url || !url.startsWith("blob:")) {
    // non-blob URLs are not tracked
    if (retainedBlob) {
      releaseAssetUrl(retainedBlob);
      retainedBlob = null;
    }
  }
}

$effect(() => {
  // react to src changes
  const raw = src;
  const safe = safeSrc(raw);
  if (!safe) {
    setRetained(null);
    resolvedSrc = null;
    loading = false;
    return;
  }
  if (!safe.startsWith("assets/")) {
    // external https or # — use directly
    setRetained(null);
    resolvedSrc = safe;
    loading = false;
    error = false;
    currentSrc = safe;
    return;
  }
  let cancelled = false;
  loading = true;
  error = false;
  currentSrc = safe;
  void resolveAssetSrc(safe)
    .then((blobUrl) => {
      if (cancelled) return;
      if (blobUrl) {
        setRetained(blobUrl);
        resolvedSrc = blobUrl;
      } else {
        setRetained(null);
        resolvedSrc = null;
        error = true;
      }
      loading = false;
    })
    .catch(() => {
      if (cancelled) return;
      setRetained(null);
      error = true;
      loading = false;
      resolvedSrc = null;
    });
  return () => {
    cancelled = true;
  };
});

onDestroy(() => {
  if (retainedBlob) {
    releaseAssetUrl(retainedBlob);
    retainedBlob = null;
  }
});
</script>

{#if resolvedSrc}
  <img
    src={resolvedSrc}
    {alt}
    title={title || undefined}
    width={width && /^\d+$/.test(width.trim()) ? width.trim() : undefined}
    height={height && /^\d+$/.test(height.trim()) ? height.trim() : undefined}
    loading="lazy"
    decoding="async"
    style={width || height
      ? `${width && /^\d+$/.test(width.trim()) ? `width:${width.trim()}px;` : ""}${height && /^\d+$/.test(height.trim()) ? `height:${height.trim()}px;` : "height:auto;"}`
      : undefined} />
{:else if loading}
  <span class="asset-image-placeholder" aria-label={`Loading ${alt}`}>Loading image…</span>
{:else if error}
  <span class="asset-image-error" role="img" aria-label={`Failed to load ${alt}`}
    >Image unavailable: {alt} — {currentSrc}</span>
{:else}
  <span>{alt}</span>
{/if}

<style>
.asset-image-placeholder {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  padding: 6px 10px;
  border: 1px dashed var(--line, #e4e1d8);
  border-radius: 6px;
  background: var(--surface-muted, #f4f2ec);
  color: var(--ink-faint, #aaa79d);
  font-size: 11px;
}
.asset-image-error {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  padding: 6px 10px;
  border: 1px solid #e8c0b8;
  border-radius: 6px;
  background: #fdf0ed;
  color: #8a3a2f;
  font-size: 11px;
  word-break: break-all;
}
img {
  max-width: 100%;
  height: auto;
  border-radius: 6px;
  border: 1px solid var(--line, #e4e1d8);
  display: block;
  margin: 0.6em 0;
}
</style>
