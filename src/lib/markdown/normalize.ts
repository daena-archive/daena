import { htmlToMarkdown } from "$lib/markdown";

export function normalizeDocument(body: string, format?: string) {
  if (format === "rich-text") return htmlToMarkdown(body);
  return body;
}
