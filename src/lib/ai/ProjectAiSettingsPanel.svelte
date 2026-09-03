<script lang="ts">
import { Bot, Cloud, Cpu, KeyRound, Plus, Server, Sparkles, Trash2 } from "@lucide/svelte";
import ImageProviderSettingsCard from "$lib/ai/ImageProviderSettingsCard.svelte";
import {
  mergePromptTemplates,
  overlayFromTemplates,
  type PromptKind,
  type PromptTemplate,
} from "$lib/ai/promptTemplates";
import type { AiProviderSettings, AiProviderStatus, ImageProviderSettings } from "$lib/project/types";

type AiProviderPreset = { id: string; name: string; endpoint: string; description: string; local: boolean };

const aiProviderPresets: AiProviderPreset[] = [
  {
    id: "lm-studio",
    name: "LM Studio",
    endpoint: "http://127.0.0.1:1234/v1",
    description: "Local models on this computer",
    local: true,
  },
  {
    id: "ollama",
    name: "Ollama",
    endpoint: "http://127.0.0.1:11434/v1",
    description: "Local models on this computer",
    local: true,
  },
  {
    id: "openai",
    name: "OpenAI",
    endpoint: "https://api.openai.com/v1",
    description: "GPT models through OpenAI",
    local: false,
  },
  {
    id: "openrouter",
    name: "OpenRouter",
    endpoint: "https://openrouter.ai/api/v1",
    description: "Many hosted models behind one API",
    local: false,
  },
  {
    id: "groq",
    name: "Groq",
    endpoint: "https://api.groq.com/openai/v1",
    description: "Fast hosted inference",
    local: false,
  },
];

let {
  enabled,
  provider,
  imageProvider,
  aiStatus,
  aiModels,
  aiModelsBusy,
  aiModelsMessage,
  remoteCredential,
  remoteConsent,
  aiIndexStatus,
  aiIndexBusy,
  aiIndexMessage,
  templates,
  onToggleAi,
  onProviderChange,
  onImageChange,
  onConnect,
  onRemoteImport,
  onRemoteSave,
  onRemoteClear,
  onRemoteConsent,
  onAiIndexRefresh,
  onAiIndexRebuild,
  onAiIndexCancel,
  onTemplatesChange,
}: {
  enabled: boolean;
  provider: AiProviderSettings;
  imageProvider: ImageProviderSettings;
  aiStatus: AiProviderStatus | null;
  aiModels: string[];
  aiModelsBusy: boolean;
  aiModelsMessage: string;
  remoteCredential: { configured: boolean } | null;
  remoteConsent: boolean;
  aiIndexStatus: { state: string | null; message: string | null } | null;
  aiIndexBusy: boolean;
  aiIndexMessage: string;
  templates: PromptTemplate[];
  onToggleAi: (enabled: boolean) => void;
  onProviderChange: (key: keyof AiProviderSettings, value: string | string[]) => void;
  onImageChange: (key: keyof ImageProviderSettings, value: string | boolean) => void;
  onConnect: () => void;
  onRemoteImport: () => void;
  onRemoteSave: (apiKey: string) => Promise<boolean>;
  onRemoteClear: () => void;
  onRemoteConsent: (allowed: boolean) => void;
  onAiIndexRefresh: () => void;
  onAiIndexRebuild: () => void;
  onAiIndexCancel: () => void;
  onTemplatesChange: (templates: PromptTemplate[]) => void;
} = $props();

let credentialInput = $state("");
let credentialBusy = $state(false);
let selectedTemplateId = $state("rewrite");
let customLabel = $state("");
let customInstruction = $state("");

