<script lang="ts">
import type { ModuleManifest } from "../../../packages/module-api/src/index";
import { X } from "@lucide/svelte";
import { buildModuleContext } from "$lib/modules/context";
import { trapModalTab } from "$lib/shell/modalFocus";
import loreManifestJson from "../../../packages/modules/lore/manifest.json";
import type { ProfileDocument } from "./profile.ts";
import {
  applyPresetFields,
  blankProfilePreset,
  createProfilePreset,
  deleteProfilePreset,
  duplicateProfilePreset,
  loadAllProfilePresets,
  presetFromProfile,
  qualifiedProfileEntityType,
  resetBundledProfilePreset,
  setProfilePresetHidden,
  updateProfilePreset,
  type LoadedProfilePreset,
  type ProfilePresetDocument,
} from "./profilePresets.ts";

export type ProfileEntityTypeOption = { id: string; label: string };

let {
  projectId,
  mode = "manage",
  entityType = null,
  entityTypeOptions = [],
  profile = null,
  originId = undefined,
  onClose,
  onChanged,
}: {
  projectId: string;
  mode?: "manage" | "save";
  entityType?: string | null;
  entityTypeOptions?: ProfileEntityTypeOption[];
  profile?: ProfileDocument | null;
  originId?: string;
  onClose: () => void;
  onChanged: () => void;
} = $props();

const context = $derived(buildModuleContext(loreManifestJson as unknown as ModuleManifest, projectId));
let presets = $state<LoadedProfilePreset[]>([]);
let loading = $state(true);
let busy = $state(false);
let error = $state("");
let editing = $state<LoadedProfilePreset | null>(null);
let creating = $state(false);
const saving = $derived(mode === "save");
let name = $state("");
let description = $state("");
let genre = $state("");
let selectedTypes = $state<string[]>([]);
let dialogEl = $state<HTMLElement | null>(null);
let seededSave = false;
let pending = $state<{ title: string; message: string; confirmLabel: string; run: () => Promise<void> } | null>(null);

const genres = $derived(
  [...new Set(presets.map((preset) => preset.document.genre).filter((value): value is string => Boolean(value)))].sort(
    (left, right) => left.localeCompare(right),
  ),
);
const typeOptions = $derived.by(() => {
  const options = [...entityTypeOptions];
  const current = qualifiedProfileEntityType(entityType);
  if (current && !options.some((option) => option.id === current)) options.push({ id: current, label: current });
  for (const id of selectedTypes) {
    if (!options.some((option) => option.id === id)) options.push({ id, label: id });
  }
  return options;
});
const formOpen = $derived(saving || creating || editing !== null);

$effect(() => {
  const moduleContext = context;
  let cancelled = false;
  loading = true;
  void loadAllProfilePresets(moduleContext)
    .then((next) => {
      if (!cancelled) presets = next;
    })
    .catch((cause) => {
      if (!cancelled) error = cause instanceof Error ? cause.message : String(cause);
    })
    .finally(() => {
      if (!cancelled) loading = false;
    });
  return () => {
    cancelled = true;
  };
});

$effect(() => {
  if (mode !== "save" || seededSave) return;
  seededSave = true;
  const current = qualifiedProfileEntityType(entityType);
  name = "";
  description = "";
  genre = "";
  selectedTypes = current ? [current] : [];
});

$effect(() => {
  dialogEl?.focus();
});

function fields() {
  return { name, description, entityTypes: selectedTypes, genre };
}

function openCreate() {
  editing = null;
  creating = true;
  name = "";
  description = "";
  genre = "";
  selectedTypes = qualifiedProfileEntityType(entityType) ? [qualifiedProfileEntityType(entityType)] : [];
  error = "";
}

function openEdit(preset: LoadedProfilePreset) {
  creating = false;
  editing = preset;
  name = preset.document.name;
  description = preset.document.description ?? "";
  genre = preset.document.genre ?? "";
  selectedTypes = [...(preset.document.entityTypes ?? [])];
  error = "";
}

function closeForm() {
  if (mode === "save" && saving) {
    onClose();
    return;
  }
  creating = false;
  editing = null;
}

function toggleType(id: string) {
  selectedTypes = selectedTypes.includes(id) ? selectedTypes.filter((item) => item !== id) : [...selectedTypes, id];
}

async function persist(next: ProfilePresetDocument, preset?: LoadedProfilePreset | null) {
  busy = true;
  error = "";
  try {
    if (preset) await updateProfilePreset(context, preset, next);
    else await createProfilePreset(context, next);
    presets = await loadAllProfilePresets(context);
    onChanged();
    if (mode === "save") onClose();
    else closeForm();
  } catch (cause) {
    error = cause instanceof Error ? cause.message : String(cause);
  } finally {
    busy = false;
  }
}

async function saveForm() {
  try {
    if (saving) {
      if (!profile) throw new Error("Create a Profile before saving a preset");
      if (loading) return;
      const origin = presets.find((preset) => preset.id === originId)?.document ?? null;
      await persist(presetFromProfile(profile, fields(), origin));
      return;
    }
    if (editing) {
      await persist(applyPresetFields(editing.document, fields()), editing);
      return;
    }
    await persist(blankProfilePreset(fields()));
  } catch (cause) {
    error = cause instanceof Error ? cause.message : String(cause);
  }
}

