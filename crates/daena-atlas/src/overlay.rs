//! Vector overlays, graticule, frame, and style-only paper grain.

use daena_physical::hydrology::HydrologyField;
use daena_physical::tectonics::TectonicWorld;
use daena_physical::Grid;

use serde::{Deserialize, Serialize};

use crate::detail::domain_key;
use crate::detail::lattice_sample;
use crate::drainage::DerivedTributary;
use crate::projection::ProjectedView;
use crate::request::AtlasRenderRequest;
use crate::style::AtlasStyle;

const FRAME_PX: u32 = 8;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthoredFeature {
    pub id: String,
    pub layer_id: String,
    pub kind: String,
    pub label: Option<String>,
    pub path: Vec<[i32; 2]>,
}

pub fn lon_to_x(lon_micro: i32, width: u32) -> i32 {
    crate::projection::ProjectedView {
        projection: crate::projection::AtlasProjection::Equirectangular,
        extent: crate::projection::AtlasExtent::world(),
        width,
        height: 1,
    }
    .project(lon_micro, 0)
    .map(|(x, _)| x)
    .unwrap_or(0)
}

pub fn lat_to_y(lat_micro: i32, height: u32) -> i32 {
    crate::projection::ProjectedView {
        projection: crate::projection::AtlasProjection::Equirectangular,
        extent: crate::projection::AtlasExtent::world(),
        width: 1,
        height,
    }
    .project(0, lat_micro)
    .map(|(_, y)| y)
    .unwrap_or(0)
}

fn project_point(view: ProjectedView, lon_micro: i32, lat_micro: i32) -> Option<(i32, i32)> {
    view.project(lon_micro, lat_micro)
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

pub fn composite_overlays(
    buffer: &mut [u8],
    request: &AtlasRenderRequest,
    style: &AtlasStyle,
    hydrology: &HydrologyField,
    tectonics: &TectonicWorld,
    identity: &[u8],
    overlays: &[AuthoredFeature],
    tributaries: &[DerivedTributary],
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
    if request.layer_enabled("coastlines") {
        for segment in &hydrology.coastline_segments {
            draw_geodesic_segment(
                buffer,
                view,
                segment.first,
                segment.second,
                style.coast,
                900_000,
            );
        }
    }
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

fn apply_paper_grain(
    buffer: &mut [u8],
    request: &AtlasRenderRequest,
    style: &AtlasStyle,
    identity: &[u8],
) {
    let key = domain_key(
        identity,
        1,
        request.variant,
        &format!(
            "paper-grain\0{}\0{}\0{}x{}",
            style.id, style.version, request.width_px, request.height_px
        ),
    );
    let width = request.width_px;
    let height = request.height_px;
    let strength = style.paper_grain_ppm;
    for y in 0..height {
        for x in 0..width {
            let sample = lattice_sample(&key, x, y, 0);
            let unit = (sample >> 11) as u32 % 1_000_001;
            let delta = ((i64::from(unit) - 500_000) * i64::from(strength) / 1_000_000) as i32;
            let offset = (y as usize * width as usize + x as usize) * 4;
            for channel in 0..3 {
                let value = i32::from(buffer[offset + channel]) + delta / 8;
                buffer[offset + channel] = value.clamp(0, 255) as u8;
            }
        }
    }
}
