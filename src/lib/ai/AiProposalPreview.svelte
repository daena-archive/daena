<script lang="ts">
let {
  original,
  proposal = $bindable(""),
  streamText = "",
  busy = false,
  onCancel,
  onDiscard,
  onAccept,
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
    <div>
      <span>Original</span>
      <pre>{original}</pre>
    </div>
    <div>
      <span>Editable proposal</span><textarea class="ai-proposal-editor" rows="8" bind:value={proposal}></textarea>
    </div>
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

<style>
.primary-button {
  padding: 10px 15px;
  border: 1px solid rgba(255, 255, 255, 0.08);
  border-radius: 8px;
  background: var(--accent-dark);
  color: #fff;
  font-size: 12px;
  font-weight: 700;
  box-shadow:
    0 2px 0 #263d30,
    0 7px 16px rgba(42, 68, 51, 0.16);
  cursor: pointer;
  transition:
    background 0.16s ease,
    box-shadow 0.16s ease,
    transform 0.16s ease;
}

.primary-button:hover {
  background: #2b4535;
  box-shadow:
    0 2px 0 #263d30,
    0 10px 20px rgba(42, 68, 51, 0.2);
  transform: translateY(-1px);
}
.primary-button:active {
  box-shadow:
    0 1px 0 #263d30,
    0 3px 8px rgba(42, 68, 51, 0.14);
  transform: translateY(1px);
}
.primary-button:focus-visible {
  outline: 3px solid rgba(180, 119, 63, 0.32);
  outline-offset: 2px;
}
.primary-button:disabled {
  opacity: 0.5;
  cursor: not-allowed;
  box-shadow: none;
  transform: none;
}

.quiet-button {
  padding: 10px 12px;
  border: 1px solid #ded8cd;
  border-radius: 8px;
  background: var(--surface);
  color: var(--ink-soft);
  font-size: 12px;
  box-shadow: 0 1px 2px rgba(48, 45, 38, 0.05);
  cursor: pointer;
  transition:
    background 0.16s ease,
    border-color 0.16s ease,
    box-shadow 0.16s ease,
    color 0.16s ease,
    transform 0.16s ease;
}

.quiet-button:hover {
  border-color: #cbbda9;
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

.ai-diff-grid pre {
  max-height: 220px;
  overflow: auto;
  margin: 0;
  padding: 11px;
  border: 1px solid var(--line);
  border-radius: 7px;
  background: var(--surface);
  color: var(--ink);
  white-space: pre-wrap;
  font: 12px/1.55 var(--font-body);
}

.ai-diff-grid > div > span {
  display: block;
  margin-bottom: 5px;
  color: var(--ink-faint);
  font-size: 10px;
  font-weight: 700;
  text-transform: uppercase;
}
</style>
