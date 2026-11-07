<script lang="ts">
import { onDestroy, onMount, untrack } from "svelte";
import AiProposalPreview from "../../../../src/lib/ai/AiProposalPreview.svelte";
import { aiCanStart, buildAiStartRequest, isTerminalAiPhase, type WritingAiMode } from "./ai-flow.ts";
import { startAiRun, type WritingAiRun, type WritingAiRunRpc, type WritingAiRunView } from "./ai-run.ts";

/**
 * The rewrite and generate dialog of the Writing webview: the shell's AI dialog, run through the broker. It owns the
 * instruction, the run and the proposal the user may edit; the workspace owns the editor. The workspace takes the
 * snapshot a run starts against (`prepare`) and applies an accepted proposal (`accept`), and unmounts the dialog
 * when the entry changes, which stops a running request.
 */
let {
  rpc,
  providerName,
  mode,
  instruction: initialInstruction,
  selectionMarkdown,
  selectionPlain,
  generationContext,
  entityId,
  entityName,
  prepare,
  accept,
  onClose,
}: {
  rpc: WritingAiRunRpc;
  providerName: string;
  mode: WritingAiMode;
  instruction: string;
  selectionMarkdown: string;
  selectionPlain: string;
  generationContext: string;
  entityId: string;
  entityName: string;
  /** Sends any unsent draft to the shell and resolves to the document text the run starts against, or null to stop. */
  prepare: () => Promise<string | null>;
  /** Applies a proposal to the editor. Resolves to the message to show when it cannot, or null once it did. */
  accept: (proposal: string) => Promise<string | null>;
  onClose: () => void;
} = $props();

let instruction = $state(untrack(() => initialInstruction));
let run = $state<WritingAiRunView | null>(null);
let proposal = $state("");
let notice = $state("");
let accepting = $state(false);
let starting = $state(false);
let active: WritingAiRun | null = null;
/** Cancel was pressed while the draft was still being handed over, before any request existed. */
let abortStart = false;
let panel: HTMLDivElement | null = null;

const busy = $derived(starting || (run !== null && !isTerminalAiPhase(run.phase)));
const cancelling = $derived(run?.cancelPending ?? false);
const canStart = $derived(aiCanStart(mode, selectionMarkdown, instruction));
const message = $derived(notice || run?.error || "");
const title = $derived(
  cancelling
    ? "Cancelling request…"
    : busy
      ? mode === "generate"
        ? "Generating text…"
        : "Rewriting selection…"
      : proposal
        ? mode === "generate"
          ? "Review generated text"
          : "Review rewrite"
        : mode === "generate"
          ? "Generate text"
          : "Rewrite selection",
);

async function start() {
  if (!canStart || busy || accepting) return;
  starting = true;
  abortStart = false;
  notice = "";
  let documentMarkdown: string | null;
  try {
    documentMarkdown = await prepare();
  } catch (cause) {
    starting = false;
    notice = cause instanceof Error ? cause.message : String(cause);
    return;
  }
  // The dialog may have been closed, or the start cancelled, while the draft was handed over.
  if (documentMarkdown === null || abortStart) {
    starting = false;
    return;
  }
  active?.dispose();
  proposal = "";
  const request = buildAiStartRequest({
    mode,
    instruction,
    selectionMarkdown,
    selectionPlain,
    generationContext,
    documentMarkdown,
    entityId,
    entityName,
    taskId: `writing-${globalThis.crypto?.randomUUID?.() ?? `${Date.now()}-${Math.random().toString(36).slice(2)}`}`,
  });
  active = startAiRun(
    {
      rpc,
      onUpdate: (view) => {
        run = view;
        proposal = view.proposal;
      },
    },
    request,
  );
  starting = false;
}

function cancel() {
  if (starting) abortStart = true;
  else void active?.cancel();
}

/** Discard, Cancel before a run and Escape: stop any running request and close. */
function close() {
  active?.dispose();
  active = null;
  onClose();
}

