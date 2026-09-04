import assert from "node:assert/strict";
import {
  extractEntityReferences,
  htmlToMarkdown,
  markdownToHtml,
  markdownToPlainText,
  parseMarkdown,
} from "../src/lib/markdown/index.ts";
import { MARKDOWN_FIXTURES } from "../src/lib/markdown/markdown.fixtures.ts";

function contains(haystack, needle, name) {
  assert.ok(
    haystack.includes(needle),
    `${name}: expected ${JSON.stringify(haystack)} to include ${JSON.stringify(needle)}`,
  );
}

for (const fixture of MARKDOWN_FIXTURES) {
  const html = markdownToHtml(fixture.markdown);
  if (fixture.name === "rich-inline-and-block-formatting") {
    contains(html, "<h1>", fixture.name);
    contains(html, "<strong>", fixture.name);
    contains(html, "<em>", fixture.name);
    contains(html, 'href="https://example.com"', fixture.name);
    contains(html, "<ul>", fixture.name);
  }
  if (fixture.name === "code-and-quote") {
    contains(html, "<blockquote>", fixture.name);
    contains(html, "<pre>", fixture.name);
    contains(html, "language-ts", fixture.name);
    contains(html, "const answer = 42;", fixture.name);
  }
  if (fixture.name === "underline-and-alignment") {
    contains(html, "<u>", fixture.name);
    contains(html, "text-align: center", fixture.name);
    contains(html, "centered", fixture.name);
  }
  if (fixture.name === "text-color") {
    contains(html, "color: #b4773f", fixture.name);
    contains(html, "gold", fixture.name);
  }
  if (fixture.name === "entity-reference") {
    contains(html, 'data-entity-id="entity-ardashir"', fixture.name);
    contains(html, "Ardashir", fixture.name);
    contains(html, "daena://entity/entity-ardashir", fixture.name);
  }
  if (fixture.name === "strike-hr-ordered-nested") {
    contains(html, "<del>", fixture.name);
    contains(html, "<hr>", fixture.name);
    contains(html, "<ol>", fixture.name);
    contains(html, "<ul>", fixture.name);
  }
  if (fixture.name === "inline-code-and-image") {
    contains(html, "<code>", fixture.name);
    contains(html, 'src="https://example.com/cat.png"', fixture.name);
  }
  if (fixture.name === "unicode") {
    contains(html, "اردشیر", fixture.name);
    contains(html, "café", fixture.name);
  }

  if (fixture.name === "empty") {
    assert.equal(html.trim(), "", fixture.name);
  }

  const roundTrip = htmlToMarkdown(html);
  const again = markdownToHtml(roundTrip);
  if (fixture.markdown.trim()) {
    assert.equal(
      markdownToPlainText(roundTrip).replace(/\s+/g, " "),
      markdownToPlainText(fixture.markdown).replace(/\s+/g, " "),
      `${fixture.name} plain-text round-trip`,
    );
    assert.ok(again.length > 0, `${fixture.name} html after round-trip`);
  }
}

const entityHtml = markdownToHtml("See [[Missing]](gone-id) and [[Ok]](ok-id).\n");
contains(entityHtml, 'data-entity-id="gone-id"', "missing entity");
assert.deepEqual(
  extractEntityReferences("See [[Missing]](gone-id) and [[Ok]](ok-id).\n").map((item) => item.entityId),
  ["gone-id", "ok-id"],
);

const tree = parseMarkdown("[[Label]](abc)");
assert.equal(tree.children[0].type, "paragraph");
assert.equal(tree.children[0].children[0].type, "entityReference");
assert.equal(tree.children[0].children[0].entityId, "abc");

const xssHtml = markdownToHtml("<script>alert(1)</script>\n");
assert.equal(xssHtml.toLowerCase().includes("<script"), false);
assert.equal(xssHtml.toLowerCase().includes("javascript:"), false);

const unsafeLink = markdownToHtml("[unsafe](javascript:alert(1))\n");
assert.equal(unsafeLink.toLowerCase().includes("javascript:"), false);

const protocolEntity = markdownToHtml("[Ardashir](daena://entity/entity-ardashir)\n");
contains(protocolEntity, 'data-entity-id="entity-ardashir"', "protocol entity");
contains(htmlToMarkdown(protocolEntity), "[[Ardashir]](entity-ardashir)", "stringify entity wiki form");

