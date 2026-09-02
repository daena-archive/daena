<script lang="ts">
import { MoreHorizontal } from "@lucide/svelte";
import { confirmDialog } from "$lib/dialogs.svelte";
import { archiveConfirmOptions } from "./archive.ts";
import { ENTITY_ACTIONS, MUTATION_STATUS } from "./vocabulary.ts";

export type EntityRowActionId = "open" | "editIdentity" | "archive" | "openTree" | "openIn";

let {
  entityName,
  open = true,
  editIdentity = true,
  archive = true,
  openTree = false,
  openInLabel = "",
  disabled = false,
  onOpen,
  onEditIdentity,
  onArchive,
  onOpenTree,
  onOpenIn,
}: {
  entityName: string;
  open?: boolean;
  editIdentity?: boolean;
  archive?: boolean;
  openTree?: boolean;
  /** Author-facing destination for Open in… (for example "Lore"). */
  openInLabel?: string;
  disabled?: boolean;
  onOpen: () => void | Promise<void>;
  onEditIdentity: () => void | Promise<void>;
  onArchive: () => void | Promise<void>;
  onOpenTree?: () => void | Promise<void>;
  onOpenIn?: () => void | Promise<void>;
} = $props();

let menuOpen = $state(false);
let archiveBusy = $state(false);
let rootEl = $state<HTMLElement | null>(null);
let triggerEl = $state<HTMLButtonElement | null>(null);
let menuTop = $state(0);
let menuRight = $state(8);

const label = $derived(`Actions for ${entityName}`);
const openInText = $derived(
  openInLabel ? `${ENTITY_ACTIONS.openIn.replace("…", "")} ${openInLabel}` : ENTITY_ACTIONS.openIn,
);

function close(options?: { restoreFocus?: boolean }) {
  menuOpen = false;
  if (options?.restoreFocus !== false) {
    queueMicrotask(() => triggerEl?.focus());
  }
}

function toggle(event: MouseEvent) {
  event.preventDefault();
  event.stopPropagation();
  if (disabled || archiveBusy) return;
  if (menuOpen) {
    close({ restoreFocus: false });
    return;
  }
  menuOpen = true;
  placeMenu();
  queueMicrotask(() => triggerEl?.focus());
}

function placeMenu() {
  if (!triggerEl) return;
  const rect = triggerEl.getBoundingClientRect();
  menuTop = rect.bottom + 4;
  menuRight = Math.max(8, window.innerWidth - rect.right);
}

async function run(action: () => void | Promise<void>) {
  close({ restoreFocus: true });
  await action();
  queueMicrotask(() => triggerEl?.focus());
}

async function runArchive() {
  if (archiveBusy) return;
  close({ restoreFocus: false });
  if (!(await confirmDialog(archiveConfirmOptions(entityName)))) {
    queueMicrotask(() => triggerEl?.focus());
    return;
  }
  archiveBusy = true;
  try {
    await onArchive();
  } finally {
    archiveBusy = false;
    // Successful archive unmounts this row; the shell moves focus to the next control.
    // On failure the trigger remains mounted, so restore focus here.
    queueMicrotask(() => {
      if (triggerEl && document.contains(triggerEl)) triggerEl.focus();
    });
  }
}

$effect(() => {
  if (!menuOpen) return;
  placeMenu();
  let ignoreOpeningPointer = true;
  queueMicrotask(() => {
    ignoreOpeningPointer = false;
  });
  const onPointer = (event: PointerEvent) => {
    if (ignoreOpeningPointer) return;
    if (rootEl && !rootEl.contains(event.target as Node)) close({ restoreFocus: false });
  };
  const onKey = (event: KeyboardEvent) => {
    if (event.key === "Escape") {
      event.preventDefault();
      close();
    }
  };
  const onReposition = () => close({ restoreFocus: false });
  window.addEventListener("pointerdown", onPointer, true);
  window.addEventListener("keydown", onKey, true);
  window.addEventListener("scroll", onReposition, true);
  window.addEventListener("resize", onReposition);
  return () => {
    window.removeEventListener("pointerdown", onPointer, true);
    window.removeEventListener("keydown", onKey, true);
    window.removeEventListener("scroll", onReposition, true);
    window.removeEventListener("resize", onReposition);
  };
});
</script>

