// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/

use std::sync::{Arc, Mutex};

use tauri::Manager;
use worldbuilder_core::{
    Asset, AssetFileInput, AssetInput, AuthorityContext, CoreError, CoreService, CreateEntity,
    Entity, FieldValue, GitLogEntry, GitStatus, Migration, Operation, ProjectInfo, Relationship,
    RelationshipInput, SaveDocument, SaveEntry,
};
use worldbuilder_plugin_api::{MigrationOperation, PluginManifest, RpcRequest, RpcResponse};
use worldbuilder_plugin_host::{HostError, PluginHost, ServiceRequest};

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
    core: tauri::State<'_, SharedCore>,
    state: tauri::State<'_, SharedPluginHost>,
    plugin_id: String,
    project_id: String,
) -> Result<PluginBootstrap, String> {
    let origin = plugin_webview_identity(&window, Some(&plugin_id))?;
    let current_project = core
        .lock()
        .map_err(|_| "core lock poisoned".to_string())?
        .info()
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
async fn plugin_rpc(
    window: tauri::WebviewWindow,
    core: tauri::State<'_, SharedCore>,
    state: tauri::State<'_, SharedPluginHost>,
    request: RpcRequest,
) -> Result<RpcResponse, String> {
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
    let payload = request.payload;
    let current_project = core
        .lock()
        .map_err(|_| "core lock poisoned".to_string())?
        .info()
        .map(|info| info.root);
    let result = if current_project.as_deref() != Some(session.project_id.as_str()) {
        Err("plugin session is not bound to the open project".to_string())
    } else if matches!(
        method.as_str(),
        "event.subscribe" | "event.poll" | "event.publish" | "service.call"
    ) {
        dispatch_host_rpc(
            &state,
            &session.plugin_id,
            &session.project_id,
            &method,
            payload,
        )
    } else {
        let project_id = session.project_id;
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
            dispatch_module_rpc(core, &method, payload)
        })
        .await
    };
    match result {
        Ok(result) => Ok(RpcResponse {
            rpc_version: worldbuilder_plugin_api::RPC_VERSION,
            request_id,
            ok: true,
            result: Some(result),
            error: None,
        }),
        Err(error) => Ok(RpcResponse {
            rpc_version: worldbuilder_plugin_api::RPC_VERSION,
            request_id,
            ok: false,
            result: None,
            error: Some(worldbuilder_plugin_api::RpcError {
                code: "core.error".into(),
                message: format!("plugin {plugin_id}: {error}"),
                retryable: false,
                details: None,
            }),
        }),
    }
}

