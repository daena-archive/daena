//! Epoch-aware connected-region proposals for authored map layers.
//!
//! Landmass and watershed geometry is derived from hydrology at the viewed
//! epoch. Accepting a proposal copies that geometry into authored GeoJSON; later
//! sea-level changes do not rewrite it.

use serde::{Deserialize, Serialize};

use crate::hydrology::HydrologyField;
use crate::{Grid, PhysicalError};

pub const DETECTOR_LANDMASS: &str = "landmass";
pub const DETECTOR_WATERSHED: &str = "watershed";
pub const DETECTOR_LAND: &str = "land";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RegionProposal {
    pub detector: String,
    pub region_id: u32,
    pub label: String,
    pub epoch_dependent: bool,
    pub cell_count: u32,
    pub rings: Vec<Vec<[i32; 2]>>,
}

pub fn propose_region(
    hydrology: &HydrologyField,
    longitude_microdegrees: i32,
    latitude_microdegrees: i32,
    detector: &str,
) -> Result<RegionProposal, PhysicalError> {
    let count = hydrology.grid.sample_count();
    if hydrology.island_id.len() != count || hydrology.watershed_id.len() != count {
        return Err(PhysicalError::InvalidSettings(
            "hydrology region fields do not match the grid".into(),
        ));
    }
    match detector {
        DETECTOR_LAND => all_land(hydrology),
        DETECTOR_LANDMASS => {
            let cell = cell_from_microdegrees(
                hydrology.grid,
                longitude_microdegrees,
                latitude_microdegrees,
            );
            let id = hydrology.island_id.get(cell).copied().unwrap_or(u32::MAX);
            if id == u32::MAX {
                return Err(PhysicalError::InvalidSettings(
                    "No connected landmass at this point for the current epoch.".into(),
                ));
            }
            let rings = hydrology
                .island_polygons
                .get(id as usize)
                .cloned()
                .unwrap_or_default();
            if rings.iter().filter(|ring| ring.len() >= 3).count() == 0 {
                return Err(PhysicalError::InvalidSettings(format!(
                    "Could not build landmass geometry for region {id}."
                )));
            }
            Ok(RegionProposal {
                detector: DETECTOR_LANDMASS.into(),
                region_id: id,
                label: format!("Landmass {}", id + 1),
                epoch_dependent: true,
                cell_count: hydrology
                    .island_id
                    .iter()
                    .filter(|value| **value == id)
                    .count() as u32,
                rings,
            })
        }
        DETECTOR_WATERSHED => {
            let cell = cell_from_microdegrees(
                hydrology.grid,
                longitude_microdegrees,
                latitude_microdegrees,
            );
            let outlet = hydrology
                .watershed_id
                .get(cell)
                .copied()
                .unwrap_or(u32::MAX);
            let Some(index) = watershed_polygon_index(&hydrology.watershed_id, outlet) else {
                return Err(PhysicalError::InvalidSettings(
                    "No watershed at this point for the current epoch.".into(),
                ));
            };
            let rings = hydrology
                .watershed_polygons
                .get(index)
                .cloned()
                .unwrap_or_default();
            if rings.iter().filter(|ring| ring.len() >= 3).count() == 0 {
                return Err(PhysicalError::InvalidSettings(format!(
                    "Could not build watershed geometry for region {index}."
                )));
            }
            Ok(RegionProposal {
                detector: DETECTOR_WATERSHED.into(),
                region_id: index as u32,
                label: format!("Watershed {}", index + 1),
                epoch_dependent: true,
                cell_count: hydrology
                    .watershed_id
                    .iter()
                    .filter(|value| **value == outlet)
                    .count() as u32,
                rings,
            })
        }
        _ => Err(PhysicalError::InvalidSettings(
            "unsupported region detector".into(),
        )),
    }
}

fn all_land(hydrology: &HydrologyField) -> Result<RegionProposal, PhysicalError> {
    let rings: Vec<Vec<[i32; 2]>> = hydrology
        .land_polygons
        .iter()
        .flat_map(|polygon| polygon.iter().cloned())
        .filter(|ring| ring.len() >= 3)
        .collect();
    if rings.is_empty() {
        return Err(PhysicalError::InvalidSettings(
            "No exposed land at the current epoch.".into(),
        ));
    }
    Ok(RegionProposal {
        detector: DETECTOR_LAND.into(),
        region_id: 0,
        label: "All land".into(),
        epoch_dependent: true,
        cell_count: hydrology
            .island_id
            .iter()
            .filter(|id| **id != u32::MAX)
            .count() as u32,
        rings,
    })
}

fn watershed_polygon_index(ids: &[u32], outlet: u32) -> Option<usize> {
    if outlet == u32::MAX {
        return None;
    }
    let mut unique: Vec<u32> = ids.iter().copied().filter(|id| *id != u32::MAX).collect();
    unique.sort_unstable();
    unique.dedup();
    unique.iter().position(|id| *id == outlet)
}

