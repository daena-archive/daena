use daena_atlas::cache::AtlasDiskCache;
use daena_atlas::projection::AtlasProjection;
use daena_atlas::request::{AtlasFormat, AtlasRenderRequest};
use daena_atlas::{render_from_source_cached, spike_identity_from_source, NoopProgress};
use daena_physical::{
    decode_source, generate_world, GenerationSettings, NoopProgress as PhysicalNoop,
    DEFAULT_HEIGHT, DEFAULT_RADIUS_METRES, DEFAULT_WIDTH,
};
use std::env;
use std::fs;
use std::time::Instant;

fn main() -> Result<(), String> {
    let args = env::args().skip(1).collect::<Vec<_>>();
    let option = |name: &str| {
        args.windows(2)
            .find(|window| window[0] == name)
            .map(|window| window[1].clone())
    };
    let parse_u32 = |name: &str| {
        option(name)
            .map(|value| {
                value
                    .parse::<u32>()
                    .map_err(|error| format!("{name} must be an unsigned integer: {error}"))
            })
            .transpose()
    };
    let parse_f64 = |name: &str| {
        option(name)
            .map(|value| {
                value
                    .parse::<f64>()
                    .map_err(|error| format!("{name} must be a number: {error}"))
            })
            .transpose()
    };
    let width = parse_u32("--width")?.unwrap_or(2048);
    let height = parse_u32("--height")?.unwrap_or(1024);
    let output = option("--output");
    let source_path = option("--source");
    let started = Instant::now();
    let source = if let Some(path) = source_path {
        fs::read(path).map_err(|error| error.to_string())?
    } else {
        let settings = GenerationSettings {
            width: DEFAULT_WIDTH,
            height: DEFAULT_HEIGHT,
            radius_metres: DEFAULT_RADIUS_METRES,
            target_land_fraction_ppm: 300_000,
        };
        let world = generate_world(settings, 831_429, 0, &mut PhysicalNoop)
            .map_err(|error| error.to_string())?;
        decode_source(&world.source)?;
        world.source
    };
    let identity = spike_identity_from_source(&source);
    let mut request =
        AtlasRenderRequest::spike_png(width, height).map_err(|error| error.to_string())?;
    if let Some(offset) = option("--offset-years") {
        request.offset_years = offset
            .parse()
            .map_err(|error| format!("--offset-years must be an integer: {error}"))?;
    }
    if let Some(style) = option("--style") {
        request.style_id = style;
    }
    if let Some(format) = option("--format") {
        request.format = AtlasFormat::parse(&format).map_err(|error| error.to_string())?;
    }
    if let Some(projection) = option("--projection") {
        request.projection =
            AtlasProjection::parse(&projection).map_err(|error| error.to_string())?;
    }
    if let Some(dpi) = parse_u32("--dpi")? {
        request.dpi = dpi;
    }
    request.unlock_aspect = args.iter().any(|arg| arg == "--unlock-aspect");
    let degrees = |name: &str| -> Result<Option<i32>, String> {
        Ok(parse_f64(name)?.map(|value| (value * 1_000_000.0).round() as i32))
    };
    if let Some(west) = degrees("--west")? {
        request.extent.west_lon_micro = west;
    }
    if let Some(south) = degrees("--south")? {
        request.extent.south_lat_micro = south;
    }
    if let Some(east) = degrees("--east")? {
        request.extent.east_lon_micro = east;
    }
    if let Some(north) = degrees("--north")? {
        request.extent.north_lat_micro = north;
    }
    let request = request.normalize().map_err(|error| error.to_string())?;
    let cache = option("--cache-dir")
        .map(AtlasDiskCache::open)
        .transpose()
        .map_err(|error| error.to_string())?;
    let rendered = render_from_source_cached(
        &source,
        &identity,
        &request,
        None,
        None,
        &[],
        cache.as_ref(),
        &mut NoopProgress,
    )
    .map_err(|error| error.to_string())?;
    let elapsed_ms = started.elapsed().as_secs_f64() * 1000.0;
    if let Some(path) = output {
        fs::write(&path, &rendered.artifact).map_err(|error| error.to_string())?;
    }
    println!(
        "{{\"width\":{},\"height\":{},\"pngBytes\":{},\"artifactBytes\":{},\"rgbaBytes\":{},\"sourceBytes\":{},\"renderMs\":{:.3},\"sourceSha256\":\"{}\",\"identity\":\"{}\",\"rendererVersion\":{},\"offsetYears\":{},\"styleId\":\"{}\",\"format\":\"{}\",\"projection\":\"{}\",\"tributaryCount\":{},\"artifactCache\":\"{}\",\"residualCache\":\"{}\",\"drainageCache\":\"{}\"}}",
        rendered.request.width_px,
        rendered.request.height_px,
        rendered.png.len(),
        rendered.artifact.len(),
        rendered.rgba.len(),
        source.len(),
        elapsed_ms,
        rendered.provenance.source_sha256,
        rendered.provenance.physical_identity,
        rendered.provenance.renderer_version,
        rendered.provenance.offset_years,
        rendered.provenance.style_id,
        rendered.provenance.format,
        rendered.provenance.projection,
        rendered.tributary_count,
        rendered.artifact_cache.as_str(),
        rendered.residual_cache.as_str(),
        rendered.drainage_cache.as_str(),
    );
    Ok(())
}
