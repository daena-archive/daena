/**
 * The rewrite and generate flow of the Writing webview, as pure functions: what to ask the broker, how to read
 * its events, and when a finished proposal may still be applied. The shell runs the same flow in `+page.svelte`
 * (`startAiRewrite`, `handleAiEvent`, `acceptAiRewrite`); this module keeps the webview's version of each rule
 * in one place that scripts/writing-plugin-source.test.mjs can drive. Free of DOM and Tauri dependencies.
 */

import { reduceTextGenerationEvent } from "../../../../src/lib/ai/stream.ts";
import type { AiStreamEvent } from "../../../../src/lib/project/types.ts";

export type WritingAiMode = "rewrite" | "generate";

/** Retrieval limits of the shell's `direct_retrieval_context`; the broker applies the same host bounds. */
export const WRITING_AI_RELATIONSHIP_DEPTH = 2;
export const WRITING_AI_PASSAGE_COUNT = 8;
/** Longest retrieval query, as in the shell's `startAiRewrite`. */
export const WRITING_AI_QUERY_LIMIT = 4000;
/** Text the generate flow sends when there is no cursor context and the document is empty. */
export const WRITING_AI_CURSOR_MARKER = "[CURSOR]";

export const WRITING_AI_STALE_MESSAGE =
  "The entity or document changed while the rewrite was being prepared. Discard it and try again.";
export const WRITING_AI_GENERATE_POSITION_MESSAGE =
  "The editor position is no longer available. Discard it and try again.";
export const WRITING_AI_REWRITE_POSITION_MESSAGE =
  "The selected text is no longer at the original position. Discard it and try again.";
export const WRITING_AI_DEADLINE_PARTIAL_MESSAGE =
  "AI generation reached its time limit. The partial proposal is preserved below.";
export const WRITING_AI_DEADLINE_EMPTY_MESSAGE = "AI generation reached its time limit before producing a proposal.";
export const WRITING_AI_FAILED_MESSAGE = "The AI provider could not generate a proposal";

/**
 * The retrieval policy the broker request carries (`AiRetrievalPolicyPayload`). It reproduces what the shell's
 * `direct_retrieval_context` builds for the same request: the entity and what it relates to, two relationships
 * out, eight passages, shared fields included, no source-kind filter; with no entity, the whole project.
 */
export interface WritingRetrievalPolicy {
  mode: "related" | "project";
  query?: string;
  seedIds: string[];
  allowedSourceKinds: string[];
  relationshipDepth: number;
  passageCount: number;
  includeSharedFields: boolean;
}

/** The retrieval query: entity name, instruction and the plain selection, joined by spaces and cut to the limit. */
export function retrievalQueryFor(parts: {
  entityName: string | null | undefined;
  instruction: string;
  selectionPlain: string;
}): string {
  return [parts.entityName, parts.instruction, parts.selectionPlain]
    .filter(Boolean)
    .join(" ")
    .slice(0, WRITING_AI_QUERY_LIMIT);
}

/**
 * Maps the shell's `(entityId, retrievalQuery, retrievalDepth 2)` to a retrieval policy. Null when there is
 * neither an entity nor a query, which the shell treats as "no retrieval". Never carries a blank query.
 */
export function retrievalPolicyFor(entityId: string | null, query: string): WritingRetrievalPolicy | null {
  const seed = entityId?.trim() ? entityId : null;
  const hasQuery = query.trim().length > 0;
  if (seed === null && !hasQuery) return null;
  const related = seed !== null;
  return {
    mode: related ? "related" : "project",
    ...(hasQuery ? { query } : {}),
    seedIds: related ? [seed] : [],
    allowedSourceKinds: [],
    relationshipDepth: related ? WRITING_AI_RELATIONSHIP_DEPTH : 0,
    passageCount: WRITING_AI_PASSAGE_COUNT,
    includeSharedFields: true,
  };
}

/**
 * The text the model works on: for generate, the cursor context, else the document, else a cursor marker; for
 * rewrite, the selected Markdown as it is.
 */
export function aiSourceText(
  mode: WritingAiMode,
  source: { selectionMarkdown: string; generationContext: string; documentMarkdown: string },
): string {
  if (mode === "generate") {
    return source.generationContext.trim() || source.documentMarkdown.trim() || WRITING_AI_CURSOR_MARKER;
  }
  return source.selectionMarkdown;
}

/** Whether the dialog may start: a rewrite needs a selection, and every request needs an instruction. */
export function aiCanStart(mode: WritingAiMode, selectionMarkdown: string, instruction: string): boolean {
  return !(mode === "rewrite" && !selectionMarkdown.trim()) && instruction.trim().length > 0;
}

export interface WritingAiStartRequest {
  operation: "generate_text";
  taskId: string;
  userInstruction: string;
  immediateContext: { selection: string };
  retrievalPolicy?: WritingRetrievalPolicy;
}

