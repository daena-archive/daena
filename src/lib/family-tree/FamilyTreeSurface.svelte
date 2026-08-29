<script lang="ts">
import type { EntitySummary, ModuleContext } from "../../../packages/module-api/src/index";
import type { Snippet } from "svelte";
import { trapModalTab } from "$lib/shell/modalFocus";
import FamilyRootPicker from "./FamilyRootPicker.svelte";
import FamilyTreeCanvas from "./FamilyTreeCanvas.svelte";
import { isNeighborhoodAbort, listPersonSecondaryFields, loadGenealogyNeighborhood } from "./fetch";
import { buildElkGraph, LayoutGeneration, positionedFromElk, type LayoutResponse } from "./layout";
import LayoutWorker from "./layout.worker?worker";
import {
  BRANCH_TOO_LARGE,
  DEFAULT_SECONDARY_FIELD,
  PERSON_TYPE,
  type FamilyPerson,
  type GenealogyWarning,
} from "./model";
import { initialNeighborhood, normalizeGenealogy, wouldExceedVisibleLimit } from "./projection";
import { recentRoots, rememberRecentRoot, replaceRecentRoots } from "./state";
import { buildLayoutGraph, layoutGraphExceedsLimits } from "./unions";

let {
  context,
  projectId,
  initialRootId = null,
  avatar,
  onOpenEntity,
  onRootChange,
}: {
  context: ModuleContext;
  projectId: string;
  initialRootId?: string | null;
  avatar?: Snippet<[string, string]>;
  onOpenEntity: (entityId: string) => void;
  onRootChange?: (rootId: string | null) => void;
} = $props();

let rootId = $state<string | null>(null);
let selectedPersonId = $state<string | null>(null);
let people = $state(new Map<string, FamilyPerson>());
let warnings = $state<GenealogyWarning[]>([]);
let loading = $state(false);
let error = $state("");
let layoutFailed = $state(false);
let positioned = $state<ReturnType<typeof positionedFromElk> | null>(null);
let fitToken = $state(0);
let pickerOpen = $state(false);
let recentMenu = $state<{ id: string; name: string }[]>([]);
let secondaryField = $state(DEFAULT_SECONDARY_FIELD);
let secondaryFields = $state<{ key: string; label: string }[]>([{ key: DEFAULT_SECONDARY_FIELD, label: "Occupation" }]);
let overlayEl = $state<HTMLElement | null>(null);
let abort: AbortController | null = null;
const generations = new LayoutGeneration();
let worker: Worker | null = null;
let previousOrder: string[] = [];
let latestLayoutGraph: ReturnType<typeof buildLayoutGraph> | null = null;
let appliedInitial: string | null = null;

function cancelLoad() {
  abort?.abort();
  abort = null;
  generations.start();
}

function logLayoutFailure(generation: number, message?: string) {
  const bounded = (message ?? "layout-worker").replace(/\s+/g, " ").slice(0, 200);
  console.info("family-tree.layout.failed", { generation, code: "layout-worker", message: bounded });
}

function ensureWorker() {
  if (worker) return worker;
  worker = new LayoutWorker();
  worker.onmessage = (event: MessageEvent<LayoutResponse>) => {
    const response = event.data;
    if (!generations.accept(response.generation)) return;
    if (!response.ok || !response.graph) {
      layoutFailed = true;
      logLayoutFailure(response.generation, response.message);
      return;
    }
    const graph = latestLayoutGraph;
    if (!graph) return;
    positioned = positionedFromElk(response.generation, graph, response.graph);
    previousOrder = positioned.nodes.map((node) => node.id);
    layoutFailed = false;
  };
  return worker;
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

async function loadRoot(id: string, fit = true) {
  cancelLoad();
  abort = new AbortController();
  const signal = abort.signal;
  loading = true;
  error = "";
  layoutFailed = false;
  try {
    const loaded = await loadGenealogyNeighborhood(context, id, secondaryField, signal);
    if (signal.aborted) return;
    const { graph, warnings: graphWarnings } = normalizeGenealogy(loaded.people, loaded.relationships);
    const visible = initialNeighborhood(graph, id);
    const candidate = buildLayoutGraph(graph, visible);
    if (wouldExceedVisibleLimit(visible.size) || layoutGraphExceedsLimits(candidate)) {
      error = BRANCH_TOO_LARGE;
      return;
    }
    people = new Map(
      [...visible].flatMap((personId) => {
        const person = graph.people.get(personId);
        return person ? [[personId, person] as const] : [];
      }),
    );
    warnings = [...loaded.warnings, ...graphWarnings];
    latestLayoutGraph = candidate;
    rootId = id;
    selectedPersonId = id;
    pickerOpen = false;
    rememberRecentRoot(projectId, id);
    onRootChange?.(id);
    void refreshRecent();
    const generation = generations.start();
    if (fit) fitToken += 1;
    ensureWorker().postMessage({ generation, graph: buildElkGraph(latestLayoutGraph, previousOrder) });
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
  const generation = generations.start();
  ensureWorker().postMessage({ generation, graph: buildElkGraph(latestLayoutGraph, previousOrder) });
}

function changeRoot() {
  pickerOpen = true;
}

function fitView() {
  fitToken += 1;
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
    void loadRoot(rootId, false);
  }
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
  void listPersonSecondaryFields(context)
    .then((fields) => {
      if (fields.length > 0) secondaryFields = fields;
      if (!secondaryFields.some((field) => field.key === secondaryField)) {
        secondaryField = secondaryFields[0]?.key ?? DEFAULT_SECONDARY_FIELD;
      }
    })
    .catch(() => {});
  void refreshRecent();
});

$effect(() => {
  const next = initialRootId;
  if (!next || next === rootId || next === appliedInitial) return;
  appliedInitial = next;
  void loadRoot(next, true);
});

$effect(() => {
  return () => {
    cancelLoad();
    worker?.terminate();
    worker = null;
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
  {#if error}<p class="banner" role="alert">{error}</p>{/if}
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
    <div class="canvas-wrap">
      <FamilyTreeCanvas
        layout={positioned}
        {people}
        {rootId}
        {selectedPersonId}
        {avatar}
        {onOpenEntity}
        onSelectPerson={(id) => (selectedPersonId = id)}
        onMakeRoot={(id) => {
          previousOrder = [];
          void loadRoot(id, true);
        }}
        {fitToken} />
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
</section>

<style>
.surface {
  position: relative;
  display: grid;
  grid-template-rows: auto auto 1fr;
  gap: 16px;
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
.banner {
  color: var(--theme-warning-text, #55351f);
}
.hint {
  color: var(--ink-muted);
}
.canvas-wrap {
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
