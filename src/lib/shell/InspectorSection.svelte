<script lang="ts">
import type { Snippet } from "svelte";
import { ChevronRight } from "@lucide/svelte";

interface Props {
  title: string;
  count?: number;
  open?: boolean;
  children: Snippet;
}

let { title, count, open = true, children }: Props = $props();
</script>

<details class="inspector-group" {open}>
  <summary>
    <ChevronRight size={14} strokeWidth={1.8} aria-hidden="true" />
    <strong>{title}</strong>
    {#if count !== undefined}<span>{count}</span>{/if}
  </summary>
  <div class="inspector-group-body">{@render children()}</div>
</details>

<style>
.inspector-group {
  border-bottom: 1px solid var(--line);
}
.inspector-group summary {
  display: grid;
  min-height: 42px;
  grid-template-columns: 16px minmax(0, 1fr) auto;
  align-items: center;
  gap: 7px;
  padding: 0 15px;
  color: var(--ink-soft);
  cursor: pointer;
  list-style: none;
}
.inspector-group summary::-webkit-details-marker {
  display: none;
}
.inspector-group summary:hover {
  background: var(--surface-muted);
  color: var(--ink);
}
.inspector-group summary :global(svg) {
  transition: transform 0.16s ease;
}
.inspector-group[open] summary :global(svg) {
  transform: rotate(90deg);
}
.inspector-group summary strong {
  font-size: 10px;
  letter-spacing: 0.08em;
  text-transform: uppercase;
}
.inspector-group summary span {
  min-width: 20px;
  padding: 2px 6px;
  border-radius: 999px;
  background: var(--surface-muted);
  color: var(--ink-faint);
  font-size: 9px;
  text-align: center;
}
.inspector-group-body {
  padding: 2px 15px 17px;
}
</style>
