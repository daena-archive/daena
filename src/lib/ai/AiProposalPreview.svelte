<script lang="ts">
let {
  original,
  proposal = $bindable(""),
  streamText = "",
  progressMessage = "Generating proposal…",
  busy = false,
  cancelling = false,
  onCancel,
  onDiscard,
  onAccept,
}: {
  original: string;
  proposal?: string;
  streamText?: string;
  progressMessage?: string;
  busy?: boolean;
  cancelling?: boolean;
  onCancel: () => void;
  onDiscard: () => void;
  onAccept: () => void;
} = $props();
</script>

{#if busy || proposal}
  <div class="ai-diff-grid" aria-label="AI proposal comparison">
    <section class="ai-diff-card ai-diff-original">
      <header class="ai-diff-card-header">
        <div><strong>Original</strong><small>Current text</small></div>
        <span class="ai-diff-badge">Read-only</span>
      </header>
      <pre>{original || "No existing text — this will be inserted at the cursor."}</pre>
    </section>
    <section class="ai-diff-card ai-diff-proposal">
      <header class="ai-diff-card-header">
        <div>
          <strong>Proposal</strong><small
            >{cancelling ? "Stopping generation" : busy ? progressMessage : "Edit before accepting"}</small>
        </div>
        <span class="ai-diff-badge">{cancelling ? "Stopping" : busy ? "Streaming" : "Editable"}</span>
      </header>
      {#if busy}
        <span class="sr-only" role="status" aria-live="polite"
          >{cancelling ? "Cancellation requested" : progressMessage}</span>
        <pre class="ai-proposal-output">{streamText || (cancelling ? "Stopping generation…" : progressMessage)}</pre>
      {:else}
        <textarea class="ai-proposal-editor" rows="9" bind:value={proposal} aria-label="Editable AI proposal"
        ></textarea>
      {/if}
    </section>
  </div>
{/if}

<div class="ai-rewrite-actions" class:ai-streaming-actions={busy}>
  {#if busy}
    <button class="quiet-button" type="button" onclick={onCancel} disabled={cancelling}
      >{cancelling ? "Cancelling…" : "Cancel"}</button>
  {/if}
</div>

<style>
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

.sr-only {
  position: absolute;
  width: 1px;
  height: 1px;
  padding: 0;
  margin: -1px;
  overflow: hidden;
  clip: rect(0, 0, 0, 0);
  white-space: nowrap;
  border: 0;
}

.ai-streaming-actions {
  display: flex;
  justify-content: center;
}

.ai-diff-grid {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 12px;
}

.ai-diff-card {
  display: grid;
  align-content: start;
  min-width: 0;
  gap: 10px;
  padding: 12px;
  border: 1px solid var(--line);
  border-radius: 10px;
  background: var(--surface);
}

.ai-diff-original {
  border-color: #d9d2c7;
  background: #f7f4ef;
}

.ai-diff-proposal {
  border-color: #c9d8ca;
  background: #f5faf5;
}

.ai-diff-card-header {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 10px;
}

.ai-diff-card-header div {
  display: grid;
  gap: 3px;
}

.ai-diff-card-header strong {
  color: var(--ink);
  font-size: 12px;
}

.ai-diff-card-header small {
  color: var(--ink-faint);
  font-size: 10px;
}

.ai-diff-badge {
  flex: 0 0 auto;
  padding: 3px 6px;
  border: 1px solid currentColor;
  border-radius: 999px;
  color: var(--ink-faint);
  font-size: 9px;
  font-weight: 700;
  letter-spacing: 0.04em;
  text-transform: uppercase;
}

.ai-diff-proposal .ai-diff-badge {
  color: #46704d;
}

.ai-diff-card pre,
.ai-proposal-editor {
  box-sizing: border-box;
  height: 220px;
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

.ai-proposal-output {
  border-color: #b9d0bb !important;
  background: #fff !important;
}

.ai-proposal-editor {
  width: 100%;
  min-height: 0;
  resize: vertical;
  outline: none;
}

.ai-proposal-editor:focus {
  border-color: var(--accent-dark);
  box-shadow: 0 0 0 3px rgba(70, 112, 77, 0.14);
}

@media (max-width: 620px) {
  .ai-diff-grid {
    grid-template-columns: 1fr;
  }
}
</style>
