//! Per-tile GeoJSON for coastline, lakes, rivers, streams, and contours.
//!
//! The lattice sits on the zoom's scale-1 pixel grid, one node every two
//! pixels, with one node of halo. Shared tile edges therefore sample `h(p)`
//! at the same directions. Rivers use the terrain's displaced centre-line,
//! the same curve carved into `h(p)`.

use std::collections::BTreeMap;

use rayon::prelude::*;

use super::render::{
    direction_from_lon_lat, ground_metres_per_pixel, mercator_latitude, mercator_longitude,
    mercator_world_px,
};
use super::streams::{self, StreamLine};
use super::{AtlasTerrain, TerrainCover};
use crate::AtlasError;

const TILE_PX: f64 = 256.0;
const LATTICE_STEP_PX: f64 = 2.0;
/// 129 nodes across the tile, plus one node of halo on each side.
const NODE_SPAN: usize = 131;
const HALO_PX: f64 = LATTICE_STEP_PX;
const FIELD_FLOOR_M: f64 = 0.01;
/// Keeps a simplified river inside half a pixel of the carved centre-line.
const RIVER_SIMPLIFY_PX: f64 = 0.35;
const COORD_SCALE: f64 = 1_000_000.0;

/// One vector layer of a terrain tile.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum TerrainVectorLayer {
    Coastline,
    Lakes,
    Rivers,
    Contours,
    Streams,
}

impl TerrainVectorLayer {
    /// Parse a tile suffix: `coastline`, `lakes`, `rivers`, `streams`, or `contours`.
    #[must_use]
    pub fn from_id(id: &str) -> Option<Self> {
        match id {
            "coastline" => Some(Self::Coastline),
            "lakes" => Some(Self::Lakes),
            "rivers" => Some(Self::Rivers),
            "contours" => Some(Self::Contours),
            "streams" => Some(Self::Streams),
            _ => None,
        }
    }

    /// Stable layer id used in the tile URL.
    #[must_use]
    pub fn id(self) -> &'static str {
        match self {
            Self::Coastline => "coastline",
            Self::Lakes => "lakes",
            Self::Rivers => "rivers",
            Self::Contours => "contours",
            Self::Streams => "streams",
        }
    }
}

#[derive(Clone)]
struct RiverPiece {
    lonlat: Vec<[f64; 2]>,
    min_lon: f64,
    max_lon: f64,
    min_lat: f64,
    max_lat: f64,
    run: u32,
    s0: f64,
    s1: f64,
}

struct PreparedRiver {
    id: String,
    order: u16,
    channel_width_m: f64,
    run_start: u32,
    run_count: u32,
    pieces: Vec<RiverPiece>,
}

/// Smoothed canonical rivers for one [`AtlasTerrain`].
///
/// Built once per terrain. Tile renders only clip and simplify.
pub struct TerrainVectors {
    rivers: Vec<PreparedRiver>,
}

impl TerrainVectors {
    /// Clip geometry for the centre-lines already built on `terrain`.
    #[must_use]
    pub fn new(terrain: &AtlasTerrain) -> Self {
        let step = (terrain.cell_size_m() / 8.0).clamp(250.0, 2_000.0);
        let rivers = terrain
            .rivers
            .export_rivers(step)
            .into_iter()
            .map(|river| PreparedRiver {
                id: river.id,
                order: river.order,
                channel_width_m: river.channel_width_m,
                run_start: river.run_start,
                run_count: river.run_count,
                pieces: river
                    .pieces
                    .into_iter()
                    .map(|piece| RiverPiece {
                        lonlat: piece.lonlat,
                        min_lon: piece.min_lon,
                        max_lon: piece.max_lon,
                        min_lat: piece.min_lat,
                        max_lat: piece.max_lat,
                        run: piece.run,
                        s0: piece.s0,
                        s1: piece.s1,
                    })
                    .collect(),
            })
            .collect();
        Self { rivers }
    }

    /// Bytes retained by the smoothed polylines, including spare capacity.
    #[must_use]
    pub fn storage_bytes(&self) -> usize {
        let mut bytes = self.rivers.capacity() * std::mem::size_of::<PreparedRiver>();
        for river in &self.rivers {
            bytes += river.id.capacity();
            bytes += river.pieces.capacity() * std::mem::size_of::<RiverPiece>();
            for piece in &river.pieces {
                bytes += piece.lonlat.capacity() * std::mem::size_of::<[f64; 2]>();
            }
        }
        bytes
    }
}

struct Node {
    px: [f64; 2],
    height: f64,
    cover: TerrainCover,
    lake_level: Option<f64>,
}

enum Props {
    Empty,
    River {
        id: String,
        order: u16,
        channel_width_m: f64,
    },
    Contour {
        elevation_m: i32,
        index: bool,
    },
    Stream {
        id: String,
        order: u16,
        accumulation: u32,
    },
}

struct Drawn {
    props: Props,
    ring: Vec<[f64; 2]>,
}

/// GeoJSON `FeatureCollection` for one layer of one Web Mercator tile.
///
/// Coordinates are longitude, latitude in degrees, rounded to 1e-6°.
/// The bytes do not depend on the Rayon thread count.
pub fn render_terrain_vector_tile(
    terrain: &AtlasTerrain,
    vectors: &TerrainVectors,
    layer: TerrainVectorLayer,
    z: u32,
    x: u32,
    y: u32,
) -> Result<Vec<u8>, AtlasError> {
    validate_tile(z, x, y)?;
    let drawn = match layer {
        TerrainVectorLayer::Coastline => coastline(terrain, z, x, y),
        TerrainVectorLayer::Lakes => lakes(terrain, z, x, y),
        TerrainVectorLayer::Rivers => rivers(terrain, vectors, z, x, y),
        TerrainVectorLayer::Contours => contours(terrain, z, x, y),
        TerrainVectorLayer::Streams => streams(terrain, z, x, y),
    };
    Ok(encode_collection(&drawn))
}

fn validate_tile(z: u32, x: u32, y: u32) -> Result<(), AtlasError> {
    if z > super::TERRAIN_MAX_ZOOM {
        return Err(AtlasError::invalid("atlas terrain zoom exceeds 12"));
    }
    let n = 1u32
        .checked_shl(z)
        .ok_or_else(|| AtlasError::invalid("atlas terrain zoom exceeds 12"))?;
    if x >= n || y >= n {
        return Err(AtlasError::invalid(
            "atlas terrain tile is outside the zoom",
        ));
    }
    Ok(())
}

/// Lowest Strahler order drawn at `z`.
///
/// Earthlike worlds at n=512 and n=1024 top out at order 5 (17 rivers). The
/// brief's order-6 band is empty, so the ladder starts at 5. Order 1 stays
/// off until z 11, where a tile is small enough for creeks.
fn min_strahler_order(z: u32) -> u16 {
    match z {
        0..=2 => 5,
        3..=4 => 4,
        5..=6 => 3,
        7..=10 => 2,
        _ => 0,
    }
}

fn contour_interval_m(z: u32) -> i32 {
    match z {
        0..=6 => 500,
        7..=8 => 200,
        9..=10 => 100,
        _ => 50,
    }
}

fn node_gx(tile_x: u32, col: usize) -> f64 {
    f64::from(tile_x) * TILE_PX + LATTICE_STEP_PX * (col as f64 - 1.0)
}

fn node_gy(tile_y: u32, row: usize) -> f64 {
    f64::from(tile_y) * TILE_PX + LATTICE_STEP_PX * (row as f64 - 1.0)
}

fn sample_nodes(terrain: &AtlasTerrain, z: u32, x: u32, y: u32) -> Vec<Node> {
    let world = mercator_world_px(z);
    let radius = terrain.radius_m();
    let mut nodes = Vec::with_capacity(NODE_SPAN * NODE_SPAN);
    nodes.resize_with(NODE_SPAN * NODE_SPAN, || Node {
        px: [0.0, 0.0],
        height: 0.0,
        cover: TerrainCover::Land,
        lake_level: None,
    });
    nodes
        .par_chunks_mut(NODE_SPAN)
        .enumerate()
        .for_each(|(row, chunk)| {
            let gy = node_gy(y, row);
            let lat = mercator_latitude(gy, world);
            let ground = ground_metres_per_pixel(z, lat, radius);
            for (col, node) in chunk.iter_mut().enumerate() {
                let gx = node_gx(x, col);
                let lon = mercator_longitude(gx, world);
                let direction = direction_from_lon_lat(lon, lat);
                let (height, cover, _) = terrain.sample_pixel(direction, ground);
                let lake_level = terrain.dominant_lake_level_m(direction);
                *node = Node {
                    px: [gx, gy],
                    height,
                    cover,
                    lake_level,
                };
            }
        });
    nodes
}

