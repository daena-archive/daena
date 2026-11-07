<script lang="ts">
import type { Snippet } from "svelte";
import { onDestroy, tick } from "svelte";
import { ChevronDown, Plus, X } from "@lucide/svelte";
import type { TabIcon } from "$lib/navigation/tabs";
import { tabDropIndex } from "$lib/shell/tab-drag";

type TabModel = {
  id: string;
  title: string;
  tooltip: string;
  icon: TabIcon;
  active: boolean;
  pinned: boolean;
};

type Anchor = { left: number; right: number; top: number; bottom: number; align: "left" | "right" };

type DragState = {
  id: string;
  fromIndex: number;
  startX: number;
  pointerId: number;
  dragging: boolean;
  dropIndex: number;
  barStyle: string;
  captureEl: HTMLElement | null;
};

let {
  tabs,
  panelId,
  onActivate,
  onClose,
  onNew,
  busy = false,
  icon,
  canReopen,
  onPin,
  onMove,
  onCloseOthers,
  onCloseToRight,
  onCloseAll,
  onReopen,
  popoverOpen = $bindable(false),
}: {
  tabs: TabModel[];
  panelId: string;
  onActivate: (id: string) => void;
  onClose: (id: string) => void;
  onNew: () => void;
  busy?: boolean;
  icon: Snippet<[TabIcon]>;
  canReopen: boolean;
  onPin: (id: string, pinned: boolean) => void;
  onMove: (id: string, toIndex: number) => void;
  onCloseOthers: (id: string) => void;
  onCloseToRight: (id: string) => void;
  onCloseAll: (id: string) => void;
  onReopen: () => void;
  popoverOpen?: boolean;
} = $props();

let tabListEl = $state<HTMLDivElement | null>(null);
let allTabsButton = $state<HTMLButtonElement | null>(null);
let narrowButton = $state<HTMLButtonElement | null>(null);
let menuEl = $state<HTMLDivElement | null>(null);
let overflows = $state(false);
let narrow = $state(false);
let popoverKind = $state<"menu" | "list" | null>(null);
let menuTabId = $state<string | null>(null);
let anchor = $state<Anchor | null>(null);
let anchorEl = $state<HTMLElement | null>(null);
let returnFocus = $state<HTMLElement | null>(null);
let menuStyle = $state("");
let focusOnOpen = $state(false);
let drag = $state<DragState | null>(null);
let suppressClick = false;
let dragListening = false;
let scrollFrame = 0;
let scrollDir = 0;
let lastPointerX = 0;

const activeTabId = $derived(tabs.find((tab) => tab.active)?.id ?? "");
const activeTabModel = $derived(tabs.find((tab) => tab.active) ?? tabs[0] ?? null);
const menuTab = $derived(tabs.find((tab) => tab.id === menuTabId) ?? null);
const menuIndex = $derived(tabs.findIndex((tab) => tab.id === menuTabId));

function tabButtons(list: HTMLElement) {
  return [...list.querySelectorAll<HTMLButtonElement>('[role="tab"]')];
}

function enabledMenuItems(root: HTMLElement) {
  return [...root.querySelectorAll<HTMLButtonElement>("button[role='menuitem'], button[role='menuitemradio']")].filter(
    (button) => button.getAttribute("aria-disabled") !== "true",
  );
}

function moveDisabled(direction: -1 | 1): boolean {
  if (menuIndex < 0) return true;
  const pinnedCount = tabs.filter((tab) => tab.pinned).length;
  const pinned = tabs[menuIndex]?.pinned ?? false;
  const start = pinned ? 0 : pinnedCount;
  const end = pinned ? pinnedCount - 1 : tabs.length - 1;
  const next = menuIndex + direction;
  return next < start || next > end;
}

function closeOthersDisabled(): boolean {
  if (!menuTab) return true;
  return !tabs.some((tab) => !tab.pinned && tab.id !== menuTab.id);
}

function closeRightDisabled(): boolean {
  if (menuIndex < 0) return true;
  return !tabs.some((tab, index) => index > menuIndex && !tab.pinned);
}

function closeAllDisabled(): boolean {
  return !tabs.some((tab) => !tab.pinned);
}

