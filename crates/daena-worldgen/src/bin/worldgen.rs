//! Command line previews for the cube-sphere world generator.

use std::env;
use std::path::PathBuf;
use std::process::ExitCode;

use daena_worldgen::{
    generate, hillshade, render_equirect_with_shade, render_orthographic_with_shade, write_png,
    Biome, ContinentLayout, LakeKind, Layer, NoProgress, PlanetSettings, Preset, World, WorldError,
    WorldSettings,
};

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("{err}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse(env::args().skip(1))?;
    if args.list_presets {
        for preset in Preset::ALL {
            println!("{}\t{}", preset.as_str(), preset.description());
        }
        return Ok(());
    }
    if let Some(threads) = args.threads {
        rayon::ThreadPoolBuilder::new()
            .num_threads(threads)
            .build_global()?;
    }
    let base = match args.preset {
        Some(preset) => preset.settings(),
        None => WorldSettings::default(),
    };
    let settings = WorldSettings {
        seed: args.seed.unwrap_or(base.seed),
        resolution: args.resolution.unwrap_or(base.resolution),
        land_fraction: args.land_fraction.unwrap_or(base.land_fraction),
        plate_count: args.plate_count.unwrap_or(base.plate_count),
        continent_layout: args.layout.unwrap_or(base.continent_layout),
        tectonic_activity: args.activity.unwrap_or(base.tectonic_activity),
        planet: args.planet.apply(base.planet),
        river_min_m3s: args.river_min.unwrap_or(base.river_min_m3s),
        ..base
    };
    let mut progress = NoProgress;
    let world = generate(&settings, &mut progress)?;
    // `--hash` is accepted. The elevation hash is always part of the JSON line.
    let _ = args.hash;
    std::fs::create_dir_all(&args.out)?;
    if args.layers.is_empty() {
        println!("{}", report_json(&world, args.preset));
        return Ok(());
    }
    let need_shade = args.globe || args.layers.iter().any(|layer| layer.shaded());
    let shade = if need_shade {
        hillshade(&world)
    } else {
        Vec::new()
    };
    let height = args.width / 2;
    for layer in &args.layers {
        let rgb = render_equirect_with_shade(&world, *layer, args.width, height, &shade);
        let path = args.out.join(format!("{}.png", layer.name()));
        write_png(&path, args.width, height, &rgb)?;
    }
    if args.globe {
        let size = args.width / 2;
        let views = [
            ("globe-0.png", 20.0, 0.0),
            ("globe-1.png", 20.0, 90.0),
            ("globe-2.png", 20.0, 180.0),
            ("globe-3.png", 20.0, 270.0),
            ("globe-north.png", 90.0, 0.0),
            ("globe-south.png", -90.0, 0.0),
        ];
        for (name, lat, lon) in views {
            let rgb = render_orthographic_with_shade(&world, Layer::Relief, size, lat, lon, &shade);
            write_png(&args.out.join(name), size, size, &rgb)?;
        }
    }
    println!("{}", report_json(&world, args.preset));
    Ok(())
}

struct Args {
    preset: Option<Preset>,
    list_presets: bool,
    seed: Option<u64>,
    resolution: Option<u32>,
    land_fraction: Option<f64>,
    plate_count: Option<u32>,
    layout: Option<ContinentLayout>,
    activity: Option<f64>,
    out: PathBuf,
    width: u32,
    layers: Vec<Layer>,
    globe: bool,
    threads: Option<usize>,
    hash: bool,
    planet: PlanetOverrides,
    river_min: Option<f64>,
}

