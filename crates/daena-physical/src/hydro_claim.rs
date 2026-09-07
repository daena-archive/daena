//! Stable identities for generated lakes and rivers.
//!
//! Derived hydrology GeoJSON uses list indexes. Claims instead key a lake by
//! basin `minimum_cell` and a river by `source_cell` so a name can follow the
//! spine when sea-level moves the mouth.

use crate::hydrology::{BasinStatus, HydrologyField};
use crate::Grid;

pub const KIND_LAKE: &str = "physical-lake";
pub const KIND_RIVER: &str = "physical-river";
pub const LAYER_LAKES: &str = "lakes";
pub const LAYER_RIVERS: &str = "rivers";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HydroClaim {
    pub kind: &'static str,
    pub id: String,
    pub layer_id: &'static str,
    pub label: String,
    pub lon_micro: i32,
    pub lat_micro: i32,
}

#[must_use]
pub fn lake_id(minimum_cell: usize) -> String {
    format!("lake:{minimum_cell}")
}

#[must_use]
pub fn river_id(source_cell: usize) -> String {
    format!("river:{source_cell}")
}

#[must_use]
pub fn is_valid_id(kind: &str, id: &str) -> bool {
    match kind {
        KIND_LAKE => parse_lake_id(id).is_some(),
        KIND_RIVER => parse_river_id(id).is_some(),
        _ => false,
    }
}

#[must_use]
pub fn claim_at(
    hydrology: &HydrologyField,
    lon_micro: i32,
    lat_micro: i32,
    radius_micro: i32,
) -> Option<HydroClaim> {
    lake_at(hydrology, lon_micro, lat_micro).or_else(|| {
        river_at(
            hydrology,
            lon_micro,
            lat_micro,
            i64::from(radius_micro.max(1)),
        )
    })
}

#[must_use]
pub fn resolve(hydrology: &HydrologyField, kind: &str, id: &str) -> Option<HydroClaim> {
    match kind {
        KIND_LAKE => {
            let cell = parse_lake_id(id)?;
            hydrology
                .basins
                .iter()
                .find(|basin| basin.minimum_cell == cell && basin_is_lake(hydrology, basin.id))
                .and_then(|basin| lake_claim(hydrology, basin.minimum_cell))
        }
        KIND_RIVER => {
            let source = parse_river_id(id)?;
            hydrology
                .rivers
                .iter()
                .zip(&hydrology.river_coordinates)
                .filter(|(river, _)| river.source_cell == source)
                .min_by_key(|(river, _)| (river.mouth_cell, river.id))
                .map(|(river, path)| river_claim(river, path))
        }
        _ => None,
    }
}

fn lake_at(hydrology: &HydrologyField, lon_micro: i32, lat_micro: i32) -> Option<HydroClaim> {
    let cell = cell_from_microdegrees(hydrology.grid, lon_micro, lat_micro);
    if hydrology.lake_cells.get(cell) != Some(&true) {
        return None;
    }
    let basin_id = hydrology.basin_by_cell.get(cell).copied()?;
    let basin = hydrology.basins.get(basin_id as usize)?;
    if basin.status == BasinStatus::Merged {
        return None;
    }
    lake_claim(hydrology, basin.minimum_cell)
}

fn lake_claim(hydrology: &HydrologyField, minimum_cell: usize) -> Option<HydroClaim> {
    let [lon_micro, lat_micro] = coordinate_for_cell(hydrology.grid, minimum_cell);
    Some(HydroClaim {
        kind: KIND_LAKE,
        id: lake_id(minimum_cell),
        layer_id: LAYER_LAKES,
        label: "Unnamed lake".into(),
        lon_micro,
        lat_micro,
    })
}

fn river_at(
    hydrology: &HydrologyField,
    lon_micro: i32,
    lat_micro: i32,
    radius: i64,
) -> Option<HydroClaim> {
    let radius2 = radius.saturating_mul(radius);
    let mut best: Option<(i64, HydroClaim)> = None;
    for (river, path) in hydrology.rivers.iter().zip(&hydrology.river_coordinates) {
        let Some(distance2) = path
            .iter()
            .map(|point| distance2(point[0], point[1], lon_micro, lat_micro))
            .min()
        else {
            continue;
        };
        if distance2 > radius2 {
            continue;
        }
        if best
            .as_ref()
            .is_some_and(|(best_distance, _)| distance2 >= *best_distance)
        {
            continue;
        }
        best = Some((distance2, river_claim(river, path)));
    }
    best.map(|(_, claim)| claim)
}

