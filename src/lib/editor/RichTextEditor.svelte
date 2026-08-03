<script lang="ts">
  import { onMount } from "svelte";

  export let value = "";
  export let placeholder = "Start writing…";
  export let onChange: (value: string) => void = () => {};

  let editor: HTMLDivElement;

  function sanitizeHtml(value: string): string {
    if (typeof document === "undefined") return value;
    const template = document.createElement("template");
    template.innerHTML = value;
    for (const node of template.content.querySelectorAll("script, style, iframe, object, embed, form")) node.remove();
    for (const element of template.content.querySelectorAll("*")) {
      for (const attribute of [...element.attributes]) {
        const name = attribute.name.toLowerCase();
        const content = attribute.value.trim().toLowerCase();
        if (name.startsWith("on") || name === "style" || ((name === "href" || name === "src") && content.startsWith("javascript:"))) element.removeAttribute(attribute.name);
      }
    }
    return template.innerHTML;
  }

  function run(command: string, argument?: string) {
    editor?.focus();
    document.execCommand(command, false, argument);
    onChange(editor?.innerHTML ?? "");
  }

  function handleInput() {
    const clean = sanitizeHtml(editor.innerHTML);
    if (clean !== editor.innerHTML) editor.innerHTML = clean;
    onChange(clean);
  }

  onMount(() => {
    if (editor) editor.innerHTML = sanitizeHtml(value);
  });

  $: if (editor && document.activeElement !== editor && editor.innerHTML !== value) {
    editor.innerHTML = sanitizeHtml(value);
  }
</script>

<div class="editor-shell">
  <div class="editor-toolbar" aria-label="Formatting tools">
    <button type="button" title="Bold" aria-label="Bold" onclick={() => run("bold")}><strong>B</strong></button>
    <button type="button" title="Italic" aria-label="Italic" onclick={() => run("italic")}><em>I</em></button>
    <button type="button" title="Underline" aria-label="Underline" onclick={() => run("underline")}><u>U</u></button>
    <span class="toolbar-divider"></span>
    <button type="button" title="Heading" aria-label="Heading" onclick={() => run("formatBlock", "h2")}>H2</button>
    <button type="button" title="Quote" aria-label="Quote" onclick={() => run("formatBlock", "blockquote")}>“</button>
    <button type="button" title="Bulleted list" aria-label="Bulleted list" onclick={() => run("insertUnorderedList")}>• list</button>
    <button type="button" title="Numbered list" aria-label="Numbered list" onclick={() => run("insertOrderedList")}>1. list</button>
    <span class="toolbar-spacer"></span>
    <span class="editor-hint">Markdown-friendly rich text</span>
  </div>
  <div
    class="editor-content"
    contenteditable="true"
    role="textbox"
    aria-multiline="true"
    aria-label="Document editor"
    data-placeholder={placeholder}
    bind:this={editor}
    oninput={handleInput}
  ></div>
</div>

<style>
  .editor-shell { overflow: hidden; border: 1px solid var(--line); border-radius: 14px; background: var(--surface); box-shadow: var(--shadow-sm); }
  .editor-toolbar { display: flex; align-items: center; gap: 4px; min-height: 48px; padding: 7px 10px; border-bottom: 1px solid var(--line); background: var(--surface-muted); }
  .editor-toolbar button { min-width: 31px; height: 31px; padding: 0 8px; border: 0; border-radius: 7px; background: transparent; color: var(--ink-soft); font: inherit; cursor: pointer; }
  .editor-toolbar button:hover, .editor-toolbar button:focus-visible { background: var(--surface); color: var(--accent); outline: 0; }
  .toolbar-divider { width: 1px; height: 22px; margin: 0 5px; background: var(--line); }
  .toolbar-spacer { flex: 1; }
  .editor-hint { color: var(--ink-faint); font-size: 11px; }
  .editor-content { min-height: 390px; padding: 28px 34px 42px; color: var(--ink); font: 400 17px/1.75 Georgia, serif; outline: 0; }
  .editor-content:empty::before { content: attr(data-placeholder); color: var(--ink-faint); pointer-events: none; }
  .editor-content :global(h2) { margin: 1.2em 0 .45em; font: 600 25px/1.2 var(--font-display); }
  .editor-content :global(blockquote) { margin: 1em 0; padding: 4px 0 4px 18px; border-left: 3px solid var(--accent); color: var(--ink-soft); }
  .editor-content :global(a) { color: var(--accent); }
  @media (max-width: 700px) { .editor-content { min-height: 300px; padding: 22px 20px 30px; font-size: 16px; } .editor-hint { display: none; } }
</style>
