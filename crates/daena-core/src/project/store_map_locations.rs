// Map location projection operations.
use super::*;

impl ProjectStore {
    pub(crate) fn ensure_map_location_projection_schema(&self) -> Result<(), CoreError> {
        let mut statement = self
            .connection
            .prepare("PRAGMA table_info(map_location_projection)")?;
        let columns = statement
            .query_map([], |row| row.get::<_, String>(1))?
            .collect::<Result<Vec<_>, _>>()?;
        if !columns.iter().any(|name| name == "geometry") {
            self.connection.execute(
                "ALTER TABLE map_location_projection ADD COLUMN geometry TEXT",
                [],
            )?;
        }
        self.connection.execute_batch(
            "CREATE INDEX IF NOT EXISTS map_location_bbox_idx ON map_location_projection(map_entity_id, min_x, max_x, min_y, max_y);
             CREATE TABLE IF NOT EXISTS map_feature_projection (
                 map_entity_id TEXT NOT NULL,
                 feature_id TEXT NOT NULL,
                 layer_id TEXT NOT NULL,
                 kind TEXT NOT NULL,
                 min_x REAL NOT NULL,
                 min_y REAL NOT NULL,
                 max_x REAL NOT NULL,
                 max_y REAL NOT NULL,
                 PRIMARY KEY (map_entity_id, feature_id)
             );",
        )?;
        Ok(())
    }