function portal(node: HTMLElement) {
  document.body.appendChild(node);
  return {
    destroy() {
      node.remove();
    },
  };
}

function closePopover(restore: boolean) {
  const target = returnFocus;
  popoverKind = null;
  menuTabId = null;
  anchor = null;
  anchorEl = null;
  returnFocus = null;
  popoverOpen = false;
  if (restore) target?.focus();
}

function placeMenu() {
  const menu = menuEl;
  const box = anchor;
  if (!menu || !box) return;
  const margin = 8;
  const width = menu.offsetWidth;
  const height = menu.offsetHeight;
  let left = box.align === "right" ? box.right - width : box.left;
  let top = box.bottom + 4;
  left = Math.min(Math.max(left, margin), Math.max(margin, window.innerWidth - width - margin));
  const maxTop = window.innerHeight - height - margin;
  if (top > maxTop) top = Math.max(margin, maxTop);
  const next = `left:${Math.round(left)}px;top:${Math.round(top)}px`;
  if (next !== menuStyle) menuStyle = next;
}

function openPopover(kind: "menu" | "list", element: HTMLElement, align: "left" | "right", tabId: string | null) {
  if (busy) return;
  const rect = element.getBoundingClientRect();
  anchorEl = element;
  returnFocus = element;
  menuTabId = tabId;
  anchor = { left: rect.left, right: rect.right, top: rect.top, bottom: rect.bottom, align };
  const left = align === "left" ? rect.left : Math.max(8, rect.right - 200);
  menuStyle = `left:${Math.round(left)}px;top:${Math.round(rect.bottom + 4)}px`;
  focusOnOpen = true;
  popoverKind = kind;
  popoverOpen = true;
}

function runItem(event: MouseEvent, disabled: boolean, action: () => void) {
  event.preventDefault();
  event.stopPropagation();
  if (disabled || busy) return;
  // Run the action first: it reads `menuTab` / `menuIndex`, which closing the popover clears.
  action();
  closePopover(true);
}

function checkOverflow() {
  const list = tabListEl;
  if (!list) return;
  overflows = list.scrollWidth > list.clientWidth + 1;
}

function clampDrop(fromIndex: number, dropIndex: number): number {
  const tab = tabs[fromIndex];
  if (!tab) return dropIndex;
  const pinnedCount = tabs.filter((item) => item.pinned).length;
  const start = tab.pinned ? 0 : pinnedCount;
  const end = tab.pinned ? pinnedCount - 1 : tabs.length - 1;
  if (end < start) return fromIndex;
  return Math.min(Math.max(dropIndex, start), end);
}

function dragAt(current: DragState, clientX: number): DragState {
  const dropIndex = clampDrop(current.fromIndex, tabDropIndex(tabRects(), clientX, current.fromIndex));
  return { ...current, dragging: true, dropIndex, barStyle: barStyleFor(dropIndex, current.fromIndex) };
}

function tabRects() {
  const list = tabListEl;
  if (!list) return [];
  return [...list.querySelectorAll<HTMLElement>(".tab-item")].map((item) => {
    const rect = item.getBoundingClientRect();
    return { left: rect.left, width: rect.width };
  });
}

function barStyleFor(dropIndex: number, fromIndex: number): string {
  const list = tabListEl;
  if (!list) return "display:none";
  const items = [...list.querySelectorAll<HTMLElement>(".tab-item")];
  if (items.length === 0) return "display:none";
  const visual = dropIndex <= fromIndex ? dropIndex : Math.min(dropIndex + 1, items.length);
  const listRect = list.getBoundingClientRect();
  const edge =
    visual >= items.length
      ? items[items.length - 1].getBoundingClientRect().right
      : items[visual].getBoundingClientRect().left;
  return `left:${Math.round(edge - 1)}px;top:${Math.round(listRect.top)}px;height:${Math.round(listRect.height)}px`;
}

function stopAutoScroll() {
  scrollDir = 0;
  if (scrollFrame) cancelAnimationFrame(scrollFrame);
  scrollFrame = 0;
}

