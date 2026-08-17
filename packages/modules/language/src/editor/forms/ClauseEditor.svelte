<script lang="ts">
import {
  CLAUSE_NEGATION_OPTIONS,
  CONTENT_QUESTION_OPTIONS,
  IMPERATIVE_OPTIONS,
  INTERROGATIVE_TEMPLATES,
  PLACEMENT_OPTIONS,
  RELATIVIZATION_OPTIONS,
  YES_NO_OPTIONS,
  addInterrogative,
  moveInterrogative,
  removeInterrogative,
  setContentBehavior,
  setContentCustomBehavior,
  setNegationImperatives,
  setNegationParticle,
  setNegationPlacement,
  setNegationQuestions,
  setRelativeHeadBehavior,
  setRelativeResumptives,
  setYesNoParticle,
  setYesNoPlacement,
  toggleImperativeStrategy,
  toggleInterrogative,
  toggleNegationStrategy,
  toggleRelativization,
  toggleYesNoStrategy,
  updateInterrogative,
} from "../../grammar/clause";
import type {
  ClauseNegationConfig,
  ClauseNegationStrategy,
  ContentQuestionBehavior,
  ContentQuestionsConfig,
  GrammarSystemRecord,
  ImperativeStrategy,
  ImperativesConfig,
  ParticlePlacement,
  RelativeClausesConfig,
  RelativizationStrategy,
  YesNoQuestionStrategy,
  YesNoQuestionsConfig,
} from "../../grammar/types";
import ChoiceCards from "../parts/ChoiceCards.svelte";
import Field from "../parts/Field.svelte";
import Group from "../parts/Group.svelte";
import InventoryItem from "../parts/InventoryItem.svelte";
import StatusRow from "../parts/StatusRow.svelte";
import TemplateChecks from "../parts/TemplateChecks.svelte";

let {
  draft,
  locked = false,
  lexemes,
  negativeVerbSummary,
  relativePositionSummary,
}: {
  draft: GrammarSystemRecord;
  locked?: boolean;
  lexemes: { id: string; lemma: string }[];
  negativeVerbSummary?: string;
  relativePositionSummary?: string;
} = $props();

function toChecks(options: { value: string; label: string; expansion?: string }[]) {
  return options.map((option) => ({ id: option.value, label: option.label, meaning: option.expansion }));
}
</script>

