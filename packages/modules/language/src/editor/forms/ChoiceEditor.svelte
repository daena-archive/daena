<script lang="ts">
import {
  ADJECTIVE_POSITION_OPTIONS,
  ADPOSITION_OPTIONS,
  POSSESSIVE_POSITION_OPTIONS,
  RELATIVE_CLAUSE_POSITION_OPTIONS,
  WORD_ORDER_INFLUENCE_OPTIONS,
  WORD_ORDER_OPTIONS,
  WORD_ORDER_STRENGTH_OPTIONS,
  applyAdjectivePosition,
  applyAdpositions,
  applyBasicWordOrder,
  applyPossessivePosition,
  applyRelativeClausePosition,
} from "../../grammar/choice";
import type {
  AdpositionStrategy,
  AdpositionsConfig,
  BasicWordOrderConfig,
  GrammarSystemRecord,
  PositionChoice,
  PositionConfig,
  PossessivePositionChoice,
  PossessivePositionConfig,
  RelativeClausePositionChoice,
  RelativeClausePositionConfig,
  WordOrderInfluence,
  WordOrderPattern,
  WordOrderStrength,
} from "../../grammar/types";
import CheckRow from "../parts/CheckRow.svelte";
import ChoiceCards from "../parts/ChoiceCards.svelte";
import Field from "../parts/Field.svelte";
import Group from "../parts/Group.svelte";
import StatusRow from "../parts/StatusRow.svelte";

let { draft, locked = false }: { draft: GrammarSystemRecord; locked?: boolean } = $props();
</script>

