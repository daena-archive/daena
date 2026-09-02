// Archive and zip entry helpers.
use super::*;
use crate::CoreError;
use std::collections::BTreeSet;
use std::fs;
use std::io::{Cursor, Read};
use std::path::Path;
use zip::ZipArchive;

pub(super) const MAX_ARCHIVE_COMPRESSED_BYTES: u64 = 128 * 1024 * 1024;
pub(super) const MAX_ARCHIVE_COMPRESSION_RATIO: u64 = 200;
pub(super) const MAX_ARCHIVE_PATH_BYTES: usize = 1_024;
pub(super) const MAX_ARCHIVE_EXPANDED_BYTES: u64 = 512 * 1024 * 1024;
pub(super) const MAX_ARCHIVE_ENTRIES: usize = 20_000;
pub(super) const MAX_ARCHIVE_FILE_BYTES: u64 = 16 * 1024 * 1024;

pub(super) fn is_zip_path(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("zip"))
}

pub(super) fn validate_archive_source_path(
    raw_name: &[u8],
    is_dir: bool,
) -> Result<String, CoreError> {
    let name = std::str::from_utf8(raw_name)
        .map_err(|_| CoreError::Validation("ZIP entry path is not valid UTF-8".into()))?;
    if name.is_empty()
        || name.len() > MAX_ARCHIVE_PATH_BYTES
        || name.starts_with('/')
        || name.contains('\\')
        || name.contains(':')
        || name.chars().any(char::is_control)
    {
        return Err(CoreError::Validation(format!(
            "ZIP entry path is not portable: {name}"
        )));
    }
    let normalized = if is_dir {
        name.strip_suffix('/').unwrap_or(name)
    } else {
        name
    };
    if normalized.is_empty()
        || normalized
            .split('/')
            .any(|component| component.is_empty() || matches!(component, "." | ".."))
    {
        return Err(CoreError::Validation(format!(
            "ZIP entry path escapes or is not normalized: {name}"
        )));
    }
    Ok(normalized.into())
}

pub(crate) fn read_archive_asset_bytes(
    archive_path: &Path,
    target_path: &str,
    expected_size: u64,
) -> Result<Vec<u8>, CoreError> {
    read_archive_entry_bytes(archive_path, target_path, Some(expected_size))
}

pub(super) fn read_archive_entry_bytes(
    archive_path: &Path,
    target_path: &str,
    expected_size: Option<u64>,
) -> Result<Vec<u8>, CoreError> {
    let metadata = fs::symlink_metadata(archive_path).map_err(|source| CoreError::Io {
        operation: "read import ZIP metadata",
        source,
    })?;
    if metadata.file_type().is_symlink()
        || !metadata.is_file()
        || metadata.len() > MAX_ARCHIVE_COMPRESSED_BYTES
    {
        return Err(CoreError::Validation(
            "import ZIP source is unavailable or exceeds its compressed-size limit".into(),
        ));
    }
    let file = fs::File::open(archive_path).map_err(|source| CoreError::Io {
        operation: "open import ZIP archive",
        source,
    })?;
    let mut archive = ZipArchive::new(file)
        .map_err(|error| CoreError::Validation(format!("invalid ZIP archive: {error}")))?;
    if archive.len() > MAX_ARCHIVE_ENTRIES {
        return Err(CoreError::Validation(
            "ZIP archive exceeds the entry limit during asset preflight".into(),
        ));
    }
    let mut names = BTreeSet::new();
    let mut folded_names = BTreeSet::new();
    let mut expanded_bytes = 0_u64;
    let mut target_index = None;
    for index in 0..archive.len() {
        let entry = archive.by_index(index).map_err(|error| {
            CoreError::Validation(format!("invalid ZIP central-directory entry: {error}"))
        })?;
        let is_dir = entry.is_dir();
        let source_path = validate_archive_source_path(entry.name_raw(), is_dir)?;
        if !names.insert(source_path.clone()) || !folded_names.insert(source_path.to_lowercase()) {
            return Err(CoreError::Validation(format!(
                "ZIP archive contains duplicate or case-colliding path: {source_path}"
            )));
        }
        if !is_dir
            && entry
                .unix_mode()
                .is_some_and(|mode| mode & 0o170000 != 0 && mode & 0o170000 != 0o100000)
        {
            return Err(CoreError::Validation(format!(
                "ZIP links and special files are not allowed: {source_path}"
            )));
        }
        let size = entry.size();
        if !is_dir && size > MAX_ARCHIVE_FILE_BYTES {
            return Err(CoreError::Validation(format!(
                "ZIP entry exceeds the file-size limit: {source_path}"
            )));
        }
        expanded_bytes = expanded_bytes
            .checked_add(size)
            .ok_or_else(|| CoreError::Validation("ZIP expanded size overflowed".into()))?;
        if expanded_bytes > MAX_ARCHIVE_EXPANDED_BYTES {
            return Err(CoreError::Validation(
                "ZIP archive exceeds the expanded-size limit".into(),
            ));
        }
        let packed = entry.compressed_size();
        if size > 0 && (packed == 0 || size > packed.saturating_mul(MAX_ARCHIVE_COMPRESSION_RATIO))
        {
            return Err(CoreError::Validation(format!(
                "ZIP entry exceeds the compression-ratio limit: {source_path}"
            )));
        }
        if !is_dir && source_path == target_path {
            target_index = Some(index);
        }
    }
    let target_index = target_index.ok_or_else(|| {
        CoreError::Conflict(format!(
            "import ZIP asset disappeared after analysis: {target_path}"
        ))
    })?;
    let mut entry = archive
        .by_index(target_index)
        .map_err(|error| CoreError::Validation(format!("invalid ZIP asset entry: {error}")))?;
    if expected_size.is_some_and(|expected_size| entry.size() != expected_size) {
        return Err(CoreError::Conflict(format!(
            "import ZIP asset changed size after analysis: {target_path}"
        )));
    }
    let actual_size = entry.size();
    let mut bytes = Vec::with_capacity(actual_size.min(1024 * 1024) as usize);
    entry
        .by_ref()
        .take(actual_size.saturating_add(1))
        .read_to_end(&mut bytes)
        .map_err(|source| CoreError::Io {
            operation: "read import ZIP asset",
            source,
        })?;
    if bytes.len() as u64 != actual_size {
        return Err(CoreError::Conflict(format!(
            "import ZIP asset data changed after analysis: {target_path}"
        )));
    }
    Ok(bytes)
}