    pub(crate) fn provider_feature_resolution(
        &self,
        map_entity_id: &str,
        feature_kind: Option<&str>,
        feature_id: Option<&str>,
    ) -> &'static str {
        let (Some(feature_kind), Some(feature_id)) = (feature_kind, feature_id) else {
            return "resolved";
        };
        if feature_kind != "geojson-feature" {
            return "unresolved";
        }
        let source: Option<(String, String)> = self.connection.query_row(
            "SELECT a.path,a.content_hash FROM entity_fields f JOIN assets a ON a.id=json_extract(f.value, '$.sourceAssetId') WHERE f.entity_id=?1 AND f.namespace=?2 AND f.key='map'",
            rusqlite::params![map_entity_id, crate::maps::MAP_NAMESPACE], |row| Ok((row.get(0)?, row.get(1)?))).optional().ok().flatten();
        let Some((source_path, content_hash)) = source else {
            return "unresolved";
        };
        let source_path = self
            .root
            .as_ref()
            .and_then(|root| runtime_asset_path(root, &content_hash).ok())
            .unwrap_or_else(|| PathBuf::from(source_path));
        let Ok(bytes) = std::fs::read(source_path) else {
            return "unresolved";
        };
        if crate::maps::vector::contains_feature_id(&bytes, feature_id) {
            "resolved"
        } else {
            "unresolved"
        }
    }

    pub(crate) fn rebuild_maps_projection(&self) -> Result<(), CoreError> {
        let transaction = self.connection.unchecked_transaction()?;
        transaction.execute_batch(
            "DELETE FROM map_projection;
             DELETE FROM map_location_projection;
             DELETE FROM map_feature_projection;
             DELETE FROM map_feature_search;
             DELETE FROM world_search WHERE source_key LIKE 'map-feature:%';",
        )?;
        {
            let mut maps = transaction.prepare("SELECT e.id, json_extract(f.value, '$.provider.id'), json_extract(f.value, '$.sourceAssetId'), a.path, a.content_hash, authored.content_hash FROM entities e JOIN entity_fields f ON f.entity_id=e.id AND f.namespace=?1 AND f.key='map' LEFT JOIN assets a ON a.id=json_extract(f.value, '$.sourceAssetId') LEFT JOIN assets authored ON authored.id=json_extract(f.value, '$.authoredSourceAssetId') WHERE e.entity_type='daena.maps:world-map' AND e.deleted=0")?;
            let rows = maps.query_map([crate::maps::MAP_NAMESPACE], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, Option<String>>(1)?.unwrap_or_default(),
                    row.get::<_, Option<String>>(2)?.unwrap_or_default(),
                    row.get::<_, Option<String>>(3)?,
                    row.get::<_, Option<String>>(4)?,
                    row.get::<_, Option<String>>(5)?,
                ))
            })?;
            for row in rows {
                let (id, provider, asset, path, hash, authored_hash) = row?;
                transaction.execute("INSERT INTO map_projection(map_entity_id,provider,source_asset_id,source_path,source_hash) VALUES (?1,?2,?3,?4,?5)", rusqlite::params![id, provider, asset, path, hash])?;
                write_vector_feature_projection(
                    &transaction,
                    self.root.as_deref(),
                    &id,
                    &provider,
                    authored_hash.as_deref().or(hash.as_deref()),
                )?;
            }
        }
        {
            let mut locations = transaction.prepare("SELECT f.entity_id, json_each.value FROM entity_fields f, json_each(json_extract(f.value, '$.locations')) WHERE f.namespace=?1 AND f.key='locations'")?;
            let rows = locations.query_map([crate::maps::MAP_NAMESPACE], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })?;
            for row in rows {
                let (entity_id, raw) = row?;
                let location: serde_json::Value = serde_json::from_str(&raw)
                    .map_err(|e| CoreError::Serialization(e.to_string()))?;
                let anchor = location.get("anchor").cloned().unwrap_or_default();
                let kind = anchor
                    .get("kind")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or("unknown");
                let resolution = if kind == "provider-feature" {
                    self.provider_feature_resolution(
                        location
                            .get("mapEntityId")
                            .and_then(serde_json::Value::as_str)
                            .unwrap_or_default(),
                        anchor
                            .get("featureKind")
                            .and_then(serde_json::Value::as_str),
                        anchor.get("featureId").and_then(serde_json::Value::as_str),
                    )
                } else {
                    "resolved"
                };
                write_location_projection(&transaction, &entity_id, &location, resolution)?;
            }
        }
        transaction.commit()?;
        Ok(())
    }

    pub(crate) fn refresh_maps_projection_for_entities(
        &self,
        entity_ids: &[String],
    ) -> Result<(), CoreError> {
        let ids = entity_ids.iter().collect::<BTreeSet<_>>();
        for entity_id in &ids {
            self.connection.execute(
                "DELETE FROM map_projection WHERE map_entity_id=?1",
                params![entity_id],
            )?;
            self.connection.execute(
                "DELETE FROM map_location_projection WHERE entity_id=?1 OR map_entity_id=?1",
                params![entity_id],
            )?;
            self.connection.execute(
                "DELETE FROM map_feature_projection WHERE map_entity_id=?1",
                params![entity_id],
            )?;
            self.connection.execute(
                "DELETE FROM map_feature_search WHERE map_entity_id=?1",
                params![entity_id],
            )?;
            self.connection.execute(
                "DELETE FROM world_search WHERE entity_id=?1 AND source_key LIKE 'map-feature:%'",
                params![entity_id],
            )?;
            let map = self.connection.query_row(
                "SELECT e.id,json_extract(f.value,'$.provider.id'),json_extract(f.value,'$.sourceAssetId'),a.path,a.content_hash,authored.content_hash FROM entities e JOIN entity_fields f ON f.entity_id=e.id AND f.namespace=?1 AND f.key='map' LEFT JOIN assets a ON a.id=json_extract(f.value,'$.sourceAssetId') LEFT JOIN assets authored ON authored.id=json_extract(f.value,'$.authoredSourceAssetId') WHERE e.id=?2 AND e.entity_type='daena.maps:world-map' AND e.deleted=0",
                params![crate::maps::MAP_NAMESPACE, entity_id],
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, Option<String>>(1)?.unwrap_or_default(), row.get::<_, Option<String>>(2)?.unwrap_or_default(), row.get::<_, Option<String>>(3)?, row.get::<_, Option<String>>(4)?, row.get::<_, Option<String>>(5)?)),
            ).optional()?;
            if let Some((id, provider, asset, path, hash, authored_hash)) = map {
                self.connection.execute(
                    "INSERT INTO map_projection(map_entity_id,provider,source_asset_id,source_path,source_hash) VALUES (?1,?2,?3,?4,?5)",
                    params![id, provider, asset, path, hash],
                )?;
                write_vector_feature_projection(
                    &self.connection,
                    self.root.as_deref(),
                    &id,
                    &provider,
                    authored_hash.as_deref().or(hash.as_deref()),
                )?;
            }
            let mut location_owners = BTreeSet::from([(*entity_id).clone()]);
            let referenced_owners = self
                .connection
                .prepare("SELECT DISTINCT f.entity_id FROM entity_fields f,json_each(json_extract(f.value,'$.locations')) WHERE f.namespace=?1 AND f.key='locations' AND json_extract(json_each.value,'$.mapEntityId')=?2")?
                .query_map(params![crate::maps::MAP_NAMESPACE, entity_id], |row| {
                    row.get::<_, String>(0)
                })?
                .collect::<Result<Vec<_>, _>>()?;
            location_owners.extend(referenced_owners);
            let mut locations = self.connection.prepare(
                "SELECT f.entity_id,json_each.value FROM entity_fields f,json_each(json_extract(f.value,'$.locations')) WHERE f.entity_id=?1 AND f.namespace=?2 AND f.key='locations'",
            )?;
            for owner in location_owners {
                let locations = locations
                    .query_map(params![owner, crate::maps::MAP_NAMESPACE], |row| {
                        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
                    })?
                    .collect::<Result<Vec<_>, _>>()?;
                for (owner, raw) in locations {
                    self.insert_map_location_projection(&owner, &raw)?;
                }
            }
        }
        Ok(())
    }

    pub(crate) fn insert_map_location_projection(
        &self,
        entity_id: &str,
        raw: &str,
    ) -> Result<(), CoreError> {
        let location: serde_json::Value = serde_json::from_str(raw)
            .map_err(|error| CoreError::Serialization(error.to_string()))?;
        let anchor = location.get("anchor").cloned().unwrap_or_default();
        let kind = anchor
            .get("kind")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("unknown");
        let resolution = if kind == "provider-feature" {
            self.provider_feature_resolution(
                location
                    .get("mapEntityId")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or_default(),
                anchor
                    .get("featureKind")
                    .and_then(serde_json::Value::as_str),
                anchor.get("featureId").and_then(serde_json::Value::as_str),
            )
        } else {
            "resolved"
        };
        write_location_projection(&self.connection, entity_id, &location, resolution)
    }

    pub fn map_locations_for_entity(
        &self,
        entity_id: String,
    ) -> Result<Vec<serde_json::Value>, CoreError> {
        let mut statement = self.connection.prepare(&format!(
            "{LOCATION_PROJECTION_SELECT} WHERE entity_id=?1 ORDER BY location_id"
        ))?;
        let rows = statement.query_map(rusqlite::params![entity_id], location_projection_json)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(CoreError::from)
    }

    /// Returns the disposable projection rows for all locations on a map.
    /// The projection carries provider selectors, bounds, validity, geometry,
    /// and resolution state; it is rebuilt from the canonical `locations` fields
    /// and the source asset bytes and is never treated as durable state.
    pub fn map_location_projection(
        &self,
        map_entity_id: String,
    ) -> Result<Vec<serde_json::Value>, CoreError> {
        let mut statement = self.connection.prepare(&format!(
            "{LOCATION_PROJECTION_SELECT} WHERE map_entity_id=?1 ORDER BY location_id"
        ))?;
        let rows =
            statement.query_map(rusqlite::params![map_entity_id], location_projection_json)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(CoreError::from)
    }

    /// Returns locations whose stored bounding boxes overlap the query rectangle.
    /// Coordinates are normalized to `[0, 1]`. This reads disposable projection
    /// rows only; canonical geometry remains on entity `locations` fields.
    pub fn query_map_locations(
        &self,
        map_entity_id: String,
        min_x: f64,
        min_y: f64,
        max_x: f64,
        max_y: f64,
    ) -> Result<Vec<serde_json::Value>, CoreError> {
        if ![min_x, min_y, max_x, max_y].into_iter().all(f64::is_finite)
            || min_x > max_x
            || min_y > max_y
        {
            return Err(CoreError::Validation(
                "maps: spatial query bounds must be finite with min ≤ max".into(),
            ));
        }
        let mut statement = self.connection.prepare(&format!(
            "{LOCATION_PROJECTION_SELECT} WHERE map_entity_id=?1 AND min_x IS NOT NULL AND max_x IS NOT NULL AND min_y IS NOT NULL AND max_y IS NOT NULL AND min_x <= ?4 AND max_x >= ?2 AND min_y <= ?5 AND max_y >= ?3 ORDER BY location_id"
        ))?;
        let rows = statement.query_map(
            rusqlite::params![map_entity_id, min_x, min_y, max_x, max_y],
            location_projection_json,
        )?;
        rows.collect::<Result<Vec<_>, _>>().map_err(CoreError::from)
    }

    /// Rebuilds the disposable map projections so provider-feature resolution
    /// reflects the current source asset bytes, then returns per-location
    /// resolution results for the given map. Called after every source save so
    /// removed GeoJSON features surface as `unresolved` immediately rather than
    /// on the next full index build.
    pub fn reconcile_map_links(
        &self,
        map_entity_id: String,
    ) -> Result<Vec<serde_json::Value>, CoreError> {
        self.rebuild_maps_projection()?;
        let mut statement = self.connection.prepare("SELECT location_id,resolution FROM map_location_projection WHERE map_entity_id=?1 ORDER BY location_id")?;
        let rows = statement.query_map(rusqlite::params![map_entity_id], |row| {
            Ok(serde_json::json!({"locationId": row.get::<_, String>(0)?, "resolved": row.get::<_, String>(1)? == "resolved"}))
        })?;
        rows.collect::<Result<Vec<_>, _>>().map_err(CoreError::from)
    }

    /// Returns the canonical location references owned by an entity.  The
    /// disposable projection is intentionally not used for mutation: the
    /// JSON field remains the source of truth and is rewritten atomically.
    pub fn map_locations(
        &self,
        entity_id: String,
    ) -> Result<Vec<crate::maps::LocationReference>, CoreError> {
        let field = self.list_fields(entity_id)?.into_iter().find(|field| {
            field.namespace == crate::maps::MAP_NAMESPACE && field.key == "locations"
        });
        let Some(field) = field else {
            return Ok(Vec::new());
        };
        let object = field
            .value
            .as_object()
            .ok_or_else(|| CoreError::Serialization("maps.locations is not an object".into()))?;
        let locations = object
            .get("locations")
            .and_then(serde_json::Value::as_array)
            .ok_or_else(|| CoreError::Serialization("maps.locations is not an array".into()))?;
        locations
            .iter()
            .cloned()
            .map(|value| {
                serde_json::from_value(value)
                    .map_err(|error| CoreError::Serialization(error.to_string()))
            })
            .collect()
    }

    pub fn upsert_map_location(
        &self,
        entity_id: String,
        location: crate::maps::LocationReference,
        request_id: Option<&str>,
    ) -> Result<(), CoreError> {
        let mut locations = self.map_locations(entity_id.clone())?;
        if let Some(existing) = locations.iter_mut().find(|item| item.id == location.id) {
            *existing = location;
        } else {
            locations.push(location);
        }
        self.set_field_with_request(
            FieldValue {
                entity_id,
                namespace: crate::maps::MAP_NAMESPACE.into(),
                key: "locations".into(),
                value: serde_json::json!({"schemaVersion": 1, "locations": locations}),
                revision: String::new(),
            },
            request_id,
        )
    }

    pub fn unlink_map_location(
        &self,
        entity_id: String,
        location_id: String,
        request_id: Option<&str>,
    ) -> Result<(), CoreError> {
        let mut locations = self.map_locations(entity_id.clone())?;
        let before = locations.len();
        locations.retain(|location| location.id != location_id);
        if locations.len() == before {
            return Err(CoreError::NotFound("map location not found".into()));
        }
        self.set_field_with_request(
            FieldValue {
                entity_id,
                namespace: crate::maps::MAP_NAMESPACE.into(),
                key: "locations".into(),
                value: serde_json::json!({"schemaVersion": 1, "locations": locations}),
                revision: String::new(),
            },
            request_id,
        )
    }
}
