//! Natural-colour imagery. Albedo follows continuous climate; light only modulates it.

use rayon::prelude::*;

use super::noise::sample_octave;
use super::palette::{
    biome_linear, is_ice_biome, linear_to_srgb, srgb_to_linear, vegetation_targets, ALPINE_SRGB,
    BEACH_SRGB, FOREST_BOREAL_SRGB, FOREST_TEMPERATE_SRGB, FOREST_TROPICAL_SRGB, GRASS_DRY_SRGB,
    GRASS_LUSH_SRGB, GREY_ROCK_SRGB, HAZE_SRGB, ICE_SRGB, LAKE_DEEP_SRGB, LAKE_SHALLOW_SRGB,
    OCEAN_DEEP_SRGB, OCEAN_MID_SRGB, OCEAN_SHELF_SRGB, RED_BROWN_SRGB, REEF_SRGB, SALT_CRUST_SRGB,
    SALT_LAKE_SRGB, SAND_BARE_SRGB, SNOW_SRGB, TUNDRA_SRGB,
};
use super::render::{
    direction_from_lon_lat, mercator_latitude, mercator_longitude, mercator_world_px,
};
use super::{octave_fade, smoothstep, unit, AtlasTerrain, TerrainCover};
use daena_physical::products::Biome;

const AMBIENT: f64 = 0.30;
const SUN_ALTITUDE_DEG: f64 = 35.0;
const SUN_AZIMUTH_DEG: f64 = 315.0;
const LAPSE_C_PER_KM: f64 = 6.5;
const HUE_WAVELENGTHS_M: [f64; 3] = [4_000.0, 600.0, 80.0];
const LITHOLOGY_WAVELENGTHS_M: [f64; 2] = [30_000.0, 6_000.0];
const SNOW_WAVELENGTHS_M: [f64; 2] = [2_000.0, 400.0];
/// Warmest dithered snow line, plus the smoothstep shoulder.
const SNOW_SKIP_C: f64 = 2.2;
/// Domain-warp displacement per octave, as a share of that octave's wavelength.
const WARP_PER_WAVELENGTH: f64 = 0.12;

#[derive(Clone, Copy)]
struct Sample {
    height: f64,
    cover: u8,
    ice: u8,
}

/// RGBA pixels for the imagery style. Geometry matches [`super::render_pixels`].
#[allow(clippy::too_many_arguments)]
#[must_use]
pub fn render_imagery_pixels(
    terrain: &AtlasTerrain,
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
    let exaggeration = imagery_exaggeration(z);
    let sun = sun_direction();
    let radius = terrain.radius_m();
    let sea_m = terrain.sea_level_m();
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
            let ground = ground_or_zero(z, lat, radius);
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
            let ground = ground_or_zero(z, lat, radius).max(1e-6);
            for x in 0..out_w {
                let col = x + halo;
                let centre = samples[buf_row * buf_w + col];
                let east = samples[buf_row * buf_w + col + halo].height;
                let west = samples[buf_row * buf_w + col - halo].height;
                let north = samples[(buf_row - halo) * buf_w + col].height;
                let south = samples[(buf_row + halo) * buf_w + col].height;
                let gx = gx0_f + (x as f64 + 0.5) / scale_f;
                let lon = mercator_longitude(gx, world);
                let direction = direction_from_lon_lat(lon, lat);
                let rgb = shade_pixel(
                    terrain,
                    Shade {
                        height: centre.height,
                        cover: cover_from_byte(centre.cover),
                        ice: centre.ice != 0,
                        east,
                        west,
                        north,
                        south,
                        ground,
                        lat,
                        direction,
                        exaggeration,
                        sun,
                        z,
                        sea_m,
                    },
                );
                let pixel = x * 4;
                row[pixel] = rgb[0];
                row[pixel + 1] = rgb[1];
                row[pixel + 2] = rgb[2];
                row[pixel + 3] = 255;
            }
        });
    rgba
}

struct Shade {
    height: f64,
    cover: TerrainCover,
    ice: bool,
    east: f64,
    west: f64,
    north: f64,
    south: f64,
    ground: f64,
    lat: f64,
    direction: [f64; 3],
    exaggeration: f64,
    sun: [f64; 3],
    z: u32,
    sea_m: f64,
}

