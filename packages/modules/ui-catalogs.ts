import japanese from "../../src/lib/i18n/ui-ja.json" with { type: "json" };
import type { DisplayLanguage } from "../../src/lib/i18n/index.ts";

const catalogs: Record<DisplayLanguage, Readonly<Record<string, string>>> = { en: {}, ja: japanese };

export type UiMessageKey = keyof typeof japanese;

export function uiMessage(language: DisplayLanguage, source: string, context?: string): string {
  const catalog = catalogs[language];
  if (context) {
    const key = `${context}.${source}`;
    if (Object.hasOwn(catalog, key)) return catalog[key];
  }
  return Object.hasOwn(catalog, source) ? catalog[source] : source;
}
