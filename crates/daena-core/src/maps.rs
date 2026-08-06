use crate::error::CoreError;
use rusqlite::{Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeSet;
use uuid::Uuid;

pub const MAP_ENTITY_TYPE: &str = "daena.maps:map";
pub const MAP_NAMESPACE: &str = "daena.maps";
pub const FMG_PROVIDER: &str = "azgaar-fmg";

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
pub struct MapDescriptor {
    #[serde(rename = "schemaVersion")]
    pub schema_version: u32,
    pub provider: ProviderDescriptor,
    #[serde(rename = "sourceAssetId")]
    pub source_asset_id: String,
    #[serde(rename = "previewAssetId")]
    pub preview_asset_id: Option<String>,
    #[serde(rename = "defaultView")]
    pub default_view: DefaultView,
}

fn invalid(message: impl Into<String>) -> CoreError {
    CoreError::Validation(format!("maps: {}", message.into()))
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
            "coordinates must be finite normalized values in [0, 1]",
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
        || !matches!(precision, "year" | "month" | "day")
    {
        return Err(invalid(format!("{label} is not a valid Daena date")));
    }
    let month = object.get("month").and_then(Value::as_i64);
    let day = object.get("day").and_then(Value::as_i64);
    if matches!(precision, "month" | "day") && !matches!(month, Some(1..=12))
        || precision == "day" && !matches!(day, Some(1..=31))
        || precision == "year" && (month.is_some() || day.is_some())
        || precision == "month" && day.is_some()
    {
        return Err(invalid(format!("{label} has inconsistent precision")));
    }
    Ok(())
}

fn validity(value: &Value, label: &str) -> Result<(), CoreError> {
    let object = value
        .as_object()
        .ok_or_else(|| invalid(format!("{label} must be an object")))?;
    if object.len() != 2 || !object.contains_key("from") || !object.contains_key("to") {
        return Err(invalid(format!("{label} must contain only from and to")));
    }
    if let Some(from) = object.get("from").filter(|v| !v.is_null()) {
        date(from, &format!("{label}.from"))?;
    }
    if let Some(to) = object.get("to").filter(|v| !v.is_null()) {
        date(to, &format!("{label}.to"))?;
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
                return Err(invalid("path requires at least two points"));
            }
            for p in points {
                point(p)?;
            }
        }
        Anchor::Area { rings } => {
            if rings.is_empty() {
                return Err(invalid("area requires at least one ring"));
            }
            for ring in rings {
                if ring.len() < 4 || ring.first() != ring.last() {
                    return Err(invalid(
                        "area rings must be closed and contain at least four points",
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
        if descriptor.schema_version != 1
            || descriptor.provider.id != FMG_PROVIDER
            || descriptor.provider.adapter_version != 1
            || descriptor.provider.source_format != "fmg-map"
        {
            return Err(invalid("unsupported map provider or descriptor version"));
        }
        uuid(&descriptor.source_asset_id, "sourceAssetId")?;
        if let Some(id) = &descriptor.preview_asset_id {
            uuid(id, "previewAssetId")?;
        }
        let source_owner: Option<String> = connection
            .query_row(
                "SELECT entity_id FROM assets WHERE id=?1 AND namespace=?2",
                [&descriptor.source_asset_id, MAP_NAMESPACE],
                |row| row.get(0),
            )
            .optional()
            .map_err(CoreError::from)?;
        if source_owner.as_deref() != Some(entity_id) {
            return Err(invalid(
                "sourceAssetId must name an asset owned by the map entity in daena.maps",
            ));
        }
        if let Some(preview) = &descriptor.preview_asset_id {
            let preview_owner: Option<String> = connection
                .query_row(
                    "SELECT entity_id FROM assets WHERE id=?1 AND namespace=?2",
                    [preview, MAP_NAMESPACE],
                    |row| row.get(0),
                )
                .optional()
                .map_err(CoreError::from)?;
            if preview_owner.as_deref() != Some(entity_id) {
                return Err(invalid(
                    "previewAssetId must name an asset owned by the map entity in daena.maps",
                ));
            }
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
                return Err(invalid("location references a missing or non-map entity"));
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
        let object = value
            .as_object()
            .ok_or_else(|| invalid("layers must be an object"))?;
        if object.keys().any(|key| !matches!(key.as_str(), "schemaVersion" | "layers"))
            || object.get("schemaVersion").and_then(Value::as_i64) != Some(1)
        {
            return Err(invalid("layers must contain schemaVersion 1 and layers only"));
        }
        let layers = object
            .get("layers")
            .and_then(Value::as_array)
            .ok_or_else(|| invalid("layers.layers must be an array"))?;
        let mut ids = BTreeSet::new();
        for layer in layers {
            let layer = layer
                .as_object()
                .ok_or_else(|| invalid("each layer must be an object"))?;
            if layer.keys().any(|key| !matches!(key.as_str(), "id" | "name" | "order" | "defaultVisible" | "style" | "selector"))
                || !ids.insert(layer.get("id").and_then(Value::as_str).unwrap_or_default().to_owned())
                || Uuid::parse_str(layer.get("id").and_then(Value::as_str).unwrap_or_default()).is_err()
                || layer.get("name").and_then(Value::as_str).is_none_or(|name| name.trim().is_empty() || name.len() > 128)
                || layer.get("order").and_then(Value::as_i64).is_none()
                || layer.get("defaultVisible").and_then(Value::as_bool).is_none()
                || !layer.get("style").is_some_and(Value::is_object)
                || !layer.get("selector").is_some_and(Value::is_object)
            {
                return Err(invalid("layer definition is invalid"));
            }
        }
        Ok(())
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_out_of_range_and_open_geometry() {
        assert!(point(&Point(1.1, 0.2)).is_err());
        assert!(anchor(
            &serde_json::json!({"kind":"area","rings":[[[0.1,0.1],[0.2,0.1],[0.2,0.2],[0.1,0.2]]]})
        )
        .is_err());
        assert!(anchor(&serde_json::json!({"kind":"path","points":[[0.1,0.1],[0.2,0.2]]})).is_ok());
    }

    #[test]
    fn validates_dates_without_inventing_precision() {
        let year =
            serde_json::json!({"calendar":"gregorian","era":"CE","year":42,"precision":"year"});
        assert!(date(&year, "date").is_ok());
        let incomplete_month =
            serde_json::json!({"calendar":"gregorian","era":"CE","year":42,"precision":"month"});
        assert!(date(&incomplete_month, "date").is_err());
    }
}