fn any_lake_in_stencils(terrain: &AtlasTerrain, z: u32, x: u32, y: u32) -> bool {
    let world = mercator_world_px(z);
    for row in 0..NODE_SPAN {
        let gy = node_gy(y, row);
        let lat = mercator_latitude(gy, world);
        for col in 0..NODE_SPAN {
            let gx = node_gx(x, col);
            let lon = mercator_longitude(gx, world);
            let direction = direction_from_lon_lat(lon, lat);
            if terrain.dominant_lake_level_m(direction).is_some() {
                return true;
            }
        }
    }
    false
}

fn signed_magnitude(height: f64, level: f64, inside: bool) -> f64 {
    let magnitude = (height - level).abs().max(FIELD_FLOOR_M);
    if inside {
        -magnitude
    } else {
        magnitude
    }
}

fn coastline(terrain: &AtlasTerrain, z: u32, x: u32, y: u32) -> Vec<Drawn> {
    let nodes = sample_nodes(terrain, z, x, y);
    let sea = terrain.sea_level_m();
    let mut fields = Vec::with_capacity(nodes.len());
    let mut saw_ocean = false;
    let mut saw_other = false;
    for node in &nodes {
        if node.cover == TerrainCover::Ocean {
            saw_ocean = true;
        } else {
            saw_other = true;
        }
        fields.push(signed_magnitude(
            node.height,
            sea,
            node.cover == TerrainCover::Ocean,
        ));
    }
    if !saw_ocean || !saw_other {
        return Vec::new();
    }
    finish_lattice(z, x, y, &march(&nodes, &fields), Props::Empty)
}

fn lakes(terrain: &AtlasTerrain, z: u32, x: u32, y: u32) -> Vec<Drawn> {
    if !any_lake_in_stencils(terrain, z, x, y) {
        return Vec::new();
    }
    let nodes = sample_nodes(terrain, z, x, y);
    let mut fields = Vec::with_capacity(nodes.len());
    let mut saw_lake = false;
    let mut saw_other = false;
    for node in &nodes {
        let inside = node.cover == TerrainCover::Lake;
        if inside {
            saw_lake = true;
        } else {
            saw_other = true;
        }
        let level = node.lake_level.unwrap_or(node.height);
        fields.push(signed_magnitude(node.height, level, inside));
    }
    if !saw_lake || !saw_other {
        return Vec::new();
    }
    finish_lattice(z, x, y, &march(&nodes, &fields), Props::Empty)
}

fn contours(terrain: &AtlasTerrain, z: u32, x: u32, y: u32) -> Vec<Drawn> {
    let nodes = sample_nodes(terrain, z, x, y);
    let sea = terrain.sea_level_m();
    let mut min_rel = f64::MAX;
    let mut max_rel = f64::MIN;
    let mut land = false;
    for node in &nodes {
        if node.cover != TerrainCover::Land {
            continue;
        }
        land = true;
        let relative = node.height - sea;
        min_rel = min_rel.min(relative);
        max_rel = max_rel.max(relative);
    }
    if !land {
        return Vec::new();
    }
    let interval = contour_interval_m(z);
    let mut drawn = Vec::new();
    for level in contour_levels(min_rel, max_rel, interval) {
        let mut fields = Vec::with_capacity(nodes.len());
        for node in &nodes {
            fields.push(node.height - sea - f64::from(level));
        }
        let lines = march_land(&nodes, &fields);
        let index = level.rem_euclid(interval.saturating_mul(5)) == 0;
        drawn.extend(finish_lattice(
            z,
            x,
            y,
            &lines,
            Props::Contour {
                elevation_m: level,
                index,
            },
        ));
    }
    drawn
}

fn contour_levels(min_rel: f64, max_rel: f64, interval: i32) -> Vec<i32> {
    if interval <= 0 || !min_rel.is_finite() || !max_rel.is_finite() {
        return Vec::new();
    }
    let step = f64::from(interval);
    let start = (min_rel / step).ceil();
    let end = (max_rel / step).floor();
    if !start.is_finite() || !end.is_finite() || start > end {
        return Vec::new();
    }
    let mut levels = Vec::new();
    let mut k = start;
    while k <= end {
        let level = (k * step).round();
        if level >= f64::from(i32::MIN) && level <= f64::from(i32::MAX) {
            levels.push(level as i32);
        }
        k += 1.0;
        if levels.len() > 10_000 {
            break;
        }
    }
    levels
}

fn rivers(terrain: &AtlasTerrain, vectors: &TerrainVectors, z: u32, x: u32, y: u32) -> Vec<Drawn> {
    let min_order = min_strahler_order(z);
    let expanded = expanded_rect(x, y);
    let bounds = tile_lonlat_bounds(z, x, y, HALO_PX);
    let world = mercator_world_px(z);
    let gy = (f64::from(y) + 0.5) * TILE_PX;
    let lat = mercator_latitude(gy, world);
    let ground = super::ground_metres_per_pixel(z, lat, terrain.radius_m()).max(1.0);
    let coarse = (terrain.cell_size_m() / 8.0).clamp(250.0, 2_000.0);
    let mut drawn = Vec::new();
    for river in &vectors.rivers {
        if river.order < min_order {
            continue;
        }
        let step = river_sample_step(ground, river.channel_width_m);
        let spans = if river.pieces.is_empty() {
            (river.run_start..river.run_start + river.run_count)
                .map(|run| (run, 0.0, f64::MAX))
                .collect::<Vec<_>>()
        } else {
            let hits = river
                .pieces
                .iter()
                .filter(|piece| piece_hits(piece, bounds))
                .map(|piece| (piece.run, piece.s0, piece.s1))
                .collect::<Vec<_>>();
            merge_run_spans(&hits)
        };
        for (run, s0, s1) in spans {
            let lonlat = terrain
                .rivers
                .sample_run_span(run, step, s0 - coarse, s1 + coarse);
            for part in split_dateline(&lonlat) {
                let pixels = part
                    .iter()
                    .map(|point| lonlat_to_px(point[0], point[1], z, x))
                    .collect::<Vec<_>>();
                for clipped in clip_polyline(&pixels, expanded) {
                    let simplified = douglas_peucker(&clipped, RIVER_SIMPLIFY_PX);
                    let lonlat = simplified
                        .iter()
                        .map(|px| px_to_lonlat(z, px[0], px[1]))
                        .collect::<Vec<_>>();
                    for ring in split_dateline(&lonlat) {
                        let ring = round_ring(&ring);
                        if ring.len() < 2 {
                            continue;
                        }
                        drawn.push(Drawn {
                            props: Props::River {
                                id: river.id.clone(),
                                order: river.order,
                                channel_width_m: river.channel_width_m,
                            },
                            ring,
                        });
                    }
                }
            }
        }
    }
    drawn
}

/// Land cells whose region can meet the tile. Below zoom 8 the tile is empty.
///
/// Streams are clipped to the strict tile rectangle, the same way a shared
/// coastline edge is, so both tiles cut a crossing at the same point.
fn streams(terrain: &AtlasTerrain, z: u32, x: u32, y: u32) -> Vec<Drawn> {
    if streams::stream_threshold(z, 1, false).is_none() {
        return Vec::new();
    }
    let cells = cells_touching(terrain, z, x, y);
    let trees: Vec<_> = cells
        .par_iter()
        .map(|cell| streams::tree(terrain, *cell).map(|tree| (*cell, tree)))
        .collect();
    let strict = strict_rect(x, y);
    let mut drawn = Vec::new();
    for entry in trees.into_iter().flatten() {
        let (cell, tree) = entry;
        for line in tree.lines(cell, z) {
            drawn.extend(draw_stream(z, x, &line, strict));
        }
    }
    drawn
}

fn cells_touching(terrain: &AtlasTerrain, z: u32, x: u32, y: u32) -> Vec<u32> {
    let (lon0, lat0, lon1, lat1) = tile_lonlat_bounds(z, x, y, HALO_PX);
    let cell_deg = (terrain.cell_size_m() / terrain.radius_m()).to_degrees();
    let margin = cell_deg * 0.75;
    let step = (cell_deg * 0.45).max(1e-4);
    let lat_start = (lat0 - margin).max(-85.0);
    let lat_end = (lat1 + margin).min(85.0);
    let lon_start = lon0 - margin;
    let lon_end = lon1 + margin;
    let mut cells = Vec::new();
    let mut lat = lat_start;
    while lat <= lat_end + step * 0.1 {
        let mut lon = lon_start;
        while lon <= lon_end + step * 0.1 {
            let dir = direction_from_lon_lat(lon.to_radians(), lat.to_radians());
            let cell = terrain.surface().cell_at_direction(dir);
            cells.push(cell);
            for neighbor in terrain.surface().neighbors(cell) {
                if neighbor != daena_physical::engine::TOOL_SURFACE_NONE {
                    cells.push(neighbor);
                }
            }
            lon += step;
        }
        lat += step;
    }
    cells.sort_unstable();
    cells.dedup();
    cells
}

