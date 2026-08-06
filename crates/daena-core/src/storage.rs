use crate::error::CoreError;
use crate::project::{
    Asset, Document, Entity, FieldValue, MigrationHistoryEntry, ModuleNamespace, ModuleState,
    ProjectSnapshot, Relationship,
};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Component, Path, PathBuf};
use uuid::Uuid;

pub const PROJECT_FORMAT_VERSION: u32 = 2;
pub const CORE_PLUGIN_ID: &str = "daena.core";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
pub struct ProjectManifest {
    pub format_version: u32,
    pub id: String,
    pub name: String,
    pub created_at: String,
}

impl ProjectManifest {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            format_version: PROJECT_FORMAT_VERSION,
            id: Uuid::new_v4().to_string(),
            name: name.into(),
            created_at: crate::project::chrono_like_now(),
        }
    }

    pub fn validate(&self, path: &Path) -> Result<(), CoreError> {
        if self.format_version != PROJECT_FORMAT_VERSION {
            return Err(codec_error(
                path,
                "project.format-version",
                format!(
                    "expected formatVersion {}, found {}",
                    PROJECT_FORMAT_VERSION, self.format_version
                ),
            ));
        }
        validate_uuid(path, "project.id", &self.id)?;
        if self.name.trim().is_empty() {
            return Err(codec_error(path, "project.name", "name cannot be empty"));
        }
        if self.created_at.trim().is_empty() {
            return Err(codec_error(
                path,
                "project.created-at",
                "createdAt cannot be empty",
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
pub struct EntityFile {
    pub id: String,
    pub name: String,
    #[serde(rename = "type")]
    pub entity_type: Option<String>,
    pub deleted: bool,
    pub created_at: String,
    pub updated_at: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub document: Option<EntityDocumentRef>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct EntityDocumentRef {
    pub id: String,
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct RelationshipsFile {
    pub relationships: Vec<CanonicalRelationship>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
pub struct CanonicalRelationship {
    pub id: String,
    pub target_id: String,
    #[serde(rename = "type")]
    pub relationship_type: String,
    pub metadata: serde_json::Value,
}

pub type FieldsFile = BTreeMap<String, serde_json::Value>;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct AssetsFile {
    pub assets: Vec<CanonicalAsset>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
pub struct CanonicalAsset {
    pub id: String,
    pub namespace: String,
    pub filename: String,
    pub content_hash: String,
    pub size: i64,
    pub mime_type: String,
    pub path: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
pub struct PluginStateFile {
    pub plugin_id: String,
    pub namespaces: Vec<String>,
    pub data_version: i64,
    pub schema_version: i64,
    pub schema_checksum: String,
    pub selected_package_version: Option<String>,
    pub migrations: Vec<CanonicalMigration>,
    pub preserved_state: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
pub struct CanonicalMigration {
    pub id: String,
    pub from_version: i64,
    pub to_version: i64,
    pub checksum: String,
    pub package_digest: String,
    pub applied_at: String,
}

#[derive(Debug, Clone)]
pub struct CanonicalProject {
    pub manifest: ProjectManifest,
    pub snapshot: ProjectSnapshot,
    pub sources: Vec<CanonicalSource>,
}

/// A canonical file that was part of a successful project scan.
///
/// The disposable index stores these rows verbatim.  Keeping the source path
/// and hash beside derived data makes it possible to explain where an index
/// row came from and prevents a database-only interpretation of a project.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanonicalSource {
    pub path: String,
    pub content_hash: String,
    pub format_version: u32,
}

/// Filesystem-backed repository boundary for canonical project data.
///
/// The repository owns the scan and codec; callers receive a validated
/// snapshot and its source manifest, never a database connection or an
/// arbitrary filesystem handle.
#[derive(Debug, Clone)]
pub struct FilesystemRepository {
    root: PathBuf,
}

impl FilesystemRepository {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, CoreError> {
        let root = path.as_ref();
        let metadata = fs::symlink_metadata(root)
            .map_err(|error| codec_error(root, "repository.open", error))?;
        if metadata.file_type().is_symlink() || !metadata.is_dir() {
            return Err(codec_error(
                root,
                "repository.root",
                "repository root must be a real directory",
            ));
        }
        validate_canonical_layout(root)?;
        Ok(Self {
            root: root.to_path_buf(),
        })
    }

    pub fn scan(&self) -> Result<CanonicalProject, CoreError> {
        read_canonical_project(&self.root)
    }

    pub fn root(&self) -> &Path {
        &self.root
    }
}

impl PluginStateFile {
    fn validate(&self, path: &Path) -> Result<(), CoreError> {
        validate_component(&self.plugin_id, path, "plugin.id")?;
        if self.data_version < 0 || self.schema_version < 0 {
            return Err(codec_error(
                path,
                "plugin.version",
                "versions cannot be negative",
            ));
        }
        if self.schema_checksum.trim().is_empty() {
            return Err(codec_error(
                path,
                "plugin.schema-checksum",
                "schema checksum cannot be empty",
            ));
        }
        if !self.preserved_state.is_object() {
            return Err(codec_error(
                path,
                "plugin.preserved-state",
                "preserved state must be an object",
            ));
        }
        let mut namespaces = BTreeSet::new();
        for namespace in &self.namespaces {
            validate_component(namespace, path, "plugin.namespace")?;
            if !namespaces.insert(namespace.to_ascii_lowercase()) {
                return Err(codec_error(path, "plugin.namespace", "duplicate namespace"));
            }
        }
        Ok(())
    }
}

impl EntityFile {
    pub fn validate(&self, path: &Path) -> Result<(), CoreError> {
        validate_uuid(path, "entity.id", &self.id)?;
        if self.name.trim().is_empty() {
            return Err(codec_error(path, "entity.name", "name cannot be empty"));
        }
        if self.created_at.trim().is_empty() || self.updated_at.trim().is_empty() {
            return Err(codec_error(
                path,
                "entity.timestamp",
                "createdAt and updatedAt cannot be empty",
            ));
        }
        if let Some(document) = &self.document {
            validate_uuid(path, "entity.document.id", &document.id)?;
            if document.path != "document.md" {
                return Err(codec_error(
                    path,
                    "entity.document-path",
                    "document path must be document.md",
                ));
            }
        }
        Ok(())
    }
}

pub fn write_canonical_project(
    root: &Path,
    manifest: &ProjectManifest,
    snapshot: &ProjectSnapshot,
) -> Result<(), CoreError> {
    let manifest_path = root.join("project.json");
    manifest.validate(&manifest_path)?;
    fs::create_dir_all(root).map_err(|error| codec_error(root, "project.mkdir", error))?;
    let entities_root = normalized_project_path(root, "entities")?;
    let plugins_root = normalized_project_path(root, "plugins")?;
    fs::create_dir_all(&entities_root)
        .map_err(|error| codec_error(root, "entities.mkdir", error))?;
    fs::create_dir_all(&plugins_root).map_err(|error| codec_error(root, "plugins.mkdir", error))?;
    write_json(&manifest_path, manifest)?;

    let entity_ids = snapshot
        .entities
        .iter()
        .map(|entity| entity.id.clone())
        .collect::<BTreeSet<_>>();
    if entity_ids.len() != snapshot.entities.len() {
        return Err(codec_error(
            &manifest_path,
            "entity.duplicate-id",
            "entity IDs must be globally unique",
        ));
    }

    // The caller writes into transaction staging, which starts as a copy of
    // the current canonical tree. Remove records absent from the proposed
    // snapshot here so replacement/restore mutations cannot leave stale
    // entity or plugin files behind.
    if entities_root.exists() {
        for entry in fs::read_dir(&entities_root)
            .map_err(|error| codec_error(&entities_root, "entities.read", error))?
        {
            let entry =
                entry.map_err(|error| codec_error(&entities_root, "entities.read", error))?;
            let name = entry.file_name().to_string_lossy().into_owned();
            if !is_ignored_metadata_entry(&name) && !entity_ids.contains(&name) {
                fs::remove_dir_all(entry.path())
                    .map_err(|error| codec_error(&entities_root, "entity.remove", error))?;
            }
        }
    }

    let namespace_owners = namespace_owners(snapshot)?;
    let mut field_files: BTreeMap<(String, String, String), FieldsFile> = BTreeMap::new();
    for field in &snapshot.fields {
        validate_uuid(&manifest_path, "field.entity-id", &field.entity_id)?;
        if !entity_ids.contains(&field.entity_id) {
            return Err(codec_error(
                &manifest_path,
                "field.entity-reference",
                format!("field references unknown entity {}", field.entity_id),
            ));
        }
        validate_component(&field.namespace, &manifest_path, "field.namespace")?;
        validate_component(&field.key, &manifest_path, "field.key")?;
        let owner = namespace_owners
            .get(&field.namespace)
            .cloned()
            .unwrap_or_else(|| CORE_PLUGIN_ID.to_string());
        field_files
            .entry((field.entity_id.clone(), owner, field.namespace.clone()))
            .or_default()
            .insert(field.key.clone(), field.value.clone());
    }

    let mut relationships: BTreeMap<String, Vec<CanonicalRelationship>> = BTreeMap::new();
    let mut relationship_ids = BTreeSet::new();
    for relationship in &snapshot.relationships {
        validate_uuid(&manifest_path, "relationship.id", &relationship.id)?;
        validate_uuid(
            &manifest_path,
            "relationship.source-id",
            &relationship.source_id,
        )?;
        validate_uuid(
            &manifest_path,
            "relationship.target-id",
            &relationship.target_id,
        )?;
        if !entity_ids.contains(&relationship.source_id)
            || !entity_ids.contains(&relationship.target_id)
        {
            return Err(codec_error(
                &manifest_path,
                "relationship.entity-reference",
                "relationship must reference known entities",
            ));
        }
        if !relationship_ids.insert(relationship.id.clone()) {
            return Err(codec_error(
                &manifest_path,
                "relationship.duplicate-id",
                format!("duplicate relationship {}", relationship.id),
            ));
        }
        let metadata: serde_json::Value = serde_json::from_str(&relationship.metadata)
            .map_err(|error| codec_error(&manifest_path, "relationship.metadata", error))?;
        if !metadata.is_object() {
            return Err(codec_error(
                &manifest_path,
                "relationship.metadata",
                "metadata must be a JSON object",
            ));
        }
        relationships
            .entry(relationship.source_id.clone())
            .or_default()
            .push(CanonicalRelationship {
                id: relationship.id.clone(),
                target_id: relationship.target_id.clone(),
                relationship_type: relationship.relationship_type.clone(),
                metadata,
            });
    }

    let mut assets: BTreeMap<String, Vec<CanonicalAsset>> = BTreeMap::new();
    let mut asset_ids = BTreeSet::new();
    for asset in &snapshot.assets {
        validate_uuid(&manifest_path, "asset.id", &asset.id)?;
        validate_uuid(&manifest_path, "asset.entity-id", &asset.entity_id)?;
        if !entity_ids.contains(&asset.entity_id) {
            return Err(codec_error(
                &manifest_path,
                "asset.entity-reference",
                "asset must reference a known entity",
            ));
        }
        validate_component(&asset.namespace, &manifest_path, "asset.namespace")?;
        if asset.filename.trim().is_empty() {
            return Err(codec_error(
                &manifest_path,
                "asset.filename",
                "asset filename cannot be empty",
            ));
        }
        if !asset_ids.insert(asset.id.clone()) {
            return Err(codec_error(
                &manifest_path,
                "asset.duplicate-id",
                format!("duplicate asset {}", asset.id),
            ));
        }
        let asset_path = normalized_project_path(root, &asset.path)?;
        if !asset.path.starts_with("assets/") {
            return Err(codec_error(
                &manifest_path,
                "asset.path",
                "asset path must remain below assets/",
            ));
        }
        if !asset_path.is_file() {
            return Err(codec_error(
                &manifest_path,
                "asset.missing",
                format!("asset bytes are missing at {}", asset.path),
            ));
        }
        if digest_file(&asset_path)? != asset.content_hash {
            return Err(codec_error(
                &manifest_path,
                "asset.hash",
                format!("asset hash mismatch at {}", asset.path),
            ));
        }
        assets
            .entry(asset.entity_id.clone())
            .or_default()
            .push(CanonicalAsset {
                id: asset.id.clone(),
                namespace: asset.namespace.clone(),
                filename: asset.filename.clone(),
                content_hash: asset.content_hash.clone(),
                size: asset.size,
                mime_type: asset.mime_type.clone(),
                path: asset.path.clone(),
                created_at: asset.created_at.clone(),
            });
    }

    let mut documents_by_entity = BTreeMap::new();
    for document in &snapshot.documents {
        if documents_by_entity
            .insert(document.entity_id.clone(), document)
            .is_some()
        {
            return Err(codec_error(
                &manifest_path,
                "document.multiple",
                format!("entity {} has more than one document", document.entity_id),
            ));
        }
    }

    for entity in &snapshot.entities {
        let entity_dir = normalized_project_path(root, &format!("entities/{}", entity.id))?;
        fs::create_dir_all(&entity_dir)
            .map_err(|error| codec_error(&entity_dir, "entity.mkdir", error))?;
        let document = documents_by_entity
            .get(&entity.id)
            .map(|document| EntityDocumentRef {
                id: document.id.clone(),
                path: "document.md".into(),
            });
        let entity_file = EntityFile {
            id: entity.id.clone(),
            name: entity.name.clone(),
            entity_type: entity.entity_type.clone(),
            deleted: entity.deleted,
            created_at: entity.created_at.clone(),
            updated_at: entity.updated_at.clone(),
            document,
        };
        entity_file.validate(&entity_dir.join("entity.json"))?;
        let entity_json_path =
            normalized_project_path(root, &format!("entities/{}/entity.json", entity.id))?;
        write_json(&entity_json_path, &entity_file)?;

        let document_path =
            normalized_project_path(root, &format!("entities/{}/document.md", entity.id))?;
        if let Some(document) = documents_by_entity.get(&entity.id) {
            fs::write(
                &document_path,
                canonical_markdown(&document.body).as_bytes(),
            )
            .map_err(|error| codec_error(&document_path, "markdown.write", error))?;
        } else if document_path.exists() {
            fs::remove_file(&document_path)
                .map_err(|error| codec_error(&document_path, "markdown.remove", error))?;
        }

        let fields_dir = normalized_project_path(root, &format!("entities/{}/fields", entity.id))?;
        fs::create_dir_all(&fields_dir)
            .map_err(|error| codec_error(&fields_dir, "fields.mkdir", error))?;
        clear_json_files(&fields_dir)?;
        let mut field_paths = BTreeSet::new();
        for ((entity_id, plugin_id, namespace), fields) in field_files
            .iter()
            .filter(|((id, _, _), _)| id == &entity.id)
        {
            let path = fields_dir.join(format!("{plugin_id}--{namespace}.json"));
            if entity_id != &entity.id {
                continue;
            }
            if !field_paths.insert(path.to_string_lossy().to_ascii_lowercase()) {
                return Err(codec_error(
                    &path,
                    "path.case-collision",
                    "field paths collide case-insensitively",
                ));
            }
            write_json(&path, fields)?;
        }

        let relationships_path =
            normalized_project_path(root, &format!("entities/{}/relationships.json", entity.id))?;
        if let Some(mut values) = relationships.remove(&entity.id) {
            values.sort_by(|left, right| left.id.cmp(&right.id));
            write_json(
                &relationships_path,
                &RelationshipsFile {
                    relationships: values,
                },
            )?;
        } else if relationships_path.exists() {
            fs::remove_file(&relationships_path)
                .map_err(|error| codec_error(&relationships_path, "relationships.remove", error))?;
        }

        let assets_path =
            normalized_project_path(root, &format!("entities/{}/assets.json", entity.id))?;
        if let Some(mut values) = assets.remove(&entity.id) {
            values.sort_by(|left, right| left.id.cmp(&right.id));
            write_json(&assets_path, &AssetsFile { assets: values })?;
        } else if assets_path.exists() {
            fs::remove_file(&assets_path)
                .map_err(|error| codec_error(&assets_path, "assets.remove", error))?;
        }
    }

    let mut plugin_ids = BTreeSet::new();
    for module in &snapshot.modules {
        plugin_ids.insert(module.module_id.clone());
    }
    for namespace in &snapshot.module_namespaces {
        plugin_ids.insert(namespace.module_id.clone());
    }
    for migration in &snapshot.migration_history {
        plugin_ids.insert(migration.module_id.clone());
    }
    if plugins_root.exists() {
        for entry in fs::read_dir(&plugins_root)
            .map_err(|error| codec_error(&plugins_root, "plugins.read", error))?
        {
            let entry = entry.map_err(|error| codec_error(&plugins_root, "plugins.read", error))?;
            let filename = entry.file_name().to_string_lossy().into_owned();
            let plugin_id = filename.strip_suffix(".json").unwrap_or_default();
            if !is_ignored_metadata_entry(&filename) && !plugin_ids.contains(plugin_id) {
                fs::remove_file(entry.path())
                    .map_err(|error| codec_error(&plugins_root, "plugin.remove", error))?;
            }
        }
    }
    for plugin_id in plugin_ids {
        validate_component(&plugin_id, &manifest_path, "plugin.id")?;
        let module = snapshot
            .modules
            .iter()
            .find(|module| module.module_id == plugin_id);
        let mut namespaces = snapshot
            .module_namespaces
            .iter()
            .filter(|namespace| namespace.module_id == plugin_id)
            .map(|namespace| namespace.namespace.clone())
            .collect::<Vec<_>>();
        namespaces.sort();
        namespaces.dedup();
        for namespace in &namespaces {
            validate_component(namespace, &manifest_path, "plugin.namespace")?;
        }
        let schema_fields = snapshot
            .module_fields
            .iter()
            .filter(|field| field.module_id == plugin_id)
            .collect::<Vec<_>>();
        let path = normalized_project_path(root, &format!("plugins/{plugin_id}.json"))?;
        let existing_state = read_existing_plugin_state(&path)?;
        let schema_checksum = if schema_fields.is_empty() {
            if let Some(state) = &existing_state {
                state.schema_checksum.clone()
            } else {
                digest_bytes(&canonical_json_bytes(&schema_fields)?)
            }
        } else {
            digest_bytes(&canonical_json_bytes(&schema_fields)?)
        };
        let migrations = snapshot
            .migration_history
            .iter()
            .filter(|migration| migration.module_id == plugin_id)
            .map(|migration| CanonicalMigration {
                id: migration.migration_id.clone(),
                from_version: migration.from_version,
                to_version: migration.to_version,
                checksum: migration.checksum.clone(),
                package_digest: migration.package_digest.clone(),
                applied_at: migration.applied_at.clone(),
            })
            .collect::<Vec<_>>();
        let preserved_state = existing_state
            .map(|state| state.preserved_state)
            .unwrap_or_else(|| serde_json::json!({}));
        let state = PluginStateFile {
            plugin_id,
            namespaces,
            data_version: module.map(|module| module.version).unwrap_or_default(),
            schema_version: module.map(|module| module.version).unwrap_or_default(),
            schema_checksum,
            selected_package_version: module.and_then(|module| module.package_version.clone()),
            migrations,
            preserved_state,
        };
        write_json(&path, &state)?;
    }
    Ok(())
}

pub fn read_canonical_project(root: &Path) -> Result<CanonicalProject, CoreError> {
    let manifest_path = root.join("project.json");
    let manifest: ProjectManifest = read_json(&manifest_path)?;
    manifest.validate(&manifest_path)?;
    validate_canonical_layout(root)?;
    let entities_dir = normalized_project_path(root, "entities")?;
    let mut entities = Vec::new();
    let mut documents = Vec::new();
    let mut fields = Vec::new();
    let mut relationships = Vec::new();
    let mut assets = Vec::new();
    let mut entity_ids = BTreeSet::new();
    let mut document_ids = BTreeSet::new();
    let mut relationship_ids = BTreeSet::new();
    let mut asset_ids = BTreeSet::new();
    let mut field_owners = Vec::new();
    if entities_dir.exists() {
        let mut entries = fs::read_dir(&entities_dir)
            .map_err(|error| codec_error(&entities_dir, "entities.read", error))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|error| codec_error(&entities_dir, "entities.read", error))?;
        entries.sort_by_key(|entry| entry.file_name());
        let mut names = BTreeSet::new();
        for entry in entries {
            let name = entry.file_name().to_string_lossy().into_owned();
            if is_ignored_metadata_entry(&name) {
                continue;
            }
            if !names.insert(name.to_ascii_lowercase()) {
                return Err(codec_error(
                    &entities_dir,
                    "path.case-collision",
                    format!("case-colliding entity directory {name}"),
                ));
            }
            validate_component(&name, &entities_dir, "entity.directory")?;
            let entity_dir = normalized_project_path(root, &format!("entities/{name}"))?;
            if !entry
                .file_type()
                .map_err(|error| codec_error(&entity_dir, "entity.type", error))?
                .is_dir()
            {
                return Err(codec_error(
                    &entity_dir,
                    "entity.directory",
                    "entity entry must be a directory",
                ));
            }
            let entity_file_path =
                normalized_project_path(root, &format!("entities/{name}/entity.json"))?;
            let entity_file: EntityFile = read_json(&entity_file_path)?;
            entity_file.validate(&entity_file_path)?;
            Uuid::parse_str(&name)
                .map_err(|error| codec_error(&entity_dir, "entity.directory", error))?;
            if entity_file.id != name || !entity_ids.insert(entity_file.id.clone()) {
                return Err(codec_error(
                    &entity_file_path,
                    "entity.id",
                    "entity ID must match its directory and be unique",
                ));
            }
            entities.push(Entity {
                id: entity_file.id.clone(),
                name: entity_file.name,
                entity_type: entity_file.entity_type,
                deleted: entity_file.deleted,
                created_at: entity_file.created_at.clone(),
                updated_at: entity_file.updated_at.clone(),
                revision: String::new(),
            });
            let document_path =
                normalized_project_path(root, &format!("entities/{name}/document.md"))?;
            if let Some(document_ref) = entity_file.document {
                validate_uuid(&entity_file_path, "entity.document.id", &document_ref.id)?;
                if !document_ids.insert(document_ref.id.clone()) {
                    return Err(codec_error(
                        &entity_file_path,
                        "document.duplicate-id",
                        "document IDs must be globally unique",
                    ));
                }
                let bytes = fs::read(&document_path)
                    .map_err(|error| codec_error(&document_path, "markdown.read", error))?;
                let body = canonical_markdown_bytes(&document_path, &bytes)?;
                documents.push(Document {
                    id: document_ref.id,
                    entity_id: entity_file.id.clone(),
                    format: "markdown".into(),
                    body: String::from_utf8(body)
                        .map_err(|error| codec_error(&document_path, "markdown.utf8", error))?,
                    updated_at: entity_file.updated_at.clone(),
                    revision: String::new(),
                });
            } else if document_path.exists() {
                return Err(codec_error(
                    &document_path,
                    "markdown.orphan",
                    "document.md has no entity document reference",
                ));
            }

            let fields_dir = normalized_project_path(root, &format!("entities/{name}/fields"))?;
            if fields_dir.exists() {
                let mut field_entries = fs::read_dir(&fields_dir)
                    .map_err(|error| codec_error(&fields_dir, "fields.read", error))?
                    .collect::<Result<Vec<_>, _>>()
                    .map_err(|error| codec_error(&fields_dir, "fields.read", error))?;
                field_entries.sort_by_key(|entry| entry.file_name());
                let mut field_names = BTreeSet::new();
                for entry in field_entries {
                    let filename = entry.file_name().to_string_lossy().into_owned();
                    if is_ignored_metadata_entry(&filename) {
                        continue;
                    }
                    if !field_names.insert(filename.to_ascii_lowercase()) {
                        return Err(codec_error(&fields_dir, "path.case-collision", filename));
                    }
                    if !filename.ends_with(".json") {
                        return Err(codec_error(
                            &fields_dir,
                            "fields.file",
                            "field files must use .json",
                        ));
                    }
                    let stem = filename.trim_end_matches(".json");
                    let mut parts = stem.split("--");
                    let plugin_id = parts.next().unwrap_or_default();
                    let namespace = parts.next().unwrap_or_default();
                    if parts.next().is_some() {
                        return Err(codec_error(
                            &fields_dir,
                            "fields.filename",
                            "field filename must contain one plugin--namespace separator",
                        ));
                    }
                    validate_component(plugin_id, &fields_dir, "fields.plugin-id")?;
                    validate_component(namespace, &fields_dir, "fields.namespace")?;
                    let path = fields_dir.join(&filename);
                    let values: FieldsFile = read_json(&path)?;
                    for (key, value) in values {
                        validate_component(&key, &path, "field.key")?;
                        fields.push(FieldValue {
                            entity_id: entity_file.id.clone(),
                            namespace: namespace.into(),
                            key,
                            value,
                            revision: String::new(),
                        });
                    }
                    field_owners.push((plugin_id.to_string(), namespace.to_string()));
                }
            }

            let relationships_path =
                normalized_project_path(root, &format!("entities/{name}/relationships.json"))?;
            if relationships_path.exists() {
                let file: RelationshipsFile = read_json(&relationships_path)?;
                for relationship in file.relationships {
                    validate_uuid(&relationships_path, "relationship.id", &relationship.id)?;
                    validate_uuid(
                        &relationships_path,
                        "relationship.target-id",
                        &relationship.target_id,
                    )?;
                    if !relationship_ids.insert(relationship.id.clone()) {
                        return Err(codec_error(
                            &relationships_path,
                            "relationship.duplicate-id",
                            relationship.id,
                        ));
                    }
                    if relationship.relationship_type.trim().is_empty()
                        || !relationship.metadata.is_object()
                    {
                        return Err(codec_error(
                            &relationships_path,
                            "relationship.value",
                            "relationship type and object metadata are required",
                        ));
                    }
                    relationships.push(Relationship {
                        id: relationship.id,
                        source_id: entity_file.id.clone(),
                        target_id: relationship.target_id,
                        relationship_type: relationship.relationship_type,
                        metadata: canonical_json_string(&relationship.metadata)?,
                        revision: String::new(),
                    });
                }
            }

            let assets_path =
                normalized_project_path(root, &format!("entities/{name}/assets.json"))?;
            if assets_path.exists() {
                let file: AssetsFile = read_json(&assets_path)?;
                for asset in file.assets {
                    validate_uuid(&assets_path, "asset.id", &asset.id)?;
                    validate_component(&asset.namespace, &assets_path, "asset.namespace")?;
                    if !asset_ids.insert(asset.id.clone()) {
                        return Err(codec_error(&assets_path, "asset.duplicate-id", asset.id));
                    }
                    if asset.size < 0 {
                        return Err(codec_error(
                            &assets_path,
                            "asset.size",
                            "asset size cannot be negative",
                        ));
                    }
                    if asset.filename.trim().is_empty() {
                        return Err(codec_error(
                            &assets_path,
                            "asset.filename",
                            "asset filename cannot be empty",
                        ));
                    }
                    let asset_path = normalized_project_path(root, &asset.path)?;
                    if !asset.path.starts_with("assets/") || !asset_path.is_file() {
                        return Err(codec_error(
                            &assets_path,
                            "asset.path",
                            "asset must be a file below assets/",
                        ));
                    }
                    if digest_file(&asset_path)? != asset.content_hash {
                        return Err(codec_error(&assets_path, "asset.hash", asset.path));
                    }
                    let actual_size = fs::metadata(&asset_path)
                        .map_err(|error| codec_error(&asset_path, "asset.size", error))?
                        .len();
                    if actual_size != asset.size as u64 {
                        return Err(codec_error(
                            &assets_path,
                            "asset.size",
                            format!("declared {}, found {actual_size}", asset.size),
                        ));
                    }
                    assets.push(Asset {
                        id: asset.id,
                        entity_id: entity_file.id.clone(),
                        namespace: asset.namespace,
                        filename: asset.filename,
                        content_hash: asset.content_hash,
                        size: asset.size,
                        mime_type: asset.mime_type,
                        path: asset.path,
                        created_at: asset.created_at,
                        revision: String::new(),
                    });
                }
            }
        }
    }

    for relationship in &relationships {
        if !entity_ids.contains(&relationship.target_id) {
            return Err(codec_error(
                &manifest_path,
                "relationship.target-reference",
                relationship.target_id.clone(),
            ));
        }
    }
    let plugins_dir = normalized_project_path(root, "plugins")?;
    let mut modules = Vec::new();
    let mut module_namespaces = Vec::new();
    let mut migration_history = Vec::new();
    let mut known_owners = BTreeSet::new();
    if plugins_dir.exists() {
        let mut entries = fs::read_dir(&plugins_dir)
            .map_err(|error| codec_error(&plugins_dir, "plugins.read", error))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|error| codec_error(&plugins_dir, "plugins.read", error))?;
        entries.sort_by_key(|entry| entry.file_name());
        let mut plugin_paths = BTreeSet::new();
        for entry in entries {
            let filename = entry.file_name().to_string_lossy().into_owned();
            if is_ignored_metadata_entry(&filename) {
                continue;
            }
            if !filename.ends_with(".json") {
                return Err(codec_error(
                    &plugins_dir,
                    "plugins.file",
                    "plugin state files must use .json",
                ));
            }
            let plugin_id = filename.trim_end_matches(".json");
            if !plugin_paths.insert(filename.to_ascii_lowercase()) {
                return Err(codec_error(&plugins_dir, "path.case-collision", filename));
            }
            validate_component(plugin_id, &plugins_dir, "plugin.id")?;
            let path = plugins_dir.join(&filename);
            let state: PluginStateFile = read_json(&path)?;
            state.validate(&path)?;
            if state.plugin_id != plugin_id || !known_owners.insert(plugin_id.to_string()) {
                return Err(codec_error(
                    &path,
                    "plugin.id",
                    "plugin ID must match its filename and be unique",
                ));
            }
            validate_component(plugin_id, &path, "plugin.id")?;
            let mut namespaces = state.namespaces.clone();
            namespaces.sort();
            namespaces.dedup();
            for namespace in namespaces {
                validate_component(&namespace, &path, "plugin.namespace")?;
                module_namespaces.push(ModuleNamespace {
                    module_id: plugin_id.into(),
                    namespace,
                });
            }
            modules.push(ModuleState {
                module_id: plugin_id.into(),
                enabled: true,
                version: state.data_version,
                package_version: state.selected_package_version,
            });
            for migration in state.migrations {
                migration_history.push(MigrationHistoryEntry {
                    module_id: plugin_id.into(),
                    migration_id: migration.id,
                    from_version: migration.from_version,
                    to_version: migration.to_version,
                    checksum: migration.checksum,
                    package_digest: migration.package_digest,
                    applied_at: migration.applied_at,
                });
            }
        }
    }
    for (plugin_id, namespace) in field_owners {
        if plugin_id != CORE_PLUGIN_ID
            && !module_namespaces
                .iter()
                .any(|owner| owner.module_id == plugin_id && owner.namespace == namespace)
        {
            return Err(codec_error(
                &manifest_path,
                "namespace.owner",
                format!("{plugin_id} does not own {namespace}"),
            ));
        }
    }
    for relationship in &relationships {
        if !entity_ids.contains(&relationship.source_id) {
            return Err(codec_error(
                &manifest_path,
                "relationship.source-reference",
                relationship.source_id.clone(),
            ));
        }
    }
    modules.sort_by(|left, right| left.module_id.cmp(&right.module_id));
    module_namespaces.sort_by(|left, right| {
        (&left.module_id, &left.namespace).cmp(&(&right.module_id, &right.namespace))
    });
    migration_history.sort_by(|left, right| {
        (&left.module_id, &left.migration_id).cmp(&(&right.module_id, &right.migration_id))
    });
    let sources = collect_canonical_sources(root)?;
    Ok(CanonicalProject {
        manifest,
        snapshot: ProjectSnapshot {
            format_version: 1,
            entities,
            documents,
            fields,
            relationships,
            assets,
            modules,
            module_namespaces,
            module_fields: Vec::new(),
            migration_history,
        },
        sources,
    })
}

fn validate_canonical_layout(root: &Path) -> Result<(), CoreError> {
    for relative in ["project.json", "entities", "plugins", "assets"] {
        let path = root.join(relative);
        let metadata = match fs::symlink_metadata(&path) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
            Err(error) => return Err(codec_error(&path, "path.read", error)),
        };
        if metadata.file_type().is_symlink() {
            return Err(codec_error(
                &path,
                "path.symlink",
                "canonical paths cannot cross symlinks",
            ));
        }
    }

    validate_canonical_directory(root, "entities", |relative, metadata| {
        if metadata.is_dir() {
            validate_entity_directory(root, relative)
        } else {
            Err(codec_error(
                relative,
                "entity.directory",
                "entity entry must be a directory",
            ))
        }
    })?;
    validate_canonical_directory(root, "plugins", |relative, metadata| {
        if !metadata.is_file() {
            return Err(codec_error(
                relative,
                "plugins.file",
                "plugin state entries must be files",
            ));
        }
        let name = relative
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or_default();
        if !name.ends_with(".json") {
            return Err(codec_error(
                relative,
                "plugins.file",
                "plugin state files must use .json",
            ));
        }
        Ok(())
    })?;
    validate_asset_directory(root, Path::new("assets"))?;
    Ok(())
}

fn validate_canonical_directory<F>(
    root: &Path,
    relative: &str,
    validate_entry: F,
) -> Result<(), CoreError>
where
    F: Fn(&Path, &fs::Metadata) -> Result<(), CoreError>,
{
    let directory = root.join(relative);
    let metadata = match fs::symlink_metadata(&directory) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(codec_error(&directory, "directory.read", error)),
    };
    if !metadata.is_dir() || metadata.file_type().is_symlink() {
        return Err(codec_error(
            &directory,
            "path.symlink",
            "canonical directory must be a real directory",
        ));
    }
    let mut entries = fs::read_dir(&directory)
        .map_err(|error| codec_error(&directory, "directory.read", error))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| codec_error(&directory, "directory.read", error))?;
    entries.sort_by_key(|entry| entry.file_name());
    for entry in entries {
        let name = entry.file_name().to_string_lossy().into_owned();
        if is_ignored_metadata_entry(&name) {
            continue;
        }
        let entry_path = directory.join(&name);
        let metadata = fs::symlink_metadata(&entry_path)
            .map_err(|error| codec_error(&entry_path, "path.read", error))?;
        if metadata.file_type().is_symlink() {
            return Err(codec_error(
                &entry_path,
                "path.symlink",
                "canonical paths cannot cross symlinks",
            ));
        }
        validate_entry(&entry_path, &metadata)?;
    }
    Ok(())
}

fn validate_entity_directory(root: &Path, entity_dir: &Path) -> Result<(), CoreError> {
    let name = entity_dir
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or_default();
    validate_component(name, entity_dir, "entity.directory")?;
    Uuid::parse_str(name).map_err(|error| codec_error(entity_dir, "entity.directory", error))?;
    let allowed = [
        "entity.json",
        "document.md",
        "fields",
        "relationships.json",
        "assets.json",
    ];
    let mut entries = fs::read_dir(entity_dir)
        .map_err(|error| codec_error(entity_dir, "entity.read", error))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| codec_error(entity_dir, "entity.read", error))?;
    entries.sort_by_key(|entry| entry.file_name());
    for entry in entries {
        let name = entry.file_name().to_string_lossy().into_owned();
        if is_ignored_metadata_entry(&name) {
            continue;
        }
        if !allowed.contains(&name.as_str()) {
            return Err(codec_error(
                &entry.path(),
                "entity.path",
                "unknown entry in entity directory",
            ));
        }
        let metadata = fs::symlink_metadata(entry.path())
            .map_err(|error| codec_error(&entry.path(), "path.read", error))?;
        if metadata.file_type().is_symlink() {
            return Err(codec_error(
                &entry.path(),
                "path.symlink",
                "canonical paths cannot cross symlinks",
            ));
        }
        if name == "fields" {
            if !metadata.is_dir() {
                return Err(codec_error(
                    &entry.path(),
                    "fields.directory",
                    "fields must be a directory",
                ));
            }
            validate_fields_directory(&entry.path())?;
        } else if !metadata.is_file() {
            return Err(codec_error(
                &entry.path(),
                "entity.file",
                "entity records must be files",
            ));
        }
    }
    let _ = root;
    Ok(())
}

fn validate_fields_directory(directory: &Path) -> Result<(), CoreError> {
    let mut entries = fs::read_dir(directory)
        .map_err(|error| codec_error(directory, "fields.read", error))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| codec_error(directory, "fields.read", error))?;
    entries.sort_by_key(|entry| entry.file_name());
    let mut names = BTreeSet::new();
    for entry in entries {
        let name = entry.file_name().to_string_lossy().into_owned();
        if is_ignored_metadata_entry(&name) {
            continue;
        }
        if !names.insert(name.to_ascii_lowercase()) {
            return Err(codec_error(directory, "path.case-collision", name));
        }
        let metadata = fs::symlink_metadata(entry.path())
            .map_err(|error| codec_error(&entry.path(), "path.read", error))?;
        if metadata.file_type().is_symlink() || !metadata.is_file() {
            return Err(codec_error(
                &entry.path(),
                "fields.file",
                "field entries must be regular files",
            ));
        }
    }
    Ok(())
}

fn validate_asset_directory(root: &Path, relative: &Path) -> Result<(), CoreError> {
    let directory = root.join(relative);
    let metadata = match fs::symlink_metadata(&directory) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(codec_error(&directory, "assets.read", error)),
    };
    if metadata.file_type().is_symlink() {
        return Err(codec_error(
            &directory,
            "path.symlink",
            "asset directories cannot be symlinks",
        ));
    }
    if metadata.is_file() {
        return Ok(());
    }
    for entry in
        fs::read_dir(&directory).map_err(|error| codec_error(&directory, "assets.read", error))?
    {
        let entry = entry.map_err(|error| codec_error(&directory, "assets.read", error))?;
        let path = entry.path();
        let metadata = fs::symlink_metadata(&path)
            .map_err(|error| codec_error(&path, "assets.read", error))?;
        if metadata.file_type().is_symlink() {
            return Err(codec_error(
                &path,
                "path.symlink",
                "asset paths cannot cross symlinks",
            ));
        }
        let child = relative.join(entry.file_name());
        if metadata.is_dir() {
            validate_asset_directory(root, &child)?;
        }
    }
    Ok(())
}

fn collect_canonical_sources(root: &Path) -> Result<Vec<CanonicalSource>, CoreError> {
    let mut paths = Vec::new();
    for relative in ["project.json", "entities", "plugins", "assets"] {
        let path = root.join(relative);
        if path.is_dir() {
            collect_files(root, &path, &mut paths)?;
        } else if path.is_file() && relative == "project.json" {
            paths.push(path);
        }
    }
    paths.sort_by(|left, right| {
        left.strip_prefix(root)
            .unwrap()
            .cmp(right.strip_prefix(root).unwrap())
    });
    paths
        .into_iter()
        .map(|path| {
            let bytes =
                fs::read(&path).map_err(|error| codec_error(&path, "source.read", error))?;
            let relative = path
                .strip_prefix(root)
                .map_err(|error| codec_error(&path, "source.path", error))?;
            let path = relative.to_string_lossy().replace('\\', "/");
            let format_version = PROJECT_FORMAT_VERSION;
            Ok(CanonicalSource {
                path,
                content_hash: digest_bytes(&bytes),
                format_version,
            })
        })
        .collect()
}

fn collect_files(root: &Path, directory: &Path, paths: &mut Vec<PathBuf>) -> Result<(), CoreError> {
    let mut entries = fs::read_dir(directory)
        .map_err(|error| codec_error(directory, "source.read", error))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| codec_error(directory, "source.read", error))?;
    entries.sort_by_key(|entry| entry.file_name());
    for entry in entries {
        let name = entry.file_name().to_string_lossy().into_owned();
        if is_ignored_metadata_entry(&name) {
            continue;
        }
        let path = entry.path();
        let metadata = fs::symlink_metadata(&path)
            .map_err(|error| codec_error(&path, "source.read", error))?;
        if metadata.file_type().is_symlink() {
            return Err(codec_error(
                &path,
                "path.symlink",
                "canonical paths cannot cross symlinks",
            ));
        }
        if metadata.is_dir() {
            collect_files(root, &path, paths)?;
        } else if metadata.is_file() {
            let _ = root;
            paths.push(path);
        }
    }
    Ok(())
}

