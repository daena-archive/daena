<script lang="ts">
import { onMount } from "svelte";

let {
  x,
  y,
  label,
  items,
  returnFocus,
  onClose,
}: {
  /** Viewport point the menu opens at (its top-left corner). */
  x: number;
  y: number;
  label: string;
  items: Array<{ label: string; action: () => void }>;
  returnFocus: HTMLElement | null;
  onClose: () => void;
} = $props();

let menuEl = $state<HTMLDivElement | null>(null);
/** Set once the menu is measured. Until then it sits at the requested point. */
let placed = $state<{ left: number; top: number } | null>(null);
const menuStyle = $derived(placed ? `left: ${placed.left}px; top: ${placed.top}px` : `left: ${x}px; top: ${y}px`);

function portal(node: HTMLElement) {
  document.body.appendChild(node);
  return {
    destroy() {
      node.remove();
    },
  };
}

function enabledItems(): HTMLButtonElement[] {
  return [...(menuEl?.querySelectorAll<HTMLButtonElement>("button[role='menuitem']") ?? [])];
}

function place() {
  const menu = menuEl;
  if (!menu) return;
  const margin = 8;
  const left = Math.min(Math.max(x, margin), Math.max(margin, window.innerWidth - menu.offsetWidth - margin));
  const top = Math.min(Math.max(y, margin), Math.max(margin, window.innerHeight - menu.offsetHeight - margin));
  placed = { left: Math.round(left), top: Math.round(top) };
}

function runItem(event: MouseEvent, action: () => void) {
  event.preventDefault();
  event.stopPropagation();
  // Run the action first: closing clears the menu, and the action reads what the menu was opened for.
  action();
  if (returnFocus?.isConnected) returnFocus.focus();
  onClose();
}

function onMenuKeydown(event: KeyboardEvent) {
  if (event.key === "Escape" || event.key === "Tab") {
    event.preventDefault();
    event.stopPropagation();
    if (returnFocus?.isConnected) returnFocus.focus();
    onClose();
    return;
  }
  const root = menuEl;
  if (!root?.contains(document.activeElement)) return;
  const items = enabledItems();
  if (items.length === 0) return;
  const current = items.findIndex((item) => item === document.activeElement);
  if (event.key === "ArrowDown" || event.key === "ArrowUp") {
    event.preventDefault();
    const delta = event.key === "ArrowDown" ? 1 : -1;
    const next = current < 0 ? (delta === 1 ? 0 : items.length - 1) : (current + delta + items.length) % items.length;
    items[next]?.focus();
    return;
  }
  if (event.key === "Home") {
    event.preventDefault();
    items[0]?.focus();
    return;
  }
  if (event.key === "End") {
    event.preventDefault();
    items[items.length - 1]?.focus();
    return;
  }
  if ((event.key === "Enter" || event.key === " ") && current >= 0) {
    event.preventDefault();
    items[current]?.click();
  }
}

function onOutsidePointerDown(event: PointerEvent) {
  const target = event.target;
  if (!(target instanceof Node)) return;
  if (menuEl?.contains(target)) return;
  // The pointer is going somewhere else; let it take focus instead of pulling focus back to the anchor.
  onClose();
}

$effect(() => {
  // Measured again whenever the anchor point changes.
  void x;
  void y;
  place();
});

onMount(() => {
  enabledItems()[0]?.focus();
  const onKey = (event: KeyboardEvent) => onMenuKeydown(event);
  window.addEventListener("keydown", onKey, true);
  document.addEventListener("pointerdown", onOutsidePointerDown, true);
  window.addEventListener("resize", place);
  return () => {
    window.removeEventListener("keydown", onKey, true);
    document.removeEventListener("pointerdown", onOutsidePointerDown, true);
    window.removeEventListener("resize", place);
  };
});
</script>

<div
  bind:this={menuEl}
  use:portal
  class="rail-menu"
  role="menu"
  tabindex="-1"
  aria-label={label}
  data-shell-popover="rail-menu"
  style={menuStyle}
  oncontextmenu={(event) => event.preventDefault()}>
  {#each items as item (item.label)}
    <button type="button" role="menuitem" tabindex="-1" onclick={(event) => runItem(event, item.action)}
      >{item.label}</button>
  {/each}
</div>

<style>
.rail-menu {
  position: fixed;
  z-index: 400;
  min-width: 188px;
  padding: 4px;
  border: 1px solid var(--line-strong);
  border-radius: 13px;
  background: var(--surface);
  box-shadow: var(--shadow-lg);
}
.rail-menu button {
  display: flex;
  align-items: center;
  width: 100%;
  height: 28px;
  padding: 0 10px;
  border: 0;
  border-radius: 6px;
  background: transparent;
  color: var(--ink);
  font: inherit;
  font-size: 12px;
  text-align: left;
  cursor: pointer;
}
.rail-menu button:hover {
  background: var(--surface-muted);
}
.rail-menu button:focus-visible {
  outline: 3px solid var(--focus-ring);
  outline-offset: 2px;
}

@media (forced-colors: active) {
  .rail-menu button:focus-visible {
    outline: 3px solid Highlight;
  }
}
</style>
