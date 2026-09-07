<script lang="ts">
import type { Component } from "svelte";
import type { ModuleManifest } from "../../../packages/module-api/src/index";
import { Check, Dices, Minus, Plus, Rocket, Sparkles, UserRound, X } from "@lucide/svelte";
import { tick, untrack } from "svelte";
import { confirmDialog } from "$lib/dialogs.svelte";
import { buildModuleContext } from "$lib/modules/context";
import { trapModalTab } from "$lib/shell/modalFocus";
import loreManifestJson from "../../../packages/modules/lore/manifest.json";
import {
  allocationRemaining,
  allocationSpent,
  emptyProfile,
  evaluateProfile,
  formatProfileValue,
  profileValidationErrors,
  withDerivedDependencies,
  type ProfileComponent,
  type ProfileDocument,
  type ProfilePresetOrigin,
} from "./profile.ts";
import {
  applyDndAncestry,
  dndAncestriesByLineage,
  matchingDndAncestry,
  PROFILE_PRESETS,
  profileFromPreset,
  profilePresetLabel,
} from "./profilePresets.ts";
import { createProfile, deleteProfile, loadProfile, saveProfile, type StoredProfile } from "./profileStore";

const PRESET_BLURBS: Record<ProfilePresetOrigin, string> = {
  custom: "Blank sheet. Field schema comes later.",
  dnd: "Six abilities, skills, and combat basics.",
  fantasy: "Eight attributes with a 40-point pool.",
  scifi: "Technical scores, skills, and resources.",
};

const PRESET_ICONS: Record<ProfilePresetOrigin, Component> = {
  custom: UserRound,
  dnd: Dices,
  fantasy: Sparkles,
  scifi: Rocket,
};

const SHORT_NAMES: Record<string, string> = {
  Strength: "STR",
  Dexterity: "DEX",
  Constitution: "CON",
  Intelligence: "INT",
  Wisdom: "WIS",
  Charisma: "CHA",
  Agility: "AGI",
  Endurance: "END",
  Willpower: "WIL",
  Perception: "PER",
  Presence: "PRE",
  Magic: "MAG",
  Awareness: "AWA",
  "Technical Aptitude": "TEC",
};

type SheetLayout = "identity" | "abilities" | "stats" | "vitals" | "chips" | "skills" | "ranks" | "fields";
type SheetSection = { id: string; title: string; layout: SheetLayout; items: ProfileComponent[] };
type SheetTab = { id: string; label: string; sections: SheetSection[] };

let {
  projectId,
  entityId,
  entityName,
  open = $bindable(false),
}: {
  projectId: string;
  entityId: string;
  entityName: string;
  open?: boolean;
} = $props();

const context = $derived(buildModuleContext(loreManifestJson as unknown as ModuleManifest, projectId));