pub fn normalized_project_path(root: &Path, relative: &str) -> Result<PathBuf, CoreError> {
    if relative.is_empty()
        || relative.contains('\\')
        || relative.contains('\0')
        || relative.starts_with('/')
        || relative.split('/').any(|component| component.is_empty())
    {
        return Err(codec_error(
            root,
            "path.invalid",
            "path must be a normalized relative path",
        ));
    }
    let path = Path::new(relative);
    for component in path.components() {
        match component {
            Component::Normal(value) => {
                let value = value.to_string_lossy();
                if value == "." || value == ".." || value.contains(':') {
                    return Err(codec_error(root, "path.invalid", relative));
                }
            }
            _ => return Err(codec_error(root, "path.invalid", relative)),
        }
    }
    let mut current = root.to_path_buf();
    if fs::symlink_metadata(&current)
        .map_err(|error| codec_error(&current, "path.root", error))?
        .file_type()
        .is_symlink()
    {
        return Err(codec_error(
            root,
            "path.symlink",
            "project root cannot be a symlink",
        ));
    }
    for component in path.components() {
        current.push(component.as_os_str());
        if let Ok(metadata) = fs::symlink_metadata(&current) {
            if metadata.file_type().is_symlink() {
                return Err(codec_error(
                    &current,
                    "path.symlink",
                    "canonical paths cannot cross symlinks",
                ));
            }
        }
    }
    Ok(root.join(path))
}

