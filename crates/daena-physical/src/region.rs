//! Epoch-aware connected-region proposals for authored map layers.
//!
//! Landmass and watershed geometry is derived from hydrology at the viewed
//! epoch. Accepting a proposal copies that geometry into authored GeoJSON; later
//! sea-level changes do not rewrite it.

use serde::{Deserialize, Serialize};

use crate::contours;
use crate::engine::{ToolSurface, TOOL_SURFACE_NONE};
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

/// Region proposal from the canonical cells of a tool surface.
///
/// The selected cells are rasterized onto the requested view through each
/// view pixel's nearest canonical cell, then polygonized with the product
/// contour helpers. Every ring is closed.
pub fn propose_region_canonical(
    surface: &ToolSurface,
    longitude_microdegrees: i32,
    latitude_microdegrees: i32,
    detector: &str,
    view_width: u32,
    view_height: u32,
) -> Result<RegionProposal, PhysicalError> {
    let cell = surface.cell_at_microdegrees(longitude_microdegrees, latitude_microdegrees);
    let (selected, region_id, label) = match detector {
        DETECTOR_LAND => {
            let selected = (0..surface.cell_count())
                .map(|cell| !surface.ocean(cell as u32))
                .collect::<Vec<_>>();
            (selected, 0, "All land".to_string())
        }
        DETECTOR_LANDMASS => {
            let region_id = surface.landmass_id(cell);
            if region_id == TOOL_SURFACE_NONE {
                return Err(PhysicalError::InvalidSettings(
                    "No connected landmass at this point for the current epoch.".into(),
                ));
            }
            let selected = (0..surface.cell_count())
                .map(|cell| surface.landmass_id(cell as u32) == region_id)
                .collect::<Vec<_>>();
            (selected, region_id, format!("Landmass {region_id}"))
        }
        DETECTOR_WATERSHED => {
            if surface.ocean(cell) {
                return Err(PhysicalError::InvalidSettings(
                    "No watershed at this point for the current epoch.".into(),
                ));
            }
            let roots = receiver_roots(surface);
            let outlet = roots[cell as usize];
            if outlet == TOOL_SURFACE_NONE {
                return Err(PhysicalError::InvalidSettings(
                    "No watershed at this point for the current epoch.".into(),
                ));
            }
            let selected = (0..surface.cell_count())
                .map(|cell| {
                    let cell = cell as u32;
                    !surface.ocean(cell) && roots[cell as usize] == outlet
                })
                .collect::<Vec<_>>();
            (selected, outlet, format!("Watershed {outlet}"))
        }
        _ => {
            return Err(PhysicalError::InvalidSettings(
                "unsupported region detector".into(),
            ))
        }
    };
    let cell_count = selected.iter().filter(|value| **value).count() as u32;
    if cell_count == 0 {
        return Err(PhysicalError::InvalidSettings(match detector {
            DETECTOR_LAND => "No exposed land at the current epoch.".into(),
            DETECTOR_LANDMASS => {
                "No connected landmass at this point for the current epoch.".into()
            }
            _ => "No watershed at this point for the current epoch.".into(),
        }));
    }
    if view_width == 0 || view_height == 0 {
        return Err(PhysicalError::InvalidSettings(
            "region view size must be positive".into(),
        ));
    }
    let view = Grid::new(
        view_width,
        view_height,
        surface.radius_metres().round() as u64,
    )
    .map_err(PhysicalError::InvalidSettings)?;
    let mut view_mask = vec![false; view.sample_count()];
    for row in 0..view_height {
        let latitude = -90.0 + 180.0 * (f64::from(row) + 0.5) / f64::from(view_height);
        for col in 0..view_width {
            let longitude = -180.0 + 360.0 * (f64::from(col) + 0.5) / f64::from(view_width);
            let canonical = surface.cell_at_degrees(latitude, longitude);
            view_mask[view.index(row, col)] = selected[canonical as usize];
        }
    }
    let rings = contours::polygons_from_mask_with_cancel(view, &view_mask, &|| false)?
        .into_iter()
        .flat_map(|polygon| polygon.into_iter())
        .map(|mut ring| {
            if ring.len() >= 3 && ring.first() != ring.last() {
                if let Some(first) = ring.first().copied() {
                    ring.push(first);
                }
            }
            ring
        })
        .filter(|ring| ring.len() >= 4)
        .collect::<Vec<_>>();
    if rings.len() > crate::MAX_GEOJSON_FEATURES {
        return Err(PhysicalError::coded(
            crate::PhysicalErrorCode::LimitExceeded,
            "region geometry exceeds the feature budget",
        ));
    }
    if rings.is_empty() {
        return Err(PhysicalError::InvalidSettings(format!(
            "Could not build {detector} geometry for region {region_id}."
        )));
    }
    Ok(RegionProposal {
        detector: detector.to_owned(),
        region_id,
        label,
        epoch_dependent: true,
        cell_count,
        rings,
    })
}

