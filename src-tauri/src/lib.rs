// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Component, Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{mpsc, Arc, Mutex, OnceLock};
use std::thread;
use std::time::{Duration, Instant};

use daena_core::{
    read_json, set_checkpoint_export_status_listener, validate_checkpoint, Asset, AssetFileInput,
    AssetFileReplaceInput, AssetInput, AssetMetadataUpdate, AssetReplaceInput, AuthorityContext,
    CheckpointHandle, CheckpointManifest, CoreError, CoreService, CreateEntity, CreateEntry,
    CreateEntryField, CreateEntryRelationship, Entity, EntityListQuery, EntityPage,
    EntitySortDirection, EntitySortField, ExternalChangeReport, FieldValue, GitLogEntry,
    GitPreflight, GitRemote, GitResetResult, GitStatus, GitToolInfo, Migration, Operation,
    ProjectInfo, ProjectStore, Relationship, RelationshipInput, SaveDocument, SaveEntry,
    WikiPageExportFormat,
};
use daena_plugin_api::{
    merge_module_manifest, parse_module_overlay, supports_schema_overlay, CommandAction,
    MetadataFieldDefinition, MigrationOperation, ModuleSchemaEditorState, ModuleSchemaOverlay,
    ModuleSchemaOverlayMutationResult, PluginManifest, RpcRequest, RpcResponse,
    SchemaOverlayPreviewResult, ViewComponent, SCHEMA_OVERLAY_VERSION,
};
use daena_plugin_host::{
    plugin_window_label, webview_policy, ArchiveLimits, DependencyResolver, PluginHost, Session,
    VerificationPolicy, BUNDLED_TIMELINE_SERVICE_WASM,
};
use notify::{RecommendedWatcher, RecursiveMode, Watcher};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tauri::{Emitter, Manager};
use tauri_plugin_dialog::DialogExt;

mod ai;
mod app_update;
mod atlas_jobs;
mod atlas_studio;
mod external_import_jobs;
mod image_generation;
mod physical_commands;
mod physical_jobs;
mod plugin_webview;
mod project_commands;
mod settings;
mod version;

use self::physical_commands::*;
use self::physical_jobs::*;
use self::plugin_webview::*;
use self::project_commands::*;
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

fn cancel_external_import_jobs() -> Result<(), String> {
    if let Some(imports) = EXTERNAL_IMPORTS.get() {
        external_import_jobs::cancel_external_imports(imports)?;
    }
    Ok(())
}

fn cancel_image_generation_jobs(
    jobs: &crate::image_generation::SharedImageGeneration,
) -> Result<(), String> {
    jobs.lock()
        .map_err(|_| "image generation state is unavailable".to_string())?
        .cancel_all();
    Ok(())
}

const MAX_ASSET_TRANSFER_BYTES: usize = daena_core::maps::PHYSICAL_MAX_SOURCE_BYTES;
const ASSET_TRANSFER_TTL: Duration = Duration::from_secs(60);

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
            .is_some_and(|info| info.sync.state == "pending");
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
        || relative
            .components()
            .any(|component| matches!(component.as_os_str().to_str(), Some(".daena" | ".git")))
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
            .map_or_else(|| package_root.to_path_buf(), Path::to_path_buf);
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
            Some("js" | "mjs") => "text/javascript",
            Some("css") => "text/css",
            Some("json") => "application/json",
            Some("svg") => "image/svg+xml",
            Some("png") => "image/png",
            Some("jpg" | "jpeg") => "image/jpeg",
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
            "daena.houses" => include_str!("../../packages/modules/houses/manifest.json"),
            _ => return None,
        };
        serde_json::from_str::<PluginManifest>(manifest).ok()
    });
    let csp = manifest
        .as_ref()
        .and_then(webview_policy)
        .map_or_else(|| "default-src 'none'".into(), |policy| policy.csp);
    tauri::http::Response::builder()
        .status(200)
        .header("Content-Type", content_type)
        .header("Content-Security-Policy", csp)
        .header("X-Content-Type-Options", "nosniff")
        .body(bytes)
        .unwrap()
}