fn dispatch_host_rpc(
    plugins: &SharedPluginHost,
    plugin_id: &str,
    project_id: &str,
    method: &str,
    payload: serde_json::Value,
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
            .call_service(
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
            host.subscribe_event(plugin_id, project_id, name, version)
                .map_err(|error| error.to_string())?;
            Ok(serde_json::Value::Null)
        }
        "event.poll" => serde_json::to_value(
            host.poll_events(plugin_id, project_id, name, version)
                .map_err(|error| error.to_string())?,
        )
        .map_err(|error| error.to_string()),
        "event.publish" => serde_json::to_value(
            host.publish_event(
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
    let plugin_id = label
        .strip_prefix("plugin:")
        .filter(|value| !value.is_empty())
        .ok_or_else(|| "plugin RPC requires a plugin webview".to_string())?;
    if expected.is_some_and(|expected| expected != plugin_id) {
        return Err("plugin webview identity mismatch".into());
    }
    Ok(label.to_string())
}

#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {name}! You've been greeted from Rust!")
}

fn bundled_plugin_host() -> Result<PluginHost, String> {
    let mut host = PluginHost::new();
    for manifest in [
        include_str!("../../packages/modules/lore/manifest.json"),
        include_str!("../../packages/modules/timeline/manifest.json"),
    ] {
        host.register_bundled_json(manifest)
            .map_err(|error| error.to_string())?;
    }
    host.services
        .register(
            "worldbuilder.timeline",
            "worldbuilder.timeline.resolve-date",
            1,
            Arc::new(|request: ServiceRequest| {
                if request.cancellation.is_cancelled() {
                    return Err(HostError("timeline service call cancelled".into()));
                }
                let date = request
                    .payload
                    .get("date")
                    .and_then(serde_json::Value::as_str)
                    .ok_or_else(|| HostError("timeline service requires date".into()))?;
                Ok(serde_json::json!({"date": date}))
            }),
        )
        .map_err(|error| error.to_string())?;
    Ok(host)
}

fn core_migration(manifest: &PluginManifest) -> Result<Option<Migration>, String> {
    let Some(migration) = manifest.migrations.first() else {
        return Ok(None);
    };
    let operations = migration
        .operations
        .iter()
        .map(|operation| match operation {
            MigrationOperation::CreateNamespace { namespace } => Operation::CreateNamespace {
                namespace: namespace.clone(),
            },
            MigrationOperation::AddField {
                namespace, field, ..
            } => Operation::AddField {
                namespace: namespace.clone(),
                field: worldbuilder_core::FieldDefinition {
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
    Ok(Some(Migration {
        id: migration.id.clone(),
        module_id: manifest.id.clone(),
        from: migration.from as i64,
        to: migration.to as i64,
        operations,
        recovery: migration.recovery.clone(),
    }))
}

#[tauri::command]
async fn module_list_manifests(
    state: tauri::State<'_, SharedCore>,
    plugins: tauri::State<'_, SharedPluginHost>,
) -> Result<Vec<serde_json::Value>, String> {
    let mut manifests = plugins
        .lock()
        .map_err(|_| "plugin host lock poisoned".to_string())?
        .catalog
        .list()
        .map(|entry| serde_json::to_value(&entry.manifest).map_err(|error| error.to_string()))
        .collect::<Result<Vec<_>, _>>()?;
    with_core(state, |core| {
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
async fn module_enable(
    state: tauri::State<'_, SharedCore>,
    plugins: tauri::State<'_, SharedPluginHost>,
    id: String,
) -> Result<(), String> {
    let manifest = plugins
        .lock()
        .map_err(|_| "plugin host lock poisoned".to_string())?
        .catalog
        .get(&id)
        .ok_or_else(|| format!("unknown bundled plugin: {id}"))?
        .manifest
        .clone();
    let migration = core_migration(&manifest)?;
    let plugins = plugins.inner().clone();
    with_core(state, move |core| {
        let context = trusted_shell();
        let project = core.project_mut(context)?;
        let current = project.get_module_version(&id)?;
        if let Some(migration) = &migration {
            if current == migration.from {
                project.apply_migration(migration)?;
            } else if current != migration.to {
                return Err(CoreError::Validation(format!(
                    "unsupported stored version {current} for plugin {}",
                    manifest.id
                )));
            }
        }
        let project_id = project
            .info()
            .map(|info| info.root)
            .ok_or(CoreError::ProjectNotOpen)?;
        plugins
            .lock()
            .map_err(|_| CoreError::Conflict("plugin host lock poisoned".into()))?
            .activate_bundled(&project_id, &manifest.id)
            .map_err(|error| CoreError::Validation(error.to_string()))?;
        project.set_module_enabled(id, true)?;
        Ok(())
    })
    .await
}

#[tauri::command]
async fn module_disable(
    state: tauri::State<'_, SharedCore>,
    plugins: tauri::State<'_, SharedPluginHost>,
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
    with_core(state, move |core| {
        let project = core.project(trusted_shell())?;
        let project_id = project
            .info()
            .map(|info| info.root)
            .ok_or(CoreError::ProjectNotOpen)?;
        project.set_module_enabled(id.clone(), false)?;
        plugins
            .lock()
            .map_err(|_| CoreError::Conflict("plugin host lock poisoned".into()))?
            .deactivate_bundled(&project_id, &id);
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

fn payload_value<T: serde::de::DeserializeOwned>(
    payload: &serde_json::Value,
) -> Result<T, CoreError> {
    serde_json::from_value(payload.clone())
        .map_err(|error| CoreError::Validation(format!("invalid plugin RPC payload: {error}")))
}

fn dispatch_module_rpc(
    core: &mut CoreService,
    method: &str,
    payload: serde_json::Value,
) -> Result<serde_json::Value, CoreError> {
    let project = core.project_mut(AuthorityContext::plugin())?;
    match method {
        "entity.list" => serde_json::to_value(project.list_entities()?)
            .map_err(|error| CoreError::Validation(error.to_string())),
        "entity.get" => {
            let id = payload_string(&payload, "id")?;
            let entity = project
                .list_entities()?
                .into_iter()
                .find(|entity| entity.id == id);
            serde_json::to_value(entity).map_err(|error| CoreError::Validation(error.to_string()))
        }
        "entity.create" => {
            let input = worldbuilder_core::CreateEntity {
                name: payload_string(&payload, "name")?,
                entity_type: payload
                    .get("type")
                    .and_then(serde_json::Value::as_str)
                    .map(str::to_owned),
            };
            serde_json::to_value(project.create_entity(input)?)
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
            serde_json::to_value(project.update_entity(id, name, entity_type)?)
                .map_err(|error| CoreError::Validation(error.to_string()))
        }
        "entity.delete" => {
            project.delete_entity(payload_string(&payload, "id")?)?;
            Ok(serde_json::Value::Null)
        }
        "document.list" => {
            serde_json::to_value(project.list_documents(payload_string(&payload, "entityId")?)?)
                .map_err(|error| CoreError::Validation(error.to_string()))
        }
        "document.save" => {
            let input = worldbuilder_core::SaveDocument {
                entity_id: payload_string(&payload, "entityId")?,
                body: payload_string(&payload, "body")?,
                format: payload
                    .get("format")
                    .and_then(serde_json::Value::as_str)
                    .map(str::to_owned),
            };
            project.save_document(input)?;
            Ok(serde_json::Value::Null)
        }
        "field.list" => {
            serde_json::to_value(project.list_fields(payload_string(&payload, "entityId")?)?)
                .map_err(|error| CoreError::Validation(error.to_string()))
        }
        "field.set" => {
            let field = worldbuilder_core::FieldValue {
                entity_id: payload_string(&payload, "entityId")?,
                namespace: payload_string(&payload, "namespace")?,
                key: payload_string(&payload, "key")?,
                value: payload
                    .get("value")
                    .cloned()
                    .unwrap_or(serde_json::Value::Null),
            };
            project.set_field(field)?;
            Ok(serde_json::Value::Null)
        }
        "relationship.list" => {
            serde_json::to_value(project.list_relationships(payload_string(&payload, "entityId")?)?)
                .map_err(|error| CoreError::Validation(error.to_string()))
        }
        "relationship.create" => {
            let input: RelationshipInput = payload_value(&payload)?;
            serde_json::to_value(project.create_relationship(input)?)
                .map_err(|error| CoreError::Validation(error.to_string()))
        }
        "asset.list" => {
            let entity_id = payload_string(&payload, "entityId")?;
            let assets = project.list_assets(entity_id)?;
            serde_json::to_value(assets).map_err(|error| CoreError::Validation(error.to_string()))
        }
        "asset.register" => {
            let input: AssetInput = payload_value(&payload)?;
            serde_json::to_value(project.register_asset(input)?)
                .map_err(|error| CoreError::Validation(error.to_string()))
        }
        "search.query" => serde_json::to_value(project.search(payload_string(&payload, "query")?)?)
            .map_err(|error| CoreError::Validation(error.to_string())),
        _ => Err(CoreError::Validation(format!(
            "unknown plugin RPC method: {method}"
        ))),
    }
}

#[tauri::command]
async fn module_rpc(
    state: tauri::State<'_, SharedCore>,
    plugins: tauri::State<'_, SharedPluginHost>,
    plugin_id: String,
    project_id: String,
    method: String,
    payload: serde_json::Value,
) -> Result<serde_json::Value, String> {
    plugins
        .lock()
        .map_err(|_| "plugin host lock poisoned".to_string())?
        .authorize_bundled(&plugin_id, &project_id, &method, payload.clone())
        .map_err(|error| error.to_string())?;
    if matches!(
        method.as_str(),
        "event.subscribe" | "event.poll" | "event.publish" | "service.call"
    ) {
        return dispatch_host_rpc(plugins.inner(), &plugin_id, &project_id, &method, payload);
    }
    let event_method = method.clone();
    let event_project_id = project_id.clone();
    let event_plugins = plugins.inner().clone();
    with_core(state, move |core| {
        let current_project = core
            .info()
            .map(|info| info.root)
            .ok_or(CoreError::ProjectNotOpen)?;
        if current_project != project_id {
            return Err(CoreError::Unauthorized {
                operation: "access another project",
            });
        }
        dispatch_module_rpc(core, &method, payload)
    })
    .await
    .and_then(move |result| {
        if matches!(
            event_method.as_str(),
            "entity.create"
                | "entity.update"
                | "entity.delete"
                | "document.save"
                | "field.set"
                | "relationship.create"
                | "asset.register"
        ) {
            let event_payload = serde_json::json!({
                "method": event_method,
                "result": result.clone(),
            });
            event_plugins
                .lock()
                .map_err(|_| "plugin host lock poisoned".to_string())?
                .publish_core_event(
                    &event_project_id,
                    "worldbuilder.core/entity-changed",
                    1,
                    event_payload,
                )
                .map_err(|error| error.to_string())?;
        }
        Ok(result)
    })
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
async fn project_close(
    state: tauri::State<'_, SharedCore>,
    plugins: tauri::State<'_, SharedPluginHost>,
) -> Result<(), String> {
    let plugins = plugins.inner().clone();
    with_core(state, move |core| {
        let project_id = core.info().map(|info| info.root);
        core.close(trusted_shell())?;
        if let Some(project_id) = project_id {
            plugins
                .lock()
                .map_err(|_| CoreError::Conflict("plugin host lock poisoned".into()))?
                .deactivate_project(&project_id);
        }
        Ok(())
    })
    .await
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
        .manage(Arc::new(Mutex::new(
            bundled_plugin_host().expect("canonical bundled plugin manifests must validate"),
        )))
        .invoke_handler(tauri::generate_handler![
            greet,
            plugin_bootstrap,
            plugin_rpc,
            module_list_manifests,
            module_enable,
            module_disable,
            module_rpc,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundled_manifests_supply_generic_migrations() {
        let host = bundled_plugin_host().unwrap();
        let lore = host.catalog.get("worldbuilder.lore").unwrap();
        let timeline = host.catalog.get("worldbuilder.timeline").unwrap();
        assert_eq!(
            core_migration(&lore.manifest).unwrap().unwrap().id,
            "lore-v1"
        );
        assert_eq!(
            core_migration(&timeline.manifest).unwrap().unwrap().id,
            "timeline-v1"
        );
    }

    #[test]
    fn broker_dispatch_uses_plugin_project_authority() {
        let mut core = CoreService::new();
        core.open_memory(AuthorityContext::trusted_shell()).unwrap();
        let created = dispatch_module_rpc(
            &mut core,
            "entity.create",
            serde_json::json!({"name": "Broker Entity", "type": "person"}),
        )
        .unwrap();
        assert_eq!(created["name"], "Broker Entity");
        let entities =
            dispatch_module_rpc(&mut core, "entity.list", serde_json::json!({})).unwrap();
        assert_eq!(entities.as_array().unwrap().len(), 1);
    }
}