fn river_claim(river: &crate::hydrology::RiverSegment, path: &[[i32; 2]]) -> HydroClaim {
    let point = path
        .get(path.len() / 2)
        .or_else(|| path.first())
        .copied()
        .unwrap_or([0, 0]);
    HydroClaim {
        kind: KIND_RIVER,
        id: river_id(river.source_cell),
        layer_id: LAYER_RIVERS,
        label: format!("River order {}", river.strahler_order),
        lon_micro: point[0],
        lat_micro: point[1],
    }
}

fn basin_is_lake(hydrology: &HydrologyField, basin_id: usize) -> bool {
    hydrology
        .basin_by_cell
        .iter()
        .enumerate()
        .any(|(cell, id)| *id as usize == basin_id && hydrology.lake_cells.get(cell) == Some(&true))
}

fn parse_lake_id(id: &str) -> Option<usize> {
    id.strip_prefix("lake:")?.parse().ok()
}

fn parse_river_id(id: &str) -> Option<usize> {
    id.strip_prefix("river:")?.parse().ok()
}

fn cell_from_microdegrees(grid: Grid, lon: i32, lat: i32) -> usize {
    let col = ((i64::from(lon) + 180_000_000) * i64::from(grid.width) / 360_000_000)
        .clamp(0, i64::from(grid.width) - 1) as u32;
    let row = ((i64::from(lat) + 90_000_000) * i64::from(grid.height) / 180_000_000)
        .clamp(0, i64::from(grid.height) - 1) as u32;
    grid.index(row, col)
}

