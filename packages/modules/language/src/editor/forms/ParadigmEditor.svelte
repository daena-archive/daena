<script lang="ts">
import {
  DISTANCE_VALUES,
  EXTRA_DEMONSTRATIVE_AXES,
  EXTRA_PRONOUN_AXES,
  NUMBER_VALUES,
  PARTICIPANT_OPTIONS,
  PERSON_VALUES,
  REPRESENTATION_OPTIONS,
  addCustomAxis,
  addCustomAxisValue,
  addParadigmAxis,
  paradigmAxes,
  paradigmCells,
  removeAxisValue,
  removeParadigmAxis,
  renameAxisValue,
  setArgumentAgreement,
  setArgumentParticipants,
  setArgumentRepresentation,
  toggleAxisValue,
  toggleDistance,
  updateParadigmCell,
} from "../../grammar/paradigm";
import type { ParadigmMutation } from "../../grammar/paradigm";
import type {
  ArgumentIndexingConfig,
  ArgumentParticipants,
  ArgumentRepresentation,
  DemonstrativeConfig,
  GrammarSystemRecord,
  ParadigmAxis,
  ParadigmCell,
} from "../../grammar/types";
import { applySystemMutation } from "../../grammar/session";
import ChoiceCards from "../parts/ChoiceCards.svelte";
import Field from "../parts/Field.svelte";
import Group from "../parts/Group.svelte";
import ParadigmGrid from "../parts/ParadigmGrid.svelte";
import TemplateChecks from "../parts/TemplateChecks.svelte";

let {
  draft,
  locked = false,
  confirm,
  referencedIds,
  pronounAxes,
  agreements,
}: {
  draft: GrammarSystemRecord;
  locked?: boolean;
  confirm: (message: string) => Promise<boolean>;
  referencedIds: Set<string>;
  pronounAxes?: ParadigmAxis[];
  agreements: { id: string; title: string }[];
} = $props();

const axes = $derived(paradigmAxes(draft));
const cells = $derived(paradigmCells(draft));

function toChecks(values: { id: string; label: string }[]) {
  return values.map((item) => ({ id: item.id, label: item.label }));
}

async function applyMutation(result: ParadigmMutation) {
  if (result.blocked) {
    const extra = result.blocked.references ? ` ${result.blocked.references} agreement reference(s) will break.` : "";
    if (
      !(await confirm(
        `Removing ${result.blocked.label} will discard ${result.blocked.populated} filled cell(s).${extra} Continue?`,
      ))
    ) {
      return;
    }
    if (result.retry) applySystemMutation(draft, result.retry());
    return;
  }
  applySystemMutation(draft, result.draft);
}

function handleCell(cellId: string, patch: Partial<Omit<ParadigmCell, "id" | "coordinates">>) {
  applySystemMutation(draft, updateParadigmCell(draft, cellId, patch));
}
</script>