fn draw_stream(z: u32, x: u32, line: &StreamLine, rect: [f64; 4]) -> Vec<Drawn> {
    let mut drawn = Vec::new();
    for part in split_dateline(&line.lonlat) {
        let pixels = part
            .iter()
            .map(|point| lonlat_to_px(point[0], point[1], z, x))
            .collect::<Vec<_>>();
        for clipped in clip_polyline(&pixels, rect) {
            let simplified = douglas_peucker(&clipped, RIVER_SIMPLIFY_PX);
            let lonlat = simplified
                .iter()
                .map(|px| px_to_lonlat(z, px[0], px[1]))
                .collect::<Vec<_>>();
            for ring in split_dateline(&lonlat) {
                let ring = round_ring(&ring);
                if ring.len() < 2 {
                    continue;
                }
                drawn.push(Drawn {
                    props: Props::Stream {
                        id: line.id.clone(),
                        order: line.order,
                        accumulation: line.accumulation,
                    },
                    ring,
                });
            }
        }
    }
    drawn
}

fn merge_run_spans(spans: &[(u32, f64, f64)]) -> Vec<(u32, f64, f64)> {
    let mut spans = spans.to_vec();
    spans.sort_by(|left, right| left.0.cmp(&right.0).then(left.1.total_cmp(&right.1)));
    let mut merged = Vec::new();
    for (run, s0, s1) in spans {
        if let Some((last_run, _, last_s1)) = merged.last_mut() {
            if *last_run == run && s0 <= *last_s1 {
                *last_s1 = last_s1.max(s1);
                continue;
            }
        }
        merged.push((run, s0, s1));
    }
    merged
}

/// Dense enough that a chord stays inside half a pixel, and never coarser than
/// a fraction of the meander wavelength.
fn river_sample_step(ground_m: f64, width_m: f64) -> f64 {
    let lambda = (12.0 * width_m).max(1_000.0);
    let pixel = (ground_m * 0.2).max(4.0);
    pixel.min(lambda / 8.0)
}

fn finish_lattice(z: u32, x: u32, y: u32, lines: &[Vec<[f64; 2]>], props: Props) -> Vec<Drawn> {
    let strict = strict_rect(x, y);
    let mut drawn = Vec::new();
    for line in lines {
        for part in split_on_rect(line, strict) {
            let lonlat = part
                .iter()
                .map(|px| px_to_lonlat(z, px[0], px[1]))
                .collect::<Vec<_>>();
            for ring in split_dateline(&lonlat) {
                let ring = round_ring(&ring);
                if ring.len() < 2 {
                    continue;
                }
                drawn.push(Drawn {
                    props: clone_props(&props),
                    ring,
                });
            }
        }
    }
    drawn
}

fn clone_props(props: &Props) -> Props {
    match props {
        Props::Empty => Props::Empty,
        Props::River {
            id,
            order,
            channel_width_m,
        } => Props::River {
            id: id.clone(),
            order: *order,
            channel_width_m: *channel_width_m,
        },
        Props::Contour { elevation_m, index } => Props::Contour {
            elevation_m: *elevation_m,
            index: *index,
        },
        Props::Stream {
            id,
            order,
            accumulation,
        } => Props::Stream {
            id: id.clone(),
            order: *order,
            accumulation: *accumulation,
        },
    }
}

fn march(nodes: &[Node], fields: &[f64]) -> Vec<Vec<[f64; 2]>> {
    let mut segments = Vec::new();
    for row in 0..NODE_SPAN - 1 {
        for col in 0..NODE_SPAN - 1 {
            push_cell_segments(&mut segments, nodes, fields, col, row, |_| true);
        }
    }
    chain_segments(segments)
}

fn march_land(nodes: &[Node], fields: &[f64]) -> Vec<Vec<[f64; 2]>> {
    let mut segments = Vec::new();
    for row in 0..NODE_SPAN - 1 {
        for col in 0..NODE_SPAN - 1 {
            push_cell_segments(&mut segments, nodes, fields, col, row, |cover| {
                cover == TerrainCover::Land
            });
        }
    }
    chain_segments(segments)
}

fn push_cell_segments(
    segments: &mut Vec<([f64; 2], [f64; 2])>,
    nodes: &[Node],
    fields: &[f64],
    col: usize,
    row: usize,
    keep: impl Fn(TerrainCover) -> bool,
) {
    let corners = [
        row * NODE_SPAN + col,
        row * NODE_SPAN + col + 1,
        (row + 1) * NODE_SPAN + col + 1,
        (row + 1) * NODE_SPAN + col,
    ];
    if !corners.iter().all(|index| keep(nodes[*index].cover)) {
        return;
    }
    let values = [
        fields[corners[0]],
        fields[corners[1]],
        fields[corners[2]],
        fields[corners[3]],
    ];
    let mut mask = 0u8;
    for (bit, value) in values.iter().enumerate() {
        if *value < 0.0 {
            mask |= 1 << bit;
        }
    }
    let centre = (values[0] + values[1] + values[2] + values[3]) * 0.25;
    for (left, right) in edge_pairs(mask, centre < 0.0) {
        if left > 3 {
            continue;
        }
        let a = edge_point(left, &corners, nodes, &values);
        let b = edge_point(right, &corners, nodes, &values);
        if (a[0] - b[0]).abs() <= 1e-9 && (a[1] - b[1]).abs() <= 1e-9 {
            continue;
        }
        segments.push((a, b));
    }
}

/// Edge pairs for one marching-squares cell.
///
/// Corners are NW, NE, SE, SW. Bit 0 is NW. A set bit is the negative side.
/// Masks 5 and 10 are saddles: a negative centre joins the negative corners.
fn edge_pairs(mask: u8, centre_negative: bool) -> [(u8, u8); 2] {
    const NONE: (u8, u8) = (255, 255);
    match mask {
        0 | 15 => [NONE, NONE],
        1 => [(3, 0), NONE],
        2 => [(0, 1), NONE],
        3 => [(3, 1), NONE],
        4 => [(1, 2), NONE],
        5 => {
            if centre_negative {
                [(0, 1), (2, 3)]
            } else {
                [(3, 0), (1, 2)]
            }
        }
        6 => [(0, 2), NONE],
        7 => [(2, 3), NONE],
        8 => [(2, 3), NONE],
        9 => [(2, 0), NONE],
        10 => {
            if centre_negative {
                [(3, 0), (1, 2)]
            } else {
                [(0, 1), (2, 3)]
            }
        }
        11 => [(1, 2), NONE],
        12 => [(1, 3), NONE],
        13 => [(0, 1), NONE],
        14 => [(3, 0), NONE],
        _ => [NONE, NONE],
    }
}

fn edge_point(edge: u8, corners: &[usize; 4], nodes: &[Node], values: &[f64; 4]) -> [f64; 2] {
    let (from, to) = match edge {
        0 => (0, 1),
        1 => (1, 2),
        2 => (2, 3),
        _ => (3, 0),
    };
    let start = nodes[corners[from]].px;
    let end = nodes[corners[to]].px;
    let v0 = values[from];
    let v1 = values[to];
    let denom = v0 - v1;
    let t = if denom.abs() < 1e-15 {
        0.5
    } else {
        (v0 / denom).clamp(0.0, 1.0)
    };
    [
        start[0] + t * (end[0] - start[0]),
        start[1] + t * (end[1] - start[1]),
    ]
}

fn chain_segments(segments: Vec<([f64; 2], [f64; 2])>) -> Vec<Vec<[f64; 2]>> {
    #[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
    struct Key(i64, i64);
    fn key(point: [f64; 2]) -> Key {
        Key(
            (point[0] * COORD_SCALE).round() as i64,
            (point[1] * COORD_SCALE).round() as i64,
        )
    }
    struct Seg {
        a: Key,
        b: Key,
        pa: [f64; 2],
        pb: [f64; 2],
    }
    let segs = segments
        .into_iter()
        .filter_map(|(pa, pb)| {
            let a = key(pa);
            let b = key(pb);
            if a == b {
                None
            } else {
                Some(Seg { a, b, pa, pb })
            }
        })
        .collect::<Vec<_>>();
    if segs.is_empty() {
        return Vec::new();
    }
    let mut adjacent: BTreeMap<Key, Vec<usize>> = BTreeMap::new();
    for (index, seg) in segs.iter().enumerate() {
        adjacent.entry(seg.a).or_default().push(index);
        adjacent.entry(seg.b).or_default().push(index);
    }
    let mut used = vec![false; segs.len()];
    let mut lines = Vec::new();
    let odd_keys = adjacent
        .iter()
        .filter(|(_, edges)| edges.len() % 2 == 1)
        .map(|(key, _)| *key)
        .collect::<Vec<_>>();
    let all_keys = adjacent.keys().copied().collect::<Vec<_>>();
    for starts in [odd_keys, all_keys] {
        for start in starts {
            while adjacent
                .get(&start)
                .is_some_and(|edges| edges.iter().any(|index| !used[*index]))
            {
                let mut line = Vec::new();
                let mut current = start;
                for _ in 0..=segs.len() {
                    let Some(next) = adjacent
                        .get(&current)
                        .and_then(|edges| edges.iter().copied().find(|index| !used[*index]))
                    else {
                        break;
                    };
                    used[next] = true;
                    let seg = &segs[next];
                    let (here, next_key, there) = if seg.a == current {
                        (seg.pa, seg.b, seg.pb)
                    } else {
                        (seg.pb, seg.a, seg.pa)
                    };
                    if line.is_empty() {
                        line.push(here);
                    }
                    line.push(there);
                    if next_key == start {
                        break;
                    }
                    current = next_key;
                }
                if line.len() >= 2 {
                    lines.push(line);
                }
            }
        }
    }
    lines
}

