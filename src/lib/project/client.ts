import { invoke } from "@tauri-apps/api/core";
import type { ModuleManifest } from "../../../packages/module-api/src/index";
import type { EntityTemplate, FieldDefinition } from "../../../packages/plugin-sdk/src/generated";

export type { EntityTemplate, FieldDefinition };

export interface FieldScopeOverride {
  fieldKey: string;
  entityTypes: string[];
}

export interface TemplateOverride {
  templateId: string;
  fields: Record<string, unknown>;
  requiredFields?: string[] | null;
}

export interface ModuleSchemaOverlay {
  version: number;
  disabledEntityTypes?: string[];
  disabledFields?: string[];
  disabledTemplates?: string[];
  customEntityTypes?: string[];
  customFields?: FieldDefinition[];
  customTemplates?: EntityTemplate[];
  fieldScopeOverrides?: FieldScopeOverride[];
  templateOverrides?: TemplateOverride[];
}

export interface ModuleSchemaEditorState {
  id: string;
  name: string;
  schemas: Array<{
    namespace: string;
    entityTypes: string[];
    fields: FieldDefinition[];
  }>;
  templates: EntityTemplate[];
  overlay: ModuleSchemaOverlay;
}

export interface Entity {
  id: string;
  name: string;
  entity_type: string | null;
  deleted: boolean;
  created_at: string;
  updated_at: string;
  revision: string;
}
export interface Document {
  id: string;
  entity_id: string;
  format: string;
  body: string;
  updated_at: string;
  revision: string;
}
export interface FieldValue {
  entity_id: string;
  namespace: string;
  key: string;
  value: unknown;
  revision: string;
}
export interface Relationship {
  id: string;
  source_id: string;
  target_id: string;
  relationship_type: string;
  metadata: string;
  revision: string;
}
export interface MapLocation {
  id: string;
  mapEntityId: string;
  role: string;
  label: string;
  anchor: unknown;
  validity: { from: unknown | null; to: unknown | null };
  anchorKind?: string;
  resolution?: string;
}
export interface MapPin {
  id: string;
  entityId: string;
  mapEntityId?: string;
  label: string | null;
  role: string;
  anchorKind: string;
  bounds: [number | null, number | null, number | null, number | null];
  resolution: string;
  validity?: { from: unknown | null; to: unknown | null };
  anchor?: unknown;
}
export interface RasterLayerChange {
  layer_id: string;
  asset: Asset | null;
  layers: FieldValue;
}
export interface VectorSourceReplace {
  source: Asset;
}
export interface PhysicalJobStatus {
  jobId: string;
  requestId: string;
  state: string;
  stage: string;
  completed: number;
  total: number;
  error: string | null;
  errorCode: string | null;
  physicalIdentity: string | null;
}
export interface AcceptedPhysicalMap {
  entity: Entity;
  source: Asset;
  physicalIdentity: string;
}
export interface PhysicalClimateProducts {
  derivationVersion: number;
  width: number;
  height: number;
  temperatureCentiC: number[];
  moistureMmPerYear: number[];
  precipitationMmPerYear: number[];
  runoffMmPerYear: number[];
  runoffVolumeM3PerYear: number[];
  maritimeFactorPpm: number[];
  metrics: {
    precipitationVolumeM3PerYear: number;
    runoffVolumeM3PerYear: number;
    meanTemperatureCentiC: number;
    minimumTemperatureCentiC: number;
    maximumTemperatureCentiC: number;
    meanPrecipitationMmPerYear: number;
    meanRunoffMmPerYear: number;
    wettestCellPrecipitationMmPerYear: number;
    driestLandCellPrecipitationMmPerYear: number;
    transportIterations: number;
  };
}
export interface PhysicalEvolutionProducts {
  derivationVersion: number;
  preset: "young" | "mature" | "old";
  width: number;
  height: number;
  beforeElevationsMm: number[];
  elevationsMm: number[];
  routingElevationMm: number[];
  fillDepthMm: number[];
  slopePpm: number[];
  accumulationM3PerYear: number[];
  outletCells: number[];
  edges: Array<{
    sourceCell: number;
    destinationCell: number;
    weightPpm: number;
    distanceMetres: number;
  }>;
  drainageMetrics: {
    directRunoffM3PerYear: number;
    routedRunoffM3PerYear: number;
    routedEdgeCount: number;
    drainageDensityPpm: number;
    gridAnisotropyPpm: number;
    convergencePpm: number;
    outletCount: number;
    routingSurfaceRaiseMaxMm: number;
  };
  evolutionMetrics: {
    initialReliefSpanMm: number;
    finalReliefSpanMm: number;
    reliefChangeMm: number;
    meanAbsoluteElevationChangeMm: number;
    erosionWorkM3: number;
    upliftWorkM3: number;
    maxStepReliefLossMm: number;
    drainageDensityPpm: number;
    gridAnisotropyPpm: number;
    convergencePpm: number;
    tectonicRangeOrientationPpm: number;
  };
}
export interface PhysicalHydrologyProducts {
  derivationVersion: number;
  width: number;
  height: number;
  seaLevelMm: number;
  waterLevelMm: number[];
  lakeLevelMm: number[];
  slopePpm: number[];
  hillshadePpm: number[];
  bathymetryMm: number[];
  watershedId: number[];
  basinByCell: number[];
  lakeCells: boolean[];
  iceCells: boolean[];
  iceThicknessMm: number[];
  shelfCells: boolean[];
  islandId: number[];
  basins: Array<{
    id: number;
    minimumCell: number;
    minimumElevationMm: number;
    cellCount: number;
    spillCell: number | null;
    spillElevationMm: number | null;
    volumeToSpillM3: number;
    parentBasin: number | null;
    children: number[];
    destination: "ocean" | "basin" | "endorheic" | "junction";
    waterLevelMm: number;
    waterVolumeM3: number;
    inflowM3PerYear: number;
    directPrecipitationM3PerYear: number;
    evaporationM3PerYear: number;
    outflowM3PerYear: number;
    status: "dry" | "endorheic" | "active" | "overflowing" | "merged";
  }>;
  rivers: Array<{
    id: number;
    sourceCell: number;
    mouthCell: number;
    strahlerOrder: number;
    destination: "ocean" | "basin" | "endorheic" | "junction";
    spillOutlet: boolean;
    coordinateCount: number;
  }>;
  metrics: {
    totalWaterM3: number;
    oceanWaterM3: number;
    inlandWaterM3: number;
    landIceM3: number;
    balanceErrorM3: number;
    toleranceM3: number;
    fixedPointIterations: number;
    converged: boolean;
    lakeCount: number;
    riverCount: number;
    watershedCount: number;
    coastlineSegmentCount: number;
    landPolygonCount: number;
    oceanPolygonCount: number;
    shelfCellCount: number;
    bathymetryContourCount: number;
    islandCount: number;
  };
}
export interface PhysicalHistoricalProducts {
  cacheKey: string;
  sourceHash: string;
  physicalIdentity: string;
  epochOffsetYears: number;
  normalizedEpoch: number;
  chronology: PhysicalEpochMapping;
  geojson: string;
  climate: PhysicalClimateProducts;
  hydrology: PhysicalHydrologyProducts;
  hazards: {
    derivationVersion: number;
    model: "relative-generated-v1";
    volcanicSourceDerivationVersion: number;
    prediction: false;
  };
  derivedHashes: {
    canonicalSource: string;
    finalElevation: string;
    tectonics: string;
    geography: string;
    climate: string;
    hydrology: string;
  };
  forcing: {
    version: number;
    components: Array<{
      amplitudeCentiC: number;
      periodYears: number;
      phaseOffsetYears: number;
    }>;
    sensitivityPpm: number;
    landIceAmplitudePpm: number;
    iceResponseYears: number;
    iceMidpointCentiC: number;
    iceTransitionWidthCentiC: number;
    thermalExpansionPpmPerDegreeC: number;
  };
  history: {
    derivationVersion: number;
    epochOffsetYears: number;
    normalizedEpoch: number;
    temperatureOffsetCentiC: number;
    laggedTemperatureOffsetCentiC: number;
    landIceEquilibriumM3: number;
    landIceM3: number;
    thermalExpansionM3: number;
    effectiveOceanWaterM3: number;
    conservedWaterM3: number;
    balanceErrorM3: number;
    seaLevelMm: number;
  };
}
export interface PhysicalEpochMapping {
  contractVersion: 1;
  kind: "physical-offset-years";
  reference: "accepted-source";
  epochOffsetYears: number;
}
export interface PhysicalHistoricalProgress {
  mapEntityId: string;
  requestId: string;
  phase: string;
  completed: number;
  total: number;
}
export const PHYSICAL_HISTORICAL_PROGRESS_EVENT = "physical-historical-progress";
export const ATLAS_PROGRESS_EVENT = "atlas-progress";
export const ATLAS_STUDIO_PROGRESS_EVENT = "atlas-studio-progress";
export const EXTERNAL_IMPORT_PROGRESS_EVENT = "external-import-progress";
export interface AtlasLayerChoice {
  id: string;
  name: string;
  role: string;
  defaultVisible: boolean;
}
export interface AtlasRenderCapabilities {
  supported: boolean;
  timeModes: string[];
  projections: string[];
  formats: string[];
  styles: string[];
  layers: AtlasLayerChoice[];
  maxWidthPx: number;
  maxHeightPx: number;
  maxPixelCount: number;
  supportsAuthoredLayers: boolean;
  supportsSemanticLayers: boolean;
  supportsStudio: boolean;
  studioMaxZoom: number;
  studioTileSize: number;
  calendarBinding: {
    schemaVersion: number;
    calendarId: string;
    calendarReferenceYear: number;
    physicalOffsetAtReference: number;
    hasYearZero: boolean;
  } | null;
  presets: Array<{ id: string; name: string }>;
}
export interface AtlasRenderRequest {
  schemaVersion: number;
  offsetYears: number;
  algorithmVersion: number;
  level: "standard" | "detailed" | "print";
  variant: number;
  styleId: string;
  widthPx: number;
  heightPx: number;
  dpi: number;
  format: "png" | "svg" | "pdf";
  projection: "equirectangular" | "web-mercator";
  extent: {
    westLonMicro: number;
    southLatMicro: number;
    eastLonMicro: number;
    northLatMicro: number;
  };
  unlockAspect: boolean;
  activeLayerIds: string[];
  timeKind: "physical-offset-year" | "calendar-year";
  authoredYear: number | null;
  bindingRevision: string | null;
}
export interface AtlasJobStatus {
  jobId: string;
  requestId: string;
  mapEntityId: string;
  kind: string;
  state: string;
  stage: string;
  completed: number;
  total: number;
  sequence: number;
  error: string | null;
  errorCode: string | null;
  widthPx: number;
  heightPx: number;
  previewToken: string | null;
  capturedContentGeneration: number | null;
  currentContentGeneration: number | null;
  provenance: unknown;
  estimate: {
    pixelCount: number;
    rgbaBytes: number;
    estimatedPngBytes: number;
    tileCount: number;
    printWidthInchesMilli: number;
    printHeightInchesMilli: number;
  } | null;
}
export interface AtlasStudioSessionRequest {
  schemaVersion: number;
  mapEntityId: string;
  offsetYears: number;
  algorithmVersion: number;
  level: "standard" | "detailed" | "print";
  variant: number;
  styleId: string;
  activeLayerIds: string[];
  projection: string;
  timeKind: "physical-offset-year" | "calendar-year";
  authoredYear: number | null;
}
export interface AtlasStudioSessionStatus {
  sessionToken: string;
  mapEntityId: string;
  tileUrlTemplate: string;
  maxZoom: number;
  tileSize: number;
  deviceScale: number;
  capturedContentGeneration: number;
  currentContentGeneration: number | null;
  styleId: string;
  offsetYears: number;
  timeKind: string;
  authoredYear: number | null;
  activeLayerIds: string[];
  projection: string;
  stage: string;
  error: string | null;
  errorCode: string | null;
}
export interface AtlasStudioProgress {
  sessionToken: string;
  mapEntityId: string;
  stage: string;
  completed: number;
  total: number;
}
export interface AtlasStudioInspectHit {
  id: string;
  layerId: string;
  kind: string;
  label: string | null;
  derived: boolean;
}
export interface AtlasStudioSurfaceSample {
  lonMicro: number;
  latMicro: number;
  elevationMm: number;
  waterSurfaceMm: number;
  temperatureCentiC: number;
  precipitationMm: number;
  climate: string;
  surface: string;
  iceThicknessMm: number;
}
export interface AtlasStudioInspectResult {
  hits: AtlasStudioInspectHit[];
  surface: AtlasStudioSurfaceSample;
}
export interface PhysicalGenerationInput {
  seed: number;
  retryIndex: number;
  evolutionPreset?: "young" | "mature" | "old";
  settings: {
    width: number;
    height: number;
    radiusMetres: number;
    targetLandFractionPpm: number;
  };
}
export type PhysicalNaturalEventKind = "earthquake" | "eruption";
export interface PhysicalEventMaterializationRequest {
  eventKind: PhysicalNaturalEventKind;
  intervalStartYears: number;
  intervalEndYears: number;
  maxEvents: number;
  hazardSeed: number;
}
export interface PhysicalMaterializedEvent {
  entityId: string;
  eventKind: PhysicalNaturalEventKind;
  ordinal: number;
  yearOffset: number;
  cell: number;
  longitudeMicrodegrees: number;
  latitudeMicrodegrees: number;
  magnitudeMilli: number;
  hazardPpm: number;
  annualRateNano: number;
  ratePerMillionYearsPpm: number;
  sampledCenterId: number | null;
  volcanicSourceDerivationVersion: number;
}
export interface PhysicalEventMaterializationResult {
  requestId: string;
  mapEntityId: string;
  materializationVersion: number;
  hazardDerivationVersion: number;
  prediction: false;
  events: PhysicalMaterializedEvent[];
}
export interface VectorLayerDelete {
  layers: FieldValue;
  source: Asset;
  deletedFeatureCount: number;
}
export interface MapRecoveryCopy {
  fileName: string;
  path: string;
  createdAt: string;
}
export interface Asset {
  id: string;
  entity_id: string;
  namespace: string;
  filename: string;
  content_hash: string;
  size: number;
  mime_type: string;
  path: string;
  created_at: string;
  role: "attachment" | "profile";
  reference_scope: "entity" | "project";
  revision: string;
}
export interface SyncSummary {
  state: string;
  dirty_count: number;
  export_error: string | null;
}
export interface ProjectInfo {
  name: string;
  root: string;
  index_status: string;
  assets: string;
  sync: SyncSummary;
  aiEnabled: boolean;
}
export interface ExternalChangeReport {
  changed: boolean;
  paths: string[];
  diagnostics: string[];
}
export interface GitStatus {
  repository: boolean;
  branch: string | null;
  changes: string[];
  canonical_changes: string[];
  staged_canonical_changes: string[];
}
export interface GitPreflight {
  ready: boolean;
  diagnostics: string[];
  canonical_paths: string[];
  asset_paths: string[];
  staging_paths: string[];
  staged_paths: string[];
  unmerged_paths: string[];
}
export interface GitLogEntry {
  hash: string;
  date: string;
  subject: string;
}
export interface GitChange {
  status: string;
  path: string;
}
export interface GitToolInfo {
  available: boolean;
  version: string | null;
  error: string | null;
}
export interface GitRemote {
  name: string;
  fetchUrl: string;
  pushUrl: string;
}
export interface GitUpstream {
  remote: string;
  branch: string;
  remoteHash: string | null;
}
export interface GitResetResult {
  status: GitStatus;
  previousHead: string | null;
  currentHead: string | null;
  upstream: GitUpstream | null;
  divergedFromUpstream: boolean;
  rebuild: ExternalChangeReport;
}
export type { ModuleManifest };
export type ProjectModuleManifest = ModuleManifest & { enabled: boolean };
export interface InstalledPluginVersion {
  id: string;
  version: string;
  publisher: string;
  digest: string;
  signed: boolean;
}
export interface LifecycleInfo {
  state: string;
  failures: number;
  lastError: string | null;
}
export interface InstalledVersionInfo {
  version: string;
  publisher: string;
  digest: string;
  signed: boolean;
  unsignedConsent: boolean;
  installedAt: number;
  isSelected: boolean;
  isActiveCandidate: boolean;
  bundled: boolean;
  rollbackAvailable: boolean;
}
export interface DependencyState {
  resolved: boolean;
  order: string[];
  error: string | null;
}
export interface PluginDistribution {
  origin: "bundled" | "installed";
  management: "app" | "user";
  canUninstall: boolean;
}
export interface PluginAdminEntry extends ProjectModuleManifest {
  enabled: boolean;
  selectedVersion: string | null;
  dataVersion: number;
  lifecycle: LifecycleInfo;
  runtimeRunning: boolean;
  grantedCapabilities: string[];
  installedVersions: InstalledVersionInfo[];
  dependencyState: DependencyState;
  distribution: PluginDistribution;
}
export interface PluginAdminView {
  plugins: PluginAdminEntry[];
}
export interface RecentProjectSetting {
  name: string;
  root: string;
}
export interface GeneralSettings {
  recentProjects: RecentProjectSetting[];
}
export interface AppSettings {
  formatVersion: number;
  general: GeneralSettings;
  ai: AiSettings;
}
export interface AiSettings {
  provider: AiProviderSettings;
  consents: Array<{ projectId: string; provider: string; endpoint: string }>;
}
export interface AiProviderSettings {
  id: string;
  name: string;
  adapter: string;
  endpoint: string;
  model: string;
  embeddingModel: string;
  capabilities: string[];
}
export interface RemoteCredentialStatus {
  provider: string;
  configured: boolean;
}
export interface AppSettingsUpdate {
  general?: { recentProjects?: RecentProjectSetting[] };
  ai?: {
    provider?: Partial<AiProviderSettings>;
  };
}
export interface AiProviderStatus {
  endpoint: string;
  model: string;
  available: boolean;
  modelAvailable: boolean;
  embeddingAvailable: boolean;
  credentialAvailable: boolean;
  error: string | null;
}
export interface AiIndexStatus {
  available: boolean;
  state: "disabled" | "absent" | "indexing" | "ready" | "partially_stale" | "incompatible" | "failed" | null;
  provider: string | null;
  embeddingAvailable: boolean;
  message: string | null;
}
export interface AiIndexRebuildResult {
  chunkCount: number;
  embeddedCount: number;
  reusedCount: number;
  state: AiIndexStatus["state"];
}
export interface AiHybridMatch {
  chunkId: string;
  sourceId: string;
  sourceKind: string;
  score: number;
}
export interface AiStreamEvent {
  sequence: number;
  requestId: string;
  phase: "started" | "delta" | "usage" | "completed" | "cancelled" | "deadline_exceeded" | "failed";
  delta: string | null;
  output: string | null;
  error: string | null;
}
export interface HostViewData {
  lists: Record<string, Entity[]>;
  selected: Entity | null;
  fields: Record<string, unknown>;
}
export interface PluginUpgradePlan {
  pluginId: string;
  fromVersion: string | null;
  toVersion: string;
  consent: { added: string[]; removed: string[]; requiresRenewal: boolean };
  migrations: { from: number; to: number; migrationIds: string[]; requiresBackup: boolean };
  target: { signed: boolean; publisher: string };
}
type DialogSelection = string | string[] | null;
export interface ExternalImporterDescriptor {
  id: string;
  version: string;
  name: string;
  description: string;
  sourceKinds: Array<"file" | "folder">;
  extensions: string[];
}
export interface ExternalImportSourceHandle {
  sourceHandle: string;
  sourceKind: "file" | "folder";
  displayName: string;
}
export interface ImporterIdentity {
  id: string;
  version: string;
  name: string;
}
export interface ExternalImportSource {
  id: string;
  kind: "file" | "folder" | "archive" | "vault" | "wiki_dump" | "plugin";
  display_name: string;
}
export interface ImportDiagnostic {
  severity: "fatal" | "error" | "warning";
  code: string;
  message: string;
  source_path?: string | null;
  object_id?: string | null;
}
export interface StagedLink {
  kind: "internal" | "external" | "embed";
  target: string;
  label?: string | null;
  resolution: "unresolved" | "resolved" | "ambiguous" | "missing" | "not_applicable";
  resolved_object_id?: string | null;
  candidate_object_ids?: string[];
  raw?: string | null;
}
export interface StagedMappingHint {
  kind: "entity_type" | "field" | "relationship" | "hierarchy" | "asset_relationship" | "source_category";
  source_key?: string | null;
  suggested_value: unknown;
  confidence?: number | null;
  reason?: string | null;
}
export interface StagedObject {
  id: string;
  source_id: string;
  source_kind: string;
  source_path: string;
  content_hash: string;
  title: string;
  body?: { format: string; body: string } | null;
  parent_source_path?: string | null;
  tags?: string[];
  aliases?: string[];
  fields?: Record<string, unknown>;
  metadata?: Record<string, unknown>;
  raw_source_data?: Record<string, unknown>;
  links?: StagedLink[];
  mapping_hints?: StagedMappingHint[];
  diagnostics?: ImportDiagnostic[];
}
export interface StagedAsset {
  id: string;
  source_path: string;
  filename: string;
  size: number;
  mime_type?: string | null;
  content_hash?: string | null;
  owner_object_id?: string | null;
  relationship?: string | null;
  raw_metadata?: Record<string, unknown>;
  diagnostics?: ImportDiagnostic[];
}
export interface UnsupportedSourceData {
  source_path: string;
  source_kind: string;
  reason: string;
  raw_metadata?: Record<string, unknown>;
}
export interface ImportAnalysisSummary {
  document_count: number;
  candidate_entity_count: number;
  folder_count: number;
  asset_count: number;
  link_count: number;
  unresolved_link_count: number;
  unsupported_count: number;
  warning_count: number;
  error_count: number;
  total_source_bytes: number;
}
export interface ExternalImportResultMetadata {
  schemaVersion: number;
  importer: ImporterIdentity;
  source: ExternalImportSource;
  summary: ImportAnalysisSummary;
  totalItems: number;
  spilledToLocalStorage: boolean;
}
export interface ExternalImportAnalysisStatus {
  sessionId: string;
  importerId: string;
  state: "queued" | "analyzing" | "ready" | "failed" | "cancelled";
  stage: string;
  processedEntries: number;
  stagedObjectCount: number;
  unsupportedCount: number;
  sourceBytes: number;
  sequence: number;
  currentSourcePath: string | null;
  error: string | null;
  errorCode: string | null;
  capturedContentGeneration: number;
  currentContentGeneration: number | null;
  result: ExternalImportResultMetadata | null;
}
export type ExternalImportPageItem =
  | { kind: "object"; value: StagedObject }
  | { kind: "asset"; value: StagedAsset }
  | { kind: "unsupported"; value: UnsupportedSourceData }
  | { kind: "diagnostic"; value: ImportDiagnostic };