pub(crate) fn read_docx_import_asset_bytes(
    source_root: &Path,
    source_kind: &ImportSourceKind,
    asset_source_path: &str,
    expected_size: u64,
    expected_package_hash: &str,
) -> Result<Vec<u8>, CoreError> {
    let (container_path, entry_path) = asset_source_path.split_once("!/").ok_or_else(|| {
        CoreError::Validation("DOCX import asset path is missing its container boundary".into())
    })?;
    validate_source_path(container_path)?;
    if document_format(container_path) != Some("docx") {
        return Err(CoreError::Validation(
            "DOCX import asset container is not a DOCX source".into(),
        ));
    }
    let entry_path = validate_archive_source_path(entry_path.as_bytes(), false)?;
    let package_bytes = match source_kind {
        ImportSourceKind::File => {
            if source_root.file_name().and_then(|name| name.to_str()) != Some(container_path) {
                return Err(CoreError::Conflict(
                    "DOCX import source path changed after analysis".into(),
                ));
            }
            let metadata = fs::symlink_metadata(source_root).map_err(|source| CoreError::Io {
                operation: "read DOCX import source metadata",
                source,
            })?;
            if metadata.file_type().is_symlink()
                || !metadata.is_file()
                || metadata.len() > MAX_ARCHIVE_COMPRESSED_BYTES
            {
                return Err(CoreError::Validation(
                    "DOCX import source must remain a bounded regular file".into(),
                ));
            }
            fs::read(source_root).map_err(|source| CoreError::Io {
                operation: "read DOCX import source",
                source,
            })?
        }
        ImportSourceKind::Folder => {
            let path = crate::storage::normalized_project_path(source_root, container_path)?;
            let metadata = fs::symlink_metadata(&path).map_err(|source| CoreError::Io {
                operation: "read DOCX import source metadata",
                source,
            })?;
            if metadata.file_type().is_symlink()
                || !metadata.is_file()
                || metadata.len() > MAX_ARCHIVE_COMPRESSED_BYTES
            {
                return Err(CoreError::Validation(
                    "DOCX import source must remain a bounded regular file".into(),
                ));
            }
            fs::read(path).map_err(|source| CoreError::Io {
                operation: "read DOCX import source",
                source,
            })?
        }
        ImportSourceKind::Archive => read_archive_entry_bytes(source_root, container_path, None)?,
        _ => {
            return Err(CoreError::Validation(
                "this import source kind cannot provide DOCX attachments".into(),
            ));
        }
    };
    if hex_digest(&package_bytes) != expected_package_hash {
        return Err(CoreError::Conflict(format!(
            "DOCX import package changed after analysis: {container_path}"
        )));
    }
    let mut package = ZipArchive::new(Cursor::new(package_bytes.as_slice()))
        .map_err(|error| CoreError::Validation(format!("invalid DOCX package: {error}")))?;
    let entries = preflight_docx_package(&mut package)?;
    let content_types = read_docx_entry(&mut package, &entries, "[Content_Types].xml")?;
    let content_types = decode_docx_xml(&content_types, "[Content_Types].xml")?;
    validate_docx_content_types(&content_types)?;
    let bytes = read_docx_entry(&mut package, &entries, &entry_path)?;
    if bytes.len() as u64 != expected_size {
        return Err(CoreError::Conflict(format!(
            "DOCX import asset changed size after analysis: {asset_source_path}"
        )));
    }
    Ok(bytes)
}

pub(crate) fn is_docx_import_asset_source_path(source_path: &str) -> bool {
    source_path
        .split_once("!/")
        .and_then(|(container, entry)| (!entry.is_empty()).then_some(container))
        .is_some_and(|container| document_format(container) == Some("docx"))
}

pub(super) fn record_parent_folders(folders: &mut BTreeSet<String>, source_path: &str) {
    let parts = source_path.split('/').collect::<Vec<_>>();
    for end in 1..parts.len() {
        folders.insert(parts[..end].join("/"));
    }
}
