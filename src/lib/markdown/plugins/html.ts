import type { Handle as ToMarkdownHandle } from "mdast-util-to-markdown";
import type { AlignedParagraph, Spoiler, Underline } from "../types.ts";

export const underlineToMarkdown: ToMarkdownHandle = (node, _parent, state, info) => {
  return `<u>${state.containerPhrasing(node as never, info)}</u>`;
};

export const spoilerToMarkdown: ToMarkdownHandle = (node, _parent, state, info) => {
  return `<span data-spoiler>${state.containerPhrasing(node as never, info)}</span>`;
};

function escapeHtml(value: string): string {
  return value.replaceAll("&", "&amp;").replaceAll("<", "&lt;").replaceAll(">", "&gt;").replaceAll('"', "&quot;");
}

function phrasingToHtml(nodes: unknown[], state: unknown, info: unknown): string {
  // Fallback helper for dir-wrapped blocks: serialize inline mdast as HTML so that
  // raw HTML wrapper <p dir> / <hN dir> can be reparsed without losing inline marks.
  const s = state as { containerPhrasing: (node: unknown, info: unknown) => string };
  let out = "";
  for (const raw of nodes as Array<Record<string, unknown>>) {
    const type = raw.type as string;
    const children = (raw.children as unknown[]) ?? [];
    switch (type) {
      case "text":
        out += escapeHtml(String(raw.value ?? ""));
        break;
      case "strong":
        out += `<strong>${phrasingToHtml(children, state, info)}</strong>`;
        break;
      case "emphasis":
        out += `<em>${phrasingToHtml(children, state, info)}</em>`;
        break;
      case "delete":
        out += `<s>${phrasingToHtml(children, state, info)}</s>`;
        break;
      case "underline":
        out += `<u>${phrasingToHtml(children, state, info)}</u>`;
        break;
      case "spoiler":
        out += `<span data-spoiler class="spoiler">${phrasingToHtml(children, state, info)}</span>`;
        break;
      case "inlineCode":
        out += `<code>${escapeHtml(String(raw.value ?? ""))}</code>`;
        break;
      case "break":
        out += "<br>";
        break;
      case "link": {
        const url = String((raw as { url?: string }).url ?? "");
        const href = escapeHtml(url);
        out += `<a href="${href}">${phrasingToHtml(children, state, info)}</a>`;
        break;
      }
      case "entityReference": {
        const id = String((raw as { entityId?: string }).entityId ?? "");
        out += `<a data-entity-id="${escapeHtml(id)}" class="entity-reference">${phrasingToHtml(children, state, info)}</a>`;
        break;
      }
      case "image": {
        const url = String((raw as { url?: string }).url ?? "");
        const alt = String((raw as { alt?: string }).alt ?? "");
        out += `<img src="${escapeHtml(url)}" alt="${escapeHtml(alt)}">`;
        break;
      }
      default: {
        // For unknown inline (e.g., custom), fall back to markdown phrasing then escape?
        // Use state's containerPhrasing for a single node wrapper.
        try {
          out += s.containerPhrasing(raw as never, info);
        } catch {
          out += escapeHtml(String(raw.value ?? ""));
        }
        break;
      }
    }
  }
  return out;
}

export const alignedParagraphToMarkdown: ToMarkdownHandle = (node, _parent, state, info) => {
  const aligned = node as AlignedParagraph;
  if (aligned.dir === "rtl" || aligned.dir === "ltr") {
    const content = phrasingToHtml((aligned as unknown as { children: unknown[] }).children ?? [], state, info);
    return `<p dir="${aligned.dir}" style="text-align: ${aligned.align}">${content}</p>`;
  }
  const content = state.containerPhrasing(aligned as never, info);
  return `<div align="${aligned.align}">${content}</div>`;
};

function hPropertiesDir(node: unknown): string {
  const data = (node as { data?: { hProperties?: Record<string, unknown> } })?.data;
  const dir = data?.hProperties?.dir;
  if (dir === "rtl" || dir === "ltr") return dir as string;
  const direct = (node as { dir?: unknown })?.dir;
  if (direct === "rtl" || direct === "ltr") return direct as string;
  return "";
}

function hPropertiesStyle(node: unknown): string {
  const data = (node as { data?: { hProperties?: Record<string, unknown> } })?.data;
  const style = data?.hProperties?.style;
  return typeof style === "string" ? style : "";
}

export const paragraphToMarkdown: ToMarkdownHandle = (node, _parent, state, info) => {
  const dir = hPropertiesDir(node);
  const style = hPropertiesStyle(node);
  if (dir === "rtl" || dir === "ltr") {
    const children = (node as { children?: unknown[] }).children ?? [];
    const content = phrasingToHtml(children, state, info);
    const attrs = [`dir="${dir}"`];
    if (style && /^text-align\s*:\s*(?:left|center|right)\s*;?$/i.test(style)) attrs.push(`style="${style}"`);
    return `<p ${attrs.join(" ")}>${content}</p>`;
  }
  // fallback to default paragraph handling
  const exit = state.enter("paragraph");
  const subexit = state.enter("phrasing");
  const value = state.containerPhrasing(node as never, info);
  subexit();
  exit();
  return value;
};

export const headingToMarkdown: ToMarkdownHandle = (node, _parent, state, info) => {
  const dir = hPropertiesDir(node);
  const style = hPropertiesStyle(node);
  const hasDir = dir === "rtl" || dir === "ltr";
  const hasStyle = style && /^text-align\s*:\s*(?:left|center|right)\s*;?$/i.test(style);
  if (hasDir || hasStyle) {
    const heading = node as { depth?: number; children?: unknown[] };
    const depth = Math.max(1, Math.min(6, Number(heading.depth ?? 1)));
    const tag = `h${depth}`;
    const children = heading.children ?? [];
    const content = phrasingToHtml(children, state, info);
    const attrs: string[] = [];
    if (hasDir) attrs.push(`dir="${dir}"`);
    if (hasStyle) attrs.push(`style="${style}"`);
    return `<${tag} ${attrs.join(" ")}>${content}</${tag}>`;
  }
  // fallback to default heading handling (copied from mdast-util-to-markdown)
  const heading = node as { depth?: number };
  const rank = Math.max(Math.min(6, Number(heading.depth ?? 1)), 1);
  const sequence = "#".repeat(rank);
  const exit = state.enter("headingAtx");
  const subexit = state.enter("phrasing");
  const tracker = state.createTracker(info as never);
  tracker.move(sequence + " ");
  let value = state.containerPhrasing(
    node as never,
    {
      before: "# ",
      after: "\n",
      ...tracker.current(),
    } as never,
  );
  if (/^[\t ]/.test(value)) {
    const first = value.charCodeAt(0);
    // encodeCharacterReference fallback
    value = `&#x${first.toString(16).toUpperCase()};` + value.slice(1);
  }
  value = value ? sequence + " " + value : sequence;
  if ((state.options as { closeAtx?: boolean })?.closeAtx) value += " " + sequence;
  subexit();
  exit();
  return value;
};
