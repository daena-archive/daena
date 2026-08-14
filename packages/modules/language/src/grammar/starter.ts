import { grammarSystemDescriptor } from "./catalog.ts";
import { systemStatus } from "./normalize.ts";
import type { GrammarSystemId, IndexedGrammar } from "./types.ts";

export const GRAMMAR_STARTER_STEPS = [
  "syntax.basic-word-order",
  "syntax.adjective-position",
  "nouns.number",
  "pronouns.personal",
  "verbs.tense",
  "clauses.yes-no-questions",
  "clauses.negation",
] as const satisfies readonly GrammarSystemId[];

export type GrammarStarterStep = (typeof GRAMMAR_STARTER_STEPS)[number];

export function remainingStarterSystems(index: IndexedGrammar): GrammarSystemId[] {
  return GRAMMAR_STARTER_STEPS.filter((systemId) => systemStatus(index, systemId) === "unconfigured");
}

export function nextStarterSystem(index: IndexedGrammar, current?: GrammarSystemId): GrammarSystemId | undefined {
  const start = current ? GRAMMAR_STARTER_STEPS.indexOf(current as GrammarStarterStep) + 1 : 0;
  for (const systemId of GRAMMAR_STARTER_STEPS.slice(Math.max(start, 0))) {
    if (systemStatus(index, systemId) === "unconfigured") return systemId;
  }
  return undefined;
}

export function starterStepLabel(systemId: GrammarSystemId) {
  return grammarSystemDescriptor(systemId)?.label ?? systemId;
}

export function starterPosition(systemId: GrammarSystemId) {
  const index = GRAMMAR_STARTER_STEPS.indexOf(systemId as GrammarStarterStep);
  return { current: index + 1, total: GRAMMAR_STARTER_STEPS.length };
}
