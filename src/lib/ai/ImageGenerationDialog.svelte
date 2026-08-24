<script lang="ts">
import { onDestroy, onMount } from "svelte";
import { Check, ChevronDown, Image, RefreshCw, Sparkles, Trash2, WandSparkles, X } from "@lucide/svelte";
import {
  project,
  type AiProviderSettings,
  type Asset,
  type Entity,
  type ImageCandidate,
  type ImageContextItem,
  type ImageGenerationStatus,
  type ImagePromptProvenance,
  type ImageProviderDiscovery,
  type ImageProviderSettings,
} from "$lib/project/client";

export type ImageContextChoice = {
  id: string;
  entityId: string;
  label: string;
  value: string;
  sourceKind: ImageContextItem["sourceKind"];
  defaultSelected: boolean;
};

let {
  projectId,
  entity,
  namespace,
  contextChoices,
  imageProvider,
  textProvider,
  onAccepted,
  onClose,
}: {
  projectId: string;
  entity: Entity;
  namespace: string;
  contextChoices: ImageContextChoice[];
  imageProvider: ImageProviderSettings;
  textProvider: AiProviderSettings;
  onAccepted: (asset: Asset) => void;
  onClose: () => void;
} = $props();

// svelte-ignore state_referenced_locally
let selectedContext = $state<Record<string, boolean>>(
  Object.fromEntries(contextChoices.map((choice) => [choice.id, choice.defaultSelected])),
);
let prompt = $state("");
let negativePrompt = $state("");
let provenance = $state<ImagePromptProvenance>({
  method: "manual",
  llmAssisted: false,
  editedAfterAssistance: false,
  textProviderId: null,
  textModel: null,
});
let discovery = $state<ImageProviderDiscovery | null>(null);
let model = $state("");
let width = $state(1024);
let height = $state(1024);
let seed = $state(randomSeed());
let outputCount = $state(1);
let steps = $state(24);
let guidanceScale = $state(7);
let sampler = $state("euler");
let scheduler = $state("normal");
let advancedOpen = $state(false);
let busy = $state(false);
let promptBusy = $state(false);
let status = $state<ImageGenerationStatus | null>(null);
let error = $state("");
let jobId = $state<string | null>(null);
let pollTimer: number | null = null;
let candidateUrls = $state<Record<string, string>>({});
let accepting = $state<Record<string, boolean>>({});

const selectedChoices = $derived(contextChoices.filter((choice) => selectedContext[choice.id]));
const textBoundary = $derived(endpointBoundary(textProvider.endpoint));
const canUseTextAi = $derived(Boolean(textProvider.endpoint.trim() && textProvider.model.trim()));
const generationActive = $derived(status && ["queued", "running", "downloading"].includes(status.state));

function randomSeed() {
  const values = new Uint32Array(2);
  crypto.getRandomValues(values);
  return values[0] * 0x100000 + (values[1] & 0xfffff);
}

function endpointBoundary(endpoint: string) {
  try {
    const hostname = new URL(endpoint.trim()).hostname.toLowerCase().replace(/^\[|\]$/g, "");
    if (
      hostname === "localhost" ||
      hostname === "localhost.localdomain" ||
      hostname === "::1" ||
      hostname.startsWith("127.")
    )
      return "Local";
  } catch {
    // An invalid endpoint is reported by the text provider when an action is attempted.
  }
  return "Remote";
}

function friendlyError(cause: unknown) {
  return cause instanceof Error ? cause.message : String(cause);
}

function contextPayload(onlyDefaults = false) {
  const choices = onlyDefaults ? contextChoices.filter((choice) => choice.defaultSelected) : selectedChoices;
  return {
    entity: { id: entity.id, name: entity.name, type: entity.entity_type ?? "Uncategorized" },
    selectedContext: choices.map((choice) => ({
      label: choice.label,
      value: choice.value,
      sourceKind: choice.sourceKind,
    })),
  };
}

function generationContext(): ImageContextItem[] {
  return selectedChoices.map((choice) => ({
    entityId: choice.entityId,
    label: choice.label,
    sourceKind: choice.sourceKind,
  }));
}

