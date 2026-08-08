// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Component, Path};
use std::io::{Cursor, Read};
use std::sync::{mpsc, Arc, Mutex, OnceLock};
use std::thread;
use std::time::{Duration, Instant};

use daena_core::{
    Asset, AssetFileInput, AssetInput, AssetReplaceInput, AuthorityContext, CoreError, CoreService,
    CreateEntity, Entity, ExternalChangeReport, FieldValue, GitLogEntry, GitPreflight, GitRemote,
    GitResetResult, GitStatus, GitToolInfo, Migration, Operation, ProjectInfo, ProjectStore,
    Relationship, RelationshipInput, SaveDocument, SaveEntry,
};
use daena_plugin_api::{
    CommandAction, MigrationOperation, PluginManifest, RpcRequest, RpcResponse, ViewComponent,
};
use daena_plugin_host::{
    plugin_window_label, webview_policy, ArchiveLimits, DependencyResolver, PluginHost, Session,
    VerificationPolicy, BUNDLED_TIMELINE_SERVICE_WASM,
};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use tauri::{Emitter, Manager};

mod settings;

use settings::{AppSettings, AppSettingsUpdate, SettingsStore};

type SharedCore = Arc<Mutex<CoreService>>;
type SharedPluginHost = Arc<Mutex<PluginHost>>;
type SharedBinaryTransfers = Arc<Mutex<BinaryTransferManager>>;
type SharedSettings = Arc<Mutex<SettingsStore>>;

const MAX_ASSET_TRANSFER_BYTES: usize = 64 * 1024 * 1024;
const ASSET_TRANSFER_TTL: Duration = Duration::from_secs(60);
const BUNDLED_FMG_ARCHIVE: &[u8] = include_bytes!("../plugin-assets/maps/fmg-v1.119.zip");

/// Set during `setup` so the custom protocol handler (which does not receive
/// an `AppHandle`) can forward plugin state events to the shell.
static APP_HANDLE: OnceLock<tauri::AppHandle> = OnceLock::new();

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
}

impl BinaryTransferManager {
    fn cleanup(&mut self) {
        let now = Instant::now();
        self.transfers.retain(|_, transfer| match transfer {
            BinaryTransfer::Read { expires_at, .. }
            | BinaryTransfer::Upload { expires_at, .. }
            | BinaryTransfer::RecoveryUpload { expires_at, .. }
            | BinaryTransfer::Create { expires_at, .. } => *expires_at > now,
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
                let digest = format!(
                    "sha256:{:x}",
                    Sha256::digest(bytes)
                );
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
                let digest = format!(
                    "sha256:{:x}",
                    Sha256::digest(bytes)
                );
                if digest != content_hash {
                    return Err("asset upload content hash does not match bytes".into());
                }
                Ok((entity_id.clone(), bytes.clone()))
            }
            _ => Err("asset upload handle is not valid for this session or project".into()),
        }
    }

