import { derived, get } from "svelte/store";
import { displayLanguage, setDisplayLanguage, type DisplayLanguage } from "../../src/lib/i18n/index.ts";
import { pluginJapanese } from "./plugin-ja.ts";

type PluginText = keyof typeof pluginJapanese;
const pluginCatalogs: Record<DisplayLanguage, Readonly<Record<string, string>>> = { en: {}, ja: pluginJapanese };
export const pluginText = derived(
  displayLanguage,
  (language) => (english: string, values?: Record<string, string | number>) => {
    const catalog = pluginCatalogs[language];
    const text = Object.hasOwn(catalog, english) ? catalog[english] : english;
    return values ? text.replace(/\{(\w+)\}/g, (placeholder, key: string) => String(values[key] ?? placeholder)) : text;
  },
);

export function translatePluginText(english: PluginText, values?: Record<string, string | number>): string {
  return get(pluginText)(english, values);
}

/** Native plugin windows run on their own origins. The shell owns their display language. */
export function listenForPluginLanguage(title: PluginText) {
  function applyLanguage() {
    const language = (window as Window & { __DAENA_DISPLAY_LANGUAGE__?: string }).__DAENA_DISPLAY_LANGUAGE__;
    if (language) {
      setDisplayLanguage(language);
      document.documentElement.lang = language;
    }
    document.title = translatePluginText(title);
  }
  applyLanguage();
  window.addEventListener("daena:language", applyLanguage);
}
