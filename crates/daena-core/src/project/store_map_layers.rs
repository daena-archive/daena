// Map layer operations.
use super::*;

impl ProjectStore {
    pub fn create_raster_layer(
        &self,
        map_entity_id: String,
        name: String,
        expected_revision: &str,
        request_id: Option<&str>,
    ) -> Result<RasterLayerChange, CoreError> {
        let input_fingerprint = self.layer_mutation_fingerprint(
            "create",
            &map_entity_id,
            None,
            Some(&name),
            expected_revision,
            None,
        )?;
        if let Some(change) = self.committed_mutation_with_fingerprint::<RasterLayerChange>(
            request_id,
            Some(&input_fingerprint),
        )? {
            return Ok(change);
        }
        let (width, height) = self.map_source_dimensions(&map_entity_id)?;
        let mut layers_field = self.layers_field(&map_entity_id)?;
        Self::ensure_expected_revision(
            Some(expected_revision),
            layers_field.revision.clone(),
            "field",
        )?;
        let layers = layers_field
            .value
            .get("layers")
            .and_then(serde_json::Value::as_array)
            .cloned()
            .unwrap_or_default();
        let raster_count = layers
            .iter()
            .filter(|layer| layer.get("kind").and_then(serde_json::Value::as_str) == Some("raster"))
            .count();
        if raster_count >= crate::maps::IMAGE_MAX_RASTER_LAYERS {
            return Err(CoreError::Validation(format!(
                "maps: raster layer count exceeds the budget of {}",
                crate::maps::IMAGE_MAX_RASTER_LAYERS
            )));
        }
        let png = crate::maps::encode_transparent_png(width, height)?;
        let content_hash = crate::maps::image::content_hash(&png);
        let size = png.len() as i64;
        if let Some(root) = self.root.as_deref() {
            store_runtime_asset(root, png.as_slice(), Some(&content_hash))?;
        }
        let layer_id = Uuid::new_v4().to_string();
        let asset_id = Uuid::new_v4().to_string();
        let now = chrono_like_now();
        let relative_path = format!("assets/maps/{asset_id}-layer.png");
        let next_order = layers
            .iter()
            .filter_map(|layer| layer.get("order").and_then(serde_json::Value::as_i64))
            .max()
            .map_or(0, |order| order + 1);
        let mut layers = layers;
        layers.push(serde_json::json!({
            "id": layer_id,
            "name": name,
            "order": next_order,
            "defaultVisible": true,
            "style": {},
            "selector": {},
            "kind": "raster",
            "rasterAssetId": asset_id,
            "opacity": 1.0,
            "locked": false,
            "blendMode": "normal"
        }));
        let layers_value = serde_json::json!({"schemaVersion": crate::maps::MAP_LAYERS_SCHEMA_VERSION, "layers": layers});
        let asset = Asset {
            id: asset_id.clone(),
            entity_id: map_entity_id.clone(),
            namespace: crate::maps::MAP_NAMESPACE.into(),
            filename: "layer.png".into(),
            content_hash: content_hash.clone(),
            size,
            mime_type: "image/png".into(),
            path: relative_path.clone(),
            created_at: now.clone(),
            role: ASSET_ROLE_ATTACHMENT.into(),
            reference_scope: ASSET_REFERENCE_SCOPE_ENTITY.into(),
            provenance: None,
            revision: String::new(),
        };
        let request_id = self.request_id(request_id)?;
        let result = serde_json::to_value(&RasterLayerChange {
            layer_id: layer_id.clone(),
            asset: Some(asset.clone()),
            layers: FieldValue {
                entity_id: map_entity_id.clone(),
                namespace: crate::maps::MAP_NAMESPACE.into(),
                key: "layers".into(),
                value: layers_value.clone(),
                revision: String::new(),
            },
        })
        .map_err(|error| CoreError::Serialization(error.to_string()))?;
        let transaction = self.begin_mutation_with_fingerprint(
            &request_id,
            Some(&result),
            &[format!("entities/{map_entity_id}/"), relative_path.clone()],
            &input_fingerprint,
        )?;
        transaction.execute(
            "INSERT INTO assets(id,entity_id,namespace,filename,content_hash,size,mime_type,path,created_at) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9)",
            params![asset_id, map_entity_id, crate::maps::MAP_NAMESPACE, "layer.png", content_hash, size, "image/png", relative_path, now],
        )?;
        crate::maps::validate_field(&transaction, &map_entity_id, "layers", &layers_value)?;
        transaction.execute(
            "INSERT INTO entity_fields(entity_id,namespace,key,value) VALUES (?1,?2,?3,?4) ON CONFLICT(entity_id,namespace,key) DO UPDATE SET value=excluded.value",
            params![map_entity_id, crate::maps::MAP_NAMESPACE, "layers", encode_field_value(&layers_value)?],
        )?;
        transaction.commit()?;
        self.refresh_maps_projection_for_entities(std::slice::from_ref(&map_entity_id))?;
        self.notify_export_worker()?;
        layers_field.value = layers_value;
        layers_field.revision = self.revision_for_field(&layers_field)?;
        let mut asset = asset;
        asset.revision = self.revision_for_asset(&asset.id)?;
        let change = RasterLayerChange {
            layer_id,
            asset: Some(asset),
            layers: layers_field,
        };
        self.write_mutation_result(
            &request_id,
            &serde_json::to_value(&change)
                .map_err(|error| CoreError::Serialization(error.to_string()))?,
        )?;
        Ok(change)
    }

