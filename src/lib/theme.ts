import { BUILTIN_THEME_TOKENS, THEME_TOKEN_IDS, type ThemeTokenId } from "../../packages/plugin-sdk/src/generated.ts";

const TOKEN_IDS = new Set<string>(THEME_TOKEN_IDS);
const TEXT_CONTRAST_PAIRS: Array<[ThemeTokenId, ThemeTokenId]> = [
  ["ink", "surface"],
  ["ink", "canvas"],
];
const CHROME_CONTRAST_PAIRS: Array<[ThemeTokenId, ThemeTokenId]> = [["rail-text", "rail-bg"]];
const MIN_TEXT_CONTRAST = 4.5;
const MIN_CHROME_CONTRAST = 3.0;

export type ThemePreference = "light" | "dark" | "system";
export type ResolvedTheme = Exclude<ThemePreference, "system">;
export type ThemeTokenMap = Record<ThemeTokenId, string>;
export type ThemePackRef = { pluginId: string; themeId: string };
export type InstalledThemePack = {
  pluginId: string;
  pluginName: string;
  themeId: string;
  name: string;
  tokens: { light: Record<string, string>; dark: Record<string, string> };
};

export const THEME_STORAGE_KEY = "daena-theme";
export const THEME_PACK_STORAGE_KEY = "daena-theme-pack";
export type CachedThemePackTokens = { light: ThemeTokenMap; dark: ThemeTokenMap };

export type ThemeRoot = {
  dataset: { theme?: string; themePreference?: string };
  style: { colorScheme: string; setProperty(property: string, value: string): void };
};

export function normalizeThemePreference(value: unknown): ThemePreference {
  return value === "light" || value === "dark" || value === "system" ? value : "system";
}

export function normalizeThemePackRef(value: unknown): ThemePackRef | null {
  if (!value || typeof value !== "object") return null;
  const pluginId = "pluginId" in value && typeof value.pluginId === "string" ? value.pluginId.trim() : "";
  const themeId = "themeId" in value && typeof value.themeId === "string" ? value.themeId.trim() : "";
  return pluginId && themeId ? { pluginId, themeId } : null;
}

export function resolveTheme(preference: ThemePreference, systemPrefersDark: boolean): ResolvedTheme {
  return preference === "system" ? (systemPrefersDark ? "dark" : "light") : preference;
}

export function readCachedThemePreference(storage: Pick<Storage, "getItem"> = localStorage): ThemePreference {
  try {
    return normalizeThemePreference(storage.getItem(THEME_STORAGE_KEY));
  } catch {
    return "system";
  }
}

export function cacheThemePreference(
  preference: ThemePreference,
  storage: Pick<Storage, "setItem"> = localStorage,
): void {
  try {
    storage.setItem(THEME_STORAGE_KEY, preference);
  } catch {
    // Theme changes should still apply when storage is unavailable.
  }
}

function isTokenMap(value: unknown): value is Record<string, string> {
  if (!value || typeof value !== "object" || Array.isArray(value)) return false;
  return Object.values(value).every((item) => typeof item === "string");
}

function isCachedTokenMap(value: unknown): value is ThemeTokenMap {
  if (!isTokenMap(value)) return false;
  return THEME_TOKEN_IDS.every((id) => typeof value[id] === "string" && value[id].startsWith("#"));
}

export function readCachedThemePackTokens(
  storage: Pick<Storage, "getItem"> = localStorage,
): CachedThemePackTokens | null {
  try {
    const parsed: unknown = JSON.parse(storage.getItem(THEME_PACK_STORAGE_KEY) ?? "");
    if (!parsed || typeof parsed !== "object") return null;
    const light = "light" in parsed ? parsed.light : null;
    const dark = "dark" in parsed ? parsed.dark : null;
    if (!isCachedTokenMap(light) || !isCachedTokenMap(dark)) return null;
    return { light, dark };
  } catch {
    return null;
  }
}

export function cacheThemePackTokens(
  tokens: CachedThemePackTokens | null,
  storage: Pick<Storage, "setItem" | "removeItem"> = localStorage,
): void {
  try {
    if (!tokens) storage.removeItem(THEME_PACK_STORAGE_KEY);
    else storage.setItem(THEME_PACK_STORAGE_KEY, JSON.stringify(tokens));
  } catch {
    // Theme changes should still apply when storage is unavailable.
  }
}

export function resolveBuiltinTokens(resolved: ResolvedTheme): ThemeTokenMap {
  return { ...BUILTIN_THEME_TOKENS[resolved] };
}

