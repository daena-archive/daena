use crate::error::CoreError;
use rusqlite::{params, Connection, OptionalExtension};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Mutex;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateEntity {
    pub name: String,
    pub entity_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateEntryDocument {
    pub body: String,
    pub format: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateEntryField {
    pub namespace: String,
    pub key: String,
    pub value: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateEntryRelationship {
    pub relationship_type: String,
    pub target_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateEntry {
    pub name: String,
    pub entity_type: Option<String>,
    pub document: Option<CreateEntryDocument>,
    pub fields: Vec<CreateEntryField>,
    #[serde(default)]
    pub relationships: Vec<CreateEntryRelationship>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entity {
    pub id: String,
    pub name: String,
    pub entity_type: Option<String>,
    pub deleted: bool,
    pub created_at: String,
    pub updated_at: String,
    #[serde(default)]
    pub revision: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SaveDocument {
    pub entity_id: String,
    pub body: String,
    pub format: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SaveEntry {
    pub document: SaveDocument,
    pub fields: Vec<FieldValue>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Document {
    pub id: String,
    pub entity_id: String,
    pub format: String,
    pub body: String,
    pub updated_at: String,
    #[serde(default)]
    pub revision: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldValue {
    pub entity_id: String,
    pub namespace: String,
    pub key: String,
    pub value: serde_json::Value,
    #[serde(default)]
    pub revision: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelationshipInput {
    pub source_id: String,
    pub target_id: String,
    pub relationship_type: String,
    pub metadata: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Relationship {
    pub id: String,
    pub source_id: String,
    pub target_id: String,
    pub relationship_type: String,
    pub metadata: String,
    #[serde(default)]
    pub revision: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetInput {
    pub entity_id: String,
    pub namespace: String,
    pub filename: String,
    pub content_hash: String,
    pub size: i64,
    pub mime_type: String,
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetFileInput {
    pub entity_id: String,
    pub namespace: String,
    pub source_path: String,
    pub filename: String,
    pub mime_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetReplaceInput {
    pub asset_id: String,
    pub content_hash: String,
    pub size: i64,
    pub mime_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Asset {
    pub id: String,
    pub entity_id: String,
    pub namespace: String,
    pub filename: String,
    pub content_hash: String,
    pub size: i64,
    pub mime_type: String,
    pub path: String,
    pub created_at: String,
    #[serde(default)]
    pub revision: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleState {
    pub module_id: String,
    pub enabled: bool,
    pub version: i64,
    #[serde(default)]
    pub package_version: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleNamespace {
    pub module_id: String,
    pub namespace: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleField {
    pub module_id: String,
    pub namespace: String,
    pub key: String,
    pub field_type: String,
    pub required: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationHistoryEntry {
    pub module_id: String,
    pub migration_id: String,
    pub from_version: i64,
    pub to_version: i64,
    pub checksum: String,
    #[serde(default)]
    pub package_digest: String,
    #[serde(default)]
    pub applied_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PluginBackup {
    pub id: String,
    pub module_id: String,
    pub from_package_version: Option<String>,
    pub to_package_version: Option<String>,
    pub data_version: i64,
    pub path: String,
    pub content_hash: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectSnapshot {
    #[serde(default = "current_snapshot_version")]
    pub format_version: u32,
    pub entities: Vec<Entity>,
    pub documents: Vec<Document>,
    #[serde(default)]
    pub fields: Vec<FieldValue>,
    pub relationships: Vec<Relationship>,
    #[serde(default)]
    pub assets: Vec<Asset>,
    #[serde(default)]
    pub modules: Vec<ModuleState>,
    #[serde(default)]
    pub module_namespaces: Vec<ModuleNamespace>,
    #[serde(default)]
    pub module_fields: Vec<ModuleField>,
    #[serde(default)]
    pub migration_history: Vec<MigrationHistoryEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProjectInfo {
    pub name: String,
    pub root: String,
    pub index_status: String,
    pub assets: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitStatus {
    pub repository: bool,
    pub branch: Option<String>,
    pub changes: Vec<String>,
    #[serde(default)]
    pub canonical_changes: Vec<String>,
    #[serde(default)]
    pub staged_canonical_changes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitLogEntry {
    pub hash: String,
    pub date: String,
    pub subject: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GitPreflight {
    pub ready: bool,
    pub diagnostics: Vec<String>,
    pub canonical_paths: Vec<String>,
    pub asset_paths: Vec<String>,
    pub staging_paths: Vec<String>,
    pub staged_paths: Vec<String>,
    pub unmerged_paths: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct GitToolInfo {
    pub available: bool,
    pub version: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct GitRemote {
    pub name: String,
    pub fetch_url: String,
    pub push_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct GitUpstream {
    pub remote: String,
    pub branch: String,
    pub remote_hash: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GitResetResult {
    pub status: GitStatus,
    pub previous_head: Option<String>,
    pub current_head: Option<String>,
    pub upstream: Option<GitUpstream>,
    pub diverged_from_upstream: bool,
    pub rebuild: ExternalChangeReport,
}

const GIT_SHOW_FILE_MAX_BYTES: usize = 512 * 1024;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ExternalChangeReport {
    pub changed: bool,
    pub paths: Vec<String>,
    pub diagnostics: Vec<String>,
}

fn current_snapshot_version() -> u32 {
    1
}

fn decode_field_value(value: String) -> serde_json::Value {
    serde_json::from_str(&value).unwrap_or(serde_json::Value::String(value))
}

fn encode_field_value(value: &serde_json::Value) -> Result<String, CoreError> {
    serde_json::to_string(value).map_err(|error| CoreError::NotFound(error.to_string()))
}

fn validate_document_format(format: Option<&str>, directory_backed: bool) -> Result<(), CoreError> {
    let format = format.unwrap_or("markdown");
    if directory_backed && format != "markdown" {
        return Err(CoreError::Validation(
            "directory-backed projects require Markdown documents".into(),
        ));
    }
    if format != "markdown" && format != "plain-text" && format != "rich-text" {
        return Err(CoreError::NotFound("unsupported document format".into()));
    }
    Ok(())
}

fn project_database_path(root: &Path) -> PathBuf {
    root.join(".daena/index.sqlite")
}

pub struct ProjectStore {
    connection: Connection,
    root: Option<PathBuf>,
    pending_asset_imports: Mutex<BTreeMap<String, Vec<u8>>>,
}

impl ProjectStore {
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
        let repository = crate::storage::FilesystemRepository::open(root)?;
        for directory in [
            "entities",
            "plugins",
            ".daena",
            ".daena/transactions",
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
        crate::transactions::recover_transactions(root)?;
        let metadata_path = root.join("project.json");
        if !metadata_path.exists() {
            let name = root
                .file_name()
                .and_then(|value| value.to_str())
                .unwrap_or("Daena Archive project");
            let metadata = crate::storage::ProjectManifest::new(name);
            metadata.validate(&metadata_path)?;
            crate::storage::write_json(&metadata_path, &metadata)?;
        } else {
            let metadata =
                crate::storage::read_json::<crate::storage::ProjectManifest>(&metadata_path)?;
            metadata.validate(&metadata_path)?;
        }
        let gitignore = root.join(".gitignore");
        let required_gitignore = [".daena/", "daena.sqlite", "daena.sqlite-*"];
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
        let canonical = repository.scan()?;
        let index_path = project_database_path(root);
        if index_path.is_file() {
            if let Ok(store) = Self::open_database(&index_path, Some(root.to_path_buf())) {
                if store
                    .source_index_matches(&canonical.sources)?
                    && store.verify_index(canonical.sources.len()).is_ok()
                {
                    return Ok(store);
                }
            }
        }
        Self::rebuild_directory_index(root, &canonical)
    }

    fn rebuild_directory_index(
        root: &Path,
        canonical: &crate::storage::CanonicalProject,
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

        let store = Self::open_database(&next_path, Some(root.to_path_buf()))?;
        let payload = serde_json::to_string(&canonical.snapshot)
            .map_err(|error| CoreError::Serialization(error.to_string()))?;
        store.import_json_with_mode_and_sync_with_request_and_search(
            &payload,
            true,
            false,
            None,
            false,
        )?;
        store.replace_source_index(&canonical.sources)?;
        store.rebuild_search()?;
        store.verify_index(canonical.sources.len())?;
        store.connection.execute_batch(
            "PRAGMA wal_checkpoint(TRUNCATE);
             PRAGMA journal_mode=DELETE;",
        )?;
        drop(store);

        // Both paths are in .daena, so rename is atomic on the supported
        // local filesystems.  The old index is never opened or modified while
        // the new database is being built.
        std::fs::rename(&next_path, &index_path).map_err(|error| CoreError::Io {
            operation: "replace disposable index",
            source: error,
        })?;
        for suffix in ["-wal", "-shm", "-journal"] {
            let path = PathBuf::from(format!("{}{}", index_path.display(), suffix));
            let _ = std::fs::remove_file(path);
        }
        Self::open_database(&index_path, Some(root.to_path_buf()))
    }

    fn open_database(path: impl AsRef<Path>, root: Option<PathBuf>) -> Result<Self, CoreError> {
        let connection = Connection::open(path)?;
        connection.pragma_update(None, "foreign_keys", true)?;
        let store = Self {
            connection,
            root,
            pending_asset_imports: Mutex::new(BTreeMap::new()),
        };
        store.initialize()?;
        Ok(store)
    }

    fn replace_source_index(
        &self,
        sources: &[crate::storage::CanonicalSource],
    ) -> Result<(), CoreError> {
        let transaction = self.connection.unchecked_transaction()?;
        transaction.execute("DELETE FROM source_files", [])?;
        for source in sources {
            transaction.execute(
                "INSERT INTO source_files(path,content_hash,format_version,logical_revision) VALUES (?1,?2,?3,?2)",
                params![source.path, source.content_hash, source.format_version],
            )?;
        }
        transaction.commit()?;
        Ok(())
    }

    fn update_source_index(
        &self,
        previous: &[crate::storage::CanonicalSource],
        current: &[crate::storage::CanonicalSource],
    ) -> Result<(), CoreError> {
        let previous = previous
            .iter()
            .map(|source| (source.path.as_str(), source))
            .collect::<BTreeMap<_, _>>();
        let current = current
            .iter()
            .map(|source| (source.path.as_str(), source))
            .collect::<BTreeMap<_, _>>();
        let paths = previous
            .keys()
            .copied()
            .chain(current.keys().copied())
            .collect::<BTreeSet<_>>();
        let transaction = self.connection.unchecked_transaction()?;
        for path in paths {
            match (previous.get(path), current.get(path)) {
                (Some(before), Some(after)) if *before == *after => {}
                (Some(_), Some(after)) => {
                    transaction.execute(
                        "INSERT INTO source_files(path,content_hash,format_version,logical_revision) VALUES (?1,?2,?3,?2) ON CONFLICT(path) DO UPDATE SET content_hash=excluded.content_hash,format_version=excluded.format_version,logical_revision=excluded.logical_revision",
                        params![after.path, after.content_hash, after.format_version],
                    )?;
                }
                (None, Some(after)) => {
                    transaction.execute(
                        "INSERT INTO source_files(path,content_hash,format_version,logical_revision) VALUES (?1,?2,?3,?2)",
                        params![after.path, after.content_hash, after.format_version],
                    )?;
                }
                (Some(_), None) => {
                    transaction.execute("DELETE FROM source_files WHERE path=?1", params![path])?;
                }
                (None, None) => unreachable!("source path came from neither source set"),
            }
        }
        transaction.commit()?;
        Ok(())
    }

    fn update_source_index_from_hashes(
        &self,
        previous: &BTreeMap<String, String>,
        current: &[crate::storage::CanonicalSource],
    ) -> Result<(), CoreError> {
        let current = current
            .iter()
            .map(|source| (source.path.as_str(), source))
            .collect::<BTreeMap<_, _>>();
        let paths = previous
            .keys()
            .map(|path| path.as_str())
            .chain(current.keys().copied())
            .collect::<BTreeSet<_>>();
        let transaction = self.connection.unchecked_transaction()?;
        for path in paths {
            match (previous.get(path), current.get(path)) {
                (Some(before), Some(after)) if before == &after.content_hash => {}
                (Some(_), Some(after)) => {
                    transaction.execute(
                        "INSERT INTO source_files(path,content_hash,format_version,logical_revision) VALUES (?1,?2,?3,?2) ON CONFLICT(path) DO UPDATE SET content_hash=excluded.content_hash,format_version=excluded.format_version,logical_revision=excluded.logical_revision",
                        params![after.path, after.content_hash, after.format_version],
                    )?;
                }
                (None, Some(after)) => {
                    transaction.execute(
                        "INSERT INTO source_files(path,content_hash,format_version,logical_revision) VALUES (?1,?2,?3,?2)",
                        params![after.path, after.content_hash, after.format_version],
                    )?;
                }
                (Some(_), None) => {
                    transaction.execute("DELETE FROM source_files WHERE path=?1", params![path])?;
                }
                (None, None) => unreachable!("source path came from neither source set"),
            }
        }
        transaction.commit()?;
        Ok(())
    }

    fn verify_index_after_mutation(&self, source_count: usize) -> Result<(), CoreError> {
        let foreign_key_error: Option<String> = self
            .connection
            .query_row("PRAGMA foreign_key_check", [], |row| row.get(0))
            .optional()?;
        if let Some(error) = foreign_key_error {
            return Err(CoreError::Validation(format!(
                "index foreign-key check failed: {error}"
            )));
        }
        let quick_check: String = self
            .connection
            .query_row("PRAGMA quick_check", [], |row| row.get(0))?;
        if quick_check != "ok" {
            return Err(CoreError::Validation(format!(
                "index quick check failed: {quick_check}"
            )));
        }
        let actual: usize = self
            .connection
            .query_row("SELECT COUNT(*) FROM source_files", [], |row| row.get(0))?;
        if actual != source_count {
            return Err(CoreError::Validation(format!(
                "index source count mismatch: expected {source_count}, found {actual}"
            )));
        }
        Ok(())
    }

    fn verify_index(&self, source_count: usize) -> Result<(), CoreError> {
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
        let actual: usize =
            self.connection
                .query_row("SELECT COUNT(*) FROM source_files", [], |row| row.get(0))?;
        if actual != source_count {
            return Err(CoreError::Validation(format!(
                "index source count mismatch: expected {source_count}, found {actual}"
            )));
        }
        Ok(())
    }

    fn ensure_source_index_current(&self) -> Result<(), CoreError> {
        let Some(root) = self.root.as_deref() else {
            return Ok(());
        };
        let canonical = crate::storage::FilesystemRepository::open(root)?.scan()?;
        let expected = canonical
            .sources
            .into_iter()
            .map(|source| (source.path, (source.content_hash, source.format_version)))
            .collect::<BTreeMap<_, _>>();
        let mut actual = BTreeMap::new();
        let mut statement = self
            .connection
            .prepare("SELECT path,content_hash,format_version FROM source_files")?;
        let rows = statement.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                (row.get::<_, String>(1)?, row.get::<_, u32>(2)?),
            ))
        })?;
        for row in rows {
            let (path, source) = row?;
            actual.insert(path, source);
        }
        if actual != expected {
            return Err(CoreError::Validation(
                "disposable index is stale relative to canonical source files".into(),
            ));
        }
        Ok(())
    }

    fn source_index_matches(
        &self,
        sources: &[crate::storage::CanonicalSource],
    ) -> Result<bool, CoreError> {
        let expected = sources
            .iter()
            .map(|source| {
                (
                    source.path.clone(),
                    (source.content_hash.clone(), source.format_version),
                )
            })
            .collect::<BTreeMap<_, _>>();
        let mut actual = BTreeMap::new();
        let mut statement = self
            .connection
            .prepare("SELECT path,content_hash,format_version FROM source_files")?;
        let rows = statement.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                (row.get::<_, String>(1)?, row.get::<_, u32>(2)?),
            ))
        })?;
        for row in rows {
            let (path, source) = row?;
            actual.insert(path, source);
        }
        Ok(actual == expected)
    }

    fn indexed_source_hashes(&self) -> Result<BTreeMap<String, String>, CoreError> {
        let mut statement = self
            .connection
            .prepare("SELECT path,content_hash FROM source_files ORDER BY path")?;
        let rows = statement.query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?;
        rows.collect::<Result<BTreeMap<_, _>, _>>()
            .map_err(CoreError::from)
    }

    /// Reconcile canonical files after an external filesystem change.
    ///
    /// The scanner is deliberately authoritative here: a valid snapshot is
    /// imported into the disposable index, while an invalid snapshot is only
    /// reported and never replaces the last valid projection.
    pub fn reconcile_external_changes(&self) -> Result<ExternalChangeReport, CoreError> {
        let Some(root) = self.root.as_deref() else {
            return Ok(ExternalChangeReport {
                changed: false,
                paths: Vec::new(),
                diagnostics: Vec::new(),
            });
        };
        let git_unmerged = self.git_unmerged_paths();
        if !git_unmerged.is_empty() {
            return Ok(ExternalChangeReport {
                changed: false,
                paths: git_unmerged.clone(),
                diagnostics: git_unmerged
                    .iter()
                    .map(|path| format!("git.unmerged: {path}"))
                    .collect(),
            });
        }
        let previous = self.indexed_source_hashes()?;
        let repository = crate::storage::FilesystemRepository::open(root)?;
        let canonical = match repository.scan() {
            Ok(canonical) => canonical,
            Err(error) => {
                return Ok(ExternalChangeReport {
                    changed: false,
                    paths: Vec::new(),
                    diagnostics: vec![error.to_string()],
                });
            }
        };
        let current = canonical
            .sources
            .iter()
            .map(|source| (source.path.clone(), source.content_hash.clone()))
            .collect::<BTreeMap<_, _>>();
        let paths = previous
            .keys()
            .chain(current.keys())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .filter(|path| previous.get(*path) != current.get(*path))
            .cloned()
            .collect::<Vec<_>>();
        if paths.is_empty() {
            return Ok(ExternalChangeReport {
                changed: false,
                paths,
                diagnostics: Vec::new(),
            });
        }

        let payload = serde_json::to_string(&canonical.snapshot)
            .map_err(|error| CoreError::Serialization(error.to_string()))?;
        self.import_json_with_mode_and_sync_with_request_and_search(
            &payload,
            true,
            false,
            None,
            false,
        )?;
        self.update_source_index_from_hashes(&previous, &canonical.sources)?;
        self.rebuild_search()?;
        self.verify_index_after_mutation(canonical.sources.len())?;
        Ok(ExternalChangeReport {
            changed: true,
            paths,
            diagnostics: Vec::new(),
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

    fn require_map_entity(&self, entity_id: &str) -> Result<(), CoreError> {
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

    fn map_recovery_dir(&self, create: bool) -> Result<std::path::PathBuf, CoreError> {
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

    /// Writes a rejected map save draft to `.daena/conflicts/maps/`. The
    /// original source asset is never overwritten without review; the draft
    /// lives only in the disposable derived state directory.
    pub fn save_map_recovery_copy(
        &self,
        entity_id: &str,
        bytes: &[u8],
    ) -> Result<String, CoreError> {
        self.require_map_entity(entity_id)?;
        if bytes.len() > crate::maps::MAP_RECOVERY_MAX_BYTES {
            return Err(CoreError::Validation(
                "map recovery copy exceeds the host transfer limit".into(),
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

    /// Replaces a map source asset with a previously exported recovery draft.
    /// This is an explicit user action, so the revision preflight runs against
    /// the current revision instead of a stale one.
    pub fn restore_map_recovery_copy(
        &self,
        entity_id: &str,
        file_name: &str,
        request_id: Option<&str>,
    ) -> Result<Asset, CoreError> {
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
        if bytes.len() > crate::maps::MAP_RECOVERY_MAX_BYTES {
            return Err(CoreError::Validation(
                "map recovery copy exceeds the host transfer limit".into(),
            ));
        }
        let descriptor: crate::maps::MapDescriptor = self
            .list_fields(entity_id.into())?
            .into_iter()
            .find(|field| {
                field.namespace == crate::maps::MAP_NAMESPACE && field.key == "map"
            })
            .map(|field| {
                serde_json::from_value(field.value)
                    .map_err(|error| CoreError::Serialization(error.to_string()))
            })
            .transpose()?
            .ok_or_else(|| CoreError::NotFound("map descriptor not found".into()))?;
        let Some(source_asset_id) = descriptor.source_asset_id else {
            return Err(CoreError::NotFound(
                "map has no saved source asset to restore".into(),
            ));
        };
        let mut source = self.asset_unchecked(&source_asset_id)?;
        let expected_revision = self.revision_for_asset(&source.id)?;
        let digest = format!("sha256:{}", digest_bytes(&bytes));
        self.replace_asset_bytes_with_request(
            AssetReplaceInput {
                asset_id: source_asset_id,
                content_hash: digest,
                size: bytes.len() as i64,
                mime_type: std::mem::take(&mut source.mime_type),
            },
            bytes,
            &expected_revision,
            request_id,
        )
    }


    fn revision_for_entity(&self, entity_id: &str) -> Result<String, CoreError> {
        let entity = self.connection.query_row(
            "SELECT id,name,entity_type,deleted,created_at,updated_at FROM entities WHERE id=?1",
            params![entity_id],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, Option<String>>(2)?,
                    row.get::<_, i64>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, String>(5)?,
                ))
            },
        )?;
        let documents = self
            .connection
            .prepare(
                "SELECT id,format,body,updated_at FROM documents WHERE entity_id=?1 ORDER BY id",
            )?
            .query_map(params![entity_id], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                ))
            })?
            .collect::<Result<Vec<_>, _>>()?;
        let fields = self
            .connection
            .prepare("SELECT namespace,key,value FROM entity_fields WHERE entity_id=?1 ORDER BY namespace,key")?
            .query_map(params![entity_id], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                ))
            })?
            .collect::<Result<Vec<_>, _>>()?;
        let relationships = self
            .connection
            .prepare("SELECT id,source_id,target_id,relationship_type,metadata FROM relationships WHERE source_id=?1 OR target_id=?1 ORDER BY id")?
            .query_map(params![entity_id], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                ))
            })?
            .collect::<Result<Vec<_>, _>>()?;
        let assets = self
            .connection
            .prepare("SELECT id,namespace,filename,content_hash,size,mime_type,path,created_at FROM assets WHERE entity_id=?1 ORDER BY id")?
            .query_map(params![entity_id], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, i64>(4)?,
                    row.get::<_, String>(5)?,
                    row.get::<_, String>(6)?,
                    row.get::<_, String>(7)?,
                ))
            })?
            .collect::<Result<Vec<_>, _>>()?;
        let source_hashes = self.canonical_source_hashes(&format!("entities/{entity_id}/%"))?;
        let revision = revision_digest(&(
            entity,
            documents,
            fields,
            relationships,
            assets,
            source_hashes,
        ))?;
        Ok(revision)
    }

    fn revision_for_document(&self, id: &str) -> Result<String, CoreError> {
        let value = self.connection.query_row(
            "SELECT id,entity_id,format,body,updated_at FROM documents WHERE id=?1",
            params![id],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                ))
            },
        )?;
        let source_hashes =
            self.canonical_source_hashes(&format!("entities/{}/document.md", value.1))?;
        revision_digest(&(value, source_hashes))
    }

    fn revision_for_field(&self, field: &FieldValue) -> Result<String, CoreError> {
        let source_hashes =
            self.canonical_source_hashes(&format!("entities/{}/fields/%", field.entity_id))?;
        revision_digest(&(
            &field.entity_id,
            &field.namespace,
            &field.key,
            encode_field_value(&field.value)?,
            source_hashes,
        ))
    }

    fn revision_for_relationship(&self, id: &str) -> Result<String, CoreError> {
        let value = self.connection.query_row(
            "SELECT id,source_id,target_id,relationship_type,metadata FROM relationships WHERE id=?1",
            params![id],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                ))
            },
        )?;
        let source_hashes =
            self.canonical_source_hashes(&format!("entities/{}/relationships.json", value.1))?;
        revision_digest(&(value, source_hashes))
    }

    fn revision_for_asset(&self, id: &str) -> Result<String, CoreError> {
        let value = self.connection.query_row(
            "SELECT id,entity_id,namespace,filename,content_hash,size,mime_type,path,created_at FROM assets WHERE id=?1",
            params![id],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, i64>(5)?,
                    row.get::<_, String>(6)?,
                    row.get::<_, String>(7)?,
                    row.get::<_, String>(8)?,
                ))
            },
        )?;
        let source_hashes = self.canonical_source_hashes(&value.7)?;
        revision_digest(&(value, source_hashes))
    }

    fn canonical_source_hashes(&self, pattern: &str) -> Result<Vec<(String, String)>, CoreError> {
        self.connection
            .prepare(
                "SELECT path,content_hash FROM source_files WHERE path=?1 OR path LIKE ?1 ORDER BY path",
            )?
            .query_map(params![pattern], |row| Ok((row.get(0)?, row.get(1)?)))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(CoreError::from)
    }

    fn sync_canonical(&self) -> Result<(), CoreError> {
        let request_id = Uuid::new_v4().to_string();
        self.sync_canonical_with_request_id(&request_id, None)
    }

    fn sync_canonical_with_request_id(
        &self,
        request_id: &str,
        result: Option<&serde_json::Value>,
    ) -> Result<(), CoreError> {
        self.sync_canonical_with_request_id_inner(request_id, result)
    }

    fn sync_canonical_with_request_id_inner(
        &self,
        request_id: &str,
        result: Option<&serde_json::Value>,
    ) -> Result<(), CoreError> {
        let Some(root) = self.root.as_deref() else {
            return Ok(());
        };
        let manifest_path = root.join("project.json");
        let manifest: crate::storage::ProjectManifest = crate::storage::read_json(&manifest_path)?;
        manifest.validate(&manifest_path)?;
        let snapshot: ProjectSnapshot = serde_json::from_str(&self.export_json_inner()?)
            .map_err(|error| CoreError::Serialization(error.to_string()))?;
        let pending_assets = self
            .pending_asset_imports
            .lock()
            .map_err(|_| CoreError::Conflict("asset import state is poisoned".into()))?
            .clone();
        self.commit_canonical_snapshot(
            root,
            &snapshot,
            request_id,
            result,
            &pending_assets,
            &BTreeMap::new(),
        )
    }

    fn commit_canonical_snapshot(
        &self,
        root: &Path,
        snapshot: &ProjectSnapshot,
        request_id: &str,
        result: Option<&serde_json::Value>,
        pending_assets: &BTreeMap<String, Vec<u8>>,
        additional_files: &BTreeMap<String, Vec<u8>>,
    ) -> Result<(), CoreError> {
        let manifest_path = root.join("project.json");
        let manifest: crate::storage::ProjectManifest = crate::storage::read_json(&manifest_path)?;
        manifest.validate(&manifest_path)?;
        let previous_canonical = crate::storage::FilesystemRepository::open(root)?.scan()?;
        let mut transaction = match crate::transactions::FileTransaction::begin(root, request_id)? {
            crate::transactions::TransactionStart::Ready(transaction) => transaction,
            crate::transactions::TransactionStart::AlreadyCommitted => {
                return Err(CoreError::Conflict(
                    "generated transaction request ID was already committed".into(),
                ));
            }
        };
        let staging_root = transaction.staging_root();
        std::fs::create_dir_all(&staging_root).map_err(|error| CoreError::Io {
            operation: "create canonical transaction staging root",
            source: error,
        })?;
        for source in &previous_canonical.sources {
            let source_path = crate::storage::normalized_project_path(root, &source.path)?;
            let staged_path = crate::storage::normalized_project_path(&staging_root, &source.path)?;
            if let Some(parent) = staged_path.parent() {
                std::fs::create_dir_all(parent).map_err(|error| CoreError::Io {
                    operation: "create canonical transaction staging parent",
                    source: error,
                })?;
            }
            std::fs::copy(&source_path, &staged_path).map_err(|error| CoreError::Io {
                operation: "copy canonical data into transaction staging",
                source: error,
            })?;
        }
        for (relative_path, bytes) in pending_assets {
            let staged_path =
                crate::storage::normalized_project_path(&staging_root, relative_path)?;
            if let Some(parent) = staged_path.parent() {
                std::fs::create_dir_all(parent).map_err(|error| CoreError::Io {
                    operation: "create staged asset parent",
                    source: error,
                })?;
            }
            std::fs::write(&staged_path, bytes).map_err(|error| CoreError::Io {
                operation: "write staged asset import",
                source: error,
            })?;
        }
        for (target, bytes) in additional_files {
            transaction.stage_bytes(target, bytes)?;
        }
        crate::storage::write_canonical_project(&staging_root, &manifest, snapshot)?;
        let staged = crate::storage::FilesystemRepository::open(&staging_root)?.scan()?;
        let canonical_hashes = previous_canonical
            .sources
            .iter()
            .map(|source| (source.path.as_str(), source.content_hash.as_str()))
            .collect::<BTreeMap<_, _>>();
        let staged_paths = staged
            .sources
            .iter()
            .map(|source| source.path.clone())
            .collect::<BTreeSet<_>>();
        for source in &staged.sources {
            // Canonical serialization still provides the complete, validated
            // projection in staging, but the transaction should contain only
            // the actual change set.  This preserves the journal's optimistic
            // revision checks and crash recovery while avoiding a staged file,
            // hash, journal entry, and fsync for every unrelated source.
            if canonical_hashes
                .get(source.path.as_str())
                .is_some_and(|hash| *hash == source.content_hash)
            {
                continue;
            }
            let path = crate::storage::normalized_project_path(&staging_root, &source.path)?;
            let bytes = std::fs::read(&path).map_err(|error| CoreError::Io {
                operation: "read staged canonical data",
                source: error,
            })?;
            transaction.stage_bytes(&source.path, &bytes)?;
        }
        for source in &previous_canonical.sources {
            if !staged_paths.contains(&source.path) {
                transaction.stage_remove(&source.path)?;
            }
        }
        if let Some(result) = result {
            transaction.commit_with_result(result)?;
        } else {
            transaction.commit()?;
        }
        if !pending_assets.is_empty() {
            let mut pending = self
                .pending_asset_imports
                .lock()
                .map_err(|_| CoreError::Conflict("asset import state is poisoned".into()))?;
            for path in pending_assets.keys() {
                pending.remove(path);
            }
        }
        let canonical = crate::storage::FilesystemRepository::open(root)?.scan()?;
        let payload = serde_json::to_string(&canonical.snapshot)
            .map_err(|error| CoreError::Serialization(error.to_string()))?;
        // The journal is the commit point.  Only after it succeeds do we
        // replace the disposable projection with a fresh interpretation of
        // the canonical files.
        self.import_json_with_mode_and_sync_with_request_and_search(
            &payload,
            true,
            false,
            None,
            false,
        )?;
        self.update_source_index(&previous_canonical.sources, &canonical.sources)?;
        self.rebuild_search()?;
        self.verify_index_after_mutation(canonical.sources.len())
    }

    fn repository_first_mutation<T, F>(
        &self,
        request_id: Option<&str>,
        operation: F,
    ) -> Result<T, CoreError>
    where
        T: Serialize + DeserializeOwned,
        F: FnOnce(&mut ProjectStore, &str) -> Result<T, CoreError>,
    {
        let Some(root) = self.root.as_deref() else {
            return Err(CoreError::Validation(
                "repository-first mutation requires a directory-backed project".into(),
            ));
        };
        if let Some(result) = self.committed_mutation::<T>(request_id)? {
            return self.refresh_mutation_revision(result);
        }
        let request_id = self.request_id(request_id)?;
        let canonical = crate::storage::FilesystemRepository::open(root)?.scan()?;
        let mut projection = Self::open_database(":memory:", None)?;
        let payload = serde_json::to_string(&canonical.snapshot)
            .map_err(|error| CoreError::Serialization(error.to_string()))?;
        projection.import_json_with_mode_and_sync(&payload, true, false)?;
        projection.replace_source_index(&canonical.sources)?;
        let result = operation(&mut projection, &request_id)?;
        let snapshot: ProjectSnapshot = serde_json::from_str(&projection.export_json_inner()?)
            .map_err(|error| CoreError::Serialization(error.to_string()))?;
        let pending_assets = projection
            .pending_asset_imports
            .lock()
            .map_err(|_| CoreError::Conflict("asset import state is poisoned".into()))?
            .clone();
        let result_value = serde_json::to_value(&result)
            .map_err(|error| CoreError::Serialization(error.to_string()))?;
        self.commit_canonical_snapshot(
            root,
            &snapshot,
            &request_id,
            Some(&result_value),
            &pending_assets,
            &BTreeMap::new(),
        )?;
        self.refresh_mutation_revision(result)
    }

    fn refresh_mutation_revision<T>(&self, result: T) -> Result<T, CoreError>
    where
        T: Serialize + DeserializeOwned,
    {
        let mut value = serde_json::to_value(result)
            .map_err(|error| CoreError::Serialization(error.to_string()))?;
        let Some(object) = value.as_object_mut() else {
            return serde_json::from_value(value)
                .map_err(|error| CoreError::Serialization(error.to_string()));
        };
        let Some(id) = object.get("id").and_then(serde_json::Value::as_str) else {
            return serde_json::from_value(value)
                .map_err(|error| CoreError::Serialization(error.to_string()));
        };
        let revision = if object.contains_key("source_id") {
            self.revision_for_relationship(id)?
        } else if object.contains_key("path") && object.contains_key("content_hash") {
            self.revision_for_asset(id)?
        } else if object.contains_key("entity_id") && object.contains_key("format") {
            self.revision_for_document(id)?
        } else {
            self.revision_for_entity(id)?
        };
        object.insert("revision".into(), serde_json::Value::String(revision));
        serde_json::from_value(value).map_err(|error| CoreError::Serialization(error.to_string()))
    }

    pub fn in_memory() -> Result<Self, CoreError> {
        Self::open_database(":memory:", None)
    }

    pub fn info(&self) -> Option<ProjectInfo> {
        let root = self.root.as_ref()?;
        Some(ProjectInfo {
            name: crate::storage::read_json::<crate::storage::ProjectManifest>(
                &root.join("project.json"),
            )
            .map(|manifest| manifest.name)
            .unwrap_or_else(|_| {
                root.file_name()
                    .and_then(|value| value.to_str())
                    .unwrap_or("Daena Archive project")
                    .to_string()
            }),
            root: root.to_string_lossy().to_string(),
            index_status: if self.ensure_source_index_current().is_ok() {
                "ready"
            } else {
                "diagnostic"
            }
            .into(),
            assets: root.join("assets").to_string_lossy().to_string(),
        })
    }

    fn project_root(&self) -> Result<&Path, CoreError> {
        self.root
            .as_deref()
            .ok_or_else(|| CoreError::NotFound("project is not directory-backed".into()))
    }

    fn request_id(&self, request_id: Option<&str>) -> Result<String, CoreError> {
        let request_id = request_id
            .map(str::to_owned)
            .unwrap_or_else(|| Uuid::new_v4().to_string());
        Uuid::parse_str(&request_id)
            .map_err(|_| CoreError::Validation("mutation request ID must be a UUID".into()))?;
        Ok(request_id)
    }

    fn committed_mutation<T: DeserializeOwned>(
        &self,
        request_id: Option<&str>,
    ) -> Result<Option<T>, CoreError> {
        let Some(request_id) = request_id else {
            return Ok(None);
        };
        let Some(root) = self.root.as_deref() else {
            return Ok(None);
        };
        let Some(result) = crate::transactions::committed_result(root, request_id)? else {
            return Ok(None);
        };
        serde_json::from_value(result)
            .map(Some)
            .map_err(|error| CoreError::Serialization(error.to_string()))
    }

    fn ensure_expected_revision(
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

    fn run_git(&self, args: &[&str]) -> Result<std::process::Output, CoreError> {
        let root = self.project_root()?;
        Command::new("git")
            .args(args)
            .current_dir(root)
            .output()
            .map_err(|error| CoreError::NotFound(format!("git is unavailable: {error}")))
    }

    fn git_unmerged_paths(&self) -> Vec<String> {
        let Ok(status) = self.git_status() else {
            return Vec::new();
        };
        if !status.repository {
            return Vec::new();
        }
        status
            .changes
            .into_iter()
            .filter_map(|change| {
                let bytes = change.as_bytes();
                if bytes.len() < 4 {
                    return None;
                }
                let unmerged = matches!(
                    &bytes[..2],
                    b"DD" | b"AU" | b"UD" | b"UA" | b"DU" | b"AA" | b"UU"
                );
                unmerged.then(|| change[3..].trim().to_string())
            })
            .collect()
    }

    fn git_status_entries(&self) -> Result<Vec<(u8, u8, String)>, CoreError> {
        let output = self.run_git(&["status", "--porcelain=v1", "-z", "--untracked-files=all"])?;
        if !output.status.success() {
            return Err(CoreError::Git(
                String::from_utf8_lossy(&output.stderr).trim().to_string(),
            ));
        }
        let mut entries = Vec::new();
        let mut records = output.stdout.split(|byte| *byte == 0);
        while let Some(record) = records.next() {
            if record.len() < 4 || record[2] != b' ' {
                continue;
            }
            let index_status = record[0];
            let worktree_status = record[1];
            let mut path = String::from_utf8_lossy(&record[3..]).into_owned();
            if matches!(index_status, b'R' | b'C') || matches!(worktree_status, b'R' | b'C') {
                if let Some(new_path) = records.next() {
                    path = String::from_utf8_lossy(new_path).into_owned();
                }
            }
            if !path.is_empty() {
                entries.push((index_status, worktree_status, path));
            }
        }
        Ok(entries)
    }

    fn git_staged_paths(&self) -> Result<Vec<String>, CoreError> {
        let output = self.run_git(&["diff", "--cached", "--name-only", "-z"])?;
        if !output.status.success() {
            return Err(CoreError::Git(
                String::from_utf8_lossy(&output.stderr).trim().to_string(),
            ));
        }
        Ok(output
            .stdout
            .split(|byte| *byte == 0)
            .filter(|path| !path.is_empty())
            .map(|path| String::from_utf8_lossy(path).into_owned())
            .collect())
    }

    fn is_canonical_git_path(path: &str) -> bool {
        path == "project.json"
            || path == ".gitignore"
            || path.starts_with("entities/")
            || path.starts_with("plugins/")
            || path.starts_with("assets/")
    }

    fn is_asset_git_path(path: &str) -> bool {
        path.starts_with("assets/")
    }

    pub fn git_status(&self) -> Result<GitStatus, CoreError> {
        let repository = self.run_git(&["rev-parse", "--is-inside-work-tree"])?;
        if !repository.status.success() {
            return Ok(GitStatus {
                repository: false,
                branch: None,
                changes: Vec::new(),
                canonical_changes: Vec::new(),
                staged_canonical_changes: Vec::new(),
            });
        }
        let branch = self.run_git(&["branch", "--show-current"])?;
        let entries = self.git_status_entries()?;
        let changes = entries
            .iter()
            .map(|(index_status, worktree_status, path)| {
                format!(
                    "{}{} {}",
                    *index_status as char, *worktree_status as char, path
                )
            })
            .collect::<Vec<_>>();
        let canonical_changes = entries
            .iter()
            .filter(|(_, _, path)| Self::is_canonical_git_path(path))
            .map(|(_, _, path)| path.clone())
            .collect::<Vec<_>>();
        let staged_canonical_changes = self
            .git_staged_paths()?
            .into_iter()
            .filter(|path| Self::is_canonical_git_path(path))
            .collect();
        Ok(GitStatus {
            repository: true,
            branch: Some(String::from_utf8_lossy(&branch.stdout).trim().to_string()),
            changes,
            canonical_changes,
            staged_canonical_changes,
        })
    }

    pub fn git_init(&self) -> Result<GitStatus, CoreError> {
        let output = self.run_git(&["init"])?;
        if !output.status.success() {
            return Err(CoreError::NotFound(
                String::from_utf8_lossy(&output.stderr).trim().into(),
            ));
        }
        self.git_status()
    }

    pub fn git_log(&self) -> Result<Vec<GitLogEntry>, CoreError> {
        if !self.git_status()?.repository {
            return Ok(Vec::new());
        }
        let output = self.run_git(&[
            "log",
            "-50",
            "--date=short",
            "--pretty=format:%h%x09%ad%x09%s",
        ])?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            if stderr.contains("does not have any commits yet") {
                return Ok(Vec::new());
            }
            return Err(CoreError::NotFound(stderr.trim().into()));
        }
        Ok(String::from_utf8_lossy(&output.stdout)
            .lines()
            .filter_map(|line| {
                let mut parts = line.splitn(3, '\t');
                Some(GitLogEntry {
                    hash: parts.next()?.into(),
                    date: parts.next()?.into(),
                    subject: parts.next()?.into(),
                })
            })
            .collect())
    }

    pub fn git_preflight(&self) -> Result<GitPreflight, CoreError> {
        let status = self.git_status()?;
        if !status.repository {
            return Ok(GitPreflight {
                ready: false,
                diagnostics: vec!["project is not a git repository".into()],
                canonical_paths: Vec::new(),
                asset_paths: Vec::new(),
                staging_paths: Vec::new(),
                staged_paths: Vec::new(),
                unmerged_paths: Vec::new(),
            });
        }

        let root = self.project_root()?;
        crate::transactions::recover_transactions(root)?;
        let entries = self.git_status_entries()?;
        let staged_paths = self.git_staged_paths()?;
        let mut diagnostics = Vec::new();
        let unmerged_paths = entries
            .iter()
            .filter(|(index_status, worktree_status, _)| {
                matches!(
                    (*index_status, *worktree_status),
                    (b'D', b'D')
                        | (b'A', b'U')
                        | (b'U', b'D')
                        | (b'U', b'A')
                        | (b'D', b'U')
                        | (b'A', b'A')
                        | (b'U', b'U')
                )
            })
            .map(|(_, _, path)| path.clone())
            .collect::<Vec<_>>();
        if !unmerged_paths.is_empty() {
            diagnostics.extend(
                unmerged_paths
                    .iter()
                    .map(|path| format!("git.unmerged: {path}")),
            );
        }

        let report = self.reconcile_external_changes()?;
        diagnostics.extend(report.diagnostics);
        if diagnostics.is_empty() {
            if let Err(error) = self.ensure_source_index_current() {
                diagnostics.push(format!("index.stale: {error}"));
            }
        }

        let noncanonical_staged = staged_paths
            .iter()
            .filter(|path| !Self::is_canonical_git_path(path))
            .cloned()
            .collect::<Vec<_>>();
        if !noncanonical_staged.is_empty() {
            diagnostics.push(format!(
                "git.noncanonical-staged: {}",
                noncanonical_staged.join(", ")
            ));
        }

        let canonical_paths = entries
            .iter()
            .filter(|(_, _, path)| Self::is_canonical_git_path(path))
            .map(|(_, _, path)| path.clone())
            .chain(
                staged_paths
                    .iter()
                    .filter(|path| Self::is_canonical_git_path(path))
                    .cloned(),
            )
            .collect::<BTreeSet<_>>();
        let mut canonical_paths = canonical_paths.iter().cloned().collect::<Vec<_>>();
        let asset_paths = canonical_paths
            .iter()
            .filter(|path| Self::is_asset_git_path(path))
            .cloned()
            .collect::<Vec<_>>();
        let staging_paths = canonical_paths.clone();
        canonical_paths.shrink_to_fit();

        Ok(GitPreflight {
            ready: diagnostics.is_empty(),
            diagnostics,
            canonical_paths,
            asset_paths,
            staging_paths,
            staged_paths,
            unmerged_paths,
        })
    }

    pub fn git_staging_preview(&self) -> Result<GitPreflight, CoreError> {
        self.git_preflight()
    }

    pub fn git_commit(
        &self,
        message: String,
        paths: Option<Vec<String>>,
    ) -> Result<GitStatus, CoreError> {
        if message.trim().is_empty() {
            return Err(CoreError::NotFound("commit message cannot be empty".into()));
        }
        let preflight = self.git_preflight()?;
        if !preflight.ready {
            return Err(CoreError::Conflict(format!(
                "cannot commit while canonical diagnostics remain: {}",
                preflight.diagnostics.join("; ")
            )));
        }
        if preflight.staging_paths.is_empty() {
            return Err(CoreError::Git(
                "no canonical project changes to commit".into(),
            ));
        }
        let selected = match paths {
            Some(paths) if paths.is_empty() => {
                return Err(CoreError::Git("no paths selected for commit".into()));
            }
            Some(paths) => {
                let allowed = preflight.staging_paths.iter().cloned().collect::<BTreeSet<_>>();
                for path in &paths {
                    if !allowed.contains(path) {
                        return Err(CoreError::Git(format!(
                            "path is not in the canonical staging preview: {path}"
                        )));
                    }
                    if !Self::is_canonical_git_path(path) {
                        return Err(CoreError::Git(format!(
                            "path is not a canonical project path: {path}"
                        )));
                    }
                }
                paths
            }
            None => preflight.staging_paths.clone(),
        };
        // Rebuild the canonical portion of the index so a selective commit
        // cannot accidentally include canonical paths staged before the UI
        // preview. `git reset` without a worktree target preserves all edits.
        let mut reset_args = vec![
            "reset".to_string(),
            "--mixed".to_string(),
            "HEAD".to_string(),
            "--".to_string(),
        ];
        reset_args.extend(preflight.staging_paths.iter().cloned());
        let reset_args = reset_args.iter().map(String::as_str).collect::<Vec<_>>();
        let reset = self.run_git(&reset_args)?;
        if !reset.status.success() {
            return Err(CoreError::Git(
                String::from_utf8_lossy(&reset.stderr).trim().into(),
            ));
        }

        let mut add_args = vec!["add".to_string(), "--all".to_string(), "--".to_string()];
        add_args.extend(selected);
        let add_args = add_args.iter().map(String::as_str).collect::<Vec<_>>();
        let add = self.run_git(&add_args)?;
        if !add.status.success() {
            return Err(CoreError::Git(
                String::from_utf8_lossy(&add.stderr).trim().into(),
            ));
        }
        let commit = self.run_git(&["commit", "-m", message.trim()])?;
        if !commit.status.success() {
            return Err(CoreError::Git(
                String::from_utf8_lossy(&commit.stderr).trim().into(),
            ));
        }
        self.git_status()
    }

    pub fn git_tool_info() -> GitToolInfo {
        match Command::new("git").args(["--version"]).output() {
            Ok(output) if output.status.success() => {
                let version = String::from_utf8_lossy(&output.stdout).trim().to_string();
                GitToolInfo {
                    available: true,
                    version: Some(version),
                    error: None,
                }
            }
            Ok(output) => {
                let error = String::from_utf8_lossy(&output.stderr).trim().to_string();
                GitToolInfo {
                    available: false,
                    version: None,
                    error: Some(if error.is_empty() {
                        "git --version failed".into()
                    } else {
                        error
                    }),
                }
            }
            Err(error) => GitToolInfo {
                available: false,
                version: None,
                error: Some(format!("git is unavailable: {error}")),
            },
        }
    }

    fn git_rev_parse(&self, rev: &str) -> Result<Option<String>, CoreError> {
        let output = self.run_git(&["rev-parse", rev])?;
        if !output.status.success() {
            return Ok(None);
        }
        let hash = String::from_utf8_lossy(&output.stdout).trim().to_string();
        Ok((!hash.is_empty()).then_some(hash))
    }

    fn git_upstream(&self) -> Result<Option<GitUpstream>, CoreError> {
        let remote = self.run_git(&["rev-parse", "--abbrev-ref", "--symbolic-full-name", "@{u}"])?;
        if !remote.status.success() {
            return Ok(None);
        }
        let full = String::from_utf8_lossy(&remote.stdout).trim().to_string();
        let Some((remote_name, branch)) = full.split_once('/') else {
            return Ok(None);
        };
        let remote_hash = self.git_rev_parse("@{u}")?;
        Ok(Some(GitUpstream {
            remote: remote_name.to_string(),
            branch: branch.to_string(),
            remote_hash,
        }))
    }

    fn validate_remote_url(url: &str) -> Result<(), CoreError> {
        let url = url.trim();
        if url.is_empty() {
            return Err(CoreError::Validation("remote URL cannot be empty".into()));
        }
        let ok = url.starts_with("https://")
            || url.starts_with("http://")
            || url.starts_with("ssh://")
            || url.starts_with("git://")
            || url.starts_with("git@")
            || url.starts_with("file://")
            || Path::new(url).is_absolute();
        if !ok {
            return Err(CoreError::Validation(format!(
                "unsupported remote URL: {url}"
            )));
        }
        Ok(())
    }

    fn validate_remote_name(name: &str) -> Result<(), CoreError> {
        let name = name.trim();
        if name.is_empty() {
            return Err(CoreError::Validation("remote name cannot be empty".into()));
        }
        if !name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
        {
            return Err(CoreError::Validation(format!(
                "invalid remote name: {name}"
            )));
        }
        Ok(())
    }

    pub fn git_show_tree(&self, hash: &str) -> Result<Vec<String>, CoreError> {
        if !self.git_status()?.repository {
            return Err(CoreError::Git("project is not a git repository".into()));
        }
        let hash = hash.trim();
        if hash.is_empty() {
            return Err(CoreError::Validation("commit hash cannot be empty".into()));
        }
        let output = self.run_git(&["ls-tree", "-r", "--name-only", hash])?;
        if !output.status.success() {
            return Err(CoreError::Git(
                String::from_utf8_lossy(&output.stderr).trim().into(),
            ));
        }
        Ok(String::from_utf8_lossy(&output.stdout)
            .lines()
            .map(str::trim)
            .filter(|path| !path.is_empty() && Self::is_canonical_git_path(path))
            .map(str::to_string)
            .collect())
    }

    pub fn git_show_file(&self, hash: &str, path: &str) -> Result<String, CoreError> {
        if !self.git_status()?.repository {
            return Err(CoreError::Git("project is not a git repository".into()));
        }
        let hash = hash.trim();
        let path = path.trim();
        if hash.is_empty() || path.is_empty() {
            return Err(CoreError::Validation(
                "commit hash and path are required".into(),
            ));
        }
        if !Self::is_canonical_git_path(path) {
            return Err(CoreError::Validation(format!(
                "path is not a canonical project path: {path}"
            )));
        }
        if path.contains('\0') || path.starts_with('-') {
            return Err(CoreError::Validation("invalid snapshot path".into()));
        }
        let spec = format!("{hash}:{path}");
        let output = self.run_git(&["show", &spec])?;
        if !output.status.success() {
            return Err(CoreError::Git(
                String::from_utf8_lossy(&output.stderr).trim().into(),
            ));
        }
        if output.stdout.len() > GIT_SHOW_FILE_MAX_BYTES {
            return Err(CoreError::Validation(format!(
                "snapshot file is too large to preview ({} bytes)",
                output.stdout.len()
            )));
        }
        if output.stdout.iter().any(|byte| *byte == 0) {
            return Err(CoreError::Validation(
                "binary snapshot files cannot be previewed".into(),
            ));
        }
        String::from_utf8(output.stdout).map_err(|_| {
            CoreError::Validation("snapshot file is not valid UTF-8 text".into())
        })
    }

    pub fn git_reset_hard(&self, hash: &str) -> Result<GitResetResult, CoreError> {
        if !self.git_status()?.repository {
            return Err(CoreError::Git("project is not a git repository".into()));
        }
        let hash = hash.trim();
        if hash.is_empty() {
            return Err(CoreError::Validation("commit hash cannot be empty".into()));
        }
        let previous_head = self.git_rev_parse("HEAD")?;
        let verify = self.run_git(&["rev-parse", "--verify", hash])?;
        if !verify.status.success() {
            return Err(CoreError::Git(format!(
                "unknown commit: {}",
                String::from_utf8_lossy(&verify.stderr).trim()
            )));
        }
        let upstream_before = self.git_upstream()?;
        let reset = self.run_git(&["reset", "--hard", hash])?;
        if !reset.status.success() {
            return Err(CoreError::Git(
                String::from_utf8_lossy(&reset.stderr).trim().into(),
            ));
        }
        let rebuild = self.reconcile_external_changes()?;
        let current_head = self.git_rev_parse("HEAD")?;
        let upstream = self.git_upstream()?.or(upstream_before);
        let diverged_from_upstream = match (&upstream, &current_head) {
            (Some(upstream), Some(head)) => {
                upstream.remote_hash.as_ref().is_some_and(|remote| remote != head)
            }
            (Some(_), _) => true,
            _ => false,
        };
        Ok(GitResetResult {
            status: self.git_status()?,
            previous_head,
            current_head,
            upstream,
            diverged_from_upstream,
            rebuild,
        })
    }

    pub fn git_remote_list(&self) -> Result<Vec<GitRemote>, CoreError> {
        if !self.git_status()?.repository {
            return Ok(Vec::new());
        }
        let output = self.run_git(&["remote", "-v"])?;
        if !output.status.success() {
            return Err(CoreError::Git(
                String::from_utf8_lossy(&output.stderr).trim().into(),
            ));
        }
        let mut remotes = BTreeMap::<String, GitRemote>::new();
        for line in String::from_utf8_lossy(&output.stdout).lines() {
            let mut parts = line.split_whitespace();
            let Some(name) = parts.next() else { continue };
            let Some(url) = parts.next() else { continue };
            let mode = parts.next().unwrap_or("(fetch)");
            let entry = remotes.entry(name.to_string()).or_insert(GitRemote {
                name: name.to_string(),
                fetch_url: String::new(),
                push_url: String::new(),
            });
            if mode.contains("push") {
                entry.push_url = url.to_string();
            } else {
                entry.fetch_url = url.to_string();
            }
        }
        for remote in remotes.values_mut() {
            if remote.push_url.is_empty() {
                remote.push_url = remote.fetch_url.clone();
            }
            if remote.fetch_url.is_empty() {
                remote.fetch_url = remote.push_url.clone();
            }
        }
        Ok(remotes.into_values().collect())
    }

    pub fn git_remote_add(&self, name: &str, url: &str) -> Result<Vec<GitRemote>, CoreError> {
        if !self.git_status()?.repository {
            return Err(CoreError::Git("project is not a git repository".into()));
        }
        Self::validate_remote_name(name)?;
        Self::validate_remote_url(url)?;
        let output = self.run_git(&["remote", "add", name.trim(), url.trim()])?;
        if !output.status.success() {
            return Err(CoreError::Git(
                String::from_utf8_lossy(&output.stderr).trim().into(),
            ));
        }
        self.git_remote_list()
    }

    pub fn git_remote_set_url(&self, name: &str, url: &str) -> Result<Vec<GitRemote>, CoreError> {
        if !self.git_status()?.repository {
            return Err(CoreError::Git("project is not a git repository".into()));
        }
        Self::validate_remote_name(name)?;
        Self::validate_remote_url(url)?;
        let output = self.run_git(&["remote", "set-url", name.trim(), url.trim()])?;
        if !output.status.success() {
            return Err(CoreError::Git(
                String::from_utf8_lossy(&output.stderr).trim().into(),
            ));
        }
        self.git_remote_list()
    }

    pub fn git_remote_remove(&self, name: &str) -> Result<Vec<GitRemote>, CoreError> {
        if !self.git_status()?.repository {
            return Err(CoreError::Git("project is not a git repository".into()));
        }
        Self::validate_remote_name(name)?;
        let output = self.run_git(&["remote", "remove", name.trim()])?;
        if !output.status.success() {
            return Err(CoreError::Git(
                String::from_utf8_lossy(&output.stderr).trim().into(),
            ));
        }
        self.git_remote_list()
    }

    pub fn git_push(
        &self,
        remote: &str,
        branch: Option<&str>,
        force_with_lease: bool,
    ) -> Result<GitStatus, CoreError> {
        if !self.git_status()?.repository {
            return Err(CoreError::Git("project is not a git repository".into()));
        }
        Self::validate_remote_name(remote)?;
        let branch = match branch {
            Some(branch) if !branch.trim().is_empty() => branch.trim().to_string(),
            _ => {
                let status = self.git_status()?;
                status.branch.filter(|value| !value.is_empty()).ok_or_else(|| {
                    CoreError::Git("current branch is required for push".into())
                })?
            }
        };
        let mut args = vec!["push".to_string()];
        if force_with_lease {
            args.push("--force-with-lease".into());
        }
        args.push(remote.trim().into());
        args.push(branch);
        let args = args.iter().map(String::as_str).collect::<Vec<_>>();
        let output = self.run_git(&args)?;
        if !output.status.success() {
            return Err(CoreError::Git(
                String::from_utf8_lossy(&output.stderr).trim().into(),
            ));
        }
        self.git_status()
    }

    pub fn git_restore_from_upstream(&self) -> Result<GitResetResult, CoreError> {
        if !self.git_status()?.repository {
            return Err(CoreError::Git("project is not a git repository".into()));
        }
        let upstream = self.git_upstream()?.ok_or_else(|| {
            CoreError::Git("no upstream branch is configured for restore".into())
        })?;
        let previous_head = self.git_rev_parse("HEAD")?;
        let fetch = self.run_git(&["fetch", &upstream.remote, &upstream.branch])?;
        if !fetch.status.success() {
            return Err(CoreError::Git(
                String::from_utf8_lossy(&fetch.stderr).trim().into(),
            ));
        }
        let upstream_ref = format!("{}/{}", upstream.remote, upstream.branch);
        let reset = self.run_git(&["reset", "--hard", &upstream_ref])?;
        if !reset.status.success() {
            return Err(CoreError::Git(
                String::from_utf8_lossy(&reset.stderr).trim().into(),
            ));
        }
        let rebuild = self.reconcile_external_changes()?;
        let current_head = self.git_rev_parse("HEAD")?;
        let upstream = self.git_upstream()?;
        Ok(GitResetResult {
            status: self.git_status()?,
            previous_head,
            current_head,
            upstream,
            diverged_from_upstream: false,
            rebuild,
        })
    }

    fn initialize(&self) -> Result<(), CoreError> {
        self.connection.execute_batch(
            "PRAGMA journal_mode = WAL;
             CREATE TABLE IF NOT EXISTS project_meta (key TEXT PRIMARY KEY, value TEXT NOT NULL);
             INSERT OR IGNORE INTO project_meta(key, value) VALUES ('schema_version', '1');
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
             CREATE INDEX IF NOT EXISTS relationships_source_idx ON relationships(source_id);
             CREATE INDEX IF NOT EXISTS relationships_target_idx ON relationships(target_id);
             CREATE TABLE IF NOT EXISTS source_files (
               path TEXT PRIMARY KEY,
               content_hash TEXT NOT NULL,
               format_version INTEGER NOT NULL,
               logical_revision TEXT NOT NULL
             );"
        )?;
        self.connection.execute_batch("CREATE TABLE IF NOT EXISTS module_versions(module_id TEXT PRIMARY KEY, version INTEGER NOT NULL DEFAULT 0);
              CREATE TABLE IF NOT EXISTS module_state(module_id TEXT PRIMARY KEY, enabled INTEGER NOT NULL DEFAULT 1);
              CREATE TABLE IF NOT EXISTS module_package_versions(module_id TEXT PRIMARY KEY, package_version TEXT NOT NULL);
              CREATE TABLE IF NOT EXISTS module_namespaces(module_id TEXT NOT NULL, namespace TEXT NOT NULL, PRIMARY KEY(module_id, namespace));
              CREATE TABLE IF NOT EXISTS module_fields(module_id TEXT NOT NULL, namespace TEXT NOT NULL, key TEXT NOT NULL, field_type TEXT NOT NULL, required INTEGER NOT NULL, PRIMARY KEY(module_id, namespace, key));
              CREATE TABLE IF NOT EXISTS entity_fields(entity_id TEXT NOT NULL REFERENCES entities(id), namespace TEXT NOT NULL, key TEXT NOT NULL, value TEXT NOT NULL, PRIMARY KEY(entity_id, namespace, key));
             CREATE TABLE IF NOT EXISTS assets (id TEXT PRIMARY KEY, entity_id TEXT NOT NULL REFERENCES entities(id), namespace TEXT NOT NULL, filename TEXT NOT NULL, content_hash TEXT NOT NULL, size INTEGER NOT NULL, mime_type TEXT NOT NULL, path TEXT NOT NULL, created_at TEXT NOT NULL);
             CREATE TABLE IF NOT EXISTS map_projection (map_entity_id TEXT PRIMARY KEY, provider TEXT NOT NULL, source_asset_id TEXT NOT NULL, source_path TEXT, source_hash TEXT);
             CREATE TABLE IF NOT EXISTS map_location_projection (location_id TEXT PRIMARY KEY, entity_id TEXT NOT NULL, map_entity_id TEXT NOT NULL, label TEXT, role TEXT NOT NULL, anchor_kind TEXT NOT NULL, provider TEXT, feature_kind TEXT, feature_id TEXT, min_x REAL, min_y REAL, max_x REAL, max_y REAL, valid_from TEXT, valid_to TEXT, resolution TEXT NOT NULL);
             CREATE INDEX IF NOT EXISTS map_location_entity_idx ON map_location_projection(entity_id);
             CREATE INDEX IF NOT EXISTS map_location_map_idx ON map_location_projection(map_entity_id);")?;
        self.connection.execute_batch("CREATE TABLE IF NOT EXISTS migration_history(module_id TEXT NOT NULL, migration_id TEXT NOT NULL, from_version INTEGER NOT NULL, to_version INTEGER NOT NULL, checksum TEXT NOT NULL, package_digest TEXT NOT NULL DEFAULT '', applied_at TEXT NOT NULL DEFAULT '', PRIMARY KEY(module_id, migration_id)); CREATE TABLE IF NOT EXISTS plugin_backups(id TEXT PRIMARY KEY, module_id TEXT NOT NULL, from_package_version TEXT, to_package_version TEXT, data_version INTEGER NOT NULL, path TEXT NOT NULL, content_hash TEXT NOT NULL, created_at TEXT NOT NULL);")?;
        let _ = self.connection.execute(
            "ALTER TABLE migration_history ADD COLUMN package_digest TEXT NOT NULL DEFAULT ''",
            [],
        );
        let _ = self.connection.execute(
            "ALTER TABLE migration_history ADD COLUMN applied_at TEXT NOT NULL DEFAULT ''",
            [],
        );
        let _ = self.connection.execute(
            "ALTER TABLE map_location_projection ADD COLUMN label TEXT",
            [],
        );
        self.rebuild_search()?;
        Ok(())
    }

    pub fn create_entity(&self, input: CreateEntity) -> Result<Entity, CoreError> {
        self.create_entity_with_request(input, None)
    }

    pub fn create_entity_with_request(
        &self,
        input: CreateEntity,
        request_id: Option<&str>,
    ) -> Result<Entity, CoreError> {
        if self.root.is_some() {
            return self.repository_first_mutation(request_id, move |projection, request_id| {
                projection.create_entity_with_request(input, Some(request_id))
            });
        }
        if let Some(mut entity) = self.committed_mutation::<Entity>(request_id)? {
            entity.revision = self.revision_for_entity(&entity.id)?;
            return Ok(entity);
        }
        if input.name.trim().is_empty() {
            return Err(CoreError::NotFound("entity name cannot be empty".into()));
        }
        let entity_type = input.entity_type.map(|value| value.trim().to_owned());
        let id = Uuid::new_v4().to_string();
        let now = chrono_like_now();
        self.connection.execute(
            "INSERT INTO entities(id,name,entity_type,created_at,updated_at) VALUES (?1,?2,?3,?4,?4)",
            params![id, input.name.trim(), entity_type, now],
        )?;
        self.rebuild_search()?;
        let request_id = self.request_id(request_id)?;
        let result = serde_json::to_value(&Entity {
            id: id.clone(),
            name: input.name.trim().into(),
            entity_type: entity_type.clone(),
            deleted: false,
            created_at: now.clone(),
            updated_at: now.clone(),
            revision: String::new(),
        })
        .map_err(|error| CoreError::Serialization(error.to_string()))?;
        self.sync_canonical_with_request_id(&request_id, Some(&result))?;
        let revision = self.revision_for_entity(&id)?;
        Ok(Entity {
            id,
            name: input.name.trim().into(),
            entity_type,
            deleted: false,
            created_at: now.clone(),
            updated_at: now,
            revision,
        })
    }

    pub fn create_entry(&self, input: CreateEntry) -> Result<Entity, CoreError> {
        self.create_entry_with_request(input, None)
    }

    pub fn create_entry_with_request(
        &self,
        input: CreateEntry,
        request_id: Option<&str>,
    ) -> Result<Entity, CoreError> {
        validate_document_format(
            input
                .document
                .as_ref()
                .and_then(|document| document.format.as_deref()),
            self.root.is_some(),
        )?;
        if self.root.is_some() {
            return self.repository_first_mutation(request_id, move |projection, request_id| {
                projection.create_entry_with_request(input, Some(request_id))
            });
        }
        if let Some(mut entity) = self.committed_mutation::<Entity>(request_id)? {
            entity.revision = self.revision_for_entity(&entity.id)?;
            return Ok(entity);
        }
        if input.name.trim().is_empty() {
            return Err(CoreError::NotFound("entity name cannot be empty".into()));
        }
        let format = input
            .document
            .as_ref()
            .and_then(|document| document.format.as_deref())
            .unwrap_or("markdown")
            .to_owned();
        validate_document_format(Some(&format), false)?;
        let encoded_fields = input
            .fields
            .iter()
            .map(|field| {
                if field.namespace.trim().is_empty() || field.key.trim().is_empty() {
                    return Err(CoreError::NotFound(
                        "field namespace and key are required".into(),
                    ));
                }
                Ok((field, encode_field_value(&field.value)?))
            })
            .collect::<Result<Vec<_>, CoreError>>()?;
        let entity_type = input.entity_type.map(|value| value.trim().to_owned());
        let id = Uuid::new_v4().to_string();
        let now = chrono_like_now();
        let mut relationship_rows = Vec::new();
        for relationship in &input.relationships {
            if relationship.relationship_type.trim().is_empty() {
                return Err(CoreError::Validation(
                    "relationship type cannot be empty".into(),
                ));
            }
            let mut target_ids = BTreeSet::new();
            for target_id in &relationship.target_ids {
                if !target_ids.insert(target_id) {
                    return Err(CoreError::Validation(
                        "duplicate relationship target".into(),
                    ));
                }
                let exists: Option<String> = self
                    .connection
                    .query_row(
                        "SELECT id FROM entities WHERE id=?1 AND deleted=0",
                        params![target_id],
                        |row| row.get(0),
                    )
                    .optional()?;
                if exists.is_none() {
                    return Err(CoreError::NotFound("relationship entity not found".into()));
                }
                relationship_rows.push((
                    target_id.clone(),
                    relationship.relationship_type.trim().to_owned(),
                ));
            }
        }
        let transaction = self.connection.unchecked_transaction()?;
        transaction.execute(
            "INSERT INTO entities(id,name,entity_type,created_at,updated_at) VALUES (?1,?2,?3,?4,?4)",
            params![id, input.name.trim(), entity_type, now],
        )?;
        if let Some(document) = input.document {
            transaction.execute(
                "INSERT INTO documents(id,entity_id,format,body,updated_at) VALUES (?1,?2,?3,?4,?5)",
                params![Uuid::new_v4().to_string(), id, format, document.body, now],
            )?;
        }
        for (field, value) in encoded_fields {
            if field.namespace == crate::maps::MAP_NAMESPACE {
                crate::maps::validate_field(
                    &transaction,
                    &id,
                    &field.key,
                    &field.value,
                )?;
            }
            transaction.execute(
                "INSERT INTO entity_fields(entity_id,namespace,key,value) VALUES (?1,?2,?3,?4)",
                params![id, field.namespace, field.key, value],
            )?;
        }
        for (target_id, relationship_type) in relationship_rows {
            transaction.execute(
                "INSERT INTO relationships(id,source_id,target_id,relationship_type,metadata) VALUES (?1,?2,?3,?4,?5)",
                params![Uuid::new_v4().to_string(), id, target_id, relationship_type, "{}"],
            )?;
        }
        transaction.commit()?;
        self.rebuild_search()?;
        let request_id = self.request_id(request_id)?;
        let result = serde_json::to_value(&Entity {
            id: id.clone(),
            name: input.name.trim().into(),
            entity_type: entity_type.clone(),
            deleted: false,
            created_at: now.clone(),
            updated_at: now.clone(),
            revision: String::new(),
        })
        .map_err(|error| CoreError::Serialization(error.to_string()))?;
        self.sync_canonical_with_request_id(&request_id, Some(&result))?;
        let revision = self.revision_for_entity(&id)?;
        Ok(Entity {
            id,
            name: input.name.trim().into(),
            entity_type,
            deleted: false,
            created_at: now.clone(),
            updated_at: now,
            revision,
        })
    }

    pub fn list_entities(&self) -> Result<Vec<Entity>, CoreError> {
        self.ensure_source_index_current()?;
        self.list_entities_where("WHERE deleted=0")
    }

    fn list_all_entities(&self) -> Result<Vec<Entity>, CoreError> {
        self.list_entities_where("")
    }

    fn list_entities_where(&self, predicate: &str) -> Result<Vec<Entity>, CoreError> {
        let mut statement = self.connection.prepare(&format!("SELECT id,name,entity_type,deleted,created_at,updated_at FROM entities {predicate} ORDER BY name"))?;
        let rows = statement.query_map([], |row| {
            Ok(Entity {
                id: row.get(0)?,
                name: row.get(1)?,
                entity_type: row.get(2)?,
                deleted: row.get::<_, i64>(3)? != 0,
                created_at: row.get(4)?,
                updated_at: row.get(5)?,
                revision: String::new(),
            })
        })?;
        let mut entities = rows.collect::<Result<Vec<_>, _>>()?;
        for entity in &mut entities {
            entity.revision = self.revision_for_entity(&entity.id)?;
        }
        Ok(entities)
    }

    pub fn update_entity(
        &self,
        id: String,
        name: Option<String>,
        entity_type: Option<String>,
    ) -> Result<Entity, CoreError> {
        self.update_entity_with_options(id, name, entity_type, None, None)
    }

    pub fn update_entity_with_options(
        &self,
        id: String,
        name: Option<String>,
        entity_type: Option<String>,
        expected_revision: Option<&str>,
        request_id: Option<&str>,
    ) -> Result<Entity, CoreError> {
        if self.root.is_some() {
            return self.repository_first_mutation(request_id, move |projection, request_id| {
                projection.update_entity_with_options(
                    id,
                    name,
                    entity_type,
                    expected_revision,
                    Some(request_id),
                )
            });
        }
        if let Some(mut entity) = self.committed_mutation::<Entity>(request_id)? {
            entity.revision = self.revision_for_entity(&entity.id)?;
            return Ok(entity);
        }
        if self.root.is_some() {
            self.ensure_source_index_current()?;
        }
        Self::ensure_expected_revision(
            expected_revision,
            self.revision_for_entity(&id)?,
            "entity",
        )?;
        if let Some(value) = &name {
            if value.trim().is_empty() {
                return Err(CoreError::NotFound("entity name cannot be empty".into()));
            }
        }
        let now = chrono_like_now();
        if self.connection.execute("UPDATE entities SET name=COALESCE(?2,name), entity_type=COALESCE(?3,entity_type), updated_at=?4 WHERE id=?1 AND deleted=0", params![id, name, entity_type, now])? == 0 { return Err(CoreError::NotFound("entity not found".into())); }
        self.rebuild_search()?;
        let mut entity = self.connection.query_row(
            "SELECT id,name,entity_type,deleted,created_at,updated_at FROM entities WHERE id=?1",
            params![id],
            |row| {
                Ok(Entity {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    entity_type: row.get(2)?,
                    deleted: row.get::<_, i64>(3)? != 0,
                    created_at: row.get(4)?,
                    updated_at: row.get(5)?,
                    revision: String::new(),
                })
            },
        )?;
        let request_id = self.request_id(request_id)?;
        let result = serde_json::to_value(&entity)
            .map_err(|error| CoreError::Serialization(error.to_string()))?;
        self.sync_canonical_with_request_id(&request_id, Some(&result))?;
        entity.revision = self.revision_for_entity(&entity.id)?;
        Ok(entity)
    }

    /// Creates the minimum canonical shape required before the map provider opens.
    /// No source asset exists until the first save; the descriptor carries a null
    /// sourceAssetId and the provider generates its first map on load.
    pub fn create_map(&self, name: String) -> Result<Entity, CoreError> {
        let entity = self.create_entity(CreateEntity {
            name,
            entity_type: Some(crate::maps::MAP_ENTITY_TYPE.into()),
        })?;
        self.set_field(FieldValue {
            entity_id: entity.id.clone(),
            namespace: crate::maps::MAP_NAMESPACE.into(),
            key: "map".into(),
            value: serde_json::json!({
                "schemaVersion": 1,
                "provider": {"id": "azgaar-fmg", "adapterVersion": 1, "sourceFormat": "fmg-map"},
                "sourceAssetId": null,
                "previewAssetId": null,
                "defaultView": {"center": [0.5, 0.5], "zoom": 1}
            }),
            revision: String::new(),
        })?;
        Ok(entity)
    }

    pub fn delete_entity(&self, id: String) -> Result<(), CoreError> {
        self.delete_entity_with_options(id, None, None)
    }

    pub fn delete_entity_with_options(
        &self,
        id: String,
        expected_revision: Option<&str>,
        request_id: Option<&str>,
    ) -> Result<(), CoreError> {
        if self.root.is_some() {
            return self.repository_first_mutation(request_id, move |projection, request_id| {
                projection.delete_entity_with_options(id, expected_revision, Some(request_id))
            });
        }
        if self
            .committed_mutation::<serde_json::Value>(request_id)?
            .is_some()
        {
            return Ok(());
        }
        if self.root.is_some() {
            self.ensure_source_index_current()?;
        }
        Self::ensure_expected_revision(
            expected_revision,
            self.revision_for_entity(&id)?,
            "entity",
        )?;
        if self.connection.execute(
            "UPDATE entities SET deleted=1, updated_at=?2 WHERE id=?1 AND deleted=0",
            params![id, chrono_like_now()],
        )? == 0
        {
            return Err(CoreError::NotFound("entity not found".into()));
        }
        self.rebuild_search()?;
        let request_id = self.request_id(request_id)?;
        self.sync_canonical_with_request_id(&request_id, Some(&serde_json::Value::Null))?;
        Ok(())
    }

    pub fn save_document(&self, input: SaveDocument) -> Result<(), CoreError> {
        self.save_document_with_options(input, None, None)
    }

    pub fn save_document_with_options(
        &self,
        input: SaveDocument,
        expected_revision: Option<&str>,
        request_id: Option<&str>,
    ) -> Result<(), CoreError> {
        self.save_entry_with_options(
            SaveEntry {
                document: input,
                fields: Vec::new(),
            },
            expected_revision,
            request_id,
        )
    }

    pub fn save_entry(&self, input: SaveEntry) -> Result<(), CoreError> {
        self.save_entry_with_options(input, None, None)
    }

    pub fn save_entry_with_options(
        &self,
        input: SaveEntry,
        expected_revision: Option<&str>,
        request_id: Option<&str>,
    ) -> Result<(), CoreError> {
        validate_document_format(input.document.format.as_deref(), self.root.is_some())?;
        if self.root.is_some() {
            return self.repository_first_mutation(request_id, move |projection, request_id| {
                projection.save_entry_with_options(input, expected_revision, Some(request_id))
            });
        }
        if self
            .committed_mutation::<serde_json::Value>(request_id)?
            .is_some()
        {
            return Ok(());
        }
        let document = input.document;
        let format = document.format.unwrap_or_else(|| "markdown".into());
        validate_document_format(Some(&format), false)?;
        let exists: Option<String> = self
            .connection
            .query_row(
                "SELECT id FROM entities WHERE id=?1 AND deleted=0",
                params![document.entity_id],
                |row| row.get(0),
            )
            .optional()?;
        if exists.is_none() {
            return Err(CoreError::NotFound("entity not found".into()));
        }
        let current_document_id: Option<String> = self
            .connection
            .query_row(
                "SELECT id FROM documents WHERE entity_id=?1 ORDER BY updated_at DESC LIMIT 1",
                params![document.entity_id],
                |row| row.get(0),
            )
            .optional()?;
        if let Some(expected_revision) = expected_revision {
            let Some(document_id) = current_document_id.as_deref() else {
                return Err(CoreError::Conflict(
                    "document revision conflict: document does not exist".into(),
                ));
            };
            Self::ensure_expected_revision(
                Some(expected_revision),
                self.revision_for_document(document_id)?,
                "document",
            )?;
        }
        let encoded_fields = input
            .fields
            .iter()
            .map(|field| {
                if field.entity_id != document.entity_id {
                    return Err(CoreError::NotFound(
                        "field entity does not match document entity".into(),
                    ));
                }
                if field.namespace.trim().is_empty() || field.key.trim().is_empty() {
                    return Err(CoreError::NotFound(
                        "field namespace and key are required".into(),
                    ));
                }
                Ok((field, encode_field_value(&field.value)?))
            })
            .collect::<Result<Vec<_>, CoreError>>()?;
        for (field, _) in &encoded_fields {
            if field.namespace == crate::maps::MAP_NAMESPACE {
                crate::maps::validate_field(
                    &self.connection,
                    &field.entity_id,
                    &field.key,
                    &field.value,
                )?;
            }
        }
        for (field, _) in &encoded_fields {
            if field.revision.is_empty() {
                continue;
            }
            let current: Option<String> = self
                .connection
                .query_row(
                    "SELECT value FROM entity_fields WHERE entity_id=?1 AND namespace=?2 AND key=?3",
                    params![field.entity_id, field.namespace, field.key],
                    |row| row.get(0),
                )
                .optional()?;
            let Some(current) = current else {
                return Err(CoreError::Conflict(
                    "field revision conflict: field does not exist".into(),
                ));
            };
            let current_field = FieldValue {
                entity_id: field.entity_id.clone(),
                namespace: field.namespace.clone(),
                key: field.key.clone(),
                value: decode_field_value(current),
                revision: String::new(),
            };
            Self::ensure_expected_revision(
                Some(&field.revision),
                self.revision_for_field(&current_field)?,
                "field",
            )?;
        }
        let now = chrono_like_now();
        let document_id: Option<String> = self
            .connection
            .query_row(
                "SELECT id FROM documents WHERE entity_id=?1 ORDER BY updated_at DESC LIMIT 1",
                params![document.entity_id],
                |row| row.get(0),
            )
            .optional()?;
        let transaction = self.connection.unchecked_transaction()?;
        if let Some(document_id) = document_id {
            transaction.execute(
                "UPDATE documents SET format=?2, body=?3, updated_at=?4 WHERE id=?1",
                params![document_id, format, document.body, now],
            )?;
        } else {
            transaction.execute(
                "INSERT INTO documents(id,entity_id,format,body,updated_at) VALUES (?1,?2,?3,?4,?5)",
                params![Uuid::new_v4().to_string(), document.entity_id, format, document.body, now],
            )?;
        }
        for (field, value) in encoded_fields {
            transaction.execute("INSERT INTO entity_fields(entity_id,namespace,key,value) VALUES (?1,?2,?3,?4) ON CONFLICT(entity_id,namespace,key) DO UPDATE SET value=excluded.value", params![field.entity_id, field.namespace, field.key, value])?;
        }
        transaction.commit()?;
        self.rebuild_search()?;
        let request_id = self.request_id(request_id)?;
        self.sync_canonical_with_request_id(&request_id, Some(&serde_json::Value::Null))?;
        Ok(())
    }

    pub fn list_documents(&self, entity_id: String) -> Result<Vec<Document>, CoreError> {
        self.ensure_source_index_current()?;
        self.list_documents_unchecked(entity_id)
    }

    fn list_documents_unchecked(&self, entity_id: String) -> Result<Vec<Document>, CoreError> {
        let mut statement = self.connection.prepare("SELECT id,entity_id,format,body,updated_at FROM documents WHERE entity_id=?1 ORDER BY updated_at DESC")?;
        let rows = statement.query_map(params![entity_id], |row| {
            Ok(Document {
                id: row.get(0)?,
                entity_id: row.get(1)?,
                format: row.get(2)?,
                body: row.get(3)?,
                updated_at: row.get(4)?,
                revision: String::new(),
            })
        })?;
        let mut documents = rows.collect::<Result<Vec<_>, _>>()?;
        for document in &mut documents {
            document.revision = self.revision_for_document(&document.id)?;
        }
        Ok(documents)
    }
    pub fn set_field(&self, field: FieldValue) -> Result<(), CoreError> {
        self.set_field_with_request(field, None)
    }

    pub fn set_field_with_request(
        &self,
        field: FieldValue,
        request_id: Option<&str>,
    ) -> Result<(), CoreError> {
        if self.root.is_some() {
            return self.repository_first_mutation(request_id, move |projection, request_id| {
                projection.set_field_with_request(field, Some(request_id))
            });
        }
        if self
            .committed_mutation::<serde_json::Value>(request_id)?
            .is_some()
        {
            return Ok(());
        }
        let exists: Option<String> = self
            .connection
            .query_row(
                "SELECT id FROM entities WHERE id=?1 AND deleted=0",
                params![field.entity_id],
                |row| row.get(0),
            )
            .optional()?;
        if exists.is_none() {
            return Err(CoreError::NotFound("entity not found".into()));
        }
        if field.namespace.trim().is_empty() || field.key.trim().is_empty() {
            return Err(CoreError::NotFound(
                "field namespace and key are required".into(),
            ));
        }
        if field.namespace == crate::maps::MAP_NAMESPACE {
            crate::maps::validate_field(
                &self.connection,
                &field.entity_id,
                &field.key,
                &field.value,
            )?;
        }
        if !field.revision.is_empty() {
            let current: Option<String> = self
                .connection
                .query_row(
                    "SELECT value FROM entity_fields WHERE entity_id=?1 AND namespace=?2 AND key=?3",
                    params![field.entity_id, field.namespace, field.key],
                    |row| row.get(0),
                )
                .optional()?;
            let Some(current) = current else {
                return Err(CoreError::Conflict(
                    "field revision conflict: field does not exist".into(),
                ));
            };
            let current_field = FieldValue {
                entity_id: field.entity_id.clone(),
                namespace: field.namespace.clone(),
                key: field.key.clone(),
                value: decode_field_value(current),
                revision: String::new(),
            };
            Self::ensure_expected_revision(
                Some(&field.revision),
                self.revision_for_field(&current_field)?,
                "field",
            )?;
        }
        let value = encode_field_value(&field.value)?;
        self.connection.execute("INSERT INTO entity_fields(entity_id,namespace,key,value) VALUES (?1,?2,?3,?4) ON CONFLICT(entity_id,namespace,key) DO UPDATE SET value=excluded.value", params![field.entity_id, field.namespace, field.key, value])?;
        self.rebuild_search()?;
        let request_id = self.request_id(request_id)?;
        self.sync_canonical_with_request_id(&request_id, Some(&serde_json::Value::Null))?;
        Ok(())
    }
    pub fn list_fields(&self, entity_id: String) -> Result<Vec<FieldValue>, CoreError> {
        self.ensure_source_index_current()?;
        self.list_fields_unchecked(entity_id)
    }

    fn list_fields_unchecked(&self, entity_id: String) -> Result<Vec<FieldValue>, CoreError> {
        let mut s = self.connection.prepare(
            "SELECT entity_id,namespace,key,value FROM entity_fields WHERE entity_id=?1",
        )?;
        let rows = s.query_map(params![entity_id], |r| {
            let value: String = r.get(3)?;
            Ok(FieldValue {
                entity_id: r.get(0)?,
                namespace: r.get(1)?,
                key: r.get(2)?,
                value: decode_field_value(value),
                revision: String::new(),
            })
        })?;
        let mut fields = rows.collect::<Result<Vec<_>, _>>()?;
        for field in &mut fields {
            field.revision = self.revision_for_field(field)?;
        }
        Ok(fields)
    }

    pub fn create_relationship(&self, input: RelationshipInput) -> Result<Relationship, CoreError> {
        self.create_relationship_with_options(input, None, None)
    }

    pub fn create_relationship_with_request(
        &self,
        input: RelationshipInput,
        request_id: Option<&str>,
    ) -> Result<Relationship, CoreError> {
        self.create_relationship_with_options(input, None, request_id)
    }

    pub fn create_relationship_with_options(
        &self,
        input: RelationshipInput,
        expected_revision: Option<&str>,
        request_id: Option<&str>,
    ) -> Result<Relationship, CoreError> {
        if self.root.is_some() {
            return self.repository_first_mutation(request_id, move |projection, request_id| {
                projection.create_relationship_with_options(
                    input,
                    expected_revision,
                    Some(request_id),
                )
            });
        }
        if let Some(relationship) = self.committed_mutation::<Relationship>(request_id)? {
            let mut relationship = relationship;
            relationship.revision = self.revision_for_relationship(&relationship.id)?;
            return Ok(relationship);
        }
        for entity_id in [&input.source_id, &input.target_id] {
            let exists: Option<String> = self
                .connection
                .query_row(
                    "SELECT id FROM entities WHERE id=?1 AND deleted=0",
                    params![entity_id],
                    |row| row.get(0),
                )
                .optional()?;
            if exists.is_none() {
                return Err(CoreError::NotFound("relationship entity not found".into()));
            }
        }
        if input.relationship_type.trim().is_empty() {
            return Err(CoreError::NotFound(
                "relationship type cannot be empty".into(),
            ));
        }
        Self::ensure_expected_revision(
            expected_revision,
            self.revision_for_entity(&input.source_id)?,
            "relationship source entity",
        )?;
        let id = Uuid::new_v4().to_string();
        let metadata = input.metadata.unwrap_or_else(|| "{}".into());
        self.connection.execute("INSERT INTO relationships(id,source_id,target_id,relationship_type,metadata) VALUES (?1,?2,?3,?4,?5)", params![id, input.source_id, input.target_id, input.relationship_type, metadata])?;
        let request_id = self.request_id(request_id)?;
        let result = serde_json::to_value(&Relationship {
            id: id.clone(),
            source_id: input.source_id.clone(),
            target_id: input.target_id.clone(),
            relationship_type: input.relationship_type.clone(),
            metadata: metadata.clone(),
            revision: String::new(),
        })
        .map_err(|error| CoreError::Serialization(error.to_string()))?;
        self.sync_canonical_with_request_id(&request_id, Some(&result))?;
        let revision = self.revision_for_relationship(&id)?;
        Ok(Relationship {
            id,
            source_id: input.source_id,
            target_id: input.target_id,
            relationship_type: input.relationship_type,
            metadata,
            revision,
        })
    }

    pub fn delete_relationship(&self, id: String) -> Result<(), CoreError> {
        self.delete_relationship_with_options(id, None, None)
    }

    pub fn delete_relationship_with_options(
        &self,
        id: String,
        expected_revision: Option<&str>,
        request_id: Option<&str>,
    ) -> Result<(), CoreError> {
        if self.root.is_some() {
            return self.repository_first_mutation(request_id, move |projection, request_id| {
                projection.delete_relationship_with_options(id, expected_revision, Some(request_id))
            });
        }
        if self
            .committed_mutation::<serde_json::Value>(request_id)?
            .is_some()
        {
            return Ok(());
        }
        Self::ensure_expected_revision(
            expected_revision,
            self.revision_for_relationship(&id)?,
            "relationship",
        )?;
        if self
            .connection
            .execute("DELETE FROM relationships WHERE id=?1", params![id])?
            == 0
        {
            return Err(CoreError::NotFound("relationship not found".into()));
        }
        let request_id = self.request_id(request_id)?;
        self.sync_canonical_with_request_id(&request_id, Some(&serde_json::Value::Null))?;
        Ok(())
    }

    pub fn relationship(&self, id: String) -> Result<Relationship, CoreError> {
        self.ensure_source_index_current()?;
        self.connection
            .query_row(
                "SELECT id,source_id,target_id,relationship_type,metadata FROM relationships WHERE id=?1",
                params![id],
                |row| {
                    Ok(Relationship {
                        id: row.get(0)?,
                        source_id: row.get(1)?,
                        target_id: row.get(2)?,
                        relationship_type: row.get(3)?,
                        metadata: row.get(4)?,
                        revision: String::new(),
                    })
                },
            )
            .optional()?
            .ok_or_else(|| CoreError::NotFound("relationship not found".into()))
            .and_then(|mut relationship| {
                relationship.revision = self.revision_for_relationship(&relationship.id)?;
                Ok(relationship)
            })
    }

    pub fn list_relationships(&self, entity_id: String) -> Result<Vec<Relationship>, CoreError> {
        self.ensure_source_index_current()?;
        self.list_relationships_unchecked(entity_id)
    }

    fn list_relationships_unchecked(
        &self,
        entity_id: String,
    ) -> Result<Vec<Relationship>, CoreError> {
        let mut statement = self.connection.prepare("SELECT id,source_id,target_id,relationship_type,metadata FROM relationships WHERE source_id=?1 OR target_id=?1")?;
        let rows = statement.query_map(params![entity_id], |row| {
            Ok(Relationship {
                id: row.get(0)?,
                source_id: row.get(1)?,
                target_id: row.get(2)?,
                relationship_type: row.get(3)?,
                metadata: row.get(4)?,
                revision: String::new(),
            })
        })?;
        let mut relationships = rows.collect::<Result<Vec<_>, _>>()?;
        for relationship in &mut relationships {
            relationship.revision = self.revision_for_relationship(&relationship.id)?;
        }
        Ok(relationships)
    }

    pub fn search(&self, query: String) -> Result<Vec<Entity>, CoreError> {
        self.ensure_source_index_current()?;
        if query.trim().is_empty() {
            return self.list_entities();
        }
        let terms = query
            .split_whitespace()
            .map(|term| format!("\"{}\"*", term.replace('"', "")))
            .collect::<Vec<_>>()
            .join(" AND ");
        let mut statement = self.connection.prepare("SELECT e.id,e.name,e.entity_type,e.deleted,e.created_at,e.updated_at FROM entities e JOIN (SELECT entity_id, MIN(rowid) AS rank FROM world_search WHERE world_search MATCH ?1 GROUP BY entity_id) s ON s.entity_id=e.id WHERE e.deleted=0 ORDER BY s.rank LIMIT 100")?;
        let rows = statement.query_map(params![terms], |row| {
            Ok(Entity {
                id: row.get(0)?,
                name: row.get(1)?,
                entity_type: row.get(2)?,
                deleted: row.get::<_, i64>(3)? != 0,
                created_at: row.get(4)?,
                updated_at: row.get(5)?,
                revision: String::new(),
            })
        })?;
        let mut entities = rows.collect::<Result<Vec<_>, _>>()?;
        for entity in &mut entities {
            entity.revision = self.revision_for_entity(&entity.id)?;
        }
        Ok(entities)
    }

    pub fn export_json(&self) -> Result<String, CoreError> {
        self.ensure_source_index_current()?;
        self.export_json_inner()
    }

    fn export_json_inner(&self) -> Result<String, CoreError> {
        let entities = self.list_all_entities()?;
        let mut documents = Vec::new();
        let mut fields = Vec::new();
        let mut assets = Vec::new();
        let mut relationships = Vec::new();
        for entity in &entities {
            documents.extend(self.list_documents_unchecked(entity.id.clone())?);
            fields.extend(self.list_fields_unchecked(entity.id.clone())?);
            assets.extend(self.list_assets_unchecked(entity.id.clone())?);
            relationships.extend(self.list_relationships_unchecked(entity.id.clone())?);
        }
        relationships.sort_by(|a, b| a.id.cmp(&b.id));
        relationships.dedup_by(|a, b| a.id == b.id);
        let mut module_statement = self.connection.prepare("SELECT m.module_id, COALESCE(s.enabled, 1), m.version, p.package_version FROM module_versions m LEFT JOIN module_state s ON s.module_id = m.module_id LEFT JOIN module_package_versions p ON p.module_id = m.module_id ORDER BY m.module_id")?;
        let modules = module_statement
            .query_map([], |row| {
                Ok(ModuleState {
                    module_id: row.get(0)?,
                    enabled: row.get::<_, i64>(1)? != 0,
                    version: row.get(2)?,
                    package_version: row.get(3)?,
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
        serde_json::to_string_pretty(&ProjectSnapshot {
            format_version: current_snapshot_version(),
            entities,
            documents,
            fields,
            relationships,
            assets,
            modules,
            module_namespaces,
            module_fields,
            migration_history,
        })
        .map_err(|error| CoreError::NotFound(error.to_string()))
    }

    #[allow(dead_code)]
    fn import_json_with_mode(&self, payload: &str, replace: bool) -> Result<usize, CoreError> {
        self.import_json_with_mode_and_sync(payload, replace, true)
    }

    fn import_json_with_mode_and_sync(
        &self,
        payload: &str,
        replace: bool,
        sync_canonical: bool,
    ) -> Result<usize, CoreError> {
        self.import_json_with_mode_and_sync_with_request(payload, replace, sync_canonical, None)
    }

    fn import_json_with_mode_and_sync_with_request(
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

    fn import_json_with_mode_and_sync_with_request_and_search(
        &self,
        payload: &str,
        replace: bool,
        sync_canonical: bool,
        request_id: Option<&str>,
        rebuild_search: bool,
    ) -> Result<usize, CoreError> {
        let snapshot: ProjectSnapshot = serde_json::from_str(payload)
            .map_err(|error| CoreError::NotFound(error.to_string()))?;
        if snapshot.format_version != current_snapshot_version() {
            return Err(CoreError::NotFound(format!(
                "unsupported project snapshot version {}",
                snapshot.format_version
            )));
        }
        let transaction = self.connection.unchecked_transaction()?;
        if replace {
            transaction.execute_batch(
                "DELETE FROM assets;
                 DELETE FROM entity_fields;
                 DELETE FROM documents;
                 DELETE FROM relationships;
                 DELETE FROM entities;
                 DELETE FROM module_state;
                 DELETE FROM module_versions;
                 DELETE FROM module_package_versions;
                 DELETE FROM module_fields;
                 DELETE FROM module_namespaces;
                 DELETE FROM migration_history;",
            )?;
        }
        for entity in &snapshot.entities {
            transaction.execute("INSERT INTO entities(id,name,entity_type,deleted,created_at,updated_at) VALUES (?1,?2,?3,?4,?5,?6) ON CONFLICT(id) DO UPDATE SET name=excluded.name,entity_type=excluded.entity_type,deleted=excluded.deleted,created_at=excluded.created_at,updated_at=excluded.updated_at", params![entity.id, entity.name, entity.entity_type, entity.deleted as i64, entity.created_at, entity.updated_at])?;
        }
        for document in &snapshot.documents {
            transaction.execute("INSERT INTO documents(id,entity_id,format,body,updated_at) VALUES (?1,?2,?3,?4,?5) ON CONFLICT(id) DO UPDATE SET entity_id=excluded.entity_id,format=excluded.format,body=excluded.body,updated_at=excluded.updated_at", params![document.id, document.entity_id, document.format, document.body, document.updated_at])?;
        }
        for field in &snapshot.fields {
            let value = encode_field_value(&field.value)?;
            transaction.execute("INSERT INTO entity_fields(entity_id,namespace,key,value) VALUES (?1,?2,?3,?4) ON CONFLICT(entity_id,namespace,key) DO UPDATE SET value=excluded.value", params![field.entity_id, field.namespace, field.key, value])?;
        }
        for relationship in &snapshot.relationships {
            transaction.execute("INSERT INTO relationships(id,source_id,target_id,relationship_type,metadata) VALUES (?1,?2,?3,?4,?5) ON CONFLICT(id) DO UPDATE SET source_id=excluded.source_id,target_id=excluded.target_id,relationship_type=excluded.relationship_type,metadata=excluded.metadata", params![relationship.id, relationship.source_id, relationship.target_id, relationship.relationship_type, relationship.metadata])?;
        }
        for asset in &snapshot.assets {
            transaction.execute("INSERT INTO assets(id,entity_id,namespace,filename,content_hash,size,mime_type,path,created_at) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9) ON CONFLICT(id) DO UPDATE SET entity_id=excluded.entity_id,namespace=excluded.namespace,filename=excluded.filename,content_hash=excluded.content_hash,size=excluded.size,mime_type=excluded.mime_type,path=excluded.path,created_at=excluded.created_at", params![asset.id, asset.entity_id, asset.namespace, asset.filename, asset.content_hash, asset.size, asset.mime_type, asset.path, asset.created_at])?;
        }
        for module in &snapshot.modules {
            transaction.execute("INSERT INTO module_versions(module_id,version) VALUES (?1,?2) ON CONFLICT(module_id) DO UPDATE SET version=excluded.version", params![module.module_id, module.version])?;
            transaction.execute("INSERT INTO module_state(module_id,enabled) VALUES (?1,?2) ON CONFLICT(module_id) DO UPDATE SET enabled=excluded.enabled", params![module.module_id, module.enabled as i64])?;
            if let Some(package_version) = &module.package_version {
                transaction.execute("INSERT INTO module_package_versions(module_id,package_version) VALUES (?1,?2) ON CONFLICT(module_id) DO UPDATE SET package_version=excluded.package_version", params![module.module_id, package_version])?;
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
                params![field.module_id, field.namespace, field.key, field.field_type, field.required as i64],
            )?;
        }
        for migration in &snapshot.migration_history {
            transaction.execute(
                "INSERT INTO migration_history(module_id,migration_id,from_version,to_version,checksum,package_digest,applied_at) VALUES (?1,?2,?3,?4,?5,?6,?7)",
                params![migration.module_id, migration.migration_id, migration.from_version, migration.to_version, migration.checksum, migration.package_digest, migration.applied_at],
            )?;
        }
        transaction.commit()?;
        if rebuild_search {
            self.rebuild_search()?;
        }
        if sync_canonical {
            let request_id = self.request_id(request_id)?;
            self.sync_canonical_with_request_id(&request_id, Some(&serde_json::Value::Null))?;
        }
        Ok(snapshot.entities.len())
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
        if self.root.is_some() {
            return self.repository_first_mutation(request_id, move |projection, request_id| {
                projection.register_asset_with_options(input, expected_revision, Some(request_id))
            });
        }
        if let Some(mut asset) = self.committed_mutation::<Asset>(request_id)? {
            asset.revision = self.revision_for_asset(&asset.id)?;
            return Ok(asset);
        }
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
        let id = Uuid::new_v4().to_string();
        let now = chrono_like_now();
        self.connection.execute(
            "INSERT INTO assets(id,entity_id,namespace,filename,content_hash,size,mime_type,path,created_at) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9)",
            params![id, input.entity_id, input.namespace, input.filename, input.content_hash, input.size, input.mime_type, input.path, now],
        )?;
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
            revision: String::new(),
        })
        .map_err(|error| CoreError::Serialization(error.to_string()))?;
        self.sync_canonical_with_request_id(&request_id, Some(&result))?;
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
        if self.root.is_some() {
            return self.repository_first_mutation(request_id, move |projection, request_id| {
                projection.register_asset_file_with_options(
                    input,
                    expected_revision,
                    Some(request_id),
                )
            });
        }
        let source = Path::new(&input.source_path);
        let metadata =
            std::fs::metadata(source).map_err(|error| CoreError::NotFound(error.to_string()))?;
        if !metadata.is_file() {
            return Err(CoreError::NotFound("asset source is not a file".into()));
        }
        let filename = Path::new(&input.filename)
            .file_name()
            .and_then(|value| value.to_str())
            .filter(|value| !value.is_empty() && *value != "." && *value != "..")
            .ok_or_else(|| CoreError::NotFound("asset filename is invalid".into()))?;
        let category = if input.mime_type.starts_with("image/") {
            "images"
        } else if input.mime_type.starts_with("video/") {
            "videos"
        } else if input.mime_type.contains("map")
            || matches!(
                Path::new(filename)
                    .extension()
                    .and_then(|value| value.to_str()),
                Some("geojson" | "tmx" | "mbtiles")
            )
        {
            "maps"
        } else {
            "files"
        };
        let bytes =
            std::fs::read(source).map_err(|error| CoreError::NotFound(error.to_string()))?;
        let relative_path = format!("assets/{category}/{}-{}", Uuid::new_v4(), filename);
        self.pending_asset_imports
            .lock()
            .map_err(|_| CoreError::Conflict("asset import state is poisoned".into()))?
            .insert(relative_path.clone(), bytes.clone());
        let result = self.register_asset_with_options(
            AssetInput {
                entity_id: input.entity_id,
                namespace: input.namespace,
                filename: filename.into(),
                content_hash: format!("sha256:{:x}", Sha256::digest(&bytes)),
                size: bytes.len() as i64,
                mime_type: input.mime_type,
                path: relative_path,
            },
            expected_revision,
            request_id,
        );
        if result.is_err() {
            if let Ok(mut pending) = self.pending_asset_imports.lock() {
                pending.retain(|_, value| value != &bytes);
            }
        }
        result
    }

    pub fn list_assets(&self, entity_id: String) -> Result<Vec<Asset>, CoreError> {
        self.ensure_source_index_current()?;
        self.list_assets_unchecked(entity_id)
    }

    pub fn asset(&self, asset_id: String) -> Result<Asset, CoreError> {
        self.ensure_source_index_current()?;
        self.asset_unchecked(&asset_id)
    }

    fn asset_unchecked(&self, asset_id: &str) -> Result<Asset, CoreError> {
        let mut asset = self
            .connection
            .query_row(
                "SELECT id,entity_id,namespace,filename,content_hash,size,mime_type,path,created_at FROM assets WHERE id=?1",
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
        if self.root.is_some() {
            return self.repository_first_mutation(request_id, move |projection, request_id| {
                projection.replace_asset_bytes_with_request(
                    input,
                    bytes,
                    expected_revision,
                    Some(request_id),
                )
            });
        }
        if let Some(mut asset) = self.committed_mutation::<Asset>(request_id)? {
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
        let mut asset = self.asset_unchecked(&input.asset_id)?;
        Self::ensure_expected_revision(Some(expected_revision), asset.revision.clone(), "asset")?;
        if input.mime_type.trim().is_empty() {
            return Err(CoreError::Validation(
                "asset replacement MIME type is required".into(),
            ));
        }
        self.pending_asset_imports
            .lock()
            .map_err(|_| CoreError::Conflict("asset import state is poisoned".into()))?
            .insert(asset.path.clone(), bytes);
        self.connection.execute(
            "UPDATE assets SET content_hash=?1,size=?2,mime_type=?3 WHERE id=?4",
            params![
                input.content_hash,
                input.size,
                input.mime_type,
                input.asset_id
            ],
        )?;
        let request_id = self.request_id(request_id)?;
        self.sync_canonical_with_request_id(&request_id, None)?;
        asset.content_hash = input.content_hash;
        asset.size = input.size;
        asset.mime_type = input.mime_type;
        asset.revision = self.revision_for_asset(&asset.id)?;
        Ok(asset)
    }

    fn list_assets_unchecked(&self, entity_id: String) -> Result<Vec<Asset>, CoreError> {
        let mut statement = self.connection.prepare("SELECT id,entity_id,namespace,filename,content_hash,size,mime_type,path,created_at FROM assets WHERE entity_id=?1 ORDER BY created_at")?;
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
                revision: String::new(),
            })
        })?;
        let mut assets = rows.collect::<Result<Vec<_>, _>>()?;
        for asset in &mut assets {
            asset.revision = self.revision_for_asset(&asset.id)?;
        }
        Ok(assets)
    }

    pub fn backup(&self) -> Result<String, CoreError> {
        self.backup_to(std::env::temp_dir())
    }

    pub fn backup_to(&self, dir: impl AsRef<Path>) -> Result<String, CoreError> {
        let export = self.export_json()?;
        let dir = dir.as_ref();
        std::fs::create_dir_all(dir).map_err(|e| CoreError::NotFound(e.to_string()))?;
        let timestamp = chrono_like_now();
        let filename = format!("daena-backup-{}-{}.json", timestamp, Uuid::new_v4());
        let path = dir.join(&filename);
        std::fs::write(&path, export).map_err(|e| CoreError::NotFound(e.to_string()))?;
        Ok(path.to_string_lossy().to_string())
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
            return self.create_plugin_backup_repository_first(
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
        let mut transaction = match crate::transactions::FileTransaction::begin(&root, &request_id)?
        {
            crate::transactions::TransactionStart::Ready(transaction) => transaction,
            crate::transactions::TransactionStart::AlreadyCommitted => {
                return self
                    .committed_mutation::<PluginBackup>(Some(&request_id))?
                    .ok_or_else(|| {
                        CoreError::Conflict("backup request was already committed".into())
                    });
            }
        };
        transaction.stage_bytes(&relative_path, payload.as_bytes())?;
        let result = serde_json::to_value(&backup)
            .map_err(|error| CoreError::Serialization(error.to_string()))?;
        transaction.commit_with_result(&result)?;
        let result = self.connection.execute(
            "INSERT INTO plugin_backups(id,module_id,from_package_version,to_package_version,data_version,path,content_hash,created_at) VALUES (?1,?2,?3,?4,?5,?6,?7,?8)",
            params![backup.id, backup.module_id, backup.from_package_version, backup.to_package_version, backup.data_version, backup.path, backup.content_hash, backup.created_at],
        );
        if let Err(error) = result {
            return Err(error.into());
        }
        Ok(backup)
    }

    fn create_plugin_backup_repository_first(
        &self,
        module_id: &str,
        from_package_version: Option<&str>,
        to_package_version: Option<&str>,
        data_version: i64,
        request_id: Option<&str>,
    ) -> Result<PluginBackup, CoreError> {
        if let Some(backup) = self.committed_mutation::<PluginBackup>(request_id)? {
            self.insert_plugin_backup_index(&backup)?;
            return Ok(backup);
        }
        if module_id.trim().is_empty() {
            return Err(CoreError::Validation(
                "plugin backup requires a module ID".into(),
            ));
        }
        let root = self.project_root()?.to_path_buf();
        let canonical = crate::storage::FilesystemRepository::open(&root)?.scan()?;
        let payload = serde_json::to_vec_pretty(&canonical.snapshot)
            .map_err(|error| CoreError::Serialization(error.to_string()))?;
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
        let mut additional_files = BTreeMap::new();
        additional_files.insert(relative_path, payload);
        self.commit_canonical_snapshot(
            &root,
            &canonical.snapshot,
            &request_id,
            Some(
                &serde_json::to_value(&backup)
                    .map_err(|error| CoreError::Serialization(error.to_string()))?,
            ),
            &BTreeMap::new(),
            &additional_files,
        )?;
        self.insert_plugin_backup_index(&backup)?;
        Ok(backup)
    }

    fn insert_plugin_backup_index(&self, backup: &PluginBackup) -> Result<(), CoreError> {
        self.connection.execute(
            "INSERT OR IGNORE INTO plugin_backups(id,module_id,from_package_version,to_package_version,data_version,path,content_hash,created_at) VALUES (?1,?2,?3,?4,?5,?6,?7,?8)",
            params![backup.id, backup.module_id, backup.from_package_version, backup.to_package_version, backup.data_version, backup.path, backup.content_hash, backup.created_at],
        )?;
        self.rebuild_maps_projection()?;
        Ok(())
    }

    fn provider_feature_resolution(&self, map_entity_id: &str, feature_kind: Option<&str>, feature_id: Option<&str>) -> &'static str {
        let (Some(feature_kind), Some(feature_id)) = (feature_kind, feature_id) else { return "resolved"; };
        let source_path: Option<String> = self.connection.query_row(
            "SELECT a.path FROM entity_fields f JOIN assets a ON a.id=json_extract(f.value, '$.sourceAssetId') WHERE f.entity_id=?1 AND f.namespace=?2 AND f.key='map'",
            rusqlite::params![map_entity_id, crate::maps::MAP_NAMESPACE], |row| row.get(0)).optional().ok().flatten();
        let Some(source_path) = source_path else { return "unresolved"; };
        let source_path = self
            .root
            .as_ref()
            .map(|root| root.join(&source_path))
            .unwrap_or_else(|| PathBuf::from(source_path));
        let Ok(bytes) = std::fs::read(source_path) else { return "unresolved"; };
        let Ok(value) = serde_json::from_slice::<serde_json::Value>(&bytes) else { return "resolved"; };
        let Some(features) = value.get("features").and_then(serde_json::Value::as_array) else { return "unresolved"; };
        if features.iter().any(|feature| feature.get("kind").and_then(serde_json::Value::as_str) == Some(feature_kind) && feature.get("id").and_then(serde_json::Value::as_str) == Some(feature_id)) { "resolved" } else { "unresolved" }
    }

    fn rebuild_maps_projection(&self) -> Result<(), CoreError> {
        self.connection
            .execute_batch("DELETE FROM map_projection; DELETE FROM map_location_projection;")?;
        let mut maps = self.connection.prepare("SELECT e.id, json_extract(f.value, '$.provider.id'), json_extract(f.value, '$.sourceAssetId'), a.path, a.content_hash FROM entities e JOIN entity_fields f ON f.entity_id=e.id AND f.namespace=?1 AND f.key='map' LEFT JOIN assets a ON a.id=json_extract(f.value, '$.sourceAssetId') WHERE e.entity_type='daena.maps:map' AND e.deleted=0")?;
        let rows = maps.query_map([crate::maps::MAP_NAMESPACE], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, Option<String>>(1)?.unwrap_or_default(),
                row.get::<_, Option<String>>(2)?.unwrap_or_default(),
                row.get::<_, Option<String>>(3)?,
                row.get::<_, Option<String>>(4)?,
            ))
        })?;
        for row in rows {
            let (id, provider, asset, path, hash) = row?;
            self.connection.execute("INSERT INTO map_projection(map_entity_id,provider,source_asset_id,source_path,source_hash) VALUES (?1,?2,?3,?4,?5)", rusqlite::params![id, provider, asset, path, hash])?;
        }
        let mut locations = self.connection.prepare("SELECT f.entity_id, json_each.value FROM entity_fields f, json_each(json_extract(f.value, '$.locations')) WHERE f.namespace=?1 AND f.key='locations'")?;
        let rows = locations.query_map([crate::maps::MAP_NAMESPACE], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?;
        for row in rows {
            let (entity_id, raw) = row?;
            let location: serde_json::Value =
                serde_json::from_str(&raw).map_err(|e| CoreError::Serialization(e.to_string()))?;
            let anchor = location.get("anchor").cloned().unwrap_or_default();
            let kind = anchor
                .get("kind")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("unknown");
            let bounds = match kind {
                "point" => {
                    bounds_for_points(anchor.get("point").and_then(serde_json::Value::as_array))
                }
                "provider-feature" => bounds_for_points(
                    anchor
                        .get("fallbackPoint")
                        .and_then(serde_json::Value::as_array),
                ),
                "path" => {
                    bounds_for_points(anchor.get("points").and_then(serde_json::Value::as_array))
                }
                "area" => anchor
                    .get("rings")
                    .and_then(serde_json::Value::as_array)
                    .map(|rings| {
                        bounds_for_points(Some(
                            &rings
                                .iter()
                                .filter_map(serde_json::Value::as_array)
                                .flatten()
                                .cloned()
                                .collect::<Vec<_>>(),
                        ))
                    })
                    .unwrap_or((None, None, None, None)),
                _ => (None, None, None, None),
            };
            let resolution = if kind == "provider-feature" { self.provider_feature_resolution(location.get("mapEntityId").and_then(serde_json::Value::as_str).unwrap_or_default(), anchor.get("featureKind").and_then(serde_json::Value::as_str), anchor.get("featureId").and_then(serde_json::Value::as_str)) } else { "resolved" };
            self.connection.execute("INSERT INTO map_location_projection(location_id,entity_id,map_entity_id,label,role,anchor_kind,provider,feature_kind,feature_id,min_x,min_y,max_x,max_y,valid_from,valid_to,resolution) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16)", rusqlite::params![location.get("id").and_then(serde_json::Value::as_str).unwrap_or_default(), entity_id, location.get("mapEntityId").and_then(serde_json::Value::as_str).unwrap_or_default(), location.get("label").and_then(serde_json::Value::as_str), location.get("role").and_then(serde_json::Value::as_str).unwrap_or_default(), kind, anchor.get("provider").and_then(serde_json::Value::as_str), anchor.get("featureKind").and_then(serde_json::Value::as_str), anchor.get("featureId").and_then(serde_json::Value::as_str), bounds.0, bounds.1, bounds.2, bounds.3, location.pointer("/validity/from").filter(|v| !v.is_null()).map(ToString::to_string), location.pointer("/validity/to").filter(|v| !v.is_null()).map(ToString::to_string), resolution])?;
        }
        Ok(())
    }

    pub fn map_locations_for_entity(
        &self,
        entity_id: String,
    ) -> Result<Vec<serde_json::Value>, CoreError> {
        self.ensure_source_index_current()?;
        let mut statement = self.connection.prepare("SELECT location_id,map_entity_id,label,role,anchor_kind,provider,feature_kind,feature_id,min_x,min_y,max_x,max_y,valid_from,valid_to,resolution FROM map_location_projection WHERE entity_id=?1 ORDER BY location_id")?;
        let rows = statement.query_map(rusqlite::params![entity_id], |row| Ok(serde_json::json!({"id":row.get::<_,String>(0)?,"mapEntityId":row.get::<_,String>(1)?,"label":row.get::<_,Option<String>>(2)?,"role":row.get::<_,String>(3)?,"anchorKind":row.get::<_,String>(4)?,"provider":row.get::<_,Option<String>>(5)?,"featureKind":row.get::<_,Option<String>>(6)?,"featureId":row.get::<_,Option<String>>(7)?,"bounds":[row.get::<_,Option<f64>>(8)?,row.get::<_,Option<f64>>(9)?,row.get::<_,Option<f64>>(10)?,row.get::<_,Option<f64>>(11)?],"validity":{"from":row.get::<_,Option<String>>(12)?,"to":row.get::<_,Option<String>>(13)?},"resolution":row.get::<_,String>(14)?})))?;
        rows.collect::<Result<Vec<_>, _>>().map_err(CoreError::from)
    }

    /// Returns the disposable projection rows for all locations on a map.
    /// The projection carries provider selectors, bounds, validity, and
    /// resolution state; it is rebuilt from the canonical `locations` fields
    /// and the source asset bytes and is never treated as durable state.
    pub fn map_location_projection(
        &self,
        map_entity_id: String,
    ) -> Result<Vec<serde_json::Value>, CoreError> {
        self.ensure_source_index_current()?;
        let mut statement = self.connection.prepare("SELECT location_id,entity_id,label,role,anchor_kind,provider,feature_kind,feature_id,min_x,min_y,max_x,max_y,valid_from,valid_to,resolution FROM map_location_projection WHERE map_entity_id=?1 ORDER BY location_id")?;
        let rows = statement.query_map(rusqlite::params![map_entity_id], |row| Ok(serde_json::json!({"id":row.get::<_,String>(0)?,"entityId":row.get::<_,String>(1)?,"label":row.get::<_,Option<String>>(2)?,"role":row.get::<_,String>(3)?,"anchorKind":row.get::<_,String>(4)?,"provider":row.get::<_,Option<String>>(5)?,"featureKind":row.get::<_,Option<String>>(6)?,"featureId":row.get::<_,Option<String>>(7)?,"bounds":[row.get::<_,Option<f64>>(8)?,row.get::<_,Option<f64>>(9)?,row.get::<_,Option<f64>>(10)?,row.get::<_,Option<f64>>(11)?],"validity":{"from":row.get::<_,Option<String>>(12)?,"to":row.get::<_,Option<String>>(13)?},"resolution":row.get::<_,String>(14)?})))?;
        rows.collect::<Result<Vec<_>, _>>().map_err(CoreError::from)
    }

    /// Rebuilds the disposable map projections so provider-feature resolution
    /// reflects the current source asset bytes, then returns per-location
    /// resolution results for the given map. Called after every source save so
    /// removed or renumbered FMG features surface as `unresolved` immediately
    /// rather than on the next full index build.
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
    pub fn map_locations(&self, entity_id: String) -> Result<Vec<crate::maps::LocationReference>, CoreError> {
        let field = self
            .list_fields(entity_id)?
            .into_iter()
            .find(|field| field.namespace == crate::maps::MAP_NAMESPACE && field.key == "locations");
        let Some(field) = field else { return Ok(Vec::new()); };
        let object = field.value.as_object().ok_or_else(|| CoreError::Serialization("maps.locations is not an object".into()))?;
        let locations = object.get("locations").and_then(serde_json::Value::as_array).ok_or_else(|| CoreError::Serialization("maps.locations is not an array".into()))?;
        locations.iter().cloned().map(|value| serde_json::from_value(value).map_err(|error| CoreError::Serialization(error.to_string()))).collect()
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
        self.set_field_with_request(FieldValue {
            entity_id,
            namespace: crate::maps::MAP_NAMESPACE.into(),
            key: "locations".into(),
            value: serde_json::json!({"schemaVersion": 1, "locations": locations}),
            revision: String::new(),
        }, request_id)
    }

    pub fn unlink_map_location(&self, entity_id: String, location_id: String, request_id: Option<&str>) -> Result<(), CoreError> {
        let mut locations = self.map_locations(entity_id.clone())?;
        let before = locations.len();
        locations.retain(|location| location.id != location_id);
        if locations.len() == before { return Err(CoreError::NotFound("map location not found".into())); }
        self.set_field_with_request(FieldValue {
            entity_id,
            namespace: crate::maps::MAP_NAMESPACE.into(),
            key: "locations".into(),
            value: serde_json::json!({"schemaVersion": 1, "locations": locations}),
            revision: String::new(),
        }, request_id)
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

    pub fn restore_plugin_backup(&self, backup: &PluginBackup) -> Result<(), CoreError> {
        self.restore_plugin_backup_with_request(backup, None)
    }

    pub fn restore_plugin_backup_with_request(
        &self,
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

    pub fn restore(&self, path: String) -> Result<(), CoreError> {
        let content =
            std::fs::read_to_string(&path).map_err(|e| CoreError::NotFound(e.to_string()))?;
        self.restore_payload(&content)?;
        Ok(())
    }

    pub fn restore_payload(&self, payload: &str) -> Result<(), CoreError> {
        self.restore_payload_with_request(payload, None)?;
        Ok(())
    }

    pub fn restore_payload_with_request(
        &self,
        payload: &str,
        request_id: Option<&str>,
    ) -> Result<(), CoreError> {
        if self.root.is_some() {
            let payload = payload.to_owned();
            return self.repository_first_mutation(request_id, move |projection, request_id| {
                projection.restore_payload_with_request(&payload, Some(request_id))
            });
        }
        if self
            .committed_mutation::<serde_json::Value>(request_id)?
            .is_some()
        {
            return Ok(());
        }
        self.import_json_with_mode_and_sync_with_request(payload, true, true, request_id)?;
        Ok(())
    }

    /// Remove only data owned by a plugin. Code uninstall and disablement do
    /// not call this method; callers must present the explicit confirmation
    /// phrase and a backup is created before the destructive transaction.
    pub fn delete_plugin_data(
        &self,
        plugin_id: &str,
        confirmation: &str,
    ) -> Result<String, CoreError> {
        self.delete_plugin_data_with_request(plugin_id, confirmation, None)
    }

    pub fn delete_plugin_data_with_request(
        &self,
        plugin_id: &str,
        confirmation: &str,
        request_id: Option<&str>,
    ) -> Result<String, CoreError> {
        if self.root.is_some() {
            return self.repository_first_mutation(request_id, move |projection, request_id| {
                projection.delete_plugin_data_with_request(
                    plugin_id,
                    confirmation,
                    Some(request_id),
                )
            });
        }
        if let Some(backup) = self.committed_mutation::<String>(request_id)? {
            return Ok(backup);
        }
        if plugin_id.trim().is_empty() || confirmation != plugin_id {
            return Err(CoreError::Unauthorized {
                operation: "confirm plugin data deletion",
            });
        }
        let backup = self.backup()?;
        let transaction = self.connection.unchecked_transaction()?;
        transaction.execute(
            "DELETE FROM entity_fields WHERE namespace IN (SELECT namespace FROM module_namespaces WHERE module_id=?1)",
            params![plugin_id],
        )?;
        transaction.execute(
            "DELETE FROM assets WHERE namespace IN (SELECT namespace FROM module_namespaces WHERE module_id=?1)",
            params![plugin_id],
        )?;
        transaction.execute(
            "DELETE FROM module_fields WHERE module_id=?1",
            params![plugin_id],
        )?;
        transaction.execute(
            "DELETE FROM module_namespaces WHERE module_id=?1",
            params![plugin_id],
        )?;
        transaction.execute(
            "DELETE FROM module_versions WHERE module_id=?1",
            params![plugin_id],
        )?;
        transaction.execute(
            "DELETE FROM module_state WHERE module_id=?1",
            params![plugin_id],
        )?;
        transaction.execute(
            "DELETE FROM module_package_versions WHERE module_id=?1",
            params![plugin_id],
        )?;
        transaction.execute(
            "DELETE FROM migration_history WHERE module_id=?1",
            params![plugin_id],
        )?;
        transaction.commit()?;
        let request_id = self.request_id(request_id)?;
        let result = serde_json::to_value(&backup)
            .map_err(|error| CoreError::Serialization(error.to_string()))?;
        self.sync_canonical_with_request_id(&request_id, Some(&result))?;
        Ok(backup)
    }

    pub fn rebuild_search(&self) -> Result<(), CoreError> {
        self.connection.execute_batch(
            "DROP TRIGGER IF EXISTS entities_search_insert;
             DROP TRIGGER IF EXISTS entities_search_update;
             DROP TRIGGER IF EXISTS documents_search_insert;
             DROP TRIGGER IF EXISTS documents_search_update;
             DROP TRIGGER IF EXISTS entity_fields_search_insert;
             DROP TRIGGER IF EXISTS entity_fields_search_update;
             DROP TABLE IF EXISTS world_search;
             CREATE VIRTUAL TABLE world_search USING fts5(entity_id UNINDEXED, source_path UNINDEXED, source_hash UNINDEXED, content);
             INSERT INTO world_search(entity_id, source_path, source_hash, content) SELECT e.id, COALESCE(sf.path, 'entities/' || e.id || '/entity.json'), COALESCE(sf.content_hash, ''), e.name || ' ' || COALESCE(e.entity_type, '') FROM entities e LEFT JOIN source_files sf ON sf.path = 'entities/' || e.id || '/entity.json' WHERE e.deleted=0;
             INSERT INTO world_search(entity_id, source_path, source_hash, content) SELECT d.entity_id, COALESCE(sf.path, 'entities/' || d.entity_id || '/document.md'), COALESCE(sf.content_hash, ''), d.body FROM documents d JOIN entities e ON e.id = d.entity_id LEFT JOIN source_files sf ON sf.path = 'entities/' || d.entity_id || '/document.md' WHERE e.deleted=0;
             INSERT INTO world_search(entity_id, source_path, source_hash, content) SELECT f.entity_id, COALESCE(sf.path, ''), COALESCE(sf.content_hash, ''), f.namespace || ' ' || f.key || ' ' || f.value FROM entity_fields f JOIN entities e ON e.id = f.entity_id LEFT JOIN source_files sf ON sf.path LIKE 'entities/' || f.entity_id || '/fields/%.json' WHERE e.deleted=0;
             CREATE TRIGGER entities_search_insert AFTER INSERT ON entities BEGIN INSERT INTO world_search(entity_id, content) SELECT new.id, new.name || ' ' || COALESCE(new.entity_type, '') WHERE new.deleted=0; END;
             CREATE TRIGGER entities_search_update AFTER UPDATE OF name, entity_type, deleted ON entities BEGIN DELETE FROM world_search WHERE entity_id = old.id; INSERT INTO world_search(entity_id, content) SELECT new.id, new.name || ' ' || COALESCE(new.entity_type, '') WHERE new.deleted=0; INSERT INTO world_search(entity_id, content) SELECT documents.entity_id, documents.body FROM documents WHERE documents.entity_id = new.id AND new.deleted=0; INSERT INTO world_search(entity_id, content) SELECT entity_fields.entity_id, entity_fields.namespace || ' ' || entity_fields.key || ' ' || entity_fields.value FROM entity_fields WHERE entity_fields.entity_id = new.id AND new.deleted=0; END;
             CREATE TRIGGER documents_search_insert AFTER INSERT ON documents BEGIN INSERT INTO world_search(entity_id, content) VALUES (new.entity_id, new.body); END;
             CREATE TRIGGER documents_search_update AFTER UPDATE OF body ON documents BEGIN DELETE FROM world_search WHERE entity_id = old.entity_id; INSERT INTO world_search(entity_id, content) SELECT id, name || ' ' || COALESCE(entity_type, '') FROM entities WHERE id = new.entity_id AND deleted=0; INSERT INTO world_search(entity_id, content) SELECT documents.entity_id, documents.body FROM documents JOIN entities ON entities.id = documents.entity_id WHERE documents.entity_id = new.entity_id AND entities.deleted=0; INSERT INTO world_search(entity_id, content) SELECT entity_fields.entity_id, entity_fields.namespace || ' ' || entity_fields.key || ' ' || entity_fields.value FROM entity_fields JOIN entities ON entities.id = entity_fields.entity_id WHERE entity_fields.entity_id = new.entity_id AND entities.deleted=0; END;
             CREATE TRIGGER entity_fields_search_insert AFTER INSERT ON entity_fields BEGIN DELETE FROM world_search WHERE entity_id = new.entity_id; INSERT INTO world_search(entity_id, content) SELECT id, name || ' ' || COALESCE(entity_type, '') FROM entities WHERE id = new.entity_id AND deleted=0; INSERT INTO world_search(entity_id, content) SELECT documents.entity_id, documents.body FROM documents JOIN entities ON entities.id = documents.entity_id WHERE documents.entity_id = new.entity_id AND entities.deleted=0; INSERT INTO world_search(entity_id, content) SELECT entity_fields.entity_id, entity_fields.namespace || ' ' || entity_fields.key || ' ' || entity_fields.value FROM entity_fields JOIN entities ON entities.id = entity_fields.entity_id WHERE entity_fields.entity_id = new.entity_id AND entities.deleted=0; END;
             CREATE TRIGGER entity_fields_search_update AFTER UPDATE OF namespace, key, value ON entity_fields BEGIN DELETE FROM world_search WHERE entity_id = new.entity_id; INSERT INTO world_search(entity_id, content) SELECT id, name || ' ' || COALESCE(entity_type, '') FROM entities WHERE id = new.entity_id AND deleted=0; INSERT INTO world_search(entity_id, content) SELECT documents.entity_id, documents.body FROM documents JOIN entities ON entities.id = documents.entity_id WHERE documents.entity_id = new.entity_id AND entities.deleted=0; INSERT INTO world_search(entity_id, content) SELECT entity_fields.entity_id, entity_fields.namespace || ' ' || entity_fields.key || ' ' || entity_fields.value FROM entity_fields JOIN entities ON entities.id = entity_fields.entity_id WHERE entity_fields.entity_id = new.entity_id AND entities.deleted=0; END;"
        )?;
        self.rebuild_maps_projection()?;
        Ok(())
    }

    pub fn seed_example(&mut self) -> Result<usize, CoreError> {
        if self.root.is_some() {
            return self.repository_first_mutation(None, |projection, _request_id| {
                projection.seed_example_unchecked()
            });
        }
        self.seed_example_unchecked()
    }

    fn seed_example_unchecked(&mut self) -> Result<usize, CoreError> {
        let tx = self.connection.transaction()?;
        tx.execute("DELETE FROM assets", [])?;
        tx.execute("DELETE FROM entity_fields", [])?;
        tx.execute("DELETE FROM documents", [])?;
        tx.execute("DELETE FROM relationships", [])?;
        tx.execute("DELETE FROM entities", [])?;
        let now = chrono_like_now();
        let eldermere_id = Uuid::new_v4().to_string();
        tx.execute("INSERT INTO entities(id,name,entity_type,deleted,created_at,updated_at) VALUES (?1,?2,?3,0,?4,?4)", params![eldermere_id, "Eldermere", "place", now])?;
        let lord_ashford_id = Uuid::new_v4().to_string();
        tx.execute("INSERT INTO entities(id,name,entity_type,deleted,created_at,updated_at) VALUES (?1,?2,?3,0,?4,?4)", params![lord_ashford_id, "Lord Ashford", "person", now])?;
        let glass_coast_id = Uuid::new_v4().to_string();
        tx.execute("INSERT INTO entities(id,name,entity_type,deleted,created_at,updated_at) VALUES (?1,?2,?3,0,?4,?4)", params![glass_coast_id, "The Glass Coast", "place", now])?;
        let silver_hand_id = Uuid::new_v4().to_string();
        tx.execute("INSERT INTO entities(id,name,entity_type,deleted,created_at,updated_at) VALUES (?1,?2,?3,0,?4,?4)", params![silver_hand_id, "The Silver Hand", "faction", now])?;
        let amulet_id = Uuid::new_v4().to_string();
        tx.execute("INSERT INTO entities(id,name,entity_type,deleted,created_at,updated_at) VALUES (?1,?2,?3,0,?4,?4)", params![amulet_id, "Amulet of Tides", "artifact", now])?;
        let highland_id = Uuid::new_v4().to_string();
        tx.execute("INSERT INTO entities(id,name,entity_type,deleted,created_at,updated_at) VALUES (?1,?2,?3,0,?4,?4)", params![highland_id, "Highland Culture", "culture", now])?;
        let founding_id = Uuid::new_v4().to_string();
        tx.execute("INSERT INTO entities(id,name,entity_type,deleted,created_at,updated_at) VALUES (?1,?2,?3,0,?4,?4)", params![founding_id, "Founding of Eldermere", "event", now])?;
        let treaty_id = Uuid::new_v4().to_string();
        tx.execute("INSERT INTO entities(id,name,entity_type,deleted,created_at,updated_at) VALUES (?1,?2,?3,0,?4,?4)", params![treaty_id, "The Treaty of Ashes", "event", now])?;
        let rebellion_id = Uuid::new_v4().to_string();
        tx.execute("INSERT INTO entities(id,name,entity_type,deleted,created_at,updated_at) VALUES (?1,?2,?3,0,?4,?4)", params![rebellion_id, "The Tide Rebellion", "event", now])?;
        let mira_vale_id = Uuid::new_v4().to_string();
        tx.execute("INSERT INTO entities(id,name,entity_type,deleted,created_at,updated_at) VALUES (?1,?2,?3,0,?4,?4)", params![mira_vale_id, "Mira Vale", "person", now])?;
        let sunken_archive_id = Uuid::new_v4().to_string();
        tx.execute("INSERT INTO entities(id,name,entity_type,deleted,created_at,updated_at) VALUES (?1,?2,?3,0,?4,?4)", params![sunken_archive_id, "The Sunken Archive", "place", now])?;
        let ember_court_id = Uuid::new_v4().to_string();
        tx.execute("INSERT INTO entities(id,name,entity_type,deleted,created_at,updated_at) VALUES (?1,?2,?3,0,?4,?4)", params![ember_court_id, "The Ember Court", "faction", now])?;
        let star_compass_id = Uuid::new_v4().to_string();
        tx.execute("INSERT INTO entities(id,name,entity_type,deleted,created_at,updated_at) VALUES (?1,?2,?3,0,?4,?4)", params![star_compass_id, "Star Compass", "artifact", now])?;
        let riverborn_id = Uuid::new_v4().to_string();
        tx.execute("INSERT INTO entities(id,name,entity_type,deleted,created_at,updated_at) VALUES (?1,?2,?3,0,?4,?4)", params![riverborn_id, "Riverborn Culture", "culture", now])?;
        let war_embers_id = Uuid::new_v4().to_string();
        tx.execute("INSERT INTO entities(id,name,entity_type,deleted,created_at,updated_at) VALUES (?1,?2,?3,0,?4,?4)", params![war_embers_id, "The War of Embers", "event", now])?;
        let first_tide_id = Uuid::new_v4().to_string();
        tx.execute("INSERT INTO entities(id,name,entity_type,deleted,created_at,updated_at) VALUES (?1,?2,?3,0,?4,?4)", params![first_tide_id, "The First Tide", "event", now])?;
        let archive_opening_id = Uuid::new_v4().to_string();
        tx.execute("INSERT INTO entities(id,name,entity_type,deleted,created_at,updated_at) VALUES (?1,?2,?3,0,?4,?4)", params![archive_opening_id, "Opening of the Sunken Archive", "event", now])?;
        let frostgate_id = Uuid::new_v4().to_string();
        tx.execute("INSERT INTO entities(id,name,entity_type,deleted,created_at,updated_at) VALUES (?1,?2,?3,0,?4,?4)", params![frostgate_id, "Frostgate Pass", "place", now])?;
        let lantern_marsh_id = Uuid::new_v4().to_string();
        tx.execute("INSERT INTO entities(id,name,entity_type,deleted,created_at,updated_at) VALUES (?1,?2,?3,0,?4,?4)", params![lantern_marsh_id, "Lantern Marsh", "place", now])?;
        let elian_rook_id = Uuid::new_v4().to_string();
        tx.execute("INSERT INTO entities(id,name,entity_type,deleted,created_at,updated_at) VALUES (?1,?2,?3,0,?4,?4)", params![elian_rook_id, "Captain Elian Rook", "person", now])?;
        let sera_ashdown_id = Uuid::new_v4().to_string();
        tx.execute("INSERT INTO entities(id,name,entity_type,deleted,created_at,updated_at) VALUES (?1,?2,?3,0,?4,?4)", params![sera_ashdown_id, "Sera Ashdown", "person", now])?;
        let tidewatch_id = Uuid::new_v4().to_string();
        tx.execute("INSERT INTO entities(id,name,entity_type,deleted,created_at,updated_at) VALUES (?1,?2,?3,0,?4,?4)", params![tidewatch_id, "The Tidewatch", "faction", now])?;
        let crown_salt_id = Uuid::new_v4().to_string();
        tx.execute("INSERT INTO entities(id,name,entity_type,deleted,created_at,updated_at) VALUES (?1,?2,?3,0,?4,?4)", params![crown_salt_id, "Crown of Salt", "artifact", now])?;
        let coastfolk_id = Uuid::new_v4().to_string();
        tx.execute("INSERT INTO entities(id,name,entity_type,deleted,created_at,updated_at) VALUES (?1,?2,?3,0,?4,?4)", params![coastfolk_id, "Coastfolk Culture", "culture", now])?;
        let frostgate_battle_id = Uuid::new_v4().to_string();
        tx.execute("INSERT INTO entities(id,name,entity_type,deleted,created_at,updated_at) VALUES (?1,?2,?3,0,?4,?4)", params![frostgate_battle_id, "The Battle of Frostgate", "event", now])?;
        tx.execute("INSERT INTO documents(id,entity_id,format,body,updated_at) VALUES (?1,?2,?3,?4,?5)", params![Uuid::new_v4().to_string(), eldermere_id, "markdown", "Eldermere is the ancient seat of power, a coastal fortress built upon the cliffs where the river meets the sea.", now])?;
        tx.execute(
            "INSERT INTO documents(id,entity_id,format,body,updated_at) VALUES (?1,?2,?3,?4,?5)",
            params![
                Uuid::new_v4().to_string(),
                lord_ashford_id,
                "markdown",
                "Lord Ashford rules Eldermere with a steady hand and a sharp mind.",
                now
            ],
        )?;
        tx.execute("INSERT INTO documents(id,entity_id,format,body,updated_at) VALUES (?1,?2,?3,?4,?5)", params![Uuid::new_v4().to_string(), glass_coast_id, "markdown", "The Glass Coast is a stretch of shoreline where the sea glass glitters like scattered jewels.", now])?;
        tx.execute(
            "INSERT INTO documents(id,entity_id,format,body,updated_at) VALUES (?1,?2,?3,?4,?5)",
            params![
                Uuid::new_v4().to_string(),
                silver_hand_id,
                "markdown",
                "The Silver Hand is a sworn brotherhood dedicated to the protection of Eldermere.",
                now
            ],
        )?;
        tx.execute(
            "INSERT INTO documents(id,entity_id,format,body,updated_at) VALUES (?1,?2,?3,?4,?5)",
            params![
                Uuid::new_v4().to_string(),
                amulet_id,
                "markdown",
                "The Amulet of Tides grants its bearer the ability to breathe beneath the waves.",
                now
            ],
        )?;
        tx.execute(
            "INSERT INTO documents(id,entity_id,format,body,updated_at) VALUES (?1,?2,?3,?4,?5)",
            params![
                Uuid::new_v4().to_string(),
                highland_id,
                "markdown",
                "The Highland Culture values honor, craftsmanship, and the old songs.",
                now
            ],
        )?;
        tx.execute("INSERT INTO documents(id,entity_id,format,body,updated_at) VALUES (?1,?2,?3,?4,?5)", params![Uuid::new_v4().to_string(), founding_id, "markdown", "The founding of Eldermere marked the unification of the coastal clans under one banner.", now])?;
        tx.execute("INSERT INTO documents(id,entity_id,format,body,updated_at) VALUES (?1,?2,?3,?4,?5)", params![Uuid::new_v4().to_string(), treaty_id, "markdown", "The Treaty of Ashes ended the War of Embers and established the borders of the realm.", now])?;
        tx.execute(
            "INSERT INTO documents(id,entity_id,format,body,updated_at) VALUES (?1,?2,?3,?4,?5)",
            params![
                Uuid::new_v4().to_string(),
                rebellion_id,
                "markdown",
                "The Tide Rebellion was a brief but violent uprising against the crown.",
                now
            ],
        )?;
        tx.execute("INSERT INTO documents(id,entity_id,format,body,updated_at) VALUES (?1,?2,?3,?4,?5)", params![Uuid::new_v4().to_string(), mira_vale_id, "markdown", "Mira Vale is a mapmaker who can read the coast by moonlight and remembers every vanished inlet.", now])?;
        tx.execute("INSERT INTO documents(id,entity_id,format,body,updated_at) VALUES (?1,?2,?3,?4,?5)", params![Uuid::new_v4().to_string(), sunken_archive_id, "markdown", "The Sunken Archive lies beneath the old river delta, its sealed chambers holding histories the crown chose to forget.", now])?;
        tx.execute("INSERT INTO documents(id,entity_id,format,body,updated_at) VALUES (?1,?2,?3,?4,?5)", params![Uuid::new_v4().to_string(), ember_court_id, "markdown", "The Ember Court preserves the old fire rites and quietly contests the Silver Hand's claim to protect the realm.", now])?;
        tx.execute(
            "INSERT INTO documents(id,entity_id,format,body,updated_at) VALUES (?1,?2,?3,?4,?5)",
            params![
                Uuid::new_v4().to_string(),
                star_compass_id,
                "markdown",
                "The Star Compass points toward what has been lost rather than toward north.",
                now
            ],
        )?;
        tx.execute("INSERT INTO documents(id,entity_id,format,body,updated_at) VALUES (?1,?2,?3,?4,?5)", params![Uuid::new_v4().to_string(), riverborn_id, "markdown", "The Riverborn Culture follows the delta's shifting channels and keeps songs for every flood season.", now])?;
        tx.execute(
            "INSERT INTO documents(id,entity_id,format,body,updated_at) VALUES (?1,?2,?3,?4,?5)",
            params![
                Uuid::new_v4().to_string(),
                war_embers_id,
                "markdown",
                "The War of Embers began when the frontier forges refused the crown's new tribute.",
                now
            ],
        )?;
        tx.execute("INSERT INTO documents(id,entity_id,format,body,updated_at) VALUES (?1,?2,?3,?4,?5)", params![Uuid::new_v4().to_string(), first_tide_id, "markdown", "The First Tide is remembered as the night the sea crossed the old boundary stones.", now])?;
        tx.execute("INSERT INTO documents(id,entity_id,format,body,updated_at) VALUES (?1,?2,?3,?4,?5)", params![Uuid::new_v4().to_string(), archive_opening_id, "markdown", "Mira Vale opened the upper vault of the Sunken Archive and found a map bearing the Star Compass mark.", now])?;
        tx.execute("INSERT INTO documents(id,entity_id,format,body,updated_at) VALUES (?1,?2,?3,?4,?5)", params![Uuid::new_v4().to_string(), frostgate_id, "markdown", "Frostgate Pass is the only safe road through the northern cliffs, guarded by old watchtowers and a newer border wall.", now])?;
        tx.execute("INSERT INTO documents(id,entity_id,format,body,updated_at) VALUES (?1,?2,?3,?4,?5)", params![Uuid::new_v4().to_string(), lantern_marsh_id, "markdown", "Lantern Marsh glows with blue reeds after dusk; its hidden channels lead from the delta to the western sea.", now])?;
        tx.execute("INSERT INTO documents(id,entity_id,format,body,updated_at) VALUES (?1,?2,?3,?4,?5)", params![Uuid::new_v4().to_string(), elian_rook_id, "markdown", "Captain Elian Rook commands the coast patrols and distrusts any map that has not survived a flood season.", now])?;
        tx.execute("INSERT INTO documents(id,entity_id,format,body,updated_at) VALUES (?1,?2,?3,?4,?5)", params![Uuid::new_v4().to_string(), sera_ashdown_id, "markdown", "Sera Ashdown is an archivist of the Ember Court who believes the forgotten histories can prevent another war.", now])?;
        tx.execute("INSERT INTO documents(id,entity_id,format,body,updated_at) VALUES (?1,?2,?3,?4,?5)", params![Uuid::new_v4().to_string(), tidewatch_id, "markdown", "The Tidewatch keeps signal fires along the coast and answers to no single noble house.", now])?;
        tx.execute("INSERT INTO documents(id,entity_id,format,body,updated_at) VALUES (?1,?2,?3,?4,?5)", params![Uuid::new_v4().to_string(), crown_salt_id, "markdown", "The Crown of Salt is a ceremonial relic worn by the first ruler to unite the river and coast clans.", now])?;
        tx.execute("INSERT INTO documents(id,entity_id,format,body,updated_at) VALUES (?1,?2,?3,?4,?5)", params![Uuid::new_v4().to_string(), coastfolk_id, "markdown", "The Coastfolk Culture treats the tides as a calendar and settles disputes by reciting the names of drowned villages.", now])?;
        tx.execute("INSERT INTO documents(id,entity_id,format,body,updated_at) VALUES (?1,?2,?3,?4,?5)", params![Uuid::new_v4().to_string(), frostgate_battle_id, "markdown", "The Battle of Frostgate broke the northern siege and made the pass a symbol of the realm's uneasy unity.", now])?;
        tx.execute("INSERT INTO entity_fields(entity_id,namespace,key,value) VALUES (?1,'lore','summary',?2)", params![eldermere_id, "Ancient coastal fortress and seat of power"])?;
        tx.execute("INSERT INTO entity_fields(entity_id,namespace,key,value) VALUES (?1,'lore','aliases',?2)", params![eldermere_id, "The Elder Hold, The Clifftop"])?;
        tx.execute("INSERT INTO entity_fields(entity_id,namespace,key,value) VALUES (?1,'lore','summary',?2)", params![lord_ashford_id, "Ruler of Eldermere and keeper of the realm"])?;
        tx.execute("INSERT INTO entity_fields(entity_id,namespace,key,value) VALUES (?1,'lore','aliases',?2)", params![lord_ashford_id, "Ash, The Lord"])?;
        tx.execute("INSERT INTO entity_fields(entity_id,namespace,key,value) VALUES (?1,'lore','summary',?2)", params![mira_vale_id, "Cartographer of the changing coast"])?;
        tx.execute("INSERT INTO entity_fields(entity_id,namespace,key,value) VALUES (?1,'lore','aliases',?2)", params![mira_vale_id, "The Moonlit Mapmaker"])?;
        tx.execute("INSERT INTO entity_fields(entity_id,namespace,key,value) VALUES (?1,'lore','summary',?2)", params![sunken_archive_id, "Buried repository beneath the river delta"])?;
        tx.execute("INSERT INTO entity_fields(entity_id,namespace,key,value) VALUES (?1,'lore','summary',?2)", params![ember_court_id, "Keepers of the old fire rites"])?;
        tx.execute("INSERT INTO entity_fields(entity_id,namespace,key,value) VALUES (?1,'lore','summary',?2)", params![star_compass_id, "Artifact that points toward lost places"])?;
        tx.execute("INSERT INTO entity_fields(entity_id,namespace,key,value) VALUES (?1,'lore','summary',?2)", params![riverborn_id, "Delta culture of navigators and singers"])?;
        tx.execute("INSERT INTO entity_fields(entity_id,namespace,key,value) VALUES (?1,'timeline','startsAt',?2)", params![founding_id, "0001-01-01"])?;
        tx.execute("INSERT INTO entity_fields(entity_id,namespace,key,value) VALUES (?1,'timeline','endsAt',?2)", params![founding_id, "0001-01-01"])?;
        tx.execute("INSERT INTO entity_fields(entity_id,namespace,key,value) VALUES (?1,'timeline','startsAt',?2)", params![treaty_id, "0042-03-15"])?;
        tx.execute("INSERT INTO entity_fields(entity_id,namespace,key,value) VALUES (?1,'timeline','endsAt',?2)", params![treaty_id, "0042-06-02"])?;
        tx.execute("INSERT INTO entity_fields(entity_id,namespace,key,value) VALUES (?1,'timeline','startsAt',?2)", params![rebellion_id, "0067-07-22"])?;
        tx.execute("INSERT INTO entity_fields(entity_id,namespace,key,value) VALUES (?1,'timeline','endsAt',?2)", params![rebellion_id, "0068-02-11"])?;
        tx.execute("INSERT INTO entity_fields(entity_id,namespace,key,value) VALUES (?1,'timeline','startsAt',?2)", params![war_embers_id, "0038-09-04"])?;
        tx.execute("INSERT INTO entity_fields(entity_id,namespace,key,value) VALUES (?1,'timeline','endsAt',?2)", params![war_embers_id, "0042-03-14"])?;
        tx.execute("INSERT INTO entity_fields(entity_id,namespace,key,value) VALUES (?1,'timeline','startsAt',?2)", params![first_tide_id, "0021-11-19"])?;
        tx.execute("INSERT INTO entity_fields(entity_id,namespace,key,value) VALUES (?1,'timeline','startsAt',?2)", params![archive_opening_id, "0071-04-08"])?;
        tx.execute("INSERT INTO entity_fields(entity_id,namespace,key,value) VALUES (?1,'timeline','endsAt',?2)", params![archive_opening_id, "0071-04-10"])?;
        tx.execute("INSERT INTO entity_fields(entity_id,namespace,key,value) VALUES (?1,'lore','region',?2)", params![eldermere_id, "Southern coast"])?;
        tx.execute("INSERT INTO entity_fields(entity_id,namespace,key,value) VALUES (?1,'lore','climate',?2)", params![eldermere_id, "Salt wind and mild winters"])?;
        tx.execute("INSERT INTO entity_fields(entity_id,namespace,key,value) VALUES (?1,'lore','population',?2)", params![eldermere_id, "18000"])?;
        tx.execute("INSERT INTO entity_fields(entity_id,namespace,key,value) VALUES (?1,'lore','region',?2)", params![glass_coast_id, "Western shore"])?;
        tx.execute("INSERT INTO entity_fields(entity_id,namespace,key,value) VALUES (?1,'lore','climate',?2)", params![glass_coast_id, "Stormy and bright"])?;
        tx.execute("INSERT INTO entity_fields(entity_id,namespace,key,value) VALUES (?1,'lore','population',?2)", params![glass_coast_id, "4200"])?;
        tx.execute(
            "INSERT INTO entity_fields(entity_id,namespace,key,value) VALUES (?1,'lore','goal',?2)",
            params![silver_hand_id, "Protect the realm's roads and rulers"],
        )?;
        tx.execute(
            "INSERT INTO entity_fields(entity_id,namespace,key,value) VALUES (?1,'lore','goal',?2)",
            params![ember_court_id, "Preserve the old fire rites"],
        )?;
        tx.execute("INSERT INTO entity_fields(entity_id,namespace,key,value) VALUES (?1,'lore','values',?2)", params![highland_id, "Honor, craft, and ancestral songs"])?;
        tx.execute("INSERT INTO entity_fields(entity_id,namespace,key,value) VALUES (?1,'lore','language',?2)", params![highland_id, "High Cant"])?;
        tx.execute("INSERT INTO entity_fields(entity_id,namespace,key,value) VALUES (?1,'lore','values',?2)", params![riverborn_id, "Adaptability, memory, and hospitality"])?;
        tx.execute("INSERT INTO entity_fields(entity_id,namespace,key,value) VALUES (?1,'lore','language',?2)", params![riverborn_id, "River Cant"])?;
        tx.execute("INSERT INTO entity_fields(entity_id,namespace,key,value) VALUES (?1,'lore','occupation',?2)", params![lord_ashford_id, "Ruler and treaty keeper"])?;
        tx.execute("INSERT INTO entity_fields(entity_id,namespace,key,value) VALUES (?1,'lore','occupation',?2)", params![mira_vale_id, "Cartographer and coastal guide"])?;
        tx.execute("INSERT INTO entity_fields(entity_id,namespace,key,value) VALUES (?1,'lore','region',?2)", params![frostgate_id, "Northern frontier"])?;
        tx.execute("INSERT INTO entity_fields(entity_id,namespace,key,value) VALUES (?1,'lore','climate',?2)", params![frostgate_id, "Cold, windy, and snowbound"])?;
        tx.execute("INSERT INTO entity_fields(entity_id,namespace,key,value) VALUES (?1,'lore','population',?2)", params![frostgate_id, "900"])?;
        tx.execute("INSERT INTO entity_fields(entity_id,namespace,key,value) VALUES (?1,'lore','region',?2)", params![lantern_marsh_id, "Eastern delta"])?;
        tx.execute("INSERT INTO entity_fields(entity_id,namespace,key,value) VALUES (?1,'lore','climate',?2)", params![lantern_marsh_id, "Wet and misty"])?;
        tx.execute("INSERT INTO entity_fields(entity_id,namespace,key,value) VALUES (?1,'lore','population',?2)", params![lantern_marsh_id, "2300"])?;
        tx.execute("INSERT INTO entity_fields(entity_id,namespace,key,value) VALUES (?1,'lore','occupation',?2)", params![elian_rook_id, "Coast patrol captain"])?;
        tx.execute("INSERT INTO entity_fields(entity_id,namespace,key,value) VALUES (?1,'lore','occupation',?2)", params![sera_ashdown_id, "Archivist and fire-rite scholar"])?;
        tx.execute(
            "INSERT INTO entity_fields(entity_id,namespace,key,value) VALUES (?1,'lore','goal',?2)",
            params![tidewatch_id, "Keep the coast's signal network alive"],
        )?;
        tx.execute("INSERT INTO entity_fields(entity_id,namespace,key,value) VALUES (?1,'lore','material',?2)", params![crown_salt_id, "Silver, pearl, and black coral"])?;
        tx.execute("INSERT INTO entity_fields(entity_id,namespace,key,value) VALUES (?1,'lore','values',?2)", params![coastfolk_id, "Reciprocity, remembrance, and safe passage"])?;
        tx.execute("INSERT INTO entity_fields(entity_id,namespace,key,value) VALUES (?1,'lore','language',?2)", params![coastfolk_id, "Coastal Sign"])?;
        tx.execute("INSERT INTO relationships(id,source_id,target_id,relationship_type,metadata) VALUES (?1,?2,?3,?4,?5)", params![Uuid::new_v4().to_string(), lord_ashford_id, eldermere_id, "originates_from", "{}"])?;
        tx.execute("INSERT INTO relationships(id,source_id,target_id,relationship_type,metadata) VALUES (?1,?2,?3,?4,?5)", params![Uuid::new_v4().to_string(), lord_ashford_id, silver_hand_id, "affiliated_with", "{}"])?;
        tx.execute("INSERT INTO relationships(id,source_id,target_id,relationship_type,metadata) VALUES (?1,?2,?3,?4,?5)", params![Uuid::new_v4().to_string(), amulet_id, mira_vale_id, "created_by", "{}"])?;
        tx.execute("INSERT INTO relationships(id,source_id,target_id,relationship_type,metadata) VALUES (?1,?2,?3,?4,?5)", params![Uuid::new_v4().to_string(), amulet_id, lord_ashford_id, "owned_by", "{}"])?;
        tx.execute("INSERT INTO relationships(id,source_id,target_id,relationship_type,metadata) VALUES (?1,?2,?3,?4,?5)", params![Uuid::new_v4().to_string(), silver_hand_id, lord_ashford_id, "led_by", "{}"])?;
        tx.execute("INSERT INTO relationships(id,source_id,target_id,relationship_type,metadata) VALUES (?1,?2,?3,?4,?5)", params![Uuid::new_v4().to_string(), silver_hand_id, eldermere_id, "based_in", "{}"])?;
        tx.execute("INSERT INTO relationships(id,source_id,target_id,relationship_type,metadata) VALUES (?1,?2,?3,?4,?5)", params![Uuid::new_v4().to_string(), highland_id, eldermere_id, "rooted_in", "{}"])?;
        tx.execute("INSERT INTO relationships(id,source_id,target_id,relationship_type,metadata) VALUES (?1,?2,?3,?4,?5)", params![Uuid::new_v4().to_string(), elian_rook_id, glass_coast_id, "originates_from", "{}"])?;
        tx.execute("INSERT INTO relationships(id,source_id,target_id,relationship_type,metadata) VALUES (?1,?2,?3,?4,?5)", params![Uuid::new_v4().to_string(), elian_rook_id, silver_hand_id, "affiliated_with", "{}"])?;
        tx.execute("INSERT INTO relationships(id,source_id,target_id,relationship_type,metadata) VALUES (?1,?2,?3,?4,?5)", params![Uuid::new_v4().to_string(), sera_ashdown_id, sunken_archive_id, "originates_from", "{}"])?;
        tx.execute("INSERT INTO relationships(id,source_id,target_id,relationship_type,metadata) VALUES (?1,?2,?3,?4,?5)", params![Uuid::new_v4().to_string(), sera_ashdown_id, ember_court_id, "affiliated_with", "{}"])?;
        tx.execute("INSERT INTO relationships(id,source_id,target_id,relationship_type,metadata) VALUES (?1,?2,?3,?4,?5)", params![Uuid::new_v4().to_string(), tidewatch_id, elian_rook_id, "led_by", "{}"])?;
        tx.execute("INSERT INTO relationships(id,source_id,target_id,relationship_type,metadata) VALUES (?1,?2,?3,?4,?5)", params![Uuid::new_v4().to_string(), tidewatch_id, lantern_marsh_id, "based_in", "{}"])?;
        tx.execute("INSERT INTO relationships(id,source_id,target_id,relationship_type,metadata) VALUES (?1,?2,?3,?4,?5)", params![Uuid::new_v4().to_string(), tidewatch_id, frostgate_id, "based_in", "{}"])?;
        tx.execute("INSERT INTO relationships(id,source_id,target_id,relationship_type,metadata) VALUES (?1,?2,?3,?4,?5)", params![Uuid::new_v4().to_string(), crown_salt_id, sera_ashdown_id, "created_by", "{}"])?;
        tx.execute("INSERT INTO relationships(id,source_id,target_id,relationship_type,metadata) VALUES (?1,?2,?3,?4,?5)", params![Uuid::new_v4().to_string(), crown_salt_id, lord_ashford_id, "owned_by", "{}"])?;
        tx.execute("INSERT INTO relationships(id,source_id,target_id,relationship_type,metadata) VALUES (?1,?2,?3,?4,?5)", params![Uuid::new_v4().to_string(), coastfolk_id, glass_coast_id, "rooted_in", "{}"])?;
        tx.execute("INSERT INTO relationships(id,source_id,target_id,relationship_type,metadata) VALUES (?1,?2,?3,?4,?5)", params![Uuid::new_v4().to_string(), coastfolk_id, lantern_marsh_id, "rooted_in", "{}"])?;
        tx.execute("INSERT INTO relationships(id,source_id,target_id,relationship_type,metadata) VALUES (?1,?2,?3,?4,?5)", params![Uuid::new_v4().to_string(), ember_court_id, sera_ashdown_id, "led_by", "{}"])?;
        tx.execute("INSERT OR IGNORE INTO module_versions(module_id,version) VALUES ('daena.lore',1), ('daena.timeline',1)", [])?;
        tx.execute("INSERT OR IGNORE INTO module_namespaces(module_id,namespace) VALUES ('daena.lore','lore'), ('daena.timeline','timeline')", [])?;
        tx.commit()?;
        let map = self.create_map("The Known Coast".into())?;
        for (entity_id, role, label, feature_id, x, y) in [
            (eldermere_id, "capital", "Eldermere", "101", 0.62, 0.44),
            (glass_coast_id, "region", "The Glass Coast", "102", 0.18, 0.71),
            (frostgate_id, "frontier", "Frostgate Pass", "103", 0.85, 0.12),
            (lantern_marsh_id, "region", "Lantern Marsh", "104", 0.77, 0.38),
            (mira_vale_id, "home", "Mira Vale's workshop", "105", 0.55, 0.63),
            (crown_salt_id, "resting-place", "Crown of Salt", "106", 0.31, 0.29),
        ] {
            self.upsert_map_location(
                entity_id,
                crate::maps::LocationReference {
                    id: Uuid::new_v4().to_string(),
                    map_entity_id: map.id.clone(),
                    role: role.into(),
                    label: label.into(),
                    anchor: crate::maps::Anchor::ProviderFeature {
                        provider: crate::maps::FMG_PROVIDER.into(),
                        feature_kind: "burg".into(),
                        feature_id: feature_id.into(),
                        fallback_point: crate::maps::Point(x, y),
                    },
                    validity: crate::maps::Validity {
                        from: None,
                        to: None,
                    },
                },
                None,
            )?;
        }
        self.rebuild_search()?;
        self.sync_canonical()?;
        Ok(26)
    }

    pub fn get_module_version(&self, module_id: &str) -> Result<i64, CoreError> {
        self.ensure_source_index_current()?;
        self.connection
            .query_row(
                "SELECT COALESCE(version, 0) FROM module_versions WHERE module_id=?1",
                params![module_id],
                |row| row.get(0),
            )
            .or_else(|e| {
                if e == rusqlite::Error::QueryReturnedNoRows {
                    Ok(0)
                } else {
                    Err(e)
                }
            })
            .map_err(CoreError::Database)
    }

    pub fn module_states(&self) -> Result<Vec<ModuleState>, CoreError> {
        self.ensure_source_index_current()?;
        let mut statement = self.connection.prepare("SELECT m.module_id, COALESCE(s.enabled, 1), m.version, p.package_version FROM module_versions m LEFT JOIN module_state s ON s.module_id = m.module_id LEFT JOIN module_package_versions p ON p.module_id = m.module_id ORDER BY m.module_id")?;
        let rows = statement.query_map([], |row| {
            Ok(ModuleState {
                module_id: row.get(0)?,
                enabled: row.get::<_, i64>(1)? != 0,
                version: row.get(2)?,
                package_version: row.get(3)?,
            })
        })?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    pub fn validate_migration(
        &self,
        migration: &crate::migrations::Migration,
        current: i64,
    ) -> Result<(), CoreError> {
        crate::migrations::validate(migration, current)
    }

    pub fn set_module_enabled(&self, module_id: String, enabled: bool) -> Result<(), CoreError> {
        self.set_module_enabled_with_request(module_id, enabled, None)
    }

    pub fn set_module_enabled_with_request(
        &self,
        module_id: String,
        enabled: bool,
        request_id: Option<&str>,
    ) -> Result<(), CoreError> {
        if self.root.is_some() {
            return self.repository_first_mutation(request_id, move |projection, request_id| {
                projection.set_module_enabled_with_request(module_id, enabled, Some(request_id))
            });
        }
        if self
            .committed_mutation::<serde_json::Value>(request_id)?
            .is_some()
        {
            return Ok(());
        }
        self.connection.execute(
            "INSERT OR IGNORE INTO module_versions(module_id,version) VALUES (?1,0)",
            params![module_id],
        )?;
        self.connection.execute(
            "INSERT INTO module_state(module_id, enabled) VALUES (?1, ?2) ON CONFLICT(module_id) DO UPDATE SET enabled=excluded.enabled",
            params![module_id, enabled as i64],
        )?;
        let request_id = self.request_id(request_id)?;
        self.sync_canonical_with_request_id(&request_id, Some(&serde_json::Value::Null))?;
        Ok(())
    }

    pub fn is_module_enabled(&self, module_id: &str) -> Result<bool, CoreError> {
        self.ensure_source_index_current()?;
        Ok(self
            .connection
            .query_row(
                "SELECT enabled FROM module_state WHERE module_id=?1",
                params![module_id],
                |row| row.get::<_, i64>(0),
            )
            .optional()?
            .unwrap_or(1)
            != 0)
    }

    pub fn set_module_package_version(
        &self,
        module_id: &str,
        package_version: Option<&str>,
    ) -> Result<(), CoreError> {
        self.set_module_package_version_with_request(module_id, package_version, None)
    }

    pub fn set_module_package_version_with_request(
        &self,
        module_id: &str,
        package_version: Option<&str>,
        request_id: Option<&str>,
    ) -> Result<(), CoreError> {
        if self.root.is_some() {
            let module_id = module_id.to_owned();
            let package_version = package_version.map(str::to_owned);
            return self.repository_first_mutation(request_id, move |projection, request_id| {
                projection.set_module_package_version_with_request(
                    &module_id,
                    package_version.as_deref(),
                    Some(request_id),
                )
            });
        }
        if self
            .committed_mutation::<serde_json::Value>(request_id)?
            .is_some()
        {
            return Ok(());
        }
        match package_version {
            Some(version) => {
                self.connection.execute(
                    "INSERT OR IGNORE INTO module_versions(module_id,version) VALUES (?1,0)",
                    params![module_id],
                )?;
                self.connection.execute(
                    "INSERT INTO module_package_versions(module_id,package_version) VALUES (?1,?2) ON CONFLICT(module_id) DO UPDATE SET package_version=excluded.package_version",
                    params![module_id, version],
                )?;
            }
            None => {
                self.connection.execute(
                    "DELETE FROM module_package_versions WHERE module_id=?1",
                    params![module_id],
                )?;
            }
        }
        let request_id = self.request_id(request_id)?;
        self.sync_canonical_with_request_id(&request_id, Some(&serde_json::Value::Null))?;
        Ok(())
    }

    pub fn module_package_version(&self, module_id: &str) -> Result<Option<String>, CoreError> {
        self.ensure_source_index_current()?;
        self.connection
            .query_row(
                "SELECT package_version FROM module_package_versions WHERE module_id=?1",
                params![module_id],
                |row| row.get(0),
            )
            .optional()
            .map_err(CoreError::from)
    }

    pub fn apply_migration(
        &mut self,
        migration: &crate::migrations::Migration,
    ) -> Result<(), CoreError> {
        self.apply_migrations_with_request(std::slice::from_ref(migration), None)
            .map(|_| ())
    }

    /// Apply a plugin migration chain after one backup. If a later migration
    /// fails, restore the backup so the project remains usable at its prior
    /// data version.
    pub fn apply_migrations(
        &mut self,
        migrations: &[crate::migrations::Migration],
    ) -> Result<String, CoreError> {
        self.apply_migrations_with_request(migrations, None)
    }

    pub fn apply_migrations_with_request(
        &mut self,
        migrations: &[crate::migrations::Migration],
        request_id: Option<&str>,
    ) -> Result<String, CoreError> {
        if self.root.is_some() {
            let migrations = migrations.to_vec();
            return self.repository_first_mutation(request_id, move |projection, request_id| {
                projection.apply_migrations_with_request(&migrations, Some(request_id))
            });
        }
        if let Some(backup) = self.committed_mutation::<String>(request_id)? {
            return Ok(backup);
        }
        let backup = self
            .backup()
            .map_err(|error| CoreError::Validation(error.to_string()))?;
        for migration in migrations {
            if let Err(error) = crate::migrations::apply(&mut self.connection, migration) {
                let restore_result = self.restore(backup.clone());
                return match restore_result {
                    Ok(()) => Err(error),
                    Err(restore_error) => Err(CoreError::Validation(format!(
                        "migration failed and backup restore failed: {error}; {restore_error}"
                    ))),
                };
            }
        }
        self.rebuild_search()?;
        let request_id = self.request_id(request_id)?;
        let result = serde_json::to_value(&backup)
            .map_err(|error| CoreError::Serialization(error.to_string()))?;
        self.sync_canonical_with_request_id(&request_id, Some(&result))?;
        Ok(backup)
    }
}

pub(crate) fn chrono_like_now() -> String {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
        .to_string()
}

/// Parses a recovery copy file name of the form
/// `<epochMillis>-<entityId>-<uuid>.map`, returning the epoch milliseconds when
/// the name matches the entity. Both the trailing UUID and the embedded entity
/// ID must validate so that files from other maps (or traversal attempts) are
/// never touched.
fn parse_map_recovery_file_name(entity_id: &str, file_name: &str) -> Option<String> {
    let stem = file_name.strip_suffix(".map")?;
    if stem.len() < 37 + 36 {
        return None;
    }
    let uuid_part = &stem[stem.len() - 36..];
    Uuid::parse_str(uuid_part).ok()?;
    let prefix = &stem[..stem.len() - 37];
    let created_at = prefix.strip_suffix(&format!("-{entity_id}"))?;
    if created_at.is_empty() || !created_at.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    Some(created_at.to_string())
}

fn digest_bytes(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn revision_digest<T: Serialize>(value: &T) -> Result<String, CoreError> {
    let bytes =
        serde_json::to_vec(value).map_err(|error| CoreError::Serialization(error.to_string()))?;
    Ok(format!("sha256:{}", digest_bytes(&bytes)))
}

fn bounds_for_points(
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

#[cfg(test)]
mod tests;
