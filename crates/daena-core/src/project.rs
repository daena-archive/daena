use crate::error::CoreError;
use crate::external_import::{
    is_docx_import_asset_source_path, read_archive_asset_bytes, read_docx_import_asset_bytes,
    ExternalImportCommitReport, ImportDecisionReport, ImportExistingTarget,
    ImportMissingReferenceReport, ImportObjectDecision, ImportSourceKind, ImportedAssetReport,
    ImportedFieldReport, ImportedObjectReport, ImportedRelationshipReport, StagedLinkKind,
    StagedLinkResolution, ValidatedImportPlan, VALIDATED_IMPORT_PLAN_SCHEMA_VERSION,
};
use daena_plugin_api::{
    MetadataFieldDefinition, PluginManifest, RelationshipConstraints, RelationshipUniqueness,
};
use rusqlite::{
    named_params, params, types::Value as SqlValue, Connection, OpenFlags, OptionalExtension,
};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::cell::Cell;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Mutex, OnceLock};
use std::time::Duration;
use uuid::Uuid;

const EXTERNAL_IMPORT_SOURCE_NAMESPACE: &str = "daena.core";
const EXTERNAL_IMPORT_SOURCE_KEY_PREFIX: &str = "externalImportSource.";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ModuleSchemaOverlayReceipt {
    module_id: String,
    overlay: serde_json::Value,
    revision: String,
}

mod assets;
mod checkpoint;
mod fields;
mod fs_util;
mod markdown;
mod model;
mod module_records;
mod projection;
mod runtime_assets;
mod store_assets;
mod store_backup;
mod store_entities;
mod store_git;
mod store_map_layers;
mod store_map_locations;
mod store_maps;
mod store_modules;
mod store_open;
mod store_relationships;
mod store_revisions;
mod store_writer;

use self::assets::*;
use self::checkpoint::*;
pub use self::checkpoint::{set_checkpoint_export_status_listener, CheckpointHandle};
use self::fields::*;
pub(crate) use self::fs_util::chrono_like_now;
use self::fs_util::*;
use self::markdown::*;
use self::model::current_snapshot_version;
pub use self::model::{
    AcceptedPhysicalMap, AcceptedVectorMap, Asset, AssetFileInput, AssetFileReplaceInput,
    AssetInput, AssetMetadataUpdate, AssetReplaceInput, AttachedMapRaster, CreateEntity,
    CreateEntry, CreateEntryDocument, CreateEntryField, CreateEntryRelationship, Document, Entity,
    EntityListQuery, EntityPage, EntitySortDirection, EntitySortField, EntityTypeCount,
    ExternalChangeReport, FieldValue, Generation, GitChange, GitLogEntry, GitPreflight, GitRemote,
    GitResetResult, GitStatus, GitToolInfo, GitUpstream, ImportedImageMap, MapEditApply,
    MapFeatureSearchResult, MapLinkMutation, MigrationHistoryEntry, ModuleField, ModuleNamespace,
    ModuleRecord, ModuleRecordListParams, ModuleState, PluginBackup, ProjectInfo, ProjectSnapshot,
    RasterLayerChange, RasterLayerUpdate, Relationship, RelationshipInput, RelationshipPage,
    RelationshipQuery, RelationshipQueryDirection, RelationshipUpdate, SaveDocument, SaveEntry,
    SearchPassage, SyncSummary, VectorLayerDelete, VectorSourceReplace, WikiPageExportFormat,
    ASSET_REFERENCE_SCOPE_ENTITY, ASSET_REFERENCE_SCOPE_PROJECT, ASSET_ROLE_ATTACHMENT,
    ASSET_ROLE_PROFILE, DEFAULT_ENTITY_QUERY_LIMIT, DEFAULT_RELATIONSHIP_QUERY_LIMIT,
    MAX_ENTITY_GET_MANY, MAX_ENTITY_QUERY_LIMIT, MAX_RELATIONSHIP_QUERY_ENTITIES,
    MAX_RELATIONSHIP_QUERY_LIMIT,
};
use self::module_records::*;
use self::projection::*;
#[cfg(test)]
pub(crate) use self::runtime_assets::FAIL_NEXT_RUNTIME_ASSET_INSTALL;
use self::runtime_assets::*;

const GIT_SHOW_FILE_MAX_BYTES: usize = 512 * 1024;

static GIT_REPOSITORY_PROBES: OnceLock<Mutex<BTreeMap<PathBuf, bool>>> = OnceLock::new();

fn git_repository_probe_slot() -> &'static Mutex<BTreeMap<PathBuf, bool>> {
    GIT_REPOSITORY_PROBES.get_or_init(|| Mutex::new(BTreeMap::new()))
}

static GIT_TOOL_INFO_CACHE: OnceLock<Mutex<Option<GitToolInfo>>> = OnceLock::new();

fn git_tool_info_slot() -> &'static Mutex<Option<GitToolInfo>> {
    GIT_TOOL_INFO_CACHE.get_or_init(|| Mutex::new(None))
}

pub struct ProjectStore {
    connection: Connection,
    database_epoch: String,
    root: Option<PathBuf>,
    relationship_metadata_schemas: BTreeMap<String, Vec<MetadataFieldDefinition>>,
    relationship_constraints: BTreeMap<String, RelationshipConstraints>,
    suppress_sync: Cell<bool>,
    _session_lock: Option<crate::sync::ProjectSessionLock>,
    export_worker: Option<ExportWorker>,
}

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

fn git_command(args: &[&str]) -> Command {
    let mut command = Command::new("git");
    command.args(args);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(CREATE_NO_WINDOW);
    }
    command
}

impl Drop for ProjectStore {
    fn drop(&mut self) {
        if let Some(worker) = self.export_worker.take() {
            let _ = worker.stop();
        }
    }
}

#[cfg(test)]
mod tests;