function stepScroll() {
  scrollFrame = 0;
  const current = drag;
  const list = tabListEl;
  if (!current?.dragging || !list || scrollDir === 0) return;
  list.scrollLeft += scrollDir * 8;
  if (drag !== current) return;
  drag = dragAt(current, lastPointerX);
  scrollFrame = requestAnimationFrame(stepScroll);
}

function autoScroll(clientX: number) {
  lastPointerX = clientX;
  const list = tabListEl;
  if (!list) return;
  const rect = list.getBoundingClientRect();
  const edge = 28;
  if (clientX < rect.left + edge) scrollDir = -1;
  else if (clientX > rect.right - edge) scrollDir = 1;
  else scrollDir = 0;
  if (scrollDir !== 0 && scrollFrame === 0) scrollFrame = requestAnimationFrame(stepScroll);
  if (scrollDir === 0 && scrollFrame) stopAutoScroll();
}

function silenceDrag() {
  if (!dragListening) return;
  dragListening = false;
  window.removeEventListener("pointermove", onWindowPointerMove);
  window.removeEventListener("pointerup", onWindowPointerUp);
  window.removeEventListener("pointercancel", onWindowPointerCancel);
  window.removeEventListener("keydown", onDragKeydown, true);
  stopAutoScroll();
}

function releaseCapture(current: DragState) {
  if (!current.dragging || !current.captureEl) return;
  try {
    current.captureEl.releasePointerCapture(current.pointerId);
  } catch {
    // The pointer is already gone.
  }
}

function cancelDrag() {
  const current = drag;
  if (current) releaseCapture(current);
  silenceDrag();
  drag = null;
}

function finishDrag(event: PointerEvent) {
  if (!drag || event.pointerId !== drag.pointerId) return;
  const current = drag;
  releaseCapture(current);
  silenceDrag();
  drag = null;
  if (!current.dragging) return;
  suppressClick = true;
  window.setTimeout(() => {
    suppressClick = false;
  }, 0);
  if (!busy && current.dropIndex !== current.fromIndex) onMove(current.id, current.dropIndex);
}

function onWindowPointerMove(event: PointerEvent) {
  if (!drag || event.pointerId !== drag.pointerId || busy) return;
  if (!drag.dragging && Math.abs(event.clientX - drag.startX) <= 4) return;
  if (!drag.dragging) {
    try {
      drag.captureEl?.setPointerCapture(event.pointerId);
    } catch {
      // Capture can fail once the pointer is gone.
    }
  }
  drag = dragAt(drag, event.clientX);
  autoScroll(event.clientX);
}

function onWindowPointerUp(event: PointerEvent) {
  finishDrag(event);
}

function onWindowPointerCancel(event: PointerEvent) {
  if (!drag || event.pointerId !== drag.pointerId) return;
  cancelDrag();
}

function onDragKeydown(event: KeyboardEvent) {
  if (event.key !== "Escape" || !drag) return;
  event.preventDefault();
  event.stopPropagation();
  if (drag.dragging) {
    suppressClick = true;
    window.setTimeout(() => {
      suppressClick = false;
    }, 0);
  }
  cancelDrag();
}

function onPointerDown(event: PointerEvent, tab: TabModel, index: number) {
  if (busy || event.button !== 0) return;
  const target = event.target;
  if (!(target instanceof Element) || target.closest(".tab-close")) return;
  drag = {
    id: tab.id,
    fromIndex: index,
    startX: event.clientX,
    pointerId: event.pointerId,
    dragging: false,
    dropIndex: index,
    barStyle: "",
    captureEl: event.currentTarget instanceof HTMLElement ? event.currentTarget : null,
  };
  if (dragListening) return;
  dragListening = true;
  window.addEventListener("pointermove", onWindowPointerMove);
  window.addEventListener("pointerup", onWindowPointerUp);
  window.addEventListener("pointercancel", onWindowPointerCancel);
  window.addEventListener("keydown", onDragKeydown, true);
}

