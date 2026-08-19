import type { Root } from "mdast";
import { visit } from "unist-util-visit";
import { safeSrc } from "../urls.ts";

export function applyDaenaAssets(tree: Root): void {
  visit(tree, "image", (node, index, parent) => {
    if (index == null || !parent) return;
    if (safeSrc(node.url)) return;
    parent.children[index] = { type: "text", value: node.alt ?? "" };
  });
}

export function remarkDaenaAssets() {
  return (tree: Root) => {
    applyDaenaAssets(tree);
  };
}
