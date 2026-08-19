import type { Handle as ToMarkdownHandle } from "mdast-util-to-markdown";
import type { AlignedParagraph, Underline } from "../types.ts";

export const underlineToMarkdown: ToMarkdownHandle = (node, _parent, state, info) => {
  return `<u>${state.containerPhrasing(node as never, info)}</u>`;
};

export const alignedParagraphToMarkdown: ToMarkdownHandle = (node, _parent, state, info) => {
  const aligned = node as AlignedParagraph;
  return `<div align="${aligned.align}">${state.containerPhrasing(aligned as never, info)}</div>`;
};
