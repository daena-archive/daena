//! Natural-colour imagery. Albedo follows continuous climate; light only modulates it.

use rayon::prelude::*;

use super::noise::sample_octave;
use super::palette::{
    biome_linear, is_ice_biome, linear_to_srgb, srgb_to_linear, ALPINE_SRGB, BARE_ICE_SRGB,
    BEACH_SRGB, BLUE_ICE_SRGB, FOREST_BOREAL_SRGB, FOREST_TEMPERATE_SRGB, FOREST_TROPICAL_SRGB,
    GRASS_DRY_SRGB, GRASS_LUSH_SRGB, GREY_ROCK_SRGB, HAZE_SRGB, ICE_DIRT_SRGB, LAKE_DEEP_SRGB,
    LAKE_SHALLOW_SRGB, MORAINE_SRGB, OCEAN_DEEP_SRGB, OCEAN_MID_SRGB, OCEAN_SHELF_SRGB,
    PACK_ICE_SRGB, RED_BROWN_SRGB, REEF_SRGB, SALT_CRUST_SRGB, SALT_LAKE_SRGB, SAND_BARE_SRGB,
    SNOW_SRGB, TREE_CLUMP_SRGB, TUNDRA_SRGB,
};
use super::render::{
    direction_from_lon_lat, mercator_latitude, mercator_longitude, mercator_world_px,
};
use super::{octave_fade, smoothstep, unit, AtlasTerrain, TerrainCover};
use daena_physical::products::Biome;

