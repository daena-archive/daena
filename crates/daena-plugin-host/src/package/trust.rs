use super::{PackageError, PackageSignature, VerifiedPackage};
use daena_plugin_api::PluginManifest;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

pub const TRUST_SNAPSHOT_FILE: &str = "trust.json";
const TRUST_SCHEMA_VERSION: u32 = 1;
const MAX_TRUST_SNAPSHOT_BYTES: u64 = 256 * 1024;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PublisherKey {
    #[serde(default)]
    pub key_id: String,
    pub public_key: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PublisherIdentity {
    #[serde(default)]
    pub keys: Vec<PublisherKey>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RevokedKey {
    pub publisher: String,
    #[serde(default)]
    pub key_id: Option<String>,
    #[serde(default)]
    pub public_key: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RevokedPackage {
    pub plugin_id: String,
    #[serde(default)]
    pub version: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RevocationList {
    #[serde(default)]
    pub keys: Vec<RevokedKey>,
    #[serde(default)]
    pub digests: Vec<String>,
    #[serde(default)]
    pub packages: Vec<RevokedPackage>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TrustSnapshot {
    #[serde(default = "trust_schema_version")]
    pub schema_version: u32,
    #[serde(default)]
    pub publishers: BTreeMap<String, PublisherIdentity>,
    #[serde(default)]
    pub revocations: RevocationList,
}

fn trust_schema_version() -> u32 {
    TRUST_SCHEMA_VERSION
}

impl Default for TrustSnapshot {
    fn default() -> Self {
        Self {
            schema_version: TRUST_SCHEMA_VERSION,
            publishers: BTreeMap::new(),
            revocations: RevocationList::default(),
        }
    }
}

impl TrustSnapshot {
    pub fn from_pinned_keys(keys: BTreeMap<String, String>) -> Self {
        Self {
            schema_version: TRUST_SCHEMA_VERSION,
            publishers: keys
                .into_iter()
                .map(|(id, public_key)| {
                    (
                        id,
                        PublisherIdentity {
                            keys: vec![PublisherKey {
                                key_id: String::new(),
                                public_key,
                            }],
                        },
                    )
                })
                .collect(),
            revocations: RevocationList::default(),
        }
    }

    pub fn load(path: impl AsRef<Path>) -> Result<Self, PackageError> {
        let path = path.as_ref();
        if !path.is_file() {
            return Ok(Self::default());
        }
        let bytes = fs::read(path).map_err(|error| PackageError(error.to_string()))?;
        if bytes.len() as u64 > MAX_TRUST_SNAPSHOT_BYTES {
            return Err(PackageError("trust snapshot exceeds size limit".into()));
        }
        let snapshot: Self = serde_json::from_slice(&bytes)
            .map_err(|error| PackageError(format!("invalid trust snapshot: {error}")))?;
        snapshot.validate()
    }

    fn validate(self) -> Result<Self, PackageError> {
        if self.schema_version != TRUST_SCHEMA_VERSION {
            return Err(PackageError(
                "unsupported trust snapshot schema version".into(),
            ));
        }
        for (publisher, identity) in &self.publishers {
            if publisher.is_empty() {
                return Err(PackageError("publisher identity is missing an id".into()));
            }
            for key in &identity.keys {
                if key.public_key.is_empty() {
                    return Err(PackageError("publisher key is missing publicKey".into()));
                }
            }
        }
        for entry in &self.revocations.keys {
            if entry.publisher.is_empty() {
                return Err(PackageError("key revocation is missing a publisher".into()));
            }
            if entry.public_key.as_deref().is_some_and(str::is_empty)
                || (entry.public_key.is_none() && entry.key_id.is_some())
            {
                return Err(PackageError("key revocation must include publicKey".into()));
            }
        }
        Ok(self)
    }

    pub fn digest_revoked(&self, digest: &str) -> bool {
        self.revocations.digests.iter().any(|value| value == digest)
    }

    pub fn package_revoked(&self, plugin_id: &str, version: &str) -> bool {
        self.revocations.packages.iter().any(|entry| {
            entry.plugin_id == plugin_id
                && entry
                    .version
                    .as_deref()
                    .is_none_or(|expected| expected == version)
        })
    }

    pub fn key_revoked(&self, publisher: &str, signature: &PackageSignature) -> bool {
        self.revocations.keys.iter().any(|entry| {
            if entry.publisher != publisher {
                return false;
            }
            match entry.public_key.as_deref().filter(|key| !key.is_empty()) {
                Some(key) => key == signature.public_key,
                None => entry.key_id.is_none(),
            }
        })
    }

    pub fn pinned_identity(&self, publisher: &str) -> Option<&PublisherIdentity> {
        self.publishers.get(publisher)
    }

    pub fn pins_publishers(&self) -> bool {
        !self.publishers.is_empty()
    }

    pub fn key_is_pinned(&self, publisher: &str, signature: &PackageSignature) -> bool {
        let Some(identity) = self.pinned_identity(publisher) else {
            return false;
        };
        identity.keys.iter().any(|key| {
            key.public_key == signature.public_key
                && (key.key_id.is_empty()
                    || signature
                        .key_id
                        .as_deref()
                        .is_none_or(|id| id == key.key_id))
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum TrustStatus {
    Unsigned,
    Signed,
    TrustedPublisher,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PackageReview {
    pub plugin_id: String,
    pub version: String,
    pub publisher: String,
    pub digest: String,
    pub signed: bool,
    pub key_id: Option<String>,
    pub trust_status: TrustStatus,
    pub capabilities: Vec<String>,
    pub disclosures: Vec<String>,
}

pub fn review_package(package: &VerifiedPackage, trust: &TrustSnapshot) -> PackageReview {
    review_manifest(
        &package.manifest,
        &package.digest,
        package.signature.as_ref(),
        package.signed,
        trust,
    )
}

pub fn review_manifest(
    manifest: &PluginManifest,
    digest: &str,
    signature: Option<&PackageSignature>,
    signed: bool,
    trust: &TrustSnapshot,
) -> PackageReview {
    let key_id = signature.and_then(|value| value.key_id.clone());
    let trust_status = if !signed {
        TrustStatus::Unsigned
    } else if signature.is_some_and(|value| trust.key_is_pinned(&manifest.publisher, value)) {
        TrustStatus::TrustedPublisher
    } else {
        TrustStatus::Signed
    };
    let mut disclosures = Vec::new();
    match trust_status {
        TrustStatus::Unsigned => {
            disclosures.push("unsigned package requires explicit install consent".into())
        }
        TrustStatus::Signed => {
            disclosures.push("signature is valid but the publisher is not pinned".into())
        }
        TrustStatus::TrustedPublisher => {
            disclosures.push("publisher key is pinned in the local trust snapshot".into())
        }
    }
    PackageReview {
        plugin_id: manifest.id.clone(),
        version: manifest.version.clone(),
        publisher: manifest.publisher.clone(),
        digest: digest.to_string(),
        signed,
        key_id,
        trust_status,
        capabilities: manifest.capabilities.clone(),
        disclosures,
    }
}
