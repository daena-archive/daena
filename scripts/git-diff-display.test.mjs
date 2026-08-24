import assert from "node:assert/strict";

import { documentDiffText, formatDiffLineForDisplay } from "../src/lib/git/diff-display.ts";

assert.equal(
  formatDiffLineForDisplay(
    "entities/018f89ec-25fc-7816-8b47-6f80905f2868/document.md",
    '+<p class="editor-paragraph" data-align="left">The <strong>sea</strong> remembers &amp; waits.</p>',
  ),
  "+The sea remembers & waits.",
);
assert.equal(
  formatDiffLineForDisplay(
    "entities/018f89ec-25fc-7816-8b47-6f80905f2868/document.md",
    '-<p>First kingdom.</p><p data-text-direction="rtl">Second kingdom.</p>',
  ),
  "-First kingdom. Second kingdom.",
);
assert.equal(formatDiffLineForDisplay("entities/id/document.md", " ## A [linked place](place-id)"), " A linked place");
assert.equal(formatDiffLineForDisplay("entities/id/document.md", "@@ -1,2 +1,2 @@"), "@@ -1,2 +1,2 @@");
assert.equal(
  formatDiffLineForDisplay("plugins/example/view.html", '+<div class="panel">Stored HTML</div>'),
  '+<div class="panel">Stored HTML</div>',
  "non-document source diffs must remain exact",
);
assert.equal(documentDiffText("## Heading\n\n- One\n- Two"), "Heading One Two");

console.log("author-facing document diff checks passed");
