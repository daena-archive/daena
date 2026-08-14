import { emptyMessage, field, input, textarea } from "../ui.ts";
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
  const tags = present ? draft.tags.filter((item) => item !== tag) : draft.tags.length >= MAX_TAGS ? draft.tags : [...draft.tags, tag];
  return { ...draft, tags };
}

export function setCustomRuleTitle(draft: GrammarCustomRuleRecord, title: string): GrammarCustomRuleRecord {
  return { ...draft, title };
}

export function setCustomRuleBody(draft: GrammarCustomRuleRecord, body: string): GrammarCustomRuleRecord {
  return { ...draft, body };
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

export function renderCustomRuleEditor(
  draft: GrammarCustomRuleRecord,
  locked: boolean,
  onChange: (next: GrammarCustomRuleRecord, rerender: boolean) => void,
): HTMLElement {
  const section = document.createElement("section");
  section.className = "language-group grammar-choice-stack";
  section.append(
    emptyMessage(
      "Use this for grammatical features that do not fit Daena's built-in grammar systems. If a feature becomes common enough, it may eventually deserve its own dedicated editor.",
    ),
  );
  const title = input("title", draft.title);
  title.disabled = locked;
  title.oninput = () => onChange(setCustomRuleTitle(draft, title.value), false);
  section.append(field("Title", title));
  const tags = document.createElement("fieldset");
  tags.className = "grammar-checks";
  const legend = document.createElement("legend");
  legend.textContent = "Category / tags (optional)";
  tags.append(legend);
  for (const tag of CUSTOM_RULE_TAGS) {
    const label = document.createElement("label");
    const box = document.createElement("input");
    box.type = "checkbox";
    box.checked = draft.tags.includes(tag);
    box.disabled = locked;
    box.onchange = () => onChange(toggleCustomRuleTag(draft, tag), true);
    label.append(box, ` ${tag}`);
    tags.append(label);
  }
  section.append(tags);
  const extra = input("tags", extraCustomRuleTags(draft));
  extra.disabled = locked;
  extra.placeholder = "Other tags, comma-separated";
  extra.oninput = () => onChange(setCustomRuleExtraTags(draft, extra.value), false);
  const body = textarea("body", draft.body, 8);
  body.disabled = locked;
  body.oninput = () => onChange(setCustomRuleBody(draft, body.value), false);
  section.append(field("Additional tags", extra), field("Description", body));
  return section;
}
