<script lang="ts">
import type { EntitySummary, ModuleContext, UUID } from "../../../packages/module-api/src/index";
import { trapModalTab } from "$lib/shell/modalFocus";
import AsyncEntityPicker from "$lib/entity-lifecycle/AsyncEntityPicker.svelte";
import { toAsyncEntityPage, type AsyncEntityOption } from "$lib/entity-lifecycle/asyncEntityQuery.ts";
import {
  PARENT_KINDS,
  PARTNER_KINDS,
  PARTNER_STATUSES,
  PERSON_TYPE,
  type ParentKind,
  type PartnerKind,
  type PartnerStatus,
  type RelativeRole,
} from "./model.ts";
import {
  classifyMutationError,
  createFamilyRelationship,
  createMinimalPerson,
  metadataFingerprint,
} from "./mutations.ts";

let {
  context,
  currentId,
  currentName,
  currentRevision: _currentRevision,
  role,
  excludeIds,
  coParentIds = [],
  coParentName = "",
  otherPerson = null,
  wouldCycle = () => false,
  onLinked,
  onCreatedPerson,
  onOpenEntity,
  onClose,
}: {
  context: ModuleContext;
  currentId: string;
  currentName: string;
  currentRevision: string;
  role: RelativeRole;
  excludeIds: string[];
  coParentIds?: string[];
  coParentName?: string;
  otherPerson?: EntitySummary | null;
  wouldCycle?: (otherId: string) => boolean | string | null;
  onLinked: (relationshipId: string) => void;
  onCreatedPerson: (person: EntitySummary) => void;
  onOpenEntity: (id: string) => void;
  onClose: () => void;
} = $props();

let mode = $state<"link" | "create">("link");
let saving = $state(false);
let error = $state("");
let name = $state("");
let parentKind = $state<ParentKind>("biological");
let partnerKind = $state<PartnerKind>("marriage");
let partnerStatus = $state<PartnerStatus>("active");
let customLabel = $state("");
let notes = $state("");
let created = $state<EntitySummary | null>(null);
let linkedOk = $state(false);
let pendingRequestId = $state<string | null>(null);
let lastLinkFingerprint = $state("");
let dialogEl = $state<HTMLElement | null>(null);
const excluded = $derived([currentId, ...excludeIds]);

const title = $derived(
  role === "parent"
    ? `Add parent of ${currentName}`
    : role === "child"
      ? coParentName
        ? `Add child of ${currentName} and ${coParentName}`
        : `Add child of ${currentName}`
      : otherPerson
        ? `Add partnership of ${currentName} and ${otherPerson.name}`
        : `Add partner of ${currentName}`,
);

function metadata() {
  if (role === "partner") {
    return {
      kind: partnerKind,
      status: partnerStatus,
      customLabel: partnerKind === "custom" ? customLabel : undefined,
      notes,
    };
  }
  return { kind: parentKind, customLabel: parentKind === "custom" ? customLabel : undefined, notes };
}

async function searchPeople(query: { text: string; offset: number; limit: number; excludeIds?: string[] }) {
  const page = await context.entities.query({
    types: [PERSON_TYPE],
    text: query.text || undefined,
    sortField: "name",
    sortDirection: "asc",
    offset: query.offset,
    limit: query.limit,
  });
  return toAsyncEntityPage(page, { excludeIds: query.excludeIds ?? excluded });
}

function toSummary(entity: AsyncEntityOption): EntitySummary {
  return {
    id: entity.id as EntitySummary["id"],
    name: entity.name,
    type: entity.entityType ?? PERSON_TYPE,
    deleted: false,
    revision: entity.revision ?? "",
  };
}

function sourceIdFor(candidate: EntitySummary) {
  if (role === "parent") return candidate.id;
  if (role === "child") return currentId;
  return currentId < candidate.id ? currentId : candidate.id;
}

function requestIdForLink(preferred?: string) {
  const next = metadataFingerprint(metadata());
  if (preferred && next === lastLinkFingerprint) return preferred;
  lastLinkFingerprint = next;
  return crypto.randomUUID();
}