<Group stack={false}>
  {#if draft.systemId === "syntax.basic-word-order"}
    {@const config = draft.config as BasicWordOrderConfig}
    {@const influences = config.influences ?? []}
    <ChoiceCards
      name="order"
      legend="Usual order"
      options={WORD_ORDER_OPTIONS}
      value={config.order}
      {locked}
      onselect={(order) => {
        draft.config = applyBasicWordOrder(draft, { order: order as WordOrderPattern }).config;
      }} />
    {#if config.order === "custom"}
      <Field label="Custom order">
        <input
          name="customOrder"
          type="text"
          placeholder="Describe the usual order."
          bind:value={config.customOrder}
          disabled={locked} />
      </Field>
    {/if}
    {#if config.order}
      <StatusRow
        name="strength"
        legend="How strong is this ordering?"
        options={WORD_ORDER_STRENGTH_OPTIONS}
        value={config.strength}
        {locked}
        onselect={(strength) => {
          draft.config = applyBasicWordOrder(draft, { strength: strength as WordOrderStrength }).config;
        }} />
      <Field label="What can cause the order to change?">
        <textarea rows="3" name="changeNotes" bind:value={config.changeNotes} disabled={locked}></textarea>
      </Field>
    {/if}
    {#if config.order === "flexible"}
      <CheckRow
        name="influences"
        legend="What can influence the order?"
        options={WORD_ORDER_INFLUENCE_OPTIONS}
        selected={influences}
        {locked}
        ontoggle={(influence) => {
          draft.config = applyBasicWordOrder(draft, { toggleInfluence: influence as WordOrderInfluence }).config;
        }} />
      {#if influences.includes("custom")}
        <Field label="Custom influence">
          <input name="customInfluence" type="text" bind:value={config.customInfluence} disabled={locked} />
        </Field>
      {/if}
    {/if}
  {:else if draft.systemId === "syntax.adjective-position"}
    {@const config = draft.config as PositionConfig}
    <ChoiceCards
      name="position"
      legend="Usual adjective position"
      options={ADJECTIVE_POSITION_OPTIONS}
      value={config.position}
      {locked}
      onselect={(position) => {
        draft.config = applyAdjectivePosition(draft, { position: position as PositionChoice }).config;
      }} />
    {#if config.position === "custom"}
      <Field label="Custom position">
        <input name="customPosition" type="text" bind:value={config.customPosition} disabled={locked} />
      </Field>
    {/if}
    {#if config.position}
      {@const alternates = ADJECTIVE_POSITION_OPTIONS.filter((option) => option.value !== config.position)}
      <CheckRow
        name="alternatePositions"
        legend="Other positions that also occur (optional)"
        options={alternates}
        selected={config.alternatePositions ?? []}
        {locked}
        ontoggle={(value) => {
          draft.config = applyAdjectivePosition(draft, { toggleAlternate: value as PositionChoice }).config;
        }} />
      <Field label="Does adjective position change in special situations?">
        <textarea rows="3" name="conditions" bind:value={config.conditions} disabled={locked}></textarea>
      </Field>
    {/if}
  {:else if draft.systemId === "syntax.possessive-position"}
    {@const config = draft.config as PossessivePositionConfig}
    <ChoiceCards
      name="position"
      legend="Usual possessive position"
      options={POSSESSIVE_POSITION_OPTIONS}
      value={config.position}
      {locked}
      onselect={(position) => {
        draft.config = applyPossessivePosition(draft, { position: position as PossessivePositionChoice }).config;
      }} />
    {#if config.position === "custom"}
      <Field label="Custom position">
        <input name="customPosition" type="text" bind:value={config.customPosition} disabled={locked} />
      </Field>
    {/if}
    {#if config.position}
      {@const alternates = POSSESSIVE_POSITION_OPTIONS.filter((option) => option.value !== config.position)}
      <CheckRow
        name="alternatePositions"
        legend="Other positions that also occur (optional)"
        options={alternates}
        selected={config.alternatePositions ?? []}
        {locked}
        ontoggle={(value) => {
          draft.config = applyPossessivePosition(draft, { toggleAlternate: value as PossessivePositionChoice }).config;
        }} />
      <Field label="When does this change?">
        <textarea rows="3" name="conditions" bind:value={config.conditions} disabled={locked}></textarea>
      </Field>
    {/if}
  {:else if draft.systemId === "syntax.relative-clause-position"}
    {@const config = draft.config as RelativeClausePositionConfig}
    <ChoiceCards
      name="position"
      legend="Usual relative-clause position"
      options={RELATIVE_CLAUSE_POSITION_OPTIONS}
      value={config.position}
      {locked}
      onselect={(position) => {
        draft.config = applyRelativeClausePosition(draft, {
          position: position as RelativeClausePositionChoice,
        }).config;
      }} />
    <p class="language-empty" role="status">Detailed relative-clause behavior belongs under Clause Types.</p>
    {#if config.position === "custom"}
      <Field label="Custom position">
        <input name="customPosition" type="text" bind:value={config.customPosition} disabled={locked} />
      </Field>
    {/if}
    {#if config.position}
      {@const alternates = RELATIVE_CLAUSE_POSITION_OPTIONS.filter((option) => option.value !== config.position)}
      <CheckRow
        name="alternatePositions"
        legend="Other positions that also occur (optional)"
        options={alternates}
        selected={config.alternatePositions ?? []}
        {locked}
        ontoggle={(value) => {
          draft.config = applyRelativeClausePosition(draft, {
            toggleAlternate: value as RelativeClausePositionChoice,
          }).config;
        }} />
      <Field label="When does this change?">
        <textarea rows="3" name="conditions" bind:value={config.conditions} disabled={locked}></textarea>
      </Field>
    {/if}
  {:else}
    {@const config = draft.config as AdpositionsConfig}
    <ChoiceCards
      name="strategy"
      legend="Adposition strategy"
      options={ADPOSITION_OPTIONS}
      value={config.strategy}
      {locked}
      onselect={(strategy) => {
        draft.config = applyAdpositions(draft, { strategy: strategy as AdpositionStrategy }).config;
      }} />
    <p class="language-empty" role="status">
      If this language does not use adpositions, mark the system as not used. Case is configured under Nouns.
    </p>
    {#if config.strategy === "both" || config.strategy === "other"}
      <Field label={config.strategy === "both" ? "When does each appear?" : "Describe the strategy"}>
        <textarea
          rows="3"
          name="distributionNotes"
          placeholder={config.strategy === "both" ? "When does each appear?" : "Describe the strategy."}
          bind:value={config.distributionNotes}
          disabled={locked}></textarea>
      </Field>
    {/if}
  {/if}
</Group>