impl Args {
    fn parse(input: impl Iterator<Item = String>) -> Result<Self, WorldError> {
        let mut preset: Option<Preset> = None;
        let mut list_presets = false;
        let mut seed: Option<u64> = None;
        let mut resolution: Option<u32> = None;
        let mut land_fraction: Option<f64> = None;
        let mut plate_count: Option<u32> = None;
        let mut layout: Option<ContinentLayout> = None;
        let mut activity: Option<f64> = None;
        let mut out: Option<PathBuf> = None;
        let mut width = 2048u32;
        let mut layers: Option<Vec<Layer>> = None;
        let mut globe = false;
        let mut threads: Option<usize> = None;
        let mut hash = false;
        let mut planet = PlanetOverrides::default();
        let mut river_min: Option<f64> = None;
        let mut tokens: Vec<String> = input.collect();
        let mut index = 0;
        while index < tokens.len() {
            let arg = tokens[index].clone();
            let arg = arg.as_str();
            match arg {
                "--globe" => globe = true,
                "--hash" => hash = true,
                "--list-presets" => list_presets = true,
                "--preset" => {
                    if preset.is_some() {
                        return Err(WorldError::InvalidSettings(
                            "--preset given more than once".to_string(),
                        ));
                    }
                    let text = take_value(&mut tokens, &mut index, "--preset")?;
                    preset = Some(Preset::parse(&text).ok_or_else(|| {
                        WorldError::InvalidSettings(format!("unknown preset {text}"))
                    })?);
                }
                "--seed" => {
                    seed = Some(
                        take_value(&mut tokens, &mut index, "--seed")?
                            .parse()
                            .map_err(|_| {
                                WorldError::InvalidSettings("--seed is not a u64".to_string())
                            })?,
                    )
                }
                "--n" => {
                    resolution = Some(
                        take_value(&mut tokens, &mut index, "--n")?
                            .parse()
                            .map_err(|_| {
                                WorldError::InvalidSettings("--n is not a u32".to_string())
                            })?,
                    )
                }
                "--land" => {
                    land_fraction = Some(
                        take_value(&mut tokens, &mut index, "--land")?
                            .parse()
                            .map_err(|_| {
                                WorldError::InvalidSettings("--land is not a number".to_string())
                            })?,
                    )
                }
                "--plates" => {
                    plate_count = Some(
                        take_value(&mut tokens, &mut index, "--plates")?
                            .parse()
                            .map_err(|_| {
                                WorldError::InvalidSettings("--plates is not a u32".to_string())
                            })?,
                    )
                }
                "--layout" => {
                    let text = take_value(&mut tokens, &mut index, "--layout")?;
                    layout = Some(ContinentLayout::parse(&text).ok_or_else(|| {
                        WorldError::InvalidSettings(format!("unknown layout {text}"))
                    })?);
                }
                "--activity" => {
                    activity = Some(
                        take_value(&mut tokens, &mut index, "--activity")?
                            .parse()
                            .map_err(|_| {
                                WorldError::InvalidSettings(
                                    "--activity is not a number".to_string(),
                                )
                            })?,
                    )
                }
                "--out" => out = Some(PathBuf::from(take_value(&mut tokens, &mut index, "--out")?)),
                "--width" => {
                    width = take_value(&mut tokens, &mut index, "--width")?
                        .parse()
                        .map_err(|_| {
                            WorldError::InvalidSettings("--width is not a u32".to_string())
                        })?
                }
                "--layers" => {
                    let text = take_value(&mut tokens, &mut index, "--layers")?;
                    if text == "none" {
                        layers = Some(Vec::new());
                    } else {
                        let mut parsed = Vec::new();
                        for part in text.split(',') {
                            if part.is_empty() {
                                continue;
                            }
                            let layer = Layer::parse(part).ok_or_else(|| {
                                WorldError::InvalidSettings(format!("unknown layer {part}"))
                            })?;
                            parsed.push(layer);
                        }
                        if parsed.is_empty() {
                            return Err(WorldError::InvalidSettings(
                                "--layers did not name a layer".to_string(),
                            ));
                        }
                        layers = Some(parsed);
                    }
                }
                "--river-min" => {
                    river_min = Some(
                        take_value(&mut tokens, &mut index, "--river-min")?
                            .parse()
                            .map_err(|_| {
                                WorldError::InvalidSettings(
                                    "--river-min is not a number".to_string(),
                                )
                            })?,
                    );
                }
                "--threads" => {
                    let value: usize = take_value(&mut tokens, &mut index, "--threads")?
                        .parse()
                        .map_err(|_| {
                            WorldError::InvalidSettings("--threads is not a usize".to_string())
                        })?;
                    if value == 0 {
                        return Err(WorldError::InvalidSettings(
                            "--threads must be >= 1".to_string(),
                        ));
                    }
                    threads = Some(value);
                }
                other => {
                    if !take_planet_flag(other, &mut tokens, &mut index, &mut planet)? {
                        return Err(WorldError::InvalidSettings(format!(
                            "unknown argument {other}"
                        )));
                    }
                }
            }
            index += 1;
        }
        let out = match out {
            Some(out) => out,
            None if list_presets => PathBuf::new(),
            None => return Err(WorldError::InvalidSettings("--out is required".to_string())),
        };
        if width < 2 {
            return Err(WorldError::InvalidSettings(
                "--width must be >= 2".to_string(),
            ));
        }
        Ok(Self {
            preset,
            list_presets,
            seed,
            resolution,
            land_fraction,
            plate_count,
            layout,
            activity,
            out,
            width,
            layers: layers.unwrap_or_else(default_layers),
            globe,
            threads,
            hash,
            planet,
            river_min,
        })
    }
}

