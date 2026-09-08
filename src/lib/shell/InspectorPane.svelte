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
      message="Select an entry to see its details and edit more fields." />
  {:else if children}
    {@render children()}
  {/if}
</aside>

<style>
.inspector-panel {
  min-width: 0;
  min-height: 0;
  overflow: auto;
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
    min-height: auto;
    grid-column: 1 / -1;
  }
}
@media (max-width: 760px) {
  .inspector-panel {
    width: 100%;
    min-height: auto;
    border-radius: 11px;
  }
}
</style>
