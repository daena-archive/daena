// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::{Cursor, Read};
use std::path::{Component, Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{mpsc, Arc, Mutex, OnceLock};
use std::thread;
use std::time::{Duration, Instant};

use daena_core::{
    read_json, validate_checkpoint, Asset, AssetFileInput, AssetFileReplaceInput, AssetInput,
    AssetMetadataUpdate, AssetReplaceInput, AuthorityContext, CheckpointHandle, CheckpointManifest,
    CoreError, CoreService, CreateEntity, CreateEntry, CreateEntryField, CreateEntryRelationship,
    Entity, ExternalChangeReport, FieldValue, GitLogEntry, GitPreflight, GitRemote, GitResetResult,
    GitStatus, GitToolInfo, Migration, Operation, ProjectInfo, ProjectStore, Relationship,
    RelationshipInput, SaveDocument, SaveEntry,
};
use daena_plugin_api::{
    merge_module_manifest, parse_module_overlay, supports_schema_overlay, CommandAction,
    MetadataFieldDefinition, MigrationOperation, ModuleSchemaEditorState, ModuleSchemaOverlay,
    PluginManifest, RpcRequest, RpcResponse, ViewComponent, SCHEMA_OVERLAY_VERSION,
};
use daena_plugin_host::{
    plugin_window_label, webview_policy, ArchiveLimits, DependencyResolver, PluginHost, Session,
    VerificationPolicy, BUNDLED_TIMELINE_SERVICE_WASM,
};
use notify::{RecommendedWatcher, RecursiveMode, Watcher};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tauri::{Emitter, Manager};

mod ai;
mod atlas_jobs;
mod atlas_studio;
mod external_import_jobs;
mod settings;

use settings::{AppSettings, AppSettingsUpdate, SettingsStore};

struct ProjectSession {
    core: Mutex<CoreService>,
    read_pool: Mutex<Vec<ProjectStore>>,
}

type SharedCore = Arc<Mutex<Arc<ProjectSession>>>;
type SharedPluginHost = Arc<Mutex<PluginHost>>;
type SharedBinaryTransfers = Arc<Mutex<BinaryTransferManager>>;
type SharedPhysicalJobs = Arc<Mutex<PhysicalJobManager>>;
type SharedAtlasJobs = Arc<Mutex<crate::atlas_jobs::AtlasJobManager>>;
type SharedAtlasStudio = Arc<Mutex<crate::atlas_studio::AtlasStudioManager>>;
type SharedExternalImports = Arc<Mutex<crate::external_import_jobs::ExternalImportJobManager>>;
type SharedSettings = Arc<Mutex<SettingsStore>>;
const READ_CONNECTION_POOL_CAPACITY: usize = 4;
const PHYSICAL_JOB_TTL: Duration = Duration::from_secs(15 * 60);
const HISTORICAL_CACHE_CAPACITY: usize = 16;
static HISTORICAL_EPOCH_CACHE: OnceLock<Mutex<BTreeMap<String, serde_json::Value>>> =
    OnceLock::new();
static HISTORICAL_EPOCH_REQUESTS: OnceLock<Mutex<BTreeMap<String, Arc<AtomicU64>>>> =
    OnceLock::new();
const PHYSICAL_HISTORICAL_PROGRESS_EVENT: &str = "physical-historical-progress";
const ATLAS_PROGRESS_EVENT: &str = "atlas-progress";
const ATLAS_STUDIO_PROGRESS_EVENT: &str = "atlas-studio-progress";
static ATLAS_STUDIO: OnceLock<SharedAtlasStudio> = OnceLock::new();
static EXTERNAL_IMPORTS: OnceLock<SharedExternalImports> = OnceLock::new();

fn new_shared_core() -> SharedCore {
    Arc::new(Mutex::new(Arc::new(ProjectSession {
        core: Mutex::new(CoreService::new()),
        read_pool: Mutex::new(Vec::new()),
    })))
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct PhysicalJobStatus {
    job_id: String,
    request_id: String,
    state: String,
    stage: String,
    completed: u32,
    total: u32,
    error: Option<String>,
    error_code: Option<String>,
    physical_identity: Option<String>,
}

#[derive(Debug, Clone)]
struct PhysicalJobResult {
    source: Vec<u8>,
    generation: serde_json::Value,
    physical_identity: String,
    derived_geojson: String,
    climate: daena_physical::climate::ClimateField,
    evolution: daena_physical::evolution::EvolutionField,
    hydrology: daena_physical::hydrology::HydrologyField,
}

struct PhysicalJob {
    project_id: String,
    session_id: String,
    expires_at: Instant,
    cancel: Arc<AtomicBool>,
    status: PhysicalJobStatus,
    result: Option<PhysicalJobResult>,
}

#[derive(Default)]
struct PhysicalJobManager {
    jobs: BTreeMap<String, PhysicalJob>,
    active_session: Option<PhysicalSession>,
}

struct PhysicalSession {
    id: String,
    project_id: String,
}

impl PhysicalJobManager {
    fn reap_expired(&mut self) {
        let now = Instant::now();
        self.jobs.retain(|_, job| {
            if job.expires_at <= now {
                job.cancel.store(true, Ordering::Relaxed);
                false
            } else {
                true
            }
        });
    }

    fn cancel_all(&mut self) {
        for job in self.jobs.values() {
            job.cancel.store(true, Ordering::Relaxed);
        }
        self.jobs.clear();
        self.active_session = None;
    }

    fn begin_session(&mut self, project_id: String) -> String {
        self.cancel_all();
        let id = uuid::Uuid::new_v4().to_string();
        self.active_session = Some(PhysicalSession {
            id: id.clone(),
            project_id,
        });
        id
    }

    fn ensure_session(&mut self, project_id: &str) -> String {
        self.reap_expired();
        if let Some(session) = &self.active_session {
            if session.project_id == project_id {
                return session.id.clone();
            }
        }
        self.begin_session(project_id.to_string())
    }

    fn active_session_matches(&self, project_id: &str, session_id: &str) -> bool {
        self.active_session
            .as_ref()
            .is_some_and(|session| session.project_id == project_id && session.id == session_id)
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct PhysicalGenerationSettingsInput {
    width: u32,
    height: u32,
    radius_metres: u64,
    target_land_fraction_ppm: u32,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct PhysicalGenerationInput {
    seed: u32,
    retry_index: u32,
    #[serde(default)]
    evolution_preset: Option<String>,
    settings: PhysicalGenerationSettingsInput,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct MaterializedPhysicalEvent {
    entity_id: String,
    #[serde(flatten)]
    event: daena_physical::events::MaterializedEvent,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct PhysicalEventMaterializationResult {
    request_id: String,
    map_entity_id: String,
    materialization_version: u16,
    hazard_derivation_version: u16,
    prediction: bool,
    events: Vec<MaterializedPhysicalEvent>,
}

struct PhysicalProgress {
    jobs: SharedPhysicalJobs,
    job_id: String,
    cancel: Arc<AtomicBool>,
}

struct HistoricalProgress {
    generation: Arc<AtomicU64>,
    expected: u64,
    reporter: Option<HistoricalProgressReporter>,
}

#[derive(Clone)]
struct HistoricalProgressReporter {
    app: tauri::AppHandle,
    map_entity_id: String,
    request_id: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct HistoricalProgressEvent {
    map_entity_id: String,
    request_id: String,
    phase: String,
    completed: u32,
    total: u32,
}

impl HistoricalProgress {
    fn with_reporter(
        generation: Arc<AtomicU64>,
        expected: u64,
        app: tauri::AppHandle,
        map_entity_id: String,
        request_id: String,
    ) -> Self {
        Self {
            generation,
            expected,
            reporter: Some(HistoricalProgressReporter {
                app,
                map_entity_id,
                request_id,
            }),
        }
    }
}

impl daena_physical::ProgressSink for HistoricalProgress {
    fn report(
        &mut self,
        phase: daena_physical::ProgressPhase,
        completed: u32,
        total: u32,
    ) -> Result<(), daena_physical::PhysicalError> {
        self.check_cancelled()?;
        if let Some(reporter) = &self.reporter {
            let _ = reporter.app.emit(
                PHYSICAL_HISTORICAL_PROGRESS_EVENT,
                HistoricalProgressEvent {
                    map_entity_id: reporter.map_entity_id.clone(),
                    request_id: reporter.request_id.clone(),
                    phase: phase.label().into(),
                    completed,
                    total,
                },
            );
        }
        Ok(())
    }

    fn check_cancelled(&self) -> Result<(), daena_physical::PhysicalError> {
        if self.generation.load(Ordering::Acquire) != self.expected {
            return Err(daena_physical::PhysicalError::Cancelled);
        }
        Ok(())
    }
}

impl daena_physical::ProgressSink for PhysicalProgress {
    fn report(
        &mut self,
        phase: daena_physical::ProgressPhase,
        completed: u32,
        total: u32,
    ) -> Result<(), daena_physical::PhysicalError> {
        self.check_cancelled()?;
        let mut manager = self.jobs.lock().map_err(|_| {
            daena_physical::PhysicalError::Validation("job state is unavailable".into())
        })?;
        manager.reap_expired();
        if let Some(job) = manager.jobs.get_mut(&self.job_id) {
            job.status.stage = phase.label().into();
            job.status.completed = completed;
            job.status.total = total;
        }
        Ok(())
    }

    fn check_cancelled(&self) -> Result<(), daena_physical::PhysicalError> {
        if self.cancel.load(Ordering::Relaxed) {
            Err(daena_physical::PhysicalError::Cancelled)
        } else {
            Ok(())
        }
    }
}

fn current_session(core: &SharedCore) -> Result<Arc<ProjectSession>, String> {
    core.lock()
        .map_err(|_| "project lifecycle lock poisoned".to_string())
        .map(|session| session.clone())
}

fn current_info(core: &SharedCore) -> Result<Option<ProjectInfo>, String> {
    let session = current_session(core)?;
    let core = session
        .core
        .lock()
        .map_err(|_| "core lock poisoned".to_string())?;
    Ok(core.info())
}

fn cancel_physical_jobs(jobs: &SharedPhysicalJobs) -> Result<(), String> {
    jobs.lock()
        .map_err(|_| "physical job state is unavailable".to_string())?
        .cancel_all();
    if let Some(atlas) = ATLAS_JOBS.get() {
        atlas_jobs::cancel_atlas_jobs(atlas)?;
    }
    if let Some(studio) = ATLAS_STUDIO.get() {
        atlas_studio::cancel_atlas_studio(studio)?;
    }
    Ok(())
}

fn cancel_external_import_jobs() -> Result<(), String> {
    if let Some(imports) = EXTERNAL_IMPORTS.get() {
        external_import_jobs::cancel_external_imports(imports)?;
    }
    Ok(())
}

fn begin_physical_session(jobs: &SharedPhysicalJobs, project_id: String) -> Result<(), String> {
    jobs.lock()
        .map_err(|_| "physical job state is unavailable".to_string())?
        .begin_session(project_id);
    Ok(())
}

fn begin_current_physical_session(
    jobs: &SharedPhysicalJobs,
    core: &SharedCore,
) -> Result<(), String> {
    if let Some(project_id) = current_info(core)?.map(|info| info.root) {
        begin_physical_session(jobs, project_id)
    } else {
        cancel_physical_jobs(jobs)
    }
}

const MAX_ASSET_TRANSFER_BYTES: usize = daena_core::maps::PHYSICAL_MAX_SOURCE_BYTES;
const ASSET_TRANSFER_TTL: Duration = Duration::from_secs(60);
const BUNDLED_FMG_ARCHIVE: &[u8] = include_bytes!("../plugin-assets/maps/fmg-v1.119.zip");

fn cancel_ai_requests_for(
    plugins: &SharedPluginHost,
    runtime: &ai::SharedAiRuntime,
    project_id: &str,
    plugin_id: Option<&str>,
) -> Result<(), String> {
    let mut host = plugins
        .lock()
        .map_err(|_| "plugin host lock poisoned".to_string())?;
    for request_id in host.ai_request_ids_for(project_id, plugin_id) {
        let _ = ai::cancel_ai_request(runtime, &request_id);
        let _ = ai::remove_ai_citations(runtime, &request_id);
        host.remove_ai_request(&request_id);
    }
    Ok(())
}

fn refresh_ai_index(core: &SharedCore, ai_runtime: &ai::SharedAiRuntime) {
    ai::detach_project_index(ai_runtime);
    let root = core.lock().ok().and_then(|session| {
        session
            .core
            .lock()
            .ok()
            .and_then(|core| core.info().map(|info| info.root))
    });
    match root {
        Some(root) if !root.is_empty() => ai::attach_project_index(ai_runtime, &root),
        _ => ai::detach_project_index(ai_runtime),
    }
}

fn schedule_ai_index_refresh(core: &SharedCore, ai_runtime: &ai::SharedAiRuntime) {
    let core = core.clone();
    let ai_runtime = ai_runtime.clone();
    std::mem::drop(tauri::async_runtime::spawn_blocking(move || {
        refresh_ai_index(&core, &ai_runtime);
    }));
}
// Child plugin webviews must not keep a usable Tauri IPC bridge. Newer WebKit
// builds may already seal __TAURI_INTERNALS__; throwing here aborts Global Code
// and prevents the Maps shell from loading, so neutralize fail-soft.
const PLUGIN_WEBVIEW_ISOLATION_SCRIPT: &str = r#"(function () {
  try {
    var key = "__TAURI_INTERNALS__";
    var desc = Object.getOwnPropertyDescriptor(window, key);
    if (desc && desc.configurable === false) {
      try { window[key] = undefined; } catch (_) {}
      return;
    }
    Object.defineProperty(window, key, { value: undefined, configurable: false, writable: false });
  } catch (_) {}
})();"#;

/// Set during `setup` so the custom protocol handler (which does not receive
/// an `AppHandle`) can forward plugin state events to the shell.
static APP_HANDLE: OnceLock<tauri::AppHandle> = OnceLock::new();
static ATLAS_JOBS: OnceLock<SharedAtlasJobs> = OnceLock::new();

#[derive(Default)]
struct BinaryTransferManager {
    transfers: BTreeMap<String, BinaryTransfer>,
}

enum BinaryTransfer {
    Read {
        plugin_id: String,
        session_id: String,
        bytes: Vec<u8>,
        mime_type: String,
        expires_at: Instant,
    },
    Upload {
        plugin_id: String,
        session_id: String,
        project_id: String,
        asset_id: String,
        expected_revision: String,
        mime_type: String,
        declared_size: usize,
        next_chunk: u64,
        bytes: Vec<u8>,
        expires_at: Instant,
    },
    RecoveryUpload {
        plugin_id: String,
        session_id: String,
        project_id: String,
        entity_id: String,
        declared_size: usize,
        next_chunk: u64,
        bytes: Vec<u8>,
        expires_at: Instant,
    },
    Create {
        plugin_id: String,
        session_id: String,
        project_id: String,
        entity_id: String,
        declared_size: usize,
        next_chunk: u64,
        bytes: Vec<u8>,
        expires_at: Instant,
    },
    ImageImport {
        plugin_id: String,
        session_id: String,
        project_id: String,
        name: String,
        filename: String,
        mime_type: String,
        declared_size: usize,
        next_chunk: u64,
        bytes: Vec<u8>,
        expires_at: Instant,
    },
    VectorCreate {
        plugin_id: String,
        session_id: String,
        project_id: String,
        name: String,
        generation: serde_json::Value,
        declared_size: usize,
        next_chunk: u64,
        bytes: Vec<u8>,
        expires_at: Instant,
    },
    PhysicalCreate {
        plugin_id: String,
        session_id: String,
        project_id: String,
        name: String,
        generation: serde_json::Value,
        declared_size: usize,
        next_chunk: u64,
        bytes: Vec<u8>,
        expires_at: Instant,
    },
    VectorReplace {
        plugin_id: String,
        session_id: String,
        project_id: String,
        asset_id: String,
        expected_revision: String,
        declared_size: usize,
        next_chunk: u64,
        bytes: Vec<u8>,
        expires_at: Instant,
    },
}

impl BinaryTransferManager {
    fn cleanup(&mut self) {
        let now = Instant::now();
        self.transfers.retain(|_, transfer| match transfer {
            BinaryTransfer::Read { expires_at, .. }
            | BinaryTransfer::Upload { expires_at, .. }
            | BinaryTransfer::RecoveryUpload { expires_at, .. }
            | BinaryTransfer::Create { expires_at, .. }
            | BinaryTransfer::ImageImport { expires_at, .. }
            | BinaryTransfer::VectorCreate { expires_at, .. }
            | BinaryTransfer::PhysicalCreate { expires_at, .. }
            | BinaryTransfer::VectorReplace { expires_at, .. } => *expires_at > now,
        });
    }

    fn token(&mut self, transfer: BinaryTransfer) -> String {
        self.cleanup();
        let token = uuid::Uuid::new_v4().to_string();
        self.transfers.insert(token.clone(), transfer);
        token
    }

    fn take_read(
        &mut self,
        token: &str,
        plugin_id: &str,
        session_id: &str,
    ) -> Result<(Vec<u8>, String), String> {
        self.cleanup();
        let Some(transfer) = self.transfers.remove(token) else {
            return Err("asset read handle is invalid or expired".into());
        };
        match transfer {
            BinaryTransfer::Read {
                plugin_id: owner,
                session_id: expected_session,
                bytes,
                mime_type,
                ..
            } if owner == plugin_id && expected_session == session_id => Ok((bytes, mime_type)),
            other => {
                self.transfers.insert(token.into(), other);
                Err("asset read handle is not valid for this plugin".into())
            }
        }
    }

    fn append_upload(
        &mut self,
        token: &str,
        plugin_id: &str,
        session_id: &str,
        chunk: u64,
        body: &[u8],
    ) -> Result<usize, String> {
        self.cleanup();
        let transfer = self
            .transfers
            .get_mut(token)
            .ok_or_else(|| "asset upload handle is invalid or expired".to_string())?;
        let (owner, expected_session, next_chunk, declared_size, bytes) = match transfer {
            BinaryTransfer::Upload {
                plugin_id,
                session_id,
                next_chunk,
                declared_size,
                bytes,
                ..
            }
            | BinaryTransfer::RecoveryUpload {
                plugin_id,
                session_id,
                next_chunk,
                declared_size,
                bytes,
                ..
            }
            | BinaryTransfer::Create {
                plugin_id,
                session_id,
                next_chunk,
                declared_size,
                bytes,
                ..
            }
            | BinaryTransfer::ImageImport {
                plugin_id,
                session_id,
                next_chunk,
                declared_size,
                bytes,
                ..
            }
            | BinaryTransfer::VectorCreate {
                plugin_id,
                session_id,
                next_chunk,
                declared_size,
                bytes,
                ..
            }
            | BinaryTransfer::PhysicalCreate {
                plugin_id,
                session_id,
                next_chunk,
                declared_size,
                bytes,
                ..
            }
            | BinaryTransfer::VectorReplace {
                plugin_id,
                session_id,
                next_chunk,
                declared_size,
                bytes,
                ..
            } => (plugin_id, session_id, next_chunk, declared_size, bytes),
            _ => return Err("asset handle is not an upload handle".into()),
        };
        if owner != plugin_id || expected_session != session_id {
            return Err("asset upload handle is not valid for this plugin session".into());
        }
        if *next_chunk != chunk {
            return Err("asset upload chunks must be sequential".into());
        }
        if body.len() > daena_plugin_host::runtime::MAX_RPC_BYTES
            || bytes.len().saturating_add(body.len()) > *declared_size
            || bytes.len().saturating_add(body.len()) > MAX_ASSET_TRANSFER_BYTES
        {
            return Err("asset upload exceeds its declared or host limit".into());
        }
        bytes.extend_from_slice(body);
        *next_chunk += 1;
        Ok(bytes.len())
    }

    fn prepare_upload(
        &mut self,
        token: &str,
        plugin_id: &str,
        session_id: &str,
        project_id: &str,
        content_hash: &str,
    ) -> Result<(AssetReplaceInput, Vec<u8>, String), String> {
        self.cleanup();
        let transfer = self
            .transfers
            .get(token)
            .ok_or_else(|| "asset upload handle is invalid or expired".to_string())?;
        match transfer {
            BinaryTransfer::Upload {
                plugin_id: owner,
                session_id: expected_session,
                project_id: expected_project,
                asset_id,
                expected_revision,
                mime_type,
                declared_size,
                bytes,
                ..
            } if owner == plugin_id
                && expected_session == session_id
                && expected_project == project_id =>
            {
                if bytes.len() != *declared_size {
                    return Err("asset upload is incomplete".into());
                }
                Ok((
                    AssetReplaceInput {
                        asset_id: asset_id.clone(),
                        content_hash: content_hash.into(),
                        size: *declared_size as i64,
                        mime_type: mime_type.clone(),
                    },
                    bytes.clone(),
                    expected_revision.clone(),
                ))
            }
            _ => Err("asset upload handle is not valid for this session or project".into()),
        }
    }

    fn prepare_recovery_upload(
        &mut self,
        token: &str,
        plugin_id: &str,
        session_id: &str,
        project_id: &str,
        content_hash: &str,
    ) -> Result<(String, Vec<u8>), String> {
        self.cleanup();
        let transfer = self
            .transfers
            .get(token)
            .ok_or_else(|| "asset upload handle is invalid or expired".to_string())?;
        match transfer {
            BinaryTransfer::RecoveryUpload {
                plugin_id: owner,
                session_id: expected_session,
                project_id: expected_project,
                entity_id,
                declared_size,
                bytes,
                ..
            } if owner == plugin_id
                && expected_session == session_id
                && expected_project == project_id =>
            {
                if bytes.len() != *declared_size {
                    return Err("asset upload is incomplete".into());
                }
                let digest = format!("sha256:{:x}", Sha256::digest(bytes));
                if digest != content_hash {
                    return Err("asset upload content hash does not match bytes".into());
                }
                Ok((entity_id.clone(), bytes.clone()))
            }
            _ => Err("asset upload handle is not valid for this session or project".into()),
        }
    }

    fn prepare_create_upload(
        &mut self,
        token: &str,
        plugin_id: &str,
        session_id: &str,
        project_id: &str,
        content_hash: &str,
    ) -> Result<(String, Vec<u8>), String> {
        self.cleanup();
        let transfer = self
            .transfers
            .get(token)
            .ok_or_else(|| "asset upload handle is invalid or expired".to_string())?;
        match transfer {
            BinaryTransfer::Create {
                plugin_id: owner,
                session_id: expected_session,
                project_id: expected_project,
                entity_id,
                declared_size,
                bytes,
                ..
            } if owner == plugin_id
                && expected_session == session_id
                && expected_project == project_id =>
            {
                if bytes.len() != *declared_size {
                    return Err("asset upload is incomplete".into());
                }
                let digest = format!("sha256:{:x}", Sha256::digest(bytes));
                if digest != content_hash {
                    return Err("asset upload content hash does not match bytes".into());
                }
                Ok((entity_id.clone(), bytes.clone()))
            }
            _ => Err("asset upload handle is not valid for this session or project".into()),
        }
    }

    fn prepare_image_import(
        &mut self,
        token: &str,
        plugin_id: &str,
        session_id: &str,
        project_id: &str,
        content_hash: &str,
    ) -> Result<(String, String, String, Vec<u8>), String> {
        self.cleanup();
        let transfer = self
            .transfers
            .get(token)
            .ok_or_else(|| "asset upload handle is invalid or expired".to_string())?;
        match transfer {
            BinaryTransfer::ImageImport {
                plugin_id: owner,
                session_id: expected_session,
                project_id: expected_project,
                name,
                filename,
                mime_type,
                declared_size,
                bytes,
                ..
            } if owner == plugin_id
                && expected_session == session_id
                && expected_project == project_id =>
            {
                if bytes.len() != *declared_size {
                    return Err("asset upload is incomplete".into());
                }
                let digest = format!("sha256:{:x}", Sha256::digest(bytes));
                if digest != content_hash {
                    return Err("asset upload content hash does not match bytes".into());
                }
                Ok((
                    name.clone(),
                    filename.clone(),
                    mime_type.clone(),
                    bytes.clone(),
                ))
            }
            _ => Err("asset upload handle is not valid for this session or project".into()),
        }
    }

    fn prepare_vector_create(
        &mut self,
        token: &str,
        plugin_id: &str,
        session_id: &str,
        project_id: &str,
        content_hash: &str,
    ) -> Result<(String, serde_json::Value, Vec<u8>), String> {
        self.cleanup();
        let transfer = self
            .transfers
            .get(token)
            .ok_or_else(|| "asset upload handle is invalid or expired".to_string())?;
        match transfer {
            BinaryTransfer::VectorCreate {
                plugin_id: owner,
                session_id: expected_session,
                project_id: expected_project,
                name,
                generation,
                declared_size,
                bytes,
                ..
            } if owner == plugin_id
                && expected_session == session_id
                && expected_project == project_id =>
            {
                if bytes.len() != *declared_size {
                    return Err("asset upload is incomplete".into());
                }
                let digest = format!("sha256:{:x}", Sha256::digest(bytes));
                if digest != content_hash {
                    return Err("asset upload content hash does not match bytes".into());
                }
                Ok((name.clone(), generation.clone(), bytes.clone()))
            }
            _ => Err("asset upload handle is not valid for this session or project".into()),
        }
    }

    fn prepare_physical_create(
        &mut self,
        token: &str,
        plugin_id: &str,
        session_id: &str,
        project_id: &str,
        content_hash: &str,
    ) -> Result<(String, serde_json::Value, Vec<u8>), String> {
        self.cleanup();
        let transfer = self
            .transfers
            .get(token)
            .ok_or_else(|| "asset upload handle is invalid or expired".to_string())?;
        match transfer {
            BinaryTransfer::PhysicalCreate {
                plugin_id: owner,
                session_id: expected_session,
                project_id: expected_project,
                name,
                generation,
                declared_size,
                bytes,
                ..
            } if owner == plugin_id
                && expected_session == session_id
                && expected_project == project_id =>
            {
                if bytes.len() != *declared_size {
                    return Err("asset upload is incomplete".into());
                }
                let digest = format!("sha256:{:x}", Sha256::digest(bytes));
                if digest != content_hash {
                    return Err("asset upload content hash does not match bytes".into());
                }
                Ok((name.clone(), generation.clone(), bytes.clone()))
            }
            _ => Err("asset upload handle is not valid for this session or project".into()),
        }
    }

    fn prepare_vector_replace(
        &mut self,
        token: &str,
        plugin_id: &str,
        session_id: &str,
        project_id: &str,
        content_hash: &str,
    ) -> Result<(String, String, Vec<u8>), String> {
        self.cleanup();
        let transfer = self
            .transfers
            .get(token)
            .ok_or_else(|| "asset upload handle is invalid or expired".to_string())?;
        match transfer {
            BinaryTransfer::VectorReplace {
                plugin_id: owner,
                session_id: expected_session,
                project_id: expected_project,
                asset_id,
                expected_revision,
                declared_size,
                bytes,
                ..
            } if owner == plugin_id
                && expected_session == session_id
                && expected_project == project_id =>
            {
                if bytes.len() != *declared_size {
                    return Err("asset upload is incomplete".into());
                }
                let digest = format!("sha256:{:x}", Sha256::digest(bytes));
                if digest != content_hash {
                    return Err("asset upload content hash does not match bytes".into());
                }
                Ok((asset_id.clone(), expected_revision.clone(), bytes.clone()))
            }
            _ => Err("asset upload handle is not valid for this session or project".into()),
        }
    }

    fn complete_upload(
        &mut self,
        token: &str,
        plugin_id: &str,
        session_id: &str,
    ) -> Result<(), String> {
        let Some(transfer) = self.transfers.remove(token) else {
            return Err("asset upload handle is invalid or expired".into());
        };
        match transfer {
            BinaryTransfer::Upload {
                plugin_id: owner,
                session_id: expected_session,
                ..
            }
            | BinaryTransfer::RecoveryUpload {
                plugin_id: owner,
                session_id: expected_session,
                ..
            }
            | BinaryTransfer::Create {
                plugin_id: owner,
                session_id: expected_session,
                ..
            }
            | BinaryTransfer::ImageImport {
                plugin_id: owner,
                session_id: expected_session,
                ..
            }
            | BinaryTransfer::VectorCreate {
                plugin_id: owner,
                session_id: expected_session,
                ..
            }
            | BinaryTransfer::PhysicalCreate {
                plugin_id: owner,
                session_id: expected_session,
                ..
            }
            | BinaryTransfer::VectorReplace {
                plugin_id: owner,
                session_id: expected_session,
                ..
            } if owner == plugin_id && expected_session == session_id => Ok(()),
            other => {
                self.transfers.insert(token.into(), other);
                Err("asset upload handle is not valid for this plugin".into())
            }
        }
    }

    fn cancel(&mut self, token: &str, plugin_id: &str) -> Result<(), String> {
        self.cleanup();
        let Some(transfer) = self.transfers.get(token) else {
            return Ok(());
        };
        let owner = match transfer {
            BinaryTransfer::Read { plugin_id, .. }
            | BinaryTransfer::Upload { plugin_id, .. }
            | BinaryTransfer::RecoveryUpload { plugin_id, .. }
            | BinaryTransfer::Create { plugin_id, .. }
            | BinaryTransfer::ImageImport { plugin_id, .. }
            | BinaryTransfer::VectorCreate { plugin_id, .. }
            | BinaryTransfer::PhysicalCreate { plugin_id, .. }
            | BinaryTransfer::VectorReplace { plugin_id, .. } => plugin_id,
        };
        if owner != plugin_id {
            return Err("asset handle is not valid for this plugin".into());
        }
        self.transfers.remove(token);
        Ok(())
    }
}

#[derive(Default)]
struct ProjectWatcher {
    stop: Option<mpsc::Sender<()>>,
    filesystem: Option<RecommendedWatcher>,
}

type SharedProjectWatcher = Arc<Mutex<ProjectWatcher>>;

fn stop_project_watcher(watcher: &SharedProjectWatcher) -> Result<(), String> {
    let stop = {
        let mut watcher = watcher
            .lock()
            .map_err(|_| "project watcher lock poisoned".to_string())?;
        let stop = watcher.stop.take();
        watcher.filesystem.take();
        stop
    };
    if let Some(stop) = stop {
        let _ = stop.send(());
    }
    Ok(())
}

fn start_project_watcher(
    app: &tauri::AppHandle,
    state: &SharedCore,
    watcher: &SharedProjectWatcher,
) -> Result<(), String> {
    stop_project_watcher(watcher)?;
    current_session(state)?
        .core
        .lock()
        .map_err(|_| "core lock poisoned".to_string())?
        .info()
        .ok_or_else(|| "project is not open".to_string())?;
    let (stop, receiver) = mpsc::channel();
    let (event_sender, event_receiver) = mpsc::channel();
    let root = current_info(state)?
        .ok_or_else(|| "project is not open".to_string())?
        .root;
    let startup_snapshot = portable_tree_snapshot(std::path::Path::new(&root))?;
    let watched_root = root.clone();
    let callback_root = watched_root.clone();
    let mut filesystem = RecommendedWatcher::new(
        move |event: notify::Result<notify::Event>| {
            let paths = event
                .ok()
                .into_iter()
                .flat_map(|event| event.paths)
                .filter_map(|path| {
                    watched_portable_path(std::path::Path::new(&callback_root), &path)
                })
                .collect::<BTreeSet<_>>();
            if !paths.is_empty() {
                let _ = event_sender.send(paths.into_iter().collect::<Vec<_>>());
            }
        },
        notify::Config::default(),
    )
    .map_err(|error| format!("start filesystem watcher: {error}"))?;
    filesystem
        .watch(std::path::Path::new(&root), RecursiveMode::Recursive)
        .map_err(|error| format!("watch project root: {error}"))?;
    watcher
        .lock()
        .map_err(|_| "project watcher lock poisoned".to_string())?
        .stop = Some(stop);
    watcher
        .lock()
        .map_err(|_| "project watcher lock poisoned".to_string())?
        .filesystem = Some(filesystem);
    let app = app.clone();
    let watched_core = state.clone();
    let startup_filter_until = Instant::now() + Duration::from_secs(1);
    thread::spawn(move || loop {
        match receiver.try_recv() {
            Ok(()) | Err(mpsc::TryRecvError::Disconnected) => return,
            Err(mpsc::TryRecvError::Empty) => {}
        }
        let mut paths = BTreeSet::new();
        match event_receiver.recv_timeout(Duration::from_millis(500)) {
            Ok(batch) => paths.extend(batch),
            Err(mpsc::RecvTimeoutError::Timeout) => continue,
            Err(mpsc::RecvTimeoutError::Disconnected) => return,
        }
        while let Ok(batch) = event_receiver.try_recv() {
            paths.extend(batch);
        }
        // A lifecycle transition can stop this thread while it is draining a
        // queued batch. Do not publish that stale batch after the next project
        // has already become current.
        match receiver.try_recv() {
            Ok(()) | Err(mpsc::TryRecvError::Disconnected) => return,
            Err(mpsc::TryRecvError::Empty) => {}
        }
        if Instant::now() < startup_filter_until {
            paths.retain(|path| {
                portable_path_fingerprint(std::path::Path::new(&watched_root), path)
                    .ok()
                    .flatten()
                    != startup_snapshot.get(path).cloned()
            });
        }
        // Mutations are committed to SQLite before the asynchronous exporter
        // rewrites the portable tree. Do not classify that in-flight export
        // as an external edit.
        let app_export_pending = current_info(&watched_core)
            .ok()
            .flatten()
            .map(|info| info.sync.state == "pending")
            .unwrap_or(false);
        if !paths.is_empty() && app_export_pending {
            continue;
        }
        if !paths.is_empty() && portable_checkpoint_is_current(std::path::Path::new(&watched_root))
        {
            continue;
        }
        if !paths.is_empty() {
            let _ = app.emit("project-portable-files-changed", paths);
        }
    });
    Ok(())
}

fn portable_tree_snapshot(root: &std::path::Path) -> Result<BTreeMap<String, String>, String> {
    let mut snapshot = BTreeMap::new();
    for relative in ["project.json", "entities", "plugins", "assets"] {
        let path = root.join(relative);
        if path.exists() {
            collect_portable_snapshot(root, &path, &mut snapshot)?;
        }
    }
    Ok(snapshot)
}

fn collect_portable_snapshot(
    root: &std::path::Path,
    path: &std::path::Path,
    snapshot: &mut BTreeMap<String, String>,
) -> Result<(), String> {
    let relative = path
        .strip_prefix(root)
        .map_err(|error| format!("portable snapshot path: {error}"))?
        .to_string_lossy()
        .replace('\\', "/");
    let metadata = fs::symlink_metadata(path)
        .map_err(|error| format!("portable snapshot metadata: {error}"))?;
    if metadata.is_dir() {
        snapshot.insert(relative, "directory".into());
        let entries =
            fs::read_dir(path).map_err(|error| format!("portable snapshot directory: {error}"))?;
        for entry in entries {
            let entry = entry.map_err(|error| format!("portable snapshot entry: {error}"))?;
            collect_portable_snapshot(root, &entry.path(), snapshot)?;
        }
    } else if metadata.is_file() {
        let bytes = fs::read(path).map_err(|error| format!("portable snapshot file: {error}"))?;
        snapshot.insert(relative, format!("file:{:x}", Sha256::digest(bytes)));
    }
    Ok(())
}

fn portable_path_fingerprint(
    root: &std::path::Path,
    relative: &str,
) -> Result<Option<String>, String> {
    let path = root.join(relative);
    let metadata = match fs::symlink_metadata(&path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(format!("portable path metadata: {error}")),
    };
    if metadata.is_dir() {
        return Ok(Some("directory".into()));
    }
    if metadata.is_file() {
        let bytes = fs::read(path).map_err(|error| format!("portable path file: {error}"))?;
        return Ok(Some(format!("file:{:x}", Sha256::digest(bytes))));
    }
    Ok(Some("other".into()))
}

fn portable_checkpoint_is_current(root: &std::path::Path) -> bool {
    let checkpoint_path = root.join(daena_core::CHECKPOINT_MANIFEST_FILE);
    let Ok(checkpoint) = read_json::<CheckpointManifest>(&checkpoint_path) else {
        return false;
    };
    validate_checkpoint(root, &checkpoint).is_ok()
}

fn watched_portable_path(root: &std::path::Path, path: &std::path::Path) -> Option<String> {
    let relative = path.strip_prefix(root).ok()?;
    let mut components = relative.components();
    let first = components.next()?.as_os_str().to_str()?;
    if matches!(first, ".daena" | ".git")
        || relative.components().any(|component| {
            matches!(
                component.as_os_str().to_str(),
                Some(".daena") | Some(".git")
            )
        })
    {
        return None;
    }
    let filename = relative.file_name()?.to_str()?;
    if filename == ".DS_Store"
        || filename.starts_with(".~")
        || filename.ends_with('~')
        || filename.ends_with(".swp")
        || filename.ends_with(".tmp")
    {
        return None;
    }
    if !matches!(first, "project.json" | "entities" | "plugins" | "assets") {
        return None;
    }
    Some(relative.to_string_lossy().replace('\\', "/"))
}

fn percent_encode(value: &str) -> String {
    value
        .bytes()
        .map(|byte| match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                (byte as char).to_string()
            }
            _ => format!("%{byte:02X}"),
        })
        .collect()
}

fn sanitize_bundled_maps_html(bytes: Vec<u8>) -> Vec<u8> {
    let html = match String::from_utf8(bytes) {
        Ok(html) => html,
        Err(error) => return error.into_bytes(),
    };
    let Some(start) = html.find("<script async src=\"https://www.googletagmanager.com/gtag/js")
    else {
        return html.into_bytes();
    };
    let Some(external_end) = html[start..].find("</script>") else {
        return html.into_bytes();
    };
    let after_external = start + external_end + "</script>".len();
    let Some(inline_start_offset) = html[after_external..].find("<script>") else {
        return html.into_bytes();
    };
    let inline_start = after_external + inline_start_offset;
    let Some(inline_end_offset) = html[inline_start..].find("</script>") else {
        return html.into_bytes();
    };
    let after_inline = inline_start + inline_end_offset + "</script>".len();
    format!("{}{}", &html[..start], &html[after_inline..]).into_bytes()
}

fn bundled_maps_asset(path: &str) -> Option<(Vec<u8>, &'static str)> {
    let relative = if path == "/dist/ui/index.html" {
        "index.html"
    } else if let Some(relative) = path.strip_prefix("/dist/ui/fmg/") {
        relative
    } else if let Some(relative) = path.strip_prefix("/dist/ui/") {
        // Keep serving archives generated before the base-href rewrite. The
        // entrypoint historically emitted relative and absolute FMG URLs,
        // which resolve here instead of under /dist/ui/fmg/.
        relative
    } else {
        path.strip_prefix("/Fantasy-Map-Generator/")?
    };
    if relative.is_empty()
        || relative
            .split('/')
            .any(|part| part.is_empty() || part == "." || part == "..")
    {
        return None;
    }
    // FMG's bridge lives in Daena source rather than the pinned archive.
    // Native Image Maps are rendered by Svelte and never pass through this server.
    if relative == "daena-bridge.js" {
        return Some((
            include_bytes!("../../scripts/fmg-bridge-template.js").to_vec(),
            "text/javascript",
        ));
    }
    let mut archive = zip::ZipArchive::new(Cursor::new(BUNDLED_FMG_ARCHIVE)).ok()?;
    let mut file = archive.by_name(relative).ok()?;
    if file.is_dir() {
        return None;
    }
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes).ok()?;
    if relative == "index.html" {
        bytes = sanitize_bundled_maps_html(bytes);
    }
    let content_type = match Path::new(relative)
        .extension()
        .and_then(|value| value.to_str())
    {
        Some("html") => "text/html",
        Some("js") | Some("mjs") => "text/javascript",
        Some("css") => "text/css",
        Some("json") | Some("webmanifest") => "application/json",
        Some("svg") => "image/svg+xml",
        Some("png") => "image/png",
        Some("jpg") | Some("jpeg") => "image/jpeg",
        Some("webp") => "image/webp",
        Some("woff") => "font/woff",
        Some("woff2") => "font/woff2",
        _ => "application/octet-stream",
    };
    Some((bytes, content_type))
}

fn plugin_asset_response(
    plugin_id: &str,
    request: &tauri::http::Request<Vec<u8>>,
    package_root: Option<&Path>,
    package_manifest: Option<&PluginManifest>,
) -> tauri::http::Response<Vec<u8>> {
    let path = request.uri().path();
    let (bytes, content_type) = if let Some(package_root) = package_root {
        let Some(manifest) = package_manifest else {
            return tauri::http::Response::builder()
                .status(404)
                .body(Vec::new())
                .unwrap();
        };
        let Some(entrypoint) = manifest.entrypoints.ui.as_deref() else {
            return tauri::http::Response::builder()
                .status(404)
                .body(Vec::new())
                .unwrap();
        };
        let Some(relative) = path.strip_prefix('/') else {
            return tauri::http::Response::builder()
                .status(404)
                .body(Vec::new())
                .unwrap();
        };
        if relative.is_empty()
            || Path::new(relative).components().any(|component| {
                matches!(
                    component,
                    Component::CurDir
                        | Component::ParentDir
                        | Component::RootDir
                        | Component::Prefix(_)
                )
            })
        {
            return tauri::http::Response::builder()
                .status(404)
                .body(Vec::new())
                .unwrap();
        }
        let ui_root = package_root
            .join(entrypoint)
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| package_root.to_path_buf());
        let requested = package_root.join(relative);
        let Ok(canonical_ui_root) = ui_root.canonicalize() else {
            return tauri::http::Response::builder()
                .status(404)
                .body(Vec::new())
                .unwrap();
        };
        let Ok(canonical_requested) = requested.canonicalize() else {
            return tauri::http::Response::builder()
                .status(404)
                .body(Vec::new())
                .unwrap();
        };
        if !canonical_requested.starts_with(&canonical_ui_root) || !canonical_requested.is_file() {
            return tauri::http::Response::builder()
                .status(404)
                .body(Vec::new())
                .unwrap();
        }
        let content_type = match requested.extension().and_then(|value| value.to_str()) {
            Some("html") => "text/html",
            Some("js") | Some("mjs") => "text/javascript",
            Some("css") => "text/css",
            Some("json") => "application/json",
            Some("svg") => "image/svg+xml",
            Some("png") => "image/png",
            Some("jpg") | Some("jpeg") => "image/jpeg",
            Some("webp") => "image/webp",
            Some("woff") => "font/woff",
            Some("woff2") => "font/woff2",
            _ => "application/octet-stream",
        };
        let Ok(bytes) = std::fs::read(&canonical_requested) else {
            return tauri::http::Response::builder()
                .status(404)
                .body(Vec::new())
                .unwrap();
        };
        (bytes, content_type)
    } else {
        if plugin_id == "daena.maps" {
            if let Some((bytes, content_type)) = bundled_maps_asset(path) {
                (bytes, content_type)
            } else {
                return tauri::http::Response::builder()
                    .status(404)
                    .body(Vec::new())
                    .unwrap();
            }
        } else {
            let (bytes, content_type): (&[u8], &str) = match (plugin_id, path) {
                ("daena.lore", "/dist/ui/index.html") => (
                    include_bytes!("../plugin-assets/lore/index.html"),
                    "text/html",
                ),
                ("daena.timeline", "/dist/ui/index.html") => (
                    include_bytes!("../plugin-assets/timeline/index.html"),
                    "text/html",
                ),
                ("daena.writing", "/dist/ui/index.html") => (
                    include_bytes!("../plugin-assets/writing/index.html"),
                    "text/html",
                ),
                (_, "/dist/ui/plugin.js") => (
                    include_bytes!("../plugin-assets/shared/plugin.js"),
                    "text/javascript",
                ),
                (_, "/dist/ui/plugin-sdk.js") => (
                    include_bytes!("../../packages/plugin-sdk/dist/index.js"),
                    "text/javascript",
                ),
                (_, "/dist/ui/generated.js") => (
                    include_bytes!("../../packages/plugin-sdk/dist/generated.js"),
                    "text/javascript",
                ),
                (_, "/dist/ui/maps.js") => (
                    include_bytes!("../../packages/plugin-sdk/dist/maps.js"),
                    "text/javascript",
                ),
                (_, "/dist/ui/plugin.css") => (
                    include_bytes!("../plugin-assets/shared/plugin.css"),
                    "text/css",
                ),
                _ => {
                    return tauri::http::Response::builder()
                        .status(404)
                        .body(Vec::new())
                        .unwrap()
                }
            };
            (bytes.to_vec(), content_type)
        }
    };
    let manifest = package_manifest.cloned().or_else(|| {
        let manifest = match plugin_id {
            "daena.lore" => include_str!("../../packages/modules/lore/manifest.json"),
            "daena.timeline" => {
                include_str!("../../packages/modules/timeline/manifest.json")
            }
            "daena.writing" => {
                include_str!("../../packages/modules/writing/manifest.json")
            }
            "daena.maps" => include_str!("../../packages/modules/maps/manifest.json"),
            "daena.language" => {
                include_str!("../../packages/modules/language/manifest.json")
            }
            _ => return None,
        };
        serde_json::from_str::<PluginManifest>(manifest).ok()
    });
    let csp = manifest
        .as_ref()
        .and_then(webview_policy)
        .map(|policy| policy.csp)
        .unwrap_or_else(|| "default-src 'none'".into());
    tauri::http::Response::builder()
        .status(200)
        .header("Content-Type", content_type)
        .header("Content-Security-Policy", csp)
        .header("X-Content-Type-Options", "nosniff")
        .body(bytes)
        .unwrap()
}

fn binary_asset_response(
    plugin_id: &str,
    request: &tauri::http::Request<Vec<u8>>,
    transfers: &SharedBinaryTransfers,
) -> Option<tauri::http::Response<Vec<u8>>> {
    let path = request.uri().path();
    let mut parts = path.split('/');
    if parts.next() != Some("") || parts.next() != Some("__asset") {
        return None;
    }
    let token = parts.next().unwrap_or_default();
    let session_id = request
        .uri()
        .query()
        .and_then(|query| {
            query
                .split('&')
                .find_map(|part| part.strip_prefix("sessionId="))
        })
        .unwrap_or_default();
    if token.is_empty() {
        return Some(json_response(
            serde_json::json!({"error":"asset handle is required"}),
            400,
        ));
    }
    let mut manager = match transfers.lock() {
        Ok(manager) => manager,
        Err(_) => {
            return Some(json_response(
                serde_json::json!({"error":"asset transfer state is unavailable"}),
                500,
            ))
        }
    };
    if request.method().as_str() == "GET" && parts.next().is_none() {
        return Some(match manager.take_read(token, plugin_id, session_id) {
            Ok((bytes, mime_type)) => tauri::http::Response::builder()
                .status(200)
                .header("Content-Type", mime_type)
                .header("Content-Length", bytes.len().to_string())
                .body(bytes)
                .unwrap(),
            Err(error) => json_response(serde_json::json!({"error":error}), 404),
        });
    }
    if request.method().as_str() == "PUT" {
        let chunk = parts.next().and_then(|value| value.parse::<u64>().ok());
        return Some(match chunk {
            Some(chunk) => {
                match manager.append_upload(token, plugin_id, session_id, chunk, request.body()) {
                    Ok(received) => {
                        json_response(serde_json::json!({"received":received,"chunk":chunk}), 200)
                    }
                    Err(error) => json_response(serde_json::json!({"error":error}), 409),
                }
            }
            None => json_response(
                serde_json::json!({"error":"asset upload chunk index is required"}),
                400,
            ),
        });
    }
    Some(json_response(
        serde_json::json!({"error":"unsupported asset transfer request"}),
        405,
    ))
}

fn json_response(value: serde_json::Value, status: u16) -> tauri::http::Response<Vec<u8>> {
    tauri::http::Response::builder()
        .status(status)
        .header("Content-Type", "application/json")
        .header("Access-Control-Allow-Origin", "*")
        .body(serde_json::to_vec(&value).unwrap_or_else(|_| b"{}".to_vec()))
        .unwrap()
}

/// JSON response scoped to a single plugin origin instead of `*`.
fn json_response_with_origin(
    value: serde_json::Value,
    status: u16,
    origin: &str,
) -> tauri::http::Response<Vec<u8>> {
    tauri::http::Response::builder()
        .status(status)
        .header("Content-Type", "application/json")
        .header("Access-Control-Allow-Origin", origin)
        .body(serde_json::to_vec(&value).unwrap_or_else(|_| b"{}".to_vec()))
        .unwrap()
}

/// Navigation allowlist for sandboxed plugin webviews. A plugin document may
/// only navigate within its own `plugin://<id>` origin; every other scheme,
/// host, or external URL is rejected by the native webview.
fn plugin_navigation_allowed(
    url: &tauri::Url,
    policy: &daena_plugin_host::runtime::PluginWebviewPolicy,
) -> bool {
    match url.host_str() {
        Some(host) => {
            url.scheme() == policy.protocol
                && format!("{}://{host}", policy.protocol) == policy.origin
        }
        None => false,
    }
}

/// The request Origin must match the plugin's own `plugin://<id>` origin for
/// bridge POSTs. Some webviews omit the `Origin` header for custom-scheme
/// fetches, so a missing header is tolerated — the webview-label binding and
/// session validation still tie the caller to the plugin. A present but
/// mismatched Origin is always rejected. GET asset requests never carry an
/// Origin and stay allowed.
fn plugin_request_origin_matches(request: &tauri::http::Request<Vec<u8>>, plugin_id: &str) -> bool {
    if request.method() != tauri::http::Method::POST {
        return true;
    }
    let expected = format!(
        "{}://{plugin_id}",
        daena_plugin_host::runtime::plugin_protocol(plugin_id)
    );
    match request.headers().get("Origin").and_then(|value| value.to_str().ok()) {
        Some(origin) => origin.eq_ignore_ascii_case(&expected),
        None => true,
    }
}

fn plugin_protocol_response(
    webview_label: &str,
    request: &tauri::http::Request<Vec<u8>>,
    core: &SharedCore,
    plugins: &SharedPluginHost,
    transfers: &SharedBinaryTransfers,
    ai_runtime: &ai::SharedAiRuntime,
) -> tauri::http::Response<Vec<u8>> {
    if request.body().len() > daena_plugin_host::runtime::MAX_RPC_BYTES {
        return json_response(
            serde_json::json!({"error": "plugin protocol request exceeds payload limit"}),
            413,
        );
    }
    let plugin_id = request.uri().host().unwrap_or_default();
    if plugin_id.is_empty() {
        return json_response(
            serde_json::json!({"error": "plugin protocol requires an ID"}),
            400,
        );
    }
    if plugin_window_label(plugin_id) != webview_label {
        return json_response(
            serde_json::json!({"error": "plugin protocol origin mismatch"}),
            403,
        );
    }
    let plugin_origin = format!(
        "{}://{plugin_id}",
        daena_plugin_host::runtime::plugin_protocol(plugin_id)
    );
    if !plugin_request_origin_matches(request, plugin_id) {
        return json_response_with_origin(
            serde_json::json!({"error": "plugin protocol document origin mismatch"}),
            403,
            &plugin_origin,
        );
    }
    if request.uri().path() != "/__rpc" {
        if let Some(response) = binary_asset_response(plugin_id, request, transfers) {
            return response;
        }
        let project_id = current_info(core).ok().flatten().map(|info| info.root);
        let Some(project_id) = project_id else {
            return tauri::http::Response::builder()
                .status(404)
                .body(Vec::new())
                .unwrap();
        };
        let entry = plugins
            .lock()
            .ok()
            .and_then(|host| host.runtime_entry(&project_id, plugin_id));
        let Some(entry) = entry else {
            return tauri::http::Response::builder()
                .status(404)
                .body(Vec::new())
                .unwrap();
        };
        let package_root =
            (!entry.package_root.as_os_str().is_empty()).then_some(entry.package_root.as_path());
        return plugin_asset_response(plugin_id, request, package_root, Some(&entry.manifest));
    }
    let envelope = match serde_json::from_slice::<serde_json::Value>(request.body()) {
        Ok(value) => value,
        Err(error) => {
            return json_response_with_origin(
                serde_json::json!({"error": error.to_string()}),
                400,
                &plugin_origin,
            )
        }
    };
    let operation = envelope.get("op").and_then(serde_json::Value::as_str);
    let result = match operation {
        Some("bootstrap") => {
            let requested_plugin = envelope.get("pluginId").and_then(serde_json::Value::as_str);
            let project_id = envelope
                .get("projectId")
                .and_then(serde_json::Value::as_str);
            if requested_plugin != Some(plugin_id) || project_id.is_none() {
                Err("plugin bootstrap identity mismatch".to_string())
            } else {
                let project_id = project_id.unwrap_or_default();
                let current_project = current_info(core).ok().flatten().map(|info| info.root);
                if current_project.as_deref() != Some(project_id) {
                    Err("plugin bootstrap project mismatch".into())
                } else {
                    let host = plugins
                        .lock()
                        .map_err(|_| "plugin host lock poisoned".to_string());
                    host.and_then(|mut host| {
                        let session = host
                            .bootstrap(plugin_id, project_id, webview_label)
                            .map_err(|error| error.to_string())?;
                        let entry = host
                            .runtime_entry(project_id, plugin_id)
                            .ok_or_else(|| "plugin was removed during bootstrap".to_string())?;
                        serde_json::to_value(PluginBootstrap {
                            rpc_version: daena_plugin_api::RPC_VERSION,
                            session_id: session.id.clone(),
                            plugin_id: plugin_id.to_string(),
                            project_id: project_id.to_string(),
                            version: entry.manifest.version.clone(),
                            host_api: entry.manifest.host_api.clone(),
                            granted_capabilities: session.grants.iter().cloned().collect(),
                            optional_features: Vec::new(),
                            package_digest: entry.digest,
                            manifest: entry.manifest,
                        })
                        .map_err(|error| error.to_string())
                    })
                }
            }
        }
        Some("rpc") => {
            let request = match envelope
                .get("request")
                .cloned()
                .and_then(|value| serde_json::from_value::<RpcRequest>(value).ok())
            {
                Some(request) => request,
                None => {
                    return json_response_with_origin(
                        serde_json::json!({"ok": false, "error": {"message": "invalid plugin RPC request"}}),
                        400,
                        &plugin_origin,
                    )
                }
            };
            let request_id = request.request_id.clone();
            let mutation_request_id = sanitize_mutation_request_id(&request_id);
            let result = (|| -> Result<serde_json::Value, String> {
                let session = plugins
                    .lock()
                    .map_err(|_| "plugin host lock poisoned".to_string())?
                    .authorize_rpc(webview_label, &request)
                    .map_err(|error| error.message)?;
                validate_broker_payload(&request.method, &request.payload)
                    .map_err(|error| error.to_string())?;
                let current_project = current_info(core)?.map(|info| info.root);
                if current_project.as_deref() != Some(session.project_id.as_str()) {
                    return Err("plugin session is not bound to the open project".into());
                }
                let record_owner_entity_types = request
                    .method
                    .starts_with("record.")
                    .then(|| {
                        let collection = request
                            .payload
                            .get("collection")
                            .and_then(serde_json::Value::as_str)?;
                        plugins.lock().ok()?.record_owner_entity_types(
                            &session.project_id,
                            &session.plugin_id,
                            collection,
                        )
                    })
                    .flatten();
                let publish_payload =
                    (request.method == "event.publish").then(|| request.payload.clone());
                let value = if matches!(
                    request.method.as_str(),
                    "asset.read.begin"
                        | "asset.replace.begin"
                        | "asset.replace.commit"
                        | "asset.transfer.cancel"
                        | "maps.asset.create.begin"
                        | "maps.asset.create.commit"
                        | "maps.image.import.begin"
                        | "maps.image.import.commit"
                        | "maps.vector.create.begin"
                        | "maps.vector.create.commit"
                        | "maps.physical.create.begin"
                        | "maps.physical.create.commit"
                        | "maps.vector.replace.begin"
                        | "maps.vector.replace.commit"
                        | "maps.recovery.export.begin"
                        | "maps.recovery.export.commit"
                ) {
                    let value = dispatch_binary_asset_rpc(
                        core,
                        transfers,
                        &session,
                        &request.method,
                        request.payload,
                        mutation_request_id,
                    )?;
                    match request.method.as_str() {
                        "asset.replace.commit" => {
                            flush_checkpoint_for_shared_core(core, "maps asset replace")?;
                        }
                        "maps.asset.create.commit" => {
                            flush_checkpoint_for_shared_core(core, "maps asset create")?;
                        }
                        "maps.image.import.commit" => {
                            flush_checkpoint_for_shared_core(core, "maps image import")?;
                        }
                        "maps.vector.create.commit" => {
                            flush_checkpoint_for_shared_core(core, "maps vector create")?;
                        }
                        "maps.physical.create.commit" => {
                            flush_checkpoint_for_shared_core(core, "maps physical create")?;
                        }
                        "maps.vector.replace.commit" => {
                            flush_checkpoint_for_shared_core(core, "maps vector replace")?;
                        }
                        _ => {}
                    }
                    value
                } else if matches!(
                    request.method.as_str(),
                    "event.subscribe"
                        | "event.poll"
                        | "event.publish"
                        | "service.call"
                        | "ai.request.start"
                        | "ai.request.poll"
                        | "ai.request.cancel"
                        | "ai.request.result"
                        | "ai.request.citations"
                ) {
                    dispatch_host_rpc(
                        plugins,
                        &session.plugin_id,
                        &session.project_id,
                        &request.method,
                        request.payload,
                        AiBrokerContext {
                            app: APP_HANDLE.get().cloned(),
                            core: Some(core.clone()),
                            settings: None,
                            ai_runtime: ai_runtime.clone(),
                            session_id: session.id.clone(),
                            caller: daena_ai::AiCaller::authorized_plugin(
                                session.plugin_id.clone(),
                                session.project_id.clone(),
                                session.grants.iter().cloned().collect(),
                                vec![format!("project:{}", session.project_id)],
                                session.generation,
                                "pending",
                            ),
                        },
                    )?
                } else {
                    let plugin_id = session.plugin_id.clone();
                    let core_session = current_session(core)?;
                    let mut core = core_session
                        .core
                        .lock()
                        .map_err(|_| "core lock poisoned".to_string())?;
                    dispatch_module_rpc(
                        &mut core,
                        Some(&plugin_id),
                        record_owner_entity_types,
                        &request.method,
                        request.payload,
                        mutation_request_id,
                    )
                    .map_err(|error| error.to_string())?
                };
                if let Some(payload) = publish_payload {
                    forward_maps_state_event(&payload);
                }
                Ok(
                    serde_json::json!({"rpcVersion": 1, "requestId": request_id, "ok": true, "result": value}),
                )
            })();
            match result {
                Ok(value) => Ok(value),
                Err(error) => Ok(
                    serde_json::json!({"rpcVersion": 1, "requestId": request_id, "ok": false, "error": {"code": "core.error", "message": error, "retryable": false}}),
                ),
            }
        }
        _ => Err("unknown plugin protocol operation".into()),
    };
    match result {
        Ok(value) => json_response_with_origin(value, 200, &plugin_origin),
        Err(error) => json_response_with_origin(
            serde_json::json!({"error": error}),
            400,
            &plugin_origin,
        ),
    }
}

/// Forwards plugin-published `daena.maps/state@1` and `daena.maps/selection@1`
/// events to the shell as `maps-state` / `maps-selection` Tauri events. The
/// shell owns the wrapper chrome (save chip, conflict banner, capture tool)
/// around the child webview, which has no other way to reach it. Forwarding
/// is a hint only; the shell re-queries canonical records before acting.
fn forward_maps_state_event(payload: &serde_json::Value) {
    let Some(event_type) = payload.get("type").and_then(serde_json::Value::as_str) else {
        return;
    };
    let Some(app) = APP_HANDLE.get() else {
        return;
    };
    let state = payload.get("payload").cloned().unwrap_or_default();
    if event_type.starts_with("daena.maps/state@") {
        let _ = app.emit(
            "maps-state",
            serde_json::json!({
                "mapEntityId": state.get("mapEntityId").cloned().unwrap_or_default(),
                "status": state.get("status").and_then(serde_json::Value::as_str).unwrap_or("state"),
                "detail": state.get("detail").cloned().unwrap_or(serde_json::Value::Null),
            }),
        );
    } else if event_type.starts_with("daena.maps/selection@") {
        let _ = app.emit(
            "maps-selection",
            serde_json::json!({
                "mapEntityId": state.get("mapEntityId").cloned().unwrap_or_default(),
                "anchor": state.get("anchor").cloned().unwrap_or(serde_json::Value::Null),
            }),
        );
    }
}

fn plugin_asset_url(token: &str, session_id: &str, chunk: Option<u64>) -> String {
    match chunk {
        Some(index) => format!("/__asset/{token}/{index}?sessionId={session_id}"),
        None => format!("/__asset/{token}?sessionId={session_id}"),
    }
}

fn dispatch_binary_asset_rpc(
    core: &SharedCore,
    transfers: &SharedBinaryTransfers,
    session: &Session,
    method: &str,
    payload: serde_json::Value,
    request_id: Option<&str>,
) -> Result<serde_json::Value, String> {
    let core_session = current_session(core)?;
    let project = core_session
        .core
        .lock()
        .map_err(|_| "core lock poisoned".to_string())?;
    let project = project
        .project(AuthorityContext::plugin())
        .map_err(|e| e.to_string())?;
    let asset_id = payload.get("assetId").and_then(serde_json::Value::as_str);
    let namespace = payload.get("namespace").and_then(serde_json::Value::as_str);
    let mut manager = transfers
        .lock()
        .map_err(|_| "asset transfer state is unavailable".to_string())?;
    match method {
        "asset.read.begin" => {
            let asset = project
                .asset(
                    asset_id
                        .ok_or_else(|| "assetId is required".to_string())?
                        .into(),
                )
                .map_err(|e| e.to_string())?;
            if namespace != Some(asset.namespace.as_str()) {
                return Err("asset namespace does not match the owned asset".into());
            }
            let bytes = project
                .asset_bytes(asset.id.clone())
                .map_err(|e| e.to_string())?;
            if bytes.len() > MAX_ASSET_TRANSFER_BYTES {
                return Err("asset exceeds host transfer limit".into());
            }
            if asset.size < 0 || asset.size as usize != bytes.len() {
                return Err("asset metadata size does not match its bytes".into());
            }
            let token = manager.token(BinaryTransfer::Read {
                plugin_id: session.plugin_id.clone(),
                session_id: session.id.clone(),
                bytes,
                mime_type: asset.mime_type.clone(),
                expires_at: Instant::now() + ASSET_TRANSFER_TTL,
            });
            Ok(
                serde_json::json!({"handle":token,"url":plugin_asset_url(&token, &session.id, None),"size":asset.size,"contentHash":asset.content_hash,"mimeType":asset.mime_type,"revision":asset.revision}),
            )
        }
        "asset.replace.begin" => {
            let asset = project
                .asset(
                    asset_id
                        .ok_or_else(|| "assetId is required".to_string())?
                        .into(),
                )
                .map_err(|e| e.to_string())?;
            if namespace != Some(asset.namespace.as_str()) {
                return Err("asset namespace does not match the owned asset".into());
            }
            let size_value = payload
                .get("size")
                .and_then(serde_json::Value::as_u64)
                .ok_or_else(|| "size is required".to_string())?;
            if size_value > MAX_ASSET_TRANSFER_BYTES as u64 {
                return Err("asset exceeds host transfer limit".into());
            }
            let size = size_value as usize;
            let mime_type = payload
                .get("mimeType")
                .and_then(serde_json::Value::as_str)
                .ok_or_else(|| "mimeType is required".to_string())?;
            let expected_revision = payload
                .get("expectedRevision")
                .and_then(serde_json::Value::as_str)
                .ok_or_else(|| "expectedRevision is required".to_string())?;
            if expected_revision != asset.revision {
                return Err("asset revision conflict".into());
            }
            let token = manager.token(BinaryTransfer::Upload {
                plugin_id: session.plugin_id.clone(),
                session_id: session.id.clone(),
                project_id: session.project_id.clone(),
                asset_id: asset.id,
                expected_revision: expected_revision.into(),
                mime_type: mime_type.into(),
                declared_size: size,
                next_chunk: 0,
                bytes: Vec::with_capacity(size.min(MAX_ASSET_TRANSFER_BYTES)),
                expires_at: Instant::now() + ASSET_TRANSFER_TTL,
            });
            Ok(
                serde_json::json!({"handle":token,"url":plugin_asset_url(&token, &session.id, Some(0)),"maxChunkBytes":daena_plugin_host::runtime::MAX_RPC_BYTES,"expiresInMs":ASSET_TRANSFER_TTL.as_millis()}),
            )
        }
        "asset.replace.commit" => {
            let token = payload
                .get("handle")
                .and_then(serde_json::Value::as_str)
                .ok_or_else(|| "handle is required".to_string())?;
            let content_hash = payload
                .get("contentHash")
                .and_then(serde_json::Value::as_str)
                .ok_or_else(|| "contentHash is required".to_string())?;
            let (input, bytes, expected_revision) = manager.prepare_upload(
                token,
                &session.plugin_id,
                &session.id,
                &session.project_id,
                content_hash,
            )?;
            drop(manager);
            let asset = project
                .replace_asset_bytes_with_request(input, bytes, &expected_revision, request_id)
                .map_err(|e| e.to_string())?;
            let mut manager = transfers
                .lock()
                .map_err(|_| "asset transfer state is unavailable".to_string())?;
            manager.complete_upload(token, &session.plugin_id, &session.id)?;
            serde_json::to_value(asset).map_err(|e| e.to_string())
        }
        "asset.transfer.cancel" => {
            let token = payload
                .get("handle")
                .and_then(serde_json::Value::as_str)
                .ok_or_else(|| "handle is required".to_string())?;
            manager.cancel(token, &session.plugin_id)?;
            Ok(serde_json::Value::Null)
        }
        "maps.asset.create.begin" => {
            let entity_id = payload
                .get("mapEntityId")
                .and_then(serde_json::Value::as_str)
                .ok_or_else(|| "mapEntityId is required".to_string())?;
            let exists = project
                .list_entities()
                .map_err(|e| e.to_string())?
                .into_iter()
                .any(|entity| {
                    entity.id == entity_id
                        && entity.entity_type.as_deref() == Some(daena_core::maps::MAP_ENTITY_TYPE)
                });
            if !exists {
                return Err("map entity not found".into());
            }
            let size_value = payload
                .get("size")
                .and_then(serde_json::Value::as_u64)
                .ok_or_else(|| "size is required".to_string())?;
            if size_value > MAX_ASSET_TRANSFER_BYTES as u64 {
                return Err("asset exceeds host transfer limit".into());
            }
            let size = size_value as usize;
            if let Some(mime_type) = payload.get("mimeType") {
                if !mime_type.is_string() {
                    return Err("mimeType must be a string".into());
                }
            }
            let token = manager.token(BinaryTransfer::Create {
                plugin_id: session.plugin_id.clone(),
                session_id: session.id.clone(),
                project_id: session.project_id.clone(),
                entity_id: entity_id.into(),
                declared_size: size,
                next_chunk: 0,
                bytes: Vec::with_capacity(size.min(MAX_ASSET_TRANSFER_BYTES)),
                expires_at: Instant::now() + ASSET_TRANSFER_TTL,
            });
            Ok(
                serde_json::json!({"handle":token,"url":plugin_asset_url(&token, &session.id, Some(0)),"maxChunkBytes":daena_plugin_host::runtime::MAX_RPC_BYTES,"expiresInMs":ASSET_TRANSFER_TTL.as_millis()}),
            )
        }
        "maps.asset.create.commit" => {
            let token = payload
                .get("handle")
                .and_then(serde_json::Value::as_str)
                .ok_or_else(|| "handle is required".to_string())?;
            let content_hash = payload
                .get("contentHash")
                .and_then(serde_json::Value::as_str)
                .ok_or_else(|| "contentHash is required".to_string())?;
            let (entity_id, bytes) = manager.prepare_create_upload(
                token,
                &session.plugin_id,
                &session.id,
                &session.project_id,
                content_hash,
            )?;
            drop(manager);
            let source_path = std::env::temp_dir().join(format!("daena-map-{entity_id}.map"));
            std::fs::write(&source_path, &bytes)
                .map_err(|error| format!("write map source temp file: {error}"))?;
            let result = project
                .register_asset_file_with_request(
                    AssetFileInput {
                        entity_id: entity_id.clone(),
                        namespace: daena_core::maps::MAP_NAMESPACE.into(),
                        source_path: source_path.to_string_lossy().into_owned(),
                        filename: "map.map".into(),
                        mime_type: "application/x-fmg-map".into(),
                    },
                    request_id,
                )
                .map_err(|e| e.to_string());
            let _ = std::fs::remove_file(&source_path);
            let asset = result?;
            // Link the new source into the map descriptor in the same commit path.
            // Leaving this to a follow-up field.set caused orphan .map assets when
            // that call failed: retries then used asset.replace and reported Saved
            // while sourceAssetId stayed null, so the welcome list hid the map.
            let fields = project
                .list_fields(entity_id.clone())
                .map_err(|e| e.to_string())?;
            let mut descriptor = fields
                .into_iter()
                .find(|field| {
                    field.namespace == daena_core::maps::MAP_NAMESPACE && field.key == "map"
                })
                .map(|field| field.value)
                .unwrap_or_else(|| {
                    serde_json::json!({
                        "schemaVersion": 1,
                        "provider": {"id": "azgaar-fmg", "adapterVersion": 1, "sourceFormat": "fmg-map"},
                        "sourceAssetId": null,
                        "previewAssetId": null,
                        "defaultView": {"center": [0.5, 0.5], "zoom": 1}
                    })
                });
            if let Some(object) = descriptor.as_object_mut() {
                object.insert(
                    "sourceAssetId".into(),
                    serde_json::Value::String(asset.id.clone()),
                );
            }
            project
                .set_field_with_request(
                    FieldValue {
                        entity_id,
                        namespace: daena_core::maps::MAP_NAMESPACE.into(),
                        key: "map".into(),
                        value: descriptor,
                        revision: String::new(),
                    },
                    // Always a fresh mutation id: reusing the upload request id
                    // would no-op after register_asset already committed it.
                    None,
                )
                .map_err(|e| e.to_string())?;
            let mut manager = transfers
                .lock()
                .map_err(|_| "asset transfer state is unavailable".to_string())?;
            manager.complete_upload(token, &session.plugin_id, &session.id)?;
            serde_json::to_value(asset).map_err(|e| e.to_string())
        }
        "maps.image.import.begin" => {
            let name = payload
                .get("name")
                .and_then(serde_json::Value::as_str)
                .ok_or_else(|| "name is required".to_string())?;
            let filename = payload
                .get("filename")
                .and_then(serde_json::Value::as_str)
                .ok_or_else(|| "filename is required".to_string())?;
            let mime_type = payload
                .get("mimeType")
                .and_then(serde_json::Value::as_str)
                .ok_or_else(|| "mimeType is required".to_string())?;
            daena_core::maps::source_format_for_mime(mime_type).map_err(|e| e.to_string())?;
            let size_value = payload
                .get("size")
                .and_then(serde_json::Value::as_u64)
                .ok_or_else(|| "size is required".to_string())?;
            if size_value > MAX_ASSET_TRANSFER_BYTES as u64
                || size_value > daena_core::maps::IMAGE_MAX_ENCODED_BYTES as u64
            {
                return Err("asset exceeds host transfer limit".into());
            }
            let size = size_value as usize;
            let token = manager.token(BinaryTransfer::ImageImport {
                plugin_id: session.plugin_id.clone(),
                session_id: session.id.clone(),
                project_id: session.project_id.clone(),
                name: name.into(),
                filename: filename.into(),
                mime_type: mime_type.into(),
                declared_size: size,
                next_chunk: 0,
                bytes: Vec::with_capacity(size.min(MAX_ASSET_TRANSFER_BYTES)),
                expires_at: Instant::now() + ASSET_TRANSFER_TTL,
            });
            Ok(
                serde_json::json!({"handle":token,"url":plugin_asset_url(&token, &session.id, Some(0)),"maxChunkBytes":daena_plugin_host::runtime::MAX_RPC_BYTES,"expiresInMs":ASSET_TRANSFER_TTL.as_millis()}),
            )
        }
        "maps.image.import.commit" => {
            let token = payload
                .get("handle")
                .and_then(serde_json::Value::as_str)
                .ok_or_else(|| "handle is required".to_string())?;
            let content_hash = payload
                .get("contentHash")
                .and_then(serde_json::Value::as_str)
                .ok_or_else(|| "contentHash is required".to_string())?;
            let prepared = manager.prepare_image_import(
                token,
                &session.plugin_id,
                &session.id,
                &session.project_id,
                content_hash,
            );
            drop(manager);
            let imported = match prepared {
                Ok((name, filename, mime_type, bytes)) => {
                    let imported = project
                        .import_image_map(name, bytes, mime_type, filename, request_id)
                        .map_err(|e| e.to_string())?;
                    let mut manager = transfers
                        .lock()
                        .map_err(|_| "asset transfer state is unavailable".to_string())?;
                    let _ = manager.complete_upload(token, &session.plugin_id, &session.id);
                    imported
                }
                Err(error) => project
                    .replay_imported_image_map(request_id)
                    .map_err(|e| e.to_string())?
                    .ok_or(error)?,
            };
            serde_json::to_value(imported).map_err(|e| e.to_string())
        }
        "maps.vector.create.begin" => {
            let name = payload
                .get("name")
                .and_then(serde_json::Value::as_str)
                .ok_or_else(|| "name is required".to_string())?;
            let generation = payload
                .get("generation")
                .cloned()
                .ok_or_else(|| "generation is required".to_string())?;
            daena_core::maps::vector::validate_generation(&generation)
                .map_err(|e| e.to_string())?;
            let size_value = payload
                .get("size")
                .and_then(serde_json::Value::as_u64)
                .ok_or_else(|| "size is required".to_string())?;
            if size_value > MAX_ASSET_TRANSFER_BYTES as u64
                || size_value > daena_core::maps::VECTOR_MAX_BYTES as u64
            {
                return Err("asset exceeds host transfer limit".into());
            }
            let size = size_value as usize;
            let token = manager.token(BinaryTransfer::VectorCreate {
                plugin_id: session.plugin_id.clone(),
                session_id: session.id.clone(),
                project_id: session.project_id.clone(),
                name: name.into(),
                generation,
                declared_size: size,
                next_chunk: 0,
                bytes: Vec::with_capacity(size.min(MAX_ASSET_TRANSFER_BYTES)),
                expires_at: Instant::now() + ASSET_TRANSFER_TTL,
            });
            Ok(
                serde_json::json!({"handle":token,"url":plugin_asset_url(&token, &session.id, Some(0)),"maxChunkBytes":daena_plugin_host::runtime::MAX_RPC_BYTES,"expiresInMs":ASSET_TRANSFER_TTL.as_millis()}),
            )
        }
        "maps.vector.create.commit" => {
            let token = payload
                .get("handle")
                .and_then(serde_json::Value::as_str)
                .ok_or_else(|| "handle is required".to_string())?;
            let content_hash = payload
                .get("contentHash")
                .and_then(serde_json::Value::as_str)
                .ok_or_else(|| "contentHash is required".to_string())?;
            let prepared = manager.prepare_vector_create(
                token,
                &session.plugin_id,
                &session.id,
                &session.project_id,
                content_hash,
            );
            drop(manager);
            let accepted = match prepared {
                Ok((name, generation, bytes)) => {
                    let accepted = project
                        .accept_vector_map(name, bytes, generation, request_id)
                        .map_err(|e| e.to_string())?;
                    let mut manager = transfers
                        .lock()
                        .map_err(|_| "asset transfer state is unavailable".to_string())?;
                    let _ = manager.complete_upload(token, &session.plugin_id, &session.id);
                    accepted
                }
                Err(error) => project
                    .replay_accepted_vector_map(request_id)
                    .map_err(|e| e.to_string())?
                    .ok_or(error)?,
            };
            serde_json::to_value(accepted).map_err(|e| e.to_string())
        }
        "maps.physical.create.begin" => {
            let name = payload
                .get("name")
                .and_then(serde_json::Value::as_str)
                .ok_or_else(|| "name is required".to_string())?;
            let generation = payload
                .get("generation")
                .cloned()
                .ok_or_else(|| "generation is required".to_string())?;
            daena_core::maps::physical::validate_generation(&generation)
                .map_err(|e| e.to_string())?;
            let size_value = payload
                .get("size")
                .and_then(serde_json::Value::as_u64)
                .ok_or_else(|| "size is required".to_string())?;
            if size_value > MAX_ASSET_TRANSFER_BYTES as u64
                || size_value > daena_core::maps::PHYSICAL_MAX_SOURCE_BYTES as u64
            {
                return Err("asset exceeds host transfer limit".into());
            }
            let size = size_value as usize;
            let token = manager.token(BinaryTransfer::PhysicalCreate {
                plugin_id: session.plugin_id.clone(),
                session_id: session.id.clone(),
                project_id: session.project_id.clone(),
                name: name.into(),
                generation,
                declared_size: size,
                next_chunk: 0,
                bytes: Vec::with_capacity(size.min(MAX_ASSET_TRANSFER_BYTES)),
                expires_at: Instant::now() + ASSET_TRANSFER_TTL,
            });
            Ok(
                serde_json::json!({"handle":token,"url":plugin_asset_url(&token, &session.id, Some(0)),"maxChunkBytes":daena_plugin_host::runtime::MAX_RPC_BYTES,"expiresInMs":ASSET_TRANSFER_TTL.as_millis()}),
            )
        }
        "maps.physical.create.commit" => {
            let token = payload
                .get("handle")
                .and_then(serde_json::Value::as_str)
                .ok_or_else(|| "handle is required".to_string())?;
            let content_hash = payload
                .get("contentHash")
                .and_then(serde_json::Value::as_str)
                .ok_or_else(|| "contentHash is required".to_string())?;
            let prepared = manager.prepare_physical_create(
                token,
                &session.plugin_id,
                &session.id,
                &session.project_id,
                content_hash,
            );
            drop(manager);
            let accepted = match prepared {
                Ok((name, generation, bytes)) => {
                    let accepted = project
                        .accept_physical_map(name, bytes, generation, request_id)
                        .map_err(|e| e.to_string())?;
                    let mut manager = transfers
                        .lock()
                        .map_err(|_| "asset transfer state is unavailable".to_string())?;
                    let _ = manager.complete_upload(token, &session.plugin_id, &session.id);
                    accepted
                }
                Err(error) => project
                    .replay_accepted_physical_map(request_id)
                    .map_err(|e| e.to_string())?
                    .ok_or(error)?,
            };
            serde_json::to_value(accepted).map_err(|e| e.to_string())
        }
        "maps.vector.replace.begin" => {
            let asset_id = payload
                .get("assetId")
                .and_then(serde_json::Value::as_str)
                .ok_or_else(|| "assetId is required".to_string())?;
            let asset = project.asset(asset_id.into()).map_err(|e| e.to_string())?;
            if asset.namespace != daena_core::maps::MAP_NAMESPACE
                || asset.mime_type != daena_core::maps::VECTOR_MIME
            {
                return Err("replacement target is not a daena-vector source asset".into());
            }
            let size_value = payload
                .get("size")
                .and_then(serde_json::Value::as_u64)
                .ok_or_else(|| "size is required".to_string())?;
            if size_value > MAX_ASSET_TRANSFER_BYTES as u64
                || size_value > daena_core::maps::VECTOR_MAX_BYTES as u64
            {
                return Err("asset exceeds host transfer limit".into());
            }
            let expected_revision = payload
                .get("expectedRevision")
                .and_then(serde_json::Value::as_str)
                .ok_or_else(|| "expectedRevision is required".to_string())?;
            if expected_revision != asset.revision {
                return Err("asset revision conflict".into());
            }
            let size = size_value as usize;
            let token = manager.token(BinaryTransfer::VectorReplace {
                plugin_id: session.plugin_id.clone(),
                session_id: session.id.clone(),
                project_id: session.project_id.clone(),
                asset_id: asset.id,
                expected_revision: expected_revision.into(),
                declared_size: size,
                next_chunk: 0,
                bytes: Vec::with_capacity(size.min(MAX_ASSET_TRANSFER_BYTES)),
                expires_at: Instant::now() + ASSET_TRANSFER_TTL,
            });
            Ok(
                serde_json::json!({"handle":token,"url":plugin_asset_url(&token, &session.id, Some(0)),"maxChunkBytes":daena_plugin_host::runtime::MAX_RPC_BYTES,"expiresInMs":ASSET_TRANSFER_TTL.as_millis()}),
            )
        }
        "maps.vector.replace.commit" => {
            let token = payload
                .get("handle")
                .and_then(serde_json::Value::as_str)
                .ok_or_else(|| "handle is required".to_string())?;
            let content_hash = payload
                .get("contentHash")
                .and_then(serde_json::Value::as_str)
                .ok_or_else(|| "contentHash is required".to_string())?;
            let prepared = manager.prepare_vector_replace(
                token,
                &session.plugin_id,
                &session.id,
                &session.project_id,
                content_hash,
            );
            drop(manager);
            let replaced = match prepared {
                Ok((asset_id, expected_revision, bytes)) => {
                    let replaced = project
                        .replace_vector_source(
                            asset_id,
                            bytes,
                            content_hash.into(),
                            &expected_revision,
                            request_id,
                        )
                        .map_err(|e| e.to_string())?;
                    let mut manager = transfers
                        .lock()
                        .map_err(|_| "asset transfer state is unavailable".to_string())?;
                    let _ = manager.complete_upload(token, &session.plugin_id, &session.id);
                    replaced
                }
                Err(error) => project
                    .replay_replaced_vector_source(request_id)
                    .map_err(|e| e.to_string())?
                    .ok_or(error)?,
            };
            serde_json::to_value(replaced).map_err(|e| e.to_string())
        }
        "maps.recovery.export.begin" => {
            let entity_id = payload
                .get("mapEntityId")
                .and_then(serde_json::Value::as_str)
                .ok_or_else(|| "mapEntityId is required".to_string())?;
            let exists = project
                .list_entities()
                .map_err(|e| e.to_string())?
                .into_iter()
                .any(|entity| {
                    entity.id == entity_id
                        && entity.entity_type.as_deref() == Some(daena_core::maps::MAP_ENTITY_TYPE)
                });
            if !exists {
                return Err("map entity not found".into());
            }
            let size_value = payload
                .get("size")
                .and_then(serde_json::Value::as_u64)
                .ok_or_else(|| "size is required".to_string())?;
            if size_value > MAX_ASSET_TRANSFER_BYTES as u64 {
                return Err("asset exceeds host transfer limit".into());
            }
            let size = size_value as usize;
            let token = manager.token(BinaryTransfer::RecoveryUpload {
                plugin_id: session.plugin_id.clone(),
                session_id: session.id.clone(),
                project_id: session.project_id.clone(),
                entity_id: entity_id.into(),
                declared_size: size,
                next_chunk: 0,
                bytes: Vec::with_capacity(size.min(MAX_ASSET_TRANSFER_BYTES)),
                expires_at: Instant::now() + ASSET_TRANSFER_TTL,
            });
            Ok(
                serde_json::json!({"handle":token,"url":plugin_asset_url(&token, &session.id, Some(0)),"maxChunkBytes":daena_plugin_host::runtime::MAX_RPC_BYTES,"expiresInMs":ASSET_TRANSFER_TTL.as_millis()}),
            )
        }
        "maps.recovery.export.commit" => {
            let token = payload
                .get("handle")
                .and_then(serde_json::Value::as_str)
                .ok_or_else(|| "handle is required".to_string())?;
            let content_hash = payload
                .get("contentHash")
                .and_then(serde_json::Value::as_str)
                .ok_or_else(|| "contentHash is required".to_string())?;
            let (entity_id, bytes) = manager.prepare_recovery_upload(
                token,
                &session.plugin_id,
                &session.id,
                &session.project_id,
                content_hash,
            )?;
            drop(manager);
            let path = project
                .save_map_recovery_copy(&entity_id, &bytes)
                .map_err(|e| e.to_string())?;
            let mut manager = transfers
                .lock()
                .map_err(|_| "asset transfer state is unavailable".to_string())?;
            manager.complete_upload(token, &session.plugin_id, &session.id)?;
            let file_name = path.rsplit('/').next().unwrap_or(&path).to_string();
            Ok(serde_json::json!({"path": path, "fileName": file_name}))
        }
        _ => Err("unknown binary asset operation".into()),
    }
}

fn open_plugin_webview(
    app: &tauri::AppHandle,
    plugin_id: &str,
    project_id: &str,
    host: &PluginHost,
    view_id: Option<&str>,
) -> Result<(), String> {
    let entry = host
        .runtime_entry(project_id, plugin_id)
        .ok_or_else(|| "plugin is not installed".to_string())?;
    if entry.manifest.kind != daena_plugin_api::PluginKind::Sandboxed {
        return Err("only sandboxed plugins have UI webviews".into());
    }
    if host.lifecycle.state(project_id, plugin_id).state != daena_plugin_api::LifecycleState::Active
    {
        return Err("plugin is not active".into());
    }
    let policy =
        webview_policy(&entry.manifest).ok_or_else(|| "plugin has no UI entrypoint".to_string())?;
    validate_plugin_view(&entry.manifest, view_id)?;
    let host_surface = host_surface_for_view(&entry.manifest, view_id);
    if let Some(window) = app.get_webview_window(&policy.label) {
        window.show().map_err(|error| error.to_string())?;
        window.set_focus().map_err(|error| error.to_string())?;
        return Ok(());
    }
    let mut url = format!("{}?project={}", policy.url, percent_encode(project_id));
    append_host_surface_query(&mut url, host_surface);
    if let Some(view_id) = view_id {
        url.push_str("&view=");
        url.push_str(&percent_encode(view_id));
    }
    let navigation_policy = policy.clone();
    tauri::WebviewWindowBuilder::new(
        app,
        policy.label,
        tauri::WebviewUrl::External(
            url.parse()
                .map_err(|error| format!("invalid plugin URL: {error}"))?,
        ),
    )
    .use_https_scheme(true)
    .initialization_script(PLUGIN_WEBVIEW_ISOLATION_SCRIPT)
    .on_navigation(move |url| plugin_navigation_allowed(&url, &navigation_policy))
    .title(entry.manifest.name.clone())
    .inner_size(980.0, 720.0)
    .visible(true)
    .build()
    .map_err(|error| error.to_string())?;
    Ok(())
}

fn host_surface_for_view<'a>(
    manifest: &'a PluginManifest,
    view_id: Option<&str>,
) -> Option<(&'a str, u32)> {
    let view = view_id
        .and_then(|id| manifest.views.iter().find(|view| view.id == id))
        .or_else(|| manifest.views.first())?;
    match &view.renderer {
        daena_plugin_api::ViewRenderer::HostSurface { id, major } => Some((id.as_str(), *major)),
        _ => None,
    }
}

fn append_host_surface_query(url: &mut String, host_surface: Option<(&str, u32)>) {
    let Some((id, major)) = host_surface else {
        return;
    };
    url.push_str("&hostSurface=");
    url.push_str(&percent_encode(id));
    url.push_str("&hostSurfaceMajor=");
    url.push_str(&major.to_string());
    if id == "daena.maps/editor" && major == 1 {
        // FMG uses this explicit host marker to disable browser-only
        // integrations such as its ServiceWorker and OpenWidget import.
        url.push_str("&daena=1");
    }
}

fn close_plugin_webview(app: &tauri::AppHandle, plugin_id: &str) {
    let label = plugin_window_label(plugin_id);
    if let Ok(mut states) = embedded_webview_states().lock() {
        states.remove(&label);
    }
    if let Some(webview) = app.get_webview(&label) {
        let _ = webview.close();
    }
    if let Some(window) = app.get_webview_window(&label) {
        let _ = window.close();
    }
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(deny_unknown_fields)]
struct PluginWebviewBounds {
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    #[serde(rename = "viewportWidth")]
    viewport_width: f64,
    #[serde(rename = "viewportHeight")]
    viewport_height: f64,
}

#[derive(Debug, Clone, Copy)]
struct EmbeddedPluginWebviewState {
    bounds: PluginWebviewBounds,
    ready: bool,
}

fn embedded_webview_states() -> &'static Mutex<BTreeMap<String, EmbeddedPluginWebviewState>> {
    static STATES: OnceLock<Mutex<BTreeMap<String, EmbeddedPluginWebviewState>>> = OnceLock::new();
    STATES.get_or_init(|| Mutex::new(BTreeMap::new()))
}

