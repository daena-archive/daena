<script lang="ts">
import ModuleSchemaPanel from "$lib/ModuleSchemaPanel.svelte";
import type { EntityTemplate, FieldDefinition, ModuleSchemaOverlay } from "$lib/project/client";
import { allowLeaveSchemaEditor } from "$lib/schemaEditorGuard";
import { Puzzle, ChevronLeft, ChevronRight, SlidersHorizontal, Layers, Sparkles } from "@lucide/svelte";

export type SchemaPluginCandidate = {
  id: string;
  name: string;
};

type PackageManifestSlice = {
  schemas: Array<{
    namespace: string;
    entityTypes: string[];
    fields: FieldDefinition[];
  }>;
  templates: EntityTemplate[];
};

let {
  projectOpen,
  candidates = [],
  selectedPluginId = null,
  selectedPluginName = "",
  packageManifest = null,
  overlay,
  overlayRevision = 0,
  busy = false,
  message = "",
  onSelectPlugin,
  onSave,
  onDirtyChange,
}: {
  projectOpen: boolean;
  /** Enabled plugins that declare schema.overlay. */
  candidates?: SchemaPluginCandidate[];
  selectedPluginId?: string | null;
  selectedPluginName?: string;
  /** Packaged (unmerged) schemas/templates for the selected plugin. */
  packageManifest?: PackageManifestSlice | null;
  overlay: ModuleSchemaOverlay;
  overlayRevision?: number;
  busy?: boolean;
  message?: string;
  onSelectPlugin: (id: string | null) => void;
  onSave: (overlay: ModuleSchemaOverlay) => Promise<void>;
  onDirtyChange?: (dirty: boolean) => void;
} = $props();

let editorDirty = $state(false);

const selectedInList = $derived(candidates.some((plugin) => plugin.id === selectedPluginId));
const showEditor = $derived(Boolean(selectedPluginId && selectedInList && packageManifest));

async function selectPlugin(id: string) {
  if (!candidates.some((plugin) => plugin.id === id)) return;
  if (selectedPluginId && selectedPluginId !== id && !(await allowLeaveSchemaEditor())) return;
  onSelectPlugin(id);
}

async function clearSelection() {
  if (!(await allowLeaveSchemaEditor())) return;
  editorDirty = false;
  onDirtyChange?.(false);
  onSelectPlugin(null);
}

function handleDirtyChange(next: boolean) {
  editorDirty = next;
  onDirtyChange?.(next);
}

function pluginInitial(name: string) {
  return (name.trim()[0] ?? "?").toUpperCase();
}
</script>