function promptInstruction(action: ImagePromptProvenance["method"]) {
  if (action === "rewrite") {
    return "Rewrite the current image prompt into one concise, visually specific text-to-image prompt. Preserve all stated facts. Return only the final prompt.";
  }
  if (action === "detailed") {
    return "Make the current image prompt more visually detailed while preserving every stated fact. Add composition, lighting, materials, and atmosphere only when consistent with the supplied context. Return only the final prompt.";
  }
  if (action === "simplified") {
    return "Simplify the current image prompt into a concise, clear text-to-image prompt without losing stated world facts. Return only the final prompt.";
  }
  return "Write one concise, visually oriented text-to-image prompt from the selected world facts. Preserve facts, avoid unsupported inventions, and return only the final prompt.";
}

async function buildPrompt(action: ImagePromptProvenance["method"], defaultsOnly = false) {
  if (!canUseTextAi || promptBusy) return;
  promptBusy = true;
  error = "";
  try {
    if (defaultsOnly) {
      selectedContext = Object.fromEntries(contextChoices.map((choice) => [choice.id, choice.defaultSelected]));
    }
    const selection = {
      ...contextPayload(defaultsOnly),
      ...(action === "rewrite" || action === "detailed" || action === "simplified" ? { currentPrompt: prompt } : {}),
    };
    prompt = await project.aiGenerateText(
      projectId,
      promptInstruction(action),
      JSON.stringify(selection, null, 2),
      entity.id,
      undefined,
      0,
      false,
    );
    provenance = {
      method: action,
      llmAssisted: true,
      editedAfterAssistance: false,
      textProviderId: textProvider.id,
      textModel: textProvider.model,
    };
  } catch (cause) {
    error = friendlyError(cause);
  } finally {
    promptBusy = false;
  }
}

function markPromptEdited(value: string) {
  prompt = value;
  if (provenance.llmAssisted) {
    provenance = { ...provenance, editedAfterAssistance: true };
  } else {
    provenance = {
      method: "manual",
      llmAssisted: false,
      editedAfterAssistance: false,
      textProviderId: null,
      textModel: null,
    };
  }
}

async function loadDiscovery() {
  try {
    discovery = await project.imageProviderDiscover();
    if (!discovery.models.includes(model)) model = discovery.models[0] ?? "";
    if (!discovery.samplers.includes(sampler)) sampler = discovery.samplers[0] ?? "";
    if (!discovery.schedulers.includes(scheduler)) scheduler = discovery.schedulers[0] ?? "";
  } catch (cause) {
    error = friendlyError(cause);
  }
}

function clearPolling() {
  if (pollTimer !== null) {
    window.clearInterval(pollTimer);
    pollTimer = null;
  }
}

function releaseCandidateUrl(candidateId: string) {
  const url = candidateUrls[candidateId];
  if (!url) return;
  URL.revokeObjectURL(url);
  const next = { ...candidateUrls };
  delete next[candidateId];
  candidateUrls = next;
}

function clearCandidates() {
  for (const url of Object.values(candidateUrls)) URL.revokeObjectURL(url);
  candidateUrls = {};
}

async function loadCandidate(candidate: ImageCandidate) {
  if (!jobId || candidateUrls[candidate.id] || candidate.acceptedAssetId) return;
  try {
    const bytes = await project.imageCandidateBytes(jobId, candidate.id, projectId);
    const blob = new Blob([Uint8Array.from(bytes)], { type: candidate.mimeType });
    candidateUrls = { ...candidateUrls, [candidate.id]: URL.createObjectURL(blob) };
  } catch (cause) {
    error = friendlyError(cause);
  }
}

async function applyStatus(next: ImageGenerationStatus) {
  status = next;
  if (["completed", "failed", "cancelled"].includes(next.state)) clearPolling();
  if (next.error) error = next.error;
  if (next.state === "completed") {
    await Promise.all(next.candidates.map(loadCandidate));
  }
}

async function poll() {
  if (!jobId) return;
  try {
    await applyStatus(await project.imageGenerationStatus(jobId, projectId));
  } catch (cause) {
    clearPolling();
    error = friendlyError(cause);
  }
}

