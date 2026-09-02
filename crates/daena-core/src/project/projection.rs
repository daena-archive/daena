// Map location projection helpers.
use super::runtime_assets::runtime_asset_path;
use crate::error::CoreError;
use rusqlite::OptionalExtension;
use std::collections::BTreeMap;
use std::path::Path;

pub(super) const LOCATION_PROJECTION_SELECT: &str = "SELECT location_id,entity_id,map_entity_id,label,role,anchor_kind,provider,feature_kind,feature_id,min_x,min_y,max_x,max_y,valid_from,valid_to,resolution,geometry FROM map_location_projection";

pub(super) fn location_projection_json(
    row: &rusqlite::Row<'_>,
) -> rusqlite::Result<serde_json::Value> {
    let geometry: Option<String> = row.get(16)?;
    let anchor = geometry
        .as_deref()
        .and_then(|raw| serde_json::from_str::<serde_json::Value>(raw).ok())
        .unwrap_or(serde_json::Value::Null);
    Ok(serde_json::json!({
        "id": row.get::<_, String>(0)?,
        "entityId": row.get::<_, String>(1)?,
        "mapEntityId": row.get::<_, String>(2)?,
        "label": row.get::<_, Option<String>>(3)?,
        "role": row.get::<_, String>(4)?,
        "anchorKind": row.get::<_, String>(5)?,
        "provider": row.get::<_, Option<String>>(6)?,
        "featureKind": row.get::<_, Option<String>>(7)?,
        "featureId": row.get::<_, Option<String>>(8)?,
        "bounds": [
            row.get::<_, Option<f64>>(9)?,
            row.get::<_, Option<f64>>(10)?,
            row.get::<_, Option<f64>>(11)?,
            row.get::<_, Option<f64>>(12)?
        ],
        "validity": {
            "from": row.get::<_, Option<String>>(13)?,
            "to": row.get::<_, Option<String>>(14)?
        },
        "resolution": row.get::<_, String>(15)?,
        "anchor": anchor
    }))
}

pub(super) fn write_vector_feature_projection(
    connection: &rusqlite::Connection,
    root: Option<&Path>,
    map_entity_id: &str,
    provider: &str,
    source_hash: Option<&str>,
) -> Result<(), CoreError> {
    if provider != crate::maps::VECTOR_PROVIDER && provider != crate::maps::PHYSICAL_PROVIDER {
        return Ok(());
    }
    let Some(hash) = source_hash.filter(|value| !value.is_empty()) else {
        return Ok(());
    };
    let Some(root) = root else {
        return Ok(());
    };
    let path = runtime_asset_path(root, hash)?;
    let bytes = match std::fs::read(&path) {
        Ok(bytes) => bytes,
        Err(_) => return Ok(()),
    };
    let layers: serde_json::Value = connection
        .query_row(
            "SELECT value FROM entity_fields WHERE entity_id=?1 AND namespace=?2 AND key='layers'",
            rusqlite::params![map_entity_id, crate::maps::MAP_NAMESPACE],
            |row| row.get::<_, String>(0),
        )
        .optional()?
        .map(|raw| serde_json::from_str(&raw))
        .transpose()
        .map_err(|error| CoreError::Serialization(error.to_string()))?
        .unwrap_or_else(|| serde_json::json!({"schemaVersion": crate::maps::MAP_LAYERS_SCHEMA_VERSION, "layers": []}));
    let known = crate::maps::vector::layer_ids_from_layers_field(&layers);
    // Resolve vector space from the map descriptor for correct bounds normalization
    let descriptor_value: Option<serde_json::Value> = connection
        .query_row(
            "SELECT value FROM entity_fields WHERE entity_id=?1 AND namespace=?2 AND key='map'",
            rusqlite::params![map_entity_id, crate::maps::MAP_NAMESPACE],
            |row| row.get::<_, String>(0),
        )
        .optional()?
        .map(|raw| serde_json::from_str(&raw))
        .transpose()
        .map_err(|error| CoreError::Serialization(error.to_string()))?;
    let vector_space = descriptor_value
        .as_ref()
        .map(crate::maps::vector::VectorSpace::from_descriptor_value)
        .unwrap_or(crate::maps::vector::VectorSpace::Geographic);
    let features = crate::maps::vector::feature_bounds_with_space(&bytes, &known, &vector_space)?;
    connection.execute(
        "DELETE FROM map_feature_search WHERE map_entity_id=?1",
        rusqlite::params![map_entity_id],
    )?;
    connection.execute(
        "DELETE FROM world_search WHERE entity_id=?1 AND source_key LIKE 'map-feature:%'",
        rusqlite::params![map_entity_id],
    )?;
    for (feature_id, layer_id, kind, min_x, min_y, max_x, max_y) in features {
        connection.execute(
            "INSERT OR REPLACE INTO map_feature_projection(map_entity_id,feature_id,layer_id,kind,min_x,min_y,max_x,max_y) VALUES (?1,?2,?3,?4,?5,?6,?7,?8)",
            rusqlite::params![map_entity_id, feature_id, layer_id, kind, min_x, min_y, max_x, max_y],
        )?;
    }
    let layer_names = layers
        .get("layers")
        .and_then(serde_json::Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|layer| {
            Some((
                layer.get("id")?.as_str()?.to_owned(),
                layer.get("name")?.as_str()?.to_owned(),
            ))
        })
        .collect::<BTreeMap<_, _>>();
    let source: serde_json::Value = serde_json::from_slice(&bytes)
        .map_err(|error| CoreError::Serialization(error.to_string()))?;
    for feature in source
        .get("features")
        .and_then(serde_json::Value::as_array)
        .into_iter()
        .flatten()
    {
        let Some(feature_id) = feature.get("id").and_then(serde_json::Value::as_str) else {
            continue;
        };
        let Some(daena) = feature.pointer("/properties/daena") else {
            continue;
        };
        let layer_id = daena
            .get("layerId")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("base");
        let semantic_type = daena
            .get("semanticType")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("custom");
        let name = daena
            .get("name")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("Untitled feature");
        let layer_name = layer_names
            .get(layer_id)
            .map(String::as_str)
            .unwrap_or("Unknown layer");
        let custom = daena
            .get("custom")
            .map(serde_json::Value::to_string)
            .unwrap_or_default();
        let content = format!("{feature_id} {name} {semantic_type} {layer_name} {custom}");
        connection.execute(
            "INSERT INTO map_feature_search(map_entity_id,feature_id,name,semantic_type,layer_id,layer_name,content) VALUES (?1,?2,?3,?4,?5,?6,?7)",
            rusqlite::params![map_entity_id, feature_id, name, semantic_type, layer_id, layer_name, content],
        )?;
        connection.execute(
            "INSERT INTO world_search(entity_id,source_path,source_hash,content,source_key) VALUES (?1,?2,?3,?4,?5)",
            rusqlite::params![
                map_entity_id,
                format!("entities/{map_entity_id}/map-features/{feature_id}"),
                hash,
                content,
                format!("map-feature:{feature_id}")
            ],
        )?;
    }
    Ok(())
}

