<script lang="ts">
import { onMount, tick } from "svelte";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { appVersion, appVersionSyncFallback } from "$lib/appVersion";
const logoUrl = "/branding/logo.png";
import { revokeAllResolvedAssetUrls } from "$lib/assets/resolve";
import {
  project,
  type Asset,
  type Entity,
  type EntityPage,
  type Relationship,
  type MapLocation,
  type ProjectModuleManifest,
  type ProjectInfo,
  type GitStatus,
  type PluginAdminEntry,
  type PluginUpgradePlan,
  type AiSettings,
  type AiProviderStatus,
  type AiStreamEvent,
  type AiIndexStatus,
  type ModuleSchemaOverlay,
  type EntityTypeColor,
  type MapFeatureSearchResult,
} from "$lib/project/client";
import type {
  EntityTypeDefinition,
  EntityTemplate,
  FieldDefinition,
  IconRef,
  ModuleContext,
  ModuleId,
  UUID,
  ModuleManifest,
  DaenaModule,
} from "../../packages/module-api/src/index";
import { buildModuleContext } from "$lib/modules/context";
import { fieldAppliesToEntity as fieldAppliesToEnabledTypes } from "$lib/modules/fields";
import {
  contributedRelationshipFields,
  counterpartId,
  counterpartIds,
  coveredRelationshipIds,
  defaultRelationshipMetadata,
  endpointsForCreate,
  manifestOwningRelationshipType,
  relationshipsForField,
} from "$lib/modules/contributed-fields";
import {
  emptyShellNavigationHistory,
  recordShellLocation,
  sameShellLocation,
  shellHistoryBack,
  shellHistoryForward,
  type ShellLocation,
  type ShellNavigationHistory,
  type WorkspaceCollectionLocation,
  type WorkspaceLocationView,
  type WorkspacePaneDimensions,
} from "$lib/navigation/history";
import {
  collectionEntityTypes,
  collectionTabForEntityType,
  presentCollectionPage,
  DEFAULT_COLLECTION_QUERY,
  type LanguagePane,
  type TimelineView,
  type WorkspaceSection,
  type ProjectSection,
  type SettingsSection,
  type CollectionQuery,
  type CollectionResult,
  type WritingView,
  workspaceCollectionTabs,
  workspaceSectionDescription,
  workspaceSectionViewNav,
  workspaceModuleId,
} from "$lib/modules/workspace";
import {
  chronologyWarnings,
  firstEraCalendarId,
  isChronologyDateKey,
  isChronologyPropertyField,
  isEraRelationshipField,
  type EraContext,
} from "$lib/modules/chronology";
import CalendarEditor from "../../packages/modules/timeline/src/CalendarEditor.svelte";
import {
  calendarDateToParts,
  daysInCalendarMonth,
  formatWithCalendar,
  normalizeCalendarDefinition,
  partsToCalendarDate,
  type CalendarDefinition,
} from "../../packages/modules/timeline/src/calendar";
import HostView from "$lib/plugins/HostView.svelte";
import SandboxView from "$lib/plugins/SandboxView.svelte";
import NativeVectorMapEditor from "$lib/maps/native-vector/NativeVectorMapEditor.svelte";
import PhysicalMapEditor from "$lib/maps/physical/PhysicalMapEditor.svelte";
import { nativeVectorSession } from "$lib/maps/native-vector/session";
import ProjectionView from "$lib/ProjectionView.svelte";
import WikiView from "$lib/lore/WikiView.svelte";
import ProjectHome from "$lib/shell/ProjectHome.svelte";
import ProjectCenter from "$lib/ProjectCenter.svelte";
import AppSidebar from "$lib/shell/AppSidebar.svelte";
import GlobalToolbar from "$lib/shell/GlobalToolbar.svelte";
import WorkspaceHeader from "$lib/shell/WorkspaceHeader.svelte";
import WorkspaceViewNav from "$lib/shell/WorkspaceViewNav.svelte";
import CollectionPane from "$lib/shell/CollectionPane.svelte";
import ContentPane from "$lib/shell/ContentPane.svelte";
import InspectorPane from "$lib/shell/InspectorPane.svelte";
import InspectorSection from "$lib/shell/InspectorSection.svelte";
import PaneResizeHandle from "$lib/shell/PaneResizeHandle.svelte";
import StatusSummary from "$lib/shell/StatusSummary.svelte";
import StatusCenter, { type StatusCenterItem, type StatusCenterTone } from "$lib/shell/StatusCenter.svelte";
import "$lib/shell/controls.css";
import SpecializedSurface from "$lib/shell/SpecializedSurface.svelte";
import WorkbenchState from "$lib/shell/WorkbenchState.svelte";
import QuickOpen from "$lib/shell/QuickOpen.svelte";
import EntityGlyph from "$lib/entity-colors/EntityGlyph.svelte";
import { DEFAULT_TYPE_COLOR } from "$lib/entity-colors/presets";
import { FALLBACK_ICON } from "$lib/entity-icons/catalog";
import { trapModalTab } from "$lib/shell/modalFocus";
import { rankQuickOpenItems, type QuickOpenItem } from "$lib/quick-open/model";
import ModuleMount from "$lib/ModuleMount.svelte";
import SettingsView from "$lib/SettingsView.svelte";
import SchemaSettingsPanel from "$lib/SchemaSettingsPanel.svelte";
import SchemaFieldInput from "$lib/schema-workbench/SchemaFieldInput.svelte";
import { overlayValidationStatus, primarySchemaNamespace, summarizePackageCounts } from "$lib/schema-workbench";
import { allowLeaveSchemaEditor, isSchemaEditorDirty } from "$lib/schemaEditorGuard";
import GitSettingsPanel from "$lib/GitSettingsPanel.svelte";
import RelationshipPicker from "$lib/RelationshipPicker.svelte";
import RelationshipMetadataDialog from "$lib/RelationshipMetadataDialog.svelte";
import AssetDialog from "$lib/AssetDialog.svelte";
import ExternalImportDialog from "$lib/ExternalImportDialog.svelte";
import CalendarPicker from "$lib/CalendarPicker.svelte";
import DateEditor from "$lib/date/DateEditor.svelte";
import EntityHoverCard from "$lib/EntityHoverCard.svelte";
import { confirmDialog, promptDialog } from "$lib/dialogs.svelte";
import DialogHost from "$lib/DialogHost.svelte";
import loreManifestJson from "../../packages/modules/lore/manifest.json";
import timelineManifestJson from "../../packages/modules/timeline/manifest.json";
import writingManifestJson from "../../packages/modules/writing/manifest.json";
import languageManifestJson from "../../packages/modules/language/manifest.json";
import housesManifestJson from "../../packages/modules/houses/manifest.json";
import FamilyTreeSurface from "$lib/family-tree/FamilyTreeSurface.svelte";
import { formatHouseMemberSummary, houseMemberSummaries } from "$lib/family-tree/fetch.ts";
import {
  familyTreeHistoryKey,
  sameFamilyTreeSession,
  HOUSE_TYPE,
  PERSON_TYPE,
  type FamilyTreeSession,
  type HouseMemberSummary,
} from "$lib/family-tree/model.ts";
import EntityAvatar from "$lib/EntityAvatar.svelte";
import EntityArchiveAction from "$lib/ui-ux/EntityArchiveAction.svelte";
import EntityEmptyState from "$lib/ui-ux/EntityEmptyState.svelte";
import EntityIdentityDialog from "$lib/ui-ux/EntityIdentityDialog.svelte";
import EntityRowActions from "$lib/ui-ux/EntityRowActions.svelte";
import MutationStatus from "$lib/ui-ux/MutationStatus.svelte";
import { archivedToastMessage } from "$lib/ui-ux/archive.ts";
import {
  toAsyncEntityPage,
  toShellSortDirection,
  toShellSortField,
  type AsyncEntityOption,
  type AsyncEntitySearchFn,
  type AsyncEntitySearchQuery,
} from "$lib/ui-ux/asyncEntityQuery.ts";
import { createMutationController } from "$lib/ui-ux/mutationState.ts";
import { ENTITY_ACTIONS } from "$lib/ui-ux/vocabulary.ts";
import type { MutationSnapshot } from "$lib/ui-ux/mutationState.ts";
import {
  Pencil,
  Map as MapIcon,
  Plus,
  Puzzle,
  ShieldCheck,
  X,
  Search,
  ChevronDown,
  ChevronRight,
  UsersRound,
  Sword,
  TreePine,
} from "@lucide/svelte";
import { projectionModule, type ProjectionKind } from "$lib/modules/projections";
import RichTextEditor from "$lib/editor/RichTextEditor.svelte";
import MarkdownArticle from "$lib/markdown/MarkdownArticle.svelte";
import AiProposalPreview from "$lib/ai/AiProposalPreview.svelte";
import { reduceTextGenerationEvent } from "$lib/ai/stream";
import { htmlToMarkdown } from "$lib/markdown";
import {
  applyThemePreference,
  cacheThemePreference,
  normalizeThemePreference,
  readCachedThemePreference,
  type ThemePreference,
} from "$lib/theme";
import {
  formatCalendarDate,
  formatRuntimeTimestampLabel,
  GREGORIAN_CALENDAR_ID,
  isCompleteCalendarDate,
  isGregorianCalendarId,
  parseCalendarDate,
  serializeCalendarDate,
  type CalendarDate,
} from "$lib/date";
import {
  isEmptyFieldValue,
  isStructuredFieldValue,
  restoreStructuredFieldValue,
  shouldPersistFieldValue,
} from "$lib/fields/persistence";

type InstalledModule = ProjectModuleManifest;
type AiFieldSuggestion = { value: unknown; rationale: string; confidence: string };
type RecentProject = { name: string; root: string };
type CreateOption = { key: string; module: InstalledModule; template: EntityTemplate };
type CreateGroup = { module: InstalledModule; options: CreateOption[] };
type CreateField = { namespace: string; field: FieldDefinition; required: boolean };
type CreateDialogView = "templates" | "form";
type WorkbenchPane = "collection" | "content" | "inspector";
type WorkbenchLayoutPrefs = {
  visibility: Record<WorkbenchPane, boolean>;
  collectionWidth: number;
  inspectorWidth: number;
};
const collectionPaneMin = 190;
const collectionPaneMax = 380;
const collectionPaneDefault = 245;
const inspectorPaneMin = 230;
const inspectorPaneMax = 440;
const inspectorPaneDefault = 290;

function workbenchLayoutStorageKey(sec: WorkspaceSection) {
  return `daena:workbench-layout:${workspaceModuleId(sec)}`;
}

function clampWorkbenchPaneWidth(value: number, min: number, max: number, fallback: number) {
  return Number.isFinite(value) && value > 0 ? Math.max(min, Math.min(max, Math.round(value))) : fallback;
}

function loadWorkbenchLayout(sec: WorkspaceSection): WorkbenchLayoutPrefs {
  try {
    const raw = localStorage.getItem(workbenchLayoutStorageKey(sec));
    if (raw) {
      const parsed = JSON.parse(raw) as Partial<WorkbenchLayoutPrefs> & {
        visibility?: Partial<Record<WorkbenchPane, boolean>>;
      };
      return {
        visibility: {
          collection: parsed.visibility?.collection !== false,
          content: parsed.visibility?.content !== false,
          inspector: parsed.visibility?.inspector !== false,
        },
        collectionWidth: clampWorkbenchPaneWidth(
          Number(parsed.collectionWidth),
          collectionPaneMin,
          collectionPaneMax,
          collectionPaneDefault,
        ),
        inspectorWidth: clampWorkbenchPaneWidth(
          Number(parsed.inspectorWidth),
          inspectorPaneMin,
          inspectorPaneMax,
          inspectorPaneDefault,
        ),
      };
    }
  } catch {
    // Fall through to legacy global keys / defaults.
  }
  return {
    visibility: {
      collection: localStorage.getItem("daena:workbench-pane:collection") !== "false",
      content: localStorage.getItem("daena:workbench-pane:content") !== "false",
      inspector: localStorage.getItem("daena:workbench-pane:inspector") !== "false",
    },
    collectionWidth: clampWorkbenchPaneWidth(
      Number(localStorage.getItem("daena:workbench-pane-width:collection")),
      collectionPaneMin,
      collectionPaneMax,
      collectionPaneDefault,
    ),
    inspectorWidth: clampWorkbenchPaneWidth(
      Number(localStorage.getItem("daena:workbench-pane-width:inspector")),
      inspectorPaneMin,
      inspectorPaneMax,
      inspectorPaneDefault,
    ),
  };
}

function saveWorkbenchLayout(sec: WorkspaceSection, layout: WorkbenchLayoutPrefs) {
  try {
    localStorage.setItem(workbenchLayoutStorageKey(sec), JSON.stringify(layout));
  } catch {
    // Ignore quota / private-mode failures; in-session layout still works.
  }
}

type NavigationRenderer = "workspace" | "maps" | "host" | "webview";
type WorkspaceNavigationItem = {
  kind: "workspace";
  plugin: PluginAdminEntry;
  key: string;
  section: WorkspaceSection;
  title: string;
  beta: boolean;
  renderer: "workspace" | "maps";
  view?: PluginAdminEntry["views"][number];
};
type PluginNavigationItem = {
  kind: "plugin";
  plugin: PluginAdminEntry;
  view: PluginAdminEntry["views"][number];
  key: string;
  renderer: Exclude<NavigationRenderer, "workspace">;
};
type NavigationItem = WorkspaceNavigationItem | PluginNavigationItem;

const recentProjectsKey = "daena.recent-projects";
let settingsMigrated = false;
const initialThemePreference = readCachedThemePreference();
let themePreference = $state<ThemePreference>(initialThemePreference);
applyThemePreference(initialThemePreference);

let ready = $state(false);
let error = $state("");
let projectTransitionBusy = $state(false);
let projectTransitionMessage = $state("");
let projectHomeOpen = $state(true);
let shellNavigationHistory = $state<ShellNavigationHistory>(emptyShellNavigationHistory());
let shellNavigationBusy = $state(false);
let shellNavigationRestoring = false;
let section = $state<WorkspaceSection>("lore");
let writingView = $state<WritingView>("manuscripts");
let timelineView = $state<TimelineView>("events");
let languagePane = $state<LanguagePane>("overview");
let housesView = $state<WorkspaceLocationView>("houses");
let calendarDefinitions = $state<Record<string, CalendarDefinition>>({});
/** Bounded calendar entity list from paged queries — avoids scanning the full entity cache. */
let calendarEntities = $state<Entity[]>([]);
let entities = $state<Entity[]>([]);
let selected = $state<Entity | null>(null);
let selectedLoading = $state(false);
let selectedLoadError = $state("");
let documentBody = $state("");
let documentMode = $state<"read" | "edit">("edit");
let fields = $state<Record<string, unknown>>({});
let relationships = $state<Relationship[]>([]);
let eraContexts = $state<EraContext[]>([]);
let createEraContexts = $state<EraContext[]>([]);
let metadataDialog = $state<{ relationship: Relationship; definition: FieldDefinition | null } | null>(null);
let assets = $state<Asset[]>([]);
let assetBusyId = $state<string | null>(null);
let assetDialog = $state<Asset | null>(null);
let entityEditDialog = $state<{ entity: Entity; name: string; entityType: string | null; busy: boolean } | null>(null);
let entityMutationSnapshot = $state<MutationSnapshot>({ phase: "idle", message: "", detail: "" });
const entityMutation = createMutationController({
  get: () => entityMutationSnapshot,
  set: (next) => {
    entityMutationSnapshot = next;
  },
});
type LifecycleToast = { message: string; actionLabel?: string; onAction?: () => void };
let lifecycleToast = $state<LifecycleToast | null>(null);
let lifecycleToastTimer = 0;
let mapLocations = $state<MapLocation[]>([]);
let modules = $state<InstalledModule[]>([]);
let globalQuery = $state("");
let quickOpenOpen = $state(false);
let quickOpenSearchLoading = $state(false);
let filterOpen = $state(false);
let expandedGroups = $state<Set<string>>(new Set());
let railCollapsed = $state(localStorage.getItem("daena:rail-collapsed") === "true");

function loadCollectionQuery(sec: WorkspaceSection): CollectionQuery {
  try {
    const raw = localStorage.getItem(`daena:collection-query:${sec}`);
    if (raw) {
      const parsed = JSON.parse(raw);
      return {
        ...DEFAULT_COLLECTION_QUERY,
        ...parsed,
        pageSize: [25, 50, 100].includes(parsed.pageSize) ? parsed.pageSize : DEFAULT_COLLECTION_QUERY.pageSize,
        section: sec,
        excludedTypes: parsed.excludedTypes ?? [],
      };
    }
  } catch {}
  return {
    ...DEFAULT_COLLECTION_QUERY,
    section: sec,
    viewMode: sec === "language" ? "flat" : DEFAULT_COLLECTION_QUERY.viewMode,
  };
}

let collectionQuery = $state<CollectionQuery>(loadCollectionQuery("lore"));
const emptyEntityPage = (): EntityPage => ({
  items: [],
  total: 0,
  offset: 0,
  limit: DEFAULT_COLLECTION_QUERY.pageSize,
  has_more: false,
  type_counts: [],
});
let collectionPage = $state<EntityPage>(emptyEntityPage());
let collectionLoading = $state(false);
let collectionError = $state("");
let collectionRequest = 0;
/** Bumped to re-query the visible collection page without reloading every entity. */
let collectionRefreshEpoch = $state(0);
let houseCollectionSummaries = $state(new Map<string, HouseMemberSummary>());
let houseSummaryRequest = 0;
let houseSummariesPending = $state(false);

let collectionScopeKey = "";
let collectionQueryRestoring = false;
let collectionListElement = $state<HTMLDivElement | null>(null);
let collectionPaneElement = $state<HTMLElement | null>(null);
let contentPaneElement = $state<HTMLElement | null>(null);
let inspectorPaneElement = $state<HTMLElement | null>(null);
let collectionScrollBySection = $state<Partial<Record<WorkspaceSection, number>>>({});
let pendingCollectionScroll = $state<{ section: WorkspaceSection; scrollTop: number } | null>(null);
let restoredWorkspacePaneDimensions = $state<WorkspacePaneDimensions | null>(null);
let workbenchViewportWidth = $state(window.innerWidth);
const initialWorkbenchLayout = loadWorkbenchLayout("lore");
let workbenchPaneVisibility = $state<Record<WorkbenchPane, boolean>>(initialWorkbenchLayout.visibility);
let workbenchPaneWidths = $state({
  collection: initialWorkbenchLayout.collectionWidth,
  inspector: initialWorkbenchLayout.inspectorWidth,
});
let specializedSurfaceElement = $state<HTMLElement | null>(null);
let specializedSurfaceScrollByKey = $state<Record<string, number>>({});
let pendingSpecializedSurfaceScroll = $state<{ key: string; scrollTop: number } | null>(null);

$effect(() => {
  const q = collectionQuery;
  try {
    const serializable = { ...q, excludedTypes: [...q.excludedTypes] };
    localStorage.setItem(`daena:collection-query:${q.section}`, JSON.stringify(serializable));
  } catch {}
});

$effect(() => {
  collectionQueryRestoring = true;
  collectionQuery = loadCollectionQuery(section);
  expandedGroups = new Set();
  void tick().then(() => {
    collectionQueryRestoring = false;
  });
});

$effect(() => {
  const layout = loadWorkbenchLayout(section);
  workbenchPaneVisibility = layout.visibility;
  workbenchPaneWidths = {
    collection: layout.collectionWidth,
    inspector: layout.inspectorWidth,
  };
  restoredWorkspacePaneDimensions = null;
});
let name = $state("");
let selectedCreateKey = $state("");
let createFieldValues = $state<Record<string, unknown>>({});
let createDateEditorOpen = $state<Record<string, boolean>>({});
let createDateCalendarByField = $state<Record<string, string>>({});
let createDocumentBody = $state("");
let showDiscardPrompt = $state(false);
let pendingCreateDiscard = $state<(() => void) | null>(null);
let isSaving = $state(false);
let savedAt = $state("");
let saveError = $state("");
let editorFullscreen = $state(false);
let hasUnsavedChanges = $state(false);
let autoSaveTimer: number | null = null;
let saveInFlight: Promise<boolean> | null = null;
let saveQueued = false;
let autoSaveFailureCount = 0;
let documentRevision = 0;
let loadedDocumentRevision = "";
let loadedFieldRevisions: Record<string, string> = {};
let loadedStructuredFieldKeys = new Set<string>();
let selectedLoadToken = 0;
let documentConflict = $state<{ paths: string[]; diagnostics: string[] } | null>(null);
let conflictDiskBody = $state("");
let mapSaveStates = $state<Record<string, { status: string; detail: unknown }>>({});
let mapReloadCounter = $state(0);
let mapsEditorKey = $state("welcome");
let mapRecoveryBusy = $state(false);
let mapFocusLinkId = $state<string | null>(null);
let mapFocusFeatureId = $state<string | null>(null);
let mapSelection = $state<unknown | null>(null);
let mapPickPending = $state<
  | null
  | { kind: "link"; entityId: string; role: string; mapEntityId: string }
  | { kind: "rebind"; entityId: string; location: MapLocation; mapEntityId: string }
>(null);
let mapReconcileNotice = $state("");
let mapPickNotice = $state("");
let projectDiagnostics = $state<string[]>([]);
let showSettings = $state(false);
let settingsSurface = $state<"application" | "project">("application");
let settingsSection = $state<SettingsSection>("general");
let projectSection = $state<ProjectSection>("overview");
let archivedCount = $state(0);
let statusCenterOpen = $state(false);
let moduleSchemaOverlay = $state<ModuleSchemaOverlay>({ version: 1 });
let moduleSchemaPackage = $state<{
  schemas: Array<{ namespace: string; entityTypes: EntityTypeDefinition[]; fields: FieldDefinition[] }>;
  templates: EntityTemplate[];
} | null>(null);
let moduleSchemaBusy = $state(false);
let moduleSchemaMessage = $state("");
let moduleSchemaRevision = $state(0);
let schemaPluginId = $state<string | null>(null);
let schemaPluginName = $state("");
let schemaEditorDirty = $state(false);
let schemaOverlayLoadToken = 0;
let schemaOverlayCache = $state<Record<string, ModuleSchemaOverlay>>({});
let schemaEntityCountsByType = $state<Record<string, number>>({});
let schemaEntityCountsLoaded = $state(false);
let displayVersion = $state(appVersionSyncFallback());

const SCHEMA_OVERLAY_CAPABILITY = "schema.overlay";
const MAP_NAVIGATION_SERVICE = "daena.maps/navigation";
const MAP_HOST_SURFACE = "daena.maps/editor";

function enabledServices() {
  return new Set(
    modules
      .filter((module) => module.enabled)
      .flatMap((module) => module.services.provides.map((service) => `${service.name}@${service.major}`)),
  );
}

function serviceAvailable(name: string, major: number) {
  return enabledServices().has(`${name}@${major}`);
}

function mapsEnabled() {
  return serviceAvailable(MAP_NAVIGATION_SERVICE, 1);
}

function schemaOverlayCandidates() {
  return modules
    .filter(
      (module) =>
        module.enabled &&
        (module.capabilities ?? []).includes(SCHEMA_OVERLAY_CAPABILITY) &&
        (module.schemas ?? []).some((schema) => (schema.entityTypes?.length ?? 0) > 0),
    )
    .map((module) => {
      const overlay =
        schemaPluginId === module.id ? moduleSchemaOverlay : (schemaOverlayCache[module.id] ?? { version: 1 });
      const counts = summarizePackageCounts(
        { schemas: module.schemas ?? [], templates: module.templates ?? [] },
        overlay,
      );
      const validation = overlayValidationStatus(overlay);
      return {
        id: module.id,
        name: module.name,
        typeCount: counts.types,
        fieldCount: counts.fields,
        templateCount: counts.templates,
        customization: counts.customized ? ("customized" as const) : ("default" as const),
        validationStatus: validation.status,
        validationMessage: validation.message,
      };
    })
    .sort((left, right) => left.name.localeCompare(right.name));
}

function managedSchemaPlugins() {
  return modules
    .filter(
      (module) =>
        module.enabled &&
        !(module.capabilities ?? []).includes(SCHEMA_OVERLAY_CAPABILITY) &&
        (module.schemas ?? []).some(
          (schema) => (schema.entityTypes?.length ?? 0) > 0 || (schema.fields?.length ?? 0) > 0,
        ),
    )
    .map((module) => ({
      id: module.id,
      name: module.name,
      reason: module.id.includes("maps")
        ? "Maps provider fields stay extension-managed."
        : module.id.includes("language")
          ? "Language keeps a specialized workspace until merged schema rendering is ready."
          : "Schema structure is owned by this extension.",
    }))
    .sort((left, right) => left.name.localeCompare(right.name));
}

function moduleSupportsSchemaOverlay(moduleId: string) {
  return schemaOverlayCandidates().some((candidate) => candidate.id === moduleId);
}

function schemaReferenceEntityTypes(moduleId: string | null) {
  if (!moduleId) return [];
  const module = modules.find((candidate) => candidate.id === moduleId);
  if (!module) return [];
  const required = Object.entries(module.dependencies ?? {})
    .filter(([, dependency]) => dependency.required)
    .map(([id]) => id);
  const types: { id: string; name: string }[] = [];
  const seen = new Set<string>();
  for (const dependencyId of required) {
    const dependency = modules.find((candidate) => candidate.id === dependencyId && candidate.enabled);
    if (!dependency) continue;
    for (const schema of dependency.schemas ?? []) {
      for (const entityType of schema.entityTypes ?? []) {
        if (seen.has(entityType.id)) continue;
        seen.add(entityType.id);
        types.push({ id: entityType.id, name: entityType.name });
      }
    }
  }
  return types;
}
let aiSettings = $state<AiSettings>({
  provider: {
    id: "lm-studio",
    name: "LM Studio",
    adapter: "openai-compatible",
    endpoint: "http://127.0.0.1:1234/v1",
    model: "",
    embeddingModel: "",
    capabilities: [],
  },
  imageProvider: {
    enabled: false,
    id: "comfyui-local",
    name: "ComfyUI",
    adapter: "comfyui",
    endpoint: "http://127.0.0.1:8188",
    model: "",
  },
  consents: [],
});
let aiStatus = $state<AiProviderStatus | null>(null);
let aiModels = $state<string[]>([]);
let aiModelsBusy = $state(false);
let aiModelsMessage = $state("");
let aiModelsMessageTimer: number | null = null;
let aiStatusTimer: number | null = null;
let aiIndexStatus = $state<AiIndexStatus | null>(null);
let aiIndexBusy = $state(false);
let aiIndexMessage = $state("");
let aiIndexMessageTimer: number | null = null;
let aiIndexStatusMessageTimer: number | null = null;
let remoteCredential = $state<{ provider: string; configured: boolean } | null>(null);
let aiUsage = $state<{ inputTokens: number; outputTokens: number; totalTokens: number } | null>(null);
let aiRewriteOpen = $state(false);
let aiBusy = $state(false);
let aiCancelPending = $state(false);
let aiMode = $state<"rewrite" | "generate">("rewrite");
let aiRequestId = $state<string | null>(null);
let aiInstruction = $state("Rewrite this to be more vivid while preserving the meaning.");
let aiStreamText = $state("");
let aiPreviewOutput = $state("");
let aiProgressMessage = $state("Preparing model…");
let aiSourceSelection = $state("");
let aiSourceSelectionPlain = $state("");
let aiGenerationContext = $state("");
let aiSourceBody = $state("");
let aiSourceRevision = $state("");
let aiLastSequence = $state(-1);
let aiUnlisten: (() => void) | null = null;
let editorRef = $state<{
  insertAiTextAtRequest: (value: string) => boolean;
  replaceAiTextWithMarkdown: (value: string) => string | null;
  flushPendingChanges: () => void;
} | null>(null);
let aiFieldFillBusy = $state(false);
let aiFieldFillOpen = $state(false);
let aiFieldFillRequestId = $state<string | null>(null);
let aiFieldFillStream = $state("");
let aiFieldSuggestions = $state<Record<string, AiFieldSuggestion>>({});
let aiFieldUnlisten: (() => void) | null = null;
let aiStartCancelled = $state(false);
let aiFieldFillStartCancelled = $state(false);
let aiSourceEntityId = $state<string | null>(null);
let aiFieldFillEntityId = $state<string | null>(null);
let aiFieldFillLastSequence = $state(-1);
let adminPlugins = $state<PluginAdminEntry[] | null>(null);
let hostView = $state<{ plugin: PluginAdminEntry; view: PluginAdminEntry["views"][number] } | null>(null);
let sandboxView = $state<{
  plugin: PluginAdminEntry;
  view: PluginAdminEntry["views"][number] | null;
  renderer: "maps" | "webview";
} | null>(null);
let familyTreeRootId = $state<string | null>(null);
let familyTreeSession = $state<FamilyTreeSession | null>(null);
let familyTreeRestoreNonce = $state(0);
let projectionView = $state<{
  title: string;
  subtitle: string;
  kind: ProjectionKind;
  module: DaenaModule;
  manifest: ModuleManifest;
} | null>(null);
let loreWikiOpen = $state(false);
let loreWikiEntityId = $state<string | null>(null);
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
let projectStatusEpoch = 0;
let gitStatus = $state<GitStatus | null>(null);
let gitMessage = $state("");
let showProjectMenu = $state(false);
let showExternalImport = $state(false);
let recentProjects = $state<RecentProject[]>([]);
let searchMatches = $state<Entity[] | null>(null);
let mapFeatureMatches = $state<MapFeatureSearchResult[] | null>(null);
let searchRequest = 0;
let showCreateForm = $state(false);
let createDialogView = $state<CreateDialogView>("templates");
let createMoreDetailsOpen = $state(false);
let createBusy = $state(false);
let createDialogElement = $state<HTMLElement | null>(null);
let createDialogReturnFocus: HTMLElement | null = null;
let dateEditorOpen = $state<Record<string, boolean>>({});
let dateCalendarByField = $state<Record<string, string>>({});

const toastDurationMs = 3500;
function showToast(message: string) {
  error = message;
}
function showLifecycleToast(toast: LifecycleToast) {
  if (lifecycleToastTimer) window.clearTimeout(lifecycleToastTimer);
  lifecycleToast = toast;
  lifecycleToastTimer = window.setTimeout(() => {
    lifecycleToast = null;
    lifecycleToastTimer = 0;
  }, toastDurationMs);
}
function dismissLifecycleToast() {
  if (lifecycleToastTimer) window.clearTimeout(lifecycleToastTimer);
  lifecycleToastTimer = 0;
  lifecycleToast = null;
}
$effect(() => {
  if (!error) return;
  const timeout = window.setTimeout(() => {
    error = "";
  }, toastDurationMs);
  return () => window.clearTimeout(timeout);
});
$effect(() => {
  const modalOpen =
    quickOpenOpen ||
    showCreateForm ||
    aiRewriteOpen ||
    editorFullscreen ||
    upgradePreview !== null ||
    confirmAction !== null ||
    deleteTarget !== null ||
    installConsent !== null ||
    deleteBackupPath !== "" ||
    metadataDialog !== null ||
    assetDialog !== null ||
    showExternalImport ||
    entityEditDialog !== null;
  document.body.classList.toggle("modal-open", modalOpen);
  if (!modalOpen) return;
  const onKey = (event: KeyboardEvent) => {
    if (event.key !== "Escape") return;
    if (quickOpenOpen) {
      event.preventDefault();
      closeQuickOpen();
    } else if (showCreateForm) {
      event.preventDefault();
      closeCreateForm();
    } else if (entityEditDialog) {
      event.preventDefault();
      if (!entityEditDialog.busy) closeEntityEditDialog();
    } else if (deleteBackupPath) {
      event.preventDefault();
      deleteBackupPath = "";
    } else if (deleteTarget) {
      event.preventDefault();
      deleteTarget = null;
    } else if (upgradePreview) {
      event.preventDefault();
      upgradePreview = null;
    } else if (installConsent) {
      event.preventDefault();
      installConsent = null;
    } else if (confirmAction) {
      event.preventDefault();
      confirmAction = null;
    } else if (aiRewriteOpen) {
      event.preventDefault();
      closeAiRewrite();
    } else if (metadataDialog) {
      event.preventDefault();
      metadataDialog = null;
    } else if (assetDialog) {
      event.preventDefault();
      assetDialog = null;
    }
  };
  window.addEventListener("keydown", onKey, true);
  return () => {
    window.removeEventListener("keydown", onKey, true);
    document.body.classList.remove("modal-open");
  };
});

const activeModuleId = () =>
  section === "lore"
    ? "daena.lore"
    : section === "timeline"
      ? "daena.timeline"
      : section === "writing"
        ? "daena.writing"
        : section === "language"
          ? "daena.language"
          : section === "houses"
            ? "daena.houses"
            : "daena.maps";
const activeManifest = () => {
  const fromProject = modules.find((module) => module.id === activeModuleId());
  if (fromProject) return fromProject as unknown as ModuleManifest;
  return (section === "lore"
    ? loreManifestJson
    : section === "timeline"
      ? timelineManifestJson
      : section === "writing"
        ? writingManifestJson
        : section === "language"
          ? languageManifestJson
          : section === "houses"
            ? housesManifestJson
            : null) as unknown as ModuleManifest | null;
};
const workspaceSectionOrder: WorkspaceSection[] = ["lore", "timeline", "writing", "language", "maps", "houses"];
function workspaceDescription(target: WorkspaceSection) {
  return workspaceSectionDescription(target);
}
function manifestForWorkspaceSection(target: WorkspaceSection): ModuleManifest | null {
  const moduleId = workspaceModuleId(target);
  const fromProject = modules.find((module) => module.id === moduleId);
  if (fromProject) return fromProject as unknown as ModuleManifest;
  if (target === "lore") return loreManifestJson as unknown as ModuleManifest;
  if (target === "timeline") return timelineManifestJson as unknown as ModuleManifest;
  if (target === "writing") return writingManifestJson as unknown as ModuleManifest;
  if (target === "language") return languageManifestJson as unknown as ModuleManifest;
  if (target === "houses") return housesManifestJson as unknown as ModuleManifest;
  return null;
}
function schemaEntityTypeIds(schema: { entityTypes: EntityTypeDefinition[] }): string[] {
  return schema.entityTypes.map((entityType) => entityType.id);
}
function sectionEntityTypeDefs(target: WorkspaceSection) {
  return (
    manifestForWorkspaceSection(target)?.schemas.flatMap((schema) =>
      schema.entityTypes.map((entityType) => ({ id: entityType.id, name: entityType.name })),
    ) ?? []
  );
}
function collectionTabsFor(target: WorkspaceSection) {
  return workspaceCollectionTabs(target, sectionEntityTypeDefs(target));
}
function activeCollectionTab(target: WorkspaceSection = section) {
  const tabs = collectionTabsFor(target);
  const id = target === "timeline" ? timelineView : target === "writing" ? writingView : null;
  return (id ? tabs.find((tab) => tab.id === id) : undefined) ?? tabs[0];
}
function applyCollectionTabForEntityType(entityType: string | null) {
  const owner = sectionForEntityType(entityType) ?? section;
  const tab = collectionTabForEntityType(collectionTabsFor(owner), entityType);
  if (!tab) return;
  if (owner === "writing") writingView = tab.id;
  if (owner === "timeline") timelineView = tab.id;
}
function workspaceViewNavItems() {
  return workspaceSectionViewNav(section, sectionEntityTypeDefs(section));
}
function enabledWorkspaceSections() {
  return workspaceSectionOrder.filter((target) =>
    modules.some((module) => module.id === workspaceModuleId(target) && module.enabled),
  );
}
function workspaceSectionLabel(target: WorkspaceSection) {
  return target === "lore"
    ? "Lore library"
    : target === "timeline"
      ? "Timeline"
      : target === "writing"
        ? "Writing Studio"
        : target === "language"
          ? "Languages"
          : target === "houses"
            ? "Houses"
            : "Maps";
}
function entityTypePresentation(entityType: string | null): {
  definition: EntityTypeDefinition;
  pluginId: string;
} | null {
  if (!entityType) return null;
  for (const module of modules) {
    for (const schema of module.schemas) {
      const definition = schema.entityTypes.find((candidate) => candidate.id === entityType);
      if (definition) return { definition, pluginId: module.id };
    }
  }
  return null;
}
function iconForEntityType(entityType: string | null): {
  icon: IconRef;
  pluginId: string | null;
  iconColor: EntityTypeColor;
} {
  const presentation = entityTypePresentation(entityType);
  return presentation
    ? {
        icon: presentation.definition.icon,
        pluginId: presentation.pluginId,
        iconColor: presentation.definition.iconColor,
      }
    : { icon: FALLBACK_ICON, pluginId: null, iconColor: DEFAULT_TYPE_COLOR };
}
function workspaceEntityCount(target: WorkspaceSection) {
  const entityTypes = new Set(manifestForWorkspaceSection(target)?.schemas.flatMap(schemaEntityTypeIds) ?? []);
  return entities.filter((entity) => !entity.deleted && entityTypes.has(entity.entity_type ?? "")).length;
}
function recentlyUpdatedEntities() {
  return [...entities]
    .filter((entity) => !entity.deleted)
    .sort((left, right) => Date.parse(right.updated_at) - Date.parse(left.updated_at))
    .slice(0, 6);
}
function updatedDateLabel(timestamp: string) {
  return formatRuntimeTimestampLabel(timestamp, { dateStyle: "medium" });
}
function viewRenderer(
  plugin: PluginAdminEntry,
  view: PluginAdminEntry["views"][number],
): Exclude<NavigationRenderer, "workspace"> {
  if (view.renderer?.type === "host-surface") {
    if (view.renderer.id === MAP_HOST_SURFACE && view.renderer.major === 1) return "maps";
    return "webview";
  }
  if (view.renderer?.type === "sandboxed") return "webview";
  if (view.renderer?.type === "declarative") return "host";
  return plugin.kind === "sandboxed" ? "webview" : "host";
}
function workspaceNavigationItems(): WorkspaceNavigationItem[] {
  return workspaceSectionOrder.flatMap((target) => {
    const plugin = (adminPlugins ?? []).find(
      (candidate) =>
        candidate.id === workspaceModuleId(target) && candidate.enabled && candidate.lifecycle.state === "active",
    );
    if (!plugin) return [];
    const view =
      target === "maps"
        ? plugin.views.find(
            (candidate) => viewRenderer(plugin, candidate) === "maps" && candidate.renderer?.type === "host-surface",
          )
        : undefined;
    return [
      {
        kind: "workspace",
        plugin,
        key: `workspace:${plugin.id}`,
        section: target,
        title: workspaceSectionLabel(target),
        beta: target === "maps",
        renderer: target === "maps" && view ? "maps" : "workspace",
        ...(view ? { view } : {}),
      },
    ];
  });
}
function enabledEntityTypes() {
  return new Set(
    modules.filter((module) => module.enabled).flatMap((module) => module.schemas.flatMap(schemaEntityTypeIds)),
  );
}
function fieldAppliesToEntity(field: FieldDefinition, entityType?: string | null, moduleId = activeModuleId()) {
  return fieldAppliesToEnabledTypes(
    field,
    entityType,
    modules.length === 0 ? null : enabledEntityTypes(),
    schemaPluginId === moduleId ? moduleSchemaOverlay : null,
  );
}
function availableEditTypes(): string[] {
  const types = new Set<string>();
  for (const mod of modules) {
    if (!mod.enabled) continue;
    for (const schema of mod.schemas) for (const t of schema.entityTypes) types.add(t.id);
  }
  // Keep current selection visible even if its type is disabled/custom or from maps
  if (selected?.entity_type) types.add(selected.entity_type);
  if (entityEditDialog?.entity.entity_type) types.add(entityEditDialog.entity.entity_type);
  // Ensure map type is selectable if maps module exists in any state
  const hasMapDecl = modules.some((m) =>
    m.schemas.some((s) => schemaEntityTypeIds(s).includes("daena.maps:world-map")),
  );
  if (hasMapDecl) types.add("daena.maps:world-map");
  return [...types].sort((a, b) => entityTypeLabel(a).localeCompare(entityTypeLabel(b)));
}
function groupedEditTypes(): Array<{ heading: string; types: string[] }> {
  const enabled = new Set<string>();
  for (const mod of modules) {
    if (!mod.enabled) continue;
    for (const schema of mod.schemas) for (const t of schema.entityTypes) enabled.add(t.id);
  }
  if (selected?.entity_type) enabled.add(selected.entity_type);
  if (entityEditDialog?.entity.entity_type) enabled.add(entityEditDialog.entity.entity_type);
  if (modules.some((m) => m.schemas.some((s) => schemaEntityTypeIds(s).includes("daena.maps:world-map"))))
    enabled.add("daena.maps:world-map");
  const groups: Array<{ heading: string; types: string[] }> = [];
  for (const sec of workspaceSectionOrder) {
    const manifest = manifestForWorkspaceSection(sec);
    if (!manifest) continue;
    const secTypes = manifest.schemas.flatMap(schemaEntityTypeIds).filter((t) => enabled.has(t));
    if (secTypes.length === 0) continue;
    secTypes.sort((a, b) => entityTypeLabel(a).localeCompare(entityTypeLabel(b)));
    groups.push({ heading: workspaceSectionLabel(sec), types: secTypes });
    for (const t of secTypes) enabled.delete(t);
  }
  if (enabled.size > 0) {
    const remaining = [...enabled].sort((a, b) => entityTypeLabel(a).localeCompare(entityTypeLabel(b)));
    groups.push({ heading: "Other", types: remaining });
  }
  return groups;
}
function sectionForEntityType(entityType: string | null): WorkspaceSection | null {
  if (!entityType) return null;
  for (const target of workspaceSectionOrder) {
    const types = manifestForWorkspaceSection(target)?.schemas.flatMap(schemaEntityTypeIds) ?? [];
    if (types.includes(entityType)) return target;
  }
  // Custom type not mapped to a known section - keep current section
  return null;
}
function editTypeWarning(): string | null {
  if (!entityEditDialog) return null;
  const from = entityEditDialog.entity.entity_type;
  const to = entityEditDialog.entityType;
  if (from === to) return null;
  // Maps and physical events have locked provider fields
  if (from === "daena.maps:world-map" || to === "daena.maps:world-map") {
    return "Maps store provider fields that only apply to maps. Changing away will hide map layers and source, changing into a map cannot restore them.";
  }
  if (from === "daena.language:language" || to === "daena.language:language") {
    return "Languages own lexemes, phonemes and grammar records. Those records require the language type and will become read-only if the type changes.";
  }
  // Generic field/relationship hiding
  const hasPopulated = Object.entries(fields).some(([, v]) => !isEmptyFieldValue(v)) || relationships.length > 0;
  if (hasPopulated)
    return "Fields and relationships that don't apply to the new type will be hidden but preserved. You can revert the type to restore them.";
  return "The entry will move to the collection for the new type.";
}
const definitions = () => {
  const entityType =
    selected?.entity_type ??
    (section === "timeline" || section === "writing" ? activeCollectionTab()?.entityTypes[0] : undefined);
  const active = activeManifest();
  if (!active) return [];
  return contributedRelationshipFields<FieldDefinition>(
    active,
    entityType,
    modules.filter((module) => module.enabled),
    modules.length === 0 ? null : enabledEntityTypes(),
    schemaPluginId === active.id ? moduleSchemaOverlay : null,
  );
};
function namespaceForField(definition: FieldDefinition): string {
  const manifest = activeManifest();
  return primarySchemaNamespace(manifest?.schemas, {
    entityType: selected?.entity_type,
    fieldKey: definition.key,
    fallback: activeModuleId(),
  });
}
function fieldRevisionKey(namespace: string, key: string) {
  return `${namespace}\u0000${key}`;
}
function fieldDisplayValue(value: unknown): string {
  if (Array.isArray(value)) return value.map((item) => fieldDisplayValue(item)).join(", ");
  if (typeof value === "object" && value !== null) {
    try {
      return JSON.stringify(value);
    } catch {
      return "";
    }
  }
  return String(value ?? "");
}
function fieldInputValue(definition: FieldDefinition, value: unknown): string | number | boolean | string[] {
  if (definition.type === "boolean") return value === true;
  if (definition.type === "number") return typeof value === "number" && Number.isFinite(value) ? value : "";
  if ((definition as any).type === "oneof") return fieldDisplayValue(value);
  if (definition.multiple) return Array.isArray(value) ? value.map((item) => String(item)) : [];
  return fieldDisplayValue(value);
}
function fieldValueForSave(definition: FieldDefinition, value: unknown) {
  if (definition.type === "number") {
    if (value === "" || value === null || value === undefined) return "";
    const numberValue = typeof value === "number" ? value : Number(value);
    return Number.isFinite(numberValue) ? numberValue : "";
  }
  if (definition.type === "boolean") return value === true || value === "true";
  if ((definition as any).type === "oneof") return value;
  if (definition.multiple) return Array.isArray(value) ? value.map((item) => String(item)) : [];
  return value;
}
function aiScalarValue(definition: FieldDefinition, raw: unknown): unknown | null {
  if (definition.type === "number") {
    const value = typeof raw === "number" ? raw : typeof raw === "string" ? Number(raw.trim()) : Number.NaN;
    return Number.isFinite(value) ? value : null;
  }
  if (definition.type === "boolean") {
    if (typeof raw === "boolean") return raw;
    if (raw === "true") return true;
    if (raw === "false") return false;
    return null;
  }
  if (definition.type === "enum") {
    return typeof raw === "string" && definition.options?.includes(raw) ? raw : null;
  }
  if ((definition as any).type === "oneof") {
    const opts =
      definition.options ??
      ((definition as any).oneOf as Array<{ options?: string[] }> | undefined)?.flatMap((v) => v.options ?? []) ??
      [];
    return typeof raw === "string" && opts.includes(raw) ? raw : null;
  }
  if (definition.type === "date") {
    if (typeof raw !== "string") return null;
    const date = parseCalendarDate(raw.trim());
    return date ? serializeCalendarDate(date) : null;
  }
  return typeof raw === "string" && raw.trim() ? raw : null;
}
function coerceAiFieldValue(definition: FieldDefinition, raw: unknown): unknown | null {
  const isOne = (definition as any).cardinality === "one";
  const isRelationship = definition.type === "relationship";
  const isMultiple = definition.multiple || (isRelationship && !isOne);
  if (isMultiple || isRelationship) {
    // For cardinality "one", allow single string as well as array with 1
    if (isOne && typeof raw === "string" && raw.trim()) {
      return raw.trim();
    }
    if (!Array.isArray(raw) || raw.length === 0 || raw.length > 5) {
      // For "one", also allow single string case already handled, so fail for array
      if (isOne && typeof raw === "string") return null;
      return null;
    }
    if (isOne && raw.length > 1) return null;
    const values = raw.map((item) =>
      isRelationship ? (typeof item === "string" && item.trim() ? item : null) : aiScalarValue(definition, item),
    );
    return values.every((value) => value !== null) ? values : null;
  }
  return aiScalarValue(definition, raw);
}
function aiJsonValueSchema(definition: FieldDefinition) {
  const isOne = (definition as any).cardinality === "one";
  const scalarType = definition.type === "number" ? "number" : definition.type === "boolean" ? "boolean" : "string";
  const isOneOf = (definition as any).type === "oneof";
  const enumOptions = isOneOf
    ? (((definition as any).oneOf as Array<{ options?: string[] }> | undefined)?.flatMap((v) => v.options ?? []) ??
      definition.options)
    : definition.options;
  const scalar: any = {
    type: scalarType,
    ...(definition.type === "enum" && definition.options?.length ? { enum: definition.options } : {}),
    ...(isOneOf && (enumOptions as string[])?.length ? { enum: enumOptions } : {}),
  };
  const isMulti = definition.multiple || (definition.type === "relationship" && !isOne);
  return isMulti || definition.type === "relationship"
    ? { type: "array", items: scalar, maxItems: isOne ? 1 : 5, uniqueItems: true }
    : scalar;
}
function suggestionDisplayValue(key: string, suggestion: AiFieldSuggestion) {
  const definition = definitions().find((candidate) => candidate.key === key);
  if (definition?.type !== "relationship" || !Array.isArray(suggestion.value))
    return fieldDisplayValue(suggestion.value);
  const names = new Map(entities.map((entity) => [entity.id, entity.name]));
  return suggestion.value.map((id) => names.get(String(id)) ?? String(id)).join(", ");
}
function suggestionConfidenceTone(confidence: string) {
  const normalized = confidence.trim().toLowerCase();
  return normalized === "high" || normalized === "medium" || normalized === "low" ? normalized : "unknown";
}
function suggestionConfidenceLabel(confidence: string) {
  const tone = suggestionConfidenceTone(confidence);
  return tone.charAt(0).toUpperCase() + tone.slice(1);
}
function createOptions(): CreateOption[] {
  return modules
    .filter((module) => module.enabled)
    .flatMap((module) =>
      module.templates.map((template) => ({ key: `${module.id}:${template.id}`, module, template })),
    );
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
function iconForCreateOption(option: CreateOption): { icon: IconRef; pluginId: string; iconColor: EntityTypeColor } {
  const entityType = option.module.schemas
    .flatMap((schema) => schema.entityTypes)
    .find((candidate) => candidate.id === option.template.entityType);
  return {
    icon: option.template.icon ?? entityType?.icon ?? FALLBACK_ICON,
    pluginId: option.module.id,
    iconColor: entityType?.iconColor ?? DEFAULT_TYPE_COLOR,
  };
}
function selectedCreateOption() {
  return createOptions().find((option) => option.key === selectedCreateKey) ?? null;
}
function defaultCreateOption(options: CreateOption[]) {
  const moduleId = workspaceModuleId(section);
  const tabTypes = section === "timeline" || section === "writing" ? (activeCollectionTab()?.entityTypes ?? []) : null;
  return (
    options.find(
      (option) => option.module.id === moduleId && (tabTypes ? tabTypes.includes(option.template.entityType) : true),
    ) ??
    options.find((option) => tabTypes?.includes(option.template.entityType)) ??
    options[0] ??
    null
  );
}
function createFieldsFor(option: CreateOption | null = selectedCreateOption()): CreateField[] {
  if (!option) return [];
  return option.module.schemas
    .filter((schema) => schemaEntityTypeIds(schema).includes(option.template.entityType))
    .flatMap((schema) =>
      schema.fields
        .filter((field) => fieldAppliesToEntity(field, option.template.entityType, option.module.id))
        .map((field) => ({
          namespace: schema.namespace,
          field,
          required: option.template.requiredFields?.includes(field.key) ?? false,
        })),
    );
}
function createRelationshipValues(key: string) {
  const value = createFieldValues[key];
  return Array.isArray(value) ? value.filter((targetId): targetId is string => typeof targetId === "string") : [];
}
function setCreateRelationshipValues(key: string, values: string[]) {
  createFieldValues = { ...createFieldValues, [key]: values };
  if (key === "era") {
    void loadEraContexts(values).then((contexts) => {
      createEraContexts = contexts;
      hintCreateDateCalendarsFromEras(contexts);
    });
  }
}
function defaultCreateFieldValue(field: FieldDefinition, template: EntityTemplate) {
  if (Object.prototype.hasOwnProperty.call(template.fields, field.key)) return template.fields[field.key];
  return field.type === "boolean"
    ? false
    : field.type === "relationship" || (field.type === "enum" && field.multiple)
      ? []
      : "";
}
function resetCreateFields(option: CreateOption | null) {
  createFieldValues = Object.fromEntries(
    createFieldsFor(option).map(({ field }) => [field.key, defaultCreateFieldValue(field, option!.template)]),
  );
  createDateEditorOpen = {};
  createDateCalendarByField = {};
  createEraContexts = [];
  createDocumentBody = option?.template.document ?? "";
  createMoreDetailsOpen = false;
}
function requiredCreateFields(option: CreateOption | null = selectedCreateOption()) {
  return createFieldsFor(option).filter((item) => item.required && !isChronologyPropertyField(item.field));
}
function optionalCreateFields(option: CreateOption | null = selectedCreateOption()) {
  return createFieldsFor(option).filter((item) => !item.required && !isChronologyPropertyField(item.field));
}
function chronologyCreateFields(option: CreateOption | null = selectedCreateOption()) {
  const items = createFieldsFor(option).filter((item) => isChronologyPropertyField(item.field));
  return [
    ...items.filter((item) => isEraRelationshipField(item.field)),
    ...items.filter((item) => item.field.type === "date"),
  ];
}
function createChronologyWarnings() {
  return chronologyWarnings(
    chronologyCreateFields()
      .filter((item) => item.field.type === "date")
      .map((item) => ({ label: item.field.label, value: createFieldValues[item.field.key] })),
    createEraContexts,
  );
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
function isCreateDropdownField(field: FieldDefinition) {
  return field.type === "enum";
}
function hasCreateValues() {
  const hasNonDropdownValues = createFieldsFor().some(
    ({ field }) => !isCreateDropdownField(field) && isCreateValuePopulated(createFieldValues[field.key]),
  );
  return Boolean(name.trim() || createDocumentBody.trim() || hasNonDropdownValues);
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
function createDateForField(key: string) {
  return parseCalendarDate(createFieldValues[key]);
}
function worldCalendars() {
  return calendarEntities;
}
function calendarDefinitionForId(calendarId: string | undefined): CalendarDefinition | null {
  if (isGregorianCalendarId(calendarId)) return null;
  return calendarDefinitions[calendarId!] ?? null;
}
function calendarIdForStoredDate(date: Partial<CalendarDate> | null | undefined, fallback: string | undefined): string {
  if (date?.calendar) return date.calendar;
  return fallback || GREGORIAN_CALENDAR_ID;
}
function createDateDraftForField(key: string): Partial<CalendarDate> | null {
  return (
    createDateForField(key) ??
    (createDateEditorOpen[key]
      ? { calendar: calendarIdForStoredDate(null, createDateCalendarByField[key]), era: "CE", precision: "day" }
      : null)
  );
}
function createCalendarDefinition(key: string): CalendarDefinition | null {
  return calendarDefinitionForId(calendarIdForStoredDate(createDateForField(key), createDateCalendarByField[key]));
}
function createDatePartsDraft(key: string) {
  const stored = createDateForField(key);
  const calendar = createCalendarDefinition(key);
  if (stored) return calendarDateToParts(stored, calendar);
  return createDateEditorOpen[key]
    ? {
        year: undefined as number | undefined,
        month: undefined as number | undefined,
        day: undefined as number | undefined,
        precision: "day" as const,
      }
    : null;
}
function setCreateDateCalendar(key: string, calendarId: string) {
  createDateCalendarByField = { ...createDateCalendarByField, [key]: calendarId };
  const previous = createDateForField(key);
  if (!previous) return;
  setCreateField(key, serializeCalendarDate({ ...previous, calendar: calendarId }));
}
function openCreateDateEditor(key: string) {
  createDateEditorOpen = { ...createDateEditorOpen, [key]: true };
  createDateCalendarByField = {
    ...createDateCalendarByField,
    [key]: createDateCalendarByField[key] ?? GREGORIAN_CALENDAR_ID,
  };
  setCreateField(key, "");
}
function updateCreateDateField(key: string, patch: Partial<CalendarDate>) {
  const calendar = createCalendarDefinition(key);
  const calendarId = calendarIdForStoredDate(createDateForField(key), createDateCalendarByField[key]);
  const previous = createDateForField(key);
  const currentParts = calendarDateToParts(
    previous ?? { calendar: calendarId, era: "CE", year: 1, precision: "year" },
    calendar,
  ) ?? { year: 1, precision: "year" as const };
  const nextParts = { ...currentParts, ...patch };
  if ((patch as Record<string, unknown>).precision === undefined) {
    const hasMonth = (nextParts as Record<string, unknown>).month !== undefined;
    const hasDay = (nextParts as Record<string, unknown>).day !== undefined;
    if (!hasMonth) {
      nextParts.precision = "year";
      delete (nextParts as Record<string, unknown>).day;
    } else if (!hasDay) {
      nextParts.precision = "month";
    } else {
      if (nextParts.precision !== "hour" && nextParts.precision !== "minute" && nextParts.precision !== "second") {
        nextParts.precision = "day";
      }
    }
  }
  if (patch.precision === "year") {
    delete nextParts.month;
    delete nextParts.day;
  }
  if (patch.precision === "month") {
    delete nextParts.day;
    if (nextParts.month === undefined) nextParts.month = 1;
  }
  if (patch.precision === "day") {
    nextParts.month ??= 1;
    nextParts.day ??= 1;
  }
  const stored = partsToCalendarDate(nextParts, calendar);
  stored.calendar = calendarId;
  if (previous) {
    stored.hour = patch.hour ?? previous.hour;
    stored.minute = patch.minute ?? previous.minute;
    stored.second = patch.second ?? previous.second;
  } else if (patch.hour !== undefined) {
    stored.hour = patch.hour;
    stored.minute = patch.minute;
    stored.second = patch.second;
  }
  if (patch.precision === "hour" || patch.precision === "minute" || patch.precision === "second") {
    stored.precision = patch.precision;
  }
  setCreateField(key, serializeCalendarDate(stored));
}
function updateCreateDatePart(key: string, part: "year" | "month" | "day", raw: string, min: number, max?: number) {
  if (!raw.trim()) {
    if (part === "month") {
      updateCreateDateField(key, { precision: "year" });
    } else if (part === "day") {
      updateCreateDateField(key, { precision: "month" });
    } else if (part === "year") {
      clearCreateDateField(key);
    }
    return;
  }
  const parsed = Math.floor(Number(raw));
  if (!Number.isFinite(parsed)) return;
  updateCreateDateField(key, { [part]: Math.min(max ?? parsed, Math.max(min, parsed)) });
}
function updateCreateDateTime(key: string, raw: string) {
  const [hour, minute, second] = raw.split(":").map(Number);
  if (![hour, minute, second].every(Number.isFinite)) return;
  updateCreateDateField(key, { hour, minute, second, precision: "second" });
}
function clearCreateDateField(key: string) {
  setCreateField(key, "");
  createDateEditorOpen = { ...createDateEditorOpen, [key]: false };
  const next = { ...createDateCalendarByField };
  delete next[key];
  createDateCalendarByField = next;
}

function contextFor(currentSection = section): ModuleContext {
  if (!projectInfo?.root) throw new Error("No project is open");
  const moduleId =
    currentSection === "lore"
      ? "daena.lore"
      : currentSection === "timeline"
        ? "daena.timeline"
        : currentSection === "writing"
          ? "daena.writing"
          : currentSection === "language"
            ? "daena.language"
            : currentSection === "houses"
              ? "daena.houses"
              : "daena.maps";
  const fromProject = modules.find((module) => module.id === moduleId);
  const fallback =
    currentSection === "lore"
      ? loreManifestJson
      : currentSection === "timeline"
        ? timelineManifestJson
        : currentSection === "writing"
          ? writingManifestJson
          : currentSection === "language"
            ? languageManifestJson
            : currentSection === "houses"
              ? housesManifestJson
              : languageManifestJson;
  return buildModuleContext((fromProject ?? fallback) as unknown as ModuleManifest, projectInfo.root, {
    availableServices: enabledServices(),
  });
}

function sectionEnabled() {
  return modules.find((module) => module.id === activeModuleId())?.enabled ?? false;
}

async function closeNativePluginWebviews() {
  try {
    await project.closeAllPluginWebviews();
  } catch {
    // The webview cleanup is best effort so browser previews remain usable.
  }
}

async function resolveDirtyMapSession(): Promise<boolean> {
  const mapId = currentMapId();
  if (!mapId || sandboxView?.renderer !== "maps" || mapSaveStates[mapId]?.status !== "dirty") return true;
  if (mapsEditorMode === "physical") return true;
  if (
    await confirmDialog({
      title: "Save changes to this map before leaving?",
      message: "Your map has unsaved edits.",
      confirmLabel: "Save",
    })
  ) {
    try {
      await nativeVectorSession()?.save();
      return nativeVectorSession()?.isDirty() !== true && mapSaveStates[mapId]?.status !== "dirty";
    } catch (cause) {
      error = friendlyError(cause);
      return false;
    }
  }
  return confirmDialog({
    title: "Discard unsaved map edits?",
    message: "Choose Cancel to remain in the editor.",
    confirmLabel: "Discard",
    danger: true,
  });
}

async function leavePluginView(): Promise<boolean> {
  if (!(await resolveDirtyMapSession())) return false;
  editorFullscreen = false;
  hostView = null;
  sandboxView = null;
  projectionView = null;
  await closeNativePluginWebviews();
  return true;
}

function pluginViewLabel(item: PluginNavigationItem) {
  return item.plugin.name === item.view.title ? item.plugin.name : `${item.plugin.name} · ${item.view.title}`;
}

function workspaceNavigationActive(target: WorkspaceSection) {
  if (projectHomeOpen) return false;
  if (target === "maps") {
    return (
      section === "maps" &&
      !hostView &&
      !projectionView &&
      (!sandboxView || sandboxView.plugin.id === workspaceModuleId(target))
    );
  }
  return !hostView && !sandboxView && !projectionView && section === target;
}

function pluginNavigationActive(item: PluginNavigationItem) {
  if (item.renderer === "host") {
    return hostView?.plugin.id === item.plugin.id && hostView.view.id === item.view.id;
  }
  return sandboxView?.plugin.id === item.plugin.id && sandboxView.view?.id === item.view.id;
}

function navigationActive(item: NavigationItem) {
  return item.kind === "workspace" ? workspaceNavigationActive(item.section) : pluginNavigationActive(item);
}

function currentWorkspaceLocationView(): WorkspaceLocationView {
  if (section === "lore") {
    if (loreWikiOpen) return "wiki";
    if (projectionView?.kind === "graph") return "graph";
    return "library";
  }
  if (section === "timeline") {
    if (projectionView?.kind === "timeline") return "timeline";
    return timelineView;
  }
  if (section === "writing") return writingView;
  if (section === "houses") return housesView;
  return "default";
}

function currentModuleState(): Record<string, unknown> | null {
  if (section === "language") return { pane: languagePane };
  if (section === "houses" && housesView === "tree")
    return (familyTreeSession as Record<string, unknown> | null) ?? null;
  return null;
}

function restoreModuleState(section: WorkspaceSection, state: Record<string, unknown> | null | undefined) {
  if (section === "language") {
    if (state && typeof state.pane === "string") {
      const pane = state.pane as LanguagePane;
      if (["overview", "lexicon", "sounds", "writing", "grammar", "forms", "samples"].includes(pane)) {
        languagePane = pane;
        return;
      }
    }
    languagePane = "overview";
  }
  if (section === "houses") {
    familyTreeSession = (state as FamilyTreeSession | null) ?? null;
  }
}

function currentWorkspaceCollectionLocation(): WorkspaceCollectionLocation {
  const pendingScrollTop = pendingCollectionScroll?.section === section ? pendingCollectionScroll.scrollTop : null;
  return {
    query: {
      textSearch: collectionQuery.textSearch,
      sortField: collectionQuery.sortField,
      sortDir: collectionQuery.sortDir,
      pageSize: collectionQuery.pageSize,
      page: collectionQuery.page,
      excludedTypes: [...collectionQuery.excludedTypes].sort(),
      viewMode: collectionQuery.viewMode,
    },
    expandedGroups: [...expandedGroups].sort(),
    scrollTop: pendingScrollTop ?? collectionListElement?.scrollTop ?? collectionScrollBySection[section] ?? 0,
  };
}

function measuredPaneWidth(element: HTMLElement | null, fallback: number) {
  const width = element?.getBoundingClientRect().width ?? 0;
  return width > 0 ? Math.round(width) : fallback;
}

function currentWorkspacePaneDimensions(): WorkspacePaneDimensions {
  const fallback = restoredWorkspacePaneDimensions;
  return {
    collectionWidth: measuredPaneWidth(collectionPaneElement, fallback?.collectionWidth ?? 0),
    contentWidth: measuredPaneWidth(contentPaneElement, fallback?.contentWidth ?? 0),
    inspectorWidth: measuredPaneWidth(inspectorPaneElement, fallback?.inspectorWidth ?? 0),
    viewportWidth: typeof window === "undefined" ? (fallback?.viewportWidth ?? 0) : window.innerWidth,
  };
}

async function restoreWorkspacePaneDimensions(panes: WorkspacePaneDimensions) {
  const viewportWidth = typeof window === "undefined" ? panes.viewportWidth : window.innerWidth;
  restoredWorkspacePaneDimensions =
    panes.viewportWidth > 0 && Math.abs(panes.viewportWidth - viewportWidth) <= 2 ? { ...panes } : null;
  await tick();
}

function persistWorkbenchLayout() {
  saveWorkbenchLayout(section, {
    visibility: workbenchPaneVisibility,
    collectionWidth: workbenchPaneWidths.collection,
    inspectorWidth: workbenchPaneWidths.inspector,
  });
}

function activePaneDimensions(): WorkspacePaneDimensions {
  return {
    collectionWidth: restoredWorkspacePaneDimensions?.collectionWidth || workbenchPaneWidths.collection,
    contentWidth: restoredWorkspacePaneDimensions?.contentWidth || 640,
    inspectorWidth: restoredWorkspacePaneDimensions?.inspectorWidth || workbenchPaneWidths.inspector,
    viewportWidth: typeof window === "undefined" ? 0 : window.innerWidth,
  };
}

function workbenchSupportsInspector() {
  return section !== "language" && (section !== "maps" || sandboxView?.renderer !== "maps");
}

function resizeWorkbenchPane(pane: "collection" | "inspector", delta: number) {
  const panes = activePaneDimensions();
  if (pane === "collection")
    panes.collectionWidth = Math.max(collectionPaneMin, Math.min(collectionPaneMax, panes.collectionWidth + delta));
  else panes.inspectorWidth = Math.max(inspectorPaneMin, Math.min(inspectorPaneMax, panes.inspectorWidth + delta));
  workbenchPaneWidths = {
    collection: panes.collectionWidth,
    inspector: panes.inspectorWidth,
  };
  restoredWorkspacePaneDimensions = panes;
  persistWorkbenchLayout();
}

function resetWorkbenchPane(pane: "collection" | "inspector") {
  const panes = activePaneDimensions();
  if (pane === "collection") panes.collectionWidth = collectionPaneDefault;
  else panes.inspectorWidth = inspectorPaneDefault;
  workbenchPaneWidths = {
    collection: panes.collectionWidth,
    inspector: panes.inspectorWidth,
  };
  restoredWorkspacePaneDimensions = panes;
  persistWorkbenchLayout();
}

function toggleWorkbenchPane(pane: WorkbenchPane) {
  const visible = !workbenchPaneVisibility[pane];
  workbenchPaneVisibility = { ...workbenchPaneVisibility, [pane]: visible };
  persistWorkbenchLayout();
  if (pane === "content" && !visible) editorFullscreen = false;
}

function workspaceGridStyle() {
  if (mapSurfaceOpen) return undefined;
  const panes = activePaneDimensions();
  const collectionVisible = workbenchPaneVisibility.collection;
  const contentVisible = workbenchPaneVisibility.content;
  const inspectorVisible = workbenchPaneVisibility.inspector && workbenchSupportsInspector();
  const columns: string[] = [];
  if (workbenchViewportWidth <= 1180) {
    if (collectionVisible && contentVisible) return "grid-template-columns: 220px minmax(320px, 1fr)";
    if (collectionVisible && !contentVisible && inspectorVisible) return "grid-template-columns: 220px minmax(0, 1fr)";
    return "grid-template-columns: minmax(0, 1fr)";
  }
  if (collectionVisible) columns.push(`${Math.round(panes.collectionWidth)}px`);
  if (collectionVisible && contentVisible) columns.push("10px");
  if (contentVisible) columns.push("minmax(360px, 1fr)");
  if (contentVisible && inspectorVisible) columns.push("10px");
  if (inspectorVisible) columns.push(`${Math.round(panes.inspectorWidth)}px`);
  return `grid-template-columns: ${columns.length ? columns.join(" ") : "minmax(0, 1fr)"}`;
}

function rememberCollectionScroll() {
  if (!collectionListElement) return;
  collectionScrollBySection[section] = collectionListElement.scrollTop;
}

function queueCollectionScroll(target: WorkspaceSection, scrollTop: number) {
  collectionScrollBySection[target] = scrollTop;
  pendingCollectionScroll = { section: target, scrollTop };
}

async function applyWorkspaceCollectionLocation(location: WorkspaceCollectionLocation) {
  collectionQueryRestoring = true;
  collectionQuery = {
    section,
    ...location.query,
    excludedTypes: [...location.query.excludedTypes],
  };
  expandedGroups = new Set(location.expandedGroups);
  queueCollectionScroll(section, location.scrollTop);
  await tick();
  collectionQueryRestoring = false;
}

function specializedSurfaceKey(): string | null {
  if (hostView) return `plugin:${hostView.plugin.id}:${hostView.view.id}`;
  if (sandboxView?.view && sandboxView.renderer !== "maps") {
    return `plugin:${sandboxView.plugin.id}:${sandboxView.view.id}`;
  }
  if (loreWikiOpen) return "workspace:lore:wiki";
  if (projectionView?.kind === "graph") return "workspace:lore:graph";
  if (projectionView?.kind === "timeline") return "workspace:timeline:timeline";
  if (section === "houses" && housesView === "tree") return "workspace:houses:tree";
  return null;
}

function specializedSurfaceKeyForLocation(location: ShellLocation): string | null {
  if (location.kind === "plugin") return `plugin:${location.key}`;
  if (location.kind !== "workspace") return null;
  if (
    location.view === "wiki" ||
    location.view === "graph" ||
    location.view === "timeline" ||
    location.view === "tree"
  ) {
    return `workspace:${location.section}:${location.view}`;
  }
  return null;
}

function currentSpecializedSurfaceScrollTop() {
  const key = specializedSurfaceKey();
  if (!key) return 0;
  return pendingSpecializedSurfaceScroll?.key === key
    ? pendingSpecializedSurfaceScroll.scrollTop
    : (specializedSurfaceElement?.scrollTop ?? specializedSurfaceScrollByKey[key] ?? 0);
}

function rememberSpecializedSurfaceScroll(scrollTop: number) {
  const key = specializedSurfaceKey();
  if (!key) return;
  specializedSurfaceScrollByKey[key] = scrollTop;
  if (pendingSpecializedSurfaceScroll?.key === key) pendingSpecializedSurfaceScroll = null;
}

async function restoreSpecializedSurfaceScroll(location: ShellLocation) {
  if (location.kind === "home" || location.kind === "settings" || location.kind === "project") return;
  const key = specializedSurfaceKeyForLocation(location);
  if (!key) return;
  pendingSpecializedSurfaceScroll = { key, scrollTop: location.surfaceScrollTop };
  specializedSurfaceScrollByKey[key] = location.surfaceScrollTop;
  await tick();
  await new Promise<void>((resolve) => window.requestAnimationFrame(() => resolve()));
  if (specializedSurfaceKey() !== key || !specializedSurfaceElement) return;
  specializedSurfaceElement.scrollTop = location.surfaceScrollTop;
  specializedSurfaceScrollByKey[key] = specializedSurfaceElement.scrollTop;
  pendingSpecializedSurfaceScroll = null;
}

function currentShellLocation(): ShellLocation {
  if (showSettings && settingsSurface === "project") return { kind: "project", section: projectSection };
  if (showSettings) return { kind: "settings", section: settingsSection };
  if (projectHomeOpen) return { kind: "home" };
  if (hostView) {
    return {
      kind: "plugin",
      key: `${hostView.plugin.id}:${hostView.view.id}`,
      section,
      entityId: selected?.id ?? null,
      surfaceScrollTop: currentSpecializedSurfaceScrollTop(),
    };
  }
  if (sandboxView?.view) {
    return {
      kind: "plugin",
      key: `${sandboxView.plugin.id}:${sandboxView.view.id}`,
      section,
      entityId: selected?.id ?? null,
      surfaceScrollTop: currentSpecializedSurfaceScrollTop(),
      moduleState: null,
    };
  }
  return {
    kind: "workspace",
    section,
    view: currentWorkspaceLocationView(),
    entityId: section === "houses" && housesView === "tree" ? familyTreeRootId : (selected?.id ?? null),
    writingView,
    timelineView,
    collection: currentWorkspaceCollectionLocation(),
    panes: currentWorkspacePaneDimensions(),
    surfaceScrollTop: currentSpecializedSurfaceScrollTop(),
    moduleState: currentModuleState(),
  };
}

function recordCurrentShellLocation() {
  if (!ready || shellNavigationRestoring) return;
  shellNavigationHistory = recordShellLocation(shellNavigationHistory, currentShellLocation());
}

function recordShellDeparture(location: ShellLocation) {
  if (!ready || shellNavigationRestoring) return;
  shellNavigationHistory = recordShellLocation(shellNavigationHistory, location);
}

async function openFamilyTreePerson(entityId: string) {
  let entity = entities.find((candidate) => candidate.id === entityId) ?? null;
  if (!entity) {
    try {
      entity = await project.getEntity(entityId);
      if (entity && !entities.some((candidate) => candidate.id === entity!.id)) entities = [...entities, entity];
    } catch (cause) {
      error = friendlyError(cause);
      return;
    }
  }
  if (!entity || entity.deleted) return;
  if (!(await leavePluginView())) return;
  section = "lore";
  projectHomeOpen = false;
  loreWikiOpen = false;
  await selectEntity(entity);
}

async function openHostView(
  plugin: PluginAdminEntry,
  view: PluginAdminEntry["views"][number],
  departure = currentShellLocation(),
) {
  if (!(await dismissSettings())) return;
  if (!(await leavePluginView())) return;
  recordShellDeparture(departure);
  projectHomeOpen = false;
  hostView = { plugin, view };
}

function pluginViews(): PluginNavigationItem[] {
  const workspaceViewKeys = new Set(
    workspaceNavigationItems()
      .filter((item): item is WorkspaceNavigationItem & { view: PluginAdminEntry["views"][number] } =>
        Boolean(item.view),
      )
      .map((item) => `${item.plugin.id}:${item.view.id}`),
  );
  return (adminPlugins ?? [])
    .filter((plugin) => plugin.enabled && plugin.lifecycle.state === "active")
    .flatMap((plugin) =>
      plugin.views
        .filter((view) => !workspaceViewKeys.has(`${plugin.id}:${view.id}`))
        .filter(
          (view) =>
            view.renderer?.type === "host-surface" || plugin.kind === "sandboxed" || (view.components?.length ?? 0) > 0,
        )
        .map(
          (view) =>
            ({
              kind: "plugin",
              plugin,
              view,
              key: `${plugin.id}:${view.id}`,
              renderer: viewRenderer(plugin, view),
            }) satisfies PluginNavigationItem,
        ),
    )
    .sort((left, right) => left.view.title.localeCompare(right.view.title));
}

function mapsNavigationItem(): PluginNavigationItem | null {
  const workspace = workspaceNavigationItems().find((item) => item.renderer === "maps");
  if (workspace?.view) {
    return {
      kind: "plugin",
      plugin: workspace.plugin,
      view: workspace.view,
      key: `${workspace.plugin.id}:${workspace.view.id}`,
      renderer: "maps",
    };
  }
  return pluginViews().find((item) => item.renderer === "maps") ?? null;
}

function pluginNavigationItemByKey(key: string): PluginNavigationItem | null {
  const maps = mapsNavigationItem();
  if (maps?.key === key) return maps;
  return pluginViews().find((item) => item.key === key) ?? null;
}

async function openNavigationItem(item: NavigationItem) {
  if (item.kind === "workspace") {
    await switchSection(item.section);
    return;
  }
  await openPluginView(item);
}

function openSidebarNavigationItem(key: string) {
  const item = [...workspaceNavigationItems(), ...pluginViews()].find((candidate) => candidate.key === key);
  if (item) void openNavigationItem(item);
}

function updateRailCollapsed(collapsed: boolean) {
  railCollapsed = collapsed;
  localStorage.setItem("daena:rail-collapsed", String(collapsed));
}

async function openPluginView(item: PluginNavigationItem, departure = currentShellLocation()) {
  if (!(await flushAutoSave())) return;
  if (item.renderer === "maps") {
    if (!(await dismissSettings())) return;
    const mapId = currentMapId();
    if (sandboxView?.renderer === "maps") {
      const mapsWelcome = sandboxView.view === null;
      if ((mapId === null && mapsWelcome) || (mapId !== null && !mapsWelcome)) return;
    }
    if (mapId) {
      const mapField = (await project.listFields(mapId)).find(
        (field) => field.namespace === "maps" && field.key === "map",
      );
      const descriptor = mapField?.value as { provider?: { id?: string } } | undefined;
      mapsEditorMode = "vector";
    }
    mapFocusLinkId = null;
    mapFocusFeatureId = null;
    if (!(await leavePluginView())) return;
    recordShellDeparture(departure);
    projectHomeOpen = false;
    loreWikiOpen = false;
    section = "maps";
    sandboxView = mapId
      ? { plugin: item.plugin, view: item.view, renderer: "maps" }
      : { plugin: item.plugin, view: null, renderer: "maps" };
    if (mapId) mapsEditorKey = mapId;
    else if (!sandboxView.view) mapsEditorKey = "welcome";
    return;
  }
  if (item.renderer === "host") {
    await openHostView(item.plugin, item.view, departure);
    return;
  }
  if (!(await dismissSettings())) return;
  if (!(await leavePluginView())) return;
  recordShellDeparture(departure);
  projectHomeOpen = false;
  sandboxView = { plugin: item.plugin, view: item.view, renderer: "webview" };
}

let mapsEditorMode = $state<"vector" | "physical">("vector");
let mapsVectorStart = $state<"import" | "geojson">("geojson");
let mapProviderMenuOpen = $state<"header" | "empty" | null>(null);
const mapSurfaceOpen = $derived(section === "maps" && sandboxView?.renderer === "maps" && Boolean(sandboxView?.view));
async function createMap(provider: "image" | "vector" | "physical" = "physical") {
  if (projectDiagnostics.length > 0) return;
  try {
    if (!(await flushAutoSave())) return;
    const departure = currentShellLocation();
    mapProviderMenuOpen = null;
    const mapView = mapsNavigationItem();
    if (!mapView) throw new Error("The Maps plugin view is not available");
    if (!(await dismissSettings())) return;
    if (!(await leavePluginView())) return;
    recordShellDeparture(departure);
    selected = null;
    fields = {};
    relationships = [];
    assets = [];
    mapLocations = [];
    mapFocusLinkId = null;
    mapFocusFeatureId = null;
    projectHomeOpen = false;
    loreWikiOpen = false;
    section = "maps";
    // Draft editor: no map entity until the native editor accepts/creates one.
    mapsEditorMode = provider === "physical" ? "physical" : "vector";
    mapsVectorStart = provider === "image" ? "import" : "geojson";
    mapsEditorKey = `draft-${Date.now()}`;
    sandboxView = { plugin: mapView.plugin, view: mapView.view, renderer: "maps" };
  } catch (cause) {
    error = friendlyError(cause);
  }
}

function currentMapId() {
  return selected?.entity_type === "daena.maps:world-map" ? selected.id : null;
}

$effect(() => {
  if (!mapProviderMenuOpen) return;
  const handlePointerDown = (event: PointerEvent) => {
    const target = event.target as Element | null;
    if (target?.closest(".map-provider-menu") || target?.closest('[aria-haspopup="menu"]')) return;
    mapProviderMenuOpen = null;
  };
  document.addEventListener("pointerdown", handlePointerDown, true);
  return () => document.removeEventListener("pointerdown", handlePointerDown, true);
});

type SavedMapEntry = Entity & { size: number };
let savedMapsCache = $state<SavedMapEntry[] | null>(null);
let savedMapsRequest = 0;
$effect(() => {
  const mapsWorkspaceOpen = section === "maps";
  void entities;
  if (!ready || !mapsWorkspaceOpen) {
    savedMapsCache = null;
    return;
  }
  const request = ++savedMapsRequest;
  void (async () => {
    try {
      const maps: Entity[] = [];
      let offset = 0;
      while (true) {
        const page = await project.queryEntities({
          entityTypes: ["daena.maps:world-map"],
          sortField: "updated_at",
          sortDirection: "desc",
          offset,
          limit: 200,
        });
        maps.push(...page.items);
        if (!page.has_more) break;
        offset += page.items.length;
      }
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
        // Legacy drafts without a source asset are not openable; drop them quietly.
        if (!sourceId || mapAssets.length === 0) {
          if (!sourceId && mapAssets.length === 0) {
            await project.deleteEntity(map.id).catch(() => undefined);
          }
          continue;
        }
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

let languageListRefreshed = false;
$effect(() => {
  if (!ready || section !== "language") {
    languageListRefreshed = false;
    return;
  }
  if (languageListRefreshed) return;
  languageListRefreshed = true;
  bumpCollectionRefresh();
  if (!selected && collectionResult().entities.length > 0) void selectEntity(collectionResult().entities[0], false);
});

function savedMaps() {
  return savedMapsCache ?? [];
}

async function saveCurrentMap() {
  try {
    if (mapsEditorMode === "physical") return;
    await nativeVectorSession()?.save();
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

function mapConflictDetail(detail: unknown): { path?: string } {
  return typeof detail === "object" && detail !== null ? (detail as { path?: string }) : {};
}

function sectionEntityTypes(): string[] {
  return manifestForWorkspaceSection(section)?.schemas.flatMap(schemaEntityTypeIds) ?? [];
}

function activeTabEntityTypes(): string[] | undefined {
  if (section !== "writing" && section !== "timeline") return undefined;
  return activeCollectionTab()?.entityTypes ?? [];
}

$effect(() => {
  if (!modules.length) return;
  const timelineTabs = collectionTabsFor("timeline");
  if (timelineTabs.length > 0 && !timelineTabs.some((tab) => tab.id === timelineView)) {
    timelineView = timelineTabs[0].id;
  }
  const writingTabs = collectionTabsFor("writing");
  if (writingTabs.length > 0 && !writingTabs.some((tab) => tab.id === writingView)) {
    writingView = writingTabs[0].id;
  }
});

$effect(() => {
  const entityTypes = collectionEntityTypes({
    entityTypes: new Set(sectionEntityTypes()),
    tabEntityTypes: activeTabEntityTypes(),
  });
  const scopeKey = JSON.stringify([
    section,
    entityTypes,
    collectionQuery.textSearch.trim(),
    collectionQuery.sortField,
    collectionQuery.sortDir,
    collectionQuery.pageSize,
    collectionQuery.excludedTypes,
  ]);
  if (collectionScopeKey && collectionScopeKey !== scopeKey && !collectionQueryRestoring) {
    collectionPage = emptyEntityPage();
    if (collectionQuery.page !== 0) collectionQuery.page = 0;
  }
  collectionScopeKey = scopeKey;
});

$effect(() => {
  void collectionRefreshEpoch;
  const active = ready && Boolean(projectInfo);
  const page = collectionQuery.page;
  const limit = collectionQuery.pageSize;
  const entityTypes = collectionEntityTypes({
    entityTypes: new Set(sectionEntityTypes()),
    tabEntityTypes: activeTabEntityTypes(),
  });
  const query = collectionQuery.textSearch.trim();
  const excludedEntityTypes = [...collectionQuery.excludedTypes];
  const sortField = collectionQuery.sortField;
  const sortDirection = collectionQuery.sortDir;
  const request = ++collectionRequest;
  collectionError = "";
  if (!active) {
    collectionPage = emptyEntityPage();
    collectionLoading = false;
    return;
  }
  collectionLoading = true;
  const timer = window.setTimeout(
    () => {
      void project
        .queryEntities({
          query: query || undefined,
          entityTypes,
          excludedEntityTypes,
          sortField,
          sortDirection,
          offset: page * limit,
          limit,
        })
        .then((result) => {
          if (request !== collectionRequest) return;
          if (page > 0 && result.items.length === 0 && result.total > 0) {
            collectionQuery.page = Math.max(0, Math.ceil(result.total / limit) - 1);
            return;
          }
          collectionPage = result;
          for (const entity of result.items) upsertEntityInCache(entity);
        })
        .catch((cause) => {
          if (request !== collectionRequest) return;
          collectionPage = emptyEntityPage();
          collectionError = friendlyError(cause);
        })
        .finally(() => {
          if (request === collectionRequest) collectionLoading = false;
        });
    },
    query ? 180 : 0,
  );
  return () => window.clearTimeout(timer);
});

$effect(() => {
  const housesSection = ready && section === "houses" && housesView === "houses";
  const ids = collectionPage.items
    .filter((entity) => {
      const type = entity.entity_type ?? "";
      return type === HOUSE_TYPE || type.endsWith(":house");
    })
    .map((entity) => entity.id);
  void collectionRefreshEpoch;
  if (!housesSection || ids.length === 0) {
    if (!housesSection) houseCollectionSummaries = new Map();
    houseSummariesPending = false;
    return;
  }
  const request = ++houseSummaryRequest;
  houseSummariesPending = true;
  const context = buildModuleContext(housesManifestJson as unknown as ModuleManifest, projectInfo?.root ?? "", {
    availableServices: enabledServices(),
  });
  void houseMemberSummaries(context, ids)
    .then((summaries) => {
      if (request !== houseSummaryRequest) return;
      houseCollectionSummaries = summaries;
    })
    .catch(() => {
      if (request !== houseSummaryRequest) return;
      houseCollectionSummaries = new Map();
    })
    .finally(() => {
      if (request === houseSummaryRequest) houseSummariesPending = false;
    });
});

$effect(() => {
  const pending = pendingCollectionScroll;
  void collectionPage.items;
  if (
    !pending ||
    collectionLoading ||
    collectionQuery.section !== pending.section ||
    section !== pending.section ||
    projectHomeOpen ||
    showSettings ||
    hostView ||
    sandboxView ||
    projectionView ||
    loreWikiOpen
  )
    return;
  pendingCollectionScroll = null;
  void tick().then(() => {
    if (!collectionListElement || section !== pending.section) return;
    collectionListElement.scrollTop = pending.scrollTop;
    collectionScrollBySection[pending.section] = collectionListElement.scrollTop;
  });
});

function toggleTypeFilter(type: string) {
  const idx = collectionQuery.excludedTypes.indexOf(type);
  if (idx >= 0) {
    collectionQuery.excludedTypes = collectionQuery.excludedTypes.filter((t) => t !== type);
  } else {
    collectionQuery.excludedTypes = [...collectionQuery.excludedTypes, type];
  }
}

function toggleGroup(type: string) {
  const next = new Set(expandedGroups);
  if (next.has(type)) next.delete(type);
  else next.add(type);
  expandedGroups = next;
}

function collectionResult(): CollectionResult {
  return presentCollectionPage(collectionPage, collectionQuery.viewMode, entityTypeLabel);
}

async function selectSearchResult(entity: Entity) {
  if (!(await flushAutoSave())) return;
  const departure = currentShellLocation();
  if (!(await leavePluginView())) return;
  recordShellDeparture(departure);
  const owner = workspaceSectionOrder.find((target) =>
    manifestForWorkspaceSection(target)?.schemas.some((schema) =>
      schemaEntityTypeIds(schema).includes(entity.entity_type ?? ""),
    ),
  );
  section = owner && owner !== "maps" ? owner : "lore";
  applyCollectionTabForEntityType(entity.entity_type);
  projectHomeOpen = false;
  globalQuery = "";
  collectionQuery.textSearch = "";
  await selectEntity(entity, false);
}

async function switchSection(next: WorkspaceSection) {
  if (!(await flushAutoSave())) return;
  if (section === next && !projectHomeOpen && (next !== "maps" || sandboxView?.renderer === "maps") && !showSettings)
    return;
  if (section === "language" && next !== "language" && !(await canLeaveLanguageSection())) return;
  const departure = currentShellLocation();
  if (!(await dismissSettings())) return;
  if (!(await leavePluginView())) return;
  recordShellDeparture(departure);
  const sectionChanged = section !== next;
  section = next;
  queueCollectionScroll(next, collectionScrollBySection[next] ?? 0);
  projectHomeOpen = false;
  if (sectionChanged) {
    clearSelection();
    collectionQuery.textSearch = "";
  }
}

async function openProjectHome() {
  if (!ready || (projectHomeOpen && !showSettings && !hostView && !sandboxView && !projectionView)) return;
  if (!(await flushAutoSave())) return;
  if (section === "language" && !(await canLeaveLanguageSection())) return;
  const departure = currentShellLocation();
  if (!(await dismissSettings())) return;
  if (!(await leavePluginView())) return;
  recordShellDeparture(departure);
  projectHomeOpen = true;
  globalQuery = "";
}

async function reconcileWorkspaceSection() {
  if (enabledWorkspaceSections().includes(section)) return;
  if (!(await flushAutoSave())) return;
  if (!(await leavePluginView())) return;
  section = enabledWorkspaceSections()[0] ?? "lore";
  clearSelection();
  collectionQuery.textSearch = "";
  editorFullscreen = false;
}

async function switchWritingView(next: WritingView) {
  const tabs = collectionTabsFor("writing");
  const resolved = tabs.some((tab) => tab.id === next) ? next : (tabs[0]?.id ?? next);
  if (!(await flushAutoSave())) return;
  if (writingView === resolved && !projectHomeOpen) return;
  const departure = currentShellLocation();
  if (!(await leavePluginView())) return;
  recordShellDeparture(departure);
  projectHomeOpen = false;
  writingView = resolved;
  clearSelection();
  collectionQuery.textSearch = "";
}

async function switchTimelineView(next: TimelineView) {
  const tabs = collectionTabsFor("timeline");
  const resolved = tabs.some((tab) => tab.id === next) ? next : (tabs[0]?.id ?? next);
  if (!(await flushAutoSave())) return;
  if (timelineView === resolved && !projectHomeOpen && !projectionView) return;
  const departure = currentShellLocation();
  if (!(await leavePluginView())) return;
  recordShellDeparture(departure);
  projectHomeOpen = false;
  timelineView = resolved;
  clearSelection();
  collectionQuery.textSearch = "";
}

async function switchHousesView(next: WorkspaceLocationView) {
  const allowed: WorkspaceLocationView[] = ["houses", "tree"];
  const resolved = allowed.includes(next) ? next : "houses";
  if (!(await flushAutoSave())) return;
  if (section === "houses" && housesView === resolved && !projectHomeOpen) {
    if (resolved === "tree" && familyTreeRootId) {
      recordShellDeparture(currentShellLocation());
      familyTreeRootId = null;
      familyTreeSession = null;
    }
    return;
  }
  const departure = currentShellLocation();
  if (!(await leavePluginView())) return;
  recordShellDeparture(departure);
  projectHomeOpen = false;
  if (section !== "houses") section = "houses";
  housesView = resolved;
}

async function switchLanguagePane(next: LanguagePane) {
  if (!(await flushAutoSave())) return;
  if (languagePane === next && !projectHomeOpen && section === "language") return;
  const departure = currentShellLocation();
  if (!(await leavePluginView())) return;
  recordShellDeparture(departure);
  projectHomeOpen = false;
  languagePane = next;
}

async function canLeaveLanguageSection(): Promise<boolean> {
  if (section !== "language") return true;
  try {
    const fn = (window as unknown as Record<string, unknown>).__daena_canLeaveLanguage as
      (() => Promise<boolean> | boolean) | undefined;
    if (typeof fn === "function") return await fn();
  } catch {}
  return true;
}

function dismissTransientMenus(): boolean {
  let dismissed = false;
  if (filterOpen) {
    filterOpen = false;
    dismissed = true;
  }
  if (showProjectMenu) {
    showProjectMenu = false;
    dismissed = true;
  }
  if (quickOpenOpen) {
    closeQuickOpen();
    dismissed = true;
  }
  if (showCreateForm) {
    closeCreateForm();
    // closeCreateForm may prompt; treat as dismissed
    dismissed = true;
  }
  return dismissed;
}

function sectionLabel() {
  if (showSettings) {
    if (settingsSurface === "application") return settingsSection === "ai" ? "Settings · AI" : "Settings";
    if (projectSection === "data") return "Project · Data & recovery";
    if (projectSection === "extensions") return "Project · Extensions";
    if (projectSection === "fields") return "Project · Fields & Types";
    if (projectSection === "snapshots") return "Project · Snapshots";
    if (projectSection === "archive") return "Project · Archive";
    if (projectSection === "advanced") return "Project · Advanced";
    return "Project";
  }
  if (projectHomeOpen) return "Home";
  return section === "lore"
    ? "Lore library"
    : section === "timeline"
      ? "Timeline"
      : section === "writing"
        ? "Writing Studio"
        : section === "language"
          ? "Languages"
          : section === "houses"
            ? "Houses"
            : "Maps";
}

function breadcrumbItems() {
  const items = ["Private studio", sectionLabel()];
  if ((section === "writing" || section === "timeline") && !projectHomeOpen) {
    const tab = activeCollectionTab();
    items.push(
      tab ? (tab.id === "reference" ? "Reference pages" : tab.label) : section === "writing" ? "Writing" : "Events",
    );
  }
  if (selected && !projectHomeOpen && !showSettings) items.push(selected.name);
  return items;
}

function workspaceHeadingKicker() {
  return section === "lore"
    ? "WORLD BIBLE"
    : section === "timeline"
      ? "CHRONOLOGY"
      : section === "maps"
        ? "MAP ATLAS"
        : section === "houses"
          ? "HOUSES"
          : section === "language"
            ? "LANGUAGE WORKSHOP"
            : "DRAFTING DESK";
}

function workspaceHeadingDescription() {
  if (section === "lore") return "A living reference for every person, place, and power.";
  if (section === "maps") return "Keep every map beside its notes, links, and provider source.";
  if (section === "houses") return "Manage Houses and explore kinship in the family tree.";
  if (section === "language") return "Words, sounds, writing, and grammar for every language of your world.";
  const tab = activeCollectionTab();
  if (section === "timeline") {
    if (tab?.id === "calendars") return "Optional ways to name years, months, weeks, and seasons.";
    if (tab?.id === "events" || !tab) return "Events, eras, and the threads that connect them.";
    return `${tab.label} in the chronology of your world.`;
  }
  if (tab?.id === "manuscripts") return "Draft stories, essays, and other long-form work.";
  if (tab?.id === "reference") return "Build the pages, notes, and references behind the story.";
  return tab
    ? `Draft and keep ${tab.label.toLowerCase()} beside the world they draw from.`
    : "Manuscripts and reference pages beside the world they draw from.";
}

function collectionLabel() {
  if (section === "lore") return "entries";
  if (section === "language") return "languages";
  if (section === "maps") return "maps";
  if (section === "houses") return "houses";
  const tab = activeCollectionTab();
  if (!tab) return "entries";
  if (tab.id === "reference") return "reference pages";
  return tab.label.toLowerCase();
}

function collectionKicker() {
  if (section === "lore") return "LORE LIBRARY";
  if (section === "maps") return "MAPS";
  if (section === "houses") return "HOUSES";
  if (section === "language") return "LANGUAGES";
  const tab = activeCollectionTab();
  if (section === "timeline" && (tab?.id === "events" || !tab)) return "TIMELINE";
  if (!tab) return "ENTRIES";
  if (tab.id === "reference") return "REFERENCE PAGES";
  return tab.label.toUpperCase();
}

function createLabel() {
  if (section === "lore") return "entry";
  if (section === "language") return "language";
  if (section === "maps") return "map";
  if (section === "houses") return "house";
  const tab = activeCollectionTab();
  if (!tab) return "entry";
  if (tab.id === "events") return "event";
  if (tab.id === "calendars") return "calendar";
  if (tab.id === "manuscripts") return "manuscript";
  if (tab.id === "reference") return "reference page";
  return tab.label.toLowerCase();
}

function emptyEditorKicker() {
  if (section === "lore") return "LORE ENTRY";
  if (section === "maps") return "MAP";
  if (section === "houses") return "HOUSE";
  const tab = activeCollectionTab();
  if (!tab) return "ENTRY";
  if (tab.id === "events") return "TIMELINE EVENT";
  if (tab.id === "calendars") return "CALENDAR";
  if (tab.id === "manuscripts") return "MANUSCRIPT";
  if (tab.id === "reference") return "REFERENCE PAGE";
  return tab.label.toUpperCase();
}

function entityTypeLabel(entityType: string | null) {
  if (!entityType) return "Uncategorized";
  return entityTypePresentation(entityType)?.definition.name ?? entityType;
}

async function openProjection() {
  if (!(await flushAutoSave())) return;
  const expectedKind = section === "lore" ? "graph" : "timeline";
  if (!projectHomeOpen && !showSettings && projectionView?.kind === expectedKind) return;
  const departure = currentShellLocation();
  if (!(await leavePluginView())) return;
  recordShellDeparture(departure);
  projectHomeOpen = false;
  loreWikiOpen = false;
  loreWikiEntityId = null;
  const projection = projectionModule(section === "lore" ? "lore" : "timeline");
  projectionView = {
    title: projection.title,
    subtitle: projection.subtitle,
    kind: projection.kind,
    module: projection.module,
    manifest: (manifestForWorkspaceSection(section) ?? projection.module.manifest) as ModuleManifest,
  };
}

async function openLoreLibrary() {
  if (!(await flushAutoSave())) return;
  if (!projectHomeOpen && !showSettings && !loreWikiOpen && !projectionView && !hostView && !sandboxView) return;
  const departure = currentShellLocation();
  if (!(await dismissSettings())) return;
  if (!(await leavePluginView())) return;
  recordShellDeparture(departure);
  projectHomeOpen = false;
  loreWikiOpen = false;
  loreWikiEntityId = null;
}

async function openLoreWiki() {
  if (!(await flushAutoSave())) return;
  if (!projectHomeOpen && !showSettings && loreWikiOpen && !hostView && !sandboxView) return;
  const departure = currentShellLocation();
  if (!(await leavePluginView())) return;
  recordShellDeparture(departure);
  projectHomeOpen = false;
  loreWikiEntityId =
    selected?.entity_type && allEntityTypesForSection("lore").has(selected.entity_type) ? selected.id : null;
  loreWikiOpen = true;
  projectionView = null;
}

function allEntityTypesForSection(target: WorkspaceSection) {
  return new Set(manifestForWorkspaceSection(target)?.schemas.flatMap(schemaEntityTypeIds) ?? []);
}

async function closeLoreWiki() {
  if (!(await flushAutoSave())) return;
  const departure = currentShellLocation();
  recordShellDeparture(departure);
  loreWikiOpen = false;
  loreWikiEntityId = null;
}

async function closeProjectionView() {
  if (!(await flushAutoSave())) return;
  const departure = currentShellLocation();
  recordShellDeparture(departure);
  projectionView = null;
}

async function closePluginView() {
  if (!(await flushAutoSave())) return;
  const departure = currentShellLocation();
  if (!(await leavePluginView())) return;
  recordShellDeparture(departure);
}

async function openWorkspaceView(view: WorkspaceLocationView) {
  if (view === "library") return openLoreLibrary();
  if (view === "wiki") return openLoreWiki();
  if (view === "graph" || view === "timeline") return openProjection();
  if (view === "houses" || view === "tree") return switchHousesView(view);
  if (section === "timeline" && collectionTabsFor("timeline").some((tab) => tab.id === view)) {
    return switchTimelineView(view);
  }
  if (section === "writing" && collectionTabsFor("writing").some((tab) => tab.id === view)) {
    return switchWritingView(view);
  }
  if (section === "houses") return switchHousesView(view);
}

async function restoreShellEntity(entityId: string | null) {
  if (!entityId) {
    if (selected) clearSelection();
    return null;
  }
  let entity = entities.find((candidate) => candidate.id === entityId) ?? null;
  if (!entity) {
    try {
      const loaded = await project.getEntity(entityId);
      entity = loaded;
      if (loaded && !entities.some((candidate) => candidate.id === loaded.id)) entities = [...entities, loaded];
    } catch {
      entity = null;
    }
  }
  if (!entity || entity.deleted) {
    if (selected) clearSelection();
    return null;
  }
  await selectEntity(entity, false);
  return entity;
}

async function restoreShellLocation(target: ShellLocation): Promise<boolean> {
  const pluginItem = target.kind === "plugin" ? pluginNavigationItemByKey(target.key) : null;
  if (target.kind === "plugin" && !pluginItem) return false;
  shellNavigationRestoring = true;
  try {
    if (target.kind === "home") {
      await openProjectHome();
    } else if (target.kind === "settings") {
      await openSettings(target.section);
    } else if (target.kind === "project") {
      await openProjectCenter(target.section);
    } else if (target.kind === "plugin") {
      await switchSection(target.section);
      await restoreShellEntity(target.entityId);
      await openPluginView(pluginItem!);
      await restoreSpecializedSurfaceScroll(target);
    } else {
      await switchSection(target.section);
      if (target.section === "lore") {
        await openLoreLibrary();
      } else if (target.section === "timeline") {
        await switchTimelineView(target.timelineView);
      } else if (target.section === "writing") {
        await switchWritingView(target.writingView);
      } else if (target.section === "houses") {
        if (target.view === "tree") {
          familyTreeRootId = target.entityId;
          familyTreeRestoreNonce += 1;
        }
        restoreModuleState(target.section, target.moduleState);
        await switchHousesView(target.view);
      } else if (target.section === "language") {
        restoreModuleState(target.section, target.moduleState);
      }
      await tick();
      await applyWorkspaceCollectionLocation(target.collection);
      const restoredEntity =
        target.section === "houses" && target.view === "tree" ? null : await restoreShellEntity(target.entityId);
      if (target.view === "wiki" || target.view === "graph" || target.view === "timeline") {
        await openWorkspaceView(target.view);
      }
      await restoreWorkspacePaneDimensions(target.panes);
      await restoreSpecializedSurfaceScroll(target);
      const expected =
        (target.section === "houses" && target.view === "tree") || restoredEntity || !target.entityId
          ? target
          : { ...target, entityId: null };
      return sameShellLocation(currentShellLocation(), expected);
    }
    return sameShellLocation(currentShellLocation(), target);
  } finally {
    shellNavigationRestoring = false;
  }
}

function shellLocationAvailable(target: ShellLocation) {
  if (target.kind === "workspace") return enabledWorkspaceSections().includes(target.section);
  if (target.kind === "plugin") return pluginNavigationItemByKey(target.key) !== null;
  return true;
}

async function navigateShellHistory(direction: "back" | "forward") {
  if (shellNavigationBusy) return;
  if (dismissTransientMenus()) return;
  const current = currentShellLocation();
  let history = shellNavigationHistory;
  let transition = direction === "back" ? shellHistoryBack(history, current) : shellHistoryForward(history, current);
  while (transition && !shellLocationAvailable(transition.target)) {
    history =
      direction === "back"
        ? { ...history, back: history.back.slice(0, -1) }
        : { ...history, forward: history.forward.slice(1) };
    shellNavigationHistory = history;
    transition = direction === "back" ? shellHistoryBack(history, current) : shellHistoryForward(history, current);
  }
  if (!transition) return;
  if (current.kind === "workspace" && current.section === "language" && !(await canLeaveLanguageSection())) return;
  shellNavigationBusy = true;
  try {
    if (await restoreShellLocation(transition.target)) shellNavigationHistory = transition.history;
  } finally {
    shellNavigationBusy = false;
  }
}

$effect(() => {
  void section;
  if (section !== "lore" && loreWikiOpen) {
    loreWikiOpen = false;
    loreWikiEntityId = null;
  }
});

function normalizeDocument(body: string, format?: string) {
  if (format === "rich-text") return htmlToMarkdown(body);
  return body;
}

function dateForField(key: string) {
  return parseCalendarDate(fields[key]);
}
function selectedCalendarId(key: string): string {
  return calendarIdForStoredDate(dateForField(key), dateCalendarByField[key]);
}
function definitionForDateField(key: string): CalendarDefinition | null {
  return calendarDefinitionForId(selectedCalendarId(key));
}
function dateDraftForField(key: string): Partial<CalendarDate> | null {
  return (
    dateForField(key) ??
    (dateEditorOpen[key] ? { calendar: selectedCalendarId(key), era: "CE", precision: "day" } : null)
  );
}
function datePartsDraft(key: string) {
  const stored = dateForField(key);
  const calendar = definitionForDateField(key);
  if (stored) return calendarDateToParts(stored, calendar);
  return dateEditorOpen[key]
    ? {
        year: undefined as number | undefined,
        month: undefined as number | undefined,
        day: undefined as number | undefined,
        precision: "day" as const,
      }
    : null;
}
function setDateCalendar(key: string, calendarId: string) {
  dateCalendarByField = { ...dateCalendarByField, [key]: calendarId };
  const previous = dateForField(key);
  if (!previous) {
    dateEditorOpen = { ...dateEditorOpen, [key]: true };
    return;
  }
  fields = { ...fields, [key]: serializeCalendarDate({ ...previous, calendar: calendarId }) };
  markEntryDirty();
}
function openDateEditor(key: string) {
  dateEditorOpen = { ...dateEditorOpen, [key]: true };
  dateCalendarByField = { ...dateCalendarByField, [key]: dateCalendarByField[key] ?? GREGORIAN_CALENDAR_ID };
  fields = { ...fields, [key]: "" };
  markEntryDirty();
}
function updateDateField(key: string, patch: Partial<CalendarDate>) {
  if (projectDiagnostics.length > 0) return;
  const calendar = definitionForDateField(key);
  const calendarId = selectedCalendarId(key);
  const previous = dateForField(key);
  const currentParts = calendarDateToParts(
    previous ?? { calendar: calendarId, era: "CE", year: 1, precision: "year" },
    calendar,
  ) ?? { year: 1, precision: "year" as const };
  const nextParts = { ...currentParts, ...patch };
  if ((patch as Record<string, unknown>).precision === undefined) {
    const hasMonth = (nextParts as Record<string, unknown>).month !== undefined;
    const hasDay = (nextParts as Record<string, unknown>).day !== undefined;
    if (!hasMonth) {
      nextParts.precision = "year";
      delete (nextParts as Record<string, unknown>).day;
    } else if (!hasDay) {
      nextParts.precision = "month";
    } else {
      if (nextParts.precision !== "hour" && nextParts.precision !== "minute" && nextParts.precision !== "second") {
        nextParts.precision = "day";
      }
    }
  }
  if (patch.precision === "year") {
    delete nextParts.month;
    delete nextParts.day;
  }
  if (patch.precision === "month") {
    delete nextParts.day;
    if (nextParts.month === undefined) nextParts.month = 1;
  }
  if (patch.precision === "day") {
    nextParts.month ??= 1;
    nextParts.day ??= 1;
  }
  const stored = partsToCalendarDate(nextParts, calendar);
  stored.calendar = calendarId;
  if (previous) {
    stored.hour = patch.hour ?? previous.hour;
    stored.minute = patch.minute ?? previous.minute;
    stored.second = patch.second ?? previous.second;
  } else if (patch.hour !== undefined) {
    stored.hour = patch.hour;
    stored.minute = patch.minute;
    stored.second = patch.second;
  }
  if (patch.precision === "hour" || patch.precision === "minute" || patch.precision === "second") {
    stored.precision = patch.precision;
  }
  fields = { ...fields, [key]: serializeCalendarDate(stored) };
  markEntryDirty();
}
function updateDatePart(key: string, part: "year" | "month" | "day", raw: string, min: number, max?: number) {
  if (!raw.trim()) {
    if (part === "month") {
      updateDateField(key, { precision: "year" });
    } else if (part === "day") {
      updateDateField(key, { precision: "month" });
    } else if (part === "year") {
      clearDateField(key);
    }
    return;
  }
  const parsed = Math.floor(Number(raw));
  if (!Number.isFinite(parsed)) return;
  const value = Math.min(max ?? parsed, Math.max(min, parsed));
  updateDateField(key, { [part]: value });
}
function updateDateTime(key: string, raw: string) {
  const [hour, minute, second] = raw.split(":").map(Number);
  if (![hour, minute, second].every(Number.isFinite)) return;
  updateDateField(key, { hour, minute, second, precision: "second" });
}
function calendarTimeValue(date: Partial<CalendarDate>): string {
  if (![date.hour, date.minute, date.second].every((part) => typeof part === "number")) return "";
  return [date.hour, date.minute, date.second].map((part) => String(part).padStart(2, "0")).join(":");
}
function clearDateField(key: string) {
  fields = { ...fields, [key]: "" };
  dateEditorOpen = { ...dateEditorOpen, [key]: false };
  const next = { ...dateCalendarByField };
  delete next[key];
  dateCalendarByField = next;
  markEntryDirty();
}

function cancelAutoSave() {
  if (autoSaveTimer !== null) {
    window.clearTimeout(autoSaveTimer);
    autoSaveTimer = null;
  }
}
function scheduleAutoSave(delay = 900) {
  cancelAutoSave();
  if (!selected || !sectionEnabled() || documentConflict || projectDiagnostics.length > 0) return;
  autoSaveTimer = window.setTimeout(() => {
    autoSaveTimer = null;
    void saveDocument();
  }, delay);
}
function markEntryDirty() {
  documentRevision += 1;
  hasUnsavedChanges = true;
  savedAt = "";
  autoSaveFailureCount = 0;
  scheduleAutoSave();
}
function updateDocumentBody(value: string) {
  if (selectedLoading || selectedLoadError || projectDiagnostics.length > 0) return;
  documentBody = value;
  markEntryDirty();
}
function setEditorFullscreen(value: boolean) {
  editorFullscreen = value;
}
async function setDocumentMode(mode: "read" | "edit") {
  if (mode === documentMode) return;
  if (documentMode === "edit" && !(await flushAutoSave())) return;
  documentMode = mode;
}
function friendlyError(cause: unknown) {
  const message = cause instanceof Error ? cause.message : String(cause);
  return message.includes("invoke") || message.includes("undefined")
    ? "The desktop bridge is unavailable. Open this workspace in the Tauri app to use local project storage."
    : message;
}
function updateAiSetting(
  key: "id" | "name" | "adapter" | "endpoint" | "model" | "embeddingModel" | "capabilities",
  value: string,
) {
  aiStatus = null;
  if (key === "id" || key === "endpoint") aiModels = [];
  if (key === "capabilities") {
    const capabilities = value
      .split(",")
      .map((capability) => capability.trim())
      .filter(Boolean);
    aiSettings = { ...aiSettings, provider: { ...aiSettings.provider, capabilities } };
    void project.settingsUpdate({ ai: { provider: { capabilities } } });
    return;
  }
  aiSettings = { ...aiSettings, provider: { ...aiSettings.provider, [key]: value } };
  void project.settingsUpdate({ ai: { provider: { [key]: value } } });
  if (key === "id" || key === "endpoint") void refreshRemoteCredential();
}
function updateAiImageSetting(
  key: "enabled" | "id" | "name" | "adapter" | "endpoint" | "model",
  value: string | boolean,
) {
  aiSettings = {
    ...aiSettings,
    imageProvider: { ...aiSettings.imageProvider, [key]: value },
  };
  void project.settingsUpdate({ ai: { imageProvider: { [key]: value } } });
}
function showAiIndexMessage(message: string) {
  if (aiIndexMessageTimer !== null) window.clearTimeout(aiIndexMessageTimer);
  aiIndexMessage = message;
  if (!message) {
    aiIndexMessageTimer = null;
    return;
  }
  aiIndexMessageTimer = window.setTimeout(() => {
    aiIndexMessage = "";
    aiIndexMessageTimer = null;
  }, toastDurationMs);
}
async function refreshRemoteCredential() {
  if (!aiSettings.provider.endpoint.trim().toLowerCase().startsWith("https://") || !aiSettings.provider.id.trim()) {
    remoteCredential = null;
    return;
  }
  try {
    remoteCredential = await project.aiProviderCredentialStatus();
  } catch (_) {
    remoteCredential = { provider: aiSettings.provider.id, configured: false };
  }
}
async function importRemoteCredential() {
  if (!aiSettings.provider.id.trim()) return;
  try {
    remoteCredential = await project.aiProviderImportCredential();
  } catch (cause) {
    showAiIndexMessage(friendlyError(cause));
  }
}
async function saveRemoteCredential(apiKey: string): Promise<boolean> {
  if (!aiSettings.provider.id.trim()) return false;
  try {
    remoteCredential = await project.aiProviderSetCredential(apiKey);
    return true;
  } catch (cause) {
    showAiIndexMessage(friendlyError(cause));
    return false;
  }
}
async function clearRemoteCredential() {
  if (!aiSettings.provider.id.trim()) return;
  try {
    remoteCredential = await project.aiProviderClearCredential();
  } catch (cause) {
    showAiIndexMessage(friendlyError(cause));
  }
}
async function setProjectAiEnabled(enabled: boolean) {
  if (!projectInfo) return;
  if (enabled) {
    const confirmed = await confirmDialog({
      title: "Enable AI for this project?",
      message:
        "AI features become available in this project. Requests run through the configured provider; remote providers still require per-project consent before any context leaves this machine.",
      confirmLabel: "Enable AI",
    });
    if (!confirmed) return;
  }
  try {
    projectInfo = await project.setAiEnabled(enabled);
  } catch (cause) {
    showAiIndexMessage(friendlyError(cause));
  }
}
async function setRemoteConsent(allowed: boolean) {
  if (
    !projectInfo?.root ||
    !aiSettings.provider.endpoint.trim().toLowerCase().startsWith("https://") ||
    !aiSettings.provider.id ||
    !aiSettings.provider.endpoint
  )
    return;
  try {
    await project.aiRemoteSetConsent(projectInfo.root, allowed);
    const consents = aiSettings.consents.filter(
      (consent) => !(consent.projectId === projectInfo?.root && consent.provider === aiSettings.provider.id),
    );
    aiSettings = {
      ...aiSettings,
      consents: allowed
        ? [
            ...consents,
            {
              projectId: projectInfo.root,
              provider: aiSettings.provider.id,
              endpoint: aiSettings.provider.endpoint,
            },
          ]
        : consents,
    };
  } catch (cause) {
    showAiIndexMessage(friendlyError(cause));
  }
}
async function checkAiProvider() {
  if (aiStatusTimer !== null) window.clearTimeout(aiStatusTimer);
  try {
    aiStatus = await project.aiProviderStatus();
  } catch (cause) {
    aiStatus = {
      endpoint: aiSettings.provider.endpoint,
      model: aiSettings.provider.model,
      available: false,
      modelAvailable: false,
      embeddingAvailable: false,
      credentialAvailable: false,
      error: friendlyError(cause),
    };
  }
  aiStatusTimer = window.setTimeout(() => {
    aiStatus = null;
    aiStatusTimer = null;
  }, toastDurationMs);
}
async function loadAiModels() {
  const endpoint = aiSettings.provider.endpoint.trim();
  if (!endpoint) {
    aiModelsMessage = "Enter an active provider endpoint before loading models.";
    return;
  }
  aiModelsBusy = true;
  aiModelsMessage = "";
  try {
    aiModels = await project.aiProviderModels();
    if (aiModels.length === 0) {
      aiModelsMessage = "No models were returned by the active provider.";
    } else {
      aiModelsMessage = `${aiModels.length} model${aiModels.length === 1 ? "" : "s"} available.`;
      if (aiModelsMessageTimer !== null) window.clearTimeout(aiModelsMessageTimer);
      aiModelsMessageTimer = window.setTimeout(() => {
        aiModelsMessage = "";
        aiModelsMessageTimer = null;
      }, toastDurationMs);
      if (!aiSettings.provider.model.trim() && aiModels.length === 1) updateAiSetting("model", aiModels[0]);
    }
  } catch (cause) {
    aiModels = [];
    aiModelsMessage = friendlyError(cause);
  } finally {
    aiModelsBusy = false;
  }
}
async function refreshAiIndexStatus() {
  if (aiIndexStatusMessageTimer !== null) window.clearTimeout(aiIndexStatusMessageTimer);
  if (!projectInfo?.aiEnabled) {
    aiIndexStatus = { available: false, state: null, provider: null, embeddingAvailable: false, message: null };
    return;
  }
  try {
    const status = await project.aiIndexStatus();
    aiIndexStatus = status;
    if (status.message) {
      const message = status.message;
      aiIndexStatusMessageTimer = window.setTimeout(() => {
        if (aiIndexStatus?.message === message) aiIndexStatus = { ...aiIndexStatus, message: null };
        aiIndexStatusMessageTimer = null;
      }, toastDurationMs);
    } else {
      aiIndexStatusMessageTimer = null;
    }
  } catch (cause) {
    aiIndexStatus = { available: false, state: null, provider: null, embeddingAvailable: false, message: null };
    showAiIndexMessage(friendlyError(cause));
  }
}
async function rebuildAiIndex() {
  if (!projectInfo?.aiEnabled) {
    showAiIndexMessage("AI is disabled for this project. Enable AI in Settings first.");
    return;
  }
  if (!aiSettings.provider.endpoint.trim() || !aiSettings.provider.model.trim()) {
    showAiIndexMessage("Configure the active provider endpoint and model before building the semantic index.");
    return;
  }
  aiIndexBusy = true;
  showAiIndexMessage("");
  try {
    const result = await project.aiIndexRebuild();
    aiIndexStatus = {
      available: true,
      state: result.state,
      provider: aiSettings.provider.id,
      embeddingAvailable: true,
      message: null,
    };
    showAiIndexMessage(
      `Indexed ${result.chunkCount} chunks (${result.embeddedCount} embedded, ${result.reusedCount} reused).`,
    );
  } catch (cause) {
    showAiIndexMessage(friendlyError(cause));
  } finally {
    aiIndexBusy = false;
  }
}
async function cancelAiIndex() {
  try {
    await project.aiIndexCancel();
    showAiIndexMessage("Cancellation requested.");
  } catch (cause) {
    showAiIndexMessage(friendlyError(cause));
  }
}
function setAiSelection(markdown: string, plainText: string) {
  if (!aiBusy && !aiPreviewOutput) {
    aiSourceSelection = markdown;
    aiSourceSelectionPlain = plainText;
  }
}
function openAiAction(
  action: "rewrite" | "generate" | "concise" | "expand" | "grammar" | "tone" | "custom",
  markdown: string,
  plainText: string,
  context: string,
) {
  if (!projectInfo?.aiEnabled) return;
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
  return definitions().filter((definition) =>
    definition.type === "relationship"
      ? selectedRelationshipIds(definition).length === 0
      : isEmptyFieldValue(fields[definition.key]),
  );
}
function clearAiFieldListener() {
  aiFieldUnlisten?.();
  aiFieldUnlisten = null;
}
function closeAiFieldFill() {
  if (aiFieldFillRequestId) void project.aiCancelText(aiFieldFillRequestId).catch(() => {});
  aiFieldFillStartCancelled = true;
  clearAiFieldListener();
  aiFieldFillOpen = false;
  aiFieldFillBusy = false;
  aiFieldFillRequestId = null;
  aiFieldFillEntityId = null;
  aiFieldFillLastSequence = -1;
  aiFieldFillStream = "";
  aiFieldSuggestions = {};
}
function handleAiFieldFillEvent(payload: AiStreamEvent) {
  if (aiFieldFillEntityId !== null && selected?.id !== aiFieldFillEntityId) return;
  if (payload.sequence <= aiFieldFillLastSequence) return;
  aiFieldFillLastSequence = payload.sequence;
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
    if (payload.phase === "deadline_exceeded") {
      error = "AI field generation reached its time limit before returning complete suggestions.";
    }
  } else if (payload.phase === "completed") {
    clearAiFieldListener();
    aiFieldFillBusy = false;
    aiFieldFillRequestId = null;
    try {
      const parsed = JSON.parse(payload.output ?? aiFieldFillStream) as {
        suggestions?: Record<string, { value?: unknown; values?: unknown; rationale?: unknown; confidence?: unknown }>;
      };
      const allowed = new Set(emptyInspectorDefinitions().map((definition) => definition.key));
      const suggestions: Record<string, AiFieldSuggestion> = {};
      for (const [key, value] of Object.entries(parsed.suggestions ?? {})) {
        const definition = definitions().find((candidate) => candidate.key === key);
        if (!definition || !allowed.has(key)) continue;
        const rawValue = definition.multiple || definition.type === "relationship" ? value?.values : value?.value;
        const normalizedValue = coerceAiFieldValue(definition, rawValue);
        if (normalizedValue === null) continue;
        if (definition?.type === "relationship") {
          const allowedIds = new Set(
            entities
              .filter(
                (entity) =>
                  !entity.deleted &&
                  (!definition.targetEntityTypes?.length ||
                    definition.targetEntityTypes.includes(entity.entity_type ?? "")),
              )
              .map((entity) => entity.id),
          );
          if (!(normalizedValue as string[]).every((id) => allowedIds.has(id))) continue;
        }
        suggestions[key] = {
          value: normalizedValue,
          rationale: typeof value.rationale === "string" ? value.rationale.trim() : "",
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
  if (!projectInfo?.aiEnabled) return;
  const empty = emptyInspectorDefinitions();
  if (empty.length === 0) return;
  const endpoint = aiSettings.provider.endpoint.trim();
  const model = aiSettings.provider.model.trim();
  if (!endpoint || !model) {
    error = "Configure the active provider endpoint and model before filling fields.";
    return;
  }
  aiFieldFillOpen = true;
  aiFieldFillBusy = true;
  aiFieldFillStream = "";
  aiFieldSuggestions = {};
  aiFieldFillLastSequence = -1;
  aiFieldFillStartCancelled = false;
  aiFieldFillEntityId = selected.id;
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
      allowedEntities:
        definition.type === "relationship"
          ? entities
              .filter(
                (entity) =>
                  !entity.deleted &&
                  (!definition.targetEntityTypes?.length ||
                    definition.targetEntityTypes.includes(entity.entity_type ?? "")),
              )
              .map((entity) => ({ id: entity.id, name: entity.name, type: entity.entity_type }))
          : [],
    })),
  });
  const suggestionProperties = Object.fromEntries(
    empty.map((definition) => [
      definition.key,
      {
        type: "object",
        properties: {
          ...(definition.multiple || definition.type === "relationship"
            ? { values: aiJsonValueSchema(definition) }
            : { value: aiJsonValueSchema(definition) }),
          rationale: { type: "string", maxLength: 1000 },
          confidence: { type: "string", maxLength: 32 },
        },
        required: [definition.multiple || definition.type === "relationship" ? "values" : "value"],
        additionalProperties: false,
      },
    ]),
  );
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
  const retrievalQuery =
    `${selected.name} ${selected.entity_type ?? ""} ${empty.map((definition) => definition.label).join(" ")}`.slice(
      0,
      4000,
    );
  let startedRequestId: string | null = null;
  try {
    const requestId = await project.aiGenerateStructured(
      projectInfo!.root,
      `Fill only these empty fields: ${fieldKeys.join(", ")}. For multi-select and relationship fields, return up to five distinct values in the values array. For relationship fields, use only allowed entity IDs from the context. Use only configured options when options are provided. Return evidence-backed suggestions. Do not invent facts.`,
      context,
      outputContract,
      aiFieldFillEntityId!,
      retrievalQuery,
      2,
    );
    startedRequestId = requestId;
    if (aiFieldFillStartCancelled || selected?.id !== aiFieldFillEntityId) {
      void project.aiCancelText(requestId).catch(() => {});
      return;
    }
    aiFieldFillRequestId = requestId;
    aiFieldUnlisten = await listen<AiStreamEvent>(`ai-stream:${requestId}`, (event) =>
      handleAiFieldFillEvent(event.payload),
    );
    const buffered = await project.aiPollText(requestId);
    for (const event of buffered) handleAiFieldFillEvent(event);
  } catch (cause) {
    if (startedRequestId) void project.aiCancelText(startedRequestId).catch(() => {});
    clearAiFieldListener();
    aiFieldFillBusy = false;
    aiFieldFillRequestId = null;
    if (aiFieldFillStartCancelled || selected?.id !== aiFieldFillEntityId) return;
    error = friendlyError(cause);
  }
}
async function acceptAiFieldSuggestion(key: string) {
  if (selected?.id !== aiFieldFillEntityId) return;
  const suggestion = aiFieldSuggestions[key];
  const definition = definitions().find((candidate) => candidate.key === key);
  if (
    !suggestion ||
    (definition?.type === "relationship"
      ? selectedRelationshipIds(definition).length > 0
      : !isEmptyFieldValue(fields[key]))
  )
    return;
  if (definition?.type === "relationship") await updateRelationshipField(definition, suggestion.value as string[]);
  else fields = { ...fields, [key]: suggestion.value };
  const remaining = { ...aiFieldSuggestions };
  delete remaining[key];
  aiFieldSuggestions = remaining;
  markEntryDirty();
}
async function acceptAllAiFieldSuggestions() {
  for (const key of Object.keys(aiFieldSuggestions)) await acceptAiFieldSuggestion(key);
  if (Object.keys(aiFieldSuggestions).length === 0) closeAiFieldFill();
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
  aiStartCancelled = true;
  clearAiStreamListener();
  aiRewriteOpen = false;
  aiBusy = false;
  aiCancelPending = false;
  aiRequestId = null;
  aiSourceEntityId = null;
  aiStreamText = "";
  aiPreviewOutput = "";
  aiProgressMessage = "Preparing model…";
  aiUsage = null;
  aiSourceSelection = "";
  aiSourceSelectionPlain = "";
  aiGenerationContext = "";
  aiLastSequence = -1;
  aiMode = "rewrite";
}
async function cancelAiRewrite() {
  if (!aiBusy || aiCancelPending) return;
  if (!aiRequestId) {
    closeAiRewrite();
    return;
  }
  aiCancelPending = true;
  try {
    await project.aiCancelText(aiRequestId);
  } catch (cause) {
    aiCancelPending = false;
    error = friendlyError(cause);
  }
}
function validateAiProposal(value: string): string | null {
  if (!value.trim()) return "The AI provider returned an empty proposal.";
  if (/(^|\n)\s*(#{1,6}\s|>\s|[-*+]\s|\d+\.\s|```|~~~)/.test(value)) {
    return "The proposal contains block-level Markdown. Edit it to plain text before accepting.";
  }
  if (/<\/?[a-z][^>]*>/i.test(value))
    return "The proposal contains HTML markup. Edit it to plain text before accepting.";
  return null;
}
function handleAiEvent(payload: AiStreamEvent) {
  if (aiSourceEntityId !== null && selected?.id !== aiSourceEntityId) return;
  if (payload.sequence <= aiLastSequence) return;
  aiLastSequence = payload.sequence;
  const nextStreamState = reduceTextGenerationEvent(
    {
      streamText: aiStreamText,
      proposal: aiPreviewOutput,
      progressMessage: aiProgressMessage,
    },
    payload,
  );
  aiStreamText = nextStreamState.streamText;
  aiPreviewOutput = nextStreamState.proposal;
  aiProgressMessage = nextStreamState.progressMessage;
  if (payload.phase === "usage" && payload.output) {
    try {
      aiUsage = JSON.parse(payload.output);
    } catch (_) {
      aiUsage = null;
    }
  }
  if (payload.phase === "completed") {
    aiBusy = false;
    aiCancelPending = false;
    aiRequestId = null;
    clearAiStreamListener();
  } else if (payload.phase === "cancelled" || payload.phase === "deadline_exceeded") {
    aiBusy = false;
    aiCancelPending = false;
    aiRequestId = null;
    clearAiStreamListener();
    if (payload.phase === "deadline_exceeded") {
      error = aiPreviewOutput
        ? "AI generation reached its time limit. The partial proposal is preserved below."
        : "AI generation reached its time limit before producing a proposal.";
    }
  } else if (payload.phase === "failed") {
    aiBusy = false;
    aiCancelPending = false;
    aiRequestId = null;
    clearAiStreamListener();
    error = payload.error ?? "The AI provider could not generate a proposal";
  }
}
async function startAiRewrite() {
  if (!selected || (aiMode === "rewrite" && !aiSourceSelection.trim()) || !aiInstruction.trim() || aiBusy) return;
  if (!(await flushAutoSave())) return;
  aiSourceEntityId = selected.id;
  aiSourceBody = documentBody;
  aiSourceRevision = loadedDocumentRevision;
  aiStreamText = "";
  aiPreviewOutput = "";
  aiProgressMessage = "Preparing model…";
  aiLastSequence = -1;
  aiStartCancelled = false;
  aiBusy = true;
  aiCancelPending = false;
  let startedRequestId: string | null = null;
  try {
    const sourceText =
      aiMode === "generate" ? aiGenerationContext.trim() || documentBody.trim() || "[CURSOR]" : aiSourceSelection;
    const retrievalQuery = [selected?.name, aiInstruction, aiSourceSelectionPlain]
      .filter(Boolean)
      .join(" ")
      .slice(0, 4000);
    const requestId = await project.aiGenerateText(
      projectInfo!.root,
      aiInstruction,
      sourceText,
      aiSourceEntityId,
      retrievalQuery,
      2,
    );
    startedRequestId = requestId;
    if (aiStartCancelled || selected?.id !== aiSourceEntityId) {
      void project.aiCancelText(requestId).catch(() => {});
      return;
    }
    aiRequestId = requestId;
    aiUnlisten = await listen<AiStreamEvent>(`ai-stream:${requestId}`, (event) => {
      handleAiEvent(event.payload);
    });
    const buffered = await project.aiPollText(requestId);
    for (const event of buffered) handleAiEvent(event);
  } catch (cause) {
    if (startedRequestId) void project.aiCancelText(startedRequestId).catch(() => {});
    clearAiStreamListener();
    aiBusy = false;
    aiCancelPending = false;
    aiRequestId = null;
    if (aiStartCancelled || selected?.id !== aiSourceEntityId) return;
    error = friendlyError(cause);
  }
}
async function acceptAiRewrite() {
  if (!selected || !aiPreviewOutput || aiBusy) return;
  if (
    selected.id !== aiSourceEntityId ||
    documentBody !== aiSourceBody ||
    loadedDocumentRevision !== aiSourceRevision
  ) {
    error = "The entity or document changed while the rewrite was being prepared. Discard it and try again.";
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
  const nextMarkdown = editorRef?.replaceAiTextWithMarkdown(aiPreviewOutput);
  if (nextMarkdown === null || nextMarkdown === undefined) {
    error = "The selected text is no longer at the original position. Discard it and try again.";
    return;
  }
  const bodyBefore = documentBody;
  documentBody = nextMarkdown;
  markEntryDirty();
  if (await saveDocument()) {
    closeAiRewrite();
  } else {
    documentBody = bodyBefore;
  }
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
  void persistRecentProjects(
    [{ name: info.name, root: info.root }, ...recentProjects.filter((entry) => entry.root !== info.root)].slice(0, 6),
  );
}
function removeRecentProject(root: string) {
  void persistRecentProjects(recentProjects.filter((entry) => entry.root !== root));
}
function updateThemePreference(preference: ThemePreference) {
  themePreference = preference;
  cacheThemePreference(preference);
  applyThemePreference(preference);
  void project.settingsUpdate({ general: { appearance: { theme: preference } } }).catch(() => {});
}
async function loadRecentProjects() {
  try {
    const settings = await project.settingsGet();
    recentProjects = settings.general.recentProjects.slice(0, 6);
    themePreference = normalizeThemePreference(settings.general.appearance.theme);
    cacheThemePreference(themePreference);
    applyThemePreference(themePreference);
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
      if (Array.isArray(stored))
        recentProjects = stored
          .filter((item): item is RecentProject => typeof item?.name === "string" && typeof item?.root === "string")
          .slice(0, 6);
    } catch {
      recentProjects = [];
    }
  }
}
async function loadEntities() {
  entities = await project.listEntities();
  await refreshCalendarDefinitions();
}

function bumpCollectionRefresh() {
  if (collectionListElement) queueCollectionScroll(section, collectionListElement.scrollTop);
  collectionRefreshEpoch += 1;
}

function upsertEntityInCache(entity: Entity) {
  const index = entities.findIndex((candidate) => candidate.id === entity.id);
  if (index >= 0) {
    entities = entities.map((candidate) => (candidate.id === entity.id ? entity : candidate));
  } else {
    entities = [...entities, entity];
  }
}

function removeEntityFromCache(id: string) {
  entities = entities.filter((entity) => entity.id !== id);
}

/** Patch one entity (or drop it) and re-query the current collection page/counts. */
async function refreshAfterEntityMutation(options?: { entityId?: string; removed?: boolean }) {
  if (options?.removed && options.entityId) {
    removeEntityFromCache(options.entityId);
  } else if (options?.entityId) {
    try {
      const loaded = await project.getEntity(options.entityId);
      if (!loaded || loaded.deleted) removeEntityFromCache(options.entityId);
      else upsertEntityInCache(loaded);
    } catch {
      // Keep the prior cache entry; collection re-query still runs.
    }
  }
  bumpCollectionRefresh();
  await refreshArchivedCount();
}

function searchEntitiesPaged(field?: FieldDefinition): AsyncEntitySearchFn {
  return async (query: AsyncEntitySearchQuery) => {
    const page = await project.queryEntities({
      query: query.text || undefined,
      entityTypes: query.entityTypes ?? field?.targetEntityTypes,
      excludedEntityTypes: query.excludedEntityTypes,
      sortField: toShellSortField(query.sortField),
      sortDirection: toShellSortDirection(query.sortDirection),
      offset: query.offset,
      limit: query.limit,
    });
    return toAsyncEntityPage(page, { excludeIds: query.excludeIds });
  };
}

async function resolveSelectedEntities(ids: string[]): Promise<AsyncEntityOption[]> {
  if (ids.length === 0) return [];
  const byId = new Map(entities.filter((entity) => !entity.deleted).map((entity) => [entity.id, entity]));
  const missing = ids.filter((id) => !byId.has(id));
  if (missing.length > 0) {
    const loaded = await Promise.all(
      missing.map(async (id) => {
        try {
          return await project.getEntity(id);
        } catch {
          return null;
        }
      }),
    );
    for (const entity of loaded) {
      if (entity && !entity.deleted) {
        upsertEntityInCache(entity);
        byId.set(entity.id, entity);
      }
    }
  }
  return ids.map((id) => {
    const cached = byId.get(id);
    if (cached) {
      return {
        id: cached.id,
        name: cached.name,
        entityType: cached.entity_type,
        revision: cached.revision,
      };
    }
    return { id, name: id, entityType: null };
  });
}

async function refreshArchivedCount() {
  try {
    const page = await project.queryEntities({ archived: true, limit: 1 });
    archivedCount = page.total;
  } catch {
    archivedCount = 0;
  }
}

async function handleArchiveChanged() {
  bumpCollectionRefresh();
  await refreshArchivedCount();
}

async function refreshCalendarDefinitions() {
  const next: Record<string, CalendarDefinition> = {};
  try {
    if (!projectInfo?.root) {
      calendarDefinitions = next;
      calendarEntities = [];
      return;
    }
    const context = contextFor("timeline");
    const page = await project.queryEntities({
      entityTypes: ["daena.timeline:calendar"],
      sortField: "name",
      sortDirection: "asc",
      limit: 200,
    });
    const calendars = page.items.filter((entity) => !entity.deleted);
    calendarEntities = calendars;
    for (const calendar of calendars) upsertEntityInCache(calendar);
    await Promise.all(
      calendars.map(async (calendar) => {
        const records = await context.records.list("calendar-definition", calendar.id as UUID, { limit: 1 });
        next[calendar.id] = records[0]
          ? normalizeCalendarDefinition(records[0].value)
          : normalizeCalendarDefinition({});
      }),
    );
  } catch {
    // A missing record capability or empty project still leaves Gregorian as the default.
  }
  calendarDefinitions = next;
}

async function refreshGit(epoch = projectStatusEpoch) {
  gitMessage = "";
  try {
    const status = await project.gitStatus();
    if (epoch !== projectStatusEpoch) return;
    gitStatus = status;
  } catch (cause) {
    if (epoch !== projectStatusEpoch) return;
    gitMessage = friendlyError(cause);
  }
}

async function applyProjectInfo(info: ProjectInfo | null, epoch: number) {
  if (epoch !== projectStatusEpoch) return;
  if (info) projectInfo = info;
}

async function loadProjectStatus(includeGit: boolean) {
  if (!ready) return;
  const epoch = ++projectStatusEpoch;
  try {
    const info = await project.info();
    applyProjectInfo(info, epoch);
  } catch (cause) {
    if (epoch !== projectStatusEpoch) return;
    error = friendlyError(cause);
  }
  if (!includeGit || epoch !== projectStatusEpoch) return;
  await refreshGit(epoch);
}

async function refreshProjectStatus() {
  await loadProjectStatus(true);
}

function openStatusDestination(section: ProjectSection) {
  statusCenterOpen = false;
  void openProjectCenter(section);
}

function statusCenterItems(): StatusCenterItem[] {
  if (!ready || !projectInfo) return [];
  const items: StatusCenterItem[] = [];
  const saving = saveInFlight !== null;
  if (selected) {
    items.push(
      documentConflict
        ? {
            id: "save",
            label: "Document conflict",
            detail: "The open draft and the project copy diverged. Compare or recover before continuing.",
            tone: "danger",
            actionLabel: "Return to draft",
            onAction: () => (statusCenterOpen = false),
          }
        : saveError
          ? {
              id: "save",
              label: "Save paused",
              detail: saveError,
              tone: "danger",
              actionLabel: "Retry",
              onAction: () => {
                statusCenterOpen = false;
                void saveDocument();
              },
            }
          : saving
            ? { id: "save", label: "Saving entry", detail: selected.name, tone: "busy" }
            : hasUnsavedChanges
              ? {
                  id: "save",
                  label: "Unsaved changes",
                  detail: `${selected.name} is queued for local save.`,
                  tone: "warning",
                }
              : {
                  id: "save",
                  label: "Entry saved",
                  detail: savedAt ? `Saved locally at ${savedAt}.` : "The open entry has no pending edits.",
                  tone: "success",
                },
    );
  }

  const sync = projectInfo.sync;
  items.push(
    sync.export_error
      ? {
          id: "checkpoint",
          label: "Checkpoint failed",
          detail: sync.export_error,
          tone: "danger",
          actionLabel: "Review",
          onAction: () => openStatusDestination("advanced"),
        }
      : sync.state === "pending"
        ? {
            id: "checkpoint",
            label: "Checkpoint pending",
            detail: `${sync.dirty_count} portable change${sync.dirty_count === 1 ? "" : "s"} waiting to be written.`,
            tone: "busy",
            actionLabel: "Project data",
            onAction: () => openStatusDestination("data"),
          }
        : {
            id: "checkpoint",
            label: "Checkpoint current",
            detail: "Portable project files match the local project state.",
            tone: "success",
            actionLabel: "Project data",
            onAction: () => openStatusDestination("data"),
          },
  );

  items.push(
    gitMessage
      ? {
          id: "snapshot",
          label: "Snapshot status unavailable",
          detail: gitMessage,
          tone: "warning",
          actionLabel: "Review",
          onAction: () => openStatusDestination("snapshots"),
        }
      : {
          id: "snapshot",
          label: gitStatus?.repository ? "Snapshots ready" : "Snapshots not configured",
          detail: gitStatus?.repository
            ? `${gitStatus.canonical_changes.length} project change${gitStatus.canonical_changes.length === 1 ? "" : "s"} since the last Snapshot.`
            : "Set up project history when you are ready to preserve milestones.",
          tone: gitStatus?.canonical_changes.length ? "warning" : "neutral",
          actionLabel: "Open Snapshots",
          onAction: () => openStatusDestination("snapshots"),
        },
  );

  const mapStates = Object.values(mapSaveStates).map((state) => state.status);
  const backgroundBusy =
    projectTransitionBusy ||
    aiIndexBusy ||
    moduleSchemaBusy ||
    mapStates.some((status) => ["saving", "restoring", "loading"].includes(status));
  const backgroundFailed = mapStates.some((status) => status === "error" || status === "conflict");
  const diagnostic = projectDiagnostics[0];
  items.push(
    diagnostic || backgroundFailed
      ? {
          id: "background",
          label: diagnostic ? "Project conflict" : "Background task needs attention",
          detail: diagnostic ?? "A map save or recovery task did not complete cleanly.",
          tone: "danger",
          actionLabel: diagnostic ? "Diagnostics" : undefined,
          onAction: diagnostic ? () => openStatusDestination("advanced") : undefined,
        }
      : backgroundBusy
        ? {
            id: "background",
            label: "Background work in progress",
            detail:
              projectTransitionMessage ||
              (aiIndexBusy
                ? "Building the semantic index…"
                : moduleSchemaBusy
                  ? "Saving Fields & Types…"
                  : "Saving map changes…"),
            tone: "busy",
          }
        : {
            id: "background",
            label: "No background work",
            detail: "Daena has no unfinished maintenance task.",
            tone: "neutral",
          },
  );
  return items;
}

function statusCenterTone(): StatusCenterTone {
  const tones = statusCenterItems().map((item) => item.tone);
  if (tones.includes("danger")) return "danger";
  if (tones.includes("busy")) return "busy";
  if (tones.includes("warning")) return "warning";
  if (tones.includes("success")) return "success";
  return "neutral";
}

function statusCenterSummary() {
  const tone = statusCenterTone();
  if (tone === "danger") return "Needs attention";
  if (tone === "busy") return "Working…";
  if (tone === "warning") return "Changes pending";
  return "Project current";
}

async function finishOpening(info?: ProjectInfo) {
  projectInfo = info ?? (await project.info());
  if (!projectInfo) throw new Error("The project did not return an identity");
  modules = await project.listModuleManifests();
  await reconcileWorkspaceSection();
  rememberProject(projectInfo);
  await loadEntities();
  await refreshArchivedCount();
  await refreshGit();
  await refreshAdmin();
  shellNavigationHistory = emptyShellNavigationHistory();
  projectHomeOpen = true;
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
      if (!(await leavePluginView())) return;
      resetProjectSessionState();
      await project.close();
      await finishOpening(await project.openDirectory(path));
    });
  } catch (cause) {
    error = friendlyError(cause);
  }
}

async function openRecentProject(path: string) {
  if (projectTransitionBusy) return;
  showProjectMenu = false;
  await runProjectTransition("Opening project…", async () => {
    if (!(await flushAutoSave())) return;
    if (!(await leavePluginView())) return;
    resetProjectSessionState();
    await project.close();
    await finishOpening(await project.openDirectory(path));
  });
}

async function closeProject() {
  if (projectTransitionBusy) return;
  showProjectMenu = false;
  await runProjectTransition("Closing project…", async () => {
    if (!(await flushAutoSave())) return;
    if (!(await leavePluginView())) return;
    resetProjectSessionState();
    await project.close();
  });
}

async function flushAutoSave() {
  editorRef?.flushPendingChanges();
  cancelAutoSave();
  if (!hasUnsavedChanges) return true;
  return saveDocument();
}

async function loadSelectedState(entity: Entity) {
  const token = ++selectedLoadToken;
  const entityId = entity.id;
  const isCurrent = () => token === selectedLoadToken && selected?.id === entityId;
  selectedLoading = true;
  selectedLoadError = "";
  try {
    closeAiFieldFill();
    closeAiRewrite();
    documentBody = "";
    fields = {};
    relationships = [];
    eraContexts = [];
    metadataDialog = null;
    assets = [];
    mapLocations = [];
    loadedDocumentRevision = "";
    loadedFieldRevisions = {};
    loadedStructuredFieldKeys = new Set();
    const context = contextFor();
    const documents = await project.listDocuments(entityId);
    if (!isCurrent()) return;
    const document = documents[0];
    documentBody = normalizeDocument(document?.body ?? "", document?.format);
    loadedDocumentRevision = document?.revision ?? "";
    const storedFields = await project.listFields(entityId);
    if (!isCurrent()) return;
    const activeNamespaces = new Set(
      activeManifest()
        ?.schemas.filter((schema) => !entity.entity_type || schemaEntityTypeIds(schema).includes(entity.entity_type))
        .map((schema) => schema.namespace) ?? [],
    );
    const relevantFields = storedFields.filter(
      (field) => activeNamespaces.size === 0 || activeNamespaces.has(field.namespace),
    );
    const values = Object.fromEntries(relevantFields.map((field) => [field.key, field.value]));
    loadedFieldRevisions = Object.fromEntries(
      relevantFields.map((field) => [fieldRevisionKey(field.namespace, field.key), field.revision]),
    );
    loadedStructuredFieldKeys = new Set(
      relevantFields
        .filter((field) => isStructuredFieldValue(field.value))
        .map((field) => fieldRevisionKey(field.namespace, field.key)),
    );
    dateEditorOpen = {};
    const nextDateCalendars: Record<string, string> = {};
    fields = Object.fromEntries(
      Object.entries(values).map(([key, value]) => {
        const definition = definitions().find((candidate) => candidate.key === key);
        if (definition?.type === "date") {
          const date = parseCalendarDate(value);
          if (date && !isGregorianCalendarId(date.calendar)) nextDateCalendars[key] = date.calendar;
          const serialized = date ? serializeCalendarDate(date) : "";
          const iso = typeof serialized === "string" ? serialized : formatCalendarDate(date);
          if (iso === "1" || iso === "1-1" || iso === "1-1-1") return [key, ""];
          return [key, serialized === "" ? String(value ?? "") : serialized];
        }
        if (definition && (definition.type === "number" || definition.type === "boolean" || definition.multiple))
          return [key, value];
        return [key, fieldDisplayValue(value)];
      }),
    );
    dateCalendarByField = nextDateCalendars;
    relationships = context.module.capabilities.includes("relationship.read")
      ? (await context.relationships.list(entityId as UUID)).map((relationship) => ({
          id: relationship.id,
          source_id: relationship.sourceId,
          target_id: relationship.targetId,
          relationship_type: relationship.type,
          metadata: JSON.stringify(relationship.metadata),
          revision: relationship.revision,
        }))
      : [];
    if (!isCurrent()) return;
    eraContexts = await loadEraContexts(
      relationships
        .filter((relationship) => relationship.relationship_type === "during" && relationship.source_id === entityId)
        .map((relationship) => relationship.target_id),
    );
    if (!isCurrent()) return;
    hintDateCalendarsFromEras(eraContexts, false);
    assets = context.module.capabilities.includes("asset.read:self")
      ? (await context.assets.list(entityId as UUID)).map((asset) => ({
          id: asset.id,
          entity_id: asset.entityId,
          namespace: asset.namespace,
          filename: asset.filename,
          content_hash: asset.contentHash,
          size: asset.size,
          mime_type: asset.mimeType,
          path: asset.path,
          created_at: asset.createdAt,
          role: asset.role,
          reference_scope: asset.referenceScope,
          revision: asset.revision,
        }))
      : [];
    if (!isCurrent()) return;
    const nextMapLocations = entityId && mapsEnabled() ? await project.listMapLocations(entityId) : [];
    if (!isCurrent()) return;
    mapLocations = nextMapLocations;
    savedAt = "";
  } catch (cause) {
    if (isCurrent()) selectedLoadError = friendlyError(cause);
    throw cause;
  } finally {
    if (isCurrent()) selectedLoading = false;
  }
}

async function refreshSelectedMapLocations(entityId = selected?.id) {
  mapLocations = entityId && mapsEnabled() ? await project.listMapLocations(entityId) : [];
}

async function openMapLocation(location: MapLocation) {
  try {
    const mapEntityId = location.mapEntityId;
    if (!mapEntityId) throw new Error("map-unavailable: this location is missing its map id");
    await project.mapsNavigation("openMap", { mapEntityId, linkId: location.id });
  } catch (cause) {
    const message = friendlyError(cause);
    if (message.includes("link-unresolved")) {
      mapReconcileNotice = `This link is unresolved — the map feature it pointed to was removed or renumbered.`;
      return;
    }
    error = message;
  }
}

async function ensureMapEditorOpen(mapEntityId: string) {
  const departure = currentShellLocation();
  const map = entities.find((entity) => entity.id === mapEntityId) ?? (await project.getEntity(mapEntityId));
  if (!map) throw new Error("map-unavailable: choose a saved map first");
  if (!(await flushAutoSave())) throw new Error("Save the current draft before opening the map editor.");
  const mapsView = mapsNavigationItem();
  selected = map;
  mapsEditorKey = map.id;
  await loadSelectedState(map);
  if (mapsView) await openPluginView(mapsView, departure);
}

async function beginMapPick(pending: NonNullable<typeof mapPickPending>) {
  mapPickPending = pending;
  mapPickNotice =
    pending.kind === "rebind"
      ? "Click the map to rebind this location."
      : "Click for a point, or use Path/Area to draw a route or region.";
  await ensureMapEditorOpen(pending.mapEntityId);
  if (mapsEditorMode === "vector") return;
}

async function applyMapPick(anchor: unknown) {
  const pending = mapPickPending;
  mapPickPending = null;
  mapPickNotice = "";
  if (!pending || !anchor) return;
  try {
    if (pending.kind === "link") {
      const entity =
        entities.find((candidate) => candidate.id === pending.entityId) ?? (await project.getEntity(pending.entityId));
      if (!entity) throw new Error("Choose an entity to link.");
      const location: MapLocation = {
        id: crypto.randomUUID(),
        mapEntityId: pending.mapEntityId,
        role: pending.role,
        label: entity.name,
        anchor,
        validity: { from: null, to: null },
      };
      await project.upsertMapLocation(entity.id, location);
      const departure = currentShellLocation();
      if (!(await leavePluginView())) return;
      recordShellDeparture(departure);
      section =
        entity.entity_type === "daena.timeline:event" ||
        entity.entity_type === "daena.timeline:encounter" ||
        entity.entity_type === "daena.timeline:era" ||
        entity.entity_type === "daena.timeline:calendar"
          ? "timeline"
          : "lore";
      await selectEntity(entity, false);
      mapLocations = await project.listMapLocations(entity.id);
    } else {
      await project.upsertMapLocation(pending.entityId, { ...pending.location, anchor });
      const entity =
        entities.find((candidate) => candidate.id === pending.entityId) ?? (await project.getEntity(pending.entityId));
      if (entity) {
        const departure = currentShellLocation();
        if (!(await leavePluginView())) return;
        recordShellDeparture(departure);
        section =
          entity.entity_type === "daena.timeline:event" ||
          entity.entity_type === "daena.timeline:encounter" ||
          entity.entity_type === "daena.timeline:era" ||
          entity.entity_type === "daena.timeline:calendar"
            ? "timeline"
            : "lore";
        await selectEntity(entity, false);
        mapLocations = await project.listMapLocations(entity.id);
      }
    }
  } catch (cause) {
    error = friendlyError(cause);
  }
}

async function openMapEntityFromLink(entityId: string) {
  try {
    const entity = await project.getEntity(entityId);
    if (!entity) throw new Error("Linked entity was not found.");
    if (!entities.some((candidate) => candidate.id === entity.id)) entities = [...entities, entity];
    const target =
      entity.entity_type === "daena.lore:person" ||
      entity.entity_type === "daena.lore:place" ||
      entity.entity_type === "daena.lore:faction" ||
      entity.entity_type === "daena.lore:artifact" ||
      entity.entity_type === "daena.lore:culture"
        ? "lore"
        : entity.entity_type?.startsWith("timeline") ||
            entity.entity_type === "daena.timeline:event" ||
            entity.entity_type === "daena.timeline:encounter" ||
            entity.entity_type === "daena.timeline:era" ||
            entity.entity_type === "daena.timeline:calendar"
          ? "timeline"
          : "lore";
    const departure = currentShellLocation();
    if (!(await leavePluginView())) return;
    recordShellDeparture(departure);
    section = target;
    await selectEntity(entity, false);
  } catch (cause) {
    error = friendlyError(cause);
  }
}

async function unlinkMapLocation(location: MapLocation) {
  if (
    !selected ||
    !(await confirmDialog({
      title: "Unlink this location?",
      message: `${location.label || "This location"} will be unlinked from ${selected.name}. The entity and map feature will remain.`,
      confirmLabel: "Unlink",
      danger: true,
    }))
  )
    return;
  try {
    await project.unlinkMapLocation(selected.id, location.id);
    mapLocations = await project.listMapLocations(selected.id);
  } catch (cause) {
    error = friendlyError(cause);
  }
}

async function editMapLocation(location: MapLocation) {
  if (!selected) return;
  const role = await promptDialog({
    title: "Edit location role",
    message: "Describe what this location represents for this entity.",
    value: location.role,
    placeholder: "e.g. Birthplace, Residence",
    confirmLabel: "Save",
  });
  if (role === null) return;
  const nextRole = role.trim();
  if (!nextRole) return;
  try {
    await project.upsertMapLocation(selected.id, { ...location, role });
    mapLocations = await project.listMapLocations(selected.id);
  } catch (cause) {
    error = friendlyError(cause);
  }
}

async function rebindMapLocation(location: MapLocation) {
  if (!selected) return;
  try {
    await beginMapPick({ kind: "rebind", entityId: selected.id, location, mapEntityId: location.mapEntityId });
  } catch (cause) {
    error = friendlyError(cause);
  }
}

async function selectEntity(entity: Entity, recordHistory = true) {
  if (selected?.id === entity.id) return;
  if (!(await flushAutoSave())) return;
  if (section === "language" && !(await canLeaveLanguageSection())) return;
  const departure = currentShellLocation();
  if (section === "maps" && sandboxView?.renderer === "maps" && !(await leavePluginView())) return;
  if (recordHistory) recordShellDeparture(departure);
  editorFullscreen = false;
  selected = entity;
  applyCollectionTabForEntityType(entity.entity_type);
  hasUnsavedChanges = false;
  documentConflict = null;
  documentRevision = 0;
  saveError = "";
  error = "";
  try {
    await loadSelectedState(entity);
  } catch (cause) {
    error = friendlyError(cause);
  }
}

async function openSelectedMapEditor() {
  if (!selected || selected.entity_type !== "daena.maps:world-map") return;
  const mapsView = mapsNavigationItem();
  if (!mapsView) {
    error = "The Maps integration is not available.";
    return;
  }
  try {
    mapsEditorKey = selected.id;
    mapFocusLinkId = null;
    mapFocusFeatureId = null;
    await openPluginView(mapsView);
  } catch (cause) {
    error = friendlyError(cause);
  }
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
  saveError = "";
}

function handlePortableFilesChanged(paths: string[]) {
  if (paths.length === 0) return;
  projectDiagnostics = [
    `Portable files changed externally (${paths.length}); review and import the checkpoint explicitly if needed.`,
  ];
}

async function reloadConflict() {
  try {
    await reloadSelectedFromDisk();
  } catch (cause) {
    error = friendlyError(cause);
  }
}

async function overwriteConflict() {
  if (!selected) return;
  try {
    const [documents, storedFields] = await Promise.all([
      project.listDocuments(selected.id),
      project.listFields(selected.id),
    ]);
    loadedDocumentRevision = documents[0]?.revision ?? "";
    loadedFieldRevisions = Object.fromEntries(
      storedFields.map((field) => [fieldRevisionKey(field.namespace, field.key), field.revision]),
    );
    loadedStructuredFieldKeys = new Set(
      storedFields
        .filter((field) => isStructuredFieldValue(field.value))
        .map((field) => fieldRevisionKey(field.namespace, field.key)),
    );
    documentConflict = null;
    conflictDiskBody = "";
    if (!(await saveDocument()) && !documentConflict && !saveError)
      documentConflict = { paths: [], diagnostics: ["The draft could not be written as a new revision."] };
  } catch (cause) {
    documentConflict = { paths: [], diagnostics: [friendlyError(cause)] };
  }
}

async function saveConflictRecoveryCopy() {
  if (!selected) return;
  try {
    const path = await project.saveRecoveryCopy(selected.id, documentBody);
    error = `Draft saved as ${path}`;
  } catch (cause) {
    error = friendlyError(cause);
  }
}

async function createWithOption(
  option: CreateOption,
  requestedName: string,
  values: Record<string, unknown>,
  openingDocument: string,
) {
  if (projectDiagnostics.length > 0 || createBusy || !requestedName.trim() || !option.module.enabled) return false;
  const departure = currentShellLocation();
  createBusy = true;
  try {
    const fieldsForCreate: Record<string, unknown> = {};
    const relationshipsForCreate: Record<string, UUID[]> = {};
    for (const { field, required } of createFieldsFor(option)) {
      const value = values[field.key];
      const empty =
        value === "" ||
        value === null ||
        value === undefined ||
        (typeof value === "string" && value.trim() === "") ||
        (Array.isArray(value) && value.length === 0);
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
    const context = buildModuleContext(option.module, projectInfo?.root ?? "", {
      availableServices: enabledServices(),
    });
    const created = await context.entities.create({
      name: requestedName.trim(),
      type: option.template.entityType,
      fields: fieldsForCreate,
      relationships: relationshipsForCreate,
      document: openingDocument.trim() ? { body: openingDocument.trim(), format: "markdown" } : undefined,
    });
    recordShellDeparture(departure);
    const createdType = option.template.entityType ?? "";
    const isHouse = createdType === HOUSE_TYPE || createdType === "house" || createdType.endsWith(":house");
    const isPerson = createdType === PERSON_TYPE || createdType.endsWith(":person");
    const fromTree = departure.kind === "workspace" && departure.section === "houses" && departure.view === "tree";
    projectHomeOpen = false;
    name = "";
    showCreateForm = false;
    createDialogView = "templates";
    const returnFocus = createDialogReturnFocus;
    createDialogReturnFocus = null;
    resetCreateFields(null);
    await refreshAfterEntityMutation({ entityId: created.id });
    if (fromTree && (isHouse || isPerson)) {
      section = "houses";
      housesView = "tree";
      familyTreeRootId = created.id;
      familyTreeSession = {
        expansions: [],
        selectedPersonId: null,
        selectedRelationshipId: null,
        viewport: null,
        houseId: isHouse ? created.id : null,
      };
      familyTreeRestoreNonce += 1;
      clearSelection();
    } else {
      section =
        workspaceSectionOrder.find((target) =>
          manifestForWorkspaceSection(target)?.schemas.some((schema) =>
            schemaEntityTypeIds(schema).includes(option.template.entityType),
          ),
        ) ?? section;
      applyCollectionTabForEntityType(option.template.entityType);
      await selectEntity(
        {
          id: created.id,
          name: created.name,
          entity_type: created.type,
          deleted: created.deleted,
          created_at: created.createdAt,
          updated_at: created.updatedAt,
          revision: "",
        },
        false,
      );
    }
    void tick().then(() => returnFocus?.focus());
    return true;
  } catch (cause) {
    error = friendlyError(cause);
    return false;
  } finally {
    createBusy = false;
  }
}

async function createEntity(event: SubmitEvent) {
  event.preventDefault();
  const option = selectedCreateOption();
  if (!option) return;
  await createWithOption(option, name, createFieldValues, createDocumentBody);
}

function closeCreateForm() {
  requestCreateDiscard(() => {
    showCreateForm = false;
    createDialogView = "templates";
    name = "";
    resetCreateFields(null);
    const returnFocus = createDialogReturnFocus;
    createDialogReturnFocus = null;
    void tick().then(() => returnFocus?.focus());
  });
}

function rememberCreateReturnFocus() {
  if (!showCreateForm)
    createDialogReturnFocus = document.activeElement instanceof HTMLElement ? document.activeElement : null;
}

function activateCreateOption(option: CreateOption, initialName = "") {
  const switchingTemplate = option.key !== selectedCreateKey;
  if (switchingTemplate || Object.keys(createFieldValues).length === 0) {
    name = initialName;
    selectedCreateKey = option.key;
    resetCreateFields(option);
  } else if (initialName) name = initialName;
  createDialogView = "form";
  showCreateForm = true;
  setTimeout(() => document.getElementById("new-entity")?.focus(), 0);
}

function openFocusedCreate(optionKey?: string, initialName = "") {
  const options = createOptions();
  if (options.length === 0) {
    error = "Enable a module with a creation template to get started.";
    return;
  }
  const option = options.find((candidate) => candidate.key === optionKey) ?? defaultCreateOption(options);
  if (!option) return;
  rememberCreateReturnFocus();
  const activate = () => activateCreateOption(option, initialName);
  if (option.key !== selectedCreateKey && hasCreateValues()) requestCreateDiscard(activate);
  else activate();
}

function createOptionKeyForEntityType(entityType: string) {
  return (
    createOptions().find((option) => option.template.entityType === entityType)?.key ??
    createOptions().find((option) => option.template.entityType.endsWith(`:${entityType.split(":").at(-1)}`))?.key ??
    null
  );
}

function openNewPerson() {
  const key = createOptionKeyForEntityType(PERSON_TYPE);
  if (!key) {
    error = "Enable Lore with a Person template to create people.";
    return;
  }
  openFocusedCreate(key);
}

function openNewHouse() {
  const key = createOptionKeyForEntityType(HOUSE_TYPE) ?? createOptionKeyForEntityType("house");
  if (!key) {
    error = "Enable Houses with a House template to create houses.";
    return;
  }
  openFocusedCreate(key);
}

function openCreationMenu() {
  const options = createOptions();
  if (options.length === 0) {
    error = "Enable a module with a creation template to get started.";
    return;
  }
  rememberCreateReturnFocus();
  createDialogView = "templates";
  showCreateForm = true;
  setTimeout(() => createDialogElement?.querySelector<HTMLButtonElement>("[data-template-index]")?.focus(), 0);
}

function openContextualCreate() {
  if (section === "houses" && housesView === "tree") {
    openNewPerson();
    return;
  }
  const options = createOptions();
  if (options.length === 0) {
    error = "Enable a module with a creation template to get started.";
    return;
  }
  const option = defaultCreateOption(options);
  if (!option) return;
  openFocusedCreate(option.key);
}

function returnToCreationMenu() {
  createDialogView = "templates";
  setTimeout(
    () =>
      createDialogElement?.querySelector<HTMLButtonElement>(`[data-template-index="${selectedCreateKey}"]`)?.focus(),
    0,
  );
}

function toggleCreateForm() {
  if (showCreateForm) closeCreateForm();
  else openCreationMenu();
}

function handleCreateDialogKeydown(event: KeyboardEvent) {
  const templateButton = (event.target as HTMLElement | null)?.closest<HTMLButtonElement>("[data-template-index]");
  if (templateButton && ["ArrowDown", "ArrowUp", "ArrowLeft", "ArrowRight"].includes(event.key)) {
    const buttons = Array.from(createDialogElement?.querySelectorAll<HTMLButtonElement>("[data-template-index]") ?? []);
    const index = buttons.indexOf(templateButton);
    if (index >= 0 && buttons.length > 0) {
      event.preventDefault();
      const columnCount = Math.max(
        1,
        getComputedStyle(templateButton.parentElement ?? templateButton).gridTemplateColumns.split(" ").length,
      );
      const offset =
        event.key === "ArrowDown"
          ? columnCount
          : event.key === "ArrowUp"
            ? -columnCount
            : event.key === "ArrowRight"
              ? 1
              : -1;
      buttons[(index + offset + buttons.length) % buttons.length]?.focus();
    }
    return;
  }
  trapModalTab(event, createDialogElement);
}

function quickOpenShortcutLabel() {
  if (typeof navigator === "undefined") return "Ctrl K";
  return /Mac|iPhone|iPad/.test(navigator.platform) ? "⌘K" : "Ctrl K";
}

function quickOpenItems(): QuickOpenItem[] {
  const query = globalQuery.trim();
  const entityItems: QuickOpenItem[] = (query ? (searchMatches ?? []) : recentlyUpdatedEntities().slice(0, 7)).map(
    (entity) => {
      const presentation = iconForEntityType(entity.entity_type);
      return {
        id: `entity:${entity.id}`,
        category: query ? "Results" : "Recent",
        label: entity.name,
        description: entityTypeLabel(entity.entity_type),
        keywords: [entity.entity_type ?? ""],
        icon: presentation.icon,
        pluginId: presentation.pluginId,
        iconColor: presentation.iconColor,
        action: { kind: "entity", entityId: entity.id },
      };
    },
  );
  const mapFeatureItems: QuickOpenItem[] = query
    ? (mapFeatureMatches ?? []).map((feature) => ({
        id: `map-feature:${feature.mapEntityId}:${feature.featureId}`,
        category: "Results",
        label: feature.name,
        description: `${feature.semanticType} · ${feature.layerName} · ${feature.mapName}`,
        keywords: [feature.featureId, feature.semanticType, feature.layerName, feature.mapName],
        action: { kind: "map-feature", mapEntityId: feature.mapEntityId, featureId: feature.featureId },
      }))
    : [];
  const destinations: QuickOpenItem[] = [
    {
      id: "destination:home",
      category: "Destinations",
      label: "Project Home",
      description: "Overview, workspaces, and recent activity",
      keywords: ["home", "overview"],
      action: { kind: "destination", destination: "home" },
    },
    ...workspaceNavigationItems().map((item): QuickOpenItem => ({
      id: `destination:${item.key}`,
      category: "Destinations",
      label: item.title,
      description: `${workspaceDescription(item.section)} workspace`,
      keywords: [item.section, "workspace"],
      action: { kind: "destination", destination: `navigation:${item.key}` },
    })),
    ...pluginViews().map((item): QuickOpenItem => ({
      id: `destination:${item.key}`,
      category: "Destinations",
      label: pluginViewLabel(item),
      description: "Plugin tool",
      keywords: [item.plugin.name, item.view.title, "tool"],
      action: { kind: "destination", destination: `navigation:${item.key}` },
    })),
  ];
  const creation: QuickOpenItem[] = createOptions().map((option) => {
    const presentation = iconForCreateOption(option);
    return {
      id: `create:${option.key}`,
      category: "Create",
      label: `Create ${option.template.name}`,
      description: `${option.module.name} · open focused creation`,
      keywords: [option.module.name, option.template.entityType, option.template.description ?? ""],
      icon: presentation.icon,
      pluginId: presentation.pluginId,
      iconColor: presentation.iconColor,
      action: { kind: "create", templateKey: option.key },
    };
  });
  const commands: QuickOpenItem[] = [
    {
      id: "command:template-gallery",
      category: "Commands",
      label: "Browse creation templates",
      description: "Choose from every enabled template",
      keywords: ["new", "create", "gallery", "template"],
      action: { kind: "command", command: "template-gallery" },
    },
    {
      id: "command:snapshots",
      category: "Commands",
      label: "Open Snapshots",
      description: "Review project history and checkpoint changes",
      keywords: ["git", "history", "checkpoint"],
      action: { kind: "command", command: "snapshots" },
    },
    {
      id: "command:plugins",
      category: "Commands",
      label: "Manage Extensions",
      description: "Enable, disable, or inspect project extensions",
      keywords: ["extensions", "modules", "tools"],
      action: { kind: "command", command: "plugins" },
    },
    {
      id: "command:settings",
      category: "Commands",
      label: "Open Settings",
      description: "Application preferences",
      keywords: ["preferences", "configuration", "theme", "provider"],
      action: { kind: "command", command: "settings" },
    },
  ];
  return rankQuickOpenItems([...mapFeatureItems, ...entityItems, ...destinations, ...creation, ...commands], query, 80);
}

function openQuickOpen() {
  if (!ready || showCreateForm || entityEditDialog || showExternalImport) return;
  quickOpenOpen = true;
}

function closeQuickOpen() {
  quickOpenOpen = false;
  globalQuery = "";
  searchMatches = null;
  mapFeatureMatches = null;
  quickOpenSearchLoading = false;
  searchRequest += 1;
}

async function selectQuickOpenItem(item: QuickOpenItem) {
  const action = item.action;
  const matchedEntity =
    action.kind === "entity"
      ? [...(searchMatches ?? []), ...entities].find((candidate) => candidate.id === action.entityId)
      : null;
  closeQuickOpen();
  await tick();
  if (action.kind === "entity") {
    if (matchedEntity) await selectSearchResult(matchedEntity);
    return;
  }
  if (action.kind === "map-feature") {
    await ensureMapEditorOpen(action.mapEntityId);
    mapFocusLinkId = null;
    mapFocusFeatureId = null;
    await tick();
    mapFocusFeatureId = action.featureId;
    return;
  }
  if (action.kind === "destination") {
    if (action.destination === "home") await openProjectHome();
    else if (action.destination.startsWith("navigation:")) {
      openSidebarNavigationItem(action.destination.slice("navigation:".length));
    }
    return;
  }
  if (action.kind === "create") {
    const option = createOptions().find((candidate) => candidate.key === action.templateKey);
    if (!option) return;
    openFocusedCreate(option.key);
    return;
  }
  if (action.command === "template-gallery") openCreationMenu();
  else if (action.command === "snapshots") await openProjectCenter("snapshots");
  else if (action.command === "plugins") await openProjectCenter("extensions");
  else await openSettings();
}

function updateField(definition: FieldDefinition, event: Event) {
  if (selectedLoading || selectedLoadError || projectDiagnostics.length > 0) return;
  const target = event.currentTarget as HTMLInputElement | HTMLSelectElement;
  let value: unknown;
  if (definition.type === "boolean") {
    value = (target as HTMLInputElement).checked;
  } else if (definition.type === "number") {
    const raw = target.value;
    value = raw === "" ? "" : Number(raw);
  } else if (target instanceof HTMLSelectElement && target.multiple) {
    value = Array.from(target.selectedOptions, (option) => option.value);
  } else {
    value = target.value;
  }
  fields = { ...fields, [definition.key]: value };
  markEntryDirty();
}
function isRevisionConflict(cause: unknown) {
  return friendlyError(cause).toLowerCase().includes("revision conflict");
}

async function persistDocumentSnapshot(): Promise<boolean> {
  if (
    !selected ||
    selectedLoading ||
    selectedLoadError ||
    !sectionEnabled() ||
    documentConflict ||
    projectDiagnostics.length > 0
  )
    return false;
  const entityId = selected.id;
  const body = documentBody;
  const revision = documentRevision;
  const fieldsSnapshot = { ...fields };
  const definitionsForSave = definitions().filter((definition) => {
    if (definition.type === "relationship") return false;
    const namespace = namespaceForField(definition);
    const key = fieldRevisionKey(namespace, definition.key);
    return shouldPersistFieldValue(
      fieldsSnapshot[definition.key],
      Object.prototype.hasOwnProperty.call(loadedFieldRevisions, key),
    );
  });
  try {
    await project.saveEntry(
      {
        document: { entity_id: entityId, body, format: "markdown" },
        fields: definitionsForSave.map((definition) => {
          const namespace = namespaceForField(definition);
          const value = fieldValueForSave(definition, fieldsSnapshot[definition.key] ?? "");
          const persistedValue =
            definition.type === "date"
              ? value
                ? (parseCalendarDate(value) ?? value)
                : value
              : restoreStructuredFieldValue(
                  value,
                  loadedStructuredFieldKeys.has(fieldRevisionKey(namespace, definition.key)),
                  definition.label,
                );
          return {
            entity_id: entityId,
            namespace,
            key: definition.key,
            value: persistedValue,
            revision: loadedFieldRevisions[fieldRevisionKey(namespace, definition.key)] ?? "",
          };
        }),
      },
      { expectedRevision: loadedDocumentRevision || undefined },
    );
    const [documents, storedFields] = await Promise.all([
      project.listDocuments(entityId),
      project.listFields(entityId),
    ]);
    if (selected?.id === entityId) {
      loadedDocumentRevision = documents[0]?.revision ?? "";
      loadedFieldRevisions = Object.fromEntries(
        storedFields.map((field) => [fieldRevisionKey(field.namespace, field.key), field.revision]),
      );
      loadedStructuredFieldKeys = new Set(
        storedFields
          .filter((field) => isStructuredFieldValue(field.value))
          .map((field) => fieldRevisionKey(field.namespace, field.key)),
      );
    }
    if (selected?.id === entityId && documentRevision === revision) {
      hasUnsavedChanges = false;
      autoSaveFailureCount = 0;
      saveError = "";
      savedAt = new Date().toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" });
      void loadProjectStatus(false);
    }
    return true;
  } catch (cause) {
    if (isRevisionConflict(cause) && selected?.id === entityId) {
      try {
        const documents = await project.listDocuments(entityId);
        conflictDiskBody = normalizeDocument(documents[0]?.body ?? "", documents[0]?.format);
      } catch {
        conflictDiskBody = "";
      }
      documentConflict = { paths: [], diagnostics: [] };
      savedAt = "";
      saveError = "";
    } else {
      saveError = friendlyError(cause);
      autoSaveFailureCount += 1;
      const retryDelay = Math.min(30_000, 2_000 * 2 ** Math.min(autoSaveFailureCount - 1, 4));
      scheduleAutoSave(retryDelay);
    }
    return false;
  }
}

async function saveDocument(): Promise<boolean> {
  cancelAutoSave();
  if (saveInFlight) {
    saveQueued = true;
    const currentResult = await saveInFlight;
    if (!currentResult || !hasUnsavedChanges) return currentResult;
    return saveDocument();
  }
  if (!hasUnsavedChanges) return true;
  isSaving = true;
  saveQueued = false;
  const operation = persistDocumentSnapshot();
  saveInFlight = operation;
  const result = await operation;
  if (saveInFlight === operation) saveInFlight = null;
  isSaving = false;
  const shouldSaveQueuedChanges = result && saveQueued && hasUnsavedChanges;
  saveQueued = false;
  return shouldSaveQueuedChanges ? saveDocument() : result;
}
async function openEntityEditDialog(target: Entity | null = selected) {
  if (projectDiagnostics.length > 0) return;
  if (!target) return;
  if (selected?.id === target.id) {
    if (!(await flushAutoSave())) return;
  } else {
    await selectEntity(target);
    if (!(await flushAutoSave())) return;
  }
  try {
    const loaded = await project.getEntity(target.id);
    if (loaded && !loaded.deleted) upsertEntityInCache(loaded);
  } catch {}
  const current = entities.find((entity) => entity.id === target.id) ?? selected ?? target;
  entityEditDialog = { entity: current, name: current.name, entityType: current.entity_type, busy: false };
  entityMutation.reset();
  setTimeout(() => document.getElementById("entity-edit-name")?.focus(), 0);
}
function closeEntityEditDialog() {
  entityEditDialog = null;
  entityMutation.reset();
  clearSavedMutationTimer();
}

let savedMutationTimer: ReturnType<typeof setTimeout> | null = null;
function clearSavedMutationTimer() {
  if (savedMutationTimer) {
    clearTimeout(savedMutationTimer);
    savedMutationTimer = null;
  }
}
function scheduleClearSavedMutation() {
  clearSavedMutationTimer();
  savedMutationTimer = setTimeout(() => {
    savedMutationTimer = null;
    if (entityMutation.phase === "saved") entityMutation.reset();
  }, 1800);
}

async function saveEntityEditDialog() {
  if (!entityEditDialog) return;
  const trimmed = entityEditDialog.name.trim();
  if (!trimmed) {
    error = "Name cannot be empty.";
    return;
  }
  const current = entityEditDialog.entity;
  const fresh = entities.find((e) => e.id === current.id) ?? current;
  const nameChanged = trimmed !== fresh.name;
  const typeChanged = (entityEditDialog.entityType ?? null) !== (fresh.entity_type ?? null);
  if (!nameChanged && !typeChanged) {
    closeEntityEditDialog();
    return;
  }
  if (!fresh.revision) {
    try {
      const loaded = await project.getEntity(current.id);
      if (loaded && !loaded.deleted) {
        upsertEntityInCache(loaded);
        entityEditDialog.entity = loaded;
      }
    } catch {}
    const refreshed = entities.find((e) => e.id === current.id) ?? entityEditDialog.entity;
    if (!refreshed?.revision) {
      error = "The entity revision is unavailable. Reload the project and try again.";
      return;
    }
    entityEditDialog.entity = refreshed;
  }
  const target = entities.find((e) => e.id === current.id) ?? entityEditDialog.entity;
  entityEditDialog.busy = true;
  const result = await entityMutation.run(async () => {
    return project.updateEntity(
      target.id,
      nameChanged ? trimmed : null,
      typeChanged ? (entityEditDialog!.entityType ?? null) : null,
      { expectedRevision: target.revision },
    );
  }, trimmed);
  if (!result.ok) {
    error = friendlyError(result.error);
    if (entityEditDialog) entityEditDialog.busy = false;
    return;
  }
  const updated = result.value;
  selected = updated;
  await refreshAfterEntityMutation({ entityId: updated.id });
  if (typeChanged) {
    const newSection = sectionForEntityType(updated.entity_type);
    if (newSection && newSection !== section) section = newSection;
    applyCollectionTabForEntityType(updated.entity_type);
    await loadSelectedState(updated);
  } else {
    await loadSelectedState(updated).catch(() => {});
  }
  // Keep Saved chrome in the editor footer briefly; cancel path still resets via closeEntityEditDialog.
  entityEditDialog = null;
  scheduleClearSavedMutation();
}
async function renameSelected() {
  return openEntityEditDialog();
}

async function archiveEntity(target: Entity, options?: { skipConfirm?: boolean; returnFocus?: HTMLElement | null }) {
  if (projectDiagnostics.length > 0) return;
  if (selected?.id === target.id && !(await flushAutoSave())) return;
  const owning = contextOwningEntityType(target.entity_type ?? "");
  const result = await entityMutation.run(async () => {
    const loaded = await project.getEntity(target.id);
    const current = loaded && !loaded.deleted ? loaded : (entities.find((entity) => entity.id === target.id) ?? target);
    if (!current.revision) throw new Error("The entity revision is unavailable. Reload the project and try again.");
    await owning.entities.delete(current.id as UUID, { expectedRevision: current.revision });
    return current.name;
  }, target.name);
  if (!result.ok) {
    error = friendlyError(result.error);
    queueMicrotask(() => options?.returnFocus?.focus());
    return;
  }
  // Toast is the success signal for archive; do not leave a lingering Saved chrome.
  entityMutation.reset();
  const wasSelected = selected?.id === target.id;
  if (wasSelected) clearSelection();
  await refreshAfterEntityMutation({ entityId: target.id, removed: true });
  showLifecycleToast({
    message: archivedToastMessage(result.value),
    actionLabel: ENTITY_ACTIONS.viewArchive,
    onAction: () => {
      dismissLifecycleToast();
      void openProjectCenter("archive");
    },
  });
  await tick();
  const nextFocus =
    options?.returnFocus && document.contains(options.returnFocus)
      ? options.returnFocus
      : (collectionListElement?.querySelector<HTMLElement>(".collection-item-main, .row-actions-trigger") ??
        collectionPaneElement?.querySelector<HTMLElement>("button, [href], input") ??
        null);
  nextFocus?.focus();
}

async function archiveSelected() {
  if (!selected) return;
  return archiveEntity(selected, {
    returnFocus: document.activeElement instanceof HTMLElement ? document.activeElement : null,
  });
}

async function reloadEntityEditFromServer() {
  if (!entityEditDialog) return;
  try {
    const loaded = await project.getEntity(entityEditDialog.entity.id);
    if (!loaded || loaded.deleted) {
      error = "This entry is no longer available.";
      closeEntityEditDialog();
      return;
    }
    entities = entities.map((entity) => (entity.id === loaded.id ? loaded : entity));
    if (selected?.id === loaded.id) selected = loaded;
    entityEditDialog = {
      entity: loaded,
      name: loaded.name,
      entityType: loaded.entity_type,
      busy: false,
    };
    entityMutation.reset();
  } catch (cause) {
    error = friendlyError(cause);
  }
}

async function openHouseTree(entity: Entity) {
  if (entity.entity_type !== HOUSE_TYPE) return;
  if (!(await flushAutoSave())) return;
  const departure = currentShellLocation();
  if (!(await leavePluginView())) return;
  recordShellDeparture(departure);
  projectHomeOpen = false;
  section = "houses";
  housesView = "tree";
  familyTreeRootId = entity.id;
  familyTreeSession = {
    expansions: [],
    selectedPersonId: null,
    selectedRelationshipId: null,
    viewport: null,
    houseId: entity.id,
  };
  familyTreeRestoreNonce += 1;
  clearSelection();
}
function selectedRelationshipIds(definition: FieldDefinition) {
  if (!selected || !definition.relationshipType) return [];
  return counterpartIds(selected.id, relationships, definition);
}
function relationshipsForDefinition(definition: FieldDefinition) {
  if (!selected) return [];
  return relationshipsForField(selected.id, relationships, definition);
}
function relationshipDefinitions() {
  return definitions().filter((candidate) => candidate.type === "relationship");
}
function eraRelationshipDefinition() {
  return definitions().find(isEraRelationshipField) ?? null;
}
function chronologyDateDefinitions() {
  return definitions().filter((candidate) => candidate.type === "date" && isChronologyDateKey(candidate.key));
}
function propertyDefinitions() {
  return definitions().filter((candidate) => candidate.type !== "relationship" && !isChronologyDateKey(candidate.key));
}
function otherRelationshipDefinitions() {
  return relationshipDefinitions().filter((candidate) => !isEraRelationshipField(candidate));
}
function hasChronologySection() {
  return Boolean(eraRelationshipDefinition()) || chronologyDateDefinitions().length > 0;
}
function inspectorChronologyWarnings() {
  return chronologyWarnings(
    chronologyDateDefinitions().map((definition) => ({ label: definition.label, value: fields[definition.key] })),
    eraContexts,
  );
}
async function loadEraContexts(eraIds: string[]): Promise<EraContext[]> {
  const unique = [...new Set(eraIds.filter(Boolean))];
  return Promise.all(
    unique.map(async (id) => {
      let start: unknown;
      let end: unknown;
      let calendarIds: string[] = [];
      try {
        const stored = await project.listFields(id);
        start = stored.find((field) => field.key === "startsAt")?.value;
        end = stored.find((field) => field.key === "endsAt")?.value;
      } catch {}
      try {
        const linked = await project.listRelationships(id);
        calendarIds = linked
          .filter((relationship) => relationship.relationship_type === "uses_calendar" && relationship.source_id === id)
          .map((relationship) => relationship.target_id);
      } catch {}
      return {
        id,
        name: entities.find((entity) => entity.id === id)?.name ?? id,
        start,
        end,
        calendarIds,
      };
    }),
  );
}
function hintDateCalendarsFromEras(contexts: EraContext[], persistDates = false) {
  const calendarId = firstEraCalendarId(contexts);
  if (!calendarId) return;
  for (const definition of chronologyDateDefinitions()) {
    const current = dateForField(definition.key);
    if (current && !isGregorianCalendarId(current.calendar)) continue;
    dateCalendarByField = { ...dateCalendarByField, [definition.key]: calendarId };
    if (persistDates && current) {
      fields = { ...fields, [definition.key]: serializeCalendarDate({ ...current, calendar: calendarId }) };
      markEntryDirty();
    }
  }
}
function hintCreateDateCalendarsFromEras(contexts: EraContext[]) {
  const calendarId = firstEraCalendarId(contexts);
  if (!calendarId) return;
  for (const key of ["startsAt", "endsAt"]) {
    const current = createDateForField(key);
    if (current && !isGregorianCalendarId(current.calendar)) continue;
    createDateCalendarByField = { ...createDateCalendarByField, [key]: calendarId };
    if (current) setCreateField(key, serializeCalendarDate({ ...current, calendar: calendarId }));
  }
}
function backlinkRelationships() {
  if (!selected) return [];
  const covered = coveredRelationshipIds(selected.id, relationships, otherRelationshipDefinitions());
  return relationships.filter(
    (relationship) => relationship.target_id === selected!.id && !covered.has(relationship.id),
  );
}
function relationshipSourceName(relationship: Relationship) {
  return entities.find((entity) => entity.id === relationship.source_id)?.name ?? relationship.source_id;
}
function definitionForRelationship(relationship: Relationship): FieldDefinition | null {
  const manifests = modules.filter((module) => module.enabled).map((module) => module as unknown as ModuleManifest);
  if (manifests.length === 0) {
    const fallback = activeManifest();
    if (fallback) manifests.push(fallback);
  }
  for (const manifest of manifests) {
    for (const schema of manifest.schemas ?? []) {
      const definition = schema.fields.find(
        (field) =>
          field.type === "relationship" &&
          field.relationshipType === relationship.relationship_type &&
          field.metadataFields !== undefined,
      );
      if (definition) return definition;
    }
  }
  return null;
}
function relationshipOtherId(definition: FieldDefinition, relationship: Relationship) {
  return counterpartId(selected?.id ?? "", relationship, definition) ?? relationship.target_id;
}
function relationshipTargetName(relationship: Relationship, definition?: FieldDefinition) {
  const id = definition && selected ? relationshipOtherId(definition, relationship) : relationship.target_id;
  return entities.find((entity) => entity.id === id)?.name ?? id;
}
function relationshipMetadataSummary(relationship: Relationship, definition: FieldDefinition | null) {
  let metadata: Record<string, unknown> = {};
  try {
    const parsed: unknown = JSON.parse(relationship.metadata || "{}");
    if (typeof parsed === "object" && parsed !== null && !Array.isArray(parsed)) {
      metadata = parsed as Record<string, unknown>;
    }
  } catch {
    return "Metadata needs repair";
  }
  const fieldByKey = new Map((definition?.metadataFields ?? []).map((field) => [field.key, field]));
  const labels = new Map((definition?.metadataFields ?? []).map((field) => [field.key, field.label]));
  return Object.entries(metadata)
    .filter(([, value]) => value !== undefined && value !== null && value !== "")
    .slice(0, 3)
    .map(([key, value]) => {
      const field = fieldByKey.get(key);
      if (field?.type === "date") {
        const parsed = parseCalendarDate(value);
        if (parsed) {
          const calendar = calendarDefinitionForId(parsed.calendar);
          const formatted = formatWithCalendar(value, calendar);
          if (formatted !== "Undated") return `${labels.get(key) ?? key}: ${formatted}`;
        }
      }
      return `${labels.get(key) ?? key}: ${fieldDisplayValue(value)}`;
    })
    .join(" · ");
}
function openRelationshipMetadata(relationship: Relationship) {
  metadataDialog = { relationship, definition: definitionForRelationship(relationship) };
}
async function saveRelationshipMetadata(relationship: Relationship, metadata: Record<string, unknown>) {
  const updated = await project.updateRelationship(relationship.id, metadata, {
    expectedRevision: relationship.revision,
  });
  relationships = relationships.map((current) => (current.id === updated.id ? updated : current));
}
async function confirmRemoveRelationship(definition: FieldDefinition, relationship: Relationship) {
  const target = relationshipTargetName(relationship, definition);
  const label = definition.label ?? relationship.relationship_type;
  if (
    !(await confirmDialog({
      title: `Remove ${label}?`,
      message: `${target} will be unlinked from ${selected?.name ?? "this entry"}. The relationship can be recreated.`,
      confirmLabel: "Remove",
      danger: true,
    }))
  )
    return;
  const otherId = relationshipOtherId(definition, relationship);
  await updateRelationshipField(
    definition,
    selectedRelationshipIds(definition).filter((id) => id !== otherId),
  );
}
function contextOwningRelationshipType(relationshipType: string): ModuleContext {
  const owner = manifestOwningRelationshipType(
    relationshipType,
    modules.filter((module) => module.enabled),
  );
  if (owner && projectInfo?.root) {
    return buildModuleContext(owner as unknown as ModuleManifest, projectInfo.root, {
      availableServices: enabledServices(),
    });
  }
  return contextFor();
}
function contextOwningEntityType(entityType: string): ModuleContext {
  const owner = modules.find(
    (module) =>
      module.enabled &&
      (module.schemas ?? []).some((schema) => (schema.entityTypes ?? []).some((type) => type.id === entityType)),
  );
  if (owner && projectInfo?.root) {
    return buildModuleContext(owner as unknown as ModuleManifest, projectInfo.root, {
      availableServices: enabledServices(),
    });
  }
  return contextFor();
}
async function createRelationshipTarget(definition: FieldDefinition, name: string): Promise<string | null> {
  const targetType = definition.targetEntityTypes?.[0];
  if (!targetType || projectDiagnostics.length > 0) return null;
  try {
    const created = await contextOwningEntityType(targetType).entities.create(
      { name: name.trim(), type: targetType },
      { requestId: crypto.randomUUID() },
    );
    await refreshAfterEntityMutation({ entityId: created.id });
    return created.id;
  } catch (cause) {
    error = friendlyError(cause);
    return null;
  }
}
function toHostRelationship(relationship: {
  id: string;
  sourceId: string;
  targetId: string;
  type: string;
  metadata: Record<string, unknown>;
  revision: string;
}): Relationship {
  return {
    id: relationship.id,
    source_id: relationship.sourceId,
    target_id: relationship.targetId,
    relationship_type: relationship.type,
    metadata: JSON.stringify(relationship.metadata ?? {}),
    revision: relationship.revision,
  };
}
async function updateRelationshipField(definition: FieldDefinition, targetIds: string[]) {
  if (projectDiagnostics.length > 0) return;
  if (!selected || !definition.relationshipType) return;
  const desired = new Set(targetIds);
  const current = relationshipsForDefinition(definition);
  const currentIds = new Set(
    current
      .map((relationship) => counterpartId(selected!.id, relationship, definition))
      .filter((id): id is string => Boolean(id)),
  );
  const toRemove = current.filter((relationship) => {
    const otherId = counterpartId(selected!.id, relationship, definition);
    return !otherId || !desired.has(otherId);
  });
  const toAdd = [...desired].filter((targetId) => !currentIds.has(targetId));
  try {
    const context = contextOwningRelationshipType(definition.relationshipType);
    await Promise.all(
      toRemove.map((relationship) =>
        context.relationships.delete(relationship.id as UUID, relationship.relationship_type, {
          expectedRevision: relationship.revision,
        }),
      ),
    );
    const created = [];
    for (const otherId of toAdd) {
      const endpoints = endpointsForCreate(selected!.id, otherId, definition);
      const source =
        selected.id === endpoints.sourceId && selected.revision
          ? selected
          : await project.getEntity(endpoints.sourceId);
      const sourceRevision = source?.revision ?? "";
      if (!sourceRevision) throw new Error("The entity revision is unavailable. Reload the project and try again.");
      const createdRelationship = await context.relationships.create(
        {
          sourceId: endpoints.sourceId as UUID,
          targetId: endpoints.targetId as UUID,
          type: definition.relationshipType!,
          metadata: defaultRelationshipMetadata(definition),
        },
        { expectedRevision: sourceRevision, requestId: crypto.randomUUID() },
      );
      created.push(toHostRelationship(createdRelationship));
    }
    const removedIds = new Set(toRemove.map((relationship) => relationship.id));
    relationships = [...relationships.filter((relationship) => !removedIds.has(relationship.id)), ...created];
    if (isEraRelationshipField(definition)) {
      eraContexts = await loadEraContexts(targetIds);
      hintDateCalendarsFromEras(eraContexts, true);
    }
  } catch (cause) {
    error = friendlyError(cause);
  }
}
function mimeTypeFor(filename: string) {
  const extension = filename.split(".").pop()?.toLowerCase();
  return extension === "png"
    ? "image/png"
    : extension === "jpg" || extension === "jpeg"
      ? "image/jpeg"
      : extension === "gif"
        ? "image/gif"
        : extension === "webp"
          ? "image/webp"
          : extension === "mp4"
            ? "video/mp4"
            : extension === "webm"
              ? "video/webm"
              : "application/octet-stream";
}
function canWriteAssets() {
  return section === "lore" && (activeManifest()?.capabilities.includes("asset.write:self") ?? false);
}
function canUseAsProfile(asset: Asset) {
  return ["image/png", "image/jpeg", "image/gif", "image/webp"].includes(asset.mime_type);
}
async function refreshSelectedAssets() {
  if (!selected) {
    assets = [];
    return;
  }
  const namespaces = new Set(activeManifest()?.schemas.map((schema) => schema.namespace) ?? []);
  assets = (await project.listAssets(selected.id)).filter((asset) => namespaces.has(asset.namespace));
}
async function attachAsset() {
  if (projectDiagnostics.length > 0) return;
  if (!selected) return;
  try {
    const selection = await project.pickFile();
    const source = typeof selection === "string" ? selection : null;
    if (!source) return;
    const filename = source.split(/[\\/]/).pop() ?? "asset";
    const asset = await project.registerAssetFile({
      entity_id: selected.id,
      namespace: primarySchemaNamespace(activeManifest()?.schemas, {
        entityType: selected.entity_type,
        fallback: activeModuleId(),
      }),
      source_path: source,
      filename,
      mime_type: mimeTypeFor(filename),
    });
    assets = [...assets, asset];
  } catch (cause) {
    error = friendlyError(cause);
  }
}
function openAssetDialog(asset: Asset) {
  if (projectDiagnostics.length > 0) return;
  assetDialog = asset;
}
async function handleAssetSave(update: {
  filename?: string;
  role?: "attachment" | "profile";
  referenceScope?: "entity" | "project";
}) {
  const current = assetDialog;
  if (!current) return;
  assetBusyId = current.id;
  try {
    await project.updateAssetMetadata(current.id, update, current.revision);
    await refreshSelectedAssets();
  } catch (cause) {
    error = friendlyError(cause);
    throw cause;
  } finally {
    assetBusyId = null;
  }
}
async function handleAssetDelete() {
  const current = assetDialog;
  if (!current) return;
  if (
    !(await confirmDialog({
      title: `Delete ${current.filename}?`,
      message: "This file will be permanently removed from the project and the checkpoint. This cannot be undone.",
      confirmLabel: "Delete",
      danger: true,
    }))
  )
    return;
  assetBusyId = current.id;
  try {
    await project.deleteAsset(current.id, current.revision);
    await refreshSelectedAssets();
  } catch (cause) {
    error = friendlyError(cause);
    throw cause;
  } finally {
    assetBusyId = null;
  }
}
async function handleAssetReplace() {
  const current = assetDialog;
  if (!current) return;
  const selection = await project.pickFile();
  const source = typeof selection === "string" ? selection : null;
  if (!source) return;
  const replacementName = source.split(/[\\/]/).pop() ?? "replacement";
  if (
    !(await confirmDialog({
      title: `Replace ${current.filename}?`,
      message: `Use ${replacementName} as the new content? The attachment name and metadata will stay unchanged.`,
      confirmLabel: "Replace",
    }))
  )
    return;
  assetBusyId = current.id;
  try {
    await project.replaceAssetFile(current.id, source, mimeTypeFor(replacementName), current.revision);
    await refreshSelectedAssets();
    // refresh dialog with new revision
    const refreshed = assets.find((a) => a.id === current.id) ?? null;
    if (refreshed) assetDialog = refreshed;
  } catch (cause) {
    error = friendlyError(cause);
    throw cause;
  } finally {
    assetBusyId = null;
  }
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
        await refreshSelectedMapLocations();
        await reconcileWorkspaceSection();
        await refreshAdmin();
      },
      installed.capabilities,
    );
    return;
  }
  try {
    if (id === "daena.maps" && !(await resolveDirtyMapSession())) return;
    await project.disableModule(id);
    modules = await project.listModuleManifests();
    await refreshSelectedMapLocations();
    await reconcileWorkspaceSection();
    await refreshAdmin();
    if (!selectedCreateOption()) selectedCreateKey = "";
    if (showCreateForm && !selectedCreateOption()) closeCreateForm();
  } catch (cause) {
    error = friendlyError(cause);
  }
}
async function refreshAdmin() {
  adminBusy = true;
  try {
    const view = await project.adminView();
    adminPlugins = view.plugins;
  } catch (cause) {
    error = friendlyError(cause);
  } finally {
    adminBusy = false;
  }
}
function setAdminPluginEnabled(id: string, enabled: boolean) {
  if (!adminPlugins) return;
  adminPlugins = adminPlugins.map((plugin) =>
    plugin.id === id ? { ...plugin, enabled, runtimeRunning: enabled ? plugin.runtimeRunning : false } : plugin,
  );
}
async function openSettings(nextSection: SettingsSection = "general") {
  if (!(await flushAutoSave())) return;
  if (showSettings && settingsSurface === "application" && settingsSection === nextSection) return;
  if (!showSettings && section === "language" && !(await canLeaveLanguageSection())) return;
  const departure = currentShellLocation();
  if (showSettings && !(await beforeAdministrationNavigate())) return;
  if (!(await leavePluginView())) return;
  recordShellDeparture(departure);
  showSettings = true;
  settingsSurface = "application";
  settingsSection = nextSection;
  projectionView = null;
  installSummary = null;
  deleteBackupPath = "";
  showProjectMenu = false;
}
async function openProjectCenter(nextSection: ProjectSection = "overview") {
  if (!ready || !(await flushAutoSave())) return;
  if (showSettings && settingsSurface === "project" && projectSection === nextSection) return;
  if (!showSettings && section === "language" && !(await canLeaveLanguageSection())) return;
  const departure = currentShellLocation();
  const wasEditingFields =
    showSettings && settingsSurface === "project" && projectSection === "fields" && !!schemaPluginId;
  if (showSettings && !(await beforeAdministrationNavigate(nextSection))) return;
  if (!(await leavePluginView())) return;
  recordShellDeparture(departure);
  showSettings = true;
  settingsSurface = "project";
  projectSection = nextSection;
  projectionView = null;
  installSummary = null;
  deleteBackupPath = "";
  showProjectMenu = false;
  if (nextSection === "extensions") {
    adminPlugins = null;
    await refreshAdmin();
  }
  if (nextSection === "fields") {
    if (schemaPluginId && !moduleSupportsSchemaOverlay(schemaPluginId)) {
      schemaPluginId = null;
      schemaPluginName = "";
      moduleSchemaPackage = null;
    }
    if (schemaPluginId && (!wasEditingFields || !isSchemaEditorDirty())) {
      await refreshModuleSchemaEditor(schemaPluginId);
    }
  }
  if (nextSection === "advanced") {
    showAiIndexMessage("");
    await refreshAiIndexStatus();
  }
  await refreshArchivedCount();
}
function closeSettings() {
  recordCurrentShellLocation();
  showSettings = false;
  settingsSurface = "application";
  settingsSection = "general";
  projectSection = "overview";
  statusCenterOpen = false;
}
function setSchemaEditorDirty(dirty: boolean) {
  schemaEditorDirty = dirty;
}
async function beforeSettingsNavigate(next: SettingsSection | null): Promise<boolean> {
  if (next && next !== settingsSection) recordCurrentShellLocation();
  return true;
}
async function beforeProjectNavigate(next: ProjectSection | null): Promise<boolean> {
  if (projectSection === "fields" && next === "fields") return true;
  if (projectSection === "fields") {
    if (!(await allowLeaveSchemaEditor())) return false;
    schemaEditorDirty = false;
  }
  if (next && next !== projectSection) recordCurrentShellLocation();
  return true;
}
async function beforeVisibleSettingsNavigate(next: SettingsSection | null): Promise<boolean> {
  return beforeSettingsNavigate(next);
}
async function beforeVisibleProjectNavigate(next: ProjectSection | null): Promise<boolean> {
  return beforeProjectNavigate(next);
}
async function beforeAdministrationNavigate(nextProjectSection: ProjectSection | null = null): Promise<boolean> {
  if (!showSettings || settingsSurface === "application") return true;
  return beforeProjectNavigate(nextProjectSection);
}
/** Close settings from outside SettingsView (rail, plugin open, etc.). */
async function dismissSettings(): Promise<boolean> {
  if (!(await beforeAdministrationNavigate())) return false;
  showSettings = false;
  return true;
}
async function refreshModuleSchemaEditor(moduleId: string) {
  if (!ready) return;
  const token = ++schemaOverlayLoadToken;
  try {
    const editor = await project.loadModuleSchemaEditor(moduleId);
    if (token !== schemaOverlayLoadToken || schemaPluginId !== moduleId) return;
    schemaPluginName = editor.name;
    moduleSchemaPackage = { schemas: editor.schemas, templates: editor.templates };
    moduleSchemaOverlay = editor.overlay;
    schemaOverlayCache = { ...schemaOverlayCache, [moduleId]: editor.overlay };
    moduleSchemaRevision += 1;
    moduleSchemaMessage = "";
    void refreshSchemaEntityCounts();
  } catch (cause) {
    if (token !== schemaOverlayLoadToken || schemaPluginId !== moduleId) return;
    moduleSchemaMessage = friendlyError(cause);
  }
}

async function refreshSchemaEntityCounts() {
  if (!ready) return;
  try {
    const page = await project.queryEntities({ limit: 1, offset: 0 });
    const next: Record<string, number> = {};
    for (const entry of page.type_counts ?? []) {
      if (entry.entity_type) next[entry.entity_type] = entry.count;
    }
    schemaEntityCountsByType = next;
    schemaEntityCountsLoaded = true;
  } catch {
    schemaEntityCountsLoaded = false;
  }
}

function schemaEntityCountForType(typeId: string): number | null {
  if (!schemaEntityCountsLoaded) return null;
  return schemaEntityCountsByType[typeId] ?? 0;
}

async function reassignSchemaEntities(fromTypeId: string, toTypeId: string) {
  const limit = 100;
  let offset = 0;
  let moved = 0;
  for (;;) {
    const page = await project.queryEntities({
      entityTypes: [fromTypeId],
      offset,
      limit,
      archived: false,
    });
    for (const entity of page.items) {
      await project.updateEntity(entity.id, null, toTypeId, {
        expectedRevision: entity.revision,
      });
      moved += 1;
    }
    if (!page.has_more || page.items.length === 0) break;
    offset += page.items.length;
  }
  await refreshSchemaEntityCounts();
  bumpCollectionRefresh();
  if (moved > 0) {
    moduleSchemaMessage = `Reassigned ${moved} ${moved === 1 ? "entity" : "entities"} to the chosen type.`;
  }
}
function selectSchemaPlugin(moduleId: string | null) {
  if (moduleId && !moduleSupportsSchemaOverlay(moduleId)) {
    schemaPluginId = null;
    schemaPluginName = "";
    moduleSchemaPackage = null;
    moduleSchemaMessage = "";
    return;
  }
  schemaPluginId = moduleId;
  moduleSchemaMessage = "";
  if (!moduleId) {
    schemaPluginName = "";
    moduleSchemaPackage = null;
    return;
  }
  schemaPluginName = schemaOverlayCandidates().find((candidate) => candidate.id === moduleId)?.name ?? moduleId;
  moduleSchemaPackage = { schemas: [], templates: [] };
  moduleSchemaOverlay = { version: 1 };
  moduleSchemaRevision += 1;
  void refreshModuleSchemaEditor(moduleId);
}
$effect(() => {
  if (!schemaPluginId) return;
  if (!moduleSupportsSchemaOverlay(schemaPluginId)) {
    schemaPluginId = null;
    schemaPluginName = "";
    moduleSchemaPackage = null;
  }
});
async function saveModuleSchemaOverlay(overlay: ModuleSchemaOverlay) {
  if (!schemaPluginId) {
    moduleSchemaMessage = "Could not save: no plugin selected.";
    return;
  }
  const moduleId = schemaPluginId;
  moduleSchemaBusy = true;
  moduleSchemaMessage = "";
  try {
    const saved = await project.setModuleSchemaOverlay(moduleId, overlay);
    if (schemaPluginId !== moduleId) return;
    moduleSchemaOverlay = saved;
    schemaOverlayCache = { ...schemaOverlayCache, [moduleId]: saved };
    moduleSchemaRevision += 1;
    schemaEditorDirty = false;
    modules = await project.listModuleManifests();
    moduleSchemaMessage = "Saved.";
  } catch (cause) {
    moduleSchemaMessage = `Could not save: ${friendlyError(cause)}`;
  } finally {
    moduleSchemaBusy = false;
  }
}
$effect(() => {
  if (!showSettings || settingsSurface !== "project" || projectSection !== "extensions" || !ready) return;
  void refreshAdmin();
});
async function installFromPicker() {
  try {
    const selection = await project.pickPluginPackage();
    const source = typeof selection === "string" ? selection : null;
    if (source) await installPackage(source);
  } catch (cause) {
    error = friendlyError(cause);
  }
}
async function installPackage(path: string, allowUnsigned = false) {
  installing = true;
  installConsent = null;
  installSummary = null;
  try {
    const installed = await project.installPlugin(path, allowUnsigned);
    installSummary = {
      id: installed.id,
      version: installed.version,
      signed: installed.signed,
      digest: installed.digest,
    };
    await refreshAdmin();
    modules = await project.listModuleManifests();
  } catch (cause) {
    const message = friendlyError(cause);
    if (!allowUnsigned && message.toLowerCase().includes("unsigned")) {
      installConsent = { path, message };
    } else {
      error = message;
    }
  } finally {
    installing = false;
  }
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
  } catch (cause) {
    error = friendlyError(cause);
  }
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
  } catch (cause) {
    error = friendlyError(cause);
  } finally {
    upgradeBusy = false;
  }
}
function askConfirm(
  title: string,
  message: string,
  confirmLabel: string,
  run: () => Promise<void>,
  capabilities?: string[],
) {
  confirmAction = { title, message, confirmLabel, run, capabilities };
}
function selectedUninstallableVersion(plugin: PluginAdminEntry) {
  if (!plugin.distribution.canUninstall) return null;
  return plugin.installedVersions.find((version) => version.isSelected) ?? null;
}
async function runConfirm() {
  const action = confirmAction;
  if (!action) return;
  confirmBusy = true;
  try {
    await action.run();
    confirmAction = null;
  } catch (cause) {
    error = friendlyError(cause);
  } finally {
    confirmBusy = false;
  }
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
  } catch (cause) {
    error = friendlyError(cause);
  }
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
  } catch (cause) {
    error = friendlyError(cause);
  } finally {
    deleteBusy = false;
  }
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
          await refreshSelectedMapLocations();
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
    if (plugin.id === "daena.maps" && !(await resolveDirtyMapSession())) return;
    await project.disableModule(plugin.id);
    modules = await project.listModuleManifests();
    await refreshSelectedMapLocations();
    await reconcileWorkspaceSection();
    setAdminPluginEnabled(plugin.id, false);
    await refreshAdmin();
    if (hostView?.plugin.id === plugin.id) hostView = null;
    if (sandboxView?.plugin.id === plugin.id) sandboxView = null;
  } catch (cause) {
    error = friendlyError(cause);
  } finally {
    pluginActionId = null;
  }
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
function shortDigest(digest: string) {
  return digest ? digest.slice(0, 12) : "";
}
function installedAtLabel(timestamp: number) {
  return timestamp ? new Date(timestamp * 1000).toLocaleString() : "";
}
function runtimeTimestampLabel(timestamp: string) {
  try {
    const ms = Number(BigInt(timestamp) / 1_000_000n);
    const date = new Date(ms);
    return Number.isFinite(ms) && ms > 0 && !Number.isNaN(date.getTime()) ? date.toLocaleString() : "Unknown";
  } catch {
    return "Unknown";
  }
}
function clearSelection() {
  cancelAutoSave();
  editorFullscreen = false;
  hasUnsavedChanges = false;
  selected = null;
  selectedLoading = false;
  selectedLoadError = "";
  documentBody = "";
  fields = {};
  relationships = [];
  eraContexts = [];
  createEraContexts = [];
  metadataDialog = null;
  assets = [];
  savedAt = "";
  saveError = "";
  loadedDocumentRevision = "";
  loadedFieldRevisions = {};
  loadedStructuredFieldKeys = new Set();
  documentConflict = null;
  conflictDiskBody = "";
  projectDiagnostics = [];
  showCreateForm = false;
}
function resetProjectSessionState() {
  try {
    revokeAllResolvedAssetUrls();
  } catch {}
  selectedLoadToken += 1;
  closeAiFieldFill();
  closeAiRewrite();
  clearSelection();
  shellNavigationHistory = emptyShellNavigationHistory();
  shellNavigationBusy = false;
  collectionScrollBySection = {};
  pendingCollectionScroll = null;
  collectionQueryRestoring = false;
  projectHomeOpen = true;
  showExternalImport = false;
  projectInfo = null;
  projectStatusEpoch += 1;
  modules = [];
  adminPlugins = null;
  hostView = null;
  sandboxView = null;
  projectionView = null;
  gitStatus = null;
  gitMessage = "";
  mapSaveStates = {};
  mapsEditorKey = "welcome";
  mapReloadCounter = 0;
  mapRecoveryBusy = false;
  mapFocusLinkId = null;
  mapFocusFeatureId = null;
  mapSelection = null;
  mapPickPending = null;
  mapReconcileNotice = "";
  mapPickNotice = "";
  searchMatches = null;
  globalQuery = "";
  quickOpenOpen = false;
  quickOpenSearchLoading = false;
  collectionQuery.textSearch = "";
  showSettings = false;
  settingsSurface = "application";
  settingsSection = "general";
  projectSection = "overview";
  statusCenterOpen = false;
  showCreateForm = false;
  createDialogView = "templates";
  createMoreDetailsOpen = false;
  createDialogReturnFocus = null;
  schemaPluginId = null;
  schemaPluginName = "";
  moduleSchemaPackage = null;
  moduleSchemaOverlay = { version: 1 };
  moduleSchemaMessage = "";
  schemaEditorDirty = false;
  schemaOverlayLoadToken += 1;
  ready = false;
}
async function openExternalImport() {
  showProjectMenu = false;
  if (!(await flushAutoSave())) return;
  showExternalImport = true;
}
async function seedExample() {
  try {
    if (!(await flushAutoSave())) return;
    await project.seedExample();
    clearSelection();
    await loadEntities();
    modules = await project.listModuleManifests();
    error = "Example world seeded.";
  } catch (cause) {
    throw new Error(friendlyError(cause));
  }
}
async function rebuildSearchIndex() {
  const request = ++searchRequest;
  try {
    await project.rebuildSearch();
    const term = globalQuery.trim();
    if (!term || request !== searchRequest) return;
    const [matches, features] = await Promise.all([project.search(term), project.searchMapFeatures(term)]);
    if (request === searchRequest) {
      searchMatches = matches;
      mapFeatureMatches = features;
    }
  } catch (cause) {
    if (request === searchRequest) throw new Error(friendlyError(cause));
  }
}
async function importPortableCheckpoint() {
  await runProjectTransition("Importing checkpoint…", async () => {
    if (!(await flushAutoSave())) return;
    if (!(await leavePluginView())) return;
    await project.importCheckpoint();
    resetProjectSessionState();
    await finishOpening();
    projectDiagnostics = [];
  });
}
async function createPortableBackup() {
  if (!(await flushAutoSave())) throw new Error("Save the current draft before creating a backup.");
  return project.backup();
}
async function exportMarkdownProject() {
  if (projectTransitionBusy) return;
  showProjectMenu = false;
  try {
    const selection = await project.pickDirectory();
    const destination = typeof selection === "string" ? selection : null;
    if (!destination) return;
    let output = "";
    await runProjectTransition("Exporting Markdown…", async () => {
      if (!(await flushAutoSave())) return;
      output = await project.exportMarkdown(destination);
    });
    if (output) error = `Markdown export written to ${output}`;
  } catch (cause) {
    throw new Error(friendlyError(cause));
  }
}
async function createRecoveryBackup() {
  if (!(await flushAutoSave())) throw new Error("Save the current draft before creating a recovery backup.");
  return project.recoveryBackup();
}
async function restoreRecoveryBackup(path: string) {
  await runProjectTransition("Restoring recovery backup…", async () => {
    if (!(await flushAutoSave())) return;
    if (!(await leavePluginView())) return;
    await project.restoreRecoveryBackup(path);
    resetProjectSessionState();
    await finishOpening();
  });
}
$effect(() => {
  const term = globalQuery.trim();
  if (!ready || !quickOpenOpen || !term) {
    searchMatches = null;
    mapFeatureMatches = null;
    quickOpenSearchLoading = false;
    return;
  }
  const request = ++searchRequest;
  quickOpenSearchLoading = true;
  void Promise.all([project.search(term), project.searchMapFeatures(term)])
    .then(([matches, features]) => {
      if (request === searchRequest) {
        searchMatches = matches;
        mapFeatureMatches = features;
      }
    })
    .catch((cause) => {
      if (request === searchRequest) error = friendlyError(cause);
    })
    .finally(() => {
      if (request === searchRequest) quickOpenSearchLoading = false;
    });
});
onMount(() => {
  void appVersion().then((v) => (displayVersion = v));
  void loadRecentProjects();
  void closeNativePluginWebviews();
  const themeMedia = matchMedia("(prefers-color-scheme: dark)");
  const handleSystemThemeChange = () => {
    if (themePreference === "system")
      applyThemePreference(themePreference, document.documentElement, themeMedia.matches);
  };
  themeMedia.addEventListener("change", handleSystemThemeChange);
  const handleBeforeUnload = (event: BeforeUnloadEvent) => {
    editorRef?.flushPendingChanges();
    if (!hasUnsavedChanges) return;
    event.preventDefault();
    event.returnValue = "";
  };
  const handleHistoryKey = (event: KeyboardEvent) => {
    if (!ready || !event.altKey || event.metaKey || event.ctrlKey || event.shiftKey) return;
    if (event.key !== "ArrowLeft" && event.key !== "ArrowRight") return;
    event.preventDefault();
    void navigateShellHistory(event.key === "ArrowLeft" ? "back" : "forward");
  };
  const handleQuickOpenKey = (event: KeyboardEvent) => {
    const modifier = event.metaKey || event.ctrlKey;
    if (!ready || !modifier || event.altKey || event.shiftKey || event.repeat) return;
    const key = event.key.toLowerCase();
    if (key === "k") {
      event.preventDefault();
      if (quickOpenOpen) closeQuickOpen();
      else openQuickOpen();
    } else if (key === "n" && !quickOpenOpen && !showCreateForm && !entityEditDialog && !showExternalImport) {
      event.preventDefault();
      // Global New (⌘/Ctrl+N) opens the full template gallery.
      openCreationMenu();
    }
  };
  const handleWorkbenchResize = () => {
    workbenchViewportWidth = window.innerWidth;
    if (restoredWorkspacePaneDimensions)
      restoredWorkspacePaneDimensions = { ...restoredWorkspacePaneDimensions, viewportWidth: window.innerWidth };
  };
  window.addEventListener("beforeunload", handleBeforeUnload);
  window.addEventListener("keydown", handleHistoryKey);
  window.addEventListener("keydown", handleQuickOpenKey);
  window.addEventListener("resize", handleWorkbenchResize);
  let closeListenerDisposed = false;
  let unlistenWindowClose: (() => void) | undefined;
  void getCurrentWindow()
    .onCloseRequested(async (event) => {
      if (!ready) return;
      event.preventDefault();
      if (!(await flushAutoSave())) {
        error = "The window stayed open because the current draft could not be saved.";
        return;
      }
      if (!(await leavePluginView())) return;
      await getCurrentWindow().destroy();
    })
    .then((cleanup) => {
      if (closeListenerDisposed) cleanup();
      else unlistenWindowClose = cleanup;
    })
    .catch(() => {});
  let unlisten: (() => void) | undefined;
  void listen<string[]>("project-portable-files-changed", (event) => handlePortableFilesChanged(event.payload))
    .then((cleanup) => {
      unlisten = cleanup;
    })
    .catch(() => {});
  let unlistenCheckpointExport: (() => void) | undefined;
  void listen("project-checkpoint-export-status", () => {
    void refreshProjectStatus();
  })
    .then((cleanup) => {
      unlistenCheckpointExport = cleanup;
    })
    .catch(() => {});
  let unlistenMaps: (() => void) | undefined;
  void listen<{ mapEntityId: string; linkId?: string }>("maps-navigation", async (event) => {
    try {
      let map = entities.find((entity) => entity.id === event.payload.mapEntityId && !entity.deleted) ?? null;
      if (!map) {
        const loaded = await project.getEntity(event.payload.mapEntityId);
        if (loaded && !loaded.deleted) {
          upsertEntityInCache(loaded);
          map = loaded;
        }
      }
      const item = mapsNavigationItem();
      if (!map || !item) throw new Error("map-unavailable: enable the Maps module to open this location");
      if (!(await flushAutoSave())) return;
      const departure = currentShellLocation();
      if (!(await leavePluginView())) return;
      selected = map;
      mapsEditorKey = map.id;
      await loadSelectedState(map);
      await openPluginView(item, departure);
      const linkId = event.payload.linkId ?? null;
      // openPluginView clears focus; also re-assert when already on the same map so
      // the vector editor retries focus after pins load.
      mapFocusLinkId = null;
      mapFocusFeatureId = null;
      await tick();
      mapFocusLinkId = linkId;
    } catch (cause) {
      error = friendlyError(cause);
    }
  })
    .then((cleanup) => {
      unlistenMaps = cleanup;
    })
    .catch(() => {});
  let unlistenMapsState: (() => void) | undefined;
  void listen<{ mapEntityId: string; status: string; detail: unknown }>("maps-state", (event) => {
    const { mapEntityId, status, detail } = event.payload;
    if (status === "fullscreen") {
      const enabled = typeof detail === "object" && detail !== null && "enabled" in detail && detail.enabled === true;
      editorFullscreen = enabled;
      return;
    }
    if (status === "back") {
      editorFullscreen = false;
      void closePluginView();
      return;
    }
    if (mapEntityId) mapSaveStates[mapEntityId] = { status, detail };
    if (status === "saved" && mapEntityId) {
      void refreshAfterEntityMutation({ entityId: mapEntityId }).then(() => {
        const map = entities.find((entity) => entity.id === mapEntityId && !entity.deleted);
        if (map) {
          selected = map;
          void loadSelectedState(map).catch(() => {});
        }
      });
    }
    if (status === "reconcile") {
      const reconcileDetail = detail as { unresolved?: unknown } | null;
      const unresolved = Array.isArray(reconcileDetail?.unresolved) ? reconcileDetail.unresolved.length : 0;
      mapReconcileNotice =
        unresolved > 0
          ? `${unresolved} link${unresolved === 1 ? "" : "s"} on this map are unresolved — the features they pointed to were removed or renumbered.`
          : "";
      if (selected && mapLocations.length > 0)
        void project
          .listMapLocations(selected.id)
          .then((locations) => {
            mapLocations = locations;
          })
          .catch(() => {});
    }
    if (status === "pick-complete") {
      const pickDetail = detail as { anchor?: unknown } | null;
      void applyMapPick(pickDetail?.anchor ?? null);
    }
    if (status === "pick-cancelled") {
      mapPickPending = null;
      mapPickNotice = "";
    }
    if (status === "open-entity") {
      const openDetail = detail as { entityId?: string } | null;
      if (openDetail?.entityId) void openMapEntityFromLink(openDetail.entityId);
    }
    if (status === "linked") {
      void refreshAfterEntityMutation(mapEntityId ? { entityId: mapEntityId } : undefined);
      if (selected)
        void project
          .listMapLocations(selected.id)
          .then((locations) => {
            mapLocations = locations;
          })
          .catch(() => {});
    }
  })
    .then((cleanup) => {
      unlistenMapsState = cleanup;
    })
    .catch(() => {});
  let unlistenMapsSelection: (() => void) | undefined;
  void listen<{ mapEntityId: string; anchor: unknown }>("maps-selection", (event) => {
    if (event.payload.mapEntityId === currentMapId()) mapSelection = event.payload.anchor;
  })
    .then((cleanup) => {
      unlistenMapsSelection = cleanup;
    })
    .catch(() => {});
  return () => {
    themeMedia.removeEventListener("change", handleSystemThemeChange);
    closeListenerDisposed = true;
    unlistenWindowClose?.();
    window.removeEventListener("beforeunload", handleBeforeUnload);
    window.removeEventListener("keydown", handleHistoryKey);
    window.removeEventListener("keydown", handleQuickOpenKey);
    window.removeEventListener("resize", handleWorkbenchResize);
    cancelAutoSave();
    if (aiModelsMessageTimer !== null) window.clearTimeout(aiModelsMessageTimer);
    unlisten?.();
    unlistenCheckpointExport?.();
    if (aiRequestId) void project.aiCancelText(aiRequestId).catch(() => {});
    if (aiFieldFillRequestId) void project.aiCancelText(aiFieldFillRequestId).catch(() => {});
    clearAiStreamListener();
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
        <small
          >{projectTransitionMessage === "Closing project…"
            ? "Returning to the project launcher."
            : "Your project will be ready in a moment."}</small>
      </div>
    </div>
  {/if}
  <AppSidebar
    {ready}
    collapsed={railCollapsed}
    projectMenuOpen={showProjectMenu}
    projectName={projectInfo?.name ?? "Local project"}
    {recentProjects}
    workspaces={workspaceNavigationItems().map((item) => ({
      key: item.key,
      section: item.section,
      title: item.title,
      beta: item.beta,
      active: navigationActive(item),
    }))}
    tools={pluginViews().map((item) => ({
      key: item.key,
      title: pluginViewLabel(item),
      ariaLabel: `Open ${item.plugin.name}: ${item.view.title}`,
      active: navigationActive(item),
    }))}
    homeActive={projectHomeOpen && !showSettings}
    createOpen={showCreateForm}
    projectCenterActive={showSettings && settingsSurface === "project"}
    settingsActive={showSettings && settingsSurface === "application"}
    version={displayVersion}
    onOpenProject={openProjectDirectory}
    onOpenRecent={(root) => void openRecentProject(root)}
    onRemoveRecent={removeRecentProject}
    onProjectMenuChange={(open) => (showProjectMenu = open)}
    onOpenProjectCenter={() => void openProjectCenter()}
    onCloseProject={closeProject}
    onOpenHome={() => void openProjectHome()}
    onCreate={toggleCreateForm}
    onOpenWorkspace={openSidebarNavigationItem}
    onOpenTool={openSidebarNavigationItem}
    onOpenSettings={() => void openSettings()}
    onCollapsedChange={updateRailCollapsed} />

  <section class:sandbox-active={Boolean(sandboxView)} class:map-surface-open={mapSurfaceOpen} class="app-main">
    {#snippet projectStatusControl()}
      <StatusCenter
        bind:open={statusCenterOpen}
        summary={statusCenterSummary()}
        tone={statusCenterTone()}
        items={statusCenterItems()}
        onOpenChange={(open) => open && void refreshProjectStatus()}
        onRefresh={refreshProjectStatus} />
    {/snippet}
    <GlobalToolbar
      {ready}
      breadcrumbs={breadcrumbItems()}
      quickOpenShortcut={quickOpenShortcutLabel()}
      navigationBusy={shellNavigationBusy}
      canGoBack={shellNavigationHistory.back.length > 0}
      canGoForward={shellNavigationHistory.forward.length > 0}
      onQuickOpen={openQuickOpen}
      onBack={() => void navigateShellHistory("back")}
      onForward={() => void navigateShellHistory("forward")}
      status={projectStatusControl} />
    {#if quickOpenOpen}<QuickOpen
        query={globalQuery}
        items={quickOpenItems()}
        loading={quickOpenSearchLoading}
        onQueryChange={(query) => (globalQuery = query)}
        onSelect={(item) => void selectQuickOpenItem(item)}
        onClose={closeQuickOpen} />{/if}
    {#if showCreateForm}{@const createOption = selectedCreateOption()}
      <div class="modal-backdrop">
        <div
          bind:this={createDialogElement}
          class="dialog create-dialog"
          role="dialog"
          aria-modal="true"
          aria-labelledby="create-dialog-title"
          tabindex="-1"
          onkeydown={handleCreateDialogKeydown}>
          <form class="create-dialog-form" onsubmit={createEntity}>
            <div class="create-dialog-heading">
              <div>
                {#if createDialogView === "templates"}<span class="panel-kicker">CREATE</span><strong
                    id="create-dialog-title">Choose a template</strong>
                  <p>Start with the kind of entry you want to add to this world.</p>{:else if createOption}<button
                    type="button"
                    class="create-dialog-back"
                    disabled={createBusy}
                    onclick={returnToCreationMenu}>All templates</button
                  ><span class="panel-kicker">{createOption.module.name.toUpperCase()}</span><strong
                    id="create-dialog-title">Create {createOption.template.name}</strong>
                  <p>{createOption.template.description ?? `Create a new ${createOption.template.entityType}.`}</p>{/if}
              </div>
              <button
                type="button"
                class="new-form-close"
                aria-label="Close create dialog"
                disabled={createBusy}
                onclick={closeCreateForm}><X size={16} strokeWidth={1.8} aria-hidden="true" /></button>
            </div>
            {#if createDialogView === "templates"}<div class="create-template-gallery">
                {#each createGroups() as group}<section class="create-template-group">
                    <div class="create-template-group-heading">
                      <span>{group.module.name}</span><small
                        >{group.options.length} {group.options.length === 1 ? "template" : "templates"}</small>
                    </div>
                    <div class="create-template-tiles">
                      {#each group.options as option}{@const optionIcon = iconForCreateOption(option)}<button
                          type="button"
                          data-template-index={option.key}
                          class="create-template-card"
                          disabled={createBusy}
                          onclick={() => openFocusedCreate(option.key)}
                          ><EntityGlyph
                            icon={optionIcon.icon}
                            iconColor={optionIcon.iconColor}
                            pluginId={optionIcon.pluginId}
                            size={19}
                            box={40} /><span class="create-template-copy"><strong>{option.template.name}</strong></span
                          ><span class="create-template-arrow">›</span><span class="create-template-detail"
                            ><small>{option.template.description ?? option.template.entityType}</small></span
                          ></button
                        >{/each}
                    </div>
                  </section>{/each}
              </div>{:else}<section class="create-form-panel">
                {#if createOption}<label class="create-input-field" for="new-entity"
                    ><span>Name <b>*</b></span><input
                      id="new-entity"
                      required
                      bind:value={name}
                      placeholder={`e.g. ${createOption.template.name}`}
                      autocomplete="off" /></label
                  >{#snippet createFieldControl(item: CreateField)}
                    <SchemaFieldInput
                      field={item.field}
                      required={item.required}
                      idPrefix="create"
                      class="create-input-field"
                      value={createFieldValues[item.field.key]}
                      search={searchEntitiesPaged(item.field)}
                      resolveSelected={resolveSelectedEntities}
                      calendars={worldCalendars() as any}
                      calendar={createCalendarDefinition(item.field.key)}
                      selectedCalendarId={calendarIdForStoredDate(
                        createDateForField(item.field.key),
                        createDateCalendarByField[item.field.key],
                      )}
                      onChange={(next) => {
                        if (item.field.type === "relationship") {
                          setCreateRelationshipValues(
                            item.field.key,
                            Array.isArray(next) ? next.filter((id): id is string => typeof id === "string") : [],
                          );
                          return;
                        }
                        setCreateField(item.field.key, next);
                      }}
                      onClearDate={() => clearCreateDateField(item.field.key)}
                      onSelectCalendar={(id) => setCreateDateCalendar(item.field.key, id)} />
                  {/snippet}{#if chronologyCreateFields(createOption).length > 0}{#each chronologyCreateFields(createOption) as item}{@render createFieldControl(
                        item,
                      )}{/each}{#each createChronologyWarnings() as warning}<p class="chronology-warning" role="status">
                        {warning}
                      </p>{/each}{/if}{#each requiredCreateFields(createOption) as item}{@render createFieldControl(
                      item,
                    )}{/each}{@const optionalFields =
                    optionalCreateFields(
                      createOption,
                    )}{#if optionalFields.length > 0 || createOption.template.document !== undefined}<button
                      class="create-more-details-toggle"
                      type="button"
                      aria-expanded={createMoreDetailsOpen}
                      aria-controls="create-more-details"
                      onclick={() => (createMoreDetailsOpen = !createMoreDetailsOpen)}
                      ><span
                        ><strong>More details</strong><small
                          >{optionalFields.length} optional {optionalFields.length === 1
                            ? "field"
                            : "fields"}{createOption.template.document !== undefined
                            ? `${optionalFields.length > 0 ? " and an " : "An "}opening note`
                            : ""}</small
                        ></span
                      ><ChevronDown
                        size={16}
                        strokeWidth={1.8}
                        class={createMoreDetailsOpen ? "expanded" : ""}
                        aria-hidden="true" /></button
                    >{#if createMoreDetailsOpen}<div id="create-more-details" class="create-more-details">
                        {#each optionalFields as item}{@render createFieldControl(
                            item,
                          )}{/each}{#if createOption.template.document !== undefined}<label
                            class="create-input-field"
                            for="create-document"
                            ><span>Opening note</span><textarea
                              id="create-document"
                              rows="5"
                              bind:value={createDocumentBody}
                              placeholder="Add a first note (optional)"></textarea
                            ></label
                          >{/if}
                      </div>{/if}{/if}{:else}<div class="create-form-empty">Choose a template to begin.</div>{/if}
              </section>{/if}
            {#if createDialogView === "form"}<div class="create-dialog-actions">
                <button type="button" class="quiet-button" disabled={createBusy} onclick={closeCreateForm}
                  >Cancel</button
                ><button class="primary-button" type="submit" disabled={createBusy || !name.trim() || !createOption}
                  >{createBusy ? "Creating…" : `Create ${createOption?.template.name ?? "entry"}`}</button>
              </div>{/if}
          </form>
        </div>
      </div>{/if}
    {#if showDiscardPrompt}<div class="discard-backdrop">
        <div class="discard-dialog" role="alertdialog" aria-modal="true" aria-labelledby="discard-create-title">
          <span class="panel-kicker">UNSAVED VALUES</span>
          <h2 id="discard-create-title">Discard this creation?</h2>
          <p>Your entered values will be cleared. Cancel to return without losing them.</p>
          <div class="discard-actions">
            <button type="button" class="quiet-button" onclick={keepCreateEditing}>Cancel</button><button
              type="button"
              class="primary-button"
              onclick={discardCreateValues}>Discard</button>
          </div>
        </div>
      </div>{/if}
    {#if upgradePreview}
      {@const preview = upgradePreview}
      <div class="modal-backdrop">
        <div class="dialog upgrade-dialog" role="dialog" aria-modal="true">
          <div class="new-form-heading">
            <div>
              <span class="panel-kicker">UPDATE PLUGIN</span><strong
                >Update {preview.entry.name} to v{preview.version}</strong>
            </div>
            <button type="button" class="new-form-close" onclick={() => (upgradePreview = null)}
              ><X size={16} strokeWidth={1.8} aria-hidden="true" /></button>
          </div>
          <p class="dialog-body-copy">
            From <code>v{preview.plan.fromVersion ?? preview.entry.version}</code> to
            <code>v{preview.plan.toVersion}</code>{preview.plan.target.signed ? ", signed by" : ", unsigned — "}{preview
              .plan.target.publisher}.
          </p>
          {#if preview.plan.consent.requiresRenewal}
            <p class="plugin-warning">
              This update requests new capabilities. Your consent is required before they are granted.
            </p>
          {/if}
          {#if preview.plan.consent.added.length > 0}
            <h4 class="plugin-subhead">New capabilities</h4>
            <ul class="plugin-detail-list">
              {#each preview.plan.consent.added as capability}<li class="provides">
                  {capabilityLabel(capability)}
                </li>{/each}
            </ul>
          {/if}
          {#if preview.plan.consent.removed.length > 0}
            <h4 class="plugin-subhead">Removed capabilities</h4>
            <ul class="plugin-detail-list">
              {#each preview.plan.consent.removed as capability}<li class="consumes">
                  {capabilityLabel(capability)}
                </li>{/each}
            </ul>
          {/if}
          <h4 class="plugin-subhead">Data migrations</h4>
          {#if preview.plan.migrations.migrationIds.length > 0}
            <p class="dialog-body-copy">
              Data will migrate from <code>v{preview.plan.migrations.from}</code> to
              <code>v{preview.plan.migrations.to}</code>{preview.plan.migrations.requiresBackup
                ? ". A backup is created before migrating."
                : "."}
            </p>
            <ul class="plugin-detail-list">
              {#each preview.plan.migrations.migrationIds as id}<li>{id}</li>{/each}
            </ul>
          {:else}
            <p class="dialog-body-copy">No data migrations required.</p>
          {/if}
          <div class="new-form-actions">
            <button type="button" class="quiet-button" onclick={() => (upgradePreview = null)}>Cancel</button><button
              type="button"
              class="primary-button"
              onclick={confirmUpgrade}
              disabled={upgradeBusy}>{upgradeBusy ? "Updating…" : "Confirm update"}</button>
          </div>
        </div>
      </div>
    {/if}
    {#if confirmAction}
      <div class="modal-backdrop plugin-confirm-modal">
        <div class="dialog" role="alertdialog" aria-modal="true">
          <div class="new-form-heading">
            <div><span class="panel-kicker">CONFIRM ACTION</span><strong>{confirmAction.title}</strong></div>
            <button type="button" class="new-form-close" onclick={() => (confirmAction = null)}
              ><X size={16} strokeWidth={1.8} aria-hidden="true" /></button>
          </div>
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
          <div class="new-form-actions">
            <button type="button" class="quiet-button" onclick={() => (confirmAction = null)}>Cancel</button><button
              type="button"
              class="primary-button"
              onclick={runConfirm}
              disabled={confirmBusy}>{confirmBusy ? "Working…" : confirmAction.confirmLabel}</button>
          </div>
        </div>
      </div>
    {/if}
    {#if deleteTarget}
      <div class="modal-backdrop">
        <div class="dialog" role="alertdialog" aria-modal="true">
          <div class="new-form-heading">
            <div>
              <span class="panel-kicker">DELETE PROJECT DATA</span><strong>Delete {deleteTarget.name} data?</strong>
            </div>
            <button type="button" class="new-form-close" onclick={() => (deleteTarget = null)}
              ><X size={16} strokeWidth={1.8} aria-hidden="true" /></button>
          </div>
          <p class="dialog-body-copy">
            All entities, documents, fields, relationships, and assets owned by <code>{deleteTarget.id}</code> in this project
            will be deleted. A backup is kept on disk.
          </p>
          <p class="dialog-body-copy">Type <code>{deleteTarget.id}</code> to confirm.</p>
          <input aria-label="Delete confirmation" bind:value={deleteInput} placeholder={deleteTarget.id} />
          <div class="new-form-actions">
            <button type="button" class="quiet-button" onclick={() => (deleteTarget = null)}>Cancel</button><button
              type="button"
              class="primary-button danger-button"
              onclick={confirmDeleteData}
              disabled={deleteBusy || deleteInput.trim() !== deleteTarget.id}
              >{deleteBusy ? "Deleting…" : "Delete project data"}</button>
          </div>
        </div>
      </div>
    {/if}
    {#if deleteBackupPath}
      <div class="modal-backdrop">
        <div class="dialog" role="alertdialog" aria-modal="true">
          <div class="new-form-heading">
            <div><span class="panel-kicker">DATA DELETED</span><strong>Extension data deleted</strong></div>
            <button type="button" class="new-form-close" onclick={() => (deleteBackupPath = "")}
              ><X size={16} strokeWidth={1.8} aria-hidden="true" /></button>
          </div>
          <p class="dialog-body-copy">A backup was kept at:</p>
          <code class="backup-path">{deleteBackupPath}</code>
          <div class="new-form-actions">
            <button type="button" class="primary-button" onclick={() => (deleteBackupPath = "")}>Done</button>
          </div>
        </div>
      </div>
    {/if}
    {#if ready && !showSettings && !projectHomeOpen && !hostView && !sandboxView && (section === "lore" || section === "timeline" || section === "writing" || section === "houses")}
      <WorkspaceViewNav
        label={`${section} views`}
        views={workspaceViewNavItems()}
        activeView={currentWorkspaceLocationView()}
        onSelect={(view) => void openWorkspaceView(view)} />
    {/if}
    {#if showSettings && settingsSurface === "application"}
      <SettingsView
        bind:section={settingsSection}
        {recentProjects}
        {themePreference}
        onThemeChange={updateThemePreference}
        onRemoveRecent={removeRecentProject}
        onClose={closeSettings}
        onBeforeNavigate={beforeVisibleSettingsNavigate}
        {aiSettings}
        {aiStatus}
        {aiModels}
        {aiModelsBusy}
        {aiModelsMessage}
        onAiSettingsChange={updateAiSetting}
        onAiImageSettingsChange={updateAiImageSetting}
        onAiCheck={() => void checkAiProvider()}
        onAiModelsLoad={() => void loadAiModels()}
        {remoteCredential}
        onAiRemoteImport={() => void importRemoteCredential()}
        onAiRemoteSave={(apiKey) => saveRemoteCredential(apiKey)}
        onAiRemoteClear={() => void clearRemoteCredential()} />
    {:else if showSettings && projectInfo}
      <ProjectCenter
        bind:section={projectSection}
        summary={{
          name: projectInfo.name,
          root: projectInfo.root,
          indexStatus: projectInfo.index_status,
          sync: projectInfo.sync,
          aiEnabled: projectInfo.aiEnabled,
        }}
        diagnostics={projectDiagnostics}
        snapshotChangeCount={gitStatus?.canonical_changes.length ?? 0}
        snapshotRepository={gitStatus?.repository ?? false}
        snapshotBranch={gitStatus?.branch ?? null}
        {archivedCount}
        {aiIndexStatus}
        {aiIndexBusy}
        {aiIndexMessage}
        remoteProvider={aiSettings.provider.endpoint.trim().toLowerCase().startsWith("https://")}
        onClose={closeSettings}
        onBeforeNavigate={beforeVisibleProjectNavigate}
        onImportExternal={openExternalImport}
        onExportMarkdown={exportMarkdownProject}
        onPortableBackup={createPortableBackup}
        onRecoveryBackup={createRecoveryBackup}
        onRestoreRecoveryBackup={restoreRecoveryBackup}
        onImportCheckpoint={importPortableCheckpoint}
        onRebuildIndex={rebuildSearchIndex}
        onSeedExample={seedExample}
        onToggleAi={(enabled) => void setProjectAiEnabled(enabled)}
        onAiRemoteConsent={(allowed) => void setRemoteConsent(allowed)}
        onAiIndexRefresh={() => void refreshAiIndexStatus()}
        onAiIndexRebuild={() => void rebuildAiIndex()}
        onAiIndexCancel={() => void cancelAiIndex()}
        typeLabel={entityTypeLabel}
        onArchiveChanged={() => void handleArchiveChanged()}
        onArchiveToast={showToast}>
        {#snippet extensions()}
          <div class="panel-hero">
            <div class="hero-icon">
              <Puzzle size={18} strokeWidth={1.8} aria-hidden="true" />
            </div>
            <div class="hero-copy">
              <span class="kicker">EXTENSIONS</span>
              <strong>Extensions</strong>
              <p>
                Add and manage the tools that power this project. Installs, updates, and rollbacks are verified and
                reversible.
              </p>
            </div>
            <div class="hero-stats">
              <span class="stat-pill"
                ><Puzzle size={16} strokeWidth={1.8} aria-hidden="true" />
                {adminPlugins ? adminPlugins.length : 0} installed</span>
              <span class="stat-pill"
                ><ShieldCheck size={16} strokeWidth={1.8} aria-hidden="true" />
                {adminPlugins ? adminPlugins.filter((p) => p.enabled).length : 0} enabled</span>
            </div>
          </div>
          <div class="plugins-toolbar">
            <button
              type="button"
              class="primary-button"
              disabled={installing || adminBusy}
              onclick={() => void installFromPicker()}>{installing ? "Installing…" : "Install extension…"}</button>
            <span class="muted-note">{adminBusy ? "Refreshing…" : ""}</span>
          </div>
          {#if installSummary}
            <div class="plugins-note">
              Installed {installSummary.id}
              {installSummary.version}{installSummary.signed ? " (signed)" : " (unsigned)"}{installSummary.digest
                ? ` · ${shortDigest(installSummary.digest)}`
                : ""}.
            </div>
          {/if}
          <div class="plugins-list">
            {#if adminPlugins === null}
              <p class="search-state">Loading extensions…</p>
            {:else if adminPlugins.length === 0}
              <p class="search-state">No extensions installed. Choose an extension package to get started.</p>
            {:else}
              {#each adminPlugins as plugin (plugin.id)}
                <article class="plugin-card">
                  <header class="plugin-card-head">
                    <div class="plugin-card-title">
                      <strong>{plugin.name}</strong><span class="plugin-id">{plugin.id}</span>
                    </div>
                    <div class="plugin-badges">
                      <span class:badge-off={!plugin.enabled} class="plugin-badge"
                        >{plugin.enabled ? "Enabled" : "Disabled"}</span>
                      <span
                        class="plugin-badge"
                        title={plugin.distribution.management === "app"
                          ? "This extension is included with Daena and managed by the application."
                          : "This extension was installed separately from the application."}
                        >{plugin.distribution.origin === "bundled" ? "Included with Daena" : "Installed"}</span>
                      {#if plugin.stability === "beta"}<span
                          class="plugin-badge beta"
                          title="Beta release: this plugin is useful but may be unstable">Beta · unstable</span
                        >{:else if plugin.stability === "experimental"}<span
                          class="plugin-badge experimental"
                          title="Experimental release: behavior may change">Experimental</span
                        >{/if}
                      {#if plugin.lifecycle.failures > 0}<span
                          class="plugin-badge danger"
                          title={plugin.lifecycle.lastError ?? ""}
                          >{plugin.lifecycle.failures} failure{plugin.lifecycle.failures === 1 ? "" : "s"}</span
                        >{/if}
                    </div>
                  </header>
                  <div class="plugin-card-meta">
                    <span>v{plugin.selectedVersion ?? plugin.version} · {plugin.publisher}</span>
                  </div>
                  {#if !plugin.dependencyState.resolved}
                    <p class="plugin-warning">
                      Dependency problem: {plugin.dependencyState.error ?? "could not resolve dependencies"}
                    </p>
                  {:else if plugin.dependencyState.order.length > 0}
                    <p class="plugin-muted">Loads after: {plugin.dependencyState.order.join(" → ")}</p>
                  {/if}
                  {#if plugin.lifecycle.lastError}
                    <p class="plugin-error" title={plugin.lifecycle.lastError}>
                      Last failure: {plugin.lifecycle.lastError}
                    </p>
                  {/if}
                  <div class="plugin-actions">
                    <button
                      type="button"
                      class:on={plugin.enabled}
                      class="plugin-toggle"
                      onclick={() => void togglePluginEnabled(plugin)}
                      disabled={adminBusy || pluginActionId !== null}
                      >{pluginActionId === plugin.id ? "Working…" : plugin.enabled ? "Disable" : "Enable"}</button>
                    {#if plugin.enabled}<button
                        type="button"
                        class="quiet-button"
                        onclick={() => void reviewPluginCapabilities(plugin)}
                        disabled={adminBusy || pluginActionId !== null}>Review capabilities</button
                      >{/if}
                    {#if selectedUninstallableVersion(plugin)}
                      <button
                        class="quiet-button"
                        onclick={() => confirmUninstall(plugin, selectedUninstallableVersion(plugin)!.version)}
                        disabled={adminBusy || plugin.enabled}
                        title={plugin.enabled
                          ? "Disable the extension before uninstalling its selected code."
                          : "Remove the selected extension code while preserving project data."}
                        >{plugin.enabled ? "Disable to uninstall" : "Uninstall code"}</button>
                    {/if}
                    {#if plugin.lifecycle.state === "quarantined"}<button
                        class="quiet-button"
                        onclick={() => retryPlugin(plugin)}
                        disabled={adminBusy}>Retry</button
                      >{/if}
                    <button class="quiet-button" onclick={() => openDeleteData(plugin)} disabled={adminBusy}
                      >Delete project data…</button>
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
                              {#if version.signed}<span class="version-tag signed">Signed</span
                                >{:else if !version.bundled}<span class="version-tag unsigned">Unsigned</span>{/if}
                            </span>
                            <span class="version-detail"
                              >{version.publisher}{version.installedAt
                                ? ` · installed ${installedAtLabel(version.installedAt)}`
                                : ""}{version.digest ? ` · ${shortDigest(version.digest)}` : ""}</span>
                          </div>
                          <div class="version-actions">
                            {#if version.isActiveCandidate && !version.isSelected}
                              <button
                                class="quiet-button"
                                onclick={() => previewUpgrade(plugin, version.version)}
                                disabled={adminBusy}>Update to v{version.version}</button>
                            {/if}
                            {#if version.rollbackAvailable}
                              <button
                                class="quiet-button"
                                onclick={() => confirmRollback(plugin, version.version)}
                                disabled={adminBusy}>Rollback</button>
                            {/if}
                            {#if !version.isSelected && plugin.distribution.canUninstall}
                              <button
                                class="quiet-button"
                                onclick={() => confirmUninstall(plugin, version.version)}
                                disabled={adminBusy}>Uninstall code</button>
                            {/if}
                          </div>
                        </div>
                      {/each}
                    </div>
                  {/if}
                  <details class="plugin-details">
                    <summary>Capabilities, namespaces, services &amp; migrations</summary>
                    <p class="plugin-muted">
                      {plugin.kind} · Host API {plugin.hostApi} · Data format {plugin.dataVersion}
                    </p>
                    <div class="plugin-details-grid">
                      <section class="plugin-detail-section">
                        <h4>Capabilities</h4>
                        <ul class="plugin-detail-list">
                          {#each plugin.capabilities as capability}
                            {@const granted = plugin.grantedCapabilities.includes(capability)}
                            <li class:granted class="capability-row">
                              <span class="capability-name">{capabilityLabel(capability)}</span><span class="cap-mark"
                                >{granted ? "✓" : "—"}</span>
                            </li>
                          {/each}
                        </ul>
                      </section>
                      <section class="plugin-detail-section">
                        <h4>Namespaces</h4>
                        <ul class="plugin-detail-list">
                          {#each plugin.namespaces as namespace}<li>{namespace}</li>{/each}
                        </ul>
                      </section>
                      <section class="plugin-detail-section">
                        <h4>Services</h4>
                        <ul class="plugin-detail-list">
                          {#each plugin.services.provides as service}<li class="provides">
                              provides <code>{service.name}@{service.major}</code>
                            </li>{/each}
                          {#each plugin.services.consumes as service}<li class="consumes">
                              consumes <code>{service.name}@{service.major}</code>
                            </li>{/each}
                          {#if plugin.services.provides.length === 0 && plugin.services.consumes.length === 0}<li
                              class="muted-item">
                              none
                            </li>{/if}
                        </ul>
                      </section>
                      <section class="plugin-detail-section">
                        <h4>Events</h4>
                        <ul class="plugin-detail-list">
                          {#each plugin.events.publishes as event}<li class="provides">
                              publishes <code>{event.name}@{event.version}</code>
                            </li>{/each}
                          {#each plugin.events.subscribes as event}<li class="consumes">
                              subscribes <code>{event.name}@{event.version}</code>
                            </li>{/each}
                          {#if plugin.events.publishes.length === 0 && plugin.events.subscribes.length === 0}<li
                              class="muted-item">
                              none
                            </li>{/if}
                        </ul>
                      </section>
                      <section class="plugin-detail-section">
                        <h4>Migrations</h4>
                        <ul class="plugin-detail-list">
                          {#each plugin.migrations as migration}<li class="migration">
                              <code>{migration.from} → {migration.to}</code>{migration.recovery === "backup"
                                ? " · backs up"
                                : ""}
                            </li>{/each}
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
        {#snippet fields()}
          <SchemaSettingsPanel
            projectOpen={ready}
            candidates={schemaOverlayCandidates()}
            managedPlugins={managedSchemaPlugins()}
            selectedPluginId={schemaPluginId}
            selectedPluginName={schemaPluginName}
            packageManifest={moduleSchemaPackage}
            overlay={moduleSchemaOverlay}
            referenceEntityTypes={schemaReferenceEntityTypes(schemaPluginId)}
            overlayRevision={moduleSchemaRevision}
            busy={moduleSchemaBusy}
            message={moduleSchemaMessage}
            entityCountForType={schemaEntityCountForType}
            onReassignEntities={reassignSchemaEntities}
            onSelectPlugin={selectSchemaPlugin}
            onSave={saveModuleSchemaOverlay}
            onDirtyChange={setSchemaEditorDirty} />
        {/snippet}
        {#snippet snapshots()}
          <GitSettingsPanel
            projectOpen={ready}
            projectId={projectInfo?.root ?? ""}
            aiEnabled={projectInfo?.aiEnabled ?? false}
            onError={(message) => (error = message)}
            onStatusChange={(status) => (gitStatus = status)}
            beforeWrite={flushAutoSave} />
        {/snippet}
      </ProjectCenter>
    {:else if !ready}
      <section class="welcome">
        <div class="welcome-copy">
          <span class="overline">A private place for impossible worlds</span>
          <h1>Build the world<br /><em>behind the story.</em></h1>
          <p>Shape characters, places, factions, and history in one calm, local-first studio.</p>
        </div>
        <div class="welcome-map" aria-hidden="true">
          <span class="map-marker marker-atlas"><MapIcon size={21} strokeWidth={1.65} /></span>
          <span class="map-marker marker-people"><UsersRound size={21} strokeWidth={1.65} /></span>
          <span class="map-marker marker-sword"><Sword size={21} strokeWidth={1.65} /></span>
          <span class="map-marker marker-tree"><TreePine size={21} strokeWidth={1.65} /></span>
        </div>
        <div class="art-card">
          <span>ELDERMERE</span><strong>The sea remembers<br />what kingdoms forget.</strong><small
            >Fragments · 12</small>
        </div>
      </section>
    {:else if projectHomeOpen}
      <ProjectHome
        projectName={projectInfo?.name ?? "Your world"}
        activeEntityCount={entities.filter((entity) => !entity.deleted).length}
        snapshotChangeCount={gitStatus?.canonical_changes.length ?? 0}
        workspaces={enabledWorkspaceSections().map((target) => ({
          section: target,
          title: workspaceSectionLabel(target),
          description: workspaceDescription(target),
          count: workspaceEntityCount(target),
        }))}
        recents={recentlyUpdatedEntities().map((entity) => {
          const presentation = iconForEntityType(entity.entity_type);
          return {
            entity,
            icon: presentation.icon,
            pluginId: presentation.pluginId,
            iconColor: presentation.iconColor,
            typeLabel: entityTypeLabel(entity.entity_type),
            updatedLabel: updatedDateLabel(entity.updated_at),
          };
        })}
        onNewEntry={toggleCreateForm}
        onSnapshots={() => void openProjectCenter("snapshots")}
        onProjectCenter={() => void openProjectCenter()}
        onExtensions={() => void openProjectCenter("extensions")}
        onOpenWorkspace={(target) => void switchSection(target)}
        onOpenEntity={(entity) => void selectSearchResult(entity)} />
    {:else if projectionView}
      <SpecializedSurface
        restoreKey={specializedSurfaceKey() ?? "projection"}
        restoreScrollTop={currentSpecializedSurfaceScrollTop()}
        bind:element={specializedSurfaceElement}
        onScroll={rememberSpecializedSurfaceScroll}>
        {#key projectionView.title}
          <ProjectionView
            title={projectionView.title}
            subtitle={projectionView.subtitle}
            kind={projectionView.kind}
            view={projectionView.module.views[0]}
            context={buildModuleContext(projectionView.manifest, projectInfo?.root ?? "", {
              focusEntityId: selected?.id as UUID | undefined,
              availableServices: enabledServices(),
            })}
            onClose={() => void closeProjectionView()} />
        {/key}
      </SpecializedSurface>
    {:else if hostView}
      <SpecializedSurface
        restoreKey={specializedSurfaceKey() ?? "host"}
        restoreScrollTop={currentSpecializedSurfaceScrollTop()}
        bind:element={specializedSurfaceElement}
        onScroll={rememberSpecializedSurfaceScroll}>
        <div class="host-view-shell">
          <button class="quiet-button host-view-back" onclick={() => void closePluginView()}>Back to workspace</button
          ><HostView plugin={hostView.plugin} view={hostView.view} />
        </div>
      </SpecializedSurface>
    {:else if loreWikiOpen}
      <SpecializedSurface
        restoreKey={specializedSurfaceKey() ?? "wiki"}
        restoreScrollTop={currentSpecializedSurfaceScrollTop()}
        bind:element={specializedSurfaceElement}
        onScroll={rememberSpecializedSurfaceScroll}>
        <WikiView
          manifest={manifestForWorkspaceSection("lore")!}
          enabledManifests={modules.filter((module) => module.enabled)}
          initialEntityId={loreWikiEntityId}
          projectId={projectInfo?.root ?? ""}
          aiEnabled={projectInfo?.aiEnabled ?? false}
          imageProvider={aiSettings.imageProvider}
          textProvider={aiSettings.provider}
          onClose={() => void closeLoreWiki()}
          onSelectEntity={(id) => {
            const ent = entities.find((e) => e.id === id);
            if (ent) void selectEntity(ent);
          }} />
      </SpecializedSurface>
    {:else if section === "houses" && housesView === "tree"}
      {#snippet familyTreeAvatar(entityId: string, name: string)}
        <EntityAvatar {entityId} {name} />
      {/snippet}
      <SpecializedSurface
        restoreKey={specializedSurfaceKey() ?? "workspace:houses:tree"}
        restoreScrollTop={currentSpecializedSurfaceScrollTop()}
        bind:element={specializedSurfaceElement}
        onScroll={rememberSpecializedSurfaceScroll}>
        <FamilyTreeSurface
          context={buildModuleContext(housesManifestJson as unknown as ModuleManifest, projectInfo?.root ?? "", {
            availableServices: enabledServices(),
          })}
          projectId={projectInfo?.root ?? ""}
          initialRootId={familyTreeRootId}
          initialSession={familyTreeSession}
          restoreNonce={familyTreeRestoreNonce}
          avatar={familyTreeAvatar}
          onNewPerson={openNewPerson}
          onNewHouse={openNewHouse}
          onOpenHouseEntry={(houseId) => {
            void (async () => {
              const entity = await project.getEntity(houseId);
              if (!entity) return;
              upsertEntityInCache(entity);
              section = "houses";
              housesView = "houses";
              familyTreeRootId = null;
              familyTreeSession = null;
              await selectEntity(entity);
            })();
          }}
          onArchiveHouse={(houseId) => {
            void (async () => {
              const entity = entities.find((item) => item.id === houseId) ?? (await project.getEntity(houseId));
              if (!entity) return;
              await archiveEntity(entity, { skipConfirm: true });
              familyTreeRootId = null;
              familyTreeSession = null;
            })();
          }}
          onRenameHouse={async (houseId, name) => {
            const entity = entities.find((item) => item.id === houseId) ?? (await project.getEntity(houseId));
            if (!entity) return;
            const updated = await project.updateEntity(houseId, name, null, {
              expectedRevision: entity.revision,
            });
            upsertEntityInCache(updated);
            await refreshAfterEntityMutation({ entityId: updated.id });
          }}
          onRootChange={(id) => {
            if (id === familyTreeRootId) return;
            recordShellDeparture(currentShellLocation());
            familyTreeRootId = id;
            familyTreeSession = null;
          }}
          onSessionChange={(session) => {
            if (sameFamilyTreeSession(familyTreeSession, session)) return;
            const historyChanged = familyTreeHistoryKey(familyTreeSession) !== familyTreeHistoryKey(session);
            if (historyChanged && familyTreeRootId && familyTreeSession) {
              recordShellDeparture(currentShellLocation());
            }
            familyTreeSession = session;
          }}
          onOpenEntity={(entityId) => void openFamilyTreePerson(entityId)}
          onMembershipChanged={() => bumpCollectionRefresh()}
          onEditPersonIdentity={(personId) => {
            void (async () => {
              const entity = entities.find((item) => item.id === personId) ?? (await project.getEntity(personId));
              if (!entity) return;
              upsertEntityInCache(entity);
              await openEntityEditDialog(entity);
            })();
          }}
          onArchivePerson={(personId) => {
            void (async () => {
              const entity = entities.find((item) => item.id === personId) ?? (await project.getEntity(personId));
              if (!entity) return;
              await archiveEntity(entity, { skipConfirm: true });
            })();
          }}
          onBack={() => {
            const current = currentShellLocation();
            const transition = shellHistoryBack(shellNavigationHistory, current);
            if (transition && shellLocationAvailable(transition.target)) {
              void navigateShellHistory("back");
              return;
            }
            familyTreeRootId = null;
            familyTreeSession = null;
            familyTreeRestoreNonce += 1;
          }} />
      </SpecializedSurface>
    {:else if sandboxView && sandboxView.renderer !== "maps"}
      <SpecializedSurface
        restoreKey={specializedSurfaceKey() ?? "sandbox"}
        restoreScrollTop={currentSpecializedSurfaceScrollTop()}
        bind:element={specializedSurfaceElement}
        onScroll={rememberSpecializedSurfaceScroll}>
        {#key `${sandboxView.plugin.id}:${sandboxView.view?.id ?? "default"}`}
          <SandboxView pluginId={sandboxView.plugin.id} viewId={sandboxView.view?.id} title={sandboxView.plugin.name} />
        {/key}
      </SpecializedSurface>
    {:else if enabledWorkspaceSections().length === 0}
      <section class="empty-workspace-state">
        <div class="disabled-icon">◌</div>
        <span class="overline">WORKSPACE READY</span>
        <h1>Choose a workspace to begin.</h1>
        <p>No workspace modules are enabled in this project. Enable one from Project → Extensions to start working.</p>
        <button class="primary-button" onclick={() => void openProjectCenter("extensions")}>Open Extensions</button>
      </section>
    {:else}
      {#if projectDiagnostics.length}<div class="project-diagnostics" role="alert">
          <span>{projectDiagnostics[0]}</span><button
            class="quiet-button"
            onclick={() => void importPortableCheckpoint()}>Import checkpoint</button>
        </div>{/if}
      {#if !mapSurfaceOpen}
        {#snippet workspaceHeaderActions()}
          {#if section === "maps"}<div class="map-provider-create">
              <button
                class="primary-button"
                type="button"
                aria-haspopup="menu"
                aria-expanded={mapProviderMenuOpen === "header"}
                onclick={() => (mapProviderMenuOpen = mapProviderMenuOpen === "header" ? null : "header")}
                >Create map</button>
              {#if mapProviderMenuOpen === "header"}<div class="map-provider-menu" role="menu">
                  <div class="map-provider-row">
                    <button type="button" role="menuitem" onclick={() => void createMap("physical")}
                      >Generate physical world</button
                    ><span class="map-help-wrapper"
                      ><button type="button" class="map-help" aria-label="Help for Generate physical world">?</button
                      ><span class="map-help-tooltip" role="tooltip"
                        >Create a whole world from scratch — continents, oceans, climate and hazards are generated. The
                        base world can't be edited directly; copy any part you want to change into an editable layer.</span
                      ></span>
                  </div>
                  <div class="map-provider-row">
                    <button type="button" role="menuitem" onclick={() => void createMap("vector")}
                      >Import vector map</button
                    ><span class="map-help-wrapper"
                      ><button type="button" class="map-help" aria-label="Help for Import vector map">?</button><span
                        class="map-help-tooltip"
                        role="tooltip"
                        >Import a GeoJSON file. Draw places, borders and routes as shapes you can edit.</span
                      ></span>
                  </div>
                  <div class="map-provider-row">
                    <button type="button" role="menuitem" onclick={() => void createMap("image")}
                      >Import image map</button
                    ><span class="map-help-wrapper"
                      ><button type="button" class="map-help" aria-label="Help for Import image map">?</button><span
                        class="map-help-tooltip"
                        role="tooltip"
                        >Use any picture (PNG, JPG, SVG) as a background and draw your map on top of it.</span
                      ></span>
                  </div>
                </div>{/if}
            </div>
          {:else if section === "houses"}
            <div class="heading-create-group" role="group" aria-label="Create">
              <button class="primary-button" type="button" aria-label={ENTITY_ACTIONS.newHouse} onclick={openNewHouse}
                >{ENTITY_ACTIONS.newHouse}</button>
            </div>
          {:else}
            <button
              class="primary-button"
              type="button"
              aria-label={`${ENTITY_ACTIONS.new} ${createLabel()}`}
              onclick={openContextualCreate}
              ><span style="display:inline-flex;vertical-align:middle" aria-hidden="true"
                ><Plus size={14} strokeWidth={1.8} /></span>
              {ENTITY_ACTIONS.new}</button>
          {/if}
        {/snippet}
        <WorkspaceHeader
          kicker={workspaceHeadingKicker()}
          title={sectionLabel()}
          description={workspaceHeadingDescription()}
          actions={workspaceHeaderActions} />
        <nav class="workbench-layout-controls" aria-label="Workbench panes">
          <span>Layout</span><button
            type="button"
            aria-pressed={workbenchPaneVisibility.collection}
            onclick={() => toggleWorkbenchPane("collection")}>Collection</button
          ><button
            type="button"
            aria-pressed={workbenchPaneVisibility.content}
            onclick={() => toggleWorkbenchPane("content")}>Content</button
          >{#if workbenchSupportsInspector()}<button
              type="button"
              aria-pressed={workbenchPaneVisibility.inspector}
              onclick={() => toggleWorkbenchPane("inspector")}>Inspector</button
            >{/if}
        </nav>
      {/if}
      <section
        class:maps-workspace={section === "maps" && sandboxView?.renderer === "maps"}
        class:map-surface-expanded={mapSurfaceOpen}
        class:workspace-grid-no-inspector={section === "language"}
        class="workspace-grid"
        style={workspaceGridStyle()}>
        {#snippet collectionControls()}
          <div class="collection-search">
            <span><Search size={16} strokeWidth={1.8} aria-hidden="true" /></span><input
              aria-label={`Search ${collectionLabel()}`}
              bind:value={collectionQuery.textSearch}
              placeholder={`Search ${collectionLabel()}`} /><button
              class="filter-toggle"
              class:active={filterOpen}
              aria-label="Filters"
              onclick={() => (filterOpen = !filterOpen)}>⫶</button>
            {#if filterOpen}<div class="filter-popover" role="dialog" aria-label="Collection filters">
                <fieldset class="filter-section">
                  <legend>Sort by</legend>
                  <div class="filter-row">
                    <label
                      ><input
                        type="radio"
                        name="sortField"
                        value="name"
                        checked={collectionQuery.sortField === "name"}
                        onchange={() => (collectionQuery.sortField = "name")} /> Name</label
                    ><label
                      ><input
                        type="radio"
                        name="sortField"
                        value="created_at"
                        checked={collectionQuery.sortField === "created_at"}
                        onchange={() => (collectionQuery.sortField = "created_at")} /> Created</label
                    ><label
                      ><input
                        type="radio"
                        name="sortField"
                        value="updated_at"
                        checked={collectionQuery.sortField === "updated_at"}
                        onchange={() => (collectionQuery.sortField = "updated_at")} /> Updated</label
                    ><button
                      class="sort-dir-toggle"
                      aria-label={collectionQuery.sortDir === "asc" ? "Ascending" : "Descending"}
                      onclick={() => (collectionQuery.sortDir = collectionQuery.sortDir === "asc" ? "desc" : "asc")}
                      >{collectionQuery.sortDir === "asc" ? "↑" : "↓"}</button>
                  </div>
                </fieldset>
                <fieldset class="filter-section">
                  <legend>Per page</legend>
                  <div class="filter-row">
                    {#each [25, 50, 100] as size}<label
                        ><input
                          type="radio"
                          name="pageSize"
                          value={size}
                          checked={collectionQuery.pageSize === size}
                          onchange={() => {
                            collectionQuery.pageSize = size;
                            collectionQuery.page = 0;
                          }} />
                        {size}</label
                      >{/each}
                  </div>
                </fieldset>
                <fieldset class="filter-section">
                  <legend>Entity types</legend>
                  <div class="filter-type-list">
                    {#each sectionEntityTypes() as type}<label
                        ><input
                          type="checkbox"
                          checked={!collectionQuery.excludedTypes.includes(type)}
                          onchange={() => toggleTypeFilter(type)} />
                        {entityTypeLabel(type)}</label
                      >{/each}
                  </div>
                </fieldset>
              </div>{/if}
          </div>
        {/snippet}
        {#snippet collectionItems()}
          {#if collectionError}<p class="collection-error" role="alert">{collectionError}</p>{/if}
          {#if collectionLoading && collectionPage.items.length === 0}<div class="collection-loading" role="status">
              Loading {collectionLabel()}…
            </div>{:else if collectionResult().total === 0}
            {#if section === "maps"}
              <EntityEmptyState
                title={collectionQuery.textSearch
                  ? `No ${collectionLabel()} match that search.`
                  : `No ${collectionLabel()} yet.`}
                message={collectionQuery.textSearch
                  ? "Try another search or create something new."
                  : "Create a map through an installed map integration."}
                maps>
                {#snippet actions()}
                  <div class="empty-create-actions">
                    <button
                      class="empty-create"
                      type="button"
                      aria-haspopup="menu"
                      aria-expanded={mapProviderMenuOpen === "empty"}
                      onclick={() => (mapProviderMenuOpen = mapProviderMenuOpen === "empty" ? null : "empty")}
                      ><span style="display:inline-flex;vertical-align:middle" aria-hidden="true"
                        ><Plus size={16} strokeWidth={1.8} aria-hidden="true" /></span> Create map</button>
                    {#if mapProviderMenuOpen === "empty"}<div
                        class="map-provider-menu empty-map-provider-menu"
                        role="menu">
                        <div class="map-provider-row">
                          <button type="button" role="menuitem" onclick={() => void createMap("physical")}
                            >Generate physical world</button
                          ><span class="map-help-wrapper"
                            ><button type="button" class="map-help" aria-label="Help for Generate physical world"
                              >?</button
                            ><span class="map-help-tooltip" role="tooltip"
                              >Create a whole world from scratch — continents, oceans, climate and hazards are
                              generated. The base world can't be edited directly; copy any part you want to change into
                              an editable layer.</span
                            ></span>
                        </div>
                        <div class="map-provider-row">
                          <button type="button" role="menuitem" onclick={() => void createMap("vector")}
                            >Import vector map</button
                          ><span class="map-help-wrapper"
                            ><button type="button" class="map-help" aria-label="Help for Import vector map">?</button
                            ><span class="map-help-tooltip" role="tooltip"
                              >Import a GeoJSON file. Draw places, borders and routes as shapes you can edit.</span
                            ></span>
                        </div>
                        <div class="map-provider-row">
                          <button type="button" role="menuitem" onclick={() => void createMap("image")}
                            >Import image map</button
                          ><span class="map-help-wrapper"
                            ><button type="button" class="map-help" aria-label="Help for Import image map">?</button
                            ><span class="map-help-tooltip" role="tooltip"
                              >Use any picture (PNG, JPG, SVG) as a background and draw your map on top of it.</span
                            ></span>
                        </div>
                      </div>{/if}
                  </div>
                {/snippet}
              </EntityEmptyState>
            {:else}
              <EntityEmptyState
                title={collectionQuery.textSearch
                  ? `No ${collectionLabel()} match that search.`
                  : `No ${collectionLabel()} yet.`}
                message={collectionQuery.textSearch
                  ? "Try another search or create something new."
                  : `Create your first ${createLabel()} to begin building this collection.`}
                createLabel={createLabel()}
                onCreate={openContextualCreate} />
            {/if}{:else if collectionQuery.viewMode === "grouped"}{#each collectionResult().groups ?? [] as group}{@const groupIcon =
                iconForEntityType(group.type === "__uncategorized" ? null : group.type)}
              <div class="collection-group">
                <button type="button" class="collection-group-header" onclick={() => toggleGroup(group.type)}
                  ><span class="group-chevron"
                    >{#if expandedGroups.has(group.type)}<ChevronDown
                        size={16}
                        strokeWidth={1.8}
                        aria-hidden="true" />{:else}<ChevronRight
                        size={16}
                        strokeWidth={1.8}
                        aria-hidden="true" />{/if}</span
                  ><EntityGlyph
                    icon={groupIcon.icon}
                    iconColor={groupIcon.iconColor}
                    pluginId={groupIcon.pluginId}
                    size={16}
                    box={22} /><strong>{group.label}</strong><small>{group.count}</small></button
                >{#if expandedGroups.has(group.type)}{#each group.entities as entity}{@const rowIcon =
                      iconForEntityType(entity.entity_type)}
                    <div class:selected={selected?.id === entity.id} class="collection-item">
                      <button type="button" class="collection-item-main" onclick={() => void selectEntity(entity)}>
                        <EntityGlyph
                          icon={rowIcon.icon}
                          iconColor={rowIcon.iconColor}
                          pluginId={rowIcon.pluginId}
                          size={16}
                          box={40} /><span class="item-copy"
                          ><strong>{entity.name}</strong><small
                            >{section === "houses" &&
                            ((entity.entity_type ?? "") === HOUSE_TYPE || (entity.entity_type ?? "").endsWith(":house"))
                              ? formatHouseMemberSummary(houseCollectionSummaries.get(entity.id), {
                                  pending: houseSummariesPending && !houseCollectionSummaries.has(entity.id),
                                })
                              : entityTypeLabel(entity.entity_type)}</small
                          ></span>
                      </button>
                      <EntityRowActions
                        entityName={entity.name}
                        openTree={section === "houses" && entity.entity_type === HOUSE_TYPE}
                        onOpen={() => void selectEntity(entity)}
                        onEditIdentity={() => void openEntityEditDialog(entity)}
                        onArchive={() => void archiveEntity(entity)}
                        onOpenTree={() => void openHouseTree(entity)} />
                    </div>{/each}{/if}
              </div>{/each}{:else}{#each collectionResult().entities as entity}{@const rowIcon = iconForEntityType(
                entity.entity_type,
              )}
              <div class:selected={selected?.id === entity.id} class="collection-item">
                <button type="button" class="collection-item-main" onclick={() => void selectEntity(entity)}>
                  <EntityGlyph
                    icon={rowIcon.icon}
                    iconColor={rowIcon.iconColor}
                    pluginId={rowIcon.pluginId}
                    size={16}
                    box={40} /><span class="item-copy"
                    ><strong>{entity.name}</strong><small
                      >{section === "houses" &&
                      ((entity.entity_type ?? "") === HOUSE_TYPE || (entity.entity_type ?? "").endsWith(":house"))
                        ? formatHouseMemberSummary(houseCollectionSummaries.get(entity.id), {
                            pending: houseSummariesPending && !houseCollectionSummaries.has(entity.id),
                          })
                        : entityTypeLabel(entity.entity_type)}</small
                    ></span>
                </button>
                <EntityRowActions
                  entityName={entity.name}
                  openTree={section === "houses" && entity.entity_type === HOUSE_TYPE}
                  onOpen={() => void selectEntity(entity)}
                  onEditIdentity={() => void openEntityEditDialog(entity)}
                  onArchive={() => void archiveEntity(entity)}
                  onOpenTree={() => void openHouseTree(entity)} />
              </div>{/each}{/if}
        {/snippet}
        {#snippet collectionFooter()}
          {#if collectionResult().total > 0}<nav class="collection-pagination" aria-label="Collection pages">
              <button
                type="button"
                disabled={collectionQuery.page === 0 || collectionLoading}
                onclick={() => (collectionQuery.page = Math.max(0, collectionQuery.page - 1))}>Previous</button
              ><span
                >{collectionPage.offset + 1}–{Math.min(
                  collectionPage.offset + collectionPage.items.length,
                  collectionPage.total,
                )} of {collectionPage.total}</span
              ><button
                type="button"
                disabled={!collectionPage.has_more || collectionLoading}
                onclick={() => (collectionQuery.page += 1)}>Next</button>
            </nav>{/if}
        {/snippet}
        {#if workbenchPaneVisibility.collection}<CollectionPane
            kicker={collectionKicker()}
            count={collectionResult().total}
            label={collectionLabel()}
            viewMode={collectionQuery.viewMode}
            controls={collectionControls}
            children={collectionItems}
            footer={collectionFooter}
            hidden={mapSurfaceOpen}
            bind:element={collectionPaneElement}
            bind:listElement={collectionListElement}
            onViewModeChange={(mode) => (collectionQuery.viewMode = mode)}
            onScroll={rememberCollectionScroll} />{/if}
        {#if !mapSurfaceOpen && workbenchPaneVisibility.collection && workbenchPaneVisibility.content}<PaneResizeHandle
            label="Resize collection pane"
            value={activePaneDimensions().collectionWidth}
            min={collectionPaneMin}
            max={collectionPaneMax}
            onResize={(delta) => resizeWorkbenchPane("collection", delta)}
            onReset={() => resetWorkbenchPane("collection")} />{/if}

        {#if workbenchPaneVisibility.content && section === "language"}
          {@const languageProjection = projectionModule("language")}
          {#key selected?.id ?? "language"}
            <ModuleMount
              view={languageProjection.module.views[0]}
              context={buildModuleContext(languageProjection.module.manifest, projectInfo?.root ?? "", {
                focusEntityId: selected?.id as UUID | undefined,
                availableServices: enabledServices(),
                onEntityDeleted: () => {
                  void refreshAfterEntityMutation();
                },
                moduleState: { pane: languagePane },
                onModuleStateChange: (state: Record<string, unknown> | null) => {
                  const next = (state as { pane?: LanguagePane } | null)?.pane;
                  if (next && next !== languagePane) void switchLanguagePane(next);
                },
              })}
              className="language-mount" />
          {/key}
        {:else if workbenchPaneVisibility.content}
          {#snippet contentPaneBody()}
            {#if section === "maps" && sandboxView?.renderer === "maps" && sandboxView.view}
              {@const mapId = selected?.entity_type === "daena.maps:world-map" ? selected.id : null}
              {@const mapState = mapId ? (mapSaveStates[mapId] ?? null) : null}
              {@const mapDetail = mapConflictDetail(mapState?.detail)}
              <div class="map-editor-shell">
                {#if mapReconcileNotice || mapPickNotice}
                  <div class="map-editor-notices">
                    {#if mapReconcileNotice}<span class="map-reconcile-notice">{mapReconcileNotice}</span>{/if}
                    {#if mapPickNotice}<span class="map-reconcile-notice">{mapPickNotice}</span>{/if}
                  </div>
                {/if}
                {#if mapState?.status === "conflict"}
                  <div class="map-conflict-banner" role="alert">
                    <div class="map-conflict-copy">
                      <strong>This map changed on disk while you were editing</strong>
                      <p>Your draft was not saved over it. A recovery copy was exported so nothing is lost.</p>
                      {#if mapDetail.path}<code>{mapDetail.path}</code>{/if}
                    </div>
                    <div class="map-conflict-actions">
                      <button class="primary-button" type="button" disabled={mapRecoveryBusy} onclick={restoreMapDraft}
                        >{mapRecoveryBusy ? "Restoring…" : "Restore draft"}</button
                      ><button class="quiet-button" type="button" onclick={reloadMapOriginal}>Reload original</button
                      ><button class="quiet-button" type="button" onclick={dismissMapConflict}>Keep editing</button>
                    </div>
                  </div>
                {/if}
                <div class="map-surface">
                  {#if mapsEditorMode === "physical"}
                    {#key `${mapsEditorKey}:${mapReloadCounter}`}
                      <PhysicalMapEditor
                        mapId={mapsEditorKey.startsWith("draft-") ? undefined : (mapId ?? undefined)}
                        onstate={(status, detail) => {
                          if (!mapId) return;
                          mapSaveStates[mapId] = { status, detail };
                        }}
                        oncreated={async (map) => {
                          entities = [...entities.filter((entity) => entity.id !== map.id), map];
                          savedMapsCache = null;
                          selected = map;
                          mapsEditorKey = map.id;
                          mapsEditorMode = "vector";
                          await loadSelectedState(map);
                        }}
                        oncancel={() => {
                          void leavePluginView();
                        }} />
                    {/key}
                  {:else if mapsEditorMode === "vector"}
                    {#key `${mapsEditorKey}:${mapReloadCounter}`}
                      <NativeVectorMapEditor
                        mapId={mapsEditorKey.startsWith("draft-") ? undefined : (mapId ?? undefined)}
                        picking={Boolean(mapPickPending)}
                        start={mapsEditorKey.startsWith("draft-") ? mapsVectorStart : "geojson"}
                        focusLinkId={mapFocusLinkId ?? undefined}
                        focusFeatureId={mapFocusFeatureId ?? undefined}
                        onpick={(anchor) => void applyMapPick(anchor)}
                        onopen={(entityId) => void openMapEntityFromLink(entityId)}
                        onstate={(status, detail) => {
                          if (status === "fullscreen") {
                            editorFullscreen =
                              typeof detail === "object" &&
                              detail !== null &&
                              "enabled" in detail &&
                              detail.enabled === true;
                            return;
                          }
                          if (status === "back") {
                            void leavePluginView();
                            return;
                          }
                          if (!mapId) return;
                          mapSaveStates[mapId] = { status, detail };
                        }}
                        oncreated={async (map) => {
                          entities = [...entities.filter((entity) => entity.id !== map.id), map];
                          savedMapsCache = null;
                          selected = map;
                          mapsEditorKey = map.id;
                          mapsEditorMode = "vector";
                          await loadSelectedState(map);
                        }}
                        oncancel={() => {
                          void leavePluginView();
                        }} />
                    {/key}
                  {/if}
                </div>
              </div>
            {:else}
              <div class="editor-header">
                <div class="editor-title">
                  <span class="panel-kicker"
                    >{selected ? entityTypeLabel(selected.entity_type).toUpperCase() : emptyEditorKicker()}</span>
                  <div class="editor-title-row">
                    <h2
                      ondblclick={() => {
                        if (selected) void openEntityEditDialog();
                      }}
                      title={selected ? "Double-click to edit" : undefined}
                      style={selected ? "cursor:text" : undefined}>
                      {selected?.name ?? (section === "maps" ? "Choose a map" : "Choose an entry")}
                    </h2>
                    {#if selected}<button
                        class="quiet-button editor-rename-button"
                        type="button"
                        aria-label={`${ENTITY_ACTIONS.editIdentity} for ${selected.name}`}
                        title={ENTITY_ACTIONS.editIdentity}
                        onclick={() => void openEntityEditDialog()}
                        ><Pencil size={16} strokeWidth={1.8} aria-hidden="true" /></button
                      >{/if}
                    {#if selected && section === "houses" && selected.entity_type === HOUSE_TYPE}
                      {@const houseEntity = selected}
                      <button
                        class="quiet-button"
                        type="button"
                        aria-label={`${ENTITY_ACTIONS.openTree} for ${houseEntity.name}`}
                        onclick={() => void openHouseTree(houseEntity)}>{ENTITY_ACTIONS.openTree}</button>
                    {/if}
                  </div>
                </div>
                {#snippet editorStatusActions()}
                  {#if section === "maps"}<button
                      class="quiet-button"
                      type="button"
                      onclick={() => void openSelectedMapEditor()}>Open map editor</button
                    >{/if}
                {/snippet}
                <div class="editor-header-controls">
                  {#if selected}<div class="document-mode-toggle" aria-label="Document mode">
                      <button
                        type="button"
                        class:active={documentMode === "read"}
                        aria-pressed={documentMode === "read"}
                        disabled={selectedLoading || Boolean(selectedLoadError)}
                        onclick={() => void setDocumentMode("read")}>Article</button
                      ><button
                        type="button"
                        class:active={documentMode === "edit"}
                        aria-pressed={documentMode === "edit"}
                        disabled={selectedLoading || Boolean(selectedLoadError)}
                        onclick={() => void setDocumentMode("edit")}>Edit</button>
                    </div>{/if}
                  <StatusSummary
                    visible={Boolean(selected)}
                    loading={selectedLoading}
                    loadError={selectedLoadError}
                    saving={isSaving}
                    {saveError}
                    dirty={hasUnsavedChanges}
                    {savedAt}
                    actions={editorStatusActions}
                    onRetryLoad={() => void reloadSelectedFromDisk()}
                    onRetrySave={() => void flushAutoSave()} />
                </div>
              </div>
              {#if selected}
                {#if selectedLoading}
                  <WorkbenchState
                    kind="loading"
                    title="Loading entry"
                    message="Reading the document, details, relationships, and assets." />
                {:else if selectedLoadError}
                  {#snippet retrySelectedLoad()}<button
                      class="quiet-button"
                      type="button"
                      onclick={() => void reloadSelectedFromDisk()}>Retry</button
                    >{/snippet}
                  <WorkbenchState
                    kind="error"
                    title="Entry unavailable"
                    message={selectedLoadError}
                    actions={retrySelectedLoad} />
                {:else}
                  {@const activeConflict = documentConflict}
                  {#if activeConflict}
                    {#snippet conflictResolutionActions()}<div class="conflict-workbench-actions">
                        {#if !activeConflict.diagnostics.length}<details class="conflict-compare">
                            <summary>Compare with the currently saved document</summary>
                            <pre>{conflictDiskBody}</pre>
                          </details>{/if}
                        <div class="conflict-actions">
                          <button class="quiet-button" type="button" onclick={reloadConflict}>Reload disk</button
                          ><button
                            class="quiet-button"
                            type="button"
                            onclick={overwriteConflict}
                            disabled={activeConflict.diagnostics.length > 0}>Overwrite as new revision</button
                          ><button class="quiet-button" type="button" onclick={saveConflictRecoveryCopy}
                            >Save recovery copy</button>
                        </div>
                      </div>{/snippet}
                    <WorkbenchState
                      kind="conflict"
                      compact
                      title={activeConflict.diagnostics.length
                        ? "Canonical source needs attention"
                        : "This entry changed elsewhere"}
                      message={activeConflict.diagnostics.length
                        ? activeConflict.diagnostics[0]
                        : "Your unsaved document and details are preserved. Choose how to reconcile them before saving."}
                      actions={conflictResolutionActions} />
                  {/if}
                  {#if aiRewriteOpen}
                    <div class="ai-rewrite-modal-backdrop">
                      <div
                        class="ai-rewrite-panel"
                        role="dialog"
                        aria-modal="true"
                        aria-labelledby="ai-rewrite-title"
                        tabindex="-1"
                        onkeydown={(event) => {
                          if (event.key === "Escape" && !aiBusy) closeAiRewrite();
                        }}>
                        <div class="ai-rewrite-heading">
                          <div>
                            <span class="panel-kicker">{aiSettings.provider.name || "AI provider"}</span><strong
                              id="ai-rewrite-title"
                              >{aiCancelPending
                                ? "Cancelling request…"
                                : aiBusy
                                  ? aiMode === "generate"
                                    ? "Generating text…"
                                    : "Rewriting selection…"
                                  : aiPreviewOutput
                                    ? aiMode === "generate"
                                      ? "Review generated text"
                                      : "Review rewrite"
                                    : aiMode === "generate"
                                      ? "Generate text"
                                      : "Rewrite selection"}</strong>
                          </div>
                        </div>
                        {#if !aiBusy}<label class="ai-instruction"
                            >Instruction<textarea
                              rows="2"
                              bind:value={aiInstruction}
                              disabled={aiBusy}
                              placeholder={`Tell ${aiSettings.provider.name || "the AI provider"} how to rewrite the selection`}
                            ></textarea
                            ></label
                          >{/if}
                        <AiProposalPreview
                          original={aiSourceSelectionPlain}
                          bind:proposal={aiPreviewOutput}
                          streamText={aiStreamText}
                          progressMessage={aiProgressMessage}
                          busy={aiBusy}
                          cancelling={aiCancelPending}
                          onCancel={() => void cancelAiRewrite()}
                          onDiscard={closeAiRewrite}
                          onAccept={() => void acceptAiRewrite()} />
                        {#if aiUsage}<p class="muted-note">
                            Provider usage: {aiUsage.inputTokens} input + {aiUsage.outputTokens} output tokens.
                          </p>{/if}
                        {#if !aiBusy && aiPreviewOutput}<div class="ai-rewrite-actions">
                            <button class="primary-button" type="button" onclick={() => void acceptAiRewrite()}
                              >Accept proposal</button>
                            <button
                              class="quiet-button ai-retry-button"
                              type="button"
                              onclick={() => void startAiRewrite()}>Retry</button>
                            <button class="quiet-button ai-discard-button" type="button" onclick={closeAiRewrite}
                              >Discard</button>
                          </div>
                        {:else if !aiBusy}<div class="ai-rewrite-actions">
                            <button
                              class="primary-button"
                              type="button"
                              disabled={(aiMode === "rewrite" && !aiSourceSelection.trim()) || !aiInstruction.trim()}
                              onclick={() => void startAiRewrite()}
                              >{aiMode === "generate" ? "Generate text" : "Generate rewrite"}</button>
                            <button class="quiet-button" type="button" onclick={closeAiRewrite}>Cancel</button>
                          </div>{/if}
                      </div>
                    </div>
                  {/if}
                  {#if documentMode === "read"}
                    <MarkdownArticle
                      markdown={documentBody}
                      {entities}
                      onOpenEntity={(id) => {
                        const target = entities.find((entity) => entity.id === id && !entity.deleted);
                        if (target) void selectEntity(target);
                      }} />
                  {:else}
                    {#key selected?.id}
                      <RichTextEditor
                        bind:this={editorRef}
                        value={documentBody}
                        {entities}
                        searchEntities={searchEntitiesPaged()}
                        entityId={selected?.id ?? null}
                        defaultNamespace={primarySchemaNamespace(activeManifest()?.schemas, {
                          entityType: selected?.entity_type,
                          fallback: activeModuleId(),
                        })}
                        editable={!selectedLoading &&
                          !selectedLoadError &&
                          projectDiagnostics.length === 0 &&
                          !aiBusy &&
                          !aiRewriteOpen}
                        fullscreen={editorFullscreen}
                        aiEnabled={projectInfo?.aiEnabled ?? false}
                        onChange={updateDocumentBody}
                        onSelectionChange={setAiSelection}
                        onAiRequest={openAiAction}
                        onSaveRequest={() => void flushAutoSave()}
                        onFullscreenChange={setEditorFullscreen}
                        placeholder={section === "writing"
                          ? writingView === "manuscripts"
                            ? "Write your manuscript…"
                            : writingView === "reference"
                              ? "Write this reference page…"
                              : `Write this ${createLabel()}…`
                          : section === "maps"
                            ? "Describe this map and the world it contains…"
                            : "Write the canonical story of this entry…"} />
                    {/key}
                  {/if}
                  <div class="editor-footer">
                    <div>
                      {#if entityMutation.phase !== "idle"}
                        <MutationStatus
                          snapshot={entityMutation.snapshot}
                          onRetry={() => void archiveSelected()}
                          onReload={() => void reloadEntityEditFromServer()}
                          onReviewDraft={() => entityMutation.reset()} />
                      {/if}
                    </div>
                    <div>
                      {#if selected}
                        <EntityArchiveAction
                          entityName={selected.name}
                          busy={entityMutation.busy}
                          disabled={selectedLoading || projectDiagnostics.length > 0}
                          onArchive={() => void archiveSelected()} />
                      {/if}
                    </div>
                  </div>
                {/if}
              {:else}
                <WorkbenchState
                  kind="empty"
                  title={section === "maps"
                    ? "Your map notes are waiting."
                    : section === "writing"
                      ? writingView === "manuscripts"
                        ? "Your draft is waiting."
                        : writingView === "reference"
                          ? "Your reference desk is waiting."
                          : "Your canvas is waiting."
                      : "Your canvas is waiting."}
                  message={section === "maps"
                    ? "Select a map from the atlas, or create one with a map integration."
                    : "Select an entry from the collection, or create something new to begin writing."} />
              {/if}
            {/if}
          {/snippet}
          <ContentPane
            fullscreen={editorFullscreen}
            mapEditorActive={section === "maps" && sandboxView?.renderer === "maps" && Boolean(sandboxView.view)}
            bind:element={contentPaneElement}
            children={contentPaneBody} />
        {/if}

        {#if !mapSurfaceOpen && workbenchPaneVisibility.content && workbenchPaneVisibility.inspector && workbenchSupportsInspector()}<PaneResizeHandle
            label="Resize inspector pane"
            value={activePaneDimensions().inspectorWidth}
            min={inspectorPaneMin}
            max={inspectorPaneMax}
            direction={-1}
            onResize={(delta) => resizeWorkbenchPane("inspector", delta)}
            onReset={() => resetWorkbenchPane("inspector")} />{/if}

        {#if workbenchPaneVisibility.inspector && workbenchSupportsInspector() && selected}
          {@const inspectedEntity = selected}
          {#snippet inspectorBody()}
            <div class="inspector-heading">
              <div>
                <span class="panel-kicker">INSPECTOR</span><strong
                  >{entityTypeLabel(inspectedEntity.entity_type)}</strong>
              </div>
              <div class="inspector-heading-actions">
                <span class="inspector-type">{inspectedEntity.entity_type}</span
                >{#if projectInfo?.aiEnabled && emptyInspectorDefinitions().length}<button
                    class="inspector-ai-action"
                    type="button"
                    onclick={() => void fillAiFields()}
                    disabled={aiFieldFillBusy}
                    ><span aria-hidden="true">✦</span>{aiFieldFillBusy ? "Finding…" : "Fill with AI"}</button
                  >{/if}
              </div>
            </div>
            {#if section === "houses" && inspectedEntity.entity_type === HOUSE_TYPE}
              <InspectorSection title="House" count={1}>
                <section class="inspector-section inspector-section-plain">
                  <p class="inspector-group-empty">
                    {formatHouseMemberSummary(houseCollectionSummaries.get(inspectedEntity.id), {
                      pending: houseSummariesPending && !houseCollectionSummaries.has(inspectedEntity.id),
                    })}
                  </p>
                  <div class="inspector-ai-fill-actions">
                    <button class="quiet-button" type="button" onclick={() => void openHouseTree(inspectedEntity)}
                      >{ENTITY_ACTIONS.openTree}</button>
                  </div>
                  <p class="inspector-group-empty">
                    Edit membership roles in Tree → House dock. The Members relationship field below links people.
                  </p>
                </section>
              </InspectorSection>
            {/if}
            {#if aiFieldFillOpen}<section class="inspector-ai-fill">
                <div class="inspector-ai-fill-heading">
                  <strong>{aiFieldFillBusy ? "Finding field suggestions…" : "Review field suggestions"}</strong><button
                    class="quiet-button"
                    type="button"
                    onclick={closeAiFieldFill}>{aiFieldFillBusy ? "Cancel" : "Close"}</button>
                </div>
                {#if aiFieldFillBusy}<p>
                    Using this entry and related project context.
                  </p>{:else if Object.keys(aiFieldSuggestions).length === 0}<p>
                    No suggestions are available.
                  </p>{:else}{#each Object.entries(aiFieldSuggestions) as [key, suggestion]}{@const definition =
                      definitions().find((candidate) => candidate.key === key)}
                    <div class="inspector-ai-suggestion">
                      <div class="inspector-ai-suggestion-heading">
                        <strong>{definition?.label ?? key}</strong><button
                          type="button"
                          class={`inspector-ai-confidence confidence-${suggestionConfidenceTone(suggestion.confidence)}`}
                          aria-label={`${suggestionConfidenceLabel(suggestion.confidence)} confidence${suggestion.rationale ? ". Show reasoning" : ""}`}
                          >{suggestionConfidenceLabel(suggestion.confidence)}{#if suggestion.rationale}<span
                              class="inspector-ai-reasoning"
                              role="tooltip">{suggestion.rationale}</span
                            >{/if}</button>
                      </div>
                      <span>{suggestionDisplayValue(key, suggestion)}</span>
                      <div>
                        <button class="quiet-button" type="button" onclick={() => acceptAiFieldSuggestion(key)}
                          >Accept</button
                        ><button class="quiet-button" type="button" onclick={() => discardAiFieldSuggestion(key)}
                          >Discard</button>
                      </div>
                    </div>{/each}{/if}
                {#if !aiFieldFillBusy}<div class="inspector-ai-fill-actions">
                    {#if Object.keys(aiFieldSuggestions).length > 0}<button
                        class="primary-button"
                        type="button"
                        onclick={() => void acceptAllAiFieldSuggestions()}>Accept all</button
                      >{/if}<button class="quiet-button" type="button" onclick={() => void fillAiFields()}>Retry</button
                    ><button class="quiet-button" type="button" onclick={closeAiFieldFill}>Close</button>
                  </div>{/if}
              </section>{/if}
            <InspectorSection title="Details" count={propertyDefinitions().length + chronologyDateDefinitions().length}>
              {#if hasChronologySection()}
                <section class="inspector-section inspector-section-plain">
                  <h3>Chronology</h3>
                  {#if eraRelationshipDefinition()}
                    {@const eraField = eraRelationshipDefinition()!}
                    <div class="property-field">
                      <span>{eraField.label}</span>
                      <RelationshipPicker
                        field={eraField}
                        search={searchEntitiesPaged(eraField)}
                        resolveSelected={resolveSelectedEntities}
                        selectedIds={selectedRelationshipIds(eraField)}
                        placeholder="Search eras…"
                        onChange={(ids) => void updateRelationshipField(eraField, ids)} />
                    </div>
                  {/if}
                  {#each inspectorChronologyWarnings() as warning}
                    <p class="chronology-warning" role="status">{warning}</p>
                  {/each}
                  {#each chronologyDateDefinitions() as definition}
                    <div class="property-field">
                      <span
                        >{definition.label}{#if definition.required}<b>*</b>{/if}</span>
                      {#if dateForField(definition.key) || dateEditorOpen[definition.key]}
                        <DateEditor
                          label={definition.label}
                          value={fields[definition.key]}
                          calendar={definitionForDateField(definition.key)}
                          calendars={worldCalendars() as any}
                          selectedCalendarId={selectedCalendarId(definition.key)}
                          onChange={(next) => {
                            fields = { ...fields, [definition.key]: next };
                            markEntryDirty();
                          }}
                          onClear={() => clearDateField(definition.key)}
                          onSelectCalendar={(id) => setDateCalendar(definition.key, id)} />
                      {:else}
                        <button class="date-empty" type="button" onclick={() => openDateEditor(definition.key)}
                          >Add a date</button>
                      {/if}
                    </div>
                  {/each}
                </section>
              {/if}
              <section class="inspector-section inspector-section-plain">
                <h3>Properties</h3>
                {#each propertyDefinitions() as definition}<div class="property-field">
                    <span
                      >{definition.label}{#if definition.required}<b>*</b>{/if}</span
                    >{#if definition.type === "date"}{#if dateForField(definition.key) || dateEditorOpen[definition.key]}{@const date =
                          dateDraftForField(definition.key) ?? {
                            calendar: GREGORIAN_CALENDAR_ID,
                            era: "CE",
                            precision: "day",
                          }}{@const parts = datePartsDraft(definition.key)}{@const calendar = definitionForDateField(
                          definition.key,
                        )}{@const months = calendar?.months ?? []}
                        <DateEditor
                          label={definition.label}
                          value={fields[definition.key]}
                          calendar={definitionForDateField(definition.key)}
                          calendars={worldCalendars() as any}
                          selectedCalendarId={selectedCalendarId(definition.key)}
                          onChange={(next) => {
                            fields = { ...fields, [definition.key]: next };
                            markEntryDirty();
                          }}
                          onClear={() => clearDateField(definition.key)}
                          onSelectCalendar={(id) => setDateCalendar(definition.key, id)} />{:else}<button
                          class="date-empty"
                          type="button"
                          onclick={() => openDateEditor(definition.key)}>Add a date</button
                        >{/if}{:else if definition.type === "boolean"}<input
                        type="checkbox"
                        aria-label={definition.label}
                        checked={fieldInputValue(definition, fields[definition.key]) === true}
                        onchange={(event) =>
                          updateField(definition, event)} />{:else if definition.type === "number"}<input
                        type="number"
                        aria-label={definition.label}
                        value={fieldInputValue(definition, fields[definition.key])}
                        placeholder="Add {definition.label.toLowerCase()}"
                        onchange={(event) =>
                          updateField(
                            definition,
                            event,
                          )} />{:else if definition.type === "enum" && definition.options?.length}<select
                        aria-label={definition.label}
                        multiple={definition.multiple ?? false}
                        value={definition.multiple
                          ? Array.isArray(fields[definition.key])
                            ? fields[definition.key]
                            : []
                          : String(fields[definition.key] ?? "")}
                        onchange={(event) => updateField(definition, event)}
                        >{#each definition.options ?? [] as option}<option value={option}>{option}</option
                          >{/each}</select>
                      >{:else if (definition as any).type === "oneof"}<select
                        aria-label={definition.label}
                        value={String(fields[definition.key] ?? "")}
                        onchange={(event) => updateField(definition, event)}
                        ><option value="">Choose {definition.label.toLowerCase()}</option
                        >{#each definition.options ?? [] as option}<option value={option}>{option}</option>{/each}
                        {#each (definition as any).oneOf ?? [] as variant}
                          {#each variant.options ?? [] as opt}<option value={opt}>{variant.label}: {opt}</option>{/each}
                        {/each}</select>
                      >{:else}<input
                        type="text"
                        value={fieldInputValue(definition, fields[definition.key])}
                        placeholder="Add {definition.label.toLowerCase()}"
                        oninput={(event) => updateField(definition, event)} />{/if}
                  </div>{/each}
              </section>
              {#if selected?.entity_type === "daena.timeline:calendar" && projectInfo}
                <section class="inspector-section inspector-section-plain">
                  <CalendarEditor
                    context={contextFor("timeline")}
                    entityId={inspectedEntity.id as UUID}
                    onsaved={(definition) => {
                      if (selected) calendarDefinitions = { ...calendarDefinitions, [selected.id]: definition };
                    }}
                    onOpenEra={(eraId) => {
                      const entity = entities.find((candidate) => candidate.id === eraId);
                      if (entity) void selectEntity(entity);
                    }}
                    onEraCreated={(created) => {
                      void refreshAfterEntityMutation({ entityId: created.id }).then(() =>
                        selectEntity({
                          id: created.id,
                          name: created.name,
                          entity_type: created.type,
                          deleted: created.deleted,
                          created_at: created.createdAt,
                          updated_at: created.updatedAt,
                          revision: created.revision,
                        }),
                      );
                    }} />
                </section>
              {/if}
            </InspectorSection>
            <InspectorSection
              title="Relationships"
              count={otherRelationshipDefinitions().reduce(
                (total, definition) => total + selectedRelationshipIds(definition).length,
                0,
              )}>
              {#if otherRelationshipDefinitions().length === 0}<p class="inspector-group-empty">
                  No relationship fields are available for this entry.
                </p>{/if}
              {#each otherRelationshipDefinitions() as definition}<section
                  class="inspector-section inspector-section-plain relationship-field-section">
                  <div class="section-title">
                    <h3>{definition.label}</h3>
                    <span>{selectedRelationshipIds(definition).length}</span>
                  </div>
                  <RelationshipPicker
                    field={definition}
                    search={searchEntitiesPaged(definition)}
                    resolveSelected={resolveSelectedEntities}
                    selectedIds={selectedRelationshipIds(definition)}
                    hideChips
                    onChange={(ids) => void updateRelationshipField(definition, ids)}
                    onCreate={definition.relationshipType === "family_member_of"
                      ? (name) => createRelationshipTarget(definition, name)
                      : undefined} />
                  {#if relationshipsForDefinition(definition).length > 0}<div
                      class="relationship-detail-list"
                      aria-label={`${definition.label} details`}>
                      {#each relationshipsForDefinition(definition) as relationship (relationship.id)}
                        {@const relDefinition = definitionForRelationship(relationship) ?? definition}
                        {@const summary = relationshipMetadataSummary(relationship, relDefinition)}
                        <div class="relationship-detail-row">
                          <button
                            type="button"
                            class="relationship-detail-copy related-item-trigger"
                            data-entity-id={relationshipOtherId(definition, relationship)}
                            aria-label={`Preview ${relationshipTargetName(relationship, definition)}`}>
                            <strong>{relationshipTargetName(relationship, definition)}</strong>
                            <small
                              >{entities.find((entity) => entity.id === relationshipOtherId(definition, relationship))
                                ?.entity_type ?? "Entity"}{#if summary}
                                · {summary}{/if}</small>
                          </button>
                          <div class="relationship-detail-actions">
                            {#if relDefinition.metadataFields?.length}
                              <button
                                class="quiet-button relationship-details-button"
                                type="button"
                                aria-label={`Edit details for ${relationship.relationship_type} to ${relationshipTargetName(relationship, definition)}`}
                                onclick={() => openRelationshipMetadata(relationship)}
                                ><Pencil size={16} strokeWidth={1.8} aria-hidden="true" /></button>
                            {/if}
                            <button
                              class="quiet-button relationship-remove-button"
                              type="button"
                              aria-label={`Remove ${relationshipTargetName(relationship, definition)} from ${definition.label}`}
                              onclick={() => void confirmRemoveRelationship(definition, relationship)}
                              ><X size={16} strokeWidth={1.8} aria-hidden="true" /></button>
                          </div>
                        </div>
                      {/each}
                    </div>{/if}
                </section>{/each}
            </InspectorSection>
            <InspectorSection title="Assets" count={assets.length}>
              <section class="inspector-section inspector-section-plain">
                <div class="section-title">
                  <h3>Attached files</h3>
                  <span>{assets.length}</span>
                </div>
                <button class="drop-zone" type="button" onclick={attachAsset}
                  ><span><Plus size={16} strokeWidth={1.8} aria-hidden="true" /></span><strong>Attach a file</strong
                  ><small>Copied into this project</small></button
                >{#each assets as asset (asset.id)}<button
                    type="button"
                    class:asset-main={asset.role === "profile"}
                    class="asset-row asset-row-button"
                    onclick={() => openAssetDialog(asset)}
                    disabled={assetBusyId === asset.id}
                    aria-label={`Edit ${asset.filename}`}>
                    <span class="asset-icon" aria-hidden="true">{asset.role === "profile" ? "◆" : "□"}</span>
                    <div class="asset-details">
                      <strong>{asset.filename}</strong><small
                        >{Math.max(1, Math.round(asset.size / 1024))} KB · {asset.reference_scope === "project"
                          ? "Project references allowed"
                          : "Entity only"} · {asset.mime_type}</small
                      >{#if asset.role === "profile"}<span class="asset-role">Main file</span>{/if}
                    </div>
                    <span class="asset-row-edit-hint" aria-hidden="true">Edit →</span>
                  </button>{/each}
              </section>
            </InspectorSection>
            <InspectorSection title="Backlinks" count={backlinkRelationships().length} open={false}>
              {#if backlinkRelationships().length === 0}<p class="inspector-group-empty">
                  Nothing links to this entry yet.
                </p>{:else}<div class="relationship-detail-list backlink-list" aria-label="Backlinks">
                  {#each backlinkRelationships() as relationship (relationship.id)}<div class="relationship-detail-row">
                      <button
                        type="button"
                        class="relationship-detail-copy related-item-trigger"
                        data-entity-id={relationship.source_id}
                        aria-label={`Preview ${relationshipSourceName(relationship)}`}>
                        <strong>{relationshipSourceName(relationship)}</strong><small
                          >{entityTypeLabel(
                            entities.find((entity) => entity.id === relationship.source_id)?.entity_type ?? null,
                          )} · {relationship.relationship_type}</small>
                      </button>
                    </div>{/each}
                </div>{/if}
            </InspectorSection>
            {#if mapsEnabled()}<InspectorSection title="Maps" count={mapLocations.length} open={false}
                ><section
                  class="inspector-section inspector-section-plain map-contribution"
                  aria-label="Maps contribution">
                  <div class="section-title">
                    <h3>Maps</h3>
                    <span>{mapLocations.length}</span>
                  </div>
                  {#if mapLocations.length === 0}<small>No map links yet.</small
                    >{:else}{#each mapLocations as location (location.id)}<div class="map-location-row">
                        <div>
                          <strong>{location.label || location.role}</strong><small
                            >{location.role} · {location.mapEntityId.slice(
                              0,
                              8,
                            )}{#if location.resolution === "unresolved"}
                              · <span class="map-unresolved-badge">Unresolved</span>{/if}</small>
                        </div>
                        <div>
                          {#if location.resolution === "unresolved"}<span
                              class="map-unresolved-note"
                              title="The map feature this link pointed to was removed or renumbered."
                              >Feature missing</span
                            >{:else}<button
                              class="quiet-button"
                              type="button"
                              onclick={() => void openMapLocation(location)}>Show on map</button
                            >{/if}<button
                            class="quiet-button"
                            type="button"
                            onclick={() => void editMapLocation(location)}>Edit</button
                          ><button class="quiet-button" type="button" onclick={() => void rebindMapLocation(location)}
                            >Rebind</button
                          ><button class="quiet-button" type="button" onclick={() => void unlinkMapLocation(location)}
                            >Unlink</button>
                        </div>
                      </div>{/each}{/if}
                </section></InspectorSection
              >{/if}
          {/snippet}
          <InspectorPane
            loading={selectedLoading}
            error={selectedLoadError}
            bind:element={inspectorPaneElement}
            children={inspectorBody}
            onRetry={() => void reloadSelectedFromDisk()} />
        {:else if workbenchPaneVisibility.inspector && workbenchSupportsInspector()}
          <InspectorPane bind:element={inspectorPaneElement} empty />
        {/if}
      </section>
    {/if}
    {#if error}<div class="toast" role="alert" aria-live="assertive">
        {error}<button aria-label="Dismiss" onclick={() => (error = "")}
          ><X size={16} strokeWidth={1.8} aria-hidden="true" /></button>
      </div>{/if}
    {#if lifecycleToast}<div class="toast lifecycle-toast" role="status" aria-live="polite">
        <span>{lifecycleToast.message}</span>
        {#if lifecycleToast.actionLabel && lifecycleToast.onAction}
          <button type="button" class="toast-action" onclick={() => lifecycleToast?.onAction?.()}
            >{lifecycleToast.actionLabel}</button>
        {/if}
        <button aria-label="Dismiss" onclick={dismissLifecycleToast}
          ><X size={16} strokeWidth={1.8} aria-hidden="true" /></button>
      </div>{/if}
  </section>
  {#if ready}<EntityHoverCard {entities} onOpen={(entity) => void selectEntity(entity)} />
    <button
      class="mobile-create-button"
      aria-label={`${ENTITY_ACTIONS.new} ${createLabel()}`}
      aria-expanded={showCreateForm}
      onclick={openContextualCreate}><Plus size={18} strokeWidth={1.8} aria-hidden="true" /></button
    >{/if}
</main>
{#if metadataDialog}
  {@const dialog = metadataDialog}
  <RelationshipMetadataDialog
    relationship={dialog.relationship}
    definition={dialog.definition}
    {entities}
    {calendarDefinitions}
    onSave={(metadata) => saveRelationshipMetadata(dialog.relationship, metadata)}
    onClose={() => (metadataDialog = null)} />
{/if}
{#if assetDialog}
  <AssetDialog
    asset={assetDialog}
    editable={canWriteAssets()}
    onSave={handleAssetSave}
    onDelete={handleAssetDelete}
    onReplace={handleAssetReplace}
    onClose={() => (assetDialog = null)} />
{/if}
{#if showExternalImport}
  <ExternalImportDialog
    {modules}
    {entities}
    onCommitted={async () => {
      clearSelection();
      await loadEntities();
    }}
    onClose={() => (showExternalImport = false)} />
{/if}
{#if entityEditDialog}
  <EntityIdentityDialog
    entityName={entityEditDialog.entity.name}
    bind:name={entityEditDialog.name}
    bind:entityType={entityEditDialog.entityType}
    originalType={entityEditDialog.entity.entity_type}
    typeGroups={groupedEditTypes()}
    typeLabel={entityTypeLabel}
    workspaceLabel={(type) => {
      const owner = sectionForEntityType(type);
      if (owner) return workspaceSectionLabel(owner);
      return type ? "Other" : "Uncategorized";
    }}
    warning={editTypeWarning()}
    busy={entityEditDialog.busy || entityMutation.busy}
    allowUncategorized={entityEditDialog.entity.entity_type == null}
    onSave={() => void saveEntityEditDialog()}
    onClose={closeEntityEditDialog}>
    {#snippet mutation()}
      {#if entityMutation.phase === "conflict" || entityMutation.phase === "failed" || entityMutation.phase === "saving"}
        <MutationStatus
          snapshot={entityMutation.snapshot}
          onRetry={() => void saveEntityEditDialog()}
          onReload={() => void reloadEntityEditFromServer()}
          onReviewDraft={() => entityMutation.reset()} />
      {/if}
    {/snippet}
  </EntityIdentityDialog>
{/if}
<DialogHost />

<style>
:global(*) {
  box-sizing: border-box;
}
:global(:root) {
  --ink: #25251f;
  --ink-soft: #77766d;
  --ink-faint: #aaa79d;
  --ink-muted: #62594e;
  --line: #e4e1d8;
  --line-soft: #e9e1d4;
  --line-strong: #d9cdbd;
  --surface: #fffefa;
  --surface-muted: #f4f2ec;
  --surface-warm: #f4eee3;
  --surface-subtle: #f7f3ec;
  --surface-quiet: #fffcf7;
  --canvas: #f7f6f2;
  --accent: #b4773f;
  --accent-dark: #365342;
  --accent-soft: #c99965;
  --accent-bg: #f2e4d2;
  --on-accent: #fffefa;
  --on-bright-accent: #fffefa;
  --brass-ink: #2f2619;
  --danger: #a14f42;
  --danger-bg: #fdf2ef;
  --danger-line: #e7c4bc;
  --success: #557d63;
  --success-bg: #eef5ef;
  --success-line: #c8d8cb;
  --warning: #8a5f24;
  --warning-bg: #fff8ed;
  --warning-line: #ead7bc;
  --info: #4e6f7c;
  --info-bg: #e8f1f3;
  --info-line: #bfd3d9;
  --theme-surface-bg: var(--surface);
  --theme-muted-bg: var(--surface-muted);
  --theme-neutral-border: var(--line);
  --theme-neutral-border-strong: var(--line-strong);
  --theme-neutral-text: var(--ink);
  --theme-neutral-text-soft: var(--ink-soft);
  --theme-neutral-text-muted: var(--ink-faint);
  --theme-danger-bg: var(--danger-bg);
  --theme-danger-border: var(--danger-line);
  --theme-danger-text: var(--danger);
  --theme-success-bg: var(--success-bg);
  --theme-success-border: var(--success-line);
  --theme-success-text: var(--success);
  --theme-warning-bg: var(--warning-bg);
  --theme-warning-border: var(--warning-line);
  --theme-warning-text: var(--warning);
  --theme-info-bg: var(--info-bg);
  --theme-info-border: var(--info-line);
  --theme-info-text: var(--info);
  --focus-ring: color-mix(in srgb, var(--accent-soft) 72%, transparent);
  --focus-ring-strong: var(--accent-soft);
  --control-min-height: 36px;
  --touch-target-min: 44px;
  --rail-bg: #283a30;
  --rail-surface: #3b5243;
  --rail-surface-strong: #486052;
  --rail-popover: #2f4a38;
  --rail-text: #eef0e9;
  --rail-text-soft: #b9c8bc;
  --rail-text-muted: #aab9ad;
  --rail-text-faint: #91a397;
  --rail-accent: #d5ab6c;
  --rail-accent-hover: #e1bc82;
  --rail-online: #88c18e;
  --rail-offline: #777f78;
  --rail-border: #486052;
  --shadow-sm: 0 2px 8px rgba(38, 42, 33, 0.05);
  --shadow-md: 0 8px 24px rgba(38, 42, 33, 0.12);
  --shadow-lg: 0 18px 50px rgba(38, 42, 33, 0.08);
  --font-display: Georgia, serif;
  color-scheme: light;
}
:global(:root[data-theme="dark"]) {
  --ink: #f2eee4;
  --ink-soft: #d8d1c3;
  --ink-faint: #a49e92;
  --ink-muted: #b8b1a5;
  --line: #31443a;
  --line-soft: #26372f;
  --line-strong: #435a4e;
  --surface: #131f1b;
  --surface-muted: #182720;
  --surface-warm: #182720;
  --surface-subtle: #15231d;
  --surface-quiet: #101b17;
  --canvas: #0e1714;
  --accent: #c58a4a;
  --accent-dark: #557d63;
  --accent-soft: #d7a25c;
  --accent-bg: #3c3525;
  --on-accent: #fffefa;
  --on-bright-accent: #25251f;
  --brass-ink: #2f2619;
  --danger: #e09a8d;
  --danger-bg: #321f1c;
  --danger-line: #70443d;
  --success: #8aad69;
  --success-bg: #1b2d20;
  --success-line: #3f5d44;
  --warning: #e4c786;
  --warning-bg: #3c3525;
  --warning-line: #6c5830;
  --info: #91a4ae;
  --info-bg: #1d2930;
  --info-line: #3f5662;
  --shadow-sm: 0 2px 10px rgba(0, 0, 0, 0.22);
  --shadow-md: 0 10px 28px rgba(0, 0, 0, 0.28);
  --shadow-lg: 0 22px 70px rgba(0, 0, 0, 0.34);
  color-scheme: dark;
}
:global(body) {
  margin: 0;
  background: var(--canvas);
  color: var(--ink);
  font-family: Inter, ui-sans-serif, system-ui, sans-serif;
}
:global(button),
:global(input),
:global(select) {
  font: inherit;
}
.studio-shell {
  min-height: 100vh;
  display: flex;
}
.app-main {
  min-width: 0;
  flex: 1;
}
.app-main.sandbox-active {
  display: flex;
  min-height: 0;
  flex-direction: column;
  overflow: hidden;
}
.disabled-state {
  max-width: 1080px;
  min-height: calc(100vh - 58px);
  margin: auto;
  padding: 10vh 7vw;
  display: flex;
  align-items: center;
  gap: 8vw;
}
.welcome {
  position: relative;
  isolation: isolate;
  min-height: 100vh;
  overflow: hidden;
  display: flex;
  align-items: center;
  padding: clamp(60px, 8vh, 100px) clamp(38px, 6vw, 96px);
  background: var(--theme-warning-bg, #f6f2ea) url("/hero.png") center / cover no-repeat;
}
.welcome::after {
  content: "";
  position: absolute;
  inset: 0;
  z-index: -1;
  pointer-events: none;
  background: linear-gradient(90deg, rgba(250, 247, 240, 0.58) 0%, rgba(250, 247, 240, 0.18) 36%, transparent 58%);
}
:global(:root[data-theme="dark"]) .welcome::after {
  background: linear-gradient(90deg, rgba(14, 23, 20, 0.9) 0%, rgba(14, 23, 20, 0.64) 38%, rgba(14, 23, 20, 0.12) 68%);
}
.welcome-copy {
  position: relative;
  z-index: 2;
  width: min(44%, 500px);
}
.overline,
.panel-kicker {
  display: block;
  color: var(--accent);
  font-size: 10px;
  font-weight: 800;
  letter-spacing: 0.18em;
}
.welcome h1 {
  margin: 20px 0 18px;
  color: var(--ink);
  font: 500 clamp(48px, 6vw, 78px)/0.98 var(--font-display);
  letter-spacing: -0.04em;
}
.welcome h1 em {
  color: var(--accent);
  font-style: italic;
}
.welcome p {
  max-width: 380px;
  margin: 0;
  color: var(--ink-soft);
  font-size: 16px;
  line-height: 1.7;
}
.welcome-map {
  position: absolute;
  inset: 0;
  pointer-events: none;
}
.map-marker {
  position: absolute;
  z-index: 1;
  display: grid;
  place-items: center;
  width: 48px;
  height: 48px;
  border: 1px solid rgba(93, 83, 65, 0.14);
  border-radius: 50%;
  background: rgba(255, 253, 247, 0.96);
  color: var(--accent);
  box-shadow: 0 9px 24px rgba(52, 50, 40, 0.16);
}
.marker-atlas {
  top: 10%;
  left: 67%;
  background: #294a39;
  color: #f4efe5;
}
.marker-people {
  top: 25%;
  left: 56%;
  background: #b87a3d;
  color: #fff9ee;
}
.marker-sword {
  top: 54%;
  left: 49%;
}
.marker-tree {
  top: 74%;
  left: 56%;
  background: var(--accent-dark);
  color: #f4efe5;
}
.art-card {
  position: absolute;
  top: 41%;
  right: 11%;
  z-index: 2;
  width: 260px;
  padding: 25px 25px 22px;
  border: 1px solid color-mix(in srgb, var(--line-strong) 72%, transparent);
  border-radius: 13px;
  background: color-mix(in srgb, var(--surface) 93%, transparent);
  box-shadow: 0 18px 42px rgba(40, 48, 38, 0.18);
  backdrop-filter: blur(4px);
}
.art-card span,
.art-card small {
  display: block;
  color: var(--accent);
  font-size: 9px;
  font-weight: 800;
  letter-spacing: 0.16em;
}
.art-card strong {
  display: block;
  margin: 16px 0 28px;
  color: var(--ink);
  font: 500 21px/1.2 var(--font-display);
}
.art-card small {
  color: var(--ink-faint);
  font-weight: 500;
  letter-spacing: 0;
}
.primary-button,
.quiet-button,
.add-button {
  border: 0;
  border-radius: 8px;
  cursor: pointer;
}
.primary-button {
  padding: 10px 15px;
  border: 1px solid rgba(255, 255, 255, 0.08);
  background: var(--accent-dark);
  color: #fff;
  font-weight: 700;
  font-size: 12px;
  box-shadow:
    0 2px 0 #263d30,
    0 7px 16px rgba(42, 68, 51, 0.16);
  transition:
    background 0.16s ease,
    box-shadow 0.16s ease,
    transform 0.16s ease;
}
.primary-button:hover {
  background: #2b4535;
  box-shadow:
    0 2px 0 #263d30,
    0 10px 20px rgba(42, 68, 51, 0.2);
  transform: translateY(-1px);
}
.primary-button:active {
  box-shadow:
    0 1px 0 #263d30,
    0 3px 8px rgba(42, 68, 51, 0.14);
  transform: translateY(1px);
}
.primary-button:focus-visible {
  outline: 3px solid rgba(180, 119, 63, 0.32);
  outline-offset: 2px;
}
.primary-button:disabled {
  opacity: 0.5;
  cursor: not-allowed;
  box-shadow: none;
  transform: none;
}
.quiet-button {
  padding: 10px 12px;
  border: 1px solid var(--theme-warning-border, #ded8cd);
  background: var(--surface);
  color: var(--ink-soft);
  font-size: 12px;
  box-shadow: 0 1px 2px rgba(48, 45, 38, 0.05);
  transition:
    background 0.16s ease,
    border-color 0.16s ease,
    box-shadow 0.16s ease,
    color 0.16s ease,
    transform 0.16s ease;
}
.quiet-button:hover {
  border-color: var(--theme-warning-border, #cbbda9);
  background: var(--surface-muted);
  color: var(--ink);
  box-shadow: 0 3px 8px rgba(48, 45, 38, 0.08);
  transform: translateY(-1px);
}
.quiet-button:active {
  box-shadow: 0 1px 2px rgba(48, 45, 38, 0.05);
  transform: translateY(1px);
}
.quiet-button:focus-visible {
  outline: 3px solid rgba(180, 119, 63, 0.24);
  outline-offset: 2px;
}
.quiet-button:disabled {
  opacity: 0.5;
  cursor: not-allowed;
  box-shadow: none;
  transform: none;
}
:global(.module-mount.language-mount) {
  display: flex;
  flex-direction: column;
  min-height: 650px;
  border: none;
  background: transparent;
  box-shadow: none;
}
:global(.module-mount.language-mount .language-workspace) {
  display: flex;
  flex-direction: column;
  height: 100%;
  min-height: 0;
  flex: 1;
}
.projection-bar {
  min-height: 42px;
  margin: 0 40px 15px;
  padding: 0 14px;
  border: 1px solid var(--line);
  border-radius: 9px;
  background: rgba(255, 254, 250, 0.72);
}
.projection-bar:empty {
  display: none;
}
.workbench-layout-controls {
  display: flex;
  align-items: center;
  justify-content: flex-end;
  gap: 4px;
  margin: -14px 40px 12px;
}
.workbench-layout-controls > span {
  margin-right: 3px;
  color: var(--ink-faint);
  font-size: 9px;
  font-weight: 750;
  letter-spacing: 0.1em;
  text-transform: uppercase;
}
.workbench-layout-controls button {
  padding: 5px 8px;
  border: 1px solid var(--line);
  border-radius: 6px;
  background: transparent;
  color: var(--ink-faint);
  cursor: pointer;
  font-size: 9px;
}
.workbench-layout-controls button:hover,
.workbench-layout-controls button[aria-pressed="true"] {
  border-color: var(--line-strong);
  background: var(--surface);
  color: var(--ink);
}
.workspace-grid {
  display: grid;
  grid-template-columns: var(--collection-pane-width, 245px) minmax(360px, 1fr) var(--inspector-pane-width, 270px);
  gap: 4px;
  padding: 0 40px 40px;
}
.workspace-grid-no-inspector {
  grid-template-columns: var(--collection-pane-width, 245px) minmax(360px, 1fr);
}
.maps-workspace {
  grid-template-columns: var(--collection-pane-width, 245px) minmax(0, 1fr);
}
.app-main.map-surface-open {
  display: flex;
  min-height: 0;
  height: 100vh;
  flex-direction: column;
  overflow: hidden;
}
.workspace-grid.map-surface-expanded {
  display: grid;
  min-height: 0;
  flex: 1 1 auto;
  grid-template-columns: minmax(0, 1fr);
  gap: 0;
  padding: 8px 10px 10px;
  align-items: stretch;
}
.workspace-grid.map-surface-expanded .map-editor-shell,
.workspace-grid.map-surface-expanded .map-surface {
  min-height: 0;
  height: 100%;
}
@media (max-width: 1180px) {
  .workspace-grid.map-surface-expanded {
    display: grid;
    grid-template-columns: minmax(0, 1fr);
    padding: 8px 10px 10px;
  }
}
@media (max-width: 760px) {
  .workspace-grid.map-surface-expanded {
    padding: 6px 8px 8px;
  }
}
.date-empty {
  padding: 8px 10px;
  border: 1px dashed var(--theme-warning-border, #d3c0a9);
  border-radius: 7px;
  background: transparent;
  color: var(--accent);
  font-size: 10px;
  cursor: pointer;
}
.date-empty:hover {
  border-color: var(--accent-soft);
  background: var(--surface-muted);
}
.inspector-heading {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 18px 17px 12px;
}
.project-diagnostics {
  display: grid;
  gap: 5px;
  margin: 0 25px 14px;
  padding: 12px 14px;
  border: 1px solid var(--theme-warning-border, #e2b48c);
  border-radius: 9px;
  background: var(--theme-warning-bg, #fff5e9);
  color: var(--theme-warning-text, #765a39);
  font-size: 11px;
}
.editor-header {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  min-height: 58px;
  gap: 16px;
  padding-bottom: 10px;
  border-bottom: 1px solid var(--line);
}
.editor-header h2 {
  margin: 5px 0 0;
  font: 500 22px/1.1 var(--font-display);
}
.editor-title {
  min-width: 0;
  flex: 1;
}
.editor-title-row {
  display: flex;
  align-items: center;
  gap: 8px;
}
.editor-rename-button {
  flex: 0 0 auto;
  width: 30px;
  height: 30px;
  padding: 0;
  display: inline-grid;
  place-items: center;
  margin-top: 3px;
}
.editor-header-controls {
  display: flex;
  flex: 0 0 auto;
  align-items: flex-end;
  flex-direction: column;
  gap: 7px;
}
.document-mode-toggle {
  display: flex;
  gap: 2px;
  padding: 2px;
  border: 1px solid var(--line);
  border-radius: 7px;
  background: var(--canvas);
}
.document-mode-toggle button {
  padding: 5px 9px;
  border: 0;
  border-radius: 5px;
  background: transparent;
  color: var(--ink-faint);
  cursor: pointer;
  font-size: 10px;
  font-weight: 700;
}
.document-mode-toggle button.active {
  background: var(--surface);
  box-shadow: var(--shadow-sm);
  color: var(--ink);
}
.document-mode-toggle button:disabled {
  cursor: not-allowed;
  opacity: 0.45;
}
.entity-edit-dialog {
  width: min(460px, 92vw);
  max-height: 90vh;
  overflow: auto;
}
.entity-edit-warning {
  margin: 12px 0 0;
  padding: 10px 11px;
  border-left: 3px solid var(--theme-warning-border, #d9a46a);
  background: var(--theme-warning-bg, #fff8ee);
  font-size: 12px;
  line-height: 1.45;
  color: var(--ink-soft);
  border-radius: 0 8px 8px 0;
}
.entity-edit-select {
  appearance: none;
  -webkit-appearance: none;
  background-color: var(--canvas);
  background-image: url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='12' height='12' viewBox='0 0 24 24' fill='none' stroke='%2377766d' stroke-width='1.8' stroke-linecap='round' stroke-linejoin='round'%3E%3Cpath d='m6 9 6 6 6-6'/%3E%3C/svg%3E");
  background-repeat: no-repeat;
  background-position: right 11px center;
  padding-right: 32px !important;
  cursor: pointer;
}
.entity-edit-select:disabled {
  opacity: 0.6;
  cursor: not-allowed;
}
.create-input-field .field-hint {
  display: block;
  margin-top: 6px;
  color: var(--ink-faint);
  font-size: 11px;
  line-height: 1.4;
}
.entity-edit-dialog .create-input-field + .create-input-field {
  margin-top: 14px;
}
.dialog .new-form-heading + .dialog-body-copy {
  margin-top: 4px;
}
.conflict-workbench-actions {
  width: min(560px, 100%);
}
.conflict-compare {
  margin: 8px 0 10px;
  padding: 8px 10px;
  border: 1px solid var(--theme-warning-border, #ead7c2);
  border-radius: 7px;
  background: var(--theme-warning-bg, #fffaf3);
}
.conflict-compare summary {
  cursor: pointer;
  font-size: 11px;
  font-weight: 700;
}
.conflict-compare pre {
  max-height: 180px;
  margin: 8px 0 0;
  overflow: auto;
  white-space: pre-wrap;
  font:
    11px/1.5 ui-monospace,
    monospace;
}
.conflict-actions {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
}
.map-editor-shell {
  display: flex;
  min-height: 0;
  flex: 1 1 auto;
  flex-direction: column;
}
.map-surface {
  position: relative;
  display: flex;
  width: 100%;
  min-height: 0;
  flex: 1 1 auto;
}
.map-surface :global(.native-vector-editor),
.map-surface :global(.generator) {
  width: 100%;
  height: 100%;
  min-height: 0;
}
.map-editor-notices {
  display: flex;
  gap: 12px;
  padding: 8px 18px;
  border-bottom: 1px solid var(--line);
  background: var(--surface);
}
.map-conflict-banner {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 16px;
  padding: 12px 18px;
  border-bottom: 1px solid var(--theme-warning-border, #e2b48c);
  background: var(--theme-warning-bg, #fff5e9);
  color: var(--theme-warning-text, #765a39);
}
.map-conflict-copy strong {
  font-size: 12px;
}
.map-conflict-copy p {
  margin: 4px 0 0;
  font-size: 11px;
  line-height: 1.5;
}
.map-conflict-copy code {
  display: block;
  margin-top: 6px;
  font:
    11px ui-monospace,
    monospace;
}
.map-conflict-actions {
  display: flex;
  flex: 0 0 auto;
  align-items: center;
  gap: 6px;
}
.ai-rewrite-modal-backdrop {
  position: fixed;
  z-index: 80;
  inset: 0;
  display: grid;
  place-items: center;
  padding: 24px;
  background: rgba(34, 31, 26, 0.42);
  overscroll-behavior: contain;
}
.ai-rewrite-panel {
  display: grid;
  width: min(760px, 100%);
  max-height: min(82vh, 760px);
  gap: 12px;
  overflow: auto;
  padding: 14px;
  border: 1px solid var(--theme-warning-border, #d8c3a5);
  border-radius: 10px;
  background: var(--warning-bg);
  box-shadow: 0 24px 70px rgba(34, 31, 26, 0.24);
  overscroll-behavior: contain;
}
.ai-rewrite-heading,
.ai-rewrite-actions {
  display: flex;
  align-items: center;
  justify-content: flex-start;
  gap: 10px;
}
.ai-retry-button {
  border-color: var(--theme-warning-border, #c9b486);
  background: var(--theme-warning-bg, #fffaf1);
  color: var(--theme-warning-text, #795a2e);
}
.ai-retry-button:hover {
  border-color: var(--theme-warning-border, #ae8e57);
  background: var(--theme-warning-bg, #fff4df);
  color: var(--theme-warning-text, #63471f);
}
.ai-discard-button {
  border-color: var(--theme-danger-border, #d8b2a8);
  background: var(--theme-danger-bg, #fff8f6);
  color: var(--danger);
}
.ai-discard-button:hover {
  border-color: var(--theme-danger-border, #bd8276);
  background: var(--theme-danger-bg, #fff0ec);
  color: var(--theme-danger-text, #813d32);
}
.ai-rewrite-heading strong {
  display: block;
  margin-top: 4px;
  color: var(--ink);
  font-size: 14px;
}
.ai-instruction {
  display: grid;
  gap: 5px;
  color: var(--ink-soft);
  font-size: 10px;
  font-weight: 700;
}
.ai-instruction textarea {
  width: 100%;
  resize: vertical;
  padding: 9px;
  border: 1px solid var(--line);
  border-radius: 7px;
  background: var(--surface);
  color: var(--ink);
  font: 12px/1.45 var(--font-body);
}
@media (max-width: 620px) {
  .ai-rewrite-modal-backdrop {
    align-items: end;
    padding: 10px;
  }
  .ai-rewrite-panel {
    max-height: 90vh;
  }
}
@media (max-width: 760px) {
  .map-conflict-banner {
    align-items: flex-start;
    flex-direction: column;
  }
}
.editor-footer {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding-top: 14px;
  color: var(--ink-faint);
  font-size: 11px;
}
.editor-footer div {
  display: flex;
  gap: 4px;
}
.empty-mark,
.disabled-icon {
  display: grid;
  place-items: center;
  width: 52px;
  height: 52px;
  border-radius: 16px;
  background: var(--accent-dark);
  color: var(--on-accent);
  font-size: 23px;
}
.inspector-heading-actions {
  display: flex;
  flex-direction: column;
  align-items: flex-end;
  gap: 5px;
}
.inspector-ai-action {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  padding: 3px 6px;
  border: 1px solid var(--theme-warning-border, #d9b98f);
  border-radius: 5px;
  background: var(--warning-bg);
  color: var(--accent);
  font-size: 9px;
  font-weight: 700;
  cursor: pointer;
}
.inspector-ai-action:disabled {
  opacity: 0.65;
  cursor: wait;
}
.inspector-ai-action span {
  font-size: 10px;
}
.inspector-ai-fill {
  padding: 12px 16px;
  border-bottom: 1px solid var(--line);
  background: var(--warning-bg);
}
.inspector-ai-fill-heading {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
}
.inspector-ai-fill-heading strong {
  color: var(--ink);
  font-size: 11px;
}
.inspector-ai-fill p {
  margin: 8px 0 0;
  color: var(--ink-soft);
  font-size: 10px;
  line-height: 1.45;
}
.inspector-ai-suggestion {
  display: grid;
  gap: 8px;
  margin-top: 10px;
  padding-top: 10px;
  border-top: 1px solid var(--theme-warning-border, #ead7c2);
}
.inspector-ai-suggestion strong,
.inspector-ai-suggestion span {
  display: block;
}
.inspector-ai-suggestion strong {
  color: var(--ink-soft);
  font-size: 10px;
}
.inspector-ai-suggestion-heading {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
}
.inspector-ai-suggestion-heading strong {
  min-width: 0;
}
.inspector-ai-suggestion span {
  margin-top: 0;
  color: var(--ink);
  font-size: 11px;
  line-height: 1.45;
}
.inspector-ai-confidence {
  all: unset;
  box-sizing: border-box;
  position: relative;
  width: max-content;
  min-width: 0;
  max-width: 100%;
  height: 20px;
  min-height: 20px;
  max-height: 20px;
  margin-top: 4px;
  display: inline-flex;
  flex: 0 0 auto;
  align-items: center;
  padding: 3px 7px;
  border: 1px solid currentColor;
  border-radius: 999px;
  font: 700 9px/1 var(--font-body);
  font-size: 9px !important;
  line-height: 1 !important;
  white-space: nowrap;
  cursor: help;
}
.confidence-high {
  color: var(--theme-success-text, #46704d) !important;
  background: var(--theme-success-bg, #f1f8f1);
}
.confidence-medium {
  color: var(--theme-warning-text, #9a702f) !important;
  background: var(--theme-warning-bg, #fff8e8);
}
.confidence-low,
.confidence-unknown {
  color: var(--danger) !important;
  background: var(--theme-danger-bg, #fff3f0);
}
.inspector-ai-suggestion .inspector-ai-reasoning {
  position: absolute;
  z-index: 5;
  bottom: calc(100% + 7px);
  right: 0;
  display: none;
  width: min(280px, 70vw);
  height: auto;
  max-height: none;
  margin-top: 0;
  padding: 8px 9px;
  border: 1px solid var(--line-strong);
  border-radius: 7px;
  background: var(--ink);
  color: var(--surface) !important;
  font-size: 10px !important;
  font-weight: 400;
  line-height: 1.45;
  white-space: normal;
  overflow-wrap: anywhere;
  box-shadow: 0 6px 18px rgba(48, 44, 38, 0.18);
}
.inspector-ai-confidence:hover .inspector-ai-reasoning,
.inspector-ai-confidence:focus-visible .inspector-ai-reasoning {
  display: block;
}
.inspector-ai-suggestion > div:last-child {
  display: flex;
  gap: 6px;
}
.inspector-ai-fill-actions {
  display: flex;
  flex-wrap: wrap;
  gap: 7px;
  margin-top: 14px;
  padding-top: 12px;
  border-top: 1px solid var(--theme-warning-border, #ead7c2);
}
.inspector-ai-suggestion .quiet-button:last-child {
  border-color: var(--theme-danger-border, #d8b2a8);
  background: var(--theme-danger-bg, #fff8f6);
  color: var(--danger);
}
.inspector-ai-suggestion .quiet-button:last-child:hover {
  border-color: var(--theme-danger-border, #bd8276);
  background: var(--theme-danger-bg, #fff0ec);
  color: var(--theme-danger-text, #813d32);
}

.date-empty {
  padding: 8px 10px;
  border: 1px dashed var(--theme-warning-border, #d3c0a9);
  border-radius: 7px;
  background: transparent;
  color: var(--accent);
  font-size: 10px;
  cursor: pointer;
}
.date-empty:hover {
  border-color: var(--accent-soft);
  background: var(--surface-muted);
}
.chronology-warning {
  margin: 0;
  color: var(--theme-warning-text, #55351f);
  font-size: 12px;
  line-height: 1.45;
}
.inspector-heading {
  border-bottom: 1px solid var(--line);
}
.inspector-heading strong {
  display: block;
  margin-top: 7px;
  font: 500 20px var(--font-display);
}
.inspector-type {
  padding: 4px 7px;
  border-radius: 5px;
  background: var(--accent-bg);
  color: var(--accent);
  font-size: 9px;
  font-weight: 800;
  text-transform: uppercase;
}
.inspector-section {
  padding: 18px 16px;
  border-bottom: 1px solid var(--line);
}
.inspector-section.inspector-section-plain {
  padding: 9px 0 0;
  border-bottom: 0;
}
.inspector-section.inspector-section-plain + .inspector-section.inspector-section-plain {
  margin-top: 15px;
  padding-top: 15px;
  border-top: 1px solid var(--line);
}
.inspector-group-empty {
  margin: 8px 0 0;
  color: var(--ink-faint);
  font-size: 10px;
  line-height: 1.5;
}
.inspector-section h3,
.section-title h3 {
  margin: 0;
  color: var(--ink-soft);
  font-size: 10px;
  letter-spacing: 0.08em;
  text-transform: uppercase;
}
.property-field {
  display: block;
  margin-top: 14px;
}
.property-field span {
  display: block;
  margin-bottom: 5px;
  color: var(--ink-soft);
  font-size: 10px;
}
.property-field b {
  margin-left: 3px;
  color: var(--accent);
}
.property-field input {
  width: 100%;
  padding: 8px 9px;
  border: 1px solid var(--line);
  border-radius: 7px;
  outline: 0;
  background: var(--canvas);
  color: var(--ink);
  font-size: 11px;
}
.property-field input:focus {
  border-color: var(--accent-soft);
  box-shadow: 0 0 0 3px rgba(180, 119, 63, 0.1);
}
.section-title {
  display: flex;
  align-items: center;
  justify-content: space-between;
}
.section-title span {
  color: var(--ink-faint);
  font-size: 11px;
}
.relationship-detail-list {
  display: grid;
  gap: 6px;
  margin-top: 10px;
}
.relationship-detail-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 10px;
  padding: 8px 9px;
  border: 1px solid var(--line);
  border-radius: 8px;
  background: var(--theme-warning-bg, #fcf8f1);
}
.relationship-detail-copy {
  min-width: 0;
  flex: 1;
  padding: 0;
  border: 0;
  background: transparent;
  text-align: left;
  cursor: pointer;
}
.relationship-detail-copy:hover strong,
.relationship-detail-copy:focus-visible strong {
  color: var(--accent-dark);
  text-decoration: underline;
}
.relationship-detail-copy:focus-visible {
  border-radius: 4px;
  outline: 2px solid color-mix(in srgb, var(--accent-soft) 40%, transparent);
  outline-offset: 3px;
}
.relationship-detail-copy strong,
.relationship-detail-copy small {
  display: block;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.relationship-detail-copy strong {
  color: var(--ink);
  font-size: 11px;
}
.relationship-detail-copy small {
  margin-top: 3px;
  color: var(--ink-faint);
  font-size: 9px;
}
.relationship-detail-actions {
  display: flex;
  align-items: center;
  gap: 6px;
  flex: none;
}
.relationship-details-button {
  flex: none;
  width: 26px;
  height: 26px;
  display: grid;
  place-items: center;
  padding: 0;
  border-radius: 7px;
  line-height: 1;
  color: var(--ink-faint);
}
.relationship-details-button:hover,
.relationship-details-button:focus-visible {
  background: var(--theme-warning-bg, #f0e6d8);
  color: var(--ink);
}
.relationship-details-button :global(svg) {
  width: 14px;
  height: 14px;
}
.relationship-remove-button {
  flex: none;
  width: 26px;
  height: 26px;
  display: grid;
  place-items: center;
  padding: 0;
  border-radius: 7px;
  font-size: 14px;
  line-height: 1;
  color: var(--theme-danger-text, #a1482f);
}
.relationship-remove-button:hover,
.relationship-remove-button:focus-visible {
  background: var(--theme-danger-bg, #f8ece8);
  color: var(--theme-danger-text, #8f3f28);
}
.asset-row strong,
.asset-row small {
  display: block;
}
.asset-row strong {
  font-size: 10px;
}
.asset-row small {
  margin-top: 3px;
  color: var(--ink-faint);
  font-size: 9px;
}
.drop-zone {
  display: flex;
  flex-direction: row;
  align-items: center;
  justify-content: center;
  gap: 10px;
  width: 100%;
  margin-top: 12px;
  padding: 10px 20px;
  border: 1px dashed var(--theme-warning-border, #d3c0a9);
  border-radius: 8px;
  background: var(--theme-warning-bg, #fcf8f1);
  color: var(--accent);
  text-align: center;
  cursor: pointer;
}
.drop-zone span {
  font-size: 16px;
  line-height: 1;
}
.drop-zone strong {
  color: var(--ink-soft);
  font-size: 11px;
  white-space: nowrap;
}
.drop-zone small {
  color: var(--ink-faint);
  font-size: 9px;
  white-space: nowrap;
}
.asset-row {
  display: flex;
  align-items: flex-start;
  flex-wrap: wrap;
  gap: 8px;
  margin-top: 9px;
  padding: 7px;
  border: 1px solid transparent;
  border-radius: 8px;
}
.asset-row-button {
  width: 100%;
  background: transparent;
  text-align: left;
  cursor: pointer;
  font: inherit;
}
.asset-row-button:hover {
  border-color: var(--line);
  background: var(--surface-muted);
}
.asset-row-button:disabled {
  opacity: 0.6;
  cursor: not-allowed;
}
.asset-row-button:focus-visible {
  border-color: var(--accent-soft);
  box-shadow: 0 0 0 3px rgba(180, 119, 63, 0.1);
  outline: none;
}
.asset-row.asset-main {
  border-color: var(--theme-warning-border, #d3c0a9);
  background: var(--theme-warning-bg, #fcf8f1);
}
.asset-details {
  min-width: 0;
  flex: 1;
}
.asset-role {
  display: inline-block;
  margin-top: 5px;
  padding: 2px 5px;
  border-radius: 999px;
  background: var(--theme-warning-bg, #ede2d2);
  color: var(--accent);
  font-size: 8px;
  font-weight: 700;
  text-transform: uppercase;
  letter-spacing: 0.05em;
}
.asset-row-edit-hint {
  align-self: center;
  color: var(--ink-faint);
  font-size: 10px;
  font-weight: 600;
}
.asset-icon {
  display: grid;
  place-items: center;
  width: 25px;
  height: 25px;
  border-radius: 6px;
  background: var(--theme-warning-bg, #ede9e0);
  color: var(--accent);
}
.map-contribution > small {
  display: block;
  color: var(--ink-faint);
  font-size: 10px;
}
.map-location-row {
  display: grid;
  gap: 8px;
  margin-top: 10px;
  padding-top: 10px;
  border-top: 1px solid var(--line);
}
.map-location-row > div:first-child {
  min-width: 0;
}
.map-location-row strong,
.map-location-row small {
  display: block;
  overflow-wrap: anywhere;
}
.map-location-row strong {
  color: var(--ink);
  font-size: 10px;
}
.map-location-row small {
  margin-top: 3px;
  color: var(--ink-faint);
  font-size: 9px;
  line-height: 1.4;
}
.map-location-row > div:last-child {
  display: flex;
  flex-wrap: wrap;
  gap: 3px;
  align-items: center;
}
.map-location-row .quiet-button {
  padding: 5px 6px;
  font-size: 9px;
}
.map-unresolved-badge {
  color: var(--danger);
  font-weight: 700;
}
.map-unresolved-note {
  color: var(--danger);
  font-size: 9px;
}
.empty-workspace-state {
  display: grid;
  min-height: calc(100vh - 58px);
  place-content: center;
  justify-items: center;
  padding: 40px;
  text-align: center;
}
.empty-workspace-state h1 {
  margin: 12px 0 10px;
  font: 500 42px var(--font-display);
}
.empty-workspace-state p {
  max-width: 360px;
  margin: 0 0 24px;
  color: var(--ink-soft);
  font-size: 13px;
  line-height: 1.6;
}
.project-transition-backdrop {
  position: fixed;
  inset: 0;
  z-index: 100;
  display: grid;
  place-items: center;
  background: rgba(37, 37, 31, 0.34);
}
.project-transition-card {
  display: grid;
  justify-items: center;
  gap: 10px;
  min-width: 230px;
  padding: 24px 28px;
  border: 1px solid var(--theme-warning-border, #d8cdbd);
  border-radius: 12px;
  background: var(--surface);
  box-shadow: var(--shadow-lg);
  color: var(--ink);
  text-align: center;
}
.project-transition-card strong {
  font: 500 20px var(--font-display);
}
.project-transition-card small {
  color: var(--ink-soft);
  font-size: 11px;
}
.project-transition-spinner {
  width: 22px;
  height: 22px;
  border: 3px solid var(--theme-warning-border, #eadfce);
  border-top-color: var(--accent);
  border-radius: 50%;
  animation: project-transition-spin 0.8s linear infinite;
}
@keyframes project-transition-spin {
  to {
    transform: rotate(360deg);
  }
}
.toast {
  position: fixed;
  right: 24px;
  bottom: 24px;
  z-index: 60;
  display: flex;
  max-width: 430px;
  align-items: center;
  gap: 8px;
  padding: 13px 14px;
  border: 1px solid var(--theme-warning-border, #e5d4ba);
  border-radius: 9px;
  background: var(--warning-bg);
  box-shadow: var(--shadow-lg);
  color: var(--theme-warning-text, #765a39);
  font-size: 12px;
}
.lifecycle-toast {
  border-color: var(--success-line);
  background: var(--success-bg);
  color: var(--success);
}
.toast button {
  margin-left: 0;
  border: 0;
  background: none;
  color: inherit;
  cursor: pointer;
  font-size: 17px;
}
.toast .toast-action {
  margin-left: auto;
  min-height: 28px;
  padding: 0 10px;
  border: 1px solid currentColor;
  border-radius: 7px;
  font-size: 11px;
  font-weight: 650;
}
@media (max-width: 1180px) {
  .workspace-grid {
    grid-template-columns: 220px minmax(320px, 1fr);
  }
}
@media (max-width: 760px) {
  .studio-shell {
    display: block;
  }
  .welcome {
    min-height: 720px;
    align-items: flex-start;
    padding: 55px 24px;
    background-position: 61% center;
  }
  .welcome::after {
    background: linear-gradient(180deg, rgba(250, 247, 240, 0.96) 0%, rgba(250, 247, 240, 0.78) 36%, transparent 68%);
  }
  .welcome-copy {
    width: 100%;
    max-width: 460px;
  }
  .welcome h1 {
    font-size: 52px;
  }
  .map-marker {
    transform: scale(0.84);
  }
  .marker-atlas {
    top: 46%;
    left: 72%;
  }
  .marker-people {
    top: 57%;
    left: 48%;
  }
  .marker-sword {
    top: 72%;
    left: 18%;
  }
  .marker-tree {
    top: 84%;
    left: 37%;
  }
  .art-card {
    top: auto;
    right: 24px;
    bottom: 48px;
    width: 235px;
    padding: 21px;
  }
  .projection-bar {
    margin: 0 17px 12px;
  }
  .workspace-grid {
    display: flex;
    flex-direction: column;
    padding: 0 17px 25px;
  }
  :global(.module-mount.language-mount) {
    width: 100%;
    min-height: auto;
  }
  .editor-header h2 {
    font-size: 24px;
  }
  .editor-footer {
    align-items: flex-end;
    gap: 10px;
  }
  .toast {
    right: 12px;
    bottom: 12px;
    left: 12px;
  }
}
:global(.projection-bar) {
  min-height: 0;
  padding: 0;
  border: 0;
  background: transparent;
  box-shadow: none;
}
:global(.projection-graph),
:global(.timeline-projection) {
  overflow: hidden;
  border: 1px solid var(--line);
  border-radius: 12px;
  background: var(--surface);
  box-shadow: var(--shadow-sm);
}
:global(.projection-graph svg) {
  display: block;
  width: 100%;
  height: 230px;
  background: linear-gradient(135deg, var(--theme-warning-bg, #fbfaf5), var(--theme-warning-bg, #f5f1e8));
}
:global(.projection-edge) {
  stroke: var(--theme-warning-text, #c9b89f);
  stroke-width: 1.5;
}
:global(.projection-node) {
  fill: var(--surface);
  stroke: var(--accent);
  stroke-width: 2;
}
:global(.projection-node-label) {
  fill: var(--ink);
  font:
    600 11px Inter,
    ui-sans-serif,
    system-ui,
    sans-serif;
}
:global(.projection-node-type) {
  fill: var(--ink-muted);
  font:
    9px Inter,
    ui-sans-serif,
    system-ui,
    sans-serif;
}
.collection-search {
  position: relative;
  display: flex;
  align-items: center;
  gap: 8px;
  min-height: 40px;
  margin: 0 10px 8px;
  padding: 0 10px;
  border: 1px solid var(--theme-warning-border, #ebe7de);
  border-radius: 9px;
  background: var(--surface);
  box-shadow: 0 1px 2px rgba(38, 42, 33, 0.03);
  color: var(--ink-faint);
}
.collection-search span {
  flex: 0 0 auto;
  font-size: 17px;
  line-height: 1;
}
.collection-search input {
  min-width: 0;
  flex: 1;
  border: 0;
  outline: 0;
  background: transparent;
  color: var(--ink);
  font-size: 11px;
}
.filter-toggle {
  appearance: none;
  flex: 0 0 auto;
  display: grid;
  place-items: center;
  width: 26px;
  height: 26px;
  border: 0;
  border-radius: 6px;
  background: transparent;
  color: var(--ink-faint);
  font-size: 14px;
  cursor: pointer;
}
.filter-toggle:hover,
.filter-toggle.active {
  background: var(--theme-warning-bg, #f0ece5);
  color: var(--accent-dark);
}
.filter-popover {
  position: absolute;
  top: calc(100% + 4px);
  left: 0;
  right: 0;
  z-index: 100;
  display: grid;
  gap: 10px;
  padding: 12px;
  border: 1px solid var(--theme-warning-border, #ebe7de);
  border-radius: 9px;
  background: var(--surface);
  box-shadow: var(--shadow-md, 0 4px 12px rgba(38, 42, 33, 0.12));
}
.filter-section {
  display: grid;
  gap: 6px;
  margin: 0;
  padding: 0;
  border: 0;
}
.filter-section legend {
  color: var(--ink-faint);
  font-size: 10px;
  font-weight: 600;
  letter-spacing: 0.04em;
  text-transform: uppercase;
}
.filter-row {
  display: flex;
  flex-wrap: wrap;
  gap: 4px 10px;
  align-items: center;
}
.filter-row label,
.filter-type-list label {
  display: flex;
  align-items: center;
  gap: 4px;
  color: var(--ink);
  font-size: 11px;
  cursor: pointer;
}
.filter-type-list {
  display: grid;
  gap: 4px;
}
.sort-dir-toggle {
  appearance: none;
  display: grid;
  place-items: center;
  width: 24px;
  height: 24px;
  border: 1px solid var(--theme-warning-border, #ebe7de);
  border-radius: 6px;
  background: var(--surface);
  color: var(--ink);
  font-size: 13px;
  cursor: pointer;
}
.sort-dir-toggle:hover {
  background: var(--theme-warning-bg, #f0ece5);
}
.collection-group-header {
  appearance: none;
  display: flex;
  align-items: center;
  gap: 8px;
  width: 100%;
  margin: 4px 0 0;
  padding: 8px 6px;
  border: 0;
  background: transparent;
  color: var(--ink);
  font: inherit;
  cursor: pointer;
}
.collection-group-header:hover {
  background: var(--theme-warning-bg, #f8f5ef);
  border-radius: 6px;
}
.collection-group-header :global(.entity-glyph) {
  flex: 0 0 22px;
}
.collection-group-header strong {
  font-size: 12px;
  font-weight: 600;
  letter-spacing: 0.02em;
  text-transform: uppercase;
}
.collection-group-header small {
  color: var(--ink-faint);
  font-size: 10px;
}
.group-chevron {
  flex: 0 0 auto;
  width: 12px;
  color: var(--ink-faint);
  font-size: 13px;
  line-height: 1;
  text-align: center;
}
.collection-group {
  display: grid;
  align-content: start;
  gap: 6px;
}
.collection-group + .collection-group {
  margin-top: 6px;
}
.collection-group .collection-item {
  margin-left: 6px;
}
.collection-item {
  appearance: none;
  border: 1px solid transparent;
  box-shadow: 0 1px 2px rgba(38, 42, 33, 0.03);
}
.collection-item:hover {
  border-color: var(--theme-warning-border, #e5d8c6);
  box-shadow: var(--shadow-sm);
}
.collection-item.selected {
  border-color: var(--theme-warning-border, #d8c3a5);
  box-shadow:
    inset 3px 0 var(--accent),
    var(--shadow-sm);
}
.collection-loading,
.collection-error {
  margin: 10px 4px;
  padding: 10px 12px;
  color: var(--ink-faint);
  font-size: 11px;
}
.collection-error {
  border: 1px solid var(--theme-danger-border, #ecd7d0);
  border-radius: 8px;
  background: var(--theme-danger-bg, #fbefeb);
  color: var(--theme-danger-text, #9d4938);
}
.collection-pagination {
  display: grid;
  grid-template-columns: auto 1fr auto;
  align-items: center;
  gap: 8px;
  padding: 9px 10px 10px;
  border-top: 1px solid var(--line);
  color: var(--ink-faint);
  font-size: 10px;
  text-align: center;
}
.collection-pagination button {
  padding: 6px 8px;
  border: 1px solid var(--line);
  border-radius: 6px;
  background: var(--surface);
  color: var(--ink-soft);
  font: inherit;
  font-weight: 700;
  cursor: pointer;
}
.collection-pagination button:hover:not(:disabled) {
  border-color: var(--theme-warning-border, #d8c3a5);
  color: var(--ink);
}
.collection-pagination button:disabled {
  opacity: 0.45;
  cursor: default;
}
.collection-item {
  display: flex;
  align-items: center;
  gap: 4px;
  width: 100%;
  height: 58px;
  min-height: 58px;
  max-height: 58px;
  margin: 0;
  padding: 4px 6px 4px 4px;
  overflow: hidden;
  border: 1px solid var(--theme-warning-border, #ebe7de);
  border-radius: 9px;
  background: var(--surface);
  color: var(--ink);
  font: inherit;
  line-height: 1.2;
  text-align: left;
  text-decoration: none;
}
.collection-item-main {
  display: flex;
  min-width: 0;
  flex: 1;
  align-items: center;
  gap: 9px;
  height: 100%;
  padding: 5px 6px;
  border: 0;
  border-radius: 7px;
  background: transparent;
  color: inherit;
  font: inherit;
  text-align: left;
  cursor: pointer;
}
.collection-item-main:focus-visible {
  outline: 3px solid rgba(180, 119, 63, 0.25);
  outline-offset: 0;
}
.collection-item:focus-visible {
  outline: 3px solid rgba(180, 119, 63, 0.25);
  outline-offset: 1px;
}
.collection-item :global(.entity-glyph) {
  flex: 0 0 40px;
}
.collection-item .item-copy {
  display: grid;
  min-width: 0;
  align-content: center;
  gap: 4px;
  overflow: hidden;
}
.collection-item .item-copy strong {
  overflow: hidden;
  color: var(--ink);
  font-size: 13px;
  font-weight: 700;
  line-height: 1.15;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.collection-item .item-copy small {
  width: max-content;
  max-width: 220px;
  margin: 0;
  padding: 3px 6px;
  border-radius: 4px;
  background: var(--theme-warning-bg, #f4f0e8);
  color: var(--ink-faint);
  font-size: 10px;
  line-height: 1;
  letter-spacing: 0.04em;
  text-transform: uppercase;
  white-space: nowrap;
}
.new-form-heading {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 10px;
  margin-bottom: 13px;
}
.new-form-heading strong {
  display: block;
  margin-top: 5px;
  font: 500 19px var(--font-display);
}
.new-form-close {
  border: 0;
  background: transparent;
  color: var(--ink-faint);
  font-size: 20px;
  line-height: 1;
  cursor: pointer;
}
.new-form-actions {
  display: flex;
  justify-content: flex-end;
  gap: 7px;
  margin-top: 14px;
}
.new-form-actions .quiet-button {
  padding: 9px 10px;
}
.modal-backdrop {
  position: fixed;
  inset: 0;
  z-index: 20;
  display: grid;
  place-items: center;
  padding: 20px;
  background: rgba(37, 37, 31, 0.28);
  backdrop-filter: blur(4px);
}
.plugin-confirm-modal {
  z-index: 30;
}
.dialog {
  width: min(440px, 100%);
  margin: 0;
  padding: 22px;
  border: 1px solid var(--theme-warning-border, #e3d9ca);
  border-radius: 14px;
  background: var(--surface);
  box-shadow: 0 22px 70px rgba(37, 37, 31, 0.2);
}
.dialog .new-form-heading {
  margin-bottom: 18px;
}
.dialog .new-form-heading strong {
  font-size: 23px;
}
.dialog .new-form-close {
  width: 30px;
  height: 30px;
  border-radius: 7px;
  background: var(--surface-muted);
  color: var(--ink-soft);
}
.dialog .new-form-close:hover {
  background: var(--theme-warning-bg, #ebe6dd);
  color: var(--ink);
}
.capability-list {
  display: grid;
  gap: 6px;
  max-height: min(240px, 35vh);
  margin: 2px 0 12px;
  padding: 10px;
  overflow-y: auto;
  border: 1px solid var(--line-strong);
  border-radius: 8px;
  background: var(--canvas);
  color: var(--ink-soft);
}
.capability-item {
  padding: 6px 8px;
  border-bottom: 1px solid rgba(217, 205, 189, 0.65);
  font-size: 11px;
  line-height: 1.4;
}
.capability-item:last-child {
  border-bottom: 0;
}
.capability-empty {
  margin-top: -2px;
}
.dialog .new-form-actions {
  margin-top: 20px;
}
.plugins-list .search-state {
  margin: 0;
  padding: 28px 16px;
  color: var(--ink-faint);
  font-size: 11px;
  text-align: center;
}
:global(html) {
  background: var(--canvas);
}
:global(body) {
  min-width: 320px;
  text-rendering: optimizeLegibility;
}
:global(body.modal-open) {
  overflow: hidden;
}
:global(button),
:global(input),
:global(select) {
  -webkit-tap-highlight-color: transparent;
}
:global(button:focus-visible),
:global(input:focus-visible),
:global(select:focus-visible) {
  outline: 3px solid rgba(180, 119, 63, 0.28);
  outline-offset: 2px;
}
.primary-button:active {
  transform: translateY(1px);
}
.editor-header > div:first-child,
.inspector-heading > div {
  min-width: 0;
}
.editor-header h2 {
  overflow-wrap: anywhere;
}
.collection-search,
.property-field input {
  transition:
    border-color 0.16s ease,
    box-shadow 0.16s ease;
}
.collection-search:focus-within {
  border-color: var(--accent-soft);
  box-shadow: 0 0 0 3px rgba(180, 119, 63, 0.1);
}
.primary-button,
.quiet-button,
.add-button,
.new-form-close {
  transition:
    background 0.16s ease,
    color 0.16s ease,
    opacity 0.16s ease,
    transform 0.16s ease;
}
.dialog {
  max-height: min(680px, calc(100vh - 32px));
  overflow-y: auto;
}
@media (max-width: 1040px) {
  .projection-bar {
    margin-inline: 28px;
  }
  .workbench-layout-controls {
    margin-inline: 28px;
  }
  .workspace-grid {
    grid-template-columns: 215px minmax(280px, 1fr);
    padding-inline: 28px;
  }
}

@media (max-width: 760px) {
  :global(body) {
    overflow-x: hidden;
  }
  .welcome {
    min-height: 680px;
    padding-top: 42px;
  }
  .welcome h1 {
    font-size: clamp(43px, 13vw, 56px);
  }
  .welcome p {
    font-size: 14px;
  }
  .projection-bar {
    margin: 0 17px 12px;
  }
  .workbench-layout-controls {
    justify-content: flex-start;
    margin: 0 17px 10px;
    overflow-x: auto;
  }
  .workspace-grid {
    gap: 12px;
    padding: 0 17px 25px;
  }
  .editor-header {
    min-height: 62px;
    gap: 10px;
  }
  .editor-header-controls {
    align-items: flex-end;
  }
  .editor-footer {
    align-items: flex-start;
    flex-wrap: wrap;
  }
  .editor-footer > div {
    width: 100%;
    justify-content: flex-end;
  }

  .modal-backdrop {
    padding: 12px;
  }
  .dialog {
    padding: 18px;
    border-radius: 12px;
  }
  .toast {
    right: 12px;
    bottom: 12px;
    left: 12px;
    max-width: none;
  }
}

@media (max-width: 430px) {
  .welcome {
    min-height: 650px;
    padding: 38px 18px;
  }
  .welcome h1 {
    font-size: clamp(40px, 12vw, 50px);
  }
  .welcome p {
    max-width: 34ch;
  }
  .marker-sword {
    left: 8%;
  }
  .marker-tree {
    left: 31%;
  }
  .art-card {
    right: 18px;
    bottom: 34px;
    width: 218px;
    padding: 18px;
  }
  .art-card strong {
    margin: 13px 0 20px;
    font-size: 18px;
  }
  .editor-footer > div {
    flex-direction: column-reverse;
  }
  .editor-footer > div .quiet-button {
    width: 100%;
    text-align: center;
  }
}

.create-dialog {
  display: flex;
  flex-direction: column;
  width: min(960px, 100%);
  max-height: min(780px, calc(100vh - 32px));
  padding: 0;
  overflow: hidden;
}
.create-dialog-form {
  display: contents;
}
.create-dialog-heading {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 20px;
  padding: 24px 26px 20px;
  border-bottom: 1px solid var(--line);
}
.create-dialog-heading strong {
  display: block;
  margin-top: 6px;
  font: 500 27px/1.05 var(--font-display);
}
.create-dialog-heading p {
  max-width: 600px;
  margin: 9px 0 0;
  color: var(--ink-soft);
  font-size: 12px;
  line-height: 1.5;
}
.create-dialog-back {
  display: block;
  margin: 0 0 9px;
  padding: 0;
  border: 0;
  background: transparent;
  color: var(--accent-dark);
  cursor: pointer;
  font-size: 10px;
  font-weight: 750;
}
.create-dialog-back::before {
  content: "←";
  margin-right: 6px;
}
.create-template-gallery {
  min-height: 360px;
  padding: 24px 26px 30px;
  overflow-y: auto;
  background: var(--canvas);
}
.create-template-group {
  margin-top: 26px;
}
.create-template-group:first-child {
  margin-top: 0;
}
.create-template-group-heading {
  display: flex;
  align-items: baseline;
  justify-content: space-between;
  gap: 12px;
  margin-bottom: 10px;
}
.create-template-group-heading span {
  color: var(--ink-soft);
  font-size: 10px;
  font-weight: 800;
  letter-spacing: 0.13em;
  text-transform: uppercase;
}
.create-template-group-heading small {
  color: var(--ink-faint);
  font-size: 9px;
}
.create-template-tiles {
  display: grid;
  grid-template-columns: repeat(3, minmax(0, 1fr));
  gap: 10px;
}
.create-template-card {
  display: grid;
  min-height: 132px;
  grid-template-columns: 40px minmax(0, 1fr) auto;
  grid-template-rows: auto auto;
  align-content: center;
  gap: 6px 10px;
  width: 100%;
  padding: 15px;
  border: 1px solid var(--line);
  border-radius: 11px;
  background: var(--surface);
  color: var(--ink);
  text-align: left;
  cursor: pointer;
  box-shadow: var(--shadow-sm);
  transition:
    border-color 0.16s ease,
    box-shadow 0.16s ease,
    transform 0.16s ease;
}
.create-template-card:hover,
.create-template-card:focus-visible {
  border-color: var(--accent-soft);
  box-shadow: var(--shadow-md, 0 4px 12px rgba(38, 42, 33, 0.12));
  transform: translateY(-1px);
}
.create-template-card :global(.entity-glyph) {
  grid-row: 1;
  grid-column: 1;
  align-self: center;
}
.create-template-copy {
  grid-row: 1;
  grid-column: 2;
  min-width: 0;
  align-self: center;
}
.create-template-copy strong {
  display: block;
  overflow: hidden;
  text-overflow: ellipsis;
  font-size: 13px;
}
.create-template-detail {
  grid-row: 2;
  grid-column: 1 / 3;
  min-width: 0;
}
.create-template-detail small {
  display: -webkit-box;
  color: var(--ink-faint);
  font-size: 10px;
  line-height: 1.45;
  white-space: normal;
  -webkit-box-orient: vertical;
  -webkit-line-clamp: 2;
  line-clamp: 2;
  overflow: hidden;
}
.create-template-arrow {
  grid-row: 1 / span 2;
  grid-column: 3;
  align-self: center;
  justify-self: end;
  color: var(--ink-faint);
  font-size: 22px;
  line-height: 1;
}
.create-form-panel {
  width: min(620px, 100%);
  min-width: 0;
  margin: 0 auto;
  overflow-y: auto;
  padding: 8px 28px 30px;
}
.create-input-field {
  display: block;
  margin-top: 17px;
}
.create-input-field > span,
.create-input-field > label > span {
  display: block;
  margin-bottom: 6px;
  color: var(--ink-soft);
  font-size: 10px;
  font-weight: 700;
}
.create-input-field b {
  margin-left: 3px;
  color: var(--accent);
}
.create-input-field > input,
.create-input-field > textarea,
.create-input-field > select,
.create-input-field > label + input,
.create-input-field > label + textarea,
.create-input-field > label + select {
  width: 100%;
  padding: 10px 11px;
  border: 1px solid var(--line-strong);
  border-radius: 8px;
  outline: 0;
  background: var(--canvas);
  color: var(--ink);
  font-size: 12px;
}
.create-input-field > textarea {
  min-height: 78px;
  resize: vertical;
  line-height: 1.5;
}
.create-input-field > input:focus,
.create-input-field > textarea:focus,
.create-input-field > select:focus {
  border-color: var(--accent-soft);
  box-shadow: 0 0 0 3px rgba(180, 119, 63, 0.1);
}
.create-input-field > label + .create-checkbox {
  display: flex;
}
.create-checkbox {
  align-items: center;
  gap: 8px;
  min-height: 38px;
  color: var(--ink-soft);
  font-size: 12px;
}
.create-checkbox input {
  width: 16px;
  height: 16px;
  accent-color: var(--accent-dark);
}
.create-more-details-toggle {
  display: flex;
  width: 100%;
  align-items: center;
  justify-content: space-between;
  gap: 16px;
  margin-top: 22px;
  padding: 12px 13px;
  border: 1px solid var(--line);
  border-radius: 9px;
  background: var(--surface-muted);
  color: var(--ink-soft);
  text-align: left;
  cursor: pointer;
}
.create-more-details-toggle:hover {
  border-color: var(--line-strong);
  background: var(--surface-quiet);
}
.create-more-details-toggle strong,
.create-more-details-toggle small {
  display: block;
}
.create-more-details-toggle strong {
  color: var(--ink);
  font-size: 11px;
}
.create-more-details-toggle small {
  margin-top: 3px;
  color: var(--ink-faint);
  font-size: 9px;
}
.create-more-details-toggle :global(svg) {
  flex: 0 0 auto;
  transition: transform 0.16s ease;
}
.create-more-details-toggle :global(svg.expanded) {
  transform: rotate(180deg);
}
.create-more-details {
  padding: 1px 2px 4px;
}
.create-form-empty {
  display: grid;
  min-height: 300px;
  place-items: center;
  color: var(--ink-faint);
  font-size: 12px;
}
.create-dialog-actions {
  display: flex;
  justify-content: flex-end;
  gap: 8px;
  padding: 15px 26px;
  border-top: 1px solid var(--line-strong);
  background: var(--surface-quiet);
  box-shadow: 0 -6px 18px color-mix(in srgb, var(--canvas) 18%, transparent);
}
.discard-backdrop {
  position: fixed;
  inset: 0;
  z-index: 30;
  display: grid;
  place-items: center;
  padding: 20px;
  background: rgba(37, 37, 31, 0.28);
}
.discard-dialog {
  width: min(390px, 100%);
  padding: 24px;
  border: 1px solid var(--theme-warning-border, #e3d9ca);
  border-radius: 14px;
  background: var(--surface);
  box-shadow: 0 22px 70px rgba(37, 37, 31, 0.2);
}
.discard-dialog h2 {
  margin: 8px 0 7px;
  font: 500 23px/1.1 var(--font-display);
}
.discard-dialog p {
  margin: 0;
  color: var(--ink-soft);
  font-size: 12px;
  line-height: 1.55;
}
.discard-actions {
  display: flex;
  justify-content: flex-end;
  gap: 7px;
  margin-top: 20px;
}
.capture-dialog {
  width: min(420px, 100%);
  padding: 22px;
}
.capture-preview {
  display: grid;
  gap: 4px;
  margin: 14px 0 2px;
  padding: 10px 12px;
  border: 1px dashed var(--theme-warning-border, #d3c0a9);
  border-radius: 8px;
  background: var(--theme-warning-bg, #fcf8f1);
}
.capture-preview-label {
  color: var(--ink-faint);
  font-size: 9px;
  font-weight: 700;
  letter-spacing: 0.1em;
  text-transform: uppercase;
}
.capture-mode {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 6px;
  margin-top: 17px;
  padding: 4px;
  border: 1px solid var(--line-strong);
  border-radius: 9px;
  background: var(--canvas);
}
.map-reconcile-notice {
  min-width: 0;
  overflow: hidden;
  color: var(--theme-warning-text, #8a6a3b);
  font-size: 11px;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.map-unresolved-badge {
  display: inline-block;
  padding: 1px 6px;
  border-radius: 5px;
  background: var(--theme-danger-bg, #f7e6dd);
  color: var(--danger);
  font-weight: 700;
}
.map-unresolved-note {
  color: var(--danger);
  font-size: 10px;
  font-weight: 700;
  white-space: nowrap;
}
.mobile-create-button {
  display: none;
}

@media (max-width: 760px) {
  .mobile-create-button {
    position: fixed;
    right: 18px;
    bottom: 18px;
    z-index: 15;
    display: grid;
    place-items: center;
    width: 52px;
    height: 52px;
    border: 1px solid rgba(213, 171, 108, 0.7);
    border-radius: 50%;
    background: #d5ab6c;
    color: var(--brass-ink);
    box-shadow: 0 10px 24px rgba(37, 37, 31, 0.2);
    font-size: 26px;
    line-height: 1;
    cursor: pointer;
  }
  .mobile-create-button:hover {
    background: #e1bc82;
  }
  .create-dialog {
    max-height: calc(100vh - 24px);
  }
  .create-dialog-heading {
    padding: 19px 18px 16px;
  }
  .create-dialog-heading strong {
    font-size: 23px;
  }
  .create-template-gallery {
    min-height: 0;
    padding: 18px;
  }
  .create-template-group {
    margin-top: 20px;
  }
  .create-template-tiles {
    grid-template-columns: repeat(2, minmax(0, 1fr));
  }
  .create-template-card {
    min-height: 116px;
    padding: 12px;
  }
  .create-template-icon {
    width: 34px;
    height: 34px;
    border-radius: 8px;
  }
  .create-form-panel {
    padding: 4px 18px 22px;
  }
  .create-dialog-actions {
    padding: 13px 18px;
  }
  .create-dialog-actions .primary-button {
    max-width: 70%;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
}

@media (max-width: 470px) {
  .create-template-tiles {
    grid-template-columns: 1fr;
  }
  .create-template-card {
    min-height: 106px;
  }
}

.host-view-back {
  margin: 24px 40px 0;
}
.list-empty {
  display: grid;
  align-content: center;
  justify-items: start;
  min-height: 100%;
  padding: 36px 18px 42px;
  text-align: left;
}
.list-empty .empty-mark {
  display: grid;
  place-items: center;
  width: 42px;
  height: 42px;
  margin-bottom: 18px;
  border-radius: 13px;
  background: var(--accent-dark);
  color: var(--on-accent);
  font-size: 19px;
}
.list-empty strong {
  max-width: 22ch;
  color: var(--ink);
  font: 500 23px/1.08 var(--font-display);
}
.list-empty p {
  max-width: 28ch;
  margin: 10px 0 18px;
  color: var(--ink-soft);
  font-size: 12px;
  line-height: 1.5;
}
.empty-create {
  padding: 9px 11px;
  border: 1px solid var(--theme-warning-border, #d8c3a5);
  border-radius: 8px;
  background: var(--accent-bg);
  color: var(--accent-dark);
  font-size: 11px;
  font-weight: 700;
  cursor: pointer;
}
.empty-create-actions {
  position: relative;
  display: grid;
  gap: 8px;
  width: min(220px, 100%);
}
.empty-create-actions .empty-create {
  width: 100%;
}
.map-provider-create {
  position: relative;
}
.map-provider-menu {
  position: absolute;
  z-index: 20;
  top: calc(100% + 6px);
  right: 0;
  display: grid;
  min-width: 220px;
  padding: 5px;
  border: 1px solid var(--line);
  border-radius: 8px;
  background: var(--surface);
  box-shadow: var(--shadow-lg);
}
.map-provider-menu button {
  border: 0;
  border-radius: 5px;
  padding: 8px 9px;
  background: transparent;
  color: var(--ink);
  text-align: left;
  font: inherit;
  white-space: nowrap;
  cursor: pointer;
}
.map-provider-menu button:hover,
.map-provider-menu button:focus-visible {
  background: var(--surface-muted);
}
.map-provider-menu button:disabled {
  opacity: 0.4;
  cursor: not-allowed;
}
.map-provider-menu button:disabled:hover {
  background: transparent;
}
.map-provider-row {
  display: flex;
  align-items: center;
  gap: 8px;
}
.map-provider-row [role="menuitem"] {
  flex: 1;
  display: flex;
  align-items: center;
}
.map-help {
  flex: 0 0 24px;
  width: 24px;
  height: 24px;
  display: flex;
  align-items: center;
  justify-content: center;
  border: 1px solid rgba(255, 255, 255, 0.18);
  border-radius: 50%;
  background: rgba(255, 255, 255, 0.08);
  color: #f4f1ea;
  font:
    700 12px/1 Inter,
    ui-sans-serif,
    system-ui,
    sans-serif;
  line-height: 1;
  text-align: center;
  cursor: help;
}
.map-help:hover,
.map-help:focus-visible {
  border-color: rgba(255, 255, 255, 0.28);
  background: rgba(255, 255, 255, 0.12);
  color: #ffffff;
}
.map-help-wrapper {
  position: relative;
  display: flex;
  align-items: center;
}
.map-help-tooltip {
  position: absolute;
  top: calc(100% + 8px);
  right: 0;
  width: 240px;
  padding: 8px 10px;
  border-radius: 6px;
  background: var(--ink);
  color: var(--surface);
  font: 400 11px/1.4 var(--font-body, system-ui, sans-serif);
  box-shadow: 0 8px 20px rgba(38, 42, 33, 0.18);
  opacity: 0;
  visibility: hidden;
  transform: translateY(4px);
  transition:
    opacity 0.15s ease,
    transform 0.15s ease,
    visibility 0.15s;
  z-index: 3;
  pointer-events: none;
}
.map-help-tooltip::after {
  content: "";
  position: absolute;
  top: -5px;
  right: 6px;
  width: 10px;
  height: 10px;
  background: var(--ink);
  transform: rotate(45deg);
}
.map-help-wrapper:hover .map-help-tooltip,
.map-help:focus-visible + .map-help-tooltip,
.map-help:focus + .map-help-tooltip {
  opacity: 1;
  visibility: visible;
  transform: translateY(0);
  pointer-events: auto;
}
.empty-map-provider-menu {
  top: calc(100% + 6px);
  right: auto;
  left: 0;
}
.empty-create:hover {
  background: var(--warning-line);
}
@media (max-width: 760px) {
  .list-empty {
    min-height: 260px;
    padding: 28px 14px 32px;
  }
  .list-empty strong {
    font-size: 21px;
  }
}

.plugins-toolbar {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 0 0 12px;
}
.muted-note {
  color: var(--ink-faint);
  font-size: 10px;
}
.plugins-note {
  margin: 0 0 10px;
  padding: 9px 12px;
  border: 1px solid var(--theme-success-border, #d9e6db);
  border-radius: 8px;
  background: var(--theme-success-bg, #f2f8f3);
  color: var(--theme-success-text, #3f6b4c);
  font-size: 11px;
}
.plugins-list {
  min-height: 200px;
  max-height: min(620px, calc(100vh - 260px));
  overflow-y: auto;
  padding: 4px 0 8px;
}
.panel-hero {
  display: grid;
  grid-template-columns: 40px 1fr;
  gap: 14px;
  padding: 16px 16px 14px;
  border: 1px solid var(--line);
  border-radius: 14px;
  background: var(--surface);
}
.panel-hero .hero-icon {
  width: 40px;
  height: 40px;
  display: grid;
  place-items: center;
  border-radius: 11px;
  background: var(--accent-dark);
  color: var(--on-accent);
}
.hero-copy .kicker {
  color: var(--accent);
  font:
    700 10px/1 Inter,
    ui-sans-serif,
    system-ui,
    sans-serif;
  letter-spacing: 0.08em;
  text-transform: uppercase;
}
.hero-copy strong {
  display: block;
  margin-top: 3px;
  color: var(--ink);
  font: 600 16px/1.15 var(--font-display, Georgia, serif);
}
.hero-copy p {
  margin: 6px 0 0;
  max-width: 640px;
  color: var(--ink-soft);
  font:
    400 12.5px/1.5 Inter,
    ui-sans-serif,
    system-ui,
    sans-serif;
}
.hero-stats {
  grid-column: 2;
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
  margin-top: 2px;
}
.stat-pill {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  padding: 5px 9px;
  border-radius: 999px;
  background: var(--surface-warm);
  border: 1px solid var(--line-soft);
  color: var(--ink-muted);
  font:
    600 11px Inter,
    ui-sans-serif,
    system-ui,
    sans-serif;
}
.plugin-card {
  padding: 18px;
  border: 1px solid var(--line);
  border-radius: 14px;
  background: var(--surface);
  box-shadow:
    0 1px 0 rgba(48, 44, 38, 0.03),
    0 8px 24px rgba(48, 44, 38, 0.04);
  transition:
    border-color 0.14s ease,
    box-shadow 0.14s ease,
    transform 0.14s ease;
}
.plugin-card:hover {
  border-color: var(--theme-warning-border, #e0d6c4);
  box-shadow: 0 8px 24px rgba(48, 44, 38, 0.08);
  transform: translateY(-1px);
}
.plugin-card + .plugin-card {
  margin-top: 14px;
}
.plugin-card-head {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 12px;
}
.plugin-card-title {
  min-width: 0;
}
.plugin-card-title strong {
  display: block;
  font: 500 17px/1.2 var(--font-display);
}
.plugin-id {
  display: block;
  margin-top: 3px;
  color: var(--ink-faint);
  font-size: 10px;
}
.plugin-badges {
  display: flex;
  flex-wrap: wrap;
  justify-content: flex-end;
  gap: 5px;
}
.plugin-badge {
  padding: 3px 7px;
  border-radius: 5px;
  background: var(--theme-warning-bg, #f0ece5);
  color: var(--ink-soft);
  font-size: 9px;
  font-weight: 800;
  text-transform: uppercase;
  letter-spacing: 0.05em;
}
.plugin-badge.badge-off {
  background: var(--theme-warning-bg, #efe9dd);
  color: var(--ink-faint);
}
.plugin-badge.beta {
  background: var(--theme-warning-bg, #f7ead3);
  color: var(--theme-warning-text, #936525);
}
.plugin-badge.experimental {
  background: var(--theme-danger-bg, #f5e0da);
  color: var(--theme-danger-text, #a1482f);
}
.plugin-badge.danger {
  background: var(--theme-danger-bg, #f5e0da);
  color: var(--theme-danger-text, #a1482f);
}
.plugin-card-meta {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 6px 13px;
  margin-top: 10px;
  color: var(--ink-faint);
  font-size: 10px;
}
.runtime-dot {
  color: var(--theme-success-text, #6fa276);
  font-size: 10px;
}
.runtime-dot.runtime-off {
  color: var(--theme-warning-text, #c0b7a8);
}
.plugin-warning {
  margin: 10px 0 0;
  padding: 8px 10px;
  border: 1px solid var(--theme-warning-border, #ecd9bb);
  border-radius: 7px;
  background: var(--theme-warning-bg, #fcf5ea);
  color: var(--warning);
  font-size: 11px;
  line-height: 1.45;
}
.plugin-error {
  margin: 8px 0 0;
  color: var(--theme-danger-text, #a1482f);
  font-size: 11px;
  line-height: 1.45;
}
.plugin-muted {
  margin: 8px 0 0;
  color: var(--ink-faint);
  font-size: 10px;
}
.plugin-actions {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 4px;
  margin-top: 11px;
}
.plugin-actions .quiet-button {
  padding: 7px 10px;
  font-size: 11px;
}
.plugin-toggle {
  padding: 8px 16px;
  border: 1px solid var(--accent);
  border-radius: 8px;
  background: var(--accent);
  color: var(--on-bright-accent);
  font-size: 11px;
  font-weight: 700;
  cursor: pointer;
  box-shadow: 0 4px 10px rgba(180, 119, 63, 0.18);
  transition:
    background 0.16s ease,
    transform 0.16s ease;
}
.plugin-toggle:hover {
  background: #a86b37;
}
.plugin-toggle.on {
  background: var(--surface);
  border-color: var(--theme-warning-border, #d8c3a5);
  color: var(--ink-soft);
  box-shadow: none;
}
.plugin-toggle.on:hover {
  background: var(--surface-muted);
  color: var(--ink);
}
.plugin-toggle:disabled {
  opacity: 0.55;
  cursor: wait;
}
.plugin-toggle:active {
  transform: translateY(1px);
}
.version-list {
  margin-top: 12px;
  border: 1px solid var(--line);
  border-radius: 9px;
  overflow: hidden;
}
.version-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 10px;
  padding: 9px 11px;
  background: var(--surface);
}
.version-row + .version-row {
  border-top: 1px solid var(--line);
}
.version-copy {
  min-width: 0;
}
.version-name {
  display: flex;
  align-items: center;
  flex-wrap: wrap;
  gap: 5px;
  color: var(--ink);
  font-size: 12px;
  font-weight: 700;
}
.version-detail {
  display: block;
  margin-top: 3px;
  color: var(--ink-faint);
  font-size: 10px;
}
.version-tag {
  padding: 2px 6px;
  border-radius: 4px;
  background: var(--theme-warning-bg, #f0ece5);
  color: var(--ink-soft);
  font-size: 8px;
  font-weight: 800;
  text-transform: uppercase;
  letter-spacing: 0.05em;
}
.version-tag.latest {
  background: var(--theme-success-bg, #e4efdf);
  color: var(--theme-success-text, #3f6b4c);
}
.version-tag.selected {
  background: var(--accent-bg);
  color: var(--accent);
}
.version-tag.bundled {
  background: var(--theme-info-bg, #e8e4ee);
  color: var(--theme-info-text, #6a5b8a);
}
.version-tag.signed {
  background: var(--theme-success-bg, #e4efdf);
  color: var(--theme-success-text, #3f6b4c);
}
.version-tag.unsigned {
  background: var(--theme-danger-bg, #f5e0da);
  color: var(--theme-danger-text, #a1482f);
}
.version-actions {
  display: flex;
  flex-wrap: wrap;
  justify-content: flex-end;
  gap: 3px;
}
.version-actions .quiet-button {
  padding: 6px 8px;
  font-size: 10px;
}
.plugin-details {
  margin-top: 12px;
}
.plugin-details summary {
  color: var(--ink-soft);
  font-size: 11px;
  font-weight: 700;
  cursor: pointer;
}
.plugin-details-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(200px, 1fr));
  gap: 10px;
  margin-top: 11px;
}
.plugin-detail-section {
  padding: 11px 12px;
  border: 1px solid var(--line);
  border-radius: 8px;
  background: var(--canvas);
}
.plugin-detail-section h4 {
  margin: 0 0 7px;
  color: var(--ink-soft);
  font-size: 9px;
  letter-spacing: 0.1em;
  text-transform: uppercase;
}
.plugin-detail-list {
  display: grid;
  gap: 4px;
  margin: 0;
  padding: 0;
  list-style: none;
}
.plugin-detail-list li {
  color: var(--ink-soft);
  font-size: 11px;
  line-height: 1.35;
}
.plugin-detail-list li.capability-row {
  display: flex;
  align-items: flex-start;
  gap: 8px;
  min-width: 0;
}
.capability-name {
  min-width: 0;
  overflow-wrap: anywhere;
}
.plugin-detail-list li.granted {
  color: var(--ink);
}
.plugin-detail-list li.provides {
  color: var(--theme-success-text, #3f6b4c);
}
.plugin-detail-list li.consumes {
  color: var(--warning);
}
.plugin-detail-list li.muted-item {
  color: var(--ink-faint);
}
.plugin-detail-list code,
.dialog-body-copy code,
.backup-path {
  color: var(--accent-dark);
  font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
  font-size: 0.95em;
}
.cap-mark {
  flex: 0 0 auto;
  margin-left: auto;
  color: var(--ink-faint);
}
li.granted .cap-mark {
  color: var(--theme-success-text, #6fa276);
}
.dialog-body-copy {
  margin: 0 0 12px;
  color: var(--ink-soft);
  font-size: 12px;
  line-height: 1.55;
}
.plugin-subhead {
  margin: 14px 0 7px;
  color: var(--ink-soft);
  font-size: 10px;
  letter-spacing: 0.08em;
  text-transform: uppercase;
}
.backup-path {
  display: block;
  margin: 4px 0 2px;
  overflow-wrap: anywhere;
  font-size: 11px;
}
.danger-button {
  background: #a1482f;
  box-shadow: none;
}
.danger-button:hover {
  background: #8f3f28;
}

@media (max-width: 600px) {
  .version-row {
    align-items: flex-start;
    flex-direction: column;
  }
  .version-actions {
    justify-content: flex-start;
  }
  .plugin-card-head {
    flex-direction: column;
  }
  .plugin-badges {
    justify-content: flex-start;
  }
}
</style>
