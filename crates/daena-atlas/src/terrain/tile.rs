//! Web Mercator PNG tiles for [`AtlasTerrain`](super::AtlasTerrain).

use std::sync::OnceLock;

use crate::style::{load_style, AtlasStyle, IMAGERY_STYLE_ID, RELIEF_STYLE_ID};
use crate::terrain::{render_imagery_pixels, render_pixels, AtlasTerrain};
use crate::{AtlasError, CODE_ENCODER_FAILED, CODE_REQUEST_INVALID};

/// Highest Web Mercator zoom the terrain renderer serves.
pub const TERRAIN_MAX_ZOOM: u32 = 12;

const TILE_PX: u32 = 256;

/// Base-map paint for one terrain tile.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TerrainStyle {
    Imagery,
    Relief,
}

/// Map a bundled style id onto the terrain renderer.
///
/// Any other id stays on the legacy Atlas pipeline.
#[must_use]
pub fn terrain_style_for(style_id: &str) -> Option<TerrainStyle> {
    match style_id {
        IMAGERY_STYLE_ID => Some(TerrainStyle::Imagery),
        RELIEF_STYLE_ID => Some(TerrainStyle::Relief),
        _ => None,
    }
}

/// Render one 256×256 Web Mercator tile and encode it as PNG.
///
/// `x` and `y` are tile indices. The pixel buffer is `256 * device_scale` on
/// each side. Relief loads its bundled style once for the process.
pub fn render_terrain_tile_png(
    terrain: &AtlasTerrain,
    style: TerrainStyle,
    z: u32,
    x: u32,
    y: u32,
    device_scale: u32,
) -> Result<Vec<u8>, AtlasError> {
    if z > TERRAIN_MAX_ZOOM {
        return Err(AtlasError::invalid("atlas terrain zoom exceeds 12"));
    }
    if device_scale != 1 && device_scale != 2 {
        return Err(AtlasError::invalid(
            "atlas terrain device scale must be 1 or 2",
        ));
    }
    let n = 1u32
        .checked_shl(z)
        .ok_or_else(|| AtlasError::invalid("atlas terrain zoom exceeds 12"))?;
    if x >= n || y >= n {
        return Err(AtlasError::invalid(
            "atlas terrain tile is outside the zoom",
        ));
    }
    let gx = x.saturating_mul(TILE_PX);
    let gy = y.saturating_mul(TILE_PX);
    let rgba = match style {
        TerrainStyle::Imagery => {
            render_imagery_pixels(terrain, z, gx, gy, TILE_PX, TILE_PX, device_scale)
        }
        TerrainStyle::Relief => {
            let relief = relief_style()?;
            render_pixels(terrain, relief, z, gx, gy, TILE_PX, TILE_PX, device_scale)
        }
    };
    let side = TILE_PX.saturating_mul(device_scale);
    let expected = (side as usize)
        .saturating_mul(side as usize)
        .saturating_mul(4);
    if rgba.len() != expected {
        return Err(AtlasError::new(
            CODE_ENCODER_FAILED,
            "atlas terrain tile has the wrong pixel count",
        ));
    }
    encode_rgba_png(side, side, &rgba)
}

fn relief_style() -> Result<&'static AtlasStyle, AtlasError> {
    static STYLE: OnceLock<Result<AtlasStyle, String>> = OnceLock::new();
    let cached = STYLE.get_or_init(|| {
        load_style(RELIEF_STYLE_ID)
            .map(|(style, _)| style)
            .map_err(|error| error.message)
    });
    match cached {
        Ok(style) => Ok(style),
        Err(message) => Err(AtlasError::new(CODE_REQUEST_INVALID, message.clone())),
    }
}

