// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
mod migrations;
mod project;

use migrations::{Migration, Operation};
use project::{
    AssetFileInput, CreateEntity, Entity, GitLogEntry, GitStatus, ProjectInfo, ProjectStore,
    Relationship, RelationshipInput, SaveDocument, SaveEntry,
};
use std::sync::Mutex;
use tauri::Manager;

#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[tauri::command]
fn module_list_manifests(
    store: tauri::State<'_, Mutex<Option<ProjectStore>>>,
) -> Result<Vec<serde_json::Value>, String> {
    let manifests = vec![
        serde_json::json!({
            "id": "worldbuilder.lore",
            "name": "Lore",
            "version": "0.1.0",
            "apiVersion": "1",
            "capabilities": ["entity.read", "entity.write", "document.read", "document.write", "relationship.read", "relationship.write", "asset.read", "asset.write", "search.query"],
            "schemas": [{
                "namespace": "lore",
                "entityTypes": ["person", "place", "faction", "artifact", "culture"],
                "fields": [
                    {"key": "summary", "label": "Summary", "type": "text"},
                    {"key": "aliases", "label": "Aliases", "type": "text"},
                ],
            }],
            "templates": [
                {"id": "person", "name": "Person", "entityType": "person", "fields": {"summary": "", "aliases": ""}},
                {"id": "place", "name": "Place", "entityType": "place", "fields": {"summary": "", "aliases": ""}},
                {"id": "faction", "name": "Faction", "entityType": "faction", "fields": {"summary": "", "aliases": ""}}
            ],
            "migrations": [{"id": "lore-v1", "from": 0, "to": 1, "recovery": "backup", "operations": [{"kind": "create-namespace", "namespace": "lore"}]}]
        }),
        serde_json::json!({
            "id": "worldbuilder.timeline",
            "name": "Timeline",
            "version": "0.1.0",
            "apiVersion": "1",
            "capabilities": ["entity.read", "entity.write", "document.read", "document.write", "relationship.read", "relationship.write", "asset.read", "asset.write", "search.query"],
            "schemas": [{
                "namespace": "timeline",
                "entityTypes": ["event"],
                "fields": [
                    {"key": "startsAt", "label": "Starts", "type": "date", "required": true},
                    {"key": "endsAt", "label": "Ends", "type": "date"},
                ],
            }],
            "templates": [{"id": "event", "name": "Timeline event", "entityType": "event", "fields": {"startsAt": "", "endsAt": ""}}],
            "migrations": [{"id": "timeline-v1", "from": 0, "to": 1, "recovery": "backup", "operations": [{"kind": "create-namespace", "namespace": "timeline"}]}]
        }),
    ];
    let guard = store
        .lock()
        .map_err(|_| "project lock poisoned".to_string())?;
    let Some(store) = guard.as_ref() else {
        return Ok(manifests);
    };
    manifests
        .into_iter()
        .map(|mut manifest| {
            let id = manifest
                .get("id")
                .and_then(|value| value.as_str())
                .unwrap_or_default();
            let enabled = store
                .is_module_enabled(id)
                .map_err(|error| error.to_string())?;
            manifest
                .as_object_mut()
                .expect("manifest is an object")
                .insert("enabled".into(), serde_json::Value::Bool(enabled));
            Ok(manifest)
        })
        .collect()
}

