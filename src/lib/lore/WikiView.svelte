<script lang="ts">
import { onMount } from "svelte";
import {
  ArrowLeft,
  ArrowRight,
  BookOpen,
  Diamond,
  Link2,
  MapPinned,
  Paperclip,
  Pencil,
  Sparkles,
} from "@lucide/svelte";
import MarkdownArticle from "$lib/markdown/MarkdownArticle.svelte";
import { headingOutline } from "$lib/markdown";
import { project, type Asset, type Entity, type ModuleManifest } from "$lib/project/client";
import { formatCalendarDate, parseCalendarDate } from "$lib/date";
import WikiExportMenu from "./WikiExportMenu.svelte";
import WikiSidebar from "./WikiSidebar.svelte";

let {
  manifest,
  initialEntityId = null as string | null,
  onClose = () => {},
  onSelectEntity = (_id: string) => {},
}: {
  manifest: ModuleManifest;
  initialEntityId?: string | null;
  onClose?: () => void;
  onSelectEntity?: (id: string) => void;
} = $props();

// svelte-ignore state_referenced_locally
let currentId = $state<string | null>(initialEntityId ?? null);
// svelte-ignore state_referenced_locally
let history = $state<string[]>(initialEntityId ? [initialEntityId] : []);
// svelte-ignore state_referenced_locally
let historyIndex = $state(initialEntityId ? 0 : -1);
let entities = $state<Entity[]>([]);
let entity = $state<Entity | null>(null);
let documentBody = $state("");
let fields = $state<Record<string, unknown>>({});
let relationships = $state<any[]>([]);
let assets = $state<Asset[]>([]);
let profileMediaUrl = $state("");
let mapLocations = $state<any[]>([]);
let loading = $state(true);
let tocSearch = $state("");
let searchMatches = $state<Entity[] | null>(null);
let searching = $state(false);
let searchError = $state("");
let searchRequest = 0;
let entityLoadRequest = 0;

const schemas = $derived(manifest.schemas ?? []);
const allEntityTypes = $derived(schemas.flatMap((schema: any) => schema.entityTypes));
const allFields = $derived(schemas.flatMap((schema: any) => schema.fields));
const articleOutline = $derived(documentBody ? headingOutline(documentBody) : []);

function labelForType(type: string | null) {
  if (!type) return "Uncategorized";
  const template = manifest.templates.find((candidate: any) => candidate.entityType === type);
  return template?.name ?? humanizeType(type);
}

function fieldsForType(entityType: string | null) {
  if (!entityType) return allFields;
  return allFields.filter((field: any) => !field.entityTypes || field.entityTypes.includes(entityType));
}

function isEmptyValue(value: unknown) {
  if (value === null || value === undefined) return true;
  if (typeof value === "string" && value.trim() === "") return true;
  if (Array.isArray(value) && value.length === 0) return true;
  if (typeof value === "object" && value !== null) {
    try {
      if (parseCalendarDate(value)) return false;
      if (Object.keys(value).length === 0) return true;
    } catch {}
  }
  return false;
}

function fieldDisplay(value: unknown) {
  if (Array.isArray(value)) return value.join(", ");
  if (value === null || value === undefined || value === "") return "";
  if (typeof value === "object") {
    try {
      if (parseCalendarDate(value)) return formatCalendarDate(value);
    } catch {}
    try {
      return JSON.stringify(value);
    } catch {
      return String(value);
    }
  }
  return String(value);
}

function entityName(id: string) {
  return entities.find((candidate) => candidate.id === id)?.name ?? id.slice(0, 8);
}

function entityTypeOf(id: string) {
  return entities.find((candidate) => candidate.id === id)?.entity_type ?? null;
}

function humanizeType(value: string) {
  const label = value.replaceAll("_", " ").replaceAll("-", " ");
  return label.charAt(0).toUpperCase() + label.slice(1);
}

function formatSystemTimestamp(value: unknown): string {
  const timestamp = typeof value === "number" ? value : Number(value);
  if (!Number.isFinite(timestamp)) return "";
  const date = new Date(Math.floor(timestamp / 1_000_000));
  if (Number.isNaN(date.getTime())) return "";
  return date.toLocaleDateString(undefined, { year: "numeric", month: "short", day: "numeric" });
}

