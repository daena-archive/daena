//! Epoch-aware connected-region proposals for authored map layers.
//!
//! Landmass and watershed geometry is derived from hydrology at the viewed
//! epoch. Accepting a proposal copies that geometry into authored GeoJSON; later
//! sea-level changes do not rewrite it.

use serde::{Deserialize, Serialize};

use crate::contours;
use crate::engine::{receiver_roots, ToolSurface, TOOL_SURFACE_NONE};
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
            let rank = surface.landmass_rank(region_id);
            let label = if rank == 0 {
                format!("Landmass {region_id}")
            } else {
                format!("Landmass {rank}")
            };
            (selected, region_id, label)
        }
        DETECTOR_WATERSHED => {
            if surface.ocean(cell) {
                return Err(PhysicalError::InvalidSettings(
                    "No watershed at this point for the current epoch.".into(),
                ));
            }
            let roots = receiver_roots(&surface.receiver);
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
            let rank = surface.watershed_rank(outlet);
            let label = if rank == 0 {
                format!("Watershed {outlet}")
            } else {
                format!("Watershed {rank}")
            };
            (selected, outlet, label)
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
        assert_eq!(proposal.label, "Landmass 1");
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
    fn canonical_landmass_labels_are_size_ranks_across_calls() {
        let mut surface = ToolSurface::synthetic(16);
        let count = surface.cell_count() as u32;
        for cell in 0..12 {
            surface.test_set_land(cell);
            surface.landmass_id[cell as usize] = 500;
        }
        for cell in 40..48 {
            surface.test_set_land(cell);
            surface.landmass_id[cell as usize] = 700;
        }
        let large = surface.center_microdegrees(0);
        let small = surface.center_microdegrees(40);
        let first =
            propose_region_canonical(&surface, large[0], large[1], DETECTOR_LANDMASS, 64, 32)
                .unwrap();
        let second =
            propose_region_canonical(&surface, large[0], large[1], DETECTOR_LANDMASS, 64, 32)
                .unwrap();
        let smaller =
            propose_region_canonical(&surface, small[0], small[1], DETECTOR_LANDMASS, 64, 32)
                .unwrap();
        assert_eq!(first.region_id, 500);
        assert_eq!(smaller.region_id, 700);
        assert_eq!(first.label, "Landmass 1");
        assert_eq!(second.label, first.label);
        assert_eq!(smaller.label, "Landmass 2");
        assert!(first.cell_count > smaller.cell_count);
        assert!(count > 48);
    }

    #[test]
    fn canonical_watershed_labels_are_size_ranks_across_calls() {
        let mut surface = ToolSurface::synthetic(16);
        let roots = [100u32, 200u32];
        for root in roots {
            surface.test_set_land(root);
            surface.receiver[root as usize] = root;
        }
        for cell in [101u32, 102, 103, 104, 105] {
            surface.test_set_land(cell);
            surface.receiver[cell as usize] = roots[0];
        }
        for cell in [201u32, 202, 203] {
            surface.test_set_land(cell);
            surface.receiver[cell as usize] = roots[1];
        }
        let large = surface.center_microdegrees(101);
        let small = surface.center_microdegrees(201);
        let first =
            propose_region_canonical(&surface, large[0], large[1], DETECTOR_WATERSHED, 64, 32)
                .unwrap();
        let second =
            propose_region_canonical(&surface, large[0], large[1], DETECTOR_WATERSHED, 64, 32)
                .unwrap();
        let smaller =
            propose_region_canonical(&surface, small[0], small[1], DETECTOR_WATERSHED, 64, 32)
                .unwrap();
        assert_eq!(first.region_id, roots[0]);
        assert_eq!(second.label, first.label);
        assert_eq!(first.label, "Watershed 1");
        assert_eq!(smaller.label, "Watershed 2");
        assert!(first.cell_count > smaller.cell_count);
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
