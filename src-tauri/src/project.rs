use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::io::{BufReader, Read};
use std::path::{Path, PathBuf};
use std::process::Command;
use uuid::Uuid;

#[derive(Debug)]
pub enum ProjectError {
    Sql(rusqlite::Error),
    NotFound(String),
}

impl std::fmt::Display for ProjectError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Sql(error) => write!(f, "database error: {error}"),
            Self::NotFound(message) => f.write_str(message),
        }
    }
}

impl std::error::Error for ProjectError {}
impl From<rusqlite::Error> for ProjectError {
    fn from(error: rusqlite::Error) -> Self {
        Self::Sql(error)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateEntity {
    pub name: String,
    pub entity_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entity {
    pub id: String,
    pub name: String,
    pub entity_type: Option<String>,
    pub deleted: bool,
    pub created_at: String,
    pub updated_at: String,
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
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldValue {
    pub entity_id: String,
    pub namespace: String,
    pub key: String,
    pub value: serde_json::Value,
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
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleState {
    pub module_id: String,
    pub enabled: bool,
    pub version: i64,
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
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectInfo {
    pub name: String,
    pub root: String,
    pub database: String,
    pub assets: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitStatus {
    pub repository: bool,
    pub branch: Option<String>,
    pub changes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitLogEntry {
    pub hash: String,
    pub date: String,
    pub subject: String,
}

fn current_snapshot_version() -> u32 {
    1
}

fn decode_field_value(value: String) -> serde_json::Value {
    serde_json::from_str(&value).unwrap_or(serde_json::Value::String(value))
}

fn encode_field_value(value: &serde_json::Value) -> Result<String, ProjectError> {
    serde_json::to_string(value).map_err(|error| ProjectError::NotFound(error.to_string()))
}

pub struct ProjectStore {
    connection: Connection,
    root: Option<PathBuf>,
}

impl ProjectStore {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, ProjectError> {
        let path = path.as_ref();
        if path.is_dir() {
            return Self::open_directory(path);
        }
        let root = path.parent().map(Path::to_path_buf);
        Self::open_database(path, root)
    }

    pub fn open_directory(path: impl AsRef<Path>) -> Result<Self, ProjectError> {
        let root = path.as_ref();
        std::fs::create_dir_all(root).map_err(|error| ProjectError::NotFound(error.to_string()))?;
        std::fs::create_dir_all(root.join("assets/images"))
            .map_err(|error| ProjectError::NotFound(error.to_string()))?;
        std::fs::create_dir_all(root.join("assets/videos"))
            .map_err(|error| ProjectError::NotFound(error.to_string()))?;
        std::fs::create_dir_all(root.join("assets/maps"))
            .map_err(|error| ProjectError::NotFound(error.to_string()))?;
        std::fs::create_dir_all(root.join("assets/files"))
            .map_err(|error| ProjectError::NotFound(error.to_string()))?;
        let metadata_path = root.join("project.json");
        if !metadata_path.exists() {
            let name = root
                .file_name()
                .and_then(|value| value.to_str())
                .unwrap_or("Worldbuilder project");
            let metadata = ProjectInfo {
                name: name.to_string(),
                root: ".".into(),
                database: "worldbuilder.sqlite".into(),
                assets: "assets".into(),
            };
            let content = serde_json::to_string_pretty(&metadata)
                .map_err(|error| ProjectError::NotFound(error.to_string()))?;
            std::fs::write(&metadata_path, format!("{content}\n"))
                .map_err(|error| ProjectError::NotFound(error.to_string()))?;
        }
        let gitignore = root.join(".gitignore");
        if !gitignore.exists() {
            std::fs::write(
                &gitignore,
                "worldbuilder.sqlite-wal\nworldbuilder.sqlite-shm\nworldbuilder.sqlite-journal\n",
            )
            .map_err(|error| ProjectError::NotFound(error.to_string()))?;
        }
        Self::open_database(root.join("worldbuilder.sqlite"), Some(root.to_path_buf()))
    }

    fn open_database(path: impl AsRef<Path>, root: Option<PathBuf>) -> Result<Self, ProjectError> {
        let connection = Connection::open(path)?;
        connection.pragma_update(None, "foreign_keys", true)?;
        let store = Self { connection, root };
        store.initialize()?;
        Ok(store)
    }

    pub fn in_memory() -> Result<Self, ProjectError> {
        Self::open_database(":memory:", None)
    }

    pub fn info(&self) -> Option<ProjectInfo> {
        let root = self.root.as_ref()?;
        Some(ProjectInfo {
            name: root
                .file_name()
                .and_then(|value| value.to_str())
                .unwrap_or("Worldbuilder project")
                .to_string(),
            root: root.to_string_lossy().to_string(),
            database: root
                .join("worldbuilder.sqlite")
                .to_string_lossy()
                .to_string(),
            assets: root.join("assets").to_string_lossy().to_string(),
        })
    }

    fn project_root(&self) -> Result<&Path, ProjectError> {
        self.root
            .as_deref()
            .ok_or_else(|| ProjectError::NotFound("project is not directory-backed".into()))
    }

    fn run_git(&self, args: &[&str]) -> Result<std::process::Output, ProjectError> {
        let root = self.project_root()?;
        Command::new("git")
            .args(args)
            .current_dir(root)
            .output()
            .map_err(|error| ProjectError::NotFound(format!("git is unavailable: {error}")))
    }

    pub fn git_status(&self) -> Result<GitStatus, ProjectError> {
        let repository = self.run_git(&["rev-parse", "--is-inside-work-tree"])?;
        if !repository.status.success() {
            return Ok(GitStatus {
                repository: false,
                branch: None,
                changes: Vec::new(),
            });
        }
        let branch = self.run_git(&["branch", "--show-current"])?;
        let changes = self
            .run_git(&["status", "--short"])?
            .stdout
            .split(|byte| *byte == b'\n')
            .filter(|line| !line.is_empty())
            .map(|line| String::from_utf8_lossy(line).to_string())
            .collect();
        Ok(GitStatus {
            repository: true,
            branch: Some(String::from_utf8_lossy(&branch.stdout).trim().to_string()),
            changes,
        })
    }

    pub fn git_init(&self) -> Result<GitStatus, ProjectError> {
        let output = self.run_git(&["init"])?;
        if !output.status.success() {
            return Err(ProjectError::NotFound(
                String::from_utf8_lossy(&output.stderr).trim().into(),
            ));
        }
        self.git_status()
    }

    pub fn git_log(&self) -> Result<Vec<GitLogEntry>, ProjectError> {
        if !self.git_status()?.repository {
            return Ok(Vec::new());
        }
        let output = self.run_git(&[
            "log",
            "-20",
            "--date=short",
            "--pretty=format:%h%x09%ad%x09%s",
        ])?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            if stderr.contains("does not have any commits yet") {
                return Ok(Vec::new());
            }
            return Err(ProjectError::NotFound(stderr.trim().into()));
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

    pub fn git_commit(&self, message: String) -> Result<GitStatus, ProjectError> {
        if message.trim().is_empty() {
            return Err(ProjectError::NotFound(
                "commit message cannot be empty".into(),
            ));
        }
        if !self.git_status()?.repository {
            return Err(ProjectError::NotFound(
                "project is not a git repository".into(),
            ));
        }
        let add = self.run_git(&["add", "--all"])?;
        if !add.status.success() {
            return Err(ProjectError::NotFound(
                String::from_utf8_lossy(&add.stderr).trim().into(),
            ));
        }
        let commit = self.run_git(&["commit", "-m", message.trim()])?;
        if !commit.status.success() {
            return Err(ProjectError::NotFound(
                String::from_utf8_lossy(&commit.stderr).trim().into(),
            ));
        }
        self.git_status()
    }

    fn initialize(&self) -> Result<(), ProjectError> {
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
             CREATE INDEX IF NOT EXISTS relationships_target_idx ON relationships(target_id);"
        )?;
        self.connection.execute_batch("CREATE TABLE IF NOT EXISTS module_versions(module_id TEXT PRIMARY KEY, version INTEGER NOT NULL DEFAULT 0);
              CREATE TABLE IF NOT EXISTS module_state(module_id TEXT PRIMARY KEY, enabled INTEGER NOT NULL DEFAULT 1);
              CREATE TABLE IF NOT EXISTS module_namespaces(module_id TEXT NOT NULL, namespace TEXT NOT NULL, PRIMARY KEY(module_id, namespace));
              CREATE TABLE IF NOT EXISTS module_fields(module_id TEXT NOT NULL, namespace TEXT NOT NULL, key TEXT NOT NULL, field_type TEXT NOT NULL, required INTEGER NOT NULL, PRIMARY KEY(module_id, namespace, key));
              CREATE TABLE IF NOT EXISTS entity_fields(entity_id TEXT NOT NULL REFERENCES entities(id), namespace TEXT NOT NULL, key TEXT NOT NULL, value TEXT NOT NULL, PRIMARY KEY(entity_id, namespace, key));
             CREATE TABLE IF NOT EXISTS assets (id TEXT PRIMARY KEY, entity_id TEXT NOT NULL REFERENCES entities(id), namespace TEXT NOT NULL, filename TEXT NOT NULL, content_hash TEXT NOT NULL, size INTEGER NOT NULL, mime_type TEXT NOT NULL, path TEXT NOT NULL, created_at TEXT NOT NULL);")?;
        self.connection.execute_batch("CREATE VIRTUAL TABLE IF NOT EXISTS world_search USING fts5(entity_id UNINDEXED, content);
             CREATE TRIGGER IF NOT EXISTS entities_search_insert AFTER INSERT ON entities BEGIN INSERT INTO world_search(entity_id, content) VALUES (new.id, new.name || ' ' || COALESCE(new.entity_type, '')); END;
             CREATE TRIGGER IF NOT EXISTS documents_search_insert AFTER INSERT ON documents BEGIN INSERT INTO world_search(entity_id, content) VALUES (new.entity_id, new.body); END;")?;
        self.connection.execute_batch("CREATE TABLE IF NOT EXISTS migration_history(module_id TEXT NOT NULL, migration_id TEXT NOT NULL, from_version INTEGER NOT NULL, to_version INTEGER NOT NULL, checksum TEXT NOT NULL, PRIMARY KEY(module_id, migration_id));")?;
        Ok(())
    }

    pub fn create_entity(&self, input: CreateEntity) -> Result<Entity, ProjectError> {
        if input.name.trim().is_empty() {
            return Err(ProjectError::NotFound("entity name cannot be empty".into()));
        }
        let entity_type = input.entity_type.map(|value| value.trim().to_owned());
        let id = Uuid::new_v4().to_string();
        let now = chrono_like_now();
        self.connection.execute(
            "INSERT INTO entities(id,name,entity_type,created_at,updated_at) VALUES (?1,?2,?3,?4,?4)",
            params![id, input.name.trim(), entity_type, now],
        )?;
        self.rebuild_search()?;
        Ok(Entity {
            id,
            name: input.name.trim().into(),
            entity_type,
            deleted: false,
            created_at: now.clone(),
            updated_at: now,
        })
    }

    pub fn list_entities(&self) -> Result<Vec<Entity>, ProjectError> {
        self.list_entities_where("WHERE deleted=0")
    }

    fn list_all_entities(&self) -> Result<Vec<Entity>, ProjectError> {
        self.list_entities_where("")
    }

    fn list_entities_where(&self, predicate: &str) -> Result<Vec<Entity>, ProjectError> {
        let mut statement = self.connection.prepare(&format!("SELECT id,name,entity_type,deleted,created_at,updated_at FROM entities {predicate} ORDER BY name"))?;
        let rows = statement.query_map([], |row| {
            Ok(Entity {
                id: row.get(0)?,
                name: row.get(1)?,
                entity_type: row.get(2)?,
                deleted: row.get::<_, i64>(3)? != 0,
                created_at: row.get(4)?,
                updated_at: row.get(5)?,
            })
        })?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    pub fn update_entity(
        &self,
        id: String,
        name: Option<String>,
        entity_type: Option<String>,
    ) -> Result<Entity, ProjectError> {
        if let Some(value) = &name {
            if value.trim().is_empty() {
                return Err(ProjectError::NotFound("entity name cannot be empty".into()));
            }
        }
        let now = chrono_like_now();
        if self.connection.execute("UPDATE entities SET name=COALESCE(?2,name), entity_type=COALESCE(?3,entity_type), updated_at=?4 WHERE id=?1 AND deleted=0", params![id, name, entity_type, now])? == 0 { return Err(ProjectError::NotFound("entity not found".into())); }
        self.rebuild_search()?;
        self.connection.query_row("SELECT id,name,entity_type,deleted,created_at,updated_at FROM entities WHERE id=?1", params![id], |row| Ok(Entity { id: row.get(0)?, name: row.get(1)?, entity_type: row.get(2)?, deleted: row.get::<_, i64>(3)? != 0, created_at: row.get(4)?, updated_at: row.get(5)? })).map_err(Into::into)
    }

    pub fn delete_entity(&self, id: String) -> Result<(), ProjectError> {
        if self.connection.execute(
            "UPDATE entities SET deleted=1, updated_at=?2 WHERE id=?1 AND deleted=0",
            params![id, chrono_like_now()],
        )? == 0
        {
            return Err(ProjectError::NotFound("entity not found".into()));
        }
        self.rebuild_search()?;
        Ok(())
    }

    pub fn save_document(&self, input: SaveDocument) -> Result<(), ProjectError> {
        self.save_entry(SaveEntry {
            document: input,
            fields: Vec::new(),
        })
    }

    pub fn save_entry(&self, input: SaveEntry) -> Result<(), ProjectError> {
        let document = input.document;
        let format = document.format.unwrap_or_else(|| "markdown".into());
        if format != "markdown" && format != "plain-text" && format != "rich-text" {
            return Err(ProjectError::NotFound("unsupported document format".into()));
        }
        let exists: Option<String> = self
            .connection
            .query_row(
                "SELECT id FROM entities WHERE id=?1 AND deleted=0",
                params![document.entity_id],
                |row| row.get(0),
            )
            .optional()?;
        if exists.is_none() {
            return Err(ProjectError::NotFound("entity not found".into()));
        }
        let encoded_fields = input
            .fields
            .iter()
            .map(|field| {
                if field.entity_id != document.entity_id {
                    return Err(ProjectError::NotFound(
                        "field entity does not match document entity".into(),
                    ));
                }
                if field.namespace.trim().is_empty() || field.key.trim().is_empty() {
                    return Err(ProjectError::NotFound(
                        "field namespace and key are required".into(),
                    ));
                }
                Ok((field, encode_field_value(&field.value)?))
            })
            .collect::<Result<Vec<_>, ProjectError>>()?;
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
        Ok(())
    }

    pub fn list_documents(&self, entity_id: String) -> Result<Vec<Document>, ProjectError> {
        let mut statement = self.connection.prepare("SELECT id,entity_id,format,body,updated_at FROM documents WHERE entity_id=?1 ORDER BY updated_at DESC")?;
        let rows = statement.query_map(params![entity_id], |row| {
            Ok(Document {
                id: row.get(0)?,
                entity_id: row.get(1)?,
                format: row.get(2)?,
                body: row.get(3)?,
                updated_at: row.get(4)?,
            })
        })?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }
    pub fn set_field(&self, field: FieldValue) -> Result<(), ProjectError> {
        let exists: Option<String> = self
            .connection
            .query_row(
                "SELECT id FROM entities WHERE id=?1 AND deleted=0",
                params![field.entity_id],
                |row| row.get(0),
            )
            .optional()?;
        if exists.is_none() {
            return Err(ProjectError::NotFound("entity not found".into()));
        }
        if field.namespace.trim().is_empty() || field.key.trim().is_empty() {
            return Err(ProjectError::NotFound(
                "field namespace and key are required".into(),
            ));
        }
        let value = encode_field_value(&field.value)?;
        self.connection.execute("INSERT INTO entity_fields(entity_id,namespace,key,value) VALUES (?1,?2,?3,?4) ON CONFLICT(entity_id,namespace,key) DO UPDATE SET value=excluded.value", params![field.entity_id, field.namespace, field.key, value])?;
        Ok(())
    }
    pub fn list_fields(&self, entity_id: String) -> Result<Vec<FieldValue>, ProjectError> {
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
            })
        })?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    pub fn create_relationship(
        &self,
        input: RelationshipInput,
    ) -> Result<Relationship, ProjectError> {
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
                return Err(ProjectError::NotFound(
                    "relationship entity not found".into(),
                ));
            }
        }
        if input.relationship_type.trim().is_empty() {
            return Err(ProjectError::NotFound(
                "relationship type cannot be empty".into(),
            ));
        }
        let id = Uuid::new_v4().to_string();
        let metadata = input.metadata.unwrap_or_else(|| "{}".into());
        self.connection.execute("INSERT INTO relationships(id,source_id,target_id,relationship_type,metadata) VALUES (?1,?2,?3,?4,?5)", params![id, input.source_id, input.target_id, input.relationship_type, metadata])?;
        Ok(Relationship {
            id,
            source_id: input.source_id,
            target_id: input.target_id,
            relationship_type: input.relationship_type,
            metadata,
        })
    }

    pub fn list_relationships(&self, entity_id: String) -> Result<Vec<Relationship>, ProjectError> {
        let mut statement = self.connection.prepare("SELECT id,source_id,target_id,relationship_type,metadata FROM relationships WHERE source_id=?1 OR target_id=?1")?;
        let rows = statement.query_map(params![entity_id], |row| {
            Ok(Relationship {
                id: row.get(0)?,
                source_id: row.get(1)?,
                target_id: row.get(2)?,
                relationship_type: row.get(3)?,
                metadata: row.get(4)?,
            })
        })?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    pub fn search(&self, query: String) -> Result<Vec<Entity>, ProjectError> {
        if query.trim().is_empty() {
            return self.list_entities();
        }
        let terms = query
            .split_whitespace()
            .map(|term| format!("\"{}\"", term.replace('"', "")))
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
            })
        })?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    pub fn export_json(&self) -> Result<String, ProjectError> {
        let entities = self.list_all_entities()?;
        let mut documents = Vec::new();
        let mut fields = Vec::new();
        let mut assets = Vec::new();
        let mut relationships = Vec::new();
        for entity in &entities {
            documents.extend(self.list_documents(entity.id.clone())?);
            fields.extend(self.list_fields(entity.id.clone())?);
            assets.extend(self.list_assets(entity.id.clone())?);
            relationships.extend(self.list_relationships(entity.id.clone())?);
        }
        relationships.sort_by(|a, b| a.id.cmp(&b.id));
        relationships.dedup_by(|a, b| a.id == b.id);
        let mut module_statement = self.connection.prepare("SELECT m.module_id, COALESCE(s.enabled, 1), m.version FROM module_versions m LEFT JOIN module_state s ON s.module_id = m.module_id ORDER BY m.module_id")?;
        let modules = module_statement
            .query_map([], |row| {
                Ok(ModuleState {
                    module_id: row.get(0)?,
                    enabled: row.get::<_, i64>(1)? != 0,
                    version: row.get(2)?,
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
        })
        .map_err(|error| ProjectError::NotFound(error.to_string()))
    }

    fn import_json_with_mode(&self, payload: &str, replace: bool) -> Result<usize, ProjectError> {
        let snapshot: ProjectSnapshot = serde_json::from_str(payload)
            .map_err(|error| ProjectError::NotFound(error.to_string()))?;
        if snapshot.format_version != current_snapshot_version() {
            return Err(ProjectError::NotFound(format!(
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
                 DELETE FROM module_versions;",
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
        }
        transaction.commit()?;
        self.rebuild_search()?;
        Ok(snapshot.entities.len())
    }

    pub fn register_asset(&self, input: AssetInput) -> Result<Asset, ProjectError> {
        let exists: Option<String> = self
            .connection
            .query_row(
                "SELECT id FROM entities WHERE id=?1 AND deleted=0",
                params![input.entity_id],
                |row| row.get(0),
            )
            .optional()?;
        if exists.is_none() {
            return Err(ProjectError::NotFound("entity not found".into()));
        }
        let id = Uuid::new_v4().to_string();
        let now = chrono_like_now();
        self.connection.execute(
            "INSERT INTO assets(id,entity_id,namespace,filename,content_hash,size,mime_type,path,created_at) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9)",
            params![id, input.entity_id, input.namespace, input.filename, input.content_hash, input.size, input.mime_type, input.path, now],
        )?;
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
        })
    }

    pub fn register_asset_file(&self, input: AssetFileInput) -> Result<Asset, ProjectError> {
        let root = self.project_root()?.to_path_buf();
        let source = Path::new(&input.source_path);
        let metadata =
            std::fs::metadata(source).map_err(|error| ProjectError::NotFound(error.to_string()))?;
        if !metadata.is_file() {
            return Err(ProjectError::NotFound("asset source is not a file".into()));
        }
        let filename = Path::new(&input.filename)
            .file_name()
            .and_then(|value| value.to_str())
            .filter(|value| !value.is_empty() && *value != "." && *value != "..")
            .ok_or_else(|| ProjectError::NotFound("asset filename is invalid".into()))?;
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
        let destination_dir = root.join("assets").join(category);
        std::fs::create_dir_all(&destination_dir)
            .map_err(|error| ProjectError::NotFound(error.to_string()))?;
        let destination = destination_dir.join(format!("{}-{}", Uuid::new_v4(), filename));
        std::fs::copy(source, &destination)
            .map_err(|error| ProjectError::NotFound(error.to_string()))?;
        let mut reader = BufReader::new(
            std::fs::File::open(source)
                .map_err(|error| ProjectError::NotFound(error.to_string()))?,
        );
        let mut hasher = Sha256::new();
        let mut buffer = [0_u8; 64 * 1024];
        loop {
            let count = reader
                .read(&mut buffer)
                .map_err(|error| ProjectError::NotFound(error.to_string()))?;
            if count == 0 {
                break;
            }
            hasher.update(&buffer[..count]);
        }
        let relative_path = destination
            .strip_prefix(root)
            .unwrap_or(&destination)
            .to_string_lossy()
            .replace('\\', "/");
        self.register_asset(AssetInput {
            entity_id: input.entity_id,
            namespace: input.namespace,
            filename: filename.into(),
            content_hash: format!("sha256:{:x}", hasher.finalize()),
            size: metadata.len() as i64,
            mime_type: input.mime_type,
            path: relative_path,
        })
    }

    pub fn list_assets(&self, entity_id: String) -> Result<Vec<Asset>, ProjectError> {
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
            })
        })?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    pub fn backup(&self) -> Result<String, ProjectError> {
        self.backup_to(std::env::temp_dir())
    }

    pub fn backup_to(&self, dir: impl AsRef<Path>) -> Result<String, ProjectError> {
        let export = self.export_json()?;
        let dir = dir.as_ref();
        std::fs::create_dir_all(dir).map_err(|e| ProjectError::NotFound(e.to_string()))?;
        let timestamp = chrono_like_now();
        let filename = format!("worldbuilder-backup-{}-{}.json", timestamp, Uuid::new_v4());
        let path = dir.join(&filename);
        std::fs::write(&path, export).map_err(|e| ProjectError::NotFound(e.to_string()))?;
        Ok(path.to_string_lossy().to_string())
    }

    pub fn restore(&self, path: String) -> Result<(), ProjectError> {
        let content =
            std::fs::read_to_string(&path).map_err(|e| ProjectError::NotFound(e.to_string()))?;
        self.restore_payload(&content)?;
        Ok(())
    }

    pub fn restore_payload(&self, payload: &str) -> Result<(), ProjectError> {
        self.import_json_with_mode(payload, true)?;
        Ok(())
    }

    pub fn rebuild_search(&self) -> Result<(), ProjectError> {
        self.connection.execute_batch(
            "DROP TRIGGER IF EXISTS entities_search_insert;
             DROP TRIGGER IF EXISTS entities_search_update;
             DROP TRIGGER IF EXISTS documents_search_insert;
             DROP TRIGGER IF EXISTS documents_search_update;
             DROP TABLE IF EXISTS world_search;
             CREATE VIRTUAL TABLE world_search USING fts5(entity_id UNINDEXED, content);
             INSERT INTO world_search(entity_id, content) SELECT id, name || ' ' || COALESCE(entity_type, '') FROM entities WHERE deleted=0;
             INSERT INTO world_search(entity_id, content) SELECT entity_id, body FROM documents;
             CREATE TRIGGER entities_search_insert AFTER INSERT ON entities BEGIN INSERT INTO world_search(entity_id, content) SELECT new.id, new.name || ' ' || COALESCE(new.entity_type, '') WHERE new.deleted=0; END;
             CREATE TRIGGER entities_search_update AFTER UPDATE OF name, entity_type, deleted ON entities BEGIN DELETE FROM world_search WHERE entity_id = old.id; INSERT INTO world_search(entity_id, content) SELECT new.id, new.name || ' ' || COALESCE(new.entity_type, '') WHERE new.deleted=0; INSERT INTO world_search(entity_id, content) SELECT entity_id, body FROM documents WHERE entity_id = new.id AND new.deleted=0; END;
             CREATE TRIGGER documents_search_insert AFTER INSERT ON documents BEGIN INSERT INTO world_search(entity_id, content) VALUES (new.entity_id, new.body); END;
             CREATE TRIGGER documents_search_update AFTER UPDATE OF body ON documents BEGIN DELETE FROM world_search WHERE entity_id = old.entity_id; INSERT INTO world_search(entity_id, content) SELECT id, name || ' ' || COALESCE(entity_type, '') FROM entities WHERE id = new.entity_id AND deleted=0; INSERT INTO world_search(entity_id, content) SELECT entity_id, body FROM documents WHERE entity_id = new.entity_id; END;"
        )?;
        Ok(())
    }

    pub fn seed_example(&mut self) -> Result<usize, ProjectError> {
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
        tx.execute("INSERT INTO entity_fields(entity_id,namespace,key,value) VALUES (?1,'lore','summary',?2)", params![eldermere_id, "Ancient coastal fortress and seat of power"])?;
        tx.execute("INSERT INTO entity_fields(entity_id,namespace,key,value) VALUES (?1,'lore','aliases',?2)", params![eldermere_id, "The Elder Hold, The Clifftop"])?;
        tx.execute("INSERT INTO entity_fields(entity_id,namespace,key,value) VALUES (?1,'lore','summary',?2)", params![lord_ashford_id, "Ruler of Eldermere and keeper of the realm"])?;
        tx.execute("INSERT INTO entity_fields(entity_id,namespace,key,value) VALUES (?1,'lore','aliases',?2)", params![lord_ashford_id, "Ash, The Lord"])?;
        tx.execute("INSERT INTO entity_fields(entity_id,namespace,key,value) VALUES (?1,'timeline','startsAt',?2)", params![founding_id, "0001-01-01"])?;
        tx.execute("INSERT INTO entity_fields(entity_id,namespace,key,value) VALUES (?1,'timeline','startsAt',?2)", params![treaty_id, "0042-03-15"])?;
        tx.execute("INSERT INTO entity_fields(entity_id,namespace,key,value) VALUES (?1,'timeline','startsAt',?2)", params![rebellion_id, "0067-07-22"])?;
        tx.execute("INSERT INTO relationships(id,source_id,target_id,relationship_type,metadata) VALUES (?1,?2,?3,?4,?5)", params![Uuid::new_v4().to_string(), lord_ashford_id, eldermere_id, "ruler_of", "{\"since\":\"founding\"}"])?;
        tx.execute("INSERT INTO relationships(id,source_id,target_id,relationship_type,metadata) VALUES (?1,?2,?3,?4,?5)", params![Uuid::new_v4().to_string(), silver_hand_id, lord_ashford_id, "sworn_to", "{}"])?;
        tx.execute("INSERT INTO relationships(id,source_id,target_id,relationship_type,metadata) VALUES (?1,?2,?3,?4,?5)", params![Uuid::new_v4().to_string(), amulet_id, highland_id, "created_by", "{}"])?;
        tx.execute("INSERT INTO relationships(id,source_id,target_id,relationship_type,metadata) VALUES (?1,?2,?3,?4,?5)", params![Uuid::new_v4().to_string(), founding_id, eldermere_id, "founded", "{}"])?;
        tx.execute("INSERT INTO relationships(id,source_id,target_id,relationship_type,metadata) VALUES (?1,?2,?3,?4,?5)", params![Uuid::new_v4().to_string(), treaty_id, founding_id, "follows", "{}"])?;
        tx.execute("INSERT INTO relationships(id,source_id,target_id,relationship_type,metadata) VALUES (?1,?2,?3,?4,?5)", params![Uuid::new_v4().to_string(), rebellion_id, founding_id, "challenges", "{}"])?;
        tx.execute("INSERT INTO relationships(id,source_id,target_id,relationship_type,metadata) VALUES (?1,?2,?3,?4,?5)", params![Uuid::new_v4().to_string(), rebellion_id, lord_ashford_id, "against", "{}"])?;
        tx.execute("INSERT OR IGNORE INTO module_versions(module_id,version) VALUES ('worldbuilder.lore',1), ('worldbuilder.timeline',1)", [])?;
        tx.execute("INSERT OR IGNORE INTO module_namespaces(module_id,namespace) VALUES ('worldbuilder.lore','lore'), ('worldbuilder.timeline','timeline')", [])?;
        tx.commit()?;
        self.rebuild_search()?;
        Ok(9)
    }

    pub fn get_module_version(&self, module_id: &str) -> Result<i64, ProjectError> {
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
            .map_err(ProjectError::Sql)
    }

    pub fn validate_migration(
        &self,
        migration: &crate::migrations::Migration,
        current: i64,
    ) -> Result<(), String> {
        crate::migrations::validate(migration, current)
    }

    pub fn set_module_enabled(&self, module_id: String, enabled: bool) -> Result<(), ProjectError> {
        self.connection.execute(
            "INSERT OR IGNORE INTO module_versions(module_id,version) VALUES (?1,0)",
            params![module_id],
        )?;
        self.connection.execute(
            "INSERT INTO module_state(module_id, enabled) VALUES (?1, ?2) ON CONFLICT(module_id) DO UPDATE SET enabled=excluded.enabled",
            params![module_id, enabled as i64],
        )?;
        Ok(())
    }

    pub fn is_module_enabled(&self, module_id: &str) -> Result<bool, ProjectError> {
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

    pub fn apply_migration(
        &mut self,
        migration: &crate::migrations::Migration,
    ) -> Result<(), String> {
        self.backup().map_err(|error| error.to_string())?;
        crate::migrations::apply(&mut self.connection, migration)
    }
}

fn chrono_like_now() -> String {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn creates_entities_and_rejects_empty_names() {
        let store = ProjectStore::in_memory().unwrap();
        let entity = store
            .create_entity(CreateEntity {
                name: "Eldermere".into(),
                entity_type: None,
            })
            .unwrap();
        assert_eq!(store.list_entities().unwrap()[0].id, entity.id);
        assert!(store
            .create_entity(CreateEntity {
                name: "  ".into(),
                entity_type: None
            })
            .is_err());
    }

    #[test]
    fn export_round_trip_preserves_entities_and_documents() {
        let source = ProjectStore::in_memory().unwrap();
        let entity = source
            .create_entity(CreateEntity {
                name: "Ash Court".into(),
                entity_type: Some("faction".into()),
            })
            .unwrap();
        source
            .save_document(SaveDocument {
                entity_id: entity.id.clone(),
                body: "A quiet power.".into(),
                format: Some("markdown".into()),
            })
            .unwrap();
        let target = ProjectStore::in_memory().unwrap();
        let imported = target
            .import_json_with_mode(&source.export_json().unwrap(), false)
            .unwrap();
        assert_eq!(imported, 1);
        assert_eq!(target.list_entities().unwrap()[0].name, "Ash Court");
        assert_eq!(
            target.list_documents(entity.id).unwrap()[0].body,
            "A quiet power."
        );
    }

    #[test]
    fn importing_the_same_snapshot_twice_preserves_children() {
        let source = ProjectStore::in_memory().unwrap();
        let entity = source
            .create_entity(CreateEntity {
                name: "Repeated import".into(),
                entity_type: None,
            })
            .unwrap();
        source
            .save_document(SaveDocument {
                entity_id: entity.id.clone(),
                body: "Content".into(),
                format: Some("plain-text".into()),
            })
            .unwrap();
        let payload = source.export_json().unwrap();
        let target = ProjectStore::in_memory().unwrap();
        target.import_json_with_mode(&payload, false).unwrap();
        target.import_json_with_mode(&payload, false).unwrap();
        assert_eq!(target.list_entities().unwrap().len(), 1);
        assert_eq!(target.list_documents(entity.id).unwrap().len(), 1);
    }

    #[test]
    fn updates_canonical_document_and_preserves_namespaced_fields() {
        let store = ProjectStore::in_memory().unwrap();
        let entity = store
            .create_entity(CreateEntity {
                name: "Harbor".into(),
                entity_type: Some("place".into()),
            })
            .unwrap();
        store
            .save_document(SaveDocument {
                entity_id: entity.id.clone(),
                body: "First".into(),
                format: Some("markdown".into()),
            })
            .unwrap();
        store
            .save_document(SaveDocument {
                entity_id: entity.id.clone(),
                body: "Second".into(),
                format: Some("plain-text".into()),
            })
            .unwrap();
        store
            .set_field(FieldValue {
                entity_id: entity.id.clone(),
                namespace: "lore".into(),
                key: "summary".into(),
                value: serde_json::json!("A port"),
            })
            .unwrap();
        store
            .set_field(FieldValue {
                entity_id: entity.id.clone(),
                namespace: "timeline".into(),
                key: "startsAt".into(),
                value: serde_json::json!("0010-01-01"),
            })
            .unwrap();
        assert_eq!(store.list_documents(entity.id.clone()).unwrap().len(), 1);
        assert_eq!(
            store.list_documents(entity.id.clone()).unwrap()[0].body,
            "Second"
        );
        assert_eq!(store.list_fields(entity.id).unwrap().len(), 2);
    }

    #[test]
    fn typed_fields_round_trip_as_json_values() {
        let store = ProjectStore::in_memory().unwrap();
        let entity = store
            .create_entity(CreateEntity {
                name: "Typed fields".into(),
                entity_type: None,
            })
            .unwrap();
        store
            .set_field(FieldValue {
                entity_id: entity.id.clone(),
                namespace: "test".into(),
                key: "count".into(),
                value: serde_json::json!(42),
            })
            .unwrap();
        store
            .set_field(FieldValue {
                entity_id: entity.id.clone(),
                namespace: "test".into(),
                key: "published".into(),
                value: serde_json::json!(true),
            })
            .unwrap();
        let fields = store.list_fields(entity.id).unwrap();
        assert!(fields
            .iter()
            .any(|field| field.value == serde_json::json!(42)));
        assert!(fields
            .iter()
            .any(|field| field.value == serde_json::json!(true)));
    }

    #[test]
    fn save_entry_rejects_invalid_fields_before_writing_document() {
        let store = ProjectStore::in_memory().unwrap();
        let entity = store
            .create_entity(CreateEntity {
                name: "Atomic entry".into(),
                entity_type: None,
            })
            .unwrap();
        let result = store.save_entry(SaveEntry {
            document: SaveDocument {
                entity_id: entity.id.clone(),
                body: "Should not persist".into(),
                format: Some("plain-text".into()),
            },
            fields: vec![FieldValue {
                entity_id: "different-entity".into(),
                namespace: "test".into(),
                key: "value".into(),
                value: serde_json::json!("invalid"),
            }],
        });
        assert!(result.is_err());
        assert!(store.list_documents(entity.id).unwrap().is_empty());
    }

    #[test]
    fn rename_updates_search_and_relationships_require_live_entities() {
        let store = ProjectStore::in_memory().unwrap();
        let source = store
            .create_entity(CreateEntity {
                name: "Old Name".into(),
                entity_type: None,
            })
            .unwrap();
        let target = store
            .create_entity(CreateEntity {
                name: "Target".into(),
                entity_type: None,
            })
            .unwrap();
        store
            .update_entity(source.id.clone(), Some("New Name".into()), None)
            .unwrap();
        assert!(store.search("Old Name".into()).unwrap().is_empty());
        assert_eq!(store.search("New Name".into()).unwrap()[0].id, source.id);
        store.delete_entity(target.id.clone()).unwrap();
        assert!(store
            .create_relationship(RelationshipInput {
                source_id: source.id,
                target_id: target.id,
                relationship_type: "points_to".into(),
                metadata: None
            })
            .is_err());
    }

    #[test]
    fn assets_and_module_state_survive_export_import() {
        let source = ProjectStore::in_memory().unwrap();
        let entity = source
            .create_entity(CreateEntity {
                name: "Map Room".into(),
                entity_type: Some("place".into()),
            })
            .unwrap();
        let asset = source
            .register_asset(AssetInput {
                entity_id: entity.id.clone(),
                namespace: "lore".into(),
                filename: "map.png".into(),
                content_hash: "abc123".into(),
                size: 42,
                mime_type: "image/png".into(),
                path: "map.png".into(),
            })
            .unwrap();
        source
            .set_module_enabled("worldbuilder.lore".into(), false)
            .unwrap();
        let target = ProjectStore::in_memory().unwrap();
        target
            .import_json_with_mode(&source.export_json().unwrap(), false)
            .unwrap();
        assert_eq!(target.list_assets(entity.id).unwrap()[0].id, asset.id);
        assert!(!target.is_module_enabled("worldbuilder.lore").unwrap());
    }

    #[test]
    fn seed_example_is_repeatable_after_modules_are_initialized() {
        let mut store = ProjectStore::in_memory().unwrap();
        store
            .set_module_enabled("worldbuilder.lore".into(), true)
            .unwrap();
        store
            .set_module_enabled("worldbuilder.timeline".into(), true)
            .unwrap();

        assert_eq!(store.seed_example().unwrap(), 9);
        assert_eq!(store.seed_example().unwrap(), 9);
        assert_eq!(store.list_entities().unwrap().len(), 9);
        assert_eq!(store.search("Highland Culture".into()).unwrap().len(), 1);
    }

    #[test]
    fn restore_replaces_records_missing_from_the_backup() {
        let source = ProjectStore::in_memory().unwrap();
        source
            .create_entity(CreateEntity {
                name: "From backup".into(),
                entity_type: None,
            })
            .unwrap();
        let path =
            std::env::temp_dir().join(format!("worldbuilder-restore-test-{}.json", Uuid::new_v4()));
        std::fs::write(&path, source.export_json().unwrap()).unwrap();

        let target = ProjectStore::in_memory().unwrap();
        target
            .create_entity(CreateEntity {
                name: "Stale record".into(),
                entity_type: None,
            })
            .unwrap();
        target.restore(path.to_string_lossy().into_owned()).unwrap();

        assert_eq!(target.list_entities().unwrap().len(), 1);
        assert_eq!(target.list_entities().unwrap()[0].name, "From backup");
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn applying_migration_creates_backup_and_records_version() {
        let mut store = ProjectStore::in_memory().unwrap();
        let migration = crate::migrations::Migration {
            id: "timeline-v1".into(),
            module_id: "worldbuilder.timeline".into(),
            from: 0,
            to: 1,
            operations: vec![crate::migrations::Operation::CreateNamespace {
                namespace: "timeline".into(),
            }],
            recovery: "backup".into(),
        };
        store.apply_migration(&migration).unwrap();
        assert_eq!(
            store.get_module_version("worldbuilder.timeline").unwrap(),
            1
        );
    }

    #[test]
    fn directory_projects_create_portable_layout() {
        let root = std::env::temp_dir().join(format!("worldbuilder-project-{}", Uuid::new_v4()));
        let store = ProjectStore::open_directory(&root).unwrap();
        assert_eq!(store.info().unwrap().root, root.to_string_lossy());
        assert!(root.join("project.json").is_file());
        assert!(root.join("worldbuilder.sqlite").is_file());
        assert!(root.join("assets/images").is_dir());
        assert!(root.join("assets/videos").is_dir());
        assert!(root.join("assets/maps").is_dir());
        assert!(root.join("assets/files").is_dir());
        drop(store);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn directory_assets_are_copied_and_hashed() {
        let root = std::env::temp_dir().join(format!("worldbuilder-project-{}", Uuid::new_v4()));
        let source =
            std::env::temp_dir().join(format!("worldbuilder-asset-{}.txt", Uuid::new_v4()));
        std::fs::write(&source, b"asset contents").unwrap();
        let store = ProjectStore::open_directory(&root).unwrap();
        let entity = store
            .create_entity(CreateEntity {
                name: "Asset owner".into(),
                entity_type: None,
            })
            .unwrap();
        let asset = store
            .register_asset_file(AssetFileInput {
                entity_id: entity.id,
                namespace: "lore".into(),
                source_path: source.to_string_lossy().into_owned(),
                filename: "notes.txt".into(),
                mime_type: "text/plain".into(),
            })
            .unwrap();
        assert_eq!(
            asset.content_hash,
            "sha256:f64ec9687efc98edc9ed69b2024bb23bcee2ba0a4e52b64ac3ab204f818716d4"
        );
        assert!(asset.path.starts_with("assets/files/"));
        assert!(root.join(&asset.path).is_file());
        drop(store);
        std::fs::remove_file(source).unwrap();
        std::fs::remove_dir_all(root).unwrap();
    }
}
