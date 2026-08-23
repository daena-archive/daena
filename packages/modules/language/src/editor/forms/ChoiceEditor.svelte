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
import { applySystemMutation } from "../../grammar/session";
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
        applySystemMutation(draft, applyBasicWordOrder(draft, { order: order as WordOrderPattern }));
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
          applySystemMutation(draft, applyBasicWordOrder(draft, { strength: strength as WordOrderStrength }));
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
          applySystemMutation(draft, applyBasicWordOrder(draft, { toggleInfluence: influence as WordOrderInfluence }));
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
        applySystemMutation(draft, applyAdjectivePosition(draft, { position: position as PositionChoice }));
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
          applySystemMutation(draft, applyAdjectivePosition(draft, { toggleAlternate: value as PositionChoice }));
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
        applySystemMutation(draft, applyPossessivePosition(draft, { position: position as PossessivePositionChoice }));
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
          applySystemMutation(
            draft,
            applyPossessivePosition(draft, { toggleAlternate: value as PossessivePositionChoice }),
          );
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
        applySystemMutation(
          draft,
          applyRelativeClausePosition(draft, {
            position: position as RelativeClausePositionChoice,
          }),
        );
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
          applySystemMutation(
            draft,
            applyRelativeClausePosition(draft, {
              toggleAlternate: value as RelativeClausePositionChoice,
            }),
          );
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
        applySystemMutation(draft, applyAdpositions(draft, { strategy: strategy as AdpositionStrategy }));
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