async function discardCurrentJob() {
  const current = jobId;
  jobId = null;
  clearPolling();
  clearCandidates();
  if (!current) return;
  try {
    await project.imageGenerationDiscard(current, projectId);
  } catch {
    // Expired jobs are already gone and require no further cleanup.
  }
}

async function generate(variant = false) {
  if (busy || !prompt.trim() || !discovery) return;
  busy = true;
  error = "";
  if (variant) seed = randomSeed();
  await discardCurrentJob();
  status = null;
  try {
    const started = await project.imageGenerateStart({
      projectId,
      entityId: entity.id,
      prompt: prompt.trim(),
      negativePrompt: negativePrompt.trim(),
      model,
      width,
      height,
      seed,
      outputCount,
      steps,
      guidanceScale,
      sampler,
      scheduler,
      context: generationContext(),
      promptProvenance: provenance,
    });
    jobId = started.jobId;
    status = started;
    pollTimer = window.setInterval(() => void poll(), 600);
  } catch (cause) {
    error = friendlyError(cause);
  } finally {
    busy = false;
  }
}

async function cancelGeneration() {
  if (!jobId) return;
  try {
    await applyStatus(await project.imageGenerationCancel(jobId, projectId));
  } catch (cause) {
    error = friendlyError(cause);
  }
}

function candidateFilename(candidate: ImageCandidate, index: number) {
  const suffix = status && status.candidates.length > 1 ? `-${index + 1}` : "";
  const extension = candidate.mimeType === "image/jpeg" ? "jpg" : candidate.mimeType.split("/")[1];
  return `${entity.name}-illustration${suffix}.${extension}`;
}

async function acceptCandidate(candidate: ImageCandidate, index: number) {
  if (!jobId || accepting[candidate.id] || candidate.acceptedAssetId) return;
  accepting = { ...accepting, [candidate.id]: true };
  error = "";
  try {
    const asset = await project.imageCandidateAccept(
      jobId,
      candidate.id,
      projectId,
      entity.id,
      namespace,
      candidateFilename(candidate, index),
    );
    onAccepted(asset);
    status = status
      ? {
          ...status,
          candidates: status.candidates.map((current) =>
            current.id === candidate.id ? { ...current, acceptedAssetId: asset.id } : current,
          ),
        }
      : status;
  } catch (cause) {
    error = friendlyError(cause);
  } finally {
    const next = { ...accepting };
    delete next[candidate.id];
    accepting = next;
  }
}

async function discardCandidate(candidate: ImageCandidate) {
  if (!jobId || candidate.acceptedAssetId) return;
  try {
    releaseCandidateUrl(candidate.id);
    await applyStatus(await project.imageCandidateDiscard(jobId, candidate.id, projectId));
  } catch (cause) {
    error = friendlyError(cause);
  }
}

async function closeDialog() {
  await discardCurrentJob();
  onClose();
}

onMount(() => {
  model = imageProvider.model;
  void loadDiscovery();
});
onDestroy(() => {
  clearPolling();
  const currentJobId = jobId;
  jobId = null;
  if (currentJobId) {
    void project.imageGenerationDiscard(currentJobId, projectId).catch(() => undefined);
  }
  clearCandidates();
});
</script>

