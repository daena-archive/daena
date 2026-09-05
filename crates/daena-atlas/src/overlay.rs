//! Vector overlays, graticule, frame, and style-only paper grain.

use daena_physical::hydrology::HydrologyField;
use daena_physical::tectonics::TectonicWorld;
use daena_physical::Grid;

use serde::{Deserialize, Serialize};

use crate::detail::domain_key;
use crate::detail::lattice_sample;
use crate::drainage::DerivedTributary;
use crate::projection::{
    bilinear_i32, clamp_lat_micro, wrap_lon_micro, ProjectedView, LAT_MICRO_MIN, LAT_MICRO_SPAN,
    LON_MICRO_MIN, LON_MICRO_SPAN,
};
use crate::request::AtlasRenderRequest;
use crate::style::AtlasStyle;

/// 0.1° interpolated paper cells. Independent per-pixel hashes look like TV static.
const PAPER_CELL_MICRO: i64 = 100_000;

const FRAME_PX: u32 = 8;
const FLOW_ARROW_MIN_MILLI: f32 = 80.0;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthoredFeature {
    pub id: String,
    pub layer_id: String,
    pub kind: String,
    pub label: Option<String>,
    pub path: Vec<[i32; 2]>,
}

#[must_use]
pub fn lon_to_x(lon_micro: i32, width: u32) -> i32 {
    crate::projection::ProjectedView {
        projection: crate::projection::AtlasProjection::Equirectangular,
        extent: crate::projection::AtlasExtent::world(),
        width,
        height: 1,
    }
    .project(lon_micro, 0)
    .map_or(0, |(x, _)| x)
}

#[must_use]
pub fn lat_to_y(lat_micro: i32, height: u32) -> i32 {
    crate::projection::ProjectedView {
        projection: crate::projection::AtlasProjection::Equirectangular,
        extent: crate::projection::AtlasExtent::world(),
        width: 1,
        height,
    }
    .project(0, lat_micro)
    .map_or(0, |(_, y)| y)
}

fn project_point(view: ProjectedView, lon_micro: i32, lat_micro: i32) -> Option<(i32, i32)> {
    view.project(lon_micro, lat_micro)
}

fn mix_channel(first: u8, second: u8, t: f32) -> u8 {
    let t = t.clamp(0.0, 1.0);
    (f32::from(first) + (f32::from(second) - f32::from(first)) * t).round() as u8
}

pub(crate) fn wind_tint(east_milli: i32, north_milli: i32) -> [u8; 3] {
    let speed = (east_milli as f32).hypot(north_milli as f32).max(1.0);
    let u = east_milli as f32 / speed;
    let v = north_milli as f32 / speed;
    let easterly = [72_u8, 168, 232];
    let westerly = [232_u8, 156, 48];
    let northward = [64_u8, 196, 168];
    let southward = [196_u8, 96, 168];
    let zonal = [
        mix_channel(easterly[0], westerly[0], (u + 1.0) / 2.0),
        mix_channel(easterly[1], westerly[1], (u + 1.0) / 2.0),
        mix_channel(easterly[2], westerly[2], (u + 1.0) / 2.0),
    ];
    let meridional = if v >= 0.0 { northward } else { southward };
    let t = v.abs() * 0.7;
    [
        mix_channel(zonal[0], meridional[0], t),
        mix_channel(zonal[1], meridional[1], t),
        mix_channel(zonal[2], meridional[2], t),
    ]
}

fn arrow_step(size: u32) -> u32 {
    (size / 16).max(4).min(size.max(1))
}

pub(crate) fn current_tint(east_milli: i32, north_milli: i32) -> [u8; 3] {
    let speed = (east_milli as f32).hypot(north_milli as f32).max(1.0);
    let u = east_milli as f32 / speed;
    let v = north_milli as f32 / speed;
    let westward = [72_u8, 112, 220];
    let eastward = [48_u8, 196, 196];
    let northward = [232_u8, 148, 64];
    let southward = [64_u8, 188, 168];
    let zonal = [
        mix_channel(westward[0], eastward[0], (u + 1.0) / 2.0),
        mix_channel(westward[1], eastward[1], (u + 1.0) / 2.0),
        mix_channel(westward[2], eastward[2], (u + 1.0) / 2.0),
    ];
    let meridional = if v >= 0.0 { northward } else { southward };
    let t = v.abs() * 0.7;
    [
        mix_channel(zonal[0], meridional[0], t),
        mix_channel(zonal[1], meridional[1], t),
        mix_channel(zonal[2], meridional[2], t),
    ]
}