/// Planet flags as typed. A field that was not given keeps the base value.
#[derive(Default)]
struct PlanetOverrides {
    star_luminosity: Option<f64>,
    star_mass: Option<f64>,
    orbit_au: Option<f64>,
    eccentricity: Option<f64>,
    axial_tilt_deg: Option<f64>,
    rotation_hours: Option<f64>,
    greenhouse_wm2: Option<f64>,
    temperature_offset_c: Option<f64>,
    precipitation_scale: Option<f64>,
}

impl PlanetOverrides {
    /// Each flag that was given replaces its field of `base`.
    fn apply(&self, base: PlanetSettings) -> PlanetSettings {
        PlanetSettings {
            star_luminosity: self.star_luminosity.unwrap_or(base.star_luminosity),
            star_mass: self.star_mass.unwrap_or(base.star_mass),
            orbit_au: self.orbit_au.unwrap_or(base.orbit_au),
            eccentricity: self.eccentricity.unwrap_or(base.eccentricity),
            axial_tilt_deg: self.axial_tilt_deg.unwrap_or(base.axial_tilt_deg),
            rotation_hours: self.rotation_hours.unwrap_or(base.rotation_hours),
            greenhouse_wm2: self.greenhouse_wm2.unwrap_or(base.greenhouse_wm2),
            temperature_offset_c: self
                .temperature_offset_c
                .unwrap_or(base.temperature_offset_c),
            precipitation_scale: self.precipitation_scale.unwrap_or(base.precipitation_scale),
            ..base
        }
    }
}

fn take_planet_flag(
    flag: &str,
    tokens: &mut [String],
    index: &mut usize,
    planet: &mut PlanetOverrides,
) -> Result<bool, WorldError> {
    let parsed = match flag {
        "--luminosity" => Some(parse_f64(tokens, index, flag)?),
        "--star-mass" => Some(parse_f64(tokens, index, flag)?),
        "--orbit-au" => Some(parse_f64(tokens, index, flag)?),
        "--eccentricity" => Some(parse_f64(tokens, index, flag)?),
        "--tilt" => Some(parse_f64(tokens, index, flag)?),
        "--rotation-hours" => Some(parse_f64(tokens, index, flag)?),
        "--greenhouse" => Some(parse_f64(tokens, index, flag)?),
        "--temp-offset" => Some(parse_f64(tokens, index, flag)?),
        "--precip-scale" => Some(parse_f64(tokens, index, flag)?),
        _ => None,
    };
    let Some(value) = parsed else {
        return Ok(false);
    };
    match flag {
        "--luminosity" => planet.star_luminosity = Some(value),
        "--star-mass" => planet.star_mass = Some(value),
        "--orbit-au" => planet.orbit_au = Some(value),
        "--eccentricity" => planet.eccentricity = Some(value),
        "--tilt" => planet.axial_tilt_deg = Some(value),
        "--rotation-hours" => planet.rotation_hours = Some(value),
        "--greenhouse" => planet.greenhouse_wm2 = Some(value),
        "--temp-offset" => planet.temperature_offset_c = Some(value),
        "--precip-scale" => planet.precipitation_scale = Some(value),
        _ => {}
    }
    Ok(true)
}

fn parse_f64(tokens: &mut [String], index: &mut usize, flag: &str) -> Result<f64, WorldError> {
    take_value(tokens, index, flag)?
        .parse()
        .map_err(|_| WorldError::InvalidSettings(format!("{flag} is not a number")))
}

fn take_value(tokens: &mut [String], index: &mut usize, flag: &str) -> Result<String, WorldError> {
    let next = *index + 1;
    if next >= tokens.len() {
        return Err(WorldError::InvalidSettings(format!("{flag} needs a value")));
    }
    *index = next;
    Ok(std::mem::take(&mut tokens[next]))
}

fn default_layers() -> Vec<Layer> {
    vec![
        Layer::Relief,
        Layer::Plates,
        Layer::Crust,
        Layer::Uplift,
        Layer::Drainage,
        Layer::Base,
    ]
}

