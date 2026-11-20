import { derived, writable } from "svelte/store";
import { en } from "./en.ts";
import { ja } from "./ja.ts";
import japaneseMenu from "./menu-ja.json" with { type: "json" };

export const languageOptions = [
  { value: "en", label: "English", status: "stable" },
  { value: "ja", label: "日本語", status: "alpha" },
] as const;

export type DisplayLanguage = (typeof languageOptions)[number]["value"];

const FALLBACK_LANGUAGE = "en" satisfies DisplayLanguage;
const catalogs = { en, ja } satisfies Record<DisplayLanguage, typeof en>;
export const menuCatalogs: Record<DisplayLanguage, Record<string, string>> = { en: {}, ja: japaneseMenu };
const storageKey = "daena:display-language";

function normalizeDisplayLanguage(value: unknown): DisplayLanguage {
  return languageOptions.find((language) => language.value === value)?.value ?? FALLBACK_LANGUAGE;
}

function readDisplayLanguage(): DisplayLanguage {
  try {
    return normalizeDisplayLanguage(globalThis.localStorage?.getItem(storageKey));
  } catch {
    return FALLBACK_LANGUAGE;
  }
}

const language = writable<DisplayLanguage>(readDisplayLanguage());
export const displayLanguage = { subscribe: language.subscribe };
export const messages = derived(language, (value) => catalogs[value]);

export function setDisplayLanguage(value: string): void {
  const next = normalizeDisplayLanguage(value);
  language.set(next);
  try {
    globalThis.localStorage?.setItem(storageKey, next);
  } catch {
    // The active window can still use the new language.
  }
}