export interface ExternalImportPage {
  sessionId: string;
  offset: number;
  limit: number;
  totalItems: number;
  items: ExternalImportPageItem[];
}
export interface ExternalImportLimits {
  maxEntries: number;
  maxFiles: number;
  maxFileBytes: number;
  maxTotalBytes: number;
  maxDepth: number;
  maxDiagnostics: number;
}
export interface ImportMappingDecision {
  entityType?: string | null;
  fieldMappings?: Record<string, string>;
  relationshipMappings?: Record<string, string>;
}
export interface ImportMappingOverrides {
  global?: ImportMappingDecision;
  categories?: Record<string, ImportMappingDecision>;
  folders?: Record<string, ImportMappingDecision>;
  items?: Record<string, ImportMappingDecision>;
}
export interface ImportCandidateIssue {
  code: string;
  message: string;
  sourcePath?: string | null;
  objectId?: string | null;
}
export interface ImportCandidatePlanObject {
  stagedObjectId: string;
  sourceId: string;
  sourcePath: string;
  title: string;
  decision: "create";
  mapping: {
    entityType: string | null;
    fieldMappings: Record<string, string>;
    relationshipMappings: Record<string, string>;
  };
  issues: ImportCandidateIssue[];
}
export interface ImportCandidatePlan {
  schemaVersion: number;
  planId: string;
  sessionId: string;
  importer: ImporterIdentity;
  source: ExternalImportSource;
  capturedContentGeneration: number;
  currentContentGeneration: number;
  manifestFingerprint: string;
  objects: ImportCandidatePlanObject[];
  unsupportedCount: number;
  diagnostics: ImportDiagnostic[];
  issues: ImportCandidateIssue[];
  unresolvedDecisionCount: number;
}
export type ImportObjectDecision =
  { kind: "create" } | { kind: "skip" } | { kind: "map_to_existing"; entity_id: string; expected_revision: string };