/** The `ai.request.start` payload for one run of the dialog. */
export function buildAiStartRequest(input: {
  mode: WritingAiMode;
  instruction: string;
  selectionMarkdown: string;
  selectionPlain: string;
  generationContext: string;
  documentMarkdown: string;
  entityId: string | null;
  entityName: string | null | undefined;
  taskId: string;
}): WritingAiStartRequest {
  const policy = retrievalPolicyFor(
    input.entityId,
    retrievalQueryFor({
      entityName: input.entityName,
      instruction: input.instruction,
      selectionPlain: input.selectionPlain,
    }),
  );
  return {
    operation: "generate_text",
    taskId: input.taskId,
    userInstruction: input.instruction,
    immediateContext: { selection: aiSourceText(input.mode, input) },
    ...(policy ? { retrievalPolicy: policy } : {}),
  };
}

/** What a run was started against, and what the editor holds now. */
export interface WritingAiAcceptCheck {
  source: { entityId: string; token: string; markdown: string };
  current: { entityId: string; token: string; markdown: string } | null;
}

/**
 * Null when the proposal may still be applied, else the message to show. The proposal belongs to one entry, one
 * load of it and one text; if any of them moved, the editor's captured position cannot be trusted.
 */
export function aiAcceptGuard(check: WritingAiAcceptCheck): string | null {
  const { source, current } = check;
  if (
    current === null ||
    current.entityId !== source.entityId ||
    current.token !== source.token ||
    current.markdown !== source.markdown
  ) {
    return WRITING_AI_STALE_MESSAGE;
  }
  return null;
}

export type WritingAiRunPhase = "starting" | "running" | "completed" | "cancelled" | "deadline_exceeded" | "failed";

export function isTerminalAiPhase(phase: WritingAiRunPhase): boolean {
  return phase === "completed" || phase === "cancelled" || phase === "deadline_exceeded" || phase === "failed";
}

export interface WritingAiUsage {
  inputTokens: number;
  outputTokens: number;
  totalTokens: number;
}

/** What the dialog shows of a run. */
export interface WritingAiRunState {
  phase: WritingAiRunPhase;
  streamText: string;
  proposal: string;
  progressMessage: string;
  usage: WritingAiUsage | null;
  error: string;
  /** The highest event sequence applied; later polls repeat the buffer, so anything at or below it is skipped. */
  lastSequence: number;
  /** True when the broker's bounded buffer dropped events this run never saw, so `streamText` has holes. */
  gap: boolean;
}

export function initialAiRunState(): WritingAiRunState {
  return {
    phase: "starting",
    streamText: "",
    proposal: "",
    progressMessage: "Preparing model…",
    usage: null,
    error: "",
    lastSequence: -1,
    gap: false,
  };
}

function parseUsage(output: string | null): WritingAiUsage | null {
  if (!output) return null;
  try {
    const value = JSON.parse(output) as Partial<WritingAiUsage> | null;
    if (
      value &&
      typeof value.inputTokens === "number" &&
      typeof value.outputTokens === "number" &&
      typeof value.totalTokens === "number"
    ) {
      return { inputTokens: value.inputTokens, outputTokens: value.outputTokens, totalTokens: value.totalTokens };
    }
  } catch {
    // An unreadable usage line is shown as no usage, like the shell does.
  }
  return null;
}

/**
 * Applies one broker event to a run, the way the shell's `handleAiEvent` does. Events at or below the last
 * applied sequence are skipped (a poll returns the whole buffer every time), and nothing is applied after a
 * terminal phase. A terminal event's output is the authoritative proposal: deltas can be missing when the
 * buffer dropped them, in which case a terminal event without output leaves no proposal rather than a gappy one.
 */
export function applyAiEvent(state: WritingAiRunState, event: AiStreamEvent): WritingAiRunState {
  if (isTerminalAiPhase(state.phase) || event.sequence <= state.lastSequence) return state;
  const gap = state.gap || event.sequence !== state.lastSequence + 1;
  const reduced = reduceTextGenerationEvent(
    { streamText: state.streamText, proposal: state.proposal, progressMessage: state.progressMessage },
    event,
  );
  let next: WritingAiRunState = {
    ...state,
    streamText: reduced.streamText,
    proposal: reduced.proposal,
    progressMessage: reduced.progressMessage,
    lastSequence: event.sequence,
    gap,
    phase: state.phase === "starting" ? "running" : state.phase,
  };
  if (event.phase === "usage") next.usage = parseUsage(event.output) ?? next.usage;
  switch (event.phase) {
    case "completed":
      next.phase = "completed";
      break;
    case "cancelled":
      next.phase = "cancelled";
      break;
    case "deadline_exceeded":
      next.phase = "deadline_exceeded";
      next.error = next.proposal ? WRITING_AI_DEADLINE_PARTIAL_MESSAGE : WRITING_AI_DEADLINE_EMPTY_MESSAGE;
      break;
    case "failed":
      next.phase = "failed";
      next.error = event.error ?? WRITING_AI_FAILED_MESSAGE;
      break;
  }
  if (isTerminalAiPhase(next.phase) && gap && event.output === null) next = { ...next, proposal: "" };
  return next;
}

/** Applies a poll's events in order. */
export function applyAiEvents(state: WritingAiRunState, events: readonly AiStreamEvent[]): WritingAiRunState {
  return events.reduce(applyAiEvent, state);
}