fn strict_rect(x: u32, y: u32) -> [f64; 4] {
    let min_x = f64::from(x) * TILE_PX;
    let min_y = f64::from(y) * TILE_PX;
    [min_x, min_y, min_x + TILE_PX, min_y + TILE_PX]
}

fn expanded_rect(x: u32, y: u32) -> [f64; 4] {
    let strict = strict_rect(x, y);
    [
        strict[0] - HALO_PX,
        strict[1] - HALO_PX,
        strict[2] + HALO_PX,
        strict[3] + HALO_PX,
    ]
}

fn inside_rect(point: [f64; 2], rect: [f64; 4], eps: f64) -> bool {
    point[0] >= rect[0] - eps
        && point[0] <= rect[2] + eps
        && point[1] >= rect[1] - eps
        && point[1] <= rect[3] + eps
}

fn on_boundary(point: [f64; 2], rect: [f64; 4]) -> bool {
    const EPS: f64 = 1e-6;
    if !inside_rect(point, rect, EPS) {
        return false;
    }
    (point[0] - rect[0]).abs() <= EPS
        || (point[0] - rect[2]).abs() <= EPS
        || (point[1] - rect[1]).abs() <= EPS
        || (point[1] - rect[3]).abs() <= EPS
}

fn points_close(left: [f64; 2], right: [f64; 2]) -> bool {
    (left[0] - right[0]).abs() <= 1e-9 && (left[1] - right[1]).abs() <= 1e-9
}

fn push_distinct(line: &mut Vec<[f64; 2]>, point: [f64; 2]) {
    if line.last().is_none_or(|last| !points_close(*last, point)) {
        line.push(point);
    }
}

/// Break a polyline where it crosses the strict tile rectangle.
///
/// Both the interior and the one-cell halo are kept, so a shared edge becomes
/// a linestring endpoint in each tile.
fn split_on_rect(line: &[[f64; 2]], rect: [f64; 4]) -> Vec<Vec<[f64; 2]>> {
    if line.len() < 2 {
        return Vec::new();
    }
    let mut parts = Vec::new();
    let mut current = vec![line[0]];
    for pair in line.windows(2) {
        let a = pair[0];
        let b = pair[1];
        let a_in = inside_rect(a, rect, 1e-6);
        let b_in = inside_rect(b, rect, 1e-6);
        if a_in == b_in {
            push_distinct(&mut current, b);
            continue;
        }
        let hit = if a_in && on_boundary(a, rect) {
            a
        } else if b_in && on_boundary(b, rect) {
            b
        } else {
            boundary_cross(a, b, rect).unwrap_or(if a_in { b } else { a })
        };
        push_distinct(&mut current, hit);
        if current.len() >= 2 {
            parts.push(std::mem::take(&mut current));
        } else {
            current.clear();
        }
        current.push(hit);
        push_distinct(&mut current, b);
    }
    if current.len() >= 2 {
        parts.push(current);
    }
    parts
}

fn boundary_cross(a: [f64; 2], b: [f64; 2], rect: [f64; 4]) -> Option<[f64; 2]> {
    let dx = b[0] - a[0];
    let dy = b[1] - a[1];
    let planes = [
        (dx, rect[0] - a[0]),
        (dx, rect[2] - a[0]),
        (dy, rect[1] - a[1]),
        (dy, rect[3] - a[1]),
    ];
    let mut best: Option<(f64, [f64; 2])> = None;
    for (delta, offset) in planes {
        if delta.abs() < 1e-15 {
            continue;
        }
        let t = offset / delta;
        if !(1e-9..=1.0 - 1e-9).contains(&t) {
            continue;
        }
        let point = [a[0] + t * dx, a[1] + t * dy];
        if !inside_rect(point, rect, 1e-5) {
            continue;
        }
        let replace = match best {
            None => true,
            Some((best_t, _)) => t < best_t,
        };
        if replace {
            best = Some((t, point));
        }
    }
    best.map(|(_, point)| point)
}

fn px_to_lonlat(z: u32, gx: f64, gy: f64) -> [f64; 2] {
    let world = mercator_world_px(z);
    let lon = wrap_lon(180.0 * (2.0 * gx / world - 1.0));
    let lat = mercator_latitude(gy, world).to_degrees();
    [lon, lat]
}

fn lonlat_to_px(lon: f64, lat: f64, z: u32, tile_x: u32) -> [f64; 2] {
    let world = mercator_world_px(z);
    let mut gx = (lon + 180.0) / 360.0 * world;
    let tile_gx = f64::from(tile_x) * TILE_PX;
    if gx - tile_gx > world * 0.5 {
        gx -= world;
    } else if tile_gx - gx > world * 0.5 {
        gx += world;
    }
    let lat = lat.clamp(-85.051_128_78, 85.051_128_78);
    let mercator_y = lat.to_radians().tan().asinh();
    let gy = (std::f64::consts::PI - mercator_y) / std::f64::consts::TAU * world;
    [gx, gy]
}

fn wrap_lon(lon: f64) -> f64 {
    if !lon.is_finite() {
        return 0.0;
    }
    if (-180.0..=180.0).contains(&lon) {
        return if lon == 0.0 { 0.0 } else { lon };
    }
    let wrapped = (lon + 180.0).rem_euclid(360.0) - 180.0;
    if wrapped <= -180.0 {
        180.0
    } else {
        wrapped
    }
}

fn split_dateline(line: &[[f64; 2]]) -> Vec<Vec<[f64; 2]>> {
    if line.len() < 2 {
        return Vec::new();
    }
    let mut parts = Vec::new();
    let mut current = vec![line[0]];
    for pair in line.windows(2) {
        let a = pair[0];
        let b = pair[1];
        if (a[0] - b[0]).abs() <= 180.0 {
            current.push(b);
            continue;
        }
        let (end_lon, start_lon) = if a[0] > b[0] {
            (180.0, -180.0)
        } else {
            (-180.0, 180.0)
        };
        let span = (end_lon - a[0]).abs() + (b[0] - start_lon).abs();
        let t = if span < 1e-12 {
            0.5
        } else {
            (end_lon - a[0]).abs() / span
        };
        let lat = a[1] + t * (b[1] - a[1]);
        current.push([end_lon, lat]);
        if current.len() >= 2 {
            parts.push(std::mem::take(&mut current));
        }
        current = vec![[start_lon, lat], b];
    }
    if current.len() >= 2 {
        parts.push(current);
    }
    parts
}

fn round_ring(line: &[[f64; 2]]) -> Vec<[f64; 2]> {
    let mut rounded = Vec::with_capacity(line.len());
    for point in line {
        let next = [round_1e6(point[0]), round_1e6(point[1])];
        if rounded.last().is_none_or(|last: &[f64; 2]| *last != next) {
            rounded.push(next);
        }
    }
    rounded
}

fn round_1e6(value: f64) -> f64 {
    if !value.is_finite() {
        return 0.0;
    }
    (value * COORD_SCALE).round() / COORD_SCALE
}

fn tile_lonlat_bounds(z: u32, x: u32, y: u32, halo_px: f64) -> (f64, f64, f64, f64) {
    let world = mercator_world_px(z);
    let gx0 = f64::from(x) * TILE_PX - halo_px;
    let gx1 = (f64::from(x) + 1.0) * TILE_PX + halo_px;
    let gy0 = f64::from(y) * TILE_PX - halo_px;
    let gy1 = (f64::from(y) + 1.0) * TILE_PX + halo_px;
    let lon0 = 180.0 * (2.0 * gx0 / world - 1.0);
    let lon1 = 180.0 * (2.0 * gx1 / world - 1.0);
    let lat_north = mercator_latitude(gy0, world).to_degrees();
    let lat_south = mercator_latitude(gy1, world).to_degrees();
    (
        lon0,
        lat_south.min(lat_north),
        lon1,
        lat_south.max(lat_north),
    )
}

fn piece_hits(piece: &RiverPiece, bounds: (f64, f64, f64, f64)) -> bool {
    let (min_lon, min_lat, max_lon, max_lat) = bounds;
    if piece.max_lat < min_lat || piece.min_lat > max_lat {
        return false;
    }
    lon_ranges_overlap(piece.min_lon, piece.max_lon, min_lon, max_lon)
}

