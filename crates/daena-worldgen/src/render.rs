//! Equirectangular and orthographic previews, plus PNG output.
//!
//! Relief shades a hypsometric tint. The shade is the 3D tangent-plane gradient
//! of elevation, lit from the north-west at 35° in each cell's east/north frame,
//! with vertical exaggeration ×20. Categorical layers use nearest samples.
//! Continuous layers use bilinear samples. Face borders clamp to that face.

use std::fs::File;
use std::io::BufWriter;
use std::path::Path;

use png::{BitDepth, ColorType, Compression, Encoder, FilterType};
use rayon::prelude::*;

use crate::grid::{cross, dot3, CubeSphere, NONE};
use crate::{direction_from_lat_lon, World, WorldError};

/// Preview layer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Layer {
    /// Hypsometric tint with hillshade.
    Relief,
    /// Plate colours, with boundaries recoloured by kind.
    Plates,
    /// Continental and oceanic crust.
    Crust,
    /// Diverging uplift ramp.
    Uplift,
    /// Log drainage area on land. Ocean is dark.
    Drainage,
    /// Pre-erosion `base_m` with the relief ramp and no shade.
    Base,
}

impl Layer {
    /// File stem: `relief`, `plates`, `crust`, `uplift`, `drainage`, `base`.
    pub fn name(self) -> &'static str {
        match self {
            Self::Relief => "relief",
            Self::Plates => "plates",
            Self::Crust => "crust",
            Self::Uplift => "uplift",
            Self::Drainage => "drainage",
            Self::Base => "base",
        }
    }

    /// Parse a layer name.
    pub fn parse(text: &str) -> Option<Self> {
        match text {
            "relief" => Some(Self::Relief),
            "plates" => Some(Self::Plates),
            "crust" => Some(Self::Crust),
            "uplift" => Some(Self::Uplift),
            "drainage" => Some(Self::Drainage),
            "base" => Some(Self::Base),
            _ => None,
        }
    }
}

/// Per-cell hillshade in `[0, 1]`. Flat cells keep the sun's vertical component.
pub fn hillshade(world: &World) -> Vec<f32> {
    let n = world.grid.cell_count();
    if world.elevation_m.len() != n {
        return vec![1.0; n];
    }
    let sun = sun_direction();
    let mut shade = vec![1.0f32; n];
    let grid = &world.grid;
    let elev = &world.elevation_m;
    shade.par_iter_mut().enumerate().for_each(|(cell, slot)| {
        *slot = shade_cell(grid, elev, cell as u32, sun);
    });
    shade
}

/// Equirectangular RGB image, row-major, `width * height * 3` bytes.
pub fn render_equirect(world: &World, layer: Layer, width: u32, height: u32) -> Vec<u8> {
    let owned = relief_shade(world, layer);
    render_equirect_with_shade(world, layer, width, height, owned.as_deref().unwrap_or(&[]))
}

/// [`render_equirect`] using a precomputed [`hillshade`] for [`Layer::Relief`].
pub fn render_equirect_with_shade(
    world: &World,
    layer: Layer,
    width: u32,
    height: u32,
    shade: &[f32],
) -> Vec<u8> {
    paint(world, layer, width, height, Projection::Equirect, shade)
}

/// Orthographic RGB disc. Pixels outside the disc are a dark background.
pub fn render_orthographic(
    world: &World,
    layer: Layer,
    size: u32,
    centre_lat: f64,
    centre_lon: f64,
) -> Vec<u8> {
    let owned = relief_shade(world, layer);
    render_orthographic_with_shade(
        world,
        layer,
        size,
        centre_lat,
        centre_lon,
        owned.as_deref().unwrap_or(&[]),
    )
}

/// [`render_orthographic`] using a precomputed [`hillshade`] for relief.
pub fn render_orthographic_with_shade(
    world: &World,
    layer: Layer,
    size: u32,
    centre_lat: f64,
    centre_lon: f64,
    shade: &[f32],
) -> Vec<u8> {
    let (centre, east, north) = view_basis(centre_lat, centre_lon);
    paint(
        world,
        layer,
        size,
        size,
        Projection::Ortho {
            centre,
            east,
            north,
        },
        shade,
    )
}

