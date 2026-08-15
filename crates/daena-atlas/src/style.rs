//! Bundled declarative atlas styles.

use serde::Deserialize;
use sha2::{Digest, Sha256};

use crate::{AtlasError, CODE_REQUEST_INVALID};

pub const RELIEF_STYLE_ID: &str = "daena-atlas-relief";
pub const ANTIQUE_STYLE_ID: &str = "daena-atlas-antique";
pub const SPIKE_STYLE_ALIAS: &str = crate::SPIKE_STYLE_ID;
pub const BUNDLED_FONT_ID: &str = "daena-atlas-bitmap-5x7";
pub const STYLE_VERSION: u32 = 1;

const RELIEF_JSON: &str =
    include_str!("../../../docs/maps/atlas/styles/daena-atlas-relief.v1.json");
const ANTIQUE_JSON: &str =
    include_str!("../../../docs/maps/atlas/styles/daena-atlas-antique.v1.json");

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct AtlasStyle {
    pub id: String,
    pub version: u32,
    pub title: String,
    pub font_id: String,
    pub paper_grain_ppm: u32,
    pub ocean_deep: [u8; 3],
    pub ocean_shallow: [u8; 3],
    pub land_low: [u8; 3],
    pub land_high: [u8; 3],
    pub land_peak: [u8; 3],
    pub lake: [u8; 3],
    pub ice: [u8; 3],
    pub river: [u8; 3],
    pub coast: [u8; 3],
    pub contour: [u8; 3],
    pub graticule: [u8; 3],
    pub frame: [u8; 3],
    pub background: [u8; 3],
    pub default_layer_ids: Vec<String>,
}

impl AtlasStyle {
    pub fn content_hash(&self, raw: &str) -> String {
        format!("sha256:{:x}", Sha256::digest(raw.as_bytes()))
    }
}

pub fn bundled_style_ids() -> [&'static str; 2] {
    [RELIEF_STYLE_ID, ANTIQUE_STYLE_ID]
}

pub fn resolve_style_id(id: &str) -> Result<&'static str, AtlasError> {
    match id {
        RELIEF_STYLE_ID | SPIKE_STYLE_ALIAS => Ok(RELIEF_STYLE_ID),
        ANTIQUE_STYLE_ID => Ok(ANTIQUE_STYLE_ID),
        _ => Err(AtlasError::new(
            CODE_REQUEST_INVALID,
            format!("unavailable atlas style: {id}"),
        )),
    }
}

pub fn load_style(id: &str) -> Result<(AtlasStyle, &'static str), AtlasError> {
    let canonical = resolve_style_id(id)?;
    let raw = match canonical {
        RELIEF_STYLE_ID => RELIEF_JSON,
        ANTIQUE_STYLE_ID => ANTIQUE_JSON,
        _ => {
            return Err(AtlasError::new(
                CODE_REQUEST_INVALID,
                format!("unavailable atlas style: {id}"),
            ))
        }
    };
    validate_style_source(raw)?;
    let style: AtlasStyle = serde_json::from_str(raw).map_err(|error| {
        AtlasError::new(
            CODE_REQUEST_INVALID,
            format!("invalid atlas style: {error}"),
        )
    })?;
    if style.id != canonical || style.version != STYLE_VERSION {
        return Err(AtlasError::new(
            CODE_REQUEST_INVALID,
            "atlas style identity does not match the bundled file",
        ));
    }
    if style.font_id != BUNDLED_FONT_ID {
        return Err(AtlasError::new(
            CODE_REQUEST_INVALID,
            "atlas style references an unavailable font",
        ));
    }
    if style.paper_grain_ppm > 250_000 {
        return Err(AtlasError::limit("paper grain exceeds the style budget"));
    }
    Ok((style, raw))
}

pub fn validate_style_source(raw: &str) -> Result<(), AtlasError> {
    let lower = raw.to_ascii_lowercase();
    for needle in [
        "http://",
        "https://",
        "javascript:",
        "file:",
        "<script",
        "shader",
    ] {
        if lower.contains(needle) {
            return Err(AtlasError::new(
                CODE_REQUEST_INVALID,
                format!("atlas style contains forbidden token {needle}"),
            ));
        }
    }
    Ok(())
}

pub fn mix_rgb(from: [u8; 3], to: [u8; 3], t_ppm: u32) -> [u8; 3] {
    let t = u32::from(t_ppm.min(1_000_000));
    let inv = 1_000_000 - t;
    [
        ((u32::from(from[0]) * inv + u32::from(to[0]) * t) / 1_000_000) as u8,
        ((u32::from(from[1]) * inv + u32::from(to[1]) * t) / 1_000_000) as u8,
        ((u32::from(from[2]) * inv + u32::from(to[2]) * t) / 1_000_000) as u8,
    ]
}

pub fn apply_shade(rgb: [u8; 3], shade_ppm: u32) -> [u8; 4] {
    let shade = shade_ppm.clamp(350_000, 1_000_000);
    [
        ((u32::from(rgb[0]) * shade) / 1_000_000) as u8,
        ((u32::from(rgb[1]) * shade) / 1_000_000) as u8,
        ((u32::from(rgb[2]) * shade) / 1_000_000) as u8,
        255,
    ]
}

pub fn hypsometric(style: &AtlasStyle, elevation_mm: i32, sea_level_mm: i32) -> [u8; 3] {
    let relative = elevation_mm.saturating_sub(sea_level_mm);
    if relative < 0 {
        let depth = relative.unsigned_abs().min(8_000_000);
        let t = (u64::from(depth) * 1_000_000 / 8_000_000) as u32;
        mix_rgb(style.ocean_shallow, style.ocean_deep, t)
    } else {
        let height = relative.unsigned_abs().min(6_000_000);
        if height < 1_500_000 {
            mix_rgb(
                style.land_low,
                style.land_high,
                (u64::from(height) * 1_000_000 / 1_500_000) as u32,
            )
        } else {
            mix_rgb(
                style.land_high,
                style.land_peak,
                (u64::from(height - 1_500_000) * 1_000_000 / 4_500_000) as u32,
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundled_styles_load_and_reject_urls() {
        let (relief, raw) = load_style(RELIEF_STYLE_ID).unwrap();
        let (antique, _) = load_style(ANTIQUE_STYLE_ID).unwrap();
        assert_ne!(relief.ocean_deep, antique.ocean_deep);
        assert_eq!(relief.font_id, BUNDLED_FONT_ID);
        assert!(raw.contains("http") == false);
        assert!(validate_style_source("{\"fill\":\"https://example\"}").is_err());
        assert!(load_style("missing-style").is_err());
    }
}
