// Filesystem, hashing, and copy helpers.
use crate::error::CoreError;
use sha2::{Digest, Sha256};
use std::path::Path;
use uuid::Uuid;

pub(crate) fn chrono_like_now() -> String {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos()
        .to_string()
}

pub(super) fn copy_directory(source: &Path, destination: &Path) -> Result<(), CoreError> {
    std::fs::create_dir_all(destination).map_err(|source| CoreError::Io {
        operation: "create recovery backup payload directory",
        source,
    })?;
    for entry in std::fs::read_dir(source).map_err(|source| CoreError::Io {
        operation: "read recovery backup payload directory",
        source,
    })? {
        let entry = entry.map_err(|source| CoreError::Io {
            operation: "read recovery backup payload entry",
            source,
        })?;
        let source_path = entry.path();
        let destination_path = destination.join(entry.file_name());
        if source_path.is_dir() {
            copy_directory(&source_path, &destination_path)?;
        } else {
            std::fs::copy(&source_path, &destination_path).map_err(|source| CoreError::Io {
                operation: "copy recovery backup payload",
                source,
            })?;
        }
    }
    Ok(())
}

pub(super) fn copy_portable_project(source: &Path, destination: &Path) -> Result<(), CoreError> {
    std::fs::create_dir_all(destination).map_err(|source| CoreError::Io {
        operation: "create portable backup artifact",
        source,
    })?;
    for entry in ["project.json", "entities", "plugins", "assets"] {
        let source_path = source.join(entry);
        if !source_path.exists() {
            continue;
        }
        let destination_path = destination.join(entry);
        if source_path.is_dir() {
            copy_directory(&source_path, &destination_path)?;
        } else {
            std::fs::copy(&source_path, &destination_path).map_err(|source| CoreError::Io {
                operation: "copy portable backup file",
                source,
            })?;
        }
    }
    Ok(())
}

/// Parses a recovery copy file name of the form
/// `<epochMillis>-<entityId>-<uuid>.map`, returning the epoch milliseconds when
/// the name matches the entity. Both the trailing UUID and the embedded entity
/// ID must validate so that files from other maps (or traversal attempts) are
/// never touched.
pub(super) fn parse_map_recovery_file_name(entity_id: &str, file_name: &str) -> Option<String> {
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

pub(super) fn digest_bytes(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

pub(super) fn streamed_file_digest(path: &Path) -> Result<(String, i64), CoreError> {
    let mut file =
        std::fs::File::open(path).map_err(|error| CoreError::NotFound(error.to_string()))?;
    let mut digest = Sha256::new();
    let mut buffer = vec![0_u8; 128 * 1024];
    let mut size = 0_i64;
    loop {
        let count = std::io::Read::read(&mut file, &mut buffer).map_err(|error| CoreError::Io {
            operation: "read asset source",
            source: error,
        })?;
        if count == 0 {
            break;
        }
        digest.update(&buffer[..count]);
        size = size
            .checked_add(count as i64)
            .ok_or_else(|| CoreError::Validation("asset is too large".into()))?;
    }
    Ok((format!("sha256:{:x}", digest.finalize()), size))
}
