// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/

use std::sync::{Arc, Mutex};

use tauri::Manager;
use worldbuilder_core::{
    Asset, AssetFileInput, AssetInput, AuthorityContext, CoreError, CoreService, CreateEntity,
    Entity, FieldValue, GitLogEntry, GitStatus, Migration, Operation, ProjectInfo, Relationship,
    RelationshipInput, SaveDocument, SaveEntry,
};
use worldbuilder_plugin_api::{PluginManifest, RpcRequest, RpcResponse};
use worldbuilder_plugin_host::PluginHost;

type SharedCore = Arc<Mutex<CoreService>>;
type SharedPluginHost = Arc<Mutex<PluginHost>>;

fn trusted_shell() -> AuthorityContext {
    AuthorityContext::trusted_shell()
}

async fn with_core<T, F>(state: tauri::State<'_, SharedCore>, operation: F) -> Result<T, String>
where
    T: Send + 'static,
    F: FnOnce(&mut CoreService) -> Result<T, CoreError> + Send + 'static,
{
    let core = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let mut core = core
            .lock()
            .map_err(|_| CoreError::Conflict("core lock poisoned".into()))?;
        operation(&mut core)
    })
    .await
    .map_err(|error| format!("core worker failed: {error}"))?
    .map_err(|error| error.to_string())
}

#[derive(Debug, serde::Serialize)]
struct PluginBootstrap {
    session_id: String,
    plugin_id: String,
    package_digest: String,
    manifest: PluginManifest,
}

/// Bootstrap and RPC use a host-created `plugin:<id>` webview label as the
/// origin binding. The trusted main window cannot call this surface, and the
/// plugin cannot submit an arbitrary origin in its request payload.
#[tauri::command]
fn plugin_bootstrap(
    window: tauri::WebviewWindow,
    state: tauri::State<'_, SharedPluginHost>,
    plugin_id: String,
    project_id: String,
) -> Result<PluginBootstrap, String> {
    let origin = plugin_webview_identity(&window, Some(&plugin_id))?;
    let mut host = state
        .lock()
        .map_err(|_| "plugin host lock poisoned".to_string())?;
    let session = host
        .bootstrap(&plugin_id, &project_id, &origin)
        .map_err(|error| error.to_string())?;
    let entry = host
        .catalog
        .get(&plugin_id)
        .ok_or_else(|| "plugin was removed during bootstrap".to_string())?;
    Ok(PluginBootstrap {
        session_id: session.id,
        plugin_id,
        package_digest: entry.digest.clone(),
        manifest: entry.manifest.clone(),
    })
}

#[tauri::command]
fn plugin_rpc(
    window: tauri::WebviewWindow,
    state: tauri::State<'_, SharedPluginHost>,
    request: RpcRequest,
) -> Result<RpcResponse, String> {
    let origin = plugin_webview_identity(&window, None)?;
    let host = state
        .lock()
        .map_err(|_| "plugin host lock poisoned".to_string())?;
    Ok(host.rpc(&origin, &request))
}

fn plugin_webview_identity(
    window: &tauri::WebviewWindow,
    expected: Option<&str>,
) -> Result<String, String> {
    let label = window.label();
    let plugin_id = label
        .strip_prefix("plugin:")
        .filter(|value| !value.is_empty())
        .ok_or_else(|| "plugin RPC requires a plugin webview".to_string())?;
    if expected.is_some_and(|expected| expected != plugin_id) {
        return Err("plugin webview identity mismatch".into());
    }
    Ok(label.to_string())
}

