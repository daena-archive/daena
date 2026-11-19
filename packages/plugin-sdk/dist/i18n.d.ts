/** Flat message dictionaries from this package's locales/daena/<language>.json files. */
export type PluginCatalogs = Readonly<Record<string, Readonly<Record<string, string>>>>;
export declare function translatePluginMessage(catalogs: PluginCatalogs, language: string, source: string): string;
/** Read and observe the app language without opening an RPC session. */
export declare function createPluginI18n(): {
    readonly language: string;
    t(source: string, values?: Readonly<Record<string, string | number>>): string;
    /** Invokes the listener immediately and on language changes. Returns an unsubscribe function. */
    subscribe(listener: (language: string) => void): () => void;
};
//# sourceMappingURL=i18n.d.ts.map