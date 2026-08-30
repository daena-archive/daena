<script lang="ts">
import { MUTATION_STATUS_MESSAGES } from "./vocabulary.ts";
import type { MutationSnapshot } from "./mutationState.ts";

let {
  snapshot,
  onRetry,
  onReload,
  onReviewDraft,
}: {
  snapshot: MutationSnapshot;
  onRetry?: () => void;
  onReload?: () => void;
  onReviewDraft?: () => void;
} = $props();
</script>

{#if snapshot.phase !== "idle"}
  <div
    class="mutation-status"
    class:is-busy={snapshot.phase === "saving"}
    class:is-saved={snapshot.phase === "saved"}
    class:is-conflict={snapshot.phase === "conflict"}
    class:is-failed={snapshot.phase === "failed"}
    role={snapshot.phase === "conflict" || snapshot.phase === "failed" ? "alert" : "status"}
    aria-live={snapshot.phase === "saving" ? "polite" : undefined}
    aria-busy={snapshot.phase === "saving"}>
    <span class="mutation-mark" aria-hidden="true"></span>
    <div class="mutation-copy">
      <strong>{snapshot.message}</strong>
      {#if snapshot.detail}<p>{snapshot.detail}</p>{/if}
    </div>
    <div class="mutation-actions">
      {#if snapshot.phase === "conflict"}
        {#if onReload}
          <button type="button" class="quiet-button" onclick={onReload}
            >{MUTATION_STATUS_MESSAGES.conflictReload}</button>
        {/if}
        {#if onReviewDraft}
          <button type="button" class="quiet-button ghost" onclick={onReviewDraft}
            >{MUTATION_STATUS_MESSAGES.conflictReviewDraft}</button>
        {/if}
      {:else if snapshot.phase === "failed" && onRetry}
        <button type="button" class="quiet-button" onclick={onRetry}>{MUTATION_STATUS_MESSAGES.retry}</button>
      {/if}
    </div>
  </div>
{/if}

<style>
.mutation-status {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 8px;
  padding: 8px 10px;
  border: 1px solid var(--line);
  border-radius: 8px;
  background: var(--surface-muted);
  color: var(--ink-soft);
  font-size: 11px;
}
.mutation-status.is-conflict,
.mutation-status.is-failed {
  border-color: var(--danger-line);
  background: var(--danger-bg);
  color: var(--danger);
}
.mutation-status.is-saved {
  border-color: var(--success-line);
  background: var(--success-bg);
}
.mutation-mark {
  width: 7px;
  height: 7px;
  border-radius: 50%;
  background: var(--warning);
}
.is-busy .mutation-mark {
  animation: mutation-pulse 0.9s linear infinite;
}
.is-saved .mutation-mark {
  background: var(--success);
}
.is-conflict .mutation-mark,
.is-failed .mutation-mark {
  background: var(--danger);
}
.mutation-copy {
  display: grid;
  gap: 2px;
  min-width: 0;
  flex: 1;
}
.mutation-copy strong {
  color: inherit;
  font-weight: 650;
}
.mutation-copy p {
  margin: 0;
  color: var(--ink-faint);
  line-height: 1.4;
}
.mutation-actions {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
}
.quiet-button {
  min-height: 28px;
  padding: 0 10px;
  border: 1px solid var(--line-strong);
  border-radius: 7px;
  background: var(--surface);
  color: var(--ink-soft);
  cursor: pointer;
  font-size: 11px;
}
.quiet-button.ghost {
  background: transparent;
}
.quiet-button:hover,
.quiet-button:focus-visible {
  border-color: var(--accent);
  color: var(--ink);
  outline: 0;
}
@keyframes mutation-pulse {
  50% {
    opacity: 0.35;
  }
}
@media (prefers-reduced-motion: reduce) {
  .is-busy .mutation-mark {
    animation: none;
  }
}
</style>