fn report_json(world: &World, preset: Option<Preset>) -> String {
    let mut out = String::new();
    out.push('{');
    out.push_str("\"settings\":{");
    push_u64(&mut out, "seed", world.settings.seed);
    out.push(',');
    push_u32(&mut out, "resolution", world.settings.resolution);
    out.push(',');
    push_f64(&mut out, "radius_m", world.settings.radius_m);
    out.push(',');
    push_f64(&mut out, "land_fraction", world.settings.land_fraction);
    out.push(',');
    push_u32(&mut out, "plate_count", world.settings.plate_count);
    out.push(',');
    push_str(
        &mut out,
        "continent_layout",
        world.settings.continent_layout.as_str(),
    );
    out.push(',');
    push_f64(
        &mut out,
        "tectonic_activity",
        world.settings.tectonic_activity,
    );
    out.push(',');
    match preset {
        Some(preset) => push_str(&mut out, "preset", preset.as_str()),
        None => push_null(&mut out, "preset"),
    }
    out.push(',');
    push_f64(&mut out, "river_min_m3s", world.settings.river_min_m3s);
    out.push(',');
    push_planet(&mut out, &world.settings.planet);
    out.push(',');
    out.push_str("\"erosion\":{");
    push_f64(&mut out, "m", world.settings.erosion.m);
    out.push(',');
    push_f64(&mut out, "k", world.settings.erosion.k);
    out.push(',');
    push_f64(&mut out, "s_c", world.settings.erosion.s_c);
    out.push(',');
    push_f64(&mut out, "kappa", world.settings.erosion.kappa);
    out.push(',');
    push_f64(
        &mut out,
        "max_elevation_m",
        world.settings.erosion.max_elevation_m,
    );
    out.push_str("}}");
    out.push(',');
    out.push_str("\"timings_ms\":{");
    for (i, (name, ms)) in world.timings.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        push_f64(&mut out, name, *ms);
    }
    out.push('}');
    out.push(',');
    push_u32(&mut out, "cell_count", world.grid.cell_count() as u32);
    out.push(',');
    push_f64(&mut out, "cell_size_km", world.grid.cell_size_m() / 1000.0);
    out.push(',');
    push_f64(
        &mut out,
        "achieved_land_fraction",
        world.stats.achieved_land_fraction,
    );
    out.push(',');
    push_f64(
        &mut out,
        "sea_level_shift_m",
        f64::from(world.sea_level_shift_m),
    );
    out.push(',');
    push_f64(
        &mut out,
        "elevation_min_m",
        f64::from(world.stats.elevation_min_m),
    );
    out.push(',');
    push_f64(
        &mut out,
        "elevation_max_m",
        f64::from(world.stats.elevation_max_m),
    );
    out.push(',');
    push_f64(&mut out, "land_p50_m", f64::from(world.stats.land_p50_m));
    out.push(',');
    push_f64(&mut out, "land_p90_m", f64::from(world.stats.land_p90_m));
    out.push(',');
    push_f64(&mut out, "land_p99_m", f64::from(world.stats.land_p99_m));
    out.push(',');
    push_u32(&mut out, "dry_depressions", world.stats.dry_depressions);
    out.push(',');
    push_u32(
        &mut out,
        "fragments_reassigned",
        world.tectonics.fragments_reassigned,
    );
    out.push(',');
    out.push_str("\"levels\":[");
    for (i, level) in world.stats.levels.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        out.push('{');
        push_u32(&mut out, "n", level.n);
        out.push(',');
        push_u32(&mut out, "iterations", level.iterations);
        out.push(',');
        push_u32(&mut out, "pits", level.pits);
        out.push(',');
        push_u32(&mut out, "pits_first", level.pits_first);
        out.push(',');
        push_u32(&mut out, "carved_basins", level.carved_basins);
        out.push(',');
        push_u32(&mut out, "edges_peak", level.edges_peak);
        out.push(',');
        push_f64(
            &mut out,
            "max_elevation_m",
            f64::from(level.max_elevation_m),
        );
        out.push(',');
        push_f64(
            &mut out,
            "mean_elevation_m",
            f64::from(level.mean_elevation_m),
        );
        out.push(',');
        push_u32(&mut out, "clamped_cells", level.clamped_cells);
        out.push(',');
        push_u32(&mut out, "closed_basins", level.closed_basins);
        out.push(',');
        push_u32(&mut out, "glaciated_cells", level.glaciated_cells);
        out.push(',');
        push_f64(&mut out, "wall_ms", level.wall_ms);
        out.push('}');
    }
    out.push(']');
    out.push(',');
    out.push_str("\"climate_passes\":[");
    for (i, pass) in world.climate_passes.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        out.push('{');
        push_str(&mut out, "stage", pass.label);
        out.push(',');
        push_u32(&mut out, "source_n", pass.source_n);
        out.push(',');
        push_u32(&mut out, "spinup_years", pass.spinup_years);
        out.push(',');
        push_bool(&mut out, "converged", pass.converged);
        out.push(',');
        push_f64(&mut out, "residual_k", pass.residual_k);
        out.push(',');
        push_f64(&mut out, "wall_ms", pass.wall_ms);
        out.push('}');
    }
    out.push(']');
    out.push(',');
    push_climate(&mut out, world);
    out.push(',');
    push_str(
        &mut out,
        "elevation_hash",
        &world.elevation_hash().to_string(),
    );
    out.push('}');
    out
}

