// Project data-model types.
use serde::{Deserialize, Serialize};

pub(crate) fn current_snapshot_version() -> u32 {
    1
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateEntity {
    pub name: String,
    pub entity_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateEntryDocument {
    pub body: String,
    pub format: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateEntryField {
    pub namespace: String,
    pub key: String,
    pub value: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateEntryRelationship {
    pub relationship_type: String,
    pub target_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateEntry {
    pub name: String,
    pub entity_type: Option<String>,
    pub document: Option<CreateEntryDocument>,
    pub fields: Vec<CreateEntryField>,
    #[serde(default)]
    pub relationships: Vec<CreateEntryRelationship>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entity {
    pub id: String,
    pub name: String,
    pub entity_type: Option<String>,
    pub deleted: bool,
    pub created_at: String,
    pub updated_at: String,
    #[serde(default)]
    pub revision: String,
}

pub const DEFAULT_ENTITY_QUERY_LIMIT: u32 = 50;
pub const MAX_ENTITY_QUERY_LIMIT: u32 = 200;
pub const MAX_ENTITY_GET_MANY: usize = 500;
pub const MAX_RELATIONSHIP_QUERY_ENTITIES: usize = 200;
pub const DEFAULT_RELATIONSHIP_QUERY_LIMIT: u32 = 200;
pub const MAX_RELATIONSHIP_QUERY_LIMIT: u32 = 500;

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum EntitySortField {
    #[default]
    Name,
    CreatedAt,
    UpdatedAt,
    Relevance,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum EntitySortDirection {
    #[default]
    Asc,
    Desc,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct EntityListQuery {
    pub query: Option<String>,
    #[serde(default)]
    pub entity_types: Vec<String>,
    #[serde(default)]
    pub excluded_entity_types: Vec<String>,
    pub sort_field: Option<EntitySortField>,
    pub sort_direction: Option<EntitySortDirection>,
    pub offset: Option<u64>,
    pub limit: Option<u32>,
    pub archived: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EntityTypeCount {
    pub entity_type: Option<String>,
    pub count: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityPage {
    pub items: Vec<Entity>,
    pub total: u64,
    pub offset: u64,
    pub limit: u32,
    pub has_more: bool,
    pub type_counts: Vec<EntityTypeCount>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SaveDocument {
    pub entity_id: String,
    pub body: String,
    pub format: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SaveEntry {
    pub document: SaveDocument,
    pub fields: Vec<FieldValue>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Document {
    pub id: String,
    pub entity_id: String,
    pub format: String,
    pub body: String,
    pub updated_at: String,
    #[serde(default)]
    pub revision: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchPassage {
    pub entity_id: String,
    pub source_path: String,
    pub source_hash: String,
    pub content: String,
    pub lexical_rank: f64,
    pub source_kind: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct MapFeatureSearchResult {
    pub map_entity_id: String,
    pub map_name: String,
    pub feature_id: String,
    pub name: String,
    pub semantic_type: String,
    pub layer_id: String,
    pub layer_name: String,
    pub rank: f64,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum WikiPageExportFormat {
    Markdown,
    Html,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldValue {
    pub entity_id: String,
    pub namespace: String,
    pub key: String,
    pub value: serde_json::Value,
    #[serde(default)]
    pub revision: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelationshipInput {
    pub source_id: String,
    pub target_id: String,
    pub relationship_type: String,
    pub metadata: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelationshipUpdate {
    pub id: String,
    pub metadata: Option<String>,
    pub target_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Relationship {
    pub id: String,
    pub source_id: String,
    pub target_id: String,
    pub relationship_type: String,
    pub metadata: String,
    #[serde(default)]
    pub revision: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RelationshipQueryDirection {
    Incoming,
    Outgoing,
    Any,
}

#[derive(Debug, Clone)]
pub struct RelationshipQuery {
    pub entity_ids: Vec<String>,
    pub relationship_types: Vec<String>,
    pub direction: RelationshipQueryDirection,
    pub offset: Option<u64>,
    pub limit: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelationshipPage {
    pub items: Vec<Relationship>,
    pub total: u64,
    pub offset: u64,
    pub limit: u32,
    pub has_more: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetInput {
    pub entity_id: String,
    pub namespace: String,
    pub filename: String,
    pub content_hash: String,
    pub size: i64,
    pub mime_type: String,
    pub path: String,
    #[serde(default)]
    pub provenance: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetFileInput {
    pub entity_id: String,
    pub namespace: String,
    pub source_path: String,
    pub filename: String,
    pub mime_type: String,
    #[serde(default)]
    pub provenance: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetReplaceInput {
    pub asset_id: String,
    pub content_hash: String,
    pub size: i64,
    pub mime_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetFileReplaceInput {
    pub asset_id: String,
    pub source_path: String,
    pub mime_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetMetadataUpdate {
    pub asset_id: String,
    pub filename: Option<String>,
    pub role: Option<String>,
    pub reference_scope: Option<String>,
}

pub const ASSET_ROLE_ATTACHMENT: &str = "attachment";
pub const ASSET_ROLE_PROFILE: &str = "profile";
pub const ASSET_REFERENCE_SCOPE_ENTITY: &str = "entity";
pub const ASSET_REFERENCE_SCOPE_PROJECT: &str = "project";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Asset {
    pub id: String,
    pub entity_id: String,
    pub namespace: String,
    pub filename: String,
    pub content_hash: String,
    pub size: i64,
    pub mime_type: String,
    pub path: String,
    pub created_at: String,
    pub role: String,
    pub reference_scope: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provenance: Option<serde_json::Value>,
    #[serde(default)]
    pub revision: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportedImageMap {
    pub entity: Entity,
    pub source: Asset,
    pub preview: Asset,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AttachedMapRaster {
    pub asset: Asset,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AcceptedVectorMap {
    pub entity: Entity,
    pub source: Asset,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AcceptedPhysicalMap {
    pub entity: Entity,
    pub source: Asset,
    pub physical_identity: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorSourceReplace {
    pub source: Asset,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MapLinkMutation {
    pub entity_id: String,
    pub expected_locations_revision: String,
    pub locations: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MapEditApply {
    pub map: FieldValue,
    pub layers: FieldValue,
    pub source: Asset,
    #[serde(default)]
    pub locations: Vec<FieldValue>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorLayerDelete {
    pub layers: FieldValue,
    pub source: Asset,
    #[serde(rename = "deletedFeatureCount")]
    pub deleted_feature_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RasterLayerChange {
    pub layer_id: String,
    pub asset: Option<Asset>,
    pub layers: FieldValue,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RasterLayerUpdate {
    pub name: Option<String>,
    pub order: Option<i64>,
    pub default_visible: Option<bool>,
    pub opacity: Option<f64>,
    pub locked: Option<bool>,
    #[serde(default)]
    pub style: Option<serde_json::Value>,
    #[serde(default)]
    pub selector: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleState {
    pub module_id: String,
    pub enabled: bool,
    pub version: i64,
    #[serde(default)]
    pub package_version: Option<String>,
    /// Opaque project schema overlay for this module (Lore uses this).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub schema_overlay: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleNamespace {
    pub module_id: String,
    pub namespace: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleField {
    pub module_id: String,
    pub namespace: String,
    pub key: String,
    pub field_type: String,
    pub required: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ModuleRecord {
    pub module_id: String,
    pub collection: String,
    pub id: String,
    pub owner_entity_id: String,
    pub value: serde_json::Value,
    pub created_at: String,
    pub updated_at: String,
    #[serde(default)]
    pub revision: String,
}

#[derive(Debug, Clone, Default)]
pub struct ModuleRecordListParams<'a> {
    pub query: Option<&'a str>,
    pub limit: usize,
    pub offset: usize,
    pub sort: Option<&'a str>,
    pub status: Option<&'a str>,
    pub tag: Option<&'a str>,
    pub homonyms_only: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationHistoryEntry {
    pub module_id: String,
    pub migration_id: String,
    pub from_version: i64,
    pub to_version: i64,
    pub checksum: String,
    #[serde(default)]
    pub package_digest: String,
    #[serde(default)]
    pub applied_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PluginBackup {
    pub id: String,
    pub module_id: String,
    pub from_package_version: Option<String>,
    pub to_package_version: Option<String>,
    pub data_version: i64,
    pub path: String,
    pub content_hash: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectSnapshot {
    #[serde(default = "current_snapshot_version")]
    pub format_version: u32,
    pub entities: Vec<Entity>,
    pub documents: Vec<Document>,
    #[serde(default)]
    pub fields: Vec<FieldValue>,
    pub relationships: Vec<Relationship>,
    #[serde(default)]
    pub assets: Vec<Asset>,
    #[serde(default)]
    pub modules: Vec<ModuleState>,
    #[serde(default)]
    pub module_namespaces: Vec<ModuleNamespace>,
    #[serde(default)]
    pub module_fields: Vec<ModuleField>,
    #[serde(default)]
    pub module_records: Vec<ModuleRecord>,
    #[serde(default)]
    pub migration_history: Vec<MigrationHistoryEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProjectInfo {
    pub name: String,
    pub root: String,
    pub index_status: String,
    pub assets: String,
    pub sync: SyncSummary,
    #[serde(rename = "aiEnabled", alias = "ai_enabled", default)]
    pub ai_enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SyncSummary {
    pub state: String,
    pub dirty_count: i64,
    pub export_error: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GitStatus {
    pub repository: bool,
    pub branch: Option<String>,
    pub changes: Vec<String>,
    #[serde(default)]
    pub canonical_changes: Vec<String>,
    #[serde(default)]
    pub staged_canonical_changes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitLogEntry {
    pub hash: String,
    pub date: String,
    pub subject: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitChange {
    pub status: String,
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GitPreflight {
    pub ready: bool,
    pub diagnostics: Vec<String>,
    pub canonical_paths: Vec<String>,
    pub asset_paths: Vec<String>,
    pub staging_paths: Vec<String>,
    pub staged_paths: Vec<String>,
    pub unmerged_paths: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct GitToolInfo {
    pub available: bool,
    pub version: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct GitRemote {
    pub name: String,
    pub fetch_url: String,
    pub push_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct GitUpstream {
    pub remote: String,
    pub branch: String,
    pub remote_hash: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GitResetResult {
    pub status: GitStatus,
    pub previous_head: Option<String>,
    pub current_head: Option<String>,
    pub upstream: Option<GitUpstream>,
    pub diverged_from_upstream: bool,
    pub rebuild: ExternalChangeReport,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ExternalChangeReport {
    pub changed: bool,
    pub paths: Vec<String>,
    pub diagnostics: Vec<String>,
}

pub type Generation = i64;