/// Write an 8-bit RGB PNG. Parent directories are created. Compression is fast and unfiltered.
pub fn write_png(path: &Path, width: u32, height: u32, rgb: &[u8]) -> Result<(), WorldError> {
    let expect = (width as usize)
        .saturating_mul(height as usize)
        .saturating_mul(3);
    if rgb.len() != expect {
        return Err(WorldError::Internal(format!(
            "png buffer has {} bytes, expected {expect}",
            rgb.len()
        )));
    }
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent).map_err(|err| WorldError::Internal(err.to_string()))?;
        }
    }
    let file = File::create(path).map_err(|err| WorldError::Internal(err.to_string()))?;
    let mut encoder = Encoder::new(BufWriter::new(file), width, height);
    encoder.set_color(ColorType::Rgb);
    encoder.set_depth(BitDepth::Eight);
    encoder.set_compression(Compression::Fast);
    encoder.set_filter(FilterType::NoFilter);
    let mut writer = encoder
        .write_header()
        .map_err(|err| WorldError::Internal(err.to_string()))?;
    writer
        .write_image_data(rgb)
        .map_err(|err| WorldError::Internal(err.to_string()))?;
    Ok(())
}

/// Bilinear equirectangular sample of one field.
pub(crate) fn sample_equirect(
    grid: &CubeSphere,
    field: &[f32],
    width: u32,
    height: u32,
) -> Vec<f32> {
    let pixels = (width as usize).saturating_mul(height as usize);
    if width == 0 || height == 0 || field.len() != grid.cell_count() {
        return vec![0.0; pixels];
    }
    let mut out = vec![0.0f32; pixels];
    out.par_chunks_mut(width as usize)
        .enumerate()
        .for_each(|(y, row)| {
            for (x, dest) in row.iter_mut().enumerate() {
                let dir = equirect_dir(x as u32, y as u32, width, height);
                *dest = grid.sample_bilinear(field, dir);
            }
        });
    out
}

#[derive(Clone, Copy)]
enum Projection {
    Equirect,
    Ortho {
        centre: [f64; 3],
        east: [f64; 3],
        north: [f64; 3],
    },
}

struct Caches {
    plates: Option<Vec<[u8; 3]>>,
    drain: Option<Vec<f32>>,
    land: Option<Vec<u8>>,
}

fn relief_shade(world: &World, layer: Layer) -> Option<Vec<f32>> {
    if layer == Layer::Relief {
        Some(hillshade(world))
    } else {
        None
    }
}

fn paint(
    world: &World,
    layer: Layer,
    width: u32,
    height: u32,
    projection: Projection,
    shade: &[f32],
) -> Vec<u8> {
    let pixels = (width as usize)
        .saturating_mul(height as usize)
        .saturating_mul(3);
    if width == 0 || height == 0 {
        return Vec::new();
    }
    let caches = build_caches(world, layer);
    let mut rgb = vec![0u8; pixels];
    let row_bytes = width as usize * 3;
    let ctx = PixelCtx {
        world,
        layer,
        shade,
        caches: &caches,
        projection,
        width,
        height,
    };
    rgb.par_chunks_mut(row_bytes)
        .enumerate()
        .for_each(|(y, row)| {
            for x in 0..width {
                let color = pixel_color(&ctx, x, y as u32);
                let offset = x as usize * 3;
                row[offset] = color[0];
                row[offset + 1] = color[1];
                row[offset + 2] = color[2];
            }
        });
    rgb
}

struct PixelCtx<'a> {
    world: &'a World,
    layer: Layer,
    shade: &'a [f32],
    caches: &'a Caches,
    projection: Projection,
    width: u32,
    height: u32,
}

fn build_caches(world: &World, layer: Layer) -> Caches {
    match layer {
        Layer::Plates => Caches {
            plates: Some(plate_field(world)),
            drain: None,
            land: None,
        },
        Layer::Drainage => {
            let (drain, land) = drainage_fields(world);
            Caches {
                plates: None,
                drain: Some(drain),
                land: Some(land),
            }
        }
        _ => Caches {
            plates: None,
            drain: None,
            land: None,
        },
    }
}

