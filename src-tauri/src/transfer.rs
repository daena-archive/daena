// Binary asset transfers and plugin protocol responses.
use super::APP_HANDLE;
use super::*;

const MAX_ASSET_TRANSFER_BYTES: usize = daena_core::maps::PHYSICAL_MAX_SOURCE_BYTES;
pub(super) const ASSET_TRANSFER_TTL: Duration = Duration::from_secs(60);
#[derive(Default)]
pub(super) struct BinaryTransferManager {
    pub(super) transfers: BTreeMap<String, BinaryTransfer>,
}

pub(super) enum BinaryTransfer {
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
    pub(super) fn cleanup(&mut self) {
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

    pub(super) fn token(&mut self, transfer: BinaryTransfer) -> String {
        self.cleanup();
        let token = uuid::Uuid::new_v4().to_string();
        self.transfers.insert(token.clone(), transfer);
        token
    }

    pub(super) fn take_read(
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

    pub(super) fn append_upload(
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

    pub(super) fn prepare_upload(
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

    pub(super) fn prepare_recovery_upload(
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

    pub(super) fn prepare_image_import(
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

    pub(super) fn prepare_vector_create(
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

    pub(super) fn prepare_physical_create(
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

    pub(super) fn prepare_vector_replace(
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

    pub(super) fn complete_upload(
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

    pub(super) fn cancel(&mut self, token: &str, plugin_id: &str) -> Result<(), String> {
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
pub(super) fn percent_encode(value: &str) -> String {
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

pub(super) fn plugin_asset_response(
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

pub(super) fn plugin_icon_response(
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

pub(super) fn binary_asset_response(
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

pub(super) fn json_response(
    value: serde_json::Value,
    status: u16,
) -> tauri::http::Response<Vec<u8>> {
    tauri::http::Response::builder()
        .status(status)
        .header("Content-Type", "application/json")
        .header("Access-Control-Allow-Origin", "*")
        .body(serde_json::to_vec(&value).unwrap_or_else(|_| b"{}".to_vec()))
        .unwrap()
}

/// JSON response scoped to a single plugin origin instead of `*`.
pub(super) fn json_response_with_origin(
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
pub(super) fn plugin_navigation_allowed(
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
pub(super) fn plugin_request_origin_matches(
    request: &tauri::http::Request<Vec<u8>>,
    plugin_id: &str,
) -> bool {
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

pub(super) fn plugin_protocol_response(
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
pub(super) fn forward_maps_state_event(payload: &serde_json::Value) {
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

pub(super) fn plugin_asset_url(token: &str, session_id: &str, chunk: Option<u64>) -> String {
    match chunk {
        Some(index) => format!("/__asset/{token}/{index}?sessionId={session_id}"),
        None => format!("/__asset/{token}?sessionId={session_id}"),
    }
}

pub(super) fn dispatch_binary_asset_rpc(
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