fn shade_pixel(terrain: &AtlasTerrain, shade: Shade) -> [u8; 3] {
    let dzdx = (shade.east - shade.west) / (2.0 * shade.ground);
    let dzdy = (shade.north - shade.south) / (2.0 * shade.ground);
    let slope = (dzdx * dzdx + dzdy * dzdy).sqrt();
    let bowl = (shade.east + shade.west + shade.north + shade.south - 4.0 * shade.height)
        / (4.0 * shade.ground);
    let light = imagery_light(dzdx, dzdy, shade.exaggeration, bowl, shade.sun);
    let p = unit(shade.direction);
    let (cells, weights) = terrain.surface().bilinear_stencil(p);
    let depth = shade.sea_m - shade.height;
    if shade.cover == TerrainCover::Ocean && depth > slope * shade.ground + 0.5 {
        let colour = haze_colour(
            ocean_linear(terrain, &shade, cells, weights, light, p),
            shade.z,
        );
        return encode(colour);
    }
    let mut albedo = land_albedo(terrain, &shade, p, slope, bowl);
    let climate = bilinear_climate(terrain, cells, weights);
    let summer = summer_temperature(climate.temp_c, shade.lat, climate.elevation_m, shade.height);
    // Warmer than the dithered snow line. Skip the snow samples.
    let snow = if summer > SNOW_SKIP_C {
        0.0
    } else {
        let snow_n = faded_noise(
            &terrain.snow_octaves,
            &SNOW_WAVELENGTHS_M,
            p,
            shade.ground,
            terrain.radius_m(),
        );
        snow_fraction(summer, slope, pole_facing(shade.lat, dzdx, dzdy), snow_n)
    };
    albedo = lerp3(albedo, srgb_to_linear(SNOW_SRGB), snow);
    let mut colour = scale3(albedo, light);
    colour = blend_water(terrain, colour, &shade, cells, weights, slope, light, p);
    encode(haze_colour(colour, shade.z))
}

struct Climate {
    temp_c: f64,
    elevation_m: f64,
}

fn bilinear_climate(terrain: &AtlasTerrain, cells: [u32; 4], weights: [f64; 4]) -> Climate {
    let mut temp_c = 0.0;
    let mut elevation_m = 0.0;
    for (slot, cell) in cells.iter().enumerate() {
        let index = *cell as usize;
        let weight = weights[slot];
        temp_c += weight * f64::from(terrain.temp_c[index]);
        elevation_m += weight * f64::from(terrain.elevation_m[index]);
    }
    Climate {
        temp_c,
        elevation_m,
    }
}

struct Field {
    temp_c: f64,
    moisture: f64,
    elevation_m: f64,
    palette: [f64; 3],
    veg_target: f64,
    forest_target: f64,
    ice_w: f64,
    salt_w: f64,
    wet_w: f64,
}

fn bilinear_field(terrain: &AtlasTerrain, cells: [u32; 4], weights: [f64; 4]) -> Field {
    let mut field = Field {
        temp_c: 0.0,
        moisture: 0.0,
        elevation_m: 0.0,
        palette: [0.0; 3],
        veg_target: 0.0,
        forest_target: 0.0,
        ice_w: 0.0,
        salt_w: 0.0,
        wet_w: 0.0,
    };
    for (slot, cell) in cells.iter().enumerate() {
        let index = *cell as usize;
        let weight = weights[slot];
        field.temp_c += weight * f64::from(terrain.temp_c[index]);
        field.moisture += weight * f64::from(terrain.moisture[index]);
        field.elevation_m += weight * f64::from(terrain.elevation_m[index]);
        let biome_id = terrain.biome[index];
        let palette = biome_linear(biome_id);
        field.palette[0] += weight * palette[0];
        field.palette[1] += weight * palette[1];
        field.palette[2] += weight * palette[2];
        let (veg_target, forest_target) = vegetation_targets(biome_id);
        field.veg_target += weight * veg_target;
        field.forest_target += weight * forest_target;
        let Some(biome) = Biome::from_u8(biome_id) else {
            continue;
        };
        if is_ice_biome(biome) {
            field.ice_w += weight;
        }
        if biome == Biome::SaltFlat {
            field.salt_w += weight;
        }
        if matches!(biome, Biome::Wetland | Biome::Mangrove) {
            field.wet_w += weight;
        }
    }
    field
}

