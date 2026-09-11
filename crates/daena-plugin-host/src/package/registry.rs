use super::{write_atomic_bytes, PackageError};
use crate::package::trust::CATALOG_FILE;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

const CATALOG_SCHEMA_VERSION: u32 = 1;
pub const MAX_CATALOG_BYTES: u64 = 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CatalogPlugin {
    pub id: String,
    pub publisher: String,
    pub name: String,
    pub version: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub digest: Option<String>,
    pub artifact_url: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DiscoveryCatalog {
    #[serde(default = "catalog_schema_version")]
    pub schema_version: u32,
    #[serde(default)]
    pub plugins: Vec<CatalogPlugin>,
}

fn catalog_schema_version() -> u32 {
    CATALOG_SCHEMA_VERSION
}

impl Default for DiscoveryCatalog {
    fn default() -> Self {
        Self {
            schema_version: CATALOG_SCHEMA_VERSION,
            plugins: Vec::new(),
        }
    }
}

impl DiscoveryCatalog {
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, PackageError> {
        if bytes.len() as u64 > MAX_CATALOG_BYTES {
            return Err(PackageError("discovery catalog exceeds size limit".into()));
        }
        let catalog: Self = serde_json::from_slice(bytes)
            .map_err(|error| PackageError(format!("invalid discovery catalog: {error}")))?;
        catalog.validate()?;
        Ok(catalog)
    }

    pub fn load(path: impl AsRef<Path>) -> Result<Self, PackageError> {
        let path = path.as_ref();
        if !path.is_file() {
            return Ok(Self::default());
        }
        let bytes = fs::read(path).map_err(|error| PackageError(error.to_string()))?;
        Self::from_bytes(&bytes)
    }

    pub fn load_from_install_root(root: impl AsRef<Path>) -> Result<Self, PackageError> {
        Self::load(root.as_ref().join(CATALOG_FILE))
    }

    pub fn store(&self, path: impl AsRef<Path>) -> Result<(), PackageError> {
        self.validate()?;
        let path = path.as_ref();
        let mut bytes = serde_json::to_vec_pretty(self)
            .map_err(|error| PackageError(format!("invalid discovery catalog: {error}")))?;
        bytes.push(b'\n');
        if bytes.len() as u64 > MAX_CATALOG_BYTES {
            return Err(PackageError("discovery catalog exceeds size limit".into()));
        }
        write_atomic_bytes(path, &bytes)
    }

    fn validate(&self) -> Result<(), PackageError> {
        if self.schema_version != CATALOG_SCHEMA_VERSION {
            return Err(PackageError(
                "unsupported discovery catalog schema version".into(),
            ));
        }
        for plugin in &self.plugins {
            if plugin.id.is_empty()
                || plugin.publisher.is_empty()
                || plugin.name.is_empty()
                || plugin.version.is_empty()
                || plugin.artifact_url.is_empty()
            {
                return Err(PackageError(
                    "catalog plugin is missing required fields".into(),
                ));
            }
            if let Some(digest) = plugin.digest.as_deref() {
                if !is_hex_digest(digest) {
                    return Err(PackageError(
                        "catalog digest is not a SHA-256 hex digest".into(),
                    ));
                }
            }
        }
        Ok(())
    }
}

pub fn advertised_digest_matches(
    advertised: Option<&str>,
    computed: &str,
) -> Result<(), PackageError> {
    let Some(advertised) = advertised.filter(|value| !value.is_empty()) else {
        return Ok(());
    };
    if !advertised.eq_ignore_ascii_case(computed) {
        return Err(PackageError(
            "downloaded package digest does not match catalog digest".into(),
        ));
    }
    Ok(())
}

fn is_hex_digest(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}
