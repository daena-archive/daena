//! Bundled declarative atlas styles.

use serde::Deserialize;
use sha2::{Digest, Sha256};

use crate::{AtlasError, CODE_REQUEST_INVALID};

pub const RELIEF_STYLE_ID: &str = "daena-atlas-relief";
pub const ANTIQUE_STYLE_ID: &str = "daena-atlas-antique";
pub const POLITICAL_STYLE_ID: &str = "daena-atlas-political";
pub const BIOME_STYLE_ID: &str = "daena-atlas-biome";
pub const TEMPERATURE_STYLE_ID: &str = "daena-atlas-temperature";
pub const PRECIPITATION_STYLE_ID: &str = "daena-atlas-precipitation";
pub const HUMIDITY_STYLE_ID: &str = "daena-atlas-humidity";
pub const ARIDITY_STYLE_ID: &str = "daena-atlas-aridity";
pub const BATHYMETRY_STYLE_ID: &str = "daena-atlas-bathymetry";
pub const HYDROLOGY_STYLE_ID: &str = "daena-atlas-hydrology";
pub const SPIKE_STYLE_ALIAS: &str = crate::SPIKE_STYLE_ID;
pub const BUNDLED_FONT_ID: &str = "daena-atlas-bitmap-5x7";
pub const STYLE_VERSION: u32 = 1;

const RELIEF_JSON: &str =
    include_str!("../../../docs/maps/atlas/styles/daena-atlas-relief.v1.json");
const ANTIQUE_JSON: &str =
    include_str!("../../../docs/maps/atlas/styles/daena-atlas-antique.v1.json");
const POLITICAL_JSON: &str =
    include_str!("../../../docs/maps/atlas/styles/daena-atlas-political.v1.json");
const BIOME_JSON: &str = include_str!("../../../docs/maps/atlas/styles/daena-atlas-biome.v1.json");
const TEMPERATURE_JSON: &str =
    include_str!("../../../docs/maps/atlas/styles/daena-atlas-temperature.v1.json");
const PRECIPITATION_JSON: &str =
    include_str!("../../../docs/maps/atlas/styles/daena-atlas-precipitation.v1.json");
const HUMIDITY_JSON: &str =
    include_str!("../../../docs/maps/atlas/styles/daena-atlas-humidity.v1.json");
const ARIDITY_JSON: &str =
    include_str!("../../../docs/maps/atlas/styles/daena-atlas-aridity.v1.json");
const BATHYMETRY_JSON: &str =
    include_str!("../../../docs/maps/atlas/styles/daena-atlas-bathymetry.v1.json");
const HYDROLOGY_JSON: &str =
    include_str!("../../../docs/maps/atlas/styles/daena-atlas-hydrology.v1.json");

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
    pub land_snow: [u8; 3],
    pub lake: [u8; 3],
    pub ice: [u8; 3],
    pub river: [u8; 3],
    pub coast: [u8; 3],
    pub contour: [u8; 3],
    pub graticule: [u8; 3],
    pub frame: [u8; 3],
    pub background: [u8; 3],
    pub political: [u8; 3],
    pub label_ink: [u8; 3],
    pub biome_tundra: [u8; 3],
    pub biome_arid: [u8; 3],
    pub biome_grassland: [u8; 3],
    pub biome_forest: [u8; 3],
    pub default_layer_ids: Vec<String>,
}

impl AtlasStyle {
    #[must_use]
    pub fn content_hash(&self, raw: &str) -> String {
        format!("sha256:{:x}", Sha256::digest(raw.as_bytes()))
    }
}

#[must_use]
pub fn bundled_style_ids() -> [&'static str; 10] {
    [
        RELIEF_STYLE_ID,
        BIOME_STYLE_ID,
        TEMPERATURE_STYLE_ID,
        PRECIPITATION_STYLE_ID,
        HUMIDITY_STYLE_ID,
        ARIDITY_STYLE_ID,
        BATHYMETRY_STYLE_ID,
        HYDROLOGY_STYLE_ID,
        ANTIQUE_STYLE_ID,
        POLITICAL_STYLE_ID,
    ]
}