fn push_climate(out: &mut String, world: &World) {
    let stats = &world.climate.stats;
    out.push_str("\"climate\":{");
    push_f64(out, "t_global_c", stats.t_global_c);
    out.push(',');
    push_f64(out, "t_equator_c", stats.t_equator_c);
    out.push(',');
    push_f64(out, "t_polar_c", stats.t_polar_c);
    out.push(',');
    push_f64(out, "p_global_mm", stats.p_global_mm);
    out.push(',');
    push_f64(out, "p_equator_mm", stats.p_equator_mm);
    out.push(',');
    push_f64(out, "p_subtropical_land_mm", stats.p_subtropical_land_mm);
    out.push(',');
    push_u32(out, "spinup_years", stats.spinup_years);
    out.push(',');
    push_bool(out, "spinup_converged", stats.spinup_converged);
    out.push(',');
    push_f64(out, "spinup_residual_k", stats.spinup_residual_k);
    out.push(',');
    push_f64(out, "phi_h_deg", stats.phi_h_deg);
    let rain = world.land_moisture();
    out.push(',');
    push_f64(out, "land_desert_fraction", rain.desert_fraction);
    out.push(',');
    push_f64(out, "p_land_mm", rain.p_land_mm);
    out.push(',');
    push_f64(out, "p_land_midlat_mm", rain.p_midlat_mm);
    out.push(',');
    push_f64(out, "p_ocean_equator_mm", rain.ocean_equator_mm);
    out.push(',');
    push_f64(out, "p_ocean_subtrop_mm", rain.ocean_subtrop_mm);
    out.push(',');
    push_f64(out, "p_ocean_midlat_mm", rain.ocean_midlat_mm);
    out.push_str("},");
    push_str(out, "climate_hash", &world.climate_hash().to_string());
    out.push(',');
    push_water(out, world);
    out.push(',');
    push_str(out, "hydrology_hash", &world.hydrology_hash().to_string());
    out.push(',');
    push_str(out, "biome_hash", &world.biome_hash().to_string());
}

