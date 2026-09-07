<script lang="ts">
import type { ModuleManifest } from "../../../packages/module-api/src/index";
import { confirmDialog } from "$lib/dialogs.svelte";
import { buildModuleContext } from "$lib/modules/context";
import loreManifestJson from "../../../packages/modules/lore/manifest.json";
import {
  PROFILE_COMPONENT_KINDS,
  PROFILE_VALUE_TYPES,
  allocationRemaining,
  allocationSpent,
  componentHasValue,
  emptyProfile,
  emptyValue,
  newComponent,
  profileValidationErrors,
  type ProfileComponent,
  type ProfileComponentKind,
  type ProfileDocument,
  type ProfilePresetOrigin,
  type ProfileValueType,
} from "./profile.ts";
import { PROFILE_PRESETS, profileFromPreset, profilePresetLabel } from "./profilePresets.ts";
import { createProfile, deleteProfile, loadProfile, saveProfile, type StoredProfile } from "./profileStore";

let {
  projectId,
  entityId,
  entityName,
}: {
  projectId: string;
  entityId: string;
  entityName: string;
} = $props();

const context = $derived(buildModuleContext(loreManifestJson as unknown as ModuleManifest, projectId));

let stored = $state<StoredProfile | null>(null);
let draft = $state<ProfileDocument>(emptyProfile());
let loading = $state(true);
let error = $state("");
let addingKind = $state<ProfileComponentKind>("attribute");
let selectedPreset = $state<ProfilePresetOrigin>("custom");
let writeChain = Promise.resolve();
let persistGeneration = 0;
const remaining = $derived(allocationRemaining(draft));

$effect(() => {
  const id = entityId;
  const moduleContext = context;
  let cancelled = false;
  persistGeneration += 1;
  writeChain = Promise.resolve();
  loading = true;
  error = "";
  void loadProfile(moduleContext, id)
    .then((next) => {
      if (cancelled) return;
      stored = next;
      draft = next ? structuredClone(next.value) : emptyProfile();
      error = next?.error ?? "";
    })
    .catch((cause) => {
      if (cancelled) return;
      stored = null;
      draft = emptyProfile();
      error = cause instanceof Error ? cause.message : String(cause);
    })
    .finally(() => {
      if (!cancelled) loading = false;
    });
  return () => {
    cancelled = true;
  };
});

async function persist(next: ProfileDocument) {
  const ownerId = entityId;
  const generation = persistGeneration;
  writeChain = writeChain.then(async () => {
    const current = stored;
    if (!current || generation !== persistGeneration) return;
    try {
      const saved = await saveProfile(context, ownerId, current, next);
      if (generation !== persistGeneration) return;
      stored = saved;
      draft = structuredClone(saved.value);
      error = "";
    } catch (cause) {
      if (generation !== persistGeneration) return;
      const message = cause instanceof Error ? cause.message : String(cause);
      if (/revision/i.test(message)) {
        try {
          const reloaded = await loadProfile(context, ownerId);
          if (generation !== persistGeneration) return;
          stored = reloaded;
          draft = reloaded ? structuredClone(reloaded.value) : emptyProfile();
          error = reloaded?.error ?? "Profile changed elsewhere. Reloaded.";
          return;
        } catch (reloadCause) {
          error = reloadCause instanceof Error ? reloadCause.message : String(reloadCause);
          return;
        }
      }
      error = message;
    }
  });
  await writeChain;
}

async function persistDraft(next: ProfileDocument) {
  draft = next;
  const errors = profileValidationErrors(next);
  if (errors.length) {
    error = errors[0];
    return;
  }
  await persist(next);
}

function parseOptionalNumber(raw: string): number | null {
  const trimmed = raw.trim();
  if (!trimmed) return null;
  const value = Number(trimmed);
  if (!Number.isFinite(value)) {
    error = "Enter a number";
    return Number.NaN;
  }
  return value;
}

async function addProfile() {
  error = "";
  try {
    stored = await createProfile(context, entityId, profileFromPreset(selectedPreset));
    draft = structuredClone(stored.value);
  } catch (cause) {
    error = cause instanceof Error ? cause.message : String(cause);
  }
}

async function removeProfile() {
  if (!stored) return;
  if (
    !(await confirmDialog({
      title: "Remove Profile?",
      message: `Remove the Profile from ${entityName}? This does not archive the entity.`,
      confirmLabel: "Remove Profile",
      danger: true,
    }))
  )
    return;
  error = "";
  try {
    await deleteProfile(context, entityId, stored);
    stored = null;
    draft = emptyProfile();
  } catch (cause) {
    error = cause instanceof Error ? cause.message : String(cause);
  }
}

async function addComponent() {
  await persistDraft({ ...draft, components: [...draft.components, newComponent(addingKind)] });
}

async function removeComponent(component: ProfileComponent) {
  if (componentHasValue(component)) {
    if (
      !(await confirmDialog({
        title: "Remove this component?",
        message: `${component.name || "This component"} has a value. Remove it from the Profile?`,
        confirmLabel: "Remove",
        danger: true,
      }))
    )
      return;
  }
  await persistDraft({ ...draft, components: draft.components.filter((candidate) => candidate.id !== component.id) });
}

