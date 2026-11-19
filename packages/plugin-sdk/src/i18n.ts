/** Flat message dictionaries from this package's locales/<language>.json files. */
export type PluginCatalogs = Readonly<Record<string, Readonly<Record<string, string>>>>;

export function translatePluginMessage(catalogs: PluginCatalogs, language: string, source: string): string {
  let tag = language.toLowerCase();
  while (tag) {
    const messages = Object.hasOwn(catalogs, tag) ? catalogs[tag] : undefined;
    if (messages && Object.hasOwn(messages, source)) return messages[source];
    const separator = tag.lastIndexOf("-");
    if (separator < 0) break;
    tag = tag.slice(0, separator);
  }
  return source;
}

type PluginWindow = Window & {
  __DAENA_DISPLAY_LANGUAGE__?: string;
  __DAENA_PLUGIN_CATALOGS__?: PluginCatalogs;
};

/** Read and observe the app language without opening an RPC session. */
export function createPluginI18n() {
  const host: PluginWindow | undefined = typeof window === "undefined" ? undefined : window;
  const language = () => host?.__DAENA_DISPLAY_LANGUAGE__ ?? "en";
  return {
    get language() { return language(); },
    t(source: string, values?: Readonly<Record<string, string | number>>): string {
      const text = translatePluginMessage(host?.__DAENA_PLUGIN_CATALOGS__ ?? {}, language(), source);
      return values ? text.replace(/\{(\w+)\}/g, (token, key: string) =>
        Object.hasOwn(values, key) ? String(values[key]) : token) : text;
    },
    /** Invokes the listener immediately and on language changes. Returns an unsubscribe function. */
    subscribe(listener: (language: string) => void): () => void {
      let current = language();
      const update = () => {
        const next = language();
        if (current !== next) {
          current = next;
          listener(next);
        }
      };
      host?.addEventListener("daena:language", update);
      listener(current);
      return () => host?.removeEventListener("daena:language", update);
    },
  };
}
