//! Stable identities for generated lakes and rivers.
//!
//! Derived hydrology GeoJSON uses list indexes. Claims instead key a lake by
//! basin `minimum_cell` and a river by `source_cell` so a name can follow the
//! spine when sea-level moves the mouth.

use crate::engine::ToolSurface;

pub const KIND_LAKE: &str = "physical-lake";
pub const KIND_RIVER: &str = "physical-river";
pub const KIND_DERIVED_FLOOD: &str = "derived-flood";
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

/// Canonical claim ids carry a `c` marker so a retired view-cell id such as
/// `lake:1234` cannot pass the format validator.
#[must_use]
pub fn canonical_lake_id(minimum_cell: u32) -> String {
    format!("lake:c{minimum_cell}")
}

#[must_use]
pub fn canonical_river_id(source_cell: u32) -> String {
    format!("river:c{source_cell}")
}

#[must_use]
pub fn canonical_flood_id(minimum_cell: u32) -> String {
    format!("flood:c{minimum_cell}")
}

#[must_use]
pub fn is_valid_id(kind: &str, id: &str) -> bool {
    match kind {
        KIND_LAKE => parse_canonical_cell("lake:c", id).is_some(),
        KIND_RIVER => parse_canonical_cell("river:c", id).is_some(),
        KIND_DERIVED_FLOOD => parse_canonical_cell("flood:c", id).is_some(),
        _ => false,
    }
}

/// Canonical lake or river claim at a point, lake first.
#[must_use]
pub fn claim_at_canonical(
    surface: &ToolSurface,
    lon_micro: i32,
    lat_micro: i32,
    radius_micro: i32,
) -> Option<HydroClaim> {
    lake_at_canonical(surface, lon_micro, lat_micro).or_else(|| {
        river_at_canonical(
            surface,
            lon_micro,
            lat_micro,
            i64::from(radius_micro.max(1)),
        )
    })
}

/// Resolve a canonical claim id in one surface epoch.
///
/// A lake resolves only while it holds water at this epoch; a merged or dry
/// lake is unresolved. A river resolves while its head cell still starts a
/// river segment.
#[must_use]
pub fn resolve_canonical(surface: &ToolSurface, kind: &str, id: &str) -> Option<HydroClaim> {
    match kind {
        KIND_LAKE => {
            let cell = parse_canonical_cell("lake:c", id)?;
            let lake = surface.lake_at_minimum_cell(cell)?;
            lake.has_water
                .then(|| lake_claim_canonical(surface, lake.minimum_cell))
        }
        KIND_RIVER => {
            let source = parse_canonical_cell("river:c", id)?;
            surface
                .river_at_source_cell(source)
                .map(|river| river_claim_canonical(surface, river))
        }
        KIND_DERIVED_FLOOD => {
            let cell = parse_canonical_cell("flood:c", id)?;
            let lake = surface.lake_at_minimum_cell(cell)?;
            lake.overflowing
                .then(|| overflowing_claim_canonical(surface, lake.minimum_cell))
        }
        _ => None,
    }
}

/// Overflowing (fresh, spilling) lake at a point.
#[must_use]
pub fn overflowing_at_canonical(
    surface: &ToolSurface,
    lon_micro: i32,
    lat_micro: i32,
) -> Option<HydroClaim> {
    let cell = surface.cell_at_microdegrees(lon_micro, lat_micro);
    let lake = surface.lake(surface.lake_id(cell))?;
    lake.overflowing
        .then(|| overflowing_claim_canonical(surface, lake.minimum_cell))
}

/// Every overflowing lake at this surface epoch.
#[must_use]
pub fn overflowing_claims_canonical(surface: &ToolSurface) -> Vec<HydroClaim> {
    surface
        .lakes
        .iter()
        .filter(|lake| lake.overflowing)
        .map(|lake| overflowing_claim_canonical(surface, lake.minimum_cell))
        .collect()
}

fn lake_at_canonical(surface: &ToolSurface, lon_micro: i32, lat_micro: i32) -> Option<HydroClaim> {
    let cell = surface.cell_at_microdegrees(lon_micro, lat_micro);
    let lake = surface.lake(surface.lake_id(cell))?;
    if !lake.has_water {
        return None;
    }
    Some(lake_claim_canonical(surface, lake.minimum_cell))
}

