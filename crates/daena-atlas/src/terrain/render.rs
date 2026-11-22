//! Relief and multi-directional hillshade in Web Mercator pixel space.

use rayon::prelude::*;

use super::{AtlasTerrain, TerrainCover};
use crate::style::{hypsometric, AtlasStyle};

const ALTITUDE_DEG: f64 = 45.0;
const AZIMUTHS_DEG: [f64; 4] = [315.0, 270.0, 0.0, 225.0];
const AZIMUTH_WEIGHTS: [f64; 4] = [0.45, 0.25, 0.2, 0.1];

#[derive(Clone, Copy)]
struct Sample {
    height: f64,
    cover: u8,
    ice: u8,
}

/// Ground metres of one scale-1 pixel. Web Mercator is conformal, so east and
/// north share `2π R cos(lat) / (256 · 2^z)`.
#[must_use]
pub fn ground_metres_per_pixel(z: u32, lat_rad: f64, radius_m: f64) -> f64 {
    let world = mercator_world_px(z);
    std::f64::consts::TAU * radius_m * lat_rad.cos().abs() / world
}

/// Tile column and row containing `(lon_deg, lat_deg)` at zoom `z`.
#[must_use]
pub fn lon_lat_to_tile(lon_deg: f64, lat_deg: f64, z: u32) -> (u32, u32) {
    let n = (1u64 << z.min(30)) as f64;
    let x = ((lon_deg + 180.0) / 360.0 * n).floor().clamp(0.0, n - 1.0) as u32;
    let lat = lat_deg.clamp(-85.051_128_78, 85.051_128_78);
    let mercator_y = lat.to_radians().tan().asinh();
    let y = ((std::f64::consts::PI - mercator_y) / std::f64::consts::TAU * n)
        .floor()
        .clamp(0.0, n - 1.0) as u32;
    (x, y)
}

/// RGBA pixels for a rectangle of scale-1 Web Mercator pixels.
///
/// `width` and `height` are scale-1 pixels. `device_scale` samples that
/// rectangle more densely and multiplies the output size. Fade and hillshade
/// both use the scale-1 pixel, so a scale-2 image is a denser view of the
/// same field. Longitude wraps. Latitude clamps to the Web Mercator range.
#[allow(clippy::too_many_arguments)]
#[must_use]
pub fn render_pixels(
    terrain: &AtlasTerrain,
    style: &AtlasStyle,
    z: u32,
    gx0: u32,
    gy0: u32,
    width: u32,
    height: u32,
    device_scale: u32,
) -> Vec<u8> {
    if width == 0 || height == 0 {
        return Vec::new();
    }
    let scale = device_scale.max(1) as usize;
    let out_w = width as usize * scale;
    let out_h = height as usize * scale;
    let halo = scale;
    let buf_w = out_w + 2 * halo;
    let buf_h = out_h + 2 * halo;
    let world = mercator_world_px(z);
    let exaggeration = vertical_exaggeration(z);
    let suns = sun_vectors();
    let radius = terrain.radius_m();
    let sea_m = terrain.sea_level_m();
    let sea_mm = terrain.sea_level_mm();
    let gx0_f = f64::from(gx0);
    let gy0_f = f64::from(gy0);
    let scale_f = scale as f64;

    let mut samples = vec![
        Sample {
            height: 0.0,
            cover: 0,
            ice: 0,
        };
        buf_w * buf_h
    ];
    samples
        .par_chunks_mut(buf_w)
        .enumerate()
        .for_each(|(row, row_samples)| {
            let gy = gy0_f + (row as f64 - halo as f64 + 0.5) / scale_f;
            let lat = mercator_latitude(gy, world);
            let ground = ground_metres_per_pixel(z, lat, radius);
            for (col, sample) in row_samples.iter_mut().enumerate() {
                let gx = gx0_f + (col as f64 - halo as f64 + 0.5) / scale_f;
                let lon = mercator_longitude(gx, world);
                let (height, cover, ice) =
                    terrain.sample_pixel(direction_from_lon_lat(lon, lat), ground);
                *sample = Sample {
                    height,
                    cover: cover_byte(cover),
                    ice: u8::from(ice),
                };
            }
        });

    let mut rgba = vec![0u8; out_w * out_h * 4];
    rgba.par_chunks_mut(out_w * 4)
        .enumerate()
        .for_each(|(y, row)| {
            let buf_row = y + halo;
            let gy = gy0_f + (y as f64 + 0.5) / scale_f;
            let lat = mercator_latitude(gy, world);
            let ground = ground_metres_per_pixel(z, lat, radius).max(1e-6);
            for x in 0..out_w {
                let col = x + halo;
                let centre = samples[buf_row * buf_w + col];
                let east = samples[buf_row * buf_w + col + halo].height;
                let west = samples[buf_row * buf_w + col - halo].height;
                let north = samples[(buf_row - halo) * buf_w + col].height;
                let south = samples[(buf_row + halo) * buf_w + col].height;
                let dzdx = (east - west) / (2.0 * ground);
                let dzdy = (north - south) / (2.0 * ground);
                let shade = combined_shade(dzdx, dzdy, exaggeration, &suns);
                let cover = cover_from_byte(centre.cover);
                let rgb = paint(style, cover, centre.ice != 0, centre.height, sea_m, sea_mm);
                let factor = match cover {
                    TerrainCover::Lake => 1.0,
                    TerrainCover::Ocean => 0.85 + 0.15 * shade,
                    TerrainCover::Land => 0.30 + 0.70 * shade,
                };
                let pixel = x * 4;
                row[pixel] = scale_channel(rgb[0], factor);
                row[pixel + 1] = scale_channel(rgb[1], factor);
                row[pixel + 2] = scale_channel(rgb[2], factor);
                row[pixel + 3] = 255;
            }
        });
    rgba
}