<section class="schema-settings">
  {#if !showEditor}
    <div class="settings-section-heading">
      <div class="heading-icon">
        <SlidersHorizontal size={16} strokeWidth={1.8} aria-hidden="true" />
      </div>
      <div class="heading-copy">
        <span class="kicker">PROJECT OVERLAYS</span>
        <strong>Schema</strong>
        <p>
          Extend installed plugins without forking them. Disable built-ins you don’t need and add your own types,
          fields, and create-templates — stored as a project overlay.
        </p>
      </div>
    </div>

    {#if !projectOpen}
      <div class="empty-state">
        <div class="empty-icon">
          <Layers size={20} strokeWidth={1.7} aria-hidden="true" />
        </div>
        <strong>Open a project to customize schemas</strong>
        <p>Schema overlays are saved inside the project’s folder and travel with the project.</p>
      </div>
    {:else if candidates.length === 0}
      <div class="empty-state">
        <div class="empty-icon">
          <Puzzle size={20} strokeWidth={1.7} aria-hidden="true" />
        </div>
        <strong>No customizable plugins found</strong>
        <p>
          No enabled plugin declares <code>schema.overlay</code>. Enable a plugin that supports overlays — like
          <strong>Lore</strong> — or install one from Settings → Plugins.
        </p>
      </div>
    {:else}
      <div class="plugin-grid-meta">
        <span class="meta-count">
          <Sparkles size={12} strokeWidth={1.8} aria-hidden="true" />
          {candidates.length}
          {candidates.length === 1 ? "plugin" : "plugins"} customizable
        </span>
      </div>
      <ul class="schema-plugin-list">
        {#each candidates as plugin}
          <li>
            <button type="button" class="schema-plugin-card" onclick={() => void selectPlugin(plugin.id)}>
              <span class="card-icon" aria-hidden="true">{pluginInitial(plugin.name)}</span>
              <span class="card-copy">
                <strong>{plugin.name}</strong>
                <span class="card-desc"
                  >Customize types, fields, and create-templates. Built-ins stay intact — toggle what you need.</span>
                <span class="card-id">{plugin.id}</span>
              </span>
              <span class="card-arrow" aria-hidden="true">
                <ChevronRight size={16} strokeWidth={1.8} />
              </span>
            </button>
          </li>
        {/each}
      </ul>
    {/if}
  {:else if packageManifest}
    <div class="schema-plugin-toolbar">
      <button type="button" class="crumb-button" onclick={() => void clearSelection()}>
        <ChevronLeft size={14} strokeWidth={1.9} aria-hidden="true" />
        All schemas
      </button>
      <span class="crumb-divider" aria-hidden="true">/</span>
      <span class="schema-plugin-crumb" title={selectedPluginId ?? ""}>
        <span class="crumb-icon">{pluginInitial(selectedPluginName || selectedPluginId || "?")}</span>
        {selectedPluginName || selectedPluginId}
      </span>
      {#if editorDirty}
        <span class="schema-dirty-hint"><span class="dirty-dot" aria-hidden="true"></span> Unsaved changes</span>
      {:else}
        <span class="schema-clean-hint">No unsaved changes</span>
      {/if}
    </div>

    {#key `${selectedPluginId}:${overlayRevision}`}
      <ModuleSchemaPanel
        {projectOpen}
        {packageManifest}
        {overlay}
        {busy}
        {message}
        {onSave}
        onDirtyChange={handleDirtyChange} />
    {/key}
  {/if}
</section>

<style>
.schema-settings {
  display: grid;
  gap: 20px;
}
.settings-section-heading {
  display: flex;
  gap: 14px;
  align-items: flex-start;
  padding: 16px 16px 14px;
  border: 1px solid var(--line);
  border-radius: 14px;
  background: var(--surface);
}
.heading-icon {
  flex: 0 0 36px;
  width: 36px;
  height: 36px;
  display: grid;
  place-items: center;
  border-radius: 10px;
  background: var(--accent-dark);
  color: var(--on-accent);
  box-shadow: 0 1px 0 rgba(48, 44, 38, 0.16);
}
.heading-copy {
  min-width: 0;
}
.kicker {
  display: inline-block;
  color: var(--accent);
  font:
    700 10px/1 Inter,
    ui-sans-serif,
    system-ui,
    sans-serif;
  letter-spacing: 0.08em;
  text-transform: uppercase;
}
.settings-section-heading strong {
  display: block;
  margin-top: 4px;
  color: var(--ink);
  font: 600 18px/1.1 var(--font-display, Georgia, serif);
  letter-spacing: -0.01em;
}
.settings-section-heading p {
  margin: 7px 0 0;
  color: var(--ink-soft);
  font:
    400 12.5px/1.55 Inter,
    ui-sans-serif,
    system-ui,
    sans-serif;
}
.empty-state {
  display: grid;
  gap: 10px;
  justify-items: start;
  padding: 22px 18px;
  border: 1px dashed var(--line-strong);
  border-radius: 14px;
  background: var(--surface-quiet);
}
.empty-icon {
  width: 36px;
  height: 36px;
  display: grid;
  place-items: center;
  border-radius: 999px;
  background: var(--surface-warm);
  color: var(--ink-muted);
  border: 1px solid var(--line-soft);
}
.empty-state strong {
  color: var(--ink);
  font: 600 14px/1.2 var(--font-display, Georgia, serif);
}
.empty-state p {
  margin: 0;
  max-width: 560px;
  color: var(--ink-soft);
  font:
    400 12.5px/1.5 Inter,
    ui-sans-serif,
    system-ui,
    sans-serif;
}
.empty-state code {
  padding: 1px 6px;
  border-radius: 6px;
  background: var(--theme-warning-bg, #f1ebe1);
  border: 1px solid var(--line-soft);
  color: var(--theme-neutral-text-soft, #6f675c);
  font:
    500 11px ui-monospace,
    SFMono-Regular,
    Menlo,
    monospace;
}
.plugin-grid-meta {
  display: flex;
  flex-wrap: wrap;
  gap: 10px 16px;
  align-items: center;
  justify-content: space-between;
  padding: 0 2px;
}
.meta-count {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  padding: 5px 9px;
  border-radius: 999px;
  background: var(--surface-warm);
  border: 1px solid var(--line-soft);
  color: var(--ink-muted);
  font:
    600 11px Inter,
    ui-sans-serif,
    system-ui,
    sans-serif;
}
.schema-plugin-list {
  display: grid;
  gap: 10px;
  margin: 0;
  padding: 0;
  list-style: none;
}
.schema-plugin-card {
  width: 100%;
  display: flex;
  align-items: center;
  gap: 14px;
  padding: 14px 14px 14px 16px;
  border: 1px solid var(--line);
  border-radius: 14px;
  background: var(--surface);
  text-align: left;
  cursor: pointer;
  transition:
    border-color 0.16s ease,
    background 0.16s ease,
    box-shadow 0.16s ease,
    transform 0.16s ease;
}
.schema-plugin-card:hover,
.schema-plugin-card:focus-visible {
  border-color: var(--theme-warning-border, #cbbda9);
  background: var(--surface-quiet);
  box-shadow: 0 8px 24px rgba(48, 44, 38, 0.08);
  transform: translateY(-1px);
  outline: none;
}
.schema-plugin-card:active {
  transform: translateY(0);
  box-shadow: 0 2px 10px rgba(48, 44, 38, 0.06);
}
.card-icon {
  flex: 0 0 40px;
  width: 40px;
  height: 40px;
  display: grid;
  place-items: center;
  border-radius: 11px;
  background: var(--surface-warm);
  border: 1px solid var(--line-soft);
  color: var(--ink-muted);
  font: 700 14px var(--font-display, Georgia, serif);
}
.card-copy {
  flex: 1;
  min-width: 0;
  display: grid;
  gap: 3px;
}
.card-copy strong {
  color: var(--ink);
  font: 600 14.5px/1.15 var(--font-display, Georgia, serif);
}
.card-desc {
  color: var(--ink-soft);
  font:
    400 12px/1.45 Inter,
    ui-sans-serif,
    system-ui,
    sans-serif;
  display: -webkit-box;
  line-clamp: 2;
  -webkit-line-clamp: 2;
  -webkit-box-orient: vertical;
  overflow: hidden;
}
.card-id {
  color: var(--ink-faint);
  font:
    500 11px ui-monospace,
    SFMono-Regular,
    Menlo,
    monospace;
}
.card-arrow {
  flex: 0 0 28px;
  width: 28px;
  height: 28px;
  display: grid;
  place-items: center;
  border-radius: 999px;
  background: var(--theme-surface-bg, #fff);
  border: 1px solid var(--line-soft);
  color: var(--ink-faint);
}
.schema-plugin-card:hover .card-arrow {
  border-color: var(--theme-warning-border, #cbbda9);
  color: var(--ink-muted);
  background: var(--theme-warning-bg, #f7f1e7);
}
.schema-plugin-toolbar {
  position: sticky;
  top: 0;
  z-index: 3;
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 10px;
  margin: -6px -4px 0;
  padding: 10px 12px;
  border: 1px solid var(--line);
  border-radius: 12px;
  background: rgba(255, 254, 250, 0.92);
  backdrop-filter: blur(10px);
  -webkit-backdrop-filter: blur(10px);
  box-shadow: 0 4px 18px rgba(48, 44, 38, 0.06);
}
.crumb-button {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  padding: 6px 10px;
  border: 1px solid var(--line-strong);
  border-radius: 999px;
  background: var(--surface);
  color: var(--ink-muted);
  font:
    600 11.5px Inter,
    ui-sans-serif,
    system-ui,
    sans-serif;
  cursor: pointer;
}
.crumb-button:hover,
.crumb-button:focus-visible {
  border-color: var(--theme-warning-border, #b7a88f);
  background: var(--surface-warm);
  outline: none;
}
.crumb-divider {
  color: #cbbda9;
  font:
    300 14px Inter,
    sans-serif;
}
.schema-plugin-crumb {
  display: inline-flex;
  align-items: center;
  gap: 8px;
  color: var(--ink);
  font: 600 14px var(--font-display, Georgia, serif);
}
.crumb-icon {
  width: 22px;
  height: 22px;
  display: grid;
  place-items: center;
  border-radius: 999px;
  background: var(--accent-dark);
  color: var(--on-accent);
  font: 700 11px var(--font-display, Georgia, serif);
}
.schema-dirty-hint,
.schema-clean-hint {
  margin-left: auto;
  display: inline-flex;
  align-items: center;
  gap: 7px;
  padding: 5px 9px;
  border-radius: 999px;
  border: 1px solid transparent;
  font:
    600 11px Inter,
    ui-sans-serif,
    system-ui,
    sans-serif;
}
.schema-dirty-hint {
  background: var(--theme-danger-bg, #f8ece8);
  border-color: var(--danger-line);
  color: var(--danger);
}
.dirty-dot {
  width: 7px;
  height: 7px;
  border-radius: 999px;
  background: #c35a46;
  box-shadow: 0 0 0 4px rgba(195, 90, 70, 0.14);
}
.schema-clean-hint {
  background: var(--surface-warm);
  border-color: var(--line-soft);
  color: var(--ink-muted);
}
@media (max-width: 640px) {
  .settings-section-heading {
    padding: 14px 14px 13px;
  }
  .schema-plugin-toolbar {
    gap: 8px;
  }
  .schema-dirty-hint,
  .schema-clean-hint {
    margin-left: 0;
  }
}
</style>
