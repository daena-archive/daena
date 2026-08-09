<script lang="ts">
  import { onMount } from "svelte";
  import { listen } from "@tauri-apps/api/event";
  const logoUrl = "/branding/logo.png";
  import { project, type Asset, type Entity, type Relationship, type MapLocation, type ProjectModuleManifest, type ProjectInfo, type GitStatus, type GitPreflight, type GitLogEntry, type PluginAdminEntry, type PluginUpgradePlan, type AiSettings, type AiProviderStatus, type AiStreamEvent, type AiIndexStatus } from "$lib/project/client";
  import type { EntityTemplate, FieldDefinition, ModuleContext, ModuleId, UUID, ModuleManifest, DaenaModule } from "../../packages/module-api/src/index";
  import { buildModuleContext } from "$lib/modules/context";
  import HostView from "$lib/plugins/HostView.svelte";
  import SandboxView from "$lib/plugins/SandboxView.svelte";
  import ProjectionView from "$lib/ProjectionView.svelte";
  import SettingsView from "$lib/SettingsView.svelte";
  import GitSettingsPanel from "$lib/GitSettingsPanel.svelte";
  import RelationshipPicker from "$lib/RelationshipPicker.svelte";
  import loreManifestJson from "../../packages/modules/lore/manifest.json";
  import timelineManifestJson from "../../packages/modules/timeline/manifest.json";
  import writingManifestJson from "../../packages/modules/writing/manifest.json";
  import { projectionModule } from "$lib/modules/projections";
  import RichTextEditor from "$lib/editor/RichTextEditor.svelte";
  import AiProposalPreview from "$lib/ai/AiProposalPreview.svelte";
  import { htmlToMarkdown } from "$lib/editor/markdown";
  import { formatCalendarDate, isCompleteCalendarDate, parseCalendarDate, serializeCalendarDate, type CalendarDate } from "$lib/date";

  type InstalledModule = ProjectModuleManifest;
  type WorkspaceSection = "lore" | "timeline" | "writing" | "maps";
  type SettingsSection = "general" | "ai" | "plugins" | "git";
  type WritingView = "manuscripts" | "reference";
  type AiFieldSuggestion = { value: string | string[]; rationale: string; confidence: string };
  type RecentProject = { name: string; root: string };
  type CreateOption = { key: string; module: InstalledModule; template: EntityTemplate };
  type CreateGroup = { module: InstalledModule; options: CreateOption[] };
  type CreateField = { namespace: string; field: FieldDefinition; required: boolean };
  type PluginNavigationItem = {
    plugin: PluginAdminEntry;
    view: PluginAdminEntry["views"][number];
    key: string;
    mode: "host" | "webview";
  };

  const recentProjectsKey = "daena.recent-projects";
  let settingsMigrated = false;

  let ready = $state(false);
  let error = $state("");
  let projectTransitionBusy = $state(false);
  let projectTransitionMessage = $state("");
  let section = $state<WorkspaceSection>("lore");
  let writingView = $state<WritingView>("manuscripts");
  let entities = $state<Entity[]>([]);
  let selected = $state<Entity | null>(null);
  let documentBody = $state("");
  let fields = $state<Record<string, unknown>>({});
  let relationships = $state<Relationship[]>([]);
  let assets = $state<Asset[]>([]);
  let mapLocations = $state<MapLocation[]>([]);
  let modules = $state<InstalledModule[]>([]);
  let query = $state("");
  let globalQuery = $state("");
  let name = $state("");
  let selectedCreateKey = $state("");
  let createFieldValues = $state<Record<string, unknown>>({});
  let createDateEditorOpen = $state<Record<string, boolean>>({});
  let createDocumentBody = $state("");
  let showDiscardPrompt = $state(false);
  let pendingCreateDiscard = $state<(() => void) | null>(null);
  let isSaving = $state(false);
  let savedAt = $state("");
  let editorFullscreen = $state(false);
  let hasUnsavedChanges = $state(false);
  let autoSaveTimer: number | null = null;
  let documentRevision = 0;
  let loadedDocumentRevision = "";
  let documentConflict = $state<{ paths: string[]; diagnostics: string[] } | null>(null);
  let conflictDiskBody = $state("");
  let mapSaveStates = $state<Record<string, { status: string; detail: unknown }>>({});
  let mapReloadCounter = $state(0);
  let mapRecoveryBusy = $state(false);
  let mapFocusLinkId = $state<string | null>(null);
  let mapSelection = $state<unknown | null>(null);
  let captureDialog = $state(false);
  let captureMode = $state<"existing" | "create">("existing");
  let captureEntityId = $state("");
  let captureTemplateKey = $state("");
  let captureName = $state("");
  let captureRole = $state("story-location");
  let mapReconcileNotice = $state("");
  let projectDiagnostics = $state<string[]>([]);
  let showSettings = $state(false);
  let settingsSection = $state<SettingsSection>("general");
  let aiSettings = $state<AiSettings>({
    localEndpoint: "http://127.0.0.1:1234/v1",
    localModel: "",
    localEmbeddingModel: "",
    remotePolicy: "localOnly",
    remote: { provider: "", endpoint: "", model: "", consents: [] },
  });
  let aiStatus = $state<AiProviderStatus | null>(null);
  let aiModels = $state<string[]>([]);
  let aiModelsBusy = $state(false);
  let aiModelsMessage = $state("");
  let aiModelsMessageTimer: number | null = null;
  let aiIndexStatus = $state<AiIndexStatus | null>(null);
  let aiIndexBusy = $state(false);
  let aiIndexMessage = $state("");
  let remoteCredential = $state<{ provider: string; configured: boolean } | null>(null);
  let aiUseRemote = $state(false);
  let aiUsage = $state<{ inputTokens: number; outputTokens: number; totalTokens: number } | null>(null);
  let aiRewriteOpen = $state(false);
  let aiBusy = $state(false);
  let aiMode = $state<"rewrite" | "generate">("rewrite");
  let aiRequestId = $state<string | null>(null);
  let aiInstruction = $state("Rewrite this to be more vivid while preserving the meaning.");
  let aiStreamText = $state("");
  let aiPreviewOutput = $state("");
  let aiSourceSelection = $state("");
  let aiSourceSelectionPlain = $state("");
  let aiGenerationContext = $state("");
  let aiSourceBody = $state("");
  let aiSourceRevision = $state("");
  let aiLastSequence = $state(-1);
  let aiUnlisten: (() => void) | null = null;
  let editorRef = $state<{ insertAiTextAtRequest: (value: string) => boolean } | null>(null);
  let aiFieldFillBusy = $state(false);
  let aiFieldFillOpen = $state(false);
  let aiFieldFillRequestId = $state<string | null>(null);
  let aiFieldFillStream = $state("");
  let aiFieldSuggestions = $state<Record<string, AiFieldSuggestion>>({});
  let aiFieldUnlisten: (() => void) | null = null;
  let adminPlugins = $state<PluginAdminEntry[] | null>(null);
  let hostView = $state<{ plugin: PluginAdminEntry; view: PluginAdminEntry["views"][number] } | null>(null);
  let sandboxView = $state<{ plugin: PluginAdminEntry; view: PluginAdminEntry["views"][number] | null } | null>(null);
  let projectionView = $state<{ title: string; module: DaenaModule } | null>(null);
  let adminBusy = $state(false);
  let pluginActionId = $state<string | null>(null);
  let installing = $state(false);
  let installConsent = $state<{ path: string; message: string } | null>(null);
  let installSummary = $state<{ id: string; version: string; signed: boolean; digest: string } | null>(null);
  let upgradePreview = $state<{ entry: PluginAdminEntry; version: string; plan: PluginUpgradePlan } | null>(null);
  let upgradeBusy = $state(false);
  let confirmAction = $state<{
    title: string;
    message: string;
    confirmLabel: string;
    run: () => Promise<void>;
    capabilities?: string[];
  } | null>(null);
  let confirmBusy = $state(false);
  let deleteTarget = $state<PluginAdminEntry | null>(null);
  let deleteInput = $state("");
  let deleteBusy = $state(false);
  let deleteBackupPath = $state("");
  let projectInfo = $state<ProjectInfo | null>(null);
  let gitStatus = $state<GitStatus | null>(null);
  let gitPreflight = $state<GitPreflight | null>(null);
  let gitLog = $state<GitLogEntry[]>([]);
  let showGit = $state(false);
  let gitBusy = $state(false);
  let gitMessage = $state("");
  let showProjectMenu = $state(false);
  let recentProjects = $state<RecentProject[]>([]);
  let searchMatches = $state<Entity[] | null>(null);
  let searchRequest = 0;
  let showCreateForm = $state(false);
  let dateEditorOpen = $state<Record<string, boolean>>({});

  const toastDurationMs = 3500;
  $effect(() => {
    if (!error) return;
    const timeout = window.setTimeout(() => { error = ""; }, toastDurationMs);
    return () => window.clearTimeout(timeout);
  });
  $effect(() => {
    const modalOpen = showCreateForm || editorFullscreen || upgradePreview !== null || confirmAction !== null || deleteTarget !== null || installConsent !== null || deleteBackupPath !== "";
    document.body.classList.toggle("modal-open", modalOpen);
    return () => document.body.classList.remove("modal-open");
  });

  const activeModuleId = () => section === "lore" ? "daena.lore" : section === "timeline" ? "daena.timeline" : section === "writing" ? "daena.writing" : "daena.maps";
  const activeManifest = () => (section === "lore" ? loreManifestJson : section === "timeline" ? timelineManifestJson : section === "writing" ? writingManifestJson : null) as unknown as ModuleManifest | null;
  const workspaceSectionOrder: WorkspaceSection[] = ["lore", "timeline", "writing", "maps"];
  function workspaceModuleId(target: WorkspaceSection) {
    return target === "lore" ? "daena.lore" : target === "timeline" ? "daena.timeline" : target === "writing" ? "daena.writing" : "daena.maps";
  }
  function enabledWorkspaceSections() {
    return workspaceSectionOrder.filter((target) => modules.some((module) => module.id === workspaceModuleId(target) && module.enabled));
  }
  function sectionIcon(target: WorkspaceSection) {
    return target === "lore" ? "✦" : target === "timeline" ? "◷" : target === "writing" ? "✎" : "◇";
  }
  function fieldAppliesToEntity(field: FieldDefinition, entityType?: string | null) {
    return !field.entityTypes || !entityType || field.entityTypes.includes(entityType);
  }
  const definitions = () => {
    const entityType = selected?.entity_type ?? (section === "timeline" ? "event" : section === "writing" ? writingView === "manuscripts" ? "manuscript" : "reference-page" : undefined);
    return activeManifest()?.schemas
      .filter((schema) => !entityType || schema.entityTypes.includes(entityType))
      .flatMap((schema) => schema.fields.filter((field) => fieldAppliesToEntity(field, entityType))) ?? [];
  };
  function isEmptyFieldValue(value: unknown) {
    return value === undefined || value === null || value === "" || (typeof value === "string" && !value.trim()) || (Array.isArray(value) && value.length === 0);
  }
  function fieldDisplayValue(value: unknown) {
    return Array.isArray(value) ? value.join(", ") : String(value ?? "");
  }
  function suggestionDisplayValue(key: string, suggestion: AiFieldSuggestion) {
    const definition = definitions().find((candidate) => candidate.key === key);
    if (definition?.type !== "relationship" || !Array.isArray(suggestion.value)) return fieldDisplayValue(suggestion.value);
    const names = new Map(entities.map((entity) => [entity.id, entity.name]));
    return suggestion.value.map((id) => names.get(id) ?? id).join(", ");
  }
  function createOptions(): CreateOption[] {
    return modules
      .filter((module) => module.enabled)
      .flatMap((module) => module.templates.map((template) => ({ key: `${module.id}:${template.id}`, module, template })));
  }
  function createGroups(): CreateGroup[] {
    const groups = new Map<string, CreateGroup>();
    for (const option of createOptions()) {
      const group = groups.get(option.module.id) ?? { module: option.module, options: [] };
      group.options.push(option);
      groups.set(option.module.id, group);
    }
    return [...groups.values()];
  }
  function selectedCreateOption() { return createOptions().find((option) => option.key === selectedCreateKey) ?? null; }
  function createFieldsFor(option: CreateOption | null = selectedCreateOption()): CreateField[] {
    if (!option) return [];
    return option.module.schemas
      .filter((schema) => schema.entityTypes.includes(option.template.entityType))
      .flatMap((schema) => schema.fields
        .filter((field) => fieldAppliesToEntity(field, option.template.entityType))
        .map((field) => ({
          namespace: schema.namespace,
          field,
          required: Boolean(field.required || option.template.requiredFields?.includes(field.key)),
        })));
  }
  function createRelationshipValues(key: string) {
    const value = createFieldValues[key];
    return Array.isArray(value) ? value.filter((targetId): targetId is string => typeof targetId === "string") : [];
  }
  function setCreateRelationshipValues(key: string, values: string[]) {
    createFieldValues = { ...createFieldValues, [key]: values };
  }
  function defaultCreateFieldValue(field: FieldDefinition, template: EntityTemplate) {
    if (Object.prototype.hasOwnProperty.call(template.fields, field.key)) return template.fields[field.key];
    return field.type === "boolean" ? false : field.type === "relationship" || (field.type === "enum" && field.multiple) ? [] : "";
  }
  function resetCreateFields(option: CreateOption | null) {
    createFieldValues = Object.fromEntries(createFieldsFor(option).map(({ field }) => [field.key, defaultCreateFieldValue(field, option!.template)]));
    createDateEditorOpen = {};
    createDocumentBody = option?.template.document ?? "";
  }
  function selectCreateOption(key: string) {
    if (key === selectedCreateKey && Object.keys(createFieldValues).length > 0) return;
    requestCreateDiscard(() => {
      name = "";
      selectedCreateKey = key;
      resetCreateFields(createOptions().find((option) => option.key === key) ?? null);
    });
  }
  function setCreateField(key: string, value: unknown) {
    createFieldValues = { ...createFieldValues, [key]: value };
  }
  function updateCreateEnumField(key: string, event: Event, multiple: boolean) {
    const target = event.currentTarget as HTMLSelectElement;
    setCreateField(key, multiple ? Array.from(target.selectedOptions, (option) => option.value) : target.value);
  }
  function isCreateValuePopulated(value: unknown) {
    if (Array.isArray(value)) return value.length > 0;
    return value !== "" && value !== null && value !== undefined && value !== false;
  }
  function hasCreateValues() {
    return Boolean(name.trim() || createDocumentBody.trim() || Object.values(createFieldValues).some(isCreateValuePopulated));
  }
  function requestCreateDiscard(action: () => void) {
    if (!hasCreateValues()) {
      action();
      return;
    }
    pendingCreateDiscard = action;
    showDiscardPrompt = true;
  }
  function keepCreateEditing() {
    showDiscardPrompt = false;
    pendingCreateDiscard = null;
  }
  function discardCreateValues() {
    const action = pendingCreateDiscard;
    showDiscardPrompt = false;
    pendingCreateDiscard = null;
    action?.();
  }
  function createDateForField(key: string) { return parseCalendarDate(createFieldValues[key]); }
  function createDateDraftForField(key: string): Partial<CalendarDate> | null {
    return createDateForField(key) ?? (createDateEditorOpen[key] ? { calendar: "gregorian", era: "CE", precision: "day" } : null);
  }
  function openCreateDateEditor(key: string) {
    createDateEditorOpen = { ...createDateEditorOpen, [key]: true };
    setCreateField(key, "");
  }
  function updateCreateDateField(key: string, patch: Partial<CalendarDate>) {
    const current = createDateForField(key) ?? { calendar: "gregorian", era: "CE", year: 1, month: 1, day: 1, precision: "day" };
    const next = { ...current, ...patch } as CalendarDate;
    if (patch.precision === "year") { delete next.month; delete next.day; }
    if (patch.precision === "month" && next.month === undefined) next.month = 1;
    if (patch.precision === "day") { next.month ??= 1; next.day ??= 1; }
    setCreateField(key, serializeCalendarDate(next));
  }
  function updateCreateDatePart(key: string, part: "year" | "month" | "day", raw: string, min: number, max?: number) {
    if (!raw.trim()) return;
    const parsed = Math.floor(Number(raw));
    if (!Number.isFinite(parsed)) return;
    updateCreateDateField(key, { [part]: Math.min(max ?? parsed, Math.max(min, parsed)) });
  }
  function clearCreateDateField(key: string) {
    setCreateField(key, "");
    createDateEditorOpen = { ...createDateEditorOpen, [key]: false };
  }

  function contextFor(currentSection = section): ModuleContext {
    if (!projectInfo?.root) throw new Error("No project is open");
    const manifest = currentSection === "lore" ? loreManifestJson : currentSection === "timeline" ? timelineManifestJson : writingManifestJson;
    return buildModuleContext(manifest as unknown as ModuleManifest, projectInfo.root);
  }

  function sectionEnabled() { return modules.find((module) => module.id === activeModuleId())?.enabled ?? false; }

  async function closeNativePluginWebviews() {
    try {
      await project.closeAllPluginWebviews();
    } catch {
      // The webview cleanup is best effort so browser previews remain usable.
    }
  }

  async function leavePluginView() {
    hostView = null;
    sandboxView = null;
    projectionView = null;
    await closeNativePluginWebviews();
  }

  function pluginViewLabel(item: PluginNavigationItem) {
    return item.plugin.name === item.view.title ? item.plugin.name : `${item.plugin.name} · ${item.view.title}`;
  }

  function workspaceNavigationActive(target: WorkspaceSection) {
    if (target === "maps") {
      return section === "maps" && !hostView && !projectionView && (!sandboxView || sandboxView.plugin.id === "daena.maps");
    }
    return !hostView && !sandboxView && !projectionView && section === target;
  }

  function pluginNavigationActive(item: PluginNavigationItem) {
    if (item.plugin.id === "daena.maps") return sandboxView?.plugin.id === "daena.maps";
    if (item.mode === "host") {
      return hostView?.plugin.id === item.plugin.id && hostView.view.id === item.view.id;
    }
    return sandboxView?.plugin.id === item.plugin.id && sandboxView.view?.id === item.view.id;
  }

  async function openHostView(plugin: PluginAdminEntry, view: PluginAdminEntry["views"][number]) {
    await closeNativePluginWebviews();
    hostView = { plugin, view };
    sandboxView = null;
    showSettings = false;
  }

  function pluginViews() {
    return (adminPlugins ?? [])
      .filter((plugin) => plugin.enabled && plugin.lifecycle.state === "active")
      .flatMap((plugin) => plugin.views
        .filter((view) => plugin.kind === "sandboxed" || (view.components?.length ?? 0) > 0)
        .map((view) => ({
          plugin,
          view,
          key: `${plugin.id}:${view.id}`,
          mode: plugin.kind === "sandboxed" ? "webview" : "host",
        } satisfies PluginNavigationItem)))
      .sort((left, right) => left.view.title.localeCompare(right.view.title));
  }

  async function openPluginView(item: PluginNavigationItem) {
    if (item.plugin.id === "daena.maps") {
      showSettings = false;
      const mapId = currentMapId();
      if (sandboxView?.plugin.id === "daena.maps") {
        const mapsWelcome = sandboxView.view === null;
        if ((mapId === null && mapsWelcome) || (mapId !== null && !mapsWelcome)) return;
      }
      mapFocusLinkId = null;
      await leavePluginView();
      sandboxView = mapId
        ? { plugin: item.plugin, view: item.view }
        : { plugin: item.plugin, view: null };
      return;
    }
    if (item.mode === "host") {
      await openHostView(item.plugin, item.view);
      return;
    }
    await closeNativePluginWebviews();
    hostView = null;
    sandboxView = { plugin: item.plugin, view: item.view };
    showSettings = false;
  }

  async function createMap() {
    if (projectDiagnostics.length > 0) return;
    try {
      const created = await project.createMap();
      await loadEntities();
      selected = created;
      fields = {};
      relationships = [];
      assets = [];
      mapLocations = [];
      const mapView = pluginViews().find((item) => item.plugin.id === "daena.maps");
      if (!mapView) throw new Error("The Maps plugin view is not available");
      await openPluginView(mapView);
    } catch (cause) {
      error = friendlyError(cause);
    }
  }

  function currentMapId() {
    return selected?.entity_type === "daena.maps:map" ? selected.id : null;
  }

  type SavedMapEntry = Entity & { size: number };
  let savedMapsCache = $state<SavedMapEntry[] | null>(null);
  let savedMapsRequest = 0;
  $effect(() => {
    const mapsWelcomeOpen = sandboxView?.plugin.id === "daena.maps" && sandboxView.view == null;
    void entities;
    if (!ready || !mapsWelcomeOpen) {
      savedMapsCache = null;
      return;
    }
    const request = ++savedMapsRequest;
    void (async () => {
      try {
        const all = await project.listEntities();
        const maps = all.filter((entity) => entity.entity_type === "daena.maps:map");
        const entries: SavedMapEntry[] = [];
        for (const map of maps) {
          const fields = await project.listFields(map.id);
          const field = fields.find((item) => item.namespace === "maps" && item.key === "map") ?? null;
          let descriptor = (field?.value ?? null) as {
            schemaVersion?: number;
            provider?: { id: string; adapterVersion: number; sourceFormat: string };
            sourceAssetId?: string | null;
            previewAssetId?: string | null;
            defaultView?: { center: [number, number]; zoom: number };
          } | null;
          const assets = await project.listAssets(map.id);
          const mapAssets = assets
            .filter((asset) => asset.namespace === "maps" && asset.size > 0)
            .sort((left, right) => right.created_at.localeCompare(left.created_at));
          let sourceId = descriptor?.sourceAssetId ?? null;
          // Repair orphan first-saves: bytes landed under assets/maps but the
          // descriptor never received sourceAssetId, so Saved Maps stayed empty.
          if (!sourceId && mapAssets.length > 0) {
            const orphan = mapAssets[0];
            const repaired = {
              schemaVersion: 1,
              provider: descriptor?.provider ?? { id: "azgaar-fmg", adapterVersion: 1, sourceFormat: "fmg-map" },
              sourceAssetId: orphan.id,
              previewAssetId: descriptor?.previewAssetId ?? null,
              defaultView: descriptor?.defaultView ?? { center: [0.5, 0.5] as [number, number], zoom: 1 },
            };
            await project.setField({
              entity_id: map.id,
              namespace: "maps",
              key: "map",
              value: repaired,
              revision: "",
            });
            descriptor = repaired;
            sourceId = orphan.id;
          }
          if (!sourceId) continue;
          const source = mapAssets.find((asset) => asset.id === sourceId);
          if (!source) continue;
          entries.push({ ...map, size: source.size });
        }
        entries.sort((left, right) => right.updated_at.localeCompare(left.updated_at));
        if (request === savedMapsRequest) savedMapsCache = entries;
      } catch {
        // Keep the previous cache on transient failures; retry on next change.
      }
    })();
  });

  function savedMaps() {
    return savedMapsCache ?? [];
  }

  async function saveCurrentMap() {
    try {
      await project.mapsEditorSave();
    } catch (cause) {
      error = friendlyError(cause);
    }
  }

  function reloadMapOriginal() {
    if (!currentMapId()) return;
    mapReloadCounter += 1;
  }

  async function restoreMapDraft() {
    const mapId = currentMapId();
    if (!mapId || mapRecoveryBusy) return;
    mapRecoveryBusy = true;
    try {
      const copies = await project.mapsRecoveryList(mapId);
      if (copies.length === 0) throw new Error("No recovery copy was found for this map.");
      await project.mapsRecoveryRestore(mapId, copies[0].fileName);
      mapSaveStates[mapId] = { status: "restoring", detail: null };
      mapReloadCounter += 1;
    } catch (cause) {
      error = friendlyError(cause);
    } finally {
      mapRecoveryBusy = false;
    }
  }

  function dismissMapConflict() {
    const mapId = currentMapId();
    if (!mapId) return;
    mapSaveStates[mapId] = { status: "clean", detail: null };
  }

  function mapSaveLabel(state: { status: string; detail: unknown } | null) {
    switch (state?.status) {
      case "loading": return "Loading map…";
      case "generating": return "Generating map…";
      case "dirty": return "Unsaved changes";
      case "saving": return "Saving…";
      case "saved": return "Saved";
      case "conflict": return "Save blocked by changes on disk";
      case "reconcile": return "Links re-checked";
      case "error": return "Save failed";
      case "restoring": return "Restoring draft…";
      default: return "Up to date";
    }
  }

  function mapConflictDetail(detail: unknown): { path?: string } {
    return typeof detail === "object" && detail !== null ? (detail as { path?: string }) : {};
  }

  function syncStatusLabel(info: ProjectInfo | null) {
    const sync = info?.sync;
    if (!sync) return "Storage status unavailable";
    if (sync.state === "failed") return `Export failed · ${sync.export_error ?? "checkpoint unavailable"}`;
    if (sync.state === "exporting") return `Saving to project files · ${sync.dirty_count} pending`;
    return "Project files up to date";
  }

  function visibleEntities() {
    const term = query.trim().toLowerCase();
    if (section === "maps") return [];
    return entities.filter((entity) => {
      const belongs = section === "timeline"
        ? entity.entity_type === "event"
        : section === "writing"
          ? entity.entity_type === (writingView === "manuscripts" ? "manuscript" : "reference-page")
          : entity.entity_type !== "event" && entity.entity_type !== "manuscript" && entity.entity_type !== "reference-page";
      return belongs && (!term || `${entity.name} ${entity.entity_type ?? ""}`.toLowerCase().includes(term));
    });
  }

  function entityGlyph(entity: Pick<Entity, "entity_type">) {
    return entity.entity_type === "person" ? "P" : entity.entity_type === "place" ? "L" : entity.entity_type === "faction" ? "F" : entity.entity_type === "artifact" ? "A" : entity.entity_type === "culture" ? "C" : entity.entity_type === "event" ? "E" : entity.entity_type === "manuscript" ? "M" : entity.entity_type === "reference-page" ? "R" : "?";
  }

  function entityGlyphClass(entity: Pick<Entity, "entity_type">) {
    return `entity-glyph-${entity.entity_type ?? "unknown"}`;
  }

  async function selectSearchResult(entity: Entity) {
    if (!(await flushAutoSave())) return;
    section = entity.entity_type === "event" ? "timeline" : entity.entity_type === "manuscript" || entity.entity_type === "reference-page" ? "writing" : "lore";
    if (entity.entity_type === "reference-page") writingView = "reference";
    if (entity.entity_type === "manuscript") writingView = "manuscripts";
    hostView = null;
    sandboxView = null;
    globalQuery = "";
    query = "";
    await selectEntity(entity);
  }

  async function switchSection(next: WorkspaceSection) {
    if (!(await flushAutoSave())) return;
    if (section === next && (next !== "maps" || sandboxView?.plugin.id === "daena.maps") && !showSettings) return;
    showSettings = false;
    await leavePluginView();
    section = next;
    clearSelection();
    query = "";
    if (next === "maps") {
      const mapView = pluginViews().find((item) => item.plugin.id === "daena.maps");
      if (mapView) await openPluginView(mapView);
    }
  }

  async function reconcileWorkspaceSection() {
    if (enabledWorkspaceSections().includes(section)) return;
    await leavePluginView();
    section = enabledWorkspaceSections()[0] ?? "lore";
    clearSelection();
    query = "";
    editorFullscreen = false;
  }

  async function switchWritingView(next: WritingView) {
    if (!(await flushAutoSave())) return;
    await leavePluginView();
    if (writingView === next) return;
    writingView = next;
    clearSelection();
    query = "";
  }

  function sectionLabel() {
    if (showSettings) {
      if (settingsSection === "plugins") return "Settings · Plugins";
      if (settingsSection === "git") return "Settings · Git";
      return "Settings";
    }
    return section === "lore" ? "Lore library" : section === "timeline" ? "Timeline" : section === "writing" ? "Writing Studio" : "Maps";
  }

  function collectionLabel() {
    return section === "lore" ? "entries" : section === "timeline" ? "events" : section === "writing" ? writingView === "manuscripts" ? "manuscripts" : "reference pages" : "maps";
  }

  function createLabel() {
    return section === "lore" ? "entry" : section === "timeline" ? "event" : section === "writing" ? writingView === "manuscripts" ? "manuscript" : "reference page" : "map";
  }

  function entityTypeLabel(entityType: string | null) {
    return entityType === "reference-page" ? "Reference page" : entityType === "manuscript" ? "Manuscript" : entityType ?? "Uncategorized";
  }

  function openProjection() {
    const projection = projectionModule(section === "lore" ? "lore" : "timeline");
    hostView = null;
    sandboxView = null;
    projectionView = projection;
  }

  function normalizeDocument(body: string, format?: string) {
    if (format === "rich-text") return htmlToMarkdown(body);
    return body;
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
    markEntryDirty();
  }
  function updateDateField(key: string, patch: Partial<CalendarDate>) {
    if (projectDiagnostics.length > 0) return;
    const current = dateForField(key) ?? { calendar: "gregorian", era: "CE", year: 1, month: 1, day: 1, precision: "day" };
    const next = { ...current, ...patch } as CalendarDate;
    if (patch.precision === "year") { delete next.month; delete next.day; }
    if (patch.precision === "month" && next.month === undefined) next.month = 1;
    if (patch.precision === "day") { next.month ??= 1; next.day ??= 1; }
    fields = { ...fields, [key]: serializeCalendarDate(next) };
    markEntryDirty();
  }
  function updateDatePart(key: string, part: "year" | "month" | "day", raw: string, min: number, max?: number) {
    if (!raw.trim()) return;
    const parsed = Math.floor(Number(raw));
    if (!Number.isFinite(parsed)) return;
    const value = Math.min(max ?? parsed, Math.max(min, parsed));
    updateDateField(key, { [part]: value });
  }
  function clearDateField(key: string) {
    fields = { ...fields, [key]: "" };
    dateEditorOpen = { ...dateEditorOpen, [key]: false };
    markEntryDirty();
  }

  function wordCount() { return documentBody.replace(/[`*_>#\[\]()]/g, " ").trim().split(/\s+/).filter(Boolean).length; }
  function cancelAutoSave() {
    if (autoSaveTimer !== null) {
      window.clearTimeout(autoSaveTimer);
      autoSaveTimer = null;
    }
  }
  function scheduleAutoSave() {
    cancelAutoSave();
    if (!selected || !sectionEnabled()) return;
    autoSaveTimer = window.setTimeout(() => {
      autoSaveTimer = null;
      void saveDocument();
    }, 900);
  }
  function markEntryDirty() {
    documentRevision += 1;
    hasUnsavedChanges = true;
    savedAt = "";
    scheduleAutoSave();
  }
  function updateDocumentBody(value: string) {
    if (projectDiagnostics.length > 0) return;
    documentBody = value;
    markEntryDirty();
  }
  function setEditorFullscreen(value: boolean) { editorFullscreen = value; }
  function friendlyError(cause: unknown) {
    const message = cause instanceof Error ? cause.message : String(cause);
    return message.includes("invoke") || message.includes("undefined") ? "The desktop bridge is unavailable. Open this workspace in the Tauri app to use local project storage." : message;
  }
  function updateAiSetting(key: "localEndpoint" | "localModel" | "localEmbeddingModel", value: string) {
    aiSettings = { ...aiSettings, [key]: value };
    aiStatus = null;
    void project.settingsUpdate({ ai: { [key]: value } });
  }
  function updateAiRemoteSetting(key: "provider" | "endpoint" | "model", value: string) {
    aiSettings = { ...aiSettings, remote: { ...aiSettings.remote, [key]: value } };
    remoteCredential = null;
    void project.settingsUpdate({ ai: { remote: { [key]: value } } });
  }
  function updateAiRemotePolicy(value: AiSettings["remotePolicy"]) {
    aiSettings = { ...aiSettings, remotePolicy: value };
    void project.settingsUpdate({ ai: { remotePolicy: value } });
  }
  async function refreshRemoteCredential() {
    if (!aiSettings.remote.provider.trim()) { remoteCredential = null; return; }
    try { remoteCredential = await project.aiRemoteCredentialStatus(aiSettings.remote.provider); }
    catch (_) { remoteCredential = { provider: aiSettings.remote.provider, configured: false }; }
  }
  async function importRemoteCredential() {
    if (!aiSettings.remote.provider.trim()) return;
    try {
      remoteCredential = await project.aiRemoteImportCredential(aiSettings.remote.provider);
    } catch (cause) { aiIndexMessage = friendlyError(cause); }
  }
  async function setRemoteConsent(allowed: boolean) {
    if (!projectInfo?.root || !aiSettings.remote.provider || !aiSettings.remote.endpoint) return;
    try {
      await project.aiRemoteSetConsent(projectInfo.root, aiSettings.remote.provider, aiSettings.remote.endpoint, allowed);
      const consents = aiSettings.remote.consents.filter((consent) => !(consent.projectId === projectInfo?.root && consent.provider === aiSettings.remote.provider));
      aiSettings = { ...aiSettings, remote: { ...aiSettings.remote, consents: allowed ? [...consents, { projectId: projectInfo.root, provider: aiSettings.remote.provider, endpoint: aiSettings.remote.endpoint }] : consents } };
    } catch (cause) { aiIndexMessage = friendlyError(cause); }
  }
  async function checkAiProvider() {
    try { aiStatus = await project.aiLocalStatus(aiSettings.localEndpoint, aiSettings.localModel); }
    catch (cause) { aiStatus = { endpoint: aiSettings.localEndpoint, model: aiSettings.localModel, available: false, modelAvailable: false, error: friendlyError(cause) }; }
  }
  async function loadAiModels() {
    const endpoint = aiSettings.localEndpoint.trim();
    if (!endpoint) {
      aiModelsMessage = "Enter a local endpoint before loading models.";
      return;
    }
    aiModelsBusy = true;
    aiModelsMessage = "";
    try {
      aiModels = await project.aiLocalModels(endpoint);
      if (aiModels.length === 0) {
        aiModelsMessage = "No models were returned by the local provider.";
      } else {
        aiModelsMessage = `${aiModels.length} model${aiModels.length === 1 ? "" : "s"} available.`;
        if (aiModelsMessageTimer !== null) window.clearTimeout(aiModelsMessageTimer);
        aiModelsMessageTimer = window.setTimeout(() => {
          aiModelsMessage = "";
          aiModelsMessageTimer = null;
        }, toastDurationMs);
        if (!aiSettings.localModel.trim() && aiModels.length === 1) updateAiSetting("localModel", aiModels[0]);
      }
    } catch (cause) {
      aiModels = [];
      aiModelsMessage = friendlyError(cause);
    } finally { aiModelsBusy = false; }
  }
  async function refreshAiIndexStatus() {
    try { aiIndexStatus = await project.aiIndexStatus(); }
    catch (cause) { aiIndexStatus = { available: false, state: null }; aiIndexMessage = friendlyError(cause); }
  }
  async function rebuildAiIndex() {
    const endpoint = aiSettings.localEndpoint.trim();
    const chatModel = aiSettings.localModel.trim();
    if (!endpoint || !chatModel) {
      aiIndexMessage = "Configure a local endpoint and chat model in AI providers before building the semantic index.";
      return;
    }
    aiIndexBusy = true;
    aiIndexMessage = "";
    try {
      const embeddingModel = aiSettings.localEmbeddingModel.trim() || chatModel;
      const result = await project.aiIndexRebuild(endpoint, embeddingModel);
      aiIndexStatus = { available: true, state: result.state };
      aiIndexMessage = `Indexed ${result.chunkCount} chunks (${result.embeddedCount} embedded, ${result.reusedCount} reused).`;
    } catch (cause) { aiIndexMessage = friendlyError(cause); }
    finally { aiIndexBusy = false; }
  }
  async function cancelAiIndex() {
    try { await project.aiIndexCancel(); aiIndexMessage = "Cancellation requested."; }
    catch (cause) { aiIndexMessage = friendlyError(cause); }
  }
  function setAiSelection(markdown: string, plainText: string) {
    if (!aiBusy && !aiPreviewOutput) {
      aiSourceSelection = markdown;
      aiSourceSelectionPlain = plainText;
    }
  }
  function openAiAction(action: "rewrite" | "generate" | "concise" | "expand" | "grammar" | "tone" | "custom", markdown: string, plainText: string, context: string) {
    if (action !== "generate" && !markdown.trim()) return;
    if (action !== "generate") {
      aiSourceSelection = markdown;
      aiSourceSelectionPlain = plainText;
      aiGenerationContext = "";
    } else {
      aiSourceSelection = "";
      aiSourceSelectionPlain = "";
      aiGenerationContext = context;
    }
    aiMode = action === "generate" ? "generate" : "rewrite";
    const instructions = {
      rewrite: "Rewrite this to be more vivid while preserving the meaning.",
      concise: "Make this more concise while preserving the meaning.",
      expand: "Expand this with useful detail while preserving the meaning.",
      grammar: "Fix grammar, spelling, and awkward phrasing while preserving the meaning.",
      tone: "Change the tone of this passage while preserving its meaning. Ask for the desired tone if needed.",
      custom: "",
      generate: "Write text that fits naturally at the cursor position.",
    } as const;
    aiInstruction = instructions[action];
    aiRewriteOpen = true;
  }
  function emptyInspectorDefinitions() {
    return definitions().filter((definition) => definition.type === "relationship"
      ? selectedRelationshipIds(definition).length === 0
      : isEmptyFieldValue(fields[definition.key]));
  }
  function clearAiFieldListener() {
    aiFieldUnlisten?.();
    aiFieldUnlisten = null;
  }
  function closeAiFieldFill() {
    if (aiFieldFillRequestId) void project.aiCancelText(aiFieldFillRequestId).catch(() => {});
    clearAiFieldListener();
    aiFieldFillOpen = false;
    aiFieldFillBusy = false;
    aiFieldFillRequestId = null;
    aiFieldFillStream = "";
    aiFieldSuggestions = {};
  }
  function handleAiFieldFillEvent(payload: AiStreamEvent) {
    if (payload.phase === "delta" && payload.delta) aiFieldFillStream += payload.delta;
    if (payload.phase === "failed") {
      clearAiFieldListener();
      aiFieldFillBusy = false;
      aiFieldFillRequestId = null;
      error = payload.error ?? "AI field suggestions failed";
    } else if (payload.phase === "cancelled" || payload.phase === "deadline_exceeded") {
      clearAiFieldListener();
      aiFieldFillBusy = false;
      aiFieldFillRequestId = null;
    } else if (payload.phase === "completed") {
      clearAiFieldListener();
      aiFieldFillBusy = false;
      aiFieldFillRequestId = null;
      try {
        const parsed = JSON.parse(payload.output ?? aiFieldFillStream) as { suggestions?: Record<string, { value?: unknown; values?: unknown; rationale?: unknown; confidence?: unknown }> };
        const allowed = new Set(emptyInspectorDefinitions().map((definition) => definition.key));
        const suggestions: Record<string, AiFieldSuggestion> = {};
        for (const [key, value] of Object.entries(parsed.suggestions ?? {})) {
          const definition = definitions().find((candidate) => candidate.key === key);
          const usesValues = definition?.multiple || definition?.type === "relationship";
          const rawValue = usesValues ? value?.values : value?.value;
          if (!allowed.has(key) || rawValue === undefined || rawValue === null) continue;
          if (usesValues && (!Array.isArray(rawValue) || rawValue.length === 0 || rawValue.length > 5 || rawValue.some((item) => typeof item !== "string"))) continue;
          if (definition?.type === "relationship") {
            const allowedIds = new Set(entities.filter((entity) => !entity.deleted && (!definition.targetEntityTypes?.length || definition.targetEntityTypes.includes(entity.entity_type ?? ""))).map((entity) => entity.id));
            if (!(rawValue as string[]).every((id) => allowedIds.has(id))) continue;
          }
          suggestions[key] = {
            value: usesValues ? (rawValue as string[]) : String(rawValue),
            rationale: String(value.rationale ?? "Suggested from related project context."),
            confidence: String(value.confidence ?? "unknown"),
          };
        }
        aiFieldSuggestions = suggestions;
        if (Object.keys(suggestions).length === 0) {
          aiFieldFillOpen = false;
          error = "AI found no supported suggestions for the empty fields.";
        }
      } catch {
        aiFieldFillOpen = false;
        error = "AI returned invalid field suggestions.";
      }
    }
  }
  async function fillAiFields() {
    if (!selected || aiFieldFillBusy) return;
    const empty = emptyInspectorDefinitions();
    if (empty.length === 0) return;
    const endpoint = aiSettings.localEndpoint.trim();
    const model = aiSettings.localModel.trim();
    if (!endpoint || !model) {
      error = "Configure a local endpoint and chat model in AI providers before filling fields.";
      return;
    }
    aiFieldFillOpen = true;
    aiFieldFillBusy = true;
    aiFieldFillStream = "";
    aiFieldSuggestions = {};
    const fieldKeys = empty.map((definition) => definition.key);
    const context = JSON.stringify({
      entity: { name: selected.name, type: selected.entity_type },
      document: documentBody,
      populatedFields: Object.fromEntries(Object.entries(fields).filter(([, value]) => !isEmptyFieldValue(value))),
      emptyFields: empty.map((definition) => ({
        key: definition.key,
        label: definition.label,
        type: definition.type,
        multiple: definition.multiple ?? definition.type === "relationship",
        options: definition.options ?? [],
        allowedEntities: definition.type === "relationship"
          ? entities.filter((entity) => !entity.deleted && (!definition.targetEntityTypes?.length || definition.targetEntityTypes.includes(entity.entity_type ?? ""))).map((entity) => ({ id: entity.id, name: entity.name, type: entity.entity_type }))
          : [],
      })),
    });
    const suggestionProperties = Object.fromEntries(empty.map((definition) => [definition.key, {
      type: "object",
      properties: {
        ...(definition.multiple || definition.type === "relationship"
          ? { values: { type: "array", items: { type: "string", maxLength: 400 }, maxItems: 5, uniqueItems: true } }
          : { value: { type: "string", maxLength: 4000 } }),
        rationale: { type: "string", maxLength: 1000 },
        confidence: { type: "string", maxLength: 32 },
      },
      required: [definition.multiple || definition.type === "relationship" ? "values" : "value"],
      additionalProperties: false,
    }]));
    const outputContract = {
      type: "object",
      properties: {
        suggestions: {
          type: "object",
          properties: suggestionProperties,
          additionalProperties: false,
        },
      },
      required: ["suggestions"],
      additionalProperties: false,
    };
    const retrievalQuery = `${selected.name} ${selected.entity_type ?? ""} ${empty.map((definition) => definition.label).join(" ")}`.slice(0, 4000);
    try {
      const requestId = await project.aiGenerateStructured(endpoint, model, `Fill only these empty fields: ${fieldKeys.join(", ")}. For multi-select and relationship fields, return up to five distinct values in the values array. For relationship fields, use only allowed entity IDs from the context. Use only configured options when options are provided. Return evidence-backed suggestions. Do not invent facts.`, context, outputContract, selected.id, retrievalQuery);
      aiFieldFillRequestId = requestId;
      aiFieldUnlisten = await listen<AiStreamEvent>(`ai-stream:${requestId}`, (event) => handleAiFieldFillEvent(event.payload));
      const buffered = await project.aiPollText(requestId);
      for (const event of buffered) handleAiFieldFillEvent(event);
    } catch (cause) {
      clearAiFieldListener();
      aiFieldFillBusy = false;
      aiFieldFillRequestId = null;
      error = friendlyError(cause);
    }
  }
  async function acceptAiFieldSuggestion(key: string) {
    const suggestion = aiFieldSuggestions[key];
    const definition = definitions().find((candidate) => candidate.key === key);
    if (!suggestion || (definition?.type === "relationship" ? selectedRelationshipIds(definition).length > 0 : !isEmptyFieldValue(fields[key]))) return;
    if (definition?.type === "relationship") await updateRelationshipField(definition, suggestion.value as string[]);
    else fields = { ...fields, [key]: suggestion.value };
    const remaining = { ...aiFieldSuggestions };
    delete remaining[key];
    aiFieldSuggestions = remaining;
    markEntryDirty();
  }
  function discardAiFieldSuggestion(key: string) {
    const remaining = { ...aiFieldSuggestions };
    delete remaining[key];
    aiFieldSuggestions = remaining;
  }
  function clearAiStreamListener() {
    aiUnlisten?.();
    aiUnlisten = null;
  }
  function closeAiRewrite() {
    if (aiRequestId) void project.aiCancelText(aiRequestId).catch(() => {});
    clearAiStreamListener();
    aiRewriteOpen = false;
    aiBusy = false;
    aiRequestId = null;
    aiStreamText = "";
    aiPreviewOutput = "";
    aiUsage = null;
    aiSourceSelection = "";
    aiSourceSelectionPlain = "";
    aiGenerationContext = "";
    aiLastSequence = -1;
    aiMode = "rewrite";
  }
  function validateAiProposal(value: string): string | null {
    if (!value.trim()) return "LM Studio returned an empty proposal.";
    if (/(^|\n)\s*(#{1,6}\s|>\s|[-*+]\s|\d+\.\s|```|~~~)/.test(value)) {
      return "The proposal contains block-level Markdown. Edit it to plain text before accepting.";
    }
    if (/<\/?[a-z][^>]*>/i.test(value)) return "The proposal contains HTML markup. Edit it to plain text before accepting.";
    return null;
  }
  function handleAiEvent(payload: AiStreamEvent) {
    if (payload.sequence <= aiLastSequence) return;
    aiLastSequence = payload.sequence;
    if (payload.phase === "delta" && payload.delta) aiStreamText += payload.delta;
    if (payload.phase === "usage" && payload.output) {
      try { aiUsage = JSON.parse(payload.output); } catch (_) { aiUsage = null; }
    }
    if (payload.phase === "completed") {
      aiPreviewOutput = payload.output ?? aiStreamText;
      aiBusy = false;
      aiRequestId = null;
      clearAiStreamListener();
    } else if (payload.phase === "cancelled" || payload.phase === "deadline_exceeded") {
      aiBusy = false;
      aiRequestId = null;
      clearAiStreamListener();
      if (payload.phase === "deadline_exceeded") error = payload.error ?? "AI request exceeded its deadline";
    } else if (payload.phase === "failed") {
      aiBusy = false;
      aiRequestId = null;
      clearAiStreamListener();
      error = payload.error ?? "LM Studio rewrite failed";
    }
  }
  async function startAiRewrite() {
    if (!selected || (aiMode === "rewrite" && !aiSourceSelection.trim()) || !aiInstruction.trim() || aiBusy) return;
    if (!(await flushAutoSave())) return;
    aiSourceBody = documentBody;
    aiSourceRevision = loadedDocumentRevision;
    aiStreamText = "";
    aiPreviewOutput = "";
    aiLastSequence = -1;
    aiBusy = true;
    try {
      const remoteConsent = aiSettings.remote.consents.some((consent) => consent.projectId === projectInfo?.root && consent.provider === aiSettings.remote.provider && consent.endpoint === aiSettings.remote.endpoint);
      if (aiUseRemote && (!projectInfo?.root || !remoteConsent || !remoteCredential?.configured)) {
        throw new Error("Remote AI requires an approved provider, endpoint, and OS credential for this project.");
      }
      const sourceText = aiMode === "generate"
        ? aiGenerationContext.trim() || documentBody.trim() || "[CURSOR]"
        : aiSourceSelection;
      const retrievalQuery = [selected?.name, aiInstruction, aiSourceSelectionPlain]
        .filter(Boolean)
        .join(" ")
        .slice(0, 4000);
      const requestId = aiUseRemote
        ? await project.aiGenerateRemoteText(projectInfo!.root, aiSettings.remote.provider, aiSettings.remote.endpoint, aiSettings.remote.model, aiInstruction, sourceText, selected?.id, retrievalQuery)
        : await project.aiGenerateText(aiSettings.localEndpoint, aiSettings.localModel, aiInstruction, sourceText, selected?.id, retrievalQuery);
      aiRequestId = requestId;
      aiUnlisten = await listen<AiStreamEvent>(`ai-stream:${requestId}`, (event) => {
        handleAiEvent(event.payload);
      });
      const buffered = await project.aiPollText(requestId);
      for (const event of buffered) handleAiEvent(event);
    } catch (cause) {
      clearAiStreamListener();
      aiBusy = false;
      aiRequestId = null;
      error = friendlyError(cause);
    }
  }
  async function acceptAiRewrite() {
    if (!selected || !aiPreviewOutput || aiBusy) return;
    if (documentBody !== aiSourceBody || loadedDocumentRevision !== aiSourceRevision) {
      error = "The document changed while the rewrite was being prepared. Discard it and try again.";
      return;
    }
    const validationError = validateAiProposal(aiPreviewOutput);
    if (validationError) {
      error = validationError;
      return;
    }
    if (aiMode === "generate") {
      if (!editorRef?.insertAiTextAtRequest(aiPreviewOutput)) {
        error = "The editor position is no longer available. Discard it and try again.";
        return;
      }
      markEntryDirty();
      if (await saveDocument()) closeAiRewrite();
      return;
    }
    const start = documentBody.indexOf(aiSourceSelection);
    if (start < 0) {
      error = "The selected Markdown is no longer present in the document. Discard it and try again.";
      return;
    }
    documentBody = `${documentBody.slice(0, start)}${aiPreviewOutput}${documentBody.slice(start + aiSourceSelection.length)}`;
    markEntryDirty();
    if (await saveDocument()) closeAiRewrite();
  }
  async function persistRecentProjects(next: RecentProject[]) {
    recentProjects = next;
    try {
      const settings = await project.settingsUpdate({ general: { recentProjects: next } });
      recentProjects = settings.general.recentProjects;
    } catch {
      localStorage.setItem(recentProjectsKey, JSON.stringify(next));
    }
  }
  function rememberProject(info: ProjectInfo) {
    void persistRecentProjects([{ name: info.name, root: info.root }, ...recentProjects.filter((entry) => entry.root !== info.root)].slice(0, 6));
  }
  function removeRecentProject(root: string) {
    void persistRecentProjects(recentProjects.filter((entry) => entry.root !== root));
  }
  async function loadRecentProjects() {
    try {
      const settings = await project.settingsGet();
      recentProjects = settings.general.recentProjects.slice(0, 6);
      aiSettings = settings.ai;
      await refreshRemoteCredential();
      if (recentProjects.length > 0 || settingsMigrated) return;
      const stored = JSON.parse(localStorage.getItem(recentProjectsKey) ?? "[]");
      if (Array.isArray(stored)) {
        const migrated = stored
          .filter((item): item is RecentProject => typeof item?.name === "string" && typeof item?.root === "string")
          .slice(0, 6);
        if (migrated.length > 0) {
          await persistRecentProjects(migrated);
          localStorage.removeItem(recentProjectsKey);
        }
      }
      settingsMigrated = true;
    } catch {
      try {
        const stored = JSON.parse(localStorage.getItem(recentProjectsKey) ?? "[]");
        if (Array.isArray(stored)) recentProjects = stored.filter((item): item is RecentProject => typeof item?.name === "string" && typeof item?.root === "string").slice(0, 6);
      } catch {
        recentProjects = [];
      }
    }
  }
  async function loadEntities() {
    entities = await project.listEntities();
  }

  async function refreshGit() {
    gitBusy = true;
    gitMessage = "";
    try {
      gitStatus = await project.gitStatus();
      gitPreflight = gitStatus.repository ? await project.gitStagingPreview() : null;
      gitLog = gitStatus.repository ? await project.gitLog() : [];
    } catch (cause) { gitMessage = friendlyError(cause); }
    finally { gitBusy = false; }
  }

  async function finishOpening(info?: ProjectInfo) {
    projectInfo = info ?? await project.info();
    if (!projectInfo) throw new Error("The project did not return an identity");
    modules = await project.listModuleManifests();
    await reconcileWorkspaceSection();
    rememberProject(projectInfo);
    await loadEntities();
    await refreshGit();
    await refreshAdmin();
    ready = true;
  }

  async function runProjectTransition(message: string, operation: () => Promise<void>) {
    if (projectTransitionBusy) return;
    projectTransitionBusy = true;
    projectTransitionMessage = message;
    error = "";
    try {
      await operation();
    } catch (cause) {
      error = friendlyError(cause);
    } finally {
      projectTransitionBusy = false;
      projectTransitionMessage = "";
    }
  }

  async function openWorkspace() {
    await runProjectTransition("Opening workspace…", async () => {
      await project.openDefault();
      await finishOpening();
    });
  }

  async function openProjectDirectory() {
    if (projectTransitionBusy) return;
    showProjectMenu = false;
    try {
      const selection = await project.pickDirectory();
      const path = typeof selection === "string" ? selection : null;
      if (!path) return;
      await runProjectTransition("Opening project…", async () => {
        if (!(await flushAutoSave())) return;
        await project.close();
        await finishOpening(await project.openDirectory(path));
      });
    } catch (cause) { error = friendlyError(cause); }
  }

  async function openRecentProject(path: string) {
    if (projectTransitionBusy) return;
    showProjectMenu = false;
    await runProjectTransition("Opening project…", async () => {
      if (!(await flushAutoSave())) return;
      await project.close();
      await finishOpening(await project.openDirectory(path));
    });
  }

  async function closeProject() {
    if (projectTransitionBusy) return;
    showProjectMenu = false;
    await runProjectTransition("Closing project…", async () => {
      if (!(await flushAutoSave())) return;
      await leavePluginView();
      await project.close();
      clearSelection();
      projectInfo = null;
      adminPlugins = null;
      hostView = null;
      sandboxView = null;
      gitStatus = null;
      gitPreflight = null;
      gitLog = [];
      ready = false;
    });
  }

  async function initializeGit() {
    gitBusy = true;
    gitMessage = "";
    try { await project.gitInit(); await refreshGit(); }
    catch (cause) { gitMessage = friendlyError(cause); }
    finally { gitBusy = false; }
  }

  async function flushAutoSave() {
    cancelAutoSave();
    if (!hasUnsavedChanges) return true;
    return saveDocument();
  }

  async function loadSelectedState(entity: Entity) {
    closeAiFieldFill();
    const context = contextFor();
    const record = await context.entities.get(entity.id as UUID);
    const document = record?.documents[0];
    documentBody = normalizeDocument(document?.body ?? "", document?.format);
    loadedDocumentRevision = (await project.listDocuments(entity.id))[0]?.revision ?? "";
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
      return [key, Array.isArray(value) ? value.map(String) : String(value ?? "")];
    }));
    relationships = (await context.relationships.list(entity.id as UUID)).map((relationship) => ({ id: relationship.id, source_id: relationship.sourceId, target_id: relationship.targetId, relationship_type: relationship.type, metadata: JSON.stringify(relationship.metadata), revision: relationship.revision }));
    assets = (await context.assets.list(entity.id as UUID)).map((asset) => ({ id: asset.id, entity_id: asset.entityId, namespace: asset.namespace, filename: asset.filename, content_hash: asset.contentHash, size: asset.size, mime_type: asset.mimeType, path: asset.path, created_at: asset.createdAt, revision: "" }));
    mapLocations = await project.listMapLocations(entity.id);
    savedAt = "";
  }

  async function openMapLocation(location: MapLocation) {
    try {
      await project.mapsNavigation("openMap", { mapEntityId: location.mapEntityId, linkId: location.id });
    } catch (cause) {
      const message = friendlyError(cause);
      if (message.includes("link-unresolved")) {
        mapReconcileNotice = `This link is unresolved — the map feature it pointed to was removed or renumbered.`;
        return;
      }
      error = message;
    }
  }

  async function unlinkMapLocation(location: MapLocation) {
    if (!selected || !confirm(`Unlink ${location.label || "this location"}? The entity and map feature will remain.`)) return;
    try { await project.unlinkMapLocation(selected.id, location.id); mapLocations = await project.listMapLocations(selected.id); }
    catch (cause) { error = friendlyError(cause); }
  }

  async function editMapLocation(location: MapLocation) {
    if (!selected) return;
    const role = prompt("Edit location role", location.role)?.trim();
    if (!role) return;
    const validityText = prompt("Validity JSON (use null bounds for unbounded)", JSON.stringify(location.validity));
    if (validityText === null) return;
    let validity: MapLocation["validity"];
    try { validity = JSON.parse(validityText) as MapLocation["validity"]; } catch { error = "Validity must be valid JSON with from and to fields."; return; }
    try { await project.upsertMapLocation(selected.id, { ...location, role, validity }); mapLocations = await project.listMapLocations(selected.id); }
    catch (cause) { error = friendlyError(cause); }
  }

  async function rebindMapLocation(location: MapLocation) {
    if (!selected) return;
    const raw = prompt("Rebind to normalized point x,y (0..1)", "0.5,0.5") ?? "";
    const [x, y] = raw.split(",").map(Number);
    if (![x, y].every((value) => Number.isFinite(value) && value >= 0 && value <= 1)) { error = "Use normalized coordinates such as 0.5,0.5."; return; }
    try { await project.upsertMapLocation(selected.id, { ...location, anchor: { kind: "point", point: [x, y] } }); mapLocations = await project.listMapLocations(selected.id); }
    catch (cause) { error = friendlyError(cause); }
  }

  function captureAnchorLabel(): string {
    const anchor = mapSelection as { kind?: string; featureKind?: string; featureId?: string; point?: number[] } | null;
    if (!anchor) return "Nothing selected yet — click the map first";
    if (anchor.kind === "provider-feature") return `${anchor.featureKind ?? "feature"} ${anchor.featureId ?? ""}`.trim();
    return anchor.point ? `Point (${anchor.point[0].toFixed(3)}, ${anchor.point[1].toFixed(3)})` : "Arbitrary selection";
  }

  async function requestMapCapture() {
    if (!currentMapId()) return;
    try {
      mapSelection = await project.mapsEditorCaptureAnchor();
      captureMode = "existing";
      captureEntityId = "";
      captureTemplateKey = "";
      captureName = "";
      captureRole = "story-location";
      captureDialog = true;
    } catch (cause) { error = friendlyError(cause); }
  }

  function closeCaptureDialog() {
    captureDialog = false;
    mapSelection = null;
  }

  async function submitMapCapture() {
    const mapId = currentMapId();
    if (!mapId || !mapSelection) return;
    let entity: Entity;
    try {
      if (captureMode === "create") {
        const option = createGroups().flatMap((group) => group.options).find((candidate) => candidate.key === captureTemplateKey);
        if (!option || !captureName.trim()) throw new Error("Choose a template and enter a name for the new entity.");
        const context = buildModuleContext(option.module, projectInfo?.root ?? "");
        const created = await context.entities.create({ name: captureName.trim(), type: option.template.entityType, fields: {}, relationships: {} });
        await loadEntities();
        entity = { id: created.id, name: created.name, entity_type: created.type, deleted: created.deleted, created_at: created.createdAt, updated_at: created.updatedAt, revision: "" };
      } else {
        const found = entities.find((candidate) => candidate.id === captureEntityId);
        if (!found) throw new Error("Choose an entity to link.");
        entity = found;
      }
      const location: MapLocation = { id: crypto.randomUUID(), mapEntityId: mapId, role: captureRole.trim() || "story-location", label: entity.name, anchor: mapSelection, validity: { from: null, to: null } };
      await project.upsertMapLocation(entity.id, location);
      if (selected?.id === entity.id) mapLocations = await project.listMapLocations(entity.id);
      await project.mapsEditorFocusLink(location.id);
      closeCaptureDialog();
    } catch (cause) { error = friendlyError(cause); }
  }

  async function linkEntityToMap() {
    if (!selected) return;
    const maps = entities.filter((entity) => entity.entity_type === "daena.maps:map");
    if (maps.length === 0) { error = "Create or enable a Maps map before linking a location."; return; }
    let map = maps[0];
    if (maps.length > 1) {
      const pick = prompt(`Pick a map:\n${maps.map((candidate, index) => `${index + 1}. ${candidate.name}`).join("\n")}`);
      const choice = maps[Number(pick) - 1];
      if (!choice) return;
      map = choice;
    }
    const role = prompt("Role for this map location", "story-location")?.trim();
    if (!role) return;
    const raw = prompt("Normalized point x,y (0..1)", "0.5,0.5") ?? "";
    const [x, y] = raw.split(",").map(Number);
    if (![x, y].every((value) => Number.isFinite(value) && value >= 0 && value <= 1)) { error = "Use normalized coordinates such as 0.5,0.5."; return; }
    try {
      const location: MapLocation = { id: crypto.randomUUID(), mapEntityId: map.id, role, label: selected.name, anchor: { kind: "point", point: [x, y] }, validity: { from: null, to: null } };
      await project.upsertMapLocation(selected.id, location);
      mapLocations = await project.listMapLocations(selected.id);
    } catch (cause) { error = friendlyError(cause); }
  }

  async function selectEntity(entity: Entity) {
    if (selected?.id === entity.id) return;
    if (!(await flushAutoSave())) return;
    editorFullscreen = false;
    selected = entity;
    hasUnsavedChanges = false;
    documentConflict = null;
    documentRevision = 0;
    error = "";
    try {
      await loadSelectedState(entity);
    } catch (cause) { error = friendlyError(cause); }
  }

  async function reloadSelectedFromDisk() {
    if (!selected) return;
    const current = entities.find((entity) => entity.id === selected?.id) ?? selected;
    selected = current;
    await loadSelectedState(current);
    hasUnsavedChanges = false;
    documentRevision = 0;
    documentConflict = null;
    conflictDiskBody = "";
    savedAt = "";
  }

  function handlePortableFilesChanged(paths: string[]) {
    if (paths.length === 0) return;
    projectDiagnostics = [
      `Portable files changed externally (${paths.length}); review and import the checkpoint explicitly if needed.`,
    ];
  }

  async function reloadConflict() {
    try { await reloadSelectedFromDisk(); }
    catch (cause) { error = friendlyError(cause); }
  }

  async function overwriteConflict() {
    if (!selected) return;
    try {
      const documents = await project.listDocuments(selected.id);
      loadedDocumentRevision = documents[0]?.revision ?? "";
      documentConflict = null;
      conflictDiskBody = "";
      if (!(await saveDocument())) documentConflict = { paths: [], diagnostics: ["The draft could not be written as a new revision."] };
    } catch (cause) { documentConflict = { paths: [], diagnostics: [friendlyError(cause)] }; }
  }

  async function saveConflictRecoveryCopy() {
    if (!selected) return;
    try {
      const path = await project.saveRecoveryCopy(selected.id, documentBody);
      error = `Draft saved as ${path}`;
    } catch (cause) { error = friendlyError(cause); }
  }

  async function createEntity(event: SubmitEvent) {
    event.preventDefault();
    if (projectDiagnostics.length > 0) return;
    const option = selectedCreateOption();
    if (!name.trim() || !option || !option.module.enabled) return;
    try {
      const fieldsForCreate: Record<string, unknown> = {};
      const relationshipsForCreate: Record<string, UUID[]> = {};
      for (const { field, required } of createFieldsFor(option)) {
        const value = createFieldValues[field.key];
        const empty = value === "" || value === null || value === undefined || (typeof value === "string" && value.trim() === "") || (Array.isArray(value) && value.length === 0);
        if (empty) {
          if (required) throw new Error(`${field.label} is required`);
          continue;
        }
        if (field.type === "relationship") {
          if (!Array.isArray(value)) throw new Error(`${field.label} must contain one or more entities`);
          relationshipsForCreate[field.key] = value as UUID[];
        } else if (field.type === "number") {
          const numberValue = typeof value === "number" ? value : Number(value);
          if (!Number.isFinite(numberValue)) throw new Error(`${field.label} must be a number`);
          fieldsForCreate[field.key] = numberValue;
        } else if (field.type === "date") {
          if (!isCompleteCalendarDate(value)) throw new Error(`${field.label} needs a year, month, and day`);
          fieldsForCreate[field.key] = parseCalendarDate(value) ?? value;
        } else {
          fieldsForCreate[field.key] = value;
        }
      }
      const context = buildModuleContext(option.module, projectInfo?.root ?? "");
      const created = await context.entities.create({
        name: name.trim(),
        type: option.template.entityType,
        fields: fieldsForCreate,
        relationships: relationshipsForCreate,
        document: createDocumentBody.trim() ? { body: createDocumentBody.trim(), format: "markdown" } : undefined,
      });
      section = option.template.entityType === "event" ? "timeline" : option.template.entityType === "manuscript" || option.template.entityType === "reference-page" ? "writing" : "lore";
      if (option.template.entityType === "manuscript") writingView = "manuscripts";
      if (option.template.entityType === "reference-page") writingView = "reference";
      name = "";
      showCreateForm = false;
      resetCreateFields(null);
      await loadEntities();
      await selectEntity({ id: created.id, name: created.name, entity_type: created.type, deleted: created.deleted, created_at: created.createdAt, updated_at: created.updatedAt, revision: "" });
    } catch (cause) { error = friendlyError(cause); }
  }

  function closeCreateForm() {
    requestCreateDiscard(() => {
      showCreateForm = false;
      name = "";
      resetCreateFields(null);
    });
  }

  function toggleCreateForm() {
    if (showCreateForm) {
      closeCreateForm();
      return;
    }
    const options = createOptions();
    if (options.length === 0) {
      error = "Enable a module with a creation template to get started.";
      return;
    }
    if (!options.some((option) => option.key === selectedCreateKey)) selectCreateOption(options[0].key);
    else if (Object.keys(createFieldValues).length === 0) resetCreateFields(selectedCreateOption());
    showCreateForm = true;
    setTimeout(() => document.getElementById("new-entity")?.focus(), 0);
  }

  function updateField(key: string, event: Event) {
    if (projectDiagnostics.length > 0) return;
    const target = event.currentTarget as HTMLInputElement | HTMLSelectElement;
    const value = target instanceof HTMLSelectElement && target.multiple
      ? Array.from(target.selectedOptions, (option) => option.value)
      : target.value;
    fields = { ...fields, [key]: value };
    markEntryDirty();
  }
  async function saveDocument(): Promise<boolean> {
    if (!selected || !sectionEnabled() || documentConflict || projectDiagnostics.length > 0) return false;
    cancelAutoSave();
    const entityId = selected.id;
    const body = documentBody;
    const revision = documentRevision;
    const definitionsForSave = definitions().filter((definition) => definition.type !== "relationship");
    const fieldsSnapshot = { ...fields };
    isSaving = true;
    try {
      await project.saveEntry({
        document: { entity_id: entityId, body, format: "markdown" },
        fields: definitionsForSave.map((definition) => {
          const value = fieldsSnapshot[definition.key] ?? "";
          return { entity_id: entityId, namespace: activeManifest()?.schemas[0]?.namespace ?? activeModuleId(), key: definition.key, value: definition.type === "date" && value ? parseCalendarDate(value) ?? value : value, revision: "" };
        }),
      }, { expectedRevision: loadedDocumentRevision || undefined });
      const documents = await project.listDocuments(entityId);
      loadedDocumentRevision = documents[0]?.revision ?? "";
      if (selected?.id === entityId && documentRevision === revision) {
        hasUnsavedChanges = false;
        savedAt = new Date().toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" });
      }
      return true;
    } catch (cause) {
      error = friendlyError(cause);
      return false;
    } finally { isSaving = false; }
  }
  async function archiveSelected() {
    if (projectDiagnostics.length > 0) return;
    if (!(await flushAutoSave())) return;
    if (!selected || !confirm(`Archive ${selected.name}?`)) return;
    try {
      await loadEntities();
      const current = entities.find((entity) => entity.id === selected?.id);
      if (!current?.revision) throw new Error("The entity revision is unavailable. Reload the project and try again.");
      await contextFor().entities.delete(current.id as UUID, { expectedRevision: current.revision });
      clearSelection();
      await loadEntities();
    } catch (cause) { error = friendlyError(cause); }
  }
  function selectedRelationshipIds(definition: FieldDefinition) {
    if (!selected || !definition.relationshipType) return [];
    return relationships
      .filter((relationship) => relationship.source_id === selected!.id && relationship.relationship_type === definition.relationshipType)
      .map((relationship) => relationship.target_id);
  }
  async function updateRelationshipField(definition: FieldDefinition, targetIds: string[]) {
    if (projectDiagnostics.length > 0) return;
    if (!selected || !definition.relationshipType) return;
    const desired = new Set(targetIds);
    const current = relationships.filter((relationship) => relationship.source_id === selected!.id && relationship.relationship_type === definition.relationshipType);
    const toRemove = current.filter((relationship) => !desired.has(relationship.target_id));
    const toAdd = [...desired].filter((targetId) => !current.some((relationship) => relationship.target_id === targetId));
    try {
      const context = contextFor();
      await Promise.all(toRemove.map((relationship) => context.relationships.delete(relationship.id as UUID, relationship.relationship_type, { expectedRevision: relationship.revision })));
      const created = await Promise.all(toAdd.map((targetId) => project.createRelationship(
        selected!.id,
        targetId,
        definition.relationshipType!,
        {},
      )));
      const removedIds = new Set(toRemove.map((relationship) => relationship.id));
      relationships = [
        ...relationships.filter((relationship) => !removedIds.has(relationship.id)),
        ...created,
      ];
    } catch (cause) { error = friendlyError(cause); }
  }
  function mimeTypeFor(filename: string) {
    const extension = filename.split(".").pop()?.toLowerCase();
    return extension === "png" ? "image/png" : extension === "jpg" || extension === "jpeg" ? "image/jpeg" : extension === "gif" ? "image/gif" : extension === "mp4" ? "video/mp4" : extension === "webm" ? "video/webm" : "application/octet-stream";
  }
  async function attachAsset() {
    if (projectDiagnostics.length > 0) return;
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
    if (!installed) return;
    if (!installed.enabled) {
      askConfirm(
        "Grant plugin capabilities",
        `Enable ${installed.name} with these requested capabilities?`,
        "Enable plugin",
        async () => {
          await project.enableModule(id, installed.capabilities);
          modules = await project.listModuleManifests();
          await reconcileWorkspaceSection();
          await refreshAdmin();
        },
        installed.capabilities,
      );
      return;
    }
    try {
      await project.disableModule(id);
      modules = await project.listModuleManifests();
      await reconcileWorkspaceSection();
      await refreshAdmin();
      if (!selectedCreateOption()) selectedCreateKey = "";
      if (showCreateForm && !selectedCreateOption()) closeCreateForm();
    } catch (cause) { error = friendlyError(cause); }
  }
  async function refreshAdmin() {
    adminBusy = true;
    try {
      const view = await project.adminView();
      adminPlugins = view.plugins;
    } catch (cause) { error = friendlyError(cause); }
    finally { adminBusy = false; }
  }
  function setAdminPluginEnabled(id: string, enabled: boolean) {
    if (!adminPlugins) return;
    adminPlugins = adminPlugins.map((plugin) =>
      plugin.id === id
        ? { ...plugin, enabled, runtimeRunning: enabled ? plugin.runtimeRunning : false }
        : plugin,
    );
  }
  async function openSettings(section: SettingsSection = "general") {
    showSettings = true;
    settingsSection = section;
    await leavePluginView();
    projectionView = null;
    installSummary = null;
    deleteBackupPath = "";
    if (section === "plugins" && ready) {
      adminPlugins = null;
      await refreshAdmin();
    }
    if (section === "ai" && ready) {
      aiIndexMessage = "";
      await refreshAiIndexStatus();
    }
  }
  function closeSettings() {
    showSettings = false;
  }
  $effect(() => {
    if (!showSettings || settingsSection !== "plugins" || !ready) return;
    void refreshAdmin();
  });
  async function installFromPicker() {
    try {
      const selection = await project.pickPluginPackage();
      const source = typeof selection === "string" ? selection : null;
      if (source) await installPackage(source);
    } catch (cause) { error = friendlyError(cause); }
  }
  async function installPackage(path: string, allowUnsigned = false) {
    installing = true;
    installConsent = null;
    installSummary = null;
    try {
      const installed = await project.installPlugin(path, allowUnsigned);
      installSummary = { id: installed.id, version: installed.version, signed: installed.signed, digest: installed.digest };
      await refreshAdmin();
      modules = await project.listModuleManifests();
    } catch (cause) {
      const message = friendlyError(cause);
      if (!allowUnsigned && message.toLowerCase().includes("unsigned")) {
        installConsent = { path, message };
      } else {
        error = message;
      }
    } finally { installing = false; }
  }
  async function installWithConsent() {
    if (!installConsent) return;
    const path = installConsent.path;
    installConsent = null;
    await installPackage(path, true);
  }
  async function previewUpgrade(plugin: PluginAdminEntry, version: string) {
    try {
      const plan = await project.pluginUpgradePlan(plugin.id, version);
      upgradePreview = { entry: plugin, version, plan };
    } catch (cause) { error = friendlyError(cause); }
  }
  async function confirmUpgrade() {
    const preview = upgradePreview;
    if (!preview) return;
    upgradeBusy = true;
    try {
      await project.upgradePlugin(preview.entry.id, preview.version, true);
      upgradePreview = null;
      await refreshAdmin();
      modules = await project.listModuleManifests();
      error = `Upgraded ${preview.entry.name} to ${preview.version}.`;
    } catch (cause) { error = friendlyError(cause); }
    finally { upgradeBusy = false; }
  }
  function askConfirm(title: string, message: string, confirmLabel: string, run: () => Promise<void>, capabilities?: string[]) {
    confirmAction = { title, message, confirmLabel, run, capabilities };
  }
  function selectedUninstallableVersion(plugin: PluginAdminEntry) {
    return plugin.installedVersions.find((version) => version.isSelected && !version.bundled) ?? null;
  }
  async function runConfirm() {
    const action = confirmAction;
    if (!action) return;
    confirmBusy = true;
    try {
      await action.run();
      confirmAction = null;
    }
    catch (cause) { error = friendlyError(cause); }
    finally { confirmBusy = false; }
  }
  async function confirmRollback(plugin: PluginAdminEntry, version: string) {
    askConfirm(
      "Roll back plugin",
      `Restore ${plugin.name} to version ${version}? The pre-upgrade backup will be restored and the previous version reactivated.`,
      "Roll back",
      async () => {
        await project.rollbackPlugin(plugin.id, version);
        await refreshAdmin();
        modules = await project.listModuleManifests();
        error = `Rolled ${plugin.name} back to ${version}.`;
      },
    );
  }
  async function confirmUninstall(plugin: PluginAdminEntry, version: string) {
    askConfirm(
      "Uninstall code",
      `Remove ${plugin.name} ${version} from the plugin library? It will still be listed if another version is installed.`,
      "Uninstall",
      async () => {
        await project.uninstallPluginCode(plugin.id, version);
        await refreshAdmin();
        modules = await project.listModuleManifests();
        error = `Uninstalled ${plugin.name} ${version}.`;
      },
    );
  }
  async function retryPlugin(plugin: PluginAdminEntry) {
    try {
      await project.retryPlugin(plugin.id);
      if (!plugin.enabled) await project.enableModule(plugin.id, plugin.capabilities);
      modules = await project.listModuleManifests();
      await reconcileWorkspaceSection();
      await refreshAdmin();
      error = `Retried ${plugin.name}.`;
    } catch (cause) { error = friendlyError(cause); }
  }
  async function openDeleteData(plugin: PluginAdminEntry) {
    deleteInput = "";
    deleteBackupPath = "";
    deleteTarget = plugin;
  }
  async function confirmDeleteData() {
    const target = deleteTarget;
    if (!target || deleteInput.trim() !== target.id) return;
    deleteBusy = true;
    try {
      deleteBackupPath = await project.deletePluginData(target.id, deleteInput.trim());
      deleteTarget = null;
      await refreshAdmin();
      modules = await project.listModuleManifests();
    } catch (cause) { error = friendlyError(cause); }
    finally { deleteBusy = false; }
  }
  async function togglePluginEnabled(plugin: PluginAdminEntry) {
    if (!plugin.enabled) {
      askConfirm(
        "Grant plugin capabilities",
        `Enable ${plugin.name} with these requested capabilities?`,
        "Enable plugin",
        async () => {
          pluginActionId = plugin.id;
          try {
            await project.enableModule(plugin.id, plugin.capabilities);
            modules = await project.listModuleManifests();
            await reconcileWorkspaceSection();
            await refreshAdmin();
          } finally {
            pluginActionId = null;
          }
        },
        plugin.capabilities,
      );
      return;
    }
    pluginActionId = plugin.id;
    try {
      await project.disableModule(plugin.id);
      modules = await project.listModuleManifests();
      await reconcileWorkspaceSection();
      setAdminPluginEnabled(plugin.id, false);
      await refreshAdmin();
      if (hostView?.plugin.id === plugin.id) hostView = null;
      if (sandboxView?.plugin.id === plugin.id) sandboxView = null;
    } catch (cause) { error = friendlyError(cause); }
    finally { pluginActionId = null; }
  }
  async function reviewPluginCapabilities(plugin: PluginAdminEntry) {
    askConfirm(
      "Review plugin capabilities",
      `Update ${plugin.name} with these requested capabilities?`,
      "Update capabilities",
      async () => {
        pluginActionId = plugin.id;
        try {
          await project.enableModule(plugin.id, plugin.capabilities);
          await refreshAdmin();
          modules = await project.listModuleManifests();
        } finally {
          pluginActionId = null;
        }
      },
      plugin.capabilities,
    );
  }
  function capabilityLabel(capability: string) {
    const labels: Record<string, string> = {
      "entity.read": "Read entities",
      "entity.write": "Create and edit entities",
      "entity.delete": "Delete entities",
      "document.read": "Read documents",
      "document.write": "Save and edit documents",
      "relationship.read": "Read relationships",
      "relationship.write": "Create and delete relationships",
      "search.query": "Search the whole world",
      "asset.register": "Register assets",
      "asset.read:self": "Read assets in own namespace",
      "field.read:self": "Read fields in own namespace",
      "field.write:self": "Write fields in own namespace",
    };
    return labels[capability] ?? capability;
  }
  function shortDigest(digest: string) { return digest ? digest.slice(0, 12) : ""; }
  function installedAtLabel(timestamp: number) { return timestamp ? new Date(timestamp * 1000).toLocaleString() : ""; }
  function clearSelection() {
    cancelAutoSave();
    editorFullscreen = false;
    hasUnsavedChanges = false;
    selected = null;
    documentBody = "";
    fields = {};
    relationships = [];
    assets = [];
    savedAt = "";
    loadedDocumentRevision = "";
    documentConflict = null;
    conflictDiskBody = "";
    projectDiagnostics = [];
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
  async function rebuildSearchIndex() {
    const request = ++searchRequest;
    try {
      await project.rebuildSearch();
      const term = globalQuery.trim();
      if (!term || request !== searchRequest) return;
      const matches = await project.search(term);
      if (request === searchRequest) searchMatches = matches;
    } catch (cause) {
      if (request === searchRequest) error = friendlyError(cause);
    }
  }
  async function importPortableCheckpoint() {
    try {
      await project.importCheckpoint();
      projectInfo = await project.info();
      await loadEntities();
      projectDiagnostics = [];
    } catch (cause) { error = friendlyError(cause); }
  }
  async function createPortableBackup() {
    return project.backup();
  }
  async function createRecoveryBackup() {
    return project.recoveryBackup();
  }
  async function restoreRecoveryBackup(path: string) {
    await project.restoreRecoveryBackup(path);
    projectInfo = await project.info();
    await loadEntities();
  }
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
  onMount(() => {
    void loadRecentProjects();
    void closeNativePluginWebviews();
    let unlisten: (() => void) | undefined;
    void listen<string[]>("project-portable-files-changed", (event) => handlePortableFilesChanged(event.payload)).then((cleanup) => { unlisten = cleanup; }).catch(() => {});
    let unlistenMaps: (() => void) | undefined;
    void listen<{ mapEntityId: string; linkId?: string }>("maps-navigation", async (event) => {
      try {
        if (!entities.length) await loadEntities();
        const map = entities.find((entity) => entity.id === event.payload.mapEntityId);
        const item = pluginViews().find((candidate) => candidate.plugin.id === "daena.maps");
        if (!map || !item) throw new Error("map-unavailable: enable the Maps module to open this location");
        selected = map;
        await loadSelectedState(map);
        await openPluginView(item);
        const linkId = event.payload.linkId ?? null;
        mapFocusLinkId = linkId;
        if (linkId && sandboxView?.plugin.id === "daena.maps") {
          setTimeout(() => { void project.mapsEditorFocusLink(linkId).catch(() => {}); }, 400);
        }
      } catch (cause) { error = friendlyError(cause); }
    }).then((cleanup) => { unlistenMaps = cleanup; }).catch(() => {});
    let unlistenMapsState: (() => void) | undefined;
    void listen<{ mapEntityId: string; status: string; detail: unknown }>("maps-state", (event) => {
      const { mapEntityId, status, detail } = event.payload;
      mapSaveStates[mapEntityId] = { status, detail };
      if (status === "reconcile") {
        const reconcileDetail = detail as { unresolved?: unknown } | null;
        const unresolved = Array.isArray(reconcileDetail?.unresolved) ? reconcileDetail.unresolved.length : 0;
        mapReconcileNotice = unresolved > 0
          ? `${unresolved} link${unresolved === 1 ? "" : "s"} on this map are unresolved — the features they pointed to were removed or renumbered.`
          : "";
        if (selected && mapLocations.length > 0) void project.listMapLocations(selected.id).then((locations) => { mapLocations = locations; }).catch(() => {});
      }
    }).then((cleanup) => { unlistenMapsState = cleanup; }).catch(() => {});
    let unlistenMapsSelection: (() => void) | undefined;
    void listen<{ mapEntityId: string; anchor: unknown }>("maps-selection", (event) => {
      if (event.payload.mapEntityId === currentMapId()) mapSelection = event.payload.anchor;
    }).then((cleanup) => { unlistenMapsSelection = cleanup; }).catch(() => {});
    const syncTimer = window.setInterval(() => {
      if (!ready) return;
      void project.info().then((info) => { if (info) projectInfo = info; }).catch(() => {});
    }, 1000);
    return () => {
      window.clearInterval(syncTimer);
      if (aiModelsMessageTimer !== null) window.clearTimeout(aiModelsMessageTimer);
      unlisten?.();
      clearAiFieldListener();
      unlistenMaps?.();
      unlistenMapsState?.();
      unlistenMapsSelection?.();
    };
  });
</script>

<svelte:head><title>Daena Archive</title><link rel="icon" href={logoUrl} /></svelte:head>

<main class="studio-shell" aria-label="Daena Archive">
  {#if projectTransitionBusy}
    <div class="project-transition-backdrop" role="status" aria-live="polite" aria-busy="true">
      <div class="project-transition-card">
        <span class="project-transition-spinner" aria-hidden="true"></span>
        <strong>{projectTransitionMessage}</strong>
        <small>{projectTransitionMessage === "Closing project…" ? "Returning to the project launcher." : "Your project will be ready in a moment."}</small>
      </div>
    </div>
  {/if}
  <aside class:startup-rail={!ready} class="rail">
    <div class="brand"><img class="brand-logo" src={logoUrl} alt="Daena Archive" /></div>
    {#if !ready}
      <div class="startup-actions">
        <button class="rail-button startup-primary" onclick={openProjectDirectory}><span class="rail-icon">↗</span><span>Open project folder</span></button>
      </div>
      {#if recentProjects.length > 0}
        <div class="rail-label recent-label">RECENT PROJECTS</div>
        <div class="recent-projects">{#each recentProjects as recent}<div class="recent-project"><button class="recent-project-open" onclick={() => openRecentProject(recent.root)}><span class="project-dot"></span><span><strong>{recent.name}</strong><small>{recent.root}</small></span></button><button class="recent-project-remove" aria-label={`Remove ${recent.name} from recent projects`} title="Remove from recent projects" onclick={() => removeRecentProject(recent.root)}>×</button></div>{/each}</div>
      {/if}
    {:else}
      <div class="project-switcher">
        <button type="button" aria-expanded={showProjectMenu} aria-haspopup="menu" class:active={showProjectMenu} class="project-card" onclick={() => showProjectMenu = !showProjectMenu}>
          <span class:online={ready} class="project-dot"></span>
          <span class="project-copy"><strong>{projectInfo?.name ?? "Local project"}</strong></span>
          <span class="project-chevron" aria-hidden="true">⌄</span>
        </button>
        {#if showProjectMenu}
          <div class="project-menu" role="menu">
            <button class="rail-button" role="menuitem" onclick={openProjectDirectory}><span class="rail-icon">↗</span><span>Open another folder</span></button>
            <button class="rail-button" role="menuitem" onclick={() => void rebuildSearchIndex()}><span class="rail-icon">⌕</span><span>Rebuild index</span></button>
            <button class="rail-button" role="menuitem" onclick={seedExample}><span class="rail-icon">✣</span><span>Seed example</span></button>
            <button class="rail-button" role="menuitem" onclick={closeProject}><span class="rail-icon">×</span><span>Close project</span></button>
          </div>
        {/if}
      </div>
      <div class:sync-failed={projectInfo?.sync.state === "failed"} class:sync-pending={projectInfo?.sync.state === "exporting"} class="sync-summary" aria-live="polite" title={projectInfo?.sync.export_error ?? undefined}>
        <span class="sync-dot"></span><span>{syncStatusLabel(projectInfo)}</span>
      </div>
      {#if enabledWorkspaceSections().length > 0}<button aria-expanded={showCreateForm} class="rail-create-button" onclick={toggleCreateForm}><span class="rail-icon">＋</span><span>New entry</span></button>{/if}
      {#if enabledWorkspaceSections().length > 0}
        <div class="rail-label">WORKSPACE</div>
        <nav class="workspace-nav" aria-label="Workspace sections">
          {#each enabledWorkspaceSections() as target (target)}
            <button title={target === "maps" ? "Maps · Beta plugin — may be unstable" : undefined} aria-current={workspaceNavigationActive(target) ? "page" : undefined} class:active={workspaceNavigationActive(target)} class="rail-button" onclick={() => void switchSection(target)}><span class="rail-icon">{sectionIcon(target)}</span><span>{target === "lore" ? "Lore library" : target === "timeline" ? "Timeline" : target === "writing" ? "Writing Studio" : "Maps"}{#if target === "maps"}<em class="workspace-beta">Beta</em>{/if}</span></button>
          {/each}
        </nav>
      {/if}
      {#if pluginViews().some((item) => item.plugin.id !== "daena.maps")}
        <div class="rail-label plugin-views-label">PLUGIN VIEWS</div>
        <nav class="workspace-nav" aria-label="Plugin views">
          {#each pluginViews().filter((item) => item.plugin.id !== "daena.maps") as item (item.key)}
            <div class="plugin-nav-row">
              <button class:active={pluginNavigationActive(item)} class="rail-button" title={pluginViewLabel(item)} aria-current={pluginNavigationActive(item) ? "page" : undefined} aria-label={`Open ${item.plugin.name}: ${item.view.title}`} onclick={() => void openPluginView(item)}><span class="rail-icon">◇</span><span class="plugin-nav-title">{pluginViewLabel(item)}</span></button>
            </div>
          {/each}
        </nav>
      {/if}
    {/if}
    <div class="rail-spacer"></div>
    {#if ready}
      <button aria-expanded={showGit} class:active={showGit} class="rail-button muted-button" onclick={() => { showGit = !showGit; if (showGit) void refreshGit(); }}><span class="rail-icon">⑂</span><span>Git</span></button>
      {#if showGit}<div class="module-menu git-menu"><strong>{gitBusy ? "Checking Git…" : gitStatus?.repository ? `Git · ${gitStatus.branch || "detached"}` : "Git is not initialized"}</strong><small>{gitMessage || (gitStatus?.repository ? gitStatus.changes.length === 0 ? "Working tree clean" : `${gitStatus.changes.length} changed file${gitStatus.changes.length === 1 ? "" : "s"}` : "Initialize Git to track this project")}</small>{#if gitStatus?.repository}<button disabled={gitBusy} onclick={() => { showGit = false; void openSettings("git"); }}>Open Git settings</button>{:else}<button disabled={gitBusy} onclick={initializeGit}>{gitBusy ? "Initializing…" : "Initialize Git"}</button>{/if}{#if gitPreflight}<div class="git-preview"><strong>{gitPreflight.ready ? (gitPreflight.staging_paths.length === 0 ? "Nothing to commit" : `${gitPreflight.staging_paths.length} change${gitPreflight.staging_paths.length === 1 ? "" : "s"} ready`) : "Commit preflight blocked"}</strong>{#if gitPreflight.ready && gitPreflight.staging_paths.length > 0}<small>Review and commit in Git settings</small>{:else if gitPreflight.diagnostics.length > 0}<small>{gitPreflight.diagnostics[0]}</small>{/if}</div>{/if}</div>{/if}
    {/if}
    <button aria-expanded={showSettings} class:active={showSettings} class="rail-button muted-button" onclick={() => void openSettings()}><span class="rail-icon">⚙</span><span>Settings</span></button>
    <div class="rail-footer">v0.2 · local first</div>
  </aside>

  <section class:sandbox-active={Boolean(sandboxView)} class="app-main">
    <header class="topbar"><div class="breadcrumbs" aria-label="Breadcrumb"><span>Private studio</span><i>/</i><strong>{sectionLabel()}</strong>{#if section === "writing"}<i>/</i><span>{writingView === "manuscripts" ? "Manuscripts" : "Reference pages"}</span>{/if}{#if selected}<i>/</i><span>{selected.name}</span>{/if}</div><div class="top-actions">{#if ready}<label class="global-search"><span aria-hidden="true">⌕</span><input aria-label="Search your world" bind:value={globalQuery} placeholder="Search whole world" /></label><span class="sync-badge" title="Your work is stored locally"><span></span> Local</span>{/if}</div></header>
    {#if ready && globalQuery.trim()}<div class="search-modal" role="dialog" aria-label="World search results"><div class="search-modal-heading"><strong>Search results</strong><button class="quiet-button" aria-label="Close search" onclick={() => globalQuery = ""}>×</button></div>{#if searchMatches === null}<p class="search-state">Searching the whole world…</p>{:else if searchMatches.length === 0}<p class="search-state">No matches found.</p>{:else}<div class="search-results">{#each searchMatches as result}<button class="search-result" onclick={() => selectSearchResult(result)}><span class={`entity-glyph ${entityGlyphClass(result)}`}>{entityGlyph(result)}</span><span><strong>{result.name}</strong><small>{result.entity_type ?? "Uncategorized"}</small></span></button>{/each}</div>{/if}</div>{/if}
    {#if showCreateForm}{@const createOption = selectedCreateOption()}<div class="modal-backdrop"><form class="dialog create-dialog" onsubmit={createEntity}><div class="create-dialog-heading"><div><span class="panel-kicker">CREATE SOMETHING NEW</span><strong>Choose a starting point</strong><p>Templates set the shape of your new entry. You can fill in the details before it is saved.</p></div><button type="button" class="new-form-close" aria-label="Close create dialog" onclick={closeCreateForm}>×</button></div><div class="create-dialog-body"><aside class="create-template-panel"><div class="create-panel-label">TEMPLATES</div><div class="create-template-list">{#each createGroups() as group}<div class="create-template-group"><span>{group.module.name}</span>{#each group.options as option}<button type="button" class:selected={option.key === selectedCreateKey} class="create-template-card" onclick={() => selectCreateOption(option.key)}><span class="create-template-icon">{option.template.icon ?? option.template.name.slice(0, 1)}</span><span class="create-template-copy"><strong>{option.template.name}</strong><small>{option.template.description ?? option.template.entityType}</small></span><span class="create-template-check">{option.key === selectedCreateKey ? "✓" : ""}</span></button>{/each}</div>{/each}</div></aside><section class="create-form-panel">{#if createOption}<div class="create-form-title"><span class="panel-kicker">{createOption.module.name.toUpperCase()}</span><h2>{createOption.template.name}</h2><p>{createOption.template.description ?? `Create a new ${createOption.template.entityType}.`}</p></div><label class="create-input-field" for="new-entity"><span>Name <b>*</b></span><input id="new-entity" bind:value={name} placeholder={`e.g. ${createOption.template.name}`} autocomplete="off" /></label>{#each createFieldsFor(createOption) as item}<div class="create-input-field"><label for={`create-${item.field.key}`}><span>{item.field.label} {#if item.required}<b>*</b>{/if}</span></label>{#if item.field.type === "relationship"}<RelationshipPicker field={item.field} entities={entities} selectedIds={createRelationshipValues(item.field.key)} onChange={(ids) => setCreateRelationshipValues(item.field.key, ids)} />{:else if item.field.type === "text"}<textarea id={`create-${item.field.key}`} rows="3" value={String(createFieldValues[item.field.key] ?? "")} placeholder={`Add ${item.field.label.toLowerCase()}`} oninput={(event) => setCreateField(item.field.key, (event.currentTarget as HTMLTextAreaElement).value)}></textarea>{:else if item.field.type === "number"}<input id={`create-${item.field.key}`} type="number" value={String(createFieldValues[item.field.key] ?? "")} placeholder={`Add ${item.field.label.toLowerCase()}`} oninput={(event) => setCreateField(item.field.key, (event.currentTarget as HTMLInputElement).value)} />{:else if item.field.type === "boolean"}<label class="create-checkbox" for={`create-${item.field.key}`}><input id={`create-${item.field.key}`} type="checkbox" checked={createFieldValues[item.field.key] === true} onchange={(event) => setCreateField(item.field.key, (event.currentTarget as HTMLInputElement).checked)} /><span>Yes</span></label>{:else if item.field.type === "enum"}<select id={`create-${item.field.key}`} multiple={item.field.multiple ?? false} value={item.field.multiple ? (Array.isArray(createFieldValues[item.field.key]) ? createFieldValues[item.field.key] : []) : String(createFieldValues[item.field.key] ?? "")} onchange={(event) => updateCreateEnumField(item.field.key, event, item.field.multiple ?? false)}><option value="">Choose {item.field.label.toLowerCase()}</option>{#each item.field.options ?? [] as option}<option value={option}>{option}</option>{/each}</select>{:else if item.field.type === "entity-ref"}<select id={`create-${item.field.key}`} value={String(createFieldValues[item.field.key] ?? "")} onchange={(event) => setCreateField(item.field.key, (event.currentTarget as HTMLSelectElement).value)}><option value="">Choose an entity</option>{#each entities.filter((entity) => !entity.deleted) as entity}<option value={entity.id}>{entity.name} · {entity.entity_type ?? "Uncategorized"}</option>{/each}</select>{:else if item.field.type === "date"}{#if createDateForField(item.field.key) || createDateEditorOpen[item.field.key]}{@const date = createDateDraftForField(item.field.key) ?? { calendar: "gregorian", era: "CE", precision: "day" }}<div class="date-editor"><div class="date-fields"><label for={`create-${item.field.key}-year`}>Year<input id={`create-${item.field.key}-year`} aria-label={`${item.field.label} year`} type="number" min="1" value={date.year ?? ""} onchange={(event) => updateCreateDatePart(item.field.key, "year", (event.currentTarget as HTMLInputElement).value, 1)} /></label><label for={`create-${item.field.key}-month`}>Month<input id={`create-${item.field.key}-month`} aria-label={`${item.field.label} month`} type="number" min="1" max="12" value={date.month ?? ""} onchange={(event) => updateCreateDatePart(item.field.key, "month", (event.currentTarget as HTMLInputElement).value, 1, 12)} /></label><label for={`create-${item.field.key}-day`}>Day<input id={`create-${item.field.key}-day`} aria-label={`${item.field.label} day`} type="number" min="1" max="31" value={date.day ?? ""} onchange={(event) => updateCreateDatePart(item.field.key, "day", (event.currentTarget as HTMLInputElement).value, 1, 31)} /></label></div><small class="date-preview">{typeof date.year === "number" ? formatCalendarDate(date) : "Add a date"}</small><button class="date-clear" type="button" onclick={() => clearCreateDateField(item.field.key)}>Clear date</button></div>{:else}<button class="date-empty" type="button" onclick={() => openCreateDateEditor(item.field.key)}>Add a date</button>{/if}{/if}</div>{/each}{#if createOption.template.document}<label class="create-input-field" for="create-document"><span>Opening note</span><textarea id="create-document" rows="5" bind:value={createDocumentBody} placeholder="Add a first note or leave the template text as-is"></textarea></label>{/if}{:else}<div class="create-form-empty">Select a template to begin.</div>{/if}</section></div><div class="create-dialog-actions"><button type="button" class="quiet-button" onclick={closeCreateForm}>Cancel</button><button class="primary-button" type="submit" disabled={!name.trim() || !createOption}>Create {createOption?.template.name ?? "entry"}</button></div></form></div>{/if}
    {#if showDiscardPrompt}<div class="discard-backdrop"><div class="discard-dialog" role="alertdialog" aria-modal="true" aria-labelledby="discard-create-title"><span class="panel-kicker">UNSAVED VALUES</span><h2 id="discard-create-title">Discard this creation?</h2><p>Your entered values will be cleared. You can keep editing or start over with the new template.</p><div class="discard-actions"><button type="button" class="quiet-button" onclick={keepCreateEditing}>Keep editing</button><button type="button" class="primary-button" onclick={discardCreateValues}>Discard values</button></div></div></div>{/if}
    {#if upgradePreview}
      {@const preview = upgradePreview}
      <div class="modal-backdrop"><div class="dialog upgrade-dialog" role="dialog" aria-modal="true">
        <div class="new-form-heading"><div><span class="panel-kicker">UPDATE PLUGIN</span><strong>Update {preview.entry.name} to v{preview.version}</strong></div><button type="button" class="new-form-close" onclick={() => upgradePreview = null}>×</button></div>
        <p class="dialog-body-copy">From <code>v{preview.plan.fromVersion ?? preview.entry.version}</code> to <code>v{preview.plan.toVersion}</code>{preview.plan.target.signed ? ", signed by" : ", unsigned — "}{preview.plan.target.publisher}.</p>
        {#if preview.plan.consent.requiresRenewal}
          <p class="plugin-warning">This update requests new capabilities. Your consent is required before they are granted.</p>
        {/if}
        {#if preview.plan.consent.added.length > 0}
          <h4 class="plugin-subhead">New capabilities</h4>
          <ul class="plugin-detail-list">{#each preview.plan.consent.added as capability}<li class="provides">{capabilityLabel(capability)}</li>{/each}</ul>
        {/if}
        {#if preview.plan.consent.removed.length > 0}
          <h4 class="plugin-subhead">Removed capabilities</h4>
          <ul class="plugin-detail-list">{#each preview.plan.consent.removed as capability}<li class="consumes">{capabilityLabel(capability)}</li>{/each}</ul>
        {/if}
        <h4 class="plugin-subhead">Data migrations</h4>
        {#if preview.plan.migrations.migrationIds.length > 0}
          <p class="dialog-body-copy">Data will migrate from <code>v{preview.plan.migrations.from}</code> to <code>v{preview.plan.migrations.to}</code>{preview.plan.migrations.requiresBackup ? ". A backup is created before migrating." : "."}</p>
          <ul class="plugin-detail-list">{#each preview.plan.migrations.migrationIds as id}<li>{id}</li>{/each}</ul>
        {:else}
          <p class="dialog-body-copy">No data migrations required.</p>
        {/if}
        <div class="new-form-actions"><button type="button" class="quiet-button" onclick={() => upgradePreview = null}>Cancel</button><button type="button" class="primary-button" onclick={confirmUpgrade} disabled={upgradeBusy}>{upgradeBusy ? "Updating…" : "Confirm update"}</button></div>
      </div></div>
    {/if}
    {#if confirmAction}
      <div class="modal-backdrop plugin-confirm-modal"><div class="dialog" role="alertdialog" aria-modal="true">
        <div class="new-form-heading"><div><span class="panel-kicker">CONFIRM ACTION</span><strong>{confirmAction.title}</strong></div><button type="button" class="new-form-close" onclick={() => confirmAction = null}>×</button></div>
        <p class="dialog-body-copy">{confirmAction.message}</p>
        {#if confirmAction.capabilities}
          {#if confirmAction.capabilities.length > 0}
            <div class="capability-list" role="list" aria-label="Requested capabilities">
              {#each confirmAction.capabilities as capability}
                <div class="capability-item" role="listitem">{capabilityLabel(capability)}</div>
              {/each}
            </div>
          {:else}
            <p class="dialog-body-copy capability-empty">No capabilities requested.</p>
          {/if}
        {/if}
        <div class="new-form-actions"><button type="button" class="quiet-button" onclick={() => confirmAction = null}>Cancel</button><button type="button" class="primary-button" onclick={runConfirm} disabled={confirmBusy}>{confirmBusy ? "Working…" : confirmAction.confirmLabel}</button></div>
      </div></div>
    {/if}
    {#if deleteTarget}
      <div class="modal-backdrop"><div class="dialog" role="alertdialog" aria-modal="true">
        <div class="new-form-heading"><div><span class="panel-kicker">DELETE PROJECT DATA</span><strong>Delete {deleteTarget.name} data?</strong></div><button type="button" class="new-form-close" onclick={() => deleteTarget = null}>×</button></div>
        <p class="dialog-body-copy">All entities, documents, fields, relationships, and assets owned by <code>{deleteTarget.id}</code> in this project will be deleted. A backup is kept on disk.</p>
        <p class="dialog-body-copy">Type <code>{deleteTarget.id}</code> to confirm.</p>
        <input aria-label="Delete confirmation" bind:value={deleteInput} placeholder={deleteTarget.id} />
        <div class="new-form-actions"><button type="button" class="quiet-button" onclick={() => deleteTarget = null}>Cancel</button><button type="button" class="primary-button danger-button" onclick={confirmDeleteData} disabled={deleteBusy || deleteInput.trim() !== deleteTarget.id}>{deleteBusy ? "Deleting…" : "Delete project data"}</button></div>
      </div></div>
    {/if}
    {#if deleteBackupPath}
      <div class="modal-backdrop"><div class="dialog" role="alertdialog" aria-modal="true">
        <div class="new-form-heading"><div><span class="panel-kicker">DATA DELETED</span><strong>Plugin data deleted</strong></div><button type="button" class="new-form-close" onclick={() => deleteBackupPath = ""}>×</button></div>
        <p class="dialog-body-copy">A backup was kept at:</p>
        <code class="backup-path">{deleteBackupPath}</code>
        <div class="new-form-actions"><button type="button" class="primary-button" onclick={() => deleteBackupPath = ""}>Done</button></div>
      </div></div>
    {/if}
    {#if showSettings}
      <SettingsView
        bind:section={settingsSection}
        recentProjects={recentProjects}
        projectOpen={ready}
        onRemoveRecent={removeRecentProject}
        onClose={closeSettings}
        aiSettings={aiSettings}
        aiStatus={aiStatus}
        aiModels={aiModels}
        aiModelsBusy={aiModelsBusy}
        aiModelsMessage={aiModelsMessage}
        aiIndexStatus={aiIndexStatus}
        aiIndexBusy={aiIndexBusy}
        aiIndexMessage={aiIndexMessage}
        onAiSettingsChange={updateAiSetting}
        onAiCheck={() => void checkAiProvider()}
        onAiModelsLoad={() => void loadAiModels()}
        onAiIndexRefresh={() => void refreshAiIndexStatus()}
        onAiIndexRebuild={() => void rebuildAiIndex()}
        onAiIndexCancel={() => void cancelAiIndex()}
        onAiRemoteSettingsChange={updateAiRemoteSetting}
        onAiRemotePolicyChange={updateAiRemotePolicy}
        remoteCredential={remoteCredential}
        onAiRemoteConsent={(allowed) => void setRemoteConsent(allowed)}
        onAiRemoteImport={() => void importRemoteCredential()}
        onPortableBackup={createPortableBackup}
        onRecoveryBackup={createRecoveryBackup}
        onRestoreRecoveryBackup={restoreRecoveryBackup}
      >
        {#snippet plugins()}
          <div class="settings-section-heading plugins-settings-heading">
            <strong>Plugins</strong>
            <p>Extensions that power this project. Every install, upgrade, and rollback is verified and reversible.</p>
          </div>
          <div class="plugins-toolbar">
            <button type="button" class="primary-button" disabled={installing || adminBusy} onclick={() => void installFromPicker()}>{installing ? "Installing…" : "Install package…"}</button>
            <span class="muted-note">{adminBusy ? "Refreshing…" : ""}</span>
          </div>
          {#if installSummary}
            <div class="plugins-note">Installed {installSummary.id} {installSummary.version}{installSummary.signed ? " (signed)" : " (unsigned)"}{installSummary.digest ? ` · ${shortDigest(installSummary.digest)}` : ""}.</div>
          {/if}
          <div class="plugins-list">
            {#if adminPlugins === null}
              <p class="search-state">Loading plugins…</p>
            {:else if adminPlugins.length === 0}
              <p class="search-state">No plugins installed. Install a .wbplugin package to get started.</p>
            {:else}
              {#each adminPlugins as plugin (plugin.id)}
                <article class="plugin-card">
                  <header class="plugin-card-head">
                    <div class="plugin-card-title"><strong>{plugin.name}</strong><span class="plugin-id">{plugin.id}</span></div>
                    <div class="plugin-badges">
                      <span class:badge-off={!plugin.enabled} class="plugin-badge">{plugin.enabled ? "Enabled" : "Disabled"}</span>
                      {#if plugin.stability === "beta"}<span class="plugin-badge beta" title="Beta release: this plugin is useful but may be unstable">Beta · unstable</span>{:else if plugin.stability === "experimental"}<span class="plugin-badge experimental" title="Experimental release: behavior may change">Experimental</span>{/if}
                      <span class="plugin-badge">{plugin.kind}</span>
                      <span class="plugin-badge" title={`Lifecycle: ${plugin.lifecycle.state}`}>{plugin.lifecycle.state}</span>
                      {#if plugin.lifecycle.failures > 0}<span class="plugin-badge danger" title={plugin.lifecycle.lastError ?? ""}>{plugin.lifecycle.failures} failure{plugin.lifecycle.failures === 1 ? "" : "s"}</span>{/if}
                    </div>
                  </header>
                  <div class="plugin-card-meta">
                    <span>v{plugin.selectedVersion ?? plugin.version} · {plugin.publisher}</span>
                    <span>host API {plugin.hostApi}</span>
                    <span>data v{plugin.dataVersion}</span>
                    <span class:runtime-off={!plugin.runtimeRunning} class="runtime-dot" title={plugin.runtimeRunning ? "Runtime active" : "Runtime stopped"}>{plugin.runtimeRunning ? "● running" : "○ stopped"}</span>
                  </div>
                  {#if !plugin.dependencyState.resolved}
                    <p class="plugin-warning">Dependency problem: {plugin.dependencyState.error ?? "could not resolve dependencies"}</p>
                  {:else if plugin.dependencyState.order.length > 0}
                    <p class="plugin-muted">Loads after: {plugin.dependencyState.order.join(" → ")}</p>
                  {/if}
                  {#if plugin.lifecycle.lastError}
                    <p class="plugin-error" title={plugin.lifecycle.lastError}>Last failure: {plugin.lifecycle.lastError}</p>
                  {/if}
                  <div class="plugin-actions">
                    <button type="button" class:on={plugin.enabled} class="plugin-toggle" onclick={() => void togglePluginEnabled(plugin)} disabled={adminBusy || pluginActionId !== null}>{pluginActionId === plugin.id ? "Working…" : plugin.enabled ? "Disable" : "Enable"}</button>
                    {#if plugin.enabled}<button type="button" class="quiet-button" onclick={() => void reviewPluginCapabilities(plugin)} disabled={adminBusy || pluginActionId !== null}>Review capabilities</button>{/if}
                    {#if selectedUninstallableVersion(plugin)}
                      <button
                        class="quiet-button"
                        onclick={() => confirmUninstall(plugin, selectedUninstallableVersion(plugin)!.version)}
                        disabled={adminBusy || plugin.enabled}
                        title={plugin.enabled ? "Disable the plugin before uninstalling its selected code." : "Remove the selected plugin code while preserving project data."}
                      >{plugin.enabled ? "Disable to uninstall" : "Uninstall code"}</button>
                    {/if}
                    {#if plugin.lifecycle.state === "quarantined"}<button class="quiet-button" onclick={() => retryPlugin(plugin)} disabled={adminBusy}>Retry</button>{/if}
                    <button class="quiet-button" onclick={() => openDeleteData(plugin)} disabled={adminBusy}>Delete project data…</button>
                  </div>
                  {#if plugin.installedVersions.length > 0}
                    <div class="version-list">
                      {#each plugin.installedVersions as version (version.version)}
                        <div class="version-row">
                          <div class="version-copy">
                            <span class="version-name">
                              v{version.version}
                              {#if version.isActiveCandidate}<span class="version-tag latest">Latest</span>{/if}
                              {#if version.isSelected}<span class="version-tag selected">Selected</span>{/if}
                              {#if version.bundled}<span class="version-tag bundled">Bundled</span>{/if}
                              {#if version.signed}<span class="version-tag signed">Signed</span>{:else if !version.bundled}<span class="version-tag unsigned">Unsigned</span>{/if}
                            </span>
                            <span class="version-detail">{version.publisher}{version.installedAt ? ` · installed ${installedAtLabel(version.installedAt)}` : ""}{version.digest ? ` · ${shortDigest(version.digest)}` : ""}</span>
                          </div>
                          <div class="version-actions">
                            {#if version.isActiveCandidate && !version.isSelected && !version.bundled}
                              <button class="quiet-button" onclick={() => previewUpgrade(plugin, version.version)} disabled={adminBusy}>Update to v{version.version}</button>
                            {/if}
                            {#if version.rollbackAvailable}
                              <button class="quiet-button" onclick={() => confirmRollback(plugin, version.version)} disabled={adminBusy}>Rollback</button>
                            {/if}
                            {#if !version.isSelected && !version.bundled}
                              <button class="quiet-button" onclick={() => confirmUninstall(plugin, version.version)} disabled={adminBusy}>Uninstall code</button>
                            {/if}
                          </div>
                        </div>
                      {/each}
                    </div>
                  {/if}
                  <details class="plugin-details">
                    <summary>Capabilities, namespaces, services &amp; migrations</summary>
                    <div class="plugin-details-grid">
                      <section class="plugin-detail-section">
                        <h4>Capabilities</h4>
                        <ul class="plugin-detail-list">
                          {#each plugin.capabilities as capability}
                            {@const granted = plugin.grantedCapabilities.includes(capability)}
                            <li class:granted={granted}>{capabilityLabel(capability)} <span class="cap-mark">{granted ? "✓" : "—"}</span></li>
                          {/each}
                        </ul>
                      </section>
                      <section class="plugin-detail-section">
                        <h4>Namespaces</h4>
                        <ul class="plugin-detail-list">{#each plugin.namespaces as namespace}<li>{namespace}</li>{/each}</ul>
                      </section>
                      <section class="plugin-detail-section">
                        <h4>Services</h4>
                        <ul class="plugin-detail-list">
                          {#each plugin.services.provides as service}<li class="provides">provides <code>{service.name}@{service.major}</code></li>{/each}
                          {#each plugin.services.consumes as service}<li class="consumes">consumes <code>{service.name}@{service.major}</code></li>{/each}
                          {#if plugin.services.provides.length === 0 && plugin.services.consumes.length === 0}<li class="muted-item">none</li>{/if}
                        </ul>
                      </section>
                      <section class="plugin-detail-section">
                        <h4>Events</h4>
                        <ul class="plugin-detail-list">
                          {#each plugin.events.publishes as event}<li class="provides">publishes <code>{event.name}@{event.version}</code></li>{/each}
                          {#each plugin.events.subscribes as event}<li class="consumes">subscribes <code>{event.name}@{event.version}</code></li>{/each}
                          {#if plugin.events.publishes.length === 0 && plugin.events.subscribes.length === 0}<li class="muted-item">none</li>{/if}
                        </ul>
                      </section>
                      <section class="plugin-detail-section">
                        <h4>Migrations</h4>
                        <ul class="plugin-detail-list">
                          {#each plugin.migrations as migration}<li class="migration"><code>{migration.from} → {migration.to}</code>{migration.recovery === "backup" ? " · backs up" : ""}</li>{/each}
                          {#if plugin.migrations.length === 0}<li class="muted-item">no data migrations</li>{/if}
                        </ul>
                      </section>
                    </div>
                  </details>
                </article>
              {/each}
            {/if}
          </div>
        {/snippet}
        {#snippet git()}
          <GitSettingsPanel
            projectOpen={ready}
            onError={(message) => error = message}
            beforeWrite={flushAutoSave}
          />
        {/snippet}
      </SettingsView>
    {:else if !ready}
      <section class="welcome"><div class="welcome-copy"><span class="overline">A private place for impossible worlds</span><h1>Build the world<br /><em>behind the story.</em></h1><p>Shape characters, places, factions, and history in one calm, local-first studio.</p></div><div class="welcome-art"><div class="orb orb-one"></div><div class="orb orb-two"></div><div class="art-card"><span>ELDERMERE</span><strong>The sea remembers<br />what kingdoms forget.</strong><small>Fragments · 12</small></div></div></section>
    {:else if projectionView}
      {#key projectionView.title}
        <ProjectionView title={projectionView.title} view={projectionView.module.views[0]} context={buildModuleContext(projectionView.module.manifest, projectInfo?.root ?? "")} onClose={() => projectionView = null} />
      {/key}
    {:else if hostView}
      <div class="host-view-shell"><button class="quiet-button host-view-back" onclick={() => hostView = null}>Back to workspace</button><HostView plugin={hostView.plugin} view={hostView.view} /></div>
    {:else if sandboxView}
      {#key `${sandboxView.plugin.id}:${sandboxView.view?.id ?? "default"}:${sandboxView.plugin.id === "daena.maps" && selected?.entity_type === "daena.maps:map" ? selected.id : ""}:${sandboxView.plugin.id === "daena.maps" ? mapReloadCounter : 0}`}
        {#if sandboxView.plugin.id === "daena.maps" && selected?.entity_type === "daena.maps:map"}
          {@const mapId = selected.id}
          {@const mapState = mapSaveStates[mapId] ?? null}
          {@const mapDetail = mapConflictDetail(mapState?.detail)}
          <div class="map-editor-shell">
            <div class="map-editor-toolbar">
              <span class:map-state-warn={mapState?.status === "dirty" || mapState?.status === "error"} class:map-state-error={mapState?.status === "conflict"} class="map-save-chip"><span class="map-save-dot"></span>{mapSaveLabel(mapState)}</span>
              {#if mapState?.status === "conflict"}<span class="map-conflict-path">{mapDetail.path ?? "Draft exported as a recovery copy."}</span>{/if}
              {#if mapReconcileNotice}<span class="map-reconcile-notice">{mapReconcileNotice}</span>{/if}
              <div class="map-toolbar-actions">
                <button class="quiet-button" type="button" onclick={requestMapCapture}>Link selection</button>
                <button class="quiet-button" type="button" disabled={mapState?.status !== "dirty" && mapState?.status !== "error"} onclick={saveCurrentMap}>Save</button>
              </div>
            </div>
            {#if mapState?.status === "conflict"}
              <div class="map-conflict-banner" role="alert">
                <div class="map-conflict-copy"><strong>This map changed on disk while you were editing</strong><p>Your draft was not saved over it. A recovery copy was exported so nothing is lost.</p>{#if mapDetail.path}<code>{mapDetail.path}</code>{/if}</div>
                <div class="map-conflict-actions"><button class="primary-button" type="button" disabled={mapRecoveryBusy} onclick={restoreMapDraft}>{mapRecoveryBusy ? "Restoring…" : "Restore draft"}</button><button class="quiet-button" type="button" onclick={reloadMapOriginal}>Reload original</button><button class="quiet-button" type="button" onclick={dismissMapConflict}>Keep editing</button></div>
              </div>
            {/if}
            <SandboxView pluginId={sandboxView.plugin.id} viewId={sandboxView.view?.id} title={sandboxView.plugin.name} mapEntityId={mapId} linkId={mapFocusLinkId ?? undefined} />
          </div>
        {:else if sandboxView.plugin.id === "daena.maps" && sandboxView.view == null}
          {@const mapsView = pluginViews().find((item) => item.plugin.id === "daena.maps")}
          <section class="welcome maps-welcome">
            <div class="welcome-copy">
              <span class="overline">MAPS</span>
              <h1>Draw the world<br /><em>behind the story.</em></h1>
              <p>Shape continents, kingdoms, and borders in the built-in generator. Create a new map or open one you have saved.</p>
              <div class="maps-welcome-actions">
                <button class="primary-button" type="button" onclick={() => void createMap()}>Create Map</button>
              </div>
            </div>
            <div class="maps-welcome-list panel-surface">
              <div class="panel-heading"><div><span class="panel-kicker">SAVED MAPS</span><strong>{savedMaps().length} saved</strong></div></div>
              {#if savedMaps().length === 0}
                <div class="list-empty" role="status"><span class="empty-mark" aria-hidden="true">◇</span><strong>No saved maps yet.</strong><p>Create a map and press Save to keep your first world.</p></div>
              {:else}
                <div class="collection-list">
                  {#each savedMaps() as map (map.id)}
                    <button class="collection-item" type="button" onclick={async () => {
                      selected = map;
                      await loadSelectedState(map);
                      if (mapsView) await openPluginView(mapsView);
                    }}><span class="item-copy"><strong>{map.name}</strong><small>Updated {new Date(map.updated_at).toLocaleString()}</small></span><span class="item-arrow" aria-hidden="true">›</span></button>
                  {/each}
                </div>
              {/if}
            </div>
          </section>
        {:else}
          <SandboxView pluginId={sandboxView.plugin.id} viewId={sandboxView.view?.id} title={sandboxView.plugin.name} mapEntityId={sandboxView.plugin.id === "daena.maps" && selected?.entity_type === "daena.maps:map" ? selected.id : undefined} />
        {/if}
      {/key}
    {:else if enabledWorkspaceSections().length === 0}
      <section class="empty-workspace-state"><div class="disabled-icon">◌</div><span class="overline">WORKSPACE READY</span><h1>Choose a workspace to begin.</h1><p>No workspace modules are enabled in this project. Enable one from Settings → Plugins to start working.</p><button class="primary-button" onclick={() => void openSettings("plugins")}>Open Plugins</button></section>
    {:else}
      {#if projectDiagnostics.length}<div class="project-diagnostics" role="alert"><span>{projectDiagnostics[0]}</span><button class="quiet-button" onclick={() => void importPortableCheckpoint()}>Import checkpoint</button></div>{/if}
      <div class="workspace-heading"><div><span class="overline">{section === "lore" ? "WORLD BIBLE" : section === "timeline" ? "CHRONOLOGY" : "DRAFTING DESK"}</span><h1>{sectionLabel()}</h1><p>{section === "lore" ? "A living reference for every person, place, and power." : section === "timeline" ? "Events, eras, and the threads that connect them." : writingView === "manuscripts" ? "Draft stories, essays, and other long-form work." : "Build the pages, notes, and references behind the story."}</p></div><div class="heading-actions">{#if section !== "writing"}<button class="quiet-button" onclick={openProjection}>Open {section === "lore" ? "graph" : "timeline"} ↗</button>{/if}</div></div>
      <section class="workspace-grid">
        <aside class="collection-panel panel-surface">
          <div class="panel-heading">
            <div><span class="panel-kicker">{section === "lore" ? "LORE LIBRARY" : section === "timeline" ? "TIMELINE" : writingView === "manuscripts" ? "MANUSCRIPTS" : "REFERENCE PAGES"}</span><strong>{visibleEntities().length} {collectionLabel()}</strong></div>
          </div>
          {#if section === "writing"}<div class="collection-tabs" role="tablist" aria-label="Writing collections"><button role="tab" aria-selected={writingView === "manuscripts"} class:active={writingView === "manuscripts"} onclick={() => switchWritingView("manuscripts")}>Manuscripts</button><button role="tab" aria-selected={writingView === "reference"} class:active={writingView === "reference"} onclick={() => switchWritingView("reference")}>Reference pages</button></div>{/if}
          <div class="collection-search"><span>⌕</span><input aria-label={`Filter ${collectionLabel()}`} bind:value={query} placeholder={`Filter ${collectionLabel()}`} /></div>
          <div class="collection-list">
            {#if visibleEntities().length === 0}<div class="list-empty" role="status"><span class="empty-mark" aria-hidden="true">✦</span><strong>{query ? `No ${collectionLabel()} match that filter.` : `No ${collectionLabel()} yet.`}</strong><p>{query ? "Try another filter or create something new." : `Create your first ${createLabel()} to begin building this collection.`}</p><button class="empty-create" type="button" onclick={toggleCreateForm}>＋ Create {createLabel()}</button></div>{:else}{#each visibleEntities() as entity}<button class:selected={selected?.id === entity.id} class="collection-item" onclick={() => selectEntity(entity)}><span class={`entity-glyph ${entityGlyphClass(entity)}`}>{entityGlyph(entity)}</span><span class="item-copy"><strong>{entity.name}</strong><small>{entityTypeLabel(entity.entity_type)}</small></span><span class="item-arrow" aria-hidden="true">›</span></button>{/each}{/if}
          </div>
        </aside>

        <article class:editor-fullscreen={editorFullscreen} class="editor-panel">
          <div class="editor-header">
            <div>
              <span class="panel-kicker">{selected ? entityTypeLabel(selected.entity_type).toUpperCase() : section === "lore" ? "LORE ENTRY" : section === "timeline" ? "TIMELINE EVENT" : writingView === "manuscripts" ? "MANUSCRIPT" : "REFERENCE PAGE"}</span>
              <h2>{selected?.name ?? "Choose an entry"}</h2>
            </div>
            {#if selected}
              <div class="editor-status">
                {#if isSaving}<span class="saving-dot"></span> Saving…{:else if hasUnsavedChanges}<span class="unsaved-dot"></span> Unsaved changes{:else if savedAt}<span class="saved-dot">✓</span> Saved {savedAt}{/if}
              </div>
            {/if}
          </div>
          {#if selected}
            {#if documentConflict}
              <div class="document-conflict" role="alert">
                <strong>{documentConflict.diagnostics.length ? "Canonical source needs attention" : "This draft changed on disk"}</strong>
                <p>{documentConflict.diagnostics.length ? documentConflict.diagnostics[0] : "Your unsaved draft is preserved. Choose how to reconcile it before saving."}</p>
                {#if !documentConflict.diagnostics.length}<details class="conflict-compare"><summary>Compare with disk</summary><pre>{conflictDiskBody}</pre></details>{/if}
                <div class="conflict-actions"><button class="quiet-button" type="button" onclick={reloadConflict}>Reload disk</button><button class="quiet-button" type="button" onclick={overwriteConflict} disabled={documentConflict.diagnostics.length > 0}>Overwrite as new revision</button><button class="quiet-button" type="button" onclick={saveConflictRecoveryCopy}>Save recovery copy</button></div>
              </div>
            {/if}
            {#if aiRewriteOpen}
              <section class="ai-rewrite-panel" aria-label="AI proposal">
                <div class="ai-rewrite-heading"><div><span class="panel-kicker">{aiUseRemote ? "REMOTE AI · APPROVED PROVIDER" : "LOCAL AI · LM STUDIO"}</span><strong>{aiBusy ? aiMode === "generate" ? "Generating text…" : "Rewriting selection…" : aiPreviewOutput ? aiMode === "generate" ? "Review generated text" : "Review rewrite" : aiMode === "generate" ? "Generate text" : "Rewrite selection"}</strong></div><button class="quiet-button" type="button" onclick={closeAiRewrite}>Discard</button></div>
                {#if !aiPreviewOutput && aiSettings.remote.provider && aiSettings.remote.endpoint}<label class="ai-remote-choice"><input type="checkbox" bind:checked={aiUseRemote} disabled={aiBusy} /> Use the approved remote provider for this request</label>{/if}
                {#if !aiPreviewOutput}<label class="ai-instruction">Instruction<textarea rows="2" bind:value={aiInstruction} disabled={aiBusy} placeholder="Tell LM Studio how to rewrite the selection"></textarea></label>{/if}
                <AiProposalPreview
                  original={aiSourceSelectionPlain}
                  bind:proposal={aiPreviewOutput}
                  streamText={aiStreamText}
                  busy={aiBusy}
                  onCancel={() => aiBusy && aiRequestId ? void project.aiCancelText(aiRequestId) : closeAiRewrite()}
                  onDiscard={closeAiRewrite}
                  onAccept={() => void acceptAiRewrite()}
                />
                {#if aiUsage}<p class="muted-note">Provider usage: {aiUsage.inputTokens} input + {aiUsage.outputTokens} output tokens.</p>{/if}
                {#if !aiBusy && !aiPreviewOutput}<div class="ai-rewrite-actions"><button class="primary-button" type="button" disabled={(aiMode === "rewrite" && !aiSourceSelection.trim()) || !aiInstruction.trim()} onclick={() => void startAiRewrite()}>{aiMode === "generate" ? "Generate text" : "Generate rewrite"}</button></div>{/if}
              </section>
            {/if}
            <RichTextEditor bind:this={editorRef} value={documentBody} editable={projectDiagnostics.length === 0 && !aiBusy} fullscreen={editorFullscreen} onChange={updateDocumentBody} onSelectionChange={setAiSelection} onAiRequest={openAiAction} onFullscreenChange={setEditorFullscreen} placeholder={section === "writing" ? writingView === "manuscripts" ? "Write your manuscript…" : "Write this reference page…" : "Write the canonical story of this entry…"} />
            <div class="editor-footer">
              <span>{wordCount()} words</span>
              <div><button class="quiet-button" onclick={archiveSelected}>Archive</button></div>
            </div>
          {:else}
            <div class="editor-empty"><div class="empty-mark">✦</div><h3>{section === "writing" ? writingView === "manuscripts" ? "Your draft is waiting." : "Your reference desk is waiting." : "Your canvas is waiting."}</h3><p>Select an entry from the library, or create something new to begin writing.</p></div>
          {/if}
        </article>

        {#if selected}<aside class="inspector-panel panel-surface"><div class="inspector-heading"><div><span class="panel-kicker">INSPECTOR</span><strong>Details</strong></div><div class="inspector-heading-actions"><span class="inspector-type">{selected.entity_type}</span>{#if emptyInspectorDefinitions().length}<button class="inspector-ai-action" type="button" onclick={() => void fillAiFields()} disabled={aiFieldFillBusy}><span aria-hidden="true">✦</span>{aiFieldFillBusy ? "Finding…" : "Fill with AI"}</button>{/if}</div></div>{#if aiFieldFillOpen}<section class="inspector-ai-fill"><div class="inspector-ai-fill-heading"><strong>{aiFieldFillBusy ? "Finding field suggestions…" : "Review field suggestions"}</strong><button class="quiet-button" type="button" onclick={closeAiFieldFill}>Close</button></div>{#if aiFieldFillBusy}<p>Using this entry and related project context.</p>{:else if Object.keys(aiFieldSuggestions).length === 0}<p>No suggestions are available.</p>{:else}{#each Object.entries(aiFieldSuggestions) as [key, suggestion]}{@const definition = definitions().find((candidate) => candidate.key === key)}<div class="inspector-ai-suggestion"><div><strong>{definition?.label ?? key}</strong><span>{suggestionDisplayValue(key, suggestion)}</span><small>{suggestion.rationale} · {suggestion.confidence} confidence</small></div><div><button class="quiet-button" type="button" onclick={() => acceptAiFieldSuggestion(key)}>Accept</button><button class="quiet-button" type="button" onclick={() => discardAiFieldSuggestion(key)}>Discard</button></div></div>{/each}{/if}</section>{/if}<section class="inspector-section"><h3>Properties</h3>{#each definitions().filter((candidate) => candidate.type !== "relationship") as definition}<div class="property-field"><span>{definition.label}{#if definition.required}<b>*</b>{/if}</span>{#if definition.type === "date"}{#if dateForField(definition.key) || dateEditorOpen[definition.key]}{@const date = dateDraftForField(definition.key) ?? { calendar: "gregorian", era: "CE", precision: "day" }}<div class="date-editor"><div class="date-fields"><label for={`${definition.key}-year`}>Year<input id={`${definition.key}-year`} aria-label={`${definition.label} year`} type="number" min="1" value={date.year ?? ""} onchange={(event) => updateDatePart(definition.key, "year", (event.currentTarget as HTMLInputElement).value, 1)} /></label><label for={`${definition.key}-month`}>Month<input id={`${definition.key}-month`} aria-label={`${definition.label} month`} type="number" min="1" max="12" value={date.month ?? ""} onchange={(event) => updateDatePart(definition.key, "month", (event.currentTarget as HTMLInputElement).value, 1, 12)} /></label><label for={`${definition.key}-day`}>Day<input id={`${definition.key}-day`} aria-label={`${definition.label} day`} type="number" min="1" max="31" value={date.day ?? ""} onchange={(event) => updateDatePart(definition.key, "day", (event.currentTarget as HTMLInputElement).value, 1, 31)} /></label></div><small class="date-preview">{typeof date.year === "number" ? formatCalendarDate(date) : "Add a date"}</small><button class="date-clear" type="button" onclick={() => clearDateField(definition.key)}>Clear date</button></div>{:else}<button class="date-empty" type="button" onclick={() => openDateEditor(definition.key)}>Add a date</button>{/if}{:else if definition.type === "enum" && definition.options?.length}<select aria-label={definition.label} multiple={definition.multiple ?? false} value={definition.multiple ? (Array.isArray(fields[definition.key]) ? fields[definition.key] : []) : String(fields[definition.key] ?? "")} onchange={(event) => updateField(definition.key, event)}>{#each definition.options ?? [] as option}<option value={option}>{option}</option>{/each}</select>{:else}<input type="text" value={fieldDisplayValue(fields[definition.key])} placeholder="Add {definition.label.toLowerCase()}" oninput={(event) => updateField(definition.key, event)} />{/if}</div>{/each}</section>{#each definitions().filter((candidate) => candidate.type === "relationship") as definition}<section class="inspector-section"><div class="section-title"><h3>{definition.label}</h3><span>{selectedRelationshipIds(definition).length}</span></div><RelationshipPicker field={definition} entities={entities} selectedIds={selectedRelationshipIds(definition)} onChange={(ids) => void updateRelationshipField(definition, ids)} /></section>{/each}<section class="inspector-section"><div class="section-title"><h3>Attachments</h3><span>{assets.length}</span></div><button class="drop-zone" type="button" onclick={attachAsset}><span>＋</span><strong>Attach a file</strong><small>Copied into this project</small></button>{#each assets as asset}<div class="asset-row"><span class="asset-icon">□</span><span><strong>{asset.filename}</strong><small>{Math.max(1, Math.round(asset.size / 1024))} KB</small></span></div>{/each}</section><section class="inspector-section map-contribution" aria-label="Maps contribution"><div class="section-title"><h3>Maps</h3><span>{mapLocations.length}</span></div><p>Shared locations remain on the entity even when Maps is disabled.</p><button class="quiet-button" type="button" onclick={() => void linkEntityToMap()}>＋ Link location</button>{#if mapLocations.length === 0}<small>No map links yet.</small>{:else}{#each mapLocations as location (location.id)}<div class="map-location-row"><div><strong>{location.label || location.role}</strong><small>{location.role} · {location.mapEntityId.slice(0, 8)}{#if location.resolution === "unresolved"} · <span class="map-unresolved-badge">Unresolved</span>{/if}</small></div><div>{#if location.resolution === "unresolved"}<span class="map-unresolved-note" title="The map feature this link pointed to was removed or renumbered.">Feature missing</span>{:else}<button class="quiet-button" type="button" onclick={() => void openMapLocation(location)}>Show on map</button>{/if}<button class="quiet-button" type="button" onclick={() => void editMapLocation(location)}>Edit</button><button class="quiet-button" type="button" onclick={() => void rebindMapLocation(location)}>Rebind</button><button class="quiet-button" type="button" onclick={() => void unlinkMapLocation(location)}>Unlink</button></div></div>{/each}{/if}</section></aside>{:else}<aside class="inspector-panel panel-surface inspector-empty"><span>INSPECTOR</span><p>Select an entry to see its properties, relationships, and attachments.</p></aside>{/if}
      </section>
    {/if}
    {#if error}<div class="toast" role="alert" aria-live="assertive">{error}<button aria-label="Dismiss" onclick={() => error = ""}>×</button></div>{/if}
  </section>
  {#if ready}<button class="mobile-create-button" aria-label="New entry" aria-expanded={showCreateForm} onclick={toggleCreateForm}>＋</button>{/if}
</main>

<style>
  :global(*) { box-sizing: border-box; }
  :global(:root) { --ink: #25251f; --ink-soft: #77766d; --ink-faint: #aaa79d; --line: #e4e1d8; --surface: #fffefa; --surface-muted: #f4f2ec; --canvas: #f7f6f2; --accent: #b4773f; --accent-dark: #365342; --shadow-sm: 0 2px 8px rgba(38, 42, 33, .05); --shadow-lg: 0 18px 50px rgba(38, 42, 33, .08); --font-display: Georgia, serif; }
  :global(body) { margin: 0; background: var(--canvas); color: var(--ink); font-family: Inter, ui-sans-serif, system-ui, sans-serif; }
  :global(button), :global(input), :global(select) { font: inherit; }
  .studio-shell { min-height: 100vh; display: flex; } .rail { width: 248px; flex: 0 0 248px; display: flex; flex-direction: column; padding: 25px 15px 18px; background: #283a30; color: #eef0e9; } .startup-rail { padding-top: 34px; } .brand { display: flex; align-items: center; gap: 11px; padding: 0 10px 40px; } .rail:not(.startup-rail) .brand { padding-bottom: 20px; } .brand-mark { display: grid; place-items: start center; width: 31px; height: 31px; overflow: hidden; border-radius: 9px; background: #d5ab6c; } .project-card strong, .recent-project strong, .recent-project small { display: block; } .recent-project small { margin-top: 3px; color: #aab9ad; font-size: 11px; } .rail-label { margin: 0 10px 9px; color: #819688; font-size: 10px; font-weight: 700; letter-spacing: .16em; } .recent-label { margin-top: 27px; } .rail-button { width: 100%; display: flex; align-items: center; gap: 11px; padding: 10px 11px; margin-bottom: 3px; border: 0; border-radius: 8px; background: transparent; color: #b9c8bc; text-align: left; cursor: pointer; } .rail-button:hover, .rail-button.active { background: #3b5243; color: #fff; } .startup-primary { margin-top: 8px; background: #d5ab6c; color: #2c4032; font-weight: 700; } .startup-primary:hover { background: #e1bc82; color: #2c4032; } .rail-icon { width: 18px; color: #d5ab6c; text-align: center; } .startup-primary .rail-icon { color: #2c4032; } .muted-button { color: #91a397; } .rail-spacer { flex: 1; } .rail-footer { padding: 17px 10px 0; color: #708476; font-size: 11px; } .project-switcher { margin-bottom: 18px; } .project-card { display: flex; align-items: center; width: 100%; gap: 10px; padding: 10px; border: 0; border-radius: 8px; background: transparent; color: #eef0e9; font: inherit; text-align: left; cursor: pointer; } .project-card:hover, .project-card.active { background: #3b5243; } .project-copy { min-width: 0; flex: 1; } .project-chevron { flex: 0 0 auto; color: #aab9ad; font-size: 16px; line-height: 1; transform: translateY(-3px); } .project-card strong, .recent-project strong { font-size: 13px; max-width: 185px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; } .project-dot { flex: 0 0 auto; width: 8px; height: 8px; border-radius: 50%; background: #777f78; } .project-dot.online { background: #88c18e; box-shadow: 0 0 0 4px rgba(136,193,142,.12); } .recent-projects { display: grid; gap: 3px; } .recent-project { width: 100%; display: flex; align-items: flex-start; gap: 10px; padding: 10px; border: 0; border-radius: 8px; background: transparent; color: #eef0e9; text-align: left; cursor: pointer; } .recent-project:hover { background: #3b5243; } .recent-project small { overflow: hidden; max-width: 180px; text-overflow: ellipsis; white-space: nowrap; } .project-menu { margin: 3px 0 8px 8px; padding-left: 8px; border-left: 1px solid #486052; } .project-menu .rail-button { padding: 8px 9px; color: #aab9ad; font-size: 11px; } .module-menu { margin: 6px 8px 12px; padding: 8px 10px; border: 1px solid #486052; border-radius: 8px; background: #30483a; }
  .app-main { min-width: 0; flex: 1; } .app-main.sandbox-active { display: flex; min-height: 0; flex-direction: column; overflow: hidden; } .app-main.sandbox-active > .topbar { flex: 0 0 auto; } .topbar { display: flex; align-items: center; justify-content: space-between; min-height: 58px; padding: 0 40px 0; border-bottom: 1px solid var(--line); background: rgba(255,254,250,.78); } .breadcrumbs, .top-actions { display: flex; align-items: center; gap: 10px; } .breadcrumbs { min-width: 0; color: var(--ink-faint); font-size: 12px; } .breadcrumbs strong { color: var(--ink-soft); } .breadcrumbs span:last-child { overflow: hidden; max-width: 180px; text-overflow: ellipsis; white-space: nowrap; } .breadcrumbs i { color: #d0ccc2; font-style: normal; } .global-search { display: flex; align-items: center; gap: 8px; width: 230px; padding: 8px 10px; border: 1px solid var(--line); border-radius: 8px; background: var(--surface); color: var(--ink-faint); } .global-search input { min-width: 0; flex: 1; border: 0; outline: 0; background: transparent; color: var(--ink); font-size: 12px; } .sync-badge { color: var(--ink-faint); font-size: 10px; } .sync-badge { display: flex; align-items: center; gap: 6px; color: var(--ink-soft); } .sync-badge span { width: 6px; height: 6px; border-radius: 50%; background: #72a97a; }
  .welcome, .disabled-state { max-width: 1080px; min-height: calc(100vh - 58px); margin: auto; padding: 10vh 7vw; display: flex; align-items: center; gap: 8vw; } .welcome-copy { flex: 1; } .overline, .panel-kicker { display: block; color: var(--accent); font-size: 10px; font-weight: 800; letter-spacing: .18em; } .welcome h1 { margin: 20px 0 18px; font: 500 clamp(48px, 6vw, 78px)/.98 var(--font-display); letter-spacing: -.04em; } .welcome h1 em { color: var(--accent); font-style: italic; } .welcome p { max-width: 380px; margin: 0; color: var(--ink-soft); font-size: 16px; line-height: 1.7; } .welcome-art { position: relative; width: 360px; height: 390px; } .orb { position: absolute; border-radius: 50%; } .orb-one { top: 16px; right: 15px; width: 275px; height: 275px; background: radial-gradient(circle at 33% 30%, #eed5a5, #c2794d 64%, #7b4d3f); box-shadow: 30px 35px 60px rgba(115,74,56,.22); } .orb-two { left: 10px; bottom: 36px; width: 140px; height: 140px; background: #365342; box-shadow: 14px 16px 30px rgba(45,71,54,.2); } .art-card { position: absolute; right: -10px; bottom: 0; width: 235px; padding: 22px; border: 1px solid rgba(255,255,255,.65); border-radius: 12px; background: rgba(255,254,250,.86); box-shadow: var(--shadow-lg); } .art-card span, .art-card small { display: block; color: var(--accent); font-size: 9px; font-weight: 800; letter-spacing: .16em; } .art-card strong { display: block; margin: 17px 0 27px; font: 500 20px/1.18 var(--font-display); } .art-card small { color: var(--ink-faint); font-weight: 500; letter-spacing: 0; }
  .primary-button, .quiet-button, .add-button { border: 0; border-radius: 8px; cursor: pointer; } .primary-button { padding: 10px 15px; border: 1px solid rgba(255,255,255,.08); background: var(--accent-dark); color: #fff; font-weight: 700; font-size: 12px; box-shadow: 0 2px 0 #263d30, 0 7px 16px rgba(42,68,51,.16); transition: background .16s ease, box-shadow .16s ease, transform .16s ease; } .primary-button:hover { background: #2b4535; box-shadow: 0 2px 0 #263d30, 0 10px 20px rgba(42,68,51,.2); transform: translateY(-1px); } .primary-button:active { box-shadow: 0 1px 0 #263d30, 0 3px 8px rgba(42,68,51,.14); transform: translateY(1px); } .primary-button:focus-visible { outline: 3px solid rgba(180,119,63,.32); outline-offset: 2px; } .primary-button:disabled { opacity: .5; cursor: not-allowed; box-shadow: none; transform: none; } .quiet-button { padding: 10px 12px; border: 1px solid #ded8cd; background: var(--surface); color: var(--ink-soft); font-size: 12px; box-shadow: 0 1px 2px rgba(48,45,38,.05); transition: background .16s ease, border-color .16s ease, box-shadow .16s ease, color .16s ease, transform .16s ease; } .quiet-button:hover { border-color: #cbbda9; background: var(--surface-muted); color: var(--ink); box-shadow: 0 3px 8px rgba(48,45,38,.08); transform: translateY(-1px); } .quiet-button:active { box-shadow: 0 1px 2px rgba(48,45,38,.05); transform: translateY(1px); } .quiet-button:focus-visible { outline: 3px solid rgba(180,119,63,.24); outline-offset: 2px; } .quiet-button:disabled { opacity: .5; cursor: not-allowed; box-shadow: none; transform: none; } .maps-welcome { align-items: stretch; } .maps-welcome-actions { display: flex; gap: 8px; margin-top: 26px; } .maps-welcome-list { flex: 0 0 380px; display: flex; width: 380px; min-height: 380px; max-height: calc(100vh - 220px); flex-direction: column; } .maps-welcome-list .panel-heading { flex: 0 0 auto; } .maps-welcome-list .collection-list { max-height: none; flex: 1 1 auto; overflow-y: auto; }
  .workspace-heading { display: flex; align-items: flex-end; justify-content: space-between; gap: 20px; padding: 42px 40px 25px; } .workspace-heading h1 { margin: 8px 0 4px; font: 500 38px/1 var(--font-display); } .workspace-heading p { margin: 0; color: var(--ink-soft); font-size: 13px; } .heading-actions { display: flex; gap: 7px; } .projection-bar { min-height: 42px; margin: 0 40px 15px; padding: 0 14px; border: 1px solid var(--line); border-radius: 9px; background: rgba(255,254,250,.72); } .projection-bar:empty { display: none; } .workspace-grid { display: grid; grid-template-columns: 245px minmax(360px, 1fr) 270px; gap: 14px; padding: 0 40px 40px; align-items: start; } .panel-surface, .editor-panel { border: 1px solid var(--line); border-radius: 12px; background: var(--surface); box-shadow: var(--shadow-sm); } .collection-panel, .inspector-panel { min-height: 650px; } .collection-panel { display: flex; flex-direction: column; } .panel-heading, .inspector-heading { display: flex; align-items: center; justify-content: space-between; padding: 18px 17px 12px; } .panel-heading strong { display: block; margin-top: 5px; font: 500 28px var(--font-display); }
  .project-diagnostics { display: grid; gap: 5px; margin: 0 25px 14px; padding: 12px 14px; border: 1px solid #e2b48c; border-radius: 9px; background: #fff5e9; color: #765a39; font-size: 11px; } .editor-panel { min-height: 650px; padding: 24px 25px 18px; } .editor-header { display: flex; align-items: flex-start; justify-content: space-between; min-height: 72px; } .editor-header h2 { margin: 8px 0 0; font: 500 28px/1.1 var(--font-display); } .editor-status { color: var(--ink-faint); font-size: 11px; } .saving-dot, .saved-dot { display: inline-block; width: 7px; height: 7px; margin-right: 5px; border-radius: 50%; background: #d6a35f; } .saved-dot { width: auto; height: auto; margin: 0 4px 0 0; color: #6fa276; background: transparent; } .document-conflict { margin: -4px 0 16px; padding: 13px 14px; border: 1px solid #e2b48c; border-radius: 9px; background: #fff5e9; color: #765a39; } .document-conflict strong { font-size: 12px; } .document-conflict p { margin: 5px 0 10px; font-size: 11px; line-height: 1.5; } .conflict-compare { margin: 8px 0 10px; padding: 8px 10px; border: 1px solid #ead7c2; border-radius: 7px; background: #fffaf3; } .conflict-compare summary { cursor: pointer; font-size: 11px; font-weight: 700; } .conflict-compare pre { max-height: 180px; margin: 8px 0 0; overflow: auto; white-space: pre-wrap; font: 11px/1.5 ui-monospace, monospace; }   .conflict-actions { display: flex; flex-wrap: wrap; gap: 6px; }
  .map-editor-shell { display: flex; min-height: 0; flex: 1 1 auto; flex-direction: column; }
  .map-editor-toolbar { display: flex; align-items: center; gap: 12px; min-height: 44px; padding: 0 18px; border-bottom: 1px solid var(--line); background: var(--surface); }
  .map-save-chip { display: inline-flex; align-items: center; gap: 7px; color: var(--ink-soft); font-size: 11px; } .map-save-dot { width: 7px; height: 7px; border-radius: 50%; background: #72a97a; }
  .map-state-warn { color: #8a6a3b; } .map-state-warn .map-save-dot { background: #d6a35f; }
  .map-state-error { color: #a14f42; } .map-state-error .map-save-dot { background: #c05a4b; }
  .map-conflict-path { min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; color: var(--ink-faint); font: 11px ui-monospace, monospace; }
  .map-toolbar-actions { margin-left: auto; display: flex; gap: 6px; } .map-toolbar-actions .quiet-button:disabled { opacity: .45; cursor: not-allowed; }
  .map-conflict-banner { display: flex; align-items: center; justify-content: space-between; gap: 16px; padding: 12px 18px; border-bottom: 1px solid #e2b48c; background: #fff5e9; color: #765a39; }
  .map-conflict-copy strong { font-size: 12px; } .map-conflict-copy p { margin: 4px 0 0; font-size: 11px; line-height: 1.5; } .map-conflict-copy code { display: block; margin-top: 6px; font: 11px ui-monospace, monospace; }
  .map-conflict-actions { display: flex; flex: 0 0 auto; align-items: center; gap: 6px; }
  .ai-rewrite-panel { display: grid; gap: 12px; margin-bottom: 14px; padding: 14px; border: 1px solid #d8c3a5; border-radius: 10px; background: #fff8ed; }
  .ai-rewrite-heading, .ai-rewrite-actions { display: flex; align-items: center; justify-content: space-between; gap: 10px; }
  .ai-rewrite-heading strong { display: block; margin-top: 4px; color: var(--ink); font-size: 14px; }
  .ai-instruction { display: grid; gap: 5px; color: var(--ink-soft); font-size: 10px; font-weight: 700; }
  .ai-instruction textarea { width: 100%; resize: vertical; padding: 9px; border: 1px solid var(--line); border-radius: 7px; background: var(--surface); color: var(--ink); font: 12px/1.45 var(--font-body); }
  .ai-stream-output, .ai-proposal-editor { max-height: 220px; overflow: auto; margin: 0; padding: 11px; border: 1px solid var(--line); border-radius: 7px; background: var(--surface); color: var(--ink); white-space: pre-wrap; font: 12px/1.55 var(--font-body); }
  .ai-proposal-editor { width: 100%; resize: vertical; }
  .ai-diff-grid { display: grid; grid-template-columns: repeat(2, minmax(0, 1fr)); gap: 10px; }
  @media (max-width: 620px) { .ai-diff-grid { grid-template-columns: 1fr; } }
  @media (max-width: 760px) { .map-conflict-banner { align-items: flex-start; flex-direction: column; } } .editor-footer { display: flex; align-items: center; justify-content: space-between; padding-top: 14px; color: var(--ink-faint); font-size: 11px; } .editor-footer div { display: flex; gap: 4px; } .editor-empty { display: grid; place-items: center; min-height: 500px; padding: 30px; text-align: center; } .empty-mark, .disabled-icon { display: grid; place-items: center; width: 52px; height: 52px; border-radius: 16px; background: #f2e4d2; color: var(--accent); font-size: 23px; } .editor-empty h3 { margin: 18px 0 6px; font: 500 23px var(--font-display); } .editor-empty p { max-width: 280px; margin: 0; color: var(--ink-soft); font-size: 12px; line-height: 1.6; }
  .inspector-heading-actions { display: flex; flex-direction: column; align-items: flex-end; gap: 5px; } .inspector-ai-action { display: inline-flex; align-items: center; gap: 4px; padding: 3px 6px; border: 1px solid #d9b98f; border-radius: 5px; background: #fff8ed; color: var(--accent); font-size: 9px; font-weight: 700; cursor: pointer; } .inspector-ai-action:disabled { opacity: .65; cursor: wait; } .inspector-ai-action span { font-size: 10px; } .inspector-ai-fill { padding: 12px 16px; border-bottom: 1px solid var(--line); background: #fff8ed; } .inspector-ai-fill-heading { display: flex; align-items: center; justify-content: space-between; gap: 8px; } .inspector-ai-fill-heading strong { color: var(--ink); font-size: 11px; } .inspector-ai-fill p { margin: 8px 0 0; color: var(--ink-soft); font-size: 10px; line-height: 1.45; } .inspector-ai-suggestion { display: grid; gap: 8px; margin-top: 10px; padding-top: 10px; border-top: 1px solid #ead7c2; } .inspector-ai-suggestion strong, .inspector-ai-suggestion span, .inspector-ai-suggestion small { display: block; } .inspector-ai-suggestion strong { color: var(--ink-soft); font-size: 10px; } .inspector-ai-suggestion span { margin-top: 4px; color: var(--ink); font-size: 11px; line-height: 1.45; } .inspector-ai-suggestion small { margin-top: 4px; color: var(--ink-faint); font-size: 9px; line-height: 1.4; } .inspector-ai-suggestion > div:last-child { display: flex; gap: 6px; }
  .date-editor { display: grid; gap: 8px; padding: 10px; border: 1px solid var(--line); border-radius: 8px; background: #fcf8f1; } .date-fields { display: grid; grid-template-columns: repeat(3, minmax(0, 1fr)); gap: 6px; } .date-fields label { display: grid; gap: 4px; color: var(--ink-faint); font-size: 9px; font-weight: 700; text-transform: uppercase; } .date-fields input { min-width: 0; width: 100%; padding: 8px 6px; border: 1px solid var(--line); border-radius: 7px; background: var(--canvas); color: var(--ink); font-size: 11px; } .date-fields input:focus { border-color: #c99965; box-shadow: 0 0 0 3px rgba(180,119,63,.1); outline: 0; } .date-preview { color: var(--accent); font-size: 10px; font-weight: 700; } .date-clear, .date-empty { width: fit-content; padding: 0; border: 0; background: transparent; color: var(--ink-faint); font-size: 10px; cursor: pointer; } .date-empty { padding: 8px 10px; border: 1px dashed #d3c0a9; border-radius: 7px; color: var(--accent); } .inspector-heading { border-bottom: 1px solid var(--line); } .inspector-heading strong { display: block; margin-top: 7px; font: 500 20px var(--font-display); } .inspector-type { padding: 4px 7px; border-radius: 5px; background: #f2e4d2; color: var(--accent); font-size: 9px; font-weight: 800; text-transform: uppercase; } .inspector-section { padding: 18px 16px; border-bottom: 1px solid var(--line); } .inspector-section h3, .section-title h3 { margin: 0; color: var(--ink-soft); font-size: 10px; letter-spacing: .08em; text-transform: uppercase; } .property-field { display: block; margin-top: 14px; } .property-field span { display: block; margin-bottom: 5px; color: var(--ink-soft); font-size: 10px; } .property-field b { margin-left: 3px; color: var(--accent); } .property-field input { width: 100%; padding: 8px 9px; border: 1px solid var(--line); border-radius: 7px; outline: 0; background: var(--canvas); color: var(--ink); font-size: 11px; } .property-field input:focus { border-color: #c99965; box-shadow: 0 0 0 3px rgba(180,119,63,.1); } .section-title { display: flex; align-items: center; justify-content: space-between; } .section-title span { color: var(--ink-faint); font-size: 11px; } .asset-row strong, .asset-row small { display: block; } .asset-row strong { font-size: 10px; } .asset-row small { margin-top: 3px; color: var(--ink-faint); font-size: 9px; } .drop-zone { display: flex; flex-direction: column; align-items: center; gap: 4px; margin-top: 12px; padding: 16px 8px; border: 1px dashed #d3c0a9; border-radius: 8px; background: #fcf8f1; color: var(--accent); text-align: center; cursor: pointer; } .drop-zone span { font-size: 22px; } .drop-zone strong { color: var(--ink-soft); font-size: 10px; } .drop-zone small { color: var(--ink-faint); font-size: 9px; } .asset-row { display: flex; align-items: center; gap: 8px; margin-top: 9px; } .asset-icon { display: grid; place-items: center; width: 25px; height: 25px; border-radius: 6px; background: #ede9e0; color: var(--accent); }
  .map-contribution p { margin: 9px 0 10px; color: var(--ink-soft); font-size: 10px; line-height: 1.5; }
  .map-contribution > .quiet-button { margin: 0 0 8px; padding: 6px 8px; border: 1px solid #d9cdbd; border-radius: 7px; background: #fcf8f1; font-size: 10px; }
  .map-contribution > small { display: block; color: var(--ink-faint); font-size: 10px; }
  .map-location-row { display: grid; gap: 8px; margin-top: 10px; padding-top: 10px; border-top: 1px solid var(--line); }
  .map-location-row > div:first-child { min-width: 0; }
  .map-location-row strong, .map-location-row small { display: block; overflow-wrap: anywhere; }
  .map-location-row strong { color: var(--ink); font-size: 10px; }
  .map-location-row small { margin-top: 3px; color: var(--ink-faint); font-size: 9px; line-height: 1.4; }
  .map-location-row > div:last-child { display: flex; flex-wrap: wrap; gap: 3px; align-items: center; }
  .map-location-row .quiet-button { padding: 5px 6px; font-size: 9px; }
  .map-unresolved-badge { color: #a14f42; font-weight: 700; }
  .map-unresolved-note { color: #a14f42; font-size: 9px; }
  .empty-workspace-state { display: grid; min-height: calc(100vh - 58px); place-content: center; justify-items: center; padding: 40px; text-align: center; } .empty-workspace-state h1 { margin: 12px 0 10px; font: 500 42px var(--font-display); } .empty-workspace-state p { max-width: 360px; margin: 0 0 24px; color: var(--ink-soft); font-size: 13px; line-height: 1.6; } .project-transition-backdrop { position: fixed; inset: 0; z-index: 100; display: grid; place-items: center; background: rgba(37, 37, 31, .34); } .project-transition-card { display: grid; justify-items: center; gap: 10px; min-width: 230px; padding: 24px 28px; border: 1px solid #d8cdbd; border-radius: 12px; background: var(--surface); box-shadow: var(--shadow-lg); color: var(--ink); text-align: center; } .project-transition-card strong { font: 500 20px var(--font-display); } .project-transition-card small { color: var(--ink-soft); font-size: 11px; } .project-transition-spinner { width: 22px; height: 22px; border: 3px solid #eadfce; border-top-color: var(--accent); border-radius: 50%; animation: project-transition-spin .8s linear infinite; } @keyframes project-transition-spin { to { transform: rotate(360deg); } } .toast { position: fixed; right: 24px; bottom: 24px; z-index: 60; max-width: 430px; padding: 13px 14px; border: 1px solid #e5d4ba; border-radius: 9px; background: #fff8ed; box-shadow: var(--shadow-lg); color: #765a39; font-size: 12px; } .toast button { margin-left: 10px; border: 0; background: none; color: inherit; cursor: pointer; font-size: 17px; } .inspector-empty { display: grid; place-items: center; min-height: 240px; padding: 30px; color: var(--ink-faint); text-align: center; font-size: 10px; } .inspector-empty p { max-width: 170px; margin-top: 13px; line-height: 1.6; }
  @media (max-width: 1180px) { .workspace-grid { grid-template-columns: 220px minmax(320px, 1fr); } .inspector-panel { grid-column: 1 / -1; min-height: auto; display: grid; grid-template-columns: repeat(3, 1fr); } .inspector-heading { grid-column: 1 / -1; } .inspector-section { border-right: 1px solid var(--line); border-bottom: 0; } }
  @media (max-width: 760px) { .studio-shell { display: block; } .rail { display: block; width: 100%; height: auto; padding: 12px 14px; } .startup-rail { min-height: 100vh; padding: 24px 14px; } .brand { padding: 0 4px 12px; } .rail-label, .rail-spacer, .rail-footer, .module-menu { display: none; } .startup-rail .rail-label, .startup-rail .recent-projects { display: block; } .startup-rail .recent-label { margin-top: 27px; } .startup-rail .rail-button { display: flex; width: 100%; margin: 0 0 5px; padding: 10px 11px; } .startup-rail .rail-button span:not(.rail-icon) { display: inline; } .rail-button { display: inline-flex; width: auto; margin: 0 3px 0 0; padding: 8px 10px; } .rail-button span:not(.rail-icon) { display: none; } .project-menu .rail-button { display: flex; width: 100%; margin: 0 0 3px; } .project-menu .rail-button span:not(.rail-icon) { display: inline; } .topbar { min-height: 58px; padding: 0 17px; } .breadcrumbs span:first-child, .sync-badge { display: none; } .global-search { width: 150px; } .welcome { min-height: calc(100vh - 58px); display: block; padding: 55px 24px; } .welcome h1 { font-size: 52px; } .welcome-art { width: 100%; height: 270px; margin-top: 35px; transform: scale(.84); transform-origin: left top; } .workspace-heading { display: block; padding: 30px 17px 18px; } .workspace-heading h1 { font-size: 33px; } .heading-actions { margin-top: 18px; } .projection-bar { margin: 0 17px 12px; } .workspace-grid { display: flex; flex-direction: column; padding: 0 17px 25px; } .collection-panel, .editor-panel, .inspector-panel { width: 100%; min-height: auto; } .collection-list { max-height: 260px; overflow-y: auto; } .inspector-panel { display: block; } .inspector-section { border-bottom: 1px solid var(--line); border-right: 0; } .editor-panel { padding: 18px 14px 14px; } .editor-header h2 { font-size: 24px; } .editor-footer { align-items: flex-end; gap: 10px; } .toast { right: 12px; bottom: 12px; left: 12px; } }
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
  :global(.timeline-card small) { display: block; margin-top: 3px; color: var(--ink-faint); font-size: 10px; }
  :global(.timeline-map-button) { margin-top: 8px; padding: 5px 9px; border: 1px solid #d9cdbd; border-radius: 7px; background: #fffefa; color: #62594e; font: 600 10px Inter, ui-sans-serif, system-ui, sans-serif; cursor: pointer; }
  :global(.timeline-map-button:hover) { border-color: #b4773f; color: #55351f; }
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
  .collection-search { display: flex; align-items: center; gap: 8px; min-height: 40px; margin: 0 10px 8px; padding: 0 10px; border: 1px solid #ebe7de; border-radius: 9px; background: #fffefa; box-shadow: 0 1px 2px rgba(38, 42, 33, .03); color: var(--ink-faint); }
  .collection-search span { flex: 0 0 auto; font-size: 17px; line-height: 1; }
  .collection-search input { min-width: 0; flex: 1; border: 0; outline: 0; background: transparent; color: var(--ink); font-size: 11px; }
  .collection-item { appearance: none; border: 1px solid transparent; box-shadow: 0 1px 2px rgba(38, 42, 33, .03); }
  .collection-item:hover { border-color: #e5d8c6; box-shadow: var(--shadow-sm); }
  .collection-item.selected { border-color: #d8c3a5; box-shadow: inset 3px 0 var(--accent), var(--shadow-sm); }
  .collection-list { display: grid; align-content: start; gap: 5px; padding: 0 10px 10px; }
  .collection-item { display: flex; align-items: center; gap: 9px; width: 100%; height: 58px; min-height: 58px; max-height: 58px; margin: 0; padding: 9px 10px; overflow: hidden; border: 1px solid #ebe7de; border-radius: 9px; background: #fffefa; color: var(--ink); font: inherit; line-height: 1.2; text-align: left; text-decoration: none; cursor: pointer; }
  .collection-item:focus-visible { outline: 3px solid rgba(180, 119, 63, .25); outline-offset: 1px; }
  .collection-item .entity-glyph { display: grid; place-items: center; flex: 0 0 40px; width: 40px; height: 40px; border: 0; border-radius: 50%; background: #f0ece5; font-size: 13px; font-weight: 800; line-height: 1; letter-spacing: .02em; }
  .search-result .entity-glyph { display: grid; place-items: center; width: 28px; height: 28px; border-radius: 50%; font-size: 10px; font-weight: 800; }
  .entity-glyph-person { color: #9b6847; background: #f8eadf !important; }
  .entity-glyph-place { color: #557d63; background: #e8f0e8 !important; }
  .entity-glyph-faction { color: #7b638e; background: #eee8f3 !important; }
  .entity-glyph-artifact { color: #a2783c; background: #f7eed8 !important; }
  .entity-glyph-culture { color: #4e7890; background: #e4eff3 !important; }
  .entity-glyph-event { color: #ae6a56; background: #f8e8e2 !important; }
  .entity-glyph-manuscript { color: #7c6548; background: #f4eadb !important; }
  .entity-glyph-reference-page { color: #597a84; background: #e5eff0 !important; }
  .entity-glyph-unknown { color: #837d73; background: #eeeae3 !important; }
  .collection-tabs { display: flex; gap: 4px; margin: 0 10px 8px; padding: 3px; border: 1px solid var(--line); border-radius: 8px; background: var(--surface-muted); }
  .collection-tabs button { flex: 1; min-width: 0; padding: 7px 5px; border: 0; border-radius: 6px; background: transparent; color: var(--ink-faint); font-size: 10px; cursor: pointer; }
  .collection-tabs button:hover, .collection-tabs button.active { background: var(--surface); color: var(--accent-dark); box-shadow: var(--shadow-sm); }
  .collection-item .item-copy { display: grid; min-width: 0; align-content: center; gap: 4px; overflow: hidden; }
  .collection-item .item-copy strong { overflow: hidden; color: var(--ink); font-size: 13px; font-weight: 700; line-height: 1.15; text-overflow: ellipsis; white-space: nowrap; }
  .collection-item .item-copy small { width: max-content; max-width: 150px; margin: 0; padding: 3px 6px; border-radius: 4px; background: #f4f0e8; color: var(--ink-faint); font-size: 10px; line-height: 1; letter-spacing: .04em; text-transform: uppercase; }
  .collection-item .item-arrow { flex: 0 0 10px; width: 10px; margin-left: auto; color: #c3b6a4; font-size: 18px; line-height: 1; text-align: right; }
  .collection-item:hover .item-arrow, .collection-item.selected .item-arrow { color: var(--accent); }
  .new-form-heading { display: flex; align-items: flex-start; justify-content: space-between; gap: 10px; margin-bottom: 13px; }
  .new-form-heading strong { display: block; margin-top: 5px; font: 500 19px var(--font-display); }
  .new-form-close { border: 0; background: transparent; color: var(--ink-faint); font-size: 20px; line-height: 1; cursor: pointer; }
  .new-form-actions { display: flex; justify-content: flex-end; gap: 7px; margin-top: 14px; }
  .new-form-actions .quiet-button { padding: 9px 10px; }
  .modal-backdrop { position: fixed; inset: 0; z-index: 20; display: grid; place-items: center; padding: 20px; background: rgba(37, 37, 31, .28); }
  .plugin-confirm-modal { z-index: 30; }
  .dialog { width: min(440px, 100%); margin: 0; padding: 22px; border: 1px solid #e3d9ca; border-radius: 14px; background: var(--surface); box-shadow: 0 22px 70px rgba(37, 37, 31, .2); }
  .dialog .new-form-heading { margin-bottom: 18px; }
  .dialog .new-form-heading strong { font-size: 23px; }
  .dialog .new-form-close { width: 30px; height: 30px; border-radius: 7px; background: var(--surface-muted); color: var(--ink-soft); }
  .dialog .new-form-close:hover { background: #ebe6dd; color: var(--ink); }
  .capability-list { display: grid; gap: 6px; max-height: min(240px, 35vh); margin: 2px 0 12px; padding: 10px; overflow-y: auto; border: 1px solid #d9cdbd; border-radius: 8px; background: var(--canvas); color: var(--ink-soft); }
  .capability-item { padding: 6px 8px; border-bottom: 1px solid rgba(217, 205, 189, .65); font-size: 11px; line-height: 1.4; }
  .capability-item:last-child { border-bottom: 0; }
  .capability-empty { margin-top: -2px; }
  .dialog .new-form-actions { margin-top: 20px; }
  .git-menu strong, .git-menu small { display: block; }
  .git-menu strong { color: #eef0e9; font-size: 11px; }
  .git-menu small { padding: 4px 0; color: #aab9ad; font-size: 10px; }
  .git-preview { display: grid; gap: 4px; margin-top: 5px; padding-top: 6px; border-top: 1px solid rgba(170, 185, 173, .2); }
  .git-preview strong { font-size: 10px; }
  .git-preview small { max-height: 72px; overflow: auto; line-height: 1.45; }
  .git-menu button { width: 100%; margin-top: 7px; padding: 7px; border: 0; border-radius: 6px; background: #d5ab6c; color: #2c4032; font-size: 10px; cursor: pointer; }
  .git-menu button:disabled { opacity: .55; cursor: wait; }
  .search-modal { position: absolute; top: 58px; right: 40px; z-index: 5; width: min(460px, calc(100vw - 80px)); max-height: min(560px, calc(100vh - 100px)); overflow: auto; border: 1px solid var(--line); border-radius: 12px; background: var(--surface); box-shadow: var(--shadow-lg); }
  .search-modal-heading { display: flex; align-items: center; justify-content: space-between; padding: 13px 15px 10px; border-bottom: 1px solid var(--line); color: var(--ink-soft); font-size: 11px; }
  .search-modal-heading .quiet-button { padding: 0 4px; font-size: 18px; }
  .search-state { margin: 0; padding: 28px 16px; color: var(--ink-faint); font-size: 11px; text-align: center; }
  .search-results { padding: 7px; }
  .search-result { width: 100%; display: flex; align-items: center; gap: 9px; padding: 9px; border: 1px solid transparent; border-radius: 8px; background: transparent; color: var(--ink); text-align: left; cursor: pointer; }
  .search-result:hover { border-color: #e5d8c6; background: var(--surface-muted); }
  .search-result strong, .search-result small { display: block; }
  .search-result strong { font-size: 12px; }
  .search-result small { margin-top: 3px; color: var(--ink-faint); font-size: 10px; }

  :global(html) { background: var(--canvas); }
  :global(body) { min-width: 320px; text-rendering: optimizeLegibility; }
  :global(body.modal-open) { overflow: hidden; }
  :global(button), :global(input), :global(select) { -webkit-tap-highlight-color: transparent; }
  :global(button:focus-visible), :global(input:focus-visible), :global(select:focus-visible) { outline: 3px solid rgba(180, 119, 63, .28); outline-offset: 2px; }
  .workspace-nav { display: grid; gap: 3px; }
  .plugin-nav-title { min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .rail-button { transition: background .16s ease, color .16s ease, transform .16s ease; }
  .rail-button:active, .primary-button:active { transform: translateY(1px); }
  .rail { position: sticky; top: 0; align-self: flex-start; height: 100vh; max-height: 100vh; overflow-y: auto; overscroll-behavior: contain; }
  .topbar { position: sticky; top: 0; z-index: 4; backdrop-filter: blur(14px); }
  .workspace-grid > * { min-width: 0; }
  .collection-panel, .editor-panel { overflow: hidden; }
  .inspector-panel { overflow: visible; }
  .panel-heading { gap: 12px; }
  .panel-heading > div, .editor-header > div:first-child, .inspector-heading > div { min-width: 0; }
  .editor-header h2 { overflow-wrap: anywhere; }
  .collection-search, .global-search, .property-field input { transition: border-color .16s ease, box-shadow .16s ease; }
  .collection-search:focus-within, .global-search:focus-within { border-color: #c99965; box-shadow: 0 0 0 3px rgba(180, 119, 63, .1); }
  .primary-button, .quiet-button, .add-button, .new-form-close { transition: background .16s ease, color .16s ease, opacity .16s ease, transform .16s ease; }
  .unsaved-dot { display: inline-block; width: 7px; height: 7px; margin-right: 5px; border-radius: 50%; background: #d6a35f; }
  .editor-fullscreen { position: fixed; inset: 0 0 0 248px; z-index: 30; display: flex; flex-direction: column; min-height: 100vh; overflow: auto; padding: 28px clamp(22px, 5vw, 72px) 24px; border: 0; border-radius: 0; background: var(--canvas); box-shadow: 0 24px 80px rgba(37, 37, 31, .18); }
  .editor-fullscreen .editor-header { width: min(1120px, 100%); flex: 0 0 auto; align-self: center; }
  .editor-fullscreen :global(.editor-shell) { display: grid; width: min(1120px, 100%); min-height: 0; flex: 1 1 auto; align-self: center; grid-template-rows: auto minmax(0, 1fr) auto; }
  .editor-fullscreen :global(.editor-content) { overflow: auto; }
  .editor-fullscreen .editor-footer { width: min(1120px, 100%); flex: 0 0 auto; align-self: center; }
  .editor-fullscreen .editor-header h2 { font-size: 32px; }
  .editor-fullscreen .editor-footer { padding-top: 12px; }
  .dialog { max-height: min(680px, calc(100vh - 32px)); overflow-y: auto; }
  .search-modal { top: 58px; }

  @media (max-width: 1040px) {
    .topbar { padding-inline: 28px; }
    .workspace-heading { padding: 36px 28px 23px; }
    .projection-bar { margin-inline: 28px; }
    .workspace-grid { grid-template-columns: 215px minmax(280px, 1fr); padding-inline: 28px; }
  }

  @media (max-width: 760px) {
    :global(body) { overflow-x: hidden; }
    .rail { position: static; height: auto; max-height: none; overflow: visible; display: flex; flex-direction: column; gap: 0; }
    .workspace-nav { display: flex; gap: 4px; margin: 0 -4px 9px; overflow-x: auto; scrollbar-width: none; }
    .workspace-nav::-webkit-scrollbar { display: none; }
    .workspace-nav .rail-button { flex: 1 0 auto; justify-content: center; width: auto; margin: 0; padding-inline: 12px; }
    .workspace-nav .rail-button span:not(.rail-icon) { display: inline; }
    .topbar { position: relative; align-items: stretch; flex-direction: column; gap: 10px; min-height: 0; padding: 12px 17px; }
    .breadcrumbs { width: 100%; }
    .top-actions { width: 100%; }
    .global-search { flex: 1; width: auto; }
    .search-modal { top: 105px; right: 17px; left: 17px; width: auto; }
    .welcome { min-height: calc(100vh - 130px); padding-top: 42px; }
    .welcome h1 { font-size: clamp(43px, 13vw, 56px); }
    .welcome p { font-size: 14px; }
    .welcome-art { margin-top: 22px; transform: scale(.72); transform-origin: left top; }
    .maps-welcome-list { flex: 0 0 300px; width: 300px; transform: scale(.72); transform-origin: left top; margin-top: 22px; }
    .workspace-heading { padding: 28px 17px 18px; }
    .workspace-heading h1 { font-size: clamp(31px, 10vw, 38px); }
    .workspace-heading p { max-width: 38ch; line-height: 1.5; }
    .heading-actions, .heading-actions .quiet-button { width: 100%; }
    .heading-actions .quiet-button { text-align: left; }
    .projection-bar { margin: 0 17px 12px; }
    .workspace-grid { gap: 12px; padding: 0 17px 25px; }
    .collection-panel, .editor-panel, .inspector-panel { border-radius: 11px; }
    .collection-list { max-height: 320px; -webkit-overflow-scrolling: touch; }
    .panel-heading strong { font-size: 24px; }
    .editor-panel { padding: 18px 14px 14px; }
    .editor-fullscreen { inset: 0; padding: 16px 14px 12px; }
    .editor-fullscreen .editor-header { min-height: 58px; }
    .editor-fullscreen .editor-header h2 { font-size: 24px; }
    .editor-header { min-height: 62px; gap: 10px; }
    .editor-status { flex: 0 0 auto; }
    .editor-footer { align-items: flex-start; flex-wrap: wrap; }
    .editor-footer > div { width: 100%; justify-content: flex-end; }
    .editor-empty { min-height: 300px; }
    .inspector-panel { display: block; }
    .inspector-section { border-right: 0; }
    .date-fields { gap: 5px; }
    .date-fields input { padding-inline: 5px; }
    .modal-backdrop { padding: 12px; }
    .dialog { padding: 18px; border-radius: 12px; }
    .toast { right: 12px; bottom: 12px; left: 12px; max-width: none; }
    :global(.timeline-event) { grid-template-columns: 1fr; gap: 5px; }
    :global(.timeline-date) { padding-left: 1px; }
  }

  @media (max-width: 430px) {
    .startup-rail { padding-top: 20px; }
    .brand { padding-bottom: 10px; }
    .startup-primary { min-height: 43px; }
    .welcome-art { height: 225px; }
    .editor-footer > div { flex-direction: column-reverse; }
    .editor-footer > div .quiet-button { width: 100%; text-align: center; }
  }

  .rail-create-button { width: 100%; display: flex; align-items: center; gap: 11px; margin: 0 0 18px; padding: 12px 11px; border: 1px solid rgba(213,171,108,.55); border-radius: 8px; background: #d5ab6c; color: #2c4032; font-size: 14px; font-weight: 800; text-align: left; cursor: pointer; }
  .rail-create-button:hover { background: #e1bc82; }
  .rail-create-button .rail-icon { color: #2c4032; font-size: 17px; }
  .create-dialog { display: flex; flex-direction: column; width: min(980px, 100%); max-height: min(760px, calc(100vh - 32px)); padding: 0; overflow: hidden; }
  .create-dialog-heading { display: flex; align-items: flex-start; justify-content: space-between; gap: 20px; padding: 24px 26px 20px; border-bottom: 1px solid var(--line); }
  .create-dialog-heading strong { display: block; margin-top: 6px; font: 500 27px/1.05 var(--font-display); }
  .create-dialog-heading p { max-width: 560px; margin: 9px 0 0; color: var(--ink-soft); font-size: 12px; line-height: 1.5; }
  .create-dialog-body { display: grid; grid-template-columns: 300px minmax(0, 1fr); min-height: 440px; overflow: hidden; }
  .create-template-panel { min-width: 0; overflow-y: auto; padding: 20px 13px 20px; border-right: 1px solid var(--line); background: #faf8f2; }
  .create-panel-label { display: block; margin-bottom: 16px; color: var(--accent); font-size: 9px; font-weight: 800; letter-spacing: .18em; text-transform: uppercase; }
  .create-template-group > span { display: block; margin: 0 4px 2px; color: var(--ink-faint); font-size: 9px; font-weight: 800; letter-spacing: .14em; text-transform: uppercase; }
  .create-template-group { display: grid; gap: 6px; margin-top: 18px; }
  .create-template-group:first-child { margin-top: 0; }
  .create-template-card { display: grid; grid-template-columns: 36px minmax(0, 1fr) 18px; align-items: center; gap: 9px; width: 100%; padding: 10px; border: 1px solid transparent; border-radius: 9px; background: transparent; color: var(--ink); text-align: left; cursor: pointer; }
  .create-template-card:hover { border-color: #e5d8c6; background: #fffefa; }
  .create-template-card.selected { border-color: #d8c3a5; background: #fffefa; box-shadow: inset 3px 0 var(--accent), var(--shadow-sm); }
  .create-template-icon { display: grid; place-items: center; width: 34px; height: 34px; border-radius: 10px; background: #f2e4d2; color: var(--accent); font-size: 13px; font-weight: 800; }
  .create-template-copy { min-width: 0; }
  .create-template-copy strong, .create-template-copy small { display: block; overflow: hidden; text-overflow: ellipsis; }
  .create-template-copy strong { font-size: 12px; }
  .create-template-copy small { margin-top: 4px; color: var(--ink-faint); font-size: 10px; line-height: 1.35; white-space: nowrap; }
  .create-template-check { color: var(--accent); font-size: 15px; font-weight: 800; text-align: center; }
  .create-form-panel { min-width: 0; overflow-y: auto; padding: 25px 28px 28px; }
  .create-form-title { padding-bottom: 18px; border-bottom: 1px solid var(--line); }
  .create-form-title h2 { margin: 7px 0 4px; font: 500 25px/1.1 var(--font-display); }
  .create-form-title p { margin: 0; color: var(--ink-soft); font-size: 12px; line-height: 1.5; }
  .create-input-field { display: block; margin-top: 17px; }
  .create-input-field > span, .create-input-field > label > span { display: block; margin-bottom: 6px; color: var(--ink-soft); font-size: 10px; font-weight: 700; }
  .create-input-field b { margin-left: 3px; color: var(--accent); }
  .create-input-field > input, .create-input-field > textarea, .create-input-field > select, .create-input-field > label + input, .create-input-field > label + textarea, .create-input-field > label + select { width: 100%; padding: 10px 11px; border: 1px solid #d9cdbd; border-radius: 8px; outline: 0; background: var(--canvas); color: var(--ink); font-size: 12px; }
  .create-input-field > textarea { min-height: 78px; resize: vertical; line-height: 1.5; }
  .create-input-field > input:focus, .create-input-field > textarea:focus, .create-input-field > select:focus { border-color: #c99965; box-shadow: 0 0 0 3px rgba(180,119,63,.1); }
  .create-input-field > label + .date-editor, .create-input-field > label + .create-checkbox { display: flex; }
  .create-checkbox { align-items: center; gap: 8px; min-height: 38px; color: var(--ink-soft); font-size: 12px; }
  .create-checkbox input { width: 16px; height: 16px; accent-color: var(--accent-dark); }
  .create-form-empty { display: grid; min-height: 300px; place-items: center; color: var(--ink-faint); font-size: 12px; }
  .create-dialog-actions { display: flex; justify-content: flex-end; gap: 8px; padding: 15px 26px; border-top: 1px solid var(--line); background: #fcfbf7; }
  .discard-backdrop { position: fixed; inset: 0; z-index: 30; display: grid; place-items: center; padding: 20px; background: rgba(37, 37, 31, .28); }
  .discard-dialog { width: min(390px, 100%); padding: 24px; border: 1px solid #e3d9ca; border-radius: 14px; background: var(--surface); box-shadow: 0 22px 70px rgba(37, 37, 31, .2); }
  .discard-dialog h2 { margin: 8px 0 7px; font: 500 23px/1.1 var(--font-display); }
  .discard-dialog p { margin: 0; color: var(--ink-soft); font-size: 12px; line-height: 1.55; }
  .discard-actions { display: flex; justify-content: flex-end; gap: 7px; margin-top: 20px; }
  .capture-dialog { width: min(420px, 100%); padding: 22px; }
  .capture-preview { display: grid; gap: 4px; margin: 14px 0 2px; padding: 10px 12px; border: 1px dashed #d3c0a9; border-radius: 8px; background: #fcf8f1; }
  .capture-preview-label { color: var(--ink-faint); font-size: 9px; font-weight: 700; letter-spacing: .1em; text-transform: uppercase; }
  .capture-mode { display: grid; grid-template-columns: 1fr 1fr; gap: 6px; margin-top: 17px; padding: 4px; border: 1px solid #d9cdbd; border-radius: 9px; background: var(--canvas); }
  .map-reconcile-notice { min-width: 0; overflow: hidden; color: #8a6a3b; font-size: 11px; text-overflow: ellipsis; white-space: nowrap; }
  .map-unresolved-badge { display: inline-block; padding: 1px 6px; border-radius: 5px; background: #f7e6dd; color: #a14f42; font-weight: 700; }
  .map-unresolved-note { color: #a14f42; font-size: 10px; font-weight: 700; white-space: nowrap; }
  .mobile-create-button { display: none; }

  @media (max-width: 760px) {
    .rail-create-button { display: none; }
    .mobile-create-button { position: fixed; right: 18px; bottom: 18px; z-index: 15; display: grid; place-items: center; width: 52px; height: 52px; border: 1px solid rgba(213,171,108,.7); border-radius: 50%; background: #d5ab6c; color: #2c4032; box-shadow: 0 10px 24px rgba(37,37,31,.2); font-size: 26px; line-height: 1; cursor: pointer; }
    .mobile-create-button:hover { background: #e1bc82; }
    .create-dialog { max-height: calc(100vh - 24px); }
    .create-dialog-heading { padding: 19px 18px 16px; }
    .create-dialog-heading strong { font-size: 23px; }
    .create-dialog-body { grid-template-columns: 1fr; min-height: 0; overflow: auto; }
    .create-template-panel { max-height: 235px; padding: 16px 12px 14px; border-right: 0; border-bottom: 1px solid var(--line); }
    .create-panel-label { margin-bottom: 12px; }
    .create-template-group { display: grid; grid-template-columns: repeat(2, minmax(0, 1fr)); margin-top: 13px; }
    .create-template-group > span { grid-column: 1 / -1; }
    .create-template-card { grid-template-columns: 30px minmax(0, 1fr) 14px; padding: 8px; }
    .create-template-icon { width: 29px; height: 29px; border-radius: 8px; font-size: 11px; }
    .create-template-copy strong { font-size: 11px; }
    .create-template-copy small { font-size: 9px; }
    .create-form-panel { padding: 20px 18px 22px; overflow: visible; }
    .create-dialog-actions { padding: 13px 18px; }
    .create-dialog-actions .primary-button { max-width: 70%; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  }

  .rail:not(.startup-rail) .brand { padding-bottom: 8px; }
  .host-view-back { margin: 24px 40px 0; }
  .collection-list { flex: 1; }
  .list-empty { display: grid; align-content: center; justify-items: start; min-height: 100%; padding: 36px 18px 42px; text-align: left; }
  .list-empty .empty-mark { display: grid; place-items: center; width: 42px; height: 42px; margin-bottom: 18px; border-radius: 13px; background: #f2e4d2; color: var(--accent); font-size: 19px; }
  .list-empty strong { max-width: 22ch; color: var(--ink); font: 500 23px/1.08 var(--font-display); }
  .list-empty p { max-width: 28ch; margin: 10px 0 18px; color: var(--ink-soft); font-size: 12px; line-height: 1.5; }
  .empty-create { padding: 9px 11px; border: 1px solid #d8c3a5; border-radius: 8px; background: #f2e4d2; color: var(--accent-dark); font-size: 11px; font-weight: 700; cursor: pointer; }
  .empty-create:hover { background: #ead7bc; }
  @media (max-width: 1040px) {
    .workspace-heading { padding: 28px 28px 16px; }
  }
  @media (max-width: 760px) {
    .workspace-heading { padding: 20px 17px 12px; }
    .list-empty { min-height: 260px; padding: 28px 14px 32px; }
    .list-empty strong { font-size: 21px; }
  }

  .plugins-toolbar { display: flex; align-items: center; gap: 10px; padding: 0 0 12px; }
  .muted-note { color: var(--ink-faint); font-size: 10px; }
  .plugins-note { margin: 0 0 10px; padding: 9px 12px; border: 1px solid #d9e6db; border-radius: 8px; background: #f2f8f3; color: #3f6b4c; font-size: 11px; }
  .plugins-list { min-height: 200px; max-height: min(620px, calc(100vh - 260px)); overflow-y: auto; padding: 4px 0 8px; }
  .plugins-settings-heading { margin-bottom: 14px; }
  .plugins-settings-heading strong { display: block; font-size: 16px; }
  .plugins-settings-heading p { margin: 6px 0 0; color: var(--ink-soft); font-size: 12px; line-height: 1.5; }
  .plugin-card { padding: 16px 17px; border: 1px solid var(--line); border-radius: 11px; background: var(--surface); box-shadow: var(--shadow-sm); }
  .plugin-card + .plugin-card { margin-top: 11px; }
  .plugin-card-head { display: flex; align-items: flex-start; justify-content: space-between; gap: 12px; }
  .plugin-card-title { min-width: 0; }
  .plugin-card-title strong { display: block; font: 500 17px/1.2 var(--font-display); }
  .plugin-id { display: block; margin-top: 3px; color: var(--ink-faint); font-size: 10px; }
  .plugin-badges { display: flex; flex-wrap: wrap; justify-content: flex-end; gap: 5px; }
  .plugin-badge { padding: 3px 7px; border-radius: 5px; background: #f0ece5; color: var(--ink-soft); font-size: 9px; font-weight: 800; text-transform: uppercase; letter-spacing: .05em; }
  .plugin-badge.badge-off { background: #efe9dd; color: var(--ink-faint); }
  .plugin-badge.beta { background: #f7ead3; color: #936525; }
  .plugin-badge.experimental { background: #f5e0da; color: #a1482f; }
  .plugin-badge.danger { background: #f5e0da; color: #a1482f; }
  .plugin-card-meta { display: flex; flex-wrap: wrap; align-items: center; gap: 6px 13px; margin-top: 10px; color: var(--ink-faint); font-size: 10px; }
  .runtime-dot { color: #6fa276; font-size: 10px; }
  .runtime-dot.runtime-off { color: #c0b7a8; }
  .workspace-beta { margin-left: 5px; color: #936525; font-size: 9px; font-style: normal; font-weight: 800; letter-spacing: .04em; text-transform: uppercase; }
  .plugin-warning { margin: 10px 0 0; padding: 8px 10px; border: 1px solid #ecd9bb; border-radius: 7px; background: #fcf5ea; color: #8a5f24; font-size: 11px; line-height: 1.45; }
  .plugin-error { margin: 8px 0 0; color: #a1482f; font-size: 11px; line-height: 1.45; }
  .plugin-muted { margin: 8px 0 0; color: var(--ink-faint); font-size: 10px; }
  .plugin-actions { display: flex; flex-wrap: wrap; align-items: center; gap: 4px; margin-top: 11px; }
  .plugin-actions .quiet-button { padding: 7px 10px; font-size: 11px; }
  .plugin-toggle { padding: 8px 16px; border: 1px solid var(--accent); border-radius: 8px; background: var(--accent); color: #fff; font-size: 11px; font-weight: 700; cursor: pointer; box-shadow: 0 4px 10px rgba(180, 119, 63, .18); transition: background .16s ease, transform .16s ease; }
  .plugin-toggle:hover { background: #a86b37; }
  .plugin-toggle.on { background: var(--surface); border-color: #d8c3a5; color: var(--ink-soft); box-shadow: none; }
  .plugin-toggle.on:hover { background: var(--surface-muted); color: var(--ink); }
  .plugin-toggle:disabled { opacity: .55; cursor: wait; }
  .plugin-toggle:active { transform: translateY(1px); }
  .version-list { margin-top: 12px; border: 1px solid var(--line); border-radius: 9px; overflow: hidden; }
  .version-row { display: flex; align-items: center; justify-content: space-between; gap: 10px; padding: 9px 11px; background: var(--surface); }
  .version-row + .version-row { border-top: 1px solid var(--line); }
  .version-copy { min-width: 0; }
  .version-name { display: flex; align-items: center; flex-wrap: wrap; gap: 5px; color: var(--ink); font-size: 12px; font-weight: 700; }
  .version-detail { display: block; margin-top: 3px; color: var(--ink-faint); font-size: 10px; }
  .version-tag { padding: 2px 6px; border-radius: 4px; background: #f0ece5; color: var(--ink-soft); font-size: 8px; font-weight: 800; text-transform: uppercase; letter-spacing: .05em; }
  .version-tag.latest { background: #e4efdf; color: #3f6b4c; }
  .version-tag.selected { background: #f2e4d2; color: var(--accent); }
  .version-tag.bundled { background: #e8e4ee; color: #6a5b8a; }
  .version-tag.signed { background: #e4efdf; color: #3f6b4c; }
  .version-tag.unsigned { background: #f5e0da; color: #a1482f; }
  .version-actions { display: flex; flex-wrap: wrap; justify-content: flex-end; gap: 3px; }
  .version-actions .quiet-button { padding: 6px 8px; font-size: 10px; }
  .plugin-details { margin-top: 12px; }
  .plugin-details summary { color: var(--ink-soft); font-size: 11px; font-weight: 700; cursor: pointer; }
  .plugin-details-grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(200px, 1fr)); gap: 10px; margin-top: 11px; }
  .plugin-detail-section { padding: 11px 12px; border: 1px solid var(--line); border-radius: 8px; background: var(--canvas); }
  .plugin-detail-section h4 { margin: 0 0 7px; color: var(--ink-soft); font-size: 9px; letter-spacing: .1em; text-transform: uppercase; }
  .plugin-detail-list { display: grid; gap: 4px; margin: 0; padding: 0; list-style: none; }
  .plugin-detail-list li { color: var(--ink-soft); font-size: 11px; line-height: 1.35; }
  .plugin-detail-list li.granted { color: var(--ink); }
  .plugin-detail-list li.provides { color: #3f6b4c; }
  .plugin-detail-list li.consumes { color: #8a5f24; }
  .plugin-detail-list li.muted-item { color: var(--ink-faint); }
  .plugin-detail-list code, .dialog-body-copy code, .backup-path { color: var(--accent-dark); font-family: ui-monospace, SFMono-Regular, Menlo, monospace; font-size: .95em; }
  .cap-mark { float: right; color: var(--ink-faint); }
  li.granted .cap-mark { color: #6fa276; }
  .dialog-body-copy { margin: 0 0 12px; color: var(--ink-soft); font-size: 12px; line-height: 1.55; }
  .plugin-subhead { margin: 14px 0 7px; color: var(--ink-soft); font-size: 10px; letter-spacing: .08em; text-transform: uppercase; }
  .backup-path { display: block; margin: 4px 0 2px; overflow-wrap: anywhere; font-size: 11px; }
  .danger-button { background: #a1482f; box-shadow: none; }
  .danger-button:hover { background: #8f3f28; }

  .rail { padding-top: 23px; }
  .startup-rail { padding-top: 23px; }
  .brand { align-self: center; align-items: center; justify-content: center; width: 100%; min-height: 0; margin: 0 0 8px; padding: 0; }
  .rail:not(.startup-rail) .brand { padding-bottom: 0; }
  .brand-logo { display: block; width: min(100%, 220px); height: auto; }

  @media (max-width: 600px) {
    .version-row { align-items: flex-start; flex-direction: column; }
    .version-actions { justify-content: flex-start; }
    .plugin-card-head { flex-direction: column; }
    .plugin-badges { justify-content: flex-start; }
  }
  .sync-summary { display: inline-flex; align-items: center; gap: 6px; color: var(--ink-soft); font-size: 10px; }
  .sync-dot { width: 7px; height: 7px; border-radius: 50%; background: #72a97a; }
  .sync-pending .sync-dot { background: #d6a35f; box-shadow: 0 0 0 3px rgba(214,163,95,.15); }
  .sync-failed { color: #a14f42; }
  .sync-failed .sync-dot { background: #c05a4b; box-shadow: 0 0 0 3px rgba(192,90,75,.14); }
</style>