fn bundled_manifests() -> Vec<serde_json::Value> {
    vec![
        serde_json::json!({
            "id": "worldbuilder.lore", "name": "Lore", "version": "0.1.0", "apiVersion": "1",
            "capabilities": ["entity.read", "entity.write", "document.read", "document.write", "relationship.read", "relationship.write", "asset.read", "asset.write", "search.query"],
            "schemas": [{"namespace": "lore", "entityTypes": ["person", "place", "faction", "artifact", "culture"], "fields": [
                {"key": "summary", "label": "Summary", "type": "text"}, {"key": "aliases", "label": "Aliases", "type": "text"}
            ]}],
            "templates": [
                {"id": "person", "name": "Person", "entityType": "person", "fields": {"summary": "", "aliases": ""}},
                {"id": "place", "name": "Place", "entityType": "place", "fields": {"summary": "", "aliases": ""}},
                {"id": "faction", "name": "Faction", "entityType": "faction", "fields": {"summary": "", "aliases": ""}}
            ],
            "migrations": [{"id": "lore-v1", "from": 0, "to": 1, "recovery": "backup", "operations": [{"kind": "create-namespace", "namespace": "lore"}] }]
        }),
        serde_json::json!({
            "id": "worldbuilder.timeline", "name": "Timeline", "version": "0.1.0", "apiVersion": "1",
            "capabilities": ["entity.read", "entity.write", "document.read", "document.write", "relationship.read", "relationship.write", "asset.read", "asset.write", "search.query"],
            "schemas": [{"namespace": "timeline", "entityTypes": ["event"], "fields": [
                {"key": "startsAt", "label": "Starts", "type": "date", "required": true}, {"key": "endsAt", "label": "Ends", "type": "date"}
            ]}],
            "templates": [{"id": "event", "name": "Timeline event", "entityType": "event", "fields": {"startsAt": "", "endsAt": ""}}],
            "migrations": [{"id": "timeline-v1", "from": 0, "to": 1, "recovery": "backup", "operations": [{"kind": "create-namespace", "namespace": "timeline"}] }]
        }),
    ]
}

#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {name}! You've been greeted from Rust!")
}

#[tauri::command]
async fn module_list_manifests(
    state: tauri::State<'_, SharedCore>,
) -> Result<Vec<serde_json::Value>, String> {
    with_core(state, |core| {
        let mut manifests = bundled_manifests();
        let context = trusted_shell();
        let Some(project) = core.project(context).ok() else {
            return Ok(manifests);
        };
        for manifest in &mut manifests {
            let id = manifest
                .get("id")
                .and_then(serde_json::Value::as_str)
                .unwrap_or_default();
            let enabled = project
                .is_module_enabled(id)
                .map_err(|error| CoreError::Validation(error.to_string()))?;
            manifest
                .as_object_mut()
                .expect("manifest is an object")
                .insert("enabled".into(), serde_json::Value::Bool(enabled));
        }
        Ok(manifests)
    })
    .await
}

#[tauri::command]
async fn module_enable(state: tauri::State<'_, SharedCore>, id: String) -> Result<(), String> {
    with_core(state, move |core| {
        if id != "worldbuilder.lore" && id != "worldbuilder.timeline" {
            return Err(CoreError::Validation("unknown module".into()));
        }
        let context = trusted_shell();
        let project = core.project_mut(context)?;
        if project.get_module_version(&id)? == 0 {
            let namespace = if id == "worldbuilder.lore" {
                "lore"
            } else {
                "timeline"
            };
            project.apply_migration(&Migration {
                id: format!("{id}-v1"),
                module_id: id.clone(),
                from: 0,
                to: 1,
                operations: vec![Operation::CreateNamespace {
                    namespace: namespace.into(),
                }],
                recovery: "backup".into(),
            })?;
        }
        project.set_module_enabled(id, true)?;
        Ok(())
    })
    .await
}

#[tauri::command]
async fn module_disable(state: tauri::State<'_, SharedCore>, id: String) -> Result<(), String> {
    with_core(state, move |core| {
        if id != "worldbuilder.lore" && id != "worldbuilder.timeline" {
            return Err(CoreError::Validation("unknown module".into()));
        }
        core.project(trusted_shell())?
            .set_module_enabled(id, false)?;
        Ok(())
    })
    .await
}

#[tauri::command]
async fn project_open(state: tauri::State<'_, SharedCore>, path: String) -> Result<(), String> {
    with_core(state, move |core| core.open(trusted_shell(), path)).await
}

#[tauri::command]
async fn project_open_directory(
    state: tauri::State<'_, SharedCore>,
    path: String,
) -> Result<ProjectInfo, String> {
    with_core(state, move |core| {
        core.open_directory(trusted_shell(), path)
    })
    .await
}