pub(super) fn write_location_projection(
    connection: &rusqlite::Connection,
    entity_id: &str,
    location: &serde_json::Value,
    resolution: &str,
) -> Result<(), CoreError> {
    let anchor = location.get("anchor").cloned().unwrap_or_default();
    let kind = anchor
        .get("kind")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("unknown");
    let bounds = match kind {
        "point" => bounds_for_points(anchor.get("point").and_then(serde_json::Value::as_array)),
        "provider-feature" => bounds_for_points(
            anchor
                .get("fallbackPoint")
                .and_then(serde_json::Value::as_array),
        ),
        "path" => bounds_for_points(anchor.get("points").and_then(serde_json::Value::as_array)),
        "area" => anchor
            .get("rings")
            .and_then(serde_json::Value::as_array)
            .map_or((None, None, None, None), |rings| {
                bounds_for_points(Some(
                    &rings
                        .iter()
                        .filter_map(serde_json::Value::as_array)
                        .flatten()
                        .cloned()
                        .collect::<Vec<_>>(),
                ))
            }),
        _ => (None, None, None, None),
    };
    let geometry = serde_json::to_string(&anchor)
        .map_err(|error| CoreError::Serialization(error.to_string()))?;
    connection.execute(
        "INSERT OR REPLACE INTO map_location_projection(location_id,entity_id,map_entity_id,label,role,anchor_kind,provider,feature_kind,feature_id,min_x,min_y,max_x,max_y,valid_from,valid_to,resolution,geometry) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17)",
        rusqlite::params![
            location.get("id").and_then(serde_json::Value::as_str).unwrap_or_default(),
            entity_id,
            location.get("mapEntityId").and_then(serde_json::Value::as_str).unwrap_or_default(),
            location.get("label").and_then(serde_json::Value::as_str),
            location.get("role").and_then(serde_json::Value::as_str).unwrap_or_default(),
            kind,
            anchor.get("provider").and_then(serde_json::Value::as_str),
            anchor.get("featureKind").and_then(serde_json::Value::as_str),
            anchor.get("featureId").and_then(serde_json::Value::as_str),
            bounds.0,
            bounds.1,
            bounds.2,
            bounds.3,
            location.pointer("/validity/from").filter(|value| !value.is_null()).map(ToString::to_string),
            location.pointer("/validity/to").filter(|value| !value.is_null()).map(ToString::to_string),
            resolution,
            geometry
        ],
    )?;
    Ok(())
}

pub(super) fn bounds_for_points(
    points: Option<&Vec<serde_json::Value>>,
) -> (Option<f64>, Option<f64>, Option<f64>, Option<f64>) {
    let points = points.into_iter().flatten().collect::<Vec<_>>();
    let coordinates = points
        .iter()
        .filter_map(|point| Some((point.get(0)?.as_f64()?, point.get(1)?.as_f64()?)))
        .collect::<Vec<_>>();
    if coordinates.is_empty() {
        return (None, None, None, None);
    }
    let (mut min_x, mut min_y, mut max_x, mut max_y) = (
        coordinates[0].0,
        coordinates[0].1,
        coordinates[0].0,
        coordinates[0].1,
    );
    for (x, y) in coordinates {
        min_x = min_x.min(x);
        min_y = min_y.min(y);
        max_x = max_x.max(x);
        max_y = max_y.max(y);
    }
    (Some(min_x), Some(min_y), Some(max_x), Some(max_y))
}