#[tauri::command]
fn module_enable(
    store: tauri::State<'_, Mutex<Option<ProjectStore>>>,
    id: String,
) -> Result<(), String> {
    if id != "worldbuilder.lore" && id != "worldbuilder.timeline" {
        return Err("unknown module".into());
    }
    let mut guard = store
        .lock()
        .map_err(|_| "project lock poisoned".to_string())?;
    let project = guard
        .as_mut()
        .ok_or_else(|| "no project is open".to_string())?;
    if project
        .get_module_version(&id)
        .map_err(|error| error.to_string())?
        == 0
    {
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
    project
        .set_module_enabled(id, true)
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn module_disable(
    store: tauri::State<'_, Mutex<Option<ProjectStore>>>,
    id: String,
) -> Result<(), String> {
    if id != "worldbuilder.lore" && id != "worldbuilder.timeline" {
        return Err("unknown module".into());
    }
    store
        .lock()
        .map_err(|_| "project lock poisoned".to_string())?
        .as_ref()
        .ok_or_else(|| "no project is open".to_string())?
        .set_module_enabled(id, false)
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn project_open(
    store: tauri::State<'_, Mutex<Option<ProjectStore>>>,
    path: String,
) -> Result<(), String> {
    let opened = ProjectStore::open(path).map_err(|error| error.to_string())?;
    *store
        .lock()
        .map_err(|_| "project lock poisoned".to_string())? = Some(opened);
    Ok(())
}

#[tauri::command]
fn project_open_directory(
    store: tauri::State<'_, Mutex<Option<ProjectStore>>>,
    path: String,
) -> Result<ProjectInfo, String> {
    let opened = ProjectStore::open_directory(path).map_err(|error| error.to_string())?;
    let info = opened
        .info()
        .ok_or_else(|| "project has no directory root".to_string())?;
    *store
        .lock()
        .map_err(|_| "project lock poisoned".to_string())? = Some(opened);
    Ok(info)
}

#[tauri::command]
fn project_new(
    store: tauri::State<'_, Mutex<Option<ProjectStore>>>,
    path: String,
) -> Result<ProjectInfo, String> {
    project_open_directory(store, path)
}

#[tauri::command]
fn project_close(store: tauri::State<'_, Mutex<Option<ProjectStore>>>) -> Result<(), String> {
    *store
        .lock()
        .map_err(|_| "project lock poisoned".to_string())? = None;
    Ok(())
}

#[tauri::command]
fn project_info(
    store: tauri::State<'_, Mutex<Option<ProjectStore>>>,
) -> Result<Option<ProjectInfo>, String> {
    Ok(store
        .lock()
        .map_err(|_| "project lock poisoned".to_string())?
        .as_ref()
        .and_then(ProjectStore::info))
}

#[tauri::command]
fn project_git_status(
    store: tauri::State<'_, Mutex<Option<ProjectStore>>>,
) -> Result<GitStatus, String> {
    store
        .lock()
        .map_err(|_| "project lock poisoned".to_string())?
        .as_ref()
        .ok_or_else(|| "no project is open".to_string())?
        .git_status()
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn project_git_init(
    store: tauri::State<'_, Mutex<Option<ProjectStore>>>,
) -> Result<GitStatus, String> {
    store
        .lock()
        .map_err(|_| "project lock poisoned".to_string())?
        .as_ref()
        .ok_or_else(|| "no project is open".to_string())?
        .git_init()
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn project_git_log(
    store: tauri::State<'_, Mutex<Option<ProjectStore>>>,
) -> Result<Vec<GitLogEntry>, String> {
    store
        .lock()
        .map_err(|_| "project lock poisoned".to_string())?
        .as_ref()
        .ok_or_else(|| "no project is open".to_string())?
        .git_log()
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn project_git_commit(
    store: tauri::State<'_, Mutex<Option<ProjectStore>>>,
    message: String,
) -> Result<GitStatus, String> {
    store
        .lock()
        .map_err(|_| "project lock poisoned".to_string())?
        .as_ref()
        .ok_or_else(|| "no project is open".to_string())?
        .git_commit(message)
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn project_open_memory(store: tauri::State<'_, Mutex<Option<ProjectStore>>>) -> Result<(), String> {
    let opened = ProjectStore::in_memory().map_err(|error| error.to_string())?;
    *store
        .lock()
        .map_err(|_| "project lock poisoned".to_string())? = Some(opened);
    Ok(())
}

#[tauri::command]
fn project_open_default(
    app: tauri::AppHandle,
    store: tauri::State<'_, Mutex<Option<ProjectStore>>>,
) -> Result<(), String> {
    let directory = app
        .path()
        .app_data_dir()
        .map_err(|error| error.to_string())?;
    std::fs::create_dir_all(&directory).map_err(|error| error.to_string())?;
    let project_directory = directory.join("Worldbuilder");
    let legacy_database = directory.join("worldbuilder.sqlite");
    let project_database = project_directory.join("worldbuilder.sqlite");
    if legacy_database.is_file() && !project_database.exists() {
        std::fs::create_dir_all(&project_directory).map_err(|error| error.to_string())?;
        std::fs::copy(&legacy_database, &project_database).map_err(|error| error.to_string())?;
    }
    let opened =
        ProjectStore::open_directory(project_directory).map_err(|error| error.to_string())?;
    *store
        .lock()
        .map_err(|_| "project lock poisoned".to_string())? = Some(opened);
    Ok(())
}

#[tauri::command]
fn project_create_entity(
    store: tauri::State<'_, Mutex<Option<ProjectStore>>>,
    input: CreateEntity,
) -> Result<Entity, String> {
    store
        .lock()
        .map_err(|_| "project lock poisoned".to_string())?
        .as_ref()
        .ok_or_else(|| "no project is open".to_string())?
        .create_entity(input)
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn project_list_entities(
    store: tauri::State<'_, Mutex<Option<ProjectStore>>>,
) -> Result<Vec<Entity>, String> {
    store
        .lock()
        .map_err(|_| "project lock poisoned".to_string())?
        .as_ref()
        .ok_or_else(|| "no project is open".to_string())?
        .list_entities()
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn project_search(
    store: tauri::State<'_, Mutex<Option<ProjectStore>>>,
    query: String,
) -> Result<Vec<Entity>, String> {
    store
        .lock()
        .map_err(|_| "project lock poisoned".to_string())?
        .as_ref()
        .ok_or_else(|| "no project is open".to_string())?
        .search(query)
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn project_update_entity(
    store: tauri::State<'_, Mutex<Option<ProjectStore>>>,
    id: String,
    name: Option<String>,
    entity_type: Option<String>,
) -> Result<Entity, String> {
    store
        .lock()
        .map_err(|_| "project lock poisoned".to_string())?
        .as_ref()
        .ok_or_else(|| "no project is open".to_string())?
        .update_entity(id, name, entity_type)
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn project_delete_entity(
    store: tauri::State<'_, Mutex<Option<ProjectStore>>>,
    id: String,
) -> Result<(), String> {
    store
        .lock()
        .map_err(|_| "project lock poisoned".to_string())?
        .as_ref()
        .ok_or_else(|| "no project is open".to_string())?
        .delete_entity(id)
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn project_save_document(
    store: tauri::State<'_, Mutex<Option<ProjectStore>>>,
    input: SaveDocument,
) -> Result<(), String> {
    store
        .lock()
        .map_err(|_| "project lock poisoned".to_string())?
        .as_ref()
        .ok_or_else(|| "no project is open".to_string())?
        .save_document(input)
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn project_save_entry(
    store: tauri::State<'_, Mutex<Option<ProjectStore>>>,
    input: SaveEntry,
) -> Result<(), String> {
    store
        .lock()
        .map_err(|_| "project lock poisoned".to_string())?
        .as_ref()
        .ok_or_else(|| "no project is open".to_string())?
        .save_entry(input)
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn project_list_documents(
    store: tauri::State<'_, Mutex<Option<ProjectStore>>>,
    entity_id: String,
) -> Result<Vec<project::Document>, String> {
    store
        .lock()
        .map_err(|_| "project lock poisoned".to_string())?
        .as_ref()
        .ok_or_else(|| "no project is open".to_string())?
        .list_documents(entity_id)
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn project_set_field(
    store: tauri::State<'_, Mutex<Option<ProjectStore>>>,
    field: project::FieldValue,
) -> Result<(), String> {
    store
        .lock()
        .map_err(|_| "project lock poisoned".to_string())?
        .as_ref()
        .ok_or_else(|| "no project is open".to_string())?
        .set_field(field)
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn project_list_fields(
    store: tauri::State<'_, Mutex<Option<ProjectStore>>>,
    entity_id: String,
) -> Result<Vec<project::FieldValue>, String> {
    store
        .lock()
        .map_err(|_| "project lock poisoned".to_string())?
        .as_ref()
        .ok_or_else(|| "no project is open".to_string())?
        .list_fields(entity_id)
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn project_list_relationships(
    store: tauri::State<'_, Mutex<Option<ProjectStore>>>,
    entity_id: String,
) -> Result<Vec<project::Relationship>, String> {
    store
        .lock()
        .map_err(|_| "project lock poisoned".to_string())?
        .as_ref()
        .ok_or_else(|| "no project is open".to_string())?
        .list_relationships(entity_id)
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn project_register_asset(
    store: tauri::State<'_, Mutex<Option<ProjectStore>>>,
    input: project::AssetInput,
) -> Result<project::Asset, String> {
    store
        .lock()
        .map_err(|_| "project lock poisoned".to_string())?
        .as_ref()
        .ok_or_else(|| "no project is open".to_string())?
        .register_asset(input)
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn project_register_asset_file(
    store: tauri::State<'_, Mutex<Option<ProjectStore>>>,
    input: AssetFileInput,
) -> Result<project::Asset, String> {
    store
        .lock()
        .map_err(|_| "project lock poisoned".to_string())?
        .as_ref()
        .ok_or_else(|| "no project is open".to_string())?
        .register_asset_file(input)
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn project_list_assets(
    store: tauri::State<'_, Mutex<Option<ProjectStore>>>,
    entity_id: String,
) -> Result<Vec<project::Asset>, String> {
    store
        .lock()
        .map_err(|_| "project lock poisoned".to_string())?
        .as_ref()
        .ok_or_else(|| "no project is open".to_string())?
        .list_assets(entity_id)
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn project_backup(
    app: tauri::AppHandle,
    store: tauri::State<'_, Mutex<Option<ProjectStore>>>,
) -> Result<String, String> {
    let directory = app
        .path()
        .app_data_dir()
        .map_err(|error| error.to_string())?;
    store
        .lock()
        .map_err(|_| "project lock poisoned".to_string())?
        .as_ref()
        .ok_or_else(|| "no project is open".to_string())?
        .backup_to(directory)
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn project_restore(
    store: tauri::State<'_, Mutex<Option<ProjectStore>>>,
    path: String,
) -> Result<(), String> {
    store
        .lock()
        .map_err(|_| "project lock poisoned".to_string())?
        .as_ref()
        .ok_or_else(|| "no project is open".to_string())?
        .restore(path)
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn project_restore_payload(
    store: tauri::State<'_, Mutex<Option<ProjectStore>>>,
    payload: String,
) -> Result<(), String> {
    store
        .lock()
        .map_err(|_| "project lock poisoned".to_string())?
        .as_ref()
        .ok_or_else(|| "no project is open".to_string())?
        .restore_payload(&payload)
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn project_rebuild_search(
    store: tauri::State<'_, Mutex<Option<ProjectStore>>>,
) -> Result<(), String> {
    store
        .lock()
        .map_err(|_| "project lock poisoned".to_string())?
        .as_ref()
        .ok_or_else(|| "no project is open".to_string())?
        .rebuild_search()
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn project_seed_example(
    store: tauri::State<'_, Mutex<Option<ProjectStore>>>,
) -> Result<usize, String> {
    store
        .lock()
        .map_err(|_| "project lock poisoned".to_string())?
        .as_mut()
        .ok_or_else(|| "no project is open".to_string())?
        .seed_example()
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn migration_validate(
    store: tauri::State<'_, Mutex<Option<ProjectStore>>>,
    module_id: String,
    migration: serde_json::Value,
) -> Result<(), String> {
    let migration: Migration = serde_json::from_value(migration).map_err(|e| e.to_string())?;
    if migration.module_id != module_id {
        return Err("migration module ID does not match command module ID".into());
    }
    let mut store_lock = store
        .lock()
        .map_err(|_| "project lock poisoned".to_string())?;
    let conn = store_lock
        .as_mut()
        .ok_or_else(|| "no project is open".to_string())?;
    let current = conn
        .get_module_version(&module_id)
        .map_err(|e| e.to_string())?;
    conn.validate_migration(&migration, current)
}

#[tauri::command]
fn migration_apply(
    store: tauri::State<'_, Mutex<Option<ProjectStore>>>,
    module_id: String,
    migration: serde_json::Value,
) -> Result<(), String> {
    let migration: Migration = serde_json::from_value(migration).map_err(|e| e.to_string())?;
    if migration.module_id != module_id {
        return Err("migration module ID does not match command module ID".into());
    }
    let mut store_lock = store
        .lock()
        .map_err(|_| "project lock poisoned".to_string())?;
    let conn = store_lock
        .as_mut()
        .ok_or_else(|| "no project is open".to_string())?;
    conn.apply_migration(&migration)
}

#[tauri::command]
fn project_create_relationship(
    store: tauri::State<'_, Mutex<Option<ProjectStore>>>,
    input: RelationshipInput,
) -> Result<Relationship, String> {
    store
        .lock()
        .map_err(|_| "project lock poisoned".to_string())?
        .as_ref()
        .ok_or_else(|| "no project is open".to_string())?
        .create_relationship(input)
        .map_err(|error| error.to_string())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .manage(Mutex::new(None::<ProjectStore>))
        .invoke_handler(tauri::generate_handler![
            greet,
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
