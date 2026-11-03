<script>
import RichTextEditor from "$lib/editor/RichTextEditor.svelte";

export let onChange = (entityId, value) => {};
let shown = true;
let entityId = "A";
let documents = { A: "", B: "" };
let editorRef;

export function setShown(next) {
  shown = next;
}

export function setEntity(next) {
  entityId = next;
}

export function flushEditor() {
  editorRef?.flushPendingChanges();
}

function handleChange(next) {
  documents = { ...documents, [entityId]: next };
  onChange(entityId, next);
}
</script>

{#if shown}
  {#key entityId}
    <RichTextEditor bind:this={editorRef} value={documents[entityId]} onChange={handleChange} />
  {/key}
{/if}
