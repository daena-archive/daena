import { Extension } from "@tiptap/core";
import { Plugin, PluginKey, type EditorState, type Transaction } from "@tiptap/pm/state";
import { Decoration, DecorationSet } from "@tiptap/pm/view";

export type SearchState = {
  query: string;
  caseSensitive: boolean;
  wholeWord: boolean;
  useRegex: boolean;
  decorations: DecorationSet;
  matches: Array<{ from: number; to: number }>;
  activeIndex: number;
};

export const searchPluginKey = new PluginKey<SearchState>("search");

export function escapeRegExp(s: string) {
  return s.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}

export function buildSearchRegex(query: string, caseSensitive: boolean, wholeWord: boolean, useRegex: boolean): RegExp | null {
  if (!query) return null;
  try {
    let pattern = useRegex ? query : escapeRegExp(query);
    if (wholeWord) pattern = `\\b${pattern}\\b`;
    return new RegExp(pattern, caseSensitive ? "g" : "gi");
  } catch {
    return null;
  }
}

export function findMatches(
  doc: any,
  query: string,
  caseSensitive: boolean,
  wholeWord: boolean,
  useRegex: boolean,
): Array<{ from: number; to: number }> {
  const regex = buildSearchRegex(query, caseSensitive, wholeWord, useRegex);
  if (!regex) return [];
  const matches: Array<{ from: number; to: number }> = [];
  doc.descendants((node: any, pos: number) => {
    if (!node.isText || !node.text) return;
    const text = node.text as string;
    let m: RegExpExecArray | null;
    regex.lastIndex = 0;
    while ((m = regex.exec(text))) {
      if (m[0].length === 0) {
        regex.lastIndex++;
        continue;
      }
      const from = pos + m.index;
      const to = from + m[0].length;
      matches.push({ from, to });
      if (m[0].length === 0) break;
    }
  });
  return matches;
}

export function createDecorations(doc: any, matches: Array<{ from: number; to: number }>, activeIndex: number): DecorationSet {
  if (matches.length === 0) return DecorationSet.empty;
  const decos = matches.map((m, idx) =>
    Decoration.inline(m.from, m.to, { class: idx === activeIndex ? "search-match-active" : "search-match" }),
  );
  return DecorationSet.create(doc, decos);
}

export function createSearchPlugin() {
  return new Plugin<SearchState>({
    key: searchPluginKey,
    state: {
      init(): SearchState {
        return {
          query: "",
          caseSensitive: false,
          wholeWord: false,
          useRegex: false,
          decorations: DecorationSet.empty,
          matches: [],
          activeIndex: -1,
        };
      },
      apply(tr: Transaction, prev: SearchState, _oldState: EditorState, newState: EditorState): SearchState {
        let query = prev.query;
        let caseSensitive = prev.caseSensitive;
        let wholeWord = prev.wholeWord;
        let useRegex = prev.useRegex;
        let activeIndex = prev.activeIndex;
        const meta = tr.getMeta(searchPluginKey) as
          Partial<SearchState & { activeDelta?: number; setActiveIndex?: number }> | undefined;
        let queryChanged = false;
        if (meta) {
          if (typeof meta.query === "string") {
            query = meta.query;
            queryChanged = true;
          }
          if (typeof meta.caseSensitive === "boolean") {
            caseSensitive = meta.caseSensitive;
            queryChanged = true;
          }
          if (typeof meta.wholeWord === "boolean") {
            wholeWord = meta.wholeWord;
            queryChanged = true;
          }
          if (typeof meta.useRegex === "boolean") {
            useRegex = meta.useRegex;
            queryChanged = true;
          }
          if (typeof meta.setActiveIndex === "number") {
            activeIndex = meta.setActiveIndex;
          } else if (typeof meta.activeDelta === "number") {
            if (prev.matches.length > 0) {
              activeIndex = (activeIndex + meta.activeDelta + prev.matches.length) % prev.matches.length;
            }
          }
        }
        const docChanged = tr.docChanged;
        const needRecompute = queryChanged || docChanged;
        let matches = prev.matches;
        let decorations = prev.decorations;
        if (needRecompute) {
          if (!query) {
            matches = [];
            decorations = DecorationSet.empty;
            activeIndex = -1;
          } else {
            matches = findMatches(newState.doc, query, caseSensitive, wholeWord, useRegex);
            if (matches.length === 0) {
              activeIndex = -1;
            } else if (activeIndex < 0 || activeIndex >= matches.length) {
              activeIndex = 0;
            } else if (queryChanged) {
              activeIndex = 0;
            }
            decorations = createDecorations(newState.doc, matches, activeIndex);
          }
        } else if (meta && (typeof meta.setActiveIndex === "number" || typeof meta.activeDelta === "number")) {
          decorations = createDecorations(newState.doc, matches, activeIndex);
        } else if (decorations !== DecorationSet.empty) {
          decorations = decorations.map(tr.mapping, tr.doc);
        }
        return { query, caseSensitive, wholeWord, useRegex, decorations, matches, activeIndex };
      },
    },
    props: {
      decorations(state: EditorState) {
        return searchPluginKey.getState(state)?.decorations ?? DecorationSet.empty;
      },
    },
  });
}

export const SearchAndReplace = Extension.create({
  name: "searchAndReplace",
  addProseMirrorPlugins() {
    return [createSearchPlugin()];
  },
});