impl PluginWebviewBounds {
    fn validate(&self) -> Result<(), String> {
        if !self.x.is_finite()
            || !self.y.is_finite()
            || !self.width.is_finite()
            || !self.height.is_finite()
            || !self.viewport_width.is_finite()
            || !self.viewport_height.is_finite()
            || self.x < 0.0
            || self.y < 0.0
            || !(1.0..=10_000.0).contains(&self.width)
            || !(1.0..=10_000.0).contains(&self.height)
            || !(1.0..=10_000.0).contains(&self.viewport_width)
            || !(1.0..=10_000.0).contains(&self.viewport_height)
        {
            return Err("plugin webview bounds are invalid".into());
        }
        Ok(())
    }
}

fn scale_plugin_bounds(
    bounds: PluginWebviewBounds,
    native_viewport_width: f64,
    native_viewport_height: f64,
) -> PluginWebviewBounds {
    let scale_x = native_viewport_width / bounds.viewport_width;
    let scale_y = native_viewport_height / bounds.viewport_height;
    PluginWebviewBounds {
        x: bounds.x * scale_x,
        y: bounds.y * scale_y,
        width: bounds.width * scale_x,
        height: bounds.height * scale_y,
        viewport_width: native_viewport_width,
        viewport_height: native_viewport_height,
    }
}

