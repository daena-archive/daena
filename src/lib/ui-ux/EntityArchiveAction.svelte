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
