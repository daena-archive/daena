<script lang="ts">
import type { EntitySummary, ModuleContext, Relationship } from "../../../packages/module-api/src/index";
import type { Snippet } from "svelte";
import { untrack } from "svelte";
import { trapModalTab } from "$lib/shell/modalFocus";
import FamilyMemberDialog from "./FamilyMemberDialog.svelte";
import FamilyRelationshipPanel from "./FamilyRelationshipPanel.svelte";
import FamilyRootPicker from "./FamilyRootPicker.svelte";
import FamilyTreeCanvas from "./FamilyTreeCanvas.svelte";
import {
  isNeighborhoodAbort,
  listPersonSecondaryFields,
  loadExpansionLayer,
  loadGenealogyNeighborhood,
} from "./fetch.ts";
import { requestElkLayout, terminateElkLayout } from "./elk.ts";
import { buildElkGraph, LayoutGeneration, placeUnions, positionedFromElk } from "./layout";
import {
  BRANCH_TOO_LARGE,
  DEFAULT_SECONDARY_FIELD,
  PERSON_TYPE,
  expansionKey,
  type BranchDirection,
  type FamilyPerson,
  type FamilyRelationship,
  type FamilyTreeSession,
  type FamilyViewport,
  type GenealogyWarning,
  type HiddenCounts,
  type RelativeRole,
} from "./model.ts";
import {
  expansionBlocked,
  formatParentCycleMessage,
  hiddenCounts,
  initialNeighborhood,
  normalizeGenealogy,
  parentCyclePath,
  seedInitialExpansions,
  visibleFromExpansions,
  wouldExceedVisibleLimit,
} from "./projection.ts";
import { recentRoots, rememberRecentRoot, replaceRecentRoots } from "./state.ts";
import { buildLayoutGraph, layoutGraphExceedsLimits } from "./unions.ts";

let {
  context,
  projectId,
  initialRootId = null,
  initialSession = null,
  restoreNonce = 0,
  avatar,
  onOpenEntity,
  onRootChange,
  onSessionChange,
}: {
  context: ModuleContext;
  projectId: string;
  initialRootId?: string | null;
  initialSession?: FamilyTreeSession | null;
  restoreNonce?: number;
  avatar?: Snippet<[string, string]>;
  onOpenEntity: (entityId: string) => void;
  onRootChange?: (rootId: string | null) => void;
  onSessionChange?: (session: FamilyTreeSession | null) => void;
} = $props();

let rootId = $state<string | null>(null);
let selectedPersonId = $state<string | null>(null);
let selectedRelationshipId = $state<string | null>(null);
let people = $state(new Map<string, FamilyPerson>());
let warnings = $state<GenealogyWarning[]>([]);
let loading = $state(false);
let error = $state("");
let layoutFailed = $state(false);
let positioned = $state<ReturnType<typeof positionedFromElk> | null>(null);
let fitToken = $state(0);
let canvasFit = $state(true);
let viewport = $state<FamilyViewport | null>(null);
let rerootCandidate = $state<{ id: string; name: string } | null>(null);
let truncationLowerBound = $state(0);
let pickerOpen = $state(false);
let recentMenu = $state<{ id: string; name: string }[]>([]);
let secondaryField = $state(DEFAULT_SECONDARY_FIELD);
let secondaryFields = $state<{ key: string; label: string }[]>([{ key: DEFAULT_SECONDARY_FIELD, label: "Occupation" }]);
let overlayEl = $state<HTMLElement | null>(null);
let expansions = $state<string[]>([]);
let truncated = $state(false);
let member = $state<{ id: string; role: RelativeRole; coParentIds?: string[] } | null>(null);
let abort: AbortController | null = null;
const generations = new LayoutGeneration();
let previousOrder: string[] = [];
let latestLayoutGraph: ReturnType<typeof buildLayoutGraph> | null = null;
let appliedInitial: string | null = null;
let appliedRestore = 0;
let rawPeople: FamilyPerson[] = [];
let rawRelationships: Relationship[] = [];
let collected = new Map<string, Relationship>();

