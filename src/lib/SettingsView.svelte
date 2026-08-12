<script lang="ts">
import type { Snippet } from "svelte";
import { onMount } from "svelte";
import { setSchemaEditorDiscardPrompt } from "$lib/schemaEditorGuard";

type SettingsSection = "general" | "ai" | "plugins" | "schema" | "git";
type RecentProject = { name: string; root: string };
type AiProviderPreset = {
  id: string;
  name: string;
  endpoint: string;
  description: string;
};

const aiProviderPresets: AiProviderPreset[] = [
  {
    id: "lm-studio",
    name: "LM Studio",
    endpoint: "http://127.0.0.1:1234/v1",
    description: "Local models on this computer",
  },
  {
    id: "ollama",
    name: "Ollama",
    endpoint: "http://127.0.0.1:11434/v1",
    description: "Local models on this computer",
  },
  {
    id: "openai",
    name: "OpenAI",
    endpoint: "https://api.openai.com/v1",
    description: "GPT models through OpenAI",
  },
  {
    id: "openrouter",
    name: "OpenRouter",
    endpoint: "https://openrouter.ai/api/v1",
    description: "Many hosted models behind one API",
  },
  {
    id: "groq",
    name: "Groq",
    endpoint: "https://api.groq.com/openai/v1",
    description: "Fast hosted inference",
  },
];

let {
  section = $bindable("general" as SettingsSection),
  recentProjects,
  projectOpen,
  onRemoveRecent,
  onClose,
  onBeforeNavigate,
  plugins,
  schema,
  git,
  aiSettings,
  aiStatus,
  aiModels,
  aiModelsBusy,
  aiModelsMessage,
  aiIndexStatus,
  aiIndexBusy,
  aiIndexMessage,
  remoteCredential,
  onAiRemoteConsent,
  onAiRemoteImport,
  onPortableBackup,
  onRecoveryBackup,
  onRestoreRecoveryBackup,
  onAiSettingsChange,
  onAiCheck,
  onAiModelsLoad,
  onAiIndexRefresh,
  onAiIndexRebuild,
  onAiIndexCancel,
}: {
  section?: SettingsSection;
  recentProjects: RecentProject[];
  projectOpen: boolean;
  onRemoveRecent: (root: string) => void;
  onClose: () => void;
  /** Return false to cancel leaving the current settings section or closing Settings. */
  onBeforeNavigate?: (next: SettingsSection | null) => boolean | Promise<boolean>;
  plugins: Snippet;
  schema: Snippet;
  git: Snippet;
  aiSettings: {
    provider: {
      id: string;
      name: string;
      adapter: string;
      endpoint: string;
      model: string;
      embeddingModel: string;
      capabilities: string[];
    };
    consents: Array<{ projectId: string; provider: string; endpoint: string }>;
  };
  aiStatus: {
    available: boolean;
    modelAvailable: boolean;
    embeddingAvailable: boolean;
    credentialAvailable: boolean;
    error: string | null;
  } | null;
  aiModels: string[];
  aiModelsBusy: boolean;
  aiModelsMessage: string;
  aiIndexStatus: {
    available: boolean;
    state: string | null;
    provider: string | null;
    embeddingAvailable: boolean;
    message: string | null;
  } | null;
  aiIndexBusy: boolean;
  aiIndexMessage: string;
  onAiSettingsChange: (
    key: "id" | "name" | "adapter" | "endpoint" | "model" | "embeddingModel" | "capabilities",
    value: string,
  ) => void;
  onAiCheck: () => void;
  onAiModelsLoad: () => void;
  onAiIndexRefresh: () => void;
  onAiIndexRebuild: () => void;
  onAiIndexCancel: () => void;
  remoteCredential: { configured: boolean } | null;
  onAiRemoteConsent: (allowed: boolean) => void;
  onAiRemoteImport: () => void;
  onPortableBackup: () => Promise<string>;
  onRecoveryBackup: () => Promise<string>;
  onRestoreRecoveryBackup: (path: string) => Promise<void>;
} = $props();

let providerModalOpen = $state(false);
let modelPickerOpen = $state<"chat" | "embedding" | null>(null);
let embeddingSectionOpen = $state(false);
let recoveryPath = $state("");
let storageBusy = $state(false);
let storageMessage = $state("");
let schemaDiscardOpen = $state(false);
let schemaDiscardResolver: ((allowed: boolean) => void) | null = null;

$effect(() => {
  if (aiSettings.provider.embeddingModel.trim()) embeddingSectionOpen = true;
});

