<script lang="ts">
import { ChevronRight, Eye, EyeOff, Hexagon, Layers } from "@lucide/svelte";

type VisibilityLayer = {
  id: string;
  name: string;
  enabled: boolean;
  meta?: string;
  overlay?: boolean;
};

type VisibilityGroup = {
  id: string;
  label: string;
  layers: VisibilityLayer[];
};

let {
  layers = [],
  groups = null,
  onToggle,
  onSelect,
  activeId = null,
  variant = "surface",
  collapsible = true,
  defaultCollapsed = false,
  label = "Layers",
}: {
  layers?: VisibilityLayer[];
  groups?: VisibilityGroup[] | null;
  onToggle: (id: string) => void;
  onSelect?: (id: string) => void;
  activeId?: string | null;
  variant?: "surface" | "studio";
  collapsible?: boolean;
  defaultCollapsed?: boolean;
  label?: string;
} = $props();

// svelte-ignore state_referenced_locally
let collapsed = $state(defaultCollapsed);
const flatLayers = $derived(groups?.flatMap((group) => group.layers) ?? layers);
const enabledCount = $derived(flatLayers.filter((layer) => layer.enabled).length);
</script>

{#snippet layerList(items: VisibilityLayer[], listLabel: string)}
  <ul class="layer-visibility-list" class:studio={variant === "studio"} role="list" aria-label={listLabel}>
    {#each items as layer (layer.id)}
      <li class="layer-row" class:hidden={!layer.enabled} class:active={layer.id === activeId}>
        <span class="layer-kind-icon" aria-hidden="true">
          {#if layer.overlay}<Hexagon size={13} strokeWidth={1.8} />{:else}<Layers size={13} strokeWidth={1.8} />{/if}
        </span>
        {#if onSelect && layer.overlay}
          <button
            type="button"
            class="layer-name"
            aria-pressed={layer.id === activeId}
            onclick={() => onSelect(layer.id)}>
            <span>{layer.name}</span>
            {#if layer.meta}<small>{layer.meta}</small>{/if}
          </button>
        {:else}
          <span class="layer-name">
            <span>{layer.name}</span>
            {#if layer.meta}<small>{layer.meta}</small>{/if}
          </span>
        {/if}
        <button
          type="button"
          class="mini-icon"
          class:off={!layer.enabled}
          aria-pressed={layer.enabled}
          aria-label={layer.enabled ? `Hide ${layer.name}` : `Show ${layer.name}`}
          title={layer.enabled ? "Hide" : "Show"}
          onclick={() => onToggle(layer.id)}>
          {#if layer.enabled}<Eye size={14} strokeWidth={1.8} />{:else}<EyeOff size={14} strokeWidth={1.8} />{/if}
        </button>
      </li>
    {/each}
  </ul>
{/snippet}

{#if groups}
  <div class="layer-book" class:studio={variant === "studio"}>
    {#each groups as group (group.id)}
      <section class="layer-book-group" aria-label={group.label}>
        <div class="layer-book-head">
          <strong>{group.label}</strong>
          <span class="section-count"
            >{group.layers.filter((layer) => layer.enabled).length}/{group.layers.length}</span>
        </div>
        {#if group.layers.length === 0}
          <p class="empty-note">None yet.</p>
        {:else}
          {@render layerList(group.layers, group.label)}
        {/if}
      </section>
    {/each}
  </div>
{:else if collapsible}
  <details class="layer-section" class:studio={variant === "studio"} open={!collapsed}>
    <summary
      onclick={(event) => {
        event.preventDefault();
        collapsed = !collapsed;
      }}>
      <ChevronRight size={14} strokeWidth={1.8} aria-hidden="true" />
      <strong>{label}</strong>
      <span class="section-count">{enabledCount}/{layers.length}</span>
    </summary>
    <div class="section-body">
      {@render layerList(layers, label)}
    </div>
  </details>
{:else}
  {@render layerList(layers, label)}
{/if}

<style>
.layer-section {
  display: block;
}
.layer-section summary {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 4px 2px 8px;
  list-style: none;
  cursor: pointer;
  user-select: none;
}
.layer-section summary::-webkit-details-marker {
  display: none;
}
.layer-section summary :global(svg) {
  flex: 0 0 auto;
  color: var(--ink-faint, #aaa79d);
  transition: transform 0.15s ease;
}
.layer-section[open] summary :global(svg) {
  transform: rotate(90deg);
}
.layer-section summary strong {
  flex: 1;
  color: var(--ink);
  font-size: 11px;
  font-weight: 700;
  letter-spacing: 0.04em;
  text-transform: uppercase;
}
.layer-section summary:hover strong {
  color: var(--accent-dark, #2f4e35);
}
.section-count {
  padding: 2px 6px;
  border-radius: 999px;
  background: var(--surface-muted, #f4f2ec);
  color: var(--ink-faint, #aaa79d);
  font-size: 10px;
  font-weight: 700;
  font-variant-numeric: tabular-nums;
}
.layer-section[open] .section-count {
  background: var(--accent-dark, #2f4e35);
  color: var(--on-accent, #f7f6f2);
}
.section-body {
  display: grid;
  gap: 4px;
}
.layer-section.studio summary :global(svg) {
  color: #aebdb1;
}
.layer-section.studio summary strong {
  color: #edf2ec;
}
.layer-section.studio summary:hover strong {
  color: #f4f1ea;
}
.layer-section.studio .section-count {
  background: rgb(255 255 255 / 8%);
  color: #b8c8bc;
}
.layer-section.studio[open] .section-count {
  background: #c9a96e;
  color: #1b2822;
}
.layer-book {
  display: grid;
  gap: 12px;
}
.layer-book-group {
  display: grid;
  gap: 6px;
}
.layer-book-head {
  display: flex;
  align-items: center;
  gap: 6px;
}
.layer-book-head strong {
  flex: 1;
  color: var(--ink-soft, #77766d);
  font-size: 10px;
  font-weight: 700;
  letter-spacing: 0.06em;
  text-transform: uppercase;
}
.layer-book.studio .layer-book-head strong {
  color: #b8c8bc;
}
.layer-book.studio .section-count {
  background: rgb(255 255 255 / 8%);
  color: #b8c8bc;
}
.empty-note {
  margin: 0;
  color: var(--ink-faint, #aaa79d);
  font-size: 11px;
}
.layer-book.studio .empty-note {
  color: #aebdb1;
}
.layer-visibility-list {
  display: grid;
  gap: 4px;
  margin: 0;
  padding: 0;
  list-style: none;
}
.layer-row {
  display: grid;
  grid-template-columns: auto minmax(0, 1fr) auto;
  align-items: center;
  gap: 8px;
  padding: 6px 8px;
  border: 1px solid var(--line, #e4e1d8);
  border-radius: 8px;
  background: var(--surface, #fffefa);
}
.layer-row.hidden {
  opacity: 0.72;
}
.layer-row.active {
  border-color: var(--accent-soft, #c99965);
  box-shadow: 0 0 0 1px color-mix(in srgb, var(--accent-soft, #c99965) 35%, transparent);
}
.layer-kind-icon {
  display: grid;
  width: 24px;
  height: 24px;
  place-items: center;
  border-radius: 6px;
  background: var(--surface-muted, #f4f2ec);
  color: var(--ink-soft, #77766d);
}
.layer-name {
  display: grid;
  min-width: 0;
  overflow: hidden;
  text-align: left;
  background: none;
  border: 0;
  padding: 0;
  color: var(--ink);
  font-weight: 600;
  font-size: 11px;
  line-height: 1.2;
  cursor: default;
}
button.layer-name {
  cursor: pointer;
}
.layer-name span,
.layer-name small {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.layer-name small {
  color: var(--ink-faint, #aaa79d);
  font-weight: 500;
  font-size: 10px;
}
.mini-icon {
  display: grid;
  place-items: center;
  width: 28px;
  height: 28px;
  padding: 0;
  border: 1px solid var(--line, #e4e1d8);
  border-radius: 7px;
  background: var(--surface, #fffefa);
  color: var(--ink-soft, #77766d);
  cursor: pointer;
}
.mini-icon:hover:not(:disabled) {
  border-color: var(--line-strong, #d9cdbd);
  background: var(--surface-muted, #f4f2ec);
  color: var(--ink);
}
.mini-icon.off {
  opacity: 0.55;
}
.mini-icon:disabled {
  opacity: 0.4;
  cursor: not-allowed;
}
.studio .layer-row {
  border-color: var(--theme-neutral-border-strong, #405047);
  background: rgb(255 255 255 / 4%);
}
.studio .layer-kind-icon {
  background: rgb(255 255 255 / 8%);
  color: #c5d4c8;
}
.studio .layer-name {
  color: #edf2ec;
}
.studio .layer-name small {
  color: #aebdb1;
}
.studio .mini-icon {
  border-color: rgb(255 255 255 / 16%);
  background: rgb(255 255 255 / 6%);
  color: #d9d0c3;
}
.studio .mini-icon:hover:not(:disabled) {
  border-color: rgb(255 255 255 / 28%);
  background: rgb(255 255 255 / 10%);
  color: #fff;
}
</style>
