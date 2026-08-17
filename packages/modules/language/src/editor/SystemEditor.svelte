<script lang="ts">
import type { GrammarSystemRecord, ParadigmAxis } from "../grammar/types";
import { isChoiceSystem } from "../grammar/choice";
import { isClauseSystem } from "../grammar/clause";
import { isInventorySystem } from "../grammar/inventory";
import { isParadigmSystem } from "../grammar/paradigm";
import { isStrategySystem } from "../grammar/strategy";
import ChoiceEditor from "./forms/ChoiceEditor.svelte";
import ClauseEditor from "./forms/ClauseEditor.svelte";
import InventoryEditor from "./forms/InventoryEditor.svelte";
import ParadigmEditor from "./forms/ParadigmEditor.svelte";
import StrategyEditor from "./forms/StrategyEditor.svelte";

let {
  draft,
  locked = false,
  lexemes,
  referencedIds,
  confirm,
  agreements,
  negativeVerbSummary,
  relativePositionSummary,
  pronounAxes,
}: {
  draft: GrammarSystemRecord;
  locked?: boolean;
  lexemes: { id: string; lemma: string }[];
  referencedIds: Set<string>;
  confirm: (message: string) => Promise<boolean>;
  agreements: { id: string; title: string }[];
  negativeVerbSummary?: string;
  relativePositionSummary?: string;
  pronounAxes?: ParadigmAxis[];
} = $props();
</script>

{#if isChoiceSystem(draft.systemId)}
  <ChoiceEditor {draft} {locked} />
{:else if isInventorySystem(draft.systemId)}
  <InventoryEditor {draft} {locked} {referencedIds} {confirm} />
{:else if isStrategySystem(draft.systemId)}
  <StrategyEditor {draft} {locked} {agreements} />
{:else if isClauseSystem(draft.systemId)}
  <ClauseEditor {draft} {locked} {lexemes} {negativeVerbSummary} {relativePositionSummary} />
{:else if isParadigmSystem(draft.systemId)}
  <ParadigmEditor {draft} {locked} {confirm} {referencedIds} {pronounAxes} {agreements} />
{/if}
