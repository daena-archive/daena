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
