//! Deterministic 5×7 bitmap labels keyed by stable feature IDs.

use sha2::{Digest, Sha256};

use crate::overlay::AuthoredFeature;
use crate::projection::ProjectedView;
use crate::style::AtlasStyle;

const GLYPH_WIDTH: i32 = 6;
const GLYPH_HEIGHT: i32 = 8;
const MAX_LABELS: usize = 256;
const FONT_ID: &str = "daena-atlas-bitmap-5x7";

/// Apache-2.0 original 5×7 glyphs. Rows are 5-bit LSB-left.
const GLYPHS: [(u8, [u8; 7]); 37] = [
    (b' ', [0, 0, 0, 0, 0, 0, 0]),
    (b'-', [0, 0, 0, 0x1F, 0, 0, 0]),
    (b'0', [0x0E, 0x11, 0x13, 0x15, 0x19, 0x11, 0x0E]),
    (b'1', [0x04, 0x0C, 0x04, 0x04, 0x04, 0x04, 0x0E]),
    (b'2', [0x0E, 0x11, 0x01, 0x06, 0x08, 0x10, 0x1F]),
    (b'3', [0x1E, 0x01, 0x01, 0x0E, 0x01, 0x01, 0x1E]),
    (b'4', [0x02, 0x06, 0x0A, 0x12, 0x1F, 0x02, 0x02]),
    (b'5', [0x1F, 0x10, 0x1E, 0x01, 0x01, 0x11, 0x0E]),
    (b'6', [0x06, 0x08, 0x10, 0x1E, 0x11, 0x11, 0x0E]),
    (b'7', [0x1F, 0x01, 0x02, 0x04, 0x08, 0x08, 0x08]),
    (b'8', [0x0E, 0x11, 0x11, 0x0E, 0x11, 0x11, 0x0E]),
    (b'9', [0x0E, 0x11, 0x11, 0x0F, 0x01, 0x02, 0x0C]),
    (b'A', [0x0E, 0x11, 0x11, 0x1F, 0x11, 0x11, 0x11]),
    (b'B', [0x1E, 0x11, 0x11, 0x1E, 0x11, 0x11, 0x1E]),
    (b'C', [0x0E, 0x11, 0x10, 0x10, 0x10, 0x11, 0x0E]),
    (b'D', [0x1C, 0x12, 0x11, 0x11, 0x11, 0x12, 0x1C]),
    (b'E', [0x1F, 0x10, 0x10, 0x1E, 0x10, 0x10, 0x1F]),
    (b'F', [0x1F, 0x10, 0x10, 0x1E, 0x10, 0x10, 0x10]),
    (b'G', [0x0E, 0x11, 0x10, 0x17, 0x11, 0x11, 0x0F]),
    (b'H', [0x11, 0x11, 0x11, 0x1F, 0x11, 0x11, 0x11]),
    (b'I', [0x0E, 0x04, 0x04, 0x04, 0x04, 0x04, 0x0E]),
    (b'L', [0x10, 0x10, 0x10, 0x10, 0x10, 0x10, 0x1F]),
    (b'M', [0x11, 0x1B, 0x15, 0x15, 0x11, 0x11, 0x11]),
    (b'N', [0x11, 0x19, 0x15, 0x13, 0x11, 0x11, 0x11]),
    (b'O', [0x0E, 0x11, 0x11, 0x11, 0x11, 0x11, 0x0E]),
    (b'P', [0x1E, 0x11, 0x11, 0x1E, 0x10, 0x10, 0x10]),
    (b'R', [0x1E, 0x11, 0x11, 0x1E, 0x14, 0x12, 0x11]),
    (b'S', [0x0F, 0x10, 0x10, 0x0E, 0x01, 0x01, 0x1E]),
    (b'T', [0x1F, 0x04, 0x04, 0x04, 0x04, 0x04, 0x04]),
    (b'U', [0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x0E]),
    (b'Y', [0x11, 0x11, 0x0A, 0x04, 0x04, 0x04, 0x04]),
    (b'a', [0, 0, 0x0E, 0x01, 0x0F, 0x11, 0x0F]),
    (b'e', [0, 0, 0x0E, 0x11, 0x1F, 0x10, 0x0E]),
    (b'i', [0x04, 0, 0x0C, 0x04, 0x04, 0x04, 0x0E]),
    (b'n', [0, 0, 0x16, 0x19, 0x11, 0x11, 0x11]),
    (b'o', [0, 0, 0x0E, 0x11, 0x11, 0x11, 0x0E]),
    (b'r', [0, 0, 0x16, 0x19, 0x10, 0x10, 0x10]),
];

pub fn font_hash() -> String {
    let mut bytes = FONT_ID.as_bytes().to_vec();
    for (ch, rows) in GLYPHS {
        bytes.push(ch);
        bytes.extend_from_slice(&rows);
    }
    format!("sha256:{:x}", Sha256::digest(&bytes))
}

