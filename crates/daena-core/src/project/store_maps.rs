// Map accept, import, replace, and edit operations.
use super::*;

impl ProjectStore {
    pub fn import_image_map(
        &self,
        name: String,
        bytes: Vec<u8>,
        mime_type: String,
        filename: String,
        request_id: Option<&str>,
    ) -> Result<ImportedImageMap, CoreError> {
        let input_fingerprint = digest_bytes(
            &serde_json::to_vec(&serde_json::json!({
                "name": name,
                "mimeType": mime_type,
                "filename": filename,
                "contentHash": crate::maps::image::content_hash(&bytes),
            }))
            .map_err(|error| CoreError::Serialization(error.to_string()))?,
        );
        if let Some(imported) = self.committed_mutation_with_fingerprint::<ImportedImageMap>(
            request_id,
            Some(&input_fingerprint),
        )? {
            return Ok(imported);
        }
        let source = crate::maps::validate_image_source(&bytes, &mime_type)?;
        let filename = Path::new(&filename)
            .file_name()
            .and_then(|value| value.to_str())
            .filter(|value| !value.is_empty() && *value != "." && *value != "..")
            .unwrap_or(match source.source_format {
                "jpeg" => "map.jpeg",
                "svg" => "map.svg",
                _ => "map.png",
            })
            .to_owned();
        let preview_hash = crate::maps::image::content_hash(&bytes);
        let preview_size = bytes.len() as i64;
        let canonical = crate::maps::empty_canonical_bytes();
        let source_hash = format!("sha256:{:x}", Sha256::digest(&canonical));
        let source_size = canonical.len() as i64;
        if let Some(root) = self.root.as_deref() {
            store_runtime_asset(root, bytes.as_slice(), Some(&preview_hash))?;
            store_runtime_asset(root, canonical.as_slice(), Some(&source_hash))?;
        }
        let entity_id = Uuid::new_v4().to_string();
        let source_id = Uuid::new_v4().to_string();
        let preview_id = Uuid::new_v4().to_string();
        let background_id = Uuid::new_v4().to_string();
        let now = chrono_like_now();
        let source_path = format!(
            "assets/maps/{}-{}",
            Uuid::new_v4(),
            crate::maps::VECTOR_FILENAME
        );
        let preview_path = format!("assets/maps/{}-{filename}", Uuid::new_v4());
        let descriptor = serde_json::json!({
            "schemaVersion": crate::maps::MAP_DESCRIPTOR_SCHEMA_VERSION,
            "provider": {
                "id": crate::maps::VECTOR_PROVIDER,
                "adapterVersion": crate::maps::VECTOR_ADAPTER_VERSION,
                "sourceFormat": crate::maps::VECTOR_SOURCE_FORMAT
            },
            "sourceAssetId": source_id,
            "previewAssetId": preview_id,
            "coordinateSpace": {
                "kind": "image",
                "extent": [0, 0, source.width, source.height],
                "origin": "top-left",
                "units": "pixels"
            },
            "backgrounds": [{
                "id": background_id,
                "assetId": preview_id,
                "name": "Base image",
                "visible": true,
                "locked": true,
                "opacity": 1,
                "order": 0,
                "extent": [0, 0, source.width, source.height]
            }],
            "defaultView": {"center": [f64::from(source.width) / 2.0, f64::from(source.height) / 2.0], "zoom": 1, "rotation": 0},
            "settings": {"snapEnabled": true, "grid": null}
        });
        let layers = serde_json::json!({"schemaVersion": crate::maps::MAP_LAYERS_SCHEMA_VERSION, "layers": []});
        let entity = Entity {
            id: entity_id.clone(),
            name: name.trim().into(),
            entity_type: Some(crate::maps::MAP_ENTITY_TYPE.into()),
            deleted: false,
            created_at: now.clone(),
            updated_at: now.clone(),
            revision: String::new(),
        };
        let source_asset = Asset {
            id: source_id.clone(),
            entity_id: entity_id.clone(),
            namespace: crate::maps::MAP_NAMESPACE.into(),
            filename: crate::maps::VECTOR_FILENAME.into(),
            content_hash: source_hash.clone(),
            size: source_size,
            mime_type: crate::maps::VECTOR_MIME.into(),
            path: source_path.clone(),
            created_at: now.clone(),
            role: ASSET_ROLE_ATTACHMENT.into(),
            reference_scope: ASSET_REFERENCE_SCOPE_ENTITY.into(),
            provenance: None,
            revision: String::new(),
        };
        let preview_asset = Asset {
            id: preview_id.clone(),
            entity_id: entity_id.clone(),
            namespace: crate::maps::MAP_NAMESPACE.into(),
            filename: filename.clone(),
            content_hash: preview_hash.clone(),
            size: preview_size,
            mime_type: mime_type.clone(),
            path: preview_path.clone(),
            created_at: now.clone(),
            role: ASSET_ROLE_ATTACHMENT.into(),
            reference_scope: ASSET_REFERENCE_SCOPE_ENTITY.into(),
            provenance: None,
            revision: String::new(),
        };
        let imported = ImportedImageMap {
            entity: entity.clone(),
            source: source_asset.clone(),
            preview: preview_asset.clone(),
        };
        let request_id = self.request_id(request_id)?;
        let result = serde_json::to_value(&imported)
            .map_err(|error| CoreError::Serialization(error.to_string()))?;
        let transaction = self.begin_mutation_with_fingerprint(
            &request_id,
            Some(&result),
            &[
                format!("entities/{entity_id}/"),
                source_path.clone(),
                preview_path.clone(),
            ],
            &input_fingerprint,
        )?;
        transaction.execute(
            "INSERT INTO entities(id,name,entity_type,created_at,updated_at) VALUES (?1,?2,?3,?4,?4)",
            params![entity_id, entity.name, crate::maps::MAP_ENTITY_TYPE, now],
        )?;
        transaction.execute(
            "INSERT INTO assets(id,entity_id,namespace,filename,content_hash,size,mime_type,path,created_at) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9)",
            params![source_id, entity_id, crate::maps::MAP_NAMESPACE, crate::maps::VECTOR_FILENAME, source_hash, source_size, crate::maps::VECTOR_MIME, source_path, now],
        )?;
        transaction.execute(
            "INSERT INTO assets(id,entity_id,namespace,filename,content_hash,size,mime_type,path,created_at) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9)",
            params![preview_id, entity_id, crate::maps::MAP_NAMESPACE, filename, preview_hash, preview_size, mime_type, preview_path, now],
        )?;
        crate::maps::validate_field(&transaction, &entity_id, "map", &descriptor)?;
        crate::maps::validate_field(&transaction, &entity_id, "layers", &layers)?;
        transaction.execute(
            "INSERT INTO entity_fields(entity_id,namespace,key,value) VALUES (?1,?2,?3,?4)",
            params![
                entity_id,
                crate::maps::MAP_NAMESPACE,
                "map",
                encode_field_value(&descriptor)?
            ],
        )?;
        transaction.execute(
            "INSERT INTO entity_fields(entity_id,namespace,key,value) VALUES (?1,?2,?3,?4)",
            params![
                entity_id,
                crate::maps::MAP_NAMESPACE,
                "layers",
                encode_field_value(&layers)?
            ],
        )?;
        transaction.commit()?;
        self.refresh_maps_projection_for_entities(std::slice::from_ref(&imported.entity.id))?;
        self.notify_export_worker()?;
        let mut imported = imported;
        imported.entity.revision = self.revision_for_entity(&imported.entity.id)?;
        imported.source.revision = self.revision_for_asset(&imported.source.id)?;
        imported.preview.revision = self.revision_for_asset(&imported.preview.id)?;
        self.write_mutation_result(
            &request_id,
            &serde_json::to_value(&imported)
                .map_err(|error| CoreError::Serialization(error.to_string()))?,
        )?;
        Ok(imported)
    }

