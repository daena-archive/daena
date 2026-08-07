//! Verification and lifecycle planning for `.wbplugin` packages.
//!
//! This module is deliberately independent from the webview and WASM runtime.
//! Nothing is made executable until the archive has been fully checked and
//! atomically moved into the host-owned version store.

use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::{Cursor, Read, Write};
use std::path::{Component, Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use daena_plugin_api::{parse_manifest, PluginManifest};
use zip::ZipArchive;

const SIGNATURE_FILE: &str = "signature.json";
type PackageFiles = Vec<(String, Vec<u8>)>;
type PackageTree = (PackageFiles, BTreeSet<String>);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackageError(pub String);

impl std::fmt::Display for PackageError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}
impl std::error::Error for PackageError {}

#[derive(Debug, Clone, Copy)]
pub struct ArchiveLimits {
    pub max_compressed_bytes: u64,
    pub max_uncompressed_bytes: u64,
    pub max_file_count: usize,
    pub max_path_length: usize,
    pub max_file_bytes: u64,
}

impl Default for ArchiveLimits {
    fn default() -> Self {
        Self {
            max_compressed_bytes: 128 * 1024 * 1024,
            max_uncompressed_bytes: 512 * 1024 * 1024,
            max_file_count: 4096,
            max_path_length: 512,
            max_file_bytes: 128 * 1024 * 1024,
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PackageSignature {
    pub algorithm: String,
    #[serde(default)]
    pub publisher: Option<String>,
    #[serde(rename = "publicKey")]
    pub public_key: String,
    pub signature: String,
    pub digest: String,
}

#[derive(Debug, Clone)]
pub struct PluginPackage {
    pub manifest: PluginManifest,
    pub root: PathBuf,
    pub digest: String,
    pub signature: Option<PackageSignature>,
    pub signed: bool,
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct PackageCatalog {
    versions: BTreeMap<String, BTreeMap<String, InstalledVersion>>,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct InstalledVersion {
    pub plugin_id: String,
    pub version: String,
    pub digest: String,
    pub root: PathBuf,
    pub publisher: String,
    pub signed: bool,
    pub installed_at: u64,
    /// Unsigned packages are accepted again on restart only when the user
    /// explicitly consented during installation. The package contents are
    /// still rehashed and, when present, signatures are reverified.
    #[serde(default)]
    pub unsigned_consent: bool,
}

#[derive(Debug, Clone, Default)]
pub struct VerificationPolicy {
    pub require_signature: bool,
    pub allow_unsigned: bool,
    pub trusted_publishers: BTreeMap<String, String>,
}

impl VerificationPolicy {
    pub fn with_unsigned_consent() -> Self {
        Self {
            allow_unsigned: true,
            ..Self::default()
        }
    }

    pub fn trusted_publishers(publishers: BTreeMap<String, String>) -> Self {
        Self {
            require_signature: true,
            trusted_publishers: publishers,
            ..Self::default()
        }
    }
}

impl PackageCatalog {
    pub fn list(&self, plugin_id: &str) -> impl Iterator<Item = &InstalledVersion> {
        self.versions
            .get(plugin_id)
            .into_iter()
            .flat_map(|v| v.values())
    }

    pub fn get(&self, plugin_id: &str, version: &str) -> Option<&InstalledVersion> {
        self.versions.get(plugin_id)?.get(version)
    }

    pub fn plugin_ids(&self) -> impl Iterator<Item = &String> {
        self.versions.keys()
    }

    pub fn active_candidate(&self, plugin_id: &str) -> Option<&InstalledVersion> {
        self.list(plugin_id)
            .max_by(|a, b| compare_versions(&a.version, &b.version))
    }

    pub fn install(
        &mut self,
        archive: impl AsRef<Path>,
        install_root: impl AsRef<Path>,
        limits: ArchiveLimits,
        policy: VerificationPolicy,
    ) -> Result<PluginPackage, PackageError> {
        let unsigned_consent = policy.allow_unsigned;
        let package = verify_and_extract(archive.as_ref(), install_root.as_ref(), limits, policy)?;
        let id = package.manifest.id.clone();
        let version = package.manifest.version.clone();
        let installed = InstalledVersion {
            plugin_id: id.clone(),
            version: version.clone(),
            digest: package.digest.clone(),
            root: package.root.clone(),
            publisher: package.manifest.publisher.clone(),
            signed: package.signed,
            installed_at: unix_now(),
            unsigned_consent: unsigned_consent && !package.signed,
        };
        if self.get(&id, &version).is_some() {
            return Err(PackageError("plugin version is already installed".into()));
        }
        self.versions
            .entry(id)
            .or_default()
            .insert(version, installed);
        Ok(package)
    }

    pub fn load(path: impl AsRef<Path>) -> Result<Self, PackageError> {
        let path = path.as_ref();
        if !path.is_file() {
            return Ok(Self::default());
        }
        let bytes = fs::read(path).map_err(io_error)?;
        serde_json::from_slice(&bytes)
            .map_err(|error| PackageError(format!("invalid plugin catalog: {error}")))
    }

    pub fn persist(&self, path: impl AsRef<Path>) -> Result<(), PackageError> {
        let path = path.as_ref();
        let parent = path
            .parent()
            .ok_or_else(|| PackageError("plugin catalog has no parent directory".into()))?;
        fs::create_dir_all(parent).map_err(io_error)?;
        let bytes = serde_json::to_vec_pretty(self)
            .map_err(|error| PackageError(format!("serialize plugin catalog: {error}")))?;
        let mut temporary = tempfile::NamedTempFile::new_in(parent).map_err(io_error)?;
        temporary.write_all(&bytes).map_err(io_error)?;
        temporary.flush().map_err(io_error)?;
        temporary
            .persist(path)
            .map_err(|error| PackageError(error.error.to_string()))?;
        Ok(())
    }

    /// Rebuild the in-memory catalog from the app-owned version store. The
    /// JSON catalog is used only for installation metadata and unsigned-package
    /// consent; package contents and manifests remain the source of truth.
    pub fn rediscover(
        &mut self,
        install_root: impl AsRef<Path>,
        limits: ArchiveLimits,
        policy: &VerificationPolicy,
    ) -> Result<Vec<PackageError>, PackageError> {
        let install_root = install_root.as_ref();
        if !install_root.exists() {
            self.versions.clear();
            return Ok(Vec::new());
        }
        let previous = self.clone();
        self.versions.clear();
        let mut rejected = Vec::new();
        for plugin_entry in fs::read_dir(install_root).map_err(io_error)? {
            let plugin_entry = plugin_entry.map_err(io_error)?;
            let plugin_root = plugin_entry.path();
            let plugin_type = plugin_entry.file_type().map_err(io_error)?;
            if !plugin_type.is_dir() || plugin_type.is_symlink() {
                continue;
            }
            for version_entry in fs::read_dir(&plugin_root).map_err(io_error)? {
                let version_entry = version_entry.map_err(io_error)?;
                let version_root = version_entry.path();
                let version_type = version_entry.file_type().map_err(io_error)?;
                if !version_type.is_dir()
                    || version_type.is_symlink()
                    || version_root
                        .file_name()
                        .and_then(|value| value.to_str())
                        .is_some_and(|name| name.starts_with('.'))
                {
                    continue;
                }
                let expected_id = plugin_root.file_name().and_then(|value| value.to_str());
                let expected_version = version_root.file_name().and_then(|value| value.to_str());
                let prior = expected_id
                    .zip(expected_version)
                    .and_then(|(id, version)| previous.get(id, version));
                let mut verification = policy.clone();
                verification.allow_unsigned |= prior.is_some_and(|entry| entry.unsigned_consent);
                match verify_installed(
                    &version_root,
                    limits,
                    &verification,
                    expected_id,
                    expected_version,
                ) {
                    Ok(mut installed) => {
                        if prior.is_some_and(|entry| entry.digest != installed.digest) {
                            rejected.push(PackageError(
                                "installed package digest differs from the recorded digest".into(),
                            ));
                            continue;
                        }
                        if let Some(previous) = prior {
                            installed.installed_at = previous.installed_at;
                            installed.unsigned_consent = previous.unsigned_consent;
                        }
                        self.versions
                            .entry(installed.plugin_id.clone())
                            .or_default()
                            .insert(installed.version.clone(), installed);
                    }
                    Err(error) => rejected.push(error),
                }
            }
        }
        Ok(rejected)
    }

    pub fn remove_version(&mut self, plugin_id: &str, version: &str) -> Result<(), PackageError> {
        let Some(entry) = self.get(plugin_id, version) else {
            return Err(PackageError("plugin is not installed".into()));
        };
        fs::remove_dir_all(&entry.root).map_err(|e| PackageError(e.to_string()))?;
        let versions = self
            .versions
            .get_mut(plugin_id)
            .expect("installed version has a plugin catalog entry");
        versions.remove(version);
        if versions.is_empty() {
            self.versions.remove(plugin_id);
        }
        Ok(())
    }
}

fn verify_and_extract(
    archive_path: &Path,
    install_root: &Path,
    limits: ArchiveLimits,
    policy: VerificationPolicy,
) -> Result<PluginPackage, PackageError> {
    if archive_path.extension().and_then(|v| v.to_str()) != Some("wbplugin") {
        return Err(PackageError(
            "package must use the .wbplugin extension".into(),
        ));
    }
    let compressed = fs::metadata(archive_path).map_err(io_error)?.len();
    if compressed > limits.max_compressed_bytes {
        return Err(PackageError("compressed package exceeds size limit".into()));
    }
    let bytes = fs::read(archive_path).map_err(io_error)?;
    let mut zip = ZipArchive::new(Cursor::new(bytes))
        .map_err(|e| PackageError(format!("invalid ZIP archive: {e}")))?;
    if zip.len() > limits.max_file_count {
        return Err(PackageError("package contains too many entries".into()));
    }
    let mut names = BTreeSet::new();
    let mut folded = BTreeSet::new();
    let mut total = 0u64;
    let mut manifest_bytes = None;
    let mut signature = None;
    let mut files = Vec::new();
    for index in 0..zip.len() {
        let mut entry = zip
            .by_index(index)
            .map_err(|e| PackageError(format!("invalid ZIP entry: {e}")))?;
        let name = entry.name().replace('\\', "/");
        validate_archive_path(&name, limits.max_path_length)?;
        if !names.insert(name.clone()) || !folded.insert(name.to_ascii_lowercase()) {
            return Err(PackageError(format!(
                "duplicate or case-colliding archive path: {name}"
            )));
        }
        if entry
            .unix_mode()
            .is_some_and(|mode| mode & 0o170000 != 0 && mode & 0o170000 != 0o100000)
        {
            return Err(PackageError(format!(
                "links or special files are not allowed: {name}"
            )));
        }
        if entry.is_dir() {
            continue;
        }
        let size = entry.size();
        if size > limits.max_file_bytes
            || total.saturating_add(size) > limits.max_uncompressed_bytes
        {
            return Err(PackageError(
                "uncompressed package exceeds size limit".into(),
            ));
        }
        let remaining = limits.max_uncompressed_bytes.saturating_sub(total);
        let read_limit = limits.max_file_bytes.min(remaining);
        let mut content = Vec::with_capacity(size.min(1024 * 1024) as usize);
        entry
            .by_ref()
            .take(read_limit.saturating_add(1))
            .read_to_end(&mut content)
            .map_err(io_error)?;
        if content.len() as u64 > read_limit {
            return Err(PackageError(
                "actual uncompressed package data exceeds size limit".into(),
            ));
        }
        total += content.len() as u64;
        if name == "manifest.json" {
            manifest_bytes = Some(content.clone());
        }
        if name == SIGNATURE_FILE {
            signature = Some(
                serde_json::from_slice::<PackageSignature>(&content)
                    .map_err(|e| PackageError(format!("invalid signature metadata: {e}")))?,
            );
        }
        files.push((name, content));
    }
    let manifest_bytes =
        manifest_bytes.ok_or_else(|| PackageError("package is missing manifest.json".into()))?;
    let manifest = parse_manifest(
        std::str::from_utf8(&manifest_bytes).map_err(|e| PackageError(e.to_string()))?,
    )
    .map_err(|e| PackageError(e.to_string()))?;
    if !host_api_compatible(&manifest.host_api, "1.0.0") {
        return Err(PackageError(
            "package is incompatible with the current host API".into(),
        ));
    }
    validate_references(&manifest, &names)?;
    let digest = archive_digest(&files)?;
    let signed = verify_signature(
        signature.as_ref(),
        &digest,
        &manifest.publisher,
        &policy.trusted_publishers,
    )?;
    if !signed && !policy.allow_unsigned {
        return Err(PackageError(
            "unsigned package requires explicit install consent".into(),
        ));
    }
    if policy.require_signature && !signed {
        return Err(PackageError("package signature is required".into()));
    }
    let version_root = install_root.join(&manifest.id).join(&manifest.version);
    let parent = version_root
        .parent()
        .ok_or_else(|| PackageError("invalid install root".into()))?;
    fs::create_dir_all(parent).map_err(io_error)?;
    if version_root.exists() {
        return Err(PackageError("plugin version is already installed".into()));
    }
    let staging = tempfile::Builder::new()
        .prefix(".staging-")
        .tempdir_in(parent)
        .map_err(io_error)?;
    let result = (|| {
        for (name, content) in &files {
            let path = staging.path().join(name);
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).map_err(io_error)?;
            }
            fs::write(path, content).map_err(io_error)?;
        }
        fs::rename(staging.path(), &version_root).map_err(io_error)?;
        Ok(())
    })();
    result?;
    Ok(PluginPackage {
        manifest,
        root: version_root,
        digest,
        signature,
        signed,
    })
}

fn verify_installed(
    root: &Path,
    limits: ArchiveLimits,
    policy: &VerificationPolicy,
    expected_id: Option<&str>,
    expected_version: Option<&str>,
) -> Result<InstalledVersion, PackageError> {
    let (files, names) = read_package_tree(root, limits)?;
    let manifest_bytes = files
        .iter()
        .find(|(name, _)| name == "manifest.json")
        .map(|(_, bytes)| bytes)
        .ok_or_else(|| PackageError("installed package is missing manifest.json".into()))?;
    let manifest = parse_manifest(
        std::str::from_utf8(manifest_bytes).map_err(|error| PackageError(error.to_string()))?,
    )
    .map_err(|error| PackageError(error.to_string()))?;
    if expected_id != Some(manifest.id.as_str())
        || expected_version != Some(manifest.version.as_str())
    {
        return Err(PackageError(
            "installed package path does not match its manifest".into(),
        ));
    }
    if !host_api_compatible(&manifest.host_api, "1.0.0") {
        return Err(PackageError(
            "installed package is incompatible with the current host API".into(),
        ));
    }
    validate_references(&manifest, &names)?;
    let signature = files
        .iter()
        .find(|(name, _)| name == SIGNATURE_FILE)
        .map(|(_, bytes)| {
            serde_json::from_slice::<PackageSignature>(bytes)
                .map_err(|error| PackageError(format!("invalid signature metadata: {error}")))
        })
        .transpose()?;
    let digest = archive_digest(&files)?;
    let signed = verify_signature(
        signature.as_ref(),
        &digest,
        &manifest.publisher,
        &policy.trusted_publishers,
    )?;
    if !signed && !policy.allow_unsigned {
        return Err(PackageError(
            "unsigned installed package lacks recorded consent".into(),
        ));
    }
    if policy.require_signature && !signed {
        return Err(PackageError(
            "installed package signature is required".into(),
        ));
    }
    Ok(InstalledVersion {
        plugin_id: manifest.id,
        version: manifest.version,
        digest,
        root: root.to_path_buf(),
        publisher: manifest.publisher,
        signed,
        installed_at: fs::metadata(root)
            .and_then(|metadata| metadata.modified())
            .ok()
            .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
            .map_or(0, |duration| duration.as_secs()),
        unsigned_consent: !signed && policy.allow_unsigned,
    })
}

fn read_package_tree(
    root: &Path,
    limits: ArchiveLimits,
) -> Result<PackageTree, PackageError> {
    let mut files = Vec::new();
    let mut names = BTreeSet::new();
    let mut folded = BTreeSet::new();
    let mut total = 0u64;
    read_package_tree_at(
        root,
        root,
        limits,
        &mut files,
        &mut names,
        &mut folded,
        &mut total,
    )?;
    Ok((files, names))
}

fn read_package_tree_at(
    root: &Path,
    current: &Path,
    limits: ArchiveLimits,
    files: &mut Vec<(String, Vec<u8>)>,
    names: &mut BTreeSet<String>,
    folded: &mut BTreeSet<String>,
    total: &mut u64,
) -> Result<(), PackageError> {
    for entry in fs::read_dir(current).map_err(io_error)? {
        let entry = entry.map_err(io_error)?;
        let path = entry.path();
        let file_type = entry.file_type().map_err(io_error)?;
        if file_type.is_symlink() || (!file_type.is_dir() && !file_type.is_file()) {
            return Err(PackageError(format!(
                "installed package contains a link or special file: {}",
                path.display()
            )));
        }
        if file_type.is_dir() {
            read_package_tree_at(root, &path, limits, files, names, folded, total)?;
            continue;
        }
        let relative = path
            .strip_prefix(root)
            .map_err(|error| PackageError(error.to_string()))?
            .to_string_lossy()
            .replace('\\', "/");
        validate_archive_path(&relative, limits.max_path_length)?;
        if !names.insert(relative.clone()) || !folded.insert(relative.to_ascii_lowercase()) {
            return Err(PackageError(format!(
                "duplicate or case-colliding installed path: {relative}"
            )));
        }
        if files.len() >= limits.max_file_count {
            return Err(PackageError(
                "installed package contains too many files".into(),
            ));
        }
        let size = fs::metadata(&path).map_err(io_error)?.len();
        if size > limits.max_file_bytes
            || total.saturating_add(size) > limits.max_uncompressed_bytes
        {
            return Err(PackageError("installed package exceeds size limit".into()));
        }
        let bytes = fs::read(&path).map_err(io_error)?;
        *total += bytes.len() as u64;
        files.push((relative, bytes));
    }
    Ok(())
}

fn validate_archive_path(name: &str, max_len: usize) -> Result<(), PackageError> {
    if name.is_empty()
        || name.len() > max_len
        || name.contains('\0')
        || name.contains(':')
        || name.starts_with('/')
    {
        return Err(PackageError(format!("invalid archive path: {name}")));
    }
    let path = Path::new(name);
    if path.components().any(|c| {
        matches!(
            c,
            Component::CurDir | Component::ParentDir | Component::RootDir | Component::Prefix(_)
        )
    }) {
        return Err(PackageError(format!(
            "archive path escapes package root: {name}"
        )));
    }
    Ok(())
}

fn validate_references(
    manifest: &PluginManifest,
    names: &BTreeSet<String>,
) -> Result<(), PackageError> {
    for path in [
        manifest.entrypoints.ui.as_ref(),
        manifest.entrypoints.wasm.as_ref(),
    ]
    .into_iter()
    .flatten()
    {
        if !names.contains(path) {
            return Err(PackageError(format!(
                "manifest entrypoint is missing: {path}"
            )));
        }
    }
    Ok(())
}

fn archive_digest(files: &[(String, Vec<u8>)]) -> Result<String, PackageError> {
    let mut sorted = files.iter().collect::<Vec<_>>();
    sorted.sort_by(|a, b| a.0.cmp(&b.0));
    let mut hasher = Sha256::new();
    for (name, bytes) in sorted {
        hasher.update(name.as_bytes());
        hasher.update([0]);
        if name == SIGNATURE_FILE {
            let mut metadata: serde_json::Value = serde_json::from_slice(bytes)
                .map_err(|e| PackageError(format!("invalid signature metadata: {e}")))?;
            if let Some(object) = metadata.as_object_mut() {
                object.insert("digest".into(), serde_json::Value::String(String::new()));
                object.insert("signature".into(), serde_json::Value::String(String::new()));
            }
            hasher.update(
                serde_json::to_vec(&metadata)
                    .map_err(|e| PackageError(format!("invalid signature metadata: {e}")))?,
            );
        } else {
            hasher.update(bytes);
        }
        hasher.update([0]);
    }
    Ok(hasher
        .finalize()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect())
}

fn verify_signature(
    signature: Option<&PackageSignature>,
    digest: &str,
    publisher: &str,
    trusted_publishers: &BTreeMap<String, String>,
) -> Result<bool, PackageError> {
    let Some(signature) = signature else {
        return Ok(false);
    };
    if signature
        .publisher
        .as_deref()
        .is_some_and(|value| value != publisher)
    {
        return Err(PackageError(
            "signature publisher does not match manifest".into(),
        ));
    }
    if signature.algorithm != "ed25519" || signature.digest != digest {
        return Err(PackageError(
            "package signature does not match digest".into(),
        ));
    }
    let key = BASE64
        .decode(&signature.public_key)
        .map_err(|e| PackageError(format!("invalid public key: {e}")))?;
    let key: [u8; 32] = key
        .try_into()
        .map_err(|_| PackageError("invalid Ed25519 public key length".into()))?;
    if let Some(trusted_key) = trusted_publishers.get(publisher) {
        if trusted_key != &signature.public_key {
            return Err(PackageError(
                "signature key is not trusted for publisher".into(),
            ));
        }
    } else if !trusted_publishers.is_empty() {
        return Err(PackageError("publisher is not trusted".into()));
    }
    let verifying = VerifyingKey::from_bytes(&key)
        .map_err(|e| PackageError(format!("invalid public key: {e}")))?;
    let sig = BASE64
        .decode(&signature.signature)
        .map_err(|e| PackageError(format!("invalid signature: {e}")))?;
    let sig =
        Signature::from_slice(&sig).map_err(|e| PackageError(format!("invalid signature: {e}")))?;
    verifying
        .verify(digest.as_bytes(), &sig)
        .map_err(|_| PackageError(format!("signature is not valid for publisher {publisher}")))?;
    Ok(true)
}

fn host_api_compatible(range: &str, current: &str) -> bool {
    let normalized = range.split_whitespace().collect::<Vec<_>>().join(", ");
    if normalized.is_empty() {
        return false;
    }
    let Ok(requirement) = semver::VersionReq::parse(&normalized) else {
        return false;
    };
    let Ok(actual) = semver::Version::parse(current) else {
        return false;
    };
    requirement.matches(&actual)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapabilityConsent {
    pub added: BTreeSet<String>,
    pub removed: BTreeSet<String>,
    pub requires_renewal: bool,
}

impl CapabilityConsent {
    pub fn compare(previous: &[String], next: &[String]) -> Self {
        let previous = previous.iter().cloned().collect::<BTreeSet<_>>();
        let next = next.iter().cloned().collect::<BTreeSet<_>>();
        let added = next.difference(&previous).cloned().collect::<BTreeSet<_>>();
        let removed = previous.difference(&next).cloned().collect::<BTreeSet<_>>();
        Self {
            requires_renewal: !added.is_empty(),
            added,
            removed,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MigrationPlan {
    pub from: u32,
    pub to: u32,
    pub migration_ids: Vec<String>,
    pub checksums: Vec<String>,
    pub requires_backup: bool,
}

pub fn select_migrations(
    manifest: &PluginManifest,
    current: u32,
) -> Result<MigrationPlan, PackageError> {
    let mut migrations = manifest.migrations.iter().collect::<Vec<_>>();
    migrations.sort_by_key(|m| m.from);
    let mut from = current;
    let mut ids = Vec::new();
    let mut checksums = Vec::new();
    let mut requires_backup = false;
    for migration in migrations.into_iter().filter(|m| m.from >= current) {
        if migration.from != from {
            return Err(PackageError(
                "migration chain does not start at stored version".into(),
            ));
        }
        let bytes = serde_json::to_vec(migration).map_err(|e| PackageError(e.to_string()))?;
        let mut hash = Sha256::new();
        hash.update(bytes);
        checksums.push(hash.finalize().iter().map(|b| format!("{b:02x}")).collect());
        ids.push(migration.id.clone());
        from = migration.to;
        requires_backup |= migration.recovery == "backup";
    }
    Ok(MigrationPlan {
        from: current,
        to: from,
        migration_ids: ids,
        checksums,
        requires_backup,
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpgradePlan {
    pub plugin_id: String,
    pub from_version: Option<String>,
    pub to_version: String,
    pub consent: CapabilityConsent,
    pub migrations: MigrationPlan,
}

pub fn plan_upgrade(
    previous: Option<(&PluginManifest, u32)>,
    next: &PluginManifest,
) -> Result<UpgradePlan, PackageError> {
    let (from_version, consent, current) = previous.map_or(
        (None, CapabilityConsent::compare(&[], &next.capabilities), 0),
        |(manifest, version)| {
            (
                Some(manifest.version.clone()),
                CapabilityConsent::compare(&manifest.capabilities, &next.capabilities),
                version,
            )
        },
    );
    Ok(UpgradePlan {
        plugin_id: next.id.clone(),
        from_version,
        to_version: next.version.clone(),
        consent,
        migrations: select_migrations(next, current)?,
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RollbackPlan {
    pub plugin_id: String,
    pub from_version: String,
    pub to_version: String,
    pub requires_backup_restore: bool,
}

pub fn plan_rollback(
    plugin_id: &str,
    active: &InstalledVersion,
    previous: &InstalledVersion,
    current_data_version: u32,
    target: &PluginManifest,
) -> Result<RollbackPlan, PackageError> {
    if target.id != plugin_id || !is_version_compatible(&target.version, &previous.version) {
        return Err(PackageError("rollback target is incompatible".into()));
    }
    let migrations = select_migrations(target, current_data_version)?;
    if migrations.to != current_data_version {
        return Err(PackageError(
            "rollback target cannot preserve stored data".into(),
        ));
    }
    Ok(RollbackPlan {
        plugin_id: plugin_id.into(),
        from_version: active.version.clone(),
        to_version: previous.version.clone(),
        requires_backup_restore: true,
    })
}

fn is_version_compatible(target: &str, expected: &str) -> bool {
    target == expected
}

fn compare_versions(a: &str, b: &str) -> std::cmp::Ordering {
    match (semver::Version::parse(a), semver::Version::parse(b)) {
        (Ok(a), Ok(b)) => a.cmp(&b),
        _ => a.cmp(b),
    }
}
fn unix_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}
fn io_error(error: std::io::Error) -> PackageError {
    PackageError(error.to_string())
}

#[cfg(test)]
mod tests;
