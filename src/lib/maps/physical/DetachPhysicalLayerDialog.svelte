<script lang="ts">
import { onMount, tick } from "svelte";
import type { PhysicalDetachScope } from "./detach";

let {
  sourceLayerName,
  epochOffsetYears,
  selectedFeatureCount,
  totalSourceLayerFeatureCount,
  initialScope,
  busy = false,
  error = "",
  onconfirm,
  oncancel,
}: {
  sourceLayerName: string;
  epochOffsetYears: number;
  selectedFeatureCount: number;
  totalSourceLayerFeatureCount: number;
  initialScope: PhysicalDetachScope;
  busy?: boolean;
  error?: string;
  onconfirm: (scope: PhysicalDetachScope) => void;
  oncancel: () => void;
} = $props();
let dialog = $state<HTMLDivElement | null>(null);
let selectedRadio = $state<HTMLInputElement | null>(null);
let confirmButton = $state<HTMLButtonElement | null>(null);
const hasSelectionChoice = $derived(selectedFeatureCount > 0 && selectedFeatureCount < totalSourceLayerFeatureCount);
let scope = $state<PhysicalDetachScope>("layer");
const count = $derived(scope === "selected" ? selectedFeatureCount : totalSourceLayerFeatureCount);
const epoch = $derived(
  epochOffsetYears === 0
    ? "the reference epoch"
    : `${epochOffsetYears > 0 ? "+" : ""}${epochOffsetYears.toLocaleString()} years`,
);
$effect(() => {
  scope = initialScope;
});
onMount(() => {
  const opener = document.activeElement instanceof HTMLElement ? document.activeElement : null;
  void tick().then(() => (hasSelectionChoice ? selectedRadio : confirmButton)?.focus());
  return () => opener?.focus();
});
function trapFocus(event: KeyboardEvent) {
  if (event.key === "Escape") {
    event.preventDefault();
    oncancel();
    return;
  }
  if (event.key !== "Tab" || !dialog) return;
  const focusable = [
    ...dialog.querySelectorAll<HTMLElement>(
      'button:not([disabled]), input:not([disabled]), [href], select:not([disabled]), textarea:not([disabled]), [tabindex]:not([tabindex="-1"])',
    ),
  ];
  if (focusable.length === 0) return;
  const index = focusable.indexOf(document.activeElement as HTMLElement);
  const next = event.shiftKey
    ? index <= 0
      ? focusable.length - 1
      : index - 1
    : index === focusable.length - 1
      ? 0
      : index + 1;
  event.preventDefault();
  focusable[next].focus();
}
</script>

<div
  class="backdrop"
  role="presentation"
  onclick={(event) => {
    if (event.target === event.currentTarget && !busy) oncancel();
  }}>
  <div
    bind:this={dialog}
    class="dialog"
    role="dialog"
    tabindex="-1"
    aria-modal="true"
    aria-labelledby="detach-title"
    onkeydown={trapFocus}>
    <h2 id="detach-title">Detach {sourceLayerName} for editing?</h2>
    <p>
      Daena will copy {count} generated features from {epoch} into a new authored layer and hide {sourceLayerName}. The
      copy will no longer follow epoch changes or physical re-derivation. The accepted physical world will not be
      changed.
    </p>
    {#if hasSelectionChoice}
      <label
        ><input bind:this={selectedRadio} type="radio" name="detach-scope" value="selected" bind:group={scope} />
        Selected features ({selectedFeatureCount})</label>
      <label
        ><input type="radio" name="detach-scope" value="layer" bind:group={scope} /> Entire layer ({totalSourceLayerFeatureCount})</label>
    {:else}<p>Entire layer ({totalSourceLayerFeatureCount})</p>{/if}
    {#if error}<p class="error" role="alert">{error}</p>{/if}
    <div class="actions">
      <button type="button" disabled={busy} onclick={oncancel}>Cancel</button><button
        bind:this={confirmButton}
        type="button"
        disabled={busy}
        onclick={() => onconfirm(scope)}>Detach snapshot</button>
    </div>
  </div>
</div>

<style>
.backdrop {
  position: fixed;
  inset: 0;
  z-index: 10000;
  display: grid;
  place-items: center;
  padding: 1rem;
  background: rgb(4 10 16 / 0.7);
}
.dialog {
  width: min(34rem, 100%);
  display: grid;
  gap: 0.8rem;
  padding: 1.25rem;
  border: 1px solid #385064;
  border-radius: 0.7rem;
  background: #142433;
  color: #f7f0e5;
  box-shadow: 0 1rem 3rem rgb(0 0 0 / 0.4);
}
h2,
p {
  margin: 0;
}
label {
  display: flex;
  gap: 0.55rem;
  align-items: center;
}
.actions {
  display: flex;
  justify-content: end;
  gap: 0.6rem;
}
.error {
  color: #ffb4a9;
}
</style>
