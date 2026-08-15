use daena_physical_spike::{
    decode_source, generate_world, land_fraction, GenerationSettings, NoopProgress, DEFAULT_HEIGHT,
    DEFAULT_RADIUS_METRES, DEFAULT_WIDTH, MAX_HEIGHT, MAX_WIDTH,
};
use std::env;
use std::fs;
use std::time::Instant;

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
    let (width, height) = if args.iter().any(|arg| arg == "--max") {
        (MAX_WIDTH, MAX_HEIGHT)
    } else {
        (DEFAULT_WIDTH, DEFAULT_HEIGHT)
    };
    let started = Instant::now();
    let settings = GenerationSettings {
        width,
        height,
        radius_metres: DEFAULT_RADIUS_METRES,
        target_land_fraction_ppm: 300_000,
    };
    let mut progress = NoopProgress;
    let world =
        generate_world(settings, 831_429, 0, &mut progress).map_err(|error| error.to_string())?;
    let generation_ms = started.elapsed().as_secs_f64() * 1000.0;
    let field = &world.field;
    let source = &world.source;
    let geojson_started = Instant::now();
    let geojson = world.derived_geojson.clone();
    let geojson_ms = geojson_started.elapsed().as_secs_f64() * 1000.0;
    decode_source(source)?;
    if let Some(path) = source_path {
        fs::write(path, source).map_err(|error| error.to_string())?;
    }
    if let Some(path) = emit_path {
        fs::write(path, geojson.as_bytes()).map_err(|error| error.to_string())?;
    }
    println!(
        "{{\"width\":{},\"height\":{},\"seaLevelMm\":{},\"landFraction\":{:.12},\"sourceBytes\":{},\"geojsonBytes\":{},\"geojsonFeatures\":{},\"generationMs\":{:.3},\"geojsonMs\":{:.3}}}",
        field.grid.width,
        field.grid.height,
        field.sea_level_mm,
        land_fraction(&field.grid, &field.elevations_mm, field.sea_level_mm),
        source.len(),
        geojson.len(),
        geojson.match_indices("\"type\":\"Feature\"").count(),
        generation_ms,
        geojson_ms,
    );
    Ok(())
}
