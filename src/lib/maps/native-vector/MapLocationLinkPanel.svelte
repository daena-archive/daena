<script lang="ts">
import { MAP_ENTITY_TYPE, type MapAnchor } from "../../../../packages/plugin-sdk/src/maps";
import { project, type Entity, type MapLocation, type MapPin } from "$lib/project/client";

type CreateTypeOption = { value: string; label: string };

let {
  mapId,
  anchor = $bindable<MapAnchor | null>(null),
  arming = false,
  onclose,
  onresnap,
  onlinked,
}: {
  mapId: string;
  anchor?: MapAnchor | null;
  arming?: boolean;
  onclose: () => void;
  onresnap: () => void;
  onlinked?: (entityId: string) => void;
} = $props();

let mode = $state<"existing" | "create">("existing");
let role = $state("story-location");
let search = $state("");
let entities = $state<Entity[]>([]);
let selectedEntityId = $state<string | null>(null);
let createName = $state("");
let createTypes = $state<CreateTypeOption[]>([]);
let createType = $state("");
let typeLabels = $state<Map<string, string>>(new Map());
let status = $state("");
let busy = $state(false);
let existing = $state<MapPin[]>([]);
let pointX = $state("0.500");
let pointY = $state("0.500");

function entityTypeLabel(entityType: string | null | undefined): string {
  if (!entityType) return "entry";
  return typeLabels.get(entityType) ?? entityType;
}

function preferCreateType(options: CreateTypeOption[]): string {
  return options.find((option) => option.value === "place")?.value ?? options[0]?.value ?? "";
}

async function refreshCreateTypes() {
  const modules = await project.listModuleManifests().catch(() => []);
  const labels = new Map<string, string>();
  const options: CreateTypeOption[] = [];
  const seen = new Set<string>();
  for (const module of modules) {
    if (!module.enabled) continue;
    for (const schema of module.schemas ?? []) {
      for (const entityType of schema.entityTypes ?? []) {
        if (!entityType?.id || entityType.id === MAP_ENTITY_TYPE || seen.has(entityType.id)) continue;
        seen.add(entityType.id);
        const label = entityType.name?.trim() || entityType.id;
        labels.set(entityType.id, label);
        options.push({ value: entityType.id, label });
      }
    }
  }
  options.sort((left, right) => left.label.localeCompare(right.label));
  typeLabels = labels;
  createTypes = options;
  if (!options.some((option) => option.value === createType)) {
    createType = preferCreateType(options);
  }
}

function anchorLabel(value: MapAnchor | null): string {
  if (!value) return arming ? "Click the map to choose a location" : "Nothing selected";
  if (value.kind === "provider-feature") return `${value.featureKind} ${value.featureId}`.trim();
  if (value.kind === "point") return `Point (${value.point[0].toFixed(3)}, ${value.point[1].toFixed(3)})`;
  if (value.kind === "path") return `Path (${value.points.length} points)`;
  if (value.kind === "area") return `Area (${value.rings.length} rings)`;
  return "Selection";
}

function anchorPoint(value: MapAnchor | null): [number, number] | null {
  if (!value) return null;
  if (value.kind === "point") return [value.point[0], value.point[1]];
  if (value.kind === "provider-feature") return [value.fallbackPoint[0], value.fallbackPoint[1]];
  if (value.kind === "path" && value.points[0]) return [value.points[0][0], value.points[0][1]];
  if (value.kind === "area" && value.rings[0]?.[0]) return [value.rings[0][0][0], value.rings[0][0][1]];
  return null;
}

function fillCoordinates(value: MapAnchor | null) {
  const point = anchorPoint(value);
  if (!point) {
    pointX = "0.500";
    pointY = "0.500";
    return;
  }
  pointX = point[0].toFixed(3);
  pointY = point[1].toFixed(3);
}

function applyCoordinates() {
  const x = Number(pointX);
  const y = Number(pointY);
  if (!Number.isFinite(x) || !Number.isFinite(y) || x < 0 || x > 1 || y < 0 || y > 1) {
    status = "Coordinates must be between 0 and 1";
    return;
  }
  anchor = { kind: "point", point: [x, y] };
  status = "";
}

async function refreshEntities() {
  const page = await project
    .queryEntities({
      query: search.trim() || undefined,
      excludedEntityTypes: [MAP_ENTITY_TYPE],
      sortField: "name",
      sortDirection: "asc",
      limit: 120,
    })
    .catch(() => ({ items: [] as Entity[] }));
  entities = page.items ?? [];
  if (!entities.some((entity) => entity.id === selectedEntityId)) {
    selectedEntityId = entities[0]?.id ?? null;
  }
}