    pub fn create_semantic_layer(
        &self,
        map_entity_id: String,
        name: String,
        expected_revision: &str,
        request_id: Option<&str>,
        style: Option<serde_json::Value>,
        selector: Option<serde_json::Value>,
    ) -> Result<RasterLayerChange, CoreError> {
        let input_fingerprint = self.layer_mutation_fingerprint(
            "create-semantic",
            &map_entity_id,
            None,
            Some(&name),
            expected_revision,
            None,
        )?;
        if let Some(change) = self.committed_mutation_with_fingerprint::<RasterLayerChange>(
            request_id,
            Some(&input_fingerprint),
        )? {
            return Ok(change);
        }
        let mut layers_field = self.layers_field(&map_entity_id)?;
        Self::ensure_expected_revision(
            Some(expected_revision),
            layers_field.revision.clone(),
            "field",
        )?;
        let layers = layers_field
            .value
            .get("layers")
            .and_then(serde_json::Value::as_array)
            .cloned()
            .unwrap_or_default();
        let semantic_count = layers
            .iter()
            .filter(|layer| layer.get("kind").and_then(serde_json::Value::as_str) != Some("raster"))
            .count();
        if semantic_count >= crate::maps::IMAGE_MAX_SEMANTIC_LAYERS {
            return Err(CoreError::Validation(format!(
                "maps: semantic layer count exceeds the budget of {}",
                crate::maps::IMAGE_MAX_SEMANTIC_LAYERS
            )));
        }
        let layer_id = Uuid::new_v4().to_string();
        let next_order = layers
            .iter()
            .filter_map(|layer| layer.get("order").and_then(serde_json::Value::as_i64))
            .max()
            .map_or(0, |order| order + 1);
        let mut layers = layers;
        layers.push(serde_json::json!({
            "id": layer_id,
            "name": name,
            "order": next_order,
            "defaultVisible": true,
            "style": style.unwrap_or_else(|| serde_json::json!({
                "stroke": "#d5ab6c",
                "fill": "rgba(213,171,108,0.25)",
                "strokeWidth": 2
            })),
            "selector": selector.unwrap_or_else(|| serde_json::json!({})),
            "kind": "semantic"
        }));
        let layers_value = serde_json::json!({"schemaVersion": crate::maps::MAP_LAYERS_SCHEMA_VERSION, "layers": layers});
        let request_id = self.request_id(request_id)?;
        let result = serde_json::to_value(&RasterLayerChange {
            layer_id: layer_id.clone(),
            asset: None,
            layers: FieldValue {
                entity_id: map_entity_id.clone(),
                namespace: crate::maps::MAP_NAMESPACE.into(),
                key: "layers".into(),
                value: layers_value.clone(),
                revision: String::new(),
            },
        })
        .map_err(|error| CoreError::Serialization(error.to_string()))?;
        let transaction = self.begin_mutation_with_fingerprint(
            &request_id,
            Some(&result),
            &[format!("entities/{map_entity_id}/")],
            &input_fingerprint,
        )?;
        crate::maps::validate_field(&transaction, &map_entity_id, "layers", &layers_value)?;
        transaction.execute(
            "INSERT INTO entity_fields(entity_id,namespace,key,value) VALUES (?1,?2,?3,?4) ON CONFLICT(entity_id,namespace,key) DO UPDATE SET value=excluded.value",
            params![map_entity_id, crate::maps::MAP_NAMESPACE, "layers", encode_field_value(&layers_value)?],
        )?;
        transaction.commit()?;
        self.refresh_maps_projection_for_entities(std::slice::from_ref(&map_entity_id))?;
        self.notify_export_worker()?;
        layers_field.value = layers_value;
        layers_field.revision = self.revision_for_field(&layers_field)?;
        let change = RasterLayerChange {
            layer_id,
            asset: None,
            layers: layers_field,
        };
        self.write_mutation_result(
            &request_id,
            &serde_json::to_value(&change)
                .map_err(|error| CoreError::Serialization(error.to_string()))?,
        )?;
        Ok(change)
    }

