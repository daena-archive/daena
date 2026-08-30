<script lang="ts">
import type { EntitySummary, ModuleContext, Relationship, UUID } from "../../../packages/module-api/src/index";
import type { Snippet } from "svelte";
import { untrack } from "svelte";
import { Settings2 } from "@lucide/svelte";
import { promptDialog } from "$lib/dialogs.svelte";
import FamilyMemberDialog from "./FamilyMemberDialog.svelte";
import FamilyPersonPanel from "./FamilyPersonPanel.svelte";
import FamilyRelationshipPanel from "./FamilyRelationshipPanel.svelte";
import FamilyRootPicker from "./FamilyRootPicker.svelte";
import FamilyTreeCanvas from "./FamilyTreeCanvas.svelte";
import {
  isNeighborhoodAbort,
  listPersonSecondaryFields,
  listHouses,
  loadExpansionLayer,
  loadGenealogyNeighborhood,
  loadHouseMemberships,
} from "./fetch.ts";
import { requestElkLayout, terminateElkLayout } from "./elk.ts";
import { buildElkGraph, LayoutGeneration, placeUnions, positionedFromElk } from "./layout";
import {
  BRANCH_TOO_LARGE,
  DEFAULT_SECONDARY_FIELD,
  LIMITS_OVER_BUDGET,
  MAX_ANCESTOR_GENERATIONS,
  MAX_DESCENDANT_GENERATIONS,
  MAX_VISIBLE_PERSON_LIMIT,
  PERSON_TYPE,
  expansionKey,
  familyTreeLimitsOverBudget,
  type BranchDirection,
  type FamilyPerson,
  type FamilyRelationship,
  type FamilyTreeLimits,
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
import {
  readFamilyTreeLimits,
  recentRoots,
  rememberRecentRoot,
  replaceRecentRoots,
  writeFamilyTreeLimits,
} from "./state.ts";
import { createHouse, createMembership } from "./mutations.ts";
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
let recentMenu = $state<{ id: string; name: string }[]>([]);
let secondaryField = $state(DEFAULT_SECONDARY_FIELD);
let secondaryFields = $state<{ key: string; label: string }[]>([{ key: DEFAULT_SECONDARY_FIELD, label: "Occupation" }]);
let limits = $state<FamilyTreeLimits>(readFamilyTreeLimits());
let settingsOpen = $state(false);
let settingsEl = $state<HTMLElement | null>(null);
let expansions = $state<string[]>([]);
let truncated = $state(false);
let member = $state<{ id: string; role: RelativeRole; coParentIds?: string[]; otherId?: string } | null>(null);
let abort: AbortController | null = null;
const generations = new LayoutGeneration();
let previousOrder: string[] = [];
let latestLayoutGraph: ReturnType<typeof buildLayoutGraph> | null = null;
let appliedInitial: string | null = null;
let appliedRestore = 0;
let rawPeople: FamilyPerson[] = [];
let rawRelationships: Relationship[] = [];
let collected = new Map<string, Relationship>();
let houses = $state<{ id: string; name: string }[]>([]);
let houseFilterId = $state<string | null>(null);
let memberships = $state<Map<string, { houseId: string; houseName: string }[]>>(new Map());

const graph = $derived(normalizeGenealogy(rawPeople, rawRelationships).graph);
const selectedRelationship = $derived(
  selectedRelationshipId ? (graph.relationships.get(selectedRelationshipId) ?? null) : null,
);
const selectedPerson = $derived(selectedPersonId ? (people.get(selectedPersonId) ?? null) : null);
const dockOpen = $derived(Boolean(selectedRelationship || selectedPerson));
const personConnections = $derived.by(() => {
  if (!selectedPersonId) return [] as Array<{ id: string; label: string; otherId: string; relationshipId: string }>;
  const items: Array<{ id: string; label: string; otherId: string; relationshipId: string }> = [];
  for (const rel of graph.parentRelationshipsByChild.get(selectedPersonId) ?? []) {
    if (!people.has(rel.sourceId)) continue;
    items.push({
      id: rel.id,
      label: `Parent · ${people.get(rel.sourceId)?.name ?? rel.sourceId}`,
      otherId: rel.sourceId,
      relationshipId: rel.id,
    });
  }
  for (const rel of graph.relationships.values()) {
    if (rel.kind !== "parent" || rel.sourceId !== selectedPersonId || !people.has(rel.targetId)) continue;
    items.push({
      id: rel.id,
      label: `Child · ${people.get(rel.targetId)?.name ?? rel.targetId}`,
      otherId: rel.targetId,
      relationshipId: rel.id,
    });
  }
  const seen = new Set<string>();
  for (const rel of graph.partnerRelationshipsByPerson.get(selectedPersonId) ?? []) {
    if (seen.has(rel.id)) continue;
    seen.add(rel.id);
    const other = rel.sourceId === selectedPersonId ? rel.targetId : rel.sourceId;
    if (!people.has(other)) continue;
    items.push({
      id: rel.id,
      label: `Partner · ${people.get(other)?.name ?? other}`,
      otherId: other,
      relationshipId: rel.id,
    });
  }
  return items;
});
const hiddenByPerson = $derived.by(() => {
  const visible = new Set(people.keys());
  const map = new Map<string, HiddenCounts>();
  for (const id of visible) map.set(id, hiddenCounts(graph, id, visible, truncated, truncationLowerBound));
  return map;
});
const housesByPerson = $derived.by(() => {
  const map = new Map<string, string[]>();
  for (const [personId, entries] of memberships) {
    const names = [...new Set(entries.map((entry) => entry.houseName))];
    if (names.length) map.set(personId, names);
  }
  return map;
});
const houseFilterIdsByPerson = $derived.by(() => {
  const map = new Map<string, string[]>();
  for (const [personId, entries] of memberships) {
    map.set(
      personId,
      entries.map((entry) => entry.houseId),
    );
  }
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

async function refreshHouses() {
  try {
    houses = await listHouses(context);
  } catch {
    houses = [];
  }
}

async function refreshMemberships(ids: string[]) {
  try {
    const loaded = await loadHouseMemberships(context, ids);
    const next = new Map<string, { houseId: string; houseName: string }[]>();
    for (const entry of loaded) {
      const list = next.get(entry.personId) ?? [];
      list.push({ houseId: entry.houseId, houseName: entry.houseName });
      next.set(entry.personId, list);
    }
    memberships = next;
  } catch {
    memberships = new Map();
  }
}

async function createHouseFromToolbar() {
  const name = await promptDialog({
    title: "New house",
    message: "Create a house to group people. Membership does not change the tree layout.",
    placeholder: "House name",
    confirmLabel: "Create",
  });
  if (!name?.trim()) return;
  try {
    const created = await createHouse(context, name.trim(), crypto.randomUUID());
    const personId = selectedPersonId ?? rootId;
    const person = personId ? people.get(personId) : null;
    if (person) {
      await createMembership(context, person.id, created.id, person.revision, crypto.randomUUID());
    }
    await refreshHouses();
    await refreshMemberships([...people.keys()]);
    houseFilterId = created.id;
  } catch (cause) {
    error = cause instanceof Error ? cause.message : String(cause);
  }
}

function onHouseFilterChange(event: Event) {
  const value = (event.currentTarget as HTMLSelectElement).value;
  houseFilterId = value || null;
}

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
  if (wouldExceedVisibleLimit(visible.size, limits.visiblePersonLimit) || layoutGraphExceedsLimits(candidate, limits)) {
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
  void refreshMemberships([...people.keys()]);
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
    const loaded = await loadGenealogyNeighborhood(context, id, secondaryField, signal, limits);
    if (signal.aborted) return;
    collected = new Map(loaded.relationships.map((relationship) => [relationship.id, relationship]));
    rawPeople = loaded.people;
    rawRelationships = loaded.relationships;
    truncated = loaded.truncated;
    truncationLowerBound = loaded.truncationLowerBound;
    const { graph: nextGraph, warnings: graphWarnings } = normalizeGenealogy(loaded.people, loaded.relationships);
    const nextExpansions = restored ?? [
      ...seedInitialExpansions(nextGraph, id, limits.ancestorGenerations, limits.descendantGenerations),
    ];
    const visible = restored
      ? visibleFromExpansions(nextGraph, id, nextExpansions).visible
      : initialNeighborhood(nextGraph, id, limits.ancestorGenerations, limits.descendantGenerations);
    const candidate = buildLayoutGraph(nextGraph, visible);
    if (
      wouldExceedVisibleLimit(visible.size, limits.visiblePersonLimit) ||
      layoutGraphExceedsLimits(candidate, limits)
    ) {
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
    void refreshHouses();
    void refreshMemberships([...people.keys()]);
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

function applyLimits(next: Partial<FamilyTreeLimits>, reload: boolean) {
  limits = writeFamilyTreeLimits(next);
  if (!rootId) return;
  previousOrder = [...(positioned?.nodes.map((node) => node.id) ?? [])];
  if (reload) void loadRoot(rootId, true);
  else applyVisible(expansions, false);
}

function onAncestorChange(event: Event) {
  const value = Number((event.currentTarget as HTMLInputElement).value);
  applyLimits({ ...limits, ancestorGenerations: value, maxExpansionDepth: undefined }, true);
}

function onDescendantChange(event: Event) {
  const value = Number((event.currentTarget as HTMLInputElement).value);
  applyLimits({ ...limits, descendantGenerations: value, maxExpansionDepth: undefined }, true);
}

function onPersonCapChange(event: Event) {
  const value = Number((event.currentTarget as HTMLInputElement).value);
  applyLimits(
    { ...limits, visiblePersonLimit: value, visibleUnionLimit: undefined, visibleEdgeLimit: undefined },
    false,
  );
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
  if (expansionBlocked(graph, rootId, personId, direction, limits.maxExpansionDepth)) {
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

$effect(() => {
  if (!settingsOpen) return;
  function onPointer(event: PointerEvent) {
    if (settingsEl?.contains(event.target as Node)) return;
    settingsOpen = false;
  }
  window.addEventListener("pointerdown", onPointer);
  return () => window.removeEventListener("pointerdown", onPointer);
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

function selectCanvasPerson(id: string | null) {
  selectedPersonId = id;
  if (id) selectedRelationshipId = null;
}

function applyRelationshipUpdate(relationship: FamilyRelationship) {
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
}

function applyRelationshipDelete(id: string) {
  collected.delete(id);
  rawRelationships = [...collected.values()];
  selectedRelationshipId = null;
  applyVisible(expansions, false);
}
</script>

<section class="surface">
  <header>
    <div>
      <span class="overline">FAMILY TREE</span>
      <h1>{rootId ? (people.get(rootId)?.name ?? "Family Tree") : "Family Tree"}</h1>
      {#if !rootId}
        <p>Choose a Lore person to inspect their family neighborhood.</p>
      {/if}
    </div>
    {#if rootId}
      <div class="toolbar">
        <FamilyRootPicker {context} compact dropdown recents={recentMenu} onSelect={selectRoot} />
        <label class="recent">
          <span class="sr">Filter by house</span>
          <select aria-label="Filter by house" value={houseFilterId ?? ""} onchange={onHouseFilterChange}>
            <option value="">All houses</option>
            {#each houses as house (house.id)}
              <option value={house.id}>{house.name}</option>
            {/each}
          </select>
        </label>
        <div class="settings" bind:this={settingsEl}>
          <button
            type="button"
            class="quiet-button icon"
            aria-expanded={settingsOpen}
            aria-label="View settings"
            onclick={() => (settingsOpen = !settingsOpen)}>
            <Settings2 size={16} />
          </button>
          {#if settingsOpen}
            <div class="settings-panel" role="dialog" aria-label="View settings">
              <label class="recent">
                <span>Secondary field</span>
                <select aria-label="Secondary field" value={secondaryField} onchange={onSecondaryChange}>
                  {#each secondaryFields as field (field.key)}
                    <option value={field.key}>{field.label}</option>
                  {/each}
                </select>
              </label>
              <label class="recent limit">
                <span>Ancestor generations</span>
                <input
                  type="number"
                  min="1"
                  max={MAX_ANCESTOR_GENERATIONS}
                  value={limits.ancestorGenerations}
                  aria-label="Ancestor generations"
                  onchange={onAncestorChange} />
              </label>
              <label class="recent limit">
                <span>Descendant generations</span>
                <input
                  type="number"
                  min="1"
                  max={MAX_DESCENDANT_GENERATIONS}
                  value={limits.descendantGenerations}
                  aria-label="Descendant generations"
                  onchange={onDescendantChange} />
              </label>
              <label class="recent limit">
                <span>Visible people cap</span>
                <input
                  type="number"
                  min="1"
                  max={MAX_VISIBLE_PERSON_LIMIT}
                  step="50"
                  value={limits.visiblePersonLimit}
                  aria-label="Visible people cap"
                  onchange={onPersonCapChange} />
              </label>
              <button type="button" class="quiet-button" onclick={() => void createHouseFromToolbar()}
                >New house</button>
            </div>
          {/if}
        </div>
        <button type="button" class="quiet-button" onclick={fitView}>Fit</button>
        <button type="button" class="quiet-button" onclick={resetView}>Reset</button>
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
  {#if familyTreeLimitsOverBudget(limits)}
    <p class="hint">{LIMITS_OVER_BUDGET}</p>
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
    <div class="workspace" class:has-dock={dockOpen}>
      <div class="canvas-wrap">
        <FamilyTreeCanvas
          layout={positioned}
          {people}
          {rootId}
          {selectedPersonId}
          {selectedRelationshipId}
          {hiddenByPerson}
          {expandedByPerson}
          {housesByPerson}
          memberHouseIds={houseFilterIdsByPerson}
          {houseFilterId}
          {avatar}
          onSelectPerson={selectCanvasPerson}
          onSelectRelationship={(id) => (selectedRelationshipId = id)}
          onMakeRoot={(id) => {
            previousOrder = [];
            void loadRoot(id, true);
          }}
          onToggleBranch={(id, direction) => void toggleBranch(id, direction)}
          onAddUnionChild={(memberIds) => {
            const [id, ...coParentIds] = memberIds.filter((personId) => people.has(personId));
            if (!id) return;
            member = { id, role: "child", coParentIds };
          }}
          onLinkPartners={(memberIds) => {
            member = { id: memberIds[0], role: "partner", otherId: memberIds[1] };
          }}
          {fitToken}
          fitView={canvasFit}
          initialViewport={canvasFit ? null : viewport}
          onViewportChange={(next) => {
            if (viewport && viewport.x === next.x && viewport.y === next.y && viewport.zoom === next.zoom) return;
            viewport = next;
          }} />
      </div>
      {#if dockOpen}
        <div class="dock">
          {#if selectedRelationship}
            <FamilyRelationshipPanel
              docked
              {context}
              relationship={selectedRelationship}
              {people}
              onClose={() => (selectedRelationshipId = null)}
              onUpdated={applyRelationshipUpdate}
              onDeleted={applyRelationshipDelete} />
          {:else if selectedPerson}
            <FamilyPersonPanel
              person={selectedPerson}
              isRoot={selectedPerson.id === rootId}
              houses={housesByPerson.get(selectedPerson.id) ?? []}
              connections={personConnections}
              onOpen={onOpenEntity}
              onMakeRoot={(id) => {
                previousOrder = [];
                void loadRoot(id, true);
              }}
              onAddRelative={(id, role) => (member = { id, role })}
              onSelectPerson={selectCanvasPerson}
              onSelectRelationship={(id) => (selectedRelationshipId = id)}
              onClose={() => (selectedPersonId = null)} />
          {/if}
        </div>
      {/if}
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
      otherPerson={member.otherId
        ? {
            id: member.otherId as UUID,
            name: people.get(member.otherId)?.name ?? member.otherId,
            type: PERSON_TYPE,
            deleted: false,
            revision: people.get(member.otherId)?.revision ?? "",
          }
        : null}
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
  gap: 12px;
  width: 100%;
  height: 100%;
  min-height: 0;
  padding: 12px 16px;
}
header {
  display: flex;
  flex-wrap: wrap;
  align-items: flex-end;
  justify-content: space-between;
  gap: 12px;
}
.overline {
  display: block;
  color: var(--accent);
  font-size: 10px;
  font-weight: 800;
  letter-spacing: 0.18em;
}
h1 {
  margin: 2px 0 0;
  color: var(--ink);
  font: 500 22px/1.15 var(--font-display, Georgia, serif);
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
.quiet-button.icon {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  padding: 6px;
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
  grid-template-columns: minmax(0, 1fr);
  gap: 12px;
  min-height: 0;
  height: 100%;
}
.workspace.has-dock {
  grid-template-columns: minmax(0, 1fr) minmax(240px, 280px);
}
.canvas-wrap,
.dock {
  display: flex;
  flex-direction: column;
  min-height: 0;
  height: 100%;
}
.dock {
  overflow: hidden;
  border: 1px solid var(--line-strong);
  border-radius: 12px;
  background: var(--surface);
}
.settings {
  position: relative;
}
.settings-panel {
  position: absolute;
  top: calc(100% + 6px);
  right: 0;
  z-index: 6;
  display: grid;
  gap: 10px;
  width: 240px;
  padding: 12px;
  border: 1px solid var(--line-strong);
  border-radius: 10px;
  background: var(--surface);
}
.settings-panel .recent,
.settings-panel .recent.limit {
  display: grid;
  gap: 4px;
}
.settings-panel .recent span {
  color: var(--ink-muted);
  font-size: 11px;
  font-weight: 700;
}
.recent select,
.recent input {
  min-height: 32px;
  padding: 4px 8px;
  border: 1px solid var(--line-strong);
  border-radius: 8px;
  background: var(--surface);
  color: var(--ink);
}
.recent.limit input {
  width: 100%;
}
.sr {
  position: absolute;
  width: 1px;
  height: 1px;
  overflow: hidden;
  clip: rect(0 0 0 0);
}
</style>
