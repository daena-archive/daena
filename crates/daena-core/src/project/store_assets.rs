// Asset registration, replacement, and export operations.
use super::*;

impl ProjectStore {
    pub(crate) fn read_asset_bytes(&self, asset: &Asset) -> Result<Vec<u8>, CoreError> {
        let root = self.project_root()?;
        let path = runtime_asset_path(root, &asset.content_hash)?;
        std::fs::read(&path).map_err(|source| CoreError::Io {
            operation: "read runtime asset",
            source,
        })
    }

    pub(crate) fn is_immutable_image_source(&self, asset: &Asset) -> Result<bool, CoreError> {
        if asset.namespace != crate::maps::MAP_NAMESPACE {
            return Ok(false);
        }
        let Some(descriptor) = self
            .list_fields_unchecked(asset.entity_id.clone())?
            .into_iter()
            .find(|field| field.namespace == crate::maps::MAP_NAMESPACE && field.key == "map")
        else {
            return Ok(false);
        };
        Ok(descriptor
            .value
            .pointer("/previewAssetId")
            .and_then(serde_json::Value::as_str)
            == Some(asset.id.as_str())
            && descriptor
                .value
                .pointer("/provider/id")
                .and_then(serde_json::Value::as_str)
                == Some(crate::maps::VECTOR_PROVIDER))
    }

