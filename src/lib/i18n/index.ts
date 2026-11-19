import { derived, writable } from "svelte/store";
import { en } from "./en.ts";
import { ja } from "./ja.ts";

export const languageOptions = [
  { value: "en", label: "English" },
  { value: "ja", label: "日本語" },
] as const;

export type DisplayLanguage = (typeof languageOptions)[number]["value"];

const catalogs = { en, ja } satisfies Record<DisplayLanguage, typeof en>;
const storageKey = "daena:display-language";

function normalizeDisplayLanguage(value: unknown): DisplayLanguage {
  return value === "ja" ? "ja" : "en";
}

function readDisplayLanguage(): DisplayLanguage {
  try {
    return normalizeDisplayLanguage(globalThis.localStorage?.getItem(storageKey));
  } catch {
    return "en";
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