    pub fn attach_map_raster_asset(
        &self,
        map_entity_id: String,
        bytes: Vec<u8>,
        mime_type: String,
        filename: String,
        request_id: Option<&str>,
    ) -> Result<AttachedMapRaster, CoreError> {
        let entity_type: Option<String> = self
            .connection
            .query_row(
                "SELECT entity_type FROM entities WHERE id=?1 AND deleted=0",
                params![map_entity_id],
                |row| row.get(0),
            )
            .optional()?;
        if entity_type.as_deref() != Some(crate::maps::MAP_ENTITY_TYPE) {
            return Err(CoreError::Validation(
                "maps: raster assets belong only on a map entity".into(),
            ));
        }
        let provider: Option<String> = self
            .connection
            .query_row(
                "SELECT json_extract(value, '$.provider.id') FROM entity_fields WHERE entity_id=?1 AND namespace=?2 AND key='map'",
                params![map_entity_id, crate::maps::MAP_NAMESPACE],
                |row| row.get(0),
            )
            .optional()?;
        if provider.as_deref() != Some(crate::maps::VECTOR_PROVIDER) {
            return Err(CoreError::Validation(
                "maps: raster assets can only be attached to daena-openlayers maps".into(),
            ));
        }
        let source = crate::maps::validate_image_source(&bytes, &mime_type)?;
        let filename = Path::new(&filename)
            .file_name()
            .and_then(|value| value.to_str())
            .filter(|value| !value.is_empty() && *value != "." && *value != "..")
            .unwrap_or(match source.source_format {
                "jpeg" => "overlay.jpeg",
                "svg" => "overlay.svg",
                _ => "overlay.png",
            })
            .to_owned();
        let content_hash = crate::maps::image::content_hash(&bytes);
        let input_fingerprint = digest_bytes(
            &serde_json::to_vec(&serde_json::json!({
                "mapEntityId": map_entity_id,
                "mimeType": mime_type,
                "filename": filename,
                "contentHash": content_hash,
            }))
            .map_err(|error| CoreError::Serialization(error.to_string()))?,
        );
        if let Some(attached) = self.committed_mutation_with_fingerprint::<AttachedMapRaster>(
            request_id,
            Some(&input_fingerprint),
        )? {
            return Ok(attached);
        }
        let size = bytes.len() as i64;
        if let Some(root) = self.root.as_deref() {
            store_runtime_asset(root, bytes.as_slice(), Some(&content_hash))?;
        }
        let asset_id = Uuid::new_v4().to_string();
        let now = chrono_like_now();
        let relative_path = format!("assets/maps/{}-{filename}", Uuid::new_v4());
        let asset = Asset {
            id: asset_id.clone(),
            entity_id: map_entity_id.clone(),
            namespace: crate::maps::MAP_NAMESPACE.into(),
            filename: filename.clone(),
            content_hash: content_hash.clone(),
            size,
            mime_type: mime_type.clone(),
            path: relative_path.clone(),
            created_at: now.clone(),
            role: ASSET_ROLE_ATTACHMENT.into(),
            reference_scope: ASSET_REFERENCE_SCOPE_ENTITY.into(),
            provenance: None,
            revision: String::new(),
        };
        let attached = AttachedMapRaster {
            asset: asset.clone(),
            width: source.width,
            height: source.height,
        };
        let request_id = self.request_id(request_id)?;
        let result = serde_json::to_value(&attached)
            .map_err(|error| CoreError::Serialization(error.to_string()))?;
        let transaction = self.begin_mutation_with_fingerprint(
            &request_id,
            Some(&result),
            &[format!("entities/{map_entity_id}/"), relative_path.clone()],
            &input_fingerprint,
        )?;
        transaction.execute(
            "INSERT INTO assets(id,entity_id,namespace,filename,content_hash,size,mime_type,path,created_at) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9)",
            params![asset_id, map_entity_id, crate::maps::MAP_NAMESPACE, filename, content_hash, size, mime_type, relative_path, now],
        )?;
        transaction.commit()?;
        self.notify_export_worker()?;
        let mut attached = attached;
        attached.asset.revision = self.revision_for_asset(&attached.asset.id)?;
        self.write_mutation_result(
            &request_id,
            &serde_json::to_value(&attached)
                .map_err(|error| CoreError::Serialization(error.to_string()))?,
        )?;
        Ok(attached)
    }

