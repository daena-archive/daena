<script lang="ts">
import type { Snippet } from "svelte";
import { onMount } from "svelte";
import { setSchemaEditorDiscardPrompt } from "$lib/schemaEditorGuard";

type SettingsSection = "general" | "ai" | "plugins" | "schema" | "git";
type RecentProject = { name: string; root: string };

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
      dataBoundary: "local" | "remote";
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
    key: "id" | "name" | "adapter" | "endpoint" | "model" | "embeddingModel" | "capabilities" | "dataBoundary",
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
        onclick={() => void goToSection("git")}>Git</button>
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
              <span class="ai-card-badge">{aiSettings.provider.dataBoundary === "remote" ? "Remote" : "Local"}</span>
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
                  <div class="ai-field-grid">
                    <label
                      >Provider name<input
                        value={aiSettings.provider.name}
                        oninput={(event) =>
                          onAiSettingsChange("name", (event.currentTarget as HTMLInputElement).value)} /></label>
                    <label
                      >Provider ID<input
                        value={aiSettings.provider.id}
                        oninput={(event) =>
                          onAiSettingsChange("id", (event.currentTarget as HTMLInputElement).value)} /></label>
                    <label class="ai-field-wide"
                      >Endpoint<input
                        value={aiSettings.provider.endpoint}
                        oninput={(event) =>
                          onAiSettingsChange("endpoint", (event.currentTarget as HTMLInputElement).value)} /></label>
                    <label
                      >Model ID<input
                        value={aiSettings.provider.model}
                        oninput={(event) =>
                          onAiSettingsChange("model", (event.currentTarget as HTMLInputElement).value)} /></label>
                    <label
                      >Embedding model<input
                        value={aiSettings.provider.embeddingModel}
                        oninput={(event) =>
                          onAiSettingsChange(
                            "embeddingModel",
                            (event.currentTarget as HTMLInputElement).value,
                          )} /></label>
                    <label class="ai-field-wide"
                      >Model capabilities<input
                        value={aiSettings.provider.capabilities.join(", ")}
                        placeholder="text.generate, text.generate.structured, text.embed"
                        oninput={(event) =>
                          onAiSettingsChange(
                            "capabilities",
                            (event.currentTarget as HTMLInputElement).value,
                          )} /></label>
                    <label
                      >Data boundary<select
                        value={aiSettings.provider.dataBoundary}
                        onchange={(event) =>
                          onAiSettingsChange(
                            "dataBoundary",
                            (event.currentTarget as HTMLSelectElement).value as "local" | "remote",
                          )}>
                        <option value="local">Local</option><option value="remote">Remote</option></select
                      ></label>
                  </div>
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
                              : "Server found; model is missing"
                            : (aiStatus.error ?? "Provider unavailable")}</span
                      >{/if}
                    {#if aiModelsMessage}<span class="ai-status">{aiModelsMessage}</span>{/if}
                  </div>
                  {#if aiSettings.provider.dataBoundary === "remote"}
                    <div class="ai-settings-actions ai-card-actions">
                      <span class="ai-status"
                        >{remoteCredential?.configured
                          ? "Credential stored in OS keychain"
                          : "No OS credential configured"}</span>
                      {#if !remoteCredential?.configured}<button
                          type="button"
                          class="quiet-button"
                          onclick={onAiRemoteImport}>Import environment key</button
                        >{/if}
                      {#if projectOpen}
                        <button type="button" class="primary-button" onclick={() => onAiRemoteConsent(true)}
                          >Allow for this project</button>
                        <button type="button" class="quiet-button" onclick={() => onAiRemoteConsent(false)}
                          >Revoke</button>
                      {/if}
                    </div>
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
  max-width: 760px;
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
.ai-settings-form input,
.ai-settings-form select {
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
.ai-settings-form input:focus,
.ai-settings-form select:focus {
  border-color: #bc8d5d;
  box-shadow: 0 0 0 3px rgb(188 141 93 / 12%);
}
.ai-settings-form input:disabled,
.ai-settings-form select:disabled {
  border-color: #e1dcd3;
  background: #f2efe9;
  color: var(--ink-faint);
  cursor: not-allowed;
  opacity: 0.7;
}
.ai-settings-form input:disabled:hover,
.ai-settings-form select:disabled:hover {
  border-color: #e1dcd3;
  box-shadow: none;
}
.ai-settings-actions {
  display: flex;
  align-items: center;
  gap: 8px;
  flex-wrap: wrap;
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
