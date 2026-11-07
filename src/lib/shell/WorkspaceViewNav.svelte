<script lang="ts">
import type { WorkspaceLocationView } from "$lib/navigation/history";

let {
  label,
  views,
  activeView,
  onSelect,
}: {
  label: string;
  views: { id: WorkspaceLocationView; label: string }[];
  activeView: WorkspaceLocationView;
  onSelect: (view: WorkspaceLocationView) => void;
} = $props();

let nav = $state<HTMLElement | null>(null);

function handleKeydown(event: KeyboardEvent, index: number) {
  if (views.length === 0) return;
  let nextIndex: number | null = null;
  if (event.key === "ArrowRight") nextIndex = (index + 1) % views.length;
  if (event.key === "ArrowLeft") nextIndex = (index - 1 + views.length) % views.length;
  if (event.key === "Home") nextIndex = 0;
  if (event.key === "End") nextIndex = views.length - 1;
  if (nextIndex === null || !views[nextIndex]) return;

  event.preventDefault();
  onSelect(views[nextIndex].id);
  nav?.querySelectorAll<HTMLButtonElement>("button")[nextIndex]?.focus();
}
</script>

<nav bind:this={nav} class="workspace-view-nav" aria-label={label}>
  {#each views as view, index}
    <button
      type="button"
      class:active={activeView === view.id}
      aria-current={activeView === view.id ? "page" : undefined}
      tabindex={activeView === view.id ? 0 : -1}
      onkeydown={(event) => handleKeydown(event, index)}
      data-guide={`workspace-view-${view.id}`}
      onclick={() => onSelect(view.id)}>{view.label}</button>
  {/each}
</nav>

<style>
.workspace-view-nav {
  display: inline-flex;
  gap: 2px;
  max-width: 100%;
  padding: 2px;
  border: 1px solid var(--line);
  border-radius: 8px;
  background: var(--surface-muted);
}
.workspace-view-nav button {
  padding: 4px 10px;
  min-height: 26px;
  border: 1px solid transparent;
  border-radius: 6px;
  background: transparent;
  color: var(--ink-soft);
  font: inherit;
  font-size: 11px;
  font-weight: 700;
  cursor: pointer;
}
.workspace-view-nav button:hover:not(.active) {
  background: var(--surface);
  color: var(--ink);
}
.workspace-view-nav button.active {
  background: var(--accent-dark);
  color: var(--on-accent);
}

@media (max-width: 760px) {
  .workspace-view-nav {
    overflow-x: auto;
  }
  .workspace-view-nav button {
    flex: 0 0 auto;
  }
}
</style>
