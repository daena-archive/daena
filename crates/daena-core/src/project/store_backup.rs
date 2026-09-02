// Backup, restore, and snapshot import and export operations.
use super::*;

impl ProjectStore {
    pub fn export_json(&self) -> Result<String, CoreError> {
        self.export_json_inner()
    }

    pub(crate) fn export_json_inner(&self) -> Result<String, CoreError> {
        serde_json::to_string_pretty(&self.export_snapshot()?)
            .map_err(|error| CoreError::Serialization(error.to_string()))
    }

    pub(crate) fn export_snapshot(&self) -> Result<ProjectSnapshot, CoreError> {
        let entities = self
            .connection
            .prepare("SELECT id,name,entity_type,deleted,created_at,updated_at FROM entities ORDER BY name,id")?
            .query_map([], |row| {
                Ok(Entity {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    entity_type: row.get(2)?,
                    deleted: row.get::<_, i64>(3)? != 0,
                    created_at: row.get(4)?,
                    updated_at: row.get(5)?,
                    revision: String::new(),
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        let documents = self
            .connection
            .prepare("SELECT id,entity_id,format,body,updated_at FROM documents ORDER BY entity_id,updated_at DESC,id")?
            .query_map([], |row| {
                Ok(Document {
                    id: row.get(0)?,
                    entity_id: row.get(1)?,
                    format: row.get(2)?,
                    body: row.get(3)?,
                    updated_at: row.get(4)?,
                    revision: String::new(),
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        let fields = self
            .connection
            .prepare("SELECT entity_id,namespace,key,value FROM entity_fields ORDER BY entity_id,namespace,key")?
            .query_map([], |row| {
                let value: String = row.get(3)?;
                Ok(FieldValue {
                    entity_id: row.get(0)?,
                    namespace: row.get(1)?,
                    key: row.get(2)?,
                    value: decode_field_value(value),
                    revision: String::new(),
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        let relationships = self
            .connection
            .prepare("SELECT id,source_id,target_id,relationship_type,metadata FROM relationships ORDER BY id")?
            .query_map([], |row| {
                Ok(Relationship {
                    id: row.get(0)?,
                    source_id: row.get(1)?,
                    target_id: row.get(2)?,
                    relationship_type: row.get(3)?,
                    metadata: row.get(4)?,
                    revision: String::new(),
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        let assets = self
            .connection
            .prepare("SELECT id,entity_id,namespace,filename,content_hash,size,mime_type,path,created_at,role,reference_scope,provenance FROM assets ORDER BY entity_id,created_at,id")?
            .query_map([], |row| {
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
            })?
            .collect::<Result<Vec<_>, _>>()?;
        let mut module_statement = self.connection.prepare("SELECT m.module_id, COALESCE(s.enabled, 1), m.version, p.package_version, o.overlay_json FROM module_versions m LEFT JOIN module_state s ON s.module_id = m.module_id LEFT JOIN module_package_versions p ON p.module_id = m.module_id LEFT JOIN module_schema_overlays o ON o.module_id = m.module_id ORDER BY m.module_id")?;
        let modules = module_statement
            .query_map([], |row| {
                let overlay_json: Option<String> = row.get(4)?;
                let schema_overlay = overlay_json.and_then(|json| {
                    serde_json::from_str(&json)
                        .ok()
                        .filter(|value: &serde_json::Value| !value.is_null())
                });
                Ok(ModuleState {
                    module_id: row.get(0)?,
                    enabled: row.get::<_, i64>(1)? != 0,
                    version: row.get(2)?,
                    package_version: row.get(3)?,
                    schema_overlay,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        let module_namespaces = self
            .connection
            .prepare(
                "SELECT module_id,namespace FROM module_namespaces ORDER BY module_id,namespace",
            )?
            .query_map([], |row| {
                Ok(ModuleNamespace {
                    module_id: row.get(0)?,
                    namespace: row.get(1)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        let module_fields = self
            .connection
            .prepare("SELECT module_id,namespace,key,field_type,required FROM module_fields ORDER BY module_id,namespace,key")?
            .query_map([], |row| {
                Ok(ModuleField {
                    module_id: row.get(0)?,
                    namespace: row.get(1)?,
                    key: row.get(2)?,
                    field_type: row.get(3)?,
                    required: row.get::<_, i64>(4)? != 0,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        let module_records = self
            .connection
            .prepare("SELECT module_id,collection,id,owner_entity_id,value,created_at,updated_at FROM module_records ORDER BY module_id,collection,id")?
            .query_map([], |row| {
                let value: String = row.get(4)?;
                Ok(ModuleRecord {
                    module_id: row.get(0)?,
                    collection: row.get(1)?,
                    id: row.get(2)?,
                    owner_entity_id: row.get(3)?,
                    value: decode_field_value(value),
                    created_at: row.get(5)?,
                    updated_at: row.get(6)?,
                    revision: String::new(),
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        let migration_history = self
            .connection
            .prepare("SELECT module_id,migration_id,from_version,to_version,checksum,package_digest,applied_at FROM migration_history ORDER BY module_id,migration_id")?
            .query_map([], |row| {
                Ok(MigrationHistoryEntry {
                    module_id: row.get(0)?,
                    migration_id: row.get(1)?,
                    from_version: row.get(2)?,
                    to_version: row.get(3)?,
                    checksum: row.get(4)?,
                    package_digest: row.get(5)?,
                    applied_at: row.get(6)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(ProjectSnapshot {
            format_version: current_snapshot_version(),
            entities,
            documents,
            fields,
            relationships,
            assets,
            modules,
            module_namespaces,
            module_fields,
            module_records,
            migration_history,
        })
    }

    pub(crate) fn import_json_with_mode_and_sync_with_request(
        &self,
        payload: &str,
        replace: bool,
        sync_canonical: bool,
        request_id: Option<&str>,
    ) -> Result<usize, CoreError> {
        self.import_json_with_mode_and_sync_with_request_and_search(
            payload,
            replace,
            sync_canonical,
            request_id,
            true,
        )
    }

    pub(crate) fn import_json_with_mode_and_sync_with_request_and_search(
        &self,
        payload: &str,
        replace: bool,
        sync_canonical: bool,
        request_id: Option<&str>,
        rebuild_search: bool,
    ) -> Result<usize, CoreError> {
        let snapshot: ProjectSnapshot = serde_json::from_str(payload)
            .map_err(|error| CoreError::NotFound(error.to_string()))?;
        self.import_snapshot_with_mode_and_sync_with_request_and_search(
            &snapshot,
            replace,
            sync_canonical,
            request_id,
            rebuild_search,
        )
    }

    pub(crate) fn import_snapshot_with_mode_and_sync_with_request_and_search(
        &self,
        snapshot: &ProjectSnapshot,
        replace: bool,
        sync_canonical: bool,
        request_id: Option<&str>,
        rebuild_search: bool,
    ) -> Result<usize, CoreError> {
        if snapshot.format_version != current_snapshot_version() {
            return Err(CoreError::NotFound(format!(
                "unsupported project snapshot version {}",
                snapshot.format_version
            )));
        }
        if let Some(root) = self.root.as_deref() {
            for asset in &snapshot.assets {
                ensure_runtime_asset(root, &asset.path, &asset.content_hash, asset.size)?;
            }
        }
        let mutation_request_id = if sync_canonical {
            Some(self.request_id(request_id)?)
        } else {
            None
        };
        let transaction = if let Some(ref request_id) = mutation_request_id {
            self.begin_mutation(
                request_id,
                Some(&serde_json::Value::Null),
                &["entities/".into(), "plugins/".into()],
            )?
        } else {
            self.connection.unchecked_transaction()?
        };
        if replace {
            transaction.execute_batch(
                "DELETE FROM assets;
                 DELETE FROM entity_fields;
                 DELETE FROM documents;
                 DELETE FROM relationships;
                 DELETE FROM module_records;
                 DELETE FROM entities;
                 DELETE FROM module_state;
                 DELETE FROM module_versions;
                 DELETE FROM module_package_versions;
                 DELETE FROM module_fields;
                 DELETE FROM module_namespaces;
                 DELETE FROM module_schema_overlays;
                 DELETE FROM migration_history;",
            )?;
        }
        {
            let mut statement = transaction.prepare_cached("INSERT INTO entities(id,name,entity_type,deleted,created_at,updated_at) VALUES (?1,?2,?3,?4,?5,?6) ON CONFLICT(id) DO UPDATE SET name=excluded.name,entity_type=excluded.entity_type,deleted=excluded.deleted,created_at=excluded.created_at,updated_at=excluded.updated_at")?;
            for entity in &snapshot.entities {
                statement.execute(params![
                    entity.id,
                    entity.name,
                    entity.entity_type,
                    i64::from(entity.deleted),
                    entity.created_at,
                    entity.updated_at
                ])?;
            }
        }
        {
            let mut statement = transaction.prepare_cached("INSERT INTO documents(id,entity_id,format,body,updated_at) VALUES (?1,?2,?3,?4,?5) ON CONFLICT(id) DO UPDATE SET entity_id=excluded.entity_id,format=excluded.format,body=excluded.body,updated_at=excluded.updated_at")?;
            for document in &snapshot.documents {
                statement.execute(params![
                    document.id,
                    document.entity_id,
                    document.format,
                    document.body,
                    document.updated_at
                ])?;
            }
        }
        {
            let mut statement = transaction.prepare_cached("INSERT INTO entity_fields(entity_id,namespace,key,value) VALUES (?1,?2,?3,?4) ON CONFLICT(entity_id,namespace,key) DO UPDATE SET value=excluded.value")?;
            for field in &snapshot.fields {
                let value = encode_field_value(&field.value)?;
                statement.execute(params![field.entity_id, field.namespace, field.key, value])?;
            }
        }
        {
            let mut statement = transaction.prepare_cached("INSERT INTO relationships(id,source_id,target_id,relationship_type,metadata) VALUES (?1,?2,?3,?4,?5) ON CONFLICT(id) DO UPDATE SET source_id=excluded.source_id,target_id=excluded.target_id,relationship_type=excluded.relationship_type,metadata=excluded.metadata")?;
            for relationship in &snapshot.relationships {
                statement.execute(params![
                    relationship.id,
                    relationship.source_id,
                    relationship.target_id,
                    relationship.relationship_type,
                    relationship.metadata
                ])?;
            }
        }
        {
            let mut statement = transaction.prepare_cached("INSERT INTO assets(id,entity_id,namespace,filename,content_hash,size,mime_type,path,created_at,role,reference_scope,provenance) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12) ON CONFLICT(id) DO UPDATE SET entity_id=excluded.entity_id,namespace=excluded.namespace,filename=excluded.filename,content_hash=excluded.content_hash,size=excluded.size,mime_type=excluded.mime_type,path=excluded.path,created_at=excluded.created_at,role=excluded.role,reference_scope=excluded.reference_scope,provenance=excluded.provenance")?;
            for asset in &snapshot.assets {
                validate_asset_role(&asset.role)?;
                validate_asset_reference_scope(&asset.reference_scope)?;
                if asset.role == ASSET_ROLE_PROFILE && !asset_can_be_profile_media(&asset.mime_type)
                {
                    return Err(CoreError::Validation(
                        "profile assets must use a supported raster image MIME type".into(),
                    ));
                }
                let provenance = encode_asset_provenance(&asset.provenance)?;
                statement.execute(params![
                    asset.id,
                    asset.entity_id,
                    asset.namespace,
                    asset.filename,
                    asset.content_hash,
                    asset.size,
                    asset.mime_type,
                    asset.path,
                    asset.created_at,
                    asset.role,
                    asset.reference_scope,
                    provenance
                ])?;
            }
        }
        for module in &snapshot.modules {
            transaction.execute("INSERT INTO module_versions(module_id,version) VALUES (?1,?2) ON CONFLICT(module_id) DO UPDATE SET version=excluded.version", params![module.module_id, module.version])?;
            transaction.execute("INSERT INTO module_state(module_id,enabled) VALUES (?1,?2) ON CONFLICT(module_id) DO UPDATE SET enabled=excluded.enabled", params![module.module_id, i64::from(module.enabled)])?;
            if let Some(package_version) = &module.package_version {
                transaction.execute("INSERT INTO module_package_versions(module_id,package_version) VALUES (?1,?2) ON CONFLICT(module_id) DO UPDATE SET package_version=excluded.package_version", params![module.module_id, package_version])?;
            }
            if let Some(overlay) = &module.schema_overlay {
                if !overlay.is_null() {
                    let overlay_json = serde_json::to_string(overlay)
                        .map_err(|error| CoreError::Validation(error.to_string()))?;
                    transaction.execute(
                        "INSERT INTO module_schema_overlays(module_id, overlay_json) VALUES (?1, ?2) ON CONFLICT(module_id) DO UPDATE SET overlay_json=excluded.overlay_json",
                        params![module.module_id, overlay_json],
                    )?;
                }
            }
        }
        for namespace in &snapshot.module_namespaces {
            transaction.execute(
                "INSERT INTO module_namespaces(module_id,namespace) VALUES (?1,?2)",
                params![namespace.module_id, namespace.namespace],
            )?;
        }
        for field in &snapshot.module_fields {
            transaction.execute(
                "INSERT INTO module_fields(module_id,namespace,key,field_type,required) VALUES (?1,?2,?3,?4,?5)",
                params![field.module_id, field.namespace, field.key, field.field_type, i64::from(field.required)],
            )?;
        }
        {
            let mut statement = transaction.prepare_cached(
                "INSERT INTO module_records(module_id,collection,id,owner_entity_id,value,created_at,updated_at) VALUES (?1,?2,?3,?4,?5,?6,?7) ON CONFLICT(id) DO UPDATE SET module_id=excluded.module_id,collection=excluded.collection,owner_entity_id=excluded.owner_entity_id,value=excluded.value,created_at=excluded.created_at,updated_at=excluded.updated_at",
            )?;
            for record in &snapshot.module_records {
                let value = serde_json::to_string(&record.value)
                    .map_err(|error| CoreError::Validation(error.to_string()))?;
                statement.execute(params![
                    record.module_id,
                    record.collection,
                    record.id,
                    record.owner_entity_id,
                    value,
                    record.created_at,
                    record.updated_at
                ])?;
            }
        }
        for migration in &snapshot.migration_history {
            transaction.execute(
                "INSERT INTO migration_history(module_id,migration_id,from_version,to_version,checksum,package_digest,applied_at) VALUES (?1,?2,?3,?4,?5,?6,?7)",
                params![migration.module_id, migration.migration_id, migration.from_version, migration.to_version, migration.checksum, migration.package_digest, migration.applied_at],
            )?;
        }
        if let Some(root) = self.root.as_deref() {
            crate::maps::validate_image_map_content(&transaction, |asset_id| {
                let hash: String = transaction.query_row(
                    "SELECT content_hash FROM assets WHERE id=?1",
                    params![asset_id],
                    |row| row.get(0),
                )?;
                let path = runtime_asset_path(root, &hash)?;
                std::fs::read(&path).map_err(|source| CoreError::Io {
                    operation: "read image map asset",
                    source,
                })
            })?;
        }
        transaction.commit()?;
        if rebuild_search {
            self.rebuild_search()?;
        }
        if sync_canonical {
            self.notify_export_worker()?;
        }
        Ok(snapshot.entities.len())
    }

    pub fn backup(&self) -> Result<String, CoreError> {
        self.backup_to(std::env::temp_dir())
    }

    pub fn backup_to(&self, dir: impl AsRef<Path>) -> Result<String, CoreError> {
        if self.root.is_some() {
            self.flush_checkpoint("runtime backup")?;
        }
        let export = self.export_json()?;
        let dir = dir.as_ref();
        std::fs::create_dir_all(dir).map_err(|e| CoreError::NotFound(e.to_string()))?;
        let timestamp = chrono_like_now();
        let filename = format!("daena-backup-{}-{}.json", timestamp, Uuid::new_v4());
        let path = dir.join(&filename);
        std::fs::write(&path, export).map_err(|e| CoreError::NotFound(e.to_string()))?;
        Ok(path.to_string_lossy().to_string())
    }

    /// Create a files-only portable checkpoint.  Runtime SQLite state and
    /// `.daena/` are intentionally excluded so the result can be restored or
    /// copied using only the canonical project representation.
    pub fn portable_backup_to(&self, dir: impl AsRef<Path>) -> Result<String, CoreError> {
        self.flush_checkpoint("portable backup")?;
        self.portable_backup_after_checkpoint(dir)
    }

    pub fn portable_backup_after_checkpoint(
        &self,
        dir: impl AsRef<Path>,
    ) -> Result<String, CoreError> {
        let root = self.project_root()?;
        crate::storage::FilesystemRepository::open(root)?.scan()?;

        let dir = dir.as_ref();
        std::fs::create_dir_all(dir).map_err(|source| CoreError::Io {
            operation: "create portable backup directory",
            source,
        })?;
        let destination = dir.join(format!(
            "daena-portable-{}-{}",
            chrono_like_now(),
            Uuid::new_v4()
        ));
        copy_portable_project(root, &destination)?;
        Ok(destination.to_string_lossy().into_owned())
    }

    /// Create a machine-local recovery artifact for the current runtime.
    /// Unlike a portable backup, this deliberately preserves the SQLite
    /// runtime state and any staged exporter payloads needed to resume it.
    pub fn recovery_backup_to(&mut self, dir: impl AsRef<Path>) -> Result<String, CoreError> {
        let stopped_worker = self.export_worker.take();
        if let Some(worker) = stopped_worker {
            worker.stop_without_drain()?;
        }
        let result = self.recovery_backup_to_quiesced(dir);
        if self.export_worker.is_none() {
            self.restart_export_worker()?;
        }
        result
    }

    pub(crate) fn recovery_backup_to_quiesced(
        &self,
        dir: impl AsRef<Path>,
    ) -> Result<String, CoreError> {
        let dir = dir.as_ref();
        std::fs::create_dir_all(dir).map_err(|source| CoreError::Io {
            operation: "create recovery backup directory",
            source,
        })?;
        let recovery = dir.join(format!(
            "daena-recovery-{}-{}",
            chrono_like_now(),
            Uuid::new_v4()
        ));
        std::fs::create_dir_all(&recovery).map_err(|source| CoreError::Io {
            operation: "create recovery backup artifact",
            source,
        })?;
        let database_path = recovery.join("index.sqlite");
        {
            let mut destination = Connection::open(&database_path)?;
            let backup = rusqlite::backup::Backup::new(&self.connection, &mut destination)?;
            backup.run_to_completion(5, Duration::from_millis(25), None)?;
        }

        Ok(recovery.to_string_lossy().into_owned())
    }

    pub(crate) fn restart_export_worker(&mut self) -> Result<(), CoreError> {
        if self.export_worker.is_none() {
            if let Some(root) = self.root.as_deref() {
                let database = project_database_path(root);
                if database.is_file() {
                    self.export_worker = Some(ExportWorker::start(root, &database)?);
                }
            }
        }
        Ok(())
    }

    /// Restore a runtime recovery artifact. The current runtime is archived
    /// before replacement; the next generation barrier recreates any files.
    pub fn restore_recovery_backup(&mut self, path: impl AsRef<Path>) -> Result<(), CoreError> {
        let artifact = path.as_ref();
        let source_path = artifact.join("index.sqlite");
        if !source_path.is_file() {
            return Err(CoreError::NotFound(
                "recovery backup is missing index.sqlite".into(),
            ));
        }
        let root = self.project_root()?.to_path_buf();
        let source = Connection::open(&source_path)?;
        Self::validate_runtime_metadata(&source, Some(&root))?;
        let backup_dir = root.join(".daena/backups");
        if let Some(worker) = self.export_worker.take() {
            worker.stop_without_drain()?;
        }
        if let Err(error) = self.recovery_backup_to_quiesced(&backup_dir) {
            self.restart_export_worker()?;
            return Err(error);
        }

        {
            let backup = rusqlite::backup::Backup::new(&source, &mut self.connection)?;
            backup.run_to_completion(5, Duration::from_millis(25), None)?;
        }
        Self::validate_runtime_metadata(&self.connection, Some(&root))?;
        self.export_worker = Some(ExportWorker::start(&root, &project_database_path(&root))?);
        Ok(())
    }

    pub fn create_plugin_backup(
        &self,
        module_id: &str,
        from_package_version: Option<&str>,
        to_package_version: Option<&str>,
        data_version: i64,
    ) -> Result<PluginBackup, CoreError> {
        self.create_plugin_backup_with_request(
            module_id,
            from_package_version,
            to_package_version,
            data_version,
            None,
        )
    }

    pub fn create_plugin_backup_with_request(
        &self,
        module_id: &str,
        from_package_version: Option<&str>,
        to_package_version: Option<&str>,
        data_version: i64,
        request_id: Option<&str>,
    ) -> Result<PluginBackup, CoreError> {
        if self.root.is_some() {
            return self.create_plugin_backup_runtime_snapshot(
                module_id,
                from_package_version,
                to_package_version,
                data_version,
                request_id,
            );
        }
        if let Some(backup) = self.committed_mutation::<PluginBackup>(request_id)? {
            return Ok(backup);
        }
        if module_id.trim().is_empty() {
            return Err(CoreError::Validation(
                "plugin backup requires a module ID".into(),
            ));
        }
        let root = self.project_root()?.to_path_buf();
        let created_at = chrono_like_now();
        let backup_id = Uuid::new_v4().to_string();
        let safe_module = module_id
            .chars()
            .map(|character| {
                if character.is_ascii_alphanumeric() || matches!(character, '.' | '-' | '_') {
                    character
                } else {
                    '_'
                }
            })
            .collect::<String>();
        let relative_path =
            format!(".daena/backups/plugins/plugin-{safe_module}-{created_at}-{backup_id}.json");
        let path = root.join(&relative_path);
        let payload = self.export_json()?;
        let content_hash = digest_bytes(payload.as_bytes());
        let backup = PluginBackup {
            id: backup_id,
            module_id: module_id.into(),
            from_package_version: from_package_version.map(str::to_owned),
            to_package_version: to_package_version.map(str::to_owned),
            data_version,
            path: path.to_string_lossy().into_owned(),
            content_hash,
            created_at,
        };
        let request_id = self.request_id(request_id)?;
        let mut transaction = crate::sync::SyncExporter::begin(&root, &request_id)?;
        transaction.stage_bytes(&relative_path, payload.as_bytes())?;
        let result = serde_json::to_value(&backup)
            .map_err(|error| CoreError::Serialization(error.to_string()))?;
        transaction.commit(Some(&result))?;
        let result = self.connection.execute(
            "INSERT INTO plugin_backups(id,module_id,from_package_version,to_package_version,data_version,path,content_hash,created_at) VALUES (?1,?2,?3,?4,?5,?6,?7,?8)",
            params![backup.id, backup.module_id, backup.from_package_version, backup.to_package_version, backup.data_version, backup.path, backup.content_hash, backup.created_at],
        );
        if let Err(error) = result {
            return Err(error.into());
        }
        Ok(backup)
    }

    pub(crate) fn create_plugin_backup_runtime_snapshot(
        &self,
        module_id: &str,
        from_package_version: Option<&str>,
        to_package_version: Option<&str>,
        data_version: i64,
        request_id: Option<&str>,
    ) -> Result<PluginBackup, CoreError> {
        if let Some(backup) = self.committed_mutation::<PluginBackup>(request_id)? {
            return Ok(backup);
        }
        if module_id.trim().is_empty() {
            return Err(CoreError::Validation(
                "plugin backup requires a module ID".into(),
            ));
        }
        let root = self.project_root()?.to_path_buf();
        let payload = self.export_json_inner()?.into_bytes();
        let created_at = chrono_like_now();
        let backup_id = Uuid::new_v4().to_string();
        let safe_module = module_id
            .chars()
            .map(|character| {
                if character.is_ascii_alphanumeric() || matches!(character, '.' | '-' | '_') {
                    character
                } else {
                    '_'
                }
            })
            .collect::<String>();
        let relative_path =
            format!(".daena/backups/plugins/plugin-{safe_module}-{created_at}-{backup_id}.json");
        let backup = PluginBackup {
            id: backup_id,
            module_id: module_id.into(),
            from_package_version: from_package_version.map(str::to_owned),
            to_package_version: to_package_version.map(str::to_owned),
            data_version,
            path: root.join(&relative_path).to_string_lossy().into_owned(),
            content_hash: digest_bytes(&payload),
            created_at,
        };
        let request_id = self.request_id(request_id)?;
        let result = serde_json::to_value(&backup)
            .map_err(|error| CoreError::Serialization(error.to_string()))?;
        let transaction = self.begin_mutation(
            &request_id,
            Some(&result),
            std::slice::from_ref(&relative_path),
        )?;
        self.insert_plugin_backup_index_on(&transaction, &backup)?;
        transaction.commit()?;
        let mut exporter = crate::sync::SyncExporter::begin(&root, &request_id)?;
        exporter.stage_bytes(&relative_path, &payload)?;
        exporter.commit(Some(&result))?;
        self.export_latest_generation()?;
        Ok(backup)
    }

    pub(crate) fn insert_plugin_backup_index_on(
        &self,
        transaction: &rusqlite::Transaction<'_>,
        backup: &PluginBackup,
    ) -> Result<(), CoreError> {
        transaction.execute(
            "INSERT OR IGNORE INTO plugin_backups(id,module_id,from_package_version,to_package_version,data_version,path,content_hash,created_at) VALUES (?1,?2,?3,?4,?5,?6,?7,?8)",
            params![backup.id, backup.module_id, backup.from_package_version, backup.to_package_version, backup.data_version, backup.path, backup.content_hash, backup.created_at],
        )?;
        Ok(())
    }

    pub fn latest_plugin_backup(
        &self,
        module_id: &str,
        from_package_version: Option<&str>,
        to_package_version: Option<&str>,
    ) -> Result<Option<PluginBackup>, CoreError> {
        self.connection
            .query_row(
                "SELECT id,module_id,from_package_version,to_package_version,data_version,path,content_hash,created_at FROM plugin_backups WHERE module_id=?1 AND from_package_version IS ?2 AND to_package_version IS ?3 ORDER BY created_at DESC LIMIT 1",
                params![module_id, from_package_version, to_package_version],
                |row| {
                    Ok(PluginBackup {
                        id: row.get(0)?,
                        module_id: row.get(1)?,
                        from_package_version: row.get(2)?,
                        to_package_version: row.get(3)?,
                        data_version: row.get(4)?,
                        path: row.get(5)?,
                        content_hash: row.get(6)?,
                        created_at: row.get(7)?,
                    })
                },
            )
            .optional()
            .map_err(CoreError::from)
    }

    pub fn restore_plugin_backup(&mut self, backup: &PluginBackup) -> Result<(), CoreError> {
        self.restore_plugin_backup_with_request(backup, None)
    }

    pub fn restore_plugin_backup_with_request(
        &mut self,
        backup: &PluginBackup,
        request_id: Option<&str>,
    ) -> Result<(), CoreError> {
        let payload = std::fs::read(&backup.path)
            .map_err(|error| CoreError::NotFound(format!("read plugin backup: {error}")))?;
        if digest_bytes(&payload) != backup.content_hash {
            return Err(CoreError::Validation(
                "plugin backup integrity check failed".into(),
            ));
        }
        self.restore_payload_with_request(
            std::str::from_utf8(&payload).map_err(|error| {
                CoreError::Validation(format!("plugin backup is not UTF-8: {error}"))
            })?,
            request_id,
        )
    }

    pub fn restore(&mut self, path: String) -> Result<(), CoreError> {
        let path_ref = Path::new(&path);
        if path_ref.is_dir() {
            let canonical = crate::storage::FilesystemRepository::open(path_ref)?.scan()?;
            let payload = serde_json::to_string(&canonical.snapshot)
                .map_err(|error| CoreError::Serialization(error.to_string()))?;
            self.restore_payload(&payload)?;
            return Ok(());
        }
        let content =
            std::fs::read_to_string(&path).map_err(|e| CoreError::NotFound(e.to_string()))?;
        self.restore_payload(&content)?;
        Ok(())
    }

    pub fn restore_payload(&mut self, payload: &str) -> Result<(), CoreError> {
        self.restore_payload_with_request(payload, None)?;
        Ok(())
    }

    pub fn restore_payload_with_request(
        &mut self,
        payload: &str,
        request_id: Option<&str>,
    ) -> Result<(), CoreError> {
        if self
            .committed_mutation::<serde_json::Value>(request_id)?
            .is_some()
        {
            return Ok(());
        }
        if self.root.is_some() && self.runtime_requires_recovery_archive()? {
            let root = self.project_root()?.to_path_buf();
            self.recovery_backup_to(root.join(".daena/backups"))?;
        }
        self.import_json_with_mode_and_sync_with_request(payload, true, true, request_id)?;
        Ok(())
    }
}
