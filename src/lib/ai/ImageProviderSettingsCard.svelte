<script lang="ts">
import { Image, RefreshCw, ServerCog } from "@lucide/svelte";
import {
  project,
  type ImageProviderDiscovery,
  type ImageProviderSettings,
  type ImageProviderStatus,
} from "$lib/project/client";

let {
  settings,
  onChange,
}: {
  settings: ImageProviderSettings;
  onChange: (key: keyof ImageProviderSettings, value: string | boolean) => void;
} = $props();

let discovery = $state<ImageProviderDiscovery | null>(null);
let status = $state<ImageProviderStatus | null>(null);
let busy = $state(false);
let message = $state("");
let messageTimer: number | null = null;

function showMessage(value: string) {
  if (messageTimer !== null) window.clearTimeout(messageTimer);
  message = value;
  if (!value) return;
  messageTimer = window.setTimeout(() => {
    message = "";
    messageTimer = null;
  }, 5000);
}

function friendlyError(cause: unknown) {
  return cause instanceof Error ? cause.message : String(cause);
}

async function discover() {
  if (busy) return;
  busy = true;
  showMessage("");
  try {
    discovery = await project.imageProviderDiscover();
    showMessage(
      discovery.models.length === 0
        ? "ComfyUI is available, but no checkpoint models were found."
        : `${discovery.models.length} checkpoint model${discovery.models.length === 1 ? "" : "s"} available.`,
    );
    if (!settings.model && discovery.models.length === 1) onChange("model", discovery.models[0]);
  } catch (cause) {
    discovery = null;
    showMessage(friendlyError(cause));
  } finally {
    busy = false;
  }
}

async function check() {
  if (busy) return;
  busy = true;
  showMessage("");
  try {
    status = await project.imageProviderStatus();
    showMessage(
      status.available
        ? status.modelAvailable
          ? "ComfyUI and the selected model are ready."
          : "ComfyUI is available; select a discovered checkpoint model."
        : (status.error ?? "ComfyUI is unavailable."),
    );
  } catch (cause) {
    status = null;
    showMessage(friendlyError(cause));
  } finally {
    busy = false;
  }
}
</script>

