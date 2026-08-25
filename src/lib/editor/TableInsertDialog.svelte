<script lang="ts">
import { tick } from "svelte";
import { X } from "@lucide/svelte";

export let open = false;
export let onConfirm: (options: { rows: number; cols: number }) => void = () => {};
export let onCancel: () => void = () => {};

const MIN = 1;
const MAX = 12;

let rows = 3;
let cols = 3;
let wasOpen = false;
let lastFocused: Element | null = null;
let rowsInput: HTMLInputElement | null = null;

$: {
  if (!open) {
    wasOpen = false;
  } else if (!wasOpen) {
    rows = 3;
    cols = 3;
    wasOpen = true;
    lastFocused = document.activeElement;
    void tick().then(() => {
      rowsInput?.focus();
      rowsInput?.select();
    });
  }
}

$: if (!open && lastFocused) {
  if (lastFocused instanceof HTMLElement && lastFocused.isConnected) lastFocused.focus();
  lastFocused = null;
}

function clamp(value: number) {
  if (!Number.isFinite(value)) return MIN;
  return Math.min(MAX, Math.max(MIN, Math.trunc(value)));
}

function submit() {
  onConfirm({
    rows: clamp(rows),
    cols: clamp(cols),
  });
}

function handleKeydown(event: KeyboardEvent) {
  if (event.key === "Escape") {
    event.preventDefault();
    event.stopPropagation();
    onCancel();
    return;
  }
  if (event.key === "Tab") {
    trapFocus(event);
    return;
  }
  if (event.key === "Enter") {
    if ((event.target as HTMLElement | null)?.closest("button")) return;
    event.preventDefault();
    submit();
  }
}

function trapFocus(event: KeyboardEvent) {
  const dialog = event.currentTarget as HTMLElement;
  const focusable = [
    ...dialog.querySelectorAll<HTMLElement>(
      'button:not([disabled]), input:not([disabled]), [tabindex]:not([tabindex="-1"])',
    ),
  ].filter((element) => !element.hasAttribute("hidden"));
  if (focusable.length === 0) return;
  const first = focusable[0];
  const last = focusable[focusable.length - 1];
  if (!dialog.contains(document.activeElement)) {
    event.preventDefault();
    (event.shiftKey ? last : first).focus();
  } else if (event.shiftKey && document.activeElement === first) {
    event.preventDefault();
    last.focus();
  } else if (!event.shiftKey && document.activeElement === last) {
    event.preventDefault();
    first.focus();
  }
}
</script>

{#if open}
  <div class="table-dialog-backdrop" role="presentation" onclick={onCancel}>
    <div
      class="table-dialog"
      role="dialog"
      aria-modal="true"
      aria-labelledby="table-dialog-title"
      tabindex="-1"
      onclick={(event) => event.stopPropagation()}
      onkeydown={handleKeydown}>
      <header>
        <div>
          <span class="panel-kicker">TABLE</span>
          <h2 id="table-dialog-title">Insert table</h2>
        </div>
        <button type="button" aria-label="Close table dialog" onclick={onCancel}
          ><X size={16} strokeWidth={1.8} aria-hidden="true" /></button>
      </header>

      <div class="table-size-grid">
        <label class="table-field">
          <span>Rows</span>
          <input
            bind:this={rowsInput}
            type="number"
            min={MIN}
            max={MAX}
            bind:value={rows}
            onchange={() => (rows = clamp(rows))} />
        </label>
        <label class="table-field">
          <span>Columns</span>
          <input type="number" min={MIN} max={MAX} bind:value={cols} onchange={() => (cols = clamp(cols))} />
        </label>
      </div>
      <p class="table-hint">Tables always include a header row so they stay portable as Markdown.</p>

      <footer>
        <button type="button" class="quiet" onclick={onCancel}>Cancel</button>
        <button type="button" class="primary" onclick={submit}>Insert table</button>
      </footer>
    </div>
  </div>
{/if}

<style>
.table-dialog-backdrop {
  position: fixed;
  inset: 0;
  z-index: 80;
  display: grid;
  place-items: center;
  padding: 18px;
  background: rgba(37, 37, 31, 0.28);
}
.table-dialog {
  width: min(360px, 100%);
  display: grid;
  gap: 14px;
  padding: 20px;
  border: 1px solid var(--line);
  border-radius: 12px;
  background: var(--surface);
  box-shadow: 0 24px 64px rgba(38, 42, 33, 0.24);
}
header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
}
.panel-kicker {
  display: block;
  color: var(--accent);
  font-size: 9px;
  font-weight: 700;
  letter-spacing: 0.08em;
  text-transform: uppercase;
}
h2 {
  margin: 3px 0 0;
  color: var(--ink);
  font: 700 18px/1.2 var(--font-display, Georgia, serif);
}
header button {
  width: 30px;
  height: 30px;
  border: 0;
  border-radius: 6px;
  background: transparent;
  color: var(--ink-soft);
  cursor: pointer;
}
header button:hover,
header button:focus-visible {
  background: var(--surface-muted);
  color: var(--ink);
  outline: 0;
}
.table-size-grid {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 12px;
}
.table-field {
  display: grid;
  gap: 6px;
}
.table-field span {
  color: var(--accent);
  font-size: 9px;
  font-weight: 700;
  letter-spacing: 0.08em;
  text-transform: uppercase;
}
.table-field input {
  width: 100%;
  height: 38px;
  padding: 0 10px;
  border: 1px solid var(--line);
  border-radius: 8px;
  background: var(--canvas);
  color: var(--ink);
  font: 500 14px/1.2 var(--font-body, system-ui, sans-serif);
}
.table-field input:focus {
  outline: 2px solid color-mix(in srgb, var(--accent) 35%, transparent);
  outline-offset: 1px;
}
.table-hint {
  margin: 0;
  color: var(--ink-faint);
  font: 12px/1.4 var(--font-body, system-ui, sans-serif);
}
footer {
  display: flex;
  justify-content: flex-end;
  gap: 8px;
  margin-top: 2px;
}
footer button {
  height: 34px;
  padding: 0 12px;
  border-radius: 8px;
  border: 1px solid var(--line);
  font: 600 13px/1 var(--font-body, system-ui, sans-serif);
  cursor: pointer;
}
footer .quiet {
  background: transparent;
  color: var(--ink-soft);
}
footer .quiet:hover,
footer .quiet:focus-visible {
  background: var(--surface-muted);
  color: var(--ink);
  outline: 0;
}
footer .primary {
  border-color: transparent;
  background: var(--accent);
  color: var(--accent-ink, #fff);
}
footer .primary:hover,
footer .primary:focus-visible {
  filter: brightness(0.96);
  outline: 0;
}
</style>
