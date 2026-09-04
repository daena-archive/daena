import type { ModuleManifest } from "../../../packages/module-api/src/index";
import type {
  EntityTypeColor,
  EntityTypeDefinition,
  EntityTemplate,
  FieldDefinition,
  IconRef,
  MetadataFieldDefinition,
} from "../../../packages/plugin-sdk/src/generated";

export type {
  EntityTypeColor,
  EntityTypeDefinition,
  EntityTemplate,
  FieldDefinition,
  IconRef,
  MetadataFieldDefinition,
};

export interface EntityTypeAppearanceOverride {
  entityTypeId: string;
  icon?: IconRef;
  iconColor?: EntityTypeColor;
}

export interface FieldScopeOverride {
  fieldKey: string;
  entityTypes: string[];
}

export interface TemplateOverride {
  templateId: string;
  fields: Record<string, unknown>;
  requiredFields?: string[] | null;
}

export interface FieldMetadataOverride {
  fieldKey: string;
  metadataFields: MetadataFieldDefinition[];
}

export interface FieldTimelineOverride {
  fieldKey: string;
  timeline?: {
    role: "point" | "start" | "end";
    group?: string | null;
    label?: string | null;
    layer?: "dates" | "lifelines" | null;
  } | null;
}

export interface ModuleSchemaOverlay {
  version: number;
  disabledEntityTypes?: string[];
  disabledFields?: string[];
  disabledTemplates?: string[];
  customEntityTypes?: EntityTypeDefinition[];
  customFields?: FieldDefinition[];
  customTemplates?: EntityTemplate[];
  fieldScopeOverrides?: FieldScopeOverride[];
  templateOverrides?: TemplateOverride[];
  fieldMetadataOverrides?: FieldMetadataOverride[];
  entityTypeAppearanceOverrides?: EntityTypeAppearanceOverride[];
  fieldTimelineOverrides?: FieldTimelineOverride[];
}

export type SchemaOverlayChangeKind = "additive" | "hiding-only" | "requires-reassignment";

export interface SchemaOverlayItemIssue {
  kind: string;
  id: string;
  property?: string;
  message: string;
}

export interface SchemaOverlayTypeImpact {
  entityType: string;
  change: string;
  entityCount: number;
}

export interface SchemaOverlayFieldImpact {
  fieldKey: string;
  change: string;
  valueCount: number;
}

export interface SchemaOverlayPreviewResult {
  ok: boolean;
  changeKind: SchemaOverlayChangeKind;
  requiresAcknowledgement: boolean;
  errors: SchemaOverlayItemIssue[];
  warnings: SchemaOverlayItemIssue[];
  affectedTypes: SchemaOverlayTypeImpact[];
  affectedFields: SchemaOverlayFieldImpact[];
  affectedTemplates: string[];
  relationshipMetadataKeys: string[];
  compatibilityNotes: string[];
  unresolvedTypeRemovals: string[];
}

export interface ModuleSchemaOverlayMutationResult {
  overlay: ModuleSchemaOverlay;
  revision: string;
}

