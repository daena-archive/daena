// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/

use std::path::{Component, Path};
use std::sync::{Arc, Mutex};

use tauri::Manager;
use worldbuilder_core::{
    Asset, AssetFileInput, AssetInput, AuthorityContext, CoreError, CoreService, CreateEntity,
    Entity, FieldValue, GitLogEntry, GitStatus, Migration, Operation, ProjectInfo, ProjectStore,
    Relationship, RelationshipInput, SaveDocument, SaveEntry,
};
use worldbuilder_plugin_api::{MigrationOperation, PluginManifest, RpcRequest, RpcResponse};
use worldbuilder_plugin_host::{
    plugin_window_label, webview_policy, ArchiveLimits, HostError, PluginHost, ServiceRequest,
    VerificationPolicy,
};

type SharedCore = Arc<Mutex<CoreService>>;
type SharedPluginHost = Arc<Mutex<PluginHost>>;

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
        let (bytes, content_type): (&[u8], &str) = match (plugin_id, path) {
            ("worldbuilder.lore", "/dist/ui/index.html") => (
                include_bytes!("../plugin-assets/lore/index.html"),
                "text/html",
            ),
            ("worldbuilder.timeline", "/dist/ui/index.html") => (
                include_bytes!("../plugin-assets/timeline/index.html"),
                "text/html",
            ),
            (_, "/dist/ui/plugin.js") => (
                include_bytes!("../plugin-assets/shared/plugin.js"),
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
            "worldbuilder.lore" => include_str!("../../packages/modules/lore/manifest.json"),
            "worldbuilder.timeline" => {
                include_str!("../../packages/modules/timeline/manifest.json")
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

fn json_response(value: serde_json::Value, status: u16) -> tauri::http::Response<Vec<u8>> {
    tauri::http::Response::builder()
        .status(status)
        .header("Content-Type", "application/json")
        .header("Access-Control-Allow-Origin", "*")
        .body(serde_json::to_vec(&value).unwrap_or_else(|_| b"{}".to_vec()))
        .unwrap()
}

fn plugin_protocol_response(
    webview_label: &str,
    request: &tauri::http::Request<Vec<u8>>,
    core: &SharedCore,
    plugins: &SharedPluginHost,
) -> tauri::http::Response<Vec<u8>> {
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
    if request.uri().path() != "/__rpc" {
        let project_id = core
            .lock()
            .ok()
            .and_then(|core| core.info())
            .map(|info| info.root);
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
        Err(error) => return json_response(serde_json::json!({"error": error.to_string()}), 400),
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
                let current_project = core
                    .lock()
                    .ok()
                    .and_then(|core| core.info())
                    .map(|info| info.root);
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
                            session_id: session.id,
                            plugin_id: plugin_id.to_string(),
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
                    return json_response(
                        serde_json::json!({"ok": false, "error": {"message": "invalid plugin RPC request"}}),
                        400,
                    )
                }
            };
            let request_id = request.request_id.clone();
            let result = (|| -> Result<serde_json::Value, String> {
                let session = plugins
                    .lock()
                    .map_err(|_| "plugin host lock poisoned".to_string())?
                    .authorize_rpc(webview_label, &request)
                    .map_err(|error| error.message)?;
                let current_project = core
                    .lock()
                    .map_err(|_| "core lock poisoned".to_string())?
                    .info()
                    .map(|info| info.root);
                if current_project.as_deref() != Some(session.project_id.as_str()) {
                    return Err("plugin session is not bound to the open project".into());
                }
                let value = if matches!(
                    request.method.as_str(),
                    "event.subscribe" | "event.poll" | "event.publish" | "service.call"
                ) {
                    dispatch_host_rpc(
                        plugins,
                        &session.plugin_id,
                        &session.project_id,
                        &request.method,
                        request.payload,
                    )?
                } else {
                    let mut core = core.lock().map_err(|_| "core lock poisoned".to_string())?;
                    dispatch_module_rpc(&mut core, &request.method, request.payload)
                        .map_err(|error| error.to_string())?
                };
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
        Ok(value) => json_response(value, 200),
        Err(error) => json_response(serde_json::json!({"error": error}), 400),
    }
}

fn open_plugin_webview(
    app: &tauri::AppHandle,
    plugin_id: &str,
    project_id: &str,
    host: &PluginHost,
) -> Result<(), String> {
    let entry = host
        .runtime_entry(project_id, plugin_id)
        .ok_or_else(|| "plugin is not installed".to_string())?;
    if entry.manifest.kind != worldbuilder_plugin_api::PluginKind::Sandboxed {
        return Err("only sandboxed plugins have UI webviews".into());
    }
    if host.lifecycle.state(project_id, plugin_id).state
        != worldbuilder_plugin_api::LifecycleState::Active
    {
        return Err("plugin is not active".into());
    }
    let policy =
        webview_policy(&entry.manifest).ok_or_else(|| "plugin has no UI entrypoint".to_string())?;
    if let Some(window) = app.get_webview_window(&policy.label) {
        window.show().map_err(|error| error.to_string())?;
        window.set_focus().map_err(|error| error.to_string())?;
        return Ok(());
    }
    let url = format!("{}?project={}", policy.url, percent_encode(project_id));
    tauri::WebviewWindowBuilder::new(
        app,
        policy.label,
        tauri::WebviewUrl::External(url.parse().map_err(|error| format!("invalid plugin URL: {error}"))?),
    )
    .use_https_scheme(true)
    .initialization_script("Object.defineProperty(window, '__TAURI_INTERNALS__', { value: undefined, configurable: false });")
    .title(entry.manifest.name.clone())
    .inner_size(980.0, 720.0)
    .visible(true)
    .build()
    .map_err(|error| error.to_string())?;
    Ok(())
}

fn close_plugin_webview(app: &tauri::AppHandle, plugin_id: &str) {
    if let Some(window) = app.get_webview_window(&plugin_window_label(plugin_id)) {
        let _ = window.close();
    }
}

#[tauri::command]
fn plugin_open_webview(
    app: tauri::AppHandle,
    state: tauri::State<'_, SharedCore>,
    plugins: tauri::State<'_, SharedPluginHost>,
    plugin_id: String,
) -> Result<(), String> {
    let project_id = state
        .lock()
        .map_err(|_| "core lock poisoned".to_string())?
        .info()
        .map(|info| info.root)
        .ok_or_else(|| "project is not open".to_string())?;
    let host = plugins
        .lock()
        .map_err(|_| "plugin host lock poisoned".to_string())?;
    open_plugin_webview(&app, &plugin_id, &project_id, &host)
}

#[tauri::command]
fn plugin_close_webview(app: tauri::AppHandle, plugin_id: String) -> Result<(), String> {
    close_plugin_webview(&app, &plugin_id);
    Ok(())
}

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
#[serde(rename_all = "camelCase")]
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
        .runtime_entry(&project_id, &plugin_id)
        .ok_or_else(|| "plugin was removed during bootstrap".to_string())?;
    Ok(PluginBootstrap {
        session_id: session.id,
        plugin_id,
        package_digest: entry.digest,
        manifest: entry.manifest,
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
    for module in project.module_states()? {
        if let Some(version) = module.package_version {
            host.record_project_usage(&project_id, &module.module_id, &version)
                .map_err(|error| CoreError::Conflict(error.to_string()))?;
        }
    }
    Ok(())
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

#[cfg(test)]
fn core_migration(manifest: &PluginManifest) -> Result<Option<Migration>, String> {
    Ok(core_migrations(manifest)?.into_iter().next())
}

fn core_migrations(manifest: &PluginManifest) -> Result<Vec<Migration>, String> {
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
            Ok(Migration {
                id: migration.id.clone(),
                module_id: manifest.id.clone(),
                from: migration.from as i64,
                to: migration.to as i64,
                operations,
                recovery: migration.recovery.clone(),
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
    with_core(state, move |core| {
        let context = trusted_shell();
        let project = core.project(context).ok();
        let project_id = project.and_then(|project| project.info().map(|info| info.root));
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
            let mut manifest = serde_json::to_value(&entry.manifest)
                .map_err(|error| CoreError::Validation(error.to_string()))?;
            let enabled = if let Some(project) = project {
                project.is_module_enabled(&id)?
            } else {
                false
            };
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
async fn module_enable(
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
        let manifest = {
            let mut host = plugins
                .lock()
                .map_err(|_| CoreError::Conflict("plugin host lock poisoned".into()))?;
            if let Some(version) = project.module_package_version(&id)? {
                host.select_project_version(&project_id, &id, &version)
                    .map_err(|error| CoreError::Validation(error.to_string()))?;
            }
            host.runtime_entry(&project_id, &id)
                .ok_or_else(|| CoreError::Validation("plugin runtime version is missing".into()))?
                .manifest
        };
        let migrations =
            core_migrations(&manifest).map_err(|error| CoreError::Validation(error.to_string()))?;
        let current = project.get_module_version(&id)?;
        let pending = migrations
            .iter()
            .filter(|migration| migration.from >= current)
            .cloned()
            .collect::<Vec<_>>();
        if let Some(first) = pending.first() {
            if current != first.from {
                return Err(CoreError::Validation(format!(
                    "unsupported stored version {current} for plugin {}",
                    manifest.id
                )));
            }
            project.apply_migrations(&pending)?;
        }
        let mut host = plugins
            .lock()
            .map_err(|_| CoreError::Conflict("plugin host lock poisoned".into()))?;
        host.activate_bundled(&project_id, &manifest.id)
            .map_err(|error| CoreError::Validation(error.to_string()))?;
        host.record_project_usage(&project_id, &id, &manifest.version)
            .map_err(|error| CoreError::Conflict(error.to_string()))?;
        project.set_module_enabled(id, true)?;
        Ok(())
    })
    .await
}

#[tauri::command]
async fn module_disable(
    app: tauri::AppHandle,
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
        close_plugin_webview(&app, &id);
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
async fn plugin_select_version(
    state: tauri::State<'_, SharedCore>,
    plugins: tauri::State<'_, SharedPluginHost>,
    project_id: String,
    plugin_id: String,
    version: String,
) -> Result<(), String> {
    let plugins = plugins.inner().clone();
    with_core(state, move |core| {
        let project = core.project(trusted_shell())?;
        if project.info().map(|info| info.root) != Some(project_id.clone()) {
            return Err(CoreError::Unauthorized {
                operation: "select plugin version for another project",
            });
        }
        let mut host = plugins
            .lock()
            .map_err(|_| CoreError::Conflict("plugin host lock poisoned".into()))?;
        let installed = host.packages.get(&plugin_id, &version).is_some()
            || host
                .catalog
                .get(&plugin_id)
                .is_some_and(|entry| entry.manifest.version == version);
        if !installed {
            return Err(CoreError::Validation(
                "plugin version is not installed".into(),
            ));
        }
        project.set_module_package_version(&plugin_id, Some(&version))?;
        host.select_project_version(&project_id, &plugin_id, &version)
            .map_err(|error| CoreError::Validation(error.to_string()))
    })
    .await
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
        let (old_version, old_grants, target_manifest, plan) = {
            let host = plugins
                .lock()
                .map_err(|_| CoreError::Conflict("plugin host lock poisoned".into()))?;
            let old_version = host.selected_project_version(&project_id, &plugin_id);
            let old_grants = host.grants.get(&project_id, &plugin_id);
            let target = host.packages.get(&plugin_id, &version).ok_or_else(|| {
                CoreError::Validation("target plugin version is not installed".into())
            })?;
            let target_manifest = worldbuilder_plugin_api::parse_manifest(
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
            (old_version, old_grants, target_manifest, plan)
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
        }
        let migrations = core_migrations(&target_manifest)
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
            let _ = host.activate_bundled(&project_id, &plugin_id);
            return Err(CoreError::Validation(error.to_string()));
        }
        project.set_module_package_version(&plugin_id, Some(&version))?;
        project.set_module_enabled(plugin_id, true)?;
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
        if core.info().is_some() {
            let project = core.project(trusted_shell())?;
            if project.module_package_version(&plugin_id)?.as_deref() == Some(version.as_str()) {
                return Err(CoreError::Conflict(
                    "cannot uninstall code selected by the open project".into(),
                ));
            }
        }
        plugins
            .lock()
            .map_err(|_| CoreError::Conflict("plugin host lock poisoned".into()))?
            .uninstall_code(&plugin_id, &version)
            .map_err(|error| CoreError::Validation(error.to_string()))
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
        let project_id = core
            .info()
            .map(|info| info.root)
            .ok_or(CoreError::ProjectNotOpen)?;
        let backup = core
            .project_mut(trusted_shell())?
            .delete_plugin_data(&plugin_id, &confirmation)?;
        plugins
            .lock()
            .map_err(|_| CoreError::Conflict("plugin host lock poisoned".into()))?
            .clear_project_usage(&project_id, &plugin_id)
            .map_err(|error| CoreError::Conflict(error.to_string()))?;
        Ok(backup)
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
            let input = worldbuilder_core::CreateEntry {
                name: payload_string(&payload, "name")?,
                entity_type: payload
                    .get("type")
                    .and_then(serde_json::Value::as_str)
                    .map(str::to_owned),
                document,
                fields,
                relationships,
            };
            serde_json::to_value(project.create_entry(input)?)
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
        "relationship.delete" => {
            project.delete_relationship(payload_string(&payload, "id")?)?;
            Ok(serde_json::Value::Null)
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
                | "relationship.delete"
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
async fn project_open(
    state: tauri::State<'_, SharedCore>,
    plugins: tauri::State<'_, SharedPluginHost>,
    path: String,
) -> Result<(), String> {
    let plugins = plugins.inner().clone();
    with_core(state, move |core| {
        core.open(trusted_shell(), path)?;
        let project = core.project(trusted_shell())?;
        let mut host = plugins
            .lock()
            .map_err(|_| CoreError::Conflict("plugin host lock poisoned".into()))?;
        sync_project_usage(project, &mut host)
    })
    .await
}

#[tauri::command]
async fn project_open_directory(
    state: tauri::State<'_, SharedCore>,
    plugins: tauri::State<'_, SharedPluginHost>,
    path: String,
) -> Result<ProjectInfo, String> {
    let plugins = plugins.inner().clone();
    with_core(state, move |core| {
        let info = core.open_directory(trusted_shell(), path)?;
        let mut host = plugins
            .lock()
            .map_err(|_| CoreError::Conflict("plugin host lock poisoned".into()))?;
        sync_project_usage(core.project(trusted_shell())?, &mut host)?;
        Ok(info)
    })
    .await
}

#[tauri::command]
async fn project_new(
    state: tauri::State<'_, SharedCore>,
    plugins: tauri::State<'_, SharedPluginHost>,
    path: String,
) -> Result<ProjectInfo, String> {
    let plugins = plugins.inner().clone();
    with_core(state, move |core| {
        let info = core.open_directory(trusted_shell(), path)?;
        let mut host = plugins
            .lock()
            .map_err(|_| CoreError::Conflict("plugin host lock poisoned".into()))?;
        sync_project_usage(core.project(trusted_shell())?, &mut host)?;
        Ok(info)
    })
    .await
}

#[tauri::command]
async fn project_close(
    app: tauri::AppHandle,
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
            for entry in plugins
                .lock()
                .map_err(|_| CoreError::Conflict("plugin host lock poisoned".into()))?
                .catalog
                .list()
            {
                close_plugin_webview(&app, &entry.manifest.id);
            }
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
    plugins: tauri::State<'_, SharedPluginHost>,
) -> Result<(), String> {
    let directory = app
        .path()
        .app_data_dir()
        .map_err(|error| error.to_string())?;
    let plugins = plugins.inner().clone();
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
        core.open_directory(trusted_shell(), project_directory)?;
        let mut host = plugins
            .lock()
            .map_err(|_| CoreError::Conflict("plugin host lock poisoned".into()))?;
        sync_project_usage(core.project(trusted_shell())?, &mut host)
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
    let core = Arc::new(Mutex::new(CoreService::new()));
    let plugins = Arc::new(Mutex::new(
        bundled_plugin_host().expect("canonical bundled plugin manifests must validate"),
    ));
    let protocol_core = core.clone();
    let protocol_plugins = plugins.clone();
    let startup_plugins = plugins.clone();
    tauri::Builder::default()
        .setup(move |app| {
            let app_data = app
                .path()
                .app_data_dir()
                .map_err(|error| error.to_string())?;
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
        .register_uri_scheme_protocol("plugin", move |ctx, request| {
            plugin_protocol_response(
                ctx.webview_label(),
                &request,
                &protocol_core,
                &protocol_plugins,
            )
        })
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .manage(core)
        .manage(plugins)
        .invoke_handler(tauri::generate_handler![
            greet,
            plugin_bootstrap,
            plugin_rpc,
            plugin_open_webview,
            plugin_close_webview,
            module_list_manifests,
            module_enable,
            module_disable,
            plugin_install_package,
            plugin_select_version,
            plugin_upgrade,
            plugin_rollback,
            plugin_uninstall_code,
            plugin_delete_data,
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

    #[test]
    fn bundled_plugin_protocol_serves_only_embedded_assets() {
        let request = tauri::http::Request::builder()
            .uri("plugin://worldbuilder.lore/dist/ui/index.html")
            .body(Vec::new())
            .unwrap();
        let response = plugin_asset_response("worldbuilder.lore", &request, None, None);
        assert_eq!(response.status(), 200);
        assert_eq!(response.headers().get("Content-Type").unwrap(), "text/html");
        assert!(String::from_utf8_lossy(response.body()).contains("plugin.js"));

        let traversal = tauri::http::Request::builder()
            .uri("plugin://worldbuilder.lore/../manifest.json")
            .body(Vec::new())
            .unwrap();
        assert_eq!(
            plugin_asset_response("worldbuilder.lore", &traversal, None, None).status(),
            404
        );
    }

    #[test]
    fn installed_plugin_assets_are_served_from_the_verified_ui_root() {
        let root =
            std::env::temp_dir().join(format!("worldbuilder-protocol-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("dist/ui")).unwrap();
        std::fs::write(root.join("dist/ui/index.html"), b"installed plugin").unwrap();
        let mut manifest: PluginManifest =
            serde_json::from_str(include_str!("../../packages/modules/lore/manifest.json"))
                .unwrap();
        manifest.id = "com.example.third-party".into();
        let request = tauri::http::Request::builder()
            .uri("plugin://com.example.third-party/dist/ui/index.html")
            .body(Vec::new())
            .unwrap();
        let response = plugin_asset_response(
            "com.example.third-party",
            &request,
            Some(&root),
            Some(&manifest),
        );
        assert_eq!(response.status(), 200);
        assert_eq!(response.body(), b"installed plugin");

        let traversal = tauri::http::Request::builder()
            .uri("plugin://com.example.third-party/dist/ui/../../manifest.json")
            .body(Vec::new())
            .unwrap();
        assert_eq!(
            plugin_asset_response(
                "com.example.third-party",
                &traversal,
                Some(&root),
                Some(&manifest),
            )
            .status(),
            404
        );
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn plugin_bootstrap_uses_camel_case_wire_fields() {
        let value = serde_json::to_value(PluginBootstrap {
            session_id: "session".into(),
            plugin_id: "worldbuilder.lore".into(),
            package_digest: "digest".into(),
            manifest: serde_json::from_str(include_str!(
                "../../packages/modules/lore/manifest.json"
            ))
            .unwrap(),
        })
        .unwrap();
        assert_eq!(value["sessionId"], "session");
        assert!(value.get("session_id").is_none());
    }
}