#[tauri::command]
async fn project_new(
    state: tauri::State<'_, SharedCore>,
    path: String,
) -> Result<ProjectInfo, String> {
    with_core(state, move |core| {
        core.open_directory(trusted_shell(), path)
    })
    .await
}

#[tauri::command]
async fn project_close(state: tauri::State<'_, SharedCore>) -> Result<(), String> {
    with_core(state, |core| core.close(trusted_shell())).await
}

#[tauri::command]
async fn project_info(state: tauri::State<'_, SharedCore>) -> Result<Option<ProjectInfo>, String> {
    with_core(state, |core| Ok(core.info())).await
}

#[tauri::command]
async fn project_git_status(state: tauri::State<'_, SharedCore>) -> Result<GitStatus, String> {
    with_core(state, |core| core.project(trusted_shell())?.git_status()).await
}

#[tauri::command]
async fn project_git_init(state: tauri::State<'_, SharedCore>) -> Result<GitStatus, String> {
    with_core(state, |core| core.project(trusted_shell())?.git_init()).await
}

#[tauri::command]
async fn project_git_log(state: tauri::State<'_, SharedCore>) -> Result<Vec<GitLogEntry>, String> {
    with_core(state, |core| core.project(trusted_shell())?.git_log()).await
}

#[tauri::command]
async fn project_git_commit(
    state: tauri::State<'_, SharedCore>,
    message: String,
) -> Result<GitStatus, String> {
    with_core(state, move |core| {
        core.project(trusted_shell())?.git_commit(message)
    })
    .await
}

#[tauri::command]
async fn project_open_memory(state: tauri::State<'_, SharedCore>) -> Result<(), String> {
    with_core(state, |core| core.open_memory(trusted_shell())).await
}

#[tauri::command]
async fn project_open_default(
    app: tauri::AppHandle,
    state: tauri::State<'_, SharedCore>,
) -> Result<(), String> {
    let directory = app
        .path()
        .app_data_dir()
        .map_err(|error| error.to_string())?;
    with_core(state, move |core| {
        std::fs::create_dir_all(&directory).map_err(|error| CoreError::Io {
            operation: "create app data directory",
            source: error,
        })?;
        let project_directory = directory.join("Worldbuilder");
        let legacy_database = directory.join("worldbuilder.sqlite");
        let project_database = project_directory.join("worldbuilder.sqlite");
        if legacy_database.is_file() && !project_database.exists() {
            std::fs::create_dir_all(&project_directory).map_err(|error| CoreError::Io {
                operation: "create project directory",
                source: error,
            })?;
            std::fs::copy(&legacy_database, &project_database).map_err(|error| CoreError::Io {
                operation: "migrate legacy database",
                source: error,
            })?;
        }
        core.open_directory(trusted_shell(), project_directory)
            .map(|_| ())
    })
    .await
}

#[tauri::command]
async fn project_create_entity(
    state: tauri::State<'_, SharedCore>,
    input: CreateEntity,
) -> Result<Entity, String> {
    with_core(state, move |core| {
        core.project(trusted_shell())?.create_entity(input)
    })
    .await
}

#[tauri::command]
async fn project_list_entities(state: tauri::State<'_, SharedCore>) -> Result<Vec<Entity>, String> {
    with_core(state, |core| core.project(trusted_shell())?.list_entities()).await
}

#[tauri::command]
async fn project_search(
    state: tauri::State<'_, SharedCore>,
    query: String,
) -> Result<Vec<Entity>, String> {
    with_core(state, move |core| {
        core.project(trusted_shell())?.search(query)
    })
    .await
}

#[tauri::command]
async fn project_update_entity(
    state: tauri::State<'_, SharedCore>,
    id: String,
    name: Option<String>,
    entity_type: Option<String>,
) -> Result<Entity, String> {
    with_core(state, move |core| {
        core.project(trusted_shell())?
            .update_entity(id, name, entity_type)
    })
    .await
}

#[tauri::command]
async fn project_delete_entity(
    state: tauri::State<'_, SharedCore>,
    id: String,
) -> Result<(), String> {
    with_core(state, move |core| {
        core.project(trusted_shell())?.delete_entity(id)
    })
    .await
}

