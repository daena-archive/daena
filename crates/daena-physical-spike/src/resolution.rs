//! Measured production-resolution policy for the native physical world.
//!
//! Feature sizes are authored in metres. They are deliberately independent of
//! UI pixels and are used to classify a grid as preview-only or production
//! capable before a resolution is selected.

use crate::{Grid, DEFAULT_RADIUS_METRES};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResolutionTier {
    Preview,
    ProductionDefault,
    ProductionMaximum,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResolutionCandidate {
    pub width: u32,
    pub height: u32,
    pub tier: ResolutionTier,
}

pub const RESOLUTION_CANDIDATES: [ResolutionCandidate; 4] = [
    ResolutionCandidate {
        width: 256,
        height: 128,
        tier: ResolutionTier::ProductionDefault,
    },
    ResolutionCandidate {
        width: 512,
        height: 256,
        tier: ResolutionTier::Preview,
    },
    ResolutionCandidate {
        width: 1024,
        height: 512,
        tier: ResolutionTier::Preview,
    },
    ResolutionCandidate {
        width: 2048,
        height: 1024,
        tier: ResolutionTier::Preview,
    },
];

/// Controlled physical fixtures used by the resolution gate.
///
/// The dimensions describe the smallest retained feature in each category,
/// not a rendered pixel size. The internal-shape fixture is evaluated with the
/// stricter eight-sample rule.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FeatureFixtureSpec {
    pub trench_half_width_metres: u64,
    pub trench_arc_offset_metres: u64,
    pub collision_belt_width_metres: u64,
    pub rift_floor_width_metres: u64,
    pub rift_shoulder_width_metres: u64,
    pub shelf_width_metres: u64,
    pub slope_width_metres: u64,
    pub retained_strait_width_metres: u64,
    pub retained_island_width_metres: u64,
    pub lake_sill_width_metres: u64,
    pub drainage_divide_width_metres: u64,
    pub displayed_tributary_catchment_metres: u64,
    pub internal_shape_width_metres: u64,
}

/// Reviewable metre-based fixture specification for the first production
/// resolution decision. It intentionally describes author-visible features at
/// the supported world scale rather than pretending to preserve sub-cell
/// islands or drainage divides.
pub const FEATURE_FIXTURES: FeatureFixtureSpec = FeatureFixtureSpec {
    trench_half_width_metres: 800_000,
    trench_arc_offset_metres: 1_200_000,
    collision_belt_width_metres: 1_000_000,
    rift_floor_width_metres: 750_000,
    rift_shoulder_width_metres: 1_500_000,
    shelf_width_metres: 750_000,
    slope_width_metres: 1_500_000,
    retained_strait_width_metres: 650_000,
    retained_island_width_metres: 750_000,
    lake_sill_width_metres: 650_000,
    drainage_divide_width_metres: 650_000,
    displayed_tributary_catchment_metres: 1_000_000,
    internal_shape_width_metres: 1_300_000,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResolutionAssessment {
    pub candidate: ResolutionCandidate,
    pub max_cell_width_metres: u64,
    pub minimum_feature_samples: u32,
    pub internal_shape_samples: u32,
    pub meets_four_sample_gate: bool,
    pub meets_eight_sample_gate: bool,
}

impl ResolutionAssessment {
    pub const fn production_eligible(self) -> bool {
        self.meets_four_sample_gate
            && self.meets_eight_sample_gate
            && !matches!(self.candidate.tier, ResolutionTier::Preview)
    }
}

fn ceil_samples(feature_metres: u64, cell_metres: u64) -> u32 {
    feature_metres
        .saturating_add(cell_metres.saturating_sub(1))
        .checked_div(cell_metres.max(1))
        .unwrap_or(u64::MAX)
        .min(u64::from(u32::MAX)) as u32
}

/// Returns the conservative largest cell dimension in metres. A feature may
/// run either along a latitude row or along a longitude row at the equator;
/// using the larger dimension keeps the gate independent of orientation.
pub fn max_cell_width_metres(width: u32, height: u32, radius_metres: u64) -> u64 {
    let radius = radius_metres as f64;
    let longitudinal = std::f64::consts::TAU * radius / f64::from(width.max(1));
    let meridional = std::f64::consts::PI * radius / f64::from(height.max(1));
    longitudinal.max(meridional).ceil() as u64
}

pub fn assess(candidate: ResolutionCandidate) -> ResolutionAssessment {
    let cell = max_cell_width_metres(candidate.width, candidate.height, DEFAULT_RADIUS_METRES);
    let minimum_feature_samples = [
        FEATURE_FIXTURES.trench_half_width_metres,
        FEATURE_FIXTURES.trench_arc_offset_metres,
        FEATURE_FIXTURES.collision_belt_width_metres,
        FEATURE_FIXTURES.rift_floor_width_metres,
        FEATURE_FIXTURES.rift_shoulder_width_metres,
        FEATURE_FIXTURES.shelf_width_metres,
        FEATURE_FIXTURES.slope_width_metres,
        FEATURE_FIXTURES.retained_strait_width_metres,
        FEATURE_FIXTURES.retained_island_width_metres,
        FEATURE_FIXTURES.lake_sill_width_metres,
        FEATURE_FIXTURES.drainage_divide_width_metres,
        FEATURE_FIXTURES.displayed_tributary_catchment_metres,
    ]
    .into_iter()
    .map(|feature| ceil_samples(feature, cell))
    .min()
    .unwrap_or(0);
    let internal_shape_samples = ceil_samples(FEATURE_FIXTURES.internal_shape_width_metres, cell);
    ResolutionAssessment {
        candidate,
        max_cell_width_metres: cell,
        minimum_feature_samples,
        internal_shape_samples,
        meets_four_sample_gate: minimum_feature_samples >= 4,
        meets_eight_sample_gate: internal_shape_samples >= 8,
    }
}

pub fn assess_all() -> [ResolutionAssessment; RESOLUTION_CANDIDATES.len()] {
    RESOLUTION_CANDIDATES.map(assess)
}

pub fn is_supported_preview_grid(grid: Grid) -> bool {
    RESOLUTION_CANDIDATES
        .iter()
        .any(|candidate| candidate.width == grid.width && candidate.height == grid.height)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn feature_fixture_is_metre_based_and_selected_tiers_meet_sampling_gate() {
        let assessments = assess_all();
        assert!(assessments[0].production_eligible());
        assert!(!assessments[1].production_eligible());
        assert!(!assessments[2].production_eligible());
        assert!(assessments[3].meets_four_sample_gate);
        assert!(assessments[3].meets_eight_sample_gate);
        assert!(FEATURE_FIXTURES.retained_strait_width_metres > 0);
        assert!(FEATURE_FIXTURES.internal_shape_width_metres > 0);
    }

    #[test]
    fn conservative_cell_width_uses_the_larger_spherical_dimension() {
        let equatorial_longitude = max_cell_width_metres(256, 128, DEFAULT_RADIUS_METRES);
        let polar_longitude = max_cell_width_metres(128, 256, DEFAULT_RADIUS_METRES);
        assert!(equatorial_longitude < polar_longitude);
        assert_eq!(equatorial_longitude, 156_368);
    }
}