    pub fn duplicate_map_raster_asset(
        &self,
        map_entity_id: String,
        asset_id: String,
        request_id: Option<&str>,
    ) -> Result<AttachedMapRaster, CoreError> {
        let asset = self.asset_unchecked(&asset_id)?;
        if asset.entity_id != map_entity_id {
            return Err(CoreError::Validation(
                "maps: raster assets can only be duplicated on their owning map".into(),
            ));
        }
        if asset.mime_type != "image/png" {
            return Err(CoreError::Validation(
                "maps: rasterAssetId must name a PNG asset".into(),
            ));
        }
        let bytes = self.asset_bytes(asset_id)?;
        self.attach_map_raster_asset(
            map_entity_id,
            bytes,
            asset.mime_type,
            asset.filename,
            request_id,
        )
    }

    pub fn replay_imported_image_map(
        &self,
        request_id: Option<&str>,
    ) -> Result<Option<ImportedImageMap>, CoreError> {
        self.committed_mutation(request_id)
    }

    pub fn accept_vector_map(
        &self,
        name: String,
        bytes: Vec<u8>,
        generation: serde_json::Value,
        request_id: Option<&str>,
    ) -> Result<AcceptedVectorMap, CoreError> {
        crate::maps::vector::validate_generation(&generation)?;
        let upload_hash = format!("sha256:{:x}", Sha256::digest(&bytes));
        let input_fingerprint = digest_bytes(
            &serde_json::to_vec(&serde_json::json!({
                "name": name,
                "generation": generation,
                "uploadHash": upload_hash,
            }))
            .map_err(|error| CoreError::Serialization(error.to_string()))?,
        );
        if let Some(accepted) = self.committed_mutation_with_fingerprint::<AcceptedVectorMap>(
            request_id,
            Some(&input_fingerprint),
        )? {
            return Ok(accepted);
        }
        if bytes.len() > crate::maps::VECTOR_MAX_BYTES {
            return Err(crate::maps::vector::fail(
                crate::maps::vector::CODE_LIMIT,
                "$",
                "source asset exceeds 16 MiB",
            ));
        }
        let canonical = crate::maps::vector::canonicalize_candidate(&bytes)?;
        let content_hash = format!("sha256:{:x}", Sha256::digest(&canonical));
        let size = canonical.len() as i64;
        if let Some(root) = self.root.as_deref() {
            store_runtime_asset(root, canonical.as_slice(), Some(&content_hash))?;
        }
        let entity_id = Uuid::new_v4().to_string();
        let asset_id = Uuid::new_v4().to_string();
        let now = chrono_like_now();
        let relative_path = format!("assets/maps/{}-map.geojson", Uuid::new_v4());
        let descriptor = serde_json::json!({
            "schemaVersion": crate::maps::MAP_DESCRIPTOR_SCHEMA_VERSION,
            "provider": {
                "id": crate::maps::VECTOR_PROVIDER,
                "adapterVersion": crate::maps::VECTOR_ADAPTER_VERSION,
                "sourceFormat": crate::maps::VECTOR_SOURCE_FORMAT
            },
            "sourceAssetId": asset_id,
            "previewAssetId": null,
            "coordinateSpace": {"kind": "world", "extent": [-180, -90, 180, 90], "origin": "bottom-left", "units": {"id": "world-unit", "label": "World units", "metresPerUnit": null}, "wrapX": false},
            "backgrounds": [],
            "defaultView": {"center": [0, 0], "zoom": 1, "rotation": 0},
            "settings": {"snapEnabled": true, "grid": null},
            "generation": generation
        });
        let layers = serde_json::json!({"schemaVersion": crate::maps::MAP_LAYERS_SCHEMA_VERSION, "layers": []});
        let entity = Entity {
            id: entity_id.clone(),
            name: name.trim().into(),
            entity_type: Some(crate::maps::MAP_ENTITY_TYPE.into()),
            deleted: false,
            created_at: now.clone(),
            updated_at: now.clone(),
            revision: String::new(),
        };
        let asset = Asset {
            id: asset_id.clone(),
            entity_id: entity_id.clone(),
            namespace: crate::maps::MAP_NAMESPACE.into(),
            filename: crate::maps::VECTOR_FILENAME.into(),
            content_hash: content_hash.clone(),
            size,
            mime_type: crate::maps::VECTOR_MIME.into(),
            path: relative_path.clone(),
            created_at: now.clone(),
            role: ASSET_ROLE_ATTACHMENT.into(),
            reference_scope: ASSET_REFERENCE_SCOPE_ENTITY.into(),
            provenance: None,
            revision: String::new(),
        };
        let accepted = AcceptedVectorMap {
            entity: entity.clone(),
            source: asset.clone(),
        };
        let request_id = self.request_id(request_id)?;
        let result = serde_json::to_value(&accepted)
            .map_err(|error| CoreError::Serialization(error.to_string()))?;
        let transaction = self.begin_mutation_with_fingerprint(
            &request_id,
            Some(&result),
            &[format!("entities/{entity_id}/"), relative_path.clone()],
            &input_fingerprint,
        )?;
        transaction.execute(
            "INSERT INTO entities(id,name,entity_type,created_at,updated_at) VALUES (?1,?2,?3,?4,?4)",
            params![entity_id, entity.name, crate::maps::MAP_ENTITY_TYPE, now],
        )?;
        transaction.execute(
            "INSERT INTO assets(id,entity_id,namespace,filename,content_hash,size,mime_type,path,created_at) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9)",
            params![asset_id, entity_id, crate::maps::MAP_NAMESPACE, crate::maps::VECTOR_FILENAME, content_hash, size, crate::maps::VECTOR_MIME, relative_path, now],
        )?;
        crate::maps::validate_field(&transaction, &entity_id, "map", &descriptor)?;
        crate::maps::validate_field(&transaction, &entity_id, "layers", &layers)?;
        transaction.execute(
            "INSERT INTO entity_fields(entity_id,namespace,key,value) VALUES (?1,?2,?3,?4)",
            params![
                entity_id,
                crate::maps::MAP_NAMESPACE,
                "map",
                encode_field_value(&descriptor)?
            ],
        )?;
        transaction.execute(
            "INSERT INTO entity_fields(entity_id,namespace,key,value) VALUES (?1,?2,?3,?4)",
            params![
                entity_id,
                crate::maps::MAP_NAMESPACE,
                "layers",
                encode_field_value(&layers)?
            ],
        )?;
        transaction.commit()?;
        self.refresh_maps_projection_for_entities(std::slice::from_ref(&accepted.entity.id))?;
        self.notify_export_worker()?;
        let mut accepted = accepted;
        accepted.entity.revision = self.revision_for_entity(&accepted.entity.id)?;
        accepted.source.revision = self.revision_for_asset(&accepted.source.id)?;
        self.write_mutation_result(
            &request_id,
            &serde_json::to_value(&accepted)
                .map_err(|error| CoreError::Serialization(error.to_string()))?,
        )?;
        Ok(accepted)
    }