onMount(() => {
  // In-app confirm: window.confirm is a silent no-op on macOS Tauri/WKWebView.
  setSchemaEditorDiscardPrompt(
    () =>
      new Promise<boolean>((resolve) => {
        schemaDiscardResolver = resolve;
        schemaDiscardOpen = true;
      }),
  );
  return () => {
    setSchemaEditorDiscardPrompt(null);
    if (schemaDiscardResolver) {
      schemaDiscardResolver(false);
      schemaDiscardResolver = null;
    }
    schemaDiscardOpen = false;
  };
});

function resolveSchemaDiscard(allowed: boolean) {
  schemaDiscardOpen = false;
  schemaDiscardResolver?.(allowed);
  schemaDiscardResolver = null;
}

function chooseModel(kind: "chat" | "embedding", value: string) {
  onAiSettingsChange(kind === "chat" ? "model" : "embeddingModel", value);
  modelPickerOpen = null;
}

function modelValue(kind: "chat" | "embedding") {
  return kind === "chat" ? aiSettings.provider.model : aiSettings.provider.embeddingModel;
}

function matchingModels(kind: "chat" | "embedding") {
  const value = modelValue(kind).trim().toLowerCase();
  return value ? aiModels.filter((model) => model.toLowerCase().includes(value)) : aiModels;
}

function chooseProvider(preset: AiProviderPreset) {
  const changes: Array<["id" | "name" | "endpoint", string]> = [
    ["id", preset.id],
    ["name", preset.name],
    ["endpoint", preset.endpoint],
  ];
  for (const [key, value] of changes) onAiSettingsChange(key, value);
  onAiSettingsChange("model", "");
  onAiSettingsChange("embeddingModel", "");
  onAiSettingsChange("capabilities", "");
}

function chooseCustomProvider() {
  chooseProvider({
    id: "openai-compatible",
    name: "OpenAI-compatible provider",
    endpoint: "",
    description: "Any compatible endpoint",
  });
}

function isRemoteEndpoint(endpoint: string) {
  return endpoint.trim().toLowerCase().startsWith("https://");
}

function toggleCapability(capability: string, enabled: boolean) {
  const capabilities = new Set(aiSettings.provider.capabilities);
  if (enabled) capabilities.add(capability);
  else capabilities.delete(capability);
  onAiSettingsChange("capabilities", [...capabilities].join(", "));
}

function closeModelPickerSoon() {
  window.setTimeout(() => {
    modelPickerOpen = null;
  }, 120);
}

async function createPortableBackup() {
  storageBusy = true;
  try {
    storageMessage = `Portable backup: ${await onPortableBackup()}`;
  } finally {
    storageBusy = false;
  }
}

async function createRecoveryBackup() {
  storageBusy = true;
  try {
    recoveryPath = await onRecoveryBackup();
    storageMessage = `Recovery backup: ${recoveryPath}`;
  } finally {
    storageBusy = false;
  }
}

async function restoreRecoveryBackup() {
  if (!recoveryPath.trim() || !window.confirm("Restore this recovery backup and replace the current runtime state?"))
    return;
  storageBusy = true;
  try {
    await onRestoreRecoveryBackup(recoveryPath.trim());
    storageMessage = "Recovery backup restored.";
  } finally {
    storageBusy = false;
  }
}

async function goToSection(next: SettingsSection) {
  if (next === section) return;
  if (onBeforeNavigate && !(await onBeforeNavigate(next))) return;
  section = next;
}

async function handleClose() {
  if (onBeforeNavigate && !(await onBeforeNavigate(null))) return;
  onClose();
}
</script>