const tiptapEntity = htmlToMarkdown('<p><a class="entity-reference" data-entity-id="abc">Ardashir</a></p>');
contains(tiptapEntity, "[[Ardashir]](abc)", "tiptap entity without href");

const tiptapAlign = htmlToMarkdown('<p style="text-align: center"><u>Hi</u></p>');
contains(tiptapAlign, 'align="center"', "tiptap alignment");
contains(tiptapAlign, "<u>Hi</u>", "tiptap underline");

const tiptapColor = htmlToMarkdown('<p>A <span style="color: #B4773F">gold</span> word.</p>');
contains(tiptapColor, 'style="color: #b4773f"', "tiptap text color");
contains(tiptapColor, "gold", "tiptap text color text");

const unsafeColor = markdownToHtml(
  '<span style="color: expression(alert(1)); background: url(javascript:x)">x</span>\n',
);
assert.equal(unsafeColor.toLowerCase().includes("expression"), false);
assert.equal(unsafeColor.toLowerCase().includes("javascript"), false);
assert.equal(unsafeColor.toLowerCase().includes("background"), false);

const namedColor = markdownToHtml('<span style="color: red">x</span>\n');
assert.equal(namedColor.toLowerCase().includes("color: red"), false);

const tiptapCode = htmlToMarkdown('<pre><code data-language="ts">const x = 1;</code></pre>');
contains(tiptapCode, "```ts", "tiptap data-language fence");

assert.equal(htmlToMarkdown('<p><a href="javascript:alert(1)">x</a></p>').toLowerCase().includes("javascript:"), false);
assert.equal(
  htmlToMarkdown('<p><img src="javascript:alert(1)" alt="x"></p>').toLowerCase().includes("javascript:"),
  false,
);
assert.equal(markdownToHtml("![x](javascript:alert(1))\n").toLowerCase().includes("javascript:"), false);

const entityClass = markdownToHtml("[[Ardashir]](abc)\n");
contains(entityClass, "entity-reference", "entity class");

assert.equal(htmlToMarkdown("<p>hello</p>"), "hello\n", "plain text is unchanged");
assert.equal(htmlToMarkdown("<p>hello</p><p>world</p>"), "hello\n\nworld\n", "adjacent paragraphs stay markdown");
assert.equal(
  htmlToMarkdown("<p>hello</p><p>&nbsp;</p><p>world</p>"),
  "hello\n\n \n\nworld\n",
  "nbsp paragraphs stay as text",
);

assert.equal(htmlToMarkdown("<p></p>"), "", "sole empty paragraph stays empty document");
assert.equal(htmlToMarkdown("<p><br></p>"), "", "sole break paragraph stays empty document");

const trailingBlank = htmlToMarkdown("<p>hello</p><p></p>");
assert.equal(trailingBlank, "hello\n\n<p></p>\n", "trailing empty paragraph is stored");
assert.equal(htmlToMarkdown(markdownToHtml(trailingBlank)), trailingBlank, "trailing empty paragraph round-trips");

const breakBlank = htmlToMarkdown("<p>hello</p><p><br></p>");
assert.equal(breakBlank, "hello\n\n<p></p>\n", "trailing break paragraph is stored");
assert.equal(htmlToMarkdown(markdownToHtml(breakBlank)), breakBlank, "trailing break paragraph round-trips");

const middleBlank = htmlToMarkdown("<p>hello</p><p></p><p>world</p>");
assert.equal(middleBlank, "hello\n\n<p></p>\n\nworld\n", "middle empty paragraph is stored");
assert.equal(htmlToMarkdown(markdownToHtml(middleBlank)), middleBlank, "middle empty paragraph round-trips");

const leadingBlank = htmlToMarkdown("<p></p><p>hello</p>");
assert.equal(leadingBlank, "<p></p>\n\nhello\n", "leading empty paragraph is stored");
assert.equal(htmlToMarkdown(markdownToHtml(leadingBlank)), leadingBlank, "leading empty paragraph round-trips");

console.log(`markdown fixtures passed (${MARKDOWN_FIXTURES.length} documents + entity/XSS/round-trip checks)`);