    /// Imports polygonal GeoJSON as a native vector map with read-only base land.
    /// Does not record generator provenance; editing base land is deferred.
    pub fn import_vector_map(
        &self,
        name: String,
        bytes: Vec<u8>,
        request_id: Option<&str>,
    ) -> Result<AcceptedVectorMap, CoreError> {
        let upload_hash = format!("sha256:{:x}", Sha256::digest(&bytes));
        let input_fingerprint = digest_bytes(
            &serde_json::to_vec(&serde_json::json!({
                "name": name,
                "uploadHash": upload_hash,
                "kind": "import-vector-geojson",
            }))
            .map_err(|error| CoreError::Serialization(error.to_string()))?,
        );
        if let Some(accepted) = self.committed_mutation_with_fingerprint::<AcceptedVectorMap>(
            request_id,
            Some(&input_fingerprint),
        )? {
            return Ok(accepted);
        }
        if bytes.len() > crate::maps::VECTOR_MAX_BYTES {
            return Err(crate::maps::vector::fail(
                crate::maps::vector::CODE_LIMIT,
                "$",
                "source asset exceeds 16 MiB",
            ));
        }
        let canonical = crate::maps::vector::canonicalize_imported_base(&bytes)?;
        let content_hash = format!("sha256:{:x}", Sha256::digest(&canonical));
        let size = canonical.len() as i64;
        if let Some(root) = self.root.as_deref() {
            store_runtime_asset(root, canonical.as_slice(), Some(&content_hash))?;
        }
        let entity_id = Uuid::new_v4().to_string();
        let asset_id = Uuid::new_v4().to_string();
        let now = chrono_like_now();
        let relative_path = format!("assets/maps/{}-map.geojson", Uuid::new_v4());
        let descriptor = serde_json::json!({
            "schemaVersion": crate::maps::MAP_DESCRIPTOR_SCHEMA_VERSION,
            "provider": {
                "id": crate::maps::VECTOR_PROVIDER,
                "adapterVersion": crate::maps::VECTOR_ADAPTER_VERSION,
                "sourceFormat": crate::maps::VECTOR_SOURCE_FORMAT
            },
            "sourceAssetId": asset_id,
            "previewAssetId": null,
            "coordinateSpace": {"kind": "world", "extent": [-180, -90, 180, 90], "origin": "bottom-left", "units": {"id": "world-unit", "label": "World units", "metresPerUnit": null}, "wrapX": false},
            "backgrounds": [],
            "defaultView": {"center": [0, 0], "zoom": 1, "rotation": 0},
            "settings": {"snapEnabled": true, "grid": null}
        });
        let layers = serde_json::json!({"schemaVersion": crate::maps::MAP_LAYERS_SCHEMA_VERSION, "layers": []});
        let entity = Entity {
            id: entity_id.clone(),
            name: name.trim().into(),
            entity_type: Some(crate::maps::MAP_ENTITY_TYPE.into()),
            deleted: false,
            created_at: now.clone(),
            updated_at: now.clone(),
            revision: String::new(),
        };
        let asset = Asset {
            id: asset_id.clone(),
            entity_id: entity_id.clone(),
            namespace: crate::maps::MAP_NAMESPACE.into(),
            filename: crate::maps::VECTOR_FILENAME.into(),
            content_hash: content_hash.clone(),
            size,
            mime_type: crate::maps::VECTOR_MIME.into(),
            path: relative_path.clone(),
            created_at: now.clone(),
            role: ASSET_ROLE_ATTACHMENT.into(),
            reference_scope: ASSET_REFERENCE_SCOPE_ENTITY.into(),
            provenance: None,
            revision: String::new(),
        };
        let accepted = AcceptedVectorMap {
            entity: entity.clone(),
            source: asset.clone(),
        };
        let request_id = self.request_id(request_id)?;
        let result = serde_json::to_value(&accepted)
            .map_err(|error| CoreError::Serialization(error.to_string()))?;
        let transaction = self.begin_mutation_with_fingerprint(
            &request_id,
            Some(&result),
            &[format!("entities/{entity_id}/"), relative_path.clone()],
            &input_fingerprint,
        )?;
        transaction.execute(
            "INSERT INTO entities(id,name,entity_type,created_at,updated_at) VALUES (?1,?2,?3,?4,?4)",
            params![entity_id, entity.name, crate::maps::MAP_ENTITY_TYPE, now],
        )?;
        transaction.execute(
            "INSERT INTO assets(id,entity_id,namespace,filename,content_hash,size,mime_type,path,created_at) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9)",
            params![asset_id, entity_id, crate::maps::MAP_NAMESPACE, crate::maps::VECTOR_FILENAME, content_hash, size, crate::maps::VECTOR_MIME, relative_path, now],
        )?;
        crate::maps::validate_field(&transaction, &entity_id, "map", &descriptor)?;
        crate::maps::validate_field(&transaction, &entity_id, "layers", &layers)?;
        transaction.execute(
            "INSERT INTO entity_fields(entity_id,namespace,key,value) VALUES (?1,?2,?3,?4)",
            params![
                entity_id,
                crate::maps::MAP_NAMESPACE,
                "map",
                encode_field_value(&descriptor)?
            ],
        )?;
        transaction.execute(
            "INSERT INTO entity_fields(entity_id,namespace,key,value) VALUES (?1,?2,?3,?4)",
            params![
                entity_id,
                crate::maps::MAP_NAMESPACE,
                "layers",
                encode_field_value(&layers)?
            ],
        )?;
        transaction.commit()?;
        self.refresh_maps_projection_for_entities(std::slice::from_ref(&accepted.entity.id))?;
        self.notify_export_worker()?;
        let mut accepted = accepted;
        accepted.entity.revision = self.revision_for_entity(&accepted.entity.id)?;
        accepted.source.revision = self.revision_for_asset(&accepted.source.id)?;
        self.write_mutation_result(
            &request_id,
            &serde_json::to_value(&accepted)
                .map_err(|error| CoreError::Serialization(error.to_string()))?,
        )?;
        Ok(accepted)
    }