pub fn draw_labels(
    buffer: &mut [u8],
    view: ProjectedView,
    overlays: &[AuthoredFeature],
    style: &AtlasStyle,
) -> u32 {
    let width = view.width;
    let height = view.height;
    let mut candidates = overlays
        .iter()
        .filter_map(|feature| {
            let label = feature.label.as_deref()?.trim();
            if label.is_empty() || feature.path.is_empty() {
                return None;
            }
            Some((feature.id.as_str(), label, feature.path[0]))
        })
        .collect::<Vec<_>>();
    candidates.sort_by(|a, b| a.0.cmp(b.0));
    let mut occupied = Vec::new();
    let mut omitted = 0_u32;
    for (id, label, point) in candidates {
        if occupied.len() >= MAX_LABELS {
            omitted += 1;
            continue;
        }
        let Some((x, y)) = view.project(point[0], point[1]) else {
            omitted += 1;
            continue;
        };
        let y = y - 8;
        let box_w = (label.chars().count() as i32) * GLYPH_WIDTH;
        let rect = [x, y, x + box_w, y + GLYPH_HEIGHT];
        if occupied
            .iter()
            .any(|other: &[i32; 4]| intersects(*other, rect))
        {
            omitted += 1;
            let _ = id;
            continue;
        }
        occupied.push(rect);
        blit_text(buffer, width, height, x, y, label, style.label_ink);
    }
    omitted
}

pub fn draw_labels_xyz(
    buffer: &mut [u8],
    overlays: &[AuthoredFeature],
    style: &AtlasStyle,
    world_px: u32,
    origin_x: i32,
    origin_y: i32,
    width: u32,
    height: u32,
) -> u32 {
    let view = crate::projection::ProjectedView {
        projection: crate::projection::AtlasProjection::WebMercator,
        extent: crate::projection::AtlasExtent {
            west_lon_micro: -180_000_000,
            south_lat_micro: -crate::projection::WEB_MERCATOR_MAX_LAT_MICRO,
            east_lon_micro: 180_000_000,
            north_lat_micro: crate::projection::WEB_MERCATOR_MAX_LAT_MICRO,
        },
        width: world_px,
        height: world_px,
    };
    let mut candidates = overlays
        .iter()
        .filter_map(|feature| {
            let label = feature.label.as_deref()?.trim();
            if label.is_empty() || feature.path.is_empty() {
                return None;
            }
            Some((feature.id.as_str(), label, feature.path[0]))
        })
        .collect::<Vec<_>>();
    candidates.sort_by(|a, b| a.0.cmp(b.0));
    let mut occupied = Vec::new();
    let mut omitted = 0_u32;
    let world = world_px as i32;
    for (id, label, point) in candidates {
        if occupied.len() >= MAX_LABELS {
            omitted += 1;
            continue;
        }
        let Some((x, south_y)) = view.project(point[0], point[1]) else {
            omitted += 1;
            continue;
        };
        let y = world - 1 - south_y - 8;
        let box_w = (label.chars().count() as i32) * GLYPH_WIDTH;
        let rect = [x, y, x + box_w, y + GLYPH_HEIGHT];
        if occupied
            .iter()
            .any(|other: &[i32; 4]| intersects(*other, rect))
        {
            omitted += 1;
            let _ = id;
            continue;
        }
        occupied.push(rect);
        blit_text(
            buffer,
            width,
            height,
            x - origin_x,
            y - origin_y,
            label,
            style.label_ink,
        );
        if x < 8 || x + box_w > world - 8 {
            blit_text(
                buffer,
                width,
                height,
                x + world - origin_x,
                y - origin_y,
                label,
                style.label_ink,
            );
            blit_text(
                buffer,
                width,
                height,
                x - world - origin_x,
                y - origin_y,
                label,
                style.label_ink,
            );
        }
    }
    omitted
}

fn intersects(a: [i32; 4], b: [i32; 4]) -> bool {
    a[0] < b[2] && b[0] < a[2] && a[1] < b[3] && b[1] < a[3]
}

fn blit_text(
    buffer: &mut [u8],
    width: u32,
    height: u32,
    origin_x: i32,
    origin_y: i32,
    text: &str,
    rgb: [u8; 3],
) {
    for (index, ch) in text.chars().enumerate() {
        let glyph = glyph_for(ch);
        let x0 = origin_x + index as i32 * GLYPH_WIDTH;
        for (row, bits) in glyph.iter().enumerate() {
            for col in 0..5 {
                if bits & (1 << col) != 0 {
                    crate::overlay::put_pixel(
                        buffer,
                        width,
                        height,
                        x0 + col,
                        origin_y + row as i32,
                        rgb,
                        1_000_000,
                    );
                }
            }
        }
    }
}

fn glyph_for(ch: char) -> [u8; 7] {
    let key = if ch.is_ascii_lowercase() && !matches!(ch, 'a' | 'e' | 'i' | 'n' | 'o' | 'r') {
        ch.to_ascii_uppercase() as u8
    } else if ch.is_ascii() {
        ch as u8
    } else {
        b'?'
    };
    GLYPHS
        .iter()
        .find(|(ch, _)| *ch == key)
        .map(|(_, rows)| *rows)
        .unwrap_or([0x1F, 0x11, 0x11, 0x11, 0x11, 0x11, 0x1F])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn font_hash_is_stable() {
        assert!(font_hash().starts_with("sha256:"));
        assert_eq!(font_hash(), font_hash());
    }
}
