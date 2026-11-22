//! Relief tiles around one site, plus a zoom-2 world image.

use std::env;
use std::fs;
use std::io::BufWriter;
use std::path::Path;
use std::time::Instant;

use daena_atlas::style::{self, load_style, AtlasStyle};
use daena_atlas::terrain::{
    ground_metres_per_pixel, lon_lat_to_tile, render_imagery_pixels, render_pixels, AtlasTerrain,
};
use daena_physical::products::Biome;

fn main() -> Result<(), String> {
    let args = env::args().skip(1).collect::<Vec<_>>();
    let option = |name: &str| -> Option<String> {
        args.windows(2)
            .find(|window| window[0] == name)
            .map(|window| window[1].clone())
    };
    let source_path = option("--source").ok_or_else(|| "--source is required".to_string())?;
    let out_dir = option("--out").ok_or_else(|| "--out is required".to_string())?;
    let zooms_text = option("--zooms").ok_or_else(|| "--zooms is required".to_string())?;
    let scale = match option("--scale") {
        None => 1u32,
        Some(value) => {
            let scale = value
                .parse::<u32>()
                .map_err(|_| "--scale must be 1 or 2".to_string())?;
            if scale != 1 && scale != 2 {
                return Err("--scale must be 1 or 2".to_string());
            }
            scale
        }
    };
    let offset = match option("--offset-years") {
        None => None,
        Some(value) => Some(
            value
                .parse::<i64>()
                .map_err(|error| format!("--offset-years must be an integer: {error}"))?,
        ),
    };
    let zooms = parse_zooms(&zooms_text)?;
    let style_name = option("--style").unwrap_or_else(|| "imagery".to_string());
    if style_name != "relief" && style_name != "imagery" {
        return Err("--style must be relief or imagery".to_string());
    }
    let source = fs::read(&source_path).map_err(|error| error.to_string())?;
    let terrain = AtlasTerrain::from_source(&source, offset).map_err(|error| error.to_string())?;
    let (lon, lat) = site(&terrain, &option)?;
    let relief = if style_name == "relief" {
        Some(
            load_style(style::RELIEF_STYLE_ID)
                .map_err(|error| error.to_string())?
                .0,
        )
    } else {
        None
    };
    fs::create_dir_all(&out_dir).map_err(|error| error.to_string())?;

    for z in zooms {
        let (tile_x, tile_y) = lon_lat_to_tile(lon, lat, z);
        let n = 1u32 << z;
        let tile_px = 256 * scale;
        let stitched = (tile_px as usize) * 3;
        let mut image = vec![0u8; stitched * stitched * 4];
        let mut times = Vec::with_capacity(9);
        for dy in -1i32..=1 {
            for dx in -1i32..=1 {
                let x = (tile_x as i32 + dx).rem_euclid(n as i32) as u32;
                let y = (tile_y as i32 + dy).clamp(0, n as i32 - 1) as u32;
                let started = Instant::now();
                let tile = render_view(
                    &terrain,
                    relief.as_ref(),
                    z,
                    x * 256,
                    y * 256,
                    256,
                    256,
                    scale,
                );
                times.push(started.elapsed().as_secs_f64() * 1000.0);
                blit(
                    &mut image,
                    stitched,
                    &tile,
                    tile_px as usize,
                    ((dx + 1) as usize) * tile_px as usize,
                    ((dy + 1) as usize) * tile_px as usize,
                );
            }
        }
        write_png(
            &Path::new(&out_dir).join(format!("z{z}.png")),
            stitched as u32,
            stitched as u32,
            &image,
        )?;
        times.sort_by(f64::total_cmp);
        let median = times[4];
        let worst = times[8];
        let ground = ground_metres_per_pixel(z, lat.to_radians(), terrain.radius_m());
        let line = serde_json::json!({
            "lon": lon,
            "lat": lat,
            "z": z,
            "x": tile_x,
            "y": tile_y,
            "octaves": terrain.octave_count(ground),
            "medianMs": median,
            "worstMs": worst,
        });
        println!("{line}");
    }

    let world_px = 1024 * scale;
    let world = render_view(&terrain, relief.as_ref(), 2, 0, 0, 1024, 1024, scale);
    write_png(
        &Path::new(&out_dir).join("whole.png"),
        world_px,
        world_px,
        &world,
    )?;
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn render_view(
    terrain: &AtlasTerrain,
    relief: Option<&AtlasStyle>,
    z: u32,
    gx: u32,
    gy: u32,
    width: u32,
    height: u32,
    scale: u32,
) -> Vec<u8> {
    match relief {
        Some(style) => render_pixels(terrain, style, z, gx, gy, width, height, scale),
        None => render_imagery_pixels(terrain, z, gx, gy, width, height, scale),
    }
}

fn site(
    terrain: &AtlasTerrain,
    option: &impl Fn(&str) -> Option<String>,
) -> Result<(f64, f64), String> {
    let site = option("--site");
    let lon = option("--lon");
    let lat = option("--lat");
    if site.as_deref() == Some("auto") {
        if lon.is_some() || lat.is_some() {
            return Err("--site auto does not take --lon or --lat".to_string());
        }
        let cell = terrain.auto_site_cell();
        let (lat, lon) = terrain.surface().cell_lat_lon_degrees(cell);
        return Ok((lon, lat));
    }
    if let Some(name) = site.as_deref() {
        let biome_name = name
            .strip_prefix("biome:")
            .ok_or("--site must be auto, biome:<name>, or omit it for --lon/--lat")?;
        let biome = Biome::ALL
            .into_iter()
            .find(|biome| biome.name() == biome_name)
            .ok_or_else(|| format!("unknown biome {biome_name}"))?;
        if lon.is_some() || lat.is_some() {
            return Err("--site biome does not take --lon or --lat".to_string());
        }
        let cell = terrain
            .biome_site_cell(biome as u8)
            .ok_or_else(|| format!("world has no land cell of biome {biome_name}"))?;
        let (lat, lon) = terrain.surface().cell_lat_lon_degrees(cell);
        return Ok((lon, lat));
    }
    let lon = lon.ok_or_else(|| "pass --site auto or both --lon and --lat".to_string())?;
    let lat = lat.ok_or_else(|| "pass --site auto or both --lon and --lat".to_string())?;
    let lon = lon
        .parse::<f64>()
        .map_err(|_| "--lon must be a number".to_string())?;
    let lat = lat
        .parse::<f64>()
        .map_err(|_| "--lat must be a number".to_string())?;
    Ok((lon, lat))
}

fn parse_zooms(text: &str) -> Result<Vec<u32>, String> {
    let mut zooms = Vec::new();
    for part in text.split(',') {
        if part.is_empty() {
            return Err("--zooms must be a comma-separated list".to_string());
        }
        let zoom = part
            .parse::<u32>()
            .map_err(|_| format!("--zooms has a bad entry: {part}"))?;
        if zoom > 22 {
            return Err(format!("--zooms entry {zoom} is above 22"));
        }
        zooms.push(zoom);
    }
    if zooms.is_empty() {
        return Err("--zooms must list at least one zoom".to_string());
    }
    Ok(zooms)
}

fn blit(dest: &mut [u8], dest_w: usize, src: &[u8], src_w: usize, x: usize, y: usize) {
    let row_bytes = src_w * 4;
    for row in 0..src_w {
        let from = row * row_bytes;
        let to = ((y + row) * dest_w + x) * 4;
        dest[to..to + row_bytes].copy_from_slice(&src[from..from + row_bytes]);
    }
}

fn write_png(path: &Path, width: u32, height: u32, rgba: &[u8]) -> Result<(), String> {
    let file = fs::File::create(path).map_err(|error| error.to_string())?;
    let mut encoder = png::Encoder::new(BufWriter::new(file), width, height);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header().map_err(|error| error.to_string())?;
    writer
        .write_image_data(rgba)
        .map_err(|error| error.to_string())?;
    Ok(())
}