async function duplicate(preset: LoadedProfilePreset) {
  busy = true;
  error = "";
  try {
    await duplicateProfilePreset(context, preset);
    presets = await loadAllProfilePresets(context);
    onChanged();
  } catch (cause) {
    error = cause instanceof Error ? cause.message : String(cause);
  } finally {
    busy = false;
  }
}

async function toggleHidden(preset: LoadedProfilePreset) {
  busy = true;
  error = "";
  try {
    await setProfilePresetHidden(context, preset, preset.document.hidden !== true);
    presets = await loadAllProfilePresets(context);
    onChanged();
  } catch (cause) {
    error = cause instanceof Error ? cause.message : String(cause);
  } finally {
    busy = false;
  }
}

function askConfirm(title: string, message: string, confirmLabel: string, run: () => Promise<void>) {
  pending = { title, message, confirmLabel, run };
}

async function runPending() {
  const action = pending;
  pending = null;
  if (!action) return;
  busy = true;
  error = "";
  try {
    await action.run();
    presets = await loadAllProfilePresets(context);
    onChanged();
  } catch (cause) {
    error = cause instanceof Error ? cause.message : String(cause);
  } finally {
    busy = false;
  }
}

function resetPreset(preset: LoadedProfilePreset) {
  askConfirm(
    "Reset preset?",
    `Restore ${preset.document.name} to the shipped definition? Your edits to this preset are discarded. Existing Profiles are not changed.`,
    "Reset",
    () => resetBundledProfilePreset(context, preset).then(() => undefined),
  );
}

function removePreset(preset: LoadedProfilePreset) {
  askConfirm(
    "Delete preset?",
    `Delete ${preset.document.name}? Existing Profiles keep their values and are not changed.`,
    "Delete preset",
    () => deleteProfilePreset(context, preset),
  );
}

function formTitle() {
  if (saving) return "Save as Preset";
  if (editing) return "Edit preset";
  if (creating) return "New preset";
  return "Presets";
}

function onKeydown(event: KeyboardEvent) {
  if (event.key === "Escape") {
    event.preventDefault();
    event.stopPropagation();
    if (pending) {
      pending = null;
      return;
    }
    if (formOpen && mode !== "save") closeForm();
    else onClose();
    return;
  }
  trapModalTab(event, dialogEl);
}
</script>