fn land_albedo(
    terrain: &AtlasTerrain,
    shade: &Shade,
    p: [f64; 3],
    slope: f64,
    bowl: f64,
) -> [f64; 3] {
    let warped = warp_direction(terrain, p, shade.ground);
    let (cells, weights) = terrain.surface().bilinear_stencil(warped);
    let field = bilinear_field(terrain, cells, weights);
    let summer = summer_temperature(field.temp_c, shade.lat, field.elevation_m, shade.height);
    let moisture = field.moisture;
    let breakup = breakup_noise(terrain, warped, shade.ground);
    // The biome sets the vegetation; climate nudges it and summer cold thins it.
    let climate_veg = smoothstep(0.12, 0.9, moisture);
    let cold = smoothstep(-2.0, 8.0, summer);
    let mut veg = (0.8 * field.veg_target + 0.2 * climate_veg) * cold;
    veg = (veg + 0.25 * breakup).clamp(0.0, 1.0);
    veg *= 1.0 - smoothstep(0.35, 0.9, slope);
    let forest_share = (field.forest_target + 0.1 * breakup).clamp(0.0, 1.0);
    let grass = lerp3(
        srgb_to_linear(GRASS_DRY_SRGB),
        srgb_to_linear(GRASS_LUSH_SRGB),
        smoothstep(0.12, 1.0, moisture),
    );
    let forest = forest_colour(summer);
    let alpine = srgb_to_linear(ALPINE_SRGB);
    let treeline = smoothstep(11.0, 7.0, summer);
    // Cold open ground is tundra: low, olive-brown, not lush grass.
    let grass = lerp3(
        grass,
        srgb_to_linear(TUNDRA_SRGB),
        smoothstep(13.0, 6.0, summer),
    );
    let vegetation = lerp3(
        lerp3(grass, forest, forest_share),
        alpine,
        treeline * forest_share,
    );
    let bare_w = 1.0 - veg;
    let bare = if bare_w > 0.02 {
        bare_colour(terrain, warped, shade.ground, summer, moisture)
    } else {
        vegetation
    };
    let mut albedo = lerp3(bare, vegetation, veg);
    albedo = lerp3(albedo, field.palette, 0.20);
    if field.wet_w > 0.0 {
        albedo = scale3(albedo, 1.0 - 0.25 * field.wet_w);
    }
    let (broad, canopy, canopy_on) = grain_noise(terrain, warped, shade.ground);
    if field.salt_w > 0.0 {
        let crust = scale3(srgb_to_linear(SALT_CRUST_SRGB), 1.0 + 0.04 * broad);
        albedo = lerp3(albedo, crust, field.salt_w);
    }
    if shade.cover != TerrainCover::Ocean && (shade.ice || field.ice_w > 0.0) {
        let ice_w = if shade.ice { 1.0 } else { field.ice_w };
        albedo = lerp3(albedo, srgb_to_linear(ICE_SRGB), ice_w);
    }
    albedo = apply_beach(
        terrain,
        albedo,
        shade.height,
        shade.sea_m,
        slope,
        p,
        shade.cover,
    );
    let concave = smoothstep(0.0, 0.15, bowl) * (1.0 - smoothstep(0.8, 1.6, moisture));
    let ridge = smoothstep(0.0, 0.15, -bowl);
    albedo = scale3(albedo, 1.0 - 0.08 * concave + 0.05 * ridge);
    let signal = if canopy_on {
        broad * (1.0 - forest_share) + canopy * forest_share
    } else {
        broad
    };
    let amp = 0.07 + 0.05 * forest_share;
    let hue = faded_noise(
        &terrain.green_hue_octaves,
        &HUE_WAVELENGTHS_M,
        warped,
        shade.ground,
        terrain.radius_m(),
    );
    let hue_shift = 0.04 * hue * veg;
    let gain = 1.0 + amp * signal;
    albedo[0] = albedo[0] * gain - hue_shift * 0.45;
    albedo[1] = albedo[1] * gain + hue_shift;
    albedo[2] = albedo[2] * gain - hue_shift * 0.55;
    albedo
}

