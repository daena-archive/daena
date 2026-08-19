export function nodeText(node: { type?: string; value?: unknown; children?: unknown[] }): string {
  if (typeof node.value === "string") return node.value;
  if (!Array.isArray(node.children)) return "";
  return node.children
    .map((child) => nodeText(child as { type?: string; value?: unknown; children?: unknown[] }))
    .join("");
}

export function slugify(value: string): string {
  const slug = value
    .toLowerCase()
    .trim()
    .replace(/[^\p{L}\p{N}]+/gu, "-")
    .replace(/^-+|-+$/g, "");
  return slug || "heading";
}
