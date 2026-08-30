<script lang="ts">
import type { EntitySummary, ModuleContext, Relationship, UUID } from "../../../packages/module-api/src/index";
import type { Snippet } from "svelte";
import { untrack } from "svelte";
import { ArrowLeft, Maximize2, RotateCcw, Settings2, UsersRound } from "@lucide/svelte";
import { promptDialog } from "$lib/dialogs.svelte";
import WorkbenchState from "$lib/shell/WorkbenchState.svelte";
import FamilyMemberDialog from "./FamilyMemberDialog.svelte";
import FamilyPersonPanel from "./FamilyPersonPanel.svelte";
import FamilyRelationshipPanel from "./FamilyRelationshipPanel.svelte";
import FamilyRootPicker from "./FamilyRootPicker.svelte";
import FamilyTreeLanding from "./FamilyTreeLanding.svelte";
import FamilyTreeCanvas from "./FamilyTreeCanvas.svelte";
import {
  isNeighborhoodAbort,
  listPersonSecondaryFields,
  listHouses,
  loadExpansionLayer,
  loadGenealogyNeighborhood,
  loadHouseMemberships,
  loadHouseNeighborhood,
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
let houseId = $state<string | null>(null);
let houseName = $state("");
let memberships = $state<Map<string, { houseId: string; houseName: string }[]>>(new Map());

const graph = $derived(normalizeGenealogy(rawPeople, rawRelationships).graph);
const selectedRelationship = $derived(
  selectedRelationshipId ? (graph.relationships.get(selectedRelationshipId) ?? null) : null,
);
const selectedPerson = $derived(selectedPersonId ? (people.get(selectedPersonId) ?? null) : null);
const dockOpen = $derived(Boolean(selectedRelationship || selectedPerson));
const subtitle = $derived.by(() => {
  if (!rootId) return "Choose a person or house to explore a family neighborhood.";
  if (houseId) return houseName ? `${houseName} · ${people.size} members` : `${people.size} members`;
  const parts: string[] = [`${people.size} in view`];
  if (truncated) parts.push(`truncated ${truncationLowerBound ? `(${truncationLowerBound}+)` : ""}`.trim());
  if (warnings.length) parts.push(`${warnings.length} warning${warnings.length === 1 ? "" : "s"}`);
  if (familyTreeLimitsOverBudget(limits)) parts.push("over budget");
  return parts.join(" · ");
});
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
  } catch (cause) {
    error = cause instanceof Error ? cause.message : String(cause);
  }
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
  const { visible } = houseId
    ? { visible: new Set(nextGraph.people.keys()) }
    : visibleFromExpansions(nextGraph, rootId, nextExpansions, protect);
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
    houseId = null;
    houseName = "";
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

