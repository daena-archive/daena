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
    light: { fg: "#867022", bg: "#f7f2e8" },
    dark: { fg: "#c4a854", bg: "#2a2618" },
  },
  copper: {
    id: "copper",
    label: "Copper",
    light: { fg: "#9e5038", bg: "#f9eee8" },
    dark: { fg: "#c88862", bg: "#2a1e16" },
  },
  ember: {
    id: "ember",
    label: "Ember",
    light: { fg: "#a84840", bg: "#faf0ee" },
    dark: { fg: "#c88078", bg: "#2a1816" },
  },
  moss: {
    id: "moss",
    label: "Moss",
    light: { fg: "#6a7828", bg: "#f2f4ea" },
    dark: { fg: "#98a860", bg: "#1e2214" },
  },
  pine: {
    id: "pine",
    label: "Pine",
    light: { fg: "#3a6450", bg: "#edf2ee" },
    dark: { fg: "#6a9078", bg: "#141e18" },
  },
  ocean: {
    id: "ocean",
    label: "Ocean",
    light: { fg: "#3a6868", bg: "#eaf0f0" },
    dark: { fg: "#72a0a0", bg: "#161e1e" },
  },
  sky: {
    id: "sky",
    label: "Sky",
    light: { fg: "#4a6080", bg: "#eef1f4" },
    dark: { fg: "#849ab4", bg: "#181e28" },
  },
  frost: {
    id: "frost",
    label: "Frost",
    light: { fg: "#4a7088", bg: "#eef3f4" },
    dark: { fg: "#88acb8", bg: "#181e22" },
  },
  amber: {
    id: "amber",
    label: "Amber",
    light: { fg: "#a06828", bg: "#f8f0e6" },
    dark: { fg: "#c89450", bg: "#2a2218" },
  },
  gold: {
    id: "gold",
    label: "Gold",
    light: { fg: "#9a8428", bg: "#f8f4ea" },
    dark: { fg: "#c8aa50", bg: "#2a2618" },
  },
  sand: {
    id: "sand",
    label: "Sand",
    light: { fg: "#847870", bg: "#f5f1ea" },
    dark: { fg: "#b0a498", bg: "#242018" },
  },
  rose: {
    id: "rose",
    label: "Rose",
    light: { fg: "#a05060", bg: "#f9f0f2" },
    dark: { fg: "#c48898", bg: "#281820" },
  },
  plum: {
    id: "plum",
    label: "Plum",
    light: { fg: "#804860", bg: "#f4eef2" },
    dark: { fg: "#a87898", bg: "#221820" },
  },
  violet: {
    id: "violet",
    label: "Violet",
    light: { fg: "#605878", bg: "#f0eef4" },
    dark: { fg: "#9088a8", bg: "#1e1828" },
  },
  slate: {
    id: "slate",
    label: "Slate",
    light: { fg: "#586068", bg: "#eef0f2" },
    dark: { fg: "#949aa4", bg: "#222428" },
  },
  ink: {
    id: "ink",
    label: "Ink",
    light: { fg: "#484440", bg: "#f2f0ec" },
    dark: { fg: "#a8a098", bg: "#1e1c18" },
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
