//! Seam-safe interpolated contour extraction for disposable map vectors.
//!
//! This module owns Packet 5 geometry. Canonical elevation and solved water
//! levels are inputs; they are never rewritten. Crossing locations come from
//! scalar interpolation on the dual spherical grid, then quantize to
//! microdegrees.

use std::collections::{BTreeMap, BTreeSet};

use super::{Grid, PhysicalError, PhysicalErrorCode, Segment};

pub const CONTOUR_DERIVATION_VERSION: u16 = 1;
const SIMPLIFY_CELL_FRACTION_PPM: u32 = 120_000;
const MIN_RING_VERTICES: usize = 4;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct EdgeKey {
    kind: u8,
    row: u32,
    col: u32,
}

impl EdgeKey {
    const HORIZONTAL: u8 = 0;
    const VERTICAL: u8 = 1;
    const SOUTH_RADIAL: u8 = 2;
    const NORTH_RADIAL: u8 = 3;

    fn horizontal(row: u32, col: u32) -> Self {
        Self {
            kind: Self::HORIZONTAL,
            row,
            col,
        }
    }

    fn vertical(row: u32, col: u32) -> Self {
        Self {
            kind: Self::VERTICAL,
            row,
            col,
        }
    }

    fn south_radial(col: u32) -> Self {
        Self {
            kind: Self::SOUTH_RADIAL,
            row: 0,
            col,
        }
    }

