<script lang="ts">
import { confirmDialog } from "$lib/dialogs.svelte";
import { archiveConfirmOptions, archivePendingLabel } from "./archive.ts";
import { ENTITY_ACTIONS } from "./vocabulary.ts";

let {
  entityName,
  busy = false,
  disabled = false,
  dirtyNote = "",
  dangerClass = "quiet-button",
  label = ENTITY_ACTIONS.archive,
  onArchive,
}: {
  entityName: string;
  busy?: boolean;
  disabled?: boolean;
  dirtyNote?: string;
  dangerClass?: string;
  label?: string;
  /** Called only after the shared archive confirmation succeeds. */
  onArchive: () => void | Promise<void>;
} = $props();

let confirming = $state(false);

async function onClick() {
  if (disabled || busy || confirming) return;
  confirming = true;
  try {
    if (!(await confirmDialog(archiveConfirmOptions(entityName, dirtyNote ? { dirtyNote } : undefined)))) return;
    await onArchive();
  } finally {
    confirming = false;
  }
}
</script>

<button type="button" class={dangerClass} disabled={disabled || busy || confirming} onclick={() => void onClick()}
  >{busy || confirming ? archivePendingLabel(true) : label}</button>

<style>
.quiet-button {
  padding: 10px 12px;
  border: 1px solid var(--theme-warning-border, #ded8cd);
  border-radius: 8px;
  background: var(--surface);
  color: var(--ink-soft);
  font-size: 12px;
  cursor: pointer;
  box-shadow: 0 1px 2px rgba(48, 45, 38, 0.05);
  transition:
    background 0.16s ease,
    border-color 0.16s ease,
    box-shadow 0.16s ease,
    color 0.16s ease,
    transform 0.16s ease;
}
.quiet-button:hover {
  border-color: var(--theme-warning-border, #cbbda9);
  background: var(--surface-muted);
  color: var(--ink);
  box-shadow: 0 3px 8px rgba(48, 45, 38, 0.08);
  transform: translateY(-1px);
}
.quiet-button:active {
  box-shadow: 0 1px 2px rgba(48, 45, 38, 0.05);
  transform: translateY(1px);
}
.quiet-button:focus-visible {
  outline: 3px solid rgba(180, 119, 63, 0.24);
  outline-offset: 2px;
}
.quiet-button:disabled {
  opacity: 0.5;
  cursor: not-allowed;
  box-shadow: none;
  transform: none;
}
</style>