pub(crate) fn draw_wind_arrows(
    buffer: &mut [u8],
    view: ProjectedView,
    grid: Grid,
    east: &[i32],
    north: &[i32],
) {
    draw_flow_arrows(buffer, view, grid, east, north, wind_tint);
}

pub(crate) fn draw_current_arrows(
    buffer: &mut [u8],
    view: ProjectedView,
    grid: Grid,
    east: &[i32],
    north: &[i32],
) {
    draw_flow_arrows(buffer, view, grid, east, north, current_tint);
}

fn draw_flow_arrows(
    buffer: &mut [u8],
    view: ProjectedView,
    grid: Grid,
    east: &[i32],
    north: &[i32],
    tint: fn(i32, i32) -> [u8; 3],
) {
    if east.len() != grid.sample_count() || north.len() != grid.sample_count() {
        return;
    }
    if view.width == 0 || view.height == 0 {
        return;
    }
    let step_x = arrow_step(view.width);
    let step_y = arrow_step(view.height);
    let mut y = step_y / 2;
    while y < view.height {
        let mut x = step_x / 2;
        while x < view.width {
            let jitter = x
                .wrapping_mul(0x9e37_79b9)
                .wrapping_add(y.wrapping_mul(0x85eb_ca6b));
            let ox = ((jitter % 7) as i32) - 3;
            let oy = (((jitter / 7) % 7) as i32) - 3;
            let px = (x as i32 + ox).clamp(0, view.width.saturating_sub(1) as i32) as u32;
            let py = (y as i32 + oy).clamp(0, view.height.saturating_sub(1) as i32) as u32;
            let (lon, lat) = view.pixel_center(px, py);
            let east_milli = crate::detail::sample_field_mm(grid, east, lon, lat);
            let north_milli = crate::detail::sample_field_mm(grid, north, lon, lat);
            let speed = (east_milli as f32).hypot(north_milli as f32);
            if speed >= FLOW_ARROW_MIN_MILLI {
                let length = 8.0 + (speed / 450.0).min(10.0);
                let angle = (-north_milli as f32).atan2(east_milli as f32);
                let ax = px as i32;
                let ay = py as i32;
                let dx = (angle.cos() * length).round() as i32;
                let dy = (angle.sin() * length).round() as i32;
                let rgb = tint(east_milli, north_milli);
                let alpha = 480_000 + ((speed / 2_000.0).min(1.0) * 440_000.0) as u32;
                draw_segment(
                    buffer,
                    view.width,
                    view.height,
                    ax - dx,
                    ay - dy,
                    ax + dx,
                    ay + dy,
                    rgb,
                    alpha,
                );
                let left = angle - 0.45;
                let right = angle + 0.45;
                draw_segment(
                    buffer,
                    view.width,
                    view.height,
                    ax + dx,
                    ay + dy,
                    ax + dx - (left.cos() * 4.5).round() as i32,
                    ay + dy - (left.sin() * 4.5).round() as i32,
                    rgb,
                    alpha,
                );
                draw_segment(
                    buffer,
                    view.width,
                    view.height,
                    ax + dx,
                    ay + dy,
                    ax + dx - (right.cos() * 4.5).round() as i32,
                    ay + dy - (right.sin() * 4.5).round() as i32,
                    rgb,
                    alpha,
                );
            }
            x = x.saturating_add(step_x);
            if step_x == 0 {
                break;
            }
        }
        y = y.saturating_add(step_y);
        if step_y == 0 {
            break;
        }
    }
}