<div class="overlay" role="presentation">
  <button type="button" class="backdrop" tabindex="-1" aria-label="Close presets" onclick={onClose}></button>
  <div
    class="dialog"
    role="dialog"
    aria-modal="true"
    aria-labelledby="preset-manager-title"
    tabindex="-1"
    bind:this={dialogEl}
    onkeydown={onKeydown}>
    <header>
      <div>
        <span class="kicker">PRESETS</span>
        <h2 id="preset-manager-title">{formTitle()}</h2>
      </div>
      <button type="button" class="close" aria-label="Close presets" onclick={onClose}
        ><X size={16} strokeWidth={1.8} /></button>
    </header>
    {#if error}<p class="banner" role="alert">{error}</p>{/if}
    {#if formOpen}
      <form
        class="body"
        onsubmit={(event) => {
          event.preventDefault();
          void saveForm();
        }}>
        <label>
          Name
          <input bind:value={name} required maxlength="80" />
        </label>
        <label>
          Description
          <input bind:value={description} maxlength="200" />
        </label>
        <label>
          Genre
          <input bind:value={genre} list="preset-genres" placeholder="Optional" />
        </label>
        <datalist id="preset-genres">
          {#each genres as item (item)}<option value={item}></option>{/each}
        </datalist>
        <fieldset>
          <legend>Entity types</legend>
          <p>Leave empty for a universal preset. Suggestions use these qualified types.</p>
          {#each typeOptions as option (option.id)}
            <label class="check">
              <input
                type="checkbox"
                checked={selectedTypes.includes(option.id)}
                onchange={() => toggleType(option.id)} />
              {option.label}
            </label>
          {/each}
        </fieldset>
        <footer>
          <button class="quiet-button" type="button" disabled={busy} onclick={closeForm}>Cancel</button>
          <button class="primary-button" type="submit" disabled={busy || (saving && loading)}
            >{busy ? "Saving…" : "Save"}</button>
        </footer>
      </form>
    {:else if loading}
      <p class="body">Loading presets…</p>
    {:else}
      <div class="body">
        <ul>
          {#each presets as preset (preset.id)}
            <li>
              <div>
                <strong>{preset.document.name}</strong>
                <span>
                  {preset.document.builtin ? "Bundled" : "Yours"}
                  {#if preset.document.hidden}· Hidden{/if}
                  {#if preset.document.genre}· {preset.document.genre}{/if}
                  · {preset.document.entityTypes?.length ? preset.document.entityTypes.join(", ") : "Universal"}
                </span>
              </div>
              <div class="actions">
                <button type="button" class="quiet-button" disabled={busy} onclick={() => openEdit(preset)}
                  >Edit</button>
                <button type="button" class="quiet-button" disabled={busy} onclick={() => void duplicate(preset)}
                  >Duplicate</button>
                <button type="button" class="quiet-button" disabled={busy} onclick={() => void toggleHidden(preset)}>
                  {preset.document.hidden ? "Show" : "Hide"}
                </button>
                {#if preset.document.builtin}
                  <button type="button" class="quiet-button" disabled={busy} onclick={() => resetPreset(preset)}
                    >Reset</button>
                {:else}
                  <button type="button" class="danger-button" disabled={busy} onclick={() => removePreset(preset)}
                    >Delete</button>
                {/if}
              </div>
            </li>
          {/each}
        </ul>
      </div>
      <footer>
        <button class="quiet-button" type="button" onclick={onClose}>Done</button>
        <button class="primary-button" type="button" disabled={busy} onclick={openCreate}>New preset</button>
      </footer>
    {/if}
    {#if pending}
      <div class="confirm" role="alertdialog" aria-labelledby="preset-confirm-title">
        <h3 id="preset-confirm-title">{pending.title}</h3>
        <p>{pending.message}</p>
        <div class="actions">
          <button class="quiet-button" type="button" disabled={busy} onclick={() => (pending = null)}>Cancel</button>
          <button class="danger-button" type="button" disabled={busy} onclick={() => void runPending()}
            >{pending.confirmLabel}</button>
        </div>
      </div>
    {/if}
  </div>
</div>

<style>
.overlay {
  position: fixed;
  inset: 0;
  z-index: 360;
  display: grid;
  place-items: center;
  padding: 20px;
}
.backdrop {
  position: absolute;
  inset: 0;
  border: 0;
  background: rgba(14, 23, 20, 0.45);
}
.dialog {
  position: relative;
  z-index: 1;
  display: flex;
  flex-direction: column;
  width: min(720px, 100%);
  max-height: min(84vh, 760px);
  overflow: hidden;
  border: 1px solid var(--line-strong);
  border-radius: 16px;
  background: var(--surface);
  outline: none;
}
header,
footer {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  padding: 16px 20px;
}
footer {
  border-top: 1px solid var(--line);
}
.kicker {
  color: var(--accent);
  font-size: 9px;
  font-weight: 800;
  letter-spacing: 0.14em;
}
h2 {
  margin: 4px 0 0;
  color: var(--ink);
  font: 500 22px/1.2 var(--font-display);
}
.close {
  width: 32px;
  height: 32px;
  border: 0;
  border-radius: 8px;
  background: var(--surface-muted);
  color: var(--ink-soft);
}
.body {
  display: grid;
  gap: 12px;
  min-height: 0;
  overflow: auto;
  padding: 0 20px 16px;
}
label,
fieldset {
  display: grid;
  gap: 6px;
  color: var(--ink-soft);
  font-size: 12px;
}
input {
  border: 1px solid var(--line);
  border-radius: 8px;
  background: var(--surface);
  color: var(--ink);
  padding: 8px 10px;
}
fieldset {
  border: 1px solid var(--line);
  border-radius: 10px;
  padding: 10px;
}
fieldset p,
li span {
  margin: 0;
  color: var(--ink-muted);
  font-size: 12px;
}
.check {
  display: flex;
  align-items: center;
  gap: 8px;
  color: var(--ink);
}
ul {
  display: grid;
  gap: 8px;
  margin: 0;
  padding: 0;
  list-style: none;
}
li {
  display: flex;
  justify-content: space-between;
  gap: 12px;
  padding: 10px 0;
  border-bottom: 1px solid var(--line);
}
.actions {
  display: flex;
  flex-wrap: wrap;
  justify-content: flex-end;
  gap: 6px;
}
.quiet-button,
.primary-button,
.danger-button {
  min-height: 34px;
  padding: 0 14px;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  border: 1px solid var(--line-strong);
  border-radius: 8px;
  font: 600 12px/1 var(--font-ui, inherit);
  cursor: pointer;
}
.quiet-button {
  background: var(--surface);
  color: var(--ink-soft);
}
.quiet-button:hover {
  background: var(--surface-muted);
  color: var(--ink);
}
.primary-button {
  border-color: transparent;
  background: var(--accent-dark);
  color: var(--on-accent);
}
.danger-button {
  border-color: var(--danger-line);
  background: transparent;
  color: var(--danger);
}
.quiet-button:focus-visible,
.primary-button:focus-visible,
.danger-button:focus-visible {
  outline: 2px solid var(--accent);
  outline-offset: 1px;
}
.quiet-button:disabled,
.primary-button:disabled,
.danger-button:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}
.banner {
  margin: 0 20px 8px;
  color: var(--danger, #9a3412);
}
.confirm {
  display: grid;
  gap: 8px;
  margin: 0 20px 16px;
  padding: 12px;
  border: 1px solid var(--line);
  border-radius: 10px;
  background: var(--surface-muted);
}
.confirm h3,
.confirm p {
  margin: 0;
}
.confirm p {
  color: var(--ink-soft);
  font-size: 13px;
}
</style>
