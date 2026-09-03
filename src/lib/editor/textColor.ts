import { Mark } from "@tiptap/core";
import { colorFromStyle, normalizeHexColor } from "$lib/markdown/color.ts";

export const DEFAULT_TEXT_COLOR = "#25251f";

export const TextColor = Mark.create({
  name: "textColor",
  addAttributes() {
    return {
      color: {
        default: null,
        parseHTML: (element) => colorFromStyle(element.getAttribute("style") ?? ""),
        renderHTML: (attributes) => {
          const color = normalizeHexColor(String(attributes.color ?? ""));
          return color ? { style: `color: ${color}` } : {};
        },
      },
    };
  },
  parseHTML() {
    return [
      {
        tag: "span",
        getAttrs: (element) => {
          if (!(element instanceof HTMLElement)) return false;
          if (element.hasAttribute("data-spoiler") || element.classList.contains("spoiler")) return false;
          const color = colorFromStyle(element.getAttribute("style") ?? "");
          return color ? { color } : false;
        },
      },
    ];
  },
  renderHTML({ HTMLAttributes }) {
    return ["span", HTMLAttributes, 0];
  },
});