fn forest_colour(summer_c: f64) -> [f64; 3] {
    let boreal = srgb_to_linear(FOREST_BOREAL_SRGB);
    let temperate = srgb_to_linear(FOREST_TEMPERATE_SRGB);
    let tropical = srgb_to_linear(FOREST_TROPICAL_SRGB);
    let warm = smoothstep(10.0, 18.0, summer_c);
    let hot = smoothstep(18.0, 26.0, summer_c);
    lerp3(lerp3(boreal, temperate, warm), tropical, hot)
}

fn bare_colour(
    terrain: &AtlasTerrain,
    p: [f64; 3],
    ground: f64,
    summer_c: f64,
    moisture: f64,
) -> [f64; 3] {
    let lith = faded_noise(
        &terrain.lithology_octaves,
        &LITHOLOGY_WAVELENGTHS_M,
        p,
        ground,
        terrain.radius_m(),
    );
    let mut t = (lith * 0.5 + 0.5).clamp(0.0, 1.0);
    let cold = smoothstep(8.0, -2.0, summer_c) * (1.0 - smoothstep(0.15, 0.7, moisture));
    t *= 1.0 - 0.8 * cold;
    let grey = srgb_to_linear(GREY_ROCK_SRGB);
    let red = srgb_to_linear(RED_BROWN_SRGB);
    let sand = srgb_to_linear(SAND_BARE_SRGB);
    // Sand is the middle of the range; red-brown only reaches 60% at the far
    // end, so a continent does not read as one orange slab.
    if t < 0.5 {
        lerp3(grey, sand, t * 2.0)
    } else {
        lerp3(sand, red, (t - 0.5) * 1.2)
    }
}

fn apply_beach(
    terrain: &AtlasTerrain,
    albedo: [f64; 3],
    height: f64,
    sea_m: f64,
    slope: f64,
    p: [f64; 3],
    cover: TerrainCover,
) -> [f64; 3] {
    if cover != TerrainCover::Land || height <= sea_m {
        return albedo;
    }
    let noise = if terrain.beach_octaves.is_empty() {
        0.0
    } else {
        sample_octave(
            &terrain.beach_octaves[0],
            p,
            terrain.radius_m() / (0.5 * terrain.cell_size_m()),
        )
    };
    let width = 2.0 + 1.1 * noise;
    if width <= 0.0 {
        return albedo;
    }
    let near = 1.0 - smoothstep(width * 0.35, width, height - sea_m);
    let gentle = 1.0 - smoothstep(0.04, 0.22, slope);
    lerp3(albedo, srgb_to_linear(BEACH_SRGB), near * gentle)
}

#[allow(clippy::too_many_arguments)]
fn blend_water(
    terrain: &AtlasTerrain,
    land: [f64; 3],
    shade: &Shade,
    cells: [u32; 4],
    weights: [f64; 4],
    slope: f64,
    light: f64,
    p: [f64; 3],
) -> [f64; 3] {
    let ocean_w = parent_weight(terrain, cells, weights, true);
    let (lake_w, lake_level, salt) = lake_parent(terrain, cells, weights);
    let (water_level, water) =
        if shade.cover == TerrainCover::Ocean || (ocean_w > 0.0 && lake_w == 0.0) {
            (
                shade.sea_m,
                ocean_linear(terrain, shade, cells, weights, light, p),
            )
        } else if shade.cover == TerrainCover::Lake || lake_w > 0.0 {
            let level = if lake_w > 0.0 {
                lake_level
            } else {
                f64::from(terrain.lake_level_m[terrain.surface().cell_at_direction(p) as usize])
            };
            let colour = if salt {
                salt_lake_colour(level - shade.height, light)
            } else {
                lake_colour(level - shade.height, light)
            };
            (level, colour)
        } else {
            return land;
        };
    if shade.cover == TerrainCover::Land && ocean_w == 0.0 && lake_w == 0.0 {
        return land;
    }
    let alpha = land_coverage(shade.height, water_level, slope, shade.ground);
    lerp3(water, land, alpha)
}

