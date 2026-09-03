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
mod broker;
mod external_import_jobs;
mod host;
mod image_generation;
mod maps_nav;
mod module_commands;
mod physical_commands;
mod physical_jobs;
mod plugin_commands;
mod plugin_webview;
mod project_commands;
mod settings;
mod transfer;
mod version;
mod watcher;

use self::broker::*;
use self::host::*;
use self::maps_nav::*;
use self::module_commands::*;
use self::physical_commands::*;
use self::physical_jobs::*;
use self::plugin_commands::*;
use self::plugin_webview::*;
use self::project_commands::*;
use self::transfer::*;
use self::watcher::*;
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
            ai::ai_provider_connect,
            ai::ai_provider_credential_status,
            ai::ai_provider_import_credential,
            ai::ai_provider_set_credential,
            ai::ai_provider_clear_credential,
            project_set_ai_enabled,
            project_ai_prompts_get,
            project_ai_prompts_set,
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
