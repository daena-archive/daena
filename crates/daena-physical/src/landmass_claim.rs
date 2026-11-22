//! Stable identities for generated landmasses.
//!
//! Hydrology assigns `island_id` in discovery order. Claims instead key a
//! landmass by the lowest cell index in that connected land so a name can
//! follow the land across overlay redraws without copying geometry.

use crate::engine::{ToolSurface, TOOL_SURFACE_NONE};

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

/// Canonical claim ids carry a `c` marker so a retired view-cell id such as
/// `landmass:1234` cannot pass the format validator.
#[must_use]
pub fn canonical_landmass_id(minimum_cell: u32) -> String {
    format!("landmass:c{minimum_cell}")
}

#[must_use]
pub fn is_valid_id(kind: &str, id: &str) -> bool {
    kind == KIND_LANDMASS && parse_canonical_landmass_id(id).is_some()
}

/// Landmass claim on the canonical cube cells of a tool surface.
#[must_use]
pub fn claim_at_canonical(
    surface: &ToolSurface,
    lon_micro: i32,
    lat_micro: i32,
) -> Option<LandmassClaim> {
    let cell = surface.cell_at_microdegrees(lon_micro, lat_micro);
    let minimum_cell = surface.landmass_id(cell);
    if minimum_cell == TOOL_SURFACE_NONE {
        return None;
    }
    Some(landmass_claim_canonical(surface, minimum_cell))
}

/// Resolve a canonical landmass id in one surface epoch. The id is the lowest
/// cell of the component, so an id stops resolving after its component merges
/// into a lower-id landmass.
#[must_use]
pub fn resolve_canonical(surface: &ToolSurface, kind: &str, id: &str) -> Option<LandmassClaim> {
    if kind != KIND_LANDMASS {
        return None;
    }
    let minimum_cell = parse_canonical_landmass_id(id)?;
    if surface.landmass_id(minimum_cell) != minimum_cell {
        return None;
    }
    Some(landmass_claim_canonical(surface, minimum_cell))
}

fn landmass_claim_canonical(surface: &ToolSurface, minimum_cell: u32) -> LandmassClaim {
    let [lon_micro, lat_micro] = surface.center_microdegrees(minimum_cell);
    LandmassClaim {
        kind: KIND_LANDMASS,
        id: canonical_landmass_id(minimum_cell),
        layer_id: LAYER_LANDMASS,
        label: "Unnamed landmass".into(),
        lon_micro,
        lat_micro,
    }
}

fn parse_canonical_landmass_id(id: &str) -> Option<u32> {
    let digits = id.strip_prefix("landmass:c")?;
    if digits.is_empty() || !digits.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    digits.parse().ok()
}

#[cfg(test)]
mod canonical_tests {
    use super::*;
    use crate::engine::ToolSurface;

    fn surface_with_landmass(cells: &[u32], id: u32) -> ToolSurface {
        let mut surface = ToolSurface::synthetic(16);
        for cell in cells {
            surface.test_set_land(*cell);
            surface.landmass_id[*cell as usize] = id;
        }
        surface
    }

    #[test]
    fn canonical_landmass_claim_uses_the_lowest_cell_and_merges_by_epoch() {
        let reference = surface_with_landmass(&[10, 11, 20, 21], 10);
        let point = reference.center_microdegrees(11);
        let claim = claim_at_canonical(&reference, point[0], point[1]).expect("landmass");
        assert_eq!(claim.id, "landmass:c10");
        assert_eq!(
            resolve_canonical(&reference, KIND_LANDMASS, &claim.id)
                .expect("reference")
                .id,
            claim.id
        );
        assert!(is_valid_id(KIND_LANDMASS, "landmass:c10"));
        assert!(!is_valid_id(KIND_LANDMASS, "landmass:10"));

        let merged = surface_with_landmass(&[10, 11, 20, 21], 10);
        assert_eq!(
            resolve_canonical(&merged, KIND_LANDMASS, "landmass:c20").map(|claim| claim.id),
            None
        );
        assert_eq!(
            resolve_canonical(&merged, KIND_LANDMASS, "landmass:c10")
                .expect("merged")
                .id,
            "landmass:c10"
        );
    }

    #[test]
    fn canonical_landmass_claim_rejects_ocean() {
        let surface = ToolSurface::synthetic(16);
        let point = surface.center_microdegrees(10);
        assert!(claim_at_canonical(&surface, point[0], point[1]).is_none());
    }
}
