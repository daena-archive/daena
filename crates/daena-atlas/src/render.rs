use daena_physical::hydrology::HydrologyField;
use daena_physical::Grid;

use crate::detail::{sample_sdf_ppm, AtlasDetailModel};
use crate::overlay::composite_overlays;
use crate::projection::wrap_lon_micro;
use crate::request::{AtlasRenderRequest, TILE_HALO, TILE_SIZE};
use crate::style::{
    apply_shade, biome_fill, hypsometric, mix_rgb, precipitation_fill, temperature_fill,
    AtlasStyle, BIOME_STYLE_ID, PRECIPITATION_STYLE_ID, TEMPERATURE_STYLE_ID,
};
use crate::{AtlasError, AtlasPhase, AtlasProgress};

/// Isolated sinks smaller than this stay land so one-cell puddles do not
/// speckle continents. Matches `MIN_VISIBLE_INLAND_WATER_CELLS` on the
/// Physical Map raster.
pub const MIN_VISIBLE_INLAND_WATER_CELLS: usize = 8;

#[derive(Debug, Clone)]
pub struct VisibleWater {
    pub ocean: Vec<bool>,
    pub inland: Vec<bool>,
}

fn cardinal_neighbors(width: u32, height: u32, index: usize) -> [Option<usize>; 4] {
    let w = width as usize;
    let row = index / w;
    let col = index % w;
    let left = if col == 0 { index + w - 1 } else { index - 1 };
    let right = if col + 1 == w {
        index + 1 - w
    } else {
        index + 1
    };
    let north = if row > 0 { Some(index - w) } else { None };
    let south = if row + 1 < height as usize {
        Some(index + w)
    } else {
        None
    };
    [Some(left), Some(right), north, south]
}

#[derive(Debug, Clone, Copy)]
pub struct PaintFields<'a> {
    pub climate_class: &'a [i32],
    pub temperature_centi_c: &'a [i32],
    pub precipitation_mm: &'a [i32],
}

fn flood_component(
    width: u32,
    height: u32,
    start: usize,
    wet: &[bool],
    seen: &mut [bool],
) -> Vec<usize> {
    let mut cells = Vec::new();
    let mut stack = vec![start];
    seen[start] = true;
    while let Some(index) = stack.pop() {
        cells.push(index);
        for next in cardinal_neighbors(width, height, index)
            .into_iter()
            .flatten()
        {
            if seen[next] || !wet[next] {
                continue;
            }
            seen[next] = true;
            stack.push(next);
        }
    }
    cells
}