const displayedEntities = $derived(searchMatches ?? entities);
const grouped = $derived(
  (() => {
    const groups = new Map<string, Entity[]>();
    for (const candidate of displayedEntities) {
      const key = candidate.entity_type ?? "__unknown";
      const list = groups.get(key) ?? [];
      list.push(candidate);
      groups.set(key, list);
    }
    return [...groups.entries()]
      .map(([type, list]) => ({
        type,
        label: labelForType(type),
        count: list.length,
        list: list
          .sort((left, right) => left.name.localeCompare(right.name))
          .map((candidate) => ({
            id: candidate.id,
            name: candidate.name,
            typeLabel: labelForType(candidate.entity_type),
          })),
      }))
      .sort((left, right) => left.label.localeCompare(right.label));
  })(),
);
const recent = $derived(
  [...entities]
    .sort((left, right) => Number(right.updated_at) - Number(left.updated_at))
    .slice(0, 6)
    .map((candidate) => ({
      id: candidate.id,
      name: candidate.name,
      typeLabel: labelForType(candidate.entity_type),
    })),
);
const outbound = $derived(relationships.filter((relationship: any) => relationship.source_id === currentId));
const inbound = $derived(relationships.filter((relationship: any) => relationship.target_id === currentId));
const profileAssetAny = $derived(
  assets.find((asset) => manifest.namespaces.includes(asset.namespace) && asset.role === "profile") ?? null,
);
const profileAsset = $derived(
  profileAssetAny && ["image/png", "image/jpeg", "image/gif", "image/webp"].includes(profileAssetAny.mime_type)
    ? profileAssetAny
    : null,
);
const profileFallback = $derived(profileAssetAny && !profileAsset ? profileAssetAny : null);
const visibleFields = $derived(
  (() => {
    if (!entity) return [];
    return fieldsForType(entity.entity_type)
      .filter((definition: any) => definition.type !== "relationship")
      .map((definition: any) => ({
        key: definition.key,
        label: definition.label,
        value: fieldDisplay(fields[definition.key]),
      }))
      .filter((row: any) => !isEmptyValue(fields[row.key]) && row.value);
  })(),
);
const visibleRelationshipFields = $derived(
  (() => {
    if (!entity) return [];
    return fieldsForType(entity.entity_type)
      .filter((definition: any) => definition.type === "relationship")
      .map((definition: any) => ({
        label: definition.label,
        targets: outbound
          .filter((relationship: any) => relationship.relationship_type === definition.relationshipType)
          .map((relationship: any) => ({ id: relationship.target_id, name: entityName(relationship.target_id) })),
      }))
      .filter((row: any) => row.targets.length > 0);
  })(),
);

$effect(() => {
  const asset = profileAsset;
  let disposed = false;
  let objectUrl = "";
  profileMediaUrl = "";
  if (asset) {
    void project
      .readAssetBytes(asset.id)
      .then((bytes) => {
        if (disposed) return;
        const blob = new Blob([Uint8Array.from(bytes)], { type: asset.mime_type });
        objectUrl = URL.createObjectURL(blob);
        profileMediaUrl = objectUrl;
      })
      .catch(() => {
        if (!disposed) profileMediaUrl = "";
      });
  }
  return () => {
    disposed = true;
    if (objectUrl) URL.revokeObjectURL(objectUrl);
  };
});

$effect(() => {
  const query = tocSearch.trim();
  const request = ++searchRequest;
  searchError = "";
  if (!query) {
    searchMatches = null;
    searching = false;
    return;
  }
  searching = true;
  const timer = window.setTimeout(() => {
    void project
      .search(query)
      .then((matches) => {
        if (request !== searchRequest) return;
        searchMatches = matches.filter(
          (candidate) => !candidate.deleted && candidate.entity_type && allEntityTypes.includes(candidate.entity_type),
        );
      })
      .catch((cause) => {
        if (request !== searchRequest) return;
        searchMatches = [];
        searchError = cause instanceof Error ? cause.message : String(cause);
      })
      .finally(() => {
        if (request === searchRequest) searching = false;
      });
  }, 180);
  return () => window.clearTimeout(timer);
});

$effect(() => {
  const id = currentId;
  if (id) void loadEntity(id);
});

async function loadAll() {
  loading = true;
  try {
    entities = (await project.listEntities())
      .filter(
        (candidate) => !candidate.deleted && candidate.entity_type && allEntityTypes.includes(candidate.entity_type),
      )
      .sort((left, right) => left.name.localeCompare(right.name));
  } finally {
    loading = false;
  }
}