<Group>
  {#if draft.systemId === "clauses.yes-no-questions"}
    {@const config = draft.config as YesNoQuestionsConfig}
    <TemplateChecks
      legend="How are yes/no questions formed?"
      options={toChecks(YES_NO_OPTIONS)}
      selected={config.strategies}
      {locked}
      ontoggle={(value) => {
        draft.config = toggleYesNoStrategy(draft, value as YesNoQuestionStrategy).config;
      }} />
    {#if config.strategies.includes("particle")}
      <Field label="Particle"
        ><input name="particle" type="text" bind:value={config.particle} disabled={locked} /></Field>
      <StatusRow
        name="placement"
        legend="Position"
        options={PLACEMENT_OPTIONS}
        value={config.placement}
        {locked}
        onselect={(value) => {
          draft.config = setYesNoPlacement(draft, value as ParticlePlacement).config;
        }} />
    {/if}
  {:else if draft.systemId === "clauses.content-questions"}
    {@const config = draft.config as ContentQuestionsConfig}
    {@const interrogativeSelected = new Set(config.interrogatives.map((item) => item.meaning))}
    <ChoiceCards
      name="behavior"
      legend="Where do question words appear?"
      options={CONTENT_QUESTION_OPTIONS}
      value={config.behavior}
      {locked}
      onselect={(value) => {
        draft.config = setContentBehavior(draft, value as ContentQuestionBehavior).config;
      }} />
    {#if config.behavior === "custom"}
      <Field label="Custom behavior">
        <input name="customBehavior" type="text" bind:value={config.customBehavior} disabled={locked} />
      </Field>
    {/if}
    <TemplateChecks
      legend="Common interrogatives"
      options={INTERROGATIVE_TEMPLATES.map((meaning) => ({ id: meaning, label: meaning }))}
      selected={interrogativeSelected}
      {locked}
      ontoggle={(meaning) => {
        draft.config = toggleInterrogative(draft, meaning).config;
      }} />
    {#if !locked}
      <button
        type="button"
        class="language-button secondary"
        onclick={() => {
          draft.config = addInterrogative(draft).config;
        }}>
        Add interrogative
      </button>
    {/if}
    {#each config.interrogatives as item, index (item.id)}
      <InventoryItem
        title={item.meaning || `Interrogative ${index + 1}`}
        {index}
        total={config.interrogatives.length}
        {locked}
        onmove={(delta) => {
          draft.config = moveInterrogative(draft, item.id, delta).config;
        }}
        onremove={() => {
          draft.config = removeInterrogative(draft, item.id).config;
        }}>
        <Field label="Meaning"><input name="meaning" type="text" bind:value={item.meaning} disabled={locked} /></Field>
        <Field label="Form"><input name="form" type="text" bind:value={item.form} disabled={locked} /></Field>
        <Field label="Linked word (optional)">
          <select
            name="lexemeId"
            value={item.lexemeId ?? ""}
            disabled={locked}
            onchange={(event) => {
              draft.config = updateInterrogative(draft, item.id, {
                lexemeId: event.currentTarget.value || undefined,
              }).config;
            }}>
            <option value="">Not linked to a word</option>
            {#each lexemes as lexeme (lexeme.id)}
              <option value={lexeme.id}>{lexeme.lemma}</option>
            {/each}
          </select>
        </Field>
      </InventoryItem>
    {/each}
    <p class="language-empty" role="status">Interrogatives do not become lexicon entries unless you link a word.</p>
  {:else if draft.systemId === "clauses.imperatives"}
    {@const config = draft.config as ImperativesConfig}
    {@const distinctions = [
      { key: "numberDistinction", label: "Singular vs plural imperative" },
      { key: "polarityDistinction", label: "Positive vs negative imperative" },
      { key: "politenessDistinction", label: "Polite imperative" },
    ] as const}
    <TemplateChecks
      legend="How are commands formed?"
      options={toChecks(IMPERATIVE_OPTIONS)}
      selected={config.strategies}
      {locked}
      ontoggle={(value) => {
        draft.config = toggleImperativeStrategy(draft, value as ImperativeStrategy).config;
      }} />
    <details
      class="grammar-learn"
      open={Boolean(config.numberDistinction || config.polarityDistinction || config.politenessDistinction)}>
      <summary>Advanced</summary>
      {#each distinctions as item (item.key)}
        <label>
          <input type="checkbox" name="distinction" bind:checked={config[item.key]} disabled={locked} />
          {item.label}
        </label>
      {/each}
    </details>
  {:else if draft.systemId === "clauses.negation"}
    {@const config = draft.config as ClauseNegationConfig}
    <p class="language-empty" role="status">
      This editor owns clause negation. Do not re-enter negative verb morphology configured under Verbs.
    </p>
    <TemplateChecks
      legend="Primary strategy"
      options={toChecks(CLAUSE_NEGATION_OPTIONS)}
      selected={config.strategies}
      {locked}
      ontoggle={(value) => {
        draft.config = toggleNegationStrategy(draft, value as ClauseNegationStrategy).config;
      }} />
    {#if negativeVerbSummary}
      <p class="language-empty" role="status">Negative verb forms: {negativeVerbSummary}</p>
    {/if}
    {#if config.strategies.includes("particle")}
      <Field label="Particle"
        ><input name="particle" type="text" bind:value={config.particle} disabled={locked} /></Field>
      <StatusRow
        name="placement"
        legend="Position"
        options={PLACEMENT_OPTIONS}
        value={config.placement}
        {locked}
        onselect={(value) => {
          draft.config = setNegationPlacement(draft, value as ParticlePlacement).config;
        }} />
    {/if}
    <Field label="Negative questions">
      <textarea rows="3" name="negativeQuestions" bind:value={config.negativeQuestions} disabled={locked}></textarea>
    </Field>
    <Field label="Negative imperatives">
      <textarea rows="3" name="negativeImperatives" bind:value={config.negativeImperatives} disabled={locked}
      ></textarea>
    </Field>
  {:else if draft.systemId === "clauses.relative-clauses"}
    {@const config = draft.config as RelativeClausesConfig}
    <p class="language-empty" role="status">
      Placement relative to the head noun is configured under Syntax → Relative clause position.
    </p>
    <TemplateChecks
      legend="Relativization strategy"
      options={toChecks(RELATIVIZATION_OPTIONS)}
      selected={config.strategies}
      {locked}
      ontoggle={(value) => {
        draft.config = toggleRelativization(draft, value as RelativizationStrategy).config;
      }} />
    {#if relativePositionSummary}
      <p class="language-empty" role="status">Relative clause position: {relativePositionSummary}</p>
    {/if}
    <Field label="Head behavior">
      <textarea rows="3" name="headBehavior" bind:value={config.headBehavior} disabled={locked}></textarea>
    </Field>
    <Field label="Resumptives or gaps">
      <textarea rows="3" name="resumptives" bind:value={config.resumptives} disabled={locked}></textarea>
    </Field>
  {/if}
</Group>
