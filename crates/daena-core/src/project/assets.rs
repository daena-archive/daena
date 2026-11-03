// Asset filename, role, and scope helpers.
use super::{
    Asset, ASSET_REFERENCE_SCOPE_ENTITY, ASSET_REFERENCE_SCOPE_PROJECT, ASSET_ROLE_ATTACHMENT,
    ASSET_ROLE_PROFILE,
};
use crate::error::CoreError;
use std::path::Path;
use uuid::Uuid;

/// Longest MIME type an asset may carry, in characters.
pub(super) const ASSET_MIME_TYPE_MAX_CHARS: usize = 255;

pub(super) fn validated_asset_filename(filename: &str) -> Result<String, CoreError> {
    let filename = filename.trim();
    if filename.is_empty()
        || matches!(filename, "." | "..")
        || filename.contains('/')
        || filename.contains('\\')
    {
        return Err(CoreError::Validation("asset filename is invalid".into()));
    }
    Ok(filename.into())
}

pub(super) fn imported_asset_category(filename: &str, mime_type: &str) -> &'static str {
    if mime_type.starts_with("image/") {
        "images"
    } else if mime_type.starts_with("video/") {
        "videos"
    } else if mime_type.contains("map")
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
    }
}

/// Filename and MIME type checks shared by upload begin and commit.
pub(super) fn validated_upload_filename(
    filename: &str,
    mime_type: &str,
) -> Result<String, CoreError> {
    let filename = validated_asset_filename(filename)?;
    if mime_type.trim().is_empty() {
        return Err(CoreError::Validation("asset MIME type is required".into()));
    }
    if mime_type.chars().count() > ASSET_MIME_TYPE_MAX_CHARS {
        return Err(CoreError::Validation(format!(
            "asset MIME type must be at most {ASSET_MIME_TYPE_MAX_CHARS} characters"
        )));
    }
    Ok(filename)
}

/// Portable checkpoint path for a newly registered asset: a category folder
/// chosen from its MIME type, then a unique prefix so names never collide.
pub(super) fn new_asset_portable_path(filename: &str, mime_type: &str) -> String {
    format!(
        "assets/{}/{}-{filename}",
        imported_asset_category(filename, mime_type),
        Uuid::new_v4()
    )
}

pub(super) fn validate_asset_role(role: &str) -> Result<(), CoreError> {
    if matches!(role, ASSET_ROLE_ATTACHMENT | ASSET_ROLE_PROFILE) {
        Ok(())
    } else {
        Err(CoreError::Validation(format!(
            "unsupported asset role: {role}"
        )))
    }
}

pub(super) fn validate_asset_reference_scope(reference_scope: &str) -> Result<(), CoreError> {
    if matches!(
        reference_scope,
        ASSET_REFERENCE_SCOPE_ENTITY | ASSET_REFERENCE_SCOPE_PROJECT
    ) {
        Ok(())
    } else {
        Err(CoreError::Validation(format!(
            "unsupported asset reference scope: {reference_scope}"
        )))
    }
}

pub(super) fn asset_can_be_profile_media(mime_type: &str) -> bool {
    matches!(
        mime_type,
        "image/png" | "image/jpeg" | "image/gif" | "image/webp"
    )
}

pub(super) fn renamed_asset_path(asset: &Asset, filename: &str) -> Result<String, CoreError> {
    let (parent, _) = asset
        .path
        .rsplit_once('/')
        .ok_or_else(|| CoreError::Validation("asset path has no parent directory".into()))?;
    if parent.is_empty() {
        return Err(CoreError::Validation(
            "asset path has no parent directory".into(),
        ));
    }
    Ok(format!("{parent}/{}-{filename}", asset.id))
}
