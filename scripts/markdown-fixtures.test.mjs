import assert from "node:assert/strict";
import { MARKDOWN_FIXTURES } from "../src/lib/editor/markdown.fixtures.ts";
import { markdownToHtml } from "../src/lib/editor/markdown.ts";

for (const fixture of MARKDOWN_FIXTURES) {
  assert.equal(markdownToHtml(fixture.markdown), fixture.html, fixture.name);
}

assert.equal(markdownToHtml("<script>alert(1)</script>\n"), "<p>&lt;script&gt;alert(1)&lt;/script&gt;</p>");
assert.equal(markdownToHtml("[unsafe](javascript:alert(1))\n"), "<p>unsafe</p>");
console.log(`markdown fixtures passed (${MARKDOWN_FIXTURES.length} round-trip inputs + sanitization checks)`);
