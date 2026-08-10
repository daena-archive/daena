/** Small deterministic fixtures for the supported editor subset. */
export const MARKDOWN_FIXTURES = [
  {
    name: "rich-inline-and-block-formatting",
    markdown: "# A title\n\nA **bold** and *quiet* [link](https://example.com).\n\n- one\n- two\n",
    html: '<h1>A title</h1><p>A <strong>bold</strong> and <em>quiet</em> <a href="https://example.com">link</a>.</p><ul><li>one</li><li>two</li></ul>',
  },
  {
    name: "code-and-quote",
    markdown: "> Keep the draft safe.\n\n```ts\nconst answer = 42;\n```\n",
    html: '<blockquote><p>Keep the draft safe.</p></blockquote><pre><code data-language="ts">const answer = 42;</code></pre>',
  },
  {
    name: "underline-and-alignment",
    markdown: '<div align="center">A <u>centered</u> note.</div>\n',
    html: '<p style="text-align: center">A <u>centered</u> note.</p>',
  },
] as const;
