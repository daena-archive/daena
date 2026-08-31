<script lang="ts">
import type { ModuleContext } from "../../../packages/module-api/src/index";
import { Archive, ExternalLink, Pencil, Plus, Search, UserPlus } from "@lucide/svelte";
import { ENTITY_ACTIONS, ENTITY_ACTION_CONFIRM } from "$lib/entity-lifecycle/vocabulary.ts";
import { archiveConfirmOptions } from "$lib/entity-lifecycle/archive.ts";
import { confirmDialog } from "$lib/dialogs.svelte";
import { formatMembershipRole, type HouseMemberRecord } from "./model.ts";
import FamilyMembershipDialog from "./FamilyMembershipDialog.svelte";

let {
  context,
  houseId,
  houseName,
  members = [],
  busy = false,
  onOpenEntry,
  onOpenPerson,
  onArchive,
  onRename,
  onMembersChanged,
  onClose,
}: {
  context: ModuleContext;
  houseId: string;
  houseName: string;
  members?: HouseMemberRecord[];
  busy?: boolean;
  onOpenEntry: (houseId: string) => void;
  onOpenPerson: (personId: string) => void;
  onArchive: (houseId: string) => void | Promise<void>;
  onRename?: (houseId: string, name: string) => void | Promise<void>;
  onMembersChanged: () => void | Promise<void>;
  onClose: () => void;
} = $props();

let memberQuery = $state("");
let roleFilter = $state("all");
let editingName = $state(false);
// svelte-ignore state_referenced_locally: initial draft reflects initial houseName, synced via effect below
let nameDraft = $state(houseName);
let membershipDialog = $state<"add" | "create" | HouseMemberRecord | null>(null);

$effect(() => {
  if (!editingName) nameDraft = houseName;
});

const roleOptions = $derived.by(() => {
  const roles = new Set<string>();
  for (const member of members) {
    roles.add(member.role?.trim() || "member");
  }
  return ["all", ...[...roles].sort((a, b) => a.localeCompare(b))];
});

const visibleMembers = $derived.by(() => {
  const needle = memberQuery.trim().toLowerCase();
  return members.filter((member) => {
    if (roleFilter !== "all" && (member.role?.trim() || "member") !== roleFilter) return false;
    if (!needle) return true;
    const hay =
      `${member.personName} ${formatMembershipRole(member.role, member.customLabel)} ${member.notes ?? ""}`.toLowerCase();
    return hay.includes(needle);
  });
});

async function commitRename() {
  const next = nameDraft.trim();
  editingName = false;
  if (!next || next === houseName || !onRename) {
    nameDraft = houseName;
    return;
  }
  await onRename(houseId, next);
}

async function archiveHouse() {
  const ok = await confirmDialog(archiveConfirmOptions(houseName || "House"));
  if (!ok) return;
  await onArchive(houseId);
}
</script>

