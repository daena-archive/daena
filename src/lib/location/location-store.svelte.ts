/**
 * Everything the shell keeps per location: which entry is open, its document, field drafts, revisions, dirty
 * tracking, the autosave timer and the save queue, and the conflict state; and the collection beside it: its query,
 * fetched page, open groups, manuscript outline, scroll positions and pane layout, with the effects that reload and
 * fetch them, and which surface the location shows (section, views, wiki, projection or plugin surface, home,
 * settings) with the functions that switch, capture and restore it, the Writing and Language webview bridges, the map
 * editor session, and the selection AI dialogs. The effects run in the store's own
 * `$effect.root` and key off `section`.
 *
 * `+page.svelte` builds exactly one store today (side-by-side will build one per visible group). The store never
 * imports the page. Shell-global state (the project, the entity cache, modules, dialogs, toasts) and page functions
 * that have not moved yet reach it through the `LocationContext` the page passes in.
 */
import { tick, untrack } from "svelte";
import type {
  AiStreamEvent,
  Asset,
  Entity,
  EntityPage,
  MapLocation,
  PluginAdminEntry,
  ProjectInfo,
  Relationship,
  project,
} from "$lib/project/client";
import { aiActionRequest, instructionFor, type PromptTemplate } from "$lib/ai/promptTemplates";
import { validateAiProposal } from "$lib/ai/proposal";
import { reduceTextGenerationEvent } from "$lib/ai/stream";
import { aiJsonValueSchema, coerceAiFieldValue } from "$lib/fields/values";
import type { FieldDefinition, ModuleContext, ModuleManifest, UUID } from "../../../packages/module-api/src/index";
import type { CalendarDefinition } from "../../../packages/modules/timeline/src/calendar";
import type { confirmDialog } from "$lib/dialogs.svelte";
import type { EntityFieldsTabId } from "$lib/shell/entityFields";
import type { EraContext } from "$lib/modules/chronology";
import {
  sameShellLocation,
  type ShellLocation,
  type WorkspaceCollectionLocation,
  type WorkspaceLocationView,
  type WorkspacePaneDimensions,
} from "$lib/navigation/history";
import {
  collectionEntityTypes,
  collectionTabForEntityType,
  DEFAULT_COLLECTION_QUERY,
  type CollectionQuery,
  type CollectionResult,
  type LanguagePane,
  type ProjectSection,
  type SettingsSection,
  type SortDirection,
  type SortField,
  type TimelineView,
  type WorkspaceCollectionTab,
  type WorkspaceSection,
  type WritingView,
} from "$lib/modules/workspace";
import type { NavigationRenderer } from "$lib/modules/workspace-presentation";
import {
  LANGUAGE_PLUGIN_ID,
  LanguageLeaveTracker,
  isLanguagePane,
  openableLinkHref,
  type LanguageStateEvent,
} from "$lib/modules/language-bridge";
import {
  WRITING_PLUGIN_ID,
  createWritingFlushBroker,
  writingAiStateMessage,
  writingConflictMessage,
  writingFullscreenMessage,
  writingLoadMessage,
  writingMessageCurrent,
  writingTitleMessage,
  writingWebviewHandles,
  type WritingAiStateSource,
  type WritingBridgeDraft,
  type WritingBridgeShortcut,
  type WritingLoad,
  type WritingStateEvent,
} from "$lib/modules/writing-bridge";
import type { TabShortcut } from "$lib/navigation/tab-shortcuts";
import type { WebviewInstanceRef } from "$lib/plugins/webview-pool";
import { PROJECTION_SANDBOXES, type ProjectionSandboxKey } from "$lib/location/projection-sandboxes";
import {
  collectionPaneDefault,
  collectionPaneMax,
  collectionPaneMin,
  inspectorPaneDefault,
  inspectorPaneMax,
  inspectorPaneMin,
  loadCollectionQuery,
  loadWorkbenchLayout,
  saveWorkbenchLayout,
  type WorkbenchPane,
} from "$lib/shell/workbench-persistence";
import {
  appearanceLabel,
  buildManuscriptOutline,
  collectOutlineIds,
  collectPartOfEdges,
  containmentNames,
  defaultExpandedOutlineIds,
  MANUSCRIPT_OUTLINE_CAP,
  outlinePathIds,
  parentByChild,
  partOfEdgesFromRelationships,
  WRITING_FEATURES,
  WRITING_PART_OF,
  type OutlineNode,
} from "$lib/writing/outline";
import { houseMemberSummaries } from "$lib/houses/fetch.ts";
import { HOUSE_TYPE, type HouseMemberSummary, type TreeSession } from "$lib/houses/model.ts";
import { GREGORIAN_CALENDAR_ID, isGregorianCalendarId, parseCalendarDate, serializeCalendarDate } from "$lib/date";
import {
  hasMeaningfulDateValue,
  isEmptyFieldValue,
  isSentinelDateValue,
  isStructuredFieldValue,
  restoreStructuredFieldValue,
  shouldPersistFieldValue,
} from "$lib/fields/persistence";
import { fieldDisplayValue, fieldRevisionKey, fieldValueForSave } from "$lib/fields/values";
import { mimeTypeFor } from "$lib/assets/mime";
import { normalizeDocument } from "$lib/markdown/normalize";
import { friendlyError } from "$lib/shell/errors";
import { chronologyWarnings, firstEraCalendarId, isEraRelationshipField } from "$lib/modules/chronology";
import * as eraContextCache from "$lib/chronology/eraContextCache";
import {
  counterpartId,
  defaultRelationshipMetadata,
  endpointsForCreate,
  relationshipsForField,
} from "$lib/modules/contributed-fields";
import { toHostRelationship } from "$lib/modules/host-relationship";
import { schemaEntityTypeIds } from "$lib/modules/workspace-presentation";
import { primarySchemaNamespace } from "$lib/schema-workbench/model";
import { classifyMutationError } from "$lib/houses/mutations";
import { isKinshipRelationshipType } from "$lib/houses/model";
import { createMutationController } from "$lib/entity-lifecycle/mutationState";
import type { MutationSnapshot } from "$lib/entity-lifecycle/mutationState";

/** A plugin view the sidebar offers: a host surface, a sandboxed webview or the maps editor. */
export type PluginNavigationItem = {
  kind: "plugin";
  plugin: PluginAdminEntry;
  view: PluginAdminEntry["views"][number];
  key: string;
  renderer: Exclude<NavigationRenderer, "workspace">;
};

type ProjectClient = typeof project;
type ConfirmOptions = Parameters<typeof confirmDialog>[0];

type MapPickPending =
  | { kind: "link"; entityId: string; role: string; mapEntityId: string }
  | { kind: "rebind"; entityId: string; location: MapLocation; mapEntityId: string };

type AiFieldSuggestion = { value: unknown; rationale: string; confidence: string };

/**
 * What the store needs from the page. Reads are getters, so the store always sees the page's current value;
 * callbacks reach page functions that have not moved into the store yet.
 */
export interface LocationContext {
  /** The project client. Given here, not imported, so the store can run outside Tauri. */
  readonly project: ProjectClient;
  /** The shell's entity cache. */
  readonly entities: Entity[];
  /** Non-empty while the project has diagnostics; blocks saving and edits. */
  readonly projectDiagnostics: string[];
  readonly calendarDefinitions: Record<string, CalendarDefinition>;
  /** True once a project has finished opening. */
  readonly ready: boolean;
  readonly projectInfo: ProjectInfo | null;
  /** The installed plugins the admin view lists, or null before they are loaded. */
  readonly adminPlugins: PluginAdminEntry[] | null;
  /** How many modules are installed. The Writing and Timeline tab clamp waits until there are some. */
  readonly moduleCount: number;
  /** Every workspace section, in navigation order. */
  readonly workspaceSectionOrder: WorkspaceSection[];

  confirmDialog(options: ConfirmOptions): Promise<boolean>;
  /** The shell's single error line. */
  setError(message: string): void;
  /** Clears the shell-global state that follows the selection (create dialog era contexts, diagnostics, create form). */
  resetSelectionShellState(): void;

  // Shell navigation around a leave. The Language and Writing leave guards live on the store.
  recordShellDeparture(location: ShellLocation): void;
  /** The tab and history work around a restore: `restoreShellLocation` marks it so no departure is recorded. */
  setShellNavigationRestoring(restoring: boolean): void;
  /** Closes every native plugin webview. Best effort. */
  closeNativePluginWebviews(): Promise<void>;
  /** Closes the settings or project surface; false if the administration view refuses to leave. */
  dismissSettings(): Promise<boolean>;
  openSettings(section: SettingsSection): Promise<void>;
  openProjectCenter(section: ProjectSection): Promise<void>;
  /** Clears the shell's global search box. */
  clearGlobalQuery(): void;
  refreshAdmin(): Promise<void>;
  pluginNavigationItemByKey(key: string): PluginNavigationItem | null;
  /** Adds an entity the shell has not cached yet. */
  appendEntityToCache(entity: Entity): void;

  // Module and schema lookups for the location's section.
  contextFor(section?: WorkspaceSection): ModuleContext;
  contextOwningRelationshipType(relationshipType: string): ModuleContext;
  activeManifest(): ModuleManifest | null;
  activeModuleId(): string;
  definitions(): FieldDefinition[];
  chronologyDateDefinitions(): FieldDefinition[];
  namespaceForField(definition: FieldDefinition): string;
  sectionEnabled(): boolean;
  mapsEnabled(): boolean;
  /** Entity types the section lists, and the active tab's own types where the section has tabs. */
  sectionEntityTypes(): string[];
  activeTabEntityTypes(): string[] | undefined;
  /** The module manifest the section is installed from, if the module is installed. */
  manifestForWorkspaceSection(section: WorkspaceSection): ModuleManifest | null;
  /** The collection tabs the section shows (Writing and Timeline have several). */
  collectionTabsFor(section: WorkspaceSection): WorkspaceCollectionTab[];
  /** The workspace sections whose modules are enabled. */
  enabledWorkspaceSections(): WorkspaceSection[];
  /** Every entity type the section's schemas declare. */
  allEntityTypesForSection(section: WorkspaceSection): Set<string>;
  /** The section whose module owns an entity type. */
  sectionForEntityType(entityType: string | null): WorkspaceSection | null;
  /** The visible collection page as the list shows it. */
  collectionResult(): CollectionResult;
  /** The module context the Houses collection summaries read through. */
  housesModuleContext(): ModuleContext;

  // Shell refreshes.
  upsertEntityInCache(entity: Entity): void;
  refreshAfterEntityMutation(options?: { entityId?: string; removed?: boolean }): Promise<void>;
  loadEraContexts(eraIds: string[]): Promise<EraContext[]>;
  notifyProfileTimelineChanged(eventId: string): void;
  loadProjectStatus(includeGit: boolean): Promise<void>;

  // What the Writing and Language bridges read from the shell. The values mix location and shell state, so they
  // stay on the page; the store reads them at call time.
  /** Tab id of the webview on screen. Message defaults read this when the call runs, not when the method was built. */
  readonly shownWebviewInstance: string;
  /** The Writing webview is the content column's editor. */
  readonly writingWebviewShown: boolean;
  /** The AI provider. The Writing bridge reads `name`; the selection dialog also reads `endpoint` and `model`. */
  readonly aiProvider: { readonly name: string; readonly endpoint: string; readonly model: string };
  /** The editor prompt templates the Writing webview is told about. */
  readonly aiPromptTemplates: WritingAiStateSource["templates"];

  /** Marks the shown instance ready in the shell-global webview pool. */
  markShownWebviewReady(pluginId: string): void;
  /** Opens a link a Writing or Language webview named. */
  openExternal(href: string): void;
  /** Opens an entity a Writing or Language webview named, in this tab or a new one. */
  openEntityFromWebview(entityId: string, newTab: boolean): void;
  /** Runs a shell shortcut a Writing or Language webview forwarded. */
  runShellShortcut(shortcut: WritingBridgeShortcut | TabShortcut): void;
  /** Opens the rename dialog for the entry the Writing webview named. */
  openEntityEditDialog(entity: Entity): void;
  /** Confirms and archives the entry the Writing webview named. */
  confirmWritingArchive(entityId: string): void;

  /** The Maps plugin view, for opening the editor. */
  mapsNavigationItem(): PluginNavigationItem | null;
  /** Empty inspector fields the selection AI may fill. The template reads this too, so it stays on the page. */
  emptyInspectorDefinitions(): FieldDefinition[];
  /** Relationship targets already chosen for a definition. */
  selectedRelationshipIds(definition: FieldDefinition): string[];
  /**
   * Subscribes to `ai-stream:<requestId>`. The page unwraps the Tauri event and passes the payload.
   * Resolves to the unlisten function.
   */
  listenAiStream(requestId: string, handler: (payload: AiStreamEvent) => void): Promise<() => void>;
  /** The mounted native vector map session, or null when no editor has registered one. */
  nativeVectorSession(): { save: () => Promise<void>; isDirty: () => boolean } | null;
}

export function isRevisionConflict(cause: unknown) {
  return friendlyError(cause).toLowerCase().includes("revision conflict");
}

export const emptyEntityPage = (): EntityPage => ({
  items: [],
  total: 0,
  offset: 0,
  limit: DEFAULT_COLLECTION_QUERY.pageSize,
  has_more: false,
  type_counts: [],
});

function measuredPaneWidth(element: HTMLElement | null, fallback: number) {
  const width = element?.getBoundingClientRect().width ?? 0;
  return width > 0 ? Math.round(width) : fallback;
}

