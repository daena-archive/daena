import { htmlToMarkdown, parseMarkdown } from "../markdown/index.ts";

type TextNode = {
  type?: string;
  value?: unknown;
  alt?: unknown;
  children?: unknown[];
};

const separatedNodeTypes = new Set(["root", "blockquote", "list", "listItem", "table", "tableRow"]);
const htmlTagPattern = /<\/?[A-Za-z][^>]*>/;

function readableNodeText(node: TextNode): string {
  if (typeof node.value === "string") return node.value;
  if (typeof node.alt === "string") return node.alt;
  if (node.type === "break") return " ";
  if (!Array.isArray(node.children)) return "";

  const separator = separatedNodeTypes.has(node.type ?? "") ? " " : "";
  return node.children
    .map((child) => readableNodeText(child as TextNode))
    .filter(Boolean)
    .join(separator);
}

export function documentDiffText(value: string): string {
  const markdown = htmlTagPattern.test(value) ? htmlToMarkdown(value) : value;
  return readableNodeText(parseMarkdown(markdown)).replace(/\s+/g, " ").trim();
}

export function formatDiffLineForDisplay(path: string | null, line: string): string {
  if (!path || (path !== "document.md" && !path.endsWith("/document.md"))) return line;
  if (line.startsWith("@@") || line.startsWith("\\ No newline at end of file")) return line;

  const prefix = line.startsWith("+") || line.startsWith("-") || line.startsWith(" ") ? line[0] : "";
  const content = prefix ? line.slice(1) : line;
  return `${prefix}${documentDiffText(content)}`;
}
