//! Bounded Atlas Studio sessions and application-controlled tile bytes.

use std::collections::BTreeMap;
use std::path::Path;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::time::{Duration, Instant};

use daena_atlas::cache::AtlasDiskCache;
use daena_atlas::overlay::{hit_test_features, AtlasInspectResult, AuthoredFeature};
use daena_atlas::studio::{
    render_studio_tile_with_style_overlays, tile_count, AtlasStudioSceneRequestV1,
    AtlasStudioTileRequestV1, CODE_STUDIO_CANCELLED, CODE_STUDIO_EXPIRED,
    CODE_STUDIO_PROTOCOL_DENIED, CODE_STUDIO_RESOURCE_LIMIT, CODE_STUDIO_TILE_FAILED,
    CODE_STUDIO_TILE_INVALID, STUDIO_MAX_DEVICE_SCALE, STUDIO_MAX_ZOOM, STUDIO_TILE_SIZE,
};
use daena_atlas::{
    prepare_from_source, AtlasError, AtlasPhase, AtlasPreparedScene, AtlasProgress, CancelFlag,
    FlagProgress,
};
use daena_core::maps::atlas::{
    atlas_cache_dir, capture_studio_session, regenerate_atlas_cache, AtlasCacheRegenerateResult,
    AtlasStudioSessionRequestV1,
};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter};
use uuid::Uuid;

use super::{current_info, with_read_project, SharedCore, ATLAS_STUDIO_PROGRESS_EVENT};

const SESSION_IDLE: Duration = Duration::from_mins(15);
const MAX_SESSIONS: usize = 4;
const MAX_PREPARED_SCENES: usize = 4;
const MAX_WAITING_TILES: u32 = 24;
const MAX_PREFETCH_WAITING: u32 = 8;
const MAX_CONCURRENT_TILE_RENDERS: u32 = 2;
const MAX_TILE_PNG_BYTES: usize = 2 * 1024 * 1024;
const MAX_TILE_CACHE_ENTRIES: usize = 64;
const MAX_TILE_CACHE_BYTES: usize = 32 * 1024 * 1024;
static WAITING_TILES: AtomicU32 = AtomicU32::new(0);

#[derive(Default)]
struct TileGate {
    active: Mutex<u32>,
    changed: Condvar,
}

struct TilePermit {
    gate: Arc<TileGate>,
}

impl TileGate {
    fn acquire(self: &Arc<Self>, prefetch: bool) -> Result<TilePermit, AtlasError> {
        let mut active = self.active.lock().map_err(|_| {
            AtlasError::new(CODE_STUDIO_TILE_FAILED, "atlas studio tile gate poisoned")
        })?;
        if prefetch {
            if *active != 0 {
                return Err(AtlasError::new(
                    CODE_STUDIO_RESOURCE_LIMIT,
                    "atlas studio prefetch deferred",
                ));
            }
        } else {
            while *active >= MAX_CONCURRENT_TILE_RENDERS {
                active = self.changed.wait(active).map_err(|_| {
                    AtlasError::new(CODE_STUDIO_TILE_FAILED, "atlas studio tile gate poisoned")
                })?;
            }
        }
        *active += 1;
        Ok(TilePermit { gate: self.clone() })
    }
}

