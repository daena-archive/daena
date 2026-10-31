<script lang="ts">
import type { ModuleManifest } from "../../../packages/module-api/src/index";
import { X } from "@lucide/svelte";
import { buildModuleContext } from "$lib/modules/context";
import { trapModalTab } from "$lib/shell/modalFocus";
import loreManifestJson from "../../../packages/modules/lore/manifest.json";
import type { ProfileComponent, ProfileComponentKind, ProfileDocument } from "./profile.ts";
import { PROFILE_COMPONENT_KINDS } from "./profile.ts";
import {
  applyPresetFields,
  blankProfilePreset,
  createProfilePreset,
  deleteProfilePreset,
  duplicateProfilePreset,
  groupManagerProfilePresets,
  loadAllProfilePresets,
  presetFromProfile,
  PROFILE_PRESET_ICON_KEYS,
  profilePresetComponentPreview,
  profilePresetIconKey,
  qualifiedProfileEntityType,
  resetBundledProfilePreset,
  setProfilePresetHidden,
  updateProfilePreset,
  type LoadedProfilePreset,
  type ProfilePresetComponentDraft,
  type ProfilePresetDocument,
  type ProfilePresetIconKey,
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
let icon = $state<ProfilePresetIconKey>("custom");
let selectedTypes = $state<string[]>([]);
let componentDrafts = $state<ComponentDraft[]>([]);
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
const managerGroups = $derived(groupManagerProfilePresets(presets));
const previewItems = $derived(
  saving
    ? profilePresetComponentPreview({ components: profile?.components ?? [] })
    : componentDrafts
        .filter((component) => component.name.trim())
        .map((component) => ({ id: component.key, name: component.name.trim(), kind: component.kind })),
);
const iconLabels: Record<ProfilePresetIconKey, string> = {
  custom: "Custom",
  dnd: "D&D",
  fantasy: "Fantasy",
  scifi: "Sci-Fi",
};

type ComponentDraft = {
  key: string;
  id: string;
  kind: ProfileComponentKind;
  name: string;
  scale: string;
  min: string;
  max: string;
  unit: string;
  formula: string;
};

let componentKey = 0;

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
  icon = "custom";
  selectedTypes = current ? [current] : [];
  componentDrafts = [];
});

$effect(() => {
  dialogEl?.focus();
});

function fields() {
  return {
    name,
    description,
    entityTypes: selectedTypes,
    genre,
    icon,
    ...(saving ? {} : { components: componentFields() }),
  };
}

function draftFromComponent(component: ProfileComponent): ComponentDraft {
  componentKey += 1;
  const unit = component.unit ?? (component.value.type === "resource" ? (component.value.unit ?? "") : "");
  return {
    key: component.id || `draft-${componentKey}`,
    id: component.id,
    kind: component.kind,
    name: component.name,
    scale: component.scale?.join(", ") ?? "",
    min: component.min === undefined ? "" : String(component.min),
    max: component.max === undefined ? "" : String(component.max),
    unit,
    formula: component.formula ?? "",
  };
}

function numericField(value: string, label: string): number | undefined {
  const trimmed = value.trim();
  if (!trimmed) return undefined;
  const number = Number(trimmed);
  if (!Number.isFinite(number)) throw new Error(`Component ${label} must be a number`);
  return number;
}

function componentFields(): ProfilePresetComponentDraft[] {
  return componentDrafts.map((draft) => {
    const min = numericField(draft.min, "min");
    const max = numericField(draft.max, "max");
    const unit = draft.unit.trim();
    const formula = draft.formula.trim();
    return {
      ...(draft.id ? { id: draft.id } : {}),
      kind: draft.kind,
      name: draft.name,
      scale: draft.scale
        .split(",")
        .map((item) => item.trim())
        .filter(Boolean),
      ...(min !== undefined ? { min } : {}),
      ...(max !== undefined ? { max } : {}),
      unit,
      ...(formula ? { formula } : {}),
    };
  });
}

function entityBadge(preset: LoadedProfilePreset): string {
  const types = preset.document.entityTypes ?? [];
  if (!types.length) return "Universal";
  return types.map((id) => entityTypeOptions.find((option) => option.id === id)?.label ?? id).join(", ");
}

