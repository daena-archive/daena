use daena_physical::{
    decode_source, generate_world, land_fraction, resolution, GenerationSettings, NoopProgress,
    DEFAULT_RADIUS_METRES, PRODUCTION_DEFAULT_HEIGHT, PRODUCTION_DEFAULT_WIDTH,
    PRODUCTION_MAX_HEIGHT, PRODUCTION_MAX_WIDTH,
};
use std::env;
use std::fs;
use std::time::Instant;

fn main() -> Result<(), String> {
    let args = env::args().skip(1).collect::<Vec<_>>();
    let option = |name: &str| {
        args.windows(2)
            .find(|window| window[0] == name)
            .map(|window| window[1].parse::<u32>())
            .transpose()
            .map_err(|error| format!("{name} must be an unsigned integer: {error}"))
    };
    let emit_path = args
        .windows(2)
        .find(|window| window[0] == "--geojson")
        .map(|window| window[1].clone());
    let source_path = args
        .windows(2)
        .find(|window| window[0] == "--source")
        .map(|window| window[1].clone());
    let (width, height) =
        if let (Some(width), Some(height)) = (option("--width")?, option("--height")?) {
            (width, height)
        } else if args.iter().any(|arg| arg == "--max") {
            (PRODUCTION_MAX_WIDTH, PRODUCTION_MAX_HEIGHT)
        } else {
            (PRODUCTION_DEFAULT_WIDTH, PRODUCTION_DEFAULT_HEIGHT)
        };
    if args.iter().any(|arg| arg == "--resolution-matrix") {
        for assessment in resolution::assess_all() {
            println!(
                "{{\"width\":{},\"height\":{},\"tier\":\"{:?}\",\"cellWidthMetres\":{},\"minimumFeatureSamples\":{},\"internalShapeSamples\":{},\"productionEligibleByFeatures\":{}}}",
                assessment.candidate.width,
                assessment.candidate.height,
                assessment.candidate.tier,
                assessment.max_cell_width_metres,
                assessment.minimum_feature_samples,
                assessment.internal_shape_samples,
                assessment.production_eligible(),
            );
        }
        return Ok(());
    }
    let started = Instant::now();
    let settings = GenerationSettings {
        width,
        height,
        radius_metres: DEFAULT_RADIUS_METRES,
        target_land_fraction_ppm: 300_000,
    };
    let seed = option("--seed")?.unwrap_or(831_429);
    let mut progress = NoopProgress;
    let world =
        generate_world(settings, seed, 0, &mut progress).map_err(|error| error.to_string())?;
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
        "{{\"width\":{},\"height\":{},\"seed\":{},\"seaLevelMm\":{},\"landFraction\":{:.12},\"sourceBytes\":{},\"geojsonBytes\":{},\"geojsonFeatures\":{},\"generationMs\":{:.3},\"geojsonMs\":{:.3}}}",
        field.grid.width,
        field.grid.height,
        seed,
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
