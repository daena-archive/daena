<script lang="ts">
import { Editor, Mark, Extension, getMarkRange } from "@tiptap/core";
import { Plugin, PluginKey } from "@tiptap/pm/state";
import type { EditorState, Transaction } from "@tiptap/pm/state";
import { Decoration, DecorationSet } from "@tiptap/pm/view";
import Code from "@tiptap/extension-code";
import Blockquote from "@tiptap/extension-blockquote";
import Bold from "@tiptap/extension-bold";
import Document from "@tiptap/extension-document";
import Heading from "@tiptap/extension-heading";
import HorizontalRule from "@tiptap/extension-horizontal-rule";
import HardBreak from "@tiptap/extension-hard-break";
import Italic from "@tiptap/extension-italic";
import Link from "@tiptap/extension-link";
import { BulletList, ListItem, OrderedList } from "@tiptap/extension-list";
import Paragraph from "@tiptap/extension-paragraph";
import Strike from "@tiptap/extension-strike";
import { Table, TableRow } from "@tiptap/extension-table";
import TaskItem from "@tiptap/extension-task-item";
import TaskList from "@tiptap/extension-task-list";
import TextAlign from "@tiptap/extension-text-align";
import Text from "@tiptap/extension-text";
import Underline from "@tiptap/extension-underline";
import { UndoRedo } from "@tiptap/extensions";
import TextDirection from "tiptap-text-direction";
import {
  TextAlignStart,
  TextAlignCenter,
  TextAlignEnd,
  ArrowRightToLine,
  ArrowLeftToLine,
  Undo2,
  Redo2,
  Bold as BoldIcon,
  Italic as ItalicIcon,
  Underline as UnderlineIcon,
  Strikethrough as StrikethroughIcon,
  EyeOff,
  AtSign,
  List,
  ListOrdered,
  Quote as QuoteIcon,
  SeparatorHorizontal,
  Eraser,
  Sparkles as SparklesIcon,
  Maximize2,
  Minimize2,
  X as XIcon,
  Link as LinkIcon,
  Search as SearchIcon,
  ChevronUp,
  ChevronDown,
  Replace as ReplaceIcon,
  Image as ImageIcon,
  Paperclip,
  Table as TableIcon,
  BetweenHorizontalEnd,
  BetweenVerticalEnd,
  BetweenHorizontalStart,
  BetweenVerticalStart,
  Trash2,
} from "@lucide/svelte";
import { onMount, tick } from "svelte";
import { htmlToMarkdown, markdownToHtml } from "$lib/markdown";
import type { Asset, Entity } from "$lib/project/client";
import type { AsyncEntityOption, AsyncEntitySearchFn } from "$lib/ui-ux/asyncEntityQuery.ts";
import EntityReferenceDialog from "$lib/editor/EntityReferenceDialog.svelte";
import LinkDialog from "$lib/editor/LinkDialog.svelte";
import InsertAssetDialog from "$lib/editor/InsertAssetDialog.svelte";
import TableInsertDialog from "$lib/editor/TableInsertDialog.svelte";
import { AssetImage } from "$lib/editor/AssetImageExtension";
import { taskListsForEditor, taskListsForMarkdown } from "$lib/editor/markdownRoundTrip";
import { AlignedTableCell, AlignedTableHeader } from "$lib/editor/editorTable";
import { LanguageCodeBlock } from "$lib/editor/editorCodeBlock";
import { denormalizeAssetHtml, resolveAssetSrc } from "$lib/assets/resolve";
import { openUrl } from "@tauri-apps/plugin-opener";
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
      isCustom: {
        default: false,
        parseHTML: (element) => element.getAttribute("data-is-custom") === "true",
        renderHTML: (attributes) => ({ "data-is-custom": attributes.isCustom ? "true" : "false" }),
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

const Spoiler = Mark.create({
  name: "spoiler",
  addOptions() {
    return { HTMLAttributes: {} };
  },
  parseHTML() {
    return [{ tag: "span[data-spoiler]" }, { tag: "span.spoiler" }];
  },
  renderHTML({ HTMLAttributes }) {
    return [
      "span",
      {
        "data-spoiler": "",
        class: "spoiler",
        role: "button",
        tabindex: "0",
        title: "Click to reveal spoiler",
        ...HTMLAttributes,
      },
      0,
    ];
  },
  addCommands() {
    return {
      toggleSpoiler:
        () =>
        ({ commands }: any) =>
          commands.toggleMark("spoiler"),
    } as any;
  },
});

type SearchState = {
  query: string;
  caseSensitive: boolean;
  wholeWord: boolean;
  useRegex: boolean;
  decorations: DecorationSet;
  matches: Array<{ from: number; to: number }>;
  activeIndex: number;
};

const searchPluginKey = new PluginKey<SearchState>("search");

function escapeRegExp(s: string) {
  return s.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}

function buildSearchRegex(query: string, caseSensitive: boolean, wholeWord: boolean, useRegex: boolean): RegExp | null {
  if (!query) return null;
  try {
    let pattern = useRegex ? query : escapeRegExp(query);
    if (wholeWord) pattern = `\\b${pattern}\\b`;
    return new RegExp(pattern, caseSensitive ? "g" : "gi");
  } catch {
    return null;
  }
}

function findMatches(
  doc: any,
  query: string,
  caseSensitive: boolean,
  wholeWord: boolean,
  useRegex: boolean,
): Array<{ from: number; to: number }> {
  const regex = buildSearchRegex(query, caseSensitive, wholeWord, useRegex);
  if (!regex) return [];
  const matches: Array<{ from: number; to: number }> = [];
  doc.descendants((node: any, pos: number) => {
    if (!node.isText || !node.text) return;
    const text = node.text as string;
    let m: RegExpExecArray | null;
    regex.lastIndex = 0;
    while ((m = regex.exec(text))) {
      if (m[0].length === 0) {
        regex.lastIndex++;
        continue;
      }
      const from = pos + m.index;
      const to = from + m[0].length;
      matches.push({ from, to });
      if (m[0].length === 0) break;
    }
  });
  return matches;
}

function createDecorations(doc: any, matches: Array<{ from: number; to: number }>, activeIndex: number): DecorationSet {
  if (matches.length === 0) return DecorationSet.empty;
  const decos = matches.map((m, idx) =>
    Decoration.inline(m.from, m.to, { class: idx === activeIndex ? "search-match-active" : "search-match" }),
  );
  return DecorationSet.create(doc, decos);
}

function createSearchPlugin() {
  return new Plugin<SearchState>({
    key: searchPluginKey,
    state: {
      init(): SearchState {
        return {
          query: "",
          caseSensitive: false,
          wholeWord: false,
          useRegex: false,
          decorations: DecorationSet.empty,
          matches: [],
          activeIndex: -1,
        };
      },
      apply(tr: Transaction, prev: SearchState, _oldState: EditorState, newState: EditorState): SearchState {
        let query = prev.query;
        let caseSensitive = prev.caseSensitive;
        let wholeWord = prev.wholeWord;
        let useRegex = prev.useRegex;
        let activeIndex = prev.activeIndex;
        const meta = tr.getMeta(searchPluginKey) as
          Partial<SearchState & { activeDelta?: number; setActiveIndex?: number }> | undefined;
        let queryChanged = false;
        if (meta) {
          if (typeof meta.query === "string") {
            query = meta.query;
            queryChanged = true;
          }
          if (typeof meta.caseSensitive === "boolean") {
            caseSensitive = meta.caseSensitive;
            queryChanged = true;
          }
          if (typeof meta.wholeWord === "boolean") {
            wholeWord = meta.wholeWord;
            queryChanged = true;
          }
          if (typeof meta.useRegex === "boolean") {
            useRegex = meta.useRegex;
            queryChanged = true;
          }
          if (typeof meta.setActiveIndex === "number") {
            activeIndex = meta.setActiveIndex;
          } else if (typeof meta.activeDelta === "number") {
            if (prev.matches.length > 0) {
              activeIndex = (activeIndex + meta.activeDelta + prev.matches.length) % prev.matches.length;
            }
          }
        }
        const docChanged = tr.docChanged;
        const needRecompute = queryChanged || docChanged;
        let matches = prev.matches;
        let decorations = prev.decorations;
        if (needRecompute) {
          if (!query) {
            matches = [];
            decorations = DecorationSet.empty;
            activeIndex = -1;
          } else {
            matches = findMatches(newState.doc, query, caseSensitive, wholeWord, useRegex);
            if (matches.length === 0) {
              activeIndex = -1;
            } else if (activeIndex < 0 || activeIndex >= matches.length) {
              activeIndex = 0;
            } else if (queryChanged) {
              activeIndex = 0;
            }
            decorations = createDecorations(newState.doc, matches, activeIndex);
          }
        } else if (meta && (typeof meta.setActiveIndex === "number" || typeof meta.activeDelta === "number")) {
          decorations = createDecorations(newState.doc, matches, activeIndex);
        } else if (decorations !== DecorationSet.empty) {
          decorations = decorations.map(tr.mapping, tr.doc);
        }
        return { query, caseSensitive, wholeWord, useRegex, decorations, matches, activeIndex };
      },
    },
    props: {
      decorations(state: EditorState) {
        return searchPluginKey.getState(state)?.decorations ?? DecorationSet.empty;
      },
    },
  });
}

const SearchAndReplace = Extension.create({
  name: "searchAndReplace",
  addProseMirrorPlugins() {
    return [createSearchPlugin()];
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
/** Project-level AI opt-in; hides the Ask-AI toolbar entry when false. */
export let aiEnabled = false;
export let onFullscreenChange: (value: boolean) => void = () => {};
export let onSaveRequest: () => void = () => {};
export let entities: Entity[] = [];
export let searchEntities: AsyncEntitySearchFn | null = null;
export let entityId: string | null = null;
export let defaultNamespace: string | null = null;

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
let entityReferenceSuppressedRange: { from: number; to: number } | null = null;
let entityReferenceDialogOpen = false;
let entityReferenceDialogMode: "insert" | "edit" = "insert";
let entityReferenceEdit: {
  entityId: string;
  label: string;
  isCustom: boolean;
  from: number;
  to: number;
  top: number;
  left: number;
} | null = null;
let aiRequestRange: { from: number; to: number } | null = null;
let linkDialogOpen = false;
let linkDialogInitialText = "";
let linkDialogInitialUrl = "";
let linkDialogHasSelection = false;
let linkDialogRange: { from: number; to: number } | null = null;
let linkPopover: { href: string; text: string; from: number; to: number; top: number; left: number } | null = null;
let linkPopoverEl: HTMLDivElement | null = null;
let insertAssetOpen = false;
let insertAssetRange: { from: number; to: number } | null = null;
let insertAssetInitialAlign: "" | "left" | "center" | "right" = "";
let tableInsertOpen = false;
let imagePopover: {
  from: number;
  to: number;
  src: string;
  alt: string;
  title: string;
  width: string;
  height: string;
  top: number;
  left: number;
} | null = null;
let imagePopoverEl: HTMLDivElement | null = null;
let imageDraftAlt = "";
let imageDraftTitle = "";
let imageDraftWidth = "";
let imageDraftHeight = "";
let imageTitleCustom = false;
let imagePreserveAspect = true;
let imageNaturalWidth = 0;
let imageNaturalHeight = 0;
let imageNaturalCache = new Map<string, { w: number; h: number }>();
let imageAltInputEl: HTMLInputElement | null = null;
let imageReplaceMode = false;
let isFullscreen = false;
let searchOpen = false;
let searchReplaceOpen = false;
let searchQuery = "";
let replaceQuery = "";
let searchCaseSensitive = false;
let searchWholeWord = false;
let searchUseRegex = false;
let searchMatchCount = 0;
let searchActiveIndex = -1;
let searchInputEl: HTMLInputElement | null = null;
let replaceInputEl: HTMLInputElement | null = null;
let pendingChangeTimer: number | null = null;
$: wordCountValue = editorText.trim() ? editorText.trim().split(/\s+/).length : 0;
$: characterCountValue = editorText.length;
$: if (fullscreen !== isFullscreen) isFullscreen = fullscreen;
$: if (typeof window !== "undefined") {
  try {
    localStorage.setItem("daena:imagePreserveAspect", String(imagePreserveAspect));
  } catch {}
}

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

function editorHtmlFromMarkdown(markdown: string) {
  return hydrateEntityReferences(taskListsForEditor(sanitizeHtml(markdownToHtml(markdown))));
}

function markdownFromEditorHtml(html: string) {
  return htmlToMarkdown(taskListsForMarkdown(denormalizeAssetHtml(html)));
}

function editorPlainText(currentEditor: Editor) {
  return currentEditor.state.doc.textBetween(0, currentEditor.state.doc.content.size, "\n");
}

function emitChange() {
  if (!editor) return;
  editorText = editorPlainText(editor);
  currentMarkdown = markdownFromEditorHtml(editor.getHTML());
  onChange(currentMarkdown);
}

function cancelPendingChange() {
  if (pendingChangeTimer === null) return;
  window.clearTimeout(pendingChangeTimer);
  pendingChangeTimer = null;
}

function scheduleChange() {
  cancelPendingChange();
  pendingChangeTimer = window.setTimeout(() => {
    pendingChangeTimer = null;
    emitChange();
  }, 120);
}

export function flushPendingChanges() {
  if (pendingChangeTimer === null) return;
  cancelPendingChange();
  emitChange();
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
  const activeElement = document.activeElement as HTMLElement | null;
  if (!activeElement?.closest(".editor-shell")) return;
  const blockingDialogOpen = entityReferenceDialogOpen || linkDialogOpen || insertAssetOpen || tableInsertOpen;
  if (blockingDialogOpen && event.key !== "Escape") return;
  const isMod = event.metaKey || event.ctrlKey;
  if (isMod && event.key.toLowerCase() === "s") {
    event.preventDefault();
    flushPendingChanges();
    onSaveRequest();
    return;
  }
  if (isMod && event.key.toLowerCase() === "f") {
    event.preventDefault();
    openSearch(false);
    return;
  }
  if (
    (event.ctrlKey && !event.metaKey && event.key.toLowerCase() === "h") ||
    (event.metaKey && event.altKey && event.key.toLowerCase() === "f")
  ) {
    event.preventDefault();
    openSearch(true);
    return;
  }
  if (isMod && event.key.toLowerCase() === "g") {
    event.preventDefault();
    if (searchOpen) goSearch(event.shiftKey ? -1 : 1);
    return;
  }
  if (event.key === "Escape" && entityReferenceDialogOpen) {
    event.preventDefault();
    cancelEntityReference();
    return;
  }
  if (event.key === "Escape" && entityReferenceMenuOpen) {
    event.preventDefault();
    cancelEntityReference();
    return;
  }
  if (event.key === "Escape" && imagePopover) {
    event.preventDefault();
    hideImagePopover();
    return;
  }
  if (event.key === "Escape" && linkPopover) {
    event.preventDefault();
    hideLinkPopover();
    return;
  }
  if (event.key === "Escape" && linkDialogOpen) {
    event.preventDefault();
    cancelLink();
    return;
  }
  if (event.key === "Escape" && searchOpen) {
    event.preventDefault();
    closeSearch();
    return;
  }
  if (event.key === "Escape" && insertAssetOpen) {
    event.preventDefault();
    cancelInsertAsset();
    return;
  }
  if (event.key === "Escape" && tableInsertOpen) {
    event.preventDefault();
    cancelInsertTable();
    return;
  }
  if (event.key === "Escape" && isFullscreen) {
    event.preventDefault();
    setFullscreen(false);
  }
}

function portal(node: HTMLElement) {
  if (typeof document !== "undefined" && document.body) {
    document.body.appendChild(node);
  }
  return {
    destroy() {
      if (node.parentNode) node.remove();
    },
  };
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

/**
 * Replaces the selection captured at request time with rewritten Markdown.
 * Uses the captured ProseMirror range so the exact occurrence is rewritten;
 * returns null if the document no longer contains the original selection.
 */
export function replaceAiTextWithMarkdown(value: string): string | null {
  if (!editor || !aiRequestRange || !selectionText) return null;
  const { from, to } = aiRequestRange;
  if (editor.state.doc.textBetween(from, to, "\n") !== selectionText) return null;
  let html = sanitizeHtml(markdownToHtml(value));
  if (html.startsWith("<p>") && html.endsWith("</p>")) html = html.slice(3, -4);
  const ok = editor.chain().focus().insertContentAt({ from, to }, html).run();
  if (!ok) return null;
  aiRequestRange = null;
  currentMarkdown = markdownFromEditorHtml(editor.getHTML());
  editorText = editorPlainText(editor);
  return currentMarkdown;
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
  if (!editorState || !editor) return;
  const { from, to, empty } = editor.state.selection;
  const previousUrl = editorState.getAttributes("link").href ?? "";

  if (!empty) {
    const selectedText = editor.state.doc.textBetween(from, to, " ");
    if (selectedText.trim()) {
      linkDialogInitialText = selectedText;
      linkDialogInitialUrl = previousUrl;
      linkDialogHasSelection = true;
      linkDialogRange = { from, to };
      linkDialogOpen = true;
      return;
    }
  }

  const $pos = editor.state.doc.resolve(from);
  const linkType = editor.state.schema.marks.link;
  const linkRange = linkType ? getMarkRange($pos, linkType) : null;
  if (linkRange) {
    const linkText = editor.state.doc.textBetween(linkRange.from, linkRange.to, " ");
    const href = editorState.getAttributes("link").href ?? "";
    linkDialogInitialText = linkText;
    linkDialogInitialUrl = href;
    linkDialogHasSelection = false;
    linkDialogRange = { from: linkRange.from, to: linkRange.to };
    linkDialogOpen = true;
    return;
  }

  linkDialogInitialText = "";
  linkDialogInitialUrl = "";
  linkDialogHasSelection = false;
  linkDialogRange = { from, to };
  linkDialogOpen = true;
}

function confirmLink(displayText: string, url: string) {
  if (!editorState || !editor || !linkDialogRange) {
    linkDialogOpen = false;
    return;
  }
  const href = url.trim();
  if (!href) {
    if (linkDialogInitialUrl) editorState.chain().focus().extendMarkRange("link").unsetLink().run();
    linkDialogOpen = false;
    return;
  }
  const range = linkDialogRange;
  const wasSelection = linkDialogHasSelection;
  // close before transaction to avoid focus issues; keep range copy
  linkDialogOpen = false;
  if (wasSelection) {
    // Apply link to the original selection; preserve inline marks via extendMarkRange
    editorState
      .chain()
      .focus()
      .setTextSelection(range)
      .extendMarkRange("link")
      .setLink({ href, target: "_blank", rel: "noopener noreferrer" })
      .run();
  } else {
    const text = displayText.trim();
    if (!text) return;
    const isEditingExisting = !!linkDialogInitialUrl && range.from !== range.to;
    if (isEditingExisting) {
      editorState
        .chain()
        .focus()
        .insertContentAt(range, [
          {
            type: "text",
            text,
            marks: [{ type: "link", attrs: { href, target: "_blank", rel: "noopener noreferrer" } }],
          },
        ])
        .run();
    } else {
      editorState
        .chain()
        .focus()
        .insertContentAt(range.from, [
          {
            type: "text",
            text,
            marks: [{ type: "link", attrs: { href, target: "_blank", rel: "noopener noreferrer" } }],
          },
        ])
        .run();
    }
  }
}

function cancelLink() {
  linkDialogOpen = false;
  editor?.commands.focus();
}

function removeLink() {
  if (!editorState) {
    linkDialogOpen = false;
    return;
  }
  if (linkDialogRange && linkDialogRange.from !== linkDialogRange.to) {
    editorState.chain().focus().setTextSelection(linkDialogRange).extendMarkRange("link").unsetLink().run();
  } else {
    editorState.chain().focus().extendMarkRange("link").unsetLink().run();
  }
  linkDialogOpen = false;
}

function showLinkPopover(href: string, text: string, from: number, to: number, bounds: DOMRect) {
  linkPopover = { href, text, from, to, top: bounds.bottom + 6, left: bounds.left };
  tick().then(() => {
    if (!linkPopover || !linkPopoverEl) return;
    const rect = linkPopoverEl.getBoundingClientRect();
    let left = linkPopover.left;
    let top = linkPopover.top;
    left = Math.max(8, Math.min(left, window.innerWidth - rect.width - 8));
    if (top + rect.height > window.innerHeight - 8) top = bounds.top - rect.height - 6;
    top = Math.max(8, Math.min(top, window.innerHeight - rect.height - 8));
    if (top !== linkPopover.top || left !== linkPopover.left) linkPopover = { ...linkPopover, top, left };
  });
}

function hideLinkPopover() {
  linkPopover = null;
}

async function openLinkExternal() {
  if (!linkPopover) return;
  const rawHref = linkPopover.href.trim();
  if (!rawHref) return;
  // Internal asset paths should resolve to a blob URL, not an external https:// URL
  if (rawHref.startsWith("assets/")) {
    hideLinkPopover();
    try {
      const blobUrl = await resolveAssetSrc(rawHref);
      if (blobUrl) {
        window.open(blobUrl, "_blank", "noopener,noreferrer");
        return;
      }
    } catch {}
    console.warn("Failed to open asset link", rawHref);
    return;
  }
  const href =
    /^https?:\/\//i.test(rawHref) || /^mailto:/i.test(rawHref) || /^ftp:/i.test(rawHref)
      ? rawHref
      : `https://${rawHref}`;
  hideLinkPopover();
  const isTauri = typeof window !== "undefined" && ((window as any).__TAURI__ || (window as any).__TAURI_INTERNALS__);
  if (isTauri) {
    try {
      await openUrl(href);
      return;
    } catch (e) {
      console.warn("openUrl failed, fallback to window.open", e);
    }
    try {
      window.open(href, "_blank", "noopener,noreferrer");
      return;
    } catch (e) {
      console.warn("window.open fallback failed", e);
    }
  } else {
    try {
      const w = window.open(href, "_blank", "noopener,noreferrer");
      if (w) return;
    } catch (e) {
      console.warn("window.open failed", e);
    }
    try {
      await openUrl(href);
      return;
    } catch (e) {
      console.warn("openUrl fallback failed", e);
    }
    try {
      const a = document.createElement("a");
      a.href = href;
      a.target = "_blank";
      a.rel = "noopener noreferrer";
      document.body.appendChild(a);
      a.click();
      a.remove();
    } catch (e) {
      console.warn("anchor fallback failed", e);
    }
  }
}

function editLinkFromPopover() {
  if (!linkPopover) return;
  const { href, text, from, to } = linkPopover;
  hideLinkPopover();
  linkDialogInitialText = text;
  linkDialogInitialUrl = href;
  linkDialogHasSelection = false;
  linkDialogRange = { from, to };
  linkDialogOpen = true;
}

function unlinkFromPopover() {
  if (!linkPopover || !editorState) return;
  const range = { from: linkPopover.from, to: linkPopover.to };
  hideLinkPopover();
  editorState.chain().focus().setTextSelection(range).extendMarkRange("link").unsetLink().run();
}

function syncLinkPopover(nextEditor: Editor) {
  if (!linkPopover || linkDialogOpen) return;
  const { from, to } = nextEditor.state.selection;
  // hide if selection moves outside the popover link range
  if (from < linkPopover.from || from > linkPopover.to) {
    // also check if still on same link href
    const $pos = nextEditor.state.doc.resolve(from);
    const linkType = nextEditor.state.schema.marks.link;
    const range = linkType ? getMarkRange($pos, linkType) : null;
    const href = nextEditor.getAttributes("link").href ?? "";
    if (!range || range.from !== linkPopover.from || range.to !== linkPopover.to || href !== linkPopover.href) {
      hideLinkPopover();
    }
  }
}

function getImageNodeAtPos(pos: number): { node: any; pos: number } | null {
  if (!editorState) return null;
  try {
    let found: { node: any; pos: number } | null = null;
    editorState.state.doc.descendants((n: any, p: number) => {
      if (n.type.name === "image" && p <= pos && pos < p + n.nodeSize) {
        found = { node: n, pos: p };
        return false;
      }
      return true;
    });
    if (found) return found;
    const sel: any = editorState.state.selection;
    if (sel.node && sel.node.type.name === "image") return { node: sel.node, pos: sel.from };
    const $pos = editorState.state.doc.resolve(pos);
    const maybe = $pos.parent.maybeChild($pos.index());
    if (maybe && maybe.type.name === "image") {
      return { node: maybe, pos: $pos.pos - $pos.parentOffset };
    }
  } catch {}
  return null;
}

function probeNatural(src: string) {
  const cached = imageNaturalCache.get(src);
  if (cached) {
    imageNaturalWidth = cached.w;
    imageNaturalHeight = cached.h;
    return;
  }
  const tryProbe = (url: string) => {
    const img = new window.Image();
    img.onload = () => {
      const w = (img as HTMLImageElement).naturalWidth;
      const h = (img as HTMLImageElement).naturalHeight;
      if (w && h) {
        imageNaturalCache.set(src, { w, h });
        if (imagePopover && imagePopover.src === src) {
          imageNaturalWidth = w;
          imageNaturalHeight = h;
        }
      }
    };
    img.onerror = () => {};
    img.src = url;
  };
  if (src.startsWith("assets/")) {
    void resolveAssetSrc(src).then((blob) => {
      if (blob) tryProbe(blob);
      // else: asset missing or not yet available — do not fall back to
      // `assets/...` fetch which would emit `GET /assets/... 404` in the
      // webview/dev console. Natural size stays 0 until blob is available.
    });
  } else {
    tryProbe(src);
  }
}

function showImagePopover(pos: number, bounds: DOMRect) {
  if (!editorState) return;
  const info = getImageNodeAtPos(pos);
  if (!info) return;
  const node = info.node;
  const from = info.pos;
  const to = from + node.nodeSize;
  const src = String(node.attrs.src ?? "");
  const alt = String(node.attrs.alt ?? "");
  const title = String(node.attrs.title ?? "");
  const widthRaw = node.attrs.width;
  const heightRaw = node.attrs.height;
  const width = widthRaw != null && String(widthRaw).trim() !== "" ? String(widthRaw).trim() : "";
  const height = heightRaw != null && String(heightRaw).trim() !== "" ? String(heightRaw).trim() : "";
  hideLinkPopover();
  imageDraftAlt = alt;
  imageDraftTitle = title;
  imageDraftWidth = /^\d+$/.test(width) ? width : "";
  imageDraftHeight = /^\d+$/.test(height) ? height : "";
  imageTitleCustom = !!(title && title !== alt);
  imageNaturalWidth = 0;
  imageNaturalHeight = 0;
  if (src) probeNatural(src);
  imagePopover = {
    from,
    to,
    src,
    alt,
    title,
    width: imageDraftWidth,
    height: imageDraftHeight,
    top: bounds.bottom + 6,
    left: bounds.left,
  };
  tick().then(() => {
    if (!imagePopover || !imagePopoverEl) return;
    const rect = imagePopoverEl.getBoundingClientRect();
    let left = imagePopover.left;
    let top = imagePopover.top;
    left = Math.max(8, Math.min(left, window.innerWidth - rect.width - 8));
    if (top + rect.height > window.innerHeight - 8) top = bounds.top - rect.height - 6;
    top = Math.max(8, Math.min(top, window.innerHeight - rect.height - 8));
    if (top !== imagePopover.top || left !== imagePopover.left) imagePopover = { ...imagePopover, top, left };
  });
}

function hideImagePopover() {
  imagePopover = null;
}

function syncImagePopover(nextEditor: Editor, transaction?: Transaction) {
  if (!imagePopover) return;
  if (!nextEditor.isActive("image")) {
    const sel: any = nextEditor.state.selection;
    const isImageSel = sel.node && sel.node.type.name === "image";
    if (!isImageSel) {
      hideImagePopover();
      return;
    }
  }
  try {
    const mapping = transaction ? transaction.mapping : nextEditor.state.tr.mapping;
    const mappedFrom = mapping.map(imagePopover.from, -1);
    const mappedTo = mapping.map(imagePopover.to, -1);
    if (mappedFrom === mappedTo) {
      hideImagePopover();
      return;
    }
    const node = nextEditor.state.doc.nodeAt(mappedFrom);
    if (!node || node.type.name !== "image") {
      // fallback: search by src
      const currentSrc = imagePopover?.src ?? "";
      let found: any = null;
      let foundPos = -1;
      nextEditor.state.doc.descendants((n: any, p: number) => {
        if (n.type.name === "image" && String(n.attrs.src) === currentSrc) {
          found = n;
          foundPos = p;
          return false;
        }
        return true;
      });
      if (found && imagePopover) {
        imagePopover = { ...imagePopover, from: foundPos, to: foundPos + found.nodeSize };
        return;
      }
      hideImagePopover();
      return;
    }
    const src = String(node.attrs.src ?? "");
    if (src !== imagePopover?.src) {
      hideImagePopover();
      return;
    }
    // sync width/height/alt/title from node if changed externally (e.g., undo)
    const w = node.attrs.width != null ? String(node.attrs.width) : "";
    const h = node.attrs.height != null ? String(node.attrs.height) : "";
    const alt = String(node.attrs.alt ?? "");
    const title = String(node.attrs.title ?? "");
    imagePopover = { ...imagePopover, from: mappedFrom, to: mappedTo, width: w, height: h, alt, title };
    imageDraftAlt = alt;
    imageDraftTitle = title;
    imageDraftWidth = /^\d+$/.test(w) ? w : "";
    imageDraftHeight = /^\d+$/.test(h) ? h : "";
    imageTitleCustom = !!(title && title !== alt);
  } catch {
    hideImagePopover();
  }
}

function handleImageClick(target: EventTarget | null, posHint?: number): boolean {
  if (!editorState || !editorElement) return false;
  const el = target as HTMLElement | null;
  const img = el?.closest?.("img") as HTMLImageElement | null;
  if (!img || !editorElement.contains(img)) return false;
  // only for images that are part of editor (check ProseMirror-selectednode or daena-content-image)
  if (!img.classList.contains("ProseMirror-selectednode") && !img.classList.contains("daena-content-image")) {
    // still allow if inside editor
    if (!img.closest(".daena-asset-image-wrapper")) return false;
  }
  let pos: number | null = null;
  // Prefer the ProseMirror position hint from handleClickOn, which is authoritative
  if (typeof posHint === "number" && Number.isFinite(posHint)) {
    const hintInfo = getImageNodeAtPos(posHint);
    if (hintInfo) {
      pos = hintInfo.pos;
    } else {
      // posHint might be inside paragraph text offset near image; try nearby offsets
      for (const delta of [0, 1, -1, 2, -2]) {
        const probe = posHint + delta;
        if (probe >= 0) {
          const probeInfo = getImageNodeAtPos(probe);
          if (probeInfo) {
            pos = probeInfo.pos;
            break;
          }
        }
      }
      // Fallback: resolve parent index
      if (pos == null) {
        try {
          const $pos = editorState.state.doc.resolve(posHint);
          const maybe = $pos.parent.maybeChild($pos.index());
          if (maybe && maybe.type.name === "image") {
            pos = $pos.pos - $pos.parentOffset;
          } else if ($pos.parent.maybeChild($pos.index() - 1)?.type.name === "image") {
            const idx = $pos.index() - 1;
            const before = $pos.parent.maybeChild(idx);
            if (before) {
              // find its start pos by scanning
              let p = $pos.pos - $pos.parentOffset;
              // walk children before idx to sum sizes
              for (let i = 0; i < idx; i++) p += $pos.parent.child(i).nodeSize;
              pos = p;
            }
          }
        } catch {}
      }
    }
  }
  if (pos == null) {
    try {
      const wrapper = img.closest(".daena-asset-image-wrapper") as HTMLElement | null;
      const dom = wrapper ?? img;
      const rawPos = editorState.view.posAtDOM(dom, 0);
      const test = getImageNodeAtPos(rawPos);
      if (test) {
        pos = test.pos;
      } else {
        // Try offsets around rawPos as NodeView wrapper offset can be off by 1
        for (const delta of [1, -1, 2, -2]) {
          const probe = rawPos + delta;
          const probeInfo = getImageNodeAtPos(probe);
          if (probeInfo) {
            pos = probeInfo.pos;
            break;
          }
        }
        if (pos == null) pos = rawPos;
      }
    } catch {}
  }
  if (pos == null || !Number.isFinite(pos)) return false;
  const bounds = img.getBoundingClientRect();
  showImagePopover(pos, bounds);
  // Ensure NodeSelection is set so isActive('image') and styling work; ProseMirror won't auto-select when handleClick returns true
  try {
    const node = editorState.state.doc.nodeAt(pos);
    if (node && node.type.name === "image") {
      const sel: any = editorState.state.selection;
      const needsSelect = !sel.node || sel.from !== pos || sel.node.type?.name !== "image";
      if (needsSelect) {
        // Defer selection to avoid nesting dispatch inside handleClick stack
        tick().then(() => {
          try {
            if (editorState && editorState.state.doc.nodeAt(pos)?.type.name === "image") {
              editorState.chain().focus().setNodeSelection(pos).run();
            }
          } catch {}
        });
      }
    }
  } catch {}
  return true;
}

function clampDim(value: string): string {
  const trimmed = value.trim();
  if (trimmed === "") return "";
  const n = Number(trimmed);
  if (!Number.isFinite(n)) return "";
  const clamped = Math.max(16, Math.min(2000, Math.round(n)));
  return String(clamped);
}

function commitImageAttributes(partial: Record<string, unknown>) {
  if (!editorState || !imagePopover) return;
  const from = imagePopover.from;
  try {
    const node = editorState.state.doc.nodeAt(from);
    if (!node || node.type.name !== "image") return;
    editorState.chain().focus().setNodeSelection(from).updateAttributes("image", partial).run();
    // update popover state to reflect new attrs
    const nextNode = editorState.state.doc.nodeAt(from);
    if (nextNode) {
      const w = nextNode.attrs.width != null ? String(nextNode.attrs.width) : "";
      const h = nextNode.attrs.height != null ? String(nextNode.attrs.height) : "";
      imagePopover = {
        ...imagePopover,
        width: w,
        height: h,
        alt: String(nextNode.attrs.alt ?? ""),
        title: String(nextNode.attrs.title ?? ""),
      };
    }
  } catch {}
}

function updateImageAlt(value: string) {
  imageDraftAlt = value;
  const alt = value;
  const title = imageTitleCustom ? imageDraftTitle : alt;
  if (!imageTitleCustom) imageDraftTitle = alt;
  commitImageAttributes({ alt, title: imageTitleCustom ? imageDraftTitle : alt });
}

function updateImageTitle(value: string) {
  imageDraftTitle = value;
  imageTitleCustom = value !== imageDraftAlt;
  commitImageAttributes({ title: value });
}

function updateImageWidth(value: string) {
  const clamped = clampDim(value);
  // if empty, user wants auto
  if (clamped === "" && value.trim() !== "") {
    // invalid, ignore
    imageDraftWidth = value;
    return;
  }
  imageDraftWidth = clamped === "" ? "" : clamped;
  if (imageDraftWidth === "") {
    // auto: clear width (and maybe height if both auto? keep height as is? plan says Auto clears both)
    // For single field edit, only clear that field; Auto button clears both
    commitImageAttributes({ width: null });
    return;
  }
  let newWidth = imageDraftWidth;
  let newHeight = imageDraftHeight;
  if (imagePreserveAspect && imageNaturalWidth && imageNaturalHeight) {
    const wNum = Number(newWidth);
    if (Number.isFinite(wNum) && wNum > 0) {
      const hNum = Math.round((wNum * imageNaturalHeight) / imageNaturalWidth);
      newHeight = String(Math.max(16, Math.min(2000, hNum)));
      imageDraftHeight = newHeight;
      commitImageAttributes({ width: Number(newWidth), height: Number(newHeight) });
      return;
    }
  }
  commitImageAttributes({ width: Number(newWidth) });
}

function updateImageHeight(value: string) {
  const clamped = clampDim(value);
  if (clamped === "" && value.trim() !== "") {
    imageDraftHeight = value;
    return;
  }
  imageDraftHeight = clamped === "" ? "" : clamped;
  if (imageDraftHeight === "") {
    commitImageAttributes({ height: null });
    return;
  }
  let newHeight = imageDraftHeight;
  let newWidth = imageDraftWidth;
  if (imagePreserveAspect && imageNaturalWidth && imageNaturalHeight) {
    const hNum = Number(newHeight);
    if (Number.isFinite(hNum) && hNum > 0) {
      const wNum = Math.round((hNum * imageNaturalWidth) / imageNaturalHeight);
      newWidth = String(Math.max(16, Math.min(2000, wNum)));
      imageDraftWidth = newWidth;
      commitImageAttributes({ width: Number(newWidth), height: Number(newHeight) });
      return;
    }
  }
  commitImageAttributes({ height: Number(newHeight) });
}

function clearImageDimensions() {
  imageDraftWidth = "";
  imageDraftHeight = "";
  commitImageAttributes({ width: null, height: null });
}

function applyImagePreset(preset: string) {
  if (!editorState || !imagePopover) return;
  if (preset === "S") {
    imageDraftWidth = "320";
    if (imagePreserveAspect && imageNaturalWidth && imageNaturalHeight) {
      const h = Math.round((320 * imageNaturalHeight) / imageNaturalWidth);
      imageDraftHeight = String(h);
      commitImageAttributes({ width: 320, height: h });
    } else {
      commitImageAttributes({ width: 320 });
    }
    return;
  }
  if (preset === "M") {
    imageDraftWidth = "640";
    if (imagePreserveAspect && imageNaturalWidth && imageNaturalHeight) {
      const h = Math.round((640 * imageNaturalHeight) / imageNaturalWidth);
      imageDraftHeight = String(h);
      commitImageAttributes({ width: 640, height: h });
    } else {
      commitImageAttributes({ width: 640 });
    }
    return;
  }
  if (preset === "L") {
    imageDraftWidth = "960";
    if (imagePreserveAspect && imageNaturalWidth && imageNaturalHeight) {
      const h = Math.round((960 * imageNaturalHeight) / imageNaturalWidth);
      imageDraftHeight = String(h);
      commitImageAttributes({ width: 960, height: h });
    } else {
      commitImageAttributes({ width: 960 });
    }
    return;
  }
  if (preset === "Original") {
    if (imageNaturalWidth && imageNaturalHeight) {
      imageDraftWidth = String(imageNaturalWidth);
      imageDraftHeight = String(imageNaturalHeight);
      commitImageAttributes({ width: imageNaturalWidth, height: imageNaturalHeight });
    }
    return;
  }
  if (preset === "Full") {
    clearImageDimensions();
    return;
  }
}

function alignImage(dir: string) {
  if (!editorState) return;
  editorState.chain().focus().setTextAlign(dir).run();
}

function removeImage() {
  if (!editorState || !imagePopover) return;
  const from = imagePopover.from;
  hideImagePopover();
  editorState.chain().focus().setNodeSelection(from).deleteSelection().run();
}

function replaceImage() {
  if (!editorState || !imagePopover) return;
  imageReplaceMode = true;
  const from = imagePopover.from;
  insertAssetRange = { from, to: from + 1 };
  // use from/to of image node (nodeSize 1)
  const node = editorState.state.doc.nodeAt(from);
  if (node) insertAssetRange = { from, to: from + node.nodeSize };
  insertAssetInitialAlign = getCurrentAlign();
  insertAssetOpen = true;
}

function isImageActive(): boolean {
  return !!editorState?.isActive("image");
}

function hydrateEntityReferences(html: string): string {
  if (typeof document === "undefined") return html;
  if (!entities || entities.length === 0) return html;
  const template = document.createElement("template");
  template.innerHTML = html;
  const map = new Map(entities.filter((e) => !e.deleted).map((e) => [e.id, e.name]));
  for (const el of template.content.querySelectorAll("a[data-entity-id]")) {
    const a = el as HTMLElement;
    const id = a.getAttribute("data-entity-id") ?? "";
    const name = map.get(id);
    if (!name) continue;
    const isCustomAttr = a.getAttribute("data-is-custom");
    const text = (a.textContent ?? "").trim();
    const isCustom = isCustomAttr != null ? isCustomAttr === "true" : !!text;
    if (!isCustom) {
      if (text !== name) a.textContent = name;
      a.setAttribute("data-is-custom", "false");
    } else {
      a.setAttribute("data-is-custom", "true");
      if (!text) a.textContent = name;
    }
  }
  return template.innerHTML;
}

function updateAutoEntityReferences() {
  if (!editor || !editorState || !entities) return;
  const map = new Map(entities.filter((e) => !e.deleted).map((e) => [e.id, e.name]));
  const doc = editor.state.doc;
  const replacements: Array<{ from: number; to: number; text: string; mark: any }> = [];
  doc.descendants((node, pos) => {
    if (!node.isText || !node.text) return;
    const erMark = node.marks.find((m) => m.type.name === "entityReference");
    if (!erMark) return;
    if (!!erMark.attrs.isCustom) return;
    const id = erMark.attrs.entityId;
    const expected = map.get(id);
    if (!expected) return;
    if (node.text !== expected)
      replacements.push({ from: pos, to: pos + node.text.length, text: expected, mark: erMark });
  });
  if (replacements.length === 0) return;
  // apply from end to start to keep positions valid
  replacements.sort((a, b) => b.from - a.from);
  let tr = editor.state.tr;
  for (const r of replacements) {
    const markType = editor.state.schema.marks.entityReference;
    const mark = markType.create({ entityId: r.mark.attrs.entityId, isCustom: false });
    tr = tr.replaceWith(r.from, r.to, editor.state.schema.text(r.text, [mark]));
  }
  if (tr.docChanged) editor.view.dispatch(tr);
}

function getExternalLinkAnchor(target: EventTarget | null): HTMLAnchorElement | null {
  if (!target) return null;
  const el = target as HTMLElement;
  if ((el as any)?.closest) {
    const found = (el as HTMLElement).closest<HTMLAnchorElement>("a[href]:not([data-entity-id])");
    if (found) return found;
  }
  const parent = (target as any)?.parentElement as HTMLElement | null;
  if (parent?.closest) return parent.closest<HTMLAnchorElement>("a[href]:not([data-entity-id])");
  return null;
}

function getSpoilerEl(target: EventTarget | null): HTMLElement | null {
  if (!target) return null;
  const el = target as HTMLElement;
  if ((el as any)?.closest) {
    const found = (el as HTMLElement).closest<HTMLElement>("span[data-spoiler]");
    if (found) return found;
  }
  const parent = (target as any)?.parentElement as HTMLElement | null;
  return parent?.closest?.("span[data-spoiler]") ?? null;
}

function insertEntityReference(
  entity: Pick<Entity, "id" | "name"> | AsyncEntityOption,
  label: string,
  isCustom: boolean,
) {
  if (!editorState || !editable) return;
  const range = entityReferenceRange;
  if (!range) return;
  const displayText = isCustom ? label : entity.name;
  editorState
    .chain()
    .focus()
    .insertContentAt(range, {
      type: "text",
      text: displayText,
      marks: [{ type: "entityReference", attrs: { entityId: entity.id, isCustom } }],
    })
    .run();
  entityReferenceMenuOpen = false;
  entityReferenceDialogOpen = false;
  entityReferenceQuery = "";
  entityReferenceRange = null;
  entityReferenceSuppressedRange = null;
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
  if (!range || !editorState) return;
  let isCustom = false;
  try {
    const $pos = editorState.state.doc.resolve(range.from + 1);
    const mark = $pos.marks().find((m) => m.type.name === "entityReference" && m.attrs.entityId === entityId);
    if (mark) isCustom = !!mark.attrs.isCustom;
  } catch {}
  entityReferenceEdit = { entityId, label, isCustom, ...range, top: bounds.top - 36, left: bounds.left };
}

function openEntityReferenceEditor() {
  if (!entityReferenceEdit) return;
  entityReferenceDialogMode = "edit";
  entityReferenceDialogOpen = true;
}

function saveEntityReference(
  entity: Pick<Entity, "id" | "name"> | AsyncEntityOption,
  label: string,
  isCustom: boolean,
) {
  if (entityReferenceDialogMode === "edit") {
    const reference = entityReferenceEdit;
    if (!editorState || !reference) return;
    const displayText = isCustom ? label : entity.name;
    editorState
      .chain()
      .focus()
      .insertContentAt(
        { from: reference.from, to: reference.to },
        {
          type: "text",
          text: displayText,
          marks: [{ type: "entityReference", attrs: { entityId: entity.id, isCustom } }],
        },
      )
      .run();
    entityReferenceEdit = null;
    entityReferenceDialogOpen = false;
    return;
  }
  insertEntityReference(entity, label, isCustom);
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
  if (entityReferenceRange) entityReferenceSuppressedRange = { ...entityReferenceRange };
  entityReferenceMenuOpen = false;
  entityReferenceDialogOpen = false;
  entityReferenceQuery = "";
  entityReferenceRange = null;
}

function updateEntityReferenceTrigger(nextEditor: Editor) {
  const { from, to } = nextEditor.state.selection;
  if (!editable || from !== to) {
    if (entityReferenceMenuOpen && entityReferenceRange) entityReferenceSuppressedRange = { ...entityReferenceRange };
    entityReferenceMenuOpen = false;
    entityReferenceRange = null;
    return;
  }
  const beforeCursor = nextEditor.state.doc.textBetween(Math.max(0, from - 80), from, "\n");
  const trigger = beforeCursor.match(/@([^\s@]*)$/);
  const triggerStart = trigger ? beforeCursor.length - trigger[0].length : -1;
  const preceding = triggerStart > 0 ? beforeCursor[triggerStart - 1] : "";
  if (!trigger || (preceding && !/[\s([{]/.test(preceding))) {
    if (entityReferenceMenuOpen && entityReferenceRange) entityReferenceSuppressedRange = { ...entityReferenceRange };
    entityReferenceMenuOpen = false;
    entityReferenceRange = null;
    return;
  }
  const newRange = { from: from - trigger[0].length, to: from };
  if (entityReferenceSuppressedRange && newRange.from === entityReferenceSuppressedRange.from) {
    entityReferenceMenuOpen = false;
    entityReferenceRange = null;
    return;
  }
  entityReferenceSuppressedRange = null;
  const coords = nextEditor.view.coordsAtPos(from);
  entityReferenceMenuPosition = {
    top: coords.bottom + 6,
    left: coords.left,
  };
  entityReferenceQuery = trigger[1];
  entityReferenceRange = newRange;
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

function getCurrentAlign(): "" | "left" | "center" | "right" {
  if (isAligned("center")) return "center";
  if (isAligned("right")) return "right";
  if (isAligned("left")) return "left";
  return "";
}

function isDirection(direction: string): boolean {
  if (!editorState) return false;
  return (
    editorState.getAttributes("paragraph").dir === direction || editorState.getAttributes("heading").dir === direction
  );
}

function syncSearchState(nextEditor: Editor) {
  const state = searchPluginKey.getState(nextEditor.state);
  if (!state) return;
  searchMatchCount = state.matches.length;
  searchActiveIndex = state.activeIndex;
  if (state.matches.length > 0 && state.activeIndex >= 0) {
    tick().then(() => {
      try {
        const matchEl = document.querySelector(".search-match-active") as HTMLElement | null;
        matchEl?.scrollIntoView({ block: "nearest", behavior: "smooth" });
      } catch {}
    });
  }
}

function dispatchSearch() {
  if (!editor) return;
  editor.view.dispatch(
    editor.view.state.tr.setMeta(searchPluginKey, {
      query: searchQuery,
      caseSensitive: searchCaseSensitive,
      wholeWord: searchWholeWord,
      useRegex: searchUseRegex,
    }),
  );
  // sync will happen via onTransaction
}

function openSearch(withReplace = false) {
  searchOpen = true;
  if (withReplace) searchReplaceOpen = true;
  tick().then(() => searchInputEl?.focus());
  if (searchQuery) dispatchSearch();
}

function closeSearch() {
  searchOpen = false;
  searchReplaceOpen = false;
  if (editor) {
    editor.view.dispatch(editor.view.state.tr.setMeta(searchPluginKey, { query: "" }));
  }
  editor?.commands.focus();
}

function goSearch(delta: number) {
  if (!editor) return;
  editor.view.dispatch(editor.view.state.tr.setMeta(searchPluginKey, { activeDelta: delta }));
  tick().then(() => editor && syncSearchState(editor));
}

function replaceOne() {
  if (!editor || !searchQuery) return;
  const state = searchPluginKey.getState(editor.state);
  if (!state || state.matches.length === 0 || state.activeIndex < 0) return;
  const { from, to } = state.matches[state.activeIndex];
  let replacement = replaceQuery;
  if (searchUseRegex) {
    try {
      const regex = buildSearchRegex(searchQuery, searchCaseSensitive, searchWholeWord, true);
      if (regex) {
        const text = editor.state.doc.textBetween(from, to, "\n");
        const nonGlobal = new RegExp(regex.source, regex.flags.replace(/g/g, ""));
        replacement = text.replace(nonGlobal, replaceQuery);
      }
    } catch {}
  }
  const tr = editor.state.tr.insertText(replacement, from, to);
  editor.view.dispatch(tr);
  editor.commands.focus();
}

function replaceAll() {
  if (!editor || !searchQuery) return;
  const state = searchPluginKey.getState(editor.state);
  if (!state || state.matches.length === 0) return;
  const matches = [...state.matches].sort((a, b) => b.from - a.from);
  let tr = editor.state.tr;
  for (const m of matches) {
    let replacement = replaceQuery;
    if (searchUseRegex) {
      try {
        const regex = buildSearchRegex(searchQuery, searchCaseSensitive, searchWholeWord, true);
        if (regex) {
          const text = editor.state.doc.textBetween(m.from, m.to, "\n");
          const nonGlobal = new RegExp(regex.source, regex.flags.replace(/g/g, ""));
          replacement = text.replace(nonGlobal, replaceQuery);
        }
      } catch {}
    }
    tr = tr.insertText(replacement, m.from, m.to);
  }
  editor.view.dispatch(tr);
  editor.commands.focus();
}

function handleSearchKeydown(event: KeyboardEvent) {
  if (event.key === "Enter") {
    event.preventDefault();
    goSearch(event.shiftKey ? -1 : 1);
  } else if (event.key === "Escape") {
    event.preventDefault();
    closeSearch();
  }
}

function openInsertAsset() {
  if (!editor || !editable) return;
  imageReplaceMode = false;
  const { from, to } = editor.state.selection;
  insertAssetRange = { from, to };
  insertAssetInitialAlign = getCurrentAlign();
  insertAssetOpen = true;
}
function handleInsertAsset(
  asset: Asset | null,
  meta?: { alt: string; title: string; width: string; height: string; align: "" | "left" | "center" | "right" },
) {
  if (!editor || !editorState) return;
  const range = insertAssetRange ?? editor.state.selection;
  // close first to avoid focus issues
  insertAssetOpen = false;
  // Handle edit of existing image without picking new asset (meta only)
  if (imageReplaceMode && imagePopover) {
    const from = imagePopover.from;
    const node = editorState.state.doc.nodeAt(from);
    if (node && node.type.name === "image") {
      if (asset && asset.mime_type.startsWith("image/")) {
        const src = asset.path;
        const alt = meta?.alt?.trim() ? meta.alt.trim() : asset.filename;
        const title = meta?.title?.trim() ? meta.title.trim() : alt;
        const width = meta?.width && /^\d+$/.test(meta.width) ? Number(meta.width) : null;
        const height = meta?.height && /^\d+$/.test(meta.height) ? Number(meta.height) : null;
        try {
          editorState
            .chain()
            .focus()
            .setNodeSelection(from)
            .updateAttributes("image", { src, alt, title, width, height })
            .run();
          imagePopover = {
            ...imagePopover,
            src,
            alt,
            title,
            width: width ? String(width) : "",
            height: height ? String(height) : "",
          };
          imageDraftAlt = alt;
          imageDraftTitle = title;
          imageDraftWidth = width ? String(width) : "";
          imageDraftHeight = height ? String(height) : "";
          imageTitleCustom = !!(title && title !== alt);
          probeNatural(src);
          if (meta?.align === "left" || meta?.align === "center" || meta?.align === "right") {
            try {
              editorState.chain().focus().setTextAlign(meta.align).run();
            } catch {}
          }
        } catch {
          const md = `![${alt}](${src})`;
          const html = markdownToHtml(md);
          try {
            editor.chain().focus().insertContentAt(range.from, html).run();
          } catch {}
        }
      } else if (!asset && meta) {
        // meta-only edit (no new asset)
        const alt = meta.alt;
        const title = meta.title;
        const width = meta.width && /^\d+$/.test(meta.width) ? Number(meta.width) : null;
        const height = meta.height && /^\d+$/.test(meta.height) ? Number(meta.height) : null;
        try {
          editorState
            .chain()
            .focus()
            .setNodeSelection(from)
            .updateAttributes("image", { alt, title, width, height })
            .run();
          imagePopover = {
            ...imagePopover,
            alt,
            title,
            width: width ? String(width) : "",
            height: height ? String(height) : "",
          };
          imageDraftAlt = alt;
          imageDraftTitle = title;
          imageDraftWidth = width ? String(width) : "";
          imageDraftHeight = height ? String(height) : "";
          imageTitleCustom = !!(title && title !== alt);
          if (meta?.align === "left" || meta?.align === "center" || meta?.align === "right") {
            try {
              editorState.chain().focus().setTextAlign(meta.align).run();
            } catch {}
          }
        } catch {}
      } else if (asset) {
        // non-image replace? fallback to file link insertion at same position
        const src = asset.path;
        const alt = meta?.alt?.trim() ? meta.alt.trim() : asset.filename;
        try {
          editor
            .chain()
            .focus()
            .setNodeSelection(from)
            .deleteSelection()
            .setTextSelection(from)
            .insertContentAt(from, [
              { type: "text", text: alt, marks: [{ type: "link", attrs: { href: src } }] } as any,
              { type: "text", text: " " } as any,
            ])
            .run();
          hideImagePopover();
        } catch {}
      }
      imageReplaceMode = false;
      insertAssetRange = null;
      tick().then(() => editor?.commands.focus());
      return;
    }
  }
  // Normal insert (not replace mode)
  if (!asset) {
    imageReplaceMode = false;
    insertAssetRange = null;
    tick().then(() => editor?.commands.focus());
    return;
  }
  const isImg = asset.mime_type.startsWith("image/");
  const src = asset.path;
  const alt = meta?.alt?.trim() ? meta.alt.trim() : asset.filename;
  const title = meta?.title?.trim() ? meta.title.trim() : alt;
  const width = meta?.width && /^\d+$/.test(meta.width) ? Number(meta.width) : null;
  const height = meta?.height && /^\d+$/.test(meta.height) ? Number(meta.height) : null;
  imageReplaceMode = false;
  try {
    if (isImg) {
      // @ts-ignore setImage from Image extension
      const chain: any = editor.chain().focus();
      if (range.from !== range.to) chain.setTextSelection(range);
      const attrs: Record<string, unknown> = { src, alt, title };
      if (width != null) attrs.width = width;
      if (height != null) attrs.height = height;
      chain.setImage(attrs).run();
      if (meta?.align === "left" || meta?.align === "center" || meta?.align === "right") {
        try {
          editor.chain().focus().setTextAlign(meta.align).run();
        } catch {}
      }
    } else {
      editor
        .chain()
        .focus()
        .setTextSelection(range.from === range.to ? { from: range.from, to: range.to } : range)
        .insertContentAt(range, [
          { type: "text", text: alt, marks: [{ type: "link", attrs: { href: src } }] } as any,
          { type: "text", text: " " } as any,
        ])
        .run();
    }
  } catch {
    // fallback: parse markdown to HTML before inserting so tiptap creates proper nodes
    const md = isImg ? `![${alt}](${src})` : `[${alt}](${src})`;
    const html = markdownToHtml(md);
    try {
      editor.chain().focus().insertContentAt(range.from, html).run();
      if (isImg && (meta?.align === "left" || meta?.align === "center" || meta?.align === "right")) {
        try {
          editor.chain().focus().setTextAlign(meta.align).run();
        } catch {}
      }
    } catch {}
  }
  insertAssetRange = null;
  tick().then(() => editor?.commands.focus());
}
function cancelInsertAsset() {
  insertAssetOpen = false;
  insertAssetRange = null;
  imageReplaceMode = false;
  editor?.commands.focus();
}

function openInsertTable() {
  if (!editable || !editorState) return;
  tableInsertOpen = true;
}

function cancelInsertTable() {
  tableInsertOpen = false;
  editor?.commands.focus();
}

function confirmInsertTable(options: { rows: number; cols: number }) {
  tableInsertOpen = false;
  if (!editorState) return;
  editorState
    .chain()
    .focus()
    .insertTable({
      rows: options.rows,
      cols: options.cols,
      withHeaderRow: true,
    })
    .run();
}

function runTableCommand(command: (currentEditor: Editor) => boolean) {
  if (!editorState?.isActive("table")) return;
  command(editorState);
}

onMount(() => {
  try {
    const savedAspect = localStorage.getItem("daena:imagePreserveAspect");
    if (savedAspect !== null) imagePreserveAspect = savedAspect === "true";
  } catch {}
  window.addEventListener("keydown", handleFullscreenKeydown);
  const handleLinkPopoverOutside = (event: MouseEvent) => {
    if (!linkPopover) return;
    const target = event.target as HTMLElement | null;
    if (target?.closest(".link-popover") || target?.closest(".link-dialog")) return;
    if (target?.closest("a[href]:not([data-entity-id])")) return;
    hideLinkPopover();
  };
  const handleEntityReferenceMenuOutside = (event: MouseEvent) => {
    if (!entityReferenceMenuOpen) return;
    const target = event.target as HTMLElement | null;
    if (target?.closest(".entity-reference-menu") || target?.closest(".entity-reference-dialog")) return;
    cancelEntityReference();
  };
  const handleImagePopoverOutside = (event: MouseEvent) => {
    if (!imagePopover) return;
    const target = event.target as HTMLElement | null;
    if (target?.closest(".image-popover") || target?.closest(".insert-asset-dialog")) return;
    if (target?.closest("img")) return;
    hideImagePopover();
  };
  const handleImageResize = () => {
    if (imagePopover) hideImagePopover();
  };
  window.addEventListener("mousedown", handleLinkPopoverOutside);
  window.addEventListener("mousedown", handleEntityReferenceMenuOutside);
  window.addEventListener("mousedown", handleImagePopoverOutside);
  window.addEventListener("resize", hideLinkPopover);
  window.addEventListener("resize", handleImageResize);
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
      LanguageCodeBlock,
      HorizontalRule,
      HardBreak,
      BulletList,
      OrderedList,
      ListItem,
      TaskList,
      TaskItem.configure({ nested: true }),
      Table.configure({ resizable: false, allowTableNodeSelection: true }),
      TableRow,
      AlignedTableHeader,
      AlignedTableCell,
      Spoiler,
      SearchAndReplace,
      ExternalLink.configure({ openOnClick: false, autolink: true, linkOnPaste: true }),
      EntityReference,
      AssetImage.configure({ inline: true, allowBase64: false, HTMLAttributes: { class: "daena-content-image" } }),
      TextAlign.configure({ types: ["heading", "paragraph"], alignments: ["left", "center", "right"] }),
      TextDirection.configure({ types: ["heading", "paragraph"] }),
      UndoRedo,
    ],
    content: editorHtmlFromMarkdown(value),
    editable,
    editorProps: {
      attributes: {
        "aria-label": "Document editor",
        "aria-multiline": "true",
        spellcheck: "true",
      },
      handleClick: (_view, _position, event) => {
        const spoilerEl = getSpoilerEl(event.target);
        if (spoilerEl) {
          event.preventDefault();
          const revealed = spoilerEl.classList.toggle("revealed");
          spoilerEl.setAttribute("aria-expanded", revealed ? "true" : "false");
          return true;
        }
        if (handleImageClick(event.target, _position)) {
          event.preventDefault();
          event.stopPropagation();
          return true;
        }
        const linkAnchor = getExternalLinkAnchor(event.target);
        if (linkAnchor) {
          event.preventDefault();
          event.stopPropagation();
          const href = linkAnchor.getAttribute("href") ?? "";
          if (!href) return true;
          const targetEditor = editor ?? editorState;
          const linkType = targetEditor?.state.schema.marks.link;
          const doc = targetEditor?.state.doc;
          let from = _position;
          let to = _position;
          let text = linkAnchor.textContent ?? "";
          if (linkType && doc) {
            try {
              const $pos = doc.resolve(_position);
              const range = getMarkRange($pos, linkType);
              if (range) {
                from = range.from;
                to = range.to;
                text = doc.textBetween(range.from, range.to, " ");
              }
            } catch {}
          }
          if (linkPopover && linkPopover.href === href && linkPopover.from === from && linkPopover.to === to)
            hideLinkPopover();
          else showLinkPopover(href, text, from, to, linkAnchor.getBoundingClientRect());
          return true;
        }
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
        click: (_view, event) => {
          const img = (event.target as HTMLElement | null)?.closest?.("img");
          if (img && editorElement.contains(img)) {
            // Let handleClick (handleClickOn) show the image popover; do not swallow here.
            // Only prevent default link navigation if img is inside a link – delegate to image handler.
            try {
              const targetView = _view as any;
              let posHint: number | undefined;
              try {
                const c = { left: (event as MouseEvent).clientX, top: (event as MouseEvent).clientY };
                const probe = targetView.posAtCoords?.(c);
                if (probe?.pos != null) posHint = probe.pos;
                else if (targetView.posAtDOM) posHint = targetView.posAtDOM(img, 0);
              } catch {}
              if (handleImageClick(event.target, posHint)) {
                event.preventDefault();
                event.stopPropagation();
                return true;
              }
            } catch {}
            // Fallback: at least prevent navigation, still return true to avoid double handling
            event.preventDefault();
            event.stopPropagation();
            return true;
          }
          const linkAnchor = getExternalLinkAnchor(event.target);
          if (linkAnchor) {
            event.preventDefault();
            event.stopPropagation();
            return true;
          }
          return false;
        },
        auxclick: (_view, event) => {
          const linkAnchor = getExternalLinkAnchor(event.target);
          if (linkAnchor) {
            event.preventDefault();
            event.stopPropagation();
            return true;
          }
          return false;
        },
      },
      handleKeyDown: (_view, event) => {
        if (
          (event.key === "Enter" || event.key === " ") &&
          (event.target as HTMLElement | null)?.closest("span[data-spoiler]")
        ) {
          event.preventDefault();
          const el = (event.target as HTMLElement).closest("span[data-spoiler]") as HTMLElement;
          const revealed = el.classList.toggle("revealed");
          el.setAttribute("aria-expanded", revealed ? "true" : "false");
          return true;
        }
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
    onUpdate: () => scheduleChange(),
    onBlur: () => flushPendingChanges(),
    onTransaction: ({ editor: nextEditor, transaction }) => {
      if (entityReferenceSuppressedRange && transaction) {
        const newFrom = transaction.mapping.map(entityReferenceSuppressedRange.from, -1);
        const newTo = transaction.mapping.map(entityReferenceSuppressedRange.to, -1);
        entityReferenceSuppressedRange = { from: newFrom, to: newTo };
        try {
          const check = nextEditor.state.doc.textBetween(newFrom, newFrom + 1, "\n");
          if (check !== "@") entityReferenceSuppressedRange = null;
        } catch {
          entityReferenceSuppressedRange = null;
        }
        if (entityReferenceSuppressedRange) {
          const fromResult = transaction.mapping.mapResult(entityReferenceSuppressedRange.from, -1);
          const toResult = transaction.mapping.mapResult(entityReferenceSuppressedRange.to, -1);
          if (fromResult.deleted || toResult.deleted) entityReferenceSuppressedRange = null;
        }
      }
      editorState = nextEditor;
      emitSelection();
      updateEntityReferenceTrigger(nextEditor);
      syncEntityReferenceEditor(nextEditor);
      syncSearchState(nextEditor);
      syncLinkPopover(nextEditor);
      syncImagePopover(nextEditor, transaction);
    },
  });
  editorState = editor;
  currentMarkdown = markdownFromEditorHtml(editor.getHTML());
  editorText = editorPlainText(editor);
  const preventLinkNavigationCapture = (event: MouseEvent) => {
    const anchor = getExternalLinkAnchor(event.target);
    if (anchor) event.preventDefault();
  };
  const preventWindowLinkClickCapture = (event: MouseEvent) => {
    const anchor = getExternalLinkAnchor(event.target);
    if (anchor && editorElement.contains(anchor as Node)) event.preventDefault();
  };
  editorElement.addEventListener("click", preventLinkNavigationCapture, true);
  editorElement.addEventListener("auxclick", preventLinkNavigationCapture, true);
  window.addEventListener("click", preventWindowLinkClickCapture, true);
  window.addEventListener("auxclick", preventWindowLinkClickCapture, true);
  const initialTextFrame = requestAnimationFrame(() => {
    editorText = editor ? editorPlainText(editor) : "";
  });

  return () => {
    cancelPendingChange();
    window.removeEventListener("keydown", handleFullscreenKeydown);
    window.removeEventListener("mousedown", handleLinkPopoverOutside);
    window.removeEventListener("mousedown", handleEntityReferenceMenuOutside);
    window.removeEventListener("mousedown", handleImagePopoverOutside);
    window.removeEventListener("resize", hideLinkPopover);
    window.removeEventListener("resize", handleImageResize);
    window.removeEventListener("click", preventWindowLinkClickCapture, true);
    window.removeEventListener("auxclick", preventWindowLinkClickCapture, true);
    editorElement.removeEventListener("click", preventLinkNavigationCapture, true);
    editorElement.removeEventListener("auxclick", preventLinkNavigationCapture, true);
    cancelAnimationFrame(initialTextFrame);
    editor?.destroy();
  };
});

$: if (editor && !editor.isFocused && value !== currentMarkdown) {
  cancelPendingChange();
  const nextHtml = editorHtmlFromMarkdown(value);
  if (nextHtml !== editor.getHTML()) editor.commands.setContent(nextHtml, { emitUpdate: false });
  currentMarkdown = markdownFromEditorHtml(editor.getHTML());
  editorText = editorPlainText(editor);
}
$: if (editor && entities) {
  // live update auto references when entity names change
  tick().then(() => {
    if (!editor) return;
    if (editor.isFocused) updateAutoEntityReferences();
    else {
      const hydrated = editorHtmlFromMarkdown(value);
      if (hydrated !== editor.getHTML()) {
        editor.commands.setContent(hydrated, { emitUpdate: false });
        currentMarkdown = markdownFromEditorHtml(editor.getHTML());
        editorText = editorPlainText(editor);
      } else updateAutoEntityReferences();
    }
  });
}
$: if (editor && editor.isEditable !== editable) editor.setEditable(editable);
$: inTable = editorState?.isActive("table") ?? false;
</script>

<div class="editor-shell">
  <div class="editor-toolbar" role="toolbar" aria-label="Formatting tools">
    <div class="editor-toolbar-tools">
      <div class="toolbar-group" aria-label="History">
        <button
          class="history-button"
          type="button"
          title="Undo (⌘/Ctrl + Z)"
          aria-label="Undo"
          disabled={!editorState?.can().undo()}
          onclick={() => run((currentEditor) => currentEditor.chain().focus().undo().run())}
          ><Undo2 size={14} strokeWidth={1.8} /></button>
        <button
          class="history-button"
          type="button"
          title="Redo (⌘/Ctrl + Shift + Z)"
          aria-label="Redo"
          disabled={!editorState?.can().redo()}
          onclick={() => run((currentEditor) => currentEditor.chain().focus().redo().run())}
          ><Redo2 size={14} strokeWidth={1.8} /></button>
      </div>

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

      <div class="toolbar-group" aria-label="Text formatting">
        <button
          type="button"
          title="Bold (⌘/Ctrl + B)"
          aria-label="Bold"
          aria-pressed={editorState?.isActive("bold") ?? false}
          class:active={editorState?.isActive("bold")}
          onclick={() => run((currentEditor) => currentEditor.chain().focus().toggleBold().run())}
          ><BoldIcon size={14} strokeWidth={1.8} /></button>
        <button
          type="button"
          title="Italic (⌘/Ctrl + I)"
          aria-label="Italic"
          aria-pressed={editorState?.isActive("italic") ?? false}
          class:active={editorState?.isActive("italic")}
          onclick={() => run((currentEditor) => currentEditor.chain().focus().toggleItalic().run())}
          ><ItalicIcon size={14} strokeWidth={1.8} /></button>
        <button
          type="button"
          title="Underline (⌘/Ctrl + U)"
          aria-label="Underline"
          aria-pressed={editorState?.isActive("underline") ?? false}
          class:active={editorState?.isActive("underline")}
          onclick={() => run((currentEditor) => currentEditor.chain().focus().toggleUnderline().run())}
          ><UnderlineIcon size={14} strokeWidth={1.8} /></button>
        <button
          type="button"
          title="Strikethrough"
          aria-label="Strikethrough"
          aria-pressed={editorState?.isActive("strike") ?? false}
          class:active={editorState?.isActive("strike")}
          onclick={() => run((currentEditor) => currentEditor.chain().focus().toggleStrike().run())}
          ><StrikethroughIcon size={14} strokeWidth={1.8} /></button>
        <button
          type="button"
          title="Spoiler (hidden text)"
          aria-label="Spoiler"
          aria-pressed={editorState?.isActive("spoiler") ?? false}
          class:active={editorState?.isActive("spoiler")}
          onclick={() => run((currentEditor) => (currentEditor.chain().focus() as any).toggleSpoiler().run())}
          ><EyeOff size={14} strokeWidth={1.8} /></button>
      </div>
      <div class="toolbar-group" aria-label="Links and media">
        <button
          type="button"
          title="Link (⌘/Ctrl + K)"
          aria-label="Link"
          aria-pressed={editorState?.isActive("link") ?? false}
          class:active={editorState?.isActive("link")}
          onclick={setLink}><LinkIcon size={14} strokeWidth={1.8} /></button>
        <button
          type="button"
          title="Link to lore entry (@)"
          aria-label="Link to lore entry"
          onclick={() => {
            if (!editorState) return;
            editorState.chain().focus().insertContent("@").run();
          }}><AtSign size={14} strokeWidth={1.8} /></button>
        <button
          type="button"
          title="Insert image or file"
          aria-label="Insert image or file"
          disabled={!editable}
          onclick={openInsertAsset}><ImageIcon size={14} strokeWidth={1.8} /></button>
      </div>
      <div class="toolbar-group" aria-label="Text direction">
        <button
          type="button"
          title="Left to right (⌘/Ctrl + Alt + L) — click again to auto"
          aria-label="Left to right"
          aria-pressed={isDirection("ltr")}
          class:active={isDirection("ltr")}
          onclick={() =>
            run((currentEditor) =>
              isDirection("ltr")
                ? (currentEditor.chain().focus() as any).unsetTextDirection().run()
                : (currentEditor.chain().focus() as any).setTextDirection("ltr").run(),
            )}><ArrowRightToLine size={14} strokeWidth={1.8} /></button>
        <button
          type="button"
          title="Right to left (⌘/Ctrl + Alt + R) — click again to auto"
          aria-label="Right to left"
          aria-pressed={isDirection("rtl")}
          class:active={isDirection("rtl")}
          onclick={() =>
            run((currentEditor) =>
              isDirection("rtl")
                ? (currentEditor.chain().focus() as any).unsetTextDirection().run()
                : (currentEditor.chain().focus() as any).setTextDirection("rtl").run(),
            )}><ArrowLeftToLine size={14} strokeWidth={1.8} /></button>
      </div>
      <div class="toolbar-group" aria-label="Text alignment">
        <button
          type="button"
          title="Align left"
          aria-label="Align left"
          aria-pressed={isAligned("left")}
          class:active={isAligned("left")}
          onclick={() => run((currentEditor) => currentEditor.chain().focus().setTextAlign("left").run())}
          ><TextAlignStart size={14} strokeWidth={1.8} /></button>
        <button
          type="button"
          title="Align center"
          aria-label="Align center"
          aria-pressed={isAligned("center")}
          class:active={isAligned("center")}
          onclick={() => run((currentEditor) => currentEditor.chain().focus().setTextAlign("center").run())}
          ><TextAlignCenter size={14} strokeWidth={1.8} /></button>
        <button
          type="button"
          title="Align right"
          aria-label="Align right"
          aria-pressed={isAligned("right")}
          class:active={isAligned("right")}
          onclick={() => run((currentEditor) => currentEditor.chain().focus().setTextAlign("right").run())}
          ><TextAlignEnd size={14} strokeWidth={1.8} /></button>
      </div>

      <div class="toolbar-group" aria-label="Lists and blocks">
        <button
          type="button"
          title="Bulleted list"
          aria-label="Bulleted list"
          aria-pressed={editorState?.isActive("bulletList") ?? false}
          class:active={editorState?.isActive("bulletList")}
          onclick={() => run((currentEditor) => currentEditor.chain().focus().toggleBulletList().run())}
          ><List size={14} strokeWidth={1.8} /></button>
        <button
          type="button"
          title="Numbered list"
          aria-label="Numbered list"
          aria-pressed={editorState?.isActive("orderedList") ?? false}
          class:active={editorState?.isActive("orderedList")}
          onclick={() => run((currentEditor) => currentEditor.chain().focus().toggleOrderedList().run())}
          ><ListOrdered size={14} strokeWidth={1.8} /></button>
        <button
          type="button"
          title="Quote"
          aria-label="Quote"
          aria-pressed={editorState?.isActive("blockquote") ?? false}
          class:active={editorState?.isActive("blockquote")}
          onclick={() => run((currentEditor) => currentEditor.chain().focus().toggleBlockquote().run())}
          ><QuoteIcon size={14} strokeWidth={1.8} /></button>
        <button
          type="button"
          title="Horizontal rule"
          aria-label="Horizontal rule"
          onclick={() => run((currentEditor) => currentEditor.chain().focus().setHorizontalRule().run())}
          ><SeparatorHorizontal size={14} strokeWidth={1.8} /></button>
        <button
          type="button"
          title="Insert table"
          aria-label="Insert table"
          aria-pressed={inTable}
          class:active={inTable}
          disabled={!editable}
          onclick={openInsertTable}><TableIcon size={14} strokeWidth={1.8} /></button>
        <button
          type="button"
          title="Clear formatting"
          aria-label="Clear formatting"
          onclick={() => run((currentEditor) => currentEditor.chain().focus().clearNodes().unsetAllMarks().run())}
          ><Eraser size={14} strokeWidth={1.8} /></button>
      </div>
      {#if inTable}
        <div class="toolbar-group" aria-label="Table">
          <button
            type="button"
            title="Add row below"
            aria-label="Add row below"
            disabled={!editable || !editorState?.can().addRowAfter()}
            onclick={() => runTableCommand((currentEditor) => currentEditor.chain().focus().addRowAfter().run())}
            ><BetweenHorizontalEnd size={14} strokeWidth={1.8} /></button>
          <button
            type="button"
            title="Delete row"
            aria-label="Delete row"
            disabled={!editable || !editorState?.can().deleteRow()}
            onclick={() => runTableCommand((currentEditor) => currentEditor.chain().focus().deleteRow().run())}
            ><BetweenHorizontalStart size={14} strokeWidth={1.8} /></button>
          <button
            type="button"
            title="Add column after"
            aria-label="Add column after"
            disabled={!editable || !editorState?.can().addColumnAfter()}
            onclick={() => runTableCommand((currentEditor) => currentEditor.chain().focus().addColumnAfter().run())}
            ><BetweenVerticalEnd size={14} strokeWidth={1.8} /></button>
          <button
            type="button"
            title="Delete column"
            aria-label="Delete column"
            disabled={!editable || !editorState?.can().deleteColumn()}
            onclick={() => runTableCommand((currentEditor) => currentEditor.chain().focus().deleteColumn().run())}
            ><BetweenVerticalStart size={14} strokeWidth={1.8} /></button>
          <button
            type="button"
            title="Delete table"
            aria-label="Delete table"
            disabled={!editable || !editorState?.can().deleteTable()}
            onclick={() => runTableCommand((currentEditor) => currentEditor.chain().focus().deleteTable().run())}
            ><Trash2 size={14} strokeWidth={1.8} /></button>
        </div>
      {/if}
    </div>
    <div class="editor-toolbar-actions">
      {#if aiEnabled}
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
            onclick={() => (aiMenuOpen = !aiMenuOpen)}><SparklesIcon size={14} strokeWidth={1.8} /></button>
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
      {/if}

      <button
        class="search-toggle"
        type="button"
        title="Find and replace (⌘/Ctrl + F)"
        aria-label="Find and replace"
        aria-pressed={searchOpen}
        class:active={searchOpen}
        onclick={() => (searchOpen ? closeSearch() : openSearch(false))}
        ><SearchIcon size={14} strokeWidth={1.8} /></button>
      <button
        class="fullscreen-toggle"
        type="button"
        title={isFullscreen ? "Exit full screen editor (Esc)" : "Open full screen editor"}
        aria-label={isFullscreen ? "Exit full screen editor" : "Open full screen editor"}
        aria-pressed={isFullscreen}
        onclick={toggleFullscreen}
        >{#if isFullscreen}<Minimize2 size={14} strokeWidth={1.8} />{:else}<Maximize2
            size={14}
            strokeWidth={1.8} />{/if}</button>
    </div>
  </div>
  {#if searchOpen}
    <div class="search-bar" role="search" aria-label="Find and replace">
      <div class="search-row">
        <div class="search-input-group">
          <SearchIcon size={12} strokeWidth={1.8} class="search-input-icon" />
          <input
            bind:this={searchInputEl}
            type="text"
            placeholder="Find"
            aria-label="Find"
            bind:value={searchQuery}
            oninput={dispatchSearch}
            onkeydown={handleSearchKeydown} />
          <div class="search-inline-options">
            <button
              type="button"
              class="search-inline-btn"
              title="Match case (Aa)"
              aria-label="Match case"
              aria-pressed={searchCaseSensitive}
              class:active={searchCaseSensitive}
              onclick={() => {
                searchCaseSensitive = !searchCaseSensitive;
                dispatchSearch();
              }}>Aa</button>
            <button
              type="button"
              class="search-inline-btn"
              title="Match whole word"
              aria-label="Match whole word"
              aria-pressed={searchWholeWord}
              class:active={searchWholeWord}
              onclick={() => {
                searchWholeWord = !searchWholeWord;
                dispatchSearch();
              }}>wd</button>
            <button
              type="button"
              class="search-inline-btn"
              title="Use regular expression"
              aria-label="Use regular expression"
              aria-pressed={searchUseRegex}
              class:active={searchUseRegex}
              onclick={() => {
                searchUseRegex = !searchUseRegex;
                dispatchSearch();
              }}>.*</button>
          </div>
        </div>
        <button
          type="button"
          class="search-icon-btn"
          title={searchReplaceOpen ? "Hide replace" : "Show replace (Ctrl+H or ⌘⌥F)"}
          aria-label="Toggle replace"
          aria-pressed={searchReplaceOpen}
          class:active={searchReplaceOpen}
          onclick={() => {
            searchReplaceOpen = !searchReplaceOpen;
            tick().then(() => (searchReplaceOpen ? replaceInputEl?.focus() : searchInputEl?.focus()));
          }}><ReplaceIcon size={14} strokeWidth={1.8} /></button>
        <div class="search-nav-group">
          <button
            type="button"
            class="search-nav"
            title="Previous (Shift+Enter)"
            aria-label="Previous match"
            disabled={!searchMatchCount}
            onclick={() => goSearch(-1)}>‹</button>
          <button
            type="button"
            class="search-nav"
            title="Next (Enter)"
            aria-label="Next match"
            disabled={!searchMatchCount}
            onclick={() => goSearch(1)}>›</button>
          <span class="search-count"
            >{searchQuery ? `${searchMatchCount ? searchActiveIndex + 1 : 0}/${searchMatchCount}` : "0/0"}</span>
        </div>
        <button type="button" class="search-close" title="Close (Esc)" aria-label="Close search" onclick={closeSearch}
          ><XIcon size={14} strokeWidth={1.8} /></button>
      </div>
      {#if searchReplaceOpen}
        <div class="search-row replace-row">
          <div class="search-input-group">
            <ReplaceIcon size={12} strokeWidth={1.8} class="search-input-icon" />
            <input
              bind:this={replaceInputEl}
              type="text"
              placeholder="Replace"
              aria-label="Replace"
              bind:value={replaceQuery}
              onkeydown={(e) => {
                if (e.key === "Enter") {
                  e.preventDefault();
                  replaceOne();
                } else if (e.key === "Escape") {
                  e.preventDefault();
                  closeSearch();
                }
              }} />
          </div>
          <button type="button" class="search-action" disabled={!searchMatchCount} onclick={replaceOne}>Replace</button>
          <button type="button" class="search-action" disabled={!searchMatchCount} onclick={replaceAll}>All</button>
        </div>
      {/if}
    </div>
  {/if}

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
  {#if linkPopover && !linkDialogOpen}
    <div
      use:portal
      class="link-popover"
      bind:this={linkPopoverEl}
      style={`top: ${linkPopover.top}px; left: ${linkPopover.left}px;`}
      role="dialog"
      aria-label="Link actions">
      <div class="link-popover-url" title={linkPopover.href}>{linkPopover.href}</div>
      <div class="link-popover-actions">
        <button
          type="button"
          class="link-popover-btn"
          onmousedown={(event) => event.preventDefault()}
          onclick={openLinkExternal}>Open</button>
        <button
          type="button"
          class="link-popover-btn primary"
          onmousedown={(event) => event.preventDefault()}
          onclick={editLinkFromPopover}>Edit</button>
        <button
          type="button"
          class="link-popover-btn danger"
          onmousedown={(event) => event.preventDefault()}
          onclick={unlinkFromPopover}>Unlink</button>
      </div>
      <button
        type="button"
        class="link-popover-close"
        aria-label="Close link popover"
        onmousedown={(event) => event.preventDefault()}
        onclick={hideLinkPopover}><XIcon size={12} strokeWidth={1.8} /></button>
    </div>
  {/if}
  {#if imagePopover}
    <div
      use:portal
      class="image-popover image-popover--compact"
      bind:this={imagePopoverEl}
      style={`top: ${imagePopover.top}px; left: ${imagePopover.left}px;`}
      role="dialog"
      aria-label="Image actions"
      aria-modal="false">
      <div class="image-popover-header">
        <strong>Image</strong>
        <button
          type="button"
          class="image-popover-close"
          aria-label="Close"
          onmousedown={(event) => event.preventDefault()}
          onclick={hideImagePopover}><XIcon size={12} strokeWidth={1.8} /></button>
      </div>
      {#if imageDraftAlt}
        <div class="image-popover-alt" title={imageDraftAlt}>{imageDraftAlt}</div>
      {/if}
      <div class="image-compact-actions">
        <button
          type="button"
          class="image-compact-btn primary"
          onmousedown={(event) => event.preventDefault()}
          onclick={replaceImage}>Edit…</button>
        <button
          type="button"
          class="image-compact-btn danger"
          onmousedown={(event) => event.preventDefault()}
          onclick={removeImage}>Remove</button>
      </div>
      <div class="image-align-row compact">
        <button
          type="button"
          title="Align left"
          aria-label="Align left"
          aria-pressed={isAligned("left")}
          class:active={isAligned("left")}
          onmousedown={(event) => event.preventDefault()}
          onclick={() => alignImage("left")}><TextAlignStart size={14} strokeWidth={1.8} /></button>
        <button
          type="button"
          title="Align center"
          aria-label="Align center"
          aria-pressed={isAligned("center")}
          class:active={isAligned("center")}
          onmousedown={(event) => event.preventDefault()}
          onclick={() => alignImage("center")}><TextAlignCenter size={14} strokeWidth={1.8} /></button>
        <button
          type="button"
          title="Align right"
          aria-label="Align right"
          aria-pressed={isAligned("right")}
          class:active={isAligned("right")}
          onmousedown={(event) => event.preventDefault()}
          onclick={() => alignImage("right")}><TextAlignEnd size={14} strokeWidth={1.8} /></button>
      </div>
    </div>
  {/if}

  <div class="editor-statusbar">
    <span>{wordCountValue} {wordCountValue === 1 ? "word" : "words"}</span>
    <span class="status-separator">·</span>
    <span>{characterCountValue} {characterCountValue === 1 ? "character" : "characters"}</span>
    <span class="status-spacer"></span>
    <span class="editor-mode">Markdown</span>
  </div>
  <EntityReferenceDialog
    open={entityReferenceDialogOpen}
    search={searchEntities ?? (async () => ({ items: [], total: 0, offset: 0, limit: 20, hasMore: false }))}
    {entities}
    initialQuery={entityReferenceDialogMode === "insert" ? entityReferenceQuery : ""}
    initialSelectedId={entityReferenceDialogMode === "edit" ? (entityReferenceEdit?.entityId ?? "") : ""}
    initialLabel={entityReferenceDialogMode === "edit" ? (entityReferenceEdit?.label ?? "") : ""}
    initialIsCustom={entityReferenceDialogMode === "edit" ? (entityReferenceEdit?.isCustom ?? false) : false}
    onInsert={saveEntityReference}
    onCancel={cancelEntityReference} />
  <LinkDialog
    open={linkDialogOpen}
    initialText={linkDialogInitialText}
    initialUrl={linkDialogInitialUrl}
    hasSelection={linkDialogHasSelection}
    onConfirm={confirmLink}
    onCancel={cancelLink}
    onRemove={linkDialogInitialUrl ? removeLink : null} />
  <InsertAssetDialog
    open={insertAssetOpen}
    {entityId}
    {entities}
    {defaultNamespace}
    mode={imageReplaceMode ? "replace" : "insert"}
    initialAlt={imageReplaceMode && imagePopover ? imageDraftAlt : ""}
    initialTitle={imageReplaceMode && imagePopover ? imageDraftTitle : ""}
    initialWidth={imageReplaceMode && imagePopover ? imageDraftWidth : ""}
    initialHeight={imageReplaceMode && imagePopover ? imageDraftHeight : ""}
    initialSrc={imageReplaceMode && imagePopover ? imagePopover.src : ""}
    initialAlign={insertAssetInitialAlign}
    onInsert={handleInsertAsset}
    onCancel={cancelInsertAsset} />
  <TableInsertDialog open={tableInsertOpen} onConfirm={confirmInsertTable} onCancel={cancelInsertTable} />
</div>

<style>
.editor-shell {
  position: relative;
  display: grid;
  grid-template-rows: auto auto minmax(390px, 1fr) auto;
  overflow: hidden;
  border: 1px solid var(--line);
  border-radius: 10px;
  background: var(--surface);
  box-shadow: var(--shadow-sm, 0 2px 8px rgba(38, 42, 33, 0.05));
}
.editor-toolbar {
  grid-row: 1;
  position: relative;
  display: grid;
  grid-template-columns: minmax(0, 1fr) auto;
  align-items: start;
  gap: 6px;
  min-height: 48px;
  padding: 6px 8px;
  border-bottom: 1px solid var(--line);
  background: var(--surface-muted);
  overflow: visible;
}
.editor-toolbar-tools {
  display: flex;
  align-items: center;
  align-content: flex-start;
  gap: 5px;
  min-width: 0;
  flex-wrap: wrap;
}
.editor-toolbar-actions {
  position: relative;
  z-index: 3;
  display: inline-flex;
  align-items: center;
  justify-content: flex-end;
  gap: 1px;
  min-height: 36px;
  padding: 1px;
  border: 1px solid var(--line);
  border-radius: 8px;
  background: var(--surface);
}
.toolbar-group {
  display: inline-flex;
  align-items: center;
  gap: 1px;
  min-height: 36px;
  padding: 1px;
  border: 1px solid var(--line);
  border-radius: 8px;
  background: var(--surface);
  flex: 0 0 auto;
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
  color: var(--ink-soft);
  font: 500 13px/1 var(--font-body, system-ui, sans-serif);
  cursor: pointer;
}
.editor-toolbar button:hover,
.editor-toolbar button:focus-visible,
.editor-toolbar button.active {
  border-color: var(--theme-warning-border, #d3c0a9);
  background: var(--accent-bg);
  color: var(--accent-dark);
  outline: 0;
}
.editor-toolbar button:disabled {
  color: var(--ink-faint);
  cursor: not-allowed;
  opacity: 0.65;
}
.editor-toolbar button:disabled:hover {
  border-color: transparent;
  background: transparent;
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
  border: 1px solid var(--theme-warning-border, #d3c0a9);
  border-radius: 7px;
  background: var(--surface);
  color: var(--ink);
  font: 700 12px/1 var(--font-body, system-ui, sans-serif);
  box-shadow: 0 10px 24px rgba(48, 45, 38, 0.16);
  cursor: pointer;
}
.entity-reference-menu:hover,
.entity-reference-menu:focus-visible {
  border-color: var(--accent);
  background: var(--surface-muted);
  outline: 0;
}
.entity-reference-menu > span {
  color: var(--accent);
  font-size: 15px;
}
.entity-reference-menu kbd {
  padding: 2px 4px;
  border: 1px solid var(--line);
  border-radius: 3px;
  background: var(--canvas);
  color: var(--ink-faint);
  font:
    700 10px/1 ui-monospace,
    monospace;
}
.entity-reference-edit {
  position: fixed;
  z-index: 75;
  min-height: 28px;
  padding: 0 8px;
  border: 1px solid var(--theme-warning-border, #d3c0a9);
  border-radius: 6px;
  background: var(--surface);
  color: var(--accent-dark);
  box-shadow: 0 6px 16px rgba(38, 42, 33, 0.14);
  font: 700 11px/1 var(--font-body, system-ui, sans-serif);
  cursor: pointer;
}
.entity-reference-edit:hover,
.entity-reference-edit:focus-visible {
  border-color: var(--accent);
  background: var(--accent-bg);
  outline: 0;
}
.link-popover {
  position: fixed;
  z-index: 76;
  display: inline-flex;
  align-items: center;
  gap: 8px;
  max-width: min(380px, calc(100vw - 16px));
  min-height: 36px;
  padding: 6px 8px 6px 12px;
  border: 1px solid var(--theme-warning-border, #d3c0a9);
  border-radius: 8px;
  background: var(--surface);
  box-shadow: 0 10px 24px rgba(48, 45, 38, 0.18);
}
.link-popover-url {
  flex: 1 1 auto;
  min-width: 0;
  max-width: 200px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  color: var(--accent-dark);
  font: 500 12px/1 var(--font-body, system-ui, sans-serif);
  text-decoration: underline;
  text-underline-offset: 2px;
}
.link-popover-actions {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  flex: 0 0 auto;
}
.link-popover-btn {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  min-height: 26px;
  padding: 0 8px;
  border: 1px solid var(--theme-warning-border, #d3c0a9);
  border-radius: 6px;
  background: var(--surface);
  color: var(--ink-soft);
  font: 600 11px/1 var(--font-body, system-ui, sans-serif);
  cursor: pointer;
}
.link-popover-btn:hover,
.link-popover-btn:focus-visible {
  border-color: var(--accent);
  background: var(--surface-muted);
  color: var(--ink);
  outline: 0;
}
.link-popover-btn.primary {
  border-color: var(--accent-dark);
  background: var(--accent-dark);
  color: #fff;
}
.link-popover-btn.primary:hover {
  filter: brightness(1.06);
}
.link-popover-btn.danger {
  border-color: transparent;
  color: var(--danger);
}
.link-popover-btn.danger:hover {
  border-color: var(--theme-danger-border, #e8c0b8);
  background: var(--theme-danger-bg, #fdf0ed);
  color: var(--theme-danger-text, #8a3a2f);
}
.link-popover-close {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 22px;
  height: 22px;
  flex: 0 0 auto;
  border: 0;
  border-radius: 4px;
  background: transparent;
  color: var(--ink-faint);
  cursor: pointer;
}
.link-popover-close:hover,
.link-popover-close:focus-visible {
  background: var(--surface-muted);
  color: var(--ink);
  outline: 0;
}
.image-popover {
  position: fixed;
  z-index: 76;
  display: grid;
  gap: 10px;
  max-width: min(380px, calc(100vw - 16px));
  min-width: 320px;
  padding: 12px;
  border: 1px solid var(--theme-warning-border, #d3c0a9);
  border-radius: 10px;
  background: var(--surface);
  box-shadow: 0 12px 28px rgba(48, 45, 38, 0.18);
}
.image-popover--compact {
  min-width: 220px;
  max-width: 260px;
  gap: 8px;
  padding: 10px;
}
.image-popover-alt {
  font: 500 11px/1.4 var(--font-body, system-ui, sans-serif);
  color: var(--ink-soft);
  background: var(--canvas);
  border: 1px solid var(--line);
  border-radius: 6px;
  padding: 6px 8px;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.image-compact-actions {
  display: flex;
  gap: 6px;
}
.image-compact-btn {
  flex: 1;
  min-height: 28px;
  padding: 0 10px;
  border-radius: 6px;
  font: 700 11px/1 var(--font-body, system-ui, sans-serif);
  cursor: pointer;
  border: 1px solid var(--line);
  background: var(--surface);
  color: var(--ink-soft);
}
.image-compact-btn.primary {
  border-color: var(--accent-dark);
  background: var(--accent-dark);
  color: #fff;
}
.image-compact-btn.danger {
  border-color: transparent;
  color: var(--danger);
}
.image-compact-btn:hover,
.image-compact-btn:focus-visible {
  border-color: var(--theme-warning-border, #d3c0a9);
  background: var(--accent-bg);
  outline: 0;
}
.image-compact-btn.primary:hover {
  filter: brightness(1.06);
}
.image-compact-btn.danger:hover {
  border-color: var(--theme-danger-border, #e8c0b8);
  background: var(--theme-danger-bg, #fdf0ed);
}
.image-align-row.compact {
  justify-content: center;
}
.image-popover-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  font: 700 12px/1 var(--font-body, system-ui, sans-serif);
  color: var(--ink);
}
.image-popover-close {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 22px;
  height: 22px;
  border: 0;
  border-radius: 4px;
  background: transparent;
  color: var(--ink-faint);
  cursor: pointer;
}
.image-popover-close:hover,
.image-popover-close:focus-visible {
  background: var(--surface-muted);
  color: var(--ink);
  outline: 0;
}
.image-align-row {
  display: inline-flex;
  gap: 4px;
}
.image-align-row button {
  min-width: 28px;
  height: 28px;
  padding: 0 6px;
  border: 1px solid var(--line);
  border-radius: 6px;
  background: var(--surface);
  color: var(--ink-soft);
  cursor: pointer;
}
.image-align-row button:hover,
.image-align-row button:focus-visible {
  border-color: var(--theme-warning-border, #d3c0a9);
  background: var(--accent-bg);
  outline: 0;
}
.image-align-row button.active {
  border-color: var(--theme-warning-border, #d3c0a9);
  background: var(--accent-bg);
  color: var(--accent-dark);
}
.editor-toolbar button.ai-toolbar-button {
  color: var(--accent-dark);
  font-size: 17px;
}
.editor-toolbar button.ai-toolbar-button:not(:disabled) {
  border-color: var(--theme-warning-border, #d3c0a9);
  background: var(--theme-warning-bg, #f8efe3);
}
.editor-toolbar button.ai-toolbar-button:not(:disabled):hover {
  background: var(--accent-bg);
}
.ai-toolbar-menu {
  position: absolute;
  top: calc(100% + 6px);
  right: 0;
  z-index: 20;
  display: grid;
  min-width: 174px;
  padding: 5px;
  border: 1px solid var(--theme-warning-border, #d8cdbd);
  border-radius: 8px;
  background: var(--surface);
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
  color: var(--ink-soft);
  font-size: 11px;
  text-align: left;
}
.ai-toolbar-menu button:hover,
.ai-toolbar-menu button:focus-visible {
  border-color: transparent;
  background: var(--surface-muted);
  color: var(--ink);
  outline: 0;
}
.ai-toolbar-menu button:disabled {
  color: var(--ink-faint);
  cursor: not-allowed;
  opacity: 0.55;
}
.toolbar-group[aria-label="History"] {
  gap: 2px;
}
.editor-toolbar button.history-button {
  width: 32px;
  min-width: 32px;
  padding: 0;
  font-family: "Apple Symbols", "Segoe UI Symbol", sans-serif;
  font-size: 17px;
  font-weight: 400;
  line-height: 1;
}
.fullscreen-toggle {
  flex: 0 0 32px;
  font-size: 16px !important;
}
.style-select {
  height: 32px;
  min-width: 92px;
  padding: 0 6px;
  border: 1px solid transparent;
  border-radius: 6px;
  background: transparent;
  color: var(--ink-soft);
  font: 500 11px/1 var(--font-body, system-ui, sans-serif);
  cursor: pointer;
}
.style-select:hover,
.style-select:focus-visible {
  border-color: var(--theme-warning-border, #d3c0a9);
  background: var(--surface);
  outline: 0;
}
.editor-content {
  grid-row: 3;
  position: relative;
  min-width: 0;
  padding: 24px 26px 36px;
  color: var(--ink);
  background: var(--canvas);
  font: 400 16px/1.7 var(--font-body, ui-sans-serif, system-ui, sans-serif);
  outline: 0;
  cursor: text;
}
.editor-shell:focus-within {
  border-color: var(--theme-warning-border, #d3c0a9);
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
  border-bottom: 1px solid var(--accent);
  color: var(--accent-dark);
  cursor: pointer;
  text-decoration: none;
}
.editor-content :global(a[data-entity-id]:hover),
.editor-content :global(a[data-entity-id]:focus-visible) {
  border-bottom-color: currentColor;
  border-radius: 2px;
  background: var(--accent-bg);
  outline: 0;
}
.editor-content.is-empty::before {
  position: absolute;
  top: 24px;
  right: 26px;
  left: 26px;
  content: attr(data-placeholder);
  color: var(--ink-faint);
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
  color: var(--ink);
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
  border-left: 3px solid var(--accent);
  background: var(--theme-warning-bg, #fcf8f1);
  color: var(--ink-soft);
  font-style: italic;
}
.editor-content :global(pre) {
  overflow-x: auto;
  margin: 1.2em 0;
  padding: 13px 15px;
  border: 1px solid var(--line);
  border-radius: 7px;
  background: var(--surface-muted);
  color: var(--ink);
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
  background: var(--theme-warning-bg, #ede9e0);
  color: var(--theme-warning-text, #765a39);
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
  border-top: 1px solid var(--line);
}
.editor-content :global(a) {
  color: var(--accent-dark);
  text-decoration: underline;
  text-underline-offset: 2px;
}
.editor-content :global(.search-match) {
  background: var(--theme-warning-bg, #ffe8a3);
  padding: 0 1px;
  border-radius: 2px;
  border-bottom: 1px solid var(--theme-warning-border, #e6c87a);
}
.editor-content :global(.search-match-active) {
  background: #ffb84d;
  padding: 0 1px;
  border-radius: 2px;
  border-bottom: 1px solid var(--theme-warning-border, #d98a1f);
  box-shadow: 0 0 0 1px #ffb84d;
}
.editor-content :global(mark) {
  background: var(--theme-warning-bg, #ffe8a3);
  padding: 0 2px;
  border-radius: 2px;
}
.editor-content :global(span.spoiler) {
  background: #2b2b2b;
  color: transparent;
  border-radius: 3px;
  padding: 0 4px;
  cursor: pointer;
  user-select: none;
  transition:
    color 0.15s ease,
    background 0.15s ease;
}
.editor-content :global(span.spoiler.revealed) {
  background: #3a3a3a;
  color: var(--canvas);
}
.editor-content :global(span.spoiler:focus-visible) {
  outline: 2px solid var(--accent);
  outline-offset: 2px;
}
.editor-content :global(img) {
  max-width: 100%;
  height: auto;
  display: block;
  margin: 0.8em 0;
  border-radius: 6px;
  border: 1px solid var(--line);
}
.editor-content :global(img.daena-content-image) {
  max-width: 100%;
}
.editor-content :global(a) {
  word-break: break-all;
}
.editor-content :global(table) {
  width: 100%;
  margin: 1.2em 0;
  border-collapse: collapse;
  font-size: 0.95em;
}
.editor-content :global(th),
.editor-content :global(td) {
  min-width: 80px;
  padding: 8px 10px;
  border: 1px solid var(--line);
  text-align: left;
  vertical-align: top;
  font-weight: inherit;
}
.editor-content :global(ul[data-type="taskList"]) {
  list-style: none;
  padding-left: 0;
}
.editor-content :global(li[data-type="taskItem"]) {
  display: flex;
  align-items: flex-start;
  gap: 8px;
}
.editor-content :global(li[data-type="taskItem"] > label) {
  flex: 0 0 auto;
  margin-top: 0.35em;
}
.editor-content :global(li[data-type="taskItem"] > div) {
  flex: 1;
}
.editor-statusbar {
  grid-row: 4;
  display: flex;
  align-items: center;
  gap: 6px;
  min-height: 34px;
  padding: 7px 13px;
  border-top: 1px solid var(--line);
  background: var(--surface-muted);
  color: var(--ink-faint);
  font: 11px/1.3 var(--font-body, system-ui, sans-serif);
}
.status-separator {
  color: var(--ink-faint);
}
.status-spacer {
  flex: 1;
}
.editor-mode {
  color: var(--ink-faint);
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
.search-bar {
  grid-row: 2;
  display: grid;
  gap: 4px;
  padding: 8px 12px;
  background: var(--surface-muted);
  border-bottom: 1px solid var(--line);
}
.search-row {
  display: flex;
  align-items: center;
  gap: 4px;
  min-height: 28px;
  width: 100%;
}
.search-input-group {
  display: flex;
  align-items: center;
  gap: 6px;
  flex: 1 1 0;
  min-width: 0;
  height: 28px;
  padding: 0 2px 0 6px;
  border: 1px solid var(--line);
  border-radius: 6px;
  background: var(--canvas);
  overflow: hidden;
}
.search-input-group:focus-within {
  border-color: var(--theme-warning-border, #d3c0a9);
  box-shadow: 0 0 0 2px rgba(211, 192, 169, 0.18);
}
.search-input-group input {
  flex: 1 1 auto;
  min-width: 0;
  height: 22px;
  border: 0;
  background: transparent;
  color: var(--ink);
  font: 12px/1 var(--font-body, system-ui, sans-serif);
  outline: 0;
}
.search-input-group :global(svg) {
  flex: 0 0 auto;
  color: var(--ink-faint);
  opacity: 0.7;
}
.search-inline-options {
  display: inline-flex;
  align-items: center;
  gap: 2px;
  flex: 0 0 auto;
  margin-left: 6px;
  padding-left: 6px;
  border-left: 1px solid var(--line);
  height: 22px;
}
.search-inline-btn {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  min-width: 26px;
  height: 22px;
  padding: 0 4px;
  border: 1px solid transparent;
  border-radius: 5px;
  background: transparent;
  color: var(--ink-soft);
  font: 600 11px/1 var(--font-body, system-ui, sans-serif);
  cursor: pointer;
}
.search-inline-btn:hover,
.search-inline-btn:focus-visible {
  background: var(--accent-bg);
  color: var(--accent-dark);
  outline: 0;
}
.search-inline-btn.active {
  background: var(--accent-bg);
  border-color: var(--theme-warning-border, #d3c0a9);
  color: var(--accent-dark);
}
.search-inline-btn[aria-label="Match whole word"].active {
  text-decoration: underline;
  text-underline-offset: 2px;
  text-decoration-thickness: 1.4px;
}
.search-close,
.search-action {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  min-width: 24px;
  height: 24px;
  padding: 0 4px;
  border: 1px solid transparent;
  border-radius: 5px;
  background: transparent;
  color: var(--ink-soft);
  font: 500 10px/1 var(--font-body, system-ui, sans-serif);
  cursor: pointer;
}
.search-icon-btn {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  min-width: 28px;
  height: 28px;
  padding: 0 5px;
  border: 1px solid transparent;
  border-radius: 6px;
  background: transparent;
  color: var(--ink-soft);
  font: 500 11px/1 var(--font-body, system-ui, sans-serif);
  cursor: pointer;
}
.search-nav {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  min-width: 24px;
  height: 24px;
  padding: 0 4px;
  border: 1px solid transparent;
  border-radius: 5px;
  background: transparent;
  color: var(--ink-soft);
  font: 500 10px/1 var(--font-body, system-ui, sans-serif);
  cursor: pointer;
}
.search-icon-btn.active {
  border-color: var(--theme-warning-border, #d3c0a9);
  background: var(--accent-bg);
  color: var(--accent-dark);
}
.search-icon-btn:hover,
.search-nav:hover,
.search-close:hover,
.search-action:hover,
.search-icon-btn:focus-visible,
.search-nav:focus-visible,
.search-close:focus-visible,
.search-action:focus-visible {
  border-color: var(--theme-warning-border, #d3c0a9);
  background: var(--accent-bg);
  color: var(--accent-dark);
  outline: 0;
}
.search-nav:disabled,
.search-action:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}
.search-nav-group {
  display: inline-flex;
  align-items: center;
  gap: 1px;
  flex: 0 0 auto;
}
.search-nav-group .search-nav {
  min-width: 24px;
  width: 24px;
  height: 24px;
  padding: 0;
  font-size: 14px;
  line-height: 1;
}
.search-count {
  min-width: 42px;
  text-align: center;
  color: var(--ink-soft);
  font: 11px/1 var(--font-body, system-ui, sans-serif);
  white-space: nowrap;
}
.search-close {
  flex: 0 0 auto;
}
.search-action {
  min-width: 56px;
  height: 22px;
  font-size: 11px;
}
@media (max-width: 760px) {
  .editor-toolbar {
    gap: 5px;
    padding: 5px;
  }
  .editor-toolbar-tools {
    gap: 4px;
  }
  .editor-mode {
    display: none;
  }
}
@media (max-width: 560px) {
  .editor-shell {
    grid-template-rows: auto auto minmax(300px, 1fr) auto;
  }
  .editor-toolbar button {
    min-width: 30px;
    height: 30px;
    padding-inline: 7px;
  }
  .toolbar-group,
  .editor-toolbar-actions {
    min-height: 34px;
  }
  .fullscreen-toggle {
    flex-basis: 30px;
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
    min-width: 88px;
  }
}
</style>
