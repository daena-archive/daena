//! Host-owned Phase 2 plugin authority.
//!
//! This crate deliberately contains no Tauri or runtime implementation.  It
//! owns the facts needed to attribute and authorize a plugin request before a
//! future core service is called.

use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, SystemTime};
use worldbuilder_plugin_api::{
    parse_manifest, PluginManifest, RpcError, RpcRequest, RpcResponse, RPC_VERSION,
};

static SESSION_COUNTER: AtomicU64 = AtomicU64::new(1);

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

    pub fn register_bundled_json(&mut self, json: &str) -> Result<&CatalogEntry, HostError> {
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

#[derive(Debug, Default, Clone)]
pub struct GrantStore {
    grants: BTreeMap<(String, String), BTreeSet<String>>,
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
}

fn capability_matches(requested: &str, granted: &str) -> bool {
    requested == granted
        || requested
            .strip_suffix(":<type>")
            .is_some_and(|prefix| granted.starts_with(prefix))
        || requested
            .strip_suffix(":<name>")
            .is_some_and(|prefix| granted.starts_with(prefix))
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
}

impl NamespaceOwnership {
    pub fn register_manifest(&mut self, manifest: &PluginManifest) -> Result<(), HostError> {
        for namespace in &manifest.namespaces {
            if let Some(owner) = self.owners.get(namespace) {
                if owner != &manifest.id {
                    return Err(HostError(format!(
                        "namespace {namespace} is owned by {owner}"
                    )));
                }
            }
            self.owners.insert(namespace.clone(), manifest.id.clone());
        }
        Ok(())
    }
    pub fn owner(&self, namespace: &str) -> Option<&str> {
        self.owners.get(namespace).map(String::as_str)
    }
}

#[derive(Debug, Clone)]
pub struct PluginHost {
    pub catalog: PluginCatalog,
    pub grants: GrantStore,
    pub sessions: SessionRegistry,
    pub namespaces: NamespaceOwnership,
    pub session_ttl: Duration,
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
            grants: GrantStore::default(),
            sessions: SessionRegistry::default(),
            namespaces: NamespaceOwnership::default(),
            session_ttl: Duration::from_secs(15 * 60),
        }
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

    pub fn register_bundled_json(&mut self, json: &str) -> Result<&CatalogEntry, HostError> {
        let entry = self.catalog.register_bundled_json(json)?;
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
    pub fn bootstrap(
        &mut self,
        plugin_id: &str,
        project_id: &str,
        origin: &str,
    ) -> Result<Session, HostError> {
        let entry = self
            .catalog
            .get(plugin_id)
            .ok_or_else(|| HostError("plugin is not installed".into()))?;
        let grants = self.grants.get(project_id, plugin_id);
        Ok(self
            .sessions
            .issue(entry, project_id, origin, grants, self.session_ttl))
    }
    pub fn revoke_plugin(&mut self, project_id: &str, plugin_id: &str) {
        self.sessions.revoke_plugin(project_id, plugin_id);
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
        let entry = self
            .catalog
            .get(plugin_id)
            .ok_or_else(|| HostError("bundled plugin is not registered".into()))?
            .clone();
        if self.grants.get(project_id, plugin_id).is_empty() {
            self.grants.set(
                project_id,
                plugin_id,
                &entry.manifest.capabilities,
                entry.manifest.capabilities.iter().cloned().collect(),
            )?;
        }
        self.bootstrap(plugin_id, project_id, &origin)
    }
    pub fn authorize_bundled(
        &mut self,
        plugin_id: &str,
        project_id: &str,
        method: &str,
        payload: serde_json::Value,
    ) -> Result<(), HostError> {
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
    pub fn rpc(&self, origin: &str, request: &RpcRequest) -> RpcResponse {
        let result = self
            .authorize(origin, request)
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
    fn authorize(&self, origin: &str, request: &RpcRequest) -> Result<(), RpcError> {
        if request.rpc_version != RPC_VERSION {
            return Err(rpc_error("rpc.version", "unsupported RPC version", false));
        }
        let session = self.sessions.valid(&request.session_id, origin)?;
        let manifest = self
            .catalog
            .get(&session.plugin_id)
            .ok_or_else(|| rpc_error("plugin.missing", "plugin is not installed", false))?
            .manifest
            .clone();
        validate_declared_resource(&manifest, &request.method, &request.payload)?;
        let capabilities =
            required_capabilities(&request.method, &request.payload, session, &self.namespaces)?;
        if !capabilities
            .iter()
            .any(|capability| session.grants.contains(capability))
        {
            return Err(rpc_error(
                "capability.denied",
                "operation is not granted",
                false,
            ));
        }
        Ok(())
    }
}

fn validate_declared_resource(
    manifest: &PluginManifest,
    method: &str,
    payload: &serde_json::Value,
) -> Result<(), RpcError> {
    match method {
        "event.publish" | "event.subscribe" => {
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
            if !events.iter().any(|event| {
                event.name == event_type
                    || format!("{}@{}", event.name, event.version) == event_type
            }) {
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
            let services = if method == "service.provide" {
                &manifest.services.provides
            } else {
                &manifest.services.consumes
            };
            if !services.iter().any(|service| {
                service.name == name || format!("{}@{}", service.name, service.major) == name
            }) {
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
    match method {
        "entity.read" | "entity.list" | "entity.get" => Ok(vec!["entity.read".into()]),
        "entity.write" | "entity.create" | "entity.update" => Ok(vec!["entity.write".into()]),
        "entity.delete" => Ok(vec!["entity.delete".into()]),
        "document.read" | "document.list" => Ok(vec!["document.read".into()]),
        "document.write" | "document.save" => Ok(vec!["document.write".into()]),
        "relationship.read" | "relationship.list" => Ok(vec!["relationship.read".into()]),
        "relationship.write" | "relationship.create" => Ok(vec!["relationship.write".into()]),
        "search.query" => Ok(vec!["search.query".into()]),
        "asset.import" | "asset.register" => Ok(vec!["asset.import".into()]),
        "asset.read" => {
            ensure_owned_namespace(payload, session, namespaces)?;
            Ok(vec!["asset.read:self".into()])
        }
        "field.read" | "field.list" | "field.write" | "field.set" => {
            ensure_owned_namespace(payload, session, namespaces)?;
            Ok(vec![if matches!(method, "field.read" | "field.list") {
                "field.read:self".into()
            } else {
                "field.write:self".into()
            }])
        }
        "event.publish" | "event.subscribe" => {
            let event_type = payload
                .get("type")
                .and_then(serde_json::Value::as_str)
                .ok_or_else(|| {
                    rpc_error("payload.invalid", "event operations require type", false)
                })?;
            Ok(vec![format!(
                "event.{}:{event_type}",
                if method == "event.publish" {
                    "publish"
                } else {
                    "subscribe"
                }
            )])
        }
        "service.provide" | "service.call" => {
            let name = payload
                .get("name")
                .and_then(serde_json::Value::as_str)
                .ok_or_else(|| {
                    rpc_error("payload.invalid", "service operations require name", false)
                })?;
            Ok(vec![format!(
                "service.{}:{name}",
                if method == "service.provide" {
                    "provide"
                } else {
                    "call"
                }
            )])
        }
        _ => Err(rpc_error(
            "method.unknown",
            "unknown or unavailable plugin method",
            false,
        )),
    }
}

fn ensure_owned_namespace(
    payload: &serde_json::Value,
    session: &Session,
    namespaces: &NamespaceOwnership,
) -> Result<(), RpcError> {
    let namespace = payload
        .get("namespace")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| rpc_error("payload.invalid", "operation requires namespace", false))?;
    if namespaces.owner(namespace) != Some(session.plugin_id.as_str()) {
        return Err(rpc_error(
            "namespace.denied",
            "plugin does not own namespace",
            false,
        ));
    }
    Ok(())
}

fn rpc_error(code: &str, message: &str, retryable: bool) -> RpcError {
    RpcError {
        code: code.into(),
        message: message.into(),
        retryable,
        details: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    use worldbuilder_plugin_api::{Entrypoints, PluginKind};

    fn manifest(id: &str, namespace: &str) -> PluginManifest {
        PluginManifest {
            manifest_version: 1,
            id: id.into(),
            name: id.into(),
            version: "1.0.0".into(),
            publisher: "example".into(),
            host_api: ">=1.0.0 <2.0.0".into(),
            kind: PluginKind::Sandboxed,
            entrypoints: Entrypoints {
                ui: Some("dist/index.html".into()),
                wasm: None,
            },
            capabilities: vec![
                "entity.read".into(),
                "field.read:self".into(),
                "event.publish:<type>".into(),
                "service.call:<name>".into(),
            ],
            dependencies: BTreeMap::new(),
            namespaces: vec![namespace.into()],
            schemas: vec![],
            templates: vec![],
            views: vec![],
            commands: vec![],
            services: worldbuilder_plugin_api::Services {
                provides: vec![],
                consumes: vec![worldbuilder_plugin_api::Service {
                    name: "com.example.calculate".into(),
                    major: 1,
                }],
            },
            events: worldbuilder_plugin_api::Events {
                publishes: vec![worldbuilder_plugin_api::Event {
                    name: "worldbuilder.core/event".into(),
                    version: 1,
                }],
                subscribes: vec![],
            },
            migrations: vec![],
        }
    }
    fn host() -> PluginHost {
        let mut host = PluginHost::new();
        let entry = CatalogEntry {
            manifest: manifest("com.example.one", "one"),
            package_root: PathBuf::new(),
            digest: "a".repeat(64),
        };
        host.catalog.insert_for_test(entry.clone()).unwrap();
        host.namespaces.register_manifest(&entry.manifest).unwrap();
        host.grants
            .set(
                "project",
                "com.example.one",
                &entry.manifest.capabilities,
                ["entity.read".into(), "field.read:self".into()]
                    .into_iter()
                    .collect(),
            )
            .unwrap();
        host
    }
    #[test]
    fn digest_changes_when_file_changes() {
        let root = std::env::temp_dir().join(format!(
            "wb-plugin-{}",
            SESSION_COUNTER.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(root.join("dist")).unwrap();
        fs::write(root.join("manifest.json"), b"{}").unwrap();
        fs::write(root.join("dist/index.html"), b"one").unwrap();
        let first = package_digest(&root).unwrap();
        fs::write(root.join("dist/index.html"), b"two").unwrap();
        assert_ne!(first, package_digest(&root).unwrap());
        let _ = fs::remove_dir_all(root);
    }
    #[test]
    fn development_catalog_validates_referenced_files_and_duplicate_ids() {
        let root = std::env::temp_dir().join(format!(
            "wb-plugin-package-{}",
            SESSION_COUNTER.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(root.join("dist")).unwrap();
        fs::write(
            root.join("manifest.json"),
            serde_json::to_vec(&manifest("com.example.one", "one")).unwrap(),
        )
        .unwrap();
        fs::write(root.join("dist/index.html"), b"plugin").unwrap();
        let mut catalog = PluginCatalog::default();
        assert_eq!(
            catalog.install_development_dir(&root).unwrap().manifest.id,
            "com.example.one"
        );
        assert!(catalog.install_development_dir(&root).is_err());
        fs::remove_file(root.join("dist/index.html")).unwrap();
        assert!(catalog.install_development_dir(root.join(".")).is_err());
        let _ = fs::remove_dir_all(root);
    }
    #[cfg(unix)]
    #[test]
    fn development_catalog_rejects_symlinks() {
        use std::os::unix::fs::symlink;
        let root = std::env::temp_dir().join(format!(
            "wb-plugin-link-{}",
            SESSION_COUNTER.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(root.join("dist")).unwrap();
        fs::write(
            root.join("manifest.json"),
            serde_json::to_vec(&manifest("com.example.link", "link")).unwrap(),
        )
        .unwrap();
        fs::write(root.join("real.html"), b"plugin").unwrap();
        symlink(root.join("real.html"), root.join("dist/index.html")).unwrap();
        assert!(PluginCatalog::default()
            .install_development_dir(&root)
            .is_err());
        let _ = fs::remove_dir_all(root);
    }
    #[test]
    fn forged_identity_and_origin_are_rejected() {
        let mut host = host();
        let session = host
            .bootstrap("com.example.one", "project", "plugin://one")
            .unwrap();
        let request = RpcRequest {
            rpc_version: 1,
            session_id: session.id.clone(),
            request_id: "1".into(),
            method: "entity.read".into(),
            payload: serde_json::json!({}),
        };
        assert_eq!(
            host.rpc("plugin://other", &request).error.unwrap().code,
            "session.origin"
        );
        let forged = RpcRequest {
            session_id: "forged".into(),
            ..request
        };
        assert_eq!(
            host.rpc("plugin://one", &forged).error.unwrap().code,
            "session.invalid"
        );
    }
    #[test]
    fn undeclared_and_foreign_namespace_operations_are_rejected() {
        let mut host = host();
        let session = host
            .bootstrap("com.example.one", "project", "plugin://one")
            .unwrap();
        let denied = RpcRequest {
            rpc_version: 1,
            session_id: session.id.clone(),
            request_id: "1".into(),
            method: "entity.write".into(),
            payload: serde_json::json!({}),
        };
        assert_eq!(
            host.rpc("plugin://one", &denied).error.unwrap().code,
            "capability.denied"
        );
        let trusted = RpcRequest {
            method: "project.open".into(),
            ..denied.clone()
        };
        assert_eq!(
            host.rpc("plugin://one", &trusted).error.unwrap().code,
            "method.unknown"
        );
        let foreign = RpcRequest {
            method: "field.read".into(),
            payload: serde_json::json!({"namespace":"other"}),
            ..denied
        };
        assert_eq!(
            host.rpc("plugin://one", &foreign).error.unwrap().code,
            "namespace.denied"
        );
    }
    #[test]
    fn revoked_session_cannot_be_replayed() {
        let mut host = host();
        let session = host
            .bootstrap("com.example.one", "project", "plugin://one")
            .unwrap();
        host.revoke_plugin("project", "com.example.one");
        let request = RpcRequest {
            rpc_version: 1,
            session_id: session.id,
            request_id: "1".into(),
            method: "entity.read".into(),
            payload: serde_json::json!({}),
        };
        assert_eq!(
            host.rpc("plugin://one", &request).error.unwrap().code,
            "session.revoked"
        );
    }
    #[test]
    fn activation_generation_invalidates_previous_session() {
        let mut host = host();
        let first = host
            .bootstrap("com.example.one", "project", "plugin://one")
            .unwrap();
        let second = host
            .bootstrap("com.example.one", "project", "plugin://one")
            .unwrap();
        assert_ne!(first.id, second.id);
        let request = RpcRequest {
            rpc_version: 1,
            session_id: first.id,
            request_id: "1".into(),
            method: "entity.read".into(),
            payload: serde_json::json!({}),
        };
        assert_eq!(
            host.rpc("plugin://one", &request).error.unwrap().code,
            "session.revoked"
        );
    }
    #[test]
    fn dynamic_event_and_service_grants_are_checked() {
        let mut host = host();
        let entry = host
            .catalog
            .get("com.example.one")
            .unwrap()
            .manifest
            .clone();
        host.grants
            .set(
                "project",
                "com.example.one",
                &entry.capabilities,
                [
                    "event.publish:worldbuilder.core/event".into(),
                    "service.call:com.example.calculate".into(),
                ]
                .into_iter()
                .collect(),
            )
            .unwrap();
        let session = host
            .bootstrap("com.example.one", "project", "plugin://one")
            .unwrap();
        for (method, payload) in [
            (
                "event.publish",
                serde_json::json!({"type":"worldbuilder.core/event"}),
            ),
            (
                "service.call",
                serde_json::json!({"name":"com.example.calculate"}),
            ),
        ] {
            let request = RpcRequest {
                rpc_version: 1,
                session_id: session.id.clone(),
                request_id: method.into(),
                method: method.into(),
                payload,
            };
            assert!(host.rpc("plugin://one", &request).ok);
        }
    }
    #[test]
    fn namespace_collisions_are_rejected() {
        let mut ownership = NamespaceOwnership::default();
        ownership
            .register_manifest(&manifest("com.example.one", "shared"))
            .unwrap();
        assert!(ownership
            .register_manifest(&manifest("com.example.two", "shared"))
            .is_err());
    }
    #[test]
    fn expired_session_is_rejected() {
        let mut host = host();
        host.session_ttl = Duration::ZERO;
        let session = host
            .bootstrap("com.example.one", "project", "plugin://one")
            .unwrap();
        let request = RpcRequest {
            rpc_version: 1,
            session_id: session.id,
            request_id: "1".into(),
            method: "entity.read".into(),
            payload: serde_json::json!({}),
        };
        assert_eq!(
            host.rpc("plugin://one", &request).error.unwrap().code,
            "session.expired"
        );
    }

    #[test]
    fn canonical_bundled_manifests_register_without_handwritten_rust_copies() {
        let mut host = PluginHost::new();
        host.register_bundled_json(include_str!("../../../packages/modules/lore/manifest.json"))
            .unwrap();
        host.register_bundled_json(include_str!(
            "../../../packages/modules/timeline/manifest.json"
        ))
        .unwrap();
        assert_eq!(host.catalog.list().count(), 2);
        assert!(host.catalog.get("worldbuilder.lore").is_some());
        assert!(host.catalog.get("worldbuilder.timeline").is_some());
    }
}