fn pixel_color(ctx: &PixelCtx<'_>, x: u32, y: u32) -> [u8; 3] {
    let Some(dir) = project(ctx.projection, x, y, ctx.width, ctx.height) else {
        return BACKGROUND;
    };
    let world = ctx.world;
    let caches = ctx.caches;
    let shade = ctx.shade;
    match ctx.layer {
        Layer::Relief => {
            let cell = world.grid.cell_at(dir) as usize;
            let h = world.grid.sample_bilinear(&world.elevation_m, dir);
            let light = sample_shade(&world.grid, shade, dir);
            let cell_h = world.elevation_m.get(cell).copied().unwrap_or(h);
            let is_ocean = world.ocean.get(cell).copied().unwrap_or(1) == 1;
            if !is_ocean && cell_h <= 0.0 {
                tone(dry_depression_color(), light, false)
            } else {
                tone(relief_color(h), light, h <= 0.0)
            }
        }
        Layer::Base => relief_color(world.grid.sample_bilinear(&world.base_m, dir)),
        Layer::Uplift => uplift_color(world.grid.sample_bilinear(&world.uplift_m_yr, dir)),
        Layer::Crust => {
            let cell = world.grid.cell_at(dir) as usize;
            crust_color(world.crust.get(cell).copied().unwrap_or(0))
        }
        Layer::Plates => {
            let cell = world.grid.cell_at(dir) as usize;
            caches
                .plates
                .as_ref()
                .and_then(|field| field.get(cell).copied())
                .unwrap_or(GREY)
        }
        Layer::Drainage => drainage_pixel(world, caches, dir),
    }
}

fn project(projection: Projection, x: u32, y: u32, width: u32, height: u32) -> Option<[f64; 3]> {
    match projection {
        Projection::Equirect => Some(equirect_dir(x, y, width, height)),
        Projection::Ortho {
            centre,
            east,
            north,
        } => ortho_dir(x, y, width, centre, east, north),
    }
}

fn equirect_dir(x: u32, y: u32, width: u32, height: u32) -> [f64; 3] {
    let lon =
        (f64::from(x) + 0.5) / f64::from(width) * std::f64::consts::TAU - std::f64::consts::PI;
    let lat = std::f64::consts::FRAC_PI_2
        - (f64::from(y) + 0.5) / f64::from(height) * std::f64::consts::PI;
    direction_from_lat_lon(lat.to_degrees(), lon.to_degrees())
}

fn ortho_dir(
    x: u32,
    y: u32,
    size: u32,
    centre: [f64; 3],
    east: [f64; 3],
    north: [f64; 3],
) -> Option<[f64; 3]> {
    if size == 0 {
        return None;
    }
    let nx = (f64::from(x) + 0.5) / f64::from(size) * 2.0 - 1.0;
    let ny = 1.0 - (f64::from(y) + 0.5) / f64::from(size) * 2.0;
    let r2 = nx * nx + ny * ny;
    if r2 > 1.0 {
        return None;
    }
    let nz = (1.0 - r2).sqrt();
    Some([
        nz * centre[0] + nx * east[0] + ny * north[0],
        nz * centre[1] + nx * east[1] + ny * north[1],
        nz * centre[2] + nx * east[2] + ny * north[2],
    ])
}

fn view_basis(lat_deg: f64, lon_deg: f64) -> ([f64; 3], [f64; 3], [f64; 3]) {
    let centre = direction_from_lat_lon(lat_deg, lon_deg);
    let lon = lon_deg.to_radians();
    let east = [-lon.sin(), lon.cos(), 0.0];
    // `centre × east` is the direction of increasing latitude.
    let north = cross(centre, east);
    (centre, east, north)
}

fn sample_shade(grid: &CubeSphere, shade: &[f32], dir: [f64; 3]) -> f32 {
    if shade.len() == grid.cell_count() {
        grid.sample_bilinear(shade, dir)
    } else {
        1.0
    }
}

fn drainage_pixel(world: &World, caches: &Caches, dir: [f64; 3]) -> [u8; 3] {
    let (Some(drain), Some(land)) = (&caches.drain, &caches.land) else {
        return OCEAN_DARK;
    };
    let (face, fx, fy) = world.grid.face_coords(dir);
    match world.grid.sample_face_masked(drain, land, face, fx, fy) {
        Some(log_area) => drainage_color(log_area),
        None => OCEAN_DARK,
    }
}

fn plate_field(world: &World) -> Vec<[u8; 3]> {
    let mut colors = vec![GREY; world.plate.len()];
    colors.par_iter_mut().enumerate().for_each(|(cell, slot)| {
        *slot = plate_cell_color(world, cell as u32);
    });
    colors
}

