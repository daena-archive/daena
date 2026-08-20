import type { Root as HastRoot } from "hast";
import rehypeSanitize, { defaultSchema } from "rehype-sanitize";
import { visit } from "unist-util-visit";

export const daenaSanitizeSchema = {
  ...defaultSchema,
  tagNames: [
    ...(defaultSchema.tagNames ?? []),
    "u",
    "mark",
    "span",
    "table",
    "thead",
    "tbody",
    "tr",
    "th",
    "td",
    "details",
    "summary",
  ],
  attributes: {
    ...defaultSchema.attributes,
    a: [...(defaultSchema.attributes?.a ?? []), "dataEntityId", "className"],
    code: [...(defaultSchema.attributes?.code ?? []), "className", "dataLanguage"],
    img: [...(defaultSchema.attributes?.img ?? []), "src", "alt", "title"],
    p: [...(defaultSchema.attributes?.p ?? []), "style", "dir"],
    h1: [...(defaultSchema.attributes?.h1 ?? []), "style", "dir"],
    h2: [...(defaultSchema.attributes?.h2 ?? []), "style", "dir"],
    h3: [...(defaultSchema.attributes?.h3 ?? []), "style", "dir"],
    h4: [...(defaultSchema.attributes?.h4 ?? []), "style", "dir"],
    h5: [...(defaultSchema.attributes?.h5 ?? []), "style", "dir"],
    h6: [...(defaultSchema.attributes?.h6 ?? []), "style", "dir"],
    pre: [...(defaultSchema.attributes?.pre ?? []), "dataLanguage"],
    mark: [...(defaultSchema.attributes?.mark ?? []), "className"],
    span: [...(defaultSchema.attributes?.span ?? []), "className", "dataSpoiler"],
    sub: [...(defaultSchema.attributes?.sub ?? []), "className"],
    sup: [...(defaultSchema.attributes?.sup ?? []), "className"],
    table: [...(defaultSchema.attributes?.table ?? []), "className"],
    th: [...(defaultSchema.attributes?.th ?? []), "colspan", "rowspan", "style", "className"],
    td: [...(defaultSchema.attributes?.td ?? []), "colspan", "rowspan", "style", "className"],
    tr: [...(defaultSchema.attributes?.tr ?? []), "className"],
    ul: [...(defaultSchema.attributes?.ul ?? []), "dataType", "className"],
    li: [...(defaultSchema.attributes?.li ?? []), "dataChecked", "dataType", "className"],
    details: [...(defaultSchema.attributes?.details ?? []), "open", "className"],
    summary: [...(defaultSchema.attributes?.summary ?? []), "className"],
  },
  protocols: {
    ...defaultSchema.protocols,
    href: [...(defaultSchema.protocols?.href ?? []), "daena"],
  },
};

export function rehypeSafeInlineStyle() {
  return (tree: HastRoot) => {
    visit(tree, "element", (node) => {
      if (!node.properties || node.properties.style == null) return;
      const style = String(node.properties.style).trim();
      if (!/^text-align\s*:\s*(?:left|center|right)\s*;?$/i.test(style)) delete node.properties.style;
    });
  };
}

export function rehypeNormalizeCodeLanguage() {
  return (tree: HastRoot) => {
    visit(tree, "element", (node) => {
      if (node.tagName !== "code" || !node.properties) return;
      const className = node.properties.className;
      const classes = Array.isArray(className) ? className.map(String) : className ? [String(className)] : [];
      const fromClass = classes.find((value) => value.startsWith("language-"))?.slice("language-".length);
      const fromData = node.properties.dataLanguage == null ? "" : String(node.properties.dataLanguage);
      const language = fromClass || fromData;
      if (!language) return;
      node.properties.dataLanguage = language;
      node.properties.className = [
        ...classes.filter((value) => !value.startsWith("language-")),
        `language-${language}`,
      ];
    });
  };
}

export function rehypeEntityReferenceClass() {
  return (tree: HastRoot) => {
    visit(tree, "element", (node) => {
      if (node.tagName !== "a" || !node.properties || node.properties.dataEntityId == null) return;
      const className = node.properties.className;
      const classes = Array.isArray(className) ? className.map(String) : className ? [String(className)] : [];
      if (!classes.includes("entity-reference")) classes.push("entity-reference");
      node.properties.className = classes.filter(Boolean);
    });
  };
}

export { rehypeSanitize };