fn plugin_icon_response(
    webview_label: &str,
    request: &tauri::http::Request<Vec<u8>>,
    core: &SharedCore,
    plugins: &SharedPluginHost,
) -> tauri::http::Response<Vec<u8>> {
    if webview_label != "main" || request.method() != tauri::http::Method::GET {
        return tauri::http::Response::builder()
            .status(403)
            .body(Vec::new())
            .unwrap();
    }
    let plugin_id = request.uri().host().unwrap_or_default();
    let Some(relative) = request.uri().path().strip_prefix('/') else {
        return tauri::http::Response::builder()
            .status(404)
            .body(Vec::new())
            .unwrap();
    };
    if plugin_id.is_empty()
        || relative.is_empty()
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
    let project_id = current_info(core).ok().flatten().map(|info| info.root);
    let entry = project_id.and_then(|project_id| {
        plugins
            .lock()
            .ok()
            .and_then(|host| host.runtime_entry(&project_id, plugin_id))
    });
    let Some(entry) = entry else {
        return tauri::http::Response::builder()
            .status(404)
            .body(Vec::new())
            .unwrap();
    };
    if entry.package_root.as_os_str().is_empty()
        || !daena_plugin_host::manifest_svg_icon_paths(&entry.manifest).contains(relative)
    {
        return tauri::http::Response::builder()
            .status(404)
            .body(Vec::new())
            .unwrap();
    }
    let Ok(canonical_root) = entry.package_root.canonicalize() else {
        return tauri::http::Response::builder()
            .status(404)
            .body(Vec::new())
            .unwrap();
    };
    let Ok(canonical_icon) = entry.package_root.join(relative).canonicalize() else {
        return tauri::http::Response::builder()
            .status(404)
            .body(Vec::new())
            .unwrap();
    };
    if !canonical_icon.starts_with(canonical_root) || !canonical_icon.is_file() {
        return tauri::http::Response::builder()
            .status(404)
            .body(Vec::new())
            .unwrap();
    }
    let Ok(bytes) = std::fs::read(canonical_icon) else {
        return tauri::http::Response::builder()
            .status(404)
            .body(Vec::new())
            .unwrap();
    };
    if daena_plugin_host::validate_icon_svg(&bytes).is_err() {
        return tauri::http::Response::builder()
            .status(404)
            .body(Vec::new())
            .unwrap();
    }
    tauri::http::Response::builder()
        .status(200)
        .header("Content-Type", "image/svg+xml")
        .header("Content-Security-Policy", "default-src 'none'")
        .header("X-Content-Type-Options", "nosniff")
        .header("Cache-Control", "public, max-age=31536000, immutable")
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
    match request
        .headers()
        .get("Origin")
        .and_then(|value| value.to_str().ok())
    {
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
                if current_project.as_deref() == Some(project_id) {
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
                } else {
                    Err("plugin bootstrap project mismatch".into())
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
                    let shared_field_keys = {
                        let host = plugins
                            .lock()
                            .map_err(|_| "plugin host lock poisoned".to_string())?;
                        shared_field_keys_for_request(
                            &host,
                            &plugin_id,
                            &request.method,
                            &request.payload,
                        )?
                    };
                    let core_session = current_session(core)?;
                    let mut core = core_session
                        .core
                        .lock()
                        .map_err(|_| "core lock poisoned".to_string())?;
                    dispatch_module_rpc(
                        &mut core,
                        Some(&plugin_id),
                        shared_field_keys,
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
        Err(error) => {
            json_response_with_origin(serde_json::json!({"error": error}), 400, &plugin_origin)
        }
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
                return Err("replacement target is not a daena-openlayers source asset".into());
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
        .map_or_else(|| ProjectStore::open_read_only(&root), Ok)?;
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
    let shared_field_keys = {
        let host = state
            .lock()
            .map_err(|_| "plugin host lock poisoned".to_string())?;
        shared_field_keys_for_request(&host, &plugin_id, &method, &payload)?
    };
    let current_project = current_info(&core)?.map(|info| info.root);
    let event_project_id = session.project_id.clone();
    let result = if method == "app.version" {
        Ok(serde_json::json!({ "version": crate::version::current() }))
    } else if current_project.as_deref() != Some(session.project_id.as_str()) {
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
                shared_field_keys,
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
            if let Some(core) = context.core.as_ref() {
                if current_info(core)?.is_some() {
                    ai::ensure_active_project(core, &context.caller.project_id)?;
                }
            }
            crate::ensure_project_ai_enabled(&context.caller.project_id)?;
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
                .unwrap_or(daena_ai::DEFAULT_GENERATION_DEADLINE.as_millis() as u64)
                .clamp(1, daena_ai::MAX_GENERATION_DEADLINE.as_millis() as u64);
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
                        .map_err(|error| error.clone())?,
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

fn relationship_constraints_for_project(
    project: &ProjectStore,
    host: &PluginHost,
) -> Result<BTreeMap<String, daena_plugin_api::RelationshipConstraints>, CoreError> {
    let project_id = project.info().map(|info| info.root);
    let mut entries = host
        .catalog
        .list()
        .map(|entry| entry.manifest.id.clone())
        .collect::<Vec<_>>();
    entries.sort();
    let mut merged: BTreeMap<String, daena_plugin_api::RelationshipConstraints> = BTreeMap::new();
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
                let Some(relationship_type) = field.relationship_type.as_deref() else {
                    continue;
                };
                let constraints = daena_plugin_api::RelationshipConstraints::from_field(field);
                if let Some(existing) = merged.get(relationship_type) {
                    if existing != &constraints {
                        return Err(CoreError::Validation(format!(
                            "conflicting relationship constraints for {relationship_type}"
                        )));
                    }
                } else {
                    merged.insert(relationship_type.to_owned(), constraints);
                }
            }
        }
    }
    Ok(merged)
}

fn apply_relationship_runtime_schemas(
    project: &mut ProjectStore,
    host: &PluginHost,
) -> Result<(), CoreError> {
    project.set_relationship_metadata_schemas(relationship_metadata_schemas_for_project(
        project, host,
    )?)?;
    project.set_relationship_constraints(relationship_constraints_for_project(project, host)?)
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
    apply_relationship_runtime_schemas(project, &host).map_err(|error| error.to_string())
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
                states.get(&manifest.id).map_or_else(
                    || manifest.enabled_by_default.unwrap_or(true),
                    |(enabled, _)| *enabled,
                )
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
fn app_version() -> String {
    crate::version::current().to_string()
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
        (
            include_str!("../../packages/modules/houses/manifest.json"),
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
                from: i64::from(migration.from),
                to: i64::from(migration.to),
                operations,
                recovery: migration.recovery.clone(),
                package_digest: package_digest.into(),
            })
        })
        .collect()
}

