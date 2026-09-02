// Project lifecycle, open, and checkpoint-barrier operations.
use super::*;

impl ProjectStore {
    /// Open an independent read-only connection for project reads.
    /// reads. It never starts an exporter, acquires the project writer lock,
    /// or initializes/mutates schema state.
    pub fn open_read_only(root: impl AsRef<Path>) -> Result<Self, CoreError> {
        let root = root.as_ref().to_path_buf();
        let path = project_database_path(&root);
        let connection = Connection::open_with_flags(
            &path,
            OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_URI,
        )?;
        connection.busy_timeout(Duration::from_secs(2))?;
        connection.pragma_update(None, "foreign_keys", true)?;
        Self::validate_runtime_metadata(&connection, Some(&root))?;
        let database_epoch = connection.query_row(
            "SELECT database_epoch FROM runtime_meta WHERE key='runtime'",
            [],
            |row| row.get(0),
        )?;
        Ok(Self {
            connection,
            database_epoch,
            root: Some(root),
            relationship_metadata_schemas: BTreeMap::new(),
            relationship_constraints: BTreeMap::new(),
            suppress_sync: Cell::new(true),
            _session_lock: None,
            export_worker: None,
        })
    }

    pub fn checkpoint_handle(&self) -> Result<CheckpointHandle, CoreError> {
        let root = self.root.clone().ok_or_else(|| {
            CoreError::Validation("checkpoint requires a directory-backed project".into())
        })?;
        Ok(CheckpointHandle {
            database: project_database_path(&root),
            root,
            export_sender: self
                .export_worker
                .as_ref()
                .map(|worker| worker.sender.clone()),
        })
    }

    pub(crate) fn open_checkpoint_writer(
        database: impl AsRef<Path>,
        root: PathBuf,
    ) -> Result<Self, CoreError> {
        Self::open_database(database, Some(root), None, false, false)
    }

    pub fn open(path: impl AsRef<Path>) -> Result<Self, CoreError> {
        let path = path.as_ref();
        if path.is_dir() {
            return Self::open_directory(path);
        }
        Err(CoreError::Validation(
            "project storage must be opened from a directory".into(),
        ))
    }

    pub fn open_directory(path: impl AsRef<Path>) -> Result<Self, CoreError> {
        let root = path.as_ref();
        std::fs::create_dir_all(root).map_err(|error| CoreError::NotFound(error.to_string()))?;
        if root.join("daena.sqlite").exists() {
            return Err(CoreError::Validation(
                "legacy daena.sqlite projects are not supported by format version 2".into(),
            ));
        }
        std::fs::create_dir_all(root.join(".daena"))
            .map_err(|error| CoreError::NotFound(error.to_string()))?;
        let session_lock = crate::sync::ProjectSessionLock::acquire(root)?;
        let repository = crate::storage::FilesystemRepository::open(root)?;
        for directory in [
            "entities",
            "plugins",
            ".daena",
            ".daena/checkpoints",
            ".daena/assets",
            ".daena/backups",
            ".daena/conflicts",
            ".daena/local",
        ] {
            std::fs::create_dir_all(root.join(directory))
                .map_err(|error| CoreError::NotFound(error.to_string()))?;
        }
        std::fs::create_dir_all(root.join("assets/images"))
            .map_err(|error| CoreError::NotFound(error.to_string()))?;
        std::fs::create_dir_all(root.join("assets/videos"))
            .map_err(|error| CoreError::NotFound(error.to_string()))?;
        std::fs::create_dir_all(root.join("assets/maps"))
            .map_err(|error| CoreError::NotFound(error.to_string()))?;
        std::fs::create_dir_all(root.join("assets/files"))
            .map_err(|error| CoreError::NotFound(error.to_string()))?;
        let metadata_path = root.join("project.json");
        if metadata_path.exists() {
            let metadata =
                crate::storage::read_json::<crate::storage::ProjectManifest>(&metadata_path)?;
            metadata.validate(&metadata_path)?;
        } else {
            let name = root
                .file_name()
                .and_then(|value| value.to_str())
                .unwrap_or("Daena Archive project");
            let metadata = crate::storage::ProjectManifest::new(name);
            metadata.validate(&metadata_path)?;
            crate::storage::write_json(&metadata_path, &metadata)?;
        }
        let gitignore = root.join(".gitignore");
        let required_gitignore = [".daena/", "checkpoint.json"];
        let existing_gitignore = if gitignore.exists() {
            std::fs::read_to_string(&gitignore)
                .map_err(|error| CoreError::NotFound(error.to_string()))?
        } else {
            String::new()
        };
        let mut gitignore_content = existing_gitignore.clone();
        for pattern in required_gitignore {
            if !existing_gitignore
                .lines()
                .any(|line| line.trim() == pattern)
            {
                if !gitignore_content.is_empty() && !gitignore_content.ends_with('\n') {
                    gitignore_content.push('\n');
                }
                gitignore_content.push_str(pattern);
                gitignore_content.push('\n');
            }
        }
        if gitignore_content != existing_gitignore {
            std::fs::write(&gitignore, gitignore_content)
                .map_err(|error| CoreError::NotFound(error.to_string()))?;
        }
        let index_path = project_database_path(root);
        if index_path.is_file() {
            return Self::open_database(
                &index_path,
                Some(root.to_path_buf()),
                Some(session_lock),
                false,
                true,
            );
        }
        let canonical = repository.scan()?;
        Self::rebuild_directory_index(root, &canonical, session_lock)
    }

