import type { Editor } from "@tiptap/core";
import { TableCell, TableHeader } from "@tiptap/extension-table";

function tableAlignment(element: HTMLElement): "left" | "center" | "right" | null {
  const attribute = element.getAttribute("align")?.toLowerCase();
  if (attribute === "left" || attribute === "center" || attribute === "right") return attribute;
  const style = element.style.textAlign.toLowerCase();
  return style === "left" || style === "center" || style === "right" ? style : null;
}

const alignmentAttribute = {
  default: null,
  parseHTML: tableAlignment,
  renderHTML: ({ align }: { align?: string | null }) =>
    align === "left" || align === "center" || align === "right" ? { align, style: `text-align: ${align}` } : {},
};

export const AlignedTableHeader = TableHeader.extend({
  addAttributes() {
    return { ...this.parent?.(), align: alignmentAttribute };
  },
});

export const AlignedTableCell = TableCell.extend({
  addAttributes() {
    return { ...this.parent?.(), align: alignmentAttribute };
  },
});

/** True when the selection is inside a table whose first row uses header cells. */
export function tableHasHeaderRow(editor: Editor | null | undefined): boolean {
  if (!editor) return false;
  const { $from } = editor.state.selection;
  for (let depth = $from.depth; depth > 0; depth -= 1) {
    const node = $from.node(depth);
    if (node.type.name !== "table") continue;
    const firstRow = node.firstChild;
    if (!firstRow || firstRow.childCount === 0) return false;
    return firstRow.firstChild?.type.name === "tableHeader";
  }
  return false;
}
