import { invoke } from "@tauri-apps/api/core";
import type {
  AcceptedPhysicalMap,
  AiHybridMatch,
  AiIndexRebuildResult,
  AiIndexStatus,
  AiProviderStatus,
  AiStreamEvent,
  AppSettings,
  AppSettingsUpdate,
  Asset,
  AtlasJobStatus,
  AtlasRenderCapabilities,
  AtlasRenderRequest,
  AtlasStudioInspectResult,
  AtlasStudioSessionRequest,
  AtlasStudioSessionStatus,
  DialogSelection,
  Document,
  Entity,
  EntityListQuery,
  EntityPage,
  ExternalChangeReport,
  ExternalImportAnalysisStatus,
  ExternalImportCommitReport,
  ExternalImportLimits,
  ExternalImportPage,
  ExternalImportSourceHandle,
  ExternalImportValidationSummary,
  ExternalImporterDescriptor,
  FieldValue,
  GitChange,
  GitLogEntry,
  GitPreflight,
  GitRemote,
  GitResetResult,
  GitStatus,
  GitToolInfo,
  HostViewData,
  ImageGenerationRequest,
  ImageGenerationStatus,
  ImageProviderDiscovery,
  ImageProviderStatus,
  ImportCandidatePlan,
  ImportMappingOverrides,
  ImportObjectDecision,
  InstalledPluginVersion,
  MapEditApply,
  MapFeatureSearchResult,
  MapLinkMutation,
  MapLocation,
  MapPin,
  MapRecoveryCopy,
  ModuleSchemaEditorState,
  ModuleSchemaOverlay,
  ModuleSchemaOverlayMutationResult,
  MutationOptions,
  PhysicalClimateProducts,
  PhysicalEventMaterializationRequest,
  PhysicalEventMaterializationResult,
  PhysicalEvolutionProducts,
  PhysicalGenerationInput,
  PhysicalHistoricalProducts,
  PhysicalHydrologyProducts,
  PhysicalJobStatus,
  PluginAdminView,
  PluginUpgradePlan,
  ProjectInfo,
  ProjectModuleManifest,
  RasterLayerChange,
  Relationship,
  RemoteCredentialStatus,
  SchemaOverlayPreviewResult,
  VectorLayerDelete,
  VectorSourceReplace,
  WikiPageExportFormat,
} from "./types";

export type * from "./types";
export {
  PHYSICAL_HISTORICAL_PROGRESS_EVENT,
  ATLAS_PROGRESS_EVENT,
  ATLAS_STUDIO_PROGRESS_EVENT,
  EXTERNAL_IMPORT_PROGRESS_EVENT,
} from "./types";

const requestId = (options?: MutationOptions) => options?.requestId ?? crypto.randomUUID();

