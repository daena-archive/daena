use daena_physical::{engine, land_fraction, NoopProgress, DEFAULT_RADIUS_METRES};
use std::env;
use std::fs;
use std::time::Instant;

fn option<T: std::str::FromStr>(args: &[String], name: &str) -> Result<Option<T>, String> {
    args.windows(2)
        .find(|window| window[0] == name)
        .map(|window| {
            window[1]
                .parse::<T>()
                .map_err(|_| format!("{name} has an invalid value: {}", window[1]))
        })
        .transpose()
}

fn main() -> Result<(), String> {
    let args = env::args().skip(1).collect::<Vec<_>>();
    let emit_path = args
        .windows(2)
        .find(|window| window[0] == "--geojson")
        .map(|window| window[1].clone());
    let source_path = args
        .windows(2)
        .find(|window| window[0] == "--source")
        .map(|window| window[1].clone());
    let preset_name = args
        .windows(2)
        .find(|window| window[0] == "--preset")
        .map(|window| window[1].clone())
        .unwrap_or_else(|| "earthlike".into());
    let preset = engine::WorldPreset::parse(&preset_name)
        .ok_or_else(|| format!("unknown world preset {preset_name}"))?;
    let resolution: u32 = option(&args, "--resolution")?.unwrap_or(64);
    let seed: u64 = option(&args, "--seed")?.unwrap_or(831_429);
    let settings = engine::Settings::from_parts(
        seed,
        preset,
        resolution,
        DEFAULT_RADIUS_METRES,
        0,
        daena_physical::planetary::PlanetaryConfiguration::earth_like(),
    );
    let started = Instant::now();
    let mut progress = NoopProgress;
    let products = engine::generate(settings, &mut progress).map_err(|error| error.to_string())?;
    let generation_ms = started.elapsed().as_secs_f64() * 1000.0;
    let source = products.source();
    let view = products.view();
    let geojson = products.derived_geojson();
    if let Some(path) = source_path {
        fs::write(path, source).map_err(|error| error.to_string())?;
    }
    if let Some(path) = emit_path {
        fs::write(path, geojson.as_bytes()).map_err(|error| error.to_string())?;
    }
    println!(
        "{{\"resolution\":{},\"width\":{},\"height\":{},\"seed\":{},\"preset\":\"{}\",\"seaLevelMm\":{},\"landFraction\":{:.12},\"sourceBytes\":{},\"geojsonBytes\":{},\"geojsonFeatures\":{},\"generationMs\":{:.3}}}",
        products.resolution(),
        view.grid.width,
        view.grid.height,
        products.seed(),
        preset.as_str(),
        view.sea_level_mm,
        land_fraction(&view.grid, &view.elevations_mm, view.sea_level_mm),
        source.len(),
        geojson.len(),
        geojson.match_indices("\"type\":\"Feature\"").count(),
        generation_ms,
    );
    Ok(())
}
