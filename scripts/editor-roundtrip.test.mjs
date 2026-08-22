import assert from "node:assert/strict";
import { Window } from "happy-dom";
import { Editor } from "@tiptap/core";
import Document from "@tiptap/extension-document";
import HardBreak from "@tiptap/extension-hard-break";
import { BulletList, ListItem, OrderedList } from "@tiptap/extension-list";
import Paragraph from "@tiptap/extension-paragraph";
import { Table, TableRow } from "@tiptap/extension-table";
import TaskItem from "@tiptap/extension-task-item";
import TaskList from "@tiptap/extension-task-list";
import Text from "@tiptap/extension-text";
import { htmlToMarkdown, markdownToHtml } from "../src/lib/markdown/index.ts";
import { taskListsForEditor, taskListsForMarkdown } from "../src/lib/editor/markdownRoundTrip.ts";
import { AlignedTableCell, AlignedTableHeader } from "../src/lib/editor/editorTable.ts";
import { LanguageCodeBlock } from "../src/lib/editor/editorCodeBlock.ts";

const window = new Window({ url: "http://localhost" });
Object.assign(globalThis, {
  window,
  document: window.document,
  Node: window.Node,
  HTMLElement: window.HTMLElement,
  MutationObserver: window.MutationObserver,
  DOMParser: window.DOMParser,
  getSelection: window.getSelection.bind(window),
});
Object.defineProperty(globalThis, "navigator", { value: window.navigator, configurable: true });

const source = [
  "| Name | State |",
  "| :--- | ---: |",
  "| Draft | Ready |",
  "",
  "- [x] Preserve tables",
  "- [ ] Preserve tasks",
  "",
  "First line  ",
  "Second line",
  "",
  "```ts",
  "const answer = 42;",
  "```",
  "",
].join("\n");

const editor = new Editor({
  element: window.document.createElement("div"),
  extensions: [
    Document,
    Paragraph,
    Text,
    HardBreak,
    LanguageCodeBlock,
    BulletList,
    OrderedList,
    ListItem,
    TaskList,
    TaskItem.configure({ nested: true }),
    Table.configure({ resizable: false, allowTableNodeSelection: true }),
    TableRow,
    AlignedTableHeader,
    AlignedTableCell,
  ],
  content: taskListsForEditor(markdownToHtml(source)),
});

const roundTrip = htmlToMarkdown(taskListsForMarkdown(editor.getHTML()));
assert.match(roundTrip, /\| Name\s+\| State\s+\|/);
assert.match(roundTrip, /\| :[-]+\s+\| [-]+:\s+\|/);
assert.match(roundTrip, /- \[x\] Preserve tables/i);
assert.match(roundTrip, /- \[ \] Preserve tasks/);
assert.match(markdownToHtml(roundTrip), /First line<br>\s*Second line/);
assert.match(roundTrip, /```ts\nconst answer = 42;\n```/);
editor.destroy();

console.log("editor Markdown schema round-trip passed");
