<script lang="ts">
import { onMount } from "svelte";
import MarkdownArticle from "$lib/markdown/MarkdownArticle.svelte";
import { project, type Asset, type Entity } from "$lib/project/client";
import loreManifestJson from "../../../packages/modules/lore/manifest.json";
import { formatCalendarDate, parseCalendarDate } from "$lib/date";

let {
  initialEntityId = null as string | null,
  onClose = () => {},
  onSelectEntity = (_id: string) => {},
}: {
  initialEntityId?: string | null;
  onClose?: () => void;
  onSelectEntity?: (id: string) => void;
} = $props();

// svelte-ignore state_referenced_locally
let currentId = $state<string | null>(initialEntityId ?? null);

// navigation history for browsing between entities (browser-like back/forward)
let history = $state<string[]>([]);
let historyIndex = $state(-1);
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

const manifest = loreManifestJson as any;
const allEntityTypes = manifest.schemas.flatMap((s: any) => s.entityTypes);
const allFields = manifest.schemas.flatMap((s: any) => s.fields);

function labelForType(type: string | null) {
  if (!type) return "Uncategorized";
  const t = manifest.templates.find((tpl: any) => tpl.entityType === type);
  return t?.name ?? type;
}

function fieldsForType(entityType: string | null) {
  if (!entityType) return allFields;
  return allFields.filter((f: any) => !f.entityTypes || f.entityTypes.includes(entityType));
}

function isEmptyValue(v: unknown) {
  if (v === null || v === undefined) return true;
  if (typeof v === "string" && v.trim() === "") return true;
  if (Array.isArray(v) && v.length === 0) return true;
  // date objects with no year etc. considered empty if parse fails
  if (typeof v === "object" && v !== null) {
    try {
      const asDate = parseCalendarDate(v);
      if (asDate) return false;
      // empty object like {} considered empty
      if (Object.keys(v as object).length === 0) return true;
    } catch {}
  }
  return false;
}

