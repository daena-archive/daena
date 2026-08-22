/** Semantic fixtures for the editor Markdown subset. */
export const MARKDOWN_FIXTURES = [
  {
    name: "rich-inline-and-block-formatting",
    markdown: "# A title\n\nA **bold** and *quiet* [link](https://example.com).\n\n- one\n- two\n",
  },
  {
    name: "code-and-quote",
    markdown: "> Keep the draft safe.\n\n```ts\nconst answer = 42;\n```\n",
  },
  {
    name: "underline-and-alignment",
    markdown: '<div align="center">A <u>centered</u> note.</div>\n',
  },
  {
    name: "entity-reference",
    markdown: "Meet [[Ardashir]](entity-ardashir) in the archive.\n",
  },
  {
    name: "strike-hr-ordered-nested",
    markdown: "This is ~~gone~~.\n\n---\n\n1. one\n2. two\n\n- parent\n  - nested\n",
  },
  {
    name: "inline-code-and-image",
    markdown: "Use `code` and ![cat](https://example.com/cat.png).\n",
  },
  {
    name: "unicode",
    markdown: "اردشیر and naïve café.\n",
  },
  {
    name: "empty",
    markdown: "",
  },
  {
    name: "image-with-dimensions",
    markdown: '<img src="assets/a.png" alt="x" width="400" height="264">\n',
  },
  {
    name: "image-auto",
    markdown: "![auto](https://example.com/auto.png)\n",
  },
  {
    name: "image-align",
    markdown: '<p style="text-align: center"><img src="assets/b.png" alt="centered" width="320"></p>\n',
  },
] as const;
