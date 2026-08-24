<script lang="ts">
import { onMount, tick } from "svelte";
import { X } from "@lucide/svelte";
import type { Asset } from "$lib/project/client";
import { project } from "$lib/project/client";

let {
  asset,
  editable = true,
  onSave,
  onDelete,
  onReplace,
  onClose,
}: {
  asset: Asset;
  editable?: boolean;
  onSave: (update: {
    filename?: string;
    role?: "attachment" | "profile";
    referenceScope?: "entity" | "project";
  }) => Promise<void>;
  onDelete: () => Promise<void>;
  onReplace: () => Promise<void>;
  onClose: () => void;
} = $props();

let dialogElement = $state<HTMLDivElement | null>(null);
function splitFilename(name: string): { base: string; ext: string } {
  const idx = name.lastIndexOf(".");
  if (idx > 0 && idx < name.length - 1) return { base: name.slice(0, idx), ext: name.slice(idx + 1) };
  return { base: name, ext: "" };
}
// svelte-ignore state_referenced_locally
const _split = splitFilename(asset.filename);
// svelte-ignore state_referenced_locally
let filenameBase = $state(_split.base);
// svelte-ignore state_referenced_locally
const readonlyExt = _split.ext;
// svelte-ignore state_referenced_locally
let role = $state<"attachment" | "profile">(asset.role);
// svelte-ignore state_referenced_locally
let referenceScope = $state<"entity" | "project">(asset.reference_scope);
let saving = $state(false);
let deleting = $state(false);
let replacing = $state(false);
let saveError = $state("");
let deleteError = $state("");
let replaceError = $state("");
let lastFocused: Element | null = null;
let previewUrl = $state("");
let previewError = $state(false);

// svelte-ignore state_referenced_locally
const canBeProfile = ["image/png", "image/jpeg", "image/gif", "image/webp"].includes(asset.mime_type);
// svelte-ignore state_referenced_locally
const original = { filename: asset.filename, role: asset.role, referenceScope: asset.reference_scope };
const reconstructedFilename = () => (readonlyExt ? `${filenameBase.trim()}.${readonlyExt}` : filenameBase.trim());
const hasChanges = () =>
  reconstructedFilename() !== original.filename || role !== original.role || referenceScope !== original.referenceScope;

function formatCreatedAt(value: string): string {
  // created_at is stored as nanoseconds since epoch (chrono_like_now)
  const n = Number(value);
  if (Number.isFinite(n) && value.trim() !== "" && /^\d+$/.test(value.trim())) {
    const ms = Math.floor(n / 1_000_000);
    const d = new Date(ms);
    if (!Number.isNaN(d.getTime())) {
      return d.toLocaleString(undefined, {
        year: "numeric",
        month: "short",
        day: "numeric",
        hour: "2-digit",
        minute: "2-digit",
      });
    }
  }
  const fallback = new Date(value);
  if (!Number.isNaN(fallback.getTime())) return fallback.toLocaleString();
  return value;
}

function isImage(mime: string) {
  return mime.startsWith("image/");
}
function isVideo(mime: string) {
  return mime.startsWith("video/");
}
function extOf(name: string): string {
  const parts = name.split(".");
  return parts.length > 1 ? (parts.pop() ?? "") : "";
}

