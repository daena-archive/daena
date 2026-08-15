use daena_physical::hydrology::HydrologyField;
use daena_physical::Grid;

use crate::detail::{sample_sdf_ppm, AtlasDetailModel};
use crate::overlay::composite_overlays;
use crate::projection::{pixel_center_lat_micro, pixel_center_lon_micro, wrap_lon_micro};
use crate::request::{AtlasRenderRequest, TILE_HALO, TILE_SIZE};
use crate::style::{apply_shade, hypsometric, AtlasStyle};
use crate::{AtlasError, AtlasPhase, AtlasProgress};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TileRect {
    pub index: u32,
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

pub fn tile_rects(width: u32, height: u32) -> Vec<TileRect> {
    let mut tiles = Vec::new();
    let mut index = 0_u32;
    let mut y = 0;
    while y < height {
        let mut x = 0;
        while x < width {
            tiles.push(TileRect {
                index,
                x,
                y,
                width: TILE_SIZE.min(width - x),
                height: TILE_SIZE.min(height - y),
            });
            index += 1;
            x += TILE_SIZE;
        }
        y += TILE_SIZE;
    }
    tiles
}

fn nearest_cell(grid: Grid, lon_micro: i32, lat_micro: i32) -> usize {
    crate::detail::nearest_cell(grid, lon_micro, lat_micro)
}

fn shade_ppm(model: &AtlasDetailModel, lon: i32, lat: i32, sea: i32, sdf: &[i32]) -> u32 {
    let delta_lon = (360_000_000i64 / i64::from(model.lattice_width.max(1))) as i32;
    let delta_lat = (180_000_000i64 / i64::from(model.lattice_height.max(1))) as i32;
    let sample = |lon, lat| {
        let sdf_ppm = sample_sdf_ppm(model.grid, sdf, lon, lat);
        i64::from(model.refined_at(lon, lat, sea, sdf_ppm))
    };
    let west = sample(wrap_lon_micro(i64::from(lon) - i64::from(delta_lon)), lat);
    let east = sample(wrap_lon_micro(i64::from(lon) + i64::from(delta_lon)), lat);
    let north = sample(
        lon,
        (i64::from(lat) + i64::from(delta_lat)).clamp(-90_000_000, 90_000_000) as i32,
    );
    let south = sample(
        lon,
        (i64::from(lat) - i64::from(delta_lat)).clamp(-90_000_000, 90_000_000) as i32,
    );
    let nx = west - east;
    let ny = south - north;
    let nz = 12_000_i64;
    let dot = nx
        .saturating_mul(4)
        .saturating_add(ny.saturating_mul(5))
        .saturating_add(nz.saturating_mul(8));
    let bounded = dot.clamp(3_000_000, 12_000_000);
    ((bounded - 3_000_000) as u64 * 1_000_000 / 9_000_000) as u32
}

fn pixel_rgba(
    model: &AtlasDetailModel,
    hydrology: &HydrologyField,
    sdf: &[i32],
    style: &AtlasStyle,
    request: &AtlasRenderRequest,
    lon: i32,
    lat: i32,
) -> [u8; 4] {
    let sea = hydrology.sea_level_mm;
    let sdf_ppm = sample_sdf_ppm(model.grid, sdf, lon, lat);
    let elevation = model.refined_at(lon, lat, sea, sdf_ppm);
    let cell = nearest_cell(model.grid, lon, lat);
    let shade = shade_ppm(model, lon, lat, sea, sdf);
    if request.layer_enabled("ice") && hydrology.ice_cells.get(cell).copied().unwrap_or(false) {
        return apply_shade(style.ice, shade.max(700_000));
    }
    if request.layer_enabled("lakes") && hydrology.lake_cells.get(cell).copied().unwrap_or(false) {
        return apply_shade(style.lake, shade);
    }
    let land = elevation >= sea;
    if land && !request.layer_enabled("relief") {
        return [
            style.background[0],
            style.background[1],
            style.background[2],
            255,
        ];
    }
    if !land && !request.layer_enabled("ocean") {
        return [
            style.background[0],
            style.background[1],
            style.background[2],
            255,
        ];
    }
    apply_shade(hypsometric(style, elevation, sea), shade)
}

pub fn render_rgba(
    request: &AtlasRenderRequest,
    model: &AtlasDetailModel,
    hydrology: &HydrologyField,
    sdf: &[i32],
    style: &AtlasStyle,
    identity: &[u8],
    tile_order: &[u32],
    overlays: &[crate::overlay::AuthoredFeature],
    progress: &mut dyn AtlasProgress,
) -> Result<Vec<u8>, AtlasError> {
    let width = request.width_px;
    let height = request.height_px;
    let pixels = (width as usize)
        .checked_mul(height as usize)
        .and_then(|count| count.checked_mul(4))
        .ok_or_else(|| AtlasError::limit("RGBA buffer overflowed"))?;
    let mut buffer = vec![style.background[0]; pixels];
    for index in (0..pixels).step_by(4) {
        buffer[index] = style.background[0];
        buffer[index + 1] = style.background[1];
        buffer[index + 2] = style.background[2];
        buffer[index + 3] = 255;
    }
    let tiles = tile_rects(width, height);
    if tile_order.len() != tiles.len() {
        return Err(AtlasError::invalid("tile order does not match tile count"));
    }
    let total = tiles.len() as u32;
    for (step, tile_index) in tile_order.iter().enumerate() {
        progress.report(AtlasPhase::Rendering, step as u32, total)?;
        progress.check_cancelled()?;
        let tile = tiles
            .get(*tile_index as usize)
            .ok_or_else(|| AtlasError::invalid("tile index is out of range"))?;
        let _halo = TILE_HALO;
        for local_y in 0..tile.height {
            for local_x in 0..tile.width {
                let x = tile.x + local_x;
                let y = tile.y + local_y;
                let lon = pixel_center_lon_micro(x, width);
                let lat = pixel_center_lat_micro(y, height);
                let rgba = pixel_rgba(model, hydrology, sdf, style, request, lon, lat);
                let offset = ((y as usize) * width as usize + x as usize) * 4;
                buffer[offset..offset + 4].copy_from_slice(&rgba);
            }
        }
    }
    progress.report(AtlasPhase::Rendering, total, total)?;
    composite_overlays(&mut buffer, request, style, hydrology, identity, overlays);
    Ok(buffer)
}

pub fn forward_tile_order(count: u32) -> Vec<u32> {
    (0..count).collect()
}

pub fn reverse_tile_order(count: u32) -> Vec<u32> {
    (0..count).rev().collect()
}

pub fn shuffled_tile_order(count: u32, seed: u64) -> Vec<u32> {
    let mut order = forward_tile_order(count);
    let mut state = seed ^ 0x9e37_79b9_7f4a_7c15;
    for i in (1..order.len()).rev() {
        state = state
            .wrapping_mul(0xbf58_476d_1ce4_e5b9)
            .wrapping_add(0x94d0_49bb_1331_11eb);
        let j = (state as usize) % (i + 1);
        order.swap(i, j);
    }
    order
}
