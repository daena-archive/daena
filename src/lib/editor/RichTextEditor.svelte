<script lang="ts">
  import { Editor } from "@tiptap/core";
  import Code from "@tiptap/extension-code";
  import CodeBlock from "@tiptap/extension-code-block";
  import Blockquote from "@tiptap/extension-blockquote";
  import Bold from "@tiptap/extension-bold";
  import Document from "@tiptap/extension-document";
  import Heading from "@tiptap/extension-heading";
  import HorizontalRule from "@tiptap/extension-horizontal-rule";
  import Italic from "@tiptap/extension-italic";
  import Link from "@tiptap/extension-link";
  import { BulletList, ListItem, OrderedList } from "@tiptap/extension-list";
  import Paragraph from "@tiptap/extension-paragraph";
  import Strike from "@tiptap/extension-strike";
  import TextAlign from "@tiptap/extension-text-align";
  import Text from "@tiptap/extension-text";
  import Underline from "@tiptap/extension-underline";
  import { UndoRedo } from "@tiptap/extensions";
  import { onMount } from "svelte";

  export let value = "";
  export let placeholder = "Start writing…";
  export let onChange: (value: string) => void = () => {};
  export let fullscreen = false;
  export let onFullscreenChange: (value: boolean) => void = () => {};

  let editorElement: HTMLDivElement;
  let editor: Editor | null = null;
  let editorState: Editor | null = null;
  let currentHtml = "";
  let editorText = "";
  let isFullscreen = false;
  $: wordCountValue = editorText.trim() ? editorText.trim().split(/\s+/).length : 0;
  $: characterCountValue = editorText.length;
  $: if (fullscreen !== isFullscreen) isFullscreen = fullscreen;

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
    editorText = editor.view.dom.textContent ?? "";
    currentHtml = editor.getHTML();
    onChange(currentHtml);
  }

  function run(command: (currentEditor: Editor) => boolean) {
    if (editorState) command(editorState);
  }

  function setFullscreen(nextValue: boolean) {
    if (isFullscreen === nextValue) return;
    isFullscreen = nextValue;
    onFullscreenChange(isFullscreen);
  }

  function toggleFullscreen() {
    setFullscreen(!isFullscreen);
  }

  function handleFullscreenKeydown(event: KeyboardEvent) {
    if (event.key === "Escape" && isFullscreen) {
      event.preventDefault();
      setFullscreen(false);
    }
  }

  function focusEditorSurface(event: MouseEvent) {
    if (event.target === event.currentTarget) editor?.commands.focus();
  }

  function blockStyle(): string {
    if (!editorState) return "paragraph";
    if (editorState.isActive("heading", { level: 1 })) return "heading-1";
    if (editorState.isActive("heading", { level: 2 })) return "heading-2";
    if (editorState.isActive("heading", { level: 3 })) return "heading-3";
    if (editorState.isActive("blockquote")) return "blockquote";
    if (editorState.isActive("codeBlock")) return "codeBlock";
    return "paragraph";
  }

  function changeBlockStyle(event: Event) {
    const nextStyle = (event.currentTarget as HTMLSelectElement).value;
    run((currentEditor) => {
      const chain = currentEditor.chain().focus();
      if (nextStyle === "paragraph") return chain.setParagraph().run();
      if (nextStyle === "heading-1") return chain.setHeading({ level: 1 }).run();
      if (nextStyle === "heading-2") return chain.setHeading({ level: 2 }).run();
      if (nextStyle === "heading-3") return chain.setHeading({ level: 3 }).run();
      if (nextStyle === "blockquote") return chain.toggleBlockquote().run();
      return chain.toggleCodeBlock().run();
    });
  }

  function setLink() {
    if (!editorState) return;
    const previousUrl = editorState.getAttributes("link").href ?? "";
    const nextUrl = window.prompt("Enter a link URL", previousUrl);
    if (nextUrl === null) return;
    const url = nextUrl.trim();
    if (!url) {
      editorState.chain().focus().unsetLink().run();
      return;
    }
    editorState.chain().focus().extendMarkRange("link").setLink({ href: url, target: "_blank", rel: "noopener noreferrer" }).run();
  }

  function isAligned(alignment: string): boolean {
    if (!editorState) return false;
    return editorState.getAttributes("paragraph").textAlign === alignment || editorState.getAttributes("heading").textAlign === alignment;
  }

  onMount(() => {
    window.addEventListener("keydown", handleFullscreenKeydown);
    editor = new Editor({
      element: editorElement,
      extensions: [
        Document,
        Paragraph,
        Text,
        Bold,
        Italic,
        Underline,
        Strike,
        Code,
        Heading,
        Blockquote,
        CodeBlock,
        HorizontalRule,
        BulletList,
        OrderedList,
        ListItem,
        Link.configure({ openOnClick: false, autolink: true, linkOnPaste: true }),
        TextAlign.configure({ types: ["heading", "paragraph"], alignments: ["left", "center", "right"] }),
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
        editorText = nextEditor.view.dom.textContent ?? "";
      },
    });
    editorState = editor;
    currentHtml = editor.getHTML();
    editorText = editor.view.dom.textContent ?? "";
    const initialTextFrame = requestAnimationFrame(() => {
      editorText = editor?.view.dom.textContent ?? "";
    });

    return () => {
      window.removeEventListener("keydown", handleFullscreenKeydown);
      cancelAnimationFrame(initialTextFrame);
      editor?.destroy();
    };
  });

  $: if (editor && !editor.isFocused && value !== currentHtml) {
    const nextHtml = sanitizeHtml(value);
    if (nextHtml !== editor.getHTML()) editor.commands.setContent(nextHtml, { emitUpdate: false });
    currentHtml = editor.getHTML();
    editorText = editor.view.dom.textContent ?? "";
  }
