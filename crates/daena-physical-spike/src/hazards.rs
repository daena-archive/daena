//! Deterministic tectonic and volcanic hazard fields.
//!
//! These are bounded relative/generated hazards derived from accepted tectonic
//! structure. They are not real-world predictions. Event materialization
//! consumes these fields through an explicit versioned model.

use std::cmp::Reverse;

use super::{
    derive_subsystem_seed, splitmix64,
    tectonics::{BoundaryKind, TectonicWorld, VolcanicCenterKind},
    Grid, PhysicalError, PhysicalErrorCode, SeedDomain,
};

pub const HAZARD_DERIVATION_VERSION: u16 = 2;
pub const MAX_HAZARD_FEATURES: usize = 512;
const MAX_DISTANCE_SOURCES: usize = 512;
const EARTHQUAKE_DISTANCE_SCALE: f64 = 0.18;
const VOLCANIC_DISTANCE_SCALE: f64 = 0.12;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HazardMetrics {
    pub derivation_version: u16,
    pub boundary_source_count: u32,
    pub volcanic_center_source_count: u32,
    pub earthquake_mean_ppm: u32,
    pub earthquake_max_ppm: u32,
    pub volcanic_mean_ppm: u32,
    pub volcanic_max_ppm: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HazardField {
    pub earthquake_hazard_ppm: Vec<u32>,
    pub volcanic_hazard_ppm: Vec<u32>,
    pub earthquake_rate_per_million_years_ppm: Vec<u32>,
    pub eruption_rate_per_million_years_ppm: Vec<u32>,
    pub metrics: HazardMetrics,
}

impl HazardField {
    pub fn validate(&self, grid: Grid) -> Result<(), PhysicalError> {
        let expected = grid.sample_count();
        if self.earthquake_hazard_ppm.len() != expected
            || self.volcanic_hazard_ppm.len() != expected
            || self.earthquake_rate_per_million_years_ppm.len() != expected
            || self.eruption_rate_per_million_years_ppm.len() != expected
            || self
                .earthquake_hazard_ppm
                .iter()
                .chain(&self.volcanic_hazard_ppm)
                .chain(&self.earthquake_rate_per_million_years_ppm)
                .chain(&self.eruption_rate_per_million_years_ppm)
                .any(|value| *value > 1_000_000)
            || self.metrics.derivation_version != HAZARD_DERIVATION_VERSION
        {
            return Err(PhysicalError::coded(
                PhysicalErrorCode::GeneratorInvalidSettings,
                "hazard field is outside its bounded ranges",
            ));
        }
        Ok(())
    }
}

fn earthquake_kind_weight(kind: BoundaryKind) -> u32 {
    match kind {
        BoundaryKind::Convergent => 1_000_000,
        BoundaryKind::Transform => 720_000,
        BoundaryKind::Divergent => 520_000,
    }
}

fn volcanic_boundary_weight(kind: BoundaryKind) -> u32 {
    match kind {
        BoundaryKind::Divergent => 620_000,
        BoundaryKind::Convergent => 280_000,
        BoundaryKind::Transform => 60_000,
    }
}

fn speed_factor(relative_speed_nanoradians_per_year: u64) -> u32 {
    (relative_speed_nanoradians_per_year
        .saturating_mul(1_000_000)
        .checked_div(100_000)
        .unwrap_or(1_000_000)
        .min(1_000_000)) as u32
}

fn boundary_strength(kind: BoundaryKind, relative_speed_nanoradians_per_year: u64) -> u32 {
    let speed = speed_factor(relative_speed_nanoradians_per_year);
    50_000
        + ((u64::from(earthquake_kind_weight(kind)) * (350_000u64 + u64::from(speed) / 2))
            / 1_000_000) as u32
}

fn volcanic_boundary_strength(kind: BoundaryKind, relative_speed_nanoradians_per_year: u64) -> u32 {
    let speed = speed_factor(relative_speed_nanoradians_per_year);
    ((u64::from(volcanic_boundary_weight(kind)) * (100_000u64 + u64::from(speed))) / 1_000_000)
        as u32
}

fn volcanic_center_strength(kind: VolcanicCenterKind, intensity_ppm: u32) -> u32 {
    let kind_weight: u32 = match kind {
        VolcanicCenterKind::Hotspot => 760_000,
        VolcanicCenterKind::SubductionArc => 1_000_000,
    };
    ((u64::from(intensity_ppm.min(1_000_000)) * u64::from(kind_weight)) / 1_000_000) as u32
}

fn distance_decay(grid: Grid, first: usize, second: usize, scale_fraction: f64) -> f64 {
    let first_point = grid.center_radians(grid.row_col(first).0, grid.row_col(first).1);
    let second_point = grid.center_radians(grid.row_col(second).0, grid.row_col(second).1);
    let distance = grid.great_circle_distance(first_point, second_point);
    (-distance / (grid.radius_metres as f64 * scale_fraction)).exp()
}

fn bounded_sources(mut sources: Vec<(usize, u32)>) -> Vec<(usize, u32)> {
    sources.sort_unstable_by_key(|(cell, strength)| (Reverse(*strength), *cell));
    sources.truncate(MAX_DISTANCE_SOURCES);
    sources
}

fn seeded_background(seed: u32, _retry_index: u32, cell: usize, salt: u64) -> u32 {
    // The accepted source identity already carries retry-scoped tectonic
    // structure. Keep the independent background field stable for a seed so a
    // retry changes causal structure without adding a second background shift.
    let base = derive_subsystem_seed(seed, 0, SeedDomain::Hazards);
    15_000 + (splitmix64(base ^ (cell as u64).wrapping_mul(salt)) % 35_001) as u32
}

fn mean_ppm(values: &[u32]) -> u32 {
    (values.iter().map(|value| u64::from(*value)).sum::<u64>() / values.len().max(1) as u64) as u32
}

pub fn derive_hazards(world: &TectonicWorld) -> Result<HazardField, PhysicalError> {
    world.validate()?;
    let cell_count = world.grid.sample_count();
    let mut earthquake_local = vec![0u32; cell_count];
    let mut volcanic_local = vec![0u32; cell_count];
    let mut boundary_sources = Vec::with_capacity(world.boundaries.len().saturating_mul(2));

    for boundary in &world.boundaries {
        let earthquake =
            boundary_strength(boundary.kind, boundary.relative_speed_nanoradians_per_year);
        let volcanic =
            volcanic_boundary_strength(boundary.kind, boundary.relative_speed_nanoradians_per_year);
        for cell in [boundary.first_cell, boundary.second_cell] {
            earthquake_local[cell] = earthquake_local[cell].max(earthquake);
            volcanic_local[cell] = volcanic_local[cell].max(volcanic);
            boundary_sources.push((cell, earthquake));
        }
    }
    let boundary_sources = bounded_sources(boundary_sources);
    let volcanic_sources = bounded_sources(
        world
            .boundaries
            .iter()
            .flat_map(|boundary| {
                [boundary.first_cell, boundary.second_cell]
                    .into_iter()
                    .map(move |cell| {
                        (
                            cell,
                            volcanic_boundary_strength(
                                boundary.kind,
                                boundary.relative_speed_nanoradians_per_year,
                            ),
                        )
                    })
            })
            .chain(world.volcanic_centers.iter().map(|center| {
                (
                    center.cell,
                    volcanic_center_strength(center.kind, center.intensity_ppm),
                )
            }))
            .collect(),
    );

    let mut earthquake_hazard_ppm = vec![0u32; cell_count];
    let mut volcanic_hazard_ppm = vec![0u32; cell_count];
    let mut earthquake_rate_per_million_years_ppm = vec![0u32; cell_count];
    let mut eruption_rate_per_million_years_ppm = vec![0u32; cell_count];

    for cell in 0..cell_count {
        let earthquake_decay = boundary_sources
            .iter()
            .map(|(source, strength)| {
                f64::from(*strength)
                    * distance_decay(world.grid, cell, *source, EARTHQUAKE_DISTANCE_SCALE)
            })
            .fold(0.0, f64::max);
        let volcanic_decay = volcanic_sources
            .iter()
            .map(|(source, strength)| {
                f64::from(*strength)
                    * distance_decay(world.grid, cell, *source, VOLCANIC_DISTANCE_SCALE)
            })
            .fold(0.0, f64::max);
        let earthquake = f64::from(seeded_background(
            world.seed,
            world.retry_index,
            cell,
            0x9e37_79b9,
        )) + f64::from(earthquake_local[cell])
            + earthquake_decay;
        let volcanic = f64::from(seeded_background(
            world.seed,
            world.retry_index,
            cell,
            0xbf58_476d,
        )) + f64::from(volcanic_local[cell])
            + volcanic_decay;
        earthquake_hazard_ppm[cell] = earthquake.round().clamp(0.0, 1_000_000.0) as u32;
        volcanic_hazard_ppm[cell] = volcanic.round().clamp(0.0, 1_000_000.0) as u32;
        earthquake_rate_per_million_years_ppm[cell] = earthquake_hazard_ppm[cell];
        eruption_rate_per_million_years_ppm[cell] = volcanic_hazard_ppm[cell];
    }

    let field = HazardField {
        metrics: HazardMetrics {
            derivation_version: HAZARD_DERIVATION_VERSION,
            boundary_source_count: boundary_sources.len() as u32,
            volcanic_center_source_count: world.volcanic_centers.len() as u32,
            earthquake_mean_ppm: mean_ppm(&earthquake_hazard_ppm),
            earthquake_max_ppm: earthquake_hazard_ppm.iter().copied().max().unwrap_or(0),
            volcanic_mean_ppm: mean_ppm(&volcanic_hazard_ppm),
            volcanic_max_ppm: volcanic_hazard_ppm.iter().copied().max().unwrap_or(0),
        },
        earthquake_hazard_ppm,
        volcanic_hazard_ppm,
        earthquake_rate_per_million_years_ppm,
        eruption_rate_per_million_years_ppm,
    };
    field.validate(world.grid)?;
    Ok(field)
}

fn ranked_cells(values: &[u32]) -> Vec<usize> {
    let mut cells = (0..values.len()).collect::<Vec<_>>();
    cells.sort_unstable_by_key(|cell| (Reverse(values[*cell]), *cell));
    cells.truncate(MAX_HAZARD_FEATURES);
    cells
}

fn center_microdegrees(grid: Grid, cell: usize) -> [i32; 2] {
    let (longitude, latitude) = grid.center_radians(grid.row_col(cell).0, grid.row_col(cell).1);
    [
        (longitude.to_degrees() * 1_000_000.0).round() as i32,
        (latitude.to_degrees() * 1_000_000.0).round() as i32,
    ]
}

fn hazard_feature(
    id: &str,
    layer: &str,
    name: &str,
    cell: usize,
    hazard_ppm: u32,
    rate_ppm: u32,
    point: [i32; 2],
) -> String {
    format!(
        "{{\"type\":\"Feature\",\"id\":\"{id}\",\"properties\":{{\"daenaLayerId\":\"{layer}\",\"kind\":\"hazard\",\"name\":\"{name}\",\"cell\":{cell},\"hazardPpm\":{hazard_ppm},\"ratePerMillionYearsPpm\":{rate_ppm},\"model\":\"relative-generated-v1\"}},\"geometry\":{{\"type\":\"Point\",\"coordinates\":[{},{}]}}}}",
        f64::from(point[0]) / 1_000_000.0,
        f64::from(point[1]) / 1_000_000.0
    )
}

pub fn to_geojson(world: &TectonicWorld) -> Result<String, PhysicalError> {
    let field = derive_hazards(world)?;
    let mut features = Vec::with_capacity(MAX_HAZARD_FEATURES * 2);
    for cell in ranked_cells(&field.earthquake_hazard_ppm) {
        features.push(hazard_feature(
            &format!("earthquake-hazard-{cell:05}"),
            "earthquake-hazard",
            "Generated earthquake hazard",
            cell,
            field.earthquake_hazard_ppm[cell],
            field.earthquake_rate_per_million_years_ppm[cell],
            center_microdegrees(world.grid, cell),
        ));
    }
    for cell in ranked_cells(&field.volcanic_hazard_ppm) {
        features.push(hazard_feature(
            &format!("volcanic-hazard-{cell:05}"),
            "volcanic-hazard",
            "Generated volcanic hazard",
            cell,
            field.volcanic_hazard_ppm[cell],
            field.eruption_rate_per_million_years_ppm[cell],
            center_microdegrees(world.grid, cell),
        ));
    }
    let mut output = String::from(r#"{"type":"FeatureCollection","features":["#);
    for (index, feature) in features.iter().enumerate() {
        if index > 0 {
            output.push(',');
        }
        output.push_str(feature);
    }
    output.push_str("]}");
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{generate_world, GenerationSettings, NoopProgress, DEFAULT_RADIUS_METRES};

    fn fixture() -> TectonicWorld {
        let mut progress = NoopProgress;
        generate_world(
            GenerationSettings {
                width: 32,
                height: 16,
                radius_metres: DEFAULT_RADIUS_METRES,
                target_land_fraction_ppm: 300_000,
            },
            831_429,
            0,
            &mut progress,
        )
        .unwrap()
        .tectonics
    }

    #[test]
    fn hazard_strengths_follow_boundary_and_center_ordering() {
        assert!(
            boundary_strength(BoundaryKind::Convergent, 80_000)
                > boundary_strength(BoundaryKind::Transform, 80_000)
        );
        assert!(
            boundary_strength(BoundaryKind::Transform, 80_000)
                > boundary_strength(BoundaryKind::Divergent, 80_000)
        );
        assert!(
            boundary_strength(BoundaryKind::Convergent, 80_000)
                > boundary_strength(BoundaryKind::Convergent, 20_000)
        );
        assert!(
            volcanic_center_strength(VolcanicCenterKind::SubductionArc, 800_000)
                > volcanic_center_strength(VolcanicCenterKind::Hotspot, 800_000)
        );
    }

    #[test]
    fn derived_hazards_follow_controlled_boundary_and_center_ordering() {
        let baseline = fixture();
        let mut strong_boundaries = baseline.clone();
        let mut weak_boundaries = baseline.clone();
        for boundary in &mut strong_boundaries.boundaries {
            boundary.kind = BoundaryKind::Convergent;
            boundary.relative_speed_nanoradians_per_year = 80_000;
        }
        for boundary in &mut weak_boundaries.boundaries {
            boundary.kind = BoundaryKind::Divergent;
            boundary.relative_speed_nanoradians_per_year = 20_000;
        }
        let strong_earthquakes = derive_hazards(&strong_boundaries).unwrap();
        let weak_earthquakes = derive_hazards(&weak_boundaries).unwrap();
        assert!(
            strong_earthquakes.metrics.earthquake_max_ppm
                > weak_earthquakes.metrics.earthquake_max_ppm
        );

        let mut subduction_centers = baseline.clone();
        let mut hotspot_centers = baseline;
        for boundary in subduction_centers
            .boundaries
            .iter_mut()
            .chain(hotspot_centers.boundaries.iter_mut())
        {
            boundary.kind = BoundaryKind::Transform;
            boundary.relative_speed_nanoradians_per_year = 20_000;
        }
        for center in &mut subduction_centers.volcanic_centers {
            center.kind = VolcanicCenterKind::SubductionArc;
            center.intensity_ppm = 1_000_000;
        }
        for center in &mut hotspot_centers.volcanic_centers {
            center.kind = VolcanicCenterKind::Hotspot;
            center.intensity_ppm = 1_000_000;
        }
        let subduction = derive_hazards(&subduction_centers).unwrap();
        let hotspots = derive_hazards(&hotspot_centers).unwrap();
        assert!(subduction.metrics.volcanic_max_ppm > hotspots.metrics.volcanic_max_ppm);
    }

    #[test]
    fn hazard_means_use_a_wide_accumulator() {
        assert_eq!(mean_ppm(&vec![1_000_000; 5_000]), 1_000_000);
    }

    #[test]
    fn background_rates_are_seed_stable_across_retries() {
        let first = seeded_background(831_429, 0, 7, 0x9e37_79b9);
        let second = seeded_background(831_429, 1, 7, 0x9e37_79b9);
        assert_eq!(first, second);
    }

    #[test]
    fn hazards_are_deterministic_bounded_and_seam_safe() {
        let world = fixture();
        let first = derive_hazards(&world).unwrap();
        let second = derive_hazards(&world).unwrap();
        assert_eq!(first, second);
        first.validate(world.grid).unwrap();
        assert!(first.metrics.earthquake_max_ppm >= first.metrics.earthquake_mean_ppm);
        assert!(first.metrics.volcanic_max_ppm >= first.metrics.volcanic_mean_ppm);
        let geojson = to_geojson(&world).unwrap();
        assert!(geojson.contains("earthquake-hazard"));
        assert!(geojson.contains("volcanic-hazard"));
        assert!(!geojson.contains("http://"));
        assert!(!geojson.contains("https://"));
        assert!(geojson.len() < 4 * 1024 * 1024);
        for feature in [geojson.as_str()] {
            assert!(feature.contains("relative-generated-v1"));
        }
    }

    #[test]
    fn deleting_and_rebuilding_hazard_layers_does_not_change_source() {
        let world = fixture();
        let source_before = crate::encode_source(&world).unwrap();
        let first = to_geojson(&world).unwrap();
        let mut disposable_layer = Some(first);
        assert!(disposable_layer.take().is_some());
        let rebuilt = to_geojson(&world).unwrap();
        assert_eq!(rebuilt, to_geojson(&world).unwrap());
        assert_eq!(crate::encode_source(&world).unwrap(), source_before);
    }
}
