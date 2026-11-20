<script lang="ts">
import { nativeErrorText } from "$lib/i18n/native-errors.ts";
import { ui } from "$lib/i18n/ui";
import { RefreshCw, ServerCog } from "@lucide/svelte";
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
        ? $ui("ComfyUI is available, but no checkpoint models were found.")
        : $ui("{0} checkpoint model{1} available.", discovery.models.length, discovery.models.length === 1 ? "" : $ui("s")),
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
          ? $ui("ComfyUI and the selected model are ready.")
          : $ui("ComfyUI is available; select a discovered checkpoint model.")
        : (status.error ?? $ui("ComfyUI is unavailable.")),
    );
  } catch (cause) {
    status = null;
    showMessage(friendlyError(cause));
  } finally {
    busy = false;
  }
}
</script>

<section class="operation-card" aria-labelledby="image-provider-heading">
  <div class="split-heading">
    <div>
      <strong id="image-provider-heading">{$ui("ComfyUI")}</strong>
      <p>{$ui("Generated candidates stay temporary until you explicitly accept them as project assets.")}</p>
    </div>
    <span class="state-pill ok">{$ui("Local")}</span>
  </div>

  <label class="enable-row">
    <input
      type="checkbox"
      checked={settings.enabled}
      onchange={(event) => onChange("enabled", (event.currentTarget as HTMLInputElement).checked)} />
    <span><strong>{$ui("Enable AI image generation")}</strong><small>{$ui("No hosted fallback is used.")}</small></span>
  </label>

  {#if settings.enabled}
    <div class="fields">
      <label class="wide"
        >{$ui("Local ComfyUI address")}<input
          value={settings.endpoint}
          placeholder={$ui("http://127.0.0.1:8188")}
          spellcheck="false"
          oninput={(event) => {
            discovery = null;
            status = null;
            onChange("endpoint", (event.currentTarget as HTMLInputElement).value);
          }} />
        <small>{$ui("Only loopback HTTP addresses are accepted in V1.")}</small></label>
      <label
        >{$ui("Checkpoint model")}<select
          value={settings.model}
          onchange={(event) => onChange("model", (event.currentTarget as HTMLSelectElement).value)}>
          <option value="">{$ui("Choose a discovered model")}</option>
          {#if settings.model && !discovery?.models.includes(settings.model)}
            <option value={settings.model}>{$ui("{0} (not discovered)", settings.model)}</option>
          {/if}
          {#each discovery?.models ?? [] as model}<option value={model}>{model}</option>{/each}
        </select></label>
    </div>
    <div class="actions">
      <button type="button" class="primary-button" disabled={busy} onclick={() => void discover()}>
        <RefreshCw size={13} strokeWidth={1.8} aria-hidden="true" />
        {busy ? $ui("Checking…") : $ui("Discover models")}
      </button>
      <button type="button" class="quiet-button" disabled={busy} onclick={() => void check()}>
        <ServerCog size={13} strokeWidth={1.8} aria-hidden="true" /> {$ui("Test connection")} </button>
      {#if message}<span class="status-badge" class:ok={status?.available && status?.modelAvailable} role="status"
          >{$nativeErrorText(message)}</span
        >{/if}
    </div>
    {#if discovery}
      <div class="capabilities">
        <strong>{$ui("Available controls")}</strong>
        <p>{discovery.capabilities.join(" · ")}</p>
      </div>
    {/if}
  {/if}
</section>

<style>
.operation-card {
  display: grid;
  gap: 14px;
  padding: 16px;
  border: 1px solid var(--line);
  border-radius: 12px;
  background: var(--surface-subtle);
}
.split-heading {
  display: flex;
  justify-content: space-between;
  align-items: flex-start;
  gap: 16px;
}
.split-heading strong,
.capabilities strong {
  display: block;
  color: var(--ink);
  font-size: 14px;
}
.split-heading p,
.capabilities p,
.enable-row small,
.fields small {
  margin: 4px 0 0;
  color: var(--ink-soft);
  font-size: 12.5px;
  line-height: 1.5;
  font-weight: 400;
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
}
.state-pill.ok,
.status-badge.ok {
  background: var(--theme-success-bg, var(--accent-bg));
  color: var(--success);
}
.status-badge {
  background: var(--surface);
  border: 1px solid var(--line);
  color: var(--ink-soft);
  font-weight: 600;
}
.enable-row {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 11px 12px;
  border: 1px solid var(--line);
  border-radius: 10px;
  background: var(--surface);
  cursor: pointer;
}
.enable-row span {
  display: grid;
  gap: 2px;
}
.enable-row strong {
  color: var(--ink);
  font-size: 13px;
}
.fields {
  display: grid;
  grid-template-columns: minmax(220px, 1.25fr) minmax(200px, 1fr);
  gap: 12px;
}
.fields label {
  display: grid;
  gap: 6px;
  color: var(--ink-soft);
  font-size: 11px;
  font-weight: 700;
}
.fields input,
.fields select {
  width: 100%;
  min-height: 36px;
  padding: 8px 10px;
  border: 1px solid var(--line);
  border-radius: 8px;
  background: var(--surface);
  color: var(--ink);
  font: inherit;
  font-weight: 500;
  box-sizing: border-box;
}
.actions {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 8px;
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
button:disabled {
  opacity: 0.55;
  cursor: not-allowed;
}
.capabilities {
  padding-top: 12px;
  border-top: 1px solid var(--line);
}
@media (max-width: 720px) {
  .fields {
    grid-template-columns: 1fr;
  }
}
</style>
