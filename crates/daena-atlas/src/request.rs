use serde::{Deserialize, Serialize};

use crate::projection::{AtlasExtent, AtlasProjection, ProjectedView};
use crate::style::{bundled_style_ids, load_style, resolve_style_id};
use crate::{
    AtlasError, ATLAS_DETAIL_ALGORITHM_VERSION, ATLAS_REQUEST_SCHEMA_VERSION, SPIKE_STYLE_ID,
};

pub const PREVIEW_MAX_WIDTH: u32 = 2048;
pub const PREVIEW_MAX_HEIGHT: u32 = 1024;
pub const MAX_PIXEL_COUNT: u64 = 33_554_432;
pub const TILE_SIZE: u32 = 512;
pub const TILE_HALO: u32 = 0;

pub const ATLAS_LAYER_ROLES: [&str; 10] = [
    "ocean",
    "relief",
    "ice",
    "lakes",
    "rivers",
    "coastlines",
    "contours",
    "graticule",
    "frame",
    "labels",
];

fn default_time_kind() -> String {
    "physical-offset-year".into()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DetailLevel {
    Standard,
    Detailed,
    Print,
}

impl DetailLevel {
    pub fn parse(value: &str) -> Result<Self, AtlasError> {
        match value {
            "standard" => Ok(Self::Standard),
            "detailed" => Ok(Self::Detailed),
            "print" => Ok(Self::Print),
            _ => Err(AtlasError::invalid(format!(
                "unsupported atlas detail level: {value}"
            ))),
        }
    }

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Standard => "standard",
            Self::Detailed => "detailed",
            Self::Print => "print",
        }
    }

    pub const fn lattice_factor(self) -> u32 {
        match self {
            Self::Standard => 4,
            Self::Detailed => 8,
            Self::Print => 16,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AtlasFormat {
    Png,
    Svg,
    Pdf,
}

impl AtlasFormat {
    pub fn parse(value: &str) -> Result<Self, AtlasError> {
        match value {
            "png" => Ok(Self::Png),
            "svg" => Ok(Self::Svg),
            "pdf" => Ok(Self::Pdf),
            other => Err(AtlasError::invalid(format!(
                "unsupported atlas format: {other}"
            ))),
        }
    }

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Png => "png",
            Self::Svg => "svg",
            Self::Pdf => "pdf",
        }
    }

    pub const fn encoder_id(self) -> &'static str {
        match self {
            Self::Png => "png-0.17-fast-nofilter",
            Self::Svg => "svg-1-embedded-png",
            Self::Pdf => "pdf-1.4-flate-rgb",
        }
    }
}

fn default_projection() -> AtlasProjection {
    AtlasProjection::Equirectangular
}

