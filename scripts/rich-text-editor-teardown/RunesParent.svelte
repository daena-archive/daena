<script lang="ts">
import RichTextEditor from "$lib/editor/RichTextEditor.svelte";

let { onChange }: { onChange: (entityId: string, value: string) => void } = $props();
let shown = $state(true);
let entityId = $state("A");
let documents = $state<Record<string, string>>({ A: "", B: "" });
let editorRef: RichTextEditor | undefined = $state();

export function setShown(next: boolean) {
  shown = next;
}

export function setEntity(next: string) {
  entityId = next;
}

export function flushEditor() {
  editorRef?.flushPendingChanges();
}

// Like the shell's updateDocumentBody: the write and the callback use the entity selected at call time.
function handleChange(next: string) {
  documents[entityId] = next;
  onChange(entityId, next);
}
</script>

{#if shown}
  {#key entityId}
    <RichTextEditor bind:this={editorRef} value={documents[entityId]} onChange={handleChange} />
  {/key}
{/if}