<aside class="panel" aria-label={houseName || "House"}>
  <header class="panel-head">
    <div class="head-copy">
      <span class="kicker">HOUSE</span>
      {#if editingName && onRename}
        <input
          class="name-input"
          bind:value={nameDraft}
          aria-label="House name"
          onkeydown={(event) => {
            if (event.key === "Enter") void commitRename();
            if (event.key === "Escape") {
              editingName = false;
              nameDraft = houseName;
            }
          }}
          onblur={() => void commitRename()} />
      {:else}
        <strong class="house-name">{houseName || "House"}</strong>
      {/if}
      <span class="meta">{members.length} {members.length === 1 ? "member" : "members"}</span>
    </div>
    <button type="button" class="quiet-button ghost" onclick={onClose} aria-label="Close house details">Close</button>
  </header>

  <div class="panel-actions" role="group" aria-label="House actions">
    <button type="button" class="quiet-button pill" onclick={() => onOpenEntry(houseId)}>
      <ExternalLink size={13} strokeWidth={1.8} aria-hidden="true" /> Open full entry
    </button>
    {#if onRename}
      <button type="button" class="quiet-button pill" onclick={() => (editingName = true)}>
        <Pencil size={13} strokeWidth={1.8} aria-hidden="true" />
        {ENTITY_ACTIONS.editIdentity}
      </button>
    {/if}
    <button type="button" class="quiet-button pill danger" onclick={() => void archiveHouse()}>
      <Archive size={13} strokeWidth={1.8} aria-hidden="true" />
      {ENTITY_ACTIONS.archive}
    </button>
  </div>

  <div class="member-toolbar">
    <label class="search-field">
      <span class="input-icon" aria-hidden="true"><Search size={13} strokeWidth={1.8} /></span>
      <input type="search" bind:value={memberQuery} placeholder="Search members" aria-label="Search members" />
    </label>
    <label class="filter-field">
      <span class="sr-only">Filter by role</span>
      <select bind:value={roleFilter} aria-label="Filter by role">
        {#each roleOptions as role}
          <option value={role}>{role === "all" ? "All roles" : formatMembershipRole(role)}</option>
        {/each}
      </select>
    </label>
  </div>

  <div class="member-actions" role="group" aria-label="Membership actions">
    <button type="button" class="quiet-button" disabled={busy} onclick={() => (membershipDialog = "add")}>
      <Plus size={13} strokeWidth={1.8} aria-hidden="true" /> Add existing
    </button>
    <button type="button" class="quiet-button" disabled={busy} onclick={() => (membershipDialog = "create")}>
      <UserPlus size={13} strokeWidth={1.8} aria-hidden="true" /> Create person
    </button>
  </div>

  {#if busy && members.length === 0}
    <p class="hint">Loading members…</p>
  {:else if members.length === 0}
    <p class="hint">No members yet. Add an existing person or create one to start this house tree.</p>
  {:else if visibleMembers.length === 0}
    <p class="hint">No members match this filter.</p>
  {:else}
    <ul class="member-list">
      {#each visibleMembers as member (member.id)}
        <li>
          <button type="button" class="member-main" onclick={() => onOpenPerson(member.personId)}>
            <strong>{member.personName}</strong>
            <small>{formatMembershipRole(member.role, member.customLabel)}</small>
            {#if member.notes}<span class="notes">{member.notes}</span>{/if}
          </button>
          <button
            type="button"
            class="quiet-button ghost small"
            onclick={() => (membershipDialog = member)}
            aria-label={`Edit membership for ${member.personName}`}>Edit</button>
        </li>
      {/each}
    </ul>
  {/if}

  <p class="footnote">{ENTITY_ACTION_CONFIRM.removeMembershipMessage}</p>
</aside>

{#if membershipDialog}
  <FamilyMembershipDialog
    {context}
    {houseId}
    {houseName}
    excludeIds={members.map((member) => member.personId)}
    editing={typeof membershipDialog === "string" ? null : membershipDialog}
    initialMode={membershipDialog === "create" ? "create" : "link"}
    onClose={() => (membershipDialog = null)}
    onSaved={() => void onMembersChanged()}
    onRemoved={() => void onMembersChanged()} />
{/if}

<style>
.panel {
  display: grid;
  align-content: start;
  gap: 14px;
  height: 100%;
  min-height: 0;
  overflow: auto;
  padding: 16px;
  background: var(--surface);
}
.panel-head {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 12px;
  padding-bottom: 12px;
  border-bottom: 1px solid var(--line-soft, var(--line));
}
.head-copy {
  display: grid;
  gap: 2px;
  min-width: 0;
}
.kicker {
  color: var(--accent);
  font-size: 9px;
  font-weight: 800;
  letter-spacing: 0.12em;
}
.house-name,
.name-input {
  color: var(--ink);
  font: 600 15px/1.25 var(--font-body, Inter, ui-sans-serif, system-ui, sans-serif);
  letter-spacing: -0.01em;
}
.name-input {
  width: 100%;
  padding: 4px 6px;
  border: 1px solid var(--line);
  border-radius: 6px;
  background: var(--surface);
}
.meta {
  color: var(--ink-muted);
  font: 11px/1.35 var(--font-body, Inter, ui-sans-serif, system-ui, sans-serif);
}
.panel-actions,
.member-actions {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
}
.pill {
  border-radius: 999px !important;
}
.pill.danger {
  color: var(--theme-danger-text, #b42318);
}
.ghost {
  border-color: transparent;
  background: transparent;
}
.ghost:hover {
  border-color: var(--line);
  background: var(--surface-muted);
}
.small {
  padding: 6px 8px;
  font-size: 11px;
}
.member-toolbar {
  display: grid;
  gap: 8px;
}
.search-field,
.filter-field {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 0 10px;
  border: 1px solid var(--line);
  border-radius: 8px;
  background: var(--surface);
}
.search-field input,
.filter-field select {
  flex: 1;
  min-width: 0;
  padding: 8px 0;
  border: 0;
  background: transparent;
  color: var(--ink);
  font-size: 12px;
}
.input-icon {
  color: var(--ink-muted);
  display: inline-flex;
}
.member-list {
  display: grid;
  gap: 6px;
  margin: 0;
  padding: 0;
  list-style: none;
}
.member-list li {
  display: flex;
  align-items: center;
  gap: 6px;
}
.member-main {
  flex: 1 1 auto;
  min-width: 0;
  display: grid;
  gap: 2px;
  padding: 8px 10px;
  border: 1px solid var(--line);
  border-radius: 8px;
  background: var(--surface);
  color: var(--ink);
  text-align: left;
  cursor: pointer;
}
.member-main strong {
  font-size: 12px;
}
.member-main small {
  color: var(--ink-muted);
  font-size: 11px;
}
.notes {
  color: var(--ink-muted);
  font-size: 10px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.hint,
.footnote {
  margin: 0;
  color: var(--ink-muted);
  font-size: 12px;
  line-height: 1.4;
}
.footnote {
  font-size: 11px;
  padding-top: 8px;
  border-top: 1px solid var(--line-soft, var(--line));
}
.sr-only {
  position: absolute;
  width: 1px;
  height: 1px;
  padding: 0;
  margin: -1px;
  overflow: hidden;
  clip: rect(0, 0, 0, 0);
  white-space: nowrap;
  border: 0;
}
</style>