function openCreate() {
  editing = null;
  creating = true;
  name = "";
  description = "";
  genre = "";
  icon = "custom";
  selectedTypes = qualifiedProfileEntityType(entityType) ? [qualifiedProfileEntityType(entityType)] : [];
  componentDrafts = [];
  error = "";
}

function openEdit(preset: LoadedProfilePreset) {
  creating = false;
  editing = preset;
  name = preset.document.name;
  description = preset.document.description ?? "";
  genre = preset.document.genre ?? "";
  icon = profilePresetIconKey(preset.document.icon);
  selectedTypes = [...(preset.document.entityTypes ?? [])];
  componentDrafts = preset.document.components.map(draftFromComponent);
  error = "";
}

function addComponent() {
  componentKey += 1;
  componentDrafts = [
    ...componentDrafts,
    { key: `new-${componentKey}`, id: "", kind: "attribute", name: "", scale: "", min: "", max: "", unit: "", formula: "" },
  ];
}

function removeComponent(key: string) {
  componentDrafts = componentDrafts.filter((component) => component.key !== key);
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
        <label>
          Icon
          <select bind:value={icon}>
            {#each PROFILE_PRESET_ICON_KEYS as key (key)}
              <option value={key}>{iconLabels[key]}</option>
            {/each}
          </select>
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
        {#if !saving}
          <fieldset>
            <legend>Components</legend>
            {#each componentDrafts as draft (draft.key)}
              <div class="component">
                <label>
                  Name
                  <input bind:value={draft.name} maxlength="80" />
                </label>
                <label>
                  Kind
                  <select bind:value={draft.kind}>
                    {#each PROFILE_COMPONENT_KINDS as kind (kind)}
                      <option value={kind}>{kind}</option>
                    {/each}
                  </select>
                </label>
                <label>
                  Scale
                  <input bind:value={draft.scale} placeholder="Comma-separated" />
                </label>
                <label>
                  Min
                  <input bind:value={draft.min} inputmode="decimal" />
                </label>
                <label>
                  Max
                  <input bind:value={draft.max} inputmode="decimal" />
                </label>
                <label>
                  Unit
                  <input bind:value={draft.unit} maxlength="40" />
                </label>
                <label class="formula">
                  Formula
                  <input bind:value={draft.formula} />
                </label>
                <button class="quiet-button" type="button" onclick={() => removeComponent(draft.key)}>Remove</button>
              </div>
            {/each}
            <button class="quiet-button" type="button" onclick={addComponent}>Add component</button>
          </fieldset>
        {/if}
        <section class="preview" aria-label="Preset preview">
          <h3>Preview</h3>
          {#if previewItems.length}
            <ul>
              {#each previewItems as item (item.id)}
                <li>
                  <strong>{item.name}</strong>
                  <span>{item.kind}</span>
                </li>
              {/each}
            </ul>
          {:else}
            <p>No components.</p>
          {/if}
        </section>
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
        {#each managerGroups as group (group.genre ?? "")}
          <section class="genre" aria-label={group.genre ? `${group.genre} presets` : "Presets"}>
            {#if group.genre}<h3>{group.genre}</h3>{/if}
            <ul>
          {#each group.presets as preset (preset.id)}
            <li>
              <div>
                <strong>{preset.document.name}</strong>
                <span>
                  {preset.document.builtin ? "Bundled" : "Yours"}
                  {#if preset.document.hidden}· Hidden{/if}
                  <span class="badge">{entityBadge(preset)}</span>
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
          </section>
        {/each}
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
input,
select {
  border: 1px solid var(--line);
  border-radius: 8px;
  background: var(--surface);
  color: var(--ink);
  padding: 8px 10px;
}
h3 {
  margin: 12px 0 0;
  color: var(--ink-soft);
  font: 600 13px/1.2 var(--font-ui, inherit);
}
.badge {
  display: inline-block;
  margin-left: 6px;
  border: 1px solid var(--line);
  border-radius: 999px;
  padding: 1px 6px;
}
.component {
  display: grid;
  grid-template-columns: 1.4fr 0.8fr 1fr 0.5fr 0.5fr 0.6fr;
  gap: 8px;
  align-items: end;
  padding-bottom: 8px;
  border-bottom: 1px solid var(--line);
}
.component .formula {
  grid-column: 1 / -2;
}
.preview h3 {
  margin-top: 0;
}
.preview p {
  margin: 0;
  color: var(--ink-muted);
  font-size: 12px;
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