impl Drop for TilePermit {
    fn drop(&mut self) {
        if let Ok(mut active) = self.gate.active.lock() {
            *active = active.saturating_sub(1);
            self.gate.changed.notify_one();
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AtlasStudioSessionStatus {
    pub session_token: String,
    pub map_entity_id: String,
    pub tile_url_template: String,
    pub max_zoom: u32,
    pub tile_size: u32,
    pub device_scale: u32,
    pub captured_content_generation: i64,
    pub current_content_generation: Option<i64>,
    pub style_id: String,
    pub offset_years: i64,
    pub time_kind: String,
    pub authored_year: Option<i64>,
    pub active_layer_ids: Vec<String>,
    pub projection: String,
    pub stage: String,
    pub error: Option<String>,
    pub error_code: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AtlasStudioOpenInput {
    pub request: AtlasStudioSessionRequestV1,
    pub device_scale: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct PreparedSceneKey {
    project_id: String,
    map_entity_id: String,
    physical_identity: String,
    content_generation: i64,
    offset_years: i64,
    algorithm_version: u32,
    detail_level: String,
    variant: u32,
}

struct CachedPreparedScene {
    prepared: Arc<AtlasPreparedScene>,
    last_used: Instant,
}

struct StudioSession {
    token: String,
    project_id: String,
    map_entity_id: String,
    scene: AtlasStudioSceneRequestV1,
    style: daena_atlas::style::AtlasStyle,
    style_hash: String,
    prepared: Arc<AtlasPreparedScene>,
    captured_content_generation: i64,
    device_scale: u32,
    cancel: Arc<CancelFlag>,
    last_used: Instant,
    style_id: String,
    offset_years: i64,
    time_kind: String,
    authored_year: Option<i64>,
    overlays: Arc<Vec<AuthoredFeature>>,
    tile_cache: BTreeMap<(u32, u32, u32), (Instant, Vec<u8>)>,
    tile_cache_bytes: usize,
}

#[derive(Default)]
pub struct AtlasStudioManager {
    sessions: BTreeMap<String, StudioSession>,
    prepared_scenes: BTreeMap<PreparedSceneKey, CachedPreparedScene>,
    tile_gate: Arc<TileGate>,
}

impl AtlasStudioManager {
    fn reap(&mut self) {
        let now = Instant::now();
        self.sessions.retain(|_, session| {
            if session.cancel.is_cancelled() || session.last_used + SESSION_IDLE <= now {
                session.cancel.cancel();
                false
            } else {
                true
            }
        });
        self.prepared_scenes
            .retain(|_, cached| cached.last_used + SESSION_IDLE > now);
    }

    pub fn cancel_all(&mut self) {
        for session in self.sessions.values() {
            session.cancel.cancel();
        }
        self.sessions.clear();
        self.prepared_scenes.clear();
    }

    fn cancel_map(&mut self, project_id: &str, map_entity_id: &str) {
        let doomed = self
            .sessions
            .iter()
            .filter(|(_, session)| {
                session.project_id == project_id && session.map_entity_id == map_entity_id
            })
            .map(|(token, _)| token.clone())
            .collect::<Vec<_>>();
        for token in doomed {
            if let Some(session) = self.sessions.remove(&token) {
                session.cancel.cancel();
            }
        }
    }

    fn insert(&mut self, session: StudioSession) -> Result<String, String> {
        self.reap();
        self.cancel_map(&session.project_id, &session.map_entity_id);
        while self.sessions.len() >= MAX_SESSIONS {
            let oldest = self
                .sessions
                .iter()
                .min_by_key(|(_, session)| session.last_used)
                .map(|(token, _)| token.clone());
            let Some(token) = oldest else {
                break;
            };
            if let Some(session) = self.sessions.remove(&token) {
                session.cancel.cancel();
            }
        }
        if self.sessions.len() >= MAX_SESSIONS {
            return Err(format!(
                "{CODE_STUDIO_RESOURCE_LIMIT}: atlas studio session limit reached"
            ));
        }
        let token = session.token.clone();
        self.sessions.insert(token.clone(), session);
        Ok(token)
    }

    fn close(&mut self, token: &str) {
        if let Some(session) = self.sessions.remove(token) {
            session.cancel.cancel();
        }
    }

    fn prepared_scene(&mut self, key: &PreparedSceneKey) -> Option<Arc<AtlasPreparedScene>> {
        let cached = self.prepared_scenes.get_mut(key)?;
        cached.last_used = Instant::now();
        Some(cached.prepared.clone())
    }

    fn remember_prepared_scene(
        &mut self,
        key: PreparedSceneKey,
        prepared: Arc<AtlasPreparedScene>,
    ) {
        if let Some(cached) = self.prepared_scenes.get_mut(&key) {
            cached.prepared = prepared;
            cached.last_used = Instant::now();
            return;
        }
        while self.prepared_scenes.len() >= MAX_PREPARED_SCENES {
            let oldest = self
                .prepared_scenes
                .iter()
                .min_by_key(|(_, cached)| cached.last_used)
                .map(|(key, _)| key.clone());
            let Some(oldest) = oldest else {
                break;
            };
            self.prepared_scenes.remove(&oldest);
        }
        self.prepared_scenes.insert(
            key,
            CachedPreparedScene {
                prepared,
                last_used: Instant::now(),
            },
        );
    }

    fn status(&mut self, token: &str) -> Result<AtlasStudioSessionStatus, String> {
        self.reap();
        let session = self
            .sessions
            .get(token)
            .ok_or_else(|| format!("{CODE_STUDIO_EXPIRED}: atlas studio session expired"))?;
        Ok(status_from(session, None))
    }

    fn tile_from_cache(&mut self, token: &str, z: u32, x: u32, y: u32) -> Option<Vec<u8>> {
        let session = self.sessions.get_mut(token)?;
        let entry = session.tile_cache.get_mut(&(z, x, y))?;
        entry.0 = Instant::now();
        session.last_used = Instant::now();
        Some(entry.1.clone())
    }

    fn remember_tile(&mut self, token: &str, z: u32, x: u32, y: u32, png: Vec<u8>) {
        let Some(session) = self.sessions.get_mut(token) else {
            return;
        };
        if png.len() > MAX_TILE_PNG_BYTES {
            return;
        }
        if let Some((_, old)) = session.tile_cache.remove(&(z, x, y)) {
            session.tile_cache_bytes = session.tile_cache_bytes.saturating_sub(old.len());
        }
        while session.tile_cache.len() >= MAX_TILE_CACHE_ENTRIES
            || session.tile_cache_bytes.saturating_add(png.len()) > MAX_TILE_CACHE_BYTES
        {
            let oldest = session
                .tile_cache
                .iter()
                .min_by_key(|(_, (used, _))| *used)
                .map(|(key, _)| *key);
            let Some(key) = oldest else {
                break;
            };
            if let Some((_, old)) = session.tile_cache.remove(&key) {
                session.tile_cache_bytes = session.tile_cache_bytes.saturating_sub(old.len());
            }
        }
        session.tile_cache_bytes = session.tile_cache_bytes.saturating_add(png.len());
        session.tile_cache.insert((z, x, y), (Instant::now(), png));
        session.last_used = Instant::now();
    }
}

fn status_from(
    session: &StudioSession,
    current_generation: Option<i64>,
) -> AtlasStudioSessionStatus {
    AtlasStudioSessionStatus {
        session_token: session.token.clone(),
        map_entity_id: session.map_entity_id.clone(),
        tile_url_template: tile_url_template(&session.token, session.device_scale),
        max_zoom: STUDIO_MAX_ZOOM,
        tile_size: STUDIO_TILE_SIZE,
        device_scale: session.device_scale,
        captured_content_generation: session.captured_content_generation,
        current_content_generation: current_generation,
        style_id: session.style_id.clone(),
        offset_years: session.offset_years,
        time_kind: session.time_kind.clone(),
        authored_year: session.authored_year,
        active_layer_ids: session.scene.active_layer_ids.clone(),
        projection: daena_atlas::studio::STUDIO_PROJECTION_ID.to_string(),
        stage: "ready".into(),
        error: None,
        error_code: None,
    }
}

pub fn tile_url_template(token: &str, device_scale: u32) -> String {
    #[cfg(windows)]
    {
        format!("http://atlas-studio.localhost/{token}/{{z}}/{{x}}/{{y}}.png?scale={device_scale}")
    }
    #[cfg(not(windows))]
    {
        format!("atlas-studio://localhost/{token}/{{z}}/{{x}}/{{y}}.png?scale={device_scale}")
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedStudioTile {
    pub token: String,
    pub z: u32,
    pub x: u32,
    pub y: u32,
    pub device_scale: u32,
    pub prefetch: bool,
}

pub fn parse_studio_tile_request(
    request: &tauri::http::Request<Vec<u8>>,
) -> Result<ParsedStudioTile, AtlasError> {
    if request.method() != tauri::http::Method::GET {
        return Err(protocol_denied(
            "atlas studio protocol accepts GET after OPTIONS preflight",
        ));
    }
    if !request.body().is_empty() {
        return Err(protocol_denied(
            "atlas studio protocol rejects a request body",
        ));
    }
    let path = request.uri().path().trim_start_matches('/');
    if path.contains("..") || path.contains('\\') {
        return Err(protocol_denied("atlas studio protocol refused a traversal"));
    }
    let mut parts = path.split('/');
    let token = parts
        .next()
        .filter(|part| !part.is_empty())
        .ok_or_else(|| protocol_denied("atlas studio protocol requires a session token"))?;
    Uuid::parse_str(token).map_err(|_| protocol_denied("atlas studio token is invalid"))?;
    let z = parse_tile_part(parts.next(), "z")?;
    let x = parse_tile_part(parts.next(), "x")?;
    let y_png = parts
        .next()
        .ok_or_else(|| protocol_denied("atlas studio protocol requires z/x/y.png"))?;
    if parts.next().is_some() {
        return Err(protocol_denied(
            "atlas studio protocol refused extra path segments",
        ));
    }
    let y = y_png
        .strip_suffix(".png")
        .ok_or_else(|| protocol_denied("atlas studio protocol serves PNG tiles only"))?;
    let y = y
        .parse::<u32>()
        .map_err(|_| studio_tile_invalid("atlas studio tile y is invalid"))?;
    let mut device_scale = 1;
    let mut prefetch = false;
    if let Some(query) = request.uri().query() {
        for part in query.split('&') {
            if let Some(value) = part.strip_prefix("scale=") {
                device_scale = value
                    .parse::<u32>()
                    .map_err(|_| studio_tile_invalid("atlas studio device scale must be 1 or 2"))?;
            } else if let Some(value) = part.strip_prefix("priority=") {
                prefetch = value == "prefetch";
                if value != "prefetch" && value != "visible" {
                    return Err(protocol_denied("atlas studio protocol refused a query"));
                }
            } else if !part.is_empty() {
                return Err(protocol_denied("atlas studio protocol refused a query"));
            }
        }
    }
    if device_scale == 0 || device_scale > STUDIO_MAX_DEVICE_SCALE {
        return Err(studio_tile_invalid(
            "atlas studio device scale must be 1 or 2",
        ));
    }
    if z > STUDIO_MAX_ZOOM {
        return Err(studio_tile_invalid(
            "atlas studio zoom exceeds the locked maximum",
        ));
    }
    Ok(ParsedStudioTile {
        token: token.to_string(),
        z,
        x,
        y,
        device_scale,
        prefetch,
    })
}

fn parse_tile_part(part: Option<&str>, label: &str) -> Result<u32, AtlasError> {
    let part = part
        .filter(|part| !part.is_empty())
        .ok_or_else(|| protocol_denied(format!("atlas studio protocol requires {label}")))?;
    part.parse::<u32>()
        .map_err(|_| studio_tile_invalid(format!("atlas studio tile {label} is invalid")))
}

fn protocol_denied(message: impl Into<String>) -> AtlasError {
    AtlasError::new(CODE_STUDIO_PROTOCOL_DENIED, message)
}

fn studio_tile_invalid(message: impl Into<String>) -> AtlasError {
    AtlasError::new(CODE_STUDIO_TILE_INVALID, message)
}

fn apply_cors(builder: tauri::http::response::Builder) -> tauri::http::response::Builder {
    builder
        .header("Access-Control-Allow-Origin", "*")
        .header("Access-Control-Allow-Methods", "GET, OPTIONS")
        .header("Access-Control-Allow-Headers", "*")
}

pub fn protocol_response(
    studio: &Arc<Mutex<AtlasStudioManager>>,
    core: &SharedCore,
    request: &tauri::http::Request<Vec<u8>>,
    webview_label: &str,
) -> tauri::http::Response<Vec<u8>> {
    if webview_label != "main" {
        return error_response_message(
            CODE_STUDIO_PROTOCOL_DENIED,
            "atlas studio protocol is limited to the main window",
            403,
        );
    }
    if request.method() == tauri::http::Method::OPTIONS {
        return apply_cors(tauri::http::Response::builder())
            .status(204)
            .header("Access-Control-Max-Age", "600")
            .header("Cache-Control", "no-store")
            .body(Vec::new())
            .unwrap_or_else(|_| error_response(CODE_STUDIO_TILE_FAILED, 500));
    }
    match serve_studio_tile(studio, core, request) {
        Ok(png) => apply_cors(tauri::http::Response::builder())
            .status(200)
            .header("Content-Type", "image/png")
            .header("Content-Length", png.len().to_string())
            .header("X-Content-Type-Options", "nosniff")
            .header("Cache-Control", "no-store")
            .body(png)
            .unwrap_or_else(|_| error_response(CODE_STUDIO_TILE_FAILED, 500)),
        Err(error) => {
            let status = match error.code {
                CODE_STUDIO_PROTOCOL_DENIED => 403,
                CODE_STUDIO_EXPIRED => 404,
                CODE_STUDIO_RESOURCE_LIMIT => 503,
                CODE_STUDIO_CANCELLED => 409,
                CODE_STUDIO_TILE_INVALID => 400,
                _ => 500,
            };
            error_response_message(error.code, &error.message, status)
        }
    }
}

fn error_response(code: &str, status: u16) -> tauri::http::Response<Vec<u8>> {
    error_response_message(code, code, status)
}

fn error_response_message(
    code: &str,
    message: &str,
    status: u16,
) -> tauri::http::Response<Vec<u8>> {
    let body = format!("{code}\n").into_bytes();
    apply_cors(tauri::http::Response::builder())
        .status(status)
        .header("Content-Type", "text/plain; charset=utf-8")
        .header("X-Content-Type-Options", "nosniff")
        .header("X-Atlas-Studio-Error", code)
        .header("Cache-Control", "no-store")
        .body(body)
        .unwrap_or_else(|_| tauri::http::Response::new(format!("{code}: {message}\n").into_bytes()))
}

fn serve_studio_tile(
    studio: &Arc<Mutex<AtlasStudioManager>>,
    core: &SharedCore,
    request: &tauri::http::Request<Vec<u8>>,
) -> Result<Vec<u8>, AtlasError> {
    let parsed = parse_studio_tile_request(request)?;
    {
        let mut manager = studio
            .lock()
            .map_err(|_| AtlasError::new(CODE_STUDIO_TILE_FAILED, "atlas studio lock poisoned"))?;
        manager.reap();
        if let Some(png) = manager.tile_from_cache(&parsed.token, parsed.z, parsed.x, parsed.y) {
            return Ok(png);
        }
    }
    if parsed.prefetch && WAITING_TILES.load(Ordering::SeqCst) >= MAX_PREFETCH_WAITING {
        return Err(AtlasError::new(
            CODE_STUDIO_RESOURCE_LIMIT,
            "atlas studio prefetch deferred",
        ));
    }
    let waiting = WAITING_TILES.fetch_add(1, Ordering::SeqCst) + 1;
    if waiting > MAX_WAITING_TILES || (parsed.prefetch && waiting > MAX_PREFETCH_WAITING) {
        WAITING_TILES.fetch_sub(1, Ordering::SeqCst);
        return Err(AtlasError::new(
            CODE_STUDIO_RESOURCE_LIMIT,
            "atlas studio tile queue is full",
        ));
    }
    let tile_gate = {
        let manager = studio
            .lock()
            .map_err(|_| AtlasError::new(CODE_STUDIO_TILE_FAILED, "atlas studio lock poisoned"))?;
        manager.tile_gate.clone()
    };
    let permit = tile_gate.acquire(parsed.prefetch);
    WAITING_TILES.fetch_sub(1, Ordering::SeqCst);
    let _permit = permit?;
    let current_project = current_info(core)
        .ok()
        .flatten()
        .map(|info| info.root)
        .unwrap_or_default();
    let (prepared, scene, style, style_hash, overlays, cancel, device_scale) = {
        let mut manager = studio
            .lock()
            .map_err(|_| AtlasError::new(CODE_STUDIO_TILE_FAILED, "atlas studio lock poisoned"))?;
        manager.reap();
        if let Some(png) = manager.tile_from_cache(&parsed.token, parsed.z, parsed.x, parsed.y) {
            return Ok(png);
        }
        let session = manager
            .sessions
            .get_mut(&parsed.token)
            .ok_or_else(|| AtlasError::new(CODE_STUDIO_EXPIRED, "atlas studio session expired"))?;
        if current_project != session.project_id {
            return Err(protocol_denied(
                "atlas studio token does not match the open project",
            ));
        }
        if session.cancel.is_cancelled() {
            return Err(AtlasError::new(
                CODE_STUDIO_EXPIRED,
                "atlas studio session expired",
            ));
        }
        if parsed.device_scale != session.device_scale {
            return Err(studio_tile_invalid(
                "atlas studio device scale does not match the session",
            ));
        }
        session.last_used = Instant::now();
        (
            session.prepared.clone(),
            session.scene.clone(),
            session.style.clone(),
            session.style_hash.clone(),
            session.overlays.clone(),
            session.cancel.clone(),
            session.device_scale,
        )
    };
    if cancel.is_cancelled() {
        return Err(AtlasError::new(
            CODE_STUDIO_CANCELLED,
            "atlas studio cancelled",
        ));
    }
    let tile = AtlasStudioTileRequestV1 {
        schema_version: daena_atlas::studio::ATLAS_STUDIO_TILE_SCHEMA_VERSION,
        z: parsed.z,
        x: parsed.x,
        y: parsed.y,
        tile_size: STUDIO_TILE_SIZE,
        device_scale,
        request_id: String::new(),
    };
    let mut progress = FlagProgress {
        flag: cancel.as_ref(),
    };
    let rendered = render_studio_tile_with_style_overlays(
        &prepared,
        &scene,
        &style,
        &style_hash,
        &tile,
        overlays.as_slice(),
        &mut progress,
    )
    .map_err(|error| {
        if error.code == CODE_STUDIO_CANCELLED || error.code == daena_atlas::CODE_RENDER_CANCELLED {
            AtlasError::new(CODE_STUDIO_CANCELLED, error.message)
        } else if error.code == CODE_STUDIO_TILE_INVALID || error.code == CODE_STUDIO_RESOURCE_LIMIT
        {
            error
        } else {
            AtlasError::new(CODE_STUDIO_TILE_FAILED, error.message)
        }
    })?;
    if rendered.png.len() > MAX_TILE_PNG_BYTES {
        return Err(AtlasError::new(
            CODE_STUDIO_RESOURCE_LIMIT,
            "atlas studio tile exceeded the encoded-byte budget",
        ));
    }
    let png = rendered.png;
    if let Ok(mut manager) = studio.lock() {
        manager.remember_tile(&parsed.token, parsed.z, parsed.x, parsed.y, png.clone());
    }
    Ok(png)
}

struct SessionProgress {
    app: AppHandle,
    token: String,
    map_entity_id: String,
}

impl AtlasProgress for SessionProgress {
    fn report(&mut self, phase: AtlasPhase, completed: u32, total: u32) -> Result<(), AtlasError> {
        let _ = self.app.emit(
            ATLAS_STUDIO_PROGRESS_EVENT,
            serde_json::json!({
                "sessionToken": self.token,
                "mapEntityId": self.map_entity_id,
                "stage": phase.label(),
                "completed": completed,
                "total": total,
            }),
        );
        Ok(())
    }
}

#[tauri::command]
pub async fn project_atlas_studio_open(
    state: tauri::State<'_, SharedCore>,
    studio: tauri::State<'_, Arc<Mutex<AtlasStudioManager>>>,
    app: AppHandle,
    input: AtlasStudioOpenInput,
) -> Result<AtlasStudioSessionStatus, String> {
    let project_id = current_info(state.inner())?
        .ok_or_else(|| "open a project before opening atlas studio".to_string())?
        .root;
    let device_scale = input.device_scale.unwrap_or(1);
    if device_scale == 0 || device_scale > STUDIO_MAX_DEVICE_SCALE {
        return Err(format!(
            "{}: atlas studio device scale must be 1 or 2",
            daena_atlas::studio::CODE_STUDIO_REQUEST_INVALID
        ));
    }
    let capture = with_read_project(state, move |project| {
        capture_studio_session(project, input.request)
    })
    .await?;
    let token = Uuid::new_v4().to_string();
    let prepared_key = PreparedSceneKey {
        project_id: project_id.clone(),
        map_entity_id: capture.session.map_entity_id.clone(),
        physical_identity: capture.snapshot.identity.clone(),
        content_generation: capture.snapshot.content_generation,
        offset_years: capture.scene.offset_years,
        algorithm_version: capture.scene.algorithm_version,
        detail_level: capture.scene.level.as_str().to_string(),
        variant: capture.scene.variant,
    };
    let (style, style_raw) = daena_atlas::style::load_style(&capture.scene.style_id)
        .map_err(|error| format!("{}: {}", error.code, error.message))?;
    let style_hash = style.content_hash(style_raw);
    let cache_dir = atlas_cache_dir(Path::new(&project_id));
    let identity = capture.snapshot.identity.clone();
    let source = capture.snapshot.source_bytes.clone();
    let forcing = capture.snapshot.forcing;
    let render = capture
        .scene
        .as_render_request(STUDIO_TILE_SIZE)
        .and_then(daena_atlas::request::AtlasRenderRequest::normalize)
        .map_err(|error| format!("{}: {}", error.code, error.message))?;
    let prepared = studio
        .lock()
        .map_err(|_| "atlas studio state is unavailable".to_string())?
        .prepared_scene(&prepared_key);
    let prepared = if let Some(prepared) = prepared {
        prepared
    } else {
        let mut progress = SessionProgress {
            app,
            token: token.clone(),
            map_entity_id: capture.session.map_entity_id.clone(),
        };
        let prepared = tauri::async_runtime::spawn_blocking(move || {
            let cache = AtlasDiskCache::open(&cache_dir).ok();
            prepare_from_source(
                &source,
                identity.as_bytes(),
                &render,
                Some(forcing),
                cache.as_ref(),
                &mut progress,
            )
        })
        .await
        .map_err(|error| format!("{CODE_STUDIO_TILE_FAILED}: {error}"))?
        .map_err(|error| format!("{}: {}", error.code, error.message))?;
        let prepared = Arc::new(prepared);
        studio
            .lock()
            .map_err(|_| "atlas studio state is unavailable".to_string())?
            .remember_prepared_scene(prepared_key, prepared.clone());
        prepared
    };
    let session = StudioSession {
        token: token.clone(),
        project_id,
        map_entity_id: capture.session.map_entity_id.clone(),
        scene: capture.scene,
        style,
        style_hash,
        prepared,
        captured_content_generation: capture.snapshot.content_generation,
        device_scale,
        cancel: Arc::new(CancelFlag::default()),
        last_used: Instant::now(),
        style_id: capture.session.style_id.clone(),
        offset_years: capture.session.offset_years,
        time_kind: capture.session.time_kind.clone(),
        authored_year: capture.session.authored_year,
        overlays: Arc::new(capture.snapshot.overlays),
        tile_cache: BTreeMap::new(),
        tile_cache_bytes: 0,
    };
    let status = status_from(&session, Some(capture.snapshot.content_generation));
    studio
        .lock()
        .map_err(|_| "atlas studio state is unavailable".to_string())?
        .insert(session)?;
    Ok(status)
}

#[tauri::command]
pub async fn project_atlas_studio_close(
    studio: tauri::State<'_, Arc<Mutex<AtlasStudioManager>>>,
    session_token: String,
) -> Result<(), String> {
    studio
        .lock()
        .map_err(|_| "atlas studio state is unavailable".to_string())?
        .close(&session_token);
    Ok(())
}

#[tauri::command]
pub async fn project_atlas_studio_status(
    state: tauri::State<'_, SharedCore>,
    studio: tauri::State<'_, Arc<Mutex<AtlasStudioManager>>>,
    session_token: String,
) -> Result<AtlasStudioSessionStatus, String> {
    let mut status = studio
        .lock()
        .map_err(|_| "atlas studio state is unavailable".to_string())?
        .status(&session_token)?;
    if let Ok(current) = with_read_project(state, daena_core::ProjectStore::content_generation).await {
        status.current_content_generation = Some(current);
        if current > status.captured_content_generation {
            status.error_code = Some(daena_atlas::studio::CODE_STUDIO_STALE.into());
            status.error = Some("The project changed after this Atlas session.".into());
        }
    }
    Ok(status)
}

#[tauri::command]
pub async fn project_atlas_studio_regenerate_cache(
    state: tauri::State<'_, SharedCore>,
) -> Result<AtlasCacheRegenerateResult, String> {
    with_read_project(state, regenerate_atlas_cache).await
}

pub fn cancel_atlas_studio(studio: &Arc<Mutex<AtlasStudioManager>>) -> Result<(), String> {
    studio
        .lock()
        .map_err(|_| "atlas studio state is unavailable".to_string())?
        .cancel_all();
    Ok(())
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AtlasStudioInspectInput {
    pub session_token: String,
    pub lon_micro: i32,
    pub lat_micro: i32,
    pub zoom: u32,
}

#[tauri::command]
pub async fn project_atlas_studio_inspect(
    studio: tauri::State<'_, Arc<Mutex<AtlasStudioManager>>>,
    input: AtlasStudioInspectInput,
) -> Result<AtlasInspectResult, String> {
    let (prepared, scene, overlays) = {
        let mut manager = studio
            .lock()
            .map_err(|_| "atlas studio state is unavailable".to_string())?;
        manager.reap();
        let session = manager
            .sessions
            .get(&input.session_token)
            .ok_or_else(|| format!("{CODE_STUDIO_EXPIRED}: atlas studio session expired"))?;
        (
            session.prepared.clone(),
            session.scene.clone(),
            session.overlays.clone(),
        )
    };
    let zoom = input.zoom.min(STUDIO_MAX_ZOOM);
    let n = tile_count(zoom).map_err(|error| format!("{}: {}", error.code, error.message))?;
    let radius = (360_000_000i64 * 8 / (i64::from(STUDIO_TILE_SIZE) * i64::from(n.max(1))))
        .clamp(25_000, 2_000_000) as i32;
    let include_tributaries = scene.active_layer_ids.iter().any(|id| id == "labels")
        || scene.active_layer_ids.iter().any(|id| id == "rivers");
    let surface = prepared.sample_surface(input.lon_micro, input.lat_micro);
    if include_tributaries {
        let mut features = overlays.as_ref().clone();
        for tributary in prepared.drainage.tributaries.iter().take(32) {
            if let Some(first) = tributary.path.first() {
                features.push(AuthoredFeature {
                    id: tributary.id.clone(),
                    layer_id: "labels".into(),
                    kind: "derived-tributary".into(),
                    label: Some(format!("T{}", tributary.source_cell)),
                    path: vec![*first],
                });
            }
        }
        return Ok(AtlasInspectResult {
            hits: hit_test_features(&features, input.lon_micro, input.lat_micro, radius, 32),
            surface,
        });
    }
    Ok(AtlasInspectResult {
        hits: hit_test_features(
            overlays.as_slice(),
            input.lon_micro,
            input.lat_micro,
            radius,
            32,
        ),
        surface,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn get(uri: &str) -> tauri::http::Request<Vec<u8>> {
        tauri::http::Request::builder()
            .method("GET")
            .uri(uri)
            .body(Vec::new())
            .unwrap()
    }

    #[test]
    fn tile_url_parser_accepts_png_and_denies_writes_and_traversal() {
        let token = "00000000-0000-4000-8000-0000000000ab";
        let parsed = parse_studio_tile_request(&get(&format!(
            "atlas-studio://localhost/{token}/3/2/1.png?scale=2"
        )))
        .unwrap();
        assert_eq!(parsed.z, 3);
        assert_eq!(parsed.x, 2);
        assert_eq!(parsed.y, 1);
        assert_eq!(parsed.device_scale, 2);
        assert!(!parsed.prefetch);
        let prefetched = parse_studio_tile_request(&get(&format!(
            "atlas-studio://localhost/{token}/3/2/1.png?scale=2&priority=prefetch"
        )))
        .unwrap();
        assert!(prefetched.prefetch);

        let post = tauri::http::Request::builder()
            .method("POST")
            .uri(format!("atlas-studio://localhost/{token}/0/0/0.png"))
            .body(Vec::new())
            .unwrap();
        assert_eq!(
            parse_studio_tile_request(&post).unwrap_err().code,
            CODE_STUDIO_PROTOCOL_DENIED
        );
        assert_eq!(
            parse_studio_tile_request(&get(&format!(
                "atlas-studio://localhost/{token}/../0/0.png"
            )))
            .unwrap_err()
            .code,
            CODE_STUDIO_PROTOCOL_DENIED
        );
        assert_eq!(
            parse_studio_tile_request(&get(&format!("atlas-studio://localhost/{token}/0/0/0.svg")))
                .unwrap_err()
                .code,
            CODE_STUDIO_PROTOCOL_DENIED
        );
        assert_eq!(
            parse_studio_tile_request(&get("atlas-studio://localhost/not-a-uuid/0/0/0.png"))
                .unwrap_err()
                .code,
            CODE_STUDIO_PROTOCOL_DENIED
        );
    }

    #[test]
    fn tile_gate_parallelizes_visible_tiles_and_defers_prefetch() {
        let gate = Arc::new(TileGate::default());
        let first = gate.acquire(false).unwrap();
        let second = gate.acquire(false).unwrap();
        assert_eq!(*gate.active.lock().unwrap(), 2);
        assert_eq!(
            gate.acquire(true).err().unwrap().code,
            CODE_STUDIO_RESOURCE_LIMIT
        );
        drop(first);
        drop(second);
        let prefetch = gate.acquire(true).unwrap();
        assert_eq!(
            gate.acquire(true).err().unwrap().code,
            CODE_STUDIO_RESOURCE_LIMIT
        );
        drop(prefetch);
        assert_eq!(*gate.active.lock().unwrap(), 0);
    }

    #[test]
    fn session_registry_caps_and_replaces_per_map() {
        let mut manager = AtlasStudioManager::default();
        let settings = daena_physical::GenerationSettings {
            width: 16,
            height: 8,
            radius_metres: daena_physical::DEFAULT_RADIUS_METRES,
            target_land_fraction_ppm: 300_000,
        };
        let world =
            daena_physical::generate_world(settings, 831_429, 0, &mut daena_physical::NoopProgress)
                .unwrap();
        let identity = daena_atlas::spike_identity_from_source(&world.source);
        let scene = AtlasStudioSceneRequestV1::spike().normalize().unwrap();
        let request = scene.as_render_request(STUDIO_TILE_SIZE).unwrap();
        let prepared = Arc::new(
            prepare_from_source(
                &world.source,
                &identity,
                &request,
                None,
                None,
                &mut daena_atlas::NoopProgress,
            )
            .unwrap(),
        );
        let prepared_key = PreparedSceneKey {
            project_id: "/tmp/cache".into(),
            map_entity_id: "00000000-0000-4000-8000-0000000000aa".into(),
            physical_identity: String::from_utf8_lossy(&identity).into_owned(),
            content_generation: 1,
            offset_years: scene.offset_years,
            algorithm_version: scene.algorithm_version,
            detail_level: scene.level.as_str().to_string(),
            variant: scene.variant,
        };
        manager.remember_prepared_scene(prepared_key.clone(), prepared.clone());
        let reused = manager.prepared_scene(&prepared_key).unwrap();
        assert!(Arc::ptr_eq(&reused, &prepared));
        for index in 0..4 {
            let token = Uuid::new_v4().to_string();
            manager
                .insert(StudioSession {
                    token,
                    project_id: format!("/tmp/p{index}"),
                    map_entity_id: format!("00000000-0000-4000-8000-00000000000{index}"),
                    scene: scene.clone(),
                    style: prepared.style.clone(),
                    style_hash: prepared.style_hash.clone(),
                    prepared: prepared.clone(),
                    captured_content_generation: 1,
                    device_scale: 1,
                    cancel: Arc::new(CancelFlag::default()),
                    last_used: Instant::now(),
                    style_id: scene.style_id.clone(),
                    offset_years: 0,
                    time_kind: "physical-offset-year".into(),
                    authored_year: None,
                    overlays: Arc::new(Vec::new()),
                    tile_cache: BTreeMap::new(),
                    tile_cache_bytes: 0,
                })
                .unwrap();
        }
        assert_eq!(manager.sessions.len(), 4);
        let replacement = Uuid::new_v4().to_string();
        manager
            .insert(StudioSession {
                token: replacement.clone(),
                project_id: "/tmp/p0".into(),
                map_entity_id: "00000000-0000-4000-8000-000000000000".into(),
                scene: scene.clone(),
                style: prepared.style.clone(),
                style_hash: prepared.style_hash.clone(),
                prepared: prepared.clone(),
                captured_content_generation: 1,
                device_scale: 1,
                cancel: Arc::new(CancelFlag::default()),
                last_used: Instant::now(),
                style_id: scene.style_id.clone(),
                offset_years: 0,
                time_kind: "physical-offset-year".into(),
                authored_year: None,
                overlays: Arc::new(Vec::new()),
                tile_cache: BTreeMap::new(),
                tile_cache_bytes: 0,
            })
            .unwrap();
        assert_eq!(manager.sessions.len(), 4);
        assert!(manager.sessions.contains_key(&replacement));
        manager.close(&replacement);
        assert!(!manager.sessions.contains_key(&replacement));
        manager.cancel_all();
        assert!(manager.sessions.is_empty());
        assert!(manager.prepared_scenes.is_empty());
    }

    #[test]
    fn guessed_token_is_expired_not_a_path_leak() {
        let studio = Arc::new(Mutex::new(AtlasStudioManager::default()));
        let core = crate::new_shared_core();
        let token = "00000000-0000-4000-8000-0000000000cd";
        let request = get(&format!("atlas-studio://localhost/{token}/0/0/0.png"));
        let response = protocol_response(&studio, &core, &request, "main");
        assert_eq!(response.status(), 404);
        let body = String::from_utf8_lossy(response.body());
        assert!(body.contains(CODE_STUDIO_EXPIRED));
        assert!(!body.contains("/Users"));
        assert!(!body.contains(".daena"));
        assert_eq!(
            response.headers().get("Content-Type").unwrap(),
            "text/plain; charset=utf-8"
        );
        assert_eq!(
            response.headers().get("X-Content-Type-Options").unwrap(),
            "nosniff"
        );
        assert_eq!(
            response
                .headers()
                .get("Access-Control-Allow-Origin")
                .unwrap(),
            "*"
        );
        let preflight = tauri::http::Request::builder()
            .method("OPTIONS")
            .uri(format!("atlas-studio://localhost/{token}/0/0/0.png"))
            .body(Vec::new())
            .unwrap();
        let options = protocol_response(&studio, &core, &preflight, "main");
        assert_eq!(options.status(), 204);
        assert_eq!(
            options
                .headers()
                .get("Access-Control-Allow-Origin")
                .unwrap(),
            "*"
        );
        assert!(options.body().is_empty());
        let denied = protocol_response(&studio, &core, &request, "plugin:daena.maps");
        assert_eq!(denied.status(), 403);
        assert!(String::from_utf8_lossy(denied.body()).contains(CODE_STUDIO_PROTOCOL_DENIED));
    }
}