fn native_plugin_bounds(
    app: &tauri::AppHandle,
    bounds: PluginWebviewBounds,
) -> Result<PluginWebviewBounds, String> {
    bounds.validate()?;
    let main = app
        .get_window("main")
        .ok_or_else(|| "main window is not available".to_string())?;
    let scale_factor = main.scale_factor().map_err(|error| error.to_string())?;
    let native_size = main
        .inner_size()
        .map_err(|error| error.to_string())?
        .to_logical::<f64>(scale_factor);
    // `add_child` attaches the plugin webview to the main window's content
    // view. The browser rectangle is also measured from that content view,
    // so adding the title-bar/frame inset here would shift the child down a
    // second time. Keep the coordinates in content-view space and only
    // normalize for a scale-factor/viewport change.
    let native_bounds = scale_plugin_bounds(bounds, native_size.width, native_size.height);
    native_bounds.validate()?;
    Ok(native_bounds)
}

fn plugin_webview_url(
    policy: &daena_plugin_host::PluginWebviewPolicy,
    project_id: &str,
    view_id: Option<&str>,
    host_surface: Option<(&str, u32)>,
    map_entity_id: Option<&str>,
    link_id: Option<&str>,
    bounds: PluginWebviewBounds,
) -> Result<tauri::WebviewUrl, String> {
    let mut url = format!(
        "{}?project={}&width={}&height={}",
        policy.url,
        percent_encode(project_id),
        bounds.width,
        bounds.height
    );
    append_host_surface_query(&mut url, host_surface);
    if let Some(view_id) = view_id {
        url.push_str("&view=");
        url.push_str(&percent_encode(view_id));
    }
    if let Some(map_entity_id) = map_entity_id {
        url.push_str("&mapEntityId=");
        url.push_str(&percent_encode(map_entity_id));
    }
    if let Some(link_id) = link_id {
        url.push_str("&linkId=");
        url.push_str(&percent_encode(link_id));
    }
    Ok(tauri::WebviewUrl::External(
        url.parse()
            .map_err(|error| format!("invalid plugin URL: {error}"))?,
    ))
}

#[tauri::command]
#[allow(clippy::too_many_arguments)]
async fn plugin_mount_webview(
    app: tauri::AppHandle,
    state: tauri::State<'_, SharedCore>,
    plugins: tauri::State<'_, SharedPluginHost>,
    plugin_id: String,
    view_id: Option<String>,
    map_entity_id: Option<String>,
    link_id: Option<String>,
    bounds: PluginWebviewBounds,
) -> Result<(), String> {
    let bounds = native_plugin_bounds(&app, bounds)?;
    let project_id = current_info(state.inner())?
        .map(|info| info.root)
        .ok_or_else(|| "project is not open".to_string())?;
    let (policy, url) = {
        let host = plugins
            .lock()
            .map_err(|_| "plugin host lock poisoned".to_string())?;
        let entry = host
            .runtime_entry(&project_id, &plugin_id)
            .ok_or_else(|| "plugin is not installed".to_string())?;
        if entry.manifest.kind != daena_plugin_api::PluginKind::Sandboxed {
            return Err("only sandboxed plugins have UI webviews".into());
        }
        if host.lifecycle.state(&project_id, &plugin_id).state
            != daena_plugin_api::LifecycleState::Active
        {
            return Err("plugin is not active".into());
        }
        let policy = webview_policy(&entry.manifest)
            .ok_or_else(|| "plugin has no UI entrypoint".to_string())?;
        validate_plugin_view(&entry.manifest, view_id.as_deref())?;
        let host_surface = host_surface_for_view(&entry.manifest, view_id.as_deref());
        let url = plugin_webview_url(
            &policy,
            &project_id,
            view_id.as_deref(),
            host_surface,
            map_entity_id.as_deref(),
            link_id.as_deref(),
            bounds,
        )?;
        (policy, url)
    };
    let label = policy.label.clone();

    // Only one embedded plugin view occupies the workspace. Do not close a
    // legacy external plugin window with the same identity.
    for (other_label, webview) in app.webviews() {
        if other_label.starts_with("plugin:")
            && other_label != label
            && app.get_webview_window(&other_label).is_none()
        {
            if let Ok(mut states) = embedded_webview_states().lock() {
                states.remove(&other_label);
            }
            let _ = webview.close();
        }
    }

    // Always recreate the embedded webview on mount. Reusing an existing
    // child and only resizing it drops query params such as mapEntityId, so
    // opening a saved map (or switching maps) would keep the previous source
    // loaded and never hit asset.read / uploadMap for the requested entity.
    if let Some(window) = app.get_webview_window(&label) {
        window.close().map_err(|error| error.to_string())?;
    }
    if let Some(webview) = app.get_webview(&label) {
        if let Ok(mut states) = embedded_webview_states().lock() {
            states.remove(&label);
        }
        webview.close().map_err(|error| error.to_string())?;
    }

    let navigation_policy = policy.clone();
    let main = app
        .get_window("main")
        .ok_or_else(|| "main window is not available".to_string())?;
    let builder = tauri::WebviewBuilder::new(label.clone(), url)
        .use_https_scheme(true)
        .initialization_script(PLUGIN_WEBVIEW_ISOLATION_SCRIPT)
        .on_navigation(move |url| plugin_navigation_allowed(&url, &navigation_policy))
        .on_page_load(move |webview, payload| {
            if matches!(payload.event(), tauri::webview::PageLoadEvent::Finished) {
                let bounds = embedded_webview_states()
                    .lock()
                    .ok()
                    .and_then(|mut states| {
                        let state = states.get_mut(webview.label())?;
                        state.ready = true;
                        Some(state.bounds)
                    });
                if let Some(bounds) = bounds {
                    let _ = webview.set_position(tauri::LogicalPosition::new(bounds.x, bounds.y));
                    let _ = webview.set_size(tauri::LogicalSize::new(bounds.width, bounds.height));
                    let _ = webview.show();
                }
            }
        });
    // Keep the child effectively invisible until its first document has
    // painted. Showing a newly-created native webview at its final bounds can
    // expose its platform background for a frame, which appears as a flash.
    {
        let mut states = embedded_webview_states()
            .lock()
            .map_err(|_| "embedded webview state lock poisoned".to_string())?;
        states.insert(
            label.clone(),
            EmbeddedPluginWebviewState {
                bounds,
                ready: false,
            },
        );
    }
    if let Err(error) = main.add_child(
        builder,
        tauri::LogicalPosition::new(0.0, 0.0),
        tauri::LogicalSize::new(1.0, 1.0),
    ) {
        if let Ok(mut states) = embedded_webview_states().lock() {
            states.remove(&label);
        }
        return Err(error.to_string());
    }
    Ok(())
}

#[tauri::command]
async fn plugin_resize_webview(
    app: tauri::AppHandle,
    bounds: PluginWebviewBounds,
    plugin_id: String,
) -> Result<(), String> {
    let bounds = native_plugin_bounds(&app, bounds)?;
    let label = plugin_window_label(&plugin_id);
    let webview = app
        .get_webview(&label)
        .ok_or_else(|| "embedded plugin webview is not mounted".to_string())?;
    let ready = embedded_webview_states()
        .lock()
        .map(|mut states| {
            let state = states.entry(label).or_insert(EmbeddedPluginWebviewState {
                bounds,
                ready: true,
            });
            state.bounds = bounds;
            state.ready
        })
        .map_err(|_| "embedded webview state lock poisoned".to_string())?;
    if !ready {
        return Ok(());
    }
    webview
        .set_position(tauri::LogicalPosition::new(bounds.x, bounds.y))
        .map_err(|error| error.to_string())?;
    webview
        .set_size(tauri::LogicalSize::new(bounds.width, bounds.height))
        .map_err(|error| error.to_string())?;
    Ok(())
}

#[tauri::command]
fn plugin_unmount_webview(app: tauri::AppHandle, plugin_id: String) -> Result<(), String> {
    let label = plugin_window_label(&plugin_id);
    if let Ok(mut states) = embedded_webview_states().lock() {
        states.remove(&label);
    }
    if let Some(webview) = app.get_webview(&label) {
        if app.get_webview_window(&label).is_none() {
            webview.close().map_err(|error| error.to_string())?;
        }
    }
    Ok(())
}

fn validate_plugin_view(manifest: &PluginManifest, view_id: Option<&str>) -> Result<(), String> {
    let Some(view_id) = view_id else {
        return Ok(());
    };
    if manifest.views.iter().any(|view| view.id == view_id) {
        Ok(())
    } else {
        Err(format!("plugin view is not declared: {view_id}"))
    }
}

#[tauri::command]
fn plugin_open_webview(
    app: tauri::AppHandle,
    state: tauri::State<'_, SharedCore>,
    plugins: tauri::State<'_, SharedPluginHost>,
    plugin_id: String,
    view_id: Option<String>,
) -> Result<(), String> {
    let project_id = current_info(state.inner())?
        .map(|info| info.root)
        .ok_or_else(|| "project is not open".to_string())?;
    let host = plugins
        .lock()
        .map_err(|_| "plugin host lock poisoned".to_string())?;
    open_plugin_webview(&app, &plugin_id, &project_id, &host, view_id.as_deref())
}

#[tauri::command]
async fn plugin_host_view_data(
    state: tauri::State<'_, SharedCore>,
    plugins: tauri::State<'_, SharedPluginHost>,
    plugin_id: String,
    view_id: String,
    selected_entity_id: Option<String>,
) -> Result<serde_json::Value, String> {
    let project_id = current_info(state.inner())?
        .map(|info| info.root)
        .ok_or_else(|| "project is not open".to_string())?;
    let view = plugins
        .lock()
        .map_err(|_| "plugin host lock poisoned".to_string())?
        .host_view(&project_id, &plugin_id, &view_id)
        .map_err(|error| error.to_string())?;

    with_core(state, move |core| {
        let project = core.project(trusted_shell())?;
        let all_entities = project.list_entities()?;
        let mut lists = serde_json::Map::new();
        let mut list_entity_types = BTreeMap::new();
        for component in &view.components {
            if let ViewComponent::EntityList {
                id,
                entity_type,
                limit,
                ..
            } = component
            {
                let entities = all_entities
                    .iter()
                    .filter(|entity| {
                        !entity.deleted
                            && entity.entity_type.as_deref() == Some(entity_type.as_str())
                    })
                    .take(*limit as usize)
                    .cloned()
                    .collect::<Vec<_>>();
                lists.insert(
                    id.clone(),
                    serde_json::to_value(entities)
                        .map_err(|error| CoreError::Validation(error.to_string()))?,
                );
                list_entity_types.insert(id.clone(), entity_type.clone());
            }
        }

        let selected = selected_entity_id.as_deref().and_then(|id| {
            all_entities.iter().find(|entity| {
                !entity.deleted
                    && entity.id == id
                    && entity.entity_type.as_ref().is_some_and(|entity_type| {
                        list_entity_types
                            .values()
                            .any(|listed| listed == entity_type)
                    })
            })
        });
        let selected_id = selected.map(|entity| entity.id.as_str());
        let mut fields = serde_json::Map::new();
        if let Some(selected_id) = selected_id {
            for component in &view.components {
                if let ViewComponent::FieldForm {
                    source,
                    namespace,
                    fields: requested_fields,
                    ..
                } = component
                {
                    let Some(source_type) = list_entity_types.get(source) else {
                        continue;
                    };
                    let Some(selected_entity) = selected else {
                        continue;
                    };
                    if selected_entity.entity_type.as_deref() != Some(source_type.as_str()) {
                        continue;
                    }
                    for field in project.list_fields(selected_id.to_string())? {
                        if field.namespace == *namespace && requested_fields.contains(&field.key) {
                            fields.insert(field.key, field.value);
                        }
                    }
                }
            }
        }

        Ok(serde_json::json!({
            "lists": lists,
            "selected": selected,
            "fields": fields,
        }))
    })
    .await
}

#[tauri::command]
#[allow(clippy::too_many_arguments)]
async fn plugin_host_view_set_field(
    state: tauri::State<'_, SharedCore>,
    plugins: tauri::State<'_, SharedPluginHost>,
    plugin_id: String,
    view_id: String,
    component_id: String,
    entity_id: String,
    key: String,
    value: serde_json::Value,
) -> Result<(), String> {
    let project_id = current_info(state.inner())?
        .map(|info| info.root)
        .ok_or_else(|| "project is not open".to_string())?;
    let view = plugins
        .lock()
        .map_err(|_| "plugin host lock poisoned".to_string())?
        .host_view(&project_id, &plugin_id, &view_id)
        .map_err(|error| error.to_string())?;
    let Some(ViewComponent::FieldForm {
        source,
        namespace,
        fields,
        editable: true,
        ..
    }) = view
        .components
        .iter()
        .find(|component| component_id == component_id_of(component))
    else {
        return Err("host field form is not declared or is read-only".into());
    };
    if !fields.iter().any(|field| field == &key) {
        return Err("field is not declared by the host field form".into());
    }
    let source_type = view
        .components
        .iter()
        .find_map(|component| match component {
            ViewComponent::EntityList {
                id, entity_type, ..
            } if id == source => Some(entity_type.as_str()),
            _ => None,
        })
        .ok_or_else(|| "host field form source is invalid".to_string())?
        .to_owned();
    let namespace = namespace.clone();
    with_core(state, move |core| {
        let project = core.project(trusted_shell())?;
        let entity = project
            .list_entities()?
            .into_iter()
            .find(|entity| entity.id == entity_id && !entity.deleted);
        if entity
            .as_ref()
            .and_then(|entity| entity.entity_type.as_deref())
            != Some(source_type.as_str())
        {
            return Err(CoreError::NotFound(
                "entity is outside the host view source".into(),
            ));
        }
        project.set_field(FieldValue {
            entity_id,
            namespace,
            key,
            value,
            revision: String::new(),
        })
    })
    .await
}

#[tauri::command]
fn plugin_host_invoke_command(
    state: tauri::State<'_, SharedCore>,
    plugins: tauri::State<'_, SharedPluginHost>,
    plugin_id: String,
    view_id: Option<String>,
    command_id: String,
    payload: Option<serde_json::Value>,
) -> Result<String, String> {
    let project_id = current_info(state.inner())?
        .map(|info| info.root)
        .ok_or_else(|| "project is not open".to_string())?;
    let payload = payload.unwrap_or_else(|| serde_json::json!({}));
    let host = plugins
        .lock()
        .map_err(|_| "plugin host lock poisoned".to_string())?;
    let action = match view_id.as_deref() {
        Some(view_id) => {
            host.invoke_command_with_payload(&project_id, &plugin_id, view_id, &command_id, payload)
        }
        None => host.invoke_broker_command(&project_id, &plugin_id, &command_id, payload),
    }
    .map_err(|error| error.to_string())?;
    Ok(match action {
        CommandAction::RefreshView => "refresh-view".into(),
    })
}

fn component_id_of(component: &ViewComponent) -> &str {
    match component {
        ViewComponent::Heading { id, .. }
        | ViewComponent::Text { id, .. }
        | ViewComponent::EntityList { id, .. }
        | ViewComponent::EntityDetail { id, .. }
        | ViewComponent::FieldForm { id, .. }
        | ViewComponent::Button { id, .. } => id,
    }
}

#[tauri::command]
fn plugin_close_webview(app: tauri::AppHandle, plugin_id: String) -> Result<(), String> {
    close_plugin_webview(&app, &plugin_id);
    Ok(())
}

#[tauri::command]
fn plugin_close_all_webviews(app: tauri::AppHandle) -> Result<(), String> {
    let labels: Vec<String> = app
        .webviews()
        .into_keys()
        .filter_map(|label| label.starts_with("plugin:").then_some(label))
        .collect();
    for label in labels {
        if let Ok(mut states) = embedded_webview_states().lock() {
            states.remove(&label);
        }
        if let Some(webview) = app.get_webview(&label) {
            let _ = webview.close();
        }
        if let Some(window) = app.get_webview_window(&label) {
            let _ = window.close();
        }
    }
    Ok(())
}

fn trusted_shell() -> AuthorityContext {
    AuthorityContext::trusted_shell()
}

fn flush_checkpoint_for_shared_core(core: &SharedCore, reason: &'static str) -> Result<(), String> {
    let session = current_session(core)?;
    let handle = {
        let service = session
            .core
            .lock()
            .map_err(|_| "core lock poisoned".to_string())?;
        if service.info().is_none() {
            return Ok(());
        }
        service
            .project(trusted_shell())
            .map_err(|error| error.to_string())?
            .checkpoint_handle()
            .map_err(|error| error.to_string())?
    };
    handle
        .flush_checkpoint(reason)
        .map(|_| ())
        .map_err(|error| error.to_string())
}

async fn with_core<T, F>(state: tauri::State<'_, SharedCore>, operation: F) -> Result<T, String>
where
    T: Send + 'static,
    F: FnOnce(&mut CoreService) -> Result<T, CoreError> + Send + 'static,
{
    let lifecycle = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let session = lifecycle
            .lock()
            .map_err(|_| CoreError::Conflict("project lifecycle lock poisoned".into()))?
            .clone();
        let mut core = session
            .core
            .lock()
            .map_err(|_| CoreError::Conflict("core lock poisoned".into()))?;
        operation(&mut core)
    })
    .await
    .map_err(|error| format!("core worker failed: {error}"))?
    .map_err(|error| error.to_string())
}

async fn flush_project_checkpoint(
    state: tauri::State<'_, SharedCore>,
    reason: &'static str,
) -> Result<(), String> {
    flush_project_checkpoint_for_shared_core(state.inner().clone(), reason).await
}

async fn flush_project_checkpoint_for_shared_core(
    core: SharedCore,
    reason: &'static str,
) -> Result<(), String> {
    let handle = tauri::async_runtime::spawn_blocking(
        move || -> Result<Option<CheckpointHandle>, String> {
            let session = current_session(&core)?;
            let service = session
                .core
                .lock()
                .map_err(|_| "core lock poisoned".to_string())?;
            if service.info().is_none() {
                return Ok(None);
            }
            Ok(Some(
                service
                    .project(trusted_shell())
                    .map_err(|error| error.to_string())?
                    .checkpoint_handle()
                    .map_err(|error| error.to_string())?,
            ))
        },
    )
    .await
    .map_err(|error| format!("checkpoint handle worker failed: {error}"))??;
    let Some(handle): Option<CheckpointHandle> = handle else {
        return Ok(());
    };
    tauri::async_runtime::spawn_blocking(move || handle.flush_checkpoint(reason))
        .await
        .map_err(|error| format!("checkpoint worker failed: {error}"))?
        .map(|_| ())
        .map_err(|error| error.to_string())
}

async fn with_read_project<T, F>(
    state: tauri::State<'_, SharedCore>,
    operation: F,
) -> Result<T, String>
where
    T: Send + 'static,
    F: FnOnce(&ProjectStore) -> Result<T, CoreError> + Send + 'static,
{
    let lifecycle = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let session = lifecycle
            .lock()
            .map_err(|_| CoreError::Conflict("project lifecycle lock poisoned".into()))?
            .clone();
        let (root, database_epoch) = {
            let core = session
                .core
                .lock()
                .map_err(|_| CoreError::Conflict("core lock poisoned".into()))?;
            let project = core.project(trusted_shell())?;
            (
                project.info().ok_or(CoreError::ProjectNotOpen)?.root,
                project.database_epoch().to_string(),
            )
        };
        let project = {
            let mut pool = session
                .read_pool
                .lock()
                .map_err(|_| CoreError::Conflict("read connection pool poisoned".into()))?;
            pool.retain(|project| project.database_epoch() == database_epoch);
            pool.pop()
        }
        .map(Ok)
        .unwrap_or_else(|| ProjectStore::open_read_only(&root))?;
        let result = operation(&project);
        let mut pool = session
            .read_pool
            .lock()
            .map_err(|_| CoreError::Conflict("read connection pool poisoned".into()))?;
        if pool.len() < READ_CONNECTION_POOL_CAPACITY {
            pool.push(project);
        }
        result
    })
    .await
    .map_err(|error| format!("read worker failed: {error}"))?
    .map_err(|error| error.to_string())
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct PluginBootstrap {
    rpc_version: u32,
    session_id: String,
    plugin_id: String,
    project_id: String,
    version: String,
    host_api: String,
    granted_capabilities: Vec<String>,
    optional_features: Vec<String>,
    package_digest: String,
    manifest: PluginManifest,
}

/// Bootstrap and RPC use a host-created `plugin:<id>` webview label as the
/// origin binding. The trusted main window cannot call this surface, and the
/// plugin cannot submit an arbitrary origin in its request payload.
#[tauri::command]
fn plugin_bootstrap(
    window: tauri::WebviewWindow,
    core: tauri::State<'_, SharedCore>,
    state: tauri::State<'_, SharedPluginHost>,
    plugin_id: String,
    project_id: String,
) -> Result<PluginBootstrap, String> {
    let origin = plugin_webview_identity(&window, Some(&plugin_id))?;
    let current_project = current_info(core.inner())?
        .map(|info| info.root)
        .ok_or_else(|| "project is not open".to_string())?;
    if current_project != project_id {
        return Err("plugin bootstrap project mismatch".into());
    }
    let mut host = state
        .lock()
        .map_err(|_| "plugin host lock poisoned".to_string())?;
    let session = host
        .bootstrap(&plugin_id, &project_id, &origin)
        .map_err(|error| error.to_string())?;
    let entry = host
        .runtime_entry(&project_id, &plugin_id)
        .ok_or_else(|| "plugin was removed during bootstrap".to_string())?;
    Ok(PluginBootstrap {
        rpc_version: daena_plugin_api::RPC_VERSION,
        session_id: session.id.clone(),
        plugin_id,
        project_id,
        version: entry.manifest.version.clone(),
        host_api: entry.manifest.host_api.clone(),
        granted_capabilities: session.grants.iter().cloned().collect(),
        optional_features: Vec::new(),
        package_digest: entry.digest,
        manifest: entry.manifest,
    })
}

#[tauri::command]
async fn plugin_rpc(
    window: tauri::WebviewWindow,
    core: tauri::State<'_, SharedCore>,
    state: tauri::State<'_, SharedPluginHost>,
    settings: tauri::State<'_, SharedSettings>,
    ai_runtime: tauri::State<'_, ai::SharedAiRuntime>,
    request: RpcRequest,
) -> Result<RpcResponse, String> {
    let mut request = request;
    if matches!(
        request.method.as_str(),
        "relationship.delete" | "relationship.update"
    ) {
        let id = payload_string(&request.payload, "id").map_err(|error| error.to_string())?;
        let stored = with_core(core.clone(), move |core| {
            core.project(trusted_shell())?.relationship(id)
        })
        .await?;
        let object = request
            .payload
            .as_object_mut()
            .ok_or_else(|| "relationship delete payload must be an object".to_string())?;
        object.insert(
            "__stored_relationship_type".into(),
            serde_json::Value::String(stored.relationship_type),
        );
    }
    let origin = plugin_webview_identity(&window, None)?;
    let session = {
        let host = state
            .lock()
            .map_err(|_| "plugin host lock poisoned".to_string())?;
        host.authorize_rpc(&origin, &request).map_err(|error| {
            serde_json::to_string(&error).unwrap_or_else(|_| error.message.clone())
        })?
    };
    let request_id = request.request_id.clone();
    let plugin_id = session.plugin_id.clone();
    let method = request.method;
    let event_method = method.clone();
    let mut payload = request.payload;
    if matches!(
        method.as_str(),
        "relationship.delete" | "relationship.update"
    ) {
        payload
            .as_object_mut()
            .ok_or_else(|| "relationship mutation payload must be an object".to_string())?
            .remove("__stored_relationship_type");
    }
    if method == "field.list" {
        let namespace = payload
            .get("namespace")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| "field list payload requires namespace".to_string())?;
        let shared_only = payload
            .get("__shared_only")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false);
        let shared_keys = {
            let host = state
                .lock()
                .map_err(|_| "plugin host lock poisoned".to_string())?;
            if !shared_only && host.namespaces.owner(namespace) == Some(plugin_id.as_str()) {
                None
            } else {
                Some(host.namespaces.shared_field_keys(namespace))
            }
        };
        if let Some(keys) = shared_keys {
            payload
                .as_object_mut()
                .ok_or_else(|| "field list payload must be an object".to_string())?
                .insert(
                    "__shared_keys".into(),
                    serde_json::to_value(keys).map_err(|error| error.to_string())?,
                );
        }
    }
    let current_project = current_info(&core)?.map(|info| info.root);
    let event_project_id = session.project_id.clone();
    let result = if current_project.as_deref() != Some(session.project_id.as_str()) {
        Err("plugin session is not bound to the open project".to_string())
    } else if matches!(
        method.as_str(),
        "event.subscribe"
            | "event.poll"
            | "event.publish"
            | "service.call"
            | "ai.request.start"
            | "ai.request.poll"
            | "ai.request.cancel"
            | "ai.request.result"
            | "ai.request.citations"
    ) {
        dispatch_host_rpc(
            &state,
            &session.plugin_id,
            &session.project_id,
            &method,
            payload,
            AiBrokerContext {
                app: Some(window.app_handle().clone()),
                core: Some(core.inner().clone()),
                settings: Some(settings.inner().clone()),
                ai_runtime: ai_runtime.inner().clone(),
                session_id: session.id.clone(),
                caller: daena_ai::AiCaller::authorized_plugin(
                    session.plugin_id.clone(),
                    session.project_id.clone(),
                    session.grants.iter().cloned().collect(),
                    vec![format!("project:{}", session.project_id)],
                    session.generation,
                    "pending",
                ),
            },
        )
    } else {
        let project_id = session.project_id;
        let request_id_for_dispatch = sanitize_mutation_request_id(&request_id).map(str::to_owned);
        let record_owner_entity_types = method
            .starts_with("record.")
            .then(|| {
                let collection = payload
                    .get("collection")
                    .and_then(serde_json::Value::as_str)?;
                state.lock().ok()?.record_owner_entity_types(
                    &project_id,
                    &session.plugin_id,
                    collection,
                )
            })
            .flatten();
        with_core(core, move |core| {
            let current_project = core
                .info()
                .map(|info| info.root)
                .ok_or(CoreError::ProjectNotOpen)?;
            if current_project != project_id {
                return Err(CoreError::Unauthorized {
                    operation: "access another project",
                });
            }
            dispatch_module_rpc(
                core,
                Some(&session.plugin_id),
                record_owner_entity_types,
                &method,
                payload,
                request_id_for_dispatch.as_deref(),
            )
        })
        .await
    };
    let result = result.and_then(|result| {
        publish_core_mutation_event(state.inner(), &event_project_id, &event_method, &result)?;
        Ok(result)
    });
    match result {
        Ok(result) => Ok(RpcResponse {
            rpc_version: daena_plugin_api::RPC_VERSION,
            request_id,
            ok: true,
            result: Some(result),
            error: None,
        }),
        Err(error) => Ok(RpcResponse {
            rpc_version: daena_plugin_api::RPC_VERSION,
            request_id,
            ok: false,
            result: None,
            error: Some(daena_plugin_api::RpcError {
                code: "core.error".into(),
                message: format!("plugin {plugin_id}: {error}"),
                retryable: false,
                details: None,
            }),
        }),
    }
}

struct AiBrokerContext {
    app: Option<tauri::AppHandle>,
    core: Option<SharedCore>,
    settings: Option<SharedSettings>,
    ai_runtime: ai::SharedAiRuntime,
    session_id: String,
    caller: daena_ai::AiCaller,
}