fn ocean_linear(
    terrain: &AtlasTerrain,
    shade: &Shade,
    cells: [u32; 4],
    weights: [f64; 4],
    light: f64,
    p: [f64; 3],
) -> [f64; 3] {
    let (shelf, reef) = shelf_reef_weight(terrain, cells, weights);
    let ice = sea_ice_amount(terrain, shade, cells, weights, p);
    ocean_colour(shade.sea_m - shade.height, light, ice, shelf, reef)
}

fn ocean_colour(depth: f64, light: f64, ice_amount: f64, shelf_w: f64, reef_w: f64) -> [f64; 3] {
    let depth = depth.max(0.0);
    let deep_w = 1.0 - (-depth / 40.0).exp();
    let mut colour = lerp3(
        srgb_to_linear(OCEAN_MID_SRGB),
        srgb_to_linear(OCEAN_DEEP_SRGB),
        deep_w,
    );
    let class_w = (shelf_w + reef_w).clamp(0.0, 1.0);
    let tint_w = 0.8 * class_w * (-depth / 12.0).exp();
    if tint_w > 0.0 {
        let reef_t = reef_w / class_w.max(1e-6);
        let tint = lerp3(
            srgb_to_linear(OCEAN_SHELF_SRGB),
            srgb_to_linear(REEF_SRGB),
            reef_t.clamp(0.0, 1.0),
        );
        colour = lerp3(colour, tint, tint_w);
    }
    if ice_amount > 0.0 {
        colour = lerp3(colour, srgb_to_linear(ICE_SRGB), ice_amount.clamp(0.0, 1.0));
    }
    shade_water(colour, depth, light)
}

fn shelf_reef_weight(terrain: &AtlasTerrain, cells: [u32; 4], weights: [f64; 4]) -> (f64, f64) {
    let mut shelf = 0.0;
    let mut reef = 0.0;
    for (slot, cell) in cells.iter().enumerate() {
        let index = *cell as usize;
        if terrain.ocean[index] == 0 {
            continue;
        }
        match Biome::from_u8(terrain.biome[index]) {
            Some(Biome::Shelf) => shelf += weights[slot],
            Some(Biome::Reef) => reef += weights[slot],
            _ => {}
        }
    }
    (shelf, reef)
}

fn sea_ice_amount(
    terrain: &AtlasTerrain,
    shade: &Shade,
    cells: [u32; 4],
    weights: [f64; 4],
    p: [f64; 3],
) -> f64 {
    if shade.cover != TerrainCover::Ocean {
        return 0.0;
    }
    // Imagery is a summer view: seasonal sea ice has melted to open water,
    // so only perennial ice (or the ice flag) paints.
    let mut perennial = 0.0;
    for (slot, cell) in cells.iter().enumerate() {
        let index = *cell as usize;
        if terrain.ocean[index] == 0 {
            continue;
        }
        let biome = Biome::from_u8(terrain.biome[index]);
        if terrain.ice[index] != 0 || biome == Some(Biome::SeaIcePerennial) {
            perennial += weights[slot];
        }
    }
    if perennial == 0.0 {
        return 0.0;
    }
    perennial * perennial_cover(floe_noise(terrain, p, shade.ground))
}

/// `None` when every floe octave is smaller than the pixel.
fn floe_noise(terrain: &AtlasTerrain, p: [f64; 3], ground: f64) -> Option<f64> {
    let lambdas = [0.5 * terrain.cell_size_m(), 0.125 * terrain.cell_size_m()];
    let mut acc = 0.0;
    let mut weight_sum = 0.0;
    for (index, octave) in terrain.snow_octaves.iter().enumerate() {
        if index >= lambdas.len() {
            break;
        }
        let fade = octave_fade(lambdas[index], ground);
        if fade == 0.0 {
            continue;
        }
        let weight = fade * if index == 0 { 0.65 } else { 0.35 };
        acc += weight * sample_octave(octave, p, terrain.radius_m() / lambdas[index]);
        weight_sum += weight;
    }
    if weight_sum == 0.0 {
        None
    } else {
        Some((acc / weight_sum).clamp(-1.0, 1.0))
    }
}

fn perennial_cover(noise: Option<f64>) -> f64 {
    let Some(noise) = noise else {
        return 0.9;
    };
    // Cracks sit in the low tail so the mean stays near 0.9.
    let crack = smoothstep(-0.15, -0.55, noise);
    0.9 - 0.5 * crack
}