    pub fn replay_accepted_vector_map(
        &self,
        request_id: Option<&str>,
    ) -> Result<Option<AcceptedVectorMap>, CoreError> {
        self.committed_mutation(request_id)
    }

    pub fn accept_physical_map(
        &self,
        name: String,
        bytes: Vec<u8>,
        generation: serde_json::Value,
        request_id: Option<&str>,
    ) -> Result<AcceptedPhysicalMap, CoreError> {
        let upload_hash = format!("sha256:{:x}", Sha256::digest(&bytes));
        let input_fingerprint = digest_bytes(
            &serde_json::to_vec(&serde_json::json!({
                "name": name,
                "generation": generation,
                "uploadHash": upload_hash,
            }))
            .map_err(|error| CoreError::Serialization(error.to_string()))?,
        );
        if let Some(accepted) = self.committed_mutation_with_fingerprint::<AcceptedPhysicalMap>(
            request_id,
            Some(&input_fingerprint),
        )? {
            return Ok(accepted);
        }
        if bytes.len() > crate::maps::PHYSICAL_MAX_SOURCE_BYTES {
            return Err(CoreError::Validation(format!(
                "{}: source asset exceeds {} bytes",
                crate::maps::physical::CODE_INVALID_SOURCE,
                crate::maps::PHYSICAL_MAX_SOURCE_BYTES
            )));
        }
        let validated = crate::maps::physical::validate_source(&bytes, &generation)?;
        let content_hash = format!("sha256:{:x}", Sha256::digest(&bytes));
        let size = bytes.len() as i64;
        let authored_bytes = crate::maps::vector::empty_canonical_bytes();
        let authored_content_hash = format!("sha256:{:x}", Sha256::digest(&authored_bytes));
        let authored_size = authored_bytes.len() as i64;
        if let Some(root) = self.root.as_deref() {
            store_runtime_asset(root, bytes.as_slice(), Some(&content_hash))?;
        }
        let entity_id = Uuid::new_v4().to_string();
        let asset_id = Uuid::new_v4().to_string();
        let authored_asset_id = Uuid::new_v4().to_string();
        let now = chrono_like_now();
        let relative_path = format!("assets/maps/{}-world.pworld", Uuid::new_v4());
        let authored_relative_path = format!("assets/maps/{}-authored.geojson", Uuid::new_v4());
        if let Some(root) = self.root.as_deref() {
            store_runtime_asset(
                root,
                authored_bytes.as_slice(),
                Some(&authored_content_hash),
            )?;
        }
        let descriptor = serde_json::json!({
            "schemaVersion": crate::maps::MAP_DESCRIPTOR_SCHEMA_VERSION,
            "provider": {
                "id": crate::maps::PHYSICAL_PROVIDER,
                "adapterVersion": crate::maps::PHYSICAL_ADAPTER_VERSION,
                "sourceFormat": crate::maps::PHYSICAL_SOURCE_FORMAT
            },
            "sourceAssetId": asset_id,
            "authoredSourceAssetId": authored_asset_id,
            "previewAssetId": null,
            "defaultView": {"center": [0.5, 0.5], "zoom": 1},
            "generation": generation
        });
        let layers = crate::maps::physical::initial_layers_value();
        let entity = Entity {
            id: entity_id.clone(),
            name: name.trim().into(),
            entity_type: Some(crate::maps::MAP_ENTITY_TYPE.into()),
            deleted: false,
            created_at: now.clone(),
            updated_at: now.clone(),
            revision: String::new(),
        };
        let asset = Asset {
            id: asset_id.clone(),
            entity_id: entity_id.clone(),
            namespace: crate::maps::MAP_NAMESPACE.into(),
            filename: crate::maps::PHYSICAL_FILENAME.into(),
            content_hash: content_hash.clone(),
            size,
            mime_type: crate::maps::PHYSICAL_MIME.into(),
            path: relative_path.clone(),
            created_at: now.clone(),
            role: ASSET_ROLE_ATTACHMENT.into(),
            reference_scope: ASSET_REFERENCE_SCOPE_ENTITY.into(),
            provenance: None,
            revision: String::new(),
        };
        let accepted = AcceptedPhysicalMap {
            entity: entity.clone(),
            source: asset.clone(),
            physical_identity: validated.identity,
        };
        let request_id = self.request_id(request_id)?;
        let result = serde_json::to_value(&accepted)
            .map_err(|error| CoreError::Serialization(error.to_string()))?;
        let transaction = self.begin_mutation_with_fingerprint(
            &request_id,
            Some(&result),
            &[
                format!("entities/{entity_id}/"),
                relative_path.clone(),
                authored_relative_path.clone(),
            ],
            &input_fingerprint,
        )?;
        transaction.execute(
            "INSERT INTO entities(id,name,entity_type,created_at,updated_at) VALUES (?1,?2,?3,?4,?4)",
            params![entity_id, entity.name, crate::maps::MAP_ENTITY_TYPE, now],
        )?;
        transaction.execute(
            "INSERT INTO assets(id,entity_id,namespace,filename,content_hash,size,mime_type,path,created_at) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9)",
            params![asset_id, entity_id, crate::maps::MAP_NAMESPACE, crate::maps::PHYSICAL_FILENAME, content_hash, size, crate::maps::PHYSICAL_MIME, relative_path, now],
        )?;
        transaction.execute(
            "INSERT INTO assets(id,entity_id,namespace,filename,content_hash,size,mime_type,path,created_at) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9)",
            params![authored_asset_id, entity_id, crate::maps::MAP_NAMESPACE, crate::maps::VECTOR_FILENAME, authored_content_hash, authored_size, crate::maps::VECTOR_MIME, authored_relative_path, now],
        )?;
        crate::maps::validate_field(&transaction, &entity_id, "map", &descriptor)?;
        transaction.execute(
            "INSERT INTO entity_fields(entity_id,namespace,key,value) VALUES (?1,?2,?3,?4)",
            params![
                entity_id,
                crate::maps::MAP_NAMESPACE,
                "map",
                encode_field_value(&descriptor)?
            ],
        )?;
        crate::maps::validate_field(&transaction, &entity_id, "layers", &layers)?;
        transaction.execute(
            "INSERT INTO entity_fields(entity_id,namespace,key,value) VALUES (?1,?2,?3,?4)",
            params![
                entity_id,
                crate::maps::MAP_NAMESPACE,
                "layers",
                encode_field_value(&layers)?
            ],
        )?;
        transaction.commit()?;
        self.refresh_maps_projection_for_entities(std::slice::from_ref(&accepted.entity.id))?;
        self.notify_export_worker()?;
        let mut accepted = accepted;
        accepted.entity.revision = self.revision_for_entity(&accepted.entity.id)?;
        accepted.source.revision = self.revision_for_asset(&accepted.source.id)?;
        self.write_mutation_result(
            &request_id,
            &serde_json::to_value(&accepted)
                .map_err(|error| CoreError::Serialization(error.to_string()))?,
        )?;
        Ok(accepted)
    }

