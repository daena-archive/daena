//! Stable identities for generated landmasses.
//!
//! Hydrology assigns `island_id` in discovery order. Claims instead key a
//! landmass by the lowest cell index in that connected land so a name can
//! follow the land across overlay redraws without copying geometry.

use crate::hydrology::HydrologyField;
use crate::Grid;

pub const KIND_LANDMASS: &str = "physical-landmass";
pub const LAYER_LANDMASS: &str = "landmass";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LandmassClaim {
    pub kind: &'static str,
    pub id: String,
    pub layer_id: &'static str,
    pub label: String,
    pub lon_micro: i32,
    pub lat_micro: i32,
}

#[must_use]
pub fn landmass_id(minimum_cell: usize) -> String {
    format!("landmass:{minimum_cell}")
}

#[must_use]
pub fn is_valid_id(kind: &str, id: &str) -> bool {
    kind == KIND_LANDMASS && parse_landmass_id(id).is_some()
}

#[must_use]
pub fn claim_at(
    hydrology: &HydrologyField,
    lon_micro: i32,
    lat_micro: i32,
) -> Option<LandmassClaim> {
    let cell = cell_from_microdegrees(hydrology.grid, lon_micro, lat_micro);
    let island = *hydrology.island_id.get(cell)?;
    if island == u32::MAX {
        return None;
    }
    let minimum_cell = minimum_cell_for_island(hydrology, island)?;
    landmass_claim(hydrology.grid, minimum_cell)
}

#[must_use]
pub fn resolve(hydrology: &HydrologyField, kind: &str, id: &str) -> Option<LandmassClaim> {
    if kind != KIND_LANDMASS {
        return None;
    }
    let minimum_cell = parse_landmass_id(id)?;
    let island = *hydrology.island_id.get(minimum_cell)?;
    if island == u32::MAX {
        return None;
    }
    if minimum_cell_for_island(hydrology, island) != Some(minimum_cell) {
        return None;
    }
    landmass_claim(hydrology.grid, minimum_cell)
}

fn landmass_claim(grid: Grid, minimum_cell: usize) -> Option<LandmassClaim> {
    let [lon_micro, lat_micro] = coordinate_for_cell(grid, minimum_cell);
    Some(LandmassClaim {
        kind: KIND_LANDMASS,
        id: landmass_id(minimum_cell),
        layer_id: LAYER_LANDMASS,
        label: "Unnamed landmass".into(),
        lon_micro,
        lat_micro,
    })
}

fn minimum_cell_for_island(hydrology: &HydrologyField, island: u32) -> Option<usize> {
    hydrology.island_id.iter().position(|id| *id == island)
}

fn parse_landmass_id(id: &str) -> Option<usize> {
    id.strip_prefix("landmass:")?.parse().ok()
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hydrology::{WaterBalanceMetrics, HYDROLOGY_DERIVATION_VERSION};
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

    #[test]
    fn landmass_claim_uses_minimum_cell() {
        let grid = Grid::new(16, 8, DEFAULT_RADIUS_METRES).unwrap();
        let mut hydrology = empty_hydrology(grid);
        let first = grid.index(2, 4);
        let later = grid.index(2, 5);
        hydrology.island_id[first] = 3;
        hydrology.island_id[later] = 3;
        let [lon, lat] = coordinate_for_cell(grid, later);
        let claim = claim_at(&hydrology, lon, lat).expect("landmass");
        assert_eq!(claim.kind, KIND_LANDMASS);
        assert_eq!(claim.id, landmass_id(first));
        assert!(is_valid_id(KIND_LANDMASS, &claim.id));
        assert!(!is_valid_id(KIND_LANDMASS, "island-000003"));
        assert!(resolve(&hydrology, KIND_LANDMASS, &claim.id).is_some());
        assert!(resolve(&hydrology, KIND_LANDMASS, &landmass_id(later)).is_none());
    }

    #[test]
    fn ocean_has_no_landmass_claim() {
        let grid = Grid::new(16, 8, DEFAULT_RADIUS_METRES).unwrap();
        let hydrology = empty_hydrology(grid);
        let cell = grid.index(3, 8);
        let [lon, lat] = coordinate_for_cell(grid, cell);
        assert!(claim_at(&hydrology, lon, lat).is_none());
    }
}