function moveTabFocus(event: KeyboardEvent) {
  if (event.altKey || event.metaKey || event.ctrlKey || event.shiftKey) return;
  if (event.key !== "ArrowLeft" && event.key !== "ArrowRight" && event.key !== "Home" && event.key !== "End") return;
  const list = event.currentTarget;
  if (!(list instanceof HTMLElement)) return;
  const buttons = tabButtons(list);
  const index = buttons.findIndex((button) => button === document.activeElement);
  if (index < 0 || buttons.length === 0) return;
  event.preventDefault();
  const next =
    event.key === "Home"
      ? 0
      : event.key === "End"
        ? buttons.length - 1
        : event.key === "ArrowLeft"
          ? (index - 1 + buttons.length) % buttons.length
          : (index + 1) % buttons.length;
  buttons[next]?.focus();
}

function openFocusedTabMenu(event: KeyboardEvent): boolean {
  const contextKey = event.key === "ContextMenu" || (event.key === "F10" && event.shiftKey);
  if (!contextKey || event.altKey || event.metaKey || event.ctrlKey) return false;
  const list = event.currentTarget;
  if (!(list instanceof HTMLElement)) return false;
  const buttons = tabButtons(list);
  const index = buttons.findIndex((button) => button === document.activeElement);
  if (index < 0) return false;
  event.preventDefault();
  openPopover("menu", buttons[index], "left", tabs[index].id);
  return true;
}

function onTablistKeydown(event: KeyboardEvent) {
  if (busy) return;
  if (openFocusedTabMenu(event)) return;
  if (
    (event.key === "Delete" || event.key === "Backspace") &&
    !event.altKey &&
    !event.metaKey &&
    !event.ctrlKey &&
    !event.shiftKey
  ) {
    const list = event.currentTarget;
    if (!(list instanceof HTMLElement)) return;
    const buttons = tabButtons(list);
    const index = buttons.findIndex((button) => button === document.activeElement);
    if (index < 0) return;
    event.preventDefault();
    onClose(tabs[index].id);
    return;
  }
  moveTabFocus(event);
}

function selectTab(tab: TabModel) {
  if (busy || tab.active) return;
  onActivate(tab.id);
}

function onTabClick(event: MouseEvent, tab: TabModel) {
  if (suppressClick) {
    suppressClick = false;
    event.preventDefault();
    event.stopPropagation();
    return;
  }
  selectTab(tab);
}

function closeTab(id: string) {
  if (busy) return;
  onClose(id);
}

function newTab() {
  if (busy) return;
  onNew();
}

function toggleTabList(element: HTMLElement | null) {
  if (!element || busy) return;
  if (popoverKind === "list") {
    closePopover(true);
    return;
  }
  openPopover("list", element, "right", null);
}

function onContextMenu(event: MouseEvent, tab: TabModel) {
  event.preventDefault();
  if (busy) return;
  const host = event.currentTarget;
  const button = host instanceof HTMLElement ? host.querySelector<HTMLElement>('[role="tab"]') : null;
  openPopover("menu", button ?? (host instanceof HTMLElement ? host : document.body), "left", tab.id);
}

function blockMiddleClick(event: MouseEvent) {
  if (event.button === 1) event.preventDefault();
}

