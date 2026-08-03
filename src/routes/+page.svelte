<script lang="ts">
  import { onMount } from "svelte";
  import { project, type Asset, type Entity, type Relationship, type ProjectModuleManifest, type ProjectInfo, type GitStatus, type GitLogEntry } from "$lib/project/client";
  import type { EntityTemplate, ModuleContext, ModuleId, UUID } from "../../packages/module-api/src/index";
  import { bundledModules, ModuleRegistry } from "$lib/modules/registry";
  import { buildModuleContext } from "$lib/modules/context";
  import { viewRegistry } from "$lib/modules/view-registry";
  import RichTextEditor from "$lib/editor/RichTextEditor.svelte";
  import { formatCalendarDate, isCompleteCalendarDate, parseCalendarDate, serializeCalendarDate, type CalendarDate } from "$lib/date";

  type InstalledModule = ProjectModuleManifest;
  type RecentProject = { name: string; root: string };

  const recentProjectsKey = "worldbuilder.recent-projects";

  let ready = $state(false);
  let error = $state("");
  let section = $state<"lore" | "timeline">("lore");
  let entities = $state<Entity[]>([]);
  let selected = $state<Entity | null>(null);
  let documentBody = $state("");
  let fields = $state<Record<string, string>>({});
  let relationships = $state<Relationship[]>([]);
  let assets = $state<Asset[]>([]);
  let modules = $state<InstalledModule[]>([]);
  let query = $state("");
  let globalQuery = $state("");
  let name = $state("");
  let selectedTemplate = $state("person");
  let relationshipQuery = $state("");
  let relationshipType = $state("related_to");
  let relationshipTarget = $state<Entity | null>(null);
  let isSaving = $state(false);
  let savedAt = $state("");
  let showModules = $state(false);
  let moduleHost = $state<HTMLElement | null>(null);
  let projectionRevision = $state(0);
  let projectInfo = $state<ProjectInfo | null>(null);
  let gitStatus = $state<GitStatus | null>(null);
  let gitLog = $state<GitLogEntry[]>([]);
  let showGit = $state(false);
  let gitBusy = $state(false);
  let gitMessage = $state("");
  let showProjectMenu = $state(false);
  let recentProjects = $state<RecentProject[]>([]);
  let searchMatches = $state<Entity[] | null>(null);
  let searchRequest = 0;
  let showCreateForm = $state(false);
  let showCommitForm = $state(false);
  let commitMessage = $state("");
  let showProjection = $state(false);
  let dateEditorOpen = $state<Record<string, boolean>>({});

  const moduleRegistry = new ModuleRegistry();
  for (const module of bundledModules) moduleRegistry.register(module);

  const activeModuleId = () => section === "lore" ? "worldbuilder.lore" : "worldbuilder.timeline";
  const activeModule = () => bundledModules.find((module) => module.manifest.id === activeModuleId());
  const activeManifest = () => activeModule()?.manifest;
  const templates = () => activeManifest()?.templates ?? [];
  const definitions = () => activeManifest()?.schemas.flatMap((schema) => schema.fields) ?? [];

  function contextFor(currentSection = section): ModuleContext {
    const id = currentSection === "lore" ? "worldbuilder.lore" : "worldbuilder.timeline";
    const module = bundledModules.find((candidate) => candidate.manifest.id === id);
    if (!module) throw new Error(`Unknown module: ${id}`);
    if (!projectInfo?.root) throw new Error("No project is open");
    return buildModuleContext(module.manifest, projectInfo.root);
  }

  function sectionEnabled() { return modules.find((module) => module.id === activeModuleId())?.enabled ?? false; }

  function visibleEntities() {
    const term = query.trim().toLowerCase();
    return entities.filter((entity) => {
      const belongs = section === "timeline" ? entity.entity_type === "event" : entity.entity_type !== "event";
      return belongs && (!term || `${entity.name} ${entity.entity_type ?? ""}`.toLowerCase().includes(term));
    });
  }

  function entityGlyph(entity: Pick<Entity, "entity_type">) {
    return entity.entity_type === "person" ? "P" : entity.entity_type === "place" ? "L" : entity.entity_type === "faction" ? "F" : entity.entity_type === "artifact" ? "A" : entity.entity_type === "culture" ? "C" : entity.entity_type === "event" ? "E" : "?";
  }

  function entityGlyphClass(entity: Pick<Entity, "entity_type">) {
    return `entity-glyph-${entity.entity_type ?? "unknown"}`;
  }

  function selectSearchResult(entity: Entity) {
    section = entity.entity_type === "event" ? "timeline" : "lore";
    showProjection = false;
    globalQuery = "";
    query = "";
    void selectEntity(entity);
  }

  function switchSection(next: "lore" | "timeline") {
    section = next;
    selected = null;
    showProjection = false;
  }

  function relationshipCandidates() {
    const term = relationshipQuery.trim().toLowerCase();
    return entities.filter((entity) => entity.id !== selected?.id && !entity.deleted && (!term || `${entity.name} ${entity.entity_type ?? ""}`.toLowerCase().includes(term))).slice(0, 8);
  }

  function normalizeDocument(body: string, format?: string) {
    if (format === "rich-text") return body;
    const escaped = body.replaceAll("&", "&amp;").replaceAll("<", "&lt;").replaceAll(">", "&gt;");
    return escaped.split("\n").map((line) => line ? `<p>${line}</p>` : "").join("");
  }

  function dateForField(key: string) {
    return parseCalendarDate(fields[key]);
  }
  function dateDraftForField(key: string): Partial<CalendarDate> | null {
    return dateForField(key) ?? (dateEditorOpen[key] ? { calendar: "gregorian", era: "CE", precision: "day" } : null);
  }
  function openDateEditor(key: string) {
    dateEditorOpen = { ...dateEditorOpen, [key]: true };
    fields = { ...fields, [key]: "" };
  }
  function updateDateField(key: string, patch: Partial<CalendarDate>) {
    const current = dateForField(key) ?? { calendar: "gregorian", era: "CE", year: 1, month: 1, day: 1, precision: "day" };
    const next = { ...current, ...patch } as CalendarDate;
    if (patch.precision === "year") { delete next.month; delete next.day; }
    if (patch.precision === "month" && next.month === undefined) next.month = 1;
    if (patch.precision === "day") { next.month ??= 1; next.day ??= 1; }
    fields = { ...fields, [key]: serializeCalendarDate(next) };
  }
  function updateDatePart(key: string, part: "year" | "month" | "day", raw: string, min: number, max?: number) {
    if (!raw.trim()) return;
    const parsed = Math.floor(Number(raw));
    if (!Number.isFinite(parsed)) return;
    const value = Math.min(max ?? parsed, Math.max(min, parsed));
    updateDateField(key, { [part]: value });
  }
  function clearDateField(key: string) { fields = { ...fields, [key]: "" }; dateEditorOpen = { ...dateEditorOpen, [key]: false }; }

  function wordCount() { return documentBody.replace(/<[^>]*>/g, " ").trim().split(/\s+/).filter(Boolean).length; }
  function friendlyError(cause: unknown) {
    const message = cause instanceof Error ? cause.message : String(cause);
    return message.includes("invoke") || message.includes("undefined") ? "The desktop bridge is unavailable. Open this workspace in the Tauri app to use local project storage." : message;
  }
  function rememberProject(info: ProjectInfo) {
    recentProjects = [{ name: info.name, root: info.root }, ...recentProjects.filter((project) => project.root !== info.root)].slice(0, 6);
    localStorage.setItem(recentProjectsKey, JSON.stringify(recentProjects));
  }
  function removeRecentProject(root: string) {
    recentProjects = recentProjects.filter((project) => project.root !== root);
    localStorage.setItem(recentProjectsKey, JSON.stringify(recentProjects));
  }
  function loadRecentProjects() {
    try {
      const stored = JSON.parse(localStorage.getItem(recentProjectsKey) ?? "[]");
      if (Array.isArray(stored)) recentProjects = stored.filter((item): item is RecentProject => typeof item?.name === "string" && typeof item?.root === "string").slice(0, 6);
    } catch {
      recentProjects = [];
    }
  }
  async function loadEntities() {
    entities = await project.listEntities();
    projectionRevision += 1;
  }

  async function refreshGit() {
    gitBusy = true;
    gitMessage = "";
    try {
      gitStatus = await project.gitStatus();
      gitLog = gitStatus.repository ? await project.gitLog() : [];
    } catch (cause) { gitMessage = friendlyError(cause); }
    finally { gitBusy = false; }
  }

  async function finishOpening(info?: ProjectInfo) {
    projectInfo = info ?? await project.info();
    if (!projectInfo) throw new Error("The project did not return an identity");
    modules = await project.listModuleManifests();
    for (const module of bundledModules) if (modules.find((candidate) => candidate.id === module.manifest.id)?.enabled) {
      await project.enableModule(module.manifest.id);
      await moduleRegistry.enable(module.manifest.id, buildModuleContext(module.manifest, projectInfo.root));
    }
    rememberProject(projectInfo);
    await loadEntities();
    await refreshGit();
    ready = true;
  }

  async function openWorkspace() {
    error = "";
    try {
      await project.openDefault();
      await finishOpening();
    } catch (cause) { error = friendlyError(cause); }
  }

  async function openProjectDirectory() {
    try {
      const selection = await project.pickDirectory();
      const path = typeof selection === "string" ? selection : null;
      if (!path) return;
      await project.close();
      await finishOpening(await project.openDirectory(path));
    } catch (cause) { error = friendlyError(cause); }
  }

  async function openRecentProject(path: string) {
    error = "";
    try {
      await project.close();
      await finishOpening(await project.openDirectory(path));
    } catch (cause) { error = friendlyError(cause); }
  }

  async function closeProject() {
    try {
      await project.close();
      for (const module of bundledModules) if (moduleRegistry.isEnabled(module.manifest.id)) moduleRegistry.disable(module.manifest.id);
      clearSelection();
      projectInfo = null;
      gitStatus = null;
      gitLog = [];
      ready = false;
    } catch (cause) { error = friendlyError(cause); }
  }

  async function initializeGit() {
    gitBusy = true;
    gitMessage = "";
    try { await project.gitInit(); await refreshGit(); }
    catch (cause) { gitMessage = friendlyError(cause); gitBusy = false; }
  }
  async function commitGit() {
    const message = commitMessage.trim();
    if (!message) return;
    showCommitForm = false;
    gitBusy = true;
    gitMessage = "";
    try { await project.gitCommit(message); commitMessage = ""; await refreshGit(); }
    catch (cause) { gitMessage = friendlyError(cause); gitBusy = false; }
  }

  async function selectEntity(entity: Entity) {
    selected = entity;
    error = "";
    try {
      const context = contextFor();
      const record = await context.entities.get(entity.id as UUID);
      documentBody = normalizeDocument(record?.documents[0]?.body ?? "", record?.documents[0]?.format);
      const values = await context.fields.list(entity.id as UUID);
      dateEditorOpen = {};
      fields = Object.fromEntries(Object.entries(values).map(([key, value]) => {
        const definition = definitions().find((candidate) => candidate.key === key);
        if (definition?.type === "date") {
          const date = parseCalendarDate(value);
          const normalized = date ? serializeCalendarDate(date) : "";
          if (normalized === "1" || normalized === "1-1" || normalized === "1-1-1") return [key, ""];
          return [key, date ? serializeCalendarDate(date) : String(value ?? "")];
        }
        return [key, String(value ?? "")];
      }));
      relationships = (await context.relationships.list(entity.id as UUID)).map((relationship) => ({ id: relationship.id, source_id: relationship.sourceId, target_id: relationship.targetId, relationship_type: relationship.type, metadata: JSON.stringify(relationship.metadata) }));
      assets = (await context.assets.list(entity.id as UUID)).map((asset) => ({ id: asset.id, entity_id: asset.entityId, namespace: asset.namespace, filename: asset.filename, content_hash: asset.contentHash, size: asset.size, mime_type: asset.mimeType, path: asset.path, created_at: asset.createdAt }));
      savedAt = "";
    } catch (cause) { error = friendlyError(cause); }
  }

  async function createEntity(event: SubmitEvent) {
    event.preventDefault();
    if (!name.trim() || !sectionEnabled()) return;
    try {
      const context = contextFor();
      const template = templates().find((candidate) => candidate.id === selectedTemplate) as EntityTemplate | undefined;
      const created = await context.entities.create({ name: name.trim(), type: section === "timeline" ? "event" : template?.entityType });
      if (template) for (const [key, value] of Object.entries(template.fields)) if (value !== "") await context.fields.set(created.id, key, value);
      if (template?.document) await context.documents.save({ entityId: created.id, body: `<p>${template.document}</p>`, format: "rich-text" });
      name = "";
      showCreateForm = false;
      await loadEntities();
      await selectEntity({ id: created.id, name: created.name, entity_type: created.type, deleted: created.deleted, created_at: created.createdAt, updated_at: created.updatedAt });
    } catch (cause) { error = friendlyError(cause); }
  }

  function updateField(key: string, event: Event) { fields = { ...fields, [key]: (event.currentTarget as HTMLInputElement).value }; }
  async function saveDocument() {
    if (!selected || !sectionEnabled()) return;
    isSaving = true;
    try {
      for (const definition of definitions()) {
        const value = fields[definition.key] ?? "";
        if (definition.required && value === "") throw new Error(`${definition.label} is required`);
        if (definition.type === "date" && value !== "" && !isCompleteCalendarDate(value)) throw new Error(`${definition.label} needs a year, month, and day`);
      }
      await project.saveEntry({
        document: { entity_id: selected.id, body: documentBody, format: "rich-text" },
        fields: definitions().map((definition) => {
          const value = fields[definition.key] ?? "";
          return { entity_id: selected!.id, namespace: activeManifest()?.schemas[0]?.namespace ?? activeModuleId(), key: definition.key, value: definition.type === "date" && value ? parseCalendarDate(value) ?? value : value };
        }),
      });
      savedAt = new Date().toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" });
    } catch (cause) { error = friendlyError(cause); } finally { isSaving = false; }
  }
  async function archiveSelected() {
    if (!selected || !confirm(`Archive ${selected.name}?`)) return;
    try { await contextFor().entities.delete(selected.id as UUID); selected = null; await loadEntities(); } catch (cause) { error = friendlyError(cause); }
  }
  async function addRelationship() {
    if (!selected || !relationshipTarget) return;
    try {
      const relationship = await contextFor().relationships.create({ sourceId: selected.id as UUID, targetId: relationshipTarget.id as UUID, type: relationshipType, metadata: {} });
      relationships = [...relationships, { id: relationship.id, source_id: relationship.sourceId, target_id: relationship.targetId, relationship_type: relationship.type, metadata: "{}" }];
      relationshipTarget = null; relationshipQuery = "";
    } catch (cause) { error = friendlyError(cause); }
  }
  function mimeTypeFor(filename: string) {
    const extension = filename.split(".").pop()?.toLowerCase();
    return extension === "png" ? "image/png" : extension === "jpg" || extension === "jpeg" ? "image/jpeg" : extension === "gif" ? "image/gif" : extension === "mp4" ? "video/mp4" : extension === "webm" ? "video/webm" : "application/octet-stream";
  }
  async function attachAsset() {
    if (!selected) return;
    try {
      const selection = await project.pickFile();
      const source = typeof selection === "string" ? selection : null;
      if (!source) return;
      const filename = source.split(/[\\/]/).pop() ?? "asset";
      const asset = await project.registerAssetFile({ entity_id: selected.id, namespace: activeManifest()?.schemas[0]?.namespace ?? activeModuleId(), source_path: source, filename, mime_type: mimeTypeFor(filename) });
      assets = [...assets, asset];
    } catch (cause) { error = friendlyError(cause); }
  }
  async function toggleModule(id: ModuleId) {
    const installed = modules.find((module) => module.id === id);
    try {
      if (installed?.enabled) { await project.disableModule(id); moduleRegistry.disable(id); }
      else { await project.enableModule(id); const module = bundledModules.find((candidate) => candidate.manifest.id === id); if (module && projectInfo) await moduleRegistry.enable(id, buildModuleContext(module.manifest, projectInfo.root)); }
      modules = await project.listModuleManifests();
      if (!sectionEnabled()) selected = null;
    } catch (cause) { error = friendlyError(cause); }
  }
  function clearSelection() {
    selected = null;
    documentBody = "";
    fields = {};
    relationships = [];
    assets = [];
    savedAt = "";
    relationshipQuery = "";
    relationshipTarget = null;
    showCreateForm = false;
  }
  async function seedExample() {
    try {
      await project.seedExample();
      clearSelection();
      await loadEntities();
      modules = await project.listModuleManifests();
      error = "Example world seeded.";
    } catch (cause) { error = friendlyError(cause); }
  }
  function linkedEntity(relationship: Relationship) { return entities.find((entity) => entity.id === (relationship.source_id === selected?.id ? relationship.target_id : relationship.source_id)); }
  function focusRelated(relationship: Relationship) { const target = linkedEntity(relationship); if (target) void selectEntity(target); }
  $effect(() => {
    const view = activeModule()?.views[0];
    if (!ready || !moduleHost || !view || !sectionEnabled()) return;
    const key = `${activeModuleId()}:${view.id}`;
    viewRegistry.mountView(activeModuleId() as ModuleId, view, moduleHost, contextFor());
    return () => viewRegistry.unmountView(key);
  });
  $effect(() => {
    const term = globalQuery.trim();
    if (!ready || !term) {
      searchMatches = null;
      return;
    }
    const request = ++searchRequest;
    void project.search(term).then((matches) => {
      if (request === searchRequest) searchMatches = matches;
    }).catch((cause) => {
      if (request === searchRequest) error = friendlyError(cause);
    });
  });
  onMount(loadRecentProjects);