<section class="image-provider-card" aria-labelledby="image-provider-heading">
  <div class="heading">
    <div class="icon"><Image size={18} strokeWidth={1.7} aria-hidden="true" /></div>
    <div>
      <span>LOCAL IMAGE GENERATION</span>
      <strong id="image-provider-heading">ComfyUI</strong>
      <p>Generated candidates stay temporary until you explicitly accept them as project assets.</p>
    </div>
    <span class="local-badge">Local</span>
  </div>

  <label class="enable-row">
    <input
      type="checkbox"
      checked={settings.enabled}
      onchange={(event) => onChange("enabled", (event.currentTarget as HTMLInputElement).checked)} />
    <span><strong>Enable AI image generation</strong><small>No hosted fallback is used.</small></span>
  </label>

  {#if settings.enabled}
    <div class="fields">
      <label class="wide"
        >Local ComfyUI address<input
          value={settings.endpoint}
          placeholder="http://127.0.0.1:8188"
          spellcheck="false"
          oninput={(event) => {
            discovery = null;
            status = null;
            onChange("endpoint", (event.currentTarget as HTMLInputElement).value);
          }} />
        <small>Only loopback HTTP addresses are accepted in V1.</small></label>
      <label
        >Checkpoint model<select
          value={settings.model}
          onchange={(event) => onChange("model", (event.currentTarget as HTMLSelectElement).value)}>
          <option value="">Choose a discovered model</option>
          {#if settings.model && !discovery?.models.includes(settings.model)}
            <option value={settings.model}>{settings.model} (not discovered)</option>
          {/if}
          {#each discovery?.models ?? [] as model}<option value={model}>{model}</option>{/each}
        </select></label>
    </div>
    <div class="actions">
      <button type="button" class="primary" disabled={busy} onclick={() => void discover()}>
        <RefreshCw size={13} strokeWidth={1.8} aria-hidden="true" />
        {busy ? "Checking…" : "Discover models"}
      </button>
      <button type="button" class="quiet" disabled={busy} onclick={() => void check()}>
        <ServerCog size={13} strokeWidth={1.8} aria-hidden="true" /> Test connection
      </button>
      {#if message}<span class:ok={status?.available && status?.modelAvailable} role="status">{message}</span>{/if}
    </div>
    {#if discovery}
      <div class="capabilities">
        <strong>Available controls</strong>
        <p>{discovery.capabilities.join(" · ")}</p>
      </div>
    {/if}
  {/if}
</section>

<style>
.image-provider-card {
  padding: 20px;
  border: 1px solid var(--theme-neutral-border, #d9e0d8);
  border-radius: 14px;
  background: var(--theme-surface-bg, #fff);
  box-shadow: 0 10px 32px rgba(31, 42, 33, 0.05);
}
.heading {
  display: grid;
  grid-template-columns: auto minmax(0, 1fr) auto;
  align-items: start;
  gap: 12px;
}
.icon {
  display: grid;
  width: 34px;
  height: 34px;
  place-items: center;
  border-radius: 9px;
  background: var(--theme-success-bg, #edf4ec);
  color: var(--theme-success-text, #486a4e);
}
.heading span,
.capabilities strong {
  color: var(--theme-neutral-text-soft, #778279);
  font-size: 9px;
  font-weight: 800;
  letter-spacing: 0.09em;
  text-transform: uppercase;
}
.heading strong {
  display: block;
  margin-top: 3px;
  color: var(--theme-neutral-text, #263228);
  font-size: 15px;
}
.heading p,
.capabilities p {
  margin: 4px 0 0;
  color: var(--theme-neutral-text-soft, #737c74);
  font-size: 11px;
  line-height: 1.5;
}
.heading .local-badge {
  padding: 5px 8px;
  border-radius: 999px;
  background: var(--theme-success-bg, #e8f3e8);
  color: var(--theme-success-text, #407047);
  letter-spacing: 0;
  text-transform: none;
}
.enable-row {
  display: flex;
  align-items: center;
  gap: 9px;
  margin-top: 17px;
  padding: 11px 12px;
  border-radius: 9px;
  background: var(--theme-success-bg, #f4f6f3);
  cursor: pointer;
}
.enable-row span {
  display: flex;
  flex-direction: column;
  gap: 2px;
}
.enable-row strong {
  color: var(--theme-neutral-text, #344036);
  font-size: 11px;
}
.enable-row small,
.fields small {
  color: var(--theme-neutral-text-soft, #7a847b);
  font-size: 9px;
}
.fields {
  display: grid;
  grid-template-columns: minmax(220px, 1.25fr) minmax(200px, 1fr);
  gap: 12px;
  margin-top: 14px;
}
.fields label {
  display: flex;
  flex-direction: column;
  gap: 5px;
  color: var(--theme-neutral-text-soft, #536056);
  font-size: 10px;
  font-weight: 700;
}
.fields input,
.fields select {
  width: 100%;
  min-height: 36px;
  padding: 0 10px;
  border: 1px solid var(--theme-neutral-border, #d4dbd3);
  border-radius: 8px;
  background: var(--theme-surface-bg, #fff);
  color: var(--theme-neutral-text, #2f3931);
  font: inherit;
  font-weight: 500;
}
.actions {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 8px;
  margin-top: 13px;
}
.actions button {
  display: inline-flex;
  min-height: 32px;
  align-items: center;
  gap: 6px;
  padding: 0 11px;
  border: 1px solid var(--theme-neutral-border, #ccd5cb);
  border-radius: 8px;
  font-size: 10px;
  font-weight: 700;
  cursor: pointer;
}
.actions .primary {
  border-color: var(--theme-success-border, #486d50);
  background: #486d50;
  color: #fff;
}
.actions .quiet {
  background: var(--theme-surface-bg, #fff);
  color: var(--theme-neutral-text-soft, #4c5a4f);
}
.actions span {
  color: var(--theme-danger-text, #8a5548);
  font-size: 10px;
}
.actions span.ok {
  color: var(--theme-success-text, #3f7548);
}
.capabilities {
  margin-top: 13px;
  padding-top: 12px;
  border-top: 1px solid var(--theme-neutral-border, #edf0ec);
}
@media (max-width: 720px) {
  .fields {
    grid-template-columns: 1fr;
  }
}
</style>