pub fn resolve_style_id(id: &str) -> Result<&'static str, AtlasError> {
    match id {
        RELIEF_STYLE_ID | SPIKE_STYLE_ALIAS => Ok(RELIEF_STYLE_ID),
        ANTIQUE_STYLE_ID => Ok(ANTIQUE_STYLE_ID),
        POLITICAL_STYLE_ID => Ok(POLITICAL_STYLE_ID),
        BIOME_STYLE_ID => Ok(BIOME_STYLE_ID),
        TEMPERATURE_STYLE_ID => Ok(TEMPERATURE_STYLE_ID),
        PRECIPITATION_STYLE_ID => Ok(PRECIPITATION_STYLE_ID),
        HUMIDITY_STYLE_ID => Ok(HUMIDITY_STYLE_ID),
        ARIDITY_STYLE_ID => Ok(ARIDITY_STYLE_ID),
        BATHYMETRY_STYLE_ID => Ok(BATHYMETRY_STYLE_ID),
        HYDROLOGY_STYLE_ID => Ok(HYDROLOGY_STYLE_ID),
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
        POLITICAL_STYLE_ID => POLITICAL_JSON,
        BIOME_STYLE_ID => BIOME_JSON,
        TEMPERATURE_STYLE_ID => TEMPERATURE_JSON,
        PRECIPITATION_STYLE_ID => PRECIPITATION_JSON,
        HUMIDITY_STYLE_ID => HUMIDITY_JSON,
        ARIDITY_STYLE_ID => ARIDITY_JSON,
        BATHYMETRY_STYLE_ID => BATHYMETRY_JSON,
        HYDROLOGY_STYLE_ID => HYDROLOGY_JSON,
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

#[must_use]
pub fn mix_rgb(from: [u8; 3], to: [u8; 3], t_ppm: u32) -> [u8; 3] {
    let t = t_ppm.min(1_000_000);
    let inv = 1_000_000 - t;
    [
        ((u32::from(from[0]) * inv + u32::from(to[0]) * t) / 1_000_000) as u8,
        ((u32::from(from[1]) * inv + u32::from(to[1]) * t) / 1_000_000) as u8,
        ((u32::from(from[2]) * inv + u32::from(to[2]) * t) / 1_000_000) as u8,
    ]
}

#[must_use]
pub fn apply_shade(rgb: [u8; 3], shade_ppm: u32) -> [u8; 4] {
    let shade = shade_ppm.clamp(350_000, 1_000_000);
    [
        ((u32::from(rgb[0]) * shade) / 1_000_000) as u8,
        ((u32::from(rgb[1]) * shade) / 1_000_000) as u8,
        ((u32::from(rgb[2]) * shade) / 1_000_000) as u8,
        255,
    ]
}

#[must_use]
pub fn biome_fill(style: &AtlasStyle, climate_class: i32) -> [u8; 3] {
    match climate_class {
        crate::control::CLIMATE_CLASS_ICE => style.ice,
        crate::control::CLIMATE_CLASS_TUNDRA => style.biome_tundra,
        crate::control::CLIMATE_CLASS_ALPINE => {
            mix_rgb(style.biome_tundra, style.land_peak, 350_000)
        }
        crate::control::CLIMATE_CLASS_ARID => style.biome_arid,
        crate::control::CLIMATE_CLASS_SHRUBLAND => {
            mix_rgb(style.biome_arid, style.biome_grassland, 450_000)
        }
        crate::control::CLIMATE_CLASS_COLD_GRASSLAND => {
            mix_rgb(style.biome_grassland, style.biome_tundra, 280_000)
        }
        crate::control::CLIMATE_CLASS_FOREST => style.biome_forest,
        crate::control::CLIMATE_CLASS_TROPICAL_FOREST => {
            mix_rgb(style.biome_forest, [18, 72, 36], 280_000)
        }
        crate::control::CLIMATE_CLASS_GRASSLAND => style.biome_grassland,
        crate::control::CLIMATE_CLASS_OCEAN => style.ocean_shallow,
        _ => [120, 120, 124],
    }
}

#[must_use]
pub fn ramp3(style: &AtlasStyle, t_ppm: u32) -> [u8; 3] {
    let t = t_ppm.min(1_000_000);
    if t < 500_000 {
        mix_rgb(style.land_low, style.land_high, t.saturating_mul(2))
    } else {
        mix_rgb(
            style.land_high,
            style.land_peak,
            t.saturating_sub(500_000).saturating_mul(2),
        )
    }
}

#[must_use]
pub fn temperature_fill(style: &AtlasStyle, centi_c: i32) -> [u8; 3] {
    const LO: i32 = -3_500;
    const HI: i32 = 4_000;
    let span = (HI - LO) as u64;
    let t = ((centi_c.clamp(LO, HI) - LO) as u64 * 1_000_000 / span) as u32;
    ramp3(style, t)
}

#[must_use]
pub fn precipitation_fill(style: &AtlasStyle, precip_mm: i32) -> [u8; 3] {
    const HI: i32 = 2_800;
    let t = (i64::from(precip_mm.clamp(0, HI)) * 1_000_000 / i64::from(HI)) as u32;
    ramp3(style, t)
}

#[must_use]
pub fn humidity_fill(style: &AtlasStyle, humidity_ppm: i32) -> [u8; 3] {
    let t = (i64::from(humidity_ppm.clamp(0, 1_000_000)) * 1_000_000 / 1_000_000) as u32;
    ramp3(style, t)
}

#[must_use]
pub fn aridity_fill(style: &AtlasStyle, aridity_ppm: i32) -> [u8; 3] {
    let t = (i64::from(aridity_ppm.clamp(0, 1_000_000)) * 1_000_000 / 1_000_000) as u32;
    ramp3(style, t)
}

#[must_use]
pub fn hypsometric(style: &AtlasStyle, elevation_mm: i32, sea_level_mm: i32) -> [u8; 3] {
    let relative = elevation_mm.saturating_sub(sea_level_mm);
    if relative < 0 {
        let depth = relative.unsigned_abs().min(8_000_000);
        let t = (u64::from(depth) * 1_000_000 / 8_000_000) as u32;
        mix_rgb(style.ocean_shallow, style.ocean_deep, t)
    } else {
        let height = relative.unsigned_abs().min(6_000_000);
        if height < 400_000 {
            mix_rgb(
                style.land_low,
                style.land_high,
                (u64::from(height) * 1_000_000 / 400_000) as u32,
            )
        } else if height < 2_500_000 {
            mix_rgb(
                style.land_high,
                style.land_peak,
                (u64::from(height - 400_000) * 1_000_000 / 2_100_000) as u32,
            )
        } else {
            mix_rgb(
                style.land_peak,
                style.land_snow,
                (u64::from(height - 2_500_000) * 1_000_000 / 3_500_000) as u32,
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
        assert!(!raw.contains("http"));
        assert!(validate_style_source("{\"fill\":\"https://example\"}").is_err());
        let (biome, _) = load_style(BIOME_STYLE_ID).unwrap();
        assert_eq!(biome.id, BIOME_STYLE_ID);
        assert_ne!(biome.biome_forest, biome.biome_arid);
        assert_ne!(
            biome_fill(&biome, crate::control::CLIMATE_CLASS_ALPINE),
            biome_fill(&biome, crate::control::CLIMATE_CLASS_TUNDRA)
        );
        assert_ne!(
            biome_fill(&biome, crate::control::CLIMATE_CLASS_TROPICAL_FOREST),
            biome_fill(&biome, crate::control::CLIMATE_CLASS_FOREST)
        );
        assert_ne!(
            biome_fill(&biome, crate::control::CLIMATE_CLASS_SHRUBLAND),
            biome_fill(&biome, crate::control::CLIMATE_CLASS_ARID)
        );
        assert_ne!(biome_fill(&biome, 99), biome.biome_grassland);
        assert_eq!(bundled_style_ids().len(), 10);
        let (temperature, _) = load_style(TEMPERATURE_STYLE_ID).unwrap();
        let (precip, _) = load_style(PRECIPITATION_STYLE_ID).unwrap();
        let (bathy, _) = load_style(BATHYMETRY_STYLE_ID).unwrap();
        let (hydro, _) = load_style(HYDROLOGY_STYLE_ID).unwrap();
        assert_ne!(temperature.land_low, temperature.land_peak);
        assert_ne!(precip.land_low, precip.land_peak);
        assert_ne!(bathy.ocean_deep, relief.ocean_deep);
        assert!(hydro.default_layer_ids.iter().any(|id| id == "watersheds"));
        assert_ne!(
            temperature_fill(&temperature, -2_000),
            temperature_fill(&temperature, 3_000)
        );
        assert_ne!(
            precipitation_fill(&precip, 80),
            precipitation_fill(&precip, 2_200)
        );
        let (humidity, _) = load_style(HUMIDITY_STYLE_ID).unwrap();
        let (aridity, _) = load_style(ARIDITY_STYLE_ID).unwrap();
        assert_ne!(
            humidity_fill(&humidity, 80_000),
            humidity_fill(&humidity, 900_000)
        );
        assert_ne!(
            aridity_fill(&aridity, 80_000),
            aridity_fill(&aridity, 900_000)
        );
        let mid_mountain = hypsometric(&relief, 3_000_000, 0);
        assert_ne!(mid_mountain, [236, 236, 228]);
    }
}