fn lake_colour(depth: f64, light: f64) -> [f64; 3] {
    let t = 1.0 - (-depth.max(0.0) / 18.0).exp();
    let colour = lerp3(
        srgb_to_linear(LAKE_SHALLOW_SRGB),
        srgb_to_linear(LAKE_DEEP_SRGB),
        t,
    );
    shade_water(colour, depth, light)
}

fn salt_lake_colour(depth: f64, light: f64) -> [f64; 3] {
    let pale = srgb_to_linear(SALT_LAKE_SRGB);
    let deep = scale3(pale, 0.72);
    let colour = lerp3(pale, deep, 1.0 - (-depth.max(0.0) / 12.0).exp());
    shade_water(colour, depth, light * 0.25 + 0.75)
}

/// Seafloor relief fades with depth. Deep water keeps at most ±5% shade.
fn shade_water(colour: [f64; 3], depth: f64, light: f64) -> [f64; 3] {
    let relief = (-depth.max(0.0) / 25.0).exp();
    let delta = light - 1.0;
    let deep_delta = delta.clamp(-0.05, 0.05);
    let factor = 1.0 + relief * delta + (1.0 - relief) * deep_delta;
    scale3(colour, factor)
}

fn parent_weight(terrain: &AtlasTerrain, cells: [u32; 4], weights: [f64; 4], ocean: bool) -> f64 {
    let mut weight = 0.0;
    for (slot, cell) in cells.iter().enumerate() {
        let index = *cell as usize;
        let hit = if ocean {
            terrain.ocean[index] != 0
        } else {
            terrain.wet[index] != 0
        };
        if hit {
            weight += weights[slot];
        }
    }
    weight
}

fn lake_parent(terrain: &AtlasTerrain, cells: [u32; 4], weights: [f64; 4]) -> (f64, f64, bool) {
    let mut weight = 0.0;
    let mut level = 0.0;
    let mut salt = false;
    let mut best = 0.0;
    for (slot, cell) in cells.iter().enumerate() {
        let index = *cell as usize;
        if terrain.wet[index] == 0 {
            continue;
        }
        weight += weights[slot];
        if weights[slot] >= best {
            best = weights[slot];
            level = f64::from(terrain.lake_level_m[index]);
            let biome = Biome::from_u8(terrain.biome[index]);
            salt = matches!(biome, Some(Biome::SaltLake | Biome::SaltFlat));
        }
    }
    (weight, level, salt)
}

/// Land coverage across one pixel. `1` is land, `0` is water.
fn land_coverage(height: f64, water: f64, slope: f64, pixel_m: f64) -> f64 {
    let denom = slope * pixel_m;
    if denom < 1e-6 {
        if height >= water {
            1.0
        } else {
            0.0
        }
    } else {
        (0.5 + (height - water) / denom).clamp(0.0, 1.0)
    }
}

fn warp_direction(terrain: &AtlasTerrain, p: [f64; 3], ground: f64) -> [f64; 3] {
    let (east, north) = super::tangent_axes(p);
    let mut de = 0.0;
    let mut dn = 0.0;
    // Each octave displaces by at most 0.12 of its own wavelength, so the warp
    // never folds space. Larger warps fold and smear the texture into swirls.
    let mut amp = WARP_PER_WAVELENGTH;
    let mut lambda = terrain.cell_size_m();
    for index in 0..3 {
        let fade = octave_fade(lambda, ground);
        if fade > 0.0 {
            let frequency = terrain.radius_m() / lambda;
            let metres = amp * terrain.cell_size_m() * fade;
            let angle = metres / terrain.radius_m();
            de += sample_octave(&terrain.cover_warp[index], p, frequency) * angle;
            dn += sample_octave(&terrain.cover_warp[index + 3], p, frequency) * angle;
        }
        amp *= 0.5;
        lambda *= 0.5;
    }
    unit([
        p[0] + east[0] * de + north[0] * dn,
        p[1] + east[1] * de + north[1] * dn,
        p[2] + east[2] * de + north[2] * dn,
    ])
}

