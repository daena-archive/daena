//! Canonical RPC method payloads and wire shapes.
//!
//! These types pin down the exact wire names that `validate_broker_payload`
//! (`src-tauri/src/lib.rs`) enforces. Field names deliberately mirror the
//! frozen keys, including the mixed casing the contract has accumulated
//! (`expectedRevision`, `mapEntityId`, `source_id`). They are the single Rust
//! definition for Phase 1 contract generation and the Phase 2 data-driven
//! `RPC_METHOD_CATALOG`.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// `RpcSuccess` — `{ rpcVersion, requestId, ok: true, result }`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct RpcSuccess {
    #[serde(rename = "rpcVersion")]
    pub rpc_version: u32,
    #[serde(rename = "requestId")]
    pub request_id: String,
    pub ok: bool,
    pub result: Value,
}

/// `RpcFailure` — `{ rpcVersion, requestId, ok: false, error }`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct RpcFailure {
    #[serde(rename = "rpcVersion")]
    pub rpc_version: u32,
    #[serde(rename = "requestId")]
    pub request_id: String,
    pub ok: bool,
    pub error: super::RpcError,
}

/// Bootstrap envelope delivered to a plugin at activation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "gen", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct PluginBootstrap {
    #[serde(rename = "rpcVersion")]
    pub rpc_version: u32,
    #[serde(rename = "pluginId")]
    pub plugin_id: String,
    #[serde(rename = "sessionId")]
    pub session_id: String,
    #[serde(rename = "projectId")]
    pub project_id: String,
    pub version: String,
    #[serde(rename = "hostApi")]
    pub host_api: String,
    #[serde(rename = "grantedCapabilities")]
    pub granted_capabilities: Vec<String>,
    #[serde(rename = "optionalFeatures")]
    pub optional_features: Vec<String>,
}

/// Entity record as returned to callers.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "gen", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct EntityRecord {
    pub id: String,
    pub name: String,
    #[serde(rename = "entityType")]
    pub entity_type: Option<String>,
    pub deleted: bool,
    #[serde(rename = "createdAt")]
    pub created_at: String,
    #[serde(rename = "updatedAt")]
    pub updated_at: String,
    pub revision: String,
}

