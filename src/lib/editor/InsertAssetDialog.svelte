<script lang="ts">
import { tick, onMount } from "svelte";
import { X, Search as SearchIcon, Image as ImageIcon, File as FileIcon, Upload } from "@lucide/svelte";
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
}: {
  open: boolean;
  entityId?: string | null;
  entities?: Entity[];
  defaultNamespace?: string | null;
  onInsert: (asset: Asset | null, meta?: { alt: string; title: string; width: string; height: string }) => void;
  onCancel: () => void;
  mode?: "insert" | "replace";
  initialAlt?: string;
  initialTitle?: string;
  initialWidth?: string;
  initialHeight?: string;
  initialSrc?: string;
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

// image edit state (for alt/dim before insert or when replacing)
let draftAlt = $state("");
let draftTitle = $state("");
let draftWidth = $state("");
let draftHeight = $state("");
let draftTitleCustom = $state(false);
let draftPreserveAspect = $state(true);
let draftNaturalW = $state(0);
let draftNaturalH = $state(0);
let draftNaturalCache = new Map<string, { w: number; h: number }>();

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
  img.src = url;
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
    // init image edit drafts from props (for replace mode or new insert)
    draftAlt = initialAlt;
    draftTitle = initialTitle;
    draftWidth = /^\d+$/.test(initialWidth) ? initialWidth : "";
    draftHeight = /^\d+$/.test(initialHeight) ? initialHeight : "";
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

$effect(() => {
  if (previewUrl && (selectedAsset ? isImage(selectedAsset.mime_type) : !!initialSrc)) {
    probeNaturalForDialog(initialSrc, previewUrl);
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

function imageMeta(): { alt: string; title: string; width: string; height: string } {
  const alt = draftAlt.trim();
  const title = draftTitleCustom ? draftTitle.trim() : alt;
  const w = /^\d+$/.test(draftWidth.trim()) ? draftWidth.trim() : "";
  const h = /^\d+$/.test(draftHeight.trim()) ? draftHeight.trim() : "";
  return { alt, title, width: w, height: h };
}
function isImageContext(): boolean {
  if (selectedAsset) return isImage(selectedAsset.mime_type);
  if (mode === "replace" && initialSrc) return true;
  return false;
}
function canConfirm(): boolean {
  if (activeTab === "upload") return false;
  if (selectedAsset) return true;
  // in replace mode we allow saving meta edits without picking new asset
  if (mode === "replace" && initialSrc) return true;
  return false;
}
function confirmInsert() {
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
    onCancel();
  } else if (event.key === "Enter" && canConfirm() && activeTab !== "upload") {
    // avoid submitting when typing search?
    const target = event.target as HTMLElement | null;
    if (target?.tagName === "INPUT") return;
    event.preventDefault();
    confirmInsert();
  }
}

function selectAsset(id: string) {
  selectedId = id;
}

async function handleUploadAndInsert() {
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
    // optionally update referenceScope if shareNew
    const metaForUpload = isImage(mime) ? imageMeta() : undefined;
    // if user edited alt in dialog before upload, prefer that alt; otherwise default to filename
    if (metaForUpload && !metaForUpload.alt) metaForUpload.alt = filename;
    if (shareNew && asset.reference_scope !== "project") {
      try {
        const updated = await project.updateAssetMetadata(asset.id, { referenceScope: "project" }, asset.revision);
        onInsert(updated, metaForUpload);
      } catch {
        // still insert original
        onInsert(asset, metaForUpload);
      }
    } else {
      onInsert(asset, metaForUpload);
    }
    // also refresh local lists
    myAssets = [...myAssets, asset];
    if (asset.reference_scope === "project") sharedAssets = [asset, ...sharedAssets];
  } catch (e) {
    uploadError = e instanceof Error ? e.message : String(e);
  } finally {
    uploading = false;
  }
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
                onclick={() => selectAsset(asset.id)}
                ondblclick={() => {
                  const meta = isImage(asset.mime_type) ? imageMeta() : undefined;
                  onInsert(asset, meta);
                }}>
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
                onclick={() => selectAsset(asset.id)}
                ondblclick={() => {
                  const meta = isImage(asset.mime_type) ? imageMeta() : undefined;
                  onInsert(asset, meta);
                }}>
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
        <div class="upload-pane">
          <div class="drop-hint">
            <Upload size={20} strokeWidth={1.6} />
            <p><strong>Upload a new file to this entry</strong></p>
            <p class="muted">Images will be inserted as images, other files as links.</p>
          </div>
          <label class="share-row">
            <input type="checkbox" bind:checked={shareNew} />
            <span>Share with other entries <small>Others can pick this file from Shared files.</small></span>
          </label>
          {#if pickedFileLabel}<p class="picked">Last picked: {pickedFileLabel}</p>{/if}
          {#if uploadError}<p class="error">{uploadError}</p>{/if}
          <button
            type="button"
            class="primary upload-btn"
            disabled={uploading || !entityId}
            onclick={handleUploadAndInsert}>
            {#if uploading}Uploading…{:else}Choose file & insert{/if}
          </button>
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
          <label class="image-edit-field">
            <span>Alt text</span>
            <input
              type="text"
              placeholder="Describe image"
              value={draftAlt}
              oninput={(e) => updateDraftAlt((e.target as HTMLInputElement).value)} />
          </label>
          <details class="image-edit-advanced">
            <summary>Title (advanced)</summary>
            <input
              type="text"
              placeholder="Title defaults to alt"
              value={draftTitle}
              oninput={(e) => updateDraftTitle((e.target as HTMLInputElement).value)} />
          </details>
          <div class="image-edit-dim-row">
            <label
              >W <input
                type="number"
                min="16"
                max="2000"
                step="1"
                placeholder="Auto"
                value={draftWidth}
                oninput={(e) => updateDraftWidth((e.target as HTMLInputElement).value)} /> px</label>
            <span>×</span>
            <label
              >H <input
                type="number"
                min="16"
                max="2000"
                step="1"
                placeholder="Auto"
                value={draftHeight}
                oninput={(e) => updateDraftHeight((e.target as HTMLInputElement).value)} /> px</label>
            <button
              type="button"
              class="image-lock-btn"
              aria-pressed={draftPreserveAspect}
              onclick={() => (draftPreserveAspect = !draftPreserveAspect)}
              title={draftPreserveAspect ? "Aspect locked" : "Aspect unlocked"}
              >{draftPreserveAspect ? "🔗" : "🔓"}</button>
            <button type="button" class="image-auto-btn" onclick={clearDraftDims}>Auto</button>
          </div>
          {#if draftNaturalW && draftNaturalH}
            <small class="image-natural-hint">Original {draftNaturalW}×{draftNaturalH}</small>
          {/if}
          <div class="image-presets">
            <button type="button" onclick={() => applyDraftPreset("S")}>S 320</button>
            <button type="button" onclick={() => applyDraftPreset("M")}>M 640</button>
            <button type="button" onclick={() => applyDraftPreset("L")}>L 960</button>
            <button type="button" disabled={!draftNaturalW} onclick={() => applyDraftPreset("Original")}
              >Original</button>
            <button type="button" onclick={() => applyDraftPreset("Full")}>Full</button>
          </div>
        </div>
      {/if}

      <footer>
        <button type="button" class="quiet" onclick={onCancel}>Cancel</button>
        {#if activeTab !== "upload"}
          <button type="button" class="primary" disabled={!canConfirm()} onclick={confirmInsert}
            >{mode === "replace" ? "Save" : "Insert"}</button>
        {/if}
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
  border: 1px solid var(--line, #e4e1d8);
  border-radius: 14px;
  background: var(--surface, #fffefa);
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
  color: var(--accent, #b4773f);
  font-size: 9px;
  font-weight: 700;
  letter-spacing: 0.09em;
  text-transform: uppercase;
}
header h2 {
  margin: 4px 0 0;
  font: 700 18px/1.2 var(--font-display, Georgia, serif);
  color: var(--ink, #25251f);
}
.subtitle {
  margin: 4px 0 0;
  color: var(--ink-soft, #77766d);
  font-size: 11px;
}
.close-btn {
  width: 30px;
  height: 30px;
  border: 0;
  border-radius: 7px;
  background: var(--surface-muted, #f4f2ec);
  color: var(--ink-soft, #77766d);
  cursor: pointer;
}
.close-btn:hover {
  background: #ebe6dd;
  color: var(--ink, #25251f);
}
.tabs {
  display: flex;
  gap: 6px;
  padding: 4px;
  border-radius: 8px;
  background: var(--canvas, #f7f6f2);
  border: 1px solid var(--line, #e4e1d8);
}
.tabs button {
  flex: 1;
  min-height: 32px;
  border: 1px solid transparent;
  border-radius: 6px;
  background: transparent;
  color: var(--ink-soft, #77766d);
  font: 600 11px/1 var(--font-body, system-ui, sans-serif);
  cursor: pointer;
}
.tabs button.active {
  background: var(--surface, #fffefa);
  border-color: #d3c0a9;
  color: var(--accent-dark, #365342);
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
  border: 1px solid var(--line, #e4e1d8);
  border-radius: 8px;
  background: var(--canvas, #f7f6f2);
}
.search-row input {
  flex: 1;
  border: 0;
  background: transparent;
  outline: 0;
  color: var(--ink, #25251f);
  font: 500 13px/1 var(--font-body, system-ui, sans-serif);
}
.search-row .clear {
  width: 22px;
  height: 22px;
  border: 0;
  border-radius: 4px;
  background: transparent;
  color: var(--ink-faint, #aaa79d);
  cursor: pointer;
}
.assets-list {
  min-height: 180px;
  max-height: 260px;
  overflow-y: auto;
  display: grid;
  gap: 6px;
  padding: 4px;
  border: 1px solid var(--line, #e4e1d8);
  border-radius: 8px;
  background: var(--canvas, #f7f6f2);
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
  background: var(--surface, #fffefa);
  border-color: #d3c0a9;
  box-shadow: var(--shadow-sm, 0 2px 8px rgba(38, 42, 33, 0.06));
}
.asset-row.selected {
  border-color: #b4773f;
}
.thumb {
  display: grid;
  place-items: center;
  width: 36px;
  height: 36px;
  border-radius: 6px;
  background: #ede9e0;
  color: var(--accent, #b4773f);
}
.meta {
  display: grid;
  gap: 2px;
  min-width: 0;
}
.meta strong {
  font-size: 12px;
  color: var(--ink, #25251f);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.meta small {
  font-size: 10px;
  color: var(--ink-soft, #77766d);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.role {
  font-size: 10px;
  color: var(--ink-faint, #aaa79d);
  text-transform: uppercase;
  letter-spacing: 0.04em;
}
.empty {
  padding: 18px;
  text-align: center;
  color: var(--ink-soft, #77766d);
  font-size: 12px;
}
.error {
  padding: 10px;
  color: #a1482f;
  font-size: 12px;
  background: #fdf0ed;
  border: 1px solid #e8c0b8;
  border-radius: 6px;
}
.upload-pane {
  display: grid;
  gap: 12px;
  padding: 14px;
  border: 1px solid var(--line, #e4e1d8);
  border-radius: 8px;
  background: var(--canvas, #f7f6f2);
  place-items: center;
  text-align: center;
}
.drop-hint {
  display: grid;
  gap: 6px;
  place-items: center;
  color: var(--ink-soft, #77766d);
}
.drop-hint p {
  margin: 0;
  font-size: 12px;
}
.drop-hint .muted {
  color: var(--ink-faint, #aaa79d);
  font-size: 11px;
}
.share-row {
  display: grid;
  grid-template-columns: 18px 1fr;
  gap: 8px;
  align-items: start;
  text-align: left;
  width: 100%;
  padding: 10px;
  border: 1px solid var(--line, #e4e1d8);
  border-radius: 7px;
  background: var(--surface, #fffefa);
  font-size: 12px;
  cursor: pointer;
}
.share-row input {
  margin-top: 2px;
  accent-color: var(--accent-dark, #365342);
}
.share-row small {
  display: block;
  color: var(--ink-soft, #77766d);
  font-size: 11px;
}
.picked {
  font-size: 11px;
  color: var(--ink-soft, #77766d);
}
.upload-btn {
  width: 100%;
  min-height: 38px;
  padding: 0 16px;
  border: 1px solid var(--accent-dark, #365342);
  border-radius: 8px;
  background: var(--accent-dark, #365342);
  color: #fff;
  font: 700 12px/1 var(--font-body, system-ui, sans-serif);
  cursor: pointer;
  box-shadow: 0 2px 8px rgba(38, 42, 33, 0.12);
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
.preview {
  display: grid;
  grid-template-columns: 96px 1fr;
  gap: 10px;
  padding: 10px;
  border: 1px solid var(--line, #e4e1d8);
  border-radius: 8px;
  background: var(--surface, #fffefa);
  align-items: center;
}
.preview img,
.preview video {
  width: 96px;
  height: 72px;
  object-fit: cover;
  border-radius: 6px;
  background: var(--canvas, #f7f6f2);
  border: 1px solid var(--line, #e4e1d8);
}
.preview-fallback {
  display: grid;
  place-items: center;
  gap: 4px;
  width: 96px;
  height: 72px;
  border-radius: 6px;
  background: #ede9e0;
  color: var(--ink-soft, #77766d);
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
  color: var(--ink-soft, #77766d);
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
  color: var(--ink-soft, #77766d);
}
footer .quiet:hover {
  background: var(--surface-muted, #f4f2ec);
  color: var(--ink, #25251f);
}
footer .primary {
  background: var(--accent-dark, #365342);
  color: white;
}
footer .primary:disabled {
  opacity: 0.45;
  cursor: not-allowed;
}
.image-edit-section {
  display: grid;
  gap: 10px;
  padding: 12px;
  border: 1px solid var(--line, #e4e1d8);
  border-radius: 8px;
  background: var(--surface, #fffefa);
}
.image-edit-field {
  display: grid;
  gap: 4px;
  font: 600 11px/1 var(--font-body, system-ui, sans-serif);
  color: var(--ink-soft, #77766d);
}
.image-edit-field input,
.image-edit-advanced input {
  width: 100%;
  min-height: 30px;
  padding: 0 8px;
  border: 1px solid var(--line, #e4e1d8);
  border-radius: 6px;
  background: var(--canvas, #f7f6f2);
  color: var(--ink, #25251f);
  font: 500 12px/1 var(--font-body, system-ui, sans-serif);
  outline: 0;
}
.image-edit-field input:focus,
.image-edit-advanced input:focus {
  border-color: #d3c0a9;
  box-shadow: 0 0 0 2px rgba(211, 192, 169, 0.18);
}
.image-edit-advanced {
  font: 600 11px/1 var(--font-body, system-ui, sans-serif);
  color: var(--ink-soft, #77766d);
}
.image-edit-advanced summary {
  cursor: pointer;
  user-select: none;
  padding: 4px 0;
}
.image-edit-dim-row {
  display: flex;
  align-items: center;
  gap: 6px;
  font: 600 11px/1 var(--font-body, system-ui, sans-serif);
  color: var(--ink-soft, #77766d);
}
.image-edit-dim-row label {
  display: inline-flex;
  align-items: center;
  gap: 4px;
}
.image-edit-dim-row input {
  width: 70px;
  min-height: 28px;
  padding: 0 6px;
  border: 1px solid var(--line, #e4e1d8);
  border-radius: 6px;
  background: var(--canvas, #f7f6f2);
  color: var(--ink, #25251f);
  font: 500 12px/1 var(--font-body, system-ui, sans-serif);
  outline: 0;
}
.image-edit-dim-row input:focus {
  border-color: #d3c0a9;
  box-shadow: 0 0 0 2px rgba(211, 192, 169, 0.18);
}
.image-lock-btn,
.image-auto-btn {
  min-height: 26px;
  padding: 0 8px;
  border: 1px solid var(--line, #e4e1d8);
  border-radius: 6px;
  background: var(--surface, #fffefa);
  color: var(--ink-soft, #77766d);
  font: 600 11px/1 var(--font-body, system-ui, sans-serif);
  cursor: pointer;
}
.image-lock-btn[aria-pressed="true"] {
  border-color: #d3c0a9;
  background: #f2e4d2;
  color: var(--accent-dark, #365342);
}
.image-natural-hint {
  color: var(--ink-faint, #aaa79d);
  font: 500 10px/1 var(--font-body, system-ui, sans-serif);
}
.image-presets {
  display: flex;
  gap: 4px;
  flex-wrap: wrap;
}
.image-presets button {
  min-height: 24px;
  padding: 0 8px;
  border: 1px solid var(--line, #e4e1d8);
  border-radius: 6px;
  background: var(--surface, #fffefa);
  color: var(--ink-soft, #77766d);
  font: 600 11px/1 var(--font-body, system-ui, sans-serif);
  cursor: pointer;
}
.image-presets button:hover,
.image-presets button:focus-visible {
  border-color: #d3c0a9;
  background: #f2e4d2;
  outline: 0;
}
.image-presets button:disabled {
  opacity: 0.45;
  cursor: not-allowed;
}
</style>