fn dispatch_host_rpc(
    plugins: &SharedPluginHost,
    plugin_id: &str,
    project_id: &str,
    method: &str,
    payload: serde_json::Value,
    context: AiBrokerContext,
) -> Result<serde_json::Value, String> {
    let mut host = plugins
        .lock()
        .map_err(|_| "plugin host lock poisoned".to_string())?;
    if method == "service.call" {
        let name = payload
            .get("name")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| "service RPC payload requires name".to_string())?;
        let major = payload
            .get("major")
            .and_then(serde_json::Value::as_u64)
            .and_then(|major| u32::try_from(major).ok())
            .ok_or_else(|| "service RPC payload requires a valid major".to_string())?;
        let deadline_ms = payload
            .get("deadlineMs")
            .and_then(serde_json::Value::as_u64)
            .unwrap_or(5_000)
            .clamp(1, 30_000);
        return host
            .call_service_authorized(
                plugin_id,
                project_id,
                name,
                major,
                payload
                    .get("payload")
                    .cloned()
                    .unwrap_or(serde_json::Value::Null),
                std::time::Duration::from_millis(deadline_ms),
            )
            .map_err(|error| error.to_string());
    }

    if method.starts_with("ai.request.") {
        drop(host);
        let request_id_value = payload.get("requestId").and_then(serde_json::Value::as_str);
        if method == "ai.request.start" {
            let operation = payload
                .get("operation")
                .and_then(serde_json::Value::as_str)
                .ok_or_else(|| "AI operation is required".to_string())?;
            let instruction = payload
                .get("userInstruction")
                .and_then(serde_json::Value::as_str)
                .ok_or_else(|| "AI instruction is required".to_string())?;
            let immediate_context = payload
                .get("immediateContext")
                .cloned()
                .unwrap_or(serde_json::Value::Null);
            let deadline_ms = payload
                .get("deadlineMs")
                .and_then(serde_json::Value::as_u64)
                .unwrap_or(daena_ai::DEFAULT_LIMITS.default_deadline.as_millis() as u64)
                .clamp(
                    1,
                    daena_ai::DEFAULT_LIMITS.default_deadline.as_millis() as u64,
                );
            let selection = immediate_context
                .get("selection")
                .and_then(serde_json::Value::as_str)
                .unwrap_or_default()
                .to_string();
            let retrieval_policy = payload
                .get("retrievalPolicy")
                .cloned()
                .map(serde_json::from_value::<daena_plugin_api::AiRetrievalPolicyPayload>)
                .transpose()
                .map_err(|error| format!("invalid retrieval policy: {error}"))?;
            let (retrieved_context, citations) = if let Some(policy) = retrieval_policy {
                let core = context
                    .core
                    .as_ref()
                    .ok_or_else(|| "AI retrieval requires an open project".to_string())?;
                let session = current_session(core)?;
                let core = session
                    .core
                    .lock()
                    .map_err(|_| "core lock poisoned".to_string())?;
                let project = core
                    .project(trusted_shell())
                    .map_err(|error| error.to_string())?;
                ai::build_retrieval_context(project, &context.caller, &policy)?
            } else {
                (String::new(), Vec::new())
            };
            let selection = if retrieved_context.is_empty() {
                selection
            } else {
                format!(
                    "{selection}\n\n[RETRIEVED_CONTEXT]\n{retrieved_context}\n[/RETRIEVED_CONTEXT]"
                )
            };
            let settings = context
                .settings
                .map(|settings| {
                    settings
                        .lock()
                        .map_err(|_| "settings lock poisoned".to_string())
                        .and_then(|store| store.load())
                })
                .transpose()?
                .unwrap_or_default();
            let provider =
                ai::resolve_ai_provider(&settings, Some(&context.caller.project_id), true)?;
            let started = match operation {
                "generate_text" => ai::start_ai_request_mode(
                    context.app.clone(),
                    context.ai_runtime.clone(),
                    context.caller.clone(),
                    provider.endpoint.clone(),
                    provider.model.clone(),
                    instruction.to_string(),
                    selection,
                    None,
                    std::time::Duration::from_millis(deadline_ms),
                    citations.clone(),
                    provider.remote,
                    provider.api_key.clone(),
                )?,
                "generate_structured" => {
                    let contract = payload.get("outputContract").cloned().ok_or_else(|| {
                        "structured AI requests require outputContract".to_string()
                    })?;
                    ai::validate_structured_schema(&contract)?;
                    ai::start_ai_request_mode(
                        context.app.clone(),
                        context.ai_runtime.clone(),
                        context.caller.clone(),
                        provider.endpoint,
                        provider.model,
                        instruction.to_string(),
                        if retrieved_context.is_empty() {
                            immediate_context.to_string()
                        } else {
                            selection.clone()
                        },
                        Some(contract),
                        std::time::Duration::from_millis(deadline_ms),
                        citations,
                        provider.remote,
                        provider.api_key,
                    )?
                }
                _ => return Err("unsupported AI operation".into()),
            };
            let mut host = plugins
                .lock()
                .map_err(|_| "plugin host lock poisoned".to_string())?;
            host.register_ai_request(
                &started,
                project_id,
                plugin_id,
                &context.session_id,
                operation,
                payload.get("outputContract").cloned(),
            );
            return Ok(serde_json::json!({ "requestId": started }));
        }
        let target = request_id_value.ok_or_else(|| "AI requestId is required".to_string())?;
        let mut host = plugins
            .lock()
            .map_err(|_| "plugin host lock poisoned".to_string())?;
        let operation = host
            .authorize_ai_request(target, project_id, plugin_id, &context.session_id)
            .map_err(|error| error.to_string())?;
        let contract = host.ai_contract(target);
        match method {
            "ai.request.poll" => {
                return serde_json::to_value(
                    ai::poll_ai_events(&context.ai_runtime, target)
                        .map_err(|error| error.to_string())?,
                )
                .map_err(|error| error.to_string())
            }
            "ai.request.cancel" => {
                ai::cancel_ai_request(&context.ai_runtime, target)?;
                return Ok(serde_json::Value::Null);
            }
            "ai.request.citations" => {
                let citations = ai::ai_request_citations(&context.ai_runtime, target)?;
                host.remove_ai_request(target);
                ai::remove_ai_citations(&context.ai_runtime, target)?;
                return serde_json::to_value(citations).map_err(|error| error.to_string());
            }
            "ai.request.result" => {
                let output = ai::ai_request_result(&context.ai_runtime, target)?;
                let has_citations =
                    !ai::ai_request_citations(&context.ai_runtime, target)?.is_empty();
                if operation == "generate_structured" {
                    let value: serde_json::Value = serde_json::from_str(&output)
                        .map_err(|_| "structured AI output is not valid JSON".to_string())?;
                    if let Some(contract) = contract.as_ref() {
                        ai::validate_structured_output(contract, &value)?;
                    }
                    if !has_citations {
                        host.remove_ai_request(target);
                        ai::remove_ai_citations(&context.ai_runtime, target)?;
                    }
                    return Ok(value);
                }
                if !has_citations {
                    host.remove_ai_request(target);
                    ai::remove_ai_citations(&context.ai_runtime, target)?;
                }
                return Ok(serde_json::json!({ "output": output }));
            }
            _ => return Err("unknown AI request method".into()),
        }
    }

    let event_type = payload
        .get("type")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| "event RPC payload requires type".to_string())?;
    let (name, version) = event_type
        .rsplit_once('@')
        .ok_or_else(|| "event type must include @version".to_string())?;
    let version = version
        .parse::<u32>()
        .map_err(|_| "event version is invalid".to_string())?;
    match method {
        "event.subscribe" => {
            host.subscribe_event_authorized(plugin_id, project_id, name, version)
                .map_err(|error| error.to_string())?;
            Ok(serde_json::Value::Null)
        }
        "event.poll" => serde_json::to_value(
            host.poll_events_authorized(plugin_id, project_id, name, version)
                .map_err(|error| error.to_string())?,
        )
        .map_err(|error| error.to_string()),
        "event.publish" => serde_json::to_value(
            host.publish_event_authorized(
                plugin_id,
                project_id,
                name,
                version,
                payload
                    .get("payload")
                    .cloned()
                    .unwrap_or(serde_json::Value::Null),
            )
            .map_err(|error| error.to_string())?,
        )
        .map_err(|error| error.to_string()),
        _ => Err(format!("unknown plugin host RPC method: {method}")),
    }
}

fn plugin_webview_identity(
    window: &tauri::WebviewWindow,
    expected: Option<&str>,
) -> Result<String, String> {
    let label = window.label();
    let _plugin_id = label
        .strip_prefix("plugin:")
        .filter(|value| !value.is_empty())
        .ok_or_else(|| "plugin RPC requires a plugin webview".to_string())?;
    if expected.is_some_and(|expected| plugin_window_label(expected) != label) {
        return Err("plugin webview identity mismatch".into());
    }
    Ok(label.to_string())
}

fn sync_project_usage(project: &ProjectStore, host: &mut PluginHost) -> Result<(), CoreError> {
    let project_id = project
        .info()
        .map(|info| info.root)
        .ok_or(CoreError::ProjectNotOpen)?;
    host.bind_project_grants(Path::new(&project_id), &project_id)
        .map_err(|error| CoreError::Conflict(error.to_string()))?;
    let states = project
        .module_states()?
        .into_iter()
        .map(|module| (module.module_id, (module.enabled, module.package_version)))
        .collect::<BTreeMap<_, _>>();
    let mut module_ids = states.keys().cloned().collect::<BTreeSet<_>>();
    module_ids.extend(host.catalog.list().map(|entry| entry.manifest.id.clone()));

    for module_id in module_ids {
        let (enabled, package_version) = if let Some(state) = states.get(&module_id).cloned() {
            state
        } else {
            let enabled = host
                .catalog
                .get(&module_id)
                .and_then(|entry| entry.manifest.enabled_by_default)
                .unwrap_or(true);
            if !enabled {
                project.set_module_enabled(module_id.clone(), false)?;
            }
            (enabled, None)
        };
        if enabled {
            if let Some(version) = package_version {
                host.record_project_usage(&project_id, &module_id, &version)
                    .map_err(|error| CoreError::Conflict(error.to_string()))?;
            }
            host.ensure_first_party_bundled_grants(&project_id, &module_id)
                .map_err(|error| CoreError::Validation(error.to_string()))?;
            host.activate_bundled(&project_id, &module_id)
                .map_err(|error| CoreError::Validation(error.to_string()))?;
        } else {
            host.deactivate_bundled(&project_id, &module_id);
            host.clear_project_usage(&project_id, &module_id)
                .map_err(|error| CoreError::Conflict(error.to_string()))?;
        }
    }
    Ok(())
}

fn relationship_metadata_schemas_for_project(
    project: &ProjectStore,
    host: &PluginHost,
) -> Result<BTreeMap<String, Vec<MetadataFieldDefinition>>, CoreError> {
    let project_id = project.info().map(|info| info.root);
    let mut entries = host
        .catalog
        .list()
        .map(|entry| entry.manifest.id.clone())
        .collect::<Vec<_>>();
    entries.sort();
    let mut merged: BTreeMap<String, BTreeMap<String, MetadataFieldDefinition>> = BTreeMap::new();
    for module_id in entries {
        let enabled = if project_id.is_some() {
            project.is_module_enabled(&module_id)?
        } else {
            host.catalog
                .get(&module_id)
                .and_then(|entry| entry.manifest.enabled_by_default)
                .unwrap_or(true)
        };
        if !enabled {
            continue;
        }
        let entry = host
            .runtime_entry(project_id.as_deref().unwrap_or_default(), &module_id)
            .or_else(|| host.catalog.get(&module_id).cloned())
            .ok_or_else(|| CoreError::Validation("plugin catalog entry disappeared".into()))?;
        let mut manifest = entry.manifest.clone();
        if supports_schema_overlay(&manifest) {
            let overlay_value = if project_id.is_some() {
                project
                    .module_schema_overlay(&module_id)?
                    .unwrap_or_else(|| serde_json::json!({}))
            } else {
                serde_json::json!({})
            };
            let overlay = parse_module_overlay(&overlay_value).map_err(CoreError::Validation)?;
            manifest = merge_module_manifest(&manifest, &overlay).map_err(CoreError::Validation)?;
        }
        for schema in &manifest.schemas {
            for field in &schema.fields {
                let Some(metadata_fields) = field.metadata_fields.as_deref() else {
                    continue;
                };
                let fields = merged
                    .entry(field.relationship_type.clone().ok_or_else(|| {
                        CoreError::Validation(format!(
                            "relationship metadata field {} is missing relationshipType",
                            field.key
                        ))
                    })?)
                    .or_default();
                for metadata_field in metadata_fields {
                    if let Some(existing) = fields.get(&metadata_field.key) {
                        if existing.field_type != metadata_field.field_type {
                            return Err(CoreError::Validation(format!(
                                "conflicting relationship metadata field type for {}",
                                metadata_field.key
                            )));
                        }
                    }
                    fields.insert(metadata_field.key.clone(), metadata_field.clone());
                }
            }
        }
    }
    Ok(merged
        .into_iter()
        .map(|(relationship_type, fields)| {
            (relationship_type, fields.into_values().collect::<Vec<_>>())
        })
        .collect())
}

fn refresh_relationship_metadata_schemas(
    core: &SharedCore,
    plugins: &SharedPluginHost,
) -> Result<(), String> {
    let session = current_session(core)?;
    let mut service = session
        .core
        .lock()
        .map_err(|_| "core lock poisoned".to_string())?;
    let project = service
        .project_mut(trusted_shell())
        .map_err(|error| error.to_string())?;
    let host = plugins
        .lock()
        .map_err(|_| "plugin host lock poisoned".to_string())?;
    let schemas = relationship_metadata_schemas_for_project(project, &host)
        .map_err(|error| error.to_string())?;
    project
        .set_relationship_metadata_schemas(schemas)
        .map_err(|error| error.to_string())
}

async fn sync_project_usage_and_wait(
    core: SharedCore,
    plugins: SharedPluginHost,
) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || {
        let project_id = current_info(&core)?
            .ok_or_else(|| "project is not open".to_string())?
            .root;
        let bundled = {
            let host = plugins
                .lock()
                .map_err(|_| "plugin host lock poisoned".to_string())?;
            host.catalog
                .list()
                .map(|entry| (entry.manifest.clone(), entry.digest.clone()))
                .collect::<Vec<_>>()
        };
        let project_root = Path::new(&project_id);
        let project =
            ProjectStore::open_read_only(project_root).map_err(|error| error.to_string())?;
        let states = project
            .module_states()
            .map_err(|error| error.to_string())?
            .into_iter()
            .map(|module| (module.module_id, (module.enabled, module.version)))
            .collect::<BTreeMap<_, _>>();
        let missing_disabled = bundled
            .iter()
            .filter(|(manifest, _)| {
                !manifest.enabled_by_default.unwrap_or(true) && !states.contains_key(&manifest.id)
            })
            .map(|(manifest, _)| manifest.id.clone())
            .collect::<Vec<_>>();
        let pending_migrations = bundled
            .iter()
            .filter(|(manifest, _)| {
                states
                    .get(&manifest.id)
                    .map(|(enabled, _)| *enabled)
                    .unwrap_or_else(|| manifest.enabled_by_default.unwrap_or(true))
            })
            .map(|(manifest, digest)| {
                let current = states
                    .get(&manifest.id)
                    .map(|(_, version)| *version)
                    .unwrap_or_default();
                core_migrations(manifest, digest).map(|migrations| {
                    migrations
                        .into_iter()
                        .filter(|migration| migration.from >= current)
                        .collect::<Vec<_>>()
                })
            })
            .collect::<Result<Vec<_>, _>>()?
            .into_iter()
            .flatten()
            .collect::<Vec<_>>();

        if !missing_disabled.is_empty() || !pending_migrations.is_empty() {
            let session = current_session(&core)?;
            let mut service = session
                .core
                .lock()
                .map_err(|_| "core lock poisoned".to_string())?;
            let current_root = service
                .info()
                .ok_or_else(|| "project is not open".to_string())?
                .root;
            if current_root != project_id {
                return Err("project changed while synchronizing module state".into());
            }
            let project = service
                .project_mut(trusted_shell())
                .map_err(|error| error.to_string())?;
            for module_id in missing_disabled {
                project
                    .set_module_enabled(module_id, false)
                    .map_err(|error| error.to_string())?;
            }
            if !pending_migrations.is_empty() {
                project
                    .apply_migrations(&pending_migrations)
                    .map_err(|error| error.to_string())?;
            }
        }

        let project =
            ProjectStore::open_read_only(project_root).map_err(|error| error.to_string())?;
        {
            let mut host = plugins
                .lock()
                .map_err(|_| "plugin host lock poisoned".to_string())?;
            sync_project_usage(&project, &mut host).map_err(|error| error.to_string())?;
        }
        refresh_relationship_metadata_schemas(&core, &plugins)
    })
    .await
    .map_err(|error| format!("module synchronization worker failed: {error}"))?
}

#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {name}! You've been greeted from Rust!")
}

#[tauri::command]
fn settings_get(settings: tauri::State<'_, SharedSettings>) -> Result<AppSettings, String> {
    settings
        .lock()
        .map_err(|_| "settings lock poisoned".to_string())?
        .load()
}

#[tauri::command]
fn settings_update(
    settings: tauri::State<'_, SharedSettings>,
    update: AppSettingsUpdate,
) -> Result<AppSettings, String> {
    settings
        .lock()
        .map_err(|_| "settings lock poisoned".to_string())?
        .update(update)
}

fn bundled_plugin_host(core: SharedCore) -> Result<PluginHost, String> {
    let mut host = PluginHost::new();
    for (manifest, wasm) in [
        (
            include_str!("../../packages/modules/lore/manifest.json"),
            None,
        ),
        (
            include_str!("../../packages/modules/timeline/manifest.json"),
            Some(BUNDLED_TIMELINE_SERVICE_WASM),
        ),
        (
            include_str!("../../packages/modules/writing/manifest.json"),
            None,
        ),
        (
            include_str!("../../packages/modules/maps/manifest.json"),
            None,
        ),
        (
            include_str!("../../packages/modules/language/manifest.json"),
            None,
        ),
    ] {
        host.register_bundled_json_with_wasm(manifest, wasm)
            .map_err(|error| error.to_string())?;
    }
    // Native provider for the public navigation service. Registered before
    // project activation so the manifest-declared WASM stub is skipped and
    // Lore/Timeline consumers reach the same resolution as the shell command.
    let navigation = maps_navigation_service_handler(core);
    host.register_declared_service_provider("daena.maps", "daena.maps/navigation", 1, navigation)
        .map_err(|error| error.to_string())?;
    Ok(host)
}

fn core_migrations(
    manifest: &PluginManifest,
    package_digest: &str,
) -> Result<Vec<Migration>, String> {
    manifest
        .migrations
        .iter()
        .map(|migration| {
            let operations = migration
                .operations
                .iter()
                .map(|operation| match operation {
                    MigrationOperation::CreateNamespace { namespace } => {
                        Operation::CreateNamespace {
                            namespace: namespace.clone(),
                        }
                    }
                    MigrationOperation::AddField {
                        namespace, field, ..
                    } => Operation::AddField {
                        namespace: namespace.clone(),
                        field: daena_core::FieldDefinition {
                            key: field.key.clone(),
                            field_type: field.field_type.clone(),
                            required: field.required.unwrap_or(false),
                        },
                    },
                    MigrationOperation::RenameField {
                        namespace,
                        from,
                        to,
                    } => Operation::RenameField {
                        namespace: namespace.clone(),
                        from: from.clone(),
                        to: to.clone(),
                    },
                    MigrationOperation::DropField { namespace, key } => Operation::DropField {
                        namespace: namespace.clone(),
                        key: key.clone(),
                    },
                })
                .collect();
            Ok(Migration {
                id: migration.id.clone(),
                module_id: manifest.id.clone(),
                from: migration.from as i64,
                to: migration.to as i64,
                operations,
                recovery: migration.recovery.clone(),
                package_digest: package_digest.into(),
            })
        })
        .collect()
}

#[tauri::command]
async fn module_list_manifests(
    state: tauri::State<'_, SharedCore>,
    plugins: tauri::State<'_, SharedPluginHost>,
) -> Result<Vec<serde_json::Value>, String> {
    let plugins = plugins.inner().clone();
    with_read_project(state, move |project| {
        let project_id = project.info().map(|info| info.root);
        let host = plugins
            .lock()
            .map_err(|_| CoreError::Conflict("plugin host lock poisoned".into()))?;
        let plugin_ids = host
            .catalog
            .list()
            .map(|entry| entry.manifest.id.clone())
            .collect::<Vec<_>>();
        let mut manifests = Vec::with_capacity(plugin_ids.len());
        for id in plugin_ids {
            let entry = project_id
                .as_deref()
                .and_then(|project_id| host.runtime_entry(project_id, &id))
                .or_else(|| host.catalog.get(&id).cloned())
                .ok_or_else(|| CoreError::Validation("plugin catalog entry disappeared".into()))?;
            let mut manifest_struct = entry.manifest.clone();
            if supports_schema_overlay(&entry.manifest) {
                let overlay_value = project
                    .module_schema_overlay(&id)?
                    .unwrap_or_else(|| serde_json::json!({}));
                let overlay =
                    parse_module_overlay(&overlay_value).map_err(CoreError::Validation)?;
                manifest_struct = merge_module_manifest(&entry.manifest, &overlay)
                    .map_err(CoreError::Validation)?;
            }
            let mut manifest = serde_json::to_value(&manifest_struct)
                .map_err(|error| CoreError::Validation(error.to_string()))?;
            let enabled = project.is_module_enabled(&id)?;
            manifest
                .as_object_mut()
                .expect("manifest is an object")
                .insert("enabled".into(), serde_json::Value::Bool(enabled));
            manifests.push(manifest);
        }
        Ok(manifests)
    })
    .await
}

#[tauri::command]
async fn module_schema_overlay_get(
    state: tauri::State<'_, SharedCore>,
    plugins: tauri::State<'_, SharedPluginHost>,
    module_id: String,
) -> Result<ModuleSchemaOverlay, String> {
    let plugins = plugins.inner().clone();
    with_read_project(state, move |project| {
        let project_id = project
            .info()
            .map(|info| info.root)
            .ok_or(CoreError::ProjectNotOpen)?;
        let package = {
            let host = plugins
                .lock()
                .map_err(|_| CoreError::Conflict("plugin host lock poisoned".into()))?;
            host.runtime_entry(&project_id, &module_id)
                .or_else(|| host.catalog.get(&module_id).cloned())
                .map(|entry| entry.manifest.clone())
                .ok_or_else(|| {
                    CoreError::Validation(format!("plugin is unavailable: {module_id}"))
                })?
        };
        if !supports_schema_overlay(&package) {
            return Err(CoreError::Validation(format!(
                "schema overlays are not supported for {module_id}"
            )));
        }
        let value = project
            .module_schema_overlay(&module_id)?
            .unwrap_or_else(|| serde_json::json!({}));
        parse_module_overlay(&value).map_err(CoreError::Validation)
    })
    .await
}

#[tauri::command]
async fn module_schema_editor_load(
    state: tauri::State<'_, SharedCore>,
    plugins: tauri::State<'_, SharedPluginHost>,
    module_id: String,
) -> Result<ModuleSchemaEditorState, String> {
    let plugins = plugins.inner().clone();
    with_read_project(state, move |project| {
        let project_id = project
            .info()
            .map(|info| info.root)
            .ok_or(CoreError::ProjectNotOpen)?;
        let package = {
            let host = plugins
                .lock()
                .map_err(|_| CoreError::Conflict("plugin host lock poisoned".into()))?;
            host.runtime_entry(&project_id, &module_id)
                .or_else(|| host.catalog.get(&module_id).cloned())
                .map(|entry| entry.manifest.clone())
                .ok_or_else(|| {
                    CoreError::Validation(format!("plugin is unavailable: {module_id}"))
                })?
        };
        if !supports_schema_overlay(&package) {
            return Err(CoreError::Validation(format!(
                "schema overlays are not supported for {module_id}"
            )));
        }
        if !project.is_module_enabled(&module_id)? {
            return Err(CoreError::Validation(format!(
                "plugin is disabled: {module_id}"
            )));
        }
        let value = project
            .module_schema_overlay(&module_id)?
            .unwrap_or_else(|| serde_json::json!({}));
        let overlay = parse_module_overlay(&value).map_err(CoreError::Validation)?;
        Ok(ModuleSchemaEditorState {
            id: package.id.clone(),
            name: package.name.clone(),
            schemas: package.schemas.clone(),
            templates: package.templates.clone(),
            overlay,
        })
    })
    .await
}

#[tauri::command]
async fn module_schema_overlay_set(
    state: tauri::State<'_, SharedCore>,
    plugins: tauri::State<'_, SharedPluginHost>,
    module_id: String,
    overlay: ModuleSchemaOverlay,
) -> Result<ModuleSchemaOverlay, String> {
    let plugins = plugins.inner().clone();
    with_core(state, move |core| {
        let context = trusted_shell();
        let project = core.project_mut(context)?;
        let project_id = project
            .info()
            .map(|info| info.root)
            .ok_or(CoreError::ProjectNotOpen)?;
        let package = {
            let host = plugins
                .lock()
                .map_err(|_| CoreError::Conflict("plugin host lock poisoned".into()))?;
            host.runtime_entry(&project_id, &module_id)
                .or_else(|| host.catalog.get(&module_id).cloned())
                .map(|entry| entry.manifest.clone())
                .ok_or_else(|| {
                    CoreError::Validation(format!("plugin is unavailable: {module_id}"))
                })?
        };
        if !supports_schema_overlay(&package) {
            return Err(CoreError::Validation(format!(
                "schema overlays are not supported for {module_id}"
            )));
        }
        let mut normalized = overlay;
        if normalized.version == 0 {
            normalized.version = SCHEMA_OVERLAY_VERSION;
        }
        daena_plugin_api::validate_module_overlay(&package, &normalized)
            .map_err(CoreError::Validation)?;
        let value = if normalized.is_empty() {
            None
        } else {
            Some(
                serde_json::to_value(&normalized)
                    .map_err(|error| CoreError::Validation(error.to_string()))?,
            )
        };
        project.set_module_schema_overlay(module_id, value)?;
        let host = plugins
            .lock()
            .map_err(|_| CoreError::Conflict("plugin host lock poisoned".into()))?;
        let schemas = relationship_metadata_schemas_for_project(project, &host)?;
        project.set_relationship_metadata_schemas(schemas)?;
        Ok(normalized)
    })
    .await
}

#[tauri::command]
async fn module_enable(
    state: tauri::State<'_, SharedCore>,
    plugins: tauri::State<'_, SharedPluginHost>,
    id: String,
    granted_capabilities: Option<Vec<String>>,
) -> Result<(), String> {
    let known = plugins
        .lock()
        .map_err(|_| "plugin host lock poisoned".to_string())?
        .catalog
        .get(&id)
        .is_some();
    if !known {
        return Err(format!("unknown plugin: {id}"));
    }
    let plugins = plugins.inner().clone();
    with_core(state, move |core| {
        let context = trusted_shell();
        let project = core.project_mut(context)?;
        let project_id = project
            .info()
            .map(|info| info.root)
            .ok_or(CoreError::ProjectNotOpen)?;
        if project.is_module_enabled(&id)? {
            let granted_capabilities = granted_capabilities.ok_or(CoreError::Unauthorized {
                operation: "explicit plugin capability review",
            })?;
            let mut host = plugins
                .lock()
                .map_err(|_| CoreError::Conflict("plugin host lock poisoned".into()))?;
            host.grant_capabilities(&project_id, &id, granted_capabilities.into_iter().collect())
                .map_err(|error| CoreError::Validation(error.to_string()))?;
            return Ok(());
        }
        let (manifest, package_digest) = {
            let mut host = plugins
                .lock()
                .map_err(|_| CoreError::Conflict("plugin host lock poisoned".into()))?;
            if let Some(version) = project.module_package_version(&id)? {
                host.select_project_version(&project_id, &id, &version)
                    .map_err(|error| CoreError::Validation(error.to_string()))?;
            }
            let entry = host
                .runtime_entry(&project_id, &id)
                .ok_or_else(|| CoreError::Validation("plugin runtime version is missing".into()))?;
            (entry.manifest, entry.digest)
        };
        {
            let mut host = plugins
                .lock()
                .map_err(|_| CoreError::Conflict("plugin host lock poisoned".into()))?;
            if let Some(granted_capabilities) = granted_capabilities {
                host.grant_capabilities(
                    &project_id,
                    &manifest.id,
                    granted_capabilities.into_iter().collect(),
                )
                .map_err(|error| CoreError::Validation(error.to_string()))?;
            } else if !manifest.capabilities.is_empty()
                && host.grants.is_empty(&project_id, &manifest.id)
            {
                return Err(CoreError::Unauthorized {
                    operation: "consent to plugin capabilities",
                });
            }
        }
        let migrations = core_migrations(&manifest, &package_digest)
            .map_err(|error| CoreError::Validation(error.to_string()))?;
        let current = project.get_module_version(&id)?;
        let pending = migrations
            .iter()
            .filter(|migration| migration.from >= current)
            .cloned()
            .collect::<Vec<_>>();
        let backup = if pending.is_empty() {
            None
        } else {
            Some(project.create_plugin_backup(
                &manifest.id,
                project.module_package_version(&id)?.as_deref(),
                Some(&manifest.version),
                current,
            )?)
        };
        if let Some(first) = pending.first() {
            if current != first.from {
                return Err(CoreError::Validation(format!(
                    "unsupported stored version {current} for plugin {}",
                    manifest.id
                )));
            }
            if let Err(error) = project.apply_migrations(&pending) {
                if let Some(backup) = &backup {
                    let _ = project.restore_plugin_backup(backup);
                }
                return Err(error);
            }
        }
        let mut host = plugins
            .lock()
            .map_err(|_| CoreError::Conflict("plugin host lock poisoned".into()))?;
        if let Err(error) = host.activate_bundled(&project_id, &manifest.id) {
            if let Some(backup) = &backup {
                let _ = project.restore_plugin_backup(backup);
            }
            return Err(CoreError::Validation(error.to_string()));
        }
        host.record_project_usage(&project_id, &id, &manifest.version)
            .map_err(|error| CoreError::Conflict(error.to_string()))?;
        project.set_module_enabled(id, true)?;
        let schemas = relationship_metadata_schemas_for_project(project, &host)?;
        project.set_relationship_metadata_schemas(schemas)?;
        Ok(())
    })
    .await
}

#[tauri::command]
async fn module_disable(
    app: tauri::AppHandle,
    state: tauri::State<'_, SharedCore>,
    plugins: tauri::State<'_, SharedPluginHost>,
    ai_runtime: tauri::State<'_, ai::SharedAiRuntime>,
    id: String,
) -> Result<(), String> {
    let known = plugins
        .lock()
        .map_err(|_| "plugin host lock poisoned".to_string())?
        .catalog
        .get(&id)
        .is_some();
    if !known {
        return Err(format!("unknown bundled plugin: {id}"));
    }
    let plugins = plugins.inner().clone();
    let ai_runtime = ai_runtime.inner().clone();
    with_core(state, move |core| {
        let project = core.project_mut(trusted_shell())?;
        let project_id = project
            .info()
            .map(|info| info.root)
            .ok_or(CoreError::ProjectNotOpen)?;
        project.set_module_enabled(id.clone(), false)?;
        let mut host = plugins
            .lock()
            .map_err(|_| CoreError::Conflict("plugin host lock poisoned".into()))?;
        for request_id in host.ai_request_ids_for(&project_id, Some(&id)) {
            let _ = ai::cancel_ai_request(&ai_runtime, &request_id);
            let _ = ai::remove_ai_citations(&ai_runtime, &request_id);
            host.remove_ai_request(&request_id);
        }
        if let Err(error) = host.clear_project_usage(&project_id, &id) {
            project.set_module_enabled(id, true)?;
            return Err(CoreError::Conflict(error.to_string()));
        }
        host.deactivate_bundled(&project_id, &id);
        close_plugin_webview(&app, &id);
        let schemas = relationship_metadata_schemas_for_project(project, &host)?;
        project.set_relationship_metadata_schemas(schemas)?;
        Ok(())
    })
    .await
}

#[tauri::command]
fn plugin_install_package(
    app: tauri::AppHandle,
    plugins: tauri::State<'_, SharedPluginHost>,
    archive: String,
    allow_unsigned: bool,
) -> Result<serde_json::Value, String> {
    let install_root = app
        .path()
        .app_data_dir()
        .map_err(|error| error.to_string())?
        .join("plugins");
    let policy = if allow_unsigned {
        VerificationPolicy::with_unsigned_consent()
    } else {
        VerificationPolicy::default()
    };
    let package = plugins
        .lock()
        .map_err(|_| "plugin host lock poisoned".to_string())?
        .install_package(archive, install_root, ArchiveLimits::default(), policy)
        .map_err(|error| error.to_string())?;
    serde_json::to_value(serde_json::json!({
        "id": package.manifest.id,
        "version": package.manifest.version,
        "publisher": package.manifest.publisher,
        "digest": package.digest,
        "signed": package.signed,
    }))
    .map_err(|error| error.to_string())
}

#[tauri::command]
async fn plugin_upgrade(
    state: tauri::State<'_, SharedCore>,
    plugins: tauri::State<'_, SharedPluginHost>,
    plugin_id: String,
    version: String,
    consent: bool,
) -> Result<(), String> {
    let plugins = plugins.inner().clone();
    with_core(state, move |core| {
        let project = core.project_mut(trusted_shell())?;
        let project_id = project
            .info()
            .map(|info| info.root)
            .ok_or(CoreError::ProjectNotOpen)?;
        let (old_version, old_grants, target_manifest, target_digest, plan) = {
            let host = plugins
                .lock()
                .map_err(|_| CoreError::Conflict("plugin host lock poisoned".into()))?;
            let old_version = host.selected_project_version(&project_id, &plugin_id);
            let old_grants = host.grants.get(&project_id, &plugin_id);
            let target = host.packages.get(&plugin_id, &version).ok_or_else(|| {
                CoreError::Validation("target plugin version is not installed".into())
            })?;
            let target_manifest = daena_plugin_api::parse_manifest(
                &std::fs::read_to_string(target.root.join("manifest.json"))
                    .map_err(|error| CoreError::Validation(error.to_string()))?,
            )
            .map_err(|error| CoreError::Validation(error.to_string()))?;
            let plan = host
                .plan_upgrade(
                    &plugin_id,
                    &version,
                    &project_id,
                    project.get_module_version(&plugin_id)? as u32,
                )
                .map_err(|error| CoreError::Validation(error.to_string()))?;
            (
                old_version,
                old_grants,
                target_manifest,
                target.digest.clone(),
                plan,
            )
        };
        if plan.consent.requires_renewal && !consent {
            return Err(CoreError::Unauthorized {
                operation: "consent to plugin capability changes",
            });
        }
        let old_version =
            old_version.ok_or(CoreError::Validation("plugin has no active version".into()))?;
        let current = project.get_module_version(&plugin_id)? as u32;
        let backup = project.create_plugin_backup(
            &plugin_id,
            Some(&old_version),
            Some(&version),
            current as i64,
        )?;
        let requested = target_manifest.capabilities.clone();
        let mut next_grants = old_grants
            .iter()
            .filter(|grant| requested.contains(grant))
            .cloned()
            .collect::<std::collections::BTreeSet<_>>();
        if consent {
            next_grants.extend(plan.consent.added.iter().cloned());
        }
        {
            let mut host = plugins
                .lock()
                .map_err(|_| CoreError::Conflict("plugin host lock poisoned".into()))?;
            host.deactivate_bundled(&project_id, &plugin_id);
            host.select_project_version(&project_id, &plugin_id, &version)
                .map_err(|error| CoreError::Validation(error.to_string()))?;
            host.grants
                .set(&project_id, &plugin_id, &requested, next_grants)
                .map_err(|error| CoreError::Validation(error.to_string()))?;
            host.persist_project_grants(&project_id)
                .map_err(|error| CoreError::Conflict(error.to_string()))?;
        }
        let migrations = core_migrations(&target_manifest, &target_digest)
            .map_err(|error| CoreError::Validation(error.to_string()))?
            .into_iter()
            .filter(|migration| migration.from >= current as i64)
            .collect::<Vec<_>>();
        if let Err(error) = project.apply_migrations(&migrations) {
            let _ = project.restore_plugin_backup(&backup);
            let mut host = plugins
                .lock()
                .map_err(|_| CoreError::Conflict("plugin host lock poisoned".into()))?;
            host.select_project_version(&project_id, &plugin_id, &old_version)
                .map_err(|restore| CoreError::Validation(restore.to_string()))?;
            host.grants
                .set(
                    &project_id,
                    &plugin_id,
                    &old_grants.iter().cloned().collect::<Vec<_>>(),
                    old_grants.clone(),
                )
                .map_err(|restore| CoreError::Validation(restore.to_string()))?;
            host.persist_project_grants(&project_id)
                .map_err(|restore| CoreError::Conflict(restore.to_string()))?;
            let _ = host.activate_bundled(&project_id, &plugin_id);
            return Err(error);
        }
        let activation = plugins
            .lock()
            .map_err(|_| CoreError::Conflict("plugin host lock poisoned".into()))?
            .activate_bundled(&project_id, &plugin_id);
        if let Err(error) = activation {
            let _ = project.restore_plugin_backup(&backup);
            let mut host = plugins
                .lock()
                .map_err(|_| CoreError::Conflict("plugin host lock poisoned".into()))?;
            host.select_project_version(&project_id, &plugin_id, &old_version)
                .map_err(|restore| CoreError::Validation(restore.to_string()))?;
            host.grants
                .set(
                    &project_id,
                    &plugin_id,
                    &old_grants.iter().cloned().collect::<Vec<_>>(),
                    old_grants,
                )
                .map_err(|restore| CoreError::Validation(restore.to_string()))?;
            host.persist_project_grants(&project_id)
                .map_err(|restore| CoreError::Conflict(restore.to_string()))?;
            let _ = host.activate_bundled(&project_id, &plugin_id);
            return Err(CoreError::Validation(error.to_string()));
        }
        project.set_module_package_version(&plugin_id, Some(&version))?;
        project.set_module_enabled(plugin_id, true)?;
        let host = plugins
            .lock()
            .map_err(|_| CoreError::Conflict("plugin host lock poisoned".into()))?;
        let schemas = relationship_metadata_schemas_for_project(project, &host)?;
        project.set_relationship_metadata_schemas(schemas)?;
        Ok(())
    })
    .await
}

#[tauri::command]
async fn plugin_rollback(
    state: tauri::State<'_, SharedCore>,
    plugins: tauri::State<'_, SharedPluginHost>,
    plugin_id: String,
    version: String,
) -> Result<(), String> {
    let plugins = plugins.inner().clone();
    with_core(state, move |core| {
        let project = core.project_mut(trusted_shell())?;
        let project_id = project
            .info()
            .map(|info| info.root)
            .ok_or(CoreError::ProjectNotOpen)?;
        let current = project.get_module_version(&plugin_id)? as u32;
        let current_version = plugins
            .lock()
            .map_err(|_| CoreError::Conflict("plugin host lock poisoned".into()))?
            .selected_project_version(&project_id, &plugin_id)
            .ok_or_else(|| CoreError::Validation("plugin has no selected version".into()))?;
        if current_version == version {
            return Err(CoreError::Validation(
                "rollback target is already selected".into(),
            ));
        }
        let backup = project
            .latest_plugin_backup(&plugin_id, Some(&version), Some(&current_version))?
            .ok_or_else(|| {
                CoreError::Validation("no pre-upgrade backup exists for rollback".into())
            })?;
        let safety_backup = project.create_plugin_backup(
            &plugin_id,
            Some(&current_version),
            Some(&version),
            current as i64,
        )?;
        let result = plugins
            .lock()
            .map_err(|_| CoreError::Conflict("plugin host lock poisoned".into()))?
            .rollback_plugin(
                &project_id,
                &plugin_id,
                &version,
                backup.data_version as u32,
            );
        if let Err(error) = result {
            let _ = project.restore_plugin_backup(&safety_backup);
            return Err(CoreError::Validation(error.to_string()));
        }
        if let Err(error) = project.restore_plugin_backup(&backup) {
            let _ = project.restore_plugin_backup(&safety_backup);
            let mut host = plugins.lock().map_err(|lock_error| {
                CoreError::Conflict(format!("plugin host lock poisoned: {lock_error}"))
            })?;
            host.select_project_version(&project_id, &plugin_id, &current_version)
                .map_err(|restore| CoreError::Validation(restore.to_string()))?;
            let _ = host.activate_bundled(&project_id, &plugin_id);
            return Err(error);
        }
        project.set_module_package_version(&plugin_id, Some(&version))?;
        project.set_module_enabled(plugin_id, true)?;
        let host = plugins
            .lock()
            .map_err(|_| CoreError::Conflict("plugin host lock poisoned".into()))?;
        let schemas = relationship_metadata_schemas_for_project(project, &host)?;
        project.set_relationship_metadata_schemas(schemas)?;
        Ok(())
    })
    .await
}