fn lon_ranges_overlap(a0: f64, a1: f64, b0: f64, b1: f64) -> bool {
    fn parts(min_lon: f64, max_lon: f64) -> Vec<(f64, f64)> {
        if max_lon - min_lon >= 360.0 {
            return vec![(-180.0, 180.0)];
        }
        let mut lo = min_lon;
        let mut hi = max_lon;
        while lo < -180.0 {
            lo += 360.0;
            hi += 360.0;
        }
        while lo > 180.0 {
            lo -= 360.0;
            hi -= 360.0;
        }
        if hi <= 180.0 {
            vec![(lo, hi)]
        } else {
            vec![(lo, 180.0), (-180.0, hi - 360.0)]
        }
    }
    let left = parts(a0, a1);
    let right = parts(b0, b1);
    left.iter()
        .any(|a| right.iter().any(|b| a.0 <= b.1 && b.0 <= a.1))
}

fn douglas_peucker(points: &[[f64; 2]], epsilon: f64) -> Vec<[f64; 2]> {
    if points.len() < 3 {
        return points.to_vec();
    }
    let mut keep = vec![false; points.len()];
    keep[0] = true;
    keep[points.len() - 1] = true;
    let mut stack = vec![(0usize, points.len() - 1)];
    while let Some((start, end)) = stack.pop() {
        if end <= start + 1 {
            continue;
        }
        let mut max_distance = 0.0;
        let mut max_index = start;
        for index in start + 1..end {
            let distance = point_segment_distance(points[index], points[start], points[end]);
            if distance > max_distance {
                max_distance = distance;
                max_index = index;
            }
        }
        if max_distance > epsilon {
            keep[max_index] = true;
            stack.push((start, max_index));
            stack.push((max_index, end));
        }
    }
    points
        .iter()
        .enumerate()
        .filter(|(index, _)| keep[*index])
        .map(|(_, point)| *point)
        .collect()
}

fn point_segment_distance(point: [f64; 2], start: [f64; 2], end: [f64; 2]) -> f64 {
    let dx = end[0] - start[0];
    let dy = end[1] - start[1];
    let len2 = dx * dx + dy * dy;
    if len2 <= 1e-18 {
        let ex = point[0] - start[0];
        let ey = point[1] - start[1];
        return (ex * ex + ey * ey).sqrt();
    }
    let t = (((point[0] - start[0]) * dx + (point[1] - start[1]) * dy) / len2).clamp(0.0, 1.0);
    let qx = start[0] + t * dx;
    let qy = start[1] + t * dy;
    let ex = point[0] - qx;
    let ey = point[1] - qy;
    (ex * ex + ey * ey).sqrt()
}

fn clip_polyline(points: &[[f64; 2]], rect: [f64; 4]) -> Vec<Vec<[f64; 2]>> {
    let mut lines = Vec::new();
    let mut current = Vec::new();
    for pair in points.windows(2) {
        if let Some((start, end)) = clip_segment(pair[0], pair[1], rect) {
            let continues = current
                .last()
                .is_some_and(|last| points_close(*last, start));
            if !continues {
                if current.len() >= 2 {
                    lines.push(std::mem::take(&mut current));
                } else {
                    current.clear();
                }
                current.push(start);
            }
            push_distinct(&mut current, end);
        } else if current.len() >= 2 {
            lines.push(std::mem::take(&mut current));
        } else {
            current.clear();
        }
    }
    if current.len() >= 2 {
        lines.push(current);
    }
    lines
}

fn clip_segment(mut a: [f64; 2], mut b: [f64; 2], rect: [f64; 4]) -> Option<([f64; 2], [f64; 2])> {
    let mut code_a = out_code(a, rect);
    let mut code_b = out_code(b, rect);
    for _ in 0..8 {
        if code_a | code_b == 0 {
            return Some((a, b));
        }
        if code_a & code_b != 0 {
            return None;
        }
        let code = if code_a != 0 { code_a } else { code_b };
        let dx = b[0] - a[0];
        let dy = b[1] - a[1];
        let point = if code & 8 != 0 {
            let t = if dy.abs() < 1e-15 {
                0.0
            } else {
                (rect[3] - a[1]) / dy
            };
            [a[0] + t * dx, rect[3]]
        } else if code & 4 != 0 {
            let t = if dy.abs() < 1e-15 {
                0.0
            } else {
                (rect[1] - a[1]) / dy
            };
            [a[0] + t * dx, rect[1]]
        } else if code & 2 != 0 {
            let t = if dx.abs() < 1e-15 {
                0.0
            } else {
                (rect[2] - a[0]) / dx
            };
            [rect[2], a[1] + t * dy]
        } else {
            let t = if dx.abs() < 1e-15 {
                0.0
            } else {
                (rect[0] - a[0]) / dx
            };
            [rect[0], a[1] + t * dy]
        };
        if code == code_a {
            a = point;
            code_a = out_code(a, rect);
        } else {
            b = point;
            code_b = out_code(b, rect);
        }
    }
    None
}

fn out_code(point: [f64; 2], rect: [f64; 4]) -> u8 {
    let mut code = 0u8;
    if point[0] < rect[0] {
        code |= 1;
    } else if point[0] > rect[2] {
        code |= 2;
    }
    if point[1] < rect[1] {
        code |= 4;
    } else if point[1] > rect[3] {
        code |= 8;
    }
    code
}

fn encode_collection(features: &[Drawn]) -> Vec<u8> {
    let mut out = String::from("{\"type\":\"FeatureCollection\",\"features\":[");
    for (index, feature) in features.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        out.push_str("{\"type\":\"Feature\",\"properties\":");
        match &feature.props {
            Props::Empty => out.push_str("{}"),
            Props::River {
                id,
                order,
                channel_width_m,
            } => {
                out.push_str("{\"id\":");
                push_json_string(&mut out, id);
                out.push_str(",\"strahler_order\":");
                out.push_str(&order.to_string());
                out.push_str(",\"channel_width_m\":");
                push_metres(&mut out, *channel_width_m);
                out.push('}');
            }
            Props::Contour { elevation_m, index } => {
                out.push_str("{\"elevation_m\":");
                out.push_str(&elevation_m.to_string());
                out.push_str(",\"index\":");
                out.push_str(if *index { "true" } else { "false" });
                out.push('}');
            }
            Props::Stream {
                id,
                order,
                accumulation,
            } => {
                out.push_str("{\"id\":");
                push_json_string(&mut out, id);
                out.push_str(",\"order\":");
                out.push_str(&order.to_string());
                out.push_str(",\"accumulation\":");
                out.push_str(&accumulation.to_string());
                out.push('}');
            }
        }
        out.push_str(",\"geometry\":{\"type\":\"LineString\",\"coordinates\":[");
        for (coord_index, point) in feature.ring.iter().enumerate() {
            if coord_index > 0 {
                out.push(',');
            }
            out.push('[');
            push_coord(&mut out, point[0]);
            out.push(',');
            push_coord(&mut out, point[1]);
            out.push(']');
        }
        out.push_str("]}}");
    }
    out.push_str("]}");
    out.into_bytes()
}

fn push_json_string(out: &mut String, value: &str) {
    out.push('"');
    for ch in value.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            c if c.is_control() => {
                out.push_str(&format!("\\u{:04x}", u32::from(c)));
            }
            c => out.push(c),
        }
    }
    out.push('"');
}

fn push_metres(out: &mut String, metres: f64) {
    let rounded = (metres * 10.0).round() / 10.0;
    if (rounded - rounded.round()).abs() < 1e-6 {
        out.push_str(&format!("{}", rounded.round() as i64));
    } else {
        out.push_str(&format!("{rounded:.1}"));
    }
}