/** The scroll-restore key a stored location points at, or null when its view has no full surface. */
export function specializedSurfaceKeyForLocation(location: ShellLocation): string | null {
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

export class LocationStore {
  readonly ctx: LocationContext;
  private readonly stopEffects: () => void;

  // Which surface is open: the workspace section and its views, the home page, settings, or a plugin surface.
  projectHomeOpen = $state(true);
  section = $state<WorkspaceSection>("lore");
  writingView = $state<WritingView>("manuscripts");
  timelineView = $state<TimelineView>("events");
  loreGuideHint = $state(0);
  timelineGuideHint = $state(0);
  languagePane = $state<LanguagePane>("overview");
  housesView = $state<WorkspaceLocationView>("houses");
  showSettings = $state(false);
  settingsSurface = $state<"application" | "project">("application");
  settingsSection = $state<SettingsSection>("general");
  projectSection = $state<ProjectSection>("overview");
  hostView = $state<{ plugin: PluginAdminEntry; view: PluginAdminEntry["views"][number] } | null>(null);
  sandboxView = $state<{
    plugin: PluginAdminEntry;
    view: PluginAdminEntry["views"][number] | null;
    renderer: "maps" | "webview";
  } | null>(null);
  treeRootId = $state<string | null>(null);
  treeSession = $state<TreeSession | null>(null);
  treeRestoreNonce = $state(0);
  loreWikiOpen = $state(false);
  loreWikiEntityId = $state<string | null>(null);
  /** Plain: set once the Language section has refreshed its list, so the effect does it once per visit. */
  languageListRefreshed = false;

  // Selection and load.
  selected = $state<Entity | null>(null);
  selectedLoading = $state(false);
  selectedLoadError = $state("");
  /** Stale-load token: a load whose token is no longer current does nothing. Plain on purpose. */
  selectedLoadToken = 0;

  // Document.
  documentBody = $state("");
  documentMode = $state<"read" | "edit">("edit");
  fields = $state<Record<string, unknown>>({});
  relationships = $state<Relationship[]>([]);
  eraContexts = $state<EraContext[]>([]);
  assets = $state<Asset[]>([]);
  assetBusyId = $state<string | null>(null);
  mapLocations = $state<MapLocation[]>([]);
  dateEditorOpen = $state<Record<string, boolean>>({});
  dateCalendarByField = $state<Record<string, string>>({});

  // Dialogs tied to the selection.
  metadataDialog = $state<{ relationship: Relationship; definition: FieldDefinition | null } | null>(null);
  assetDialog = $state<Asset | null>(null);
  profileEditorOpen = $state(false);
  fieldsDialogOpen = $state(false);
  fieldsDialogTab = $state<EntityFieldsTabId>("relationships");

  // Editor.
  editorFullscreen = $state(false);
  editorRef = $state<{
    insertAiTextAtRequest: (value: string) => string | null;
    replaceAiTextWithMarkdown: (value: string) => string | null;
    flushPendingChanges: () => void;
  } | null>(null);

  // Saving.
  isSaving = $state(false);
  savedAt = $state("");
  saveError = $state("");
  hasUnsavedChanges = $state(false);
  autoSaveTimer: number | null = null;
  dateFieldDirtyTimers: Record<string, number> = {};
  saveInFlight: Promise<boolean> | null = null;
  saveQueued = false;
  autoSaveFailureCount = 0;
  /** Local edit generation, bumped on every edit. Not the stored revision. */
  documentRevision = 0;
  /**
   * Inspector definition keys with unsaved edits; persistDocumentSnapshot saves only these (plus the document).
   * Keys match definition.key, unique per selected entity's field set.
   */
  readonly dirtyFieldKeys = new Set<string>();
  loadedDocumentRevision = "";
  loadedFieldRevisions: Record<string, string> = {};
  loadedStructuredFieldKeys = new Set<string>();
  documentConflict = $state<{ paths: string[]; diagnostics: string[] } | null>(null);
  conflictDiskBody = $state("");

  // Entity mutations (rename, archive) and their "Saved" chrome.
  entityMutationSnapshot = $state<MutationSnapshot>({ phase: "idle", message: "", detail: "" });
  readonly entityMutation = createMutationController({
    get: () => this.entityMutationSnapshot,
    set: (next) => {
      this.entityMutationSnapshot = next;
    },
  });
  savedMutationTimer: ReturnType<typeof setTimeout> | null = null;

  // Collection: the query, the page it fetched, and the groups the list shows.
  collectionQuery = $state<CollectionQuery>(loadCollectionQuery("lore"));
  collectionPage = $state<EntityPage>(emptyEntityPage());
  collectionLoading = $state(false);
  collectionError = $state("");
  /** Stale-response token for the collection fetch. Plain on purpose. */
  collectionRequest = 0;
  /** Bumped to re-query the visible collection page without reloading every entity. */
  collectionRefreshEpoch = $state(0);
  filterOpen = $state(false);
  expandedGroups = $state<Set<string>>(new Set());
  /** Plain: set while a stored query is being applied, so the scope effect does not reset the page. */
  collectionQueryRestoring = false;
  /** Plain: the last scope the scope effect saw. */
  collectionScopeKey = "";

  // Houses collection.
  /** Bumped when kinship relationships or tree-relevant entities change outside TreeSurface. */
  kinshipRefreshEpoch = $state(0);
  houseCollectionSummaries = $state(new Map<string, HouseMemberSummary>());
  houseSummaryRequest = 0;
  houseSummariesPending = $state(false);

  // Manuscript outline (Writing collection).
  manuscriptOutline = $state<OutlineNode[]>([]);
  manuscriptOutlineEntities = $state<Entity[]>([]);
  manuscriptOutlineRequest = 0;
  manuscriptOutlineLoading = $state(false);
  manuscriptOutlineTruncated = $state(false);
  manuscriptOutlineLibraryKey = "";
  manuscriptExpanded = $state(new Set<string>());
  manuscriptPathLabels = $state<Record<string, string>>({});
  pendingManuscriptParentId = $state<string | null>(null);
  writingAppearanceLabels = $state<Record<string, string>>({});

  // Panes and scroll.
  collectionListElement = $state<HTMLDivElement | null>(null);
  collectionPaneElement = $state<HTMLElement | null>(null);
  contentPaneElement = $state<HTMLElement | null>(null);
  inspectorPaneElement = $state<HTMLElement | null>(null);
  collectionScrollBySection = $state<Partial<Record<WorkspaceSection, number>>>({});
  pendingCollectionScroll = $state<{ section: WorkspaceSection; scrollTop: number } | null>(null);
  restoredWorkspacePaneDimensions = $state<WorkspacePaneDimensions | null>(null);
  private readonly initialWorkbenchLayout = loadWorkbenchLayout("lore");
  workbenchPaneVisibility = $state<Record<WorkbenchPane, boolean>>(this.initialWorkbenchLayout.visibility);
  workbenchPaneWidths = $state({
    collection: this.initialWorkbenchLayout.collectionWidth,
    inspector: this.initialWorkbenchLayout.inspectorWidth,
  });
  specializedSurfaceElement = $state<HTMLElement | null>(null);
  specializedSurfaceScrollByKey = $state<Record<string, number>>({});
  pendingSpecializedSurfaceScroll = $state<{ key: string; scrollTop: number } | null>(null);

  readonly inspectorChronologyWarnings = $derived.by(() =>
    chronologyWarnings(
      this.ctx.chronologyDateDefinitions().map((definition) => ({
        label: definition.label,
        value: this.fields[definition.key],
      })),
      this.eraContexts,
      this.ctx.calendarDefinitions,
    ),
  );

  /**
   * A manuscript or reference page is selected in the Writing section. The Writing webview is the only editor for it:
   * the shell editor never renders for such an entry, even in states where the webview is not shown (see
   * `writingWebviewShown` on the page), so a second editor can never take typing next to the webview's drafts.
   */
  readonly writingEntrySelected = $derived(
    this.section === "writing" && writingWebviewHandles(this.selected?.entity_type),
  );
  readonly mapSurfaceOpen = $derived(
    this.section === "maps" && this.sandboxView?.renderer === "maps" && Boolean(this.sandboxView?.view),
  );

  // Writing and Language bridges. The shell-global pool stays on the page; this is the per-location half.
  /** Mount state of the Language webview: `mounted` while its content pane element exists, `ready` after it reports ready. */
  languageWebview = $state({ mounted: false, ready: false });
  /** Can-leave requests to the Language webview, resolved by its leave-pending/leave-reply/ready messages. */
  readonly languageLeave = new LanguageLeaveTracker();
  /** A pane the shell asked the mounted Language webview to show and has not yet seen confirmed. */
  languagePaneRequest: LanguagePane | null = null;
  /** Mount state of the Writing webview: `mounted` while its content pane element exists, `ready` after it reports ready. */
  writingWebview = $state({ mounted: false, ready: false });
  /** Counts the webview's `ready` reports, so the AI state is posted again to a webview that restarted. */
  writingReadyCount = $state(0);
  /**
   * The Writing webview's full screen, as the webview reports it. Kept apart from `editorFullscreen`: that one hides
   * every embedded webview, and this one needs the webview visible so it can fill the window.
   */
  writingFullscreen = $state(false);
  /** The instance that reported `writingFullscreen`, so ending it reaches that instance even after the tab changed. */
  writingFullscreenInstance: string | null = null;
  /** The Writing instance whose host mount finished. The visibility effect shows no other Writing instance. */
  writingHostedInstance = $state<string | null>(null);
  /** The same for Language: the instance whose host mount finished. */
  languageHostedInstance = $state<string | null>(null);
  /** Whether the mounting Language instance was ready before its mount began. See `beginPooledMount` on the page. */
  languageMountPriorReady = false;
  /** What the Language mount in progress keeps, so only those the host actually closed leave the pool. */
  languageMountRetained: WebviewInstanceRef[] = [];
  /**
   * `{#key}` creates the next attachment before it destroys the previous one
   * (`BranchManager.ensure` mounts the new branch, then `#commit` destroys the
   * old). A stale cleanup must not clear state the new instance just set.
   */
  languageMountGeneration = 0;
  /** See `languageMountGeneration`. Same `{#key}` setup-before-cleanup order. */
  writingMountGeneration = 0;
  /** Whether the mounting instance was ready before its mount began. See `beginPooledWritingMount` on the page. */
  writingMountPriorReady = false;
  /** What the mount in progress keeps, so only those the host actually closed leave the pool. */
  writingMountRetained: WebviewInstanceRef[] = [];
  /**
   * What the last `writing.load` was for (entity id and the shell's load counter), so an unchanged selection is
   * not reposted. Saving does not change it; a new entity or a reload from disk does.
   */
  writingLoadKey: string | null = null;
  /** The `writing.load` the shell last posted. Only its entity and token are accepted on drafts and conflict actions. */
  writingLoad: WritingLoad | null = null;
  /** Asks the mounted Writing webview for text it has not published yet. */
  readonly writingFlush = createWritingFlushBroker({
    post: (message) => this.postWritingMessage(message),
    newRequestId: () => crypto.randomUUID(),
    onTimeout: (requestId) =>
      console.warn(`The Writing webview did not answer flush ${requestId}; continuing without its unsent text.`),
  });
  /** Drafts are applied one at a time, in arrival order, so a slow chunked draft cannot land after a newer one. */
  writingDraftQueue: Promise<unknown> = Promise.resolve();
  /** The last `writing.editable` value the webview was told, so only changes are posted. */
  writingEditablePosted: boolean | null = null;

  // Map editor session. The project-wide saved-map list stays on the page.
  mapsEditorMode = $state<"vector" | "physical">("vector");
  mapsVectorStart = $state<"import" | "geojson">("geojson");
  mapProviderMenuOpen = $state<"header" | "empty" | null>(null);
  mapsEditorKey = $state("welcome");
  mapReloadCounter = $state(0);
  mapSaveStates = $state<Record<string, { status: string; detail: unknown }>>({});
  mapRecoveryBusy = $state(false);
  mapFocusLinkId = $state<string | null>(null);
  mapFocusFeatureId = $state<string | null>(null);
  mapSelection = $state<unknown | null>(null);
  mapPickPending = $state<MapPickPending | null>(null);
  mapPickNotice = $state("");
  mapReconcileNotice = $state("");

  // Selection AI. Settings AI (provider, models, index) stays on the page.
  aiRewriteOpen = $state(false);
  aiBusy = $state(false);
  aiCancelPending = $state(false);
  aiMode = $state<"rewrite" | "generate">("rewrite");
  aiRequestId = $state<string | null>(null);
  aiInstruction = $state("Rewrite this to be more vivid while preserving the meaning.");
  aiStreamText = $state("");
  aiPreviewOutput = $state("");
  aiProgressMessage = $state("Preparing model…");
  aiSourceSelection = $state("");
  aiSourceSelectionPlain = $state("");
  aiGenerationContext = $state("");
  aiSourceBody = $state("");
  aiSourceRevision = $state("");
  aiLastSequence = $state(-1);
  /** Plain: the unlisten for `ai-stream:<requestId>`. Not reactive on purpose. */
  aiUnlisten: (() => void) | null = null;
  aiStartCancelled = $state(false);
  aiSourceEntityId = $state<string | null>(null);
  aiUsage = $state<{ inputTokens: number; outputTokens: number; totalTokens: number } | null>(null);
  aiFieldFillBusy = $state(false);
  aiFieldFillOpen = $state(false);
  aiFieldFillRequestId = $state<string | null>(null);
  aiFieldFillStream = $state("");
  aiFieldSuggestions = $state<Record<string, AiFieldSuggestion>>({});
  /** Plain: the unlisten for the field-fill stream. Not reactive on purpose. */
  aiFieldUnlisten: (() => void) | null = null;
  aiFieldFillStartCancelled = $state(false);
  aiFieldFillEntityId = $state<string | null>(null);
  aiFieldFillLastSequence = $state(-1);

  /**
   * Whether the Writing webview may edit: not while an entry loads or fails to load, diagnostics block saving,
   * AI works or a conflict waits.
   */
  readonly writingEditable = $derived.by(
    () =>
      !this.selectedLoading &&
      !this.selectedLoadError &&
      this.ctx.projectDiagnostics.length === 0 &&
      !this.aiBusy &&
      !this.aiRewriteOpen &&
      !this.documentConflict,
  );

  constructor(ctx: LocationContext) {
    this.ctx = ctx;
    // Effects belong in here, not in the page's init scope: step 5.4 creates and destroys stores at runtime.
    // They key off the location's own `section`.
    this.stopEffects = $effect.root(() => {
      // Persists the collection query under its section's key.
      $effect(() => {
        const q = this.collectionQuery;
        try {
          const serializable = { ...q, excludedTypes: [...q.excludedTypes] };
          localStorage.setItem(`daena:collection-query:${q.section}`, JSON.stringify(serializable));
        } catch {}
      });

      // Reloads the stored query for the section whenever it changes.
      $effect(() => {
        this.collectionQueryRestoring = true;
        this.collectionQuery = loadCollectionQuery(this.section);
        this.expandedGroups = new Set();
        void tick().then(() => {
          this.collectionQueryRestoring = false;
        });
      });

      // Reloads the section's pane layout whenever the section changes.
      $effect(() => {
        const layout = loadWorkbenchLayout(this.section);
        this.workbenchPaneVisibility = layout.visibility;
        this.workbenchPaneWidths = {
          collection: layout.collectionWidth,
          inspector: layout.inspectorWidth,
        };
        this.restoredWorkspacePaneDimensions = null;
      });

      // Resets the page when the collection's scope (section, types, search, sort, size) changes.
      $effect(() => {
        const entityTypes = collectionEntityTypes({
          entityTypes: new Set(this.ctx.sectionEntityTypes()),
          tabEntityTypes: this.ctx.activeTabEntityTypes(),
        });
        const scopeKey = JSON.stringify([
          this.section,
          entityTypes,
          this.collectionQuery.textSearch.trim(),
          this.collectionQuery.sortField,
          this.collectionQuery.sortDir,
          this.collectionQuery.pageSize,
          this.collectionQuery.excludedTypes,
        ]);
        if (this.collectionScopeKey && this.collectionScopeKey !== scopeKey && !this.collectionQueryRestoring) {
          this.collectionPage = emptyEntityPage();
          if (this.collectionQuery.page !== 0) this.collectionQuery.page = 0;
        }
        this.collectionScopeKey = scopeKey;
      });

      // Fetches the visible collection page.
      $effect(() => {
        void this.collectionRefreshEpoch;
        const active = this.ctx.ready && Boolean(this.ctx.projectInfo);
        const page = this.collectionQuery.page;
        const limit = this.collectionQuery.pageSize;
        const entityTypes = collectionEntityTypes({
          entityTypes: new Set(this.ctx.sectionEntityTypes()),
          tabEntityTypes: this.ctx.activeTabEntityTypes(),
        });
        const query = this.collectionQuery.textSearch.trim();
        const excludedEntityTypes = [...this.collectionQuery.excludedTypes];
        const sortField = this.collectionQuery.sortField;
        const sortDirection = this.collectionQuery.sortDir;
        const request = ++this.collectionRequest;
        this.collectionError = "";
        const structureOutline =
          this.section === "writing" &&
          this.writingView === "manuscripts" &&
          this.collectionQuery.viewMode === "grouped" &&
          !this.collectionQuery.textSearch.trim();
        if (!active) {
          this.collectionPage = emptyEntityPage();
          this.collectionLoading = false;
          return;
        }
        if (structureOutline) {
          this.collectionLoading = false;
          return;
        }
        this.collectionLoading = true;
        const timer = window.setTimeout(
          () => {
            void this.ctx.project
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
                if (request !== this.collectionRequest) return;
                if (page > 0 && result.items.length === 0 && result.total > 0) {
                  this.collectionQuery.page = Math.max(0, Math.ceil(result.total / limit) - 1);
                  return;
                }
                this.collectionPage = result;
                for (const entity of result.items) this.ctx.upsertEntityInCache(entity);
              })
              .catch((cause) => {
                if (request !== this.collectionRequest) return;
                this.collectionPage = emptyEntityPage();
                this.collectionError = friendlyError(cause);
              })
              .finally(() => {
                if (request === this.collectionRequest) this.collectionLoading = false;
              });
          },
          query ? 180 : 0,
        );
        return () => window.clearTimeout(timer);
      });

      // Refreshes the Language list once per visit and opens its first entry when nothing is selected.
      $effect(() => {
        if (!this.ctx.ready || this.section !== "language") {
          this.languageListRefreshed = false;
          return;
        }
        if (this.languageListRefreshed) return;
        this.languageListRefreshed = true;
        this.bumpCollectionRefresh();
        if (!this.selected && this.ctx.collectionResult().entities.length > 0)
          void this.selectEntity(this.ctx.collectionResult().entities[0], false);
      });

      // Keeps the Timeline and Writing tabs on a tab that exists.
      $effect(() => {
        if (!this.ctx.moduleCount) return;
        const timelineTabs = this.ctx.collectionTabsFor("timeline");
        if (timelineTabs.length > 0 && !timelineTabs.some((tab) => tab.id === this.timelineView)) {
          this.timelineView = timelineTabs[0].id;
        }
        const writingTabs = this.ctx.collectionTabsFor("writing");
        if (writingTabs.length > 0 && !writingTabs.some((tab) => tab.id === this.writingView)) {
          this.writingView = writingTabs[0].id;
        }
      });

      // Loads the member summaries of the Houses collection page.
      $effect(() => {
        const housesSection = this.ctx.ready && this.section === "houses" && this.housesView === "houses";
        const ids = this.collectionPage.items
          .filter((entity) => {
            const type = entity.entity_type ?? "";
            return type === HOUSE_TYPE || type.endsWith(":house");
          })
          .map((entity) => entity.id);
        void this.collectionRefreshEpoch;
        if (!housesSection || ids.length === 0) {
          if (!housesSection) this.houseCollectionSummaries = new Map();
          this.houseSummariesPending = false;
          return;
        }
        const request = ++this.houseSummaryRequest;
        this.houseSummariesPending = true;
        const context = this.ctx.housesModuleContext();
        void houseMemberSummaries(context, ids)
          .then((summaries) => {
            if (request !== this.houseSummaryRequest) return;
            this.houseCollectionSummaries = summaries;
          })
          .catch(() => {
            if (request !== this.houseSummaryRequest) return;
            this.houseCollectionSummaries = new Map();
          })
          .finally(() => {
            if (request === this.houseSummaryRequest) this.houseSummariesPending = false;
          });
      });

      // Loads the Writing manuscript outline, or the containment labels of search results.
      $effect(() => {
        const onManuscripts =
          this.ctx.ready &&
          Boolean(this.ctx.projectInfo) &&
          this.section === "writing" &&
          this.writingView === "manuscripts";
        const search = this.collectionQuery.textSearch.trim();
        const grouped = this.collectionQuery.viewMode === "grouped";
        const entityTypes = collectionEntityTypes({
          entityTypes: new Set(this.ctx.sectionEntityTypes()),
          tabEntityTypes: this.ctx.activeTabEntityTypes(),
        });
        const excludedEntityTypes = [...this.collectionQuery.excludedTypes];
        const sortField = this.collectionQuery.sortField;
        const sortDirection = this.collectionQuery.sortDir;
        const pageItems = this.collectionPage.items;
        void this.collectionRefreshEpoch;
        if (!onManuscripts) {
          if (this.section !== "writing" || this.writingView !== "manuscripts") {
            this.manuscriptOutline = [];
            this.manuscriptOutlineEntities = [];
            this.manuscriptOutlineTruncated = false;
            this.manuscriptOutlineLibraryKey = "";
            this.manuscriptPathLabels = {};
            this.manuscriptOutlineLoading = false;
          }
          return;
        }
        const request = ++this.manuscriptOutlineRequest;
        if (search) {
          this.manuscriptOutline = [];
          this.manuscriptOutlineTruncated = false;
          this.manuscriptOutlineLoading = false;
          const seed = pageItems.map((entity) => entity.id);
          if (seed.length === 0) {
            this.manuscriptPathLabels = {};
            return;
          }
          void (async () => {
            try {
              const labels = await this.manuscriptContainmentLabels(seed);
              if (request !== this.manuscriptOutlineRequest) return;
              this.manuscriptPathLabels = labels;
            } catch {
              if (request !== this.manuscriptOutlineRequest) return;
              this.manuscriptPathLabels = {};
            }
          })();
          return;
        }
        this.manuscriptPathLabels = {};
        if (!grouped) {
          this.manuscriptOutline = [];
          this.manuscriptOutlineEntities = [];
          this.manuscriptOutlineTruncated = false;
          this.manuscriptOutlineLoading = false;
          return;
        }
        if (entityTypes.length === 0) {
          this.manuscriptOutline = [];
          this.manuscriptOutlineEntities = [];
          this.manuscriptOutlineTruncated = false;
          this.manuscriptOutlineLoading = false;
          return;
        }
        this.manuscriptOutlineLoading = true;
        void (async () => {
          try {
            const { items, truncated } = await this.listManuscriptOutlineSummaries(
              entityTypes,
              excludedEntityTypes,
              sortField,
              sortDirection,
            );
            if (request !== this.manuscriptOutlineRequest) return;
            const edges = await collectPartOfEdges(
              items.map((entity) => entity.id),
              this.queryWritingPartOf,
            );
            if (request !== this.manuscriptOutlineRequest) return;
            const roots = buildManuscriptOutline(items, [], edges);
            const libraryKey = roots.map((node) => node.id).join("\0");
            if (libraryKey !== this.manuscriptOutlineLibraryKey) {
              this.manuscriptOutlineLibraryKey = libraryKey;
              this.manuscriptExpanded = defaultExpandedOutlineIds(roots);
            } else {
              const allowed = collectOutlineIds(roots);
              this.manuscriptExpanded = new Set([...this.manuscriptExpanded].filter((id) => allowed.has(id)));
            }
            const lastPage = Math.max(0, Math.ceil(roots.length / Math.max(1, this.collectionQuery.pageSize)) - 1);
            if (this.collectionQuery.page > lastPage) this.collectionQuery.page = lastPage;
            this.manuscriptOutlineEntities = items;
            this.manuscriptOutlineTruncated = truncated;
            this.manuscriptOutline = roots;
            for (const entity of items) this.ctx.upsertEntityInCache(entity);
          } catch {
            if (request !== this.manuscriptOutlineRequest) return;
            this.manuscriptOutline = [];
            this.manuscriptOutlineEntities = [];
            this.manuscriptOutlineTruncated = false;
          } finally {
            if (request === this.manuscriptOutlineRequest) this.manuscriptOutlineLoading = false;
          }
        })();
      });

      // Opens the ancestors of the selected entry in the manuscript outline.
      $effect(() => {
        const id = this.selected?.id;
        if (!id || this.manuscriptOutline.length === 0) return;
        const path = outlinePathIds(this.manuscriptOutline, id);
        if (!path || path.length < 2) return;
        const next = new Set(this.manuscriptExpanded);
        let changed = false;
        for (const ancestor of path.slice(0, -1)) {
          if (!next.has(ancestor)) {
            next.add(ancestor);
            changed = true;
          }
        }
        if (changed) this.manuscriptExpanded = next;
      });

      // Applies a queued collection scroll once the list it belongs to is on screen.
      $effect(() => {
        const pending = this.pendingCollectionScroll;
        void this.collectionPage.items;
        void this.manuscriptOutline;
        if (
          !pending ||
          this.collectionLoading ||
          this.manuscriptOutlineLoading ||
          this.collectionQuery.section !== pending.section ||
          this.section !== pending.section ||
          this.projectHomeOpen ||
          this.showSettings ||
          this.hostView ||
          this.sandboxView ||
          this.loreWikiOpen
        )
          return;
        this.pendingCollectionScroll = null;
        void tick().then(() => {
          if (!this.collectionListElement || this.section !== pending.section) return;
          this.collectionListElement.scrollTop = pending.scrollTop;
          this.collectionScrollBySection[pending.section] = this.collectionListElement.scrollTop;
        });
      });

      // The wiki belongs to Lore: leaving the section closes it.
      $effect(() => {
        void this.section;
        if (this.section !== "lore" && this.loreWikiOpen) {
          this.loreWikiOpen = false;
          this.loreWikiEntityId = null;
        }
      });

      $effect(() => {
        // The webview stays mounted across entities, so the shell tells it which one to show: once it is ready, and
        // again whenever the selection or a reload from disk replaces what the shell holds.
        if (!this.ctx.writingWebviewShown || !this.writingWebview.ready) return;
        const entityId = this.selected?.id;
        if (!entityId || this.selectedLoading) return;
        const key = `${entityId}:${this.selectedLoadToken}`;
        if (key === this.writingLoadKey) return;
        this.writingLoadKey = key;
        // Only the selection and the load counter decide when to post; what the message reads is not tracked.
        void untrack(() => this.postWritingLoad(entityId, key));
      });

      $effect(() => {
        // The webview runs the AI rewrite flow itself, so it is told whether AI is on, the provider's name and the
        // editor prompt templates: after it reports ready, and again whenever any of them changes. It asks the broker
        // for nothing of this. The shell's own AI dialog stays off for it: the shell editor is not rendered here.
        void this.writingReadyCount;
        if (!this.writingWebview.mounted || !this.writingWebview.ready) return;
        const message = writingAiStateMessage({
          enabled: this.ctx.projectInfo?.aiEnabled ?? false,
          providerName: this.ctx.aiProvider.name,
          templates: this.ctx.aiPromptTemplates,
        });
        void untrack(() => this.postWritingMessage(message));
      });

      $effect(() => {
        // Editability follows the shell's state (loading, conflict, AI). The load message carries the value at its
        // posting time; this posts the changes after it.
        const editable = this.writingEditable;
        if (!this.writingWebview.ready || this.writingLoad === null || this.writingLoad.entityId !== this.selected?.id)
          return;
        if (editable === this.writingEditablePosted) return;
        this.writingEditablePosted = editable;
        void this.postWritingMessage({ type: "writing.editable", editable });
      });

      $effect(() => {
        // A revision conflict found by the shell's own save path is shown in the webview's banner.
        const conflict = this.documentConflict;
        if (
          !conflict ||
          !this.writingWebview.ready ||
          this.writingLoad === null ||
          this.writingLoad.entityId !== this.selected?.id
        )
          return;
        void this.postWritingMessage(writingConflictMessage(this.writingLoad.entityId, conflict));
      });

      $effect(() => {
        // The status area that showed save errors is hidden while the webview is shown, so the webview reports them.
        const message = this.saveError;
        if (
          !message ||
          !this.writingWebview.ready ||
          this.writingLoad === null ||
          this.writingLoad.entityId !== this.selected?.id
        )
          return;
        void this.postWritingMessage({ type: "writing.save-error", entityId: this.writingLoad.entityId, message });
      });

      // The full screen belongs to the entry on screen: choosing another entry, or leaving the Writing pane, ends it.
      $effect(() => {
        void this.selected?.id;
        void this.ctx.writingWebviewShown;
        untrack(() => this.exitWritingFullscreen());
      });
    });
  }

  /** Stops the store's effects. The page calls it on teardown. */
  destroy() {
    this.stopEffects();
  }

  // ---- Date field drafts -------------------------------------------------------------------------------------

  dateForField(key: string) {
    return parseCalendarDate(this.fields[key]);
  }
  dateFieldRevisionKey(key: string): string | null {
    const definition = this.ctx.definitions().find((candidate) => candidate.key === key);
    if (!definition) return null;
    return fieldRevisionKey(this.ctx.namespaceForField(definition), key);
  }
  dateFieldHadPersistedValue(key: string): boolean {
    const revisionKey = this.dateFieldRevisionKey(key);
    return revisionKey ? Object.prototype.hasOwnProperty.call(this.loadedFieldRevisions, revisionKey) : false;
  }
  updateInspectorDateField(key: string, next: unknown) {
    this.fields = { ...this.fields, [key]: next };
    this.scheduleDateFieldDirty(key);
  }
  setDateCalendar(key: string, calendarId: string) {
    this.dateCalendarByField = { ...this.dateCalendarByField, [key]: calendarId };
    const previous = this.dateForField(key);
    if (!previous) {
      this.dateEditorOpen = { ...this.dateEditorOpen, [key]: true };
      return;
    }
    this.updateInspectorDateField(key, serializeCalendarDate({ ...previous, calendar: calendarId }));
  }
  openDateEditor(key: string) {
    this.dateEditorOpen = { ...this.dateEditorOpen, [key]: true };
    this.dateCalendarByField = {
      ...this.dateCalendarByField,
      [key]: this.dateCalendarByField[key] ?? GREGORIAN_CALENDAR_ID,
    };
  }
  clearDateField(key: string) {
    this.cancelDateFieldDirty(key);
    this.fields = { ...this.fields, [key]: "" };
    this.dateEditorOpen = { ...this.dateEditorOpen, [key]: false };
    const next = { ...this.dateCalendarByField };
    delete next[key];
    this.dateCalendarByField = next;
    if (this.dateFieldHadPersistedValue(key)) this.markEntryDirty({ fieldKey: key });
  }

  cancelDateFieldDirty(key?: string) {
    if (key) {
      const timer = this.dateFieldDirtyTimers[key];
      if (timer) {
        window.clearTimeout(timer);
        delete this.dateFieldDirtyTimers[key];
      }
      return;
    }
    for (const timerKey of Object.keys(this.dateFieldDirtyTimers)) {
      window.clearTimeout(this.dateFieldDirtyTimers[timerKey]);
      delete this.dateFieldDirtyTimers[timerKey];
    }
  }
  dateFieldDirtyApplies(key: string): boolean {
    const value = this.fields[key];
    return hasMeaningfulDateValue(value) || (this.dateFieldHadPersistedValue(key) && isEmptyFieldValue(value));
  }
  scheduleDateFieldDirty(key: string, delay = 500) {
    this.cancelDateFieldDirty(key);
    this.dateFieldDirtyTimers[key] = window.setTimeout(() => {
      delete this.dateFieldDirtyTimers[key];
      if (this.dateFieldDirtyApplies(key)) this.markEntryDirty({ fieldKey: key });
    }, delay);
  }
  /** Apply pending debounced date edits immediately so navigation and manual saves cannot drop them. */
  flushDateFieldDirty() {
    const keys = Object.keys(this.dateFieldDirtyTimers);
    if (keys.length === 0) return;
    this.cancelDateFieldDirty();
    for (const key of keys) {
      if (this.dateFieldDirtyApplies(key)) this.markEntryDirty({ fieldKey: key });
    }
  }
  hintDateCalendarsFromEras(contexts: EraContext[], persistDates = false) {
    const calendarId = firstEraCalendarId(contexts);
    if (!calendarId) return;
    for (const definition of this.ctx.chronologyDateDefinitions()) {
      const current = this.dateForField(definition.key);
      if (current && !isGregorianCalendarId(current.calendar)) continue;
      this.dateCalendarByField = { ...this.dateCalendarByField, [definition.key]: calendarId };
      if (persistDates && current) {
        this.fields = {
          ...this.fields,
          [definition.key]: serializeCalendarDate({ ...current, calendar: calendarId }),
        };
        this.markEntryDirty({ fieldKey: definition.key });
      }
    }
  }

  // ---- Dirty tracking and autosave ---------------------------------------------------------------------------

  cancelAutoSave() {
    if (this.autoSaveTimer !== null) {
      window.clearTimeout(this.autoSaveTimer);
      this.autoSaveTimer = null;
    }
  }
  scheduleAutoSave(delay = 900) {
    this.cancelAutoSave();
    if (!this.selected || !this.ctx.sectionEnabled() || this.documentConflict || this.ctx.projectDiagnostics.length > 0)
      return;
    this.autoSaveTimer = window.setTimeout(() => {
      this.autoSaveTimer = null;
      void this.saveDocument();
    }, delay);
  }
  clearDirtyTracking() {
    this.dirtyFieldKeys.clear();
  }
  markEntryDirty(opts?: { fieldKey?: string; document?: boolean }) {
    this.documentRevision += 1;
    this.hasUnsavedChanges = true;
    this.savedAt = "";
    this.autoSaveFailureCount = 0;
    if (opts?.fieldKey) this.dirtyFieldKeys.add(opts.fieldKey);
    this.scheduleAutoSave();
  }
  updateDocumentBody = (value: string) => {
    if (this.selectedLoading || this.selectedLoadError || this.ctx.projectDiagnostics.length > 0) return;
    this.documentBody = value;
    this.markEntryDirty({ document: true });
  };
  setEditorFullscreen = (value: boolean) => {
    this.editorFullscreen = value;
  };
  async setDocumentMode(mode: "read" | "edit") {
    if (mode === this.documentMode) return;
    if (this.documentMode === "edit" && !(await this.flushAutoSave())) return;
    this.documentMode = mode;
  }
  updateField(definition: FieldDefinition, event: Event) {
    if (this.selectedLoading || this.selectedLoadError || this.ctx.projectDiagnostics.length > 0) return;
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
    this.fields = { ...this.fields, [definition.key]: value };
    this.markEntryDirty({ fieldKey: definition.key });
  }

  flushAutoSave = async () => {
    // The Writing webview holds the editor while it is shown, so its unsent text comes first.
    await this.flushWritingWebview();
    this.editorRef?.flushPendingChanges();
    this.flushDateFieldDirty();
    this.cancelAutoSave();
    if (!this.hasUnsavedChanges) return true;
    return this.saveDocument();
  };

  // ---- Saving ------------------------------------------------------------------------------------------------

  async persistDocumentSnapshot(): Promise<boolean> {
    const { project } = this.ctx;
    if (
      !this.selected ||
      this.selectedLoading ||
      this.selectedLoadError ||
      !this.ctx.sectionEnabled() ||
      this.documentConflict ||
      this.ctx.projectDiagnostics.length > 0
    )
      return false;
    const entityId = this.selected.id;
    const body = this.documentBody;
    const revision = this.documentRevision;
    const fieldsSnapshot = { ...this.fields };
    const definitionsForSave = this.ctx.definitions().filter((definition) => {
      if (definition.type === "relationship") return false;
      if (!this.dirtyFieldKeys.has(definition.key)) return false;
      const namespace = this.ctx.namespaceForField(definition);
      const key = fieldRevisionKey(namespace, definition.key);
      return shouldPersistFieldValue(
        fieldsSnapshot[definition.key],
        Object.prototype.hasOwnProperty.call(this.loadedFieldRevisions, key),
      );
    });
    try {
      await project.saveEntry(
        {
          document: { entity_id: entityId, body, format: "markdown" },
          fields: definitionsForSave.map((definition) => {
            const namespace = this.ctx.namespaceForField(definition);
            const value = fieldValueForSave(definition, fieldsSnapshot[definition.key] ?? "");
            const persistedValue =
              definition.type === "date"
                ? value
                  ? (parseCalendarDate(value) ?? value)
                  : value
                : restoreStructuredFieldValue(
                    value,
                    this.loadedStructuredFieldKeys.has(fieldRevisionKey(namespace, definition.key)),
                    definition.label,
                  );
            return {
              entity_id: entityId,
              namespace,
              key: definition.key,
              value: persistedValue,
              revision: this.loadedFieldRevisions[fieldRevisionKey(namespace, definition.key)] ?? "",
            };
          }),
        },
        { expectedRevision: this.loadedDocumentRevision || undefined },
      );
      const [documents, storedFields] = await Promise.all([
        project.listDocuments(entityId),
        project.listFields(entityId),
      ]);
      if (this.selected?.id === entityId) {
        this.loadedDocumentRevision = documents[0]?.revision ?? "";
        this.loadedFieldRevisions = Object.fromEntries(
          storedFields.map((field) => [fieldRevisionKey(field.namespace, field.key), field.revision]),
        );
        this.loadedStructuredFieldKeys = new Set(
          storedFields
            .filter((field) => isStructuredFieldValue(field.value))
            .map((field) => fieldRevisionKey(field.namespace, field.key)),
        );
      }
      if (this.selected?.id === entityId && this.documentRevision === revision) {
        this.hasUnsavedChanges = false;
        this.clearDirtyTracking();
        this.autoSaveFailureCount = 0;
        this.saveError = "";
        this.savedAt = new Date().toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" });
        this.notifyWritingSaved(entityId);
        void this.ctx.loadProjectStatus(false);
        try {
          const loaded = await project.getEntity(entityId);
          if (loaded && !loaded.deleted) this.ctx.upsertEntityInCache(loaded);
        } catch {
          // Relationship OCC still refetches the source revision at create time.
        }
        if (this.selected.entity_type === "daena.timeline:era") {
          eraContextCache.invalidate(entityId);
        }
        if (definitionsForSave.some((definition) => definition.key === "startsAt")) {
          this.ctx.notifyProfileTimelineChanged(entityId);
        }
        const loadToken = this.selectedLoadToken;
        const linkedEraIds = this.relationships
          .filter((relationship) => relationship.relationship_type === "during" && relationship.source_id === entityId)
          .map((relationship) => relationship.target_id);
        if (linkedEraIds.length > 0) {
          const nextEraContexts = await this.ctx.loadEraContexts(linkedEraIds);
          if (loadToken === this.selectedLoadToken && this.selected?.id === entityId)
            this.eraContexts = nextEraContexts;
        }
      }
      return true;
    } catch (cause) {
      if (isRevisionConflict(cause) && this.selected?.id === entityId) {
        try {
          const documents = await project.listDocuments(entityId);
          this.conflictDiskBody = normalizeDocument(documents[0]?.body ?? "", documents[0]?.format);
        } catch {
          this.conflictDiskBody = "";
        }
        this.documentConflict = { paths: [], diagnostics: [] };
        this.savedAt = "";
        this.saveError = "";
      } else {
        this.saveError = friendlyError(cause);
        this.autoSaveFailureCount += 1;
        const retryDelay = Math.min(30_000, 2_000 * 2 ** Math.min(this.autoSaveFailureCount - 1, 4));
        this.scheduleAutoSave(retryDelay);
      }
      return false;
    }
  }

  async saveDocument(): Promise<boolean> {
    this.cancelAutoSave();
    if (this.saveInFlight) {
      this.saveQueued = true;
      const currentResult = await this.saveInFlight;
      if (!currentResult || !this.hasUnsavedChanges) return currentResult;
      return this.saveDocument();
    }
    if (!this.hasUnsavedChanges) return true;
    this.isSaving = true;
    this.saveQueued = false;
    const operation = this.persistDocumentSnapshot();
    this.saveInFlight = operation;
    const result = await operation;
    if (this.saveInFlight === operation) this.saveInFlight = null;
    this.isSaving = false;
    const shouldSaveQueuedChanges = result && this.saveQueued && this.hasUnsavedChanges;
    this.saveQueued = false;
    return shouldSaveQueuedChanges ? this.saveDocument() : result;
  }

  // ---- Selection and load ------------------------------------------------------------------------------------

  async selectEntity(entity: Entity, recordHistory = true) {
    if (this.selected?.id === entity.id) return;
    if (!(await this.flushAutoSave())) return;
    if (this.section === "language" && !(await this.canLeaveLanguageSection())) return;
    const departure = this.currentShellLocation();
    if (this.section === "maps" && this.sandboxView?.renderer === "maps" && !(await this.leavePluginView())) return;
    if (recordHistory) this.ctx.recordShellDeparture(departure);
    this.editorFullscreen = false;
    this.selected = entity;
    this.applyCollectionTabForEntityType(entity.entity_type);
    this.hasUnsavedChanges = false;
    this.documentConflict = null;
    this.documentRevision = 0;
    this.clearDirtyTracking();
    this.saveError = "";
    this.ctx.setError("");
    try {
      await this.loadSelectedState(entity);
    } catch (cause) {
      this.ctx.setError(friendlyError(cause));
    }
  }

  async loadSelectedState(entity: Entity) {
    const { project } = this.ctx;
    const token = ++this.selectedLoadToken;
    const entityId = entity.id;
    const isCurrent = () => token === this.selectedLoadToken && this.selected?.id === entityId;
    this.selectedLoading = true;
    this.selectedLoadError = "";
    try {
      this.closeAiFieldFill();
      this.closeAiRewrite();
      this.documentBody = "";
      this.fields = {};
      this.relationships = [];
      this.eraContexts = [];
      this.metadataDialog = null;
      this.assets = [];
      this.mapLocations = [];
      this.loadedDocumentRevision = "";
      this.loadedFieldRevisions = {};
      this.loadedStructuredFieldKeys = new Set();
      this.clearDirtyTracking();
      const context = this.ctx.contextFor();
      const documents = await project.listDocuments(entityId);
      if (!isCurrent()) return;
      const document = documents[0];
      this.documentBody = normalizeDocument(document?.body ?? "", document?.format);
      this.loadedDocumentRevision = document?.revision ?? "";
      const storedFields = await project.listFields(entityId);
      if (!isCurrent()) return;
      const activeNamespaces = new Set(
        this.ctx
          .activeManifest()
          ?.schemas.filter((schema) => !entity.entity_type || schemaEntityTypeIds(schema).includes(entity.entity_type))
          .map((schema) => schema.namespace) ?? [],
      );
      const relevantFields = storedFields.filter(
        (field) => activeNamespaces.size === 0 || activeNamespaces.has(field.namespace),
      );
      const values = Object.fromEntries(relevantFields.map((field) => [field.key, field.value]));
      this.loadedFieldRevisions = Object.fromEntries(
        relevantFields.map((field) => [fieldRevisionKey(field.namespace, field.key), field.revision]),
      );
      this.loadedStructuredFieldKeys = new Set(
        relevantFields
          .filter((field) => isStructuredFieldValue(field.value))
          .map((field) => fieldRevisionKey(field.namespace, field.key)),
      );
      this.dateEditorOpen = {};
      this.cancelDateFieldDirty();
      const nextDateCalendars: Record<string, string> = {};
      this.fields = Object.fromEntries(
        Object.entries(values).map(([key, value]) => {
          const definition = this.ctx.definitions().find((candidate) => candidate.key === key);
          if (definition?.type === "date") {
            const date = parseCalendarDate(value);
            if (date && !isGregorianCalendarId(date.calendar)) nextDateCalendars[key] = date.calendar;
            const serialized = date ? serializeCalendarDate(date) : "";
            if (isSentinelDateValue(serialized) || isSentinelDateValue(date)) return [key, ""];
            return [key, serialized === "" ? String(value ?? "") : serialized];
          }
          if (definition && (definition.type === "number" || definition.type === "boolean" || definition.multiple))
            return [key, value];
          return [key, fieldDisplayValue(value)];
        }),
      );
      this.dateCalendarByField = nextDateCalendars;
      this.relationships = context.module.capabilities.includes("relationship.read")
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
      this.eraContexts = await this.ctx.loadEraContexts(
        this.relationships
          .filter((relationship) => relationship.relationship_type === "during" && relationship.source_id === entityId)
          .map((relationship) => relationship.target_id),
      );
      if (!isCurrent()) return;
      this.hintDateCalendarsFromEras(this.eraContexts, false);
      this.assets = context.module.capabilities.includes("asset.read:self")
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
      const nextMapLocations = entityId && this.ctx.mapsEnabled() ? await project.listMapLocations(entityId) : [];
      if (!isCurrent()) return;
      this.mapLocations = nextMapLocations;
      this.savedAt = "";
      void this.refreshWritingAppearanceLabels();
    } catch (cause) {
      if (isCurrent()) this.selectedLoadError = friendlyError(cause);
      throw cause;
    } finally {
      if (isCurrent()) this.selectedLoading = false;
    }
  }

  async refreshSelectedMapLocations(entityId = this.selected?.id) {
    this.mapLocations = entityId && this.ctx.mapsEnabled() ? await this.ctx.project.listMapLocations(entityId) : [];
  }

  clearSelection() {
    this.cancelAutoSave();
    this.cancelDateFieldDirty();
    this.editorFullscreen = false;
    this.hasUnsavedChanges = false;
    this.selected = null;
    this.selectedLoading = false;
    this.selectedLoadError = "";
    this.documentBody = "";
    this.fields = {};
    this.relationships = [];
    this.eraContexts = [];
    this.metadataDialog = null;
    this.assets = [];
    this.savedAt = "";
    this.saveError = "";
    this.loadedDocumentRevision = "";
    this.loadedFieldRevisions = {};
    this.loadedStructuredFieldKeys = new Set();
    this.documentConflict = null;
    this.conflictDiskBody = "";
    this.ctx.resetSelectionShellState();
  }

  // ---- Conflicts ---------------------------------------------------------------------------------------------

  async reloadSelectedFromDisk() {
    if (!this.selected) return;
    const current = this.ctx.entities.find((entity) => entity.id === this.selected?.id) ?? this.selected;
    this.selected = current;
    await this.loadSelectedState(current);
    this.hasUnsavedChanges = false;
    this.documentRevision = 0;
    this.clearDirtyTracking();
    this.documentConflict = null;
    this.conflictDiskBody = "";
    this.savedAt = "";
    this.saveError = "";
  }

  reloadConflict = async () => {
    try {
      await this.reloadSelectedFromDisk();
    } catch (cause) {
      this.ctx.setError(friendlyError(cause));
    }
  };

  overwriteConflict = async () => {
    if (!this.selected) return;
    const { project } = this.ctx;
    try {
      const [documents, storedFields] = await Promise.all([
        project.listDocuments(this.selected.id),
        project.listFields(this.selected.id),
      ]);
      this.loadedDocumentRevision = documents[0]?.revision ?? "";
      this.loadedFieldRevisions = Object.fromEntries(
        storedFields.map((field) => [fieldRevisionKey(field.namespace, field.key), field.revision]),
      );
      this.loadedStructuredFieldKeys = new Set(
        storedFields
          .filter((field) => isStructuredFieldValue(field.value))
          .map((field) => fieldRevisionKey(field.namespace, field.key)),
      );
      this.documentConflict = null;
      this.conflictDiskBody = "";
      if (!(await this.saveDocument()) && !this.documentConflict && !this.saveError)
        this.documentConflict = { paths: [], diagnostics: ["The draft could not be written as a new revision."] };
    } catch (cause) {
      this.documentConflict = { paths: [], diagnostics: [friendlyError(cause)] };
    }
  };

  saveConflictRecoveryCopy = async () => {
    if (!this.selected) return;
    try {
      const path = await this.ctx.project.saveRecoveryCopy(this.selected.id, this.documentBody);
      this.ctx.setError(`Draft saved as ${path}`);
    } catch (cause) {
      this.ctx.setError(friendlyError(cause));
    }
  };

  // ---- Relationships -----------------------------------------------------------------------------------------

  relationshipsForDefinition(definition: FieldDefinition) {
    if (!this.selected) return [];
    return relationshipsForField(this.selected.id, this.relationships, definition);
  }

  async updateRelationshipField(definition: FieldDefinition, targetIds: string[]) {
    if (this.ctx.projectDiagnostics.length > 0) return;
    if (!this.selected || !definition.relationshipType) return;
    const { project } = this.ctx;
    const desired = new Set(targetIds);
    const current = this.relationshipsForDefinition(definition);
    const currentIds = new Set(
      current
        .map((relationship) => counterpartId(this.selected!.id, relationship, definition))
        .filter((id): id is string => Boolean(id)),
    );
    const toRemove = current.filter((relationship) => {
      const otherId = counterpartId(this.selected!.id, relationship, definition);
      return !otherId || !desired.has(otherId);
    });
    const toAdd = [...desired].filter((targetId) => !currentIds.has(targetId));
    if (toRemove.length === 0 && toAdd.length === 0) return;
    try {
      const context = this.ctx.contextOwningRelationshipType(definition.relationshipType);
      const removedIds = new Set<string>();
      for (const relationship of toRemove) {
        try {
          await context.relationships.delete(relationship.id as UUID, relationship.relationship_type, {
            expectedRevision: relationship.revision,
          });
          removedIds.add(relationship.id);
        } catch (cause) {
          if (classifyMutationError(cause).code === "relationship.missing") {
            removedIds.add(relationship.id);
            continue;
          }
          throw cause;
        }
      }
      const created = [];
      for (const otherId of toAdd) {
        const endpoints = endpointsForCreate(this.selected!.id, otherId, definition);
        const source = await project.getEntity(endpoints.sourceId);
        const sourceRevision = source?.revision ?? "";
        if (!sourceRevision) throw new Error("The entity revision is unavailable. Reload the project and try again.");
        if (source) this.ctx.upsertEntityInCache(source);
        const createdRelationship = await context.relationships.create(
          {
            sourceId: endpoints.sourceId as UUID,
            targetId: endpoints.targetId as UUID,
            type: definition.relationshipType,
            metadata: defaultRelationshipMetadata(definition),
          },
          { expectedRevision: sourceRevision, requestId: crypto.randomUUID() },
        );
        created.push(toHostRelationship(createdRelationship));
      }
      this.relationships = [
        ...this.relationships.filter((relationship) => !removedIds.has(relationship.id)),
        ...created,
      ];
      await this.ctx.refreshAfterEntityMutation({ entityId: this.selected!.id });
      if (definition.relationshipType && isKinshipRelationshipType(definition.relationshipType)) {
        this.bumpKinshipRefresh();
      }
      if (isEraRelationshipField(definition)) {
        eraContextCache.invalidateMany(targetIds);
        this.eraContexts = await this.ctx.loadEraContexts(targetIds);
        this.hintDateCalendarsFromEras(this.eraContexts, true);
      }
      void this.refreshWritingAppearanceLabels();
    } catch (cause) {
      this.ctx.setError(friendlyError(cause));
    }
  }

  // ---- Assets ------------------------------------------------------------------------------------------------

  async refreshSelectedAssets() {
    if (!this.selected) {
      this.assets = [];
      return;
    }
    const namespaces = new Set(this.ctx.activeManifest()?.schemas.map((schema) => schema.namespace) ?? []);
    this.assets = (await this.ctx.project.listAssets(this.selected.id)).filter((asset) =>
      namespaces.has(asset.namespace),
    );
  }

  attachAsset = async () => {
    if (this.ctx.projectDiagnostics.length > 0) return;
    if (!this.selected) return;
    const { project } = this.ctx;
    try {
      const selection = await project.pickFile();
      const source = typeof selection === "string" ? selection : null;
      if (!source) return;
      const filename = source.split(/[\\/]/).pop() ?? "asset";
      const asset = await project.registerAssetFile({
        entity_id: this.selected.id,
        namespace: primarySchemaNamespace(this.ctx.activeManifest()?.schemas, {
          entityType: this.selected.entity_type,
          fallback: this.ctx.activeModuleId(),
        }),
        source_path: source,
        filename,
        mime_type: mimeTypeFor(filename),
      });
      this.assets = [...this.assets, asset];
    } catch (cause) {
      this.ctx.setError(friendlyError(cause));
    }
  };

  openAssetDialog(asset: Asset) {
    if (this.ctx.projectDiagnostics.length > 0) return;
    this.assetDialog = asset;
  }

  handleAssetSave = async (update: {
    filename?: string;
    role?: "attachment" | "profile";
    referenceScope?: "entity" | "project";
  }) => {
    const current = this.assetDialog;
    if (!current) return;
    this.assetBusyId = current.id;
    try {
      await this.ctx.project.updateAssetMetadata(current.id, update, current.revision);
      await this.refreshSelectedAssets();
    } catch (cause) {
      this.ctx.setError(friendlyError(cause));
      throw cause;
    } finally {
      this.assetBusyId = null;
    }
  };

  handleAssetDelete = async () => {
    const current = this.assetDialog;
    if (!current) return;
    if (
      !(await this.ctx.confirmDialog({
        title: `Delete ${current.filename}?`,
        message: "This file will be permanently removed from the project and the checkpoint. This cannot be undone.",
        confirmLabel: "Delete",
        danger: true,
      }))
    )
      return;
    this.assetBusyId = current.id;
    try {
      await this.ctx.project.deleteAsset(current.id, current.revision);
      await this.refreshSelectedAssets();
    } catch (cause) {
      this.ctx.setError(friendlyError(cause));
      throw cause;
    } finally {
      this.assetBusyId = null;
    }
  };

  handleAssetReplace = async () => {
    const current = this.assetDialog;
    if (!current) return;
    const { project } = this.ctx;
    const selection = await project.pickFile();
    const source = typeof selection === "string" ? selection : null;
    if (!source) return;
    const replacementName = source.split(/[\\/]/).pop() ?? "replacement";
    if (
      !(await this.ctx.confirmDialog({
        title: `Replace ${current.filename}?`,
        message: `Use ${replacementName} as the new content? The attachment name and metadata will stay unchanged.`,
        confirmLabel: "Replace",
      }))
    )
      return;
    this.assetBusyId = current.id;
    try {
      await project.replaceAssetFile(current.id, source, mimeTypeFor(replacementName), current.revision);
      await this.refreshSelectedAssets();
      // refresh dialog with new revision
      const refreshed = this.assets.find((a) => a.id === current.id) ?? null;
      if (refreshed) this.assetDialog = refreshed;
    } catch (cause) {
      this.ctx.setError(friendlyError(cause));
      throw cause;
    } finally {
      this.assetBusyId = null;
    }
  };

  // ---- Collection, scroll and panes --------------------------------------------------------------------------

  bumpCollectionRefresh() {
    if (this.collectionListElement) this.queueCollectionScroll(this.section, this.collectionListElement.scrollTop);
    this.collectionRefreshEpoch += 1;
  }

  bumpKinshipRefresh() {
    this.kinshipRefreshEpoch += 1;
    void this.refreshSelectedKinship();
  }

  async refreshSelectedKinship() {
    if (!this.selected || this.ctx.projectDiagnostics.length > 0) return;
    const entityId = this.selected.id;
    const context = this.ctx.contextFor();
    if (!context.module.capabilities.includes("relationship.read")) return;
    try {
      const listed = await context.relationships.list(entityId as UUID);
      if (this.selected?.id !== entityId) return;
      this.relationships = listed.map((relationship) => toHostRelationship(relationship));
    } catch {
      // Keep the prior inspector relationships if the refresh fails.
    }
  }

  toggleGroup(type: string) {
    const next = new Set(this.expandedGroups);
    if (next.has(type)) next.delete(type);
    else next.add(type);
    this.expandedGroups = next;
  }

  currentWorkspaceCollectionLocation(): WorkspaceCollectionLocation {
    const section = this.section;
    const pendingScrollTop =
      this.pendingCollectionScroll?.section === section ? this.pendingCollectionScroll.scrollTop : null;
    return {
      query: {
        textSearch: this.collectionQuery.textSearch,
        sortField: this.collectionQuery.sortField,
        sortDir: this.collectionQuery.sortDir,
        pageSize: this.collectionQuery.pageSize,
        page: this.collectionQuery.page,
        excludedTypes: [...this.collectionQuery.excludedTypes].sort(),
        viewMode: this.collectionQuery.viewMode,
      },
      expandedGroups: [...this.expandedGroups].sort(),
      scrollTop:
        pendingScrollTop ?? this.collectionListElement?.scrollTop ?? this.collectionScrollBySection[section] ?? 0,
    };
  }

  async applyWorkspaceCollectionLocation(location: WorkspaceCollectionLocation) {
    this.collectionQueryRestoring = true;
    this.collectionQuery = {
      section: this.section,
      ...location.query,
      excludedTypes: [...location.query.excludedTypes],
    };
    this.expandedGroups = new Set(location.expandedGroups);
    this.queueCollectionScroll(this.section, location.scrollTop);
    await tick();
    this.collectionQueryRestoring = false;
  }

  rememberCollectionScroll = () => {
    if (!this.collectionListElement) return;
    this.collectionScrollBySection[this.section] = this.collectionListElement.scrollTop;
  };

  queueCollectionScroll(target: WorkspaceSection, scrollTop: number) {
    this.collectionScrollBySection[target] = scrollTop;
    this.pendingCollectionScroll = { section: target, scrollTop };
  }

  currentWorkspacePaneDimensions(): WorkspacePaneDimensions {
    const fallback = this.restoredWorkspacePaneDimensions;
    return {
      collectionWidth: measuredPaneWidth(this.collectionPaneElement, fallback?.collectionWidth ?? 0),
      contentWidth: measuredPaneWidth(this.contentPaneElement, fallback?.contentWidth ?? 0),
      inspectorWidth: measuredPaneWidth(this.inspectorPaneElement, fallback?.inspectorWidth ?? 0),
      viewportWidth: typeof window === "undefined" ? (fallback?.viewportWidth ?? 0) : window.innerWidth,
    };
  }

  async restoreWorkspacePaneDimensions(panes: WorkspacePaneDimensions) {
    const viewportWidth = typeof window === "undefined" ? panes.viewportWidth : window.innerWidth;
    this.restoredWorkspacePaneDimensions =
      panes.viewportWidth > 0 && Math.abs(panes.viewportWidth - viewportWidth) <= 2 ? { ...panes } : null;
    await tick();
  }

  persistWorkbenchLayout() {
    saveWorkbenchLayout(this.section, {
      visibility: this.workbenchPaneVisibility,
      collectionWidth: this.workbenchPaneWidths.collection,
      inspectorWidth: this.workbenchPaneWidths.inspector,
    });
  }

  activePaneDimensions(): WorkspacePaneDimensions {
    return {
      collectionWidth: this.restoredWorkspacePaneDimensions?.collectionWidth || this.workbenchPaneWidths.collection,
      contentWidth: this.restoredWorkspacePaneDimensions?.contentWidth || 640,
      inspectorWidth: this.restoredWorkspacePaneDimensions?.inspectorWidth || this.workbenchPaneWidths.inspector,
      viewportWidth: typeof window === "undefined" ? 0 : window.innerWidth,
    };
  }

  resizeWorkbenchPane(pane: "collection" | "inspector", delta: number) {
    const panes = this.activePaneDimensions();
    if (pane === "collection")
      panes.collectionWidth = Math.max(collectionPaneMin, Math.min(collectionPaneMax, panes.collectionWidth + delta));
    else panes.inspectorWidth = Math.max(inspectorPaneMin, Math.min(inspectorPaneMax, panes.inspectorWidth + delta));
    this.workbenchPaneWidths = {
      collection: panes.collectionWidth,
      inspector: panes.inspectorWidth,
    };
    this.restoredWorkspacePaneDimensions = panes;
    this.persistWorkbenchLayout();
  }

  resetWorkbenchPane(pane: "collection" | "inspector") {
    const panes = this.activePaneDimensions();
    if (pane === "collection") panes.collectionWidth = collectionPaneDefault;
    else panes.inspectorWidth = inspectorPaneDefault;
    this.workbenchPaneWidths = {
      collection: panes.collectionWidth,
      inspector: panes.inspectorWidth,
    };
    this.restoredWorkspacePaneDimensions = panes;
    this.persistWorkbenchLayout();
  }

  async toggleWorkbenchPane(pane: WorkbenchPane) {
    // Hiding the content column tears down the Language and Writing webviews, so save and ask them first.
    if (pane === "content" && this.workbenchPaneVisibility.content) {
      if (!(await this.flushAutoSave())) return;
      if (!(await this.canLeaveLanguageSection())) return;
    }
    const visible = !this.workbenchPaneVisibility[pane];
    this.workbenchPaneVisibility = { ...this.workbenchPaneVisibility, [pane]: visible };
    this.persistWorkbenchLayout();
    if (pane === "content" && !visible) this.editorFullscreen = false;
  }

  currentSpecializedSurfaceScrollTop() {
    const key = this.specializedSurfaceKey();
    if (!key) return 0;
    return this.pendingSpecializedSurfaceScroll?.key === key
      ? this.pendingSpecializedSurfaceScroll.scrollTop
      : (this.specializedSurfaceElement?.scrollTop ?? this.specializedSurfaceScrollByKey[key] ?? 0);
  }

  rememberSpecializedSurfaceScroll = (scrollTop: number) => {
    const key = this.specializedSurfaceKey();
    if (!key) return;
    this.specializedSurfaceScrollByKey[key] = scrollTop;
    if (this.pendingSpecializedSurfaceScroll?.key === key) this.pendingSpecializedSurfaceScroll = null;
  };

  async restoreSpecializedSurfaceScroll(location: ShellLocation) {
    if (location.kind === "home" || location.kind === "settings" || location.kind === "project") return;
    const key = specializedSurfaceKeyForLocation(location);
    if (!key) return;
    this.pendingSpecializedSurfaceScroll = { key, scrollTop: location.surfaceScrollTop };
    this.specializedSurfaceScrollByKey[key] = location.surfaceScrollTop;
    await tick();
    await new Promise<void>((resolve) => window.requestAnimationFrame(() => resolve()));
    if (this.specializedSurfaceKey() !== key || !this.specializedSurfaceElement) return;
    this.specializedSurfaceElement.scrollTop = location.surfaceScrollTop;
    this.specializedSurfaceScrollByKey[key] = this.specializedSurfaceElement.scrollTop;
    this.pendingSpecializedSurfaceScroll = null;
  }

  // ---- Manuscript outline and Writing appearance labels ------------------------------------------------------

  queryWritingPartOf = async (entityIds: string[]) => {
    if (entityIds.length === 0 || !this.ctx.projectInfo?.root) return [];
    const context = this.ctx.contextFor("writing");
    const items: Array<{
      id: string;
      sourceId: string;
      targetId: string;
      type: string;
      metadata?: unknown;
    }> = [];
    let offset = 0;
    while (true) {
      const page = await context.relationships.query({
        entityIds: entityIds as UUID[],
        relationshipTypes: [WRITING_PART_OF],
        direction: "any",
        offset,
        limit: 200,
      });
      items.push(...page.items);
      if (!page.hasMore || page.items.length === 0) break;
      offset += page.items.length;
    }
    return partOfEdgesFromRelationships(
      items.map((relationship) => ({
        id: relationship.id,
        source_id: relationship.sourceId,
        target_id: relationship.targetId,
        relationship_type: relationship.type,
        metadata: relationship.metadata,
      })),
    );
  };

  async listManuscriptOutlineSummaries(
    entityTypes: string[],
    excludedEntityTypes: string[],
    sortField: SortField,
    sortDirection: SortDirection,
  ): Promise<{ items: Entity[]; truncated: boolean }> {
    const items: Entity[] = [];
    let offset = 0;
    while (items.length < MANUSCRIPT_OUTLINE_CAP) {
      const limit = Math.min(200, MANUSCRIPT_OUTLINE_CAP - items.length);
      const page = await this.ctx.project.queryEntities({
        entityTypes,
        excludedEntityTypes,
        sortField,
        sortDirection,
        offset,
        limit,
      });
      items.push(...page.items);
      if (!page.has_more || page.items.length === 0) return { items, truncated: false };
      offset += page.items.length;
    }
    return { items, truncated: true };
  }

  async manuscriptContainmentLabels(seed: string[]): Promise<Record<string, string>> {
    const edges = await collectPartOfEdges(seed, this.queryWritingPartOf);
    const ids = new Set(seed);
    for (const edge of edges) {
      ids.add(edge.sourceId);
      ids.add(edge.targetId);
    }
    const names = new Map<string, string>();
    for (const entity of [...this.manuscriptOutlineEntities, ...this.ctx.entities, ...this.collectionPage.items]) {
      if (ids.has(entity.id)) names.set(entity.id, entity.name);
    }
    const missing = [...ids].filter((id) => !names.has(id));
    for (let index = 0; index < missing.length; index += 500) {
      const loaded = await this.ctx.contextFor("writing").entities.getMany(missing.slice(index, index + 500) as UUID[]);
      for (const entity of loaded) names.set(entity.id, entity.name);
    }
    const parents = parentByChild(edges);
    const labels: Record<string, string> = {};
    for (const id of seed) labels[id] = appearanceLabel(containmentNames(id, parents, names));
    return labels;
  }

  async refreshWritingAppearanceLabels() {
    if (!this.selected) {
      this.writingAppearanceLabels = {};
      return;
    }
    const sources = [
      ...new Set(
        this.relationships
          .filter(
            (relationship) =>
              relationship.relationship_type === WRITING_FEATURES && relationship.target_id === this.selected!.id,
          )
          .map((relationship) => relationship.source_id),
      ),
    ];
    if (sources.length === 0) {
      this.writingAppearanceLabels = {};
      return;
    }
    try {
      const edges = await collectPartOfEdges(sources, this.queryWritingPartOf);
      const ids = new Set(sources);
      for (const edge of edges) {
        ids.add(edge.sourceId);
        ids.add(edge.targetId);
      }
      const names = new Map<string, string>();
      for (const entity of this.ctx.entities) {
        if (ids.has(entity.id)) names.set(entity.id, entity.name);
      }
      const missing = [...ids].filter((id) => !names.has(id));
      if (missing.length > 0) {
        const loaded = await this.ctx.contextFor("writing").entities.getMany(missing as UUID[]);
        for (const entity of loaded) names.set(entity.id, entity.name);
      }
      const parents = parentByChild(edges);
      const labels: Record<string, string> = {};
      for (const id of sources) labels[id] = appearanceLabel(containmentNames(id, parents, names));
      this.writingAppearanceLabels = labels;
    } catch {
      this.writingAppearanceLabels = {};
    }
  }

  toggleManuscriptOutlineNode(id: string) {
    const next = new Set(this.manuscriptExpanded);
    if (next.has(id)) next.delete(id);
    else next.add(id);
    this.manuscriptExpanded = next;
  }

  // ---- Which surface is open ---------------------------------------------------------------------------------

  /**
   * Leaves whatever plugin surface is open. Native plugin webviews are only closed when a
   * full-surface plugin view (host or sandbox) was open, or when the caller asks for it.
   * The Language content pane is not one of those: its SandboxView owns its webview and
   * closes it when it unmounts, so closing every webview here would blank a pane that stays.
   */
  async leavePluginView(options: { closeWebviews?: boolean } = {}): Promise<boolean> {
    if (!(await this.resolveDirtyMapSession())) return false;
    const fullSurfaceOpen = this.hostView !== null || this.sandboxView !== null;
    this.editorFullscreen = false;
    this.hostView = null;
    this.sandboxView = null;
    if (fullSurfaceOpen || options.closeWebviews) await this.ctx.closeNativePluginWebviews();
    return true;
  }

  /** The projection whose sandbox webview is the open full surface, if any. */
  projectionSandboxKey(): ProjectionSandboxKey | null {
    if (!this.sandboxView || this.sandboxView.renderer !== "webview" || !this.sandboxView.view) return null;
    const pluginId = this.sandboxView.plugin.id;
    const viewId = this.sandboxView.view.id;
    const keys = Object.keys(PROJECTION_SANDBOXES) as ProjectionSandboxKey[];
    return (
      keys.find(
        (key) => PROJECTION_SANDBOXES[key].pluginId === pluginId && PROJECTION_SANDBOXES[key].viewId === viewId,
      ) ?? null
    );
  }

  /** True while the given projection is open, or any projection when `key` is omitted. */
  projectionSandboxOpen(key?: ProjectionSandboxKey): boolean {
    const open = this.projectionSandboxKey();
    return open !== null && (key === undefined || open === key);
  }

  applyCollectionTabForEntityType(entityType: string | null) {
    const owner = this.ctx.sectionForEntityType(entityType) ?? this.section;
    const tab = collectionTabForEntityType(this.ctx.collectionTabsFor(owner), entityType);
    if (!tab) return;
    if (owner === "writing") this.writingView = tab.id;
    if (owner === "timeline") this.timelineView = tab.id;
  }

  currentWorkspaceLocationView(): WorkspaceLocationView {
    if (this.section === "lore") {
      if (this.loreWikiOpen) return "wiki";
      if (this.projectionSandboxOpen("lore")) return "graph";
      return "library";
    }
    if (this.section === "timeline") {
      if (this.projectionSandboxOpen("timeline")) return "timeline";
      return this.timelineView;
    }
    if (this.section === "writing") return this.writingView;
    if (this.section === "houses") return this.housesView;
    return "default";
  }

  currentModuleState(): Record<string, unknown> | null {
    if (this.section === "language") return { pane: this.languagePane };
    if (this.section === "houses" && this.housesView === "tree")
      return (this.treeSession as Record<string, unknown> | null) ?? null;
    return null;
  }

  restoreModuleState(section: WorkspaceSection, state: Record<string, unknown> | null | undefined) {
    if (section === "language") {
      this.requestLanguagePane(state && isLanguagePane(state.pane) ? state.pane : "overview");
      return;
    }
    if (section === "houses") {
      this.treeSession = (state as TreeSession | null) ?? null;
    }
  }

  specializedSurfaceKey(): string | null {
    if (this.hostView) return `plugin:${this.hostView.plugin.id}:${this.hostView.view.id}`;
    const projection = this.projectionSandboxKey();
    if (projection) return PROJECTION_SANDBOXES[projection].surfaceKey;
    if (this.sandboxView?.view && this.sandboxView.renderer !== "maps") {
      return `plugin:${this.sandboxView.plugin.id}:${this.sandboxView.view.id}`;
    }
    if (this.loreWikiOpen) return "workspace:lore:wiki";
    if (this.section === "houses" && this.housesView === "tree") return "workspace:houses:tree";
    return null;
  }

  currentShellLocation(): ShellLocation {
    if (this.showSettings && this.settingsSurface === "project")
      return { kind: "project", section: this.projectSection };
    if (this.showSettings) return { kind: "settings", section: this.settingsSection };
    if (this.projectHomeOpen) return { kind: "home" };
    if (this.hostView) {
      return {
        kind: "plugin",
        key: `${this.hostView.plugin.id}:${this.hostView.view.id}`,
        section: this.section,
        entityId: this.selected?.id ?? null,
        surfaceScrollTop: this.currentSpecializedSurfaceScrollTop(),
      };
    }
    if (this.sandboxView?.view && !this.projectionSandboxOpen()) {
      return {
        kind: "plugin",
        key: `${this.sandboxView.plugin.id}:${this.sandboxView.view.id}`,
        section: this.section,
        entityId: this.selected?.id ?? null,
        surfaceScrollTop: this.currentSpecializedSurfaceScrollTop(),
        moduleState: null,
      };
    }
    return {
      kind: "workspace",
      section: this.section,
      view: this.currentWorkspaceLocationView(),
      entityId: this.section === "houses" && this.housesView === "tree" ? this.treeRootId : (this.selected?.id ?? null),
      writingView: this.writingView,
      timelineView: this.timelineView,
      collection: this.currentWorkspaceCollectionLocation(),
      panes: this.currentWorkspacePaneDimensions(),
      surfaceScrollTop: this.currentSpecializedSurfaceScrollTop(),
      moduleState: this.currentModuleState(),
    };
  }

  async openHostView(
    plugin: PluginAdminEntry,
    view: PluginAdminEntry["views"][number],
    departure = this.currentShellLocation(),
  ) {
    if (!(await this.ctx.dismissSettings())) return;
    if (!(await this.leavePluginView())) return;
    this.ctx.recordShellDeparture(departure);
    this.projectHomeOpen = false;
    this.hostView = { plugin, view };
  }

  async openPluginView(item: PluginNavigationItem, departure = this.currentShellLocation()) {
    if (!(await this.flushAutoSave())) return;
    if (this.section === "language" && !(await this.canLeaveLanguageSection())) return;
    if (item.renderer === "maps") {
      if (!(await this.ctx.dismissSettings())) return;
      const mapId = this.currentMapId();
      if (this.sandboxView?.renderer === "maps") {
        const mapsWelcome = this.sandboxView.view === null;
        if ((mapId === null && mapsWelcome) || (mapId !== null && !mapsWelcome)) return;
      }
      if (mapId) {
        const mapField = (await this.ctx.project.listFields(mapId)).find(
          (field) => field.namespace === "maps" && field.key === "map",
        );
        const descriptor = mapField?.value as { provider?: { id?: string } } | undefined;
        this.mapsEditorMode = "vector";
      }
      this.clearMapFocus();
      if (!(await this.leavePluginView())) return;
      this.ctx.recordShellDeparture(departure);
      this.projectHomeOpen = false;
      this.loreWikiOpen = false;
      this.section = "maps";
      this.sandboxView = mapId
        ? { plugin: item.plugin, view: item.view, renderer: "maps" }
        : { plugin: item.plugin, view: null, renderer: "maps" };
      if (mapId) this.mapsEditorKey = mapId;
      else if (!this.sandboxView.view) this.mapsEditorKey = "welcome";
      return;
    }
    if (item.renderer === "host") {
      await this.openHostView(item.plugin, item.view, departure);
      return;
    }
    if (!(await this.ctx.dismissSettings())) return;
    if (!(await this.leavePluginView())) return;
    this.ctx.recordShellDeparture(departure);
    this.projectHomeOpen = false;
    this.sandboxView = { plugin: item.plugin, view: item.view, renderer: "webview" };
  }

  async selectSearchResult(entity: Entity) {
    if (!(await this.flushAutoSave())) return;
    const owner = this.ctx.workspaceSectionOrder.find((target) =>
      this.ctx
        .manifestForWorkspaceSection(target)
        ?.schemas.some((schema) => schemaEntityTypeIds(schema).includes(entity.entity_type ?? "")),
    );
    const nextSection: WorkspaceSection = owner && owner !== "maps" ? owner : "lore";
    if (this.section === "language" && nextSection !== "language" && !(await this.canLeaveLanguageSection())) return;
    const departure = this.currentShellLocation();
    if (!(await this.leavePluginView())) return;
    this.ctx.recordShellDeparture(departure);
    this.section = nextSection;
    this.applyCollectionTabForEntityType(entity.entity_type);
    this.projectHomeOpen = false;
    this.ctx.clearGlobalQuery();
    this.collectionQuery.textSearch = "";
    await this.selectEntity(entity, false);
  }

  async switchSection(next: WorkspaceSection) {
    if (!(await this.flushAutoSave())) return;
    if (
      this.section === next &&
      !this.projectHomeOpen &&
      (next !== "maps" || this.sandboxView?.renderer === "maps") &&
      !this.showSettings
    )
      return;
    if (this.section === "language" && next !== "language" && !(await this.canLeaveLanguageSection())) return;
    const departure = this.currentShellLocation();
    if (!(await this.ctx.dismissSettings())) return;
    if (!(await this.leavePluginView())) return;
    this.ctx.recordShellDeparture(departure);
    const sectionChanged = this.section !== next;
    this.section = next;
    this.queueCollectionScroll(next, this.collectionScrollBySection[next] ?? 0);
    this.projectHomeOpen = false;
    if (sectionChanged) {
      this.clearSelection();
      this.collectionQuery.textSearch = "";
    }
  }

  openProjectHome = async () => {
    if (!this.ctx.ready || (this.projectHomeOpen && !this.showSettings && !this.hostView && !this.sandboxView)) return;
    if (!(await this.flushAutoSave())) return;
    if (this.section === "language" && !(await this.canLeaveLanguageSection())) return;
    const departure = this.currentShellLocation();
    if (!(await this.ctx.dismissSettings())) return;
    if (!(await this.leavePluginView())) return;
    this.ctx.recordShellDeparture(departure);
    this.projectHomeOpen = true;
    this.ctx.clearGlobalQuery();
  };

  async reconcileWorkspaceSection() {
    if (this.ctx.enabledWorkspaceSections().includes(this.section)) return;
    if (!(await this.flushAutoSave())) return;
    if (!(await this.leavePluginView())) return;
    this.section = this.ctx.enabledWorkspaceSections()[0] ?? "lore";
    this.clearSelection();
    this.collectionQuery.textSearch = "";
    this.editorFullscreen = false;
  }

  async switchWritingView(next: WritingView) {
    const tabs = this.ctx.collectionTabsFor("writing");
    const resolved = tabs.some((tab) => tab.id === next) ? next : (tabs[0]?.id ?? next);
    if (!(await this.flushAutoSave())) return;
    if (this.writingView === resolved && !this.projectHomeOpen) return;
    const departure = this.currentShellLocation();
    if (!(await this.leavePluginView())) return;
    this.ctx.recordShellDeparture(departure);
    this.projectHomeOpen = false;
    this.writingView = resolved;
    this.clearSelection();
    this.collectionQuery.textSearch = "";
  }

  async switchTimelineView(next: TimelineView) {
    const tabs = this.ctx.collectionTabsFor("timeline");
    const resolved = tabs.some((tab) => tab.id === next) ? next : (tabs[0]?.id ?? next);
    if (!(await this.flushAutoSave())) return;
    if (this.timelineView === resolved && !this.projectHomeOpen && !this.projectionSandboxOpen()) return;
    const departure = this.currentShellLocation();
    if (!(await this.leavePluginView())) return;
    this.ctx.recordShellDeparture(departure);
    this.projectHomeOpen = false;
    this.timelineView = resolved;
    this.clearSelection();
    this.collectionQuery.textSearch = "";
  }

  async switchHousesView(next: WorkspaceLocationView) {
    const allowed: WorkspaceLocationView[] = ["houses", "tree"];
    const resolved = allowed.includes(next) ? next : "houses";
    if (!(await this.flushAutoSave())) return;
    if (this.section === "houses" && this.housesView === resolved && !this.projectHomeOpen) {
      if (resolved === "tree" && this.treeRootId) {
        this.ctx.recordShellDeparture(this.currentShellLocation());
        this.treeRootId = null;
        this.treeSession = null;
      }
      return;
    }
    const departure = this.currentShellLocation();
    if (!(await this.leavePluginView())) return;
    this.ctx.recordShellDeparture(departure);
    this.projectHomeOpen = false;
    if (this.section !== "houses") this.section = "houses";
    this.housesView = resolved;
  }

  async switchLanguagePane(next: LanguagePane) {
    if (!(await this.flushAutoSave())) return;
    if (this.languagePane === next && !this.projectHomeOpen && this.section === "language") return;
    const departure = this.currentShellLocation();
    // The Language pane stays mounted across pane switches, so only a full-surface view needs closing here.
    if (this.hostView || this.sandboxView) {
      if (!(await this.leavePluginView())) return;
    }
    this.ctx.recordShellDeparture(departure);
    this.projectHomeOpen = false;
    this.requestLanguagePane(next);
  }

  /** Opens the projection for the current section: Chronology in Timeline, otherwise the Lore World graph. */
  async openProjection() {
    if (!(await this.flushAutoSave())) return;
    const key: ProjectionSandboxKey = this.section === "timeline" ? "timeline" : "lore";
    const surface = PROJECTION_SANDBOXES[key];
    if (!this.projectHomeOpen && !this.showSettings && this.projectionSandboxOpen(key)) return;
    const departure = this.currentShellLocation();
    if (!(await this.leavePluginView())) return;
    this.ctx.recordShellDeparture(departure);
    this.projectHomeOpen = false;
    this.loreWikiOpen = false;
    this.loreWikiEntityId = null;
    if (!this.ctx.adminPlugins) await this.ctx.refreshAdmin();
    const plugin = this.ctx.adminPlugins?.find((entry) => entry.id === surface.pluginId && entry.enabled) ?? null;
    if (!plugin) {
      this.ctx.setError(`${surface.moduleName} is disabled, so ${surface.title} cannot open.`);
      return;
    }
    this.sandboxView = {
      plugin,
      // Shell-only id. The webview mounts the package entrypoint with no view
      // id: validate_plugin_view rejects ids that manifest.views does not list.
      view: { id: surface.viewId, title: surface.title, renderer: { type: "sandboxed" } },
      renderer: "webview",
    };
  }

  async openLoreLibrary() {
    if (!(await this.flushAutoSave())) return;
    if (!this.projectHomeOpen && !this.showSettings && !this.loreWikiOpen && !this.hostView && !this.sandboxView)
      return;
    const departure = this.currentShellLocation();
    if (!(await this.ctx.dismissSettings())) return;
    if (!(await this.leavePluginView())) return;
    this.ctx.recordShellDeparture(departure);
    this.projectHomeOpen = false;
    this.loreWikiOpen = false;
    this.loreWikiEntityId = null;
  }

  async openLoreWiki(entityId?: string) {
    if (!(await this.flushAutoSave())) return;
    const showingWiki =
      !this.projectHomeOpen && !this.showSettings && this.loreWikiOpen && !this.hostView && !this.sandboxView;
    if (showingWiki && entityId === undefined) return;
    const departure = this.currentShellLocation();
    if (!(await this.leavePluginView())) return;
    this.ctx.recordShellDeparture(departure);
    this.projectHomeOpen = false;
    if (entityId === undefined) {
      this.loreWikiEntityId =
        this.selected?.entity_type && this.ctx.allEntityTypesForSection("lore").has(this.selected.entity_type)
          ? this.selected.id
          : null;
    } else {
      const entity = await this.restoreShellEntity(entityId);
      this.loreWikiEntityId = entity ? entityId : null;
    }
    // Remount so an already-open wiki takes the entry this call names.
    if (this.loreWikiOpen) {
      this.loreWikiOpen = false;
      await tick();
    }
    this.loreWikiOpen = true;
  }

  async closeLoreWiki() {
    if (!(await this.flushAutoSave())) return;
    const departure = this.currentShellLocation();
    this.ctx.recordShellDeparture(departure);
    this.loreWikiOpen = false;
    this.loreWikiEntityId = null;
  }

  async closeProjectionView() {
    if (!(await this.flushAutoSave())) return;
    const departure = this.currentShellLocation();
    this.ctx.recordShellDeparture(departure);
    if (this.projectionSandboxOpen()) this.sandboxView = null;
  }

  async closePluginView() {
    if (!(await this.flushAutoSave())) return;
    const departure = this.currentShellLocation();
    if (!(await this.leavePluginView())) return;
    this.ctx.recordShellDeparture(departure);
  }

  async openWorkspaceView(view: WorkspaceLocationView) {
    if (view === "library") return this.openLoreLibrary();
    if (view === "wiki") return this.openLoreWiki();
    if (view === "graph" || view === "timeline") return this.openProjection();
    if (view === "houses" || view === "tree") return this.switchHousesView(view);
    if (this.section === "timeline" && this.ctx.collectionTabsFor("timeline").some((tab) => tab.id === view)) {
      return this.switchTimelineView(view);
    }
    if (this.section === "writing" && this.ctx.collectionTabsFor("writing").some((tab) => tab.id === view)) {
      return this.switchWritingView(view);
    }
    if (this.section === "houses") return this.switchHousesView(view);
  }

  async restoreShellEntity(entityId: string | null) {
    if (!entityId) {
      if (this.selected) this.clearSelection();
      return null;
    }
    let entity = this.ctx.entities.find((candidate) => candidate.id === entityId) ?? null;
    if (!entity) {
      try {
        const loaded = await this.ctx.project.getEntity(entityId);
        entity = loaded;
        if (loaded && !this.ctx.entities.some((candidate) => candidate.id === loaded.id))
          this.ctx.appendEntityToCache(loaded);
      } catch {
        entity = null;
      }
    }
    if (!entity || entity.deleted) {
      if (this.selected) this.clearSelection();
      return null;
    }
    await this.selectEntity(entity, false);
    return entity;
  }

  async restoreShellLocation(target: ShellLocation): Promise<boolean> {
    const pluginItem = target.kind === "plugin" ? this.ctx.pluginNavigationItemByKey(target.key) : null;
    if (target.kind === "plugin" && !pluginItem) return false;
    this.ctx.setShellNavigationRestoring(true);
    try {
      if (target.kind === "home") {
        await this.openProjectHome();
      } else if (target.kind === "settings") {
        await this.ctx.openSettings(target.section);
      } else if (target.kind === "project") {
        await this.ctx.openProjectCenter(target.section);
      } else if (target.kind === "plugin") {
        await this.switchSection(target.section);
        await this.restoreShellEntity(target.entityId);
        await this.openPluginView(pluginItem!);
        await this.restoreSpecializedSurfaceScroll(target);
      } else {
        await this.switchSection(target.section);
        if (target.section === "lore") {
          await this.openLoreLibrary();
        } else if (target.section === "timeline") {
          await this.switchTimelineView(target.timelineView);
        } else if (target.section === "writing") {
          await this.switchWritingView(target.writingView);
        } else if (target.section === "houses") {
          if (target.view === "tree") {
            this.treeRootId = target.entityId;
            this.treeRestoreNonce += 1;
          }
          this.restoreModuleState(target.section, target.moduleState);
          await this.switchHousesView(target.view);
        } else if (target.section === "language") {
          this.restoreModuleState(target.section, target.moduleState);
        }
        await tick();
        await this.applyWorkspaceCollectionLocation(target.collection);
        const restoredEntity =
          target.section === "houses" && target.view === "tree" ? null : await this.restoreShellEntity(target.entityId);
        if (target.view === "wiki" || target.view === "graph" || target.view === "timeline") {
          await this.openWorkspaceView(target.view);
        }
        await this.restoreWorkspacePaneDimensions(target.panes);
        await this.restoreSpecializedSurfaceScroll(target);
        const expected =
          (target.section === "houses" && target.view === "tree") || restoredEntity || !target.entityId
            ? target
            : { ...target, entityId: null };
        return sameShellLocation(this.currentShellLocation(), expected);
      }
      return sameShellLocation(this.currentShellLocation(), target);
    } finally {
      this.ctx.setShellNavigationRestoring(false);
    }
  }

  // ---- Entity mutation chrome --------------------------------------------------------------------------------

  clearSavedMutationTimer() {
    if (this.savedMutationTimer) {
      clearTimeout(this.savedMutationTimer);
      this.savedMutationTimer = null;
    }
  }
  scheduleClearSavedMutation() {
    this.clearSavedMutationTimer();
    this.savedMutationTimer = setTimeout(() => {
      this.savedMutationTimer = null;
      if (this.entityMutation.phase === "saved") this.entityMutation.reset();
    }, 1800);
  }

  // ---- Language webview --------------------------------------------------------------------------------------

  /**
   * Attachment for the Language content pane: tracks whether the pane element is in the document.
   * Passed bare to `{@attach}`, so it is an arrow: Svelte calls it without the store as `this`.
   */
  trackLanguageMount = () => {
    const generation = ++this.languageMountGeneration;
    this.languageWebview.mounted = true;
    this.languageWebview.ready = false;
    this.languageHostedInstance = null;
    this.languagePaneRequest = null;
    this.languageLeave.releaseAll();
    return () => {
      if (generation !== this.languageMountGeneration) return;
      this.languageWebview.mounted = false;
      this.languageWebview.ready = false;
      this.languageHostedInstance = null;
      this.languagePaneRequest = null;
      this.languageLeave.releaseAll();
    };
  };

  async postLanguageMessage(message: unknown, instanceId = this.ctx.shownWebviewInstance): Promise<boolean> {
    try {
      await this.ctx.project.postPluginWebviewMessage(LANGUAGE_PLUGIN_ID, instanceId, message);
      return true;
    } catch {
      return false;
    }
  }

  /**
   * Moves the Language workspace to a pane. A mounted, ready webview is told through the host so
   * its leave guards run; the webview may answer with a different pane. Otherwise the next mount
   * reads `languagePane`.
   */
  requestLanguagePane(next: LanguagePane) {
    if (this.languagePane === next) return;
    this.languagePane = next;
    if (!this.languageWebview.mounted || !this.languageWebview.ready) {
      this.languagePaneRequest = null;
      return;
    }
    this.languagePaneRequest = next;
    void this.postLanguageMessage({ type: "language.pane", pane: next }).then((sent) => {
      if (!sent && this.languagePaneRequest === next) this.languagePaneRequest = null;
    });
  }

  /** Asks the Language webview whether it may be torn down. Resolves true when it is not mounted or ready. */
  async canLeaveLanguageSection(): Promise<boolean> {
    if (this.section !== "language" || !this.languageWebview.mounted || !this.languageWebview.ready) return true;
    // Captured before the await. `switchToTab` and `applyTabsChange` call this
    // before they change `shellTabs`, so this is the outgoing tab.
    const instanceId = this.ctx.shownWebviewInstance;
    const requestId = crypto.randomUUID();
    const decision = this.languageLeave.begin(requestId);
    const sent = await this.postLanguageMessage({ type: "language.can-leave", requestId }, instanceId);
    if (!sent) this.languageLeave.reply(requestId, true);
    return decision;
  }

  handleLanguageStateEvent(event: LanguageStateEvent) {
    switch (event.kind) {
      case "ready": {
        if (!this.languageWebview.mounted) return;
        this.languageWebview.ready = true;
        this.ctx.markShownWebviewReady(LANGUAGE_PLUGIN_ID);
        this.languagePaneRequest = null;
        this.languageLeave.releaseAll();
        // The webview mounted with the pane it was given; if the shell moved on meanwhile, the shell wins.
        if (event.pane !== null && event.pane !== this.languagePane)
          void this.postLanguageMessage({ type: "language.pane", pane: this.languagePane });
        return;
      }
      case "pane": {
        if (!this.languageWebview.mounted) return;
        if (event.pane === this.languagePane) {
          this.languagePaneRequest = null;
          return;
        }
        if (this.languagePaneRequest !== null) {
          // Answer to a shell request that the webview refused: revert without a history entry.
          this.languagePaneRequest = null;
          this.languagePane = event.pane;
          return;
        }
        this.ctx.recordShellDeparture(this.currentShellLocation());
        this.languagePane = event.pane;
        return;
      }
      case "leave-pending":
        this.languageLeave.pending(event.requestId);
        return;
      case "leave-reply":
        this.languageLeave.reply(event.requestId, event.allowed);
        return;
      case "entity-changed":
        if (event.deleted) {
          if (this.selected?.id === event.entityId) this.clearSelection();
          void this.ctx.refreshAfterEntityMutation({ entityId: event.entityId, removed: true });
        } else {
          void this.ctx.refreshAfterEntityMutation({ entityId: event.entityId });
        }
        return;
      case "open-link": {
        const href = openableLinkHref(event.href);
        if (href) void this.ctx.openExternal(href);
        return;
      }
      case "open-entity":
        if (event.newTab) void this.ctx.openEntityFromWebview(event.entityId, true);
        else void this.ctx.openEntityFromWebview(event.entityId, false);
        return;
      case "shortcut":
        if (this.languageWebview.mounted) this.ctx.runShellShortcut(event.shortcut);
        return;
    }
  }

  // ---- Writing webview ---------------------------------------------------------------------------------------

  /**
   * Attachment for the Writing content pane: tracks whether the pane element is in the document.
   * Passed bare to `{@attach}`, so it is an arrow: Svelte calls it without the store as `this`.
   */
  trackWritingMount = () => {
    const generation = ++this.writingMountGeneration;
    // The outgoing instance may be fullscreen inside its own webview: end that before the state resets.
    this.endWritingFullscreen();
    this.writingWebview.mounted = true;
    this.writingWebview.ready = false;
    this.writingHostedInstance = null;
    this.writingLoadKey = null;
    this.writingLoad = null;
    this.writingEditablePosted = null;
    this.writingFlush.releaseAll();
    return () => {
      if (generation !== this.writingMountGeneration) return;
      this.endWritingFullscreen();
      this.writingWebview.mounted = false;
      this.writingWebview.ready = false;
      this.writingHostedInstance = null;
      this.writingLoadKey = null;
      this.writingLoad = null;
      this.writingEditablePosted = null;
      this.writingFlush.releaseAll();
    };
  };

  async postWritingMessage(message: unknown, instanceId = this.ctx.shownWebviewInstance): Promise<boolean> {
    try {
      await this.ctx.project.postPluginWebviewMessage(WRITING_PLUGIN_ID, instanceId, message);
      return true;
    } catch {
      return false;
    }
  }

  writingPlaceholder() {
    return this.selected?.entity_type === "daena.writing:reference-page"
      ? "Write this reference page…"
      : "Write your manuscript…";
  }

  /**
   * Applies a draft the way typing in the shell editor would: `updateDocumentBody` marks the entry dirty, the
   * autosave timer saves the document and field edits in one `saveEntry`, and conflicts stay in the shell. The
   * webview cannot write anything itself. Resolves true when the text was taken.
   */
  async applyWritingDraft(draft: WritingBridgeDraft): Promise<boolean> {
    // A transfer is claimed first so a rejected draft does not leave its text waiting.
    const text =
      "text" in draft ? draft.text : await this.ctx.project.takePluginTextTransfer(WRITING_PLUGIN_ID, draft.transfer);
    if (!writingMessageCurrent(draft, this.selected?.id, this.writingLoad)) return false;
    if (this.selectedLoading || this.selectedLoadError || this.ctx.projectDiagnostics.length > 0) return false;
    if (text === this.documentBody) return false;
    this.updateDocumentBody(text);
    return true;
  }

  enqueueWritingDraft(draft: WritingBridgeDraft): Promise<boolean> {
    const applied = this.writingDraftQueue
      .then(() => this.applyWritingDraft(draft))
      .catch((cause) => {
        console.warn("A Writing draft could not be applied", cause);
        return false;
      });
    this.writingDraftQueue = applied;
    return applied;
  }

  /**
   * Collects the Writing webview's unsent text before the shell saves, leaves or reloads: asks for it, waits
   * (a little) for the answer and applies it. Anything published before the answer is applied first. Resolves
   * true when it handed the shell text that changed the document.
   */
  async flushWritingWebview(): Promise<boolean> {
    if (!this.writingWebview.mounted || !this.writingWebview.ready || this.writingLoad === null) return false;
    // The flush post runs before this function awaits. Callers (`flushAutoSave`
    // via `switchToTab` / `applyTabsChange`) run that before `shellTabs` changes,
    // so the post addresses the outgoing tab. The default instance is read then.
    const draft = await this.writingFlush.request();
    const applied = draft ? await this.enqueueWritingDraft(draft) : false;
    await this.writingDraftQueue;
    return applied;
  }

  async postWritingLoad(entityId: string, key: string) {
    const instanceId = this.ctx.shownWebviewInstance;
    // Reloading the entry the webview already shows: its unsent text reaches the shell before the editor is
    // replaced, and if there was any it wins, so the webview keeps its editor and the shell saves that text.
    if (this.writingLoad?.entityId === entityId && (await this.flushWritingWebview())) {
      // The editor stays, but a rename may have changed the header, which a load would otherwise have refreshed.
      if (this.selected?.id === entityId)
        void this.postWritingMessage(writingTitleMessage(entityId, this.selected.name), instanceId);
      return;
    }
    if (this.writingLoadKey !== key) return;
    const load: WritingLoad = { entityId, token: crypto.randomUUID() };
    this.writingLoad = load;
    this.writingEditablePosted = this.writingEditable;
    const sent = await this.postWritingMessage(
      writingLoadMessage(entityId, load.token, this.loadedDocumentRevision, {
        editable: this.writingEditable,
        placeholder: this.writingPlaceholder(),
        mode: this.documentMode,
      }),
      instanceId,
    );
    if (sent) return;
    if (this.writingLoadKey === key) this.writingLoadKey = null;
    if (this.writingLoad === load) this.writingLoad = null;
  }

  /** Tells the webview its entry was saved; the revision is informational and does not change its token. */
  notifyWritingSaved(entityId: string) {
    if (!this.writingWebview.ready || this.writingLoad?.entityId !== entityId) return;
    void this.postWritingMessage({ type: "writing.saved", entityId, revision: this.loadedDocumentRevision });
  }

  /** Runs the shell's own conflict action the user picked in the webview banner. */
  handleWritingConflictAction(event: Extract<WritingStateEvent, { kind: "conflict-action" }>) {
    if (!writingMessageCurrent(event, this.selected?.id, this.writingLoad) || !this.documentConflict) return;
    switch (event.action) {
      case "reload":
        void this.reloadConflict();
        return;
      case "overwrite":
        // The shell's own button is disabled while the canonical source needs attention.
        if (this.documentConflict.diagnostics.length === 0) void this.overwriteConflict();
        return;
      case "recovery":
        void this.saveConflictRecoveryCopy();
        return;
    }
  }

  /** Ends the Writing webview's full screen from the shell's side, and tells the webview to leave its own. */
  exitWritingFullscreen() {
    if (!this.writingFullscreen) return;
    this.endWritingFullscreen();
  }

  /**
   * Ends the full screen and tells the instance that reported it, which is not always the one on screen: the effect
   * below runs after a tab switch has already changed `shownWebviewInstance`.
   */
  endWritingFullscreen() {
    const instance = this.writingFullscreenInstance;
    this.writingFullscreen = false;
    this.writingFullscreenInstance = null;
    if (instance) void this.postWritingMessage(writingFullscreenMessage(false), instance);
  }

  /**
   * Writing state from the webview. Each entry-scoped message is checked against the selection, the way the shell's
   * own actions are; drafts and conflict actions also carry the load token (see writingMessageCurrent).
   */
  handleWritingStateEvent(event: WritingStateEvent) {
    switch (event.kind) {
      case "ready":
        if (!this.writingWebview.mounted) return;
        // A webview that (re)started has shown nothing yet, so the effect above posts the selection again.
        this.writingLoadKey = null;
        this.writingLoad = null;
        this.writingEditablePosted = null;
        this.writingFullscreen = false;
        this.writingFullscreenInstance = null;
        this.writingFlush.releaseAll();
        this.writingWebview.ready = true;
        this.ctx.markShownWebviewReady(WRITING_PLUGIN_ID);
        this.writingReadyCount += 1;
        return;
      case "draft":
        if (this.writingWebview.mounted) void this.enqueueWritingDraft(event.draft);
        return;
      case "flush-reply":
        // A reply nobody waits for any more (it came after the timeout) still carries text, which goes
        // through the same checks as any other draft.
        if (!this.writingFlush.reply(event.requestId, event.draft) && event.draft && this.writingWebview.mounted)
          void this.enqueueWritingDraft(event.draft);
        return;
      case "conflict-action":
        if (this.writingWebview.mounted) this.handleWritingConflictAction(event);
        return;
      case "open-link": {
        const href = openableLinkHref(event.href);
        if (href) void this.ctx.openExternal(href);
        return;
      }
      case "open-entity":
        if (event.newTab) void this.ctx.openEntityFromWebview(event.entityId, true);
        else void this.ctx.openEntityFromWebview(event.entityId, false);
        return;
      case "fullscreen":
        // Only the entry on screen can change the full screen; a message for any other entry is stale.
        if (this.writingWebview.mounted && event.entityId === this.selected?.id) {
          this.writingFullscreen = event.on;
          this.writingFullscreenInstance = event.on ? this.ctx.shownWebviewInstance : null;
        }
        return;
      case "shortcut":
        if (this.writingWebview.mounted) this.ctx.runShellShortcut(event.shortcut);
        return;
      case "request-rename":
        if (this.writingWebview.mounted && this.selected && event.entityId === this.selected.id)
          void this.ctx.openEntityEditDialog(this.selected);
        return;
      case "request-archive":
        if (this.writingWebview.mounted) void this.ctx.confirmWritingArchive(event.entityId);
        return;
      default:
        return;
    }
  }

  // ---- Map editor session ------------------------------------------------------------------------------------

  /**
   * Asks before leaving a dirty vector map. No map, a clean map, or a physical map leaves without asking.
   * Saving goes through the native vector session; declining the save asks whether to discard.
   */
  async resolveDirtyMapSession(): Promise<boolean> {
    const mapId = this.currentMapId();
    if (!mapId || this.sandboxView?.renderer !== "maps" || this.mapSaveStates[mapId]?.status !== "dirty") return true;
    if (this.mapsEditorMode === "physical") return true;
    if (
      await this.ctx.confirmDialog({
        title: "Save changes to this map before leaving?",
        message: "Your map has unsaved edits.",
        confirmLabel: "Save",
      })
    ) {
      try {
        await this.ctx.nativeVectorSession()?.save();
        return this.ctx.nativeVectorSession()?.isDirty() !== true && this.mapSaveStates[mapId]?.status !== "dirty";
      } catch (cause) {
        this.ctx.setError(friendlyError(cause));
        return false;
      }
    }
    return this.ctx.confirmDialog({
      title: "Discard unsaved map edits?",
      message: "Choose Cancel to remain in the editor.",
      confirmLabel: "Discard",
      danger: true,
    });
  }

  currentMapId() {
    return this.selected?.entity_type === "daena.maps:world-map" ? this.selected.id : null;
  }

  clearMapFocus() {
    this.mapFocusLinkId = null;
    this.mapFocusFeatureId = null;
  }

  /** Clears the map session. `resetProjectSessionState` on the page calls this. Modes and the provider menu stay. */
  resetMapEditorSession() {
    this.mapSaveStates = {};
    this.mapsEditorKey = "welcome";
    this.mapReloadCounter = 0;
    this.mapRecoveryBusy = false;
    this.mapFocusLinkId = null;
    this.mapFocusFeatureId = null;
    this.mapSelection = null;
    this.mapPickPending = null;
    this.mapReconcileNotice = "";
    this.mapPickNotice = "";
  }

  async createMap(provider: "image" | "vector" | "physical" = "physical") {
    if (this.ctx.projectDiagnostics.length > 0) return;
    try {
      if (!(await this.flushAutoSave())) return;
      const departure = this.currentShellLocation();
      this.mapProviderMenuOpen = null;
      const mapView = this.ctx.mapsNavigationItem();
      if (!mapView) throw new Error("The Maps plugin view is not available");
      if (!(await this.ctx.dismissSettings())) return;
      if (!(await this.leavePluginView())) return;
      this.ctx.recordShellDeparture(departure);
      this.selected = null;
      this.fields = {};
      this.relationships = [];
      this.assets = [];
      this.mapLocations = [];
      this.mapFocusLinkId = null;
      this.mapFocusFeatureId = null;
      this.projectHomeOpen = false;
      this.loreWikiOpen = false;
      this.section = "maps";
      // Draft editor: no map entity until the native editor accepts/creates one.
      this.mapsEditorMode = provider === "physical" ? "physical" : "vector";
      this.mapsVectorStart = provider === "image" ? "import" : "geojson";
      this.mapsEditorKey = `draft-${Date.now()}`;
      this.sandboxView = { plugin: mapView.plugin, view: mapView.view, renderer: "maps" };
    } catch (cause) {
      this.ctx.setError(friendlyError(cause));
    }
  }

  /** Passed bare to the conflict banner's click handler. */
  reloadMapOriginal = () => {
    if (!this.currentMapId()) return;
    this.mapReloadCounter += 1;
  };

  /** Passed bare to the conflict banner's click handler. */
  restoreMapDraft = async () => {
    const mapId = this.currentMapId();
    if (!mapId || this.mapRecoveryBusy) return;
    this.mapRecoveryBusy = true;
    try {
      const copies = await this.ctx.project.mapsRecoveryList(mapId);
      if (copies.length === 0) throw new Error("No recovery copy was found for this map.");
      await this.ctx.project.mapsRecoveryRestore(mapId, copies[0].fileName);
      this.mapSaveStates[mapId] = { status: "restoring", detail: null };
      this.mapReloadCounter += 1;
    } catch (cause) {
      this.ctx.setError(friendlyError(cause));
    } finally {
      this.mapRecoveryBusy = false;
    }
  };

  async openMapLocation(location: MapLocation) {
    try {
      const mapEntityId = location.mapEntityId;
      if (!mapEntityId) throw new Error("map-unavailable: this location is missing its map id");
      await this.ctx.project.mapsNavigation("openMap", { mapEntityId, linkId: location.id });
    } catch (cause) {
      const message = friendlyError(cause);
      if (message.includes("link-unresolved")) {
        this.mapReconcileNotice = `This link is unresolved — the map feature it pointed to was removed or renumbered.`;
        return;
      }
      this.ctx.setError(message);
    }
  }

  async ensureMapEditorOpen(mapEntityId: string) {
    const departure = this.currentShellLocation();
    const map =
      this.ctx.entities.find((entity) => entity.id === mapEntityId) ?? (await this.ctx.project.getEntity(mapEntityId));
    if (!map) throw new Error("map-unavailable: choose a saved map first");
    if (!(await this.flushAutoSave())) throw new Error("Save the current draft before opening the map editor.");
    const mapsView = this.ctx.mapsNavigationItem();
    this.selected = map;
    this.mapsEditorKey = map.id;
    await this.loadSelectedState(map);
    if (mapsView) await this.openPluginView(mapsView, departure);
  }

  async beginMapPick(pending: MapPickPending) {
    this.mapPickPending = pending;
    this.mapPickNotice =
      pending.kind === "rebind"
        ? "Click the map to rebind this location."
        : "Click for a point, or use Path/Area to draw a route or region.";
    await this.ensureMapEditorOpen(pending.mapEntityId);
    if (this.mapsEditorMode === "vector") return;
  }

  async applyMapPick(anchor: unknown) {
    const pending = this.mapPickPending;
    this.mapPickPending = null;
    this.mapPickNotice = "";
    if (!pending || !anchor) return;
    try {
      if (pending.kind === "link") {
        const entity =
          this.ctx.entities.find((candidate) => candidate.id === pending.entityId) ??
          (await this.ctx.project.getEntity(pending.entityId));
        if (!entity) throw new Error("Choose an entity to link.");
        const location: MapLocation = {
          id: crypto.randomUUID(),
          mapEntityId: pending.mapEntityId,
          role: pending.role,
          label: entity.name,
          anchor,
          validity: { from: null, to: null },
        };
        await this.ctx.project.upsertMapLocation(entity.id, location);
        const departure = this.currentShellLocation();
        if (!(await this.leavePluginView())) return;
        this.ctx.recordShellDeparture(departure);
        this.section =
          entity.entity_type === "daena.timeline:event" ||
          entity.entity_type === "daena.timeline:encounter" ||
          entity.entity_type === "daena.timeline:era" ||
          entity.entity_type === "daena.timeline:calendar"
            ? "timeline"
            : "lore";
        await this.selectEntity(entity, false);
        this.mapLocations = await this.ctx.project.listMapLocations(entity.id);
      } else {
        await this.ctx.project.upsertMapLocation(pending.entityId, { ...pending.location, anchor });
        const entity =
          this.ctx.entities.find((candidate) => candidate.id === pending.entityId) ??
          (await this.ctx.project.getEntity(pending.entityId));
        if (entity) {
          const departure = this.currentShellLocation();
          if (!(await this.leavePluginView())) return;
          this.ctx.recordShellDeparture(departure);
          this.section =
            entity.entity_type === "daena.timeline:event" ||
            entity.entity_type === "daena.timeline:encounter" ||
            entity.entity_type === "daena.timeline:era" ||
            entity.entity_type === "daena.timeline:calendar"
              ? "timeline"
              : "lore";
          await this.selectEntity(entity, false);
          this.mapLocations = await this.ctx.project.listMapLocations(entity.id);
        }
      }
    } catch (cause) {
      this.ctx.setError(friendlyError(cause));
    }
  }

  async openMapEntityFromLink(entityId: string) {
    try {
      const entity = await this.ctx.project.getEntity(entityId);
      if (!entity) throw new Error("Linked entity was not found.");
      if (!this.ctx.entities.some((candidate) => candidate.id === entity.id)) this.ctx.appendEntityToCache(entity);
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
      const departure = this.currentShellLocation();
      if (!(await this.leavePluginView())) return;
      this.ctx.recordShellDeparture(departure);
      this.section = target;
      await this.selectEntity(entity, false);
    } catch (cause) {
      this.ctx.setError(friendlyError(cause));
    }
  }

  async openSelectedMapEditor() {
    if (!this.selected || this.selected.entity_type !== "daena.maps:world-map") return;
    const mapsView = this.ctx.mapsNavigationItem();
    if (!mapsView) {
      this.ctx.setError("The Maps integration is not available.");
      return;
    }
    try {
      this.mapsEditorKey = this.selected.id;
      this.mapFocusLinkId = null;
      this.mapFocusFeatureId = null;
      await this.openPluginView(mapsView);
    } catch (cause) {
      this.ctx.setError(friendlyError(cause));
    }
  }

  /** Body of the page's `maps-navigation` listener. The listener itself stays on the page. */
  async handleMapsNavigation(payload: { mapEntityId: string; linkId?: string }) {
    try {
      let map = this.ctx.entities.find((entity) => entity.id === payload.mapEntityId && !entity.deleted) ?? null;
      if (!map) {
        const loaded = await this.ctx.project.getEntity(payload.mapEntityId);
        if (loaded && !loaded.deleted) {
          this.ctx.upsertEntityInCache(loaded);
          map = loaded;
        }
      }
      const item = this.ctx.mapsNavigationItem();
      if (!map || !item) throw new Error("map-unavailable: enable the Maps module to open this location");
      if (!(await this.flushAutoSave())) return;
      if (!(await this.canLeaveLanguageSection())) return;
      const departure = this.currentShellLocation();
      if (!(await this.leavePluginView())) return;
      this.selected = map;
      this.mapsEditorKey = map.id;
      await this.loadSelectedState(map);
      await this.openPluginView(item, departure);
      const linkId = payload.linkId ?? null;
      // openPluginView clears focus; also re-assert when already on the same map so
      // the vector editor retries focus after pins load.
      this.mapFocusLinkId = null;
      this.mapFocusFeatureId = null;
      await tick();
      this.mapFocusLinkId = linkId;
    } catch (cause) {
      this.ctx.setError(friendlyError(cause));
    }
  }

  /** Body of the page's `maps-state` listener, after the foreign-instance check. */
  handleMapsState(payload: { mapEntityId: string; status: string; detail: unknown; instanceId?: unknown }) {
    const { mapEntityId, status, detail } = payload;
    if (status === "fullscreen") {
      const enabled = typeof detail === "object" && detail !== null && "enabled" in detail && detail.enabled === true;
      this.editorFullscreen = enabled;
      return;
    }
    if (status === "back") {
      this.editorFullscreen = false;
      void this.closePluginView();
      return;
    }
    if (mapEntityId) this.mapSaveStates[mapEntityId] = { status, detail };
    if (status === "saved" && mapEntityId) {
      void this.ctx.refreshAfterEntityMutation({ entityId: mapEntityId }).then(() => {
        const map = this.ctx.entities.find((entity) => entity.id === mapEntityId && !entity.deleted);
        if (map) {
          this.selected = map;
          void this.loadSelectedState(map).catch(() => {});
        }
      });
    }
    if (status === "reconcile") {
      const reconcileDetail = detail as { unresolved?: unknown } | null;
      const unresolved = Array.isArray(reconcileDetail?.unresolved) ? reconcileDetail.unresolved.length : 0;
      this.mapReconcileNotice =
        unresolved > 0
          ? `${unresolved} link${unresolved === 1 ? "" : "s"} on this map are unresolved — the features they pointed to were removed or renumbered.`
          : "";
      if (this.selected && this.mapLocations.length > 0)
        void this.ctx.project
          .listMapLocations(this.selected.id)
          .then((locations) => {
            this.mapLocations = locations;
          })
          .catch(() => {});
    }
    if (status === "pick-complete") {
      const pickDetail = detail as { anchor?: unknown } | null;
      void this.applyMapPick(pickDetail?.anchor ?? null);
    }
    if (status === "pick-cancelled") {
      this.mapPickPending = null;
      this.mapPickNotice = "";
    }
    if (status === "open-entity") {
      const openDetail = detail as { entityId?: string } | null;
      if (openDetail?.entityId) void this.openMapEntityFromLink(openDetail.entityId);
    }
    if (status === "linked") {
      void this.ctx.refreshAfterEntityMutation(mapEntityId ? { entityId: mapEntityId } : undefined);
      if (this.selected)
        void this.ctx.project
          .listMapLocations(this.selected.id)
          .then((locations) => {
            this.mapLocations = locations;
          })
          .catch(() => {});
    }
  }

  /** Body of the page's `maps-selection` listener, after the foreign-instance check. */
  handleMapsSelection(payload: { mapEntityId: string; anchor: unknown; instanceId?: unknown }) {
    if (payload.mapEntityId === this.currentMapId()) this.mapSelection = payload.anchor;
  }

  // ---- Selection AI ------------------------------------------------------------------------------------------

  /** Passed bare to the shell editor's selection callback. */
  setAiSelection = (markdown: string, plainText: string) => {
    if (this.aiRewriteOpen || this.aiBusy || this.aiPreviewOutput) return;
    this.aiSourceSelection = markdown;
    this.aiSourceSelectionPlain = plainText;
  };

  /** Passed bare to the shell editor's AI menu callback. */
  openAiAction = (action: string, markdown: string, plainText: string, context: string) => {
    if (!this.ctx.projectInfo?.aiEnabled) return;
    const request = aiActionRequest(this.ctx.aiPromptTemplates, action, markdown, plainText, context);
    if (!request) return;
    this.aiSourceSelection = request.selectionMarkdown;
    this.aiSourceSelectionPlain = request.selectionPlain;
    this.aiGenerationContext = request.generationContext;
    this.aiMode = request.mode;
    this.aiInstruction = instructionFor(this.ctx.aiPromptTemplates as PromptTemplate[], action);
    this.aiRewriteOpen = true;
  };

  clearAiFieldListener() {
    this.aiFieldUnlisten?.();
    this.aiFieldUnlisten = null;
  }

  /** Passed bare to the field-fill panel's close button. */
  closeAiFieldFill = () => {
    if (this.aiFieldFillRequestId) void this.ctx.project.aiCancelText(this.aiFieldFillRequestId).catch(() => {});
    this.aiFieldFillStartCancelled = true;
    this.clearAiFieldListener();
    this.aiFieldFillOpen = false;
    this.aiFieldFillBusy = false;
    this.aiFieldFillRequestId = null;
    this.aiFieldFillEntityId = null;
    this.aiFieldFillLastSequence = -1;
    this.aiFieldFillStream = "";
    this.aiFieldSuggestions = {};
  };

  handleAiFieldFillEvent(payload: AiStreamEvent) {
    if (this.aiFieldFillEntityId !== null && this.selected?.id !== this.aiFieldFillEntityId) return;
    if (payload.sequence <= this.aiFieldFillLastSequence) return;
    this.aiFieldFillLastSequence = payload.sequence;
    if (payload.phase === "delta" && payload.delta) this.aiFieldFillStream += payload.delta;
    if (payload.phase === "failed") {
      this.clearAiFieldListener();
      this.aiFieldFillBusy = false;
      this.aiFieldFillRequestId = null;
      this.ctx.setError(payload.error ?? "AI field suggestions failed");
    } else if (payload.phase === "cancelled" || payload.phase === "deadline_exceeded") {
      this.clearAiFieldListener();
      this.aiFieldFillBusy = false;
      this.aiFieldFillRequestId = null;
      if (payload.phase === "deadline_exceeded") {
        this.ctx.setError("AI field generation reached its time limit before returning complete suggestions.");
      }
    } else if (payload.phase === "completed") {
      this.clearAiFieldListener();
      this.aiFieldFillBusy = false;
      this.aiFieldFillRequestId = null;
      try {
        const parsed = JSON.parse(payload.output ?? this.aiFieldFillStream) as {
          suggestions?: Record<
            string,
            { value?: unknown; values?: unknown; rationale?: unknown; confidence?: unknown }
          >;
        };
        const allowed = new Set(this.ctx.emptyInspectorDefinitions().map((definition) => definition.key));
        const suggestions: Record<string, AiFieldSuggestion> = {};
        for (const [key, value] of Object.entries(parsed.suggestions ?? {})) {
          const definition = this.ctx.definitions().find((candidate) => candidate.key === key);
          if (!definition || !allowed.has(key)) continue;
          const rawValue = definition.multiple || definition.type === "relationship" ? value?.values : value?.value;
          const normalizedValue = coerceAiFieldValue(definition, rawValue);
          if (normalizedValue === null) continue;
          if (definition?.type === "relationship") {
            const allowedIds = new Set(
              this.ctx.entities
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
        this.aiFieldSuggestions = suggestions;
        if (Object.keys(suggestions).length === 0) {
          this.aiFieldFillOpen = false;
          this.ctx.setError("AI found no supported suggestions for the empty fields.");
        }
      } catch {
        this.aiFieldFillOpen = false;
        this.ctx.setError("AI returned invalid field suggestions.");
      }
    }
  }

  async fillAiFields() {
    if (!this.selected || this.aiFieldFillBusy) return;
    if (!this.ctx.projectInfo?.aiEnabled) return;
    const empty = this.ctx.emptyInspectorDefinitions();
    if (empty.length === 0) return;
    const endpoint = this.ctx.aiProvider.endpoint.trim();
    const model = this.ctx.aiProvider.model.trim();
    if (!endpoint || !model) {
      this.ctx.setError("Configure the active provider endpoint and model before filling fields.");
      return;
    }
    this.aiFieldFillOpen = true;
    this.aiFieldFillBusy = true;
    this.aiFieldFillStream = "";
    this.aiFieldSuggestions = {};
    this.aiFieldFillLastSequence = -1;
    this.aiFieldFillStartCancelled = false;
    this.aiFieldFillEntityId = this.selected.id;
    const fieldKeys = empty.map((definition) => definition.key);
    const context = JSON.stringify({
      entity: { name: this.selected.name, type: this.selected.entity_type },
      document: this.documentBody,
      populatedFields: Object.fromEntries(Object.entries(this.fields).filter(([, value]) => !isEmptyFieldValue(value))),
      emptyFields: empty.map((definition) => ({
        key: definition.key,
        label: definition.label,
        type: definition.type,
        multiple: definition.multiple ?? definition.type === "relationship",
        options: definition.options ?? [],
        allowedEntities:
          definition.type === "relationship"
            ? this.ctx.entities
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
      `${this.selected.name} ${this.selected.entity_type ?? ""} ${empty.map((definition) => definition.label).join(" ")}`.slice(
        0,
        4000,
      );
    let startedRequestId: string | null = null;
    try {
      const requestId = await this.ctx.project.aiGenerateStructured(
        this.ctx.projectInfo!.root,
        `Fill only these empty fields: ${fieldKeys.join(", ")}. For multi-select and relationship fields, return up to five distinct values in the values array. For relationship fields, use only allowed entity IDs from the context. Use only configured options when options are provided. Return evidence-backed suggestions. Do not invent facts.`,
        context,
        outputContract,
        this.aiFieldFillEntityId!,
        retrievalQuery,
        2,
      );
      startedRequestId = requestId;
      if (this.aiFieldFillStartCancelled || this.selected?.id !== this.aiFieldFillEntityId) {
        void this.ctx.project.aiCancelText(requestId).catch(() => {});
        return;
      }
      this.aiFieldFillRequestId = requestId;
      this.aiFieldUnlisten = await this.ctx.listenAiStream(requestId, (payload) =>
        this.handleAiFieldFillEvent(payload),
      );
      const buffered = await this.ctx.project.aiPollText(requestId);
      for (const event of buffered) this.handleAiFieldFillEvent(event);
    } catch (cause) {
      if (startedRequestId) void this.ctx.project.aiCancelText(startedRequestId).catch(() => {});
      this.clearAiFieldListener();
      this.aiFieldFillBusy = false;
      this.aiFieldFillRequestId = null;
      if (this.aiFieldFillStartCancelled || this.selected?.id !== this.aiFieldFillEntityId) return;
      this.ctx.setError(friendlyError(cause));
    }
  }

  async acceptAiFieldSuggestion(key: string) {
    if (this.selected?.id !== this.aiFieldFillEntityId) return;
    const suggestion = this.aiFieldSuggestions[key];
    const definition = this.ctx.definitions().find((candidate) => candidate.key === key);
    if (
      !suggestion ||
      (definition?.type === "relationship"
        ? this.ctx.selectedRelationshipIds(definition).length > 0
        : !isEmptyFieldValue(this.fields[key]))
    )
      return;
    if (definition?.type === "relationship")
      await this.updateRelationshipField(definition, suggestion.value as string[]);
    else this.fields = { ...this.fields, [key]: suggestion.value };
    const remaining = { ...this.aiFieldSuggestions };
    delete remaining[key];
    this.aiFieldSuggestions = remaining;
    this.markEntryDirty({ fieldKey: key });
  }

  async acceptAllAiFieldSuggestions() {
    for (const key of Object.keys(this.aiFieldSuggestions)) await this.acceptAiFieldSuggestion(key);
    if (Object.keys(this.aiFieldSuggestions).length === 0) this.closeAiFieldFill();
  }

  discardAiFieldSuggestion(key: string) {
    const remaining = { ...this.aiFieldSuggestions };
    delete remaining[key];
    this.aiFieldSuggestions = remaining;
  }

  clearAiStreamListener() {
    this.aiUnlisten?.();
    this.aiUnlisten = null;
  }

  /** Passed bare to the rewrite dialog's discard and cancel buttons. */
  closeAiRewrite = () => {
    if (this.aiRequestId) void this.ctx.project.aiCancelText(this.aiRequestId).catch(() => {});
    this.aiStartCancelled = true;
    this.clearAiStreamListener();
    this.aiRewriteOpen = false;
    this.aiBusy = false;
    this.aiCancelPending = false;
    this.aiRequestId = null;
    this.aiSourceEntityId = null;
    this.aiStreamText = "";
    this.aiPreviewOutput = "";
    this.aiProgressMessage = "Preparing model…";
    this.aiUsage = null;
    this.aiSourceSelection = "";
    this.aiSourceSelectionPlain = "";
    this.aiGenerationContext = "";
    this.aiLastSequence = -1;
    this.aiMode = "rewrite";
  };

  async cancelAiRewrite() {
    if (!this.aiBusy || this.aiCancelPending) return;
    if (!this.aiRequestId) {
      this.closeAiRewrite();
      return;
    }
    this.aiCancelPending = true;
    try {
      await this.ctx.project.aiCancelText(this.aiRequestId);
    } catch (cause) {
      this.aiCancelPending = false;
      this.ctx.setError(friendlyError(cause));
    }
  }

  handleAiEvent(payload: AiStreamEvent) {
    if (this.aiSourceEntityId !== null && this.selected?.id !== this.aiSourceEntityId) return;
    if (payload.sequence <= this.aiLastSequence) return;
    this.aiLastSequence = payload.sequence;
    const nextStreamState = reduceTextGenerationEvent(
      {
        streamText: this.aiStreamText,
        proposal: this.aiPreviewOutput,
        progressMessage: this.aiProgressMessage,
      },
      payload,
    );
    this.aiStreamText = nextStreamState.streamText;
    this.aiPreviewOutput = nextStreamState.proposal;
    this.aiProgressMessage = nextStreamState.progressMessage;
    if (payload.phase === "usage" && payload.output) {
      try {
        this.aiUsage = JSON.parse(payload.output);
      } catch (_) {
        this.aiUsage = null;
      }
    }
    if (payload.phase === "completed") {
      this.aiBusy = false;
      this.aiCancelPending = false;
      this.aiRequestId = null;
      this.clearAiStreamListener();
    } else if (payload.phase === "cancelled" || payload.phase === "deadline_exceeded") {
      this.aiBusy = false;
      this.aiCancelPending = false;
      this.aiRequestId = null;
      this.clearAiStreamListener();
      if (payload.phase === "deadline_exceeded") {
        this.ctx.setError(
          this.aiPreviewOutput
            ? "AI generation reached its time limit. The partial proposal is preserved below."
            : "AI generation reached its time limit before producing a proposal.",
        );
      }
    } else if (payload.phase === "failed") {
      this.aiBusy = false;
      this.aiCancelPending = false;
      this.aiRequestId = null;
      this.clearAiStreamListener();
      this.ctx.setError(payload.error ?? "The AI provider could not generate a proposal");
    }
  }

  async startAiRewrite() {
    if (
      !this.selected ||
      (this.aiMode === "rewrite" && !this.aiSourceSelection.trim()) ||
      !this.aiInstruction.trim() ||
      this.aiBusy
    )
      return;
    const sourceSelection = this.aiSourceSelection;
    const sourcePlain = this.aiSourceSelectionPlain;
    const generationContext = this.aiGenerationContext;
    if (!(await this.flushAutoSave())) return;
    if (this.aiMode === "rewrite" && !sourceSelection.trim()) return;
    this.aiSourceEntityId = this.selected.id;
    this.aiSourceBody = this.documentBody;
    this.aiSourceRevision = this.loadedDocumentRevision;
    this.aiStreamText = "";
    this.aiPreviewOutput = "";
    this.aiProgressMessage = "Preparing model…";
    this.aiLastSequence = -1;
    this.aiStartCancelled = false;
    this.aiBusy = true;
    this.aiCancelPending = false;
    let startedRequestId: string | null = null;
    try {
      const sourceText =
        this.aiMode === "generate"
          ? generationContext.trim() || this.documentBody.trim() || "[CURSOR]"
          : sourceSelection;
      const retrievalQuery = [this.selected?.name, this.aiInstruction, sourcePlain]
        .filter(Boolean)
        .join(" ")
        .slice(0, 4000);
      const requestId = await this.ctx.project.aiGenerateText(
        this.ctx.projectInfo!.root,
        this.aiInstruction,
        sourceText,
        this.aiSourceEntityId,
        retrievalQuery,
        2,
      );
      startedRequestId = requestId;
      if (this.aiStartCancelled || this.selected?.id !== this.aiSourceEntityId) {
        void this.ctx.project.aiCancelText(requestId).catch(() => {});
        return;
      }
      this.aiRequestId = requestId;
      this.aiUnlisten = await this.ctx.listenAiStream(requestId, (payload) => {
        this.handleAiEvent(payload);
      });
      const buffered = await this.ctx.project.aiPollText(requestId);
      for (const event of buffered) this.handleAiEvent(event);
    } catch (cause) {
      if (startedRequestId) void this.ctx.project.aiCancelText(startedRequestId).catch(() => {});
      this.clearAiStreamListener();
      this.aiBusy = false;
      this.aiCancelPending = false;
      this.aiRequestId = null;
      if (this.aiStartCancelled || this.selected?.id !== this.aiSourceEntityId) return;
      this.ctx.setError(friendlyError(cause));
    }
  }

  async acceptAiRewrite() {
    if (!this.selected || !this.aiPreviewOutput || this.aiBusy) return;
    if (
      this.selected.id !== this.aiSourceEntityId ||
      this.documentBody !== this.aiSourceBody ||
      this.loadedDocumentRevision !== this.aiSourceRevision
    ) {
      this.ctx.setError(
        "The entity or document changed while the rewrite was being prepared. Discard it and try again.",
      );
      return;
    }
    const validationError = validateAiProposal(this.aiPreviewOutput);
    if (validationError) {
      this.ctx.setError(validationError);
      return;
    }
    if (this.aiMode === "generate") {
      const nextMarkdown = this.editorRef?.insertAiTextAtRequest(this.aiPreviewOutput);
      if (nextMarkdown == null) {
        this.ctx.setError("The editor position is no longer available. Discard it and try again.");
        return;
      }
      this.documentBody = nextMarkdown;
      this.markEntryDirty({ document: true });
      if (await this.saveDocument()) this.closeAiRewrite();
      return;
    }
    const nextMarkdown = this.editorRef?.replaceAiTextWithMarkdown(this.aiPreviewOutput);
    if (nextMarkdown === null || nextMarkdown === undefined) {
      this.ctx.setError("The selected text is no longer at the original position. Discard it and try again.");
      return;
    }
    const bodyBefore = this.documentBody;
    this.documentBody = nextMarkdown;
    this.markEntryDirty({ document: true });
    if (await this.saveDocument()) {
      this.closeAiRewrite();
    } else {
      this.documentBody = bodyBefore;
    }
  }
}
