<script lang="ts">
import type { Snippet } from "svelte";
import WorkbenchState from "./WorkbenchState.svelte";

interface Props {
  loading?: boolean;
  error?: string;
  empty?: boolean;
  children?: Snippet;
  element?: HTMLElement | null;
  onScroll?: () => void;
  onRetry?: () => void;
}

let {
  loading = false,
  error = "",
  empty = false,
  children,
  element = $bindable(null),
  onScroll = () => {},
  onRetry = () => {},
}: Props = $props();
</script>

<aside
  bind:this={element}
  class:inspector-empty={empty}
  class="inspector-panel panel-surface"
  data-guide="workspace-inspector"
  aria-label="Inspector"
  aria-busy={loading}
  inert={loading}
  onscroll={onScroll}>
  {#if loading}
    <WorkbenchState
      kind="loading"
      compact
      title="Loading details"
      message="Reading fields, relationships, and assets." />
  {:else if error}
    {#snippet retryAction()}<button type="button" onclick={onRetry}>Retry</button>{/snippet}
    <WorkbenchState kind="error" compact title="Details unavailable" message={error} actions={retryAction} />
  {:else if empty}
    <WorkbenchState
      kind="empty"
      compact
      title="Nothing selected"
      message="Select an entry to see its details, relationships, assets, and backlinks." />
  {:else if children}
    {@render children()}
  {/if}
</aside>

<style>
.inspector-panel {
  min-width: 0;
  min-height: 650px;
  overflow: visible;
  border: 1px solid var(--line);
  border-radius: 12px;
  background: var(--surface);
  box-shadow: var(--shadow-sm);
}
.inspector-panel :global(.workbench-state button) {
  padding: 7px 10px;
  border: 1px solid var(--line);
  border-radius: 7px;
  background: var(--surface);
  color: var(--ink-soft);
  cursor: pointer;
  font: 700 10px var(--font-body);
}
@media (max-width: 1180px) {
  .inspector-panel {
    display: grid;
    min-height: auto;
    grid-column: 1 / -1;
    grid-template-columns: repeat(3, 1fr);
  }
  .inspector-panel :global(.inspector-heading) {
    grid-column: 1 / -1;
  }
  .inspector-panel :global(.inspector-section) {
    border-right: 1px solid var(--line);
    border-bottom: 0;
  }
}
@media (max-width: 760px) {
  .inspector-panel {
    display: block;
    width: 100%;
    min-height: auto;
    border-radius: 11px;
  }
  .inspector-panel :global(.inspector-section) {
    border-right: 0;
    border-bottom: 1px solid var(--line);
  }
}
</style>