async function moveComponent(index: number, delta: number) {
  const nextIndex = index + delta;
  if (nextIndex < 0 || nextIndex >= draft.components.length) return;
  const components = [...draft.components];
  const [item] = components.splice(index, 1);
  components.splice(nextIndex, 0, item);
  await persistDraft({ ...draft, components });
}

async function patchComponent(id: string, patch: Partial<ProfileComponent>) {
  await persistDraft({
    ...draft,
    components: draft.components.map((component) => {
      if (component.id !== id) return component;
      const next = { ...component, ...patch };
      if ("min" in patch && patch.min === undefined) delete next.min;
      if ("max" in patch && patch.max === undefined) delete next.max;
      return next;
    }),
  });
}

function kindLabel(kind: ProfileComponentKind) {
  return kind.charAt(0).toUpperCase() + kind.slice(1);
}
</script>

{#if loading}
  <p class="inspector-group-empty">Loading Profile…</p>
{:else if !stored}
  {#if error}<p class="inspector-group-empty" role="status">{error}</p>{/if}
  <p class="inspector-group-empty">Optional structured attributes, skills, and traits.</p>
  <div class="add-row">
    <select bind:value={selectedPreset} aria-label="Profile preset">
      {#each PROFILE_PRESETS as preset}
        <option value={preset.id}>{preset.name}</option>
      {/each}
    </select>
    <button class="quiet-button" type="button" onclick={() => void addProfile()}>Add Profile</button>
  </div>
{:else}
  {#if error}<p class="inspector-group-empty" role="status">{error}</p>{/if}
  <div class="profile-toolbar">
    <span class="preset-label">{profilePresetLabel(draft.presetOrigin)}</span>
    <button class="quiet-button" type="button" onclick={() => void removeProfile()}>Remove Profile</button>
  </div>
  {#if draft.allocation}
    <div class="component-head">
      <label>
        <span>Point pool</span>
        <input
          type="number"
          min="0"
          value={draft.allocation.pool}
          onchange={(event) => {
            const next = parseOptionalNumber(event.currentTarget.value);
            if (Number.isNaN(next) || next === null) return;
            void persistDraft({ ...draft, allocation: { pool: next } });
          }} />
      </label>
      <p class="inspector-group-empty" class:over-budget={remaining !== null && remaining < 0}>
        Spent {allocationSpent(draft)}
        {#if remaining !== null}· Remaining {remaining}{/if}
      </p>
    </div>
    <button
      class="quiet-button"
      type="button"
      onclick={() => {
        void persistDraft({
          schemaVersion: draft.schemaVersion,
          ...(draft.presetOrigin ? { presetOrigin: draft.presetOrigin } : {}),
          components: draft.components,
        });
      }}>Remove point pool</button>
  {:else}
    <button
      class="quiet-button"
      type="button"
      onclick={() => void persistDraft({ ...draft, allocation: { pool: allocationSpent(draft) } })}
      >Add point pool</button>
  {/if}
  {#if draft.components.length === 0}
    <p class="inspector-group-empty">No components yet. Add one to start this profile.</p>
  {/if}
  {#each draft.components as component, index (component.id)}
    <article class="component">
      <div class="component-head">
        <label>
          <span>Kind</span>
          <select
            value={component.kind}
            onchange={(event) =>
              void patchComponent(component.id, { kind: event.currentTarget.value as ProfileComponentKind })}>
            {#each PROFILE_COMPONENT_KINDS as kind}
              <option value={kind}>{kindLabel(kind)}</option>
            {/each}
          </select>
        </label>
        <label>
          <span>Name</span>
          <input
            value={component.name}
            onchange={(event) => void patchComponent(component.id, { name: event.currentTarget.value })} />
        </label>
      </div>
      <div class="component-head">
        <label>
          <span>Value type</span>
          <select
            value={component.value.type}
            onchange={(event) =>
              void patchComponent(component.id, { value: emptyValue(event.currentTarget.value as ProfileValueType) })}>
            {#each PROFILE_VALUE_TYPES as type}
              <option value={type}>{type}</option>
            {/each}
          </select>
        </label>
        {#if component.value.type === "number"}
          <label>
            <span>Value</span>
            <input
              type="number"
              min={component.min}
              max={component.max}
              value={component.value.value ?? ""}
              onchange={(event) => {
                const next = parseOptionalNumber(event.currentTarget.value);
                if (Number.isNaN(next)) return;
                void patchComponent(component.id, { value: { type: "number", value: next } });
              }} />
          </label>
        {:else if component.value.type === "boolean"}
          <label>
            <span>Value</span>
            <select
              value={component.value.value === null ? "" : component.value.value ? "yes" : "no"}
              onchange={(event) => {
                const next = event.currentTarget.value;
                void patchComponent(component.id, {
                  value: { type: "boolean", value: next === "" ? null : next === "yes" },
                });
              }}>
              <option value="">—</option>
              <option value="yes">Yes</option>
              <option value="no">No</option>
            </select>
          </label>
        {:else if component.value.type === "resource"}
          <label>
            <span>Current</span>
            <input
              type="number"
              value={component.value.current ?? ""}
              onchange={(event) => {
                const next = parseOptionalNumber(event.currentTarget.value);
                if (Number.isNaN(next) || component.value.type !== "resource") return;
                void patchComponent(component.id, { value: { ...component.value, current: next } });
              }} />
          </label>
        {:else if component.value.type === "rank" || component.value.type === "enum"}
          {#if (component.scale?.length ?? 0) > 0}
            <label>
              <span>Value</span>
              <select
                value={component.value.value ?? ""}
                onchange={(event) => {
                  if (component.value.type !== "rank" && component.value.type !== "enum") return;
                  void patchComponent(component.id, {
                    value: { type: component.value.type, value: event.currentTarget.value || null },
                  });
                }}>
                <option value="">—</option>
                {#each component.scale ?? [] as option}
                  <option value={option}>{option}</option>
                {/each}
              </select>
            </label>
          {:else}
            <p class="inspector-group-empty">Add a scale before choosing a value.</p>
          {/if}
        {:else if component.value.type === "text"}
          <label>
            <span>Value</span>
            <input
              value={component.value.value ?? ""}
              onchange={(event) => {
                void patchComponent(component.id, {
                  value: { type: "text", value: event.currentTarget.value || null },
                });
              }} />
          </label>
        {/if}
      </div>
      {#if component.value.type === "number"}
        <div class="component-head">
          <label>
            <span>Min</span>
            <input
              type="number"
              value={component.min ?? ""}
              onchange={(event) => {
                const next = parseOptionalNumber(event.currentTarget.value);
                if (Number.isNaN(next)) return;
                void patchComponent(component.id, { min: next ?? undefined });
              }} />
          </label>
          <label>
            <span>Max</span>
            <input
              type="number"
              value={component.max ?? ""}
              onchange={(event) => {
                const next = parseOptionalNumber(event.currentTarget.value);
                if (Number.isNaN(next)) return;
                void patchComponent(component.id, { max: next ?? undefined });
              }} />
          </label>
        </div>
      {/if}
      {#if component.value.type === "resource"}
        <div class="component-head">
          <label>
            <span>Max</span>
            <input
              type="number"
              value={component.value.max ?? ""}
              onchange={(event) => {
                const next = parseOptionalNumber(event.currentTarget.value);
                if (Number.isNaN(next) || component.value.type !== "resource") return;
                void patchComponent(component.id, { value: { ...component.value, max: next } });
              }} />
          </label>
          <label>
            <span>Unit</span>
            <input
              value={component.value.unit ?? ""}
              onchange={(event) => {
                if (component.value.type !== "resource") return;
                void patchComponent(component.id, {
                  value: { ...component.value, unit: event.currentTarget.value || null },
                });
              }} />
          </label>
        </div>
      {/if}
      {#if component.value.type === "rank" || component.value.type === "enum"}
        <label>
          <span>Scale (comma-separated)</span>
          <input
            value={(component.scale ?? []).join(", ")}
            onchange={(event) =>
              void patchComponent(component.id, {
                scale: event.currentTarget.value
                  .split(",")
                  .map((entry) => entry.trim())
                  .filter(Boolean),
              })} />
        </label>
      {/if}
      <div class="component-actions">
        <button class="quiet-button" type="button" disabled={index === 0} onclick={() => void moveComponent(index, -1)}
          >Up</button>
        <button
          class="quiet-button"
          type="button"
          disabled={index === draft.components.length - 1}
          onclick={() => void moveComponent(index, 1)}>Down</button>
        <button class="quiet-button" type="button" onclick={() => void removeComponent(component)}>Remove</button>
      </div>
    </article>
  {/each}
  <div class="add-row">
    <select bind:value={addingKind} aria-label="Component kind">
      {#each PROFILE_COMPONENT_KINDS as kind}
        <option value={kind}>{kindLabel(kind)}</option>
      {/each}
    </select>
    <button class="quiet-button" type="button" onclick={() => void addComponent()}>Add component</button>
  </div>
{/if}

<style>
.profile-toolbar,
.add-row,
.component-actions,
.component-head {
  display: flex;
  gap: 8px;
  align-items: end;
}
.profile-toolbar,
.add-row {
  margin: 8px 0;
}
.profile-toolbar {
  justify-content: space-between;
}
.preset-label {
  color: var(--ink-soft);
  font-size: 11px;
}
.over-budget {
  color: var(--danger, #b42318);
}
.component {
  display: grid;
  gap: 8px;
  margin-top: 12px;
  padding-top: 12px;
  border-top: 1px solid var(--line);
}
.component-head label,
.component label {
  flex: 1;
  min-width: 0;
}
.component span {
  display: block;
  margin-bottom: 5px;
  color: var(--ink-soft);
  font-size: 10px;
}
.component input,
.component select,
.add-row select {
  box-sizing: border-box;
  width: 100%;
  height: 32px;
  padding: 0 9px;
  border: 1px solid var(--line);
  border-radius: 7px;
  background: var(--canvas);
  color: var(--ink);
  font-size: 11px;
}
.component-actions {
  justify-content: flex-end;
}
</style>
