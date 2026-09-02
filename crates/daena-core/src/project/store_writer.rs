// Mutation transactions and receipts.
use super::*;

impl ProjectStore {
    pub(crate) fn notify_export_worker(&self) -> Result<(), CoreError> {
        if self.suppress_sync.get() {
            return Ok(());
        }
        if self.root.is_some() {
            if self.export_worker.is_some() {
                self.wake_export_worker();
            } else {
                self.export_latest_generation()?;
            }
        }
        Ok(())
    }

    pub(crate) fn begin_mutation<'a>(
        &'a self,
        request_id: &str,
        result: Option<&serde_json::Value>,
        affected_prefixes: &[String],
    ) -> Result<rusqlite::Transaction<'a>, CoreError> {
        let fingerprint = result.map_or_else(
            || digest_bytes(b"null"),
            |value| digest_bytes(value.to_string().as_bytes()),
        );
        self.begin_mutation_with_fingerprint(request_id, result, affected_prefixes, &fingerprint)
    }

    pub(crate) fn begin_mutation_with_fingerprint<'a>(
        &'a self,
        request_id: &str,
        result: Option<&serde_json::Value>,
        affected_prefixes: &[String],
        fingerprint: &str,
    ) -> Result<rusqlite::Transaction<'a>, CoreError> {
        let transaction = self.connection.unchecked_transaction()?;
        let _ = affected_prefixes;
        let result_json = result.map_or_else(|| "null".into(), serde_json::Value::to_string);
        let stored_fingerprint: Option<String> = transaction
            .query_row(
                "SELECT fingerprint FROM mutation_receipts WHERE request_id=?1",
                params![request_id],
                |row| row.get(0),
            )
            .optional()?;
        if let Some(stored_fingerprint) = stored_fingerprint {
            if stored_fingerprint != fingerprint {
                return Err(CoreError::Conflict(
                    "request ID was reused with different inputs".into(),
                ));
            }
        }
        transaction.execute(
            "INSERT INTO mutation_receipts(request_id,result,fingerprint,committed_at,state) VALUES (?1,?2,?3,?4,'completed') ON CONFLICT(request_id) DO UPDATE SET result=excluded.result,fingerprint=excluded.fingerprint,committed_at=excluded.committed_at,state='completed'",
            params![request_id, result_json, fingerprint, chrono_like_now()],
        )?;
        Ok(transaction)
    }

    pub(crate) fn write_mutation_result(
        &self,
        request_id: &str,
        result: &serde_json::Value,
    ) -> Result<(), CoreError> {
        self.connection.execute(
            "UPDATE mutation_receipts SET result=?1 WHERE request_id=?2",
            params![result.to_string(), request_id],
        )?;
        Ok(())
    }

    pub(crate) fn export_complete_snapshot(
        &self,
        root: &Path,
        manifest: &crate::storage::ProjectManifest,
        snapshot: &ProjectSnapshot,
        target_generation: Generation,
    ) -> Result<usize, CoreError> {
        let manifest_path = root.join("project.json");
        manifest.validate(&manifest_path)?;
        let previous_sources = Self::checkpoint_sources(root)?;
        let request_id = Uuid::new_v4().to_string();
        let mut transaction = crate::sync::SyncExporter::begin(root, &request_id)?;
        let staging_root = transaction.staging_root();
        std::fs::create_dir_all(&staging_root).map_err(|error| CoreError::Io {
            operation: "create checkpoint staging root",
            source: error,
        })?;
        crate::storage::write_json(&staging_root.join("project.json"), manifest)?;
        let mut documents_by_entity = BTreeMap::new();
        for document in &snapshot.documents {
            documents_by_entity
                .entry(document.entity_id.as_str())
                .or_insert(document);
        }
        let mut fields_by_entity = BTreeMap::<&str, Vec<&FieldValue>>::new();
        for field in &snapshot.fields {
            fields_by_entity
                .entry(field.entity_id.as_str())
                .or_default()
                .push(field);
        }
        let mut relationships_by_entity = BTreeMap::<&str, Vec<&Relationship>>::new();
        for relationship in &snapshot.relationships {
            relationships_by_entity
                .entry(relationship.source_id.as_str())
                .or_default()
                .push(relationship);
        }
        let mut assets_by_entity = BTreeMap::<&str, Vec<&Asset>>::new();
        for asset in &snapshot.assets {
            assets_by_entity
                .entry(asset.entity_id.as_str())
                .or_default()
                .push(asset);
        }
        let namespace_owners = crate::storage::namespace_owners(snapshot)?;
        for entity in &snapshot.entities {
            crate::storage::write_canonical_entity(
                &staging_root,
                entity,
                documents_by_entity.get(entity.id.as_str()).copied(),
                fields_by_entity
                    .get(entity.id.as_str())
                    .map(Vec::as_slice)
                    .unwrap_or_default(),
                relationships_by_entity
                    .get(entity.id.as_str())
                    .map(Vec::as_slice)
                    .unwrap_or_default(),
                assets_by_entity
                    .get(entity.id.as_str())
                    .map(Vec::as_slice)
                    .unwrap_or_default(),
                &namespace_owners,
            )?;
        }
        let plugin_ids = snapshot
            .modules
            .iter()
            .map(|module| module.module_id.as_str())
            .chain(
                snapshot
                    .module_records
                    .iter()
                    .map(|record| record.module_id.as_str()),
            )
            .collect::<BTreeSet<_>>();
        for plugin_id in plugin_ids {
            crate::storage::write_canonical_plugin(&staging_root, manifest, snapshot, plugin_id)?;
        }
        let mut current_sources = staged_canonical_sources(&staging_root, snapshot)?;
        let mut transaction_staged_paths = BTreeSet::new();
        let mut current_path_set = current_sources
            .iter()
            .map(|source| source.path.clone())
            .collect::<BTreeSet<_>>();
        for asset in &snapshot.assets {
            if current_path_set.contains(&asset.path) {
                continue;
            }
            let source = runtime_asset_path(root, &asset.content_hash)?;
            let (runtime_hash, runtime_size) = streamed_file_digest(&source)?;
            if runtime_hash != asset.content_hash || runtime_size != asset.size {
                return Err(CoreError::Conflict(format!(
                    "runtime asset bytes do not match metadata for {}",
                    asset.path
                )));
            }
            let portable_hash = crate::sync::hash_path(root, &asset.path)?;
            if portable_hash.as_deref() != Some(asset.content_hash.as_str()) {
                transaction.stage_file_with_expected(&asset.path, &source, portable_hash)?;
                transaction_staged_paths.insert(asset.path.clone());
            }
            current_path_set.insert(asset.path.clone());
            current_sources.push(crate::storage::CanonicalSource {
                path: asset.path.clone(),
                content_hash: runtime_hash,
                format_version: crate::storage::PROJECT_FORMAT_VERSION,
            });
        }
        let project_manifest_hash = crate::sync::hash_path(&staging_root, "project.json")?
            .ok_or_else(|| CoreError::Validation("staged project manifest is missing".into()))?;
        current_sources.push(crate::storage::CanonicalSource {
            path: "project.json".into(),
            content_hash: project_manifest_hash,
            format_version: crate::storage::PROJECT_FORMAT_VERSION,
        });
        current_sources.sort_by(|left, right| left.path.cmp(&right.path));
        current_sources.dedup_by(|left, right| left.path == right.path);
        let current_paths = current_sources
            .iter()
            .map(|source| source.path.as_str())
            .collect::<BTreeSet<_>>();
        for source in &current_sources {
            if transaction_staged_paths.contains(&source.path) {
                continue;
            }
            let portable_hash = crate::sync::hash_path(root, &source.path)?;
            if portable_hash.as_deref() == Some(source.content_hash.as_str()) {
                continue;
            }
            let staged = crate::storage::normalized_project_path(&staging_root, &source.path)?;
            transaction.stage_file_with_expected(&source.path, &staged, portable_hash)?;
        }
        for source in &previous_sources {
            if source.path != "project.json" && !current_paths.contains(source.path.as_str()) {
                transaction.stage_remove(&source.path)?;
            }
        }
        let applied = transaction.commit(None::<&serde_json::Value>)?;
        // Incremental export stages file removals individually, which can leave
        // an empty `entities/<id>` directory behind. Clean up any entity
        // directories that are now stale (not in the current snapshot) and empty,
        // so `purge` is observed as a full folder removal.
        let entities_root = root.join("entities");
        if entities_root.is_dir() {
            if let Ok(entries) = std::fs::read_dir(&entities_root) {
                for entry in entries.flatten() {
                    let name = entry.file_name().to_string_lossy().into_owned();
                    if crate::storage::is_ignored_metadata_entry(&name) {
                        continue;
                    }
                    if snapshot.entities.iter().any(|e| e.id == name) {
                        continue;
                    }
                    let path = entry.path();
                    if path.is_dir() {
                        // Only remove if the directory is empty or only contains
                        // ignored metadata files.
                        let is_empty = std::fs::read_dir(&path)
                            .map(|mut iter| {
                                iter.all(|e| {
                                    e.map(|e| {
                                        crate::storage::is_ignored_metadata_entry(
                                            &e.file_name().to_string_lossy(),
                                        )
                                    })
                                    .unwrap_or(true)
                                })
                            })
                            .unwrap_or(true);
                        if is_empty {
                            let _ = std::fs::remove_dir_all(&path);
                        }
                    }
                }
            }
        }
        self.install_checkpoint_manifest_from_verified_sources(
            root,
            target_generation,
            &current_sources,
        )?;
        Ok(applied.len())
    }

    pub(crate) fn install_checkpoint_manifest_from_verified_sources(
        &self,
        root: &Path,
        generation: Generation,
        sources: &[crate::storage::CanonicalSource],
    ) -> Result<(), CoreError> {
        let checkpoint = crate::storage::build_checkpoint_manifest_from_verified_sources(
            root, generation, sources,
        )?;
        self.install_checkpoint_manifest_value(root, generation, &checkpoint)
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn install_checkpoint_manifest(
        &self,
        root: &Path,
        generation: Generation,
    ) -> Result<(), CoreError> {
        let checkpoint = crate::storage::build_checkpoint_manifest(root, generation)?;
        self.install_checkpoint_manifest_value(root, generation, &checkpoint)
    }

    pub(crate) fn install_checkpoint_manifest_value(
        &self,
        root: &Path,
        generation: Generation,
        checkpoint: &crate::storage::CheckpointManifest,
    ) -> Result<(), CoreError> {
        let digest =
            crate::storage::canonical_json_bytes(checkpoint).map(|bytes| digest_bytes(&bytes))?;
        crate::storage::write_checkpoint_manifest(root, checkpoint)?;
        let transaction = self.connection.unchecked_transaction()?;
        transaction.execute(
            "UPDATE runtime_meta SET exported_generation=CASE WHEN content_generation>=?1 THEN ?1 ELSE exported_generation END, checkpoint_digest=?2, export_error=NULL WHERE key='runtime'",
            params![generation, digest],
        )?;
        transaction.commit()?;
        Ok(())
    }

    pub(crate) fn committed_mutation<T: DeserializeOwned>(
        &self,
        request_id: Option<&str>,
    ) -> Result<Option<T>, CoreError> {
        self.committed_mutation_with_fingerprint(request_id, None)
    }

    pub(crate) fn committed_mutation_with_fingerprint<T: DeserializeOwned>(
        &self,
        request_id: Option<&str>,
        fingerprint: Option<&str>,
    ) -> Result<Option<T>, CoreError> {
        let Some(request_id) = request_id else {
            return Ok(None);
        };
        self.request_id(Some(request_id))?;
        let Some(_root) = self.root.as_deref() else {
            return Ok(None);
        };
        if let Some(fingerprint) = fingerprint {
            let stored: Option<String> = self
                .connection
                .query_row(
                    "SELECT fingerprint FROM mutation_receipts WHERE request_id=?1",
                    params![request_id],
                    |row| row.get(0),
                )
                .optional()?;
            if let Some(stored) = stored {
                if stored != fingerprint {
                    return Err(CoreError::Conflict(
                        "request ID was reused with different inputs".into(),
                    ));
                }
            }
        }
        let receipt: Option<(String, String)> = self
            .connection
            .query_row(
                "SELECT result,fingerprint FROM mutation_receipts WHERE request_id=?1 AND state='completed'",
                params![request_id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()?;
        let result = match receipt {
            Some((result, stored_fingerprint)) => {
                if let Some(fingerprint) = fingerprint {
                    if fingerprint != stored_fingerprint {
                        return Err(CoreError::Conflict(
                            "request ID was reused with different inputs".into(),
                        ));
                    }
                }
                result
            }
            None => return Ok(None),
        };
        let result = serde_json::from_str(&result)
            .map_err(|error| CoreError::Serialization(error.to_string()))?;
        serde_json::from_value(result)
            .map(Some)
            .map_err(|error| CoreError::Serialization(error.to_string()))
    }

    pub(crate) fn ensure_expected_revision(
        expected: Option<&str>,
        actual: String,
        record: &str,
    ) -> Result<(), CoreError> {
        if let Some(expected) = expected {
            if expected != actual {
                return Err(CoreError::Conflict(format!(
                    "{record} revision conflict: expected {expected}, current {actual}"
                )));
            }
        }
        Ok(())
    }
}
