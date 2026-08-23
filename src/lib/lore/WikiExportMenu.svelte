<script lang="ts">
import { onMount, tick } from "svelte";
import { ChevronDown, Code2, Download, FileText, LoaderCircle, Printer } from "@lucide/svelte";
import { project, type WikiPageExportFormat } from "$lib/project/client";

let {
  entityId,
  manifestId,
  articleName,
}: {
  entityId: string;
  manifestId: string;
  articleName: string;
} = $props();

let open = $state(false);
let busy = $state<WikiPageExportFormat | null>(null);
let message = $state("");
let error = $state("");
let menuRoot = $state<HTMLDivElement | null>(null);

onMount(() => {
  const handlePointerDown = (event: PointerEvent) => {
    if (open && event.target instanceof Node && !menuRoot?.contains(event.target)) open = false;
  };
  const handleKeydown = (event: KeyboardEvent) => {
    if (event.key === "Escape") open = false;
  };
  window.addEventListener("pointerdown", handlePointerDown, true);
  window.addEventListener("keydown", handleKeydown, true);
  return () => {
    window.removeEventListener("pointerdown", handlePointerDown, true);
    window.removeEventListener("keydown", handleKeydown, true);
  };
});

async function exportPage(format: WikiPageExportFormat) {
  if (busy) return;
  open = false;
  busy = format;
  message = "";
  error = "";
  try {
    const selection = await project.pickDirectory();
    const destination = typeof selection === "string" ? selection : null;
    if (!destination) return;
    const output = await project.exportWikiPage(entityId, destination, format, manifestId);
    message = `${articleName} exported to ${output}`;
  } catch (cause) {
    error = cause instanceof Error ? cause.message : String(cause);
  } finally {
    busy = null;
  }
}

async function printPage() {
  open = false;
  message = "";
  error = "";
  await tick();
  window.print();
}
</script>

<div bind:this={menuRoot} class="wiki-export">
  <button
    class="export-trigger"
    type="button"
    aria-haspopup="menu"
    aria-expanded={open}
    disabled={busy !== null}
    onclick={() => (open = !open)}>
    {#if busy}<LoaderCircle class="spinner" size={15} strokeWidth={1.8} />{:else}<Download
        size={15}
        strokeWidth={1.8} />{/if}
    <span>{busy ? "Exporting…" : "Export"}</span>
    <ChevronDown size={13} strokeWidth={1.8} />
  </button>

  {#if open}
    <div class="export-menu" role="menu" aria-label="Export article">
      <button type="button" role="menuitem" onclick={() => void exportPage("markdown")}>
        <span class="format-icon"><FileText size={16} strokeWidth={1.7} /></span>
        <span><strong>Markdown</strong><small>Portable source with details and connections</small></span>
      </button>
      <button type="button" role="menuitem" onclick={() => void exportPage("html")}>
        <span class="format-icon"><Code2 size={16} strokeWidth={1.7} /></span>
        <span><strong>Standalone HTML</strong><small>Styled page with no remote dependencies</small></span>
      </button>
      <button type="button" role="menuitem" onclick={() => void printPage()}>
        <span class="format-icon"><Printer size={16} strokeWidth={1.7} /></span>
        <span><strong>PDF / Print</strong><small>Open the system dialog to save as PDF</small></span>
      </button>
    </div>
  {/if}
</div>

{#if message}<p class="export-status" role="status">{message}</p>{/if}
{#if error}<p class="export-status error" role="alert">{error}</p>{/if}

<style>
.wiki-export {
  position: relative;
}
.export-trigger {
  display: inline-flex;
  align-items: center;
  gap: 7px;
  min-height: 36px;
  padding: 0 11px;
  border: 1px solid #d5d8d0;
  border-radius: 9px;
  background: #fff;
  color: #38443a;
  font: 650 12px/1 var(--font-body, Inter, ui-sans-serif, system-ui, sans-serif);
  cursor: pointer;
}
.export-trigger:hover,
.export-trigger:focus-visible {
  border-color: #aeb9af;
  background: #f7faf7;
}
.export-trigger:disabled {
  opacity: 0.65;
  cursor: wait;
}
.export-menu {
  position: absolute;
  z-index: 30;
  top: calc(100% + 8px);
  right: 0;
  display: grid;
  width: min(330px, calc(100vw - 32px));
  padding: 6px;
  border: 1px solid #dce0d8;
  border-radius: 12px;
  background: #fff;
  box-shadow: 0 18px 48px rgba(34, 40, 34, 0.16);
}
.export-menu button {
  display: grid;
  grid-template-columns: 34px minmax(0, 1fr);
  align-items: center;
  gap: 9px;
  padding: 10px;
  border: 0;
  border-radius: 8px;
  background: transparent;
  color: #272d28;
  text-align: left;
  cursor: pointer;
}
.export-menu button:hover,
.export-menu button:focus-visible {
  background: #f3f6f2;
}
.format-icon {
  display: grid;
  place-items: center;
  width: 32px;
  height: 32px;
  border-radius: 8px;
  background: #e9f0e8;
  color: #49634e;
}
.export-menu strong,
.export-menu small {
  display: block;
}
.export-menu strong {
  font-size: 12px;
}
.export-menu small {
  margin-top: 3px;
  color: #7d847d;
  font-size: 10px;
  line-height: 1.35;
}
.export-status {
  position: fixed;
  z-index: 40;
  right: 22px;
  bottom: 22px;
  max-width: min(520px, calc(100vw - 44px));
  margin: 0;
  padding: 10px 13px;
  border: 1px solid #c9dacb;
  border-radius: 9px;
  background: #edf7ee;
  color: #35563c;
  box-shadow: 0 10px 32px rgba(34, 40, 34, 0.13);
  font-size: 11px;
  overflow-wrap: anywhere;
}
.export-status.error {
  border-color: #eccbc4;
  background: #fff1ee;
  color: #984b3b;
}
:global(.spinner) {
  animation: spin 0.8s linear infinite;
}
@keyframes spin {
  to {
    transform: rotate(360deg);
  }
}
@media print {
  .wiki-export,
  .export-status {
    display: none !important;
  }
}
</style>
