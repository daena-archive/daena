import { MAX_TAGS } from "./normalize.ts";
import type { GrammarCustomRuleRecord } from "./types.ts";

export const CUSTOM_RULE_TAGS = [
  "syntax",
  "morphology",
  "phonology interaction",
  "discourse",
  "irregularity",
  "historical",
  "custom",
] as const;

export function toggleCustomRuleTag(draft: GrammarCustomRuleRecord, tag: string): GrammarCustomRuleRecord {
  const present = draft.tags.includes(tag);
  const tags = present
    ? draft.tags.filter((item) => item !== tag)
    : draft.tags.length >= MAX_TAGS
      ? draft.tags
      : [...draft.tags, tag];
  return { ...draft, tags };
}

export function setCustomRuleExtraTags(draft: GrammarCustomRuleRecord, extra: string): GrammarCustomRuleRecord {
  const suggested = new Set<string>(CUSTOM_RULE_TAGS);
  const kept = draft.tags.filter((item) => suggested.has(item));
  const extras = extra
    .split(",")
    .map((item) => item.trim())
    .filter((item) => item && !suggested.has(item));
  return { ...draft, tags: [...kept, ...extras].slice(0, MAX_TAGS) };
}

export function extraCustomRuleTags(draft: GrammarCustomRuleRecord) {
  const suggested = new Set<string>(CUSTOM_RULE_TAGS);
  return draft.tags.filter((item) => !suggested.has(item)).join(", ");
}