const AMBIENT: f64 = 0.30;
/// Sky light. Flat ground divides this back out, so mid-tones stay on albedo.
const AMBIENT_TINT: [f64; 3] = [0.92, 0.96, 1.06];
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
                let (height, cover, _) =
                    terrain.sample_pixel(direction_from_lon_lat(lon, lat), ground);
                *sample = Sample {
                    height,
                    cover: cover_byte(cover),
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
    let p = unit(shade.direction);
    let (cells, weights) = terrain.surface().bilinear_stencil(p);
    let ice_w = super::ice_weight_of(terrain, cells, weights);
    // Valley occlusion reads as a dark ring on the ice edge. Drop it there.
    let ao_scale = 1.0 - smoothstep(0.04, 0.40, ice_w);
    let light = imagery_light(dzdx, dzdy, shade.exaggeration, bowl, shade.sun, ao_scale);
    let depth = shade.sea_m - shade.height;
    if shade.cover == TerrainCover::Ocean && depth > slope * shade.ground + 0.5 {
        let colour = haze_colour(
            ocean_linear(terrain, &shade, cells, weights, light, p),
            shade.z,
        );
        return encode(colour);
    }
    let mut albedo = land_albedo(terrain, &shade, p, slope, bowl, ice_w);
    let climate = bilinear_climate(terrain, cells, weights);
    let summer = summer_temperature(climate.temp_c, shade.lat, climate.elevation_m, shade.height);
    let extent = ice_extent(terrain, p, shade.ground, ice_w, bowl);
    let ice_cover = smoothstep(0.47, 0.53, extent);
    // Warmer than the dithered snow line, or already accumulation ice. Skip the samples.
    let snow = if summer > SNOW_SKIP_C || ice_cover > 0.85 {
        0.0
    } else {
        let snow_n = faded_noise(
            &terrain.snow_octaves,
            &SNOW_WAVELENGTHS_M,
            p,
            shade.ground,
            terrain.radius_m(),
        );
        snow_fraction(
            summer,
            slope,
            pole_facing(shade.lat, dzdx, dzdy),
            snow_n,
            bowl,
            sun_facing_slope(dzdx, dzdy, shade.sun),
        )
    };
    // Moraine only where the ground beside the ice is bare. Where the ice flag
    // ends inside a snowfield, a fringe would draw a grey line through snow.
    let bare = (1.0 - snow) * (1.0 - snow);
    let fringe = moraine_weight(terrain, extent, shade.ground, ice_w) * bare;
    if fringe > 0.0 {
        albedo = lerp3(albedo, srgb_to_linear(MORAINE_SRGB), fringe);
    }
    // Full snow here; the ice blend below covers it. Scaling snow by
    // `1 − ice_cover` as well let bare ground show through halfway across the
    // ice edge and drew a grey line round ice inside snowfields.
    albedo = lerp3(albedo, srgb_to_linear(SNOW_SRGB), snow);
    if ice_cover > 0.0 {
        let ice_alb = ice_surface(terrain, p, shade.ground, summer, cells, weights);
        albedo = lerp3(albedo, ice_alb, ice_cover);
    }
    let mut colour = mul_light(albedo, light);
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
    /// Savanna, grassland, and steppe. Tree clumps and broad patches use this.
    open_w: f64,
    glacier_w: f64,
    sheet_w: f64,
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
        open_w: 0.0,
        glacier_w: 0.0,
        sheet_w: 0.0,
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
        let smooth = terrain.veg_smooth[index];
        field.veg_target += weight * f64::from(smooth[0]) / 255.0;
        field.forest_target += weight * f64::from(smooth[1]) / 255.0;
        let Some(biome) = Biome::from_u8(biome_id) else {
            continue;
        };
        if is_ice_biome(biome) {
            field.ice_w += weight;
        }
        if biome == Biome::Glacier {
            field.glacier_w += weight;
        } else if biome == Biome::IceSheet {
            field.sheet_w += weight;
        }
        if matches!(biome, Biome::Savanna | Biome::Grassland | Biome::ColdSteppe) {
            field.open_w += weight;
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
    ice_w: f64,
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
    albedo = apply_beach(
        terrain,
        albedo,
        shade.height,
        shade.sea_m,
        slope,
        p,
        shade.cover,
    );
    // The ice edge sits in valleys. The old darkening painted a smudge around it.
    let ice_edge = smoothstep(0.04, 0.40, ice_w.max(field.ice_w));
    let concave =
        smoothstep(0.0, 0.15, bowl) * (1.0 - smoothstep(0.8, 1.6, moisture)) * (1.0 - ice_edge);
    let ridge = smoothstep(0.0, 0.15, -bowl);
    albedo = scale3(albedo, 1.0 - 0.08 * concave + 0.05 * ridge);
    let signal = if canopy_on {
        broad * (1.0 - forest_share) + canopy * forest_share
    } else {
        broad
    };
    // Bare and sparse ground carries ±12%. Closed canopy stays near ±7%.
    let sparse = 1.0 - smoothstep(0.25, 0.70, veg);
    let amp = 0.07 + 0.05 * sparse.max(forest_share);
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
    if field.open_w > 0.35 {
        albedo = apply_patches(terrain, albedo, warped, shade.ground, field.open_w, veg);
    }
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
    light: [f64; 3],
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
    light: [f64; 3],
    p: [f64; 3],
) -> [f64; 3] {
    let (shelf, reef) = shelf_reef_weight(terrain, cells, weights);
    let water = ocean_colour(shade.sea_m - shade.height, light, shelf, reef);
    apply_sea_ice(terrain, shade, cells, weights, p, water, light)
}

fn ocean_colour(depth: f64, light: [f64; 3], shelf_w: f64, reef_w: f64) -> [f64; 3] {
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

fn apply_sea_ice(
    terrain: &AtlasTerrain,
    shade: &Shade,
    cells: [u32; 4],
    weights: [f64; 4],
    p: [f64; 3],
    water: [f64; 3],
    light: [f64; 3],
) -> [f64; 3] {
    if shade.cover != TerrainCover::Ocean {
        return water;
    }
    // A latitude cutoff is a straight band at world scale. Shift the lookup
    // north–south by up to ±140 km where that octave still resolves.
    let (cells, weights) = if let Some(shifted) = sea_ice_shift(terrain, p, shade.ground, shade.lat)
    {
        terrain.surface().bilinear_stencil(shifted)
    } else {
        (cells, weights)
    };
    // Summer view: seasonal sea ice has melted. Only perennial ice paints.
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
    if perennial <= 0.0 {
        return water;
    }
    let (cover, lead, floe_n) = pack_structure(terrain, p, shade.ground, perennial);
    if cover <= 0.0 {
        return water;
    }
    let mut pack = srgb_to_linear(PACK_ICE_SRGB);
    pack = scale3(pack, 1.0 + 0.04 * floe_n);
    pack = scale3(pack, 1.0 - 0.62 * lead);
    let lum = 0.2126 * light[0] + 0.7152 * light[1] + 0.0722 * light[2];
    pack = scale3(pack, 1.0 + 0.04 * (lum - 1.0));
    lerp3(water, pack, cover)
}

/// `None` when the pixel is too far south, or the 280 km octave has faded.
fn sea_ice_shift(terrain: &AtlasTerrain, p: [f64; 3], ground: f64, lat: f64) -> Option<[f64; 3]> {
    if lat.abs() < 0.70 {
        return None;
    }
    const LAMBDA: f64 = 280_000.0;
    let fade = octave_fade(LAMBDA, ground);
    if fade == 0.0 {
        return None;
    }
    let noise = sample_octave(&terrain.sea_ice_octaves[0], p, terrain.radius_m() / LAMBDA);
    let (_, north) = super::tangent_axes(p);
    let ang = 140_000.0 * fade * noise / terrain.radius_m();
    Some(unit([
        p[0] + north[0] * ang,
        p[1] + north[1] * ang,
        p[2] + north[2] * ang,
    ]))
}

fn lake_colour(depth: f64, light: [f64; 3]) -> [f64; 3] {
    let t = 1.0 - (-depth.max(0.0) / 18.0).exp();
    let colour = lerp3(
        srgb_to_linear(LAKE_SHALLOW_SRGB),
        srgb_to_linear(LAKE_DEEP_SRGB),
        t,
    );
    shade_water(colour, depth, light)
}

fn salt_lake_colour(depth: f64, light: [f64; 3]) -> [f64; 3] {
    let pale = srgb_to_linear(SALT_LAKE_SRGB);
    let deep = scale3(pale, 0.72);
    let colour = lerp3(pale, deep, 1.0 - (-depth.max(0.0) / 12.0).exp());
    let mild = [
        light[0] * 0.25 + 0.75,
        light[1] * 0.25 + 0.75,
        light[2] * 0.25 + 0.75,
    ];
    shade_water(colour, depth, mild)
}

/// Seafloor relief fades with depth. Deep water keeps at most ±5% shade.
fn shade_water(colour: [f64; 3], depth: f64, light: [f64; 3]) -> [f64; 3] {
    let relief = (-depth.max(0.0) / 25.0).exp();
    let mut out = [0.0; 3];
    for channel in 0..3 {
        let delta = light[channel] - 1.0;
        let deep_delta = delta.clamp(-0.05, 0.05);
        let factor = 1.0 + relief * delta + (1.0 - relief) * deep_delta;
        out[channel] = colour[channel] * factor;
    }
    out
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
    // Start at four cells: at low zoom a cell is only a few pixels, and the
    // biome map shows as blocks unless the warp moves boundaries by about a
    // cell. Each octave still moves at most 0.12 of its own wavelength.
    let octaves = super::COVER_WARP_OCTAVES;
    let mut lambda = 4.0 * terrain.cell_size_m();
    for index in 0..octaves {
        let fade = octave_fade(lambda, ground);
        if fade > 0.0 {
            let frequency = terrain.radius_m() / lambda;
            let metres = WARP_PER_WAVELENGTH * lambda * fade;
            let angle = metres / terrain.radius_m();
            de += sample_octave(&terrain.cover_warp[index], p, frequency) * angle;
            dn += sample_octave(&terrain.cover_warp[index + octaves], p, frequency) * angle;
        }
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

/// Bilinear ice weight plus a valley/ridge term and multi-scale noise, compared
/// with 0.5 by the caller. No ice parent means no ice: the terrain term only
/// moves a margin that already exists.
fn ice_extent(terrain: &AtlasTerrain, p: [f64; 3], ground: f64, ice_w: f64, bowl: f64) -> f64 {
    if ice_w <= 0.0 {
        return 0.0;
    }
    // Curvature is per pixel. Once a pixel is a sizeable part of a cell, a
    // one-pixel valley spikes it and ice runs out along every small valley as
    // a thin bar, so the terrain term fades out at low zoom.
    let resolved = 1.0 - smoothstep(0.1, 0.25, ground / terrain.cell_size_m());
    let terrain_term = resolved
        * if bowl > 0.0 {
            (bowl * 2.6).min(0.55)
        } else {
            (bowl * 1.2).max(-0.22)
        };
    // Interior ice cannot fall through 0.5 even at the worst noise and ridge.
    if ice_w >= 0.94 {
        return ice_w + terrain_term;
    }
    ice_w + terrain_term + 0.18 * ice_margin_noise(terrain, p, ground)
}

fn ice_margin_noise(terrain: &AtlasTerrain, p: [f64; 3], ground: f64) -> f64 {
    let cell = terrain.cell_size_m();
    let lambdas = [4.0 * cell, 1.5 * cell];
    let coeffs = [0.65, 0.35];
    if octave_fade(lambdas[0], ground) == 0.0 {
        return 0.0;
    }
    let mut acc = 0.0;
    let mut weight_sum = 0.0;
    for index in 0..lambdas.len() {
        let fade = octave_fade(lambdas[index], ground);
        if fade == 0.0 {
            break;
        }
        let weight = coeffs[index] * fade;
        acc += weight
            * sample_octave(
                &terrain.ice_octaves[index],
                p,
                terrain.radius_m() / lambdas[index],
            );
        weight_sum += weight;
    }
    if weight_sum == 0.0 {
        0.0
    } else {
        (acc / weight_sum).clamp(-1.0, 1.0)
    }
}

fn moraine_weight(terrain: &AtlasTerrain, extent: f64, ground: f64, ice_w: f64) -> f64 {
    if ice_w <= 0.0 {
        return 0.0;
    }
    let outside = (0.5 - extent).max(0.0);
    if outside <= 0.0 {
        return 0.0;
    }
    let dist_m = outside * terrain.cell_size_m();
    let width = (3.0 * ground).clamp(450.0, 1_200.0);
    // A band only a few pixels wide reads as a pencil outline round the ice,
    // so the fringe appears only once it spans many pixels (about z 10 on).
    let resolved = 1.0 - smoothstep(60.0, 150.0, ground);
    resolved * (1.0 - smoothstep(0.0, width, dist_m))
}

/// Ice albedo by zone: snow above the dithered equilibrium line, bare and
/// blue ice with dirt below it, and a faint long undulation inside ice sheets.
fn ice_surface(
    terrain: &AtlasTerrain,
    p: [f64; 3],
    ground: f64,
    summer: f64,
    cells: [u32; 4],
    weights: [f64; 4],
) -> [f64; 3] {
    let zone = faded_octave(
        &terrain.ice_surface_octaves,
        0,
        p,
        ground,
        6_000.0,
        terrain.radius_m(),
    );
    let ela = -1.0 + 0.55 * zone;
    // About ±150 m either side of the −1 °C line, at 6.5 °C/km.
    let accum = 1.0 - smoothstep(ela - 1.0, ela + 1.0, summer);
    let snow = srgb_to_linear(SNOW_SRGB);
    let mut albedo = if accum > 0.98 {
        snow
    } else {
        let patch = faded_octave(
            &terrain.ice_surface_octaves,
            1,
            p,
            ground,
            900.0,
            terrain.radius_m(),
        );
        let blue = smoothstep(0.45, 0.72, patch);
        let dirt = smoothstep(0.50, 0.78, -patch);
        let mut ablation = lerp3(
            srgb_to_linear(BARE_ICE_SRGB),
            srgb_to_linear(BLUE_ICE_SRGB),
            blue * 0.80,
        );
        ablation = lerp3(ablation, srgb_to_linear(ICE_DIRT_SRGB), dirt * 0.65);
        lerp3(ablation, snow, accum)
    };
    // Thresholded noise lines (flow lines, crevasses) read as circuit-board
    // webs, not ice. Ice keeps only its zones, smooth surface and shading.
    let (sheet_w, interior) = ice_classes(terrain, cells, weights);
    if sheet_w > 0.60 && interior > 0.65 {
        albedo = sheet_undulation(terrain, albedo, p, ground);
    }
    albedo
}

/// Bilinear ice-sheet weight and interior (all-ice neighbourhood) weight.
fn ice_classes(terrain: &AtlasTerrain, cells: [u32; 4], weights: [f64; 4]) -> (f64, f64) {
    let mut sheet = 0.0;
    let mut interior = 0.0;
    for slot in 0..4 {
        let index = cells[slot] as usize;
        let weight = weights[slot];
        if Biome::from_u8(terrain.biome[index]) == Some(Biome::IceSheet) {
            sheet += weight;
        }
        if terrain.ice_interior[index] != 0 {
            interior += weight;
        }
    }
    (sheet, interior)
}

fn sheet_undulation(
    terrain: &AtlasTerrain,
    albedo: [f64; 3],
    p: [f64; 3],
    ground: f64,
) -> [f64; 3] {
    const LAMBDA: f64 = 12_000.0;
    let fade = octave_fade(LAMBDA, ground);
    if fade == 0.0 {
        return albedo;
    }
    let noise = sample_octave(
        &terrain.ice_sheet_octaves[0],
        p,
        terrain.radius_m() / LAMBDA,
    );
    scale3(albedo, 1.0 + 0.02 * fade * noise)
}

fn faded_octave(
    octaves: &[super::noise::Octave],
    index: usize,
    p: [f64; 3],
    ground: f64,
    lambda: f64,
    radius: f64,
) -> f64 {
    let Some(octave) = octaves.get(index) else {
        return 0.0;
    };
    let fade = octave_fade(lambda, ground);
    if fade == 0.0 {
        return 0.0;
    }
    fade * sample_octave(octave, p, radius / lambda)
}

fn stretch_sample(p: [f64; 3], along: [f64; 3], stretch: f64) -> [f64; 3] {
    let len = (along[0] * along[0] + along[1] * along[1] + along[2] * along[2]).sqrt();
    if len < 1e-8 || stretch <= 1.0 {
        return p;
    }
    let along = [along[0] / len, along[1] / len, along[2] / len];
    super::stretch_along(p, along, stretch)
}

/// Marginal ice zone plus leads. Fine octaves contribute nothing once they
/// fade, so a z2 interior is a smooth field of the bilinear perennial weight.
fn pack_structure(
    terrain: &AtlasTerrain,
    p: [f64; 3],
    ground: f64,
    perennial: f64,
) -> (f64, f64, f64) {
    let coarse = sea_noise(terrain, 0, p, ground, 32_000.0);
    let mid = sea_noise(terrain, 0, p, ground, 10_000.0);
    let thresh = if coarse.is_none() && mid.is_none() {
        0.35
    } else {
        (0.30 + 0.28 * coarse.unwrap_or(0.0) + 0.12 * mid.unwrap_or(0.0)).clamp(0.15, 0.72)
    };
    let solid = smoothstep(thresh, (thresh + 0.20).min(0.92), perennial);
    let floe_lambda = 2_500.0;
    let floe_fade = octave_fade(floe_lambda, ground);
    let floe = if floe_fade == 0.0 {
        0.0
    } else {
        sample_octave(
            &terrain.sea_ice_octaves[1],
            p,
            terrain.radius_m() / floe_lambda,
        )
    };
    let cover = if floe_fade == 0.0 {
        solid
    } else {
        let edge = 1.0 - smoothstep(thresh + 0.04, thresh + 0.42, perennial);
        let cut = 0.05 + 0.50 * edge;
        let pieces = smoothstep(cut, cut + 0.22, floe * 0.5 + 0.5);
        solid * ((1.0 - edge) + edge * pieces)
    };
    let lead = if cover < 0.2 {
        0.0
    } else {
        sea_leads(terrain, p, ground) * cover
    };
    let floe_n = if floe_fade == 0.0 {
        0.0
    } else {
        floe * floe_fade
    };
    (
        cover.clamp(0.0, 1.0),
        lead.clamp(0.0, 1.0),
        floe_n.clamp(-1.0, 1.0),
    )
}

/// `None` when the octave is smaller than the pixel.
fn sea_noise(
    terrain: &AtlasTerrain,
    index: usize,
    p: [f64; 3],
    ground: f64,
    lambda: f64,
) -> Option<f64> {
    let fade = octave_fade(lambda, ground);
    if fade == 0.0 {
        return None;
    }
    let octave = terrain.sea_ice_octaves.get(index)?;
    Some((fade * sample_octave(octave, p, terrain.radius_m() / lambda)).clamp(-1.0, 1.0))
}

fn sea_leads(terrain: &AtlasTerrain, p: [f64; 3], ground: f64) -> f64 {
    const LAMBDA: f64 = 6_000.0;
    let fade = octave_fade(LAMBDA, ground);
    if fade == 0.0 {
        return 0.0;
    }
    const AXES: [[f64; 3]; 3] = [[0.80, 0.15, 0.58], [-0.30, 0.92, 0.25], [0.20, -0.35, 0.91]];
    let mut lead = 0.0f64;
    for axis in AXES {
        let stretched = stretch_sample(p, axis, 9.0);
        let noise = sample_octave(
            &terrain.sea_ice_octaves[2],
            stretched,
            terrain.radius_m() / LAMBDA,
        );
        let line = smoothstep(0.86, 0.96, 1.0 - noise.abs());
        lead = lead.max(line);
    }
    lead * fade
}

/// Broad moisture patches and soft tree-clump mottling on open ground.
fn apply_patches(
    terrain: &AtlasTerrain,
    mut albedo: [f64; 3],
    p: [f64; 3],
    ground: f64,
    open_w: f64,
    veg: f64,
) -> [f64; 3] {
    let broad = faded_octave(
        &terrain.patch_octaves,
        0,
        p,
        ground,
        6_000.0,
        terrain.radius_m(),
    );
    if broad != 0.0 {
        albedo = scale3(albedo, 1.0 + 0.10 * broad.clamp(-1.0, 1.0) * open_w);
    }
    let speckle_lambda = 420.0;
    if octave_fade(speckle_lambda, ground) == 0.0 || veg < 0.2 {
        return albedo;
    }
    let speckle = sample_octave(
        &terrain.patch_octaves[1],
        p,
        terrain.radius_m() / speckle_lambda,
    );
    // Soft mottling, not a hard threshold: a thresholded speckle reads as a
    // camouflage pattern across the whole lowland.
    let clump = (0.5 + 0.5 * speckle).clamp(0.0, 1.0).powi(3)
        * open_w
        * octave_fade(speckle_lambda, ground);
    lerp3(albedo, srgb_to_linear(TREE_CLUMP_SRGB), clump * 0.30 * veg)
}

fn sun_facing_slope(dzdx: f64, dzdy: f64, sun: [f64; 3]) -> f64 {
    let slope = (dzdx * dzdx + dzdy * dzdy).sqrt();
    let sun_h = (sun[0] * sun[0] + sun[1] * sun[1]).sqrt();
    if slope < 1e-8 || sun_h < 1e-8 {
        return 0.0;
    }
    ((-dzdx * sun[0] - dzdy * sun[1]) / (slope * sun_h)).max(0.0)
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

/// Perennial snow from summer temperature.
///
/// `noise` in `[-1, 1]` dithers the line. Pole-facing slopes hold snow warmer.
/// Sun-facing slopes hold it colder and dither it further, so the cover is
/// patchy. Concave ground (`bowl > 0`) holds an extra degree. Slopes steeper
/// than about 0.8 shed it. With `bowl == 0` and `sun_facing == 0` the line
/// matches the previous pole-and-noise rule.
#[must_use]
pub(super) fn snow_fraction(
    temp_c: f64,
    slope: f64,
    pole_facing: f64,
    noise: f64,
    bowl: f64,
    sun_facing: f64,
) -> f64 {
    let gully = smoothstep(0.0, 0.12, bowl);
    let sun = sun_facing.clamp(0.0, 1.0);
    let dither = (0.45 + 0.65 * sun) * noise.clamp(-1.0, 1.0);
    let line = 1.5 * pole_facing.clamp(0.0, 1.0) - 1.6 * sun + 1.3 * gully + dither;
    let mut cover = 1.0 - smoothstep(line - 0.35, line + 0.35, temp_c);
    cover *= 1.0 - smoothstep(0.8, 1.05, slope);
    cover
}

/// Flat, unoccluded ground returns exactly `(1, 1, 1)`.
///
/// The ambient term is tinted by [`AMBIENT_TINT`] and then divided by the same
/// mix at `lambert == lambert_flat`, so a level surface stays on its albedo.
/// Shadowed snow therefore goes blue-grey. `ao_scale` fades the valley
/// occlusion; pass `1` for the full term.
#[must_use]
pub(super) fn imagery_light(
    dzdx: f64,
    dzdy: f64,
    exaggeration: f64,
    bowl: f64,
    sun: [f64; 3],
    ao_scale: f64,
) -> [f64; 3] {
    let nx = -dzdx * exaggeration;
    let ny = -dzdy * exaggeration;
    let nz = 1.0;
    let len = (nx * nx + ny * ny + nz * nz).sqrt();
    let lambert = ((nx * sun[0] + ny * sun[1] + nz * sun[2]) / len).max(0.0);
    let lambert_flat = sun[2].max(1e-6);
    let occlusion = 1.0 - 0.12 * smoothstep(0.0, 0.2, bowl) * ao_scale.clamp(0.0, 1.0);
    let sun_w = (1.0 - AMBIENT) * (lambert / lambert_flat) * occlusion;
    let amb_w = AMBIENT * occlusion;
    let mut light = [0.0; 3];
    for channel in 0..3 {
        let raw = amb_w * AMBIENT_TINT[channel] + sun_w;
        let flat = AMBIENT * AMBIENT_TINT[channel] + (1.0 - AMBIENT);
        light[channel] = raw / flat;
    }
    light
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
        tone_curve(colour[0]),
        tone_curve(colour[1]),
        tone_curve(colour[2]),
    ])
}

/// Filmic shoulder. Identity through 0.8 linear, then a C1 curve that
/// approaches 1 and never reaches it for a finite input.
#[must_use]
pub(super) fn tone_curve(x: f64) -> f64 {
    if x <= 0.0 {
        return 0.0;
    }
    const KNEE: f64 = 0.8;
    if x <= KNEE {
        return x;
    }
    let room = 1.0 - KNEE;
    let mapped = 1.0 - (room * room) / (room + (x - KNEE));
    if mapped >= 1.0 {
        1.0
    } else {
        mapped
    }
}

fn mul_light(albedo: [f64; 3], light: [f64; 3]) -> [f64; 3] {
    [
        albedo[0] * light[0],
        albedo[1] * light[1],
        albedo[2] * light[2],
    ]
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
        let light = imagery_light(0.0, 0.0, 4.0, 0.0, sun, 1.0);
        for channel in light {
            assert!(
                (channel - 1.0).abs() < 1e-12,
                "flat light {light:?} sun {sun:?}"
            );
        }
        let curved = imagery_light(0.0, 0.0, 1.0, 0.0, sun, 1.0);
        for channel in curved {
            assert!((channel - 1.0).abs() < 1e-12);
        }
        let albedo = [0.18, 0.22, 0.11];
        let encoded = encode(mul_light(albedo, light));
        assert_eq!(encoded, linear_to_srgb(albedo));
    }

    #[test]
    fn tone_curve_is_monotonic_and_identity_below_the_shoulder() {
        let mut previous = 0.0;
        for step in 0..=500 {
            let x = f64::from(step) * 0.01;
            let y = tone_curve(x);
            assert!(y <= 1.0, "x {x} mapped to {y}");
            assert!(y + 1e-12 >= previous, "x {x} fell from {previous} to {y}");
            if x <= 0.75 {
                assert!(
                    (y - x).abs() <= 1.0 / 255.0,
                    "x {x} moved by {}",
                    (y - x).abs()
                );
            }
            previous = y;
        }
        assert_eq!(tone_curve(-0.4), 0.0);
        let bright = tone_curve(1.4);
        assert!(bright < 1.0 && bright > 0.8, "shoulder {bright}");
        assert!(tone_curve(1.4) > tone_curve(0.9));
    }

    #[test]
    fn snow_line_uses_summer_temperature() {
        let equator = 0.0;
        let high = summer_temperature(0.0, equator, 0.0, 1_200.0);
        let low = summer_temperature(0.0, equator, 0.0, 200.0);
        assert!(
            snow_fraction(high, 0.05, 0.0, 0.0, 0.0, 0.0) > 0.95,
            "1200 m at the equator should be snow, T {high}"
        );
        assert!(
            snow_fraction(low, 0.05, 1.0, -1.0, 0.0, 0.0) < 0.05,
            "200 m at the equator should be clear, T {low}"
        );
        assert!(snow_fraction(high, 0.05, 1.0, -1.0, 0.0, 0.0) > 0.95);
        let shed = snow_fraction(high, 1.2, 0.0, 0.0, 0.0, 0.0);
        assert!(shed < 0.05, "a steep face sheds snow, got {shed}");
        let tundra_lat = 65.0_f64.to_radians();
        let tundra = summer_temperature(-8.0, tundra_lat, 200.0, 200.0);
        assert!(
            tundra > 2.0,
            "tundra summer should be above freezing, T {tundra}"
        );
        assert!(
            snow_fraction(tundra, 0.1, 1.0, -1.0, 0.0, 0.0) < 0.05,
            "tundra summer stays snow-free, T {tundra}"
        );
    }
}
