export function translatePluginMessage(catalogs, language, source) {
    let tag = language.toLowerCase();
    while (tag) {
        const messages = Object.hasOwn(catalogs, tag) ? catalogs[tag] : undefined;
        if (messages && Object.hasOwn(messages, source))
            return messages[source];
        const separator = tag.lastIndexOf("-");
        if (separator < 0)
            break;
        tag = tag.slice(0, separator);
    }
    return source;
}
/** Read and observe the app language without opening an RPC session. */
export function createPluginI18n() {
    const host = typeof window === "undefined" ? undefined : window;
    const language = () => host?.__DAENA_DISPLAY_LANGUAGE__ ?? "en";
    return {
        get language() { return language(); },
        t(source, values) {
            const text = translatePluginMessage(host?.__DAENA_PLUGIN_CATALOGS__ ?? {}, language(), source);
            return values ? text.replace(/\{(\w+)\}/g, (token, key) => Object.hasOwn(values, key) ? String(values[key]) : token) : text;
        },
        /** Invokes the listener immediately and on language changes. Returns an unsubscribe function. */
        subscribe(listener) {
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
//# sourceMappingURL=i18n.js.map