function fieldDisplay(value: unknown) {
  if (Array.isArray(value)) return value.join(", ");
  if (value === null || value === undefined || value === "") return "";
  if (typeof value === "object") {
    try {
      const asDate = parseCalendarDate(value);
      if (asDate) return formatCalendarDate(value);
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
  return entities.find((e) => e.id === id)?.name ?? id.slice(0, 8);
}

function entityTypeOf(id: string) {
  return entities.find((e) => e.id === id)?.entity_type ?? null;
}

function humanizeType(t: string) {
  return t.replaceAll("_", " ");
}

function formatSystemTimestamp(value: unknown): string {
  const n = typeof value === "number" ? value : Number(value);
  if (!Number.isFinite(n)) return "";
  // stored as nanoseconds since unix epoch
  const ms = Math.floor(n / 1_000_000);
  const d = new Date(ms);
  if (Number.isNaN(d.getTime())) return "";
  return d.toLocaleDateString(undefined, { year: "numeric", month: "short", day: "numeric" });
}

// derived groups for main TOC
const filteredEntities = $derived(
  (() => {
    const term = tocSearch.trim().toLowerCase();
    if (!term) return entities;
    return entities.filter((e) => `${e.name} ${e.entity_type ?? ""} ${e.id}`.toLowerCase().includes(term));
  })(),
);

const grouped = $derived(
  (() => {
    const map = new Map<string, Entity[]>();
    for (const e of filteredEntities) {
      const key = e.entity_type ?? "__unknown";
      const arr = map.get(key) ?? [];
      arr.push(e);
      map.set(key, arr);
    }
    return [...map.entries()]
      .map(([type, list]) => ({
        type,
        label: labelForType(type),
        count: list.length,
        list: list.sort((a, b) => a.name.localeCompare(b.name)),
      }))
      .sort((a, b) => a.label.localeCompare(b.label));
  })(),
);

// derived inbound/outbound
const outbound = $derived(relationships.filter((r: any) => r.source_id === currentId));
const inbound = $derived(relationships.filter((r: any) => r.target_id === currentId));
const profileAssetAny = $derived(
  assets.find((asset) => asset.namespace === "lore" && asset.role === "profile") ?? null,
);
const profileAsset = $derived(
  profileAssetAny &&
    ["image/png", "image/jpeg", "image/gif", "image/webp"].includes(profileAssetAny.mime_type)
    ? profileAssetAny
    : null,
);
const profileFallback = $derived(profileAssetAny && !profileAsset ? profileAssetAny : null);

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
        try {
          const blob = new Blob([Uint8Array.from(bytes)], { type: asset.mime_type });
          objectUrl = URL.createObjectURL(blob);
          if (disposed) {
            URL.revokeObjectURL(objectUrl);
            objectUrl = "";
            return;
          }
          profileMediaUrl = objectUrl;
        } catch {
          if (!disposed) profileMediaUrl = "";
        }
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

// infobox visible fields (non-empty)
const visibleFields = $derived(
  (() => {
    if (!entity) return [];
    const defs = fieldsForType(entity.entity_type);
    const rows: Array<{ key: string; label: string; value: string; kind: string }> = [];
    for (const def of defs) {
      if (def.type === "relationship") continue;
      const v = fields[def.key];
      if (isEmptyValue(v)) continue;
      const display = fieldDisplay(v);
      if (!display) continue;
      rows.push({ key: def.key, label: def.label, value: display, kind: def.type });
    }
    return rows;
  })(),
);

const visibleRelationshipFields = $derived(
  (() => {
    if (!entity) return [];
    const defs = fieldsForType(entity.entity_type).filter((d: any) => d.type === "relationship");
    const rows: Array<{ label: string; type: string; targets: Array<{ id: string; name: string }> }> = [];
    for (const def of defs) {
      const rels = outbound.filter((r: any) => r.relationship_type === def.relationshipType);
      if (rels.length === 0) continue;
      rows.push({
        label: def.label,
        type: def.relationshipType,
        targets: rels.map((r: any) => ({ id: r.target_id, name: entityName(r.target_id) })),
      });
    }
    return rows;
  })(),
);

async function loadAll() {
  loading = true;
  try {
    const all = await project.listEntities();
    entities = all
      .filter((e) => !e.deleted && e.entity_type && allEntityTypes.includes(e.entity_type))
      .sort((a, b) => a.name.localeCompare(b.name));
  } catch {}
  loading = false;
}

async function loadEntity(id: string) {
  loading = true;
  try {
    const [ent, docs, flds, rels, assts] = await Promise.all([
      project.listEntities().then((list) => list.find((e) => e.id === id) ?? null),
      project.listDocuments(id),
      project.listFields(id).catch(() => []),
      project.listRelationships(id).catch(() => []),
      project.listAssets(id).catch(() => []),
    ]);
    entity = ent;
    documentBody = docs[0]?.body ?? "";
    const fieldMap: Record<string, unknown> = {};
    for (const f of flds as any[]) fieldMap[f.key] = f.value;
    fields = fieldMap;
    relationships = rels as any[];
    assets = assts;
    try {
      mapLocations = await project.listMapLocations(id);
    } catch {
      mapLocations = [];
    }
  } catch {}
  loading = false;
}

onMount(() => {
  void loadAll().then(() => {
    if (currentId) {
      history = [currentId];
      historyIndex = 0;
      void loadEntity(currentId);
    }
  });
});

$effect(() => {
  const id = currentId;
  if (id) void loadEntity(id);
});

function pushHistory(id: string) {
  if (historyIndex >= 0 && history[historyIndex] === id) return;
  // drop any forward entries when navigating to a new entity
  if (historyIndex < history.length - 1) history = history.slice(0, historyIndex + 1);
  history = [...history, id];
  historyIndex = history.length - 1;
}

function openEntity(id: string) {
  if (id === currentId) return;
  pushHistory(id);
  currentId = id;
  onSelectEntity(id);
  // scroll to top of wiki container
  document.querySelector(".wiki-container")?.scrollTo(0, 0);
}

function goBack() {
  if (historyIndex <= 0) return;
  historyIndex -= 1;
  currentId = history[historyIndex];
  onSelectEntity(currentId);
  document.querySelector(".wiki-container")?.scrollTo(0, 0);
}

function goForward() {
  if (historyIndex < 0 || historyIndex >= history.length - 1) return;
  historyIndex += 1;
  currentId = history[historyIndex];
  onSelectEntity(currentId);
  document.querySelector(".wiki-container")?.scrollTo(0, 0);
}

function handleEdit() {
  if (!currentId) return;
  // close wiki and reveal entity in host editor
  onSelectEntity(currentId);
  onClose();
}

function goToMain() {
  currentId = null;
  entity = null;
  documentBody = "";
  fields = {};
  relationships = [];
}

function handleClose() {
  onClose();
}
</script>

<section class="wiki-shell" aria-label="Lore wiki">
  <header class="wiki-header">
    <div class="wiki-header-left">
      <button class="quiet-button" type="button" onclick={handleClose}>← Back to workspace</button>
      <div class="wiki-title">
        <span class="overline">LORE WIKI</span>
        {#if currentId && entity}
          <h1>{entity.name}</h1>
          <small>{labelForType(entity.entity_type)} · {formatSystemTimestamp(entity.updated_at)}</small>
        {:else}
          <h1>World encyclopedia</h1>
          <small>{entities.length} articles · {grouped.length} categories</small>
        {/if}
      </div>
    </div>
  </header>

  <div class="wiki-container">
    {#if loading && !entity && !grouped.length}
      <p class="wiki-empty">Loading wiki…</p>
    {:else if !currentId}
      <div class="wiki-main">
        <div class="wiki-main-intro">
          <h2>Browse the archive</h2>
          <p>
            Every person, place, faction and artifact as an interconnected encyclopedia. Search or pick a category below
            — each article shows its infobox, story, and what links to it.
          </p>
          <div class="wiki-search">
            <span aria-hidden="true">⌕</span><input
              placeholder="Search articles"
              bind:value={tocSearch}
              aria-label="Search wiki" />
          </div>
        </div>
        {#if grouped.length === 0}
          <p class="wiki-empty">No articles yet. Create your first lore entry to build the wiki.</p>
        {:else}
          <div class="wiki-groups">
            {#each grouped as group}
              <section class="wiki-group">
                <div class="wiki-group-header">
                  <h3>{group.label}</h3>
                  <small>{group.count}</small>
                </div>
                <ul class="wiki-group-list">
                  {#each group.list as ent}
                    <li>
                      <button type="button" class="wiki-link" onclick={() => openEntity(ent.id)}>
                        <span class="wiki-link-name">{ent.name}</span>
                      </button>
                    </li>
                  {/each}
                </ul>
              </section>
            {/each}
          </div>
        {/if}
      </div>
    {:else if entity}
      <div class="wiki-layout">
        <article class="wiki-article">
          <div class="wiki-nav-history">
            <button type="button" class="quiet-button small icon" onclick={goToMain} aria-label="Wiki home">⌂</button>
            <button
              type="button"
              class="quiet-button small icon"
              onclick={goBack}
              disabled={historyIndex <= 0}
              aria-label="Back">←</button>
            <button
              type="button"
              class="quiet-button small icon"
              onclick={goForward}
              disabled={historyIndex >= history.length - 1}
              aria-label="Forward">→</button>
            <button type="button" class="quiet-button small icon" onclick={handleEdit} aria-label="Edit">✎</button>
          </div>
          <nav class="wiki-breadcrumb" aria-label="Breadcrumb">
            <button type="button" class="wiki-crumb" onclick={goToMain}>Wiki</button>
            <span aria-hidden="true">›</span>
            <span class="wiki-crumb-current">{labelForType(entity.entity_type)}</span>
            <span aria-hidden="true">›</span>
            <span class="wiki-crumb-current">{entity.name}</span>
          </nav>

          <header class="wiki-article-header">
            <h1>{entity.name}</h1>
          </header>

          {#if documentBody}
            <div class="wiki-body">
              <MarkdownArticle markdown={documentBody} {entities} onOpenEntity={openEntity} />
            </div>
          {:else}
            <div class="wiki-empty-box">
              <p class="wiki-empty">This article has no body yet.</p>
              <button class="quiet-button" type="button" onclick={handleEdit}>Write article</button>
            </div>
          {/if}

          {#if assets.length > 0}
            <section class="wiki-section">
              <h3>Attachments</h3>
              <ul class="wiki-assets">
                {#each assets as a}
                  <li>
                    <span class="wiki-asset-icon">◈</span><strong>{a.filename}</strong><small
                      >{Math.max(1, Math.round(a.size / 1024))} KB</small>
                  </li>
                {/each}
              </ul>
            </section>
          {/if}

          {#if mapLocations.length > 0}
            <section class="wiki-section">
              <h3>Maps</h3>
              <ul class="wiki-map-links">
                {#each mapLocations as loc}
                  <li>
                    <span class="wiki-rel-label">{loc.role}</span><strong>{loc.label || "Location"}</strong><small
                      >{loc.mapEntityId.slice(0, 8)}</small>
                  </li>
                {/each}
              </ul>
            </section>
          {/if}
        </article>

        <aside class="wiki-infobox" aria-label="Infobox">
          <div class="wiki-infobox-card">
            {#if profileMediaUrl}<img class="wiki-profile-media" src={profileMediaUrl} alt={`${entity.name} profile`} />{:else if profileFallback}<div class="wiki-profile-fallback" role="img" aria-label={`${entity.name} profile`}>
                <span class="wiki-profile-fallback-icon" aria-hidden="true">◆</span>
                <div><strong>{profileFallback.filename}</strong><small>Main file · {profileFallback.mime_type}</small></div>
              </div>{/if}
            <div class="wiki-infobox-header">
              <h3>{entity.name}</h3>
              <small>{labelForType(entity.entity_type)}</small>
            </div>
            <dl class="wiki-infobox-fields">
              {#if visibleFields.length === 0 && visibleRelationshipFields.length === 0}
                <p class="wiki-empty small" style="padding: 4px 0;">No infobox data. Add fields in the inspector.</p>
              {/if}
              {#each visibleFields as row}
                <div class="wiki-field-row">
                  <dt>{row.label}</dt>
                  <dd>{row.value}</dd>
                </div>
              {/each}
              {#each visibleRelationshipFields as row}
                <div class="wiki-field-row">
                  <dt>{row.label}</dt>
                  <dd>
                    {#each row.targets as t, i}
                      <button type="button" class="wiki-link small" onclick={() => openEntity(t.id)}>{t.name}</button
                      >{#if i < row.targets.length - 1},
                      {/if}
                    {/each}
                  </dd>
                </div>
              {/each}
            </dl>
          </div>

          {#if entity}
            <div class="wiki-toc-card">
              <strong>Connections</strong>
              {#if outbound.length === 0 && inbound.length === 0}
                <p class="wiki-empty small">No linked articles yet.</p>
              {:else}
                {#if outbound.length > 0}
                  <p class="wiki-section-hint">Links from this article:</p>
                  <ul class="wiki-relationships">
                    {#each outbound as rel}
                      {@const otherId = rel.target_id}
                      {@const otherType = entityTypeOf(otherId)}
                      <li>
                        <span class="wiki-rel-label">{humanizeType(rel.relationship_type)}</span>
                        <span class="wiki-rel-arrow" aria-hidden="true">→</span>
                        <button type="button" class="wiki-link small" onclick={() => openEntity(otherId)}>
                          {entityName(otherId)} <small>{otherType ? `· ${labelForType(otherType)}` : ""}</small>
                        </button>
                      </li>
                    {/each}
                  </ul>
                {/if}
                {#if inbound.length > 0}
                  <p class="wiki-section-hint">Links here:</p>
                  <ul class="wiki-backlinks">
                    {#each inbound as rel}
                      {@const otherId = rel.source_id}
                      {@const otherType = entityTypeOf(otherId)}
                      <li>
                        <span class="wiki-rel-arrow" aria-hidden="true">←</span>
                        <button type="button" class="wiki-link small" onclick={() => openEntity(otherId)}
                          >{entityName(otherId)}</button>
                        <span class="wiki-backlink-meta"
                          >· {humanizeType(rel.relationship_type)}{otherType
                            ? ` · ${labelForType(otherType)}`
                            : ""}</span>
                      </li>
                    {/each}
                  </ul>
                {/if}
              {/if}
            </div>

            <div class="wiki-toc-card subtle">
              <strong>On this page</strong>
              <p class="wiki-empty small">Headings from the article appear in the body’s table of contents.</p>
            </div>
          {/if}
        </aside>
      </div>
    {:else}
      <p class="wiki-empty">Article not found.</p>
    {/if}
  </div>
</section>

<style>
.wiki-shell {
  display: flex;
  min-height: 0;
  height: calc(100vh - 58px);
  flex-direction: column;
  background: var(--canvas, #fbf8f0);
}
.wiki-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 16px;
  min-height: 70px;
  padding: 12px 40px;
  border-bottom: 1px solid var(--line, #e9e1d4);
  background: var(--surface, #fffefa);
}
.wiki-header-left {
  display: flex;
  align-items: center;
  gap: 16px;
  min-width: 0;
}
.wiki-title h1 {
  margin: 4px 0 0;
  font: 500 26px/1.05 var(--font-display, Georgia, serif);
  color: var(--ink, #302c26);
}
.wiki-title small {
  color: var(--ink-soft, #8f897e);
  font-size: 11px;
}
.overline {
  color: var(--ink-faint, #b8b0a0);
  font-size: 10px;
  letter-spacing: 0.08em;
  text-transform: uppercase;
}
.quiet-button {
  padding: 8px 10px;
  border: 1px solid #ded8cd;
  border-radius: 8px;
  background: var(--surface, #fffefa);
  color: var(--ink-soft, #62594e);
  font: 500 11px/1.2 var(--font-body, Inter, ui-sans-serif, system-ui, sans-serif);
  cursor: pointer;
}
.quiet-button:hover {
  background: var(--surface-muted, #f7f1e7);
  color: var(--ink);
}
.quiet-button:disabled {
  opacity: 0.4;
  cursor: not-allowed;
}
.quiet-button.small {
  padding: 4px 8px;
  font-size: 11px;
}
.quiet-button.small.icon {
  padding: 4px 9px;
  line-height: 1;
  font-size: 14px;
}
.quiet-button:active {
  transform: translateY(1px);
}
.wiki-container {
  flex: 1;
  overflow: auto;
  padding: 28px 40px 40px;
}
.wiki-main-intro {
  max-width: 720px;
  margin-bottom: 24px;
}
.wiki-main-intro h2 {
  margin: 0 0 6px;
  font: 500 22px var(--font-display, Georgia, serif);
  color: var(--ink);
}
.wiki-main-intro p {
  margin: 0 0 14px;
  color: var(--ink-soft);
  font-size: 12px;
  line-height: 1.6;
}
.wiki-search {
  display: flex;
  align-items: center;
  gap: 8px;
  max-width: 360px;
  padding: 9px 12px;
  border: 1px solid var(--line);
  border-radius: 10px;
  background: var(--surface);
  box-shadow: 0 1px 2px rgba(38, 42, 33, 0.04);
}
.wiki-search input {
  flex: 1;
  border: none;
  background: transparent;
  outline: none;
  font: inherit;
  font-size: 12px;
  color: var(--ink);
}
.wiki-groups {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(260px, 1fr));
  gap: 16px;
}
.wiki-group {
  border: 1px solid var(--line);
  border-radius: 12px;
  background: var(--surface);
  padding: 16px;
  box-shadow: 0 1px 3px rgba(38, 42, 33, 0.06);
}
.wiki-group-header {
  display: flex;
  align-items: baseline;
  justify-content: space-between;
  margin-bottom: 12px;
  padding-bottom: 8px;
  border-bottom: 1px solid var(--line);
}
.wiki-group-header h3 {
  margin: 0;
  font: 600 14px var(--font-display);
  color: var(--ink);
}
.wiki-group-header small {
  color: var(--ink-faint);
  font-size: 11px;
  background: var(--surface-muted);
  padding: 2px 7px;
  border-radius: 999px;
}
.wiki-group-list {
  list-style: none;
  margin: 0;
  padding: 0;
  display: grid;
  gap: 4px;
}
.wiki-link {
  border: none;
  background: transparent;
  color: var(--accent-dark, #365342);
  font: inherit;
  font-size: 12px;
  text-align: left;
  cursor: pointer;
  padding: 0;
  display: inline-flex;
  gap: 6px;
  align-items: baseline;
}
.wiki-link:hover {
  text-decoration: underline;
  text-underline-offset: 2px;
}
.wiki-link.small {
  font-size: 11px;
}
.wiki-link-name {
  font-weight: 500;
  color: var(--accent-dark);
}
.wiki-empty {
  color: var(--ink-soft);
  font-size: 12px;
  line-height: 1.6;
}
.wiki-empty.small {
  font-size: 11px;
}
.wiki-empty-box {
  display: grid;
  place-items: start;
  gap: 10px;
  padding: 18px;
  border: 1px dashed var(--line);
  border-radius: 10px;
  background: var(--surface-muted);
}
.wiki-layout {
  display: grid;
  grid-template-columns: minmax(0, 1fr) 320px;
  gap: 24px;
  align-items: start;
  max-width: 1180px;
  margin: 0 auto;
  width: 100%;
}
.wiki-article {
  min-width: 0;
  border: 1px solid var(--line);
  border-radius: 12px;
  background: var(--surface);
  padding: 24px 26px;
  box-shadow: 0 2px 8px rgba(38, 42, 33, 0.06);
}
.wiki-breadcrumb {
  display: flex;
  align-items: center;
  gap: 6px;
  margin-bottom: 16px;
  font-size: 12px;
  color: var(--ink-soft);
}
.wiki-nav-history {
  display: flex;
  gap: 8px;
  margin-bottom: 12px;
}
.wiki-crumb {
  border: none;
  background: transparent;
  color: var(--accent-dark);
  cursor: pointer;
  font: inherit;
  padding: 0;
}
.wiki-crumb:hover {
  text-decoration: underline;
}
.wiki-crumb-current {
  color: var(--ink);
  font-weight: 600;
}
.wiki-article-header {
  margin-bottom: 16px;
  padding-bottom: 14px;
  border-bottom: 1px solid var(--line);
}
.wiki-article-header h1 {
  margin: 0 0 8px;
  font: 600 28px/1.1 var(--font-display, Georgia, serif);
  color: var(--ink);
}
.wiki-body {
  min-width: 0;
}
.wiki-section {
  margin-top: 26px;
  padding-top: 18px;
  border-top: 1px solid var(--line);
}
.wiki-section h3 {
  margin: 0 0 10px;
  font: 600 13px var(--font-display);
  color: var(--ink);
}
.wiki-section-hint {
  margin: 9px 0 10px;
  color: var(--ink-faint);
  font-size: 11px;
}
.wiki-relationships,
.wiki-assets,
.wiki-map-links,
.wiki-backlinks {
  list-style: none;
  margin: 0;
  padding: 0;
  display: grid;
  gap: 8px;
  font-size: 12px;
}
.wiki-relationships li,
.wiki-backlinks li {
  display: flex;
  align-items: baseline;
  gap: 8px;
  padding: 6px 8px;
  border-radius: 8px;
  background: var(--surface-muted);
  border: 1px solid transparent;
}
.wiki-rel-label {
  color: var(--ink-faint);
  font-size: 10px;
  text-transform: uppercase;
  letter-spacing: 0.06em;
  font-weight: 600;
}
.wiki-rel-arrow {
  color: var(--ink-faint);
}
.wiki-backlink-meta {
  color: var(--ink-faint);
  font-size: 11px;
}
.wiki-assets li,
.wiki-map-links li {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 6px 8px;
  border-radius: 8px;
  background: var(--surface-muted);
}
.wiki-asset-icon {
  color: var(--accent-dark);
  font-size: 10px;
}
.wiki-infobox {
  display: grid;
  gap: 16px;
  position: sticky;
  top: 0;
}
.wiki-infobox-card {
  border: 1px solid var(--line);
  border-radius: 12px;
  background: var(--surface);
  overflow: hidden;
  box-shadow: 0 2px 8px rgba(38, 42, 33, 0.06);
}
.wiki-profile-media {
  display: block;
  width: 100%;
  max-height: 360px;
  object-fit: cover;
  border-bottom: 1px solid var(--line);
  background: var(--surface-muted);
}
.wiki-profile-fallback {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 14px 16px;
  border-bottom: 1px solid var(--line);
  background: var(--surface-muted);
}
.wiki-profile-fallback-icon {
  display: grid;
  place-items: center;
  width: 36px;
  height: 36px;
  border-radius: 8px;
  background: #ede2d2;
  color: var(--accent);
  font-size: 14px;
}
.wiki-profile-fallback div {
  min-width: 0;
}
.wiki-profile-fallback strong {
  display: block;
  font-size: 12px;
  color: var(--ink);
}
.wiki-profile-fallback small {
  display: block;
  margin-top: 2px;
  color: var(--ink-faint);
  font-size: 11px;
}
.wiki-infobox-header {
  padding: 14px 16px;
  border-bottom: 1px solid var(--line);
  background: linear-gradient(180deg, var(--surface-muted) 0%, var(--surface) 100%);
}
.wiki-infobox-header h3 {
  margin: 0;
  font: 600 16px var(--font-display);
  color: var(--ink);
}
.wiki-infobox-header small {
  color: var(--ink-soft);
  font-size: 11px;
}
.wiki-infobox-fields {
  margin: 0;
  padding: 14px 16px;
  display: grid;
  gap: 12px;
}
.wiki-field-row {
  display: grid;
  gap: 3px;
}
.wiki-field-row dt {
  color: var(--ink-faint);
  font-size: 10px;
  letter-spacing: 0.06em;
  text-transform: uppercase;
  font-weight: 600;
}
.wiki-field-row dd {
  margin: 0;
  color: var(--ink);
  font-size: 12px;
  line-height: 1.5;
}
.wiki-toc-card {
  border: 1px solid var(--line);
  border-radius: 12px;
  background: var(--surface);
  padding: 14px 16px;
  box-shadow: 0 1px 3px rgba(38, 42, 33, 0.04);
}
.wiki-toc-card strong {
  display: block;
  margin-bottom: 8px;
  font-size: 11px;
  letter-spacing: 0.06em;
  text-transform: uppercase;
  color: var(--ink-faint);
}
.wiki-toc-card.subtle {
  background: var(--surface-muted);
}
@media (max-width: 1080px) {
  .wiki-layout {
    grid-template-columns: 1fr;
  }
  .wiki-infobox {
    position: static;
  }
}
@media (max-width: 760px) {
  .wiki-header {
    display: grid;
    gap: 10px;
    padding: 14px 17px;
  }
  .wiki-header-left {
    flex-direction: column;
    align-items: flex-start;
  }
  .wiki-container {
    padding: 20px 17px 28px;
  }
  .wiki-article {
    padding: 18px 16px;
  }
}
</style>