<div class="backdrop">
  <div class="dialog" role="dialog" aria-modal="true" aria-labelledby="image-generation-title">
    <header>
      <div class="title-icon"><WandSparkles size={19} strokeWidth={1.7} aria-hidden="true" /></div>
      <div>
        <span>GENERATE ILLUSTRATION</span>
        <h1 id="image-generation-title">Visualize {entity.name}</h1>
        <p>Selected world facts can help author the prompt; the generated image never changes canonical entity data.</p>
      </div>
      <button type="button" class="icon-button" aria-label="Close image generation" onclick={() => void closeDialog()}>
        <X size={16} strokeWidth={1.8} />
      </button>
    </header>

    <div class="content">
      <section class="context-panel" aria-labelledby="generation-context-heading">
        <div class="section-heading">
          <div>
            <span>1 · CONTEXT</span>
            <h2 id="generation-context-heading">Choose world facts</h2>
          </div>
          <small>{selectedChoices.length} selected</small>
        </div>
        <p class="hint">
          Only checked facts are sent to the prompt-building LLM. Image generation receives the final prompt.
        </p>
        <div class="context-list">
          {#each contextChoices as choice}
            <label>
              <input type="checkbox" bind:checked={selectedContext[choice.id]} />
              <span><strong>{choice.label}</strong><small>{choice.value}</small></span>
            </label>
          {/each}
        </div>
      </section>

      <section class="prompt-panel" aria-labelledby="generation-prompt-heading">
        <div class="section-heading">
          <div>
            <span>2 · PROMPT</span>
            <h2 id="generation-prompt-heading">Review the final prompt</h2>
          </div>
          <small>{textBoundary} text AI</small>
        </div>
        <div class="prompt-actions">
          <button type="button" disabled={!canUseTextAi || promptBusy} onclick={() => void buildPrompt("entity", true)}>
            <Sparkles size={12} strokeWidth={1.8} /> From entity
          </button>
          <button
            type="button"
            disabled={!canUseTextAi || promptBusy || selectedChoices.length === 0}
            onclick={() => void buildPrompt("selected-context")}>From selected context</button>
          <button
            type="button"
            disabled={!canUseTextAi || promptBusy || !prompt.trim()}
            onclick={() => void buildPrompt("rewrite")}>
            Rewrite
          </button>
          <button
            type="button"
            disabled={!canUseTextAi || promptBusy || !prompt.trim()}
            onclick={() => void buildPrompt("detailed")}>
            More detail
          </button>
          <button
            type="button"
            disabled={!canUseTextAi || promptBusy || !prompt.trim()}
            onclick={() => void buildPrompt("simplified")}>
            Simplify
          </button>
        </div>
        {#if !canUseTextAi}<p class="hint">
            Text AI is not configured. You can still write the image prompt manually.
          </p>{/if}
        <label class="prompt-field"
          >Image prompt<textarea
            value={prompt}
            rows="6"
            maxlength="16384"
            placeholder="Describe the illustration you want…"
            oninput={(event) => markPromptEdited((event.currentTarget as HTMLTextAreaElement).value)}></textarea
          ></label>
        <label class="prompt-field"
          >Negative prompt <small>optional</small><textarea
            bind:value={negativePrompt}
            rows="2"
            maxlength="16384"
            placeholder="Elements to avoid…"></textarea
          ></label>
      </section>

      <section class="generation-panel" aria-labelledby="generation-settings-heading">
        <div class="section-heading">
          <div>
            <span>3 · GENERATION</span>
            <h2 id="generation-settings-heading">Local ComfyUI settings</h2>
          </div>
          <small class="local">Local · no fallback</small>
        </div>
        <dl class="provider-summary">
          <div>
            <dt>Provider</dt>
            <dd>{imageProvider.name}</dd>
          </div>
          <div>
            <dt>Model</dt>
            <dd>{model || "Not selected"}</dd>
          </div>
          <div>
            <dt>Endpoint</dt>
            <dd>{imageProvider.endpoint}</dd>
          </div>
        </dl>
        <div class="control-grid">
          <label
            >Model<select bind:value={model}
              >{#each discovery?.models ?? [] as value}<option>{value}</option>{/each}</select
            ></label>
          <label>Width<input type="number" min="64" max="4096" step="8" bind:value={width} /></label>
          <label>Height<input type="number" min="64" max="4096" step="8" bind:value={height} /></label>
          <label>Seed<input type="number" min="0" max="9007199254740991" step="1" bind:value={seed} /></label>
          <label
            >Outputs<select bind:value={outputCount}>
              <option value={1}>1</option><option value={2}>2</option><option value={3}>3</option><option value={4}
                >4</option>
            </select></label>
        </div>
        <button type="button" class="advanced-toggle" onclick={() => (advancedOpen = !advancedOpen)}>
          <ChevronDown size={13} strokeWidth={1.8} class={advancedOpen ? "open" : ""} /> Advanced
        </button>
        {#if advancedOpen}
          <div class="control-grid advanced">
            <label>Steps<input type="number" min="1" max="150" step="1" bind:value={steps} /></label>
            <label>Guidance<input type="number" min="0" max="30" step="0.5" bind:value={guidanceScale} /></label>
            <label
              >Sampler<select bind:value={sampler}
                >{#each discovery?.samplers ?? [] as value}<option>{value}</option>{/each}</select
              ></label>
            <label
              >Scheduler<select bind:value={scheduler}
                >{#each discovery?.schedulers ?? [] as value}<option>{value}</option>{/each}</select
              ></label>
          </div>
        {/if}
        <div class="generate-row">
          <button
            type="button"
            class="generate"
            disabled={busy || generationActive || !prompt.trim() || !discovery || !model}
            onclick={() => void generate(false)}>
            <Image size={14} strokeWidth={1.8} />
            {busy ? "Preparing…" : "Generate"}
          </button>
          {#if generationActive}<button type="button" class="quiet" onclick={() => void cancelGeneration()}
              >Cancel</button
            >{/if}
          {#if status}<span role="status"
              >{status.stage}{status.state === "downloading" ? ` · ${status.completed}/${status.total}` : ""}</span
            >{/if}
        </div>
      </section>

      {#if error}<p class="error" role="alert">{error}</p>{/if}

      {#if status?.candidates.length}
        <section class="results" aria-labelledby="generation-results-heading">
          <div class="section-heading">
            <div>
              <span>4 · REVIEW</span>
              <h2 id="generation-results-heading">Temporary candidates</h2>
            </div>
            <button type="button" class="quiet" disabled={busy || generationActive} onclick={() => void generate(true)}>
              <RefreshCw size={12} strokeWidth={1.8} /> Another variant
            </button>
          </div>
          <p class="hint">
            Accepting attaches a normal project asset with generation provenance. Discarded candidates are not retained.
          </p>
          <div class="candidate-grid">
            {#each status.candidates as candidate, index (candidate.id)}
              <article>
                {#if candidateUrls[candidate.id]}
                  <img src={candidateUrls[candidate.id]} alt={`Generated candidate ${index + 1} for ${entity.name}`} />
                {:else}<div class="image-loading">Loading candidate…</div>{/if}
                <div class="candidate-info">
                  <span>{candidate.width} × {candidate.height} · seed {candidate.seed}</span>
                  <small>{Math.max(1, Math.round(candidate.size / 1024))} KB</small>
                </div>
                <div class="candidate-actions">
                  {#if candidate.acceptedAssetId}
                    <span class="accepted"><Check size={12} strokeWidth={2} /> Attached to {entity.name}</span>
                  {:else}
                    <button
                      type="button"
                      class="accept"
                      disabled={accepting[candidate.id]}
                      onclick={() => void acceptCandidate(candidate, index)}>
                      {accepting[candidate.id] ? "Accepting…" : "Accept & attach"}
                    </button>
                    <button
                      type="button"
                      class="discard"
                      aria-label={`Discard candidate ${index + 1}`}
                      onclick={() => void discardCandidate(candidate)}>
                      <Trash2 size={13} strokeWidth={1.8} /> Discard
                    </button>
                  {/if}
                </div>
              </article>
            {/each}
          </div>
        </section>
      {/if}
    </div>
  </div>
</div>

<style>
.backdrop {
  position: fixed;
  z-index: 1200;
  inset: 0;
  display: grid;
  padding: 24px;
  place-items: center;
  background: rgba(18, 25, 19, 0.58);
  backdrop-filter: blur(3px);
}
.dialog {
  display: flex;
  width: min(1120px, 100%);
  max-height: min(900px, calc(100vh - 48px));
  flex-direction: column;
  overflow: hidden;
  border: 1px solid var(--theme-neutral-border, #ced7cd);
  border-radius: 18px;
  background: var(--theme-success-bg, #f7f8f5);
  color: var(--theme-neutral-text, #2b352d);
  box-shadow: 0 26px 80px rgba(16, 23, 17, 0.28);
}
.dialog > header {
  display: grid;
  grid-template-columns: auto minmax(0, 1fr) auto;
  gap: 12px;
  padding: 18px 20px;
  border-bottom: 1px solid var(--theme-neutral-border, #dce2da);
  background: var(--theme-surface-bg, #fff);
}
.title-icon {
  display: grid;
  width: 38px;
  height: 38px;
  place-items: center;
  border-radius: 10px;
  background: var(--theme-success-bg, #e9f2e8);
  color: var(--theme-success-text, #4d7053);
}
header span,
.section-heading span {
  color: var(--theme-neutral-text-soft, #708174);
  font-size: 9px;
  font-weight: 800;
  letter-spacing: 0.09em;
}
header h1 {
  margin: 2px 0 3px;
  color: var(--theme-neutral-text, #263128);
  font-size: 17px;
}
header p,
.hint {
  margin: 0;
  color: var(--theme-neutral-text-soft, #737d75);
  font-size: 10px;
  line-height: 1.5;
}
.icon-button {
  display: grid;
  width: 32px;
  height: 32px;
  place-items: center;
  border: 1px solid var(--theme-neutral-border, #d7ded6);
  border-radius: 8px;
  background: var(--theme-surface-bg, #fff);
  color: var(--theme-neutral-text-soft, #59645b);
  cursor: pointer;
}
.content {
  display: grid;
  grid-template-columns: minmax(220px, 0.75fr) minmax(360px, 1.35fr) minmax(280px, 0.9fr);
  gap: 14px;
  overflow: auto;
  padding: 14px;
}
.content > section {
  padding: 16px;
  border: 1px solid var(--theme-neutral-border, #dce2da);
  border-radius: 12px;
  background: var(--theme-surface-bg, #fff);
}
.section-heading {
  display: flex;
  align-items: start;
  justify-content: space-between;
  gap: 10px;
}
.section-heading h2 {
  margin: 3px 0 0;
  color: var(--theme-neutral-text, #2b362d);
  font-size: 13px;
}
.section-heading small {
  color: var(--theme-neutral-text-soft, #748078);
  font-size: 9px;
}
.section-heading small.local {
  padding: 4px 7px;
  border-radius: 999px;
  background: var(--theme-success-bg, #e8f3e8);
  color: var(--theme-success-text, #417048);
}
.context-panel .hint,
.results .hint {
  margin-top: 8px;
}
.context-list {
  display: flex;
  max-height: 430px;
  flex-direction: column;
  gap: 6px;
  overflow: auto;
  margin-top: 11px;
}
.context-list label {
  display: flex;
  align-items: start;
  gap: 8px;
  padding: 8px;
  border-radius: 8px;
  background: var(--theme-success-bg, #f5f7f4);
  cursor: pointer;
}
.context-list span {
  min-width: 0;
}
.context-list strong,
.context-list small {
  display: block;
}
.context-list strong {
  color: var(--theme-neutral-text-soft, #49554b);
  font-size: 9px;
}
.context-list small {
  overflow: hidden;
  margin-top: 2px;
  color: var(--theme-neutral-text-soft, #727d74);
  font-size: 9px;
  line-height: 1.4;
  text-overflow: ellipsis;
}
.prompt-actions {
  display: flex;
  flex-wrap: wrap;
  gap: 5px;
  margin-top: 11px;
}
.prompt-actions button,
.advanced-toggle,
.quiet {
  display: inline-flex;
  min-height: 28px;
  align-items: center;
  gap: 5px;
  padding: 0 8px;
  border: 1px solid var(--theme-neutral-border, #d4dbd3);
  border-radius: 7px;
  background: var(--theme-surface-bg, #fff);
  color: var(--theme-neutral-text-soft, #516055);
  font-size: 9px;
  font-weight: 700;
  cursor: pointer;
}
button:disabled {
  cursor: default;
  opacity: 0.5;
}
.prompt-field {
  display: block;
  margin-top: 11px;
  color: var(--theme-neutral-text-soft, #526056);
  font-size: 10px;
  font-weight: 700;
}
.prompt-field small {
  color: var(--theme-neutral-text-muted, #8a948c);
  font-weight: 500;
}
textarea,
input,
select {
  width: 100%;
  box-sizing: border-box;
  border: 1px solid var(--theme-neutral-border, #d4dbd3);
  border-radius: 8px;
  background: var(--theme-surface-bg, #fff);
  color: var(--theme-neutral-text, #2f3931);
  font: 500 10px/1.5 inherit;
}
textarea {
  display: block;
  margin-top: 5px;
  padding: 9px 10px;
  resize: vertical;
}
.provider-summary {
  display: grid;
  gap: 6px;
  margin: 11px 0 0;
}
.provider-summary div {
  display: grid;
  grid-template-columns: 58px minmax(0, 1fr);
  gap: 7px;
  font-size: 9px;
}
.provider-summary dt {
  color: var(--theme-neutral-text-muted, #89928b);
}
.provider-summary dd {
  overflow: hidden;
  margin: 0;
  color: var(--theme-neutral-text-soft, #4d5a50);
  text-overflow: ellipsis;
  white-space: nowrap;
}
.control-grid {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 8px;
  margin-top: 12px;
}
.control-grid label {
  color: var(--theme-neutral-text-soft, #68736a);
  font-size: 9px;
  font-weight: 700;
}
.control-grid input,
.control-grid select {
  min-height: 32px;
  margin-top: 4px;
  padding: 0 8px;
}
.advanced-toggle {
  margin-top: 10px;
}
.advanced-toggle :global(svg) {
  transition: transform 0.15s ease;
}
.advanced-toggle :global(svg.open) {
  transform: rotate(180deg);
}
.generate-row {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 7px;
  margin-top: 14px;
}
.generate {
  display: inline-flex;
  min-height: 34px;
  align-items: center;
  gap: 6px;
  padding: 0 13px;
  border: 0;
  border-radius: 8px;
  background: #486d50;
  color: #fff;
  font-size: 10px;
  font-weight: 800;
  cursor: pointer;
}
.generate-row span {
  color: var(--theme-neutral-text-soft, #69766c);
  font-size: 9px;
}
.error {
  grid-column: 1 / -1;
  margin: 0;
  padding: 9px 11px;
  border: 1px solid var(--theme-danger-border, #e9c6bb);
  border-radius: 8px;
  background: var(--theme-danger-bg, #fff0ec);
  color: var(--theme-danger-text, #944c3c);
  font-size: 10px;
}
.results {
  grid-column: 1 / -1;
}
.candidate-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(220px, 1fr));
  gap: 12px;
  margin-top: 12px;
}
.candidate-grid article {
  overflow: hidden;
  border: 1px solid var(--theme-neutral-border, #dce2da);
  border-radius: 10px;
  background: var(--theme-surface-bg, #f7f8f6);
}
.candidate-grid img,
.image-loading {
  width: 100%;
  aspect-ratio: 1;
  object-fit: contain;
  background: var(--theme-muted-bg, #e9ece8);
}
.image-loading {
  display: grid;
  place-items: center;
  color: var(--theme-neutral-text-muted, #7b857d);
  font-size: 10px;
}
.candidate-info,
.candidate-actions {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
  padding: 8px 9px;
}
.candidate-info {
  color: var(--theme-neutral-text-soft, #566158);
  font-size: 9px;
}
.candidate-info small {
  color: var(--theme-neutral-text-muted, #8a938c);
}
.candidate-actions {
  padding-top: 0;
}
.candidate-actions button {
  min-height: 30px;
  border-radius: 7px;
  font-size: 9px;
  font-weight: 800;
  cursor: pointer;
}
.candidate-actions .accept {
  flex: 1;
  border: 0;
  background: #486d50;
  color: #fff;
}
.candidate-actions .discard {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  border: 1px solid var(--theme-danger-border, #dacfc9);
  background: var(--theme-surface-bg, #fff);
  color: var(--theme-danger-text, #815b50);
}
.accepted {
  display: inline-flex;
  align-items: center;
  gap: 5px;
  color: var(--theme-success-text, #3f7548);
  font-size: 9px;
  font-weight: 800;
}
@media (max-width: 900px) {
  .content {
    grid-template-columns: 1fr 1fr;
  }
  .context-panel {
    grid-row: span 2;
  }
}
@media (max-width: 680px) {
  .backdrop {
    padding: 0;
  }
  .dialog {
    max-height: 100vh;
    border-radius: 0;
  }
  .content {
    grid-template-columns: 1fr;
  }
}
</style>