export interface ModuleSchemaEditorState {
  id: string;
  name: string;
  schemas: Array<{
    namespace: string;
    entityTypes: EntityTypeDefinition[];
    fields: FieldDefinition[];
  }>;
  templates: EntityTemplate[];
  overlay: ModuleSchemaOverlay;
  revision: string;
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
export interface MapFeatureSearchResult {
  mapEntityId: string;
  mapName: string;
  featureId: string;
  name: string;
  semanticType: string;
  layerId: string;
  layerName: string;
  rank: number;
}
export type EntitySortField = "name" | "created_at" | "updated_at" | "relevance";
export type EntitySortDirection = "asc" | "desc";
export interface EntityListQuery {
  query?: string;
  entityTypes?: string[];
  excludedEntityTypes?: string[];
  sortField?: EntitySortField;
  sortDirection?: EntitySortDirection;
  offset?: number;
  limit?: number;
  archived?: boolean;
}
export interface EntityTypeCount {
  entity_type: string | null;
  count: number;
}
export interface EntityPage {
  items: Entity[];
  total: number;
  offset: number;
  limit: number;
  has_more: boolean;
  type_counts: EntityTypeCount[];
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
  provider?: string | null;
  featureKind?: string | null;
  featureId?: string | null;
}
export interface RasterLayerChange {
  layer_id: string;
  asset: Asset | null;
  layers: FieldValue;
}
export interface VectorSourceReplace {
  source: Asset;
}
export interface MapLinkMutation {
  entityId: string;
  expectedLocationsRevision: string;
  locations: unknown;
}
export interface MapEditApply {
  map: FieldValue;
  layers: FieldValue;
  source: Asset;
  locations?: FieldValue[];
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
  temperatureNhSummerCentiC: number[];
  temperatureNhWinterCentiC: number[];
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
    meanSeasonalRangeCentiC: number;
    minimumSeasonalTemperatureCentiC: number;
    maximumSeasonalTemperatureCentiC: number;
    permanentlyFrozenLandPpm: number;
    seasonallyFrozenLandPpm: number;
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
  temperatureNhSummerCentiC: number;
  temperatureNhWinterCentiC: number;
  seasonalRangeCentiC: number;
  freeze: "none" | "seasonal" | "permanent";
  precipitationMm: number;
  climate: string;
  surface: string;
  iceThicknessMm: number;
}
export interface AtlasStudioInspectResult {
  hits: AtlasStudioInspectHit[];
  surface: AtlasStudioSurfaceSample;
}
export type { PlanetaryConfiguration, PlanetaryPreset } from "../maps/physical/planetary";
export interface PhysicalGenerationInput {
  seed: number;
  retryIndex: number;
  evolutionPreset?: "young" | "mature" | "old";
  settings: {
    width: number;
    height: number;
    radiusMetres: number;
    targetLandFractionPpm: number;
    planetary?: PlanetaryConfiguration;
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
  provenance?: Record<string, unknown> | null;
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
export type WikiPageExportFormat = "markdown" | "html";
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
  appearance: AppearanceSettings;
}
export type ThemePreference = "light" | "dark" | "system";
export type UpdateChannelPreference = "auto" | "stable" | "beta" | "alpha";
export interface AppearanceSettings {
  theme: ThemePreference;
  updateChannel?: UpdateChannelPreference;
}
export interface AppSettings {
  formatVersion: number;
  general: GeneralSettings;
  ai: AiSettings;
}
export interface AiSettings {
  projectBindings: Record<string, AiProviderSettings>;
  imageProvider: ImageProviderSettings;
  consents: Array<{ projectId: string; provider: string; endpoint: string }>;
}
export interface ImageProviderSettings {
  enabled: boolean;
  id: string;
  name: string;
  adapter: string;
  endpoint: string;
  model: string;
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
  general?: {
    recentProjects?: RecentProjectSetting[];
    appearance?: Partial<AppearanceSettings>;
  };
  ai?: {
    projectId?: string;
    provider?: Partial<AiProviderSettings>;
    imageProvider?: Partial<ImageProviderSettings>;
  };
}

export interface ImageProviderDiscovery {
  providerId: string;
  providerName: string;
  endpoint: string;
  local: true;
  capabilities: string[];
  models: string[];
  samplers: string[];
  schedulers: string[];
}

export interface ImageProviderStatus {
  providerId: string;
  providerName: string;
  endpoint: string;
  model: string;
  enabled: boolean;
  local: true;
  available: boolean;
  modelAvailable: boolean;
  capabilities: string[];
  errorCode: string | null;
  error: string | null;
}

export interface ImageContextItem {
  entityId: string;
  label: string;
  sourceKind: "identity" | "field" | "document" | "relationship" | "timeline" | "location";
}

export interface ImagePromptProvenance {
  method: "manual" | "entity" | "selected-context" | "rewrite" | "detailed" | "simplified";
  llmAssisted: boolean;
  editedAfterAssistance: boolean;
  textProviderId: string | null;
  textModel: string | null;
}

export interface ImageGenerationRequest {
  projectId: string;
  entityId: string;
  prompt: string;
  negativePrompt: string;
  model: string;
  width: number;
  height: number;
  seed: number;
  outputCount: number;
  steps: number;
  guidanceScale: number;
  sampler: string;
  scheduler: string;
  context: ImageContextItem[];
  promptProvenance: ImagePromptProvenance;
}

export interface ImageCandidate {
  id: string;
  filename: string;
  mimeType: "image/png" | "image/jpeg" | "image/webp";
  size: number;
  width: number;
  height: number;
  seed: number;
  acceptedAssetId: string | null;
}

export interface ImageGenerationStatus {
  jobId: string;
  state: "queued" | "running" | "downloading" | "completed" | "failed" | "cancelled";
  stage: string;
  completed: number;
  total: number;
  queuePosition: number | null;
  candidates: ImageCandidate[];
  errorCode: string | null;
  error: string | null;
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
export interface AiProviderConnectResult {
  status: AiProviderStatus;
  models: string[];
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
  phase: "started" | "reasoning" | "delta" | "usage" | "completed" | "cancelled" | "deadline_exceeded" | "failed";
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
export type DialogSelection = string | string[] | null;
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
  /** Required when preview reports live-data impact that must be acknowledged. */
  acknowledgeImpact?: boolean;
}
