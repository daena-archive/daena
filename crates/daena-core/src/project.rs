use crate::error::CoreError;
use rusqlite::{named_params, params, Connection, OpenFlags, OptionalExtension};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::cell::Cell;
use std::collections::{BTreeMap, BTreeSet};
use std::fs::OpenOptions;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::mpsc;
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};
use uuid::Uuid;

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
pub struct Relationship {
    pub id: String,
    pub source_id: String,
    pub target_id: String,
    pub relationship_type: String,
    pub metadata: String,
    #[serde(default)]
    pub revision: String,
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
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetFileInput {
    pub entity_id: String,
    pub namespace: String,
    pub source_path: String,
    pub filename: String,
    pub mime_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetReplaceInput {
    pub asset_id: String,
    pub content_hash: String,
    pub size: i64,
    pub mime_type: String,
}

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
    #[serde(default)]
    pub revision: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportedImageMap {
    pub entity: Entity,
    pub source: Asset,
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
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SyncSummary {
    pub state: String,
    pub dirty_count: i64,
    pub export_error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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

const GIT_SHOW_FILE_MAX_BYTES: usize = 512 * 1024;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ExternalChangeReport {
    pub changed: bool,
    pub paths: Vec<String>,
    pub diagnostics: Vec<String>,
}

pub type Generation = i64;

fn current_snapshot_version() -> u32 {
    1
}

fn decode_field_value(value: String) -> serde_json::Value {
    serde_json::from_str(&value).unwrap_or(serde_json::Value::String(value))
}

fn encode_field_value(value: &serde_json::Value) -> Result<String, CoreError> {
    serde_json::to_string(value).map_err(|error| CoreError::NotFound(error.to_string()))
}

fn validate_document_format(format: Option<&str>, directory_backed: bool) -> Result<(), CoreError> {
    let format = format.unwrap_or("markdown");
    if directory_backed && format != "markdown" {
        return Err(CoreError::Validation(
            "directory-backed projects require Markdown documents".into(),
        ));
    }
    if format != "markdown" && format != "plain-text" && format != "rich-text" {
        return Err(CoreError::NotFound("unsupported document format".into()));
    }
    Ok(())
}

fn markdown_export_stem(value: &str) -> String {
    let mut stem = value
        .chars()
        .map(|character| {
            if character.is_control() || matches!(character, '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|') {
                '_'
            } else {
                character
            }
        })
        .collect::<String>();
    stem = stem.trim().trim_matches('.').to_string();
    if stem.is_empty() || stem == "." || stem == ".." {
        stem = "Untitled".into();
    }
    stem.chars().take(120).collect()
}

fn markdown_export_target(filename: &str) -> String {
    let mut target = String::new();
    for byte in filename.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.') {
            target.push(byte as char);
        } else if byte == b' ' {
            target.push_str("%20");
        } else {
            target.push_str(&format!("%{byte:02X}"));
        }
    }
    target
}

fn markdown_escape_label(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('[', "\\[")
        .replace(']', "\\]")
}

fn rewrite_markdown_entity_links(body: &str, filenames: &BTreeMap<String, String>) -> String {
    let mut output = String::with_capacity(body.len());
    let mut in_fence = false;
    for (line_index, line) in body.split('\n').enumerate() {
        if line_index > 0 {
            output.push('\n');
        }
        let trimmed = line.trim_start();
        if trimmed.starts_with("```") || trimmed.starts_with("~~~") {
            in_fence = !in_fence;
            output.push_str(line);
            continue;
        }
        if in_fence {
            output.push_str(line);
            continue;
        }

        let mut cursor = 0;
        while cursor < line.len() {
            if line[cursor..].starts_with('`') {
                if let Some(end) = line[cursor + 1..].find('`') {
                    let end = cursor + end + 2;
                    output.push_str(&line[cursor..end]);
                    cursor = end;
                    continue;
                }
            }
            if line[cursor..].starts_with("[[") {
                if let Some(label_end) = line[cursor + 2..].find("]](") {
                    let label_start = cursor + 2;
                    let label_end = label_start + label_end;
                    let id_start = label_end + 3;
                    if let Some(id_end) = line[id_start..].find(')') {
                        let id_end = id_start + id_end;
                        let entity_id = &line[id_start..id_end];
                        if let Some(filename) = filenames.get(entity_id) {
                            let label = &line[label_start..label_end];
                            output.push('[');
                            output.push_str(&markdown_escape_label(label));
                            output.push_str("](");
                            output.push_str(&markdown_export_target(filename));
                            output.push(')');
                            cursor = id_end + 1;
                            continue;
                        }
                    }
                }
            }
            if line[cursor..].starts_with('[') && !line[cursor..].starts_with("[[") {
                if let Some(label_end) = line[cursor + 1..].find("](") {
                    let label_start = cursor + 1;
                    let label_end = label_start + label_end;
                    let target_start = label_end + 2;
                    if let Some(target_end) = line[target_start..].find(')') {
                        let target_end = target_start + target_end;
                        if let Some(entity_id) = line[target_start..target_end].strip_prefix("daena://entity/") {
                            if let Some(filename) = filenames.get(entity_id) {
                                let label = &line[label_start..label_end];
                                output.push('[');
                                output.push_str(&markdown_escape_label(label));
                                output.push_str("](");
                                output.push_str(&markdown_export_target(filename));
                                output.push(')');
                                cursor = target_end + 1;
                                continue;
                            }
                        }
                    }
                }
            }
            if let Some(next) = line[cursor..].chars().next() {
                output.push(next);
                cursor += next.len_utf8();
            } else {
                break;
            }
        }
    }
    output
}

fn markdown_relationship_heading(value: &str) -> String {
    let label = value.replace(['_', '-'], " ");
    let mut characters = label.chars();
    match characters.next() {
        Some(first) => first.to_uppercase().collect::<String>() + characters.as_str(),
        None => "Relationship".into(),
    }
}

fn project_database_path(root: &Path) -> PathBuf {
    root.join(".daena/index.sqlite")
}

fn runtime_asset_path(root: &Path, content_hash: &str) -> Result<PathBuf, CoreError> {
    let digest = content_hash
        .strip_prefix("sha256:")
        .ok_or_else(|| CoreError::Validation("asset content hash must use sha256".into()))?;
    if digest.len() != 64 || !digest.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(CoreError::Validation(
            "asset content hash must contain a 64-character SHA-256 digest".into(),
        ));
    }
    Ok(root.join(".daena/assets").join(digest.to_ascii_lowercase()))
}

fn store_runtime_asset<R: Read>(
    root: &Path,
    mut input: R,
    expected_hash: Option<&str>,
) -> Result<(String, i64), CoreError> {
    let directory = root.join(".daena/assets");
    std::fs::create_dir_all(&directory).map_err(|source| CoreError::Io {
        operation: "create runtime asset directory",
        source,
    })?;
    let temporary = directory.join(format!(".tmp-{}", Uuid::new_v4()));
    let mut output = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temporary)
        .map_err(|source| CoreError::Io {
            operation: "create runtime asset staging file",
            source,
        })?;
    let result = (|| {
        let mut digest = Sha256::new();
        let mut buffer = [0_u8; 128 * 1024];
        let mut size = 0_i64;
        loop {
            let count = input.read(&mut buffer).map_err(|source| CoreError::Io {
                operation: "read runtime asset input",
                source,
            })?;
            if count == 0 {
                break;
            }
            output
                .write_all(&buffer[..count])
                .map_err(|source| CoreError::Io {
                    operation: "write runtime asset staging file",
                    source,
                })?;
            digest.update(&buffer[..count]);
            size = size
                .checked_add(count as i64)
                .ok_or_else(|| CoreError::Validation("asset is too large".into()))?;
        }
        output.sync_all().map_err(|source| CoreError::Io {
            operation: "sync runtime asset staging file",
            source,
        })?;
        let content_hash = format!("sha256:{:x}", digest.finalize());
        if expected_hash.is_some_and(|expected| expected != content_hash) {
            return Err(CoreError::Validation(
                "asset bytes do not match the declared content hash".into(),
            ));
        }
        let destination = runtime_asset_path(root, &content_hash)?;
        if destination.is_file() {
            let (existing_hash, existing_size) = streamed_file_digest(&destination)?;
            if existing_hash != content_hash || existing_size != size {
                return Err(CoreError::Conflict(
                    "runtime asset store contains corrupted bytes".into(),
                ));
            }
            std::fs::remove_file(&temporary).map_err(|source| CoreError::Io {
                operation: "remove duplicate runtime asset staging file",
                source,
            })?;
        } else {
            std::fs::rename(&temporary, &destination).map_err(|source| CoreError::Io {
                operation: "install runtime asset",
                source,
            })?;
        }
        Ok((content_hash, size))
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(&temporary);
    }
    result
}

fn store_runtime_asset_file(
    root: &Path,
    source: &Path,
    expected_hash: Option<&str>,
) -> Result<(String, i64), CoreError> {
    let input = std::fs::File::open(source).map_err(|source| CoreError::Io {
        operation: "open runtime asset input",
        source,
    })?;
    store_runtime_asset(root, input, expected_hash)
}

fn ensure_runtime_asset(
    root: &Path,
    portable_path: &str,
    content_hash: &str,
    size: i64,
) -> Result<(), CoreError> {
    let runtime_path = runtime_asset_path(root, content_hash)?;
    let (actual_hash, actual_size) = if runtime_path.is_file() {
        streamed_file_digest(&runtime_path)?
    } else {
        let source = crate::storage::normalized_project_path(root, portable_path)?;
        store_runtime_asset_file(root, &source, Some(content_hash))?
    };
    if actual_hash != content_hash || actual_size != size {
        return Err(CoreError::Validation(format!(
            "asset bytes do not match metadata for {portable_path}"
        )));
    }
    Ok(())
}

const RUNTIME_STORAGE_ROLE: &str = "daena.runtime";
const RUNTIME_SCHEMA_VERSION: i64 = 5;
const EXPORTER_CONTRACT_VERSION: &str = "2";
const BACKGROUND_EXPORT_IDLE_DELAY: Duration = Duration::from_secs(2);
const BACKGROUND_EXPORT_MAX_DELAY: Duration = Duration::from_secs(30);

fn reset_required_error() -> CoreError {
    CoreError::ResetRequired(
        "unsupported Daena runtime storage; close Daena and remove .daena/ before reopening this project".into(),
    )
}

pub struct ProjectStore {
    connection: Connection,
    database_epoch: String,
    root: Option<PathBuf>,
    suppress_sync: Cell<bool>,
    _session_lock: Option<crate::sync::ProjectSessionLock>,
    export_worker: Option<ExportWorker>,
}

pub struct CheckpointHandle {
    root: PathBuf,
    database: PathBuf,
    export_sender: Option<mpsc::Sender<ExportWorkerCommand>>,
}

enum ExportWorkerCommand {
    Wake,
    Flush(String, mpsc::Sender<Result<Generation, CoreError>>),
    StopWithoutDrain(mpsc::Sender<Result<(), CoreError>>),
    Stop(mpsc::Sender<Result<(), CoreError>>),
}

struct ExportWorker {
    sender: mpsc::Sender<ExportWorkerCommand>,
    join: Option<JoinHandle<()>>,
}

impl ExportWorker {
    fn start(root: &Path, database: &Path) -> Result<Self, CoreError> {
        let root = root.to_path_buf();
        let database = database.to_path_buf();
        let (sender, receiver) = mpsc::channel();
        let join = thread::Builder::new()
            .name("daena-export-worker".into())
            .spawn(move || {
                while let Ok(command) = receiver.recv() {
                    let export = |reason: &str, force: bool| {
                        let worker_store = ProjectStore::open_database(
                            &database,
                            Some(root.clone()),
                            None,
                            false,
                            false,
                        )?;
                        if force {
                            worker_store.flush_checkpoint(reason)
                        } else {
                            worker_store.flush_checkpoint_if_dirty(reason)
                        }
                    };
                    match command {
                        ExportWorkerCommand::Wake => {
                            let started_at = Instant::now();
                            let max_deadline = started_at + BACKGROUND_EXPORT_MAX_DELAY;
                            let mut idle_deadline = started_at + BACKGROUND_EXPORT_IDLE_DELAY;
                            loop {
                                let deadline = idle_deadline.min(max_deadline);
                                let remaining = deadline.saturating_duration_since(Instant::now());
                                match receiver.recv_timeout(remaining) {
                                    Ok(ExportWorkerCommand::Wake) => {
                                        idle_deadline =
                                            (Instant::now() + BACKGROUND_EXPORT_IDLE_DELAY)
                                                .min(max_deadline);
                                    }
                                    Ok(ExportWorkerCommand::Flush(reason, reply)) => {
                                        let _ = reply.send(export(&reason, true));
                                        break;
                                    }
                                    Ok(ExportWorkerCommand::Stop(reply)) => {
                                        let _ = reply.send(
                                            export("project close checkpoint", false).map(|_| ()),
                                        );
                                        return;
                                    }
                                    Ok(ExportWorkerCommand::StopWithoutDrain(reply)) => {
                                        let _ = reply.send(Ok(()));
                                        return;
                                    }
                                    Err(mpsc::RecvTimeoutError::Timeout) => {
                                        let _ = export("background checkpoint export", false);
                                        break;
                                    }
                                    Err(mpsc::RecvTimeoutError::Disconnected) => return,
                                }
                            }
                        }
                        ExportWorkerCommand::Flush(reason, reply) => {
                            let _ = reply.send(export(&reason, true));
                        }
                        ExportWorkerCommand::Stop(reply) => {
                            let _ =
                                reply.send(export("project close checkpoint", false).map(|_| ()));
                            break;
                        }
                        ExportWorkerCommand::StopWithoutDrain(reply) => {
                            let _ = reply.send(Ok(()));
                            break;
                        }
                    }
                }
            })
            .map_err(|source| CoreError::Io {
                operation: "start export worker",
                source,
            })?;
        Ok(Self {
            sender,
            join: Some(join),
        })
    }

    fn wake(&self) {
        let _ = self.sender.send(ExportWorkerCommand::Wake);
    }

    fn flush(&self, reason: String) -> Result<Generation, CoreError> {
        let (sender, receiver) = mpsc::channel();
        self.sender
            .send(ExportWorkerCommand::Flush(reason, sender))
            .map_err(|_| CoreError::Conflict("export worker is not running".into()))?;
        receiver
            .recv()
            .map_err(|_| CoreError::Conflict("export worker stopped before flush".into()))?
    }

    fn stop(mut self) -> Result<(), CoreError> {
        let (sender, receiver) = mpsc::channel();
        let send_result = self.sender.send(ExportWorkerCommand::Stop(sender));
        let result = if send_result.is_ok() {
            receiver
                .recv()
                .map_err(|_| CoreError::Conflict("export worker stopped before drain".into()))?
        } else {
            Ok(())
        };
        if let Some(join) = self.join.take() {
            join.join()
                .map_err(|_| CoreError::Conflict("export worker panicked".into()))?;
        }
        result
    }

    fn stop_without_drain(mut self) -> Result<(), CoreError> {
        let (sender, receiver) = mpsc::channel();
        let send_result = self
            .sender
            .send(ExportWorkerCommand::StopWithoutDrain(sender));
        let result = if send_result.is_ok() {
            receiver
                .recv()
                .map_err(|_| CoreError::Conflict("export worker stopped before pause".into()))?
        } else {
            Ok(())
        };
        if let Some(join) = self.join.take() {
            join.join()
                .map_err(|_| CoreError::Conflict("export worker panicked".into()))?;
        }
        result
    }
}

impl CheckpointHandle {
    pub fn flush_checkpoint(&self, reason: impl Into<String>) -> Result<Generation, CoreError> {
        let reason = reason.into();
        if let Some(sender) = &self.export_sender {
            return flush_export_worker(sender, reason);
        }
        let store = ProjectStore::open_checkpoint_writer(&self.database, self.root.clone())?;
        store.flush_checkpoint(reason)
    }
}

fn flush_export_worker(
    sender: &mpsc::Sender<ExportWorkerCommand>,
    reason: String,
) -> Result<Generation, CoreError> {
    let (reply_sender, reply_receiver) = mpsc::channel();
    sender
        .send(ExportWorkerCommand::Flush(reason, reply_sender))
        .map_err(|_| CoreError::Conflict("export worker is not running".into()))?;
    reply_receiver
        .recv()
        .map_err(|_| CoreError::Conflict("export worker stopped before checkpoint flush".into()))?
}

impl Drop for ProjectStore {
    fn drop(&mut self) {
        if let Some(worker) = self.export_worker.take() {
            let _ = worker.stop();
        }
    }
}

impl ProjectStore {
    /// Open an independent read-only connection for project reads.
    /// reads. It never starts an exporter, acquires the project writer lock,
    /// or initializes/mutates schema state.
    pub fn open_read_only(root: impl AsRef<Path>) -> Result<Self, CoreError> {
        let root = root.as_ref().to_path_buf();
        let path = project_database_path(&root);
        let connection = Connection::open_with_flags(
            &path,
            OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_URI,
        )?;
        connection.busy_timeout(Duration::from_secs(2))?;
        connection.pragma_update(None, "foreign_keys", true)?;
        Self::validate_runtime_metadata(&connection, Some(&root))?;
        let database_epoch = connection.query_row(
            "SELECT database_epoch FROM runtime_meta WHERE key='runtime'",
            [],
            |row| row.get(0),
        )?;
        Ok(Self {
            connection,
            database_epoch,
            root: Some(root),
            suppress_sync: Cell::new(true),
            _session_lock: None,
            export_worker: None,
        })
    }

    pub fn checkpoint_handle(&self) -> Result<CheckpointHandle, CoreError> {
        let root = self.root.clone().ok_or_else(|| {
            CoreError::Validation("checkpoint requires a directory-backed project".into())
        })?;
        Ok(CheckpointHandle {
            database: project_database_path(&root),
            root,
            export_sender: self
                .export_worker
                .as_ref()
                .map(|worker| worker.sender.clone()),
        })
    }

    fn open_checkpoint_writer(
        database: impl AsRef<Path>,
        root: PathBuf,
    ) -> Result<Self, CoreError> {
        Self::open_database(database, Some(root), None, false, false)
    }

    pub fn open(path: impl AsRef<Path>) -> Result<Self, CoreError> {
        let path = path.as_ref();
        if path.is_dir() {
            return Self::open_directory(path);
        }
        Err(CoreError::Validation(
            "project storage must be opened from a directory".into(),
        ))
    }

    pub fn open_directory(path: impl AsRef<Path>) -> Result<Self, CoreError> {
        let root = path.as_ref();
        std::fs::create_dir_all(root).map_err(|error| CoreError::NotFound(error.to_string()))?;
        if root.join("daena.sqlite").exists() {
            return Err(CoreError::Validation(
                "legacy daena.sqlite projects are not supported by format version 2".into(),
            ));
        }
        std::fs::create_dir_all(root.join(".daena"))
            .map_err(|error| CoreError::NotFound(error.to_string()))?;
        let session_lock = crate::sync::ProjectSessionLock::acquire(root)?;
        let repository = crate::storage::FilesystemRepository::open(root)?;
        for directory in [
            "entities",
            "plugins",
            ".daena",
            ".daena/checkpoints",
            ".daena/assets",
            ".daena/backups",
            ".daena/conflicts",
            ".daena/local",
        ] {
            std::fs::create_dir_all(root.join(directory))
                .map_err(|error| CoreError::NotFound(error.to_string()))?;
        }
        std::fs::create_dir_all(root.join("assets/images"))
            .map_err(|error| CoreError::NotFound(error.to_string()))?;
        std::fs::create_dir_all(root.join("assets/videos"))
            .map_err(|error| CoreError::NotFound(error.to_string()))?;
        std::fs::create_dir_all(root.join("assets/maps"))
            .map_err(|error| CoreError::NotFound(error.to_string()))?;
        std::fs::create_dir_all(root.join("assets/files"))
            .map_err(|error| CoreError::NotFound(error.to_string()))?;
        let metadata_path = root.join("project.json");
        if !metadata_path.exists() {
            let name = root
                .file_name()
                .and_then(|value| value.to_str())
                .unwrap_or("Daena Archive project");
            let metadata = crate::storage::ProjectManifest::new(name);
            metadata.validate(&metadata_path)?;
            crate::storage::write_json(&metadata_path, &metadata)?;
        } else {
            let metadata =
                crate::storage::read_json::<crate::storage::ProjectManifest>(&metadata_path)?;
            metadata.validate(&metadata_path)?;
        }
        let gitignore = root.join(".gitignore");
        let required_gitignore = [".daena/", "checkpoint.json"];
        let existing_gitignore = if gitignore.exists() {
            std::fs::read_to_string(&gitignore)
                .map_err(|error| CoreError::NotFound(error.to_string()))?
        } else {
            String::new()
        };
        let mut gitignore_content = existing_gitignore.clone();
        for pattern in required_gitignore {
            if !existing_gitignore
                .lines()
                .any(|line| line.trim() == pattern)
            {
                if !gitignore_content.is_empty() && !gitignore_content.ends_with('\n') {
                    gitignore_content.push('\n');
                }
                gitignore_content.push_str(pattern);
                gitignore_content.push('\n');
            }
        }
        if gitignore_content != existing_gitignore {
            std::fs::write(&gitignore, gitignore_content)
                .map_err(|error| CoreError::NotFound(error.to_string()))?;
        }
        let index_path = project_database_path(root);
        if index_path.is_file() {
            return Self::open_database(
                &index_path,
                Some(root.to_path_buf()),
                Some(session_lock),
                false,
                true,
            );
        }
        let canonical = repository.scan()?;
        Self::rebuild_directory_index(root, &canonical, session_lock)
    }

    fn rebuild_directory_index(
        root: &Path,
        canonical: &crate::storage::CanonicalProject,
        session_lock: crate::sync::ProjectSessionLock,
    ) -> Result<Self, CoreError> {
        let index_path = project_database_path(root);
        let next_path = root.join(".daena/index.sqlite.next");
        for suffix in ["", "-wal", "-shm", "-journal"] {
            let path = PathBuf::from(format!("{}{}", next_path.display(), suffix));
            if path.exists() {
                std::fs::remove_file(&path).map_err(|error| CoreError::Io {
                    operation: "remove stale index rebuild",
                    source: error,
                })?;
            }
        }

        let store = Self::open_database(&next_path, Some(root.to_path_buf()), None, false, false)?;
        let payload = serde_json::to_string(&canonical.snapshot)
            .map_err(|error| CoreError::Serialization(error.to_string()))?;
        store.import_json_with_mode_and_sync_with_request_and_search(
            &payload, true, false, None, false,
        )?;
        store.rebuild_search()?;
        store.verify_index()?;
        store.connection.execute(
            "UPDATE runtime_meta SET content_generation=0, exported_generation=0, checkpoint_digest=NULL, export_error=NULL WHERE key='runtime'",
            [],
        )?;
        store.install_checkpoint_manifest(root, 0)?;
        store.connection.execute_batch(
            "PRAGMA wal_checkpoint(TRUNCATE);
             PRAGMA journal_mode=DELETE;",
        )?;
        drop(store);

        // Both paths are in .daena, so rename is atomic on the supported
        // local filesystems.  The old index is never opened or modified while
        // the new database is being built.
        std::fs::rename(&next_path, &index_path).map_err(|error| CoreError::Io {
            operation: "replace runtime index",
            source: error,
        })?;
        for suffix in ["-wal", "-shm", "-journal"] {
            let path = PathBuf::from(format!("{}{}", index_path.display(), suffix));
            let _ = std::fs::remove_file(path);
        }
        Self::open_database(
            &index_path,
            Some(root.to_path_buf()),
            Some(session_lock),
            false,
            true,
        )
    }

    fn open_database(
        path: impl AsRef<Path>,
        root: Option<PathBuf>,
        session_lock: Option<crate::sync::ProjectSessionLock>,
        acquire_session_lock: bool,
        start_worker: bool,
    ) -> Result<Self, CoreError> {
        let path = path.as_ref();
        let existing_database = path != Path::new(":memory:") && path.is_file();
        let connection = Connection::open(path)?;
        connection.busy_timeout(Duration::from_secs(2))?;
        connection.pragma_update(None, "foreign_keys", true)?;
        connection.pragma_update(None, "synchronous", "NORMAL")?;
        if existing_database {
            Self::validate_runtime_metadata(&connection, root.as_deref())?;
        }
        let session_lock = match session_lock {
            Some(lock) => Some(lock),
            None if acquire_session_lock => root
                .as_deref()
                .map(crate::sync::ProjectSessionLock::acquire)
                .transpose()?,
            None => None,
        };
        let mut store = Self {
            connection,
            database_epoch: String::new(),
            root,
            suppress_sync: Cell::new(false),
            _session_lock: session_lock,
            export_worker: None,
        };
        if !existing_database {
            store.initialize(true)?;
        } else if start_worker {
            store.ensure_query_indexes()?;
            store.ensure_search_projection()?;
        }
        store.database_epoch = store.connection.query_row(
            "SELECT database_epoch FROM runtime_meta WHERE key='runtime'",
            [],
            |row| row.get(0),
        )?;
        if start_worker {
            if let Some(root) = store.root.as_deref() {
                let worker = ExportWorker::start(root, path)?;
                let needs_export: bool = store.connection.query_row(
                    "SELECT content_generation > exported_generation OR export_error IS NOT NULL FROM runtime_meta WHERE key='runtime'",
                    [],
                    |row| row.get(0),
                )?;
                if needs_export {
                    worker.wake();
                }
                store.export_worker = Some(worker);
            }
        }
        Ok(store)
    }

    /// Export one complete checkpoint for the latest committed SQLite state.
    ///
    /// The exporter deliberately renders the whole portable tree. SQLite
    /// generations coalesce bursts of mutations; no durable per-record work
    /// queue is needed to recover or retry an export.
    fn export_latest_generation(&self) -> Result<(), CoreError> {
        let Some(root) = self.root.as_deref() else {
            return Ok(());
        };
        self.connection.execute_batch("BEGIN")?;
        let export_result = (|| {
            let target_generation: Generation = self.connection.query_row(
                "SELECT content_generation FROM runtime_meta WHERE key='runtime'",
                [],
                |row| row.get(0),
            )?;
            let snapshot = self.export_snapshot()?;
            Ok::<_, CoreError>((target_generation, snapshot))
        })();
        match export_result {
            Ok((target_generation, snapshot)) => {
                self.connection.execute_batch("COMMIT")?;
                self.export_complete_snapshot(root, &snapshot, target_generation)?;
            }
            Err(error) => {
                let _ = self.connection.execute_batch("ROLLBACK");
                return Err(error);
            }
        }
        Ok(())
    }

    /// Wake the project-scoped exporter after durable work has been queued.
    ///
    /// The committed generation is durable in SQLite, so losing this
    /// notification only delays export until the next flush or reopen; it
    /// cannot lose the mutation.
    pub fn wake_export_worker(&self) {
        if let Some(worker) = &self.export_worker {
            worker.wake();
        }
    }

    /// Flush the latest runtime generation to a complete portable checkpoint.
    /// The returned generation is the generation whose checkpoint was
    /// confirmed installed.
    pub fn flush_checkpoint(&self, reason: impl Into<String>) -> Result<Generation, CoreError> {
        let reason = reason.into();
        let target: Generation = self.connection.query_row(
            "SELECT content_generation FROM runtime_meta WHERE key='runtime'",
            [],
            |row| row.get(0),
        )?;
        let result = (|| {
            self.flush_export(reason)?;
            if let Some(root) = self.root.as_deref() {
                let exported: Generation = self.connection.query_row(
                    "SELECT exported_generation FROM runtime_meta WHERE key='runtime'",
                    [],
                    |row| row.get(0),
                )?;
                if exported < target {
                    self.install_checkpoint_manifest(root, target)?;
                }
                let exported: Generation = self.connection.query_row(
                    "SELECT exported_generation FROM runtime_meta WHERE key='runtime'",
                    [],
                    |row| row.get(0),
                )?;
                if exported < target {
                    return Err(CoreError::Conflict(format!(
                        "checkpoint barrier completed below requested generation {target}"
                    )));
                }
            }
            Ok(target)
        })();
        match result {
            Ok(generation) => {
                self.connection.execute(
                    "UPDATE runtime_meta SET export_error=NULL WHERE key='runtime'",
                    [],
                )?;
                Ok(generation)
            }
            Err(error) => {
                let message = error.to_string();
                self.connection.execute(
                    "UPDATE runtime_meta SET export_error=?1 WHERE key='runtime'",
                    params![message],
                )?;
                Err(error)
            }
        }
    }