fn paint(
    style: &AtlasStyle,
    cover: TerrainCover,
    ice: bool,
    height_m: f64,
    sea_m: f64,
    sea_mm: i32,
) -> [u8; 3] {
    match cover {
        TerrainCover::Ocean => hypsometric(style, metres_to_mm(height_m), sea_mm),
        TerrainCover::Lake => style.lake,
        TerrainCover::Land if ice => style.ice,
        TerrainCover::Land => {
            // A dry basin is land. Keep it on the land ramp.
            hypsometric(style, metres_to_mm(height_m.max(sea_m)), sea_mm)
        }
    }
}

fn metres_to_mm(metres: f64) -> i32 {
    let mm = metres * 1000.0;
    if !mm.is_finite() {
        0
    } else {
        mm.round().clamp(f64::from(i32::MIN), f64::from(i32::MAX)) as i32
    }
}

fn scale_channel(channel: u8, factor: f64) -> u8 {
    (f64::from(channel) * factor).round().clamp(0.0, 255.0) as u8
}

fn cover_byte(cover: TerrainCover) -> u8 {
    match cover {
        TerrainCover::Land => 0,
        TerrainCover::Ocean => 1,
        TerrainCover::Lake => 2,
    }
}

fn cover_from_byte(byte: u8) -> TerrainCover {
    match byte {
        1 => TerrainCover::Ocean,
        2 => TerrainCover::Lake,
        _ => TerrainCover::Land,
    }
}

fn vertical_exaggeration(z: u32) -> f64 {
    2f64.powf((11.0 - f64::from(z)) / 2.5).max(1.0)
}

fn sun_vectors() -> [[f64; 3]; 4] {
    let altitude = ALTITUDE_DEG.to_radians();
    let cos_alt = altitude.cos();
    let sin_alt = altitude.sin();
    AZIMUTHS_DEG.map(|azimuth| {
        let azimuth = azimuth.to_radians();
        [azimuth.sin() * cos_alt, azimuth.cos() * cos_alt, sin_alt]
    })
}

fn combined_shade(dzdx: f64, dzdy: f64, exaggeration: f64, suns: &[[f64; 3]; 4]) -> f64 {
    let nx = -dzdx * exaggeration;
    let ny = -dzdy * exaggeration;
    let nz = 1.0;
    let len = (nx * nx + ny * ny + nz * nz).sqrt();
    let mut shade = 0.0;
    for (sun, weight) in suns.iter().zip(AZIMUTH_WEIGHTS) {
        let lambert = ((nx * sun[0] + ny * sun[1] + nz * sun[2]) / len).max(0.0);
        shade += weight * lambert;
    }
    shade
}

fn mercator_world_px(z: u32) -> f64 {
    256.0 * (1u64 << z.min(30)) as f64
}

fn mercator_longitude(gx: f64, world: f64) -> f64 {
    let wrapped = gx.rem_euclid(world);
    std::f64::consts::PI * (2.0 * wrapped / world - 1.0)
}

fn mercator_latitude(gy: f64, world: f64) -> f64 {
    let clamped = gy.clamp(0.0, world);
    let y = std::f64::consts::PI * (1.0 - 2.0 * clamped / world);
    y.sinh().atan()
}

fn direction_from_lon_lat(lon: f64, lat: f64) -> [f64; 3] {
    let cos_lat = lat.cos();
    [cos_lat * lon.cos(), cos_lat * lon.sin(), lat.sin()]
}
