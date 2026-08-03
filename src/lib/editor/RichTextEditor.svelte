<script lang="ts">
  import { Editor } from "@tiptap/core";
  import Blockquote from "@tiptap/extension-blockquote";
  import Bold from "@tiptap/extension-bold";
  import Document from "@tiptap/extension-document";
  import Heading from "@tiptap/extension-heading";
  import Italic from "@tiptap/extension-italic";
  import { BulletList, ListItem, OrderedList } from "@tiptap/extension-list";
  import Paragraph from "@tiptap/extension-paragraph";
  import Text from "@tiptap/extension-text";
  import Underline from "@tiptap/extension-underline";
  import { UndoRedo } from "@tiptap/extensions";
  import { onMount } from "svelte";

  export let value = "";
  export let placeholder = "Start writing…";
  export let onChange: (value: string) => void = () => {};

  let editorElement: HTMLDivElement;
  let editor: Editor | null = null;
  let editorState: Editor | null = null;
  let currentHtml = "";

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

  function emitChange() {
    if (!editor) return;
    currentHtml = editor.getHTML();
    onChange(currentHtml);
  }

  function run(command: (currentEditor: Editor) => boolean) {
    if (editorState) command(editorState);
  }

  function focusEditorSurface(event: MouseEvent) {
    if (event.target === event.currentTarget) editor?.commands.focus();
  }

  onMount(() => {
    editor = new Editor({
      element: editorElement,
      extensions: [
        Document,
        Paragraph,
        Text,
        Bold,
        Italic,
        Underline,
        Heading,
        Blockquote,
        BulletList,
        OrderedList,
        ListItem,
        UndoRedo,
      ],
      content: sanitizeHtml(value),
      editorProps: {
        attributes: {
          "aria-label": "Document editor",
          "aria-multiline": "true",
          spellcheck: "true",
        },
      },
      onUpdate: () => emitChange(),
      onTransaction: ({ editor: nextEditor }) => {
        editorState = nextEditor;
      },
    });
    editorState = editor;
    currentHtml = editor.getHTML();

    return () => editor?.destroy();
  });

  $: if (editor && !editor.isFocused && value !== currentHtml) {
    const nextHtml = sanitizeHtml(value);
    if (nextHtml !== editor.getHTML()) editor.commands.setContent(nextHtml, { emitUpdate: false });
    currentHtml = editor.getHTML();
  }
</script>

<div class="editor-shell">
  <div class="editor-toolbar" role="toolbar" aria-label="Formatting tools">
    <button type="button" title="Bold" aria-label="Bold" class:active={editorState?.isActive("bold")} onclick={() => run((currentEditor) => currentEditor.chain().focus().toggleBold().run())}><strong>B</strong></button>
    <button type="button" title="Italic" aria-label="Italic" class:active={editorState?.isActive("italic")} onclick={() => run((currentEditor) => currentEditor.chain().focus().toggleItalic().run())}><em>I</em></button>
    <button type="button" title="Underline" aria-label="Underline" class:active={editorState?.isActive("underline")} onclick={() => run((currentEditor) => currentEditor.chain().focus().toggleUnderline().run())}><u>U</u></button>
    <span class="toolbar-divider"></span>
    <button type="button" title="Heading" aria-label="Heading" class:active={editorState?.isActive("heading", { level: 2 })} onclick={() => run((currentEditor) => currentEditor.chain().focus().toggleHeading({ level: 2 }).run())}>H2</button>
    <button type="button" title="Quote" aria-label="Quote" class:active={editorState?.isActive("blockquote")} onclick={() => run((currentEditor) => currentEditor.chain().focus().toggleBlockquote().run())}>“</button>
    <button type="button" title="Bulleted list" aria-label="Bulleted list" class:active={editorState?.isActive("bulletList")} onclick={() => run((currentEditor) => currentEditor.chain().focus().toggleBulletList().run())}>• list</button>
    <button type="button" title="Numbered list" aria-label="Numbered list" class:active={editorState?.isActive("orderedList")} onclick={() => run((currentEditor) => currentEditor.chain().focus().toggleOrderedList().run())}>1. list</button>
    <span class="toolbar-spacer"></span>
    <span class="editor-hint">Rich text</span>
  </div>
  <div
    class="editor-content"
    class:is-empty={editorState?.isEmpty}
    data-placeholder={placeholder}
    aria-placeholder={placeholder}
    role="textbox"
    aria-multiline="true"
    tabindex="0"
    bind:this={editorElement}
    onmousedown={focusEditorSurface}
  ></div>
</div>

<style>
  .editor-shell { overflow: hidden; border: 1px solid var(--line); border-radius: 14px; background: var(--surface); box-shadow: var(--shadow-sm); }
  .editor-toolbar { display: flex; align-items: center; gap: 4px; min-height: 48px; padding: 7px 10px; border-bottom: 1px solid var(--line); background: var(--surface-muted); }
  .editor-toolbar button { min-width: 31px; height: 31px; padding: 0 8px; border: 0; border-radius: 7px; background: transparent; color: var(--ink-soft); font: inherit; cursor: pointer; }
  .editor-toolbar button:hover, .editor-toolbar button:focus-visible, .editor-toolbar button.active { background: var(--surface); color: var(--accent); outline: 0; }
  .toolbar-divider { width: 1px; height: 22px; margin: 0 5px; background: var(--line); }
  .toolbar-spacer { flex: 1; }
  .editor-hint { color: var(--ink-faint); font-size: 11px; }
  .editor-content { min-height: 390px; padding: 28px 34px 42px; color: var(--ink); font: 400 17px/1.75 Georgia, serif; outline: 0; cursor: text; }
  .editor-content.is-empty::before { content: attr(data-placeholder); color: var(--ink-faint); pointer-events: none; }
  .editor-content :global(h2) { margin: 1.2em 0 .45em; font: 600 25px/1.2 var(--font-display); }
  .editor-content :global(blockquote) { margin: 1em 0; padding: 4px 0 4px 18px; border-left: 3px solid var(--accent); color: var(--ink-soft); }
  .editor-content :global(a) { color: var(--accent); }
  @media (max-width: 700px) { .editor-content { min-height: 300px; padding: 22px 20px 30px; font-size: 16px; } .editor-hint { display: none; } }
  @media (max-width: 520px) { .editor-toolbar { flex-wrap: wrap; gap: 2px; padding: 7px; } .editor-toolbar button { min-width: 29px; padding-inline: 6px; } .toolbar-divider { margin-inline: 2px; } }
</style>
