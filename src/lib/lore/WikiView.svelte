<script lang="ts">
import { onMount } from "svelte";
import {
  ArrowLeft,
  ArrowRight,
  BookOpen,
  Diamond,
  Link2,
  MapPinned,
  ImagePlus,
  Paperclip,
  Pencil,
  Sparkles,
} from "@lucide/svelte";
import MarkdownArticle from "$lib/markdown/MarkdownArticle.svelte";
import { headingOutline } from "$lib/markdown";
import {
  project,
  type AiProviderSettings,
  type Asset,
  type Entity,
  type ImageProviderSettings,
  type ModuleManifest,
} from "$lib/project/client";
import ImageGenerationDialog, { type ImageContextChoice } from "$lib/ai/ImageGenerationDialog.svelte";
import { formatCalendarDate, parseCalendarDate } from "$lib/date";
import { fieldDisplay, formatAttributeValue, formatSystemTimestamp, humanizeType, isEmptyValue } from "./wiki-format";
import WorkspaceTopbar from "$lib/layout/WorkspaceTopbar.svelte";
import WikiExportMenu from "./WikiExportMenu.svelte";
import WikiSidebar from "./WikiSidebar.svelte";
import {
  contributedRelationshipFields,
  counterpartId,
  coveredRelationshipIds,
  groupedRelationshipFields,
  groupRelationshipsByOwningModule,
  relationshipAttributeRows,
  relationshipFieldForType,
  type ManifestLike,
} from "$lib/modules/contributed-fields";