fn push_coord(out: &mut String, value: f64) {
    let micros = round_1e6(value);
    let scaled = (micros * COORD_SCALE).round() as i64;
    let negative = scaled < 0;
    let scaled = scaled.unsigned_abs();
    let whole = scaled / 1_000_000;
    let frac = scaled % 1_000_000;
    if negative {
        out.push('-');
    }
    out.push_str(&whole.to_string());
    if frac == 0 {
        return;
    }
    let mut digits = format!("{frac:06}");
    while digits.ends_with('0') {
        digits.pop();
    }
    out.push('.');
    out.push_str(&digits);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::terrain::terrain_fixture as fixture;

    fn collection(bytes: &[u8]) -> serde_json::Value {
        serde_json::from_slice(bytes).expect("geojson")
    }

    fn feature_list(bytes: &[u8]) -> Vec<serde_json::Value> {
        collection(bytes)["features"]
            .as_array()
            .expect("features")
            .clone()
    }

    fn vectors() -> TerrainVectors {
        TerrainVectors::new(fixture())
    }

    #[test]
    fn vector_tiles_are_deterministic_geojson() {
        let terrain = fixture();
        let vectors = vectors();
        let z = 2u32;
        let (x, y) = (1u32, 1u32);
        for layer in [
            TerrainVectorLayer::Coastline,
            TerrainVectorLayer::Lakes,
            TerrainVectorLayer::Rivers,
            TerrainVectorLayer::Contours,
            TerrainVectorLayer::Streams,
        ] {
            let once = render_terrain_vector_tile(terrain, &vectors, layer, z, x, y).unwrap();
            let twice = render_terrain_vector_tile(terrain, &vectors, layer, z, x, y).unwrap();
            assert_eq!(once, twice, "{}", layer.id());
            let pool = rayon::ThreadPoolBuilder::new()
                .num_threads(1)
                .build()
                .expect("pool");
            let serial = pool
                .install(|| render_terrain_vector_tile(terrain, &vectors, layer, z, x, y).unwrap());
            assert_eq!(once, serial, "{}", layer.id());
            let value = collection(&once);
            assert_eq!(value["type"], "FeatureCollection");
            assert!(value["features"].is_array());
        }
    }

    fn shared_edge_lon(z: u32, x: u32) -> f64 {
        let world = mercator_world_px(z);
        wrap_lon(180.0 * (2.0 * (f64::from(x) + 1.0) * TILE_PX / world - 1.0))
    }

    fn lon_close(left: f64, right: f64) -> bool {
        let mut delta = (left - right).abs() % 360.0;
        if delta > 180.0 {
            delta = 360.0 - delta;
        }
        delta <= 1e-6
    }

    fn endpoints_on_edge(bytes: &[u8], edge_lon: f64) -> Vec<[f64; 2]> {
        let mut found = Vec::new();
        for feature in feature_list(bytes) {
            let coords = feature["geometry"]["coordinates"]
                .as_array()
                .expect("coordinates");
            if coords.len() < 2 {
                continue;
            }
            for index in [0, coords.len() - 1] {
                let lon = coords[index][0].as_f64().unwrap();
                let lat = coords[index][1].as_f64().unwrap();
                if lon_close(lon, edge_lon) {
                    found.push([lon, lat]);
                }
            }
        }
        found
    }

    #[test]
    fn coastline_endpoints_meet_across_horizontal_seams() {
        let terrain = fixture();
        let vectors = vectors();
        let z = 2u32;
        let n = 1u32 << z;
        let mut matched = 0usize;
        for y in 0..n {
            for x in 0..n {
                let east = (x + 1) % n;
                let left = render_terrain_vector_tile(
                    terrain,
                    &vectors,
                    TerrainVectorLayer::Coastline,
                    z,
                    x,
                    y,
                )
                .unwrap();
                let right = render_terrain_vector_tile(
                    terrain,
                    &vectors,
                    TerrainVectorLayer::Coastline,
                    z,
                    east,
                    y,
                )
                .unwrap();
                let edge = shared_edge_lon(z, x);
                let left_ends = endpoints_on_edge(&left, edge);
                let right_ends = endpoints_on_edge(&right, edge);
                for end in &left_ends {
                    assert!(
                        right_ends.iter().any(|other| lon_close(end[0], other[0])
                            && (end[1] - other[1]).abs() <= 1e-6),
                        "z{z} {x}/{y} endpoint {end:?} missing from the eastern tile"
                    );
                    matched += 1;
                }
                for end in &right_ends {
                    assert!(
                        left_ends.iter().any(|other| lon_close(end[0], other[0])
                            && (end[1] - other[1]).abs() <= 1e-6),
                        "z{z} {east}/{y} endpoint {end:?} missing from the western tile"
                    );
                }
            }
        }
        assert!(matched > 0, "no coastline endpoint landed on a shared edge");
    }

    #[test]
    fn coastline_skips_uniform_lattice_cells() {
        let terrain = fixture();
        let vectors = vectors();
        let z = 3u32;
        let (x, y) = coast_tile(terrain, &vectors, z).expect("a coastal tile");
        let bytes =
            render_terrain_vector_tile(terrain, &vectors, TerrainVectorLayer::Coastline, z, x, y)
                .unwrap();
        let nodes = sample_nodes(terrain, z, x, y);
        for feature in feature_list(&bytes) {
            let coords = feature["geometry"]["coordinates"].as_array().unwrap();
            let pixels = coords
                .iter()
                .map(|pair| {
                    lonlat_to_px(pair[0].as_f64().unwrap(), pair[1].as_f64().unwrap(), z, x)
                })
                .collect::<Vec<_>>();
            for pair in pixels.windows(2) {
                let mid = [
                    (pair[0][0] + pair[1][0]) * 0.5,
                    (pair[0][1] + pair[1][1]) * 0.5,
                ];
                if let Some((col, row)) = cell_of(x, y, mid) {
                    let covers = cell_covers(&nodes, col, row);
                    assert!(
                        covers.iter().any(|cover| *cover != covers[0]),
                        "uniform cell {col},{row} contains a coastline segment"
                    );
                }
            }
        }
    }

    fn cell_of(tile_x: u32, tile_y: u32, px: [f64; 2]) -> Option<(usize, usize)> {
        let origin_x = f64::from(tile_x) * TILE_PX - LATTICE_STEP_PX;
        let origin_y = f64::from(tile_y) * TILE_PX - LATTICE_STEP_PX;
        let col_f = (px[0] - origin_x) / LATTICE_STEP_PX;
        let row_f = (px[1] - origin_y) / LATTICE_STEP_PX;
        let col_near = (col_f - col_f.round()).abs() < 1e-4;
        let row_near = (row_f - row_f.round()).abs() < 1e-4;
        if col_near || row_near {
            return None;
        }
        let col = col_f.floor() as isize;
        let row = row_f.floor() as isize;
        if col < 0 || row < 0 || col as usize + 1 >= NODE_SPAN || row as usize + 1 >= NODE_SPAN {
            return None;
        }
        Some((col as usize, row as usize))
    }

    fn cell_covers(nodes: &[Node], col: usize, row: usize) -> [TerrainCover; 4] {
        [
            nodes[row * NODE_SPAN + col].cover,
            nodes[row * NODE_SPAN + col + 1].cover,
            nodes[(row + 1) * NODE_SPAN + col + 1].cover,
            nodes[(row + 1) * NODE_SPAN + col].cover,
        ]
    }

    fn coast_tile(terrain: &AtlasTerrain, vectors: &TerrainVectors, z: u32) -> Option<(u32, u32)> {
        let surface = terrain.surface();
        let none = daena_physical::engine::TOOL_SURFACE_NONE;
        for cell in 0..surface.cell_count() as u32 {
            if !surface.ocean(cell) {
                continue;
            }
            if !surface
                .neighbors(cell)
                .iter()
                .any(|neighbor| *neighbor != none && !surface.ocean(*neighbor))
            {
                continue;
            }
            let (lat, lon) = surface.cell_lat_lon_degrees(cell);
            let (x, y) = super::super::lon_lat_to_tile(lon, lat, z);
            let bytes = render_terrain_vector_tile(
                terrain,
                vectors,
                TerrainVectorLayer::Coastline,
                z,
                x,
                y,
            )
            .unwrap();
            if feature_list(&bytes).is_empty() {
                continue;
            }
            return Some((x, y));
        }
        None
    }

    #[test]
    fn river_filter_clip_and_edge_crossing_hold() {
        let terrain = fixture();
        let vectors = vectors();
        assert!(!vectors.rivers.is_empty(), "fixture has no rivers");
        for z in [0u32, 2, 4, 6, 8, 10, 12] {
            let min_order = min_strahler_order(z);
            let (x, y) = river_tile(&vectors, z, min_order).unwrap_or((0, 0));
            let bytes =
                render_terrain_vector_tile(terrain, &vectors, TerrainVectorLayer::Rivers, z, x, y)
                    .unwrap();
            for feature in feature_list(&bytes) {
                let order = feature["properties"]["strahler_order"].as_u64().unwrap() as u16;
                assert!(order >= min_order, "z{z} emitted order {order}");
                let id = feature["properties"]["id"].as_str().unwrap();
                assert!(id.starts_with("river:c"), "{id}");
                for pair in feature["geometry"]["coordinates"].as_array().unwrap() {
                    let lon = pair[0].as_f64().unwrap();
                    let lat = pair[1].as_f64().unwrap();
                    let px = lonlat_to_px(lon, lat, z, x);
                    assert!(
                        inside_rect(px, expanded_rect(x, y), 0.05),
                        "z{z} river coordinate outside the halo: {lon},{lat}"
                    );
                }
            }
        }
        let cutoff = min_strahler_order(2);
        let river = vectors
            .rivers
            .iter()
            .filter(|river| {
                river.order < cutoff && river.pieces.iter().any(|piece| piece.lonlat.len() >= 2)
            })
            .max_by_key(|river| river.order)
            .expect("fixture has a river below the z2 cutoff");
        if let Some((x, y)) = piece_tile(&river.pieces[0], 2) {
            let low =
                render_terrain_vector_tile(terrain, &vectors, TerrainVectorLayer::Rivers, 2, x, y)
                    .unwrap();
            assert!(
                feature_list(&low).iter().all(|feature| {
                    feature["properties"]["id"].as_str() != Some(river.id.as_str())
                }),
                "order {} river {} passed the z2 filter",
                river.order,
                river.id
            );
        }
        let high_zoom = 12u32;
        if let Some((x, y)) = piece_tile(&river.pieces[0], high_zoom) {
            let high = render_terrain_vector_tile(
                terrain,
                &vectors,
                TerrainVectorLayer::Rivers,
                high_zoom,
                x,
                y,
            )
            .unwrap();
            assert!(
                feature_list(&high).iter().any(|feature| {
                    feature["properties"]["id"].as_str() == Some(river.id.as_str())
                }),
                "order {} river {} missing at z12",
                river.order,
                river.id
            );
        }
        let crossing = vectors
            .rivers
            .iter()
            .filter(|river| river.order >= min_strahler_order(8))
            .find_map(|river| crossing_tiles(river, 8).map(|tiles| (river.id.as_str(), tiles)));
        if let Some((id, ((x0, y0), (x1, y1)))) = crossing {
            for (x, y) in [(x0, y0), (x1, y1)] {
                let bytes = render_terrain_vector_tile(
                    terrain,
                    &vectors,
                    TerrainVectorLayer::Rivers,
                    8,
                    x,
                    y,
                )
                .unwrap();
                assert!(
                    feature_list(&bytes)
                        .iter()
                        .any(|feature| feature["properties"]["id"].as_str() == Some(id)),
                    "{id} missing from {x}/{y} at z8"
                );
            }
        }
    }

    fn river_tile(vectors: &TerrainVectors, z: u32, min_order: u16) -> Option<(u32, u32)> {
        let river = vectors
            .rivers
            .iter()
            .find(|river| river.order >= min_order && !river.pieces.is_empty())?;
        piece_tile(&river.pieces[0], z)
    }

    fn piece_tile(piece: &RiverPiece, z: u32) -> Option<(u32, u32)> {
        let point = piece.lonlat.first()?;
        Some(super::super::lon_lat_to_tile(point[0], point[1], z))
    }

    fn crossing_tiles(river: &PreparedRiver, z: u32) -> Option<((u32, u32), (u32, u32))> {
        let n = 1u32 << z;
        for piece in &river.pieces {
            let pixels = piece
                .lonlat
                .windows(2)
                .flat_map(|pair| {
                    let a = lonlat_to_px(pair[0][0], pair[0][1], z, 0);
                    let b = lonlat_to_px(pair[1][0], pair[1][1], z, 0);
                    let steps = ((a[0] - b[0]).abs().max((a[1] - b[1]).abs()) / 32.0)
                        .ceil()
                        .max(1.0) as usize;
                    (0..=steps).map(move |step| {
                        let t = step as f64 / steps as f64;
                        [a[0] + t * (b[0] - a[0]), a[1] + t * (b[1] - a[1])]
                    })
                })
                .collect::<Vec<_>>();
            let mut previous = None;
            for px in pixels {
                let tile = tile_of_px(px, z, n);
                if let Some(prev) = previous {
                    if prev != tile && tiles_share_edge(prev, tile, n) {
                        return Some((prev, tile));
                    }
                }
                previous = Some(tile);
            }
        }
        None
    }

    fn tile_of_px(px: [f64; 2], z: u32, n: u32) -> (u32, u32) {
        let world = mercator_world_px(z);
        let gx = px[0].rem_euclid(world);
        let gy = px[1].clamp(0.0, world - 1e-6);
        let x = (gx / TILE_PX).floor() as u32;
        let y = (gy / TILE_PX).floor() as u32;
        (x.min(n - 1), y.min(n - 1))
    }

    fn tiles_share_edge(a: (u32, u32), b: (u32, u32), n: u32) -> bool {
        let dx = a.0.abs_diff(b.0).min(n - a.0.abs_diff(b.0));
        let dy = a.1.abs_diff(b.1);
        (dx == 1 && dy == 0) || (dx == 0 && dy == 1)
    }

    #[test]
    fn contour_intervals_follow_zoom() {
        let terrain = fixture();
        let vectors = vectors();
        let (lat, lon) = terrain
            .surface()
            .cell_lat_lon_degrees(highest_land(terrain));
        for z in [6u32, 8, 10, 12] {
            let interval = contour_interval_m(z);
            let (x, y) = super::super::lon_lat_to_tile(lon, lat, z);
            let bytes = render_terrain_vector_tile(
                terrain,
                &vectors,
                TerrainVectorLayer::Contours,
                z,
                x,
                y,
            )
            .unwrap();
            let features = feature_list(&bytes);
            assert!(!features.is_empty(), "z{z} contour tile is empty");
            for feature in features {
                let elevation = feature["properties"]["elevation_m"].as_i64().unwrap() as i32;
                assert_eq!(
                    elevation.rem_euclid(interval),
                    0,
                    "z{z} elevation {elevation}"
                );
                let index = feature["properties"]["index"].as_bool().unwrap();
                let expected = elevation.rem_euclid(interval.saturating_mul(5)) == 0;
                assert_eq!(index, expected, "z{z} index of {elevation}");
            }
        }
    }

    fn highest_land(terrain: &AtlasTerrain) -> u32 {
        let surface = terrain.surface();
        let mut best = 0u32;
        let mut best_height = f64::MIN;
        for cell in 0..surface.cell_count() as u32 {
            if surface.ocean(cell) {
                continue;
            }
            let height = f64::from(terrain.elevation_m()[cell as usize]);
            if height > best_height {
                best_height = height;
                best = cell;
            }
        }
        best
    }

    #[test]
    fn vector_tiles_reject_bad_indexes() {
        let terrain = fixture();
        let vectors = vectors();
        for (z, x, y) in [(13u32, 0, 0), (2, 4, 0), (0, 0, 1), (1, 0, 2)] {
            let error =
                render_terrain_vector_tile(terrain, &vectors, TerrainVectorLayer::Rivers, z, x, y)
                    .unwrap_err();
            assert_eq!(error.code, crate::CODE_REQUEST_INVALID);
        }
    }

    #[test]
    fn layer_ids_round_trip() {
        for layer in [
            TerrainVectorLayer::Coastline,
            TerrainVectorLayer::Lakes,
            TerrainVectorLayer::Rivers,
            TerrainVectorLayer::Contours,
            TerrainVectorLayer::Streams,
        ] {
            assert_eq!(TerrainVectorLayer::from_id(layer.id()), Some(layer));
        }
        assert_eq!(TerrainVectorLayer::from_id("foo"), None);
    }

    #[test]
    fn streams_are_empty_below_zoom_8() {
        let terrain = fixture();
        let vectors = vectors();
        let (lat, lon) = terrain
            .surface()
            .cell_lat_lon_degrees(highest_land(terrain));
        let (x, y) = super::super::lon_lat_to_tile(lon, lat, 7);
        let bytes =
            render_terrain_vector_tile(terrain, &vectors, TerrainVectorLayer::Streams, 7, x, y)
                .unwrap();
        assert_eq!(bytes, br#"{"type":"FeatureCollection","features":[]}"#,);
    }

    #[test]
    fn stream_endpoints_meet_across_a_shared_edge() {
        let terrain = fixture();
        let vectors = vectors();
        let surface = terrain.surface();
        let z = 11u32;
        let n = 1u32 << z;
        let mut matched = 0usize;
        let mut tried = 0usize;
        for cell in 0..surface.cell_count() as u32 {
            if !streams::is_stream_cell(terrain, cell)
                || f64::from(terrain.moisture[cell as usize]) < 0.3
            {
                continue;
            }
            tried += 1;
            if tried > 24 {
                break;
            }
            let (lat, lon) = surface.cell_lat_lon_degrees(cell);
            let (x, y) = super::super::lon_lat_to_tile(lon, lat, z);
            let before = matched;
            for (dx, dy) in [(1i32, 0), (0, 1)] {
                let east = (x as i32 + dx).rem_euclid(n as i32) as u32;
                let north = (y as i32 + dy).clamp(0, n as i32 - 1) as u32;
                if dy != 0 && north == y {
                    continue;
                }
                let left = render_terrain_vector_tile(
                    terrain,
                    &vectors,
                    TerrainVectorLayer::Streams,
                    z,
                    x,
                    y,
                )
                .unwrap();
                let right = render_terrain_vector_tile(
                    terrain,
                    &vectors,
                    TerrainVectorLayer::Streams,
                    z,
                    east,
                    north,
                )
                .unwrap();
                if dx != 0 {
                    let edge = shared_edge_lon(z, x);
                    let left_ends = endpoints_on_edge(&left, edge);
                    let right_ends = endpoints_on_edge(&right, edge);
                    for end in &left_ends {
                        assert!(
                            right_ends.iter().any(|other| lon_close(end[0], other[0])
                                && (end[1] - other[1]).abs() <= 1e-6),
                            "stream endpoint {end:?} missing across {x}/{y}"
                        );
                        matched += 1;
                    }
                    for end in &right_ends {
                        assert!(
                            left_ends.iter().any(|other| lon_close(end[0], other[0])
                                && (end[1] - other[1]).abs() <= 1e-6),
                            "stream endpoint {end:?} missing from the western tile"
                        );
                    }
                }
                if dy != 0 {
                    let edge_lat = shared_edge_lat(z, y);
                    let left_ends = endpoints_on_lat(&left, edge_lat);
                    let right_ends = endpoints_on_lat(&right, edge_lat);
                    for end in &left_ends {
                        assert!(
                            right_ends
                                .iter()
                                .any(|other| (end[0] - other[0]).abs() <= 1e-6
                                    && (end[1] - other[1]).abs() <= 1e-6),
                            "stream endpoint {end:?} missing across the southern edge"
                        );
                        matched += 1;
                    }
                    for end in &right_ends {
                        assert!(
                            left_ends
                                .iter()
                                .any(|other| (end[0] - other[0]).abs() <= 1e-6
                                    && (end[1] - other[1]).abs() <= 1e-6),
                            "stream endpoint {end:?} missing from the northern tile"
                        );
                    }
                }
            }
            if matched > before {
                break;
            }
        }
        assert!(
            matched > 0,
            "no stream endpoint landed on a shared edge in {tried} cells"
        );
    }

    fn shared_edge_lat(z: u32, y: u32) -> f64 {
        let world = mercator_world_px(z);
        mercator_latitude((f64::from(y) + 1.0) * TILE_PX, world).to_degrees()
    }

    fn endpoints_on_lat(bytes: &[u8], edge_lat: f64) -> Vec<[f64; 2]> {
        let mut found = Vec::new();
        for feature in feature_list(bytes) {
            let coords = feature["geometry"]["coordinates"]
                .as_array()
                .expect("coordinates");
            if coords.len() < 2 {
                continue;
            }
            for index in [0, coords.len() - 1] {
                let lon = coords[index][0].as_f64().unwrap();
                let lat = coords[index][1].as_f64().unwrap();
                if (lat - edge_lat).abs() <= 1e-6 {
                    found.push([lon, lat]);
                }
            }
        }
        found
    }

    #[test]
    #[ignore]
    fn vector_tile_timing_n512() {
        use std::sync::Arc;
        use std::time::Instant;

        use daena_physical::engine::{self, Settings, WorldPreset};
        use daena_physical::{NoopProgress, DEFAULT_RADIUS_METRES};

        fn build(resolution: u32) -> (AtlasTerrain, TerrainVectors, f64) {
            let settings = Settings::from_parts(
                831_429,
                WorldPreset::Earthlike,
                resolution,
                DEFAULT_RADIUS_METRES,
                0,
                daena_physical::planetary::PlanetaryConfiguration::earth_like(),
            );
            let started = Instant::now();
            let products = engine::generate(settings, &mut NoopProgress).expect("world");
            let world_ms = started.elapsed().as_secs_f64() * 1000.0;
            let decoded = engine::decode_source(products.source()).expect("decode");
            let surface = decoded
                .derive_tool_surface_present(&mut NoopProgress)
                .expect("surface");
            let identity = crate::spike_identity_from_source(products.source());
            let terrain = AtlasTerrain::new(Arc::new(surface), &identity);
            let started = Instant::now();
            let vectors = TerrainVectors::new(&terrain);
            let build_ms = started.elapsed().as_secs_f64() * 1000.0;
            println!(
                "TerrainVectors::new n={resolution} {build_ms:.3} ms storage {} bytes geometry {} bytes (world gen {world_ms:.0} ms, rivers {})",
                vectors.storage_bytes(),
                terrain.river_geometry_bytes(),
                vectors.rivers.len()
            );
            if resolution >= 1024 {
                let bytes = terrain.river_geometry_bytes();
                assert!(
                    bytes <= 64 * 1024 * 1024,
                    "river geometry n={resolution} is {bytes} bytes"
                );
            }
            let mut counts = BTreeMap::<u16, usize>::new();
            for river in &vectors.rivers {
                *counts.entry(river.order).or_default() += 1;
            }
            println!("river orders n={resolution} {counts:?}");
            (terrain, vectors, build_ms)
        }

        let (terrain, vectors, _) = build(512);
        let _ =
            render_terrain_vector_tile(&terrain, &vectors, TerrainVectorLayer::Coastline, 12, 0, 0);
        let n = 1u32 << 12;
        // The auto site is lowland; the highest land cell is the contour worst case.
        for (site, cell) in [
            ("auto", terrain.auto_site_cell()),
            ("highest", highest_land(&terrain)),
        ] {
            let (lat, lon) = terrain.surface().cell_lat_lon_degrees(cell);
            let (x, y) = super::super::lon_lat_to_tile(lon, lat, 12);
            for layer in [
                TerrainVectorLayer::Coastline,
                TerrainVectorLayer::Lakes,
                TerrainVectorLayer::Rivers,
                TerrainVectorLayer::Contours,
            ] {
                let mut times = Vec::with_capacity(9);
                for dy in -1i32..=1 {
                    for dx in -1i32..=1 {
                        let tile_x = (x as i32 + dx).rem_euclid(n as i32) as u32;
                        let tile_y = (y as i32 + dy).clamp(0, n as i32 - 1) as u32;
                        let started = Instant::now();
                        let _bytes = render_terrain_vector_tile(
                            &terrain, &vectors, layer, 12, tile_x, tile_y,
                        )
                        .unwrap();
                        times.push(started.elapsed().as_secs_f64() * 1000.0);
                    }
                }
                times.sort_by(f64::total_cmp);
                let median = times[4];
                let worst = times[8];
                println!(
                    "vector {site} {} z12 median {median:.3} ms worst {worst:.3} ms",
                    layer.id()
                );
                assert!(
                    median <= 15.0,
                    "{site} {} z12 median {median:.3} ms worst {worst:.3} ms",
                    layer.id()
                );
            }
            for z in [8u32, 12] {
                let n_z = 1u32 << z;
                let (x, y) = super::super::lon_lat_to_tile(lon, lat, z);
                let mut times = Vec::with_capacity(9);
                for dy in -1i32..=1 {
                    for dx in -1i32..=1 {
                        let tile_x = (x as i32 + dx).rem_euclid(n_z as i32) as u32;
                        let tile_y = (y as i32 + dy).clamp(0, n_z as i32 - 1) as u32;
                        streams::clear_cache(&terrain);
                        let started = Instant::now();
                        let _bytes = render_terrain_vector_tile(
                            &terrain,
                            &vectors,
                            TerrainVectorLayer::Streams,
                            z,
                            tile_x,
                            tile_y,
                        )
                        .unwrap();
                        times.push(started.elapsed().as_secs_f64() * 1000.0);
                    }
                }
                times.sort_by(f64::total_cmp);
                let median = times[4];
                let worst = times[8];
                println!(
                    "vector {site} streams z{z} cold median {median:.3} ms worst {worst:.3} ms"
                );
                assert!(
                    median <= 15.0,
                    "{site} streams z{z} cold median {median:.3} ms worst {worst:.3} ms"
                );
            }
        }
        drop(terrain);
        drop(vectors);

        let (terrain, vectors, build_ms) = build(1024);
        assert!(
            build_ms <= 200.0,
            "TerrainVectors::new n=1024 took {build_ms:.3} ms"
        );
        let (max_segs, nonempty, listed) = terrain.rivers.segment_list_stats();
        let mean = if nonempty == 0 {
            0.0
        } else {
            listed as f64 / f64::from(nonempty)
        };
        println!("n=1024 index max {max_segs} nonempty {nonempty} mean {mean:.2}");
        let (lon, lat) = terrain
            .river_site(None)
            .expect("world has no river of order 4 or higher");
        let (x, y) = super::super::lon_lat_to_tile(lon, lat, 12);
        let mut times = Vec::with_capacity(9);
        for _ in 0..9 {
            let started = Instant::now();
            let bytes = render_terrain_vector_tile(
                &terrain,
                &vectors,
                TerrainVectorLayer::Rivers,
                12,
                x,
                y,
            )
            .expect("river tile");
            assert!(!bytes.is_empty());
            times.push(started.elapsed().as_secs_f64() * 1000.0);
        }
        times.sort_by(f64::total_cmp);
        let median = times[4];
        let worst = times[8];
        println!(
            "vector n=1024 river-site rivers z12 median {median:.3} ms worst {worst:.3} ms at {lon:.4},{lat:.4} tile {x}/{y}"
        );
        assert!(
            median <= 15.0,
            "n=1024 river-site rivers z12 median {median:.3} ms worst {worst:.3} ms"
        );
        let mut imagery = Vec::with_capacity(9);
        for _ in 0..9 {
            let started = Instant::now();
            let _pixels =
                super::super::render_imagery_pixels(&terrain, 12, x * 256, y * 256, 256, 256, 2);
            imagery.push(started.elapsed().as_secs_f64() * 1000.0);
        }
        imagery.sort_by(f64::total_cmp);
        println!(
            "imagery n=1024 river-site z12 median {:.3} ms worst {:.3} ms",
            imagery[4], imagery[8]
        );
    }
}