const graph = $derived(normalizeGenealogy(rawPeople, rawRelationships).graph);
const selectedRelationship = $derived(
  selectedRelationshipId ? (graph.relationships.get(selectedRelationshipId) ?? null) : null,
);
const hiddenByPerson = $derived.by(() => {
  const visible = new Set(people.keys());
  const map = new Map<string, HiddenCounts>();
  for (const id of visible) map.set(id, hiddenCounts(graph, id, visible, truncated, truncationLowerBound));
  return map;
});
const expandedByPerson = $derived.by(() => {
  const map = new Map<string, Record<BranchDirection, boolean>>();
  for (const id of people.keys()) {
    map.set(id, {
      parents: expansions.includes(expansionKey(id, "parents")),
      children: expansions.includes(expansionKey(id, "children")),
      siblings: expansions.includes(expansionKey(id, "siblings")),
      partners: expansions.includes(expansionKey(id, "partners")),
    });
  }
  return map;
});

function cancelLoad() {
  abort?.abort();
  abort = null;
  generations.start();
}

function logLayoutFailure(generation: number, message?: string) {
  const bounded = (message ?? "layout-worker").replace(/\s+/g, " ").slice(0, 200);
  console.info("family-tree.layout.failed", { generation, code: "layout-worker", message: bounded });
}

function requestLayout() {
  const graph = latestLayoutGraph;
  if (!graph) return;
  const generation = generations.start();
  void requestElkLayout(buildElkGraph(graph, previousOrder))
    .then((laidOut) => {
      if (!generations.accept(generation) || latestLayoutGraph !== graph) return;
      positioned = placeUnions(positionedFromElk(generation, graph, laidOut));
      previousOrder = positioned.nodes.map((node) => node.id);
      layoutFailed = false;
    })
    .catch((cause) => {
      if (!generations.accept(generation)) return;
      layoutFailed = true;
      logLayoutFailure(generation, cause instanceof Error ? cause.message : String(cause));
    });
}

function mergeRecords(nextPeople: FamilyPerson[], nextRelationships: Relationship[]) {
  const peopleById = new Map(rawPeople.map((person) => [person.id, person]));
  for (const person of nextPeople) peopleById.set(person.id, person);
  rawPeople = [...peopleById.values()];
  for (const relationship of nextRelationships) collected.set(relationship.id, relationship);
  rawRelationships = [...collected.values()];
}

function applyVisible(nextExpansions: string[], fit: boolean, protect: string[] = []) {
  if (!rootId) return false;
  const { graph: nextGraph, warnings: graphWarnings } = normalizeGenealogy(rawPeople, rawRelationships);
  const { visible } = visibleFromExpansions(nextGraph, rootId, nextExpansions, protect);
  const candidate = buildLayoutGraph(nextGraph, visible);
  if (wouldExceedVisibleLimit(visible.size) || layoutGraphExceedsLimits(candidate)) {
    error = BRANCH_TOO_LARGE;
    return false;
  }
  rerootCandidate = null;
  people = new Map(
    [...visible].flatMap((id) => {
      const person = nextGraph.people.get(id);
      return person ? [[id, person] as const] : [];
    }),
  );
  warnings = graphWarnings;
  expansions = nextExpansions;
  latestLayoutGraph = candidate;
  if (fit) fitToken += 1;
  requestLayout();
  return true;
}

async function refreshRecent() {
  const ids = recentRoots(projectId);
  if (ids.length === 0) {
    recentMenu = [];
    return;
  }
  try {
    const records = await context.entities.getMany(ids as EntitySummary["id"][]);
    const byId = new Map(records.map((record) => [record.id, record]));
    const kept: { id: string; name: string }[] = [];
    for (const id of ids) {
      const record = byId.get(id as EntitySummary["id"]);
      if (!record || record.deleted || record.type !== PERSON_TYPE) continue;
      kept.push({ id: record.id, name: record.name });
    }
    replaceRecentRoots(
      projectId,
      kept.map((entry) => entry.id),
    );
    recentMenu = kept;
  } catch {
    recentMenu = ids.map((id) => ({ id, name: people.get(id)?.name ?? id }));
  }
}