const remote = $derived(provider.endpoint.trim().toLowerCase().startsWith("https://"));
const selectedTemplate = $derived(templates.find((template) => template.id === selectedTemplateId) ?? templates[0]);
const statusReady = $derived(Boolean(aiStatus?.available && aiStatus?.modelAvailable));
const templateGroups = $derived(
  (
    [
      { kind: "editor" as PromptKind, label: "Editor" },
      { kind: "git" as PromptKind, label: "Snapshots" },
      { kind: "image" as PromptKind, label: "Images" },
    ] as const
  )
    .map((group) => ({ ...group, items: templates.filter((template) => template.kind === group.kind) }))
    .filter((group) => group.items.length > 0),
);

function chooseProvider(preset: AiProviderPreset) {
  onProviderChange("id", preset.id);
  onProviderChange("name", preset.name);
  onProviderChange("endpoint", preset.endpoint);
  onProviderChange("model", "");
  onProviderChange("embeddingModel", "");
  onProviderChange("capabilities", []);
}

function chooseCustomProvider() {
  chooseProvider({
    id: "openai-compatible",
    name: "OpenAI-compatible provider",
    endpoint: "",
    description: "Any compatible endpoint",
    local: true,
  });
}

function toggleCapability(capability: string, on: boolean) {
  const capabilities = new Set(provider.capabilities);
  if (on) capabilities.add(capability);
  else capabilities.delete(capability);
  onProviderChange("capabilities", [...capabilities]);
}

async function saveRemoteCredential() {
  const key = credentialInput.trim();
  if (!key || credentialBusy) return;
  credentialBusy = true;
  try {
    if (await onRemoteSave(key)) credentialInput = "";
  } finally {
    credentialBusy = false;
  }
}

function updateSelected(patch: Partial<PromptTemplate>) {
  if (!selectedTemplate) return;
  onTemplatesChange(
    templates.map((template) => (template.id === selectedTemplate.id ? { ...template, ...patch } : template)),
  );
}

function restoreSelected() {
  if (!selectedTemplate?.bundled) return;
  const restored = mergePromptTemplates({
    templates: overlayFromTemplates(templates).templates?.filter((item) => item.id !== selectedTemplate.id),
  });
  onTemplatesChange(restored);
  selectedTemplateId = selectedTemplate.id;
}

function removeSelected() {
  if (!selectedTemplate || selectedTemplate.bundled) return;
  const next = templates.filter((template) => template.id !== selectedTemplate.id);
  onTemplatesChange(next);
  selectedTemplateId = next[0]?.id ?? "rewrite";
}

function addCustom() {
  const label = customLabel.trim();
  const instruction = customInstruction.trim();
  if (!label || !instruction) return;
  const id = `custom-${crypto.randomUUID().slice(0, 8)}`;
  onTemplatesChange([
    ...templates,
    { id, label, instruction, kind: "editor" as PromptKind, requiresSelection: false, enabled: true, bundled: false },
  ]);
  selectedTemplateId = id;
  customLabel = "";
  customInstruction = "";
}

function statusCopy() {
  if (!aiStatus) return aiModelsMessage;
  if (aiStatus.available && !aiStatus.credentialAvailable) return "Provider reachable; credential missing";
  if (aiStatus.available && aiStatus.modelAvailable) {
    return `Ready · Embeddings ${aiStatus.embeddingAvailable ? "available" : "not configured"}`;
  }
  if (aiStatus.available)
    return provider.model.trim() ? "Server found; selected model was not found" : "Server found; choose a model";
  return aiStatus.error ?? "Provider unavailable";
}
</script>