#[tauri::command]
async fn plugin_uninstall_code(
    state: tauri::State<'_, SharedCore>,
    plugins: tauri::State<'_, SharedPluginHost>,
    plugin_id: String,
    version: String,
) -> Result<(), String> {
    let plugins = plugins.inner().clone();
    with_core(state, move |core| {
        let mut detached_project: Option<(String, String)> = None;
        if core.info().is_some() {
            let project = core.project(trusted_shell())?;
            if project.module_package_version(&plugin_id)?.as_deref() == Some(version.as_str()) {
                if project.is_module_enabled(&plugin_id)? {
                    return Err(CoreError::Conflict(
                        "disable the plugin before uninstalling its selected code".into(),
                    ));
                }
                let project_id = project
                    .info()
                    .map(|info| info.root)
                    .ok_or(CoreError::ProjectNotOpen)?;
                project.set_module_package_version(&plugin_id, None)?;
                let clear_result = plugins
                    .lock()
                    .map_err(|_| CoreError::Conflict("plugin host lock poisoned".into()))?
                    .clear_project_usage(&project_id, &plugin_id);
                if let Err(error) = clear_result {
                    let _ = project.set_module_package_version(&plugin_id, Some(&version));
                    return Err(CoreError::Conflict(error.to_string()));
                }
                detached_project = Some((project_id, version.clone()));
            }
        }
        let uninstall_result = plugins
            .lock()
            .map_err(|_| CoreError::Conflict("plugin host lock poisoned".into()))?
            .uninstall_code(&plugin_id, &version)
            .map_err(|error| CoreError::Validation(error.to_string()));
        if let Err(error) = uninstall_result {
            if let Some((project_id, selected_version)) = detached_project {
                let project = core.project(trusted_shell())?;
                let _ = project.set_module_package_version(&plugin_id, Some(&selected_version));
                let restore_result = plugins
                    .lock()
                    .map_err(|_| CoreError::Conflict("plugin host lock poisoned".into()))?
                    .select_project_version(&project_id, &plugin_id, &selected_version);
                if let Err(restore_error) = restore_result {
                    return Err(CoreError::Conflict(format!(
                        "{error}; failed to restore project plugin selection: {restore_error}"
                    )));
                }
            }
            return Err(error);
        }
        Ok(())
    })
    .await
}

#[tauri::command]
async fn plugin_delete_data(
    state: tauri::State<'_, SharedCore>,
    plugins: tauri::State<'_, SharedPluginHost>,
    plugin_id: String,
    confirmation: String,
) -> Result<String, String> {
    let plugins = plugins.inner().clone();
    with_core(state, move |core| {
        if plugin_id.trim().is_empty() || confirmation != plugin_id {
            return Err(CoreError::Unauthorized {
                operation: "confirm plugin data deletion",
            });
        }
        let project = core.project_mut(trusted_shell())?;
        let project_id = project
            .info()
            .map(|info| info.root)
            .ok_or(CoreError::ProjectNotOpen)?;
        {
            let mut host = plugins
                .lock()
                .map_err(|_| CoreError::Conflict("plugin host lock poisoned".into()))?;
            host.deactivate_bundled(&project_id, &plugin_id);
        }
        project.set_module_enabled(plugin_id.clone(), false)?;
        let backup = project.delete_plugin_data(&plugin_id, &confirmation)?;
        plugins
            .lock()
            .map_err(|_| CoreError::Conflict("plugin host lock poisoned".into()))?
            .clear_project_usage(&project_id, &plugin_id)
            .map_err(|error| CoreError::Conflict(error.to_string()))?;
        Ok(backup)
    })
    .await
}

#[tauri::command]
async fn plugin_admin_view(
    state: tauri::State<'_, SharedCore>,
    plugins: tauri::State<'_, SharedPluginHost>,
) -> Result<serde_json::Value, String> {
    let plugins = plugins.inner().clone();
    with_core(state, move |core| {
        let context = trusted_shell();
        let project = core.project(context).ok();
        let project_id = project.and_then(|project| project.info().map(|info| info.root));
        let host = plugins
            .lock()
            .map_err(|_| CoreError::Conflict("plugin host lock poisoned".into()))?;
        let mut plugin_ids = host
            .catalog
            .list()
            .map(|entry| entry.manifest.id.clone())
            .collect::<Vec<_>>();
        for id in host.packages.plugin_ids() {
            if !plugin_ids.contains(id) {
                plugin_ids.push(id.clone());
            }
        }
        let mut plugins_view = Vec::with_capacity(plugin_ids.len());
        for id in plugin_ids {
            let entry = project_id
                .as_deref()
                .and_then(|project_id| host.runtime_entry(project_id, &id))
                .or_else(|| host.catalog.get(&id).cloned())
                .ok_or_else(|| CoreError::Validation("plugin catalog entry disappeared".into()))?;
            let manifest = &entry.manifest;
            let selected_version = project_id.as_deref().and_then(|project_id| {
                project
                    .and_then(|project| project.module_package_version(&id).ok().flatten())
                    .or_else(|| host.selected_project_version(project_id, &id))
            });
            let empty_project = "";
            let project_for_scope = project_id.as_deref().unwrap_or(empty_project);
            let lifecycle = host.lifecycle.state(project_for_scope, &id);
            let runtime_running = host.wasm.is_running(project_for_scope, &id);
            let granted_capabilities = if project_id.is_some() {
                host.grants.get(project_for_scope, &id).into_iter().collect::<Vec<_>>()
            } else {
                Vec::new()
            };
            let mut versions = Vec::new();
            let active_candidate = host
                .packages
                .active_candidate(&id)
                .map(|version| version.version.clone());
            for installed in host.packages.list(&id) {
                versions.push(serde_json::json!({
                    "version": installed.version,
                    "publisher": installed.publisher,
                    "digest": installed.digest,
                    "signed": installed.signed,
                    "unsignedConsent": installed.unsigned_consent,
                    "installedAt": installed.installed_at,
                    "isSelected": selected_version.as_deref() == Some(installed.version.as_str()),
                    "isActiveCandidate": active_candidate.as_deref() == Some(installed.version.as_str()),
                    "bundled": false,
                    "rollbackAvailable": false,
                }));
            }
            if versions.is_empty() {
                versions.push(serde_json::json!({
                    "version": manifest.version,
                    "publisher": manifest.publisher,
                    "digest": "",
                    "signed": false,
                    "unsignedConsent": false,
                    "installedAt": 0,
                    "isSelected": true,
                    "isActiveCandidate": true,
                    "bundled": true,
                    "rollbackAvailable": false,
                }));
            } else if let (Some(project), Some(selected)) = (project, selected_version.as_deref()) {
                for version in versions.iter_mut() {
                    let candidate = version
                        .get("version")
                        .and_then(|value| value.as_str())
                        .unwrap_or_default();
                    if candidate != selected {
                        let backup =
                            project.latest_plugin_backup(&id, Some(candidate), Some(selected))?;
                        version["rollbackAvailable"] =
                            serde_json::Value::Bool(backup.is_some());
                    }
                }
            }
            let dependency_state = match DependencyResolver::resolve(&host.catalog, &id) {
                Ok(plan) => serde_json::json!({
                    "resolved": true,
                    "order": plan.order,
                    "error": null,
                }),
                Err(error) => serde_json::json!({
                    "resolved": false,
                    "order": [],
                    "error": error.to_string(),
                }),
            };
            let mut view = serde_json::to_value(manifest)
                .map_err(|error| CoreError::Validation(error.to_string()))?;
            let object = view
                .as_object_mut()
                .expect("manifest is a JSON object");
            object.insert(
                "enabled".into(),
                serde_json::Value::Bool(
                    project.is_some_and(|project| project.is_module_enabled(&id).unwrap_or(false)),
                ),
            );
            object.insert(
                "selectedVersion".into(),
                selected_version.map_or(serde_json::Value::Null, |value| {
                    serde_json::Value::String(value)
                }),
            );
            object.insert(
                "dataVersion".into(),
                serde_json::Value::Number(
                    project
                        .and_then(|project| project.get_module_version(&id).ok())
                        .unwrap_or(0)
                        .into(),
                ),
            );
            object.insert(
                "lifecycle".into(),
                serde_json::json!({
                    "state": lifecycle.state,
                    "failures": lifecycle.failures,
                    "lastError": lifecycle.last_error,
                }),
            );
            object.insert("runtimeRunning".into(), serde_json::Value::Bool(runtime_running));
            object.insert(
                "grantedCapabilities".into(),
                serde_json::Value::Array(
                    granted_capabilities
                        .into_iter()
                        .map(serde_json::Value::String)
                        .collect(),
                ),
            );
            object.insert("installedVersions".into(), serde_json::Value::Array(versions));
            object.insert("dependencyState".into(), dependency_state);
            let bundled = entry.package_root.as_os_str().is_empty();
            object.insert(
                "distribution".into(),
                serde_json::json!({
                    "origin": if bundled { "bundled" } else { "installed" },
                    "management": if bundled { "app" } else { "user" },
                    "canUninstall": !bundled,
                }),
            );
            plugins_view.push(view);
        }
        Ok(serde_json::json!({ "plugins": plugins_view }))
    })
    .await
}

#[tauri::command]
async fn plugin_upgrade_plan(
    state: tauri::State<'_, SharedCore>,
    plugins: tauri::State<'_, SharedPluginHost>,
    plugin_id: String,
    version: String,
) -> Result<serde_json::Value, String> {
    let plugins = plugins.inner().clone();
    with_core(state, move |core| {
        let project = core.project(trusted_shell())?;
        let project_id = project
            .info()
            .map(|info| info.root)
            .ok_or(CoreError::ProjectNotOpen)?;
        let current = project.get_module_version(&plugin_id)? as u32;
        let host = plugins
            .lock()
            .map_err(|_| CoreError::Conflict("plugin host lock poisoned".into()))?;
        let plan = host
            .plan_upgrade(&plugin_id, &version, &project_id, current)
            .map_err(|error| CoreError::Validation(error.to_string()))?;
        let target = host.packages.get(&plugin_id, &version).ok_or_else(|| {
            CoreError::Validation("target plugin version is not installed".into())
        })?;
        Ok(serde_json::json!({
            "pluginId": plan.plugin_id,
            "fromVersion": plan.from_version,
            "toVersion": plan.to_version,
            "consent": {
                "added": plan.consent.added,
                "removed": plan.consent.removed,
                "requiresRenewal": plan.consent.requires_renewal,
            },
            "migrations": {
                "from": plan.migrations.from,
                "to": plan.migrations.to,
                "migrationIds": plan.migrations.migration_ids,
                "requiresBackup": plan.migrations.requires_backup,
            },
            "target": {
                "signed": target.signed,
                "publisher": target.publisher,
            },
        }))
    })
    .await
}

#[tauri::command]
async fn plugin_retry(
    state: tauri::State<'_, SharedCore>,
    plugins: tauri::State<'_, SharedPluginHost>,
    plugin_id: String,
) -> Result<(), String> {
    let plugins = plugins.inner().clone();
    with_core(state, move |core| {
        let project = core.project(trusted_shell())?;
        let project_id = project
            .info()
            .map(|info| info.root)
            .ok_or(CoreError::ProjectNotOpen)?;
        let enabled = project.is_module_enabled(&plugin_id)?;
        let mut host = plugins
            .lock()
            .map_err(|_| CoreError::Conflict("plugin host lock poisoned".into()))?;
        host.retry_plugin(&project_id, &plugin_id);
        if enabled {
            host.activate_bundled(&project_id, &plugin_id)
                .map_err(|error| CoreError::Validation(error.to_string()))?;
        }
        Ok(())
    })
    .await
}

fn payload_string(payload: &serde_json::Value, key: &str) -> Result<String, CoreError> {
    payload
        .get(key)
        .and_then(serde_json::Value::as_str)
        .map(str::to_owned)
        .ok_or_else(|| CoreError::Validation(format!("plugin RPC payload requires {key}")))
}

fn optional_payload_string(payload: &serde_json::Value, keys: &[&str]) -> Option<String> {
    keys.iter()
        .find_map(|key| payload.get(*key).and_then(serde_json::Value::as_str))
        .map(str::to_owned)
}

fn required_payload_string(
    payload: &serde_json::Value,
    keys: &[&str],
    label: &str,
) -> Result<String, CoreError> {
    optional_payload_string(payload, keys).ok_or_else(|| {
        CoreError::Conflict(format!("revision-aware broker mutation requires {label}"))
    })
}

fn payload_value<T: serde::de::DeserializeOwned>(
    payload: &serde_json::Value,
) -> Result<T, CoreError> {
    serde_json::from_value(payload.clone())
        .map_err(|error| CoreError::Validation(format!("invalid plugin RPC payload: {error}")))
}

fn validate_broker_payload(method: &str, payload: &serde_json::Value) -> Result<(), CoreError> {
    let object = payload.as_object().ok_or_else(|| {
        CoreError::Validation(format!("plugin RPC payload for {method} must be an object"))
    })?;
    let (required, optional): (&[&str], &[&str]) = match method {
        "entity.list" => (&[], &["entityType"]),
        "entity.get" => (&["id"], &[]),
        "entity.create" => (&["name"], &["type", "fields", "relationships", "document"]),
        "entity.update" => (&["id", "expectedRevision"], &["name", "type"]),
        "entity.delete" => (&["id", "expectedRevision"], &[]),
        "document.list" => (&["entityId"], &[]),
        "document.save" => (&["entityId", "body", "expectedRevision"], &["format"]),
        "field.read" => (&["entityId", "namespace", "key"], &[]),
        "field.list" => (&["entityId", "namespace"], &[]),
        "field.set" => (
            &["entityId", "namespace", "key", "value", "expectedRevision"],
            &[],
        ),
        "record.list" => (
            &["collection", "ownerEntityId"],
            &[
                "query",
                "limit",
                "offset",
                "sort",
                "status",
                "tag",
                "homonymsOnly",
            ],
        ),
        "record.create" => (&["collection", "ownerEntityId", "value"], &[]),
        "record.update" => (
            &[
                "collection",
                "id",
                "ownerEntityId",
                "value",
                "expectedRevision",
            ],
            &[],
        ),
        "record.delete" => (
            &["collection", "id", "ownerEntityId", "expectedRevision"],
            &[],
        ),
        "relationship.list" => (&["entityId"], &[]),
        "relationship.create" => (
            &[
                "source_id",
                "target_id",
                "relationship_type",
                "expectedRevision",
            ],
            &["metadata"],
        ),
        "relationship.update" => (&["id", "expectedRevision"], &["metadata", "target_id"]),
        "relationship.delete" => (&["id", "expectedRevision"], &["relationship_type"]),
        "asset.list" => (&["entityId"], &["namespace"]),
        "asset.register" => (
            &[
                "entity_id",
                "namespace",
                "filename",
                "content_hash",
                "size",
                "mime_type",
                "path",
                "expectedRevision",
            ],
            &[],
        ),
        "asset.update" => (
            &["assetId", "namespace", "expectedRevision"],
            &["filename", "role", "referenceScope"],
        ),
        "asset.delete" => (&["assetId", "namespace", "expectedRevision"], &[]),
        "asset.read.begin" => (&["assetId", "namespace"], &[]),
        "asset.replace.begin" => (
            &[
                "assetId",
                "namespace",
                "expectedRevision",
                "size",
                "mimeType",
            ],
            &[],
        ),
        "asset.replace.commit" => (&["handle", "contentHash"], &[]),
        "asset.transfer.cancel" => (&["handle"], &[]),
        "maps.asset.create.begin" => (&["mapEntityId", "size"], &["mimeType"]),
        "maps.asset.create.commit" => (&["handle", "contentHash"], &[]),
        "maps.image.import.begin" => (&["name", "size", "mimeType", "filename"], &[]),
        "maps.image.import.commit" => (&["handle", "contentHash"], &[]),
        "maps.vector.create.begin" => (&["name", "size", "generation"], &[]),
        "maps.vector.create.commit" => (&["handle", "contentHash"], &[]),
        "maps.physical.create.begin" => (&["name", "size", "generation"], &[]),
        "maps.physical.create.commit" => (&["handle", "contentHash"], &[]),
        "maps.vector.replace.begin" => (&["assetId", "expectedRevision", "size"], &[]),
        "maps.vector.replace.commit" => (&["handle", "contentHash"], &[]),
        "maps.layer.create" => (&["mapEntityId", "name", "expectedRevision"], &["kind"]),
        "maps.layer.delete" => (
            &["mapEntityId", "layerId", "expectedRevision"],
            &["expectedSourceRevision", "expectedFeatureCount"],
        ),
        "maps.layer.update" => (
            &["mapEntityId", "layerId", "expectedRevision"],
            &[
                "name",
                "order",
                "defaultVisible",
                "opacity",
                "locked",
                "style",
            ],
        ),
        "maps.recovery.export.begin" => (&["mapEntityId", "size"], &[]),
        "maps.recovery.export.commit" => (&["handle", "contentHash"], &[]),
        "maps.recovery.list" => (&["mapEntityId"], &[]),
        "maps.recovery.restore" => (&["mapEntityId", "fileName"], &[]),
        "maps.locations.list" => (&["mapEntityId"], &[]),
        "maps.locations.upsert" => (&["entityId", "location"], &[]),
        "maps.locations.unlink" => (&["entityId", "locationId"], &[]),
        "maps.locations.create_and_link" => (&["name", "entityType", "location"], &[]),
        "maps.reconcile.links" => (&["mapEntityId"], &[]),
        "search.query" => (&["query"], &[]),
        "event.publish" => (&["type", "payload"], &[]),
        "event.subscribe" | "event.poll" => (&["type"], &[]),
        "service.call" => (&["name", "major", "payload"], &["deadlineMs"]),
        "ai.request.start" => (
            &["operation", "taskId", "userInstruction", "immediateContext"],
            &["outputContract", "deadlineMs", "retrievalPolicy"],
        ),
        "ai.request.poll" | "ai.request.cancel" | "ai.request.result" | "ai.request.citations" => {
            (&["requestId"], &[])
        }
        _ => {
            return Err(CoreError::Validation(format!(
                "unknown plugin RPC method: {method}"
            )));
        }
    };
    for key in required {
        if !object.contains_key(*key) {
            return Err(CoreError::Validation(format!(
                "plugin RPC payload for {method} requires {key}"
            )));
        }
    }
    for key in object.keys() {
        if !required.contains(&key.as_str()) && !optional.contains(&key.as_str()) {
            return Err(CoreError::Validation(format!(
                "plugin RPC payload for {method} contains unknown key {key}"
            )));
        }
    }
    Ok(())
}

fn validate_record_owner_entity_type(
    project: &ProjectStore,
    owner_entity_id: &str,
    allowed: Option<&[String]>,
) -> Result<(), CoreError> {
    let allowed = allowed.ok_or_else(|| CoreError::Unauthorized {
        operation: "access undeclared module record collection",
    })?;
    let owner = project
        .list_entities()?
        .into_iter()
        .find(|entity| entity.id == owner_entity_id)
        .ok_or_else(|| CoreError::NotFound("module record owner entity not found".into()))?;
    if !owner
        .entity_type
        .as_ref()
        .is_some_and(|entity_type| allowed.contains(entity_type))
    {
        return Err(CoreError::Unauthorized {
            operation: "use disallowed module record owner entity type",
        });
    }
    Ok(())
}

fn dispatch_module_rpc(
    core: &mut CoreService,
    plugin_id: Option<&str>,
    record_owner_entity_types: Option<Vec<String>>,
    method: &str,
    payload: serde_json::Value,
    request_id: Option<&str>,
) -> Result<serde_json::Value, CoreError> {
    validate_broker_payload(method, &payload)?;
    let project = core.project_mut(AuthorityContext::plugin())?;
    match method {
        "entity.list" => {
            let entity_type = payload
                .get("entityType")
                .and_then(serde_json::Value::as_str);
            let entities = project
                .list_entities()?
                .into_iter()
                .filter(|entity| {
                    entity_type.is_none_or(|kind| entity.entity_type.as_deref() == Some(kind))
                })
                .collect::<Vec<_>>();
            serde_json::to_value(entities).map_err(|error| CoreError::Validation(error.to_string()))
        }
        "entity.get" => {
            let id = payload_string(&payload, "id")?;
            let entity = project
                .list_entities()?
                .into_iter()
                .find(|entity| entity.id == id);
            serde_json::to_value(entity).map_err(|error| CoreError::Validation(error.to_string()))
        }
        "entity.create" => {
            let fields = payload
                .get("fields")
                .cloned()
                .map(serde_json::from_value)
                .transpose()
                .map_err(|error| CoreError::Validation(format!("invalid entity fields: {error}")))?
                .unwrap_or_default();
            let document = payload
                .get("document")
                .cloned()
                .map(serde_json::from_value)
                .transpose()
                .map_err(|error| {
                    CoreError::Validation(format!("invalid entity document: {error}"))
                })?;
            let relationships = payload
                .get("relationships")
                .cloned()
                .map(serde_json::from_value)
                .transpose()
                .map_err(|error| {
                    CoreError::Validation(format!("invalid entity relationships: {error}"))
                })?
                .unwrap_or_default();
            let input = daena_core::CreateEntry {
                name: payload_string(&payload, "name")?,
                entity_type: payload
                    .get("type")
                    .and_then(serde_json::Value::as_str)
                    .map(str::to_owned),
                document,
                fields,
                relationships,
            };
            serde_json::to_value(project.create_entry_with_request(input, request_id)?)
                .map_err(|error| CoreError::Validation(error.to_string()))
        }
        "entity.update" => {
            let id = payload_string(&payload, "id")?;
            let name = payload
                .get("name")
                .and_then(serde_json::Value::as_str)
                .map(str::to_owned);
            let entity_type = payload
                .get("type")
                .and_then(|value| {
                    if value.is_null() {
                        None
                    } else {
                        value.as_str()
                    }
                })
                .map(str::to_owned);
            let expected_revision = required_payload_string(
                &payload,
                &["expectedRevision", "expected_revision", "revision"],
                "expectedRevision",
            )?;
            serde_json::to_value(project.update_entity_with_options(
                id,
                name,
                entity_type,
                Some(&expected_revision),
                request_id,
            )?)
            .map_err(|error| CoreError::Validation(error.to_string()))
        }
        "entity.delete" => {
            let expected_revision = required_payload_string(
                &payload,
                &["expectedRevision", "expected_revision", "revision"],
                "expectedRevision",
            )?;
            project.delete_entity_with_options(
                payload_string(&payload, "id")?,
                Some(&expected_revision),
                request_id,
            )?;
            Ok(serde_json::Value::Null)
        }
        "document.list" => {
            serde_json::to_value(project.list_documents(payload_string(&payload, "entityId")?)?)
                .map_err(|error| CoreError::Validation(error.to_string()))
        }
        "document.save" => {
            let expected_revision = required_payload_string(
                &payload,
                &["expectedRevision", "expected_revision", "revision"],
                "expectedRevision",
            )?;
            let input = daena_core::SaveDocument {
                entity_id: payload_string(&payload, "entityId")?,
                body: payload_string(&payload, "body")?,
                format: payload
                    .get("format")
                    .and_then(serde_json::Value::as_str)
                    .map(str::to_owned),
            };
            project.save_document_with_options(input, Some(&expected_revision), request_id)?;
            Ok(serde_json::Value::Null)
        }
        "field.read" | "field.list" => {
            let entity_id = payload_string(&payload, "entityId")?;
            let namespace = payload_string(&payload, "namespace")?;
            let shared_keys = payload
                .get("__shared_keys")
                .and_then(serde_json::Value::as_array)
                .map(|keys| {
                    keys.iter()
                        .filter_map(serde_json::Value::as_str)
                        .collect::<std::collections::BTreeSet<_>>()
                });
            let mut fields = project
                .list_fields(entity_id)?
                .into_iter()
                .filter(|field| field.namespace == namespace)
                .filter(|field| {
                    shared_keys
                        .as_ref()
                        .is_none_or(|keys| keys.contains(field.key.as_str()))
                })
                .collect::<Vec<_>>();
            if method == "field.read" {
                let key = payload_string(&payload, "key")?;
                fields.retain(|field| field.key == key);
            }
            serde_json::to_value(fields).map_err(|error| CoreError::Validation(error.to_string()))
        }
        "field.set" => {
            let field = daena_core::FieldValue {
                entity_id: payload_string(&payload, "entityId")?,
                namespace: payload_string(&payload, "namespace")?,
                key: payload_string(&payload, "key")?,
                value: payload
                    .get("value")
                    .cloned()
                    .unwrap_or(serde_json::Value::Null),
                revision: required_payload_string(
                    &payload,
                    &["expectedRevision", "expected_revision", "revision"],
                    "expectedRevision",
                )?,
            };
            project.set_field_with_request(field, request_id)?;
            Ok(serde_json::Value::Null)
        }
        "record.list" => {
            let module_id = plugin_id.ok_or_else(|| CoreError::Unauthorized {
                operation: "access module records without plugin identity",
            })?;
            validate_record_owner_entity_type(
                project,
                &payload_string(&payload, "ownerEntityId")?,
                record_owner_entity_types.as_deref(),
            )?;
            let limit = payload
                .get("limit")
                .and_then(serde_json::Value::as_u64)
                .unwrap_or(50) as usize;
            let offset = payload
                .get("offset")
                .and_then(serde_json::Value::as_u64)
                .unwrap_or_default() as usize;
            serde_json::to_value(
                project.list_module_records_with(
                    module_id,
                    &payload_string(&payload, "collection")?,
                    &payload_string(&payload, "ownerEntityId")?,
                    daena_core::ModuleRecordListParams {
                        query: payload.get("query").and_then(serde_json::Value::as_str),
                        limit,
                        offset,
                        sort: payload.get("sort").and_then(serde_json::Value::as_str),
                        status: payload.get("status").and_then(serde_json::Value::as_str),
                        tag: payload.get("tag").and_then(serde_json::Value::as_str),
                        homonyms_only: payload
                            .get("homonymsOnly")
                            .and_then(serde_json::Value::as_bool)
                            .unwrap_or(false),
                    },
                )?,
            )
            .map_err(|error| CoreError::Validation(error.to_string()))
        }
        "record.create" => {
            let module_id = plugin_id.ok_or_else(|| CoreError::Unauthorized {
                operation: "create module records without plugin identity",
            })?;
            validate_record_owner_entity_type(
                project,
                &payload_string(&payload, "ownerEntityId")?,
                record_owner_entity_types.as_deref(),
            )?;
            serde_json::to_value(
                project.create_module_record(
                    module_id,
                    &payload_string(&payload, "collection")?,
                    &payload_string(&payload, "ownerEntityId")?,
                    payload
                        .get("value")
                        .cloned()
                        .ok_or_else(|| CoreError::Validation("record value is required".into()))?,
                    request_id,
                )?,
            )
            .map_err(|error| CoreError::Validation(error.to_string()))
        }
        "record.update" => {
            let module_id = plugin_id.ok_or_else(|| CoreError::Unauthorized {
                operation: "update module records without plugin identity",
            })?;
            validate_record_owner_entity_type(
                project,
                &payload_string(&payload, "ownerEntityId")?,
                record_owner_entity_types.as_deref(),
            )?;
            serde_json::to_value(
                project.update_module_record(
                    module_id,
                    &payload_string(&payload, "collection")?,
                    &payload_string(&payload, "id")?,
                    &payload_string(&payload, "ownerEntityId")?,
                    payload
                        .get("value")
                        .cloned()
                        .ok_or_else(|| CoreError::Validation("record value is required".into()))?,
                    &required_payload_string(
                        &payload,
                        &["expectedRevision", "expected_revision", "revision"],
                        "expectedRevision",
                    )?,
                    request_id,
                )?,
            )
            .map_err(|error| CoreError::Validation(error.to_string()))
        }
        "record.delete" => {
            let module_id = plugin_id.ok_or_else(|| CoreError::Unauthorized {
                operation: "delete module records without plugin identity",
            })?;
            validate_record_owner_entity_type(
                project,
                &payload_string(&payload, "ownerEntityId")?,
                record_owner_entity_types.as_deref(),
            )?;
            project.delete_module_record(
                module_id,
                &payload_string(&payload, "collection")?,
                &payload_string(&payload, "id")?,
                &payload_string(&payload, "ownerEntityId")?,
                &required_payload_string(
                    &payload,
                    &["expectedRevision", "expected_revision", "revision"],
                    "expectedRevision",
                )?,
                request_id,
            )?;
            Ok(serde_json::Value::Null)
        }
        "relationship.list" => {
            serde_json::to_value(project.list_relationships(payload_string(&payload, "entityId")?)?)
                .map_err(|error| CoreError::Validation(error.to_string()))
        }
        "relationship.create" => {
            let expected_revision = required_payload_string(
                &payload,
                &["expectedRevision", "expected_revision", "revision"],
                "expectedRevision",
            )?;
            let input: RelationshipInput = payload_value(&payload)?;
            serde_json::to_value(project.create_relationship_with_options(
                input,
                Some(&expected_revision),
                request_id,
            )?)
            .map_err(|error| CoreError::Validation(error.to_string()))
        }
        "relationship.update" => {
            let expected_revision = required_payload_string(
                &payload,
                &["expectedRevision", "expected_revision", "revision"],
                "expectedRevision",
            )?;
            let input = daena_core::RelationshipUpdate {
                id: payload_string(&payload, "id")?,
                metadata: payload
                    .get("metadata")
                    .and_then(serde_json::Value::as_str)
                    .map(str::to_owned),
                target_id: payload
                    .get("target_id")
                    .and_then(serde_json::Value::as_str)
                    .map(str::to_owned),
            };
            serde_json::to_value(project.update_relationship_with_options(
                input,
                Some(&expected_revision),
                request_id,
            )?)
            .map_err(|error| CoreError::Validation(error.to_string()))
        }
        "relationship.delete" => {
            let expected_revision = required_payload_string(
                &payload,
                &["expectedRevision", "expected_revision", "revision"],
                "expectedRevision",
            )?;
            project.delete_relationship_with_options(
                payload_string(&payload, "id")?,
                Some(&expected_revision),
                request_id,
            )?;
            Ok(serde_json::Value::Null)
        }
        "asset.list" => {
            let entity_id = payload_string(&payload, "entityId")?;
            let assets = project.list_assets(entity_id)?;
            serde_json::to_value(assets).map_err(|error| CoreError::Validation(error.to_string()))
        }
        "asset.register" => {
            let expected_revision = required_payload_string(
                &payload,
                &["expectedRevision", "expected_revision", "revision"],
                "expectedRevision",
            )?;
            let input: AssetInput = payload_value(&payload)?;
            serde_json::to_value(project.register_asset_with_options(
                input,
                Some(&expected_revision),
                request_id,
            )?)
            .map_err(|error| CoreError::Validation(error.to_string()))
        }
        "asset.update" => {
            let expected_revision = required_payload_string(
                &payload,
                &["expectedRevision", "expected_revision", "revision"],
                "expectedRevision",
            )?;
            let asset_id = payload_string(&payload, "assetId")?;
            let namespace = payload_string(&payload, "namespace")?;
            if project.asset(asset_id.clone())?.namespace != namespace {
                return Err(CoreError::Unauthorized {
                    operation: "update asset outside owned namespace",
                });
            }
            let input = AssetMetadataUpdate {
                asset_id,
                filename: payload
                    .get("filename")
                    .and_then(serde_json::Value::as_str)
                    .map(str::to_owned),
                role: payload
                    .get("role")
                    .and_then(serde_json::Value::as_str)
                    .map(str::to_owned),
                reference_scope: payload
                    .get("referenceScope")
                    .and_then(serde_json::Value::as_str)
                    .map(str::to_owned),
            };
            serde_json::to_value(project.update_asset_metadata_with_request(
                input,
                &expected_revision,
                request_id,
            )?)
            .map_err(|error| CoreError::Validation(error.to_string()))
        }
        "asset.delete" => {
            let expected_revision = required_payload_string(
                &payload,
                &["expectedRevision", "expected_revision", "revision"],
                "expectedRevision",
            )?;
            let asset_id = payload_string(&payload, "assetId")?;
            let namespace = payload_string(&payload, "namespace")?;
            if project.asset(asset_id.clone())?.namespace != namespace {
                return Err(CoreError::Unauthorized {
                    operation: "delete asset outside owned namespace",
                });
            }
            project.delete_asset_with_request(asset_id, &expected_revision, request_id)?;
            Ok(serde_json::Value::Null)
        }
        "search.query" => serde_json::to_value(project.search(payload_string(&payload, "query")?)?)
            .map_err(|error| CoreError::Validation(error.to_string())),
        "maps.recovery.list" => {
            let entity_id = payload_string(&payload, "mapEntityId")?;
            serde_json::to_value(project.list_map_recovery_copies(&entity_id)?)
                .map_err(|error| CoreError::Validation(error.to_string()))
        }
        "maps.recovery.restore" => {
            let entity_id = payload_string(&payload, "mapEntityId")?;
            let file_name = payload_string(&payload, "fileName")?;
            serde_json::to_value(
                project.restore_map_recovery_copy(&entity_id, &file_name, request_id)?,
            )
            .map_err(|error| CoreError::Validation(error.to_string()))
        }
        "maps.locations.list" => {
            let map_entity_id = payload_string(&payload, "mapEntityId")?;
            serde_json::to_value(project.map_location_projection(map_entity_id)?)
                .map_err(|error| CoreError::Validation(error.to_string()))
        }
        "maps.locations.upsert" => {
            let entity_id = payload_string(&payload, "entityId")?;
            let location = payload
                .get("location")
                .cloned()
                .ok_or_else(|| CoreError::Validation("location is required".into()))?;
            let location: daena_core::maps::LocationReference = serde_json::from_value(location)
                .map_err(|error| CoreError::Validation(format!("invalid location: {error}")))?;
            project.upsert_map_location(entity_id, location, request_id)?;
            Ok(serde_json::Value::Null)
        }
        "maps.layer.create" => {
            let map_entity_id = payload_string(&payload, "mapEntityId")?;
            let name = payload_string(&payload, "name")?;
            let expected_revision = payload_string(&payload, "expectedRevision")?;
            let kind = payload
                .get("kind")
                .and_then(serde_json::Value::as_str)
                .map(str::to_owned);
            let provider = project.map_provider_id(&map_entity_id)?;
            let create_vector = match kind.as_deref() {
                Some("vector") => true,
                Some("raster") => false,
                Some(other) => {
                    return Err(CoreError::Validation(format!(
                        "maps: unsupported layer kind {other}"
                    )));
                }
                None => provider == daena_core::maps::VECTOR_PROVIDER,
            };
            if create_vector {
                serde_json::to_value(project.create_vector_layer(
                    map_entity_id,
                    name,
                    &expected_revision,
                    request_id,
                    None,
                )?)
                .map_err(|error| CoreError::Validation(error.to_string()))
            } else {
                serde_json::to_value(project.create_raster_layer(
                    map_entity_id,
                    name,
                    &expected_revision,
                    request_id,
                )?)
                .map_err(|error| CoreError::Validation(error.to_string()))
            }
        }
        "maps.layer.delete" => {
            let map_entity_id = payload_string(&payload, "mapEntityId")?;
            let layer_id = payload_string(&payload, "layerId")?;
            let expected_revision = payload_string(&payload, "expectedRevision")?;
            let kind = project
                .map_layer_kind(&map_entity_id, &layer_id)?
                .ok_or_else(|| CoreError::NotFound("layer not found".into()))?;
            if kind == "vector" {
                let expected_source_revision = payload_string(&payload, "expectedSourceRevision")?;
                let expected_feature_count = payload
                    .get("expectedFeatureCount")
                    .and_then(serde_json::Value::as_i64)
                    .ok_or_else(|| {
                        CoreError::Validation("expectedFeatureCount is required".into())
                    })?;
                serde_json::to_value(project.delete_vector_layer(
                    map_entity_id,
                    layer_id,
                    &expected_revision,
                    &expected_source_revision,
                    expected_feature_count,
                    request_id,
                )?)
                .map_err(|error| CoreError::Validation(error.to_string()))
            } else {
                serde_json::to_value(project.delete_raster_layer(
                    map_entity_id,
                    layer_id,
                    &expected_revision,
                    request_id,
                )?)
                .map_err(|error| CoreError::Validation(error.to_string()))
            }
        }
        "maps.layer.update" => {
            let map_entity_id = payload_string(&payload, "mapEntityId")?;
            let layer_id = payload_string(&payload, "layerId")?;
            let expected_revision = payload_string(&payload, "expectedRevision")?;
            serde_json::to_value(
                project.update_map_layer(
                    map_entity_id,
                    layer_id,
                    daena_core::RasterLayerUpdate {
                        name: payload
                            .get("name")
                            .and_then(serde_json::Value::as_str)
                            .map(str::to_owned),
                        order: payload.get("order").and_then(serde_json::Value::as_i64),
                        default_visible: payload
                            .get("defaultVisible")
                            .and_then(serde_json::Value::as_bool),
                        opacity: payload.get("opacity").and_then(serde_json::Value::as_f64),
                        locked: payload.get("locked").and_then(serde_json::Value::as_bool),
                        style: payload.get("style").cloned(),
                        selector: payload.get("selector").cloned(),
                    },
                    &expected_revision,
                    request_id,
                )?,
            )
            .map_err(|error| CoreError::Validation(error.to_string()))
        }
        "maps.locations.unlink" => {
            let entity_id = payload_string(&payload, "entityId")?;
            let location_id = payload_string(&payload, "locationId")?;
            project.unlink_map_location(entity_id, location_id, request_id)?;
            Ok(serde_json::Value::Null)
        }
        "maps.locations.create_and_link" => {
            let name = payload_string(&payload, "name")?;
            let entity_type = payload_string(&payload, "entityType")?;
            if entity_type == daena_core::maps::MAP_ENTITY_TYPE {
                return Err(CoreError::Validation(
                    "create_and_link cannot create map entities".into(),
                ));
            }
            let location = payload
                .get("location")
                .cloned()
                .ok_or_else(|| CoreError::Validation("location is required".into()))?;
            let mut location: daena_core::maps::LocationReference =
                serde_json::from_value(location)
                    .map_err(|error| CoreError::Validation(format!("invalid location: {error}")))?;
            let created = project.create_entry_with_request(
                daena_core::CreateEntry {
                    name,
                    entity_type: Some(entity_type),
                    document: None,
                    fields: Vec::new(),
                    relationships: Vec::new(),
                },
                request_id,
            )?;
            location.label = created.name.clone();
            project.upsert_map_location(created.id.clone(), location, None)?;
            serde_json::to_value(created).map_err(|error| CoreError::Validation(error.to_string()))
        }
        "maps.reconcile.links" => {
            let map_entity_id = payload_string(&payload, "mapEntityId")?;
            serde_json::to_value(project.reconcile_map_links(map_entity_id)?)
                .map_err(|error| CoreError::Validation(error.to_string()))
        }
        _ => Err(CoreError::Validation(format!(
            "unknown plugin RPC method: {method}"
        ))),
    }
}

