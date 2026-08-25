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
import { htmlToMarkdown, markdownToHtml, parseMarkdown } from "../src/lib/markdown/index.ts";
import { taskListsForEditor, taskListsForMarkdown } from "../src/lib/editor/markdownRoundTrip.ts";
import {
  AlignedTableCell,
  AlignedTableHeader,
  tableHasHeaderRow,
} from "../src/lib/editor/editorTable.ts";
import { LanguageCodeBlock } from "../src/lib/editor/editorCodeBlock.ts";
import { readFile } from "node:fs/promises";

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

function createEditor(content = "<p></p>") {
  return new Editor({
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
    content,
  });
}

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

const editor = createEditor(taskListsForEditor(markdownToHtml(source)));
const roundTrip = htmlToMarkdown(taskListsForMarkdown(editor.getHTML()));
assert.match(roundTrip, /\| Name\s+\| State\s+\|/);
assert.match(roundTrip, /\| :[-]+\s+\| [-]+:\s+\|/);
assert.match(roundTrip, /- \[x\] Preserve tables/i);
assert.match(roundTrip, /- \[ \] Preserve tasks/);
assert.match(markdownToHtml(roundTrip), /First line<br>\s*Second line/);
assert.match(roundTrip, /```ts\nconst answer = 42;\n```/);
const articleTree = parseMarkdown(roundTrip);
assert.equal(
  articleTree.children.some((node) => node.type === "table"),
  true,
  "Article/Wiki AST keeps GFM tables as table nodes",
);
assert.equal(
  articleTree.children.some((node) => node.type === "html"),
  false,
  "tables are not left as raw HTML nodes for Article/Wiki",
);
editor.destroy();

const insertEditor = createEditor();
assert.equal(
  insertEditor.commands.insertTable({ rows: 2, cols: 3, withHeaderRow: true }),
  true,
);
assert.equal(insertEditor.isActive("table"), true);
assert.equal(tableHasHeaderRow(insertEditor), true);
assert.equal(insertEditor.commands.addRowAfter(), true);
assert.equal(insertEditor.commands.addColumnAfter(), true);
let inserted = htmlToMarkdown(taskListsForMarkdown(insertEditor.getHTML()));
assert.match(inserted, /\| {3}\| {3}\| {3}\| {3}\|/);
assert.match(inserted, /\| -+ \| -+ \| -+ \| -+ \|/);
const bodyRows = inserted.split("\n").filter((line) => /^\|/.test(line)).length;
assert.equal(bodyRows, 4, "header + separator + two body rows after addRow");

assert.equal(insertEditor.commands.deleteColumn(), true);
assert.equal(insertEditor.commands.deleteRow(), true);
assert.equal(insertEditor.commands.deleteTable(), true);
assert.equal(insertEditor.isActive("table"), false);
insertEditor.destroy();

const filledEditor = createEditor(
  "<table><thead><tr><th>Name</th><th>State</th></tr></thead><tbody><tr><td>Draft</td><td>Ready</td></tr></tbody></table>",
);
const headedMarkdown = htmlToMarkdown(taskListsForMarkdown(filledEditor.getHTML()));
assert.match(headedMarkdown, /\| Name\s+\| State\s+\|/, "tables with a header use GFM");
assert.doesNotMatch(headedMarkdown, /<table>/, "headed tables prefer GFM over HTML");
const reopened = createEditor(taskListsForEditor(markdownToHtml(headedMarkdown)));
assert.equal(tableHasHeaderRow(reopened), true, "header remains after close/open");
reopened.destroy();
filledEditor.destroy();

const shell = await readFile(new URL("../src/lib/editor/RichTextEditor.svelte", import.meta.url), "utf8");
assert.match(shell, /Insert table/, "toolbar exposes insert table");
assert.match(shell, /TableInsertDialog/, "insert uses a size dialog");
assert.match(shell, /addRowAfter/, "in-table controls can add rows");
assert.match(shell, /addColumnAfter/, "in-table controls can add columns");
assert.match(shell, /deleteTable/, "in-table controls can delete the table");
assert.doesNotMatch(shell, /toggleHeaderRow/, "header removal is not offered");

const dialog = await readFile(new URL("../src/lib/editor/TableInsertDialog.svelte", import.meta.url), "utf8");
assert.match(dialog, /Rows/, "insert dialog asks for row count");
assert.match(dialog, /Columns/, "insert dialog asks for column count");
assert.doesNotMatch(dialog, /withHeaderRow|Header row/, "insert always uses a header row");

console.log("editor Markdown schema round-trip passed");