fn plate_cell_color(world: &World, cell: u32) -> [u8; 3] {
    let Some(&plate_id) = world.plate.get(cell as usize) else {
        return GREY;
    };
    match stored_boundary_kind(world, cell) {
        1 => [210, 48, 42],
        2 => [46, 96, 210],
        3 => [42, 170, 78],
        _ => plate_tint(plate_id),
    }
}

fn stored_boundary_kind(world: &World, cell: u32) -> u8 {
    let kinds = &world.tectonics.boundary_kind;
    let kind_n = world.tectonics.n;
    let world_n = world.grid.n;
    let index = if world_n == kind_n {
        cell as usize
    } else if kind_n > 0 && world_n > kind_n && world_n.is_multiple_of(kind_n) {
        let scale = world_n / kind_n;
        let (face, i, j) = world.grid.decode(cell);
        (face * kind_n * kind_n + (j / scale) * kind_n + (i / scale)) as usize
    } else {
        return 0;
    };
    kinds.get(index).copied().unwrap_or(0)
}

fn drainage_fields(world: &World) -> (Vec<f32>, Vec<u8>) {
    let n = world.drainage_area_m2.len();
    let mut drain = vec![0.0f32; n];
    let mut land = vec![0u8; n];
    for i in 0..n {
        let ocean = world.ocean.get(i).copied().unwrap_or(1);
        if ocean == 1 {
            continue;
        }
        land[i] = 1;
        drain[i] = world.drainage_area_m2[i].max(1.0).log10();
    }
    (drain, land)
}

fn shade_cell(grid: &CubeSphere, elev: &[f32], cell: u32, sun: [f64; 3]) -> f32 {
    let up = grid.position(cell);
    let (east, north) = local_east_north(up);
    let nbs = grid.neighbors(cell);
    let mut a00 = 0.0f64;
    let mut a01 = 0.0f64;
    let mut a11 = 0.0f64;
    let mut b0 = 0.0f64;
    let mut b1 = 0.0f64;
    let hc = f64::from(elev[cell as usize]);
    for (k, &nb) in nbs.iter().enumerate() {
        if nb == NONE {
            continue;
        }
        let dist = f64::from(grid.neighbor_distance(cell, k));
        if dist <= 0.0 {
            continue;
        }
        let pk = grid.position(nb);
        let proj = dot3(pk, up);
        let along = [
            pk[0] - up[0] * proj,
            pk[1] - up[1] * proj,
            pk[2] - up[2] * proj,
        ];
        let dx = dot3(along, east);
        let dy = dot3(along, north);
        let horiz = (dx * dx + dy * dy).sqrt();
        if horiz < 1e-12 {
            continue;
        }
        let xm = dx / horiz * dist;
        let ym = dy / horiz * dist;
        let dh = f64::from(elev[nb as usize]) - hc;
        a00 += xm * xm;
        a01 += xm * ym;
        a11 += ym * ym;
        b0 += xm * dh;
        b1 += ym * dh;
    }
    let det = a00 * a11 - a01 * a01;
    if det.abs() < 1e-6 {
        return sun[2].clamp(0.0, 1.0) as f32;
    }
    let gx = (b0 * a11 - a01 * b1) / det;
    let gy = (a00 * b1 - a01 * b0) / det;
    let nx = -20.0 * gx;
    let ny = -20.0 * gy;
    let nz = 1.0;
    let inv = 1.0 / (nx * nx + ny * ny + nz * nz).sqrt();
    (nx * sun[0] + ny * sun[1] + nz * sun[2])
        .mul_add(inv, 0.0)
        .clamp(0.0, 1.0) as f32
}

fn local_east_north(up: [f64; 3]) -> ([f64; 3], [f64; 3]) {
    let mut east = cross([0.0, 0.0, 1.0], up);
    let mut len = norm(east);
    if len < 1e-8 {
        east = cross([0.0, 1.0, 0.0], up);
        len = norm(east);
    }
    if len < 1e-12 {
        return ([1.0, 0.0, 0.0], [0.0, 1.0, 0.0]);
    }
    east = [east[0] / len, east[1] / len, east[2] / len];
    (east, cross(up, east))
}

fn sun_direction() -> [f64; 3] {
    let altitude = 35.0_f64.to_radians();
    let azimuth = 315.0_f64.to_radians();
    [
        altitude.cos() * azimuth.sin(),
        altitude.cos() * azimuth.cos(),
        altitude.sin(),
    ]
}