<section class="settings-view" aria-label="Settings">
  <header class="settings-header">
    <div>
      <span class="panel-kicker">APPLICATION</span>
      <h1>Settings</h1>
      <p>App preferences and the plugins that power this project.</p>
    </div>
    <button type="button" class="quiet-button" onclick={() => void handleClose()}>Back</button>
  </header>

  <div class="settings-layout">
    <nav class="settings-nav" aria-label="Settings sections">
      <button
        type="button"
        class:active={section === "general"}
        class="settings-nav-button"
        onclick={() => void goToSection("general")}>General</button>
      <button
        type="button"
        class:active={section === "ai"}
        class="settings-nav-button"
        onclick={() => void goToSection("ai")}>AI</button>
      <button
        type="button"
        class:active={section === "plugins"}
        class="settings-nav-button"
        onclick={() => void goToSection("plugins")}>Plugins</button>
      <button
        type="button"
        class:active={section === "schema"}
        class="settings-nav-button"
        onclick={() => void goToSection("schema")}>Schema</button>
      <button
        type="button"
        class:active={section === "git"}
        class="settings-nav-button"
        onclick={() => void goToSection("git")}>Snapshots</button>
    </nav>

    <div class="settings-panel">
      {#if section === "general"}
        <div class="settings-section-heading">
          <strong>General</strong>
          <p>Recent projects are stored in your application profile.</p>
        </div>
        {#if recentProjects.length === 0}
          <p class="settings-empty">No recent projects yet. Open a project to begin.</p>
        {:else}
          <ul class="settings-recent-list">
            {#each recentProjects as recent}
              <li>
                <div>
                  <strong>{recent.name}</strong>
                  <small>{recent.root}</small>
                </div>
                <button type="button" class="quiet-button" onclick={() => onRemoveRecent(recent.root)}>Remove</button>
              </li>
            {/each}
          </ul>
        {/if}
        {#if projectOpen}
          <div class="settings-section-heading">
            <strong>Project storage</strong>
            <p>
              Portable backups contain canonical files. Recovery backups retain the runtime queue and staged payloads.
            </p>
          </div>
          <div class="settings-actions">
            <button
              type="button"
              class="primary-button"
              disabled={storageBusy}
              onclick={() => void createPortableBackup()}>Portable backup</button>
            <button
              type="button"
              class="quiet-button"
              disabled={storageBusy}
              onclick={() => void createRecoveryBackup()}>Recovery backup</button>
          </div>
          <label class="settings-path-field"
            >Recovery backup path<input
              bind:value={recoveryPath}
              placeholder="Paste a recovery backup directory" /></label>
          <button
            type="button"
            class="quiet-button"
            disabled={storageBusy || !recoveryPath.trim()}
            onclick={() => void restoreRecoveryBackup()}>Restore recovery backup</button>
          {#if storageMessage}<p class="settings-note" role="status">{storageMessage}</p>{/if}
        {/if}
      {:else if section === "ai"}
        <div class="settings-section-heading">
          <strong>AI provider</strong>
          <p>Configure one provider. Every AI action uses the active provider.</p>
        </div>
        <div class="ai-settings-form">
          <section class="ai-settings-card ai-overview-card" aria-labelledby="ai-overview-heading">
            <div class="ai-card-heading">
              <div>
                <span class="ai-card-kicker">ACTIVE PROVIDER</span>
                <strong id="ai-overview-heading">{aiSettings.provider.name || "AI provider"}</strong>
                <p>
                  {aiSettings.provider.endpoint || "No endpoint configured"} · {aiSettings.provider.model ||
                    "No model selected"}
                </p>
              </div>
              <span class="ai-card-badge">{isRemoteEndpoint(aiSettings.provider.endpoint) ? "Remote" : "Local"}</span>
            </div>
            <button type="button" class="primary-button" onclick={() => (providerModalOpen = true)}
              >Configure provider</button>
          </section>
          {#if providerModalOpen}
            <div class="ai-modal-backdrop">
              <div class="ai-provider-modal" role="dialog" aria-modal="true" aria-labelledby="providers-heading">
                <div class="ai-modal-heading">
                  <div>
                    <span class="ai-card-kicker">AI PROVIDER</span>
                    <strong id="providers-heading">Provider configuration</strong>
                    <p>One configured provider powers all generation actions.</p>
                  </div>
                  <button
                    type="button"
                    class="quiet-button"
                    aria-label="Close provider configuration"
                    onclick={() => (providerModalOpen = false)}>Close</button>
                </div>
                <section class="ai-settings-card ai-providers-card">
                  <div class="ai-provider-choice">
                    <div>
                      <span class="ai-card-kicker">START HERE</span>
                      <strong>Choose a provider</strong>
                      <p>These presets fill in the known address. You can change the model below.</p>
                    </div>
                    <div class="ai-provider-presets">
                      {#each aiProviderPresets as preset}
                        <button
                          type="button"
                          class:active={aiSettings.provider.id === preset.id &&
                            aiSettings.provider.endpoint === preset.endpoint}
                          class="ai-provider-preset"
                          onclick={() => chooseProvider(preset)}>
                          <strong>{preset.name}</strong>
                          <span>{preset.description}</span>
                        </button>
                      {/each}
                      <button
                        type="button"
                        class:active={aiSettings.provider.id === "openai-compatible"}
                        class="ai-provider-preset"
                        onclick={chooseCustomProvider}>
                        <strong>OpenAI-compatible</strong>
                        <span>Enter another compatible address</span>
                      </button>
                    </div>
                  </div>
                  <div class="ai-field-grid">
                    <label class="ai-field-wide"
                      >API address<input
                        value={aiSettings.provider.endpoint}
                        placeholder="https://example.com/v1"
                        oninput={(event) =>
                          onAiSettingsChange("endpoint", (event.currentTarget as HTMLInputElement).value)} />
                      {#if aiSettings.provider.id === "openai-compatible" && !isRemoteEndpoint(aiSettings.provider.endpoint)}
                        <span class="ai-field-hint"
                          >Use an HTTPS address for a remote provider; credential controls will appear below.</span>
                      {/if}</label>
                    <label
                      >Chat model
                      <div class="ai-model-field">
                        <input
                          value={aiSettings.provider.model}
                          placeholder="Choose a model or load available models"
                          onfocus={() => (modelPickerOpen = "chat")}
                          onblur={closeModelPickerSoon}
                          oninput={(event) =>
                            onAiSettingsChange("model", (event.currentTarget as HTMLInputElement).value)} />
                        {#if modelPickerOpen === "chat" && matchingModels("chat").length > 0}
                          <div class="ai-model-suggestions" role="listbox">
                            {#each matchingModels("chat") as model}
                              <button
                                type="button"
                                role="option"
                                aria-selected={model === aiSettings.provider.model}
                                onmousedown={(event) => event.preventDefault()}
                                onclick={() => chooseModel("chat", model)}>{model}</button>
                            {/each}
                          </div>
                        {/if}
                      </div></label>
                    <label
                      ><span class="ai-field-label">Embedding model <span class="ai-label-note">optional</span></span>
                      <div class="ai-model-field">
                        <input
                          value={aiSettings.provider.embeddingModel}
                          placeholder="Leave blank to use the chat model"
                          onfocus={() => (modelPickerOpen = "embedding")}
                          onblur={closeModelPickerSoon}
                          oninput={(event) =>
                            onAiSettingsChange("embeddingModel", (event.currentTarget as HTMLInputElement).value)} />
                        {#if modelPickerOpen === "embedding" && matchingModels("embedding").length > 0}
                          <div class="ai-model-suggestions" role="listbox">
                            {#each matchingModels("embedding") as model}
                              <button
                                type="button"
                                role="option"
                                aria-selected={model === aiSettings.provider.embeddingModel}
                                onmousedown={(event) => event.preventDefault()}
                                onclick={() => chooseModel("embedding", model)}>{model}</button>
                            {/each}
                          </div>
                        {/if}
                      </div></label>
                  </div>
                  <section class="ai-capabilities-help" aria-labelledby="ai-capabilities-heading">
                    <strong id="ai-capabilities-heading">Model capabilities</strong>
                    <p>
                      Enable only what the selected model supports. Leave Embedding model blank to reuse the chat model.
                    </p>
                    <div class="ai-capability-options">
                      <label
                        ><input
                          type="checkbox"
                          checked={aiSettings.provider.capabilities.includes("text.embed")}
                          onchange={(event) =>
                            toggleCapability("text.embed", (event.currentTarget as HTMLInputElement).checked)} />
                        <strong>Embeddings</strong> <span>semantic indexing</span></label>
                      <label
                        ><input
                          type="checkbox"
                          checked={aiSettings.provider.capabilities.includes("text.generate.structured")}
                          onchange={(event) =>
                            toggleCapability(
                              "text.generate.structured",
                              (event.currentTarget as HTMLInputElement).checked,
                            )} />
                        <strong>Structured output</strong> <span>typed field suggestions</span></label>
                    </div>
                  </section>
                  <div class="ai-action-group">
                    <span class="ai-action-label">Connection</span>
                    <div class="ai-settings-actions ai-card-actions">
                      <button type="button" class="primary-button" onclick={onAiModelsLoad} disabled={aiModelsBusy}
                        >{aiModelsBusy ? "Loading models…" : "Load available models"}</button>
                      <button type="button" class="quiet-button" onclick={onAiCheck}>Test connection</button>
                      {#if aiStatus}<span class:ok={aiStatus.available && aiStatus.modelAvailable} class="ai-status"
                          >{aiStatus.available && !aiStatus.credentialAvailable
                            ? "Provider reachable; credential missing"
                            : aiStatus.available
                              ? aiStatus.modelAvailable
                                ? `Ready · Embeddings ${aiStatus.embeddingAvailable ? "available" : "not configured"}`
                                : aiSettings.provider.model.trim()
                                  ? "Server found; selected model was not found"
                                  : "Server found; choose a model"
                              : (aiStatus.error ?? "Provider unavailable")}</span
                        >{/if}
                      {#if aiModelsMessage}<span class="ai-status">{aiModelsMessage}</span>{/if}
                    </div>
                  </div>
                  {#if isRemoteEndpoint(aiSettings.provider.endpoint)}
                    <div class="ai-action-group">
                      <span class="ai-action-label">Credentials</span>
                      <div class="ai-settings-actions ai-card-actions">
                        {#if remoteCredential?.configured}
                          <span class="ai-status ai-status-badge ok">Credential stored in OS keychain</span>
                        {:else}<button type="button" class="quiet-button" onclick={onAiRemoteImport}
                            >Import environment key</button
                          ><span class="ai-status ai-status-badge">No OS credential configured</span>{/if}
                      </div>
                    </div>
                    {#if projectOpen}
                      <div class="ai-action-group">
                        <span class="ai-action-label">Project access</span>
                        <div class="ai-settings-actions ai-card-actions">
                          <button type="button" class="primary-button" onclick={() => onAiRemoteConsent(true)}
                            >Allow for this project</button>
                          <button type="button" class="quiet-button" onclick={() => onAiRemoteConsent(false)}
                            >Revoke</button>
                        </div>
                      </div>
                    {/if}
                  {/if}
                </section>
              </div>
            </div>
          {/if}
          <section class="ai-settings-card" aria-labelledby="retrieval-index-heading">
            <div class="ai-card-heading ai-card-heading-compact">
              <div>
                <span class="ai-card-kicker">PROJECT CONTEXT</span>
                <strong id="retrieval-index-heading">Semantic retrieval</strong>
                <p>Embeddings use the active provider when supported; lexical retrieval remains available otherwise.</p>
              </div>
            </div>
            <div class="ai-settings-actions">
              <button type="button" class="quiet-button" onclick={onAiIndexRefresh}>Refresh status</button>
              <button type="button" class="primary-button" onclick={onAiIndexRebuild} disabled={aiIndexBusy}
                >{aiIndexBusy ? "Building index…" : "Build semantic index"}</button>
              {#if aiIndexBusy}<button type="button" class="quiet-button" onclick={onAiIndexCancel}>Cancel</button>{/if}
              {#if aiIndexStatus}<span class="ai-status"
                  >{aiIndexStatus.message ?? aiIndexStatus.state ?? "Unavailable"}</span
                >{/if}
              {#if aiIndexMessage}<span class="ai-status">{aiIndexMessage}</span>{/if}
            </div>
          </section>
        </div>
      {:else if section === "plugins"}
        {#if !projectOpen}
          <div class="settings-section-heading">
            <strong>Plugins</strong>
            <p>Open a project to install, enable, and review plugin capabilities.</p>
          </div>
          <p class="settings-empty">No project is open.</p>
        {:else}
          {@render plugins()}
        {/if}
      {:else if section === "schema"}
        {@render schema()}
      {:else}
        {@render git()}
      {/if}
    </div>
  </div>

  {#if schemaDiscardOpen}
    <div class="schema-discard-backdrop" role="presentation">
      <div class="schema-discard-dialog" role="alertdialog" aria-modal="true" aria-labelledby="schema-discard-title">
        <strong id="schema-discard-title">Discard unsaved schema changes?</strong>
        <p>Your edits to types, fields, and templates will be lost.</p>
        <div class="schema-discard-actions">
          <button type="button" class="quiet-button" onclick={() => resolveSchemaDiscard(false)}>Keep editing</button>
          <button type="button" class="danger-button" onclick={() => resolveSchemaDiscard(true)}>Discard</button>
        </div>
      </div>
    </div>
  {/if}
</section>

<style>
.settings-view {
  display: flex;
  flex-direction: column;
  min-height: calc(100vh - 58px);
  padding: 28px 32px 40px;
  background: var(--canvas);
}
.schema-discard-backdrop {
  position: fixed;
  inset: 0;
  z-index: 200;
  display: grid;
  place-items: center;
  background: rgba(48, 44, 38, 0.35);
}
.schema-discard-dialog {
  width: min(420px, calc(100vw - 32px));
  display: grid;
  gap: 10px;
  padding: 18px 20px;
  border: 1px solid #d9cdbd;
  border-radius: 14px;
  background: #fffefa;
  box-shadow: 0 18px 40px rgba(48, 44, 38, 0.18);
}
.schema-discard-dialog strong {
  font: 600 16px var(--font-display, Georgia, serif);
}
.schema-discard-dialog p {
  margin: 0;
  color: #8f897e;
  font-size: 13px;
  line-height: 1.45;
}
.schema-discard-actions {
  display: flex;
  justify-content: flex-end;
  gap: 8px;
  margin-top: 6px;
}
.danger-button {
  border: 1px solid #c9897d;
  border-radius: 8px;
  padding: 6px 12px;
  background: #f7ebe7;
  color: #9a4d3f;
  font:
    600 11px Inter,
    ui-sans-serif,
    system-ui,
    sans-serif;
  cursor: pointer;
}
.settings-header {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 16px;
  margin-bottom: 28px;
}
.settings-header h1 {
  margin: 6px 0 8px;
  font: 500 34px var(--font-display);
  color: var(--ink);
}
.settings-header p {
  margin: 0;
  max-width: 520px;
  color: var(--ink-soft);
  font-size: 13px;
  line-height: 1.55;
}
.settings-layout {
  display: grid;
  grid-template-columns: 180px minmax(0, 1fr);
  gap: 22px;
  align-items: start;
}
.settings-nav {
  display: flex;
  flex-direction: column;
  gap: 4px;
}
.settings-nav-button {
  width: 100%;
  padding: 10px 12px;
  border: 0;
  border-radius: 8px;
  background: transparent;
  color: var(--ink-soft);
  text-align: left;
  cursor: pointer;
  font-size: 13px;
}
.settings-nav-button.active,
.settings-nav-button:hover {
  background: #efe6d6;
  color: var(--ink);
}
.settings-panel {
  min-width: 0;
  padding: 22px 24px;
  border: 1px solid var(--line);
  border-radius: 14px;
  background: var(--surface);
  box-shadow: var(--shadow-sm, 0 1px 2px rgb(40 40 20 / 4%));
}
.settings-section-heading {
  margin-bottom: 18px;
}
.settings-section-heading strong {
  display: block;
  font-size: 16px;
}
.settings-section-heading p {
  margin: 6px 0 0;
  color: var(--ink-soft);
  font-size: 12px;
  line-height: 1.5;
}
.settings-empty {
  margin: 0;
  color: var(--ink-soft);
  font-size: 13px;
}
.ai-settings-form {
  display: grid;
  gap: 14px;
  width: 100%;
}
.ai-settings-card {
  display: grid;
  gap: 18px;
  padding: 18px;
  border: 1px solid var(--line);
  border-radius: 12px;
  background: #fdfbf7;
}
.ai-overview-card {
  gap: 16px;
}
.ai-modal-backdrop {
  position: fixed;
  inset: 0;
  z-index: 40;
  display: grid;
  place-items: center;
  padding: 24px;
  background: rgba(37, 37, 31, 0.34);
}
.ai-provider-modal {
  width: min(820px, 100%);
  max-height: min(820px, calc(100vh - 48px));
  overflow: auto;
  padding: 22px;
  border: 1px solid #d8cdbd;
  border-radius: 14px;
  background: var(--surface);
  box-shadow: 0 24px 80px rgba(37, 37, 31, 0.24);
}
.ai-provider-modal > .ai-settings-card {
  padding: 0;
  border: 0;
  background: transparent;
  box-shadow: none;
}
.ai-modal-heading {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 18px;
  margin-bottom: 18px;
}
.ai-modal-heading strong {
  display: block;
  margin-top: 5px;
  color: var(--ink);
  font-size: 18px;
}
.ai-modal-heading p {
  max-width: 580px;
  margin: 6px 0 0;
  color: var(--ink-soft);
  font-size: 11px;
  line-height: 1.5;
}
.ai-providers-card {
  gap: 20px;
}
.ai-provider-choice {
  display: grid;
  gap: 12px;
}
.ai-provider-choice strong {
  display: block;
  margin-top: 5px;
  color: var(--ink);
  font-size: 15px;
}
.ai-provider-choice p {
  margin: 6px 0 0;
  color: var(--ink-soft);
  font-size: 11px;
}
.ai-provider-presets {
  display: grid;
  grid-template-columns: repeat(3, minmax(0, 1fr));
  gap: 8px;
}
.ai-provider-preset {
  display: grid;
  gap: 4px;
  min-height: 62px;
  padding: 10px 11px;
  border: 1px solid #d8cdbd;
  border-radius: 9px;
  background: var(--surface);
  color: var(--ink);
  text-align: left;
  cursor: pointer;
}
.ai-provider-preset:hover,
.ai-provider-preset.active {
  border-color: #bc8d5d;
  background: #fbf2e5;
}
.ai-provider-preset strong {
  margin: 0;
  font-size: 12px;
}
.ai-provider-preset span {
  color: var(--ink-soft);
  font-size: 10px;
  line-height: 1.35;
}
.ai-label-note {
  display: inline-block;
  padding: 2px 5px;
  border: 1px solid #d8c6ad;
  border-radius: 999px;
  background: #fbf2e5;
  color: var(--accent-dark);
  font-size: 9px;
  font-weight: 500;
  letter-spacing: 0.03em;
}
.ai-field-hint {
  color: var(--ink-soft);
  font-size: 10px;
  font-weight: 400;
  line-height: 1.35;
}
.ai-field-label {
  display: inline-flex;
  align-items: baseline;
  gap: 5px;
}
.ai-capabilities-help {
  padding-top: 2px;
  color: var(--ink-soft);
  font-size: 11px;
}
.ai-capabilities-help > strong {
  color: var(--accent-dark);
  font-weight: 700;
}
.ai-capabilities-help p {
  margin: 5px 0 10px;
  color: var(--ink-soft);
  line-height: 1.35;
}
.ai-capability-options {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 8px;
}
.ai-settings-form .ai-capability-options label {
  display: flex;
  align-items: center;
  gap: 7px;
  min-width: 0;
  padding: 8px 10px;
  border: 1px solid #e2d8c9;
  border-radius: 8px;
  background: var(--surface);
  color: var(--ink);
  font-size: 11px;
  font-weight: 600;
  white-space: nowrap;
}
.ai-capability-options input {
  min-width: auto;
  padding: 0;
  border: 0;
  box-shadow: none;
  accent-color: var(--accent);
}
.ai-capability-options span {
  color: var(--ink-soft);
  font-weight: 400;
}
.ai-card-heading {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 18px;
}
.ai-card-heading strong {
  display: block;
  margin-top: 5px;
  color: var(--ink);
  font-size: 16px;
}
.ai-card-heading p {
  max-width: 520px;
  margin: 6px 0 0;
  color: var(--ink-soft);
  font-size: 11px;
  line-height: 1.5;
}
.ai-card-heading-compact {
  align-items: center;
}
.ai-card-kicker {
  color: var(--accent);
  font-size: 9px;
  font-weight: 800;
  letter-spacing: 0.12em;
}
.ai-card-badge {
  flex: 0 0 auto;
  padding: 5px 8px;
  border: 1px solid #d8c6ad;
  border-radius: 999px;
  color: var(--accent-dark);
  font-size: 10px;
  font-weight: 700;
  white-space: nowrap;
}
.ai-field-grid {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 14px;
}
.ai-field-wide {
  grid-column: 1 / -1;
}
.ai-settings-form label {
  display: grid;
  gap: 6px;
  color: var(--ink-soft);
  font-size: 11px;
  font-weight: 700;
}
.ai-settings-form input {
  min-width: 0;
  padding: 10px 11px;
  border: 1px solid #d8cdbd;
  border-radius: 8px;
  outline: 0;
  background: var(--surface);
  color: var(--ink);
  font: 12px var(--font-body);
  font-weight: 400;
  transition:
    border-color 0.15s ease,
    box-shadow 0.15s ease;
}
.ai-settings-form input:focus {
  border-color: #bc8d5d;
  box-shadow: 0 0 0 3px rgb(188 141 93 / 12%);
}
.ai-settings-form input:disabled {
  border-color: #e1dcd3;
  background: #f2efe9;
  color: var(--ink-faint);
  cursor: not-allowed;
  opacity: 0.7;
}
.ai-settings-form input:disabled:hover {
  border-color: #e1dcd3;
  box-shadow: none;
}
.ai-model-field {
  position: relative;
}
.ai-model-field input {
  width: 100%;
  box-sizing: border-box;
}
.ai-model-suggestions {
  position: absolute;
  z-index: 5;
  top: calc(100% + 4px);
  right: 0;
  left: 0;
  display: grid;
  max-height: 190px;
  overflow-y: auto;
  padding: 4px;
  border: 1px solid #d8cdbd;
  border-radius: 8px;
  background: var(--surface);
  box-shadow: 0 10px 24px rgb(48 44 38 / 14%);
}
.ai-model-suggestions button {
  padding: 7px 8px;
  border: 0;
  border-radius: 5px;
  background: transparent;
  color: var(--ink);
  font: 11px var(--font-body);
  text-align: left;
  cursor: pointer;
}
.ai-model-suggestions button:hover {
  background: #efe6d6;
}
.ai-settings-actions {
  display: flex;
  align-items: center;
  gap: 8px;
  flex-wrap: wrap;
}
.ai-action-group {
  display: grid;
  gap: 6px;
}
.ai-action-label {
  color: var(--ink-soft);
  font-size: 9px;
  font-weight: 800;
  letter-spacing: 0.1em;
  text-transform: uppercase;
}
.ai-providers-card .primary-button,
.ai-providers-card .quiet-button {
  padding: 8px 11px;
  font-size: 11px;
}
.ai-status-badge {
  display: inline-flex;
  align-items: center;
  min-height: 30px;
  padding: 0 9px;
  border: 1px solid #e2d8c9;
  border-radius: 999px;
  background: #f7f3ec;
  color: #9d5b42;
  font-size: 10px;
  white-space: nowrap;
}
.ai-status-badge.ok {
  border-color: #c8d8cb;
  background: #eef5ef;
  color: #557d63;
}
.settings-actions {
  display: flex;
  gap: 8px;
  flex-wrap: wrap;
  margin: 12px 0;
}
.settings-path-field {
  display: grid;
  gap: 6px;
  margin: 10px 0;
  color: #6d625d;
  font-size: 12px;
}
.settings-path-field input {
  width: 100%;
  box-sizing: border-box;
  padding: 9px 10px;
  border: 1px solid #d9cec7;
  border-radius: 8px;
  background: #fffdfb;
  color: #302a27;
}
.settings-note {
  margin: 10px 0;
  color: #6d625d;
  font-size: 12px;
  overflow-wrap: anywhere;
}
.ai-card-actions {
  padding-top: 2px;
}
.primary-button {
  padding: 10px 15px;
  border: 1px solid rgba(255, 255, 255, 0.08);
  border-radius: 8px;
  background: var(--accent-dark);
  color: #fff;
  font-size: 12px;
  font-weight: 700;
  box-shadow:
    0 2px 0 #263d30,
    0 7px 16px rgba(42, 68, 51, 0.16);
  cursor: pointer;
  transition:
    background 0.16s ease,
    box-shadow 0.16s ease,
    transform 0.16s ease;
}
.primary-button:hover {
  background: #2b4535;
  box-shadow:
    0 2px 0 #263d30,
    0 10px 20px rgba(42, 68, 51, 0.2);
  transform: translateY(-1px);
}
.primary-button:active {
  box-shadow:
    0 1px 0 #263d30,
    0 3px 8px rgba(42, 68, 51, 0.14);
  transform: translateY(1px);
}
.primary-button:focus-visible {
  outline: 3px solid rgba(180, 119, 63, 0.32);
  outline-offset: 2px;
}
.primary-button:disabled {
  opacity: 0.5;
  cursor: not-allowed;
  box-shadow: none;
  transform: none;
}
.quiet-button {
  padding: 10px 12px;
  border: 1px solid #ded8cd;
  border-radius: 8px;
  background: var(--surface);
  color: var(--ink-soft);
  font-size: 12px;
  box-shadow: 0 1px 2px rgba(48, 45, 38, 0.05);
  cursor: pointer;
  transition:
    background 0.16s ease,
    border-color 0.16s ease,
    box-shadow 0.16s ease,
    color 0.16s ease,
    transform 0.16s ease;
}
.quiet-button:hover {
  border-color: #cbbda9;
  background: var(--surface-muted);
  color: var(--ink);
  box-shadow: 0 3px 8px rgba(48, 45, 38, 0.08);
  transform: translateY(-1px);
}
.quiet-button:active {
  box-shadow: 0 1px 2px rgba(48, 45, 38, 0.05);
  transform: translateY(1px);
}
.quiet-button:focus-visible {
  outline: 3px solid rgba(180, 119, 63, 0.24);
  outline-offset: 2px;
}
.quiet-button:disabled {
  opacity: 0.5;
  cursor: not-allowed;
  box-shadow: none;
  transform: none;
}
.ai-status {
  color: #9d5b42;
  font-size: 11px;
  line-height: 1.4;
}
.ai-status.ok {
  color: #557d63;
  font-weight: 700;
}
.settings-recent-list {
  list-style: none;
  margin: 0;
  padding: 0;
  display: grid;
  gap: 10px;
}
.settings-recent-list li {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  padding: 12px 0;
  border-bottom: 1px solid var(--line);
}
.settings-recent-list li:last-child {
  border-bottom: 0;
}
.settings-recent-list strong,
.settings-recent-list small {
  display: block;
}
.settings-recent-list small {
  margin-top: 3px;
  color: var(--ink-soft);
  font-size: 11px;
}
@media (max-width: 760px) {
  .settings-view {
    padding: 18px 16px 28px;
  }
  .settings-layout {
    grid-template-columns: 1fr;
  }
  .settings-nav {
    flex-direction: row;
    flex-wrap: wrap;
  }
  .settings-nav-button {
    width: auto;
  }
  .ai-field-grid {
    grid-template-columns: 1fr;
  }
  .ai-field-wide {
    grid-column: auto;
  }
  .ai-provider-presets {
    grid-template-columns: 1fr 1fr;
  }
  .ai-capability-options {
    grid-template-columns: 1fr;
  }
  .ai-card-heading {
    flex-direction: column;
    gap: 10px;
  }
  .ai-modal-backdrop {
    padding: 12px;
  }
  .ai-provider-modal {
    max-height: calc(100vh - 24px);
    padding: 17px;
  }
  .ai-modal-heading {
    flex-direction: column;
    gap: 10px;
  }
}
</style>
