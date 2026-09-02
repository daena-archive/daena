// Runtime asset storage helpers.
use super::fs_util::streamed_file_digest;
use crate::error::CoreError;
use sha2::{Digest, Sha256};
#[cfg(test)]
use std::cell::Cell;
use std::collections::BTreeSet;
use std::fs::OpenOptions;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use uuid::Uuid;

pub(super) fn project_database_path(root: &Path) -> PathBuf {
    root.join(".daena/index.sqlite")
}

#[cfg(test)]
thread_local! {
    pub(crate) static FAIL_NEXT_RUNTIME_ASSET_INSTALL: Cell<bool> = const { Cell::new(false) };
}

pub(super) fn runtime_asset_path(root: &Path, content_hash: &str) -> Result<PathBuf, CoreError> {
    let digest = content_hash
        .strip_prefix("sha256:")
        .ok_or_else(|| CoreError::Validation("asset content hash must use sha256".into()))?;
    if digest.len() != 64 || !digest.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(CoreError::Validation(
            "asset content hash must contain a 64-character SHA-256 digest".into(),
        ));
    }
    Ok(root.join(".daena/assets").join(digest.to_ascii_lowercase()))
}

pub(super) fn store_runtime_asset<R: Read>(
    root: &Path,
    mut input: R,
    expected_hash: Option<&str>,
) -> Result<(String, i64), CoreError> {
    #[cfg(test)]
    if FAIL_NEXT_RUNTIME_ASSET_INSTALL.with(|flag| flag.replace(false)) {
        return Err(CoreError::Io {
            operation: "install runtime asset",
            source: std::io::Error::other("injected asset install failure"),
        });
    }
    let directory = root.join(".daena/assets");
    std::fs::create_dir_all(&directory).map_err(|source| CoreError::Io {
        operation: "create runtime asset directory",
        source,
    })?;
    let temporary = directory.join(format!(".tmp-{}", Uuid::new_v4()));
    let mut output = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temporary)
        .map_err(|source| CoreError::Io {
            operation: "create runtime asset staging file",
            source,
        })?;
    let result = (|| {
        let mut digest = Sha256::new();
        let mut buffer = vec![0_u8; 128 * 1024];
        let mut size = 0_i64;
        loop {
            let count = input.read(&mut buffer).map_err(|source| CoreError::Io {
                operation: "read runtime asset input",
                source,
            })?;
            if count == 0 {
                break;
            }
            output
                .write_all(&buffer[..count])
                .map_err(|source| CoreError::Io {
                    operation: "write runtime asset staging file",
                    source,
                })?;
            digest.update(&buffer[..count]);
            size = size
                .checked_add(count as i64)
                .ok_or_else(|| CoreError::Validation("asset is too large".into()))?;
        }
        output.sync_all().map_err(|source| CoreError::Io {
            operation: "sync runtime asset staging file",
            source,
        })?;
        let content_hash = format!("sha256:{:x}", digest.finalize());
        if expected_hash.is_some_and(|expected| expected != content_hash) {
            return Err(CoreError::Validation(
                "asset bytes do not match the declared content hash".into(),
            ));
        }
        let destination = runtime_asset_path(root, &content_hash)?;
        if destination.is_file() {
            let (existing_hash, existing_size) = streamed_file_digest(&destination)?;
            if existing_hash != content_hash || existing_size != size {
                return Err(CoreError::Conflict(
                    "runtime asset store contains corrupted bytes".into(),
                ));
            }
            std::fs::remove_file(&temporary).map_err(|source| CoreError::Io {
                operation: "remove duplicate runtime asset staging file",
                source,
            })?;
        } else {
            std::fs::rename(&temporary, &destination).map_err(|source| CoreError::Io {
                operation: "install runtime asset",
                source,
            })?;
            crate::sync::sync_directory(&directory)?;
        }
        Ok((content_hash, size))
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(&temporary);
    }
    result
}

pub(super) fn store_runtime_asset_file(
    root: &Path,
    source: &Path,
    expected_hash: Option<&str>,
) -> Result<(String, i64), CoreError> {
    let input = std::fs::File::open(source).map_err(|source| CoreError::Io {
        operation: "open runtime asset input",
        source,
    })?;
    store_runtime_asset(root, input, expected_hash)
}

#[derive(Default)]
pub(super) struct RuntimeAssetInstallGuard {
    installed_paths: BTreeSet<PathBuf>,
    committed: bool,
}

impl RuntimeAssetInstallGuard {
    pub(super) fn track(&mut self, path: PathBuf) {
        self.installed_paths.insert(path);
    }

    pub(super) fn commit(&mut self) {
        self.committed = true;
    }
}

impl Drop for RuntimeAssetInstallGuard {
    fn drop(&mut self) {
        if self.committed {
            return;
        }
        for path in &self.installed_paths {
            let _ = std::fs::remove_file(path);
            if let Some(parent) = path.parent() {
                let _ = crate::sync::sync_directory(parent);
            }
        }
    }
}

pub(super) fn ensure_runtime_asset(
    root: &Path,
    portable_path: &str,
    content_hash: &str,
    size: i64,
) -> Result<(), CoreError> {
    let runtime_path = runtime_asset_path(root, content_hash)?;
    let (actual_hash, actual_size) = if runtime_path.is_file() {
        streamed_file_digest(&runtime_path)?
    } else {
        let source = crate::storage::normalized_project_path(root, portable_path)?;
        store_runtime_asset_file(root, &source, Some(content_hash))?
    };
    if actual_hash != content_hash || actual_size != size {
        return Err(CoreError::Validation(format!(
            "asset bytes do not match metadata for {portable_path}"
        )));
    }
    Ok(())
}