/// Main-window workspace editing is a trusted shell operation. It deliberately
/// has no plugin or project identity parameters; third-party webviews use the
/// session-bound `plugin_rpc` surface above.
#[tauri::command]
#[allow(clippy::too_many_arguments)]
async fn trusted_module_rpc(
    app: tauri::AppHandle,
    state: tauri::State<'_, SharedCore>,
    plugins: tauri::State<'_, SharedPluginHost>,
    settings: tauri::State<'_, SharedSettings>,
    ai_runtime: tauri::State<'_, ai::SharedAiRuntime>,
    plugin_id: Option<String>,
    method: String,
    payload: serde_json::Value,
    request_id: Option<String>,
) -> Result<serde_json::Value, String> {
    let project_id = current_info(state.inner())?
        .map(|info| info.root)
        .ok_or_else(|| "project is not open".to_string())?;
    let event_method = method.clone();
    if method.starts_with("ai.request.") {
        let plugin_id =
            plugin_id.ok_or_else(|| "bundled AI requests require plugin identity".to_string())?;
        let granted_capabilities = {
            let mut host = plugins
                .lock()
                .map_err(|_| "plugin host lock poisoned".to_string())?;
            host.authorize_bundled(&plugin_id, &project_id, &method, payload.clone())
                .map_err(|error| error.to_string())?;
            host.ensure_bundled_session(&plugin_id, &project_id)
                .map_err(|error| error.to_string())?
                .grants
        };
        return dispatch_host_rpc(
            plugins.inner(),
            &plugin_id,
            &project_id,
            &method,
            payload,
            AiBrokerContext {
                app: Some(app),
                core: Some(state.inner().clone()),
                settings: Some(settings.inner().clone()),
                ai_runtime: ai_runtime.inner().clone(),
                session_id: format!("bundled:{plugin_id}"),
                caller: daena_ai::AiCaller::authorized_plugin(
                    plugin_id.clone(),
                    project_id.clone(),
                    granted_capabilities.into_iter().collect(),
                    vec![format!("project:{project_id}")],
                    0,
                    "pending",
                ),
            },
        );
    }
    let record_owner_entity_types = if method.starts_with("record.") {
        let record_plugin_id = plugin_id
            .as_deref()
            .ok_or_else(|| "module record requests require plugin identity".to_string())?;
        let collection = payload
            .get("collection")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| "record collection is required".to_string())?;
        let mut host = plugins
            .lock()
            .map_err(|_| "plugin host lock poisoned".to_string())?;
        host.authorize_bundled(record_plugin_id, &project_id, &method, payload.clone())
            .map_err(|error| error.to_string())?;
        host.record_owner_entity_types(&project_id, record_plugin_id, collection)
    } else {
        None
    };
    let result = with_core(state, move |core| {
        dispatch_module_rpc(
            core,
            plugin_id.as_deref(),
            record_owner_entity_types,
            &method,
            payload,
            request_id.as_deref(),
        )
    })
    .await?;
    publish_core_mutation_event(plugins.inner(), &project_id, &event_method, &result)?;
    Ok(result)
}

fn publish_core_mutation_event(
    plugins: &SharedPluginHost,
    project_id: &str,
    method: &str,
    result: &serde_json::Value,
) -> Result<(), String> {
    if !matches!(
        method,
        "entity.create"
            | "entity.update"
            | "entity.delete"
            | "document.save"
            | "field.set"
            | "relationship.create"
            | "relationship.update"
            | "relationship.delete"
            | "asset.register"
            | "asset.update"
            | "asset.delete"
    ) {
        return Ok(());
    }
    plugins
        .lock()
        .map_err(|_| "plugin host lock poisoned".to_string())?
        .publish_core_event(
            project_id,
            "daena.core/entity-changed",
            1,
            serde_json::json!({"method": method, "result": result}),
        )
        .map_err(|error| error.to_string())?;
    Ok(())
}

#[tauri::command]
async fn project_open(
    app: tauri::AppHandle,
    state: tauri::State<'_, SharedCore>,
    jobs: tauri::State<'_, SharedPhysicalJobs>,
    plugins: tauri::State<'_, SharedPluginHost>,
    ai_runtime: tauri::State<'_, ai::SharedAiRuntime>,
    watcher: tauri::State<'_, SharedProjectWatcher>,
    path: String,
) -> Result<(), String> {
    cancel_physical_jobs(jobs.inner())?;
    cancel_external_import_jobs()?;
    flush_project_checkpoint(state.clone(), "project lifecycle transition").await?;
    let plugins = plugins.inner().clone();
    let ai_runtime = ai_runtime.inner().clone();
    let core = state.inner().clone();
    let request_runtime = ai_runtime.clone();
    let sync_plugins = plugins.clone();
    let result = with_core(state, move |core| {
        if let Some(previous_project) = core.info().map(|info| info.root) {
            cancel_ai_requests_for(&plugins, &request_runtime, &previous_project, None)
                .map_err(CoreError::Conflict)?;
            plugins
                .lock()
                .map_err(|_| CoreError::Conflict("plugin host lock poisoned".into()))?
                .deactivate_project(&previous_project);
        }
        core.open_without_flush(trusted_shell(), path)?;
        Ok(())
    })
    .await;
    if result.is_ok() {
        begin_current_physical_session(jobs.inner(), &core)?;
        sync_project_usage_and_wait(core.clone(), sync_plugins).await?;
        flush_project_checkpoint_for_shared_core(core.clone(), "project startup synchronization")
            .await?;
        schedule_ai_index_refresh(&core, &ai_runtime);
        start_project_watcher(&app, &core, watcher.inner())?;
    }
    result
}

#[tauri::command]
async fn project_open_directory(
    app: tauri::AppHandle,
    state: tauri::State<'_, SharedCore>,
    jobs: tauri::State<'_, SharedPhysicalJobs>,
    plugins: tauri::State<'_, SharedPluginHost>,
    ai_runtime: tauri::State<'_, ai::SharedAiRuntime>,
    watcher: tauri::State<'_, SharedProjectWatcher>,
    path: String,
) -> Result<ProjectInfo, String> {
    cancel_physical_jobs(jobs.inner())?;
    cancel_external_import_jobs()?;
    flush_project_checkpoint(state.clone(), "project lifecycle transition").await?;
    let plugins = plugins.inner().clone();
    let ai_runtime = ai_runtime.inner().clone();
    let core = state.inner().clone();
    let request_runtime = ai_runtime.clone();
    let sync_plugins = plugins.clone();
    let result = with_core(state, move |core| {
        if let Some(previous_project) = core.info().map(|info| info.root) {
            cancel_ai_requests_for(&plugins, &request_runtime, &previous_project, None)
                .map_err(CoreError::Conflict)?;
            plugins
                .lock()
                .map_err(|_| CoreError::Conflict("plugin host lock poisoned".into()))?
                .deactivate_project(&previous_project);
        }
        let info = core.open_directory_without_flush(trusted_shell(), path)?;
        Ok(info)
    })
    .await;
    if result.is_ok() {
        begin_current_physical_session(jobs.inner(), &core)?;
        sync_project_usage_and_wait(core.clone(), sync_plugins).await?;
        flush_project_checkpoint_for_shared_core(core.clone(), "project startup synchronization")
            .await?;
        schedule_ai_index_refresh(&core, &ai_runtime);
        start_project_watcher(&app, &core, watcher.inner())?;
    }
    result
}

#[tauri::command]
async fn project_new(
    app: tauri::AppHandle,
    state: tauri::State<'_, SharedCore>,
    jobs: tauri::State<'_, SharedPhysicalJobs>,
    plugins: tauri::State<'_, SharedPluginHost>,
    ai_runtime: tauri::State<'_, ai::SharedAiRuntime>,
    watcher: tauri::State<'_, SharedProjectWatcher>,
    path: String,
) -> Result<ProjectInfo, String> {
    cancel_physical_jobs(jobs.inner())?;
    cancel_external_import_jobs()?;
    flush_project_checkpoint(state.clone(), "project lifecycle transition").await?;
    let plugins = plugins.inner().clone();
    let ai_runtime = ai_runtime.inner().clone();
    let core = state.inner().clone();
    let request_runtime = ai_runtime.clone();
    let sync_plugins = plugins.clone();
    let result = with_core(state, move |core| {
        if let Some(previous_project) = core.info().map(|info| info.root) {
            cancel_ai_requests_for(&plugins, &request_runtime, &previous_project, None)
                .map_err(CoreError::Conflict)?;
            plugins
                .lock()
                .map_err(|_| CoreError::Conflict("plugin host lock poisoned".into()))?
                .deactivate_project(&previous_project);
        }
        let info = core.open_directory_without_flush(trusted_shell(), path)?;
        Ok(info)
    })
    .await;
    if result.is_ok() {
        begin_current_physical_session(jobs.inner(), &core)?;
        sync_project_usage_and_wait(core.clone(), sync_plugins).await?;
        flush_project_checkpoint_for_shared_core(core.clone(), "project startup synchronization")
            .await?;
        schedule_ai_index_refresh(&core, &ai_runtime);
        start_project_watcher(&app, &core, watcher.inner())?;
    }
    result
}

#[tauri::command]
async fn project_close(
    app: tauri::AppHandle,
    state: tauri::State<'_, SharedCore>,
    jobs: tauri::State<'_, SharedPhysicalJobs>,
    plugins: tauri::State<'_, SharedPluginHost>,
    ai_runtime: tauri::State<'_, ai::SharedAiRuntime>,
    watcher: tauri::State<'_, SharedProjectWatcher>,
) -> Result<(), String> {
    cancel_physical_jobs(jobs.inner())?;
    cancel_external_import_jobs()?;
    let plugins = plugins.inner().clone();
    let ai_runtime = ai_runtime.inner().clone();
    let app = app.clone();
    let core = state.inner().clone();
    let jobs = jobs.inner().clone();
    let watcher = watcher.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        close_project_for_app(&app, &core, &jobs, &plugins, &ai_runtime, &watcher)
    })
    .await
    .map_err(|error| format!("project close worker failed: {error}"))?
}

fn close_project_for_app(
    app: &tauri::AppHandle,
    core: &SharedCore,
    jobs: &SharedPhysicalJobs,
    plugins: &SharedPluginHost,
    ai_runtime: &ai::SharedAiRuntime,
    watcher: &SharedProjectWatcher,
) -> Result<(), String> {
    cancel_physical_jobs(jobs)?;
    cancel_external_import_jobs()?;
    stop_project_watcher(watcher)?;
    flush_checkpoint_for_shared_core(core, "project lifecycle transition")?;
    let session = current_session(core)?;
    let mut service = session
        .core
        .lock()
        .map_err(|_| "core lock poisoned".to_string())?;
    let Some(project_id) = service.info().map(|info| info.root) else {
        return Ok(());
    };
    service
        .close_without_flush(trusted_shell())
        .map_err(|error| error.to_string())?;
    ai::detach_project_index(ai_runtime);
    let plugin_ids = {
        let mut host = plugins
            .lock()
            .map_err(|_| "plugin host lock poisoned".to_string())?;
        for request_id in host.ai_request_ids_for(&project_id, None) {
            let _ = ai::cancel_ai_request(ai_runtime, &request_id);
            let _ = ai::remove_ai_citations(ai_runtime, &request_id);
            host.remove_ai_request(&request_id);
        }
        host.deactivate_project(&project_id);
        host.catalog
            .list()
            .map(|entry| entry.manifest.id.clone())
            .collect::<Vec<_>>()
    };
    drop(service);
    for plugin_id in plugin_ids {
        close_plugin_webview(app, &plugin_id);
    }
    Ok(())
}

#[tauri::command]
async fn project_info(state: tauri::State<'_, SharedCore>) -> Result<Option<ProjectInfo>, String> {
    with_read_project(state, |project| Ok(project.info())).await
}

#[tauri::command]
async fn project_import_checkpoint(
    state: tauri::State<'_, SharedCore>,
) -> Result<ExternalChangeReport, String> {
    with_core(state, |core| {
        core.project_mut(trusted_shell())?.import_checkpoint()
    })
    .await
}

#[tauri::command]
async fn project_save_recovery_copy(
    state: tauri::State<'_, SharedCore>,
    entity_id: String,
    body: String,
) -> Result<String, String> {
    with_read_project(state, move |project| {
        project.save_recovery_copy(&entity_id, &body)
    })
    .await
}

#[tauri::command]
fn git_tool_info() -> GitToolInfo {
    ProjectStore::git_tool_info()
}

#[tauri::command]
async fn project_git_status(state: tauri::State<'_, SharedCore>) -> Result<GitStatus, String> {
    with_read_project(state, |project| project.git_status()).await
}

#[tauri::command]
async fn project_git_preflight(
    state: tauri::State<'_, SharedCore>,
) -> Result<GitPreflight, String> {
    flush_project_checkpoint(state.clone(), "git preflight").await?;
    with_read_project(state, |project| project.git_preflight_after_checkpoint()).await
}

#[tauri::command]
async fn project_git_staging_preview(
    state: tauri::State<'_, SharedCore>,
) -> Result<GitPreflight, String> {
    flush_project_checkpoint(state.clone(), "git staging preview").await?;
    with_read_project(state, |project| project.git_preflight_after_checkpoint()).await
}

#[tauri::command]
async fn project_git_init(state: tauri::State<'_, SharedCore>) -> Result<GitStatus, String> {
    with_core(state, |core| core.project(trusted_shell())?.git_init()).await
}

#[tauri::command]
async fn project_git_log(state: tauri::State<'_, SharedCore>) -> Result<Vec<GitLogEntry>, String> {
    with_read_project(state, |project| project.git_log()).await
}

#[tauri::command]
async fn project_git_commit(
    state: tauri::State<'_, SharedCore>,
    message: String,
    paths: Option<Vec<String>>,
) -> Result<GitStatus, String> {
    flush_project_checkpoint(state.clone(), "git commit").await?;
    with_read_project(state, move |project| {
        project.git_commit_after_checkpoint(message, paths)
    })
    .await
}

#[tauri::command]
async fn project_git_super_squash(
    state: tauri::State<'_, SharedCore>,
    message: String,
) -> Result<GitStatus, String> {
    flush_project_checkpoint(state.clone(), "git super squash").await?;
    with_read_project(state, move |project| {
        project.git_super_squash_after_checkpoint(&message)
    })
    .await
}

#[tauri::command]
async fn project_git_show_tree(
    state: tauri::State<'_, SharedCore>,
    hash: String,
) -> Result<Vec<String>, String> {
    with_read_project(state, move |project| project.git_show_tree(&hash)).await
}

#[tauri::command]
async fn project_git_show_message(
    state: tauri::State<'_, SharedCore>,
    hash: String,
) -> Result<String, String> {
    with_read_project(state, move |project| project.git_show_message(&hash)).await
}

#[tauri::command]
async fn project_git_show_changes(
    state: tauri::State<'_, SharedCore>,
    hash: String,
) -> Result<Vec<daena_core::GitChange>, String> {
    with_read_project(state, move |project| project.git_show_changes(&hash)).await
}

#[tauri::command]
async fn project_git_show_diff(
    state: tauri::State<'_, SharedCore>,
    hash: String,
    path: String,
) -> Result<String, String> {
    with_read_project(state, move |project| project.git_show_diff(&hash, &path)).await
}

#[tauri::command]
async fn project_git_worktree_diff(
    state: tauri::State<'_, SharedCore>,
    paths: Vec<String>,
) -> Result<String, String> {
    with_read_project(state, move |project| project.git_worktree_diff(&paths)).await
}

#[tauri::command]
async fn project_git_show_file(
    state: tauri::State<'_, SharedCore>,
    hash: String,
    path: String,
) -> Result<String, String> {
    with_read_project(state, move |project| project.git_show_file(&hash, &path)).await
}

#[tauri::command]
async fn project_git_reset_hard(
    state: tauri::State<'_, SharedCore>,
    hash: String,
) -> Result<GitResetResult, String> {
    with_core(state, move |core| {
        core.project_mut(trusted_shell())?.git_reset_hard(&hash)
    })
    .await
}

#[tauri::command]
async fn project_git_remote_list(
    state: tauri::State<'_, SharedCore>,
) -> Result<Vec<GitRemote>, String> {
    with_read_project(state, |project| project.git_remote_list()).await
}

#[tauri::command]
async fn project_git_remote_add(
    state: tauri::State<'_, SharedCore>,
    name: String,
    url: String,
) -> Result<Vec<GitRemote>, String> {
    with_core(state, move |core| {
        core.project(trusted_shell())?.git_remote_add(&name, &url)
    })
    .await
}

#[tauri::command]
async fn project_git_remote_set_url(
    state: tauri::State<'_, SharedCore>,
    name: String,
    url: String,
) -> Result<Vec<GitRemote>, String> {
    with_core(state, move |core| {
        core.project(trusted_shell())?
            .git_remote_set_url(&name, &url)
    })
    .await
}

#[tauri::command]
async fn project_git_remote_remove(
    state: tauri::State<'_, SharedCore>,
    name: String,
) -> Result<Vec<GitRemote>, String> {
    with_core(state, move |core| {
        core.project(trusted_shell())?.git_remote_remove(&name)
    })
    .await
}

#[tauri::command]
async fn project_git_push(
    state: tauri::State<'_, SharedCore>,
    remote: String,
    branch: Option<String>,
    force_with_lease: bool,
) -> Result<GitStatus, String> {
    with_core(state, move |core| {
        core.project(trusted_shell())?
            .git_push(&remote, branch.as_deref(), force_with_lease)
    })
    .await
}

#[tauri::command]
async fn project_git_restore_from_upstream(
    state: tauri::State<'_, SharedCore>,
) -> Result<GitResetResult, String> {
    with_core(state, |core| {
        core.project_mut(trusted_shell())?
            .git_restore_from_upstream()
    })
    .await
}

#[tauri::command]
async fn open_external_url(url: String) -> Result<(), String> {
    let allowed = url == "https://git-scm.com/downloads" || url.starts_with("https://git-scm.com/");
    if !allowed {
        return Err("external URL is not allowlisted".into());
    }
    tauri_plugin_opener::open_url(url, None::<&str>).map_err(|error| error.to_string())
}

#[tauri::command]
async fn project_open_memory(
    state: tauri::State<'_, SharedCore>,
    jobs: tauri::State<'_, SharedPhysicalJobs>,
    plugins: tauri::State<'_, SharedPluginHost>,
    ai_runtime: tauri::State<'_, ai::SharedAiRuntime>,
    watcher: tauri::State<'_, SharedProjectWatcher>,
) -> Result<(), String> {
    cancel_physical_jobs(jobs.inner())?;
    cancel_external_import_jobs()?;
    stop_project_watcher(watcher.inner())?;
    flush_project_checkpoint(state.clone(), "project lifecycle transition").await?;
    let core = state.inner().clone();
    let plugins = plugins.inner().clone();
    let result = with_core(state, move |core| {
        core.open_memory_without_flush(trusted_shell())?;
        let project = core.project_mut(trusted_shell())?;
        let host = plugins
            .lock()
            .map_err(|_| CoreError::Conflict("plugin host lock poisoned".into()))?;
        let schemas = relationship_metadata_schemas_for_project(project, &host)?;
        project.set_relationship_metadata_schemas(schemas)
    })
    .await;
    if result.is_ok() {
        begin_current_physical_session(jobs.inner(), &core)?;
        schedule_ai_index_refresh(&core, ai_runtime.inner());
    }
    result
}

#[tauri::command]
async fn project_open_default(
    app: tauri::AppHandle,
    state: tauri::State<'_, SharedCore>,
    jobs: tauri::State<'_, SharedPhysicalJobs>,
    plugins: tauri::State<'_, SharedPluginHost>,
    ai_runtime: tauri::State<'_, ai::SharedAiRuntime>,
    watcher: tauri::State<'_, SharedProjectWatcher>,
) -> Result<(), String> {
    cancel_physical_jobs(jobs.inner())?;
    cancel_external_import_jobs()?;
    let directory = app
        .path()
        .app_data_dir()
        .map_err(|error| error.to_string())?;
    let plugins = plugins.inner().clone();
    let core = state.inner().clone();
    let ai_runtime = ai_runtime.inner().clone();
    let sync_plugins = plugins.clone();
    flush_project_checkpoint(state.clone(), "project lifecycle transition").await?;
    let result = with_core(state, move |core| {
        std::fs::create_dir_all(&directory).map_err(|error| CoreError::Io {
            operation: "create app data directory",
            source: error,
        })?;
        let project_directory = directory.join("Daena Archive");
        if let Some(previous_project) = core.info().map(|info| info.root) {
            plugins
                .lock()
                .map_err(|_| CoreError::Conflict("plugin host lock poisoned".into()))?
                .deactivate_project(&previous_project);
        }
        core.open_directory_without_flush(trusted_shell(), project_directory)?;
        Ok(())
    })
    .await;
    if result.is_ok() {
        begin_current_physical_session(jobs.inner(), &core)?;
        sync_project_usage_and_wait(core.clone(), sync_plugins).await?;
        flush_project_checkpoint_for_shared_core(core.clone(), "project startup synchronization")
            .await?;
        schedule_ai_index_refresh(&core, &ai_runtime);
        start_project_watcher(&app, &core, watcher.inner())?;
    }
    result
}

#[tauri::command]
async fn project_create_entity(
    state: tauri::State<'_, SharedCore>,
    input: CreateEntity,
    request_id: Option<String>,
) -> Result<Entity, String> {
    with_core(state, move |core| {
        core.project(trusted_shell())?
            .create_entity_with_request(input, request_id.as_deref())
    })
    .await
}

#[tauri::command]
async fn project_create_map(
    state: tauri::State<'_, SharedCore>,
    name: String,
) -> Result<Entity, String> {
    with_core(state, move |core| {
        core.project(trusted_shell())?.create_map(name)
    })
    .await
}

#[tauri::command]
async fn project_list_entities(state: tauri::State<'_, SharedCore>) -> Result<Vec<Entity>, String> {
    with_read_project(state, |project| project.list_entities()).await
}

#[tauri::command]
async fn project_search(
    state: tauri::State<'_, SharedCore>,
    query: String,
) -> Result<Vec<Entity>, String> {
    with_read_project(state, move |project| project.search(query)).await
}

#[tauri::command]
async fn project_update_entity(
    state: tauri::State<'_, SharedCore>,
    id: String,
    name: Option<String>,
    entity_type: Option<String>,
    expected_revision: Option<String>,
    request_id: Option<String>,
) -> Result<Entity, String> {
    with_core(state, move |core| {
        core.project(trusted_shell())?.update_entity_with_options(
            id,
            name,
            entity_type,
            expected_revision.as_deref(),
            request_id.as_deref(),
        )
    })
    .await
}

#[tauri::command]
async fn project_delete_entity(
    state: tauri::State<'_, SharedCore>,
    id: String,
    expected_revision: Option<String>,
    request_id: Option<String>,
) -> Result<(), String> {
    with_core(state, move |core| {
        core.project(trusted_shell())?.delete_entity_with_options(
            id,
            expected_revision.as_deref(),
            request_id.as_deref(),
        )
    })
    .await
}

#[tauri::command]
async fn project_save_document(
    state: tauri::State<'_, SharedCore>,
    input: SaveDocument,
    expected_revision: Option<String>,
    request_id: Option<String>,
) -> Result<(), String> {
    with_core(state, move |core| {
        core.project(trusted_shell())?.save_document_with_options(
            input,
            expected_revision.as_deref(),
            request_id.as_deref(),
        )
    })
    .await
}

#[tauri::command]
async fn project_save_entry(
    state: tauri::State<'_, SharedCore>,
    input: SaveEntry,
    expected_revision: Option<String>,
    request_id: Option<String>,
) -> Result<(), String> {
    with_core(state, move |core| {
        core.project(trusted_shell())?.save_entry_with_options(
            input,
            expected_revision.as_deref(),
            request_id.as_deref(),
        )
    })
    .await
}

#[tauri::command]
async fn project_list_documents(
    state: tauri::State<'_, SharedCore>,
    entity_id: String,
) -> Result<Vec<daena_core::Document>, String> {
    with_read_project(state, move |project| project.list_documents(entity_id)).await
}

#[tauri::command]
async fn project_set_field(
    state: tauri::State<'_, SharedCore>,
    field: FieldValue,
    request_id: Option<String>,
) -> Result<(), String> {
    with_core(state, move |core| {
        core.project(trusted_shell())?
            .set_field_with_request(field, request_id.as_deref())
    })
    .await
}

#[tauri::command]
async fn project_list_fields(
    state: tauri::State<'_, SharedCore>,
    entity_id: String,
) -> Result<Vec<FieldValue>, String> {
    with_read_project(state, move |project| project.list_fields(entity_id)).await
}

#[tauri::command]
async fn project_create_relationship(
    state: tauri::State<'_, SharedCore>,
    input: RelationshipInput,
    expected_revision: Option<String>,
    request_id: Option<String>,
) -> Result<Relationship, String> {
    with_core(state, move |core| {
        core.project(trusted_shell())?
            .create_relationship_with_options(
                input,
                expected_revision.as_deref(),
                request_id.as_deref(),
            )
    })
    .await
}

#[tauri::command]
async fn project_update_relationship(
    state: tauri::State<'_, SharedCore>,
    input: daena_core::RelationshipUpdate,
    expected_revision: Option<String>,
    request_id: Option<String>,
) -> Result<Relationship, String> {
    with_core(state, move |core| {
        core.project(trusted_shell())?
            .update_relationship_with_options(
                input,
                expected_revision.as_deref(),
                request_id.as_deref(),
            )
    })
    .await
}

#[tauri::command]
async fn project_list_relationships(
    state: tauri::State<'_, SharedCore>,
    entity_id: String,
) -> Result<Vec<Relationship>, String> {
    with_read_project(state, move |project| project.list_relationships(entity_id)).await
}

#[tauri::command]
async fn project_list_map_locations(
    state: tauri::State<'_, SharedCore>,
    entity_id: String,
) -> Result<Vec<daena_core::maps::LocationReference>, String> {
    with_read_project(state, move |project| project.map_locations(entity_id)).await
}

#[tauri::command]
async fn project_upsert_map_location(
    state: tauri::State<'_, SharedCore>,
    entity_id: String,
    location: daena_core::maps::LocationReference,
    request_id: Option<String>,
) -> Result<(), String> {
    with_core(state, move |core| {
        core.project(trusted_shell())?.upsert_map_location(
            entity_id,
            location,
            request_id.as_deref(),
        )
    })
    .await
}

#[tauri::command]
async fn project_unlink_map_location(
    state: tauri::State<'_, SharedCore>,
    entity_id: String,
    location_id: String,
    request_id: Option<String>,
) -> Result<(), String> {
    with_core(state, move |core| {
        core.project(trusted_shell())?.unlink_map_location(
            entity_id,
            location_id,
            request_id.as_deref(),
        )
    })
    .await
}

/// Versioned host handoff for the public `daena.maps/navigation@1` service.
/// The service resolves canonical links before asking the shell to mount Maps;
/// provider availability remains a concern of the child webview.
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct MapsNavigationRequest {
    operation: String,
    #[serde(default)]
    map_entity_id: Option<String>,
    #[serde(default)]
    entity_id: Option<String>,
    #[serde(default)]
    link_id: Option<String>,
    #[serde(default)]
    date: Option<serde_json::Value>,
    #[serde(default)]
    entity_ids: Option<Vec<String>>,
}

/// The result of resolving a navigation operation. `emit` carries the shell
/// handoff (`maps-navigation` Tauri event) when the shell must mount or
/// re-focus the map editor; `result` is the service response. An unresolved
/// link emits the handoff (so the map opens and the notice can surface) while
/// still returning a typed `link-unresolved` error.
struct MapsNavigationOutcome {
    emit: Option<(String, Option<serde_json::Value>)>,
    result: Result<serde_json::Value, String>,
}

fn resolve_maps_navigation(
    core: &mut CoreService,
    request: &MapsNavigationRequest,
) -> Result<MapsNavigationOutcome, String> {
    let project = core
        .project(trusted_shell())
        .map_err(|error| error.to_string())?;
    let outcome = match request.operation.as_str() {
        "openMap" => {
            let id = request
                .map_entity_id
                .clone()
                .ok_or_else(|| "map-unavailable: mapEntityId is required".to_string())?;
            let exists = project
                .list_entities()
                .map_err(|error| error.to_string())?
                .into_iter()
                .any(|entity| {
                    entity.id == id
                        && entity.entity_type.as_deref() == Some(daena_core::maps::MAP_ENTITY_TYPE)
                });
            if !exists {
                return Err("map-unavailable".into());
            }
            let link = request.link_id.clone().map(serde_json::Value::String);
            MapsNavigationOutcome {
                emit: Some((id.clone(), link)),
                result: Ok(serde_json::json!({
                    "mapEntityId": id,
                    "linkId": request.link_id,
                })),
            }
        }
        "focusEntity" => {
            let id = request
                .entity_id
                .clone()
                .ok_or_else(|| "not-on-map: entityId is required".to_string())?;
            let locations = project
                .map_locations_for_entity(id)
                .map_err(|error| error.to_string())?;
            let filtered: Vec<serde_json::Value> = locations
                .into_iter()
                .filter(|location| {
                    request.map_entity_id.as_deref().is_none_or(|map| {
                        map == location["mapEntityId"].as_str().unwrap_or_default()
                    })
                })
                .collect();
            if let Some(link_id) = request.link_id.as_deref() {
                let Some(location) = filtered
                    .iter()
                    .find(|location| location["id"].as_str() == Some(link_id))
                else {
                    return Err("link-unresolved: location no longer exists on the entity".into());
                };
                let map_id = location["mapEntityId"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string();
                let unresolved = location["anchorKind"].as_str() == Some("provider-feature")
                    && location["resolution"].as_str() == Some("unresolved");
                let result = if unresolved {
                    Err("link-unresolved: the map feature was removed or renumbered".into())
                } else {
                    Ok(serde_json::json!({
                        "mapEntityId": map_id,
                        "linkId": link_id,
                    }))
                };
                MapsNavigationOutcome {
                    emit: Some((map_id, Some(serde_json::Value::String(link_id.to_string())))),
                    result,
                }
            } else if filtered.is_empty() {
                return Err("not-on-map".into());
            } else if filtered.len() > 1 {
                MapsNavigationOutcome {
                    emit: None,
                    result: Ok(serde_json::json!({
                        "status": "multiple-links",
                        "locations": filtered,
                    })),
                }
            } else {
                let location = &filtered[0];
                let map_id = location["mapEntityId"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string();
                let link_id = location["id"].as_str().unwrap_or_default().to_string();
                let unresolved = location["anchorKind"].as_str() == Some("provider-feature")
                    && location["resolution"].as_str() == Some("unresolved");
                let result = if unresolved {
                    Err("link-unresolved: the map feature was removed or renumbered".into())
                } else {
                    Ok(serde_json::json!({
                        "mapEntityId": map_id,
                        "linkId": link_id,
                    }))
                };
                MapsNavigationOutcome {
                    emit: Some((map_id, Some(serde_json::Value::String(link_id)))),
                    result,
                }
            }
        }
        "listLocations" => {
            let id = request
                .entity_id
                .clone()
                .ok_or_else(|| "entityId is required".to_string())?;
            let locations = project
                .map_locations(id)
                .map_err(|error| error.to_string())?;
            MapsNavigationOutcome {
                emit: None,
                result: serde_json::to_value(locations)
                    .map_err(|error| format!("serialize locations: {error}")),
            }
        }
        "setDate" => {
            let date = request
                .date
                .clone()
                .ok_or_else(|| "setDate requires a date payload".to_string())?;
            let era_ok = date
                .get("era")
                .and_then(serde_json::Value::as_str)
                .is_none_or(|era| era == "BCE" || era == "CE");
            if !date.is_object() || date.get("year").is_none() || !era_ok {
                return Err("validation: invalid date payload".into());
            }
            MapsNavigationOutcome {
                emit: None,
                result: Ok(serde_json::json!({ "accepted": true, "date": date })),
            }
        }
        "showResults" => {
            let ids = request
                .entity_ids
                .clone()
                .ok_or_else(|| "showResults requires entityIds".to_string())?;
            let mut rows: Vec<serde_json::Value> = Vec::new();
            for entity_id in &ids {
                let locations = project
                    .map_locations_for_entity(entity_id.clone())
                    .map_err(|error| error.to_string())?;
                rows.extend(locations.into_iter().filter(|location| {
                    request.map_entity_id.as_deref().is_none_or(|map| {
                        map == location["mapEntityId"].as_str().unwrap_or_default()
                    })
                }));
            }
            let map_id = request
                .map_entity_id
                .clone()
                .or_else(|| {
                    rows.first()
                        .and_then(|row| row["mapEntityId"].as_str())
                        .map(String::from)
                })
                .ok_or_else(|| "not-on-map: no locations for the requested entities".to_string())?;
            rows.retain(|row| row["mapEntityId"].as_str() == Some(map_id.as_str()));
            if rows.is_empty() {
                return Err("not-on-map: no locations for the requested entities".into());
            }
            MapsNavigationOutcome {
                emit: Some((map_id.clone(), None)),
                result: Ok(serde_json::json!({ "mapEntityId": map_id, "locations": rows })),
            }
        }
        _ => {
            return Err("unsupported daena.maps/navigation@1 operation".into());
        }
    };
    Ok(outcome)
}

/// Plugin RPC envelopes carry correlation-only `requestId`s (the FMG bridge
/// uses `maps-fmg-N`). Transaction receipts are UUID-keyed, so only pass a
/// request id into core mutations when it is a real UUID; the response echo
/// keeps the original envelope id.
fn sanitize_mutation_request_id(request_id: &str) -> Option<&str> {
    request_id
        .parse::<uuid::Uuid>()
        .is_ok()
        .then_some(request_id)
}

#[tauri::command]
#[allow(clippy::too_many_arguments)]
async fn maps_navigation(
    app: tauri::AppHandle,
    state: tauri::State<'_, SharedCore>,
    operation: String,
    map_entity_id: Option<String>,
    entity_id: Option<String>,
    link_id: Option<String>,
    date: Option<serde_json::Value>,
    entity_ids: Option<Vec<String>>,
) -> Result<serde_json::Value, String> {
    let request = MapsNavigationRequest {
        operation,
        map_entity_id,
        entity_id,
        link_id,
        date,
        entity_ids,
    };
    let outcome = with_core(state, move |core| {
        resolve_maps_navigation(core, &request).map_err(daena_core::CoreError::Validation)
    })
    .await?;
    if let Some((map_id, link)) = &outcome.emit {
        app.emit(
            "maps-navigation",
            serde_json::json!({
                "mapEntityId": map_id,
                "linkId": link.as_ref().and_then(|value| value.as_str()),
            }),
        )
        .map_err(|error| error.to_string())?;
    }
    outcome.result
}

/// Native handler for the public `daena.maps/navigation@1` service. Registered
/// on the plugin host before project activation so the manifest-declared WASM
/// stub is skipped; consumers such as Lore and Timeline reach the same
/// resolution and shell handoff as the `maps_navigation` Tauri command.
fn maps_navigation_service_handler(core: SharedCore) -> daena_plugin_host::ServiceHandler {
    use daena_plugin_host::{HostError, ServiceRequest};
    std::sync::Arc::new(move |request: ServiceRequest| {
        let app = APP_HANDLE
            .get()
            .ok_or_else(|| HostError("map editor is not open".into()))?;
        let request: MapsNavigationRequest = serde_json::from_value(request.payload.clone())
            .map_err(|error| {
                HostError(format!("invalid daena.maps/navigation@1 payload: {error}"))
            })?;
        let session = current_session(&core).map_err(HostError)?;
        let mut core = session
            .core
            .lock()
            .map_err(|_| HostError("core lock poisoned".into()))?;
        let outcome = resolve_maps_navigation(&mut core, &request).map_err(HostError)?;
        if let Some((map_id, link)) = &outcome.emit {
            let _ = app.emit(
                "maps-navigation",
                serde_json::json!({
                    "mapEntityId": map_id,
                    "linkId": link.as_ref().and_then(|value| value.as_str()),
                }),
            );
        }
        outcome.result.map_err(HostError)
    })
}

/// Asks the open Maps-compatible host-surface webview to save. There is no shell-to-webview
/// RPC channel; the child exposes its save path on `window.daenaMapProvider`
/// and the host evaluates a fixed literal to trigger it.
#[tauri::command]
async fn maps_editor_save(app: tauri::AppHandle, plugin_id: Option<String>) -> Result<(), String> {
    let label = plugin_window_label(plugin_id.as_deref().unwrap_or("daena.maps"));
    let webview = app
        .get_webview(&label)
        .ok_or_else(|| "the map editor is not open".to_string())?;
    webview
        .eval("window.daenaMapProvider && window.daenaMapProvider.save && window.daenaMapProvider.save()")
        .map_err(|error| format!("request map editor save: {error}"))
}

/// Asks the open Maps-compatible host-surface webview to capture the current selection. The
/// capture itself is asynchronous in the child; the anchor is delivered back
/// on `daena.maps/selection@1`, forwarded to the shell as `maps-selection`.
#[tauri::command]
async fn maps_editor_capture_anchor(
    app: tauri::AppHandle,
    plugin_id: Option<String>,
) -> Result<(), String> {
    let label = plugin_window_label(plugin_id.as_deref().unwrap_or("daena.maps"));
    let webview = app
        .get_webview(&label)
        .ok_or_else(|| "the map editor is not open".to_string())?;
    webview
        .eval("window.daenaMapProvider && window.daenaMapProvider.captureSelection && window.daenaMapProvider.captureSelection()")
        .map_err(|error| format!("request map editor capture: {error}"))
}

/// Puts the open Maps-compatible host surface into one-shot pick mode. The next map click publishes
/// `daena.maps/state@1` with status `pick-complete` (and the anchor in detail).
#[tauri::command]
async fn maps_editor_start_pick(
    app: tauri::AppHandle,
    plugin_id: Option<String>,
) -> Result<(), String> {
    let label = plugin_window_label(plugin_id.as_deref().unwrap_or("daena.maps"));
    let webview = app
        .get_webview(&label)
        .ok_or_else(|| "the map editor is not open".to_string())?;
    webview
        .eval("window.daenaMapProvider && window.daenaMapProvider.startPick && window.daenaMapProvider.startPick()")
        .map_err(|error| format!("request map editor pick: {error}"))
}

/// Pushes a semantic overlay frame into the open Maps-compatible host surface. The
/// frame is derived state (projection rows); the child never persists it.
#[tauri::command]
async fn maps_editor_set_overlay(
    app: tauri::AppHandle,
    plugin_id: Option<String>,
    frame: serde_json::Value,
) -> Result<(), String> {
    let label = plugin_window_label(plugin_id.as_deref().unwrap_or("daena.maps"));
    let webview = app
        .get_webview(&label)
        .ok_or_else(|| "the map editor is not open".to_string())?;
    let frame = serde_json::to_string(&frame)
        .map_err(|error| format!("serialize overlay frame: {error}"))?;
    webview
        .eval(format!(
            "window.daenaMapProvider && window.daenaMapProvider.setSemanticOverlay && window.daenaMapProvider.setSemanticOverlay({frame})"
        ).as_str())
        .map_err(|error| format!("request map editor overlay: {error}"))
}

/// Pushes a display date into the open Maps-compatible host surface so the overlay
/// filters locations by validity. `null` clears the temporal filter.
#[tauri::command]
async fn maps_editor_set_date(
    app: tauri::AppHandle,
    plugin_id: Option<String>,
    date: Option<serde_json::Value>,
) -> Result<(), String> {
    let label = plugin_window_label(plugin_id.as_deref().unwrap_or("daena.maps"));
    let webview = app
        .get_webview(&label)
        .ok_or_else(|| "the map editor is not open".to_string())?;
    let date =
        serde_json::to_string(&date).map_err(|error| format!("serialize overlay date: {error}"))?;
    webview
        .eval(format!(
            "window.daenaMapProvider && window.daenaMapProvider.setDate && window.daenaMapProvider.setDate({date})"
        ).as_str())
        .map_err(|error| format!("request map editor date: {error}"))
}

/// Asks the open Maps-compatible host surface to focus a linked location by ID.
#[tauri::command]
async fn maps_editor_focus_link(
    app: tauri::AppHandle,
    plugin_id: Option<String>,
    link_id: String,
) -> Result<(), String> {
    let label = plugin_window_label(plugin_id.as_deref().unwrap_or("daena.maps"));
    let webview = app
        .get_webview(&label)
        .ok_or_else(|| "the map editor is not open".to_string())?;
    let link_id =
        serde_json::to_string(&link_id).map_err(|error| format!("serialize link id: {error}"))?;
    webview
        .eval(format!(
            "window.daenaMapProvider && window.daenaMapProvider.focusByLink && window.daenaMapProvider.focusByLink({link_id})"
        ).as_str())
        .map_err(|error| format!("request map editor focus: {error}"))
}

#[tauri::command]
async fn maps_recovery_list(
    state: tauri::State<'_, SharedCore>,
    entity_id: String,
) -> Result<Vec<daena_core::maps::MapRecoveryCopy>, String> {
    with_core(state, move |core| {
        core.project(trusted_shell())?
            .list_map_recovery_copies(&entity_id)
    })
    .await
}

#[tauri::command]
async fn maps_recovery_restore(
    state: tauri::State<'_, SharedCore>,
    entity_id: String,
    file_name: String,
    request_id: Option<String>,
) -> Result<Asset, String> {
    with_core(state, move |core| {
        core.project(trusted_shell())?.restore_map_recovery_copy(
            &entity_id,
            &file_name,
            request_id.as_deref(),
        )
    })
    .await
}

#[tauri::command]
async fn project_register_asset(
    state: tauri::State<'_, SharedCore>,
    input: AssetInput,
    expected_revision: Option<String>,
    request_id: Option<String>,
) -> Result<Asset, String> {
    with_core(state, move |core| {
        core.project(trusted_shell())?.register_asset_with_options(
            input,
            expected_revision.as_deref(),
            request_id.as_deref(),
        )
    })
    .await
}

#[tauri::command]
async fn project_register_asset_file(
    state: tauri::State<'_, SharedCore>,
    input: AssetFileInput,
    expected_revision: Option<String>,
    request_id: Option<String>,
) -> Result<Asset, String> {
    with_core(state, move |core| {
        core.project(trusted_shell())?
            .register_asset_file_with_options(
                input,
                expected_revision.as_deref(),
                request_id.as_deref(),
            )
    })
    .await
}

#[tauri::command]
async fn project_list_assets(
    state: tauri::State<'_, SharedCore>,
    entity_id: String,
) -> Result<Vec<Asset>, String> {
    with_read_project(state, move |project| project.list_assets(entity_id)).await
}

#[tauri::command]
async fn project_import_image_map_file(
    state: tauri::State<'_, SharedCore>,
    source_path: String,
) -> Result<daena_core::ImportedImageMap, String> {
    let path = PathBuf::from(&source_path);
    let filename = path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or("image filename is invalid")?
        .to_string();
    let name = path
        .file_stem()
        .and_then(|name| name.to_str())
        .unwrap_or("Image map")
        .to_string();
    let mime = match path
        .extension()
        .and_then(|extension| extension.to_str())
        .map(str::to_ascii_lowercase)
        .as_deref()
    {
        Some("png") => "image/png",
        Some("jpg") | Some("jpeg") => "image/jpeg",
        Some("svg") => "image/svg+xml",
        _ => return Err("Choose a PNG, JPEG, or SVG image".into()),
    }
    .to_string();
    let bytes = std::fs::read(&path).map_err(|error| format!("read image: {error}"))?;
    with_core(state, move |core| {
        core.project(trusted_shell())?
            .import_image_map(name, bytes, mime, filename, None)
    })
    .await
}

#[tauri::command]
async fn project_accept_vector_map(
    state: tauri::State<'_, SharedCore>,
    name: String,
    candidate_json: String,
    generation: serde_json::Value,
    request_id: Option<String>,
) -> Result<daena_core::AcceptedVectorMap, String> {
    let bytes = candidate_json.into_bytes();
    with_core(state, move |core| {
        core.project(trusted_shell())?.accept_vector_map(
            name,
            bytes,
            generation,
            request_id.as_deref(),
        )
    })
    .await
}

#[tauri::command]
async fn project_physical_generate(
    state: tauri::State<'_, SharedCore>,
    jobs: tauri::State<'_, SharedPhysicalJobs>,
    input: PhysicalGenerationInput,
    request_id: Option<String>,
) -> Result<PhysicalJobStatus, String> {
    let project_id = current_info(state.inner())?
        .ok_or_else(|| "open a project before generating a physical map".to_string())?
        .root;
    let session_id = {
        let mut manager = jobs
            .lock()
            .map_err(|_| "physical job state is unavailable".to_string())?;
        manager.ensure_session(&project_id)
    };
    let job_id = uuid::Uuid::new_v4().to_string();
    let request_id = request_id.unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
    uuid::Uuid::parse_str(&request_id)
        .map_err(|_| "physical generation request ID must be a UUID".to_string())?;
    let evolution_preset = daena_physical::evolution::EvolutionPreset::parse(
        input.evolution_preset.as_deref().unwrap_or("mature"),
    )
    .map_err(|error| error.to_string())?;
    let cancel = Arc::new(AtomicBool::new(false));
    let status = PhysicalJobStatus {
        job_id: job_id.clone(),
        request_id: request_id.clone(),
        state: "running".into(),
        stage: daena_physical::ProgressPhase::BuildingTectonicStructure
            .label()
            .into(),
        completed: 0,
        total: 1,
        error: None,
        error_code: None,
        physical_identity: None,
    };
    jobs.lock()
        .map_err(|_| "physical job state is unavailable".to_string())?
        .jobs
        .insert(
            job_id.clone(),
            PhysicalJob {
                project_id,
                session_id,
                expires_at: Instant::now() + PHYSICAL_JOB_TTL,
                cancel: cancel.clone(),
                status: status.clone(),
                result: None,
            },
        );
    let jobs_for_worker = jobs.inner().clone();
    let worker_job_id = job_id.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let mut progress = PhysicalProgress {
            jobs: jobs_for_worker.clone(),
            job_id: worker_job_id.clone(),
            cancel: cancel.clone(),
        };
        let settings = daena_physical::GenerationSettings {
            width: input.settings.width,
            height: input.settings.height,
            radius_metres: input.settings.radius_metres,
            target_land_fraction_ppm: input.settings.target_land_fraction_ppm,
        };
        let outcome = daena_physical::generate_world_with_evolution(
            settings,
            input.seed,
            input.retry_index,
            daena_physical::evolution::EvolutionSettings {
                preset: evolution_preset,
            },
            &mut progress,
        );
        let mut manager = match jobs_for_worker.lock() {
            Ok(manager) => manager,
            Err(_) => return,
        };
        let Some(job) = manager.jobs.get_mut(&worker_job_id) else {
            return;
        };
        match outcome {
            Ok(world) => {
                let historical_forcing =
                    daena_physical::history::HistoricalForcingParameters::default_for(
                        input.seed,
                        input.retry_index,
                    );
                let generation = serde_json::json!({
                    "id": daena_core::maps::PHYSICAL_GENERATOR_ID,
                    "version": daena_core::maps::PHYSICAL_GENERATOR_VERSION,
                    "seed": input.seed,
                    "retryIndex": input.retry_index,
                    "settings": {
                        "width": input.settings.width,
                        "height": input.settings.height,
                        "radiusMetres": input.settings.radius_metres,
                        "targetLandFractionPpm": input.settings.target_land_fraction_ppm,
                        "referenceWaterInventoryM3": world.report.reference_water_inventory_m3,
                        "plateCount": world.tectonics.settings.plate_count,
                        "continentalPlateCount": world.tectonics.settings.continental_plate_count,
                        "tectonicActivityPpm": world.tectonics.settings.tectonic_activity_ppm,
                        "islandActivityPpm": world.tectonics.settings.island_activity_ppm,
                        "evolutionPreset": evolution_preset.as_str(),
                        "hazardDerivationVersion": daena_physical::hazards::HAZARD_DERIVATION_VERSION,
                        "historicalForcing": historical_forcing_products(historical_forcing),
                    }
                });
                let physical_identity =
                    match daena_core::maps::physical::validate_source(&world.source, &generation) {
                        Ok(validated) => validated.identity,
                        Err(error) => {
                            job.status.state = "failed".into();
                            job.status.stage = daena_physical::ProgressPhase::ValidatingWorld
                                .label()
                                .into();
                            job.status.error = Some(error.to_string());
                            job.status.error_code =
                                Some(daena_core::maps::physical::CODE_INVALID_SOURCE.into());
                            return;
                        }
                    };
                job.status.state = "completed".into();
                job.status.stage = daena_physical::ProgressPhase::ValidatingWorld
                    .label()
                    .into();
                job.status.completed = 1;
                job.status.total = 1;
                job.status.error = None;
                job.status.error_code = None;
                job.status.physical_identity = Some(physical_identity.clone());
                job.result = Some(PhysicalJobResult {
                    source: world.source,
                    generation,
                    physical_identity,
                    derived_geojson: world.derived_geojson,
                    climate: world.climate,
                    evolution: world.evolution,
                    hydrology: world.hydrology,
                });
            }
            Err(daena_physical::PhysicalError::Cancelled) => {
                job.status.state = "cancelled".into();
                job.status.stage = daena_physical::ProgressPhase::ValidatingWorld
                    .label()
                    .into();
                job.status.error = None;
                job.status.error_code = Some(daena_physical::CODE_GENERATOR_CANCELLED.into());
            }
            Err(error) => {
                job.status.state = "failed".into();
                job.status.stage = daena_physical::ProgressPhase::ValidatingWorld
                    .label()
                    .into();
                job.status.error = Some(error.to_string());
                job.status.error_code = Some(error.code().into());
            }
        }
    });
    Ok(status)
}