fn lake_claim_canonical(surface: &ToolSurface, minimum_cell: u32) -> HydroClaim {
    let [lon_micro, lat_micro] = surface.center_microdegrees(minimum_cell);
    HydroClaim {
        kind: KIND_LAKE,
        id: canonical_lake_id(minimum_cell),
        layer_id: LAYER_LAKES,
        label: "Unnamed lake".into(),
        lon_micro,
        lat_micro,
    }
}

fn overflowing_claim_canonical(surface: &ToolSurface, minimum_cell: u32) -> HydroClaim {
    let [lon_micro, lat_micro] = surface.center_microdegrees(minimum_cell);
    HydroClaim {
        kind: KIND_DERIVED_FLOOD,
        id: canonical_flood_id(minimum_cell),
        layer_id: LAYER_LAKES,
        label: "Overflowing basin".into(),
        lon_micro,
        lat_micro,
    }
}

fn river_at_canonical(
    surface: &ToolSurface,
    lon_micro: i32,
    lat_micro: i32,
    radius: i64,
) -> Option<HydroClaim> {
    let radius2 = radius.saturating_mul(radius);
    let cos_lat = (f64::from(lat_micro) / 1_000_000.0)
        .to_radians()
        .cos()
        .clamp(0.0, 1.0);
    let mut best: Option<(i64, HydroClaim)> = None;
    for river in &surface.rivers {
        let Some(distance2) = river
            .cells
            .iter()
            .map(|cell| {
                let [river_lon, river_lat] = surface.center_microdegrees(*cell);
                ground_distance2(river_lon, river_lat, lon_micro, lat_micro, cos_lat)
            })
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
        best = Some((distance2, river_claim_canonical(surface, river)));
    }
    best.map(|(_, claim)| claim)
}

fn river_claim_canonical(surface: &ToolSurface, river: &crate::engine::SurfaceRiver) -> HydroClaim {
    let [lon_micro, lat_micro] = surface.center_microdegrees(river.source_cell);
    HydroClaim {
        kind: KIND_RIVER,
        id: canonical_river_id(river.source_cell),
        layer_id: LAYER_RIVERS,
        label: format!("River order {}", river.strahler_order),
        lon_micro,
        lat_micro,
    }
}

