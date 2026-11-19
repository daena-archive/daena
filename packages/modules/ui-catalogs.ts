import japanese from "../../src/lib/i18n/ui-ja.json" with { type: "json" };
import type { DisplayLanguage } from "../../src/lib/i18n/index.ts";

const catalogs: Record<DisplayLanguage, Readonly<Record<string, string>>> = { en: {}, ja: japanese };

export function uiMessage(language: DisplayLanguage, source: string): string {
  const catalog = catalogs[language];
  return Object.hasOwn(catalog, source) ? catalog[source] : source;
}
