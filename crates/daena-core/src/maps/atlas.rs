//! Atlas capability reporting and immutable snapshot capture.

use daena_atlas::request::{
    physical_layer_role, AtlasRenderRequest, DetailLevel, ResourceEstimate, ATLAS_LAYER_ROLES,
};
use daena_atlas::studio::{
    AtlasStudioSceneRequestV1, ATLAS_STUDIO_SESSION_SCHEMA_VERSION, CODE_STUDIO_REQUEST_INVALID,
    CODE_STUDIO_UNSUPPORTED, STUDIO_MAX_ZOOM, STUDIO_PROJECTION_ID, STUDIO_SPIKE_LAYER_IDS,
    STUDIO_TILE_SIZE,
};
use daena_atlas::style::{bundled_style_ids, RELIEF_STYLE_ID};
use daena_physical::history::{
    ForcingComponent, HistoricalForcingParameters, FORCING_COMPONENT_COUNT,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::calendar::{
    physical_offset_for_authored_year, year_in_interval, PhysicalCalendarBinding,
    PHYSICAL_CALENDAR_BINDING_KEY,
};
use super::{
    HistoricalForcingSettings, MapDescriptor, MapGenerationSettings, PHYSICAL_ADAPTER_VERSION,
    PHYSICAL_PROVIDER, PHYSICAL_SOURCE_FORMAT,
};
use crate::error::CoreError;
use crate::project::ProjectStore;
use daena_atlas::overlay::AuthoredFeature;
use std::fs;
use std::path::{Path, PathBuf};
use uuid::Uuid;

pub const CODE_PROVIDER_UNSUPPORTED: &str = daena_atlas::CODE_PROVIDER_UNSUPPORTED;
pub const CODE_REQUEST_INVALID: &str = daena_atlas::CODE_REQUEST_INVALID;
pub const CODE_SNAPSHOT_CHANGED: &str = "atlas.snapshot.changed";
pub const ATLAS_PRESETS_KEY: &str = "atlasPresets";
pub const ATLAS_CACHE_RELATIVE: &str = ".daena/cache/atlas";
const MAX_PRESETS: usize = 32;
const MAX_OVERLAY_FEATURES: usize = 2_048;

#[must_use]
pub fn atlas_cache_dir(project_root: &Path) -> PathBuf {
    project_root.join(ATLAS_CACHE_RELATIVE)
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AtlasRenderCapabilities {
    pub supported: bool,
    pub time_modes: Vec<String>,
    pub projections: Vec<String>,
    pub formats: Vec<String>,
    pub styles: Vec<String>,
    pub layers: Vec<AtlasLayerChoice>,
    pub max_width_px: u32,
    pub max_height_px: u32,
    pub max_pixel_count: u64,
    pub supports_authored_layers: bool,
    pub supports_semantic_layers: bool,
    pub supports_studio: bool,
    pub studio_max_zoom: u32,
    pub studio_tile_size: u32,
    pub calendar_binding: Option<PhysicalCalendarBinding>,
    pub presets: Vec<AtlasPresetSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AtlasStudioSessionRequestV1 {
    pub schema_version: u32,
    pub map_entity_id: String,
    pub offset_years: i64,
    pub algorithm_version: u32,
    pub level: DetailLevel,
    pub variant: u32,
    pub style_id: String,
    pub active_layer_ids: Vec<String>,
    pub projection: String,
    pub time_kind: String,
    pub authored_year: Option<i64>,
}

impl AtlasStudioSessionRequestV1 {
    pub fn iteration_1(map_entity_id: impl Into<String>) -> Self {
        Self {
            schema_version: ATLAS_STUDIO_SESSION_SCHEMA_VERSION,
            map_entity_id: map_entity_id.into(),
            offset_years: 0,
            algorithm_version: daena_atlas::ATLAS_DETAIL_ALGORITHM_VERSION,
            level: DetailLevel::Detailed,
            variant: 0,
            style_id: RELIEF_STYLE_ID.to_string(),
            active_layer_ids: STUDIO_SPIKE_LAYER_IDS
                .iter()
                .map(|id| (*id).to_string())
                .collect(),
            projection: STUDIO_PROJECTION_ID.to_string(),
            time_kind: "physical-offset-year".into(),
            authored_year: None,
        }
    }

    pub fn normalize(self) -> Result<Self, CoreError> {
        if self.schema_version != ATLAS_STUDIO_SESSION_SCHEMA_VERSION {
            return Err(studio_invalid("unsupported atlas studio session schema"));
        }
        if self.algorithm_version != daena_atlas::ATLAS_DETAIL_ALGORITHM_VERSION {
            return Err(studio_invalid("unsupported atlas detail algorithm"));
        }
        if self.level != DetailLevel::Detailed {
            return Err(studio_invalid("atlas studio requires detailed level"));
        }
        if self.variant != 0 {
            return Err(studio_invalid("atlas studio requires variant 0"));
        }
        if self.projection != STUDIO_PROJECTION_ID {
            return Err(studio_invalid("atlas studio requires web-mercator"));
        }
        if bundled_style_ids()
            .iter()
            .all(|id| *id != self.style_id.as_str())
            && self.style_id != daena_atlas::SPIKE_STYLE_ID
        {
            return Err(studio_invalid("atlas studio style is not bundled"));
        }
        if Uuid::parse_str(&self.map_entity_id).is_err() {
            return Err(studio_invalid("atlas studio map entity ID must be a UUID"));
        }
        match self.time_kind.as_str() {
            "physical-offset-year" => {
                if self.authored_year.is_some() {
                    return Err(studio_invalid(
                        "physical-offset-year studio sessions cannot include authoredYear",
                    ));
                }
                if !(-daena_physical::history::MAX_HISTORICAL_OFFSET_YEARS
                    ..=daena_physical::history::MAX_HISTORICAL_OFFSET_YEARS)
                    .contains(&self.offset_years)
                {
                    return Err(studio_invalid("atlas studio epoch is out of range"));
                }
            }
            "calendar-year" => {
                if self.authored_year.is_none() {
                    return Err(studio_invalid(
                        "calendar-year studio sessions require authoredYear",
                    ));
                }
            }
            _ => return Err(studio_invalid("unsupported atlas studio time kind")),
        }
        let mut layers = self.active_layer_ids.clone();
        if layers.is_empty() {
            layers = STUDIO_SPIKE_LAYER_IDS
                .iter()
                .map(|id| (*id).to_string())
                .collect();
        }
        layers.sort();
        layers.dedup();
        Ok(Self {
            schema_version: ATLAS_STUDIO_SESSION_SCHEMA_VERSION,
            map_entity_id: self.map_entity_id,
            offset_years: self.offset_years,
            algorithm_version: daena_atlas::ATLAS_DETAIL_ALGORITHM_VERSION,
            level: DetailLevel::Detailed,
            variant: 0,
            style_id: self.style_id,
            active_layer_ids: layers,
            projection: STUDIO_PROJECTION_ID.to_string(),
            time_kind: self.time_kind,
            authored_year: self.authored_year,
        })
    }

    pub fn as_render_request(&self) -> Result<AtlasRenderRequest, CoreError> {
        self.scene_request()
            .as_render_request(STUDIO_TILE_SIZE)
            .and_then(|mut request| {
                request.time_kind = self.time_kind.clone();
                request.authored_year = self.authored_year;
                request.offset_years = self.offset_years;
                request.normalize()
            })
            .map_err(|error| CoreError::Validation(format!("{}: {}", error.code, error.message)))
    }

    #[must_use]
    pub fn scene_request(&self) -> AtlasStudioSceneRequestV1 {
        AtlasStudioSceneRequestV1 {
            schema_version: ATLAS_STUDIO_SESSION_SCHEMA_VERSION,
            offset_years: self.offset_years,
            algorithm_version: self.algorithm_version,
            level: self.level,
            variant: self.variant,
            style_id: self.style_id.clone(),
            active_layer_ids: self
                .active_layer_ids
                .iter()
                .filter(|id| id.as_str() != "frame")
                .cloned()
                .collect(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct AtlasStudioCapture {
    pub session: AtlasStudioSessionRequestV1,
    pub scene: AtlasStudioSceneRequestV1,
    pub snapshot: AtlasRenderSnapshot,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AtlasCacheRegenerateResult {
    pub deleted_entries: u32,
}

fn studio_invalid(message: impl Into<String>) -> CoreError {
    CoreError::Validation(format!(
        "{}: {}",
        CODE_STUDIO_REQUEST_INVALID,
        message.into()
    ))
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AtlasPresetSummary {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AtlasLayerChoice {
    pub id: String,
    pub name: String,
    pub role: String,
    pub default_visible: bool,
}

#[derive(Debug, Clone)]
pub struct AtlasRenderSnapshot {
    pub request: AtlasRenderRequest,
    pub estimate: ResourceEstimate,
    pub identity: String,
    pub source_bytes: Vec<u8>,
    pub source_sha256: String,
    pub forcing: HistoricalForcingParameters,
    pub content_generation: i64,
    pub database_epoch: String,
    pub map_entity_id: String,
    pub overlays: Vec<AuthoredFeature>,
    pub diagnostics: Vec<String>,
    pub binding_revision: Option<String>,
}

pub fn capabilities_for_descriptor(descriptor: &Value, layers: &Value) -> AtlasRenderCapabilities {
    let supported = descriptor
        .get("provider")
        .and_then(|provider| provider.get("id"))
        .and_then(Value::as_str)
        == Some(PHYSICAL_PROVIDER)
        && descriptor
            .get("provider")
            .and_then(|provider| provider.get("sourceFormat"))
            .and_then(Value::as_str)
            == Some(PHYSICAL_SOURCE_FORMAT)
        && descriptor
            .get("provider")
            .and_then(|provider| provider.get("adapterVersion"))
            .and_then(Value::as_u64)
            == Some(u64::from(PHYSICAL_ADAPTER_VERSION));
    let mut choices = ATLAS_LAYER_ROLES
        .iter()
        .map(|role| AtlasLayerChoice {
            id: (*role).to_string(),
            name: role_name(role),
            role: (*role).to_string(),
            default_visible: matches!(*role, "ocean" | "relief" | "ice" | "graticule"),
        })
        .collect::<Vec<_>>();
    if let Some(stored) = layers.get("layers").and_then(Value::as_array) {
        for layer in stored {
            let Some(id) = layer.get("id").and_then(Value::as_str) else {
                continue;
            };
            if let Some(role) = physical_layer_role(id) {
                if let Some(choice) = choices.iter_mut().find(|choice| choice.role == role) {
                    if let Some(name) = layer.get("name").and_then(Value::as_str) {
                        if role == id || (role == "coastlines" && id == "islands") {
                            choice.name = name.to_string();
                        }
                    }
                }
            }
        }
    }
    AtlasRenderCapabilities {
        supported,
        time_modes: if supported {
            vec!["physical-offset-year".into()]
        } else {
            Vec::new()
        },
        projections: if supported {
            vec!["equirectangular".into(), "web-mercator".into()]
        } else {
            Vec::new()
        },
        formats: if supported {
            vec!["png".into(), "svg".into(), "pdf".into()]
        } else {
            Vec::new()
        },
        styles: if supported {
            bundled_style_ids()
                .iter()
                .map(|id| (*id).to_string())
                .collect()
        } else {
            Vec::new()
        },
        layers: if supported { choices } else { Vec::new() },
        max_width_px: daena_atlas::request::PREVIEW_MAX_WIDTH * 4,
        max_height_px: daena_atlas::request::PREVIEW_MAX_HEIGHT * 4,
        max_pixel_count: daena_atlas::request::MAX_PIXEL_COUNT,
        supports_authored_layers: false,
        supports_semantic_layers: false,
        supports_studio: supported,
        studio_max_zoom: if supported { STUDIO_MAX_ZOOM } else { 0 },
        studio_tile_size: if supported { STUDIO_TILE_SIZE } else { 0 },
        calendar_binding: None,
        presets: Vec::new(),
    }
}

fn role_name(role: &str) -> String {
    match role {
        "ocean" => "Ocean".into(),
        "relief" => "Relief".into(),
        "ice" => "Ice".into(),
        "lakes" => "Lakes".into(),
        "rivers" => "Rivers".into(),
        "coastlines" => "Islands".into(),
        "contours" => "Bathymetric contours".into(),
        "graticule" => "Graticule".into(),
        "frame" => "Frame".into(),
        "labels" => "Labels".into(),
        "tectonic-plates" => "Tectonic plates".into(),
        "tectonic-boundaries" => "Plate boundaries".into(),
        "volcanic-centers" => "Volcanic centers".into(),
        "watersheds" => "Watersheds".into(),
        other => other.to_string(),
    }
}

pub fn validate_calendar_binding(value: &Value) -> Result<(), CoreError> {
    let binding: PhysicalCalendarBinding =
        serde_json::from_value(value.clone()).map_err(|error| {
            CoreError::Validation(format!("invalid physical calendar binding: {error}"))
        })?;
    binding.validate()
}

pub fn validate_presets(value: &Value) -> Result<(), CoreError> {
    let object = value
        .as_object()
        .ok_or_else(|| CoreError::Validation("atlasPresets must be an object".into()))?;
    if object
        .keys()
        .any(|key| !matches!(key.as_str(), "schemaVersion" | "presets"))
        || object.get("schemaVersion").and_then(Value::as_u64) != Some(1)
    {
        return Err(CoreError::Validation(
            "atlasPresets must contain schemaVersion 1 and presets only".into(),
        ));
    }
    let presets = object
        .get("presets")
        .and_then(Value::as_array)
        .ok_or_else(|| CoreError::Validation("atlasPresets.presets must be an array".into()))?;
    if presets.len() > MAX_PRESETS {
        return Err(CoreError::Validation(
            "atlas preset count exceeds the budget of 32".into(),
        ));
    }
    let mut ids = std::collections::BTreeSet::new();
    for preset in presets {
        let object = preset
            .as_object()
            .ok_or_else(|| CoreError::Validation("atlas preset must be an object".into()))?;
        for forbidden in ["path", "destination", "jobId", "bytes", "png", "cacheKey"] {
            if object.contains_key(forbidden) {
                return Err(CoreError::Validation(
                    "atlas presets cannot store output paths or generated bytes".into(),
                ));
            }
        }
        let id = object
            .get("id")
            .and_then(Value::as_str)
            .ok_or_else(|| CoreError::Validation("atlas preset id must be a UUID".into()))?;
        Uuid::parse_str(id)
            .map_err(|_| CoreError::Validation("atlas preset id must be a UUID".into()))?;
        if !ids.insert(id.to_string()) {
            return Err(CoreError::Validation(
                "atlas preset IDs must be unique".into(),
            ));
        }
        let name = object.get("name").and_then(Value::as_str).unwrap_or("");
        if name.is_empty() || name.len() > 128 {
            return Err(CoreError::Validation(
                "atlas preset name must be 1..=128 characters".into(),
            ));
        }
        let time = object
            .get("time")
            .and_then(Value::as_object)
            .ok_or_else(|| CoreError::Validation("atlas preset time is required".into()))?;
        match time.get("kind").and_then(Value::as_str) {
            Some("physical-offset-year") => {
                if time.get("offsetYears").and_then(Value::as_i64).is_none() {
                    return Err(CoreError::Validation(
                        "physical-offset-year presets require offsetYears".into(),
                    ));
                }
            }
            Some("calendar-year") => {
                if time.get("authoredYear").and_then(Value::as_i64).is_none() {
                    return Err(CoreError::Validation(
                        "calendar-year presets require authoredYear".into(),
                    ));
                }
            }
            _ => {
                return Err(CoreError::Validation(
                    "unsupported atlas preset time kind".into(),
                ));
            }
        }
        let output = object
            .get("output")
            .and_then(Value::as_object)
            .ok_or_else(|| CoreError::Validation("atlas preset output is required".into()))?;
        match output.get("format").and_then(Value::as_str) {
            Some("png" | "svg" | "pdf") => {}
            _ => {
                return Err(CoreError::Validation(
                    "atlas presets support png, svg, or pdf".into(),
                ));
            }
        }
    }
    Ok(())
}

pub fn capabilities_for_map(
    project: &ProjectStore,
    map_entity_id: &str,
) -> Result<AtlasRenderCapabilities, CoreError> {
    let fields = project.list_fields(map_entity_id.to_string())?;
    let descriptor = field_value(&fields, "map")
        .ok_or_else(|| CoreError::Validation("maps:map descriptor is missing".into()))?;
    let layers = field_value(&fields, "layers")
        .unwrap_or_else(|| serde_json::json!({"schemaVersion": 2, "layers": []}));
    let mut capabilities = capabilities_for_descriptor(&descriptor, &layers);
    if !capabilities.supported {
        return Ok(capabilities);
    }
    capabilities.supports_authored_layers = true;
    capabilities.supports_semantic_layers = true;
    if let Some(binding_value) = field_value(&fields, PHYSICAL_CALENDAR_BINDING_KEY) {
        if let Ok(binding) = serde_json::from_value::<PhysicalCalendarBinding>(binding_value) {
            if binding.validate().is_ok() {
                capabilities.calendar_binding = Some(binding);
                capabilities.time_modes.push("calendar-year".into());
            }
        }
    }
    if let Some(presets_value) = field_value(&fields, ATLAS_PRESETS_KEY) {
        if let Some(presets) = presets_value.get("presets").and_then(Value::as_array) {
            for preset in presets {
                if let (Some(id), Some(name)) = (
                    preset.get("id").and_then(Value::as_str),
                    preset.get("name").and_then(Value::as_str),
                ) {
                    capabilities.presets.push(AtlasPresetSummary {
                        id: id.to_string(),
                        name: name.to_string(),
                    });
                }
            }
        }
    }
    if let Some(stored) = layers.get("layers").and_then(Value::as_array) {
        for layer in stored {
            let Some(id) = layer.get("id").and_then(Value::as_str) else {
                continue;
            };
            if physical_layer_role(id).is_some() {
                continue;
            }
            let kind = layer
                .get("kind")
                .and_then(Value::as_str)
                .unwrap_or("vector");
            capabilities.layers.push(AtlasLayerChoice {
                id: id.to_string(),
                name: layer
                    .get("name")
                    .and_then(Value::as_str)
                    .unwrap_or(id)
                    .to_string(),
                role: kind.to_string(),
                default_visible: layer
                    .get("defaultVisible")
                    .and_then(Value::as_bool)
                    .unwrap_or(true),
            });
        }
    }
    Ok(capabilities)
}

fn field_value(fields: &[crate::FieldValue], key: &str) -> Option<Value> {
    fields
        .iter()
        .find(|field| field.namespace == super::MAP_NAMESPACE && field.key == key)
        .map(|field| field.value.clone())
}

pub fn capture_snapshot(
    project: &ProjectStore,
    map_entity_id: &str,
    mut request: AtlasRenderRequest,
) -> Result<AtlasRenderSnapshot, CoreError> {
    let fields = project.list_fields(map_entity_id.to_string())?;
    let descriptor_value = field_value(&fields, "map")
        .ok_or_else(|| CoreError::Validation("maps:map descriptor is missing".into()))?;
    let layers_value = field_value(&fields, "layers")
        .unwrap_or_else(|| serde_json::json!({"schemaVersion": 2, "layers": []}));
    let capabilities = capabilities_for_descriptor(&descriptor_value, &layers_value);
    if !capabilities.supported {
        return Err(CoreError::Validation(format!(
            "{CODE_PROVIDER_UNSUPPORTED}: map provider does not support atlas rendering"
        )));
    }
    let mut diagnostics = Vec::new();
    let mut binding_revision = None;
    if request.time_kind == "calendar-year" {
        let Some(authored_year) = request.authored_year else {
            return Err(CoreError::Validation(
                "calendar-year renders require authoredYear".into(),
            ));
        };
        let binding_field = fields.iter().find(|field| {
            field.namespace == super::MAP_NAMESPACE && field.key == PHYSICAL_CALENDAR_BINDING_KEY
        });
        let Some(binding_field) = binding_field else {
            return Err(CoreError::Validation(
                "calendar-year renders require physicalCalendarBinding".into(),
            ));
        };
        let binding: PhysicalCalendarBinding = serde_json::from_value(binding_field.value.clone())?;
        request.offset_years = physical_offset_for_authored_year(authored_year, &binding)?;
        binding_revision = Some(binding_field.revision.clone());
        request.binding_revision = binding_revision.clone();
    }
    let request = request
        .normalize()
        .map_err(|error| CoreError::Validation(format!("{}: {}", error.code, error.message)))?;
    let known_ids = capabilities_for_map(project, map_entity_id)?
        .layers
        .into_iter()
        .map(|layer| layer.id)
        .collect::<std::collections::BTreeSet<_>>();
    for id in &request.active_layer_ids {
        if !known_ids.contains(id) {
            diagnostics.push(format!("missing layer {id}"));
        }
    }
    let descriptor: MapDescriptor = serde_json::from_value(descriptor_value.clone())?;
    let source_id = descriptor
        .source_asset_id
        .clone()
        .ok_or_else(|| CoreError::Validation("maps: sourceAssetId is missing".into()))?;
    let generation = descriptor_value
        .get("generation")
        .cloned()
        .ok_or_else(|| CoreError::Validation("maps: physical generation is missing".into()))?;
    let source_bytes = project.asset_bytes(source_id)?;
    let validated = super::physical::validate_source(&source_bytes, &generation)?;
    let forcing = forcing_from_generation(&generation)?;
    let source_sha256 = {
        use sha2::{Digest, Sha256};
        format!("sha256:{:x}", Sha256::digest(&source_bytes))
    };
    let overlays = collect_overlays(
        project,
        &descriptor,
        &layers_value,
        &fields,
        &request,
        &mut diagnostics,
    )?;
    Ok(AtlasRenderSnapshot {
        estimate: request.estimate(),
        request,
        identity: validated.identity,
        source_bytes,
        source_sha256,
        forcing,
        content_generation: project.content_generation()?,
        database_epoch: project.database_epoch().to_string(),
        map_entity_id: map_entity_id.to_string(),
        overlays,
        diagnostics,
        binding_revision,
    })
}

pub fn capture_studio_session(
    project: &ProjectStore,
    request: AtlasStudioSessionRequestV1,
) -> Result<AtlasStudioCapture, CoreError> {
    let capabilities = capabilities_for_map(project, &request.map_entity_id)?;
    if !capabilities.supported || !capabilities.supports_studio {
        return Err(CoreError::Validation(format!(
            "{CODE_STUDIO_UNSUPPORTED}: map provider does not support atlas studio"
        )));
    }
    let session = request.normalize()?;
    let render = session.as_render_request()?;
    let snapshot = capture_snapshot(project, &session.map_entity_id, render)?;
    let mut session = session;
    session.offset_years = snapshot.request.offset_years;
    session.authored_year = snapshot.request.authored_year;
    session.time_kind = snapshot.request.time_kind.clone();
    let scene = session.scene_request();
    Ok(AtlasStudioCapture {
        session,
        scene,
        snapshot,
    })
}

pub fn regenerate_atlas_cache(
    project: &ProjectStore,
) -> Result<AtlasCacheRegenerateResult, CoreError> {
    let root = project.info().ok_or(CoreError::ProjectNotOpen)?.root;
    let cache = atlas_cache_dir(Path::new(&root));
    if !cache.exists() {
        return Ok(AtlasCacheRegenerateResult { deleted_entries: 0 });
    }
    let meta = fs::symlink_metadata(&cache).map_err(|source| CoreError::Io {
        operation: "atlas cache regenerate",
        source,
    })?;
    if meta.file_type().is_symlink() {
        return Err(CoreError::Validation(
            "atlas.studio.request.invalid: atlas cache root must not be a symlink".into(),
        ));
    }
    if !meta.is_dir() {
        return Err(CoreError::Validation(
            "atlas.studio.request.invalid: atlas cache root must be a directory".into(),
        ));
    }
    let expected = atlas_cache_dir(Path::new(&root));
    if cache != expected {
        return Err(CoreError::Validation(
            "atlas.studio.request.invalid: refused to regenerate a foreign cache".into(),
        ));
    }
    let mut deleted = 0_u32;
    for entry in fs::read_dir(&cache).map_err(|source| CoreError::Io {
        operation: "atlas cache regenerate",
        source,
    })? {
        let entry = entry.map_err(|source| CoreError::Io {
            operation: "atlas cache regenerate",
            source,
        })?;
        let path = entry.path();
        let meta = fs::symlink_metadata(&path).map_err(|source| CoreError::Io {
            operation: "atlas cache regenerate",
            source,
        })?;
        if meta.file_type().is_symlink() {
            return Err(CoreError::Validation(
                "atlas.studio.request.invalid: atlas cache refused a symlink".into(),
            ));
        }
        if !meta.is_file() {
            continue;
        }
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if !is_atlas_cache_entry_name(&name) {
            continue;
        }
        fs::remove_file(&path).map_err(|source| CoreError::Io {
            operation: "atlas cache regenerate",
            source,
        })?;
        deleted = deleted.saturating_add(1);
    }
    Ok(AtlasCacheRegenerateResult {
        deleted_entries: deleted,
    })
}

fn is_atlas_cache_entry_name(name: &str) -> bool {
    name == "index.json"
        || name == "index.json.part"
        || name.ends_with(".bin")
        || name.ends_with(".bin.part")
        || name.ends_with(".json.part")
}

fn collect_overlays(
    project: &ProjectStore,
    descriptor: &MapDescriptor,
    layers_value: &Value,
    fields: &[crate::FieldValue],
    request: &AtlasRenderRequest,
    diagnostics: &mut Vec<String>,
) -> Result<Vec<AuthoredFeature>, CoreError> {
    let mut overlays = Vec::new();
    if let Some(authored_id) = descriptor.authored_source_asset_id.as_deref() {
        if let Ok(bytes) = project.asset_bytes(authored_id.to_string()) {
            parse_geojson_overlays(&bytes, request, &mut overlays, diagnostics);
        } else {
            diagnostics.push("authored source asset is missing".into());
        }
    }
    if let Some(locations) = field_value(fields, "locations") {
        collect_semantic_overlays(
            &locations,
            layers_value,
            request,
            &mut overlays,
            diagnostics,
        );
    }
    if overlays.len() > MAX_OVERLAY_FEATURES {
        overlays.truncate(MAX_OVERLAY_FEATURES);
        diagnostics.push("authored overlay feature count exceeded the snapshot budget".into());
    }
    Ok(overlays)
}

fn parse_geojson_overlays(
    bytes: &[u8],
    request: &AtlasRenderRequest,
    overlays: &mut Vec<AuthoredFeature>,
    diagnostics: &mut Vec<String>,
) {
    let Ok(collection) = serde_json::from_slice::<Value>(bytes) else {
        diagnostics.push("authored source is not valid JSON".into());
        return;
    };
    let Some(features) = collection.get("features").and_then(Value::as_array) else {
        return;
    };
    for feature in features {
        let layer_id = feature
            .pointer("/properties/daena/layerId")
            .or_else(|| feature.pointer("/properties/daenaLayerId"))
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();
        if layer_id.is_empty() || !request.layer_enabled(&layer_id) {
            continue;
        }
        let id = feature
            .get("id")
            .and_then(|value| value.as_str().map(ToOwned::to_owned))
            .or_else(|| {
                feature
                    .pointer("/properties/id")
                    .and_then(Value::as_str)
                    .map(ToOwned::to_owned)
            })
            .unwrap_or_else(|| format!("feature-{}", overlays.len()));
        let label = feature
            .pointer("/properties/daena/name")
            .or_else(|| feature.pointer("/properties/name"))
            .and_then(Value::as_str)
            .map(ToOwned::to_owned);
        let mut path = Vec::new();
        collect_coordinates(feature.get("geometry"), &mut path);
        if path.is_empty() {
            continue;
        }
        overlays.push(AuthoredFeature {
            id,
            layer_id,
            kind: "vector".into(),
            label,
            path,
        });
        if overlays.len() >= MAX_OVERLAY_FEATURES {
            break;
        }
    }
}

fn collect_semantic_overlays(
    locations: &Value,
    layers_value: &Value,
    request: &AtlasRenderRequest,
    overlays: &mut Vec<AuthoredFeature>,
    diagnostics: &mut Vec<String>,
) {
    let Some(items) = locations.get("locations").and_then(Value::as_array) else {
        return;
    };
    let semantic_layers = layers_value
        .get("layers")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let has_year_zero = request.time_kind != "calendar-year" || request.authored_year.is_some();
    let authored_year = request.authored_year.unwrap_or(0);
    for location in items {
        let Some(id) = location.get("id").and_then(Value::as_str) else {
            continue;
        };
        let role = location.get("role").and_then(Value::as_str).unwrap_or("");
        let matching_layer = semantic_layers.iter().find(|layer| {
            layer.get("kind").and_then(Value::as_str) == Some("semantic")
                && layer
                    .pointer("/selector/roles")
                    .and_then(Value::as_array)
                    .is_some_and(|roles| roles.iter().any(|value| value.as_str() == Some(role)))
        });
        let Some(layer) = matching_layer else {
            diagnostics.push(format!("unresolved semantic location {id}"));
            continue;
        };
        let layer_id = layer.get("id").and_then(Value::as_str).unwrap_or(id);
        if !request.layer_enabled(layer_id) {
            continue;
        }
        if request.time_kind == "calendar-year" {
            let from = location.pointer("/validity/from");
            let to = location.pointer("/validity/to");
            if !year_in_interval(authored_year, from, to, has_year_zero) {
                diagnostics.push(format!("filtered time content {id}"));
                continue;
            }
        }
        let Some(point) = location.pointer("/anchor/point").and_then(Value::as_array) else {
            diagnostics.push(format!("unresolved anchor {id}"));
            continue;
        };
        if point.len() != 2 {
            continue;
        }
        let Some(x) = point[0].as_f64() else { continue };
        let Some(y) = point[1].as_f64() else { continue };
        let lon = ((x * 360.0) - 180.0) * 1_000_000.0;
        let lat = (90.0 - (y * 180.0)) * 1_000_000.0;
        overlays.push(AuthoredFeature {
            id: id.to_string(),
            layer_id: layer_id.to_string(),
            kind: "semantic".into(),
            label: location
                .get("label")
                .and_then(Value::as_str)
                .map(ToOwned::to_owned),
            path: vec![[lon as i32, lat as i32]],
        });
    }
}

fn collect_coordinates(geometry: Option<&Value>, path: &mut Vec<[i32; 2]>) {
    let Some(geometry) = geometry else {
        return;
    };
    match geometry.get("type").and_then(Value::as_str) {
        Some("Point") => push_position(geometry.get("coordinates"), path),
        Some("LineString") => {
            if let Some(coords) = geometry.get("coordinates").and_then(Value::as_array) {
                for position in coords {
                    push_position(Some(position), path);
                }
            }
        }
        Some("Polygon") => {
            if let Some(rings) = geometry.get("coordinates").and_then(Value::as_array) {
                if let Some(ring) = rings.first().and_then(Value::as_array) {
                    for position in ring {
                        push_position(Some(position), path);
                    }
                }
            }
        }
        Some("MultiLineString" | "MultiPolygon" | "GeometryCollection") => {
            if let Some(geometries) = geometry.get("geometries").and_then(Value::as_array) {
                for child in geometries {
                    collect_coordinates(Some(child), path);
                }
            }
            if let Some(coords) = geometry.get("coordinates").and_then(Value::as_array) {
                for item in coords {
                    if item
                        .as_array()
                        .and_then(|values| values.first())
                        .and_then(Value::as_f64)
                        .is_some()
                    {
                        push_position(Some(item), path);
                    } else if let Some(nested) = item.as_array() {
                        for position in nested {
                            push_position(Some(position), path);
                        }
                    }
                }
            }
        }
        _ => {}
    }
}

fn push_position(value: Option<&Value>, path: &mut Vec<[i32; 2]>) {
    let Some(values) = value.and_then(Value::as_array) else {
        return;
    };
    if values.len() < 2 {
        return;
    }
    let Some(lon) = values[0].as_f64() else {
        return;
    };
    let Some(lat) = values[1].as_f64() else {
        return;
    };
    path.push([(lon * 1_000_000.0) as i32, (lat * 1_000_000.0) as i32]);
}

fn forcing_from_generation(generation: &Value) -> Result<HistoricalForcingParameters, CoreError> {
    let settings = generation
        .get("settings")
        .cloned()
        .ok_or_else(|| CoreError::Validation("physical generation settings are missing".into()))?;
    match serde_json::from_value::<MapGenerationSettings>(settings)? {
        MapGenerationSettings::Physical(physical) => forcing_params(&physical.historical_forcing),
        MapGenerationSettings::Vector(_) => Err(CoreError::Validation(format!(
            "{CODE_PROVIDER_UNSUPPORTED}: map provider does not support atlas rendering"
        ))),
    }
}

fn forcing_params(
    settings: &HistoricalForcingSettings,
) -> Result<HistoricalForcingParameters, CoreError> {
    if settings.components.len() != FORCING_COMPONENT_COUNT {
        return Err(CoreError::Validation(
            "historicalForcing.components must contain three independent terms".into(),
        ));
    }
    Ok(HistoricalForcingParameters {
        version: settings.version,
        components: [
            component(&settings.components[0]),
            component(&settings.components[1]),
            component(&settings.components[2]),
        ],
        sensitivity_ppm: settings.sensitivity_ppm,
        land_ice_amplitude_ppm: settings.land_ice_amplitude_ppm,
        ice_response_years: settings.ice_response_years,
        ice_midpoint_centi_c: settings.ice_midpoint_centi_c,
        ice_transition_width_centi_c: settings.ice_transition_width_centi_c,
        thermal_expansion_ppm_per_degree_c: settings.thermal_expansion_ppm_per_degree_c,
    })
}

fn component(value: &super::HistoricalForcingComponentSettings) -> ForcingComponent {
    ForcingComponent {
        amplitude_centi_c: value.amplitude_centi_c,
        period_years: value.period_years,
        phase_offset_years: value.phase_offset_years,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn atlas_cache_lives_under_disposable_daena_cache() {
        let root = std::path::Path::new("/tmp/example-project");
        assert_eq!(
            atlas_cache_dir(root),
            std::path::Path::new("/tmp/example-project/.daena/cache/atlas")
        );
        assert!(ATLAS_CACHE_RELATIVE.starts_with(".daena/cache/"));
    }

    #[test]
    fn unsupported_providers_are_reported_not_inferred_from_names() {
        let descriptor = serde_json::json!({
            "schemaVersion": 1,
            "provider": {"id": "daena-openlayers", "adapterVersion": 2, "sourceFormat": "daena-geojson"}
        });
        let capabilities =
            capabilities_for_descriptor(&descriptor, &serde_json::json!({"layers": []}));
        assert!(!capabilities.supported);
        assert!(capabilities.formats.is_empty());
        let physical = serde_json::json!({
            "schemaVersion": 1,
            "provider": {
                "id": "daena-physical",
                "adapterVersion": 2,
                "sourceFormat": "physical-world-v2"
            }
        });
        let supported = capabilities_for_descriptor(&physical, &serde_json::json!({"layers": []}));
        assert!(supported.supported);
        assert_eq!(supported.formats, vec!["png", "svg", "pdf"]);
        assert!(supported.styles.contains(&"daena-atlas-relief".into()));
        assert!(supported.styles.contains(&"daena-atlas-political".into()));
        assert!(supported.styles.contains(&"daena-atlas-biome".into()));
        assert!(supported.styles.contains(&"daena-atlas-temperature".into()));
        assert!(supported
            .styles
            .contains(&"daena-atlas-precipitation".into()));
        assert!(supported.styles.contains(&"daena-atlas-bathymetry".into()));
        assert!(supported.styles.contains(&"daena-atlas-hydrology".into()));
        assert!(supported.supports_studio);
        assert_eq!(supported.studio_max_zoom, 8);
        assert_eq!(supported.studio_tile_size, 256);
        assert!(!capabilities.supports_studio);
    }

    #[test]
    fn atlas_presets_reject_output_paths_and_require_png() {
        assert!(validate_presets(&serde_json::json!({
            "schemaVersion": 1,
            "presets": [{
                "id": "00000000-0000-4000-8000-000000000001",
                "name": "Bad",
                "time": {"kind": "physical-offset-year", "offsetYears": 0},
                "output": {"format": "png"},
                "path": "/tmp/out.png"
            }]
        }))
        .is_err());
        assert!(validate_presets(&serde_json::json!({
            "schemaVersion": 1,
            "presets": [{
                "id": "00000000-0000-4000-8000-000000000001",
                "name": "Ok",
                "time": {"kind": "calendar-year", "authoredYear": 42},
                "output": {"format": "png"}
            }]
        }))
        .is_ok());
    }

    #[test]
    fn snapshot_captures_identity_forcing_and_generation_without_mutating() {
        let settings = daena_physical::GenerationSettings {
            width: 16,
            height: 8,
            radius_metres: daena_physical::DEFAULT_RADIUS_METRES,
            target_land_fraction_ppm: 300_000,
        };
        let world =
            daena_physical::generate_world(settings, 831_429, 0, &mut daena_physical::NoopProgress)
                .unwrap();
        let generation = serde_json::json!({
            "id": crate::maps::PHYSICAL_GENERATOR_ID,
            "version": crate::maps::PHYSICAL_GENERATOR_VERSION,
            "seed": 831429,
            "retryIndex": 0,
            "settings": {
                "width": settings.width,
                "height": settings.height,
                "radiusMetres": settings.radius_metres,
                "targetLandFractionPpm": settings.target_land_fraction_ppm,
                "referenceWaterInventoryM3": world.report.reference_water_inventory_m3,
                "plateCount": world.tectonics.settings.plate_count,
                "continentalPlateCount": world.tectonics.settings.continental_plate_count,
                "tectonicActivityPpm": world.tectonics.settings.tectonic_activity_ppm,
                "islandActivityPpm": world.tectonics.settings.island_activity_ppm,
                "evolutionPreset": "mature",
                "hazardDerivationVersion": daena_physical::hazards::HAZARD_DERIVATION_VERSION,
                "historicalForcing": {
                    "version": 2,
                    "components": [
                        { "amplitudeCentiC": 180, "periodYears": 12000, "phaseOffsetYears": 0 },
                        { "amplitudeCentiC": 90, "periodYears": 4100, "phaseOffsetYears": 200 },
                        { "amplitudeCentiC": 40, "periodYears": 2300, "phaseOffsetYears": 800 }
                    ],
                    "sensitivityPpm": 1000000,
                    "landIceAmplitudePpm": 24000,
                    "iceResponseYears": 800,
                    "iceMidpointCentiC": 0,
                    "iceTransitionWidthCentiC": 400,
                    "thermalExpansionPpmPerDegreeC": 210
                }
            }
        });
        let root = std::env::temp_dir().join(format!("daena-atlas-snap-{}", uuid::Uuid::new_v4()));
        let store = ProjectStore::open_directory(&root).unwrap();
        let accepted = store
            .accept_physical_map(
                "Atlas snapshot".into(),
                world.source.clone(),
                generation,
                Some("00000000-0000-4000-8000-0000000000aa"),
            )
            .unwrap();
        let before = store.content_generation().unwrap();
        let snapshot = capture_snapshot(
            &store,
            &accepted.entity.id,
            AtlasRenderRequest::spike_png(64, 32).unwrap(),
        )
        .unwrap();
        assert_eq!(store.content_generation().unwrap(), before);
        assert_eq!(snapshot.content_generation, before);
        assert_eq!(snapshot.identity, accepted.physical_identity);
        assert_eq!(snapshot.forcing.version, 2);
        assert_eq!(snapshot.forcing.land_ice_amplitude_ppm, 24_000);
        store
            .update_entity(accepted.entity.id.clone(), Some("Renamed".into()), None)
            .unwrap();
        let after = store.content_generation().unwrap();
        assert!(after > snapshot.content_generation);
        let studio = capture_studio_session(
            &store,
            AtlasStudioSessionRequestV1::iteration_1(&accepted.entity.id),
        )
        .unwrap();
        assert_eq!(studio.session.offset_years, 0);
        assert_eq!(studio.scene.style_id, "daena-atlas-relief");
        assert_eq!(store.content_generation().unwrap(), after);
        let mut foreign = AtlasStudioSessionRequestV1::iteration_1(&accepted.entity.id);
        foreign.style_id = "daena-atlas-antique".into();
        assert!(foreign.normalize().is_ok());
        foreign = AtlasStudioSessionRequestV1::iteration_1(&accepted.entity.id);
        foreign.offset_years = 42;
        assert_eq!(foreign.normalize().unwrap().offset_years, 42);
        foreign = AtlasStudioSessionRequestV1::iteration_1(&accepted.entity.id);
        foreign.time_kind = "calendar-year".into();
        foreign.authored_year = Some(0);
        assert!(foreign.normalize().is_ok());
        let cache = atlas_cache_dir(&root);
        std::fs::create_dir_all(&cache).unwrap();
        std::fs::write(cache.join("index.json"), b"{}").unwrap();
        std::fs::write(cache.join("aa.bin"), b"payload").unwrap();
        std::fs::write(cache.join("keep-me.txt"), b"no").unwrap();
        let regenerated = regenerate_atlas_cache(&store).unwrap();
        assert_eq!(regenerated.deleted_entries, 2);
        assert!(!cache.join("index.json").exists());
        assert!(cache.join("keep-me.txt").exists());
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn studio_session_rejects_unknown_projection_and_keeps_web_mercator() {
        let mut request =
            AtlasStudioSessionRequestV1::iteration_1("00000000-0000-4000-8000-000000000001");
        request.projection = "equirectangular".into();
        assert!(request.normalize().is_err());
        request = AtlasStudioSessionRequestV1::iteration_1("00000000-0000-4000-8000-000000000001");
        request.style_id = "not-a-style".into();
        assert!(request.normalize().is_err());
    }
}