fn namespace_owners(snapshot: &ProjectSnapshot) -> Result<BTreeMap<String, String>, CoreError> {
    let mut owners = BTreeMap::new();
    for namespace in &snapshot.module_namespaces {
        validate_component(
            &namespace.module_id,
            Path::new("project.json"),
            "namespace.plugin-id",
        )?;
        validate_component(
            &namespace.namespace,
            Path::new("project.json"),
            "namespace.name",
        )?;
        if let Some(existing) =
            owners.insert(namespace.namespace.clone(), namespace.module_id.clone())
        {
            if existing != namespace.module_id {
                return Err(codec_error(
                    Path::new("project.json"),
                    "namespace.owner",
                    format!("namespace {} has multiple owners", namespace.namespace),
                ));
            }
        }
    }
    Ok(owners)
}

fn validate_component(value: &str, path: &Path, code: &str) -> Result<(), CoreError> {
    if value.is_empty()
        || value == "."
        || value == ".."
        || value.contains('/')
        || value.contains('\\')
        || value.contains('\0')
        || value.chars().any(|character| character.is_control())
        || !value.chars().all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '.' | '-' | '_')
        })
    {
        return Err(codec_error(
            path,
            code,
            format!("invalid path component {value:?}"),
        ));
    }
    Ok(())
}

