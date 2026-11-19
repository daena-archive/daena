import { derived, get } from "svelte/store";
import { displayLanguage, type DisplayLanguage } from "./index.ts";
import japanese from "./native-errors.ja.json" with { type: "json" };

// CoreError and Tauri return display strings. Match only complete messages that Daena
// owns. Values captured from a path, revision, OS error, or provider stay opaque.
function compileNativeCatalog(literals: Readonly<Record<string, string>>) {
  const templates = Object.entries(literals)
    .filter(([source]) => source.includes("{0}"))
    .map(([source, translation]) => ({
      parts: source.split(/\{\d+\}/),
      translation,
    }))
    .sort((a, b) => b.parts.join("").length - a.parts.join("").length);

  return function translateMessage(message: string): string | undefined {
    if (Object.hasOwn(literals, message)) return literals[message];
    for (const { parts, translation } of templates) {
      const last = parts.length - 1;
      if (!message.startsWith(parts[0]) || !message.endsWith(parts[last])) continue;
      // Walk literal separators once. Captured paths and diagnostics may be long
      // and can contain repeated punctuation, so avoid regex backtracking.
      let start = parts[0].length;
      const values: string[] = [];
      for (let i = 1; i <= last; i++) {
        const end = i === last ? message.length - parts[i].length : message.indexOf(parts[i], start + 1);
        if (end <= start) break;
        values.push(message.slice(start, end));
        start = end + parts[i].length;
      }
      if (values.length === last) {
        return translation.replace(/\{(\d+)\}/g, (_, i: string) => values[Number(i)]);
      }
    }
  };
}

const catalogs = {
  en: compileNativeCatalog({}),
  ja: compileNativeCatalog(japanese),
} satisfies Record<DisplayLanguage, ReturnType<typeof compileNativeCatalog>>;

export function formatNativeError(language: DisplayLanguage, message: string): string {
  const translateMessage = catalogs[language];
  // These wrappers are produced by Daena around its own validation errors.
  // Provider responses, OS errors and path arguments in other templates stay opaque.
  function translateDetail(text: string, depth: number): string | undefined {
    if (depth < 4) {
      for (const prefix of ["invalid settings: ", "internal error: ", "routing validation failed: "]) {
        if (!text.startsWith(prefix)) continue;
        const detail = translateDetail(text.slice(prefix.length), depth + 1);
        if (detail !== undefined) return translateMessage(prefix + detail);
      }
    }
    return translateMessage(text);
  }
  // Preserve diagnostic codes and the storage codec's file path verbatim.
  const envelope = /^([\s\S]+ \[[\w.-]+\] |[a-z][\w-]*(?:\.[\w-]+)+: )([\s\S]+)$/.exec(message);
  if (envelope) {
    const detail = translateDetail(envelope[2], 0);
    if (detail !== undefined) return envelope[1] + detail;
  }
  const translated = translateDetail(message, 0);
  if (translated !== undefined) return translated;
  return message;
}

export const nativeErrorText = derived(
  displayLanguage,
  (language) => (message: string) => formatNativeError(language, message),
);

export function nativeErrorMessage(message: string): string {
  return formatNativeError(get(displayLanguage), message);
}
