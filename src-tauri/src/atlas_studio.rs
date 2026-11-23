//! Bounded Atlas Studio sessions and application-controlled tile bytes.

use std::collections::BTreeMap;
use std::path::Path;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::time::{Duration, Instant};

use daena_atlas::amplify::{inspect_orometry, AmplificationModel, KIND_OROMETRY, OROMETRY_LAYER};
use daena_atlas::cache::AtlasDiskCache;
use daena_atlas::overlay::{
    hit_test_features, polygon_from_micro_rings, AtlasInspectResult, AuthoredFeature, OverlayHit,
};
use daena_atlas::studio::{
    render_studio_tile_with_style_overlays, tile_count, AtlasStudioSceneRequestV1,
    AtlasStudioTileRequestV1, CODE_STUDIO_CANCELLED, CODE_STUDIO_EXPIRED,
    CODE_STUDIO_PROTOCOL_DENIED, CODE_STUDIO_RESOURCE_LIMIT, CODE_STUDIO_STALE,
    CODE_STUDIO_TILE_FAILED, CODE_STUDIO_TILE_INVALID, STUDIO_MAX_DEVICE_SCALE, STUDIO_MAX_ZOOM,
    STUDIO_TILE_SIZE,
};
use daena_atlas::terrain::{
    render_terrain_tile_png, render_terrain_vector_tile, terrain_style_for, AtlasTerrain,
    TerrainStyle, TerrainVectorLayer, TerrainVectors, TERRAIN_MAX_ZOOM,
};
use daena_atlas::{
    prepare_from_source_with_structure, AtlasError, AtlasPhase, AtlasPreparedScene, AtlasProgress,
    CancelFlag, FlagProgress,
};
use daena_core::maps::atlas::{
    atlas_cache_dir, capture_snapshot, capture_studio_session, regenerate_atlas_cache_scoped,
    AtlasCacheRegenScope, AtlasCacheRegenerateResult, AtlasStudioSessionRequestV1,
};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter};
use uuid::Uuid;

use super::{current_info, with_read_project, SharedCore, ATLAS_STUDIO_PROGRESS_EVENT};

const SESSION_IDLE: Duration = Duration::from_mins(15);
const MAX_SESSIONS: usize = 4;
const MAX_PREPARED_SCENES: usize = 4;
const MAX_TERRAINS: usize = 2;
const MAX_WAITING_TILES: u32 = 24;
const MAX_PREFETCH_WAITING: u32 = 8;
const MAX_CONCURRENT_TILE_RENDERS: u32 = 2;
/// Vector tiles take a few milliseconds and have their own queue, so up to
/// four layers per tile position never crowd out the raster tiles.
const MAX_WAITING_VECTOR_TILES: u32 = 128;
const MAX_PREFETCH_WAITING_VECTOR: u32 = 16;
const MAX_CONCURRENT_VECTOR_RENDERS: u32 = 4;
const MAX_TILE_PNG_BYTES: usize = 2 * 1024 * 1024;
const MAX_TILE_CACHE_ENTRIES: usize = 64;
const MAX_TILE_CACHE_BYTES: usize = 32 * 1024 * 1024;
static WAITING_TILES: AtomicU32 = AtomicU32::new(0);
static WAITING_VECTOR_TILES: AtomicU32 = AtomicU32::new(0);

#[derive(Default)]
struct TileGate {
    active: Mutex<u32>,
    changed: Condvar,
}

struct TilePermit {
    gate: Arc<TileGate>,
}