fn clear_json_files(directory: &Path) -> Result<(), CoreError> {
    let entries =
        fs::read_dir(directory).map_err(|error| codec_error(directory, "directory.read", error))?;
    for entry in entries {
        let entry = entry.map_err(|error| codec_error(directory, "directory.read", error))?;
        if entry
            .path()
            .extension()
            .and_then(|extension| extension.to_str())
            == Some("json")
        {
            fs::remove_file(entry.path())
                .map_err(|error| codec_error(directory, "json.remove", error))?;
        }
    }
    Ok(())
}

fn read_existing_plugin_state(path: &Path) -> Result<Option<PluginStateFile>, CoreError> {
    if !path.exists() {
        return Ok(None);
    }
    let state: PluginStateFile = read_json(path)?;
    state.validate(path)?;
    Ok(Some(state))
}

fn canonical_json_string(value: &serde_json::Value) -> Result<String, CoreError> {
    let mut bytes = canonical_json_bytes(value)?;
    if bytes.last() == Some(&b'\n') {
        bytes.pop();
    }
    String::from_utf8(bytes).map_err(|error| CoreError::Serialization(error.to_string()))
}

fn digest_file(path: &Path) -> Result<String, CoreError> {
    let bytes = fs::read(path).map_err(|error| codec_error(path, "asset.read", error))?;
    Ok(digest_bytes(&bytes))
}

