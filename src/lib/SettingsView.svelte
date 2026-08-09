<script lang="ts">
  import type { Snippet } from "svelte";

  type SettingsSection = "general" | "ai" | "plugins" | "git";
  type RecentProject = { name: string; root: string };

  let {
    section = $bindable("general" as SettingsSection),
    recentProjects,
    projectOpen,
    onRemoveRecent,
    onClose,
    plugins,
    git,
    aiSettings,
    aiStatus,
    aiModels,
    aiModelsBusy,
    aiModelsMessage,
    aiIndexStatus,
    aiIndexBusy,
    aiIndexMessage,
    onAiRemoteSettingsChange,
    onAiRemotePolicyChange,
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
    plugins: Snippet;
    git: Snippet;
    aiSettings: { localEndpoint: string; localModel: string; localEmbeddingModel: string; remotePolicy: "disabled" | "localOnly" | "ask" | "approvedPairs" | "remoteAllowed"; remote: { provider: string; endpoint: string; model: string; consents: Array<{ projectId: string; provider: string; endpoint: string }> } };
    aiStatus: { available: boolean; modelAvailable: boolean; error: string | null } | null;
    aiModels: string[];
    aiModelsBusy: boolean;
    aiModelsMessage: string;
    aiIndexStatus: { available: boolean; state: string | null } | null;
    aiIndexBusy: boolean;
    aiIndexMessage: string;
    onAiSettingsChange: (key: "localEndpoint" | "localModel" | "localEmbeddingModel", value: string) => void;
    onAiCheck: () => void;
    onAiModelsLoad: () => void;
    onAiIndexRefresh: () => void;
    onAiIndexRebuild: () => void;
    onAiIndexCancel: () => void;
    onAiRemoteSettingsChange: (key: "provider" | "endpoint" | "model", value: string) => void;
    onAiRemotePolicyChange: (value: "disabled" | "localOnly" | "ask" | "approvedPairs" | "remoteAllowed") => void;
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

  $effect(() => {
    if (aiSettings.localEmbeddingModel.trim()) embeddingSectionOpen = true;
  });

  function chooseModel(kind: "chat" | "embedding", value: string) {
    onAiSettingsChange(kind === "chat" ? "localModel" : "localEmbeddingModel", value);
    modelPickerOpen = null;
  }

  function closeModelPickerSoon() {
    window.setTimeout(() => { modelPickerOpen = null; }, 120);
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
    if (!recoveryPath.trim() || !window.confirm("Restore this recovery backup and replace the current runtime state?")) return;
    storageBusy = true;
    try {
      await onRestoreRecoveryBackup(recoveryPath.trim());
      storageMessage = "Recovery backup restored.";
    } finally {
      storageBusy = false;
    }
  }
</script>

<section class="settings-view" aria-label="Settings">
  <header class="settings-header">
    <div>
      <span class="panel-kicker">APPLICATION</span>
      <h1>Settings</h1>
      <p>App preferences and the plugins that power this project.</p>
    </div>
    <button type="button" class="quiet-button" onclick={onClose}>Back</button>
  </header>

  <div class="settings-layout">
    <nav class="settings-nav" aria-label="Settings sections">
      <button
        type="button"
        class:active={section === "general"}
        class="settings-nav-button"
        onclick={() => (section = "general")}
      >General</button>
      <button type="button" class:active={section === "ai"} class="settings-nav-button" onclick={() => (section = "ai")}>AI</button>
      <button
        type="button"
        class:active={section === "plugins"}
        class="settings-nav-button"
        onclick={() => (section = "plugins")}
      >Plugins</button>
      <button
        type="button"
        class:active={section === "git"}
        class="settings-nav-button"
        onclick={() => (section = "git")}
      >Git</button>
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
                <button
                  type="button"
                  class="quiet-button"
                  onclick={() => onRemoveRecent(recent.root)}
                >Remove</button>
              </li>
            {/each}
          </ul>
        {/if}
        {#if projectOpen}
          <div class="settings-section-heading">
            <strong>Project storage</strong>
            <p>Portable backups contain canonical files. Recovery backups retain the runtime queue and staged payloads.</p>
          </div>
          <div class="settings-actions">
            <button type="button" class="primary-button" disabled={storageBusy} onclick={() => void createPortableBackup()}>Portable backup</button>
            <button type="button" class="quiet-button" disabled={storageBusy} onclick={() => void createRecoveryBackup()}>Recovery backup</button>
          </div>
          <label class="settings-path-field">Recovery backup path<input bind:value={recoveryPath} placeholder="Paste a recovery backup directory" /></label>
          <button type="button" class="quiet-button" disabled={storageBusy || !recoveryPath.trim()} onclick={() => void restoreRecoveryBackup()}>Restore recovery backup</button>
          {#if storageMessage}<p class="settings-note" role="status">{storageMessage}</p>{/if}
        {/if}
      {:else if section === "ai"}
        <div class="settings-section-heading">
          <strong>AI providers</strong>
          <p>Configure where generation runs. Local providers stay on this machine; remote access is separately gated.</p>
        </div>
        <div class="ai-settings-form">
          <section class="ai-settings-card ai-overview-card" aria-labelledby="ai-overview-heading">
            <div class="ai-card-heading">
              <div>
                <span class="ai-card-kicker">GENERATION ROUTING</span>
                <strong id="ai-overview-heading">Local by default</strong>
                <p>Rewrites use the configured local provider unless you explicitly choose the approved remote provider for a request.</p>
              </div>
              <span class="ai-card-badge">Local first</span>
            </div>
            <div class="ai-overview-details">
              <div><span>Local provider</span><strong>OpenAI-compatible local server</strong><small>{aiSettings.localModel || "No model selected"}</small></div>
              <div><span>Remote provider</span><strong>{aiSettings.remote.provider || "Not configured"}</strong><small>{aiSettings.remotePolicy === "localOnly" || aiSettings.remotePolicy === "disabled" ? "Unavailable for requests" : "Available by request"}</small></div>
            </div>
            <button type="button" class="primary-button" onclick={() => (providerModalOpen = true)}>Configure providers</button>
          </section>
          {#if providerModalOpen}
            <div class="ai-modal-backdrop">
              <div class="ai-provider-modal" role="dialog" aria-modal="true" aria-labelledby="providers-heading">
                <div class="ai-modal-heading">
                  <div>
                    <span class="ai-card-kicker">AI PROVIDERS</span>
                    <strong id="providers-heading">Provider configuration</strong>
                    <p>Configure local and remote providers. Generation still defaults to local unless remote is chosen for a request.</p>
                  </div>
                  <button type="button" class="quiet-button" aria-label="Close provider configuration" onclick={() => (providerModalOpen = false)}>Close</button>
                </div>
                <section class="ai-settings-card ai-providers-card">
            <div class="ai-provider-grid">
              <div class="ai-provider-section" aria-labelledby="local-ai-heading">
                <div class="ai-card-heading">
                  <div>
                    <span class="ai-card-kicker">LOCAL</span>
                    <strong id="local-ai-heading">Local provider</strong>
                    <p>Private by default. Daena connects only to the configured loopback endpoint.</p>
                  </div>
                  <span class="ai-card-badge">On device</span>
                </div>
                <div class="ai-field-grid">
                  <label>Endpoint<input value={aiSettings.localEndpoint} oninput={(event) => onAiSettingsChange("localEndpoint", (event.currentTarget as HTMLInputElement).value)} /></label>
                  <label>Chat model ID<div class="ai-model-picker-control"><input value={aiSettings.localModel} placeholder="Enter or choose a model ID" aria-haspopup="listbox" aria-expanded={modelPickerOpen === "chat"} onfocus={() => (modelPickerOpen = "chat")} onclick={() => (modelPickerOpen = "chat")} onblur={closeModelPickerSoon} oninput={(event) => onAiSettingsChange("localModel", (event.currentTarget as HTMLInputElement).value)} />{#if modelPickerOpen === "chat" && aiModels.length > 0}<div class="ai-model-picker-menu" role="listbox" aria-label="Available chat models">{#each aiModels as model}<button type="button" role="option" aria-selected={model === aiSettings.localModel} onmousedown={(event) => event.preventDefault()} onclick={() => chooseModel("chat", model)}>{model}</button>{/each}</div>{/if}</div></label>
                </div>
                <div class="ai-settings-actions ai-card-actions"><button type="button" class="primary-button" onclick={onAiModelsLoad} disabled={aiModelsBusy}>{aiModelsBusy ? "Loading models…" : "Load available models"}</button><button type="button" class="quiet-button" onclick={onAiCheck}>Test connection</button>{#if aiStatus}<span class:ok={aiStatus.available && aiStatus.modelAvailable} class="ai-status">{aiStatus.available ? aiStatus.modelAvailable ? "Ready" : "Server found; model is missing" : aiStatus.error ?? "Local provider unavailable"}</span>{/if}{#if aiModelsMessage}<span class="ai-status">{aiModelsMessage}</span>{/if}</div>
                <details class="ai-provider-advanced" bind:open={embeddingSectionOpen}>
                  <summary>Use a different embedding model{#if aiSettings.localEmbeddingModel.trim()}<span class="ai-provider-advanced-indicator">Configured</span>{/if}</summary>
                  <div class="ai-provider-advanced-body">
                    <label>Embedding model ID<div class="ai-model-picker-control"><input value={aiSettings.localEmbeddingModel} placeholder="Leave blank to use the chat model" aria-haspopup="listbox" aria-expanded={modelPickerOpen === "embedding"} onfocus={() => (modelPickerOpen = "embedding")} onclick={() => (modelPickerOpen = "embedding")} onblur={closeModelPickerSoon} oninput={(event) => onAiSettingsChange("localEmbeddingModel", (event.currentTarget as HTMLInputElement).value)} />{#if modelPickerOpen === "embedding" && aiModels.length > 0}<div class="ai-model-picker-menu" role="listbox" aria-label="Available embedding models">{#each aiModels as model}<button type="button" role="option" aria-selected={model === aiSettings.localEmbeddingModel} onmousedown={(event) => event.preventDefault()} onclick={() => chooseModel("embedding", model)}>{model}</button>{/each}</div>{/if}</div></label>
                    <p>Semantic indexing uses this model when provided; otherwise it reuses the chat model above.</p>
                  </div>
                </details>
              </div>
              <div class:remote-disabled={aiSettings.remotePolicy === "disabled" || aiSettings.remotePolicy === "localOnly"} class="ai-provider-section ai-remote-section" aria-labelledby="remote-ai-heading">
                <div class="ai-card-heading">
                  <div>
                    <span class="ai-card-kicker">REMOTE</span>
                    <strong id="remote-ai-heading">External AI</strong>
                    <p>Off by default. Credentials stay in OS storage, and project consent is exact to the provider and endpoint.</p>
                  </div>
                  <span class="ai-card-badge ai-card-badge-muted">Consent required</span>
                </div>
                <div class="ai-remote-controls">
                  <label class:enabled={aiSettings.remotePolicy !== "disabled" && aiSettings.remotePolicy !== "localOnly"} class="ai-enable-remote">
                    <span class="ai-toggle-copy"><strong>Enable remote AI</strong><small>{aiSettings.remotePolicy !== "disabled" && aiSettings.remotePolicy !== "localOnly" ? "Available for approved requests" : "Remote calls are blocked"}</small></span>
                    <span class="ai-toggle-state">{aiSettings.remotePolicy !== "disabled" && aiSettings.remotePolicy !== "localOnly" ? "Enabled" : "Off"}</span>
                    <input class="ai-toggle-input" type="checkbox" checked={aiSettings.remotePolicy !== "disabled" && aiSettings.remotePolicy !== "localOnly"} onchange={(event) => onAiRemotePolicyChange((event.currentTarget as HTMLInputElement).checked ? "ask" : "disabled")} />
                  </label>
                  <label class="ai-remote-access-field">Remote access<select disabled={aiSettings.remotePolicy === "disabled" || aiSettings.remotePolicy === "localOnly"} value={aiSettings.remotePolicy === "disabled" || aiSettings.remotePolicy === "localOnly" ? "ask" : aiSettings.remotePolicy} onchange={(event) => onAiRemotePolicyChange((event.currentTarget as HTMLSelectElement).value as typeof aiSettings.remotePolicy)}><option value="ask">Ask for project approval</option><option value="approvedPairs">Allow approved project pairs</option><option value="remoteAllowed">Allow remote generation</option></select></label>
                </div>
                <div class="ai-field-grid">
                  <label>Provider ID<input disabled={aiSettings.remotePolicy === "disabled" || aiSettings.remotePolicy === "localOnly"} value={aiSettings.remote.provider} oninput={(event) => onAiRemoteSettingsChange("provider", (event.currentTarget as HTMLInputElement).value)} placeholder="openai-compatible" /></label>
                  <label class="ai-field-wide">HTTPS endpoint<input disabled={aiSettings.remotePolicy === "disabled" || aiSettings.remotePolicy === "localOnly"} value={aiSettings.remote.endpoint} oninput={(event) => onAiRemoteSettingsChange("endpoint", (event.currentTarget as HTMLInputElement).value)} placeholder="https://api.example.com/v1" /></label>
                  <label>Remote model ID<input disabled={aiSettings.remotePolicy === "disabled" || aiSettings.remotePolicy === "localOnly"} value={aiSettings.remote.model} oninput={(event) => onAiRemoteSettingsChange("model", (event.currentTarget as HTMLInputElement).value)} /></label>
                </div>
                <div class="ai-settings-actions ai-card-actions">
                  <span class="ai-status">{remoteCredential?.configured ? "Credential stored in OS keychain" : "No OS credential configured"}</span>
                  <button type="button" class="quiet-button" disabled={aiSettings.remotePolicy === "disabled" || aiSettings.remotePolicy === "localOnly"} onclick={onAiRemoteImport}>Import environment key</button>
                  {#if projectOpen}<button type="button" class="primary-button" disabled={aiSettings.remotePolicy === "disabled" || aiSettings.remotePolicy === "localOnly"} onclick={() => onAiRemoteConsent(true)}>Allow for this project</button><button type="button" class="quiet-button" disabled={aiSettings.remotePolicy === "disabled" || aiSettings.remotePolicy === "localOnly"} onclick={() => onAiRemoteConsent(false)}>Revoke</button>{/if}
                </div>
              </div>
            </div>
                </section>
              </div>
            </div>
          {/if}
          <section class="ai-settings-card" aria-labelledby="retrieval-index-heading">
            <div class="ai-card-heading ai-card-heading-compact">
              <div>
                <span class="ai-card-kicker">PROJECT CONTEXT</span>
                <strong id="retrieval-index-heading">Semantic retrieval</strong>
                <p>Embeddings are disposable and local. Lexical retrieval remains available while this index is absent or rebuilding.</p>
              </div>
              <span class="ai-index-state">{aiIndexStatus?.available ? aiIndexStatus.state ?? "unknown" : "not attached"}</span>
            </div>
            <div class="ai-settings-actions ai-card-actions">
              <button type="button" class="quiet-button" onclick={onAiIndexRefresh}>Refresh status</button>
              {#if aiIndexBusy}<button type="button" class="quiet-button" onclick={onAiIndexCancel}>Cancel rebuild</button>{/if}
              <button type="button" class="primary-button" onclick={onAiIndexRebuild} disabled={aiIndexBusy}>{aiIndexBusy ? "Rebuilding…" : "Build semantic index"}</button>
            </div>
            {#if aiIndexMessage}<p class="ai-feedback">{aiIndexMessage}</p>{/if}
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
      {:else}
        {@render git()}
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
  .ai-settings-form { display: grid; gap: 14px; max-width: 760px; }
  .ai-settings-card { display: grid; gap: 18px; padding: 18px; border: 1px solid var(--line); border-radius: 12px; background: #fdfbf7; }
  .ai-overview-card { gap: 16px; }
  .ai-overview-details { display: grid; grid-template-columns: repeat(2, minmax(0, 1fr)); gap: 10px; }
  .ai-overview-details > div { display: grid; gap: 3px; padding: 11px 12px; border: 1px solid #e2dbd0; border-radius: 8px; background: var(--surface); }
  .ai-overview-details span { color: var(--ink-faint); font-size: 9px; font-weight: 800; letter-spacing: .08em; text-transform: uppercase; }
  .ai-overview-details strong { color: var(--ink); font-size: 12px; }
  .ai-overview-details small { overflow: hidden; color: var(--ink-soft); font-size: 10px; text-overflow: ellipsis; white-space: nowrap; }
  .ai-modal-backdrop { position: fixed; inset: 0; z-index: 40; display: grid; place-items: center; padding: 24px; background: rgba(37,37,31,.34); }
  .ai-provider-modal { width: min(820px, 100%); max-height: min(820px, calc(100vh - 48px)); overflow: auto; padding: 22px; border: 1px solid #d8cdbd; border-radius: 14px; background: var(--surface); box-shadow: 0 24px 80px rgba(37,37,31,.24); }
  .ai-provider-modal > .ai-settings-card { padding: 0; border: 0; background: transparent; box-shadow: none; }
  .ai-modal-heading { display: flex; align-items: flex-start; justify-content: space-between; gap: 18px; margin-bottom: 18px; }
  .ai-modal-heading strong { display: block; margin-top: 5px; color: var(--ink); font-size: 18px; }
  .ai-modal-heading p { max-width: 580px; margin: 6px 0 0; color: var(--ink-soft); font-size: 11px; line-height: 1.5; }
  .ai-providers-card { gap: 20px; }
  .ai-provider-grid { display: grid; gap: 20px; }
  .ai-provider-section { display: grid; align-content: start; gap: 16px; }
  .ai-provider-section + .ai-provider-section { padding-top: 20px; border-top: 1px solid var(--line); }
  .ai-remote-section { background: linear-gradient(90deg, rgba(247,243,235,.45), transparent 30%); }
  .ai-remote-section.remote-disabled .ai-field-grid, .ai-remote-section.remote-disabled .ai-card-actions { opacity: .58; }
  .ai-remote-controls { display: grid; grid-template-columns: minmax(0, 1.25fr) minmax(220px, .75fr); gap: 12px; align-items: end; }
  .ai-remote-access-field { height: 77px; box-sizing: border-box; padding: 10px 12px; border: 1px solid #d8cdbd; border-radius: 9px; background: var(--surface); }
  .ai-remote-access-field select { margin-top: 0; }
  .ai-card-heading { display: flex; align-items: flex-start; justify-content: space-between; gap: 18px; }
  .ai-card-heading strong { display: block; margin-top: 5px; color: var(--ink); font-size: 16px; }
  .ai-card-heading p { max-width: 520px; margin: 6px 0 0; color: var(--ink-soft); font-size: 11px; line-height: 1.5; }
  .ai-remote-section .ai-card-heading > div { min-width: 0; flex: 1; }
  .ai-remote-section .ai-card-heading p { max-width: none; }
  .ai-card-heading-compact { align-items: center; }
  .ai-card-kicker { color: var(--accent); font-size: 9px; font-weight: 800; letter-spacing: .12em; }
  .ai-card-badge, .ai-index-state { flex: 0 0 auto; padding: 5px 8px; border: 1px solid #d8c6ad; border-radius: 999px; color: var(--accent-dark); font-size: 10px; font-weight: 700; white-space: nowrap; }
  .ai-card-badge-muted { border-color: #ded8cd; color: var(--ink-faint); }
  .ai-index-state { border-color: #c9d9cd; background: #f0f7f1; color: #557d63; text-transform: capitalize; }
  .ai-field-grid { display: grid; grid-template-columns: repeat(2, minmax(0, 1fr)); gap: 14px; }
  .ai-field-wide { grid-column: 1 / -1; }
  .ai-settings-form label { display: grid; gap: 6px; color: var(--ink-soft); font-size: 11px; font-weight: 700; }
  .ai-enable-remote { display: flex !important; align-items: center; gap: 12px; height: 77px; box-sizing: border-box; padding: 10px 12px; border: 1px solid #d8cdbd; border-radius: 9px; background: var(--surface); color: var(--ink); font-size: 12px !important; font-weight: 700 !important; cursor: pointer; transition: border-color .16s ease, background .16s ease, box-shadow .16s ease; }
  .ai-enable-remote:hover { border-color: #cbbda9; box-shadow: 0 3px 8px rgba(48,45,38,.07); }
  .ai-enable-remote.enabled { border-color: #b8cdbb; background: #f5faf5; }
  .ai-toggle-copy { display: grid; flex: 1; gap: 2px; }
  .ai-toggle-copy strong { color: var(--ink); font-size: 12px; }
  .ai-toggle-copy small { color: var(--ink-faint); font-size: 10px; font-weight: 400; }
  .ai-toggle-state { color: var(--ink-faint); font-size: 10px; font-weight: 800; letter-spacing: .06em; text-transform: uppercase; }
  .ai-enable-remote.enabled .ai-toggle-state { color: #557d63; }
  .ai-toggle-input { position: relative; width: 36px !important; height: 20px !important; margin: 0; padding: 0 !important; appearance: none; border: 0 !important; border-radius: 999px !important; background: #d2ccc1 !important; box-shadow: inset 0 0 0 1px rgba(48,45,38,.08) !important; cursor: pointer; transition: background .16s ease, box-shadow .16s ease; }
  .ai-toggle-input::after { position: absolute; top: 3px; left: 3px; width: 14px; height: 14px; border-radius: 50%; background: #fff; box-shadow: 0 1px 3px rgba(48,45,38,.2); content: ""; transition: transform .16s ease; }
  .ai-toggle-input:checked { background: #6f9d79 !important; box-shadow: inset 0 0 0 1px rgba(42,68,51,.18) !important; }
  .ai-toggle-input:checked::after { transform: translateX(16px); }
  .ai-settings-form input, .ai-settings-form select { min-width: 0; padding: 10px 11px; border: 1px solid #d8cdbd; border-radius: 8px; outline: 0; background: var(--surface); color: var(--ink); font: 12px var(--font-body); font-weight: 400; transition: border-color .15s ease, box-shadow .15s ease; }
  .ai-settings-form input:focus, .ai-settings-form select:focus { border-color: #bc8d5d; box-shadow: 0 0 0 3px rgb(188 141 93 / 12%); }
  .ai-settings-form input:disabled, .ai-settings-form select:disabled { border-color: #e1dcd3; background: #f2efe9; color: var(--ink-faint); cursor: not-allowed; opacity: .7; }
  .ai-settings-form input:disabled:hover, .ai-settings-form select:disabled:hover { border-color: #e1dcd3; box-shadow: none; }
  .ai-settings-actions { display: flex; align-items: center; gap: 8px; flex-wrap: wrap; }
  .ai-provider-advanced { border-top: 1px solid var(--line); padding-top: 14px; }
  .ai-provider-advanced summary { display: flex; align-items: center; gap: 8px; color: var(--ink-soft); cursor: pointer; font-size: 11px; font-weight: 700; }
  .ai-provider-advanced-indicator { padding: 3px 6px; border: 1px solid #b9d4bd; border-radius: 999px; background: #edf6ee; color: #557d63; font-size: 9px; font-weight: 800; letter-spacing: .04em; text-transform: uppercase; }
  .ai-provider-advanced-body { display: grid; gap: 8px; margin-top: 12px; }
  .ai-provider-advanced-body p { margin: 0; color: var(--ink-faint); font-size: 10px; line-height: 1.5; }
  .ai-model-picker-control { position: relative; }
  .ai-model-picker-control input { width: 100%; box-sizing: border-box; }
  .ai-model-picker-menu { position: absolute; top: calc(100% + 5px); right: 0; left: 0; z-index: 5; display: grid; max-height: 190px; overflow: auto; padding: 4px; border: 1px solid #d8cdbd; border-radius: 8px; background: var(--surface); box-shadow: 0 10px 24px rgba(48,45,38,.16); }
  .ai-model-picker-menu button { overflow: hidden; padding: 8px 9px; border: 0; border-radius: 5px; background: transparent; color: var(--ink); font: 11px var(--font-body); text-align: left; text-overflow: ellipsis; white-space: nowrap; cursor: pointer; }
  .ai-model-picker-menu button:hover, .ai-model-picker-menu button:focus-visible { background: var(--surface-muted); outline: 0; }
  .settings-actions { display: flex; gap: 8px; flex-wrap: wrap; margin: 12px 0; }
  .settings-path-field { display: grid; gap: 6px; margin: 10px 0; color: #6d625d; font-size: 12px; }
  .settings-path-field input { width: 100%; box-sizing: border-box; padding: 9px 10px; border: 1px solid #d9cec7; border-radius: 8px; background: #fffdfb; color: #302a27; }
  .settings-note { margin: 10px 0; color: #6d625d; font-size: 12px; overflow-wrap: anywhere; }
  .ai-card-actions { padding-top: 2px; }
  .primary-button { padding: 10px 15px; border: 1px solid rgba(255,255,255,.08); border-radius: 8px; background: var(--accent-dark); color: #fff; font-size: 12px; font-weight: 700; box-shadow: 0 2px 0 #263d30, 0 7px 16px rgba(42,68,51,.16); cursor: pointer; transition: background .16s ease, box-shadow .16s ease, transform .16s ease; }
  .primary-button:hover { background: #2b4535; box-shadow: 0 2px 0 #263d30, 0 10px 20px rgba(42,68,51,.2); transform: translateY(-1px); }
  .primary-button:active { box-shadow: 0 1px 0 #263d30, 0 3px 8px rgba(42,68,51,.14); transform: translateY(1px); }
  .primary-button:focus-visible { outline: 3px solid rgba(180,119,63,.32); outline-offset: 2px; }
  .primary-button:disabled { opacity: .5; cursor: not-allowed; box-shadow: none; transform: none; }
  .quiet-button { padding: 10px 12px; border: 1px solid #ded8cd; border-radius: 8px; background: var(--surface); color: var(--ink-soft); font-size: 12px; box-shadow: 0 1px 2px rgba(48,45,38,.05); cursor: pointer; transition: background .16s ease, border-color .16s ease, box-shadow .16s ease, color .16s ease, transform .16s ease; }
  .quiet-button:hover { border-color: #cbbda9; background: var(--surface-muted); color: var(--ink); box-shadow: 0 3px 8px rgba(48,45,38,.08); transform: translateY(-1px); }
  .quiet-button:active { box-shadow: 0 1px 2px rgba(48,45,38,.05); transform: translateY(1px); }
  .quiet-button:focus-visible { outline: 3px solid rgba(180,119,63,.24); outline-offset: 2px; }
  .quiet-button:disabled { opacity: .5; cursor: not-allowed; box-shadow: none; transform: none; }
  .ai-status { color: #9d5b42; font-size: 11px; line-height: 1.4; }
  .ai-status.ok { color: #557d63; font-weight: 700; }
  .ai-feedback { margin: -4px 0 0; padding: 9px 10px; border-left: 3px solid #c99965; background: #f8f0e5; color: var(--ink-soft); font-size: 11px; line-height: 1.45; }
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
    .settings-view { padding: 18px 16px 28px; }
    .settings-layout { grid-template-columns: 1fr; }
    .settings-nav { flex-direction: row; flex-wrap: wrap; }
    .settings-nav-button { width: auto; }
    .ai-field-grid { grid-template-columns: 1fr; }
    .ai-field-wide { grid-column: auto; }
    .ai-card-heading { flex-direction: column; gap: 10px; }
    .ai-overview-details { grid-template-columns: 1fr; }
    .ai-modal-backdrop { padding: 12px; }
    .ai-provider-modal { max-height: calc(100vh - 24px); padding: 17px; }
    .ai-modal-heading { flex-direction: column; gap: 10px; }
    .ai-remote-controls { grid-template-columns: 1fr; }
  }
</style>
