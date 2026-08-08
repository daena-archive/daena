//! Host-owned Phase 2 plugin authority.
//!
//! This crate deliberately contains no Tauri or runtime implementation.  It
//! owns the facts needed to attribute and authorize a plugin request before a
//! future core service is called.

use daena_plugin_api::{
    command_exposes, lifecycle_transition, parse_manifest, validate_command_value, Command,
    CommandAction, CommandExposure, LifecycleState, NamespaceView, PluginManifest,
    RpcAuthorizationContext, RpcError, RpcRequest, RpcResponse, View, ViewComponent,
    RPC_METHOD_CATALOG, RPC_VERSION,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{mpsc, Arc, Condvar, Mutex};
use std::thread;
use std::time::{Duration, SystemTime};

pub mod package;
pub mod runtime;
pub use package::{
    plan_rollback, plan_upgrade, select_migrations, ArchiveLimits, CapabilityConsent,
    InstalledVersion, MigrationPlan, PackageCatalog, PackageError, PackageSignature, PluginPackage,
    RollbackPlan, UpgradePlan, VerificationPolicy,
};
pub use runtime::{
    plugin_window_label, validate_bridge_request, webview_policy, PluginWebviewPolicy, WasmLimits,
    WasmRuntimeRegistry, BUNDLED_TIMELINE_SERVICE_WASM, WASM_SERVICE_ABI, WASM_SERVICE_MAX_BYTES,
};

static SESSION_COUNTER: AtomicU64 = AtomicU64::new(1);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectPluginUsage {
    pub project_id: String,
    pub plugin_id: String,
    pub version: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct PersistentHostState {
    #[serde(default)]
    packages: PackageCatalog,
    /// Legacy global grants. New writes keep only unmigrated entries; active
    /// project grants live under `{project}/.daena/local/plugin-grants.json`.
    #[serde(default)]
    grants: GrantStore,
    #[serde(default)]
    project_usage: Vec<ProjectPluginUsage>,
}

const PROJECT_GRANTS_FORMAT_VERSION: u32 = 1;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ProjectGrantsFile {
    format_version: u32,
    #[serde(default)]
    grants: Vec<ProjectGrantEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ProjectGrantEntry {
    plugin_id: String,
    grants: BTreeSet<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HostError(pub String);

impl std::fmt::Display for HostError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}
impl std::error::Error for HostError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CatalogEntry {
    pub manifest: PluginManifest,
    pub package_root: PathBuf,
    pub digest: String,
    pub embedded_wasm: Option<Arc<[u8]>>,
}

#[derive(Debug, Default, Clone)]
pub struct PluginCatalog {
    entries: BTreeMap<String, CatalogEntry>,
}

impl PluginCatalog {
    /// Development-directory installation is intentionally the only package
    /// input in Phase 2. ZIP extraction and signatures belong to Phase 6.
    pub fn install_development_dir(
        &mut self,
        root: impl AsRef<Path>,
    ) -> Result<&CatalogEntry, HostError> {
        let root = root.as_ref().canonicalize().map_err(io_error)?;
        let manifest_path = root.join("manifest.json");
        let bytes = fs::read(&manifest_path).map_err(io_error)?;
        let manifest = parse_manifest(
            std::str::from_utf8(&bytes)
                .map_err(|e| HostError(format!("manifest is not UTF-8: {e}")))?,
        )
        .map_err(|e| HostError(e.to_string()))?;
        validate_package_tree(&root, &manifest)?;
        let digest = package_digest(&root)?;
        let id = manifest.id.clone();
        if self.entries.contains_key(&id) {
            return Err(HostError("plugin ID is already installed".into()));
        }
        self.entries.insert(
            id.clone(),
            CatalogEntry {
                manifest,
                package_root: root,
                digest,
                embedded_wasm: None,
            },
        );
        Ok(self.entries.get(&id).expect("inserted catalog entry"))
    }

    pub fn insert_for_test(&mut self, entry: CatalogEntry) -> Result<(), HostError> {
        if self
            .entries
            .insert(entry.manifest.id.clone(), entry)
            .is_some()
        {
            return Err(HostError("plugin ID is already installed".into()));
        }
        Ok(())
    }

    fn replace_runtime_entry(&mut self, entry: CatalogEntry) -> Result<(), HostError> {
        if self
            .entries
            .get(&entry.manifest.id)
            .is_some_and(|existing| existing.package_root.as_os_str().is_empty())
        {
            return Err(HostError(
                "cannot replace a bundled plugin with a runtime package".into(),
            ));
        }
        self.entries.insert(entry.manifest.id.clone(), entry);
        Ok(())
    }

    pub fn register_bundled_json(&mut self, json: &str) -> Result<&CatalogEntry, HostError> {
        self.register_bundled_json_with_wasm(json, None)
    }

    pub fn register_bundled_json_with_wasm(
        &mut self,
        json: &str,
        embedded_wasm: Option<&[u8]>,
    ) -> Result<&CatalogEntry, HostError> {
        let manifest = parse_manifest(json).map_err(|error| HostError(error.to_string()))?;
        let id = manifest.id.clone();
        if self.entries.contains_key(&id) {
            return Err(HostError("plugin ID is already installed".into()));
        }
        let mut hasher = Sha256::new();
        hasher.update(json.as_bytes());
        self.entries.insert(
            id.clone(),
            CatalogEntry {
                manifest,
                package_root: PathBuf::new(),
                digest: hex_digest(&hasher.finalize()),
                embedded_wasm: embedded_wasm.map(Arc::<[u8]>::from),
            },
        );
        Ok(self.entries.get(&id).expect("inserted bundled entry"))
    }

    pub fn get(&self, id: &str) -> Option<&CatalogEntry> {
        self.entries.get(id)
    }
    fn remove(&mut self, id: &str) {
        self.entries.remove(id);
    }
    pub fn list(&self) -> impl Iterator<Item = &CatalogEntry> {
        self.entries.values()
    }
}

fn io_error(error: std::io::Error) -> HostError {
    HostError(error.to_string())
}

fn validate_package_tree(root: &Path, manifest: &PluginManifest) -> Result<(), HostError> {
    let mut seen = BTreeSet::new();
    walk_package(root, root, &mut seen)?;
    for entrypoint in [
        manifest.entrypoints.ui.as_ref(),
        manifest.entrypoints.wasm.as_ref(),
    ]
    .into_iter()
    .flatten()
    {
        let path = root.join(entrypoint);
        if !path.is_file() {
            return Err(HostError(format!(
                "manifest entrypoint is missing: {entrypoint}"
            )));
        }
    }
    Ok(())
}

fn walk_package(root: &Path, current: &Path, seen: &mut BTreeSet<String>) -> Result<(), HostError> {
    for entry in fs::read_dir(current).map_err(io_error)? {
        let entry = entry.map_err(io_error)?;
        let path = entry.path();
        let metadata = fs::symlink_metadata(&path).map_err(io_error)?;
        if metadata.file_type().is_symlink() {
            return Err(HostError(format!(
                "links are not allowed in plugin packages: {}",
                path.display()
            )));
        }
        let relative = path
            .strip_prefix(root)
            .map_err(|e| HostError(e.to_string()))?
            .to_string_lossy()
            .replace('\\', "/");
        if relative
            .split('/')
            .any(|part| part.is_empty() || part == "." || part == "..")
        {
            return Err(HostError(format!("invalid package path: {relative}")));
        }
        let folded = relative.to_ascii_lowercase();
        if !seen.insert(folded) {
            return Err(HostError(format!(
                "case-colliding package path: {relative}"
            )));
        }
        if metadata.is_dir() {
            walk_package(root, &path, seen)?;
        } else if !metadata.is_file() {
            return Err(HostError(format!("unsupported package entry: {relative}")));
        }
    }
    Ok(())
}

/// Hashes sorted relative paths and file bytes, including manifest.json.
pub fn package_digest(root: impl AsRef<Path>) -> Result<String, HostError> {
    let root = root.as_ref();
    let mut files = Vec::new();
    collect_files(root, root, &mut files)?;
    files.sort_by(|a, b| a.0.cmp(&b.0));
    let mut hasher = Sha256::new();
    for (relative, path) in files {
        hasher.update(relative.as_bytes());
        hasher.update([0]);
        hasher.update(fs::read(path).map_err(io_error)?);
        hasher.update([0]);
    }
    Ok(hex_digest(&hasher.finalize()))
}

fn collect_files(
    root: &Path,
    current: &Path,
    files: &mut Vec<(String, PathBuf)>,
) -> Result<(), HostError> {
    for entry in fs::read_dir(current).map_err(io_error)? {
        let path = entry.map_err(io_error)?.path();
        let metadata = fs::symlink_metadata(&path).map_err(io_error)?;
        if metadata.file_type().is_symlink() {
            return Err(HostError("links are not allowed in plugin packages".into()));
        }
        if metadata.is_dir() {
            collect_files(root, &path, files)?;
        } else if metadata.is_file() {
            files.push((
                path.strip_prefix(root)
                    .map_err(|e| HostError(e.to_string()))?
                    .to_string_lossy()
                    .replace('\\', "/"),
                path,
            ));
        } else {
            return Err(HostError("unsupported package entry".into()));
        }
    }
    Ok(())
}

fn hex_digest(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct GrantStore {
    #[serde(default, with = "grant_store_format")]
    grants: BTreeMap<(String, String), BTreeSet<String>>,
}

mod grant_store_format {
    use super::CapabilityGrants;
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use std::collections::{BTreeMap, BTreeSet};

    #[derive(Debug, Serialize, Deserialize)]
    struct Entry {
        project_id: String,
        plugin_id: String,
        grants: BTreeSet<String>,
    }

    #[derive(Debug, Deserialize)]
    #[serde(untagged)]
    enum StoredGrants {
        Entries(Vec<Entry>),
        LegacyEmpty(BTreeMap<String, BTreeSet<String>>),
    }

    pub fn serialize<S>(grants: &CapabilityGrants, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        grants
            .iter()
            .map(|((project_id, plugin_id), capabilities)| Entry {
                project_id: project_id.clone(),
                plugin_id: plugin_id.clone(),
                grants: capabilities.clone(),
            })
            .collect::<Vec<_>>()
            .serialize(serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<CapabilityGrants, D::Error>
    where
        D: Deserializer<'de>,
    {
        let entries = match StoredGrants::deserialize(deserializer)? {
            StoredGrants::Entries(entries) => entries,
            StoredGrants::LegacyEmpty(entries) if entries.is_empty() => Vec::new(),
            StoredGrants::LegacyEmpty(_) => {
                return Err(serde::de::Error::custom(
                    "unsupported legacy capability grant format",
                ));
            }
        };
        let mut grants = BTreeMap::new();
        for entry in entries {
            if grants
                .insert((entry.project_id, entry.plugin_id), entry.grants)
                .is_some()
            {
                return Err(serde::de::Error::custom(
                    "duplicate project/plugin capability grant",
                ));
            }
        }
        Ok(grants)
    }
}

impl GrantStore {
    pub fn set(
        &mut self,
        project_id: &str,
        plugin_id: &str,
        requested: &[String],
        granted: BTreeSet<String>,
    ) -> Result<(), HostError> {
        if granted.iter().any(|grant| {
            !requested
                .iter()
                .any(|request| capability_matches(request, grant))
        }) {
            return Err(HostError("grant is not requested by the manifest".into()));
        }
        self.grants
            .insert((project_id.into(), plugin_id.into()), granted);
        Ok(())
    }
    pub fn get(&self, project_id: &str, plugin_id: &str) -> BTreeSet<String> {
        self.grants
            .get(&(project_id.into(), plugin_id.into()))
            .cloned()
            .unwrap_or_default()
    }

    pub fn is_empty(&self, project_id: &str, plugin_id: &str) -> bool {
        self.get(project_id, plugin_id).is_empty()
    }

    pub fn insert_loaded(&mut self, project_id: &str, plugin_id: &str, granted: BTreeSet<String>) {
        self.grants
            .insert((project_id.into(), plugin_id.into()), granted);
    }

    pub fn clear_project(&mut self, project_id: &str) {
        self.grants
            .retain(|(entry_project, _), _| entry_project != project_id);
    }

    pub fn take_project(&mut self, project_id: &str) -> Vec<(String, BTreeSet<String>)> {
        let keys = self
            .grants
            .keys()
            .filter(|(entry_project, _)| entry_project == project_id)
            .cloned()
            .collect::<Vec<_>>();
        keys.into_iter()
            .filter_map(|key| {
                let plugin_id = key.1.clone();
                self.grants.remove(&key).map(|grants| (plugin_id, grants))
            })
            .collect()
    }

    pub fn plugin_ids_for(&self, project_id: &str) -> Vec<String> {
        self.grants
            .keys()
            .filter_map(|(entry_project, plugin_id)| {
                (entry_project == project_id).then(|| plugin_id.clone())
            })
            .collect()
    }
}

fn capability_matches(requested: &str, granted: &str) -> bool {
    requested == granted
        || requested
            .strip_suffix(":<type>")
            .is_some_and(|prefix| granted.starts_with(prefix))
        || requested
            .strip_suffix(":<name>")
            .is_some_and(|prefix| granted.starts_with(prefix))
        || granted.strip_prefix(requested).is_some_and(|suffix| {
            suffix.starts_with('@') && suffix[1..].chars().all(|c| c.is_ascii_digit())
        })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Session {
    pub id: String,
    pub plugin_id: String,
    pub package_digest: String,
    pub plugin_version: String,
    pub host_api: String,
    pub project_id: String,
    pub origin: String,
    pub grants: BTreeSet<String>,
    pub generation: u64,
    pub expires_at: SystemTime,
    pub revoked: bool,
}

#[derive(Debug, Default, Clone)]
pub struct SessionRegistry {
    sessions: BTreeMap<String, Session>,
    generations: BTreeMap<(String, String), u64>,
}

impl SessionRegistry {
    pub fn issue(
        &mut self,
        entry: &CatalogEntry,
        project_id: &str,
        origin: &str,
        grants: BTreeSet<String>,
        ttl: Duration,
    ) -> Session {
        let key = (project_id.into(), entry.manifest.id.clone());
        for existing in self.sessions.values_mut().filter(|session| {
            session.project_id == project_id && session.plugin_id == entry.manifest.id
        }) {
            existing.revoked = true;
        }
        let generation = self
            .generations
            .entry(key)
            .and_modify(|value| *value += 1)
            .or_insert(1);
        let sequence = SESSION_COUNTER.fetch_add(1, Ordering::Relaxed);
        let mut hasher = Sha256::new();
        hasher.update(entry.manifest.id.as_bytes());
        hasher.update(entry.digest.as_bytes());
        hasher.update(sequence.to_le_bytes());
        hasher.update(format!("{:?}", SystemTime::now()).as_bytes());
        let session = Session {
            id: hex_digest(&hasher.finalize()),
            plugin_id: entry.manifest.id.clone(),
            package_digest: entry.digest.clone(),
            plugin_version: entry.manifest.version.clone(),
            host_api: entry.manifest.host_api.clone(),
            project_id: project_id.into(),
            origin: origin.into(),
            grants,
            generation: *generation,
            expires_at: SystemTime::now() + ttl,
            revoked: false,
        };
        self.sessions.insert(session.id.clone(), session.clone());
        session
    }
    pub fn get(&self, id: &str) -> Option<&Session> {
        self.sessions.get(id)
    }
    fn find_active(&self, plugin_id: &str, project_id: &str, origin: &str) -> Option<Session> {
        self.sessions
            .values()
            .find(|session| {
                session.plugin_id == plugin_id
                    && session.project_id == project_id
                    && session.origin == origin
                    && !session.revoked
                    && session.expires_at > SystemTime::now()
            })
            .cloned()
    }
    pub fn revoke_session(&mut self, id: &str) {
        if let Some(session) = self.sessions.get_mut(id) {
            session.revoked = true;
        }
    }
    pub fn revoke_plugin(&mut self, project_id: &str, plugin_id: &str) {
        for session in self
            .sessions
            .values_mut()
            .filter(|s| s.project_id == project_id && s.plugin_id == plugin_id)
        {
            session.revoked = true;
        }
    }
    pub fn revoke_project(&mut self, project_id: &str) {
        for session in self
            .sessions
            .values_mut()
            .filter(|s| s.project_id == project_id)
        {
            session.revoked = true;
        }
    }
    fn valid(&self, id: &str, origin: &str) -> Result<&Session, RpcError> {
        let session = self
            .sessions
            .get(id)
            .ok_or_else(|| rpc_error("session.invalid", "unknown session", false))?;
        if session.revoked {
            return Err(rpc_error(
                "session.revoked",
                "session has been revoked",
                false,
            ));
        }
        if session.origin != origin {
            return Err(rpc_error(
                "session.origin",
                "session origin mismatch",
                false,
            ));
        }
        if session.expires_at <= SystemTime::now() {
            return Err(rpc_error("session.expired", "session has expired", false));
        }
        if self
            .generations
            .get(&(session.project_id.clone(), session.plugin_id.clone()))
            != Some(&session.generation)
        {
            return Err(rpc_error(
                "session.stale",
                "session activation generation is stale",
                false,
            ));
        }
        Ok(session)
    }
}

#[derive(Debug, Default, Clone)]
pub struct NamespaceOwnership {
    owners: BTreeMap<String, String>,
    relationship_types: BTreeMap<String, String>,
    fields: BTreeMap<(String, String), (String, bool)>,
}

impl NamespaceOwnership {
    pub fn register_manifest(&mut self, manifest: &PluginManifest) -> Result<(), HostError> {
        let mut next_owners = self.owners.clone();
        let mut next_relationship_types = self.relationship_types.clone();
        let mut next_fields = self.fields.clone();
        for namespace in &manifest.namespaces {
            if let Some(owner) = next_owners.get(namespace) {
                if owner != &manifest.id {
                    return Err(HostError(format!(
                        "namespace {namespace} is owned by {owner}"
                    )));
                }
            }
            next_owners.insert(namespace.clone(), manifest.id.clone());
        }
        for schema in &manifest.schemas {
            for field in &schema.fields {
                let field_key = (schema.namespace.clone(), field.key.clone());
                if let Some((owner, _)) = next_fields.get(&field_key) {
                    if owner != &manifest.id {
                        return Err(HostError(format!(
                            "field {}.{} is owned by {owner}",
                            schema.namespace, field.key
                        )));
                    }
                }
                next_fields.insert(field_key, (manifest.id.clone(), field.shared));
                if let Some(relationship_type) = &field.relationship_type {
                    if let Some(owner) = next_relationship_types.get(relationship_type) {
                        if owner != &manifest.id {
                            return Err(HostError(format!(
                                "relationship type {relationship_type} is owned by {owner}"
                            )));
                        }
                    }
                    next_relationship_types.insert(relationship_type.clone(), manifest.id.clone());
                }
            }
        }
        self.owners = next_owners;
        self.relationship_types = next_relationship_types;
        self.fields = next_fields;
        Ok(())
    }
    pub fn owner(&self, namespace: &str) -> Option<&str> {
        self.owners.get(namespace).map(String::as_str)
    }

    pub fn relationship_owner(&self, relationship_type: &str) -> Option<&str> {
        self.relationship_types
            .get(relationship_type)
            .map(String::as_str)
    }

    pub fn field_owner(&self, namespace: &str, key: &str) -> Option<&str> {
        self.fields
            .get(&(namespace.into(), key.into()))
            .map(|(owner, _)| owner.as_str())
    }

    pub fn field_is_shared(&self, namespace: &str, key: &str) -> bool {
        self.fields
            .get(&(namespace.into(), key.into()))
            .is_some_and(|(_, shared)| *shared)
    }

    pub fn namespace_has_shared_fields(&self, namespace: &str) -> bool {
        self.fields
            .iter()
            .any(|((field_namespace, _), (_, shared))| field_namespace == namespace && *shared)
    }

    pub fn shared_field_keys(&self, namespace: &str) -> BTreeSet<String> {
        self.fields
            .iter()
            .filter(|((field_namespace, _), (_, shared))| field_namespace == namespace && *shared)
            .map(|((_, key), _)| key.clone())
            .collect()
    }
}

impl NamespaceView for NamespaceOwnership {
    fn owner(&self, namespace: &str) -> Option<&str> {
        NamespaceOwnership::owner(self, namespace)
    }

    fn field_is_shared(&self, namespace: &str, key: &str) -> bool {
        NamespaceOwnership::field_is_shared(self, namespace, key)
    }

    fn namespace_has_shared_fields(&self, namespace: &str) -> bool {
        NamespaceOwnership::namespace_has_shared_fields(self, namespace)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DependencyPlan {
    pub order: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum VisitState {
    Visiting,
    Visited,
}

pub struct DependencyResolver;

impl DependencyResolver {
    pub fn resolve(catalog: &PluginCatalog, root: &str) -> Result<DependencyPlan, HostError> {
        let mut states = BTreeMap::new();
        let mut order = Vec::new();
        Self::visit(catalog, root, &mut states, &mut order)?;
        Ok(DependencyPlan { order })
    }

    fn visit(
        catalog: &PluginCatalog,
        id: &str,
        states: &mut BTreeMap<String, VisitState>,
        order: &mut Vec<String>,
    ) -> Result<(), HostError> {
        match states.get(id) {
            Some(VisitState::Visited) => return Ok(()),
            Some(VisitState::Visiting) => {
                return Err(HostError(format!("plugin dependency cycle includes {id}")))
            }
            None => {}
        }
        let entry = catalog
            .get(id)
            .ok_or_else(|| HostError(format!("plugin dependency is not installed: {id}")))?;
        states.insert(id.into(), VisitState::Visiting);
        for (dependency_id, dependency) in &entry.manifest.dependencies {
            let Some(dependency_entry) = catalog.get(dependency_id) else {
                if dependency.required {
                    return Err(HostError(format!(
                        "required plugin dependency is not installed: {dependency_id}"
                    )));
                }
                continue;
            };
            if !version_satisfies(&dependency_entry.manifest.version, &dependency.version) {
                if dependency.required {
                    return Err(HostError(format!(
                        "plugin {dependency_id} version {} does not satisfy {}",
                        dependency_entry.manifest.version, dependency.version
                    )));
                }
                continue;
            }
            Self::visit(catalog, dependency_id, states, order)?;
        }
        states.insert(id.into(), VisitState::Visited);
        order.push(id.into());
        Ok(())
    }
}

fn version_satisfies(version: &str, range: &str) -> bool {
    if range.trim().is_empty() || range.trim() == "*" {
        return true;
    }
    let Some(actual) = parse_version(version) else {
        return false;
    };
    range.split_whitespace().all(|constraint| {
        let (operator, value) = if let Some(value) = constraint.strip_prefix(">=") {
            (">=", value)
        } else if let Some(value) = constraint.strip_prefix("<=") {
            ("<=", value)
        } else if let Some(value) = constraint.strip_prefix('>') {
            (">", value)
        } else if let Some(value) = constraint.strip_prefix('<') {
            ("<", value)
        } else if let Some(value) = constraint.strip_prefix('^') {
            ("^", value)
        } else if let Some(value) = constraint.strip_prefix('~') {
            ("~", value)
        } else {
            ("=", constraint)
        };
        let Some(required) = parse_version(value) else {
            return false;
        };
        match operator {
            ">=" => actual >= required,
            "<=" => actual <= required,
            ">" => actual > required,
            "<" => actual < required,
            "^" => actual >= required && actual.0 == required.0,
            "~" => actual >= required && actual.0 == required.0 && actual.1 == required.1,
            "=" => actual == required,
            _ => false,
        }
    })
}

fn parse_version(value: &str) -> Option<(u64, u64, u64)> {
    let core = value.split(['-', '+']).next()?;
    let mut parts = core.split('.');
    Some((
        parts.next()?.parse().ok()?,
        parts.next()?.parse().ok()?,
        parts.next()?.parse().ok()?,
    ))
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EventEnvelope {
    pub project_id: String,
    pub name: String,
    pub version: u32,
    pub source_plugin: String,
    pub sequence: u64,
    pub payload: serde_json::Value,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct PublishResult {
    pub delivered: usize,
    pub dropped: usize,
}

#[derive(Debug, Clone)]
pub struct EventBus {
    queues: BTreeMap<(String, String, String), VecDeque<EventEnvelope>>,
    queue_limit: usize,
    payload_limit: usize,
    next_sequence: u64,
}

impl Default for EventBus {
    fn default() -> Self {
        Self::new(64, 256 * 1024)
    }
}

impl EventBus {
    pub fn new(queue_limit: usize, payload_limit: usize) -> Self {
        Self {
            queues: BTreeMap::new(),
            queue_limit: queue_limit.max(1),
            payload_limit,
            next_sequence: 0,
        }
    }

    pub fn subscribe(&mut self, project_id: &str, plugin_id: &str, name: &str, version: u32) {
        self.queues
            .entry((
                project_id.into(),
                plugin_id.into(),
                event_key(name, version),
            ))
            .or_default();
    }

    pub fn unsubscribe(&mut self, project_id: &str, plugin_id: &str, name: &str, version: u32) {
        self.queues.remove(&(
            project_id.into(),
            plugin_id.into(),
            event_key(name, version),
        ));
    }

    pub fn unsubscribe_plugin(&mut self, project_id: &str, plugin_id: &str) {
        self.queues
            .retain(|(project, subscriber, _), _| project != project_id || subscriber != plugin_id);
    }

    pub fn publish(
        &mut self,
        project_id: &str,
        source_plugin: &str,
        name: &str,
        version: u32,
        payload: serde_json::Value,
    ) -> Result<PublishResult, HostError> {
        let size = serde_json::to_vec(&payload)
            .map_err(|error| HostError(format!("event payload is not serializable: {error}")))?
            .len();
        if size > self.payload_limit {
            return Err(HostError("event payload exceeds host limit".into()));
        }
        self.next_sequence = self.next_sequence.wrapping_add(1);
        let key = event_key(name, version);
        let mut delivered = 0;
        let mut dropped = 0;
        for ((project, _, subscription), queue) in self.queues.iter_mut() {
            if project != project_id {
                continue;
            }
            if subscription != &key {
                continue;
            }
            if queue.len() >= self.queue_limit {
                queue.pop_front();
                dropped += 1;
            }
            queue.push_back(EventEnvelope {
                project_id: project_id.into(),
                name: name.into(),
                version,
                source_plugin: source_plugin.into(),
                sequence: self.next_sequence,
                payload: payload.clone(),
            });
            delivered += 1;
        }
        Ok(PublishResult { delivered, dropped })
    }

    pub fn drain(
        &mut self,
        project_id: &str,
        plugin_id: &str,
        name: &str,
        version: u32,
    ) -> Vec<EventEnvelope> {
        self.queues
            .get_mut(&(
                project_id.into(),
                plugin_id.into(),
                event_key(name, version),
            ))
            .map(|queue| queue.drain(..).collect())
            .unwrap_or_default()
    }
}

fn event_key(name: &str, version: u32) -> String {
    format!("{name}@{version}")
}

#[derive(Debug, Clone)]
pub struct CancellationToken(Arc<AtomicBool>);

impl CancellationToken {
    pub fn new() -> Self {
        Self(Arc::new(AtomicBool::new(false)))
    }
    pub fn cancel(&self) {
        self.0.store(true, Ordering::Release);
    }
    pub fn is_cancelled(&self) -> bool {
        self.0.load(Ordering::Acquire)
    }
}

impl Default for CancellationToken {
    fn default() -> Self {
        Self::new()
    }
}

pub struct ServiceRequest {
    pub payload: serde_json::Value,
    pub cancellation: CancellationToken,
}

pub type ServiceHandler =
    Arc<dyn Fn(ServiceRequest) -> Result<serde_json::Value, HostError> + Send + Sync>;
type CapabilityGrants = BTreeMap<(String, String), BTreeSet<String>>;
type InFlightCalls = Arc<(Mutex<BTreeMap<(String, u64), CancellationToken>>, Condvar)>;

#[derive(Clone)]
struct ServiceProvider {
    plugin_id: String,
    handler: ServiceHandler,
    health: Arc<Mutex<ProviderHealth>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProviderHealth {
    Active,
    Disabled,
    Failed,
    Quarantined,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct ServiceKey {
    name: String,
    major: u32,
}

#[derive(Clone)]
pub struct ServiceRegistry {
    providers: BTreeMap<ServiceKey, ServiceProvider>,
    payload_limit: usize,
    in_flight: InFlightCalls,
    next_call: Arc<AtomicU64>,
}

#[derive(Debug, Clone, Default)]
pub struct DeclarationRegistry {
    views: BTreeMap<(String, String, String), View>,
    commands: BTreeMap<(String, String, String), Command>,
}

impl DeclarationRegistry {
    pub fn register(
        &mut self,
        project_id: &str,
        plugin_id: &str,
        manifest: &PluginManifest,
    ) -> Result<(), HostError> {
        let mut views = self.views.clone();
        let mut commands = self.commands.clone();
        for view in &manifest.views {
            let key = (project_id.into(), plugin_id.into(), view.id.clone());
            if views.contains_key(&key) {
                return Err(HostError(format!(
                    "view is already registered: {}",
                    view.id
                )));
            }
            views.insert(key, view.clone());
        }
        for command in &manifest.commands {
            let key = (project_id.into(), plugin_id.into(), command.id.clone());
            if commands.contains_key(&key) {
                return Err(HostError(format!(
                    "command is already registered: {}",
                    command.id
                )));
            }
            commands.insert(key, command.clone());
        }
        self.views = views;
        self.commands = commands;
        Ok(())
    }

    pub fn unregister_plugin(&mut self, project_id: &str, plugin_id: &str) {
        self.views
            .retain(|(project, plugin, _), _| project != project_id || plugin != plugin_id);
        self.commands
            .retain(|(project, plugin, _), _| project != project_id || plugin != plugin_id);
    }

    pub fn views(&self, project_id: &str, plugin_id: &str) -> Vec<View> {
        self.views
            .iter()
            .filter(|((project, plugin, _), _)| project == project_id && plugin == plugin_id)
            .map(|(_, view)| view.clone())
            .collect()
    }

    pub fn commands(&self, project_id: &str, plugin_id: &str) -> Vec<Command> {
        self.commands
            .iter()
            .filter(|((project, plugin, _), _)| project == project_id && plugin == plugin_id)
            .map(|(_, command)| command.clone())
            .collect()
    }

    fn command(&self, project_id: &str, plugin_id: &str, command_id: &str) -> Option<Command> {
        self.commands
            .get(&(project_id.into(), plugin_id.into(), command_id.into()))
            .cloned()
    }
}

impl ServiceRegistry {
    pub fn new(payload_limit: usize) -> Self {
        Self {
            providers: BTreeMap::new(),
            payload_limit,
            in_flight: Arc::new((Mutex::new(BTreeMap::new()), Condvar::new())),
            next_call: Arc::new(AtomicU64::new(1)),
        }
    }

    pub fn register(
        &mut self,
        plugin_id: &str,
        name: &str,
        major: u32,
        handler: ServiceHandler,
    ) -> Result<(), HostError> {
        let key = ServiceKey {
            name: name.into(),
            major,
        };
        if let Some(provider) = self.providers.get_mut(&key) {
            if provider.plugin_id != plugin_id {
                return Err(HostError(format!(
                    "service provider already exists: {name}@{major}"
                )));
            }
            provider.handler = handler;
            if let Ok(mut health) = provider.health.lock() {
                *health = ProviderHealth::Active;
            }
            return Ok(());
        }
        self.providers.insert(
            key,
            ServiceProvider {
                plugin_id: plugin_id.into(),
                handler,
                health: Arc::new(Mutex::new(ProviderHealth::Active)),
            },
        );
        Ok(())
    }

    pub fn has_provider(&self, name: &str, major: u32) -> bool {
        self.providers.contains_key(&ServiceKey {
            name: name.into(),
            major,
        })
    }

    pub fn provider_health(&self, name: &str, major: u32) -> Option<ProviderHealth> {
        self.providers
            .get(&ServiceKey {
                name: name.into(),
                major,
            })
            .and_then(|provider| provider.health.lock().ok().map(|health| *health))
    }

    pub fn unregister_plugin(&mut self, plugin_id: &str) {
        self.deactivate_plugin(plugin_id, Duration::ZERO);
    }

    pub fn resume_plugin(&mut self, plugin_id: &str) {
        for provider in self
            .providers
            .values_mut()
            .filter(|provider| provider.plugin_id == plugin_id)
        {
            if let Ok(mut health) = provider.health.lock() {
                *health = ProviderHealth::Active;
            }
        }
    }

    /// Stop accepting provider calls, cancel cooperative work, and wait for a
    /// bounded grace period. A provider that does not drain is quarantined so
    /// a later activation cannot accidentally reuse a wedged implementation.
    pub fn deactivate_plugin(&mut self, plugin_id: &str, grace: Duration) -> bool {
        for provider in self
            .providers
            .values_mut()
            .filter(|provider| provider.plugin_id == plugin_id)
        {
            if let Ok(mut health) = provider.health.lock() {
                *health = ProviderHealth::Disabled;
            }
        }
        let (in_flight, wake) = &*self.in_flight;
        if let Ok(calls) = in_flight.lock() {
            for ((owner, _), token) in calls.iter() {
                if owner == plugin_id {
                    token.cancel();
                }
            }
        }
        let deadline = std::time::Instant::now() + grace;
        let mut calls = match in_flight.lock() {
            Ok(calls) => calls,
            Err(_) => return false,
        };
        loop {
            if !calls.keys().any(|(owner, _)| owner == plugin_id) {
                return true;
            }
            let now = std::time::Instant::now();
            if now >= deadline {
                break;
            }
            let timeout = deadline.saturating_duration_since(now);
            let (next, result) = wake.wait_timeout(calls, timeout).unwrap();
            calls = next;
            if result.timed_out() {
                break;
            }
        }
        for provider in self
            .providers
            .values_mut()
            .filter(|provider| provider.plugin_id == plugin_id)
        {
            if let Ok(mut health) = provider.health.lock() {
                *health = ProviderHealth::Quarantined;
            }
        }
        false
    }

    pub fn call(
        &self,
        consumer_id: &str,
        name: &str,
        major: u32,
        payload: serde_json::Value,
        deadline: Duration,
    ) -> Result<serde_json::Value, HostError> {
        self.call_with_stack(&[consumer_id.to_string()], name, major, payload, deadline)
    }

    pub fn call_with_stack(
        &self,
        stack: &[String],
        name: &str,
        major: u32,
        payload: serde_json::Value,
        deadline: Duration,
    ) -> Result<serde_json::Value, HostError> {
        let size = serde_json::to_vec(&payload)
            .map_err(|error| HostError(format!("service payload is not serializable: {error}")))?
            .len();
        if size > self.payload_limit {
            return Err(HostError("service payload exceeds host limit".into()));
        }
        let key = format!("{name}@{major}");
        if stack.iter().any(|item| item == &key) {
            return Err(HostError("re-entrant service call cycle detected".into()));
        }
        let provider = self
            .providers
            .get(&ServiceKey {
                name: name.into(),
                major,
            })
            .ok_or_else(|| HostError("service provider unavailable".into()))?;
        let health = provider
            .health
            .lock()
            .map(|health| *health)
            .unwrap_or(ProviderHealth::Quarantined);
        if health != ProviderHealth::Active {
            return Err(HostError("service provider unavailable".into()));
        }
        let handler = Arc::clone(&provider.handler);
        let cancellation = CancellationToken::new();
        let worker_token = cancellation.clone();
        let provider_id = provider.plugin_id.clone();
        let call_id = self.next_call.fetch_add(1, Ordering::Relaxed);
        self.in_flight
            .0
            .lock()
            .map_err(|_| HostError("service lifecycle state unavailable".into()))?
            .insert((provider_id.clone(), call_id), cancellation.clone());
        let in_flight = Arc::clone(&self.in_flight);
        let (sender, receiver) = mpsc::channel();
        thread::spawn(move || {
            let result = handler(ServiceRequest {
                payload,
                cancellation: worker_token,
            });
            let _ = sender.send(result);
            if let Ok(mut calls) = in_flight.0.lock() {
                calls.remove(&(provider_id, call_id));
                in_flight.1.notify_all();
            }
        });
        match receiver.recv_timeout(deadline) {
            Ok(Ok(result)) => Ok(result),
            Ok(Err(error)) => {
                if let Ok(mut health) = provider.health.lock() {
                    *health = ProviderHealth::Failed;
                }
                Err(HostError(format!("provider-unavailable: {error}")))
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {
                cancellation.cancel();
                if let Ok(mut health) = provider.health.lock() {
                    *health = ProviderHealth::Failed;
                }
                Err(HostError(
                    "provider-unavailable: service deadline exceeded".into(),
                ))
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                if let Ok(mut health) = provider.health.lock() {
                    *health = ProviderHealth::Failed;
                }
                Err(HostError(
                    "provider-unavailable: service provider failed".into(),
                ))
            }
        }
    }
}

impl std::fmt::Debug for ServiceRegistry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ServiceRegistry")
            .field("providers", &self.providers.keys().collect::<Vec<_>>())
            .field("payload_limit", &self.payload_limit)
            .finish()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LifecycleRecord {
    pub state: LifecycleState,
    pub failures: u32,
    pub last_error: Option<String>,
}

#[derive(Debug, Default, Clone)]
pub struct LifecycleRegistry {
    records: BTreeMap<(String, String), LifecycleRecord>,
}

impl LifecycleRegistry {
    fn record(&self, project_id: &str, plugin_id: &str) -> LifecycleRecord {
        self.records
            .get(&(project_id.into(), plugin_id.into()))
            .cloned()
            .unwrap_or(LifecycleRecord {
                state: LifecycleState::Discovered,
                failures: 0,
                last_error: None,
            })
    }

    pub fn state(&self, project_id: &str, plugin_id: &str) -> LifecycleRecord {
        self.record(project_id, plugin_id)
    }

    /// Clear a quarantine or failed state after an explicit user retry. Resets
    /// the failure counter and last error so activation can be attempted again.
    pub fn clear_quarantine(&mut self, project_id: &str, plugin_id: &str) {
        let mut record = self.record(project_id, plugin_id);
        record.failures = 0;
        record.last_error = None;
        record.state = LifecycleState::Resolved;
        self.records
            .insert((project_id.into(), plugin_id.into()), record);
    }

    pub fn activate_with<F>(
        &mut self,
        project_id: &str,
        plugin_id: &str,
        start: F,
    ) -> Result<(), HostError>
    where
        F: FnOnce() -> Result<(), HostError>,
    {
        self.begin_activation(project_id, plugin_id)?;
        match start() {
            Ok(()) => {
                self.activation_succeeded(project_id, plugin_id);
                Ok(())
            }
            Err(error) => {
                self.activation_failed(project_id, plugin_id, &error.to_string());
                Err(error)
            }
        }
    }

    fn set_state(&mut self, project_id: &str, plugin_id: &str, state: LifecycleState) {
        self.records
            .entry((project_id.into(), plugin_id.into()))
            .or_insert(LifecycleRecord {
                state: LifecycleState::Discovered,
                failures: 0,
                last_error: None,
            })
            .state = state;
    }

    fn begin_activation(&mut self, project_id: &str, plugin_id: &str) -> Result<(), HostError> {
        let mut record = self.record(project_id, plugin_id);
        if record.state == LifecycleState::Quarantined {
            return Err(HostError("plugin is quarantined".into()));
        }
        if record.state == LifecycleState::Active {
            return Ok(());
        }
        if record.state == LifecycleState::Resolved {
            record.state = LifecycleState::Activating;
            self.records
                .insert((project_id.into(), plugin_id.into()), record);
            return Ok(());
        }
        if record.state == LifecycleState::Failed {
            record.state = LifecycleState::Activating;
            self.records
                .insert((project_id.into(), plugin_id.into()), record);
            return Ok(());
        }
        for next in [
            LifecycleState::Validated,
            LifecycleState::Installed,
            LifecycleState::Resolved,
            LifecycleState::Activating,
        ] {
            if record.state == next {
                continue;
            }
            if !lifecycle_transition(record.state.clone(), next.clone()) {
                return Err(HostError(format!(
                    "invalid lifecycle transition {:?} -> {:?}",
                    record.state, next
                )));
            } else {
                record.state = next;
            }
        }
        self.records
            .insert((project_id.into(), plugin_id.into()), record);
        Ok(())
    }

    fn activation_succeeded(&mut self, project_id: &str, plugin_id: &str) {
        self.set_state(project_id, plugin_id, LifecycleState::Active);
    }

    fn activation_failed(&mut self, project_id: &str, plugin_id: &str, error: &str) {
        let mut record = self.record(project_id, plugin_id);
        record.failures = record.failures.saturating_add(1);
        record.last_error = Some(error.into());
        record.state = LifecycleState::Failed;
        if record.failures >= 3 {
            record.state = LifecycleState::Quarantined;
        }
        self.records
            .insert((project_id.into(), plugin_id.into()), record);
    }

    fn deactivate(&mut self, project_id: &str, plugin_id: &str) {
        let record = self.record(project_id, plugin_id);
        if record.state == LifecycleState::Active {
            self.set_state(project_id, plugin_id, LifecycleState::Deactivating);
            self.set_state(project_id, plugin_id, LifecycleState::Resolved);
        }
    }

    fn quarantine(&mut self, project_id: &str, plugin_id: &str, error: &str) {
        let mut record = self.record(project_id, plugin_id);
        record.last_error = Some(error.into());
        record.state = LifecycleState::Quarantined;
        self.records
            .insert((project_id.into(), plugin_id.into()), record);
    }
}

#[derive(Clone)]
pub struct PluginHost {
    pub catalog: PluginCatalog,
    pub packages: PackageCatalog,
    /// Selected package version per project; package installations remain
    /// global while activation is project-scoped.
    pub project_versions: BTreeMap<(String, String), String>,
    pub grants: GrantStore,
    pub sessions: SessionRegistry,
    pub namespaces: NamespaceOwnership,
    pub session_ttl: Duration,
    pub lifecycle: LifecycleRegistry,
    pub events: EventBus,
    pub services: ServiceRegistry,
    pub declarations: DeclarationRegistry,
    pub wasm: WasmRuntimeRegistry,
    state_path: Option<PathBuf>,
    project_usage: BTreeMap<(String, String), String>,
    /// Unmigrated grants still present in the global plugin-state file.
    legacy_grants: GrantStore,
    /// Open-project grant file paths keyed by project id (directory root).
    project_grant_paths: BTreeMap<String, PathBuf>,
    ai_requests: BTreeMap<String, (String, String, String, String, Option<serde_json::Value>)>,
}

impl Default for PluginHost {
    fn default() -> Self {
        Self::new()
    }
}
impl PluginHost {
    pub fn new() -> Self {
        Self {
            catalog: PluginCatalog::default(),
            packages: PackageCatalog::default(),
            project_versions: BTreeMap::new(),
            grants: GrantStore::default(),
            sessions: SessionRegistry::default(),
            namespaces: NamespaceOwnership::default(),
            session_ttl: Duration::from_secs(15 * 60),
            lifecycle: LifecycleRegistry::default(),
            events: EventBus::default(),
            services: ServiceRegistry::new(256 * 1024),
            declarations: DeclarationRegistry::default(),
            wasm: WasmRuntimeRegistry::default(),
            state_path: None,
            project_usage: BTreeMap::new(),
            legacy_grants: GrantStore::default(),
            project_grant_paths: BTreeMap::new(),
            ai_requests: BTreeMap::new(),
        }
    }

    /// Load and verify packages retained in the app-owned store. Invalid
    /// versions are returned as rejected entries and are never placed in the
    /// executable package catalog.
    pub fn load_installed_packages(
        &mut self,
        install_root: impl AsRef<Path>,
        state_path: impl AsRef<Path>,
        limits: ArchiveLimits,
        policy: VerificationPolicy,
    ) -> Result<Vec<String>, HostError> {
        let state_path = state_path.as_ref().to_path_buf();
        let state = if state_path.is_file() {
            let bytes = fs::read(&state_path).map_err(io_error)?;
            serde_json::from_slice::<PersistentHostState>(&bytes)
                .map_err(|error| HostError(format!("invalid persistent plugin state: {error}")))?
        } else {
            PersistentHostState::default()
        };
        self.packages = state.packages;
        self.legacy_grants = state.grants;
        self.grants = GrantStore::default();
        self.project_grant_paths.clear();
        self.project_usage = state
            .project_usage
            .into_iter()
            .map(|usage| ((usage.project_id, usage.plugin_id), usage.version))
            .collect();
        self.state_path = Some(state_path);
        let rejected = self
            .packages
            .rediscover(install_root, limits, &policy)
            .map_err(|error| HostError(error.to_string()))?
            .into_iter()
            .map(|error| error.to_string())
            .collect::<Vec<_>>();
        for plugin_id in self.packages.plugin_ids().cloned().collect::<Vec<_>>() {
            if self.catalog.get(&plugin_id).is_some() {
                continue;
            }
            let package = self
                .packages
                .active_candidate(&plugin_id)
                .ok_or_else(|| HostError("discovered plugin has no active version".into()))?;
            let manifest = parse_manifest(
                &fs::read_to_string(package.root.join("manifest.json")).map_err(io_error)?,
            )
            .map_err(|error| HostError(error.to_string()))?;
            let entry = CatalogEntry {
                manifest: manifest.clone(),
                package_root: package.root.clone(),
                digest: package.digest.clone(),
                embedded_wasm: None,
            };
            self.catalog.insert_for_test(entry)?;
            if let Err(error) = self.namespaces.register_manifest(&manifest) {
                self.catalog.remove(&plugin_id);
                return Err(error);
            }
        }
        self.persist_state()?;
        Ok(rejected)
    }

    fn persist_state(&self) -> Result<(), HostError> {
        let Some(path) = &self.state_path else {
            return Ok(());
        };
        let project_usage = self
            .project_usage
            .iter()
            .map(|((project_id, plugin_id), version)| ProjectPluginUsage {
                project_id: project_id.clone(),
                plugin_id: plugin_id.clone(),
                version: version.clone(),
            })
            .collect();
        let state = PersistentHostState {
            packages: self.packages.clone(),
            grants: self.legacy_grants.clone(),
            project_usage,
        };
        let bytes = serde_json::to_vec_pretty(&state)
            .map_err(|error| HostError(format!("serialize persistent plugin state: {error}")))?;
        let parent = path
            .parent()
            .ok_or_else(|| HostError("persistent plugin state has no parent directory".into()))?;
        fs::create_dir_all(parent).map_err(io_error)?;
        let temporary = tempfile::NamedTempFile::new_in(parent).map_err(io_error)?;
        fs::write(temporary.path(), bytes).map_err(io_error)?;
        temporary
            .persist(path)
            .map_err(|error| HostError(error.error.to_string()))?;
        Ok(())
    }

    pub fn record_project_usage(
        &mut self,
        project_id: &str,
        plugin_id: &str,
        version: &str,
    ) -> Result<(), HostError> {
        let key = (project_id.to_owned(), plugin_id.to_owned());
        let previous = self.project_usage.insert(key.clone(), version.to_owned());
        if let Err(error) = self.persist_state() {
            match previous {
                Some(version) => {
                    self.project_usage.insert(key, version);
                }
                None => {
                    self.project_usage.remove(&key);
                }
            }
            return Err(error);
        }
        Ok(())
    }

    /// Record an explicit user capability decision. Manifest capabilities are
    /// requests only; an empty grant set is a valid and safe decision.
    pub fn grant_capabilities(
        &mut self,
        project_id: &str,
        plugin_id: &str,
        granted: BTreeSet<String>,
    ) -> Result<(), HostError> {
        let entry = self
            .runtime_entry(project_id, plugin_id)
            .ok_or_else(|| HostError("plugin is not installed".into()))?;
        self.grants
            .set(project_id, plugin_id, &entry.manifest.capabilities, granted)?;
        self.persist_project_grants(project_id)
    }

    /// First-party bundled modules default to enabled without a consent dialog.
    /// When a project has no recorded grants yet, grant the full declared set
    /// so sandboxed RPC (for example Maps `entity.list`) is not deny-all.
    pub fn ensure_first_party_bundled_grants(
        &mut self,
        project_id: &str,
        plugin_id: &str,
    ) -> Result<(), HostError> {
        let Some(entry) = self.runtime_entry(project_id, plugin_id) else {
            return Ok(());
        };
        if !entry.package_root.as_os_str().is_empty() {
            return Ok(());
        }
        if entry.manifest.publisher != "daena-archive" {
            return Ok(());
        }
        if entry.manifest.capabilities.is_empty() || !self.grants.is_empty(project_id, plugin_id) {
            return Ok(());
        }
        self.grant_capabilities(
            project_id,
            plugin_id,
            entry.manifest.capabilities.iter().cloned().collect(),
        )
    }

    /// Bind an open project to its machine-local grants file, migrating any
    /// legacy global entries for that project on first open.
    pub fn bind_project_grants(
        &mut self,
        project_root: impl AsRef<Path>,
        project_id: &str,
    ) -> Result<(), HostError> {
        let grants_path = project_root
            .as_ref()
            .join(".daena")
            .join("local")
            .join("plugin-grants.json");
        self.project_grant_paths
            .insert(project_id.to_owned(), grants_path.clone());
        self.grants.clear_project(project_id);

        if grants_path.is_file() {
            let file = Self::read_project_grants_file(&grants_path)?;
            for entry in file.grants {
                self.grants
                    .insert_loaded(project_id, &entry.plugin_id, entry.grants);
            }
            return Ok(());
        }

        let migrated = self.legacy_grants.take_project(project_id);
        if migrated.is_empty() {
            return Ok(());
        }
        for (plugin_id, grants) in &migrated {
            self.grants
                .insert_loaded(project_id, plugin_id, grants.clone());
        }
        self.write_project_grants_file(&grants_path, &migrated)?;
        self.persist_state()?;
        Ok(())
    }

    pub fn persist_project_grants(&self, project_id: &str) -> Result<(), HostError> {
        let Some(path) = self.project_grant_paths.get(project_id) else {
            return Ok(());
        };
        let entries = self
            .grants
            .plugin_ids_for(project_id)
            .into_iter()
            .map(|plugin_id| {
                let grants = self.grants.get(project_id, &plugin_id);
                (plugin_id, grants)
            })
            .collect::<Vec<_>>();
        self.write_project_grants_file(path, &entries)
    }

    fn read_project_grants_file(path: &Path) -> Result<ProjectGrantsFile, HostError> {
        let bytes = fs::read(path).map_err(io_error)?;
        let file: ProjectGrantsFile = serde_json::from_slice(&bytes)
            .map_err(|error| HostError(format!("invalid project plugin grants: {error}")))?;
        if file.format_version != PROJECT_GRANTS_FORMAT_VERSION {
            return Err(HostError(format!(
                "unsupported project plugin grants format version {}",
                file.format_version
            )));
        }
        Ok(file)
    }

    fn write_project_grants_file(
        &self,
        path: &Path,
        entries: &[(String, BTreeSet<String>)],
    ) -> Result<(), HostError> {
        let file = ProjectGrantsFile {
            format_version: PROJECT_GRANTS_FORMAT_VERSION,
            grants: entries
                .iter()
                .map(|(plugin_id, grants)| ProjectGrantEntry {
                    plugin_id: plugin_id.clone(),
                    grants: grants.clone(),
                })
                .collect(),
        };
        let mut bytes = serde_json::to_vec_pretty(&file)
            .map_err(|error| HostError(format!("serialize project plugin grants: {error}")))?;
        bytes.push(b'\n');
        let parent = path
            .parent()
            .ok_or_else(|| HostError("project plugin grants path has no parent".into()))?;
        fs::create_dir_all(parent).map_err(io_error)?;
        let temporary = tempfile::NamedTempFile::new_in(parent).map_err(io_error)?;
        fs::write(temporary.path(), bytes).map_err(io_error)?;
        temporary
            .persist(path)
            .map_err(|error| HostError(error.error.to_string()))?;
        Ok(())
    }

    pub fn clear_project_usage(
        &mut self,
        project_id: &str,
        plugin_id: &str,
    ) -> Result<(), HostError> {
        let key = (project_id.to_owned(), plugin_id.to_owned());
        let previous_usage = self.project_usage.remove(&key);
        let previous_selection = self.project_versions.remove(&key);
        if let Err(error) = self.persist_state() {
            if let Some(version) = previous_usage {
                self.project_usage.insert(key.clone(), version);
            }
            if let Some(version) = previous_selection {
                self.project_versions.insert(key, version);
            }
            return Err(error);
        }
        Ok(())
    }

    pub fn project_uses_version(&self, plugin_id: &str, version: &str) -> bool {
        self.project_usage
            .iter()
            .any(|((_, id), selected)| id == plugin_id && selected == version)
    }
    pub fn install_development_dir(
        &mut self,
        root: impl AsRef<Path>,
    ) -> Result<&CatalogEntry, HostError> {
        let id = {
            let entry = self.catalog.install_development_dir(root)?;
            entry.manifest.id.clone()
        };
        let manifest = self
            .catalog
            .get(&id)
            .expect("catalog inserted entry")
            .manifest
            .clone();
        if let Err(error) = self.namespaces.register_manifest(&manifest) {
            self.catalog.remove(&manifest.id);
            return Err(error);
        }
        Ok(self.catalog.get(&id).expect("catalog entry retained"))
    }

    /// Verify and atomically install a `.wbplugin`, then register its verified
    /// manifest for the existing Phase 5 runtime authority.
    pub fn install_package(
        &mut self,
        archive: impl AsRef<Path>,
        install_root: impl AsRef<Path>,
        limits: ArchiveLimits,
        policy: VerificationPolicy,
    ) -> Result<PluginPackage, HostError> {
        let package = self
            .packages
            .install(archive, install_root, limits, policy)
            .map_err(|e| HostError(e.to_string()))?;
        if self.catalog.get(&package.manifest.id).is_some()
            && self.packages.list(&package.manifest.id).count() == 1
        {
            let _ = self
                .packages
                .remove_version(&package.manifest.id, &package.manifest.version);
            return Err(HostError(
                "plugin package ID conflicts with a bundled or development plugin".into(),
            ));
        }
        let active = self
            .packages
            .active_candidate(&package.manifest.id)
            .ok_or_else(|| HostError("installed package disappeared from the catalog".into()))?
            .clone();
        let active_manifest = parse_manifest(
            &fs::read_to_string(active.root.join("manifest.json")).map_err(io_error)?,
        )
        .map_err(|error| HostError(error.to_string()))?;
        let entry = CatalogEntry {
            manifest: active_manifest.clone(),
            package_root: active.root,
            digest: active.digest,
            embedded_wasm: None,
        };
        let previous_entry = self.catalog.get(&package.manifest.id).cloned();
        let inserted_runtime_entry = self.catalog.get(&package.manifest.id).is_none();
        if inserted_runtime_entry {
            if let Err(error) = self.catalog.insert_for_test(entry) {
                let _ = self
                    .packages
                    .remove_version(&package.manifest.id, &package.manifest.version);
                return Err(error);
            }
        } else {
            self.catalog.replace_runtime_entry(entry)?;
        }
        if let Err(error) = self.namespaces.register_manifest(&active_manifest) {
            if inserted_runtime_entry {
                self.catalog.remove(&package.manifest.id);
            } else if let Some(previous) = previous_entry {
                let _ = self.catalog.replace_runtime_entry(previous);
            }
            let _ = self
                .packages
                .remove_version(&package.manifest.id, &package.manifest.version);
            return Err(error);
        }
        if let Err(error) = self.persist_state() {
            if inserted_runtime_entry {
                self.catalog.remove(&package.manifest.id);
            }
            let _ = self
                .packages
                .remove_version(&package.manifest.id, &package.manifest.version);
            return Err(error);
        }
        Ok(package)
    }

    pub fn plan_upgrade(
        &self,
        plugin_id: &str,
        version: &str,
        project_id: &str,
        current_data_version: u32,
    ) -> Result<UpgradePlan, HostError> {
        let target = self
            .packages
            .get(plugin_id, version)
            .ok_or_else(|| HostError("target plugin version is not installed".into()))?;
        let target_json =
            fs::read_to_string(target.root.join("manifest.json")).map_err(io_error)?;
        let target_manifest = parse_manifest(&target_json).map_err(|e| HostError(e.to_string()))?;
        let previous = self
            .runtime_entry(project_id, plugin_id)
            .map(|entry| (entry.manifest.clone(), current_data_version));
        plan_upgrade(
            previous
                .as_ref()
                .map(|(manifest, version)| (manifest, *version)),
            &target_manifest,
        )
        .map_err(|e| HostError(format!("upgrade plan for {project_id}: {e}")))
    }

    /// Switch runtime selection to a retained, already verified version. Data
    /// rollback remains backup-driven; callers must restore the plan's backup
    /// before reactivating when the target cannot preserve the stored version.
    pub fn rollback_plugin(
        &mut self,
        project_id: &str,
        plugin_id: &str,
        target_version: &str,
        current_data_version: u32,
    ) -> Result<RollbackPlan, HostError> {
        let target = self
            .packages
            .get(plugin_id, target_version)
            .ok_or_else(|| HostError("rollback version is not retained".into()))?
            .clone();
        let active = self
            .runtime_entry(project_id, plugin_id)
            .ok_or_else(|| HostError("plugin is not selected in the runtime catalog".into()))?;
        let target_manifest = parse_manifest(
            &fs::read_to_string(target.root.join("manifest.json")).map_err(io_error)?,
        )
        .map_err(|e| HostError(e.to_string()))?;
        let active_installed = InstalledVersion {
            plugin_id: active.manifest.id.clone(),
            version: active.manifest.version.clone(),
            digest: active.digest.clone(),
            root: active.package_root.clone(),
            publisher: active.manifest.publisher.clone(),
            signed: true,
            installed_at: 0,
            unsigned_consent: false,
        };
        let plan = plan_rollback(
            plugin_id,
            &active_installed,
            &target,
            current_data_version,
            &target_manifest,
        )
        .map_err(|e| HostError(e.to_string()))?;
        let previous_selection = self
            .project_versions
            .get(&(project_id.into(), plugin_id.into()))
            .cloned();
        self.deactivate_bundled(project_id, plugin_id);
        self.select_project_version(project_id, plugin_id, target_version)?;
        if let Err(error) = self.activate_bundled(project_id, plugin_id) {
            if let Some(previous) = previous_selection {
                self.project_versions
                    .insert((project_id.into(), plugin_id.into()), previous);
            } else {
                self.project_versions
                    .remove(&(project_id.into(), plugin_id.into()));
            }
            let _ = self.activate_bundled(project_id, plugin_id);
            return Err(error);
        }
        Ok(plan)
    }

    pub fn uninstall_code(&mut self, plugin_id: &str, version: &str) -> Result<(), HostError> {
        if self.project_uses_version(plugin_id, version) {
            return Err(HostError(
                "cannot uninstall code while that version is selected in a project".into(),
            ));
        }
        self.packages
            .remove_version(plugin_id, version)
            .map_err(|e| HostError(e.to_string()))?;
        if let Some(active) = self.packages.active_candidate(plugin_id).cloned() {
            let manifest = parse_manifest(
                &fs::read_to_string(active.root.join("manifest.json")).map_err(io_error)?,
            )
            .map_err(|error| HostError(error.to_string()))?;
            self.catalog.replace_runtime_entry(CatalogEntry {
                manifest,
                package_root: active.root,
                digest: active.digest,
                embedded_wasm: None,
            })?;
        } else if self
            .catalog
            .get(plugin_id)
            .is_some_and(|entry| !entry.package_root.as_os_str().is_empty())
        {
            self.catalog.remove(plugin_id);
        }
        self.persist_state()
    }

    pub fn register_bundled_json(&mut self, json: &str) -> Result<&CatalogEntry, HostError> {
        self.register_bundled_json_with_wasm(json, None)
    }

    pub fn register_bundled_json_with_wasm(
        &mut self,
        json: &str,
        embedded_wasm: Option<&[u8]>,
    ) -> Result<&CatalogEntry, HostError> {
        let entry = self
            .catalog
            .register_bundled_json_with_wasm(json, embedded_wasm)?;
        let manifest = entry.manifest.clone();
        if let Err(error) = self.namespaces.register_manifest(&manifest) {
            self.catalog.remove(&manifest.id);
            return Err(error);
        }
        Ok(self
            .catalog
            .get(&manifest.id)
            .expect("bundled catalog entry retained"))
    }

    pub fn select_project_version(
        &mut self,
        project_id: &str,
        plugin_id: &str,
        version: &str,
    ) -> Result<(), HostError> {
        if self.packages.get(plugin_id, version).is_none()
            && self
                .catalog
                .get(plugin_id)
                .is_none_or(|entry| entry.manifest.version != version)
        {
            return Err(HostError("plugin version is not installed".into()));
        }
        self.revoke_plugin(project_id, plugin_id);
        let key = (project_id.into(), plugin_id.into());
        let previous = self.project_versions.insert(key.clone(), version.into());
        if let Err(error) = self.record_project_usage(project_id, plugin_id, version) {
            match previous {
                Some(version) => {
                    self.project_versions.insert(key, version);
                }
                None => {
                    self.project_versions.remove(&key);
                }
            }
            return Err(error);
        }
        Ok(())
    }

    pub fn selected_project_version(&self, project_id: &str, plugin_id: &str) -> Option<String> {
        self.project_versions
            .get(&(project_id.into(), plugin_id.into()))
            .cloned()
            .or_else(|| {
                self.project_usage
                    .get(&(project_id.into(), plugin_id.into()))
                    .cloned()
            })
            .or_else(|| {
                self.catalog
                    .get(plugin_id)
                    .map(|entry| entry.manifest.version.clone())
            })
    }

    pub fn runtime_entry(&self, project_id: &str, plugin_id: &str) -> Option<CatalogEntry> {
        let selected = self.selected_project_version(project_id, plugin_id)?;
        if let Some(package) = self.packages.get(plugin_id, &selected) {
            let manifest =
                parse_manifest(&fs::read_to_string(package.root.join("manifest.json")).ok()?)
                    .ok()?;
            return Some(CatalogEntry {
                manifest,
                package_root: package.root.clone(),
                digest: package.digest.clone(),
                embedded_wasm: None,
            });
        }
        self.catalog.get(plugin_id).cloned()
    }

    /// Return a host-rendered view only when the plugin is active and its
    /// manifest/grants authorize every data component used by that view.
    pub fn host_view(
        &self,
        project_id: &str,
        plugin_id: &str,
        view_id: &str,
    ) -> Result<View, HostError> {
        if self.lifecycle.state(project_id, plugin_id).state != LifecycleState::Active {
            return Err(HostError("plugin is not active".into()));
        }
        let entry = self
            .runtime_entry(project_id, plugin_id)
            .ok_or_else(|| HostError("plugin is not installed".into()))?;
        let view = entry
            .manifest
            .views
            .iter()
            .find(|view| view.id == view_id)
            .cloned()
            .ok_or_else(|| HostError("plugin host view is not declared".into()))?;
        let grants = self.grants.get(project_id, plugin_id);
        for component in &view.components {
            match component {
                ViewComponent::EntityList { .. } | ViewComponent::EntityDetail { .. }
                    if !grants.iter().any(|grant| grant == "entity.read") =>
                {
                    return Err(HostError(
                        "host entity view requires an entity.read grant".into(),
                    ));
                }
                ViewComponent::FieldForm { editable: true, .. }
                    if !grants.iter().any(|grant| grant == "field.write:self") =>
                {
                    return Err(HostError(
                        "host editable field form requires a field.write:self grant".into(),
                    ));
                }
                ViewComponent::FieldForm { .. }
                    if !grants.iter().any(|grant| grant == "field.read:self") =>
                {
                    return Err(HostError(
                        "host field form requires a field.read:self grant".into(),
                    ));
                }
                _ => {}
            }
        }
        Ok(view)
    }

    pub fn bootstrap(
        &mut self,
        plugin_id: &str,
        project_id: &str,
        origin: &str,
    ) -> Result<Session, HostError> {
        let entry = self
            .runtime_entry(project_id, plugin_id)
            .ok_or_else(|| HostError("plugin is not installed".into()))?;
        self.ensure_first_party_bundled_grants(project_id, plugin_id)?;
        let grants = self.grants.get(project_id, plugin_id);
        Ok(self
            .sessions
            .issue(&entry, project_id, origin, grants, self.session_ttl))
    }
    pub fn revoke_plugin(&mut self, project_id: &str, plugin_id: &str) {
        self.wasm.stop(project_id, plugin_id);
        self.sessions.revoke_plugin(project_id, plugin_id);
        self.lifecycle.deactivate(project_id, plugin_id);
        self.events.unsubscribe_plugin(project_id, plugin_id);
        let drained = self
            .services
            .deactivate_plugin(plugin_id, Duration::from_millis(250));
        if !drained {
            self.lifecycle.quarantine(
                project_id,
                plugin_id,
                "provider work did not drain before deactivation deadline",
            );
        }
        self.declarations.unregister_plugin(project_id, plugin_id);
    }
    pub fn activate_bundled(
        &mut self,
        project_id: &str,
        plugin_id: &str,
    ) -> Result<DependencyPlan, HostError> {
        let plan = DependencyResolver::resolve(&self.catalog, plugin_id)?;
        let mut activated: Vec<String> = Vec::new();
        for activation_id in &plan.order {
            if self.lifecycle.state(project_id, activation_id).state == LifecycleState::Active {
                continue;
            }
            if let Err(error) = self.lifecycle.begin_activation(project_id, activation_id) {
                for previous in activated.into_iter().rev() {
                    self.deactivate_bundled(project_id, &previous);
                }
                return Err(error);
            }
            match self.ensure_bundled_session(activation_id, project_id) {
                Ok(_) => {
                    let (package_root, wasm_entry, embedded_wasm) = {
                        let entry = self
                            .runtime_entry(project_id, activation_id)
                            .expect("resolved plugin remains installed");
                        (
                            entry.package_root.clone(),
                            entry.manifest.entrypoints.wasm.clone(),
                            entry.embedded_wasm.clone(),
                        )
                    };
                    if let Err(error) = self.wasm.start_with_bytes(
                        project_id,
                        activation_id,
                        &package_root,
                        wasm_entry.as_deref(),
                        embedded_wasm.as_deref(),
                        WasmLimits::default(),
                    ) {
                        self.lifecycle.activation_failed(
                            project_id,
                            activation_id,
                            &error.to_string(),
                        );
                        self.sessions.revoke_plugin(project_id, activation_id);
                        for previous in activated.into_iter().rev() {
                            self.deactivate_bundled(project_id, &previous);
                        }
                        return Err(HostError(format!("WASM runtime failed: {error:?}")));
                    }
                    self.lifecycle
                        .activation_succeeded(project_id, activation_id);
                    if let Err(error) =
                        self.register_manifest_declarations(project_id, activation_id)
                    {
                        self.revoke_plugin(project_id, activation_id);
                        self.lifecycle.activation_failed(
                            project_id,
                            activation_id,
                            &error.to_string(),
                        );
                        for previous in activated.into_iter().rev() {
                            self.deactivate_bundled(project_id, &previous);
                        }
                        return Err(error);
                    }
                    self.services.resume_plugin(activation_id);
                    activated.push(activation_id.clone());
                }
                Err(error) => {
                    self.lifecycle
                        .activation_failed(project_id, activation_id, &error.to_string());
                    self.sessions.revoke_plugin(project_id, activation_id);
                    for previous in activated.into_iter().rev() {
                        self.deactivate_bundled(project_id, &previous);
                    }
                    return Err(error);
                }
            }
        }
        Ok(plan)
    }

    fn register_manifest_declarations(
        &mut self,
        project_id: &str,
        plugin_id: &str,
    ) -> Result<(), HostError> {
        let entry = self
            .runtime_entry(project_id, plugin_id)
            .ok_or_else(|| HostError("plugin runtime version is missing".into()))?;
        self.declarations
            .register(project_id, plugin_id, &entry.manifest)?;
        let grants = self.grants.get(project_id, plugin_id);
        for event in &entry.manifest.events.subscribes {
            let required = format!("event.subscribe:{}@{}", event.name, event.version);
            if grants
                .iter()
                .any(|grant| capability_matches(grant, &required))
            {
                self.events
                    .subscribe(project_id, plugin_id, &event.name, event.version);
            }
        }
        let runtime = self.wasm.runtime(project_id, plugin_id);
        for service in &entry.manifest.services.provides {
            let required = format!("service.provide:{}@{}", service.name, service.major);
            if !grants
                .iter()
                .any(|grant| capability_matches(grant, &required))
            {
                continue;
            }
            if self.services.has_provider(&service.name, service.major) {
                continue;
            }
            let runtime = runtime.clone();
            self.register_declared_service_provider(
                plugin_id,
                &service.name,
                service.major,
                Arc::new(move |_request| {
                    let runtime = runtime
                        .as_ref()
                        .ok_or_else(|| HostError("service provider unavailable".into()))?;
                    let value = runtime
                        .invoke_service(&_request.payload)
                        .map_err(|error| HostError(format!("WASM service failed: {error}")))?;
                    Ok(value)
                }),
            )?;
        }
        Ok(())
    }
    pub fn deactivate_bundled(&mut self, project_id: &str, plugin_id: &str) {
        if let Ok(plan) = DependencyResolver::resolve(&self.catalog, plugin_id) {
            for dependency in plan.order.into_iter().rev() {
                self.revoke_plugin(project_id, &dependency);
            }
        } else {
            self.revoke_plugin(project_id, plugin_id);
        }
    }
    pub fn deactivate_project(&mut self, project_id: &str) {
        let plugin_ids = self
            .catalog
            .list()
            .map(|entry| entry.manifest.id.clone())
            .collect::<Vec<_>>();
        for plugin_id in plugin_ids {
            self.deactivate_bundled(project_id, &plugin_id);
        }
    }
    pub fn retry_plugin(&mut self, project_id: &str, plugin_id: &str) {
        self.lifecycle.clear_quarantine(project_id, plugin_id);
    }
    pub fn publish_event(
        &mut self,
        source_plugin: &str,
        project_id: &str,
        name: &str,
        version: u32,
        payload: serde_json::Value,
    ) -> Result<PublishResult, HostError> {
        self.authorize_bundled(
            source_plugin,
            project_id,
            "event.publish",
            serde_json::json!({"type": format!("{name}@{version}")}),
        )?;
        self.events
            .publish(project_id, source_plugin, name, version, payload)
    }
    pub fn publish_core_event(
        &mut self,
        project_id: &str,
        name: &str,
        version: u32,
        payload: serde_json::Value,
    ) -> Result<PublishResult, HostError> {
        self.events
            .publish(project_id, "daena.core", name, version, payload)
    }
    pub fn subscribe_event(
        &mut self,
        plugin_id: &str,
        project_id: &str,
        name: &str,
        version: u32,
    ) -> Result<(), HostError> {
        self.authorize_bundled(
            plugin_id,
            project_id,
            "event.subscribe",
            serde_json::json!({"type": format!("{name}@{version}")}),
        )?;
        self.events.subscribe(project_id, plugin_id, name, version);
        Ok(())
    }
    pub fn poll_events(
        &mut self,
        plugin_id: &str,
        project_id: &str,
        name: &str,
        version: u32,
    ) -> Result<Vec<EventEnvelope>, HostError> {
        self.authorize_bundled(
            plugin_id,
            project_id,
            "event.subscribe",
            serde_json::json!({"type": format!("{name}@{version}")}),
        )?;
        Ok(self.events.drain(project_id, plugin_id, name, version))
    }
    pub fn call_service(
        &mut self,
        consumer_id: &str,
        project_id: &str,
        name: &str,
        major: u32,
        payload: serde_json::Value,
        deadline: Duration,
    ) -> Result<serde_json::Value, HostError> {
        self.authorize_bundled(
            consumer_id,
            project_id,
            "service.call",
            serde_json::json!({"name": name, "major": major}),
        )?;
        self.services
            .call(consumer_id, name, major, payload, deadline)
    }
    pub fn publish_event_authorized(
        &mut self,
        source_plugin: &str,
        project_id: &str,
        name: &str,
        version: u32,
        payload: serde_json::Value,
    ) -> Result<PublishResult, HostError> {
        self.events
            .publish(project_id, source_plugin, name, version, payload)
    }
    pub fn subscribe_event_authorized(
        &mut self,
        plugin_id: &str,
        project_id: &str,
        name: &str,
        version: u32,
    ) -> Result<(), HostError> {
        self.events.subscribe(project_id, plugin_id, name, version);
        Ok(())
    }
    pub fn poll_events_authorized(
        &mut self,
        plugin_id: &str,
        project_id: &str,
        name: &str,
        version: u32,
    ) -> Result<Vec<EventEnvelope>, HostError> {
        Ok(self.events.drain(project_id, plugin_id, name, version))
    }
    pub fn call_service_authorized(
        &mut self,
        consumer_id: &str,
        _project_id: &str,
        name: &str,
        major: u32,
        payload: serde_json::Value,
        deadline: Duration,
    ) -> Result<serde_json::Value, HostError> {
        self.services
            .call(consumer_id, name, major, payload, deadline)
    }

    pub fn register_declared_service_provider(
        &mut self,
        plugin_id: &str,
        name: &str,
        major: u32,
        handler: ServiceHandler,
    ) -> Result<(), HostError> {
        let entry = self
            .catalog
            .get(plugin_id)
            .ok_or_else(|| HostError("service provider plugin is not installed".into()))?;
        if !entry
            .manifest
            .services
            .provides
            .iter()
            .any(|service| service.name == name && service.major == major)
        {
            return Err(HostError(format!(
                "service provider is not declared: {name}@{major}"
            )));
        }
        self.services.register(plugin_id, name, major, handler)
    }

    pub fn invoke_command(
        &self,
        project_id: &str,
        plugin_id: &str,
        view_id: &str,
        command_id: &str,
    ) -> Result<CommandAction, HostError> {
        self.invoke_command_with_payload(
            project_id,
            plugin_id,
            view_id,
            command_id,
            serde_json::json!({}),
        )
    }

    pub fn invoke_broker_command(
        &self,
        project_id: &str,
        plugin_id: &str,
        command_id: &str,
        payload: serde_json::Value,
    ) -> Result<CommandAction, HostError> {
        self.invoke_command_for_exposure(
            project_id,
            plugin_id,
            None,
            command_id,
            payload,
            CommandExposure::Broker,
        )
    }

    pub fn invoke_command_with_payload(
        &self,
        project_id: &str,
        plugin_id: &str,
        view_id: &str,
        command_id: &str,
        payload: serde_json::Value,
    ) -> Result<CommandAction, HostError> {
        self.invoke_command_for_exposure(
            project_id,
            plugin_id,
            Some(view_id),
            command_id,
            payload,
            CommandExposure::View,
        )
    }

    fn invoke_command_for_exposure(
        &self,
        project_id: &str,
        plugin_id: &str,
        view_id: Option<&str>,
        command_id: &str,
        payload: serde_json::Value,
        exposure: CommandExposure,
    ) -> Result<CommandAction, HostError> {
        if self.lifecycle.state(project_id, plugin_id).state != LifecycleState::Active {
            return Err(HostError("plugin is not active".into()));
        }
        let command = self
            .declarations
            .command(project_id, plugin_id, command_id)
            .ok_or_else(|| HostError("plugin command is not declared".into()))?;
        if !command_exposes(&command, exposure.clone()) {
            return Err(HostError(format!(
                "plugin command is not exposed to {}",
                match exposure {
                    CommandExposure::View => "views",
                    CommandExposure::Broker => "the broker",
                }
            )));
        }
        let grants = self.grants.get(project_id, plugin_id);
        if command
            .capabilities
            .iter()
            .any(|required| !grants.iter().any(|grant| grant == required))
        {
            return Err(HostError("plugin command capability is not granted".into()));
        }
        if let Some(schema) = &command.input {
            validate_command_value(schema, &payload)
                .map_err(|error| HostError(format!("invalid plugin command input: {error}")))?;
        } else if !payload.as_object().is_some_and(serde_json::Map::is_empty) {
            return Err(HostError(
                "plugin command does not accept input properties".into(),
            ));
        }
        if let Some(view_id) = view_id {
            let view = self
                .declarations
                .views(project_id, plugin_id)
                .into_iter()
                .find(|view| view.id == view_id)
                .ok_or_else(|| HostError("plugin command view is not declared".into()))?;
            let referenced = view.components.iter().any(|component| {
                matches!(component, ViewComponent::Button { command, .. } if command == command_id)
            });
            if !referenced {
                return Err(HostError(
                    "plugin command is not exposed by this view".into(),
                ));
            }
        }
        let action = command
            .action
            .ok_or_else(|| HostError("plugin command has no executable action".into()))?;
        if let Some(schema) = &command.output {
            let output = serde_json::to_value(&action)
                .map_err(|error| HostError(format!("plugin command output is invalid: {error}")))?;
            validate_command_value(schema, &output)
                .map_err(|error| HostError(format!("invalid plugin command output: {error}")))?;
        }
        Ok(action)
    }
    pub fn ensure_bundled_session(
        &mut self,
        plugin_id: &str,
        project_id: &str,
    ) -> Result<Session, HostError> {
        let origin = format!("bundled:{plugin_id}");
        if let Some(session) = self.sessions.find_active(plugin_id, project_id, &origin) {
            return Ok(session);
        }
        self.runtime_entry(project_id, plugin_id)
            .ok_or_else(|| HostError("bundled plugin is not registered".into()))?;
        // A missing grant is an explicit deny-all state. Activation callers
        // must obtain consent through `grant_capabilities`; runtime bootstrap
        // must never silently turn a manifest request into authority.
        self.bootstrap(plugin_id, project_id, &origin)
    }
    pub fn authorize_bundled(
        &mut self,
        plugin_id: &str,
        project_id: &str,
        method: &str,
        payload: serde_json::Value,
    ) -> Result<(), HostError> {
        if self.lifecycle.state(project_id, plugin_id).state != LifecycleState::Active {
            return Err(HostError("plugin is not active".into()));
        }
        let session = self.ensure_bundled_session(plugin_id, project_id)?;
        let request = RpcRequest {
            rpc_version: RPC_VERSION,
            session_id: session.id,
            request_id: "bundled".into(),
            method: method.into(),
            payload,
        };
        self.authorize(&session.origin, &request)
            .map_err(|error| HostError(format!("{}: {}", error.code, error.message)))
    }

    pub fn register_ai_request(
        &mut self,
        request_id: &str,
        project_id: &str,
        plugin_id: &str,
        session_id: &str,
        operation: &str,
        output_contract: Option<serde_json::Value>,
    ) {
        if self.ai_requests.len() >= 256 {
            if let Some(oldest) = self.ai_requests.keys().next().cloned() {
                self.ai_requests.remove(&oldest);
            }
        }
        self.ai_requests.insert(
            request_id.to_string(),
            (
                project_id.to_string(),
                plugin_id.to_string(),
                session_id.to_string(),
                operation.to_string(),
                output_contract,
            ),
        );
    }

    pub fn authorize_ai_request(
        &self,
        request_id: &str,
        project_id: &str,
        plugin_id: &str,
        session_id: &str,
    ) -> Result<String, HostError> {
        let Some((bound_project, bound_plugin, bound_session, operation, _contract)) =
            self.ai_requests.get(request_id)
        else {
            return Err(HostError("AI request does not exist".into()));
        };
        if bound_project != project_id || bound_plugin != plugin_id || bound_session != session_id {
            return Err(HostError(
                "AI request is not bound to this plugin session".into(),
            ));
        }
        Ok(operation.clone())
    }

    pub fn ai_contract(&self, request_id: &str) -> Option<serde_json::Value> {
        self.ai_requests
            .get(request_id)
            .and_then(|entry| entry.4.clone())
    }

    pub fn ai_request_ids_for(&self, project_id: &str, plugin_id: Option<&str>) -> Vec<String> {
        self.ai_requests
            .iter()
            .filter_map(|(id, (project, plugin, ..))| {
                (project == project_id && plugin_id.is_none_or(|expected| expected == plugin))
                    .then(|| id.clone())
            })
            .collect()
    }

    pub fn remove_ai_request(&mut self, request_id: &str) {
        self.ai_requests.remove(request_id);
    }
    pub fn rpc(&self, origin: &str, request: &RpcRequest) -> RpcResponse {
        let result = self
            .authorize_rpc(origin, request)
            .map(|_| serde_json::json!({"authorized": true}));
        match result {
            Ok(result) => RpcResponse {
                rpc_version: RPC_VERSION,
                request_id: request.request_id.clone(),
                ok: true,
                result: Some(result),
                error: None,
            },
            Err(error) => RpcResponse {
                rpc_version: RPC_VERSION,
                request_id: request.request_id.clone(),
                ok: false,
                result: None,
                error: Some(error),
            },
        }
    }
    pub fn authorize_rpc(&self, origin: &str, request: &RpcRequest) -> Result<Session, RpcError> {
        runtime::validate_bridge_request(request)
            .map_err(|message| rpc_error("payload.invalid", message, false))?;
        self.authorize(origin, request)?;
        self.sessions.valid(&request.session_id, origin).cloned()
    }
    fn authorize(&self, origin: &str, request: &RpcRequest) -> Result<(), RpcError> {
        if request.rpc_version != RPC_VERSION {
            return Err(rpc_error("rpc.version", "unsupported RPC version", false));
        }
        let session = self.sessions.valid(&request.session_id, origin)?;
        let manifest = self
            .runtime_entry(&session.project_id, &session.plugin_id)
            .ok_or_else(|| rpc_error("plugin.missing", "plugin is not installed", false))?
            .manifest;
        validate_declared_resource(&manifest, &request.method, &request.payload)?;
        validate_schema_resource(
            &manifest,
            &request.method,
            &request.payload,
            session,
            &self.namespaces,
        )?;
        let capabilities =
            required_capabilities(&request.method, &request.payload, session, &self.namespaces)?;
        // Grants are project state, not session state. A webview can remain
        // mounted while the shell refreshes consent; authorize against the
        // current persisted decision instead of a stale bootstrap snapshot.
        let grants = self.grants.get(&session.project_id, &session.plugin_id);
        let missing = capabilities
            .iter()
            .filter(|capability| {
                !capability.split('|').any(|candidate| {
                    grants
                        .iter()
                        .any(|grant| capability_matches(grant, candidate))
                })
            })
            .map(String::as_str)
            .collect::<Vec<_>>();
        if !missing.is_empty() {
            return Err(rpc_error(
                "capability.denied",
                format!("operation is not granted: {}", missing.join(", ")),
                false,
            ));
        }
        Ok(())
    }
}

fn validate_schema_resource(
    manifest: &PluginManifest,
    method: &str,
    payload: &serde_json::Value,
    session: &Session,
    namespaces: &NamespaceOwnership,
) -> Result<(), RpcError> {
    let validate_field = |namespace: &str,
                          key: &str,
                          value: Option<&serde_json::Value>,
                          entity_type: Option<&str>| {
        let owned = namespaces.owner(namespace) == Some(session.plugin_id.as_str());
        let reading = matches!(method, "field.read" | "field.list");
        if !owned {
            if !reading || !namespaces.field_is_shared(namespace, key) {
                return Err(rpc_error(
                    "namespace.denied",
                    "plugin may only read explicitly shared fields",
                    false,
                ));
            }
            // The owner manifest is authoritative for a shared field. A
            // foreign plugin cannot smuggle a schema or write value through
            // its own manifest by merely naming the same namespace/key.
            if value.is_some() || entity_type.is_some() {
                return Err(rpc_error(
                    "field.readonly",
                    "shared fields are read-only",
                    false,
                ));
            }
            return Ok(());
        }
        let field = manifest
            .schemas
            .iter()
            .find(|schema| schema.namespace == namespace)
            .and_then(|schema| schema.fields.iter().find(|field| field.key == key))
            .ok_or_else(|| rpc_error("schema.undeclared", "field is not declared", false))?;
        if let Some(entity_type) = entity_type {
            let schema = manifest
                .schemas
                .iter()
                .find(|schema| schema.namespace == namespace)
                .ok_or_else(|| rpc_error("schema.undeclared", "schema is not declared", false))?;
            if !schema.entity_types.iter().any(|kind| kind == entity_type)
                || field
                    .entity_types
                    .as_ref()
                    .is_some_and(|types| !types.iter().any(|kind| kind == entity_type))
            {
                return Err(rpc_error(
                    "schema.inapplicable",
                    "field does not apply to the entity type",
                    false,
                ));
            }
        }
        if let Some(value) = value {
            if !field_value_matches(field, value) {
                return Err(rpc_error(
                    "schema.invalid",
                    "field value does not match its declared schema",
                    false,
                ));
            }
        }
        Ok(())
    };

    match method {
        "entity.create" => {
            let entity_type = payload.get("type").and_then(serde_json::Value::as_str);
            if let Some(entity_type) = entity_type {
                if !manifest
                    .schemas
                    .iter()
                    .any(|schema| schema.entity_types.iter().any(|kind| kind == entity_type))
                {
                    return Err(rpc_error(
                        "schema.undeclared",
                        "entity type is not declared",
                        false,
                    ));
                }
            }
            if let Some(fields) = payload.get("fields") {
                for field in fields.as_array().ok_or_else(|| {
                    rpc_error("payload.invalid", "entity fields must be an array", false)
                })? {
                    let namespace = field
                        .get("namespace")
                        .and_then(serde_json::Value::as_str)
                        .ok_or_else(|| {
                            rpc_error("payload.invalid", "field namespace is required", false)
                        })?;
                    let key = field
                        .get("key")
                        .and_then(serde_json::Value::as_str)
                        .ok_or_else(|| {
                            rpc_error("payload.invalid", "field key is required", false)
                        })?;
                    validate_field(namespace, key, field.get("value"), entity_type)?;
                }
            }
            if let Some(relationships) = payload.get("relationships") {
                for relationship in relationships.as_array().ok_or_else(|| {
                    rpc_error(
                        "payload.invalid",
                        "entity relationships must be an array",
                        false,
                    )
                })? {
                    let relationship_type = relationship
                        .get("relationship_type")
                        .and_then(serde_json::Value::as_str)
                        .ok_or_else(|| {
                            rpc_error("payload.invalid", "relationship type is required", false)
                        })?;
                    match namespaces.relationship_owner(relationship_type) {
                        None => {
                            return Err(rpc_error(
                                "relationship.undeclared",
                                "relationship type is not registered",
                                false,
                            ));
                        }
                        Some(owner) if owner != session.plugin_id => {
                            return Err(rpc_error(
                                "relationship.denied",
                                "plugin does not own relationship type",
                                false,
                            ));
                        }
                        Some(_) => {}
                    }
                }
            }
        }
        "entity.update" => {
            if let Some(entity_type) = payload.get("type").and_then(serde_json::Value::as_str) {
                if !manifest
                    .schemas
                    .iter()
                    .any(|schema| schema.entity_types.iter().any(|kind| kind == entity_type))
                {
                    return Err(rpc_error(
                        "schema.undeclared",
                        "entity type is not declared",
                        false,
                    ));
                }
            }
        }
        "field.read" | "field.list" | "field.set" => {
            let namespace = payload
                .get("namespace")
                .and_then(serde_json::Value::as_str)
                .ok_or_else(|| {
                    rpc_error(
                        "payload.invalid",
                        "field operation requires namespace",
                        false,
                    )
                })?;
            if let Some(key) = payload.get("key").and_then(serde_json::Value::as_str) {
                validate_field(namespace, key, payload.get("value"), None)?;
            } else if namespaces.owner(namespace) != Some(session.plugin_id.as_str())
                && (!matches!(method, "field.read" | "field.list")
                    || !namespaces.namespace_has_shared_fields(namespace))
            {
                return Err(rpc_error(
                    "namespace.denied",
                    "plugin may only read explicitly shared fields",
                    false,
                ));
            }
        }
        "asset.list" | "asset.register" | "asset.read.begin" | "asset.replace.begin" => {
            let namespace = payload
                .get("namespace")
                .and_then(serde_json::Value::as_str)
                .ok_or_else(|| {
                    rpc_error(
                        "payload.invalid",
                        "asset operation requires namespace",
                        false,
                    )
                })?;
            if namespaces.owner(namespace) != Some(session.plugin_id.as_str()) {
                return Err(rpc_error(
                    "namespace.denied",
                    "plugin does not own namespace",
                    false,
                ));
            }
        }
        "relationship.create" => {
            let relationship_type = payload
                .get("relationship_type")
                .and_then(serde_json::Value::as_str)
                .ok_or_else(|| {
                    rpc_error("payload.invalid", "relationship type is required", false)
                })?;
            match namespaces.relationship_owner(relationship_type) {
                None => {
                    return Err(rpc_error(
                        "relationship.undeclared",
                        "relationship type is not registered",
                        false,
                    ));
                }
                Some(owner) if owner != session.plugin_id => {
                    return Err(rpc_error(
                        "relationship.denied",
                        "plugin does not own relationship type",
                        false,
                    ));
                }
                Some(_) => {}
            }
        }
        "relationship.delete" => {
            let stored_type = payload
                .get("__stored_relationship_type")
                .and_then(serde_json::Value::as_str)
                .ok_or_else(|| {
                    rpc_error(
                        "relationship.identity",
                        "relationship deletion requires the stored relationship identity",
                        false,
                    )
                })?;
            if let Some(requested_type) = payload
                .get("relationship_type")
                .and_then(serde_json::Value::as_str)
            {
                if requested_type != stored_type {
                    return Err(rpc_error(
                        "relationship.identity",
                        "relationship type does not match the stored relationship",
                        false,
                    ));
                }
            }
            match namespaces.relationship_owner(stored_type) {
                None => {
                    return Err(rpc_error(
                        "relationship.undeclared",
                        "stored relationship type is not registered",
                        false,
                    ));
                }
                Some(owner) if owner != session.plugin_id => {
                    return Err(rpc_error(
                        "relationship.denied",
                        "plugin does not own stored relationship type",
                        false,
                    ));
                }
                Some(_) => {}
            }
        }
        _ => {}
    }
    Ok(())
}

fn field_value_matches(
    field: &daena_plugin_api::FieldDefinition,
    value: &serde_json::Value,
) -> bool {
    if value.is_null() {
        return true;
    }
    match field.field_type.as_str() {
        "text" | "date" | "entity-ref" => {
            value.is_string() || (field.field_type == "date" && value.is_object())
        }
        "number" => value.is_number(),
        "boolean" => value.is_boolean(),
        "enum" => value
            .as_str()
            .zip(field.options.as_ref())
            .is_some_and(|(value, options)| options.iter().any(|option| option == value)),
        "relationship" => value.is_array(),
        _ => false,
    }
}

fn validate_declared_resource(
    manifest: &PluginManifest,
    method: &str,
    payload: &serde_json::Value,
) -> Result<(), RpcError> {
    match method {
        "event.publish" | "event.subscribe" | "event.poll" => {
            let event_type = payload
                .get("type")
                .and_then(serde_json::Value::as_str)
                .ok_or_else(|| {
                    rpc_error("payload.invalid", "event operations require type", false)
                })?;
            let events = if method == "event.publish" {
                &manifest.events.publishes
            } else {
                &manifest.events.subscribes
            };
            if !events
                .iter()
                .any(|event| format!("{}@{}", event.name, event.version) == event_type)
            {
                return Err(rpc_error(
                    "event.undeclared",
                    "event is not declared by the manifest",
                    false,
                ));
            }
        }
        "service.provide" | "service.call" => {
            let name = payload
                .get("name")
                .and_then(serde_json::Value::as_str)
                .ok_or_else(|| {
                    rpc_error("payload.invalid", "service operations require name", false)
                })?;
            let major = payload
                .get("major")
                .and_then(serde_json::Value::as_u64)
                .and_then(|major| u32::try_from(major).ok())
                .ok_or_else(|| {
                    rpc_error("payload.invalid", "service operations require major", false)
                })?;
            let services = if method == "service.provide" {
                &manifest.services.provides
            } else {
                &manifest.services.consumes
            };
            if !services
                .iter()
                .any(|service| service.name == name && service.major == major)
            {
                return Err(rpc_error(
                    "service.undeclared",
                    "service is not declared by the manifest",
                    false,
                ));
            }
        }
        _ => {}
    }
    Ok(())
}

fn required_capabilities(
    method: &str,
    payload: &serde_json::Value,
    session: &Session,
    namespaces: &NamespaceOwnership,
) -> Result<Vec<String>, RpcError> {
    let entry = RPC_METHOD_CATALOG
        .iter()
        .find(|entry| entry.name == method)
        .ok_or_else(|| {
            rpc_error(
                "method.unknown",
                "unknown or unavailable plugin method",
                false,
            )
        })?;
    let context = RpcAuthorizationContext {
        plugin_id: &session.plugin_id,
        namespaces,
    };
    entry.capability.resolve(payload, &context)
}

fn rpc_error(code: impl Into<String>, message: impl Into<String>, retryable: bool) -> RpcError {
    RpcError {
        code: code.into(),
        message: message.into(),
        retryable,
        details: None,
    }
}

#[cfg(test)]
mod tests;
