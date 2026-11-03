import type { Editor } from "@tiptap/core";
import { DOMSerializer } from "@tiptap/pm/model";
import { denormalizeAssetHtml } from "$lib/assets/resolve";
import { htmlToMarkdown } from "$lib/markdown";
import { sanitizeInlineStyle } from "$lib/markdown/color.ts";
import { taskListsForMarkdown } from "./markdownRoundTrip";

export function sanitizeHtml(value: string): string {
  if (typeof document === "undefined") return value;
  const template = document.createElement("template");
  template.innerHTML = value;
  for (const node of template.content.querySelectorAll("script, style, iframe, object, embed, form")) node.remove();
  for (const element of template.content.querySelectorAll("*")) {
    for (const attribute of [...element.attributes]) {
      const name = attribute.name.toLowerCase();
      const content = attribute.value.trim().toLowerCase();
      if (name.startsWith("on") || ((name === "href" || name === "src") && content.startsWith("javascript:")))
        element.removeAttribute(attribute.name);
      if (name === "style") {
        const style = sanitizeInlineStyle(attribute.value);
        if (style) element.setAttribute("style", style);
        else element.removeAttribute(attribute.name);
      }
    }
  }
  return template.innerHTML;
}

/**
 * The editor's content as HTML, like `editor.getHTML()`, but serialized into an inert document.
 * `getHTML()` builds the nodes in the live document first, and a browser starts fetching an `<img>`
 * as soon as it is created there, so every export requested `assets/...` paths from the page origin.
 */
export function editorHtml(editor: Editor): string {
  const inert = document.implementation.createHTMLDocument("");
  const fragment = DOMSerializer.fromSchema(editor.schema).serializeFragment(editor.state.doc.content, {
    document: inert,
  });
  const container = inert.createElement("div");
  container.appendChild(fragment);
  return container.innerHTML;
}

export function markdownFromEditorHtml(html: string) {
  return htmlToMarkdown(taskListsForMarkdown(denormalizeAssetHtml(html)));
}

export function editorPlainText(currentEditor: Editor) {
  return currentEditor.state.doc.textBetween(0, currentEditor.state.doc.content.size, "\n");
}