fn encode_rgba_png(width: u32, height: u32, rgba: &[u8]) -> Result<Vec<u8>, AtlasError> {
    let mut bytes = Vec::new();
    let mut encoder = png::Encoder::new(&mut bytes, width, height);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header().map_err(|error| {
        AtlasError::new(
            CODE_ENCODER_FAILED,
            format!("atlas terrain png header: {error}"),
        )
    })?;
    writer.write_image_data(rgba).map_err(|error| {
        AtlasError::new(CODE_ENCODER_FAILED, format!("atlas terrain png: {error}"))
    })?;
    drop(writer);
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;
    use std::sync::OnceLock;

    use daena_physical::engine::{self, Settings, WorldPreset};
    use daena_physical::{NoopProgress, DEFAULT_RADIUS_METRES};
    use rayon::ThreadPoolBuilder;

    use super::*;

    fn fixture() -> &'static AtlasTerrain {
        static TERRAIN: OnceLock<AtlasTerrain> = OnceLock::new();
        TERRAIN.get_or_init(|| {
            let settings = Settings::from_parts(
                831_429,
                WorldPreset::Earthlike,
                64,
                DEFAULT_RADIUS_METRES,
                0,
                daena_physical::planetary::PlanetaryConfiguration::earth_like(),
            );
            let products = engine::generate(settings, &mut NoopProgress).expect("n=64 world");
            AtlasTerrain::from_source(products.source(), None).expect("terrain")
        })
    }

    fn png_size(bytes: &[u8]) -> (u32, u32) {
        let decoder = png::Decoder::new(Cursor::new(bytes));
        let reader = decoder.read_info().expect("png");
        (reader.info().width, reader.info().height)
    }

    #[test]
    fn terrain_style_for_maps_imagery_and_relief_only() {
        assert_eq!(
            terrain_style_for(IMAGERY_STYLE_ID),
            Some(TerrainStyle::Imagery)
        );
        assert_eq!(
            terrain_style_for(RELIEF_STYLE_ID),
            Some(TerrainStyle::Relief)
        );
        assert_eq!(terrain_style_for("daena-atlas-biome"), None);
        assert_eq!(terrain_style_for("daena-atlas-relief-spike"), None);
        assert_eq!(terrain_style_for(""), None);
    }

    #[test]
    fn terrain_tile_png_rejects_zoom_13_and_an_x_past_the_grid() {
        let terrain = fixture();
        let high = render_terrain_tile_png(terrain, TerrainStyle::Imagery, 13, 0, 0, 1);
        assert_eq!(high.unwrap_err().code, CODE_REQUEST_INVALID);
        let wide = render_terrain_tile_png(terrain, TerrainStyle::Imagery, 2, 4, 0, 1);
        assert_eq!(wide.unwrap_err().code, CODE_REQUEST_INVALID);
        let scale = render_terrain_tile_png(terrain, TerrainStyle::Relief, 0, 0, 0, 3);
        assert_eq!(scale.unwrap_err().code, CODE_REQUEST_INVALID);
    }

    #[test]
    fn terrain_tile_png_is_square_and_stable_across_threads() {
        let terrain = fixture();
        let imagery = render_terrain_tile_png(terrain, TerrainStyle::Imagery, 1, 0, 0, 1).unwrap();
        let imagery_again =
            render_terrain_tile_png(terrain, TerrainStyle::Imagery, 1, 0, 0, 1).unwrap();
        assert_eq!(imagery, imagery_again);
        assert_eq!(png_size(&imagery), (256, 256));
        let pool = ThreadPoolBuilder::new()
            .num_threads(1)
            .build()
            .expect("pool");
        let serial = pool.install(|| {
            render_terrain_tile_png(terrain, TerrainStyle::Imagery, 1, 0, 0, 1).unwrap()
        });
        assert_eq!(imagery, serial);

        let relief = render_terrain_tile_png(terrain, TerrainStyle::Relief, 1, 1, 0, 1).unwrap();
        let relief_again =
            render_terrain_tile_png(terrain, TerrainStyle::Relief, 1, 1, 0, 1).unwrap();
        assert_eq!(relief, relief_again);
        assert_eq!(png_size(&relief), (256, 256));
        let relief_serial = pool.install(|| {
            render_terrain_tile_png(terrain, TerrainStyle::Relief, 1, 1, 0, 1).unwrap()
        });
        assert_eq!(relief, relief_serial);

        let scaled = render_terrain_tile_png(terrain, TerrainStyle::Imagery, 1, 0, 1, 2).unwrap();
        assert_eq!(png_size(&scaled), (512, 512));
        let scaled_again =
            render_terrain_tile_png(terrain, TerrainStyle::Imagery, 1, 0, 1, 2).unwrap();
        assert_eq!(scaled, scaled_again);
    }
}