async function loadRoot(id: string, fit = true, restored: string[] | null = null) {
  cancelLoad();
  abort = new AbortController();
  const signal = abort.signal;
  loading = true;
  error = "";
  layoutFailed = false;
  try {
    const loaded = await loadGenealogyNeighborhood(context, id, secondaryField, signal);
    if (signal.aborted) return;
    collected = new Map(loaded.relationships.map((relationship) => [relationship.id, relationship]));
    rawPeople = loaded.people;
    rawRelationships = loaded.relationships;
    truncated = loaded.truncated;
    truncationLowerBound = loaded.truncationLowerBound;
    const { graph: nextGraph, warnings: graphWarnings } = normalizeGenealogy(loaded.people, loaded.relationships);
    const nextExpansions = restored ?? [...seedInitialExpansions(nextGraph, id)];
    const visible = restored
      ? visibleFromExpansions(nextGraph, id, nextExpansions).visible
      : initialNeighborhood(nextGraph, id);
    const candidate = buildLayoutGraph(nextGraph, visible);
    if (wouldExceedVisibleLimit(visible.size) || layoutGraphExceedsLimits(candidate)) {
      error = BRANCH_TOO_LARGE;
      rerootCandidate = { id, name: loaded.people.find((person) => person.id === id)?.name ?? id };
      return;
    }
    rerootCandidate = null;
    people = new Map(
      [...visible].flatMap((personId) => {
        const person = nextGraph.people.get(personId);
        return person ? [[personId, person] as const] : [];
      }),
    );
    warnings = [...loaded.warnings, ...graphWarnings];
    latestLayoutGraph = candidate;
    expansions = nextExpansions;
    rootId = id;
    selectedPersonId = restored ? (initialSession?.selectedPersonId ?? id) : id;
    selectedRelationshipId = restored ? (initialSession?.selectedRelationshipId ?? null) : null;
    pickerOpen = false;
    rememberRecentRoot(projectId, id);
    onRootChange?.(id);
    void refreshRecent();
    canvasFit = fit;
    if (fit) {
      viewport = null;
      fitToken += 1;
    } else if (restored && initialSession?.viewport) {
      viewport = initialSession.viewport;
    }
    requestLayout();
  } catch (cause) {
    if (signal.aborted || isNeighborhoodAbort(cause)) return;
    error = cause instanceof Error ? cause.message : String(cause);
  } finally {
    if (!signal.aborted) loading = false;
  }
}

function selectRoot(person: EntitySummary) {
  previousOrder = [];
  void loadRoot(person.id, true);
}

function retryLayout() {
  if (!latestLayoutGraph) return;
  requestLayout();
}

function changeRoot() {
  pickerOpen = true;
}

function fitView() {
  canvasFit = true;
  viewport = null;
  fitToken += 1;
}

function offerReroot(personId: string) {
  rerootCandidate = { id: personId, name: people.get(personId)?.name ?? personId };
}

function acceptReroot() {
  if (!rerootCandidate) return;
  const id = rerootCandidate.id;
  rerootCandidate = null;
  error = "";
  previousOrder = [];
  void loadRoot(id, true);
}

function resetView() {
  if (!rootId) return;
  previousOrder = [];
  void loadRoot(rootId, true);
}

function onSecondaryChange(event: Event) {
  const value = (event.currentTarget as HTMLSelectElement).value;
  secondaryField = value;
  if (rootId) {
    previousOrder = [...(positioned?.nodes.map((node) => node.id) ?? [])];
    void loadRoot(rootId, false, expansions);
  }
}

async function toggleBranch(personId: string, direction: BranchDirection) {
  if (!rootId) return;
  const key = expansionKey(personId, direction);
  if (expansions.includes(key)) {
    applyVisible(
      expansions.filter((item) => item !== key),
      false,
      [rootId, selectedPersonId].filter((id): id is string => Boolean(id)),
    );
    return;
  }
  if (expansionBlocked(graph, rootId, personId, direction)) {
    error = BRANCH_TOO_LARGE;
    offerReroot(personId);
    return;
  }
  await Promise.resolve();
  cancelLoad();
  abort = new AbortController();
  const signal = abort.signal;
  loading = true;
  error = "";
  try {
    const loaded = await loadExpansionLayer(
      context,
      personId,
      direction,
      collected,
      new Set(rawPeople.map((person) => person.id)),
      secondaryField,
      signal,
    );
    if (signal.aborted) return;
    mergeRecords(loaded.people, loaded.relationships);
    truncated = truncated || loaded.truncated;
    truncationLowerBound = Math.max(truncationLowerBound, loaded.truncationLowerBound);
    if (!applyVisible([...expansions, key], false)) {
      offerReroot(personId);
      return;
    }
  } catch (cause) {
    if (signal.aborted || isNeighborhoodAbort(cause)) return;
    error = cause instanceof Error ? cause.message : String(cause);
  } finally {
    if (!signal.aborted) loading = false;
  }
}

