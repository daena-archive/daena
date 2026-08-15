use crate::{
    AtlasError, ATLAS_DETAIL_ALGORITHM_VERSION, ATLAS_REQUEST_SCHEMA_VERSION, SPIKE_STYLE_ID,
};

pub const PREVIEW_MAX_WIDTH: u32 = 2048;
pub const PREVIEW_MAX_HEIGHT: u32 = 1024;
pub const MAX_PIXEL_COUNT: u64 = 33_554_432;
pub const TILE_SIZE: u32 = 512;
pub const TILE_HALO: u32 = 0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AtlasFormat {
    Png,
}

impl AtlasFormat {
    pub fn parse(value: &str) -> Result<Self, AtlasError> {
        match value {
            "png" => Ok(Self::Png),
            other => Err(AtlasError::invalid(format!(
                "unsupported atlas format: {other}"
            ))),
        }
    }

    pub const fn as_str(self) -> &'static str {
        "png"
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
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
        })
    }

    pub fn normalize(self) -> Result<Self, AtlasError> {
        if self.schema_version != ATLAS_REQUEST_SCHEMA_VERSION {
            return Err(AtlasError::invalid("unsupported atlas request schema"));
        }
        if self.algorithm_version != ATLAS_DETAIL_ALGORITHM_VERSION {
            return Err(AtlasError::invalid("unsupported atlas detail algorithm"));
        }
        if self.style_id != SPIKE_STYLE_ID {
            return Err(AtlasError::new(
                crate::CODE_REQUEST_INVALID,
                format!("unavailable atlas style: {}", self.style_id),
            ));
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
        if u64::from(self.width_px) != u64::from(self.height_px) * 2 {
            return Err(AtlasError::invalid(
                "iteration-0 atlas output must be whole-world 2:1 equirectangular",
            ));
        }
        if self.dpi == 0 || self.dpi > 2_400 {
            return Err(AtlasError::invalid("DPI must be between 1 and 2400"));
        }
        Ok(self)
    }

    pub fn tile_count(&self) -> u32 {
        let tiles_x = self.width_px.div_ceil(TILE_SIZE);
        let tiles_y = self.height_px.div_ceil(TILE_SIZE);
        tiles_x.saturating_mul(tiles_y)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_non_two_to_one_and_over_budget_dimensions() {
        let mut request = AtlasRenderRequest::spike_png(64, 32).unwrap();
        request.width_px = 65;
        assert_eq!(
            request.normalize().unwrap_err().code,
            crate::CODE_REQUEST_INVALID
        );
        request = AtlasRenderRequest::spike_png(64, 32).unwrap();
        request.width_px = 16_384;
        request.height_px = 8_192;
        assert_eq!(
            request.normalize().unwrap_err().code,
            crate::CODE_RESOURCE_LIMIT
        );
    }
}