</script>

<svelte:head><title>Worldbuilder Studio</title></svelte:head>

<main class="studio-shell">
  <aside class:startup-rail={!ready} class="rail">
    <div class="brand"><span class="brand-mark">W</span><div><strong>Worldbuilder</strong><small>Studio edition</small></div></div>
    {#if !ready}
      <div class="startup-actions">
        <button class="rail-button startup-primary" onclick={openProjectDirectory}><span class="rail-icon">↗</span><span>Open project folder</span></button>
      </div>
      {#if recentProjects.length > 0}
        <div class="rail-label recent-label">RECENT PROJECTS</div>
        <div class="recent-projects">{#each recentProjects as recent}<div class="recent-project"><button class="recent-project-open" onclick={() => openRecentProject(recent.root)}><span class="project-dot"></span><span><strong>{recent.name}</strong><small>{recent.root}</small></span></button><button class="recent-project-remove" aria-label={`Remove ${recent.name} from recent projects`} title="Remove from recent projects" onclick={() => removeRecentProject(recent.root)}>×</button></div>{/each}</div>
      {/if}
    {:else}
      <div class="rail-label">WORKSPACE</div>
      <button class:active={section === "lore"} class="rail-button" onclick={() => switchSection("lore")}><span class="rail-icon">✦</span><span>Lore library</span></button>
      <button class:active={section === "timeline"} class="rail-button" onclick={() => switchSection("timeline")}><span class="rail-icon">◷</span><span>Timeline</span></button>
      <div class="rail-label project-label">PROJECT</div>
      <div class="project-card"><span class:online={ready} class="project-dot"></span><div><strong>{projectInfo?.name ?? "Local project"}</strong><small>Saved in project folder</small></div></div>
      <button class:active={showProjectMenu} class="rail-button" onclick={() => showProjectMenu = !showProjectMenu}><span class="rail-icon">⋯</span><span>Project actions</span></button>
      {#if showProjectMenu}
        <div class="project-menu">
          <button class="rail-button" onclick={openProjectDirectory}><span class="rail-icon">↗</span><span>Open another folder</span></button>
          <button class="rail-button" onclick={() => { void project.rebuildSearch().catch((cause) => error = friendlyError(cause)); }}><span class="rail-icon">⌕</span><span>Rebuild index</span></button>
          <button class="rail-button" onclick={seedExample}><span class="rail-icon">✣</span><span>Seed example</span></button>
          <button class="rail-button" onclick={closeProject}><span class="rail-icon">×</span><span>Close project</span></button>
        </div>
      {/if}
    {/if}
    <div class="rail-spacer"></div>
    {#if ready}
      <button class:active={showGit} class="rail-button muted-button" onclick={() => { showGit = !showGit; if (showGit) void refreshGit(); }}><span class="rail-icon">⑂</span><span>Git</span></button>
      {#if showGit}<div class="module-menu git-menu"><strong>{gitBusy ? "Checking Git…" : gitStatus?.repository ? `Git · ${gitStatus.branch || "detached"}` : "Git is not initialized"}</strong><small>{gitMessage || (gitStatus?.repository ? gitStatus.changes.length === 0 ? "Working tree clean" : `${gitStatus.changes.length} changed files` : "Initialize Git to track this project")}</small>{#if gitStatus?.repository}<button disabled={gitBusy} onclick={() => { commitMessage = ""; showCommitForm = true; }}>Commit changes</button>{:else}<button disabled={gitBusy} onclick={initializeGit}>{gitBusy ? "Initializing…" : "Initialize Git"}</button>{/if}</div>{/if}
      <button class="rail-button muted-button" onclick={() => showModules = !showModules}><span class="rail-icon">⚙</span><span>Module settings</span></button>
      {#if showModules}<div class="module-menu">{#each modules as module}<label><span>{module.name}</span><input type="checkbox" checked={module.enabled} onchange={() => toggleModule(module.id)} /></label>{/each}</div>{/if}
    {/if}
    <div class="rail-footer">v0.2 · local first</div>
  </aside>

  <section class="app-main">
    <header class="topbar"><div class="breadcrumbs"><span>Private studio</span><i>/</i><strong>{section === "lore" ? "Lore library" : "Timeline"}</strong>{#if selected}<i>/</i><span>{selected.name}</span>{/if}</div><div class="top-actions">{#if ready}<label class="global-search"><span>⌕</span><input aria-label="Search your world" bind:value={globalQuery} placeholder="Search whole world" /></label><span class="sync-badge"><span></span> Local</span>{/if}</div></header>
    {#if ready && globalQuery.trim()}<div class="search-modal" role="dialog" aria-label="World search results"><div class="search-modal-heading"><strong>Search results</strong><button class="quiet-button" aria-label="Close search" onclick={() => globalQuery = ""}>×</button></div>{#if searchMatches === null}<p class="search-state">Searching the whole world…</p>{:else if searchMatches.length === 0}<p class="search-state">No matches found.</p>{:else}<div class="search-results">{#each searchMatches as result}<button class="search-result" onclick={() => selectSearchResult(result)}><span class={`entity-glyph ${entityGlyphClass(result)}`}>{entityGlyph(result)}</span><span><strong>{result.name}</strong><small>{result.entity_type ?? "Uncategorized"}</small></span></button>{/each}</div>{/if}</div>{/if}
    {#if showCreateForm}<div class="modal-backdrop"><form class="dialog new-form" onsubmit={createEntity}><div class="new-form-heading"><div><span class="panel-kicker">CREATE {section === "lore" ? "LORE ENTRY" : "TIMELINE EVENT"}</span><strong>Give it a name</strong></div><button type="button" class="new-form-close" onclick={() => showCreateForm = false}>×</button></div><div class="new-input"><input id="new-entity" bind:value={name} placeholder={section === "lore" ? "e.g. The western marshes" : "e.g. The coronation"} /></div>{#if section === "lore"}<label class="template-field"><span>Entry type</span><select aria-label="Entry type" bind:value={selectedTemplate}>{#each templates() as template}<option value={template.id}>{template.name}</option>{/each}</select></label>{/if}<div class="new-form-actions"><button type="button" class="quiet-button" onclick={() => showCreateForm = false}>Cancel</button><button class="primary-button" type="submit">Create {section === "lore" ? "entry" : "event"}</button></div></form></div>{/if}
    {#if showCommitForm}<div class="modal-backdrop"><form class="dialog commit-form" onsubmit={(event) => { event.preventDefault(); void commitGit(); }}><div class="new-form-heading"><div><span class="panel-kicker">VERSION CONTROL</span><strong>Commit changes</strong></div><button type="button" class="new-form-close" onclick={() => showCommitForm = false}>×</button></div><p>Save the current project changes to Git.</p><input aria-label="Commit message" bind:value={commitMessage} placeholder="Describe the changes" /><div class="new-form-actions"><button type="button" class="quiet-button" onclick={() => showCommitForm = false}>Cancel</button><button class="primary-button" type="submit" disabled={!commitMessage.trim() || gitBusy}>{gitBusy ? "Committing…" : "Commit changes"}</button></div></form></div>{/if}
    {#if !ready}
      <section class="welcome"><div class="welcome-copy"><span class="overline">A private place for impossible worlds</span><h1>Build the world<br /><em>behind the story.</em></h1><p>Shape characters, places, factions, and history in one calm, local-first studio.</p><button class="primary-button large" onclick={openWorkspace}>Open local studio <span>→</span></button><small>Everything stays on this device.</small></div><div class="welcome-art"><div class="orb orb-one"></div><div class="orb orb-two"></div><div class="art-card"><span>ELDERMERE</span><strong>The sea remembers<br />what kingdoms forget.</strong><small>Fragments · 12</small></div></div></section>
    {:else if !sectionEnabled()}
      <section class="disabled-state"><div class="disabled-icon">◌</div><span class="overline">Module unavailable</span><h1>{section === "lore" ? "Lore library" : "Timeline"} is resting.</h1><p>Your project data is safe. Re-enable this module to continue working in this workspace.</p><button class="primary-button" onclick={() => toggleModule(activeModuleId() as ModuleId)}>Enable {section === "lore" ? "Lore" : "Timeline"}</button></section>
    {:else}
      <div class="workspace-heading"><div><span class="overline">{section === "lore" ? "WORLD BIBLE" : "CHRONOLOGY"}</span><h1>{section === "lore" ? "Lore library" : "Timeline"}</h1><p>{section === "lore" ? "A living reference for every person, place, and power." : "Events, eras, and the threads that connect them."}</p></div><div class="heading-actions"><button class="quiet-button" onclick={() => showProjection = !showProjection}>{showProjection ? "Hide" : "Show"} {section === "lore" ? "graph" : "timeline"}</button></div></div>
      {#if showProjection}{#key projectionRevision}<div class="projection-bar" bind:this={moduleHost}></div>{/key}{/if}
      <section class="workspace-grid">
        <aside class="collection-panel panel-surface">
          <div class="panel-heading">
            <div><span class="panel-kicker">{section === "lore" ? "LORE LIBRARY" : "TIMELINE"}</span><strong>{visibleEntities().length} {section === "lore" ? "entries" : "events"}</strong></div>
            <button class="new-entry-button" onclick={() => { showCreateForm = !showCreateForm; if (showCreateForm) setTimeout(() => document.getElementById("new-entity")?.focus(), 0); }}>{showCreateForm ? "Close" : `New ${section === "lore" ? "entry" : "event"}`}</button>
          </div>
          <div class="collection-search"><span>⌕</span><input aria-label={`Filter ${section === "lore" ? "entries" : "events"}`} bind:value={query} placeholder={`Filter ${section === "lore" ? "entries" : "events"}`} /></div>
          <div class="collection-list">
            {#if visibleEntities().length === 0}<div class="list-empty"><span>✦</span><p>No {section === "lore" ? "entries" : "events"} found.</p><small>{query ? "Try a different filter." : "Use the button above to create one."}</small></div>{:else}{#each visibleEntities() as entity}<button class:selected={selected?.id === entity.id} class="collection-item" onclick={() => selectEntity(entity)}><span class={`entity-glyph ${entityGlyphClass(entity)}`}>{entityGlyph(entity)}</span><span class="item-copy"><strong>{entity.name}</strong><small>{entity.entity_type ?? "Uncategorized"}</small></span><span class="item-arrow">›</span></button>{/each}{/if}
          </div>
        </aside>

        <article class="editor-panel"><div class="editor-header"><div><span class="panel-kicker">{selected?.entity_type ?? (section === "lore" ? "LORE ENTRY" : "TIMELINE EVENT")}</span><h2>{selected?.name ?? "Choose an entry"}</h2></div>{#if selected}<div class="editor-status">{#if isSaving}<span class="saving-dot"></span> Saving…{:else if savedAt}<span class="saved-dot">✓</span> Saved {savedAt}{/if}</div>{/if}</div>{#if selected}<RichTextEditor value={documentBody} onChange={(value) => documentBody = value} placeholder="Write the canonical story of this entry…" /><div class="editor-footer"><span>{wordCount()} words</span><div><button class="quiet-button" onclick={archiveSelected}>Archive</button><button class="primary-button" disabled={isSaving} onclick={saveDocument}>{isSaving ? "Saving…" : "Save changes"}</button></div></div>{:else}<div class="editor-empty"><div class="empty-mark">✦</div><h3>Your canvas is waiting.</h3><p>Select an entry from the library, or create something new to begin writing.</p></div>{/if}</article>

        {#if selected}<aside class="inspector-panel panel-surface"><div class="inspector-heading"><div><span class="panel-kicker">INSPECTOR</span><strong>Details</strong></div><span class="inspector-type">{selected.entity_type}</span></div><section class="inspector-section"><h3>Properties</h3>{#each definitions() as definition}<div class="property-field"><span>{definition.label}{#if definition.required}<b>*</b>{/if}</span>{#if definition.type === "date"}{#if dateForField(definition.key) || dateEditorOpen[definition.key]}{@const date = dateDraftForField(definition.key) ?? { calendar: "gregorian", era: "CE", precision: "day" }}<div class="date-editor"><div class="date-fields"><label for={`${definition.key}-year`}>Year<input id={`${definition.key}-year`} aria-label={`${definition.label} year`} type="number" min="1" value={date.year ?? ""} onchange={(event) => updateDatePart(definition.key, "year", (event.currentTarget as HTMLInputElement).value, 1)} /></label><label for={`${definition.key}-month`}>Month<input id={`${definition.key}-month`} aria-label={`${definition.label} month`} type="number" min="1" max="12" value={date.month ?? ""} onchange={(event) => updateDatePart(definition.key, "month", (event.currentTarget as HTMLInputElement).value, 1, 12)} /></label><label for={`${definition.key}-day`}>Day<input id={`${definition.key}-day`} aria-label={`${definition.label} day`} type="number" min="1" max="31" value={date.day ?? ""} onchange={(event) => updateDatePart(definition.key, "day", (event.currentTarget as HTMLInputElement).value, 1, 31)} /></label></div><small class="date-preview">{typeof date.year === "number" ? formatCalendarDate(date) : "Add a date"}</small><button class="date-clear" type="button" onclick={() => clearDateField(definition.key)}>Clear date</button></div>{:else}<button class="date-empty" type="button" onclick={() => openDateEditor(definition.key)}>Add a date</button>{/if}{:else}<input type="text" value={fields[definition.key] ?? ""} placeholder="Add {definition.label.toLowerCase()}" oninput={(event) => updateField(definition.key, event)} />{/if}</div>{/each}</section><section class="inspector-section"><div class="section-title"><h3>Relationships</h3><span>{relationships.length}</span></div>{#each relationships as relationship}<button class="relationship-chip" onclick={() => focusRelated(relationship)}><span class="relation-mark">↗</span><span><strong>{relationship.relationship_type}</strong><small>{linkedEntity(relationship)?.name ?? "Unknown entity"}</small></span></button>{/each}<div class="relationship-form"><input aria-label="Find related entity" bind:value={relationshipQuery} placeholder="Search entities…" />{#if relationshipQuery && !relationshipTarget}{#each relationshipCandidates() as candidate}<button class="candidate" onclick={() => { relationshipTarget = candidate; relationshipQuery = candidate.name; }}><span>{candidate.entity_type === "event" ? "◷" : "✦"}</span><span><strong>{candidate.name}</strong><small>{candidate.entity_type}</small></span></button>{/each}{/if}<div class="relation-controls"><select aria-label="Relationship type" bind:value={relationshipType}><option value="related_to">Related to</option><option value="located_in">Located in</option><option value="created_by">Created by</option><option value="ruler_of">Ruler of</option><option value="follows">Follows</option><option value="opposes">Opposes</option></select><button class="add-button" disabled={!relationshipTarget} onclick={addRelationship}>Link</button></div></div></section><section class="inspector-section"><div class="section-title"><h3>Attachments</h3><span>{assets.length}</span></div><button class="drop-zone" type="button" onclick={attachAsset}><span>＋</span><strong>Attach a file</strong><small>Copied into this project</small></button>{#each assets as asset}<div class="asset-row"><span class="asset-icon">□</span><span><strong>{asset.filename}</strong><small>{Math.max(1, Math.round(asset.size / 1024))} KB</small></span></div>{/each}</section></aside>{:else}<aside class="inspector-panel panel-surface inspector-empty"><span>INSPECTOR</span><p>Select an entry to see its properties, relationships, and attachments.</p></aside>{/if}
      </section>
    {/if}
    {#if error}<div class="toast" role="status">{error}<button aria-label="Dismiss" onclick={() => error = ""}>×</button></div>{/if}
  </section>
</main>

<style>
  :global(*) { box-sizing: border-box; }
  :global(:root) { --ink: #25251f; --ink-soft: #77766d; --ink-faint: #aaa79d; --line: #e4e1d8; --surface: #fffefa; --surface-muted: #f4f2ec; --canvas: #f7f6f2; --accent: #b4773f; --accent-dark: #365342; --shadow-sm: 0 2px 8px rgba(38, 42, 33, .05); --shadow-lg: 0 18px 50px rgba(38, 42, 33, .08); --font-display: Georgia, serif; }
  :global(body) { margin: 0; background: var(--canvas); color: var(--ink); font-family: Inter, ui-sans-serif, system-ui, sans-serif; }
  :global(button), :global(input), :global(select) { font: inherit; }
  .studio-shell { min-height: 100vh; display: flex; } .rail { width: 248px; flex: 0 0 248px; display: flex; flex-direction: column; padding: 25px 15px 18px; background: #283a30; color: #eef0e9; } .startup-rail { padding-top: 34px; } .brand { display: flex; align-items: center; gap: 11px; padding: 0 10px 40px; } .brand-mark { display: grid; place-items: center; width: 31px; height: 31px; border-radius: 9px; background: #d5ab6c; color: #2c4032; font: 700 18px Georgia, serif; } .brand strong, .brand small, .project-card strong, .project-card small, .recent-project strong, .recent-project small { display: block; } .brand strong { font-size: 14px; } .brand small, .project-card small, .recent-project small { margin-top: 3px; color: #aab9ad; font-size: 11px; } .rail-label { margin: 0 10px 9px; color: #819688; font-size: 10px; font-weight: 700; letter-spacing: .16em; } .recent-label { margin-top: 27px; } .project-label { margin-top: 22px; } .rail-button { width: 100%; display: flex; align-items: center; gap: 11px; padding: 10px 11px; margin-bottom: 3px; border: 0; border-radius: 8px; background: transparent; color: #b9c8bc; text-align: left; cursor: pointer; } .rail-button:hover, .rail-button.active { background: #3b5243; color: #fff; } .startup-primary { margin-top: 8px; background: #d5ab6c; color: #2c4032; font-weight: 700; } .startup-primary:hover { background: #e1bc82; color: #2c4032; } .rail-icon { width: 18px; color: #d5ab6c; text-align: center; } .startup-primary .rail-icon { color: #2c4032; } .muted-button { color: #91a397; } .rail-spacer { flex: 1; } .rail-footer { padding: 17px 10px 0; color: #708476; font-size: 11px; } .project-card { display: flex; align-items: center; gap: 10px; padding: 11px 10px 10px; } .project-card strong, .recent-project strong { font-size: 13px; max-width: 185px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; } .project-dot { flex: 0 0 auto; width: 8px; height: 8px; border-radius: 50%; background: #777f78; } .project-dot.online { background: #88c18e; box-shadow: 0 0 0 4px rgba(136,193,142,.12); } .recent-projects { display: grid; gap: 3px; } .recent-project { width: 100%; display: flex; align-items: flex-start; gap: 10px; padding: 10px; border: 0; border-radius: 8px; background: transparent; color: #eef0e9; text-align: left; cursor: pointer; } .recent-project:hover { background: #3b5243; } .recent-project small { overflow: hidden; max-width: 180px; text-overflow: ellipsis; white-space: nowrap; } .project-menu { margin: 3px 0 8px 8px; padding-left: 8px; border-left: 1px solid #486052; } .project-menu .rail-button { padding: 8px 9px; color: #aab9ad; font-size: 11px; } .module-menu { margin: 6px 8px 12px; padding: 8px 10px; border: 1px solid #486052; border-radius: 8px; background: #30483a; } .module-menu label { display: block; } .module-menu label { display: flex; justify-content: space-between; padding: 5px 0; color: #bdcabe; font-size: 11px; }
  .app-main { min-width: 0; flex: 1; } .topbar { display: flex; align-items: center; justify-content: space-between; min-height: 70px; padding: 0 40px; border-bottom: 1px solid var(--line); background: rgba(255,254,250,.78); } .breadcrumbs, .top-actions { display: flex; align-items: center; gap: 10px; } .breadcrumbs { min-width: 0; color: var(--ink-faint); font-size: 12px; } .breadcrumbs strong { color: var(--ink-soft); } .breadcrumbs span:last-child { overflow: hidden; max-width: 180px; text-overflow: ellipsis; white-space: nowrap; } .breadcrumbs i { color: #d0ccc2; font-style: normal; } .global-search { display: flex; align-items: center; gap: 8px; width: 230px; padding: 8px 10px; border: 1px solid var(--line); border-radius: 8px; background: var(--surface); color: var(--ink-faint); } .global-search input { min-width: 0; flex: 1; border: 0; outline: 0; background: transparent; color: var(--ink); font-size: 12px; } .sync-badge { color: var(--ink-faint); font-size: 10px; } .sync-badge { display: flex; align-items: center; gap: 6px; color: var(--ink-soft); } .sync-badge span { width: 6px; height: 6px; border-radius: 50%; background: #72a97a; }
  .welcome, .disabled-state { max-width: 1080px; min-height: calc(100vh - 70px); margin: auto; padding: 10vh 7vw; display: flex; align-items: center; gap: 8vw; } .welcome-copy { flex: 1; } .overline, .panel-kicker { display: block; color: var(--accent); font-size: 10px; font-weight: 800; letter-spacing: .18em; } .welcome h1 { margin: 20px 0 18px; font: 500 clamp(48px, 6vw, 78px)/.98 var(--font-display); letter-spacing: -.04em; } .welcome h1 em { color: var(--accent); font-style: italic; } .welcome p { max-width: 380px; margin: 0 0 30px; color: var(--ink-soft); font-size: 16px; line-height: 1.7; } .welcome-copy small { display: block; margin-top: 14px; color: var(--ink-faint); font-size: 11px; } .welcome-art { position: relative; width: 360px; height: 390px; } .orb { position: absolute; border-radius: 50%; } .orb-one { top: 16px; right: 15px; width: 275px; height: 275px; background: radial-gradient(circle at 33% 30%, #eed5a5, #c2794d 64%, #7b4d3f); box-shadow: 30px 35px 60px rgba(115,74,56,.22); } .orb-two { left: 10px; bottom: 36px; width: 140px; height: 140px; background: #365342; box-shadow: 14px 16px 30px rgba(45,71,54,.2); } .art-card { position: absolute; right: -10px; bottom: 0; width: 235px; padding: 22px; border: 1px solid rgba(255,255,255,.65); border-radius: 12px; background: rgba(255,254,250,.86); box-shadow: var(--shadow-lg); } .art-card span, .art-card small { display: block; color: var(--accent); font-size: 9px; font-weight: 800; letter-spacing: .16em; } .art-card strong { display: block; margin: 17px 0 27px; font: 500 20px/1.18 var(--font-display); } .art-card small { color: var(--ink-faint); font-weight: 500; letter-spacing: 0; }
  .primary-button, .quiet-button, .add-button { border: 0; border-radius: 8px; cursor: pointer; } .primary-button { padding: 10px 15px; background: var(--accent-dark); color: #fff; font-weight: 700; font-size: 12px; box-shadow: 0 5px 12px rgba(42,68,51,.14); } .primary-button:hover { background: #2b4535; } .primary-button:disabled { opacity: .55; cursor: wait; } .primary-button.large { padding: 14px 18px; font-size: 13px; } .primary-button span { margin-left: 18px; font-size: 18px; } .quiet-button { padding: 10px 12px; background: transparent; color: var(--ink-soft); font-size: 12px; } .quiet-button:hover { background: var(--surface-muted); color: var(--ink); }
  .workspace-heading { display: flex; align-items: flex-end; justify-content: space-between; gap: 20px; padding: 42px 40px 25px; } .workspace-heading h1 { margin: 8px 0 4px; font: 500 38px/1 var(--font-display); } .workspace-heading p { margin: 0; color: var(--ink-soft); font-size: 13px; } .heading-actions { display: flex; gap: 7px; } .projection-bar { min-height: 42px; margin: 0 40px 15px; padding: 0 14px; border: 1px solid var(--line); border-radius: 9px; background: rgba(255,254,250,.72); } .projection-bar:empty { display: none; } .workspace-grid { display: grid; grid-template-columns: 245px minmax(360px, 1fr) 270px; gap: 14px; padding: 0 40px 40px; align-items: start; } .panel-surface, .editor-panel { border: 1px solid var(--line); border-radius: 12px; background: var(--surface); box-shadow: var(--shadow-sm); } .collection-panel, .inspector-panel { min-height: 650px; } .collection-panel { display: flex; flex-direction: column; } .panel-heading, .inspector-heading { display: flex; align-items: center; justify-content: space-between; padding: 18px 17px 12px; } .panel-heading strong { display: block; margin-top: 5px; font: 500 28px var(--font-display); }
  .editor-panel { min-height: 650px; padding: 24px 25px 18px; } .editor-header { display: flex; align-items: flex-start; justify-content: space-between; min-height: 72px; } .editor-header h2 { margin: 8px 0 0; font: 500 28px/1.1 var(--font-display); } .editor-status { color: var(--ink-faint); font-size: 11px; } .saving-dot, .saved-dot { display: inline-block; width: 7px; height: 7px; margin-right: 5px; border-radius: 50%; background: #d6a35f; } .saved-dot { width: auto; height: auto; margin: 0 4px 0 0; color: #6fa276; background: transparent; } .editor-footer { display: flex; align-items: center; justify-content: space-between; padding-top: 14px; color: var(--ink-faint); font-size: 11px; } .editor-footer div { display: flex; gap: 4px; } .editor-empty { display: grid; place-items: center; min-height: 500px; padding: 30px; text-align: center; } .empty-mark, .disabled-icon { display: grid; place-items: center; width: 52px; height: 52px; border-radius: 16px; background: #f2e4d2; color: var(--accent); font-size: 23px; } .editor-empty h3 { margin: 18px 0 6px; font: 500 23px var(--font-display); } .editor-empty p, .disabled-state p { max-width: 280px; margin: 0; color: var(--ink-soft); font-size: 12px; line-height: 1.6; }
  .date-editor { display: grid; gap: 8px; padding: 10px; border: 1px solid var(--line); border-radius: 8px; background: #fcf8f1; } .date-fields { display: grid; grid-template-columns: repeat(3, minmax(0, 1fr)); gap: 6px; } .date-fields label { display: grid; gap: 4px; color: var(--ink-faint); font-size: 9px; font-weight: 700; text-transform: uppercase; } .date-fields input { min-width: 0; width: 100%; padding: 8px 6px; border: 1px solid var(--line); border-radius: 7px; background: var(--canvas); color: var(--ink); font-size: 11px; } .date-fields input:focus { border-color: #c99965; box-shadow: 0 0 0 3px rgba(180,119,63,.1); outline: 0; } .date-preview { color: var(--accent); font-size: 10px; font-weight: 700; } .date-clear, .date-empty { width: fit-content; padding: 0; border: 0; background: transparent; color: var(--ink-faint); font-size: 10px; cursor: pointer; } .date-empty { padding: 8px 10px; border: 1px dashed #d3c0a9; border-radius: 7px; color: var(--accent); } .inspector-heading { border-bottom: 1px solid var(--line); } .inspector-heading strong { display: block; margin-top: 7px; font: 500 20px var(--font-display); } .inspector-type { padding: 4px 7px; border-radius: 5px; background: #f2e4d2; color: var(--accent); font-size: 9px; font-weight: 800; text-transform: uppercase; } .inspector-section { padding: 18px 16px; border-bottom: 1px solid var(--line); } .inspector-section h3, .section-title h3 { margin: 0; color: var(--ink-soft); font-size: 10px; letter-spacing: .08em; text-transform: uppercase; } .property-field { display: block; margin-top: 14px; } .property-field span { display: block; margin-bottom: 5px; color: var(--ink-soft); font-size: 10px; } .property-field b { margin-left: 3px; color: var(--accent); } .property-field input, .relationship-form > input { width: 100%; padding: 8px 9px; border: 1px solid var(--line); border-radius: 7px; outline: 0; background: var(--canvas); color: var(--ink); font-size: 11px; } .property-field input:focus { border-color: #c99965; box-shadow: 0 0 0 3px rgba(180,119,63,.1); } .section-title { display: flex; align-items: center; justify-content: space-between; } .section-title span { color: var(--ink-faint); font-size: 11px; } .relationship-chip, .candidate { width: 100%; display: flex; align-items: center; gap: 9px; margin-top: 9px; padding: 8px; border: 0; border-radius: 7px; background: var(--surface-muted); color: var(--ink); text-align: left; cursor: pointer; } .relationship-chip:hover, .candidate:hover { background: #eee9df; } .relation-mark, .candidate > span:first-child { color: var(--accent); } .relationship-chip strong, .relationship-chip small, .candidate strong, .candidate small, .asset-row strong, .asset-row small { display: block; } .relationship-chip strong, .candidate strong, .asset-row strong { font-size: 10px; } .relationship-chip small, .candidate small, .asset-row small { margin-top: 3px; color: var(--ink-faint); font-size: 9px; } .relationship-form { position: relative; margin-top: 12px; } .candidate { margin-top: 3px; padding: 7px; } .relation-controls { display: flex; gap: 5px; margin-top: 6px; } .relation-controls select { min-width: 0; flex: 1; padding: 7px; border: 1px solid var(--line); border-radius: 6px; background: var(--surface); color: var(--ink-soft); font-size: 9px; } .add-button { padding: 0 9px; background: var(--accent-dark); color: #fff; font-size: 10px; } .add-button:disabled { opacity: .4; cursor: not-allowed; } .drop-zone { display: flex; flex-direction: column; align-items: center; gap: 4px; margin-top: 12px; padding: 16px 8px; border: 1px dashed #d3c0a9; border-radius: 8px; background: #fcf8f1; color: var(--accent); text-align: center; cursor: pointer; } .drop-zone span { font-size: 22px; } .drop-zone strong { color: var(--ink-soft); font-size: 10px; } .drop-zone small { color: var(--ink-faint); font-size: 9px; } .asset-row { display: flex; align-items: center; gap: 8px; margin-top: 9px; } .asset-icon { display: grid; place-items: center; width: 25px; height: 25px; border-radius: 6px; background: #ede9e0; color: var(--accent); }
  .disabled-state { display: grid; min-height: calc(100vh - 70px); place-content: center; justify-items: center; padding: 40px; text-align: center; } .disabled-state h1 { margin: 12px 0 10px; font: 500 42px var(--font-display); } .disabled-state p { margin-bottom: 24px; } .toast { position: fixed; right: 24px; bottom: 24px; z-index: 10; max-width: 430px; padding: 13px 14px; border: 1px solid #e5d4ba; border-radius: 9px; background: #fff8ed; box-shadow: var(--shadow-lg); color: #765a39; font-size: 12px; } .toast button { margin-left: 10px; border: 0; background: none; color: inherit; cursor: pointer; font-size: 17px; } .inspector-empty { display: grid; place-items: center; min-height: 240px; padding: 30px; color: var(--ink-faint); text-align: center; font-size: 10px; } .inspector-empty p { max-width: 170px; margin-top: 13px; line-height: 1.6; }
  @media (max-width: 1180px) { .workspace-grid { grid-template-columns: 220px minmax(320px, 1fr); } .inspector-panel { grid-column: 1 / -1; min-height: auto; display: grid; grid-template-columns: repeat(3, 1fr); } .inspector-heading { grid-column: 1 / -1; } .inspector-section { border-right: 1px solid var(--line); border-bottom: 0; } }
  @media (max-width: 760px) { .studio-shell { display: block; } .rail { display: block; width: 100%; height: auto; padding: 12px 14px; } .startup-rail { min-height: 100vh; padding: 24px 14px; } .brand { padding: 0 4px 12px; } .rail-label, .project-card, .rail-spacer, .rail-footer, .module-menu { display: none; } .startup-rail .rail-label, .startup-rail .recent-projects { display: block; } .startup-rail .recent-label { margin-top: 27px; } .startup-rail .rail-button { display: flex; width: 100%; margin: 0 0 5px; padding: 10px 11px; } .startup-rail .rail-button span:not(.rail-icon) { display: inline; } .rail-button { display: inline-flex; width: auto; margin: 0 3px 0 0; padding: 8px 10px; } .rail-button span:not(.rail-icon) { display: none; } .topbar { min-height: 58px; padding: 0 17px; } .breadcrumbs span:first-child, .sync-badge { display: none; } .global-search { width: 150px; } .welcome { min-height: calc(100vh - 58px); display: block; padding: 55px 24px; } .welcome h1 { font-size: 52px; } .welcome-art { width: 100%; height: 270px; margin-top: 35px; transform: scale(.84); transform-origin: left top; } .workspace-heading { display: block; padding: 30px 17px 18px; } .workspace-heading h1 { font-size: 33px; } .heading-actions { margin-top: 18px; } .projection-bar { margin: 0 17px 12px; } .workspace-grid { display: flex; flex-direction: column; padding: 0 17px 25px; } .collection-panel, .editor-panel, .inspector-panel { width: 100%; min-height: auto; } .collection-list { max-height: 260px; overflow-y: auto; } .inspector-panel { display: block; } .inspector-section { border-bottom: 1px solid var(--line); border-right: 0; } .editor-panel { padding: 18px 14px 14px; } .editor-header h2 { font-size: 24px; } .editor-footer { align-items: flex-end; gap: 10px; } .toast { right: 12px; bottom: 12px; left: 12px; } }
  :global(.projection-bar) { min-height: 0; padding: 0; border: 0; background: transparent; box-shadow: none; }
  :global(.projection-graph), :global(.timeline-projection) { overflow: hidden; border: 1px solid var(--line); border-radius: 12px; background: var(--surface); box-shadow: var(--shadow-sm); }
  :global(.projection-header) { display: flex; align-items: baseline; justify-content: space-between; padding: 14px 17px 10px; border-bottom: 1px solid var(--line); }
  :global(.projection-header h3) { margin: 0; font: 500 18px var(--font-display); }
  :global(.projection-header small) { color: var(--ink-faint); font-size: 10px; }
  :global(.projection-graph svg) { display: block; width: 100%; height: 230px; background: linear-gradient(135deg, #fbfaf5, #f5f1e8); }
  :global(.projection-edge) { stroke: #c9b89f; stroke-width: 1.5; }
  :global(.projection-node) { fill: #fffefa; stroke: #b4773f; stroke-width: 2; }
  :global(.projection-node-label) { fill: #25251f; font: 600 11px Inter, ui-sans-serif, system-ui, sans-serif; }
  :global(.projection-node-type) { fill: #8f897e; font: 9px Inter, ui-sans-serif, system-ui, sans-serif; }
  :global(.timeline-track) { position: relative; display: grid; gap: 12px; padding: 18px 20px 20px 42px; background: linear-gradient(90deg, transparent 29px, #d5ab6c 29px, #d5ab6c 31px, transparent 31px); }
  :global(.timeline-event) { position: relative; display: grid; grid-template-columns: 92px 1fr; gap: 14px; align-items: center; }
  :global(.timeline-event::before) { content: ""; position: absolute; left: -19px; width: 9px; height: 9px; border: 3px solid #fffefa; border-radius: 50%; background: #b4773f; box-shadow: 0 0 0 1px #b4773f; }
  :global(.timeline-date) { color: #9a7550; font-size: 10px; font-weight: 700; }
  :global(.timeline-card) { padding: 10px 12px; border: 1px solid var(--line); border-radius: 8px; background: #fffefa; }
  :global(.timeline-card strong), :global(.timeline-card small) { display: block; }
  :global(.timeline-card strong) { font: 500 15px var(--font-display); }
  :global(.timeline-card small) { margin-top: 3px; color: var(--ink-faint); font-size: 10px; }
  .recent-project { display: grid; grid-template-columns: minmax(0, 1fr) 24px; min-width: 0; gap: 4px; }
  .recent-project-open { display: flex; align-items: flex-start; width: 100%; min-width: 0; gap: 10px; padding: 0; border: 0; background: transparent; color: inherit; text-align: left; cursor: pointer; }
  .recent-project-open > span:last-child { min-width: 0; overflow: hidden; }
  .recent-project strong, .recent-project small { max-width: 100%; }
  .recent-project-open:focus-visible, .recent-project-remove:focus-visible { outline: 2px solid #d5ab6c; outline-offset: 2px; }
  .recent-project-remove { flex: 0 0 auto; width: 24px; height: 24px; margin: -2px -3px 0 0; padding: 0; border: 0; border-radius: 6px; background: transparent; color: #91a397; font-size: 18px; line-height: 1; cursor: pointer; }
  .recent-project-remove:hover { background: #486052; color: #fff; }
  .collection-item { appearance: none; border: 1px solid transparent; box-shadow: 0 1px 2px rgba(38, 42, 33, .03); }
  .collection-item:hover { border-color: #e5d8c6; box-shadow: var(--shadow-sm); }
  .collection-item.selected { border-color: #d8c3a5; box-shadow: inset 3px 0 var(--accent), var(--shadow-sm); }
  .collection-list { display: grid; align-content: start; gap: 5px; padding: 2px 10px 10px; }
  .collection-item { display: flex; align-items: center; gap: 9px; width: 100%; min-height: 58px; margin: 0; padding: 9px 10px; border: 1px solid #ebe7de; border-radius: 9px; background: #fffefa; color: var(--ink); font: inherit; line-height: 1.2; text-align: left; text-decoration: none; cursor: pointer; }
  .collection-item:focus-visible { outline: 3px solid rgba(180, 119, 63, .25); outline-offset: 1px; }
  .collection-item .entity-glyph { display: grid; place-items: center; flex: 0 0 40px; width: 40px; height: 40px; border: 0; border-radius: 50%; background: #f0ece5; font-size: 13px; font-weight: 800; line-height: 1; letter-spacing: .02em; }
  .search-result .entity-glyph { display: grid; place-items: center; width: 28px; height: 28px; border-radius: 50%; font-size: 10px; font-weight: 800; }
  .entity-glyph-person { color: #9b6847; background: #f8eadf !important; }
  .entity-glyph-place { color: #557d63; background: #e8f0e8 !important; }
  .entity-glyph-faction { color: #7b638e; background: #eee8f3 !important; }
  .entity-glyph-artifact { color: #a2783c; background: #f7eed8 !important; }
  .entity-glyph-culture { color: #4e7890; background: #e4eff3 !important; }
  .entity-glyph-event { color: #ae6a56; background: #f8e8e2 !important; }
  .entity-glyph-unknown { color: #837d73; background: #eeeae3 !important; }
  .collection-item .item-copy { display: grid; align-content: center; gap: 4px; }
  .collection-item .item-copy strong { color: var(--ink); font-size: 13px; font-weight: 700; line-height: 1.15; }
  .collection-item .item-copy small { width: max-content; max-width: 150px; margin: 0; padding: 3px 6px; border-radius: 4px; background: #f4f0e8; color: var(--ink-faint); font-size: 10px; line-height: 1; letter-spacing: .04em; text-transform: uppercase; }
  .collection-item .item-arrow { padding-left: 6px; color: #c3b6a4; font-size: 18px; line-height: 1; }
  .collection-item:hover .item-arrow, .collection-item.selected .item-arrow { color: var(--accent); }
  .new-entry-button { padding: 9px 11px; border: 1px solid #c6d4c8; border-radius: 7px; background: var(--accent-dark); color: #fff; font-size: 11px; font-weight: 700; cursor: pointer; }
  .new-entry-button:hover { background: #2b4535; }
  .new-form { margin: 8px 10px 12px; padding: 15px; border: 1px solid #dccbb4; border-radius: 10px; background: #fcf8f1; }
  .new-form-heading { display: flex; align-items: flex-start; justify-content: space-between; gap: 10px; margin-bottom: 13px; }
  .new-form-heading strong { display: block; margin-top: 5px; font: 500 19px var(--font-display); }
  .new-form-close { border: 0; background: transparent; color: var(--ink-faint); font-size: 20px; line-height: 1; cursor: pointer; }
  .new-form .new-input { display: block; border: 0; overflow: visible; }
  .new-form .new-input input { width: 100%; padding: 10px 11px; border: 1px solid #d9cdbd; border-radius: 7px; background: var(--surface); color: var(--ink); font-size: 12px; }
  .template-field { display: block; margin-top: 10px; }
  .template-field span { display: block; margin-bottom: 5px; color: var(--ink-soft); font-size: 10px; font-weight: 700; }
  .template-field select { width: 100%; padding: 9px; border: 1px solid #d9cdbd; border-radius: 7px; background: var(--surface); color: var(--ink); font-size: 11px; }
  .new-form-actions { display: flex; justify-content: flex-end; gap: 7px; margin-top: 14px; }
  .new-form-actions .quiet-button { padding: 9px 10px; }
  .modal-backdrop { position: fixed; inset: 0; z-index: 20; display: grid; place-items: center; padding: 20px; background: rgba(37, 37, 31, .28); }
  .dialog { width: min(440px, 100%); margin: 0; padding: 22px; border: 1px solid #e3d9ca; border-radius: 14px; background: var(--surface); box-shadow: 0 22px 70px rgba(37, 37, 31, .2); }
  .dialog .new-form-heading { margin-bottom: 18px; }
  .dialog .new-form-heading strong { font-size: 23px; }
  .dialog .new-form-close { width: 30px; height: 30px; border-radius: 7px; background: var(--surface-muted); color: var(--ink-soft); }
  .dialog .new-form-close:hover { background: #ebe6dd; color: var(--ink); }
  .commit-form p { margin: 0 0 14px; color: var(--ink-soft); font-size: 12px; line-height: 1.5; }
  .commit-form > input { width: 100%; padding: 11px 12px; border: 1px solid #d9cdbd; border-radius: 8px; outline: 0; background: var(--canvas); color: var(--ink); font-size: 13px; }
  .commit-form > input:focus { border-color: #b4773f; box-shadow: 0 0 0 3px rgba(180, 119, 63, .12); }
  .dialog .new-form-actions { margin-top: 20px; }
  .git-menu strong, .git-menu small { display: block; }
  .git-menu strong { color: #eef0e9; font-size: 11px; }
  .git-menu small { padding: 4px 0; color: #aab9ad; font-size: 10px; }
  .git-menu button { width: 100%; margin-top: 7px; padding: 7px; border: 0; border-radius: 6px; background: #d5ab6c; color: #2c4032; font-size: 10px; cursor: pointer; }
  .git-menu button:disabled { opacity: .55; cursor: wait; }
  .search-modal { position: absolute; top: 61px; right: 40px; z-index: 5; width: min(460px, calc(100vw - 80px)); max-height: min(560px, calc(100vh - 100px)); overflow: auto; border: 1px solid var(--line); border-radius: 12px; background: var(--surface); box-shadow: var(--shadow-lg); }
  .search-modal-heading { display: flex; align-items: center; justify-content: space-between; padding: 13px 15px 10px; border-bottom: 1px solid var(--line); color: var(--ink-soft); font-size: 11px; }
  .search-modal-heading .quiet-button { padding: 0 4px; font-size: 18px; }
  .search-state { margin: 0; padding: 28px 16px; color: var(--ink-faint); font-size: 11px; text-align: center; }
  .search-results { padding: 7px; }
  .search-result { width: 100%; display: flex; align-items: center; gap: 9px; padding: 9px; border: 1px solid transparent; border-radius: 8px; background: transparent; color: var(--ink); text-align: left; cursor: pointer; }
  .search-result:hover { border-color: #e5d8c6; background: var(--surface-muted); }
  .search-result strong, .search-result small { display: block; }
  .search-result strong { font-size: 12px; }
  .search-result small { margin-top: 3px; color: var(--ink-faint); font-size: 10px; }
</style>