fn digest_bytes(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("sha256:{:x}", hasher.finalize())
}

pub fn canonical_json_bytes<T: Serialize>(value: &T) -> Result<Vec<u8>, CoreError> {
    let value =
        serde_json::to_value(value).map_err(|error| CoreError::Serialization(error.to_string()))?;
    let value = sort_json(value);
    let mut bytes = serde_json::to_vec_pretty(&value)
        .map_err(|error| CoreError::Serialization(error.to_string()))?;
    bytes.push(b'\n');
    Ok(bytes)
}

pub fn parse_json<T: DeserializeOwned>(path: &Path, bytes: &[u8]) -> Result<T, CoreError> {
    let text = std::str::from_utf8(bytes)
        .map_err(|error| codec_error(path, "json.utf8", error.to_string()))?;
    serde_json::from_str(text).map_err(|error| codec_error(path, "json.syntax", error.to_string()))
}

pub fn read_json<T: DeserializeOwned>(path: &Path) -> Result<T, CoreError> {
    let bytes =
        std::fs::read(path).map_err(|error| codec_error(path, "json.read", error.to_string()))?;
    parse_json(path, &bytes)
}

pub fn write_json<T: Serialize>(path: &Path, value: &T) -> Result<(), CoreError> {
    let bytes = canonical_json_bytes(value)?;
    std::fs::write(path, bytes).map_err(|error| codec_error(path, "json.write", error.to_string()))
}

