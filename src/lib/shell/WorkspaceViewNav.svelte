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
  min-height: var(--control-min-height);
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
.workspace-view-nav + :global(.kb-shell),
.workspace-view-nav + :global(.specialized-surface) {
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
