import type { Element } from "hast";
import type { Link, PhrasingContent } from "mdast";
import { defaultHandlers } from "hast-util-to-mdast";
import type { State as HastState } from "hast-util-to-mdast";
import { entityIdFromHref } from "./urls.ts";
import type { AlignedParagraph, EntityReference, Underline } from "./types.ts";

function classList(node: Element): string[] {
  const className = node.properties?.className;
  if (Array.isArray(className)) return className.map(String);
  if (className) return [String(className)];
  return [];
}

function textAlign(node: Element): "center" | "right" | "" {
  const style = String(node.properties?.style ?? "");
  const match = style.match(/text-align\s*:\s*(left|center|right)/i);
  const align = match?.[1]?.toLowerCase();
  if (align === "center" || align === "right") return align;
  const attr = String(node.properties?.align ?? "").toLowerCase();
  return attr === "center" || attr === "right" ? attr : "";
}

function propertyString(node: Element, key: string): string {
  const value = node.properties?.[key];
  return value == null ? "" : String(value);
}

const alignedBlock = (state: HastState, node: Element) => {
  const align = textAlign(node);
  if (align) {
    const result: AlignedParagraph = {
      type: "alignedParagraph",
      align,
      children: state.all(node) as PhrasingContent[],
      data: { hName: "p", hProperties: { style: `text-align: ${align}` } },
    };
    state.patch(node, result as never);
    return result;
  }
  const fallback = defaultHandlers[node.tagName as keyof typeof defaultHandlers];
  return fallback ? fallback(state, node) : undefined;
};

const entityOrLink = (state: HastState, node: Element) => {
  const entityId = propertyString(node, "dataEntityId") || entityIdFromHref(propertyString(node, "href"));
  if (entityId) {
    const result: EntityReference = {
      type: "entityReference",
      entityId,
      children: state.all(node) as PhrasingContent[],
      data: {
        hName: "a",
        hProperties: {
          href: `daena://entity/${encodeURIComponent(entityId)}`,
          dataEntityId: entityId,
          className: ["entity-reference"],
        },
      },
    };
    state.patch(node, result as never);
    return result;
  }
  return defaultHandlers.a(state, node) as Link;
};

const underline = (state: HastState, node: Element) => {
  const result: Underline = {
    type: "underline",
    children: state.all(node) as Underline["children"],
    data: { hName: "u" },
  };
  state.patch(node, result as never);
  return result;
};

export const hastToMdastHandlers = {
  a: entityOrLink,
  u: underline,
  p: alignedBlock,
  div: alignedBlock,
  h1: alignedBlock,
  h2: alignedBlock,
  h3: alignedBlock,
  h4: alignedBlock,
  h5: alignedBlock,
  h6: alignedBlock,
  pre(state: HastState, node: Element) {
    const code = node.children.find((child): child is Element => child.type === "element" && child.tagName === "code");
    if (code) {
      const language =
        propertyString(code, "dataLanguage") ||
        classList(code)
          .find((value) => value.startsWith("language-"))
          ?.slice(9);
      if (language) {
        const classes = classList(code).filter((value) => !value.startsWith("language-"));
        code.properties = {
          ...code.properties,
          dataLanguage: language,
          className: [...classes, `language-${language}`],
        };
      }
    }
    return defaultHandlers.pre(state, node);
  },
};