#[tauri::command]
fn project_physical_status(
    state: tauri::State<'_, SharedCore>,
    jobs: tauri::State<'_, SharedPhysicalJobs>,
    job_id: String,
) -> Result<PhysicalJobStatus, String> {
    let project_id = current_info(state.inner())?
        .ok_or_else(|| "open a project before reading a physical job".to_string())?
        .root;
    let mut manager = jobs
        .lock()
        .map_err(|_| "physical job state is unavailable".to_string())?;
    manager.reap_expired();
    let job = manager
        .jobs
        .get(&job_id)
        .filter(|job| manager.active_session_matches(&project_id, &job.session_id))
        .ok_or("physical job was not found, expired, or belongs to another session")?;
    Ok(job.status.clone())
}

#[tauri::command]
fn project_physical_cancel(
    state: tauri::State<'_, SharedCore>,
    jobs: tauri::State<'_, SharedPhysicalJobs>,
    job_id: String,
) -> Result<PhysicalJobStatus, String> {
    let project_id = current_info(state.inner())?
        .ok_or_else(|| "open a project before cancelling a physical job".to_string())?
        .root;
    let mut manager = jobs
        .lock()
        .map_err(|_| "physical job state is unavailable".to_string())?;
    manager.reap_expired();
    let session_matches = manager
        .jobs
        .get(&job_id)
        .map(|job| manager.active_session_matches(&project_id, &job.session_id))
        .unwrap_or(false);
    if !session_matches {
        return Err("physical job was not found, expired, or belongs to another session".into());
    }
    let job = manager
        .jobs
        .get_mut(&job_id)
        .ok_or_else(|| "physical job was not found or has expired".to_string())?;
    if job.status.state == "running" {
        job.cancel.store(true, Ordering::Relaxed);
        job.status.state = "cancelling".into();
    }
    Ok(job.status.clone())
}

#[tauri::command]
fn project_physical_preview(
    state: tauri::State<'_, SharedCore>,
    jobs: tauri::State<'_, SharedPhysicalJobs>,
    job_id: String,
) -> Result<String, String> {
    let project_id = current_info(state.inner())?
        .ok_or_else(|| "open a project before reading a physical preview".to_string())?
        .root;
    let mut manager = jobs
        .lock()
        .map_err(|_| "physical job state is unavailable".to_string())?;
    manager.reap_expired();
    let job = manager
        .jobs
        .get(&job_id)
        .filter(|job| manager.active_session_matches(&project_id, &job.session_id))
        .ok_or("physical job was not found, expired, or belongs to another session")?;
    job.result
        .as_ref()
        .map(|result| result.derived_geojson.clone())
        .ok_or_else(|| "physical job preview is not ready".to_string())
}

fn physical_climate_products(climate: &daena_physical::climate::ClimateField) -> serde_json::Value {
    serde_json::json!({
        "derivationVersion": climate.derivation_version,
        "width": climate.grid.width,
        "height": climate.grid.height,
        "temperatureCentiC": climate.temperature_centi_c,
        "moistureMmPerYear": climate.moisture_mm_per_year,
        "precipitationMmPerYear": climate.precipitation_mm_per_year,
        "runoffMmPerYear": climate.runoff_mm_per_year,
        "runoffVolumeM3PerYear": climate.runoff_volume_m3_per_year,
        "maritimeFactorPpm": climate.maritime_factor_ppm,
        "metrics": {
            "precipitationVolumeM3PerYear": climate.metrics.precipitation_volume_m3_per_year,
            "runoffVolumeM3PerYear": climate.metrics.runoff_volume_m3_per_year,
            "meanTemperatureCentiC": climate.metrics.mean_temperature_centi_c,
            "minimumTemperatureCentiC": climate.metrics.minimum_temperature_centi_c,
            "maximumTemperatureCentiC": climate.metrics.maximum_temperature_centi_c,
            "meanPrecipitationMmPerYear": climate.metrics.mean_precipitation_mm_per_year,
            "meanRunoffMmPerYear": climate.metrics.mean_runoff_mm_per_year,
            "wettestCellPrecipitationMmPerYear": climate.metrics.wettest_cell_precipitation_mm_per_year,
            "driestLandCellPrecipitationMmPerYear": climate.metrics.driest_land_cell_precipitation_mm_per_year,
            "transportIterations": climate.metrics.transport_iterations,
        },
    })
}

fn physical_evolution_products(
    evolution: &daena_physical::evolution::EvolutionField,
) -> serde_json::Value {
    serde_json::json!({
        "derivationVersion": evolution.derivation_version,
        "preset": evolution.preset.as_str(),
        "width": evolution.grid.width,
        "height": evolution.grid.height,
        "beforeElevationsMm": evolution.before_elevations_mm,
        "elevationsMm": evolution.elevations_mm,
        "routingElevationMm": evolution.drainage.routing_elevation_mm,
        "fillDepthMm": evolution.drainage.fill_depth_mm,
        "slopePpm": evolution.drainage.slope_ppm,
        "accumulationM3PerYear": evolution.drainage.accumulation_m3_per_year,
        "outletCells": evolution.drainage.outlet_cells,
        "edges": evolution.drainage.edges.iter().map(|edge| serde_json::json!({
            "sourceCell": edge.source_cell,
            "destinationCell": edge.destination_cell,
            "weightPpm": edge.weight_ppm,
            "distanceMetres": edge.distance_metres,
        })).collect::<Vec<_>>(),
        "drainageMetrics": {
            "directRunoffM3PerYear": evolution.drainage.metrics.direct_runoff_m3_per_year,
            "routedRunoffM3PerYear": evolution.drainage.metrics.routed_runoff_m3_per_year,
            "routedEdgeCount": evolution.drainage.metrics.routed_edge_count,
            "drainageDensityPpm": evolution.drainage.metrics.drainage_density_ppm,
            "gridAnisotropyPpm": evolution.drainage.metrics.grid_anisotropy_ppm,
            "convergencePpm": evolution.drainage.metrics.convergence_ppm,
            "outletCount": evolution.drainage.metrics.outlet_count,
            "routingSurfaceRaiseMaxMm": evolution.drainage.metrics.routing_surface_raise_max_mm,
        },
        "evolutionMetrics": {
            "initialReliefSpanMm": evolution.metrics.initial_relief_span_mm,
            "finalReliefSpanMm": evolution.metrics.final_relief_span_mm,
            "reliefChangeMm": evolution.metrics.relief_change_mm,
            "meanAbsoluteElevationChangeMm": evolution.metrics.mean_absolute_elevation_change_mm,
            "erosionWorkM3": evolution.metrics.erosion_work_m3,
            "upliftWorkM3": evolution.metrics.uplift_work_m3,
            "maxStepReliefLossMm": evolution.metrics.max_step_relief_loss_mm,
            "drainageDensityPpm": evolution.metrics.drainage_density_ppm,
            "gridAnisotropyPpm": evolution.metrics.grid_anisotropy_ppm,
            "convergencePpm": evolution.metrics.convergence_ppm,
            "tectonicRangeOrientationPpm": evolution.metrics.tectonic_range_orientation_ppm,
        },
    })
}

fn physical_hydrology_products(
    hydrology: &daena_physical::hydrology::HydrologyField,
) -> serde_json::Value {
    serde_json::json!({
        "derivationVersion": hydrology.derivation_version,
        "width": hydrology.grid.width,
        "height": hydrology.grid.height,
        "seaLevelMm": hydrology.sea_level_mm,
        "waterLevelMm": hydrology.water_level_mm,
        "lakeLevelMm": hydrology.lake_level_mm,
        "slopePpm": hydrology.slope_ppm,
        "hillshadePpm": hydrology.hillshade_ppm,
        "bathymetryMm": hydrology.bathymetry_mm,
        "watershedId": hydrology.watershed_id,
        "basinByCell": hydrology.basin_by_cell,
        "lakeCells": hydrology.lake_cells,
        "iceCells": hydrology.ice_cells,
        "iceThicknessMm": hydrology.ice_thickness_mm,
        "shelfCells": hydrology.shelf_cells,
        "islandId": hydrology.island_id,
        "basins": hydrology.basins.iter().map(|basin| serde_json::json!({
            "id": basin.id,
            "minimumCell": basin.minimum_cell,
            "minimumElevationMm": basin.minimum_elevation_mm,
            "cellCount": basin.cell_count,
            "spillCell": basin.spill_cell,
            "spillElevationMm": basin.spill_elevation_mm,
            "volumeToSpillM3": basin.volume_to_spill_m3,
            "parentBasin": basin.parent_basin,
            "children": basin.children,
            "destination": basin.destination.label(),
            "waterLevelMm": basin.water_level_mm,
            "waterVolumeM3": basin.water_volume_m3,
            "inflowM3PerYear": basin.inflow_m3_per_year,
            "directPrecipitationM3PerYear": basin.direct_precipitation_m3_per_year,
            "evaporationM3PerYear": basin.evaporation_m3_per_year,
            "outflowM3PerYear": basin.outflow_m3_per_year,
            "status": basin.status.label(),
        })).collect::<Vec<_>>(),
        "rivers": hydrology.rivers.iter().map(|river| serde_json::json!({
            "id": river.id,
            "sourceCell": river.source_cell,
            "mouthCell": river.mouth_cell,
            "strahlerOrder": river.strahler_order,
            "destination": river.destination.label(),
            "spillOutlet": river.spill_outlet,
            "coordinateCount": river.coordinate_count,
        })).collect::<Vec<_>>(),
        "metrics": {
            "totalWaterM3": hydrology.metrics.total_water_m3,
            "oceanWaterM3": hydrology.metrics.ocean_water_m3,
            "inlandWaterM3": hydrology.metrics.inland_water_m3,
            "landIceM3": hydrology.metrics.land_ice_m3,
            "balanceErrorM3": hydrology.metrics.balance_error_m3,
            "toleranceM3": hydrology.metrics.tolerance_m3,
            "fixedPointIterations": hydrology.metrics.fixed_point_iterations,
            "converged": hydrology.metrics.converged,
            "lakeCount": hydrology.metrics.lake_count,
            "riverCount": hydrology.metrics.river_count,
            "watershedCount": hydrology.metrics.watershed_count,
            "coastlineSegmentCount": hydrology.metrics.coastline_segment_count,
            "landPolygonCount": hydrology.metrics.land_polygon_count,
            "oceanPolygonCount": hydrology.metrics.ocean_polygon_count,
            "shelfCellCount": hydrology.metrics.shelf_cell_count,
            "bathymetryContourCount": hydrology.metrics.bathymetry_contour_count,
            "islandCount": hydrology.metrics.island_count,
        },
    })
}

fn historical_forcing_from_generation(
    generation: &serde_json::Value,
) -> Result<daena_physical::history::HistoricalForcingParameters, String> {
    let Some(value) = generation
        .get("settings")
        .and_then(|settings| settings.get("historicalForcing"))
    else {
        return Err("historicalForcing is required for physical sources".into());
    };
    let settings: daena_core::maps::HistoricalForcingSettings =
        serde_json::from_value(value.clone()).map_err(|error| error.to_string())?;
    if settings.components.len() != daena_physical::history::FORCING_COMPONENT_COUNT {
        return Err("historicalForcing.components must contain three independent terms".into());
    }
    let parameters = daena_physical::history::HistoricalForcingParameters {
        version: settings.version,
        components: [
            daena_physical::history::ForcingComponent {
                amplitude_centi_c: settings.components[0].amplitude_centi_c,
                period_years: settings.components[0].period_years,
                phase_offset_years: settings.components[0].phase_offset_years,
            },
            daena_physical::history::ForcingComponent {
                amplitude_centi_c: settings.components[1].amplitude_centi_c,
                period_years: settings.components[1].period_years,
                phase_offset_years: settings.components[1].phase_offset_years,
            },
            daena_physical::history::ForcingComponent {
                amplitude_centi_c: settings.components[2].amplitude_centi_c,
                period_years: settings.components[2].period_years,
                phase_offset_years: settings.components[2].phase_offset_years,
            },
        ],
        sensitivity_ppm: settings.sensitivity_ppm,
        land_ice_amplitude_ppm: settings.land_ice_amplitude_ppm,
        ice_response_years: settings.ice_response_years,
        ice_midpoint_centi_c: settings.ice_midpoint_centi_c,
        ice_transition_width_centi_c: settings.ice_transition_width_centi_c,
        thermal_expansion_ppm_per_degree_c: settings.thermal_expansion_ppm_per_degree_c,
    };
    parameters.validate().map_err(|error| error.to_string())?;
    Ok(parameters)
}

fn historical_forcing_products(
    parameters: daena_physical::history::HistoricalForcingParameters,
) -> serde_json::Value {
    serde_json::json!({
        "version": parameters.version,
        "components": parameters.components.iter().map(|component| serde_json::json!({
            "amplitudeCentiC": component.amplitude_centi_c,
            "periodYears": component.period_years,
            "phaseOffsetYears": component.phase_offset_years,
        })).collect::<Vec<_>>(),
        "sensitivityPpm": parameters.sensitivity_ppm,
        "landIceAmplitudePpm": parameters.land_ice_amplitude_ppm,
        "iceResponseYears": parameters.ice_response_years,
        "iceMidpointCentiC": parameters.ice_midpoint_centi_c,
        "iceTransitionWidthCentiC": parameters.ice_transition_width_centi_c,
        "thermalExpansionPpmPerDegreeC": parameters.thermal_expansion_ppm_per_degree_c,
    })
}

fn historical_epoch_cache() -> &'static Mutex<BTreeMap<String, serde_json::Value>> {
    HISTORICAL_EPOCH_CACHE.get_or_init(|| Mutex::new(BTreeMap::new()))
}

fn clear_historical_epoch_cache() -> Result<(), String> {
    if let Some(requests) = HISTORICAL_EPOCH_REQUESTS.get() {
        for generation in requests
            .lock()
            .map_err(|_| "historical request state is unavailable".to_string())?
            .values()
        {
            generation.fetch_add(1, Ordering::AcqRel);
        }
    }
    historical_epoch_cache()
        .lock()
        .map_err(|_| "historical cache is unavailable".to_string())?
        .clear();
    Ok(())
}

fn historical_cache_key(
    physical_identity: &str,
    forcing: daena_physical::history::HistoricalForcingParameters,
    normalized_epoch: i64,
) -> String {
    format!(
        "{physical_identity}|history-v{}|hazards-v{}|epoch:{normalized_epoch}|forcing:{}:{}:{}:{}:{}:{}:{}:{}:{}:{}:{}:{}:{}:{}:{}:{}",
        daena_physical::history::HISTORICAL_DERIVATION_VERSION,
        daena_physical::hazards::HAZARD_DERIVATION_VERSION,
        forcing.version,
        forcing.components[0].amplitude_centi_c,
        forcing.components[0].period_years,
        forcing.components[0].phase_offset_years,
        forcing.components[1].amplitude_centi_c,
        forcing.components[1].period_years,
        forcing.components[1].phase_offset_years,
        forcing.components[2].amplitude_centi_c,
        forcing.components[2].period_years,
        forcing.components[2].phase_offset_years,
        forcing.sensitivity_ppm,
        forcing.land_ice_amplitude_ppm,
        forcing.ice_response_years,
        forcing.ice_midpoint_centi_c,
        forcing.ice_transition_width_centi_c,
        forcing.thermal_expansion_ppm_per_degree_c,
    )
}

fn hash_json_value(value: &serde_json::Value) -> String {
    let bytes = serde_json::to_vec(value).expect("JSON values used for hashes are serializable");
    format!("sha256:{:x}", Sha256::digest(bytes))
}

fn historical_derived_hashes(
    world: &daena_physical::tectonics::TectonicWorld,
    source_hash: &str,
    geojson: &str,
    climate: &serde_json::Value,
    hydrology: &serde_json::Value,
) -> serde_json::Value {
    let field = world.physical_field();
    let elevation = serde_json::to_value(field.elevations_mm)
        .expect("elevation fields used for hashes are serializable");
    serde_json::json!({
        "canonicalSource": source_hash,
        "finalElevation": hash_json_value(&elevation),
        "tectonics": source_hash,
        "geography": format!("sha256:{:x}", Sha256::digest(geojson.as_bytes())),
        "climate": hash_json_value(climate),
        "hydrology": hash_json_value(hydrology),
    })
}

fn begin_historical_epoch_request(map_entity_id: &str) -> Result<(Arc<AtomicU64>, u64), String> {
    let requests = HISTORICAL_EPOCH_REQUESTS.get_or_init(|| Mutex::new(BTreeMap::new()));
    let mut requests = requests
        .lock()
        .map_err(|_| "historical request state is unavailable".to_string())?;
    let generation = requests
        .entry(map_entity_id.to_string())
        .or_insert_with(|| Arc::new(AtomicU64::new(0)))
        .clone();
    let expected = generation.fetch_add(1, Ordering::AcqRel).saturating_add(1);
    Ok((generation, expected))
}

#[cfg(test)]
fn derive_reopened_hydrology(
    world: &daena_physical::tectonics::TectonicWorld,
    generation: &serde_json::Value,
    reference_water_inventory_m3: u64,
) -> Result<(String, daena_physical::hydrology::HydrologyField), String> {
    let physics = compute_static_derived(world, generation, reference_water_inventory_m3)?;
    Ok((physics.geojson, physics.hydrology))
}

fn compute_static_derived(
    world: &daena_physical::tectonics::TectonicWorld,
    generation: &serde_json::Value,
    reference_water_inventory_m3: u64,
) -> Result<daena_physical::derived_cache::StaticDerivedPhysics, String> {
    let field = world.physical_field();
    let mut progress = daena_physical::NoopProgress;
    let climate = daena_physical::climate::derive_current_climate(
        &field,
        daena_physical::climate::ClimateSettings::default_for(field.grid),
        world.seed,
        world.retry_index,
        &mut progress,
    )
    .map_err(|error| format!("maps.physical: {error}"))?;
    let preset = generation
        .get("settings")
        .and_then(|settings| settings.get("evolutionPreset"))
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| "evolutionPreset is required for physical sources".to_string())?;
    let preset = daena_physical::evolution::EvolutionPreset::parse(preset)
        .map_err(|error| error.to_string())?;
    let mut initial_progress = daena_physical::NoopProgress;
    let initial_world = daena_physical::tectonics::generate_tectonic_world(
        world.grid,
        world.settings,
        world.target_land_fraction_ppm,
        world.seed,
        world.retry_index,
        &mut initial_progress,
    )
    .map_err(|error| format!("maps.physical: {error}"))?;
    let evolution = daena_physical::evolution::diagnostics_from_before_after(
        initial_world.elevations_mm,
        &field,
        &climate,
        preset,
    )
    .map_err(|error| format!("maps.physical: {error}"))?;
    let hydrology = daena_physical::hydrology::derive_hydrology_with_crust(
        &field,
        &climate,
        &evolution.drainage,
        reference_water_inventory_m3,
        Some(&world.crust_by_cell),
    )
    .map_err(|error| format!("maps.physical: {error}"))?;
    let diagnostic = daena_physical::tectonics::to_diagnostic_geojson(world)
        .map_err(|error| format!("maps.physical: {error}"))?;
    let hazards = daena_physical::hazards::to_geojson(world)
        .map_err(|error| format!("maps.physical: {error}"))?;
    let water = daena_physical::hydrology::to_geojson(&hydrology)
        .map_err(|error| format!("maps.physical: {error}"))?;
    let diagnostic_with_hazards =
        daena_physical::merge_geojson_features_for_host(&diagnostic, &hazards)?;
    let geojson =
        daena_physical::merge_geojson_features_for_host(&diagnostic_with_hazards, &water)?;
    let ocean_curve = daena_physical::hydrology::ocean_volume_curve(&field)
        .map_err(|error| format!("maps.physical: {error}"))?;
    Ok(daena_physical::derived_cache::StaticDerivedPhysics {
        climate,
        evolution,
        hydrology,
        ocean_curve,
        geojson,
    })
}

fn load_or_fill_static_derived(
    project_root: &str,
    identity: &str,
    world: &daena_physical::tectonics::TectonicWorld,
    generation: &serde_json::Value,
    reference_water_inventory_m3: u64,
) -> Result<daena_physical::derived_cache::StaticDerivedPhysics, String> {
    let cache_dir =
        daena_core::maps::physical::physical_derived_cache_dir(Path::new(project_root), identity)
            .map_err(|error| error.to_string())?;
    if let Some(hit) = daena_physical::derived_cache::load(&cache_dir)
        .map_err(|error| format!("maps.physical: {error}"))?
    {
        return Ok(hit);
    }
    let physics = compute_static_derived(world, generation, reference_water_inventory_m3)?;
    let _ = daena_physical::derived_cache::save(&cache_dir, &physics);
    Ok(physics)
}

fn write_static_derived_from_job(
    project_root: &str,
    result: &PhysicalJobResult,
) -> Result<(), String> {
    let cache_dir = daena_core::maps::physical::physical_derived_cache_dir(
        Path::new(project_root),
        &result.physical_identity,
    )
    .map_err(|error| error.to_string())?;
    let world = daena_physical::decode_source(&result.source)?;
    let ocean_curve = daena_physical::hydrology::ocean_volume_curve(&world.physical_field())
        .map_err(|error| format!("maps.physical: {error}"))?;
    daena_physical::derived_cache::save(
        &cache_dir,
        &daena_physical::derived_cache::StaticDerivedPhysics {
            climate: result.climate.clone(),
            evolution: result.evolution.clone(),
            hydrology: result.hydrology.clone(),
            ocean_curve,
            geojson: result.derived_geojson.clone(),
        },
    )
    .map_err(|error| format!("maps.physical: {error}"))
}

#[cfg(test)]
fn derive_reopened_historical(
    world: &daena_physical::tectonics::TectonicWorld,
    generation: &serde_json::Value,
    reference_water_inventory_m3: u64,
    epoch_offset_years: i64,
    progress: &mut dyn daena_physical::ProgressSink,
) -> Result<
    (
        daena_physical::history::HistoricalWorld,
        daena_physical::history::HistoricalForcingParameters,
        String,
    ),
    String,
> {
    let parameters = historical_forcing_from_generation(generation)?;
    let field = world.physical_field();
    let historical = daena_physical::history::derive_historical_world(
        &field,
        reference_water_inventory_m3,
        Some(&world.crust_by_cell),
        parameters,
        epoch_offset_years,
        progress,
    )
    .map_err(|error| format!("maps.physical: {error}"))?;
    let diagnostic = daena_physical::tectonics::to_diagnostic_geojson(world)
        .map_err(|error| format!("maps.physical: {error}"))?;
    let hazards = daena_physical::hazards::to_geojson(world)
        .map_err(|error| format!("maps.physical: {error}"))?;
    let water = daena_physical::hydrology::to_geojson(&historical.hydrology)
        .map_err(|error| format!("maps.physical: {error}"))?;
    let diagnostic_with_hazards =
        daena_physical::merge_geojson_features_for_host(&diagnostic, &hazards)?;
    let geojson =
        daena_physical::merge_geojson_features_for_host(&diagnostic_with_hazards, &water)?;
    Ok((historical, parameters, geojson))
}

fn derive_reopened_historical_from_static(
    world: &daena_physical::tectonics::TectonicWorld,
    generation: &serde_json::Value,
    reference_water_inventory_m3: u64,
    epoch_offset_years: i64,
    static_physics: &daena_physical::derived_cache::StaticDerivedPhysics,
    progress: &mut dyn daena_physical::ProgressSink,
) -> Result<
    (
        daena_physical::history::HistoricalWorld,
        daena_physical::history::HistoricalForcingParameters,
        String,
    ),
    String,
> {
    let parameters = historical_forcing_from_generation(generation)?;
    let field = world.physical_field();
    let historical = daena_physical::history::derive_historical_world_from_static(
        &field,
        static_physics,
        reference_water_inventory_m3,
        Some(&world.crust_by_cell),
        parameters,
        epoch_offset_years,
        progress,
    )
    .map_err(|error| format!("maps.physical: {error}"))?;
    if epoch_offset_years == 0 {
        return Ok((historical, parameters, static_physics.geojson.clone()));
    }
    let diagnostic = daena_physical::tectonics::to_diagnostic_geojson(world)
        .map_err(|error| format!("maps.physical: {error}"))?;
    let hazards = daena_physical::hazards::to_geojson(world)
        .map_err(|error| format!("maps.physical: {error}"))?;
    let water = daena_physical::hydrology::to_geojson(&historical.hydrology)
        .map_err(|error| format!("maps.physical: {error}"))?;
    let diagnostic_with_hazards =
        daena_physical::merge_geojson_features_for_host(&diagnostic, &hazards)?;
    let geojson =
        daena_physical::merge_geojson_features_for_host(&diagnostic_with_hazards, &water)?;
    Ok((historical, parameters, geojson))
}

#[allow(clippy::too_many_arguments)]
fn historical_response(
    world: &daena_physical::tectonics::TectonicWorld,
    source_hash: &str,
    physical_identity: &str,
    cache_key: String,
    epoch_offset_years: i64,
    normalized_epoch: i64,
    historical: &daena_physical::history::HistoricalWorld,
    parameters: daena_physical::history::HistoricalForcingParameters,
    geojson: String,
) -> serde_json::Value {
    let climate = physical_climate_products(&historical.climate);
    let hydrology = physical_hydrology_products(&historical.hydrology);
    let derived_hashes =
        historical_derived_hashes(world, source_hash, &geojson, &climate, &hydrology);
    serde_json::json!({
        "cacheKey": cache_key,
        "sourceHash": source_hash,
        "physicalIdentity": physical_identity,
        "epochOffsetYears": epoch_offset_years,
        "normalizedEpoch": normalized_epoch,
        "chronology": {
            "contractVersion": 1,
            "kind": "physical-offset-years",
            "reference": "accepted-source",
            "epochOffsetYears": epoch_offset_years,
        },
        "geojson": geojson,
        "climate": climate,
        "hydrology": hydrology,
        "hazards": {
            "derivationVersion": daena_physical::hazards::HAZARD_DERIVATION_VERSION,
            "volcanicSourceDerivationVersion": daena_physical::hazards::VOLCANIC_SOURCE_DERIVATION_VERSION,
            "model": "relative-generated-v1",
            "prediction": false,
        },
        "derivedHashes": derived_hashes,
        "forcing": historical_forcing_products(parameters),
        "history": {
            "derivationVersion": historical.metrics.derivation_version,
            "epochOffsetYears": historical.metrics.epoch_offset_years,
            "normalizedEpoch": historical.metrics.normalized_epoch,
            "temperatureOffsetCentiC": historical.metrics.temperature_offset_centi_c,
            "laggedTemperatureOffsetCentiC": historical.metrics.lagged_temperature_offset_centi_c,
            "landIceEquilibriumM3": historical.metrics.land_ice_equilibrium_m3,
            "landIceM3": historical.metrics.land_ice_m3,
            "thermalExpansionM3": historical.metrics.thermal_expansion_m3,
            "effectiveOceanWaterM3": historical.metrics.effective_ocean_water_m3,
            "conservedWaterM3": historical.metrics.conserved_water_m3,
            "balanceErrorM3": historical.metrics.balance_error_m3,
            "seaLevelMm": historical.metrics.sea_level_mm,
        },
    })
}

#[tauri::command]
fn project_physical_climate(
    state: tauri::State<'_, SharedCore>,
    jobs: tauri::State<'_, SharedPhysicalJobs>,
    job_id: String,
) -> Result<serde_json::Value, String> {
    let project_id = current_info(state.inner())?
        .ok_or_else(|| "open a project before reading physical climate".to_string())?
        .root;
    let mut manager = jobs
        .lock()
        .map_err(|_| "physical job state is unavailable".to_string())?;
    manager.reap_expired();
    let job = manager
        .jobs
        .get(&job_id)
        .filter(|job| manager.active_session_matches(&project_id, &job.session_id))
        .ok_or("physical job was not found, expired, or belongs to another session")?;
    let climate = job
        .result
        .as_ref()
        .map(|result| &result.climate)
        .ok_or_else(|| "physical job climate is not ready".to_string())?;
    Ok(physical_climate_products(climate))
}

#[tauri::command]
fn project_physical_evolution(
    state: tauri::State<'_, SharedCore>,
    jobs: tauri::State<'_, SharedPhysicalJobs>,
    job_id: String,
) -> Result<serde_json::Value, String> {
    let project_id = current_info(state.inner())?
        .ok_or_else(|| "open a project before reading physical evolution".to_string())?
        .root;
    let mut manager = jobs
        .lock()
        .map_err(|_| "physical job state is unavailable".to_string())?;
    manager.reap_expired();
    let job = manager
        .jobs
        .get(&job_id)
        .filter(|job| manager.active_session_matches(&project_id, &job.session_id))
        .ok_or("physical job was not found, expired, or belongs to another session")?;
    let evolution = job
        .result
        .as_ref()
        .map(|result| &result.evolution)
        .ok_or_else(|| "physical job evolution is not ready".to_string())?;
    Ok(physical_evolution_products(evolution))
}