<Group>
  {#if draft.systemId === "pronouns.personal"}
    <p class="language-empty" role="status">
      Start with person and number. Add other distinctions only if the language uses them.
    </p>
    {@render axisChecks("Person", "person", PERSON_VALUES)}
    {@render axisChecks("Number", "number", NUMBER_VALUES)}
    {@render extraAxes(EXTRA_PRONOUN_AXES)}
    <ParadigmGrid {axes} {cells} {locked} oncell={handleCell} />
  {:else if draft.systemId === "pronouns.demonstratives"}
    {@const config = draft.config as DemonstrativeConfig}
    {@const distanceSelected = new Set(
      config.distances ?? axes.find((axis) => axis.id === "distance")?.values.map((item) => item.id),
    )}
    <p class="language-empty" role="status">The grid is generated only from the dimensions you select.</p>
    <TemplateChecks
      legend="Distance distinctions"
      options={toChecks(DISTANCE_VALUES)}
      selected={distanceSelected}
      {locked}
      ontoggle={(id) => {
        const template = DISTANCE_VALUES.find((item) => item.id === id);
        if (template) applyMutation(toggleDistance(draft, template, { referenced: referencedIds }));
      }} />
    {@render extraAxes(EXTRA_DEMONSTRATIVE_AXES.filter((axis) => axis.id !== "distance"))}
    <ParadigmGrid {axes} {cells} {locked} oncell={handleCell} />
  {:else if draft.systemId === "verbs.argument-indexing"}
    {@const config = draft.config as ArgumentIndexingConfig}
    <p class="language-empty" role="status">
      Describe whether the verb changes based on who takes part. This is not always the same as Agreement.
    </p>
    <ChoiceCards
      name="participants"
      legend="Do verbs change based on their participants?"
      options={PARTICIPANT_OPTIONS}
      value={config.participants}
      {locked}
      onselect={(value) => {
        applySystemMutation(draft, setArgumentParticipants(draft, value as ArgumentParticipants, pronounAxes));
      }} />
    {#if config.participants && config.participants !== "none"}
      <ChoiceCards
        name="representation"
        legend="What kind of forms are these?"
        options={REPRESENTATION_OPTIONS}
        value={config.representation}
        {locked}
        onselect={(value) => {
          applySystemMutation(draft, setArgumentRepresentation(draft, value as ArgumentRepresentation));
        }} />
      {#if agreements.length}
        <Field label="Analyze as Agreement (optional)">
          <select
            name="agreementRecordId"
            value={config.agreementRecordId ?? ""}
            disabled={locked}
            onchange={(event) => {
              applySystemMutation(draft, setArgumentAgreement(draft, event.currentTarget.value || undefined));
            }}>
            <option value="">Do not link an Agreement system</option>
            {#each agreements as agreement (agreement.id)}
              <option value={agreement.id}>{agreement.title}</option>
            {/each}
          </select>
        </Field>
      {/if}
      {#if config.representation === "flexible-table"}
        <Field label="Flexible table notes">
          <textarea rows="4" name="flexibleNotes" bind:value={config.flexibleNotes} disabled={locked}></textarea>
        </Field>
      {:else if !config.agreementRecordId}
        {@render axisChecks("Person", "person", PERSON_VALUES)}
        {@render axisChecks("Number", "number", NUMBER_VALUES)}
        {@render extraAxes(EXTRA_PRONOUN_AXES)}
        <ParadigmGrid {axes} {cells} {locked} oncell={handleCell} />
      {:else}
        <p class="language-empty" role="status">
          This display is linked to an Agreement system. Edit the relationship there instead of copying person/number
          rules.
        </p>
      {/if}
    {/if}
  {/if}
</Group>

{#snippet axisChecks(legendText: string, axisId: string, templates: { id: string; label: string }[])}
  {@const axis = axes.find((item) => item.id === axisId)}
  {@const selected = new Set(axis?.values.map((item) => item.id))}
  {@const known = new Set(templates.map((item) => item.id))}
  {@const extras = (axis?.values ?? []).filter((item) => !known.has(item.id))}
  <TemplateChecks
    legend={legendText}
    options={toChecks(templates)}
    {selected}
    {locked}
    ontoggle={(id) => {
      const template = templates.find((item) => item.id === id);
      if (template) applyMutation(toggleAxisValue(draft, axisId, template, { referenced: referencedIds }));
    }} />
  {#each extras as value (value.id)}
    <label>
      <input
        type="checkbox"
        name="axisValue"
        checked
        disabled={locked}
        onchange={() => applyMutation(removeAxisValue(draft, axisId, value.id, { referenced: referencedIds }))} />
      <input name="label" type="text" bind:value={value.label} disabled={locked} />
    </label>
  {/each}
{/snippet}

{#snippet extraAxes(extras: { id: string; label: string; values: { id: string; label: string }[] }[])}
  {#if !locked}
    <select
      aria-label="Add distinction"
      onchange={(event) => {
        const id = event.currentTarget.value;
        if (!id) return;
        applySystemMutation(
          draft,
          id === "custom"
            ? addCustomAxis(draft)
            : addParadigmAxis(
                draft,
                extras.find((item) => item.id === id)!,
              ),
        );
      }}>
      <option value="">Add distinction…</option>
      {#each extras.filter((extra) => !axes.some((axis) => axis.id === extra.id)) as extra (extra.id)}
        <option value={extra.id}>{extra.label}</option>
      {/each}
      <option value="custom">Custom dimension</option>
    </select>
  {/if}
  {#each axes.filter((axis) => {
    if (axis.id === "person" || axis.id === "distance") return false;
    if (axis.id === "number") return extras.some((item) => item.id === "number");
    return true;
  }) as axis (axis.id)}
    {@const extra = extras.find((item) => item.id === axis.id)}
    {@render axisChecks(axis.label, axis.id, extra?.values ?? axis.values)}
    {#if !locked}
      <button
        type="button"
        class="language-button secondary"
        onclick={() => {
          applySystemMutation(draft, addCustomAxisValue(draft, axis.id));
        }}>
        Add value
      </button>
      <button
        type="button"
        class="language-button secondary language-danger"
        onclick={() => applyMutation(removeParadigmAxis(draft, axis.id, { referenced: referencedIds }))}>
        Remove distinction
      </button>
    {/if}
  {/each}
{/snippet}

<style>
select,
input[type="text"] {
  box-sizing: border-box;
  width: 100%;
  min-width: 0;
  padding: 0 10px;
  border: 1px solid var(--theme-neutral-border, var(--line));
  border-radius: 8px;
  background: var(--theme-surface-bg, var(--surface));
  color: var(--ink);
  font: 12px/1.35 var(--font-body, Inter, ui-sans-serif, system-ui, sans-serif);
}
</style>