async function performLink(candidate: EntitySummary, requestId: string) {
  if ((parentKind === "custom" || partnerKind === "custom") && !customLabel.trim()) {
    error = "Custom label is required.";
    return false;
  }
  const cycle = wouldCycle(candidate.id);
  if (cycle) {
    error = typeof cycle === "string" ? cycle : "That parent link would create a cycle.";
    return false;
  }
  pendingRequestId = requestId;
  const sourceId = sourceIdFor(candidate);
  const latest = (await context.entities.get(sourceId as UUID)) ?? candidate;
  const relationship = await createFamilyRelationship(context, {
    role,
    currentId,
    otherId: candidate.id,
    metadata: metadata(),
    sourceRevision: latest.revision,
    requestId,
  });
  if (role === "child") {
    for (const coParentId of coParentIds) {
      if (!coParentId || coParentId === candidate.id) continue;
      try {
        const coParent = await context.entities.get(coParentId as UUID);
        await createFamilyRelationship(context, {
          role: "child",
          currentId: coParentId,
          otherId: candidate.id,
          metadata: metadata(),
          sourceRevision: coParent?.revision ?? "",
          requestId: crypto.randomUUID(),
        });
      } catch (cause) {
        const failure = classifyMutationError(cause);
        if (failure.code !== "relationship.duplicate") {
          error = `Linked to ${currentName}, but not to ${coParentName || "their partner"}: ${failure.message}`;
        }
      }
    }
  }
  onLinked(relationship.id);
  if (created) linkedOk = true;
  else onClose();
  return true;
}

async function linkPerson(candidate: EntitySummary, requestId?: string) {
  if (saving) return;
  saving = true;
  error = "";
  try {
    await performLink(candidate, requestIdForLink(requestId));
  } catch (cause) {
    error = classifyMutationError(cause).message;
  } finally {
    saving = false;
  }
}

async function createPerson() {
  if (saving || !name.trim()) return;
  saving = true;
  error = "";
  try {
    const person = await createMinimalPerson(context, name, crypto.randomUUID());
    created = person;
    onCreatedPerson(person);
    await performLink(person, requestIdForLink());
  } catch (cause) {
    error = classifyMutationError(cause).message;
  } finally {
    saving = false;
  }
}

async function retryLink() {
  if (!created || saving) return;
  saving = true;
  error = "";
  try {
    await performLink(created, requestIdForLink(pendingRequestId ?? undefined));
  } catch (cause) {
    error = classifyMutationError(cause).message;
  } finally {
    saving = false;
  }
}

function onKeydown(event: KeyboardEvent) {
  trapModalTab(event, dialogEl);
  if (event.key === "Escape" && !saving) {
    event.preventDefault();
    onClose();
  }
}

$effect(() => {
  dialogEl?.focus();
});
</script>

