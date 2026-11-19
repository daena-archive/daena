import { derived, get } from "svelte/store";
import { displayLanguage, setDisplayLanguage } from "../../src/lib/i18n/index.ts";
import { uiMessage, type UiMessageKey } from "./ui-catalogs.ts";

export { displayLanguage };

export function createPluginTranslator(context?: string) {
  const pluginText = derived(displayLanguage, (language) => (english: string, values?: Record<string, string | number>) => {
    const text = uiMessage(language, english, context);
    return values ? text.replace(/\{(\w+)\}/g, (placeholder, key: string) => String(values[key] ?? placeholder)) : text;
  });

  function translatePluginText(english: UiMessageKey, values?: Record<string, string | number>): string {
    return get(pluginText)(english, values);
  }
  return { pluginText, translatePluginText };
}

export const { pluginText, translatePluginText } = createPluginTranslator();

/** Native plugin windows run on their own origins. The shell owns their display language. */
export function listenForPluginLanguage(title: UiMessageKey) {
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
