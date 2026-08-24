<script lang="ts">
import type { Snippet } from "svelte";

interface Props {
  loading?: boolean;
  error?: string;
  empty?: boolean;
  children?: Snippet;
  element?: HTMLElement | null;
  onScroll?: () => void;
}

let {
  loading = false,
  error = "",
  empty = false,
  children,
  element = $bindable(null),
  onScroll = () => {},
}: Props = $props();
</script>

<aside
  bind:this={element}
  class:inspector-empty={empty}
  class="inspector-panel panel-surface"
  aria-label="Inspector"
  aria-busy={loading}
  inert={loading || Boolean(error)}
  onscroll={onScroll}>
  {#if empty}
    <span>INSPECTOR</span>
    <p>Select an entry to see its properties, relationships, and attachments.</p>
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
.inspector-empty {
  display: grid;
  min-height: 240px;
  padding: 30px;
  place-items: center;
  color: var(--ink-faint);
  text-align: center;
  font-size: 10px;
}
.inspector-empty p {
  max-width: 170px;
  margin-top: 13px;
  line-height: 1.6;
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