<!-- svelte-ignore a11y_click_events_have_key_events -->
<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
<div
  class="overlay"
  bind:this={dialogEl}
  tabindex="-1"
  role="dialog"
  aria-modal="true"
  aria-label={title}
  onkeydown={onKeydown}
  onclick={(e) => {
    if (e.target === e.currentTarget && !saving) onClose();
  }}>
  <div class="card" role="document" onclick={(e) => e.stopPropagation()}>
    <header class="dialog-heading">
      <div>
        <span class="panel-kicker">FAMILY</span><strong>{title}</strong>
      </div>
      <button type="button" class="dialog-close" aria-label="Close dialog" disabled={saving} onclick={onClose}
        >×</button>
    </header>
    {#if linkedOk && created}
      <p class="hint">Person created and linked. Open in Lore to add dates, portrait, and details.</p>
      <div class="actions">
        <button type="button" class="primary-button" onclick={() => onOpenEntity(created!.id)}>Open in Lore</button>
        <button type="button" class="quiet-button" onclick={onClose}>Done</button>
      </div>
    {:else if created && error}
      <p class="error" role="alert">
        {created.name} was created, but the relationship was not saved.
      </p>
      <div class="actions">
        <button type="button" class="primary-button" disabled={saving} onclick={() => void retryLink()}
          >Retry relationship</button>
        <button type="button" class="quiet-button" onclick={() => onOpenEntity(created!.id)}
          >Open person in Lore</button>
        <button type="button" class="quiet-button" disabled={saving} onclick={onClose}>Cancel</button>
      </div>
    {:else}
      {#if !otherPerson}
        <div class="tabs">
          <button type="button" class="quiet-button" aria-pressed={mode === "link"} onclick={() => (mode = "link")}
            >Link existing</button>
          <button type="button" class="quiet-button" aria-pressed={mode === "create"} onclick={() => (mode = "create")}
            >Create person</button>
        </div>
      {/if}
      {#if role !== "partner"}
        <label>
          Parent type
          <select bind:value={parentKind}>
            {#each PARENT_KINDS as kind}<option value={kind}>{kind}</option>{/each}
          </select>
        </label>
      {:else}
        <label>
          Partnership type
          <select bind:value={partnerKind}>
            {#each PARTNER_KINDS as kind}<option value={kind}>{kind}</option>{/each}
          </select>
        </label>
        <label>
          Status
          <select bind:value={partnerStatus}>
            {#each PARTNER_STATUSES as status}<option value={status}>{status}</option>{/each}
          </select>
        </label>
      {/if}
      {#if parentKind === "custom" || partnerKind === "custom"}
        <label>Custom label <input bind:value={customLabel} /></label>
      {/if}
      <label>Notes <textarea bind:value={notes} rows="2"></textarea></label>
      {#if role === "child" && coParentName}
        <p class="hint">Also links {coParentName} as a parent so the child hangs from this marriage.</p>
      {/if}
      {#if otherPerson}
        <p class="hint">These two people already share a child. Save to record a partnership.</p>
        <button type="button" class="primary-button" disabled={saving} onclick={() => void linkPerson(otherPerson)}
          >Save partnership</button>
      {:else if mode === "link"}
        <AsyncEntityPicker
          search={searchPeople}
          entityTypes={[PERSON_TYPE]}
          excludeIds={excluded}
          pageSize={20}
          dropdown={false}
          disabled={saving}
          placeholder="Search people"
          ariaLabel="Search people"
          emptyMessage="No matching people."
          onSelect={(entity) => void linkPerson(toSummary(entity))} />
      {:else}
        <label>Name <input bind:value={name} /></label>
        <p class="hint">Creates a Lore person, then links them. Dates and portraits are added in Lore.</p>
        <button
          type="button"
          class="primary-button"
          disabled={saving || !name.trim()}
          onclick={() => void createPerson()}>Create and link</button>
      {/if}
    {/if}
    {#if error && !created}<p class="error" role="alert">{error}</p>{/if}
  </div>
</div>

<style>
.overlay {
  position: fixed;
  inset: 0;
  z-index: 300;
  display: grid;
  place-items: center;
  padding: 20px;
  background: rgba(37, 37, 31, 0.28);
}
.card {
  width: min(440px, 100%);
  max-height: min(84vh, 720px);
  overflow: auto;
  display: grid;
  gap: 12px;
  padding: 22px;
  border: 1px solid var(--theme-warning-border, #e3d9ca);
  border-radius: 14px;
  background: var(--surface);
  box-shadow: 0 22px 70px rgba(37, 37, 31, 0.2);
  outline: none;
}
.dialog-heading {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 12px;
}
.dialog-heading strong {
  display: block;
  font-size: 16px;
  line-height: 1.3;
  color: var(--ink);
}
.panel-kicker {
  display: block;
  color: var(--accent);
  font-size: 9px;
  font-weight: 800;
  letter-spacing: 0.12em;
}
.dialog-close {
  width: 30px;
  height: 30px;
  flex: none;
  border: 0;
  border-radius: 7px;
  background: var(--surface-muted);
  color: var(--ink-soft);
  font-size: 20px;
  line-height: 1;
  cursor: pointer;
}
.dialog-close:hover {
  background: var(--theme-warning-bg, #ebe6dd);
  color: var(--ink);
}
.tabs,
.actions {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
  align-items: center;
  justify-content: space-between;
}
label,
.hint,
.error {
  display: grid;
  gap: 4px;
  color: var(--ink);
  font:
    12px/1.4 Inter,
    ui-sans-serif,
    system-ui,
    sans-serif;
}
.hint {
  color: var(--ink-muted);
}
.error {
  color: var(--theme-danger-text, #8a2b2b);
  background: var(--danger-bg, #fff2ee);
  border: 1px solid var(--danger-line, #edcec5);
  border-radius: 8px;
  padding: 8px 10px;
}
input,
select,
textarea {
  width: 100%;
  box-sizing: border-box;
  padding: 7px 9px;
  border: 1px solid var(--line-strong);
  border-radius: 8px;
  background: var(--surface);
  color: var(--ink);
  font-size: 13px;
}
input:focus-visible,
select:focus-visible,
textarea:focus-visible {
  outline: 2px solid var(--accent);
  outline-offset: 1px;
  border-color: var(--accent);
}
.quiet-button,
.primary-button {
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
  color: var(--ink-soft, var(--ink));
}
.primary-button {
  background: var(--accent-dark, var(--accent));
  border-color: transparent;
  color: #fff;
}
</style>