fn dry_depression_color() -> [u8; 3] {
    let rgb = LAND_STOPS[0].1;
    [
        byte(f64::from(rgb[0]) * 0.8),
        byte(f64::from(rgb[1]) * 0.8),
        byte(f64::from(rgb[2]) * 0.8),
    ]
}

fn relief_color(h: f32) -> [u8; 3] {
    let h = f64::from(h);
    if h <= 0.0 {
        ramp(&OCEAN_STOPS, h)
    } else {
        ramp(&LAND_STOPS, h)
    }
}

fn uplift_color(uplift: f32) -> [u8; 3] {
    let t = (f64::from(uplift) / 1.5e-3).clamp(-1.0, 1.0);
    if t >= 0.0 {
        lerp(GREY, [176, 42, 36], t)
    } else {
        lerp(GREY, [36, 78, 176], -t)
    }
}

fn drainage_color(log_area: f32) -> [u8; 3] {
    let t = ((f64::from(log_area) - 7.0) / 6.0).clamp(0.0, 1.0);
    lerp([18, 42, 72], [236, 224, 170], t)
}

fn crust_color(crust: u8) -> [u8; 3] {
    if crust == 1 {
        [196, 164, 110]
    } else {
        [40, 90, 160]
    }
}

fn plate_tint(id: u8) -> [u8; 3] {
    hsv(f64::from(id) * 137.50776405003785, 0.55, 0.88)
}

fn tone(rgb: [u8; 3], shade: f32, ocean: bool) -> [u8; 3] {
    let shade = f64::from(shade).clamp(0.0, 1.0);
    let (ambient, gain) = if ocean { (0.78, 0.22) } else { (0.42, 0.58) };
    let light = ambient + gain * shade;
    [
        byte(f64::from(rgb[0]) * light),
        byte(f64::from(rgb[1]) * light),
        byte(f64::from(rgb[2]) * light),
    ]
}

fn ramp(stops: &[(f64, [u8; 3])], x: f64) -> [u8; 3] {
    let Some((first_x, first_c)) = stops.first().copied() else {
        return GREY;
    };
    if x <= first_x {
        return first_c;
    }
    for pair in stops.windows(2) {
        let (x0, c0) = pair[0];
        let (x1, c1) = pair[1];
        if x <= x1 {
            let t = if x1 == x0 { 0.0 } else { (x - x0) / (x1 - x0) };
            return lerp(c0, c1, t);
        }
    }
    stops.last().map(|stop| stop.1).unwrap_or(GREY)
}

fn lerp(a: [u8; 3], b: [u8; 3], t: f64) -> [u8; 3] {
    let t = t.clamp(0.0, 1.0);
    [
        byte(f64::from(a[0]) + (f64::from(b[0]) - f64::from(a[0])) * t),
        byte(f64::from(a[1]) + (f64::from(b[1]) - f64::from(a[1])) * t),
        byte(f64::from(a[2]) + (f64::from(b[2]) - f64::from(a[2])) * t),
    ]
}

fn hsv(hue: f64, sat: f64, val: f64) -> [u8; 3] {
    let h = hue.rem_euclid(360.0) / 60.0;
    let c = val * sat;
    let x = c * (1.0 - (h % 2.0 - 1.0).abs());
    let m = val - c;
    let (r, g, b) = match h as i32 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    [byte(r + m), byte(g + m), byte(b + m)]
}

fn byte(value: f64) -> u8 {
    value.round().clamp(0.0, 255.0) as u8
}

fn norm(v: [f64; 3]) -> f64 {
    (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt()
}

const BACKGROUND: [u8; 3] = [12, 16, 28];
const OCEAN_DARK: [u8; 3] = [8, 14, 28];
const GREY: [u8; 3] = [214, 214, 214];

const OCEAN_STOPS: [(f64, [u8; 3]); 5] = [
    (-6000.0, [6, 22, 58]),
    (-4000.0, [14, 42, 88]),
    (-1000.0, [40, 96, 140]),
    (-200.0, [96, 170, 190]),
    (0.0, [150, 214, 214]),
];

const LAND_STOPS: [(f64, [u8; 3]); 6] = [
    (0.0, [64, 138, 62]),
    (300.0, [168, 184, 72]),
    (1000.0, [190, 158, 96]),
    (2000.0, [132, 90, 54]),
    (3500.0, [156, 156, 156]),
    (5000.0, [255, 255, 255]),
];