async function loadEntity(id: string) {
  const request = ++entityLoadRequest;
  loading = true;
  try {
    const knownEntity = entities.find((candidate) => candidate.id === id) ?? null;
    const [loadedEntity, documents, storedFields, storedRelationships, storedAssets, storedMapLocations] =
      await Promise.all([
        knownEntity
          ? Promise.resolve(knownEntity)
          : project.listEntities().then((list) => list.find((candidate) => candidate.id === id) ?? null),
        project.listDocuments(id),
        project.listFields(id).catch(() => []),
        project.listRelationships(id).catch(() => []),
        project.listAssets(id).catch(() => []),
        project.listMapLocations(id).catch(() => []),
      ]);
    if (request !== entityLoadRequest || currentId !== id) return;
    entity = loadedEntity;
    documentBody = documents[0]?.body ?? "";
    fields = Object.fromEntries((storedFields as any[]).map((field) => [field.key, field.value]));
    relationships = storedRelationships as any[];
    assets = storedAssets;
    mapLocations = storedMapLocations;
  } finally {
    if (request === entityLoadRequest) loading = false;
  }
}

onMount(() => void loadAll());

function pushHistory(id: string) {
  if (historyIndex >= 0 && history[historyIndex] === id) return;
  if (historyIndex < history.length - 1) history = history.slice(0, historyIndex + 1);
  history = [...history, id];
  historyIndex = history.length - 1;
}

function openEntity(id: string) {
  if (id === currentId) return;
  pushHistory(id);
  currentId = id;
  onSelectEntity(id);
  document.querySelector(".kb-content")?.scrollTo(0, 0);
}

function goBack() {
  if (historyIndex <= 0) return;
  historyIndex -= 1;
  currentId = history[historyIndex];
  onSelectEntity(currentId);
}

function goForward() {
  if (historyIndex < 0 || historyIndex >= history.length - 1) return;
  historyIndex += 1;
  currentId = history[historyIndex];
  onSelectEntity(currentId);
}

function goToMain() {
  entityLoadRequest += 1;
  loading = false;
  currentId = null;
  entity = null;
  documentBody = "";
  fields = {};
  relationships = [];
  assets = [];
  mapLocations = [];
}

function handleEdit() {
  if (!currentId) return;
  onSelectEntity(currentId);
  onClose();
}
</script>

