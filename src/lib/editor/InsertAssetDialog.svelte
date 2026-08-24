<script lang="ts">
import { tick, onMount } from "svelte";
import {
  X,
  Search as SearchIcon,
  Image as ImageIcon,
  File as FileIcon,
  Upload,
  Link as LinkIcon,
  Unlink as UnlinkIcon,
  Info as InfoIcon,
  CircleCheck as CircleCheckIcon,
  Trash2 as Trash2Icon,
  RefreshCw as RefreshCwIcon,
  TriangleAlert as TriangleAlertIcon,
  TextAlignStart as AlignLeftIcon,
  TextAlignCenter as AlignCenterIcon,
  TextAlignEnd as AlignRightIcon,
} from "@lucide/svelte";
import type { Asset, Entity } from "$lib/project/client";
import { project } from "$lib/project/client";

let {
  open = false,
  entityId = null as string | null,
  entities = [] as Entity[],
  defaultNamespace = null as string | null,
  onInsert,
  onCancel,
  mode = "insert" as "insert" | "replace",
  initialAlt = "",
  initialTitle = "",
  initialWidth = "",
  initialHeight = "",
  initialSrc = "",
  initialAlign = "" as "" | "left" | "center" | "right",
}: {
  open: boolean;
  entityId?: string | null;
  entities?: Entity[];
  defaultNamespace?: string | null;
  onInsert: (
    asset: Asset | null,
    meta?: { alt: string; title: string; width: string; height: string; align: "" | "left" | "center" | "right" },
  ) => void;
  onCancel: () => void;
  mode?: "insert" | "replace";
  initialAlt?: string;
  initialTitle?: string;
  initialWidth?: string;
  initialHeight?: string;
  initialSrc?: string;
  initialAlign?: "" | "left" | "center" | "right";
} = $props();

type Tab = "mine" | "shared" | "upload";
let activeTab = $state<Tab>("mine");
let myAssets = $state<Asset[]>([]);
let sharedAssets = $state<Asset[]>([]);
let loadingMy = $state(false);
let loadingShared = $state(false);
let myError = $state("");
let sharedError = $state("");
let query = $state("");
let selectedId = $state<string | null>(null);
let previewUrl = $state("");
let previewError = $state(false);
let lastFocused: Element | null = null;
let dialogElement = $state<HTMLDivElement | null>(null);
let searchInput = $state<HTMLInputElement | null>(null);
let wasOpen = $state(false);

// upload state
let uploading = $state(false);
let uploadError = $state("");
let shareNew = $state(false);
let pickedFileLabel = $state("");
let uploadedAsset = $state<Asset | null>(null);
let uploadedPreviewUrl = $state("");
let uploadedPreviewError = $state(false);

import { resolveAssetSrc } from "$lib/assets/resolve";

// image edit state (for alt/dim before insert or when replacing)
let draftAlt = $state("");
let draftTitle = $state("");
let draftWidth = $state("");
let draftHeight = $state("");
let draftAlign = $state<"" | "left" | "center" | "right">("");
let draftTitleCustom = $state(false);
let draftPreserveAspect = $state(true);
let draftNaturalW = $state(0);
let draftNaturalH = $state(0);
let draftNaturalCache = new Map<string, { w: number; h: number }>();

let isWidthOversized = $derived(
  draftWidth !== "" && draftNaturalW > 0 && /^\d+$/.test(draftWidth.trim()) && Number(draftWidth) > draftNaturalW,
);
let isHeightOversized = $derived(
  draftHeight !== "" && draftNaturalH > 0 && /^\d+$/.test(draftHeight.trim()) && Number(draftHeight) > draftNaturalH,
);
let isOversized = $derived(isWidthOversized || isHeightOversized);

function clampDim(v: string): string {
  const t = v.trim();
  if (t === "") return "";
  const n = Number(t);
  if (!Number.isFinite(n)) return "";
  return String(Math.max(16, Math.min(2000, Math.round(n))));
}
function probeNaturalForDialog(src: string, preview: string) {
  const cacheKey = src || preview;
  const cached = draftNaturalCache.get(cacheKey);
  if (cached) {
    draftNaturalW = cached.w;
    draftNaturalH = cached.h;
    return;
  }
  const url = preview || src;
  if (!url) return;
  // Never assign a raw `assets/...` portable path to `img.src` — the webview
  // would issue `GET /assets/...` and log `[404] GET /assets/...` in console.
  // Resolve to a blob: URL first; external http/blob URLs can be probed directly.
  const doProbe = (probeUrl: string) => {
    const img = new window.Image();
    img.onload = () => {
      const w = (img as HTMLImageElement).naturalWidth;
      const h = (img as HTMLImageElement).naturalHeight;
      if (w && h) {
        draftNaturalCache.set(cacheKey, { w, h });
        draftNaturalW = w;
        draftNaturalH = h;
      }
    };
    img.onerror = () => {};
    img.src = probeUrl;
  };
  if (url.startsWith("assets/")) {
    void resolveAssetSrc(url).then((blob) => {
      if (blob) doProbe(blob);
    });
    return;
  }
  if (url.startsWith("blob:") || /^https?:/i.test(url) || url.startsWith("data:")) {
    doProbe(url);
    return;
  }
  // Fallback: unknown scheme — do not probe to avoid stray GET
}
function updateDraftAlt(v: string) {
  draftAlt = v;
  if (!draftTitleCustom) draftTitle = v;
}
function updateDraftTitle(v: string) {
  draftTitle = v;
  draftTitleCustom = v !== draftAlt;
}
function updateDraftWidth(v: string) {
  const clamped = clampDim(v);
  if (clamped === "" && v.trim() !== "") {
    draftWidth = v;
    return;
  }
  draftWidth = clamped === "" ? "" : clamped;
  if (draftWidth === "") return;
  if (draftPreserveAspect && draftNaturalW && draftNaturalH) {
    const wNum = Number(draftWidth);
    if (Number.isFinite(wNum) && wNum > 0) {
      const hNum = Math.round((wNum * draftNaturalH) / draftNaturalW);
      draftHeight = String(Math.max(16, Math.min(2000, hNum)));
    }
  }
}
function updateDraftHeight(v: string) {
  const clamped = clampDim(v);
  if (clamped === "" && v.trim() !== "") {
    draftHeight = v;
    return;
  }
  draftHeight = clamped === "" ? "" : clamped;
  if (draftHeight === "") return;
  if (draftPreserveAspect && draftNaturalW && draftNaturalH) {
    const hNum = Number(draftHeight);
    if (Number.isFinite(hNum) && hNum > 0) {
      const wNum = Math.round((hNum * draftNaturalW) / draftNaturalH);
      draftWidth = String(Math.max(16, Math.min(2000, wNum)));
    }
  }
}
function clearDraftDims() {
  draftWidth = "";
  draftHeight = "";
}
function applyDraftPreset(preset: string) {
  if (preset === "S") {
    draftWidth = "320";
    if (draftPreserveAspect && draftNaturalW && draftNaturalH) {
      draftHeight = String(Math.round((320 * draftNaturalH) / draftNaturalW));
    }
    return;
  }
  if (preset === "M") {
    draftWidth = "640";
    if (draftPreserveAspect && draftNaturalW && draftNaturalH) {
      draftHeight = String(Math.round((640 * draftNaturalH) / draftNaturalW));
    }
    return;
  }
  if (preset === "L") {
    draftWidth = "960";
    if (draftPreserveAspect && draftNaturalW && draftNaturalH) {
      draftHeight = String(Math.round((960 * draftNaturalH) / draftNaturalW));
    }
    return;
  }
  if (preset === "Original") {
    if (draftNaturalW && draftNaturalH) {
      draftWidth = String(draftNaturalW);
      draftHeight = String(draftNaturalH);
    }
    return;
  }
  if (preset === "Full") {
    clearDraftDims();
    return;
  }
}

