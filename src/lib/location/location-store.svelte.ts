/**
 * Everything the shell keeps per location: which entry is open, its document, field drafts, revisions, dirty
 * tracking, the autosave timer and the save queue, and the conflict state.
 *
 * `+page.svelte` builds exactly one store today (side-by-side will build one per visible group). The store never
 * imports the page. Shell-global state (the project, the entity cache, modules, dialogs, toasts) and page functions
 * that have not moved yet reach it through the `LocationContext` the page passes in.
 */
import type { Asset, Entity, MapLocation, Relationship, project } from "$lib/project/client";
import type { FieldDefinition, ModuleContext, ModuleManifest, UUID } from "../../../packages/module-api/src/index";
import type { CalendarDefinition } from "../../../packages/modules/timeline/src/calendar";
import type { confirmDialog } from "$lib/dialogs.svelte";
import type { EntityFieldsTabId } from "$lib/shell/entityFields";
import type { EraContext } from "$lib/modules/chronology";
import type { ShellLocation } from "$lib/navigation/history";
import type { WorkspaceSection } from "$lib/modules/workspace";
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

type ProjectClient = typeof project;
type ConfirmOptions = Parameters<typeof confirmDialog>[0];

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
  /** The workspace section this location is in. */
  readonly section: WorkspaceSection;
  /** The plugin surface open in this location, if any. Only the renderer matters here. */
  readonly sandboxView: { readonly renderer: "maps" | "webview" } | null;

  confirmDialog(options: ConfirmOptions): Promise<boolean>;
  /** The shell's single error line. */
  setError(message: string): void;
  /** Clears the shell-global state that follows the selection (create dialog era contexts, diagnostics, create form). */
  resetSelectionShellState(): void;

  // Leave guards and shell navigation.
  canLeaveLanguageSection(): Promise<boolean>;
  leavePluginView(): Promise<boolean>;
  flushWritingWebview(): Promise<boolean>;
  currentShellLocation(): ShellLocation;
  recordShellDeparture(location: ShellLocation): void;
  applyCollectionTabForEntityType(entityType: string | null): void;

  // Selection AI dialogs.
  closeAiFieldFill(): void;
  closeAiRewrite(): void;

  // Module and schema lookups for the location's section.
  contextFor(): ModuleContext;
  contextOwningRelationshipType(relationshipType: string): ModuleContext;
  activeManifest(): ModuleManifest | null;
  activeModuleId(): string;
  definitions(): FieldDefinition[];
  chronologyDateDefinitions(): FieldDefinition[];
  namespaceForField(definition: FieldDefinition): string;
  sectionEnabled(): boolean;
  mapsEnabled(): boolean;

  // Shell refreshes.
  upsertEntityInCache(entity: Entity): void;
  refreshAfterEntityMutation(options?: { entityId?: string; removed?: boolean }): Promise<void>;
  bumpKinshipRefresh(): void;
  loadEraContexts(eraIds: string[]): Promise<EraContext[]>;
  refreshWritingAppearanceLabels(): Promise<void>;
  notifyWritingSaved(entityId: string): void;
  notifyProfileTimelineChanged(eventId: string): void;
  loadProjectStatus(includeGit: boolean): Promise<void>;
}

export function isRevisionConflict(cause: unknown) {
  return friendlyError(cause).toLowerCase().includes("revision conflict");
}

export class LocationStore {
  readonly ctx: LocationContext;
  private readonly stopEffects: () => void;

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

  constructor(ctx: LocationContext) {
    this.ctx = ctx;
    // Effects belong in here, not in the page's init scope: step 5.4 creates and destroys stores at runtime.
    // None have moved yet; later sub-steps add the collection, layout and Writing effects.
    this.stopEffects = $effect.root(() => {});
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
    await this.ctx.flushWritingWebview();
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
        this.ctx.notifyWritingSaved(entityId);
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
    if (this.ctx.section === "language" && !(await this.ctx.canLeaveLanguageSection())) return;
    const departure = this.ctx.currentShellLocation();
    if (this.ctx.section === "maps" && this.ctx.sandboxView?.renderer === "maps" && !(await this.ctx.leavePluginView()))
      return;
    if (recordHistory) this.ctx.recordShellDeparture(departure);
    this.editorFullscreen = false;
    this.selected = entity;
    this.ctx.applyCollectionTabForEntityType(entity.entity_type);
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
      this.ctx.closeAiFieldFill();
      this.ctx.closeAiRewrite();
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
      void this.ctx.refreshWritingAppearanceLabels();
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
        this.ctx.bumpKinshipRefresh();
      }
      if (isEraRelationshipField(definition)) {
        eraContextCache.invalidateMany(targetIds);
        this.eraContexts = await this.ctx.loadEraContexts(targetIds);
        this.hintDateCalendarsFromEras(this.eraContexts, true);
      }
      void this.ctx.refreshWritingAppearanceLabels();
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
}
