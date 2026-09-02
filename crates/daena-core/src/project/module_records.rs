// Module record SQL and validation helpers.
use super::fs_util::digest_bytes;
use super::ProjectSnapshot;
use crate::error::CoreError;
use serde::Serialize;
use std::path::Path;
use uuid::Uuid;

pub(super) fn staged_canonical_sources(
    root: &Path,
    snapshot: &ProjectSnapshot,
) -> Result<Vec<crate::storage::CanonicalSource>, CoreError> {
    fn visit(root: &Path, current: &Path, paths: &mut Vec<String>) -> Result<(), CoreError> {
        for entry in std::fs::read_dir(current).map_err(|source| CoreError::Io {
            operation: "read targeted staging directory",
            source,
        })? {
            let entry = entry.map_err(|source| CoreError::Io {
                operation: "read targeted staging entry",
                source,
            })?;
            let path = entry.path();
            if path.is_dir() {
                visit(root, &path, paths)?;
            } else if path.is_file() {
                paths.push(
                    path.strip_prefix(root)
                        .map_err(|error| CoreError::Validation(error.to_string()))?
                        .to_string_lossy()
                        .replace('\\', "/"),
                );
            }
        }
        Ok(())
    }

    let mut paths = Vec::new();
    for entity in &snapshot.entities {
        let path =
            crate::storage::normalized_project_path(root, &format!("entities/{}", entity.id))?;
        if path.is_dir() {
            visit(root, &path, &mut paths)?;
        }
    }
    for module in &snapshot.modules {
        let path = crate::storage::normalized_project_path(
            root,
            &format!("plugins/{}.json", module.module_id),
        )?;
        if path.is_file() {
            paths.push(format!("plugins/{}.json", module.module_id));
        }
    }
    for record in &snapshot.module_records {
        let path = crate::storage::normalized_project_path(
            root,
            &format!("plugins/{}.json", record.module_id),
        )?;
        if path.is_file() {
            paths.push(format!("plugins/{}.json", record.module_id));
        }
    }
    for asset in &snapshot.assets {
        let path = crate::storage::normalized_project_path(root, &asset.path)?;
        if path.is_file() {
            paths.push(asset.path.clone());
        }
    }
    paths.sort();
    paths.dedup();
    paths
        .into_iter()
        .map(|path| {
            let content_hash = crate::sync::hash_path(root, &path)?.ok_or_else(|| {
                CoreError::Validation(format!("targeted staged source is missing: {path}"))
            })?;
            Ok(crate::storage::CanonicalSource {
                path,
                content_hash,
                format_version: crate::storage::PROJECT_FORMAT_VERSION,
            })
        })
        .collect()
}

pub(super) fn optional_record_filter<'a>(
    value: Option<&'a str>,
    label: &str,
) -> Result<Option<&'a str>, CoreError> {
    let value = value.map(str::trim).filter(|item| !item.is_empty());
    if let Some(value) = value {
        if value.len() > 128 {
            return Err(CoreError::Validation(format!("{label} exceeds 128 bytes")));
        }
    }
    Ok(value)
}

pub(super) fn module_record_order_sql(sort: &str, alias: &str) -> Result<String, CoreError> {
    let id = format!("{alias}id");
    Ok(match sort {
        "" | "lemma" => format!("lower(json_extract({alias}value, '$.lemma')), {id}"),
        "symbol" => format!("lower(json_extract({alias}value, '$.symbol')), {id}"),
        "name" => format!("lower(json_extract({alias}value, '$.name')), {id}"),
        "title" => format!("lower(json_extract({alias}value, '$.title')), {id}"),
        "updatedAt" => format!("{alias}updated_at DESC, {id}"),
        "status" => format!(
            "lower(COALESCE(json_extract({alias}value, '$.status'), '')), lower(COALESCE(json_extract({alias}value, '$.lemma'), json_extract({alias}value, '$.name'), json_extract({alias}value, '$.symbol'), '')), {id}"
        ),
        _ => {
            return Err(CoreError::Validation(
                "record sort must be lemma, symbol, name, title, updatedAt, or status".into(),
            ))
        }
    })
}

pub(super) fn module_record_filter_sql(alias: &str) -> String {
    let json_source = if alias.is_empty() {
        "module_records.value".into()
    } else {
        format!("{alias}value")
    };
    format!(
        "AND (:status IS NULL OR json_extract({json_source}, '$.status') = :status) \
         AND (:tag IS NULL OR EXISTS (SELECT 1 FROM json_each({json_source}, '$.tags') AS tag_item WHERE tag_item.atom = :tag)) \
         AND (:homonyms = 0 OR lower(json_extract({json_source}, '$.lemma')) IN ( \
            SELECT lower(json_extract(value, '$.lemma')) FROM module_records \
            WHERE module_id=:module AND collection=:collection AND owner_entity_id=:owner \
            GROUP BY lower(json_extract(value, '$.lemma')) HAVING COUNT(*) > 1))"
    )
}

pub(super) fn validate_module_record_scope(
    module_id: &str,
    collection: &str,
    owner_entity_id: &str,
) -> Result<(), CoreError> {
    let valid_component = |value: &str| {
        !value.is_empty()
            && value.len() <= 128
            && value
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'-' | b'_'))
    };
    if !valid_component(module_id) {
        return Err(CoreError::Validation(
            "invalid module record module ID".into(),
        ));
    }
    if !valid_component(collection) {
        return Err(CoreError::Validation(
            "invalid module record collection".into(),
        ));
    }
    Uuid::parse_str(owner_entity_id)
        .map_err(|_| CoreError::Validation("module record owner must be a UUID".into()))?;
    Ok(())
}

pub(super) fn validate_module_record_input(
    module_id: &str,
    collection: &str,
    owner_entity_id: &str,
    value: &serde_json::Value,
) -> Result<(), CoreError> {
    validate_module_record_scope(module_id, collection, owner_entity_id)?;
    if !value.is_object() {
        return Err(CoreError::Validation(
            "module record value must be an object".into(),
        ));
    }
    let bytes =
        serde_json::to_vec(value).map_err(|error| CoreError::Serialization(error.to_string()))?;
    if bytes.len() > 64 * 1024 {
        return Err(CoreError::Validation("module record exceeds 64 KiB".into()));
    }
    Ok(())
}

pub(super) fn revision_digest<T: Serialize>(value: &T) -> Result<String, CoreError> {
    let bytes =
        serde_json::to_vec(value).map_err(|error| CoreError::Serialization(error.to_string()))?;
    Ok(format!("sha256:{}", digest_bytes(&bytes)))
}
