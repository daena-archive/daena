<script lang="ts">
import { pluginText } from "../i18n.ts";
import { CUSTOM_RULE_TAGS, extraCustomRuleTags, setCustomRuleExtraTags, toggleCustomRuleTag } from "../grammar/rules";
import type { GrammarCustomRuleRecord } from "../grammar/types";
import RichTextEditor from "../../../../../src/lib/editor/RichTextEditor.svelte";
import CheckRow from "./parts/CheckRow.svelte";
import Field from "./parts/Field.svelte";
import Group from "./parts/Group.svelte";

let {
  draft,
  locked = false,
  entityId = null,
}: { draft: GrammarCustomRuleRecord; locked?: boolean; entityId?: string | null } = $props();

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
    {$pluginText("Use this for grammatical features that do not fit Daena's built-in grammar systems. If a feature becomes common enough, it may eventually deserve its own dedicated editor.")}
  </p>
  <Field label={$pluginText("Title")}>
    <input name="title" type="text" bind:value={draft.title} disabled={locked} />
  </Field>
  <CheckRow
    name="tags"
    legend={$pluginText("Category / tags (optional)")}
    selected={draft.tags}
    {locked}
    options={CUSTOM_RULE_TAGS.map((tag) => ({ value: tag, label: tag }))}
    ontoggle={(value) => updateTags(toggleCustomRuleTag(draft, value).tags)} />
  <Field label={$pluginText("Additional tags")}>
    <input
      name="tags"
      type="text"
      value={extra}
      placeholder={$pluginText("Other tags, comma-separated")}
      disabled={locked}
      oninput={(event) => updateTags(setCustomRuleExtraTags(draft, event.currentTarget.value).tags)} />
  </Field>
  <Field label={$pluginText("Description")}>
    <RichTextEditor
      value={draft.body}
      onChange={(v) => (draft.body = v)}
      editable={!locked}
      {entityId}
      defaultNamespace="language" />
  </Field>
</Group>