pub fn canonical_markdown(body: &str) -> String {
    let mut normalized = body.replace("\r\n", "\n").replace('\r', "\n");
    if !normalized.ends_with('\n') {
        normalized.push('\n');
    }
    normalized
}

pub fn canonical_markdown_bytes(path: &Path, bytes: &[u8]) -> Result<Vec<u8>, CoreError> {
    let body = std::str::from_utf8(bytes)
        .map_err(|error| codec_error(path, "markdown.utf8", error.to_string()))?;
    Ok(canonical_markdown(body).into_bytes())
}

fn sort_json(value: serde_json::Value) -> serde_json::Value {
    match value {
        serde_json::Value::Object(object) => {
            let mut sorted = BTreeMap::new();
            for (key, value) in object {
                sorted.insert(key, sort_json(value));
            }
            serde_json::Value::Object(sorted.into_iter().collect())
        }
        serde_json::Value::Array(values) => {
            serde_json::Value::Array(values.into_iter().map(sort_json).collect())
        }
        value => value,
    }
}

fn validate_uuid(path: &Path, code: &str, value: &str) -> Result<(), CoreError> {
    let uuid =
        Uuid::parse_str(value).map_err(|error| codec_error(path, code, error.to_string()))?;
    if uuid.to_string() != value {
        return Err(codec_error(
            path,
            code,
            "UUID must use lowercase canonical form",
        ));
    }
    Ok(())
}

