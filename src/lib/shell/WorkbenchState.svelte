<script lang="ts">
import type { Snippet } from "svelte";

interface Props {
  kind: "empty" | "loading" | "error" | "conflict";
  title: string;
  message: string;
  compact?: boolean;
  actions?: Snippet;
}

let { kind, title, message, compact = false, actions }: Props = $props();
</script>

<section
  class:compact
  class={`workbench-state state-${kind}`}
  role={kind === "error" || kind === "conflict" ? "alert" : "status"}
  aria-live={kind === "loading" ? "polite" : undefined}
  aria-busy={kind === "loading"}>
  <span class:state-spinner={kind === "loading"} class="state-mark" aria-hidden="true"
    >{kind === "empty" ? "✦" : kind === "error" ? "!" : kind === "conflict" ? "↯" : ""}</span>
  <div>
    <strong>{title}</strong>
    <p>{message}</p>
  </div>
  {#if actions}<div class="state-actions">{@render actions()}</div>{/if}
</section>

<style>
.workbench-state {
  display: grid;
  min-height: 320px;
  align-content: center;
  justify-items: center;
  gap: 10px;
  padding: 30px;
  color: var(--ink-faint);
  text-align: center;
}
.workbench-state.compact {
  min-height: 210px;
  padding: 24px 18px;
}
.state-mark {
  display: grid;
  width: 38px;
  height: 38px;
  place-items: center;
  border-radius: 50%;
  background: var(--surface-muted);
  color: var(--accent-dark);
  font-size: 18px;
}
.state-error .state-mark,
.state-conflict .state-mark {
  background: var(--danger-bg);
  color: var(--danger);
}
.state-spinner {
  border: 2px solid var(--line-strong);
  border-top-color: var(--accent-dark);
  background: transparent;
  animation: state-spin 0.8s linear infinite;
}
.workbench-state strong {
  display: block;
  color: var(--ink);
  font: 500 18px/1.2 var(--font-display);
}
.workbench-state p {
  max-width: 42ch;
  margin: 7px 0 0;
  font-size: 11px;
  line-height: 1.55;
}
.state-actions {
  display: flex;
  justify-content: center;
  gap: 7px;
  margin-top: 4px;
}
@keyframes state-spin {
  to {
    transform: rotate(360deg);
  }
}
@media (prefers-reduced-motion: reduce) {
  .state-spinner {
    animation: none;
  }
}
</style>