async function loadHouse(id: string, fit = true, restored = false, name = "") {
  cancelLoad();
  abort = new AbortController();
  const signal = abort.signal;
  loading = true;
  error = "";
  layoutFailed = false;
  try {
    let resolvedName = name || houses.find((house) => house.id === id)?.name || "";
    if (!resolvedName) {
      try {
        const entity = await context.entities.get(id as UUID);
        if (entity && !entity.deleted) resolvedName = entity.name;
      } catch {
        resolvedName = "";
      }
    }
    const loaded = await loadHouseNeighborhood(context, id, secondaryField, signal);
    if (signal.aborted) return;
    collected = new Map(loaded.relationships.map((relationship) => [relationship.id, relationship]));
    rawPeople = loaded.people;
    rawRelationships = loaded.relationships;
    truncated = loaded.truncated;
    truncationLowerBound = loaded.truncationLowerBound;
    const { graph: nextGraph, warnings: graphWarnings } = normalizeGenealogy(loaded.people, loaded.relationships);
    const visible = new Set(nextGraph.people.keys());
    const candidate = buildLayoutGraph(nextGraph, visible);
    if (
      wouldExceedVisibleLimit(visible.size, limits.visiblePersonLimit) ||
      layoutGraphExceedsLimits(candidate, limits)
    ) {
      error = BRANCH_TOO_LARGE;
      rerootCandidate = null;
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
    latestLayoutGraph = visible.size > 0 ? candidate : null;
    expansions = [];
    rootId = id;
    houseId = id;
    houseName = resolvedName;
    selectedPersonId = restored ? (initialSession?.selectedPersonId ?? null) : null;
    selectedRelationshipId = restored ? (initialSession?.selectedRelationshipId ?? null) : null;
    onRootChange?.(id);
    canvasFit = fit;
    if (fit) {
      viewport = null;
      fitToken += 1;
    } else if (restored && initialSession?.viewport) {
      viewport = initialSession.viewport;
    }
    if (visible.size > 0) requestLayout();
    else positioned = null;
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

function selectHouse(house: { id: string; name: string }) {
  previousOrder = [];
  void loadHouse(house.id, true, false, house.name);
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
  if (houseId) void loadHouse(houseId, true, false, houseName);
  else void loadRoot(rootId, true);
}

function onSecondaryChange(event: Event) {
  const value = (event.currentTarget as HTMLSelectElement).value;
  secondaryField = value;
  if (rootId) {
    previousOrder = [...(positioned?.nodes.map((node) => node.id) ?? [])];
    if (houseId) void loadHouse(houseId, false, false, houseName);
    else void loadRoot(rootId, false, expansions);
  }
}

function applyLimits(next: Partial<FamilyTreeLimits>, reload: boolean) {
  limits = writeFamilyTreeLimits(next);
  if (!rootId) return;
  previousOrder = [...(positioned?.nodes.map((node) => node.id) ?? [])];
  if (reload) {
    if (houseId) void loadHouse(houseId, true, false, houseName);
    else void loadRoot(rootId, true);
  } else applyVisible(expansions, false);
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
  if (!rootId || houseId) return;
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
  if (houseId) await loadHouse(houseId, false, false, houseName);
  else await loadRoot(rootId, false, nextExpansions);
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

function goToLanding() {
  if (!rootId) return;
  previousOrder = [];
  clearView();
  onRootChange?.(null);
}

function clearView() {
  cancelLoad();
  loading = false;
  error = "";
  layoutFailed = false;
  rootId = null;
  houseId = null;
  houseName = "";
  selectedPersonId = null;
  selectedRelationshipId = null;
  people = new Map();
  warnings = [];
  positioned = null;
  latestLayoutGraph = null;
  expansions = [];
  truncated = false;
  truncationLowerBound = 0;
  rawPeople = [];
  rawRelationships = [];
  collected = new Map();
  rerootCandidate = null;
  viewport = null;
  member = null;
  previousOrder = [];
  appliedInitial = null;
}

$effect(() => {
  const next = initialRootId;
  if (restoreNonce !== appliedRestore) {
    appliedRestore = restoreNonce;
    appliedInitial = next ?? null;
    previousOrder = [];
    if (!next) {
      clearView();
      return;
    }
    if (initialSession?.houseId === next) void loadHouse(next, !initialSession?.viewport, true);
    else void loadRoot(next, !initialSession?.viewport, initialSession?.expansions ?? null);
    return;
  }
  if (!next) {
    if (rootId) clearView();
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
        houseId,
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
  <!-- Topbar — mirrors WorkspaceTopbar language -->
  <div class="family-topbar" role="banner">
    <div class="family-topbar-main">
      <span class="family-mark" aria-hidden="true"><UsersRound size={16} strokeWidth={1.8} /></span>
      <div class="family-copy">
        <strong
          >{houseId
            ? houseName || "House"
            : rootId
              ? (people.get(rootId)?.name ?? "Family Tree")
              : "Family Tree"}</strong>
        <small title={subtitle}>{subtitle}</small>
      </div>
      {#if loading && positioned}
        <span class="topbar-busy" aria-live="polite" aria-label="Updating family tree">
          <span class="busy-dot" aria-hidden="true"></span> Updating…
        </span>
      {/if}
    </div>
    {#if rootId}
      <div class="family-topbar-actions" role="toolbar" aria-label="Tree view actions">
        <button type="button" class="workspace-topbar-action" onclick={goToLanding} title="Back to people and houses">
          <ArrowLeft size={14} strokeWidth={1.8} aria-hidden="true" /> Trees
        </button>
        <button type="button" class="workspace-topbar-action" onclick={fitView} title="Fit to view">
          <Maximize2 size={14} strokeWidth={1.8} aria-hidden="true" /> Fit
        </button>
        <button type="button" class="workspace-topbar-action" onclick={resetView} title="Reset to initial neighborhood">
          <RotateCcw size={14} strokeWidth={1.8} aria-hidden="true" /> Reset
        </button>
        <div class="settings" bind:this={settingsEl}>
          <button
            type="button"
            class="workspace-topbar-action icon"
            aria-expanded={settingsOpen}
            aria-label="View settings"
            onclick={() => (settingsOpen = !settingsOpen)}>
            <Settings2 size={16} strokeWidth={1.8} aria-hidden="true" />
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
                <span>Ancestor generations <em>{limits.ancestorGenerations}</em></span>
                <input
                  type="range"
                  min="1"
                  max={MAX_ANCESTOR_GENERATIONS}
                  step="1"
                  value={limits.ancestorGenerations}
                  aria-label="Ancestor generations"
                  oninput={onAncestorChange} />
              </label>
              <label class="recent limit">
                <span>Descendant generations <em>{limits.descendantGenerations}</em></span>
                <input
                  type="range"
                  min="1"
                  max={MAX_DESCENDANT_GENERATIONS}
                  step="1"
                  value={limits.descendantGenerations}
                  aria-label="Descendant generations"
                  oninput={onDescendantChange} />
              </label>
              <label class="recent limit">
                <span>Visible people cap <em>{limits.visiblePersonLimit}</em></span>
                <input
                  type="range"
                  min="50"
                  max={MAX_VISIBLE_PERSON_LIMIT}
                  step="50"
                  value={limits.visiblePersonLimit}
                  aria-label="Visible people cap"
                  oninput={onPersonCapChange} />
              </label>
              <div class="settings-footnote">
                <small
                  >Partners and siblings are included with each visible person. Raising the cap keeps more of the
                  neighborhood in view.</small>
                {#if familyTreeLimitsOverBudget(limits)}
                  <small>{LIMITS_OVER_BUDGET}</small>
                {/if}
                {#if warnings.length > 0}
                  <small
                    >{warnings.length} data warning{warnings.length === 1 ? "" : "s"} — unresolved or unknown family edges
                    were skipped.</small>
                {/if}
                {#if truncated}
                  <small
                    >Relationship query truncated — counts show a lower bound ({truncationLowerBound
                      ? `${truncationLowerBound}+`
                      : "99+"}).</small>
                {/if}
              </div>
              <button type="button" class="quiet-button pill" onclick={() => void createHouseFromToolbar()}
                >New house</button>
            </div>
          {/if}
        </div>
      </div>
    {/if}
  </div>

  {#if rootId}
    <!-- Sticky sub-toolbar — filters & pickers (aligns to WorkspaceViewNav) -->
    <div class="family-subbar" role="toolbar" aria-label="Family filters">
      <div class="subbar-group picker-group">
        <span class="subbar-label">Root</span>
        <FamilyRootPicker {context} compact dropdown recents={recentMenu} onSelect={selectRoot} />
      </div>
      <div class="subbar-sep" aria-hidden="true"></div>
      <div class="subbar-group field-group">
        <label class="subbar-field">
          <span>Secondary</span>
          <select aria-label="Secondary field" value={secondaryField} onchange={onSecondaryChange}>
            {#each secondaryFields as field (field.key)}
              <option value={field.key}>{field.label}</option>
            {/each}
          </select>
        </label>
      </div>
      <div class="subbar-legend" aria-hidden="true">
        <span class="legend-swatch solid"></span> Parent
        <span class="legend-swatch dash-adopt"></span> Adoptive
        <span class="legend-swatch double"></span> Partner
      </div>
    </div>
  {/if}

  {#if rootId && (error || layoutFailed)}
    <div class="family-status">
      {#if error}
        <div class="status-alert error" role="alert">
          <span class="status-dot"></span>
          <span class="status-text">{error}</span>
          {#if rerootCandidate}
            <button type="button" class="quiet-button pill" onclick={acceptReroot}
              >Make {rerootCandidate.name} root</button>
          {/if}
          {#if error === BRANCH_TOO_LARGE}
            <button type="button" class="quiet-button pill" onclick={() => (error = "")}>Dismiss</button>
          {/if}
        </div>
      {/if}
      {#if layoutFailed}
        <div class="status-alert warning" role="alert">
          <span class="status-dot"></span>
          <span class="status-text">Layout failed. The previous arrangement was kept.</span>
          <button type="button" class="quiet-button pill" onclick={retryLayout}>Retry</button>
        </div>
      {/if}
    </div>
  {/if}

  <div class="family-body">
    {#if !rootId}
      <FamilyTreeLanding {context} {avatar} onSelect={selectRoot} onSelectHouse={selectHouse} />
    {:else if loading && !positioned}
      <WorkbenchState
        kind="loading"
        title="Loading family neighborhood"
        message="Fetching people, relationships, and houses…" />
    {:else if error && !positioned}
      <WorkbenchState kind="error" title="Could not load family" message={error}>
        {#snippet actions()}
          {#if rerootCandidate}
            <button type="button" class="quiet-button" onclick={acceptReroot}>Make {rerootCandidate.name} root</button>
          {/if}
          <button type="button" class="quiet-button" onclick={() => (error = "")}>Dismiss</button>
        {/snippet}
      </WorkbenchState>
    {:else if houseId && people.size === 0}
      <WorkbenchState
        kind="empty"
        title="This house has no members"
        message="Add people from a person neighborhood, then open the house again to see its kinship tree." />
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
  </div>

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
  width: 100%;
  height: 100%;
  min-height: 0;
  background: var(--surface, #f4f5f2);
}
/* Topbar — aligns to WorkspaceTopbar 10px 18px / 58px */
.family-topbar {
  z-index: 3;
  display: grid;
  grid-template-columns: minmax(0, 1fr) auto;
  align-items: center;
  gap: 16px;
  padding: 10px 18px;
  border-bottom: 1px solid var(--theme-neutral-border, var(--line));
  background: color-mix(in srgb, var(--surface) 96%, transparent);
  box-shadow: 0 1px 8px rgba(30, 37, 31, 0.03);
  backdrop-filter: blur(8px);
}
.family-topbar-main {
  display: flex;
  align-items: center;
  gap: 10px;
  min-width: 0;
}
.family-mark {
  display: grid;
  width: 31px;
  height: 31px;
  flex: 0 0 31px;
  place-items: center;
  border-radius: 8px;
  background: var(--theme-success-bg, var(--accent-bg, #e4ece4));
  color: var(--theme-success-text, var(--accent-dark, #2f4e35));
}
.family-copy {
  display: grid;
  min-width: 0;
  gap: 2px;
}
.family-copy strong {
  overflow: hidden;
  color: var(--theme-neutral-text, var(--ink));
  font: 600 12px/1.2 var(--font-body, Inter, ui-sans-serif, system-ui, sans-serif);
  text-overflow: ellipsis;
  white-space: nowrap;
}
.family-copy small {
  overflow: hidden;
  color: var(--theme-neutral-text-muted, var(--ink-muted));
  font-size: 9px;
  line-height: 1.2;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.topbar-busy {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  margin-left: 8px;
  color: var(--ink-faint, #899088);
  font-size: 10px;
}
.busy-dot {
  width: 7px;
  height: 7px;
  border-radius: 50%;
  background: var(--accent, #d6a35f);
  animation: pulse 1.1s ease-in-out infinite;
}
@keyframes pulse {
  0%,
  100% {
    opacity: 0.55;
  }
  50% {
    opacity: 1;
  }
}
.family-topbar-actions {
  display: flex;
  align-items: center;
  gap: 8px;
  flex-wrap: wrap;
  justify-content: flex-end;
}
/* Sub-bar — sticky like WorkspaceViewNav */
.family-subbar {
  position: sticky;
  top: 0;
  z-index: 2;
  display: flex;
  align-items: center;
  gap: 14px;
  min-height: 46px;
  padding: 7px 18px;
  border-bottom: 1px solid var(--line);
  background: color-mix(in srgb, var(--surface) 92%, transparent);
  backdrop-filter: blur(14px);
  overflow-x: auto;
  scrollbar-width: none;
}
.family-subbar::-webkit-scrollbar {
  display: none;
}
.subbar-group {
  display: flex;
  align-items: center;
  gap: 8px;
  flex: 0 0 auto;
}
.subbar-label {
  color: var(--theme-neutral-text-muted, var(--ink-muted));
  font-size: 10px;
  font-weight: 700;
  letter-spacing: 0.06em;
  text-transform: uppercase;
  white-space: nowrap;
}
.subbar-sep {
  width: 1px;
  height: 22px;
  flex: 0 0 1px;
  background: var(--line);
}
.subbar-field {
  display: flex;
  align-items: center;
  gap: 6px;
  color: var(--ink);
  font-size: 12px;
  white-space: nowrap;
}
.subbar-field span {
  color: var(--ink-muted);
  font-size: 10px;
  font-weight: 700;
  letter-spacing: 0.04em;
  text-transform: uppercase;
}
.subbar-field select {
  min-height: 32px;
  padding: 4px 8px;
  border: 1px solid var(--line-strong);
  border-radius: 8px;
  background: var(--surface);
  color: var(--ink);
  font-size: 12px;
}
.subbar-legend {
  display: flex;
  align-items: center;
  gap: 6px 10px;
  margin-left: auto;
  color: var(--ink-muted);
  font-size: 10px;
  white-space: nowrap;
}
.legend-swatch {
  display: inline-block;
  width: 14px;
  height: 0;
  border-top: 2px solid var(--ink);
  vertical-align: middle;
}
.legend-swatch.solid {
  border-top-style: solid;
}
.legend-swatch.dash-adopt {
  border-top: 2px dashed var(--ink);
}
.legend-swatch.double {
  width: 14px;
  height: 4px;
  border: 1.5px solid var(--ink);
  border-radius: 2px;
  background: transparent;
}
:global(.workspace-topbar-action) {
  display: inline-flex;
  min-height: 34px;
  align-items: center;
  justify-content: center;
  gap: 6px;
  padding: 0 10px;
  border: 1px solid var(--theme-neutral-border, #d9ddd6);
  border-radius: 8px;
  background: var(--theme-surface-bg, var(--surface));
  color: var(--theme-neutral-text-soft, #4d584f);
  font: 650 11px/1 var(--font-body, Inter, ui-sans-serif, system-ui, sans-serif);
  cursor: pointer;
}
:global(.workspace-topbar-action.icon) {
  width: 34px;
  min-width: 34px;
  padding: 0;
}
:global(.workspace-topbar-action:hover),
:global(.workspace-topbar-action:focus-visible) {
  border-color: var(--theme-neutral-border-strong, #b9c4ba);
  background: var(--theme-success-bg, #f2f6f2);
  color: var(--theme-success-text, #2f4e35);
  outline: 0;
}
/* Status strip */
.family-status {
  display: grid;
  gap: 0;
}
.status-alert {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 8px;
  padding: 8px 18px;
  border-bottom: 1px solid var(--line-soft, var(--line));
  font-size: 11px;
  line-height: 1.45;
}
.status-alert.neutral {
  background: var(--surface-muted, var(--surface));
  color: var(--ink-muted);
}
.status-alert.warning {
  background: var(--theme-warning-bg, #fff8ee);
  color: var(--theme-warning-text, #55351f);
  border-color: var(--theme-warning-border, #d8c3a5);
}
.status-alert.error {
  background: var(--danger-bg, #fff2ee);
  color: var(--danger, #8a2b2b);
  border-color: var(--danger-line, #edcec5);
}
.status-dot {
  width: 7px;
  height: 7px;
  flex: 0 0 7px;
  border-radius: 50%;
  background: currentColor;
  opacity: 0.9;
}
.status-text {
  flex: 1 1 auto;
  min-width: 160px;
}
.pill {
  border-radius: 999px !important;
}
/* Body */
.family-body {
  display: flex;
  flex: 1 1 auto;
  flex-direction: column;
  min-height: 0;
  overflow: auto;
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
.workspace {
  display: grid;
  flex: 1 1 auto;
  grid-template-columns: minmax(0, 1fr);
  gap: 12px;
  min-height: 0;
  height: 100%;
  padding: 12px 16px;
}
.workspace.has-dock {
  grid-template-columns: minmax(0, 1fr) minmax(280px, 320px);
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
  box-shadow: var(--shadow-sm, 0 1px 2px rgba(0, 0, 0, 0.04));
}
.settings {
  position: relative;
}
.settings-panel {
  position: absolute;
  top: calc(100% + 8px);
  right: 0;
  z-index: 6;
  display: grid;
  gap: 10px;
  width: 260px;
  padding: 12px;
  border: 1px solid var(--line-strong);
  border-radius: 12px;
  background: var(--surface);
  box-shadow: var(--shadow-lg, 0 8px 24px rgba(0, 0, 0, 0.08));
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
.settings-panel .recent span em {
  margin-left: 6px;
  color: var(--ink);
  font-style: normal;
  font-weight: 800;
}
.recent input[type="range"] {
  width: 100%;
  accent-color: var(--accent-dark, var(--accent));
}
.settings-footnote {
  display: grid;
  gap: 6px;
}
.settings-footnote small {
  color: var(--ink-muted);
  font-size: 10px;
  line-height: 1.4;
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
@media (max-width: 900px) {
  .family-topbar {
    grid-template-columns: 1fr;
    gap: 8px;
  }
  .family-subbar {
    padding-inline: 12px;
    gap: 10px;
  }
  .subbar-legend {
    display: none;
  }
  .workspace {
    padding: 10px 12px;
  }
  .workspace.has-dock {
    grid-template-columns: 1fr;
  }
  .dock {
    max-height: min(45vh, 520px);
  }
}
@media (prefers-reduced-motion: reduce) {
  .busy-dot {
    animation: none;
  }
}
</style>
