<script lang="ts">
import { Handle, Position } from "@xyflow/svelte";

let {
  data,
}: {
  data?: {
    memberIds?: string[];
    onAddChild?: (memberIds: string[]) => void;
  };
} = $props();

const members = $derived(data?.memberIds ?? []);
</script>

<div class="union">
  <Handle id="north" type="target" position={Position.Top} isConnectable={false} />
  <Handle id="west" type="target" position={Position.Left} isConnectable={false} />
  <Handle id="east" type="target" position={Position.Right} isConnectable={false} />
  {#if members.length >= 2 && data?.onAddChild}
    <button
      type="button"
      class="dot nodrag nopan"
      aria-label="Add child to this marriage"
      title="Add child"
      onclick={(event) => {
        event.stopPropagation();
        data?.onAddChild?.(members);
      }}>
      +
    </button>
  {:else}
    <span class="dot" aria-hidden="true"></span>
  {/if}
  <Handle id="south" type="source" position={Position.Bottom} isConnectable={false} />
</div>

<style>
.union {
  position: relative;
  display: grid;
  box-sizing: border-box;
  width: 100%;
  height: 100%;
  overflow: visible;
  place-items: center;
}
.dot {
  display: grid;
  width: 12px;
  height: 12px;
  padding: 0;
  border: 2px solid var(--ink);
  border-radius: 50%;
  background: var(--surface);
  color: var(--ink);
  font:
    700 9px/1 Inter,
    ui-sans-serif,
    system-ui,
    sans-serif;
  place-items: center;
  cursor: pointer;
}
button.dot:hover,
button.dot:focus-visible {
  background: var(--surface-muted, var(--surface));
}
button.dot:focus-visible {
  outline: 2px solid var(--accent);
  outline-offset: 1px;
}
:global(.svelte-flow__handle) {
  width: 8px;
  height: 8px;
  opacity: 0;
  border: none;
  background: transparent;
}
</style>
