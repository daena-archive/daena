<script lang="ts">
import type { WorkspaceLocationView } from "$lib/navigation/history";
import type { WorkspaceSection } from "$lib/modules/workspace";

let {
  section,
  activeView,
  onSelect,
}: {
  section: WorkspaceSection;
  activeView: WorkspaceLocationView;
  onSelect: (view: WorkspaceLocationView) => void;
} = $props();

const views = $derived.by(() => {
  if (section === "lore")
    return [
      { id: "library", label: "Library" },
      { id: "wiki", label: "Wiki" },
      { id: "graph", label: "Graph" },
    ] satisfies { id: WorkspaceLocationView; label: string }[];
  if (section === "timeline")
    return [
      { id: "timeline", label: "Timeline" },
      { id: "events", label: "Events" },
      { id: "eras", label: "Eras" },
      { id: "calendars", label: "Calendars" },
    ] satisfies { id: WorkspaceLocationView; label: string }[];
  if (section === "writing")
    return [
      { id: "manuscripts", label: "Manuscripts" },
      { id: "reference", label: "Reference" },
    ] satisfies { id: WorkspaceLocationView; label: string }[];
  return [];
});
</script>

<nav class="workspace-view-nav" aria-label={`${section} views`}>
  {#each views as view}
    <button
      type="button"
      class:active={activeView === view.id}
      aria-current={activeView === view.id ? "page" : undefined}
      onclick={() => onSelect(view.id)}>{view.label}</button>
  {/each}
</nav>

<style>
.workspace-view-nav {
  position: sticky;
  top: 58px;
  z-index: 3;
  display: flex;
  gap: 4px;
  min-height: 46px;
  padding: 7px 40px;
  border-bottom: 1px solid var(--line);
  background: color-mix(in srgb, var(--surface) 92%, transparent);
  backdrop-filter: blur(14px);
}
.workspace-view-nav button {
  padding: 7px 11px;
  border: 1px solid transparent;
  border-radius: 7px;
  background: transparent;
  color: var(--ink-soft);
  font: inherit;
  font-size: 11px;
  font-weight: 700;
  cursor: pointer;
}
.workspace-view-nav button:hover {
  background: var(--surface-muted);
  color: var(--ink);
}
.workspace-view-nav button.active {
  border-color: var(--theme-warning-border, #d8c3a5);
  background: var(--accent-bg);
  color: var(--accent-dark);
}
.workspace-view-nav + :global(.projection-view-shell),
.workspace-view-nav + :global(.kb-shell) {
  height: calc(100vh - 104px);
}

@media (max-width: 760px) {
  .workspace-view-nav {
    overflow-x: auto;
    padding-inline: 17px;
  }
  .workspace-view-nav button {
    flex: 0 0 auto;
  }
}
</style>
