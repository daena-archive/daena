<script lang="ts">
import { pluginText } from "../../i18n.ts";
import {
  CASE_TEMPLATES,
  NOUN_CLASS_KIND_OPTIONS,
  NUMBER_MARKING_OPTIONS,
  NUMBER_TEMPLATES,
  addCase,
  addNounClass,
  moveCase,
  moveNumberCategory,
  moveNounClass,
  moveTamCategory,
  removeById,
  removeCase,
  removeNumberCategory,
  removeNounClass,
  removeTamCategory,
  setNounClassKind,
  tamTemplates,
  toggleNumberMarking,
  toggleNumberTemplate,
  toggleTamTemplate,
} from "../../grammar/inventory";
import type { InventoryMutation } from "../../grammar/inventory";
import type {
  CaseConfig,
  CaseTemplateId,
  GrammarSystemRecord,
  MarkingStrategy,
  NounClassKind,
  NounClassesConfig,
  NumberCategoryId,
  NumberConfig,
  TamConfig,
} from "../../grammar/types";
import { applySystemMutation } from "../../grammar/session";
import ChoiceCards from "../parts/ChoiceCards.svelte";
import EditorButton from "../parts/EditorButton.svelte";
import Field from "../parts/Field.svelte";
import InventoryItem from "../parts/InventoryItem.svelte";
import TemplateChecks from "../parts/TemplateChecks.svelte";

let {
  draft,
  locked = false,
  referencedIds,
  confirm,
}: {
  draft: GrammarSystemRecord;
  locked?: boolean;
  referencedIds: Set<string>;
  confirm: (message: string) => Promise<boolean>;
} = $props();

async function applyRemoval(result: InventoryMutation) {
  if (result.blocked) {
    if (
      !(await confirm(
        `“${result.blocked.label}” is referenced by agreement. Remove it anyway? Agreement will keep the broken reference until you edit it.`,
      ))
    ) {
      return;
    }
    applySystemMutation(draft, removeById(result.draft, result.blocked.id, { force: true }).draft);
    return;
  }
  applySystemMutation(draft, result.draft);
}
</script>

