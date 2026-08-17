<script lang="ts">
import {
  BEHAVIOR_OPTIONS,
  CONTROLLER_OPTIONS,
  TARGET_OPTIONS,
  addCustomAgreementFeature,
  brokenAgreementFeatures,
  endpointLabel,
  groupSelected,
  offeredAgreementGroups,
  removeAgreementFeature,
  setAgreementBehavior,
  setAgreementController,
  setAgreementEndpointLabel,
  setAgreementTarget,
  toggleAgreementGroup,
} from "../grammar/agreement";
import type { AgreementBehavior, AgreementControllerKind, AgreementTargetKind } from "../grammar/types";
import type { GrammarAgreementRecord, IndexedGrammar } from "../grammar/types";
import ChoiceCards from "./parts/ChoiceCards.svelte";
import Field from "./parts/Field.svelte";
import Group from "./parts/Group.svelte";
import TemplateChecks from "./parts/TemplateChecks.svelte";

let {
  draft,
  locked = false,
  index,
}: {
  draft: GrammarAgreementRecord;
  locked?: boolean;
  index: IndexedGrammar;
} = $props();

const groups = $derived(offeredAgreementGroups(index));
const broken = $derived(brokenAgreementFeatures(index, draft));
const selected = $derived.by(() => {
  const picked = new Set<string>();
  for (const group of groups) {
    if (groupSelected(draft, group)) picked.add(group.id);
  }
  return picked;
});

function apply(next: GrammarAgreementRecord) {
  Object.assign(draft, next);
}
</script>

<Group>
  <p class="language-empty" role="status">
    Which element determines the grammatical features? Which element changes to match it?
  </p>
  <Field label="Title"><input name="title" type="text" bind:value={draft.title} disabled={locked} /></Field>
  <ChoiceCards
    name="controller"
    legend="Controller"
    options={CONTROLLER_OPTIONS}
    value={draft.controller.kind}
    {locked}
    onselect={(value) => apply(setAgreementController(draft, value as AgreementControllerKind))} />
  {#if draft.controller.kind === "custom"}
    <Field label="Custom controller">
      <input
        name="controllerCustom"
        type="text"
        value={draft.controller.customLabel ?? ""}
        disabled={locked}
        oninput={(event) => apply(setAgreementEndpointLabel(draft, "controller", event.currentTarget.value))} />
    </Field>
  {/if}
  <ChoiceCards
    name="target"
    legend="Target"
    options={TARGET_OPTIONS}
    value={draft.target.kind}
    {locked}
    onselect={(value) => apply(setAgreementTarget(draft, value as AgreementTargetKind))} />
  {#if draft.target.kind === "custom"}
    <Field label="Custom target">
      <input
        name="targetCustom"
        type="text"
        value={draft.target.customLabel ?? ""}
        disabled={locked}
        oninput={(event) => apply(setAgreementEndpointLabel(draft, "target", event.currentTarget.value))} />
    </Field>
  {/if}
  {#if groups.length === 0}
    <p class="language-empty" role="status">
      Configure number, case, classes, or pronouns first to reuse those categories here.
    </p>
  {:else}
    <TemplateChecks
      legend={`${endpointLabel(draft.target)} agrees with ${endpointLabel(draft.controller)} in`}
      options={groups.map((group) => ({ id: group.id, label: group.label }))}
      {selected}
      {locked}
      ontoggle={(id) => {
        const group = groups.find((item) => item.id === id);
        if (group) apply(toggleAgreementGroup(draft, group));
      }} />
  {/if}
  {#each draft.features as feature, index (index)}
    {#if !feature.sourceSystemId}
      <Field label="Custom feature">
        <div class="grammar-inventory-toolbar">
          <input name="customFeature" type="text" bind:value={feature.label} disabled={locked} />
          <button
            type="button"
            class="language-button secondary language-danger"
            disabled={locked}
            onclick={() => apply(removeAgreementFeature(draft, index))}>
            Remove
          </button>
        </div>
      </Field>
    {/if}
  {/each}
  {#if !locked}
    <button type="button" class="language-button secondary" onclick={() => apply(addCustomAgreementFeature(draft))}>
      Add custom feature
    </button>
  {/if}
  {#if broken.length}
    <p class="language-empty" role="status">
      Broken references: {broken.map((item) => item.label).join(", ")}. Edit the owning system to restore them, or
      remove the feature.
    </p>
  {/if}
  <ChoiceCards
    name="behavior"
    legend="Behavior"
    options={BEHAVIOR_OPTIONS}
    value={draft.behavior}
    {locked}
    onselect={(value) => apply(setAgreementBehavior(draft, value as AgreementBehavior))} />
  <Field label="Default form (optional)">
    <input name="defaultForm" type="text" bind:value={draft.defaultForm} disabled={locked} />
  </Field>
  <Field label="Conditions (optional)">
    <textarea rows="3" name="conditions" bind:value={draft.conditions} disabled={locked}></textarea>
  </Field>
  <Field label="Exceptions (optional)">
    <textarea rows="3" name="exceptions" bind:value={draft.exceptions} disabled={locked}></textarea>
  </Field>
</Group>

<style>
.grammar-inventory-toolbar {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 8px;
}
</style>