fn cell_from_microdegrees(grid: Grid, lon: i32, lat: i32) -> usize {
    let col = ((i64::from(lon) + 180_000_000) * i64::from(grid.width) / 360_000_000)
        .clamp(0, i64::from(grid.width) - 1) as u32;
    let row = ((i64::from(lat) + 90_000_000) * i64::from(grid.height) / 180_000_000)
        .clamp(0, i64::from(grid.height) - 1) as u32;
    grid.index(row, col)
}

#[cfg(test)]
fn cell_center_microdegrees(grid: Grid, cell: usize) -> [i32; 2] {
    let (row, col) = grid.row_col(cell);
    let (lon, lat) = grid.center_radians(row, col);
    [
        (lon.to_degrees() * 1_000_000.0).round() as i32,
        (lat.to_degrees() * 1_000_000.0).round() as i32,
    ]
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
            watershed_id: vec![u32::MAX; count],
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

    fn box_ring(origin: [i32; 2]) -> Vec<[i32; 2]> {
        vec![
            origin,
            [origin[0] + 1_000_000, origin[1]],
            [origin[0] + 1_000_000, origin[1] + 1_000_000],
            origin,
        ]
    }

    fn paint_island(
        hydrology: &mut HydrologyField,
        cols: std::ops::Range<u32>,
        rows: std::ops::Range<u32>,
        island: u32,
    ) {
        while hydrology.island_polygons.len() <= island as usize {
            hydrology.island_polygons.push(Vec::new());
        }
        let mut first = None;
        for row in rows {
            for col in cols.clone() {
                let cell = hydrology.grid.index(row, col);
                hydrology.island_id[cell] = island;
                hydrology.water_level_mm[cell] = 120_000;
                if first.is_none() {
                    first = Some(cell_center_microdegrees(hydrology.grid, cell));
                }
            }
        }
        if let Some(origin) = first {
            hydrology.island_polygons[island as usize] = vec![box_ring(origin)];
        }
    }

    #[test]
    fn ocean_click_has_no_landmass() {
        let grid = Grid::new(16, 8, DEFAULT_RADIUS_METRES).unwrap();
        let hydrology = empty_hydrology(grid);
        let [lon, lat] = cell_center_microdegrees(grid, grid.index(3, 8));
        let error = propose_region(&hydrology, lon, lat, DETECTOR_LANDMASS).unwrap_err();
        assert!(error.to_string().contains("No connected landmass"));
    }

    #[test]
    fn landmass_click_returns_the_whole_connected_component() {
        let grid = Grid::new(16, 8, DEFAULT_RADIUS_METRES).unwrap();
        let mut hydrology = empty_hydrology(grid);
        paint_island(&mut hydrology, 1..5, 2..6, 0);
        paint_island(&mut hydrology, 11..15, 2..6, 1);
        let west = cell_center_microdegrees(grid, grid.index(3, 2));
        let east = cell_center_microdegrees(grid, grid.index(4, 13));
        let left = propose_region(&hydrology, west[0], west[1], DETECTOR_LANDMASS).unwrap();
        let right = propose_region(&hydrology, east[0], east[1], DETECTOR_LANDMASS).unwrap();
        assert_eq!(left.region_id, 0);
        assert_eq!(right.region_id, 1);
        assert_eq!(left.cell_count, 16);
        assert_eq!(right.cell_count, 16);
        assert!(left.epoch_dependent);
        assert_ne!(left.rings, right.rings);
        assert_eq!(left.label, "Landmass 1");
    }

    #[test]
    fn land_detector_returns_every_exposed_ring() {
        let grid = Grid::new(16, 8, DEFAULT_RADIUS_METRES).unwrap();
        let mut hydrology = empty_hydrology(grid);
        paint_island(&mut hydrology, 1..5, 2..6, 0);
        paint_island(&mut hydrology, 11..15, 2..6, 1);
        hydrology.land_polygons = hydrology.island_polygons.clone();
        let [lon, lat] = cell_center_microdegrees(grid, grid.index(0, 0));
        let land = propose_region(&hydrology, lon, lat, DETECTOR_LAND).unwrap();
        assert_eq!(land.detector, DETECTOR_LAND);
        assert_eq!(land.rings.len(), 2);
        assert_eq!(land.cell_count, 32);
        assert!(land.epoch_dependent);
    }

    #[test]
    fn watershed_index_matches_sorted_outlet_order() {
        let ids = [u32::MAX, 40, 10, 40, 10, u32::MAX, 25];
        assert_eq!(watershed_polygon_index(&ids, 10), Some(0));
        assert_eq!(watershed_polygon_index(&ids, 25), Some(1));
        assert_eq!(watershed_polygon_index(&ids, 40), Some(2));
        assert_eq!(watershed_polygon_index(&ids, u32::MAX), None);
        assert_eq!(watershed_polygon_index(&ids, 99), None);
    }

    #[test]
    fn unsupported_detector_is_rejected() {
        let grid = Grid::new(16, 8, DEFAULT_RADIUS_METRES).unwrap();
        let hydrology = empty_hydrology(grid);
        assert!(propose_region(&hydrology, 0, 0, "biome").is_err());
    }
}