pub(crate) fn effective_module_manifests(
    project: &ProjectStore,
    host: &PluginHost,
) -> Result<Vec<(PluginManifest, bool)>, CoreError> {
    let project_id = project.info().map(|info| info.root);
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
        let mut manifest = entry.manifest.clone();
        if supports_schema_overlay(&entry.manifest) {
            let overlay_value = project
                .module_schema_overlay(&id)?
                .unwrap_or_else(|| serde_json::json!({}));
            let overlay = parse_module_overlay(&overlay_value).map_err(CoreError::Validation)?;
            manifest =
                merge_module_manifest(&entry.manifest, &overlay).map_err(CoreError::Validation)?;
        }
        manifests.push((manifest, project.is_module_enabled(&id)?));
    }
    Ok(manifests)
}

#[tauri::command]
async fn module_list_manifests(
    state: tauri::State<'_, SharedCore>,
    plugins: tauri::State<'_, SharedPluginHost>,
) -> Result<Vec<serde_json::Value>, String> {
    let plugins = plugins.inner().clone();
    with_read_project(state, move |project| {
        let host = plugins
            .lock()
            .map_err(|_| CoreError::Conflict("plugin host lock poisoned".into()))?;
        effective_module_manifests(project, &host)?
            .into_iter()
            .map(|(manifest_struct, enabled)| {
                let mut manifest = serde_json::to_value(&manifest_struct)
                    .map_err(|error| CoreError::Validation(error.to_string()))?;
                manifest
                    .as_object_mut()
                    .expect("manifest is an object")
                    .insert("enabled".into(), serde_json::Value::Bool(enabled));
                Ok(manifest)
            })
            .collect()
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
        let revision = project.revision_for_module_schema_overlay(&module_id)?;
        Ok(ModuleSchemaEditorState {
            id: package.id.clone(),
            name: package.name.clone(),
            schemas: package.schemas.clone(),
            templates: package.templates.clone(),
            overlay,
            revision,
        })
    })
    .await
}

