import type { Element } from "hast";
import type { Link, PhrasingContent } from "mdast";
import { defaultHandlers } from "hast-util-to-mdast";
import type { State as HastState } from "hast-util-to-mdast";
import { entityIdFromHref } from "./urls.ts";
import { nodeText } from "./text.ts";
import type { AlignedParagraph, EntityReference, Spoiler, Underline } from "./types.ts";

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

function textDir(node: Element): "ltr" | "rtl" | "" {
  const dir = String(node.properties?.dir ?? "").toLowerCase();
  if (dir === "ltr" || dir === "rtl") return dir;
  return "";
}

function propertyString(node: Element, key: string): string {
  const value = node.properties?.[key];
  return value == null ? "" : String(value);
}

const alignedBlock = (state: HastState, node: Element) => {
  const align = textAlign(node);
  const dir = textDir(node);
  const isHeading = /^h[1-6]$/.test(node.tagName);
  if (align) {
    if (isHeading) {
      const fallback = defaultHandlers[node.tagName as keyof typeof defaultHandlers];
      const mdNode = fallback ? (fallback(state, node) as unknown as Record<string, unknown>) : undefined;
      if (mdNode && typeof mdNode === "object") {
        const data = (mdNode.data as Record<string, unknown> | undefined) ?? {};
        const hProps = (data.hProperties as Record<string, unknown> | undefined) ?? {};
        const newHProps: Record<string, unknown> = { ...hProps, style: `text-align: ${align}` };
        if (dir) newHProps.dir = dir;
        (mdNode as Record<string, unknown>).data = { ...data, hProperties: newHProps };
        if (dir) (mdNode as Record<string, unknown>).dir = dir;
        // keep align for rendering symmetry, though heading uses style
        (mdNode as Record<string, unknown>).align = align;
        state.patch(node, mdNode as never);
        return mdNode;
      }
    } else {
      const hProperties: Record<string, unknown> = { style: `text-align: ${align}` };
      if (dir) hProperties.dir = dir;
      const result: AlignedParagraph = {
        type: "alignedParagraph",
        align,
        ...(dir ? { dir } : {}),
        children: state.all(node) as PhrasingContent[],
        data: { hName: "p", hProperties },
      };
      state.patch(node, result as never);
      return result;
    }
  }
  if (dir) {
    const fallback = defaultHandlers[node.tagName as keyof typeof defaultHandlers];
    const mdNode = fallback ? (fallback(state, node) as unknown as Record<string, unknown>) : undefined;
    if (mdNode && typeof mdNode === "object") {
      const data = (mdNode.data as Record<string, unknown> | undefined) ?? {};
      const hProps = (data.hProperties as Record<string, unknown> | undefined) ?? {};
      (mdNode as Record<string, unknown>).data = {
        ...data,
        hProperties: { ...hProps, dir },
      };
      // expose dir directly for easier stringify / rendering
      (mdNode as Record<string, unknown>).dir = dir;
      state.patch(node, mdNode as never);
      return mdNode;
    }
  }
  const fallback = defaultHandlers[node.tagName as keyof typeof defaultHandlers];
  return fallback ? fallback(state, node) : undefined;
};

const entityOrLink = (state: HastState, node: Element) => {
  const entityId = propertyString(node, "dataEntityId") || entityIdFromHref(propertyString(node, "href"));
  if (entityId) {
    const children = state.all(node) as PhrasingContent[];
    const rawIsCustom =
      (node.properties as Record<string, unknown> | undefined)?.["dataIsCustom"] ??
      (node.properties as Record<string, unknown> | undefined)?.["data-is-custom"];
    const hasFlag = rawIsCustom != null;
    const isCustom = hasFlag
      ? String(rawIsCustom) === "true"
      : children.length > 0 && nodeText({ children } as never).trim().length > 0;
    const result: EntityReference = {
      type: "entityReference",
      entityId,
      isCustom,
      children,
      data: {
        hName: "a",
        hProperties: {
          href: `daena://entity/${encodeURIComponent(entityId)}`,
          dataEntityId: entityId,
          ...(isCustom ? { dataIsCustom: "true" } : {}),
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

const spoiler = (state: HastState, node: Element) => {
  const isSpoiler = node.properties?.dataSpoiler != null || classList(node).includes("spoiler");
  if (!isSpoiler) {
    const fallback = (defaultHandlers as Record<string, unknown>).span as
      ((state: HastState, node: Element) => unknown) | undefined;
    return fallback ? fallback(state, node) : undefined;
  }
  const result: Spoiler = {
    type: "spoiler",
    children: state.all(node) as Spoiler["children"],
    data: { hName: "span", hProperties: { dataSpoiler: "", className: ["spoiler"] } },
  };
  state.patch(node, result as never);
  return result;
};

const imageNode = (state: HastState, node: Element) => {
  const src = propertyString(node, "src");
  const alt = propertyString(node, "alt");
  const title = propertyString(node, "title");
  const widthRaw = propertyString(node, "width");
  const heightRaw = propertyString(node, "height");
  const w = /^\d+$/.test(widthRaw) ? widthRaw : "";
  const h = /^\d+$/.test(heightRaw) ? heightRaw : "";
  const hProperties: Record<string, unknown> = { src, alt };
  if (title) hProperties.title = title;
  if (w) hProperties.width = w;
  if (h) hProperties.height = h;
  const data: Record<string, unknown> = { hName: "img", hProperties };
  const result: Record<string, unknown> = {
    type: "image",
    url: src,
    alt,
    title: title || null,
    data,
  };
  if (w) result.width = w;
  if (h) result.height = h;
  state.patch(node, result as never);
  return result;
};

export const hastToMdastHandlers = {
  img: imageNode,
  a: entityOrLink,
  u: underline,
  span: spoiler,
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
