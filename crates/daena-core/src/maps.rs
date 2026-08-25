use crate::error::CoreError;
use rusqlite::{Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use uuid::Uuid;

pub const MAP_ENTITY_TYPE: &str = "daena.maps:map";
pub const MAP_NAMESPACE: &str = "maps";
pub const FMG_PROVIDER: &str = "azgaar-fmg";
pub const VECTOR_PROVIDER: &str = "daena-openlayers";
pub const PHYSICAL_PROVIDER: &str = "daena-physical";
pub const PHYSICAL_SOURCE_FORMAT: &str = "physical-world-v2";
pub const PHYSICAL_ADAPTER_VERSION: u32 = 2;
pub const PHYSICAL_MIME: &str = "application/vnd.daena.physical-world";
pub const PHYSICAL_FILENAME: &str = "world.pworld";
pub const PHYSICAL_GENERATOR_ID: &str = "daena-physical-world";
pub const PHYSICAL_GENERATOR_VERSION: u32 = 13;
pub const PHYSICAL_MAX_SOURCE_BYTES: usize = daena_physical::MAX_SOURCE_BYTES;
pub const DETAIL_MAP_RELATIONSHIP: &str = "daena.maps:detail-map";
pub const OVERVIEW_MAP_RELATIONSHIP: &str = "daena.maps:overview-map";
pub const RELATED_MAP_RELATIONSHIP: &str = "daena.maps:related-map";
/// Stable built-in contract for explicitly materialized physical events.
pub const PHYSICAL_EVENT_ENTITY_TYPE: &str = "daena.maps:physical-natural-event";
pub const PHYSICAL_EVENT_NAMESPACE: &str = "maps.physical";
pub const PHYSICAL_EVENT_CHRONOLOGY_KEY: &str = "physicalChronology";
pub const PHYSICAL_EVENT_MAX_OFFSET_YEARS: i64 = 100_000;
pub const PHYSICAL_EVENT_ON_MAP_RELATIONSHIP: &str = "daena.maps:physical-event-on-map";

pub mod atlas;
pub mod calendar;
pub mod image;
pub mod physical;
pub mod vector;
pub use image::{
    encode_transparent_png, mime_for_source_format, source_format_for_mime, validate_image_source,
    validate_raster_png, ImageSource, IMAGE_MAX_DECODED_BYTES, IMAGE_MAX_ENCODED_BYTES,
    IMAGE_MAX_PIXELS, IMAGE_MAX_RASTER_LAYERS, IMAGE_MAX_UNDO_BYTES,
};
pub use vector::{
    empty_canonical_bytes, VECTOR_CENTER_Y_MAX, VECTOR_CENTER_Y_MIN, VECTOR_FILENAME,
    VECTOR_MAX_BYTES, VECTOR_MAX_FEATURES, VECTOR_MAX_FEATURE_POSITIONS, VECTOR_MAX_LAYERS,
    VECTOR_MAX_POSITIONS, VECTOR_MAX_PROPERTY_BYTES, VECTOR_MIME, VECTOR_SOURCE_FORMAT,
};

/// Upper bound on vertices in a path or one area ring.
pub const IMAGE_MAX_PATH_POINTS: usize = 256;
/// Upper bound on rings in an area geometry.
pub const IMAGE_MAX_AREA_RINGS: usize = 8;
/// Upper bound on semantic overlay layers on one map.
pub const IMAGE_MAX_SEMANTIC_LAYERS: usize = 32;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Point(pub f64, pub f64);

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind", deny_unknown_fields)]
pub enum Anchor {
    #[serde(rename = "point")]
    Point { point: Point },
    #[serde(rename = "provider-feature")]
    ProviderFeature {
        provider: String,
        #[serde(rename = "featureKind")]
        feature_kind: String,
        #[serde(rename = "featureId")]
        feature_id: String,
        #[serde(rename = "fallbackPoint")]
        fallback_point: Point,
    },
    #[serde(rename = "path")]
    Path { points: Vec<Point> },
    #[serde(rename = "area")]
    Area { rings: Vec<Vec<Point>> },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Validity {
    pub from: Option<Value>,
    pub to: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct LocationReference {
    pub id: String,
    #[serde(rename = "mapEntityId")]
    pub map_entity_id: String,
    pub role: String,
    pub label: String,
    pub anchor: Anchor,
    pub validity: Validity,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ProviderDescriptor {
    pub id: String,
    #[serde(rename = "adapterVersion")]
    pub adapter_version: u32,
    #[serde(rename = "sourceFormat")]
    pub source_format: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct DefaultView {
    pub center: Point,
    pub zoom: f64,
    #[serde(default)]
    pub rotation: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind", deny_unknown_fields)]
pub enum MapCoordinateSpace {
    #[serde(rename = "image")]
    Image {
        extent: [f64; 4],
        origin: String,
        units: String,
    },
    #[serde(rename = "world")]
    World {
        extent: [f64; 4],
        origin: String,
        units: MapWorldUnits,
        #[serde(rename = "wrapX")]
        wrap_x: bool,
    },
    #[serde(rename = "geographic")]
    Geographic {
        projection: String,
        extent: [f64; 4],
        #[serde(rename = "wrapX")]
        wrap_x: bool,
    },
}

impl MapCoordinateSpace {
    fn extent(&self) -> &[f64; 4] {
        match self {
            Self::Image { extent, .. } | Self::World { extent, .. } | Self::Geographic { extent, .. } => extent,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct MapWorldUnits {
    pub id: String,
    pub label: String,
    #[serde(rename = "metresPerUnit")]
    pub metres_per_unit: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct MapBackgroundRef {
    pub id: String,
    #[serde(rename = "assetId")]
    pub asset_id: String,
    pub name: String,
    pub visible: bool,
    pub locked: bool,
    pub opacity: f64,
    pub order: i64,
    pub extent: [f64; 4],
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct MapGridSettings {
    pub visible: bool,
    pub snap: bool,
    pub spacing: [f64; 2],
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct MapSettings {
    #[serde(rename = "snapEnabled")]
    pub snap_enabled: bool,
    pub grid: Option<MapGridSettings>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct MapRecoveryCopy {
    #[serde(rename = "fileName")]
    pub file_name: String,
    pub path: String,
    #[serde(rename = "createdAt")]
    pub created_at: String,
}

pub const MAP_EDIT_DRAFT_KIND: &str = "daena-map-edit-draft";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct MapEditDraftPackage {
    #[serde(rename = "schemaVersion")]
    pub schema_version: u32,
    pub kind: String,
    #[serde(rename = "mapEntityId")]
    pub map_entity_id: String,
    pub descriptor: Value,
    pub layers: Value,
    pub geojson: String,
    #[serde(rename = "linkMutations", default)]
    pub link_mutations: Vec<Value>,
}

impl MapEditDraftPackage {
    pub fn encode(&self) -> Result<Vec<u8>, CoreError> {
        if self.schema_version != 1 || self.kind != MAP_EDIT_DRAFT_KIND {
            return Err(invalid("unsupported map edit draft package"));
        }
        let bytes = serde_json::to_vec(self).map_err(|error| invalid(error.to_string()))?;
        if bytes.len() > MAP_RECOVERY_MAX_BYTES {
            return Err(invalid("map recovery copy exceeds the host transfer limit"));
        }
        Ok(bytes)
    }

    pub fn parse(bytes: &[u8]) -> Result<Self, CoreError> {
        if bytes.len() > MAP_RECOVERY_MAX_BYTES {
            return Err(invalid("map recovery copy exceeds the host transfer limit"));
        }
        let package: Self =
            serde_json::from_slice(bytes).map_err(|error| invalid(format!("invalid map edit draft: {error}")))?;
        if package.schema_version != 1 || package.kind != MAP_EDIT_DRAFT_KIND {
            return Err(invalid("unsupported map edit draft package"));
        }
        if package.map_entity_id.trim().is_empty() {
            return Err(invalid("map edit draft mapEntityId is required"));
        }
        if package.geojson.len() > VECTOR_MAX_BYTES {
            return Err(invalid("map edit draft geojson exceeds the vector byte budget"));
        }
        Ok(package)
    }
}

/// Upper bound for a rejected map draft written to the recovery directory.
/// Mirrors the host transfer ceiling; larger payloads cannot arrive here.
pub const MAP_RECOVERY_MAX_BYTES: usize = 64 * 1024 * 1024;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct VectorMapGenerationSettings {
    #[serde(rename = "landPercent")]
    pub land_percent: u32,
    #[serde(rename = "continentCount")]
    pub continent_count: u32,
    #[serde(rename = "coastlineRoughness")]
    pub coastline_roughness: String,
    #[serde(rename = "islandFrequency")]
    pub island_frequency: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct HistoricalForcingComponentSettings {
    #[serde(rename = "amplitudeCentiC")]
    pub amplitude_centi_c: i32,
    #[serde(rename = "periodYears")]
    pub period_years: i64,
    #[serde(rename = "phaseOffsetYears")]
    pub phase_offset_years: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct HistoricalForcingSettings {
    pub version: u16,
    pub components: Vec<HistoricalForcingComponentSettings>,
    #[serde(rename = "sensitivityPpm")]
    pub sensitivity_ppm: u32,
    #[serde(rename = "landIceAmplitudePpm")]
    pub land_ice_amplitude_ppm: u32,
    #[serde(rename = "iceResponseYears")]
    pub ice_response_years: i64,
    #[serde(rename = "iceMidpointCentiC")]
    pub ice_midpoint_centi_c: i32,
    #[serde(rename = "iceTransitionWidthCentiC")]
    pub ice_transition_width_centi_c: i32,
    #[serde(rename = "thermalExpansionPpmPerDegreeC")]
    pub thermal_expansion_ppm_per_degree_c: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct PhysicalMapGenerationSettings {
    pub width: u32,
    pub height: u32,
    #[serde(rename = "radiusMetres")]
    pub radius_metres: u64,
    #[serde(rename = "targetLandFractionPpm")]
    pub target_land_fraction_ppm: u32,
    #[serde(rename = "referenceWaterInventoryM3")]
    pub reference_water_inventory_m3: u64,
    #[serde(rename = "plateCount")]
    pub plate_count: u16,
    #[serde(rename = "continentalPlateCount")]
    pub continental_plate_count: u16,
    #[serde(rename = "tectonicActivityPpm")]
    pub tectonic_activity_ppm: u32,
    #[serde(rename = "islandActivityPpm")]
    pub island_activity_ppm: u32,
    #[serde(rename = "evolutionPreset")]
    pub evolution_preset: String,
    #[serde(rename = "historicalForcing")]
    pub historical_forcing: HistoricalForcingSettings,
    #[serde(
        rename = "hazardDerivationVersion",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub hazard_derivation_version: Option<u16>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum MapGenerationSettings {
    Vector(VectorMapGenerationSettings),
    Physical(PhysicalMapGenerationSettings),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct MapGeneration {
    pub id: String,
    pub version: u32,
    pub seed: u32,
    pub settings: MapGenerationSettings,
    #[serde(
        rename = "retryIndex",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub retry_index: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct MapDescriptor {
    #[serde(rename = "schemaVersion")]
    pub schema_version: u32,
    pub provider: ProviderDescriptor,
    #[serde(rename = "sourceAssetId")]
    pub source_asset_id: Option<String>,
    #[serde(
        rename = "authoredSourceAssetId",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub authored_source_asset_id: Option<String>,
    #[serde(rename = "previewAssetId")]
    pub preview_asset_id: Option<String>,
    #[serde(rename = "defaultView")]
    pub default_view: DefaultView,
    #[serde(rename = "coordinateSpace", default, skip_serializing_if = "Option::is_none")]
    pub coordinate_space: Option<MapCoordinateSpace>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub backgrounds: Vec<MapBackgroundRef>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub settings: Option<MapSettings>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub generation: Option<MapGeneration>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum RasterLayerKind {
    #[serde(rename = "raster")]
    Raster,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum VectorLayerKind {
    #[serde(rename = "vector")]
    Vector,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SemanticLayerKind {
    #[serde(rename = "semantic")]
    Semantic,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct SemanticLayerDefinition {
    pub id: String,
    pub name: String,
    pub order: i64,
    #[serde(rename = "defaultVisible")]
    pub default_visible: bool,
    pub style: Value,
    pub selector: Value,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kind: Option<SemanticLayerKind>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct RasterLayerDefinition {
    pub id: String,
    pub name: String,
    pub order: i64,
    #[serde(rename = "defaultVisible")]
    pub default_visible: bool,
    pub style: Value,
    pub selector: Value,
    pub kind: RasterLayerKind,
    #[serde(rename = "rasterAssetId")]
    pub raster_asset_id: String,
    pub opacity: f64,
    pub locked: bool,
    #[serde(rename = "blendMode", default = "default_blend_mode")]
    pub blend_mode: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct VectorLayerDefinition {
    pub id: String,
    pub name: String,
    pub order: i64,
    #[serde(rename = "defaultVisible")]
    pub default_visible: bool,
    pub locked: bool,
    #[serde(default = "default_opacity")]
    pub opacity: f64,
    #[serde(rename = "blendMode", default = "default_blend_mode")]
    pub blend_mode: String,
    pub selector: Value,
    pub style: Value,
    pub kind: VectorLayerKind,
}

fn default_opacity() -> f64 {
    1.0
}

fn default_blend_mode() -> String {
    "normal".into()
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum LayerDefinition {
    Raster(RasterLayerDefinition),
    Vector(VectorLayerDefinition),
    Semantic(SemanticLayerDefinition),
}

impl LayerDefinition {
    fn id(&self) -> &str {
        match self {
            Self::Raster(layer) => layer.id.as_str(),
            Self::Vector(layer) => layer.id.as_str(),
            Self::Semantic(layer) => layer.id.as_str(),
        }
    }

    fn name(&self) -> &str {
        match self {
            Self::Raster(layer) => layer.name.as_str(),
            Self::Vector(layer) => layer.name.as_str(),
            Self::Semantic(layer) => layer.name.as_str(),
        }
    }

    fn style(&self) -> &Value {
        match self {
            Self::Raster(layer) => &layer.style,
            Self::Vector(layer) => &layer.style,
            Self::Semantic(layer) => &layer.style,
        }
    }

    fn selector(&self) -> &Value {
        match self {
            Self::Raster(layer) => &layer.selector,
            Self::Vector(layer) => &layer.selector,
            Self::Semantic(layer) => &layer.selector,
        }
    }

    fn sort_key(&self) -> (i64, &str) {
        match self {
            Self::Raster(layer) => (layer.order, layer.id.as_str()),
            Self::Vector(layer) => (layer.order, layer.id.as_str()),
            Self::Semantic(layer) => (layer.order, layer.id.as_str()),
        }
    }
}

fn validate_semantic_style(style: &Value) -> Result<(), CoreError> {
    let object = style
        .as_object()
        .ok_or_else(|| invalid("layer definition is invalid"))?;
    for (key, value) in object {
        match key.as_str() {
            "color" | "stroke" | "fill" => {
                let color = value
                    .as_str()
                    .ok_or_else(|| invalid("semantic style color values must be strings"))?;
                if color.is_empty() || color.len() > 64 {
                    return Err(invalid("semantic style color has invalid length"));
                }
            }
            "strokeWidth" => {
                let width = value
                    .as_f64()
                    .ok_or_else(|| invalid("semantic style.strokeWidth must be a finite number"))?;
                if !width.is_finite() || !(0.0..=32.0).contains(&width) {
                    return Err(invalid(
                        "semantic style.strokeWidth must be finite in [0, 32]",
                    ));
                }
            }
            "opacity" => {
                let opacity = value
                    .as_f64()
                    .ok_or_else(|| invalid("semantic style.opacity must be a finite number"))?;
                if !opacity.is_finite() || !(0.0..=1.0).contains(&opacity) {
                    return Err(invalid("semantic style.opacity must be finite in [0, 1]"));
                }
            }
            _ => {
                return Err(invalid("semantic style contains an unsupported property"));
            }
        }
    }
    Ok(())
}

fn validate_semantic_selector(selector: &Value) -> Result<(), CoreError> {
    let object = selector
        .as_object()
        .ok_or_else(|| invalid("layer definition is invalid"))?;
    for (key, value) in object {
        match key.as_str() {
            "roles" => {
                let roles = value
                    .as_array()
                    .ok_or_else(|| invalid("semantic selector.roles must be an array"))?;
                if roles.len() > 32 {
                    return Err(invalid("semantic selector.roles exceeds 32 entries"));
                }
                for role in roles {
                    let role = role
                        .as_str()
                        .ok_or_else(|| invalid("semantic selector.roles must be strings"))?;
                    if role.trim().is_empty() || role.len() > 128 {
                        return Err(invalid("semantic selector role has invalid length"));
                    }
                }
            }
            "anchorKind" => {
                let kind = value
                    .as_str()
                    .ok_or_else(|| invalid("semantic selector.anchorKind must be a string"))?;
                if !matches!(kind, "point" | "path" | "area") {
                    return Err(invalid(
                        "semantic selector.anchorKind must be point, path, or area",
                    ));
                }
            }
            _ => {
                return Err(invalid(
                    "semantic selector contains an unsupported property",
                ));
            }
        }
    }
    Ok(())
}

fn invalid(message: impl Into<String>) -> CoreError {
    CoreError::Validation(format!("maps: {}", message.into()))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ProviderKind {
    Fmg,
    Vector,
    Physical,
}

type GenerationValidator = fn(&Value) -> Result<(), CoreError>;
type PreviewMimeValidator = fn(&str) -> Result<&'static str, CoreError>;

#[derive(Clone, Copy)]
struct ProviderSpec {
    kind: ProviderKind,
    id: &'static str,
    adapter_version: u32,
    source_format: &'static str,
    requires_source_asset: bool,
    source_mime: Option<&'static str>,
    generation_validator: Option<GenerationValidator>,
    preview_mime_validator: Option<PreviewMimeValidator>,
    validate_center: fn(&Point) -> Result<(), CoreError>,
}

fn validate_vector_generation(value: &Value) -> Result<(), CoreError> {
    vector::validate_generation(value)
}

fn validate_physical_generation(value: &Value) -> Result<(), CoreError> {
    physical::validate_generation(value).map(|_| ())
}

fn validate_normalized_center(center: &Point) -> Result<(), CoreError> {
    point(center)
}

fn validate_authored_center(center: &Point) -> Result<(), CoreError> {
    if !center.0.is_finite() || !center.1.is_finite() {
        return Err(invalid("defaultView.center must contain finite numbers"));
    }
    Ok(())
}

const PROVIDER_REGISTRY: &[ProviderSpec] = &[
    ProviderSpec {
        kind: ProviderKind::Fmg,
        id: FMG_PROVIDER,
        adapter_version: 1,
        source_format: "fmg-map",
        requires_source_asset: false,
        source_mime: None,
        generation_validator: None,
        preview_mime_validator: None,
        validate_center: validate_normalized_center,
    },
    ProviderSpec {
        kind: ProviderKind::Vector,
        id: VECTOR_PROVIDER,
        adapter_version: 2,
        source_format: VECTOR_SOURCE_FORMAT,
        requires_source_asset: true,
        source_mime: Some(VECTOR_MIME),
        generation_validator: Some(validate_vector_generation),
        preview_mime_validator: Some(source_format_for_mime),
        validate_center: validate_authored_center,
    },
    ProviderSpec {
        kind: ProviderKind::Physical,
        id: PHYSICAL_PROVIDER,
        adapter_version: PHYSICAL_ADAPTER_VERSION,
        source_format: PHYSICAL_SOURCE_FORMAT,
        requires_source_asset: true,
        source_mime: Some(PHYSICAL_MIME),
        generation_validator: Some(validate_physical_generation),
        preview_mime_validator: None,
        validate_center: validate_normalized_center,
    },
];

fn provider_spec(provider: &ProviderDescriptor) -> Result<&'static ProviderSpec, CoreError> {
    PROVIDER_REGISTRY
        .iter()
        .find(|spec| {
            spec.id == provider.id
                && spec.adapter_version == provider.adapter_version
                && spec.source_format == provider.source_format
        })
        .ok_or_else(|| invalid("unsupported map provider or descriptor version"))
}

fn map_asset(
    connection: &Connection,
    asset_id: &str,
) -> Result<Option<(String, String)>, CoreError> {
    connection
        .query_row(
            "SELECT entity_id, mime_type FROM assets WHERE id=?1 AND namespace=?2",
            [asset_id, MAP_NAMESPACE],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()
        .map_err(CoreError::from)
}

fn owned_map_asset(
    connection: &Connection,
    entity_id: &str,
    asset_id: &str,
    label: &str,
) -> Result<(String, String), CoreError> {
    uuid(asset_id, label)?;
    let Some((owner, mime_type)) = map_asset(connection, asset_id)? else {
        return Err(invalid(format!(
            "{label} must name an asset owned by the map entity in daena.maps"
        )));
    };
    if owner != entity_id {
        return Err(invalid(format!(
            "{label} must name an asset owned by the map entity in daena.maps"
        )));
    }
    Ok((owner, mime_type))
}

fn uuid(value: &str, label: &str) -> Result<(), CoreError> {
    Uuid::parse_str(value)
        .map(|_| ())
        .map_err(|_| invalid(format!("{label} must be a UUID")))
}

fn point(point: &Point) -> Result<(), CoreError> {
    if !point.0.is_finite()
        || !point.1.is_finite()
        || !(0.0..=1.0).contains(&point.0)
        || !(0.0..=1.0).contains(&point.1)
    {
        return Err(invalid(
            "invalid geometry: coordinates must be finite normalized values in [0, 1]",
        ));
    }
    Ok(())
}

fn validate_extent(extent: &[f64; 4], label: &str) -> Result<(), CoreError> {
    if extent.iter().any(|value| !value.is_finite())
        || extent[0] >= extent[2]
        || extent[1] >= extent[3]
    {
        return Err(invalid(format!(
            "{label} must be four finite values with positive width and height"
        )));
    }
    Ok(())
}

fn validate_coordinate_space(space: &MapCoordinateSpace) -> Result<(), CoreError> {
    validate_extent(space.extent(), "coordinateSpace.extent")?;
    match space {
        MapCoordinateSpace::Image { origin, units, .. } => {
            if origin != "top-left" || units != "pixels" {
                return Err(invalid(
                    "image coordinate spaces require top-left origin and pixel units",
                ));
            }
        }
        MapCoordinateSpace::World { origin, units, .. } => {
            if origin != "bottom-left"
                || units.id.trim().is_empty()
                || units.id.len() > 64
                || units.label.trim().is_empty()
                || units.label.len() > 64
                || units
                    .metres_per_unit
                    .is_some_and(|value| !value.is_finite() || value <= 0.0)
            {
                return Err(invalid("world coordinate-space units are invalid"));
            }
        }
        MapCoordinateSpace::Geographic {
            projection, extent, ..
        } => {
            if projection != "EPSG:4326"
                || extent[0] < -180.0
                || extent[2] > 180.0
                || extent[1] < -90.0
                || extent[3] > 90.0
            {
                return Err(invalid(
                    "geographic coordinate spaces require EPSG:4326 within longitude/latitude bounds",
                ));
            }
        }
    }
    Ok(())
}

fn date(value: &Value, label: &str) -> Result<(), CoreError> {
    let object = value
        .as_object()
        .ok_or_else(|| invalid(format!("{label} must be a date object")))?;
    let precision = object
        .get("precision")
        .and_then(Value::as_str)
        .ok_or_else(|| invalid(format!("{label}.precision is required")))?;
    if object.get("calendar").and_then(Value::as_str) != Some("gregorian")
        || !matches!(
            object.get("era").and_then(Value::as_str),
            Some("BCE" | "CE")
        )
        || object.get("year").and_then(Value::as_i64).is_none()
        || !matches!(
            precision,
            "year" | "month" | "day" | "hour" | "minute" | "second"
        )
    {
        return Err(invalid(format!("{label} is not a valid Daena date")));
    }
    let month = object.get("month").and_then(Value::as_i64);
    let day = object.get("day").and_then(Value::as_i64);
    let hour = object.get("hour").and_then(Value::as_i64);
    let minute = object.get("minute").and_then(Value::as_i64);
    let second = object.get("second").and_then(Value::as_i64);
    if matches!(precision, "month" | "day") && !matches!(month, Some(1..=12))
        || matches!(precision, "day" | "hour" | "minute" | "second") && !matches!(day, Some(1..=31))
        || matches!(precision, "hour" | "minute" | "second") && !matches!(hour, Some(0..=23))
        || matches!(precision, "minute" | "second") && !matches!(minute, Some(0..=59))
        || precision == "second" && !matches!(second, Some(0..=59))
        || precision == "year"
            && (month.is_some()
                || day.is_some()
                || hour.is_some()
                || minute.is_some()
                || second.is_some())
        || precision == "month"
            && (day.is_some() || hour.is_some() || minute.is_some() || second.is_some())
        || precision == "day" && (hour.is_some() || minute.is_some() || second.is_some())
        || precision == "hour" && (minute.is_some() || second.is_some())
        || precision == "minute" && second.is_some()
    {
        return Err(invalid(format!("{label} has inconsistent precision")));
    }
    Ok(())
}

fn date_lower_bound(value: &Value) -> (i64, i64, i64) {
    let era = value.get("era").and_then(Value::as_str).unwrap_or("CE");
    let year = value.get("year").and_then(Value::as_i64).unwrap_or(0);
    let signed_year = if era == "BCE" { -year } else { year };
    let month = value.get("month").and_then(Value::as_i64).unwrap_or(1);
    let day = value.get("day").and_then(Value::as_i64).unwrap_or(1);
    (signed_year, month, day)
}

fn date_upper_bound(value: &Value) -> (i64, i64, i64) {
    let era = value.get("era").and_then(Value::as_str).unwrap_or("CE");
    let year = value.get("year").and_then(Value::as_i64).unwrap_or(0);
    let signed_year = if era == "BCE" { -year } else { year };
    let month = value.get("month").and_then(Value::as_i64).unwrap_or(12);
    let day = value.get("day").and_then(Value::as_i64).unwrap_or(31);
    (signed_year, month, day)
}

fn validity(value: &Value, label: &str) -> Result<(), CoreError> {
    let object = value
        .as_object()
        .ok_or_else(|| invalid(format!("{label} must be an object")))?;
    if object.len() != 2 || !object.contains_key("from") || !object.contains_key("to") {
        return Err(invalid(format!("{label} must contain only from and to")));
    }
    let from_val = object.get("from").filter(|v| !v.is_null());
    let to_val = object.get("to").filter(|v| !v.is_null());
    if let Some(from) = from_val {
        date(from, &format!("{label}.from"))?;
    }
    if let Some(to) = to_val {
        date(to, &format!("{label}.to"))?;
    }
    if let (Some(from), Some(to)) = (from_val, to_val) {
        if date_lower_bound(from) > date_upper_bound(to) {
            return Err(invalid(format!("{label}.from cannot be after {label}.to")));
        }
    }
    Ok(())
}

fn anchor(value: &Value) -> Result<Anchor, CoreError> {
    let anchor: Anchor = serde_json::from_value(value.clone())
        .map_err(|e| invalid(format!("invalid anchor: {e}")))?;
    match &anchor {
        Anchor::Point { point: p } => point(p)?,
        Anchor::ProviderFeature {
            provider,
            feature_kind,
            feature_id,
            fallback_point,
        } => {
            let supported = match provider.as_str() {
                FMG_PROVIDER => {
                    matches!(
                        feature_kind.as_str(),
                        "burg" | "state" | "province" | "river" | "marker"
                    ) && !feature_id.is_empty()
                }
                VECTOR_PROVIDER => {
                    feature_kind == "geojson-feature"
                        && Uuid::parse_str(feature_id)
                            .is_ok_and(|uuid| uuid.to_string() == *feature_id)
                }
                _ => false,
            };
            if !supported {
                return Err(invalid("unsupported provider feature selector"));
            }
            point(fallback_point)?;
        }
        Anchor::Path { points } => {
            if points.len() < 2 {
                return Err(invalid(
                    "invalid geometry: path requires at least two points",
                ));
            }
            if points.len() > IMAGE_MAX_PATH_POINTS {
                return Err(invalid(format!(
                    "invalid geometry: path exceeds the budget of {IMAGE_MAX_PATH_POINTS} points"
                )));
            }
            for p in points {
                point(p)?;
            }
        }
        Anchor::Area { rings } => {
            if rings.is_empty() {
                return Err(invalid("invalid geometry: area requires at least one ring"));
            }
            if rings.len() > IMAGE_MAX_AREA_RINGS {
                return Err(invalid(format!(
                    "invalid geometry: area exceeds the budget of {IMAGE_MAX_AREA_RINGS} rings"
                )));
            }
            for ring in rings {
                if ring.len() < 4 || ring.first() != ring.last() {
                    return Err(invalid(
                        "invalid geometry: area rings must be closed and contain at least four points",
                    ));
                }
                if ring.len() > IMAGE_MAX_PATH_POINTS {
                    return Err(invalid(format!(
                        "invalid geometry: area ring exceeds the budget of {IMAGE_MAX_PATH_POINTS} points"
                    )));
                }
                for p in ring {
                    point(p)?;
                }
            }
        }
    }
    Ok(anchor)
}

pub fn validate_field(
    connection: &Connection,
    entity_id: &str,
    key: &str,
    value: &Value,
) -> Result<(), CoreError> {
    let entity_type: Option<String> = connection
        .query_row(
            "SELECT entity_type FROM entities WHERE id=?1 AND deleted=0",
            [entity_id],
            |row| row.get(0),
        )
        .map_err(CoreError::from)?;
    if key == "map" {
        if entity_type.as_deref() != Some(MAP_ENTITY_TYPE) {
            return Err(invalid("map descriptor belongs only on a map entity"));
        }
        let descriptor: MapDescriptor = serde_json::from_value(value.clone())
            .map_err(|e| invalid(format!("invalid map descriptor: {e}")))?;
        let provider = provider_spec(&descriptor.provider)?;
        let expected_schema = if provider.kind == ProviderKind::Vector { 2 } else { 1 };
        if descriptor.schema_version != expected_schema {
            return Err(invalid("unsupported map provider or descriptor version"));
        }
        if provider.requires_source_asset && descriptor.source_asset_id.is_none() {
            return Err(invalid(format!(
                "{} maps require sourceAssetId",
                provider.id
            )));
        }
        if let Some(generation) = &descriptor.generation {
            let value = serde_json::to_value(generation).map_err(|e| invalid(e.to_string()))?;
            let validate_generation = provider
                .generation_validator
                .ok_or_else(|| invalid("generation is only valid on generated map providers"))?;
            validate_generation(&value)?;
        }
        if let Some(source_asset_id) = &descriptor.source_asset_id {
            let (_, mime_type) =
                owned_map_asset(connection, entity_id, source_asset_id, "sourceAssetId")?;
            if let Some(expected_mime) = provider.source_mime {
                if mime_type != expected_mime {
                    return Err(invalid(format!(
                        "sourceAssetId MIME type must be {expected_mime}"
                    )));
                }
            }
        }
        if provider.kind == ProviderKind::Physical {
            let authored_source_asset_id = descriptor
                .authored_source_asset_id
                .as_deref()
                .ok_or_else(|| invalid("daena-physical maps require authoredSourceAssetId"))?;
            let (_, mime_type) = owned_map_asset(
                connection,
                entity_id,
                authored_source_asset_id,
                "authoredSourceAssetId",
            )?;
            if mime_type != VECTOR_MIME {
                return Err(invalid(format!(
                    "authoredSourceAssetId MIME type must be {VECTOR_MIME}"
                )));
            }
        }
        if let Some(preview) = &descriptor.preview_asset_id {
            let (_, mime_type) = owned_map_asset(connection, entity_id, preview, "previewAssetId")?;
            if let Some(validate_preview_mime) = provider.preview_mime_validator {
                validate_preview_mime(&mime_type)?;
            }
        }
        if descriptor.default_view.zoom <= 0.0 || !descriptor.default_view.zoom.is_finite() {
            return Err(invalid("defaultView.zoom must be finite and positive"));
        }
        if !descriptor.default_view.rotation.is_finite() {
            return Err(invalid("defaultView.rotation must be finite"));
        }
        (provider.validate_center)(&descriptor.default_view.center)?;
        if provider.kind == ProviderKind::Vector {
            let coordinate_space = descriptor
                .coordinate_space
                .as_ref()
                .ok_or_else(|| invalid("daena-openlayers maps require coordinateSpace"))?;
            validate_coordinate_space(coordinate_space)?;
            let extent = coordinate_space.extent();
            if descriptor.default_view.center.0 < extent[0]
                || descriptor.default_view.center.0 > extent[2]
                || descriptor.default_view.center.1 < extent[1]
                || descriptor.default_view.center.1 > extent[3]
            {
                return Err(invalid("defaultView.center must be inside coordinateSpace.extent"));
            }
            let settings = descriptor
                .settings
                .as_ref()
                .ok_or_else(|| invalid("daena-openlayers maps require settings"))?;
            if let Some(grid) = &settings.grid {
                if grid.spacing.iter().any(|value| !value.is_finite() || *value <= 0.0) {
                    return Err(invalid("settings.grid.spacing must contain positive finite values"));
                }
            }
            let mut background_ids = BTreeSet::new();
            let mut background_orders = BTreeSet::new();
            for background in &descriptor.backgrounds {
                uuid(&background.id, "background.id")?;
                if !background_ids.insert(background.id.as_str()) {
                    return Err(invalid("background IDs must be unique"));
                }
                if !background_orders.insert((background.order, background.id.as_str())) {
                    return Err(invalid("background order is not deterministic"));
                }
                if background.name.trim().is_empty() || background.name.len() > 128 {
                    return Err(invalid("background name has invalid length"));
                }
                if !background.opacity.is_finite() || !(0.0..=1.0).contains(&background.opacity) {
                    return Err(invalid("background opacity must be finite in [0, 1]"));
                }
                validate_extent(&background.extent, "background.extent")?;
                let (_, mime_type) = owned_map_asset(
                    connection,
                    entity_id,
                    &background.asset_id,
                    "background.assetId",
                )?;
                source_format_for_mime(&mime_type)?;
            }
        } else if descriptor.coordinate_space.is_some()
            || !descriptor.backgrounds.is_empty()
            || descriptor.settings.is_some()
        {
            return Err(invalid(
                "coordinateSpace, backgrounds, and settings are only valid on daena-openlayers maps",
            ));
        }
        Ok(())
    } else if key == PHYSICAL_EVENT_CHRONOLOGY_KEY {
        if entity_type.as_deref() != Some(PHYSICAL_EVENT_ENTITY_TYPE) {
            return Err(invalid(
                "physical chronology belongs only on a physical natural event",
            ));
        }
        let object = value
            .as_object()
            .ok_or_else(|| invalid("physical chronology must be an object"))?;
        if object.keys().any(|key| {
            !matches!(
                key.as_str(),
                "contractVersion" | "kind" | "reference" | "startOffsetYears" | "endOffsetYears"
            )
        }) {
            return Err(invalid("physical chronology contains an unknown field"));
        }
        if object.get("contractVersion").and_then(Value::as_i64) != Some(1)
            || object.get("kind").and_then(Value::as_str) != Some("physical-offset-years")
            || object.get("reference").and_then(Value::as_str) != Some("accepted-source")
        {
            return Err(invalid("unsupported physical chronology contract"));
        }
        let start = object
            .get("startOffsetYears")
            .and_then(Value::as_i64)
            .ok_or_else(|| invalid("physical chronology startOffsetYears must be an integer"))?;
        let end = object
            .get("endOffsetYears")
            .and_then(Value::as_i64)
            .ok_or_else(|| invalid("physical chronology endOffsetYears must be an integer"))?;
        if start < -PHYSICAL_EVENT_MAX_OFFSET_YEARS || end > PHYSICAL_EVENT_MAX_OFFSET_YEARS {
            return Err(invalid(format!(
                "physical chronology must stay within +/-{PHYSICAL_EVENT_MAX_OFFSET_YEARS} years"
            )));
        }
        if start > end {
            return Err(invalid(
                "physical chronology startOffsetYears cannot be after endOffsetYears",
            ));
        }
        Ok(())
    } else if key == "locations" {
        let object = value
            .as_object()
            .ok_or_else(|| invalid("locations must be an object"))?;
        if object.get("schemaVersion").and_then(Value::as_i64) != Some(1) {
            return Err(invalid("locations.schemaVersion must be 1"));
        }
        let locations = object
            .get("locations")
            .and_then(Value::as_array)
            .ok_or_else(|| invalid("locations.locations must be an array"))?;
        let mut ids = BTreeSet::new();
        for item in locations {
            let location: LocationReference = serde_json::from_value(item.clone())
                .map_err(|e| invalid(format!("invalid location reference: {e}")))?;
            uuid(&location.id, "location.id")?;
            uuid(&location.map_entity_id, "location.mapEntityId")?;
            if !ids.insert(location.id.clone()) {
                return Err(invalid("location IDs must be unique"));
            }
            let map_provider: Option<String> = connection.query_row("SELECT json_extract(value, '$.provider.id') FROM entity_fields WHERE entity_id=?1 AND namespace=?2 AND key='map'", [&location.map_entity_id, MAP_NAMESPACE], |row| row.get(0)).optional().map_err(CoreError::from)?;
            if map_provider.is_none() {
                return Err(invalid("dangling map reference"));
            }
            if location.role.trim().is_empty()
                || location.role.len() > 128
                || location.label.len() > 256
            {
                return Err(invalid("location role or label has invalid length"));
            }
            let anchor_value =
                serde_json::to_value(&location.anchor).map_err(|e| invalid(e.to_string()))?;
            anchor(&anchor_value)?;
            if let Anchor::ProviderFeature { provider, .. } = &location.anchor {
                if map_provider.as_deref() != Some(provider.as_str()) {
                    return Err(invalid(
                        "location provider does not match the referenced map",
                    ));
                }
            }
            let validity_value =
                serde_json::to_value(&location.validity).map_err(|e| invalid(e.to_string()))?;
            validity(&validity_value, "location.validity")?;
        }
        Ok(())
    } else if key == "layers" {
        if entity_type.as_deref() != Some(MAP_ENTITY_TYPE) {
            return Err(invalid("layers belong only on a map entity"));
        }
        let object = value
            .as_object()
            .ok_or_else(|| invalid("layers must be an object"))?;
        if object
            .keys()
            .any(|key| !matches!(key.as_str(), "schemaVersion" | "layers"))
        {
            return Err(invalid("layers must contain schemaVersion and layers only"));
        }
        let layers = object
            .get("layers")
            .and_then(Value::as_array)
            .ok_or_else(|| invalid("layers.layers must be an array"))?;
        let mut ids = BTreeSet::new();
        let mut raster_assets = BTreeSet::new();
        let mut sort_keys = BTreeSet::new();
        let mut raster_count = 0usize;
        let mut semantic_count = 0usize;
        let mut vector_count = 0usize;
        let map_provider = connection
            .query_row(
                "SELECT json_extract(value, '$.provider.id') FROM entity_fields WHERE entity_id=?1 AND namespace=?2 AND key='map'",
                [entity_id, MAP_NAMESPACE],
                |row| row.get::<_, Option<String>>(0),
            )
            .optional()?
            .flatten();
        let physical_map = map_provider.as_deref() == Some(PHYSICAL_PROVIDER);
        let expected_schema = 2;
        if object.get("schemaVersion").and_then(Value::as_i64) != Some(expected_schema) {
            return Err(invalid(format!(
                "layers.schemaVersion must be {expected_schema} for this map provider"
            )));
        }
        let physical_layers = if physical_map {
            physical::initial_layers_value()
                .get("layers")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default()
        } else {
            Vec::new()
        };
        let physical_layer_by_id = physical_layers
            .iter()
            .filter_map(|layer| {
                layer
                    .get("id")
                    .and_then(Value::as_str)
                    .map(|id| (id, layer))
            })
            .collect::<BTreeMap<_, _>>();
        let mut seen_physical_layers = BTreeSet::new();
        for layer in layers {
            let layer: LayerDefinition = serde_json::from_value(layer.clone())
                .map_err(|_| invalid("layer definition is invalid"))?;
            if !physical_map || !physical::is_reserved_layer_id(layer.id()) {
                uuid(layer.id(), "layer.id")?;
            }
            if !ids.insert(layer.id().to_owned()) {
                return Err(invalid("layer IDs must be unique"));
            }
            if let Some(expected) = physical_layer_by_id.get(layer.id()) {
                let actual = serde_json::to_value(&layer)
                    .map_err(|_| invalid("layer definition is invalid"))?;
                for key in ["id", "name", "order", "kind", "locked", "selector", "style"] {
                    if actual.get(key) != expected.get(key) {
                        return Err(invalid("physical layer definitions are immutable"));
                    }
                }
                if !seen_physical_layers.insert(layer.id().to_owned()) {
                    return Err(invalid("physical layer IDs must be unique"));
                }
            }
            if !sort_keys.insert((layer.sort_key().0, layer.id().to_owned())) {
                return Err(invalid("layer order is not deterministic"));
            }
            if layer.name().trim().is_empty() || layer.name().len() > 128 {
                return Err(invalid("layer definition is invalid"));
            }
            if !layer.style().is_object() || !layer.selector().is_object() {
                return Err(invalid("layer definition is invalid"));
            }
            if let LayerDefinition::Raster(raster) = &layer {
                if raster.style != serde_json::json!({}) || raster.selector != serde_json::json!({})
                {
                    return Err(invalid(
                        "raster layers must have empty style and selector objects",
                    ));
                }
                if !raster.opacity.is_finite() || !(0.0..=1.0).contains(&raster.opacity) {
                    return Err(invalid("raster layer opacity must be finite in [0, 1]"));
                }
                if !matches!(raster.blend_mode.as_str(), "normal" | "multiply" | "screen" | "overlay") {
                    return Err(invalid("raster layer blendMode is unsupported"));
                }
                if !raster_assets.insert(raster.raster_asset_id.clone()) {
                    return Err(invalid("rasterAssetId must be unique"));
                }
                let (_, mime_type) = owned_map_asset(
                    connection,
                    entity_id,
                    &raster.raster_asset_id,
                    "rasterAssetId",
                )?;
                if mime_type != "image/png" {
                    return Err(invalid("rasterAssetId must name a PNG asset"));
                }
                raster_count += 1;
                if raster_count > IMAGE_MAX_RASTER_LAYERS {
                    return Err(invalid(format!(
                        "raster layer count exceeds the budget of {IMAGE_MAX_RASTER_LAYERS}"
                    )));
                }
            } else if let LayerDefinition::Vector(vector_layer) = &layer {
                if vector_layer.selector != serde_json::json!({}) {
                    return Err(invalid("vector layers must have an empty selector"));
                }
                vector::validate_vector_style(&vector_layer.style)?;
                if !vector_layer.opacity.is_finite()
                    || !(0.0..=1.0).contains(&vector_layer.opacity)
                    || !matches!(
                        vector_layer.blend_mode.as_str(),
                        "normal" | "multiply" | "screen" | "overlay"
                    )
                {
                    return Err(invalid("vector layer opacity or blendMode is invalid"));
                }
                vector_count += 1;
                if vector_count > VECTOR_MAX_LAYERS {
                    return Err(invalid(format!(
                        "vector layer count exceeds the budget of {VECTOR_MAX_LAYERS}"
                    )));
                }
            } else if let LayerDefinition::Semantic(semantic) = &layer {
                validate_semantic_style(&semantic.style)?;
                validate_semantic_selector(&semantic.selector)?;
                semantic_count += 1;
                if semantic_count > IMAGE_MAX_SEMANTIC_LAYERS {
                    return Err(invalid(format!(
                        "semantic layer count exceeds the budget of {IMAGE_MAX_SEMANTIC_LAYERS}"
                    )));
                }
            }
        }
        if physical_map && seen_physical_layers.len() != physical_layer_by_id.len() {
            return Err(invalid(
                "physical maps must retain all locked physical layers",
            ));
        }
        Ok(())
    } else if key == calendar::PHYSICAL_CALENDAR_BINDING_KEY {
        if entity_type.as_deref() != Some(MAP_ENTITY_TYPE) {
            return Err(invalid(
                "physical calendar binding belongs only on a map entity",
            ));
        }
        atlas::validate_calendar_binding(value)
    } else if key == atlas::ATLAS_PRESETS_KEY {
        if entity_type.as_deref() != Some(MAP_ENTITY_TYPE) {
            return Err(invalid("atlas presets belong only on a map entity"));
        }
        atlas::validate_presets(value)
    } else {
        Ok(())
    }
}

pub fn validate_image_map_content(
    connection: &Connection,
    mut load_asset: impl FnMut(&str) -> Result<Vec<u8>, CoreError>,
) -> Result<(), CoreError> {
    let mut source_dimensions = std::collections::BTreeMap::new();
    let mut maps = connection
        .prepare("SELECT entity_id, value FROM entity_fields WHERE namespace=?1 AND key='map'")?;
    let map_rows = maps.query_map([MAP_NAMESPACE], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
    })?;
    for row in map_rows {
        let (entity_id, raw) = row?;
        let value: Value =
            serde_json::from_str(&raw).map_err(|error| invalid(error.to_string()))?;
        validate_field(connection, &entity_id, "map", &value)?;
        let descriptor: MapDescriptor = serde_json::from_value(value)
            .map_err(|error| invalid(format!("invalid map descriptor: {error}")))?;
        let provider = provider_spec(&descriptor.provider)?;
        match provider.kind {
            ProviderKind::Physical => {
                let source_id = descriptor
                    .source_asset_id
                    .as_deref()
                    .ok_or_else(|| invalid("daena-physical maps require sourceAssetId"))?;
                let generation = descriptor
                    .generation
                    .as_ref()
                    .ok_or_else(|| invalid("daena-physical maps require generation"))?;
                let generation =
                    serde_json::to_value(generation).map_err(|error| invalid(error.to_string()))?;
                let bytes = load_asset(source_id)?;
                physical::validate_source(&bytes, &generation)?;
                let authored_source_id = descriptor
                    .authored_source_asset_id
                    .as_deref()
                    .ok_or_else(|| invalid("daena-physical maps require authoredSourceAssetId"))?;
                let authored_bytes = load_asset(authored_source_id)?;
                let layers_value: Option<Value> = connection
                    .query_row(
                        "SELECT value FROM entity_fields WHERE entity_id=?1 AND namespace=?2 AND key='layers'",
                        [&entity_id, MAP_NAMESPACE],
                        |row| row.get(0),
                    )
                    .optional()?
                    .map(|raw: String| serde_json::from_str(&raw))
                    .transpose()
                    .map_err(|error| invalid(error.to_string()))?;
                let known = layers_value
                    .as_ref()
                    .map(vector::layer_ids_from_layers_field)
                    .unwrap_or_default();
                let canonical = vector::require_canonical_bytes(
                    Path::new("assets/maps/map.geojson"),
                    &authored_bytes,
                    &known,
                )?;
                validate_physical_authored_source_bytes(&canonical)?;
            }
            ProviderKind::Vector => {
                let source_id = descriptor
                    .source_asset_id
                    .as_deref()
                    .ok_or_else(|| invalid("daena-openlayers maps require sourceAssetId"))?;
                let bytes = load_asset(source_id)?;
                let layers_value: Option<Value> = connection
                    .query_row(
                        "SELECT value FROM entity_fields WHERE entity_id=?1 AND namespace=?2 AND key='layers'",
                        [&entity_id, MAP_NAMESPACE],
                        |row| row.get(0),
                    )
                    .optional()?
                    .map(|raw: String| serde_json::from_str(&raw))
                    .transpose()
                    .map_err(|error| invalid(error.to_string()))?;
                let known = layers_value
                    .as_ref()
                    .map(vector::layer_ids_from_layers_field)
                    .unwrap_or_default();
                vector::require_canonical_bytes(
                    Path::new("assets/maps/map.geojson"),
                    &bytes,
                    &known,
                )?;
                if let Some(preview_id) = descriptor.preview_asset_id.as_deref() {
                    let preview_bytes = load_asset(preview_id)?;
                    let (_, mime) =
                        owned_map_asset(connection, &entity_id, preview_id, "previewAssetId")?;
                    let source = validate_image_source(&preview_bytes, &mime)?;
                    source_dimensions.insert(entity_id, (source.width, source.height));
                }
            }
            ProviderKind::Fmg => {}
        }
    }
    let mut layers = connection.prepare(
        "SELECT entity_id, value FROM entity_fields WHERE namespace=?1 AND key='layers'",
    )?;
    let layer_rows = layers.query_map([MAP_NAMESPACE], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
    })?;
    for row in layer_rows {
        let (entity_id, raw) = row?;
        let value: Value =
            serde_json::from_str(&raw).map_err(|error| invalid(error.to_string()))?;
        validate_field(connection, &entity_id, "layers", &value)?;
        let Some((width, height)) = source_dimensions.get(&entity_id) else {
            continue;
        };
        let layers = value
            .get("layers")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        for layer in layers {
            if layer.get("kind").and_then(Value::as_str) != Some("raster") {
                continue;
            }
            let raster_id = layer
                .get("rasterAssetId")
                .and_then(Value::as_str)
                .ok_or_else(|| invalid("rasterAssetId is required"))?;
            let bytes = load_asset(raster_id)?;
            validate_raster_png(&bytes, *width, *height)?;
        }
    }
    Ok(())
}

pub fn validate_physical_authored_source_bytes(bytes: &[u8]) -> Result<(), CoreError> {
    let collection: Value = serde_json::from_slice(bytes)
        .map_err(|error| invalid(format!("physical authored source is invalid: {error}")))?;
    let reserved = collection
        .get("features")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|feature| {
            feature
                .pointer("/properties/daena/layerId")
                .and_then(Value::as_str)
                .or_else(|| {
                    feature
                        .pointer("/properties/daenaLayerId")
                        .and_then(Value::as_str)
                })
        })
        .any(physical::is_reserved_layer_id);
    if reserved {
        return Err(CoreError::Validation(
            "maps: physical layers are immutable".into(),
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests;