async function refreshExisting() {
  existing = await project.listMapPins(mapId).catch(() => []);
}

function buildLocation(label: string): MapLocation {
  if (!anchor) throw new Error("Choose a map location first");
  return {
    id: crypto.randomUUID(),
    mapEntityId: mapId,
    role: role.trim() || "story-location",
    label,
    anchor,
    validity: { from: null, to: null },
  };
}

async function linkExisting() {
  if (!selectedEntityId) {
    status = "Choose an entity";
    return;
  }
  const entity = entities.find((item) => item.id === selectedEntityId);
  if (!entity) {
    status = "Choose an entity";
    return;
  }
  busy = true;
  status = "";
  try {
    await project.upsertMapLocation(entity.id, buildLocation(entity.name));
    await refreshExisting();
    status = "Linked";
    onlinked?.(entity.id);
  } catch (cause) {
    status = cause instanceof Error ? cause.message : String(cause);
  } finally {
    busy = false;
  }
}

async function createAndLink() {
  const name = createName.trim();
  if (!name) {
    status = "Enter a name";
    return;
  }
  if (!createType) {
    status = "No creatable entity types are enabled";
    return;
  }
  busy = true;
  status = "";
  try {
    const created = await project.createEntity(name, createType);
    await project.upsertMapLocation(created.id, buildLocation(created.name));
    createName = "";
    await refreshEntities();
    await refreshExisting();
    status = `Created ${created.name}`;
    onlinked?.(created.id);
  } catch (cause) {
    status = cause instanceof Error ? cause.message : String(cause);
  } finally {
    busy = false;
  }
}

async function unlink(pin: MapPin) {
  busy = true;
  try {
    await project.unlinkMapLocation(pin.entityId, pin.id);
    await refreshExisting();
    status = "Unlinked";
  } catch (cause) {
    status = cause instanceof Error ? cause.message : String(cause);
  } finally {
    busy = false;
  }
}

$effect(() => {
  void refreshCreateTypes();
});

$effect(() => {
  fillCoordinates(anchor);
});

$effect(() => {
  void search;
  void refreshEntities();
});

$effect(() => {
  void mapId;
  void refreshExisting();
});

$effect(() => {
  if (!anchor) return;
  if (anchor.kind === "provider-feature") {
    createName = `${anchor.featureKind} ${anchor.featureId}`.trim();
  } else if (!createName.trim()) {
    createName = "Untitled place";
  }
});</script>