$effect(() => {
  const a = asset;
  let disposed = false;
  let objectUrl = "";
  previewUrl = "";
  previewError = false;
  // Only preview images and videos to avoid large arbitrary blobs; other types get icon
  const shouldPreview = isImage(a.mime_type) || isVideo(a.mime_type);
  if (shouldPreview) {
    void project
      .readAssetBytes(a.id)
      .then((bytes) => {
        if (disposed) return;
        try {
          const blob = new Blob([Uint8Array.from(bytes)], { type: a.mime_type });
          objectUrl = URL.createObjectURL(blob);
          if (disposed) {
            URL.revokeObjectURL(objectUrl);
            objectUrl = "";
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
  }
  return () => {
    disposed = true;
    if (objectUrl) URL.revokeObjectURL(objectUrl);
  };
});

function validateFilename(value: string): string | null {
  const trimmed = value.trim();
  if (!trimmed) return "Filename is required.";
  if (trimmed === "." || trimmed === ".." || trimmed.includes("/") || trimmed.includes("\\")) {
    return "Filename must be a single file name without path separators.";
  }
  return null;
}

async function handleSave() {
  const nextFilename = reconstructedFilename();
  const err = validateFilename(nextFilename);
  if (err) {
    saveError = err;
    return;
  }
  if (!hasChanges()) {
    onClose();
    return;
  }
  saving = true;
  saveError = "";
  try {
    const update: Record<string, string> = {};
    if (nextFilename !== original.filename) update.filename = nextFilename;
    if (role !== original.role) update.role = role;
    if (referenceScope !== original.referenceScope) update.referenceScope = referenceScope;
    await onSave(update as any);
    onClose();
  } catch (cause) {
    saveError = cause instanceof Error ? cause.message : "Could not save changes.";
  } finally {
    saving = false;
  }
}

async function handleDelete() {
  deleting = true;
  deleteError = "";
  try {
    await onDelete();
    onClose();
  } catch (cause) {
    deleteError = cause instanceof Error ? cause.message : "Could not delete file.";
  } finally {
    deleting = false;
  }
}

async function handleReplace() {
  replacing = true;
  replaceError = "";
  try {
    await onReplace();
  } catch (cause) {
    replaceError = cause instanceof Error ? cause.message : "Could not replace file.";
  } finally {
    replacing = false;
  }
}

function focusableElements(): HTMLElement[] {
  return Array.from(
    dialogElement?.querySelectorAll<HTMLElement>(
      'button:not([disabled]), input:not([disabled]), select:not([disabled]), textarea:not([disabled]), [tabindex]:not([tabindex="-1"])',
    ) ?? [],
  );
}

onMount(() => {
  lastFocused = document.activeElement;
  void tick().then(() => focusableElements()[0]?.focus());
  const handleKeydown = (event: KeyboardEvent) => {
    if (event.key === "Escape") {
      event.preventDefault();
      onClose();
      return;
    }
    if (event.key !== "Tab") return;
    const focusable = focusableElements();
    if (focusable.length === 0) {
      event.preventDefault();
      dialogElement?.focus();
      return;
    }
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
  window.addEventListener("keydown", handleKeydown, true);
  return () => {
    window.removeEventListener("keydown", handleKeydown, true);
    if (lastFocused instanceof HTMLElement && lastFocused.isConnected) lastFocused.focus();
  };
});
</script>

<div class="asset-dialog-backdrop" role="presentation" onclick={onClose}>
  <div
    bind:this={dialogElement}
    class="asset-dialog"
    role="dialog"
    tabindex="-1"
    aria-modal="true"
    aria-labelledby="asset-dialog-title"
    onclick={(event) => event.stopPropagation()}
    onkeydown={(event) => event.stopPropagation()}>
    <header class="asset-dialog-header">
      <div>
        <span class="asset-dialog-kicker">FILE DETAILS</span>
        <h2 id="asset-dialog-title">Edit file</h2>
        <p>Manage this attachment. Changes are versioned and synced via the checkpoint.</p>
      </div>
      <button type="button" class="asset-dialog-close" aria-label="Close file details" onclick={onClose}
        ><X size={16} strokeWidth={1.8} aria-hidden="true" /></button>
    </header>

    <div class="asset-dialog-preview">
      {#if previewUrl && isImage(asset.mime_type)}
        <img class="asset-dialog-preview-image" src={previewUrl} alt={`${asset.filename} preview`} />
      {:else if previewUrl && isVideo(asset.mime_type)}
        <!-- svelte-ignore a11y_media_has_caption -->
        <video class="asset-dialog-preview-image" src={previewUrl} controls preload="metadata"></video>
      {:else if isImage(asset.mime_type) && previewError}
        <div class="asset-dialog-preview-fallback" role="img" aria-label={`${asset.filename} preview unavailable`}>
          <span class="asset-dialog-preview-icon">🖼</span><span>Preview unavailable</span>
        </div>
      {:else if isImage(asset.mime_type)}
        <div class="asset-dialog-preview-fallback">
          <span class="asset-dialog-preview-icon">🖼</span><span>Loading preview…</span>
        </div>
      {:else}
        <div class="asset-dialog-preview-fallback" aria-hidden="true">
          <span class="asset-dialog-preview-icon"
            >{asset.mime_type.includes("pdf") ? "📄" : asset.mime_type.includes("audio") ? "🎵" : "📎"}</span>
          <span>{asset.mime_type}</span>
        </div>
      {/if}
    </div>

    <div class="asset-dialog-meta">
      <span><strong>{asset.filename}</strong></span>
      <small>{Math.max(1, Math.round(asset.size / 1024))} KB · {asset.mime_type}</small>
      <small>Created {formatCreatedAt(asset.created_at)}</small>
    </div>

    <form
      class="asset-dialog-form"
      onsubmit={(event) => {
        event.preventDefault();
        void handleSave();
      }}>
      <label class="asset-dialog-field">
        <span>Filename<b aria-hidden="true"> *</b></span>
        <div class="asset-filename-row">
          <input
            type="text"
            value={filenameBase}
            disabled={!editable}
            oninput={(e) => (filenameBase = (e.currentTarget as HTMLInputElement).value)}
            placeholder={readonlyExt ? `name without .${readonlyExt}` : "portrait"} />
          {#if readonlyExt}<span class="asset-filename-ext">.{readonlyExt}</span>{/if}
        </div>
        <small class="asset-dialog-hint"
          >Extension <strong>.{readonlyExt || "—"}</strong> is preserved and cannot be changed. Renaming keeps the same file
          content and identity, only the portable path changes.</small>
      </label>

      <label class="asset-dialog-field">
        <span>Role</span>
        <select
          value={role}
          disabled={!editable || (!canBeProfile && role !== "profile")}
          onchange={(e) => (role = (e.currentTarget as HTMLSelectElement).value as any)}>
          <option value="attachment">Attachment</option>
          <option value="profile" disabled={!canBeProfile}>Main file (profile) — image only</option>
        </select>
        {#if !canBeProfile}
          <small class="asset-dialog-hint">Only PNG, JPEG, GIF, and WebP can be used as main file.</small>
        {/if}
      </label>

      <label class="asset-dialog-field asset-dialog-checkbox">
        <input
          type="checkbox"
          checked={referenceScope === "project"}
          disabled={!editable}
          onchange={(e) => (referenceScope = (e.currentTarget as HTMLInputElement).checked ? "project" : "entity")} />
        <span
          >Allow references from other entities/modules <small
            >When enabled, other modules may offer this file as a reference target.</small
          ></span>
      </label>

      {#if saveError}<p class="asset-dialog-error" role="alert">{saveError}</p>{/if}

      <div class="asset-dialog-actions">
        <div class="asset-dialog-actions-left">
          <button
            type="button"
            class="asset-dialog-secondary"
            onclick={onClose}
            disabled={saving || deleting || replacing}>Cancel</button>
          {#if editable}<button
              type="button"
              class="asset-dialog-danger"
              onclick={handleDelete}
              disabled={saving || deleting || replacing}>
              {deleting ? "Deleting…" : "Delete file"}
            </button>{/if}
        </div>
        <div class="asset-dialog-actions-right">
          {#if editable}<button
              type="button"
              class="asset-dialog-secondary"
              onclick={handleReplace}
              disabled={saving || deleting || replacing}>
              {replacing ? "Replacing…" : "Replace file"}
            </button>{/if}
          {#if editable}<button
              type="submit"
              class="asset-dialog-primary"
              disabled={saving || deleting || replacing || !hasChanges()}>
              {saving ? "Saving…" : "Save changes"}
            </button>{/if}
        </div>
      </div>
      {#if deleteError}<p class="asset-dialog-error" role="alert">{deleteError}</p>{/if}
      {#if replaceError}<p class="asset-dialog-error" role="alert">{replaceError}</p>{/if}
    </form>
  </div>
</div>

<style>
.asset-dialog-backdrop {
  position: fixed;
  z-index: 85;
  inset: 0;
  display: grid;
  place-items: center;
  padding: 18px;
  background: rgba(37, 37, 31, 0.32);
}
.asset-dialog {
  width: min(560px, 100%);
  max-height: min(720px, calc(100vh - 36px));
  overflow-y: auto;
  padding: 22px;
  border: 1px solid var(--line);
  border-radius: 14px;
  background: var(--surface);
  box-shadow: 0 24px 70px rgba(38, 42, 33, 0.25);
  outline: none;
}
.asset-dialog-header {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 12px;
}
.asset-dialog-kicker {
  display: block;
  color: var(--accent);
  font-size: 9px;
  font-weight: 700;
  letter-spacing: 0.09em;
  text-transform: uppercase;
}
.asset-dialog-header h2 {
  margin: 4px 0 0;
  color: var(--ink);
  font: 700 21px/1.2 var(--font-display, Georgia, serif);
}
.asset-dialog-header p {
  margin: 6px 0 0;
  color: var(--ink-soft);
  font-size: 12px;
  line-height: 1.5;
}
.asset-dialog-close {
  width: 30px;
  height: 30px;
  flex: none;
  border: 0;
  border-radius: 7px;
  background: var(--surface-muted);
  color: var(--ink-soft);
  font-size: 20px;
  line-height: 1;
  cursor: pointer;
}
.asset-dialog-close:hover,
.asset-dialog-close:focus-visible {
  background: var(--theme-warning-bg, #ebe6dd);
  color: var(--ink);
  outline: 2px solid rgba(180, 119, 63, 0.2);
  outline-offset: 1px;
}
.asset-dialog-preview {
  margin-top: 14px;
  border: 1px solid var(--line);
  border-radius: 10px;
  overflow: hidden;
  background: var(--canvas);
  display: grid;
  place-items: center;
  min-height: 120px;
}
.asset-dialog-preview-image {
  display: block;
  width: 100%;
  max-height: 280px;
  object-fit: contain;
  background: var(--canvas);
}
.asset-dialog-preview-fallback {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 16px;
  color: var(--ink-soft);
  font-size: 12px;
}
.asset-dialog-preview-icon {
  display: grid;
  place-items: center;
  width: 36px;
  height: 36px;
  border-radius: 8px;
  background: var(--theme-warning-bg, #ede9e0);
  color: var(--accent);
  font-size: 16px;
}
.asset-dialog-meta {
  margin-top: 14px;
  padding: 12px;
  border: 1px solid var(--line);
  border-radius: 9px;
  background: var(--canvas);
  display: grid;
  gap: 4px;
}
.asset-dialog-meta small {
  color: var(--ink-soft);
  font-size: 11px;
  word-break: break-all;
}
.asset-dialog-form {
  display: grid;
  gap: 14px;
  margin-top: 18px;
}
.asset-dialog-field {
  display: grid;
  gap: 6px;
  color: var(--ink);
  font-size: 12px;
  font-weight: 650;
}
.asset-dialog-field > span {
  color: var(--ink-soft);
  font-size: 10px;
  letter-spacing: 0.04em;
  text-transform: uppercase;
}
.asset-dialog-field > span b {
  color: var(--accent);
}
.asset-dialog-field input[type="text"],
.asset-dialog-field select {
  width: 100%;
  min-height: 38px;
  padding: 8px 10px;
  border: 1px solid var(--line);
  border-radius: 7px;
  background: var(--canvas);
  color: var(--ink);
  font-size: 13px;
  outline: none;
}
.asset-dialog-field input[type="text"]:focus,
.asset-dialog-field select:focus {
  border-color: var(--accent-soft);
  box-shadow: 0 0 0 3px rgba(180, 119, 63, 0.1);
}
.asset-filename-row {
  display: flex;
  align-items: center;
  gap: 8px;
}
.asset-filename-row input {
  flex: 1;
}
.asset-filename-ext {
  display: grid;
  place-items: center;
  min-height: 38px;
  padding: 0 12px;
  border: 1px solid var(--line);
  border-radius: 7px;
  background: var(--surface-muted);
  color: var(--ink-faint);
  font-size: 12px;
  font-weight: 600;
  white-space: nowrap;
}
.asset-dialog-checkbox {
  grid-template-columns: 18px 1fr;
  align-items: start;
  gap: 8px;
  font-weight: 500;
  text-transform: none;
  letter-spacing: 0;
}
.asset-dialog-checkbox input {
  width: 18px;
  height: 18px;
  margin-top: 2px;
  accent-color: var(--accent-dark);
}
.asset-dialog-checkbox span {
  color: var(--ink);
  font-size: 12px;
  letter-spacing: 0;
  text-transform: none;
}
.asset-dialog-checkbox small {
  display: block;
  margin-top: 4px;
  color: var(--ink-soft);
  font-size: 11px;
  line-height: 1.4;
}
.asset-dialog-hint {
  color: var(--ink-faint);
  font-size: 11px;
}
.asset-dialog-error {
  color: var(--theme-danger-text, #a1482f);
  font-size: 11px;
  line-height: 1.4;
}
.asset-dialog-actions {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  margin-top: 6px;
  flex-wrap: wrap;
}
.asset-dialog-actions-left,
.asset-dialog-actions-right {
  display: flex;
  gap: 8px;
  align-items: center;
}
.asset-dialog-secondary,
.asset-dialog-primary,
.asset-dialog-danger {
  padding: 9px 14px;
  border-radius: 8px;
  font-size: 12px;
  font-weight: 650;
  cursor: pointer;
}
.asset-dialog-secondary {
  border: 1px solid var(--line);
  background: transparent;
  color: var(--ink-soft);
}
.asset-dialog-primary {
  border: 1px solid var(--accent-dark);
  background: var(--accent-dark);
  color: #fff;
}
.asset-dialog-danger {
  border: 1px solid var(--theme-danger-border, #a1482f);
  background: transparent;
  color: var(--theme-danger-text, #a1482f);
}
.asset-dialog-primary:disabled,
.asset-dialog-secondary:disabled,
.asset-dialog-danger:disabled {
  opacity: 0.55;
  cursor: not-allowed;
}
@media (max-width: 520px) {
  .asset-dialog-backdrop {
    align-items: end;
    padding: 10px;
  }
  .asset-dialog {
    max-height: calc(100vh - 20px);
    padding: 18px;
    border-radius: 12px 12px 8px 8px;
  }
  .asset-dialog-actions {
    flex-direction: column;
    align-items: stretch;
  }
  .asset-dialog-actions-left,
  .asset-dialog-actions-right {
    justify-content: stretch;
  }
  .asset-dialog-actions-left button,
  .asset-dialog-actions-right button {
    flex: 1;
  }
}
</style>