<section class="kb-shell" aria-label="Lore knowledge base">
  <header class="kb-topbar">
    <div class="kb-brand">
      <button class="back-workspace" type="button" onclick={onClose} aria-label="Back to workspace">
        <ArrowLeft size={15} strokeWidth={1.8} />
      </button>
      <span class="brand-mark"><BookOpen size={16} strokeWidth={1.8} /></span>
      <div><strong>{manifest.name} knowledge base</strong><small>{entities.length} published pages</small></div>
    </div>
    {#if entity && currentId}
      <div class="topbar-actions">
        <button
          type="button"
          class="toolbar-button icon"
          onclick={goBack}
          disabled={historyIndex <= 0}
          aria-label="Back">
          <ArrowLeft size={14} strokeWidth={1.8} />
        </button>
        <button
          type="button"
          class="toolbar-button icon"
          onclick={goForward}
          disabled={historyIndex >= history.length - 1}
          aria-label="Forward"><ArrowRight size={14} strokeWidth={1.8} /></button>
        <button type="button" class="toolbar-button" onclick={handleEdit}
          ><Pencil size={14} strokeWidth={1.8} /> Edit</button>
        <WikiExportMenu entityId={currentId} manifestId={manifest.id} articleName={entity.name} />
      </div>
    {/if}
  </header>

  <div class="kb-workspace">
    <WikiSidebar
      bind:query={tocSearch}
      groups={grouped}
      {recent}
      {currentId}
      {searching}
      onHome={goToMain}
      onOpen={openEntity} />

    <main class="kb-content">
      {#if searchError}<p class="kb-alert" role="alert">Search is unavailable: {searchError}</p>{/if}
      {#if loading && !entity && entities.length === 0}
        <div class="kb-loading" aria-live="polite">Loading knowledge base…</div>
      {:else if !currentId}
        <section class="kb-home">
          <div class="home-hero">
            <span class="eyebrow"><Sparkles size={12} strokeWidth={1.8} /> Your world, connected</span>
            <h1>A living reference for everything in your world.</h1>
            <p>Search across names, article text, and structured details—or browse the collection by category.</p>
            <div class="home-stats">
              <span><strong>{entities.length}</strong> pages</span>
              <span><strong>{grouped.length}</strong> categories</span>
              <span><strong>{recent.length}</strong> recently updated</span>
            </div>
          </div>

          {#if recent.length > 0}
            <section class="home-section">
              <div class="section-heading">
                <div>
                  <span>CONTINUE EXPLORING</span>
                  <h2>Recently updated</h2>
                </div>
              </div>
              <div class="recent-grid">
                {#each recent.slice(0, 4) as item}
                  {@const recentEntity = entities.find((candidate) => candidate.id === item.id)}
                  <button type="button" onclick={() => openEntity(item.id)}>
                    <span class="recent-icon">{item.name.slice(0, 1).toUpperCase()}</span>
                    <span
                      ><small>{item.typeLabel}</small><strong>{item.name}</strong><em
                        >Updated {formatSystemTimestamp(recentEntity?.updated_at)}</em
                      ></span>
                    <ArrowRight size={15} strokeWidth={1.7} />
                  </button>
                {/each}
              </div>
            </section>
          {/if}

          <section class="home-section">
            <div class="section-heading">
              <div>
                <span>COLLECTIONS</span>
                <h2>Browse by category</h2>
              </div>
            </div>
            {#if grouped.length === 0}
              <div class="empty-state">
                <BookOpen size={25} strokeWidth={1.5} /><strong>No wiki pages yet</strong>
                <p>Create a Lore entry to begin building this knowledge base.</p>
              </div>
            {:else}
              <div class="category-grid">
                {#each grouped as group}
                  <article>
                    <header>
                      <span>{group.label.slice(0, 1).toUpperCase()}</span>
                      <div>
                        <h3>{group.label}</h3>
                        <small>{group.count} {group.count === 1 ? "page" : "pages"}</small>
                      </div>
                    </header>
                    <ul>
                      {#each group.list.slice(0, 4) as item}
                        <li><button type="button" onclick={() => openEntity(item.id)}>{item.name}</button></li>
                      {/each}
                    </ul>
                    {#if group.count > 4}<small class="more-count">+ {group.count - 4} more in the sidebar</small>{/if}
                  </article>
                {/each}
              </div>
            {/if}
          </section>
        </section>
      {:else if entity}
        <div class="article-grid">
          <article class="kb-article">
            <nav class="breadcrumbs" aria-label="Breadcrumb">
              <button type="button" onclick={goToMain}>Knowledge base</button><span>/</span><span
                >{labelForType(entity.entity_type)}</span>
            </nav>
            <header class="article-heading">
              <span class="article-type">{labelForType(entity.entity_type)}</span>
              <h1>{entity.name}</h1>
              <p>Last updated {formatSystemTimestamp(entity.updated_at)}</p>
            </header>
            {#if loading}<div class="article-loading">Refreshing article…</div>{/if}
            {#if documentBody}
              <div class="article-body">
                {#key currentId}
                  <MarkdownArticle markdown={documentBody} {entities} onOpenEntity={openEntity} showOutline={false} />
                {/key}
              </div>
            {:else}
              <div class="empty-state article-empty">
                <BookOpen size={24} strokeWidth={1.5} /><strong>This page is ready for its story.</strong>
                <p>Add prose in the workspace editor, then return here to read it as an article.</p>
                <button type="button" onclick={handleEdit}>Write article</button>
              </div>
            {/if}

            {#if assets.length > 0}
              <section class="article-section">
                <h2><Paperclip size={16} strokeWidth={1.8} /> Attachments</h2>
                <ul class="resource-list">
                  {#each assets as asset}<li>
                      <Diamond size={12} strokeWidth={1.8} /><span
                        ><strong>{asset.filename}</strong><small
                          >{asset.mime_type} · {Math.max(1, Math.round(asset.size / 1024))} KB</small
                        ></span>
                    </li>{/each}
                </ul>
              </section>
            {/if}
            {#if mapLocations.length > 0}
              <section class="article-section">
                <h2><MapPinned size={16} strokeWidth={1.8} /> Maps</h2>
                <ul class="resource-list">
                  {#each mapLocations as location}<li>
                      <MapPinned size={13} strokeWidth={1.8} /><span
                        ><strong>{location.label || "Location"}</strong><small
                          >{location.role} · {location.mapEntityId.slice(0, 8)}</small
                        ></span>
                    </li>{/each}
                </ul>
              </section>
            {/if}
          </article>

          <aside class="kb-rail" aria-label="Article information">
            <section class="info-card">
              {#if profileMediaUrl}<img
                  class="profile-media"
                  src={profileMediaUrl}
                  alt={`${entity.name} profile`} />{:else if profileFallback}<div class="profile-file">
                  <Diamond size={18} strokeWidth={1.7} /><span
                    ><strong>{profileFallback.filename}</strong><small>{profileFallback.mime_type}</small></span>
                </div>{/if}
              <header>
                <h2>{entity.name}</h2>
                <p>{labelForType(entity.entity_type)}</p>
              </header>
              <dl>
                {#each visibleFields as row}<div>
                    <dt>{row.label}</dt>
                    <dd>{row.value}</dd>
                  </div>{/each}
                {#each visibleRelationshipFields as row}<div>
                    <dt>{row.label}</dt>
                    <dd>
                      {#each row.targets as target, index}<button type="button" onclick={() => openEntity(target.id)}
                          >{target.name}</button
                        >{#if index < row.targets.length - 1},
                        {/if}{/each}
                    </dd>
                  </div>{/each}
              </dl>
              {#if visibleFields.length === 0 && visibleRelationshipFields.length === 0}<p class="card-empty">
                  No structured details yet.
                </p>{/if}
            </section>

            {#if articleOutline.length > 0}
              <nav class="rail-card outline-card" aria-label="On this page">
                <h2>On this page</h2>
                <ol>
                  {#each articleOutline as item}<li class={`depth-${item.depth}`}>
                      <a href={`#${item.id}`}>{item.text}</a>
                    </li>{/each}
                </ol>
              </nav>
            {/if}

            <section class="rail-card connections-card">
              <h2><Link2 size={13} strokeWidth={1.8} /> Connections</h2>
              {#if outbound.length === 0 && inbound.length === 0}
                <p class="card-empty">No linked pages yet.</p>
              {:else}
                {#if outbound.length > 0}<h3>From this page</h3>
                  <ul>
                    {#each outbound as relationship}<li>
                        <span>{humanizeType(relationship.relationship_type)}</span><button
                          type="button"
                          onclick={() => openEntity(relationship.target_id)}
                          >{entityName(relationship.target_id)}</button>
                      </li>{/each}
                  </ul>{/if}
                {#if inbound.length > 0}<h3>Links here</h3>
                  <ul>
                    {#each inbound as relationship}<li>
                        <span>{humanizeType(relationship.relationship_type)}</span><button
                          type="button"
                          onclick={() => openEntity(relationship.source_id)}
                          >{entityName(relationship.source_id)}<small
                            >{entityTypeOf(relationship.source_id)
                              ? ` · ${labelForType(entityTypeOf(relationship.source_id))}`
                              : ""}</small
                          ></button>
                      </li>{/each}
                  </ul>{/if}
              {/if}
            </section>
          </aside>
        </div>
      {:else}
        <div class="empty-state">
          <BookOpen size={25} strokeWidth={1.5} /><strong>Page not found</strong>
          <p>It may have been removed or no longer belongs to this knowledge base.</p>
          <button type="button" onclick={goToMain}>Return home</button>
        </div>
      {/if}
    </main>
  </div>
</section>

<style>
.kb-shell {
  display: flex;
  height: calc(100vh - 58px);
  min-height: 0;
  flex-direction: column;
  background: #f4f5f2;
  color: #252b26;
  font-family: var(--font-body, Inter, ui-sans-serif, system-ui, sans-serif);
}
.kb-topbar {
  z-index: 20;
  display: flex;
  min-height: 58px;
  align-items: center;
  justify-content: space-between;
  gap: 16px;
  padding: 10px 18px;
  border-bottom: 1px solid #dde1da;
  background: rgba(255, 255, 255, 0.96);
  box-shadow: 0 1px 8px rgba(30, 37, 31, 0.03);
}
.kb-brand,
.topbar-actions {
  display: flex;
  align-items: center;
  gap: 9px;
}
.kb-brand {
  min-width: 0;
}
.kb-brand > div {
  display: grid;
  gap: 2px;
}
.kb-brand strong {
  font-size: 12px;
}
.kb-brand small {
  color: #899088;
  font-size: 9px;
}
.brand-mark {
  display: grid;
  width: 31px;
  height: 31px;
  place-items: center;
  border-radius: 8px;
  background: #e4ece4;
  color: #416047;
}
.back-workspace,
.toolbar-button {
  display: inline-flex;
  min-height: 34px;
  align-items: center;
  justify-content: center;
  gap: 6px;
  border: 1px solid #d9ddd6;
  border-radius: 8px;
  background: #fff;
  color: #4d584f;
  cursor: pointer;
}
.back-workspace {
  width: 34px;
}
.toolbar-button {
  padding: 0 10px;
  font: 650 11px var(--font-body, Inter, sans-serif);
}
.toolbar-button.icon {
  width: 34px;
  padding: 0;
}
.toolbar-button:hover {
  background: #f2f6f2;
  color: #2f4e35;
}
.toolbar-button:disabled {
  opacity: 0.35;
  cursor: not-allowed;
}
.kb-workspace {
  display: grid;
  min-height: 0;
  flex: 1;
  grid-template-columns: 250px minmax(0, 1fr);
}
.kb-content {
  min-width: 0;
  overflow: auto;
}
.kb-alert {
  margin: 14px 22px 0;
  padding: 9px 11px;
  border: 1px solid #edcec5;
  border-radius: 8px;
  background: #fff2ee;
  color: #934b3d;
  font-size: 11px;
}
.kb-loading,
.article-loading {
  color: #818981;
  font-size: 11px;
}
.kb-loading {
  padding: 48px;
}
.article-loading {
  margin-bottom: 16px;
  padding: 8px 10px;
  border-radius: 7px;
  background: #f3f5f1;
}
.kb-home {
  width: min(1180px, 100%);
  margin: 0 auto;
  padding: 44px clamp(24px, 5vw, 70px) 70px;
}
.home-hero {
  padding: clamp(30px, 5vw, 56px);
  border: 1px solid #dce2da;
  border-radius: 20px;
  background: linear-gradient(135deg, #fff 0%, #f0f5ee 100%);
  box-shadow: 0 16px 50px rgba(38, 46, 39, 0.06);
}
.eyebrow {
  display: flex;
  align-items: center;
  gap: 6px;
  color: #648069;
  font-size: 9px;
  font-weight: 800;
  letter-spacing: 0.1em;
  text-transform: uppercase;
}
.home-hero h1 {
  max-width: 720px;
  margin: 13px 0 12px;
  color: #1f2921;
  font: 600 clamp(32px, 5vw, 54px)/1.04 var(--font-display, Georgia, serif);
  letter-spacing: -0.025em;
}
.home-hero p {
  max-width: 620px;
  margin: 0;
  color: #697169;
  font-size: 13px;
  line-height: 1.65;
}
.home-stats {
  display: flex;
  flex-wrap: wrap;
  gap: 9px;
  margin-top: 26px;
}
.home-stats span {
  padding: 7px 11px;
  border: 1px solid #dce3db;
  border-radius: 999px;
  background: rgba(255, 255, 255, 0.72);
  color: #707870;
  font-size: 10px;
}
.home-stats strong {
  margin-right: 3px;
  color: #34503a;
  font-size: 12px;
}
.home-section {
  margin-top: 40px;
}
.section-heading {
  display: flex;
  align-items: end;
  justify-content: space-between;
  margin-bottom: 14px;
}
.section-heading span {
  color: #929990;
  font-size: 8px;
  font-weight: 800;
  letter-spacing: 0.12em;
}
.section-heading h2 {
  margin: 4px 0 0;
  font: 600 21px var(--font-display, Georgia, serif);
}
.recent-grid {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 10px;
}
.recent-grid > button {
  display: grid;
  grid-template-columns: 42px minmax(0, 1fr) 18px;
  align-items: center;
  gap: 11px;
  padding: 13px;
  border: 1px solid #dfe2dc;
  border-radius: 12px;
  background: #fff;
  color: #29302a;
  text-align: left;
  cursor: pointer;
}
.recent-grid > button:hover {
  border-color: #bdc9bd;
  box-shadow: 0 8px 24px rgba(38, 46, 39, 0.06);
}
.recent-icon {
  display: grid;
  width: 40px;
  height: 40px;
  place-items: center;
  border-radius: 10px;
  background: #e8eee6;
  color: #436048;
  font: 650 17px var(--font-display, Georgia, serif);
}
.recent-grid button > span:nth-child(2) {
  display: grid;
  gap: 2px;
  min-width: 0;
}
.recent-grid small,
.recent-grid em {
  color: #8d948d;
  font-size: 9px;
  font-style: normal;
}
.recent-grid strong {
  overflow: hidden;
  font-size: 12px;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.category-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(230px, 1fr));
  gap: 12px;
}
.category-grid article {
  padding: 16px;
  border: 1px solid #dfe2dc;
  border-radius: 13px;
  background: #fff;
}
.category-grid header {
  display: flex;
  align-items: center;
  gap: 10px;
  padding-bottom: 11px;
  border-bottom: 1px solid #eceee9;
}
.category-grid header > span {
  display: grid;
  width: 35px;
  height: 35px;
  place-items: center;
  border-radius: 9px;
  background: #edf1eb;
  color: #526c56;
  font: 650 14px var(--font-display, Georgia, serif);
}
.category-grid h3 {
  margin: 0;
  font: 600 14px var(--font-display, Georgia, serif);
}
.category-grid header small {
  color: #939993;
  font-size: 9px;
}
.category-grid ul {
  display: grid;
  gap: 2px;
  margin: 10px 0 0;
  padding: 0;
  list-style: none;
}
.category-grid li button {
  width: 100%;
  padding: 5px 2px;
  border: 0;
  background: transparent;
  color: #46604b;
  font-size: 11px;
  text-align: left;
  cursor: pointer;
}
.category-grid li button:hover {
  text-decoration: underline;
  text-underline-offset: 2px;
}
.more-count {
  display: block;
  margin-top: 7px;
  color: #9a9f99;
  font-size: 9px;
}
.article-grid {
  display: grid;
  width: min(1240px, 100%);
  min-height: 100%;
  grid-template-columns: minmax(0, 820px) 300px;
  gap: 26px;
  margin: 0 auto;
  padding: 32px clamp(22px, 4vw, 52px) 70px;
  align-items: start;
}
.kb-article {
  min-width: 0;
  padding: clamp(26px, 5vw, 58px);
  border: 1px solid #dfe2dc;
  border-radius: 16px;
  background: #fff;
  box-shadow: 0 12px 40px rgba(35, 42, 36, 0.055);
}
.breadcrumbs {
  display: flex;
  align-items: center;
  gap: 7px;
  margin-bottom: 26px;
  color: #8d948d;
  font-size: 10px;
}
.breadcrumbs button {
  border: 0;
  background: transparent;
  color: #52705a;
  font: inherit;
  cursor: pointer;
}
.article-heading {
  margin-bottom: 32px;
  padding-bottom: 23px;
  border-bottom: 1px solid #e5e7e2;
}
.article-type {
  color: #637e67;
  font-size: 9px;
  font-weight: 800;
  letter-spacing: 0.1em;
  text-transform: uppercase;
}
.article-heading h1 {
  margin: 8px 0 8px;
  color: #1e241f;
  font: 650 clamp(34px, 5vw, 50px)/1.03 var(--font-display, Georgia, serif);
  letter-spacing: -0.025em;
}
.article-heading p {
  margin: 0;
  color: #939993;
  font-size: 10px;
}
.article-body {
  font-size: 14px;
  line-height: 1.78;
}
.article-section {
  margin-top: 34px;
  padding-top: 22px;
  border-top: 1px solid #e7e9e4;
}
.article-section h2 {
  display: flex;
  align-items: center;
  gap: 7px;
  margin: 0 0 12px;
  font: 600 15px var(--font-display, Georgia, serif);
}
.resource-list {
  display: grid;
  gap: 7px;
  margin: 0;
  padding: 0;
  list-style: none;
}
.resource-list li {
  display: flex;
  align-items: center;
  gap: 9px;
  padding: 9px 10px;
  border: 1px solid #e5e8e2;
  border-radius: 8px;
  background: #f8f9f7;
  color: #58705d;
}
.resource-list li span {
  display: grid;
  gap: 2px;
}
.resource-list strong {
  color: #3a423b;
  font-size: 11px;
}
.resource-list small {
  color: #90968f;
  font-size: 9px;
}
.kb-rail {
  position: sticky;
  top: 0;
  display: grid;
  gap: 13px;
}
.info-card,
.rail-card {
  overflow: hidden;
  border: 1px solid #dde1da;
  border-radius: 13px;
  background: #fff;
  box-shadow: 0 4px 18px rgba(35, 42, 36, 0.04);
}
.profile-media {
  display: block;
  width: 100%;
  max-height: 310px;
  object-fit: cover;
  border-bottom: 1px solid #e1e4de;
}
.profile-file {
  display: flex;
  align-items: center;
  gap: 9px;
  padding: 14px;
  background: #edf1eb;
  color: #58705d;
}
.profile-file span {
  display: grid;
}
.profile-file strong {
  color: #364139;
  font-size: 11px;
}
.profile-file small {
  font-size: 9px;
}
.info-card > header {
  padding: 15px 16px 12px;
  border-bottom: 1px solid #e7e9e4;
  background: linear-gradient(#f7f9f6, #fff);
}
.info-card h2 {
  margin: 0;
  font: 600 17px var(--font-display, Georgia, serif);
}
.info-card header p {
  margin: 3px 0 0;
  color: #838b83;
  font-size: 10px;
}
.info-card dl {
  display: grid;
  gap: 12px;
  margin: 0;
  padding: 14px 16px;
}
.info-card dl div {
  display: grid;
  gap: 3px;
}
.info-card dt {
  color: #8b928b;
  font-size: 8px;
  font-weight: 800;
  letter-spacing: 0.08em;
  text-transform: uppercase;
}
.info-card dd {
  margin: 0;
  color: #39413a;
  font-size: 11px;
  line-height: 1.45;
}
.info-card dd button,
.connections-card button {
  border: 0;
  background: transparent;
  color: #42634a;
  font: inherit;
  cursor: pointer;
  padding: 0;
  text-align: left;
}
.rail-card {
  padding: 14px 15px;
}
.rail-card h2 {
  display: flex;
  align-items: center;
  gap: 6px;
  margin: 0 0 9px;
  color: #707870;
  font-size: 9px;
  font-weight: 800;
  letter-spacing: 0.09em;
  text-transform: uppercase;
}
.outline-card ol,
.connections-card ul {
  display: grid;
  gap: 5px;
  margin: 0;
  padding: 0;
  list-style: none;
}
.outline-card a {
  display: block;
  color: #59645b;
  font-size: 10px;
  line-height: 1.35;
  text-decoration: none;
}
.outline-card a:hover {
  color: #35583d;
}
.outline-card .depth-2 {
  padding-left: 9px;
}
.outline-card .depth-3,
.outline-card .depth-4,
.outline-card .depth-5,
.outline-card .depth-6 {
  padding-left: 18px;
}
.connections-card h3 {
  margin: 12px 0 6px;
  color: #979c96;
  font-size: 8px;
  letter-spacing: 0.08em;
  text-transform: uppercase;
}
.connections-card li {
  display: grid;
  gap: 2px;
  padding: 6px 0;
  border-top: 1px solid #f0f1ee;
}
.connections-card li > span {
  color: #979d97;
  font-size: 8px;
  text-transform: uppercase;
}
.connections-card button {
  font-size: 10px;
}
.connections-card button small {
  color: #939993;
}
.card-empty {
  margin: 10px 15px 15px;
  color: #959b95;
  font-size: 10px;
  line-height: 1.45;
}
.rail-card .card-empty {
  margin: 0;
}
.empty-state {
  display: grid;
  min-height: 240px;
  place-items: center;
  align-content: center;
  padding: 28px;
  color: #849087;
  text-align: center;
}
.empty-state strong {
  margin-top: 10px;
  color: #354039;
  font: 600 18px var(--font-display, Georgia, serif);
}
.empty-state p {
  max-width: 42ch;
  margin: 6px 0 0;
  font-size: 11px;
  line-height: 1.5;
}
.empty-state button {
  margin-top: 13px;
  padding: 8px 11px;
  border: 1px solid #cbd5ca;
  border-radius: 8px;
  background: #f2f6f1;
  color: #3e6146;
  font-size: 10px;
  font-weight: 700;
  cursor: pointer;
}
.article-empty {
  min-height: 260px;
  border: 1px dashed #dfe3dc;
  border-radius: 12px;
  background: #fafbf9;
}
@media (max-width: 1100px) {
  .article-grid {
    grid-template-columns: minmax(0, 1fr) 270px;
    gap: 18px;
    padding-inline: 24px;
  }
}
@media (max-width: 900px) {
  .kb-workspace {
    grid-template-columns: 1fr;
  }
  .kb-content {
    min-height: 0;
  }
  .article-grid {
    grid-template-columns: 1fr;
  }
  .kb-rail {
    position: static;
  }
  .kb-topbar {
    align-items: flex-start;
  }
  .topbar-actions {
    flex-wrap: wrap;
    justify-content: flex-end;
  }
}
@media (max-width: 650px) {
  .kb-topbar {
    padding: 9px 12px;
  }
  .kb-brand small,
  .toolbar-button.icon {
    display: none;
  }
  .toolbar-button {
    min-height: 32px;
  }
  .kb-home {
    padding: 24px 15px 45px;
  }
  .home-hero {
    padding: 26px 22px;
  }
  .recent-grid {
    grid-template-columns: 1fr;
  }
  .article-grid {
    padding: 15px 12px 42px;
  }
  .kb-article {
    padding: 24px 18px;
  }
  .article-heading h1 {
    font-size: 34px;
  }
  .topbar-actions {
    gap: 5px;
  }
}
@media print {
  .kb-shell {
    height: auto;
    background: #fff;
  }
  .kb-topbar,
  .kb-alert {
    display: none !important;
  }
  .kb-workspace {
    display: block;
  }
  .kb-content {
    overflow: visible;
  }
  .article-grid {
    display: block;
    width: auto;
    padding: 0;
  }
  .kb-article {
    padding: 0;
    border: 0;
    box-shadow: none;
  }
  .breadcrumbs,
  .article-loading,
  .article-empty button {
    display: none !important;
  }
  .kb-rail {
    display: block;
    margin-top: 28px;
  }
  .kb-rail > * {
    margin-top: 14px;
    break-inside: avoid;
    box-shadow: none;
  }
  .outline-card {
    display: none;
  }
  .article-section,
  .connections-card {
    break-inside: avoid;
  }
  .profile-media {
    max-width: 280px;
  }
  .article-heading h1 {
    font-size: 38px;
  }
}
</style>
