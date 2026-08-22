import type { Root as HastRoot } from "hast";
import type { Root } from "mdast";
import rehypeParse from "rehype-parse";
import rehypeRaw from "rehype-raw";
import rehypeRemark from "rehype-remark";
import rehypeStringify from "rehype-stringify";
import remarkGfm from "remark-gfm";
import remarkParse from "remark-parse";
import remarkRehype from "remark-rehype";
import remarkStringify from "remark-stringify";
import { unified } from "unified";
import { visit } from "unist-util-visit";
import { hastToMdastHandlers } from "./from-html.ts";
import { remarkDaenaAssets } from "./plugins/assets.ts";
import { entityReferenceToMarkdown, remarkDaenaEntities } from "./plugins/entities.ts";
import {
  alignedParagraphToMarkdown,
  headingToMarkdown,
  imageToMarkdown,
  paragraphToMarkdown,
  spoilerToMarkdown,
  underlineToMarkdown,
} from "./plugins/html.ts";
import {
  daenaSanitizeSchema,
  rehypeEntityReferenceClass,
  rehypeNormalizeCodeLanguage,
  rehypeSafeInlineStyle,
  rehypeSanitize,
} from "./sanitize.ts";
import { nodeText, slugify } from "./text.ts";
import type { EntityReferenceInfo, HeadingOutlineItem } from "./types.ts";

const stringifyHandlers = {
  entityReference: entityReferenceToMarkdown,
  underline: underlineToMarkdown,
  spoiler: spoilerToMarkdown,
  alignedParagraph: alignedParagraphToMarkdown,
  paragraph: paragraphToMarkdown,
  heading: headingToMarkdown,
  image: imageToMarkdown,
};

function rehypeAlignedDivs() {
  return (tree: HastRoot) => {
    visit(tree, "element", (node) => {
      if (node.tagName !== "div") return;
      const align = String(node.properties?.align ?? "").toLowerCase();
      if (align !== "left" && align !== "center" && align !== "right") return;
      node.tagName = "p";
      node.properties = { ...node.properties, style: `text-align: ${align}` };
      if (node.properties) delete node.properties.align;
    });
  };
}

const toHast = unified()
  .use(remarkParse)
  .use(remarkGfm)
  .use(remarkDaenaEntities)
  .use(remarkDaenaAssets)
  .use(remarkRehype, { allowDangerousHtml: true })
  .use(rehypeRaw)
  .use(rehypeAlignedDivs)
  .use(rehypeNormalizeCodeLanguage);

const htmlProcessor = unified()
  .use(remarkParse)
  .use(remarkGfm)
  .use(remarkDaenaEntities)
  .use(remarkDaenaAssets)
  .use(remarkRehype, { allowDangerousHtml: true })
  .use(rehypeRaw)
  .use(rehypeAlignedDivs)
  .use(rehypeNormalizeCodeLanguage)
  .use(rehypeSanitize, daenaSanitizeSchema as never)
  .use(rehypeSafeInlineStyle)
  .use(rehypeEntityReferenceClass)
  .use(rehypeStringify);

const fromHast = unified().use(rehypeRemark, { handlers: hastToMdastHandlers as never });

const markdownProcessor = unified()
  .use(rehypeParse, { fragment: true })
  .use(rehypeNormalizeCodeLanguage)
  .use(rehypeSanitize, daenaSanitizeSchema as never)
  .use(rehypeSafeInlineStyle)
  .use(rehypeRemark, { handlers: hastToMdastHandlers as never })
  .use(remarkGfm)
  .use(remarkStringify, {
    bullet: "-",
    emphasis: "*",
    strong: "*",
    fences: true,
    handlers: stringifyHandlers,
  } as never);

function applyHeadingIds(tree: Root): void {
  const seen = new Map<string, number>();
  visit(tree, "heading", (node) => {
    const base = slugify(nodeText(node));
    const count = seen.get(base) ?? 0;
    seen.set(base, count + 1);
    const id = count ? `${base}-${count}` : base;
    node.data = {
      ...node.data,
      hProperties: { ...(node.data?.hProperties ?? {}), id },
    };
  });
}

export function parseMarkdown(markdown: string): Root {
  const hast = toHast.runSync(toHast.parse(markdown.replace(/\r\n?/g, "\n"))) as HastRoot;
  const tree = fromHast.runSync(hast) as Root;
  applyHeadingIds(tree);
  return tree;
}

export function markdownToHtml(markdown: string): string {
  return String(htmlProcessor.processSync(markdown.replace(/\r\n?/g, "\n")));
}

export function htmlToMarkdown(html: string): string {
  const markdown = String(markdownProcessor.processSync(html))
    .replace(/\n{3,}/g, "\n\n")
    .trim();
  return markdown ? `${markdown}\n` : "";
}

export function extractEntityReferences(markdown: string): EntityReferenceInfo[] {
  const references: EntityReferenceInfo[] = [];
  visit(parseMarkdown(markdown), "entityReference", (node) => {
    const reference = node as { entityId: string };
    references.push({ entityId: reference.entityId, label: nodeText(node) });
  });
  return references;
}

export function headingOutlineFromTree(tree: Root): HeadingOutlineItem[] {
  const items: HeadingOutlineItem[] = [];
  visit(tree, "heading", (node) => {
    const text = nodeText(node);
    const id = String((node.data?.hProperties as { id?: string } | undefined)?.id ?? slugify(text));
    items.push({ depth: node.depth, text, id });
  });
  return items;
}

export function headingOutline(markdown: string): HeadingOutlineItem[] {
  return headingOutlineFromTree(parseMarkdown(markdown));
}

export function markdownToPlainText(markdown: string): string {
  return nodeText(parseMarkdown(markdown)).replace(/\s+/g, " ").trim();
}

export type { Root };
export type { EntityReferenceInfo, HeadingOutlineItem } from "./types.ts";