    pub fn replay_accepted_physical_map(
        &self,
        request_id: Option<&str>,
    ) -> Result<Option<AcceptedPhysicalMap>, CoreError> {
        self.committed_mutation(request_id)
    }

    pub fn replace_vector_source(
        &self,
        asset_id: String,
        upload_bytes: Vec<u8>,
        upload_content_hash: String,
        expected_revision: &str,
        request_id: Option<&str>,
    ) -> Result<VectorSourceReplace, CoreError> {
        let input_fingerprint = digest_bytes(
            &serde_json::to_vec(&serde_json::json!({
                "assetId": asset_id,
                "uploadContentHash": upload_content_hash,
                "expectedRevision": expected_revision,
            }))
            .map_err(|error| CoreError::Serialization(error.to_string()))?,
        );
        if let Some(replaced) = self.committed_mutation_with_fingerprint::<VectorSourceReplace>(
            request_id,
            Some(&input_fingerprint),
        )? {
            return Ok(replaced);
        }
        let digest = format!("sha256:{:x}", Sha256::digest(&upload_bytes));
        if digest != upload_content_hash {
            return Err(CoreError::Validation(
                "transfer.invalid: upload content hash does not match bytes".into(),
            ));
        }
        let mut asset = self.asset_unchecked(&asset_id)?;
        Self::ensure_expected_revision(Some(expected_revision), asset.revision.clone(), "asset")?;
        if asset.namespace != crate::maps::MAP_NAMESPACE
            || asset.mime_type != crate::maps::VECTOR_MIME
        {
            return Err(crate::maps::vector::fail(
                crate::maps::vector::CODE_SOURCE_INVALID,
                "$",
                "replacement target is not a daena-openlayers source asset",
            ));
        }
        let layers = self.layers_field(&asset.entity_id)?;
        let known = crate::maps::vector::layer_ids_from_layers_field(&layers.value);
        let descriptor_value = self
            .list_fields_unchecked(asset.entity_id.clone())?
            .into_iter()
            .find(|field| field.namespace == crate::maps::MAP_NAMESPACE && field.key == "map")
            .map(|field| field.value)
            .unwrap_or(serde_json::Value::Null);
        let vector_space =
            crate::maps::vector::VectorSpace::from_descriptor_value(&descriptor_value);
        let canonical = crate::maps::vector::canonicalize_committed_with_space(
            &upload_bytes,
            &known,
            &vector_space,
        )?;
        let physical_map = descriptor_value
            .pointer("/provider/id")
            .and_then(serde_json::Value::as_str)
            .map(|provider| provider == crate::maps::PHYSICAL_PROVIDER)
            .unwrap_or(false);
        if physical_map {
            crate::maps::validate_physical_authored_source_bytes(&canonical)?;
        }
        let content_hash = format!("sha256:{:x}", Sha256::digest(&canonical));
        let size = canonical.len() as i64;
        if let Some(root) = self.root.as_deref() {
            store_runtime_asset(root, canonical.as_slice(), Some(&content_hash))?;
        }
        let request_id = self.request_id(request_id)?;
        let result = serde_json::to_value(&VectorSourceReplace {
            source: Asset {
                revision: String::new(),
                content_hash: content_hash.clone(),
                size,
                ..asset.clone()
            },
        })
        .map_err(|error| CoreError::Serialization(error.to_string()))?;
        let transaction = self.begin_mutation_with_fingerprint(
            &request_id,
            Some(&result),
            &[format!("entities/{}/", asset.entity_id), asset.path.clone()],
            &input_fingerprint,
        )?;
        transaction.execute(
            "UPDATE assets SET content_hash=?1,size=?2,mime_type=?3 WHERE id=?4",
            params![content_hash, size, crate::maps::VECTOR_MIME, asset_id],
        )?;
        transaction.commit()?;
        self.refresh_maps_projection_for_entities(std::slice::from_ref(&asset.entity_id))?;
        self.notify_export_worker()?;
        asset.content_hash = content_hash;
        asset.size = size;
        asset.revision = self.revision_for_asset(&asset.id)?;
        let replaced = VectorSourceReplace { source: asset };
        self.write_mutation_result(
            &request_id,
            &serde_json::to_value(&replaced)
                .map_err(|error| CoreError::Serialization(error.to_string()))?,
        )?;
        Ok(replaced)
    }