let stored = $state<StoredProfile | null>(null);
let draft = $state<ProfileDocument>(emptyProfile());
let loading = $state(true);
let creating = $state(false);
let removing = $state(false);
let error = $state("");
let selectedPreset = $state<ProfilePresetOrigin>("custom");
let activeTab = $state("scores");
let writeChain = Promise.resolve();
let persistGeneration = 0;
let dialogEl = $state<HTMLElement | null>(null);
let sheetEl = $state<HTMLElement | null>(null);
const remaining = $derived(allocationRemaining(draft));
const evaluated = $derived(evaluateProfile(draft));
const spent = $derived(allocationSpent(draft));
const pool = $derived(draft.allocation?.pool);
const spentRatio = $derived(pool && pool > 0 ? Math.min(1, spent / pool) : 0);
const overBudget = $derived(remaining !== null && remaining < 0);
const modifierFor = $derived.by(() => {
  const map = new Map<string, ProfileComponent>();
  for (const component of draft.components) {
    if (component.kind !== "derived") continue;
    const deps = component.dependencies ?? [];
    if (deps.length !== 1) continue;
    const source = draft.components.find((candidate) => candidate.id === deps[0] && candidate.kind === "attribute");
    if (source) map.set(source.id, component);
  }
  return map;
});
const pairedIds = $derived(new Set([...modifierFor.values()].map((component) => component.id)));
const tabs = $derived.by(() => {
  const unused = draft.components.filter((component) => !pairedIds.has(component.id));
  const of = (kind: ProfileComponent["kind"]) => unused.filter((component) => component.kind === kind);
  const attributes = of("attribute");
  const abilities = attributes.filter(
    (component) => modifierFor.has(component.id) || (component.min !== undefined && component.max !== undefined),
  );
  const stats = attributes.filter((component) => !abilities.includes(component));
  const resources = of("resource");
  const vitals = resources.length <= 2 ? resources : [];
  const restResources = resources.length > 2 ? resources : [];
  const built: SheetTab[] = [];
  const scores: SheetSection[] = (
    [
      { id: "identity", title: "Identity", layout: "identity", items: of("tag") },
      { id: "abilities", title: "Abilities", layout: "abilities", items: abilities },
      { id: "stats", title: "Combat", layout: "stats", items: stats },
      { id: "vitals", title: "Vitals", layout: "vitals", items: vitals },
      { id: "conditions", title: "Flags", layout: "chips", items: of("condition") },
    ] satisfies SheetSection[]
  ).filter((section) => section.items.length > 0);
  if (scores.length) built.push({ id: "scores", label: "Scores", sections: scores });
  const extra: [string, string, SheetLayout, ProfileComponent[]][] = [
    ["skills", "Skills", "skills", of("skill")],
    ["proficiencies", "Proficiencies", "ranks", of("proficiency")],
    ["resources", "Resources", "vitals", restResources],
    ["traits", "Traits", "fields", of("trait")],
    ["derived", "Derived", "fields", of("derived")],
  ];
  for (const [id, label, layout, items] of extra) {
    if (!items.length) continue;
    built.push({ id, label, sections: [{ id, title: label, layout, items }] });
  }
  return built;
});
const currentTab = $derived(tabs.find((tab) => tab.id === activeTab) ?? tabs[0]);
const preview = $derived(
  draft.components
    .filter(
      (component) =>
        modifierFor.has(component.id) && component.value.type === "number" && component.value.value != null,
    )
    .slice(0, 4)
    .map(
      (component) => `${shortName(component.name)} ${component.value.type === "number" ? component.value.value : ""}`,
    )
    .join(" · "),
);