fn breakup_noise(terrain: &AtlasTerrain, p: [f64; 3], ground: f64) -> f64 {
    let cell = terrain.cell_size_m();
    let lambdas = [cell * 0.5, cell * 0.125, cell * 0.03125];
    faded_noise(
        &terrain.cover_octaves,
        &lambdas,
        p,
        ground,
        terrain.radius_m(),
    )
}

/// Broad grain and the 60–400 m canopy band. The flag is set when the canopy band resolves.
fn grain_noise(terrain: &AtlasTerrain, p: [f64; 3], ground: f64) -> (f64, f64, bool) {
    let mut lambda = terrain.cell_size_m() * 0.25;
    let mut broad = 0.0;
    let mut broad_w = 0.0;
    let mut canopy = 0.0;
    let mut canopy_w = 0.0;
    for octave in &terrain.grain_octaves {
        if lambda < 50.0 {
            break;
        }
        let fade = octave_fade(lambda, ground);
        if fade == 0.0 {
            break;
        }
        let noise = sample_octave(octave, p, terrain.radius_m() / lambda);
        broad += fade * noise;
        broad_w += fade;
        let band = smoothstep(50.0, 90.0, lambda) * (1.0 - smoothstep(400.0, 700.0, lambda));
        if band > 0.0 {
            canopy += fade * band * noise;
            canopy_w += fade * band;
        }
        lambda *= 0.5;
    }
    let broad_n = if broad_w > 0.0 {
        (broad / broad_w).clamp(-1.0, 1.0)
    } else {
        0.0
    };
    let canopy_n = if canopy_w > 0.0 {
        (canopy / canopy_w).clamp(-1.0, 1.0)
    } else {
        0.0
    };
    (broad_n, canopy_n, canopy_w > 0.0)
}

fn faded_noise(
    octaves: &[super::noise::Octave],
    wavelengths: &[f64],
    p: [f64; 3],
    ground: f64,
    radius: f64,
) -> f64 {
    let mut acc = 0.0;
    let mut weight_sum = 0.0;
    for (index, octave) in octaves.iter().enumerate() {
        if index >= wavelengths.len() {
            break;
        }
        let lambda = wavelengths[index];
        let fade = octave_fade(lambda, ground);
        if fade == 0.0 {
            continue;
        }
        let weight = fade * 0.5f64.powi(index as i32);
        acc += weight * sample_octave(octave, p, radius / lambda);
        weight_sum += weight;
    }
    if weight_sum == 0.0 {
        0.0
    } else {
        (acc / weight_sum).clamp(-1.0, 1.0)
    }
}

fn local_temperature(temp_cell_c: f64, elevation_cell_m: f64, height_m: f64) -> f64 {
    temp_cell_c - LAPSE_C_PER_KM * (height_m - elevation_cell_m) / 1_000.0
}

/// Summer temperature: annual mean plus a latitude term, then the lapse rate.
#[must_use]
pub(super) fn summer_temperature(
    annual_c: f64,
    lat_rad: f64,
    elevation_cell_m: f64,
    height_m: f64,
) -> f64 {
    let boost = 3.0 + 14.0 * lat_rad.sin().abs().powf(1.5);
    local_temperature(annual_c + boost, elevation_cell_m, height_m)
}

/// Perennial snow from summer temperature. `noise` in `[-1, 1]` dithers the
/// 0 °C line by less than half a degree. Pole-facing slopes hold snow 1.5 °C
/// warmer. Slopes steeper than about 0.8 shed it.
#[must_use]
pub(super) fn snow_fraction(temp_c: f64, slope: f64, pole_facing: f64, noise: f64) -> f64 {
    let line = 1.5 * pole_facing.clamp(0.0, 1.0) + 0.45 * noise.clamp(-1.0, 1.0);
    let mut cover = 1.0 - smoothstep(line - 0.2, line + 0.2, temp_c);
    cover *= 1.0 - smoothstep(0.8, 1.05, slope);
    cover
}

/// Flat ground (`dz = 0`, no curvature) returns exactly 1.
#[must_use]
pub(super) fn imagery_light(
    dzdx: f64,
    dzdy: f64,
    exaggeration: f64,
    bowl: f64,
    sun: [f64; 3],
) -> f64 {
    let nx = -dzdx * exaggeration;
    let ny = -dzdy * exaggeration;
    let nz = 1.0;
    let len = (nx * nx + ny * ny + nz * nz).sqrt();
    let lambert = ((nx * sun[0] + ny * sun[1] + nz * sun[2]) / len).max(0.0);
    let lambert_flat = sun[2];
    let occlusion = 1.0 - 0.12 * smoothstep(0.0, 0.2, bowl);
    (AMBIENT + (1.0 - AMBIENT) * (lambert / lambert_flat)) * occlusion
}

