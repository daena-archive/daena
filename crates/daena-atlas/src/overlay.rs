//! Vector overlays, graticule, frame, and style-only paper grain.

use daena_physical::hydrology::HydrologyField;

use serde::{Deserialize, Serialize};

use crate::detail::domain_key;
use crate::detail::lattice_sample;
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
    identity: &[u8],
    overlays: &[AuthoredFeature],
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
    if request.layer_enabled("labels") {
        omitted_labels = crate::labels::draw_labels(buffer, view, overlays, style);
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
