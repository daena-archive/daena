export type PromptKind = "editor" | "git" | "image";

export interface PromptTemplate {
  id: string;
  label: string;
  instruction: string;
  kind: PromptKind;
  requiresSelection?: boolean;
  enabled?: boolean;
  bundled?: boolean;
}

export interface PromptOverlay {
  templates?: Array<Partial<PromptTemplate> & { id: string }>;
}

export const BUNDLED_PROMPT_TEMPLATES: PromptTemplate[] = [
  {
    id: "rewrite",
    label: "Rewrite selection",
    instruction: "Rewrite this to be more vivid while preserving the meaning.",
    kind: "editor",
    requiresSelection: true,
    bundled: true,
  },
  {
    id: "generate",
    label: "Generate text",
    instruction: "Write text that fits naturally at the cursor position.",
    kind: "editor",
    bundled: true,
  },
  {
    id: "concise",
    label: "Make concise",
    instruction: "Make this more concise while preserving the meaning.",
    kind: "editor",
    requiresSelection: true,
    bundled: true,
  },
  {
    id: "expand",
    label: "Expand",
    instruction: "Expand this with useful detail while preserving the meaning.",
    kind: "editor",
    requiresSelection: true,
    bundled: true,
  },
  {
    id: "grammar",
    label: "Fix grammar",
    instruction: "Fix grammar, spelling, and awkward phrasing while preserving the meaning.",
    kind: "editor",
    requiresSelection: true,
    bundled: true,
  },
  {
    id: "tone",
    label: "Change tone",
    instruction: "Change the tone of this passage while preserving its meaning. Ask for the desired tone if needed.",
    kind: "editor",
    requiresSelection: true,
    bundled: true,
  },
  {
    id: "custom",
    label: "Custom instruction",
    instruction: "",
    kind: "editor",
    bundled: true,
  },
  {
    id: "git-message",
    label: "Snapshot message",
    instruction:
      "Write a concise snapshot message from the confirmed changes. Put a title of 72 characters or fewer on the first line. Optionally add the comment body after a newline; never combine them on one line. Use plain text without labels, Markdown, bullets, paths, UUIDs, hashes, or internal identifiers.",
    kind: "git",
    bundled: true,
  },
  {
    id: "image-entity",
    label: "Image from facts",
    instruction:
      "Write one concise, visually oriented text-to-image prompt from the selected world facts. Preserve facts, avoid unsupported inventions, and return only the final prompt.",
    kind: "image",
    bundled: true,
  },
  {
    id: "image-rewrite",
    label: "Rewrite image prompt",
    instruction:
      "Rewrite the current image prompt into one concise, visually specific text-to-image prompt. Preserve all stated facts. Return only the final prompt.",
    kind: "image",
    bundled: true,
  },
  {
    id: "image-detailed",
    label: "Detail image prompt",
    instruction:
      "Make the current image prompt more visually detailed while preserving every stated fact. Add composition, lighting, materials, and atmosphere only when consistent with the supplied context. Return only the final prompt.",
    kind: "image",
    bundled: true,
  },
  {
    id: "image-simplified",
    label: "Simplify image prompt",
    instruction:
      "Simplify the current image prompt into a concise, clear text-to-image prompt without losing stated world facts. Return only the final prompt.",
    kind: "image",
    bundled: true,
  },
];

export function emptyAiProvider(): {
  id: string;
  name: string;
  adapter: string;
  endpoint: string;
  model: string;
  embeddingModel: string;
  capabilities: string[];
} {
  return {
    id: "",
    name: "",
    adapter: "openai-compatible",
    endpoint: "",
    model: "",
    embeddingModel: "",
    capabilities: [],
  };
}

export function mergePromptTemplates(overlay?: PromptOverlay | null): PromptTemplate[] {
  const overrides = new Map(
    (overlay?.templates ?? []).filter((item) => item.id?.trim()).map((item) => [item.id, item]),
  );
  const merged: PromptTemplate[] = BUNDLED_PROMPT_TEMPLATES.map((bundled) => {
    const override = overrides.get(bundled.id);
    if (!override) return { ...bundled, enabled: bundled.enabled !== false };
    return {
      ...bundled,
      label: override.label?.trim() || bundled.label,
      instruction: override.instruction ?? bundled.instruction,
      enabled: override.enabled !== false,
      requiresSelection: override.requiresSelection ?? bundled.requiresSelection,
      kind: override.kind ?? bundled.kind,
      bundled: true,
    };
  });
  for (const item of overlay?.templates ?? []) {
    if (!item.id?.trim() || merged.some((template) => template.id === item.id)) continue;
    if (!item.label?.trim() || item.instruction == null) continue;
    merged.push({
      id: item.id.trim(),
      label: item.label.trim(),
      instruction: item.instruction,
      kind: item.kind ?? "editor",
      requiresSelection: item.requiresSelection === true,
      enabled: item.enabled !== false,
      bundled: false,
    });
  }
  return merged;
}

export function overlayFromTemplates(templates: PromptTemplate[]): PromptOverlay {
  const bundled = new Map(BUNDLED_PROMPT_TEMPLATES.map((template) => [template.id, template]));
  const records: PromptOverlay["templates"] = [];
  for (const template of templates) {
    const original = bundled.get(template.id);
    if (!original) {
      records.push({
        id: template.id,
        label: template.label,
        instruction: template.instruction,
        kind: template.kind,
        requiresSelection: template.requiresSelection,
        enabled: template.enabled !== false,
      });
      continue;
    }
    const enabled = template.enabled !== false;
    const changed =
      !enabled ||
      template.label !== original.label ||
      template.instruction !== original.instruction ||
      Boolean(template.requiresSelection) !== Boolean(original.requiresSelection);
    if (!changed) continue;
    records.push({
      id: template.id,
      label: template.label,
      instruction: template.instruction,
      enabled,
    });
  }
  return { templates: records };
}

export function instructionFor(templates: PromptTemplate[], id: string): string {
  return templates.find((template) => template.id === id && template.enabled !== false)?.instruction ?? "";
}
