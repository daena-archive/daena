use daena_atlas::{
    render_from_source, request::AtlasRenderRequest, spike_identity_from_source, NoopProgress,
};
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
    let request = request.normalize().map_err(|error| error.to_string())?;
    let rendered = render_from_source(&source, &identity, &request, None, None, &mut NoopProgress)
        .map_err(|error| error.to_string())?;
    let elapsed_ms = started.elapsed().as_secs_f64() * 1000.0;
    if let Some(path) = output {
        fs::write(&path, &rendered.png).map_err(|error| error.to_string())?;
    }
    println!(
        "{{\"width\":{},\"height\":{},\"pngBytes\":{},\"rgbaBytes\":{},\"sourceBytes\":{},\"renderMs\":{:.3},\"sourceSha256\":\"{}\",\"identity\":\"{}\",\"rendererVersion\":{},\"offsetYears\":{},\"styleId\":\"{}\"}}",
        rendered.request.width_px,
        rendered.request.height_px,
        rendered.png.len(),
        rendered.rgba.len(),
        source.len(),
        elapsed_ms,
        rendered.provenance.source_sha256,
        rendered.provenance.physical_identity,
        rendered.provenance.renderer_version,
        rendered.provenance.offset_years,
        rendered.provenance.style_id,
    );
    Ok(())
}
