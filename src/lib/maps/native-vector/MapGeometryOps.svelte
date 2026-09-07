<script lang="ts">
import { Scissors } from "@lucide/svelte";
import {
  canRunOperation,
  type GeometryOpProgress,
  type GeometryOperationKind,
} from "../editor/geometry-operation-kinds";
import type { VectorFeature } from "./types";

let {
  features,
  preview,
  progress = null,
  bufferDistance = $bindable(),
  simplifyTolerance = $bindable(),
  notice,
  onrun,
  oncommit,
  oncancel,
}: {
  features: VectorFeature[];
  preview: { label: string } | null;
  progress?: GeometryOpProgress | null;
  bufferDistance: string;
  simplifyTolerance: string;
  notice: string;
  onrun: (kind: GeometryOperationKind) => void;
  oncommit: () => void;
  oncancel: () => void;
} = $props();

const percent = $derived(progress && progress.total > 0 ? Math.round((100 * progress.completed) / progress.total) : 0);
</script>

<div class="geometry-ops" aria-label="Geometry operations">
  {#if progress && !preview}
    <p class="section-note" role="status">{progress.label}… {percent}%</p>
    <div class="quick-add-row">
      <button type="button" class="quiet-button small" onclick={() => oncancel()}>Cancel</button>
    </div>
  {:else if preview}
    <p class="section-note">Preview: {preview.label}. Commit or cancel to finish.</p>
    <div class="quick-add-row">
      <button type="button" class="primary-button small" onclick={() => oncommit()}>Apply</button>
      <button type="button" class="quiet-button small" onclick={() => oncancel()}>Cancel</button>
    </div>
  {:else}
    <div class="quick-add-row">
      <button
        type="button"
        class="quiet-button small"
        disabled={!canRunOperation("union", features)}
        onclick={() => onrun("union")}>Union</button>
      <button
        type="button"
        class="quiet-button small"
        disabled={!canRunOperation("difference", features)}
        onclick={() => onrun("difference")}>Diff</button>
      <button
        type="button"
        class="quiet-button small"
        disabled={!canRunOperation("intersection", features)}
        onclick={() => onrun("intersection")}>Intersect</button>
    </div>
    <div class="quick-add-row">
      <button
        type="button"
        class="quiet-button small"
        disabled={!canRunOperation("split", features)}
        onclick={() => onrun("split")}><Scissors size={12} strokeWidth={1.8} /> Split</button>
      <button
        type="button"
        class="quiet-button small"
        disabled={!canRunOperation("reverse", features)}
        onclick={() => onrun("reverse")}>Reverse</button>
      <button
        type="button"
        class="quiet-button small"
        disabled={!canRunOperation("merge-lines", features)}
        onclick={() => onrun("merge-lines")}>Merge</button>
      <label class="inline-field"
        ><span>Buffer</span><input
          type="number"
          min="0"
          step="any"
          bind:value={bufferDistance}
          aria-label="Buffer distance" /></label>
      <button
        type="button"
        class="quiet-button small"
        disabled={!canRunOperation("buffer", features)}
        onclick={() => onrun("buffer")}>Run</button>
    </div>
    <div class="quick-add-row">
      <label class="inline-field"
        ><span>Simplify</span><input
          type="number"
          min="0"
          step="any"
          bind:value={simplifyTolerance}
          aria-label="Simplify tolerance" /></label>
      <button
        type="button"
        class="quiet-button small"
        disabled={!canRunOperation("simplify", features)}
        onclick={() => onrun("simplify")}>Run</button>
    </div>
    {#if notice}<p class="field-hint" role="status">{notice}</p>{/if}
  {/if}
</div>

<style>
.geometry-ops {
  display: grid;
  gap: 8px;
}
.section-note {
  margin: 0;
  color: var(--ink-faint);
  font-size: 11px;
  line-height: 1.45;
}
.quick-add-row {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
  align-items: center;
}
.quick-add-row .quiet-button.small,
.quick-add-row .primary-button.small {
  padding: 6px 9px;
  border-radius: 7px;
  font-weight: 650;
  font-size: 11px;
  font-family: var(--font-body, Inter, ui-sans-serif, system-ui, sans-serif);
}
.quiet-button.small {
  border: 1px solid var(--line, #e4e1d8);
  background: var(--surface, #fffefa);
  color: var(--ink-soft);
  cursor: pointer;
}
.quiet-button.small:hover:not(:disabled) {
  border-color: var(--line-strong);
  background: var(--surface-muted);
  color: var(--ink);
}
.quiet-button.small:disabled {
  opacity: 0.45;
  cursor: not-allowed;
}
.primary-button.small {
  border: 1px solid var(--accent, #b4773f);
  background: var(--accent, #b4773f);
  color: var(--on-accent, #fffefa);
  cursor: pointer;
}
.primary-button.small:hover:not(:disabled) {
  filter: brightness(0.96);
}
.field-hint {
  margin: 0;
  color: var(--ink-faint);
  font-size: 10px;
  line-height: 1.4;
}
.inline-field {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  color: var(--ink-soft);
  font-weight: 500;
  font-size: 11px;
  font-family: var(--font-body, Inter, ui-sans-serif, system-ui, sans-serif);
}
.inline-field input {
  width: 84px;
  padding: 6px 7px;
  border: 1px solid var(--line, #e4e1d8);
  border-radius: 7px;
  background: var(--surface, #fffefa);
  color: var(--ink);
  font-weight: 500;
  font-size: 11px;
  font-family: var(--font-body, Inter, ui-sans-serif, system-ui, sans-serif);
}
</style>