export interface ImportValidationIssue {
  severity: "error" | "warning";
  code: string;
  message: string;
  sourcePath?: string | null;
  objectId?: string | null;
  existingEntityId?: string | null;
}
export interface ExternalImportValidationSummary {
  validationId: string | null;
  planId: string | null;
  createCount: number;
  skipCount: number;
  mapCount: number;
  assetCount: number;
  relationshipCount: number;
  warningCount: number;
  errorCount: number;
  issues: ImportValidationIssue[];
}
export interface ImportedObjectReport {
  stagedObjectId: string;
  sourcePath: string;
  entityId: string;
  entityType: string | null;
}
export interface ImportedAssetReport {
  stagedAssetId: string;
  sourcePath: string;
  assetId: string;
  entityId: string;
  filename: string;
  contentHash: string;
}
export interface ImportedRelationshipReport {
  relationshipId: string;
  sourceEntityId: string;
  targetEntityId: string;
  relationshipType: string;
}
export interface ImportedFieldReport {
  stagedObjectId: string;
  sourcePath: string;
  entityId: string;
  sourceKey: string;
  namespace: string;
  key: string;
}
export interface ImportDecisionReport {
  stagedObjectId: string;
  sourcePath: string;
  decision: "create" | "skip" | "map_to_existing";
  entityId?: string | null;
}
export interface ImportMissingReferenceReport {
  stagedObjectId: string;
  sourcePath: string;
  target: string;
  kind: "internal" | "external" | "embed";
}
export interface ExternalImportCommitReport {
  requestId: string;
  planId: string;
  importer: ImporterIdentity;
  source: ExternalImportSource;
  created: ImportedObjectReport[];
  mapped: ImportedObjectReport[];
  assets: ImportedAssetReport[];
  relationships: ImportedRelationshipReport[];
  fields: ImportedFieldReport[];
  decisions: ImportDecisionReport[];
  unsupported: UnsupportedSourceData[];
  missingReferences: ImportMissingReferenceReport[];
  diagnostics: ImportDiagnostic[];
  skippedSourcePaths: string[];
  warnings: ImportValidationIssue[];
}
export interface MutationOptions {
  expectedRevision?: string;
  requestId?: string;
}