fn cell_center(grid: Grid, cell: usize) -> [i32; 2] {
    if cell >= grid.sample_count() {
        return [0, 0];
    }
    let (row, col) = grid.row_col(cell);
    [
        (-180_000_000i64 + 360_000_000i64 * (i64::from(col) * 2 + 1) / i64::from(grid.width * 2))
            as i32,
        (-90_000_000i64 + 180_000_000i64 * (i64::from(row) * 2 + 1) / i64::from(grid.height * 2))
            as i32,
    ]
}

pub(crate) fn put_pixel(
    buffer: &mut [u8],
    width: u32,
    height: u32,
    x: i32,
    y: i32,
    rgb: [u8; 3],
    alpha_ppm: u32,
) {
    if x < 0 || y < 0 || x >= width as i32 || y >= height as i32 {
        return;
    }
    let offset = (y as usize * width as usize + x as usize) * 4;
    let t = alpha_ppm.min(1_000_000);
    let inv = 1_000_000 - t;
    buffer[offset] = ((u32::from(buffer[offset]) * inv + u32::from(rgb[0]) * t) / 1_000_000) as u8;
    buffer[offset + 1] =
        ((u32::from(buffer[offset + 1]) * inv + u32::from(rgb[1]) * t) / 1_000_000) as u8;
    buffer[offset + 2] =
        ((u32::from(buffer[offset + 2]) * inv + u32::from(rgb[2]) * t) / 1_000_000) as u8;
}

#[allow(clippy::too_many_arguments)]
fn draw_segment(
    buffer: &mut [u8],
    width: u32,
    height: u32,
    mut x0: i32,
    mut y0: i32,
    x1: i32,
    y1: i32,
    rgb: [u8; 3],
    alpha_ppm: u32,
) {
    let dx = (x1 - x0).abs();
    let sx = if x0 < x1 { 1 } else { -1 };
    let dy = -(y1 - y0).abs();
    let sy = if y0 < y1 { 1 } else { -1 };
    let mut err = dx + dy;
    loop {
        put_pixel(buffer, width, height, x0, y0, rgb, alpha_ppm);
        if x0 == x1 && y0 == y1 {
            break;
        }
        let e2 = err * 2;
        if e2 >= dy {
            err += dy;
            x0 += sx;
        }
        if e2 <= dx {
            err += dx;
            y0 += sy;
        }
    }
}

fn draw_geodesic_segment(
    buffer: &mut [u8],
    view: ProjectedView,
    a: [i32; 2],
    b: [i32; 2],
    rgb: [u8; 3],
    alpha_ppm: u32,
) {
    let Some((x0, y0)) = project_point(view, a[0], a[1]) else {
        return;
    };
    let Some((x1, y1)) = project_point(view, b[0], b[1]) else {
        return;
    };
    let width = view.width;
    let height = view.height;
    if (x1 - x0).abs() > width as i32 / 2 {
        let edge = if x0 < x1 { 0 } else { width as i32 };
        let other = if x0 < x1 { width as i32 } else { 0 };
        draw_segment(buffer, width, height, x0, y0, edge, y0, rgb, alpha_ppm);
        draw_segment(buffer, width, height, other, y1, x1, y1, rgb, alpha_ppm);
        return;
    }
    draw_segment(buffer, width, height, x0, y0, x1, y1, rgb, alpha_ppm);
}