<section class="language-group grammar-inventory">
  <div class="grammar-choice-stack">
    {#if draft.systemId === "nouns.number"}
      {@const config = draft.config as NumberConfig}
      <TemplateChecks
        legend={$pluginText("Number categories")}
        options={NUMBER_TEMPLATES.filter((item) => item.id !== "custom")}
        selected={new Set(config.categories.map((item) => item.templateId).filter(Boolean))}
        {locked}
        ontoggle={(templateId) =>
          applyRemoval(toggleNumberTemplate(draft, templateId as NumberCategoryId, { referenced: referencedIds }))} />
      {#if !locked}
        <EditorButton secondary onclick={() => applyRemoval(toggleNumberTemplate(draft, "custom"))}>
          {$pluginText("Add custom category")}
        </EditorButton>
      {/if}
      <TemplateChecks
        legend={$pluginText("How is number usually expressed?")}
        options={NUMBER_MARKING_OPTIONS}
        selected={new Set(config.markingStrategies)}
        {locked}
        ontoggle={(id) => {
          applySystemMutation(draft, toggleNumberMarking(draft, id as MarkingStrategy));
        }} />
      {#each config.categories as item, index (item.id)}
        <InventoryItem
          title={item.label || $pluginText("Number category")}
          {index}
          total={config.categories.length}
          referenced={referencedIds.has(item.id)}
          {locked}
          onmove={(delta) => {
            applySystemMutation(draft, moveNumberCategory(draft, item.id, delta));
          }}
          onremove={() => applyRemoval(removeNumberCategory(draft, item.id, { referenced: referencedIds }))}>
          <Field label={$pluginText("Label")}><input name="label" type="text" bind:value={item.label} disabled={locked} /></Field>
          <Field label={$pluginText("Meaning")}
            ><input name="meaning" type="text" bind:value={item.meaning} disabled={locked} /></Field>
          <Field label={$pluginText("Marker")}><input name="marker" type="text" bind:value={item.marker} disabled={locked} /></Field>
          <Field label={$pluginText("Position")}
            ><input name="position" type="text" bind:value={item.position} disabled={locked} /></Field>
          <Field label={$pluginText("Notes")}
            ><textarea rows="2" name="notes" bind:value={item.notes} disabled={locked}></textarea></Field>
        </InventoryItem>
      {/each}
    {:else if draft.systemId === "nouns.case"}
      {@const config = draft.config as CaseConfig}
      <p class="language-empty" role="status">{$pluginText("Case names are convenient labels, not universal exact meanings.")}</p>
      {#if !locked}
        <select
          aria-label={$pluginText("Add a case")}
          onchange={(event) => {
            const value = event.currentTarget.value;
            if (!value) return;
            applySystemMutation(draft, addCase(draft, value as CaseTemplateId));
          }}>
          <option value="">{$pluginText("Add a case…")}</option>
          {#each CASE_TEMPLATES as template (template.id)}
            <option value={template.id}>{$pluginText(template.label)}</option>
          {/each}
        </select>
      {/if}
      {#each config.cases as item, index (item.id)}
        <InventoryItem
          title={item.name || $pluginText("Case")}
          {index}
          total={config.cases.length}
          referenced={referencedIds.has(item.id)}
          {locked}
          onmove={(delta) => {
            applySystemMutation(draft, moveCase(draft, item.id, delta));
          }}
          onremove={() => applyRemoval(removeCase(draft, item.id, { referenced: referencedIds }))}>
          <Field label={$pluginText("Name")}><input name="name" type="text" bind:value={item.name} disabled={locked} /></Field>
          <Field label={$pluginText("Abbreviation")}>
            <input name="abbreviation" type="text" bind:value={item.abbreviation} disabled={locked} />
          </Field>
          <Field label={$pluginText("Primary function")}>
            <textarea rows="2" name="primaryFunction" bind:value={item.primaryFunction} disabled={locked}></textarea>
          </Field>
          <Field label={$pluginText("Additional functions")}>
            <textarea rows="2" name="additionalFunctions" bind:value={item.additionalFunctions} disabled={locked}
            ></textarea>
          </Field>
          <Field label={$pluginText("How it is marked")}>
            <input name="marking" type="text" bind:value={item.marking} disabled={locked} />
          </Field>
          <Field label={$pluginText("Notes")}
            ><textarea rows="2" name="notes" bind:value={item.notes} disabled={locked}></textarea></Field>
        </InventoryItem>
      {/each}
    {:else if draft.systemId === "nouns.classes"}
      {@const config = draft.config as NounClassesConfig}
      <p class="language-empty" role="status">
        {$pluginText("If this language has no grammatical classes, mark the system as not used. Agreement behavior belongs under Agreement.")}
      </p>
      <ChoiceCards
        name="kind"
        legend={$pluginText("What kind of classification is this?")}
        options={NOUN_CLASS_KIND_OPTIONS.map((option) => ({
          value: option.id,
          label: option.label,
          expansion: option.meaning,
        }))}
        value={config.kind}
        {locked}
        onselect={(kind) => {
          applySystemMutation(draft, setNounClassKind(draft, kind as NounClassKind));
        }} />
      {#if !locked}
        <EditorButton
          secondary
          onclick={() => {
            applySystemMutation(draft, addNounClass(draft));
          }}>
          {$pluginText("Add class")}
        </EditorButton>
      {/if}
      {#each config.classes ?? [] as item, index (item.id)}
        <InventoryItem
          title={item.name || $pluginText("Class")}
          {index}
          total={config.classes.length}
          referenced={referencedIds.has(item.id)}
          {locked}
          onmove={(delta) => {
            applySystemMutation(draft, moveNounClass(draft, item.id, delta));
          }}
          onremove={() => applyRemoval(removeNounClass(draft, item.id, { referenced: referencedIds }))}>
          <Field label={$pluginText("Name")}><input name="name" type="text" bind:value={item.name} disabled={locked} /></Field>
          <Field label={$pluginText("Abbreviation")}>
            <input name="abbreviation" type="text" bind:value={item.abbreviation} disabled={locked} />
          </Field>
          <Field label={$pluginText("Typical membership")}>
            <textarea rows="2" name="membership" bind:value={item.membership} disabled={locked}></textarea>
          </Field>
          <Field label={$pluginText("Exceptions")}>
            <textarea rows="2" name="exceptions" bind:value={item.exceptions} disabled={locked}></textarea>
          </Field>
        </InventoryItem>
      {/each}
    {:else}
      {@const config = draft.config as TamConfig}
      {@const selected = new Set(config.categories.map((item) => item.templateId).filter(Boolean))}
      {@const common = tamTemplates(draft.systemId).filter((item) => !item.more && item.id !== "custom")}
      {@const extra = tamTemplates(draft.systemId).filter((item) => item.more)}
      <TemplateChecks
        legend={$pluginText("Categories")}
        options={common}
        {selected}
        {locked}
        ontoggle={(templateId) => applyRemoval(toggleTamTemplate(draft, templateId, { referenced: referencedIds }))} />
      {#if extra.length}
        <details class="grammar-learn" open={extra.some((item) => selected.has(item.id))}>
          <summary>{$pluginText("More")}</summary>
          <TemplateChecks
            legend={$pluginText("Additional categories")}
            options={extra}
            {selected}
            {locked}
            ontoggle={(templateId) =>
              applyRemoval(toggleTamTemplate(draft, templateId, { referenced: referencedIds }))} />
        </details>
      {/if}
      {#if !locked}
        <EditorButton secondary onclick={() => applyRemoval(toggleTamTemplate(draft, "custom"))}>
          {$pluginText("Add custom category")}
        </EditorButton>
      {/if}
      {#each config.categories as item, index (item.id)}
        <InventoryItem
          title={item.label || $pluginText("Category")}
          {index}
          total={config.categories.length}
          referenced={referencedIds.has(item.id)}
          {locked}
          onmove={(delta) => {
            applySystemMutation(draft, moveTamCategory(draft, item.id, delta));
          }}
          onremove={() => applyRemoval(removeTamCategory(draft, item.id, { referenced: referencedIds }))}>
          <Field label={$pluginText("Label")}><input name="label" type="text" bind:value={item.label} disabled={locked} /></Field>
          <Field label={$pluginText("Meaning")}>
            <textarea rows="2" name="meaning" bind:value={item.meaning} disabled={locked}></textarea>
          </Field>
          <Field label={$pluginText("Marker or construction")}>
            <input name="marker" type="text" bind:value={item.marker} disabled={locked} />
          </Field>
          <Field label={$pluginText("Interaction notes")}>
            <textarea rows="2" name="interaction" bind:value={item.interaction} disabled={locked}></textarea>
          </Field>
          <Field label={$pluginText("Notes")}
            ><textarea rows="2" name="notes" bind:value={item.notes} disabled={locked}></textarea></Field>
        </InventoryItem>
      {/each}
    {/if}
  </div>
</section>

<style>
select {
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
.grammar-inventory {
  display: grid;
  gap: 10px;
  min-width: 0;
}
.grammar-choice-stack {
  display: grid;
  gap: 12px;
  min-width: 0;
}
.grammar-learn {
  margin: 4px 0 8px;
}
.grammar-learn summary:focus-visible {
  outline: 3px solid rgba(180, 119, 63, 0.24);
  outline-offset: 2px;
}
</style>