<div class="ai-panel">
  <div class="section-heading">
    <span class="heading-icon"><Sparkles size={17} /></span>
    <div>
      <span class="kicker">INTELLIGENCE</span>
      <h2>AI</h2>
      <p>Provider, models, and prompt templates for this project. Nothing is configured until you add it.</p>
    </div>
  </div>

  <section class="operation-card">
    <div class="split-heading">
      <div>
        <strong>Use AI in this project</strong>
        <p>
          Requests run through this project's provider. Remote providers still need consent before context leaves this
          machine.
        </p>
      </div>
      <span class:ok={enabled} class="state-pill">{enabled ? "Enabled" : "Off"}</span>
    </div>
    <div class="action-row">
      <button type="button" class={enabled ? "quiet-button" : "primary-button"} onclick={() => onToggleAi(!enabled)}>
        <Bot size={14} />
        {enabled ? "Disable AI" : "Enable AI"}
      </button>
    </div>
    {#if remote && enabled}
      <div class="sub-operation">
        <div class="split-heading">
          <div>
            <strong>Remote access</strong>
            <p>Allow this provider to receive project context from this machine.</p>
          </div>
          <span class:ok={remoteConsent} class="state-pill">{remoteConsent ? "Allowed" : "Not allowed"}</span>
        </div>
        <div class="action-row">
          <button type="button" class="quiet-button" onclick={() => onRemoteConsent(true)}
            >Allow remote provider</button>
          <button type="button" class="quiet-button" onclick={() => onRemoteConsent(false)}>Revoke</button>
        </div>
      </div>
    {/if}
  </section>

  <section class="operation-card">
    <div>
      <strong>Provider</strong>
      <p>Choose a preset to fill a suggested address, then Connect to test it and list models.</p>
    </div>
    <div class="presets" role="group" aria-label="Provider presets">
      {#each aiProviderPresets as preset}
        <button
          type="button"
          class:active={provider.id === preset.id}
          aria-pressed={provider.id === preset.id}
          onclick={() => chooseProvider(preset)}>
          {#if preset.local}
            <Cpu size={16} strokeWidth={1.8} aria-hidden="true" />
          {:else}
            <Cloud size={16} strokeWidth={1.8} aria-hidden="true" />
          {/if}
          <span><strong>{preset.name}</strong><small>{preset.description}</small></span>
        </button>
      {/each}
      <button
        type="button"
        class:active={provider.id === "openai-compatible"}
        aria-pressed={provider.id === "openai-compatible"}
        onclick={chooseCustomProvider}>
        <Server size={16} strokeWidth={1.8} aria-hidden="true" />
        <span><strong>OpenAI-compatible</strong><small>Enter another compatible address</small></span>
      </button>
    </div>
    <label class="field"
      >API address
      <input
        value={provider.endpoint}
        placeholder="http://127.0.0.1:1234/v1"
        spellcheck="false"
        oninput={(event) => onProviderChange("endpoint", event.currentTarget.value)} />
    </label>
    <div class="field-row">
      <label class="field"
        >Chat model
        <input
          value={provider.model}
          list="ai-model-options"
          placeholder="Connect to list models"
          oninput={(event) => onProviderChange("model", event.currentTarget.value)} />
      </label>
      <label class="field"
        >Embedding model
        <input
          value={provider.embeddingModel}
          list="ai-model-options"
          placeholder="Leave blank to reuse chat"
          oninput={(event) => onProviderChange("embeddingModel", event.currentTarget.value)} />
      </label>
    </div>
    <datalist id="ai-model-options">
      {#each aiModels as model}<option value={model}></option>{/each}
    </datalist>
    <div class="capability-row">
      <label class="check-chip">
        <input
          type="checkbox"
          checked={provider.capabilities.includes("text.embed")}
          onchange={(event) => toggleCapability("text.embed", event.currentTarget.checked)} />
        Embeddings
      </label>
      <label class="check-chip">
        <input
          type="checkbox"
          checked={provider.capabilities.includes("text.generate.structured")}
          onchange={(event) => toggleCapability("text.generate.structured", event.currentTarget.checked)} />
        Structured output
      </label>
    </div>
    <div class="action-row">
      <button
        type="button"
        class="primary-button"
        disabled={aiModelsBusy || !provider.endpoint.trim()}
        onclick={onConnect}>
        {aiModelsBusy ? "Connecting…" : "Connect"}
      </button>
      {#if aiStatus || aiModelsMessage}
        <span class="status-badge" class:ok={statusReady} role="status">{statusCopy()}</span>
      {/if}
    </div>
    {#if remote}
      <div class="sub-operation">
        <div class="split-heading">
          <div>
            <strong>Credentials</strong>
            <p>{remoteCredential?.configured ? "Stored in the OS keychain." : "No OS credential configured."}</p>
          </div>
          {#if remoteCredential?.configured}
            <span class="state-pill ok">Configured</span>
          {/if}
        </div>
        <div class="credential-row">
          <label class="field credential-field"
            >API key
            <input
              type="password"
              bind:value={credentialInput}
              placeholder="Paste a key"
              autocomplete="off"
              spellcheck="false" />
          </label>
          <button
            type="button"
            class="primary-button"
            disabled={!credentialInput.trim() || credentialBusy}
            onclick={() => void saveRemoteCredential()}>
            <KeyRound size={14} />
            {remoteCredential?.configured ? "Replace key" : "Save key"}
          </button>
          {#if remoteCredential?.configured}
            <button type="button" class="quiet-button" onclick={onRemoteClear}>Remove</button>
          {:else}
            <button type="button" class="quiet-button" onclick={onRemoteImport}>Import environment key</button>
          {/if}
        </div>
      </div>
    {/if}
  </section>

  {#if enabled}
    <section class="operation-card">
      <div>
        <strong>Semantic retrieval</strong>
        <p>
          {aiIndexStatus?.message ?? aiIndexStatus?.state ?? "Status not checked"}{aiIndexMessage
            ? ` · ${aiIndexMessage}`
            : ""}
        </p>
      </div>
      <div class="action-row">
        <button type="button" class="quiet-button" onclick={onAiIndexRefresh}>Refresh</button>
        <button type="button" class="primary-button" disabled={aiIndexBusy} onclick={onAiIndexRebuild}
          >{aiIndexBusy ? "Building index…" : "Build semantic index"}</button>
        {#if aiIndexBusy}<button type="button" class="quiet-button" onclick={onAiIndexCancel}>Cancel</button>{/if}
      </div>
    </section>
  {/if}

  <ImageProviderSettingsCard settings={imageProvider} onChange={(key, value) => onImageChange(key, value)} />

  <section class="operation-card">
    <div>
      <strong>Prompt templates</strong>
      <p>Task instructions sent with each action. Host safety rules stay in Daena and are not editable.</p>
    </div>
    <div class="template-layout">
      <nav class="template-list" aria-label="Prompt templates">
        {#each templateGroups as group}
          <p class="list-kicker">{group.label}</p>
          {#each group.items as template}
            <button
              type="button"
              class:active={template.id === selectedTemplateId}
              onclick={() => (selectedTemplateId = template.id)}>
              <span>{template.label}</span>
              {#if template.enabled === false}<small>off</small>{/if}
            </button>
          {/each}
        {/each}
      </nav>
      {#if selectedTemplate}
        <div class="template-editor">
          <label class="field"
            >Name
            <input
              value={selectedTemplate.label}
              oninput={(event) => updateSelected({ label: event.currentTarget.value })} />
          </label>
          <label class="field"
            >Instruction
            <textarea
              rows="7"
              value={selectedTemplate.instruction}
              oninput={(event) => updateSelected({ instruction: event.currentTarget.value })}></textarea>
          </label>
          <div class="capability-row">
            <label class="check-chip">
              <input
                type="checkbox"
                checked={selectedTemplate.enabled !== false}
                onchange={(event) => updateSelected({ enabled: event.currentTarget.checked })} />
              Show in menus
            </label>
            {#if selectedTemplate.kind === "editor"}
              <label class="check-chip">
                <input
                  type="checkbox"
                  checked={selectedTemplate.requiresSelection === true}
                  onchange={(event) => updateSelected({ requiresSelection: event.currentTarget.checked })} />
                Requires a selection
              </label>
            {/if}
          </div>
          <div class="action-row">
            {#if selectedTemplate.bundled}
              <button type="button" class="quiet-button" onclick={restoreSelected}>Restore default</button>
            {:else}
              <button type="button" class="quiet-button" onclick={removeSelected}><Trash2 size={14} /> Remove</button>
            {/if}
          </div>
        </div>
      {/if}
    </div>
    <div class="add-custom">
      <strong>Add a template</strong>
      <div class="field-row">
        <label class="field">Name <input bind:value={customLabel} placeholder="House voice" /></label>
      </div>
      <label class="field"
        >Instruction <textarea rows="3" bind:value={customInstruction} placeholder="Write in this project's voice."
        ></textarea
        ></label>
      <button
        type="button"
        class="quiet-button"
        disabled={!customLabel.trim() || !customInstruction.trim()}
        onclick={addCustom}><Plus size={14} /> Add template</button>
    </div>
  </section>
</div>

<style>
.ai-panel {
  display: grid;
  gap: 16px;
}
.section-heading {
  display: flex;
  gap: 14px;
  padding: 16px;
  border: 1px solid var(--line);
  border-radius: 14px;
  background: var(--surface);
}
.heading-icon {
  display: grid;
  place-items: center;
  width: 38px;
  height: 38px;
  flex: 0 0 38px;
  border-radius: 10px;
  background: var(--surface-warm);
  color: var(--accent-dark);
}
.kicker,
.list-kicker {
  color: var(--accent);
  font-size: 10px;
  font-weight: 800;
  letter-spacing: 0.1em;
}
.section-heading h2 {
  margin: 4px 0 6px;
  color: var(--ink);
  font: 600 19px var(--font-display);
}
.section-heading p,
.operation-card p {
  margin: 0;
  color: var(--ink-soft);
  font-size: 12.5px;
  line-height: 1.5;
}
.operation-card {
  display: grid;
  gap: 14px;
  padding: 16px;
  border: 1px solid var(--line);
  border-radius: 12px;
  background: var(--surface-subtle);
}
.operation-card > div > strong,
.add-custom > strong {
  display: block;
  margin-bottom: 4px;
  color: var(--ink);
  font-size: 14px;
}
.split-heading {
  display: flex;
  justify-content: space-between;
  align-items: flex-start;
  gap: 16px;
}
.state-pill,
.status-badge {
  display: inline-flex;
  align-items: center;
  align-self: start;
  min-height: 24px;
  padding: 4px 8px;
  border-radius: 999px;
  background: var(--surface-warm);
  color: var(--ink-muted);
  font-size: 10px;
  font-weight: 700;
  line-height: 1.3;
}
.state-pill.ok,
.status-badge.ok {
  background: var(--theme-success-bg, var(--accent-bg));
  color: var(--success);
}
.status-badge {
  max-width: 42rem;
  background: var(--surface);
  border: 1px solid var(--line);
  font-weight: 600;
  color: var(--ink-soft);
}
.action-row,
.credential-row,
.capability-row,
.field-row {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
  align-items: center;
}
.field-row {
  align-items: stretch;
}
.sub-operation {
  display: grid;
  gap: 10px;
  padding-top: 14px;
  border-top: 1px solid var(--line);
}
.presets {
  display: grid;
  grid-template-columns: repeat(3, minmax(0, 1fr));
  gap: 8px;
}
.presets button {
  min-width: 0;
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 11px 12px;
  border: 1px solid var(--line-strong, var(--line));
  border-radius: 10px;
  background: var(--surface);
  color: var(--ink-soft);
  text-align: left;
  cursor: pointer;
  transition:
    border-color 0.14s ease,
    background 0.14s ease,
    color 0.14s ease;
}
.presets button:hover {
  border-color: var(--accent-soft, var(--accent));
  background: var(--surface-warm);
  color: var(--ink);
}
.presets button.active {
  border-color: var(--accent);
  background: var(--accent-bg);
  color: var(--ink);
  box-shadow: inset 0 0 0 1px color-mix(in srgb, var(--accent) 24%, transparent);
}
.presets button span,
.presets button strong,
.presets button small {
  display: block;
  min-width: 0;
}
.presets button strong {
  overflow: hidden;
  font-size: 12px;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.presets button small {
  margin-top: 2px;
  color: var(--ink-faint);
  font-size: 10.5px;
  line-height: 1.35;
}
.field {
  display: grid;
  gap: 6px;
  flex: 1;
  min-width: 0;
  color: var(--ink-soft);
  font-size: 11px;
  font-weight: 700;
}
.field input,
.credential-row input {
  width: 100%;
  min-height: 36px;
  padding: 8px 10px;
  border: 1px solid var(--line);
  border-radius: 8px;
  background: var(--surface);
  color: var(--ink);
  font: 400 12.5px var(--font-body, Inter, sans-serif);
  box-sizing: border-box;
}
.field input:focus,
.credential-row input:focus,
.field textarea:focus {
  border-color: var(--accent);
  box-shadow: 0 0 0 3px color-mix(in srgb, var(--accent) 16%, transparent);
  outline: none;
}
.check-chip {
  display: inline-flex;
  align-items: center;
  gap: 8px;
  min-height: 34px;
  padding: 6px 10px;
  border: 1px solid var(--line);
  border-radius: 9px;
  background: var(--surface);
  color: var(--ink);
  font-size: 12px;
  font-weight: 600;
  cursor: pointer;
}
.primary-button,
.quiet-button {
  display: inline-flex;
  align-items: center;
  gap: 7px;
  min-height: 34px;
  padding: 8px 12px;
  border-radius: 8px;
  font-size: 12px;
  font-weight: 650;
  cursor: pointer;
}
.primary-button {
  border: 1px solid var(--accent-dark);
  background: var(--accent-dark);
  color: var(--on-accent);
}
.quiet-button {
  border: 1px solid var(--line);
  background: var(--surface);
  color: var(--ink-soft);
}
.primary-button:hover:not(:disabled) {
  filter: brightness(1.05);
}
.quiet-button:hover:not(:disabled) {
  border-color: var(--accent-soft, var(--line-strong, var(--line)));
  background: var(--surface-warm);
}
button:disabled {
  opacity: 0.55;
  cursor: not-allowed;
}
.credential-field {
  flex: 1 1 220px;
}
.template-layout {
  display: grid;
  grid-template-columns: 200px minmax(0, 1fr);
  gap: 12px;
  align-items: start;
}
.template-list {
  display: grid;
  gap: 4px;
  align-content: start;
  padding: 8px;
  border: 1px solid var(--line);
  border-radius: 11px;
  background: var(--surface);
}
.list-kicker {
  margin: 8px 8px 2px;
  color: var(--ink-faint);
}
.list-kicker:first-child {
  margin-top: 2px;
}
.template-list button {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
  width: 100%;
  padding: 8px 10px;
  border: 1px solid transparent;
  border-radius: 8px;
  background: transparent;
  color: var(--ink-soft);
  font-size: 12.5px;
  font-weight: 600;
  text-align: left;
  cursor: pointer;
}
.template-list button:hover {
  background: var(--surface-warm);
  color: var(--ink);
}
.template-list button.active {
  border-color: var(--accent);
  background: var(--accent-bg);
  color: var(--ink);
}
.template-list small {
  color: var(--ink-faint);
  font-size: 10px;
  font-weight: 700;
  text-transform: uppercase;
}
.template-editor,
.add-custom {
  display: grid;
  gap: 10px;
}
.template-editor {
  padding: 14px;
  border: 1px solid var(--line);
  border-radius: 11px;
  background: var(--surface);
}
.add-custom {
  padding-top: 14px;
  border-top: 1px solid var(--line);
}
@media (max-width: 860px) {
  .presets,
  .template-layout {
    grid-template-columns: 1fr;
  }
}
</style>