function excludeFor(personId: string, role: RelativeRole) {
  const parents = [...(graph.parentsByChild.get(personId) ?? [])];
  const children = [...(graph.childrenByParent.get(personId) ?? [])];
  const partners = [...(graph.partnersByPerson.get(personId) ?? [])];
  if (role === "parent") return [...parents, ...children, ...partners];
  if (role === "child") return [...children, ...parents, ...partners];
  return partners;
}

function coParentsFor(personId: string) {
  const partners = [...(graph.partnersByPerson.get(personId) ?? [])].filter((id) => people.has(id));
  return partners.length === 1 ? partners : [];
}

async function afterLink() {
  if (!rootId) return;
  const parentId = member?.id;
  const role = member?.role;
  const nextExpansions = [...expansions];
  if (parentId && role === "child") {
    for (const id of [parentId, ...(member?.coParentIds ?? coParentsFor(parentId))]) {
      if (!nextExpansions.includes(expansionKey(id, "children"))) nextExpansions.push(expansionKey(id, "children"));
    }
  }
  if (parentId && role === "parent" && !nextExpansions.includes(expansionKey(parentId, "parents"))) {
    nextExpansions.push(expansionKey(parentId, "parents"));
  }
  if (parentId && role === "partner" && !nextExpansions.includes(expansionKey(parentId, "partners"))) {
    nextExpansions.push(expansionKey(parentId, "partners"));
  }
  previousOrder = [...(positioned?.nodes.map((node) => node.id) ?? [])];
  await loadRoot(rootId, false, nextExpansions);
}

function onOverlayKeydown(event: KeyboardEvent) {
  trapModalTab(event, overlayEl);
  if (event.key === "Escape") {
    event.preventDefault();
    pickerOpen = false;
  }
}

$effect(() => {
  if (pickerOpen) overlayEl?.focus();
});

$effect(() => {
  void projectId;
  const moduleContext = untrack(() => context);
  void listPersonSecondaryFields(moduleContext)
    .then((fields) => {
      const next = fields.length > 0 ? fields : secondaryFields;
      if (JSON.stringify(next) !== JSON.stringify(untrack(() => secondaryFields))) secondaryFields = next;
      if (!next.some((field) => field.key === untrack(() => secondaryField))) {
        secondaryField = next[0]?.key ?? DEFAULT_SECONDARY_FIELD;
      }
    })
    .catch(() => {});
  void refreshRecent();
});

$effect(() => {
  const next = initialRootId;
  if (!next) return;
  if (restoreNonce !== appliedRestore) {
    appliedRestore = restoreNonce;
    appliedInitial = next;
    previousOrder = [];
    void loadRoot(next, !initialSession?.viewport, initialSession?.expansions ?? null);
    return;
  }
  if (next === rootId || next === appliedInitial) return;
  appliedInitial = next;
  previousOrder = [];
  void loadRoot(next, true);
});

$effect(() => {
  const session = rootId
    ? {
        expansions,
        selectedPersonId,
        selectedRelationshipId,
        viewport,
      }
    : null;
  untrack(() => onSessionChange?.(session));
});

$effect(() => {
  return () => {
    cancelLoad();
    terminateElkLayout();
  };
});
</script>

