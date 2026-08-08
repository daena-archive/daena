<script lang="ts">
  let {
    original,
    proposal = $bindable(""),
    streamText = "",
    busy = false,
    onCancel,
    onDiscard,
    onAccept
  }: {
    original: string;
    proposal?: string;
    streamText?: string;
    busy?: boolean;
    onCancel: () => void;
    onDiscard: () => void;
    onAccept: () => void;
  } = $props();
</script>

{#if busy}
  <pre class="ai-stream-output" aria-live="polite">{streamText || "Waiting for local AI…"}</pre>
{:else if proposal}
  <div class="ai-diff-grid">
    <div><span>Original</span><pre>{original}</pre></div>
    <div><span>Editable proposal</span><textarea class="ai-proposal-editor" rows="8" bind:value={proposal}></textarea></div>
  </div>
{/if}

<div class="ai-rewrite-actions">
  {#if busy}
    <button class="quiet-button" type="button" onclick={onCancel}>Cancel</button>
  {:else if proposal}
    <button class="quiet-button" type="button" onclick={onDiscard}>Discard</button>
    <button class="primary-button" type="button" onclick={onAccept}>Accept proposal</button>
  {:else}
    <button class="quiet-button" type="button" onclick={onCancel}>Cancel</button>
  {/if}
</div>