fn push_water(out: &mut String, world: &World) {
    let hydro = &world.hydrology;
    let mut fresh = 0u32;
    let mut saline = 0u32;
    let mut flat = 0u32;
    let mut lake_area = 0.0;
    let mut largest = 0.0f64;
    let mut endorheic = 0.0;
    let mut lake_in = 0.0;
    for lake in &hydro.lakes {
        match lake.kind {
            LakeKind::Fresh => fresh += 1,
            LakeKind::Saline => saline += 1,
            LakeKind::SaltFlat => flat += 1,
        }
        lake_area += lake.area_m2;
        largest = largest.max(lake.area_m2);
        lake_in += lake.inflow_m3;
        if lake.kind != LakeKind::Fresh {
            endorheic += lake.inflow_m3;
        }
    }
    let mut runoff = 0.0;
    let mut glacier = 0.0;
    let mut sheet = 0.0;
    let mut perennial = 0.0;
    let mut seasonal = 0.0;
    let mut max_q = 0.0f64;
    for river in &hydro.rivers {
        max_q = max_q.max(river.max_discharge_m3s);
    }
    let mut land_area = 0.0;
    let mut land_class = [0.0; 26];
    for cell in 0..world.grid.cell_count() {
        let area = world.grid.area_m2(cell as u32);
        if world.ocean.get(cell).copied().unwrap_or(1) == 1 {
            let months = world.climate.sea_ice_months.get(cell).copied().unwrap_or(0);
            if months >= 11 {
                perennial += area;
            } else if (4..=10).contains(&months) {
                seasonal += area;
            }
        } else {
            runoff += f64::from(
                world
                    .climate
                    .runoff_mm
                    .get(cell)
                    .copied()
                    .unwrap_or(0.0)
                    .max(0.0),
            ) / 1000.0
                * area;
            land_area += area;
            if let Some(class) = world.biome.get(cell).copied().and_then(Biome::from_u8) {
                if class.is_land_class() {
                    land_class[class as usize] += area;
                }
            }
        }
        match hydro.ice_cover.get(cell).copied().unwrap_or(0) {
            1 => glacier += area,
            2 => sheet += area,
            _ => {}
        }
    }
    let share = if runoff > 0.0 {
        endorheic / runoff
    } else if lake_in > 0.0 {
        endorheic / lake_in
    } else {
        0.0
    };
    out.push_str("\"hydrology\":{");
    push_u32(out, "lakes_fresh", fresh);
    out.push(',');
    push_u32(out, "lakes_saline", saline);
    out.push(',');
    push_u32(out, "lakes_salt_flat", flat);
    out.push(',');
    push_f64(out, "lake_area_km2", lake_area / 1.0e6);
    out.push(',');
    push_f64(out, "largest_lake_km2", largest / 1.0e6);
    out.push(',');
    push_f64(out, "endorheic_inflow_share", share);
    out.push(',');
    push_u32(out, "river_segments", hydro.rivers.len() as u32);
    out.push(',');
    push_f64(
        out,
        "river_length_km",
        hydro.river_length_m(&world.grid) / 1000.0,
    );
    out.push(',');
    push_f64(out, "max_discharge_m3s", max_q);
    out.push_str("},\"ice\":{");
    push_f64(out, "glacier_km2", glacier / 1.0e6);
    out.push(',');
    push_f64(out, "ice_sheet_km2", sheet / 1.0e6);
    out.push(',');
    push_f64(out, "sea_ice_perennial_km2", perennial / 1.0e6);
    out.push(',');
    push_f64(out, "sea_ice_seasonal_km2", seasonal / 1.0e6);
    out.push_str("},\"biomes\":{");
    let mut first = true;
    for id in 0..26u8 {
        let Some(class) = Biome::from_u8(id) else {
            continue;
        };
        if !class.is_land_class() {
            continue;
        }
        if !first {
            out.push(',');
        }
        first = false;
        let fraction = if land_area > 0.0 {
            land_class[id as usize] / land_area
        } else {
            0.0
        };
        push_f64(out, class.name(), fraction);
    }
    out.push('}');
}

fn push_planet(out: &mut String, planet: &PlanetSettings) {
    out.push_str("\"planet\":{");
    push_f64(out, "star_luminosity", planet.star_luminosity);
    out.push(',');
    push_f64(out, "star_mass", planet.star_mass);
    out.push(',');
    push_f64(out, "orbit_au", planet.orbit_au);
    out.push(',');
    push_f64(out, "eccentricity", planet.eccentricity);
    out.push(',');
    push_f64(out, "perihelion_deg", planet.perihelion_deg);
    out.push(',');
    push_f64(out, "axial_tilt_deg", planet.axial_tilt_deg);
    out.push(',');
    push_f64(out, "rotation_hours", planet.rotation_hours);
    out.push(',');
    push_f64(out, "greenhouse_wm2", planet.greenhouse_wm2);
    out.push(',');
    push_f64(out, "temperature_offset_c", planet.temperature_offset_c);
    out.push(',');
    push_f64(out, "precipitation_scale", planet.precipitation_scale);
    out.push('}');
}

fn push_null(out: &mut String, key: &str) {
    out.push('"');
    out.push_str(key);
    out.push_str("\":null");
}

fn push_str(out: &mut String, key: &str, value: &str) {
    out.push('"');
    out.push_str(key);
    out.push_str("\":\"");
    out.push_str(value);
    out.push('"');
}

fn push_u64(out: &mut String, key: &str, value: u64) {
    out.push('"');
    out.push_str(key);
    out.push_str("\":");
    out.push_str(&value.to_string());
}

fn push_bool(out: &mut String, key: &str, value: bool) {
    out.push('"');
    out.push_str(key);
    out.push_str("\":");
    out.push_str(if value { "true" } else { "false" });
}

fn push_u32(out: &mut String, key: &str, value: u32) {
    push_u64(out, key, u64::from(value));
}

fn push_f64(out: &mut String, key: &str, value: f64) {
    out.push('"');
    out.push_str(key);
    out.push_str("\":");
    if value.is_finite() {
        out.push_str(&format!("{value:.6}"));
    } else {
        out.push_str("null");
    }
}
