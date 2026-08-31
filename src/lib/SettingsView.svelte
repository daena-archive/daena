<script lang="ts">
import {
  X,
  Settings2,
  FolderOpen,
  Sparkles,
  Bot,
  ChevronLeft,
  Globe,
  Sun,
  Moon,
  Monitor,
  Download,
} from "@lucide/svelte";
import ImageProviderSettingsCard from "$lib/ai/ImageProviderSettingsCard.svelte";
import { checkAppUpdate, openDownloadPage } from "$lib/appUpdate";
import type { ThemePreference } from "$lib/theme";

type SettingsSection = "general" | "ai";
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
  version,
  recentProjects,
  themePreference,
  onThemeChange,
  onRemoveRecent,
  onClose,
  onBeforeNavigate,
  aiSettings,
  aiStatus,
  aiModels,
  aiModelsBusy,
  aiModelsMessage,
  remoteCredential,
  onAiRemoteImport,
  onAiRemoteSave,
  onAiRemoteClear,
  onAiSettingsChange,
  onAiImageSettingsChange,
  onAiCheck,
  onAiModelsLoad,
}: {
  section?: SettingsSection;
  version: string;
  recentProjects: RecentProject[];
  themePreference: ThemePreference;
  onThemeChange: (preference: ThemePreference) => void;
  onRemoveRecent: (root: string) => void;
  onClose: () => void;
  /** Return false to cancel leaving the current settings section or closing Settings. */
  onBeforeNavigate?: (next: SettingsSection | null) => boolean | Promise<boolean>;
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
    imageProvider: {
      enabled: boolean;
      id: string;
      name: string;
      adapter: string;
      endpoint: string;
      model: string;
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
  onAiSettingsChange: (
    key: "id" | "name" | "adapter" | "endpoint" | "model" | "embeddingModel" | "capabilities",
    value: string,
  ) => void;
  onAiImageSettingsChange: (
    key: "enabled" | "id" | "name" | "adapter" | "endpoint" | "model",
    value: string | boolean,
  ) => void;
  onAiCheck: () => void;
  onAiModelsLoad: () => void;
  remoteCredential: { configured: boolean } | null;
  onAiRemoteImport: () => void;
  /** Returns true when the credential was stored, so the input can be cleared. */
  onAiRemoteSave: (apiKey: string) => Promise<boolean>;
  onAiRemoteClear: () => void;
} = $props();

let providerModalOpen = $state(false);
let modelPickerOpen = $state<"chat" | "embedding" | null>(null);
let embeddingSectionOpen = $state(false);
let credentialInput = $state("");
let credentialBusy = $state(false);
let updateBusy = $state(false);
let updateMessage = $state("");

async function checkForUpdate() {
  if (updateBusy) return;
  updateBusy = true;
  updateMessage = "";
  try {
    const result = await checkAppUpdate();
    updateMessage = result.newer ? `Update available: ${result.latest}` : `You're up to date (${result.current})`;
  } catch (cause) {
    updateMessage = cause instanceof Error ? cause.message : String(cause);
  } finally {
    updateBusy = false;
  }
}

async function openUpdatesPage() {
  try {
    await openDownloadPage();
  } catch (cause) {
    updateMessage = cause instanceof Error ? cause.message : String(cause);
  }
}

async function saveRemoteCredential() {
  const key = credentialInput.trim();
  if (!key || credentialBusy) return;
  credentialBusy = true;
  try {
    if (await onAiRemoteSave(key)) credentialInput = "";
  } finally {
    credentialBusy = false;
  }
}

$effect(() => {
  if (aiSettings.provider.embeddingModel.trim()) embeddingSectionOpen = true;
});

$effect(() => {
  if (!providerModalOpen) return;
  const onKey = (event: KeyboardEvent) => {
    if (event.key === "Escape") {
      event.preventDefault();
      providerModalOpen = false;
    }
  };
  window.addEventListener("keydown", onKey, true);
  return () => window.removeEventListener("keydown", onKey, true);
});

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
    <div class="header-left">
      <div class="header-icon">
        <Settings2 size={18} strokeWidth={1.8} aria-hidden="true" />
      </div>
      <div>
        <span class="panel-kicker">APPLICATION</span>
        <h1>Settings</h1>
        <p>Appearance and provider preferences that follow you across projects.</p>
      </div>
    </div>
    <button type="button" class="quiet-button header-back" onclick={() => void handleClose()}
      ><ChevronLeft size={14} strokeWidth={1.9} aria-hidden="true" /> Back</button>
  </header>

  <div class="settings-layout">
    <nav class="settings-nav" aria-label="Settings sections">
      <button
        type="button"
        class:active={section === "general"}
        class="settings-nav-button"
        onclick={() => void goToSection("general")}
        ><FolderOpen size={14} strokeWidth={1.8} aria-hidden="true" /> General</button>
      <button
        type="button"
        class:active={section === "ai"}
        class="settings-nav-button"
        onclick={() => void goToSection("ai")}><Sparkles size={14} strokeWidth={1.8} aria-hidden="true" /> AI</button>
    </nav>

    <div class="settings-panel">
      {#if section === "general"}
        <div class="panel-hero">
          <div class="hero-icon">
            <FolderOpen size={18} strokeWidth={1.8} aria-hidden="true" />
          </div>
          <div class="hero-copy">
            <span class="kicker">APPLICATION</span>
            <strong>General</strong>
            <p>Appearance and recent projects. These preferences follow you across projects.</p>
          </div>
          <div class="hero-stats">
            <span class="stat-pill"
              ><FolderOpen size={12} strokeWidth={1.8} aria-hidden="true" /> {recentProjects.length} recent</span>
          </div>
        </div>

        <div class="block elevated appearance-block">
          <div class="block-heading">
            <div class="heading-left">
              <span class="heading-icon accent"><Sun size={14} strokeWidth={1.8} aria-hidden="true" /></span>
              <h4>Appearance</h4>
            </div>
            <span class="block-hint">Follows your preference across projects</span>
          </div>
          <div class="theme-options" role="group" aria-label="Color theme">
            <button
              type="button"
              class:active={themePreference === "light"}
              aria-pressed={themePreference === "light"}
              onclick={() => onThemeChange("light")}>
              <Sun size={16} strokeWidth={1.8} aria-hidden="true" />
              <span><strong>Light</strong><small>Warm paper</small></span>
            </button>
            <button
              type="button"
              class:active={themePreference === "dark"}
              aria-pressed={themePreference === "dark"}
              onclick={() => onThemeChange("dark")}>
              <Moon size={16} strokeWidth={1.8} aria-hidden="true" />
              <span><strong>Dark</strong><small>Forest night</small></span>
            </button>
            <button
              type="button"
              class:active={themePreference === "system"}
              aria-pressed={themePreference === "system"}
              onclick={() => onThemeChange("system")}>
              <Monitor size={16} strokeWidth={1.8} aria-hidden="true" />
              <span><strong>System</strong><small>Match this computer</small></span>
            </button>
          </div>
        </div>

        <div class="block elevated">
          <div class="block-heading">
            <div class="heading-left">
              <span class="heading-icon"><Download size={14} strokeWidth={1.8} aria-hidden="true" /></span>
              <h4>About</h4>
            </div>
            <span class="block-hint">v{version}</span>
          </div>
          <div class="ai-settings-actions">
            <button type="button" class="primary" onclick={() => void checkForUpdate()} disabled={updateBusy}
              >{updateBusy ? "Checking…" : "Check for update"}</button>
            <button type="button" class="quiet" onclick={() => void openUpdatesPage()}>Open download page</button>
            {#if updateMessage}<span class="ai-status">{updateMessage}</span>{/if}
          </div>
        </div>

        <div class="block elevated">
          <div class="block-heading">
            <div class="heading-left">
              <span class="heading-icon"><FolderOpen size={14} strokeWidth={1.8} aria-hidden="true" /></span>
              <h4>Recent projects</h4>
              <span class="count-badge">{recentProjects.length}</span>
            </div>
            <span class="block-hint">Stored in your application profile</span>
          </div>
          {#if recentProjects.length === 0}
            <div class="empty-inline">
              <FolderOpen size={16} strokeWidth={1.7} aria-hidden="true" />
              <div>
                <strong>No recent projects yet</strong>
                <span>Open a project to begin. Recent projects appear here for quick access.</span>
              </div>
            </div>
          {:else}
            <ul class="settings-recent-list">
              {#each recentProjects as recent}
                <li>
                  <div class="recent-copy">
                    <strong>{recent.name}</strong>
                    <small>{recent.root}</small>
                  </div>
                  <button
                    type="button"
                    class="quiet icon"
                    aria-label="Remove {recent.name}"
                    onclick={() => onRemoveRecent(recent.root)}
                    ><X size={14} strokeWidth={1.8} aria-hidden="true" /></button>
                </li>
              {/each}
            </ul>
          {/if}
        </div>
      {:else if section === "ai"}
        <div class="panel-hero">
          <div class="hero-icon">
            <Sparkles size={18} strokeWidth={1.8} aria-hidden="true" />
          </div>
          <div class="hero-copy">
            <span class="kicker">INTELLIGENCE</span>
            <strong>AI provider</strong>
            <p>Configure one provider. Every generation, rewrite, and field-fill uses the active provider.</p>
          </div>
          <div class="hero-stats">
            <span class="stat-pill"
              ><Bot size={12} strokeWidth={1.8} aria-hidden="true" /> {aiSettings.provider.name || "No provider"}</span>
            <span class="stat-pill"
              ><Globe size={12} strokeWidth={1.8} aria-hidden="true" /> {aiSettings.provider.model || "No model"}</span>
          </div>
        </div>

        <div class="ai-settings-form">
          <section class="ai-settings-card ai-overview-card elevated" aria-labelledby="ai-overview-heading">
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
            <button type="button" class="primary" onclick={() => (providerModalOpen = true)}
              ><Settings2 size={14} strokeWidth={1.8} aria-hidden="true" /> Configure provider</button>
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
                    class="quiet"
                    aria-label="Close provider configuration"
                    onclick={() => (providerModalOpen = false)}
                    ><X size={14} strokeWidth={1.8} aria-hidden="true" /> Close</button>
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
                      <button type="button" class="primary" onclick={onAiModelsLoad} disabled={aiModelsBusy}
                        >{aiModelsBusy ? "Loading models…" : "Load available models"}</button>
                      <button type="button" class="quiet" onclick={onAiCheck}>Test connection</button>
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
                          <button type="button" class="quiet" onclick={onAiRemoteClear}>Remove</button>
                        {:else}
                          <span class="ai-status ai-status-badge">No OS credential configured</span>
                        {/if}
                      </div>
                      <div class="credential-row">
                        <input
                          type="password"
                          bind:value={credentialInput}
                          placeholder="API key"
                          autocomplete="off"
                          spellcheck="false"
                          onkeydown={(event) => event.key === "Enter" && void saveRemoteCredential()} />
                        <button
                          type="button"
                          class="primary"
                          disabled={!credentialInput.trim() || credentialBusy}
                          onclick={() => void saveRemoteCredential()}>
                          {remoteCredential?.configured ? "Replace key" : "Save key"}
                        </button>
                        {#if !remoteCredential?.configured}
                          <button type="button" class="quiet" onclick={onAiRemoteImport}>
                            Import environment key
                          </button>
                        {/if}
                      </div>
                      <p class="credential-hint">
                        The key is stored in the OS keychain and never shown again. Alternatively, launch with
                        DAENA_REMOTE_API_KEY set and use the environment import.
                      </p>
                    </div>
                  {/if}
                </section>
              </div>
            </div>
          {/if}
          <ImageProviderSettingsCard
            settings={aiSettings.imageProvider}
            onChange={(key, value) => onAiImageSettingsChange(key, value)} />
        </div>
      {/if}
    </div>
  </div>
</section>

<style>
.settings-view {
  display: flex;
  flex-direction: column;
  min-height: calc(100vh - 58px);
  padding: 28px 32px 40px;
  background: var(--canvas);
}
.settings-header {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 16px;
  margin-bottom: 22px;
  padding: 16px 16px 14px;
  border: 1px solid var(--line);
  border-radius: 14px;
  background: var(--surface);
}
.header-left {
  display: flex;
  gap: 14px;
  align-items: flex-start;
}
.header-icon {
  width: 40px;
  height: 40px;
  display: grid;
  place-items: center;
  border-radius: 11px;
  background: var(--accent-dark);
  color: var(--on-accent);
  flex: 0 0 40px;
}
.settings-header h1 {
  margin: 2px 0 6px;
  font: 600 22px/1.1 var(--font-display, Georgia, serif);
  color: var(--ink);
  letter-spacing: -0.01em;
}
.settings-header p {
  margin: 0;
  max-width: 520px;
  color: var(--ink-soft);
  font:
    400 12.5px/1.5 Inter,
    ui-sans-serif,
    system-ui,
    sans-serif;
}
.panel-kicker {
  color: var(--accent);
  font:
    700 10px/1 Inter,
    ui-sans-serif,
    system-ui,
    sans-serif;
  letter-spacing: 0.08em;
  text-transform: uppercase;
}
.header-back {
  flex: 0 0 auto;
  display: inline-flex;
  align-items: center;
  gap: 6px;
  border-radius: 999px;
}
.settings-layout {
  display: grid;
  grid-template-columns: 200px minmax(0, 1fr);
  gap: 22px;
  align-items: start;
}
.settings-nav {
  display: grid;
  gap: 6px;
  padding: 6px;
  border: 1px solid var(--line);
  border-radius: 12px;
  background: var(--surface-subtle);
  position: sticky;
  top: 16px;
}
.settings-nav-button {
  width: 100%;
  display: inline-flex;
  align-items: center;
  gap: 8px;
  padding: 10px 12px;
  border: 1px solid transparent;
  border-radius: 9px;
  background: transparent;
  color: var(--ink-soft);
  text-align: left;
  cursor: pointer;
  font:
    600 13px Inter,
    ui-sans-serif,
    system-ui,
    sans-serif;
  transition: all 0.14s ease;
}
.settings-nav-button:hover {
  background: var(--theme-warning-bg, #efe8d9);
  color: var(--ink);
}
.settings-nav-button.active {
  background: var(--accent-dark);
  border-color: var(--accent-dark);
  color: var(--on-accent);
  box-shadow: 0 1px 0 rgba(48, 44, 38, 0.12);
}
.settings-panel {
  min-width: 0;
  display: grid;
  gap: 18px;
  padding: 22px 24px;
  border: 1px solid var(--line);
  border-radius: 14px;
  background: var(--surface);
  box-shadow:
    0 1px 0 rgba(40 40 20 / 4%),
    0 8px 24px rgba(48, 44, 38, 0.04);
}
.panel-hero {
  display: grid;
  grid-template-columns: 40px 1fr;
  gap: 14px;
  padding: 16px 16px 14px;
  border: 1px solid var(--line);
  border-radius: 14px;
  background: var(--surface);
}
.panel-hero .hero-icon {
  width: 40px;
  height: 40px;
  display: grid;
  place-items: center;
  border-radius: 11px;
  background: var(--accent-dark);
  color: var(--on-accent);
}
.hero-copy .kicker {
  color: var(--accent);
  font:
    700 10px/1 Inter,
    ui-sans-serif,
    system-ui,
    sans-serif;
  letter-spacing: 0.08em;
  text-transform: uppercase;
}
.hero-copy strong {
  display: block;
  margin-top: 3px;
  color: var(--ink);
  font: 600 16px/1.15 var(--font-display, Georgia, serif);
}
.hero-copy p {
  margin: 6px 0 0;
  max-width: 640px;
  color: var(--ink-soft);
  font:
    400 12.5px/1.5 Inter,
    ui-sans-serif,
    system-ui,
    sans-serif;
}
.hero-stats {
  grid-column: 2;
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
  margin-top: 2px;
}
.stat-pill {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  padding: 5px 9px;
  border-radius: 999px;
  background: var(--surface-warm);
  border: 1px solid var(--line-soft);
  color: var(--ink-muted);
  font:
    600 11px Inter,
    ui-sans-serif,
    system-ui,
    sans-serif;
}
.block {
  display: grid;
  gap: 14px;
  padding: 18px;
  border: 1px solid var(--line);
  border-radius: 14px;
  background: var(--surface);
}
.block.elevated {
  box-shadow:
    0 1px 0 rgba(48, 44, 38, 0.03),
    0 8px 24px rgba(48, 44, 38, 0.04);
}
.block-heading {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  justify-content: space-between;
  gap: 10px 16px;
  padding-bottom: 12px;
  border-bottom: 1px solid var(--theme-warning-border, #f0e8d9);
}
.heading-left {
  display: inline-flex;
  align-items: center;
  gap: 10px;
}
.heading-icon {
  width: 28px;
  height: 28px;
  display: grid;
  place-items: center;
  border-radius: 8px;
  background: var(--surface-warm);
  border: 1px solid var(--line-soft);
  color: var(--ink-muted);
}
.heading-icon.accent {
  background: var(--accent-dark);
  border-color: var(--accent-dark);
  color: var(--on-accent);
}
.heading-left h4 {
  margin: 0;
  font:
    600 13px Inter,
    ui-sans-serif,
    system-ui,
    sans-serif;
  color: var(--ink);
  letter-spacing: -0.01em;
}
.count-badge {
  display: inline-grid;
  place-items: center;
  min-width: 22px;
  height: 20px;
  padding: 0 7px;
  border-radius: 999px;
  background: var(--surface-warm);
  border: 1px solid var(--line-soft);
  color: var(--ink-muted);
  font:
    700 11px Inter,
    sans-serif;
}

.block-hint {
  display: inline-flex;
  align-items: center;
  gap: 5px;
  color: var(--ink-faint);
  font:
    500 11.5px Inter,
    ui-sans-serif,
    system-ui,
    sans-serif;
}
.theme-options {
  display: grid;
  grid-template-columns: repeat(3, minmax(0, 1fr));
  gap: 10px;
}
.theme-options button {
  min-width: 0;
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 12px;
  border: 1px solid var(--line-strong);
  border-radius: 10px;
  background: var(--surface-subtle);
  color: var(--ink-soft);
  text-align: left;
  cursor: pointer;
  transition:
    border-color 0.14s ease,
    background 0.14s ease,
    color 0.14s ease,
    box-shadow 0.14s ease;
}
.theme-options button:hover {
  border-color: var(--accent-soft);
  background: var(--surface-warm);
  color: var(--ink);
}
.theme-options button.active {
  border-color: var(--accent);
  background: var(--accent-bg);
  color: var(--ink);
  box-shadow: inset 0 0 0 1px color-mix(in srgb, var(--accent) 24%, transparent);
}
.theme-options button span,
.theme-options button strong,
.theme-options button small {
  display: block;
}
.theme-options button strong {
  color: inherit;
  font-size: 12px;
}
.theme-options button small {
  margin-top: 2px;
  color: var(--ink-faint);
  font-size: 10.5px;
}
.empty-inline {
  display: flex;
  gap: 12px;
  align-items: flex-start;
  padding: 14px 14px;
  border: 1px dashed var(--line-strong);
  border-radius: 11px;
  background: var(--surface-quiet);
  color: var(--ink-muted);
}
.empty-inline strong {
  display: block;
  color: var(--ink);
  font:
    600 13px Inter,
    sans-serif;
  margin-bottom: 3px;
}
.empty-inline span {
  font:
    400 12px/1.5 Inter,
    sans-serif;
}

.settings-recent-list {
  list-style: none;
  margin: 0;
  padding: 0;
  display: grid;
  gap: 8px;
}
.settings-recent-list li {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  padding: 12px 14px;
  border: 1px solid var(--theme-warning-border, #ebe3d6);
  border-radius: 12px;
  background: var(--surface-quiet);
  transition:
    border-color 0.14s ease,
    box-shadow 0.14s ease;
}
.settings-recent-list li:hover {
  border-color: var(--theme-warning-border, #e0d6c4);
  box-shadow: 0 4px 14px rgba(48, 44, 38, 0.05);
}
.recent-copy strong,
.recent-copy small {
  display: block;
}
.recent-copy small {
  margin-top: 3px;
  color: var(--ink-soft);
  font:
    500 11px ui-monospace,
    SFMono-Regular,
    Menlo,
    monospace;
}
.ai-settings-form {
  display: grid;
  gap: 18px;
  width: 100%;
}
.ai-settings-card {
  display: grid;
  gap: 16px;
  padding: 18px;
  border: 1px solid var(--line);
  border-radius: 14px;
  background: var(--surface);
}
.ai-settings-card.elevated {
  box-shadow:
    0 1px 0 rgba(48, 44, 38, 0.03),
    0 8px 24px rgba(48, 44, 38, 0.04);
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
  backdrop-filter: blur(4px);
}
.ai-provider-modal {
  width: min(820px, 100%);
  max-height: min(820px, calc(100vh - 48px));
  overflow: auto;
  padding: 22px;
  border: 1px solid var(--theme-warning-border, #d8cdbd);
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
  border: 1px solid var(--theme-warning-border, #d8cdbd);
  border-radius: 11px;
  background: var(--surface);
  color: var(--ink);
  text-align: left;
  cursor: pointer;
  transition: all 0.14s ease;
}
.ai-provider-preset:hover,
.ai-provider-preset.active {
  border-color: var(--accent-dark);
  background: var(--accent-dark);
  color: var(--on-accent);
  transform: translateY(-1px);
  box-shadow: 0 4px 12px rgba(48, 44, 38, 0.12);
}
.ai-provider-preset strong {
  margin: 0;
  color: inherit;
  font-size: 12px;
}
.ai-provider-preset span {
  color: var(--ink-soft);
  font-size: 10px;
  line-height: 1.35;
}
.ai-provider-preset:hover span,
.ai-provider-preset.active span {
  color: rgba(255, 255, 255, 0.7);
}
.ai-label-note {
  display: inline-block;
  padding: 2px 5px;
  border: 1px solid var(--theme-warning-border, #d8c6ad);
  border-radius: 999px;
  background: var(--theme-warning-bg, #fbf2e5);
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
  padding: 12px;
  border: 1px solid var(--theme-warning-border, #f0e8d9);
  border-radius: 11px;
  background: var(--surface-subtle);
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
  border: 1px solid var(--theme-warning-border, #e2d8c9);
  border-radius: 9px;
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
  margin: 6px 0 0;
  color: var(--ink-soft);
  font-size: 11px;
  line-height: 1.5;
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
  border-radius: 999px;
  background: var(--theme-success-bg, #e8f3e8);
  color: var(--theme-success-text, #407047);
  font-size: 10px;
  font-weight: 700;
  letter-spacing: 0;
  text-transform: none;
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
  border: 1px solid var(--theme-warning-border, #d8cdbd);
  border-radius: 9px;
  outline: 0;
  background: var(--surface);
  color: var(--ink);
  font: 400 12px var(--font-body, Inter, sans-serif);
  transition:
    border-color 0.15s ease,
    box-shadow 0.15s ease;
}
.ai-settings-form input:focus {
  border-color: var(--accent);
  box-shadow: 0 0 0 3px rgb(188 141 93 / 12%);
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
  border: 1px solid var(--theme-warning-border, #d8cdbd);
  border-radius: 9px;
  background: var(--surface);
  box-shadow: 0 10px 24px rgb(48 44 38 / 14%);
}
.ai-model-suggestions button {
  padding: 7px 8px;
  border: 0;
  border-radius: 6px;
  background: transparent;
  color: var(--ink);
  font: 11px var(--font-body, Inter, sans-serif);
  text-align: left;
  cursor: pointer;
}
.ai-model-suggestions button:hover {
  background: var(--theme-warning-bg, #efe6d6);
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
.credential-row {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
  align-items: center;
}
.credential-row input {
  flex: 1 1 200px;
  min-width: 0;
}
.credential-hint {
  margin: 0;
  color: var(--ink-soft);
  font-size: 10.5px;
  line-height: 1.5;
}
.ai-action-label {
  color: var(--ink-soft);
  font-size: 9px;
  font-weight: 800;
  letter-spacing: 0.1em;
  text-transform: uppercase;
}
.ai-status-badge {
  display: inline-flex;
  align-items: center;
  min-height: 30px;
  padding: 0 9px;
  border: 1px solid var(--theme-warning-border, #e2d8c9);
  border-radius: 999px;
  background: var(--surface-subtle);
  color: var(--theme-danger-text, #9d5b42);
  font-size: 10px;
  white-space: nowrap;
}
.ai-status-badge.ok {
  border-color: var(--success-line);
  background: var(--success-bg);
  color: var(--success);
}
.ai-card-actions {
  padding-top: 2px;
}
.primary {
  display: inline-flex;
  align-items: center;
  gap: 7px;
  padding: 9px 14px;
  border: 1px solid var(--accent-dark);
  border-radius: 9px;
  background: var(--accent-dark);
  color: var(--on-accent);
  font:
    700 12px Inter,
    sans-serif;
  cursor: pointer;
  box-shadow: 0 1px 0 rgba(48, 44, 38, 0.12);
  transition: all 0.14s ease;
}
.primary:hover {
  background: #4a6b57;
  transform: translateY(-1px);
  box-shadow: 0 4px 12px rgba(48, 44, 38, 0.12);
}
.primary:active {
  transform: translateY(0);
  box-shadow: none;
}
.primary:disabled {
  opacity: 0.45;
  cursor: not-allowed;
  transform: none;
  box-shadow: none;
}
.quiet {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  padding: 8px 12px;
  border: 1px solid var(--line-strong);
  border-radius: 9px;
  background: var(--surface);
  color: var(--ink-muted);
  font:
    600 11.5px Inter,
    sans-serif;
  cursor: pointer;
  transition: all 0.14s ease;
}
.quiet:hover {
  border-color: var(--theme-warning-border, #b7a88f);
  background: var(--surface-warm);
  transform: translateY(-1px);
  box-shadow: 0 4px 12px rgba(48, 44, 38, 0.06);
}
.quiet:active {
  transform: translateY(0);
  box-shadow: none;
}
.quiet.icon {
  width: 32px;
  height: 32px;
  padding: 0;
  display: grid;
  place-items: center;
  border-radius: 9px;
}
.quiet:disabled {
  opacity: 0.45;
  cursor: not-allowed;
  transform: none;
  box-shadow: none;
}
.quiet-button {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  padding: 8px 12px;
  border: 1px solid var(--line-strong);
  border-radius: 9px;
  background: var(--surface);
  color: var(--ink-muted);
  font:
    600 11.5px Inter,
    sans-serif;
  cursor: pointer;
  transition: all 0.14s ease;
}
.quiet-button:hover {
  background: var(--surface-warm);
  border-color: var(--theme-warning-border, #b7a88f);
}
.ai-status {
  color: var(--theme-danger-text, #9d5b42);
  font-size: 11px;
  line-height: 1.4;
}
.ai-status.ok {
  color: var(--success);
  font-weight: 700;
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
    position: static;
  }
  .settings-panel {
    padding: 16px;
  }
  .theme-options {
    grid-template-columns: 1fr;
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
  .panel-hero {
    grid-template-columns: 1fr;
  }
  .hero-stats {
    grid-column: 1;
  }
}
</style>