    fn complete_upload(&mut self, token: &str, plugin_id: &str, session_id: &str) -> Result<(), String> {
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
            | BinaryTransfer::Create { plugin_id, .. } => plugin_id,
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
}

type SharedProjectWatcher = Arc<Mutex<ProjectWatcher>>;

fn stop_project_watcher(watcher: &SharedProjectWatcher) -> Result<(), String> {
    let mut watcher = watcher
        .lock()
        .map_err(|_| "project watcher lock poisoned".to_string())?;
    if let Some(stop) = watcher.stop.take() {
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
    state
        .lock()
        .map_err(|_| "core lock poisoned".to_string())?
        .info()
        .ok_or_else(|| "project is not open".to_string())?;
    let (stop, receiver) = mpsc::channel();
    watcher
        .lock()
        .map_err(|_| "project watcher lock poisoned".to_string())?
        .stop = Some(stop);
    let app = app.clone();
    let state = state.clone();
    thread::spawn(move || {
        let mut last_report = String::new();
        let mut pending_report: Option<ExternalChangeReport> = None;
        loop {
            if receiver.recv_timeout(Duration::from_millis(500)).is_ok() {
                break;
            }
            let report = state.lock().ok().and_then(|core| {
                core.project(AuthorityContext::trusted_shell())
                    .ok()
                    .and_then(|project| project.reconcile_external_changes().ok())
            });
            let Some(report) = report else { continue };
            if report.changed {
                pending_report = Some(report);
                continue;
            }
            let report = if !report.diagnostics.is_empty() {
                pending_report = None;
                report
            } else if let Some(pending) = pending_report.take() {
                pending
            } else {
                last_report.clear();
                continue;
            };
            let Ok(serialized) = serde_json::to_string(&report) else {
                continue;
            };
            if serialized == last_report {
                continue;
            }
            last_report = serialized;
            let _ = app.emit("project-external-change", report);
        }
    });
    Ok(())
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
    let Some(start) = html.find("<script async src=\"https://www.googletagmanager.com/gtag/js") else {
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
        || relative.split('/').any(|part| part.is_empty() || part == "." || part == "..")
    {
        return None;
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
    let content_type = match Path::new(relative).extension().and_then(|value| value.to_str()) {
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

fn plugin_protocol_response(
    webview_label: &str,
    request: &tauri::http::Request<Vec<u8>>,
    core: &SharedCore,
    plugins: &SharedPluginHost,
    transfers: &SharedBinaryTransfers,
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
    if request.uri().path() != "/__rpc" {
        if let Some(response) = binary_asset_response(plugin_id, request, transfers) {
            return response;
        }
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
                    return json_response(
                        serde_json::json!({"ok": false, "error": {"message": "invalid plugin RPC request"}}),
                        400,
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
                let current_project = core
                    .lock()
                    .map_err(|_| "core lock poisoned".to_string())?
                    .info()
                    .map(|info| info.root);
                if current_project.as_deref() != Some(session.project_id.as_str()) {
                    return Err("plugin session is not bound to the open project".into());
                }
                let publish_payload = (request.method == "event.publish")
                    .then(|| request.payload.clone());
                let value = if matches!(
                    request.method.as_str(),
                    "asset.read.begin"
                        | "asset.replace.begin"
                        | "asset.replace.commit"
                        | "asset.transfer.cancel"
                        | "maps.asset.create.begin"
                        | "maps.asset.create.commit"
                        | "maps.recovery.export.begin"
                        | "maps.recovery.export.commit"
                ) {
                    dispatch_binary_asset_rpc(
                        core,
                        transfers,
                        &session,
                        &request.method,
                        request.payload,
                        mutation_request_id,
                    )?
                } else if matches!(
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
                    dispatch_module_rpc(
                        &mut core,
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
        Ok(value) => json_response(value, 200),
        Err(error) => json_response(serde_json::json!({"error": error}), 400),
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

fn dispatch_binary_asset_rpc(
    core: &SharedCore,
    transfers: &SharedBinaryTransfers,
    session: &Session,
    method: &str,
    payload: serde_json::Value,
    request_id: Option<&str>,
) -> Result<serde_json::Value, String> {
    let project = core.lock().map_err(|_| "core lock poisoned".to_string())?;
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
            let root = project
                .info()
                .ok_or_else(|| "directory-backed project is required".to_string())?
                .root;
            let path = daena_core::normalized_project_path(Path::new(&root), &asset.path)
                .map_err(|e| e.to_string())?;
            let bytes = fs::read(path).map_err(|e| format!("read asset: {e}"))?;
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
                serde_json::json!({"handle":token,"url":format!("plugin://{}/__asset/{}?sessionId={}", session.plugin_id, token, session.id),"size":asset.size,"contentHash":asset.content_hash,"mimeType":asset.mime_type,"revision":asset.revision}),
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
                serde_json::json!({"handle":token,"url":format!("plugin://{}/__asset/{}/0?sessionId={}", session.plugin_id, token, session.id),"maxChunkBytes":daena_plugin_host::runtime::MAX_RPC_BYTES,"expiresInMs":ASSET_TRANSFER_TTL.as_millis()}),
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
                        && entity.entity_type.as_deref()
                            == Some(daena_core::maps::MAP_ENTITY_TYPE)
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
                serde_json::json!({"handle":token,"url":format!("plugin://{}/__asset/{}/0?sessionId={}", session.plugin_id, token, session.id),"maxChunkBytes":daena_plugin_host::runtime::MAX_RPC_BYTES,"expiresInMs":ASSET_TRANSFER_TTL.as_millis()}),
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
            std::fs::write(&source_path, &bytes).map_err(|error| {
                format!("write map source temp file: {error}")
            })?;
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
            let mut manager = transfers
                .lock()
                .map_err(|_| "asset transfer state is unavailable".to_string())?;
            manager.complete_upload(token, &session.plugin_id, &session.id)?;
            serde_json::to_value(asset).map_err(|e| e.to_string())
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
                        && entity.entity_type.as_deref()
                            == Some(daena_core::maps::MAP_ENTITY_TYPE)
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
                serde_json::json!({"handle":token,"url":format!("plugin://{}/__asset/{}/0?sessionId={}", session.plugin_id, token, session.id),"maxChunkBytes":daena_plugin_host::runtime::MAX_RPC_BYTES,"expiresInMs":ASSET_TRANSFER_TTL.as_millis()}),
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
            let file_name = path
                .rsplit('/')
                .next()
                .unwrap_or(&path)
                .to_string();
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
    if let Some(window) = app.get_webview_window(&policy.label) {
        window.show().map_err(|error| error.to_string())?;
        window.set_focus().map_err(|error| error.to_string())?;
        return Ok(());
    }
    let mut url = format!("{}?project={}", policy.url, percent_encode(project_id));
    if policy.url.contains("daena.maps") {
        // FMG uses this explicit host marker to disable browser-only
        // integrations such as its ServiceWorker and OpenWidget import.
        url.push_str("&daena=1");
    }
    if policy.url.starts_with("plugin://daena.maps/") {
        url.push_str("&daena=1");
    }
    if let Some(view_id) = view_id {
        url.push_str("&view=");
        url.push_str(&percent_encode(view_id));
    }
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
    if policy.url.contains("daena.maps") {
        url.push_str("&daena=1");
    }
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
    let project_id = state
        .lock()
        .map_err(|_| "core lock poisoned".to_string())?
        .info()
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
        let url = plugin_webview_url(
            &policy,
            &project_id,
            view_id.as_deref(),
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

    let position = tauri::LogicalPosition::new(bounds.x, bounds.y);
    let size = tauri::LogicalSize::new(bounds.width, bounds.height);
    if let Some(window) = app.get_webview_window(&label) {
        window.close().map_err(|error| error.to_string())?;
    }
    if let Some(webview) = app.get_webview(&label) {
        let ready = embedded_webview_states()
            .lock()
            .map(|mut states| {
                let state = states
                    .entry(label.clone())
                    .or_insert(EmbeddedPluginWebviewState {
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
            .set_position(position)
            .map_err(|error| error.to_string())?;
        webview.set_size(size).map_err(|error| error.to_string())?;
        webview.set_focus().map_err(|error| error.to_string())?;
        return Ok(());
    }

    let main = app
        .get_window("main")
        .ok_or_else(|| "main window is not available".to_string())?;
    let builder = tauri::WebviewBuilder::new(label.clone(), url)
        .use_https_scheme(true)
        .initialization_script(
            "Object.defineProperty(window, '__TAURI_INTERNALS__', { value: undefined, configurable: false });",
        )
        .on_page_load(move |webview, payload| {
            if matches!(
                payload.event(),
                tauri::webview::PageLoadEvent::Finished
            ) {
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
    let project_id = state
        .lock()
        .map_err(|_| "core lock poisoned".to_string())?
        .info()
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
    let project_id = state
        .lock()
        .map_err(|_| "core lock poisoned".to_string())?
        .info()
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
    let project_id = state
        .lock()
        .map_err(|_| "core lock poisoned".to_string())?
        .info()
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
    let project_id = state
        .lock()
        .map_err(|_| "core lock poisoned".to_string())?
        .info()
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
    request: RpcRequest,
) -> Result<RpcResponse, String> {
    let mut request = request;
    if request.method == "relationship.delete" {
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
    if method == "field.list" {
        let namespace = payload
            .get("namespace")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| "field list payload requires namespace".to_string())?;
        let shared_keys = {
            let host = state
                .lock()
                .map_err(|_| "plugin host lock poisoned".to_string())?;
            if host.namespaces.owner(namespace) == Some(plugin_id.as_str()) {
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
    let current_project = core
        .lock()
        .map_err(|_| "core lock poisoned".to_string())?
        .info()
        .map(|info| info.root);
    let event_project_id = session.project_id.clone();
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
        let request_id_for_dispatch =
            sanitize_mutation_request_id(&request_id).map(str::to_owned);
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
            dispatch_module_rpc(core, &method, payload, request_id_for_dispatch.as_deref())
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
        let (enabled, package_version) = states
            .get(&module_id)
            .cloned()
            .unwrap_or((true, None));
        if enabled {
            if let Some(version) = package_version {
                host.record_project_usage(&project_id, &module_id, &version)
                    .map_err(|error| CoreError::Conflict(error.to_string()))?;
            }
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
    ] {
        host.register_bundled_json_with_wasm(manifest, wasm)
            .map_err(|error| error.to_string())?;
    }
    // Native provider for the public navigation service. Registered before
    // project activation so the manifest-declared WASM stub is skipped and
    // Lore/Timeline consumers reach the same resolution as the shell command.
        let navigation = maps_navigation_service_handler(core);
        host.register_declared_service_provider(
            "daena.maps",
            "daena.maps/navigation",
            1,
            navigation,
        )
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
            host.grant_capabilities(
                &project_id,
                &id,
                granted_capabilities.into_iter().collect(),
            )
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
        let project = core.project_mut(trusted_shell())?;
        let project_id = project
            .info()
            .map(|info| info.root)
            .ok_or(CoreError::ProjectNotOpen)?;
        project.set_module_enabled(id.clone(), false)?;
        let mut host = plugins
            .lock()
            .map_err(|_| CoreError::Conflict("plugin host lock poisoned".into()))?;
        if let Err(error) = host.clear_project_usage(&project_id, &id) {
            project.set_module_enabled(id, true)?;
            return Err(CoreError::Conflict(error.to_string()));
        }
        host.deactivate_bundled(&project_id, &id);
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
        "maps.recovery.export.begin" => (&["mapEntityId", "size"], &[]),
        "maps.recovery.export.commit" => (&["handle", "contentHash"], &[]),
        "maps.recovery.list" => (&["mapEntityId"], &[]),
        "maps.recovery.restore" => (&["mapEntityId", "fileName"], &[]),
        "maps.locations.list" => (&["mapEntityId"], &[]),
        "maps.reconcile.links" => (&["mapEntityId"], &[]),
        "search.query" => (&["query"], &[]),
        "event.publish" => (&["type", "payload"], &[]),
        "event.subscribe" | "event.poll" => (&["type"], &[]),
        "service.call" => (&["name", "major", "payload"], &["deadlineMs"]),
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

fn dispatch_module_rpc(
    core: &mut CoreService,
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
            serde_json::to_value(project.restore_map_recovery_copy(
                &entity_id,
                &file_name,
                request_id,
            )?)
            .map_err(|error| CoreError::Validation(error.to_string()))
        }
        "maps.locations.list" => {
            let map_entity_id = payload_string(&payload, "mapEntityId")?;
            serde_json::to_value(project.map_location_projection(map_entity_id)?)
                .map_err(|error| CoreError::Validation(error.to_string()))
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
async fn trusted_module_rpc(
    state: tauri::State<'_, SharedCore>,
    plugins: tauri::State<'_, SharedPluginHost>,
    method: String,
    payload: serde_json::Value,
    request_id: Option<String>,
) -> Result<serde_json::Value, String> {
    let project_id = state
        .inner()
        .lock()
        .map_err(|_| "core lock poisoned".to_string())?
        .info()
        .map(|info| info.root)
        .ok_or_else(|| "project is not open".to_string())?;
    let event_method = method.clone();
    let result = with_core(state, move |core| {
        dispatch_module_rpc(core, &method, payload, request_id.as_deref())
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
            | "relationship.delete"
            | "asset.register"
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
    plugins: tauri::State<'_, SharedPluginHost>,
    watcher: tauri::State<'_, SharedProjectWatcher>,
    path: String,
) -> Result<(), String> {
    let plugins = plugins.inner().clone();
    let core = state.inner().clone();
    let result = with_core(state, move |core| {
        if let Some(previous_project) = core.info().map(|info| info.root) {
            plugins
                .lock()
                .map_err(|_| CoreError::Conflict("plugin host lock poisoned".into()))?
                .deactivate_project(&previous_project);
        }
        core.open(trusted_shell(), path)?;
        let project = core.project(trusted_shell())?;
        let mut host = plugins
            .lock()
            .map_err(|_| CoreError::Conflict("plugin host lock poisoned".into()))?;
        sync_project_usage(project, &mut host)
    })
    .await;
    if result.is_ok() {
        start_project_watcher(&app, &core, watcher.inner())?;
    }
    result
}

#[tauri::command]
async fn project_open_directory(
    app: tauri::AppHandle,
    state: tauri::State<'_, SharedCore>,
    plugins: tauri::State<'_, SharedPluginHost>,
    watcher: tauri::State<'_, SharedProjectWatcher>,
    path: String,
) -> Result<ProjectInfo, String> {
    let plugins = plugins.inner().clone();
    let core = state.inner().clone();
    let result = with_core(state, move |core| {
        if let Some(previous_project) = core.info().map(|info| info.root) {
            plugins
                .lock()
                .map_err(|_| CoreError::Conflict("plugin host lock poisoned".into()))?
                .deactivate_project(&previous_project);
        }
        let info = core.open_directory(trusted_shell(), path)?;
        let mut host = plugins
            .lock()
            .map_err(|_| CoreError::Conflict("plugin host lock poisoned".into()))?;
        sync_project_usage(core.project(trusted_shell())?, &mut host)?;
        Ok(info)
    })
    .await;
    if result.is_ok() {
        start_project_watcher(&app, &core, watcher.inner())?;
    }
    result
}

#[tauri::command]
async fn project_new(
    app: tauri::AppHandle,
    state: tauri::State<'_, SharedCore>,
    plugins: tauri::State<'_, SharedPluginHost>,
    watcher: tauri::State<'_, SharedProjectWatcher>,
    path: String,
) -> Result<ProjectInfo, String> {
    let plugins = plugins.inner().clone();
    let core = state.inner().clone();
    let result = with_core(state, move |core| {
        if let Some(previous_project) = core.info().map(|info| info.root) {
            plugins
                .lock()
                .map_err(|_| CoreError::Conflict("plugin host lock poisoned".into()))?
                .deactivate_project(&previous_project);
        }
        let info = core.open_directory(trusted_shell(), path)?;
        let mut host = plugins
            .lock()
            .map_err(|_| CoreError::Conflict("plugin host lock poisoned".into()))?;
        sync_project_usage(core.project(trusted_shell())?, &mut host)?;
        Ok(info)
    })
    .await;
    if result.is_ok() {
        start_project_watcher(&app, &core, watcher.inner())?;
    }
    result
}

#[tauri::command]
async fn project_close(
    app: tauri::AppHandle,
    state: tauri::State<'_, SharedCore>,
    plugins: tauri::State<'_, SharedPluginHost>,
    watcher: tauri::State<'_, SharedProjectWatcher>,
) -> Result<(), String> {
    stop_project_watcher(watcher.inner())?;
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
async fn project_reconcile_external_changes(
    state: tauri::State<'_, SharedCore>,
) -> Result<ExternalChangeReport, String> {
    with_core(state, |core| {
        core.project(trusted_shell())?.reconcile_external_changes()
    })
    .await
}

#[tauri::command]
async fn project_save_recovery_copy(
    state: tauri::State<'_, SharedCore>,
    entity_id: String,
    body: String,
) -> Result<String, String> {
    with_core(state, move |core| {
        core.project(trusted_shell())?
            .save_recovery_copy(&entity_id, &body)
    })
    .await
}

#[tauri::command]
fn git_tool_info() -> GitToolInfo {
    ProjectStore::git_tool_info()
}

#[tauri::command]
async fn project_git_status(state: tauri::State<'_, SharedCore>) -> Result<GitStatus, String> {
    with_core(state, |core| core.project(trusted_shell())?.git_status()).await
}

#[tauri::command]
async fn project_git_preflight(
    state: tauri::State<'_, SharedCore>,
) -> Result<GitPreflight, String> {
    with_core(state, |core| core.project(trusted_shell())?.git_preflight()).await
}

#[tauri::command]
async fn project_git_staging_preview(
    state: tauri::State<'_, SharedCore>,
) -> Result<GitPreflight, String> {
    with_core(state, |core| {
        core.project(trusted_shell())?.git_staging_preview()
    })
    .await
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
    paths: Option<Vec<String>>,
) -> Result<GitStatus, String> {
    with_core(state, move |core| {
        core.project(trusted_shell())?.git_commit(message, paths)
    })
    .await
}

#[tauri::command]
async fn project_git_show_tree(
    state: tauri::State<'_, SharedCore>,
    hash: String,
) -> Result<Vec<String>, String> {
    with_core(state, move |core| {
        core.project(trusted_shell())?.git_show_tree(&hash)
    })
    .await
}

#[tauri::command]
async fn project_git_show_file(
    state: tauri::State<'_, SharedCore>,
    hash: String,
    path: String,
) -> Result<String, String> {
    with_core(state, move |core| {
        core.project(trusted_shell())?.git_show_file(&hash, &path)
    })
    .await
}

#[tauri::command]
async fn project_git_reset_hard(
    state: tauri::State<'_, SharedCore>,
    hash: String,
) -> Result<GitResetResult, String> {
    with_core(state, move |core| {
        core.project(trusted_shell())?.git_reset_hard(&hash)
    })
    .await
}

#[tauri::command]
async fn project_git_remote_list(
    state: tauri::State<'_, SharedCore>,
) -> Result<Vec<GitRemote>, String> {
    with_core(state, |core| core.project(trusted_shell())?.git_remote_list()).await
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
        core.project(trusted_shell())?.git_push(
            &remote,
            branch.as_deref(),
            force_with_lease,
        )
    })
    .await
}

#[tauri::command]
async fn project_git_restore_from_upstream(
    state: tauri::State<'_, SharedCore>,
) -> Result<GitResetResult, String> {
    with_core(state, |core| {
        core.project(trusted_shell())?.git_restore_from_upstream()
    })
    .await
}

#[tauri::command]
async fn open_external_url(url: String) -> Result<(), String> {
    let allowed = url == "https://git-scm.com/downloads"
        || url.starts_with("https://git-scm.com/");
    if !allowed {
        return Err("external URL is not allowlisted".into());
    }
    tauri_plugin_opener::open_url(url, None::<&str>).map_err(|error| error.to_string())
}

#[tauri::command]
async fn project_open_memory(
    state: tauri::State<'_, SharedCore>,
    watcher: tauri::State<'_, SharedProjectWatcher>,
) -> Result<(), String> {
    stop_project_watcher(watcher.inner())?;
    with_core(state, |core| core.open_memory(trusted_shell())).await
}

#[tauri::command]
async fn project_open_default(
    app: tauri::AppHandle,
    state: tauri::State<'_, SharedCore>,
    plugins: tauri::State<'_, SharedPluginHost>,
    watcher: tauri::State<'_, SharedProjectWatcher>,
) -> Result<(), String> {
    let directory = app
        .path()
        .app_data_dir()
        .map_err(|error| error.to_string())?;
    let plugins = plugins.inner().clone();
    let core = state.inner().clone();
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
        core.open_directory(trusted_shell(), project_directory)?;
        let mut host = plugins
            .lock()
            .map_err(|_| CoreError::Conflict("plugin host lock poisoned".into()))?;
        sync_project_usage(core.project(trusted_shell())?, &mut host)
    })
    .await;
    if result.is_ok() {
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
    with_core(state, move |core| core.project(trusted_shell())?.create_map(name)).await
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
    with_core(state, move |core| {
        core.project(trusted_shell())?.list_documents(entity_id)
    })
    .await
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
    with_core(state, move |core| {
        core.project(trusted_shell())?.list_fields(entity_id)
    })
    .await
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
async fn project_list_map_locations(
    state: tauri::State<'_, SharedCore>,
    entity_id: String,
) -> Result<Vec<daena_core::maps::LocationReference>, String> {
    with_core(state, move |core| core.project(trusted_shell())?.map_locations(entity_id)).await
}

#[tauri::command]
async fn project_upsert_map_location(
    state: tauri::State<'_, SharedCore>,
    entity_id: String,
    location: daena_core::maps::LocationReference,
    request_id: Option<String>,
) -> Result<(), String> {
    with_core(state, move |core| core.project(trusted_shell())?.upsert_map_location(entity_id, location, request_id.as_deref())).await
}

#[tauri::command]
async fn project_unlink_map_location(
    state: tauri::State<'_, SharedCore>,
    entity_id: String,
    location_id: String,
    request_id: Option<String>,
) -> Result<(), String> {
    with_core(state, move |core| core.project(trusted_shell())?.unlink_map_location(entity_id, location_id, request_id.as_deref())).await
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
            let id = request.map_entity_id.clone().ok_or_else(|| {
                "map-unavailable: mapEntityId is required".to_string()
            })?;
            let exists = project
                .list_entities()
                .map_err(|error| error.to_string())?
                .into_iter()
                .any(|entity| {
                    entity.id == id
                        && entity.entity_type.as_deref()
                            == Some(daena_core::maps::MAP_ENTITY_TYPE)
                });
            if !exists {
                return Err("map-unavailable".into());
            }
            let link = request
                .link_id
                .clone()
                .map(serde_json::Value::String);
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
                    emit: Some((
                        map_id,
                        Some(serde_json::Value::String(link_id.to_string())),
                    )),
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
                    emit: Some((
                        map_id,
                        Some(serde_json::Value::String(link_id)),
                    )),
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
        resolve_maps_navigation(core, &request)
            .map_err(daena_core::CoreError::Validation)
    })
    .await?;
    if let Some((map_id, link)) = &outcome.emit {
        app.emit(
            "maps-navigation",
            serde_json::json!({ "mapEntityId": map_id, "linkId": link }),
        )
        .map_err(|error| error.to_string())?;
    }
    outcome.result
}

/// Native handler for the public `daena.maps/navigation@1` service. Registered
/// on the plugin host before project activation so the manifest-declared WASM
/// stub is skipped; consumers such as Lore and Timeline reach the same
/// resolution and shell handoff as the `maps_navigation` Tauri command.
fn maps_navigation_service_handler(
    core: SharedCore,
) -> daena_plugin_host::ServiceHandler {
    use daena_plugin_host::{HostError, ServiceRequest};
    std::sync::Arc::new(move |request: ServiceRequest| {
        let app = APP_HANDLE
            .get()
            .ok_or_else(|| HostError("map editor is not open".into()))?;
        let request: MapsNavigationRequest =
            serde_json::from_value(request.payload.clone()).map_err(|error| {
                HostError(format!("invalid daena.maps/navigation@1 payload: {error}"))
            })?;
        let mut core = core
            .lock()
            .map_err(|_| HostError("core lock poisoned".into()))?;
        let outcome =
            resolve_maps_navigation(&mut core, &request).map_err(HostError)?;
        if let Some((map_id, link)) = &outcome.emit {
            let _ = app.emit(
                "maps-navigation",
                serde_json::json!({ "mapEntityId": map_id, "linkId": link }),
            );
        }
        outcome.result.map_err(HostError)
    })
}

/// Asks the open Maps child webview to save. There is no shell-to-webview
/// RPC channel; the child exposes its save path on `window.daenaMapProvider`
/// and the host evaluates a fixed literal to trigger it.
#[tauri::command]
async fn maps_editor_save(app: tauri::AppHandle) -> Result<(), String> {
    let label = plugin_window_label("daena.maps");
    let webview = app
        .get_webview(&label)
        .ok_or_else(|| "the map editor is not open".to_string())?;
    webview
        .eval("window.daenaMapProvider && window.daenaMapProvider.save && window.daenaMapProvider.save()")
        .map_err(|error| format!("request map editor save: {error}"))
}

/// Asks the open Maps child webview to capture the current selection. The
/// capture itself is asynchronous in the child; the anchor is delivered back
/// on `daena.maps/selection@1`, forwarded to the shell as `maps-selection`.
#[tauri::command]
async fn maps_editor_capture_anchor(app: tauri::AppHandle) -> Result<(), String> {
    let label = plugin_window_label("daena.maps");
    let webview = app
        .get_webview(&label)
        .ok_or_else(|| "the map editor is not open".to_string())?;
    webview
        .eval("window.daenaMapProvider && window.daenaMapProvider.captureSelection && window.daenaMapProvider.captureSelection()")
        .map_err(|error| format!("request map editor capture: {error}"))
}

/// Pushes a semantic overlay frame into the open Maps child webview. The
/// frame is derived state (projection rows); the child never persists it.
#[tauri::command]
async fn maps_editor_set_overlay(
    app: tauri::AppHandle,
    frame: serde_json::Value,
) -> Result<(), String> {
    let label = plugin_window_label("daena.maps");
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

/// Pushes a display date into the open Maps child webview so the overlay
/// filters locations by validity. `null` clears the temporal filter.
#[tauri::command]
async fn maps_editor_set_date(
    app: tauri::AppHandle,
    date: Option<serde_json::Value>,
) -> Result<(), String> {
    let label = plugin_window_label("daena.maps");
    let webview = app
        .get_webview(&label)
        .ok_or_else(|| "the map editor is not open".to_string())?;
    let date = serde_json::to_string(&date)
        .map_err(|error| format!("serialize overlay date: {error}"))?;
    webview
        .eval(format!(
            "window.daenaMapProvider && window.daenaMapProvider.setDate && window.daenaMapProvider.setDate({date})"
        ).as_str())
        .map_err(|error| format!("request map editor date: {error}"))
}

/// Asks the open Maps child webview to focus a linked location by ID.
#[tauri::command]
async fn maps_editor_focus_link(app: tauri::AppHandle, link_id: String) -> Result<(), String> {
    let label = plugin_window_label("daena.maps");
    let webview = app
        .get_webview(&label)
        .ok_or_else(|| "the map editor is not open".to_string())?;
    let link_id = serde_json::to_string(&link_id)
        .map_err(|error| format!("serialize link id: {error}"))?;
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
        core.project(trusted_shell())?.list_map_recovery_copies(&entity_id)
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
        core.project(trusted_shell())?
            .restore_map_recovery_copy(&entity_id, &file_name, request_id.as_deref())
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
    request_id: Option<String>,
) -> Result<(), String> {
    with_core(state, move |core| {
        core.project(trusted_shell())?
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
    let core = Arc::new(Mutex::new(CoreService::new()));
    let plugins = Arc::new(Mutex::new(
        bundled_plugin_host(core.clone()).expect("canonical bundled plugin manifests must validate"),
    ));
    let protocol_core = core.clone();
    let protocol_plugins = plugins.clone();
    let transfers = Arc::new(Mutex::new(BinaryTransferManager::default()));
    let protocol_transfers = transfers.clone();
    let startup_plugins = plugins.clone();
    let watcher = Arc::new(Mutex::new(ProjectWatcher::default()));
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
        .register_uri_scheme_protocol("plugin", move |ctx, request| {
            plugin_protocol_response(
                ctx.webview_label(),
                &request,
                &protocol_core,
                &protocol_plugins,
                &protocol_transfers,
            )
        })
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .manage(core)
        .manage(plugins)
        .manage(transfers)
        .manage(watcher)
        .invoke_handler(tauri::generate_handler![
            greet,
            settings_get,
            settings_update,
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
            project_reconcile_external_changes,
            project_save_recovery_copy,
            project_git_status,
            project_git_preflight,
            project_git_staging_preview,
            project_git_init,
            project_git_log,
            project_git_commit,
            project_git_show_tree,
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
            project_list_relationships,
            project_list_map_locations,
            project_upsert_map_location,
            project_unlink_map_location,
            maps_navigation,
            maps_editor_save,
            maps_editor_capture_anchor,
            maps_editor_set_overlay,
            maps_editor_set_date,
            maps_editor_focus_link,
            maps_recovery_list,
            maps_recovery_restore,
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
mod tests;
