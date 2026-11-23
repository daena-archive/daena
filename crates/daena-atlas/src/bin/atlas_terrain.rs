//! Relief tiles around one site, plus a zoom-2 world image.

use std::env;
use std::fs;
use std::io::BufWriter;
use std::path::Path;
use std::time::Instant;

use daena_atlas::style::{self, load_style, AtlasStyle};
use daena_atlas::terrain::{
    ground_metres_per_pixel, lon_lat_to_tile, render_imagery_pixels, render_pixels,
    render_terrain_vector_tile, AtlasTerrain, TerrainVectorLayer, TerrainVectors,
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
    let vector_layers = match option("--vectors") {
        None => Vec::new(),
        Some(text) => parse_vectors(&text)?,
    };
    let source = fs::read(&source_path).map_err(|error| error.to_string())?;
    let terrain = AtlasTerrain::from_source(&source, offset).map_err(|error| error.to_string())?;
    let (lon, lat) = site(&terrain, &option)?;
    println!(
        "{}",
        serde_json::json!({
            "riverGeometryBytes": terrain.river_geometry_bytes(),
            "riverCount": terrain.river_count(),
        })
    );
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
    let prepared = if vector_layers.is_empty() {
        None
    } else {
        Some(TerrainVectors::new(&terrain))
    };

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
        if let Some(prepared) = &prepared {
            for dy in -1i32..=1 {
                for dx in -1i32..=1 {
                    let x = (tile_x as i32 + dx).rem_euclid(n as i32) as u32;
                    let y = (tile_y as i32 + dy).clamp(0, n as i32 - 1) as u32;
                    paint_vectors(
                        &mut image,
                        stitched,
                        &terrain,
                        prepared,
                        &vector_layers,
                        z,
                        x,
                        y,
                        ((dx + 1) as usize) * tile_px as usize,
                        ((dy + 1) as usize) * tile_px as usize,
                        scale,
                        relief.is_some(),
                    )?;
                }
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
    let mut world = render_view(&terrain, relief.as_ref(), 2, 0, 0, 1024, 1024, scale);
    if let Some(prepared) = &prepared {
        let tiles = 1024 / 256;
        for ty in 0..tiles {
            for tx in 0..tiles {
                paint_vectors(
                    &mut world,
                    world_px as usize,
                    &terrain,
                    prepared,
                    &vector_layers,
                    2,
                    tx,
                    ty,
                    tx as usize * 256 * scale as usize,
                    ty as usize * 256 * scale as usize,
                    scale,
                    relief.is_some(),
                )?;
            }
        }
    }
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
        if name == "river" || name.starts_with("river:") {
            if lon.is_some() || lat.is_some() {
                return Err("--site river does not take --lon or --lat".to_string());
            }
            let order = if name == "river" {
                None
            } else {
                let text = &name["river:".len()..];
                if text.is_empty() {
                    return Err("--site river:<order> needs an integer order".to_string());
                }
                Some(
                    text.parse::<u16>()
                        .map_err(|_| "--site river:<order> needs an integer order".to_string())?,
                )
            };
            return terrain.river_site(order).ok_or_else(|| match order {
                None => "world has no river of order 4 or higher".to_string(),
                Some(order) => format!("world has no river of order {order}"),
            });
        }
        let biome_name = name.strip_prefix("biome:").ok_or(
            "--site must be auto, biome:<name>, river, river:<order>, or omit it for --lon/--lat",
        )?;
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

fn parse_vectors(text: &str) -> Result<Vec<TerrainVectorLayer>, String> {
    let mut layers = Vec::new();
    for part in text.split(',') {
        if part.is_empty() {
            return Err("--vectors entries must be layer ids".to_string());
        }
        let layer = TerrainVectorLayer::from_id(part)
            .ok_or_else(|| format!("unknown vector layer {part}"))?;
        if !layers.contains(&layer) {
            layers.push(layer);
        }
    }
    if layers.is_empty() {
        return Err("--vectors must list a layer".to_string());
    }
    Ok(layers)
}

#[allow(clippy::too_many_arguments)]
fn paint_vectors(
    image: &mut [u8],
    image_w: usize,
    terrain: &AtlasTerrain,
    vectors: &TerrainVectors,
    layers: &[TerrainVectorLayer],
    z: u32,
    tile_x: u32,
    tile_y: u32,
    origin_x: usize,
    origin_y: usize,
    scale: u32,
    relief: bool,
) -> Result<(), String> {
    for layer in layers {
        let bytes = render_terrain_vector_tile(terrain, vectors, *layer, z, tile_x, tile_y)
            .map_err(|error| error.to_string())?;
        let value: serde_json::Value =
            serde_json::from_slice(&bytes).map_err(|error| error.to_string())?;
        let Some(features) = value["features"].as_array() else {
            continue;
        };
        let (rgb, alpha) = layer_paint(*layer, relief);
        for feature in features {
            let Some(coords) = feature["geometry"]["coordinates"].as_array() else {
                continue;
            };
            let width = stroke_width(feature, *layer, z, scale, terrain.radius_m());
            let mut previous: Option<[f64; 2]> = None;
            for coord in coords {
                let lon = coord[0].as_f64().unwrap_or(0.0);
                let lat = coord[1].as_f64().unwrap_or(0.0);
                let point = lonlat_to_image(lon, lat, z, tile_x, tile_y, origin_x, origin_y, scale);
                if let Some(start) = previous {
                    stroke(image, image_w, start, point, rgb, alpha, width);
                }
                previous = Some(point);
            }
        }
    }
    Ok(())
}

fn layer_paint(layer: TerrainVectorLayer, relief: bool) -> ([u8; 3], f64) {
    let rgb = if relief {
        [0x3b, 0x6e, 0xa5]
    } else {
        [0x2e, 0x58, 0x68]
    };
    match layer {
        TerrainVectorLayer::Streams => (rgb, 0.7),
        TerrainVectorLayer::Rivers => (rgb, 1.0),
        _ => ([0x1c, 0x24, 0x2a], 1.0),
    }
}

fn stroke_width(
    feature: &serde_json::Value,
    layer: TerrainVectorLayer,
    z: u32,
    scale: u32,
    radius_m: f64,
) -> f64 {
    let props = &feature["properties"];
    let order = props["order"]
        .as_u64()
        .or_else(|| props["strahler_order"].as_u64())
        .unwrap_or(1) as u16;
    let zoom_px = match layer {
        TerrainVectorLayer::Streams => stream_width_px(order, z),
        TerrainVectorLayer::Rivers => {
            let styled = river_width_px(order, z);
            let metres = props["channel_width_m"].as_f64().unwrap_or(0.0);
            let lat = feature["geometry"]["coordinates"][0][1]
                .as_f64()
                .unwrap_or(0.0);
            let ground = ground_metres_per_pixel(z, lat.to_radians(), radius_m).max(1.0);
            styled.max(metres / ground)
        }
        _ => 1.0,
    };
    zoom_px * f64::from(scale.max(1))
}

/// Same ramp as `riverWidthPx` in `terrain-vectors.ts`.
fn river_width_px(order: u16, zoom: u32) -> f64 {
    let t = ((f64::from(order) - 1.0) / 4.0).clamp(0.0, 1.0);
    let at = |major: f64| 0.35 + t * (major - 0.35);
    let u = ((f64::from(zoom) - 5.0) / 7.0).clamp(0.0, 1.0);
    at(1.0) + (at(3.0) - at(1.0)) * u
}

/// Same ramp as `streamWidthPx` in `terrain-vectors.ts`.
fn stream_width_px(order: u16, zoom: u32) -> f64 {
    let t = ((f64::from(order) - 1.0) / 4.0).clamp(0.0, 1.0);
    let base = 0.5 + t * 0.7;
    let clamped = f64::from(zoom).clamp(8.0, 12.0);
    base * (1.0 + 0.06 * (clamped - 8.0))
}

#[allow(clippy::too_many_arguments)]
fn lonlat_to_image(
    lon: f64,
    lat: f64,
    z: u32,
    tile_x: u32,
    tile_y: u32,
    origin_x: usize,
    origin_y: usize,
    scale: u32,
) -> [f64; 2] {
    let world = 256.0 * (1u64 << z.min(30)) as f64;
    let mut gx = (lon + 180.0) / 360.0 * world;
    let tile_gx = f64::from(tile_x) * 256.0;
    if gx - tile_gx > world * 0.5 {
        gx -= world;
    } else if tile_gx - gx > world * 0.5 {
        gx += world;
    }
    let lat = lat.clamp(-85.051_128_78, 85.051_128_78);
    let mercator_y = lat.to_radians().tan().asinh();
    let gy = (std::f64::consts::PI - mercator_y) / std::f64::consts::TAU * world;
    let tile_gy = f64::from(tile_y) * 256.0;
    let scale_f = f64::from(scale.max(1));
    [
        (gx - tile_gx) * scale_f + origin_x as f64,
        (gy - tile_gy) * scale_f + origin_y as f64,
    ]
}

fn stroke(
    image: &mut [u8],
    image_w: usize,
    start: [f64; 2],
    end: [f64; 2],
    rgb: [u8; 3],
    alpha: f64,
    width_px: f64,
) {
    let image_h = image.len() / (image_w * 4);
    let pad = width_px * 0.5 + 1.5;
    let min_x = (start[0].min(end[0]) - pad).floor().max(0.0) as i32;
    let max_x = (start[0].max(end[0]) + pad)
        .ceil()
        .min(image_w as f64 - 1.0) as i32;
    let min_y = (start[1].min(end[1]) - pad).floor().max(0.0) as i32;
    let max_y = (start[1].max(end[1]) + pad)
        .ceil()
        .min(image_h as f64 - 1.0) as i32;
    if max_x < min_x || max_y < min_y {
        return;
    }
    for y in min_y..=max_y {
        for x in min_x..=max_x {
            let dist = point_segment([x as f64 + 0.5, y as f64 + 0.5], start, end);
            let coverage = (width_px * 0.5 + 0.5 - dist).clamp(0.0, 1.0);
            if coverage <= 0.0 {
                continue;
            }
            blend(
                &mut image[((y as usize) * image_w + x as usize) * 4..],
                rgb,
                alpha * coverage,
            );
        }
    }
}

fn blend(pixel: &mut [u8], rgb: [u8; 3], alpha: f64) {
    let alpha = alpha.clamp(0.0, 1.0);
    if alpha <= 0.0 {
        return;
    }
    for channel in 0..3 {
        let src = f64::from(rgb[channel]);
        let dst = f64::from(pixel[channel]);
        pixel[channel] = (src * alpha + dst * (1.0 - alpha)).round() as u8;
    }
    pixel[3] = 255;
}

fn point_segment(point: [f64; 2], start: [f64; 2], end: [f64; 2]) -> f64 {
    let dx = end[0] - start[0];
    let dy = end[1] - start[1];
    let len2 = dx * dx + dy * dy;
    let t = if len2 <= 1e-12 {
        0.0
    } else {
        ((point[0] - start[0]) * dx + (point[1] - start[1]) * dy) / len2
    };
    let t = t.clamp(0.0, 1.0);
    let px = start[0] + t * dx - point[0];
    let py = start[1] + t * dy - point[1];
    (px * px + py * py).sqrt()
}