    fn flush_checkpoint_if_dirty(
        &self,
        reason: impl Into<String>,
    ) -> Result<Generation, CoreError> {
        let (content, exported, failed): (Generation, Generation, bool) = self
            .connection
            .query_row(
                "SELECT content_generation,exported_generation,export_error IS NOT NULL FROM runtime_meta WHERE key='runtime'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )?;
        if exported >= content && !failed {
            return Ok(content);
        }
        self.flush_checkpoint(reason)
    }

    fn flush_export(&self, reason: String) -> Result<(), CoreError> {
        if reason.trim().is_empty() {
            return Err(CoreError::Validation(
                "checkpoint barrier operation reason cannot be empty".into(),
            ));
        }
        if let Some(worker) = &self.export_worker {
            worker.flush(reason).map(|_| ())
        } else {
            self.export_latest_generation()
        }
    }

    fn validate_runtime_metadata(
        connection: &Connection,
        root: Option<&Path>,
    ) -> Result<(), CoreError> {
        let runtime_table: Option<String> = connection
            .query_row(
                "SELECT name FROM sqlite_master WHERE type='table' AND name='runtime_meta'",
                [],
                |row| row.get(0),
            )
            .optional()?;
        if runtime_table.is_none() {
            return Err(reset_required_error());
        }
        let metadata = connection
            .query_row(
                "SELECT storage_role, schema_version, project_id, portable_format_version, exporter_version, content_generation, exported_generation FROM runtime_meta WHERE key='runtime'",
                [],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, i64>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, i64>(3)?,
                        row.get::<_, String>(4)?,
                        row.get::<_, i64>(5)?,
                        row.get::<_, i64>(6)?,
                    ))
                },
            )
            .map_err(|error| match error {
                rusqlite::Error::QueryReturnedNoRows => reset_required_error(),
                other => CoreError::CorruptStorage(format!("runtime metadata is corrupt: {other}")),
            })?;
        let expected_project_id = root
            .map(|root| {
                crate::storage::read_json::<crate::storage::ProjectManifest>(
                    &root.join("project.json"),
                )
                .map(|manifest| manifest.id)
            })
            .transpose()?;
        let project_matches = expected_project_id
            .as_deref()
            .is_none_or(|project_id| project_id == metadata.2);
        if metadata.0 != RUNTIME_STORAGE_ROLE
            || metadata.1 != RUNTIME_SCHEMA_VERSION
            || metadata.3 != crate::storage::PROJECT_FORMAT_VERSION as i64
            || metadata.4 != EXPORTER_CONTRACT_VERSION
            || metadata.5 < 0
            || metadata.6 < 0
            || metadata.6 > metadata.5
            || !project_matches
        {
            return Err(reset_required_error());
        }
        Ok(())
    }

    fn verify_index(&self) -> Result<(), CoreError> {
        let foreign_key_error: Option<String> = self
            .connection
            .query_row("PRAGMA foreign_key_check", [], |row| row.get(0))
            .optional()?;
        if let Some(error) = foreign_key_error {
            return Err(CoreError::Validation(format!(
                "index foreign-key check failed: {error}"
            )));
        }
        let integrity: String = self
            .connection
            .query_row("PRAGMA integrity_check", [], |row| row.get(0))?;
        if integrity != "ok" {
            return Err(CoreError::Validation(format!(
                "index integrity check failed: {integrity}"
            )));
        }
        Ok(())
    }

    fn checkpoint_sources(root: &Path) -> Result<Vec<crate::storage::CanonicalSource>, CoreError> {
        let path = root.join(crate::storage::CHECKPOINT_MANIFEST_FILE);
        if !path.is_file() {
            return Ok(Vec::new());
        }
        let checkpoint: crate::storage::CheckpointManifest = crate::storage::read_json(&path)?;
        checkpoint.validate(&path)?;
        Ok(checkpoint
            .files
            .into_iter()
            .map(|file| crate::storage::CanonicalSource {
                path: file.path,
                content_hash: file.sha256,
                format_version: crate::storage::PROJECT_FORMAT_VERSION,
            })
            .collect())
    }

    pub fn import_checkpoint(&mut self) -> Result<ExternalChangeReport, CoreError> {
        let root = self.project_root()?.to_path_buf();
        let checkpoint_path = root.join(crate::storage::CHECKPOINT_MANIFEST_FILE);
        let checkpoint: crate::storage::CheckpointManifest =
            crate::storage::read_json(&checkpoint_path)?;
        crate::storage::validate_checkpoint(&root, &checkpoint)?;
        let (content_generation, exported_generation): (Generation, Generation) = self
            .connection
            .query_row(
            "SELECT content_generation,exported_generation FROM runtime_meta WHERE key='runtime'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )?;
        if content_generation != exported_generation {
            return Err(CoreError::Conflict(
                "runtime has unexported changes; flush the checkpoint before importing portable files"
                    .into(),
            ));
        }
        let archive = self.recovery_backup_to(root.join(".daena/backups"))?;
        let canonical = crate::storage::FilesystemRepository::open(&root)?.scan()?;
        let payload = serde_json::to_string(&canonical.snapshot)
            .map_err(|error| CoreError::Serialization(error.to_string()))?;
        let next_path = root.join(".daena/index.sqlite.next");
        for suffix in ["", "-wal", "-shm", "-journal"] {
            let path = PathBuf::from(format!("{}{}", next_path.display(), suffix));
            if path.exists() {
                std::fs::remove_file(path).map_err(|error| CoreError::Io {
                    operation: "remove stale checkpoint candidate",
                    source: error,
                })?;
            }
        }
        let candidate = Self::open_database(&next_path, Some(root.clone()), None, false, false)?;
        candidate.import_json_with_mode_and_sync_with_request_and_search(
            &payload, true, false, None, false,
        )?;
        let digest =
            crate::storage::canonical_json_bytes(&checkpoint).map(|bytes| digest_bytes(&bytes))?;
        candidate.rebuild_search()?;
        candidate.rebuild_maps_projection()?;
        candidate.connection.execute(
            "UPDATE runtime_meta SET database_epoch=?1,content_generation=?2,exported_generation=?2,checkpoint_digest=?3,export_error=NULL WHERE key='runtime'",
            params![Uuid::new_v4().to_string(), checkpoint.content_generation, digest],
        )?;
        candidate
            .connection
            .execute_batch("PRAGMA wal_checkpoint(TRUNCATE); PRAGMA journal_mode=DELETE;")?;
        drop(candidate);
        if let Some(worker) = self.export_worker.take() {
            worker.stop_without_drain()?;
        }
        let old_connection = std::mem::replace(&mut self.connection, Connection::open_in_memory()?);
        drop(old_connection);
        let index_path = project_database_path(&root);
        std::fs::rename(&next_path, &index_path).map_err(|error| CoreError::Io {
            operation: "install checkpoint candidate database",
            source: error,
        })?;
        for suffix in ["-wal", "-shm", "-journal"] {
            let path = PathBuf::from(format!("{}{}", index_path.display(), suffix));
            let _ = std::fs::remove_file(path);
        }
        self.connection = Connection::open(&index_path)?;
        self.connection.busy_timeout(Duration::from_secs(2))?;
        self.connection.pragma_update(None, "foreign_keys", true)?;
        self.database_epoch = self.connection.query_row(
            "SELECT database_epoch FROM runtime_meta WHERE key='runtime'",
            [],
            |row| row.get(0),
        )?;
        self.restart_export_worker()?;
        Ok(ExternalChangeReport {
            changed: true,
            paths: checkpoint
                .files
                .iter()
                .map(|file| file.path.clone())
                .collect(),
            diagnostics: vec![format!("runtime archive preserved at {archive}")],
        })
    }

    pub fn save_recovery_copy(&self, entity_id: &str, body: &str) -> Result<String, CoreError> {
        let Some(root) = self.root.as_deref() else {
            return Err(CoreError::Validation(
                "recovery copies require a directory-backed project".into(),
            ));
        };
        uuid::Uuid::parse_str(entity_id)
            .map_err(|error| CoreError::Validation(format!("invalid entity ID: {error}")))?;
        let conflicts = root.join(".daena/conflicts");
        std::fs::create_dir_all(&conflicts).map_err(|error| CoreError::Io {
            operation: "create recovery copy directory",
            source: error,
        })?;
        let path = conflicts.join(format!(
            "{}-{}-{}.md",
            chrono_like_now(),
            entity_id,
            Uuid::new_v4()
        ));
        let bytes = crate::storage::canonical_markdown(body).into_bytes();
        std::fs::write(&path, bytes).map_err(|error| CoreError::Io {
            operation: "write recovery copy",
            source: error,
        })?;
        Ok(path
            .strip_prefix(root)
            .unwrap_or(&path)
            .to_string_lossy()
            .replace('\\', "/"))
    }

    fn require_map_entity(&self, entity_id: &str) -> Result<(), CoreError> {
        uuid::Uuid::parse_str(entity_id)
            .map_err(|error| CoreError::Validation(format!("invalid entity ID: {error}")))?;
        let entity_type: Option<String> = self.connection.query_row(
            "SELECT entity_type FROM entities WHERE id=?1 AND deleted=0",
            [entity_id],
            |row| row.get(0),
        )?;
        if entity_type.as_deref() != Some(crate::maps::MAP_ENTITY_TYPE) {
            return Err(CoreError::NotFound("map entity not found".into()));
        }
        Ok(())
    }

    fn map_recovery_dir(&self, create: bool) -> Result<std::path::PathBuf, CoreError> {
        let Some(root) = self.root.as_deref() else {
            return Err(CoreError::Validation(
                "map recovery copies require a directory-backed project".into(),
            ));
        };
        let conflicts = root.join(".daena/conflicts/maps");
        if create {
            std::fs::create_dir_all(&conflicts).map_err(|error| CoreError::Io {
                operation: "create map recovery copy directory",
                source: error,
            })?;
        }
        Ok(conflicts)
    }

    /// Writes a rejected map save draft to `.daena/conflicts/maps/`. The
    /// original source asset is never overwritten without review; the draft
    /// lives only in the disposable derived state directory.
    pub fn save_map_recovery_copy(
        &self,
        entity_id: &str,
        bytes: &[u8],
    ) -> Result<String, CoreError> {
        self.require_map_entity(entity_id)?;
        if bytes.len() > crate::maps::MAP_RECOVERY_MAX_BYTES {
            return Err(CoreError::Validation(
                "map recovery copy exceeds the host transfer limit".into(),
            ));
        }
        let conflicts = self.map_recovery_dir(true)?;
        let file_name = format!(
            "{}-{}-{}.map",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis(),
            entity_id,
            Uuid::new_v4()
        );
        std::fs::write(conflicts.join(&file_name), bytes).map_err(|error| CoreError::Io {
            operation: "write map recovery copy",
            source: error,
        })?;
        Ok(format!(".daena/conflicts/maps/{file_name}"))
    }

    /// Lists the recovery drafts recorded for a map, newest first. The
    /// returned `path` is root-relative so it can be surfaced to the shell
    /// without exposing filesystem layout.
    pub fn list_map_recovery_copies(
        &self,
        entity_id: &str,
    ) -> Result<Vec<crate::maps::MapRecoveryCopy>, CoreError> {
        self.require_map_entity(entity_id)?;
        let Ok(conflicts) = self.map_recovery_dir(false) else {
            return Ok(Vec::new());
        };
        let Ok(entries) = std::fs::read_dir(&conflicts) else {
            return Ok(Vec::new());
        };
        let mut copies = Vec::new();
        for entry in entries.flatten() {
            let Some(file_name) = entry.file_name().to_str().map(str::to_owned) else {
                continue;
            };
            let Some(created_at) = parse_map_recovery_file_name(entity_id, &file_name) else {
                continue;
            };
            copies.push(crate::maps::MapRecoveryCopy {
                file_name: file_name.clone(),
                path: format!(".daena/conflicts/maps/{file_name}"),
                created_at,
            });
        }
        copies.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        Ok(copies)
    }

    /// Replaces a map source asset with a previously exported recovery draft.
    /// This is an explicit user action, so the revision preflight runs against
    /// the current revision instead of a stale one.
    pub fn restore_map_recovery_copy(
        &self,
        entity_id: &str,
        file_name: &str,
        request_id: Option<&str>,
    ) -> Result<Asset, CoreError> {
        self.require_map_entity(entity_id)?;
        if file_name.trim().is_empty() || file_name.contains('/') || file_name.contains('\\') {
            return Err(CoreError::Validation(
                "map recovery copy file name is invalid".into(),
            ));
        }
        if parse_map_recovery_file_name(entity_id, file_name).is_none() {
            return Err(CoreError::Validation(
                "map recovery copy file name is invalid".into(),
            ));
        }
        let conflicts = self.map_recovery_dir(false)?;
        let bytes = std::fs::read(conflicts.join(file_name)).map_err(|error| CoreError::Io {
            operation: "read map recovery copy",
            source: error,
        })?;
        if bytes.len() > crate::maps::MAP_RECOVERY_MAX_BYTES {
            return Err(CoreError::Validation(
                "map recovery copy exceeds the host transfer limit".into(),
            ));
        }
        let descriptor: crate::maps::MapDescriptor = self
            .list_fields(entity_id.into())?
            .into_iter()
            .find(|field| field.namespace == crate::maps::MAP_NAMESPACE && field.key == "map")
            .map(|field| {
                serde_json::from_value(field.value)
                    .map_err(|error| CoreError::Serialization(error.to_string()))
            })
            .transpose()?
            .ok_or_else(|| CoreError::NotFound("map descriptor not found".into()))?;
        let Some(source_asset_id) = descriptor.source_asset_id else {
            return Err(CoreError::NotFound(
                "map has no saved source asset to restore".into(),
            ));
        };
        let mut source = self.asset_unchecked(&source_asset_id)?;
        let expected_revision = self.revision_for_asset(&source.id)?;
        let digest = format!("sha256:{}", digest_bytes(&bytes));
        self.replace_asset_bytes_with_request(
            AssetReplaceInput {
                asset_id: source_asset_id,
                content_hash: digest,
                size: bytes.len() as i64,
                mime_type: std::mem::take(&mut source.mime_type),
            },
            bytes,
            &expected_revision,
            request_id,
        )
    }

    fn revision_for_entity(&self, entity_id: &str) -> Result<String, CoreError> {
        let entity = self.connection.query_row(
            "SELECT id,name,entity_type,deleted,created_at,updated_at FROM entities WHERE id=?1",
            params![entity_id],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, Option<String>>(2)?,
                    row.get::<_, i64>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, String>(5)?,
                ))
            },
        )?;
        let documents = self
            .connection
            .prepare(
                "SELECT id,format,body,updated_at FROM documents WHERE entity_id=?1 ORDER BY id",
            )?
            .query_map(params![entity_id], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                ))
            })?
            .collect::<Result<Vec<_>, _>>()?;
        let fields = self
            .connection
            .prepare("SELECT namespace,key,value FROM entity_fields WHERE entity_id=?1 ORDER BY namespace,key")?
            .query_map(params![entity_id], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                ))
            })?
            .collect::<Result<Vec<_>, _>>()?;
        let relationships = self
            .connection
            .prepare("SELECT id,source_id,target_id,relationship_type,metadata FROM relationships WHERE source_id=?1 OR target_id=?1 ORDER BY id")?
            .query_map(params![entity_id], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                ))
            })?
            .collect::<Result<Vec<_>, _>>()?;
        let assets = self
            .connection
            .prepare("SELECT id,namespace,filename,content_hash,size,mime_type,path,created_at FROM assets WHERE entity_id=?1 ORDER BY id")?
            .query_map(params![entity_id], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, i64>(4)?,
                    row.get::<_, String>(5)?,
                    row.get::<_, String>(6)?,
                    row.get::<_, String>(7)?,
                ))
            })?
            .collect::<Result<Vec<_>, _>>()?;
        let revision = self.revision_digest(&(entity, documents, fields, relationships, assets))?;
        Ok(revision)
    }

    fn populate_entity_revisions(&self, entities: &mut [Entity]) -> Result<(), CoreError> {
        if entities.is_empty() {
            return Ok(());
        }

        let ids = entities
            .iter()
            .map(|entity| entity.id.clone())
            .collect::<Vec<_>>();
        // Search returns at most 100 entities. Restrict those supporting reads;
        // full entity lists are cheaper as four sequential table scans than as
        // hundreds of indexed point queries.
        let restricted = ids.len() <= 100;
        let placeholders = std::iter::repeat_n("?", ids.len())
            .collect::<Vec<_>>()
            .join(",");

        let mut documents: BTreeMap<String, Vec<(String, String, String, String)>> =
            BTreeMap::new();
        let sql = if restricted {
            format!("SELECT entity_id,id,format,body,updated_at FROM documents WHERE entity_id IN ({placeholders}) ORDER BY entity_id,id")
        } else {
            "SELECT entity_id,id,format,body,updated_at FROM documents ORDER BY entity_id,id".into()
        };
        let mut statement = self.connection.prepare(&sql)?;
        let rows = statement.query_map(rusqlite::params_from_iter(ids.iter().take(if restricted {
            ids.len()
        } else {
            0
        })), |row| {
            Ok((
                row.get::<_, String>(0)?,
                (
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                ),
            ))
        })?;
        for row in rows {
            let (entity_id, document) = row?;
            documents.entry(entity_id).or_default().push(document);
        }

        let mut fields: BTreeMap<String, Vec<(String, String, String)>> = BTreeMap::new();
        let sql = if restricted {
            format!("SELECT entity_id,namespace,key,value FROM entity_fields WHERE entity_id IN ({placeholders}) ORDER BY entity_id,namespace,key")
        } else {
            "SELECT entity_id,namespace,key,value FROM entity_fields ORDER BY entity_id,namespace,key".into()
        };
        let mut statement = self.connection.prepare(&sql)?;
        let rows = statement.query_map(rusqlite::params_from_iter(ids.iter().take(if restricted {
            ids.len()
        } else {
            0
        })), |row| {
            Ok((
                row.get::<_, String>(0)?,
                (
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                ),
            ))
        })?;
        for row in rows {
            let (entity_id, field) = row?;
            fields.entry(entity_id).or_default().push(field);
        }

        type RelationshipRevision = (String, String, String, String, String);
        let mut relationships: BTreeMap<String, Vec<RelationshipRevision>> = BTreeMap::new();
        let sql = if restricted {
            format!("SELECT id,source_id,target_id,relationship_type,metadata FROM relationships WHERE source_id IN ({placeholders}) OR target_id IN ({placeholders}) ORDER BY id")
        } else {
            "SELECT id,source_id,target_id,relationship_type,metadata FROM relationships ORDER BY id".into()
        };
        let relationship_params = ids.iter().chain(ids.iter()).take(if restricted {
            ids.len() * 2
        } else {
            0
        });
        let mut statement = self.connection.prepare(&sql)?;
        let rows = statement.query_map(rusqlite::params_from_iter(relationship_params), |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?,
            ))
        })?;
        for row in rows {
            let relationship = row?;
            relationships
                .entry(relationship.1.clone())
                .or_default()
                .push(relationship.clone());
            if relationship.2 != relationship.1 {
                relationships
                    .entry(relationship.2.clone())
                    .or_default()
                    .push(relationship);
            }
        }

        type AssetRevision = (String, String, String, String, i64, String, String, String);
        let mut assets: BTreeMap<String, Vec<AssetRevision>> = BTreeMap::new();
        let sql = if restricted {
            format!("SELECT entity_id,id,namespace,filename,content_hash,size,mime_type,path,created_at FROM assets WHERE entity_id IN ({placeholders}) ORDER BY entity_id,id")
        } else {
            "SELECT entity_id,id,namespace,filename,content_hash,size,mime_type,path,created_at FROM assets ORDER BY entity_id,id".into()
        };
        let mut statement = self.connection.prepare(&sql)?;
        let rows = statement.query_map(rusqlite::params_from_iter(ids.iter().take(if restricted {
            ids.len()
        } else {
            0
        })), |row| {
            Ok((
                row.get::<_, String>(0)?,
                (
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, i64>(5)?,
                    row.get::<_, String>(6)?,
                    row.get::<_, String>(7)?,
                    row.get::<_, String>(8)?,
                ),
            ))
        })?;
        for row in rows {
            let (entity_id, asset) = row?;
            assets.entry(entity_id).or_default().push(asset);
        }

        for entity in entities {
            let entity_value = (
                &entity.id,
                &entity.name,
                &entity.entity_type,
                i64::from(entity.deleted),
                &entity.created_at,
                &entity.updated_at,
            );
            entity.revision = self.revision_digest(&(
                entity_value,
                documents.remove(&entity.id).unwrap_or_default(),
                fields.remove(&entity.id).unwrap_or_default(),
                relationships.remove(&entity.id).unwrap_or_default(),
                assets.remove(&entity.id).unwrap_or_default(),
            ))?;
        }
        Ok(())
    }

    fn revision_for_document(&self, id: &str) -> Result<String, CoreError> {
        let value = self.connection.query_row(
            "SELECT id,entity_id,format,body FROM documents WHERE id=?1",
            params![id],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                ))
            },
        )?;
        self.revision_digest(&value)
    }

    fn revision_for_document_value(&self, document: &Document) -> Result<String, CoreError> {
        self.revision_digest(&(
            &document.id,
            &document.entity_id,
            &document.format,
            &document.body,
        ))
    }

    fn revision_for_field(&self, field: &FieldValue) -> Result<String, CoreError> {
        self.revision_digest(&(
            &field.entity_id,
            &field.namespace,
            &field.key,
            encode_field_value(&field.value)?,
        ))
    }

    fn revision_for_relationship(&self, id: &str) -> Result<String, CoreError> {
        let value = self.connection.query_row(
            "SELECT id,source_id,target_id,relationship_type,metadata FROM relationships WHERE id=?1",
            params![id],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                ))
            },
        )?;
        self.revision_digest(&value)
    }

    fn revision_for_relationship_value(
        &self,
        relationship: &Relationship,
    ) -> Result<String, CoreError> {
        self.revision_digest(&(
            &relationship.id,
            &relationship.source_id,
            &relationship.target_id,
            &relationship.relationship_type,
            &relationship.metadata,
        ))
    }

    fn revision_for_asset(&self, id: &str) -> Result<String, CoreError> {
        let value = self.connection.query_row(
            "SELECT id,entity_id,namespace,filename,content_hash,size,mime_type,path,created_at FROM assets WHERE id=?1",
            params![id],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, i64>(5)?,
                    row.get::<_, String>(6)?,
                    row.get::<_, String>(7)?,
                    row.get::<_, String>(8)?,
                ))
            },
        )?;
        self.revision_digest(&value)
    }

    fn revision_for_asset_value(&self, asset: &Asset) -> Result<String, CoreError> {
        self.revision_digest(&(
            &asset.id,
            &asset.entity_id,
            &asset.namespace,
            &asset.filename,
            &asset.content_hash,
            asset.size,
            &asset.mime_type,
            &asset.path,
            &asset.created_at,
        ))
    }

    fn revision_for_module_record(&self, id: &str) -> Result<String, CoreError> {
        let value = self.connection.query_row(
            "SELECT module_id,collection,id,owner_entity_id,value,created_at,updated_at FROM module_records WHERE id=?1",
            params![id],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, String>(5)?,
                    row.get::<_, String>(6)?,
                ))
            },
        )?;
        self.revision_digest(&value)
    }

    fn revision_for_module_record_value(
        &self,
        record: &ModuleRecord,
    ) -> Result<String, CoreError> {
        self.revision_digest(&(
            &record.module_id,
            &record.collection,
            &record.id,
            &record.owner_entity_id,
            encode_field_value(&record.value)?,
            &record.created_at,
            &record.updated_at,
        ))
    }

    fn revision_digest<T: Serialize>(&self, value: &T) -> Result<String, CoreError> {
        revision_digest(&(&self.database_epoch, value))
    }

    fn notify_export_worker(&self) -> Result<(), CoreError> {
        if self.suppress_sync.get() {
            return Ok(());
        }
        if self.root.is_some() {
            if self.export_worker.is_some() {
                self.wake_export_worker();
            } else {
                self.export_latest_generation()?;
            }
        }
        Ok(())
    }

    fn begin_mutation<'a>(
        &'a self,
        request_id: &str,
        result: Option<&serde_json::Value>,
        affected_prefixes: &[String],
    ) -> Result<rusqlite::Transaction<'a>, CoreError> {
        let fingerprint = result
            .map(|value| digest_bytes(value.to_string().as_bytes()))
            .unwrap_or_else(|| digest_bytes(b"null"));
        self.begin_mutation_with_fingerprint(request_id, result, affected_prefixes, &fingerprint)
    }

    fn begin_mutation_with_fingerprint<'a>(
        &'a self,
        request_id: &str,
        result: Option<&serde_json::Value>,
        affected_prefixes: &[String],
        fingerprint: &str,
    ) -> Result<rusqlite::Transaction<'a>, CoreError> {
        let transaction = self.connection.unchecked_transaction()?;
        let _ = affected_prefixes;
        let result_json = result
            .map(serde_json::Value::to_string)
            .unwrap_or_else(|| "null".into());
        let stored_fingerprint: Option<String> = transaction
            .query_row(
                "SELECT fingerprint FROM mutation_receipts WHERE request_id=?1",
                params![request_id],
                |row| row.get(0),
            )
            .optional()?;
        if let Some(stored_fingerprint) = stored_fingerprint {
            if stored_fingerprint != fingerprint {
                return Err(CoreError::Conflict(
                    "request ID was reused with different inputs".into(),
                ));
            }
        }
        transaction.execute(
            "INSERT INTO mutation_receipts(request_id,result,fingerprint,committed_at,state) VALUES (?1,?2,?3,?4,'completed') ON CONFLICT(request_id) DO UPDATE SET result=excluded.result,fingerprint=excluded.fingerprint,committed_at=excluded.committed_at,state='completed'",
            params![request_id, result_json, fingerprint, chrono_like_now()],
        )?;
        Ok(transaction)
    }

    fn write_mutation_result(
        &self,
        request_id: &str,
        result: &serde_json::Value,
    ) -> Result<(), CoreError> {
        self.connection.execute(
            "UPDATE mutation_receipts SET result=?1 WHERE request_id=?2",
            params![result.to_string(), request_id],
        )?;
        Ok(())
    }

    fn export_complete_snapshot(
        &self,
        root: &Path,
        snapshot: &ProjectSnapshot,
        target_generation: Generation,
    ) -> Result<usize, CoreError> {
        let manifest_path = root.join("project.json");
        let manifest: crate::storage::ProjectManifest = crate::storage::read_json(&manifest_path)?;
        manifest.validate(&manifest_path)?;
        let previous_sources = Self::checkpoint_sources(root)?;
        let request_id = Uuid::new_v4().to_string();
        let mut transaction = crate::sync::SyncExporter::begin(root, &request_id)?;
        let staging_root = transaction.staging_root();
        std::fs::create_dir_all(&staging_root).map_err(|error| CoreError::Io {
            operation: "create checkpoint staging root",
            source: error,
        })?;
        let mut documents_by_entity = BTreeMap::new();
        for document in &snapshot.documents {
            documents_by_entity
                .entry(document.entity_id.as_str())
                .or_insert(document);
        }
        let mut fields_by_entity = BTreeMap::<&str, Vec<&FieldValue>>::new();
        for field in &snapshot.fields {
            fields_by_entity
                .entry(field.entity_id.as_str())
                .or_default()
                .push(field);
        }
        let mut relationships_by_entity = BTreeMap::<&str, Vec<&Relationship>>::new();
        for relationship in &snapshot.relationships {
            relationships_by_entity
                .entry(relationship.source_id.as_str())
                .or_default()
                .push(relationship);
        }
        let mut assets_by_entity = BTreeMap::<&str, Vec<&Asset>>::new();
        for asset in &snapshot.assets {
            assets_by_entity
                .entry(asset.entity_id.as_str())
                .or_default()
                .push(asset);
        }
        let namespace_owners = crate::storage::namespace_owners(snapshot)?;
        for entity in &snapshot.entities {
            crate::storage::write_canonical_entity(
                &staging_root,
                entity,
                documents_by_entity.get(entity.id.as_str()).copied(),
                fields_by_entity
                    .get(entity.id.as_str())
                    .map(Vec::as_slice)
                    .unwrap_or_default(),
                relationships_by_entity
                    .get(entity.id.as_str())
                    .map(Vec::as_slice)
                    .unwrap_or_default(),
                assets_by_entity
                    .get(entity.id.as_str())
                    .map(Vec::as_slice)
                    .unwrap_or_default(),
                &namespace_owners,
            )?;
        }
        let plugin_ids = snapshot
            .modules
            .iter()
            .map(|module| module.module_id.as_str())
            .chain(
                snapshot
                    .module_records
                    .iter()
                    .map(|record| record.module_id.as_str()),
            )
            .collect::<BTreeSet<_>>();
        for plugin_id in plugin_ids {
            crate::storage::write_canonical_plugin(
                &staging_root,
                &manifest,
                snapshot,
                plugin_id,
            )?;
        }
        let mut current_sources = staged_canonical_sources(&staging_root, snapshot)?;
        let mut transaction_staged_paths = BTreeSet::new();
        let mut current_path_set = current_sources
            .iter()
            .map(|source| source.path.clone())
            .collect::<BTreeSet<_>>();
        for asset in &snapshot.assets {
            if current_path_set.contains(&asset.path) {
                continue;
            }
            let source = runtime_asset_path(root, &asset.content_hash)?;
            let (runtime_hash, runtime_size) = streamed_file_digest(&source)?;
            if runtime_hash != asset.content_hash || runtime_size != asset.size {
                return Err(CoreError::Conflict(format!(
                    "runtime asset bytes do not match metadata for {}",
                    asset.path
                )));
            }
            let portable_hash = crate::sync::hash_path(root, &asset.path)?;
            if portable_hash.as_deref() != Some(asset.content_hash.as_str()) {
                transaction.stage_file_with_expected(&asset.path, &source, portable_hash)?;
                transaction_staged_paths.insert(asset.path.clone());
            }
            current_path_set.insert(asset.path.clone());
            current_sources.push(crate::storage::CanonicalSource {
                path: asset.path.clone(),
                content_hash: runtime_hash,
                format_version: crate::storage::PROJECT_FORMAT_VERSION,
            });
        }
        if let Some(content_hash) = crate::sync::hash_path(root, "project.json")? {
            current_sources.push(crate::storage::CanonicalSource {
                path: "project.json".into(),
                content_hash,
                format_version: crate::storage::PROJECT_FORMAT_VERSION,
            });
        }
        current_sources.sort_by(|left, right| left.path.cmp(&right.path));
        current_sources.dedup_by(|left, right| left.path == right.path);
        let current_paths = current_sources
            .iter()
            .map(|source| source.path.as_str())
            .collect::<BTreeSet<_>>();
        for source in &current_sources {
            if source.path == "project.json" || transaction_staged_paths.contains(&source.path) {
                continue;
            }
            let portable_hash = crate::sync::hash_path(root, &source.path)?;
            if portable_hash.as_deref() == Some(source.content_hash.as_str()) {
                continue;
            }
            let staged = crate::storage::normalized_project_path(&staging_root, &source.path)?;
            let bytes = std::fs::read(&staged).map_err(|error| {
                CoreError::Validation(format!(
                    "checkpoint staging file {} is unavailable: {error}",
                    staged.display()
                ))
            })?;
            transaction.stage_bytes_with_expected(&source.path, &bytes, portable_hash)?;
        }
        for source in &previous_sources {
            if source.path != "project.json" && !current_paths.contains(source.path.as_str()) {
                transaction.stage_remove(&source.path)?;
            }
        }
        let applied = transaction.commit(None::<&serde_json::Value>)?;
        self.install_checkpoint_manifest(root, target_generation)?;
        Ok(applied.len())
    }

    #[allow(clippy::too_many_arguments)]
    fn install_checkpoint_manifest(
        &self,
        root: &Path,
        generation: Generation,
    ) -> Result<(), CoreError> {
        let checkpoint = crate::storage::build_checkpoint_manifest(root, generation)?;
        let digest =
            crate::storage::canonical_json_bytes(&checkpoint).map(|bytes| digest_bytes(&bytes))?;
        crate::storage::write_checkpoint_manifest(root, &checkpoint)?;
        let transaction = self.connection.unchecked_transaction()?;
        transaction.execute(
            "UPDATE runtime_meta SET exported_generation=CASE WHEN content_generation>=?1 THEN ?1 ELSE exported_generation END, checkpoint_digest=?2, export_error=NULL WHERE key='runtime'",
            params![generation, digest],
        )?;
        transaction.commit()?;
        Ok(())
    }

    pub fn in_memory() -> Result<Self, CoreError> {
        Self::open_database(":memory:", None, None, true, false)
    }

    pub fn info(&self) -> Option<ProjectInfo> {
        let root = self.root.as_ref()?;
        let sync = self.sync_summary().unwrap_or_else(|_| SyncSummary {
            state: "diagnostic".into(),
            dirty_count: 0,
            export_error: Some("runtime metadata is unavailable".into()),
        });
        Some(ProjectInfo {
            name: crate::storage::read_json::<crate::storage::ProjectManifest>(
                &root.join("project.json"),
            )
            .map(|manifest| manifest.name)
            .unwrap_or_else(|_| {
                root.file_name()
                    .and_then(|value| value.to_str())
                    .unwrap_or("Daena Archive project")
                    .to_string()
            }),
            root: root.to_string_lossy().to_string(),
            index_status: if sync.state == "clean" {
                "ready"
            } else {
                "diagnostic"
            }
            .into(),
            assets: root.join("assets").to_string_lossy().to_string(),
            sync,
        })
    }

    pub fn database_epoch(&self) -> &str {
        &self.database_epoch
    }

    pub fn sync_summary(&self) -> Result<SyncSummary, CoreError> {
        let (content, exported, export_error): (i64, i64, Option<String>) = self.connection.query_row(
            "SELECT content_generation,exported_generation,export_error FROM runtime_meta WHERE key='runtime'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )?;
        let state = if export_error.is_some() {
            "failed"
        } else if exported < content {
            "pending"
        } else {
            "clean"
        };
        Ok(SyncSummary {
            state: state.into(),
            dirty_count: content.saturating_sub(exported),
            export_error,
        })
    }

    fn runtime_requires_recovery_archive(&self) -> Result<bool, CoreError> {
        let (exported, content, export_error): (i64, i64, Option<String>) =
            self.connection.query_row(
                "SELECT exported_generation,content_generation,export_error FROM runtime_meta WHERE key='runtime'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )?;
        Ok(export_error.is_some() || exported < content)
    }

    fn project_root(&self) -> Result<&Path, CoreError> {
        self.root
            .as_deref()
            .ok_or_else(|| CoreError::NotFound("project is not directory-backed".into()))
    }

    fn request_id(&self, request_id: Option<&str>) -> Result<String, CoreError> {
        let request_id = request_id
            .map(str::to_owned)
            .unwrap_or_else(|| Uuid::new_v4().to_string());
        Uuid::parse_str(&request_id)
            .map_err(|_| CoreError::Validation("transaction request ID must be a UUID".into()))?;
        Ok(request_id)
    }

    fn committed_mutation<T: DeserializeOwned>(
        &self,
        request_id: Option<&str>,
    ) -> Result<Option<T>, CoreError> {
        self.committed_mutation_with_fingerprint(request_id, None)
    }

    fn committed_mutation_with_fingerprint<T: DeserializeOwned>(
        &self,
        request_id: Option<&str>,
        fingerprint: Option<&str>,
    ) -> Result<Option<T>, CoreError> {
        let Some(request_id) = request_id else {
            return Ok(None);
        };
        self.request_id(Some(request_id))?;
        let Some(_root) = self.root.as_deref() else {
            return Ok(None);
        };
        if let Some(fingerprint) = fingerprint {
            let stored: Option<String> = self
                .connection
                .query_row(
                    "SELECT fingerprint FROM mutation_receipts WHERE request_id=?1",
                    params![request_id],
                    |row| row.get(0),
                )
                .optional()?;
            if let Some(stored) = stored {
                if stored != fingerprint {
                    return Err(CoreError::Conflict(
                        "request ID was reused with different inputs".into(),
                    ));
                }
            }
        }
        let receipt: Option<(String, String)> = self
            .connection
            .query_row(
                "SELECT result,fingerprint FROM mutation_receipts WHERE request_id=?1 AND state='completed'",
                params![request_id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()?;
        let result = match receipt {
            Some((result, stored_fingerprint)) => {
                if let Some(fingerprint) = fingerprint {
                    if fingerprint != stored_fingerprint {
                        return Err(CoreError::Conflict(
                            "request ID was reused with different inputs".into(),
                        ));
                    }
                }
                result
            }
            None => return Ok(None),
        };
        let result = serde_json::from_str(&result)
            .map_err(|error| CoreError::Serialization(error.to_string()))?;
        serde_json::from_value(result)
            .map(Some)
            .map_err(|error| CoreError::Serialization(error.to_string()))
    }

    fn ensure_expected_revision(
        expected: Option<&str>,
        actual: String,
        record: &str,
    ) -> Result<(), CoreError> {
        if let Some(expected) = expected {
            if expected != actual {
                return Err(CoreError::Conflict(format!(
                    "{record} revision conflict: expected {expected}, current {actual}"
                )));
            }
        }
        Ok(())
    }

    fn run_git(&self, args: &[&str]) -> Result<std::process::Output, CoreError> {
        let root = self.project_root()?;
        Command::new("git")
            .args(args)
            .current_dir(root)
            .output()
            .map_err(|error| CoreError::NotFound(format!("git is unavailable: {error}")))
    }

    fn git_status_entries(&self) -> Result<Vec<(u8, u8, String)>, CoreError> {
        let output = self.run_git(&["status", "--porcelain=v1", "-z", "--untracked-files=all"])?;
        if !output.status.success() {
            return Err(CoreError::Git(
                String::from_utf8_lossy(&output.stderr).trim().to_string(),
            ));
        }
        let mut entries = Vec::new();
        let mut records = output.stdout.split(|byte| *byte == 0);
        while let Some(record) = records.next() {
            if record.len() < 4 || record[2] != b' ' {
                continue;
            }
            let index_status = record[0];
            let worktree_status = record[1];
            let mut path = String::from_utf8_lossy(&record[3..]).into_owned();
            if matches!(index_status, b'R' | b'C') || matches!(worktree_status, b'R' | b'C') {
                if let Some(new_path) = records.next() {
                    path = String::from_utf8_lossy(new_path).into_owned();
                }
            }
            if !path.is_empty() {
                entries.push((index_status, worktree_status, path));
            }
        }
        Ok(entries)
    }

    fn git_staged_paths(&self) -> Result<Vec<String>, CoreError> {
        let output = self.run_git(&["diff", "--cached", "--name-only", "-z"])?;
        if !output.status.success() {
            return Err(CoreError::Git(
                String::from_utf8_lossy(&output.stderr).trim().to_string(),
            ));
        }
        Ok(output
            .stdout
            .split(|byte| *byte == 0)
            .filter(|path| !path.is_empty())
            .map(|path| String::from_utf8_lossy(path).into_owned())
            .collect())
    }

    fn is_canonical_git_path(path: &str) -> bool {
        path == "project.json"
            || path == ".gitignore"
            || path.starts_with("entities/")
            || path.starts_with("plugins/")
            || path.starts_with("assets/")
    }

    fn is_asset_git_path(path: &str) -> bool {
        path.starts_with("assets/")
    }

    pub fn git_status(&self) -> Result<GitStatus, CoreError> {
        let repository = self.run_git(&["rev-parse", "--is-inside-work-tree"])?;
        if !repository.status.success() {
            return Ok(GitStatus {
                repository: false,
                branch: None,
                changes: Vec::new(),
                canonical_changes: Vec::new(),
                staged_canonical_changes: Vec::new(),
            });
        }
        let branch = self.run_git(&["branch", "--show-current"])?;
        let entries = self.git_status_entries()?;
        let changes = entries
            .iter()
            .map(|(index_status, worktree_status, path)| {
                format!(
                    "{}{} {}",
                    *index_status as char, *worktree_status as char, path
                )
            })
            .collect::<Vec<_>>();
        let canonical_changes = entries
            .iter()
            .filter(|(_, _, path)| Self::is_canonical_git_path(path))
            .map(|(_, _, path)| path.clone())
            .collect::<Vec<_>>();
        let staged_canonical_changes = self
            .git_staged_paths()?
            .into_iter()
            .filter(|path| Self::is_canonical_git_path(path))
            .collect();
        Ok(GitStatus {
            repository: true,
            branch: Some(String::from_utf8_lossy(&branch.stdout).trim().to_string()),
            changes,
            canonical_changes,
            staged_canonical_changes,
        })
    }

    pub fn git_init(&self) -> Result<GitStatus, CoreError> {
        let output = self.run_git(&["init"])?;
        if !output.status.success() {
            return Err(CoreError::NotFound(
                String::from_utf8_lossy(&output.stderr).trim().into(),
            ));
        }
        self.git_status()
    }

    pub fn git_log(&self) -> Result<Vec<GitLogEntry>, CoreError> {
        if !self.git_status()?.repository {
            return Ok(Vec::new());
        }
        let output = self.run_git(&[
            "log",
            "-50",
            "--date=iso-strict",
            "--pretty=format:%h%x09%ad%x09%s",
        ])?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            if stderr.contains("does not have any commits yet") {
                return Ok(Vec::new());
            }
            return Err(CoreError::NotFound(stderr.trim().into()));
        }
        Ok(String::from_utf8_lossy(&output.stdout)
            .lines()
            .filter_map(|line| {
                let mut parts = line.splitn(3, '\t');
                Some(GitLogEntry {
                    hash: parts.next()?.into(),
                    date: parts.next()?.into(),
                    subject: parts.next()?.into(),
                })
            })
            .collect())
    }

    pub fn git_preflight(&self) -> Result<GitPreflight, CoreError> {
        self.flush_checkpoint("git preflight")?;
        self.git_preflight_after_checkpoint()
    }

    pub fn git_preflight_after_checkpoint(&self) -> Result<GitPreflight, CoreError> {
        let status = self.git_status()?;
        if !status.repository {
            return Ok(GitPreflight {
                ready: false,
                diagnostics: vec!["project is not a git repository".into()],
                canonical_paths: Vec::new(),
                asset_paths: Vec::new(),
                staging_paths: Vec::new(),
                staged_paths: Vec::new(),
                unmerged_paths: Vec::new(),
            });
        }

        let entries = self.git_status_entries()?;
        let mut diagnostics = Vec::new();
        let unmerged_paths = entries
            .iter()
            .filter(|(index_status, worktree_status, _)| {
                matches!(
                    (*index_status, *worktree_status),
                    (b'D', b'D')
                        | (b'A', b'U')
                        | (b'U', b'D')
                        | (b'U', b'A')
                        | (b'D', b'U')
                        | (b'A', b'A')
                        | (b'U', b'U')
                )
            })
            .map(|(_, _, path)| path.clone())
            .collect::<Vec<_>>();
        if !unmerged_paths.is_empty() {
            diagnostics.extend(
                unmerged_paths
                    .iter()
                    .map(|path| format!("git.unmerged: {path}")),
            );
        }

        let report = if unmerged_paths.is_empty() {
            ExternalChangeReport {
                changed: false,
                paths: Vec::new(),
                diagnostics: Vec::new(),
            }
        } else {
            ExternalChangeReport {
                changed: false,
                paths: unmerged_paths.clone(),
                diagnostics: Vec::new(),
            }
        };
        diagnostics.extend(report.diagnostics);
        let sync = self.sync_summary()?;
        if sync.state != "clean" {
            diagnostics.push(format!("index.state: {}", sync.state));
        }

        // The flush may have created or replaced portable paths. Re-read Git
        // state before constructing the staging preview so queued exports are
        // visible to the caller.
        let entries = self.git_status_entries()?;
        let staged_paths = self.git_staged_paths()?;

        let noncanonical_staged = staged_paths
            .iter()
            .filter(|path| !Self::is_canonical_git_path(path))
            .cloned()
            .collect::<Vec<_>>();
        if !noncanonical_staged.is_empty() {
            diagnostics.push(format!(
                "git.noncanonical-staged: {}",
                noncanonical_staged.join(", ")
            ));
        }

        let canonical_paths = entries
            .iter()
            .filter(|(_, _, path)| Self::is_canonical_git_path(path))
            .map(|(_, _, path)| path.clone())
            .chain(
                staged_paths
                    .iter()
                    .filter(|path| Self::is_canonical_git_path(path))
                    .cloned(),
            )
            .collect::<BTreeSet<_>>();
        let mut canonical_paths = canonical_paths.iter().cloned().collect::<Vec<_>>();
        let asset_paths = canonical_paths
            .iter()
            .filter(|path| Self::is_asset_git_path(path))
            .cloned()
            .collect::<Vec<_>>();
        let staging_paths = canonical_paths.clone();
        canonical_paths.shrink_to_fit();

        Ok(GitPreflight {
            ready: diagnostics.is_empty(),
            diagnostics,
            canonical_paths,
            asset_paths,
            staging_paths,
            staged_paths,
            unmerged_paths,
        })
    }

    pub fn git_staging_preview(&self) -> Result<GitPreflight, CoreError> {
        self.git_preflight()
    }

    pub fn git_commit(
        &self,
        message: String,
        paths: Option<Vec<String>>,
    ) -> Result<GitStatus, CoreError> {
        self.flush_checkpoint("git commit")?;
        self.git_commit_after_checkpoint(message, paths)
    }

    pub fn git_commit_after_checkpoint(
        &self,
        message: String,
        paths: Option<Vec<String>>,
    ) -> Result<GitStatus, CoreError> {
        if message.trim().is_empty() {
            return Err(CoreError::NotFound("commit message cannot be empty".into()));
        }
        let preflight = self.git_preflight_after_checkpoint()?;
        if !preflight.ready {
            return Err(CoreError::Conflict(format!(
                "cannot commit while canonical diagnostics remain: {}",
                preflight.diagnostics.join("; ")
            )));
        }
        if preflight.staging_paths.is_empty() {
            return Err(CoreError::Git(
                "no canonical project changes to commit".into(),
            ));
        }
        let selected = match paths {
            Some(paths) if paths.is_empty() => {
                return Err(CoreError::Git("no paths selected for commit".into()));
            }
            Some(paths) => {
                let allowed = preflight
                    .staging_paths
                    .iter()
                    .cloned()
                    .collect::<BTreeSet<_>>();
                for path in &paths {
                    if !allowed.contains(path) {
                        return Err(CoreError::Git(format!(
                            "path is not in the canonical staging preview: {path}"
                        )));
                    }
                    if !Self::is_canonical_git_path(path) {
                        return Err(CoreError::Git(format!(
                            "path is not a canonical project path: {path}"
                        )));
                    }
                }
                paths
            }
            None => preflight.staging_paths.clone(),
        };
        // Rebuild the canonical portion of the index so a selective commit
        // cannot accidentally include canonical paths staged before the UI
        // preview. `git reset` without a worktree target preserves all edits.
        let mut reset_args = vec![
            "reset".to_string(),
            "--mixed".to_string(),
            "HEAD".to_string(),
            "--".to_string(),
        ];
        reset_args.extend(preflight.staging_paths.iter().cloned());
        let reset_args = reset_args.iter().map(String::as_str).collect::<Vec<_>>();
        let reset = self.run_git(&reset_args)?;
        if !reset.status.success() {
            return Err(CoreError::Git(
                String::from_utf8_lossy(&reset.stderr).trim().into(),
            ));
        }

        let mut add_args = vec!["add".to_string(), "--all".to_string(), "--".to_string()];
        add_args.extend(selected);
        let add_args = add_args.iter().map(String::as_str).collect::<Vec<_>>();
        let add = self.run_git(&add_args)?;
        if !add.status.success() {
            return Err(CoreError::Git(
                String::from_utf8_lossy(&add.stderr).trim().into(),
            ));
        }
        let commit = self.run_git(&["commit", "-m", message.trim()])?;
        if !commit.status.success() {
            return Err(CoreError::Git(
                String::from_utf8_lossy(&commit.stderr).trim().into(),
            ));
        }
        self.git_status()
    }

    pub fn git_super_squash_after_checkpoint(&self, message: &str) -> Result<GitStatus, CoreError> {
        if message.trim().is_empty() {
            return Err(CoreError::NotFound("snapshot message cannot be empty".into()));
        }
        let preflight = self.git_preflight_after_checkpoint()?;
        if !preflight.ready {
            return Err(CoreError::Conflict(format!(
                "cannot squash while canonical diagnostics remain: {}",
                preflight.diagnostics.join("; ")
            )));
        }
        if !preflight.staging_paths.is_empty() {
            return Err(CoreError::Git(
                "commit snapshot-ready changes before squashing history".into(),
            ));
        }
        let head = self.run_git(&["rev-parse", "--verify", "HEAD"])?;
        if !head.status.success() {
            return Err(CoreError::Git("no snapshot history to squash".into()));
        }
        let read_tree = self.run_git(&["read-tree", "HEAD"])?;
        if !read_tree.status.success() {
            return Err(CoreError::Git(
                String::from_utf8_lossy(&read_tree.stderr).trim().into(),
            ));
        }
        let tree = self.run_git(&["write-tree"])?;
        if !tree.status.success() {
            return Err(CoreError::Git(
                String::from_utf8_lossy(&tree.stderr).trim().into(),
            ));
        }
        let tree_hash = String::from_utf8_lossy(&tree.stdout).trim().to_owned();
        let commit = self.run_git(&["commit-tree", &tree_hash, "-m", message.trim()])?;
        if !commit.status.success() {
            return Err(CoreError::Git(
                String::from_utf8_lossy(&commit.stderr).trim().into(),
            ));
        }
        let commit_hash = String::from_utf8_lossy(&commit.stdout).trim().to_owned();
        let reset = self.run_git(&["reset", "--soft", &commit_hash])?;
        if !reset.status.success() {
            return Err(CoreError::Git(
                String::from_utf8_lossy(&reset.stderr).trim().into(),
            ));
        }
        self.git_status()
    }

    pub fn git_tool_info() -> GitToolInfo {
        match Command::new("git").args(["--version"]).output() {
            Ok(output) if output.status.success() => {
                let version = String::from_utf8_lossy(&output.stdout).trim().to_string();
                GitToolInfo {
                    available: true,
                    version: Some(version),
                    error: None,
                }
            }
            Ok(output) => {
                let error = String::from_utf8_lossy(&output.stderr).trim().to_string();
                GitToolInfo {
                    available: false,
                    version: None,
                    error: Some(if error.is_empty() {
                        "git --version failed".into()
                    } else {
                        error
                    }),
                }
            }
            Err(error) => GitToolInfo {
                available: false,
                version: None,
                error: Some(format!("git is unavailable: {error}")),
            },
        }
    }

    fn git_rev_parse(&self, rev: &str) -> Result<Option<String>, CoreError> {
        let output = self.run_git(&["rev-parse", rev])?;
        if !output.status.success() {
            return Ok(None);
        }
        let hash = String::from_utf8_lossy(&output.stdout).trim().to_string();
        Ok((!hash.is_empty()).then_some(hash))
    }

    fn git_upstream(&self) -> Result<Option<GitUpstream>, CoreError> {
        let remote =
            self.run_git(&["rev-parse", "--abbrev-ref", "--symbolic-full-name", "@{u}"])?;
        if !remote.status.success() {
            return Ok(None);
        }
        let full = String::from_utf8_lossy(&remote.stdout).trim().to_string();
        let Some((remote_name, branch)) = full.split_once('/') else {
            return Ok(None);
        };
        let remote_hash = self.git_rev_parse("@{u}")?;
        Ok(Some(GitUpstream {
            remote: remote_name.to_string(),
            branch: branch.to_string(),
            remote_hash,
        }))
    }

    fn validate_remote_url(url: &str) -> Result<(), CoreError> {
        let url = url.trim();
        if url.is_empty() {
            return Err(CoreError::Validation("remote URL cannot be empty".into()));
        }
        let ok = url.starts_with("https://")
            || url.starts_with("http://")
            || url.starts_with("ssh://")
            || url.starts_with("git://")
            || url.starts_with("git@")
            || url.starts_with("file://")
            || Path::new(url).is_absolute();
        if !ok {
            return Err(CoreError::Validation(format!(
                "unsupported remote URL: {url}"
            )));
        }
        Ok(())
    }

    fn validate_remote_name(name: &str) -> Result<(), CoreError> {
        let name = name.trim();
        if name.is_empty() {
            return Err(CoreError::Validation("remote name cannot be empty".into()));
        }
        if !name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
        {
            return Err(CoreError::Validation(format!(
                "invalid remote name: {name}"
            )));
        }
        Ok(())
    }

    pub fn git_show_tree(&self, hash: &str) -> Result<Vec<String>, CoreError> {
        if !self.git_status()?.repository {
            return Err(CoreError::Git("project is not a git repository".into()));
        }
        let hash = hash.trim();
        if hash.is_empty() {
            return Err(CoreError::Validation("commit hash cannot be empty".into()));
        }
        let output = self.run_git(&["ls-tree", "-r", "--name-only", hash])?;
        if !output.status.success() {
            return Err(CoreError::Git(
                String::from_utf8_lossy(&output.stderr).trim().into(),
            ));
        }
        Ok(String::from_utf8_lossy(&output.stdout)
            .lines()
            .map(str::trim)
            .filter(|path| !path.is_empty() && Self::is_canonical_git_path(path))
            .map(str::to_string)
            .collect())
    }

    pub fn git_show_message(&self, hash: &str) -> Result<String, CoreError> {
        if !self.git_status()?.repository {
            return Err(CoreError::Git("project is not a git repository".into()));
        }
        let hash = hash.trim();
        if hash.is_empty() {
            return Err(CoreError::Validation("commit hash cannot be empty".into()));
        }
        let output = self.run_git(&["show", "-s", "--format=%B", hash])?;
        if !output.status.success() {
            return Err(CoreError::Git(
                String::from_utf8_lossy(&output.stderr).trim().into(),
            ));
        }
        Ok(String::from_utf8_lossy(&output.stdout).trim().into())
    }

    pub fn git_show_changes(&self, hash: &str) -> Result<Vec<GitChange>, CoreError> {
        if !self.git_status()?.repository {
            return Err(CoreError::Git("project is not a git repository".into()));
        }
        let hash = hash.trim();
        if hash.is_empty() {
            return Err(CoreError::Validation("commit hash cannot be empty".into()));
        }
        let output = self.run_git(&[
            "diff-tree",
            "--root",
            "--no-commit-id",
            "--name-status",
            "-r",
            "--find-renames",
            hash,
        ])?;
        if !output.status.success() {
            return Err(CoreError::Git(
                String::from_utf8_lossy(&output.stderr).trim().into(),
            ));
        }
        Ok(String::from_utf8_lossy(&output.stdout)
            .lines()
            .filter_map(|line| {
                let mut parts = line.splitn(2, '\t');
                let status = parts.next()?.trim();
                let path = parts.next()?.trim();
                if status.is_empty() || path.is_empty() || !Self::is_canonical_git_path(path) {
                    return None;
                }
                Some(GitChange {
                    status: status.into(),
                    path: path.into(),
                })
            })
            .collect())
    }

    pub fn git_show_diff(&self, hash: &str, path: &str) -> Result<String, CoreError> {
        if !self.git_status()?.repository {
            return Err(CoreError::Git("project is not a git repository".into()));
        }
        let hash = hash.trim();
        let path = path.trim();
        if hash.is_empty() || path.is_empty() {
            return Err(CoreError::Validation(
                "commit hash and path are required".into(),
            ));
        }
        if !Self::is_canonical_git_path(path) {
            return Err(CoreError::Validation(
                "snapshot diffs are limited to canonical paths".into(),
            ));
        }
        let output = self.run_git(&[
            "diff-tree",
            "--root",
            "-p",
            "--no-commit-id",
            hash,
            "--",
            path,
        ])?;
        if !output.status.success() {
            return Err(CoreError::Git(
                String::from_utf8_lossy(&output.stderr).trim().into(),
            ));
        }
        Ok(String::from_utf8_lossy(&output.stdout).into())
    }

    pub fn git_worktree_diff(&self, paths: &[String]) -> Result<String, CoreError> {
        if !self.git_status()?.repository {
            return Err(CoreError::Git("project is not a git repository".into()));
        }
        if paths.is_empty() {
            return Err(CoreError::Validation("at least one diff path is required".into()));
        }
        if paths.iter().any(|path| !Self::is_canonical_git_path(path)) {
            return Err(CoreError::Validation(
                "snapshot diffs are limited to canonical paths".into(),
            ));
        }
        let head = self.run_git(&["rev-parse", "--verify", "HEAD"])?;
        if !head.status.success() {
            let mut combined = String::new();
            for path in paths {
                let output = self.run_git(&[
                    "diff",
                    "--no-index",
                    "--no-color",
                    "--no-ext-diff",
                    "--unified=3",
                    "/dev/null",
                    path,
                ])?;
                if !output.status.success() && output.status.code() != Some(1) {
                    return Err(CoreError::Git(
                        String::from_utf8_lossy(&output.stderr).trim().into(),
                    ));
                }
                combined.push_str(&String::from_utf8_lossy(&output.stdout));
            }
            return Ok(combined);
        }
        let mut args = vec![
            "diff".to_string(),
            "HEAD".to_string(),
            "--no-color".to_string(),
            "--no-ext-diff".to_string(),
            "--unified=3".to_string(),
            "--".to_string(),
        ];
        args.extend(paths.iter().cloned());
        let args = args.iter().map(String::as_str).collect::<Vec<_>>();
        let output = self.run_git(&args)?;
        if !output.status.success() {
            return Err(CoreError::Git(
                String::from_utf8_lossy(&output.stderr).trim().into(),
            ));
        }
        Ok(String::from_utf8_lossy(&output.stdout).into())
    }

    pub fn git_show_file(&self, hash: &str, path: &str) -> Result<String, CoreError> {
        if !self.git_status()?.repository {
            return Err(CoreError::Git("project is not a git repository".into()));
        }
        let hash = hash.trim();
        let path = path.trim();
        if hash.is_empty() || path.is_empty() {
            return Err(CoreError::Validation(
                "commit hash and path are required".into(),
            ));
        }
        if !Self::is_canonical_git_path(path) {
            return Err(CoreError::Validation(format!(
                "path is not a canonical project path: {path}"
            )));
        }
        if path.contains('\0') || path.starts_with('-') {
            return Err(CoreError::Validation("invalid snapshot path".into()));
        }
        let spec = format!("{hash}:{path}");
        let output = self.run_git(&["show", &spec])?;
        if !output.status.success() {
            return Err(CoreError::Git(
                String::from_utf8_lossy(&output.stderr).trim().into(),
            ));
        }
        if output.stdout.len() > GIT_SHOW_FILE_MAX_BYTES {
            return Err(CoreError::Validation(format!(
                "snapshot file is too large to preview ({} bytes)",
                output.stdout.len()
            )));
        }
        if output.stdout.contains(&0) {
            return Err(CoreError::Validation(
                "binary snapshot files cannot be previewed".into(),
            ));
        }
        String::from_utf8(output.stdout)
            .map_err(|_| CoreError::Validation("snapshot file is not valid UTF-8 text".into()))
    }

    pub fn git_reset_hard(&mut self, hash: &str) -> Result<GitResetResult, CoreError> {
        if !self.git_status()?.repository {
            return Err(CoreError::Git("project is not a git repository".into()));
        }
        let preflight = self.git_preflight()?;
        if !preflight.ready {
            return Err(CoreError::Conflict(format!(
                "cannot reset while canonical diagnostics remain: {}",
                preflight.diagnostics.join("; ")
            )));
        }
        let hash = hash.trim();
        if hash.is_empty() {
            return Err(CoreError::Validation("commit hash cannot be empty".into()));
        }
        let previous_head = self.git_rev_parse("HEAD")?;
        let verify = self.run_git(&["rev-parse", "--verify", hash])?;
        if !verify.status.success() {
            return Err(CoreError::Git(format!(
                "unknown commit: {}",
                String::from_utf8_lossy(&verify.stderr).trim()
            )));
        }
        let upstream_before = self.git_upstream()?;
        let reset = self.run_git(&["reset", "--hard", hash])?;
        if !reset.status.success() {
            return Err(CoreError::Git(
                String::from_utf8_lossy(&reset.stderr).trim().into(),
            ));
        }
        let root = self.project_root()?.to_path_buf();
        let generation: Generation = self.connection.query_row(
            "SELECT content_generation FROM runtime_meta WHERE key='runtime'",
            [],
            |row| row.get(0),
        )?;
        self.install_checkpoint_manifest(&root, generation)?;
        let rebuild = self.import_checkpoint()?;
        let current_head = self.git_rev_parse("HEAD")?;
        let upstream = self.git_upstream()?.or(upstream_before);
        let diverged_from_upstream = match (&upstream, &current_head) {
            (Some(upstream), Some(head)) => upstream
                .remote_hash
                .as_ref()
                .is_some_and(|remote| remote != head),
            (Some(_), _) => true,
            _ => false,
        };
        Ok(GitResetResult {
            status: self.git_status()?,
            previous_head,
            current_head,
            upstream,
            diverged_from_upstream,
            rebuild,
        })
    }

    pub fn git_remote_list(&self) -> Result<Vec<GitRemote>, CoreError> {
        if !self.git_status()?.repository {
            return Ok(Vec::new());
        }
        let output = self.run_git(&["remote", "-v"])?;
        if !output.status.success() {
            return Err(CoreError::Git(
                String::from_utf8_lossy(&output.stderr).trim().into(),
            ));
        }
        let mut remotes = BTreeMap::<String, GitRemote>::new();
        for line in String::from_utf8_lossy(&output.stdout).lines() {
            let mut parts = line.split_whitespace();
            let Some(name) = parts.next() else { continue };
            let Some(url) = parts.next() else { continue };
            let mode = parts.next().unwrap_or("(fetch)");
            let entry = remotes.entry(name.to_string()).or_insert(GitRemote {
                name: name.to_string(),
                fetch_url: String::new(),
                push_url: String::new(),
            });
            if mode.contains("push") {
                entry.push_url = url.to_string();
            } else {
                entry.fetch_url = url.to_string();
            }
        }
        for remote in remotes.values_mut() {
            if remote.push_url.is_empty() {
                remote.push_url = remote.fetch_url.clone();
            }
            if remote.fetch_url.is_empty() {
                remote.fetch_url = remote.push_url.clone();
            }
        }
        Ok(remotes.into_values().collect())
    }

    pub fn git_remote_add(&self, name: &str, url: &str) -> Result<Vec<GitRemote>, CoreError> {
        if !self.git_status()?.repository {
            return Err(CoreError::Git("project is not a git repository".into()));
        }
        Self::validate_remote_name(name)?;
        Self::validate_remote_url(url)?;
        let output = self.run_git(&["remote", "add", name.trim(), url.trim()])?;
        if !output.status.success() {
            return Err(CoreError::Git(
                String::from_utf8_lossy(&output.stderr).trim().into(),
            ));
        }
        self.git_remote_list()
    }

    pub fn git_remote_set_url(&self, name: &str, url: &str) -> Result<Vec<GitRemote>, CoreError> {
        if !self.git_status()?.repository {
            return Err(CoreError::Git("project is not a git repository".into()));
        }
        Self::validate_remote_name(name)?;
        Self::validate_remote_url(url)?;
        let output = self.run_git(&["remote", "set-url", name.trim(), url.trim()])?;
        if !output.status.success() {
            return Err(CoreError::Git(
                String::from_utf8_lossy(&output.stderr).trim().into(),
            ));
        }
        self.git_remote_list()
    }

    pub fn git_remote_remove(&self, name: &str) -> Result<Vec<GitRemote>, CoreError> {
        if !self.git_status()?.repository {
            return Err(CoreError::Git("project is not a git repository".into()));
        }
        Self::validate_remote_name(name)?;
        let output = self.run_git(&["remote", "remove", name.trim()])?;
        if !output.status.success() {
            return Err(CoreError::Git(
                String::from_utf8_lossy(&output.stderr).trim().into(),
            ));
        }
        self.git_remote_list()
    }

    pub fn git_push(
        &self,
        remote: &str,
        branch: Option<&str>,
        force_with_lease: bool,
    ) -> Result<GitStatus, CoreError> {
        if !self.git_status()?.repository {
            return Err(CoreError::Git("project is not a git repository".into()));
        }
        Self::validate_remote_name(remote)?;
        let branch = match branch {
            Some(branch) if !branch.trim().is_empty() => branch.trim().to_string(),
            _ => {
                let status = self.git_status()?;
                status
                    .branch
                    .filter(|value| !value.is_empty())
                    .ok_or_else(|| CoreError::Git("current branch is required for push".into()))?
            }
        };
        let mut args = vec!["push".to_string()];
        if force_with_lease {
            args.push("--force-with-lease".into());
        }
        args.push(remote.trim().into());
        args.push(branch);
        let args = args.iter().map(String::as_str).collect::<Vec<_>>();
        let output = self.run_git(&args)?;
        if !output.status.success() {
            return Err(CoreError::Git(
                String::from_utf8_lossy(&output.stderr).trim().into(),
            ));
        }
        self.git_status()
    }

    pub fn git_restore_from_upstream(&mut self) -> Result<GitResetResult, CoreError> {
        if !self.git_status()?.repository {
            return Err(CoreError::Git("project is not a git repository".into()));
        }
        let preflight = self.git_preflight()?;
        if !preflight.ready {
            return Err(CoreError::Conflict(format!(
                "cannot restore upstream while canonical diagnostics remain: {}",
                preflight.diagnostics.join("; ")
            )));
        }
        let upstream = self
            .git_upstream()?
            .ok_or_else(|| CoreError::Git("no upstream branch is configured for restore".into()))?;
        let previous_head = self.git_rev_parse("HEAD")?;
        let fetch = self.run_git(&["fetch", &upstream.remote, &upstream.branch])?;
        if !fetch.status.success() {
            return Err(CoreError::Git(
                String::from_utf8_lossy(&fetch.stderr).trim().into(),
            ));
        }
        let upstream_ref = format!("{}/{}", upstream.remote, upstream.branch);
        let reset = self.run_git(&["reset", "--hard", &upstream_ref])?;
        if !reset.status.success() {
            return Err(CoreError::Git(
                String::from_utf8_lossy(&reset.stderr).trim().into(),
            ));
        }
        let root = self.project_root()?.to_path_buf();
        let generation: Generation = self.connection.query_row(
            "SELECT content_generation FROM runtime_meta WHERE key='runtime'",
            [],
            |row| row.get(0),
        )?;
        self.install_checkpoint_manifest(&root, generation)?;
        let rebuild = self.import_checkpoint()?;
        let current_head = self.git_rev_parse("HEAD")?;
        let upstream = self.git_upstream()?;
        Ok(GitResetResult {
            status: self.git_status()?,
            previous_head,
            current_head,
            upstream,
            diverged_from_upstream: false,
            rebuild,
        })
    }

    fn initialize(&self, rebuild_derived: bool) -> Result<(), CoreError> {
        self.connection.execute_batch(
            "PRAGMA journal_mode = WAL;
             CREATE TABLE IF NOT EXISTS runtime_meta (
               key TEXT PRIMARY KEY,
               storage_role TEXT NOT NULL,
               schema_version INTEGER NOT NULL,
               project_id TEXT NOT NULL,
               portable_format_version INTEGER NOT NULL,
               database_epoch TEXT NOT NULL,
               exporter_version TEXT NOT NULL,
               content_generation INTEGER NOT NULL DEFAULT 0,
               exported_generation INTEGER NOT NULL DEFAULT 0,
               checkpoint_digest TEXT,
               export_error TEXT
             );
             CREATE TABLE IF NOT EXISTS mutation_receipts (
               request_id TEXT PRIMARY KEY,
               result TEXT NOT NULL,
               fingerprint TEXT NOT NULL DEFAULT '',
               committed_at TEXT NOT NULL,
               state TEXT NOT NULL DEFAULT 'pending'
             );
             CREATE TABLE IF NOT EXISTS project_meta (key TEXT PRIMARY KEY, value TEXT NOT NULL);
             INSERT OR IGNORE INTO project_meta(key, value) VALUES ('schema_version', '1');
             CREATE TABLE IF NOT EXISTS entities (
               id TEXT PRIMARY KEY, name TEXT NOT NULL, entity_type TEXT,
               deleted INTEGER NOT NULL DEFAULT 0, created_at TEXT NOT NULL, updated_at TEXT NOT NULL
             );
             CREATE TABLE IF NOT EXISTS documents (
               id TEXT PRIMARY KEY, entity_id TEXT NOT NULL REFERENCES entities(id),
               format TEXT NOT NULL, body TEXT NOT NULL, updated_at TEXT NOT NULL
             );
             CREATE TABLE IF NOT EXISTS relationships (
               id TEXT PRIMARY KEY, source_id TEXT NOT NULL REFERENCES entities(id),
               target_id TEXT NOT NULL REFERENCES entities(id), relationship_type TEXT NOT NULL,
               metadata TEXT NOT NULL DEFAULT '{}'
             );
             CREATE INDEX IF NOT EXISTS entities_name_idx ON entities(name);
             CREATE INDEX IF NOT EXISTS documents_entity_updated_idx ON documents(entity_id,updated_at DESC);
             CREATE INDEX IF NOT EXISTS relationships_source_idx ON relationships(source_id);
             CREATE INDEX IF NOT EXISTS relationships_target_idx ON relationships(target_id);"
        )?;
        self.connection.execute_batch("CREATE TABLE IF NOT EXISTS module_versions(module_id TEXT PRIMARY KEY, version INTEGER NOT NULL DEFAULT 0);
              CREATE TABLE IF NOT EXISTS module_state(module_id TEXT PRIMARY KEY, enabled INTEGER NOT NULL DEFAULT 1);
              CREATE TABLE IF NOT EXISTS module_package_versions(module_id TEXT PRIMARY KEY, package_version TEXT NOT NULL);
              CREATE TABLE IF NOT EXISTS module_namespaces(module_id TEXT NOT NULL, namespace TEXT NOT NULL, PRIMARY KEY(module_id, namespace));
             CREATE TABLE IF NOT EXISTS module_fields(module_id TEXT NOT NULL, namespace TEXT NOT NULL, key TEXT NOT NULL, field_type TEXT NOT NULL, required INTEGER NOT NULL, PRIMARY KEY(module_id, namespace, key));
              CREATE TABLE IF NOT EXISTS module_records(id TEXT PRIMARY KEY, module_id TEXT NOT NULL, collection TEXT NOT NULL, owner_entity_id TEXT NOT NULL REFERENCES entities(id), value TEXT NOT NULL, created_at TEXT NOT NULL, updated_at TEXT NOT NULL, UNIQUE(module_id, collection, id));
              CREATE INDEX IF NOT EXISTS module_records_owner_idx ON module_records(module_id, collection, owner_entity_id, id);
              CREATE TABLE IF NOT EXISTS entity_fields(entity_id TEXT NOT NULL REFERENCES entities(id), namespace TEXT NOT NULL, key TEXT NOT NULL, value TEXT NOT NULL, PRIMARY KEY(entity_id, namespace, key));
             CREATE TABLE IF NOT EXISTS assets (id TEXT PRIMARY KEY, entity_id TEXT NOT NULL REFERENCES entities(id), namespace TEXT NOT NULL, filename TEXT NOT NULL, content_hash TEXT NOT NULL, size INTEGER NOT NULL, mime_type TEXT NOT NULL, path TEXT NOT NULL, created_at TEXT NOT NULL);
             CREATE TABLE IF NOT EXISTS map_projection (map_entity_id TEXT PRIMARY KEY, provider TEXT NOT NULL, source_asset_id TEXT NOT NULL, source_path TEXT, source_hash TEXT);
             CREATE TABLE IF NOT EXISTS map_location_projection (location_id TEXT PRIMARY KEY, entity_id TEXT NOT NULL, map_entity_id TEXT NOT NULL, label TEXT, role TEXT NOT NULL, anchor_kind TEXT NOT NULL, provider TEXT, feature_kind TEXT, feature_id TEXT, min_x REAL, min_y REAL, max_x REAL, max_y REAL, valid_from TEXT, valid_to TEXT, resolution TEXT NOT NULL);
             CREATE INDEX IF NOT EXISTS map_location_entity_idx ON map_location_projection(entity_id);
             CREATE INDEX IF NOT EXISTS map_location_map_idx ON map_location_projection(map_entity_id);
             CREATE INDEX IF NOT EXISTS assets_entity_created_idx ON assets(entity_id,created_at);")?;
        self.ensure_map_location_projection_schema()?;
        self.connection.execute_batch("CREATE TABLE IF NOT EXISTS migration_history(module_id TEXT NOT NULL, migration_id TEXT NOT NULL, from_version INTEGER NOT NULL, to_version INTEGER NOT NULL, checksum TEXT NOT NULL, package_digest TEXT NOT NULL DEFAULT '', applied_at TEXT NOT NULL DEFAULT '', PRIMARY KEY(module_id, migration_id)); CREATE TABLE IF NOT EXISTS plugin_backups(id TEXT PRIMARY KEY, module_id TEXT NOT NULL, from_package_version TEXT, to_package_version TEXT, data_version INTEGER NOT NULL, path TEXT NOT NULL, content_hash TEXT NOT NULL, created_at TEXT NOT NULL); CREATE TABLE IF NOT EXISTS module_schema_overlays(module_id TEXT PRIMARY KEY, overlay_json TEXT NOT NULL);")?;
        for table in [
            "project_meta",
            "entities",
            "documents",
            "relationships",
            "entity_fields",
            "assets",
            "module_versions",
            "module_state",
            "module_package_versions",
            "module_namespaces",
            "module_fields",
            "module_records",
            "module_schema_overlays",
            "migration_history",
            "plugin_backups",
        ] {
            for event in ["INSERT", "UPDATE", "DELETE"] {
                self.connection.execute_batch(&format!(
                    "CREATE TRIGGER IF NOT EXISTS runtime_content_generation_{event}_{table} AFTER {event} ON {table} BEGIN UPDATE runtime_meta SET content_generation=content_generation+1 WHERE key='runtime'; END;"
                ))?;
            }
        }
        if self
            .connection
            .query_row("SELECT 1 FROM runtime_meta WHERE key='runtime'", [], |_| {
                Ok(())
            })
            .is_err()
        {
            let project_id = self
                .root
                .as_deref()
                .map(|root| {
                    crate::storage::read_json::<crate::storage::ProjectManifest>(
                        &root.join("project.json"),
                    )
                    .map(|manifest| manifest.id)
                })
                .transpose()?
                .unwrap_or_default();
            self.connection.execute(
                "INSERT INTO runtime_meta(key,storage_role,schema_version,project_id,portable_format_version,database_epoch,exporter_version,content_generation,exported_generation) VALUES ('runtime',?1,?2,?3,?4,?5,?6,0,0)",
                params![
                    RUNTIME_STORAGE_ROLE,
                    RUNTIME_SCHEMA_VERSION,
                    project_id,
                    crate::storage::PROJECT_FORMAT_VERSION as i64,
                    Uuid::new_v4().to_string(),
                    EXPORTER_CONTRACT_VERSION,
                ],
            )?;
        }
        if rebuild_derived {
            self.rebuild_search()?;
        } else {
            self.ensure_search_projection()?;
        }
        Ok(())
    }

    fn ensure_query_indexes(&self) -> Result<(), CoreError> {
        self.connection.execute_batch(
            "CREATE INDEX IF NOT EXISTS documents_entity_updated_idx ON documents(entity_id,updated_at DESC);
             CREATE INDEX IF NOT EXISTS assets_entity_created_idx ON assets(entity_id,created_at);",
        )?;
        Ok(())
    }

    fn ensure_search_projection(&self) -> Result<(), CoreError> {
        let search_table_exists: bool = self.connection.query_row(
            "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name='world_search')",
            [],
            |row| row.get(0),
        )?;
        let search_shape_current = search_table_exists
            && self
                .connection
                .prepare("SELECT source_key FROM world_search LIMIT 0")
                .is_ok();
        let search_missing = !search_shape_current
            || self.connection.query_row(
                "SELECT EXISTS(SELECT 1 FROM entities WHERE deleted=0) AND NOT EXISTS(SELECT 1 FROM world_search)",
                [],
                |row| row.get(0),
            )?;
        let record_search_table_exists: bool = self.connection.query_row(
            "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name='module_record_search')",
            [],
            |row| row.get(0),
        )?;
        let record_search_missing = !record_search_table_exists
            || self.connection.query_row(
                "SELECT EXISTS(SELECT 1 FROM module_records) AND NOT EXISTS(SELECT 1 FROM module_record_search)",
                [],
                |row| row.get(0),
            )?;
        if search_missing || record_search_missing {
            self.rebuild_search()?;
        }
        Ok(())
    }

    pub fn create_entity(&self, input: CreateEntity) -> Result<Entity, CoreError> {
        self.create_entity_with_request(input, None)
    }

    pub fn create_entity_with_request(
        &self,
        input: CreateEntity,
        request_id: Option<&str>,
    ) -> Result<Entity, CoreError> {
        let input_fingerprint = digest_bytes(
            &serde_json::to_vec(&input)
                .map_err(|error| CoreError::Serialization(error.to_string()))?,
        );
        if let Some(mut entity) = self
            .committed_mutation_with_fingerprint::<Entity>(request_id, Some(&input_fingerprint))?
        {
            entity.revision = self.revision_for_entity(&entity.id)?;
            return Ok(entity);
        }
        if input.name.trim().is_empty() {
            return Err(CoreError::NotFound("entity name cannot be empty".into()));
        }
        let entity_type = input.entity_type.map(|value| value.trim().to_owned());
        let id = Uuid::new_v4().to_string();
        let now = chrono_like_now();
        let request_id = self.request_id(request_id)?;
        let result = serde_json::to_value(&Entity {
            id: id.clone(),
            name: input.name.trim().into(),
            entity_type: entity_type.clone(),
            deleted: false,
            created_at: now.clone(),
            updated_at: now.clone(),
            revision: String::new(),
        })
        .map_err(|error| CoreError::Serialization(error.to_string()))?;
        let transaction = self.begin_mutation_with_fingerprint(
            &request_id,
            Some(&result),
            &[format!("entities/{id}/")],
            &input_fingerprint,
        )?;
        transaction.execute(
            "INSERT INTO entities(id,name,entity_type,created_at,updated_at) VALUES (?1,?2,?3,?4,?4)",
            params![id, input.name.trim(), entity_type, now],
        )?;
        transaction.commit()?;
        self.notify_export_worker()?;
        let revision = self.revision_for_entity(&id)?;
        Ok(Entity {
            id,
            name: input.name.trim().into(),
            entity_type,
            deleted: false,
            created_at: now.clone(),
            updated_at: now,
            revision,
        })
    }

    pub fn create_entry(&self, input: CreateEntry) -> Result<Entity, CoreError> {
        self.create_entry_with_request(input, None)
    }

    pub fn create_entry_with_request(
        &self,
        input: CreateEntry,
        request_id: Option<&str>,
    ) -> Result<Entity, CoreError> {
        validate_document_format(
            input
                .document
                .as_ref()
                .and_then(|document| document.format.as_deref()),
            self.root.is_some(),
        )?;
        if let Some(mut entity) = self.committed_mutation::<Entity>(request_id)? {
            entity.revision = self.revision_for_entity(&entity.id)?;
            return Ok(entity);
        }
        if input.name.trim().is_empty() {
            return Err(CoreError::NotFound("entity name cannot be empty".into()));
        }
        let format = input
            .document
            .as_ref()
            .and_then(|document| document.format.as_deref())
            .unwrap_or("markdown")
            .to_owned();
        validate_document_format(Some(&format), false)?;
        let encoded_fields = input
            .fields
            .iter()
            .map(|field| {
                if field.namespace.trim().is_empty() || field.key.trim().is_empty() {
                    return Err(CoreError::NotFound(
                        "field namespace and key are required".into(),
                    ));
                }
                Ok((field, encode_field_value(&field.value)?))
            })
            .collect::<Result<Vec<_>, CoreError>>()?;
        let entity_type = input.entity_type.map(|value| value.trim().to_owned());
        let id = Uuid::new_v4().to_string();
        let now = chrono_like_now();
        let mut relationship_rows = Vec::new();
        for relationship in &input.relationships {
            if relationship.relationship_type.trim().is_empty() {
                return Err(CoreError::Validation(
                    "relationship type cannot be empty".into(),
                ));
            }
            let mut target_ids = BTreeSet::new();
            for target_id in &relationship.target_ids {
                if !target_ids.insert(target_id) {
                    return Err(CoreError::Validation(
                        "duplicate relationship target".into(),
                    ));
                }
                let exists: Option<String> = self
                    .connection
                    .query_row(
                        "SELECT id FROM entities WHERE id=?1 AND deleted=0",
                        params![target_id],
                        |row| row.get(0),
                    )
                    .optional()?;
                if exists.is_none() {
                    return Err(CoreError::NotFound("relationship entity not found".into()));
                }
                relationship_rows.push((
                    target_id.clone(),
                    relationship.relationship_type.trim().to_owned(),
                ));
            }
        }
        let request_id = self.request_id(request_id)?;
        let result = serde_json::to_value(&Entity {
            id: id.clone(),
            name: input.name.trim().into(),
            entity_type: entity_type.clone(),
            deleted: false,
            created_at: now.clone(),
            updated_at: now.clone(),
            revision: String::new(),
        })?;
        let transaction =
            self.begin_mutation(&request_id, Some(&result), &[format!("entities/{id}/")])?;
        transaction.execute(
            "INSERT INTO entities(id,name,entity_type,created_at,updated_at) VALUES (?1,?2,?3,?4,?4)",
            params![id, input.name.trim(), entity_type, now],
        )?;
        if let Some(document) = input.document {
            transaction.execute(
                "INSERT INTO documents(id,entity_id,format,body,updated_at) VALUES (?1,?2,?3,?4,?5)",
                params![Uuid::new_v4().to_string(), id, format, document.body, now],
            )?;
        }
        for (field, value) in encoded_fields {
            if field.namespace == crate::maps::MAP_NAMESPACE {
                crate::maps::validate_field(&transaction, &id, &field.key, &field.value)?;
            }
            transaction.execute(
                "INSERT INTO entity_fields(entity_id,namespace,key,value) VALUES (?1,?2,?3,?4)",
                params![id, field.namespace, field.key, value],
            )?;
        }
        for (target_id, relationship_type) in relationship_rows {
            transaction.execute(
                "INSERT INTO relationships(id,source_id,target_id,relationship_type,metadata) VALUES (?1,?2,?3,?4,?5)",
                params![Uuid::new_v4().to_string(), id, target_id, relationship_type, "{}"],
            )?;
        }
        transaction.commit()?;
        self.notify_export_worker()?;
        let revision = self.revision_for_entity(&id)?;
        Ok(Entity {
            id,
            name: input.name.trim().into(),
            entity_type,
            deleted: false,
            created_at: now.clone(),
            updated_at: now,
            revision,
        })
    }

    pub fn list_entities(&self) -> Result<Vec<Entity>, CoreError> {
        self.list_entities_where("WHERE deleted=0")
    }

    fn list_entities_where(&self, predicate: &str) -> Result<Vec<Entity>, CoreError> {
        let mut statement = self.connection.prepare(&format!("SELECT id,name,entity_type,deleted,created_at,updated_at FROM entities {predicate} ORDER BY name"))?;
        let rows = statement.query_map([], |row| {
            Ok(Entity {
                id: row.get(0)?,
                name: row.get(1)?,
                entity_type: row.get(2)?,
                deleted: row.get::<_, i64>(3)? != 0,
                created_at: row.get(4)?,
                updated_at: row.get(5)?,
                revision: String::new(),
            })
        })?;
        let mut entities = rows.collect::<Result<Vec<_>, _>>()?;
        self.populate_entity_revisions(&mut entities)?;
        Ok(entities)
    }

    pub fn update_entity(
        &self,
        id: String,
        name: Option<String>,
        entity_type: Option<String>,
    ) -> Result<Entity, CoreError> {
        self.update_entity_with_options(id, name, entity_type, None, None)
    }

    pub fn update_entity_with_options(
        &self,
        id: String,
        name: Option<String>,
        entity_type: Option<String>,
        expected_revision: Option<&str>,
        request_id: Option<&str>,
    ) -> Result<Entity, CoreError> {
        if let Some(mut entity) = self.committed_mutation::<Entity>(request_id)? {
            entity.revision = self.revision_for_entity(&entity.id)?;
            return Ok(entity);
        }
        Self::ensure_expected_revision(
            expected_revision,
            self.revision_for_entity(&id)?,
            "entity",
        )?;
        if let Some(value) = &name {
            if value.trim().is_empty() {
                return Err(CoreError::NotFound("entity name cannot be empty".into()));
            }
        }
        let current = self
            .connection
            .query_row(
                "SELECT name,entity_type,created_at FROM entities WHERE id=?1 AND deleted=0",
                params![id],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, Option<String>>(1)?,
                        row.get::<_, String>(2)?,
                    ))
                },
            )
            .optional()?
            .ok_or_else(|| CoreError::NotFound("entity not found".into()))?;
        let now = chrono_like_now();
        let request_id = self.request_id(request_id)?;
        let result = serde_json::to_value(&Entity {
            id: id.clone(),
            name: name.as_deref().map(str::trim).unwrap_or(&current.0).into(),
            entity_type: entity_type.clone().or(current.1.clone()),
            deleted: false,
            created_at: current.2.clone(),
            updated_at: now.clone(),
            revision: String::new(),
        })?;
        let transaction =
            self.begin_mutation(&request_id, Some(&result), &[format!("entities/{id}/")])?;
        if transaction.execute("UPDATE entities SET name=COALESCE(?2,name), entity_type=COALESCE(?3,entity_type), updated_at=?4 WHERE id=?1 AND deleted=0", params![id, name, entity_type, now])? == 0 { return Err(CoreError::NotFound("entity not found".into())); }
        transaction.commit()?;
        let mut entity = self.connection.query_row(
            "SELECT id,name,entity_type,deleted,created_at,updated_at FROM entities WHERE id=?1",
            params![id],
            |row| {
                Ok(Entity {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    entity_type: row.get(2)?,
                    deleted: row.get::<_, i64>(3)? != 0,
                    created_at: row.get(4)?,
                    updated_at: row.get(5)?,
                    revision: String::new(),
                })
            },
        )?;
        self.notify_export_worker()?;
        entity.revision = self.revision_for_entity(&entity.id)?;
        Ok(entity)
    }

    /// Creates the minimum canonical shape required before the map provider opens.
    /// No source asset exists until the first save; the descriptor carries a null
    /// sourceAssetId and the provider generates its first map on load.
    pub fn create_map(&self, name: String) -> Result<Entity, CoreError> {
        let entity = self.create_entity(CreateEntity {
            name,
            entity_type: Some(crate::maps::MAP_ENTITY_TYPE.into()),
        })?;
        self.set_field(FieldValue {
            entity_id: entity.id.clone(),
            namespace: crate::maps::MAP_NAMESPACE.into(),
            key: "map".into(),
            value: serde_json::json!({
                "schemaVersion": 1,
                "provider": {"id": "azgaar-fmg", "adapterVersion": 1, "sourceFormat": "fmg-map"},
                "sourceAssetId": null,
                "previewAssetId": null,
                "defaultView": {"center": [0.5, 0.5], "zoom": 1}
            }),
            revision: String::new(),
        })?;
        Ok(entity)
    }

    pub fn import_image_map(
        &self,
        name: String,
        bytes: Vec<u8>,
        mime_type: String,
        filename: String,
        request_id: Option<&str>,
    ) -> Result<ImportedImageMap, CoreError> {
        let input_fingerprint = digest_bytes(
            &serde_json::to_vec(&serde_json::json!({
                "name": name,
                "mimeType": mime_type,
                "filename": filename,
                "contentHash": crate::maps::image::content_hash(&bytes),
            }))
            .map_err(|error| CoreError::Serialization(error.to_string()))?,
        );
        if let Some(imported) = self
            .committed_mutation_with_fingerprint::<ImportedImageMap>(
                request_id,
                Some(&input_fingerprint),
            )?
        {
            return Ok(imported);
        }
        let source = crate::maps::validate_image_source(&bytes, &mime_type)?;
        let filename = Path::new(&filename)
            .file_name()
            .and_then(|value| value.to_str())
            .filter(|value| !value.is_empty() && *value != "." && *value != "..")
            .unwrap_or(match source.source_format {
                "jpeg" => "map.jpeg",
                "svg" => "map.svg",
                _ => "map.png",
            })
            .to_owned();
        let content_hash = crate::maps::image::content_hash(&bytes);
        let size = bytes.len() as i64;
        if let Some(root) = self.root.as_deref() {
            store_runtime_asset(root, bytes.as_slice(), Some(&content_hash))?;
        }
        let entity_id = Uuid::new_v4().to_string();
        let asset_id = Uuid::new_v4().to_string();
        let now = chrono_like_now();
        let relative_path = format!("assets/maps/{}-{filename}", Uuid::new_v4());
        let descriptor = serde_json::json!({
            "schemaVersion": 1,
            "provider": {
                "id": crate::maps::IMAGE_PROVIDER,
                "adapterVersion": 1,
                "sourceFormat": source.source_format
            },
            "sourceAssetId": asset_id,
            "previewAssetId": null,
            "defaultView": {"center": [0.5, 0.5], "zoom": 1}
        });
        let layers = serde_json::json!({"schemaVersion": 1, "layers": []});
        let entity = Entity {
            id: entity_id.clone(),
            name: name.trim().into(),
            entity_type: Some(crate::maps::MAP_ENTITY_TYPE.into()),
            deleted: false,
            created_at: now.clone(),
            updated_at: now.clone(),
            revision: String::new(),
        };
        let asset = Asset {
            id: asset_id.clone(),
            entity_id: entity_id.clone(),
            namespace: crate::maps::MAP_NAMESPACE.into(),
            filename: filename.clone(),
            content_hash: content_hash.clone(),
            size,
            mime_type: mime_type.clone(),
            path: relative_path.clone(),
            created_at: now.clone(),
            revision: String::new(),
        };
        let imported = ImportedImageMap {
            entity: entity.clone(),
            source: asset.clone(),
        };
        let request_id = self.request_id(request_id)?;
        let result = serde_json::to_value(&imported)
            .map_err(|error| CoreError::Serialization(error.to_string()))?;
        let transaction = self.begin_mutation_with_fingerprint(
            &request_id,
            Some(&result),
            &[format!("entities/{entity_id}/"), relative_path.clone()],
            &input_fingerprint,
        )?;
        transaction.execute(
            "INSERT INTO entities(id,name,entity_type,created_at,updated_at) VALUES (?1,?2,?3,?4,?4)",
            params![entity_id, entity.name, crate::maps::MAP_ENTITY_TYPE, now],
        )?;
        transaction.execute(
            "INSERT INTO assets(id,entity_id,namespace,filename,content_hash,size,mime_type,path,created_at) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9)",
            params![asset_id, entity_id, crate::maps::MAP_NAMESPACE, filename, content_hash, size, mime_type, relative_path, now],
        )?;
        crate::maps::validate_field(&transaction, &entity_id, "map", &descriptor)?;
        crate::maps::validate_field(&transaction, &entity_id, "layers", &layers)?;
        transaction.execute(
            "INSERT INTO entity_fields(entity_id,namespace,key,value) VALUES (?1,?2,?3,?4)",
            params![entity_id, crate::maps::MAP_NAMESPACE, "map", encode_field_value(&descriptor)?],
        )?;
        transaction.execute(
            "INSERT INTO entity_fields(entity_id,namespace,key,value) VALUES (?1,?2,?3,?4)",
            params![entity_id, crate::maps::MAP_NAMESPACE, "layers", encode_field_value(&layers)?],
        )?;
        transaction.commit()?;
        self.refresh_maps_projection_for_entities(std::slice::from_ref(&imported.entity.id))?;
        self.notify_export_worker()?;
        let mut imported = imported;
        imported.entity.revision = self.revision_for_entity(&imported.entity.id)?;
        imported.source.revision = self.revision_for_asset(&imported.source.id)?;
        self.write_mutation_result(
            &request_id,
            &serde_json::to_value(&imported)
                .map_err(|error| CoreError::Serialization(error.to_string()))?,
        )?;
        Ok(imported)
    }

    pub fn replay_imported_image_map(
        &self,
        request_id: Option<&str>,
    ) -> Result<Option<ImportedImageMap>, CoreError> {
        self.committed_mutation(request_id)
    }

    pub fn create_raster_layer(
        &self,
        map_entity_id: String,
        name: String,
        expected_revision: &str,
        request_id: Option<&str>,
    ) -> Result<RasterLayerChange, CoreError> {
        let input_fingerprint = self.layer_mutation_fingerprint(
            "create",
            &map_entity_id,
            None,
            Some(&name),
            expected_revision,
            None,
        )?;
        if let Some(change) = self.committed_mutation_with_fingerprint::<RasterLayerChange>(
            request_id,
            Some(&input_fingerprint),
        )? {
            return Ok(change);
        }
        let (width, height) = self.map_source_dimensions(&map_entity_id)?;
        let mut layers_field = self.layers_field(&map_entity_id)?;
        Self::ensure_expected_revision(
            Some(expected_revision),
            layers_field.revision.clone(),
            "field",
        )?;
        let layers = layers_field
            .value
            .get("layers")
            .and_then(serde_json::Value::as_array)
            .cloned()
            .unwrap_or_default();
        let raster_count = layers
            .iter()
            .filter(|layer| layer.get("kind").and_then(serde_json::Value::as_str) == Some("raster"))
            .count();
        if raster_count >= crate::maps::IMAGE_MAX_RASTER_LAYERS {
            return Err(CoreError::Validation(format!(
                "maps: raster layer count exceeds the budget of {}",
                crate::maps::IMAGE_MAX_RASTER_LAYERS
            )));
        }
        let png = crate::maps::encode_transparent_png(width, height)?;
        let content_hash = crate::maps::image::content_hash(&png);
        let size = png.len() as i64;
        if let Some(root) = self.root.as_deref() {
            store_runtime_asset(root, png.as_slice(), Some(&content_hash))?;
        }
        let layer_id = Uuid::new_v4().to_string();
        let asset_id = Uuid::new_v4().to_string();
        let now = chrono_like_now();
        let relative_path = format!("assets/maps/{asset_id}-layer.png");
        let next_order = layers
            .iter()
            .filter_map(|layer| layer.get("order").and_then(serde_json::Value::as_i64))
            .max()
            .map(|order| order + 1)
            .unwrap_or(0);
        let mut layers = layers;
        layers.push(serde_json::json!({
            "id": layer_id,
            "name": name,
            "order": next_order,
            "defaultVisible": true,
            "style": {},
            "selector": {},
            "kind": "raster",
            "rasterAssetId": asset_id,
            "opacity": 1.0,
            "locked": false
        }));
        let layers_value = serde_json::json!({"schemaVersion": 1, "layers": layers});
        let asset = Asset {
            id: asset_id.clone(),
            entity_id: map_entity_id.clone(),
            namespace: crate::maps::MAP_NAMESPACE.into(),
            filename: "layer.png".into(),
            content_hash: content_hash.clone(),
            size,
            mime_type: "image/png".into(),
            path: relative_path.clone(),
            created_at: now.clone(),
            revision: String::new(),
        };
        let request_id = self.request_id(request_id)?;
        let result = serde_json::to_value(&RasterLayerChange {
            layer_id: layer_id.clone(),
            asset: Some(asset.clone()),
            layers: FieldValue {
                entity_id: map_entity_id.clone(),
                namespace: crate::maps::MAP_NAMESPACE.into(),
                key: "layers".into(),
                value: layers_value.clone(),
                revision: String::new(),
            },
        })
        .map_err(|error| CoreError::Serialization(error.to_string()))?;
        let transaction = self.begin_mutation_with_fingerprint(
            &request_id,
            Some(&result),
            &[format!("entities/{map_entity_id}/"), relative_path.clone()],
            &input_fingerprint,
        )?;
        transaction.execute(
            "INSERT INTO assets(id,entity_id,namespace,filename,content_hash,size,mime_type,path,created_at) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9)",
            params![asset_id, map_entity_id, crate::maps::MAP_NAMESPACE, "layer.png", content_hash, size, "image/png", relative_path, now],
        )?;
        crate::maps::validate_field(&transaction, &map_entity_id, "layers", &layers_value)?;
        transaction.execute(
            "INSERT INTO entity_fields(entity_id,namespace,key,value) VALUES (?1,?2,?3,?4) ON CONFLICT(entity_id,namespace,key) DO UPDATE SET value=excluded.value",
            params![map_entity_id, crate::maps::MAP_NAMESPACE, "layers", encode_field_value(&layers_value)?],
        )?;
        transaction.commit()?;
        self.refresh_maps_projection_for_entities(std::slice::from_ref(&map_entity_id))?;
        self.notify_export_worker()?;
        layers_field.value = layers_value;
        layers_field.revision = self.revision_for_field(&layers_field)?;
        let mut asset = asset;
        asset.revision = self.revision_for_asset(&asset.id)?;
        let change = RasterLayerChange {
            layer_id,
            asset: Some(asset),
            layers: layers_field,
        };
        self.write_mutation_result(
            &request_id,
            &serde_json::to_value(&change)
                .map_err(|error| CoreError::Serialization(error.to_string()))?,
        )?;
        Ok(change)
    }

    pub fn create_semantic_layer(
        &self,
        map_entity_id: String,
        name: String,
        expected_revision: &str,
        request_id: Option<&str>,
        style: Option<serde_json::Value>,
        selector: Option<serde_json::Value>,
    ) -> Result<RasterLayerChange, CoreError> {
        let input_fingerprint = self.layer_mutation_fingerprint(
            "create-semantic",
            &map_entity_id,
            None,
            Some(&name),
            expected_revision,
            None,
        )?;
        if let Some(change) = self.committed_mutation_with_fingerprint::<RasterLayerChange>(
            request_id,
            Some(&input_fingerprint),
        )? {
            return Ok(change);
        }
        let mut layers_field = self.layers_field(&map_entity_id)?;
        Self::ensure_expected_revision(
            Some(expected_revision),
            layers_field.revision.clone(),
            "field",
        )?;
        let layers = layers_field
            .value
            .get("layers")
            .and_then(serde_json::Value::as_array)
            .cloned()
            .unwrap_or_default();
        let semantic_count = layers
            .iter()
            .filter(|layer| layer.get("kind").and_then(serde_json::Value::as_str) != Some("raster"))
            .count();
        if semantic_count >= crate::maps::IMAGE_MAX_SEMANTIC_LAYERS {
            return Err(CoreError::Validation(format!(
                "maps: semantic layer count exceeds the budget of {}",
                crate::maps::IMAGE_MAX_SEMANTIC_LAYERS
            )));
        }
        let layer_id = Uuid::new_v4().to_string();
        let next_order = layers
            .iter()
            .filter_map(|layer| layer.get("order").and_then(serde_json::Value::as_i64))
            .max()
            .map(|order| order + 1)
            .unwrap_or(0);
        let mut layers = layers;
        layers.push(serde_json::json!({
            "id": layer_id,
            "name": name,
            "order": next_order,
            "defaultVisible": true,
            "style": style.unwrap_or_else(|| serde_json::json!({
                "stroke": "#d5ab6c",
                "fill": "rgba(213,171,108,0.25)",
                "strokeWidth": 2
            })),
            "selector": selector.unwrap_or_else(|| serde_json::json!({})),
            "kind": "semantic"
        }));
        let layers_value = serde_json::json!({"schemaVersion": 1, "layers": layers});
        let request_id = self.request_id(request_id)?;
        let result = serde_json::to_value(&RasterLayerChange {
            layer_id: layer_id.clone(),
            asset: None,
            layers: FieldValue {
                entity_id: map_entity_id.clone(),
                namespace: crate::maps::MAP_NAMESPACE.into(),
                key: "layers".into(),
                value: layers_value.clone(),
                revision: String::new(),
            },
        })
        .map_err(|error| CoreError::Serialization(error.to_string()))?;
        let transaction = self.begin_mutation_with_fingerprint(
            &request_id,
            Some(&result),
            &[format!("entities/{map_entity_id}/")],
            &input_fingerprint,
        )?;
        crate::maps::validate_field(&transaction, &map_entity_id, "layers", &layers_value)?;
        transaction.execute(
            "INSERT INTO entity_fields(entity_id,namespace,key,value) VALUES (?1,?2,?3,?4) ON CONFLICT(entity_id,namespace,key) DO UPDATE SET value=excluded.value",
            params![map_entity_id, crate::maps::MAP_NAMESPACE, "layers", encode_field_value(&layers_value)?],
        )?;
        transaction.commit()?;
        self.refresh_maps_projection_for_entities(std::slice::from_ref(&map_entity_id))?;
        self.notify_export_worker()?;
        layers_field.value = layers_value;
        layers_field.revision = self.revision_for_field(&layers_field)?;
        let change = RasterLayerChange {
            layer_id,
            asset: None,
            layers: layers_field,
        };
        self.write_mutation_result(
            &request_id,
            &serde_json::to_value(&change)
                .map_err(|error| CoreError::Serialization(error.to_string()))?,
        )?;
        Ok(change)
    }

    pub fn delete_semantic_layer(
        &self,
        map_entity_id: String,
        layer_id: String,
        expected_revision: &str,
        request_id: Option<&str>,
    ) -> Result<RasterLayerChange, CoreError> {
        let input_fingerprint = self.layer_mutation_fingerprint(
            "delete-semantic",
            &map_entity_id,
            Some(&layer_id),
            None,
            expected_revision,
            None,
        )?;
        if let Some(change) = self.committed_mutation_with_fingerprint::<RasterLayerChange>(
            request_id,
            Some(&input_fingerprint),
        )? {
            return Ok(change);
        }
        let mut layers_field = self.layers_field(&map_entity_id)?;
        Self::ensure_expected_revision(
            Some(expected_revision),
            layers_field.revision.clone(),
            "field",
        )?;
        let layers = layers_field
            .value
            .get("layers")
            .and_then(serde_json::Value::as_array)
            .cloned()
            .unwrap_or_default();
        let Some(removed) = layers.iter().find(|layer| {
            layer.get("id").and_then(serde_json::Value::as_str) == Some(layer_id.as_str())
        }) else {
            return Err(CoreError::NotFound("semantic layer not found".into()));
        };
        if removed.get("kind").and_then(serde_json::Value::as_str) == Some("raster") {
            return Err(CoreError::Validation(
                "maps: raster layers cannot be deleted by this operation".into(),
            ));
        }
        let remaining = layers
            .into_iter()
            .filter(|layer| layer.get("id").and_then(serde_json::Value::as_str) != Some(layer_id.as_str()))
            .collect::<Vec<_>>();
        let layers_value = serde_json::json!({"schemaVersion": 1, "layers": remaining});
        let request_id = self.request_id(request_id)?;
        let result = serde_json::to_value(&RasterLayerChange {
            layer_id: layer_id.clone(),
            asset: None,
            layers: FieldValue {
                entity_id: map_entity_id.clone(),
                namespace: crate::maps::MAP_NAMESPACE.into(),
                key: "layers".into(),
                value: layers_value.clone(),
                revision: String::new(),
            },
        })
        .map_err(|error| CoreError::Serialization(error.to_string()))?;
        let transaction = self.begin_mutation_with_fingerprint(
            &request_id,
            Some(&result),
            &[format!("entities/{map_entity_id}/")],
            &input_fingerprint,
        )?;
        crate::maps::validate_field(&transaction, &map_entity_id, "layers", &layers_value)?;
        transaction.execute(
            "INSERT INTO entity_fields(entity_id,namespace,key,value) VALUES (?1,?2,?3,?4) ON CONFLICT(entity_id,namespace,key) DO UPDATE SET value=excluded.value",
            params![map_entity_id, crate::maps::MAP_NAMESPACE, "layers", encode_field_value(&layers_value)?],
        )?;
        transaction.commit()?;
        self.refresh_maps_projection_for_entities(std::slice::from_ref(&map_entity_id))?;
        self.notify_export_worker()?;
        layers_field.value = layers_value;
        layers_field.revision = self.revision_for_field(&layers_field)?;
        let change = RasterLayerChange {
            layer_id,
            asset: None,
            layers: layers_field,
        };
        self.write_mutation_result(
            &request_id,
            &serde_json::to_value(&change)
                .map_err(|error| CoreError::Serialization(error.to_string()))?,
        )?;
        Ok(change)
    }

    pub fn delete_raster_layer(
        &self,
        map_entity_id: String,
        layer_id: String,
        expected_revision: &str,
        request_id: Option<&str>,
    ) -> Result<RasterLayerChange, CoreError> {
        let input_fingerprint = self.layer_mutation_fingerprint(
            "delete",
            &map_entity_id,
            Some(&layer_id),
            None,
            expected_revision,
            None,
        )?;
        if let Some(change) = self.committed_mutation_with_fingerprint::<RasterLayerChange>(
            request_id,
            Some(&input_fingerprint),
        )? {
            return Ok(change);
        }
        let mut layers_field = self.layers_field(&map_entity_id)?;
        Self::ensure_expected_revision(
            Some(expected_revision),
            layers_field.revision.clone(),
            "field",
        )?;
        let layers = layers_field
            .value
            .get("layers")
            .and_then(serde_json::Value::as_array)
            .cloned()
            .unwrap_or_default();
        let Some(removed) = layers.iter().find(|layer| {
            layer.get("id").and_then(serde_json::Value::as_str) == Some(layer_id.as_str())
        }) else {
            return Err(CoreError::NotFound("raster layer not found".into()));
        };
        if removed.get("kind").and_then(serde_json::Value::as_str) != Some("raster") {
            return Err(CoreError::Validation(
                "maps: only raster layers can be deleted by this operation".into(),
            ));
        }
        let raster_asset_id = removed
            .get("rasterAssetId")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| {
                CoreError::Validation("maps: raster layer is missing rasterAssetId".into())
            })?
            .to_owned();
        let remaining = layers
            .into_iter()
            .filter(|layer| layer.get("id").and_then(serde_json::Value::as_str) != Some(layer_id.as_str()))
            .collect::<Vec<_>>();
        let layers_value = serde_json::json!({"schemaVersion": 1, "layers": remaining});
        let request_id = self.request_id(request_id)?;
        let result = serde_json::to_value(&RasterLayerChange {
            layer_id: layer_id.clone(),
            asset: None,
            layers: FieldValue {
                entity_id: map_entity_id.clone(),
                namespace: crate::maps::MAP_NAMESPACE.into(),
                key: "layers".into(),
                value: layers_value.clone(),
                revision: String::new(),
            },
        })
        .map_err(|error| CoreError::Serialization(error.to_string()))?;
        let transaction = self.begin_mutation_with_fingerprint(
            &request_id,
            Some(&result),
            &[format!("entities/{map_entity_id}/")],
            &input_fingerprint,
        )?;
        transaction.execute(
            "DELETE FROM assets WHERE id=?1 AND entity_id=?2 AND namespace=?3",
            params![raster_asset_id, map_entity_id, crate::maps::MAP_NAMESPACE],
        )?;
        crate::maps::validate_field(&transaction, &map_entity_id, "layers", &layers_value)?;
        transaction.execute(
            "INSERT INTO entity_fields(entity_id,namespace,key,value) VALUES (?1,?2,?3,?4) ON CONFLICT(entity_id,namespace,key) DO UPDATE SET value=excluded.value",
            params![map_entity_id, crate::maps::MAP_NAMESPACE, "layers", encode_field_value(&layers_value)?],
        )?;
        transaction.commit()?;
        self.refresh_maps_projection_for_entities(std::slice::from_ref(&map_entity_id))?;
        self.notify_export_worker()?;
        layers_field.value = layers_value;
        layers_field.revision = self.revision_for_field(&layers_field)?;
        let change = RasterLayerChange {
            layer_id,
            asset: None,
            layers: layers_field,
        };
        self.write_mutation_result(
            &request_id,
            &serde_json::to_value(&change)
                .map_err(|error| CoreError::Serialization(error.to_string()))?,
        )?;
        Ok(change)
    }

    pub fn update_map_layer(
        &self,
        map_entity_id: String,
        layer_id: String,
        update: RasterLayerUpdate,
        expected_revision: &str,
        request_id: Option<&str>,
    ) -> Result<RasterLayerChange, CoreError> {
        let input_fingerprint = self.layer_mutation_fingerprint(
            "update",
            &map_entity_id,
            Some(&layer_id),
            None,
            expected_revision,
            Some(&update),
        )?;
        if let Some(change) = self.committed_mutation_with_fingerprint::<RasterLayerChange>(
            request_id,
            Some(&input_fingerprint),
        )? {
            return Ok(change);
        }
        let mut layers_field = self.layers_field(&map_entity_id)?;
        Self::ensure_expected_revision(
            Some(expected_revision),
            layers_field.revision.clone(),
            "field",
        )?;
        let mut layers = layers_field
            .value
            .get("layers")
            .and_then(serde_json::Value::as_array)
            .cloned()
            .unwrap_or_default();
        let Some(layer) = layers.iter_mut().find(|layer| {
            layer.get("id").and_then(serde_json::Value::as_str) == Some(layer_id.as_str())
        }) else {
            return Err(CoreError::NotFound("layer not found".into()));
        };
        let object = layer.as_object_mut().ok_or_else(|| {
            CoreError::Validation("maps: layer definition is invalid".into())
        })?;
        if let Some(name) = update.name {
            object.insert("name".into(), serde_json::Value::String(name));
        }
        if let Some(order) = update.order {
            object.insert("order".into(), serde_json::json!(order));
        }
        if let Some(default_visible) = update.default_visible {
            object.insert("defaultVisible".into(), serde_json::Value::Bool(default_visible));
        }
        if object.get("kind").and_then(serde_json::Value::as_str) == Some("raster") {
            if let Some(opacity) = update.opacity {
                object.insert("opacity".into(), serde_json::json!(opacity));
            }
            if let Some(locked) = update.locked {
                object.insert("locked".into(), serde_json::Value::Bool(locked));
            }
        } else if update.opacity.is_some() || update.locked.is_some() {
            return Err(CoreError::Validation(
                "maps: opacity and locked apply only to raster layers".into(),
            ));
        }
        if object.get("kind").and_then(serde_json::Value::as_str) == Some("raster") {
            if update.style.is_some() || update.selector.is_some() {
                return Err(CoreError::Validation(
                    "maps: raster layers must have empty style and selector objects".into(),
                ));
            }
        } else {
            if let Some(style) = update.style {
                object.insert("style".into(), style);
            }
            if let Some(selector) = update.selector {
                object.insert("selector".into(), selector);
            }
        }
        let layers_value = serde_json::json!({"schemaVersion": 1, "layers": layers});
        let request_id = self.request_id(request_id)?;
        let result = serde_json::to_value(&RasterLayerChange {
            layer_id: layer_id.clone(),
            asset: None,
            layers: FieldValue {
                entity_id: map_entity_id.clone(),
                namespace: crate::maps::MAP_NAMESPACE.into(),
                key: "layers".into(),
                value: layers_value.clone(),
                revision: String::new(),
            },
        })
        .map_err(|error| CoreError::Serialization(error.to_string()))?;
        crate::maps::validate_field(&self.connection, &map_entity_id, "layers", &layers_value)?;
        let transaction = self.begin_mutation_with_fingerprint(
            &request_id,
            Some(&result),
            &[format!("entities/{map_entity_id}/")],
            &input_fingerprint,
        )?;
        transaction.execute(
            "INSERT INTO entity_fields(entity_id,namespace,key,value) VALUES (?1,?2,?3,?4) ON CONFLICT(entity_id,namespace,key) DO UPDATE SET value=excluded.value",
            params![map_entity_id, crate::maps::MAP_NAMESPACE, "layers", encode_field_value(&layers_value)?],
        )?;
        transaction.commit()?;
        self.refresh_maps_projection_for_entities(std::slice::from_ref(&map_entity_id))?;
        self.notify_export_worker()?;
        layers_field.value = layers_value;
        layers_field.revision = self.revision_for_field(&layers_field)?;
        let change = RasterLayerChange {
            layer_id,
            asset: None,
            layers: layers_field,
        };
        self.write_mutation_result(
            &request_id,
            &serde_json::to_value(&change)
                .map_err(|error| CoreError::Serialization(error.to_string()))?,
        )?;
        Ok(change)
    }

    fn layer_mutation_fingerprint(
        &self,
        op: &str,
        map_entity_id: &str,
        layer_id: Option<&str>,
        name: Option<&str>,
        expected_revision: &str,
        update: Option<&RasterLayerUpdate>,
    ) -> Result<String, CoreError> {
        Ok(digest_bytes(
            &serde_json::to_vec(&serde_json::json!({
                "op": op,
                "mapEntityId": map_entity_id,
                "layerId": layer_id,
                "name": name,
                "expectedRevision": expected_revision,
                "update": update,
            }))
            .map_err(|error| CoreError::Serialization(error.to_string()))?,
        ))
    }

    fn layers_field(&self, map_entity_id: &str) -> Result<FieldValue, CoreError> {
        let entity_type: Option<String> = self
            .connection
            .query_row(
                "SELECT entity_type FROM entities WHERE id=?1 AND deleted=0",
                params![map_entity_id],
                |row| row.get(0),
            )
            .optional()?;
        if entity_type.as_deref() != Some(crate::maps::MAP_ENTITY_TYPE) {
            return Err(CoreError::NotFound("map entity not found".into()));
        }
        if let Some(mut field) = self
            .list_fields_unchecked(map_entity_id.to_owned())?
            .into_iter()
            .find(|field| field.namespace == crate::maps::MAP_NAMESPACE && field.key == "layers")
        {
            field.revision = self.revision_for_field(&field)?;
            return Ok(field);
        }
        let field = FieldValue {
            entity_id: map_entity_id.to_owned(),
            namespace: crate::maps::MAP_NAMESPACE.into(),
            key: "layers".into(),
            value: serde_json::json!({"schemaVersion": 1, "layers": []}),
            revision: String::new(),
        };
        let revision = self.revision_for_field(&field)?;
        Ok(FieldValue { revision, ..field })
    }

    fn ensure_map_location_projection_schema(&self) -> Result<(), CoreError> {
        let mut statement = self
            .connection
            .prepare("PRAGMA table_info(map_location_projection)")?;
        let columns = statement
            .query_map([], |row| row.get::<_, String>(1))?
            .collect::<Result<Vec<_>, _>>()?;
        if !columns.iter().any(|name| name == "geometry") {
            self.connection.execute(
                "ALTER TABLE map_location_projection ADD COLUMN geometry TEXT",
                [],
            )?;
        }
        self.connection.execute_batch(
            "CREATE INDEX IF NOT EXISTS map_location_bbox_idx ON map_location_projection(map_entity_id, min_x, max_x, min_y, max_y);",
        )?;
        Ok(())
    }

    fn map_source_dimensions(&self, map_entity_id: &str) -> Result<(u32, u32), CoreError> {
        let descriptor = self
            .list_fields_unchecked(map_entity_id.to_owned())?
            .into_iter()
            .find(|field| field.namespace == crate::maps::MAP_NAMESPACE && field.key == "map")
            .ok_or_else(|| CoreError::NotFound("map descriptor not found".into()))?;
        let source_asset_id = descriptor
            .value
            .pointer("/sourceAssetId")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| CoreError::Validation("maps: image maps require sourceAssetId".into()))?;
        let asset = self.asset_unchecked(source_asset_id)?;
        let bytes = self.read_asset_bytes(&asset)?;
        let source = crate::maps::validate_image_source(&bytes, &asset.mime_type)?;
        Ok((source.width, source.height))
    }

    fn read_asset_bytes(&self, asset: &Asset) -> Result<Vec<u8>, CoreError> {
        let root = self.project_root()?;
        let path = runtime_asset_path(root, &asset.content_hash)?;
        std::fs::read(&path).map_err(|source| CoreError::Io {
            operation: "read runtime asset",
            source,
        })
    }

    fn raster_layer_expected_size(&self, asset: &Asset) -> Result<Option<(u32, u32)>, CoreError> {
        let Some(layers) = self
            .list_fields_unchecked(asset.entity_id.clone())?
            .into_iter()
            .find(|field| field.namespace == crate::maps::MAP_NAMESPACE && field.key == "layers")
        else {
            return Ok(None);
        };
        let is_raster = layers
            .value
            .get("layers")
            .and_then(serde_json::Value::as_array)
            .into_iter()
            .flatten()
            .any(|layer| {
                layer.get("kind").and_then(serde_json::Value::as_str) == Some("raster")
                    && layer.get("rasterAssetId").and_then(serde_json::Value::as_str)
                        == Some(asset.id.as_str())
            });
        if !is_raster {
            return Ok(None);
        }
        Ok(Some(self.map_source_dimensions(&asset.entity_id)?))
    }

    fn is_immutable_image_source(&self, asset: &Asset) -> Result<bool, CoreError> {
        if asset.namespace != crate::maps::MAP_NAMESPACE {
            return Ok(false);
        }
        let Some(descriptor) = self
            .list_fields_unchecked(asset.entity_id.clone())?
            .into_iter()
            .find(|field| field.namespace == crate::maps::MAP_NAMESPACE && field.key == "map")
        else {
            return Ok(false);
        };
        Ok(
            descriptor.value.pointer("/provider/id").and_then(serde_json::Value::as_str)
                == Some(crate::maps::IMAGE_PROVIDER)
                && descriptor
                    .value
                    .pointer("/sourceAssetId")
                    .and_then(serde_json::Value::as_str)
                    == Some(asset.id.as_str()),
        )
    }

    pub fn delete_entity(&self, id: String) -> Result<(), CoreError> {
        self.delete_entity_with_options(id, None, None)
    }

    pub fn delete_entity_with_options(
        &self,
        id: String,
        expected_revision: Option<&str>,
        request_id: Option<&str>,
    ) -> Result<(), CoreError> {
        if self
            .committed_mutation::<serde_json::Value>(request_id)?
            .is_some()
        {
            return Ok(());
        }
        Self::ensure_expected_revision(
            expected_revision,
            self.revision_for_entity(&id)?,
            "entity",
        )?;
        let request_id = self.request_id(request_id)?;
        let transaction = self.begin_mutation(
            &request_id,
            Some(&serde_json::Value::Null),
            &[format!("entities/{id}/")],
        )?;
        if transaction.execute(
            "UPDATE entities SET deleted=1, updated_at=?2 WHERE id=?1 AND deleted=0",
            params![id, chrono_like_now()],
        )? == 0
        {
            return Err(CoreError::NotFound("entity not found".into()));
        }
        transaction.commit()?;
        self.refresh_maps_projection_for_entities(std::slice::from_ref(&id))?;
        self.notify_export_worker()?;
        Ok(())
    }

    pub fn save_document(&self, input: SaveDocument) -> Result<(), CoreError> {
        self.save_document_with_options(input, None, None)
    }

    pub fn save_document_with_options(
        &self,
        input: SaveDocument,
        expected_revision: Option<&str>,
        request_id: Option<&str>,
    ) -> Result<(), CoreError> {
        self.save_entry_with_options(
            SaveEntry {
                document: input,
                fields: Vec::new(),
            },
            expected_revision,
            request_id,
        )
    }

    pub fn save_entry(&self, input: SaveEntry) -> Result<(), CoreError> {
        self.save_entry_with_options(input, None, None)
    }

    pub fn save_entry_with_options(
        &self,
        input: SaveEntry,
        expected_revision: Option<&str>,
        request_id: Option<&str>,
    ) -> Result<(), CoreError> {
        validate_document_format(input.document.format.as_deref(), self.root.is_some())?;
        if self
            .committed_mutation::<serde_json::Value>(request_id)?
            .is_some()
        {
            return Ok(());
        }
        let document = input.document;
        let format = document.format.unwrap_or_else(|| "markdown".into());
        validate_document_format(Some(&format), false)?;
        let exists: Option<String> = self
            .connection
            .query_row(
                "SELECT id FROM entities WHERE id=?1 AND deleted=0",
                params![document.entity_id],
                |row| row.get(0),
            )
            .optional()?;
        if exists.is_none() {
            return Err(CoreError::NotFound("entity not found".into()));
        }
        let current_document_id: Option<String> = self
            .connection
            .query_row(
                "SELECT id FROM documents WHERE entity_id=?1 ORDER BY updated_at DESC LIMIT 1",
                params![document.entity_id],
                |row| row.get(0),
            )
            .optional()?;
        if let Some(expected_revision) = expected_revision {
            let Some(document_id) = current_document_id.as_deref() else {
                return Err(CoreError::Conflict(
                    "document revision conflict: document does not exist".into(),
                ));
            };
            Self::ensure_expected_revision(
                Some(expected_revision),
                self.revision_for_document(document_id)?,
                "document",
            )?;
        }
        let encoded_fields = input
            .fields
            .iter()
            .map(|field| {
                if field.entity_id != document.entity_id {
                    return Err(CoreError::NotFound(
                        "field entity does not match document entity".into(),
                    ));
                }
                if field.namespace.trim().is_empty() || field.key.trim().is_empty() {
                    return Err(CoreError::NotFound(
                        "field namespace and key are required".into(),
                    ));
                }
                Ok((field, encode_field_value(&field.value)?))
            })
            .collect::<Result<Vec<_>, CoreError>>()?;
        for (field, _) in &encoded_fields {
            if field.namespace == crate::maps::MAP_NAMESPACE {
                crate::maps::validate_field(
                    &self.connection,
                    &field.entity_id,
                    &field.key,
                    &field.value,
                )?;
            }
        }
        for (field, _) in &encoded_fields {
            if field.revision.is_empty() {
                continue;
            }
            let current: Option<String> = self
                .connection
                .query_row(
                    "SELECT value FROM entity_fields WHERE entity_id=?1 AND namespace=?2 AND key=?3",
                    params![field.entity_id, field.namespace, field.key],
                    |row| row.get(0),
                )
                .optional()?;
            let Some(current) = current else {
                return Err(CoreError::Conflict(
                    "field revision conflict: field does not exist".into(),
                ));
            };
            let current_field = FieldValue {
                entity_id: field.entity_id.clone(),
                namespace: field.namespace.clone(),
                key: field.key.clone(),
                value: decode_field_value(current),
                revision: String::new(),
            };
            Self::ensure_expected_revision(
                Some(&field.revision),
                self.revision_for_field(&current_field)?,
                "field",
            )?;
        }
        let now = chrono_like_now();
        let document_id: Option<String> = self
            .connection
            .query_row(
                "SELECT id FROM documents WHERE entity_id=?1 ORDER BY updated_at DESC LIMIT 1",
                params![document.entity_id],
                |row| row.get(0),
            )
            .optional()?;
        let request_id = self.request_id(request_id)?;
        let transaction = self.begin_mutation(
            &request_id,
            Some(&serde_json::Value::Null),
            &[format!("entities/{}/", document.entity_id)],
        )?;
        if let Some(document_id) = document_id {
            transaction.execute(
                "UPDATE documents SET format=?2, body=?3, updated_at=?4 WHERE id=?1",
                params![document_id, format, document.body, now],
            )?;
        } else {
            transaction.execute(
                "INSERT INTO documents(id,entity_id,format,body,updated_at) VALUES (?1,?2,?3,?4,?5)",
                params![Uuid::new_v4().to_string(), document.entity_id, format, document.body, now],
            )?;
        }
        for (field, value) in encoded_fields {
            transaction.execute("INSERT INTO entity_fields(entity_id,namespace,key,value) VALUES (?1,?2,?3,?4) ON CONFLICT(entity_id,namespace,key) DO UPDATE SET value=excluded.value", params![field.entity_id, field.namespace, field.key, value])?;
        }
        transaction.commit()?;
        self.notify_export_worker()?;
        Ok(())
    }

    pub fn list_documents(&self, entity_id: String) -> Result<Vec<Document>, CoreError> {
        self.list_documents_unchecked(entity_id)
    }

    fn list_documents_unchecked(&self, entity_id: String) -> Result<Vec<Document>, CoreError> {
        let mut statement = self.connection.prepare("SELECT id,entity_id,format,body,updated_at FROM documents WHERE entity_id=?1 ORDER BY updated_at DESC")?;
        let rows = statement.query_map(params![entity_id], |row| {
            Ok(Document {
                id: row.get(0)?,
                entity_id: row.get(1)?,
                format: row.get(2)?,
                body: row.get(3)?,
                updated_at: row.get(4)?,
                revision: String::new(),
            })
        })?;
        let mut documents = rows.collect::<Result<Vec<_>, _>>()?;
        for document in &mut documents {
            document.revision = self.revision_for_document_value(document)?;
        }
        Ok(documents)
    }
    pub fn set_field(&self, field: FieldValue) -> Result<(), CoreError> {
        self.set_field_with_request(field, None)
    }

    pub fn set_field_with_request(
        &self,
        field: FieldValue,
        request_id: Option<&str>,
    ) -> Result<(), CoreError> {
        if self
            .committed_mutation::<serde_json::Value>(request_id)?
            .is_some()
        {
            return Ok(());
        }
        let exists: Option<String> = self
            .connection
            .query_row(
                "SELECT id FROM entities WHERE id=?1 AND deleted=0",
                params![field.entity_id],
                |row| row.get(0),
            )
            .optional()?;
        if exists.is_none() {
            return Err(CoreError::NotFound("entity not found".into()));
        }
        if field.namespace.trim().is_empty() || field.key.trim().is_empty() {
            return Err(CoreError::NotFound(
                "field namespace and key are required".into(),
            ));
        }
        if field.namespace == crate::maps::MAP_NAMESPACE {
            crate::maps::validate_field(
                &self.connection,
                &field.entity_id,
                &field.key,
                &field.value,
            )?;
        }
        if !field.revision.is_empty() {
            let current: Option<String> = self
                .connection
                .query_row(
                    "SELECT value FROM entity_fields WHERE entity_id=?1 AND namespace=?2 AND key=?3",
                    params![field.entity_id, field.namespace, field.key],
                    |row| row.get(0),
                )
                .optional()?;
            let Some(current) = current else {
                return Err(CoreError::Conflict(
                    "field revision conflict: field does not exist".into(),
                ));
            };
            let current_field = FieldValue {
                entity_id: field.entity_id.clone(),
                namespace: field.namespace.clone(),
                key: field.key.clone(),
                value: decode_field_value(current),
                revision: String::new(),
            };
            Self::ensure_expected_revision(
                Some(&field.revision),
                self.revision_for_field(&current_field)?,
                "field",
            )?;
        }
        let value = encode_field_value(&field.value)?;
        let request_id = self.request_id(request_id)?;
        let transaction = self.begin_mutation(
            &request_id,
            Some(&serde_json::Value::Null),
            &[format!("entities/{}/", field.entity_id)],
        )?;
        transaction.execute("INSERT INTO entity_fields(entity_id,namespace,key,value) VALUES (?1,?2,?3,?4) ON CONFLICT(entity_id,namespace,key) DO UPDATE SET value=excluded.value", params![field.entity_id, field.namespace, field.key, value])?;
        transaction.commit()?;
        self.refresh_maps_projection_for_entities(std::slice::from_ref(&field.entity_id))?;
        self.notify_export_worker()?;
        Ok(())
    }
    pub fn list_fields(&self, entity_id: String) -> Result<Vec<FieldValue>, CoreError> {
        self.list_fields_unchecked(entity_id)
    }

    fn list_fields_unchecked(&self, entity_id: String) -> Result<Vec<FieldValue>, CoreError> {
        let mut s = self.connection.prepare(
            "SELECT entity_id,namespace,key,value FROM entity_fields WHERE entity_id=?1",
        )?;
        let rows = s.query_map(params![entity_id], |r| {
            let value: String = r.get(3)?;
            Ok(FieldValue {
                entity_id: r.get(0)?,
                namespace: r.get(1)?,
                key: r.get(2)?,
                value: decode_field_value(value),
                revision: String::new(),
            })
        })?;
        let mut fields = rows.collect::<Result<Vec<_>, _>>()?;
        for field in &mut fields {
            field.revision = self.revision_for_field(field)?;
        }
        Ok(fields)
    }

    pub fn create_relationship(&self, input: RelationshipInput) -> Result<Relationship, CoreError> {
        self.create_relationship_with_options(input, None, None)
    }

    pub fn create_relationship_with_request(
        &self,
        input: RelationshipInput,
        request_id: Option<&str>,
    ) -> Result<Relationship, CoreError> {
        self.create_relationship_with_options(input, None, request_id)
    }

    pub fn create_relationship_with_options(
        &self,
        input: RelationshipInput,
        expected_revision: Option<&str>,
        request_id: Option<&str>,
    ) -> Result<Relationship, CoreError> {
        if let Some(relationship) = self.committed_mutation::<Relationship>(request_id)? {
            let mut relationship = relationship;
            relationship.revision = self.revision_for_relationship_value(&relationship)?;
            return Ok(relationship);
        }
        for entity_id in [&input.source_id, &input.target_id] {
            let exists: Option<String> = self
                .connection
                .query_row(
                    "SELECT id FROM entities WHERE id=?1 AND deleted=0",
                    params![entity_id],
                    |row| row.get(0),
                )
                .optional()?;
            if exists.is_none() {
                return Err(CoreError::NotFound("relationship entity not found".into()));
            }
        }
        if input.relationship_type.trim().is_empty() {
            return Err(CoreError::NotFound(
                "relationship type cannot be empty".into(),
            ));
        }
        Self::ensure_expected_revision(
            expected_revision,
            self.revision_for_entity(&input.source_id)?,
            "relationship source entity",
        )?;
        let id = Uuid::new_v4().to_string();
        let metadata = input.metadata.unwrap_or_else(|| "{}".into());
        let request_id = self.request_id(request_id)?;
        let result = serde_json::to_value(&Relationship {
            id: id.clone(),
            source_id: input.source_id.clone(),
            target_id: input.target_id.clone(),
            relationship_type: input.relationship_type.clone(),
            metadata: metadata.clone(),
            revision: String::new(),
        })
        .map_err(|error| CoreError::Serialization(error.to_string()))?;
        let transaction = self.begin_mutation(
            &request_id,
            Some(&result),
            &[
                format!("entities/{}/", input.source_id),
                format!("entities/{}/", input.target_id),
            ],
        )?;
        transaction.execute("INSERT INTO relationships(id,source_id,target_id,relationship_type,metadata) VALUES (?1,?2,?3,?4,?5)", params![id, input.source_id, input.target_id, input.relationship_type, metadata])?;
        transaction.commit()?;
        self.notify_export_worker()?;
        let revision = self.revision_for_relationship(&id)?;
        Ok(Relationship {
            id,
            source_id: input.source_id,
            target_id: input.target_id,
            relationship_type: input.relationship_type,
            metadata,
            revision,
        })
    }

    pub fn delete_relationship(&self, id: String) -> Result<(), CoreError> {
        self.delete_relationship_with_options(id, None, None)
    }

    pub fn delete_relationship_with_options(
        &self,
        id: String,
        expected_revision: Option<&str>,
        request_id: Option<&str>,
    ) -> Result<(), CoreError> {
        if self
            .committed_mutation::<serde_json::Value>(request_id)?
            .is_some()
        {
            return Ok(());
        }
        Self::ensure_expected_revision(
            expected_revision,
            self.revision_for_relationship(&id)?,
            "relationship",
        )?;
        let (source_id, target_id): (String, String) = self.connection.query_row(
            "SELECT source_id,target_id FROM relationships WHERE id=?1",
            params![id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )?;
        let request_id = self.request_id(request_id)?;
        let transaction = self.begin_mutation(
            &request_id,
            Some(&serde_json::Value::Null),
            &[
                format!("entities/{source_id}/"),
                format!("entities/{target_id}/"),
            ],
        )?;
        if transaction.execute("DELETE FROM relationships WHERE id=?1", params![id])? == 0 {
            return Err(CoreError::NotFound("relationship not found".into()));
        }
        transaction.commit()?;
        self.notify_export_worker()?;
        Ok(())
    }

    pub fn relationship(&self, id: String) -> Result<Relationship, CoreError> {
        self.connection
            .query_row(
                "SELECT id,source_id,target_id,relationship_type,metadata FROM relationships WHERE id=?1",
                params![id],
                |row| {
                    Ok(Relationship {
                        id: row.get(0)?,
                        source_id: row.get(1)?,
                        target_id: row.get(2)?,
                        relationship_type: row.get(3)?,
                        metadata: row.get(4)?,
                        revision: String::new(),
                    })
                },
            )
            .optional()?
            .ok_or_else(|| CoreError::NotFound("relationship not found".into()))
            .and_then(|mut relationship| {
                relationship.revision = self.revision_for_relationship_value(&relationship)?;
                Ok(relationship)
            })
    }

    pub fn list_relationships(&self, entity_id: String) -> Result<Vec<Relationship>, CoreError> {
        self.list_relationships_unchecked(entity_id)
    }

    fn list_relationships_unchecked(
        &self,
        entity_id: String,
    ) -> Result<Vec<Relationship>, CoreError> {
        let mut statement = self.connection.prepare("SELECT id,source_id,target_id,relationship_type,metadata FROM relationships WHERE source_id=?1 OR target_id=?1")?;
        let rows = statement.query_map(params![entity_id], |row| {
            Ok(Relationship {
                id: row.get(0)?,
                source_id: row.get(1)?,
                target_id: row.get(2)?,
                relationship_type: row.get(3)?,
                metadata: row.get(4)?,
                revision: String::new(),
            })
        })?;
        let mut relationships = rows.collect::<Result<Vec<_>, _>>()?;
        for relationship in &mut relationships {
            relationship.revision = self.revision_for_relationship_value(relationship)?;
        }
        Ok(relationships)
    }

    pub fn search(&self, query: String) -> Result<Vec<Entity>, CoreError> {
        if query.trim().is_empty() {
            return self.list_entities();
        }
        let terms = query
            .split_whitespace()
            .map(|term| format!("\"{}\"*", term.replace('"', "")))
            .collect::<Vec<_>>()
            .join(" AND ");
        let mut statement = self.connection.prepare("SELECT e.id,e.name,e.entity_type,e.deleted,e.created_at,e.updated_at FROM entities e JOIN (SELECT entity_id, MIN(rowid) AS rank FROM world_search WHERE world_search MATCH ?1 GROUP BY entity_id) s ON s.entity_id=e.id WHERE e.deleted=0 ORDER BY s.rank LIMIT 100")?;
        let rows = statement.query_map(params![terms], |row| {
            Ok(Entity {
                id: row.get(0)?,
                name: row.get(1)?,
                entity_type: row.get(2)?,
                deleted: row.get::<_, i64>(3)? != 0,
                created_at: row.get(4)?,
                updated_at: row.get(5)?,
                revision: String::new(),
            })
        })?;
        let mut entities = rows.collect::<Result<Vec<_>, _>>()?;
        self.populate_entity_revisions(&mut entities)?;
        Ok(entities)
    }

    /// Return ranked FTS rows rather than collapsing matches to entities. The
    /// AI context builder adds authorization, byte ranges, and prompt framing.
    pub fn search_passages(
        &self,
        query: String,
        limit: usize,
    ) -> Result<Vec<SearchPassage>, CoreError> {
        if query.trim().is_empty() || limit == 0 {
            return Ok(Vec::new());
        }
        let terms = query
            .split_whitespace()
            .map(|term| format!("\"{}\"*", term.replace('"', "")))
            .collect::<Vec<_>>()
            .join(" AND ");
        let mut statement = self.connection.prepare(
            "SELECT entity_id,source_path,source_hash,content,rank FROM world_search WHERE world_search MATCH ?1 ORDER BY rank LIMIT ?2",
        )?;
        let rows = statement.query_map(params![terms, limit as i64], |row| {
            Ok(SearchPassage {
                entity_id: row.get(0)?,
                source_path: row.get(1)?,
                source_hash: row.get(2)?,
                content: row.get(3)?,
                lexical_rank: row.get(4)?,
                source_kind: {
                    let path: String = row.get(1)?;
                    if path.ends_with("/document.md") {
                        "document".into()
                    } else if path.contains("/fields/") {
                        "field".into()
                    } else {
                        "entity".into()
                    }
                },
            })
        })?;
        rows.collect::<Result<Vec<_>, _>>()
            .map(|mut passages| {
                for (rank, passage) in passages.iter_mut().enumerate() {
                    passage.lexical_rank = rank as f64;
                }
                passages
            })
            .map_err(CoreError::from)
    }

    pub fn export_json(&self) -> Result<String, CoreError> {
        self.export_json_inner()
    }

    fn export_json_inner(&self) -> Result<String, CoreError> {
        serde_json::to_string_pretty(&self.export_snapshot()?)
            .map_err(|error| CoreError::Serialization(error.to_string()))
    }

    fn export_snapshot(&self) -> Result<ProjectSnapshot, CoreError> {
        let entities = self
            .connection
            .prepare("SELECT id,name,entity_type,deleted,created_at,updated_at FROM entities ORDER BY name,id")?
            .query_map([], |row| {
                Ok(Entity {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    entity_type: row.get(2)?,
                    deleted: row.get::<_, i64>(3)? != 0,
                    created_at: row.get(4)?,
                    updated_at: row.get(5)?,
                    revision: String::new(),
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        let documents = self
            .connection
            .prepare("SELECT id,entity_id,format,body,updated_at FROM documents ORDER BY entity_id,updated_at DESC,id")?
            .query_map([], |row| {
                Ok(Document {
                    id: row.get(0)?,
                    entity_id: row.get(1)?,
                    format: row.get(2)?,
                    body: row.get(3)?,
                    updated_at: row.get(4)?,
                    revision: String::new(),
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        let fields = self
            .connection
            .prepare("SELECT entity_id,namespace,key,value FROM entity_fields ORDER BY entity_id,namespace,key")?
            .query_map([], |row| {
                let value: String = row.get(3)?;
                Ok(FieldValue {
                    entity_id: row.get(0)?,
                    namespace: row.get(1)?,
                    key: row.get(2)?,
                    value: decode_field_value(value),
                    revision: String::new(),
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        let relationships = self
            .connection
            .prepare("SELECT id,source_id,target_id,relationship_type,metadata FROM relationships ORDER BY id")?
            .query_map([], |row| {
                Ok(Relationship {
                    id: row.get(0)?,
                    source_id: row.get(1)?,
                    target_id: row.get(2)?,
                    relationship_type: row.get(3)?,
                    metadata: row.get(4)?,
                    revision: String::new(),
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        let assets = self
            .connection
            .prepare("SELECT id,entity_id,namespace,filename,content_hash,size,mime_type,path,created_at FROM assets ORDER BY entity_id,created_at,id")?
            .query_map([], |row| {
                Ok(Asset {
                    id: row.get(0)?,
                    entity_id: row.get(1)?,
                    namespace: row.get(2)?,
                    filename: row.get(3)?,
                    content_hash: row.get(4)?,
                    size: row.get(5)?,
                    mime_type: row.get(6)?,
                    path: row.get(7)?,
                    created_at: row.get(8)?,
                    revision: String::new(),
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        let mut module_statement = self.connection.prepare("SELECT m.module_id, COALESCE(s.enabled, 1), m.version, p.package_version, o.overlay_json FROM module_versions m LEFT JOIN module_state s ON s.module_id = m.module_id LEFT JOIN module_package_versions p ON p.module_id = m.module_id LEFT JOIN module_schema_overlays o ON o.module_id = m.module_id ORDER BY m.module_id")?;
        let modules = module_statement
            .query_map([], |row| {
                let overlay_json: Option<String> = row.get(4)?;
                let schema_overlay = overlay_json.and_then(|json| {
                    serde_json::from_str(&json)
                        .ok()
                        .filter(|value: &serde_json::Value| !value.is_null())
                });
                Ok(ModuleState {
                    module_id: row.get(0)?,
                    enabled: row.get::<_, i64>(1)? != 0,
                    version: row.get(2)?,
                    package_version: row.get(3)?,
                    schema_overlay,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        let module_namespaces = self
            .connection
            .prepare(
                "SELECT module_id,namespace FROM module_namespaces ORDER BY module_id,namespace",
            )?
            .query_map([], |row| {
                Ok(ModuleNamespace {
                    module_id: row.get(0)?,
                    namespace: row.get(1)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        let module_fields = self
            .connection
            .prepare("SELECT module_id,namespace,key,field_type,required FROM module_fields ORDER BY module_id,namespace,key")?
            .query_map([], |row| {
                Ok(ModuleField {
                    module_id: row.get(0)?,
                    namespace: row.get(1)?,
                    key: row.get(2)?,
                    field_type: row.get(3)?,
                    required: row.get::<_, i64>(4)? != 0,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        let module_records = self
            .connection
            .prepare("SELECT module_id,collection,id,owner_entity_id,value,created_at,updated_at FROM module_records ORDER BY module_id,collection,id")?
            .query_map([], |row| {
                let value: String = row.get(4)?;
                Ok(ModuleRecord {
                    module_id: row.get(0)?,
                    collection: row.get(1)?,
                    id: row.get(2)?,
                    owner_entity_id: row.get(3)?,
                    value: decode_field_value(value),
                    created_at: row.get(5)?,
                    updated_at: row.get(6)?,
                    revision: String::new(),
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        let migration_history = self
            .connection
            .prepare("SELECT module_id,migration_id,from_version,to_version,checksum,package_digest,applied_at FROM migration_history ORDER BY module_id,migration_id")?
            .query_map([], |row| {
                Ok(MigrationHistoryEntry {
                    module_id: row.get(0)?,
                    migration_id: row.get(1)?,
                    from_version: row.get(2)?,
                    to_version: row.get(3)?,
                    checksum: row.get(4)?,
                    package_digest: row.get(5)?,
                    applied_at: row.get(6)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(ProjectSnapshot {
            format_version: current_snapshot_version(),
            entities,
            documents,
            fields,
            relationships,
            assets,
            modules,
            module_namespaces,
            module_fields,
            module_records,
            migration_history,
        })
    }

    #[allow(dead_code)]
    fn import_json_with_mode(&self, payload: &str, replace: bool) -> Result<usize, CoreError> {
        self.import_json_with_mode_and_sync(payload, replace, true)
    }

    fn import_json_with_mode_and_sync(
        &self,
        payload: &str,
        replace: bool,
        sync_canonical: bool,
    ) -> Result<usize, CoreError> {
        self.import_json_with_mode_and_sync_with_request(payload, replace, sync_canonical, None)
    }

    fn import_json_with_mode_and_sync_with_request(
        &self,
        payload: &str,
        replace: bool,
        sync_canonical: bool,
        request_id: Option<&str>,
    ) -> Result<usize, CoreError> {
        self.import_json_with_mode_and_sync_with_request_and_search(
            payload,
            replace,
            sync_canonical,
            request_id,
            true,
        )
    }

    fn import_json_with_mode_and_sync_with_request_and_search(
        &self,
        payload: &str,
        replace: bool,
        sync_canonical: bool,
        request_id: Option<&str>,
        rebuild_search: bool,
    ) -> Result<usize, CoreError> {
        let snapshot: ProjectSnapshot = serde_json::from_str(payload)
            .map_err(|error| CoreError::NotFound(error.to_string()))?;
        if snapshot.format_version != current_snapshot_version() {
            return Err(CoreError::NotFound(format!(
                "unsupported project snapshot version {}",
                snapshot.format_version
            )));
        }
        if let Some(root) = self.root.as_deref() {
            for asset in &snapshot.assets {
                ensure_runtime_asset(root, &asset.path, &asset.content_hash, asset.size)?;
            }
        }
        let mutation_request_id = if sync_canonical {
            Some(self.request_id(request_id)?)
        } else {
            None
        };
        let transaction = if let Some(ref request_id) = mutation_request_id {
            self.begin_mutation(
                request_id,
                Some(&serde_json::Value::Null),
                &["entities/".into(), "plugins/".into()],
            )?
        } else {
            self.connection.unchecked_transaction()?
        };
        if replace {
            transaction.execute_batch(
                "DELETE FROM assets;
                 DELETE FROM entity_fields;
                 DELETE FROM documents;
                 DELETE FROM relationships;
                 DELETE FROM module_records;
                 DELETE FROM entities;
                 DELETE FROM module_state;
                 DELETE FROM module_versions;
                 DELETE FROM module_package_versions;
                 DELETE FROM module_fields;
                 DELETE FROM module_namespaces;
                 DELETE FROM module_schema_overlays;
                 DELETE FROM migration_history;",
            )?;
        }
        for entity in &snapshot.entities {
            transaction.execute("INSERT INTO entities(id,name,entity_type,deleted,created_at,updated_at) VALUES (?1,?2,?3,?4,?5,?6) ON CONFLICT(id) DO UPDATE SET name=excluded.name,entity_type=excluded.entity_type,deleted=excluded.deleted,created_at=excluded.created_at,updated_at=excluded.updated_at", params![entity.id, entity.name, entity.entity_type, entity.deleted as i64, entity.created_at, entity.updated_at])?;
        }
        for document in &snapshot.documents {
            transaction.execute("INSERT INTO documents(id,entity_id,format,body,updated_at) VALUES (?1,?2,?3,?4,?5) ON CONFLICT(id) DO UPDATE SET entity_id=excluded.entity_id,format=excluded.format,body=excluded.body,updated_at=excluded.updated_at", params![document.id, document.entity_id, document.format, document.body, document.updated_at])?;
        }
        for field in &snapshot.fields {
            let value = encode_field_value(&field.value)?;
            transaction.execute("INSERT INTO entity_fields(entity_id,namespace,key,value) VALUES (?1,?2,?3,?4) ON CONFLICT(entity_id,namespace,key) DO UPDATE SET value=excluded.value", params![field.entity_id, field.namespace, field.key, value])?;
        }
        for relationship in &snapshot.relationships {
            transaction.execute("INSERT INTO relationships(id,source_id,target_id,relationship_type,metadata) VALUES (?1,?2,?3,?4,?5) ON CONFLICT(id) DO UPDATE SET source_id=excluded.source_id,target_id=excluded.target_id,relationship_type=excluded.relationship_type,metadata=excluded.metadata", params![relationship.id, relationship.source_id, relationship.target_id, relationship.relationship_type, relationship.metadata])?;
        }
        for asset in &snapshot.assets {
            transaction.execute("INSERT INTO assets(id,entity_id,namespace,filename,content_hash,size,mime_type,path,created_at) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9) ON CONFLICT(id) DO UPDATE SET entity_id=excluded.entity_id,namespace=excluded.namespace,filename=excluded.filename,content_hash=excluded.content_hash,size=excluded.size,mime_type=excluded.mime_type,path=excluded.path,created_at=excluded.created_at", params![asset.id, asset.entity_id, asset.namespace, asset.filename, asset.content_hash, asset.size, asset.mime_type, asset.path, asset.created_at])?;
        }
        for module in &snapshot.modules {
            transaction.execute("INSERT INTO module_versions(module_id,version) VALUES (?1,?2) ON CONFLICT(module_id) DO UPDATE SET version=excluded.version", params![module.module_id, module.version])?;
            transaction.execute("INSERT INTO module_state(module_id,enabled) VALUES (?1,?2) ON CONFLICT(module_id) DO UPDATE SET enabled=excluded.enabled", params![module.module_id, module.enabled as i64])?;
            if let Some(package_version) = &module.package_version {
                transaction.execute("INSERT INTO module_package_versions(module_id,package_version) VALUES (?1,?2) ON CONFLICT(module_id) DO UPDATE SET package_version=excluded.package_version", params![module.module_id, package_version])?;
            }
            if let Some(overlay) = &module.schema_overlay {
                if !overlay.is_null() {
                    let overlay_json = serde_json::to_string(overlay)
                        .map_err(|error| CoreError::Validation(error.to_string()))?;
                    transaction.execute(
                        "INSERT INTO module_schema_overlays(module_id, overlay_json) VALUES (?1, ?2) ON CONFLICT(module_id) DO UPDATE SET overlay_json=excluded.overlay_json",
                        params![module.module_id, overlay_json],
                    )?;
                }
            }
        }
        for namespace in &snapshot.module_namespaces {
            transaction.execute(
                "INSERT INTO module_namespaces(module_id,namespace) VALUES (?1,?2)",
                params![namespace.module_id, namespace.namespace],
            )?;
        }
        for field in &snapshot.module_fields {
            transaction.execute(
                "INSERT INTO module_fields(module_id,namespace,key,field_type,required) VALUES (?1,?2,?3,?4,?5)",
                params![field.module_id, field.namespace, field.key, field.field_type, field.required as i64],
            )?;
        }
        for record in &snapshot.module_records {
            let value = serde_json::to_string(&record.value)
                .map_err(|error| CoreError::Validation(error.to_string()))?;
            transaction.execute(
                "INSERT INTO module_records(module_id,collection,id,owner_entity_id,value,created_at,updated_at) VALUES (?1,?2,?3,?4,?5,?6,?7) ON CONFLICT(id) DO UPDATE SET module_id=excluded.module_id,collection=excluded.collection,owner_entity_id=excluded.owner_entity_id,value=excluded.value,created_at=excluded.created_at,updated_at=excluded.updated_at",
                params![record.module_id, record.collection, record.id, record.owner_entity_id, value, record.created_at, record.updated_at],
            )?;
        }
        for migration in &snapshot.migration_history {
            transaction.execute(
                "INSERT INTO migration_history(module_id,migration_id,from_version,to_version,checksum,package_digest,applied_at) VALUES (?1,?2,?3,?4,?5,?6,?7)",
                params![migration.module_id, migration.migration_id, migration.from_version, migration.to_version, migration.checksum, migration.package_digest, migration.applied_at],
            )?;
        }
        if let Some(root) = self.root.as_deref() {
            crate::maps::validate_image_map_content(&transaction, |asset_id| {
                let hash: String = transaction.query_row(
                    "SELECT content_hash FROM assets WHERE id=?1",
                    params![asset_id],
                    |row| row.get(0),
                )?;
                let path = runtime_asset_path(root, &hash)?;
                std::fs::read(&path).map_err(|source| CoreError::Io {
                    operation: "read image map asset",
                    source,
                })
            })?;
        }
        transaction.commit()?;
        if rebuild_search {
            self.rebuild_search()?;
        }
        if sync_canonical {
            self.notify_export_worker()?;
        }
        Ok(snapshot.entities.len())
    }

    pub fn register_asset(&self, input: AssetInput) -> Result<Asset, CoreError> {
        self.register_asset_with_options(input, None, None)
    }

    pub fn register_asset_with_request(
        &self,
        input: AssetInput,
        request_id: Option<&str>,
    ) -> Result<Asset, CoreError> {
        self.register_asset_with_options(input, None, request_id)
    }

    pub fn register_asset_with_options(
        &self,
        input: AssetInput,
        expected_revision: Option<&str>,
        request_id: Option<&str>,
    ) -> Result<Asset, CoreError> {
        if let Some(mut asset) = self.committed_mutation::<Asset>(request_id)? {
            asset.revision = self.revision_for_asset_value(&asset)?;
            return Ok(asset);
        }
        let exists: Option<String> = self
            .connection
            .query_row(
                "SELECT id FROM entities WHERE id=?1 AND deleted=0",
                params![input.entity_id],
                |row| row.get(0),
            )
            .optional()?;
        if exists.is_none() {
            return Err(CoreError::NotFound("entity not found".into()));
        }
        Self::ensure_expected_revision(
            expected_revision,
            self.revision_for_entity(&input.entity_id)?,
            "asset entity",
        )?;
        if input.size < 0 {
            return Err(CoreError::Validation(
                "asset size cannot be negative".into(),
            ));
        }
        if let Some(root) = self.root.as_deref() {
            ensure_runtime_asset(root, &input.path, &input.content_hash, input.size)?;
        }
        let id = Uuid::new_v4().to_string();
        let now = chrono_like_now();
        let request_id = self.request_id(request_id)?;
        let result = serde_json::to_value(&Asset {
            id: id.clone(),
            entity_id: input.entity_id.clone(),
            namespace: input.namespace.clone(),
            filename: input.filename.clone(),
            content_hash: input.content_hash.clone(),
            size: input.size,
            mime_type: input.mime_type.clone(),
            path: input.path.clone(),
            created_at: now.clone(),
            revision: String::new(),
        })
        .map_err(|error| CoreError::Serialization(error.to_string()))?;
        let transaction = self.begin_mutation(
            &request_id,
            Some(&result),
            &[format!("entities/{}/", input.entity_id), input.path.clone()],
        )?;
        transaction.execute(
            "INSERT INTO assets(id,entity_id,namespace,filename,content_hash,size,mime_type,path,created_at) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9)",
            params![id, input.entity_id, input.namespace, input.filename, input.content_hash, input.size, input.mime_type, input.path, now],
        )?;
        transaction.commit()?;
        self.refresh_maps_projection_for_entities(std::slice::from_ref(&input.entity_id))?;
        self.notify_export_worker()?;
        let revision = self.revision_for_asset(&id)?;
        Ok(Asset {
            id,
            entity_id: input.entity_id,
            namespace: input.namespace,
            filename: input.filename,
            content_hash: input.content_hash,
            size: input.size,
            mime_type: input.mime_type,
            path: input.path,
            created_at: now,
            revision,
        })
    }

    pub fn register_asset_file(&self, input: AssetFileInput) -> Result<Asset, CoreError> {
        self.register_asset_file_with_options(input, None, None)
    }

    pub fn register_asset_file_with_request(
        &self,
        input: AssetFileInput,
        request_id: Option<&str>,
    ) -> Result<Asset, CoreError> {
        self.register_asset_file_with_options(input, None, request_id)
    }

    pub fn register_asset_file_with_options(
        &self,
        input: AssetFileInput,
        expected_revision: Option<&str>,
        request_id: Option<&str>,
    ) -> Result<Asset, CoreError> {
        let source = Path::new(&input.source_path);
        let metadata =
            std::fs::metadata(source).map_err(|error| CoreError::NotFound(error.to_string()))?;
        if !metadata.is_file() {
            return Err(CoreError::NotFound("asset source is not a file".into()));
        }
        let filename = Path::new(&input.filename)
            .file_name()
            .and_then(|value| value.to_str())
            .filter(|value| !value.is_empty() && *value != "." && *value != "..")
            .ok_or_else(|| CoreError::NotFound("asset filename is invalid".into()))?;
        let category = if input.mime_type.starts_with("image/") {
            "images"
        } else if input.mime_type.starts_with("video/") {
            "videos"
        } else if input.mime_type.contains("map")
            || matches!(
                Path::new(filename)
                    .extension()
                    .and_then(|value| value.to_str()),
                Some("geojson" | "tmx" | "mbtiles")
            )
        {
            "maps"
        } else {
            "files"
        };
        let (content_hash, size) = if let Some(root) = self.root.as_deref() {
            store_runtime_asset_file(root, source, None)?
        } else {
            streamed_file_digest(source)?
        };
        let relative_path = format!("assets/{category}/{}-{}", Uuid::new_v4(), filename);
        let request_id = self.request_id(request_id)?;
        self.register_asset_with_options(
            AssetInput {
                entity_id: input.entity_id,
                namespace: input.namespace,
                filename: filename.into(),
                content_hash,
                size,
                mime_type: input.mime_type,
                path: relative_path.clone(),
            },
            expected_revision,
            Some(&request_id),
        )
    }

    pub fn list_assets(&self, entity_id: String) -> Result<Vec<Asset>, CoreError> {
        self.list_assets_unchecked(entity_id)
    }

    pub fn asset(&self, asset_id: String) -> Result<Asset, CoreError> {
        self.asset_unchecked(&asset_id)
    }

    /// Reads the live content-addressed bytes for a registered asset. Portable
    /// paths are checkpoint output and may not exist yet immediately after a
    /// runtime mutation.
    pub fn asset_bytes(&self, asset_id: String) -> Result<Vec<u8>, CoreError> {
        let asset = self.asset_unchecked(&asset_id)?;
        self.read_asset_bytes(&asset)
    }

    fn asset_unchecked(&self, asset_id: &str) -> Result<Asset, CoreError> {
        let mut asset = self
            .connection
            .query_row(
                "SELECT id,entity_id,namespace,filename,content_hash,size,mime_type,path,created_at FROM assets WHERE id=?1",
                params![asset_id],
                |row| {
                    Ok(Asset {
                        id: row.get(0)?,
                        entity_id: row.get(1)?,
                        namespace: row.get(2)?,
                        filename: row.get(3)?,
                        content_hash: row.get(4)?,
                        size: row.get(5)?,
                        mime_type: row.get(6)?,
                        path: row.get(7)?,
                        created_at: row.get(8)?,
                        revision: String::new(),
                    })
                },
            )
            .optional()?
            .ok_or_else(|| CoreError::NotFound("asset not found".into()))?;
        asset.revision = self.revision_for_asset(&asset.id)?;
        Ok(asset)
    }

    pub fn replace_asset_bytes_with_request(
        &self,
        input: AssetReplaceInput,
        bytes: Vec<u8>,
        expected_revision: &str,
        request_id: Option<&str>,
    ) -> Result<Asset, CoreError> {
        if let Some(request_id) = request_id {
            self.request_id(Some(request_id))?;
        }
        if let Some(mut asset) = self.committed_mutation::<Asset>(request_id)? {
            asset.revision = self.revision_for_asset(&asset.id)?;
            return Ok(asset);
        }
        if input.size < 0 || input.size as usize != bytes.len() {
            return Err(CoreError::Validation(
                "asset replacement size does not match declared size".into(),
            ));
        }
        let digest = format!("sha256:{:x}", Sha256::digest(&bytes));
        if input.content_hash != digest {
            return Err(CoreError::Validation(
                "asset replacement content hash does not match bytes".into(),
            ));
        }
        let mut asset = self.asset_unchecked(&input.asset_id)?;
        Self::ensure_expected_revision(Some(expected_revision), asset.revision.clone(), "asset")?;
        if input.mime_type.trim().is_empty() {
            return Err(CoreError::Validation(
                "asset replacement MIME type is required".into(),
            ));
        }
        if let Some((width, height)) = self.raster_layer_expected_size(&asset)? {
            if input.mime_type != "image/png" {
                return Err(CoreError::Validation(
                    "maps: painted layers must remain PNG assets".into(),
                ));
            }
            crate::maps::validate_raster_png(&bytes, width, height)?;
        } else if self.is_immutable_image_source(&asset)? {
            return Err(CoreError::Validation(
                "maps: imported source assets cannot be replaced".into(),
            ));
        }
        let request_id = self.request_id(request_id)?;
        if let Some(root) = self.root.as_deref() {
            let (stored_hash, stored_size) = store_runtime_asset(root, bytes.as_slice(), None)?;
            if stored_hash != input.content_hash || stored_size != input.size {
                return Err(CoreError::Validation(
                    "asset replacement bytes do not match metadata".into(),
                ));
            }
        }
        let transaction = match self.begin_mutation(
            &request_id,
            Some(&serde_json::Value::Null),
            &[format!("entities/{}/", asset.entity_id), asset.path.clone()],
        ) {
            Ok(value) => value,
            Err(error) => {
                return Err(error);
            }
        };
        let mutation = (|| -> Result<(), CoreError> {
            transaction.execute(
                "UPDATE assets SET content_hash=?1,size=?2,mime_type=?3 WHERE id=?4",
                params![
                    input.content_hash,
                    input.size,
                    input.mime_type,
                    input.asset_id
                ],
            )?;
            transaction.commit()?;
            Ok(())
        })();
        mutation?;
        self.refresh_maps_projection_for_entities(std::slice::from_ref(&asset.entity_id))?;
        self.notify_export_worker()?;
        asset.content_hash = input.content_hash;
        asset.size = input.size;
        asset.mime_type = input.mime_type;
        asset.revision = self.revision_for_asset(&asset.id)?;
        Ok(asset)
    }

    fn list_assets_unchecked(&self, entity_id: String) -> Result<Vec<Asset>, CoreError> {
        let mut statement = self.connection.prepare("SELECT id,entity_id,namespace,filename,content_hash,size,mime_type,path,created_at FROM assets WHERE entity_id=?1 ORDER BY created_at")?;
        let rows = statement.query_map(params![entity_id], |row| {
            Ok(Asset {
                id: row.get(0)?,
                entity_id: row.get(1)?,
                namespace: row.get(2)?,
                filename: row.get(3)?,
                content_hash: row.get(4)?,
                size: row.get(5)?,
                mime_type: row.get(6)?,
                path: row.get(7)?,
                created_at: row.get(8)?,
                revision: String::new(),
            })
        })?;
        let mut assets = rows.collect::<Result<Vec<_>, _>>()?;
        for asset in &mut assets {
            asset.revision = self.revision_for_asset_value(asset)?;
        }
        Ok(assets)
    }

    pub fn export_markdown_to(&self, destination: impl AsRef<Path>) -> Result<String, CoreError> {
        let mut entities = self.list_entities()?;
        entities.sort_by(|left, right| {
            left.name
                .to_lowercase()
                .cmp(&right.name.to_lowercase())
                .then_with(|| left.id.cmp(&right.id))
        });

        let mut proposed = entities
            .iter()
            .map(|entity| format!("{}.md", markdown_export_stem(&entity.name)))
            .collect::<Vec<_>>();
        let mut collisions: BTreeMap<String, Vec<usize>> = BTreeMap::new();
        for (index, filename) in proposed.iter().enumerate() {
            collisions
                .entry(filename.to_lowercase())
                .or_default()
                .push(index);
        }
        for indexes in collisions.values().filter(|indexes| indexes.len() > 1) {
            for index in indexes {
                let suffix = entities[*index].id.chars().take(8).collect::<String>();
                proposed[*index] = format!("{}-{suffix}.md", markdown_export_stem(&entities[*index].name));
            }
        }

        let filenames = entities
            .iter()
            .zip(proposed.iter())
            .map(|(entity, filename)| (entity.id.clone(), filename.clone()))
            .collect::<BTreeMap<_, _>>();
        let names = entities
            .iter()
            .map(|entity| (entity.id.clone(), entity.name.clone()))
            .collect::<BTreeMap<_, _>>();

        let destination = destination.as_ref();
        std::fs::create_dir_all(destination).map_err(|source| CoreError::Io {
            operation: "create Markdown export destination",
            source,
        })?;
        let project_name = self
            .info()
            .map(|info| info.name)
            .unwrap_or_else(|| "Daena Archive".into());
        let export_stem = markdown_export_stem(&project_name);
        let mut export_directory = destination.join(format!("{export_stem}-markdown"));
        let mut suffix = 2;
        while export_directory.exists() {
            export_directory = destination.join(format!("{export_stem}-markdown-{suffix}"));
            suffix += 1;
        }
        std::fs::create_dir(&export_directory).map_err(|source| CoreError::Io {
            operation: "create Markdown export directory",
            source,
        })?;

        for (entity, filename) in entities.iter().zip(proposed.iter()) {
            let documents = self.list_documents(entity.id.clone())?;
            let document = documents.into_iter().next();
            if let Some(document) = &document {
                if document.format != "markdown" {
                    return Err(CoreError::Validation(
                        "Markdown export requires Markdown documents".into(),
                    ));
                }
            }
            let body = document.map(|document| document.body).unwrap_or_default();
            let mut markdown = rewrite_markdown_entity_links(&body, &filenames);
            markdown.push_str("\n\n## Relationships\n\n");

            let mut grouped: BTreeMap<String, Vec<(String, Option<String>)>> = BTreeMap::new();
            for relationship in self
                .list_relationships(entity.id.clone())?
                .into_iter()
                .filter(|relationship| relationship.source_id == entity.id)
            {
                let target_name = names
                    .get(&relationship.target_id)
                    .cloned()
                    .unwrap_or_else(|| relationship.target_id.clone());
                let target_filename = filenames.get(&relationship.target_id).cloned();
                grouped
                    .entry(relationship.relationship_type)
                    .or_default()
                    .push((target_name, target_filename));
            }

            if grouped.is_empty() {
                markdown.push_str("_No outgoing relationships._\n");
            } else {
                for (relationship_type, mut targets) in grouped {
                    targets.sort_by(|left, right| left.0.cmp(&right.0).then_with(|| left.1.cmp(&right.1)));
                    markdown.push_str("### ");
                    markdown.push_str(&markdown_relationship_heading(&relationship_type));
                    markdown.push_str("\n\n");
                    for (target_name, target_filename) in targets {
                        markdown.push_str("- ");
                        if let Some(target_filename) = target_filename {
                            markdown.push('[');
                            markdown.push_str(&markdown_escape_label(&target_name));
                            markdown.push_str("](");
                            markdown.push_str(&markdown_export_target(&target_filename));
                            markdown.push_str(")\n");
                        } else {
                            markdown.push_str(&markdown_escape_label(&target_name));
                            markdown.push('\n');
                        }
                    }
                    markdown.push('\n');
                }
            }

            std::fs::write(export_directory.join(filename), markdown).map_err(|source| {
                CoreError::Io {
                    operation: "write Markdown export file",
                    source,
                }
            })?;
        }

        Ok(export_directory.to_string_lossy().into_owned())
    }

    pub fn backup(&self) -> Result<String, CoreError> {
        self.backup_to(std::env::temp_dir())
    }

    pub fn backup_to(&self, dir: impl AsRef<Path>) -> Result<String, CoreError> {
        if self.root.is_some() {
            self.flush_checkpoint("runtime backup")?;
        }
        let export = self.export_json()?;
        let dir = dir.as_ref();
        std::fs::create_dir_all(dir).map_err(|e| CoreError::NotFound(e.to_string()))?;
        let timestamp = chrono_like_now();
        let filename = format!("daena-backup-{}-{}.json", timestamp, Uuid::new_v4());
        let path = dir.join(&filename);
        std::fs::write(&path, export).map_err(|e| CoreError::NotFound(e.to_string()))?;
        Ok(path.to_string_lossy().to_string())
    }

    /// Create a files-only portable checkpoint.  Runtime SQLite state and
    /// `.daena/` are intentionally excluded so the result can be restored or
    /// copied using only the canonical project representation.
    pub fn portable_backup_to(&self, dir: impl AsRef<Path>) -> Result<String, CoreError> {
        self.flush_checkpoint("portable backup")?;
        self.portable_backup_after_checkpoint(dir)
    }

    pub fn portable_backup_after_checkpoint(
        &self,
        dir: impl AsRef<Path>,
    ) -> Result<String, CoreError> {
        let root = self.project_root()?;
        crate::storage::FilesystemRepository::open(root)?.scan()?;

        let dir = dir.as_ref();
        std::fs::create_dir_all(dir).map_err(|source| CoreError::Io {
            operation: "create portable backup directory",
            source,
        })?;
        let destination = dir.join(format!(
            "daena-portable-{}-{}",
            chrono_like_now(),
            Uuid::new_v4()
        ));
        copy_portable_project(root, &destination)?;
        Ok(destination.to_string_lossy().into_owned())
    }

    /// Create a machine-local recovery artifact for the current runtime.
    /// Unlike a portable backup, this deliberately preserves the SQLite
    /// runtime state and any staged exporter payloads needed to resume it.
    pub fn recovery_backup_to(&mut self, dir: impl AsRef<Path>) -> Result<String, CoreError> {
        let stopped_worker = self.export_worker.take();
        if let Some(worker) = stopped_worker {
            worker.stop_without_drain()?;
        }
        let result = self.recovery_backup_to_quiesced(dir);
        if self.export_worker.is_none() {
            self.restart_export_worker()?;
        }
        result
    }

    fn recovery_backup_to_quiesced(&self, dir: impl AsRef<Path>) -> Result<String, CoreError> {
        let dir = dir.as_ref();
        std::fs::create_dir_all(dir).map_err(|source| CoreError::Io {
            operation: "create recovery backup directory",
            source,
        })?;
        let recovery = dir.join(format!(
            "daena-recovery-{}-{}",
            chrono_like_now(),
            Uuid::new_v4()
        ));
        std::fs::create_dir_all(&recovery).map_err(|source| CoreError::Io {
            operation: "create recovery backup artifact",
            source,
        })?;
        let database_path = recovery.join("index.sqlite");
        {
            let mut destination = Connection::open(&database_path)?;
            let backup = rusqlite::backup::Backup::new(&self.connection, &mut destination)?;
            backup.run_to_completion(5, Duration::from_millis(25), None)?;
        }

        Ok(recovery.to_string_lossy().into_owned())
    }

    fn restart_export_worker(&mut self) -> Result<(), CoreError> {
        if self.export_worker.is_none() {
            if let Some(root) = self.root.as_deref() {
                let database = project_database_path(root);
                if database.is_file() {
                    self.export_worker = Some(ExportWorker::start(root, &database)?);
                }
            }
        }
        Ok(())
    }

    /// Restore a runtime recovery artifact. The current runtime is archived
    /// before replacement; the next generation barrier recreates any files.
    pub fn restore_recovery_backup(&mut self, path: impl AsRef<Path>) -> Result<(), CoreError> {
        let artifact = path.as_ref();
        let source_path = artifact.join("index.sqlite");
        if !source_path.is_file() {
            return Err(CoreError::NotFound(
                "recovery backup is missing index.sqlite".into(),
            ));
        }
        let root = self.project_root()?.to_path_buf();
        let source = Connection::open(&source_path)?;
        Self::validate_runtime_metadata(&source, Some(&root))?;
        let backup_dir = root.join(".daena/backups");
        if let Some(worker) = self.export_worker.take() {
            worker.stop_without_drain()?;
        }
        if let Err(error) = self.recovery_backup_to_quiesced(&backup_dir) {
            self.restart_export_worker()?;
            return Err(error);
        }

        {
            let backup = rusqlite::backup::Backup::new(&source, &mut self.connection)?;
            backup.run_to_completion(5, Duration::from_millis(25), None)?;
        }
        Self::validate_runtime_metadata(&self.connection, Some(&root))?;
        self.export_worker = Some(ExportWorker::start(&root, &project_database_path(&root))?);
        Ok(())
    }

    pub fn create_plugin_backup(
        &self,
        module_id: &str,
        from_package_version: Option<&str>,
        to_package_version: Option<&str>,
        data_version: i64,
    ) -> Result<PluginBackup, CoreError> {
        self.create_plugin_backup_with_request(
            module_id,
            from_package_version,
            to_package_version,
            data_version,
            None,
        )
    }

    pub fn create_plugin_backup_with_request(
        &self,
        module_id: &str,
        from_package_version: Option<&str>,
        to_package_version: Option<&str>,
        data_version: i64,
        request_id: Option<&str>,
    ) -> Result<PluginBackup, CoreError> {
        if self.root.is_some() {
            return self.create_plugin_backup_runtime_snapshot(
                module_id,
                from_package_version,
                to_package_version,
                data_version,
                request_id,
            );
        }
        if let Some(backup) = self.committed_mutation::<PluginBackup>(request_id)? {
            return Ok(backup);
        }
        if module_id.trim().is_empty() {
            return Err(CoreError::Validation(
                "plugin backup requires a module ID".into(),
            ));
        }
        let root = self.project_root()?.to_path_buf();
        let created_at = chrono_like_now();
        let backup_id = Uuid::new_v4().to_string();
        let safe_module = module_id
            .chars()
            .map(|character| {
                if character.is_ascii_alphanumeric() || matches!(character, '.' | '-' | '_') {
                    character
                } else {
                    '_'
                }
            })
            .collect::<String>();
        let relative_path =
            format!(".daena/backups/plugins/plugin-{safe_module}-{created_at}-{backup_id}.json");
        let path = root.join(&relative_path);
        let payload = self.export_json()?;
        let content_hash = digest_bytes(payload.as_bytes());
        let backup = PluginBackup {
            id: backup_id,
            module_id: module_id.into(),
            from_package_version: from_package_version.map(str::to_owned),
            to_package_version: to_package_version.map(str::to_owned),
            data_version,
            path: path.to_string_lossy().into_owned(),
            content_hash,
            created_at,
        };
        let request_id = self.request_id(request_id)?;
        let mut transaction = crate::sync::SyncExporter::begin(&root, &request_id)?;
        transaction.stage_bytes(&relative_path, payload.as_bytes())?;
        let result = serde_json::to_value(&backup)
            .map_err(|error| CoreError::Serialization(error.to_string()))?;
        transaction.commit(Some(&result))?;
        let result = self.connection.execute(
            "INSERT INTO plugin_backups(id,module_id,from_package_version,to_package_version,data_version,path,content_hash,created_at) VALUES (?1,?2,?3,?4,?5,?6,?7,?8)",
            params![backup.id, backup.module_id, backup.from_package_version, backup.to_package_version, backup.data_version, backup.path, backup.content_hash, backup.created_at],
        );
        if let Err(error) = result {
            return Err(error.into());
        }
        Ok(backup)
    }

    fn create_plugin_backup_runtime_snapshot(
        &self,
        module_id: &str,
        from_package_version: Option<&str>,
        to_package_version: Option<&str>,
        data_version: i64,
        request_id: Option<&str>,
    ) -> Result<PluginBackup, CoreError> {
        if let Some(backup) = self.committed_mutation::<PluginBackup>(request_id)? {
            return Ok(backup);
        }
        if module_id.trim().is_empty() {
            return Err(CoreError::Validation(
                "plugin backup requires a module ID".into(),
            ));
        }
        let root = self.project_root()?.to_path_buf();
        let payload = self.export_json_inner()?.into_bytes();
        let created_at = chrono_like_now();
        let backup_id = Uuid::new_v4().to_string();
        let safe_module = module_id
            .chars()
            .map(|character| {
                if character.is_ascii_alphanumeric() || matches!(character, '.' | '-' | '_') {
                    character
                } else {
                    '_'
                }
            })
            .collect::<String>();
        let relative_path =
            format!(".daena/backups/plugins/plugin-{safe_module}-{created_at}-{backup_id}.json");
        let backup = PluginBackup {
            id: backup_id,
            module_id: module_id.into(),
            from_package_version: from_package_version.map(str::to_owned),
            to_package_version: to_package_version.map(str::to_owned),
            data_version,
            path: root.join(&relative_path).to_string_lossy().into_owned(),
            content_hash: digest_bytes(&payload),
            created_at,
        };
        let request_id = self.request_id(request_id)?;
        let result = serde_json::to_value(&backup)
            .map_err(|error| CoreError::Serialization(error.to_string()))?;
        let transaction = self.begin_mutation(
            &request_id,
            Some(&result),
            std::slice::from_ref(&relative_path),
        )?;
        self.insert_plugin_backup_index_on(&transaction, &backup)?;
        transaction.commit()?;
        let mut exporter = crate::sync::SyncExporter::begin(&root, &request_id)?;
        exporter.stage_bytes(&relative_path, &payload)?;
        exporter.commit(Some(&result))?;
        self.export_latest_generation()?;
        Ok(backup)
    }

    fn insert_plugin_backup_index_on(
        &self,
        transaction: &rusqlite::Transaction<'_>,
        backup: &PluginBackup,
    ) -> Result<(), CoreError> {
        transaction.execute(
            "INSERT OR IGNORE INTO plugin_backups(id,module_id,from_package_version,to_package_version,data_version,path,content_hash,created_at) VALUES (?1,?2,?3,?4,?5,?6,?7,?8)",
            params![backup.id, backup.module_id, backup.from_package_version, backup.to_package_version, backup.data_version, backup.path, backup.content_hash, backup.created_at],
        )?;
        Ok(())
    }

    fn provider_feature_resolution(
        &self,
        map_entity_id: &str,
        feature_kind: Option<&str>,
        feature_id: Option<&str>,
    ) -> &'static str {
        let (Some(feature_kind), Some(feature_id)) = (feature_kind, feature_id) else {
            return "resolved";
        };
        let source: Option<(String, String)> = self.connection.query_row(
            "SELECT a.path,a.content_hash FROM entity_fields f JOIN assets a ON a.id=json_extract(f.value, '$.sourceAssetId') WHERE f.entity_id=?1 AND f.namespace=?2 AND f.key='map'",
            rusqlite::params![map_entity_id, crate::maps::MAP_NAMESPACE], |row| Ok((row.get(0)?, row.get(1)?))).optional().ok().flatten();
        let Some((source_path, content_hash)) = source else {
            return "unresolved";
        };
        let source_path = self
            .root
            .as_ref()
            .and_then(|root| runtime_asset_path(root, &content_hash).ok())
            .unwrap_or_else(|| PathBuf::from(source_path));
        let Ok(bytes) = std::fs::read(source_path) else {
            return "unresolved";
        };
        let Ok(value) = serde_json::from_slice::<serde_json::Value>(&bytes) else {
            return "resolved";
        };
        let Some(features) = value.get("features").and_then(serde_json::Value::as_array) else {
            return "unresolved";
        };
        if features.iter().any(|feature| {
            feature.get("kind").and_then(serde_json::Value::as_str) == Some(feature_kind)
                && feature.get("id").and_then(serde_json::Value::as_str) == Some(feature_id)
        }) {
            "resolved"
        } else {
            "unresolved"
        }
    }

    fn rebuild_maps_projection(&self) -> Result<(), CoreError> {
        let transaction = self.connection.unchecked_transaction()?;
        transaction
            .execute_batch("DELETE FROM map_projection; DELETE FROM map_location_projection;")?;
        {
            let mut maps = transaction.prepare("SELECT e.id, json_extract(f.value, '$.provider.id'), json_extract(f.value, '$.sourceAssetId'), a.path, a.content_hash FROM entities e JOIN entity_fields f ON f.entity_id=e.id AND f.namespace=?1 AND f.key='map' LEFT JOIN assets a ON a.id=json_extract(f.value, '$.sourceAssetId') WHERE e.entity_type='daena.maps:map' AND e.deleted=0")?;
            let rows = maps.query_map([crate::maps::MAP_NAMESPACE], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, Option<String>>(1)?.unwrap_or_default(),
                    row.get::<_, Option<String>>(2)?.unwrap_or_default(),
                    row.get::<_, Option<String>>(3)?,
                    row.get::<_, Option<String>>(4)?,
                ))
            })?;
            for row in rows {
                let (id, provider, asset, path, hash) = row?;
                transaction.execute("INSERT INTO map_projection(map_entity_id,provider,source_asset_id,source_path,source_hash) VALUES (?1,?2,?3,?4,?5)", rusqlite::params![id, provider, asset, path, hash])?;
            }
        }
        {
            let mut locations = transaction.prepare("SELECT f.entity_id, json_each.value FROM entity_fields f, json_each(json_extract(f.value, '$.locations')) WHERE f.namespace=?1 AND f.key='locations'")?;
            let rows = locations.query_map([crate::maps::MAP_NAMESPACE], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })?;
            for row in rows {
                let (entity_id, raw) = row?;
                let location: serde_json::Value = serde_json::from_str(&raw)
                    .map_err(|e| CoreError::Serialization(e.to_string()))?;
                let anchor = location.get("anchor").cloned().unwrap_or_default();
                let kind = anchor
                    .get("kind")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or("unknown");
                let resolution = if kind == "provider-feature" {
                    self.provider_feature_resolution(
                        location
                            .get("mapEntityId")
                            .and_then(serde_json::Value::as_str)
                            .unwrap_or_default(),
                        anchor
                            .get("featureKind")
                            .and_then(serde_json::Value::as_str),
                        anchor.get("featureId").and_then(serde_json::Value::as_str),
                    )
                } else {
                    "resolved"
                };
                write_location_projection(&transaction, &entity_id, &location, resolution)?;
            }
        }
        transaction.commit()?;
        Ok(())
    }

    fn refresh_maps_projection_for_entities(&self, entity_ids: &[String]) -> Result<(), CoreError> {
        let ids = entity_ids.iter().collect::<BTreeSet<_>>();
        for entity_id in &ids {
            self.connection.execute(
                "DELETE FROM map_projection WHERE map_entity_id=?1",
                params![entity_id],
            )?;
            self.connection.execute(
                "DELETE FROM map_location_projection WHERE entity_id=?1 OR map_entity_id=?1",
                params![entity_id],
            )?;
            let map = self.connection.query_row(
                "SELECT e.id,json_extract(f.value,'$.provider.id'),json_extract(f.value,'$.sourceAssetId'),a.path,a.content_hash FROM entities e JOIN entity_fields f ON f.entity_id=e.id AND f.namespace=?1 AND f.key='map' LEFT JOIN assets a ON a.id=json_extract(f.value,'$.sourceAssetId') WHERE e.id=?2 AND e.entity_type='daena.maps:map' AND e.deleted=0",
                params![crate::maps::MAP_NAMESPACE, entity_id],
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, Option<String>>(1)?.unwrap_or_default(), row.get::<_, Option<String>>(2)?.unwrap_or_default(), row.get::<_, Option<String>>(3)?, row.get::<_, Option<String>>(4)?)),
            ).optional()?;
            if let Some((id, provider, asset, path, hash)) = map {
                self.connection.execute(
                    "INSERT INTO map_projection(map_entity_id,provider,source_asset_id,source_path,source_hash) VALUES (?1,?2,?3,?4,?5)",
                    params![id, provider, asset, path, hash],
                )?;
            }
            let mut location_owners = BTreeSet::from([(*entity_id).clone()]);
            let referenced_owners = self
                .connection
                .prepare("SELECT DISTINCT f.entity_id FROM entity_fields f,json_each(json_extract(f.value,'$.locations')) WHERE f.namespace=?1 AND f.key='locations' AND json_extract(json_each.value,'$.mapEntityId')=?2")?
                .query_map(params![crate::maps::MAP_NAMESPACE, entity_id], |row| {
                    row.get::<_, String>(0)
                })?
                .collect::<Result<Vec<_>, _>>()?;
            location_owners.extend(referenced_owners);
            let mut locations = self.connection.prepare(
                "SELECT f.entity_id,json_each.value FROM entity_fields f,json_each(json_extract(f.value,'$.locations')) WHERE f.entity_id=?1 AND f.namespace=?2 AND f.key='locations'",
            )?;
            for owner in location_owners {
                let locations = locations
                    .query_map(params![owner, crate::maps::MAP_NAMESPACE], |row| {
                        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
                    })?
                    .collect::<Result<Vec<_>, _>>()?;
                for (owner, raw) in locations {
                    self.insert_map_location_projection(&owner, &raw)?;
                }
            }
        }
        Ok(())
    }

    fn insert_map_location_projection(&self, entity_id: &str, raw: &str) -> Result<(), CoreError> {
        let location: serde_json::Value = serde_json::from_str(raw)
            .map_err(|error| CoreError::Serialization(error.to_string()))?;
        let anchor = location.get("anchor").cloned().unwrap_or_default();
        let kind = anchor
            .get("kind")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("unknown");
        let resolution = if kind == "provider-feature" {
            self.provider_feature_resolution(
                location
                    .get("mapEntityId")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or_default(),
                anchor
                    .get("featureKind")
                    .and_then(serde_json::Value::as_str),
                anchor.get("featureId").and_then(serde_json::Value::as_str),
            )
        } else {
            "resolved"
        };
        write_location_projection(&self.connection, entity_id, &location, resolution)
    }

    pub fn map_locations_for_entity(
        &self,
        entity_id: String,
    ) -> Result<Vec<serde_json::Value>, CoreError> {
        let mut statement = self.connection.prepare(&format!(
            "{LOCATION_PROJECTION_SELECT} WHERE entity_id=?1 ORDER BY location_id"
        ))?;
        let rows = statement.query_map(rusqlite::params![entity_id], location_projection_json)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(CoreError::from)
    }

    /// Returns the disposable projection rows for all locations on a map.
    /// The projection carries provider selectors, bounds, validity, geometry,
    /// and resolution state; it is rebuilt from the canonical `locations` fields
    /// and the source asset bytes and is never treated as durable state.
    pub fn map_location_projection(
        &self,
        map_entity_id: String,
    ) -> Result<Vec<serde_json::Value>, CoreError> {
        let mut statement = self.connection.prepare(&format!(
            "{LOCATION_PROJECTION_SELECT} WHERE map_entity_id=?1 ORDER BY location_id"
        ))?;
        let rows =
            statement.query_map(rusqlite::params![map_entity_id], location_projection_json)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(CoreError::from)
    }

    /// Returns locations whose stored bounding boxes overlap the query rectangle.
    /// Coordinates are normalized to `[0, 1]`. This reads disposable projection
    /// rows only; canonical geometry remains on entity `locations` fields.
    pub fn query_map_locations(
        &self,
        map_entity_id: String,
        min_x: f64,
        min_y: f64,
        max_x: f64,
        max_y: f64,
    ) -> Result<Vec<serde_json::Value>, CoreError> {
        if ![min_x, min_y, max_x, max_y]
            .into_iter()
            .all(|value| value.is_finite())
            || min_x > max_x
            || min_y > max_y
        {
            return Err(CoreError::Validation(
                "maps: spatial query bounds must be finite with min ≤ max".into(),
            ));
        }
        let mut statement = self.connection.prepare(&format!(
            "{LOCATION_PROJECTION_SELECT} WHERE map_entity_id=?1 AND min_x IS NOT NULL AND max_x IS NOT NULL AND min_y IS NOT NULL AND max_y IS NOT NULL AND min_x <= ?4 AND max_x >= ?2 AND min_y <= ?5 AND max_y >= ?3 ORDER BY location_id"
        ))?;
        let rows = statement.query_map(
            rusqlite::params![map_entity_id, min_x, min_y, max_x, max_y],
            location_projection_json,
        )?;
        rows.collect::<Result<Vec<_>, _>>().map_err(CoreError::from)
    }

    /// Rebuilds the disposable map projections so provider-feature resolution
    /// reflects the current source asset bytes, then returns per-location
    /// resolution results for the given map. Called after every source save so
    /// removed or renumbered FMG features surface as `unresolved` immediately
    /// rather than on the next full index build.
    pub fn reconcile_map_links(
        &self,
        map_entity_id: String,
    ) -> Result<Vec<serde_json::Value>, CoreError> {
        self.rebuild_maps_projection()?;
        let mut statement = self.connection.prepare("SELECT location_id,resolution FROM map_location_projection WHERE map_entity_id=?1 ORDER BY location_id")?;
        let rows = statement.query_map(rusqlite::params![map_entity_id], |row| {
            Ok(serde_json::json!({"locationId": row.get::<_, String>(0)?, "resolved": row.get::<_, String>(1)? == "resolved"}))
        })?;
        rows.collect::<Result<Vec<_>, _>>().map_err(CoreError::from)
    }

    /// Returns the canonical location references owned by an entity.  The
    /// disposable projection is intentionally not used for mutation: the
    /// JSON field remains the source of truth and is rewritten atomically.
    pub fn map_locations(
        &self,
        entity_id: String,
    ) -> Result<Vec<crate::maps::LocationReference>, CoreError> {
        let field = self.list_fields(entity_id)?.into_iter().find(|field| {
            field.namespace == crate::maps::MAP_NAMESPACE && field.key == "locations"
        });
        let Some(field) = field else {
            return Ok(Vec::new());
        };
        let object = field
            .value
            .as_object()
            .ok_or_else(|| CoreError::Serialization("maps.locations is not an object".into()))?;
        let locations = object
            .get("locations")
            .and_then(serde_json::Value::as_array)
            .ok_or_else(|| CoreError::Serialization("maps.locations is not an array".into()))?;
        locations
            .iter()
            .cloned()
            .map(|value| {
                serde_json::from_value(value)
                    .map_err(|error| CoreError::Serialization(error.to_string()))
            })
            .collect()
    }

    pub fn upsert_map_location(
        &self,
        entity_id: String,
        location: crate::maps::LocationReference,
        request_id: Option<&str>,
    ) -> Result<(), CoreError> {
        let mut locations = self.map_locations(entity_id.clone())?;
        if let Some(existing) = locations.iter_mut().find(|item| item.id == location.id) {
            *existing = location;
        } else {
            locations.push(location);
        }
        self.set_field_with_request(
            FieldValue {
                entity_id,
                namespace: crate::maps::MAP_NAMESPACE.into(),
                key: "locations".into(),
                value: serde_json::json!({"schemaVersion": 1, "locations": locations}),
                revision: String::new(),
            },
            request_id,
        )
    }

    pub fn unlink_map_location(
        &self,
        entity_id: String,
        location_id: String,
        request_id: Option<&str>,
    ) -> Result<(), CoreError> {
        let mut locations = self.map_locations(entity_id.clone())?;
        let before = locations.len();
        locations.retain(|location| location.id != location_id);
        if locations.len() == before {
            return Err(CoreError::NotFound("map location not found".into()));
        }
        self.set_field_with_request(
            FieldValue {
                entity_id,
                namespace: crate::maps::MAP_NAMESPACE.into(),
                key: "locations".into(),
                value: serde_json::json!({"schemaVersion": 1, "locations": locations}),
                revision: String::new(),
            },
            request_id,
        )
    }

    pub fn latest_plugin_backup(
        &self,
        module_id: &str,
        from_package_version: Option<&str>,
        to_package_version: Option<&str>,
    ) -> Result<Option<PluginBackup>, CoreError> {
        self.connection
            .query_row(
                "SELECT id,module_id,from_package_version,to_package_version,data_version,path,content_hash,created_at FROM plugin_backups WHERE module_id=?1 AND from_package_version IS ?2 AND to_package_version IS ?3 ORDER BY created_at DESC LIMIT 1",
                params![module_id, from_package_version, to_package_version],
                |row| {
                    Ok(PluginBackup {
                        id: row.get(0)?,
                        module_id: row.get(1)?,
                        from_package_version: row.get(2)?,
                        to_package_version: row.get(3)?,
                        data_version: row.get(4)?,
                        path: row.get(5)?,
                        content_hash: row.get(6)?,
                        created_at: row.get(7)?,
                    })
                },
            )
            .optional()
            .map_err(CoreError::from)
    }

    pub fn restore_plugin_backup(&mut self, backup: &PluginBackup) -> Result<(), CoreError> {
        self.restore_plugin_backup_with_request(backup, None)
    }

    pub fn restore_plugin_backup_with_request(
        &mut self,
        backup: &PluginBackup,
        request_id: Option<&str>,
    ) -> Result<(), CoreError> {
        let payload = std::fs::read(&backup.path)
            .map_err(|error| CoreError::NotFound(format!("read plugin backup: {error}")))?;
        if digest_bytes(&payload) != backup.content_hash {
            return Err(CoreError::Validation(
                "plugin backup integrity check failed".into(),
            ));
        }
        self.restore_payload_with_request(
            std::str::from_utf8(&payload).map_err(|error| {
                CoreError::Validation(format!("plugin backup is not UTF-8: {error}"))
            })?,
            request_id,
        )
    }

    pub fn restore(&mut self, path: String) -> Result<(), CoreError> {
        let path_ref = Path::new(&path);
        if path_ref.is_dir() {
            let canonical = crate::storage::FilesystemRepository::open(path_ref)?.scan()?;
            let payload = serde_json::to_string(&canonical.snapshot)
                .map_err(|error| CoreError::Serialization(error.to_string()))?;
            self.restore_payload(&payload)?;
            return Ok(());
        }
        let content =
            std::fs::read_to_string(&path).map_err(|e| CoreError::NotFound(e.to_string()))?;
        self.restore_payload(&content)?;
        Ok(())
    }

    pub fn restore_payload(&mut self, payload: &str) -> Result<(), CoreError> {
        self.restore_payload_with_request(payload, None)?;
        Ok(())
    }

    pub fn restore_payload_with_request(
        &mut self,
        payload: &str,
        request_id: Option<&str>,
    ) -> Result<(), CoreError> {
        if self
            .committed_mutation::<serde_json::Value>(request_id)?
            .is_some()
        {
            return Ok(());
        }
        if self.root.is_some() && self.runtime_requires_recovery_archive()? {
            let root = self.project_root()?.to_path_buf();
            self.recovery_backup_to(root.join(".daena/backups"))?;
        }
        self.import_json_with_mode_and_sync_with_request(payload, true, true, request_id)?;
        Ok(())
    }

    /// Remove only data owned by a plugin. Code uninstall and disablement do
    /// not call this method; callers must present the explicit confirmation
    /// phrase and a backup is created before the destructive transaction.
    pub fn create_module_record(
        &self,
        module_id: &str,
        collection: &str,
        owner_entity_id: &str,
        value: serde_json::Value,
        request_id: Option<&str>,
    ) -> Result<ModuleRecord, CoreError> {
        validate_module_record_input(module_id, collection, owner_entity_id, &value)?;
        let fingerprint = digest_bytes(
            &serde_json::to_vec(&(module_id, collection, owner_entity_id, &value))
                .map_err(|error| CoreError::Serialization(error.to_string()))?,
        );
        if let Some(mut record) = self.committed_mutation_with_fingerprint::<ModuleRecord>(
            request_id,
            Some(&fingerprint),
        )? {
            record.revision = self.revision_for_module_record_value(&record)?;
            return Ok(record);
        }
        self.ensure_live_module_record_owner(owner_entity_id)?;
        let id = Uuid::new_v4().to_string();
        let now = chrono_like_now();
        let encoded = serde_json::to_string(&value)
            .map_err(|error| CoreError::Serialization(error.to_string()))?;
        let result = ModuleRecord {
            module_id: module_id.into(),
            collection: collection.into(),
            id: id.clone(),
            owner_entity_id: owner_entity_id.into(),
            value: value.clone(),
            created_at: now.clone(),
            updated_at: now.clone(),
            revision: String::new(),
        };
        let request_id = self.request_id(request_id)?;
        let transaction = self.begin_mutation_with_fingerprint(
            &request_id,
            Some(&serde_json::to_value(&result)?),
            &[format!("plugins/{module_id}.json")],
            &fingerprint,
        )?;
        transaction.execute(
            "INSERT INTO module_records(module_id,collection,id,owner_entity_id,value,created_at,updated_at) VALUES (?1,?2,?3,?4,?5,?6,?6)",
            params![module_id, collection, id, owner_entity_id, encoded, now],
        )?;
        transaction.commit()?;
        self.notify_export_worker()?;
        let mut record = result;
        record.revision = self.revision_for_module_record(&record.id)?;
        Ok(record)
    }

    pub fn list_module_records(
        &self,
        module_id: &str,
        collection: &str,
        owner_entity_id: &str,
        query: Option<&str>,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<ModuleRecord>, CoreError> {
        self.list_module_records_with(
            module_id,
            collection,
            owner_entity_id,
            ModuleRecordListParams {
                query,
                limit,
                offset,
                ..ModuleRecordListParams::default()
            },
        )
    }

    pub fn list_module_records_with(
        &self,
        module_id: &str,
        collection: &str,
        owner_entity_id: &str,
        params: ModuleRecordListParams<'_>,
    ) -> Result<Vec<ModuleRecord>, CoreError> {
        validate_module_record_scope(module_id, collection, owner_entity_id)?;
        self.ensure_live_module_record_owner(owner_entity_id)?;
        let limit = params.limit.clamp(1, 100) as i64;
        let offset = i64::try_from(params.offset)
            .map_err(|_| CoreError::Validation("record offset is too large".into()))?;
        let query = params.query.unwrap_or_default().trim();
        if query.len() > 200 {
            return Err(CoreError::Validation(
                "record search query exceeds 200 bytes".into(),
            ));
        }
        let status = optional_record_filter(params.status, "record status")?;
        let tag = optional_record_filter(params.tag, "record tag")?;
        let order = module_record_order_sql(params.sort.unwrap_or("lemma"), if query.is_empty() {
            ""
        } else {
            "r."
        })?;
        let filters = module_record_filter_sql(if query.is_empty() { "" } else { "r." });
        let sql = if query.is_empty() {
            format!(
                "SELECT module_id,collection,id,owner_entity_id,value,created_at,updated_at FROM module_records WHERE module_id=:module AND collection=:collection AND owner_entity_id=:owner {filters} ORDER BY {order} LIMIT :limit OFFSET :offset"
            )
        } else {
            format!(
                "SELECT r.module_id,r.collection,r.id,r.owner_entity_id,r.value,r.created_at,r.updated_at FROM module_records r JOIN module_record_search s ON s.record_id=r.id WHERE s.module_id=:module AND s.collection=:collection AND s.owner_entity_id=:owner AND module_record_search MATCH :terms {filters} ORDER BY {order} LIMIT :limit OFFSET :offset"
            )
        };
        let terms = query
            .split_whitespace()
            .map(|term| format!("\"{}\"*", term.replace('"', "")))
            .collect::<Vec<_>>()
            .join(" AND ");
        let homonyms = i64::from(params.homonyms_only);
        let mut statement = self.connection.prepare(&sql)?;
        let read_record = |row: &rusqlite::Row<'_>| {
                let encoded: String = row.get(4)?;
                Ok(ModuleRecord {
                    module_id: row.get(0)?,
                    collection: row.get(1)?,
                    id: row.get(2)?,
                    owner_entity_id: row.get(3)?,
                    value: decode_field_value(encoded),
                    created_at: row.get(5)?,
                    updated_at: row.get(6)?,
                    revision: String::new(),
                })
            };
        let rows = if query.is_empty() {
            statement.query_map(
                named_params! {
                    ":module": module_id,
                    ":collection": collection,
                    ":owner": owner_entity_id,
                    ":status": status,
                    ":tag": tag,
                    ":homonyms": homonyms,
                    ":limit": limit,
                    ":offset": offset,
                },
                read_record,
            )?
        } else {
            statement.query_map(
                named_params! {
                    ":module": module_id,
                    ":collection": collection,
                    ":owner": owner_entity_id,
                    ":terms": terms,
                    ":status": status,
                    ":tag": tag,
                    ":homonyms": homonyms,
                    ":limit": limit,
                    ":offset": offset,
                },
                read_record,
            )?
        };
        let mut records = rows.collect::<Result<Vec<_>, _>>()?;
        for record in &mut records {
            record.revision = self.revision_for_module_record_value(record)?;
        }
        Ok(records)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn update_module_record(
        &self,
        module_id: &str,
        collection: &str,
        id: &str,
        owner_entity_id: &str,
        value: serde_json::Value,
        expected_revision: &str,
        request_id: Option<&str>,
    ) -> Result<ModuleRecord, CoreError> {
        validate_module_record_input(module_id, collection, owner_entity_id, &value)?;
        Uuid::parse_str(id)
            .map_err(|_| CoreError::Validation("module record ID must be a UUID".into()))?;
        let fingerprint = digest_bytes(
            &serde_json::to_vec(&(
                module_id,
                collection,
                id,
                owner_entity_id,
                &value,
                expected_revision,
            ))
            .map_err(|error| CoreError::Serialization(error.to_string()))?,
        );
        if let Some(mut record) = self.committed_mutation_with_fingerprint::<ModuleRecord>(
            request_id,
            Some(&fingerprint),
        )? {
            record.revision = self.revision_for_module_record_value(&record)?;
            return Ok(record);
        }
        let current = self.module_record(module_id, collection, id, owner_entity_id)?;
        Self::ensure_expected_revision(Some(expected_revision), current.revision, "module record")?;
        let now = chrono_like_now();
        let encoded = serde_json::to_string(&value)
            .map_err(|error| CoreError::Serialization(error.to_string()))?;
        let result = ModuleRecord {
            value: value.clone(),
            updated_at: now.clone(),
            revision: String::new(),
            ..current
        };
        let request_id = self.request_id(request_id)?;
        let transaction = self.begin_mutation_with_fingerprint(
            &request_id,
            Some(&serde_json::to_value(&result)?),
            &[format!("plugins/{module_id}.json")],
            &fingerprint,
        )?;
        transaction.execute(
            "UPDATE module_records SET value=?1,updated_at=?2 WHERE id=?3 AND module_id=?4 AND collection=?5 AND owner_entity_id=?6",
            params![encoded, now, id, module_id, collection, owner_entity_id],
        )?;
        transaction.commit()?;
        self.notify_export_worker()?;
        let mut record = result;
        record.revision = self.revision_for_module_record(id)?;
        Ok(record)
    }

    pub fn delete_module_record(
        &self,
        module_id: &str,
        collection: &str,
        id: &str,
        owner_entity_id: &str,
        expected_revision: &str,
        request_id: Option<&str>,
    ) -> Result<(), CoreError> {
        validate_module_record_scope(module_id, collection, owner_entity_id)?;
        Uuid::parse_str(id)
            .map_err(|_| CoreError::Validation("module record ID must be a UUID".into()))?;
        let fingerprint = digest_bytes(
            &serde_json::to_vec(&(module_id, collection, id, owner_entity_id, expected_revision))
                .map_err(|error| CoreError::Serialization(error.to_string()))?,
        );
        if self
            .committed_mutation_with_fingerprint::<serde_json::Value>(
                request_id,
                Some(&fingerprint),
            )?
            .is_some()
        {
            return Ok(());
        }
        let current = self.module_record(module_id, collection, id, owner_entity_id)?;
        Self::ensure_expected_revision(Some(expected_revision), current.revision, "module record")?;
        let request_id = self.request_id(request_id)?;
        let transaction = self.begin_mutation_with_fingerprint(
            &request_id,
            Some(&serde_json::Value::Null),
            &[format!("plugins/{module_id}.json")],
            &fingerprint,
        )?;
        transaction.execute(
            "DELETE FROM module_records WHERE id=?1 AND module_id=?2 AND collection=?3 AND owner_entity_id=?4",
            params![id, module_id, collection, owner_entity_id],
        )?;
        transaction.commit()?;
        self.notify_export_worker()?;
        Ok(())
    }

    fn module_record(
        &self,
        module_id: &str,
        collection: &str,
        id: &str,
        owner_entity_id: &str,
    ) -> Result<ModuleRecord, CoreError> {
        validate_module_record_scope(module_id, collection, owner_entity_id)?;
        let mut record = self
            .connection
            .query_row(
                "SELECT module_id,collection,id,owner_entity_id,value,created_at,updated_at FROM module_records WHERE id=?1 AND module_id=?2 AND collection=?3 AND owner_entity_id=?4",
                params![id, module_id, collection, owner_entity_id],
                |row| {
                    let encoded: String = row.get(4)?;
                    Ok(ModuleRecord {
                        module_id: row.get(0)?,
                        collection: row.get(1)?,
                        id: row.get(2)?,
                        owner_entity_id: row.get(3)?,
                        value: decode_field_value(encoded),
                        created_at: row.get(5)?,
                        updated_at: row.get(6)?,
                        revision: String::new(),
                    })
                },
            )
            .optional()?
            .ok_or_else(|| CoreError::NotFound("module record not found".into()))?;
        record.revision = self.revision_for_module_record(&record.id)?;
        Ok(record)
    }

    fn ensure_live_module_record_owner(&self, owner_entity_id: &str) -> Result<(), CoreError> {
        let exists = self
            .connection
            .query_row(
                "SELECT 1 FROM entities WHERE id=?1 AND deleted=0",
                params![owner_entity_id],
                |_| Ok(()),
            )
            .optional()?;
        if exists.is_none() {
            return Err(CoreError::NotFound(
                "module record owner entity not found".into(),
            ));
        }
        Ok(())
    }

    pub fn delete_plugin_data(
        &self,
        plugin_id: &str,
        confirmation: &str,
    ) -> Result<String, CoreError> {
        self.delete_plugin_data_with_request(plugin_id, confirmation, None)
    }

    pub fn delete_plugin_data_with_request(
        &self,
        plugin_id: &str,
        confirmation: &str,
        request_id: Option<&str>,
    ) -> Result<String, CoreError> {
        if let Some(backup) = self.committed_mutation::<String>(request_id)? {
            return Ok(backup);
        }
        if plugin_id.trim().is_empty() || confirmation != plugin_id {
            return Err(CoreError::Unauthorized {
                operation: "confirm plugin data deletion",
            });
        }
        let backup = self.backup()?;
        let request_id = self.request_id(request_id)?;
        let result = serde_json::to_value(&backup)?;
        let transaction = self.begin_mutation(
            &request_id,
            Some(&result),
            &["entities/".into(), "plugins/".into(), "assets/".into()],
        )?;
        transaction.execute(
            "DELETE FROM entity_fields WHERE namespace IN (SELECT namespace FROM module_namespaces WHERE module_id=?1)",
            params![plugin_id],
        )?;
        transaction.execute(
            "DELETE FROM assets WHERE namespace IN (SELECT namespace FROM module_namespaces WHERE module_id=?1)",
            params![plugin_id],
        )?;
        transaction.execute(
            "DELETE FROM module_fields WHERE module_id=?1",
            params![plugin_id],
        )?;
        transaction.execute(
            "DELETE FROM module_records WHERE module_id=?1",
            params![plugin_id],
        )?;
        transaction.execute(
            "DELETE FROM module_namespaces WHERE module_id=?1",
            params![plugin_id],
        )?;
        transaction.execute(
            "DELETE FROM module_versions WHERE module_id=?1",
            params![plugin_id],
        )?;
        transaction.execute(
            "DELETE FROM module_state WHERE module_id=?1",
            params![plugin_id],
        )?;
        transaction.execute(
            "DELETE FROM module_package_versions WHERE module_id=?1",
            params![plugin_id],
        )?;
        transaction.execute(
            "DELETE FROM migration_history WHERE module_id=?1",
            params![plugin_id],
        )?;
        transaction.commit()?;
        self.notify_export_worker()?;
        Ok(backup)
    }

    pub fn rebuild_search(&self) -> Result<(), CoreError> {
        self.connection.execute_batch(
            "DROP TRIGGER IF EXISTS entities_search_insert;
             DROP TRIGGER IF EXISTS entities_search_update;
             DROP TRIGGER IF EXISTS documents_search_insert;
             DROP TRIGGER IF EXISTS documents_search_update;
             DROP TRIGGER IF EXISTS documents_search_delete;
             DROP TRIGGER IF EXISTS entity_fields_search_insert;
             DROP TRIGGER IF EXISTS entity_fields_search_update;
             DROP TRIGGER IF EXISTS entity_fields_search_delete;
             DROP TRIGGER IF EXISTS module_records_search_insert;
             DROP TRIGGER IF EXISTS module_records_search_update;
             DROP TRIGGER IF EXISTS module_records_search_delete;
             DROP TABLE IF EXISTS world_search;
             DROP TABLE IF EXISTS module_record_search;
             CREATE VIRTUAL TABLE world_search USING fts5(entity_id UNINDEXED, source_path UNINDEXED, source_hash UNINDEXED, content, source_key UNINDEXED);
             INSERT INTO world_search(entity_id,source_path,source_hash,content,source_key) SELECT e.id,'entities/' || e.id || '/entity.json','',e.name || ' ' || COALESCE(e.entity_type,''),'entity' FROM entities e WHERE e.deleted=0;
             INSERT INTO world_search(entity_id,source_path,source_hash,content,source_key) SELECT d.entity_id,'entities/' || d.entity_id || '/document.md','',d.body,'document:' || d.id FROM documents d JOIN entities e ON e.id=d.entity_id WHERE e.deleted=0;
             INSERT INTO world_search(entity_id,source_path,source_hash,content,source_key) SELECT f.entity_id,'entities/' || f.entity_id || '/fields','',f.namespace || ' ' || f.key || ' ' || f.value,'field:' || f.namespace || '/' || f.key FROM entity_fields f JOIN entities e ON e.id=f.entity_id WHERE e.deleted=0;
             CREATE TRIGGER entities_search_insert AFTER INSERT ON entities BEGIN INSERT INTO world_search(entity_id,source_path,source_hash,content,source_key) SELECT new.id,'entities/' || new.id || '/entity.json','',new.name || ' ' || COALESCE(new.entity_type,''),'entity' WHERE new.deleted=0; END;
             CREATE TRIGGER entities_search_update AFTER UPDATE OF name,entity_type,deleted ON entities BEGIN DELETE FROM world_search WHERE entity_id=old.id; INSERT INTO world_search(entity_id,source_path,source_hash,content,source_key) SELECT new.id,'entities/' || new.id || '/entity.json','',new.name || ' ' || COALESCE(new.entity_type,''),'entity' WHERE new.deleted=0; INSERT INTO world_search(entity_id,source_path,source_hash,content,source_key) SELECT d.entity_id,'entities/' || d.entity_id || '/document.md','',d.body,'document:' || d.id FROM documents d WHERE d.entity_id=new.id AND new.deleted=0; INSERT INTO world_search(entity_id,source_path,source_hash,content,source_key) SELECT f.entity_id,'entities/' || f.entity_id || '/fields','',f.namespace || ' ' || f.key || ' ' || f.value,'field:' || f.namespace || '/' || f.key FROM entity_fields f WHERE f.entity_id=new.id AND new.deleted=0; END;
             CREATE TRIGGER documents_search_insert AFTER INSERT ON documents BEGIN INSERT INTO world_search(entity_id,source_path,source_hash,content,source_key) SELECT new.entity_id,'entities/' || new.entity_id || '/document.md','',new.body,'document:' || new.id FROM entities WHERE id=new.entity_id AND deleted=0; END;
             CREATE TRIGGER documents_search_update AFTER UPDATE OF body ON documents BEGIN DELETE FROM world_search WHERE entity_id=old.entity_id AND source_key='document:' || old.id; INSERT INTO world_search(entity_id,source_path,source_hash,content,source_key) SELECT new.entity_id,'entities/' || new.entity_id || '/document.md','',new.body,'document:' || new.id FROM entities WHERE id=new.entity_id AND deleted=0; END;
             CREATE TRIGGER documents_search_delete AFTER DELETE ON documents BEGIN DELETE FROM world_search WHERE entity_id=old.entity_id AND source_key='document:' || old.id; END;
             CREATE TRIGGER entity_fields_search_insert AFTER INSERT ON entity_fields BEGIN INSERT INTO world_search(entity_id,source_path,source_hash,content,source_key) SELECT new.entity_id,'entities/' || new.entity_id || '/fields','',new.namespace || ' ' || new.key || ' ' || new.value,'field:' || new.namespace || '/' || new.key FROM entities WHERE id=new.entity_id AND deleted=0; END;
             CREATE TRIGGER entity_fields_search_update AFTER UPDATE OF namespace,key,value ON entity_fields BEGIN DELETE FROM world_search WHERE entity_id=old.entity_id AND source_key='field:' || old.namespace || '/' || old.key; INSERT INTO world_search(entity_id,source_path,source_hash,content,source_key) SELECT new.entity_id,'entities/' || new.entity_id || '/fields','',new.namespace || ' ' || new.key || ' ' || new.value,'field:' || new.namespace || '/' || new.key FROM entities WHERE id=new.entity_id AND deleted=0; END;
             CREATE TRIGGER entity_fields_search_delete AFTER DELETE ON entity_fields BEGIN DELETE FROM world_search WHERE entity_id=old.entity_id AND source_key='field:' || old.namespace || '/' || old.key; END;"
        )?;
        self.connection.execute_batch(
            "CREATE VIRTUAL TABLE module_record_search USING fts5(module_id UNINDEXED, collection UNINDEXED, owner_entity_id UNINDEXED, record_id UNINDEXED, content);
             INSERT INTO module_record_search(module_id,collection,owner_entity_id,record_id,content) SELECT module_id,collection,owner_entity_id,id,value FROM module_records;
             CREATE TRIGGER module_records_search_insert AFTER INSERT ON module_records BEGIN INSERT INTO module_record_search(module_id,collection,owner_entity_id,record_id,content) VALUES (new.module_id,new.collection,new.owner_entity_id,new.id,new.value); END;
             CREATE TRIGGER module_records_search_update AFTER UPDATE ON module_records BEGIN DELETE FROM module_record_search WHERE record_id=old.id; INSERT INTO module_record_search(module_id,collection,owner_entity_id,record_id,content) VALUES (new.module_id,new.collection,new.owner_entity_id,new.id,new.value); END;
             CREATE TRIGGER module_records_search_delete AFTER DELETE ON module_records BEGIN DELETE FROM module_record_search WHERE record_id=old.id; END;",
        )?;
        self.rebuild_maps_projection()?;
        Ok(())
    }

    pub fn seed_example(&mut self) -> Result<usize, CoreError> {
        if self.root.is_none() {
            return self.seed_example_unchecked();
        }
        self.suppress_sync.set(true);
        let result = self.seed_example_unchecked();
        self.suppress_sync.set(false);
        result.and_then(|count| {
            self.notify_export_worker()?;
            Ok(count)
        })
    }

    fn seed_example_unchecked(&mut self) -> Result<usize, CoreError> {
        let tx = self.connection.transaction()?;
        tx.execute("DELETE FROM assets", [])?;
        tx.execute("DELETE FROM entity_fields", [])?;
        tx.execute("DELETE FROM documents", [])?;
        tx.execute("DELETE FROM relationships", [])?;
        tx.execute("DELETE FROM module_records", [])?;
        tx.execute("DELETE FROM map_projection", [])?;
        tx.execute("DELETE FROM map_location_projection", [])?;
        tx.execute("DELETE FROM entities", [])?;
        let now = chrono_like_now();
        let eldermere_id = Uuid::new_v4().to_string();
        tx.execute("INSERT INTO entities(id,name,entity_type,deleted,created_at,updated_at) VALUES (?1,?2,?3,0,?4,?4)", params![eldermere_id, "Eldermere", "place", now])?;
        let lord_ashford_id = Uuid::new_v4().to_string();
        tx.execute("INSERT INTO entities(id,name,entity_type,deleted,created_at,updated_at) VALUES (?1,?2,?3,0,?4,?4)", params![lord_ashford_id, "Lord Ashford", "person", now])?;
        let glass_coast_id = Uuid::new_v4().to_string();
        tx.execute("INSERT INTO entities(id,name,entity_type,deleted,created_at,updated_at) VALUES (?1,?2,?3,0,?4,?4)", params![glass_coast_id, "The Glass Coast", "place", now])?;
        let silver_hand_id = Uuid::new_v4().to_string();
        tx.execute("INSERT INTO entities(id,name,entity_type,deleted,created_at,updated_at) VALUES (?1,?2,?3,0,?4,?4)", params![silver_hand_id, "The Silver Hand", "faction", now])?;
        let amulet_id = Uuid::new_v4().to_string();
        tx.execute("INSERT INTO entities(id,name,entity_type,deleted,created_at,updated_at) VALUES (?1,?2,?3,0,?4,?4)", params![amulet_id, "Amulet of Tides", "artifact", now])?;
        let highland_id = Uuid::new_v4().to_string();
        tx.execute("INSERT INTO entities(id,name,entity_type,deleted,created_at,updated_at) VALUES (?1,?2,?3,0,?4,?4)", params![highland_id, "Highland Culture", "culture", now])?;
        let founding_id = Uuid::new_v4().to_string();
        tx.execute("INSERT INTO entities(id,name,entity_type,deleted,created_at,updated_at) VALUES (?1,?2,?3,0,?4,?4)", params![founding_id, "Founding of Eldermere", "event", now])?;
        let treaty_id = Uuid::new_v4().to_string();
        tx.execute("INSERT INTO entities(id,name,entity_type,deleted,created_at,updated_at) VALUES (?1,?2,?3,0,?4,?4)", params![treaty_id, "The Treaty of Ashes", "event", now])?;
        let rebellion_id = Uuid::new_v4().to_string();
        tx.execute("INSERT INTO entities(id,name,entity_type,deleted,created_at,updated_at) VALUES (?1,?2,?3,0,?4,?4)", params![rebellion_id, "The Tide Rebellion", "event", now])?;
        let mira_vale_id = Uuid::new_v4().to_string();
        tx.execute("INSERT INTO entities(id,name,entity_type,deleted,created_at,updated_at) VALUES (?1,?2,?3,0,?4,?4)", params![mira_vale_id, "Mira Vale", "person", now])?;
        let sunken_archive_id = Uuid::new_v4().to_string();
        tx.execute("INSERT INTO entities(id,name,entity_type,deleted,created_at,updated_at) VALUES (?1,?2,?3,0,?4,?4)", params![sunken_archive_id, "The Sunken Archive", "place", now])?;
        let ember_court_id = Uuid::new_v4().to_string();
        tx.execute("INSERT INTO entities(id,name,entity_type,deleted,created_at,updated_at) VALUES (?1,?2,?3,0,?4,?4)", params![ember_court_id, "The Ember Court", "faction", now])?;
        let star_compass_id = Uuid::new_v4().to_string();
        tx.execute("INSERT INTO entities(id,name,entity_type,deleted,created_at,updated_at) VALUES (?1,?2,?3,0,?4,?4)", params![star_compass_id, "Star Compass", "artifact", now])?;
        let riverborn_id = Uuid::new_v4().to_string();
        tx.execute("INSERT INTO entities(id,name,entity_type,deleted,created_at,updated_at) VALUES (?1,?2,?3,0,?4,?4)", params![riverborn_id, "Riverborn Culture", "culture", now])?;
        let war_embers_id = Uuid::new_v4().to_string();
        tx.execute("INSERT INTO entities(id,name,entity_type,deleted,created_at,updated_at) VALUES (?1,?2,?3,0,?4,?4)", params![war_embers_id, "The War of Embers", "event", now])?;
        let first_tide_id = Uuid::new_v4().to_string();
        tx.execute("INSERT INTO entities(id,name,entity_type,deleted,created_at,updated_at) VALUES (?1,?2,?3,0,?4,?4)", params![first_tide_id, "The First Tide", "event", now])?;
        let archive_opening_id = Uuid::new_v4().to_string();
        tx.execute("INSERT INTO entities(id,name,entity_type,deleted,created_at,updated_at) VALUES (?1,?2,?3,0,?4,?4)", params![archive_opening_id, "Opening of the Sunken Archive", "event", now])?;
        let frostgate_id = Uuid::new_v4().to_string();
        tx.execute("INSERT INTO entities(id,name,entity_type,deleted,created_at,updated_at) VALUES (?1,?2,?3,0,?4,?4)", params![frostgate_id, "Frostgate Pass", "place", now])?;
        let lantern_marsh_id = Uuid::new_v4().to_string();
        tx.execute("INSERT INTO entities(id,name,entity_type,deleted,created_at,updated_at) VALUES (?1,?2,?3,0,?4,?4)", params![lantern_marsh_id, "Lantern Marsh", "place", now])?;
        let elian_rook_id = Uuid::new_v4().to_string();
        tx.execute("INSERT INTO entities(id,name,entity_type,deleted,created_at,updated_at) VALUES (?1,?2,?3,0,?4,?4)", params![elian_rook_id, "Captain Elian Rook", "person", now])?;
        let sera_ashdown_id = Uuid::new_v4().to_string();
        tx.execute("INSERT INTO entities(id,name,entity_type,deleted,created_at,updated_at) VALUES (?1,?2,?3,0,?4,?4)", params![sera_ashdown_id, "Sera Ashdown", "person", now])?;
        let tidewatch_id = Uuid::new_v4().to_string();
        tx.execute("INSERT INTO entities(id,name,entity_type,deleted,created_at,updated_at) VALUES (?1,?2,?3,0,?4,?4)", params![tidewatch_id, "The Tidewatch", "faction", now])?;
        let crown_salt_id = Uuid::new_v4().to_string();
        tx.execute("INSERT INTO entities(id,name,entity_type,deleted,created_at,updated_at) VALUES (?1,?2,?3,0,?4,?4)", params![crown_salt_id, "Crown of Salt", "artifact", now])?;
        let coastfolk_id = Uuid::new_v4().to_string();
        tx.execute("INSERT INTO entities(id,name,entity_type,deleted,created_at,updated_at) VALUES (?1,?2,?3,0,?4,?4)", params![coastfolk_id, "Coastfolk Culture", "culture", now])?;
        let frostgate_battle_id = Uuid::new_v4().to_string();
        tx.execute("INSERT INTO entities(id,name,entity_type,deleted,created_at,updated_at) VALUES (?1,?2,?3,0,?4,?4)", params![frostgate_battle_id, "The Battle of Frostgate", "event", now])?;
        tx.execute("INSERT INTO documents(id,entity_id,format,body,updated_at) VALUES (?1,?2,?3,?4,?5)", params![Uuid::new_v4().to_string(), eldermere_id, "markdown", "Eldermere is the ancient seat of power, a coastal fortress built upon the cliffs where the river meets the sea.", now])?;
        tx.execute(
            "INSERT INTO documents(id,entity_id,format,body,updated_at) VALUES (?1,?2,?3,?4,?5)",
            params![
                Uuid::new_v4().to_string(),
                lord_ashford_id,
                "markdown",
                "Lord Ashford rules Eldermere with a steady hand and a sharp mind.",
                now
            ],
        )?;
        tx.execute("INSERT INTO documents(id,entity_id,format,body,updated_at) VALUES (?1,?2,?3,?4,?5)", params![Uuid::new_v4().to_string(), glass_coast_id, "markdown", "The Glass Coast is a stretch of shoreline where the sea glass glitters like scattered jewels.", now])?;
        tx.execute(
            "INSERT INTO documents(id,entity_id,format,body,updated_at) VALUES (?1,?2,?3,?4,?5)",
            params![
                Uuid::new_v4().to_string(),
                silver_hand_id,
                "markdown",
                "The Silver Hand is a sworn brotherhood dedicated to the protection of Eldermere.",
                now
            ],
        )?;
        tx.execute(
            "INSERT INTO documents(id,entity_id,format,body,updated_at) VALUES (?1,?2,?3,?4,?5)",
            params![
                Uuid::new_v4().to_string(),
                amulet_id,
                "markdown",
                "The Amulet of Tides grants its bearer the ability to breathe beneath the waves.",
                now
            ],
        )?;
        tx.execute(
            "INSERT INTO documents(id,entity_id,format,body,updated_at) VALUES (?1,?2,?3,?4,?5)",
            params![
                Uuid::new_v4().to_string(),
                highland_id,
                "markdown",
                "The Highland Culture values honor, craftsmanship, and the old songs.",
                now
            ],
        )?;
        tx.execute("INSERT INTO documents(id,entity_id,format,body,updated_at) VALUES (?1,?2,?3,?4,?5)", params![Uuid::new_v4().to_string(), founding_id, "markdown", "The founding of Eldermere marked the unification of the coastal clans under one banner.", now])?;
        tx.execute("INSERT INTO documents(id,entity_id,format,body,updated_at) VALUES (?1,?2,?3,?4,?5)", params![Uuid::new_v4().to_string(), treaty_id, "markdown", "The Treaty of Ashes ended the War of Embers and established the borders of the realm.", now])?;
        tx.execute(
            "INSERT INTO documents(id,entity_id,format,body,updated_at) VALUES (?1,?2,?3,?4,?5)",
            params![
                Uuid::new_v4().to_string(),
                rebellion_id,
                "markdown",
                "The Tide Rebellion was a brief but violent uprising against the crown.",
                now
            ],
        )?;
        tx.execute("INSERT INTO documents(id,entity_id,format,body,updated_at) VALUES (?1,?2,?3,?4,?5)", params![Uuid::new_v4().to_string(), mira_vale_id, "markdown", "Mira Vale is a mapmaker who can read the coast by moonlight and remembers every vanished inlet.", now])?;
        tx.execute("INSERT INTO documents(id,entity_id,format,body,updated_at) VALUES (?1,?2,?3,?4,?5)", params![Uuid::new_v4().to_string(), sunken_archive_id, "markdown", "The Sunken Archive lies beneath the old river delta, its sealed chambers holding histories the crown chose to forget.", now])?;
        tx.execute("INSERT INTO documents(id,entity_id,format,body,updated_at) VALUES (?1,?2,?3,?4,?5)", params![Uuid::new_v4().to_string(), ember_court_id, "markdown", "The Ember Court preserves the old fire rites and quietly contests the Silver Hand's claim to protect the realm.", now])?;
        tx.execute(
            "INSERT INTO documents(id,entity_id,format,body,updated_at) VALUES (?1,?2,?3,?4,?5)",
            params![
                Uuid::new_v4().to_string(),
                star_compass_id,
                "markdown",
                "The Star Compass points toward what has been lost rather than toward north.",
                now
            ],
        )?;
        tx.execute("INSERT INTO documents(id,entity_id,format,body,updated_at) VALUES (?1,?2,?3,?4,?5)", params![Uuid::new_v4().to_string(), riverborn_id, "markdown", "The Riverborn Culture follows the delta's shifting channels and keeps songs for every flood season.", now])?;
        tx.execute(
            "INSERT INTO documents(id,entity_id,format,body,updated_at) VALUES (?1,?2,?3,?4,?5)",
            params![
                Uuid::new_v4().to_string(),
                war_embers_id,
                "markdown",
                "The War of Embers began when the frontier forges refused the crown's new tribute.",
                now
            ],
        )?;
        tx.execute("INSERT INTO documents(id,entity_id,format,body,updated_at) VALUES (?1,?2,?3,?4,?5)", params![Uuid::new_v4().to_string(), first_tide_id, "markdown", "The First Tide is remembered as the night the sea crossed the old boundary stones.", now])?;
        tx.execute("INSERT INTO documents(id,entity_id,format,body,updated_at) VALUES (?1,?2,?3,?4,?5)", params![Uuid::new_v4().to_string(), archive_opening_id, "markdown", "Mira Vale opened the upper vault of the Sunken Archive and found a map bearing the Star Compass mark.", now])?;
        tx.execute("INSERT INTO documents(id,entity_id,format,body,updated_at) VALUES (?1,?2,?3,?4,?5)", params![Uuid::new_v4().to_string(), frostgate_id, "markdown", "Frostgate Pass is the only safe road through the northern cliffs, guarded by old watchtowers and a newer border wall.", now])?;
        tx.execute("INSERT INTO documents(id,entity_id,format,body,updated_at) VALUES (?1,?2,?3,?4,?5)", params![Uuid::new_v4().to_string(), lantern_marsh_id, "markdown", "Lantern Marsh glows with blue reeds after dusk; its hidden channels lead from the delta to the western sea.", now])?;
        tx.execute("INSERT INTO documents(id,entity_id,format,body,updated_at) VALUES (?1,?2,?3,?4,?5)", params![Uuid::new_v4().to_string(), elian_rook_id, "markdown", "Captain Elian Rook commands the coast patrols and distrusts any map that has not survived a flood season.", now])?;
        tx.execute("INSERT INTO documents(id,entity_id,format,body,updated_at) VALUES (?1,?2,?3,?4,?5)", params![Uuid::new_v4().to_string(), sera_ashdown_id, "markdown", "Sera Ashdown is an archivist of the Ember Court who believes the forgotten histories can prevent another war.", now])?;
        tx.execute("INSERT INTO documents(id,entity_id,format,body,updated_at) VALUES (?1,?2,?3,?4,?5)", params![Uuid::new_v4().to_string(), tidewatch_id, "markdown", "The Tidewatch keeps signal fires along the coast and answers to no single noble house.", now])?;
        tx.execute("INSERT INTO documents(id,entity_id,format,body,updated_at) VALUES (?1,?2,?3,?4,?5)", params![Uuid::new_v4().to_string(), crown_salt_id, "markdown", "The Crown of Salt is a ceremonial relic worn by the first ruler to unite the river and coast clans.", now])?;
        tx.execute("INSERT INTO documents(id,entity_id,format,body,updated_at) VALUES (?1,?2,?3,?4,?5)", params![Uuid::new_v4().to_string(), coastfolk_id, "markdown", "The Coastfolk Culture treats the tides as a calendar and settles disputes by reciting the names of drowned villages.", now])?;
        tx.execute("INSERT INTO documents(id,entity_id,format,body,updated_at) VALUES (?1,?2,?3,?4,?5)", params![Uuid::new_v4().to_string(), frostgate_battle_id, "markdown", "The Battle of Frostgate broke the northern siege and made the pass a symbol of the realm's uneasy unity.", now])?;
        tx.execute("INSERT INTO entity_fields(entity_id,namespace,key,value) VALUES (?1,'lore','summary',?2)", params![eldermere_id, "Ancient coastal fortress and seat of power"])?;
        tx.execute("INSERT INTO entity_fields(entity_id,namespace,key,value) VALUES (?1,'lore','aliases',?2)", params![eldermere_id, "The Elder Hold, The Clifftop"])?;
        tx.execute("INSERT INTO entity_fields(entity_id,namespace,key,value) VALUES (?1,'lore','summary',?2)", params![lord_ashford_id, "Ruler of Eldermere and keeper of the realm"])?;
        tx.execute("INSERT INTO entity_fields(entity_id,namespace,key,value) VALUES (?1,'lore','aliases',?2)", params![lord_ashford_id, "Ash, The Lord"])?;
        tx.execute("INSERT INTO entity_fields(entity_id,namespace,key,value) VALUES (?1,'lore','summary',?2)", params![mira_vale_id, "Cartographer of the changing coast"])?;
        tx.execute("INSERT INTO entity_fields(entity_id,namespace,key,value) VALUES (?1,'lore','aliases',?2)", params![mira_vale_id, "The Moonlit Mapmaker"])?;
        tx.execute("INSERT INTO entity_fields(entity_id,namespace,key,value) VALUES (?1,'lore','summary',?2)", params![sunken_archive_id, "Buried repository beneath the river delta"])?;
        tx.execute("INSERT INTO entity_fields(entity_id,namespace,key,value) VALUES (?1,'lore','summary',?2)", params![ember_court_id, "Keepers of the old fire rites"])?;
        tx.execute("INSERT INTO entity_fields(entity_id,namespace,key,value) VALUES (?1,'lore','summary',?2)", params![star_compass_id, "Artifact that points toward lost places"])?;
        tx.execute("INSERT INTO entity_fields(entity_id,namespace,key,value) VALUES (?1,'lore','summary',?2)", params![riverborn_id, "Delta culture of navigators and singers"])?;
        tx.execute("INSERT INTO entity_fields(entity_id,namespace,key,value) VALUES (?1,'timeline','startsAt',?2)", params![founding_id, "0001-01-01"])?;
        tx.execute("INSERT INTO entity_fields(entity_id,namespace,key,value) VALUES (?1,'timeline','endsAt',?2)", params![founding_id, "0001-01-01"])?;
        tx.execute("INSERT INTO entity_fields(entity_id,namespace,key,value) VALUES (?1,'timeline','startsAt',?2)", params![treaty_id, "0042-03-15"])?;
        tx.execute("INSERT INTO entity_fields(entity_id,namespace,key,value) VALUES (?1,'timeline','endsAt',?2)", params![treaty_id, "0042-06-02"])?;
        tx.execute("INSERT INTO entity_fields(entity_id,namespace,key,value) VALUES (?1,'timeline','startsAt',?2)", params![rebellion_id, "0067-07-22"])?;
        tx.execute("INSERT INTO entity_fields(entity_id,namespace,key,value) VALUES (?1,'timeline','endsAt',?2)", params![rebellion_id, "0068-02-11"])?;
        tx.execute("INSERT INTO entity_fields(entity_id,namespace,key,value) VALUES (?1,'timeline','startsAt',?2)", params![war_embers_id, "0038-09-04"])?;
        tx.execute("INSERT INTO entity_fields(entity_id,namespace,key,value) VALUES (?1,'timeline','endsAt',?2)", params![war_embers_id, "0042-03-14"])?;
        tx.execute("INSERT INTO entity_fields(entity_id,namespace,key,value) VALUES (?1,'timeline','startsAt',?2)", params![first_tide_id, "0021-11-19"])?;
        tx.execute("INSERT INTO entity_fields(entity_id,namespace,key,value) VALUES (?1,'timeline','startsAt',?2)", params![archive_opening_id, "0071-04-08"])?;
        tx.execute("INSERT INTO entity_fields(entity_id,namespace,key,value) VALUES (?1,'timeline','endsAt',?2)", params![archive_opening_id, "0071-04-10"])?;
        tx.execute("INSERT INTO entity_fields(entity_id,namespace,key,value) VALUES (?1,'lore','region',?2)", params![eldermere_id, "Southern coast"])?;
        tx.execute("INSERT INTO entity_fields(entity_id,namespace,key,value) VALUES (?1,'lore','climate',?2)", params![eldermere_id, "Salt wind and mild winters"])?;
        tx.execute("INSERT INTO entity_fields(entity_id,namespace,key,value) VALUES (?1,'lore','population',?2)", params![eldermere_id, "18000"])?;
        tx.execute("INSERT INTO entity_fields(entity_id,namespace,key,value) VALUES (?1,'lore','region',?2)", params![glass_coast_id, "Western shore"])?;
        tx.execute("INSERT INTO entity_fields(entity_id,namespace,key,value) VALUES (?1,'lore','climate',?2)", params![glass_coast_id, "Stormy and bright"])?;
        tx.execute("INSERT INTO entity_fields(entity_id,namespace,key,value) VALUES (?1,'lore','population',?2)", params![glass_coast_id, "4200"])?;
        tx.execute(
            "INSERT INTO entity_fields(entity_id,namespace,key,value) VALUES (?1,'lore','goal',?2)",
            params![silver_hand_id, "Protect the realm's roads and rulers"],
        )?;
        tx.execute(
            "INSERT INTO entity_fields(entity_id,namespace,key,value) VALUES (?1,'lore','goal',?2)",
            params![ember_court_id, "Preserve the old fire rites"],
        )?;
        tx.execute("INSERT INTO entity_fields(entity_id,namespace,key,value) VALUES (?1,'lore','values',?2)", params![highland_id, "Honor, craft, and ancestral songs"])?;
        tx.execute("INSERT INTO entity_fields(entity_id,namespace,key,value) VALUES (?1,'lore','language',?2)", params![highland_id, "High Cant"])?;
        tx.execute("INSERT INTO entity_fields(entity_id,namespace,key,value) VALUES (?1,'lore','values',?2)", params![riverborn_id, "Adaptability, memory, and hospitality"])?;
        tx.execute("INSERT INTO entity_fields(entity_id,namespace,key,value) VALUES (?1,'lore','language',?2)", params![riverborn_id, "River Cant"])?;
        tx.execute("INSERT INTO entity_fields(entity_id,namespace,key,value) VALUES (?1,'lore','occupation',?2)", params![lord_ashford_id, "Ruler and treaty keeper"])?;
        tx.execute("INSERT INTO entity_fields(entity_id,namespace,key,value) VALUES (?1,'lore','occupation',?2)", params![mira_vale_id, "Cartographer and coastal guide"])?;
        tx.execute("INSERT INTO entity_fields(entity_id,namespace,key,value) VALUES (?1,'lore','region',?2)", params![frostgate_id, "Northern frontier"])?;
        tx.execute("INSERT INTO entity_fields(entity_id,namespace,key,value) VALUES (?1,'lore','climate',?2)", params![frostgate_id, "Cold, windy, and snowbound"])?;
        tx.execute("INSERT INTO entity_fields(entity_id,namespace,key,value) VALUES (?1,'lore','population',?2)", params![frostgate_id, "900"])?;
        tx.execute("INSERT INTO entity_fields(entity_id,namespace,key,value) VALUES (?1,'lore','region',?2)", params![lantern_marsh_id, "Eastern delta"])?;
        tx.execute("INSERT INTO entity_fields(entity_id,namespace,key,value) VALUES (?1,'lore','climate',?2)", params![lantern_marsh_id, "Wet and misty"])?;
        tx.execute("INSERT INTO entity_fields(entity_id,namespace,key,value) VALUES (?1,'lore','population',?2)", params![lantern_marsh_id, "2300"])?;
        tx.execute("INSERT INTO entity_fields(entity_id,namespace,key,value) VALUES (?1,'lore','occupation',?2)", params![elian_rook_id, "Coast patrol captain"])?;
        tx.execute("INSERT INTO entity_fields(entity_id,namespace,key,value) VALUES (?1,'lore','occupation',?2)", params![sera_ashdown_id, "Archivist and fire-rite scholar"])?;
        tx.execute(
            "INSERT INTO entity_fields(entity_id,namespace,key,value) VALUES (?1,'lore','goal',?2)",
            params![tidewatch_id, "Keep the coast's signal network alive"],
        )?;
        tx.execute("INSERT INTO entity_fields(entity_id,namespace,key,value) VALUES (?1,'lore','material',?2)", params![crown_salt_id, "Silver, pearl, and black coral"])?;
        tx.execute("INSERT INTO entity_fields(entity_id,namespace,key,value) VALUES (?1,'lore','values',?2)", params![coastfolk_id, "Reciprocity, remembrance, and safe passage"])?;
        tx.execute("INSERT INTO entity_fields(entity_id,namespace,key,value) VALUES (?1,'lore','language',?2)", params![coastfolk_id, "Coastal Sign"])?;
        tx.execute("INSERT INTO relationships(id,source_id,target_id,relationship_type,metadata) VALUES (?1,?2,?3,?4,?5)", params![Uuid::new_v4().to_string(), lord_ashford_id, eldermere_id, "originates_from", "{}"])?;
        tx.execute("INSERT INTO relationships(id,source_id,target_id,relationship_type,metadata) VALUES (?1,?2,?3,?4,?5)", params![Uuid::new_v4().to_string(), lord_ashford_id, silver_hand_id, "affiliated_with", "{}"])?;
        tx.execute("INSERT INTO relationships(id,source_id,target_id,relationship_type,metadata) VALUES (?1,?2,?3,?4,?5)", params![Uuid::new_v4().to_string(), amulet_id, mira_vale_id, "created_by", "{}"])?;
        tx.execute("INSERT INTO relationships(id,source_id,target_id,relationship_type,metadata) VALUES (?1,?2,?3,?4,?5)", params![Uuid::new_v4().to_string(), amulet_id, lord_ashford_id, "owned_by", "{}"])?;
        tx.execute("INSERT INTO relationships(id,source_id,target_id,relationship_type,metadata) VALUES (?1,?2,?3,?4,?5)", params![Uuid::new_v4().to_string(), silver_hand_id, lord_ashford_id, "led_by", "{}"])?;
        tx.execute("INSERT INTO relationships(id,source_id,target_id,relationship_type,metadata) VALUES (?1,?2,?3,?4,?5)", params![Uuid::new_v4().to_string(), silver_hand_id, eldermere_id, "based_in", "{}"])?;
        tx.execute("INSERT INTO relationships(id,source_id,target_id,relationship_type,metadata) VALUES (?1,?2,?3,?4,?5)", params![Uuid::new_v4().to_string(), highland_id, eldermere_id, "rooted_in", "{}"])?;
        tx.execute("INSERT INTO relationships(id,source_id,target_id,relationship_type,metadata) VALUES (?1,?2,?3,?4,?5)", params![Uuid::new_v4().to_string(), elian_rook_id, glass_coast_id, "originates_from", "{}"])?;
        tx.execute("INSERT INTO relationships(id,source_id,target_id,relationship_type,metadata) VALUES (?1,?2,?3,?4,?5)", params![Uuid::new_v4().to_string(), elian_rook_id, silver_hand_id, "affiliated_with", "{}"])?;
        tx.execute("INSERT INTO relationships(id,source_id,target_id,relationship_type,metadata) VALUES (?1,?2,?3,?4,?5)", params![Uuid::new_v4().to_string(), sera_ashdown_id, sunken_archive_id, "originates_from", "{}"])?;
        tx.execute("INSERT INTO relationships(id,source_id,target_id,relationship_type,metadata) VALUES (?1,?2,?3,?4,?5)", params![Uuid::new_v4().to_string(), sera_ashdown_id, ember_court_id, "affiliated_with", "{}"])?;
        tx.execute("INSERT INTO relationships(id,source_id,target_id,relationship_type,metadata) VALUES (?1,?2,?3,?4,?5)", params![Uuid::new_v4().to_string(), tidewatch_id, elian_rook_id, "led_by", "{}"])?;
        tx.execute("INSERT INTO relationships(id,source_id,target_id,relationship_type,metadata) VALUES (?1,?2,?3,?4,?5)", params![Uuid::new_v4().to_string(), tidewatch_id, lantern_marsh_id, "based_in", "{}"])?;
        tx.execute("INSERT INTO relationships(id,source_id,target_id,relationship_type,metadata) VALUES (?1,?2,?3,?4,?5)", params![Uuid::new_v4().to_string(), tidewatch_id, frostgate_id, "based_in", "{}"])?;
        tx.execute("INSERT INTO relationships(id,source_id,target_id,relationship_type,metadata) VALUES (?1,?2,?3,?4,?5)", params![Uuid::new_v4().to_string(), crown_salt_id, sera_ashdown_id, "created_by", "{}"])?;
        tx.execute("INSERT INTO relationships(id,source_id,target_id,relationship_type,metadata) VALUES (?1,?2,?3,?4,?5)", params![Uuid::new_v4().to_string(), crown_salt_id, lord_ashford_id, "owned_by", "{}"])?;
        tx.execute("INSERT INTO relationships(id,source_id,target_id,relationship_type,metadata) VALUES (?1,?2,?3,?4,?5)", params![Uuid::new_v4().to_string(), coastfolk_id, glass_coast_id, "rooted_in", "{}"])?;
        tx.execute("INSERT INTO relationships(id,source_id,target_id,relationship_type,metadata) VALUES (?1,?2,?3,?4,?5)", params![Uuid::new_v4().to_string(), coastfolk_id, lantern_marsh_id, "rooted_in", "{}"])?;
        tx.execute("INSERT INTO relationships(id,source_id,target_id,relationship_type,metadata) VALUES (?1,?2,?3,?4,?5)", params![Uuid::new_v4().to_string(), ember_court_id, sera_ashdown_id, "led_by", "{}"])?;
        tx.execute("INSERT OR IGNORE INTO module_versions(module_id,version) VALUES ('daena.lore',1), ('daena.timeline',1)", [])?;
        tx.execute("INSERT OR IGNORE INTO module_namespaces(module_id,namespace) VALUES ('daena.lore','lore'), ('daena.timeline','timeline')", [])?;
        tx.commit()?;
        self.rebuild_search()?;
        self.notify_export_worker()?;
        Ok(25)
    }

    pub fn get_module_version(&self, module_id: &str) -> Result<i64, CoreError> {
        self.connection
            .query_row(
                "SELECT COALESCE(version, 0) FROM module_versions WHERE module_id=?1",
                params![module_id],
                |row| row.get(0),
            )
            .or_else(|e| {
                if e == rusqlite::Error::QueryReturnedNoRows {
                    Ok(0)
                } else {
                    Err(e)
                }
            })
            .map_err(CoreError::Database)
    }

    pub fn module_states(&self) -> Result<Vec<ModuleState>, CoreError> {
        let mut statement = self.connection.prepare("SELECT m.module_id, COALESCE(s.enabled, 1), m.version, p.package_version, o.overlay_json FROM module_versions m LEFT JOIN module_state s ON s.module_id = m.module_id LEFT JOIN module_package_versions p ON p.module_id = m.module_id LEFT JOIN module_schema_overlays o ON o.module_id = m.module_id ORDER BY m.module_id")?;
        let rows = statement.query_map([], |row| {
            let overlay_json: Option<String> = row.get(4)?;
            let schema_overlay = overlay_json.and_then(|json| {
                serde_json::from_str(&json)
                    .ok()
                    .filter(|value: &serde_json::Value| !value.is_null())
            });
            Ok(ModuleState {
                module_id: row.get(0)?,
                enabled: row.get::<_, i64>(1)? != 0,
                version: row.get(2)?,
                package_version: row.get(3)?,
                schema_overlay,
            })
        })?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    pub fn validate_migration(
        &self,
        migration: &crate::migrations::Migration,
        current: i64,
    ) -> Result<(), CoreError> {
        crate::migrations::validate(migration, current)
    }

    pub fn set_module_enabled(&self, module_id: String, enabled: bool) -> Result<(), CoreError> {
        self.set_module_enabled_with_request(module_id, enabled, None)
    }

    pub fn set_module_enabled_with_request(
        &self,
        module_id: String,
        enabled: bool,
        request_id: Option<&str>,
    ) -> Result<(), CoreError> {
        if self
            .committed_mutation::<serde_json::Value>(request_id)?
            .is_some()
        {
            return Ok(());
        }
        let request_id = self.request_id(request_id)?;
        let transaction = self.begin_mutation(
            &request_id,
            Some(&serde_json::Value::Null),
            &[format!("plugins/{module_id}.json")],
        )?;
        transaction.execute(
            "INSERT OR IGNORE INTO module_versions(module_id,version) VALUES (?1,0)",
            params![module_id],
        )?;
        transaction.execute(
            "INSERT INTO module_state(module_id, enabled) VALUES (?1, ?2) ON CONFLICT(module_id) DO UPDATE SET enabled=excluded.enabled",
            params![module_id, enabled as i64],
        )?;
        transaction.commit()?;
        self.notify_export_worker()?;
        Ok(())
    }

    pub fn is_module_enabled(&self, module_id: &str) -> Result<bool, CoreError> {
        Ok(self
            .connection
            .query_row(
                "SELECT enabled FROM module_state WHERE module_id=?1",
                params![module_id],
                |row| row.get::<_, i64>(0),
            )
            .optional()?
            .unwrap_or(1)
            != 0)
    }

    pub fn module_schema_overlay(&self, module_id: &str) -> Result<Option<serde_json::Value>, CoreError> {
        let overlay_json: Option<String> = self
            .connection
            .query_row(
                "SELECT overlay_json FROM module_schema_overlays WHERE module_id=?1",
                params![module_id],
                |row| row.get(0),
            )
            .optional()?;
        Ok(overlay_json.and_then(|json| serde_json::from_str(&json).ok()))
    }

    pub fn set_module_schema_overlay(
        &self,
        module_id: String,
        overlay: Option<serde_json::Value>,
    ) -> Result<(), CoreError> {
        self.set_module_schema_overlay_with_request(module_id, overlay, None)
    }

    pub fn set_module_schema_overlay_with_request(
        &self,
        module_id: String,
        overlay: Option<serde_json::Value>,
        request_id: Option<&str>,
    ) -> Result<(), CoreError> {
        if self
            .committed_mutation::<serde_json::Value>(request_id)?
            .is_some()
        {
            return Ok(());
        }
        if module_id.trim().is_empty() {
            return Err(CoreError::Validation("module id is required".into()));
        }
        let request_id = self.request_id(request_id)?;
        let transaction = self.begin_mutation(
            &request_id,
            Some(&serde_json::Value::Null),
            &[format!("plugins/{module_id}.json")],
        )?;
        transaction.execute(
            "INSERT OR IGNORE INTO module_versions(module_id,version) VALUES (?1,0)",
            params![module_id],
        )?;
        match overlay {
            Some(value) if !value.is_null() => {
                let overlay_json = serde_json::to_string(&value)
                    .map_err(|error| CoreError::Validation(error.to_string()))?;
                transaction.execute(
                    "INSERT INTO module_schema_overlays(module_id, overlay_json) VALUES (?1, ?2) ON CONFLICT(module_id) DO UPDATE SET overlay_json=excluded.overlay_json",
                    params![module_id, overlay_json],
                )?;
            }
            _ => {
                transaction.execute(
                    "DELETE FROM module_schema_overlays WHERE module_id=?1",
                    params![module_id],
                )?;
            }
        }
        transaction.commit()?;
        self.notify_export_worker()?;
        Ok(())
    }

    pub fn set_module_package_version(
        &self,
        module_id: &str,
        package_version: Option<&str>,
    ) -> Result<(), CoreError> {
        self.set_module_package_version_with_request(module_id, package_version, None)
    }

    pub fn set_module_package_version_with_request(
        &self,
        module_id: &str,
        package_version: Option<&str>,
        request_id: Option<&str>,
    ) -> Result<(), CoreError> {
        if self
            .committed_mutation::<serde_json::Value>(request_id)?
            .is_some()
        {
            return Ok(());
        }
        let request_id = self.request_id(request_id)?;
        let transaction = self.begin_mutation(
            &request_id,
            Some(&serde_json::Value::Null),
            &[format!("plugins/{module_id}.json")],
        )?;
        match package_version {
            Some(version) => {
                transaction.execute(
                    "INSERT OR IGNORE INTO module_versions(module_id,version) VALUES (?1,0)",
                    params![module_id],
                )?;
                transaction.execute(
                    "INSERT INTO module_package_versions(module_id,package_version) VALUES (?1,?2) ON CONFLICT(module_id) DO UPDATE SET package_version=excluded.package_version",
                    params![module_id, version],
                )?;
            }
            None => {
                transaction.execute(
                    "DELETE FROM module_package_versions WHERE module_id=?1",
                    params![module_id],
                )?;
            }
        }
        transaction.commit()?;
        self.notify_export_worker()?;
        Ok(())
    }

    pub fn module_package_version(&self, module_id: &str) -> Result<Option<String>, CoreError> {
        self.connection
            .query_row(
                "SELECT package_version FROM module_package_versions WHERE module_id=?1",
                params![module_id],
                |row| row.get(0),
            )
            .optional()
            .map_err(CoreError::from)
    }

    pub fn apply_migration(
        &mut self,
        migration: &crate::migrations::Migration,
    ) -> Result<(), CoreError> {
        self.apply_migrations_with_request(std::slice::from_ref(migration), None)
            .map(|_| ())
    }

    /// Apply a plugin migration chain after one backup. If a later migration
    /// fails, restore the backup so the project remains usable at its prior
    /// data version.
    pub fn apply_migrations(
        &mut self,
        migrations: &[crate::migrations::Migration],
    ) -> Result<String, CoreError> {
        self.apply_migrations_with_request(migrations, None)
    }

    pub fn apply_migrations_with_request(
        &mut self,
        migrations: &[crate::migrations::Migration],
        request_id: Option<&str>,
    ) -> Result<String, CoreError> {
        if let Some(backup) = self.committed_mutation::<String>(request_id)? {
            return Ok(backup);
        }
        let backup = self
            .backup()
            .map_err(|error| CoreError::Validation(error.to_string()))?;
        let request_id = self.request_id(request_id)?;
        let result = serde_json::to_value(&backup)
            .map_err(|error| CoreError::Serialization(error.to_string()))?;
        let affected = migrations
            .iter()
            .map(|migration| format!("plugins/{}.json", migration.module_id))
            .collect::<Vec<_>>();
        let transaction = self.begin_mutation(&request_id, Some(&result), &affected)?;
        for migration in migrations {
            crate::migrations::apply_in_transaction(&transaction, migration)?;
        }
        transaction.commit()?;
        self.notify_export_worker()?;
        Ok(backup)
    }
}

pub(crate) fn chrono_like_now() -> String {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos()
        .to_string()
}

fn copy_directory(source: &Path, destination: &Path) -> Result<(), CoreError> {
    std::fs::create_dir_all(destination).map_err(|source| CoreError::Io {
        operation: "create recovery backup payload directory",
        source,
    })?;
    for entry in std::fs::read_dir(source).map_err(|source| CoreError::Io {
        operation: "read recovery backup payload directory",
        source,
    })? {
        let entry = entry.map_err(|source| CoreError::Io {
            operation: "read recovery backup payload entry",
            source,
        })?;
        let source_path = entry.path();
        let destination_path = destination.join(entry.file_name());
        if source_path.is_dir() {
            copy_directory(&source_path, &destination_path)?;
        } else {
            std::fs::copy(&source_path, &destination_path).map_err(|source| CoreError::Io {
                operation: "copy recovery backup payload",
                source,
            })?;
        }
    }
    Ok(())
}

fn copy_portable_project(source: &Path, destination: &Path) -> Result<(), CoreError> {
    std::fs::create_dir_all(destination).map_err(|source| CoreError::Io {
        operation: "create portable backup artifact",
        source,
    })?;
    for entry in ["project.json", "entities", "plugins", "assets"] {
        let source_path = source.join(entry);
        if !source_path.exists() {
            continue;
        }
        let destination_path = destination.join(entry);
        if source_path.is_dir() {
            copy_directory(&source_path, &destination_path)?;
        } else {
            std::fs::copy(&source_path, &destination_path).map_err(|source| CoreError::Io {
                operation: "copy portable backup file",
                source,
            })?;
        }
    }
    Ok(())
}

/// Parses a recovery copy file name of the form
/// `<epochMillis>-<entityId>-<uuid>.map`, returning the epoch milliseconds when
/// the name matches the entity. Both the trailing UUID and the embedded entity
/// ID must validate so that files from other maps (or traversal attempts) are
/// never touched.
fn parse_map_recovery_file_name(entity_id: &str, file_name: &str) -> Option<String> {
    let stem = file_name.strip_suffix(".map")?;
    if stem.len() < 37 + 36 {
        return None;
    }
    let uuid_part = &stem[stem.len() - 36..];
    Uuid::parse_str(uuid_part).ok()?;
    let prefix = &stem[..stem.len() - 37];
    let created_at = prefix.strip_suffix(&format!("-{entity_id}"))?;
    if created_at.is_empty() || !created_at.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    Some(created_at.to_string())
}

fn digest_bytes(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn streamed_file_digest(path: &Path) -> Result<(String, i64), CoreError> {
    let mut file =
        std::fs::File::open(path).map_err(|error| CoreError::NotFound(error.to_string()))?;
    let mut digest = Sha256::new();
    let mut buffer = [0_u8; 128 * 1024];
    let mut size = 0_i64;
    loop {
        let count = std::io::Read::read(&mut file, &mut buffer).map_err(|error| CoreError::Io {
            operation: "read asset source",
            source: error,
        })?;
        if count == 0 {
            break;
        }
        digest.update(&buffer[..count]);
        size = size
            .checked_add(count as i64)
            .ok_or_else(|| CoreError::Validation("asset is too large".into()))?;
    }
    Ok((format!("sha256:{:x}", digest.finalize()), size))
}

fn staged_canonical_sources(
    root: &Path,
    snapshot: &ProjectSnapshot,
) -> Result<Vec<crate::storage::CanonicalSource>, CoreError> {
    fn visit(root: &Path, current: &Path, paths: &mut Vec<String>) -> Result<(), CoreError> {
        for entry in std::fs::read_dir(current).map_err(|source| CoreError::Io {
            operation: "read targeted staging directory",
            source,
        })? {
            let entry = entry.map_err(|source| CoreError::Io {
                operation: "read targeted staging entry",
                source,
            })?;
            let path = entry.path();
            if path.is_dir() {
                visit(root, &path, paths)?;
            } else if path.is_file() {
                paths.push(
                    path.strip_prefix(root)
                        .map_err(|error| CoreError::Validation(error.to_string()))?
                        .to_string_lossy()
                        .replace('\\', "/"),
                );
            }
        }
        Ok(())
    }

    let mut paths = Vec::new();
    for entity in &snapshot.entities {
        let path =
            crate::storage::normalized_project_path(root, &format!("entities/{}", entity.id))?;
        if path.is_dir() {
            visit(root, &path, &mut paths)?;
        }
    }
    for module in &snapshot.modules {
        let path = crate::storage::normalized_project_path(
            root,
            &format!("plugins/{}.json", module.module_id),
        )?;
        if path.is_file() {
            paths.push(format!("plugins/{}.json", module.module_id));
        }
    }
    for record in &snapshot.module_records {
        let path = crate::storage::normalized_project_path(
            root,
            &format!("plugins/{}.json", record.module_id),
        )?;
        if path.is_file() {
            paths.push(format!("plugins/{}.json", record.module_id));
        }
    }
    for asset in &snapshot.assets {
        let path = crate::storage::normalized_project_path(root, &asset.path)?;
        if path.is_file() {
            paths.push(asset.path.clone());
        }
    }
    paths.sort();
    paths.dedup();
    paths
        .into_iter()
        .map(|path| {
            let content_hash = crate::sync::hash_path(root, &path)?.ok_or_else(|| {
                CoreError::Validation(format!("targeted staged source is missing: {path}"))
            })?;
            Ok(crate::storage::CanonicalSource {
                path,
                content_hash,
                format_version: crate::storage::PROJECT_FORMAT_VERSION,
            })
        })
        .collect()
}

fn optional_record_filter<'a>(
    value: Option<&'a str>,
    label: &str,
) -> Result<Option<&'a str>, CoreError> {
    let value = value.map(str::trim).filter(|item| !item.is_empty());
    if let Some(value) = value {
        if value.len() > 128 {
            return Err(CoreError::Validation(format!("{label} exceeds 128 bytes")));
        }
    }
    Ok(value)
}

fn module_record_order_sql(sort: &str, alias: &str) -> Result<String, CoreError> {
    let id = format!("{alias}id");
    Ok(match sort {
        "" | "lemma" => format!("lower(json_extract({alias}value, '$.lemma')), {id}"),
        "symbol" => format!("lower(json_extract({alias}value, '$.symbol')), {id}"),
        "name" => format!("lower(json_extract({alias}value, '$.name')), {id}"),
        "title" => format!("lower(json_extract({alias}value, '$.title')), {id}"),
        "updatedAt" => format!("{alias}updated_at DESC, {id}"),
        "status" => format!(
            "lower(COALESCE(json_extract({alias}value, '$.status'), '')), lower(COALESCE(json_extract({alias}value, '$.lemma'), json_extract({alias}value, '$.name'), json_extract({alias}value, '$.symbol'), '')), {id}"
        ),
        _ => {
            return Err(CoreError::Validation(
                "record sort must be lemma, symbol, name, title, updatedAt, or status".into(),
            ))
        }
    })
}

fn module_record_filter_sql(alias: &str) -> String {
    let json_source = if alias.is_empty() {
        "module_records.value".into()
    } else {
        format!("{alias}value")
    };
    format!(
        "AND (:status IS NULL OR json_extract({json_source}, '$.status') = :status) \
         AND (:tag IS NULL OR EXISTS (SELECT 1 FROM json_each({json_source}, '$.tags') AS tag_item WHERE tag_item.atom = :tag)) \
         AND (:homonyms = 0 OR lower(json_extract({json_source}, '$.lemma')) IN ( \
            SELECT lower(json_extract(value, '$.lemma')) FROM module_records \
            WHERE module_id=:module AND collection=:collection AND owner_entity_id=:owner \
            GROUP BY lower(json_extract(value, '$.lemma')) HAVING COUNT(*) > 1))"
    )
}

fn validate_module_record_scope(
    module_id: &str,
    collection: &str,
    owner_entity_id: &str,
) -> Result<(), CoreError> {
    let valid_component = |value: &str| {
        !value.is_empty()
            && value.len() <= 128
            && value
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'-' | b'_'))
    };
    if !valid_component(module_id) {
        return Err(CoreError::Validation("invalid module record module ID".into()));
    }
    if !valid_component(collection) {
        return Err(CoreError::Validation(
            "invalid module record collection".into(),
        ));
    }
    Uuid::parse_str(owner_entity_id)
        .map_err(|_| CoreError::Validation("module record owner must be a UUID".into()))?;
    Ok(())
}

fn validate_module_record_input(
    module_id: &str,
    collection: &str,
    owner_entity_id: &str,
    value: &serde_json::Value,
) -> Result<(), CoreError> {
    validate_module_record_scope(module_id, collection, owner_entity_id)?;
    if !value.is_object() {
        return Err(CoreError::Validation(
            "module record value must be an object".into(),
        ));
    }
    let bytes = serde_json::to_vec(value)
        .map_err(|error| CoreError::Serialization(error.to_string()))?;
    if bytes.len() > 64 * 1024 {
        return Err(CoreError::Validation(
            "module record exceeds 64 KiB".into(),
        ));
    }
    Ok(())
}

fn revision_digest<T: Serialize>(value: &T) -> Result<String, CoreError> {
    let bytes =
        serde_json::to_vec(value).map_err(|error| CoreError::Serialization(error.to_string()))?;
    Ok(format!("sha256:{}", digest_bytes(&bytes)))
}

const LOCATION_PROJECTION_SELECT: &str = "SELECT location_id,entity_id,map_entity_id,label,role,anchor_kind,provider,feature_kind,feature_id,min_x,min_y,max_x,max_y,valid_from,valid_to,resolution,geometry FROM map_location_projection";

fn location_projection_json(row: &rusqlite::Row<'_>) -> rusqlite::Result<serde_json::Value> {
    let geometry: Option<String> = row.get(16)?;
    let anchor = geometry
        .as_deref()
        .and_then(|raw| serde_json::from_str::<serde_json::Value>(raw).ok())
        .unwrap_or(serde_json::Value::Null);
    Ok(serde_json::json!({
        "id": row.get::<_, String>(0)?,
        "entityId": row.get::<_, String>(1)?,
        "mapEntityId": row.get::<_, String>(2)?,
        "label": row.get::<_, Option<String>>(3)?,
        "role": row.get::<_, String>(4)?,
        "anchorKind": row.get::<_, String>(5)?,
        "provider": row.get::<_, Option<String>>(6)?,
        "featureKind": row.get::<_, Option<String>>(7)?,
        "featureId": row.get::<_, Option<String>>(8)?,
        "bounds": [
            row.get::<_, Option<f64>>(9)?,
            row.get::<_, Option<f64>>(10)?,
            row.get::<_, Option<f64>>(11)?,
            row.get::<_, Option<f64>>(12)?
        ],
        "validity": {
            "from": row.get::<_, Option<String>>(13)?,
            "to": row.get::<_, Option<String>>(14)?
        },
        "resolution": row.get::<_, String>(15)?,
        "anchor": anchor
    }))
}

fn write_location_projection(
    connection: &rusqlite::Connection,
    entity_id: &str,
    location: &serde_json::Value,
    resolution: &str,
) -> Result<(), CoreError> {
    let anchor = location.get("anchor").cloned().unwrap_or_default();
    let kind = anchor
        .get("kind")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("unknown");
    let bounds = match kind {
        "point" => bounds_for_points(anchor.get("point").and_then(serde_json::Value::as_array)),
        "provider-feature" => bounds_for_points(
            anchor
                .get("fallbackPoint")
                .and_then(serde_json::Value::as_array),
        ),
        "path" => bounds_for_points(anchor.get("points").and_then(serde_json::Value::as_array)),
        "area" => anchor
            .get("rings")
            .and_then(serde_json::Value::as_array)
            .map(|rings| {
                bounds_for_points(Some(
                    &rings
                        .iter()
                        .filter_map(serde_json::Value::as_array)
                        .flatten()
                        .cloned()
                        .collect::<Vec<_>>(),
                ))
            })
            .unwrap_or((None, None, None, None)),
        _ => (None, None, None, None),
    };
    let geometry = serde_json::to_string(&anchor)
        .map_err(|error| CoreError::Serialization(error.to_string()))?;
    connection.execute(
        "INSERT OR REPLACE INTO map_location_projection(location_id,entity_id,map_entity_id,label,role,anchor_kind,provider,feature_kind,feature_id,min_x,min_y,max_x,max_y,valid_from,valid_to,resolution,geometry) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17)",
        rusqlite::params![
            location.get("id").and_then(serde_json::Value::as_str).unwrap_or_default(),
            entity_id,
            location.get("mapEntityId").and_then(serde_json::Value::as_str).unwrap_or_default(),
            location.get("label").and_then(serde_json::Value::as_str),
            location.get("role").and_then(serde_json::Value::as_str).unwrap_or_default(),
            kind,
            anchor.get("provider").and_then(serde_json::Value::as_str),
            anchor.get("featureKind").and_then(serde_json::Value::as_str),
            anchor.get("featureId").and_then(serde_json::Value::as_str),
            bounds.0,
            bounds.1,
            bounds.2,
            bounds.3,
            location.pointer("/validity/from").filter(|value| !value.is_null()).map(ToString::to_string),
            location.pointer("/validity/to").filter(|value| !value.is_null()).map(ToString::to_string),
            resolution,
            geometry
        ],
    )?;
    Ok(())
}

fn bounds_for_points(
    points: Option<&Vec<serde_json::Value>>,
) -> (Option<f64>, Option<f64>, Option<f64>, Option<f64>) {
    let points = points.into_iter().flatten().collect::<Vec<_>>();
    let coordinates = points
        .iter()
        .filter_map(|point| Some((point.get(0)?.as_f64()?, point.get(1)?.as_f64()?)))
        .collect::<Vec<_>>();
    if coordinates.is_empty() {
        return (None, None, None, None);
    }
    let (mut min_x, mut min_y, mut max_x, mut max_y) = (
        coordinates[0].0,
        coordinates[0].1,
        coordinates[0].0,
        coordinates[0].1,
    );
    for (x, y) in coordinates {
        min_x = min_x.min(x);
        min_y = min_y.min(y);
        max_x = max_x.max(x);
        max_y = max_y.max(y);
    }
    (Some(min_x), Some(min_y), Some(max_x), Some(max_y))
}

#[cfg(test)]
mod tests;