function mimeTypeFor(filename: string): string {
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
function isImage(mime: string) {
  return mime.startsWith("image/");
}
function isVideo(mime: string) {
  return mime.startsWith("video/");
}
function formatSize(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  const kb = Math.round(bytes / 1024);
  if (kb < 1024) return `${kb} KB`;
  return `${(kb / 1024).toFixed(1)} MB`;
}
function entityName(id: string): string {
  return entities.find((e) => e.id === id)?.name ?? id.slice(0, 8);
}
function filtered(list: Asset[]): Asset[] {
  const q = query.trim().toLowerCase();
  if (!q) return list;
  return list.filter((a) => `${a.filename} ${a.mime_type}`.toLowerCase().includes(q));
}
let filteredMy = $derived(filtered(myAssets));
let filteredShared = $derived(filtered(sharedAssets));
let selectedAsset = $derived(
  (activeTab === "mine" ? myAssets : activeTab === "shared" ? sharedAssets : []).find((a) => a.id === selectedId) ??
    null,
);

async function loadMy() {
  if (!entityId) {
    myAssets = [];
    return;
  }
  loadingMy = true;
  myError = "";
  try {
    const all = await project.listAssets(entityId);
    // If defaultNamespace provided, prefer that namespace but show all for completeness?
    // Show all for editor insertion, but sort: own namespace first
    myAssets = all.sort((a, b) => {
      const an = a.namespace === defaultNamespace ? -1 : 1;
      const bn = b.namespace === defaultNamespace ? -1 : 1;
      if (an !== bn) return an - bn;
      return a.filename.localeCompare(b.filename);
    });
  } catch (e) {
    myError = e instanceof Error ? e.message : String(e);
  } finally {
    loadingMy = false;
  }
}
async function loadShared() {
  loadingShared = true;
  sharedError = "";
  try {
    const all = await project.listSharedAssets();
    sharedAssets = all.sort((a, b) => b.created_at.localeCompare(a.created_at));
  } catch (e) {
    sharedError = e instanceof Error ? e.message : String(e);
  } finally {
    loadingShared = false;
  }
}

$effect(() => {
  if (!open) {
    wasOpen = false;
    return;
  }
  if (!wasOpen) {
    wasOpen = true;
    lastFocused = document.activeElement;
    query = "";
    selectedId = null;
    previewUrl = "";
    previewError = false;
    pickedFileLabel = "";
    uploadError = "";
    uploadedAsset = null;
    uploadedPreviewUrl = "";
    uploadedPreviewError = false;
    // init image edit drafts from props (for replace mode or new insert)
    draftAlt = initialAlt;
    draftTitle = initialTitle;
    draftWidth = /^\d+$/.test(initialWidth) ? initialWidth : "";
    draftHeight = /^\d+$/.test(initialHeight) ? initialHeight : "";
    draftAlign = initialAlign === "center" || initialAlign === "right" || initialAlign === "left" ? initialAlign : "";
    draftTitleCustom = !!(initialTitle && initialTitle !== initialAlt);
    draftNaturalW = 0;
    draftNaturalH = 0;
    if (initialSrc) probeNaturalForDialog(initialSrc, "");
    try {
      const saved = localStorage.getItem("daena:imagePreserveAspect");
      if (saved !== null) draftPreserveAspect = saved === "true";
    } catch {}
    // default tab: mine if has entityId else shared
    activeTab = entityId ? "mine" : "shared";
    void tick().then(() => searchInput?.focus());
    void loadMy();
    void loadShared();
  }
});

// when a new image asset is selected, initialize alt/title and probe natural size
$effect(() => {
  const a = selectedAsset;
  if (!open || !a || !isImage(a.mime_type)) return;
  // if drafts were empty (new insert) or still equal to initial, default to filename
  if (!draftAlt || draftAlt === initialAlt) {
    draftAlt = a.filename;
    if (!draftTitleCustom) draftTitle = draftAlt;
  }
  // probe natural from previewUrl will happen separately, but also allow probing after preview loads
});

// when a newly uploaded image is staged, initialize alt/title similarly
$effect(() => {
  const a = uploadedAsset;
  if (!open || !a || !isImage(a.mime_type) || activeTab !== "upload") return;
  if (!draftAlt || draftAlt === initialAlt) {
    draftAlt = a.filename;
    if (!draftTitleCustom) draftTitle = draftAlt;
  }
});

$effect(() => {
  if (previewUrl && (selectedAsset ? isImage(selectedAsset.mime_type) : !!initialSrc)) {
    probeNaturalForDialog(initialSrc, previewUrl);
  }
});

$effect(() => {
  if (uploadedPreviewUrl && uploadedAsset && isImage(uploadedAsset.mime_type)) {
    probeNaturalForDialog("", uploadedPreviewUrl);
  }
});

$effect(() => {
  if (typeof window !== "undefined") {
    try {
      localStorage.setItem("daena:imagePreserveAspect", String(draftPreserveAspect));
    } catch {}
  }
});

$effect(() => {
  if (!open && lastFocused) {
    if (lastFocused instanceof HTMLElement && lastFocused.isConnected) lastFocused.focus();
    lastFocused = null;
  }
});

// preview blob for selected asset
$effect(() => {
  const a = selectedAsset;
  let disposed = false;
  let objectUrl = "";
  previewUrl = "";
  previewError = false;
  if (!a) return;
  const shouldPreview = isImage(a.mime_type) || isVideo(a.mime_type);
  if (!shouldPreview) return;
  void project
    .readAssetBytes(a.id)
    .then((bytes) => {
      if (disposed) return;
      try {
        const blob = new Blob([Uint8Array.from(bytes)], { type: a.mime_type });
        objectUrl = URL.createObjectURL(blob);
        if (disposed) {
          URL.revokeObjectURL(objectUrl);
          return;
        }
        previewUrl = objectUrl;
      } catch {
        if (!disposed) previewError = true;
      }
    })
    .catch(() => {
      if (!disposed) previewError = true;
    });
  return () => {
    disposed = true;
    if (objectUrl) URL.revokeObjectURL(objectUrl);
  };
});

// preview blob for staged upload
$effect(() => {
  const a = uploadedAsset;
  let disposed = false;
  let objectUrl = "";
  uploadedPreviewUrl = "";
  uploadedPreviewError = false;
  if (!a) return;
  if (activeTab !== "upload") return;
  const shouldPreview = isImage(a.mime_type) || isVideo(a.mime_type);
  if (!shouldPreview) return;
  void project
    .readAssetBytes(a.id)
    .then((bytes) => {
      if (disposed) return;
      try {
        const blob = new Blob([Uint8Array.from(bytes)], { type: a.mime_type });
        objectUrl = URL.createObjectURL(blob);
        if (disposed) {
          URL.revokeObjectURL(objectUrl);
          return;
        }
        uploadedPreviewUrl = objectUrl;
      } catch {
        if (!disposed) uploadedPreviewError = true;
      }
    })
    .catch(() => {
      if (!disposed) uploadedPreviewError = true;
    });
  return () => {
    disposed = true;
    if (objectUrl) URL.revokeObjectURL(objectUrl);
  };
});

function imageMeta(): {
  alt: string;
  title: string;
  width: string;
  height: string;
  align: "" | "left" | "center" | "right";
} {
  const alt = draftAlt.trim();
  const title = draftTitleCustom ? draftTitle.trim() : alt;
  const w = /^\d+$/.test(draftWidth.trim()) ? draftWidth.trim() : "";
  const h = /^\d+$/.test(draftHeight.trim()) ? draftHeight.trim() : "";
  return { alt, title, width: w, height: h, align: draftAlign };
}
function isImageContext(): boolean {
  if (activeTab === "upload" && uploadedAsset) return isImage(uploadedAsset.mime_type);
  if (selectedAsset) return isImage(selectedAsset.mime_type);
  if (mode === "replace" && initialSrc) return true;
  return false;
}
function canConfirm(): boolean {
  if (activeTab === "upload") {
    if (uploadedAsset) return true;
    if (mode === "replace" && initialSrc) return true;
    return false;
  }
  if (selectedAsset) return true;
  // in replace mode we allow saving meta edits without picking new asset
  if (mode === "replace" && initialSrc) return true;
  return false;
}
async function confirmUploadInsert() {
  const asset = uploadedAsset;
  if (!asset) return;
  const mime = asset.mime_type;
  let meta:
    { alt: string; title: string; width: string; height: string; align: "" | "left" | "center" | "right" } | undefined =
    undefined;
  if (isImage(mime)) {
    meta = imageMeta();
    if (!meta.alt) meta.alt = asset.filename;
  }
  // apply share scope if requested
  let finalAsset: Asset = asset;
  if (shareNew && asset.reference_scope !== "project") {
    try {
      finalAsset = await project.updateAssetMetadata(asset.id, { referenceScope: "project" }, asset.revision);
      // keep local lists in sync
      myAssets = myAssets.map((a) => (a.id === asset.id ? finalAsset : a));
      sharedAssets = [finalAsset, ...sharedAssets.filter((a) => a.id !== finalAsset.id)];
      uploadedAsset = finalAsset;
    } catch {
      // keep original
    }
  }
  onInsert(finalAsset, meta);
}

function confirmInsert() {
  if (activeTab === "upload" && uploadedAsset) {
    void confirmUploadInsert();
    return;
  }
  if (selectedAsset) {
    const meta = isImage(selectedAsset.mime_type) ? imageMeta() : undefined;
    onInsert(selectedAsset, meta);
  } else if (mode === "replace" && initialSrc) {
    // save meta changes without changing asset
    onInsert(null, imageMeta());
  }
}

function handleKeydown(event: KeyboardEvent) {
  if (event.key === "Escape") {
    event.preventDefault();
    event.stopPropagation();
    onCancel();
  } else if (event.key === "Enter" && canConfirm()) {
    // avoid submitting when typing search or image fields
    const target = event.target as HTMLElement | null;
    if (target?.tagName === "INPUT" || target?.tagName === "TEXTAREA" || target?.closest("button")) return;
    event.preventDefault();
    confirmInsert();
  }
}

function selectAsset(id: string) {
  selectedId = id;
}

async function handlePickAndStageUpload() {
  if (!entityId) {
    uploadError = "No entry selected — cannot attach file.";
    return;
  }
  uploading = true;
  uploadError = "";
  try {
    const selection = await project.pickFile();
    const source = typeof selection === "string" ? selection : null;
    if (!source) {
      uploading = false;
      return;
    }
    pickedFileLabel = source.split(/[\\/]/).pop() ?? "asset";
    const filename = pickedFileLabel;
    const namespace = defaultNamespace ?? "lore";
    const mime = mimeTypeFor(filename);
    const asset = await project.registerAssetFile({
      entity_id: entityId,
      namespace,
      source_path: source,
      filename,
      mime_type: mime,
    });
    // stage for confirmation instead of immediate insert
    uploadedAsset = asset;
    uploadedPreviewError = false;
    // initialize image meta defaults for staged upload — always reset to new file's name/dims
    if (isImage(mime)) {
      draftAlt = filename;
      draftTitle = filename;
      draftTitleCustom = false;
      // reset dimensions for new image; natural size will be probed from preview
      draftWidth = "";
      draftHeight = "";
      draftNaturalW = 0;
      draftNaturalH = 0;
    } else {
      // non-image: clear image-specific drafts
      draftAlt = "";
      draftTitle = "";
      draftTitleCustom = false;
      draftWidth = "";
      draftHeight = "";
    }
    // also refresh local lists for future picks without waiting for explicit reload
    myAssets = [...myAssets, asset];
    if (asset.reference_scope === "project") sharedAssets = [asset, ...sharedAssets];
    // probe will happen via uploadedPreviewUrl effect; also try immediate probe if possible
  } catch (e) {
    uploadError = e instanceof Error ? e.message : String(e);
  } finally {
    uploading = false;
  }
}

function clearStagedUpload() {
  uploadedAsset = null;
  uploadedPreviewUrl = "";
  uploadedPreviewError = false;
  pickedFileLabel = "";
  // reset image drafts to initial (replace-mode) defaults or empty
  draftAlt = initialAlt;
  draftTitle = initialTitle;
  draftTitleCustom = !!(initialTitle && initialTitle !== initialAlt);
  draftWidth = /^\d+$/.test(initialWidth) ? initialWidth : "";
  draftHeight = /^\d+$/.test(initialHeight) ? initialHeight : "";
  draftAlign = initialAlign === "center" || initialAlign === "right" || initialAlign === "left" ? initialAlign : "";
  draftNaturalW = 0;
  draftNaturalH = 0;
  if (initialSrc) probeNaturalForDialog(initialSrc, "");
}

function focusableElements(): HTMLElement[] {
  return Array.from(
    dialogElement?.querySelectorAll<HTMLElement>(
      'button:not([disabled]), input:not([disabled]), [tabindex]:not([tabindex="-1"])',
    ) ?? [],
  );
}
onMount(() => {
  const handleTab = (event: KeyboardEvent) => {
    if (!open) return;
    if (event.key !== "Tab") return;
    const focusable = focusableElements();
    if (focusable.length === 0) return;
    const first = focusable[0];
    const last = focusable[focusable.length - 1];
    if (event.shiftKey && document.activeElement === first) {
      event.preventDefault();
      last.focus();
    } else if (!event.shiftKey && document.activeElement === last) {
      event.preventDefault();
      first.focus();
    }
  };
  window.addEventListener("keydown", handleTab, true);
  return () => window.removeEventListener("keydown", handleTab, true);
});
</script>

{#if open}
  <div class="insert-asset-backdrop" role="presentation" onclick={onCancel}>
    <div
      bind:this={dialogElement}
      class="insert-asset-dialog"
      role="dialog"
      aria-modal="true"
      aria-labelledby="insert-asset-title"
      tabindex="-1"
      onclick={(e) => e.stopPropagation()}
      onkeydown={handleKeydown}>
      <header>
        <div>
          <span class="kicker">{mode === "replace" ? "EDIT" : "INSERT"}</span>
          <h2 id="insert-asset-title">{mode === "replace" ? "Edit image" : "Insert image or file"}</h2>
          <p class="subtitle">
            {mode === "replace"
              ? "Update image details or choose a different file."
              : "Choose from this entry, shared files, or upload a new file."}
          </p>
        </div>
        <button type="button" class="close-btn" aria-label="Close" onclick={onCancel}
          ><X size={16} strokeWidth={1.8} /></button>
      </header>

      <div class="tabs" role="tablist" aria-label="Asset source">
        <button
          type="button"
          role="tab"
          aria-selected={activeTab === "mine"}
          class:active={activeTab === "mine"}
          disabled={!entityId}
          title={!entityId ? "No entry selected" : ""}
          onclick={() => (activeTab = "mine")}
          >This entry {#if myAssets.length}· {myAssets.length}{/if}</button>
        <button
          type="button"
          role="tab"
          aria-selected={activeTab === "shared"}
          class:active={activeTab === "shared"}
          onclick={() => (activeTab = "shared")}
          >Shared files {#if sharedAssets.length}· {sharedAssets.length}{/if}</button>
        <button
          type="button"
          role="tab"
          aria-selected={activeTab === "upload"}
          class:active={activeTab === "upload"}
          onclick={() => (activeTab = "upload")}>Upload new</button>
      </div>

      {#if activeTab !== "upload"}
        <label class="search-row">
          <SearchIcon size={14} strokeWidth={1.8} />
          <input
            bind:this={searchInput}
            type="text"
            placeholder={activeTab === "mine" ? "Search this entry…" : "Search shared files…"}
            bind:value={query} />
          {#if query}<button type="button" class="clear" onclick={() => (query = "")}><X size={12} /></button>{/if}
        </label>
      {/if}

      {#if activeTab === "mine"}
        <div class="assets-list" role="listbox" aria-label="Attachments for this entry">
          {#if loadingMy}
            <p class="empty">Loading…</p>
          {:else if myError}
            <p class="error">{myError}</p>
          {:else if filteredMy.length === 0}
            <p class="empty">
              {entityId ? "No attachments on this entry. Try Shared files or Upload new." : "No entry selected."}
            </p>
          {:else}
            {#each filteredMy as asset (asset.id)}
              <button
                type="button"
                role="option"
                aria-selected={selectedId === asset.id}
                class="asset-row"
                class:selected={selectedId === asset.id}
                onclick={() => selectAsset(asset.id)}>
                <span class="thumb">
                  {#if isImage(asset.mime_type)}<ImageIcon size={16} />{:else}<FileIcon size={16} />{/if}
                </span>
                <span class="meta">
                  <strong>{asset.filename}</strong>
                  <small
                    >{formatSize(asset.size)} · {asset.mime_type}
                    {#if asset.reference_scope === "project"}· shared{/if}</small>
                </span>
                <small class="role">{asset.role}</small>
              </button>
            {/each}
          {/if}
        </div>
      {:else if activeTab === "shared"}
        <div class="assets-list" role="listbox" aria-label="Shared files">
          {#if loadingShared}
            <p class="empty">Loading…</p>
          {:else if sharedError}
            <p class="error">{sharedError}</p>
          {:else if filteredShared.length === 0}
            <p class="empty">No shared files yet. Upload a file and mark it as shared to reuse across entries.</p>
          {:else}
            {#each filteredShared as asset (asset.id)}
              <button
                type="button"
                role="option"
                aria-selected={selectedId === asset.id}
                class="asset-row"
                class:selected={selectedId === asset.id}
                onclick={() => selectAsset(asset.id)}>
                <span class="thumb">
                  {#if isImage(asset.mime_type)}<ImageIcon size={16} />{:else}<FileIcon size={16} />{/if}
                </span>
                <span class="meta">
                  <strong>{asset.filename}</strong>
                  <small>{formatSize(asset.size)} · {asset.mime_type} · {entityName(asset.entity_id)}</small>
                </span>
                <small class="role">shared</small>
              </button>
            {/each}
          {/if}
        </div>
      {:else}
        <div class="upload-pane" class:upload-pane--staged={!!uploadedAsset} class:upload-pane--empty={!uploadedAsset}>
          {#if !uploadedAsset}
            <div class="drop-hint">
              <Upload size={20} strokeWidth={1.6} />
              <p><strong>Upload a new file to this entry</strong></p>
              <p class="muted">Images insert as images, other files as links.</p>
            </div>
            <label class="share-row">
              <input type="checkbox" bind:checked={shareNew} />
              <span>Share with other entries <small>Others can pick this file from Shared files.</small></span>
            </label>
            {#if uploadError}<p class="error">{uploadError}</p>{/if}
            {#if pickedFileLabel}<p class="picked">Last picked: {pickedFileLabel}</p>{/if}
            <button
              type="button"
              class="primary upload-btn"
              disabled={uploading || !entityId}
              onclick={handlePickAndStageUpload}>
              {#if uploading}Uploading…{:else}Choose file{/if}
            </button>
          {:else}
            <div class="staged-header">
              <span class="staged-badge"><CircleCheckIcon size={13} strokeWidth={2} /> Staged</span>
              <span class="staged-hint">Review details → Insert in footer</span>
            </div>
            <div class="staged-card">
              <div class="staged-thumb">
                {#if uploadedPreviewUrl && isImage(uploadedAsset.mime_type)}
                  <img src={uploadedPreviewUrl} alt={uploadedAsset.filename} />
                {:else if uploadedPreviewUrl && isVideo(uploadedAsset.mime_type)}
                  <!-- svelte-ignore a11y_media_has_caption -->
                  <video src={uploadedPreviewUrl} controls preload="metadata"></video>
                {:else if isImage(uploadedAsset.mime_type) && uploadedPreviewError}
                  <div class="staged-fallback staged-fallback--error">Preview unavailable</div>
                {:else}
                  <div class="staged-fallback">
                    {#if isImage(uploadedAsset.mime_type)}<ImageIcon size={20} />{:else}<FileIcon size={20} />{/if}
                    <span>{uploadedAsset.mime_type}</span>
                  </div>
                {/if}
              </div>
              <div class="staged-meta">
                <strong class="staged-filename" title={uploadedAsset.filename}>{uploadedAsset.filename}</strong>
                <span class="staged-path" title={uploadedAsset.path}>{uploadedAsset.path}</span>
                <span class="staged-details">
                  <span class="staged-mime">{uploadedAsset.mime_type}</span>
                  <span class="staged-dot">·</span>
                  <span>{formatSize(uploadedAsset.size)}</span>
                  {#if isImage(uploadedAsset.mime_type)}<span class="staged-dot">·</span><span
                      class="staged-image-badge">Image</span
                    >{/if}
                </span>
                {#if shareNew}
                  <span class="staged-share-hint"
                    ><InfoIcon size={11} strokeWidth={1.8} /> Will be shared on insert</span>
                {/if}
              </div>
            </div>
            <label class="share-row">
              <input type="checkbox" bind:checked={shareNew} />
              <span>Share with other entries <small>Others can pick this file from Shared files.</small></span>
            </label>
            {#if uploadError}<p class="error">{uploadError}</p>{/if}
            <div class="upload-actions upload-actions--staged">
              <button type="button" class="quiet" onclick={clearStagedUpload}
                ><Trash2Icon size={12} strokeWidth={1.8} /> Clear</button>
              <button
                type="button"
                class="secondary"
                disabled={uploading || !entityId}
                onclick={handlePickAndStageUpload}>
                <RefreshCwIcon size={12} strokeWidth={1.8} />
                {#if uploading}Uploading…{:else}Choose different file{/if}
              </button>
            </div>
          {/if}
          {#if !entityId}<p class="empty">Select an entry before uploading.</p>{/if}
        </div>
      {/if}

      {#if selectedAsset && activeTab !== "upload"}
        <div class="preview">
          {#if previewUrl && isImage(selectedAsset.mime_type)}
            <img src={previewUrl} alt={selectedAsset.filename} />
          {:else if previewUrl && isVideo(selectedAsset.mime_type)}
            <!-- svelte-ignore a11y_media_has_caption -->
            <video src={previewUrl} controls preload="metadata"></video>
          {:else if isImage(selectedAsset.mime_type) && previewError}
            <div class="preview-fallback">Preview unavailable</div>
          {:else}
            <div class="preview-fallback">
              <FileIcon size={20} /><span>{selectedAsset.mime_type} · {formatSize(selectedAsset.size)}</span>
            </div>
          {/if}
          <div class="preview-meta">
            <strong>{selectedAsset.filename}</strong>
            <small>{selectedAsset.path}</small>
          </div>
        </div>
      {/if}

      {#if isImageContext()}
        <div class="image-edit-section">
          <div class="image-edit-header">
            <span class="image-edit-header-title"><ImageIcon size={14} strokeWidth={1.8} /> Image details</span>
            <span class="image-edit-header-hint">Applied on insert</span>
          </div>

          <label class="image-edit-field">
            <span class="field-label"
              >Alt text <span class="field-badge field-badge--required">accessibility</span></span>
            <input
              type="text"
              placeholder="Describe the image for screen readers"
              value={draftAlt}
              oninput={(e) => updateDraftAlt((e.target as HTMLInputElement).value)} />
            <span class="field-hint">Read by screen readers and shown if the image fails to load.</span>
          </label>

          <label class="image-edit-field">
            <span class="field-label">Hover title <span class="field-badge field-badge--optional">optional</span></span>
            <input
              type="text"
              placeholder="Tooltip on hover — leave empty to use alt text"
              value={draftTitle}
              oninput={(e) => updateDraftTitle((e.target as HTMLInputElement).value)} />
            <span class="field-hint"
              >Shown on hover. Leave empty and the alt text will be used as the <code>title</code>.</span>
          </label>

          <div class="image-align-group">
            <span class="field-label">Alignment</span>
            <div class="image-align-row" role="group" aria-label="Image alignment">
              <button
                type="button"
                class="align-btn"
                class:active={draftAlign === "" || draftAlign === "left"}
                aria-pressed={draftAlign === "" || draftAlign === "left"}
                title="Align left (default)"
                onclick={() => (draftAlign = "left")}>
                <AlignLeftIcon size={14} strokeWidth={1.8} /> Left
              </button>
              <button
                type="button"
                class="align-btn"
                class:active={draftAlign === "center"}
                aria-pressed={draftAlign === "center"}
                title="Center image"
                onclick={() => (draftAlign = "center")}>
                <AlignCenterIcon size={14} strokeWidth={1.8} /> Center
              </button>
              <button
                type="button"
                class="align-btn"
                class:active={draftAlign === "right"}
                aria-pressed={draftAlign === "right"}
                title="Align right"
                onclick={() => (draftAlign = "right")}>
                <AlignRightIcon size={14} strokeWidth={1.8} /> Right
              </button>
            </div>
            <span class="field-hint">Paragraph alignment — center is most common for standalone images.</span>
          </div>

          <div class="image-edit-dim-group">
            <span class="field-label">Size <span class="field-optional">pixels · empty = auto</span></span>
            <div class="image-edit-dim-row">
              <label class="dim-input" class:dim-input--oversized={isWidthOversized}
                ><span class="dim-label">W</span><input
                  type="number"
                  min="16"
                  max="2000"
                  step="1"
                  placeholder="Auto"
                  value={draftWidth}
                  oninput={(e) => updateDraftWidth((e.target as HTMLInputElement).value)}
                  aria-invalid={isWidthOversized} />
                <span class="dim-unit">px</span></label>
              <span class="dim-x" aria-hidden="true">×</span>
              <label class="dim-input" class:dim-input--oversized={isHeightOversized}
                ><span class="dim-label">H</span><input
                  type="number"
                  min="16"
                  max="2000"
                  step="1"
                  placeholder="Auto"
                  value={draftHeight}
                  oninput={(e) => updateDraftHeight((e.target as HTMLInputElement).value)}
                  aria-invalid={isHeightOversized} />
                <span class="dim-unit">px</span></label>
              <button
                type="button"
                class="image-lock-btn"
                aria-pressed={draftPreserveAspect}
                aria-label={draftPreserveAspect ? "Aspect ratio locked" : "Aspect ratio unlocked"}
                title={draftPreserveAspect
                  ? "Aspect locked — height follows width"
                  : "Aspect unlocked — width and height independent"}
                onclick={() => (draftPreserveAspect = !draftPreserveAspect)}>
                {#if draftPreserveAspect}<LinkIcon size={13} strokeWidth={1.9} />{:else}<UnlinkIcon
                    size={13}
                    strokeWidth={1.9} />{/if}
                <span>{draftPreserveAspect ? "Locked" : "Free"}</span>
              </button>
              <button
                type="button"
                class="image-auto-btn"
                onclick={clearDraftDims}
                title="Clear width and height — use natural size">Auto</button>
            </div>
            {#if isOversized}
              <div class="image-oversize-warning" role="alert">
                <TriangleAlertIcon size={13} strokeWidth={1.9} />
                <span
                  >Upscaling — larger than natural <strong>{draftNaturalW}×{draftNaturalH}</strong>
                  {#if isWidthOversized && isHeightOversized}
                    ({draftWidth}×{draftHeight})
                  {:else if isWidthOversized}
                    (width {draftWidth} > {draftNaturalW})
                  {:else}
                    (height {draftHeight} > {draftNaturalH})
                  {/if}
                  may look blurry.</span>
              </div>
            {/if}
            {#if draftNaturalW && draftNaturalH}
              <div class="image-natural-row">
                <InfoIcon size={11} strokeWidth={1.8} />
                <span>Natural <strong>{draftNaturalW}×{draftNaturalH}</strong></span>
                <span class="image-natural-sep">·</span>
                <span>Presets scale with aspect lock on</span>
              </div>
            {/if}
            <div class="image-presets">
              <span class="presets-label">Quick sizes</span>
              <div class="presets-buttons">
                <button
                  type="button"
                  onclick={() => applyDraftPreset("S")}
                  title={draftNaturalW && 320 > draftNaturalW
                    ? `Width 320px — will upscale beyond natural ${draftNaturalW}×${draftNaturalH}`
                    : "Width 320px"}
                  class:presets-btn--upscale={!!(draftNaturalW && 320 > draftNaturalW)}>
                  <span>S</span> <small>320</small>
                  {#if draftNaturalW && 320 > draftNaturalW}<TriangleAlertIcon size={10} strokeWidth={1.9} />{/if}
                </button>
                <button
                  type="button"
                  onclick={() => applyDraftPreset("M")}
                  title={draftNaturalW && 640 > draftNaturalW
                    ? `Width 640px — will upscale beyond natural ${draftNaturalW}×${draftNaturalH}`
                    : "Width 640px"}
                  class:presets-btn--upscale={!!(draftNaturalW && 640 > draftNaturalW)}>
                  <span>M</span> <small>640</small>
                  {#if draftNaturalW && 640 > draftNaturalW}<TriangleAlertIcon size={10} strokeWidth={1.9} />{/if}
                </button>
                <button
                  type="button"
                  onclick={() => applyDraftPreset("L")}
                  title={draftNaturalW && 960 > draftNaturalW
                    ? `Width 960px — will upscale beyond natural ${draftNaturalW}×${draftNaturalH}`
                    : "Width 960px"}
                  class:presets-btn--upscale={!!(draftNaturalW && 960 > draftNaturalW)}>
                  <span>L</span> <small>960</small>
                  {#if draftNaturalW && 960 > draftNaturalW}<TriangleAlertIcon size={10} strokeWidth={1.9} />{/if}
                </button>
                <button
                  type="button"
                  disabled={!draftNaturalW}
                  onclick={() => applyDraftPreset("Original")}
                  title="Use natural dimensions"
                  ><span>Original</span>
                  {#if draftNaturalW}<small>{draftNaturalW}×{draftNaturalH}</small>{/if}</button>
                <button type="button" onclick={() => applyDraftPreset("Full")} title="Remove width and height"
                  >Full</button>
              </div>
            </div>
          </div>
        </div>
      {/if}

      <footer>
        <button type="button" class="quiet" onclick={onCancel}>Cancel</button>
        <button type="button" class="primary" disabled={!canConfirm()} onclick={confirmInsert}
          >{mode === "replace" ? "Save" : "Insert"}</button>
      </footer>
    </div>
  </div>
{/if}

<style>
.insert-asset-backdrop {
  position: fixed;
  inset: 0;
  z-index: 85;
  display: grid;
  place-items: center;
  padding: 18px;
  background: rgba(37, 37, 31, 0.32);
}
.insert-asset-dialog {
  width: min(640px, 100%);
  max-height: min(760px, calc(100vh - 36px));
  display: grid;
  grid-template-rows: auto auto auto 1fr auto auto auto;
  gap: 12px;
  padding: 18px;
  border: 1px solid var(--line);
  border-radius: 14px;
  background: var(--surface);
  box-shadow: 0 24px 70px rgba(38, 42, 33, 0.25);
  overflow: auto;
}
header {
  display: flex;
  justify-content: space-between;
  gap: 12px;
  align-items: flex-start;
}
.kicker {
  display: block;
  color: var(--accent);
  font-size: 9px;
  font-weight: 700;
  letter-spacing: 0.09em;
  text-transform: uppercase;
}
header h2 {
  margin: 4px 0 0;
  font: 700 18px/1.2 var(--font-display, Georgia, serif);
  color: var(--ink);
}
.subtitle {
  margin: 4px 0 0;
  color: var(--ink-soft);
  font-size: 11px;
}
.close-btn {
  width: 30px;
  height: 30px;
  border: 0;
  border-radius: 7px;
  background: var(--surface-muted);
  color: var(--ink-soft);
  cursor: pointer;
}
.close-btn:hover {
  background: var(--theme-warning-bg, #ebe6dd);
  color: var(--ink);
}
.tabs {
  display: flex;
  gap: 6px;
  padding: 4px;
  border-radius: 8px;
  background: var(--canvas);
  border: 1px solid var(--line);
}
.tabs button {
  flex: 1;
  min-height: 32px;
  border: 1px solid transparent;
  border-radius: 6px;
  background: transparent;
  color: var(--ink-soft);
  font: 600 11px/1 var(--font-body, system-ui, sans-serif);
  cursor: pointer;
}
.tabs button.active {
  background: var(--surface);
  border-color: var(--theme-warning-border, #d3c0a9);
  color: var(--accent-dark);
  box-shadow: 0 1px 4px rgba(38, 42, 33, 0.08);
}
.tabs button:disabled {
  opacity: 0.55;
  cursor: not-allowed;
}
.search-row {
  display: flex;
  align-items: center;
  gap: 8px;
  min-height: 36px;
  padding: 0 10px;
  border: 1px solid var(--line);
  border-radius: 8px;
  background: var(--canvas);
}
.search-row input {
  flex: 1;
  border: 0;
  background: transparent;
  outline: 0;
  color: var(--ink);
  font: 500 13px/1 var(--font-body, system-ui, sans-serif);
}
.search-row .clear {
  width: 22px;
  height: 22px;
  border: 0;
  border-radius: 4px;
  background: transparent;
  color: var(--ink-faint);
  cursor: pointer;
}
.assets-list {
  min-height: 180px;
  max-height: 260px;
  overflow-y: auto;
  display: grid;
  gap: 6px;
  padding: 4px;
  border: 1px solid var(--line);
  border-radius: 8px;
  background: var(--canvas);
  align-content: start;
}
.asset-row {
  display: grid;
  grid-template-columns: 36px 1fr auto;
  align-items: center;
  gap: 10px;
  padding: 8px 10px;
  border: 1px solid transparent;
  border-radius: 7px;
  background: transparent;
  text-align: left;
  cursor: pointer;
}
.asset-row:hover,
.asset-row.selected {
  background: var(--surface);
  border-color: var(--theme-warning-border, #d3c0a9);
  box-shadow: var(--shadow-sm, 0 2px 8px rgba(38, 42, 33, 0.06));
}
.asset-row.selected {
  border-color: var(--accent);
}
.thumb {
  display: grid;
  place-items: center;
  width: 36px;
  height: 36px;
  border-radius: 6px;
  background: var(--theme-warning-bg, #ede9e0);
  color: var(--accent);
}
.meta {
  display: grid;
  gap: 2px;
  min-width: 0;
}
.meta strong {
  font-size: 12px;
  color: var(--ink);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.meta small {
  font-size: 10px;
  color: var(--ink-soft);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.role {
  font-size: 10px;
  color: var(--ink-faint);
  text-transform: uppercase;
  letter-spacing: 0.04em;
}
.empty {
  padding: 18px;
  text-align: center;
  color: var(--ink-soft);
  font-size: 12px;
}
.error {
  padding: 10px;
  color: var(--theme-danger-text, #a1482f);
  font-size: 12px;
  background: var(--theme-danger-bg, #fdf0ed);
  border: 1px solid var(--theme-danger-border, #e8c0b8);
  border-radius: 6px;
}
.upload-pane {
  display: grid;
  gap: 12px;
  padding: 14px;
  border: 1px solid var(--line);
  border-radius: 10px;
  background: var(--canvas);
}
.upload-pane--empty {
  place-items: center;
  text-align: center;
}
.upload-pane--staged {
  place-items: stretch;
  text-align: left;
  gap: 14px;
  border-color: var(--theme-warning-border, #d3c0a9);
  background: var(--surface);
  box-shadow: var(--shadow-sm, 0 2px 8px rgba(38, 42, 33, 0.06));
}
.drop-hint {
  display: grid;
  gap: 6px;
  place-items: center;
  color: var(--ink-soft);
}
.upload-pane--staged .drop-hint {
  place-items: start;
  text-align: left;
  padding-bottom: 2px;
  border-bottom: 1px solid var(--line);
}
.drop-hint p {
  margin: 0;
  font-size: 12px;
}
.drop-hint .muted {
  color: var(--ink-faint);
  font-size: 11px;
}
.staged-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  margin: -2px 0 -2px;
}
.staged-badge {
  display: inline-flex;
  align-items: center;
  gap: 5px;
  min-height: 22px;
  padding: 0 8px;
  border-radius: 999px;
  background: var(--theme-success-bg, #eaf6ec);
  border: 1px solid var(--theme-success-border, #b8dcc0);
  color: var(--theme-success-text, #2d6a3f);
  font: 700 10px/1 var(--font-body, system-ui, sans-serif);
  letter-spacing: 0.06em;
  text-transform: uppercase;
}
.staged-hint {
  font: 500 10.5px/1 var(--font-body, system-ui, sans-serif);
  color: var(--ink-soft);
  white-space: nowrap;
}
.staged-card {
  display: grid;
  grid-template-columns: 96px 1fr;
  gap: 12px;
  padding: 10px;
  border: 1px solid var(--line);
  border-radius: 10px;
  background: var(--canvas);
  align-items: center;
}
.staged-thumb {
  width: 96px;
  height: 72px;
  border-radius: 8px;
  overflow: hidden;
  background: var(--surface);
  border: 1px solid var(--line);
  display: grid;
  place-items: center;
}
.staged-thumb img,
.staged-thumb video {
  width: 100%;
  height: 100%;
  object-fit: cover;
  display: block;
}
.staged-fallback {
  display: grid;
  gap: 4px;
  place-items: center;
  width: 100%;
  height: 100%;
  padding: 8px 6px;
  background: var(--theme-warning-bg, #ede9e0);
  color: var(--ink-soft);
  font: 600 10px/1.2 var(--font-body, system-ui, sans-serif);
  text-align: center;
}
.staged-fallback--error {
  background: var(--theme-danger-bg, #fdf0ed);
  border: 1px dashed var(--theme-danger-border, #e8c0b8);
  color: var(--theme-danger-text, #a1482f);
}
.staged-meta {
  display: grid;
  gap: 3px;
  min-width: 0;
  align-content: center;
}
.staged-filename {
  font: 700 13px/1.2 var(--font-body, system-ui, sans-serif);
  color: var(--ink);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.staged-path {
  font: 500 10px/1.2 var(--font-body, system-ui, sans-serif);
  color: var(--ink-faint);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.staged-details {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  flex-wrap: wrap;
  font: 500 10px/1 var(--font-body, system-ui, sans-serif);
  color: var(--ink-soft);
}
.staged-dot {
  color: var(--ink-faint);
}
.staged-image-badge {
  display: inline-flex;
  align-items: center;
  min-height: 16px;
  padding: 0 6px;
  border-radius: 999px;
  background: var(--accent-bg);
  border: 1px solid var(--theme-warning-border, #d3c0a9);
  color: var(--accent-dark);
  font: 700 9px/1 var(--font-body, system-ui, sans-serif);
  letter-spacing: 0.05em;
  text-transform: uppercase;
}
.staged-share-hint {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  margin-top: 2px;
  font: 500 10.5px/1 var(--font-body, system-ui, sans-serif);
  color: var(--theme-success-text, #2d6a3f);
}
.share-row {
  display: grid;
  grid-template-columns: 18px 1fr;
  gap: 8px;
  align-items: start;
  text-align: left;
  width: 100%;
  padding: 10px;
  border: 1px solid var(--line);
  border-radius: 8px;
  background: var(--surface);
  font-size: 12px;
  cursor: pointer;
  transition:
    border-color 0.15s ease,
    background 0.15s ease;
}
.share-row:hover {
  border-color: var(--theme-warning-border, #d3c0a9);
  background: var(--theme-warning-bg, #fcf8f1);
}
.share-row:has(input:checked) {
  border-color: var(--theme-success-border, #b8dcc0);
  background: var(--theme-success-bg, #eaf6ec);
}
.share-row input {
  margin-top: 2px;
  accent-color: var(--accent-dark);
}
.share-row small {
  display: block;
  color: var(--ink-soft);
  font-size: 11px;
}
.picked {
  font-size: 11px;
  color: var(--ink-soft);
}
.upload-btn {
  width: 100%;
  min-height: 38px;
  padding: 0 16px;
  border: 1px solid var(--accent-dark);
  border-radius: 8px;
  background: var(--accent-dark);
  color: #fff;
  font: 700 12px/1 var(--font-body, system-ui, sans-serif);
  cursor: pointer;
  box-shadow: 0 2px 8px rgba(38, 42, 33, 0.12);
  transition: filter 0.15s ease;
}
.upload-btn:hover:not(:disabled),
.upload-btn:focus-visible:not(:disabled) {
  filter: brightness(1.06);
  outline: none;
}
.upload-btn:disabled {
  opacity: 0.45;
  cursor: not-allowed;
  box-shadow: none;
}
.upload-actions {
  display: flex;
  gap: 8px;
  width: 100%;
  justify-content: center;
}
.upload-pane--staged .upload-actions {
  justify-content: flex-end;
}
.upload-actions--staged {
  padding-top: 2px;
  border-top: 1px solid var(--line);
  margin-top: 2px;
}
.upload-actions .quiet,
.upload-actions .secondary {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: 6px;
  min-height: 32px;
  padding: 0 12px;
  border-radius: 8px;
  font: 700 11px/1 var(--font-body, system-ui, sans-serif);
  cursor: pointer;
  transition:
    border-color 0.15s ease,
    background 0.15s ease,
    color 0.15s ease;
}
.upload-actions .quiet {
  border: 1px solid var(--line);
  background: var(--surface);
  color: var(--ink-soft);
}
.upload-actions .quiet:hover {
  border-color: var(--theme-warning-border, #d3c0a9);
  background: var(--accent-bg);
  color: var(--ink);
}
.upload-actions .secondary {
  border: 1px solid var(--theme-warning-border, #d3c0a9);
  background: var(--surface);
  color: var(--accent-dark);
}
.upload-actions .secondary:hover:not(:disabled) {
  background: var(--accent-bg);
}
.upload-actions .secondary:disabled {
  opacity: 0.45;
  cursor: not-allowed;
}
.preview {
  display: grid;
  grid-template-columns: 96px 1fr;
  gap: 10px;
  padding: 10px;
  border: 1px solid var(--line);
  border-radius: 8px;
  background: var(--surface);
  align-items: center;
}
.preview img,
.preview video {
  width: 96px;
  height: 72px;
  object-fit: cover;
  border-radius: 6px;
  background: var(--canvas);
  border: 1px solid var(--line);
}
.preview-fallback {
  display: grid;
  place-items: center;
  gap: 4px;
  width: 96px;
  height: 72px;
  border-radius: 6px;
  background: var(--theme-warning-bg, #ede9e0);
  color: var(--ink-soft);
  font-size: 10px;
  text-align: center;
  padding: 6px;
}
.preview-meta {
  display: grid;
  gap: 4px;
  min-width: 0;
}
.preview-meta strong {
  font-size: 12px;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.preview-meta small {
  font-size: 10px;
  color: var(--ink-soft);
  word-break: break-all;
}
footer {
  display: flex;
  justify-content: flex-end;
  gap: 8px;
}
footer button {
  min-height: 34px;
  padding: 0 14px;
  border-radius: 7px;
  font: 700 12px/1 var(--font-body, system-ui, sans-serif);
  cursor: pointer;
  border: 0;
}
footer .quiet {
  background: transparent;
  color: var(--ink-soft);
}
footer .quiet:hover {
  background: var(--surface-muted);
  color: var(--ink);
}
footer .primary {
  background: var(--accent-dark);
  color: white;
}
footer .primary:disabled {
  opacity: 0.45;
  cursor: not-allowed;
}
.image-edit-section {
  display: grid;
  gap: 14px;
  padding: 14px;
  border: 1px solid var(--line);
  border-radius: 10px;
  background: var(--surface);
  box-shadow: var(--shadow-sm, 0 2px 8px rgba(38, 42, 33, 0.04));
}
.image-edit-header {
  display: flex;
  align-items: baseline;
  justify-content: space-between;
  gap: 12px;
  padding-bottom: 8px;
  border-bottom: 1px solid var(--line);
  margin: -2px 0 2px;
}
.image-edit-header-title {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  font: 700 11px/1 var(--font-body, system-ui, sans-serif);
  color: var(--ink);
  letter-spacing: 0.04em;
  text-transform: uppercase;
}
.image-edit-header-hint {
  font: 500 10px/1 var(--font-body, system-ui, sans-serif);
  color: var(--ink-faint);
  white-space: nowrap;
}
.image-edit-field {
  display: grid;
  gap: 6px;
}
.field-label {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  font: 700 11px/1 var(--font-body, system-ui, sans-serif);
  color: var(--ink);
  letter-spacing: 0.01em;
}
.field-badge {
  display: inline-flex;
  align-items: center;
  min-height: 16px;
  padding: 0 6px;
  border-radius: 999px;
  font: 700 9px/1 var(--font-body, system-ui, sans-serif);
  letter-spacing: 0.06em;
  text-transform: uppercase;
  border: 1px solid transparent;
}
.field-badge--required {
  background: var(--theme-danger-bg, #fdf0ed);
  border-color: var(--theme-danger-border, #e8c0b8);
  color: var(--theme-danger-text, #a1482f);
}
.field-badge--optional {
  background: var(--canvas);
  border-color: var(--line);
  color: var(--ink-soft);
}
.field-optional {
  font: 500 10px/1 var(--font-body, system-ui, sans-serif);
  color: var(--ink-faint);
  font-weight: 500;
  text-transform: none;
  letter-spacing: 0;
}
.image-edit-field input {
  width: 100%;
  min-height: 34px;
  padding: 0 10px;
  border: 1px solid var(--line);
  border-radius: 8px;
  background: var(--canvas);
  color: var(--ink);
  font: 500 13px/1 var(--font-body, system-ui, sans-serif);
  outline: 0;
  transition:
    border-color 0.15s ease,
    box-shadow 0.15s ease,
    background 0.15s ease;
}
.image-edit-field input::placeholder {
  color: var(--ink-faint);
}
.image-edit-field input:focus {
  border-color: var(--theme-warning-border, #d3c0a9);
  background: var(--surface);
  box-shadow: 0 0 0 3px rgba(211, 192, 169, 0.22);
}
.field-hint {
  display: inline-flex;
  gap: 4px;
  font: 500 10.5px/1.4 var(--font-body, system-ui, sans-serif);
  color: var(--ink-soft);
}
.field-hint code {
  padding: 0 4px;
  border-radius: 4px;
  background: var(--canvas);
  border: 1px solid var(--line);
  font:
    600 10px/1.5 ui-monospace,
    SFMono-Regular,
    Menlo,
    monospace;
  color: var(--ink-soft);
}
.image-edit-dim-group {
  display: grid;
  gap: 8px;
  padding: 10px;
  border: 1px solid var(--line);
  border-radius: 8px;
  background: var(--canvas);
}
.image-edit-dim-group .field-label {
  margin-bottom: 2px;
}
.image-edit-dim-row {
  display: flex;
  align-items: center;
  gap: 8px;
  flex-wrap: wrap;
}
.dim-input {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  min-height: 32px;
  padding: 0 8px 0 8px;
  border: 1px solid var(--line);
  border-radius: 8px;
  background: var(--surface);
  transition:
    border-color 0.15s ease,
    box-shadow 0.15s ease;
}
.dim-input:focus-within {
  border-color: var(--theme-warning-border, #d3c0a9);
  box-shadow: 0 0 0 3px rgba(211, 192, 169, 0.18);
}
.dim-label {
  font: 700 11px/1 var(--font-body, system-ui, sans-serif);
  color: var(--ink-soft);
}
.dim-input input {
  width: 64px;
  min-height: 28px;
  padding: 0 2px;
  border: 0;
  background: transparent;
  color: var(--ink);
  font: 600 13px/1 var(--font-body, system-ui, sans-serif);
  outline: 0;
  text-align: center;
}
.dim-input input::placeholder {
  color: var(--ink-faint);
  font-weight: 500;
}
.dim-unit {
  font: 600 11px/1 var(--font-body, system-ui, sans-serif);
  color: var(--ink-faint);
}
.dim-x {
  font: 500 13px/1 var(--font-body, system-ui, sans-serif);
  color: var(--ink-faint);
  padding: 0 2px;
  user-select: none;
}
.dim-input--oversized {
  border-color: var(--theme-warning-border, #e8a040) !important;
  background: var(--theme-warning-bg, #fdf6e3) !important;
  box-shadow: 0 0 0 2px rgba(232, 160, 64, 0.18);
}
.dim-input--oversized .dim-label,
.dim-input--oversized .dim-unit {
  color: var(--theme-warning-text, #7a4a08);
}
.dim-input--oversized input {
  color: var(--theme-warning-text, #7a4a08);
}
.image-oversize-warning {
  display: flex;
  align-items: flex-start;
  gap: 8px;
  padding: 8px 10px;
  border: 1px solid var(--theme-warning-border, #f0c98a);
  border-radius: 8px;
  background: var(--theme-warning-bg, #fdf6e3);
  color: var(--theme-warning-text, #7a4a08);
  font: 500 11px/1.4 var(--font-body, system-ui, sans-serif);
}
.image-oversize-warning strong {
  color: var(--theme-warning-text, #5e3800);
  font-weight: 700;
}
.image-align-group {
  display: grid;
  gap: 8px;
  padding: 10px;
  border: 1px solid var(--line);
  border-radius: 8px;
  background: var(--canvas);
}
.image-align-row {
  display: flex;
  gap: 6px;
  flex-wrap: wrap;
}
.align-btn {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: 6px;
  min-height: 32px;
  flex: 1 1 0;
  padding: 0 10px;
  border: 1px solid var(--line);
  border-radius: 8px;
  background: var(--surface);
  color: var(--ink-soft);
  font: 700 11px/1 var(--font-body, system-ui, sans-serif);
  cursor: pointer;
  transition:
    border-color 0.15s ease,
    background 0.15s ease,
    color 0.15s ease,
    box-shadow 0.15s ease;
}
.align-btn:hover,
.align-btn:focus-visible {
  border-color: var(--theme-warning-border, #d3c0a9);
  background: var(--accent-bg);
  color: var(--ink);
  outline: 0;
}
.align-btn.active,
.align-btn[aria-pressed="true"] {
  border-color: var(--theme-warning-border, #d3c0a9);
  background: var(--accent-bg);
  color: var(--accent-dark);
  box-shadow: inset 0 0 0 1px rgba(211, 192, 169, 0.35);
}
.image-lock-btn,
.image-auto-btn {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: 6px;
  min-height: 32px;
  padding: 0 10px;
  border: 1px solid var(--line);
  border-radius: 8px;
  background: var(--surface);
  color: var(--ink-soft);
  font: 700 11px/1 var(--font-body, system-ui, sans-serif);
  cursor: pointer;
  transition:
    border-color 0.15s ease,
    background 0.15s ease,
    color 0.15s ease;
  white-space: nowrap;
}
.image-lock-btn:hover,
.image-auto-btn:hover,
.image-lock-btn:focus-visible,
.image-auto-btn:focus-visible {
  border-color: var(--theme-warning-border, #d3c0a9);
  background: var(--accent-bg);
  color: var(--ink);
  outline: 0;
}
.image-lock-btn[aria-pressed="true"] {
  border-color: var(--theme-warning-border, #d3c0a9);
  background: var(--accent-bg);
  color: var(--accent-dark);
  box-shadow: inset 0 0 0 1px rgba(211, 192, 169, 0.35);
}
.image-lock-btn[aria-pressed="false"] {
  border-style: dashed;
}
.image-natural-row {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  flex-wrap: wrap;
  font: 500 11px/1 var(--font-body, system-ui, sans-serif);
  color: var(--ink-soft);
  padding: 2px 0 0;
}
.image-natural-row strong {
  color: var(--ink);
  font-weight: 700;
}
.image-natural-sep {
  color: var(--ink-faint);
}
.image-presets {
  display: grid;
  gap: 6px;
  padding-top: 2px;
  border-top: 1px dashed var(--line);
  margin-top: 2px;
}
.presets-label {
  font: 700 10px/1 var(--font-body, system-ui, sans-serif);
  color: var(--ink-faint);
  letter-spacing: 0.06em;
  text-transform: uppercase;
}
.presets-buttons {
  display: flex;
  gap: 6px;
  flex-wrap: wrap;
}
.image-presets button {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  min-height: 28px;
  padding: 0 10px;
  border: 1px solid var(--line);
  border-radius: 999px;
  background: var(--surface);
  color: var(--ink-soft);
  font: 700 11px/1 var(--font-body, system-ui, sans-serif);
  cursor: pointer;
  transition:
    border-color 0.15s ease,
    background 0.15s ease,
    color 0.15s ease,
    transform 0.08s ease;
}
.image-presets button small {
  font: 600 10px/1 var(--font-body, system-ui, sans-serif);
  color: var(--ink-faint);
}
.image-presets button:hover,
.image-presets button:focus-visible {
  border-color: var(--theme-warning-border, #d3c0a9);
  background: var(--accent-bg);
  color: var(--accent-dark);
  outline: 0;
  transform: translateY(-1px);
}
.image-presets button:disabled {
  opacity: 0.45;
  cursor: not-allowed;
  transform: none;
}
.image-presets button:active:not(:disabled) {
  transform: translateY(0);
}
.presets-btn--upscale {
  border-color: var(--theme-warning-border, #f0c98a) !important;
  background: var(--theme-warning-bg, #fdf6e3) !important;
  color: var(--theme-warning-text, #7a4a08) !important;
}
.presets-btn--upscale small {
  color: var(--theme-warning-text, #7a4a08) !important;
}
</style>