export const project = {
  openMemory: () => invoke<void>("project_open_memory"),
  openDefault: () => invoke<void>("project_open_default"),
  open: (path: string) => invoke<void>("project_open", { path }),
  openDirectory: (path: string) => invoke<ProjectInfo>("project_open_directory", { path }),
  pickDirectory: (title?: string) =>
    invoke<DialogSelection>("plugin:dialog|open", {
      options: { directory: true, multiple: false, ...(title ? { title } : {}) },
    }),
  pickFile: () => invoke<DialogSelection>("plugin:dialog|open", { options: { directory: false, multiple: false } }),
  pickImageMapFile: () =>
    invoke<DialogSelection>("plugin:dialog|open", {
      options: {
        directory: false,
        multiple: false,
        filters: [{ name: "Map image", extensions: ["png", "jpg", "jpeg", "svg"] }],
      },
    }),
  pickVectorMapFile: () =>
    invoke<DialogSelection>("plugin:dialog|open", {
      options: {
        directory: false,
        multiple: false,
        filters: [{ name: "GeoJSON", extensions: ["geojson", "json"] }],
      },
    }),
  create: (path: string) => invoke<ProjectInfo>("project_new", { path }),
  close: () => invoke<void>("project_close"),
  info: () => invoke<ProjectInfo | null>("project_info"),
  setAiEnabled: (enabled: boolean) => invoke<ProjectInfo>("project_set_ai_enabled", { enabled }),
  aiPromptsGet: () => invoke<{ templates?: Array<Record<string, unknown>> }>("project_ai_prompts_get"),
  aiPromptsSet: (overlay: { templates?: Array<Record<string, unknown>> }) =>
    invoke<{ templates?: Array<Record<string, unknown>> }>("project_ai_prompts_set", { overlay }),
  importCheckpoint: () => invoke<ExternalChangeReport>("project_import_checkpoint"),
  externalImporters: () => invoke<ExternalImporterDescriptor[]>("project_external_importers"),
  externalImportSelectSource: (sourceKind: "file" | "folder") =>
    invoke<ExternalImportSourceHandle | null>("project_external_import_select_source", { sourceKind }),
  externalImportAnalyzeBegin: (sourceHandle: string, importerId: string, limits?: ExternalImportLimits) =>
    invoke<ExternalImportAnalysisStatus>("project_external_import_analyze_begin", {
      input: { sourceHandle, importerId, limits: limits ?? null },
    }),
  externalImportAnalysisStatus: (sessionId: string) =>
    invoke<ExternalImportAnalysisStatus>("project_external_import_analysis_status", { sessionId }),
  externalImportAnalysisCancel: (sessionId: string) =>
    invoke<ExternalImportAnalysisStatus>("project_external_import_analysis_cancel", { sessionId }),
  externalImportAnalysisPage: (sessionId: string, offset: number, limit: number) =>
    invoke<ExternalImportPage>("project_external_import_analysis_page", { sessionId, offset, limit }),
  externalImportCandidatePlan: (sessionId: string, manifestFingerprint: string, mappings: ImportMappingOverrides) =>
    invoke<ImportCandidatePlan>("project_external_import_candidate_plan", {
      input: { sessionId, manifestFingerprint, mappings },
    }),
  externalImportValidate: (
    sessionId: string,
    mappings: ImportMappingOverrides,
    decisions: Record<string, ImportObjectDecision>,
  ) =>
    invoke<ExternalImportValidationSummary>("project_external_import_validate", {
      input: { sessionId, mappings, decisions },
    }),
  externalImportCommit: (
    sessionId: string,
    validationId: string,
    acknowledgeWarnings: boolean,
    commitRequestId: string = crypto.randomUUID(),
  ) =>
    invoke<ExternalImportCommitReport>("project_external_import_commit", {
      input: { sessionId, validationId, requestId: commitRequestId, acknowledgeWarnings },
    }),
  saveRecoveryCopy: (entityId: string, body: string) =>
    invoke<string>("project_save_recovery_copy", { entityId, body }),
  gitStatus: (reprobe = false) => invoke<GitStatus>("project_git_status", { reprobe }),
  gitPreflight: () => invoke<GitPreflight>("project_git_preflight"),
  gitStagingPreview: () => invoke<GitPreflight>("project_git_staging_preview"),
  gitInit: () => invoke<GitStatus>("project_git_init"),
  gitLog: () => invoke<GitLogEntry[]>("project_git_log"),
  gitCommit: (message: string, paths?: string[]) =>
    invoke<GitStatus>("project_git_commit", { message, paths: paths ?? null }),
  gitSuperSquash: (message: string) => invoke<GitResetResult>("project_git_super_squash", { message }),
  gitToolInfo: () => invoke<GitToolInfo>("git_tool_info"),
  gitShowTree: (hash: string) => invoke<string[]>("project_git_show_tree", { hash }),
  gitShowMessage: (hash: string) => invoke<string>("project_git_show_message", { hash }),
  gitShowChanges: (hash: string) => invoke<GitChange[]>("project_git_show_changes", { hash }),
  gitShowDiff: (hash: string, path: string) => invoke<string>("project_git_show_diff", { hash, path }),
  gitWorktreeDiff: (paths: string[]) => invoke<string>("project_git_worktree_diff", { paths }),
  gitShowFile: (hash: string, path: string) => invoke<string>("project_git_show_file", { hash, path }),
  gitResetHard: (hash: string) => invoke<GitResetResult>("project_git_reset_hard", { hash }),
  gitRemoteList: () => invoke<GitRemote[]>("project_git_remote_list"),
  gitRemoteAdd: (name: string, url: string) => invoke<GitRemote[]>("project_git_remote_add", { name, url }),
  gitRemoteSetUrl: (name: string, url: string) => invoke<GitRemote[]>("project_git_remote_set_url", { name, url }),
  gitRemoteRemove: (name: string) => invoke<GitRemote[]>("project_git_remote_remove", { name }),
  gitPush: (remote: string, branch?: string | null, forceWithLease = false) =>
    invoke<GitStatus>("project_git_push", { remote, branch: branch ?? null, forceWithLease }),
  gitRestoreFromUpstream: () => invoke<GitResetResult>("project_git_restore_from_upstream"),
  openExternalUrl: (url: string) => invoke<void>("open_external_url", { url }),
  listEntities: () => invoke<Entity[]>("project_list_entities"),
  getEntity: (id: string) => invoke<Entity | null>("project_get_entity", { id }),
  queryEntities: (query: EntityListQuery = {}) =>
    invoke<EntityPage>("project_query_entities", {
      query: {
        query: query.query ?? null,
        entity_types: query.entityTypes ?? [],
        excluded_entity_types: query.excludedEntityTypes ?? [],
        sort_field: query.sortField ?? null,
        sort_direction: query.sortDirection ?? null,
        offset: query.offset ?? null,
        limit: query.limit ?? null,
        archived: query.archived ?? null,
      },
    }),
  search: (query: string) => invoke<Entity[]>("project_search", { query }),
  searchMapFeatures: (query: string, limit = 50) =>
    invoke<MapFeatureSearchResult[]>("project_search_map_features", { query, limit }),
  deleteEntity: (id: string, options?: MutationOptions) =>
    invoke<void>("project_delete_entity", {
      id,
      expected_revision: options?.expectedRevision ?? null,
      request_id: requestId(options),
    }),
  restoreEntity: (id: string, options?: MutationOptions) =>
    invoke<void>("project_restore_entity", {
      id,
      expected_revision: options?.expectedRevision ?? null,
      request_id: requestId(options),
    }),
  purgeEntity: (id: string, options?: MutationOptions) =>
    invoke<void>("project_purge_entity", {
      id,
      expected_revision: options?.expectedRevision ?? null,
      request_id: requestId(options),
    }),
  createEntity: (name: string, entityType?: string, options?: MutationOptions) =>
    invoke<Entity>("project_create_entity", {
      input: { name, entity_type: entityType || null },
      request_id: requestId(options),
    }),
  updateEntity: (id: string, name?: string | null, entityType?: string | null, options?: MutationOptions) =>
    invoke<Entity>("project_update_entity", {
      id,
      name: name ?? null,
      entity_type: entityType ?? null,
      expected_revision: options?.expectedRevision ?? null,
      request_id: requestId(options),
    }),
  listDocuments: (entityId: string) => invoke<Document[]>("project_list_documents", { entityId }),
  saveDocument: (
    entityId: string,
    body: string,
    format: "markdown" | "plain-text" | "rich-text" = "markdown",
    options?: MutationOptions,
  ) =>
    invoke<void>("project_save_document", {
      input: { entity_id: entityId, body, format },
      expected_revision: options?.expectedRevision ?? null,
      request_id: requestId(options),
    }),
  saveEntry: (
    input: {
      document: { entity_id: string; body: string; format?: "markdown" | "plain-text" | "rich-text" };
      fields: FieldValue[];
    },
    options?: MutationOptions,
  ) =>
    invoke<void>("project_save_entry", {
      input,
      expected_revision: options?.expectedRevision ?? null,
      request_id: requestId(options),
    }),
  listFields: (entityId: string) => invoke<FieldValue[]>("project_list_fields", { entityId }),
  setField: (field: FieldValue, options?: MutationOptions) =>
    invoke<void>("project_set_field", { field, request_id: requestId(options) }),
  createRelationship: (
    sourceId: string,
    targetId: string,
    type: string,
    metadata?: Record<string, unknown>,
    options?: MutationOptions,
  ) =>
    invoke<Relationship>("project_create_relationship", {
      input: {
        source_id: sourceId,
        target_id: targetId,
        relationship_type: type,
        metadata: metadata ? JSON.stringify(metadata) : null,
      },
      expected_revision: options?.expectedRevision ?? null,
      request_id: requestId(options),
    }),
  updateRelationship: (id: string, metadata?: Record<string, unknown>, options?: MutationOptions) =>
    invoke<Relationship>("project_update_relationship", {
      input: {
        id,
        metadata: metadata ? JSON.stringify(metadata) : null,
        target_id: null,
      },
      expected_revision: options?.expectedRevision ?? null,
      request_id: requestId(options),
    }),
  listRelationships: (entityId: string) => invoke<Relationship[]>("project_list_relationships", { entityId }),
  listMapLocations: (entityId: string) => invoke<MapLocation[]>("project_list_map_locations", { entityId }),
  upsertMapLocation: (entityId: string, location: MapLocation, options?: MutationOptions) =>
    invoke<void>("project_upsert_map_location", { entityId, location, request_id: requestId(options) }),
  unlinkMapLocation: (entityId: string, locationId: string, options?: MutationOptions) =>
    invoke<void>("project_unlink_map_location", { entityId, locationId, request_id: requestId(options) }),
  mapsNavigation: (
    operation: string,
    input: { mapEntityId?: string; entityId?: string; linkId?: string; date?: unknown; entityIds?: string[] } = {},
  ) =>
    invoke<unknown>("maps_navigation", {
      operation,
      mapEntityId: input.mapEntityId ?? null,
      entityId: input.entityId ?? null,
      linkId: input.linkId ?? null,
      date: input.date ?? null,
      entityIds: input.entityIds ?? null,
    }),
  mapsRecoveryList: (entityId: string) => invoke<MapRecoveryCopy[]>("maps_recovery_list", { entityId }),
  mapsRecoveryRestore: (entityId: string, fileName: string) =>
    invoke<MapEditApply>("maps_recovery_restore", { entityId, fileName }),
  registerAsset: (
    input: {
      entity_id: string;
      namespace: string;
      filename: string;
      content_hash: string;
      size: number;
      mime_type: string;
      path: string;
    },
    options?: MutationOptions,
  ) =>
    invoke<Asset>("project_register_asset", {
      input,
      expected_revision: options?.expectedRevision ?? null,
      request_id: requestId(options),
    }),
  registerAssetFile: (
    input: { entity_id: string; namespace: string; source_path: string; filename: string; mime_type: string },
    options?: MutationOptions,
  ) =>
    invoke<Asset>("project_register_asset_file", {
      input,
      expected_revision: options?.expectedRevision ?? null,
      request_id: requestId(options),
    }),
  listAssets: (entityId: string) => invoke<Asset[]>("project_list_assets", { entityId }),
  listSharedAssets: () => invoke<Asset[]>("project_list_shared_assets"),
  importImageMapFile: (sourcePath: string) =>
    invoke<{ entity: Entity; source: Asset; preview: Asset }>("project_import_image_map_file", { sourcePath }),
  attachMapRasterAsset: (mapEntityId: string, sourcePath: string) =>
    invoke<{ asset: Asset; width: number; height: number }>("project_attach_map_raster_asset", {
      mapEntityId,
      sourcePath,
    }),
  duplicateMapRasterAsset: (mapEntityId: string, assetId: string) =>
    invoke<{ asset: Asset; width: number; height: number }>("project_duplicate_map_raster_asset", {
      mapEntityId,
      assetId,
    }),
  importVectorMapFile: (sourcePath: string) =>
    invoke<{ entity: Entity; source: Asset }>("project_import_vector_map_file", { sourcePath }),
  generatePhysicalMap: (input: PhysicalGenerationInput, requestId = crypto.randomUUID()) =>
    invoke<PhysicalJobStatus>("project_physical_generate", { input, requestId }),
  physicalMapStatus: (jobId: string) => invoke<PhysicalJobStatus>("project_physical_status", { jobId }),
  physicalMapPreview: (jobId: string) => invoke<string>("project_physical_preview", { jobId }),
  physicalMapClimate: (jobId: string) => invoke<PhysicalClimateProducts>("project_physical_climate", { jobId }),
  physicalMapEvolution: (jobId: string) => invoke<PhysicalEvolutionProducts>("project_physical_evolution", { jobId }),
  physicalMapHydrology: (jobId: string) => invoke<PhysicalHydrologyProducts>("project_physical_hydrology", { jobId }),
  cancelPhysicalMap: (jobId: string) => invoke<PhysicalJobStatus>("project_physical_cancel", { jobId }),
  acceptPhysicalMap: (jobId: string, name: string, options?: MutationOptions) =>
    invoke<AcceptedPhysicalMap>("project_physical_accept", {
      jobId,
      name,
      requestId: requestId(options),
    }),
  replaceVectorSource: (
    assetId: string,
    bytes: Uint8Array,
    uploadContentHash: string,
    expectedRevision: string,
    options?: MutationOptions,
  ) =>
    invoke<VectorSourceReplace>("project_replace_vector_source", {
      assetId,
      bytes: Array.from(bytes),
      uploadContentHash,
      expectedRevision,
      requestId: requestId(options),
    }),
  applyMapEdit: (input: {
    mapEntityId: string;
    descriptor: unknown;
    layers: unknown;
    bytes: Uint8Array;
    uploadContentHash: string;
    expectedMapRevision: string;
    expectedLayersRevision: string;
    expectedSourceRevision: string;
    linkMutations?: MapLinkMutation[];
    requestId?: string;
  }) =>
    invoke<MapEditApply>("project_apply_map_edit", {
      ...input,
      bytes: Array.from(input.bytes),
      linkMutations: input.linkMutations ?? [],
      requestId: input.requestId ?? crypto.randomUUID(),
    }),
  createVectorLayer: (
    mapEntityId: string,
    name: string,
    expectedRevision: string,
    options?: MutationOptions & { style?: unknown },
  ) =>
    invoke<RasterLayerChange>("project_create_vector_layer", {
      mapEntityId,
      name,
      expectedRevision,
      style: options?.style ?? null,
      requestId: requestId(options),
    }),
  deleteVectorLayer: (
    mapEntityId: string,
    layerId: string,
    expectedRevision: string,
    expectedSourceRevision: string,
    expectedFeatureCount: number,
    options?: MutationOptions,
  ) =>
    invoke<VectorLayerDelete>("project_delete_vector_layer", {
      mapEntityId,
      layerId,
      expectedRevision,
      expectedSourceRevision,
      expectedFeatureCount,
      requestId: requestId(options),
    }),
  mapsRecoveryExport: (entityId: string, bytes: Uint8Array) =>
    invoke<string>("maps_recovery_export", { entityId, bytes: Array.from(bytes) }),
  physicalMapDerivedGeoJson: (mapEntityId: string) =>
    invoke<string>("project_physical_derived_geojson", { mapEntityId }),
  physicalMapDerivedClimate: (mapEntityId: string) =>
    invoke<PhysicalClimateProducts>("project_physical_derived_climate", { mapEntityId }),
  physicalMapDerivedEvolution: (mapEntityId: string) =>
    invoke<PhysicalEvolutionProducts>("project_physical_derived_evolution", { mapEntityId }),
  physicalMapDerivedHydrology: (mapEntityId: string) =>
    invoke<PhysicalHydrologyProducts>("project_physical_derived_hydrology", { mapEntityId }),
  physicalMapDerivedEpoch: (mapEntityId: string, epochOffsetYears: number, requestId = crypto.randomUUID()) =>
    invoke<PhysicalHistoricalProducts>("project_physical_derived_epoch", {
      mapEntityId,
      epochOffsetYears,
      requestId,
    }),
  physicalMaterializeEvents: (
    mapEntityId: string,
    input: PhysicalEventMaterializationRequest,
    options?: MutationOptions,
  ) =>
    invoke<PhysicalEventMaterializationResult>("project_physical_materialize_events", {
      mapEntityId,
      request: input,
      requestId: requestId(options),
    }),
  physicalMapClearEpochCache: () => invoke<void>("project_physical_clear_epoch_cache"),
  atlasCapabilities: (mapEntityId: string) =>
    invoke<AtlasRenderCapabilities>("project_atlas_capabilities", { mapEntityId }),
  atlasPreviewBegin: (mapEntityId: string, request: AtlasRenderRequest, requestId = crypto.randomUUID()) =>
    invoke<AtlasJobStatus>("project_atlas_preview_begin", {
      input: { mapEntityId, request, requestId },
    }),
  atlasRenderBegin: (mapEntityId: string, request: AtlasRenderRequest, requestId = crypto.randomUUID()) =>
    invoke<AtlasJobStatus>("project_atlas_render_begin", {
      input: { mapEntityId, request, requestId },
    }),
  atlasJobStatus: (jobId: string) => invoke<AtlasJobStatus>("project_atlas_job_status", { jobId }),
  atlasJobCancel: (jobId: string) => invoke<AtlasJobStatus>("project_atlas_job_cancel", { jobId }),
  atlasArtifactSave: (jobId: string) => invoke<AtlasJobStatus>("project_atlas_artifact_save", { jobId }),
  atlasArtifactDiscard: (jobId: string) => invoke<AtlasJobStatus>("project_atlas_artifact_discard", { jobId }),
  atlasStudioOpen: (request: AtlasStudioSessionRequest, deviceScale = 1) =>
    invoke<AtlasStudioSessionStatus>("project_atlas_studio_open", {
      input: { request, deviceScale },
    }),
  atlasStudioClose: (sessionToken: string) => invoke<void>("project_atlas_studio_close", { sessionToken }),
  atlasStudioStatus: (sessionToken: string) =>
    invoke<AtlasStudioSessionStatus>("project_atlas_studio_status", { sessionToken }),
  atlasStudioRegenerateCache: () => invoke<{ deletedEntries: number }>("project_atlas_studio_regenerate_cache"),
  atlasStudioInspect: (sessionToken: string, lonMicro: number, latMicro: number, zoom: number) =>
    invoke<AtlasStudioInspectResult>("project_atlas_studio_inspect", {
      input: { sessionToken, lonMicro, latMicro, zoom },
    }),
  readAssetBytes: (assetId: string) => invoke<number[]>("project_read_asset_bytes", { assetId }),
  readAssetBytesByPath: (path: string) => invoke<number[]>("project_read_asset_bytes_by_path", { path }),
  getAssetByPath: (path: string) => invoke<Asset>("project_get_asset_by_path", { path }),
  createRasterLayer: (mapEntityId: string, name: string, expectedRevision: string, options?: MutationOptions) =>
    invoke<RasterLayerChange>("project_create_raster_layer", {
      mapEntityId,
      name,
      expectedRevision,
      requestId: requestId(options),
    }),
  updateMapLayer: (
    mapEntityId: string,
    layerId: string,
    expectedRevision: string,
    update: {
      name?: string;
      order?: number;
      defaultVisible?: boolean;
      opacity?: number;
      locked?: boolean;
      style?: unknown;
      selector?: unknown;
    },
    options?: MutationOptions,
  ) =>
    invoke<RasterLayerChange>("project_update_map_layer", {
      mapEntityId,
      layerId,
      expectedRevision,
      name: update.name ?? null,
      order: update.order ?? null,
      defaultVisible: update.defaultVisible ?? null,
      opacity: update.opacity ?? null,
      locked: update.locked ?? null,
      style: update.style ?? null,
      selector: update.selector ?? null,
      requestId: requestId(options),
    }),
  createSemanticLayer: (
    mapEntityId: string,
    name: string,
    expectedRevision: string,
    options?: MutationOptions & { style?: unknown; selector?: unknown },
  ) =>
    invoke<RasterLayerChange>("project_create_semantic_layer", {
      mapEntityId,
      name,
      expectedRevision,
      style: options?.style ?? null,
      selector: options?.selector ?? null,
      requestId: requestId(options),
    }),
  deleteSemanticLayer: (mapEntityId: string, layerId: string, expectedRevision: string, options?: MutationOptions) =>
    invoke<RasterLayerChange>("project_delete_semantic_layer", {
      mapEntityId,
      layerId,
      expectedRevision,
      requestId: requestId(options),
    }),
  deleteRasterLayer: (mapEntityId: string, layerId: string, expectedRevision: string, options?: MutationOptions) =>
    invoke<RasterLayerChange>("project_delete_raster_layer", {
      mapEntityId,
      layerId,
      expectedRevision,
      requestId: requestId(options),
    }),
  replaceAssetBytes: (
    assetId: string,
    bytes: Uint8Array,
    contentHash: string,
    mimeType: string,
    expectedRevision: string,
    options?: MutationOptions,
  ) =>
    invoke<Asset>("project_replace_asset_bytes", {
      assetId,
      bytes: Array.from(bytes),
      contentHash,
      mimeType,
      expectedRevision,
      requestId: requestId(options),
    }),
  replaceAssetFile: (
    assetId: string,
    sourcePath: string,
    mimeType: string,
    expectedRevision: string,
    options?: MutationOptions,
  ) =>
    invoke<Asset>("project_replace_asset_file", {
      assetId,
      sourcePath,
      mimeType,
      expectedRevision,
      requestId: requestId(options),
    }),
  updateAssetMetadata: (
    assetId: string,
    update: {
      filename?: string;
      role?: "attachment" | "profile";
      referenceScope?: "entity" | "project";
    },
    expectedRevision: string,
    options?: MutationOptions,
  ) =>
    invoke<Asset>("project_update_asset_metadata", {
      assetId,
      filename: update.filename ?? null,
      role: update.role ?? null,
      referenceScope: update.referenceScope ?? null,
      expectedRevision,
      requestId: requestId(options),
    }),
  deleteAsset: (assetId: string, expectedRevision: string, options?: MutationOptions) =>
    invoke<void>("project_delete_asset", {
      assetId,
      expectedRevision,
      requestId: requestId(options),
    }),
  listMapPins: (mapEntityId: string) => invoke<MapPin[]>("project_map_location_projection", { mapEntityId }),
  queryMapLocations: (mapEntityId: string, minX: number, minY: number, maxX: number, maxY: number) =>
    invoke<MapPin[]>("project_query_map_locations", { mapEntityId, minX, minY, maxX, maxY }),
  backup: () => invoke<string>("project_backup"),
  exportMarkdown: (destination: string) => invoke<string>("project_export_markdown", { destination }),
  exportWikiPage: (entityId: string, destination: string, format: WikiPageExportFormat, manifestId: string) =>
    invoke<string>("project_export_wiki_page", { entityId, destination, format, manifestId }),
  recoveryBackup: () => invoke<string>("project_recovery_backup"),
  restoreRecoveryBackup: (path: string) => invoke<void>("project_restore_recovery_backup", { path }),
  restore: (path: string) => invoke<void>("project_restore", { path }),
  restorePayload: (payload: string, options?: MutationOptions) =>
    invoke<void>("project_restore_payload", { payload, request_id: requestId(options) }),
  rebuildSearch: () => invoke<void>("project_rebuild_search"),
  seedExample: () => invoke<number>("project_seed_example"),
  listModuleManifests: () => invoke<ProjectModuleManifest[]>("module_list_manifests"),
  getModuleSchemaOverlay: (moduleId: string) => invoke<ModuleSchemaOverlay>("module_schema_overlay_get", { moduleId }),
  loadModuleSchemaEditor: (moduleId: string) =>
    invoke<ModuleSchemaEditorState>("module_schema_editor_load", { moduleId }),
  previewModuleSchemaOverlay: (moduleId: string, overlay: ModuleSchemaOverlay) =>
    invoke<SchemaOverlayPreviewResult>("module_schema_overlay_preview", { moduleId, overlay }),
  setModuleSchemaOverlay: (moduleId: string, overlay: ModuleSchemaOverlay, options?: MutationOptions) =>
    invoke<ModuleSchemaOverlayMutationResult>("module_schema_overlay_set", {
      moduleId,
      overlay,
      expected_revision: options?.expectedRevision ?? null,
      request_id: requestId(options),
      acknowledge_impact: options?.acknowledgeImpact ?? null,
    }),
  enableModule: (id: string, grantedCapabilities?: string[]) =>
    invoke<void>("module_enable", { id, grantedCapabilities }),
  disableModule: (id: string) => invoke<void>("module_disable", { id }),
  adminView: () => invoke<PluginAdminView>("plugin_admin_view"),
  openPluginWebview: (pluginId: string, viewId?: string) => invoke<void>("plugin_open_webview", { pluginId, viewId }),
  mountPluginWebview: (
    pluginId: string,
    viewId: string | undefined,
    bounds: { x: number; y: number; width: number; height: number; viewportWidth: number; viewportHeight: number },
    mapEntityId?: string,
    linkId?: string,
  ) => invoke<void>("plugin_mount_webview", { pluginId, viewId, mapEntityId, linkId, bounds }),
  resizePluginWebview: (
    pluginId: string,
    bounds: { x: number; y: number; width: number; height: number; viewportWidth: number; viewportHeight: number },
  ) => invoke<void>("plugin_resize_webview", { pluginId, bounds }),
  unmountPluginWebview: (pluginId: string) => invoke<void>("plugin_unmount_webview", { pluginId }),
  closeAllPluginWebviews: () => invoke<void>("plugin_close_all_webviews"),
  hostViewData: (pluginId: string, viewId: string, selectedEntityId?: string) =>
    invoke<HostViewData>("plugin_host_view_data", { pluginId, viewId, selectedEntityId: selectedEntityId ?? null }),
  hostViewSetField: (
    pluginId: string,
    viewId: string,
    componentId: string,
    entityId: string,
    key: string,
    value: unknown,
  ) => invoke<void>("plugin_host_view_set_field", { pluginId, viewId, componentId, entityId, key, value }),
  hostViewInvokeCommand: (pluginId: string, viewId: string, commandId: string, payload?: Record<string, unknown>) =>
    invoke<string>("plugin_host_invoke_command", { pluginId, viewId, commandId, payload: payload ?? {} }),
  closePluginWebview: (pluginId: string) => invoke<void>("plugin_close_webview", { pluginId }),
  installPlugin: (archive: string, allowUnsigned = false) =>
    invoke<InstalledPluginVersion>("plugin_install_package", { archive, allowUnsigned }),
  upgradePlugin: (pluginId: string, version: string, consent: boolean) =>
    invoke<void>("plugin_upgrade", { pluginId, version, consent }),
  pluginUpgradePlan: (pluginId: string, version: string) =>
    invoke<PluginUpgradePlan>("plugin_upgrade_plan", { pluginId, version }),
  rollbackPlugin: (pluginId: string, version: string) => invoke<void>("plugin_rollback", { pluginId, version }),
  uninstallPluginCode: (pluginId: string, version: string) =>
    invoke<void>("plugin_uninstall_code", { pluginId, version }),
  deletePluginData: (pluginId: string, confirmation: string) =>
    invoke<string>("plugin_delete_data", { pluginId, confirmation }),
  retryPlugin: (pluginId: string) => invoke<void>("plugin_retry", { pluginId }),
  pickPluginPackage: () =>
    invoke<DialogSelection>("plugin:dialog|open", {
      options: {
        directory: false,
        multiple: false,
        filters: [{ name: "Daena Archive plugin", extensions: ["wbplugin"] }],
      },
    }),
  settingsGet: () => invoke<AppSettings>("settings_get"),
  settingsUpdate: (update: AppSettingsUpdate) => invoke<AppSettings>("settings_update", { update }),
  aiProviderStatus: (projectId: string) => invoke<AiProviderStatus>("ai_provider_status", { projectId }),
  aiProviderModels: (projectId: string) => invoke<string[]>("ai_provider_models", { projectId }),
  aiProviderConnect: (projectId: string) =>
    invoke<import("./types").AiProviderConnectResult>("ai_provider_connect", { projectId }),
  aiProviderCredentialStatus: (projectId: string) =>
    invoke<RemoteCredentialStatus>("ai_provider_credential_status", { projectId }),
  aiProviderImportCredential: (projectId: string) =>
    invoke<RemoteCredentialStatus>("ai_provider_import_credential", { projectId }),
  aiProviderSetCredential: (projectId: string, apiKey: string) =>
    invoke<RemoteCredentialStatus>("ai_provider_set_credential", { projectId, apiKey }),
  aiProviderClearCredential: (projectId: string) =>
    invoke<RemoteCredentialStatus>("ai_provider_clear_credential", { projectId }),
  imageProviderStatus: () => invoke<ImageProviderStatus>("image_provider_status"),
  imageProviderDiscover: () => invoke<ImageProviderDiscovery>("image_provider_discover"),
  imageGenerateStart: (request: ImageGenerationRequest) =>
    invoke<ImageGenerationStatus>("image_generate_start", { request }),
  imageGenerationStatus: (jobId: string, projectId: string) =>
    invoke<ImageGenerationStatus>("image_generation_status", { jobId, projectId }),
  imageGenerationCancel: (jobId: string, projectId: string) =>
    invoke<ImageGenerationStatus>("image_generation_cancel", { jobId, projectId }),
  imageCandidateBytes: (jobId: string, candidateId: string, projectId: string) =>
    invoke<number[]>("image_candidate_bytes", { jobId, candidateId, projectId }),
  imageCandidateAccept: (
    jobId: string,
    candidateId: string,
    projectId: string,
    entityId: string,
    namespace: string,
    filename: string,
    requestId = crypto.randomUUID(),
  ) =>
    invoke<Asset>("image_candidate_accept", {
      jobId,
      candidateId,
      projectId,
      entityId,
      namespace,
      filename,
      requestId,
    }),
  imageCandidateDiscard: (jobId: string, candidateId: string, projectId: string) =>
    invoke<ImageGenerationStatus>("image_candidate_discard", { jobId, candidateId, projectId }),
  imageGenerationDiscard: (jobId: string, projectId: string) =>
    invoke<void>("image_generation_discard", { jobId, projectId }),
  aiRemoteSetConsent: (projectId: string, allowed: boolean) =>
    invoke<void>("ai_remote_set_consent", { projectId, allowed }),
  aiIndexStatus: () => invoke<AiIndexStatus>("ai_index_status"),
  aiIndexRebuild: () => invoke<AiIndexRebuildResult>("ai_index_rebuild"),
  aiIndexCancel: () => invoke<void>("ai_index_cancel"),
  aiIndexSearch: (query: string, limit = 8) => invoke<AiHybridMatch[]>("ai_index_search", { query, limit }),
  aiGenerateText: (
    projectId: string,
    instruction: string,
    selection: string,
    entityId?: string,
    retrievalQuery?: string,
    retrievalDepth = 2,
    includeRetrieval = true,
  ) =>
    invoke<string>("ai_generate_text", {
      projectId,
      instruction,
      selection,
      entityId,
      retrievalQuery,
      retrievalDepth,
      includeRetrieval,
    }),
  aiGenerateStructured: (
    projectId: string,
    instruction: string,
    context: string,
    outputContract: Record<string, unknown>,
    entityId?: string,
    retrievalQuery?: string,
    retrievalDepth = 2,
  ) =>
    invoke<string>("ai_generate_structured", {
      projectId,
      instruction,
      context,
      outputContract,
      entityId,
      retrievalQuery,
      retrievalDepth,
      includeRetrieval: true,
    }),
  aiCancelText: (requestId: string) => invoke<void>("ai_cancel_text", { requestId }),
  aiPollText: (requestId: string) => invoke<AiStreamEvent[]>("ai_poll_text", { requestId }),
};