<aside class="map-link-panel" aria-label="Link location">
  <header>
    <div>
      <span>Link location</span>
      <strong>{anchorLabel(anchor)}</strong>
    </div>
    <button type="button" class="quiet" aria-label="Close link panel" onclick={onclose}>×</button>
  </header>

  {#if !arming}
    <div class="coords">
      <label>X<input type="number" min="0" max="1" step="0.001" bind:value={pointX} /></label>
      <label>Y<input type="number" min="0" max="1" step="0.001" bind:value={pointY} /></label>
    </div>
    <div class="row">
      <button type="button" class="quiet" onclick={applyCoordinates}>Apply coordinates</button>
      <button type="button" class="quiet" onclick={onresnap}>Use map click</button>
    </div>

    {#if existing.length > 0}
      <div class="existing-links" aria-label="Existing links on this map">
        {#each existing.slice(0, 8) as pin (pin.id)}
          <div class="existing-row">
            <div>
              <strong>{pin.label || pin.role}</strong>
              <small>{pin.role}</small>
            </div>
            <button type="button" class="quiet" disabled={busy} onclick={() => void unlink(pin)}>Unlink</button>
          </div>
        {/each}
      </div>
    {/if}

    <label class="stack">Role<input type="text" maxlength="64" bind:value={role} /></label>

    <div class="mode-row" role="group" aria-label="Link mode">
      <button type="button" class:active={mode === "existing"} onclick={() => (mode = "existing")}>Existing</button>
      <button type="button" class:active={mode === "create"} onclick={() => (mode = "create")}>Create</button>
    </div>

    {#if mode === "existing"}
      <input type="search" placeholder="Search entities…" bind:value={search} />
      <div class="entity-list" role="listbox" aria-label="Entities">
        {#if entities.length === 0}
          <p>{search.trim() ? "No matching entities." : "No entities to link yet."}</p>
        {:else}
          {#each entities as entity (entity.id)}
            <button
              type="button"
              role="option"
              aria-selected={entity.id === selectedEntityId}
              class:selected={entity.id === selectedEntityId}
              onclick={() => (selectedEntityId = entity.id)}>
              <strong>{entity.name}</strong>
              <small>{entityTypeLabel(entity.entity_type)}</small>
            </button>
          {/each}
        {/if}
      </div>
      <button type="button" class="primary" disabled={busy || !anchor || !selectedEntityId} onclick={() => void linkExisting()}
        >Link entity</button>
    {:else}
      <input type="text" placeholder="New entry name" maxlength="120" bind:value={createName} />
      {#if createTypes.length === 0}
        <p class="hint">Enable a lore module with entity types to create entries from the map.</p>
      {:else}
        <select bind:value={createType} aria-label="Entity type">
          {#each createTypes as option (option.value)}
            <option value={option.value}>{option.label}</option>
          {/each}
        </select>
      {/if}
      <button
        type="button"
        class="primary"
        disabled={busy || !anchor || !createName.trim() || !createType}
        onclick={() => void createAndLink()}>Create and link</button>
    {/if}
  {:else}
    <p class="hint">Click the map to choose a location.</p>
  {/if}

  {#if status}<p class="status" role="status">{status}</p>{/if}
</aside>

<style>
.map-link-panel {
  position: absolute;
  z-index: 6;
  top: 70px;
  right: 14px;
  display: flex;
  width: min(300px, calc(100% - 28px));
  max-height: calc(100% - 90px);
  flex-direction: column;
  gap: 10px;
  overflow: auto;
  padding: 12px;
  border: 1px solid rgb(255 255 255 / 18%);
  border-radius: 10px;
  background: rgb(32 40 36 / 96%);
  color: #f4f1ea;
  box-shadow: 0 8px 24px #0005;
  font: 12px system-ui, sans-serif;
}
header {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 8px;
}
header span {
  display: block;
  opacity: 0.7;
  font-size: 10px;
  letter-spacing: 0.08em;
  text-transform: uppercase;
}
header strong {
  display: block;
  margin-top: 3px;
  font: 600 13px system-ui, sans-serif;
}
.coords {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 8px;
}
.row,
.mode-row {
  display: flex;
  gap: 6px;
}
.stack,
label {
  display: grid;
  gap: 4px;
}
input,
select {
  width: 100%;
  box-sizing: border-box;
  padding: 7px 8px;
  border: 1px solid rgb(255 255 255 / 20%);
  border-radius: 7px;
  background: #1b2420;
  color: #f4f1ea;
  font: 12px system-ui, sans-serif;
}
.entity-list {
  max-height: 240px;
  min-height: 120px;
  overflow: auto;
  padding: 4px;
  border: 1px solid rgb(255 255 255 / 20%);
  border-radius: 7px;
  background: #1b2420;
}
.entity-list p,
.hint,
.status {
  margin: 0;
  opacity: 0.8;
  font-size: 11px;
}
.entity-list button,
.existing-row {
  display: block;
  width: 100%;
  margin: 0 0 3px;
  padding: 8px 9px;
  border: 0;
  border-radius: 6px;
  background: transparent;
  color: #f4f1ea;
  text-align: left;
  font: 12px system-ui, sans-serif;
  cursor: pointer;
}
.entity-list button.selected {
  background: rgb(213 171 108 / 28%);
}
.entity-list strong,
.existing-row strong {
  display: block;
  font-size: 12px;
}
.entity-list small,
.existing-row small {
  opacity: 0.7;
}
.existing-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
  cursor: default;
}
button.quiet,
button.primary,
.mode-row button {
  appearance: none;
  border-radius: 7px;
  padding: 6px 8px;
  font: 12px system-ui, sans-serif;
  cursor: pointer;
}
button.quiet,
.mode-row button {
  flex: 1;
  border: 1px solid rgb(255 255 255 / 20%);
  background: transparent;
  color: #d7ddd6;
}
.mode-row button.active,
button.primary {
  border: 0;
  background: #d5ab6c;
  color: #2c4032;
  font-weight: 700;
}
button.primary {
  width: 100%;
  padding: 8px 10px;
}
button:disabled {
  opacity: 0.45;
  cursor: not-allowed;
}
header > button.quiet {
  flex: 0 0 auto;
  width: auto;
  border: 0;
  padding: 0;
  font-size: 16px;
  line-height: 1;
}
</style>