fn default_extent() -> AtlasExtent {
    AtlasExtent::world()
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AtlasRenderRequest {
    pub schema_version: u32,
    pub offset_years: i64,
    pub algorithm_version: u32,
    pub level: DetailLevel,
    pub variant: u32,
    pub style_id: String,
    pub width_px: u32,
    pub height_px: u32,
    pub dpi: u32,
    pub format: AtlasFormat,
    #[serde(default = "default_projection")]
    pub projection: AtlasProjection,
    #[serde(default = "default_extent")]
    pub extent: AtlasExtent,
    #[serde(default)]
    pub unlock_aspect: bool,
    #[serde(default)]
    pub active_layer_ids: Vec<String>,
    #[serde(default = "default_time_kind")]
    pub time_kind: String,
    #[serde(default)]
    pub authored_year: Option<i64>,
    #[serde(default)]
    pub binding_revision: Option<String>,
}

impl AtlasRenderRequest {
    pub fn spike_png(width_px: u32, height_px: u32) -> Result<Self, AtlasError> {
        Self::normalize(Self {
            schema_version: ATLAS_REQUEST_SCHEMA_VERSION,
            offset_years: 0,
            algorithm_version: ATLAS_DETAIL_ALGORITHM_VERSION,
            level: DetailLevel::Detailed,
            variant: 0,
            style_id: SPIKE_STYLE_ID.to_string(),
            width_px,
            height_px,
            dpi: 300,
            format: AtlasFormat::Png,
            projection: default_projection(),
            extent: default_extent(),
            unlock_aspect: false,
            active_layer_ids: Vec::new(),
            time_kind: default_time_kind(),
            authored_year: None,
            binding_revision: None,
        })
    }

    pub fn layer_enabled(&self, role: &str) -> bool {
        self.active_layer_ids.iter().any(|id| id == role)
    }

    pub fn view(&self) -> Result<ProjectedView, AtlasError> {
        ProjectedView::new(self.projection, self.extent, self.width_px, self.height_px)
    }

    pub fn normalize(mut self) -> Result<Self, AtlasError> {
        if self.schema_version != ATLAS_REQUEST_SCHEMA_VERSION {
            return Err(AtlasError::invalid("unsupported atlas request schema"));
        }
        if self.algorithm_version != ATLAS_DETAIL_ALGORITHM_VERSION {
            return Err(AtlasError::invalid("unsupported atlas detail algorithm"));
        }
        let canonical = resolve_style_id(&self.style_id)?;
        self.style_id = canonical.to_string();
        let (style, _) = load_style(&self.style_id)?;
        if self.active_layer_ids.is_empty() {
            self.active_layer_ids = style.default_layer_ids.clone();
        }
        self.active_layer_ids.sort();
        self.active_layer_ids.dedup();
        for id in &self.active_layer_ids {
            if !ATLAS_LAYER_ROLES.contains(&id.as_str()) && !looks_like_uuid(id) {
                return Err(AtlasError::invalid(format!(
                    "unsupported atlas layer role: {id}"
                )));
            }
        }
        if !matches!(
            self.time_kind.as_str(),
            "physical-offset-year" | "calendar-year"
        ) {
            return Err(AtlasError::invalid("unsupported atlas time kind"));
        }
        if self.width_px == 0 || self.height_px == 0 {
            return Err(AtlasError::invalid("output dimensions must be positive"));
        }
        let pixels = u64::from(self.width_px)
            .checked_mul(u64::from(self.height_px))
            .ok_or_else(|| AtlasError::limit("output pixel count overflowed"))?;
        if pixels > MAX_PIXEL_COUNT {
            return Err(AtlasError::limit("output exceeds the atlas pixel budget"));
        }
        self.extent.validate(self.projection)?;
        let view = ProjectedView::new(self.projection, self.extent, self.width_px, self.height_px)?;
        if !self.unlock_aspect {
            let (num, den) = view.aspect_num_den();
            let expected_width = ((u128::from(self.height_px) * num + den / 2) / den.max(1)) as u32;
            let expected_height = ((u128::from(self.width_px) * den + num / 2) / num.max(1)) as u32;
            if self.width_px.abs_diff(expected_width) > 1
                && self.height_px.abs_diff(expected_height) > 1
            {
                return Err(AtlasError::invalid(
                    "output aspect must match the projected extent unless unlockAspect is set",
                ));
            }
        }
        if self.dpi == 0 || self.dpi > 2_400 {
            return Err(AtlasError::invalid("DPI must be between 1 and 2400"));
        }
        if !(-daena_physical::history::MAX_HISTORICAL_OFFSET_YEARS
            ..=daena_physical::history::MAX_HISTORICAL_OFFSET_YEARS)
            .contains(&self.offset_years)
        {
            return Err(AtlasError::invalid("physical offset year is out of range"));
        }
        let _ = bundled_style_ids();
        Ok(self)
    }

    pub fn tile_count(&self) -> u32 {
        let tiles_x = self.width_px.div_ceil(TILE_SIZE);
        let tiles_y = self.height_px.div_ceil(TILE_SIZE);
        tiles_x.saturating_mul(tiles_y)
    }

    pub fn estimate(&self) -> ResourceEstimate {
        let pixels = u64::from(self.width_px) * u64::from(self.height_px);
        ResourceEstimate {
            pixel_count: pixels,
            rgba_bytes: pixels.saturating_mul(4),
            estimated_png_bytes: pixels.saturating_mul(4),
            tile_count: u64::from(self.tile_count()),
            print_width_inches_milli: u64::from(self.width_px) * 1000 / u64::from(self.dpi.max(1)),
            print_height_inches_milli: u64::from(self.height_px) * 1000
                / u64::from(self.dpi.max(1)),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceEstimate {
    pub pixel_count: u64,
    pub rgba_bytes: u64,
    pub estimated_png_bytes: u64,
    pub tile_count: u64,
    pub print_width_inches_milli: u64,
    pub print_height_inches_milli: u64,
}

pub fn physical_layer_role(layer_id: &str) -> Option<&'static str> {
    match layer_id {
        "ocean" | "bathymetry" | "shelves" => Some("ocean"),
        "land" | "base" => Some("relief"),
        "ice" => Some("ice"),
        "lakes" => Some("lakes"),
        "rivers" => Some("rivers"),
        "bathymetric-contours" => Some("contours"),
        "islands" => Some("coastlines"),
        "labels" => Some("labels"),
        other => ATLAS_LAYER_ROLES
            .iter()
            .copied()
            .find(|role| *role == other),
    }
}

fn looks_like_uuid(value: &str) -> bool {
    let bytes = value.as_bytes();
    bytes.len() == 36
        && bytes[8] == b'-'
        && bytes[13] == b'-'
        && bytes[18] == b'-'
        && bytes[23] == b'-'
        && bytes
            .iter()
            .enumerate()
            .all(|(index, byte)| matches!(index, 8 | 13 | 18 | 23) || byte.is_ascii_hexdigit())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_non_two_to_one_and_over_budget_dimensions() {
        let mut request = AtlasRenderRequest::spike_png(64, 32).unwrap();
        request.width_px = 80;
        assert_eq!(
            request.normalize().unwrap_err().code,
            crate::CODE_REQUEST_INVALID
        );
        request = AtlasRenderRequest::spike_png(64, 32).unwrap();
        request.extent.west_lon_micro = 0;
        request.extent.east_lon_micro = 10_000_000;
        request.extent.south_lat_micro = 0;
        request.extent.north_lat_micro = 10_000_000;
        assert_eq!(
            request.clone().normalize().unwrap_err().code,
            crate::CODE_REQUEST_INVALID
        );
        request.width_px = 32;
        request.height_px = 32;
        request.normalize().unwrap();
        request = AtlasRenderRequest::spike_png(64, 32).unwrap();
        request.width_px = 16_384;
        request.height_px = 8_192;
        assert_eq!(
            request.normalize().unwrap_err().code,
            crate::CODE_RESOURCE_LIMIT
        );
    }

    #[test]
    fn spike_alias_normalizes_to_relief_style() {
        let request = AtlasRenderRequest::spike_png(64, 32).unwrap();
        assert_eq!(request.style_id, crate::style::RELIEF_STYLE_ID);
        assert!(request.layer_enabled("relief"));
        assert!(request.estimate().pixel_count == 64 * 32);
    }
}
