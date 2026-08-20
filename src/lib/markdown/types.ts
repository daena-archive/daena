export interface EntityReference {
  type: "entityReference";
  entityId: string;
  isCustom?: boolean;
  children: Array<{ type: string; value?: string; children?: unknown[] }>;
  data?: { hName?: string; hProperties?: Record<string, unknown> };
}

export interface Underline {
  type: "underline";
  children: Array<{ type: string; value?: string; children?: unknown[] }>;
  data?: { hName?: string; hProperties?: Record<string, unknown> };
}

export interface Spoiler {
  type: "spoiler";
  children: Array<{ type: string; value?: string; children?: unknown[] }>;
  data?: { hName?: string; hProperties?: Record<string, unknown> };
}

export interface AlignedParagraph {
  type: "alignedParagraph";
  align: "center" | "right";
  dir?: "ltr" | "rtl";
  children: Array<{ type: string; value?: string; children?: unknown[] }>;
  data?: { hName?: string; hProperties?: Record<string, unknown> };
}

export interface HeadingOutlineItem {
  depth: number;
  text: string;
  id: string;
}

export interface EntityReferenceInfo {
  entityId: string;
  label: string;
}
