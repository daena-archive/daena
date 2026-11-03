import assert from "node:assert/strict";
import { Window } from "happy-dom";
import { Editor } from "@tiptap/core";
import Document from "@tiptap/extension-document";
import Heading from "@tiptap/extension-heading";
import Paragraph from "@tiptap/extension-paragraph";
import Text from "@tiptap/extension-text";
import TextAlign from "@tiptap/extension-text-align";
import TextDirection, { getTextDirection } from "tiptap-text-direction";
import { htmlToMarkdown, markdownToHtml, parseMarkdown } from "../src/lib/markdown/index.ts";
import { detectTextDirection } from "../src/lib/markdown/direction.ts";
import { directionsForEditor } from "../src/lib/editor/markdownRoundTrip.ts";

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

const FA = "اردشیر و دوستان"; // RTL (Persian)
const FA_HEADING = "فصل یکم";

// The detection mirrored into the markdown package must agree with the editor extension.
for (const sample of ["A text", FA, "", "123", "  — ", "مرحبا hello", "hello مرحبا", "1 اردشیر", "(Ardashir)"]) {
  assert.equal(detectTextDirection(sample), getTextDirection(sample), `direction parity for ${JSON.stringify(sample)}`);
}

// Editor HTML (with the dir attributes TextDirection stamps) serialises to plain Markdown.
assert.equal(htmlToMarkdown('<p dir="ltr">A text</p>'), "A text\n");
assert.equal(htmlToMarkdown(`<p dir="rtl">${FA}</p>`), `${FA}\n`);
assert.equal(htmlToMarkdown('<h2 dir="ltr">Title</h2>'), "## Title\n");
assert.equal(htmlToMarkdown(`<h1 dir="rtl">${FA_HEADING}</h1>`), `# ${FA_HEADING}\n`);
assert.equal(
  htmlToMarkdown(`<h1 dir="ltr">Title</h1><p dir="ltr">One <strong>bold</strong> line.</p><p dir="rtl">${FA}</p>`),
  `# Title\n\nOne **bold** line.\n\n${FA}\n`,
  "mixed document",
);
assert.equal(
  htmlToMarkdown(`<p dir="rtl">${FA} and <em>English</em></p>`),
  `${FA} and *English*\n`,
  "direction follows the first strong character",
);
assert.equal(
  htmlToMarkdown('<p dir="ltr" style="text-align: center">Centered</p>'),
  '<div align="center">Centered</div>\n',
  "redundant dir drops from aligned paragraphs too",
);
assert.equal(
  htmlToMarkdown('<ul><li><p dir="ltr">one</p></li><li><p dir="ltr">two</p></li></ul>'),
  htmlToMarkdown("<ul><li><p>one</p></li><li><p>two</p></li></ul>"),
  "list item paragraphs serialise as without dir",
);
assert.equal(htmlToMarkdown('<blockquote><p dir="ltr">Quoted</p></blockquote>'), "> Quoted\n");

// An explicit override of the detected direction is meaningful and is preserved.
assert.equal(htmlToMarkdown('<p dir="rtl">Mirrored English</p>'), '<p dir="rtl">Mirrored English</p>\n');
assert.equal(htmlToMarkdown(`<p dir="ltr">${FA}</p>`), `<p dir="ltr">${FA}</p>\n`);
assert.equal(htmlToMarkdown('<h2 dir="rtl">Mirrored</h2>'), '<h2 dir="rtl">Mirrored</h2>\n');
for (const markdown of [
  '<p dir="rtl">Mirrored English</p>\n',
  `<p dir="ltr">${FA}</p>\n`,
  '<h2 dir="rtl">Mirrored</h2>\n',
]) {
  assert.equal(htmlToMarkdown(markdownToHtml(markdown)), markdown, `override round trip ${JSON.stringify(markdown)}`);
}

// Saved documents: plain Markdown reopens with the detected direction and re-saves unchanged.
const documents = [
  `# Title\n\nOne **bold** line.\n\n${FA}\n`,
  `# ${FA_HEADING}\n\n${FA}\n\nEnglish again.\n`,
  "- one\n- two\n",
  '<div align="center">Centered</div>\n',
];
for (const markdown of documents) {
  const editorHtml = directionsForEditor(markdownToHtml(markdown));
  assert.equal(htmlToMarkdown(editorHtml), markdown, `plain round trip ${JSON.stringify(markdown)}`);
}
const reopened = directionsForEditor(markdownToHtml(`# ${FA_HEADING}\n\n${FA}\n\nEnglish.\n`));
assert.match(reopened, new RegExp(`<h1[^>]* dir="rtl"[^>]*>${FA_HEADING}`), "RTL heading regains dir for the editor");
assert.match(reopened, new RegExp(`<p[^>]* dir="rtl"[^>]*>${FA}`), "RTL paragraph regains dir for the editor");
assert.match(reopened, /<p[^>]* dir="ltr"[^>]*>English\./, "LTR paragraph regains dir for the editor");
assert.match(
  directionsForEditor('<p dir="rtl">Mirrored English</p>'),
  /dir="rtl"/,
  "an explicit dir already in the document is left alone",
);
assert.equal(directionsForEditor("<p>123</p>"), "<p>123</p>", "text with no strong direction stays unmarked");

// Documents saved before this change used raw <p dir> / <hN dir> for every paragraph; they migrate
// to plain Markdown on the next save and keep any real override.
const legacy = [
  '<h1 dir="ltr">Title</h1>',
  '<p dir="ltr">Plain English paragraph.</p>',
  `<p dir="rtl">${FA}</p>`,
  '<p dir="rtl" style="text-align: right">Forced RTL.</p>',
].join("\n\n");
const migrated = htmlToMarkdown(markdownToHtml(legacy));
assert.equal(
  migrated,
  `# Title\n\nPlain English paragraph.\n\n${FA}\n\n<p dir="rtl" style="text-align: right">Forced RTL.</p>\n`,
);
assert.equal(htmlToMarkdown(directionsForEditor(markdownToHtml(migrated))), migrated, "migration is stable");

// The editor itself: typed paragraphs and headings land as plain Markdown.
function createEditor(content = "<p></p>") {
  return new Editor({
    element: window.document.createElement("div"),
    extensions: [
      Document,
      Paragraph,
      Text,
      Heading,
      TextAlign.configure({ types: ["heading", "paragraph"], alignments: ["left", "center", "right"] }),
      TextDirection.configure({ types: ["heading", "paragraph"] }),
    ],
    content,
  });
}

const typed = createEditor();
typed.commands.insertContent("A text");
assert.match(typed.getHTML(), /<p dir="ltr">A text<\/p>/, "the extension stamps dir on typed text");
assert.equal(htmlToMarkdown(typed.getHTML()), "A text\n");
typed.commands.insertContentAt(typed.state.doc.content.size, `<p>${FA}</p>`);
typed.commands.insertContentAt(typed.state.doc.content.size, "<h2>Next</h2>");
assert.equal(htmlToMarkdown(typed.getHTML()), `A text\n\n${FA}\n\n## Next\n`, "mixed typed document");
typed.commands.setTextDirection("rtl", 1);
assert.equal(
  htmlToMarkdown(typed.getHTML()).split("\n\n")[0],
  '<p dir="rtl">A text</p>',
  "toolbar override survives serialisation",
);
typed.destroy();

// Markdown reaches the same plain form without going through the editor, too.
const tree = parseMarkdown(`${FA}\n`);
assert.equal(tree.children[0].type, "paragraph");

console.log("markdown text direction tests passed");