/// Authoring options for a migration (`recovery`, `description`).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "gen", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct MigrationAuthoringOptions {
    pub recovery: Option<String>,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "gen", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct EntityCreateField {
    pub namespace: String,
    pub key: String,
    pub value: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "gen", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct EntityCreateRelationship {
    #[serde(rename = "relationship_type")]
    pub relationship_type: String,
    #[serde(rename = "target_ids")]
    pub target_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "gen", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct EntityCreateDocument {
    pub body: String,
    pub format: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "gen", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct EntityListPayload {
    #[serde(rename = "entityType")]
    pub entity_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "gen", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct EntityQueryPayload {
    pub query: Option<String>,
    #[serde(rename = "entityTypes", default)]
    pub entity_types: Vec<String>,
    #[serde(rename = "excludedEntityTypes", default)]
    pub excluded_entity_types: Vec<String>,
    #[serde(rename = "sortField")]
    pub sort_field: Option<String>,
    #[serde(rename = "sortDirection")]
    pub sort_direction: Option<String>,
    pub offset: Option<u64>,
    pub limit: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "gen", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct EntityTypeCountRecord {
    #[serde(rename = "entityType")]
    pub entity_type: Option<String>,
    pub count: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "gen", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct EntityPageRecord {
    pub items: Vec<EntityRecord>,
    pub total: u64,
    pub offset: u64,
    pub limit: u32,
    #[serde(rename = "hasMore")]
    pub has_more: bool,
    #[serde(rename = "typeCounts")]
    pub type_counts: Vec<EntityTypeCountRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "gen", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct EntityGetPayload {
    pub id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "gen", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct EntityCreatePayload {
    pub name: String,
    #[serde(rename = "type")]
    pub r#type: Option<String>,
    #[serde(default)]
    pub fields: Vec<EntityCreateField>,
    #[serde(default)]
    pub relationships: Vec<EntityCreateRelationship>,
    #[serde(default)]
    pub document: Option<EntityCreateDocument>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "gen", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct EntityUpdatePayload {
    pub id: String,
    pub name: Option<String>,
    #[serde(rename = "type")]
    pub r#type: Option<String>,
    #[serde(rename = "expectedRevision")]
    pub expected_revision: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "gen", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct EntityDeletePayload {
    pub id: String,
    #[serde(rename = "expectedRevision")]
    pub expected_revision: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "gen", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct DocumentListPayload {
    #[serde(rename = "entityId")]
    pub entity_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "gen", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct DocumentSavePayload {
    #[serde(rename = "entityId")]
    pub entity_id: String,
    pub body: String,
    pub format: Option<String>,
    #[serde(rename = "expectedRevision")]
    pub expected_revision: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "gen", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct FieldReadPayload {
    #[serde(rename = "entityId")]
    pub entity_id: String,
    pub namespace: String,
    pub key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "gen", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct FieldListPayload {
    #[serde(rename = "entityId")]
    pub entity_id: String,
    pub namespace: String,
    #[serde(
        rename = "sharedOnly",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub shared_only: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "gen", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct FieldSetPayload {
    #[serde(rename = "entityId")]
    pub entity_id: String,
    pub namespace: String,
    pub key: String,
    pub value: Value,
    #[serde(rename = "expectedRevision")]
    pub expected_revision: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "gen", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct RecordListPayload {
    pub collection: String,
    #[serde(rename = "ownerEntityId")]
    pub owner_entity_id: String,
    pub query: Option<String>,
    pub limit: Option<u32>,
    pub offset: Option<u32>,
    pub sort: Option<String>,
    pub status: Option<String>,
    pub tag: Option<String>,
    #[serde(rename = "homonymsOnly")]
    pub homonyms_only: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "gen", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct RecordCreatePayload {
    pub collection: String,
    #[serde(rename = "ownerEntityId")]
    pub owner_entity_id: String,
    pub value: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "gen", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct RecordUpdatePayload {
    pub collection: String,
    pub id: String,
    #[serde(rename = "ownerEntityId")]
    pub owner_entity_id: String,
    pub value: Value,
    #[serde(rename = "expectedRevision")]
    pub expected_revision: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "gen", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct RecordDeletePayload {
    pub collection: String,
    pub id: String,
    #[serde(rename = "ownerEntityId")]
    pub owner_entity_id: String,
    #[serde(rename = "expectedRevision")]
    pub expected_revision: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "gen", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct RelationshipListPayload {
    #[serde(rename = "entityId")]
    pub entity_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "gen", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct RelationshipCreatePayload {
    #[serde(rename = "source_id")]
    pub source_id: String,
    #[serde(rename = "target_id")]
    pub target_id: String,
    #[serde(rename = "relationship_type")]
    pub relationship_type: String,
    pub metadata: Option<String>,
    #[serde(rename = "expectedRevision")]
    pub expected_revision: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "gen", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct RelationshipUpdatePayload {
    pub id: String,
    pub metadata: Option<String>,
    #[serde(rename = "target_id")]
    pub target_id: Option<String>,
    #[serde(rename = "expectedRevision")]
    pub expected_revision: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "gen", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct RelationshipDeletePayload {
    pub id: String,
    #[serde(rename = "relationship_type")]
    pub relationship_type: Option<String>,
    #[serde(rename = "expectedRevision")]
    pub expected_revision: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "gen", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct AssetListPayload {
    #[serde(rename = "entityId")]
    pub entity_id: String,
    pub namespace: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "gen", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct AssetRegisterPayload {
    #[serde(rename = "entity_id")]
    pub entity_id: String,
    pub namespace: String,
    #[serde(rename = "content_hash")]
    pub content_hash: String,
    #[serde(rename = "mime_type")]
    pub mime_type: String,
    pub filename: String,
    pub size: i64,
    pub path: String,
    #[serde(rename = "expectedRevision")]
    pub expected_revision: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "gen", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct AssetMetadataUpdatePayload {
    #[serde(rename = "assetId")]
    pub asset_id: String,
    pub namespace: String,
    pub filename: Option<String>,
    pub role: Option<String>,
    #[serde(rename = "referenceScope")]
    pub reference_scope: Option<String>,
    #[serde(rename = "expectedRevision")]
    pub expected_revision: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "gen", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct AssetDeletePayload {
    #[serde(rename = "assetId")]
    pub asset_id: String,
    pub namespace: String,
    #[serde(rename = "expectedRevision")]
    pub expected_revision: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "gen", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct AssetReadBeginPayload {
    #[serde(rename = "assetId")]
    pub asset_id: String,
    pub namespace: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "gen", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct AssetReplaceBeginPayload {
    #[serde(rename = "assetId")]
    pub asset_id: String,
    pub namespace: String,
    #[serde(rename = "expectedRevision")]
    pub expected_revision: String,
    pub size: i64,
    #[serde(rename = "mimeType")]
    pub mime_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "gen", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct AssetReplaceCommitPayload {
    pub handle: String,
    #[serde(rename = "contentHash")]
    pub content_hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "gen", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct AssetTransferCancelPayload {
    pub handle: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "gen", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct SearchQueryPayload {
    pub query: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "gen", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct MapsAssetCreateBeginPayload {
    #[serde(rename = "mapEntityId")]
    pub map_entity_id: String,
    pub size: i64,
    #[serde(rename = "mimeType")]
    pub mime_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "gen", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct MapsAssetCreateCommitPayload {
    pub handle: String,
    #[serde(rename = "contentHash")]
    pub content_hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "gen", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct MapsImageImportBeginPayload {
    pub name: String,
    pub size: i64,
    #[serde(rename = "mimeType")]
    pub mime_type: String,
    pub filename: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "gen", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct MapsImageImportCommitPayload {
    pub handle: String,
    #[serde(rename = "contentHash")]
    pub content_hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "gen", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct MapsVectorCreateBeginPayload {
    pub name: String,
    pub size: i64,
    pub generation: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "gen", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct MapsVectorCreateCommitPayload {
    pub handle: String,
    #[serde(rename = "contentHash")]
    pub content_hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "gen", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct MapsPhysicalCreateBeginPayload {
    pub name: String,
    pub size: i64,
    pub generation: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "gen", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct MapsPhysicalCreateCommitPayload {
    pub handle: String,
    #[serde(rename = "contentHash")]
    pub content_hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "gen", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct MapsVectorReplaceBeginPayload {
    #[serde(rename = "assetId")]
    pub asset_id: String,
    #[serde(rename = "expectedRevision")]
    pub expected_revision: String,
    pub size: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "gen", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct MapsVectorReplaceCommitPayload {
    pub handle: String,
    #[serde(rename = "contentHash")]
    pub content_hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "gen", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct MapsLayerCreatePayload {
    #[serde(rename = "mapEntityId")]
    pub map_entity_id: String,
    pub name: String,
    #[serde(rename = "expectedRevision")]
    pub expected_revision: String,
    pub kind: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "gen", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct MapsLayerDeletePayload {
    #[serde(rename = "mapEntityId")]
    pub map_entity_id: String,
    #[serde(rename = "layerId")]
    pub layer_id: String,
    #[serde(rename = "expectedRevision")]
    pub expected_revision: String,
    #[serde(rename = "expectedSourceRevision")]
    pub expected_source_revision: Option<String>,
    #[serde(rename = "expectedFeatureCount")]
    pub expected_feature_count: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "gen", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct MapsLayerUpdatePayload {
    #[serde(rename = "mapEntityId")]
    pub map_entity_id: String,
    #[serde(rename = "layerId")]
    pub layer_id: String,
    #[serde(rename = "expectedRevision")]
    pub expected_revision: String,
    pub name: Option<String>,
    pub order: Option<i64>,
    #[serde(rename = "defaultVisible")]
    pub default_visible: Option<bool>,
    pub opacity: Option<f64>,
    pub locked: Option<bool>,
    pub style: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "gen", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct MapsRecoveryExportBeginPayload {
    #[serde(rename = "mapEntityId")]
    pub map_entity_id: String,
    pub size: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "gen", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct MapsRecoveryExportCommitPayload {
    pub handle: String,
    #[serde(rename = "contentHash")]
    pub content_hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "gen", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct MapsRecoveryListPayload {
    #[serde(rename = "mapEntityId")]
    pub map_entity_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "gen", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct MapsRecoveryRestorePayload {
    #[serde(rename = "mapEntityId")]
    pub map_entity_id: String,
    #[serde(rename = "fileName")]
    pub file_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "gen", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct MapsLocationsListPayload {
    #[serde(rename = "mapEntityId")]
    pub map_entity_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "gen", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct MapsLocationsUpsertPayload {
    #[serde(rename = "entityId")]
    pub entity_id: String,
    /// Canonical `LocationReference` JSON. Validated by core on write.
    pub location: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "gen", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct MapsLocationsUnlinkPayload {
    #[serde(rename = "entityId")]
    pub entity_id: String,
    #[serde(rename = "locationId")]
    pub location_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "gen", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct MapsLocationsCreateAndLinkPayload {
    pub name: String,
    #[serde(rename = "entityType")]
    pub entity_type: String,
    /// Canonical `LocationReference` JSON. Validated by core on write.
    pub location: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "gen", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct MapsReconcileLinksPayload {
    #[serde(rename = "mapEntityId")]
    pub map_entity_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "gen", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct EventTypePayload {
    #[serde(rename = "type")]
    pub r#type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "gen", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct EventPublishPayload {
    #[serde(rename = "type")]
    pub r#type: String,
    pub payload: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "gen", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct ServiceCallPayload {
    pub name: String,
    pub major: u32,
    pub payload: Value,
    #[serde(rename = "deadlineMs")]
    pub deadline_ms: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "gen", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub enum AiRetrievalMode {
    #[serde(rename = "none")]
    None,
    #[serde(rename = "explicit_only")]
    ExplicitOnly,
    #[serde(rename = "related")]
    Related,
    #[serde(rename = "project")]
    Project,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "gen", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct AiRetrievalPolicyPayload {
    pub mode: AiRetrievalMode,
    pub query: Option<String>,
    #[serde(rename = "seedIds")]
    pub seed_ids: Vec<String>,
    #[serde(rename = "allowedSourceKinds")]
    pub allowed_source_kinds: Vec<String>,
    #[serde(rename = "relationshipDepth")]
    pub relationship_depth: u8,
    #[serde(rename = "passageCount")]
    pub passage_count: u16,
    #[serde(rename = "includeSharedFields")]
    pub include_shared_fields: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "gen", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct AiRequestStartPayload {
    pub operation: String,
    #[serde(rename = "taskId")]
    pub task_id: String,
    #[serde(rename = "userInstruction")]
    pub user_instruction: String,
    #[serde(rename = "immediateContext")]
    pub immediate_context: Value,
    #[serde(rename = "outputContract")]
    pub output_contract: Option<Value>,
    #[serde(rename = "deadlineMs")]
    pub deadline_ms: Option<u64>,
    #[serde(rename = "retrievalPolicy")]
    pub retrieval_policy: Option<AiRetrievalPolicyPayload>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "gen", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct AiRequestIdPayload {
    #[serde(rename = "requestId")]
    pub request_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "gen", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct AppVersionPayload {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn asset_register_uses_snake_case_wire_names() {
        let json = serde_json::json!({
            "entity_id": "e1",
            "namespace": "media",
            "filename": "cover.png",
            "content_hash": "sha256:abc",
            "size": 42,
            "mime_type": "image/png",
            "path": "media/cover.png",
            "expectedRevision": "rev-3"
        });
        let parsed: AssetRegisterPayload = serde_json::from_value(json.clone()).unwrap();
        let re_encoded = serde_json::to_value(parsed).unwrap();
        assert_eq!(re_encoded, json);
    }

    #[test]
    fn asset_metadata_update_uses_explicit_wire_names() {
        let json = serde_json::json!({
            "assetId": "asset-1",
            "namespace": "lore",
            "filename": "portrait.png",
            "role": "profile",
            "referenceScope": "project",
            "expectedRevision": "rev-4"
        });
        let parsed: AssetMetadataUpdatePayload = serde_json::from_value(json.clone()).unwrap();
        assert_eq!(parsed.asset_id, "asset-1");
        assert_eq!(parsed.reference_scope.as_deref(), Some("project"));
        assert_eq!(serde_json::to_value(parsed).unwrap(), json);
    }

    #[test]
    fn asset_delete_uses_explicit_wire_names() {
        let json = serde_json::json!({
            "assetId": "asset-1",
            "namespace": "lore",
            "expectedRevision": "rev-4"
        });
        let parsed: AssetDeletePayload = serde_json::from_value(json.clone()).unwrap();
        assert_eq!(parsed.asset_id, "asset-1");
        assert_eq!(serde_json::to_value(parsed).unwrap(), json);
    }

    #[test]
    fn relationship_create_round_trips_mixed_case() {
        let json = serde_json::json!({
            "source_id": "e1",
            "target_id": "e2",
            "relationship_type": "mentions",
            "metadata": "note",
            "expectedRevision": "rev-9"
        });
        let parsed: RelationshipCreatePayload = serde_json::from_value(json.clone()).unwrap();
        assert_eq!(parsed.source_id, "e1");
        assert_eq!(serde_json::to_value(parsed).unwrap(), json);
    }

    #[test]
    fn relationship_update_round_trips_optional_fields() {
        let json = serde_json::json!({
            "id": "relationship-1",
            "metadata": "{\"validFrom\":\"2024-01-01\"}",
            "target_id": "e2",
            "expectedRevision": "rev-10"
        });
        let parsed: RelationshipUpdatePayload = serde_json::from_value(json.clone()).unwrap();
        assert_eq!(parsed.target_id.as_deref(), Some("e2"));
        assert_eq!(serde_json::to_value(parsed).unwrap(), json);
    }

    #[test]
    fn maps_and_service_payloads_use_camel_wire_names() {
        let locations: MapsLocationsListPayload =
            serde_json::from_value(serde_json::json!({ "mapEntityId": "m1" })).unwrap();
        assert_eq!(locations.map_entity_id, "m1");

        let reconcile: MapsReconcileLinksPayload =
            serde_json::from_value(serde_json::json!({ "mapEntityId": "m1" })).unwrap();
        assert_eq!(reconcile.map_entity_id, "m1");

        let service: ServiceCallPayload = serde_json::from_value(serde_json::json!({
            "name": "daena.maps/navigation",
            "major": 1,
            "payload": { "dest": "m1" },
            "deadlineMs": 500
        }))
        .unwrap();
        assert_eq!(service.deadline_ms, Some(500));

        let publish: EventPublishPayload = serde_json::from_value(
            serde_json::json!({ "type": "daena.maps/state", "payload": {} }),
        )
        .unwrap();
        assert_eq!(publish.r#type, "daena.maps/state");
    }

    #[test]
    fn entity_create_accepts_missing_optional_collections() {
        let parsed: EntityCreatePayload =
            serde_json::from_value(serde_json::json!({ "name": "Map", "type": "daena.maps:map" }))
                .unwrap();
        assert!(parsed.fields.is_empty());
        assert!(parsed.relationships.is_empty());
        assert_eq!(parsed.document, None);
    }

    #[test]
    fn bootstrap_and_entity_record_round_trip() {
        let bootstrap = PluginBootstrap {
            rpc_version: 1,
            plugin_id: "daena.maps".into(),
            session_id: "s1".into(),
            project_id: "p1".into(),
            version: "0.1.0".into(),
            host_api: ">=1.0.0 <2.0.0".into(),
            granted_capabilities: vec!["asset.read:self".into()],
            optional_features: vec![],
        };
        let json = serde_json::to_value(&bootstrap).unwrap();
        assert_eq!(json["pluginId"], "daena.maps");
        assert_eq!(json["grantedCapabilities"][0], "asset.read:self");
        assert_eq!(
            serde_json::from_value::<PluginBootstrap>(json).unwrap(),
            bootstrap
        );

        let record: EntityRecord = serde_json::from_value(serde_json::json!({
            "id": "e1",
            "name": "Map",
            "entityType": "daena.maps:map",
            "deleted": false,
            "createdAt": "2026-01-01T00:00:00Z",
            "updatedAt": "2026-01-01T00:00:00Z",
            "revision": "rev-1"
        }))
        .unwrap();
        assert_eq!(record.entity_type.as_deref(), Some("daena.maps:map"));
    }

    #[test]
    fn rpc_success_and_failure_round_trip() {
        let success = RpcSuccess {
            rpc_version: 1,
            request_id: "r1".into(),
            ok: true,
            result: serde_json::json!({ "ok": true }),
        };
        let parsed: RpcSuccess =
            serde_json::from_value(serde_json::to_value(&success).unwrap()).unwrap();
        assert_eq!(parsed, success);

        let failure = RpcFailure {
            rpc_version: 1,
            request_id: "r2".into(),
            ok: false,
            error: super::super::RpcError {
                code: "method.unknown".into(),
                message: "nope".into(),
                retryable: false,
                details: None,
            },
        };
        let parsed: RpcFailure =
            serde_json::from_value(serde_json::to_value(&failure).unwrap()).unwrap();
        assert_eq!(parsed, failure);
    }
}