#[must_use]
pub fn classify_visible_water(
    grid: Grid,
    elevations_mm: &[i32],
    sea_level_mm: i32,
    lake_cells: &[bool],
) -> VisibleWater {
    let count = grid.sample_count();
    let mut ocean = vec![false; count];
    let mut inland = vec![false; count];
    if count == 0
        || elevations_mm.len() != count
        || lake_cells.len() != count
        || grid.width == 0
        || grid.height == 0
    {
        return VisibleWater { ocean, inland };
    }
    let below_sea: Vec<bool> = elevations_mm
        .iter()
        .map(|elevation| *elevation < sea_level_mm)
        .collect();
    let mut seen = vec![false; count];
    let mut largest: Vec<usize> = Vec::new();
    for index in 0..count {
        if !below_sea[index] || seen[index] {
            continue;
        }
        let cells = flood_component(grid.width, grid.height, index, &below_sea, &mut seen);
        if cells.len() > largest.len() {
            largest = cells;
        }
    }
    for cell in &largest {
        ocean[*cell] = true;
    }
    let inland_wet: Vec<bool> = (0..count)
        .map(|index| {
            if ocean[index] {
                false
            } else {
                lake_cells[index] || below_sea[index]
            }
        })
        .collect();
    seen.fill(false);
    for index in 0..count {
        if !inland_wet[index] || seen[index] {
            continue;
        }
        let cells = flood_component(grid.width, grid.height, index, &inland_wet, &mut seen);
        if cells.len() < MIN_VISIBLE_INLAND_WATER_CELLS {
            continue;
        }
        for cell in cells {
            inland[cell] = true;
        }
    }
    VisibleWater { ocean, inland }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TileRect {
    pub index: u32,
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

#[must_use]
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

fn shade_ppm(
    model: &AtlasDetailModel,
    lon: i32,
    lat: i32,
    sea: i32,
    sdf: &[i32],
    center: i32,
    approximate: bool,
) -> u32 {
    let cell_lon = (360_000_000i64 / i64::from(model.lattice_width.max(1))) as i32;
    let cell_lat = (180_000_000i64 / i64::from(model.lattice_height.max(1))) as i32;
    let delta_lon = (cell_lon / 4).max(20_000);
    let delta_lat = (cell_lat / 4).max(20_000);
    let sample = |lon, lat| {
        let sdf_ppm = sample_sdf_ppm(model.grid, sdf, lon, lat);
        i64::from(model.refined_at(lon, lat, sea, sdf_ppm))
    };
    let (nx, ny) = if approximate {
        let east = sample(wrap_lon_micro(i64::from(lon) + i64::from(delta_lon)), lat);
        let north = sample(
            lon,
            (i64::from(lat) + i64::from(delta_lat)).clamp(-90_000_000, 90_000_000) as i32,
        );
        // Forward differences reuse the center sample, removing two expensive
        // terrain samples per interactive pixel while preserving slope scale.
        let center = i64::from(center);
        (
            center.saturating_sub(east).saturating_mul(2),
            center.saturating_sub(north).saturating_mul(2),
        )
    } else {
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
        (west - east, south - north)
    };
    let nz = 220_000_i64;
    let mag = nx
        .unsigned_abs()
        .saturating_add(ny.unsigned_abs())
        .saturating_add(nz.unsigned_abs())
        .max(1);
    let lit = nx
        .saturating_mul(2)
        .saturating_add(ny.saturating_mul(3))
        .saturating_add(nz.saturating_mul(8));
    ((lit + mag as i64) * 500_000 / mag as i64).clamp(420_000, 1_000_000) as u32
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RasterTheme {
    Relief,
    Biome,
    Temperature,
    Precipitation,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct RasterOptions {
    ice: bool,
    lakes: bool,
    relief: bool,
    ocean: bool,
    coastlines: bool,
    theme: RasterTheme,
    approximate_shading: bool,
}

impl RasterOptions {
    pub(crate) fn new(style: &AtlasStyle, request: &AtlasRenderRequest) -> Self {
        let theme = match style.id.as_str() {
            BIOME_STYLE_ID => RasterTheme::Biome,
            TEMPERATURE_STYLE_ID => RasterTheme::Temperature,
            PRECIPITATION_STYLE_ID => RasterTheme::Precipitation,
            _ => RasterTheme::Relief,
        };
        Self {
            ice: request.layer_enabled("ice"),
            lakes: request.layer_enabled("lakes"),
            relief: request.layer_enabled("relief"),
            ocean: request.layer_enabled("ocean"),
            coastlines: request.layer_enabled("coastlines"),
            theme,
            approximate_shading: false,
        }
    }

    pub(crate) fn for_studio(style: &AtlasStyle, request: &AtlasRenderRequest) -> Self {
        Self {
            approximate_shading: true,
            ..Self::new(style, request)
        }
    }
}

pub(crate) fn studio_shade_ppm(
    model: &AtlasDetailModel,
    hydrology: &HydrologyField,
    sdf: &[i32],
    options: RasterOptions,
    lon: i32,
    lat: i32,
) -> u32 {
    let sea = hydrology.sea_level_mm;
    let sdf_ppm = sample_sdf_ppm(model.grid, sdf, lon, lat);
    let elevation = model.refined_at(lon, lat, sea, sdf_ppm);
    let shade = shade_ppm(
        model,
        lon,
        lat,
        sea,
        sdf,
        elevation,
        options.approximate_shading,
    );
    if options.theme == RasterTheme::Relief {
        shade
    } else {
        shade.max(780_000)
    }
}

#[allow(clippy::too_many_arguments)]
#[cfg(test)]
pub(crate) fn pixel_rgba(
    model: &AtlasDetailModel,
    hydrology: &HydrologyField,
    sdf: &[i32],
    style: &AtlasStyle,
    request: &AtlasRenderRequest,
    water: &VisibleWater,
    paint: PaintFields<'_>,
    lon: i32,
    lat: i32,
) -> [u8; 4] {
    pixel_rgba_with_options(
        model,
        hydrology,
        sdf,
        style,
        RasterOptions::new(style, request),
        water,
        paint,
        lon,
        lat,
    )
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn pixel_rgba_with_options(
    model: &AtlasDetailModel,
    hydrology: &HydrologyField,
    sdf: &[i32],
    style: &AtlasStyle,
    options: RasterOptions,
    water: &VisibleWater,
    paint: PaintFields<'_>,
    lon: i32,
    lat: i32,
) -> [u8; 4] {
    let sea = hydrology.sea_level_mm;
    let sdf_ppm = sample_sdf_ppm(model.grid, sdf, lon, lat);
    let elevation = model.refined_at(lon, lat, sea, sdf_ppm);
    let cell = nearest_cell(model.grid, lon, lat);
    let shade = shade_ppm(
        model,
        lon,
        lat,
        sea,
        sdf,
        elevation,
        options.approximate_shading,
    );
    let shade = if options.theme == RasterTheme::Relief {
        shade
    } else {
        shade.max(780_000)
    };
    paint_pixel(
        hydrology, style, options, water, paint, sea, elevation, cell, shade,
    )
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn pixel_rgba_with_shade(
    model: &AtlasDetailModel,
    hydrology: &HydrologyField,
    sdf: &[i32],
    style: &AtlasStyle,
    options: RasterOptions,
    water: &VisibleWater,
    paint: PaintFields<'_>,
    lon: i32,
    lat: i32,
    shade: u32,
) -> [u8; 4] {
    let sea = hydrology.sea_level_mm;
    let sdf_ppm = sample_sdf_ppm(model.grid, sdf, lon, lat);
    let elevation = model.refined_at(lon, lat, sea, sdf_ppm);
    let cell = nearest_cell(model.grid, lon, lat);
    paint_pixel(
        hydrology, style, options, water, paint, sea, elevation, cell, shade,
    )
}

#[allow(clippy::too_many_arguments)]
fn paint_pixel(
    hydrology: &HydrologyField,
    style: &AtlasStyle,
    options: RasterOptions,
    water: &VisibleWater,
    paint: PaintFields<'_>,
    sea: i32,
    elevation: i32,
    cell: usize,
    shade: u32,
) -> [u8; 4] {
    if options.ice && hydrology.ice_cells.get(cell).copied().unwrap_or(false) {
        return apply_shade(style.ice, shade.max(700_000));
    }
    let inland = water.inland.get(cell).copied().unwrap_or(false);
    if options.lakes && inland {
        return apply_shade(style.lake, shade);
    }
    let land = elevation >= sea;
    if land && !options.relief {
        return [
            style.background[0],
            style.background[1],
            style.background[2],
            255,
        ];
    }
    if !land && !options.ocean {
        return [
            style.background[0],
            style.background[1],
            style.background[2],
            255,
        ];
    }
    let painted = if land { elevation.max(sea) } else { elevation };
    let mut rgb = if land && options.theme == RasterTheme::Biome {
        biome_fill(
            style,
            paint
                .climate_class
                .get(cell)
                .copied()
                .unwrap_or(crate::control::CLIMATE_CLASS_GRASSLAND),
        )
    } else if land && options.theme == RasterTheme::Temperature {
        temperature_fill(
            style,
            paint.temperature_centi_c.get(cell).copied().unwrap_or(0),
        )
    } else if land && options.theme == RasterTheme::Precipitation {
        precipitation_fill(
            style,
            paint.precipitation_mm.get(cell).copied().unwrap_or(0),
        )
    } else {
        hypsometric(style, painted, sea)
    };
    if options.coastlines {
        let band = elevation.saturating_sub(sea).unsigned_abs();
        if band < 28_000 {
            let t = (u64::from(28_000 - band) * 620_000 / 28_000) as u32;
            rgb = mix_rgb(rgb, style.coast, t);
        }
    }
    apply_shade(rgb, shade)
}

#[allow(clippy::too_many_arguments)]
pub fn render_rgba(
    request: &AtlasRenderRequest,
    model: &AtlasDetailModel,
    hydrology: &HydrologyField,
    sdf: &[i32],
    style: &AtlasStyle,
    identity: &[u8],
    tile_order: &[u32],
    overlays: &[crate::overlay::AuthoredFeature],
    tributaries: &[crate::drainage::DerivedTributary],
    tectonics: &daena_physical::tectonics::TectonicWorld,
    water: &VisibleWater,
    paint: PaintFields<'_>,
    progress: &mut dyn AtlasProgress,
) -> Result<Vec<u8>, AtlasError> {
    let view = request.view()?;
    let width = view.width;
    let height = view.height;
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
    let options = RasterOptions::new(style, request);
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
                let (lon, lat) = view.pixel_center(x, y);
                let rgba = pixel_rgba_with_options(
                    model, hydrology, sdf, style, options, water, paint, lon, lat,
                );
                let offset = ((y as usize) * width as usize + x as usize) * 4;
                buffer[offset..offset + 4].copy_from_slice(&rgba);
            }
        }
    }
    progress.report(AtlasPhase::Rendering, total, total)?;
    composite_overlays(
        &mut buffer,
        request,
        style,
        hydrology,
        tectonics,
        identity,
        overlays,
        tributaries,
    );
    Ok(buffer)
}

#[must_use]
pub fn forward_tile_order(count: u32) -> Vec<u32> {
    (0..count).collect()
}

#[must_use]
pub fn reverse_tile_order(count: u32) -> Vec<u32> {
    (0..count).rev().collect()
}

#[must_use]
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

#[cfg(test)]
mod tests {
    use super::*;
    use daena_physical::Grid;

    fn grid(width: u32, height: u32) -> Grid {
        Grid {
            width,
            height,
            radius_metres: 6_371_000,
        }
    }

    #[test]
    fn isolated_lake_cells_stay_land() {
        let grid = grid(8, 8);
        let mut elevations = vec![1_000; 64];
        let mut lakes = vec![false; 64];
        elevations[0] = -1_000;
        elevations[1] = -1_000;
        elevations[8] = -1_000;
        for elevation in elevations.iter_mut().take(8) {
            *elevation = -4_000;
        }
        lakes[20] = true;
        lakes[21] = true;
        let water = classify_visible_water(grid, &elevations, 0, &lakes);
        assert!(water.ocean[1]);
        assert!(!water.ocean[20]);
        assert!(!water.inland[20]);
        assert!(!water.inland[21]);
    }

    #[test]
    fn large_inland_lake_remains_visible() {
        let grid = grid(8, 8);
        let mut elevations = vec![1_000; 64];
        let mut lakes = vec![false; 64];
        for elevation in elevations.iter_mut().take(8) {
            *elevation = -4_000;
        }
        for index in [20, 21, 22, 23, 28, 29, 30, 31] {
            lakes[index] = true;
        }
        let water = classify_visible_water(grid, &elevations, 0, &lakes);
        assert!(water.inland[20]);
        assert!(water.inland[31]);
        assert!(!water.ocean[20]);
    }
}
