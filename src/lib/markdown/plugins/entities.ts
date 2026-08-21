import type { Link, PhrasingContent, Root } from "mdast";
import type { Handle as ToMarkdownHandle } from "mdast-util-to-markdown";
import { visit } from "unist-util-visit";
import { nodeText } from "../text.ts";
import { entityIdFromHref } from "../urls.ts";
import type { EntityReference } from "../types.ts";

function unwrapBracketLabel(children: PhrasingContent[]): PhrasingContent[] {
  const label = nodeText({ children });
  if (!label.startsWith("[") || !label.endsWith("]") || label.length < 2) return children;
  const copy = children.map((child) => ({ ...child })) as PhrasingContent[];
  const first = copy[0];
  const last = copy[copy.length - 1];
  if (first?.type === "text") first.value = first.value.replace(/^\[/, "");
  if (last?.type === "text") last.value = last.value.replace(/\]$/, "");
  return copy.filter((child) => child.type !== "text" || child.value !== "");
}

function entityIdFromLink(node: Link): string | null {
  const fromProtocol = entityIdFromHref(node.url);
  if (fromProtocol) return fromProtocol;
  const label = nodeText(node);
  if (label.startsWith("[") && label.endsWith("]") && label.length > 2 && !/^[a-z][a-z0-9+.-]*:/i.test(node.url)) {
    return node.url.trim() || null;
  }
  return null;
}

export function applyDaenaEntities(tree: Root): void {
  visit(tree, "link", (node, index, parent) => {
    if (index == null || !parent) return;
    const entityId = entityIdFromLink(node);
    if (!entityId) return;
    const unwrapped = unwrapBracketLabel(node.children) as EntityReference["children"];
    const labelText = nodeText({ children: unwrapped }).trim();
    const isCustom = labelText.length > 0;
    const next: EntityReference = {
      type: "entityReference",
      entityId,
      isCustom,
      children: unwrapped,
      data: {
        hName: "a",
        hProperties: {
          href: `daena://entity/${encodeURIComponent(entityId)}`,
          dataEntityId: entityId,
          dataIsCustom: isCustom ? "true" : "false",
          className: ["entity-reference"],
        },
      },
    };
    parent.children[index] = next as never;
  });
}

export function remarkDaenaEntities() {
  return (tree: Root) => {
    applyDaenaEntities(tree);
  };
}

export const entityReferenceToMarkdown: ToMarkdownHandle = (node, _parent, state, info) => {
  const reference = node as EntityReference;
  const isCustom = reference.isCustom ?? nodeText(reference as never).trim().length > 0;
  if (!isCustom) return `[[]](${reference.entityId})`;
  return `[[${state.containerPhrasing(reference as never, info)}]](${reference.entityId})`;
};
