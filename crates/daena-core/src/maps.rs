use crate::error::CoreError;
use rusqlite::{Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeSet;
use uuid::Uuid;

pub const MAP_ENTITY_TYPE: &str = "daena.maps:map";
pub const MAP_NAMESPACE: &str = "maps";
pub const FMG_PROVIDER: &str = "azgaar-fmg";
pub const IMAGE_PROVIDER: &str = "daena-image";
pub const DETAIL_MAP_RELATIONSHIP: &str = "daena.maps:detail-map";
pub const OVERVIEW_MAP_RELATIONSHIP: &str = "daena.maps:overview-map";
pub const RELATED_MAP_RELATIONSHIP: &str = "daena.maps:related-map";

pub mod image;
pub use image::{
    encode_transparent_png, mime_for_source_format, source_format_for_mime, validate_image_source,
    validate_raster_png, ImageSource, IMAGE_MAX_ENCODED_BYTES, IMAGE_MAX_PIXELS,
    IMAGE_MAX_RASTER_LAYERS,
};

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

/// Upper bound for a rejected map draft written to the recovery directory.
/// Mirrors the host transfer ceiling; larger payloads cannot arrive here.
pub const MAP_RECOVERY_MAX_BYTES: usize = 64 * 1024 * 1024;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct MapDescriptor {
    #[serde(rename = "schemaVersion")]
    pub schema_version: u32,
    pub provider: ProviderDescriptor,
    #[serde(rename = "sourceAssetId")]
    pub source_asset_id: Option<String>,
    #[serde(rename = "previewAssetId")]
    pub preview_asset_id: Option<String>,
    #[serde(rename = "defaultView")]
    pub default_view: DefaultView,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum RasterLayerKind {
    #[serde(rename = "raster")]
    Raster,
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
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum LayerDefinition {
    Raster(RasterLayerDefinition),
    Semantic(SemanticLayerDefinition),
}

impl LayerDefinition {
    fn id(&self) -> &str {
        match self {
            Self::Raster(layer) => layer.id.as_str(),
            Self::Semantic(layer) => layer.id.as_str(),
        }
    }

    fn name(&self) -> &str {
        match self {
            Self::Raster(layer) => layer.name.as_str(),
            Self::Semantic(layer) => layer.name.as_str(),
        }
    }

    fn style(&self) -> &Value {
        match self {
            Self::Raster(layer) => &layer.style,
            Self::Semantic(layer) => &layer.style,
        }
    }

    fn selector(&self) -> &Value {
        match self {
            Self::Raster(layer) => &layer.selector,
            Self::Semantic(layer) => &layer.selector,
        }
    }

    fn sort_key(&self) -> (i64, &str) {
        match self {
            Self::Raster(layer) => (layer.order, layer.id.as_str()),
            Self::Semantic(layer) => (layer.order, layer.id.as_str()),
        }
    }
}

fn invalid(message: impl Into<String>) -> CoreError {
    CoreError::Validation(format!("maps: {}", message.into()))
}

fn image_source_mime(source_format: &str) -> Option<&'static str> {
    match source_format {
        "png" => Some("image/png"),
        "jpeg" => Some("image/jpeg"),
        "svg" => Some("image/svg+xml"),
        _ => None,
    }
}

fn validate_provider(provider: &ProviderDescriptor) -> Result<(), CoreError> {
    let supported = provider.adapter_version == 1
        && match provider.id.as_str() {
            FMG_PROVIDER => provider.source_format == "fmg-map",
            IMAGE_PROVIDER => matches!(provider.source_format.as_str(), "png" | "jpeg" | "svg"),
            _ => false,
        };
    if supported {
        Ok(())
    } else {
        Err(invalid("unsupported map provider or descriptor version"))
    }
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
            Some("BCE") | Some("CE")
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
            if provider != FMG_PROVIDER
                || !matches!(
                    feature_kind.as_str(),
                    "burg" | "state" | "province" | "river" | "marker"
                )
                || feature_id.is_empty()
            {
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
            for p in points {
                point(p)?;
            }
        }
        Anchor::Area { rings } => {
            if rings.is_empty() {
                return Err(invalid("invalid geometry: area requires at least one ring"));
            }
            for ring in rings {
                if ring.len() < 4 || ring.first() != ring.last() {
                    return Err(invalid(
                        "invalid geometry: area rings must be closed and contain at least four points",
                    ));
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
        if descriptor.schema_version != 1 {
            return Err(invalid("unsupported map provider or descriptor version"));
        }
        validate_provider(&descriptor.provider)?;
        if descriptor.provider.id == IMAGE_PROVIDER && descriptor.source_asset_id.is_none() {
            return Err(invalid("daena-image maps require sourceAssetId"));
        }
        if let Some(source_asset_id) = &descriptor.source_asset_id {
            let (_, mime_type) =
                owned_map_asset(connection, entity_id, source_asset_id, "sourceAssetId")?;
            if let Some(expected_mime) = image_source_mime(&descriptor.provider.source_format) {
                if mime_type != expected_mime {
                    return Err(invalid(
                        "sourceAssetId MIME type must match the image sourceFormat",
                    ));
                }
            }
        }
        if let Some(preview) = &descriptor.preview_asset_id {
            owned_map_asset(connection, entity_id, preview, "previewAssetId")?;
        }
        if descriptor.default_view.zoom <= 0.0 || !descriptor.default_view.zoom.is_finite() {
            return Err(invalid("defaultView.zoom must be finite and positive"));
        }
        point(&descriptor.default_view.center)
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
            || object.get("schemaVersion").and_then(Value::as_i64) != Some(1)
        {
            return Err(invalid(
                "layers must contain schemaVersion 1 and layers only",
            ));
        }
        let layers = object
            .get("layers")
            .and_then(Value::as_array)
            .ok_or_else(|| invalid("layers.layers must be an array"))?;
        let mut ids = BTreeSet::new();
        let mut raster_assets = BTreeSet::new();
        let mut sort_keys = BTreeSet::new();
        let mut raster_count = 0usize;
        for layer in layers {
            let layer: LayerDefinition = serde_json::from_value(layer.clone())
                .map_err(|_| invalid("layer definition is invalid"))?;
            uuid(layer.id(), "layer.id")?;
            if !ids.insert(layer.id().to_owned()) {
                return Err(invalid("layer IDs must be unique"));
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
                    return Err(invalid("raster layer count exceeds the budget"));
                }
            }
        }
        Ok(())
    } else {
        Ok(())
    }
}

pub fn validate_image_map_content(
    connection: &Connection,
    mut load_asset: impl FnMut(&str) -> Result<Vec<u8>, CoreError>,
) -> Result<(), CoreError> {
    let mut source_dimensions = std::collections::BTreeMap::new();
    let mut maps = connection.prepare(
        "SELECT entity_id, value FROM entity_fields WHERE namespace=?1 AND key='map'",
    )?;
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
        if descriptor.provider.id != IMAGE_PROVIDER {
            continue;
        }
        let source_id = descriptor
            .source_asset_id
            .as_deref()
            .ok_or_else(|| invalid("daena-image maps require sourceAssetId"))?;
        let bytes = load_asset(source_id)?;
        let mime = mime_for_source_format(&descriptor.provider.source_format)?;
        let source = validate_image_source(&bytes, mime)?;
        source_dimensions.insert(entity_id, (source.width, source.height));
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

#[cfg(test)]
mod tests;