fn codec_error(path: &Path, code: &str, detail: impl std::fmt::Display) -> CoreError {
    CoreError::Validation(format!("{} [{code}] {detail}", path.display()))
}

fn is_ignored_metadata_entry(name: &str) -> bool {
    name.eq_ignore_ascii_case(".ds_store")
        || name.eq_ignore_ascii_case("thumbs.db")
        || name.eq_ignore_ascii_case("desktop.ini")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn project_manifest_is_byte_stable_and_lexicographically_ordered() {
        let manifest = ProjectManifest {
            format_version: 2,
            id: "6f21a771-eec6-4833-9a56-89b5cfc8f126".into(),
            name: "Eldermere".into(),
            created_at: "2026-08-05T10:30:00Z".into(),
        };

        assert_eq!(
            canonical_json_bytes(&manifest).unwrap(),
            br#"{
  "createdAt": "2026-08-05T10:30:00Z",
  "formatVersion": 2,
  "id": "6f21a771-eec6-4833-9a56-89b5cfc8f126",
  "name": "Eldermere"
}
"#
        );
    }

    #[test]
    fn manifest_rejects_unknown_fields_and_duplicate_keys() {
        let path = Path::new("project.json");
        assert!(parse_json::<ProjectManifest>(path, br#"{"formatVersion":2,"id":"6f21a771-eec6-4833-9a56-89b5cfc8f126","name":"E","createdAt":"now","extra":true}"#).is_err());
        assert!(parse_json::<ProjectManifest>(path, br#"{"formatVersion":2,"formatVersion":2,"id":"6f21a771-eec6-4833-9a56-89b5cfc8f126","name":"E","createdAt":"now"}"#).is_err());
    }

    #[test]
    fn entity_rejects_document_traversal() {
        let entity = EntityFile {
            id: "018f89df-b93e-7ad0-a07f-08b1441d1550".into(),
            name: "The Glass Coast".into(),
            entity_type: Some("place".into()),
            deleted: false,
            created_at: "now".into(),
            updated_at: "now".into(),
            document: Some(EntityDocumentRef {
                id: "018f89e1-3d7b-73bb-b7c1-c83de04102e1".into(),
                path: "../document.md".into(),
            }),
        };
        let error = entity
            .validate(Path::new("entities/id/entity.json"))
            .unwrap_err();
        assert!(error.to_string().contains("[entity.document-path]"));
    }

    #[test]
    fn markdown_normalizes_line_endings_without_changing_canonical_bytes() {
        assert_eq!(
            canonical_markdown("# Title\r\n\r\nBody\n"),
            "# Title\n\nBody\n"
        );
        assert_eq!(
            canonical_markdown_bytes(Path::new("document.md"), b"Body\n").unwrap(),
            b"Body\n"
        );
    }

    #[test]
    fn json_round_trips_through_a_canonical_file() {
        let root = std::env::temp_dir().join(format!("daena-codec-{}", Uuid::new_v4()));
        fs::create_dir_all(&root).unwrap();
        let path = root.join("project.json");
        let manifest = ProjectManifest::new("Test");
        write_json(&path, &manifest).unwrap();
        assert_eq!(read_json::<ProjectManifest>(&path).unwrap(), manifest);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn canonical_scan_ignores_os_metadata_entries() {
        let root = std::env::temp_dir().join(format!("daena-metadata-{}", Uuid::new_v4()));
        fs::create_dir_all(root.join("entities")).unwrap();
        fs::create_dir_all(root.join("plugins")).unwrap();
        fs::write(root.join("entities/.DS_Store"), b"metadata").unwrap();
        fs::write(root.join("plugins/Thumbs.db"), b"metadata").unwrap();
        write_json(&root.join("project.json"), &ProjectManifest::new("Test")).unwrap();

        let canonical = read_canonical_project(&root).unwrap();
        assert!(canonical.snapshot.entities.is_empty());
        assert!(canonical.snapshot.modules.is_empty());
        fs::remove_dir_all(root).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn repository_rejects_symlinked_canonical_directories_before_creation() {
        let root = std::env::temp_dir().join(format!("daena-symlink-root-{}", Uuid::new_v4()));
        let outside = std::env::temp_dir().join(format!("daena-symlink-target-{}", Uuid::new_v4()));
        fs::create_dir_all(&root).unwrap();
        fs::create_dir_all(&outside).unwrap();
        std::os::unix::fs::symlink(&outside, root.join("assets")).unwrap();

        let error = FilesystemRepository::open(&root).unwrap_err();
        assert!(error.to_string().contains("[path.symlink]"));
        assert!(!outside.join("images").exists());

        fs::remove_dir_all(root).unwrap();
        fs::remove_dir_all(outside).unwrap();
    }
}