    pub(crate) fn rebuild_directory_index(
        root: &Path,
        canonical: &crate::storage::CanonicalProject,
        session_lock: crate::sync::ProjectSessionLock,
    ) -> Result<Self, CoreError> {
        let index_path = project_database_path(root);
        let next_path = root.join(".daena/index.sqlite.next");
        for suffix in ["", "-wal", "-shm", "-journal"] {
            let path = PathBuf::from(format!("{}{}", next_path.display(), suffix));
            if path.exists() {
                std::fs::remove_file(&path).map_err(|error| CoreError::Io {
                    operation: "remove stale index rebuild",
                    source: error,
                })?;
            }
        }

        let store = Self::open_database(&next_path, Some(root.to_path_buf()), None, false, false)?;
        store.import_snapshot_with_mode_and_sync_with_request_and_search(
            &canonical.snapshot,
            true,
            false,
            None,
            false,
        )?;
        store.connection.execute(
            "INSERT INTO project_meta(key,value) VALUES ('ai_enabled',?1) ON CONFLICT(key) DO UPDATE SET value=excluded.value",
            [if canonical.manifest.ai_enabled {
                "true"
            } else {
                "false"
            }],
        )?;
        store.rebuild_search()?;
        store.verify_index()?;
        store.connection.execute(
            "UPDATE runtime_meta SET content_generation=0, exported_generation=0, checkpoint_digest=NULL, export_error=NULL WHERE key='runtime'",
            [],
        )?;
        store.install_checkpoint_manifest(root, 0)?;
        store.connection.execute_batch(
            "PRAGMA wal_checkpoint(TRUNCATE);
             PRAGMA journal_mode=DELETE;",
        )?;
        drop(store);

        // Both paths are in .daena, so rename is atomic on the supported
        // local filesystems.  The old index is never opened or modified while
        // the new database is being built.
        crate::sync::replace_staged_file(&next_path, &index_path).map_err(|error| {
            CoreError::Io {
                operation: "replace runtime index",
                source: error,
            }
        })?;
        for suffix in ["-wal", "-shm", "-journal"] {
            let path = PathBuf::from(format!("{}{}", index_path.display(), suffix));
            let _ = std::fs::remove_file(path);
        }
        crate::sync::sync_directory(&root.join(".daena"))?;
        Self::open_database(
            &index_path,
            Some(root.to_path_buf()),
            Some(session_lock),
            false,
            true,
        )
    }

    pub(crate) fn open_database(
        path: impl AsRef<Path>,
        root: Option<PathBuf>,
        session_lock: Option<crate::sync::ProjectSessionLock>,
        acquire_session_lock: bool,
        start_worker: bool,
    ) -> Result<Self, CoreError> {
        let path = path.as_ref();
        let existing_database = path != Path::new(":memory:") && path.is_file();
        let connection = Connection::open(path)?;
        connection.busy_timeout(Duration::from_secs(2))?;
        connection.pragma_update(None, "foreign_keys", true)?;
        if existing_database {
            Self::validate_runtime_metadata(&connection, root.as_deref())?;
        }
        connection.pragma_update(None, "journal_mode", "WAL")?;
        connection.pragma_update(None, "synchronous", "NORMAL")?;
        let session_lock = match session_lock {
            Some(lock) => Some(lock),
            None if acquire_session_lock => root
                .as_deref()
                .map(crate::sync::ProjectSessionLock::acquire)
                .transpose()?,
            None => None,
        };
        let mut store = Self {
            connection,
            database_epoch: String::new(),
            root,
            relationship_metadata_schemas: BTreeMap::new(),
            relationship_constraints: BTreeMap::new(),
            suppress_sync: Cell::new(false),
            _session_lock: session_lock,
            export_worker: None,
        };
        if !existing_database {
            store.initialize(true)?;
        } else if start_worker {
            store.ensure_query_indexes()?;
            store.ensure_search_projection()?;
        }
        store.database_epoch = store.connection.query_row(
            "SELECT database_epoch FROM runtime_meta WHERE key='runtime'",
            [],
            |row| row.get(0),
        )?;
        if start_worker {
            if let Some(root) = store.root.as_deref() {
                let worker = ExportWorker::start(root, path)?;
                let needs_export: bool = store.connection.query_row(
                    "SELECT content_generation > exported_generation OR export_error IS NOT NULL FROM runtime_meta WHERE key='runtime'",
                    [],
                    |row| row.get(0),
                )?;
                if needs_export {
                    worker.wake();
                }
                store.export_worker = Some(worker);
            }
        }
        Ok(store)
    }

    /// Export one complete checkpoint for the latest committed SQLite state.
    ///
    /// The exporter deliberately renders the whole portable tree. SQLite
    /// generations coalesce bursts of mutations; no durable per-record work
    /// queue is needed to recover or retry an export.
    pub(crate) fn export_latest_generation(&self) -> Result<(), CoreError> {
        let Some(root) = self.root.as_deref() else {
            return Ok(());
        };
        self.connection.execute_batch("BEGIN")?;
        let export_result = (|| {
            let target_generation: Generation = self.connection.query_row(
                "SELECT content_generation FROM runtime_meta WHERE key='runtime'",
                [],
                |row| row.get(0),
            )?;
            let manifest = self.runtime_project_manifest()?;
            let snapshot = self.export_snapshot()?;
            Ok::<_, CoreError>((target_generation, manifest, snapshot))
        })();
        match export_result {
            Ok((target_generation, manifest, snapshot)) => {
                self.connection.execute_batch("COMMIT")?;
                self.export_complete_snapshot(root, &manifest, &snapshot, target_generation)?;
            }
            Err(error) => {
                let _ = self.connection.execute_batch("ROLLBACK");
                return Err(error);
            }
        }
        Ok(())
    }

