<script lang="ts">
import { Editor, Mark, getMarkRange } from "@tiptap/core";
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
import { htmlToMarkdown, markdownToHtml } from "$lib/editor/markdown";
import type { Entity } from "$lib/project/client";
import EntityReferenceDialog from "$lib/editor/EntityReferenceDialog.svelte";

const EntityReference = Mark.create({
  name: "entityReference",
  inclusive: false,
  addAttributes() {
    return {
      entityId: {
        default: null,
        parseHTML: (element) => element.getAttribute("data-entity-id"),
        renderHTML: (attributes) => (attributes.entityId ? { "data-entity-id": attributes.entityId } : {}),
      },
    };
  },
  parseHTML() {
    return [{ tag: "a[data-entity-id]" }];
  },
  renderHTML({ HTMLAttributes }) {
    return ["a", { ...HTMLAttributes, class: "entity-reference" }, 0];
  },
});

const ExternalLink = Link.extend({
  parseHTML() {
    return [{ tag: "a[href]:not([data-entity-id])" }];
  },
});

export let value = "";
export let placeholder = "Start writing…";
export let onChange: (value: string) => void = () => {};
export let onSelectionChange: (markdown: string, plainText: string) => void = () => {};
export let onAiRequest: (
  action: "rewrite" | "generate" | "concise" | "expand" | "grammar" | "tone" | "custom",
  markdown: string,
  plainText: string,
  context: string,
) => void = () => {};
export let editable = true;
export let fullscreen = false;
export let onFullscreenChange: (value: boolean) => void = () => {};
export let entities: Entity[] = [];

let editorElement: HTMLDivElement;
let editor: Editor | null = null;
let editorState: Editor | null = null;
let currentMarkdown = "";
let editorText = "";
let selectionText = "";
let selectionMarkdown = "";
let aiMenuOpen = false;
let entityReferenceMenuOpen = false;
let entityReferenceQuery = "";
let entityReferenceRange: { from: number; to: number } | null = null;
let entityReferenceMenuPosition = { top: 0, left: 0 };
let entityReferenceDialogOpen = false;
let entityReferenceDialogMode: "insert" | "edit" = "insert";
let entityReferenceEdit: {
  entityId: string;
  label: string;
  from: number;
  to: number;
  top: number;
  left: number;
} | null = null;
let aiRequestRange: { from: number; to: number } | null = null;
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
      if (name.startsWith("on") || ((name === "href" || name === "src") && content.startsWith("javascript:")))
        element.removeAttribute(attribute.name);
      if (name === "style" && !/^text-align\s*:\s*(?:left|center|right)\s*;?$/i.test(attribute.value.trim()))
        element.removeAttribute(attribute.name);
    }
  }
  return template.innerHTML;
}

function emitChange() {
  if (!editor) return;
  editorText = editor.view.dom.textContent ?? "";
  currentMarkdown = htmlToMarkdown(editor.getHTML());
  onChange(currentMarkdown);
}

