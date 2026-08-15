use serde::{Deserialize, Serialize};

use crate::request::AtlasRenderRequest;
use crate::{
    ATLAS_DERIVED_DRAINAGE_VERSION, ATLAS_DETAIL_ALGORITHM_VERSION,
    ATLAS_PROVENANCE_SCHEMA_VERSION, ATLAS_RENDERER_VERSION, ATLAS_REQUEST_SCHEMA_VERSION,
    ATLAS_SEED_POLICY_VERSION,
};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AtlasRenderProvenanceV1 {
    pub schema_version: u32,
    pub renderer_version: u32,
    pub request_schema_version: u32,
    pub detail_algorithm_version: u32,
    pub seed_policy_version: u32,
    #[serde(default)]
    pub derived_drainage_version: u32,
    pub style_id: String,
    pub physical_identity: String,
    pub source_sha256: String,
    pub offset_years: i64,
    pub detail_level: String,
    pub detail_variant: u32,
    pub projection: String,
    pub west_lon_micro: i32,
    pub south_lat_micro: i32,
    pub east_lon_micro: i32,
    pub north_lat_micro: i32,
    pub width_px: u32,
    pub height_px: u32,
    pub dpi: u32,
    pub format: String,
    pub encoder: String,
    pub style_hash: String,
    pub active_layer_ids: Vec<String>,
    #[serde(default)]
    pub tributary_count: u32,
}

impl AtlasRenderProvenanceV1 {
    pub fn for_request(
        request: &AtlasRenderRequest,
        physical_identity: &[u8],
        source_sha256: &str,
        style_hash: &str,
    ) -> Self {
        Self {
            schema_version: ATLAS_PROVENANCE_SCHEMA_VERSION,
            renderer_version: ATLAS_RENDERER_VERSION,
            request_schema_version: ATLAS_REQUEST_SCHEMA_VERSION,
            detail_algorithm_version: ATLAS_DETAIL_ALGORITHM_VERSION,
            seed_policy_version: ATLAS_SEED_POLICY_VERSION,
            derived_drainage_version: ATLAS_DERIVED_DRAINAGE_VERSION,
            style_id: request.style_id.clone(),
            physical_identity: String::from_utf8_lossy(physical_identity).into_owned(),
            source_sha256: source_sha256.to_string(),
            offset_years: request.offset_years,
            detail_level: request.level.as_str().to_string(),
            detail_variant: request.variant,
            projection: request.projection.as_str().to_string(),
            west_lon_micro: request.extent.west_lon_micro,
            south_lat_micro: request.extent.south_lat_micro,
            east_lon_micro: request.extent.east_lon_micro,
            north_lat_micro: request.extent.north_lat_micro,
            width_px: request.width_px,
            height_px: request.height_px,
            dpi: request.dpi,
            format: request.format.as_str().to_string(),
            encoder: request.format.encoder_id().to_string(),
            style_hash: style_hash.to_string(),
            active_layer_ids: request.active_layer_ids.clone(),
            tributary_count: 0,
        }
    }

    pub fn spike(
        request: &AtlasRenderRequest,
        physical_identity: &[u8],
        source_sha256: &str,
    ) -> Self {
        Self::for_request(request, physical_identity, source_sha256, "sha256:spike")
    }

    pub fn compact_json(&self) -> Result<String, crate::AtlasError> {
        serde_json::to_string(self).map_err(|error| {
            crate::AtlasError::new(crate::CODE_ENCODER_FAILED, format!("provenance: {error}"))
        })
    }
}