fn coordinate_for_cell(grid: Grid, cell: usize) -> [i32; 2] {
    let (row, col) = grid.row_col(cell);
    [
        (-180_000_000i64 + 360_000_000i64 * (i64::from(col) * 2 + 1) / i64::from(grid.width * 2))
            as i32,
        (-90_000_000i64 + 180_000_000i64 * (i64::from(row) * 2 + 1) / i64::from(grid.height * 2))
            as i32,
    ]
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hydrology::{
        Basin, BasinDestination, BasinStatus, RiverSegment, WaterBalanceMetrics,
        HYDROLOGY_DERIVATION_VERSION,
    };
    use crate::{Grid, Segment, DEFAULT_RADIUS_METRES};

    fn empty_hydrology(grid: Grid) -> HydrologyField {
        let count = grid.sample_count();
        HydrologyField {
            grid,
            derivation_version: HYDROLOGY_DERIVATION_VERSION,
            sea_level_mm: 0,
            water_level_mm: vec![-2_000_000; count],
            lake_level_mm: vec![0; count],
            slope_ppm: vec![0; count],
            hillshade_ppm: vec![0; count],
            bathymetry_mm: vec![2_000_000; count],
            watershed_id: vec![u32::MAX; count],
            basin_by_cell: vec![0; count],
            lake_cells: vec![false; count],
            ice_cells: vec![false; count],
            ice_thickness_mm: vec![0; count],
            shelf_cells: vec![false; count],
            island_id: vec![u32::MAX; count],
            basins: Vec::new(),
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

    fn basin(id: usize, minimum_cell: usize) -> Basin {
        Basin {
            id,
            minimum_cell,
            minimum_elevation_mm: 0,
            cell_count: 1,
            spill_cell: None,
            spill_elevation_mm: None,
            volume_to_spill_m3: 0,
            parent_basin: None,
            children: Vec::new(),
            destination: BasinDestination::Endorheic,
            water_level_mm: 1_000,
            water_volume_m3: 1,
            inflow_m3_per_year: 0,
            direct_precipitation_m3_per_year: 0,
            evaporation_m3_per_year: 0,
            outflow_m3_per_year: 0,
            status: BasinStatus::Endorheic,
        }
    }

    #[test]
    fn lake_claim_uses_basin_minimum_cell() {
        let grid = Grid::new(16, 8, DEFAULT_RADIUS_METRES).unwrap();
        let mut hydrology = empty_hydrology(grid);
        let pit = grid.index(3, 8);
        let shore = grid.index(3, 9);
        hydrology.lake_cells[pit] = true;
        hydrology.lake_cells[shore] = true;
        hydrology.basin_by_cell[pit] = 0;
        hydrology.basin_by_cell[shore] = 0;
        hydrology.basins.push(basin(0, pit));
        let [lon, lat] = coordinate_for_cell(grid, shore);
        let claim = claim_at(&hydrology, lon, lat, 50_000).expect("lake");
        assert_eq!(claim.kind, KIND_LAKE);
        assert_eq!(claim.id, lake_id(pit));
        assert!(resolve(&hydrology, KIND_LAKE, &claim.id).is_some());
        assert!(resolve(&hydrology, KIND_LAKE, &lake_id(shore)).is_none());
    }

    #[test]
    fn river_claim_uses_source_cell() {
        let grid = Grid::new(16, 8, DEFAULT_RADIUS_METRES).unwrap();
        let mut hydrology = empty_hydrology(grid);
        let source = grid.index(2, 4);
        let mouth = grid.index(5, 10);
        let start = coordinate_for_cell(grid, source);
        let end = coordinate_for_cell(grid, mouth);
        hydrology.rivers.push(RiverSegment {
            id: 99,
            source_cell: source,
            mouth_cell: mouth,
            strahler_order: 3,
            destination: BasinDestination::Ocean,
            spill_outlet: false,
            coordinate_count: 2,
        });
        hydrology.river_coordinates.push(vec![start, end]);
        let claim = claim_at(&hydrology, start[0], start[1], 50_000).expect("river");
        assert_eq!(claim.kind, KIND_RIVER);
        assert_eq!(claim.id, river_id(source));
        assert_eq!(claim.label, "River order 3");
        assert!(is_valid_id(KIND_RIVER, &claim.id));
        assert!(!is_valid_id(KIND_RIVER, "river-000099"));
        assert!(!is_valid_id(KIND_RIVER, "river:4:18"));
        assert!(resolve(&hydrology, KIND_RIVER, &claim.id).is_some());
    }

    #[test]
    fn river_claim_survives_mouth_movement() {
        let grid = Grid::new(16, 8, DEFAULT_RADIUS_METRES).unwrap();
        let mut hydrology = empty_hydrology(grid);
        let source = grid.index(2, 4);
        let old_mouth = grid.index(5, 10);
        let new_mouth = grid.index(4, 9);
        let start = coordinate_for_cell(grid, source);
        let end = coordinate_for_cell(grid, new_mouth);
        hydrology.rivers.push(RiverSegment {
            id: 1,
            source_cell: source,
            mouth_cell: new_mouth,
            strahler_order: 2,
            destination: BasinDestination::Ocean,
            spill_outlet: false,
            coordinate_count: 2,
        });
        hydrology.river_coordinates.push(vec![start, end]);
        let id = river_id(source);
        assert_ne!(id, format!("river:{source}:{old_mouth}"));
        let resolved = resolve(&hydrology, KIND_RIVER, &id).expect("spine");
        assert_eq!(resolved.id, id);
        assert_eq!(resolved.kind, KIND_RIVER);
    }

    #[test]
    fn lake_wins_over_a_river_in_the_same_cell() {
        let grid = Grid::new(16, 8, DEFAULT_RADIUS_METRES).unwrap();
        let mut hydrology = empty_hydrology(grid);
        let cell = grid.index(3, 8);
        let point = coordinate_for_cell(grid, cell);
        hydrology.lake_cells[cell] = true;
        hydrology.basin_by_cell[cell] = 0;
        hydrology.basins.push(basin(0, cell));
        hydrology.rivers.push(RiverSegment {
            id: 0,
            source_cell: cell,
            mouth_cell: cell,
            strahler_order: 1,
            destination: BasinDestination::Endorheic,
            spill_outlet: false,
            coordinate_count: 1,
        });
        hydrology.river_coordinates.push(vec![point]);
        let claim = claim_at(&hydrology, point[0], point[1], 50_000).expect("lake");
        assert_eq!(claim.kind, KIND_LAKE);
    }
}