#[tauri::command]
async fn project_save_document(
    state: tauri::State<'_, SharedCore>,
    input: SaveDocument,
) -> Result<(), String> {
    with_core(state, move |core| {
        core.project(trusted_shell())?.save_document(input)
    })
    .await
}

#[tauri::command]
async fn project_save_entry(
    state: tauri::State<'_, SharedCore>,
    input: SaveEntry,
) -> Result<(), String> {
    with_core(state, move |core| {
        core.project(trusted_shell())?.save_entry(input)
    })
    .await
}

#[tauri::command]
async fn project_list_documents(
    state: tauri::State<'_, SharedCore>,
    entity_id: String,
) -> Result<Vec<worldbuilder_core::Document>, String> {
    with_core(state, move |core| {
        core.project(trusted_shell())?.list_documents(entity_id)
    })
    .await
}

#[tauri::command]
async fn project_set_field(
    state: tauri::State<'_, SharedCore>,
    field: FieldValue,
) -> Result<(), String> {
    with_core(state, move |core| {
        core.project(trusted_shell())?.set_field(field)
    })
    .await
}

#[tauri::command]
async fn project_list_fields(
    state: tauri::State<'_, SharedCore>,
    entity_id: String,
) -> Result<Vec<FieldValue>, String> {
    with_core(state, move |core| {
        core.project(trusted_shell())?.list_fields(entity_id)
    })
    .await
}

#[tauri::command]
async fn project_create_relationship(
    state: tauri::State<'_, SharedCore>,
    input: RelationshipInput,
) -> Result<Relationship, String> {
    with_core(state, move |core| {
        core.project(trusted_shell())?.create_relationship(input)
    })
    .await
}

#[tauri::command]
async fn project_list_relationships(
    state: tauri::State<'_, SharedCore>,
    entity_id: String,
) -> Result<Vec<Relationship>, String> {
    with_core(state, move |core| {
        core.project(trusted_shell())?.list_relationships(entity_id)
    })
    .await
}

#[tauri::command]
async fn project_register_asset(
    state: tauri::State<'_, SharedCore>,
    input: AssetInput,
) -> Result<Asset, String> {
    with_core(state, move |core| {
        core.project(trusted_shell())?.register_asset(input)
    })
    .await
}

#[tauri::command]
async fn project_register_asset_file(
    state: tauri::State<'_, SharedCore>,
    input: AssetFileInput,
) -> Result<Asset, String> {
    with_core(state, move |core| {
        core.project(trusted_shell())?.register_asset_file(input)
    })
    .await
}

#[tauri::command]
async fn project_list_assets(
    state: tauri::State<'_, SharedCore>,
    entity_id: String,
) -> Result<Vec<Asset>, String> {
    with_core(state, move |core| {
        core.project(trusted_shell())?.list_assets(entity_id)
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
    with_core(state, move |core| {
        core.project(trusted_shell())?.backup_to(directory)
    })
    .await
}

#[tauri::command]
async fn project_restore(state: tauri::State<'_, SharedCore>, path: String) -> Result<(), String> {
    with_core(state, move |core| {
        core.project(trusted_shell())?.restore(path)
    })
    .await
}

#[tauri::command]
async fn project_restore_payload(
    state: tauri::State<'_, SharedCore>,
    payload: String,
) -> Result<(), String> {
    with_core(state, move |core| {
        core.project(trusted_shell())?.restore_payload(&payload)
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
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .manage(Arc::new(Mutex::new(CoreService::new())))
        .manage(Arc::new(Mutex::new(PluginHost::new())))
        .invoke_handler(tauri::generate_handler![
            greet,
            plugin_bootstrap,
            plugin_rpc,
            module_list_manifests,
            module_enable,
            module_disable,
            project_open,
            project_open_directory,
            project_new,
            project_close,
            project_info,
            project_git_status,
            project_git_init,
            project_git_log,
            project_git_commit,
            project_open_memory,
            project_open_default,
            project_create_entity,
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
            project_list_relationships,
            project_register_asset,
            project_register_asset_file,
            project_list_assets,
            project_backup,
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