function onPopoverKeydown(event: KeyboardEvent) {
  if (!popoverKind) return;
  if (event.key === "Escape" || event.key === "Tab") {
    event.preventDefault();
    event.stopPropagation();
    closePopover(true);
    return;
  }
  const root = menuEl;
  if (!root?.contains(document.activeElement)) return;
  const items = enabledMenuItems(root);
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

function onPopoverPointerDown(event: PointerEvent) {
  const target = event.target;
  if (!(target instanceof Node)) return;
  if (menuEl?.contains(target)) return;
  if (allTabsButton?.contains(target) || narrowButton?.contains(target)) return;
  // The pointer is going somewhere else; let it take focus instead of pulling focus back to the tab.
  closePopover(false);
}

$effect(() => {
  if (popoverOpen || !popoverKind) return;
  popoverKind = null;
  menuTabId = null;
  anchor = null;
  anchorEl = null;
  returnFocus = null;
});

$effect(() => {
  if (popoverKind !== "menu" || !menuTabId) return;
  if (!tabs.some((tab) => tab.id === menuTabId)) closePopover(false);
});

$effect(() => {
  if (!popoverKind || !anchor || !menuEl) return;
  placeMenu();
});

$effect(() => {
  if (!focusOnOpen || !menuEl || !popoverKind) return;
  focusOnOpen = false;
  enabledMenuItems(menuEl)[0]?.focus();
});

$effect(() => {
  if (!popoverKind) return;
  const onResize = () => {
    if (!anchorEl?.isConnected) {
      closePopover(false);
      return;
    }
    const rect = anchorEl.getBoundingClientRect();
    if (!anchor) return;
    anchor = { left: rect.left, right: rect.right, top: rect.top, bottom: rect.bottom, align: anchor.align };
    placeMenu();
  };
  window.addEventListener("resize", onResize);
  window.addEventListener("keydown", onPopoverKeydown, true);
  document.addEventListener("pointerdown", onPopoverPointerDown, true);
  return () => {
    window.removeEventListener("resize", onResize);
    window.removeEventListener("keydown", onPopoverKeydown, true);
    document.removeEventListener("pointerdown", onPopoverPointerDown, true);
  };
});

$effect(() => {
  const list = tabListEl;
  if (!list) return;
  const measure = () => checkOverflow();
  const observer = new ResizeObserver(measure);
  observer.observe(list);
  measure();
  return () => observer.disconnect();
});

$effect(() => {
  for (const tab of tabs) (void tab.id, void tab.pinned, void tab.title, void tab.active);
  void tick().then(() => checkOverflow());
});

$effect(() => {
  const id = activeTabId;
  const list = tabListEl;
  if (!id || !list || narrow) return;
  list.querySelector<HTMLElement>(`#shell-tab-${CSS.escape(id)}`)?.scrollIntoView({
    inline: "nearest",
    block: "nearest",
  });
});

$effect(() => {
  const list = tabListEl;
  if (!list) return;
  const onWheel = (event: WheelEvent) => {
    if (Math.abs(event.deltaY) <= Math.abs(event.deltaX)) return;
    list.scrollLeft += event.deltaY;
    event.preventDefault();
  };
  list.addEventListener("wheel", onWheel, { passive: false });
  return () => list.removeEventListener("wheel", onWheel);
});

$effect(() => {
  const query = window.matchMedia("(max-width: 760px)");
  const update = () => {
    narrow = query.matches;
  };
  update();
  query.addEventListener("change", update);
  return () => query.removeEventListener("change", update);
});

onDestroy(() => silenceDrag());
</script>

{#snippet actionItem(label: string, disabled: boolean, action: () => void)}
  <button
    type="button"
    role="menuitem"
    tabindex="-1"
    aria-disabled={disabled}
    onclick={(event) => runItem(event, disabled, action)}>{label}</button>
{/snippet}

<div class="tab-strip">
  <div
    role="tablist"
    aria-label="Open tabs"
    class="tab-list"
    tabindex="-1"
    bind:this={tabListEl}
    onkeydown={onTablistKeydown}>
    {#each tabs as tab, index (tab.id)}
      <div
        role="presentation"
        class="tab-item"
        class:active={tab.active}
        class:pinned={tab.pinned}
        class:dragging={drag?.dragging && drag.id === tab.id}
        onpointerdown={(event) => onPointerDown(event, tab, index)}
        onmousedown={blockMiddleClick}
        oncontextmenu={(event) => onContextMenu(event, tab)}
        onauxclick={(event) => {
          if (event.button === 1) closeTab(tab.id);
        }}>
        <button
          role="tab"
          type="button"
          id="shell-tab-{tab.id}"
          class="tab"
          aria-selected={tab.active}
          aria-controls={panelId}
          aria-label={tab.pinned ? tab.title : undefined}
          title={tab.tooltip}
          tabindex={tab.active ? 0 : -1}
          onclick={(event) => onTabClick(event, tab)}>
          <span class="tab-icon" aria-hidden="true">{@render icon(tab.icon)}</span>
          {#if !tab.pinned}<span class="tab-title">{tab.title}</span>{/if}
        </button>
        {#if !tab.pinned}<button
            type="button"
            class="tab-close"
            tabindex="-1"
            aria-label={`Close ${tab.title}`}
            title="Close tab"
            onclick={() => closeTab(tab.id)}><X size={12} strokeWidth={1.8} aria-hidden="true" /></button
          >{/if}
      </div>
    {/each}
  </div>
  <button type="button" class="tab-new" aria-label="New tab" title="New tab" onclick={newTab}>
    <Plus size={14} strokeWidth={1.8} aria-hidden="true" />
  </button>
  {#if overflows}<button
      bind:this={allTabsButton}
      type="button"
      class="tab-new tab-all"
      aria-label="All tabs"
      aria-haspopup="menu"
      aria-expanded={popoverKind === "list"}
      title="All tabs"
      onclick={() => toggleTabList(allTabsButton)}
      ><ChevronDown size={14} strokeWidth={1.8} aria-hidden="true" /></button
    >{/if}
  {#if activeTabModel}<button
      bind:this={narrowButton}
      type="button"
      class="tab-narrow"
      aria-label={`Tabs: ${activeTabModel.title}`}
      aria-haspopup="menu"
      aria-expanded={popoverKind === "list"}
      onclick={() => toggleTabList(narrowButton)}>
      <span class="tab-icon" aria-hidden="true">{@render icon(activeTabModel.icon)}</span>
      <span class="tab-narrow-title">{activeTabModel.title}</span>
      <ChevronDown size={14} strokeWidth={1.8} aria-hidden="true" />
    </button>{/if}
</div>

{#if popoverKind === "menu" && menuTab}
  <div
    bind:this={menuEl}
    use:portal
    class="tab-popover"
    role="menu"
    tabindex="-1"
    aria-label="Tab actions"
    data-shell-popover="tab-menu"
    style={menuStyle}
    oncontextmenu={(event) => event.preventDefault()}>
    {@render actionItem(menuTab.pinned ? "Unpin tab" : "Pin tab", false, () => onPin(menuTab.id, !menuTab.pinned))}
    {@render actionItem("Move left", moveDisabled(-1), () => onMove(menuTab.id, menuIndex - 1))}
    {@render actionItem("Move right", moveDisabled(1), () => onMove(menuTab.id, menuIndex + 1))}
    <div role="separator"></div>
    {@render actionItem("Close tab", false, () => onClose(menuTab.id))}
    {@render actionItem("Close other tabs", closeOthersDisabled(), () => onCloseOthers(menuTab.id))}
    {@render actionItem("Close tabs to the right", closeRightDisabled(), () => onCloseToRight(menuTab.id))}
    {@render actionItem("Close all tabs", closeAllDisabled(), () => onCloseAll(menuTab.id))}
    <div role="separator"></div>
    {@render actionItem("Reopen closed tab", !canReopen, () => onReopen())}
  </div>
{:else if popoverKind === "list"}
  <div
    bind:this={menuEl}
    use:portal
    class="tab-popover tab-popover-list"
    role="menu"
    tabindex="-1"
    aria-label="All tabs"
    data-shell-popover="tab-list"
    style={menuStyle}
    oncontextmenu={(event) => event.preventDefault()}>
    {#if narrow}
      {@render actionItem("New tab", false, onNew)}
      <div role="separator"></div>
    {/if}
    {#each tabs as tab (tab.id)}
      <button
        type="button"
        role="menuitemradio"
        tabindex="-1"
        aria-checked={tab.active}
        onclick={(event) => runItem(event, false, () => onActivate(tab.id))}>
        <span class="tab-icon" aria-hidden="true">{@render icon(tab.icon)}</span>
        <span class="tab-popover-title">{tab.title}</span>
      </button>
    {/each}
    <div role="separator"></div>
    {@render actionItem("Reopen closed tab", !canReopen, () => onReopen())}
  </div>
{/if}

{#if drag?.dragging}<div class="insert-bar" use:portal style={drag.barStyle}></div>{/if}

<style>
.tab-strip,
.tab-list {
  display: flex;
  align-items: center;
  gap: 2px;
  min-width: 0;
}
.tab-list {
  overflow-x: auto;
  scrollbar-width: none;
  scroll-behavior: smooth;
}
.tab-list::-webkit-scrollbar {
  display: none;
}
.tab-item {
  display: inline-flex;
  align-items: center;
  gap: 2px;
  height: 30px;
  flex: 0 1 auto;
  min-width: 96px;
  max-width: 220px;
  padding: 0 4px 0 10px;
  border: 1px solid transparent;
  border-radius: 7px;
  background: transparent;
  color: var(--ink-soft);
  user-select: none;
}
.tab-item.pinned {
  flex: none;
  min-width: 0;
  padding: 0 8px;
}
.tab-item.dragging {
  opacity: 0.6;
}
.tab-item:hover:not(.active) {
  background: var(--surface-muted);
  color: var(--ink);
}
.tab-item.active {
  border-color: var(--line);
  background: var(--surface);
  color: var(--ink);
}
.tab {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  flex: 1;
  min-width: 0;
  height: 100%;
  padding: 0;
  border: 0;
  border-radius: 7px;
  background: transparent;
  color: inherit;
  font: inherit;
  font-size: 12px;
  font-weight: 600;
  cursor: pointer;
}
.tab-item.active .tab {
  cursor: default;
}
.tab:focus-visible,
.tab-new:focus-visible,
.tab-narrow:focus-visible,
.tab-popover button:focus-visible {
  outline: 3px solid var(--focus-ring);
  outline-offset: 2px;
}

@media (forced-colors: active) {
  .tab:focus-visible,
  .tab-new:focus-visible,
  .tab-narrow:focus-visible,
  .tab-popover button:focus-visible {
    outline: 3px solid Highlight;
  }
}
.tab-icon {
  display: grid;
  width: 16px;
  height: 16px;
  flex: 0 0 auto;
  place-items: center;
}
.tab-title,
.tab-narrow-title,
.tab-popover-title {
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.tab-close {
  display: grid;
  width: 18px;
  height: 18px;
  flex: 0 0 auto;
  place-items: center;
  padding: 0;
  border: 0;
  border-radius: 4px;
  background: transparent;
  color: inherit;
  opacity: 0;
  cursor: pointer;
}
.tab-item.active .tab-close,
.tab-item:hover .tab-close,
.tab-close:focus-visible {
  opacity: 1;
}
.tab-close:hover {
  background: var(--surface-muted);
}
.tab-new {
  display: grid;
  width: 28px;
  height: 28px;
  flex: 0 0 auto;
  place-items: center;
  padding: 0;
  border: 1px solid transparent;
  border-radius: 7px;
  background: transparent;
  color: var(--ink-soft);
  cursor: pointer;
}
.tab-new:hover {
  border-color: var(--line);
  background: var(--surface-muted);
  color: var(--ink);
}
.tab-narrow {
  display: none;
  align-items: center;
  gap: 6px;
  max-width: min(240px, 70vw);
  height: 28px;
  padding: 0 8px;
  border: 1px solid transparent;
  border-radius: 7px;
  background: transparent;
  color: var(--ink);
  font: inherit;
  font-size: 12px;
  font-weight: 600;
  cursor: pointer;
}
.tab-narrow:hover {
  border-color: var(--line);
  background: var(--surface-muted);
}
.tab-popover {
  position: fixed;
  z-index: 400;
  min-width: 188px;
  padding: 4px;
  border: 1px solid var(--line-strong);
  border-radius: 13px;
  background: var(--surface);
  box-shadow: var(--shadow-lg);
}
.tab-popover-list {
  max-height: 60vh;
  overflow-y: auto;
}
.tab-popover button {
  display: flex;
  align-items: center;
  gap: 8px;
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
.tab-popover button[aria-checked="true"] {
  font-weight: 700;
}
.tab-popover button[aria-disabled="true"] {
  color: var(--ink-faint);
  cursor: default;
}
.tab-popover button:hover:not([aria-disabled="true"]) {
  background: var(--surface-muted);
}
.tab-popover [role="separator"] {
  height: 1px;
  margin: 4px 6px;
  background: var(--line);
}
.insert-bar {
  position: fixed;
  z-index: 80;
  width: 2px;
  background: var(--accent);
  pointer-events: none;
}

@media (max-width: 760px) {
  .tab-list,
  .tab-new {
    display: none;
  }
  .tab-narrow {
    display: inline-flex;
  }
}
</style>