#[tauri::command]
async fn module_schema_overlay_preview(
    state: tauri::State<'_, SharedCore>,
    plugins: tauri::State<'_, SharedPluginHost>,
    module_id: String,
    overlay: ModuleSchemaOverlay,
) -> Result<SchemaOverlayPreviewResult, String> {
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
        project.preview_module_schema_overlay(&module_id, &package, &overlay)
    })
    .await
}

#[tauri::command]
async fn module_schema_overlay_set(
    state: tauri::State<'_, SharedCore>,
    plugins: tauri::State<'_, SharedPluginHost>,
    module_id: String,
    overlay: ModuleSchemaOverlay,
    expected_revision: Option<String>,
    request_id: Option<String>,
    acknowledge_impact: Option<bool>,
) -> Result<ModuleSchemaOverlayMutationResult, String> {
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
        daena_plugin_api::qualify_module_overlay(&package, &mut normalized)
            .map_err(CoreError::Validation)?;
        daena_plugin_api::validate_module_overlay(&package, &normalized)
            .map_err(CoreError::Validation)?;
        let preview = project.preview_module_schema_overlay(&module_id, &package, &normalized)?;
        if !preview.ok {
            let message = preview
                .errors
                .first()
                .map(|issue| issue.message.clone())
                .unwrap_or_else(|| "schema overlay preview failed".into());
            return Err(CoreError::Validation(message));
        }
        if preview.requires_acknowledgement && !acknowledge_impact.unwrap_or(false) {
            return Err(CoreError::Validation(
                "schema overlay has live-data impact; preview and acknowledge before saving".into(),
            ));
        }
        let value = if normalized.is_empty() {
            None
        } else {
            Some(
                serde_json::to_value(&normalized)
                    .map_err(|error| CoreError::Validation(error.to_string()))?,
            )
        };
        let revision = project.set_module_schema_overlay_with_request(
            module_id,
            value,
            expected_revision.as_deref(),
            request_id.as_deref(),
        )?;
        let host = plugins
            .lock()
            .map_err(|_| CoreError::Conflict("plugin host lock poisoned".into()))?;
        apply_relationship_runtime_schemas(project, &host)?;
        Ok(ModuleSchemaOverlayMutationResult {
            overlay: normalized,
            revision,
        })
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
            .map_err(|error| CoreError::Validation(error.clone()))?;
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
        apply_relationship_runtime_schemas(project, &host)?;
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
        apply_relationship_runtime_schemas(project, &host)?;
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
            i64::from(current),
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
            .map_err(|error| CoreError::Validation(error.clone()))?
            .into_iter()
            .filter(|migration| migration.from >= i64::from(current))
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
        apply_relationship_runtime_schemas(project, &host)?;
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
            i64::from(current),
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
        apply_relationship_runtime_schemas(project, &host)?;
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
                for version in &mut versions {
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
        "entity.query" => (
            &[],
            &[
                "query",
                "entityTypes",
                "excludedEntityTypes",
                "sortField",
                "sortDirection",
                "offset",
                "limit",
            ],
        ),
        "entity.get" => (&["id"], &[]),
        "entity.getMany" => (&["ids"], &[]),
        "entity.create" => (&["name"], &["type", "fields", "relationships", "document"]),
        "entity.update" => (&["id", "expectedRevision"], &["name", "type"]),
        "entity.delete" => (&["id", "expectedRevision"], &[]),
        "document.list" => (&["entityId"], &[]),
        "document.save" => (&["entityId", "body", "expectedRevision"], &["format"]),
        "field.read" => (&["entityId", "namespace", "key"], &[]),
        "field.list" => (&["entityId", "namespace"], &["sharedOnly"]),
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
        "relationship.query" => (
            &["entityIds", "direction"],
            &["relationshipTypes", "offset", "limit"],
        ),
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
    if method == "field.list"
        && object
            .get("sharedOnly")
            .is_some_and(|value| !value.is_null() && !value.is_boolean())
    {
        return Err(CoreError::Validation(
            "plugin RPC payload for field.list requires sharedOnly to be boolean".into(),
        ));
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

fn shared_field_keys_for_request(
    host: &PluginHost,
    plugin_id: &str,
    method: &str,
    payload: &serde_json::Value,
) -> Result<Option<std::collections::BTreeSet<String>>, String> {
    if method != "field.list" {
        return Ok(None);
    }
    let namespace = payload
        .get("namespace")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| "field list payload requires namespace".to_string())?;
    let shared_only = payload
        .get("sharedOnly")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false);
    if !shared_only && host.namespaces.owner(namespace) == Some(plugin_id) {
        Ok(None)
    } else {
        Ok(Some(host.namespaces.shared_field_keys(namespace)))
    }
}

fn dispatch_module_rpc(
    core: &mut CoreService,
    plugin_id: Option<&str>,
    shared_field_keys: Option<std::collections::BTreeSet<String>>,
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
            let entities = if let Some(entity_type) = entity_type {
                let mut entities = Vec::new();
                let mut offset = 0_u64;
                loop {
                    let page = project.query_entities(EntityListQuery {
                        entity_types: vec![entity_type.to_owned()],
                        offset: Some(offset),
                        limit: Some(daena_core::MAX_ENTITY_QUERY_LIMIT),
                        ..EntityListQuery::default()
                    })?;
                    entities.extend(page.items);
                    if !page.has_more {
                        break;
                    }
                    offset = offset.saturating_add(u64::from(page.limit));
                }
                entities
            } else {
                project.list_entities()?
            };
            serde_json::to_value(entities).map_err(|error| CoreError::Validation(error.to_string()))
        }
        "entity.query" => {
            let payload: daena_plugin_api::EntityQueryPayload = serde_json::from_value(payload)
                .map_err(|error| {
                    CoreError::Validation(format!("invalid entity.query payload: {error}"))
                })?;
            let sort_field = match payload.sort_field.as_deref() {
                None => None,
                Some("name") => Some(EntitySortField::Name),
                Some("createdAt") | Some("created_at") => Some(EntitySortField::CreatedAt),
                Some("updatedAt") | Some("updated_at") => Some(EntitySortField::UpdatedAt),
                Some("relevance") => Some(EntitySortField::Relevance),
                Some(other) => {
                    return Err(CoreError::Validation(format!(
                        "unsupported entity sort field: {other}"
                    )))
                }
            };
            let sort_direction = match payload.sort_direction.as_deref() {
                None => None,
                Some("asc") => Some(EntitySortDirection::Asc),
                Some("desc") => Some(EntitySortDirection::Desc),
                Some(other) => {
                    return Err(CoreError::Validation(format!(
                        "unsupported entity sort direction: {other}"
                    )))
                }
            };
            let page = project.query_entities(EntityListQuery {
                query: payload.query,
                entity_types: payload.entity_types,
                excluded_entity_types: payload.excluded_entity_types,
                sort_field,
                sort_direction,
                offset: payload.offset,
                limit: payload.limit,
                ..EntityListQuery::default()
            })?;
            let record = daena_plugin_api::EntityPageRecord {
                items: page
                    .items
                    .into_iter()
                    .map(|entity| daena_plugin_api::EntityRecord {
                        id: entity.id,
                        name: entity.name,
                        entity_type: entity.entity_type,
                        deleted: entity.deleted,
                        created_at: entity.created_at,
                        updated_at: entity.updated_at,
                        revision: entity.revision,
                    })
                    .collect(),
                total: page.total,
                offset: page.offset,
                limit: page.limit,
                has_more: page.has_more,
                type_counts: page
                    .type_counts
                    .into_iter()
                    .map(|count| daena_plugin_api::EntityTypeCountRecord {
                        entity_type: count.entity_type,
                        count: count.count,
                    })
                    .collect(),
            };
            serde_json::to_value(record).map_err(|error| CoreError::Validation(error.to_string()))
        }
        "entity.get" => {
            let id = payload_string(&payload, "id")?;
            let entity = project.get_entity(&id)?;
            serde_json::to_value(entity).map_err(|error| CoreError::Validation(error.to_string()))
        }
        "entity.getMany" => {
            let payload: daena_plugin_api::EntityGetManyPayload = serde_json::from_value(payload)
                .map_err(|error| {
                CoreError::Validation(format!("invalid entity.getMany payload: {error}"))
            })?;
            serde_json::to_value(project.get_entities(&payload.ids)?)
                .map_err(|error| CoreError::Validation(error.to_string()))
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
            let mut fields = project
                .list_fields(entity_id)?
                .into_iter()
                .filter(|field| field.namespace == namespace)
                .filter(|field| {
                    shared_field_keys
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
            let limit = usize::try_from(
                payload
                    .get("limit")
                    .and_then(serde_json::Value::as_u64)
                    .unwrap_or(50),
            )
            .unwrap_or(usize::MAX);
            let offset = usize::try_from(
                payload
                    .get("offset")
                    .and_then(serde_json::Value::as_u64)
                    .unwrap_or_default(),
            )
            .unwrap_or(usize::MAX);
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
        "relationship.query" => {
            let payload: daena_plugin_api::RelationshipQueryPayload =
                serde_json::from_value(payload).map_err(|error| {
                    CoreError::Validation(format!("invalid relationship.query payload: {error}"))
                })?;
            let direction = match payload.direction {
                daena_plugin_api::RelationshipQueryDirection::Incoming => {
                    daena_core::RelationshipQueryDirection::Incoming
                }
                daena_plugin_api::RelationshipQueryDirection::Outgoing => {
                    daena_core::RelationshipQueryDirection::Outgoing
                }
                daena_plugin_api::RelationshipQueryDirection::Any => {
                    daena_core::RelationshipQueryDirection::Any
                }
            };
            let page = project.query_relationships(daena_core::RelationshipQuery {
                entity_ids: payload.entity_ids,
                relationship_types: payload.relationship_types,
                direction,
                offset: payload.offset,
                limit: payload.limit,
            })?;
            let record = daena_plugin_api::RelationshipPageRecord {
                items: page
                    .items
                    .into_iter()
                    .map(|relationship| daena_plugin_api::RelationshipRecord {
                        id: relationship.id,
                        source_id: relationship.source_id,
                        target_id: relationship.target_id,
                        relationship_type: relationship.relationship_type,
                        metadata: relationship.metadata,
                        revision: relationship.revision,
                    })
                    .collect(),
                total: page.total,
                offset: page.offset,
                limit: page.limit,
                has_more: page.has_more,
            };
            serde_json::to_value(record).map_err(|error| CoreError::Validation(error.to_string()))
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
            location.label.clone_from(&created.name);
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
    let shared_field_keys = if let Some(module_id) = plugin_id.as_deref() {
        let host = plugins
            .lock()
            .map_err(|_| "plugin host lock poisoned".to_string())?;
        shared_field_keys_for_request(&host, module_id, &method, &payload)?
    } else {
        None
    };
    let result = with_core(state, move |core| {
        dispatch_module_rpc(
            core,
            plugin_id.as_deref(),
            shared_field_keys,
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
    image_jobs: tauri::State<'_, image_generation::SharedImageGeneration>,
) -> Result<(), String> {
    cancel_physical_jobs(jobs.inner())?;
    cancel_external_import_jobs()?;
    let plugins = plugins.inner().clone();
    let ai_runtime = ai_runtime.inner().clone();
    let app = app.clone();
    let core = state.inner().clone();
    let jobs = jobs.inner().clone();
    let watcher = watcher.inner().clone();
    let image_jobs = image_jobs.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        close_project_for_app(
            &app,
            &core,
            &jobs,
            &plugins,
            &ai_runtime,
            &watcher,
            &image_jobs,
        )
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
    image_jobs: &image_generation::SharedImageGeneration,
) -> Result<(), String> {
    cancel_physical_jobs(jobs)?;
    cancel_external_import_jobs()?;
    cancel_image_generation_jobs(image_jobs)?;
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

/// Plugin RPC envelopes carry correlation-only `requestId`s (for example
/// `maps-request-N`). Transaction receipts are UUID-keyed, so only pass a
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

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let core = new_shared_core();
    let plugins = Arc::new(Mutex::new(
        bundled_plugin_host(core.clone())
            .expect("canonical bundled plugin manifests must validate"),
    ));
    let protocol_core = core.clone();
    let protocol_plugins = plugins.clone();
    let icon_protocol_core = core.clone();
    let icon_protocol_plugins = plugins.clone();
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
    let image_jobs = Arc::new(Mutex::new(
        image_generation::ImageGenerationManager::default(),
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
    let close_image_jobs = image_jobs.clone();
    tauri::Builder::default()
        .setup(move |app| {
            let _ = APP_HANDLE.set(app.handle().clone());
            set_checkpoint_export_status_listener(Some(Arc::new(|| {
                if let Some(app) = APP_HANDLE.get() {
                    let _ = app.emit("project-checkpoint-export-status", ());
                }
            })));
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
                    &close_image_jobs,
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
        .register_uri_scheme_protocol("plugin-icon", move |ctx, request| {
            plugin_icon_response(
                ctx.webview_label(),
                &request,
                &icon_protocol_core,
                &icon_protocol_plugins,
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
        .manage(image_jobs)
        .invoke_handler(tauri::generate_handler![
            greet,
            app_version,
            app_update::app_check_update,
            settings_get,
            settings_update,
            ai::ai_provider_status,
            ai::ai_provider_models,
            ai::ai_provider_credential_status,
            ai::ai_provider_import_credential,
            ai::ai_provider_set_credential,
            ai::ai_provider_clear_credential,
            project_set_ai_enabled,
            ai::ai_remote_set_consent,
            ai::ai_index_status,
            ai::ai_index_rebuild,
            ai::ai_index_cancel,
            ai::ai_index_search,
            ai::ai_generate_text,
            ai::ai_generate_structured,
            ai::ai_cancel_text,
            ai::ai_poll_text,
            image_generation::image_provider_status,
            image_generation::image_provider_discover,
            image_generation::image_generate_start,
            image_generation::image_generation_status,
            image_generation::image_generation_cancel,
            image_generation::image_candidate_bytes,
            image_generation::image_candidate_accept,
            image_generation::image_candidate_discard,
            image_generation::image_generation_discard,
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
            module_schema_overlay_preview,
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
            external_import_jobs::project_external_import_validate,
            external_import_jobs::project_external_import_commit,
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
            project_list_entities,
            project_get_entity,
            project_query_entities,
            project_search,
            project_search_map_features,
            project_update_entity,
            project_delete_entity,
            project_restore_entity,
            project_purge_entity,
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
            maps_recovery_list,
            maps_recovery_restore,
            project_register_asset,
            project_register_asset_file,
            project_list_assets,
            project_list_shared_assets,
            project_import_image_map_file,
            project_attach_map_raster_asset,
            project_duplicate_map_raster_asset,
            project_import_vector_map_file,
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
            project_apply_map_edit,
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
            project_read_asset_bytes_by_path,
            project_get_asset_by_path,
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
            project_export_wiki_page,
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
