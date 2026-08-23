<script lang="ts">
import {
  ADJECTIVE_BEHAVIOR_OPTIONS,
  COMPARATIVE_OPTIONS,
  DEFINITENESS_OPTIONS,
  NEGATIVE_VERB_OPTIONS,
  POSSESSION_OPTIONS,
  SUPERLATIVE_OPTIONS,
  VERB_MARKING_OPTIONS,
  addArticle,
  addNegativeForm,
  moveArticle,
  moveNegativeForm,
  removeArticle,
  removeNegativeForm,
  setAlienability,
  setAlienabilityNotes,
  setCustomAdjectiveBehavior,
  setCustomVerbMarking,
  setDegreeConstruction,
  setDegreeMarker,
  toggleAdjectiveBehavior,
  toggleAgreementRecord,
  toggleDefinitenessStrategy,
  toggleDegreeStrategy,
  toggleNegativeStrategy,
  togglePossessionStrategy,
  toggleVerbMarking,
  updateArticle,
  updateNegativeForm,
} from "../../grammar/strategy";
import type {
  AdjectiveBehaviorConfig,
  AdjectiveBehaviorKind,
  DefinitenessConfig,
  DefinitenessStrategy,
  DegreeConfig,
  GrammarSystemRecord,
  NegativeVerbConfig,
  NegativeVerbStrategy,
  PossessionConfig,
  PossessionStrategy,
  VerbMarkingConfig,
  VerbMarkingStrategy,
} from "../../grammar/types";
import { applySystemMutation } from "../../grammar/session";
import Field from "../parts/Field.svelte";
import Group from "../parts/Group.svelte";
import InventoryItem from "../parts/InventoryItem.svelte";
import StatusRow from "../parts/StatusRow.svelte";
import TemplateChecks from "../parts/TemplateChecks.svelte";

let {
  draft,
  locked = false,
  agreements,
}: {
  draft: GrammarSystemRecord;
  locked?: boolean;
  agreements: { id: string; title: string }[];
} = $props();

function toChecks(options: { value: string; label: string; expansion?: string; example?: string }[]) {
  return options.map((option) => ({
    id: option.value,
    label: option.label,
    meaning: option.expansion,
    example: option.example,
  }));
}
</script>