<div class="row-actions" bind:this={rootEl}>
  <button
    type="button"
    class="row-actions-trigger"
    bind:this={triggerEl}
    aria-label={label}
    aria-haspopup="menu"
    aria-expanded={menuOpen}
    disabled={disabled || archiveBusy}
    onclick={toggle}>
    <MoreHorizontal size={15} strokeWidth={1.8} aria-hidden="true" />
  </button>
  {#if menuOpen}
    <div class="row-actions-menu" role="menu" aria-label={label} style="top: {menuTop}px; right: {menuRight}px">
      {#if open}
        <button type="button" role="menuitem" onclick={() => void run(onOpen)}>{ENTITY_ACTIONS.open}</button>
      {/if}
      {#if editIdentity}
        <button type="button" role="menuitem" onclick={() => void run(onEditIdentity)}
          >{ENTITY_ACTIONS.editIdentity}</button>
      {/if}
      {#if archive}
        <button type="button" role="menuitem" class="danger" onclick={() => void runArchive()}
          >{archiveBusy ? MUTATION_STATUS.working : ENTITY_ACTIONS.archive}</button>
      {/if}
      {#if openTree && onOpenTree}
        <button type="button" role="menuitem" onclick={() => void run(onOpenTree)}>{ENTITY_ACTIONS.openTree}</button>
      {/if}
      {#if openInLabel && onOpenIn}
        <button type="button" role="menuitem" onclick={() => void run(onOpenIn)}>{openInText}</button>
      {/if}
    </div>
  {/if}
</div>

<style>
.row-actions {
  position: relative;
  flex: 0 0 auto;
  align-self: center;
}
.row-actions-trigger {
  display: grid;
  box-sizing: border-box;
  width: 28px;
  height: 28px;
  min-width: 28px;
  min-height: 28px;
  max-width: 28px;
  max-height: 28px;
  place-items: center;
  padding: 0;
  border: 1px solid transparent;
  border-radius: 7px;
  background: transparent;
  color: var(--ink-faint);
  cursor: pointer;
}
.row-actions-trigger :global(svg) {
  width: 15px;
  height: 15px;
}
@media (pointer: coarse) {
  .row-actions-trigger {
    width: var(--touch-target-min, 44px);
    height: var(--touch-target-min, 44px);
    min-width: var(--touch-target-min, 44px);
    min-height: var(--touch-target-min, 44px);
    max-width: var(--touch-target-min, 44px);
    max-height: var(--touch-target-min, 44px);
  }
}
.row-actions-trigger:hover,
.row-actions-trigger:focus-visible,
.row-actions-trigger[aria-expanded="true"] {
  border-color: var(--line-strong);
  background: var(--canvas);
  color: var(--ink);
  outline: 0;
}
.row-actions-trigger:disabled {
  opacity: 0.4;
  cursor: not-allowed;
}
.row-actions-menu {
  position: fixed;
  z-index: 80;
  display: grid;
  min-width: 168px;
  gap: 2px;
  padding: 6px;
  border: 1px solid var(--line);
  border-radius: 10px;
  background: var(--surface);
  box-shadow: var(--shadow-md, 0 10px 28px rgba(38, 42, 33, 0.12));
}
.row-actions-menu button {
  display: block;
  box-sizing: border-box;
  width: 100%;
  min-height: 32px;
  padding: 0 10px;
  border: 0;
  border-radius: 7px;
  background: transparent;
  color: var(--ink);
  text-align: left;
  cursor: pointer;
  font-size: 12px;
}
@media (pointer: coarse) {
  .row-actions-menu button {
    min-height: var(--touch-target-min, 44px);
  }
}
.row-actions-menu button:hover,
.row-actions-menu button:focus-visible {
  background: var(--surface-muted, var(--canvas));
  outline: 0;
}
.row-actions-menu button.danger {
  color: var(--danger);
}
</style>