let {
  manifest,
  enabledManifests = [],
  initialEntityId = null as string | null,
  projectId,
  aiEnabled,
  imageProvider,
  textProvider,
  onClose = () => {},
  onSelectEntity = (_id: string) => {},
}: {
  manifest: ModuleManifest;
  enabledManifests?: ManifestLike[];
  initialEntityId?: string | null;
  projectId: string;
  aiEnabled: boolean;
  imageProvider: ImageProviderSettings;
  textProvider: AiProviderSettings;
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
let recentEntities = $state<Entity[]>([]);
let referenceEntities = $state<Entity[]>([]);
let entity = $state<Entity | null>(null);
let documentBody = $state("");
let fields = $state<Record<string, unknown>>({});
let relationships = $state<any[]>([]);
let assets = $state<Asset[]>([]);
let profileMediaUrl = $state("");
let mapLocations = $state<any[]>([]);
let loading = $state(true);
let tocSearch = $state("");
let searching = $state(false);
let searchError = $state("");
let searchRequest = 0;
let wikiPage = $state(0);
const wikiPageSize = 50;
let wikiTotal = $state(0);
let wikiOffset = $state(0);
let wikiHasMore = $state(false);
let wikiTypeCounts = $state<Array<{ entity_type: string | null; count: number }>>([]);
let wikiSearchKey = "";
let entityLoadRequest = 0;
let imageGenerationOpen = $state(false);

type WikiRelationshipAttribute = { key: string; label: string; value: string };

const wikiManifests = $derived(enabledManifests.length ? enabledManifests : [manifest]);
const schemas = $derived(manifest.schemas ?? []);
const allEntityTypeDefinitions = $derived(schemas.flatMap((schema: any) => schema.entityTypes));
const allEntityTypes = $derived(allEntityTypeDefinitions.map((entityType: any) => entityType.id));
const allFields = $derived(schemas.flatMap((schema: any) => schema.fields));
const articleOutline = $derived(documentBody ? headingOutline(documentBody) : []);

function labelForType(type: string | null) {
  if (!type) return "Uncategorized";
  const definition = allEntityTypeDefinitions.find((candidate: any) => candidate.id === type);
  return definition?.name ?? humanizeType(type);
}

function fieldsForType(entityType: string | null) {
  const own = !entityType
    ? allFields
    : allFields.filter((field: any) => !field.entityTypes || field.entityTypes.includes(entityType));
  const enabledTypes = new Set(
    (enabledManifests.length ? enabledManifests : [manifest]).flatMap((candidate) =>
      (candidate.schemas ?? []).flatMap((schema) =>
        (schema.entityTypes ?? []).map((entityTypeDef) => entityTypeDef.id),
      ),
    ),
  );
  const merged = contributedRelationshipFields(
    manifest,
    entityType,
    enabledManifests.length ? enabledManifests : [manifest],
    enabledTypes,
  );
  const keys = new Set(own.map((field: any) => field.key));
  return [...own, ...merged.filter((field) => field.type === "relationship" && !keys.has(field.key))];
}

function entityName(id: string) {
  return (
    (entity?.id === id
      ? entity.name
      : [...entities, ...recentEntities, ...referenceEntities].find((candidate) => candidate.id === id)?.name) ??
    id.slice(0, 8)
  );
}

function entityTypeOf(id: string) {
  return (
    (entity?.id === id
      ? entity.entity_type
      : [...entities, ...recentEntities, ...referenceEntities].find((candidate) => candidate.id === id)?.entity_type) ??
    null
  );
}

const grouped = $derived(
  (() => {
    const counts = new Map(wikiTypeCounts.map((entry) => [entry.entity_type ?? "__unknown", entry.count]));
    const groups = new Map<string, Entity[]>();
    for (const candidate of entities) {
      const key = candidate.entity_type ?? "__unknown";
      const list = groups.get(key) ?? [];
      list.push(candidate);
      groups.set(key, list);
    }
    return [...groups.entries()]
      .map(([type, list]) => ({
        type,
        label: labelForType(type),
        count: counts.get(type) ?? list.length,
        list: list.map((candidate) => ({
          id: candidate.id,
          name: candidate.name,
          typeLabel: labelForType(candidate.entity_type),
        })),
      }))
      .sort((left, right) => left.label.localeCompare(right.label));
  })(),
);
const recent = $derived(
  recentEntities.map((candidate) => ({
    id: candidate.id,
    name: candidate.name,
    typeLabel: labelForType(candidate.entity_type),
  })),
);
const namedRelationshipIds = $derived(
  currentId
    ? coveredRelationshipIds(currentId, relationships, fieldsForType(entity?.entity_type ?? null))
    : new Set<string>(),
);
const outbound = $derived(
  relationships.filter(
    (relationship: any) => relationship.source_id === currentId && !namedRelationshipIds.has(relationship.id),
  ),
);
const inbound = $derived(
  relationships.filter(
    (relationship: any) => relationship.target_id === currentId && !namedRelationshipIds.has(relationship.id),
  ),
);
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
    const entityId = entity.id;
    return fieldsForType(entity.entity_type)
      .filter((definition: any) => definition.type === "relationship")
      .map((definition: any) => ({
        definition,
        label: definition.label,
        targets: relationships
          .map((relationship: any) => {
            const id = counterpartId(entityId, relationship, definition);
            if (!id) return null;
            return {
              id,
              name: entityName(id),
              attributes: attributesForRelationship(relationship, definition),
            };
          })
          .filter(
            (target): target is { id: string; name: string; attributes: WikiRelationshipAttribute[] } =>
              target !== null,
          ),
      }))
      .filter((row: any) => row.targets.length > 0);
  })(),
);
const groupedWikiRelationships = $derived(
  groupedRelationshipFields(
    visibleRelationshipFields.map((row) => row.definition),
    wikiManifests,
    (field) => visibleRelationshipFields.find((row) => row.definition.key === field.key)?.targets.length ?? 0,
  ).map((group) => ({
    moduleId: group.moduleId,
    moduleName: group.moduleName,
    rows: group.fields
      .map((field) => visibleRelationshipFields.find((row) => row.definition.key === field.key))
      .filter((row): row is (typeof visibleRelationshipFields)[number] => row != null),
  })),
);
function attributesForRelationship(
  relationship: any,
  definition?: { metadataFields?: Array<{ key: string; label?: string; type?: string }> } | null,
): WikiRelationshipAttribute[] {
  const resolved = definition ?? relationshipFieldForType(relationship.relationship_type, wikiManifests);
  const metaFields = resolved?.metadataFields ?? [];
  return relationshipAttributeRows(relationship.metadata, resolved as any)
    .filter((row) => !isEmptyValue(row.raw))
    .map((row) => {
      const field = metaFields.find((candidate) => candidate.key === row.key) ?? null;
      return {
        key: row.key,
        label: row.label === row.key ? humanizeType(row.key) : row.label,
        value: formatAttributeValue(row.raw, field),
      };
    })
    .filter((row) => row.value !== "");
}
const enrichedOutbound = $derived(
  outbound.map((relationship: any) => ({
    ...relationship,
    attributes: attributesForRelationship(relationship),
  })),
);
const enrichedInbound = $derived(
  inbound.map((relationship: any) => ({
    ...relationship,
    attributes: attributesForRelationship(relationship),
  })),
);
const groupedOutbound = $derived(groupRelationshipsByOwningModule(enrichedOutbound, wikiManifests));
const groupedInbound = $derived(groupRelationshipsByOwningModule(enrichedInbound, wikiManifests));
const imageContextChoices = $derived(
  (() => {
    if (!entity) return [] as ImageContextChoice[];
    const priority = /appearance|physical|clothing|species|culture|occupation|era|architecture|location/i;
    const choices: ImageContextChoice[] = [
      {
        id: "identity:name",
        entityId: entity.id,
        label: "Name",
        value: entity.name,
        sourceKind: "identity",
        defaultSelected: true,
      },
      {
        id: "identity:type",
        entityId: entity.id,
        label: "Entity type",
        value: labelForType(entity.entity_type),
        sourceKind: "identity",
        defaultSelected: true,
      },
    ];
    for (const row of visibleFields) {
      choices.push({
        id: `field:${row.key}`,
        entityId: entity.id,
        label: row.label,
        value: row.value,
        sourceKind: "field",
        defaultSelected: priority.test(`${row.key} ${row.label}`),
      });
    }
    const prose = documentBody.trim().replace(/\s+/g, " ");
    if (prose) {
      choices.push({
        id: "document:description",
        entityId: entity.id,
        label: "Article description",
        value: prose.slice(0, 1200),
        sourceKind: "document",
        defaultSelected: false,
      });
    }
    for (const relationship of relationships) {
      const relatedId = relationship.source_id === entity.id ? relationship.target_id : relationship.source_id;
      choices.push({
        id: `relationship:${relationship.id}`,
        entityId: relatedId,
        label: humanizeType(relationship.relationship_type),
        value: entityName(relatedId),
        sourceKind: "relationship",
        defaultSelected: false,
      });
    }
    for (const location of mapLocations) {
      choices.push({
        id: `location:${location.id ?? location.locationId ?? location.mapEntityId}`,
        entityId: entity.id,
        label: "Map location",
        value: location.label || location.role || "Linked location",
        sourceKind: "location",
        defaultSelected: false,
      });
    }
    return choices;
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
  if (wikiSearchKey && wikiSearchKey !== query && wikiPage !== 0) wikiPage = 0;
  wikiSearchKey = query;
});

$effect(() => {
  const query = tocSearch.trim();
  const entityTypes = [...allEntityTypes];
  const page = wikiPage;
  const request = ++searchRequest;
  searchError = "";
  searching = true;
  const timer = window.setTimeout(() => {
    void project
      .queryEntities({
        query: query || undefined,
        entityTypes,
        sortField: "name",
        sortDirection: "asc",
        offset: page * wikiPageSize,
        limit: wikiPageSize,
      })
      .then((result) => {
        if (request !== searchRequest) return;
        if (page > 0 && result.items.length === 0 && result.total > 0) {
          wikiPage = Math.max(0, Math.ceil(result.total / wikiPageSize) - 1);
          return;
        }
        entities = result.items;
        wikiTotal = result.total;
        wikiOffset = result.offset;
        wikiHasMore = result.has_more;
        wikiTypeCounts = result.type_counts;
      })
      .catch((cause) => {
        if (request !== searchRequest) return;
        entities = [];
        wikiTotal = 0;
        wikiOffset = 0;
        wikiHasMore = false;
        wikiTypeCounts = [];
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
    const recent = await project.queryEntities({
      entityTypes: [...allEntityTypes],
      sortField: "updated_at",
      sortDirection: "desc",
      limit: 6,
    });
    recentEntities = recent.items;
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
        knownEntity ? Promise.resolve(knownEntity) : project.getEntity(id),
        project.listDocuments(id),
        project.listFields(id).catch(() => []),
        project.listRelationships(id).catch(() => []),
        project.listAssets(id).catch(() => []),
        project.listMapLocations(id).catch(() => []),
      ]);
    if (request !== entityLoadRequest || currentId !== id) return;
    const relatedIds = [
      ...new Set(
        (storedRelationships as any[])
          .flatMap((relationship) => [relationship.source_id, relationship.target_id])
          .filter((candidate) => typeof candidate === "string" && candidate !== id),
      ),
    ] as string[];
    referenceEntities = (
      await Promise.all(relatedIds.map((relatedId) => project.getEntity(relatedId).catch(() => null)))
    ).filter((candidate): candidate is Entity => candidate !== null);
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

{#snippet topbarActions()}
  <button
    type="button"
    class="workspace-topbar-action icon"
    onclick={goBack}
    disabled={historyIndex <= 0}
    aria-label="Back">
    <ArrowLeft size={14} strokeWidth={1.8} />
  </button>
  <button
    type="button"
    class="workspace-topbar-action icon"
    onclick={goForward}
    disabled={historyIndex >= history.length - 1}
    aria-label="Forward"><ArrowRight size={14} strokeWidth={1.8} /></button>
  {#if entity && currentId && aiEnabled && imageProvider.enabled}
    <button type="button" class="workspace-topbar-action" onclick={() => (imageGenerationOpen = true)}>
      <ImagePlus size={14} strokeWidth={1.8} /> Generate illustration
    </button>
  {/if}
  <button type="button" class="workspace-topbar-action" onclick={handleEdit}
    ><Pencil size={14} strokeWidth={1.8} /> Edit</button>
  {#if entity && currentId}
    <WikiExportMenu entityId={currentId} manifestId={manifest.id} articleName={entity.name} />
  {/if}
{/snippet}

{#snippet wikiAttrChips(attributes: WikiRelationshipAttribute[])}
  {#if attributes.length > 0}
    <div class="wiki-attr-row" aria-label="Relationship details">
      {#each attributes as attr}<span class="wiki-attr-chip" title={`${attr.label}: ${attr.value}`}
          ><strong>{attr.label}</strong>
          {attr.value}</span
        >{/each}
    </div>
  {/if}
{/snippet}

<section class="kb-shell" aria-label="Lore knowledge base">
  <WorkspaceTopbar
    title={`${manifest.name} knowledge base`}
    subtitle={`${wikiTotal} published pages`}
    icon={BookOpen}
    onBack={onClose}
    actions={entity && currentId ? topbarActions : undefined}
    actionsLabel="Article actions" />

  <div class="kb-workspace">
    <WikiSidebar
      bind:query={tocSearch}
      groups={grouped}
      {recent}
      {currentId}
      {searching}
      total={wikiTotal}
      offset={wikiOffset}
      pageSize={wikiPageSize}
      hasMore={wikiHasMore}
      onPrevious={() => (wikiPage = Math.max(0, wikiPage - 1))}
      onNext={() => (wikiPage += 1)}
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
              <span><strong>{wikiTotal}</strong> pages</span>
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
                  {@const recentEntity = recentEntities.find((candidate) => candidate.id === item.id)}
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
              {#if visibleFields.length > 0}
                <dl>
                  {#each visibleFields as row}<div>
                      <dt>{row.label}</dt>
                      <dd>{row.value}</dd>
                    </div>{/each}
                </dl>
              {/if}
              {#each groupedWikiRelationships as group (group.moduleId)}
                <div class="info-rel-group">
                  <h3>{group.moduleName}</h3>
                  <dl>
                    {#each group.rows as row}<div>
                        <dt>{row.label}</dt>
                        <dd class="wiki-rel-targets">
                          {#each row.targets as target}
                            <div class="wiki-rel-target">
                              <button type="button" onclick={() => openEntity(target.id)}>{target.name}</button>
                              {@render wikiAttrChips(target.attributes)}
                            </div>
                          {/each}
                        </dd>
                      </div>{/each}
                  </dl>
                </div>
              {/each}
              {#if visibleFields.length === 0 && groupedWikiRelationships.length === 0}<p class="card-empty">
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
                {#if groupedOutbound.length > 0}<h3>From this page</h3>
                  {#each groupedOutbound as group (group.moduleId)}
                    {#if groupedOutbound.length > 1}<h4>{group.moduleName}</h4>{/if}
                    <ul>
                      {#each group.relationships as relationship}<li>
                          <span>{humanizeType(relationship.relationship_type)}</span><button
                            type="button"
                            onclick={() => openEntity(relationship.target_id)}
                            >{entityName(relationship.target_id)}</button>
                          {@render wikiAttrChips(relationship.attributes)}
                        </li>{/each}
                    </ul>
                  {/each}{/if}
                {#if groupedInbound.length > 0}<h3>Links here</h3>
                  {#each groupedInbound as group (group.moduleId)}
                    {#if groupedInbound.length > 1}<h4>{group.moduleName}</h4>{/if}
                    <ul>
                      {#each group.relationships as relationship}<li>
                          <span>{humanizeType(relationship.relationship_type)}</span><button
                            type="button"
                            onclick={() => openEntity(relationship.source_id)}
                            >{entityName(relationship.source_id)}<small
                              >{entityTypeOf(relationship.source_id)
                                ? ` · ${labelForType(entityTypeOf(relationship.source_id))}`
                                : ""}</small
                            ></button>
                          {@render wikiAttrChips(relationship.attributes)}
                        </li>{/each}
                    </ul>
                  {/each}{/if}
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

{#if imageGenerationOpen && entity && currentId}
  <ImageGenerationDialog
    {projectId}
    {entity}
    namespace={manifest.namespaces[0] ?? "daena.core"}
    contextChoices={imageContextChoices}
    {imageProvider}
    {textProvider}
    onAccepted={(asset) => {
      if (!assets.some((existing) => existing.id === asset.id)) assets = [...assets, asset];
    }}
    onClose={() => (imageGenerationOpen = false)} />
{/if}

<style>
.kb-shell {
  display: flex;
  height: calc(100vh - 58px);
  min-height: 0;
  flex-direction: column;
  background: var(--theme-surface-bg, #f4f5f2);
  color: var(--theme-neutral-text, #252b26);
  font-family: var(--font-body, Inter, ui-sans-serif, system-ui, sans-serif);
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
  border: 1px solid var(--theme-danger-border, #edcec5);
  border-radius: 8px;
  background: var(--theme-danger-bg, #fff2ee);
  color: var(--theme-danger-text, #934b3d);
  font-size: 11px;
}
.kb-loading,
.article-loading {
  color: var(--theme-neutral-text-muted, #818981);
  font-size: 11px;
}
.kb-loading {
  padding: 48px;
}
.article-loading {
  margin-bottom: 16px;
  padding: 8px 10px;
  border-radius: 7px;
  background: var(--theme-success-bg, #f3f5f1);
}
.kb-home {
  width: min(1180px, 100%);
  margin: 0 auto;
  padding: 44px clamp(24px, 5vw, 70px) 70px;
}
.home-hero {
  padding: clamp(30px, 5vw, 56px);
  border: 1px solid var(--theme-neutral-border, #dce2da);
  border-radius: 20px;
  background: linear-gradient(135deg, var(--theme-surface-bg, #fff) 0%, var(--theme-success-bg, #f0f5ee) 100%);
  box-shadow: 0 16px 50px rgba(38, 46, 39, 0.06);
}
.eyebrow {
  display: flex;
  align-items: center;
  gap: 6px;
  color: var(--theme-neutral-text-soft, #648069);
  font-size: 9px;
  font-weight: 800;
  letter-spacing: 0.1em;
  text-transform: uppercase;
}
.home-hero h1 {
  max-width: 720px;
  margin: 13px 0 12px;
  color: var(--theme-neutral-text, #1f2921);
  font: 600 clamp(32px, 5vw, 54px)/1.04 var(--font-display, Georgia, serif);
  letter-spacing: -0.025em;
}
.home-hero p {
  max-width: 620px;
  margin: 0;
  color: var(--theme-neutral-text-soft, #697169);
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
  border: 1px solid var(--theme-neutral-border, #dce3db);
  border-radius: 999px;
  background: var(--surface);
  color: var(--theme-neutral-text-soft, #707870);
  font-size: 10px;
}
.home-stats strong {
  margin-right: 3px;
  color: var(--theme-success-text, #34503a);
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
  color: var(--theme-neutral-text-muted, #929990);
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
  border: 1px solid var(--theme-neutral-border, #dfe2dc);
  border-radius: 12px;
  background: var(--theme-surface-bg, #fff);
  color: var(--theme-neutral-text, #29302a);
  text-align: left;
  cursor: pointer;
}
.recent-grid > button:hover {
  border-color: var(--theme-neutral-border-strong, #bdc9bd);
  box-shadow: 0 8px 24px rgba(38, 46, 39, 0.06);
}
.recent-icon {
  display: grid;
  width: 40px;
  height: 40px;
  place-items: center;
  border-radius: 10px;
  background: var(--theme-success-bg, #e8eee6);
  color: var(--theme-success-text, #436048);
  font: 650 17px var(--font-display, Georgia, serif);
}
.recent-grid button > span:nth-child(2) {
  display: grid;
  gap: 2px;
  min-width: 0;
}
.recent-grid small,
.recent-grid em {
  color: var(--theme-neutral-text-muted, #8d948d);
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
  border: 1px solid var(--theme-neutral-border, #dfe2dc);
  border-radius: 13px;
  background: var(--theme-surface-bg, #fff);
}
.category-grid header {
  display: flex;
  align-items: center;
  gap: 10px;
  padding-bottom: 11px;
  border-bottom: 1px solid var(--theme-neutral-border, #eceee9);
}
.category-grid header > span {
  display: grid;
  width: 35px;
  height: 35px;
  place-items: center;
  border-radius: 9px;
  background: var(--theme-success-bg, #edf1eb);
  color: var(--theme-neutral-text-soft, #526c56);
  font: 650 14px var(--font-display, Georgia, serif);
}
.category-grid h3 {
  margin: 0;
  font: 600 14px var(--font-display, Georgia, serif);
}
.category-grid header small {
  color: var(--theme-neutral-text-muted, #939993);
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
  color: var(--theme-success-text, #46604b);
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
  color: var(--theme-neutral-text-muted, #9a9f99);
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
  border: 1px solid var(--theme-neutral-border, #dfe2dc);
  border-radius: 16px;
  background: var(--theme-surface-bg, #fff);
  box-shadow: 0 12px 40px rgba(35, 42, 36, 0.055);
}
.breadcrumbs {
  display: flex;
  align-items: center;
  gap: 7px;
  margin-bottom: 26px;
  color: var(--theme-neutral-text-muted, #8d948d);
  font-size: 10px;
}
.breadcrumbs button {
  border: 0;
  background: transparent;
  color: var(--theme-success-text, #52705a);
  font: inherit;
  cursor: pointer;
}
.article-heading {
  margin-bottom: 32px;
  padding-bottom: 23px;
  border-bottom: 1px solid var(--theme-neutral-border, #e5e7e2);
}
.article-type {
  color: var(--theme-neutral-text-soft, #637e67);
  font-size: 9px;
  font-weight: 800;
  letter-spacing: 0.1em;
  text-transform: uppercase;
}
.article-heading h1 {
  margin: 8px 0 8px;
  color: var(--theme-neutral-text, #1e241f);
  font: 650 clamp(34px, 5vw, 50px)/1.03 var(--font-display, Georgia, serif);
  letter-spacing: -0.025em;
}
.article-heading p {
  margin: 0;
  color: var(--theme-neutral-text-muted, #939993);
  font-size: 10px;
}
.article-body {
  font-size: 14px;
  line-height: 1.78;
}
.article-section {
  margin-top: 34px;
  padding-top: 22px;
  border-top: 1px solid var(--theme-neutral-border, #e7e9e4);
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
  border: 1px solid var(--theme-neutral-border, #e5e8e2);
  border-radius: 8px;
  background: var(--theme-success-bg, #f8f9f7);
  color: var(--theme-neutral-text-soft, #58705d);
}
.resource-list li span {
  display: grid;
  gap: 2px;
}
.resource-list strong {
  color: var(--theme-neutral-text, #3a423b);
  font-size: 11px;
}
.resource-list small {
  color: var(--theme-neutral-text-muted, #90968f);
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
  border: 1px solid var(--theme-neutral-border, #dde1da);
  border-radius: 13px;
  background: var(--theme-surface-bg, #fff);
  box-shadow: 0 4px 18px rgba(35, 42, 36, 0.04);
}
.profile-media {
  display: block;
  width: 100%;
  max-height: 310px;
  object-fit: cover;
  border-bottom: 1px solid var(--theme-neutral-border, #e1e4de);
}
.profile-file {
  display: flex;
  align-items: center;
  gap: 9px;
  padding: 14px;
  background: var(--theme-success-bg, #edf1eb);
  color: var(--theme-neutral-text-soft, #58705d);
}
.profile-file span {
  display: grid;
}
.profile-file strong {
  color: var(--theme-neutral-text, #364139);
  font-size: 11px;
}
.profile-file small {
  font-size: 9px;
}
.info-card > header {
  padding: 15px 16px 12px;
  border-bottom: 1px solid var(--theme-neutral-border, #e7e9e4);
  background: linear-gradient(var(--theme-success-bg, #f7f9f6), var(--theme-surface-bg, #fff));
}
.info-card h2 {
  margin: 0;
  font: 600 17px var(--font-display, Georgia, serif);
}
.info-card header p {
  margin: 3px 0 0;
  color: var(--theme-neutral-text-muted, #838b83);
  font-size: 10px;
}
.info-card dl {
  display: grid;
  gap: 12px;
  margin: 0;
  padding: 14px 16px;
}
.info-rel-group + .info-rel-group,
.info-card > dl + .info-rel-group {
  border-top: 1px solid var(--theme-neutral-border, #e7e9e4);
}
.info-rel-group h3 {
  margin: 0;
  padding: 12px 16px 0;
  color: var(--theme-neutral-text-muted, #8b928b);
  font-size: 8px;
  font-weight: 800;
  letter-spacing: 0.08em;
  text-transform: uppercase;
}
.info-rel-group dl {
  padding-top: 8px;
}
.wiki-rel-targets {
  display: grid;
  gap: 8px;
}
.wiki-rel-target {
  display: grid;
  gap: 3px;
}
.info-card dl div {
  display: grid;
  gap: 3px;
}
.info-card dt {
  color: var(--theme-neutral-text-muted, #8b928b);
  font-size: 8px;
  font-weight: 800;
  letter-spacing: 0.08em;
  text-transform: uppercase;
}
.info-card dd {
  margin: 0;
  color: var(--theme-neutral-text, #39413a);
  font-size: 11px;
  line-height: 1.45;
}
.info-card dd button,
.connections-card button {
  border: 0;
  background: transparent;
  color: var(--theme-success-text, #42634a);
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
  color: var(--theme-neutral-text-soft, #707870);
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
  color: var(--theme-neutral-text-soft, #59645b);
  font-size: 10px;
  line-height: 1.35;
  text-decoration: none;
}
.outline-card a:hover {
  color: var(--theme-success-text, #35583d);
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
.connections-card h3,
.connections-card h4 {
  margin: 12px 0 6px;
  color: var(--theme-neutral-text-muted, #979c96);
  font-size: 8px;
  letter-spacing: 0.08em;
  text-transform: uppercase;
}
.connections-card h4 {
  margin: 8px 0 4px;
  letter-spacing: 0.04em;
  text-transform: none;
  font-size: 9px;
  font-weight: 700;
}
.connections-card li {
  display: grid;
  gap: 2px;
  padding: 6px 0;
  border-top: 1px solid var(--theme-neutral-border, #f0f1ee);
}
.connections-card li > span {
  color: var(--theme-neutral-text-muted, #979d97);
  font-size: 8px;
  text-transform: uppercase;
}
.connections-card button {
  font-size: 10px;
}
.connections-card button small {
  color: var(--theme-neutral-text-muted, #939993);
}
.wiki-attr-row {
  display: flex;
  flex-wrap: wrap;
  gap: 4px;
  margin-top: 4px;
}
.wiki-attr-chip {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  max-width: 100%;
  padding: 2px 7px;
  border: 1px solid var(--theme-neutral-border, #e5e8e2);
  border-radius: 999px;
  background: var(--theme-success-bg, #f8f9f7);
  color: var(--theme-neutral-text-soft, #59645b);
  font-size: 9px;
  line-height: 1.3;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.wiki-attr-chip strong {
  color: var(--theme-neutral-text-muted, #8b928b);
  font-weight: 700;
  font-size: 8px;
  letter-spacing: 0.04em;
  text-transform: uppercase;
}
.card-empty {
  margin: 10px 15px 15px;
  color: var(--theme-neutral-text-muted, #959b95);
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
  color: var(--theme-neutral-text-muted, #849087);
  text-align: center;
}
.empty-state strong {
  margin-top: 10px;
  color: var(--theme-neutral-text, #354039);
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
  border: 1px solid var(--theme-neutral-border, #cbd5ca);
  border-radius: 8px;
  background: var(--theme-success-bg, #f2f6f1);
  color: var(--theme-success-text, #3e6146);
  font-size: 10px;
  font-weight: 700;
  cursor: pointer;
}
.article-empty {
  min-height: 260px;
  border: 1px dashed var(--theme-neutral-border, #dfe3dc);
  border-radius: 12px;
  background: var(--theme-success-bg, #fafbf9);
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
}
@media (max-width: 650px) {
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
}
@media print {
  .kb-shell {
    height: auto;
    background: var(--theme-surface-bg, #fff);
  }
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