    pub(crate) fn physical_identity_for_descriptor(
        &self,
        descriptor: &serde_json::Value,
    ) -> Result<Option<String>, CoreError> {
        if descriptor
            .get("provider")
            .and_then(|provider| provider.get("id"))
            .and_then(serde_json::Value::as_str)
            != Some(crate::maps::PHYSICAL_PROVIDER)
        {
            return Ok(None);
        }
        let source_id = descriptor
            .get("sourceAssetId")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| {
                CoreError::Validation("daena-physical maps require sourceAssetId".into())
            })?;
        let generation = descriptor.get("generation").ok_or_else(|| {
            CoreError::Validation("daena-physical maps require generation".into())
        })?;
        let bytes = self.asset_bytes(source_id.to_owned())?;
        Ok(Some(
            crate::maps::physical::validate_source(&bytes, generation)?.identity,
        ))
    }

    pub fn register_asset(&self, input: AssetInput) -> Result<Asset, CoreError> {
        self.register_asset_with_options(input, None, None)
    }

    pub fn register_asset_with_request(
        &self,
        input: AssetInput,
        request_id: Option<&str>,
    ) -> Result<Asset, CoreError> {
        self.register_asset_with_options(input, None, request_id)
    }

    pub fn register_asset_with_options(
        &self,
        input: AssetInput,
        expected_revision: Option<&str>,
        request_id: Option<&str>,
    ) -> Result<Asset, CoreError> {
        if let Some(mut asset) = self.committed_mutation::<Asset>(request_id)? {
            asset.revision = self.revision_for_asset_value(&asset)?;
            return Ok(asset);
        }
        let mut input = input;
        input.filename = validated_asset_filename(&input.filename)?;
        let exists: Option<String> = self
            .connection
            .query_row(
                "SELECT id FROM entities WHERE id=?1 AND deleted=0",
                params![input.entity_id],
                |row| row.get(0),
            )
            .optional()?;
        if exists.is_none() {
            return Err(CoreError::NotFound("entity not found".into()));
        }
        Self::ensure_expected_revision(
            expected_revision,
            self.revision_for_entity(&input.entity_id)?,
            "asset entity",
        )?;
        if input.size < 0 {
            return Err(CoreError::Validation(
                "asset size cannot be negative".into(),
            ));
        }
        let provenance = encode_asset_provenance(&input.provenance)?;
        if let Some(root) = self.root.as_deref() {
            ensure_runtime_asset(root, &input.path, &input.content_hash, input.size)?;
        }
        let id = Uuid::new_v4().to_string();
        let now = chrono_like_now();
        let request_id = self.request_id(request_id)?;
        let result = serde_json::to_value(&Asset {
            id: id.clone(),
            entity_id: input.entity_id.clone(),
            namespace: input.namespace.clone(),
            filename: input.filename.clone(),
            content_hash: input.content_hash.clone(),
            size: input.size,
            mime_type: input.mime_type.clone(),
            path: input.path.clone(),
            created_at: now.clone(),
            role: ASSET_ROLE_ATTACHMENT.into(),
            reference_scope: ASSET_REFERENCE_SCOPE_ENTITY.into(),
            provenance: input.provenance.clone(),
            revision: String::new(),
        })
        .map_err(|error| CoreError::Serialization(error.to_string()))?;
        let transaction = self.begin_mutation(
            &request_id,
            Some(&result),
            &[format!("entities/{}/", input.entity_id), input.path.clone()],
        )?;
        transaction.execute(
            "INSERT INTO assets(id,entity_id,namespace,filename,content_hash,size,mime_type,path,created_at,provenance) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10)",
            params![id, input.entity_id, input.namespace, input.filename, input.content_hash, input.size, input.mime_type, input.path, now, provenance],
        )?;
        transaction.commit()?;
        self.refresh_maps_projection_for_entities(std::slice::from_ref(&input.entity_id))?;
        self.notify_export_worker()?;
        let revision = self.revision_for_asset(&id)?;
        Ok(Asset {
            id,
            entity_id: input.entity_id,
            namespace: input.namespace,
            filename: input.filename,
            content_hash: input.content_hash,
            size: input.size,
            mime_type: input.mime_type,
            path: input.path,
            created_at: now,
            role: ASSET_ROLE_ATTACHMENT.into(),
            reference_scope: ASSET_REFERENCE_SCOPE_ENTITY.into(),
            provenance: input.provenance,
            revision,
        })
    }

    pub fn register_asset_file(&self, input: AssetFileInput) -> Result<Asset, CoreError> {
        self.register_asset_file_with_options(input, None, None)
    }

    pub fn register_asset_file_with_request(
        &self,
        input: AssetFileInput,
        request_id: Option<&str>,
    ) -> Result<Asset, CoreError> {
        self.register_asset_file_with_options(input, None, request_id)
    }

    pub fn register_asset_file_with_options(
        &self,
        input: AssetFileInput,
        expected_revision: Option<&str>,
        request_id: Option<&str>,
    ) -> Result<Asset, CoreError> {
        encode_asset_provenance(&input.provenance)?;
        let source = Path::new(&input.source_path);
        let metadata =
            std::fs::metadata(source).map_err(|error| CoreError::NotFound(error.to_string()))?;
        if !metadata.is_file() {
            return Err(CoreError::NotFound("asset source is not a file".into()));
        }
        let filename = validated_asset_filename(&input.filename)?;
        let category = if input.mime_type.starts_with("image/") {
            "images"
        } else if input.mime_type.starts_with("video/") {
            "videos"
        } else if input.mime_type.contains("map")
            || matches!(
                Path::new(&filename)
                    .extension()
                    .and_then(|value| value.to_str()),
                Some("geojson" | "tmx" | "mbtiles")
            )
        {
            "maps"
        } else {
            "files"
        };
        let (content_hash, size) = if let Some(root) = self.root.as_deref() {
            store_runtime_asset_file(root, source, None)?
        } else {
            streamed_file_digest(source)?
        };
        let relative_path = format!("assets/{category}/{}-{filename}", Uuid::new_v4());
        let request_id = self.request_id(request_id)?;
        self.register_asset_with_options(
            AssetInput {
                entity_id: input.entity_id,
                namespace: input.namespace,
                filename,
                content_hash,
                size,
                mime_type: input.mime_type,
                path: relative_path.clone(),
                provenance: input.provenance,
            },
            expected_revision,
            Some(&request_id),
        )
    }

    pub fn list_assets(&self, entity_id: String) -> Result<Vec<Asset>, CoreError> {
        self.list_assets_unchecked(entity_id)
    }

    pub fn asset(&self, asset_id: String) -> Result<Asset, CoreError> {
        self.asset_unchecked(&asset_id)
    }

    /// Reads the live content-addressed bytes for a registered asset. Portable
    /// paths are checkpoint output and may not exist yet immediately after a
    /// runtime mutation.
    pub fn asset_bytes(&self, asset_id: String) -> Result<Vec<u8>, CoreError> {
        let asset = self.asset_unchecked(&asset_id)?;
        self.read_asset_bytes(&asset)
    }

    pub(crate) fn asset_unchecked(&self, asset_id: &str) -> Result<Asset, CoreError> {
        let mut asset = self
            .connection
            .query_row(
                "SELECT id,entity_id,namespace,filename,content_hash,size,mime_type,path,created_at,role,reference_scope,provenance FROM assets WHERE id=?1",
                params![asset_id],
                |row| {
                    Ok(Asset {
                        id: row.get(0)?,
                        entity_id: row.get(1)?,
                        namespace: row.get(2)?,
                        filename: row.get(3)?,
                        content_hash: row.get(4)?,
                        size: row.get(5)?,
                        mime_type: row.get(6)?,
                        path: row.get(7)?,
                        created_at: row.get(8)?,
                        role: row.get(9)?,
                        reference_scope: row.get(10)?,
                        provenance: decode_asset_provenance(row, 11)?,
                        revision: String::new(),
                    })
                },
            )
            .optional()?
            .ok_or_else(|| CoreError::NotFound("asset not found".into()))?;
        asset.revision = self.revision_for_asset(&asset.id)?;
        Ok(asset)
    }

    pub fn replace_asset_bytes_with_request(
        &self,
        input: AssetReplaceInput,
        bytes: Vec<u8>,
        expected_revision: &str,
        request_id: Option<&str>,
    ) -> Result<Asset, CoreError> {
        let input_fingerprint = digest_bytes(
            &serde_json::to_vec(&serde_json::json!({
                "input": input,
                "expectedRevision": expected_revision,
            }))
            .map_err(|error| CoreError::Serialization(error.to_string()))?,
        );
        if let Some(mut asset) =
            self.committed_mutation_with_fingerprint::<Asset>(request_id, Some(&input_fingerprint))?
        {
            asset.revision = self.revision_for_asset(&asset.id)?;
            return Ok(asset);
        }
        if input.size < 0 || input.size as usize != bytes.len() {
            return Err(CoreError::Validation(
                "asset replacement size does not match declared size".into(),
            ));
        }
        let digest = format!("sha256:{:x}", Sha256::digest(&bytes));
        if input.content_hash != digest {
            return Err(CoreError::Validation(
                "asset replacement content hash does not match bytes".into(),
            ));
        }
        let asset = self.asset_unchecked(&input.asset_id)?;
        Self::ensure_expected_revision(Some(expected_revision), asset.revision.clone(), "asset")?;
        if input.mime_type.trim().is_empty() {
            return Err(CoreError::Validation(
                "asset replacement MIME type is required".into(),
            ));
        }
        if asset.role == ASSET_ROLE_PROFILE && !asset_can_be_profile_media(&input.mime_type) {
            return Err(CoreError::Validation(
                "profile assets must use a supported raster image MIME type".into(),
            ));
        }
        if let Some((width, height)) = self.raster_layer_expected_size(&asset)? {
            if input.mime_type != "image/png" {
                return Err(CoreError::Validation(
                    "maps: painted layers must remain PNG assets".into(),
                ));
            }
            crate::maps::validate_raster_png(&bytes, width, height)?;
        } else if self.is_immutable_image_source(&asset)? {
            return Err(CoreError::Validation(
                "maps: imported source assets cannot be replaced".into(),
            ));
        }
        let request_id = self.request_id(request_id)?;
        if let Some(root) = self.root.as_deref() {
            let (stored_hash, stored_size) = store_runtime_asset(root, bytes.as_slice(), None)?;
            if stored_hash != input.content_hash || stored_size != input.size {
                return Err(CoreError::Validation(
                    "asset replacement bytes do not match metadata".into(),
                ));
            }
        }
        self.commit_asset_replacement(asset, input, &request_id, &input_fingerprint)
    }

    pub fn replace_asset_file_with_request(
        &self,
        input: AssetFileReplaceInput,
        expected_revision: &str,
        request_id: Option<&str>,
    ) -> Result<Asset, CoreError> {
        let input_fingerprint = digest_bytes(
            &serde_json::to_vec(&serde_json::json!({
                "input": input,
                "expectedRevision": expected_revision,
            }))
            .map_err(|error| CoreError::Serialization(error.to_string()))?,
        );
        if let Some(mut asset) =
            self.committed_mutation_with_fingerprint::<Asset>(request_id, Some(&input_fingerprint))?
        {
            asset.revision = self.revision_for_asset(&asset.id)?;
            return Ok(asset);
        }
        let source = Path::new(&input.source_path);
        let metadata =
            std::fs::metadata(source).map_err(|error| CoreError::NotFound(error.to_string()))?;
        if !metadata.is_file() {
            return Err(CoreError::NotFound(
                "asset replacement source is not a file".into(),
            ));
        }
        if input.mime_type.trim().is_empty() {
            return Err(CoreError::Validation(
                "asset replacement MIME type is required".into(),
            ));
        }
        let asset = self.asset_unchecked(&input.asset_id)?;
        Self::ensure_expected_revision(Some(expected_revision), asset.revision.clone(), "asset")?;
        if asset.role == ASSET_ROLE_PROFILE && !asset_can_be_profile_media(&input.mime_type) {
            return Err(CoreError::Validation(
                "profile assets must use a supported raster image MIME type".into(),
            ));
        }
        if let Some((width, height)) = self.raster_layer_expected_size(&asset)? {
            if input.mime_type != "image/png" {
                return Err(CoreError::Validation(
                    "maps: painted layers must remain PNG assets".into(),
                ));
            }
            let bytes = std::fs::read(source).map_err(|source| CoreError::Io {
                operation: "read raster layer replacement",
                source,
            })?;
            crate::maps::validate_raster_png(&bytes, width, height)?;
        } else if self.is_immutable_image_source(&asset)? {
            return Err(CoreError::Validation(
                "maps: imported source assets cannot be replaced".into(),
            ));
        }
        let (content_hash, size) = if let Some(root) = self.root.as_deref() {
            store_runtime_asset_file(root, source, None)?
        } else {
            streamed_file_digest(source)?
        };
        let request_id = self.request_id(request_id)?;
        self.commit_asset_replacement(
            asset,
            AssetReplaceInput {
                asset_id: input.asset_id,
                content_hash,
                size,
                mime_type: input.mime_type,
            },
            &request_id,
            &input_fingerprint,
        )
    }

    pub(crate) fn commit_asset_replacement(
        &self,
        mut asset: Asset,
        input: AssetReplaceInput,
        request_id: &str,
        input_fingerprint: &str,
    ) -> Result<Asset, CoreError> {
        asset.content_hash = input.content_hash.clone();
        asset.size = input.size;
        asset.mime_type = input.mime_type.clone();
        asset.revision.clear();
        let result = serde_json::to_value(&asset)
            .map_err(|error| CoreError::Serialization(error.to_string()))?;
        let transaction = match self.begin_mutation_with_fingerprint(
            request_id,
            Some(&result),
            &[format!("entities/{}/", asset.entity_id), asset.path.clone()],
            input_fingerprint,
        ) {
            Ok(value) => value,
            Err(error) => {
                return Err(error);
            }
        };
        let mutation = (|| -> Result<(), CoreError> {
            transaction.execute(
                "UPDATE assets SET content_hash=?1,size=?2,mime_type=?3 WHERE id=?4",
                params![
                    input.content_hash,
                    input.size,
                    input.mime_type,
                    input.asset_id
                ],
            )?;
            transaction.commit()?;
            Ok(())
        })();
        mutation?;
        self.refresh_maps_projection_for_entities(std::slice::from_ref(&asset.entity_id))?;
        self.notify_export_worker()?;
        asset.revision = self.revision_for_asset(&asset.id)?;
        Ok(asset)
    }

    pub fn update_asset_metadata_with_request(
        &self,
        input: AssetMetadataUpdate,
        expected_revision: &str,
        request_id: Option<&str>,
    ) -> Result<Asset, CoreError> {
        let input_fingerprint = digest_bytes(
            &serde_json::to_vec(&serde_json::json!({
                "input": input,
                "expectedRevision": expected_revision,
            }))
            .map_err(|error| CoreError::Serialization(error.to_string()))?,
        );
        if let Some(mut asset) =
            self.committed_mutation_with_fingerprint::<Asset>(request_id, Some(&input_fingerprint))?
        {
            asset.revision = self.revision_for_asset(&asset.id)?;
            return Ok(asset);
        }

        let mut asset = self.asset_unchecked(&input.asset_id)?;
        Self::ensure_expected_revision(Some(expected_revision), asset.revision.clone(), "asset")?;

        if let Some(filename) = input.filename.as_deref() {
            let filename = validated_asset_filename(filename)?;
            if filename != asset.filename {
                asset.path = renamed_asset_path(&asset, &filename)?;
                asset.filename = filename;
            }
        }
        if let Some(role) = input.role.as_deref() {
            validate_asset_role(role)?;
            if role == ASSET_ROLE_PROFILE && !asset_can_be_profile_media(&asset.mime_type) {
                return Err(CoreError::Validation(
                    "profile assets must use a supported raster image MIME type".into(),
                ));
            }
            asset.role = role.into();
        }
        if let Some(reference_scope) = input.reference_scope.as_deref() {
            validate_asset_reference_scope(reference_scope)?;
            asset.reference_scope = reference_scope.into();
        }

        let request_id = self.request_id(request_id)?;
        let current = self.asset_unchecked(&input.asset_id)?;
        let changed = asset.filename != current.filename
            || asset.path != current.path
            || asset.role != current.role
            || asset.reference_scope != current.reference_scope;
        asset.revision.clear();
        let result = serde_json::to_value(&asset)
            .map_err(|error| CoreError::Serialization(error.to_string()))?;
        let affected_paths = [
            format!("entities/{}/", asset.entity_id),
            current.path.clone(),
            asset.path.clone(),
        ];
        let transaction = self.begin_mutation_with_fingerprint(
            &request_id,
            Some(&result),
            &affected_paths,
            &input_fingerprint,
        )?;
        if changed {
            if asset.role == ASSET_ROLE_PROFILE {
                transaction.execute(
                    "UPDATE assets SET role=?1 WHERE entity_id=?2 AND namespace=?3 AND role=?4 AND id<>?5",
                    params![
                        ASSET_ROLE_ATTACHMENT,
                        asset.entity_id,
                        asset.namespace,
                        ASSET_ROLE_PROFILE,
                        asset.id
                    ],
                )?;
            }
            transaction.execute(
                "UPDATE assets SET filename=?1,path=?2,role=?3,reference_scope=?4 WHERE id=?5",
                params![
                    asset.filename,
                    asset.path,
                    asset.role,
                    asset.reference_scope,
                    asset.id
                ],
            )?;
        }
        transaction.commit()?;

        if changed {
            self.refresh_maps_projection_for_entities(std::slice::from_ref(&asset.entity_id))?;
            self.notify_export_worker()?;
        }
        asset.revision = self.revision_for_asset(&asset.id)?;
        Ok(asset)
    }

    pub fn delete_asset_with_request(
        &self,
        asset_id: String,
        expected_revision: &str,
        request_id: Option<&str>,
    ) -> Result<(), CoreError> {
        let input_fingerprint = digest_bytes(
            &serde_json::to_vec(&serde_json::json!({
                "assetId": asset_id,
                "expectedRevision": expected_revision,
            }))
            .map_err(|error| CoreError::Serialization(error.to_string()))?,
        );
        if self
            .committed_mutation_with_fingerprint::<serde_json::Value>(
                request_id,
                Some(&input_fingerprint),
            )?
            .is_some()
        {
            return Ok(());
        }
        let asset = self.asset_unchecked(&asset_id)?;
        Self::ensure_expected_revision(Some(expected_revision), asset.revision.clone(), "asset")?;
        let request_id = self.request_id(request_id)?;
        let transaction = self.begin_mutation_with_fingerprint(
            &request_id,
            Some(&serde_json::Value::Null),
            &[format!("entities/{}/", asset.entity_id), asset.path.clone()],
            &input_fingerprint,
        )?;
        let deleted = transaction.execute("DELETE FROM assets WHERE id=?1", params![asset_id])?;
        if deleted == 0 {
            return Err(CoreError::NotFound("asset not found".into()));
        }
        transaction.commit()?;
        self.refresh_maps_projection_for_entities(std::slice::from_ref(&asset.entity_id))?;
        self.notify_export_worker()?;
        Ok(())
    }

    pub fn list_shared_assets(&self) -> Result<Vec<Asset>, CoreError> {
        let mut statement = self.connection.prepare(
            "SELECT id,entity_id,namespace,filename,content_hash,size,mime_type,path,created_at,role,reference_scope,provenance FROM assets WHERE reference_scope='project' ORDER BY created_at DESC LIMIT 200",
        )?;
        let rows = statement.query_map([], |row| {
            Ok(Asset {
                id: row.get(0)?,
                entity_id: row.get(1)?,
                namespace: row.get(2)?,
                filename: row.get(3)?,
                content_hash: row.get(4)?,
                size: row.get(5)?,
                mime_type: row.get(6)?,
                path: row.get(7)?,
                created_at: row.get(8)?,
                role: row.get(9)?,
                reference_scope: row.get(10)?,
                provenance: decode_asset_provenance(row, 11)?,
                revision: String::new(),
            })
        })?;
        let mut assets = rows.collect::<Result<Vec<_>, _>>()?;
        for asset in &mut assets {
            asset.revision = self.revision_for_asset_value(asset)?;
        }
        Ok(assets)
    }

    pub fn asset_by_path(&self, path: String) -> Result<Asset, CoreError> {
        if !path.starts_with("assets/")
            || path.contains("..")
            || path.contains('\0')
            || path.len() > 1024
        {
            return Err(CoreError::Validation("invalid asset path".into()));
        }
        let mut asset = self
            .connection
            .query_row(
                "SELECT id,entity_id,namespace,filename,content_hash,size,mime_type,path,created_at,role,reference_scope,provenance FROM assets WHERE path=?1",
                params![path],
                |row| {
                    Ok(Asset {
                        id: row.get(0)?,
                        entity_id: row.get(1)?,
                        namespace: row.get(2)?,
                        filename: row.get(3)?,
                        content_hash: row.get(4)?,
                        size: row.get(5)?,
                        mime_type: row.get(6)?,
                        path: row.get(7)?,
                        created_at: row.get(8)?,
                        role: row.get(9)?,
                        reference_scope: row.get(10)?,
                        provenance: decode_asset_provenance(row, 11)?,
                        revision: String::new(),
                    })
                },
            )
            .optional()?
            .ok_or_else(|| CoreError::NotFound("asset not found".into()))?;
        asset.revision = self.revision_for_asset(&asset.id)?;
        Ok(asset)
    }

    pub fn asset_bytes_by_path(&self, path: String) -> Result<Vec<u8>, CoreError> {
        let asset = self.asset_by_path(path)?;
        self.read_asset_bytes(&asset)
    }

    pub(crate) fn list_assets_unchecked(&self, entity_id: String) -> Result<Vec<Asset>, CoreError> {
        let mut statement = self.connection.prepare("SELECT id,entity_id,namespace,filename,content_hash,size,mime_type,path,created_at,role,reference_scope,provenance FROM assets WHERE entity_id=?1 ORDER BY created_at")?;
        let rows = statement.query_map(params![entity_id], |row| {
            Ok(Asset {
                id: row.get(0)?,
                entity_id: row.get(1)?,
                namespace: row.get(2)?,
                filename: row.get(3)?,
                content_hash: row.get(4)?,
                size: row.get(5)?,
                mime_type: row.get(6)?,
                path: row.get(7)?,
                created_at: row.get(8)?,
                role: row.get(9)?,
                reference_scope: row.get(10)?,
                provenance: decode_asset_provenance(row, 11)?,
                revision: String::new(),
            })
        })?;
        let mut assets = rows.collect::<Result<Vec<_>, _>>()?;
        for asset in &mut assets {
            asset.revision = self.revision_for_asset_value(asset)?;
        }
        Ok(assets)
    }

    pub fn export_wiki_page_to(
        &self,
        entity_id: &str,
        destination: impl AsRef<Path>,
        format: WikiPageExportFormat,
        manifest: &PluginManifest,
    ) -> Result<String, CoreError> {
        let entities = self.list_entities()?;
        let entity = entities
            .iter()
            .find(|entity| entity.id == entity_id && !entity.deleted)
            .ok_or_else(|| CoreError::Validation("wiki article not found".into()))?;
        let allowed_types = manifest
            .schemas
            .iter()
            .flat_map(|schema| {
                schema
                    .entity_types
                    .iter()
                    .map(|entity_type| &entity_type.id)
            })
            .collect::<BTreeSet<_>>();
        if entity
            .entity_type
            .as_ref()
            .is_none_or(|entity_type| !allowed_types.contains(entity_type))
        {
            return Err(CoreError::Validation(
                "entity is not part of the selected wiki manifest".into(),
            ));
        }

        let document = self.list_documents(entity.id.clone())?.into_iter().next();
        if document
            .as_ref()
            .is_some_and(|document| document.format != "markdown")
        {
            return Err(CoreError::Validation(
                "wiki page export requires a Markdown document".into(),
            ));
        }
        let body = document
            .map(|document| rewrite_markdown_entity_links_as_labels(&document.body))
            .unwrap_or_default();

        let mut fields = self
            .list_fields(entity.id.clone())?
            .into_iter()
            .filter_map(|field| {
                let label = wiki_field_label(
                    manifest,
                    &field.namespace,
                    &field.key,
                    entity.entity_type.as_deref(),
                )?;
                let value = wiki_display_value(&field.value);
                (!value.trim().is_empty()).then_some((label, field.namespace, field.key, value))
            })
            .collect::<Vec<_>>();
        fields.sort_by(|left, right| {
            left.0
                .to_lowercase()
                .cmp(&right.0.to_lowercase())
                .then_with(|| left.1.cmp(&right.1))
                .then_with(|| left.2.cmp(&right.2))
        });

        let names = entities
            .iter()
            .map(|entity| (entity.id.as_str(), entity.name.as_str()))
            .collect::<BTreeMap<_, _>>();
        let relationships = self.list_relationships(entity.id.clone())?;
        let mut outbound = relationships
            .iter()
            .filter(|relationship| relationship.source_id == entity.id)
            .map(|relationship| {
                (
                    wiki_relationship_label(manifest, &relationship.relationship_type),
                    names
                        .get(relationship.target_id.as_str())
                        .copied()
                        .unwrap_or(relationship.target_id.as_str()),
                )
            })
            .collect::<Vec<_>>();
        let mut inbound = relationships
            .iter()
            .filter(|relationship| relationship.target_id == entity.id)
            .map(|relationship| {
                (
                    wiki_relationship_label(manifest, &relationship.relationship_type),
                    names
                        .get(relationship.source_id.as_str())
                        .copied()
                        .unwrap_or(relationship.source_id.as_str()),
                )
            })
            .collect::<Vec<_>>();
        outbound.sort();
        inbound.sort();

        let mut attachments = self
            .list_assets(entity.id.clone())?
            .into_iter()
            .map(|asset| (asset.filename, asset.mime_type, asset.size))
            .collect::<Vec<_>>();
        attachments.sort_by_key(|left| left.0.to_lowercase());

        let mut markdown = format!(
            "# {}\n\n*{}*\n",
            markdown_escape_label(&entity.name),
            markdown_escape_label(&wiki_entity_type_label(
                manifest,
                entity.entity_type.as_deref()
            ))
        );
        if !fields.is_empty() {
            markdown.push_str("\n## Details\n\n| Field | Value |\n| --- | --- |\n");
            for (label, _, _, value) in &fields {
                markdown.push_str("| ");
                markdown.push_str(&label.replace('|', "\\|").replace('\n', " "));
                markdown.push_str(" | ");
                markdown.push_str(&value.replace('|', "\\|").replace('\n', "<br>"));
                markdown.push_str(" |\n");
            }
        }
        if !body.trim().is_empty() {
            markdown.push('\n');
            markdown.push_str(body.trim());
            markdown.push('\n');
        }
        if !outbound.is_empty() || !inbound.is_empty() {
            markdown.push_str("\n## Connections\n");
            if !outbound.is_empty() {
                markdown.push_str("\n### From this page\n\n");
                for (label, name) in &outbound {
                    markdown.push_str(&format!(
                        "- **{}:** {}\n",
                        markdown_escape_label(label),
                        markdown_escape_label(name)
                    ));
                }
            }
            if !inbound.is_empty() {
                markdown.push_str("\n### Links here\n\n");
                for (label, name) in &inbound {
                    markdown.push_str(&format!(
                        "- **{}:** {}\n",
                        markdown_escape_label(label),
                        markdown_escape_label(name)
                    ));
                }
            }
        }
        if !attachments.is_empty() {
            markdown.push_str("\n## Attachments\n\n");
            for (filename, mime_type, size) in &attachments {
                markdown.push_str(&format!(
                    "- {} — {} ({} KB)\n",
                    markdown_escape_label(filename),
                    markdown_escape_label(mime_type),
                    (size + 1023) / 1024
                ));
            }
        }

        let destination = destination.as_ref();
        std::fs::create_dir_all(destination).map_err(|source| CoreError::Io {
            operation: "create wiki export destination",
            source,
        })?;
        let stem = markdown_export_stem(&entity.name);
        let (extension, bytes) = match format {
            WikiPageExportFormat::Markdown => ("md", markdown.into_bytes()),
            WikiPageExportFormat::Html => {
                let article = markdown_to_safe_html(&markdown);
                let title = html_escape(&entity.name);
                let html = format!(
                    "<!doctype html><html lang=\"en\"><head><meta charset=\"utf-8\"><meta name=\"viewport\" content=\"width=device-width,initial-scale=1\"><title>{title}</title><style>{}</style></head><body><main class=\"article\">{article}</main></body></html>",
                    "html{color-scheme:light}*{box-sizing:border-box}body{margin:0;background:#f5f3ee;color:#272923;font:16px/1.7 ui-sans-serif,system-ui,-apple-system,BlinkMacSystemFont,\"Segoe UI\",sans-serif}.article{width:min(820px,calc(100% - 40px));margin:48px auto;padding:clamp(28px,6vw,72px);border:1px solid #dedbd2;border-radius:18px;background:#fff;box-shadow:0 18px 55px rgba(44,45,38,.08)}h1,h2,h3{color:#20231e;font-family:ui-serif,Georgia,serif;line-height:1.18}h1{font-size:2.5rem;margin-top:0}h2{margin-top:2.2em;padding-top:.7em;border-top:1px solid #e6e3db}a{color:#35614a}blockquote{margin-left:0;padding-left:1rem;border-left:3px solid #b4773f;color:#666}table{width:100%;border-collapse:collapse;margin:1.5rem 0;font-size:.92rem}th,td{padding:.65rem .75rem;border:1px solid #dfddd5;text-align:left;vertical-align:top}th{background:#f5f3ee}pre{overflow:auto;padding:1rem;border-radius:10px;background:#f3f1eb}img{max-width:100%;height:auto}@media print{body{background:#fff}.article{width:auto;margin:0;padding:0;border:0;box-shadow:none}h2,h3{break-after:avoid}table,pre,blockquote{break-inside:avoid}}"
                );
                ("html", html.into_bytes())
            }
        };
        let target = wiki_export_target(destination, &stem, extension);
        std::fs::write(&target, bytes).map_err(|source| CoreError::Io {
            operation: "write wiki page export",
            source,
        })?;
        Ok(target.to_string_lossy().into_owned())
    }

    pub fn export_markdown_to(&self, destination: impl AsRef<Path>) -> Result<String, CoreError> {
        let mut entities = self.list_entities()?;
        entities.sort_by(|left, right| {
            left.name
                .to_lowercase()
                .cmp(&right.name.to_lowercase())
                .then_with(|| left.id.cmp(&right.id))
        });

        let mut proposed = entities
            .iter()
            .map(|entity| format!("{}.md", markdown_export_stem(&entity.name)))
            .collect::<Vec<_>>();
        let mut collisions: BTreeMap<String, Vec<usize>> = BTreeMap::new();
        for (index, filename) in proposed.iter().enumerate() {
            collisions
                .entry(filename.to_lowercase())
                .or_default()
                .push(index);
        }
        for indexes in collisions.values().filter(|indexes| indexes.len() > 1) {
            for index in indexes {
                let suffix = entities[*index].id.chars().take(8).collect::<String>();
                proposed[*index] = format!(
                    "{}-{suffix}.md",
                    markdown_export_stem(&entities[*index].name)
                );
            }
        }

        let filenames = entities
            .iter()
            .zip(proposed.iter())
            .map(|(entity, filename)| (entity.id.clone(), filename.clone()))
            .collect::<BTreeMap<_, _>>();
        let names = entities
            .iter()
            .map(|entity| (entity.id.clone(), entity.name.clone()))
            .collect::<BTreeMap<_, _>>();

        let destination = destination.as_ref();
        std::fs::create_dir_all(destination).map_err(|source| CoreError::Io {
            operation: "create Markdown export destination",
            source,
        })?;
        let project_name = self
            .info()
            .map_or_else(|| "Daena Archive".into(), |info| info.name);
        let export_stem = markdown_export_stem(&project_name);
        let mut export_directory = destination.join(format!("{export_stem}-markdown"));
        let mut suffix = 2;
        while export_directory.exists() {
            export_directory = destination.join(format!("{export_stem}-markdown-{suffix}"));
            suffix += 1;
        }
        std::fs::create_dir(&export_directory).map_err(|source| CoreError::Io {
            operation: "create Markdown export directory",
            source,
        })?;

        for (entity, filename) in entities.iter().zip(proposed.iter()) {
            let documents = self.list_documents(entity.id.clone())?;
            let document = documents.into_iter().next();
            if let Some(document) = &document {
                if document.format != "markdown" {
                    return Err(CoreError::Validation(
                        "Markdown export requires Markdown documents".into(),
                    ));
                }
            }
            let body = document.map(|document| document.body).unwrap_or_default();
            let mut markdown = rewrite_markdown_entity_links(&body, &filenames);
            markdown.push_str("\n\n## Relationships\n\n");

            let mut grouped: BTreeMap<String, Vec<(String, Option<String>)>> = BTreeMap::new();
            for relationship in self
                .list_relationships(entity.id.clone())?
                .into_iter()
                .filter(|relationship| relationship.source_id == entity.id)
            {
                let target_name = names
                    .get(&relationship.target_id)
                    .cloned()
                    .unwrap_or_else(|| relationship.target_id.clone());
                let target_filename = filenames.get(&relationship.target_id).cloned();
                grouped
                    .entry(relationship.relationship_type)
                    .or_default()
                    .push((target_name, target_filename));
            }

            if grouped.is_empty() {
                markdown.push_str("_No outgoing relationships._\n");
            } else {
                for (relationship_type, mut targets) in grouped {
                    targets.sort_by(|left, right| {
                        left.0.cmp(&right.0).then_with(|| left.1.cmp(&right.1))
                    });
                    markdown.push_str("### ");
                    markdown.push_str(&markdown_relationship_heading(&relationship_type));
                    markdown.push_str("\n\n");
                    for (target_name, target_filename) in targets {
                        markdown.push_str("- ");
                        if let Some(target_filename) = target_filename {
                            markdown.push('[');
                            markdown.push_str(&markdown_escape_label(&target_name));
                            markdown.push_str("](");
                            markdown.push_str(&markdown_export_target(&target_filename));
                            markdown.push_str(")\n");
                        } else {
                            markdown.push_str(&markdown_escape_label(&target_name));
                            markdown.push('\n');
                        }
                    }
                    markdown.push('\n');
                }
            }

            std::fs::write(export_directory.join(filename), markdown).map_err(|source| {
                CoreError::Io {
                    operation: "write Markdown export file",
                    source,
                }
            })?;
        }

        Ok(export_directory.to_string_lossy().into_owned())
    }
}