$effect(() => {
  const id = entityId;
  const project = projectId;
  let cancelled = false;
  persistGeneration += 1;
  writeChain = Promise.resolve();
  loading = true;
  creating = false;
  removing = false;
  error = "";
  activeTab = "scores";
  const moduleContext = untrack(() => buildModuleContext(loreManifestJson as unknown as ModuleManifest, project));
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

$effect(() => {
  if (!open) return;
  dialogEl?.focus();
});

let lastEntityId = $state<string | null>(null);
$effect(() => {
  if (lastEntityId === null) {
    lastEntityId = entityId;
    return;
  }
  if (entityId === lastEntityId) return;
  lastEntityId = entityId;
  open = false;
});

$effect(() => {
  if (!tabs.some((tab) => tab.id === activeTab) && tabs[0]) activeTab = tabs[0].id;
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
  next = { ...next, components: next.components.map(withDerivedDependencies) };
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
  creating = true;
  try {
    stored = await createProfile(context, entityId, profileFromPreset(selectedPreset));
    draft = structuredClone(stored.value);
    activeTab = "scores";
    await tick();
    const first = sheetEl?.querySelector<HTMLElement>("input, select, button.nudge");
    first?.focus();
  } catch (cause) {
    error = cause instanceof Error ? cause.message : String(cause);
  } finally {
    creating = false;
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
  removing = true;
  try {
    await deleteProfile(context, entityId, stored);
    stored = null;
    draft = emptyProfile();
    close();
  } catch (cause) {
    error = cause instanceof Error ? cause.message : String(cause);
  } finally {
    removing = false;
  }
}

async function patchComponent(id: string, patch: Partial<ProfileComponent>) {
  await persistDraft({
    ...draft,
    components: draft.components.map((component) => {
      if (component.id !== id) return component;
      const next = { ...component, ...patch };
      if ("min" in patch && patch.min === undefined) delete next.min;
      if ("max" in patch && patch.max === undefined) delete next.max;
      if ("override" in patch && patch.override === undefined) delete next.override;
      if ("decimals" in patch && patch.decimals === undefined) delete next.decimals;
      if ("unit" in patch && patch.unit === undefined) delete next.unit;
      delete next.dependencies;
      return withDerivedDependencies(next);
    }),
  });
}

function close() {
  open = false;
}

function isBackdropTarget(event: Event): boolean {
  return event.target === event.currentTarget;
}

function onDialogKeydown(event: KeyboardEvent) {
  if (event.key === "Escape") {
    event.preventDefault();
    close();
    return;
  }
  trapModalTab(event, dialogEl);
}

function shortName(name: string): string {
  return SHORT_NAMES[name] ?? name.slice(0, 3).toUpperCase();
}

function modifierText(component: ProfileComponent): string {
  const paired = modifierFor.get(component.id);
  if (!paired) return "";
  const text = formatProfileValue(paired, evaluated.get(paired.id) ?? null);
  if (!text) return "—";
  const value = Number(text);
  if (!Number.isFinite(value)) return text;
  return value > 0 ? `+${value}` : value === 0 ? "+0" : String(value);
}

function resourceRatio(component: ProfileComponent): number | null {
  if (component.value.type !== "resource") return null;
  const { current, max } = component.value;
  if (current === null || max === null || max <= 0) return null;
  return Math.max(0, Math.min(1, current / max));
}

function displayNumber(value: number | null): string {
  return value === null ? "" : String(value);
}

async function setNumber(component: ProfileComponent, raw: string) {
  const next = parseOptionalNumber(raw);
  if (Number.isNaN(next)) return;
  await patchComponent(component.id, { value: { type: "number", value: next } });
}

async function nudge(component: ProfileComponent, delta: number) {
  if (component.value.type !== "number") return;
  const fallback = delta > 0 ? (component.min ?? 0) : (component.max ?? 0);
  let next = (component.value.value ?? fallback) + delta;
  if (component.min !== undefined) next = Math.max(component.min, next);
  if (component.max !== undefined) next = Math.min(component.max, next);
  await patchComponent(component.id, { value: { type: "number", value: next } });
}

async function toggleFlag(component: ProfileComponent) {
  if (component.value.type !== "boolean") return;
  await patchComponent(component.id, {
    value: { type: "boolean", value: component.value.value === true ? null : true },
  });
}
</script>

{#snippet numberInput(component: ProfileComponent, size: "ability" | "stat" | "skill")}
  <input
    class="num {size}"
    type="text"
    inputmode="numeric"
    placeholder="—"
    aria-label={component.name}
    value={component.value.type === "number" ? displayNumber(component.value.value) : ""}
    onchange={(event) => void setNumber(component, event.currentTarget.value)} />
{/snippet}

{#snippet textInput(component: ProfileComponent)}
  <input
    type="text"
    placeholder="—"
    aria-label={component.name}
    value={component.value.type === "text" ? (component.value.value ?? "") : ""}
    onchange={(event) => {
      void patchComponent(component.id, { value: { type: "text", value: event.currentTarget.value || null } });
    }} />
{/snippet}

{#snippet rankInput(component: ProfileComponent)}
  <select
    aria-label={component.name}
    value={component.value.type === "rank" || component.value.type === "enum" ? (component.value.value ?? "") : ""}
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
{/snippet}

{#snippet vital(component: ProfileComponent)}
  {@const ratio = resourceRatio(component)}
  <div class="vital">
    <div class="vital-head">
      <span>{component.name}</span>
      <div class="vital-inputs">
        <input
          class="num stat"
          type="text"
          inputmode="numeric"
          placeholder="—"
          aria-label="{component.name} current"
          value={component.value.type === "resource" ? displayNumber(component.value.current) : ""}
          onchange={(event) => {
            const next = parseOptionalNumber(event.currentTarget.value);
            if (Number.isNaN(next) || component.value.type !== "resource") return;
            void patchComponent(component.id, { value: { ...component.value, current: next } });
          }} />
        <span aria-hidden="true">/</span>
        <input
          class="num stat"
          type="text"
          inputmode="numeric"
          placeholder="—"
          aria-label="{component.name} max"
          value={component.value.type === "resource" ? displayNumber(component.value.max) : ""}
          onchange={(event) => {
            const next = parseOptionalNumber(event.currentTarget.value);
            if (Number.isNaN(next) || component.value.type !== "resource") return;
            void patchComponent(component.id, { value: { ...component.value, max: next } });
          }} />
      </div>
    </div>
    <div
      class="vital-track"
      class:empty={ratio === null}
      role="progressbar"
      aria-label="{component.name} remaining"
      aria-valuemin={0}
      aria-valuemax={component.value.type === "resource" ? (component.value.max ?? 0) : 0}
      aria-valuenow={component.value.type === "resource" ? (component.value.current ?? 0) : 0}>
      <i style="width: {(ratio ?? 0) * 100}%"></i>
    </div>
  </div>
{/snippet}

{#if loading}
  <p class="inspector-copy">Loading Profile…</p>
{:else if stored}
  <p class="inspector-copy">
    {profilePresetLabel(draft.presetOrigin)}
    {#if remaining !== null}
      · {overBudget ? "Over budget" : `${remaining} pts left`}
    {/if}
  </p>
  {#if preview}<p class="inspector-preview">{preview}</p>{/if}
  <button class="primary-button inspector-open" type="button" onclick={() => (open = true)}>Edit Profile</button>
{:else}
  {#if error}<p class="inspector-copy" role="alert">{error}</p>{/if}
  <p class="inspector-copy">Optional attributes, skills, and traits.</p>
  <button class="primary-button inspector-open" type="button" onclick={() => (open = true)}>Add Profile</button>
{/if}

{#if open}
  <div class="overlay" role="presentation">
    <button
      type="button"
      class="backdrop"
      tabindex="-1"
      aria-label="Close Profile"
      onclick={(event) => {
        if (isBackdropTarget(event)) close();
      }}></button>
    <div
      class="dialog"
      class:create={!stored}
      role="dialog"
      aria-modal="true"
      aria-labelledby="profile-editor-title"
      aria-describedby={error ? "profile-editor-error" : undefined}
      tabindex="-1"
      bind:this={dialogEl}
      onkeydown={onDialogKeydown}>
      <header class="dialog-heading">
        <div>
          <span class="panel-kicker">PROFILE</span>
          <h2 id="profile-editor-title">{entityName}</h2>
        </div>
        <button type="button" class="dialog-close" aria-label="Close Profile" onclick={close}
          ><X size={16} strokeWidth={1.8} aria-hidden="true" /></button>
      </header>
      {#if error}
        <p id="profile-editor-error" class="banner" role="alert">{error}</p>
      {/if}
      {#if !stored}
        <div class="dialog-body" bind:this={sheetEl}>
          <p class="lead">Choose a starting sheet. Values can be edited after you create it.</p>
          <div class="preset-grid" role="group" aria-label="Profile presets">
            {#each PROFILE_PRESETS as preset}
              {@const Icon = PRESET_ICONS[preset.id]}
              <button
                type="button"
                class="preset-card"
                class:selected={selectedPreset === preset.id}
                aria-pressed={selectedPreset === preset.id}
                disabled={creating}
                onclick={() => (selectedPreset = preset.id)}>
                <span class="preset-icon" aria-hidden="true"><Icon size={18} strokeWidth={1.8} /></span>
                <strong>{preset.name}</strong>
                <span>{PRESET_BLURBS[preset.id]}</span>
                {#if selectedPreset === preset.id}
                  <span class="preset-check" aria-hidden="true"><Check size={14} strokeWidth={2.2} /></span>
                {/if}
              </button>
            {/each}
          </div>
        </div>
        <footer class="dialog-footer">
          <button class="quiet-button" type="button" disabled={creating} onclick={close}>Cancel</button>
          <button class="primary-button" type="button" disabled={creating} onclick={() => void addProfile()}>
            {creating ? "Creating…" : "Create Profile"}
          </button>
        </footer>
      {:else}
        <div class="meta">
          <span class="chip">{profilePresetLabel(draft.presetOrigin)}</span>
          {#if remaining !== null && pool !== undefined}
            <div class="pool" class:over={overBudget}>
              <div
                class="pool-track"
                role="progressbar"
                aria-label="Point pool"
                aria-valuemin={0}
                aria-valuemax={pool}
                aria-valuenow={spent}
                aria-valuetext="{spent} of {pool} points spent, {remaining} remaining">
                <i style="width: {spentRatio * 100}%"></i>
              </div>
              <span>{spent} / {pool} · {overBudget ? `${Math.abs(remaining)} over` : `${remaining} left`}</span>
            </div>
          {/if}
        </div>
        {#if tabs.length > 1}
          <div class="tabs" role="tablist" aria-label="Profile sections">
            {#each tabs as tab (tab.id)}
              <button
                type="button"
                role="tab"
                id="profile-tab-{tab.id}"
                aria-selected={currentTab?.id === tab.id}
                aria-controls="profile-panel-{tab.id}"
                tabindex={currentTab?.id === tab.id ? 0 : -1}
                onclick={() => (activeTab = tab.id)}>{tab.label}</button>
            {/each}
          </div>
        {/if}
        <div
          class="dialog-body sheet"
          bind:this={sheetEl}
          role="tabpanel"
          id="profile-panel-{currentTab?.id}"
          aria-labelledby="profile-tab-{currentTab?.id}">
          {#if !currentTab}
            <p class="lead">This profile has no fields yet.</p>
          {:else}
            {#each currentTab.sections as section (section.id)}
              <section class="group">
                {#if currentTab.sections.length > 1}
                  <h3>{section.title}</h3>
                {/if}
                {#if section.layout === "identity"}
                  <div class="identity-grid">
                    {#if draft.presetOrigin === "dnd"}
                      <label class="stat">
                        <span>Ancestry</span>
                        <select
                          aria-label="Ancestry"
                          value={matchingDndAncestry(draft)?.id ?? ""}
                          onchange={(event) => void persistDraft(applyDndAncestry(draft, event.currentTarget.value))}>
                          <option value="">Custom</option>
                          {#each dndAncestriesByLineage() as group}
                            <optgroup label={group.lineage}>
                              {#each group.ancestries as ancestry}
                                <option value={ancestry.id}>{ancestry.name}</option>
                              {/each}
                            </optgroup>
                          {/each}
                        </select>
                      </label>
                    {/if}
                    {#each section.items as component (component.id)}
                      {#if !(draft.presetOrigin === "dnd" && component.id === "tag-species" && matchingDndAncestry(draft))}
                        <label class="stat">
                          <span>{component.name}</span>
                          {@render textInput(component)}
                        </label>
                      {/if}
                    {/each}
                  </div>
                {:else if section.layout === "abilities"}
                  <div class="ability-grid" class:six={section.items.length === 6}>
                    {#each section.items as component (component.id)}
                      {@const paired = modifierFor.get(component.id)}
                      <div class="ability">
                        <span class="ability-abbr" title={component.name}>{shortName(component.name)}</span>
                        <span class="ability-name">{component.name}</span>
                        <div class="ability-value">
                          {#if component.value.type === "number"}
                            <button
                              type="button"
                              class="nudge"
                              aria-label="Decrease {component.name}"
                              onclick={() => void nudge(component, -1)}
                              ><Minus size={12} strokeWidth={2} aria-hidden="true" /></button>
                            {@render numberInput(component, "ability")}
                            <button
                              type="button"
                              class="nudge"
                              aria-label="Increase {component.name}"
                              onclick={() => void nudge(component, 1)}
                              ><Plus size={12} strokeWidth={2} aria-hidden="true" /></button>
                          {:else}
                            {@render textInput(component)}
                          {/if}
                        </div>
                        {#if paired}
                          <span class="ability-mod" title={paired.name}>{modifierText(component)}</span>
                        {/if}
                      </div>
                    {/each}
                  </div>
                {:else if section.layout === "stats"}
                  <div class="stat-grid">
                    {#each section.items as component (component.id)}
                      <label class="stat">
                        <span>{component.name}</span>
                        {#if component.value.type === "number"}
                          {@render numberInput(component, "stat")}
                        {:else}
                          {@render textInput(component)}
                        {/if}
                      </label>
                    {/each}
                  </div>
                {:else if section.layout === "vitals"}
                  <div class="vital-list">
                    {#each section.items as component (component.id)}
                      {@render vital(component)}
                    {/each}
                  </div>
                {:else if section.layout === "chips"}
                  <div class="flags">
                    {#each section.items as component (component.id)}
                      {#if component.value.type === "boolean"}
                        <button
                          type="button"
                          class="flag"
                          aria-pressed={component.value.value === true}
                          onclick={() => void toggleFlag(component)}>{component.name}</button>
                      {:else}
                        <label class="stat">
                          <span>{component.name}</span>
                          {@render textInput(component)}
                        </label>
                      {/if}
                    {/each}
                  </div>
                {:else if section.layout === "skills"}
                  <div class="skill-grid">
                    {#each section.items as component (component.id)}
                      <label class="skill">
                        <span>{component.name}</span>
                        {#if component.value.type === "number"}
                          {@render numberInput(component, "skill")}
                        {:else if component.value.type === "rank" || component.value.type === "enum"}
                          {@render rankInput(component)}
                        {:else}
                          {@render textInput(component)}
                        {/if}
                      </label>
                    {/each}
                  </div>
                {:else if section.layout === "ranks"}
                  <div class="rank-grid">
                    {#each section.items as component (component.id)}
                      <label class="stat">
                        <span>{component.name}</span>
                        {@render rankInput(component)}
                      </label>
                    {/each}
                  </div>
                {:else}
                  <div class="field-list">
                    {#each section.items as component (component.id)}
                      <label class="skill">
                        <span>{component.name}</span>
                        {#if component.kind === "derived"}
                          <span class="computed"
                            >{formatProfileValue(component, evaluated.get(component.id) ?? null) || "—"}</span>
                        {:else if component.value.type === "number"}
                          {@render numberInput(component, "stat")}
                        {:else}
                          {@render textInput(component)}
                        {/if}
                      </label>
                    {/each}
                  </div>
                {/if}
              </section>
            {/each}
          {/if}
        </div>
        <footer class="dialog-footer">
          <button class="danger-button" type="button" disabled={removing} onclick={() => void removeProfile()}
            >{removing ? "Removing…" : "Remove"}</button>
          <button class="primary-button" type="button" onclick={close}>Done</button>
        </footer>
      {/if}
    </div>
  </div>
{/if}

<style>
.inspector-copy,
.inspector-preview {
  margin: 0 0 8px;
  color: var(--ink-soft);
  font-size: 11px;
  line-height: 1.45;
}
.inspector-preview {
  color: var(--ink);
  font-variant-numeric: tabular-nums;
}
.inspector-open {
  width: 100%;
}
.overlay {
  position: fixed;
  inset: 0;
  z-index: 300;
  display: grid;
  place-items: center;
  padding: 20px;
}
.backdrop {
  position: absolute;
  inset: 0;
  border: 0;
  padding: 0;
  background: rgba(14, 23, 20, 0.62);
  cursor: default;
}
.dialog {
  position: relative;
  z-index: 1;
  display: flex;
  flex-direction: column;
  width: min(860px, 100%);
  max-height: min(88vh, 880px);
  overflow: hidden;
  border: 1px solid var(--line-strong);
  border-radius: 16px;
  background: var(--surface);
  box-shadow: var(--shadow-lg, 0 22px 70px rgba(0, 0, 0, 0.35));
  outline: none;
}
.dialog.create {
  width: min(560px, 100%);
}
.dialog-heading,
.dialog-footer,
.meta,
.tabs {
  flex-shrink: 0;
}
.dialog-heading,
.dialog-footer {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 12px;
  padding: 18px 22px 12px;
}
.dialog-footer {
  align-items: center;
  justify-content: flex-end;
  padding: 12px 22px 16px;
  border-top: 1px solid var(--line);
}
.dialog-footer .danger-button {
  margin-right: auto;
}
.panel-kicker {
  display: block;
  color: var(--accent);
  font-size: 9px;
  font-weight: 800;
  letter-spacing: 0.14em;
}
.dialog-heading h2 {
  margin: 4px 0 0;
  color: var(--ink);
  font: 500 26px/1.15 var(--font-display);
}
.dialog-close {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 32px;
  height: 32px;
  flex: none;
  border: 0;
  border-radius: 8px;
  background: var(--surface-muted);
  color: var(--ink-soft);
  cursor: pointer;
}
.dialog-close:hover {
  color: var(--ink);
  background: var(--line-soft);
}
.dialog-body {
  display: grid;
  gap: 18px;
  min-height: 0;
  overflow: auto;
  padding: 6px 22px 20px;
}
.lead {
  margin: 0;
  color: var(--ink-muted);
  font-size: 13px;
  line-height: 1.5;
}
.banner {
  margin: 0 22px 8px;
  padding: 8px 10px;
  border: 1px solid var(--danger-line);
  border-radius: 8px;
  background: var(--danger-bg);
  color: var(--theme-danger-text, var(--danger));
  font-size: 12px;
}
.preset-grid {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 10px;
}
.preset-card {
  position: relative;
  display: grid;
  gap: 6px;
  min-height: 108px;
  padding: 14px 16px 14px 44px;
  border: 1px solid var(--line-strong);
  border-radius: 12px;
  background: var(--canvas);
  color: var(--ink);
  text-align: left;
  cursor: pointer;
}
.preset-card:hover {
  border-color: var(--accent);
}
.preset-icon {
  position: absolute;
  top: 14px;
  left: 14px;
  color: var(--ink-soft);
}
.preset-card strong {
  font-size: 14px;
}
.preset-card > span {
  color: var(--ink-muted);
  font-size: 12px;
  line-height: 1.4;
}
.preset-card.selected,
.preset-card[aria-pressed="true"] {
  border-color: var(--accent);
  background: color-mix(in srgb, var(--accent) 12%, var(--surface));
}
.preset-check {
  position: absolute;
  top: 12px;
  right: 12px;
  display: grid;
  width: 22px;
  height: 22px;
  place-items: center;
  border-radius: 99px;
  background: var(--accent-dark);
  color: var(--on-accent);
}
.meta {
  display: flex;
  flex-wrap: wrap;
  gap: 10px 16px;
  align-items: center;
  padding: 0 22px 8px;
}
.chip {
  padding: 3px 8px;
  border-radius: 999px;
  background: var(--surface-muted);
  color: var(--ink-soft);
  font-size: 10px;
  font-weight: 700;
  letter-spacing: 0.08em;
  text-transform: uppercase;
}
.pool {
  display: grid;
  flex: 1;
  gap: 4px;
  min-width: 160px;
  color: var(--ink-muted);
  font-size: 11px;
}
.pool.over {
  color: var(--danger);
}
.pool-track,
.vital-track {
  overflow: hidden;
  height: 6px;
  border-radius: 99px;
  background: var(--surface-muted);
}
.pool-track i,
.vital-track i {
  display: block;
  height: 100%;
  background: var(--accent);
}
.pool.over .pool-track i {
  background: var(--danger);
}
.vital-track.empty i {
  width: 0;
}
.tabs {
  display: flex;
  gap: 6px;
  padding: 0 22px 10px;
}
.tabs button {
  min-height: 30px;
  padding: 0 12px;
  border: 0;
  border-radius: 999px;
  background: transparent;
  color: var(--ink-soft);
  font-size: 12px;
  font-weight: 650;
  cursor: pointer;
}
.tabs button[aria-selected="true"] {
  background: var(--surface-muted);
  color: var(--ink);
  box-shadow: inset 0 0 0 1px var(--line-strong);
}
.tabs button:hover {
  color: var(--ink);
}
.group h3 {
  margin: 0 0 10px;
  color: var(--ink-faint);
  font-size: 10px;
  font-weight: 700;
  letter-spacing: 0.1em;
  text-transform: uppercase;
}
.ability-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(112px, 1fr));
  gap: 10px;
}
.ability-grid.six {
  grid-template-columns: repeat(6, minmax(0, 1fr));
}
.ability {
  display: grid;
  justify-items: center;
  gap: 4px;
  padding: 12px 8px 10px;
  border: 1px solid var(--line);
  border-radius: 14px;
  background: var(--canvas);
}
.ability-abbr {
  color: var(--accent);
  font-size: 11px;
  font-weight: 800;
  letter-spacing: 0.12em;
}
.ability-name {
  overflow: hidden;
  max-width: 100%;
  color: var(--ink-faint);
  font-size: 9px;
  letter-spacing: 0.04em;
  text-overflow: ellipsis;
  text-transform: uppercase;
  white-space: nowrap;
}
.ability-value {
  display: flex;
  align-items: center;
  gap: 2px;
}
.nudge {
  display: grid;
  width: 22px;
  height: 28px;
  place-items: center;
  border: 0;
  border-radius: 6px;
  background: transparent;
  color: var(--ink-faint);
  cursor: pointer;
}
.nudge:hover {
  color: var(--ink);
  background: var(--surface-muted);
}
.ability-mod {
  min-width: 28px;
  padding: 1px 7px;
  border-radius: 999px;
  background: var(--surface-muted);
  color: var(--ink);
  font-size: 12px;
  font-weight: 700;
  font-variant-numeric: tabular-nums;
  text-align: center;
}
.identity-grid,
.stat-grid,
.rank-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(128px, 1fr));
  gap: 10px;
}
.identity-grid {
  grid-template-columns: repeat(auto-fill, minmax(160px, 1fr));
}
.stat,
.skill {
  display: grid;
  gap: 5px;
}
.stat > span,
.skill > span {
  color: var(--ink-muted);
  font-size: 10px;
  font-weight: 700;
  letter-spacing: 0.04em;
  text-transform: uppercase;
}
.skill-grid {
  display: grid;
  grid-template-columns: repeat(3, minmax(0, 1fr));
  gap: 2px 18px;
}
.skill {
  grid-template-columns: minmax(0, 1fr) 56px;
  align-items: center;
  gap: 8px;
  min-height: 34px;
  padding: 4px 0;
  border-bottom: 1px solid var(--line-soft);
}
.skill > span {
  text-transform: none;
  letter-spacing: 0;
  font-size: 13px;
  font-weight: 500;
  color: var(--ink);
}
.vital-list {
  display: grid;
  gap: 10px;
}
.vital {
  display: grid;
  gap: 8px;
  padding: 12px 14px;
  border: 1px solid var(--line);
  border-radius: 12px;
  background: var(--canvas);
}
.vital-head {
  display: flex;
  gap: 12px;
  align-items: center;
  justify-content: space-between;
}
.vital-head > span {
  color: var(--ink);
  font-size: 12px;
  font-weight: 700;
  letter-spacing: 0.06em;
  text-transform: uppercase;
}
.vital-inputs {
  display: flex;
  gap: 6px;
  align-items: center;
}
.vital-inputs span {
  color: var(--ink-faint);
}
.flags {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
}
.flag {
  min-height: 32px;
  padding: 0 12px;
  border: 1px solid var(--line-strong);
  border-radius: 999px;
  background: var(--surface);
  color: var(--ink-soft);
  font-size: 12px;
  font-weight: 650;
  cursor: pointer;
}
.flag[aria-pressed="true"] {
  border-color: var(--accent);
  background: color-mix(in srgb, var(--accent) 16%, var(--surface));
  color: var(--ink);
}
.field-list {
  display: grid;
  gap: 8px;
}
:global(.num),
.stat :global(input),
.stat :global(select),
.skill :global(input),
.skill :global(select),
.computed {
  box-sizing: border-box;
  width: 100%;
  height: 34px;
  padding: 0 8px;
  border: 1px solid var(--line);
  border-radius: 8px;
  background: var(--surface-muted);
  color: var(--ink);
  font-size: 13px;
}
:global(.num) {
  appearance: textfield;
  font-variant-numeric: tabular-nums;
  text-align: center;
}
:global(.num.ability) {
  width: 54px;
  height: 40px;
  border: 0;
  background: transparent;
  font: 600 26px/1 var(--font-display);
}
:global(.num.skill) {
  width: 56px;
  height: 28px;
  padding: 0;
  font-size: 13px;
}
.computed {
  display: grid;
  place-items: center end;
  border: 0;
  background: transparent;
  color: var(--ink-soft);
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
  font-size: 12px;
  font-weight: 600;
  cursor: pointer;
}
.quiet-button {
  background: var(--surface);
  color: var(--ink-soft);
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
.dialog-close:focus-visible,
.quiet-button:focus-visible,
.primary-button:focus-visible,
.danger-button:focus-visible,
.preset-card:focus-visible,
.tabs button:focus-visible,
.nudge:focus-visible,
.flag:focus-visible,
:global(.num:focus-visible),
.stat :global(input:focus-visible),
.stat :global(select:focus-visible),
.skill :global(input:focus-visible),
.skill :global(select:focus-visible) {
  outline: 2px solid var(--accent);
  outline-offset: 1px;
}
.quiet-button:disabled,
.primary-button:disabled,
.danger-button:disabled,
.preset-card:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}
@media (max-width: 760px) {
  .ability-grid.six,
  .skill-grid,
  .preset-grid {
    grid-template-columns: repeat(2, minmax(0, 1fr));
  }
}
@media (prefers-reduced-motion: reduce) {
  .dialog-body {
    scroll-behavior: auto;
  }
}
</style>
