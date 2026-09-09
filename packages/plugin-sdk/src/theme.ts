import { BUILTIN_THEME_TOKENS, THEME_TOKEN_IDS, type PluginAppearance, type ThemeTokenId } from "./generated.js";

const TOKEN_IDS = new Set<string>(THEME_TOKEN_IDS);
const TEXT_CONTRAST_PAIRS: Array<[ThemeTokenId, ThemeTokenId]> = [
  ["ink", "surface"],
  ["ink", "canvas"],
];
const CHROME_CONTRAST_PAIRS: Array<[ThemeTokenId, ThemeTokenId]> = [["rail-text", "rail-bg"]];
const MIN_TEXT_CONTRAST = 4.5;
const MIN_CHROME_CONTRAST = 3.0;
const THEME_PACK_NAME_MAX_CHARS = 128;

export type ThemeMode = "light" | "dark";
export type ThemeTokenMap = Record<ThemeTokenId, string>;

export function parseThemeColor(value: string): string {
  const trimmed = value.trim();
  if (!trimmed.startsWith("#")) throw new Error(`invalid theme color: ${value}`);
  const hex = trimmed.slice(1);
  if (![...hex].every((ch) => /[0-9A-Fa-f]/.test(ch))) throw new Error(`invalid theme color: ${value}`);
  const normalized =
    hex.length === 3
      ? `#${hex[0]}${hex[0]}${hex[1]}${hex[1]}${hex[2]}${hex[2]}`
      : hex.length === 6 || hex.length === 8
        ? `#${hex}`
        : null;
  if (!normalized) throw new Error(`invalid theme color: ${value}`);
  return normalized.toLowerCase();
}

export function mergeThemeTokens(base: ThemeTokenMap, overlay: Record<string, string>): ThemeTokenMap {
  const merged = { ...base };
  for (const [key, value] of Object.entries(overlay)) {
    if (!TOKEN_IDS.has(key)) throw new Error(`unknown theme token: ${key}`);
    merged[key as ThemeTokenId] = parseThemeColor(value);
  }
  return merged;
}

export function resolveThemeTokens(mode: ThemeMode, overlay: Record<string, string>): ThemeTokenMap {
  const merged = mergeThemeTokens({ ...BUILTIN_THEME_TOKENS[mode] }, overlay);
  validateThemeContrast(merged);
  return merged;
}

export function validateThemeContrast(tokens: ThemeTokenMap): void {
  checkContrastPairs(tokens, TEXT_CONTRAST_PAIRS, MIN_TEXT_CONTRAST);
  checkContrastPairs(tokens, CHROME_CONTRAST_PAIRS, MIN_CHROME_CONTRAST);
}

export function validateThemePack(pack: {
  id: string;
  name: string;
  tokens: { light: unknown; dark: unknown };
}): string[] {
  const errors: string[] = [];
  if (typeof pack.name !== "string" || !pack.name.trim()) errors.push(`theme ${pack.id} name is required`);
  else if ([...pack.name].length > THEME_PACK_NAME_MAX_CHARS) errors.push(`theme ${pack.id} name is too long`);
  if (!isTokenMap(pack.tokens.light)) errors.push(`theme ${pack.id} light tokens must be an object`);
  else {
    try {
      resolveThemeTokens("light", pack.tokens.light);
    } catch (cause) {
      errors.push(cause instanceof Error ? cause.message : String(cause));
    }
  }
  if (!isTokenMap(pack.tokens.dark)) errors.push(`theme ${pack.id} dark tokens must be an object`);
  else {
    try {
      resolveThemeTokens("dark", pack.tokens.dark);
    } catch (cause) {
      errors.push(cause instanceof Error ? cause.message : String(cause));
    }
  }
  return errors;
}

function isTokenMap(value: unknown): value is Record<string, string> {
  if (!value || typeof value !== "object" || Array.isArray(value)) return false;
  return Object.values(value).every((item) => typeof item === "string");
}

function checkContrastPairs(tokens: ThemeTokenMap, pairs: Array<[ThemeTokenId, ThemeTokenId]>, minimum: number): void {
  for (const [foreground, background] of pairs) {
    const fg = tokens[foreground];
    const bg = tokens[background];
    if (!fg) throw new Error(`missing theme token: ${foreground}`);
    if (!bg) throw new Error(`missing theme token: ${background}`);
    const ratio = contrastRatio(fg, bg);
    if (ratio + Number.EPSILON < minimum) {
      throw new Error(
        `theme contrast ${foreground}/${background} is ${ratio.toFixed(2)}, expected at least ${minimum}`,
      );
    }
  }
}

function contrastRatio(a: string, b: string): number {
  const l1 = relativeLuminance(a);
  const l2 = relativeLuminance(b);
  const lighter = Math.max(l1, l2);
  const darker = Math.min(l1, l2);
  return (lighter + 0.05) / (darker + 0.05);
}

function relativeLuminance(color: string): number {
  const [r, g, b] = rgbChannels(color);
  return 0.2126 * linearChannel(r) + 0.7152 * linearChannel(g) + 0.0722 * linearChannel(b);
}

function rgbChannels(color: string): [number, number, number] {
  const normalized = parseThemeColor(color);
  const hex = normalized.slice(1);
  if (hex.length === 8) {
    const alpha = Number.parseInt(hex.slice(6, 8), 16);
    if (alpha !== 0xff) throw new Error(`theme contrast requires opaque colors, got ${color}`);
  }
  return [
    Number.parseInt(hex.slice(0, 2), 16),
    Number.parseInt(hex.slice(2, 4), 16),
    Number.parseInt(hex.slice(4, 6), 16),
  ];
}

function linearChannel(value: number): number {
  const srgb = value / 255;
  return srgb <= 0.04045 ? srgb / 12.92 : ((srgb + 0.055) / 1.055) ** 2.4;
}

const TOKEN_HEX = /^#(?:[0-9A-Fa-f]{3}|[0-9A-Fa-f]{6}|[0-9A-Fa-f]{8})$/;

export type PluginAppearanceRoot = {
  dataset: { theme?: string; themePreference?: string };
  style: {
    colorScheme: string;
    setProperty(property: string, value: string): void;
    removeProperty(property: string): void;
  };
};

export function applyPluginAppearance(appearance: PluginAppearance, root: PluginAppearanceRoot): void {
  if (appearance.resolved === "light" || appearance.resolved === "dark") {
    root.dataset.theme = appearance.resolved;
    root.style.colorScheme = appearance.resolved;
  }
  if (appearance.preference === "light" || appearance.preference === "dark" || appearance.preference === "system") {
    root.dataset.themePreference = appearance.preference;
  }
  for (const id of THEME_TOKEN_IDS) {
    const value = appearance.tokens[id];
    if (typeof value === "string" && TOKEN_HEX.test(value)) root.style.setProperty(`--${id}`, value);
    else root.style.removeProperty(`--${id}`);
  }
}

export function hostHasFeature(features: readonly string[] | undefined, feature: string): boolean {
  return Boolean(features?.includes(feature));
}
