<script lang="ts">
import { CUSTOM_RULE_TAGS, extraCustomRuleTags, setCustomRuleExtraTags, toggleCustomRuleTag } from "../grammar/rules";
import type { GrammarCustomRuleRecord } from "../grammar/types";
import CheckRow from "./parts/CheckRow.svelte";
import Field from "./parts/Field.svelte";
import Group from "./parts/Group.svelte";

let { draft, locked = false }: { draft: GrammarCustomRuleRecord; locked?: boolean } = $props();

function initialExtras() {
  return extraCustomRuleTags(draft);
}

let extra = $state(initialExtras());

function updateTags(next: string[]) {
  draft.tags = next;
  extra = extraCustomRuleTags(draft);
}
</script>

<Group>
  <p class="language-empty" role="status">
    Use this for grammatical features that do not fit Daena's built-in grammar systems. If a feature becomes common
    enough, it may eventually deserve its own dedicated editor.
  </p>
  <Field label="Title">
    <input name="title" type="text" bind:value={draft.title} disabled={locked} />
  </Field>
  <CheckRow
    name="tags"
    legend="Category / tags (optional)"
    selected={draft.tags}
    {locked}
    options={CUSTOM_RULE_TAGS.map((tag) => ({ value: tag, label: tag }))}
    ontoggle={(value) => updateTags(toggleCustomRuleTag(draft, value).tags)} />
  <Field label="Additional tags">
    <input
      name="tags"
      type="text"
      value={extra}
      placeholder="Other tags, comma-separated"
      disabled={locked}
      oninput={(event) => updateTags(setCustomRuleExtraTags(draft, event.currentTarget.value).tags)} />
  </Field>
  <Field label="Description">
    <textarea name="body" rows="8" bind:value={draft.body} disabled={locked}></textarea>
  </Field>
</Group>
