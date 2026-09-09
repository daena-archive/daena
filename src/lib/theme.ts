import { BUILTIN_THEME_TOKENS, THEME_TOKEN_IDS, type ThemeTokenId } from "../../packages/plugin-sdk/src/generated.ts";

export type ThemePreference = "light" | "dark" | "system";
export type ResolvedTheme = Exclude<ThemePreference, "system">;
export type ThemeTokenMap = Record<ThemeTokenId, string>;

export const THEME_STORAGE_KEY = "daena-theme";

export type ThemeRoot = {
  dataset: { theme?: string; themePreference?: string };
  style: { colorScheme: string; setProperty(property: string, value: string): void };
};

export function normalizeThemePreference(value: unknown): ThemePreference {
  return value === "light" || value === "dark" || value === "system" ? value : "system";
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

export function resolveBuiltinTokens(resolved: ResolvedTheme): ThemeTokenMap {
  return { ...BUILTIN_THEME_TOKENS[resolved] };
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
): ResolvedTheme {
  const resolved = resolveTheme(preference, systemPrefersDark);
  root.dataset.theme = resolved;
  root.dataset.themePreference = preference;
  root.style.colorScheme = resolved;
  applyThemeTokens(resolveBuiltinTokens(resolved), root);
  return resolved;
}
