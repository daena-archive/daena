<script lang="ts">
import { tick } from "svelte";
import IpaPicker from "./IpaPicker.svelte";
import { insertIpaAtSelection } from "./ipa";

let {
  value = $bindable(),
  label,
  name,
  placeholder = "",
  multiline = false,
  rows = 3,
  required = false,
  disabled = false,
}: {
  value?: string;
  label: string;
  name?: string;
  placeholder?: string;
  multiline?: boolean;
  rows?: number;
  required?: boolean;
  disabled?: boolean;
} = $props();

let pickerOpen = $state(false);
let control: HTMLInputElement | HTMLTextAreaElement | undefined = $state();
let selectionStart = $state(0);
let selectionEnd = $state(0);

function rememberSelection() {
  const current = value ?? "";
  selectionStart = control?.selectionStart ?? current.length;
  selectionEnd = control?.selectionEnd ?? selectionStart;
}

function openPicker() {
  rememberSelection();
  pickerOpen = true;
}

function insert(symbol: string) {
  const current = value ?? "";
  const inserted = insertIpaAtSelection(current, symbol, selectionStart, selectionEnd);
  value = inserted.value;
  selectionStart = inserted.cursor;
  selectionEnd = selectionStart;
}

async function closePicker() {
  pickerOpen = false;
  await tick();
  control?.focus();
  control?.setSelectionRange(selectionStart, selectionEnd);
}
</script>

<div class="ipa-field">
  <span>{label}</span>
  <span class="ipa-control">
    {#if multiline}
      <textarea
        bind:this={control}
        bind:value
        {name}
        {placeholder}
        {rows}
        {required}
        {disabled}
        aria-label={label}
        onselect={rememberSelection}
        onkeyup={rememberSelection}
        onclick={rememberSelection}></textarea>
    {:else}
      <input
        bind:this={control}
        bind:value
        {name}
        {placeholder}
        {required}
        {disabled}
        aria-label={label}
        onselect={rememberSelection}
        onkeyup={rememberSelection}
        onclick={rememberSelection} />
    {/if}
    <button
      type="button"
      class="ipa-trigger"
      onclick={openPicker}
      {disabled}
      aria-label={`Open IPA picker for ${label}`}>IPA</button>
  </span>
</div>

{#if pickerOpen}
  <IpaPicker onselect={insert} onclose={closePicker} />
{/if}

<style>
.ipa-field {
  display: grid;
  gap: 6px;
  min-width: 0;
  color: var(--ink-soft);
  font-size: 11px;
  letter-spacing: 0.01em;
}
.ipa-control {
  display: grid;
  grid-template-columns: minmax(0, 1fr) auto;
  align-items: stretch;
  min-width: 0;
}
.ipa-control input,
.ipa-control textarea {
  box-sizing: border-box;
  width: 100%;
  min-width: 0;
  padding: 9px 10px;
  border: 1px solid var(--line);
  border-right: 0;
  border-radius: 8px 0 0 8px;
  background: var(--surface);
  color: var(--ink);
  font: inherit;
}
.ipa-control textarea {
  min-height: 4.5em;
  resize: vertical;
}
.ipa-trigger {
  padding: 7px 10px;
  border: 1px solid var(--line);
  border-radius: 0 8px 8px 0;
  background: var(--surface-muted);
  color: var(--accent-dark);
  font-size: 10px;
  font-weight: 700;
  letter-spacing: 0.04em;
  cursor: pointer;
}
.ipa-trigger:hover {
  background: var(--surface);
}
.ipa-trigger:disabled {
  cursor: not-allowed;
  opacity: 0.45;
}
.ipa-control input:focus-visible,
.ipa-control textarea:focus-visible,
.ipa-trigger:focus-visible {
  position: relative;
  z-index: 1;
  outline: 3px solid rgba(180, 119, 63, 0.24);
  outline-offset: 1px;
}
</style>