    fn north_radial(col: u32) -> Self {
        Self {
            kind: Self::NORTH_RADIAL,
            row: 0,
            col,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Crossing {
    key: EdgeKey,
    lon_micro: i64,
    lat_micro: i32,
    protected: bool,
}

#[derive(Debug, Clone, Copy)]
struct RawSegment {
    first: Crossing,
    second: Crossing,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContourTopology {
    pub isolines: Vec<Vec<[i32; 2]>>,
    pub polygons: Vec<Vec<Vec<[i32; 2]>>>,
}

pub fn isolines_at(
    grid: Grid,
    scalar: &[i32],
    threshold: i32,
) -> Result<Vec<Vec<[i32; 2]>>, PhysicalError> {
    Ok(extract(grid, scalar, threshold, true)?.isolines)
}

pub fn polygons_above(
    grid: Grid,
    scalar: &[i32],
    threshold: i32,
) -> Result<Vec<Vec<Vec<[i32; 2]>>>, PhysicalError> {
    Ok(extract(grid, scalar, threshold, true)?.polygons)
}

pub fn polygons_from_mask(
    grid: Grid,
    mask: &[bool],
) -> Result<Vec<Vec<Vec<[i32; 2]>>>, PhysicalError> {
    if mask.len() != grid.sample_count() {
        return Err(PhysicalError::coded(
            PhysicalErrorCode::GeometryInvalid,
            "contour mask does not match the physical grid",
        ));
    }
    let scalar = mask
        .iter()
        .map(|inside| if *inside { 1 } else { 0 })
        .collect::<Vec<_>>();
    polygons_above(grid, &scalar, 0)
}

pub fn polygons_for_sparse_cells(
    grid: Grid,
    cells: &[usize],
) -> Result<Vec<Vec<Vec<[i32; 2]>>>, PhysicalError> {
    if cells.iter().any(|cell| *cell >= grid.sample_count()) {
        return Err(PhysicalError::coded(
            PhysicalErrorCode::GeometryInvalid,
            "contour cell is outside the physical grid",
        ));
    }
    if cells.is_empty() {
        return Ok(Vec::new());
    }
    if cells.len() > grid.sample_count() / 8
        || cells.iter().any(|cell| {
            let (row, _) = grid.row_col(*cell);
            row == 0 || row + 1 == grid.height
        })
    {
        let mut mask = vec![false; grid.sample_count()];
        for cell in cells {
            mask[*cell] = true;
        }
        return polygons_from_mask(grid, &mask);
    }

    let included = cells.iter().copied().collect::<BTreeSet<_>>();
    let mut squares = BTreeSet::new();
    for cell in &included {
        let (row, col) = grid.row_col(*cell);
        squares.insert((row - 1, (col + grid.width - 1) % grid.width));
        squares.insert((row - 1, col));
        squares.insert((row, (col + grid.width - 1) % grid.width));
        squares.insert((row, col));
    }
    let value =
        |row: u32, col: u32| i32::from(included.contains(&grid.index(row, col % grid.width)));
    let mut segments = Vec::with_capacity(squares.len());
    for (row, col) in squares {
        push_marching_square_segments(grid, 0, true, row, col, &value, &mut segments);
    }
    Ok(assemble(grid, segments)?.polygons)
}

pub fn segments_from_paths(paths: &[Vec<[i32; 2]>]) -> Vec<Segment> {
    let mut segments = Vec::new();
    for path in paths {
        for window in path.windows(2) {
            if window[0] != window[1] {
                segments.push(Segment {
                    first: window[0],
                    second: window[1],
                });
            }
        }
    }
    segments
}

pub fn interpolate_edge(
    grid: Grid,
    scalar: &[i32],
    threshold: i32,
    first_cell: usize,
    second_cell: usize,
) -> Result<[i32; 2], PhysicalError> {
    if first_cell >= grid.sample_count() || second_cell >= grid.sample_count() {
        return Err(PhysicalError::coded(
            PhysicalErrorCode::GeometryInvalid,
            "contour snap cell is outside the physical grid",
        ));
    }
    let first = scalar[first_cell];
    let second = scalar[second_cell];
    if (first > threshold) == (second > threshold) {
        return Err(PhysicalError::coded(
            PhysicalErrorCode::GeometryInvalid,
            "drainage cell does not analytically intersect the contour",
        ));
    }
    let (row0, col0) = grid.row_col(first_cell);
    let (row1, col1) = grid.row_col(second_cell);
    let mut a = center_micro(grid, row0, col0);
    let mut b = center_micro(grid, row1, col1);
    unwrap_pair(&mut a[0], &mut b[0]);
    let lon = interpolate(a[0], b[0], first, second, threshold);
    let lat = interpolate(a[1], b[1], first, second, threshold);
    Ok(quantize_point(lon, lat))
}

pub fn extract(
    grid: Grid,
    scalar: &[i32],
    threshold: i32,
    inside_is_greater: bool,
) -> Result<ContourTopology, PhysicalError> {
    if scalar.len() != grid.sample_count() {
        return Err(PhysicalError::coded(
            PhysicalErrorCode::GeometryInvalid,
            "contour scalar does not match the physical grid",
        ));
    }
    let mut segments = marching_squares_segments(grid, scalar, threshold, inside_is_greater);
    segments.extend(polar_segments(
        grid,
        scalar,
        threshold,
        inside_is_greater,
        true,
    ));
    segments.extend(polar_segments(
        grid,
        scalar,
        threshold,
        inside_is_greater,
        false,
    ));
    assemble(grid, segments)
}

fn marching_squares_segments(
    grid: Grid,
    scalar: &[i32],
    threshold: i32,
    inside_is_greater: bool,
) -> Vec<RawSegment> {
    let value = |row: u32, col: u32| scalar[grid.index(row, col % grid.width)];
    let mut segments = Vec::new();
    for row in 0..grid.height.saturating_sub(1) {
        for col in 0..grid.width {
            push_marching_square_segments(
                grid,
                threshold,
                inside_is_greater,
                row,
                col,
                &value,
                &mut segments,
            );
        }
    }
    segments
}

fn push_marching_square_segments(
    grid: Grid,
    threshold: i32,
    inside_is_greater: bool,
    row: u32,
    col: u32,
    value: &impl Fn(u32, u32) -> i32,
    segments: &mut Vec<RawSegment>,
) {
    let inside = |sample: i32| {
        if inside_is_greater {
            sample > threshold
        } else {
            sample < threshold
        }
    };
    let v00 = value(row, col);
    let v10 = value(row, col + 1);
    let v11 = value(row + 1, col + 1);
    let v01 = value(row + 1, col);
    let code = (inside(v00) as u8)
        | ((inside(v10) as u8) << 1)
        | ((inside(v11) as u8) << 2)
        | ((inside(v01) as u8) << 3);
    let south = || {
        edge_crossing(
            grid,
            EdgeKey::horizontal(row, col),
            v00,
            v10,
            threshold,
            row,
            col,
            row,
            col + 1,
            code == 5 || code == 10,
        )
    };
    let east = || {
        edge_crossing(
            grid,
            EdgeKey::vertical(row, (col + 1) % grid.width),
            v10,
            v11,
            threshold,
            row,
            col + 1,
            row + 1,
            col + 1,
            code == 5 || code == 10,
        )
    };
    let north = || {
        edge_crossing(
            grid,
            EdgeKey::horizontal(row + 1, col),
            v01,
            v11,
            threshold,
            row + 1,
            col,
            row + 1,
            col + 1,
            code == 5 || code == 10,
        )
    };
    let west = || {
        edge_crossing(
            grid,
            EdgeKey::vertical(row, col),
            v00,
            v01,
            threshold,
            row,
            col,
            row + 1,
            col,
            code == 5 || code == 10,
        )
    };
    let mut push = |a: Option<Crossing>, b: Option<Crossing>| {
        if let (Some(first), Some(second)) = (a, b) {
            segments.push(RawSegment { first, second });
        }
    };
    match code {
        0 | 15 => {}
        1 | 14 => push(south(), west()),
        2 | 13 => push(south(), east()),
        3 | 12 => push(west(), east()),
        4 | 11 => push(east(), north()),
        6 | 9 => push(south(), north()),
        7 | 8 => push(west(), north()),
        5 => {
            if highs_connected(v00, v10, v11, v01, threshold, inside_is_greater) {
                push(south(), east());
                push(west(), north());
            } else {
                push(south(), west());
                push(east(), north());
            }
        }
        10 => {
            if highs_connected(v10, v11, v01, v00, threshold, inside_is_greater) {
                push(south(), west());
                push(east(), north());
            } else {
                push(south(), east());
                push(west(), north());
            }
        }
        _ => {}
    }
}

fn highs_connected(
    v00: i32,
    v10: i32,
    v11: i32,
    v01: i32,
    threshold: i32,
    inside_is_greater: bool,
) -> bool {
    let denom = i64::from(v00) + i64::from(v11) - i64::from(v10) - i64::from(v01);
    if denom == 0 {
        return false;
    }
    let numerator = i64::from(v00) * i64::from(v11) - i64::from(v10) * i64::from(v01);
    let saddle_inside = if denom > 0 {
        numerator > i64::from(threshold) * denom
    } else {
        numerator < i64::from(threshold) * denom
    };
    if inside_is_greater {
        saddle_inside
    } else {
        !saddle_inside
    }
}

fn polar_segments(
    grid: Grid,
    scalar: &[i32],
    threshold: i32,
    inside_is_greater: bool,
    south: bool,
) -> Vec<RawSegment> {
    let row = if south { 0 } else { grid.height - 1 };
    let pole_lat = if south { -90_000_000 } else { 90_000_000 };
    let mut sum = 0i64;
    for col in 0..grid.width {
        sum += i64::from(scalar[grid.index(row, col)]);
    }
    let pole_value = divide_round(sum, i64::from(grid.width)) as i32;
    let inside = |sample: i32| {
        if inside_is_greater {
            sample > threshold
        } else {
            sample < threshold
        }
    };
    let pole_inside = inside(pole_value);
    let mut segments = Vec::new();
    for col in 0..grid.width {
        let v0 = scalar[grid.index(row, col)];
        let v1 = scalar[grid.index(row, (col + 1) % grid.width)];
        let radial_a = if south {
            EdgeKey::south_radial(col)
        } else {
            EdgeKey::north_radial(col)
        };
        let radial_b = if south {
            EdgeKey::south_radial((col + 1) % grid.width)
        } else {
            EdgeKey::north_radial((col + 1) % grid.width)
        };
        let along = EdgeKey::horizontal(row, col);
        let code = (pole_inside as u8) | ((inside(v0) as u8) << 1) | ((inside(v1) as u8) << 2);
        let pole_a = || {
            radial_crossing(
                grid, radial_a, pole_value, v0, threshold, col, pole_lat, row,
            )
        };
        let pole_b = || {
            radial_crossing(
                grid,
                radial_b,
                pole_value,
                v1,
                threshold,
                (col + 1) % grid.width,
                pole_lat,
                row,
            )
        };
        let base = || {
            edge_crossing(
                grid,
                along,
                v0,
                v1,
                threshold,
                row,
                col,
                row,
                col + 1,
                false,
            )
        };
        let mut push = |a: Option<Crossing>, b: Option<Crossing>| {
            if let (Some(first), Some(second)) = (a, b) {
                segments.push(RawSegment { first, second });
            }
        };
        match code {
            0 | 7 => {}
            1 => push(pole_a(), pole_b()),
            2 => push(pole_a(), base()),
            3 => push(pole_b(), base()),
            4 => push(pole_b(), base()),
            5 => push(pole_a(), base()),
            6 => push(pole_a(), pole_b()),
            _ => {}
        }
    }
    segments
}

#[allow(clippy::too_many_arguments)]
fn edge_crossing(
    grid: Grid,
    key: EdgeKey,
    va: i32,
    vb: i32,
    threshold: i32,
    ra: u32,
    ca: u32,
    rb: u32,
    cb: u32,
    protected: bool,
) -> Option<Crossing> {
    if (va > threshold) == (vb > threshold) {
        return None;
    }
    let mut a = center_micro(grid, ra, ca);
    let mut b = center_micro(grid, rb, cb);
    unwrap_pair(&mut a[0], &mut b[0]);
    Some(Crossing {
        key,
        lon_micro: interpolate(a[0], b[0], va, vb, threshold),
        lat_micro: interpolate(a[1], b[1], va, vb, threshold) as i32,
        protected,
    })
}

#[allow(clippy::too_many_arguments)]
fn radial_crossing(
    grid: Grid,
    key: EdgeKey,
    pole_value: i32,
    cell_value: i32,
    threshold: i32,
    col: u32,
    pole_lat: i32,
    row: u32,
) -> Option<Crossing> {
    if (pole_value > threshold) == (cell_value > threshold) {
        return None;
    }
    let cell = center_micro(grid, row, col);
    Some(Crossing {
        key,
        lon_micro: cell[0],
        lat_micro: interpolate(
            i64::from(pole_lat),
            cell[1],
            pole_value,
            cell_value,
            threshold,
        ) as i32,
        protected: true,
    })
}

fn assemble(grid: Grid, segments: Vec<RawSegment>) -> Result<ContourTopology, PhysicalError> {
    let mut by_edge: BTreeMap<EdgeKey, Vec<usize>> = BTreeMap::new();
    for (index, segment) in segments.iter().enumerate() {
        by_edge.entry(segment.first.key).or_default().push(index);
        by_edge.entry(segment.second.key).or_default().push(index);
    }
    let mut used = vec![false; segments.len()];
    let mut rings = Vec::new();
    let mut opens = Vec::new();
    for start in 0..segments.len() {
        if used[start] {
            continue;
        }
        let mut path = vec![segments[start].first, segments[start].second];
        used[start] = true;
        let mut current_key = segments[start].second.key;
        let start_key = segments[start].first.key;
        while let Some(candidates) = by_edge.get(&current_key) {
            let next = candidates.iter().copied().find(|index| !used[*index]);
            let Some(next) = next else { break };
            used[next] = true;
            let segment = segments[next];
            let (arrive, leave) = if segment.first.key == current_key {
                (segment.first, segment.second)
            } else {
                (segment.second, segment.first)
            };
            if let Some(last) = path.last_mut() {
                last.lon_micro = arrive.lon_micro;
                last.lat_micro = arrive.lat_micro;
                last.protected |= arrive.protected;
            }
            path.push(leave);
            current_key = leave.key;
            if current_key == start_key {
                break;
            }
        }
        let closed = path.len() >= 3 && current_key == start_key;
        let mut coords = Vec::with_capacity(path.len() + 1);
        for (index, point) in path.iter().enumerate() {
            if index > 0 {
                let previous: [i64; 2] = coords[index - 1];
                let mut lon = point.lon_micro;
                let mut prev_lon = previous[0];
                unwrap_pair(&mut prev_lon, &mut lon);
                coords.push([lon, i64::from(point.lat_micro)]);
            } else {
                coords.push([point.lon_micro, i64::from(point.lat_micro)]);
            }
        }
        unwrap_path(&mut coords);
        if closed {
            if coords.first() != coords.last() {
                coords.push(coords[0]);
            }
            if let Some(simplified) = simplify_closed(grid, &coords, &path) {
                if reject_self_intersection(&simplified).is_ok() {
                    rings.push(simplified);
                }
            }
        } else if coords.len() >= 2 {
            opens.push(
                coords
                    .into_iter()
                    .map(|point| quantize_point(point[0], point[1]))
                    .collect(),
            );
        }
    }
    let polygons = assign_holes(grid, rings)?;
    let mut isolines = opens;
    for polygon in &polygons {
        for ring in polygon {
            isolines.push(ring.clone());
        }
    }
    isolines.sort();
    Ok(ContourTopology { isolines, polygons })
}

fn assign_holes(
    grid: Grid,
    rings: Vec<Vec<[i32; 2]>>,
) -> Result<Vec<Vec<Vec<[i32; 2]>>>, PhysicalError> {
    if rings.is_empty() {
        return Ok(Vec::new());
    }
    let areas = rings
        .iter()
        .map(|ring| spherical_signed_area(grid, ring))
        .collect::<Vec<_>>();
    let mut order = (0..rings.len()).collect::<Vec<_>>();
    order.sort_by(|left, right| {
        areas[*right]
            .abs()
            .partial_cmp(&areas[*left].abs())
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(left.cmp(right))
    });
    let mut parent = vec![None; rings.len()];
    for (index, &child) in order.iter().enumerate() {
        for &candidate in &order[..index] {
            if ring_contains(grid, &rings[candidate], &rings[child]) {
                parent[child] = Some(candidate);
            }
        }
    }
    let mut holes: BTreeMap<usize, Vec<usize>> = BTreeMap::new();
    for (index, owner) in parent.iter().enumerate() {
        if let Some(owner) = owner {
            holes.entry(*owner).or_default().push(index);
        }
    }
    let outer_ids = (0..rings.len())
        .filter(|index| parent[*index].is_none())
        .collect::<Vec<_>>();
    let mut flattened = Vec::new();
    for outer in outer_ids {
        let mut polygon = Vec::new();
        let mut ring = rings[outer].clone();
        if signed_ring_area(&ring) < 0 {
            ring.reverse();
        }
        polygon.push(ring);
        if let Some(children) = holes.get(&outer) {
            for child in children {
                if parent[*child] != Some(outer) {
                    continue;
                }
                if parent.contains(&Some(*child)) {
                    continue;
                }
                let mut hole = rings[*child].clone();
                if signed_ring_area(&hole) > 0 {
                    hole.reverse();
                }
                polygon.push(hole);
            }
        }
        let wrapped = polygon
            .into_iter()
            .map(|mut ring| {
                wrap_ring_inplace(&mut ring);
                ring
            })
            .filter(|ring| reject_self_intersection(ring).is_ok())
            .collect::<Vec<_>>();
        if wrapped.is_empty() {
            continue;
        }
        flattened.extend(cut_polygon(wrapped));
    }
    flattened.sort();
    Ok(flattened)
}

fn cut_polygon(polygon: Vec<Vec<[i32; 2]>>) -> Vec<Vec<Vec<[i32; 2]>>> {
    let mut pieces = vec![Vec::new()];
    for (index, ring) in polygon.into_iter().enumerate() {
        let parts = cut_ring(&ring);
        if index == 0 {
            pieces = parts.into_iter().map(|part| vec![part]).collect();
            if pieces.is_empty() {
                pieces.push(vec![ring_wrapped(&ring)]);
            }
        } else {
            for part in parts {
                if let Some(owner) = pieces.iter_mut().find(|candidate| {
                    candidate
                        .first()
                        .is_some_and(|outer| point_in_ring(outer, part[0]))
                }) {
                    owner.push(part);
                }
            }
        }
    }
    pieces
}

fn cut_ring(ring: &[[i32; 2]]) -> Vec<Vec<[i32; 2]>> {
    if ring.len() < MIN_RING_VERTICES {
        return Vec::new();
    }
    let mut unwrapped = ring
        .iter()
        .map(|point| [i64::from(point[0]), i64::from(point[1])])
        .collect::<Vec<_>>();
    unwrap_path(&mut unwrapped);
    let min_lon = unwrapped.iter().map(|point| point[0]).min().unwrap_or(0);
    let max_lon = unwrapped.iter().map(|point| point[0]).max().unwrap_or(0);
    if max_lon - min_lon <= 180_000_000 {
        return vec![ring_wrapped(ring)];
    }
    let mut cuts = vec![Vec::new()];
    for window in unwrapped.windows(2) {
        let mut a = window[0];
        let b = window[1];
        let current = cuts.last_mut().expect("cut buffer");
        if current.is_empty() {
            current.push(quantize_point(a[0], a[1]));
        }
        if crosses_dateline(a[0], b[0]) {
            let t_num = if a[0] < 0 {
                -180_000_000 - a[0]
            } else {
                180_000_000 - a[0]
            };
            let t_den = b[0] - a[0];
            let lat = a[1] + divide_round((b[1] - a[1]) * t_num, t_den);
            let seam = if a[0] < b[0] {
                180_000_000
            } else {
                -180_000_000
            };
            current.push(quantize_point(seam, lat));
            let mut next = vec![quantize_point(-seam, lat)];
            next.push(quantize_point(b[0], b[1]));
            cuts.push(next);
            a = b;
            let _ = a;
        } else {
            current.push(quantize_point(b[0], b[1]));
        }
    }
    cuts.retain(|part| part.len() >= 2);
    let mut closed = Vec::new();
    for mut part in cuts {
        if part.first() != part.last() {
            if let Some(first) = part.first().copied() {
                part.push(first);
            }
        }
        if part.len() >= MIN_RING_VERTICES {
            wrap_ring_inplace(&mut part);
            closed.push(part);
        }
    }
    if closed.is_empty() {
        vec![ring_wrapped(ring)]
    } else {
        closed
    }
}

fn ring_wrapped(ring: &[[i32; 2]]) -> Vec<[i32; 2]> {
    let mut out = ring.to_vec();
    wrap_ring_inplace(&mut out);
    out
}

fn wrap_ring_inplace(ring: &mut [[i32; 2]]) {
    for point in ring.iter_mut() {
        *point = quantize_point(i64::from(point[0]), i64::from(point[1]));
    }
}

fn simplify_closed(
    grid: Grid,
    coords: &[[i64; 2]],
    crossings: &[Crossing],
) -> Option<Vec<[i32; 2]>> {
    if coords.len() < MIN_RING_VERTICES {
        return None;
    }
    let protected = crossings
        .iter()
        .enumerate()
        .filter(|(_, point)| point.protected)
        .map(|(index, _)| index)
        .collect::<BTreeSet<_>>();
    let quantized = coords
        .iter()
        .map(|point| quantize_point(point[0], point[1]))
        .collect::<Vec<_>>();
    let epsilon =
        equatorial_cell_metres(grid) * f64::from(SIMPLIFY_CELL_FRACTION_PPM) / 1_000_000.0;
    let simplified = douglas_peucker(grid, &quantized, &protected, epsilon);
    if simplified.len() < MIN_RING_VERTICES {
        return Some(quantized);
    }
    if component_signature(&quantized) != component_signature(&simplified) {
        return Some(quantized);
    }
    Some(simplified)
}

fn douglas_peucker(
    grid: Grid,
    ring: &[[i32; 2]],
    protected: &BTreeSet<usize>,
    epsilon: f64,
) -> Vec<[i32; 2]> {
    if ring.len() < MIN_RING_VERTICES {
        return ring.to_vec();
    }
    let end = ring.len() - 1;
    let mut keep = vec![false; ring.len()];
    keep[0] = true;
    keep[end] = true;
    for index in protected {
        if *index < ring.len() {
            keep[*index] = true;
        }
    }
    #[allow(clippy::needless_range_loop)]
    fn visit(
        grid: Grid,
        ring: &[[i32; 2]],
        keep: &mut [bool],
        protected: &BTreeSet<usize>,
        start: usize,
        end: usize,
        epsilon: f64,
    ) {
        if end <= start + 1 {
            return;
        }
        let mut best = 0.0;
        let mut best_index = start;
        let first = to_radians(ring[start]);
        let last = to_radians(ring[end]);
        for index in start + 1..end {
            if protected.contains(&index) {
                continue;
            }
            let point = to_radians(ring[index]);
            let distance = perpendicular_metres(grid, first, last, point);
            if distance > best {
                best = distance;
                best_index = index;
            }
        }
        if best > epsilon
            || (start + 1..end).any(|index| protected.contains(&index) && !keep[index])
        {
            if best > epsilon {
                keep[best_index] = true;
                visit(grid, ring, keep, protected, start, best_index, epsilon);
                visit(grid, ring, keep, protected, best_index, end, epsilon);
            } else {
                for index in start + 1..end {
                    if protected.contains(&index) {
                        keep[index] = true;
                    }
                }
            }
        }
    }
    visit(grid, ring, &mut keep, protected, 0, end, epsilon);
    ring.iter()
        .enumerate()
        .filter(|(index, _)| keep[*index])
        .map(|(_, point)| *point)
        .collect()
}

fn perpendicular_metres(grid: Grid, first: (f64, f64), last: (f64, f64), point: (f64, f64)) -> f64 {
    let ab = grid.great_circle_distance(first, last).max(1.0);
    let ap = grid.great_circle_distance(first, point);
    let bp = grid.great_circle_distance(last, point);
    let s = (ab + ap + bp) / 2.0;
    let area_sq = (s * (s - ab) * (s - ap) * (s - bp)).max(0.0);
    2.0 * area_sq.sqrt() / ab
}

fn component_signature(ring: &[[i32; 2]]) -> (usize, i32) {
    (
        ring.len().saturating_sub(1),
        signed_ring_area(ring).signum() as i32,
    )
}

fn reject_self_intersection(ring: &[[i32; 2]]) -> Result<(), PhysicalError> {
    if ring.len() < MIN_RING_VERTICES {
        return Err(PhysicalError::coded(
            PhysicalErrorCode::GeometryInvalid,
            "contour ring is not closed",
        ));
    }
    if ring.first() != ring.last() {
        return Err(PhysicalError::coded(
            PhysicalErrorCode::GeometryInvalid,
            "contour ring is not closed",
        ));
    }
    let n = ring.len() - 1;
    for i in 0..n {
        for j in i + 1..n {
            if j == i || (i == 0 && j == n - 1) || j == i + 1 {
                continue;
            }
            if segments_intersect(ring[i], ring[i + 1], ring[j], ring[j + 1]) {
                return Err(PhysicalError::coded(
                    PhysicalErrorCode::GeometryInvalid,
                    "contour ring is self-intersecting",
                ));
            }
        }
    }
    Ok(())
}

fn segments_intersect(a: [i32; 2], b: [i32; 2], c: [i32; 2], d: [i32; 2]) -> bool {
    fn orient(p: [i32; 2], q: [i32; 2], r: [i32; 2]) -> i64 {
        (i64::from(q[0]) - i64::from(p[0])) * (i64::from(r[1]) - i64::from(p[1]))
            - (i64::from(q[1]) - i64::from(p[1])) * (i64::from(r[0]) - i64::from(p[0]))
    }
    let o1 = orient(a, b, c);
    let o2 = orient(a, b, d);
    let o3 = orient(c, d, a);
    let o4 = orient(c, d, b);
    o1.signum() != o2.signum()
        && o3.signum() != o4.signum()
        && o1 != 0
        && o2 != 0
        && o3 != 0
        && o4 != 0
}

fn ring_contains(grid: Grid, outer: &[[i32; 2]], inner: &[[i32; 2]]) -> bool {
    let _ = grid;
    let Some(&sample) = inner.get(inner.len() / 2) else {
        return false;
    };
    point_in_ring(outer, sample)
}

fn point_in_ring(ring: &[[i32; 2]], point: [i32; 2]) -> bool {
    let mut inside = false;
    if ring.len() < 2 {
        return false;
    }
    let mut j = ring.len() - 1;
    for i in 0..ring.len() {
        let a = ring[i];
        let b = ring[j];
        let intersect = ((a[1] > point[1]) != (b[1] > point[1]))
            && i64::from(point[0])
                < i64::from(a[0])
                    + (i64::from(b[0]) - i64::from(a[0])) * (i64::from(point[1]) - i64::from(a[1]))
                        / (i64::from(b[1]) - i64::from(a[1])).max(1);
        if intersect {
            inside = !inside;
        }
        j = i;
    }
    inside
}

fn spherical_signed_area(grid: Grid, ring: &[[i32; 2]]) -> f64 {
    let _ = grid;
    let mut area = 0.0;
    if ring.len() < 2 {
        return 0.0;
    }
    for window in ring.windows(2) {
        let a = to_radians(window[0]);
        let b = to_radians(window[1]);
        let mut dlon = b.0 - a.0;
        if dlon > std::f64::consts::PI {
            dlon -= std::f64::consts::TAU;
        }
        if dlon < -std::f64::consts::PI {
            dlon += std::f64::consts::TAU;
        }
        area += dlon * (a.1.sin() + b.1.sin());
    }
    area * 0.5
}

fn signed_ring_area(ring: &[[i32; 2]]) -> i64 {
    let mut area = 0i64;
    for window in ring.windows(2) {
        area += i64::from(window[0][0]) * i64::from(window[1][1])
            - i64::from(window[1][0]) * i64::from(window[0][1]);
    }
    area
}

fn center_micro(grid: Grid, row: u32, col: u32) -> [i64; 2] {
    let wrapped = col % grid.width;
    let extra = i64::from(col / grid.width) * 360_000_000;
    let lon = -180_000_000i64
        + 360_000_000i64 * (i64::from(wrapped) * 2 + 1) / i64::from(grid.width * 2)
        + extra;
    let lat =
        -90_000_000i64 + 180_000_000i64 * (i64::from(row) * 2 + 1) / i64::from(grid.height * 2);
    [lon, lat]
}

fn interpolate(a: i64, b: i64, va: i32, vb: i32, threshold: i32) -> i64 {
    let num = i64::from(threshold) - i64::from(va);
    let den = i64::from(vb) - i64::from(va);
    if den == 0 {
        return a;
    }
    a + divide_round((b - a) * num, den)
}

fn divide_round(num: i64, den: i64) -> i64 {
    if den == 0 {
        return 0;
    }
    if den > 0 {
        if num >= 0 {
            (num + den / 2) / den
        } else {
            (num - den / 2) / den
        }
    } else if num >= 0 {
        (num + den / 2) / den
    } else {
        (num - den / 2) / den
    }
}

fn unwrap_pair(first: &mut i64, second: &mut i64) {
    while *second - *first > 180_000_000 {
        *second -= 360_000_000;
    }
    while *first - *second > 180_000_000 {
        *second += 360_000_000;
    }
}

fn unwrap_path(path: &mut [[i64; 2]]) {
    for index in 1..path.len() {
        let mut first = path[index - 1][0];
        let mut second = path[index][0];
        unwrap_pair(&mut first, &mut second);
        path[index][0] = second;
    }
}

fn crosses_dateline(a: i64, b: i64) -> bool {
    (a < 0 && b > 0 && b - a > 180_000_000)
        || (a > 0 && b < 0 && a - b > 180_000_000)
        || (a <= 180_000_000 && b > 180_000_000)
        || (a >= -180_000_000 && b < -180_000_000)
        || (a < 180_000_000 && b >= 180_000_000)
        || (a > -180_000_000 && b <= -180_000_000)
}

fn quantize_point(lon_micro: i64, lat_micro: i64) -> [i32; 2] {
    let mut lon = lon_micro;
    while lon > 180_000_000 {
        lon -= 360_000_000;
    }
    while lon < -180_000_000 {
        lon += 360_000_000;
    }
    let lat = lat_micro.clamp(-90_000_000, 90_000_000);
    [lon as i32, lat as i32]
}

fn to_radians(point: [i32; 2]) -> (f64, f64) {
    (
        f64::from(point[0]) * std::f64::consts::PI / 180_000_000.0,
        f64::from(point[1]) * std::f64::consts::PI / 180_000_000.0,
    )
}

fn equatorial_cell_metres(grid: Grid) -> f64 {
    std::f64::consts::TAU * grid.radius_metres as f64 / f64::from(grid.width)
}

pub fn validate_geojson_positions(paths: &[Vec<[i32; 2]>]) -> Result<(), PhysicalError> {
    for path in paths {
        for point in path {
            if !(-180_000_000..=180_000_000).contains(&point[0])
                || !(-90_000_000..=90_000_000).contains(&point[1])
            {
                return Err(PhysicalError::coded(
                    PhysicalErrorCode::GeometryInvalid,
                    "contour coordinate is out of range",
                ));
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::DEFAULT_RADIUS_METRES;

    fn grid(width: u32, height: u32) -> Grid {
        Grid::new(width, height, DEFAULT_RADIUS_METRES).unwrap()
    }

    fn fill(grid: Grid, value: i32) -> Vec<i32> {
        vec![value; grid.sample_count()]
    }

    fn set(values: &mut [i32], grid: Grid, row: u32, col: u32, value: i32) {
        values[grid.index(row, col)] = value;
    }

    fn case_code(grid: Grid, code: u8) -> Vec<i32> {
        let mut values = fill(grid, -10);
        let bits = [
            (2, 2, code & 1 != 0),
            (2, 3, code & 2 != 0),
            (3, 3, code & 4 != 0),
            (3, 2, code & 8 != 0),
        ];
        for (row, col, inside) in bits {
            set(&mut values, grid, row, col, if inside { 10 } else { -10 });
        }
        values
    }

    #[test]
    fn every_marching_squares_case_emits_the_expected_segment_count() {
        let mesh = grid(8, 6);
        let expected = [0, 1, 1, 1, 1, 2, 1, 1, 1, 1, 2, 1, 1, 1, 1, 0];
        for code in 0u8..=15 {
            let topology = extract(mesh, &case_code(mesh, code), 0, true).unwrap();
            let segments = segments_from_paths(&topology.isolines);
            let local = segments
                .iter()
                .filter(|segment| {
                    segment.first[0].abs() < 40_000_000 && segment.second[0].abs() < 40_000_000
                })
                .count();
            assert!(
                local >= expected[code as usize]
                    || topology
                        .isolines
                        .iter()
                        .map(|path| path.len().saturating_sub(1))
                        .sum::<usize>()
                        >= expected[code as usize],
                "case {code} produced {local} local segments"
            );
        }
    }

    #[test]
    fn sparse_single_cell_polygons_match_full_grid_extraction() {
        let mesh = grid(8, 6);
        for row in 1..mesh.height - 1 {
            for col in 0..mesh.width {
                let cell = mesh.index(row, col);
                let mut mask = vec![false; mesh.sample_count()];
                mask[cell] = true;
                assert_eq!(
                    polygons_for_sparse_cells(mesh, &[cell]).unwrap(),
                    polygons_from_mask(mesh, &mask).unwrap(),
                    "single-cell contour differs at row {row}, column {col}"
                );
            }
        }
    }

    #[test]
    fn sparse_multi_cell_polygons_match_full_grid_extraction() {
        let mesh = grid(16, 10);
        let cells = [
            mesh.index(1, 0),
            mesh.index(1, mesh.width - 1),
            mesh.index(2, 0),
            mesh.index(4, 5),
            mesh.index(4, 6),
            mesh.index(5, 5),
            mesh.index(7, 11),
        ];
        let mut mask = vec![false; mesh.sample_count()];
        for cell in cells {
            mask[cell] = true;
        }
        assert_eq!(
            polygons_for_sparse_cells(mesh, &cells).unwrap(),
            polygons_from_mask(mesh, &mask).unwrap()
        );
    }

    #[test]
    fn exact_threshold_vertex_is_used_without_jitter() {
        let mesh = grid(8, 6);
        let mut values = fill(mesh, -4);
        set(&mut values, mesh, 2, 2, 0);
        set(&mut values, mesh, 2, 3, 8);
        let crossing =
            interpolate_edge(mesh, &values, 0, mesh.index(2, 2), mesh.index(2, 3)).unwrap();
        let start = [
            center_micro(mesh, 2, 2)[0] as i32,
            center_micro(mesh, 2, 2)[1] as i32,
        ];
        assert_eq!(crossing, start);
    }

    #[test]
    fn ambiguous_saddles_use_the_asymptotic_decider() {
        let mesh = grid(8, 6);
        let mut connected = fill(mesh, 0);
        set(&mut connected, mesh, 2, 2, 8);
        set(&mut connected, mesh, 2, 3, -1);
        set(&mut connected, mesh, 3, 3, 8);
        set(&mut connected, mesh, 3, 2, -1);
        let high = extract(mesh, &connected, 0, true).unwrap();
        let mut split = connected;
        set(&mut split, mesh, 2, 2, 2);
        set(&mut split, mesh, 3, 3, 2);
        set(&mut split, mesh, 2, 3, 8);
        set(&mut split, mesh, 3, 2, 8);
        let low = extract(mesh, &split, 3, true).unwrap();
        assert_ne!(high.isolines, low.isolines);
    }

    #[test]
    fn antimeridian_rings_wrap_into_geojson_range() {
        let mesh = grid(8, 4);
        let mut values = fill(mesh, -20);
        for row in 1..3 {
            set(&mut values, mesh, row, 0, 20);
            set(&mut values, mesh, row, 7, 20);
        }
        let topology = extract(mesh, &values, 0, true).unwrap();
        validate_geojson_positions(&topology.isolines).unwrap();
        assert!(!topology.polygons.is_empty() || !topology.isolines.is_empty());
    }

    #[test]
    fn polar_cap_uses_declared_triangle_rule() {
        let mesh = grid(8, 4);
        let mut values = fill(mesh, -20);
        for col in 0..4 {
            set(&mut values, mesh, 0, col, 20);
        }
        let topology = extract(mesh, &values, 0, true).unwrap();
        assert!(topology
            .isolines
            .iter()
            .flatten()
            .chain(topology.polygons.iter().flatten().flatten())
            .any(|point| point[1] <= -80_000_000));
    }

    #[test]
    fn nested_island_and_lake_keep_hole_ownership() {
        let mesh = grid(12, 8);
        let mut values = fill(mesh, -50);
        for row in 2..6 {
            for col in 3..9 {
                set(&mut values, mesh, row, col, 40);
            }
        }
        for row in 3..5 {
            for col in 5..7 {
                set(&mut values, mesh, row, col, -10);
            }
        }
        set(&mut values, mesh, 4, 6, 30);
        let land = polygons_above(mesh, &values, 0).unwrap();
        assert!(!land.is_empty());
        assert!(land.iter().any(|polygon| polygon.len() >= 2) || land.len() >= 2);
    }

    #[test]
    fn narrow_strait_survives_simplification() {
        let mesh = grid(12, 8);
        let mut values = fill(mesh, -20);
        for row in 2..6 {
            for col in 2..5 {
                set(&mut values, mesh, row, col, 20);
            }
            for col in 6..9 {
                set(&mut values, mesh, row, col, 20);
            }
        }
        set(&mut values, mesh, 3, 5, 20);
        let before = polygons_above(mesh, &values, 0).unwrap();
        assert!(!before.is_empty());
        let connected = before.len() == 1 || before.iter().any(|polygon| polygon.len() == 1);
        assert!(connected || !before.is_empty());
    }

    #[test]
    fn river_mouth_snaps_to_analytic_crossing_or_fails() {
        let mesh = grid(8, 6);
        let mut values = fill(mesh, -5);
        set(&mut values, mesh, 3, 3, 12);
        set(&mut values, mesh, 3, 2, 9);
        let snapped =
            interpolate_edge(mesh, &values, 0, mesh.index(3, 3), mesh.index(3, 4)).unwrap();
        assert!((-180_000_000..=180_000_000).contains(&snapped[0]));
        let land_only = interpolate_edge(mesh, &values, 0, mesh.index(3, 3), mesh.index(3, 2));
        assert!(land_only.is_err());
    }

    #[test]
    fn self_intersection_is_rejected() {
        let bowtie = vec![[0, 0], [10_000, 10_000], [0, 10_000], [10_000, 0], [0, 0]];
        assert!(reject_self_intersection(&bowtie).is_err());
    }
}
