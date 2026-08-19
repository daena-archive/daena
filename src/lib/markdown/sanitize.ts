import type { Root as HastRoot } from "hast";
import rehypeSanitize, { defaultSchema } from "rehype-sanitize";
import { visit } from "unist-util-visit";

export const daenaSanitizeSchema = {
  ...defaultSchema,
  tagNames: [...(defaultSchema.tagNames ?? []), "u"],
  attributes: {
    ...defaultSchema.attributes,
    a: [...(defaultSchema.attributes?.a ?? []), "dataEntityId", "className"],
    code: [...(defaultSchema.attributes?.code ?? []), "className", "dataLanguage"],
    img: [...(defaultSchema.attributes?.img ?? []), "src", "alt", "title"],
    p: [...(defaultSchema.attributes?.p ?? []), "style"],
    h1: [...(defaultSchema.attributes?.h1 ?? []), "style"],
    h2: [...(defaultSchema.attributes?.h2 ?? []), "style"],
    h3: [...(defaultSchema.attributes?.h3 ?? []), "style"],
    h4: [...(defaultSchema.attributes?.h4 ?? []), "style"],
    h5: [...(defaultSchema.attributes?.h5 ?? []), "style"],
    h6: [...(defaultSchema.attributes?.h6 ?? []), "style"],
    pre: [...(defaultSchema.attributes?.pre ?? []), "dataLanguage"],
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
