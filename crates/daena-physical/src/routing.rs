//! Deterministic land-route suggestions on the physical grid.
//!
//! Suggestions are disposable. An accepted road becomes authored GeoJSON.

use std::cmp::Reverse;
use std::collections::{BinaryHeap, HashSet};

use serde::{Deserialize, Serialize};

use crate::climate::{
    ClimateField, BIOME_ALPINE, BIOME_DESERT, BIOME_ICE, BIOME_TROPICAL_FOREST, BIOME_TUNDRA,
};
use crate::hydrology::HydrologyField;
use crate::{Grid, PhysicalError};

pub const ROUTE_STRATEGY_SHORTEST: &str = "shortest";
pub const ROUTE_STRATEGY_EASIEST: &str = "easiest";
pub const ROUTE_STRATEGY_REUSE: &str = "reuse";
pub const ROUTE_STRATEGY_BALANCED: &str = "balanced";
const MAX_SUGGESTIONS: usize = 4;
const DUPLICATE_JACCARD_PPM: u32 = 820_000;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RouteSuggestQuery {
    pub start_longitude_microdegrees: i32,
    pub start_latitude_microdegrees: i32,
    pub end_longitude_microdegrees: i32,
    pub end_latitude_microdegrees: i32,
    #[serde(default)]
    pub existing_routes: Vec<Vec<[i32; 2]>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RouteSuggestion {
    pub id: u32,
    pub strategy: String,
    pub label: String,
    pub tradeoff: String,
    pub reasons: Vec<String>,
    pub length_m: u32,
    pub climb_m: u32,
    pub max_slope_ppm: u32,
    pub river_crossings: u32,
    pub lake_cells: u32,
    pub ice_cells: u32,
    pub existing_road_cells: u32,
    pub cell_count: u32,
    pub coordinates: Vec<Vec<[i32; 2]>>,
    pub cells: Vec<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RouteSuggestResult {
    pub suggestion_count: u32,
    pub suggestions: Vec<RouteSuggestion>,
}

#[derive(Clone, Copy)]
struct Weights {
    climb: u32,
    slope: u32,
    river_milli: u32,
    lake_milli: u32,
    ice_milli: u32,
    road_ppm: u32,
    biome: bool,
}

pub fn suggest_routes(
    hydrology: &HydrologyField,
    climate: Option<&ClimateField>,
    query: &RouteSuggestQuery,
) -> Result<RouteSuggestResult, PhysicalError> {
    query.validate()?;
    let grid = hydrology.grid;
    let count = grid.sample_count();
    if hydrology.water_level_mm.len() != count
        || hydrology.slope_ppm.len() != count
        || hydrology.lake_cells.len() != count
        || hydrology.ice_cells.len() != count
    {
        return Err(PhysicalError::InvalidSettings(
            "hydrology fields do not share a grid".into(),
        ));
    }
    if let Some(field) = climate {
        if field.grid != grid || field.biome_class.len() != count {
            return Err(PhysicalError::InvalidSettings(
                "climate field does not match the hydrology grid".into(),
            ));
        }
    }

    let start = cell_from_microdegrees(
        grid,
        query.start_longitude_microdegrees,
        query.start_latitude_microdegrees,
    );
    let end = cell_from_microdegrees(
        grid,
        query.end_longitude_microdegrees,
        query.end_latitude_microdegrees,
    );
    let ocean = ocean_mask(hydrology);
    if ocean[start] {
        return Err(PhysicalError::InvalidSettings(
            "Start point is in the ocean.".into(),
        ));
    }
    if ocean[end] {
        return Err(PhysicalError::InvalidSettings(
            "End point is in the ocean.".into(),
        ));
    }
    if start == end {
        return Err(PhysicalError::InvalidSettings(
            "Start and end are the same cell.".into(),
        ));
    }

    let river = river_mask(hydrology);
    let roads = rasterize_routes(grid, &query.existing_routes);
    let mut found = Vec::new();
    for (strategy, weights) in strategies() {
        if strategy == ROUTE_STRATEGY_REUSE && !roads.iter().any(|on_road| *on_road) {
            continue;
        }
        let Some(path) = dijkstra(
            hydrology, climate, &ocean, &river, &roads, start, end, weights,
        ) else {
            continue;
        };
        let path_cells: Vec<u32> = path.cells.iter().map(|cell| *cell as u32).collect();
        if found
            .iter()
            .any(|previous: &RouteSuggestion| similar_cells(&previous.cells, &path_cells))
        {
            continue;
        }
        found.push(summarize(hydrology, &river, &roads, strategy, path));
        if found.len() >= MAX_SUGGESTIONS {
            break;
        }
    }
    if found.is_empty() {
        return Err(PhysicalError::InvalidSettings(
            "No land path exists between those points.".into(),
        ));
    }
    for (index, suggestion) in found.iter_mut().enumerate() {
        suggestion.id = index as u32 + 1;
    }
    Ok(RouteSuggestResult {
        suggestion_count: found.len() as u32,
        suggestions: found,
    })
}

impl RouteSuggestQuery {
    fn validate(&self) -> Result<(), PhysicalError> {
        check_lon(self.start_longitude_microdegrees)?;
        check_lon(self.end_longitude_microdegrees)?;
        check_lat(self.start_latitude_microdegrees)?;
        check_lat(self.end_latitude_microdegrees)?;
        Ok(())
    }
}

fn check_lon(value: i32) -> Result<(), PhysicalError> {
    if !(-180_000_000..=180_000_000).contains(&value) {
        Err(PhysicalError::InvalidSettings(
            "longitude must be within ±180 degrees".into(),
        ))
    } else {
        Ok(())
    }
}

fn check_lat(value: i32) -> Result<(), PhysicalError> {
    if !(-90_000_000..=90_000_000).contains(&value) {
        Err(PhysicalError::InvalidSettings(
            "latitude must be within ±90 degrees".into(),
        ))
    } else {
        Ok(())
    }
}

fn strategies() -> [(&'static str, Weights); 4] {
    [
        (
            ROUTE_STRATEGY_SHORTEST,
            Weights {
                climb: 0,
                slope: 0,
                river_milli: 1_500,
                lake_milli: 3_000,
                ice_milli: 1_500,
                road_ppm: 700_000,
                biome: false,
            },
        ),
        (
            ROUTE_STRATEGY_EASIEST,
            Weights {
                climb: 8,
                slope: 6,
                river_milli: 12_000,
                lake_milli: 20_000,
                ice_milli: 6_000,
                road_ppm: 400_000,
                biome: true,
            },
        ),
        (
            ROUTE_STRATEGY_REUSE,
            Weights {
                climb: 1,
                slope: 1,
                river_milli: 5_000,
                lake_milli: 8_000,
                ice_milli: 2_500,
                road_ppm: 80_000,
                biome: false,
            },
        ),
        (
            ROUTE_STRATEGY_BALANCED,
            Weights {
                climb: 3,
                slope: 3,
                river_milli: 7_000,
                lake_milli: 12_000,
                ice_milli: 3_500,
                road_ppm: 250_000,
                biome: true,
            },
        ),
    ]
}

struct Path {
    cells: Vec<usize>,
}

fn dijkstra(
    hydrology: &HydrologyField,
    climate: Option<&ClimateField>,
    ocean: &[bool],
    river: &[bool],
    roads: &[bool],
    start: usize,
    end: usize,
    weights: Weights,
) -> Option<Path> {
    let grid = hydrology.grid;
    let count = grid.sample_count();
    let mut cost = vec![u64::MAX; count];
    let mut parent = vec![None; count];
    let mut heap = BinaryHeap::new();
    cost[start] = 0;
    heap.push(Reverse((0_u64, start)));
    while let Some(Reverse((current, cell))) = heap.pop() {
        if current != cost[cell] {
            continue;
        }
        if cell == end {
            break;
        }
        for neighbor in adjacent_neighbors(grid, cell) {
            if ocean[neighbor] {
                continue;
            }
            let step = edge_cost(hydrology, climate, river, roads, weights, cell, neighbor)?;
            let next = current.saturating_add(step);
            if next < cost[neighbor] {
                cost[neighbor] = next;
                parent[neighbor] = Some(cell);
                heap.push(Reverse((next, neighbor)));
            }
        }
    }
    if cost[end] == u64::MAX {
        return None;
    }
    let mut cells = Vec::new();
    let mut cursor = end;
    cells.push(cursor);
    while cursor != start {
        cursor = parent[cursor]?;
        cells.push(cursor);
    }
    cells.reverse();
    Some(Path { cells })
}

fn edge_cost(
    hydrology: &HydrologyField,
    climate: Option<&ClimateField>,
    river: &[bool],
    roads: &[bool],
    weights: Weights,
    from: usize,
    to: usize,
) -> Option<u64> {
    let grid = hydrology.grid;
    let distance_m = distance_metres(grid, from, to).max(1);
    let climb_m = climb_metres(hydrology, from, to);
    let slope_ppm = edge_slope_ppm(hydrology, from, to, distance_m);
    let milli = 1_000_u64
        .saturating_add(u64::from(climb_m).saturating_mul(u64::from(weights.climb)))
        .saturating_add(u64::from(slope_ppm / 1_000).saturating_mul(u64::from(weights.slope)));
    let mut cost = u64::from(distance_m).saturating_mul(milli) / 1_000;
    if hydrology.lake_cells[to] {
        cost = cost.saturating_mul(u64::from(weights.lake_milli)) / 1_000;
    } else if river[to] {
        cost = cost.saturating_mul(u64::from(weights.river_milli)) / 1_000;
    }
    if hydrology.ice_cells[to] {
        cost = cost.saturating_mul(u64::from(weights.ice_milli)) / 1_000;
    }
    if roads[to] {
        cost = cost.saturating_mul(u64::from(weights.road_ppm)) / 1_000_000;
    }
    if weights.biome {
        if let Some(field) = climate {
            cost = cost.saturating_mul(u64::from(biome_ppm(field.biome_class[to]))) / 1_000_000;
        }
    }
    Some(cost.max(1))
}

fn summarize(
    hydrology: &HydrologyField,
    river: &[bool],
    roads: &[bool],
    strategy: &str,
    path: Path,
) -> RouteSuggestion {
    let grid = hydrology.grid;
    let mut length_m = 0_u32;
    let mut climb_m = 0_u32;
    let mut max_slope_ppm = 0_u32;
    let mut river_crossings = 0_u32;
    let mut lake_cells = 0_u32;
    let mut ice_cells = 0_u32;
    let mut existing_road_cells = 0_u32;
    for window in path.cells.windows(2) {
        let from = window[0];
        let to = window[1];
        let distance = distance_metres(grid, from, to).max(1);
        length_m = length_m.saturating_add(distance);
        climb_m = climb_m.saturating_add(climb_metres(hydrology, from, to));
        max_slope_ppm = max_slope_ppm.max(edge_slope_ppm(hydrology, from, to, distance));
        if !river[from] && river[to] {
            river_crossings = river_crossings.saturating_add(1);
        }
    }
    for cell in &path.cells {
        max_slope_ppm = max_slope_ppm.max(hydrology.slope_ppm[*cell]);
        if hydrology.lake_cells[*cell] {
            lake_cells += 1;
        }
        if hydrology.ice_cells[*cell] {
            ice_cells += 1;
        }
        if roads[*cell] {
            existing_road_cells += 1;
        }
    }
    let cells = path
        .cells
        .iter()
        .map(|cell| *cell as u32)
        .collect::<Vec<_>>();
    let coordinates = line_parts(grid, &path.cells);
    let (label, tradeoff) = strategy_copy(strategy);
    RouteSuggestion {
        id: 0,
        strategy: strategy.to_owned(),
        label: label.to_owned(),
        tradeoff: tradeoff.to_owned(),
        reasons: reasons(
            length_m,
            climb_m,
            river_crossings,
            lake_cells,
            existing_road_cells,
            cells.len() as u32,
        ),
        length_m,
        climb_m,
        max_slope_ppm,
        river_crossings,
        lake_cells,
        ice_cells,
        existing_road_cells,
        cell_count: cells.len() as u32,
        coordinates,
        cells,
    }
}

fn strategy_copy(strategy: &str) -> (&'static str, &'static str) {
    match strategy {
        ROUTE_STRATEGY_SHORTEST => ("Shortest", "Shortest land path."),
        ROUTE_STRATEGY_EASIEST => ("Easiest terrain", "Longer but avoids steep terrain."),
        ROUTE_STRATEGY_REUSE => ("Follows existing roads", "Maximizes use of existing roads."),
        _ => ("Balanced", "Balances distance and climbing."),
    }
}

fn reasons(
    length_m: u32,
    climb_m: u32,
    river_crossings: u32,
    lake_cells: u32,
    existing_road_cells: u32,
    cell_count: u32,
) -> Vec<String> {
    let mut lines = vec![format!("{} km", (length_m + 500) / 1_000)];
    if climb_m > 0 {
        lines.push(format!("{climb_m} m of climbing"));
    }
    if river_crossings > 0 {
        lines.push(format!(
            "{} river crossing{}",
            river_crossings,
            if river_crossings == 1 { "" } else { "s" }
        ));
    }
    if lake_cells > 0 {
        lines.push("crosses lake cells".into());
    }
    if existing_road_cells > 0 && cell_count > 0 {
        let pct = existing_road_cells.saturating_mul(100) / cell_count;
        lines.push(format!("uses existing roads for {pct}% of the corridor"));
    }
    lines
}

fn similar_cells(left: &[u32], right: &[u32]) -> bool {
    if left.is_empty() || right.is_empty() {
        return false;
    }
    let left_set: HashSet<u32> = left.iter().copied().collect();
    let right_set: HashSet<u32> = right.iter().copied().collect();
    let intersection = left_set.intersection(&right_set).count() as u64;
    let union = left_set.union(&right_set).count() as u64;
    if union == 0 {
        return false;
    }
    intersection.saturating_mul(1_000_000) / union >= u64::from(DUPLICATE_JACCARD_PPM)
}

fn line_parts(grid: Grid, cells: &[usize]) -> Vec<Vec<[i32; 2]>> {
    let mut parts = Vec::new();
    let mut current = Vec::new();
    let mut previous: Option<[i32; 2]> = None;
    for &cell in cells {
        let coord = cell_center_microdegrees(grid, cell);
        if let Some(prior) = previous {
            if (i64::from(coord[0]) - i64::from(prior[0])).abs() > 180_000_000 {
                if current.len() >= 2 {
                    parts.push(std::mem::take(&mut current));
                } else {
                    current.clear();
                }
            }
        }
        current.push(coord);
        previous = Some(coord);
    }
    if current.len() >= 2 {
        parts.push(current);
    } else if parts.is_empty() {
        parts.push(
            cells
                .iter()
                .map(|cell| cell_center_microdegrees(grid, *cell))
                .collect(),
        );
    }
    parts
}

fn rasterize_routes(grid: Grid, routes: &[Vec<[i32; 2]>]) -> Vec<bool> {
    let mut mask = vec![false; grid.sample_count()];
    for line in routes {
        let mut previous = None;
        for point in line {
            let cell = cell_from_microdegrees(grid, point[0], point[1]);
            mask[cell] = true;
            if let Some(prior) = previous {
                paint_segment(grid, &mut mask, prior, cell);
            }
            previous = Some(cell);
        }
    }
    mask
}

fn paint_segment(grid: Grid, mask: &mut [bool], start: usize, end: usize) {
    let (r0, c0) = grid.row_col(start);
    let (r1, c1) = grid.row_col(end);
    let width = grid.width as i32;
    let mut dc = c1 as i32 - c0 as i32;
    if dc > width / 2 {
        dc -= width;
    }
    if dc < -(width / 2) {
        dc += width;
    }
    let dr = r1 as i32 - r0 as i32;
    let steps = dr.abs().max(dc.abs()).max(1);
    for step in 0..=steps {
        let row = (r0 as i32 + dr * step / steps).clamp(0, grid.height as i32 - 1) as u32;
        let col = ((c0 as i32 + dc * step / steps).rem_euclid(width)) as u32;
        mask[grid.index(row, col)] = true;
    }
}

fn adjacent_neighbors(grid: Grid, index: usize) -> Vec<usize> {
    let (row, col) = grid.row_col(index);
    let west = (col + grid.width - 1) % grid.width;
    let east = (col + 1) % grid.width;
    let mut neighbors = vec![grid.index(row, west), grid.index(row, east)];
    if row > 0 {
        neighbors.push(grid.index(row - 1, col));
        neighbors.push(grid.index(row - 1, west));
        neighbors.push(grid.index(row - 1, east));
    }
    if row + 1 < grid.height {
        neighbors.push(grid.index(row + 1, col));
        neighbors.push(grid.index(row + 1, west));
        neighbors.push(grid.index(row + 1, east));
    }
    neighbors
}

fn ocean_mask(hydrology: &HydrologyField) -> Vec<bool> {
    hydrology
        .water_level_mm
        .iter()
        .map(|level| *level <= hydrology.sea_level_mm)
        .collect()
}

fn river_mask(hydrology: &HydrologyField) -> Vec<bool> {
    let mut mask = vec![false; hydrology.grid.sample_count()];
    for river in &hydrology.rivers {
        if river.source_cell < mask.len() {
            mask[river.source_cell] = true;
        }
        if river.mouth_cell < mask.len() {
            mask[river.mouth_cell] = true;
        }
    }
    for path in &hydrology.river_coordinates {
        let mut previous = None;
        for [lon, lat] in path {
            let cell = cell_from_microdegrees(hydrology.grid, *lon, *lat);
            mask[cell] = true;
            if let Some(prior) = previous {
                paint_segment(hydrology.grid, &mut mask, prior, cell);
            }
            previous = Some(cell);
        }
    }
    mask
}

fn cell_from_microdegrees(grid: Grid, lon: i32, lat: i32) -> usize {
    let col = ((i64::from(lon) + 180_000_000) * i64::from(grid.width) / 360_000_000)
        .clamp(0, i64::from(grid.width) - 1) as u32;
    let row = ((i64::from(lat) + 90_000_000) * i64::from(grid.height) / 180_000_000)
        .clamp(0, i64::from(grid.height) - 1) as u32;
    grid.index(row, col)
}

fn cell_center_microdegrees(grid: Grid, cell: usize) -> [i32; 2] {
    let (row, col) = grid.row_col(cell);
    let (lon, lat) = grid.center_radians(row, col);
    [
        (lon.to_degrees() * 1_000_000.0).round() as i32,
        (lat.to_degrees() * 1_000_000.0).round() as i32,
    ]
}

fn distance_metres(grid: Grid, from: usize, to: usize) -> u32 {
    let (row_a, col_a) = grid.row_col(from);
    let (row_b, col_b) = grid.row_col(to);
    grid.great_circle_distance(
        grid.center_radians(row_a, col_a),
        grid.center_radians(row_b, col_b),
    )
    .round()
    .clamp(0.0, f64::from(u32::MAX / 4)) as u32
}

fn climb_metres(hydrology: &HydrologyField, from: usize, to: usize) -> u32 {
    let delta = hydrology.water_level_mm[to] - hydrology.water_level_mm[from];
    if delta > 0 {
        (delta / 1_000) as u32
    } else {
        0
    }
}

fn edge_slope_ppm(hydrology: &HydrologyField, from: usize, to: usize, distance_m: u32) -> u32 {
    let rise_mm = (hydrology.water_level_mm[to] - hydrology.water_level_mm[from]).unsigned_abs();
    let from_slope = hydrology.slope_ppm[from];
    let to_slope = hydrology.slope_ppm[to];
    let edge = if distance_m == 0 {
        0
    } else {
        ((u64::from(rise_mm) * 1_000) / u64::from(distance_m)).min(u64::from(u32::MAX)) as u32
    };
    from_slope.max(to_slope).max(edge)
}

fn biome_ppm(class: u32) -> u32 {
    match class {
        BIOME_ICE => 2_500_000,
        BIOME_ALPINE => 1_800_000,
        BIOME_TUNDRA => 1_400_000,
        BIOME_DESERT => 1_350_000,
        BIOME_TROPICAL_FOREST => 1_450_000,
        _ => 1_000_000,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hydrology::{Basin, WaterBalanceMetrics};
    use crate::{Segment, DEFAULT_RADIUS_METRES};

    fn empty_hydrology(grid: Grid) -> HydrologyField {
        let count = grid.sample_count();
        HydrologyField {
            grid,
            derivation_version: crate::hydrology::HYDROLOGY_DERIVATION_VERSION,
            sea_level_mm: 0,
            water_level_mm: vec![-2_000_000; count],
            lake_level_mm: vec![0; count],
            slope_ppm: vec![0; count],
            hillshade_ppm: vec![0; count],
            bathymetry_mm: vec![2_000_000; count],
            watershed_id: vec![0; count],
            basin_by_cell: vec![0; count],
            lake_cells: vec![false; count],
            ice_cells: vec![false; count],
            ice_thickness_mm: vec![0; count],
            shelf_cells: vec![false; count],
            island_id: vec![u32::MAX; count],
            basins: Vec::<Basin>::new(),
            rivers: Vec::new(),
            river_coordinates: Vec::new(),
            coastline_segments: Vec::<Segment>::new(),
            lake_polygons: Vec::new(),
            watershed_polygons: Vec::new(),
            land_polygons: Vec::new(),
            ocean_polygons: Vec::new(),
            shelf_polygons: Vec::new(),
            island_polygons: Vec::new(),
            ice_polygons: Vec::new(),
            bathymetry_contours: Vec::new(),
            metrics: WaterBalanceMetrics {
                total_water_m3: 0,
                ocean_water_m3: 0,
                inland_water_m3: 0,
                land_ice_m3: 0,
                balance_error_m3: 0,
                tolerance_m3: 0,
                fixed_point_iterations: 0,
                converged: true,
                lake_count: 0,
                river_count: 0,
                watershed_count: 0,
                coastline_segment_count: 0,
                land_polygon_count: 0,
                ocean_polygon_count: 0,
                shelf_cell_count: 0,
                bathymetry_contour_count: 0,
                island_count: 0,
            },
        }
    }

    fn paint_land(
        hydrology: &mut HydrologyField,
        cols: std::ops::Range<u32>,
        rows: std::ops::Range<u32>,
        elev_mm: i32,
        slope_ppm: u32,
    ) {
        for row in rows {
            for col in cols.clone() {
                let cell = hydrology.grid.index(row, col);
                hydrology.water_level_mm[cell] = elev_mm;
                hydrology.slope_ppm[cell] = slope_ppm;
            }
        }
    }

    fn query_cells(grid: Grid, start: (u32, u32), end: (u32, u32)) -> RouteSuggestQuery {
        let start_c = cell_center_microdegrees(grid, grid.index(start.0, start.1));
        let end_c = cell_center_microdegrees(grid, grid.index(end.0, end.1));
        RouteSuggestQuery {
            start_longitude_microdegrees: start_c[0],
            start_latitude_microdegrees: start_c[1],
            end_longitude_microdegrees: end_c[0],
            end_latitude_microdegrees: end_c[1],
            existing_routes: Vec::new(),
        }
    }

    fn strategy<'a>(result: &'a RouteSuggestResult, name: &str) -> &'a RouteSuggestion {
        result
            .suggestions
            .iter()
            .find(|item| item.strategy == name)
            .unwrap_or_else(|| panic!("missing {name}"))
    }

    #[test]
    fn ocean_start_is_rejected() {
        let grid = Grid::new(16, 8, DEFAULT_RADIUS_METRES).unwrap();
        let mut hydrology = empty_hydrology(grid);
        paint_land(&mut hydrology, 8..15, 2..6, 200_000, 1_000);
        let query = query_cells(grid, (3, 2), (3, 12));
        assert!(suggest_routes(&hydrology, None, &query).is_err());
    }

    fn land_with_bridge(hydrology: &mut HydrologyField, bridge_elev_mm: i32, bridge_slope: u32) {
        paint_land(hydrology, 1..6, 3..5, 200_000, 2_000);
        paint_land(hydrology, 10..15, 3..5, 200_000, 2_000);
        paint_land(hydrology, 6..10, 3..4, bridge_elev_mm, bridge_slope);
        paint_land(hydrology, 1..6, 5..7, 200_000, 2_000);
        paint_land(hydrology, 1..15, 6..7, 200_000, 2_000);
        paint_land(hydrology, 10..15, 5..7, 200_000, 2_000);
    }

    #[test]
    fn easiest_avoids_a_ridge_that_shortest_crosses() {
        let grid = Grid::new(16, 8, DEFAULT_RADIUS_METRES).unwrap();
        let mut hydrology = empty_hydrology(grid);
        land_with_bridge(&mut hydrology, 4_000_000, 220_000);
        let result = suggest_routes(&hydrology, None, &query_cells(grid, (3, 3), (3, 12))).unwrap();
        assert!(result.suggestion_count >= 2);
        let shortest = strategy(&result, ROUTE_STRATEGY_SHORTEST);
        let easiest = strategy(&result, ROUTE_STRATEGY_EASIEST);
        assert!(easiest.climb_m < shortest.climb_m);
        assert!(easiest.length_m >= shortest.length_m);
        assert!(!shortest.reasons.is_empty());
        assert!(!easiest.tradeoff.is_empty());
        assert_ne!(shortest.cells, easiest.cells);
    }

    #[test]
    fn cell_centers_round_trip() {
        let grid = Grid::new(16, 8, DEFAULT_RADIUS_METRES).unwrap();
        for row in 0..8 {
            for col in 0..16 {
                let cell = grid.index(row, col);
                let [lon, lat] = cell_center_microdegrees(grid, cell);
                assert_eq!(cell_from_microdegrees(grid, lon, lat), cell);
            }
        }
    }

    #[test]
    fn river_coordinates_fill_cells_between_samples() {
        let grid = Grid::new(16, 8, DEFAULT_RADIUS_METRES).unwrap();
        let mut hydrology = empty_hydrology(grid);
        land_with_bridge(&mut hydrology, 200_000, 2_000);
        hydrology.river_coordinates.push(vec![
            cell_center_microdegrees(grid, grid.index(3, 6)),
            cell_center_microdegrees(grid, grid.index(3, 9)),
        ]);
        let result = suggest_routes(&hydrology, None, &query_cells(grid, (3, 3), (3, 12))).unwrap();
        let chosen = result
            .suggestions
            .iter()
            .find(|item| item.strategy == ROUTE_STRATEGY_EASIEST)
            .unwrap_or(&result.suggestions[0]);
        assert_eq!(chosen.river_crossings, 0);
        assert!(!chosen.cells.iter().any(|cell| {
            let (row, col) = grid.row_col(*cell as usize);
            row == 3 && (6..10).contains(&col)
        }));
    }

    #[test]
    fn water_is_not_treated_as_cheap_land() {
        let grid = Grid::new(16, 8, DEFAULT_RADIUS_METRES).unwrap();
        let mut hydrology = empty_hydrology(grid);
        land_with_bridge(&mut hydrology, 200_000, 2_000);
        for col in 6..10 {
            let cell = grid.index(3, col);
            hydrology.lake_cells[cell] = true;
            hydrology
                .river_coordinates
                .push(vec![cell_center_microdegrees(grid, cell)]);
        }
        let result = suggest_routes(&hydrology, None, &query_cells(grid, (3, 3), (3, 12))).unwrap();
        let chosen = result
            .suggestions
            .iter()
            .find(|item| item.strategy == ROUTE_STRATEGY_EASIEST)
            .unwrap_or(&result.suggestions[0]);
        assert_eq!(chosen.lake_cells, 0);
        assert_eq!(chosen.river_crossings, 0);
    }

    #[test]
    fn reuse_follows_an_existing_road_detour() {
        let grid = Grid::new(16, 8, DEFAULT_RADIUS_METRES).unwrap();
        let mut hydrology = empty_hydrology(grid);
        paint_land(&mut hydrology, 1..15, 1..6, 200_000, 2_000);
        let mut road = Vec::new();
        for col in 2..=13 {
            road.push(cell_center_microdegrees(grid, grid.index(5, col)));
        }
        let mut query = query_cells(grid, (2, 2), (2, 13));
        query.existing_routes = vec![road];
        let result = suggest_routes(&hydrology, None, &query).unwrap();
        let reuse = strategy(&result, ROUTE_STRATEGY_REUSE);
        let shortest = strategy(&result, ROUTE_STRATEGY_SHORTEST);
        assert!(reuse.existing_road_cells > shortest.existing_road_cells);
        assert!(reuse.existing_road_cells >= reuse.cell_count / 2);
    }

    #[test]
    fn ocean_blocks_the_short_path() {
        let grid = Grid::new(16, 8, DEFAULT_RADIUS_METRES).unwrap();
        let mut hydrology = empty_hydrology(grid);
        paint_land(&mut hydrology, 1..6, 2..6, 200_000, 1_000);
        paint_land(&mut hydrology, 10..15, 2..6, 200_000, 1_000);
        paint_land(&mut hydrology, 1..15, 5..6, 200_000, 1_000);
        let result = suggest_routes(&hydrology, None, &query_cells(grid, (3, 3), (3, 12))).unwrap();
        let shortest = strategy(&result, ROUTE_STRATEGY_SHORTEST);
        assert!(shortest.cells.iter().any(|cell| {
            let (row, _) = grid.row_col(*cell as usize);
            row == 5
        }));
    }

    #[test]
    fn wrapping_longitude_takes_the_short_seam() {
        let grid = Grid::new(16, 8, DEFAULT_RADIUS_METRES).unwrap();
        let mut hydrology = empty_hydrology(grid);
        paint_land(&mut hydrology, 0..16, 2..5, 200_000, 1_000);
        let result = suggest_routes(&hydrology, None, &query_cells(grid, (3, 1), (3, 15))).unwrap();
        let shortest = strategy(&result, ROUTE_STRATEGY_SHORTEST);
        assert!(shortest.cell_count <= 4);
        assert!(shortest.coordinates.len() >= 1);
    }

    #[test]
    fn high_plateau_is_not_penalized_for_absolute_elevation() {
        let grid = Grid::new(16, 8, DEFAULT_RADIUS_METRES).unwrap();
        let mut hydrology = empty_hydrology(grid);
        paint_land(&mut hydrology, 1..15, 4..6, 2_000_000, 1_000);
        let result = suggest_routes(&hydrology, None, &query_cells(grid, (4, 3), (4, 12))).unwrap();
        assert!(result.suggestions.iter().all(|item| item.climb_m == 0));
        assert!(result.suggestions.iter().all(|item| {
            item.cells
                .iter()
                .all(|cell| hydrology.water_level_mm[*cell as usize] >= 1_500_000)
        }));
    }
}