    pub fn create_vector_layer(
        &self,
        map_entity_id: String,
        name: String,
        expected_revision: &str,
        request_id: Option<&str>,
        style: Option<serde_json::Value>,
    ) -> Result<RasterLayerChange, CoreError> {
        let input_fingerprint = self.layer_mutation_fingerprint(
            "create-vector",
            &map_entity_id,
            None,
            Some(&name),
            expected_revision,
            None,
        )?;
        if let Some(change) = self.committed_mutation_with_fingerprint::<RasterLayerChange>(
            request_id,
            Some(&input_fingerprint),
        )? {
            return Ok(change);
        }
        let mut layers_field = self.layers_field(&map_entity_id)?;
        Self::ensure_expected_revision(
            Some(expected_revision),
            layers_field.revision.clone(),
            "field",
        )?;
        let layers = layers_field
            .value
            .get("layers")
            .and_then(serde_json::Value::as_array)
            .cloned()
            .unwrap_or_default();
        let vector_count = layers
            .iter()
            .filter(|layer| layer.get("kind").and_then(serde_json::Value::as_str) == Some("vector"))
            .count();
        if vector_count >= crate::maps::VECTOR_MAX_LAYERS {
            return Err(crate::maps::vector::fail(
                crate::maps::vector::CODE_LIMIT,
                "layers",
                format!(
                    "vector layer count exceeds {}",
                    crate::maps::VECTOR_MAX_LAYERS
                ),
            ));
        }
        let layer_id = Uuid::new_v4().to_string();
        let next_order = layers
            .iter()
            .filter_map(|layer| layer.get("order").and_then(serde_json::Value::as_i64))
            .max()
            .map_or(0, |order| order + 1);
        let style = style.unwrap_or_else(|| {
            serde_json::json!({
                "fill": "#8f6fd1",
                "fillOpacity": 0.35,
                "stroke": "#5e4893",
                "strokeWidth": 1.5,
                "pointRadius": 5
            })
        });
        crate::maps::vector::validate_vector_style(&style)?;
        let mut layers = layers;
        layers.push(serde_json::json!({
            "id": layer_id,
            "name": name,
            "order": next_order,
            "defaultVisible": true,
            "locked": false,
            "opacity": 1.0,
            "blendMode": "normal",
            "selector": {},
            "style": style,
            "kind": "vector"
        }));
        let layers_value = serde_json::json!({"schemaVersion": crate::maps::MAP_LAYERS_SCHEMA_VERSION, "layers": layers});
        let request_id = self.request_id(request_id)?;
        let result = serde_json::to_value(&RasterLayerChange {
            layer_id: layer_id.clone(),
            asset: None,
            layers: FieldValue {
                entity_id: map_entity_id.clone(),
                namespace: crate::maps::MAP_NAMESPACE.into(),
                key: "layers".into(),
                value: layers_value.clone(),
                revision: String::new(),
            },
        })
        .map_err(|error| CoreError::Serialization(error.to_string()))?;
        let transaction = self.begin_mutation_with_fingerprint(
            &request_id,
            Some(&result),
            &[format!("entities/{map_entity_id}/")],
            &input_fingerprint,
        )?;
        crate::maps::validate_field(&transaction, &map_entity_id, "layers", &layers_value)?;
        transaction.execute(
            "INSERT INTO entity_fields(entity_id,namespace,key,value) VALUES (?1,?2,?3,?4) ON CONFLICT(entity_id,namespace,key) DO UPDATE SET value=excluded.value",
            params![map_entity_id, crate::maps::MAP_NAMESPACE, "layers", encode_field_value(&layers_value)?],
        )?;
        transaction.commit()?;
        self.refresh_maps_projection_for_entities(std::slice::from_ref(&map_entity_id))?;
        self.notify_export_worker()?;
        layers_field.value = layers_value;
        layers_field.revision = self.revision_for_field(&layers_field)?;
        let change = RasterLayerChange {
            layer_id,
            asset: None,
            layers: layers_field,
        };
        self.write_mutation_result(
            &request_id,
            &serde_json::to_value(&change)
                .map_err(|error| CoreError::Serialization(error.to_string()))?,
        )?;
        Ok(change)
    }

