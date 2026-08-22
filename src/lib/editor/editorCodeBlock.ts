import CodeBlock from "@tiptap/extension-code-block";
import { mergeAttributes } from "@tiptap/core";

function languageFromElement(element: HTMLElement): string | null {
  const code = element.matches("code") ? element : element.querySelector("code");
  const fromData = code?.getAttribute("data-language")?.trim();
  if (fromData) return fromData;
  const fromClass = [...(code?.classList ?? [])]
    .find((className) => className.startsWith("language-"))
    ?.slice("language-".length)
    .trim();
  return fromClass || null;
}

export const LanguageCodeBlock = CodeBlock.extend({
  addAttributes() {
    return {
      ...this.parent?.(),
      language: {
        default: null,
        parseHTML: languageFromElement,
        rendered: false,
      },
    };
  },
  renderHTML({ node, HTMLAttributes }) {
    const language = typeof node.attrs.language === "string" ? node.attrs.language.trim() : "";
    const codeAttributes = language ? { "data-language": language, class: `language-${language}` } : {};
    return ["pre", mergeAttributes(this.options.HTMLAttributes, HTMLAttributes), ["code", codeAttributes, 0]];
  },
});