async function acceptProposal() {
  if (!proposal || busy || accepting) return;
  accepting = true;
  notice = "";
  try {
    const failure = await accept(proposal);
    // A null result means the workspace applied the proposal and closes the dialog.
    if (failure) notice = failure;
  } finally {
    accepting = false;
  }
}

onMount(() => {
  panel?.focus();
  // Escape closes the dialog when no request runs. It is taken before the editor sees it: the editor leaves
  // full screen on Escape, which is not what the key means here.
  const onKey = (event: KeyboardEvent) => {
    if (event.key !== "Escape") return;
    event.preventDefault();
    event.stopPropagation();
    if (!busy) close();
  };
  window.addEventListener("keydown", onKey, true);
  return () => window.removeEventListener("keydown", onKey, true);
});

onDestroy(() => active?.dispose());
</script>

<div class="ai-rewrite-modal-backdrop">
  <div
    class="ai-rewrite-panel"
    role="dialog"
    aria-modal="true"
    aria-labelledby="ai-rewrite-title"
    tabindex="-1"
    bind:this={panel}>
    <div class="ai-rewrite-heading">
      <div>
        <span class="panel-kicker">{providerName || "AI provider"}</span><strong id="ai-rewrite-title">{title}</strong>
      </div>
    </div>
    {#if !busy}<label class="ai-instruction"
        >Instruction<textarea
          rows="2"
          bind:value={instruction}
          disabled={busy}
          placeholder={`Tell ${providerName || "the AI provider"} ${mode === "generate" ? "what to write at the cursor" : "how to rewrite the selection"}`}
        ></textarea
        ></label
      >{/if}
    <AiProposalPreview
      original={selectionPlain}
      bind:proposal
      streamText={run?.streamText ?? ""}
      progressMessage={run?.progressMessage ?? "Preparing model…"}
      {busy}
      {cancelling}
      onCancel={cancel}
      onDiscard={close}
      onAccept={() => void acceptProposal()} />
    {#if message}<p class="ai-error" role="alert">{message}</p>{/if}
    {#if run?.usage}<p class="muted-note">
        Provider usage: {run.usage.inputTokens} input + {run.usage.outputTokens} output tokens.
      </p>{/if}
    {#if !busy && proposal}<div class="ai-rewrite-actions">
        <button class="primary-button" type="button" disabled={accepting} onclick={() => void acceptProposal()}
          >Accept proposal</button>
        <button class="quiet-button ai-retry-button" type="button" disabled={accepting} onclick={() => void start()}
          >Retry</button>
        <button class="quiet-button ai-discard-button" type="button" onclick={close}>Discard</button>
      </div>
    {:else if !busy}<div class="ai-rewrite-actions">
        <button class="primary-button" type="button" disabled={!canStart} onclick={() => void start()}
          >{mode === "generate" ? "Generate text" : "Generate rewrite"}</button>
        <button class="quiet-button" type="button" onclick={close}>Cancel</button>
      </div>{/if}
  </div>
</div>

<style>
.ai-rewrite-modal-backdrop {
  position: fixed;
  z-index: 80;
  inset: 0;
  display: grid;
  place-items: center;
  padding: 24px;
  background: rgba(34, 31, 26, 0.42);
  overscroll-behavior: contain;
}
.ai-rewrite-panel {
  display: grid;
  width: min(760px, 100%);
  max-height: min(82vh, 760px);
  gap: 12px;
  overflow: auto;
  padding: 14px;
  border: 1px solid var(--theme-warning-border, #d8c3a5);
  border-radius: 10px;
  background: var(--warning-bg);
  box-shadow: 0 24px 70px rgba(34, 31, 26, 0.24);
  overscroll-behavior: contain;
}
.ai-rewrite-panel:focus {
  outline: none;
}
.ai-rewrite-heading,
.ai-rewrite-actions {
  display: flex;
  align-items: center;
  justify-content: flex-start;
  gap: 10px;
}
.panel-kicker {
  display: block;
  color: var(--accent);
  font-size: 10px;
  font-weight: 800;
  letter-spacing: 0.18em;
}
.ai-rewrite-heading strong {
  display: block;
  margin-top: 4px;
  color: var(--ink);
  font-size: 14px;
}
.ai-instruction {
  display: grid;
  gap: 5px;
  color: var(--ink-soft);
  font-size: 11px;
  font-weight: 700;
}
.ai-instruction textarea {
  width: 100%;
  resize: vertical;
  padding: 9px;
  border: 1px solid var(--line);
  border-radius: 7px;
  background: var(--surface);
  color: var(--ink);
  font: 12px/1.45 var(--font-body, Inter, ui-sans-serif, system-ui, sans-serif);
}
.ai-error {
  margin: 0;
  color: var(--danger);
  font-size: 12px;
}
.muted-note {
  margin: 0;
  color: var(--ink-faint);
  font-size: 10px;
}
.primary-button,
.quiet-button {
  border-radius: 8px;
  cursor: pointer;
  font-size: 12px;
}
.primary-button {
  padding: 10px 15px;
  border: 1px solid rgba(255, 255, 255, 0.08);
  background: var(--accent-dark);
  color: #fff;
  font-weight: 700;
  box-shadow:
    0 2px 0 #263d30,
    0 7px 16px rgba(42, 68, 51, 0.16);
  transition:
    background 0.16s ease,
    box-shadow 0.16s ease,
    transform 0.16s ease;
}
.primary-button:hover:not(:disabled) {
  box-shadow:
    0 2px 0 #263d30,
    0 10px 20px rgba(42, 68, 51, 0.2);
  transform: translateY(-1px);
}
.primary-button:active:not(:disabled) {
  box-shadow:
    0 1px 0 #263d30,
    0 3px 8px rgba(42, 68, 51, 0.14);
  transform: translateY(1px);
}
.primary-button:focus-visible {
  outline: 3px solid rgba(180, 119, 63, 0.32);
  outline-offset: 2px;
}
.primary-button:disabled,
.quiet-button:disabled {
  opacity: 0.5;
  cursor: not-allowed;
  box-shadow: none;
  transform: none;
}
.quiet-button {
  padding: 10px 12px;
  border: 1px solid var(--theme-warning-border, #ded8cd);
  background: var(--surface);
  color: var(--ink-soft);
  box-shadow: 0 1px 2px rgba(48, 45, 38, 0.05);
  transition:
    background 0.16s ease,
    border-color 0.16s ease,
    box-shadow 0.16s ease,
    color 0.16s ease,
    transform 0.16s ease;
}
.quiet-button:hover:not(:disabled) {
  border-color: var(--theme-warning-border, #cbbda9);
  background: var(--surface-muted);
  color: var(--ink);
}
.quiet-button:focus-visible {
  outline: 3px solid rgba(180, 119, 63, 0.24);
  outline-offset: 2px;
}
.ai-retry-button {
  border-color: var(--theme-warning-border, #c9b486);
  background: var(--theme-warning-bg, #fffaf1);
  color: var(--theme-warning-text, #795a2e);
}
.ai-retry-button:hover:not(:disabled) {
  border-color: var(--theme-warning-border, #ae8e57);
  background: var(--theme-warning-bg, #fff4df);
  color: var(--theme-warning-text, #63471f);
}
.ai-discard-button {
  border-color: var(--theme-danger-border, #d8b2a8);
  background: var(--theme-danger-bg, #fff8f6);
  color: var(--danger);
}
.ai-discard-button:hover:not(:disabled) {
  border-color: var(--theme-danger-border, #bd8276);
  background: var(--theme-danger-bg, #fff0ec);
  color: var(--theme-danger-text, #813d32);
}
@media (max-width: 620px) {
  .ai-rewrite-modal-backdrop {
    align-items: end;
    padding: 10px;
  }
  .ai-rewrite-panel {
    max-height: 90vh;
  }
}
</style>