<Group>
  {#if draft.systemId === "nouns.definiteness"}
    {@const config = draft.config as DefinitenessConfig}
    {@const strategyOptions = toChecks(DEFINITENESS_OPTIONS)}
    <p class="language-empty" role="status">
      If the language has no grammatical definiteness distinction, mark this system as not used.
    </p>
    <TemplateChecks
      legend="Strategies"
      options={strategyOptions}
      selected={config.strategies}
      {locked}
      ontoggle={(value) => {
        applySystemMutation(draft, toggleDefinitenessStrategy(draft, value as DefinitenessStrategy));
      }} />
    <p class="language-empty" role="status">
      Article agreement belongs under Agreement. Record article forms here only.
    </p>
    {#if config.strategies.some((item) => item === "definite-article" || item === "indefinite-article" || item === "both" || item === "affixes")}
      {#if !locked}
        <button
          type="button"
          class="language-button secondary"
          onclick={() => {
            applySystemMutation(draft, addArticle(draft));
          }}>
          Add article form
        </button>
      {/if}
      {#each config.articles as item, index (item.id)}
        <InventoryItem
          title={`Article ${index + 1}`}
          {index}
          total={config.articles.length}
          {locked}
          onmove={(delta) => {
            applySystemMutation(draft, moveArticle(draft, item.id, delta));
          }}
          onremove={() => {
            applySystemMutation(draft, removeArticle(draft, item.id));
          }}>
          <Field label="Form"><input name="form" type="text" bind:value={item.form} disabled={locked} /></Field>
          <Field label="Position"
            ><input name="position" type="text" bind:value={item.position} disabled={locked} /></Field>
          <Field label="Notes"
            ><textarea rows="2" name="notes" bind:value={item.notes} disabled={locked}></textarea></Field>
        </InventoryItem>
      {/each}
    {/if}
  {:else if draft.systemId === "nouns.possession"}
    {@const config = draft.config as PossessionConfig}
    {@const strategyOptions = toChecks(POSSESSION_OPTIONS)}
    <p class="language-empty" role="status">
      Ordering of possessor and noun belongs under Syntax → Possessive position.
    </p>
    <TemplateChecks
      legend="Strategies"
      options={strategyOptions}
      selected={config.strategies}
      {locked}
      ontoggle={(value) => {
        applySystemMutation(draft, togglePossessionStrategy(draft, value as PossessionStrategy));
      }} />
    <details class="grammar-learn" open={Boolean(config.alienability)}>
      <summary>Advanced</summary>
      <StatusRow
        name="alienability"
        legend="Does the language distinguish alienable and inalienable possession?"
        options={[
          { value: "no", label: "No" },
          { value: "yes", label: "Yes" },
        ]}
        value={config.alienability === undefined ? undefined : config.alienability ? "yes" : "no"}
        {locked}
        onselect={(value) => {
          applySystemMutation(draft, setAlienability(draft, value === "yes"));
        }} />
      {#if config.alienability}
        <Field label="How does the distinction work?">
          <textarea rows="3" name="alienabilityNotes" bind:value={config.alienabilityNotes} disabled={locked}
          ></textarea>
        </Field>
      {/if}
    </details>
  {:else if draft.systemId === "verbs.marking-strategy"}
    {@const config = draft.config as VerbMarkingConfig}
    {@const strategyOptions = toChecks(VERB_MARKING_OPTIONS)}
    <TemplateChecks
      legend="Strategies"
      options={strategyOptions}
      selected={config.strategies}
      {locked}
      ontoggle={(value) => {
        applySystemMutation(draft, toggleVerbMarking(draft, value as VerbMarkingStrategy));
      }} />
    {#if config.strategies.includes("custom")}
      <Field label="Custom strategy">
        <input name="customStrategy" type="text" bind:value={config.customStrategy} disabled={locked} />
      </Field>
    {/if}
  {:else if draft.systemId === "verbs.negative-forms"}
    {@const config = draft.config as NegativeVerbConfig}
    {@const strategyOptions = toChecks(NEGATIVE_VERB_OPTIONS)}
    <p class="language-empty" role="status">
      Clause Types → Negation owns particles and clause behavior. Do not enter the same marker twice.
    </p>
    <TemplateChecks
      legend="Strategies"
      options={strategyOptions}
      selected={config.strategies}
      {locked}
      ontoggle={(value) => {
        applySystemMutation(draft, toggleNegativeStrategy(draft, value as NegativeVerbStrategy));
      }} />
    {#if !locked}
      <button
        type="button"
        class="language-button secondary"
        onclick={() => {
          applySystemMutation(draft, addNegativeForm(draft));
        }}>
        Add negative form
      </button>
    {/if}
    {#each config.forms as item, index (item.id)}
      <InventoryItem
        title={item.form || `Form ${index + 1}`}
        {index}
        total={config.forms.length}
        {locked}
        onmove={(delta) => {
          applySystemMutation(draft, moveNegativeForm(draft, item.id, delta));
        }}
        onremove={() => {
          applySystemMutation(draft, removeNegativeForm(draft, item.id));
        }}>
        <Field label="Marker or form"><input name="form" type="text" bind:value={item.form} disabled={locked} /></Field>
        <Field label="Changes by tense or mood">
          <textarea rows="2" name="conditions" bind:value={item.conditions} disabled={locked}></textarea>
        </Field>
        <Field label="Notes"
          ><textarea rows="2" name="notes" bind:value={item.notes} disabled={locked}></textarea></Field>
      </InventoryItem>
    {/each}
  {:else if draft.systemId === "modifiers.adjective-behavior"}
    {@const config = draft.config as AdjectiveBehaviorConfig}
    {@const strategyOptions = toChecks(ADJECTIVE_BEHAVIOR_OPTIONS)}
    <p class="language-empty" role="status">Placement is configured under Syntax → Adjective position.</p>
    <TemplateChecks
      legend="Strategies"
      options={strategyOptions}
      selected={config.behaviors}
      {locked}
      ontoggle={(value) => {
        applySystemMutation(draft, toggleAdjectiveBehavior(draft, value as AdjectiveBehaviorKind));
      }} />
    {#if config.behaviors.includes("custom")}
      <Field label="Custom behavior">
        <input name="customBehavior" type="text" bind:value={config.customBehavior} disabled={locked} />
      </Field>
    {/if}
    {#if config.behaviors.includes("agree-with-noun")}
      <p class="language-empty" role="status">
        Link the Agreement system that describes adjective agreement. Do not copy those rules here.
      </p>
      {#if agreements.length === 0}
        <p class="language-empty" role="status">No agreement systems are configured yet.</p>
      {:else}
        <TemplateChecks
          legend="Strategies"
          options={agreements.map((item) => ({ id: item.id, label: item.title }))}
          selected={config.agreementRecordIds}
          {locked}
          ontoggle={(value) => {
            applySystemMutation(draft, toggleAgreementRecord(draft, value));
          }} />
      {/if}
    {/if}
  {:else if draft.systemId === "modifiers.comparative" || draft.systemId === "modifiers.superlative"}
    {@const config = draft.config as DegreeConfig}
    {@const strategyOptions = toChecks(
      draft.systemId === "modifiers.superlative" ? SUPERLATIVE_OPTIONS : COMPARATIVE_OPTIONS,
    )}
    <TemplateChecks
      legend="Strategies"
      options={strategyOptions}
      selected={config.strategies}
      {locked}
      ontoggle={(value) => {
        applySystemMutation(draft, toggleDegreeStrategy(draft, value));
      }} />
    <Field label="Marker"><input name="marker" type="text" bind:value={config.marker} disabled={locked} /></Field>
    <Field label="Construction">
      <textarea rows="3" name="construction" bind:value={config.construction} disabled={locked}></textarea>
    </Field>
    <p class="language-empty" role="status">Irregular forms can be recorded as examples.</p>
  {/if}
</Group>

<style>
.grammar-learn {
  margin: 4px 0 8px;
}
.grammar-learn summary:focus-visible {
  outline: 3px solid rgba(180, 119, 63, 0.24);
  outline-offset: 2px;
}
</style>
