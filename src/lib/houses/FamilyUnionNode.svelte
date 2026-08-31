<script lang="ts">
import { Handle, Position } from "@xyflow/svelte";
import { getTreeCanvasHost } from "./treeCanvasHost.ts";

let {
  data,
}: {
  data?: {
    memberIds?: string[];
  };
} = $props();

const host = getTreeCanvasHost();
const members = $derived(data?.memberIds ?? []);
</script>

<div class="union">
  <Handle id="north" type="target" position={Position.Top} isConnectable={false} />
  <Handle id="west" type="target" position={Position.Left} isConnectable={false} />
  <Handle id="east" type="target" position={Position.Right} isConnectable={false} />
  {#if members.length >= 2}
    <button
      type="button"
      class="dot nodrag nopan"
      aria-label="Add child to this union — {members.length} parents"
      title="Add child"
      onclick={(event) => {
        event.stopPropagation();
        host.onAddUnionChild(members);
      }}>
      <span aria-hidden="true">+</span>
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
/* hit area is the full 12px node but we enlarge interactive dot to 32px via invisible padding */
.dot {
  display: grid;
  width: 14px;
  height: 14px;
  padding: 0;
  border: 1.5px solid var(--line-strong, #b8c4ba);
  border-radius: 50%;
  background: var(--surface);
  color: var(--ink);
  font: 700 10px/1 var(--font-body, Inter, ui-sans-serif, system-ui, sans-serif);
  place-items: center;
  cursor: pointer;
  box-shadow: 0 1px 2px rgba(0, 0, 0, 0.06);
  transition:
    border-color 140ms ease,
    background 140ms ease,
    transform 140ms ease;
}
button.dot {
  width: 22px;
  height: 22px;
  border-radius: 50%;
  font-size: 13px;
  line-height: 1;
  box-shadow: var(--shadow-sm, 0 1px 4px rgba(0, 0, 0, 0.07));
}
/* invisible enlarged hit padding — keeps visual 22px but hit 32px+ */
button.dot::before {
  content: "";
  position: absolute;
  inset: -6px;
  border-radius: 50%;
}
button.dot:hover {
  border-color: var(--accent, #b7793f);
  background: var(--accent-bg, #e4ece4);
  color: var(--accent-dark, #2f4e35);
  transform: scale(1.04);
}
button.dot:focus-visible {
  outline: 2px solid var(--accent);
  outline-offset: 2px;
  border-color: var(--accent);
}
button.dot:active {
  transform: scale(0.97);
}
:global(.svelte-flow__handle) {
  width: 8px;
  height: 8px;
  opacity: 0;
  border: none;
  background: transparent;
}
@media (prefers-reduced-motion: reduce) {
  .dot,
  button.dot {
    transition: none;
  }
}
</style>