impl TileGate {
    fn acquire(
        self: &Arc<Self>,
        prefetch: bool,
        max_active: u32,
    ) -> Result<TilePermit, AtlasError> {
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
            while *active >= max_active {
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
    /// `None` on legacy sessions. Terrain sessions end in `{z}/{x}/{y}.{layer}.json`.
    pub vector_url_template: Option<String>,
    pub max_zoom: u32,
    /// `"terrain"` or `"legacy"`.
    pub kind: String,
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
    constraint_fp: [u8; 32],
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct StructureKey {
    project_id: String,
    map_entity_id: String,
    physical_identity: String,
    content_generation: i64,
    algorithm_version: u32,
    detail_level: String,
    variant: u32,
}

impl PreparedSceneKey {
    fn structure_key(&self) -> StructureKey {
        StructureKey {
            project_id: self.project_id.clone(),
            map_entity_id: self.map_entity_id.clone(),
            physical_identity: self.physical_identity.clone(),
            content_generation: self.content_generation,
            algorithm_version: self.algorithm_version,
            detail_level: self.detail_level.clone(),
            variant: self.variant,
        }
    }
}

struct CachedPreparedScene {
    prepared: Arc<AtlasPreparedScene>,
    last_used: Instant,
}

struct CachedStructure {
    model: Arc<AmplificationModel>,
    last_used: Instant,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct TerrainKey {
    project_id: String,
    map_entity_id: String,
    physical_identity: String,
    content_generation: i64,
    normalized_epoch: i64,
}

struct CachedTerrain {
    terrain: Arc<AtlasTerrain>,
    vectors: Arc<TerrainVectors>,
    last_used: Instant,
}

enum SessionBody {
    Legacy {
        prepared: Arc<AtlasPreparedScene>,
        style: daena_atlas::style::AtlasStyle,
        style_hash: String,
    },
    Terrain {
        terrain: Arc<AtlasTerrain>,
        vectors: Arc<TerrainVectors>,
        style: TerrainStyle,
    },
}

type StudioTileCache = BTreeMap<(u32, u32, u32, u8), (Instant, Vec<u8>)>;

struct StudioSession {
    token: String,
    project_id: String,
    map_entity_id: String,
    scene: AtlasStudioSceneRequestV1,
    body: SessionBody,
    physical_identity: String,
    captured_content_generation: i64,
    device_scale: u32,
    cancel: Arc<CancelFlag>,
    last_used: Instant,
    style_id: String,
    offset_years: i64,
    time_kind: String,
    authored_year: Option<i64>,
    overlays: Arc<Vec<AuthoredFeature>>,
    tile_cache: StudioTileCache,
    tile_cache_bytes: usize,
}

/// What inspect and region proposal read for one session.
enum InspectScene {
    Legacy(Arc<AtlasPreparedScene>),
    Terrain(Arc<AtlasTerrain>),
}

impl InspectScene {
    fn tool_surface(&self) -> &daena_physical::engine::ToolSurface {
        match self {
            Self::Legacy(prepared) => &prepared.surface,
            Self::Terrain(terrain) => terrain.surface(),
        }
    }
}

impl SessionBody {
    fn inspect_scene(&self) -> InspectScene {
        match self {
            Self::Legacy { prepared, .. } => InspectScene::Legacy(prepared.clone()),
            Self::Terrain { terrain, .. } => InspectScene::Terrain(terrain.clone()),
        }
    }
}

impl StudioSession {
    fn max_zoom(&self) -> u32 {
        session_max_zoom(self.kind_name())
    }

    fn kind_name(&self) -> &'static str {
        match self.body {
            SessionBody::Legacy { .. } => "legacy",
            SessionBody::Terrain { .. } => "terrain",
        }
    }
}

/// Session zoom cap: 12 for terrain, [`STUDIO_MAX_ZOOM`] for legacy.
fn session_max_zoom(kind: &str) -> u32 {
    if kind == "terrain" {
        TERRAIN_MAX_ZOOM
    } else {
        STUDIO_MAX_ZOOM
    }
}

#[derive(Default)]
pub struct AtlasStudioManager {
    sessions: BTreeMap<String, StudioSession>,
    prepared_scenes: BTreeMap<PreparedSceneKey, CachedPreparedScene>,
    structures: BTreeMap<StructureKey, CachedStructure>,
    terrains: BTreeMap<TerrainKey, CachedTerrain>,
    tile_gate: Arc<TileGate>,
    vector_gate: Arc<TileGate>,
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
        self.structures
            .retain(|_, cached| cached.last_used + SESSION_IDLE > now);
        self.terrains
            .retain(|_, cached| cached.last_used + SESSION_IDLE > now);
    }

    pub fn cancel_all(&mut self) {
        for session in self.sessions.values() {
            session.cancel.cancel();
        }
        self.sessions.clear();
        self.prepared_scenes.clear();
        self.structures.clear();
        self.terrains.clear();
    }

    fn clear_visible_tiles(&mut self) -> u32 {
        let mut deleted = 0_u32;
        for session in self.sessions.values_mut() {
            deleted = deleted.saturating_add(session.tile_cache.len() as u32);
            session.tile_cache.clear();
        }
        deleted
    }

    fn forget_sessions(&mut self, drop_structures: bool) {
        for session in self.sessions.values() {
            session.cancel.cancel();
        }
        self.sessions.clear();
        self.prepared_scenes.clear();
        self.terrains.clear();
        if drop_structures {
            self.structures.clear();
        }
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

    fn terrain(&mut self, key: &TerrainKey) -> Option<(Arc<AtlasTerrain>, Arc<TerrainVectors>)> {
        let cached = self.terrains.get_mut(key)?;
        cached.last_used = Instant::now();
        Some((cached.terrain.clone(), cached.vectors.clone()))
    }

    fn remember_terrain(
        &mut self,
        key: TerrainKey,
        terrain: Arc<AtlasTerrain>,
        vectors: Arc<TerrainVectors>,
    ) {
        if let Some(cached) = self.terrains.get_mut(&key) {
            cached.last_used = Instant::now();
            return;
        }
        while self.terrains.len() >= MAX_TERRAINS {
            let oldest = self
                .terrains
                .iter()
                .min_by_key(|(_, cached)| cached.last_used)
                .map(|(key, _)| key.clone());
            let Some(oldest) = oldest else {
                break;
            };
            self.terrains.remove(&oldest);
        }
        self.terrains.insert(
            key,
            CachedTerrain {
                terrain,
                vectors,
                last_used: Instant::now(),
            },
        );
    }

    fn structure(&mut self, key: &StructureKey) -> Option<Arc<AmplificationModel>> {
        let cached = self.structures.get_mut(key)?;
        cached.last_used = Instant::now();
        Some(cached.model.clone())
    }

    fn remember_structure(&mut self, key: StructureKey, model: Arc<AmplificationModel>) {
        if let Some(cached) = self.structures.get_mut(&key) {
            cached.model = model;
            cached.last_used = Instant::now();
            return;
        }
        while self.structures.len() >= MAX_PREPARED_SCENES {
            let oldest = self
                .structures
                .iter()
                .min_by_key(|(_, cached)| cached.last_used)
                .map(|(key, _)| key.clone());
            let Some(oldest) = oldest else {
                break;
            };
            self.structures.remove(&oldest);
        }
        self.structures.insert(
            key,
            CachedStructure {
                model,
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

    fn tile_from_cache(
        &mut self,
        token: &str,
        z: u32,
        x: u32,
        y: u32,
        kind: u8,
    ) -> Option<Vec<u8>> {
        let session = self.sessions.get_mut(token)?;
        let entry = session.tile_cache.get_mut(&(z, x, y, kind))?;
        entry.0 = Instant::now();
        session.last_used = Instant::now();
        Some(entry.1.clone())
    }

    fn remember_tile(&mut self, token: &str, z: u32, x: u32, y: u32, kind: u8, png: Vec<u8>) {
        let Some(session) = self.sessions.get_mut(token) else {
            return;
        };
        if png.len() > MAX_TILE_PNG_BYTES {
            return;
        }
        if let Some((_, old)) = session.tile_cache.remove(&(z, x, y, kind)) {
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
        session
            .tile_cache
            .insert((z, x, y, kind), (Instant::now(), png));
        session.last_used = Instant::now();
    }

    fn adopt_overlay_snapshot(
        &mut self,
        token: &str,
        identity: &str,
        offset_years: i64,
        overlays: Vec<AuthoredFeature>,
        content_generation: i64,
    ) -> Result<bool, String> {
        let session = self
            .sessions
            .get_mut(token)
            .ok_or_else(|| format!("{CODE_STUDIO_EXPIRED}: atlas studio session expired"))?;
        if session.physical_identity != identity || session.offset_years != offset_years {
            return Ok(false);
        }
        session.overlays = Arc::new(overlays);
        session.captured_content_generation = content_generation;
        session.last_used = Instant::now();
        Ok(true)
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
        vector_url_template: (session.kind_name() == "terrain")
            .then(|| vector_url_template(&session.token)),
        max_zoom: session.max_zoom(),
        kind: session.kind_name().to_string(),
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

pub fn vector_url_template(token: &str) -> String {
    #[cfg(windows)]
    {
        format!("http://atlas-studio.localhost/{token}/{{z}}/{{x}}/{{y}}.{{layer}}.json")
    }
    #[cfg(not(windows))]
    {
        format!("atlas-studio://localhost/{token}/{{z}}/{{x}}/{{y}}.{{layer}}.json")
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
    /// `None` is a PNG tile. `Some` is `y.<layer>.json`.
    pub layer: Option<TerrainVectorLayer>,
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
    let (y, layer) = parse_y_and_layer(y_png)?;
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
    if z > TERRAIN_MAX_ZOOM {
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
        layer,
    })
}

fn parse_y_and_layer(part: &str) -> Result<(u32, Option<TerrainVectorLayer>), AtlasError> {
    if let Some(y) = part.strip_suffix(".png") {
        let y = y
            .parse::<u32>()
            .map_err(|_| studio_tile_invalid("atlas studio tile y is invalid"))?;
        return Ok((y, None));
    }
    if let Some(stem) = part.strip_suffix(".json") {
        let (y, layer_id) = stem
            .rsplit_once('.')
            .ok_or_else(|| studio_tile_invalid("atlas studio vector tile name is invalid"))?;
        let y = y
            .parse::<u32>()
            .map_err(|_| studio_tile_invalid("atlas studio tile y is invalid"))?;
        let layer = TerrainVectorLayer::from_id(layer_id)
            .ok_or_else(|| studio_tile_invalid("atlas studio vector layer is invalid"))?;
        return Ok((y, Some(layer)));
    }
    Err(protocol_denied(
        "atlas studio protocol serves PNG tiles only",
    ))
}

fn tile_cache_kind(layer: Option<TerrainVectorLayer>) -> u8 {
    match layer {
        None => 0,
        Some(TerrainVectorLayer::Coastline) => 1,
        Some(TerrainVectorLayer::Lakes) => 2,
        Some(TerrainVectorLayer::Rivers) => 3,
        Some(TerrainVectorLayer::Contours) => 4,
        Some(TerrainVectorLayer::Streams) => 5,
    }
}

fn tile_content_type(layer: Option<TerrainVectorLayer>) -> &'static str {
    if layer.is_some() {
        "application/geo+json"
    } else {
        "image/png"
    }
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
        Ok(tile) => apply_cors(tauri::http::Response::builder())
            .status(200)
            .header("Content-Type", tile.content_type)
            .header("Content-Length", tile.bytes.len().to_string())
            .header("X-Content-Type-Options", "nosniff")
            .header("Cache-Control", "no-store")
            .body(tile.bytes)
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

fn emit_terrain_progress(
    app: &AppHandle,
    token: &str,
    map_entity_id: &str,
    completed: u32,
    total: u32,
) {
    let _ = app.emit(
        ATLAS_STUDIO_PROGRESS_EVENT,
        serde_json::json!({
            "sessionToken": token,
            "mapEntityId": map_entity_id,
            "stage": "terrain",
            "completed": completed,
            "total": total,
        }),
    );
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

struct ServedTile {
    bytes: Vec<u8>,
    content_type: &'static str,
}

enum TileWork {
    Legacy {
        prepared: Arc<AtlasPreparedScene>,
        scene: AtlasStudioSceneRequestV1,
        style: Box<daena_atlas::style::AtlasStyle>,
        style_hash: String,
        overlays: Arc<Vec<AuthoredFeature>>,
        cancel: Arc<CancelFlag>,
        device_scale: u32,
    },
    Terrain {
        terrain: Arc<AtlasTerrain>,
        vectors: Arc<TerrainVectors>,
        style: TerrainStyle,
        cancel: Arc<CancelFlag>,
        device_scale: u32,
        layer: Option<TerrainVectorLayer>,
    },
}

impl TileWork {
    fn cancel(&self) -> &CancelFlag {
        match self {
            Self::Legacy { cancel, .. } | Self::Terrain { cancel, .. } => cancel,
        }
    }
}

fn reject_session_tile(
    session: &StudioSession,
    parsed: &ParsedStudioTile,
) -> Result<(), AtlasError> {
    reject_studio_tile_for_kind(session.kind_name(), parsed)
}

/// Shared by the session gate and the protocol tests.
pub(crate) fn reject_studio_tile_for_kind(
    kind: &str,
    parsed: &ParsedStudioTile,
) -> Result<(), AtlasError> {
    if parsed.layer.is_some() && kind != "terrain" {
        return Err(studio_tile_invalid(
            "atlas studio vector tiles require a terrain session",
        ));
    }
    if parsed.z > session_max_zoom(kind) {
        return Err(studio_tile_invalid(
            "atlas studio zoom exceeds the locked maximum",
        ));
    }
    if !tile_index_in_range(parsed.z, parsed.x, parsed.y) {
        return Err(studio_tile_invalid("atlas studio tile is outside the zoom"));
    }
    Ok(())
}

fn tile_index_in_range(z: u32, x: u32, y: u32) -> bool {
    let Some(n) = 1u32.checked_shl(z) else {
        return false;
    };
    x < n && y < n
}

fn serve_studio_tile(
    studio: &Arc<Mutex<AtlasStudioManager>>,
    core: &SharedCore,
    request: &tauri::http::Request<Vec<u8>>,
) -> Result<ServedTile, AtlasError> {
    let parsed = parse_studio_tile_request(request)?;
    {
        let mut manager = studio
            .lock()
            .map_err(|_| AtlasError::new(CODE_STUDIO_TILE_FAILED, "atlas studio lock poisoned"))?;
        manager.reap();
        if let Some(session) = manager.sessions.get(&parsed.token) {
            reject_session_tile(session, &parsed)?;
        }
        if let Some(bytes) = manager.tile_from_cache(
            &parsed.token,
            parsed.z,
            parsed.x,
            parsed.y,
            tile_cache_kind(parsed.layer),
        ) {
            return Ok(ServedTile {
                bytes,
                content_type: tile_content_type(parsed.layer),
            });
        }
    }
    let vector = parsed.layer.is_some();
    let (counter, max_waiting, max_prefetch_waiting, max_active) = if vector {
        (
            &WAITING_VECTOR_TILES,
            MAX_WAITING_VECTOR_TILES,
            MAX_PREFETCH_WAITING_VECTOR,
            MAX_CONCURRENT_VECTOR_RENDERS,
        )
    } else {
        (
            &WAITING_TILES,
            MAX_WAITING_TILES,
            MAX_PREFETCH_WAITING,
            MAX_CONCURRENT_TILE_RENDERS,
        )
    };
    if parsed.prefetch && counter.load(Ordering::SeqCst) >= max_prefetch_waiting {
        return Err(AtlasError::new(
            CODE_STUDIO_RESOURCE_LIMIT,
            "atlas studio prefetch deferred",
        ));
    }
    struct WaitingTileGuard(&'static AtomicU32);
    impl Drop for WaitingTileGuard {
        fn drop(&mut self) {
            self.0.fetch_sub(1, Ordering::SeqCst);
        }
    }

    let waiting = counter.fetch_add(1, Ordering::SeqCst) + 1;
    let waiting_guard = WaitingTileGuard(counter);
    if waiting > max_waiting || (parsed.prefetch && waiting > max_prefetch_waiting) {
        return Err(AtlasError::new(
            CODE_STUDIO_RESOURCE_LIMIT,
            "atlas studio tile queue is full",
        ));
    }
    let tile_gate = {
        let manager = studio
            .lock()
            .map_err(|_| AtlasError::new(CODE_STUDIO_TILE_FAILED, "atlas studio lock poisoned"))?;
        if vector {
            manager.vector_gate.clone()
        } else {
            manager.tile_gate.clone()
        }
    };
    let permit = tile_gate.acquire(parsed.prefetch, max_active);
    drop(waiting_guard);
    let _permit = permit?;
    let current_project = current_info(core)
        .ok()
        .flatten()
        .map(|info| info.root)
        .unwrap_or_default();
    let job = {
        let mut manager = studio
            .lock()
            .map_err(|_| AtlasError::new(CODE_STUDIO_TILE_FAILED, "atlas studio lock poisoned"))?;
        manager.reap();
        if let Some(bytes) = manager.tile_from_cache(
            &parsed.token,
            parsed.z,
            parsed.x,
            parsed.y,
            tile_cache_kind(parsed.layer),
        ) {
            return Ok(ServedTile {
                bytes,
                content_type: tile_content_type(parsed.layer),
            });
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
        if parsed.layer.is_none() && parsed.device_scale != session.device_scale {
            return Err(studio_tile_invalid(
                "atlas studio device scale does not match the session",
            ));
        }
        reject_session_tile(session, &parsed)?;
        session.last_used = Instant::now();
        match &session.body {
            SessionBody::Legacy {
                prepared,
                style,
                style_hash,
            } => TileWork::Legacy {
                prepared: prepared.clone(),
                scene: session.scene.clone(),
                style: Box::new(style.clone()),
                style_hash: style_hash.clone(),
                overlays: session.overlays.clone(),
                cancel: session.cancel.clone(),
                device_scale: session.device_scale,
            },
            SessionBody::Terrain {
                terrain,
                vectors,
                style,
            } => TileWork::Terrain {
                terrain: terrain.clone(),
                vectors: vectors.clone(),
                style: *style,
                cancel: session.cancel.clone(),
                device_scale: session.device_scale,
                layer: parsed.layer,
            },
        }
    };
    if job.cancel().is_cancelled() {
        return Err(AtlasError::new(
            CODE_STUDIO_CANCELLED,
            "atlas studio cancelled",
        ));
    }
    let rendered_png = match job {
        TileWork::Legacy {
            prepared,
            scene,
            style,
            style_hash,
            overlays,
            cancel,
            device_scale,
        } => {
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
                if error.code == CODE_STUDIO_CANCELLED
                    || error.code == daena_atlas::CODE_RENDER_CANCELLED
                {
                    AtlasError::new(CODE_STUDIO_CANCELLED, error.message)
                } else if error.code == CODE_STUDIO_TILE_INVALID
                    || error.code == CODE_STUDIO_RESOURCE_LIMIT
                {
                    error
                } else {
                    AtlasError::new(CODE_STUDIO_TILE_FAILED, error.message)
                }
            })?;
            if cancel.is_cancelled() {
                return Err(AtlasError::new(
                    CODE_STUDIO_CANCELLED,
                    "atlas studio cancelled",
                ));
            }
            rendered.png
        }
        TileWork::Terrain {
            terrain,
            vectors,
            style,
            cancel,
            device_scale,
            layer,
        } => {
            let bytes = if let Some(layer) = layer {
                render_terrain_vector_tile(&terrain, &vectors, layer, parsed.z, parsed.x, parsed.y)
            } else {
                render_terrain_tile_png(&terrain, style, parsed.z, parsed.x, parsed.y, device_scale)
            }
            .map_err(|error| {
                if error.code == daena_atlas::CODE_REQUEST_INVALID {
                    studio_tile_invalid(error.message)
                } else {
                    AtlasError::new(CODE_STUDIO_TILE_FAILED, error.message)
                }
            })?;
            if cancel.is_cancelled() {
                return Err(AtlasError::new(
                    CODE_STUDIO_CANCELLED,
                    "atlas studio cancelled",
                ));
            }
            bytes
        }
    };
    if rendered_png.len() > MAX_TILE_PNG_BYTES {
        return Err(AtlasError::new(
            CODE_STUDIO_RESOURCE_LIMIT,
            "atlas studio tile exceeded the encoded-byte budget",
        ));
    }
    let bytes = rendered_png;
    if let Ok(mut manager) = studio.lock() {
        manager.remember_tile(
            &parsed.token,
            parsed.z,
            parsed.x,
            parsed.y,
            tile_cache_kind(parsed.layer),
            bytes.clone(),
        );
    }
    Ok(ServedTile {
        bytes,
        content_type: tile_content_type(parsed.layer),
    })
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
    let (mut capture, generation) = with_read_project(state, move |project| {
        let capture = capture_studio_session(project, input.request)?;
        let generation = if terrain_style_for(&capture.scene.style_id).is_some() {
            Some(super::physical_generation_for_map(
                project,
                &capture.session.map_entity_id,
            )?)
        } else {
            None
        };
        Ok((capture, generation))
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
        constraint_fp: daena_atlas::constraint::fingerprint(&capture.snapshot.constraints),
    };
    let (style, style_raw) = daena_atlas::style::load_style(&capture.scene.style_id)
        .map_err(|error| format!("{}: {}", error.code, error.message))?;
    let style_hash = style.content_hash(style_raw);
    let cache_dir = atlas_cache_dir(Path::new(&project_id));
    let identity = capture.snapshot.identity.clone();
    let forcing = capture.snapshot.forcing;
    let mut render = capture
        .scene
        .as_render_request(STUDIO_TILE_SIZE)
        .and_then(daena_atlas::request::AtlasRenderRequest::normalize)
        .map_err(|error| format!("{}: {}", error.code, error.message))?;
    render.constraints = capture.snapshot.constraints.clone();
    if matches!(render.level, daena_atlas::request::DetailLevel::Print) {
        return Err(format!(
            "{}: atlas studio does not build print",
            daena_atlas::studio::CODE_STUDIO_REQUEST_INVALID
        ));
    }
    let structure_key = prepared_key.structure_key();
    let prepared = studio
        .lock()
        .map_err(|_| "atlas studio state is unavailable".to_string())?
        .prepared_scene(&prepared_key);
    let body = if let Some(terrain_style) = terrain_style_for(&capture.scene.style_id) {
        let generation = generation.ok_or_else(|| {
            format!("{CODE_STUDIO_TILE_FAILED}: atlas studio terrain generation is missing")
        })?;
        let normalized_epoch =
            daena_physical::history::normalize_epoch_offset(capture.session.offset_years)
                .map_err(|error| error.to_string())?;
        let terrain_key = TerrainKey {
            project_id: project_id.clone(),
            map_entity_id: capture.session.map_entity_id.clone(),
            physical_identity: capture.snapshot.identity.clone(),
            content_generation: capture.snapshot.content_generation,
            normalized_epoch,
        };
        let cached = studio
            .lock()
            .map_err(|_| "atlas studio state is unavailable".to_string())?
            .terrain(&terrain_key);
        let (terrain, vectors) = if let Some(cached) = cached {
            emit_terrain_progress(&app, &token, &capture.session.map_entity_id, 1, 1);
            cached
        } else {
            emit_terrain_progress(&app, &token, &capture.session.map_entity_id, 0, 1);
            let source_bytes = std::mem::take(&mut capture.snapshot.source_bytes);
            let expected_identity = capture.snapshot.identity.clone();
            let offset_years = capture.session.offset_years;
            let built = tauri::async_runtime::spawn_blocking(move || {
                let validated =
                    daena_core::maps::physical::validate_source(&source_bytes, &generation)
                        .map_err(|error| error.to_string())?;
                if validated.identity != expected_identity {
                    return Err("physical identity changed while opening atlas studio".to_string());
                }
                let surface = super::tool_surface_for_source(
                    &validated.identity,
                    &validated.source,
                    &generation,
                    offset_years,
                )?;
                let terrain = Arc::new(AtlasTerrain::new(surface, validated.identity.as_bytes()));
                let vectors = Arc::new(TerrainVectors::new(&terrain));
                Ok((terrain, vectors))
            })
            .await
            .map_err(|error| format!("{CODE_STUDIO_TILE_FAILED}: {error}"))??;
            emit_terrain_progress(&app, &token, &capture.session.map_entity_id, 1, 1);
            let mut studio = studio
                .lock()
                .map_err(|_| "atlas studio state is unavailable".to_string())?;
            studio.remember_terrain(terrain_key, built.0.clone(), built.1.clone());
            built
        };
        SessionBody::Terrain {
            terrain,
            vectors,
            style: terrain_style,
        }
    } else {
        let prepared = if let Some(prepared) = prepared {
            prepared
        } else {
            let source = capture.snapshot.source_bytes.clone();
            let injected = studio
                .lock()
                .map_err(|_| "atlas studio state is unavailable".to_string())?
                .structure(&structure_key);
            let injected_for_job = injected.clone();
            let mut progress = SessionProgress {
                app,
                token: token.clone(),
                map_entity_id: capture.session.map_entity_id.clone(),
            };
            let prepared = tauri::async_runtime::spawn_blocking(move || {
                let cache = AtlasDiskCache::open(&cache_dir).ok();
                prepare_from_source_with_structure(
                    &source,
                    identity.as_bytes(),
                    &render,
                    Some(forcing),
                    cache.as_ref(),
                    injected_for_job.as_deref(),
                    &mut progress,
                )
            })
            .await
            .map_err(|error| format!("{CODE_STUDIO_TILE_FAILED}: {error}"))?
            .map_err(|error| format!("{}: {}", error.code, error.message))?;
            let (prepared, unbaked) = prepared;
            let prepared = Arc::new(prepared);
            let structure = injected.unwrap_or_else(|| Arc::new(unbaked));
            let mut studio = studio
                .lock()
                .map_err(|_| "atlas studio state is unavailable".to_string())?;
            studio.remember_prepared_scene(prepared_key, prepared.clone());
            studio.remember_structure(structure_key, structure);
            prepared
        };
        SessionBody::Legacy {
            prepared,
            style,
            style_hash,
        }
    };
    let session = StudioSession {
        token: token.clone(),
        project_id,
        map_entity_id: capture.session.map_entity_id.clone(),
        scene: capture.scene,
        body,
        physical_identity: capture.snapshot.identity.clone(),
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
    let Ok(current) =
        with_read_project(state.clone(), daena_core::ProjectStore::content_generation).await
    else {
        return Ok(status);
    };
    status.current_content_generation = Some(current);
    if current <= status.captured_content_generation {
        return Ok(status);
    }
    let (map_entity_id, render) = {
        let guard = studio
            .lock()
            .map_err(|_| "atlas studio state is unavailable".to_string())?;
        let session = guard
            .sessions
            .get(&session_token)
            .ok_or_else(|| format!("{CODE_STUDIO_EXPIRED}: atlas studio session expired"))?;
        (
            session.map_entity_id.clone(),
            session
                .scene
                .as_render_request(STUDIO_TILE_SIZE)
                .and_then(daena_atlas::request::AtlasRenderRequest::normalize)
                .map_err(|error| format!("{}: {}", error.code, error.message))?,
        )
    };
    let snapshot = with_read_project(state, move |project| {
        capture_snapshot(project, &map_entity_id, render)
    })
    .await?;
    let mut guard = studio
        .lock()
        .map_err(|_| "atlas studio state is unavailable".to_string())?;
    if guard.adopt_overlay_snapshot(
        &session_token,
        &snapshot.identity,
        snapshot.request.offset_years,
        snapshot.overlays,
        snapshot.content_generation,
    )? {
        let session = guard
            .sessions
            .get(&session_token)
            .ok_or_else(|| format!("{CODE_STUDIO_EXPIRED}: atlas studio session expired"))?;
        return Ok(status_from(session, Some(snapshot.content_generation)));
    }
    status.error_code = Some(CODE_STUDIO_STALE.into());
    status.error = Some("The project changed after this Atlas session.".into());
    Ok(status)
}

#[tauri::command]
pub async fn project_atlas_studio_regenerate_cache(
    state: tauri::State<'_, SharedCore>,
    studio: tauri::State<'_, Arc<Mutex<AtlasStudioManager>>>,
    scope: Option<String>,
) -> Result<AtlasCacheRegenerateResult, String> {
    let scope = AtlasCacheRegenScope::parse(scope.as_deref().unwrap_or("world"))
        .map_err(|error| error.to_string())?;
    if matches!(scope, AtlasCacheRegenScope::Visible) {
        let deleted = studio
            .lock()
            .map_err(|_| "atlas studio state is unavailable".to_string())?
            .clear_visible_tiles();
        return Ok(AtlasCacheRegenerateResult {
            deleted_entries: deleted,
        });
    }
    let result = with_read_project(state, move |project| {
        regenerate_atlas_cache_scoped(project, scope)
    })
    .await?;
    let mut studio = studio
        .lock()
        .map_err(|_| "atlas studio state is unavailable".to_string())?;
    studio.clear_visible_tiles();
    match scope {
        AtlasCacheRegenScope::Visible => {}
        AtlasCacheRegenScope::World => studio.forget_sessions(true),
        AtlasCacheRegenScope::Epoch | AtlasCacheRegenScope::Feature => {
            studio.forget_sessions(false)
        }
    }
    Ok(result)
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
    let (inspect, scene, overlays, max_zoom) = {
        let mut manager = studio
            .lock()
            .map_err(|_| "atlas studio state is unavailable".to_string())?;
        manager.reap();
        let session = manager
            .sessions
            .get(&input.session_token)
            .ok_or_else(|| format!("{CODE_STUDIO_EXPIRED}: atlas studio session expired"))?;
        (
            session.body.inspect_scene(),
            session.scene.clone(),
            session.overlays.clone(),
            session.max_zoom(),
        )
    };
    let zoom = input.zoom.min(max_zoom);
    let n = tile_count(zoom).map_err(|error| format!("{}: {}", error.code, error.message))?;
    let radius = (360_000_000i64 * 8 / (i64::from(STUDIO_TILE_SIZE) * i64::from(n.max(1))))
        .clamp(25_000, 2_000_000) as i32;
    let include_tributaries = scene.active_layer_ids.iter().any(|id| id == "labels")
        || scene.active_layer_ids.iter().any(|id| id == "rivers");
    let surface = match &inspect {
        InspectScene::Legacy(prepared) => prepared.sample_surface(input.lon_micro, input.lat_micro),
        InspectScene::Terrain(terrain) => {
            terrain.inspect_sample(input.lon_micro, input.lat_micro, zoom)
        }
    };
    let mut features = overlays.as_ref().clone();
    if let (true, InspectScene::Legacy(prepared)) = (include_tributaries, &inspect) {
        for tributary in prepared.drainage.tributaries.iter().take(32) {
            if let Some(first) = tributary.path.first() {
                features.push(AuthoredFeature {
                    id: tributary.id.clone(),
                    layer_id: "labels".into(),
                    kind: "derived-tributary".into(),
                    label: Some(format!("T{}", tributary.source_cell)),
                    path: vec![*first],
                    fill: None,
                    closed: false,
                });
            }
        }
    }
    let mut hits = hit_test_features(&features, input.lon_micro, input.lat_micro, radius, 32);
    let tool_surface = inspect.tool_surface();
    let claim = daena_physical::hydro_claim::claim_at_canonical(
        tool_surface,
        input.lon_micro,
        input.lat_micro,
        radius,
    )
    .map(|claim| (claim.id, claim.layer_id, claim.kind, claim.label))
    .or_else(|| {
        daena_physical::landmass_claim::claim_at_canonical(
            tool_surface,
            input.lon_micro,
            input.lat_micro,
        )
        .map(|claim| (claim.id, claim.layer_id, claim.kind, claim.label))
    });
    let has_claim = claim.is_some();
    if let Some((id, layer_id, kind, label)) = claim {
        if !hits.iter().any(|hit| hit.id == id) {
            hits.insert(
                0,
                OverlayHit {
                    id,
                    layer_id: layer_id.into(),
                    kind: kind.into(),
                    label: Some(label),
                    derived: false,
                },
            );
            if hits.len() > 32 {
                hits.truncate(32);
            }
        }
    }
    let mut flood_hit = false;
    if let Some(flood) = daena_physical::hydro_claim::overflowing_at_canonical(
        tool_surface,
        input.lon_micro,
        input.lat_micro,
    ) {
        let insert_at = usize::from(has_claim);
        if let Some(pos) = hits.iter().position(|hit| hit.id == flood.id) {
            let hit = hits.remove(pos);
            hits.insert(insert_at.min(hits.len()), hit);
        } else {
            hits.insert(
                insert_at.min(hits.len()),
                OverlayHit {
                    id: flood.id,
                    layer_id: flood.layer_id.into(),
                    kind: flood.kind.into(),
                    label: Some(flood.label),
                    derived: true,
                },
            );
            if hits.len() > 32 {
                hits.truncate(32);
            }
        }
        flood_hit = true;
    }
    let orometry_at = usize::from(has_claim) + usize::from(flood_hit);
    let orometry = match &inspect {
        InspectScene::Legacy(prepared) => {
            inspect_orometry(&prepared.orometry, input.lon_micro, input.lat_micro, radius)
        }
        InspectScene::Terrain(_) => Vec::new(),
    };
    let orometry_hits = orometry
        .into_iter()
        .filter(|hit| !hits.iter().any(|existing| existing.id == hit.id))
        .map(|hit| OverlayHit {
            id: hit.id,
            layer_id: OROMETRY_LAYER.into(),
            kind: KIND_OROMETRY.into(),
            label: Some(hit.label),
            derived: true,
        })
        .collect::<Vec<_>>();
    hits.splice(orometry_at..orometry_at, orometry_hits);
    hits.truncate(32);
    Ok(AtlasInspectResult { hits, surface })
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AtlasStudioProposeInput {
    pub session_token: String,
    pub lon_micro: i32,
    pub lat_micro: i32,
    pub detector: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AtlasRegionProposal {
    pub detector: String,
    pub region_id: u32,
    pub label: String,
    pub epoch_dependent: bool,
    pub cell_count: u32,
    pub geometry: serde_json::Value,
}

#[tauri::command]
pub async fn project_atlas_studio_propose_region(
    studio: tauri::State<'_, Arc<Mutex<AtlasStudioManager>>>,
    input: AtlasStudioProposeInput,
) -> Result<AtlasRegionProposal, String> {
    let inspect = {
        let mut manager = studio
            .lock()
            .map_err(|_| "atlas studio state is unavailable".to_string())?;
        manager.reap();
        let session = manager
            .sessions
            .get(&input.session_token)
            .ok_or_else(|| format!("{CODE_STUDIO_EXPIRED}: atlas studio session expired"))?;
        session.body.inspect_scene()
    };
    let proposal = daena_physical::region::propose_region_canonical(
        inspect.tool_surface(),
        input.lon_micro,
        input.lat_micro,
        &input.detector,
        daena_physical::engine::PRODUCT_VIEW_WIDTH,
        daena_physical::engine::PRODUCT_VIEW_HEIGHT,
    )
    .map_err(|error| error.to_string())?;
    region_proposal_geometry(proposal)
}

pub(crate) fn region_proposal_geometry(
    proposal: daena_physical::region::RegionProposal,
) -> Result<AtlasRegionProposal, String> {
    let geometry = polygon_from_micro_rings(&proposal.rings).ok_or_else(|| {
        format!(
            "Could not build {} geometry for region {}.",
            proposal.detector, proposal.region_id
        )
    })?;
    Ok(AtlasRegionProposal {
        detector: proposal.detector,
        region_id: proposal.region_id,
        label: proposal.label,
        epoch_dependent: proposal.epoch_dependent,
        cell_count: proposal.cell_count,
        geometry,
    })
}

/// `"terrain"` when the style uses the new renderer, otherwise `"legacy"`.
#[cfg(test)]
#[must_use]
pub(crate) fn studio_render_kind(style_id: &str) -> &'static str {
    if terrain_style_for(style_id).is_some() {
        "terrain"
    } else {
        "legacy"
    }
}

/// Session zoom cap. Legacy stops at [`STUDIO_MAX_ZOOM`]. Terrain stops at 12.
#[cfg(test)]
#[must_use]
pub(crate) fn studio_zoom_allowed(kind: &str, z: u32) -> bool {
    z <= session_max_zoom(kind)
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
        let first = gate.acquire(false, MAX_CONCURRENT_TILE_RENDERS).unwrap();
        let second = gate.acquire(false, MAX_CONCURRENT_TILE_RENDERS).unwrap();
        assert_eq!(*gate.active.lock().unwrap(), 2);
        assert_eq!(
            gate.acquire(true, MAX_CONCURRENT_TILE_RENDERS)
                .err()
                .unwrap()
                .code,
            CODE_STUDIO_RESOURCE_LIMIT
        );
        drop(first);
        drop(second);
        let prefetch = gate.acquire(true, MAX_CONCURRENT_TILE_RENDERS).unwrap();
        assert_eq!(
            gate.acquire(true, MAX_CONCURRENT_TILE_RENDERS)
                .err()
                .unwrap()
                .code,
            CODE_STUDIO_RESOURCE_LIMIT
        );
        drop(prefetch);
        assert_eq!(*gate.active.lock().unwrap(), 0);
    }

    #[test]
    fn session_registry_caps_and_replaces_per_map() {
        let mut manager = AtlasStudioManager::default();
        let settings = daena_physical::engine::Settings::from_parts(
            831_429,
            daena_physical::engine::WorldPreset::Earthlike,
            128,
            daena_physical::DEFAULT_RADIUS_METRES,
            300_000,
            daena_physical::planetary::PlanetaryConfiguration::earth_like(),
        );
        let world =
            daena_physical::engine::generate(settings, &mut daena_physical::NoopProgress).unwrap();
        let source = world.source().to_vec();
        let identity = daena_atlas::spike_identity_from_source(&source);
        let scene = AtlasStudioSceneRequestV1::spike().normalize().unwrap();
        let request = scene.as_render_request(STUDIO_TILE_SIZE).unwrap();
        let prepared = Arc::new(
            daena_atlas::prepare_from_source(
                &source,
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
            constraint_fp: daena_atlas::constraint::fingerprint(&[]),
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
                    body: SessionBody::Legacy {
                        prepared: prepared.clone(),
                        style: prepared.style.clone(),
                        style_hash: prepared.style_hash.clone(),
                    },
                    physical_identity: String::from_utf8_lossy(&identity).into_owned(),
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
                body: SessionBody::Legacy {
                    prepared: prepared.clone(),
                    style: prepared.style.clone(),
                    style_hash: prepared.style_hash.clone(),
                },
                physical_identity: String::from_utf8_lossy(&identity).into_owned(),
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
        let overlay_token = manager.sessions.keys().next().cloned().unwrap();
        assert!(manager
            .adopt_overlay_snapshot(
                &overlay_token,
                &String::from_utf8_lossy(&identity),
                0,
                Vec::new(),
                2,
            )
            .unwrap());
        assert!(!manager
            .adopt_overlay_snapshot(&overlay_token, "other-world", 0, Vec::new(), 3)
            .unwrap());
        manager.remember_tile(&overlay_token, 3, 1, 2, 0, vec![9, 8, 7]);
        assert_eq!(
            manager
                .tile_from_cache(&overlay_token, 3, 1, 2, 0)
                .as_deref(),
            Some([9_u8, 8, 7].as_slice())
        );
        assert_eq!(
            manager
                .tile_from_cache(&overlay_token, 3, 1, 2, 0)
                .as_deref(),
            Some([9_u8, 8, 7].as_slice())
        );
        let legacy = manager.sessions.values().next().unwrap();
        assert_eq!(legacy.kind_name(), "legacy");
        assert_eq!(legacy.max_zoom(), STUDIO_MAX_ZOOM);
        assert_eq!(
            reject_session_tile(
                legacy,
                &ParsedStudioTile {
                    token: legacy.token.clone(),
                    z: 9,
                    x: 0,
                    y: 0,
                    device_scale: 1,
                    prefetch: false,
                    layer: None,
                },
            )
            .unwrap_err()
            .code,
            CODE_STUDIO_TILE_INVALID
        );
        assert_eq!(
            reject_session_tile(
                legacy,
                &ParsedStudioTile {
                    token: legacy.token.clone(),
                    z: 2,
                    x: 0,
                    y: 1,
                    device_scale: 1,
                    prefetch: false,
                    layer: Some(TerrainVectorLayer::Rivers),
                },
            )
            .unwrap_err()
            .code,
            CODE_STUDIO_TILE_INVALID
        );
        manager.cancel_all();
        assert!(manager.sessions.is_empty());
        assert!(manager.prepared_scenes.is_empty());
        assert!(manager.structures.is_empty());
        assert!(manager.terrains.is_empty());
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
