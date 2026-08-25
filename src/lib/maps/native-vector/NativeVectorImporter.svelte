<script lang="ts">
import { onMount } from "svelte";
import { FileJson, Image, Maximize2, Minimize2 } from "@lucide/svelte";
import type { Entity } from "$lib/project/client";
import { project } from "$lib/project/client";
import WorkspaceTopbar from "$lib/layout/WorkspaceTopbar.svelte";

let {
  mode,
  oncreated,
  oncancel,
  onfullscreen,
  fullscreen = false,
}: {
  mode: "image" | "geojson";
  oncreated?: (map: Entity) => void;
  oncancel?: () => void;
  onfullscreen?: (enabled: boolean) => void;
  fullscreen?: boolean;
} = $props();

let accepting = $state(false);
let message = $state("");
const brandIcon = $derived(mode === "image" ? Image : FileJson);
const iconProps = { size: 15, strokeWidth: 1.8, "aria-hidden": true } as const;

function cancel() {
  oncancel?.();
}

function toggleFullscreen() {
  onfullscreen?.(!fullscreen);
}

async function importFile() {
  const source =
    mode === "image" ? await project.pickImageMapFile() : await project.pickVectorMapFile();
  if (typeof source !== "string") {
    oncancel?.();
    return;
  }
  accepting = true;
  message = "";
  try {
    const imported =
      mode === "image" ? await project.importImageMapFile(source) : await project.importVectorMapFile(source);
    await oncreated?.(imported.entity);
  } catch (cause) {
    message = cause instanceof Error ? cause.message : String(cause);
  } finally {
    accepting = false;
  }
}

onMount(() => {
  void importFile();
});
</script>

<section
  class="importer"
  aria-label={mode === "geojson" ? "Import a GeoJSON vector map" : "Import an image map"}>
  <WorkspaceTopbar
    title={mode === "geojson" ? "Import vector map" : "Import image"}
    subtitle="Native vector map"
    icon={brandIcon}
    backLabel="Back to map details"
    onBack={cancel}
    actionsLabel="Map import actions">
    <div class="header-actions" data-workspace-topbar-actions>
      {#if accepting}
        <button type="button" class="primary" disabled>Importing…</button>
      {/if}
      <button
        type="button"
        class="icon-button"
        class:active={fullscreen}
        aria-label={fullscreen ? "Exit full screen" : "Full screen"}
        aria-pressed={fullscreen}
        title={fullscreen ? "Exit full screen" : "Full screen"}
        onclick={toggleFullscreen}
        >{#if fullscreen}<Minimize2 {...iconProps} />{:else}<Maximize2 {...iconProps} />{/if}</button>
    </div>
  </WorkspaceTopbar>
  <div class="import-stage" role="status">
    <p>{accepting ? "Importing…" : message || "Choose a file…"}</p>
    {#if message && !accepting}
      <button type="button" class="primary" onclick={() => void importFile()}>Try again</button>
    {/if}
  </div>
</section>

<style>
.importer {
  display: flex;
  min-height: 0;
  flex: 1;
  flex-direction: column;
  background: #17211d;
  color: #edf2ec;
}
.header-actions {
  display: flex;
  gap: 6px;
  align-items: center;
}
.import-stage {
  display: grid;
  place-content: center;
  justify-items: center;
  gap: 12px;
  flex: 1;
  padding: 24px;
  background: #0d1b2a;
  color: #d8e3d9;
  font: 14px/1.4 system-ui;
}
.import-stage p {
  margin: 0;
}
button {
  border: 0;
  border-radius: 7px;
  padding: 8px 10px;
  background: #31443b;
  color: #edf2ec;
  font: 700 12px system-ui;
  cursor: pointer;
}
button.primary {
  background: #d5ab6c;
  color: var(--brass-ink);
}
button:disabled {
  opacity: 0.45;
  cursor: not-allowed;
}
.icon-button {
  display: grid;
  place-items: center;
  width: 32px;
  height: 32px;
  padding: 0;
}
.icon-button.active {
  background: #d5ab6c;
  color: var(--brass-ink);
}
</style>
