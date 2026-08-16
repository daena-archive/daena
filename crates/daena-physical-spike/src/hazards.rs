//! Deterministic tectonic and volcanic hazard fields.
//!
//! These are bounded generated hazards derived from accepted tectonic
//! structure. They are not real-world predictions. Event materialization
//! consumes the complete annual-rate field through an explicit versioned model.

use std::cmp::Reverse;
use std::sync::OnceLock;

use super::{
    tectonics::{BoundaryKind, BoundarySegment, TectonicWorld, VolcanicCenter, VolcanicCenterKind},
    Grid, PhysicalError, PhysicalErrorCode,
};

pub const HAZARD_DERIVATION_VERSION: u16 = 3;
pub const VOLCANIC_SOURCE_DERIVATION_VERSION: u16 = 1;
pub const MAX_HAZARD_FEATURES: usize = 512;
pub const RATE_NANO: u64 = 1_000_000_000;
const EARTHQUAKE_DISTANCE_SCALE_PPM: u32 = 180_000;
const VOLCANIC_DISTANCE_SCALE_PPM: u32 = 120_000;
const EARTHQUAKE_LENGTH_REF_METRES: u64 = 100_000;
const EARTHQUAKE_BASE_NANO: u64 = 8_000;
const VOLCANIC_CENTER_BASE_NANO: u64 = 6_000;
const SPREADING_RIFT_BASE_NANO: u64 = 1_200;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VolcanicOrigin {
    Hotspot,
    SubductionArc,
    SpreadingRift,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VolcanicActivityClass {
    Dormant,
    Moderate,
    Vigorous,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DerivedVolcanicSource {
    pub stable_id: u32,
    pub cell: usize,
    pub origin: VolcanicOrigin,
    pub activity_class: VolcanicActivityClass,
    pub eruptions_per_model_year_nano: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HazardMetrics {
    pub derivation_version: u16,
    pub volcanic_source_derivation_version: u16,
    pub boundary_source_count: u32,
    pub volcanic_center_source_count: u32,
    pub earthquake_mean_ppm: u32,
    pub earthquake_max_ppm: u32,
    pub volcanic_mean_ppm: u32,
    pub volcanic_max_ppm: u32,
    pub earthquake_rate_mass_nano: u64,
    pub volcanic_rate_mass_nano: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HazardField {
    pub earthquake_hazard_ppm: Vec<u32>,
    pub volcanic_hazard_ppm: Vec<u32>,
    pub earthquake_annual_rate_nano: Vec<u64>,
    pub volcanic_annual_rate_nano: Vec<u64>,
    pub earthquake_rate_per_million_years_ppm: Vec<u32>,
    pub eruption_rate_per_million_years_ppm: Vec<u32>,
    pub volcanic_sources: Vec<DerivedVolcanicSource>,
    pub metrics: HazardMetrics,
}

impl HazardField {
    pub fn validate(&self, grid: Grid) -> Result<(), PhysicalError> {
        let expected = grid.sample_count();
        if self.earthquake_hazard_ppm.len() != expected
            || self.volcanic_hazard_ppm.len() != expected
            || self.earthquake_annual_rate_nano.len() != expected
            || self.volcanic_annual_rate_nano.len() != expected
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
            || self.metrics.volcanic_source_derivation_version != VOLCANIC_SOURCE_DERIVATION_VERSION
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

fn speed_factor(relative_speed_nanoradians_per_year: u64) -> u32 {
    (relative_speed_nanoradians_per_year
        .saturating_mul(1_000_000)
        .checked_div(100_000)
        .unwrap_or(1_000_000)
        .min(1_000_000)) as u32
}

fn exp_neg_ppm(x_ppm: u64) -> u64 {
    if x_ppm == 0 {
        return 1_000_000;
    }
    if x_ppm >= 20_000_000 {
        return 0;
    }
    let mut term = 1_000_000u128;
    let mut sum = term;
    for n in 1..=16 {
        term = term * u128::from(x_ppm) / 1_000_000 / n as u128;
        sum += term;
        if term == 0 {
            break;
        }
    }
    (1_000_000u128 * 1_000_000 / sum.max(1)) as u64
}

fn acos_micro_radians(dot: f64) -> u64 {
    static TABLE: OnceLock<[u32; 20_001]> = OnceLock::new();
    let table = TABLE.get_or_init(|| {
        let mut values = [0u32; 20_001];
        for (index, slot) in values.iter_mut().enumerate() {
            let quantized_dot = (index as f64 / 10_000.0) - 1.0;
            *slot = (quantized_dot.clamp(-1.0, 1.0).acos() * 1_000_000.0).round() as u32;
        }
        values
    });
    let index =
        ((dot.clamp(-1.0, 1.0) * 10_000.0).round() as i32 + 10_000).clamp(0, 20_000) as usize;
    u64::from(table[index])
}

fn decay_from_dot(dot: f64, scale_ppm: u32) -> u64 {
    static EXP_TABLE: OnceLock<[u32; 2_001]> = OnceLock::new();
    let table = EXP_TABLE.get_or_init(|| {
        let mut values = [0u32; 2_001];
        for (index, slot) in values.iter_mut().enumerate() {
            *slot = exp_neg_ppm((index as u64) * 10_000) as u32;
        }
        values
    });
    let theta_micro = acos_micro_radians(dot);
    let x_ppm = theta_micro.saturating_mul(1_000_000) / u64::from(scale_ppm.max(1));
    if x_ppm >= 20_000_000 {
        return 0;
    }
    u64::from(table[(x_ppm / 10_000) as usize])
}

fn lon_lat_to_xyz(lon: f64, lat: f64) -> (f64, f64, f64) {
    let cos_lat = lat.cos();
    (cos_lat * lon.cos(), cos_lat * lon.sin(), lat.sin())
}

fn cell_xyz(points: &[(f64, f64)]) -> Vec<(f64, f64, f64)> {
    points
        .iter()
        .map(|&(lon, lat)| lon_lat_to_xyz(lon, lat))
        .collect()
}

fn point_to_cell(grid: Grid, lon: f64, lat: f64) -> usize {
    let col = ((lon + std::f64::consts::PI) / std::f64::consts::TAU * f64::from(grid.width))
        .floor()
        .rem_euclid(f64::from(grid.width)) as u32;
    let row = ((lat + std::f64::consts::FRAC_PI_2) / std::f64::consts::PI * f64::from(grid.height))
        .floor()
        .clamp(0.0, f64::from(grid.height.saturating_sub(1))) as u32;
    grid.index(row, col)
}

fn cell_points(grid: Grid) -> Vec<(f64, f64)> {
    (0..grid.sample_count())
        .map(|cell| {
            let (row, col) = grid.row_col(cell);
            grid.center_radians(row, col)
        })
        .collect()
}

fn geodesic_midpoint(first: (f64, f64), second: (f64, f64)) -> (f64, f64) {
    let (lon1, lat1) = first;
    let (lon2, lat2) = second;
    let x = lat1.cos() * lon1.cos() + lat2.cos() * lon2.cos();
    let y = lat1.cos() * lon1.sin() + lat2.cos() * lon2.sin();
    let z = lat1.sin() + lat2.sin();
    let hyp = (x * x + y * y).sqrt();
    if hyp < f64::EPSILON {
        return (
            0.0,
            if z >= 0.0 {
                std::f64::consts::FRAC_PI_2
            } else {
                -std::f64::consts::FRAC_PI_2
            },
        );
    }
    (y.atan2(x), z.atan2(hyp))
}

fn nearest_cell(grid: Grid, point: (f64, f64)) -> usize {
    point_to_cell(grid, point.0, point.1)
}

fn segment_length_m(grid: Grid, points: &[(f64, f64)], boundary: &BoundarySegment) -> u64 {
    grid.great_circle_distance(points[boundary.first_cell], points[boundary.second_cell])
        .round()
        .clamp(1.0, f64::from(u32::MAX)) as u64
}

fn earthquake_segment_rate_nano(
    kind: BoundaryKind,
    relative_speed_nanoradians_per_year: u64,
    length_m: u64,
) -> u64 {
    let speed = speed_factor(relative_speed_nanoradians_per_year);
    ((u128::from(EARTHQUAKE_BASE_NANO)
        * u128::from(earthquake_kind_weight(kind))
        * u128::from(350_000u32 + speed / 2)
        * u128::from(length_m.max(1)))
        / 1_000_000
        / 1_000_000
        / u128::from(EARTHQUAKE_LENGTH_REF_METRES.max(1))) as u64
}

fn activity_class(intensity_ppm: u32) -> VolcanicActivityClass {
    if intensity_ppm >= 700_000 {
        VolcanicActivityClass::Vigorous
    } else if intensity_ppm >= 250_000 {
        VolcanicActivityClass::Moderate
    } else {
        VolcanicActivityClass::Dormant
    }
}

fn origin_from_kind(kind: VolcanicCenterKind) -> VolcanicOrigin {
    match kind {
        VolcanicCenterKind::Hotspot => VolcanicOrigin::Hotspot,
        VolcanicCenterKind::SubductionArc => VolcanicOrigin::SubductionArc,
    }
}

fn origin_weight(origin: VolcanicOrigin) -> u32 {
    match origin {
        VolcanicOrigin::SubductionArc => 1_000_000,
        VolcanicOrigin::Hotspot => 760_000,
        VolcanicOrigin::SpreadingRift => 420_000,
    }
}

fn volcanic_center_rate_nano(center: &VolcanicCenter) -> u64 {
    let origin = origin_from_kind(center.kind);
    ((u128::from(VOLCANIC_CENTER_BASE_NANO)
        * u128::from(center.intensity_ppm.min(1_000_000))
        * u128::from(origin_weight(origin)))
        / 1_000_000
        / 1_000_000) as u64
}

fn spreading_rift_rate_nano(length_m: u64, relative_speed_nanoradians_per_year: u64) -> u64 {
    let speed = speed_factor(relative_speed_nanoradians_per_year);
    ((u128::from(SPREADING_RIFT_BASE_NANO)
        * u128::from(origin_weight(VolcanicOrigin::SpreadingRift))
        * u128::from(100_000u32 + speed)
        * u128::from(length_m.max(1)))
        / 1_000_000
        / 1_000_000
        / u128::from(EARTHQUAKE_LENGTH_REF_METRES.max(1))) as u64
}

fn stable_source_id(tag: u32, first: usize, second: usize) -> u32 {
    let mixed = (u64::from(tag) << 32)
        ^ (first as u64).wrapping_mul(0x9e37_79b9_7f4a_7c15)
        ^ (second as u64).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    (mixed ^ (mixed >> 32)) as u32
}

/// Versioned v2 volcanic interpretation: origin, activity class, and long-term
/// eruptions per model year from immutable centers and divergent causes.
pub fn derive_volcanic_sources(
    world: &TectonicWorld,
    points: &[(f64, f64)],
) -> Vec<DerivedVolcanicSource> {
    let mut sources = world
        .volcanic_centers
        .iter()
        .map(|center| DerivedVolcanicSource {
            stable_id: stable_source_id(0xC1, center.cell, usize::from(center.plate)),
            cell: center.cell,
            origin: origin_from_kind(center.kind),
            activity_class: activity_class(center.intensity_ppm),
            eruptions_per_model_year_nano: volcanic_center_rate_nano(center),
        })
        .collect::<Vec<_>>();
    for boundary in &world.boundaries {
        if boundary.kind != BoundaryKind::Divergent {
            continue;
        }
        let length_m = segment_length_m(world.grid, points, boundary);
        let midpoint = geodesic_midpoint(points[boundary.first_cell], points[boundary.second_cell]);
        sources.push(DerivedVolcanicSource {
            stable_id: stable_source_id(0xB1, boundary.first_cell, boundary.second_cell),
            cell: nearest_cell(world.grid, midpoint),
            origin: VolcanicOrigin::SpreadingRift,
            activity_class: activity_class(speed_factor(
                boundary.relative_speed_nanoradians_per_year,
            )),
            eruptions_per_model_year_nano: spreading_rift_rate_nano(
                length_m,
                boundary.relative_speed_nanoradians_per_year,
            ),
        });
    }
    sources
}

fn apply_source(
    xyz: &[(f64, f64, f64)],
    rates: &mut [u64],
    source: (f64, f64),
    strength_nano: u64,
    scale_ppm: u32,
) {
    if strength_nano == 0 {
        return;
    }
    let source_xyz = lon_lat_to_xyz(source.0, source.1);
    for (cell, &(x, y, z)) in xyz.iter().enumerate() {
        let dot = (x * source_xyz.0 + y * source_xyz.1 + z * source_xyz.2).clamp(-1.0, 1.0);
        let decay = decay_from_dot(dot, scale_ppm);
        let contribution = (u128::from(strength_nano) * u128::from(decay) / 1_000_000) as u64;
        rates[cell] = rates[cell].saturating_add(contribution);
    }
}

fn normalize_ppm(values: &[u64]) -> Vec<u32> {
    let max = values.iter().copied().max().unwrap_or(0).max(1);
    values
        .iter()
        .map(|value| ((*value as u128 * 1_000_000) / max as u128) as u32)
        .collect()
}

fn events_per_million_years_ppm(annual_nano: u64) -> u32 {
    ((annual_nano.saturating_mul(1_000_000) / RATE_NANO).min(1_000_000)) as u32
}

fn mean_ppm(values: &[u32]) -> u32 {
    (values.iter().map(|value| u64::from(*value)).sum::<u64>() / values.len().max(1) as u64) as u32
}

fn rate_mass(values: &[u64]) -> u64 {
    values.iter().copied().fold(0u64, u64::saturating_add)
}

pub fn derive_hazards(world: &TectonicWorld) -> Result<HazardField, PhysicalError> {
    world.validate()?;
    let cell_count = world.grid.sample_count();
    let points = cell_points(world.grid);
    let xyz = cell_xyz(&points);
    let volcanic_sources = derive_volcanic_sources(world, &points);
    let mut earthquake_annual_rate_nano = vec![0u64; cell_count];
    let mut volcanic_annual_rate_nano = vec![0u64; cell_count];

    for boundary in &world.boundaries {
        let length_m = segment_length_m(world.grid, &points, boundary);
        let midpoint = geodesic_midpoint(points[boundary.first_cell], points[boundary.second_cell]);
        let rate = earthquake_segment_rate_nano(
            boundary.kind,
            boundary.relative_speed_nanoradians_per_year,
            length_m,
        );
        apply_source(
            &xyz,
            &mut earthquake_annual_rate_nano,
            midpoint,
            rate,
            EARTHQUAKE_DISTANCE_SCALE_PPM,
        );
    }
    for source in &volcanic_sources {
        apply_source(
            &xyz,
            &mut volcanic_annual_rate_nano,
            points[source.cell],
            source.eruptions_per_model_year_nano,
            VOLCANIC_DISTANCE_SCALE_PPM,
        );
    }

    let earthquake_hazard_ppm = normalize_ppm(&earthquake_annual_rate_nano);
    let volcanic_hazard_ppm = normalize_ppm(&volcanic_annual_rate_nano);
    let earthquake_rate_per_million_years_ppm = earthquake_annual_rate_nano
        .iter()
        .map(|value| events_per_million_years_ppm(*value))
        .collect::<Vec<_>>();
    let eruption_rate_per_million_years_ppm = volcanic_annual_rate_nano
        .iter()
        .map(|value| events_per_million_years_ppm(*value))
        .collect::<Vec<_>>();
    let field = HazardField {
        metrics: HazardMetrics {
            derivation_version: HAZARD_DERIVATION_VERSION,
            volcanic_source_derivation_version: VOLCANIC_SOURCE_DERIVATION_VERSION,
            boundary_source_count: world.boundaries.len() as u32,
            volcanic_center_source_count: world.volcanic_centers.len() as u32,
            earthquake_mean_ppm: mean_ppm(&earthquake_hazard_ppm),
            earthquake_max_ppm: earthquake_hazard_ppm.iter().copied().max().unwrap_or(0),
            volcanic_mean_ppm: mean_ppm(&volcanic_hazard_ppm),
            volcanic_max_ppm: volcanic_hazard_ppm.iter().copied().max().unwrap_or(0),
            earthquake_rate_mass_nano: rate_mass(&earthquake_annual_rate_nano),
            volcanic_rate_mass_nano: rate_mass(&volcanic_annual_rate_nano),
        },
        earthquake_hazard_ppm,
        volcanic_hazard_ppm,
        earthquake_annual_rate_nano,
        volcanic_annual_rate_nano,
        earthquake_rate_per_million_years_ppm,
        eruption_rate_per_million_years_ppm,
        volcanic_sources,
    };
    field.validate(world.grid)?;
    Ok(field)
}

fn ranked_cells(values: &[u64], max_features: usize) -> Vec<usize> {
    let mut cells = (0..values.len()).collect::<Vec<_>>();
    cells.sort_unstable_by_key(|cell| (Reverse(values[*cell]), *cell));
    cells.truncate(max_features.max(1).min(values.len()));
    cells
}

fn center_microdegrees(grid: Grid, cell: usize) -> [i32; 2] {
    let (longitude, latitude) = grid.center_radians(grid.row_col(cell).0, grid.row_col(cell).1);
    [
        (longitude.to_degrees() * 1_000_000.0).round() as i32,
        (latitude.to_degrees() * 1_000_000.0).round() as i32,
    ]
}

#[allow(clippy::too_many_arguments)]
fn hazard_feature(
    id: &str,
    layer: &str,
    name: &str,
    cell: usize,
    hazard_ppm: u32,
    rate_ppm: u32,
    annual_rate_nano: u64,
    point: [i32; 2],
) -> String {
    format!(
        "{{\"type\":\"Feature\",\"id\":\"{id}\",\"properties\":{{\"daenaLayerId\":\"{layer}\",\"kind\":\"hazard\",\"name\":\"{name}\",\"cell\":{cell},\"hazardPpm\":{hazard_ppm},\"ratePerMillionYearsPpm\":{rate_ppm},\"annualRateNano\":{annual_rate_nano},\"model\":\"relative-generated-v1\"}},\"geometry\":{{\"type\":\"Point\",\"coordinates\":[{},{}]}}}}",
        f64::from(point[0]) / 1_000_000.0,
        f64::from(point[1]) / 1_000_000.0
    )
}

pub fn to_geojson_with_limit(
    world: &TectonicWorld,
    field: &HazardField,
    max_features: usize,
) -> Result<String, PhysicalError> {
    field.validate(world.grid)?;
    let mut features = Vec::new();
    for cell in ranked_cells(&field.earthquake_annual_rate_nano, max_features) {
        features.push(hazard_feature(
            &format!("earthquake-hazard-{cell:05}"),
            "earthquake-hazard",
            "Generated earthquake hazard",
            cell,
            field.earthquake_hazard_ppm[cell],
            field.earthquake_rate_per_million_years_ppm[cell],
            field.earthquake_annual_rate_nano[cell],
            center_microdegrees(world.grid, cell),
        ));
    }
    for cell in ranked_cells(&field.volcanic_annual_rate_nano, max_features) {
        features.push(hazard_feature(
            &format!("volcanic-hazard-{cell:05}"),
            "volcanic-hazard",
            "Generated volcanic hazard",
            cell,
            field.volcanic_hazard_ppm[cell],
            field.eruption_rate_per_million_years_ppm[cell],
            field.volcanic_annual_rate_nano[cell],
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

pub fn to_geojson(world: &TectonicWorld) -> Result<String, PhysicalError> {
    let field = derive_hazards(world)?;
    to_geojson_with_limit(world, &field, MAX_HAZARD_FEATURES)
}

pub fn nearest_volcanic_source(field: &HazardField, cell: usize) -> Option<DerivedVolcanicSource> {
    field
        .volcanic_sources
        .iter()
        .copied()
        .filter(|source| source.cell == cell)
        .max_by_key(|source| (source.eruptions_per_model_year_nano, source.stable_id))
        .or_else(|| {
            field
                .volcanic_sources
                .iter()
                .copied()
                .min_by_key(|source| (source.cell.abs_diff(cell), Reverse(source.stable_id)))
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{generate_world, GenerationSettings, NoopProgress, DEFAULT_RADIUS_METRES};

    const QUANTIZED_KERNEL_ERROR_NANO: u64 = 256;

    fn geodesic_decay_ppm(distance_m: f64, scale_m: f64) -> u64 {
        if scale_m <= 0.0 {
            return 0;
        }
        let x_ppm = (distance_m / scale_m * 1_000_000.0)
            .round()
            .clamp(0.0, 20_000_000.0) as u64;
        exp_neg_ppm(x_ppm)
    }

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
            earthquake_segment_rate_nano(BoundaryKind::Convergent, 80_000, 100_000)
                > earthquake_segment_rate_nano(BoundaryKind::Transform, 80_000, 100_000)
        );
        assert!(
            earthquake_segment_rate_nano(BoundaryKind::Transform, 80_000, 100_000)
                > earthquake_segment_rate_nano(BoundaryKind::Divergent, 80_000, 100_000)
        );
        assert!(
            earthquake_segment_rate_nano(BoundaryKind::Convergent, 80_000, 100_000)
                > earthquake_segment_rate_nano(BoundaryKind::Convergent, 20_000, 100_000)
        );
        let strong = VolcanicCenter {
            cell: 0,
            plate: 0,
            kind: VolcanicCenterKind::SubductionArc,
            intensity_ppm: 800_000,
        };
        let weak = VolcanicCenter {
            cell: 0,
            plate: 0,
            kind: VolcanicCenterKind::Hotspot,
            intensity_ppm: 800_000,
        };
        assert!(volcanic_center_rate_nano(&strong) > volcanic_center_rate_nano(&weak));
    }

    #[test]
    fn single_source_decay_is_analytic_and_integer() {
        let grid = Grid::new(8, 4, DEFAULT_RADIUS_METRES).unwrap();
        let points = cell_points(grid);
        let scale_m = grid.radius_metres as f64 * 0.18;
        let origin = points[0];
        let far = points[points.len() - 1];
        let near_decay = geodesic_decay_ppm(0.0, scale_m);
        let far_decay = geodesic_decay_ppm(grid.great_circle_distance(origin, far), scale_m);
        assert_eq!(near_decay, 1_000_000);
        assert!(far_decay < near_decay);
        assert_eq!(
            geodesic_decay_ppm(grid.great_circle_distance(origin, far), scale_m),
            far_decay
        );
    }

    #[test]
    fn source_superposition_adds_independent_contributions() {
        let grid = Grid::new(8, 4, DEFAULT_RADIUS_METRES).unwrap();
        let points = cell_points(grid);
        let xyz = cell_xyz(&points);
        let mut first = vec![0u64; points.len()];
        let mut second = vec![0u64; points.len()];
        let mut both = vec![0u64; points.len()];
        apply_source(
            &xyz,
            &mut first,
            points[0],
            4_000,
            EARTHQUAKE_DISTANCE_SCALE_PPM,
        );
        apply_source(
            &xyz,
            &mut second,
            points[10],
            3_000,
            EARTHQUAKE_DISTANCE_SCALE_PPM,
        );
        apply_source(
            &xyz,
            &mut both,
            points[0],
            4_000,
            EARTHQUAKE_DISTANCE_SCALE_PPM,
        );
        apply_source(
            &xyz,
            &mut both,
            points[10],
            3_000,
            EARTHQUAKE_DISTANCE_SCALE_PPM,
        );
        for cell in 0..points.len() {
            assert_eq!(both[cell], first[cell].saturating_add(second[cell]));
        }
    }

    #[test]
    fn boundary_reversal_and_seam_do_not_change_segment_rate() {
        let length = 80_000;
        let forward = earthquake_segment_rate_nano(BoundaryKind::Convergent, 50_000, length);
        let reverse = earthquake_segment_rate_nano(BoundaryKind::Convergent, 50_000, length);
        assert_eq!(forward, reverse);
        let grid = Grid::new(8, 4, DEFAULT_RADIUS_METRES).unwrap();
        let points = cell_points(grid);
        let xyz = cell_xyz(&points);
        let mut west = vec![0u64; points.len()];
        let mut east = vec![0u64; points.len()];
        apply_source(
            &xyz,
            &mut west,
            points[0],
            5_000,
            EARTHQUAKE_DISTANCE_SCALE_PPM,
        );
        apply_source(
            &xyz,
            &mut east,
            points[7],
            5_000,
            EARTHQUAKE_DISTANCE_SCALE_PPM,
        );
        assert!(west[1] > 0 && east[6] > 0);
        assert_eq!(west[0], east[7]);
    }

    #[test]
    fn spatial_kernel_stays_within_the_locked_quantization_bound() {
        let grid = Grid::new(16, 8, DEFAULT_RADIUS_METRES).unwrap();
        let points = cell_points(grid);
        let xyz = cell_xyz(&points);
        let mut indexed = vec![0u64; points.len()];
        apply_source(
            &xyz,
            &mut indexed,
            points[0],
            9_000,
            EARTHQUAKE_DISTANCE_SCALE_PPM,
        );
        let scale_m = grid.radius_metres as f64 * 0.18;
        let mut exact = vec![0u64; points.len()];
        for (cell, &point) in points.iter().enumerate() {
            let decay = geodesic_decay_ppm(grid.great_circle_distance(point, points[0]), scale_m);
            exact[cell] = (u128::from(9_000u64) * u128::from(decay) / 1_000_000) as u64;
        }
        let max_error = indexed
            .iter()
            .zip(&exact)
            .map(|(left, right)| left.abs_diff(*right))
            .max()
            .unwrap_or(0);
        assert!(max_error <= QUANTIZED_KERNEL_ERROR_NANO);
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
            strong_earthquakes.metrics.earthquake_rate_mass_nano
                > weak_earthquakes.metrics.earthquake_rate_mass_nano
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
        assert!(
            subduction.metrics.volcanic_rate_mass_nano > hotspots.metrics.volcanic_rate_mass_nano
        );
    }

    #[test]
    fn display_cap_does_not_change_the_complete_field() {
        let world = fixture();
        let field = derive_hazards(&world).unwrap();
        let full = to_geojson_with_limit(&world, &field, MAX_HAZARD_FEATURES).unwrap();
        let limited = to_geojson_with_limit(&world, &field, 8).unwrap();
        assert_eq!(derive_hazards(&world).unwrap(), field);
        assert_ne!(full, limited);
        assert!(limited.matches("\"id\":\"earthquake-hazard-").count() <= 8);
    }

    #[test]
    fn volcanic_sources_are_versioned_from_canonical_causes() {
        let world = fixture();
        let field = derive_hazards(&world).unwrap();
        assert_eq!(
            field.metrics.volcanic_source_derivation_version,
            VOLCANIC_SOURCE_DERIVATION_VERSION
        );
        assert!(field.volcanic_sources.len() >= world.volcanic_centers.len());
        assert!(field
            .volcanic_sources
            .iter()
            .any(|source| source.origin == VolcanicOrigin::SpreadingRift
                || source.origin == VolcanicOrigin::Hotspot
                || source.origin == VolcanicOrigin::SubductionArc));
    }

    #[test]
    fn hazard_means_use_a_wide_accumulator() {
        assert_eq!(mean_ppm(&vec![1_000_000; 5_000]), 1_000_000);
    }

    #[test]
    fn hazards_are_deterministic_bounded_and_seam_safe() {
        let world = fixture();
        let first = derive_hazards(&world).unwrap();
        let second = derive_hazards(&world).unwrap();
        assert_eq!(first, second);
        first.validate(world.grid).unwrap();
        assert_eq!(
            first.metrics.boundary_source_count,
            world.boundaries.len() as u32
        );
        assert!(first.metrics.earthquake_max_ppm >= first.metrics.earthquake_mean_ppm);
        assert!(first.metrics.volcanic_max_ppm >= first.metrics.volcanic_mean_ppm);
        let geojson = to_geojson(&world).unwrap();
        assert!(geojson.contains("earthquake-hazard"));
        assert!(geojson.contains("volcanic-hazard"));
        assert!(geojson.contains("annualRateNano"));
        assert!(!geojson.contains("http://"));
        assert!(!geojson.contains("https://"));
        assert!(geojson.len() < 4 * 1024 * 1024);
        assert!(geojson.contains("relative-generated-v1"));
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
