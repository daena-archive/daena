<script lang="ts">
import { Handle, Position } from "@xyflow/svelte";
let { data: _data }: { data?: unknown } = $props();
void _data;
</script>

<div class="union" aria-hidden="true">
  <Handle type="target" position={Position.Top} isConnectable={false} />
  <Handle type="source" position={Position.Bottom} isConnectable={false} />
</div>

<style>
.union {
  width: 12px;
  height: 12px;
  border: 2px solid var(--ink-muted);
  border-radius: 50%;
  background: var(--surface);
}
</style>