    /// Wake the project-scoped exporter after durable work has been queued.
    ///
    /// The committed generation is durable in SQLite, so losing this
    /// notification only delays export until the next flush or reopen; it
    /// cannot lose the mutation.
    pub fn wake_export_worker(&self) {
        if let Some(worker) = &self.export_worker {
            worker.wake();
        }
    }

    /// Flush the latest runtime generation to a complete portable checkpoint.
    /// The returned generation is the generation whose checkpoint was
    /// confirmed installed.
    pub fn flush_checkpoint(&self, reason: impl Into<String>) -> Result<Generation, CoreError> {
        let reason = reason.into();
        let target: Generation = self.connection.query_row(
            "SELECT content_generation FROM runtime_meta WHERE key='runtime'",
            [],
            |row| row.get(0),
        )?;
        let result = (|| {
            self.flush_export(reason)?;
            if let Some(root) = self.root.as_deref() {
                let exported: Generation = self.connection.query_row(
                    "SELECT exported_generation FROM runtime_meta WHERE key='runtime'",
                    [],
                    |row| row.get(0),
                )?;
                if exported < target {
                    self.install_checkpoint_manifest(root, target)?;
                }
                let exported: Generation = self.connection.query_row(
                    "SELECT exported_generation FROM runtime_meta WHERE key='runtime'",
                    [],
                    |row| row.get(0),
                )?;
                if exported < target {
                    return Err(CoreError::Conflict(format!(
                        "checkpoint barrier completed below requested generation {target}"
                    )));
                }
            }
            Ok(target)
        })();
        match result {
            Ok(generation) => {
                self.connection.execute(
                    "UPDATE runtime_meta SET export_error=NULL WHERE key='runtime'",
                    [],
                )?;
                Ok(generation)
            }
            Err(error) => {
                let message = error.to_string();
                self.connection.execute(
                    "UPDATE runtime_meta SET export_error=?1 WHERE key='runtime'",
                    params![message],
                )?;
                Err(error)
            }
        }
    }