fn parse_canonical_cell(prefix: &str, id: &str) -> Option<u32> {
    let digits = id.strip_prefix(prefix)?;
    if digits.is_empty() || !digits.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    digits.parse().ok()
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

/// Square distance in micro-degrees of latitude: the longitude difference is
/// wrapped across the antimeridian and scaled by `cos(latitude)`.
fn ground_distance2(lon_a: i32, lat_a: i32, lon_b: i32, lat_b: i32, cos_lat: f64) -> i64 {
    let dlon = wrap_dlon(lon_a, lon_b) as f64 * cos_lat;
    let dlat = f64::from(lat_a) - f64::from(lat_b);
    (dlon * dlon + dlat * dlat).round() as i64
}

#[cfg(test)]
mod canonical_tests {
    use super::*;
    use crate::engine::{SurfaceLake, SurfaceRiver, ToolSurface};

    fn land_surface(n: u32) -> ToolSurface {
        let mut surface = ToolSurface::synthetic(n);
        for cell in 0..surface.cell_count() as u32 {
            surface.test_set_land(cell);
        }
        surface
    }

    fn lake(minimum_cell: u32, spill_cell: u32, overflowing: bool) -> SurfaceLake {
        SurfaceLake {
            id: 1,
            minimum_cell,
            spill_cell,
            fresh: true,
            overflowing,
            has_water: true,
            cell_count: 2,
        }
    }

    #[test]
    fn canonical_lake_and_flood_ids_round_trip_at_the_reference_and_epoch() {
        let mut surface = land_surface(16);
        let minimum = 42u32;
        let shore = 43u32;
        surface.lake_id[minimum as usize] = 1;
        surface.lake_id[shore as usize] = 1;
        surface.lakes.push(lake(minimum, shore, true));
        let point = surface.center_microdegrees(shore);
        let claim = claim_at_canonical(&surface, point[0], point[1], 50_000).expect("lake");
        assert_eq!(claim.kind, KIND_LAKE);
        assert_eq!(claim.id, "lake:c42");
        assert_eq!(
            resolve_canonical(&surface, KIND_LAKE, &claim.id)
                .expect("reference")
                .id,
            claim.id
        );
        let flood = overflowing_at_canonical(&surface, point[0], point[1]).expect("flood");
        assert_eq!(flood.id, "flood:c42");
        assert_eq!(
            resolve_canonical(&surface, KIND_DERIVED_FLOOD, &flood.id)
                .expect("reference")
                .id,
            flood.id
        );
        assert_eq!(overflowing_claims_canonical(&surface).len(), 1);

        let epoch = land_surface(16);
        assert!(resolve_canonical(&epoch, KIND_LAKE, "lake:c42").is_none());
        assert!(resolve_canonical(&epoch, KIND_DERIVED_FLOOD, "flood:c42").is_none());
    }

    #[test]
    fn canonical_river_ids_round_trip_and_old_ids_do_not() {
        let mut surface = land_surface(16);
        surface.rivers.push(SurfaceRiver {
            source_cell: 7,
            mouth_cell: 8,
            strahler_order: 1,
            cells: vec![7, 8],
        });
        let point = surface.center_microdegrees(7);
        let claim = claim_at_canonical(&surface, point[0], point[1], 50_000).expect("river");
        assert_eq!(claim.id, "river:c7");
        assert_eq!(
            resolve_canonical(&surface, KIND_RIVER, &claim.id)
                .expect("river")
                .id,
            claim.id
        );
        assert!(is_valid_id(KIND_RIVER, "river:c7"));
        assert!(!is_valid_id(KIND_RIVER, "river:7"));
        let epoch = land_surface(16);
        assert!(resolve_canonical(&epoch, KIND_RIVER, "river:c7").is_none());
    }

    #[test]
    fn river_pick_wraps_the_antimeridian_and_scales_longitude_by_latitude() {
        let mut surface = land_surface(128);
        let cell = surface.cell_at_degrees(0.0, 179.9);
        let [_river_lon, river_lat] = surface.center_microdegrees(cell);
        surface.rivers.push(SurfaceRiver {
            source_cell: cell,
            mouth_cell: cell,
            strahler_order: 1,
            cells: vec![cell],
        });
        let claim = claim_at_canonical(&surface, -179_900_000, river_lat, 500_000)
            .expect("river across the antimeridian");
        assert_eq!(claim.id, canonical_river_id(cell));

        let high = surface.cell_at_degrees(70.0, 0.0);
        let [high_lon, high_lat] = surface.center_microdegrees(high);
        surface.rivers.clear();
        surface.rivers.push(SurfaceRiver {
            source_cell: high,
            mouth_cell: high,
            strahler_order: 1,
            cells: vec![high],
        });
        let cos_lat = (f64::from(high_lat) / 1_000_000.0)
            .to_radians()
            .cos()
            .max(0.01);
        let mut last_picked = None;
        for delta in (0..3_000_000).step_by(20_000) {
            if claim_at_canonical(&surface, high_lon.saturating_add(delta), high_lat, 500_000)
                .is_some()
            {
                last_picked = Some(delta);
            }
        }
        let picked = last_picked.expect("east-west reach at 70 N");
        assert!(
            picked > 1_000_000,
            "longitude reach {picked} did not scale by cos(latitude)"
        );
        assert!(
            f64::from(picked) * cos_lat <= 520_000.0,
            "picked longitude {picked} is beyond the ground radius"
        );
        assert!(
            f64::from(picked + 1_000_000) * cos_lat > 500_000.0,
            "picker accepted a longitude far beyond the ground radius"
        );
        assert!(
            claim_at_canonical(
                &surface,
                high_lon,
                high_lat.saturating_add(400_000),
                500_000
            )
            .is_some(),
            "0.4 degrees north is inside the radius"
        );
    }

    #[test]
    fn dry_lakes_do_not_resolve_or_claim() {
        let mut surface = land_surface(16);
        let minimum = 42u32;
        surface.lake_id[minimum as usize] = 1;
        let mut dry = lake(minimum, minimum + 1, false);
        dry.has_water = false;
        surface.lakes.push(dry);
        let point = surface.center_microdegrees(minimum);
        assert!(claim_at_canonical(&surface, point[0], point[1], 50_000).is_none());
        assert!(resolve_canonical(&surface, KIND_LAKE, "lake:c42").is_none());
    }
}