#[tauri::command]
fn project_physical_hydrology(
    state: tauri::State<'_, SharedCore>,
    jobs: tauri::State<'_, SharedPhysicalJobs>,
    job_id: String,
) -> Result<serde_json::Value, String> {
    let project_id = current_info(state.inner())?
        .ok_or_else(|| "open a project before reading physical hydrology".to_string())?
        .root;
    let mut manager = jobs
        .lock()
        .map_err(|_| "physical job state is unavailable".to_string())?;
    manager.reap_expired();
    let job = manager
        .jobs
        .get(&job_id)
        .filter(|job| manager.active_session_matches(&project_id, &job.session_id))
        .ok_or("physical job was not found, expired, or belongs to another session")?;
    let hydrology = job
        .result
        .as_ref()
        .map(|result| &result.hydrology)
        .ok_or_else(|| "physical job hydrology is not ready".to_string())?;
    Ok(physical_hydrology_products(hydrology))
}

#[tauri::command]
async fn project_physical_accept(
    state: tauri::State<'_, SharedCore>,
    jobs: tauri::State<'_, SharedPhysicalJobs>,
    job_id: String,
    name: String,
    request_id: Option<String>,
) -> Result<daena_core::AcceptedPhysicalMap, String> {
    let project_id = current_info(state.inner())?
        .ok_or_else(|| "open a project before accepting a physical map".to_string())?
        .root;
    let result = {
        let mut manager = jobs
            .lock()
            .map_err(|_| "physical job state is unavailable".to_string())?;
        manager.reap_expired();
        let job = manager
            .jobs
            .get(&job_id)
            .ok_or_else(|| "physical job was not found or has expired".to_string())?;
        if job.project_id != project_id {
            return Err("physical job belongs to a different project".into());
        }
        if !manager.active_session_matches(&project_id, &job.session_id) {
            return Err("physical job belongs to another session".into());
        }
        if job.status.state != "completed" {
            return Err("physical job is not ready for acceptance".into());
        }
        job.result
            .clone()
            .ok_or_else(|| "physical job result is missing".to_string())?
    };
    let expected_identity = result.physical_identity.clone();
    let _ = write_static_derived_from_job(&project_id, &result);
    let accepted = with_core(state, move |core| {
        let accepted = core.project(trusted_shell())?.accept_physical_map(
            name,
            result.source,
            result.generation,
            request_id.as_deref(),
        )?;
        if accepted.physical_identity != expected_identity {
            return Err(CoreError::Validation(
                "physical identity changed between generation and acceptance".into(),
            ));
        }
        Ok(accepted)
    })
    .await?;
    jobs.lock()
        .map_err(|_| "physical job state is unavailable".to_string())?
        .jobs
        .remove(&job_id);
    Ok(accepted)
}

#[tauri::command]
async fn project_replace_vector_source(
    state: tauri::State<'_, SharedCore>,
    asset_id: String,
    bytes: Vec<u8>,
    upload_content_hash: String,
    expected_revision: String,
    request_id: Option<String>,
) -> Result<daena_core::VectorSourceReplace, String> {
    with_core(state, move |core| {
        core.project(trusted_shell())?.replace_vector_source(
            asset_id,
            bytes,
            upload_content_hash,
            &expected_revision,
            request_id.as_deref(),
        )
    })
    .await
}

#[tauri::command]
async fn project_create_vector_layer(
    state: tauri::State<'_, SharedCore>,
    map_entity_id: String,
    name: String,
    expected_revision: String,
    style: Option<serde_json::Value>,
    request_id: Option<String>,
) -> Result<daena_core::RasterLayerChange, String> {
    with_core(state, move |core| {
        core.project(trusted_shell())?.create_vector_layer(
            map_entity_id,
            name,
            &expected_revision,
            request_id.as_deref(),
            style,
        )
    })
    .await
}

#[tauri::command]
#[allow(clippy::too_many_arguments)]
async fn project_delete_vector_layer(
    state: tauri::State<'_, SharedCore>,
    map_entity_id: String,
    layer_id: String,
    expected_revision: String,
    expected_source_revision: String,
    expected_feature_count: i64,
    request_id: Option<String>,
) -> Result<daena_core::VectorLayerDelete, String> {
    with_core(state, move |core| {
        core.project(trusted_shell())?.delete_vector_layer(
            map_entity_id,
            layer_id,
            &expected_revision,
            &expected_source_revision,
            expected_feature_count,
            request_id.as_deref(),
        )
    })
    .await
}

#[tauri::command]
async fn maps_recovery_export(
    state: tauri::State<'_, SharedCore>,
    entity_id: String,
    bytes: Vec<u8>,
) -> Result<String, String> {
    with_core(state, move |core| {
        core.project(trusted_shell())?
            .save_map_recovery_copy(&entity_id, &bytes)
    })
    .await
}

#[tauri::command]
async fn project_physical_derived_geojson(
    state: tauri::State<'_, SharedCore>,
    map_entity_id: String,
) -> Result<String, String> {
    with_read_project(state, move |project| {
        let descriptor = project
            .list_fields(map_entity_id.clone())?
            .into_iter()
            .find(|field| field.namespace == daena_core::maps::MAP_NAMESPACE && field.key == "map")
            .ok_or_else(|| CoreError::Validation("maps:map descriptor is missing".into()))?;
        let source_id = descriptor
            .value
            .get("sourceAssetId")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| CoreError::Validation("maps: sourceAssetId is missing".into()))?;
        let generation =
            descriptor.value.get("generation").cloned().ok_or_else(|| {
                CoreError::Validation("maps: physical generation is missing".into())
            })?;
        let bytes = project.asset_bytes(source_id.to_string())?;
        let validated = daena_core::maps::physical::validate_source(&bytes, &generation)?;
        let world = &validated.world;
        let report = validated.report;
        let physics = load_or_fill_static_derived(
            &project.info().ok_or(CoreError::ProjectNotOpen)?.root,
            &validated.identity,
            world,
            &generation,
            report.reference_water_inventory_m3,
        )
        .map_err(CoreError::Validation)?;
        Ok(physics.geojson)
    })
    .await
}

#[tauri::command]
async fn project_physical_derived_climate(
    state: tauri::State<'_, SharedCore>,
    map_entity_id: String,
) -> Result<serde_json::Value, String> {
    with_read_project(state, move |project| {
        let descriptor = project
            .list_fields(map_entity_id.clone())?
            .into_iter()
            .find(|field| field.namespace == daena_core::maps::MAP_NAMESPACE && field.key == "map")
            .ok_or_else(|| CoreError::Validation("maps:map descriptor is missing".into()))?;
        let source_id = descriptor
            .value
            .get("sourceAssetId")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| CoreError::Validation("maps: sourceAssetId is missing".into()))?;
        let generation =
            descriptor.value.get("generation").cloned().ok_or_else(|| {
                CoreError::Validation("maps: physical generation is missing".into())
            })?;
        let bytes = project.asset_bytes(source_id.to_string())?;
        let validated = daena_core::maps::physical::validate_source(&bytes, &generation)?;
        let world = &validated.world;
        let physics = load_or_fill_static_derived(
            &project.info().ok_or(CoreError::ProjectNotOpen)?.root,
            &validated.identity,
            world,
            &generation,
            validated.report.reference_water_inventory_m3,
        )
        .map_err(CoreError::Validation)?;
        Ok(physical_climate_products(&physics.climate))
    })
    .await
}

#[tauri::command]
async fn project_physical_derived_evolution(
    state: tauri::State<'_, SharedCore>,
    map_entity_id: String,
) -> Result<serde_json::Value, String> {
    with_read_project(state, move |project| {
        let descriptor = project
            .list_fields(map_entity_id.clone())?
            .into_iter()
            .find(|field| field.namespace == daena_core::maps::MAP_NAMESPACE && field.key == "map")
            .ok_or_else(|| CoreError::Validation("maps:map descriptor is missing".into()))?;
        let source_id = descriptor
            .value
            .get("sourceAssetId")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| CoreError::Validation("maps: sourceAssetId is missing".into()))?;
        let generation =
            descriptor.value.get("generation").cloned().ok_or_else(|| {
                CoreError::Validation("maps: physical generation is missing".into())
            })?;
        let bytes = project.asset_bytes(source_id.to_string())?;
        let validated = daena_core::maps::physical::validate_source(&bytes, &generation)?;
        let world = &validated.world;
        let physics = load_or_fill_static_derived(
            &project.info().ok_or(CoreError::ProjectNotOpen)?.root,
            &validated.identity,
            world,
            &generation,
            validated.report.reference_water_inventory_m3,
        )
        .map_err(CoreError::Validation)?;
        Ok(physical_evolution_products(&physics.evolution))
    })
    .await
}

#[tauri::command]
async fn project_physical_derived_hydrology(
    state: tauri::State<'_, SharedCore>,
    map_entity_id: String,
) -> Result<serde_json::Value, String> {
    with_read_project(state, move |project| {
        let descriptor = project
            .list_fields(map_entity_id.clone())?
            .into_iter()
            .find(|field| field.namespace == daena_core::maps::MAP_NAMESPACE && field.key == "map")
            .ok_or_else(|| CoreError::Validation("maps:map descriptor is missing".into()))?;
        let source_id = descriptor
            .value
            .get("sourceAssetId")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| CoreError::Validation("maps: sourceAssetId is missing".into()))?;
        let generation =
            descriptor.value.get("generation").cloned().ok_or_else(|| {
                CoreError::Validation("maps: physical generation is missing".into())
            })?;
        let bytes = project.asset_bytes(source_id.to_string())?;
        let validated = daena_core::maps::physical::validate_source(&bytes, &generation)?;
        let world = &validated.world;
        let report = validated.report;
        let physics = load_or_fill_static_derived(
            &project.info().ok_or(CoreError::ProjectNotOpen)?.root,
            &validated.identity,
            world,
            &generation,
            report.reference_water_inventory_m3,
        )
        .map_err(CoreError::Validation)?;
        Ok(physical_hydrology_products(&physics.hydrology))
    })
    .await
}

#[tauri::command]
async fn project_physical_derived_epoch(
    state: tauri::State<'_, SharedCore>,
    app: tauri::AppHandle,
    map_entity_id: String,
    epoch_offset_years: i64,
    request_id: Option<String>,
) -> Result<serde_json::Value, String> {
    let (request_generation, expected_generation) = begin_historical_epoch_request(&map_entity_id)?;
    let request_id = request_id.unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
    with_read_project(state, move |project| {
        let descriptor = project
            .list_fields(map_entity_id.clone())?
            .into_iter()
            .find(|field| field.namespace == daena_core::maps::MAP_NAMESPACE && field.key == "map")
            .ok_or_else(|| CoreError::Validation("maps:map descriptor is missing".into()))?;
        let source_id = descriptor
            .value
            .get("sourceAssetId")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| CoreError::Validation("maps: sourceAssetId is missing".into()))?;
        let generation =
            descriptor.value.get("generation").cloned().ok_or_else(|| {
                CoreError::Validation("maps: physical generation is missing".into())
            })?;
        let bytes = project.asset_bytes(source_id.to_string())?;
        let validated = daena_core::maps::physical::validate_source(&bytes, &generation)?;
        let world = &validated.world;
        let report = validated.report;
        let physical_identity = validated.identity.clone();
        let forcing =
            historical_forcing_from_generation(&generation).map_err(CoreError::Validation)?;
        let normalized_epoch = daena_physical::history::normalize_epoch_offset(epoch_offset_years)
            .map_err(|error| CoreError::Validation(error.to_string()))?;
        let source_hash = format!("sha256:{:x}", Sha256::digest(&bytes));
        let cache_key = historical_cache_key(&physical_identity, forcing, normalized_epoch);
        {
            let cache = historical_epoch_cache()
                .lock()
                .map_err(|_| CoreError::Validation("historical cache is unavailable".into()))?;
            if let Some(value) = cache.get(&cache_key) {
                return Ok(value.clone());
            }
        }
        let physics = load_or_fill_static_derived(
            &project.info().ok_or(CoreError::ProjectNotOpen)?.root,
            &physical_identity,
            world,
            &generation,
            report.reference_water_inventory_m3,
        )
        .map_err(CoreError::Validation)?;
        let (historical, parameters, geojson) = derive_reopened_historical_from_static(
            world,
            &generation,
            report.reference_water_inventory_m3,
            normalized_epoch,
            &physics,
            &mut HistoricalProgress::with_reporter(
                request_generation,
                expected_generation,
                app,
                map_entity_id.clone(),
                request_id,
            ),
        )
        .map_err(CoreError::Validation)?;
        let value = historical_response(
            world,
            &source_hash,
            &physical_identity,
            cache_key.clone(),
            epoch_offset_years,
            normalized_epoch,
            &historical,
            parameters,
            geojson,
        );
        let mut cache = historical_epoch_cache()
            .lock()
            .map_err(|_| CoreError::Validation("historical cache is unavailable".into()))?;
        while cache.len() >= HISTORICAL_CACHE_CAPACITY {
            let Some(oldest) = cache.keys().next().cloned() else {
                break;
            };
            cache.remove(&oldest);
        }
        cache.insert(cache_key, value.clone());
        Ok(value)
    })
    .await
}

fn deterministic_event_location_id(request_id: &str, ordinal: u32) -> String {
    let digest =
        Sha256::digest(format!("daena-physical-event-location:{request_id}:{ordinal}").as_bytes());
    let mut bytes = [0u8; 16];
    bytes.copy_from_slice(&digest[..16]);
    // UUIDv4-shaped IDs are accepted by the canonical maps.locations schema;
    // the bytes remain deterministic for idempotent retries.
    bytes[6] = (bytes[6] & 0x0f) | 0x40;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    uuid::Uuid::from_bytes(bytes).to_string()
}

#[tauri::command]
async fn project_physical_materialize_events(
    state: tauri::State<'_, SharedCore>,
    map_entity_id: String,
    request: daena_physical::events::EventMaterializationRequest,
    request_id: Option<String>,
) -> Result<PhysicalEventMaterializationResult, String> {
    let request_id = request_id.unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
    let response_map_id = map_entity_id.clone();
    with_core(state, move |core| {
        let project = core.project(trusted_shell())?;
        let provider = project.map_provider_id(&map_entity_id)?;
        if provider != daena_core::maps::PHYSICAL_PROVIDER {
            return Err(CoreError::Validation(
                "natural-event materialization requires a daena-physical map".into(),
            ));
        }
        let descriptor = project
            .list_fields(map_entity_id.clone())?
            .into_iter()
            .find(|field| field.namespace == daena_core::maps::MAP_NAMESPACE && field.key == "map")
            .ok_or_else(|| CoreError::Validation("maps:map descriptor is missing".into()))?;
        let source_id = descriptor
            .value
            .get("sourceAssetId")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| CoreError::Validation("maps: sourceAssetId is missing".into()))?;
        let generation = descriptor
            .value
            .get("generation")
            .cloned()
            .ok_or_else(|| CoreError::Validation("maps: physical generation is missing".into()))?;
        let bytes = project.asset_bytes(source_id.to_string())?;
        let validated = daena_core::maps::physical::validate_source(&bytes, &generation)?;
        let world = &validated.world;
        let hazards = daena_physical::hazards::derive_hazards(world)
            .map_err(|error| CoreError::Validation(error.to_string()))?;
        let events = daena_physical::events::sample_events(world, &hazards, &request)
            .map_err(CoreError::Validation)?;
        let source_hash = format!("sha256:{:x}", Sha256::digest(&bytes));
        let generator_id = generation
            .get("id")
            .and_then(serde_json::Value::as_str)
            .unwrap_or(daena_physical::GENERATOR_ID)
            .to_owned();
        let generator_version = generation
            .get("version")
            .and_then(serde_json::Value::as_u64)
            .unwrap_or(daena_physical::GENERATOR_VERSION as u64);
        let retry_index = generation
            .get("retryIndex")
            .and_then(serde_json::Value::as_u64)
            .unwrap_or(u64::from(world.retry_index));
        let entries = events
            .iter()
            .map(|event| {
                let name = format!(
                    "{} · year {} · M {:.3}",
                    event.event_kind.label(),
                    event.year_offset,
                    f64::from(event.magnitude_milli) / 1_000.0
                );
                let location_id = deterministic_event_location_id(&request_id, event.ordinal);
                let x = (f64::from(event.longitude_microdegrees) / 1_000_000.0 + 180.0) / 360.0;
                let y = (f64::from(event.latitude_microdegrees) / 1_000_000.0 + 90.0) / 180.0;
                let provenance = serde_json::json!({
                    "materializationVersion": daena_physical::events::EVENT_MATERIALIZATION_VERSION,
                    "hazardDerivationVersion": daena_physical::hazards::HAZARD_DERIVATION_VERSION,
                    "eventModel": event.event_kind.model_label(),
                    "eventKind": event.event_kind.label(),
                    "hazardSeed": request.hazard_seed,
                    "intervalStartYears": request.interval_start_years,
                    "intervalEndYears": request.interval_end_years,
                    "yearOffset": event.year_offset,
                    "cell": event.cell,
                    "longitudeMicrodegrees": event.longitude_microdegrees,
                    "latitudeMicrodegrees": event.latitude_microdegrees,
                    "magnitudeMilli": event.magnitude_milli,
                    "hazardPpm": event.hazard_ppm,
                    "annualRateNano": event.annual_rate_nano,
                    "ratePerMillionYearsPpm": event.rate_per_million_years_ppm,
                    "sampledCenterId": event.sampled_center_id,
                    "volcanicSourceDerivationVersion": event.volcanic_source_derivation_version,
                    "physicalIdentity": validated.identity,
                    "requestId": request_id,
                    "materializationKey": format!("{}:{}:{}", event.event_kind.label(), request.hazard_seed, event.ordinal),
                    "sourceHash": source_hash,
                    "generatorId": generator_id,
                    "generatorVersion": generator_version,
                    "sourceRetryIndex": retry_index,
                    "prediction": false,
                });
                let document = format!(
                    "# {}\n\n- Relative time offset: {} years\n- Magnitude/index: {:.3}\n- Location: {:.3}°, {:.3}°\n- Model: {}\n- Prediction: no; this is generated relative history.\n",
                    name,
                    event.year_offset,
                    f64::from(event.magnitude_milli) / 1_000.0,
                    f64::from(event.latitude_microdegrees) / 1_000_000.0,
                    f64::from(event.longitude_microdegrees) / 1_000_000.0,
                    event.event_kind.model_label(),
                );
                CreateEntry {
                    name: name.clone(),
                    entity_type: Some(daena_core::maps::PHYSICAL_EVENT_ENTITY_TYPE.into()),
                    document: Some(daena_core::CreateEntryDocument {
                        body: document,
                        format: Some("markdown".into()),
                    }),
                    fields: vec![
                        CreateEntryField {
                            namespace: daena_core::maps::PHYSICAL_EVENT_NAMESPACE.into(),
                            key: "provenance".into(),
                            value: provenance,
                        },
                        CreateEntryField {
                            namespace: daena_core::maps::PHYSICAL_EVENT_NAMESPACE.into(),
                            key: "materializationKey".into(),
                            value: serde_json::json!(format!(
                                "{}:{}:{}",
                                event.event_kind.label(),
                                request.hazard_seed,
                                event.ordinal
                            )),
                        },
                        CreateEntryField {
                            namespace: daena_core::maps::MAP_NAMESPACE.into(),
                            key: daena_core::maps::PHYSICAL_EVENT_CHRONOLOGY_KEY.into(),
                            value: serde_json::json!({
                                "contractVersion": 1,
                                "kind": "physical-offset-years",
                                "reference": "accepted-source",
                                "startOffsetYears": event.year_offset,
                                "endOffsetYears": event.year_offset,
                            }),
                        },
                        CreateEntryField {
                            namespace: daena_core::maps::MAP_NAMESPACE.into(),
                            key: "locations".into(),
                            value: serde_json::json!({
                                "schemaVersion": 1,
                                "locations": [{
                                    "id": location_id,
                                    "mapEntityId": map_entity_id,
                                    "role": "physical-event",
                                    "label": name,
                                    "anchor": {"kind": "point", "point": [x, y]},
                                    "validity": {"from": null, "to": null}
                                }]
                            }),
                        },
                    ],
                    relationships: vec![CreateEntryRelationship {
                        relationship_type: daena_core::maps::PHYSICAL_EVENT_ON_MAP_RELATIONSHIP.into(),
                        target_ids: vec![map_entity_id.clone()],
                    }],
                }
            })
            .collect::<Vec<_>>();
        let entities = project.create_entries_with_request(entries, Some(&request_id))?;
        let materialized = events
            .into_iter()
            .zip(entities)
            .map(|(event, entity)| MaterializedPhysicalEvent {
                entity_id: entity.id,
                event,
            })
            .collect();
        Ok(PhysicalEventMaterializationResult {
            request_id,
            map_entity_id: response_map_id,
            materialization_version: daena_physical::events::EVENT_MATERIALIZATION_VERSION,
            hazard_derivation_version: daena_physical::hazards::HAZARD_DERIVATION_VERSION,
            prediction: false,
            events: materialized,
        })
    })
    .await
}

#[tauri::command]
fn project_physical_clear_epoch_cache() -> Result<(), String> {
    clear_historical_epoch_cache()
}

#[tauri::command]
async fn project_read_asset_bytes(
    state: tauri::State<'_, SharedCore>,
    asset_id: String,
) -> Result<Vec<u8>, String> {
    with_read_project(state, move |project| project.asset_bytes(asset_id)).await
}

#[tauri::command]
async fn project_create_raster_layer(
    state: tauri::State<'_, SharedCore>,
    map_entity_id: String,
    name: String,
    expected_revision: String,
    request_id: Option<String>,
) -> Result<daena_core::RasterLayerChange, String> {
    with_core(state, move |core| {
        core.project(trusted_shell())?.create_raster_layer(
            map_entity_id,
            name,
            &expected_revision,
            request_id.as_deref(),
        )
    })
    .await
}

#[tauri::command]
async fn project_create_semantic_layer(
    state: tauri::State<'_, SharedCore>,
    map_entity_id: String,
    name: String,
    expected_revision: String,
    style: Option<serde_json::Value>,
    selector: Option<serde_json::Value>,
    request_id: Option<String>,
) -> Result<daena_core::RasterLayerChange, String> {
    with_core(state, move |core| {
        core.project(trusted_shell())?.create_semantic_layer(
            map_entity_id,
            name,
            &expected_revision,
            request_id.as_deref(),
            style,
            selector,
        )
    })
    .await
}

#[tauri::command]
async fn project_delete_semantic_layer(
    state: tauri::State<'_, SharedCore>,
    map_entity_id: String,
    layer_id: String,
    expected_revision: String,
    request_id: Option<String>,
) -> Result<daena_core::RasterLayerChange, String> {
    with_core(state, move |core| {
        core.project(trusted_shell())?.delete_semantic_layer(
            map_entity_id,
            layer_id,
            &expected_revision,
            request_id.as_deref(),
        )
    })
    .await
}

#[tauri::command]
#[allow(clippy::too_many_arguments)]
async fn project_update_map_layer(
    state: tauri::State<'_, SharedCore>,
    map_entity_id: String,
    layer_id: String,
    expected_revision: String,
    name: Option<String>,
    order: Option<i64>,
    default_visible: Option<bool>,
    opacity: Option<f64>,
    locked: Option<bool>,
    style: Option<serde_json::Value>,
    selector: Option<serde_json::Value>,
    request_id: Option<String>,
) -> Result<daena_core::RasterLayerChange, String> {
    with_core(state, move |core| {
        core.project(trusted_shell())?.update_map_layer(
            map_entity_id,
            layer_id,
            daena_core::RasterLayerUpdate {
                name,
                order,
                default_visible,
                opacity,
                locked,
                style,
                selector,
            },
            &expected_revision,
            request_id.as_deref(),
        )
    })
    .await
}

#[tauri::command]
async fn project_delete_raster_layer(
    state: tauri::State<'_, SharedCore>,
    map_entity_id: String,
    layer_id: String,
    expected_revision: String,
    request_id: Option<String>,
) -> Result<daena_core::RasterLayerChange, String> {
    with_core(state, move |core| {
        core.project(trusted_shell())?.delete_raster_layer(
            map_entity_id,
            layer_id,
            &expected_revision,
            request_id.as_deref(),
        )
    })
    .await
}

#[tauri::command]
async fn project_replace_asset_bytes(
    state: tauri::State<'_, SharedCore>,
    asset_id: String,
    bytes: Vec<u8>,
    content_hash: String,
    mime_type: String,
    expected_revision: String,
    request_id: Option<String>,
) -> Result<Asset, String> {
    let size = bytes.len() as i64;
    with_core(state, move |core| {
        core.project(trusted_shell())?
            .replace_asset_bytes_with_request(
                AssetReplaceInput {
                    asset_id,
                    content_hash,
                    size,
                    mime_type,
                },
                bytes,
                &expected_revision,
                request_id.as_deref(),
            )
    })
    .await
}

#[tauri::command]
async fn project_replace_asset_file(
    state: tauri::State<'_, SharedCore>,
    asset_id: String,
    source_path: String,
    mime_type: String,
    expected_revision: String,
    request_id: Option<String>,
) -> Result<Asset, String> {
    with_core(state, move |core| {
        core.project(trusted_shell())?
            .replace_asset_file_with_request(
                AssetFileReplaceInput {
                    asset_id,
                    source_path,
                    mime_type,
                },
                &expected_revision,
                request_id.as_deref(),
            )
    })
    .await
}

#[tauri::command]
async fn project_update_asset_metadata(
    state: tauri::State<'_, SharedCore>,
    asset_id: String,
    filename: Option<String>,
    role: Option<String>,
    reference_scope: Option<String>,
    expected_revision: String,
    request_id: Option<String>,
) -> Result<Asset, String> {
    with_core(state, move |core| {
        core.project(trusted_shell())?
            .update_asset_metadata_with_request(
                AssetMetadataUpdate {
                    asset_id,
                    filename,
                    role,
                    reference_scope,
                },
                &expected_revision,
                request_id.as_deref(),
            )
    })
    .await
}

#[tauri::command]
async fn project_delete_asset(
    state: tauri::State<'_, SharedCore>,
    asset_id: String,
    expected_revision: String,
    request_id: Option<String>,
) -> Result<(), String> {
    with_core(state, move |core| {
        core.project(trusted_shell())?
            .delete_asset_with_request(asset_id, &expected_revision, request_id.as_deref())
    })
    .await
}

#[tauri::command]
async fn project_map_location_projection(
    state: tauri::State<'_, SharedCore>,
    map_entity_id: String,
) -> Result<Vec<serde_json::Value>, String> {
    with_read_project(state, move |project| {
        project.map_location_projection(map_entity_id)
    })
    .await
}

#[tauri::command]
async fn project_query_map_locations(
    state: tauri::State<'_, SharedCore>,
    map_entity_id: String,
    min_x: f64,
    min_y: f64,
    max_x: f64,
    max_y: f64,
) -> Result<Vec<serde_json::Value>, String> {
    with_read_project(state, move |project| {
        project.query_map_locations(map_entity_id, min_x, min_y, max_x, max_y)
    })
    .await
}

#[tauri::command]
async fn project_backup(
    app: tauri::AppHandle,
    state: tauri::State<'_, SharedCore>,
) -> Result<String, String> {
    let directory = app
        .path()
        .app_data_dir()
        .map_err(|error| error.to_string())?;
    flush_project_checkpoint(state.clone(), "portable backup").await?;
    with_read_project(state, move |project| {
        project.portable_backup_after_checkpoint(directory)
    })
    .await
}

#[tauri::command]
async fn project_export_markdown(
    state: tauri::State<'_, SharedCore>,
    destination: String,
) -> Result<String, String> {
    with_read_project(state, move |project| {
        project.export_markdown_to(destination)
    })
    .await
}

#[tauri::command]
async fn project_recovery_backup(
    app: tauri::AppHandle,
    state: tauri::State<'_, SharedCore>,
) -> Result<String, String> {
    let directory = app
        .path()
        .app_data_dir()
        .map_err(|error| error.to_string())?;
    with_core(state, move |core| {
        core.project_mut(trusted_shell())?
            .recovery_backup_to(directory)
    })
    .await
}

#[tauri::command]
async fn project_restore_recovery_backup(
    state: tauri::State<'_, SharedCore>,
    path: String,
) -> Result<(), String> {
    with_core(state, move |core| {
        core.project_mut(trusted_shell())?
            .restore_recovery_backup(path)
    })
    .await
}

#[tauri::command]
async fn project_restore(state: tauri::State<'_, SharedCore>, path: String) -> Result<(), String> {
    with_core(state, move |core| {
        core.project_mut(trusted_shell())?.restore(path)
    })
    .await
}

#[tauri::command]
async fn project_restore_payload(
    state: tauri::State<'_, SharedCore>,
    payload: String,
    request_id: Option<String>,
) -> Result<(), String> {
    with_core(state, move |core| {
        core.project_mut(trusted_shell())?
            .restore_payload_with_request(&payload, request_id.as_deref())
    })
    .await
}

#[tauri::command]
async fn project_rebuild_search(state: tauri::State<'_, SharedCore>) -> Result<(), String> {
    with_core(state, |core| {
        core.project(trusted_shell())?.rebuild_search()
    })
    .await
}

#[tauri::command]
async fn project_seed_example(state: tauri::State<'_, SharedCore>) -> Result<usize, String> {
    with_core(state, |core| {
        core.project_mut(trusted_shell())?.seed_example()
    })
    .await
}

#[tauri::command]
async fn migration_validate(
    state: tauri::State<'_, SharedCore>,
    module_id: String,
    migration: serde_json::Value,
) -> Result<(), String> {
    let migration: Migration =
        serde_json::from_value(migration).map_err(|error| error.to_string())?;
    if migration.module_id != module_id {
        return Err("migration module ID does not match command module ID".into());
    }
    with_core(state, move |core| {
        let project = core.project(trusted_shell())?;
        let current = project.get_module_version(&module_id)?;
        project.validate_migration(&migration, current)
    })
    .await
}

#[tauri::command]
async fn migration_apply(
    state: tauri::State<'_, SharedCore>,
    module_id: String,
    migration: serde_json::Value,
) -> Result<(), String> {
    let migration: Migration =
        serde_json::from_value(migration).map_err(|error| error.to_string())?;
    if migration.module_id != module_id {
        return Err("migration module ID does not match command module ID".into());
    }
    with_core(state, move |core| {
        core.project_mut(trusted_shell())?
            .apply_migration(&migration)
    })
    .await
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let core = new_shared_core();
    let plugins = Arc::new(Mutex::new(
        bundled_plugin_host(core.clone())
            .expect("canonical bundled plugin manifests must validate"),
    ));
    let protocol_core = core.clone();
    let protocol_plugins = plugins.clone();
    let transfers = Arc::new(Mutex::new(BinaryTransferManager::default()));
    let protocol_transfers = transfers.clone();
    let physical_jobs = Arc::new(Mutex::new(PhysicalJobManager::default()));
    let atlas_jobs = Arc::new(Mutex::new(atlas_jobs::AtlasJobManager::default()));
    let _ = ATLAS_JOBS.set(atlas_jobs.clone());
    let atlas_studio = Arc::new(Mutex::new(atlas_studio::AtlasStudioManager::default()));
    let _ = ATLAS_STUDIO.set(atlas_studio.clone());
    let external_imports = Arc::new(Mutex::new(
        external_import_jobs::ExternalImportJobManager::default(),
    ));
    let _ = EXTERNAL_IMPORTS.set(external_imports.clone());
    let protocol_studio = atlas_studio.clone();
    let protocol_studio_core = core.clone();
    let protocol_ai_runtime = Arc::new(Mutex::new(ai::AiRuntime::default()));
    let startup_plugins = plugins.clone();
    let watcher = Arc::new(Mutex::new(ProjectWatcher::default()));
    let ai_runtime = protocol_ai_runtime.clone();
    let close_core = core.clone();
    let close_jobs = physical_jobs.clone();
    let close_plugins = plugins.clone();
    let close_watcher = watcher.clone();
    let close_ai_runtime = ai_runtime.clone();
    tauri::Builder::default()
        .setup(move |app| {
            let _ = APP_HANDLE.set(app.handle().clone());
            let app_data = app
                .path()
                .app_data_dir()
                .map_err(|error| error.to_string())?;
            app.manage(Arc::new(Mutex::new(SettingsStore::new(&app_data))) as SharedSettings);
            let install_root = app_data.join("plugins");
            let state_path = app_data.join("plugin-state.json");
            let rejected = startup_plugins
                .lock()
                .map_err(|_| "plugin host lock poisoned".to_string())?
                .load_installed_packages(
                    install_root,
                    state_path,
                    ArchiveLimits::default(),
                    VerificationPolicy::default(),
                )
                .map_err(|error| error.to_string())?;
            for error in rejected {
                eprintln!("ignoring rejected installed plugin package: {error}");
            }
            Ok(())
        })
        .on_window_event(move |window, event| {
            if window.label() != "main" {
                return;
            }
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                if let Err(error) = close_project_for_app(
                    window.app_handle(),
                    &close_core,
                    &close_jobs,
                    &close_plugins,
                    &close_ai_runtime,
                    &close_watcher,
                ) {
                    eprintln!("project cleanup during window close failed: {error}");
                }
                let _ = window.destroy();
            }
        })
        .register_uri_scheme_protocol("plugin", move |ctx, request| {
            plugin_protocol_response(
                ctx.webview_label(),
                &request,
                &protocol_core,
                &protocol_plugins,
                &protocol_transfers,
                &protocol_ai_runtime,
            )
        })
        .register_asynchronous_uri_scheme_protocol(
            "atlas-studio",
            move |ctx, request, responder| {
                let studio = protocol_studio.clone();
                let core = protocol_studio_core.clone();
                let label = ctx.webview_label().to_string();
                tauri::async_runtime::spawn_blocking(move || {
                    responder.respond(atlas_studio::protocol_response(
                        &studio, &core, &request, &label,
                    ));
                });
            },
        )
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .manage(core)
        .manage(plugins)
        .manage(transfers)
        .manage(physical_jobs)
        .manage(atlas_jobs)
        .manage(atlas_studio)
        .manage(external_imports)
        .manage(watcher)
        .manage(ai_runtime)
        .invoke_handler(tauri::generate_handler![
            greet,
            settings_get,
            settings_update,
            ai::ai_provider_status,
            ai::ai_provider_models,
            ai::ai_provider_credential_status,
            ai::ai_provider_import_credential,
            ai::ai_remote_set_consent,
            ai::ai_index_status,
            ai::ai_index_rebuild,
            ai::ai_index_cancel,
            ai::ai_index_search,
            ai::ai_generate_text,
            ai::ai_generate_structured,
            ai::ai_cancel_text,
            ai::ai_poll_text,
            git_tool_info,
            open_external_url,
            plugin_bootstrap,
            plugin_rpc,
            plugin_open_webview,
            plugin_mount_webview,
            plugin_resize_webview,
            plugin_unmount_webview,
            plugin_host_view_data,
            plugin_host_view_set_field,
            plugin_host_invoke_command,
            plugin_close_webview,
            plugin_close_all_webviews,
            module_list_manifests,
            module_schema_overlay_get,
            module_schema_editor_load,
            module_schema_overlay_set,
            module_enable,
            module_disable,
            plugin_install_package,
            plugin_upgrade,
            plugin_upgrade_plan,
            plugin_rollback,
            plugin_uninstall_code,
            plugin_delete_data,
            plugin_admin_view,
            plugin_retry,
            trusted_module_rpc,
            project_open,
            project_open_directory,
            project_new,
            project_close,
            project_info,
            project_import_checkpoint,
            external_import_jobs::project_external_importers,
            external_import_jobs::project_external_import_select_source,
            external_import_jobs::project_external_import_analyze_begin,
            external_import_jobs::project_external_import_analysis_status,
            external_import_jobs::project_external_import_analysis_cancel,
            external_import_jobs::project_external_import_analysis_page,
            external_import_jobs::project_external_import_candidate_plan,
            project_save_recovery_copy,
            project_git_status,
            project_git_preflight,
            project_git_staging_preview,
            project_git_init,
            project_git_log,
            project_git_commit,
            project_git_super_squash,
            project_git_show_tree,
            project_git_show_message,
            project_git_show_changes,
            project_git_show_diff,
            project_git_worktree_diff,
            project_git_show_file,
            project_git_reset_hard,
            project_git_remote_list,
            project_git_remote_add,
            project_git_remote_set_url,
            project_git_remote_remove,
            project_git_push,
            project_git_restore_from_upstream,
            project_open_memory,
            project_open_default,
            project_create_entity,
            project_create_map,
            project_list_entities,
            project_search,
            project_update_entity,
            project_delete_entity,
            project_save_document,
            project_save_entry,
            project_list_documents,
            project_set_field,
            project_list_fields,
            project_create_relationship,
            project_update_relationship,
            project_list_relationships,
            project_list_map_locations,
            project_upsert_map_location,
            project_unlink_map_location,
            maps_navigation,
            maps_editor_save,
            maps_editor_capture_anchor,
            maps_editor_start_pick,
            maps_editor_set_overlay,
            maps_editor_set_date,
            maps_editor_focus_link,
            maps_recovery_list,
            maps_recovery_restore,
            project_register_asset,
            project_register_asset_file,
            project_list_assets,
            project_import_image_map_file,
            project_accept_vector_map,
            project_physical_generate,
            project_physical_status,
            project_physical_cancel,
            project_physical_preview,
            project_physical_climate,
            project_physical_evolution,
            project_physical_hydrology,
            project_physical_accept,
            project_replace_vector_source,
            project_create_vector_layer,
            project_delete_vector_layer,
            maps_recovery_export,
            project_physical_derived_geojson,
            project_physical_derived_climate,
            project_physical_derived_evolution,
            project_physical_derived_hydrology,
            project_physical_derived_epoch,
            project_physical_materialize_events,
            project_physical_clear_epoch_cache,
            atlas_jobs::project_atlas_capabilities,
            atlas_jobs::project_atlas_preview_begin,
            atlas_jobs::project_atlas_render_begin,
            atlas_jobs::project_atlas_job_status,
            atlas_jobs::project_atlas_job_cancel,
            atlas_jobs::project_atlas_artifact_save,
            atlas_jobs::project_atlas_artifact_discard,
            atlas_studio::project_atlas_studio_open,
            atlas_studio::project_atlas_studio_close,
            atlas_studio::project_atlas_studio_status,
            atlas_studio::project_atlas_studio_regenerate_cache,
            atlas_studio::project_atlas_studio_inspect,
            project_read_asset_bytes,
            project_create_raster_layer,
            project_create_semantic_layer,
            project_update_map_layer,
            project_delete_raster_layer,
            project_delete_semantic_layer,
            project_replace_asset_bytes,
            project_replace_asset_file,
            project_update_asset_metadata,
            project_delete_asset,
            project_map_location_projection,
            project_query_map_locations,
            project_backup,
            project_export_markdown,
            project_recovery_backup,
            project_restore_recovery_backup,
            project_restore,
            project_restore_payload,
            project_rebuild_search,
            project_seed_example,
            migration_validate,
            migration_apply,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(test)]
mod tests;