<section class="surface">
  <header>
    <div>
      <span class="overline">FAMILY TREE</span>
      <h1>{rootId ? (people.get(rootId)?.name ?? "Family Tree") : "Family Tree"}</h1>
      <p>
        {rootId
          ? "A bounded neighborhood of Lore people and family relationships."
          : "Choose a Lore person to inspect their family neighborhood."}
      </p>
    </div>
    {#if rootId}
      <div class="toolbar">
        <button type="button" class="quiet-button" onclick={changeRoot}>Search root</button>
        {#if recentMenu.length > 0}
          <label class="recent">
            <span class="sr">Recent roots</span>
            <select
              aria-label="Recent roots"
              onchange={(event) => void loadRoot((event.currentTarget as HTMLSelectElement).value, true)}>
              <option value="" disabled selected>Recent roots</option>
              {#each recentMenu as entry (entry.id)}
                <option value={entry.id}>{entry.name}</option>
              {/each}
            </select>
          </label>
        {/if}
        <label class="recent">
          <span class="sr">Secondary field</span>
          <select aria-label="Secondary field" value={secondaryField} onchange={onSecondaryChange}>
            {#each secondaryFields as field (field.key)}
              <option value={field.key}>{field.label}</option>
            {/each}
          </select>
        </label>
        <button type="button" class="quiet-button" onclick={fitView}>Fit</button>
        <button type="button" class="quiet-button" onclick={resetView}>Reset</button>
        <button type="button" class="quiet-button" onclick={changeRoot}>Change root</button>
      </div>
    {/if}
  </header>
  {#if error}
    <p class="banner" role="alert">
      {error}
      {#if rerootCandidate}
        <button type="button" class="quiet-button" onclick={acceptReroot}>Make {rerootCandidate.name} root</button>
      {/if}
    </p>
  {/if}
  {#if layoutFailed}
    <p class="banner" role="alert">
      Layout failed. The previous arrangement was kept.
      <button type="button" class="quiet-button" onclick={retryLayout}>Retry</button>
    </p>
  {/if}
  {#if warnings.length > 0}
    <p class="hint">
      {warnings.length} data warning{warnings.length === 1 ? "" : "s"} — unresolved or unknown family edges were skipped.
    </p>
  {/if}
  {#if !rootId}
    <FamilyRootPicker {context} onSelect={selectRoot} />
  {:else if loading && !positioned}
    <p class="hint">Loading family neighborhood…</p>
  {:else if positioned}
    <div class="workspace">
      <div class="canvas-wrap">
        <FamilyTreeCanvas
          layout={positioned}
          {people}
          {rootId}
          {selectedPersonId}
          {selectedRelationshipId}
          {hiddenByPerson}
          {expandedByPerson}
          {avatar}
          {onOpenEntity}
          onSelectPerson={(id) => (selectedPersonId = id)}
          onSelectRelationship={(id) => (selectedRelationshipId = id)}
          onMakeRoot={(id) => {
            previousOrder = [];
            void loadRoot(id, true);
          }}
          onToggleBranch={(id, direction) => void toggleBranch(id, direction)}
          onAddRelative={(id, role) => (member = { id, role })}
          onAddUnionChild={(memberIds) => {
            const [id, ...coParentIds] = memberIds.filter((personId) => people.has(personId));
            if (!id) return;
            member = { id, role: "child", coParentIds };
          }}
          {fitToken}
          fitView={canvasFit}
          initialViewport={canvasFit ? null : viewport}
          onViewportChange={(next) => {
            if (viewport && viewport.x === next.x && viewport.y === next.y && viewport.zoom === next.zoom) return;
            viewport = next;
          }} />
      </div>
    </div>
  {/if}
  {#if selectedRelationship}
    <div
      class="overlay"
      tabindex="-1"
      role="dialog"
      aria-modal="true"
      aria-label="Relationship"
      onclick={(event) => {
        if (event.target === event.currentTarget) selectedRelationshipId = null;
      }}
      onkeydown={(event) => {
        if (event.key === "Escape") {
          event.preventDefault();
          selectedRelationshipId = null;
        }
      }}>
      <FamilyRelationshipPanel
        {context}
        relationship={selectedRelationship}
        {people}
        onClose={() => (selectedRelationshipId = null)}
        onUpdated={(relationship: FamilyRelationship) => {
          collected.set(relationship.id, {
            id: relationship.id as Relationship["id"],
            sourceId: relationship.sourceId as Relationship["sourceId"],
            targetId: relationship.targetId as Relationship["targetId"],
            type: relationship.type,
            metadata: {
              kind: relationship.parentKind ?? relationship.partnerKind,
              customLabel: relationship.customLabel,
              status: relationship.status,
              start: relationship.start,
              end: relationship.end,
              notes: relationship.notes,
            },
            revision: relationship.revision,
          });
          rawRelationships = [...collected.values()];
          applyVisible(expansions, false);
        }}
        onDeleted={(id) => {
          collected.delete(id);
          rawRelationships = [...collected.values()];
          selectedRelationshipId = null;
          applyVisible(expansions, false);
        }} />
    </div>
  {/if}
  {#if pickerOpen && rootId}
    <div
      class="overlay"
      bind:this={overlayEl}
      tabindex="-1"
      role="dialog"
      aria-modal="true"
      aria-label="Choose a new root"
      onkeydown={onOverlayKeydown}>
      <div class="overlay-card">
        <header>
          <strong>Change root</strong>
          <button type="button" class="quiet-button" onclick={() => (pickerOpen = false)}>Close</button>
        </header>
        <FamilyRootPicker {context} compact onSelect={selectRoot} />
      </div>
    </div>
  {/if}
  {#if member && rootId}
    <FamilyMemberDialog
      {context}
      currentId={member.id}
      currentName={people.get(member.id)?.name ?? member.id}
      currentRevision={people.get(member.id)?.revision ?? ""}
      role={member.role}
      excludeIds={excludeFor(member.id, member.role)}
      coParentIds={member.role === "child" ? (member.coParentIds ?? coParentsFor(member.id)) : []}
      coParentName={member.role === "child"
        ? (people.get((member.coParentIds ?? coParentsFor(member.id))[0] ?? "")?.name ?? "")
        : ""}
      wouldCycle={(otherId) => {
        const path =
          member!.role === "parent"
            ? parentCyclePath(graph, otherId, member!.id)
            : member!.role === "child"
              ? parentCyclePath(graph, member!.id, otherId)
              : null;
        return path ? formatParentCycleMessage(path, (id) => people.get(id)?.name ?? id) : false;
      }}
      onLinked={() => void afterLink()}
      onCreatedPerson={() => {}}
      {onOpenEntity}
      onClose={() => {
        const id = member?.id ?? null;
        member = null;
        if (!id) return;
        queueMicrotask(() => {
          const card = document.querySelector(`[data-person-id="${CSS.escape(id)}"]`);
          if (card instanceof HTMLElement) card.focus();
        });
      }} />
  {/if}
</section>

<style>
.surface {
  position: relative;
  display: flex;
  flex: 1 1 auto;
  flex-direction: column;
  gap: 16px;
  width: 100%;
  height: 100%;
  min-height: 0;
  padding: 28px 32px;
}
header {
  display: flex;
  align-items: flex-end;
  justify-content: space-between;
  gap: 16px;
}
.overline {
  display: block;
  color: var(--accent);
  font-size: 10px;
  font-weight: 800;
  letter-spacing: 0.18em;
}
h1 {
  margin: 8px 0 4px;
  color: var(--ink);
  font: 500 32px/1 var(--font-display, Georgia, serif);
}
p {
  margin: 0;
  color: var(--ink-muted);
  font:
    13px/1.45 Inter,
    ui-sans-serif,
    system-ui,
    sans-serif;
}
.toolbar,
.banner {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
  align-items: center;
}
.quiet-button {
  padding: 8px 12px;
  border: 1px solid var(--line-strong);
  border-radius: 8px;
  background: var(--surface);
  color: var(--ink-soft, var(--ink));
  font-size: 12px;
  cursor: pointer;
}
.quiet-button:hover {
  background: var(--surface-muted, var(--surface));
  color: var(--ink);
}
.banner {
  color: var(--theme-warning-text, #55351f);
}
.hint {
  color: var(--ink-muted);
}
.workspace {
  display: grid;
  flex: 1 1 auto;
  grid-template-columns: 1fr auto;
  gap: 12px;
  min-height: 0;
  height: 100%;
}
.canvas-wrap {
  display: flex;
  flex-direction: column;
  min-height: 0;
  height: 100%;
}
.recent select {
  min-height: 32px;
  padding: 4px 8px;
  border: 1px solid var(--line-strong);
  border-radius: 8px;
  background: var(--surface);
  color: var(--ink);
}
.sr {
  position: absolute;
  width: 1px;
  height: 1px;
  overflow: hidden;
  clip: rect(0 0 0 0);
}
.overlay {
  position: absolute;
  inset: 0;
  z-index: 4;
  display: grid;
  place-items: start center;
  padding: 48px 24px;
  background: color-mix(in srgb, var(--ink) 28%, transparent);
}
.overlay-card {
  width: min(420px, 100%);
  padding: 16px;
  border: 1px solid var(--line-strong);
  border-radius: 12px;
  background: var(--surface);
}
.overlay-card header {
  margin-bottom: 12px;
}
</style>