fn receiver_roots(surface: &ToolSurface) -> Vec<u32> {
    let count = surface.cell_count();
    let mut roots = vec![TOOL_SURFACE_NONE; count];
    for start in 0..count {
        if roots[start] != TOOL_SURFACE_NONE {
            continue;
        }
        let mut path = Vec::new();
        let mut cursor = start as u32;
        let root = loop {
            if roots[cursor as usize] != TOOL_SURFACE_NONE {
                break roots[cursor as usize];
            }
            let next = surface.receiver(cursor);
            if next == cursor
                || next == TOOL_SURFACE_NONE
                || next as usize >= count
                || path.len() > count
            {
                roots[cursor as usize] = cursor;
                break cursor;
            }
            path.push(cursor);
            cursor = next;
        };
        for cursor in path {
            roots[cursor as usize] = root;
        }
    }
    roots
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

#[cfg(test)]
mod canonical_tests {
    use super::*;
    use crate::engine::ToolSurface;

    fn paint_band(surface: &mut ToolSurface, min_lat: f64, max_lat: f64) -> u32 {
        let mut first = None;
        for cell in 0..surface.cell_count() as u32 {
            let (lat, _) = surface.cell_lat_lon_degrees(cell);
            if (min_lat..=max_lat).contains(&lat) {
                surface.test_set_land(cell);
                first = Some(first.map_or(cell, |value: u32| value.min(cell)));
            }
        }
        let id = first.expect("band must contain a cell");
        for cell in 0..surface.cell_count() as u32 {
            if !surface.ocean(cell) {
                surface.landmass_id[cell as usize] = id;
            }
        }
        id
    }

    #[test]
    fn canonical_landmass_region_closes_and_keeps_canonical_ids() {
        let mut surface = ToolSurface::synthetic(16);
        let id = paint_band(&mut surface, -20.0, 20.0);
        let point = surface.center_microdegrees(id);
        let proposal =
            propose_region_canonical(&surface, point[0], point[1], DETECTOR_LANDMASS, 64, 32)
                .unwrap();
        assert_eq!(proposal.region_id, id);
        assert_eq!(proposal.label, format!("Landmass {id}"));
        assert!(proposal.cell_count > 0);
        assert!(proposal.epoch_dependent);
        assert!(!proposal.rings.is_empty());
        for ring in &proposal.rings {
            assert!(ring.len() >= 4);
            assert_eq!(ring.first(), ring.last());
            for [lon, lat] in ring {
                assert!((-180_000_000..=180_000_000).contains(lon));
                assert!((-90_000_000..=90_000_000).contains(lat));
            }
        }
    }

    #[test]
    fn canonical_region_differs_when_the_epoch_ocean_moves() {
        let mut reference = ToolSurface::synthetic(16);
        paint_band(&mut reference, -20.0, 20.0);
        let mut epoch = ToolSurface::synthetic(16);
        paint_band(&mut epoch, -20.0, 20.0);
        epoch.sea_level_mm = 120_000;
        for cell in 0..epoch.cell_count() as u32 {
            let (lat, _) = epoch.cell_lat_lon_degrees(cell);
            if lat < 0.0 {
                epoch.test_set_ocean(cell);
                epoch.landmass_id[cell as usize] = TOOL_SURFACE_NONE;
            }
        }
        let point = epoch.center_microdegrees(
            (0..epoch.cell_count() as u32)
                .find(|cell| !epoch.ocean(*cell) && epoch.cell_lat_lon_degrees(*cell).0 >= 0.0)
                .unwrap(),
        );
        let first =
            propose_region_canonical(&reference, point[0], point[1], DETECTOR_LANDMASS, 64, 32)
                .unwrap();
        let second =
            propose_region_canonical(&epoch, point[0], point[1], DETECTOR_LANDMASS, 64, 32)
                .unwrap();
        assert_ne!(first.cell_count, second.cell_count);
        assert_ne!(first.rings, second.rings);
    }

    #[test]
    fn canonical_watershed_region_counts_receiver_root_cells() {
        let mut surface = ToolSurface::synthetic(16);
        let root = 100u32;
        let neighbors = surface.neighbors(root);
        let first = neighbors
            .iter()
            .copied()
            .find(|cell| *cell != TOOL_SURFACE_NONE)
            .unwrap();
        let second = neighbors
            .iter()
            .copied()
            .find(|cell| *cell != TOOL_SURFACE_NONE && *cell != first)
            .unwrap();
        for cell in [root, first, second] {
            surface.test_set_land(cell);
            surface.receiver[cell as usize] = root;
        }
        let point = surface.center_microdegrees(first);
        let proposal =
            propose_region_canonical(&surface, point[0], point[1], DETECTOR_WATERSHED, 64, 32)
                .unwrap();
        assert_eq!(proposal.region_id, root);
        assert_eq!(proposal.cell_count, 3);
        assert!(proposal
            .rings
            .iter()
            .all(|ring| ring.first() == ring.last()));
    }

    #[test]
    fn canonical_region_rejects_unsupported_detectors_and_empty_inputs() {
        let surface = ToolSurface::synthetic(16);
        assert!(propose_region_canonical(&surface, 0, 0, "biome", 64, 32).is_err());
        assert!(propose_region_canonical(&surface, 0, 0, DETECTOR_LAND, 0, 32).is_err());
        assert!(propose_region_canonical(&surface, 0, 0, DETECTOR_LAND, 64, 32).is_err());
        assert!(propose_region_canonical(&surface, 0, 0, DETECTOR_LANDMASS, 64, 32).is_err());
    }
}