    pub(crate) fn flush_checkpoint_if_dirty(
        &self,
        reason: impl Into<String>,
    ) -> Result<Generation, CoreError> {
        let (content, exported, failed): (Generation, Generation, bool) = self
            .connection
            .query_row(
                "SELECT content_generation,exported_generation,export_error IS NOT NULL FROM runtime_meta WHERE key='runtime'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )?;
        if exported >= content && !failed {
            return Ok(content);
        }
        self.flush_checkpoint(reason)
    }

    pub(crate) fn flush_export(&self, reason: String) -> Result<(), CoreError> {
        if reason.trim().is_empty() {
            return Err(CoreError::Validation(
                "checkpoint barrier operation reason cannot be empty".into(),
            ));
        }
        if let Some(worker) = &self.export_worker {
            worker.flush(reason).map(|_| ())
        } else {
            self.export_latest_generation()
        }
    }

    pub(crate) fn validate_runtime_metadata(
        connection: &Connection,
        root: Option<&Path>,
    ) -> Result<(), CoreError> {
        let runtime_table: Option<String> = connection
            .query_row(
                "SELECT name FROM sqlite_master WHERE type='table' AND name='runtime_meta'",
                [],
                |row| row.get(0),
            )
            .optional()?;
        if runtime_table.is_none() {
            return Err(reset_required_error());
        }
        let metadata = connection
            .query_row(
                "SELECT storage_role, schema_version, project_id, portable_format_version, exporter_version, content_generation, exported_generation FROM runtime_meta WHERE key='runtime'",
                [],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, i64>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, i64>(3)?,
                        row.get::<_, String>(4)?,
                        row.get::<_, i64>(5)?,
                        row.get::<_, i64>(6)?,
                    ))
                },
            )
            .map_err(|error| match error {
                rusqlite::Error::QueryReturnedNoRows => reset_required_error(),
                other => CoreError::CorruptStorage(format!("runtime metadata is corrupt: {other}")),
            })?;
        let expected_project_id = root
            .map(|root| {
                crate::storage::read_json::<crate::storage::ProjectManifest>(
                    &root.join("project.json"),
                )
                .map(|manifest| manifest.id)
            })
            .transpose()?;
        let project_matches = expected_project_id
            .as_deref()
            .is_none_or(|project_id| project_id == metadata.2);
        if metadata.0 != RUNTIME_STORAGE_ROLE
            || metadata.1 != RUNTIME_SCHEMA_VERSION
            || metadata.3 != i64::from(crate::storage::PROJECT_FORMAT_VERSION)
            || metadata.4 != EXPORTER_CONTRACT_VERSION
            || metadata.5 < 0
            || metadata.6 < 0
            || metadata.6 > metadata.5
            || !project_matches
        {
            return Err(reset_required_error());
        }
        Ok(())
    }

    pub(crate) fn verify_index(&self) -> Result<(), CoreError> {
        let foreign_key_error: Option<String> = self
            .connection
            .query_row("PRAGMA foreign_key_check", [], |row| row.get(0))
            .optional()?;
        if let Some(error) = foreign_key_error {
            return Err(CoreError::Validation(format!(
                "index foreign-key check failed: {error}"
            )));
        }
        let integrity: String = self
            .connection
            .query_row("PRAGMA integrity_check", [], |row| row.get(0))?;
        if integrity != "ok" {
            return Err(CoreError::Validation(format!(
                "index integrity check failed: {integrity}"
            )));
        }
        Ok(())
    }

    pub(crate) fn checkpoint_sources(
        root: &Path,
    ) -> Result<Vec<crate::storage::CanonicalSource>, CoreError> {
        let path = root.join(crate::storage::CHECKPOINT_MANIFEST_FILE);
        if !path.is_file() {
            return Ok(Vec::new());
        }
        let checkpoint: crate::storage::CheckpointManifest = crate::storage::read_json(&path)?;
        checkpoint.validate(&path)?;
        Ok(checkpoint
            .files
            .into_iter()
            .map(|file| crate::storage::CanonicalSource {
                path: file.path,
                content_hash: file.sha256,
                format_version: crate::storage::PROJECT_FORMAT_VERSION,
            })
            .collect())
    }

    pub fn import_checkpoint(&mut self) -> Result<ExternalChangeReport, CoreError> {
        let root = self.project_root()?.to_path_buf();
        let checkpoint_path = root.join(crate::storage::CHECKPOINT_MANIFEST_FILE);
        let checkpoint: crate::storage::CheckpointManifest =
            crate::storage::read_json(&checkpoint_path)?;
        crate::storage::validate_checkpoint(&root, &checkpoint)?;
        let (content_generation, exported_generation): (Generation, Generation) = self
            .connection
            .query_row(
            "SELECT content_generation,exported_generation FROM runtime_meta WHERE key='runtime'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )?;
        if content_generation != exported_generation {
            return Err(CoreError::Conflict(
                "runtime has unexported changes; flush the checkpoint before importing portable files"
                    .into(),
            ));
        }
        let archive = self.recovery_backup_to(root.join(".daena/backups"))?;
        let canonical = crate::storage::FilesystemRepository::open(&root)?.scan()?;
        let payload = serde_json::to_string(&canonical.snapshot)
            .map_err(|error| CoreError::Serialization(error.to_string()))?;
        let next_path = root.join(".daena/index.sqlite.next");
        for suffix in ["", "-wal", "-shm", "-journal"] {
            let path = PathBuf::from(format!("{}{}", next_path.display(), suffix));
            if path.exists() {
                std::fs::remove_file(path).map_err(|error| CoreError::Io {
                    operation: "remove stale checkpoint candidate",
                    source: error,
                })?;
            }
        }
        let candidate = Self::open_database(&next_path, Some(root.clone()), None, false, false)?;
        candidate.import_json_with_mode_and_sync_with_request_and_search(
            &payload, true, false, None, false,
        )?;
        candidate.connection.execute(
            "INSERT INTO project_meta(key,value) VALUES ('ai_enabled',?1) ON CONFLICT(key) DO UPDATE SET value=excluded.value",
            [if canonical.manifest.ai_enabled {
                "true"
            } else {
                "false"
            }],
        )?;
        let digest =
            crate::storage::canonical_json_bytes(&checkpoint).map(|bytes| digest_bytes(&bytes))?;
        candidate.rebuild_search()?;
        candidate.rebuild_maps_projection()?;
        candidate.connection.execute(
            "UPDATE runtime_meta SET database_epoch=?1,content_generation=?2,exported_generation=?2,checkpoint_digest=?3,export_error=NULL WHERE key='runtime'",
            params![Uuid::new_v4().to_string(), checkpoint.content_generation, digest],
        )?;
        candidate
            .connection
            .execute_batch("PRAGMA wal_checkpoint(TRUNCATE); PRAGMA journal_mode=DELETE;")?;
        drop(candidate);
        if let Some(worker) = self.export_worker.take() {
            worker.stop_without_drain()?;
        }
        let old_connection = std::mem::replace(&mut self.connection, Connection::open_in_memory()?);
        drop(old_connection);
        let index_path = project_database_path(&root);
        crate::sync::replace_staged_file(&next_path, &index_path).map_err(|error| {
            CoreError::Io {
                operation: "install checkpoint candidate database",
                source: error,
            }
        })?;
        for suffix in ["-wal", "-shm", "-journal"] {
            let path = PathBuf::from(format!("{}{}", index_path.display(), suffix));
            let _ = std::fs::remove_file(path);
        }
        crate::sync::sync_directory(&root.join(".daena"))?;
        self.connection = Connection::open(&index_path)?;
        self.connection.busy_timeout(Duration::from_secs(2))?;
        self.connection.pragma_update(None, "foreign_keys", true)?;
        self.connection.pragma_update(None, "journal_mode", "WAL")?;
        self.connection
            .pragma_update(None, "synchronous", "NORMAL")?;
        self.database_epoch = self.connection.query_row(
            "SELECT database_epoch FROM runtime_meta WHERE key='runtime'",
            [],
            |row| row.get(0),
        )?;
        self.restart_export_worker()?;
        Ok(ExternalChangeReport {
            changed: true,
            paths: checkpoint
                .files
                .iter()
                .map(|file| file.path.clone())
                .collect(),
            diagnostics: vec![format!("runtime archive preserved at {archive}")],
        })
    }

    pub fn save_recovery_copy(&self, entity_id: &str, body: &str) -> Result<String, CoreError> {
        let Some(root) = self.root.as_deref() else {
            return Err(CoreError::Validation(
                "recovery copies require a directory-backed project".into(),
            ));
        };
        uuid::Uuid::parse_str(entity_id)
            .map_err(|error| CoreError::Validation(format!("invalid entity ID: {error}")))?;
        let conflicts = root.join(".daena/conflicts");
        std::fs::create_dir_all(&conflicts).map_err(|error| CoreError::Io {
            operation: "create recovery copy directory",
            source: error,
        })?;
        let path = conflicts.join(format!(
            "{}-{}-{}.md",
            chrono_like_now(),
            entity_id,
            Uuid::new_v4()
        ));
        let bytes = crate::storage::canonical_markdown(body).into_bytes();
        std::fs::write(&path, bytes).map_err(|error| CoreError::Io {
            operation: "write recovery copy",
            source: error,
        })?;
        Ok(path
            .strip_prefix(root)
            .unwrap_or(&path)
            .to_string_lossy()
            .replace('\\', "/"))
    }

    pub(crate) fn require_map_entity(&self, entity_id: &str) -> Result<(), CoreError> {
        uuid::Uuid::parse_str(entity_id)
            .map_err(|error| CoreError::Validation(format!("invalid entity ID: {error}")))?;
        let entity_type: Option<String> = self.connection.query_row(
            "SELECT entity_type FROM entities WHERE id=?1 AND deleted=0",
            [entity_id],
            |row| row.get(0),
        )?;
        if entity_type.as_deref() != Some(crate::maps::MAP_ENTITY_TYPE) {
            return Err(CoreError::NotFound("map entity not found".into()));
        }
        Ok(())
    }

    pub(crate) fn map_recovery_dir(&self, create: bool) -> Result<std::path::PathBuf, CoreError> {
        let Some(root) = self.root.as_deref() else {
            return Err(CoreError::Validation(
                "map recovery copies require a directory-backed project".into(),
            ));
        };
        let conflicts = root.join(".daena/conflicts/maps");
        if create {
            std::fs::create_dir_all(&conflicts).map_err(|error| CoreError::Io {
                operation: "create map recovery copy directory",
                source: error,
            })?;
        }
        Ok(conflicts)
    }

    /// Writes a rejected map save draft package to `.daena/conflicts/maps/`.
    /// The original source asset is never overwritten without review; the draft
    /// lives only in the disposable derived state directory.
    pub fn save_map_recovery_copy(
        &self,
        entity_id: &str,
        bytes: &[u8],
    ) -> Result<String, CoreError> {
        self.require_map_entity(entity_id)?;
        let package = crate::maps::MapEditDraftPackage::parse(bytes)?;
        if package.map_entity_id != entity_id {
            return Err(CoreError::Validation(
                "map edit draft mapEntityId does not match the target map".into(),
            ));
        }
        let conflicts = self.map_recovery_dir(true)?;
        let file_name = format!(
            "{}-{}-{}.map",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis(),
            entity_id,
            Uuid::new_v4()
        );
        std::fs::write(conflicts.join(&file_name), bytes).map_err(|error| CoreError::Io {
            operation: "write map recovery copy",
            source: error,
        })?;
        Ok(format!(".daena/conflicts/maps/{file_name}"))
    }

    /// Lists the recovery drafts recorded for a map, newest first. The
    /// returned `path` is root-relative so it can be surfaced to the shell
    /// without exposing filesystem layout.
    pub fn list_map_recovery_copies(
        &self,
        entity_id: &str,
    ) -> Result<Vec<crate::maps::MapRecoveryCopy>, CoreError> {
        self.require_map_entity(entity_id)?;
        let Ok(conflicts) = self.map_recovery_dir(false) else {
            return Ok(Vec::new());
        };
        let Ok(entries) = std::fs::read_dir(&conflicts) else {
            return Ok(Vec::new());
        };
        let mut copies = Vec::new();
        for entry in entries.flatten() {
            let Some(file_name) = entry.file_name().to_str().map(str::to_owned) else {
                continue;
            };
            let Some(created_at) = parse_map_recovery_file_name(entity_id, &file_name) else {
                continue;
            };
            copies.push(crate::maps::MapRecoveryCopy {
                file_name: file_name.clone(),
                path: format!(".daena/conflicts/maps/{file_name}"),
                created_at,
            });
        }
        copies.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        Ok(copies)
    }

    /// Restores a previously exported map edit draft package. This is an
    /// explicit user action, so compare-and-swap uses the current revisions.
    pub fn restore_map_recovery_copy(
        &self,
        entity_id: &str,
        file_name: &str,
        request_id: Option<&str>,
    ) -> Result<MapEditApply, CoreError> {
        self.require_map_entity(entity_id)?;
        if file_name.trim().is_empty() || file_name.contains('/') || file_name.contains('\\') {
            return Err(CoreError::Validation(
                "map recovery copy file name is invalid".into(),
            ));
        }
        if parse_map_recovery_file_name(entity_id, file_name).is_none() {
            return Err(CoreError::Validation(
                "map recovery copy file name is invalid".into(),
            ));
        }
        let conflicts = self.map_recovery_dir(false)?;
        let bytes = std::fs::read(conflicts.join(file_name)).map_err(|error| CoreError::Io {
            operation: "read map recovery copy",
            source: error,
        })?;
        let package = crate::maps::MapEditDraftPackage::parse(&bytes)?;
        if package.map_entity_id != entity_id {
            return Err(CoreError::Validation(
                "map edit draft mapEntityId does not match the target map".into(),
            ));
        }
        let fields = self.list_fields_unchecked(entity_id.to_owned())?;
        let map_field = fields
            .iter()
            .find(|field| field.namespace == crate::maps::MAP_NAMESPACE && field.key == "map")
            .ok_or_else(|| CoreError::NotFound("map descriptor not found".into()))?;
        let map_revision = self.revision_for_field(map_field)?;
        let layers_field = self.layers_field(entity_id)?;
        let source_id = map_field
            .value
            .get("authoredSourceAssetId")
            .or_else(|| map_field.value.get("sourceAssetId"))
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| CoreError::NotFound("vector source asset not found".into()))?
            .to_owned();
        let source = self.asset_unchecked(&source_id)?;
        let source_revision = self.revision_for_asset(&source.id)?;
        let geojson_bytes = package.geojson.into_bytes();
        let upload_content_hash = format!("sha256:{:x}", Sha256::digest(&geojson_bytes));
        let mut link_mutations = Vec::with_capacity(package.link_mutations.len());
        for value in package.link_mutations {
            let mutation: MapLinkMutation = serde_json::from_value(value).map_err(|error| {
                CoreError::Validation(format!("invalid link mutation: {error}"))
            })?;
            let current = self
                .list_fields_unchecked(mutation.entity_id.clone())?
                .into_iter()
                .find(|field| {
                    field.namespace == crate::maps::MAP_NAMESPACE && field.key == "locations"
                });
            let expected_locations_revision = match &current {
                Some(field) => self.revision_for_field(field)?,
                None => String::new(),
            };
            link_mutations.push(MapLinkMutation {
                entity_id: mutation.entity_id,
                expected_locations_revision,
                locations: mutation.locations,
            });
        }
        self.apply_map_edit(
            entity_id.to_owned(),
            package.descriptor,
            package.layers,
            geojson_bytes,
            upload_content_hash,
            &map_revision,
            &layers_field.revision,
            &source_revision,
            link_mutations,
            request_id,
        )
    }

    pub fn in_memory() -> Result<Self, CoreError> {
        Self::open_database(":memory:", None, None, true, false)
    }

    pub fn info(&self) -> Option<ProjectInfo> {
        let root = self.root.as_ref()?;
        let sync = self.sync_summary().unwrap_or_else(|_| SyncSummary {
            state: "diagnostic".into(),
            dirty_count: 0,
            export_error: Some("runtime metadata is unavailable".into()),
        });
        let manifest = crate::storage::read_json::<crate::storage::ProjectManifest>(
            &root.join("project.json"),
        )
        .ok();
        Some(ProjectInfo {
            name: manifest.as_ref().map_or_else(
                || {
                    root.file_name()
                        .and_then(|value| value.to_str())
                        .unwrap_or("Daena Archive project")
                        .to_string()
                },
                |manifest| manifest.name.clone(),
            ),
            root: root.to_string_lossy().to_string(),
            index_status: if sync.state == "clean" {
                "ready"
            } else {
                "diagnostic"
            }
            .into(),
            assets: root.join("assets").to_string_lossy().to_string(),
            sync,
            ai_enabled: self.ai_enabled().unwrap_or(false),
        })
    }

    pub fn database_epoch(&self) -> &str {
        &self.database_epoch
    }

    pub fn content_generation(&self) -> Result<i64, CoreError> {
        self.connection
            .query_row(
                "SELECT content_generation FROM runtime_meta WHERE key='runtime'",
                [],
                |row| row.get(0),
            )
            .map_err(CoreError::from)
    }

    pub fn sync_summary(&self) -> Result<SyncSummary, CoreError> {
        let (content, exported, export_error): (i64, i64, Option<String>) = self.connection.query_row(
            "SELECT content_generation,exported_generation,export_error FROM runtime_meta WHERE key='runtime'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )?;
        let state = if export_error.is_some() {
            "failed"
        } else if exported < content {
            "pending"
        } else {
            "clean"
        };
        Ok(SyncSummary {
            state: state.into(),
            dirty_count: content.saturating_sub(exported),
            export_error,
        })
    }

    pub(crate) fn runtime_requires_recovery_archive(&self) -> Result<bool, CoreError> {
        let (exported, content, export_error): (i64, i64, Option<String>) =
            self.connection.query_row(
                "SELECT exported_generation,content_generation,export_error FROM runtime_meta WHERE key='runtime'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )?;
        Ok(export_error.is_some() || exported < content)
    }

    pub(crate) fn project_root(&self) -> Result<&Path, CoreError> {
        self.root
            .as_deref()
            .ok_or_else(|| CoreError::NotFound("project is not directory-backed".into()))
    }

    pub(crate) fn request_id(&self, request_id: Option<&str>) -> Result<String, CoreError> {
        let request_id = request_id.map_or_else(|| Uuid::new_v4().to_string(), str::to_owned);
        Uuid::parse_str(&request_id)
            .map_err(|_| CoreError::Validation("transaction request ID must be a UUID".into()))?;
        Ok(request_id)
    }

    pub(crate) fn initialize(&self, rebuild_derived: bool) -> Result<(), CoreError> {
        self.connection.execute_batch(
            "PRAGMA journal_mode = WAL;
             CREATE TABLE IF NOT EXISTS runtime_meta (
               key TEXT PRIMARY KEY,
               storage_role TEXT NOT NULL,
               schema_version INTEGER NOT NULL,
               project_id TEXT NOT NULL,
               portable_format_version INTEGER NOT NULL,
               database_epoch TEXT NOT NULL,
               exporter_version TEXT NOT NULL,
               content_generation INTEGER NOT NULL DEFAULT 0,
               exported_generation INTEGER NOT NULL DEFAULT 0,
               checkpoint_digest TEXT,
               export_error TEXT
             );
             CREATE TABLE IF NOT EXISTS mutation_receipts (
               request_id TEXT PRIMARY KEY,
               result TEXT NOT NULL,
               fingerprint TEXT NOT NULL DEFAULT '',
               committed_at TEXT NOT NULL,
               state TEXT NOT NULL DEFAULT 'pending'
             );
             CREATE TABLE IF NOT EXISTS project_meta (key TEXT PRIMARY KEY, value TEXT NOT NULL);
             INSERT OR IGNORE INTO project_meta(key, value) VALUES ('schema_version', '1');
             INSERT OR IGNORE INTO project_meta(key, value) VALUES ('ai_enabled', 'false');
             CREATE TABLE IF NOT EXISTS entities (
               id TEXT PRIMARY KEY, name TEXT NOT NULL, entity_type TEXT,
               deleted INTEGER NOT NULL DEFAULT 0, created_at TEXT NOT NULL, updated_at TEXT NOT NULL
             );
             CREATE TABLE IF NOT EXISTS documents (
               id TEXT PRIMARY KEY, entity_id TEXT NOT NULL REFERENCES entities(id),
               format TEXT NOT NULL, body TEXT NOT NULL, updated_at TEXT NOT NULL
             );
             CREATE TABLE IF NOT EXISTS relationships (
               id TEXT PRIMARY KEY, source_id TEXT NOT NULL REFERENCES entities(id),
               target_id TEXT NOT NULL REFERENCES entities(id), relationship_type TEXT NOT NULL,
               metadata TEXT NOT NULL DEFAULT '{}'
             );
             CREATE INDEX IF NOT EXISTS entities_name_idx ON entities(name);
             CREATE INDEX IF NOT EXISTS entities_live_name_nocase_idx ON entities(name COLLATE NOCASE,id) WHERE deleted=0;
             CREATE INDEX IF NOT EXISTS entities_type_name_idx ON entities(entity_type,name,id) WHERE deleted=0;
             CREATE INDEX IF NOT EXISTS entities_live_type_name_nocase_idx ON entities(entity_type,name COLLATE NOCASE,id) WHERE deleted=0;
             CREATE INDEX IF NOT EXISTS entities_created_idx ON entities(created_at,id) WHERE deleted=0;
             CREATE INDEX IF NOT EXISTS entities_updated_idx ON entities(updated_at,id) WHERE deleted=0;
             CREATE INDEX IF NOT EXISTS documents_entity_updated_idx ON documents(entity_id,updated_at DESC);
             CREATE INDEX IF NOT EXISTS relationships_source_idx ON relationships(source_id);
             CREATE INDEX IF NOT EXISTS relationships_target_idx ON relationships(target_id);"
        )?;
        self.connection.execute_batch("CREATE TABLE IF NOT EXISTS module_versions(module_id TEXT PRIMARY KEY, version INTEGER NOT NULL DEFAULT 0);
              CREATE TABLE IF NOT EXISTS module_state(module_id TEXT PRIMARY KEY, enabled INTEGER NOT NULL DEFAULT 1);
              CREATE TABLE IF NOT EXISTS module_package_versions(module_id TEXT PRIMARY KEY, package_version TEXT NOT NULL);
              CREATE TABLE IF NOT EXISTS module_namespaces(module_id TEXT NOT NULL, namespace TEXT NOT NULL, PRIMARY KEY(module_id, namespace));
             CREATE TABLE IF NOT EXISTS module_fields(module_id TEXT NOT NULL, namespace TEXT NOT NULL, key TEXT NOT NULL, field_type TEXT NOT NULL, required INTEGER NOT NULL, PRIMARY KEY(module_id, namespace, key));
              CREATE TABLE IF NOT EXISTS module_records(id TEXT PRIMARY KEY, module_id TEXT NOT NULL, collection TEXT NOT NULL, owner_entity_id TEXT NOT NULL REFERENCES entities(id), value TEXT NOT NULL, created_at TEXT NOT NULL, updated_at TEXT NOT NULL, UNIQUE(module_id, collection, id));
              CREATE INDEX IF NOT EXISTS module_records_owner_idx ON module_records(module_id, collection, owner_entity_id, id);
              CREATE TABLE IF NOT EXISTS entity_fields(entity_id TEXT NOT NULL REFERENCES entities(id), namespace TEXT NOT NULL, key TEXT NOT NULL, value TEXT NOT NULL, PRIMARY KEY(entity_id, namespace, key));
             CREATE TABLE IF NOT EXISTS assets (id TEXT PRIMARY KEY, entity_id TEXT NOT NULL REFERENCES entities(id), namespace TEXT NOT NULL, filename TEXT NOT NULL, content_hash TEXT NOT NULL, size INTEGER NOT NULL, mime_type TEXT NOT NULL, path TEXT NOT NULL, created_at TEXT NOT NULL, role TEXT NOT NULL DEFAULT 'attachment' CHECK(role IN ('attachment','profile')), reference_scope TEXT NOT NULL DEFAULT 'entity' CHECK(reference_scope IN ('entity','project')), provenance TEXT);
             CREATE TABLE IF NOT EXISTS map_projection (map_entity_id TEXT PRIMARY KEY, provider TEXT NOT NULL, source_asset_id TEXT NOT NULL, source_path TEXT, source_hash TEXT);
             CREATE TABLE IF NOT EXISTS map_location_projection (location_id TEXT PRIMARY KEY, entity_id TEXT NOT NULL, map_entity_id TEXT NOT NULL, label TEXT, role TEXT NOT NULL, anchor_kind TEXT NOT NULL, provider TEXT, feature_kind TEXT, feature_id TEXT, min_x REAL, min_y REAL, max_x REAL, max_y REAL, valid_from TEXT, valid_to TEXT, resolution TEXT NOT NULL);
             CREATE TABLE IF NOT EXISTS map_feature_projection (map_entity_id TEXT NOT NULL, feature_id TEXT NOT NULL, layer_id TEXT NOT NULL, kind TEXT NOT NULL, min_x REAL NOT NULL, min_y REAL NOT NULL, max_x REAL NOT NULL, max_y REAL NOT NULL, PRIMARY KEY (map_entity_id, feature_id));
             CREATE INDEX IF NOT EXISTS map_location_entity_idx ON map_location_projection(entity_id);
             CREATE INDEX IF NOT EXISTS map_location_map_idx ON map_location_projection(map_entity_id);
             CREATE INDEX IF NOT EXISTS assets_entity_created_idx ON assets(entity_id,created_at);
             CREATE UNIQUE INDEX IF NOT EXISTS assets_profile_namespace_idx ON assets(entity_id,namespace) WHERE role='profile';")?;
        self.ensure_map_location_projection_schema()?;
        self.connection.execute_batch("CREATE TABLE IF NOT EXISTS migration_history(module_id TEXT NOT NULL, migration_id TEXT NOT NULL, from_version INTEGER NOT NULL, to_version INTEGER NOT NULL, checksum TEXT NOT NULL, package_digest TEXT NOT NULL DEFAULT '', applied_at TEXT NOT NULL DEFAULT '', PRIMARY KEY(module_id, migration_id)); CREATE TABLE IF NOT EXISTS plugin_backups(id TEXT PRIMARY KEY, module_id TEXT NOT NULL, from_package_version TEXT, to_package_version TEXT, data_version INTEGER NOT NULL, path TEXT NOT NULL, content_hash TEXT NOT NULL, created_at TEXT NOT NULL); CREATE TABLE IF NOT EXISTS module_schema_overlays(module_id TEXT PRIMARY KEY, overlay_json TEXT NOT NULL);")?;
        for table in [
            "project_meta",
            "entities",
            "documents",
            "relationships",
            "entity_fields",
            "assets",
            "module_versions",
            "module_state",
            "module_package_versions",
            "module_namespaces",
            "module_fields",
            "module_records",
            "module_schema_overlays",
            "migration_history",
            "plugin_backups",
        ] {
            for event in ["INSERT", "UPDATE", "DELETE"] {
                self.connection.execute_batch(&format!(
                    "CREATE TRIGGER IF NOT EXISTS runtime_content_generation_{event}_{table} AFTER {event} ON {table} BEGIN UPDATE runtime_meta SET content_generation=content_generation+1 WHERE key='runtime'; END;"
                ))?;
            }
        }
        if self
            .connection
            .query_row("SELECT 1 FROM runtime_meta WHERE key='runtime'", [], |_| {
                Ok(())
            })
            .is_err()
        {
            let project_id = self
                .root
                .as_deref()
                .map(|root| {
                    crate::storage::read_json::<crate::storage::ProjectManifest>(
                        &root.join("project.json"),
                    )
                    .map(|manifest| manifest.id)
                })
                .transpose()?
                .unwrap_or_default();
            self.connection.execute(
                "INSERT INTO runtime_meta(key,storage_role,schema_version,project_id,portable_format_version,database_epoch,exporter_version,content_generation,exported_generation) VALUES ('runtime',?1,?2,?3,?4,?5,?6,0,0)",
                params![
                    RUNTIME_STORAGE_ROLE,
                    RUNTIME_SCHEMA_VERSION,
                    project_id,
                    i64::from(crate::storage::PROJECT_FORMAT_VERSION),
                    Uuid::new_v4().to_string(),
                    EXPORTER_CONTRACT_VERSION,
                ],
            )?;
        }
        if rebuild_derived {
            self.rebuild_search()?;
        } else {
            self.ensure_search_projection()?;
        }
        Ok(())
    }

    pub(crate) fn ensure_query_indexes(&self) -> Result<(), CoreError> {
        self.connection.execute_batch(
            "CREATE INDEX IF NOT EXISTS entities_live_name_nocase_idx ON entities(name COLLATE NOCASE,id) WHERE deleted=0;
             CREATE INDEX IF NOT EXISTS entities_live_type_name_nocase_idx ON entities(entity_type,name COLLATE NOCASE,id) WHERE deleted=0;
             CREATE INDEX IF NOT EXISTS documents_entity_updated_idx ON documents(entity_id,updated_at DESC);
             CREATE INDEX IF NOT EXISTS assets_entity_created_idx ON assets(entity_id,created_at);",
        )?;
        Ok(())
    }

    pub(crate) fn ensure_search_projection(&self) -> Result<(), CoreError> {
        let search_table_exists: bool = self.connection.query_row(
            "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name='world_search')",
            [],
            |row| row.get(0),
        )?;
        let search_shape_current = search_table_exists
            && self
                .connection
                .prepare("SELECT source_key FROM world_search LIMIT 0")
                .is_ok();
        let search_triggers_current: bool = self.connection.query_row(
            "SELECT COUNT(*)=12 FROM sqlite_master WHERE type='trigger' AND name IN ('entities_search_insert','entities_search_update','entities_search_deleted','documents_search_insert','documents_search_update','documents_search_delete','entity_fields_search_insert','entity_fields_search_update','entity_fields_search_delete','module_records_search_insert','module_records_search_update','module_records_search_delete')",
            [],
            |row| row.get(0),
        )?;
        let search_missing = !search_shape_current
            || !search_triggers_current
            || self.connection.query_row(
                "SELECT EXISTS(SELECT 1 FROM entities WHERE deleted=0) AND NOT EXISTS(SELECT 1 FROM world_search)",
                [],
                |row| row.get(0),
            )?;
        let record_search_table_exists: bool = self.connection.query_row(
            "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name='module_record_search')",
            [],
            |row| row.get(0),
        )?;
        let record_search_missing = !record_search_table_exists
            || self.connection.query_row(
                "SELECT EXISTS(SELECT 1 FROM module_records) AND NOT EXISTS(SELECT 1 FROM module_record_search)",
                [],
                |row| row.get(0),
            )?;
        let map_feature_search_exists: bool = self.connection.query_row(
            "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name='map_feature_search')",
            [],
            |row| row.get(0),
        )?;
        if search_missing || record_search_missing || !map_feature_search_exists {
            self.rebuild_search()?;
        }
        Ok(())
    }
}
