//! Deterministic land-route suggestions on the physical grid.
//!
//! Suggestions are disposable. An accepted road becomes authored GeoJSON.

use std::cmp::Reverse;
use std::collections::{BinaryHeap, HashSet};

use serde::{Deserialize, Serialize};

use crate::engine::{ToolSurface, TOOL_SURFACE_NONE};
use crate::PhysicalError;

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

struct Path {
    cells: Vec<usize>,
}

/// Route suggestions over the canonical cube cells of a tool surface.
///
/// Dijkstra runs on the eight cube neighbours, so a path never enters an
/// ocean cell. Lengths, climbs and slopes use true cell-centre distances.
/// Authored route geometry is projected to canonical cells for the reuse
/// strategy.
pub fn suggest_routes_canonical(
    surface: &ToolSurface,
    query: &RouteSuggestQuery,
) -> Result<RouteSuggestResult, PhysicalError> {
    query.validate()?;
    let start = surface.cell_at_microdegrees(
        query.start_longitude_microdegrees,
        query.start_latitude_microdegrees,
    );
    let end = surface.cell_at_microdegrees(
        query.end_longitude_microdegrees,
        query.end_latitude_microdegrees,
    );
    if surface.ocean(start) {
        return Err(PhysicalError::InvalidSettings(
            "Start point is in the ocean.".into(),
        ));
    }
    if surface.ocean(end) {
        return Err(PhysicalError::InvalidSettings(
            "End point is in the ocean.".into(),
        ));
    }
    if start == end {
        return Err(PhysicalError::InvalidSettings(
            "Start and end are the same cell.".into(),
        ));
    }

    let roads = rasterize_surface_routes(surface, &query.existing_routes);
    let mut found = Vec::new();
    for (strategy, weights) in strategies() {
        if strategy == ROUTE_STRATEGY_REUSE && !roads.iter().any(|on_road| *on_road) {
            continue;
        }
        let Some(path) = dijkstra_surface(surface, &roads, start, end, weights) else {
            continue;
        };
        let path_cells: Vec<u32> = path.cells.iter().map(|cell| *cell as u32).collect();
        if found
            .iter()
            .any(|previous: &RouteSuggestion| similar_cells(&previous.cells, &path_cells))
        {
            continue;
        }
        found.push(summarize_surface(surface, &roads, strategy, path));
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

fn dijkstra_surface(
    surface: &ToolSurface,
    roads: &[bool],
    start: u32,
    end: u32,
    weights: Weights,
) -> Option<Path> {
    let count = surface.cell_count();
    let mut cost = vec![u64::MAX; count];
    let mut parent = vec![None; count];
    let mut heap = BinaryHeap::new();
    cost[start as usize] = 0;
    heap.push(Reverse((0_u64, start)));
    while let Some(Reverse((current, cell))) = heap.pop() {
        if current != cost[cell as usize] {
            continue;
        }
        if cell == end {
            break;
        }
        for neighbor in surface.neighbors(cell) {
            if neighbor == TOOL_SURFACE_NONE || surface.ocean(neighbor) {
                continue;
            }
            let step = surface_edge_cost(surface, roads, weights, cell, neighbor);
            let next = current.saturating_add(step);
            if next < cost[neighbor as usize] {
                cost[neighbor as usize] = next;
                parent[neighbor as usize] = Some(cell);
                heap.push(Reverse((next, neighbor)));
            }
        }
    }
    if cost[end as usize] == u64::MAX {
        return None;
    }
    let mut cells = Vec::new();
    let mut cursor = end;
    cells.push(cursor);
    while cursor != start {
        cursor = parent[cursor as usize]?;
        cells.push(cursor);
    }
    cells.reverse();
    Some(Path {
        cells: cells.into_iter().map(|cell| cell as usize).collect(),
    })
}

fn surface_edge_cost(
    surface: &ToolSurface,
    roads: &[bool],
    weights: Weights,
    from: u32,
    to: u32,
) -> u64 {
    let distance_m = u64::from(surface.edge_distance_metres(from, to).unwrap_or(1).max(1));
    let climb_m = surface_climb_metres(surface, from, to);
    let slope_ppm = surface_edge_slope_ppm(surface, from, to, distance_m as u32);
    let milli = 1_000_u64
        .saturating_add(climb_m.saturating_mul(u64::from(weights.climb)))
        .saturating_add(u64::from(slope_ppm / 1_000).saturating_mul(u64::from(weights.slope)));
    let mut cost = distance_m.saturating_mul(milli) / 1_000;
    if surface.wet_lake(to) {
        cost = cost.saturating_mul(u64::from(weights.lake_milli)) / 1_000;
    } else if surface.river(to) {
        cost = cost.saturating_mul(u64::from(weights.river_milli)) / 1_000;
    }
    if surface.ice(to) {
        cost = cost.saturating_mul(u64::from(weights.ice_milli)) / 1_000;
    }
    if roads[to as usize] {
        cost = cost.saturating_mul(u64::from(weights.road_ppm)) / 1_000_000;
    }
    if weights.biome {
        cost = cost.saturating_mul(u64::from(surface_biome_ppm(
            surface.biome_class[to as usize],
        ))) / 1_000_000;
    }
    cost.max(1)
}

fn summarize_surface(
    surface: &ToolSurface,
    roads: &[bool],
    strategy: &str,
    path: Path,
) -> RouteSuggestion {
    let mut length_m = 0_u32;
    let mut climb_m = 0_u32;
    let mut max_slope_ppm = 0_u32;
    let mut river_crossings = 0_u32;
    let mut lake_cells = 0_u32;
    let mut ice_cells = 0_u32;
    let mut existing_road_cells = 0_u32;
    for window in path.cells.windows(2) {
        let from = window[0] as u32;
        let to = window[1] as u32;
        let distance = surface.edge_distance_metres(from, to).unwrap_or(1).max(1);
        length_m = length_m.saturating_add(distance);
        climb_m = climb_m.saturating_add(surface_climb_metres(surface, from, to) as u32);
        max_slope_ppm = max_slope_ppm.max(surface_edge_slope_ppm(surface, from, to, distance));
        if !surface.river(from) && surface.river(to) {
            river_crossings = river_crossings.saturating_add(1);
        }
    }
    for cell in &path.cells {
        let cell = *cell as u32;
        max_slope_ppm = max_slope_ppm.max(surface.slope_ppm[cell as usize]);
        if surface.wet_lake(cell) {
            lake_cells += 1;
        }
        if surface.ice(cell) {
            ice_cells += 1;
        }
        if roads[cell as usize] {
            existing_road_cells += 1;
        }
    }
    let cells = path
        .cells
        .iter()
        .map(|cell| *cell as u32)
        .collect::<Vec<_>>();
    let coordinates = line_parts_surface(surface, &cells);
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

fn surface_climb_metres(surface: &ToolSurface, from: u32, to: u32) -> u64 {
    let delta = i64::from(surface.elevation_mm[to as usize])
        - i64::from(surface.elevation_mm[from as usize]);
    if delta > 0 {
        (delta / 1_000) as u64
    } else {
        0
    }
}

fn surface_edge_slope_ppm(surface: &ToolSurface, from: u32, to: u32, distance_m: u32) -> u32 {
    let rise_mm = (i64::from(surface.elevation_mm[to as usize])
        - i64::from(surface.elevation_mm[from as usize]))
    .unsigned_abs();
    let edge = if distance_m == 0 {
        0
    } else {
        (rise_mm.saturating_mul(1_000) / u64::from(distance_m)).min(u64::from(u32::MAX)) as u32
    };
    surface.slope_ppm[from as usize]
        .max(surface.slope_ppm[to as usize])
        .max(edge)
}

/// One adapter from the twenty-six engine biomes to the existing route-cost
/// families: ice, alpine, tundra, desert, tropical forest and default.
fn surface_biome_ppm(biome: u8) -> u32 {
    match biome {
        // SeaIceSeasonal, SeaIcePerennial, IceSheet, Glacier.
        4 | 5 | 9 | 10 => 2_500_000,
        // Alpine.
        12 => 1_800_000,
        // Tundra.
        11 => 1_400_000,
        // SaltFlat, ColdDesert, HotDesert.
        8 | 15 | 20 => 1_350_000,
        // TropicalSeasonalForest, TropicalRainforest, Mangrove.
        22 | 23 | 25 => 1_450_000,
        _ => 1_000_000,
    }
}

fn rasterize_surface_routes(surface: &ToolSurface, routes: &[Vec<[i32; 2]>]) -> Vec<bool> {
    let mut mask = vec![false; surface.cell_count()];
    for line in routes {
        let mut previous: Option<u32> = None;
        for point in line {
            let cell = surface.cell_at_microdegrees(point[0], point[1]);
            mask[cell as usize] = true;
            if let Some(prior) = previous {
                if prior != cell {
                    paint_surface_segment(surface, &mut mask, prior, cell);
                }
            }
            previous = Some(cell);
        }
    }
    mask
}

fn paint_surface_segment(surface: &ToolSurface, mask: &mut [bool], from: u32, to: u32) {
    let start = surface.direction_of(from);
    let end = surface.direction_of(to);
    let dot = (start[0] * end[0] + start[1] * end[1] + start[2] * end[2]).clamp(-1.0, 1.0);
    let angle = dot.acos();
    let cell_angle = std::f64::consts::FRAC_PI_2 / f64::from(surface.resolution().max(1));
    let steps = ((angle / cell_angle).ceil() as usize).clamp(1, 4_096);
    for step in 0..=steps {
        let t = step as f64 / steps as f64;
        let direction = slerp(start, end, t, angle);
        let cell = surface.cell_at_direction(direction);
        mask[cell as usize] = true;
    }
}

fn slerp(from: [f64; 3], to: [f64; 3], t: f64, angle: f64) -> [f64; 3] {
    if angle < 1e-12 {
        return from;
    }
    let sin = angle.sin();
    let first = ((1.0 - t) * angle).sin() / sin;
    let second = (t * angle).sin() / sin;
    [
        first * from[0] + second * to[0],
        first * from[1] + second * to[1],
        first * from[2] + second * to[2],
    ]
}

fn line_parts_surface(surface: &ToolSurface, cells: &[u32]) -> Vec<Vec<[i32; 2]>> {
    let mut parts = Vec::new();
    let mut current = Vec::new();
    let mut previous: Option<[i32; 2]> = None;
    for &cell in cells {
        let coordinate = surface.center_microdegrees(cell);
        if let Some(prior) = previous {
            if (i64::from(coordinate[0]) - i64::from(prior[0])).abs() > 180_000_000 {
                if current.len() >= 2 {
                    parts.push(std::mem::take(&mut current));
                } else {
                    current.clear();
                }
            }
        }
        current.push(coordinate);
        previous = Some(coordinate);
    }
    if current.len() >= 2 {
        parts.push(current);
    } else if parts.is_empty() {
        parts.push(
            cells
                .iter()
                .map(|cell| surface.center_microdegrees(*cell))
                .collect(),
        );
    }
    parts
}

#[cfg(test)]
mod canonical_tests {
    use super::*;
    use crate::engine::ToolSurface;

    fn land_surface(n: u32) -> ToolSurface {
        let mut surface = ToolSurface::synthetic(n);
        for cell in 0..surface.cell_count() as u32 {
            surface.test_set_land(cell);
            surface.elevation_mm[cell as usize] = 0;
        }
        surface
    }

    fn query_between(surface: &ToolSurface, start: u32, end: u32) -> RouteSuggestQuery {
        let start = surface.center_microdegrees(start);
        let end = surface.center_microdegrees(end);
        RouteSuggestQuery {
            start_longitude_microdegrees: start[0],
            start_latitude_microdegrees: start[1],
            end_longitude_microdegrees: end[0],
            end_latitude_microdegrees: end[1],
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
    fn canonical_path_stays_on_land_and_snaps_endpoints() {
        let surface = land_surface(16);
        let start = 100u32;
        let end = 900u32;
        let query = query_between(&surface, start, end);
        let result = suggest_routes_canonical(&surface, &query).unwrap();
        assert!(result.suggestion_count >= 1);
        let path = strategy(&result, ROUTE_STRATEGY_SHORTEST);
        assert_eq!(path.cells.first().copied(), Some(start));
        assert_eq!(path.cells.last().copied(), Some(end));
        assert!(path.cells.iter().all(|cell| !surface.ocean(*cell)));
        assert!(path
            .cells
            .iter()
            .all(|cell| (*cell as usize) < surface.cell_count()));
        assert_eq!(path.cell_count as usize, path.cells.len());
        assert!(path.length_m > 0);
        assert!(!path.coordinates.is_empty());
    }

    #[test]
    fn canonical_route_counts_match_the_surface() {
        let mut surface = land_surface(16);
        for cell in 0..surface.cell_count() as u32 {
            surface.slope_ppm[cell as usize] = 12_345;
        }
        let query = query_between(&surface, 100, 900);
        let result = suggest_routes_canonical(&surface, &query).unwrap();
        let path = strategy(&result, ROUTE_STRATEGY_SHORTEST);
        assert_eq!(path.max_slope_ppm, 12_345);
        assert_eq!(path.river_crossings, 0);
        assert_eq!(path.lake_cells, 0);
        assert_eq!(path.ice_cells, 0);

        for cell in 0..surface.cell_count() as u32 {
            surface.test_set_lake(cell, 1, false);
            surface.test_set_ice(cell);
        }
        let result = suggest_routes_canonical(&surface, &query).unwrap();
        let path = strategy(&result, ROUTE_STRATEGY_SHORTEST);
        assert_eq!(path.lake_cells, path.cell_count);
        assert_eq!(path.ice_cells, path.cell_count);
    }

    #[test]
    fn canonical_route_rejects_ocean_endpoints_and_unreachable_islands() {
        let mut surface = ToolSurface::synthetic(16);
        let first = surface.cell_at_degrees(0.0, 0.0);
        let second = surface.cell_at_degrees(0.0, 180.0);
        let query = query_between(&surface, first, second);
        let error = suggest_routes_canonical(&surface, &query).unwrap_err();
        assert!(error.to_string().contains("Start point is in the ocean"));

        surface.test_set_land(first);
        surface.test_set_land(second);
        let error = suggest_routes_canonical(&surface, &query).unwrap_err();
        assert!(error.to_string().contains("No land path"));
    }

    #[test]
    fn canonical_route_reuses_projected_authored_geometry() {
        let surface = land_surface(16);
        let mut query = query_between(&surface, 100, 900);
        let road_cells = [100u32, 200, 300, 400, 500, 600, 700, 800, 900];
        let road = road_cells
            .iter()
            .map(|cell| surface.center_microdegrees(*cell))
            .collect::<Vec<_>>();
        query.existing_routes = vec![road];
        let result = suggest_routes_canonical(&surface, &query).unwrap();
        let reuse = strategy(&result, ROUTE_STRATEGY_REUSE);
        assert!(reuse.existing_road_cells > 0);
        assert!(reuse.existing_road_cells <= reuse.cell_count);
    }

    #[test]
    fn canonical_route_crosses_the_antimeridian() {
        let surface = land_surface(16);
        let start = surface.cell_at_degrees(0.0, 160.0);
        let end = surface.cell_at_degrees(0.0, -160.0);
        let query = query_between(&surface, start, end);
        let result = suggest_routes_canonical(&surface, &query).unwrap();
        let path = strategy(&result, ROUTE_STRATEGY_SHORTEST);
        assert!(path.cell_count < 100);
        assert!(path
            .coordinates
            .iter()
            .flat_map(|part| part.iter())
            .all(|point| (-180_000_000..=180_000_000).contains(&point[0])
                && (-90_000_000..=90_000_000).contains(&point[1])));
    }
}
