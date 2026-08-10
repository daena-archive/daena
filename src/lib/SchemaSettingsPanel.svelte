<script lang="ts">
import ModuleSchemaPanel from "$lib/ModuleSchemaPanel.svelte";
import type { EntityTemplate, FieldDefinition, ModuleSchemaOverlay } from "$lib/project/client";
import { allowLeaveSchemaEditor } from "$lib/schemaEditorGuard";

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
</script>

<section class="schema-settings">
  {#if !showEditor}
    <div class="settings-section-heading">
      <strong>Schema</strong>
      <p>Customize templates and fields for plugins that support project overlays.</p>
    </div>

    {#if !projectOpen}
      <p class="settings-empty">Open a project to customize schemas.</p>
    {:else if candidates.length === 0}
      <p class="settings-empty">
        No enabled plugins declare schema customization. Enable a plugin that includes the
        <code>schema.overlay</code> capability, or install one that does.
      </p>
    {:else}
      <ul class="schema-plugin-list">
        {#each candidates as plugin}
          <li>
            <button type="button" class="schema-plugin-card" onclick={() => void selectPlugin(plugin.id)}>
              <strong>{plugin.name}</strong>
              <span>Customize types, fields, and create templates for this plugin.</span>
            </button>
          </li>
        {/each}
      </ul>
    {/if}
  {:else if packageManifest}
    <div class="schema-plugin-toolbar">
      <button type="button" class="quiet-button" onclick={() => void clearSelection()}>All schemas</button>
      <span class="schema-plugin-crumb">{selectedPluginName || selectedPluginId}</span>
      {#if editorDirty}
        <span class="schema-dirty-hint">Unsaved changes</span>
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
  gap: 18px;
}
.settings-section-heading strong {
  display: block;
  font: 500 18px var(--font-display, Georgia, serif);
}
.settings-section-heading p,
.settings-empty {
  margin: 6px 0 0;
  color: var(--ink-soft, #8f897e);
  font-size: 12px;
  line-height: 1.45;
}
.settings-empty code {
  padding: 1px 5px;
  border-radius: 4px;
  background: #f1ebe1;
  font:
    500 11px ui-monospace,
    SFMono-Regular,
    Menlo,
    monospace;
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
  display: grid;
  gap: 5px;
  padding: 15px 16px;
  border: 1px solid var(--line, #e9e1d4);
  border-radius: 12px;
  background: #fffefa;
  color: inherit;
  text-align: left;
  cursor: pointer;
  transition:
    border-color 0.12s ease,
    background 0.12s ease;
}
.schema-plugin-card:hover {
  border-color: #d0c4b2;
  background: #fffcf7;
}
.schema-plugin-card strong {
  font: 600 15px var(--font-display, Georgia, serif);
}
.schema-plugin-card span {
  color: var(--ink-soft, #8f897e);
  font-size: 12px;
  line-height: 1.45;
}
.schema-plugin-toolbar {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 10px;
}
.quiet-button {
  border: 1px solid #d9cdbd;
  border-radius: 8px;
  padding: 6px 10px;
  background: #fffefa;
  color: #62594e;
  font:
    600 11px Inter,
    ui-sans-serif,
    system-ui,
    sans-serif;
  cursor: pointer;
}
.schema-plugin-crumb {
  color: #62594e;
  font: 600 14px var(--font-display, Georgia, serif);
}
.schema-dirty-hint {
  margin-left: auto;
  color: #9a4d3f;
  font:
    600 11px Inter,
    ui-sans-serif,
    system-ui,
    sans-serif;
}
</style>