export function collectInstalledThemePacks(
  plugins: ReadonlyArray<{
    id: string;
    name: string;
    themes?: ReadonlyArray<{
      id?: unknown;
      name?: unknown;
      tokens?: { light?: unknown; dark?: unknown };
    }>;
  }>,
): InstalledThemePack[] {
  return plugins.flatMap((plugin) =>
    (plugin.themes ?? []).flatMap((theme) => {
      if (typeof theme.id !== "string" || typeof theme.name !== "string") return [];
      if (!isTokenMap(theme.tokens?.light) || !isTokenMap(theme.tokens?.dark)) return [];
      return [
        {
          pluginId: plugin.id,
          pluginName: plugin.name,
          themeId: theme.id,
          name: theme.name,
          tokens: { light: theme.tokens.light, dark: theme.tokens.dark },
        },
      ];
    }),
  );
}

export function overlayForThemePack(
  packs: readonly InstalledThemePack[],
  ref: ThemePackRef | null,
  resolved: ResolvedTheme,
): Record<string, string> | null {
  if (!ref) return null;
  const pack = packs.find((item) => item.pluginId === ref.pluginId && item.themeId === ref.themeId);
  return pack?.tokens[resolved] ?? null;
}

function parseThemeColor(value: string): string {
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

function contrastRatio(a: string, b: string): number {
  const luminance = (color: string) => {
    const normalized = parseThemeColor(color);
    const hex = normalized.slice(1);
    if (hex.length === 8 && Number.parseInt(hex.slice(6, 8), 16) !== 0xff) {
      throw new Error(`theme contrast requires opaque colors, got ${color}`);
    }
    const channel = (offset: number) => {
      const value = Number.parseInt(hex.slice(offset, offset + 2), 16) / 255;
      return value <= 0.04045 ? value / 12.92 : ((value + 0.055) / 1.055) ** 2.4;
    };
    return 0.2126 * channel(0) + 0.7152 * channel(2) + 0.0722 * channel(4);
  };
  const first = luminance(a);
  const second = luminance(b);
  const lighter = Math.max(first, second);
  const darker = Math.min(first, second);
  return (lighter + 0.05) / (darker + 0.05);
}

function mergeResolvedTokens(resolved: ResolvedTheme, overlay: Record<string, string>): ThemeTokenMap {
  const merged: ThemeTokenMap = { ...BUILTIN_THEME_TOKENS[resolved] };
  for (const [key, value] of Object.entries(overlay)) {
    if (!TOKEN_IDS.has(key)) throw new Error(`unknown theme token: ${key}`);
    merged[key as ThemeTokenId] = parseThemeColor(value);
  }
  for (const [foreground, background] of [...TEXT_CONTRAST_PAIRS, ...CHROME_CONTRAST_PAIRS]) {
    const minimum = TEXT_CONTRAST_PAIRS.some((pair) => pair[0] === foreground && pair[1] === background)
      ? MIN_TEXT_CONTRAST
      : MIN_CHROME_CONTRAST;
    const ratio = contrastRatio(merged[foreground], merged[background]);
    if (ratio + Number.EPSILON < minimum) throw new Error("theme contrast");
  }
  return merged;
}

export function resolvedPackTokenCache(
  packs: readonly InstalledThemePack[],
  ref: ThemePackRef | null,
): CachedThemePackTokens | null {
  if (!ref) return null;
  const lightOverlay = overlayForThemePack(packs, ref, "light");
  const darkOverlay = overlayForThemePack(packs, ref, "dark");
  if (!lightOverlay || !darkOverlay) return null;
  try {
    return {
      light: mergeResolvedTokens("light", lightOverlay),
      dark: mergeResolvedTokens("dark", darkOverlay),
    };
  } catch {
    return null;
  }
}

export function resolveAppliedTokens(
  resolved: ResolvedTheme,
  overlay: Record<string, string> | null = null,
): ThemeTokenMap {
  if (!overlay) return resolveBuiltinTokens(resolved);
  try {
    return mergeResolvedTokens(resolved, overlay);
  } catch {
    return resolveBuiltinTokens(resolved);
  }
}

export function applyThemeTokens(tokens: ThemeTokenMap, root: ThemeRoot): void {
  for (const id of THEME_TOKEN_IDS) {
    const value = tokens[id];
    if (!value) throw new Error(`missing theme token: ${id}`);
    root.style.setProperty(`--${id}`, value);
  }
}

export function applyThemePreference(
  preference: ThemePreference,
  root: ThemeRoot = document.documentElement,
  systemPrefersDark = matchMedia("(prefers-color-scheme: dark)").matches,
  overlay: Record<string, string> | null = null,
): ResolvedTheme {
  const resolved = resolveTheme(preference, systemPrefersDark);
  root.dataset.theme = resolved;
  root.dataset.themePreference = preference;
  root.style.colorScheme = resolved;
  applyThemeTokens(resolveAppliedTokens(resolved, overlay), root);
  return resolved;
}
