import { TYPE_COLOR_PRESET_IDS, type EntityTypeColor } from "../../../packages/plugin-sdk/src/generated";
import type { ResolvedTheme } from "$lib/theme";

export type TypeColorPresetId = (typeof TYPE_COLOR_PRESET_IDS)[number];

export interface ResolvedTypeColor {
  fg: string;
  bg: string;
}

export interface TypeColorPreset {
  id: TypeColorPresetId;
  label: string;
  light: ResolvedTypeColor;
  dark: ResolvedTypeColor;
}

export const TYPE_COLOR_PRESETS = {
  brass: {
    id: "brass",
    label: "Brass",
    light: { fg: "#6f6820", bg: "#f4f1e4" },
    dark: { fg: "#c2b56a", bg: "#262418" },
  },
  copper: {
    id: "copper",
    label: "Copper",
    light: { fg: "#9a4a2e", bg: "#f8ebe4" },
    dark: { fg: "#d09068", bg: "#2a1c14" },
  },
  ember: {
    id: "ember",
    label: "Ember",
    light: { fg: "#a83838", bg: "#faf0ee" },
    dark: { fg: "#d08078", bg: "#2a1614" },
  },
  moss: {
    id: "moss",
    label: "Moss",
    light: { fg: "#5f7424", bg: "#f1f4e8" },
    dark: { fg: "#9ab060", bg: "#1c2214" },
  },
  pine: {
    id: "pine",
    label: "Pine",
    light: { fg: "#2f5c48", bg: "#ebf1ec" },
    dark: { fg: "#6e9880", bg: "#121c16" },
  },
  ocean: {
    id: "ocean",
    label: "Ocean",
    light: { fg: "#2f6464", bg: "#e8f0f0" },
    dark: { fg: "#6ea8a4", bg: "#121c1c" },
  },
  sky: {
    id: "sky",
    label: "Sky",
    light: { fg: "#3f5a8a", bg: "#eef1f6" },
    dark: { fg: "#8aa0c4", bg: "#161c28" },
  },
  frost: {
    id: "frost",
    label: "Frost",
    light: { fg: "#2f687c", bg: "#eaf4f6" },
    dark: { fg: "#7eb8c4", bg: "#141e22" },
  },
  amber: {
    id: "amber",
    label: "Amber",
    light: { fg: "#a05a18", bg: "#f8efe4" },
    dark: { fg: "#d09048", bg: "#2a1e12" },
  },
  gold: {
    id: "gold",
    label: "Gold",
    light: { fg: "#80640a", bg: "#f7f2e2" },
    dark: { fg: "#e0c45c", bg: "#2a2414" },
  },
  sand: {
    id: "sand",
    label: "Sand",
    light: { fg: "#74604c", bg: "#f5f0e8" },
    dark: { fg: "#c4b49c", bg: "#262018" },
  },
  rose: {
    id: "rose",
    label: "Rose",
    light: { fg: "#a04458", bg: "#f9eef1" },
    dark: { fg: "#d08898", bg: "#28161c" },
  },
  plum: {
    id: "plum",
    label: "Plum",
    light: { fg: "#703858", bg: "#f3ecf1" },
    dark: { fg: "#b080a0", bg: "#20141e" },
  },
  violet: {
    id: "violet",
    label: "Violet",
    light: { fg: "#58507a", bg: "#efedf4" },
    dark: { fg: "#9a90b8", bg: "#1c1628" },
  },
  slate: {
    id: "slate",
    label: "Slate",
    light: { fg: "#4e5864", bg: "#eef0f2" },
    dark: { fg: "#9aa2ac", bg: "#1e2226" },
  },
  ink: {
    id: "ink",
    label: "Ink",
    light: { fg: "#3c3a38", bg: "#f1efec" },
    dark: { fg: "#b0aaa4", bg: "#1a1816" },
  },
} as const satisfies Record<TypeColorPresetId, TypeColorPreset>;

export const TYPE_COLOR_PRESET_OPTIONS = TYPE_COLOR_PRESET_IDS.map((id) => TYPE_COLOR_PRESETS[id]);

export const DEFAULT_TYPE_COLOR: EntityTypeColor = { kind: "preset", id: "brass" };

export function resolveEntityTypeColor(color: EntityTypeColor, theme: ResolvedTheme): ResolvedTypeColor {
  if (color.kind === "preset") {
    const preset = TYPE_COLOR_PRESETS[color.id as TypeColorPresetId] ?? TYPE_COLOR_PRESETS.brass;
    return theme === "dark" ? preset.dark : preset.light;
  }
  return {
    fg: theme === "dark" ? color.dark : color.light,
    bg: mixTypeBackground(theme === "dark" ? color.dark : color.light, theme),
  };
}

export function mixTypeBackground(fg: string, theme: ResolvedTheme): string {
  const surface = theme === "dark" ? "#131f1b" : "#fffefa";
  return `color-mix(in srgb, ${fg} 16%, ${surface})`;
}

export function normalizeHexColor(value: string): string | null {
  const trimmed = value.trim();
  if (!/^#[0-9A-Fa-f]{6}$/.test(trimmed)) return null;
  return trimmed.toLowerCase();
}

export function validateCustomTypeColor(light: string, dark: string): string | null {
  if (!normalizeHexColor(light)) return "Light color must be a #RRGGBB hex value.";
  if (!normalizeHexColor(dark)) return "Dark color must be a #RRGGBB hex value.";
  return null;
}
