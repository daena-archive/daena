use daena_physical_spike::{
    decode_source, encode_source, generate_field, land_fraction, to_geojson, Grid, DEFAULT_HEIGHT,
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
    let grid = Grid::new(width, height, DEFAULT_RADIUS_METRES)?;
    let started = Instant::now();
    let field = generate_field(grid, 831_429, 0, 300_000)?;
    let generation_ms = started.elapsed().as_secs_f64() * 1000.0;
    let encode_started = Instant::now();
    let source = encode_source(&field)?;
    let encode_ms = encode_started.elapsed().as_secs_f64() * 1000.0;
    let geojson_started = Instant::now();
    let geojson = to_geojson(&field)?;
    let geojson_ms = geojson_started.elapsed().as_secs_f64() * 1000.0;
    decode_source(&source)?;
    if let Some(path) = source_path {
        fs::write(path, &source).map_err(|error| error.to_string())?;
    }
    if let Some(path) = emit_path {
        fs::write(path, geojson.as_bytes()).map_err(|error| error.to_string())?;
    }
    println!(
        "{{\"width\":{},\"height\":{},\"seaLevelMm\":{},\"landFraction\":{:.12},\"sourceBytes\":{},\"geojsonBytes\":{},\"geojsonFeatures\":{},\"generationMs\":{:.3},\"encodeMs\":{:.3},\"geojsonMs\":{:.3}}}",
        field.grid.width,
        field.grid.height,
        field.sea_level_mm,
        land_fraction(&field.grid, &field.elevations_mm, field.sea_level_mm),
        source.len(),
        geojson.len(),
        geojson.match_indices("\"type\":\"Feature\"").count(),
        generation_ms,
        encode_ms,
        geojson_ms,
    );
    Ok(())
}
