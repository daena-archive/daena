<script lang="ts">
import type { ModuleManifest, UUID } from "../../../packages/module-api/src/index";
import type { AsyncEntitySearchFn } from "$lib/entity-lifecycle/asyncEntityQuery.ts";
import { parseCalendarDate } from "$lib/date";
import { project } from "$lib/project/client";
import { buildModuleContext } from "$lib/modules/context";
import AsyncEntityPicker from "$lib/entity-lifecycle/AsyncEntityPicker.svelte";
import loreManifestJson from "../../../packages/modules/lore/manifest.json";
import timelineManifestJson from "../../../packages/modules/timeline/manifest.json";
import { formatProfileValue, profileValidationErrors, type ProfileComponent, type ProfileDocument } from "./profile.ts";
import { changesForEvent, foldProfile, PROFILE_CHANGE_RELATIONSHIP, type StoredProfileChange } from "./profileHistory";
import {
  deleteProfileChange,
  eventDatesForChanges,
  loadProfile,
  loadProfileChanges,
  upsertEventProfileChange,
  type StoredProfile,
} from "./profileStore";

type Row = {
  entityId: string;
  name: string;
  relationshipId: string;
  relationshipRevision: string;
  profile: StoredProfile | null;
  change: StoredProfileChange | null;
  draft: ProfileDocument | null;
  reason: string;
};

let {
  projectId,
  eventId,
  eventDate,
  search,
  ownerTypes,
}: {
  projectId: string;
  eventId: string;
  eventDate: unknown;
  search: AsyncEntitySearchFn;
  ownerTypes: string[];
} = $props();

const lore = $derived(buildModuleContext(loreManifestJson as unknown as ModuleManifest, projectId));
const timeline = $derived(buildModuleContext(timelineManifestJson as unknown as ModuleManifest, projectId));
const dated = $derived(Boolean(parseCalendarDate(eventDate)));

let rows = $state<Row[]>([]);
let loading = $state(true);
let error = $state("");
let saving = $state(false);