    pub fn replay_replaced_vector_source(
        &self,
        request_id: Option<&str>,
    ) -> Result<Option<VectorSourceReplace>, CoreError> {
        self.committed_mutation(request_id)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn apply_map_edit(
        &self,
        map_entity_id: String,
        descriptor: serde_json::Value,
        layers_value: serde_json::Value,
        upload_bytes: Vec<u8>,
        upload_content_hash: String,
        expected_map_revision: &str,
        expected_layers_revision: &str,
        expected_source_revision: &str,
        link_mutations: Vec<MapLinkMutation>,
        request_id: Option<&str>,
    ) -> Result<MapEditApply, CoreError> {
        let input_fingerprint = digest_bytes(
            &serde_json::to_vec(&serde_json::json!({
                "mapEntityId": map_entity_id,
                "descriptor": descriptor,
                "layers": layers_value,
                "uploadContentHash": upload_content_hash,
                "expectedMapRevision": expected_map_revision,
                "expectedLayersRevision": expected_layers_revision,
                "expectedSourceRevision": expected_source_revision,
                "linkMutations": link_mutations,
            }))
            .map_err(|error| CoreError::Serialization(error.to_string()))?,
        );
        if let Some(applied) = self.committed_mutation_with_fingerprint::<MapEditApply>(
            request_id,
            Some(&input_fingerprint),
        )? {
            return Ok(applied);
        }
        let digest = format!("sha256:{:x}", Sha256::digest(&upload_bytes));
        if digest != upload_content_hash {
            return Err(CoreError::Validation(
                "transfer.invalid: upload content hash does not match bytes".into(),
            ));
        }
        let mut fields = self.list_fields_unchecked(map_entity_id.clone())?;
        let mut map_field = fields
            .iter_mut()
            .find(|field| field.namespace == crate::maps::MAP_NAMESPACE && field.key == "map")
            .cloned()
            .ok_or_else(|| CoreError::NotFound("map descriptor not found".into()))?;
        map_field.revision = self.revision_for_field(&map_field)?;
        Self::ensure_expected_revision(
            Some(expected_map_revision),
            map_field.revision.clone(),
            "field",
        )?;
        let mut layers_field = self.layers_field(&map_entity_id)?;
        Self::ensure_expected_revision(
            Some(expected_layers_revision),
            layers_field.revision.clone(),
            "field",
        )?;
        let source_id = map_field
            .value
            .get("authoredSourceAssetId")
            .or_else(|| map_field.value.get("sourceAssetId"))
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| CoreError::NotFound("vector source asset not found".into()))?
            .to_owned();
        let next_source_id = descriptor
            .get("authoredSourceAssetId")
            .or_else(|| descriptor.get("sourceAssetId"))
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| CoreError::Validation("maps: vector source asset is required".into()))?;
        if next_source_id != source_id {
            return Err(CoreError::Validation(
                "maps: apply_map_edit cannot replace the canonical source asset identity".into(),
            ));
        }
        let mut source = self.asset_unchecked(&source_id)?;
        Self::ensure_expected_revision(
            Some(expected_source_revision),
            source.revision.clone(),
            "asset",
        )?;
        if source.namespace != crate::maps::MAP_NAMESPACE
            || source.mime_type != crate::maps::VECTOR_MIME
        {
            return Err(crate::maps::vector::fail(
                crate::maps::vector::CODE_SOURCE_INVALID,
                "$",
                "map edit target is not a canonical authored GeoJSON asset",
            ));
        }
        let mut prepared_links = Vec::with_capacity(link_mutations.len());
        let mut seen_link_entities = BTreeSet::new();
        for mutation in &link_mutations {
            if !seen_link_entities.insert(mutation.entity_id.clone()) {
                return Err(CoreError::Validation(
                    "maps: apply_map_edit linkMutations must target distinct entities".into(),
                ));
            }
            let exists: Option<String> = self
                .connection
                .query_row(
                    "SELECT id FROM entities WHERE id=?1 AND deleted=0",
                    params![mutation.entity_id],
                    |row| row.get(0),
                )
                .optional()?;
            if exists.is_none() {
                return Err(CoreError::NotFound(format!(
                    "link entity {} not found",
                    mutation.entity_id
                )));
            }
            let current = self
                .list_fields_unchecked(mutation.entity_id.clone())?
                .into_iter()
                .find(|field| {
                    field.namespace == crate::maps::MAP_NAMESPACE && field.key == "locations"
                });
            let current_revision = match &current {
                Some(field) => self.revision_for_field(field)?,
                None => String::new(),
            };
            Self::ensure_expected_revision(
                Some(mutation.expected_locations_revision.as_str()),
                current_revision,
                "field",
            )?;
            prepared_links.push(FieldValue {
                entity_id: mutation.entity_id.clone(),
                namespace: crate::maps::MAP_NAMESPACE.into(),
                key: "locations".into(),
                value: mutation.locations.clone(),
                revision: String::new(),
            });
        }
        let known = crate::maps::vector::layer_ids_from_layers_field(&layers_value);
        let next_space = crate::maps::vector::VectorSpace::from_descriptor_value(&descriptor);
        let previous_space =
            crate::maps::vector::VectorSpace::from_descriptor_value(&map_field.value);
        let canonical = crate::maps::vector::canonicalize_committed_with_space(
            &upload_bytes,
            &known,
            &next_space,
        )?;
        let previous_source_bytes = self.asset_bytes(source_id.clone())?;
        let previous_known = crate::maps::vector::layer_ids_from_layers_field(&layers_field.value);
        let previous_locked = crate::maps::locked_layer_ids(&layers_field.value);
        let next_locked = crate::maps::locked_layer_ids(&layers_value);
        let still_locked = previous_locked
            .intersection(&next_locked)
            .cloned()
            .collect();
        crate::maps::vector::assert_locked_layer_features_unchanged_with_space(
            &previous_source_bytes,
            &canonical,
            &previous_known,
            &known,
            &still_locked,
            &next_space,
        )?;
        // For validation of previous vs next with differing spaces, also ensure previous bytes
        // were valid in its own space when spaces differ (e.g., image -> world calibration).
        if previous_space != next_space {
            // Re-validate previous bytes in its own space to ensure the comparison is meaningful;
            // if it fails, surface the error as locked-layer unchanged failure.
            let _ = crate::maps::vector::canonicalize_committed_with_space(
                &previous_source_bytes,
                &previous_known,
                &previous_space,
            )?;
        }
        if descriptor
            .pointer("/provider/id")
            .and_then(serde_json::Value::as_str)
            == Some(crate::maps::PHYSICAL_PROVIDER)
        {
            crate::maps::validate_physical_authored_source_bytes(&canonical)?;
        }
        let content_hash = format!("sha256:{:x}", Sha256::digest(&canonical));
        let size = canonical.len() as i64;
        if let Some(root) = self.root.as_deref() {
            store_runtime_asset(root, canonical.as_slice(), Some(&content_hash))?;
        }
        let request_id = self.request_id(request_id)?;
        let result = serde_json::to_value(&MapEditApply {
            map: FieldValue {
                entity_id: map_entity_id.clone(),
                namespace: crate::maps::MAP_NAMESPACE.into(),
                key: "map".into(),
                value: descriptor.clone(),
                revision: String::new(),
            },
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
            locations: prepared_links.clone(),
        })
        .map_err(|error| CoreError::Serialization(error.to_string()))?;
        let mut affected = vec![format!("entities/{map_entity_id}/"), source.path.clone()];
        for link in &prepared_links {
            affected.push(format!("entities/{}/", link.entity_id));
        }
        let transaction = self.begin_mutation_with_fingerprint(
            &request_id,
            Some(&result),
            &affected,
            &input_fingerprint,
        )?;
        crate::maps::validate_field(&transaction, &map_entity_id, "map", &descriptor)?;
        crate::maps::validate_field(&transaction, &map_entity_id, "layers", &layers_value)?;
        for link in &prepared_links {
            crate::maps::validate_field(&transaction, &link.entity_id, "locations", &link.value)?;
        }
        transaction.execute(
            "UPDATE assets SET content_hash=?1,size=?2,mime_type=?3 WHERE id=?4",
            params![content_hash, size, crate::maps::VECTOR_MIME, source.id],
        )?;
        transaction.execute(
            "UPDATE entity_fields SET value=?1 WHERE entity_id=?2 AND namespace=?3 AND key='map'",
            params![
                encode_field_value(&descriptor)?,
                map_entity_id,
                crate::maps::MAP_NAMESPACE
            ],
        )?;
        transaction.execute(
            "UPDATE entity_fields SET value=?1 WHERE entity_id=?2 AND namespace=?3 AND key='layers'",
            params![encode_field_value(&layers_value)?, map_entity_id, crate::maps::MAP_NAMESPACE],
        )?;
        for link in &prepared_links {
            transaction.execute(
                "INSERT INTO entity_fields(entity_id,namespace,key,value) VALUES (?1,?2,?3,?4) ON CONFLICT(entity_id,namespace,key) DO UPDATE SET value=excluded.value",
                params![
                    link.entity_id,
                    crate::maps::MAP_NAMESPACE,
                    "locations",
                    encode_field_value(&link.value)?
                ],
            )?;
        }
        transaction.commit()?;
        let mut projection_ids = vec![map_entity_id.clone()];
        projection_ids.extend(prepared_links.iter().map(|link| link.entity_id.clone()));
        self.refresh_maps_projection_for_entities(&projection_ids)?;
        self.notify_export_worker()?;
        map_field.value = descriptor;
        map_field.revision = self.revision_for_field(&map_field)?;
        layers_field.value = layers_value;
        layers_field.revision = self.revision_for_field(&layers_field)?;
        source.content_hash = content_hash;
        source.size = size;
        source.revision = self.revision_for_asset(&source.id)?;
        let mut locations = Vec::with_capacity(prepared_links.len());
        for mut link in prepared_links {
            link.revision = self.revision_for_field(&link)?;
            locations.push(link);
        }
        let applied = MapEditApply {
            map: map_field,
            layers: layers_field,
            source,
            locations,
        };
        self.write_mutation_result(
            &request_id,
            &serde_json::to_value(&applied)
                .map_err(|error| CoreError::Serialization(error.to_string()))?,
        )?;
        Ok(applied)
    }
}