fn imagery_exaggeration(z: u32) -> f64 {
    2f64.powf((11.0 - f64::from(z)) / 2.5).max(1.0)
}

fn sun_direction() -> [f64; 3] {
    let altitude = SUN_ALTITUDE_DEG.to_radians();
    let azimuth = SUN_AZIMUTH_DEG.to_radians();
    let cos_alt = altitude.cos();
    [
        azimuth.sin() * cos_alt,
        azimuth.cos() * cos_alt,
        altitude.sin(),
    ]
}

fn pole_facing(lat: f64, dzdx: f64, dzdy: f64) -> f64 {
    let slope = (dzdx * dzdx + dzdy * dzdy).sqrt();
    if slope < 1e-8 {
        return 0.0;
    }
    let pole_north = if lat >= 0.0 { 1.0 } else { -1.0 };
    ((-dzdy * pole_north) / slope).max(0.0)
}

fn haze_amount(z: u32) -> f64 {
    0.06 * (1.0 - smoothstep(4.0, 6.0, f64::from(z)))
}

fn haze_colour(colour: [f64; 3], z: u32) -> [f64; 3] {
    let haze = haze_amount(z);
    if haze > 0.0 {
        lerp3(colour, srgb_to_linear(HAZE_SRGB), haze)
    } else {
        colour
    }
}

fn encode(colour: [f64; 3]) -> [u8; 3] {
    linear_to_srgb([
        colour[0].clamp(0.0, 1.0),
        colour[1].clamp(0.0, 1.0),
        colour[2].clamp(0.0, 1.0),
    ])
}

fn ground_or_zero(z: u32, lat: f64, radius: f64) -> f64 {
    super::ground_metres_per_pixel(z, lat, radius)
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

fn lerp3(a: [f64; 3], b: [f64; 3], t: f64) -> [f64; 3] {
    [
        a[0] + (b[0] - a[0]) * t,
        a[1] + (b[1] - a[1]) * t,
        a[2] + (b[2] - a[2]) * t,
    ]
}

fn scale3(rgb: [f64; 3], factor: f64) -> [f64; 3] {
    [rgb[0] * factor, rgb[1] * factor, rgb[2] * factor]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flat_ground_with_no_detail_lights_to_its_albedo() {
        let sun = sun_direction();
        let light = imagery_light(0.0, 0.0, 4.0, 0.0, sun);
        assert!(
            (light - 1.0).abs() < 1e-12,
            "flat light {light} sun {sun:?}"
        );
        let curved = imagery_light(0.0, 0.0, 1.0, 0.0, sun);
        assert!((curved - 1.0).abs() < 1e-12);
    }

    #[test]
    fn snow_line_uses_summer_temperature() {
        let equator = 0.0;
        let high = summer_temperature(0.0, equator, 0.0, 1_200.0);
        let low = summer_temperature(0.0, equator, 0.0, 200.0);
        assert!(
            snow_fraction(high, 0.05, 0.0, 0.0) > 0.95,
            "1200 m at the equator should be snow, T {high}"
        );
        assert!(
            snow_fraction(low, 0.05, 1.0, -1.0) < 0.05,
            "200 m at the equator should be clear, T {low}"
        );
        assert!(snow_fraction(high, 0.05, 1.0, -1.0) > 0.95);
        let shed = snow_fraction(high, 1.2, 0.0, 0.0);
        assert!(shed < 0.05, "a steep face sheds snow, got {shed}");
        let tundra_lat = 65.0_f64.to_radians();
        let tundra = summer_temperature(-8.0, tundra_lat, 200.0, 200.0);
        assert!(
            tundra > 2.0,
            "tundra summer should be above freezing, T {tundra}"
        );
        assert!(
            snow_fraction(tundra, 0.1, 1.0, -1.0) < 0.05,
            "tundra summer stays snow-free, T {tundra}"
        );
    }
}