$effect(() => {
  const id = eventId;
  eventDate;
  let cancelled = false;
  loading = true;
  error = "";
  void loadRows(id)
    .then((next) => {
      if (!cancelled) rows = next;
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

async function loadRows(id: string): Promise<Row[]> {
  const rels = (await timeline.relationships.list(id as UUID)).filter(
    (relationship) => relationship.type === PROFILE_CHANGE_RELATIONSHIP && relationship.sourceId === id,
  );
  const next: Row[] = [];
  for (const relationship of rels) {
    const entity = await project.getEntity(relationship.targetId).catch(() => null);
    if (!entity || entity.deleted) continue;
    const profile = await loadProfile(lore, entity.id);
    const changes = profile ? await loadProfileChanges(lore, entity.id) : [];
    const change = changesForEvent(changes, id)[0] ?? null;
    const eventDates = await eventDatesForChanges(changes, { eventId: id, date: eventDate });
    const folded = profile && !profile.invalid ? foldProfile(profile.value, changes, eventDate, eventDates) : null;
    next.push({
      entityId: entity.id,
      name: entity.name,
      relationshipId: relationship.id,
      relationshipRevision: relationship.revision,
      profile,
      change,
      draft: folded,
      reason: change?.value.reason ?? "",
    });
  }
  return next;
}

async function addEntity(entityId: string) {
  if (rows.some((row) => row.entityId === entityId)) return;
  error = "";
  try {
    const entity = await project.getEntity(entityId);
    if (!entity || entity.deleted) throw new Error("That entity is gone.");
    const profile = await loadProfile(lore, entity.id);
    const changes = profile ? await loadProfileChanges(lore, entity.id) : [];
    const eventDates = await eventDatesForChanges(changes, { eventId, date: eventDate });
    const folded = profile && !profile.invalid ? foldProfile(profile.value, changes, eventDate, eventDates) : null;
    rows = [
      ...rows,
      {
        entityId: entity.id,
        name: entity.name,
        relationshipId: "",
        relationshipRevision: "",
        profile,
        change: changesForEvent(changes, eventId)[0] ?? null,
        draft: folded,
        reason: changesForEvent(changes, eventId)[0]?.value.reason ?? "",
      },
    ];
  } catch (cause) {
    error = cause instanceof Error ? cause.message : String(cause);
  }
}

async function removeRow(row: Row) {
  error = "";
  try {
    if (row.change) await deleteProfileChange(lore, row.entityId, row.change);
    if (row.relationshipId) {
      await timeline.relationships.delete(row.relationshipId as UUID, PROFILE_CHANGE_RELATIONSHIP, {
        expectedRevision: row.relationshipRevision,
      });
    }
    rows = rows.filter((candidate) => candidate.entityId !== row.entityId);
  } catch (cause) {
    error = cause instanceof Error ? cause.message : String(cause);
  }
}

async function saveRow(row: Row) {
  if (!row.profile || !row.draft || !dated) return;
  const errors = profileValidationErrors(row.draft);
  if (errors.length) {
    error = errors[0];
    return;
  }
  error = "";
  saving = true;
  try {
    const changes = await loadProfileChanges(lore, row.entityId);
    const eventDates = await eventDatesForChanges(changes, { eventId, date: eventDate });
    const nextChanges = await upsertEventProfileChange(
      lore,
      row.entityId,
      row.profile,
      row.draft,
      changes,
      eventDates,
      eventId,
      eventDate,
      row.reason,
    );
    const change = changesForEvent(nextChanges, eventId)[0];
    if (change && !row.relationshipId) {
      const source = await project.getEntity(eventId);
      if (!source?.revision) throw new Error("Reload the event and try again.");
      await timeline.relationships.create(
        {
          sourceId: eventId as UUID,
          targetId: row.entityId as UUID,
          type: PROFILE_CHANGE_RELATIONSHIP,
          metadata: {},
        },
        { expectedRevision: source.revision, requestId: crypto.randomUUID() },
      );
    } else if (!change && row.relationshipId) {
      await timeline.relationships.delete(row.relationshipId as UUID, PROFILE_CHANGE_RELATIONSHIP, {
        expectedRevision: row.relationshipRevision,
      });
    }
    rows = await loadRows(eventId);
  } catch (cause) {
    error = cause instanceof Error ? cause.message : String(cause);
  } finally {
    saving = false;
  }
}

function patchValue(row: Row, component: ProfileComponent, value: ProfileComponent["value"]) {
  if (!row.draft) return;
  row.draft = {
    ...row.draft,
    components: row.draft.components.map((candidate) =>
      candidate.id === component.id ? { ...candidate, value } : candidate,
    ),
  };
  rows = [...rows];
}

function parseNumber(raw: string): number | null {
  const trimmed = raw.trim();
  if (!trimmed) return null;
  const value = Number(trimmed);
  return Number.isFinite(value) ? value : null;
}
</script>

{#if loading}
  <p class="copy">Loading Profile changes…</p>
{:else}
  {#if error}<p class="copy" role="alert">{error}</p>{/if}
  {#if !dated}
    <p class="copy">Set Starts on this event to date Profile changes.</p>
  {/if}
  <AsyncEntityPicker
    {search}
    entityTypes={ownerTypes}
    excludeIds={rows.map((row) => row.entityId)}
    placeholder="Add an entity…"
    ariaLabel="Add entity Profile changes"
    resultsSectionLabel="Entities"
    onSelect={(entity) => void addEntity(entity.id)} />
  {#if rows.length === 0}
    <p class="copy">No Profile changes on this event.</p>
  {:else}
    <ul>
      {#each rows as row (row.entityId)}
        <li>
          <div class="head">
            <strong>{row.name}</strong>
            <button type="button" class="quiet" onclick={() => void removeRow(row)}>Remove</button>
          </div>
          {#if !row.profile}
            <p class="copy">This entity has no Profile.</p>
          {:else if row.profile.invalid}
            <p class="copy">{row.profile.error}</p>
          {:else if row.draft}
            <div class="fields">
              {#each row.draft.components.filter((component) => component.kind !== "derived") as component (component.id)}
                <label>
                  <span>{component.name}</span>
                  {#if component.value.type === "number"}
                    <input
                      type="text"
                      inputmode="numeric"
                      value={component.value.value ?? ""}
                      disabled={!dated || saving}
                      onchange={(event) =>
                        patchValue(row, component, {
                          type: "number",
                          value: parseNumber(event.currentTarget.value),
                        })} />
                  {:else if component.value.type === "boolean"}
                    <select
                      value={component.value.value === null ? "" : component.value.value ? "yes" : "no"}
                      disabled={!dated || saving}
                      onchange={(event) => {
                        const raw = event.currentTarget.value;
                        patchValue(row, component, {
                          type: "boolean",
                          value: raw === "" ? null : raw === "yes",
                        });
                      }}>
                      <option value="">—</option>
                      <option value="yes">Yes</option>
                      <option value="no">No</option>
                    </select>
                  {:else if component.value.type === "rank" || component.value.type === "enum"}
                    <select
                      value={component.value.value ?? ""}
                      disabled={!dated || saving}
                      onchange={(event) => {
                        const type = component.value.type;
                        if (type !== "rank" && type !== "enum") return;
                        patchValue(row, component, {
                          type,
                          value: event.currentTarget.value || null,
                        });
                      }}>
                      <option value="">—</option>
                      {#each component.scale ?? [] as option}
                        <option value={option}>{option}</option>
                      {/each}
                    </select>
                  {:else if component.value.type === "resource"}
                    <span class="resource">
                      <input
                        type="text"
                        inputmode="numeric"
                        aria-label="{component.name} current"
                        value={component.value.current ?? ""}
                        disabled={!dated || saving}
                        onchange={(event) => {
                          if (component.value.type !== "resource") return;
                          patchValue(row, component, {
                            ...component.value,
                            current: parseNumber(event.currentTarget.value),
                          });
                        }} />
                      <span>/</span>
                      <input
                        type="text"
                        inputmode="numeric"
                        aria-label="{component.name} max"
                        value={component.value.max ?? ""}
                        disabled={!dated || saving}
                        onchange={(event) => {
                          if (component.value.type !== "resource") return;
                          patchValue(row, component, {
                            ...component.value,
                            max: parseNumber(event.currentTarget.value),
                          });
                        }} />
                    </span>
                  {:else}
                    <input
                      type="text"
                      value={component.value.type === "text"
                        ? (component.value.value ?? "")
                        : formatProfileValue(component)}
                      disabled={!dated || saving}
                      onchange={(event) =>
                        patchValue(row, component, { type: "text", value: event.currentTarget.value || null })} />
                  {/if}
                </label>
              {/each}
              <label>
                <span>Reason</span>
                <input
                  type="text"
                  placeholder="Optional"
                  value={row.reason}
                  disabled={!dated || saving}
                  onchange={(event) => {
                    row.reason = event.currentTarget.value;
                    rows = [...rows];
                  }} />
              </label>
            </div>
            <button class="primary-button" type="button" disabled={!dated || saving} onclick={() => void saveRow(row)}>
              Save changes
            </button>
          {/if}
        </li>
      {/each}
    </ul>
  {/if}
{/if}

<style>
.copy {
  margin: 8px 0;
  color: var(--ink-soft);
  font-size: 11px;
  line-height: 1.45;
}
ul {
  display: grid;
  gap: 12px;
  margin: 10px 0 0;
  padding: 0;
  list-style: none;
}
li {
  display: grid;
  gap: 8px;
  padding: 10px 0 0;
  border-top: 1px solid var(--line);
}
.head {
  display: flex;
  gap: 8px;
  align-items: center;
  justify-content: space-between;
}
.head strong {
  font-size: 12px;
}
.quiet {
  border: 0;
  background: transparent;
  color: var(--ink-soft);
  cursor: pointer;
  font: 700 10px var(--font-body);
}
.fields {
  display: grid;
  gap: 8px;
}
.fields label {
  display: grid;
  gap: 4px;
}
.fields span {
  color: var(--ink-muted);
  font-size: 10px;
  font-weight: 700;
  letter-spacing: 0.04em;
  text-transform: uppercase;
}
.fields input,
.fields select {
  width: 100%;
  min-height: 28px;
  padding: 4px 8px;
  border: 1px solid var(--line);
  border-radius: 6px;
  background: var(--canvas);
  color: var(--ink);
}
.resource {
  display: flex;
  gap: 6px;
  align-items: center;
  text-transform: none;
  letter-spacing: 0;
  font-weight: 500;
}
.primary-button {
  width: fit-content;
}
</style>