const requestId = (options?: MutationOptions) => options?.requestId ?? crypto.randomUUID();

export const project = {
  openMemory: () => invoke<void>("project_open_memory"),
  openDefault: () => invoke<void>("project_open_default"),
  open: (path: string) => invoke<void>("project_open", { path }),
  openDirectory: (path: string) => invoke<ProjectInfo>("project_open_directory", { path }),
  pickDirectory: () => invoke<DialogSelection>("plugin:dialog|open", { options: { directory: true, multiple: false } }),
  pickFile: () => invoke<DialogSelection>("plugin:dialog|open", { options: { directory: false, multiple: false } }),
  create: (path: string) => invoke<ProjectInfo>("project_new", { path }),
  close: () => invoke<void>("project_close"),
  info: () => invoke<ProjectInfo | null>("project_info"),
  setAiEnabled: (enabled: boolean) => invoke<ProjectInfo>("project_set_ai_enabled", { enabled }),
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
  gitStatus: () => invoke<GitStatus>("project_git_status"),
  gitPreflight: () => invoke<GitPreflight>("project_git_preflight"),
  gitStagingPreview: () => invoke<GitPreflight>("project_git_staging_preview"),
  gitInit: () => invoke<GitStatus>("project_git_init"),
  gitLog: () => invoke<GitLogEntry[]>("project_git_log"),
  gitCommit: (message: string, paths?: string[]) =>
    invoke<GitStatus>("project_git_commit", { message, paths: paths ?? null }),
  gitSuperSquash: (message: string) => invoke<GitStatus>("project_git_super_squash", { message }),
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
  search: (query: string) => invoke<Entity[]>("project_search", { query }),
  deleteEntity: (id: string, options?: MutationOptions) =>
    invoke<void>("project_delete_entity", {
      id,
      expected_revision: options?.expectedRevision ?? null,
      request_id: requestId(options),
    }),
  createEntity: (name: string, entityType?: string, options?: MutationOptions) =>
    invoke<Entity>("project_create_entity", {
      input: { name, entity_type: entityType || null },
      request_id: requestId(options),
    }),
  createMap: (name = "Untitled map") => invoke<Entity>("project_create_map", { name }),
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
  mapsEditorSave: (pluginId?: string) => invoke<void>("maps_editor_save", { pluginId: pluginId ?? null }),
  mapsEditorCaptureAnchor: (pluginId?: string) =>
    invoke<void>("maps_editor_capture_anchor", { pluginId: pluginId ?? null }),
  mapsEditorStartPick: (pluginId?: string) => invoke<void>("maps_editor_start_pick", { pluginId: pluginId ?? null }),
  mapsEditorSetOverlay: (frame: unknown, pluginId?: string) =>
    invoke<void>("maps_editor_set_overlay", { pluginId: pluginId ?? null, frame }),
  mapsEditorSetDate: (date: unknown, pluginId?: string) =>
    invoke<void>("maps_editor_set_date", { pluginId: pluginId ?? null, date }),
  mapsEditorFocusLink: (linkId: string, pluginId?: string) =>
    invoke<void>("maps_editor_focus_link", { pluginId: pluginId ?? null, linkId }),
  mapsRecoveryList: (entityId: string) => invoke<MapRecoveryCopy[]>("maps_recovery_list", { entityId }),
  mapsRecoveryRestore: (entityId: string, fileName: string) =>
    invoke<Asset>("maps_recovery_restore", { entityId, fileName }),
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
  acceptVectorMap: (name: string, candidateJson: string, generation: unknown, options?: MutationOptions) =>
    invoke<{ entity: Entity; source: Asset }>("project_accept_vector_map", {
      name,
      candidateJson,
      generation,
      requestId: requestId(options),
    }),
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
  setModuleSchemaOverlay: (moduleId: string, overlay: ModuleSchemaOverlay) =>
    invoke<ModuleSchemaOverlay>("module_schema_overlay_set", { moduleId, overlay }),
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
  aiProviderStatus: () => invoke<AiProviderStatus>("ai_provider_status"),
  aiProviderModels: () => invoke<string[]>("ai_provider_models"),
  aiProviderCredentialStatus: () => invoke<RemoteCredentialStatus>("ai_provider_credential_status"),
  aiProviderImportCredential: () => invoke<RemoteCredentialStatus>("ai_provider_import_credential"),
  aiProviderSetCredential: (apiKey: string) => invoke<RemoteCredentialStatus>("ai_provider_set_credential", { apiKey }),
  aiProviderClearCredential: () => invoke<RemoteCredentialStatus>("ai_provider_clear_credential"),
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
  ) =>
    invoke<string>("ai_generate_text", {
      projectId,
      instruction,
      selection,
      entityId,
      retrievalQuery,
      retrievalDepth,
      includeRetrieval: true,
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