</script>

<div class="editor-shell">
  <div class="editor-toolbar" role="toolbar" aria-label="Formatting tools">
    <div class="toolbar-group" aria-label="History">
      <button class="history-button" type="button" title="Undo (⌘/Ctrl + Z)" aria-label="Undo" disabled={!editorState?.can().undo()} onclick={() => run((currentEditor) => currentEditor.chain().focus().undo().run())}>↶</button>
      <button class="history-button" type="button" title="Redo (⌘/Ctrl + Shift + Z)" aria-label="Redo" disabled={!editorState?.can().redo()} onclick={() => run((currentEditor) => currentEditor.chain().focus().redo().run())}>↷</button>
    </div>
    <span class="toolbar-divider"></span>

    <div class="toolbar-group">
      <label class="sr-only" for="block-style">Text style</label>
      <select id="block-style" class="style-select" aria-label="Text style" value={blockStyle()} onchange={changeBlockStyle}>
        <option value="paragraph">Normal text</option>
        <option value="heading-1">Heading 1</option>
        <option value="heading-2">Heading 2</option>
        <option value="heading-3">Heading 3</option>
        <option value="blockquote">Quote</option>
        <option value="codeBlock">Code block</option>
      </select>
    </div>
    <span class="toolbar-divider"></span>

    <div class="toolbar-group" aria-label="Text formatting">
      <button type="button" title="Bold (⌘/Ctrl + B)" aria-label="Bold" aria-pressed={editorState?.isActive("bold") ?? false} class:active={editorState?.isActive("bold")} onclick={() => run((currentEditor) => currentEditor.chain().focus().toggleBold().run())}><strong>B</strong></button>
      <button type="button" title="Italic (⌘/Ctrl + I)" aria-label="Italic" aria-pressed={editorState?.isActive("italic") ?? false} class:active={editorState?.isActive("italic")} onclick={() => run((currentEditor) => currentEditor.chain().focus().toggleItalic().run())}><em>I</em></button>
      <button type="button" title="Underline (⌘/Ctrl + U)" aria-label="Underline" aria-pressed={editorState?.isActive("underline") ?? false} class:active={editorState?.isActive("underline")} onclick={() => run((currentEditor) => currentEditor.chain().focus().toggleUnderline().run())}><u>U</u></button>
      {#if isFullscreen}
        <button type="button" title="Strikethrough" aria-label="Strikethrough" aria-pressed={editorState?.isActive("strike") ?? false} class:active={editorState?.isActive("strike")} onclick={() => run((currentEditor) => currentEditor.chain().focus().toggleStrike().run())}><s>S</s></button>
        <button type="button" title="Inline code" aria-label="Inline code" aria-pressed={editorState?.isActive("code") ?? false} class:active={editorState?.isActive("code")} onclick={() => run((currentEditor) => currentEditor.chain().focus().toggleCode().run())}><code>&lt;/&gt;</code></button>
        <button type="button" title="Add or edit link" aria-label="Add or edit link" aria-pressed={editorState?.isActive("link") ?? false} class:active={editorState?.isActive("link")} onclick={setLink}>↗</button>
      {/if}
    </div>
    {#if isFullscreen}
      <span class="toolbar-divider"></span>

      <div class="toolbar-group" aria-label="Paragraph alignment">
        <button type="button" title="Align left" aria-label="Align left" aria-pressed={isAligned("left")} class:active={isAligned("left")} onclick={() => run((currentEditor) => currentEditor.chain().focus().setTextAlign("left").run())}>≡</button>
        <button type="button" title="Align center" aria-label="Align center" aria-pressed={isAligned("center")} class:active={isAligned("center")} onclick={() => run((currentEditor) => currentEditor.chain().focus().setTextAlign("center").run())}>☰</button>
        <button type="button" title="Align right" aria-label="Align right" aria-pressed={isAligned("right")} class:active={isAligned("right")} onclick={() => run((currentEditor) => currentEditor.chain().focus().setTextAlign("right").run())}>≡</button>
      </div>
      <span class="toolbar-divider"></span>
    {/if}

    <div class="toolbar-group" aria-label="Lists and blocks">
      <button type="button" title="Bulleted list" aria-label="Bulleted list" aria-pressed={editorState?.isActive("bulletList") ?? false} class:active={editorState?.isActive("bulletList")} onclick={() => run((currentEditor) => currentEditor.chain().focus().toggleBulletList().run())}>•≡</button>
      <button type="button" title="Numbered list" aria-label="Numbered list" aria-pressed={editorState?.isActive("orderedList") ?? false} class:active={editorState?.isActive("orderedList")} onclick={() => run((currentEditor) => currentEditor.chain().focus().toggleOrderedList().run())}>1≡</button>
      {#if isFullscreen}
        <button type="button" title="Quote" aria-label="Quote" aria-pressed={editorState?.isActive("blockquote") ?? false} class:active={editorState?.isActive("blockquote")} onclick={() => run((currentEditor) => currentEditor.chain().focus().toggleBlockquote().run())}>“</button>
        <button type="button" title="Code block" aria-label="Code block" aria-pressed={editorState?.isActive("codeBlock") ?? false} class:active={editorState?.isActive("codeBlock")} onclick={() => run((currentEditor) => currentEditor.chain().focus().toggleCodeBlock().run())}>&lt;/&gt;</button>
        <button type="button" title="Horizontal rule" aria-label="Horizontal rule" onclick={() => run((currentEditor) => currentEditor.chain().focus().setHorizontalRule().run())}>—</button>
        <button type="button" title="Clear formatting" aria-label="Clear formatting" onclick={() => run((currentEditor) => currentEditor.chain().focus().clearNodes().unsetAllMarks().run())}>Tx</button>
      {/if}
    </div>
    <button class="fullscreen-toggle" type="button" title={isFullscreen ? "Exit full screen editor (Esc)" : "Open full screen editor"} aria-label={isFullscreen ? "Exit full screen editor" : "Open full screen editor"} aria-pressed={isFullscreen} onclick={toggleFullscreen}>{isFullscreen ? "×" : "⛶"}</button>
  </div>

  <div
    class="editor-content"
    class:is-empty={editorState?.isEmpty}
    data-placeholder={placeholder}
    role="presentation"
    bind:this={editorElement}
    onmousedown={focusEditorSurface}
  ></div>

  <div class="editor-statusbar" aria-live="polite">
    <span>{wordCountValue} {wordCountValue === 1 ? "word" : "words"}</span>
    <span class="status-separator">·</span>
    <span>{characterCountValue} {characterCountValue === 1 ? "character" : "characters"}</span>
    <span class="status-spacer"></span>
    <span class="editor-mode">Rich text</span>
  </div>
</div>

<style>
  .editor-shell { display: grid; grid-template-rows: auto minmax(390px, 1fr) auto; overflow: hidden; border: 1px solid var(--line, #e4e1d8); border-radius: 10px; background: var(--surface, #fffefa); box-shadow: var(--shadow-sm, 0 2px 8px rgba(38, 42, 33, .05)); }
  .editor-toolbar { display: flex; flex-wrap: wrap; align-items: center; gap: 2px; min-height: 48px; padding: 6px 10px; border-bottom: 1px solid var(--line, #e4e1d8); background: var(--surface-muted, #f4f2ec); }
  .toolbar-group { display: inline-flex; align-items: center; gap: 2px; }
  .editor-toolbar button { display: inline-flex; align-items: center; justify-content: center; min-width: 32px; height: 32px; padding: 0 8px; border: 1px solid transparent; border-radius: 6px; background: transparent; color: var(--ink-soft, #77766d); font: 500 13px/1 var(--font-body, system-ui, sans-serif); cursor: pointer; }
  .editor-toolbar button:hover, .editor-toolbar button:focus-visible, .editor-toolbar button.active { border-color: #d3c0a9; background: #f2e4d2; color: var(--accent-dark, #365342); outline: 0; }
  .editor-toolbar button:disabled { color: var(--ink-faint, #aaa79d); cursor: not-allowed; opacity: .65; }
  .editor-toolbar button:disabled:hover { border-color: transparent; background: transparent; }
  .editor-toolbar button code { font-size: 11px; }
  .toolbar-group[aria-label="History"] { gap: 4px; }
  .editor-toolbar button.history-button { width: 36px; min-width: 36px; padding: 0; font-family: "Apple Symbols", "Segoe UI Symbol", sans-serif; font-size: 21px; font-weight: 400; line-height: 1; }
  .fullscreen-toggle { margin-left: auto; font-size: 16px !important; }
  .toolbar-divider { width: 1px; height: 22px; margin: 0 6px; background: var(--line, #e4e1d8); }
  .style-select { height: 32px; min-width: 112px; padding: 0 8px; border: 1px solid transparent; border-radius: 6px; background: transparent; color: var(--ink-soft, #77766d); font: 500 12px/1 var(--font-body, system-ui, sans-serif); cursor: pointer; }
  .style-select:hover, .style-select:focus-visible { border-color: #d3c0a9; background: var(--surface, #fffefa); outline: 0; }
  .editor-content { position: relative; min-width: 0; padding: 24px 26px 36px; color: var(--ink, #25251f); background: var(--canvas, #f7f6f2); font: 400 16px/1.7 var(--font-body, ui-sans-serif, system-ui, sans-serif); outline: 0; cursor: text; }
  .editor-shell:focus-within { border-color: #d3c0a9; }
  .editor-content :global(.ProseMirror) { min-height: 100%; outline: 0; }
  .editor-content :global(.ProseMirror:focus), .editor-content :global(.ProseMirror:focus-visible) { outline: 0; box-shadow: none; }
  .editor-content.is-empty::before { position: absolute; top: 24px; right: 26px; left: 26px; content: attr(data-placeholder); color: var(--ink-faint, #aaa79d); pointer-events: none; }
  .editor-content:focus-within::before { content: none; }
  .editor-content :global(p) { margin: 0 0 1em; }
  .editor-content :global(h1), .editor-content :global(h2), .editor-content :global(h3) { color: var(--ink, #25251f); font-family: var(--font-display, Georgia, serif); line-height: 1.25; }
  .editor-content :global(h1) { margin: .2em 0 .55em; font-size: 2.05em; }
  .editor-content :global(h2) { margin: 1.1em 0 .45em; font-size: 1.55em; }
  .editor-content :global(h3) { margin: 1em 0 .4em; font-size: 1.25em; }
  .editor-content :global(ul), .editor-content :global(ol) { margin: 0 0 1em; padding-left: 1.5em; }
  .editor-content :global(li) { padding-left: .25em; }
  .editor-content :global(blockquote) { margin: 1.2em 0; padding: 7px 16px; border-left: 3px solid var(--accent, #b4773f); background: #fcf8f1; color: var(--ink-soft, #77766d); font-style: italic; }
  .editor-content :global(pre) { overflow-x: auto; margin: 1.2em 0; padding: 13px 15px; border: 1px solid var(--line, #e4e1d8); border-radius: 7px; background: var(--surface-muted, #f4f2ec); color: var(--ink, #25251f); font: 13px/1.6 ui-monospace, SFMono-Regular, Menlo, Consolas, monospace; }
  .editor-content :global(code) { padding: .1em .3em; border-radius: 4px; background: #ede9e0; color: #765a39; font: .86em ui-monospace, SFMono-Regular, Menlo, Consolas, monospace; }
  .editor-content :global(pre code) { padding: 0; background: transparent; color: inherit; font-size: inherit; }
  .editor-content :global(hr) { margin: 2em 0; border: 0; border-top: 1px solid var(--line, #e4e1d8); }
  .editor-content :global(a) { color: var(--accent-dark, #365342); text-decoration: underline; text-underline-offset: 2px; }
  .editor-statusbar { display: flex; align-items: center; gap: 6px; min-height: 34px; padding: 7px 13px; border-top: 1px solid var(--line, #e4e1d8); background: var(--surface-muted, #f4f2ec); color: var(--ink-faint, #aaa79d); font: 11px/1.3 var(--font-body, system-ui, sans-serif); }
  .status-separator { color: var(--ink-faint, #aaa79d); }
  .status-spacer { flex: 1; }
  .editor-mode { color: var(--ink-faint, #aaa79d); }
  .sr-only { position: absolute; width: 1px; height: 1px; padding: 0; overflow: hidden; clip: rect(0, 0, 0, 0); white-space: nowrap; border: 0; }
  @media (max-width: 760px) { .editor-toolbar { padding: 7px; } .toolbar-divider { margin-inline: 3px; } .editor-mode { display: none; } }
  @media (max-width: 560px) { .editor-shell { grid-template-rows: auto minmax(300px, 1fr) auto; } .editor-toolbar { overflow-x: auto; flex-wrap: nowrap; } .fullscreen-toggle { position: sticky; right: 0; flex: 0 0 32px; background: var(--surface-muted, #f4f2ec) !important; box-shadow: -6px 0 8px var(--surface-muted, #f4f2ec); } .editor-content { min-height: 300px; padding: 20px 16px 30px; font-size: 15px; } .editor-content.is-empty::before { top: 20px; right: 16px; left: 16px; } .style-select { min-width: 104px; } }
</style>
