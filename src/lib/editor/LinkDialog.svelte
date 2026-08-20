<script lang="ts">
  import { tick } from "svelte";
  import { X } from "@lucide/svelte";

  export let open = false;
  export let initialText = "";
  export let initialUrl = "";
  export let hasSelection = false;
  export let onConfirm: (text: string, url: string) => void = () => {};
  export let onCancel: () => void = () => {};
  export let onRemove: (() => void) | null = null;

  let text = "";
  let url = "";
  let wasOpen = false;
  let lastFocused: Element | null = null;
  let textInput: HTMLInputElement | null = null;
  let urlInput: HTMLInputElement | null = null;

  $: displayText = initialText.length > 120 ? initialText.slice(0, 120) + "…" : initialText;

  $: {
    if (!open) {
      wasOpen = false;
    } else if (!wasOpen) {
      text = initialText;
      url = initialUrl;
      wasOpen = true;
      lastFocused = document.activeElement;
      void tick().then(() => {
        if (hasSelection) urlInput?.focus();
        else textInput?.focus();
        // select content for easy replacement
        if (hasSelection) urlInput?.select();
        else if (text) textInput?.select();
      });
    }
  }

  $: if (!open && lastFocused) {
    if (lastFocused instanceof HTMLElement && lastFocused.isConnected) lastFocused.focus();
    lastFocused = null;
  }

  function submit() {
    const trimmedUrl = url.trim();
    const trimmedText = text.trim();
    if (!trimmedUrl) return;
    // when hasSelection, text is fixed to initialText (display), but allow trimmedText fallback?
    // if hasSelection we ignore text input and use initialText
    const finalText = hasSelection ? initialText : trimmedText;
    if (!hasSelection && !finalText) return;
    onConfirm(finalText, trimmedUrl);
  }

  function handleKeydown(event: KeyboardEvent) {
    if (event.key === "Escape") {
      event.preventDefault();
      onCancel();
    } else if (event.key === "Enter") {
      event.preventDefault();
      submit();
    }
  }
</script>

{#if open}
  <div class="link-dialog-backdrop" role="presentation" onclick={onCancel}>
    <div
      class="link-dialog"
      role="dialog"
      aria-modal="true"
      aria-labelledby="link-dialog-title"
      tabindex="-1"
      onclick={(event) => event.stopPropagation()}
      onkeydown={handleKeydown}>
      <header>
        <div>
          <span class="panel-kicker">LINK</span>
          <h2 id="link-dialog-title">{initialUrl ? "Edit link" : "Insert link"}</h2>
        </div>
        <button type="button" aria-label="Close link dialog" onclick={onCancel}><X size={16} strokeWidth={1.8} aria-hidden="true" /></button>
      </header>

      {#if hasSelection}
        <div class="link-preview">
          <span>Selected text</span>
          <p title={initialText}>{displayText}</p>
        </div>
        <label class="link-field">
          <span>Link URL</span>
          <input
            bind:this={urlInput}
            bind:value={url}
            placeholder="https://…"
            autocomplete="off"
            spellcheck="false" />
        </label>
      {:else}
        <label class="link-field">
          <span>Display text</span>
          <input
            bind:this={textInput}
            bind:value={text}
            placeholder="Text to display"
            autocomplete="off" />
        </label>
        <label class="link-field">
          <span>Link URL</span>
          <input
            bind:this={urlInput}
            bind:value={url}
            placeholder="https://…"
            autocomplete="off"
            spellcheck="false" />
        </label>
      {/if}

      <footer>
        {#if initialUrl && onRemove}
          <button type="button" class="quiet danger" onclick={onRemove}>Remove link</button>
          <span style="flex:1"></span>
        {/if}
        <button type="button" class="quiet" onclick={onCancel}>Cancel</button>
        <button
          type="button"
          class="primary"
          disabled={hasSelection ? !url.trim() : !text.trim() || !url.trim()}
          onclick={submit}>
          {initialUrl ? "Update link" : "Insert link"}
        </button>
      </footer>
    </div>
  </div>
{/if}

<style>
  .link-dialog-backdrop {
    position: fixed;
    inset: 0;
    z-index: 80;
    display: grid;
    place-items: center;
    padding: 18px;
    background: rgba(37, 37, 31, 0.28);
  }
  .link-dialog {
    width: min(440px, 100%);
    display: grid;
    gap: 14px;
    padding: 20px;
    border: 1px solid var(--line, #e4e1d8);
    border-radius: 12px;
    background: var(--surface, #fffefa);
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
    color: var(--accent, #b4773f);
    font-size: 9px;
    font-weight: 700;
    letter-spacing: 0.08em;
    text-transform: uppercase;
  }
  h2 {
    margin: 3px 0 0;
    color: var(--ink, #25251f);
    font: 700 18px/1.2 var(--font-display, Georgia, serif);
  }
  header button {
    width: 30px;
    height: 30px;
    border: 0;
    border-radius: 6px;
    background: transparent;
    color: var(--ink-soft, #77766d);
    cursor: pointer;
  }
  header button:hover,
  header button:focus-visible {
    background: var(--surface-muted, #f4f2ec);
    color: var(--ink, #25251f);
    outline: 0;
  }
  .link-preview {
    display: grid;
    gap: 6px;
    padding: 10px 12px;
    border: 1px solid var(--line, #e4e1d8);
    border-radius: 8px;
    background: var(--canvas, #f7f6f2);
  }
  .link-preview span {
    color: var(--accent, #b4773f);
    font-size: 9px;
    font-weight: 700;
    letter-spacing: 0.08em;
    text-transform: uppercase;
  }
  .link-preview p {
    margin: 0;
    color: var(--ink, #25251f);
    font: 500 13px/1.4 var(--font-body, system-ui, sans-serif);
    word-break: break-word;
    font-style: italic;
  }
  .link-field {
    display: grid;
    gap: 6px;
  }
  .link-field span {
    color: var(--accent, #b4773f);
    font-size: 9px;
    font-weight: 700;
    letter-spacing: 0.08em;
    text-transform: uppercase;
  }
  .link-field input {
    width: 100%;
    height: 38px;
    padding: 0 10px;
    border: 1px solid var(--line, #e4e1d8);
    border-radius: 7px;
    background: var(--canvas, #f7f6f2);
    color: var(--ink, #25251f);
    font: 500 13px/1 var(--font-body, system-ui, sans-serif);
    outline: 0;
  }
  .link-field input:focus {
    border-color: #c99965;
    box-shadow: 0 0 0 3px rgba(180, 119, 63, 0.1);
  }
  footer {
    display: flex;
    align-items: center;
    justify-content: flex-end;
    gap: 8px;
    margin-top: 2px;
  }
  footer button {
    min-height: 34px;
    padding: 0 14px;
    border: 0;
    border-radius: 7px;
    font: 700 12px/1 var(--font-body, system-ui, sans-serif);
    cursor: pointer;
  }
  footer .quiet {
    background: transparent;
    color: var(--ink-soft, #77766d);
  }
  footer .quiet:hover,
  footer .quiet:focus-visible {
    background: var(--surface-muted, #f4f2ec);
    color: var(--ink, #25251f);
    outline: 0;
  }
  footer .quiet.danger {
    color: #a14f42;
  }
  footer .quiet.danger:hover {
    background: #fdf0ed;
    color: #8a3a2f;
  }
  footer .primary {
    background: var(--accent-dark, #365342);
    color: white;
  }
  footer .primary:disabled {
    opacity: 0.45;
    cursor: not-allowed;
  }
</style>