function emitSelection() {
  if (!editor) return;
  const { from, to } = editor.state.selection;
  if (from === to) {
    selectionText = "";
    selectionMarkdown = "";
    onSelectionChange("", "");
    return;
  }
  const plainText = editor.state.doc.textBetween(from, to, "\n");
  selectionText = plainText;
  try {
    const start = editor.view.domAtPos(from);
    const end = editor.view.domAtPos(to);
    const range = document.createRange();
    range.setStart(start.node, start.offset);
    range.setEnd(end.node, end.offset);
    const wrapper = document.createElement("div");
    wrapper.appendChild(range.cloneContents());
    selectionMarkdown = htmlToMarkdown(wrapper.innerHTML);
    onSelectionChange(selectionMarkdown, plainText);
  } catch {
    selectionMarkdown = plainText;
    onSelectionChange(plainText, plainText);
  }
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

function requestAi(action: "rewrite" | "generate" | "concise" | "expand" | "grammar" | "tone" | "custom") {
  if (!editor || (action !== "generate" && !selectionText.trim())) return;
  const { from, to } = editor.state.selection;
  aiRequestRange = { from, to };
  aiMenuOpen = false;
  const beforeCursor = editor.state.doc.textBetween(0, from, "\n").slice(-8000);
  const afterCursor = editor.state.doc.textBetween(to, editor.state.doc.content.size, "\n").slice(0, 8000);
  const cursorContext = `${beforeCursor}\n[CURSOR]\n${afterCursor}`.trim();
  onAiRequest(action, selectionMarkdown, selectionText, action === "generate" ? cursorContext : selectionMarkdown);
}

export function insertAiTextAtRequest(value: string): boolean {
  if (!editor || !aiRequestRange) return false;
  const inserted = editor.chain().focus().insertContentAt(aiRequestRange, value).run();
  aiRequestRange = null;
  return inserted;
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
  editorState
    .chain()
    .focus()
    .extendMarkRange("link")
    .setLink({ href: url, target: "_blank", rel: "noopener noreferrer" })
    .run();
}

function insertEntityReference(entity: Entity, label: string) {
  if (!editorState || !editable) return;
  const range = entityReferenceRange;
  if (!range) return;
  editorState
    .chain()
    .focus()
    .insertContentAt(range, {
      type: "text",
      text: label,
      marks: [{ type: "entityReference", attrs: { entityId: entity.id } }],
    })
    .run();
  entityReferenceMenuOpen = false;
  entityReferenceDialogOpen = false;
  entityReferenceQuery = "";
  entityReferenceRange = null;
}

function entityReferenceRangeAt(position: number) {
  if (!editorState) return null;
  const $position = editorState.state.doc.resolve(position);
  const type = editorState.state.schema.marks.entityReference;
  return type ? getMarkRange($position, type) : null;
}

function entityReferenceRangeForSelection(from: number, to: number) {
  const positions = [from, from - 1, to, to - 1].filter((position) => position >= 0);
  for (const position of positions) {
    const range = entityReferenceRangeAt(position);
    if (range && from <= range.to && to >= range.from) return range;
  }
  return null;
}

function showEntityReferenceEditor(entityId: string, label: string, position: number, bounds: DOMRect) {
  const range = entityReferenceRangeAt(position);
  if (!range) return;
  entityReferenceEdit = { entityId, label, ...range, top: bounds.top - 36, left: bounds.left };
}

function openEntityReferenceEditor() {
  if (!entityReferenceEdit) return;
  entityReferenceDialogMode = "edit";
  entityReferenceDialogOpen = true;
}

function saveEntityReference(entity: Entity, label: string) {
  if (entityReferenceDialogMode === "edit") {
    const reference = entityReferenceEdit;
    if (!editorState || !reference) return;
    editorState
      .chain()
      .focus()
      .insertContentAt(
        { from: reference.from, to: reference.to },
        { type: "text", text: label, marks: [{ type: "entityReference", attrs: { entityId: entity.id } }] },
      )
      .run();
    entityReferenceEdit = null;
    entityReferenceDialogOpen = false;
    return;
  }
  insertEntityReference(entity, label);
}

function openEntityReferenceDialog() {
  if (!entityReferenceRange) return;
  entityReferenceMenuOpen = false;
  entityReferenceDialogMode = "insert";
  entityReferenceDialogOpen = true;
}

function cancelEntityReference() {
  if (entityReferenceDialogMode === "edit") {
    entityReferenceEdit = null;
    entityReferenceDialogOpen = false;
    return;
  }
  if (editorState && entityReferenceRange) editorState.chain().focus().deleteRange(entityReferenceRange).run();
  entityReferenceMenuOpen = false;
  entityReferenceDialogOpen = false;
  entityReferenceQuery = "";
  entityReferenceRange = null;
}

function updateEntityReferenceTrigger(nextEditor: Editor) {
  const { from, to } = nextEditor.state.selection;
  if (!editable || from !== to) {
    entityReferenceMenuOpen = false;
    entityReferenceRange = null;
    return;
  }
  const beforeCursor = nextEditor.state.doc.textBetween(Math.max(0, from - 80), from, "\n");
  const trigger = beforeCursor.match(/@([^\s@]*)$/);
  const triggerStart = trigger ? beforeCursor.length - trigger[0].length : -1;
  const preceding = triggerStart > 0 ? beforeCursor[triggerStart - 1] : "";
  if (!trigger || (preceding && !/[\s([{]/.test(preceding))) {
    entityReferenceMenuOpen = false;
    entityReferenceRange = null;
    return;
  }
  const coords = nextEditor.view.coordsAtPos(from);
  entityReferenceMenuPosition = {
    top: coords.bottom + 6,
    left: coords.left,
  };
  entityReferenceQuery = trigger[1];
  entityReferenceRange = { from: from - trigger[0].length, to: from };
  entityReferenceMenuOpen = true;
}

function syncEntityReferenceEditor(nextEditor: Editor) {
  const { from, to } = nextEditor.state.selection;
  if (from !== to || entityReferenceDialogOpen) return;
  const dom = nextEditor.view.domAtPos(from).node;
  const element = (dom.nodeType === Node.TEXT_NODE ? dom.parentElement : (dom as Element | null))?.closest<HTMLElement>(
    "a[data-entity-id]",
  );
  const entityId = element?.dataset.entityId;
  if (!element || !entityId) {
    entityReferenceEdit = null;
    return;
  }
  showEntityReferenceEditor(entityId, element.textContent ?? "", from, element.getBoundingClientRect());
}

function isAligned(alignment: string): boolean {
  if (!editorState) return false;
  return (
    editorState.getAttributes("paragraph").textAlign === alignment ||
    editorState.getAttributes("heading").textAlign === alignment
  );
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
      ExternalLink.configure({ openOnClick: false, autolink: true, linkOnPaste: true }),
      EntityReference,
      TextAlign.configure({ types: ["heading", "paragraph"], alignments: ["left", "center", "right"] }),
      UndoRedo,
    ],
    content: sanitizeHtml(markdownToHtml(value)),
    editable,
    editorProps: {
      attributes: {
        "aria-label": "Document editor",
        "aria-multiline": "true",
        spellcheck: "true",
      },
      handleClick: (_view, _position, event) => {
        const anchor = (event.target as HTMLElement | null)?.closest<HTMLAnchorElement>("a[data-entity-id]");
        const entityId = anchor?.dataset.entityId;
        if (!entityId) return false;
        event.preventDefault();
        showEntityReferenceEditor(entityId, anchor.textContent ?? "", _position, anchor.getBoundingClientRect());
        return true;
      },
      handleDOMEvents: {
        mouseover: (_view, event) => {
          const anchor = (event.target as HTMLElement | null)?.closest<HTMLAnchorElement>("a[data-entity-id]");
          const entityId = anchor?.dataset.entityId;
          if (anchor && entityId) {
            showEntityReferenceEditor(
              entityId,
              anchor.textContent ?? "",
              _view.posAtDOM(anchor, 0),
              anchor.getBoundingClientRect(),
            );
          }
          return false;
        },
      },
      handleKeyDown: (_view, event) => {
        if (entityReferenceMenuOpen && (event.key === "Enter" || event.key === "Escape")) {
          event.preventDefault();
          if (event.key === "Enter") openEntityReferenceDialog();
          else cancelEntityReference();
          return true;
        }
        if (event.key === "Backspace" || event.key === "Delete") {
          const { from, to } = editorState?.state.selection ?? { from: 0, to: 0 };
          const range = entityReferenceRangeForSelection(from, to);
          if (range && editorState && from === to && from === range.to) {
            event.preventDefault();
            editorState.chain().focus().deleteRange(range).run();
            entityReferenceEdit = null;
            return true;
          }
        }
        return false;
      },
    },
    onUpdate: () => emitChange(),
    onTransaction: ({ editor: nextEditor }) => {
      editorState = nextEditor;
      editorText = nextEditor.view.dom.textContent ?? "";
      emitSelection();
      updateEntityReferenceTrigger(nextEditor);
      syncEntityReferenceEditor(nextEditor);
    },
  });
  editorState = editor;
  currentMarkdown = htmlToMarkdown(editor.getHTML());
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

$: if (editor && !editor.isFocused && value !== currentMarkdown) {
  const nextHtml = sanitizeHtml(markdownToHtml(value));
  if (nextHtml !== editor.getHTML()) editor.commands.setContent(nextHtml, { emitUpdate: false });
  currentMarkdown = htmlToMarkdown(editor.getHTML());
  editorText = editor.view.dom.textContent ?? "";
}
$: if (editor && editor.isEditable !== editable) editor.setEditable(editable);
</script>

<div class="editor-shell">
  <div class="editor-toolbar" role="toolbar" aria-label="Formatting tools">
    <div class="toolbar-group" aria-label="History">
      <button
        class="history-button"
        type="button"
        title="Undo (⌘/Ctrl + Z)"
        aria-label="Undo"
        disabled={!editorState?.can().undo()}
        onclick={() => run((currentEditor) => currentEditor.chain().focus().undo().run())}>↶</button>
      <button
        class="history-button"
        type="button"
        title="Redo (⌘/Ctrl + Shift + Z)"
        aria-label="Redo"
        disabled={!editorState?.can().redo()}
        onclick={() => run((currentEditor) => currentEditor.chain().focus().redo().run())}>↷</button>
    </div>
    <span class="toolbar-divider"></span>

    <div class="toolbar-group">
      <label class="sr-only" for="block-style">Text style</label>
      <select
        id="block-style"
        class="style-select"
        aria-label="Text style"
        value={blockStyle()}
        onchange={changeBlockStyle}>
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
      <button
        type="button"
        title="Bold (⌘/Ctrl + B)"
        aria-label="Bold"
        aria-pressed={editorState?.isActive("bold") ?? false}
        class:active={editorState?.isActive("bold")}
        onclick={() => run((currentEditor) => currentEditor.chain().focus().toggleBold().run())}
        ><strong>B</strong></button>
      <button
        type="button"
        title="Italic (⌘/Ctrl + I)"
        aria-label="Italic"
        aria-pressed={editorState?.isActive("italic") ?? false}
        class:active={editorState?.isActive("italic")}
        onclick={() => run((currentEditor) => currentEditor.chain().focus().toggleItalic().run())}><em>I</em></button>
      <button
        type="button"
        title="Underline (⌘/Ctrl + U)"
        aria-label="Underline"
        aria-pressed={editorState?.isActive("underline") ?? false}
        class:active={editorState?.isActive("underline")}
        onclick={() => run((currentEditor) => currentEditor.chain().focus().toggleUnderline().run())}><u>U</u></button>
      {#if isFullscreen}
        <button
          type="button"
          title="Strikethrough"
          aria-label="Strikethrough"
          aria-pressed={editorState?.isActive("strike") ?? false}
          class:active={editorState?.isActive("strike")}
          onclick={() => run((currentEditor) => currentEditor.chain().focus().toggleStrike().run())}><s>S</s></button>
        <button
          type="button"
          title="Inline code"
          aria-label="Inline code"
          aria-pressed={editorState?.isActive("code") ?? false}
          class:active={editorState?.isActive("code")}
          onclick={() => run((currentEditor) => currentEditor.chain().focus().toggleCode().run())}
          ><code>&lt;/&gt;</code></button>
        <button
          type="button"
          title="Add or edit link"
          aria-label="Add or edit link"
          aria-pressed={editorState?.isActive("link") ?? false}
          class:active={editorState?.isActive("link")}
          onclick={setLink}>↗</button>
      {/if}
    </div>
    {#if isFullscreen}
      <span class="toolbar-divider"></span>

      <div class="toolbar-group" aria-label="Paragraph alignment">
        <button
          type="button"
          title="Align left"
          aria-label="Align left"
          aria-pressed={isAligned("left")}
          class:active={isAligned("left")}
          onclick={() => run((currentEditor) => currentEditor.chain().focus().setTextAlign("left").run())}>≡</button>
        <button
          type="button"
          title="Align center"
          aria-label="Align center"
          aria-pressed={isAligned("center")}
          class:active={isAligned("center")}
          onclick={() => run((currentEditor) => currentEditor.chain().focus().setTextAlign("center").run())}>☰</button>
        <button
          type="button"
          title="Align right"
          aria-label="Align right"
          aria-pressed={isAligned("right")}
          class:active={isAligned("right")}
          onclick={() => run((currentEditor) => currentEditor.chain().focus().setTextAlign("right").run())}>≡</button>
      </div>
      <span class="toolbar-divider"></span>
    {/if}

    <div class="toolbar-group" aria-label="Lists and blocks">
      <button
        type="button"
        title="Bulleted list"
        aria-label="Bulleted list"
        aria-pressed={editorState?.isActive("bulletList") ?? false}
        class:active={editorState?.isActive("bulletList")}
        onclick={() => run((currentEditor) => currentEditor.chain().focus().toggleBulletList().run())}>•≡</button>
      <button
        type="button"
        title="Numbered list"
        aria-label="Numbered list"
        aria-pressed={editorState?.isActive("orderedList") ?? false}
        class:active={editorState?.isActive("orderedList")}
        onclick={() => run((currentEditor) => currentEditor.chain().focus().toggleOrderedList().run())}>1≡</button>
      {#if isFullscreen}
        <button
          type="button"
          title="Quote"
          aria-label="Quote"
          aria-pressed={editorState?.isActive("blockquote") ?? false}
          class:active={editorState?.isActive("blockquote")}
          onclick={() => run((currentEditor) => currentEditor.chain().focus().toggleBlockquote().run())}>“</button>
        <button
          type="button"
          title="Code block"
          aria-label="Code block"
          aria-pressed={editorState?.isActive("codeBlock") ?? false}
          class:active={editorState?.isActive("codeBlock")}
          onclick={() => run((currentEditor) => currentEditor.chain().focus().toggleCodeBlock().run())}
          >&lt;/&gt;</button>
        <button
          type="button"
          title="Horizontal rule"
          aria-label="Horizontal rule"
          onclick={() => run((currentEditor) => currentEditor.chain().focus().setHorizontalRule().run())}>—</button>
        <button
          type="button"
          title="Clear formatting"
          aria-label="Clear formatting"
          onclick={() => run((currentEditor) => currentEditor.chain().focus().clearNodes().unsetAllMarks().run())}
          >Tx</button>
      {/if}
    </div>
    <span class="toolbar-divider"></span>
    <div class="ai-toolbar-menu-control">
      <button
        class="ai-toolbar-button"
        type="button"
        title="Ask AI"
        aria-label="Ask AI"
        aria-haspopup="menu"
        aria-expanded={aiMenuOpen}
        disabled={!editable}
        onmousedown={(event) => event.preventDefault()}
        onclick={() => (aiMenuOpen = !aiMenuOpen)}>✦</button>
      {#if aiMenuOpen}
        <div class="ai-toolbar-menu" role="menu" aria-label="Ask AI">
          <button
            type="button"
            role="menuitem"
            disabled={!selectionText.trim()}
            onmousedown={(event) => event.preventDefault()}
            onclick={() => requestAi("rewrite")}>Rewrite selection</button>
          <button
            type="button"
            role="menuitem"
            onmousedown={(event) => event.preventDefault()}
            onclick={() => requestAi("generate")}>Generate text</button>
          <button
            type="button"
            role="menuitem"
            disabled={!selectionText.trim()}
            onmousedown={(event) => event.preventDefault()}
            onclick={() => requestAi("concise")}>Make concise</button>
          <button
            type="button"
            role="menuitem"
            disabled={!selectionText.trim()}
            onmousedown={(event) => event.preventDefault()}
            onclick={() => requestAi("expand")}>Expand</button>
          <button
            type="button"
            role="menuitem"
            disabled={!selectionText.trim()}
            onmousedown={(event) => event.preventDefault()}
            onclick={() => requestAi("grammar")}>Fix grammar</button>
          <button
            type="button"
            role="menuitem"
            disabled={!selectionText.trim()}
            onmousedown={(event) => event.preventDefault()}
            onclick={() => requestAi("tone")}>Change tone</button>
          <button
            type="button"
            role="menuitem"
            onmousedown={(event) => event.preventDefault()}
            onclick={() => requestAi("custom")}>Custom instruction</button>
        </div>
      {/if}
    </div>
    <button
      class="fullscreen-toggle"
      type="button"
      title={isFullscreen ? "Exit full screen editor (Esc)" : "Open full screen editor"}
      aria-label={isFullscreen ? "Exit full screen editor" : "Open full screen editor"}
      aria-pressed={isFullscreen}
      onclick={toggleFullscreen}>{isFullscreen ? "×" : "⛶"}</button>
  </div>

  <div
    class="editor-content"
    class:is-empty={editorState?.isEmpty}
    data-placeholder={placeholder}
    role="presentation"
    bind:this={editorElement}
    onmousedown={focusEditorSurface}>
  </div>
  {#if entityReferenceMenuOpen}
    <button
      type="button"
      class="entity-reference-menu"
      style={`top: ${entityReferenceMenuPosition.top}px; left: ${entityReferenceMenuPosition.left}px;`}
      onmousedown={(event) => event.preventDefault()}
      onclick={openEntityReferenceDialog}>
      <span>@</span> Link to another entity <kbd>↵</kbd>
    </button>
  {/if}
  {#if entityReferenceEdit && !entityReferenceDialogOpen}
    <button
      type="button"
      class="entity-reference-edit"
      style={`top: ${entityReferenceEdit.top}px; left: ${entityReferenceEdit.left}px;`}
      onmousedown={(event) => event.preventDefault()}
      onclick={openEntityReferenceEditor}>Edit reference</button>
  {/if}

  <div class="editor-statusbar" aria-live="polite">
    <span>{wordCountValue} {wordCountValue === 1 ? "word" : "words"}</span>
    <span class="status-separator">·</span>
    <span>{characterCountValue} {characterCountValue === 1 ? "character" : "characters"}</span>
    <span class="status-spacer"></span>
    <span class="editor-mode">Markdown</span>
  </div>
  <EntityReferenceDialog
    open={entityReferenceDialogOpen}
    {entities}
    initialQuery={entityReferenceDialogMode === "insert" ? entityReferenceQuery : ""}
    initialSelectedId={entityReferenceDialogMode === "edit" ? (entityReferenceEdit?.entityId ?? "") : ""}
    initialLabel={entityReferenceDialogMode === "edit" ? (entityReferenceEdit?.label ?? "") : ""}
    onInsert={saveEntityReference}
    onCancel={cancelEntityReference} />
</div>

<style>
.editor-shell {
  position: relative;
  display: grid;
  grid-template-rows: auto minmax(390px, 1fr) auto;
  overflow: hidden;
  border: 1px solid var(--line, #e4e1d8);
  border-radius: 10px;
  background: var(--surface, #fffefa);
  box-shadow: var(--shadow-sm, 0 2px 8px rgba(38, 42, 33, 0.05));
}
.editor-toolbar {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 2px;
  min-height: 48px;
  padding: 6px 10px;
  border-bottom: 1px solid var(--line, #e4e1d8);
  background: var(--surface-muted, #f4f2ec);
}
.toolbar-group {
  display: inline-flex;
  align-items: center;
  gap: 2px;
}
.editor-toolbar button {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  min-width: 32px;
  height: 32px;
  padding: 0 8px;
  border: 1px solid transparent;
  border-radius: 6px;
  background: transparent;
  color: var(--ink-soft, #77766d);
  font: 500 13px/1 var(--font-body, system-ui, sans-serif);
  cursor: pointer;
}
.editor-toolbar button:hover,
.editor-toolbar button:focus-visible,
.editor-toolbar button.active {
  border-color: #d3c0a9;
  background: #f2e4d2;
  color: var(--accent-dark, #365342);
  outline: 0;
}
.editor-toolbar button:disabled {
  color: var(--ink-faint, #aaa79d);
  cursor: not-allowed;
  opacity: 0.65;
}
.editor-toolbar button:disabled:hover {
  border-color: transparent;
  background: transparent;
}
.editor-toolbar button code {
  font-size: 11px;
}
.ai-toolbar-menu-control {
  position: relative;
}
.entity-reference-menu {
  position: fixed;
  z-index: 75;
  display: inline-flex;
  align-items: center;
  gap: 7px;
  max-width: calc(100vw - 24px);
  min-height: 34px;
  padding: 0 10px;
  border: 1px solid #d3c0a9;
  border-radius: 7px;
  background: var(--surface, #fffefa);
  color: var(--ink, #25251f);
  font: 700 12px/1 var(--font-body, system-ui, sans-serif);
  box-shadow: 0 10px 24px rgba(48, 45, 38, 0.16);
  cursor: pointer;
}
.entity-reference-menu:hover,
.entity-reference-menu:focus-visible {
  border-color: #b4773f;
  background: var(--surface-muted, #f4f2ec);
  outline: 0;
}
.entity-reference-menu > span {
  color: var(--accent, #b4773f);
  font-size: 15px;
}
.entity-reference-menu kbd {
  padding: 2px 4px;
  border: 1px solid var(--line, #e4e1d8);
  border-radius: 3px;
  background: var(--canvas, #f7f6f2);
  color: var(--ink-faint, #aaa79d);
  font:
    700 10px/1 ui-monospace,
    monospace;
}
.entity-reference-edit {
  position: fixed;
  z-index: 75;
  min-height: 28px;
  padding: 0 8px;
  border: 1px solid #d3c0a9;
  border-radius: 6px;
  background: var(--surface, #fffefa);
  color: var(--accent-dark, #365342);
  box-shadow: 0 6px 16px rgba(38, 42, 33, 0.14);
  font: 700 11px/1 var(--font-body, system-ui, sans-serif);
  cursor: pointer;
}
.entity-reference-edit:hover,
.entity-reference-edit:focus-visible {
  border-color: #b4773f;
  background: #f2e4d2;
  outline: 0;
}
.editor-toolbar button.ai-toolbar-button {
  color: var(--accent-dark, #365342);
  font-size: 17px;
}
.editor-toolbar button.ai-toolbar-button:not(:disabled) {
  border-color: #d3c0a9;
  background: #f8efe3;
}
.editor-toolbar button.ai-toolbar-button:not(:disabled):hover {
  background: #f2e4d2;
}
.ai-toolbar-menu {
  position: absolute;
  top: calc(100% + 6px);
  right: 0;
  z-index: 20;
  display: grid;
  min-width: 174px;
  padding: 5px;
  border: 1px solid #d8cdbd;
  border-radius: 8px;
  background: var(--surface, #fffefa);
  box-shadow: 0 10px 24px rgba(48, 45, 38, 0.16);
}
.ai-toolbar-menu button {
  justify-content: flex-start;
  width: 100%;
  min-width: 0;
  height: 30px;
  padding: 0 9px;
  border: 0;
  border-radius: 5px;
  color: var(--ink-soft, #77766d);
  font-size: 11px;
  text-align: left;
}
.ai-toolbar-menu button:hover,
.ai-toolbar-menu button:focus-visible {
  border-color: transparent;
  background: var(--surface-muted, #f4f2ec);
  color: var(--ink, #302a27);
  outline: 0;
}
.ai-toolbar-menu button:disabled {
  color: var(--ink-faint, #aaa79d);
  cursor: not-allowed;
  opacity: 0.55;
}
.toolbar-group[aria-label="History"] {
  gap: 4px;
}
.editor-toolbar button.history-button {
  width: 36px;
  min-width: 36px;
  padding: 0;
  font-family: "Apple Symbols", "Segoe UI Symbol", sans-serif;
  font-size: 21px;
  font-weight: 400;
  line-height: 1;
}
.fullscreen-toggle {
  margin-left: auto;
  font-size: 16px !important;
}
.toolbar-divider {
  width: 1px;
  height: 22px;
  margin: 0 6px;
  background: var(--line, #e4e1d8);
}
.style-select {
  height: 32px;
  min-width: 112px;
  padding: 0 8px;
  border: 1px solid transparent;
  border-radius: 6px;
  background: transparent;
  color: var(--ink-soft, #77766d);
  font: 500 12px/1 var(--font-body, system-ui, sans-serif);
  cursor: pointer;
}
.style-select:hover,
.style-select:focus-visible {
  border-color: #d3c0a9;
  background: var(--surface, #fffefa);
  outline: 0;
}
.editor-content {
  position: relative;
  min-width: 0;
  padding: 24px 26px 36px;
  color: var(--ink, #25251f);
  background: var(--canvas, #f7f6f2);
  font: 400 16px/1.7 var(--font-body, ui-sans-serif, system-ui, sans-serif);
  outline: 0;
  cursor: text;
}
.editor-shell:focus-within {
  border-color: #d3c0a9;
}
.editor-content :global(.ProseMirror) {
  min-height: 100%;
  outline: 0;
}
.editor-content :global(.ProseMirror:focus),
.editor-content :global(.ProseMirror:focus-visible) {
  outline: 0;
  box-shadow: none;
}
.editor-content :global(a[data-entity-id]) {
  border-bottom: 1px solid #b4773f;
  color: var(--accent-dark, #365342);
  cursor: pointer;
  text-decoration: none;
}
.editor-content :global(a[data-entity-id]:hover),
.editor-content :global(a[data-entity-id]:focus-visible) {
  border-bottom-color: currentColor;
  border-radius: 2px;
  background: #f2e4d2;
  outline: 0;
}
.editor-content.is-empty::before {
  position: absolute;
  top: 24px;
  right: 26px;
  left: 26px;
  content: attr(data-placeholder);
  color: var(--ink-faint, #aaa79d);
  pointer-events: none;
}
.editor-content:focus-within::before {
  content: none;
}
.editor-content :global(p) {
  margin: 0 0 1em;
}
.editor-content :global(h1),
.editor-content :global(h2),
.editor-content :global(h3) {
  color: var(--ink, #25251f);
  font-family: var(--font-display, Georgia, serif);
  line-height: 1.25;
}
.editor-content :global(h1) {
  margin: 0.2em 0 0.55em;
  font-size: 2.05em;
}
.editor-content :global(h2) {
  margin: 1.1em 0 0.45em;
  font-size: 1.55em;
}
.editor-content :global(h3) {
  margin: 1em 0 0.4em;
  font-size: 1.25em;
}
.editor-content :global(ul),
.editor-content :global(ol) {
  margin: 0 0 1em;
  padding-left: 1.5em;
}
.editor-content :global(li) {
  padding-left: 0.25em;
}
.editor-content :global(blockquote) {
  margin: 1.2em 0;
  padding: 7px 16px;
  border-left: 3px solid var(--accent, #b4773f);
  background: #fcf8f1;
  color: var(--ink-soft, #77766d);
  font-style: italic;
}
.editor-content :global(pre) {
  overflow-x: auto;
  margin: 1.2em 0;
  padding: 13px 15px;
  border: 1px solid var(--line, #e4e1d8);
  border-radius: 7px;
  background: var(--surface-muted, #f4f2ec);
  color: var(--ink, #25251f);
  font:
    13px/1.6 ui-monospace,
    SFMono-Regular,
    Menlo,
    Consolas,
    monospace;
}
.editor-content :global(code) {
  padding: 0.1em 0.3em;
  border-radius: 4px;
  background: #ede9e0;
  color: #765a39;
  font:
    0.86em ui-monospace,
    SFMono-Regular,
    Menlo,
    Consolas,
    monospace;
}
.editor-content :global(pre code) {
  padding: 0;
  background: transparent;
  color: inherit;
  font-size: inherit;
}
.editor-content :global(hr) {
  margin: 2em 0;
  border: 0;
  border-top: 1px solid var(--line, #e4e1d8);
}
.editor-content :global(a) {
  color: var(--accent-dark, #365342);
  text-decoration: underline;
  text-underline-offset: 2px;
}
.editor-statusbar {
  display: flex;
  align-items: center;
  gap: 6px;
  min-height: 34px;
  padding: 7px 13px;
  border-top: 1px solid var(--line, #e4e1d8);
  background: var(--surface-muted, #f4f2ec);
  color: var(--ink-faint, #aaa79d);
  font: 11px/1.3 var(--font-body, system-ui, sans-serif);
}
.status-separator {
  color: var(--ink-faint, #aaa79d);
}
.status-spacer {
  flex: 1;
}
.editor-mode {
  color: var(--ink-faint, #aaa79d);
}
.sr-only {
  position: absolute;
  width: 1px;
  height: 1px;
  padding: 0;
  overflow: hidden;
  clip: rect(0, 0, 0, 0);
  white-space: nowrap;
  border: 0;
}
@media (max-width: 760px) {
  .editor-toolbar {
    padding: 7px;
  }
  .toolbar-divider {
    margin-inline: 3px;
  }
  .editor-mode {
    display: none;
  }
}
@media (max-width: 560px) {
  .editor-shell {
    grid-template-rows: auto minmax(300px, 1fr) auto;
  }
  .editor-toolbar {
    overflow-x: auto;
    flex-wrap: nowrap;
  }
  .fullscreen-toggle {
    position: sticky;
    right: 0;
    flex: 0 0 32px;
    background: var(--surface-muted, #f4f2ec) !important;
    box-shadow: -6px 0 8px var(--surface-muted, #f4f2ec);
  }
  .editor-content {
    min-height: 300px;
    padding: 20px 16px 30px;
    font-size: 15px;
  }
  .editor-content.is-empty::before {
    top: 20px;
    right: 16px;
    left: 16px;
  }
  .style-select {
    min-width: 104px;
  }
}
</style>