    pub fn delete_vector_layer(
        &self,
        map_entity_id: String,
        layer_id: String,
        expected_revision: &str,
        expected_source_revision: &str,
        expected_feature_count: i64,
        request_id: Option<&str>,
    ) -> Result<VectorLayerDelete, CoreError> {
        let input_fingerprint = digest_bytes(
            &serde_json::to_vec(&serde_json::json!({
                "op": "delete-vector",
                "mapEntityId": map_entity_id,
                "layerId": layer_id,
                "expectedRevision": expected_revision,
                "expectedSourceRevision": expected_source_revision,
                "expectedFeatureCount": expected_feature_count,
            }))
            .map_err(|error| CoreError::Serialization(error.to_string()))?,
        );
        if let Some(change) = self.committed_mutation_with_fingerprint::<VectorLayerDelete>(
            request_id,
            Some(&input_fingerprint),
        )? {
            return Ok(change);
        }
        if expected_feature_count < 0 {
            return Err(crate::maps::vector::fail(
                crate::maps::vector::CODE_SOURCE_INVALID,
                "expectedFeatureCount",
                "must be a non-negative integer",
            ));
        }
        let mut layers_field = self.layers_field(&map_entity_id)?;
        Self::ensure_expected_revision(
            Some(expected_revision),
            layers_field.revision.clone(),
            "field",
        )?;
        let descriptor = self
            .list_fields_unchecked(map_entity_id.clone())?
            .into_iter()
            .find(|field| field.namespace == crate::maps::MAP_NAMESPACE && field.key == "map")
            .ok_or_else(|| CoreError::NotFound("map descriptor not found".into()))?;
        let source_id = descriptor
            .value
            .get("authoredSourceAssetId")
            .or_else(|| descriptor.value.get("sourceAssetId"))
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| CoreError::NotFound("vector source asset not found".into()))?
            .to_owned();
        let mut source = self.asset_unchecked(&source_id)?;
        Self::ensure_expected_revision(
            Some(expected_source_revision),
            source.revision.clone(),
            "asset",
        )?;
        let layers = layers_field
            .value
            .get("layers")
            .and_then(serde_json::Value::as_array)
            .cloned()
            .unwrap_or_default();
        let Some(removed) = layers.iter().find(|layer| {
            layer.get("id").and_then(serde_json::Value::as_str) == Some(layer_id.as_str())
        }) else {
            return Err(crate::maps::vector::fail(
                crate::maps::vector::CODE_LAYER_MISSING,
                "layerId",
                "vector layer not found",
            ));
        };
        if removed.get("kind").and_then(serde_json::Value::as_str) != Some("vector") {
            return Err(CoreError::Validation(
                "maps: only vector layers can be deleted by this operation".into(),
            ));
        }
        if descriptor
            .value
            .pointer("/provider/id")
            .and_then(serde_json::Value::as_str)
            == Some(crate::maps::PHYSICAL_PROVIDER)
            && removed
                .get("locked")
                .and_then(serde_json::Value::as_bool)
                .unwrap_or(false)
        {
            return Err(CoreError::Validation(
                "maps: physical layers are immutable".into(),
            ));
        }
        let remaining: Vec<_> = layers
            .into_iter()
            .filter(|layer| {
                layer.get("id").and_then(serde_json::Value::as_str) != Some(layer_id.as_str())
            })
            .collect();
        let layers_value = serde_json::json!({"schemaVersion": crate::maps::MAP_LAYERS_SCHEMA_VERSION, "layers": remaining});
        let known = crate::maps::vector::layer_ids_from_layers_field(&layers_value);
        let vector_space =
            crate::maps::vector::VectorSpace::from_descriptor_value(&descriptor.value);
        let bytes = self.read_asset_bytes(&source)?;
        let (canonical, deleted) = crate::maps::vector::remove_layer_features_with_space(
            &bytes,
            &layer_id,
            &known,
            &vector_space,
        )?;
        if deleted as i64 != expected_feature_count {
            return Err(CoreError::Conflict(
                "vector.layer.in-use: expectedFeatureCount does not match the source".into(),
            ));
        }
        let content_hash = format!("sha256:{:x}", Sha256::digest(&canonical));
        let size = canonical.len() as i64;
        if let Some(root) = self.root.as_deref() {
            store_runtime_asset(root, canonical.as_slice(), Some(&content_hash))?;
        }
        let request_id = self.request_id(request_id)?;
        let result = serde_json::to_value(&VectorLayerDelete {
            layers: FieldValue {
                entity_id: map_entity_id.clone(),
                namespace: crate::maps::MAP_NAMESPACE.into(),
                key: "layers".into(),
                value: layers_value.clone(),
                revision: String::new(),
            },
            source: Asset {
                content_hash: content_hash.clone(),
                size,
                revision: String::new(),
                ..source.clone()
            },
            deleted_feature_count: deleted,
        })
        .map_err(|error| CoreError::Serialization(error.to_string()))?;
        let transaction = self.begin_mutation_with_fingerprint(
            &request_id,
            Some(&result),
            &[format!("entities/{map_entity_id}/"), source.path.clone()],
            &input_fingerprint,
        )?;
        crate::maps::validate_field(&transaction, &map_entity_id, "layers", &layers_value)?;
        transaction.execute(
            "UPDATE assets SET content_hash=?1,size=?2 WHERE id=?3",
            params![content_hash, size, source.id],
        )?;
        transaction.execute(
            "INSERT INTO entity_fields(entity_id,namespace,key,value) VALUES (?1,?2,?3,?4) ON CONFLICT(entity_id,namespace,key) DO UPDATE SET value=excluded.value",
            params![map_entity_id, crate::maps::MAP_NAMESPACE, "layers", encode_field_value(&layers_value)?],
        )?;
        transaction.commit()?;
        self.refresh_maps_projection_for_entities(std::slice::from_ref(&map_entity_id))?;
        self.notify_export_worker()?;
        layers_field.value = layers_value;
        layers_field.revision = self.revision_for_field(&layers_field)?;
        source.content_hash = content_hash;
        source.size = size;
        source.revision = self.revision_for_asset(&source.id)?;
        let change = VectorLayerDelete {
            layers: layers_field,
            source,
            deleted_feature_count: deleted,
        };
        self.write_mutation_result(
            &request_id,
            &serde_json::to_value(&change)
                .map_err(|error| CoreError::Serialization(error.to_string()))?,
        )?;
        Ok(change)
    }

    pub fn map_provider_id(&self, map_entity_id: &str) -> Result<String, CoreError> {
        let descriptor = self
            .list_fields_unchecked(map_entity_id.to_owned())?
            .into_iter()
            .find(|field| field.namespace == crate::maps::MAP_NAMESPACE && field.key == "map")
            .ok_or_else(|| CoreError::NotFound("map descriptor not found".into()))?;
        descriptor
            .value
            .pointer("/provider/id")
            .and_then(serde_json::Value::as_str)
            .map(str::to_owned)
            .ok_or_else(|| CoreError::Validation("maps: provider id is required".into()))
    }

    pub fn map_layer_kind(
        &self,
        map_entity_id: &str,
        layer_id: &str,
    ) -> Result<Option<String>, CoreError> {
        let layers = self.layers_field(map_entity_id)?;
        Ok(layers
            .value
            .get("layers")
            .and_then(serde_json::Value::as_array)
            .and_then(|layers| {
                layers.iter().find(|layer| {
                    layer.get("id").and_then(serde_json::Value::as_str) == Some(layer_id)
                })
            })
            .map(|layer| {
                layer
                    .get("kind")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or("semantic")
                    .to_owned()
            }))
    }

    pub fn delete_semantic_layer(
        &self,
        map_entity_id: String,
        layer_id: String,
        expected_revision: &str,
        request_id: Option<&str>,
    ) -> Result<RasterLayerChange, CoreError> {
        let input_fingerprint = self.layer_mutation_fingerprint(
            "delete-semantic",
            &map_entity_id,
            Some(&layer_id),
            None,
            expected_revision,
            None,
        )?;
        if let Some(change) = self.committed_mutation_with_fingerprint::<RasterLayerChange>(
            request_id,
            Some(&input_fingerprint),
        )? {
            return Ok(change);
        }
        let mut layers_field = self.layers_field(&map_entity_id)?;
        Self::ensure_expected_revision(
            Some(expected_revision),
            layers_field.revision.clone(),
            "field",
        )?;
        let layers = layers_field
            .value
            .get("layers")
            .and_then(serde_json::Value::as_array)
            .cloned()
            .unwrap_or_default();
        let Some(removed) = layers.iter().find(|layer| {
            layer.get("id").and_then(serde_json::Value::as_str) == Some(layer_id.as_str())
        }) else {
            return Err(CoreError::NotFound("semantic layer not found".into()));
        };
        if removed.get("kind").and_then(serde_json::Value::as_str) == Some("raster") {
            return Err(CoreError::Validation(
                "maps: raster layers cannot be deleted by this operation".into(),
            ));
        }
        if removed.get("kind").and_then(serde_json::Value::as_str) == Some("vector") {
            return Err(CoreError::Validation(
                "maps: vector layers cannot be deleted by this operation".into(),
            ));
        }
        let remaining = layers
            .into_iter()
            .filter(|layer| {
                layer.get("id").and_then(serde_json::Value::as_str) != Some(layer_id.as_str())
            })
            .collect::<Vec<_>>();
        let layers_value = serde_json::json!({"schemaVersion": crate::maps::MAP_LAYERS_SCHEMA_VERSION, "layers": remaining});
        let request_id = self.request_id(request_id)?;
        let result = serde_json::to_value(&RasterLayerChange {
            layer_id: layer_id.clone(),
            asset: None,
            layers: FieldValue {
                entity_id: map_entity_id.clone(),
                namespace: crate::maps::MAP_NAMESPACE.into(),
                key: "layers".into(),
                value: layers_value.clone(),
                revision: String::new(),
            },
        })
        .map_err(|error| CoreError::Serialization(error.to_string()))?;
        let transaction = self.begin_mutation_with_fingerprint(
            &request_id,
            Some(&result),
            &[format!("entities/{map_entity_id}/")],
            &input_fingerprint,
        )?;
        crate::maps::validate_field(&transaction, &map_entity_id, "layers", &layers_value)?;
        transaction.execute(
            "INSERT INTO entity_fields(entity_id,namespace,key,value) VALUES (?1,?2,?3,?4) ON CONFLICT(entity_id,namespace,key) DO UPDATE SET value=excluded.value",
            params![map_entity_id, crate::maps::MAP_NAMESPACE, "layers", encode_field_value(&layers_value)?],
        )?;
        transaction.commit()?;
        self.refresh_maps_projection_for_entities(std::slice::from_ref(&map_entity_id))?;
        self.notify_export_worker()?;
        layers_field.value = layers_value;
        layers_field.revision = self.revision_for_field(&layers_field)?;
        let change = RasterLayerChange {
            layer_id,
            asset: None,
            layers: layers_field,
        };
        self.write_mutation_result(
            &request_id,
            &serde_json::to_value(&change)
                .map_err(|error| CoreError::Serialization(error.to_string()))?,
        )?;
        Ok(change)
    }

    pub fn delete_raster_layer(
        &self,
        map_entity_id: String,
        layer_id: String,
        expected_revision: &str,
        request_id: Option<&str>,
    ) -> Result<RasterLayerChange, CoreError> {
        let input_fingerprint = self.layer_mutation_fingerprint(
            "delete",
            &map_entity_id,
            Some(&layer_id),
            None,
            expected_revision,
            None,
        )?;
        if let Some(change) = self.committed_mutation_with_fingerprint::<RasterLayerChange>(
            request_id,
            Some(&input_fingerprint),
        )? {
            return Ok(change);
        }
        let mut layers_field = self.layers_field(&map_entity_id)?;
        Self::ensure_expected_revision(
            Some(expected_revision),
            layers_field.revision.clone(),
            "field",
        )?;
        let layers = layers_field
            .value
            .get("layers")
            .and_then(serde_json::Value::as_array)
            .cloned()
            .unwrap_or_default();
        let Some(removed) = layers.iter().find(|layer| {
            layer.get("id").and_then(serde_json::Value::as_str) == Some(layer_id.as_str())
        }) else {
            return Err(CoreError::NotFound("raster layer not found".into()));
        };
        if removed.get("kind").and_then(serde_json::Value::as_str) != Some("raster") {
            return Err(CoreError::Validation(
                "maps: only raster layers can be deleted by this operation".into(),
            ));
        }
        let raster_asset_id = removed
            .get("rasterAssetId")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| {
                CoreError::Validation("maps: raster layer is missing rasterAssetId".into())
            })?
            .to_owned();
        let remaining = layers
            .into_iter()
            .filter(|layer| {
                layer.get("id").and_then(serde_json::Value::as_str) != Some(layer_id.as_str())
            })
            .collect::<Vec<_>>();
        let layers_value = serde_json::json!({"schemaVersion": crate::maps::MAP_LAYERS_SCHEMA_VERSION, "layers": remaining});
        let request_id = self.request_id(request_id)?;
        let result = serde_json::to_value(&RasterLayerChange {
            layer_id: layer_id.clone(),
            asset: None,
            layers: FieldValue {
                entity_id: map_entity_id.clone(),
                namespace: crate::maps::MAP_NAMESPACE.into(),
                key: "layers".into(),
                value: layers_value.clone(),
                revision: String::new(),
            },
        })
        .map_err(|error| CoreError::Serialization(error.to_string()))?;
        let transaction = self.begin_mutation_with_fingerprint(
            &request_id,
            Some(&result),
            &[format!("entities/{map_entity_id}/")],
            &input_fingerprint,
        )?;
        transaction.execute(
            "DELETE FROM assets WHERE id=?1 AND entity_id=?2 AND namespace=?3",
            params![raster_asset_id, map_entity_id, crate::maps::MAP_NAMESPACE],
        )?;
        crate::maps::validate_field(&transaction, &map_entity_id, "layers", &layers_value)?;
        transaction.execute(
            "INSERT INTO entity_fields(entity_id,namespace,key,value) VALUES (?1,?2,?3,?4) ON CONFLICT(entity_id,namespace,key) DO UPDATE SET value=excluded.value",
            params![map_entity_id, crate::maps::MAP_NAMESPACE, "layers", encode_field_value(&layers_value)?],
        )?;
        transaction.commit()?;
        self.refresh_maps_projection_for_entities(std::slice::from_ref(&map_entity_id))?;
        self.notify_export_worker()?;
        layers_field.value = layers_value;
        layers_field.revision = self.revision_for_field(&layers_field)?;
        let change = RasterLayerChange {
            layer_id,
            asset: None,
            layers: layers_field,
        };
        self.write_mutation_result(
            &request_id,
            &serde_json::to_value(&change)
                .map_err(|error| CoreError::Serialization(error.to_string()))?,
        )?;
        Ok(change)
    }

    pub fn update_map_layer(
        &self,
        map_entity_id: String,
        layer_id: String,
        update: RasterLayerUpdate,
        expected_revision: &str,
        request_id: Option<&str>,
    ) -> Result<RasterLayerChange, CoreError> {
        let input_fingerprint = self.layer_mutation_fingerprint(
            "update",
            &map_entity_id,
            Some(&layer_id),
            None,
            expected_revision,
            Some(&update),
        )?;
        if let Some(change) = self.committed_mutation_with_fingerprint::<RasterLayerChange>(
            request_id,
            Some(&input_fingerprint),
        )? {
            return Ok(change);
        }
        let mut layers_field = self.layers_field(&map_entity_id)?;
        Self::ensure_expected_revision(
            Some(expected_revision),
            layers_field.revision.clone(),
            "field",
        )?;
        let mut layers = layers_field
            .value
            .get("layers")
            .and_then(serde_json::Value::as_array)
            .cloned()
            .unwrap_or_default();
        let physical_map = self
            .list_fields_unchecked(map_entity_id.clone())?
            .into_iter()
            .find(|field| field.namespace == crate::maps::MAP_NAMESPACE && field.key == "map")
            .and_then(|field| {
                field
                    .value
                    .pointer("/provider/id")
                    .and_then(serde_json::Value::as_str)
                    .map(|provider| provider == crate::maps::PHYSICAL_PROVIDER)
            })
            .unwrap_or(false);
        let Some(layer) = layers.iter_mut().find(|layer| {
            layer.get("id").and_then(serde_json::Value::as_str) == Some(layer_id.as_str())
        }) else {
            return Err(CoreError::NotFound("layer not found".into()));
        };
        if physical_map
            && layer
                .get("locked")
                .and_then(serde_json::Value::as_bool)
                .unwrap_or(false)
            && (update.name.is_some()
                || update.order.is_some()
                || update.locked.is_some()
                || update.style.is_some()
                || update.selector.is_some())
        {
            return Err(CoreError::Validation(
                "maps: physical layers are immutable".into(),
            ));
        }
        let object = layer
            .as_object_mut()
            .ok_or_else(|| CoreError::Validation("maps: layer definition is invalid".into()))?;
        if let Some(name) = update.name {
            object.insert("name".into(), serde_json::Value::String(name));
        }
        if let Some(order) = update.order {
            object.insert("order".into(), serde_json::json!(order));
        }
        if let Some(default_visible) = update.default_visible {
            object.insert(
                "defaultVisible".into(),
                serde_json::Value::Bool(default_visible),
            );
        }
        if object.get("kind").and_then(serde_json::Value::as_str) == Some("raster") {
            if let Some(opacity) = update.opacity {
                object.insert("opacity".into(), serde_json::json!(opacity));
            }
            if let Some(locked) = update.locked {
                object.insert("locked".into(), serde_json::Value::Bool(locked));
            }
            if update.style.is_some() || update.selector.is_some() {
                return Err(CoreError::Validation(
                    "maps: raster layers must have empty style and selector objects".into(),
                ));
            }
        } else if object.get("kind").and_then(serde_json::Value::as_str) == Some("vector") {
            if update.opacity.is_some() || update.selector.is_some() {
                return Err(CoreError::Validation(
                    "maps: opacity and selector do not apply to vector layers".into(),
                ));
            }
            if let Some(locked) = update.locked {
                object.insert("locked".into(), serde_json::Value::Bool(locked));
            }
            if let Some(style) = update.style {
                crate::maps::vector::validate_vector_style(&style)?;
                object.insert("style".into(), style);
            }
        } else if update.opacity.is_some() || update.locked.is_some() {
            return Err(CoreError::Validation(
                "maps: opacity and locked apply only to raster or vector layers".into(),
            ));
        } else {
            if let Some(style) = update.style {
                object.insert("style".into(), style);
            }
            if let Some(selector) = update.selector {
                object.insert("selector".into(), selector);
            }
        }
        let layers_value = serde_json::json!({"schemaVersion": crate::maps::MAP_LAYERS_SCHEMA_VERSION, "layers": layers});
        let request_id = self.request_id(request_id)?;
        let result = serde_json::to_value(&RasterLayerChange {
            layer_id: layer_id.clone(),
            asset: None,
            layers: FieldValue {
                entity_id: map_entity_id.clone(),
                namespace: crate::maps::MAP_NAMESPACE.into(),
                key: "layers".into(),
                value: layers_value.clone(),
                revision: String::new(),
            },
        })
        .map_err(|error| CoreError::Serialization(error.to_string()))?;
        crate::maps::validate_field(&self.connection, &map_entity_id, "layers", &layers_value)?;
        let transaction = self.begin_mutation_with_fingerprint(
            &request_id,
            Some(&result),
            &[format!("entities/{map_entity_id}/")],
            &input_fingerprint,
        )?;
        transaction.execute(
            "INSERT INTO entity_fields(entity_id,namespace,key,value) VALUES (?1,?2,?3,?4) ON CONFLICT(entity_id,namespace,key) DO UPDATE SET value=excluded.value",
            params![map_entity_id, crate::maps::MAP_NAMESPACE, "layers", encode_field_value(&layers_value)?],
        )?;
        transaction.commit()?;
        self.refresh_maps_projection_for_entities(std::slice::from_ref(&map_entity_id))?;
        self.notify_export_worker()?;
        layers_field.value = layers_value;
        layers_field.revision = self.revision_for_field(&layers_field)?;
        let change = RasterLayerChange {
            layer_id,
            asset: None,
            layers: layers_field,
        };
        self.write_mutation_result(
            &request_id,
            &serde_json::to_value(&change)
                .map_err(|error| CoreError::Serialization(error.to_string()))?,
        )?;
        Ok(change)
    }

    pub(crate) fn layer_mutation_fingerprint(
        &self,
        op: &str,
        map_entity_id: &str,
        layer_id: Option<&str>,
        name: Option<&str>,
        expected_revision: &str,
        update: Option<&RasterLayerUpdate>,
    ) -> Result<String, CoreError> {
        Ok(digest_bytes(
            &serde_json::to_vec(&serde_json::json!({
                "op": op,
                "mapEntityId": map_entity_id,
                "layerId": layer_id,
                "name": name,
                "expectedRevision": expected_revision,
                "update": update,
            }))
            .map_err(|error| CoreError::Serialization(error.to_string()))?,
        ))
    }

    pub(crate) fn layers_field(&self, map_entity_id: &str) -> Result<FieldValue, CoreError> {
        let entity_type: Option<String> = self
            .connection
            .query_row(
                "SELECT entity_type FROM entities WHERE id=?1 AND deleted=0",
                params![map_entity_id],
                |row| row.get(0),
            )
            .optional()?;
        if entity_type.as_deref() != Some(crate::maps::MAP_ENTITY_TYPE) {
            return Err(CoreError::NotFound("map entity not found".into()));
        }
        if let Some(mut field) = self
            .list_fields_unchecked(map_entity_id.to_owned())?
            .into_iter()
            .find(|field| field.namespace == crate::maps::MAP_NAMESPACE && field.key == "layers")
        {
            field.revision = self.revision_for_field(&field)?;
            return Ok(field);
        }
        let field = FieldValue {
            entity_id: map_entity_id.to_owned(),
            namespace: crate::maps::MAP_NAMESPACE.into(),
            key: "layers".into(),
            value: serde_json::json!({"schemaVersion": crate::maps::MAP_LAYERS_SCHEMA_VERSION, "layers": []}),
            revision: String::new(),
        };
        let revision = self.revision_for_field(&field)?;
        Ok(FieldValue { revision, ..field })
    }

    pub(crate) fn map_source_dimensions(
        &self,
        map_entity_id: &str,
    ) -> Result<(u32, u32), CoreError> {
        let descriptor = self
            .list_fields_unchecked(map_entity_id.to_owned())?
            .into_iter()
            .find(|field| field.namespace == crate::maps::MAP_NAMESPACE && field.key == "map")
            .ok_or_else(|| CoreError::NotFound("map descriptor not found".into()))?;
        let preview_asset_id = descriptor
            .value
            .pointer("/previewAssetId")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| {
                CoreError::Validation("maps: imported vector maps require previewAssetId".into())
            })?;
        let asset = self.asset_unchecked(preview_asset_id)?;
        let bytes = self.read_asset_bytes(&asset)?;
        let source = crate::maps::validate_image_source(&bytes, &asset.mime_type)?;
        Ok((source.width, source.height))
    }

    pub(crate) fn raster_layer_expected_size(
        &self,
        asset: &Asset,
    ) -> Result<Option<(u32, u32)>, CoreError> {
        let Some(layers) = self
            .list_fields_unchecked(asset.entity_id.clone())?
            .into_iter()
            .find(|field| field.namespace == crate::maps::MAP_NAMESPACE && field.key == "layers")
        else {
            return Ok(None);
        };
        let is_raster = layers
            .value
            .get("layers")
            .and_then(serde_json::Value::as_array)
            .into_iter()
            .flatten()
            .any(|layer| {
                layer.get("kind").and_then(serde_json::Value::as_str) == Some("raster")
                    && layer
                        .get("rasterAssetId")
                        .and_then(serde_json::Value::as_str)
                        == Some(asset.id.as_str())
            });
        if !is_raster {
            return Ok(None);
        }
        Ok(Some(self.map_source_dimensions(&asset.entity_id)?))
    }
}