#[allow(clippy::too_many_arguments)]
pub fn composite_overlays(
    buffer: &mut [u8],
    request: &AtlasRenderRequest,
    style: &AtlasStyle,
    hydrology: &HydrologyField,
    tectonics: &TectonicWorld,
    identity: &[u8],
    overlays: &[AuthoredFeature],
    tributaries: &[DerivedTributary],
    winds: Option<(&[i32], &[i32])>,
    currents: Option<(&[i32], &[i32])>,
) {
    let view = request.view().unwrap_or(ProjectedView {
        projection: request.projection,
        extent: request.extent,
        width: request.width_px,
        height: request.height_px,
    });
    let width = view.width;
    let height = view.height;
    if request.layer_enabled("rivers") {
        for path in &hydrology.river_coordinates {
            for window in path.windows(2) {
                draw_geodesic_segment(buffer, view, window[0], window[1], style.river, 850_000);
            }
        }
        for tributary in tributaries {
            for window in tributary.path.windows(2) {
                draw_geodesic_segment(buffer, view, window[0], window[1], style.river, 550_000);
            }
        }
    }
    // Physical-grid coastline polylines are the simulation-cell edges.
    // Renderer 11 inks the reconstructed shore in `pixel_rgba` instead.
    if request.layer_enabled("contours") {
        for segment in &hydrology.bathymetry_contours {
            draw_geodesic_segment(
                buffer,
                view,
                segment.first,
                segment.second,
                style.contour,
                450_000,
            );
        }
    }
    if request.layer_enabled("watersheds") {
        for polygons in &hydrology.watershed_polygons {
            for ring in polygons {
                for window in ring.windows(2) {
                    draw_geodesic_segment(
                        buffer,
                        view,
                        window[0],
                        window[1],
                        style.contour,
                        280_000,
                    );
                }
            }
        }
    }
    if request.layer_enabled("tectonic-plates") || request.layer_enabled("tectonic-boundaries") {
        let alpha = if request.layer_enabled("tectonic-boundaries") {
            850_000
        } else {
            420_000
        };
        for boundary in &tectonics.boundaries {
            draw_geodesic_segment(
                buffer,
                view,
                cell_center(tectonics.grid, boundary.first_cell),
                cell_center(tectonics.grid, boundary.second_cell),
                style.political,
                alpha,
            );
        }
    }
    if request.layer_enabled("volcanic-centers") {
        for center in &tectonics.volcanic_centers {
            let point = cell_center(tectonics.grid, center.cell);
            if let Some((x, y)) = project_point(view, point[0], point[1]) {
                for dx in -1..=1 {
                    for dy in -1..=1 {
                        put_pixel(
                            buffer,
                            width,
                            height,
                            x + dx,
                            y + dy,
                            style.political,
                            1_000_000,
                        );
                    }
                }
            }
        }
    }
    if request.layer_enabled("winds") {
        if let Some((east, north)) = winds {
            draw_wind_arrows(buffer, view, hydrology.grid, east, north);
        }
    }
    if request.layer_enabled("currents") {
        if let Some((east, north)) = currents {
            draw_current_arrows(buffer, view, hydrology.grid, east, north);
        }
    }
    if request.layer_enabled("graticule") {
        draw_graticule(buffer, view, style.graticule);
    }
    let mut omitted_labels = 0_u32;
    for feature in overlays {
        if !request.layer_enabled(&feature.layer_id) && feature.kind != "semantic" {
            continue;
        }
        if feature.kind == "semantic"
            && !request
                .active_layer_ids
                .iter()
                .any(|id| id == &feature.layer_id)
        {
            continue;
        }
        let color = if feature.kind == "semantic" {
            style.label_ink
        } else {
            style.political
        };
        match feature.path.len() {
            0 => {}
            1 => {
                if let Some((x, y)) = project_point(view, feature.path[0][0], feature.path[0][1]) {
                    put_pixel(buffer, width, height, x, y, color, 1_000_000);
                }
            }
            _ => {
                for window in feature.path.windows(2) {
                    draw_geodesic_segment(buffer, view, window[0], window[1], color, 900_000);
                }
            }
        }
    }
    let mut derived_labels = Vec::new();
    if request.layer_enabled("labels") {
        for tributary in tributaries.iter().take(32) {
            if let Some(first) = tributary.path.first() {
                derived_labels.push(AuthoredFeature {
                    id: tributary.id.clone(),
                    layer_id: "labels".into(),
                    kind: "derived-tributary".into(),
                    label: Some(format!("T{}", tributary.source_cell)),
                    path: vec![*first],
                });
            }
        }
    }
    let mut label_source = overlays.to_vec();
    label_source.extend(derived_labels);
    if request.layer_enabled("labels") {
        omitted_labels = crate::labels::draw_labels(buffer, view, &label_source, style);
    }
    let _ = omitted_labels;
    if request.layer_enabled("frame") {
        for x in 0..width {
            for y in 0..FRAME_PX.min(height) {
                put_pixel(
                    buffer,
                    width,
                    height,
                    x as i32,
                    y as i32,
                    style.frame,
                    1_000_000,
                );
                put_pixel(
                    buffer,
                    width,
                    height,
                    x as i32,
                    height as i32 - 1 - y as i32,
                    style.frame,
                    1_000_000,
                );
            }
        }
        for y in 0..height {
            for x in 0..FRAME_PX.min(width) {
                put_pixel(
                    buffer,
                    width,
                    height,
                    x as i32,
                    y as i32,
                    style.frame,
                    1_000_000,
                );
                put_pixel(
                    buffer,
                    width,
                    height,
                    width as i32 - 1 - x as i32,
                    y as i32,
                    style.frame,
                    1_000_000,
                );
            }
        }
    }
    if style.paper_grain_ppm > 0 {
        apply_paper_grain(buffer, request, style, identity);
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OverlayHit {
    pub id: String,
    pub layer_id: String,
    pub kind: String,
    pub label: Option<String>,
    pub derived: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AtlasSurfaceSample {
    pub lon_micro: i32,
    pub lat_micro: i32,
    pub elevation_mm: i32,
    pub water_surface_mm: i32,
    pub temperature_centi_c: i32,
    pub temperature_nh_summer_centi_c: i32,
    pub temperature_nh_winter_centi_c: i32,
    pub seasonal_range_centi_c: u32,
    pub freeze: String,
    pub wind_east_milli: i32,
    pub wind_north_milli: i32,
    pub wind_east_nh_summer_milli: i32,
    pub wind_north_nh_summer_milli: i32,
    pub wind_east_nh_winter_milli: i32,
    pub wind_north_nh_winter_milli: i32,
    pub wind_divergence_ppm: i32,
    pub wind_divergence_nh_summer_ppm: i32,
    pub wind_divergence_nh_winter_ppm: i32,
    pub wind_band: String,
    pub wind_band_nh_summer: String,
    pub wind_band_nh_winter: String,
    pub current_east_milli: i32,
    pub current_north_milli: i32,
    pub precipitation_mm: i32,
    pub humidity_ppm: i32,
    pub aridity_ppm: i32,
    pub precipitation_nh_summer_mm: i32,
    pub precipitation_nh_winter_mm: i32,
    pub climate: String,
    pub biome_reason: String,
    pub surface: String,
    pub ice_thickness_mm: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AtlasInspectResult {
    pub hits: Vec<OverlayHit>,
    pub surface: AtlasSurfaceSample,
}

#[must_use]
pub fn hit_test_features(
    overlays: &[AuthoredFeature],
    lon_micro: i32,
    lat_micro: i32,
    radius_micro: i32,
    limit: usize,
) -> Vec<OverlayHit> {
    let radius = i64::from(radius_micro.max(1));
    let radius2 = radius.saturating_mul(radius);
    let mut hits = Vec::new();
    for feature in overlays {
        if hits.len() >= limit {
            break;
        }
        if feature_hits(feature, lon_micro, lat_micro, radius2) {
            hits.push(OverlayHit {
                id: feature.id.clone(),
                layer_id: feature.layer_id.clone(),
                kind: feature.kind.clone(),
                label: feature.label.clone(),
                derived: feature.kind == "derived-tributary" || feature.layer_id == "labels",
            });
        }
    }
    hits
}

fn feature_hits(feature: &AuthoredFeature, lon: i32, lat: i32, radius2: i64) -> bool {
    if feature.path.is_empty() {
        return false;
    }
    if feature
        .path
        .iter()
        .any(|point| distance2(point[0], point[1], lon, lat) <= radius2)
    {
        return true;
    }
    feature
        .path
        .windows(2)
        .any(|window| segment_hits(window[0], window[1], lon, lat, radius2))
}

fn wrap_dlon(a: i32, b: i32) -> i64 {
    let mut delta = i64::from(a) - i64::from(b);
    if delta > 180_000_000 {
        delta -= 360_000_000;
    } else if delta < -180_000_000 {
        delta += 360_000_000;
    }
    delta
}

fn distance2(lon_a: i32, lat_a: i32, lon_b: i32, lat_b: i32) -> i64 {
    let dlon = wrap_dlon(lon_a, lon_b);
    let dlat = i64::from(lat_a) - i64::from(lat_b);
    dlon.saturating_mul(dlon)
        .saturating_add(dlat.saturating_mul(dlat))
}

fn segment_hits(a: [i32; 2], b: [i32; 2], lon: i32, lat: i32, radius2: i64) -> bool {
    let dx = wrap_dlon(b[0], a[0]);
    let dy = i64::from(b[1]) - i64::from(a[1]);
    let len2 = dx.saturating_mul(dx).saturating_add(dy.saturating_mul(dy));
    if len2 == 0 {
        return distance2(a[0], a[1], lon, lat) <= radius2;
    }
    let px = wrap_dlon(lon, a[0]);
    let py = i64::from(lat) - i64::from(a[1]);
    let mut t = (px.saturating_mul(dx).saturating_add(py.saturating_mul(dy))) * 1_000 / len2;
    t = t.clamp(0, 1_000);
    let qx = i64::from(a[0]) + dx * t / 1_000;
    let qy = i64::from(a[1]) + dy * t / 1_000;
    let qlon = crate::projection::wrap_lon_micro(qx);
    distance2(qlon, qy as i32, lon, lat) <= radius2
}

fn draw_graticule(buffer: &mut [u8], view: ProjectedView, rgb: [u8; 3]) {
    let width = view.width;
    let height = view.height;
    let mid_lat = view.extent.south_lat_micro + (view.extent.lat_span_micro() / 2) as i32;
    let mid_lon = view.extent.central_meridian_micro();
    for lon in (-180..=180).step_by(30) {
        let Some((x, _)) = project_point(view, lon * 1_000_000, mid_lat) else {
            continue;
        };
        draw_segment(
            buffer,
            width,
            height,
            x,
            0,
            x,
            height as i32 - 1,
            rgb,
            280_000,
        );
    }
    for lat in (-90..=90).step_by(30) {
        let Some((_, y)) = project_point(view, mid_lon, lat * 1_000_000) else {
            continue;
        };
        draw_segment(
            buffer,
            width,
            height,
            0,
            y,
            width as i32 - 1,
            y,
            rgb,
            280_000,
        );
    }
}

fn paper_unit_ppm(
    key: &[u8; 32],
    lon_micro: i32,
    lat_micro: i32,
    octave: u32,
    cell_micro: i64,
) -> i32 {
    let cell_micro = cell_micro.max(1);
    let lon_cells = (LON_MICRO_SPAN / cell_micro).max(1) as u32;
    let lat_cells = (LAT_MICRO_SPAN / cell_micro).max(1) as u32;
    let lon_pos = (i64::from(wrap_lon_micro(i64::from(lon_micro))) - i64::from(LON_MICRO_MIN))
        .rem_euclid(LON_MICRO_SPAN);
    let lat_pos = (i64::from(clamp_lat_micro(i64::from(lat_micro))) - i64::from(LAT_MICRO_MIN))
        .clamp(0, LAT_MICRO_SPAN - 1);
    let i = (lon_pos / cell_micro) as u32 % lon_cells;
    let j = ((lat_pos / cell_micro) as u32).min(lat_cells.saturating_sub(1));
    let ni = (i + 1) % lon_cells;
    let nj = (j + 1).min(lat_cells.saturating_sub(1));
    let fx = ((lon_pos.rem_euclid(cell_micro)) * 1_000_000 / cell_micro) as u32;
    let fy = ((lat_pos.rem_euclid(cell_micro)) * 1_000_000 / cell_micro) as u32;
    let corner = |ii: u32, jj: u32| -> i32 {
        ((lattice_sample(key, ii, jj, octave) >> 11) % 1_000_001) as i32
    };
    bilinear_i32(
        corner(i, j),
        corner(ni, j),
        corner(i, nj),
        corner(ni, nj),
        fx,
        fy,
    )
}

fn paper_grain_delta(key: &[u8; 32], lon_micro: i32, lat_micro: i32, strength_ppm: u32) -> i32 {
    let fine = paper_unit_ppm(key, lon_micro, lat_micro, 0, PAPER_CELL_MICRO);
    let coarse = paper_unit_ppm(key, lon_micro, lat_micro, 1, PAPER_CELL_MICRO * 4);
    let unit = (fine * 6 + coarse * 4) / 10;
    let signed = i64::from(unit) - 500_000;
    (signed * i64::from(strength_ppm) * 255 / 1_000_000 / 1_000_000) as i32
}

fn apply_paper_grain(
    buffer: &mut [u8],
    request: &AtlasRenderRequest,
    style: &AtlasStyle,
    identity: &[u8],
) {
    let view = request.view().unwrap_or(ProjectedView {
        projection: request.projection,
        extent: request.extent,
        width: request.width_px,
        height: request.height_px,
    });
    let key = domain_key(
        identity,
        1,
        request.variant,
        &format!("paper-grain\0{}\0{}", style.id, style.version),
    );
    let width = request.width_px;
    let height = request.height_px;
    let strength = style.paper_grain_ppm;
    for y in 0..height {
        for x in 0..width {
            let (lon, lat) = view.pixel_center(x, y);
            let delta = paper_grain_delta(&key, lon, lat, strength);
            let offset = (y as usize * width as usize + x as usize) * 4;
            for channel in 0..3 {
                let value = i32::from(buffer[offset + channel]) + delta;
                buffer[offset + channel] = value.clamp(0, 255) as u8;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::detail::domain_key;

    #[test]
    fn antique_paper_grain_stays_within_8bit_ppm() {
        let key = domain_key(b"fixture", 1, 0, "paper-grain\0daena-atlas-antique\0\x31");
        let strength = 90_000;
        let mut max_abs = 0i32;
        let mut nearby_jump = 0i32;
        for lon in (-150_000_000..=150_000_000).step_by(7_000_000) {
            for lat in (-70_000_000..=70_000_000).step_by(7_000_000) {
                let delta = paper_grain_delta(&key, lon, lat, strength);
                max_abs = max_abs.max(delta.abs());
                let neighbor = paper_grain_delta(&key, lon.saturating_add(20_000), lat, strength);
                nearby_jump = nearby_jump.max((delta - neighbor).abs());
            }
        }
        assert!(max_abs <= 12, "grain clipped into static: max {max_abs}");
        assert!(
            nearby_jump <= 4,
            "grain is per-sample hash, not paper: jump {nearby_jump}"
        );
    }

    #[test]
    fn easterly_wind_tint_is_blue() {
        let rgb = wind_tint(-1_500, 0);
        assert!(rgb[2] > rgb[0], "easterly should lean blue: {rgb:?}");
        let westerly = wind_tint(1_500, 0);
        assert!(
            westerly[0] > westerly[2],
            "westerly should lean amber: {westerly:?}"
        );
    }

    #[test]
    fn poleward_current_tint_is_warm() {
        let rgb = current_tint(0, 1_500);
        assert!(
            rgb[0] > rgb[2],
            "northward current should lean amber: {rgb:?}"
        );
    }

    #[test]
    fn wind_arrows_paint_zoomed_views() {
        let grid = daena_physical::Grid::new(8, 4, 6_371_000).unwrap();
        let east = vec![-1_200; grid.sample_count()];
        let north = vec![0; grid.sample_count()];
        let view = ProjectedView {
            projection: crate::projection::AtlasProjection::Equirectangular,
            extent: crate::projection::AtlasExtent {
                west_lon_micro: 12_000_000,
                south_lat_micro: 8_000_000,
                east_lon_micro: 18_000_000,
                north_lat_micro: 12_000_000,
            },
            width: 64,
            height: 48,
        };
        let mut buffer = vec![0_u8; 64 * 48 * 4];
        draw_wind_arrows(&mut buffer, view, grid, &east, &north);
        let painted = buffer
            .chunks(4)
            .filter(|px| px[0] | px[1] | px[2] != 0)
            .count();
        assert!(
            painted > 20,
            "zoomed view should still receive arrows: {painted}"
        );
        let blueish = buffer
            .chunks(4)
            .filter(|px| px[2] > px[0] && px[2] > 40)
            .count();
        assert!(blueish > 10, "easterly arrows should be blue: {blueish}");
    }
}
