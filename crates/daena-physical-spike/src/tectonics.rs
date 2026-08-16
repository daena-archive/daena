//! Deterministic tectonic cause chain for the native physical generator.
//!
//! This module owns the active v2 source codec. The source stores the signed
//! elevation field together with the physical causes required to derive later
//! terrain, climate, hydrology, and hazard products.

use super::{
    derive_subsystem_seed, resolution, splitmix64, Grid, PhysicalError, ProgressPhase,
    ProgressSink, SeedDomain,
};
use std::cmp::Reverse;
use std::collections::{BinaryHeap, HashSet};
use std::fmt::Write as _;

const MICRODEGREES_PER_DEGREE: f64 = 1_000_000.0;
const MIN_PLATES: u16 = 4;
const MAX_PLATES: u16 = 64;
const MAX_VOLCANIC_CENTERS: usize = 256;
const MAX_BOUNDARIES: usize = 2_000_000;
pub const TECTONIC_SOURCE_VERSION: u16 = 2;
pub const TECTONIC_SOURCE_HEADER_BYTES: usize = 68;
pub const TECTONIC_MAX_SOURCE_BYTES: usize = crate::MAX_SOURCE_BYTES;
/// Versioned v2 physical classification parameter. Keep this value stable
/// unless the generator version changes; it is intentionally not serialized
/// because the v2 header and payload are locked.
pub const RELATIVE_MOTION_THRESHOLD_NANORADIANS_PER_YEAR: f64 = 25_000.0;
const CRATON_WARP_AMPLITUDE: f64 = 0.42;
const CRATON_ANISOTROPY: f64 = 0.34;
const RIFT_SHOULDER_FRACTION: f64 = 0.22;
const RIDGE_RELIEF_FRACTION: f64 = 0.55;
const BOUNDARY_COST_PERTURBATION_PPM: i64 = 480_000;
const BOUNDARY_WAVE_COST_PPM: i64 = 280_000;
const FAULT_FOLLOW_PPM: i64 = 420_000;
const PLATE_RESHAPE_FRACTION_PPM: u32 = 280_000;
const MIN_SHATTER_CELLS: usize = 12;
const TRANSFORM_COUPLE_PAIR_DIVISOR: usize = 6;
const TRANSFORM_SLIP_MIN_HOPS: usize = 2;
const TRANSFORM_SLIP_EXTRA_HOPS: u64 = 3;
const CONTINENTAL_AREA_TARGET_PPM: u32 = 450_000;
const TERRANE_AREA_BUDGET_PPM: u32 = 40_000;
const LITHOLOGY_COST_PPM: i64 = 180_000;
const LAYOUT_MEGA_WEIGHT: u64 = 24;
const LAYOUT_GIANT_SCRAPS_WEIGHT: u64 = 18;
const LAYOUT_DUAL_WEIGHT: u64 = 28;
const LAYOUT_SCATTERED_WEIGHT: u64 = 30;
const LAYOUT_WEIGHT_TOTAL: u64 =
    LAYOUT_MEGA_WEIGHT + LAYOUT_GIANT_SCRAPS_WEIGHT + LAYOUT_DUAL_WEIGHT + LAYOUT_SCATTERED_WEIGHT;
const MOTION_SPEED_REF_NANORADIANS: f64 = 600_000.0;
const JUNCTION_NEIGHBOR_PLATES: usize = 3;
const HOTSPOT_CHAIN_MIN: usize = 3;
const HOTSPOT_CHAIN_MAX: usize = 7;
const HOTSPOT_STEP_RADIANS: f64 = 0.11;
const TRANSFORM_TIE_RATIO_PPM: i64 = 1_000_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TectonicSettings {
    pub plate_count: u16,
    pub continental_plate_count: u16,
    pub tectonic_activity_ppm: u32,
    pub island_activity_ppm: u32,
}

impl TectonicSettings {
    pub fn default_for(grid: Grid) -> Self {
        let plate_count = (grid.sample_count() / 128).clamp(MIN_PLATES as usize, 16) as u16;
        Self {
            plate_count,
            continental_plate_count: (u32::from(plate_count) * 45 / 100).max(2) as u16,
            tectonic_activity_ppm: 600_000,
            island_activity_ppm: 300_000,
        }
    }

    pub fn validate(self) -> Result<(), PhysicalError> {
        if !(MIN_PLATES..=MAX_PLATES).contains(&self.plate_count) {
            return Err(PhysicalError::InvalidSettings(
                "tectonic plate count is outside the bounded range".into(),
            ));
        }
        if self.continental_plate_count == 0 || self.continental_plate_count > self.plate_count {
            return Err(PhysicalError::InvalidSettings(
                "continental plate count must be within the plate count".into(),
            ));
        }
        if self.tectonic_activity_ppm > 1_000_000 || self.island_activity_ppm > 1_000_000 {
            return Err(PhysicalError::InvalidSettings(
                "tectonic activity values must be parts per million".into(),
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CrustType {
    Continental,
    Oceanic,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BoundaryKind {
    Convergent,
    Divergent,
    Transform,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VolcanicCenterKind {
    Hotspot,
    SubductionArc,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Plate {
    pub id: u16,
    pub seed_cell: usize,
    pub site_longitude_microdegrees: i32,
    pub site_latitude_microdegrees: i32,
    pub rotation_axis_longitude_microdegrees: i32,
    pub rotation_axis_latitude_microdegrees: i32,
    pub angular_speed_nanoradians_per_year: i64,
    pub crust_type: CrustType,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BoundarySegment {
    pub first_cell: usize,
    pub second_cell: usize,
    pub first_plate: u16,
    pub second_plate: u16,
    pub kind: BoundaryKind,
    pub relative_speed_nanoradians_per_year: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VolcanicCenter {
    pub cell: usize,
    pub plate: u16,
    pub kind: VolcanicCenterKind,
    pub intensity_ppm: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TectonicMetrics {
    pub plate_area_p05_ppm: u32,
    pub plate_area_median_ppm: u32,
    pub plate_area_p95_ppm: u32,
    pub boundary_coverage_ppm: u32,
    pub convergent_boundary_count: u32,
    pub divergent_boundary_count: u32,
    pub transform_boundary_count: u32,
    pub continental_crust_area_ppm: u32,
    pub exposed_land_area_ppm: u32,
    pub continental_exposed_land_area_ppm: u32,
    pub continental_shelf_area_ppm: u32,
    pub elevation_p05_mm: i32,
    pub elevation_median_mm: i32,
    pub elevation_p95_mm: i32,
    pub trench_ridge_separation_metres: u64,
    pub volcanic_arc_offset_metres: u64,
    pub island_component_count: u32,
    pub largest_island_component_cells: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TectonicWorld {
    pub grid: Grid,
    pub seed: u32,
    pub retry_index: u32,
    pub target_land_fraction_ppm: u32,
    pub sea_level_mm: i32,
    pub settings: TectonicSettings,
    pub plates: Vec<Plate>,
    pub plate_by_cell: Vec<u16>,
    pub crust_by_cell: Vec<CrustType>,
    pub boundaries: Vec<BoundarySegment>,
    pub volcanic_centers: Vec<VolcanicCenter>,
    pub elevations_mm: Vec<i32>,
}

impl TectonicWorld {
    pub fn physical_field(&self) -> super::PhysicalField {
        super::PhysicalField {
            grid: self.grid,
            seed: self.seed,
            retry_index: self.retry_index,
            target_land_fraction_ppm: self.target_land_fraction_ppm,
            sea_level_mm: self.sea_level_mm,
            elevations_mm: self.elevations_mm.clone(),
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct Vec3 {
    x: f64,
    y: f64,
    z: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum ContinentLayoutKind {
    Mega,
    GiantScraps,
    Dual,
    Scattered,
}

#[derive(Debug, Clone)]
struct ContinentLayout {
    group_count: usize,
    companion_threshold: u64,
    base_radius: f64,
    companion_offset_min: f64,
    companion_offset_span: f64,
    plate_crossing_ppm: i64,
    same_group_attraction_ppm: i64,
    other_group_repulsion_ppm: i64,
    occupied_ppm: i64,
    junction_attraction_ppm: i64,
    crust_target_ppm: u32,
    terrane_budget_ppm: u32,
    group_share_ppm: Vec<u32>,
}

#[derive(Debug, Clone, Copy)]
struct CratonSeed {
    group: u8,
    center: Vec3,
    home_plate: u16,
    variation_axis: Vec3,
    radius_radians: f64,
    warp_seed: u64,
}

#[derive(Debug, Clone, Copy)]
struct GenerationIdentity {
    seed: u32,
    retry_index: u32,
}

struct ReliefInputs<'a> {
    grid: Grid,
    identity: GenerationIdentity,
    settings: TectonicSettings,
    plates: &'a [Plate],
    plate_by_cell: &'a [u16],
    crust_by_cell: &'a [CrustType],
    boundaries: &'a [BoundarySegment],
    progress: &'a mut dyn ProgressSink,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ReliefTerm {
    Baseline,
    Collision,
    Trench,
    InlandArc,
    OceanicArc,
    Ridge,
    RiftFloor,
    RiftShoulders,
    Transform,
    Hotspot,
    Detail,
}

const RELIEF_TERMS: [ReliefTerm; 11] = [
    ReliefTerm::Baseline,
    ReliefTerm::Collision,
    ReliefTerm::Trench,
    ReliefTerm::InlandArc,
    ReliefTerm::OceanicArc,
    ReliefTerm::Ridge,
    ReliefTerm::RiftFloor,
    ReliefTerm::RiftShoulders,
    ReliefTerm::Transform,
    ReliefTerm::Hotspot,
    ReliefTerm::Detail,
];

struct NamedRelief {
    terms: [Vec<f64>; 11],
}

impl NamedRelief {
    fn new(sample_count: usize) -> Self {
        Self {
            terms: std::array::from_fn(|_| vec![0.0; sample_count]),
        }
    }

    fn term_mut(&mut self, term: ReliefTerm) -> &mut [f64] {
        &mut self.terms[term as usize]
    }

    fn add(&mut self, term: ReliefTerm, cell: usize, value: f64) {
        self.terms[term as usize][cell] += value;
    }

    fn summed_mm(&self) -> Vec<i32> {
        let sample_count = self.terms[0].len();
        (0..sample_count)
            .map(|cell| {
                let value = RELIEF_TERMS
                    .iter()
                    .map(|term| self.terms[*term as usize][cell])
                    .sum::<f64>();
                value.round().clamp(-9_000_000.0, 9_000_000.0) as i32
            })
            .collect()
    }
}

impl Vec3 {
    fn from_lon_lat(longitude: f64, latitude: f64) -> Self {
        let latitude_cos = latitude.cos();
        Self {
            x: latitude_cos * longitude.cos(),
            y: latitude_cos * longitude.sin(),
            z: latitude.sin(),
        }
    }

    fn dot(self, other: Self) -> f64 {
        self.x * other.x + self.y * other.y + self.z * other.z
    }

    fn angular_distance(self, other: Self) -> f64 {
        self.dot(other).clamp(-1.0, 1.0).acos()
    }

    fn cross(self, other: Self) -> Self {
        Self {
            x: self.y * other.z - self.z * other.y,
            y: self.z * other.x - self.x * other.z,
            z: self.x * other.y - self.y * other.x,
        }
    }

    fn subtract(self, other: Self) -> Self {
        Self {
            x: self.x - other.x,
            y: self.y - other.y,
            z: self.z - other.z,
        }
    }

    fn scale(self, factor: f64) -> Self {
        Self {
            x: self.x * factor,
            y: self.y * factor,
            z: self.z * factor,
        }
    }

    fn add(self, other: Self) -> Self {
        Self {
            x: self.x + other.x,
            y: self.y + other.y,
            z: self.z + other.z,
        }
    }

    fn normalize(self) -> Self {
        let length = self.dot(self).sqrt();
        if length == 0.0 {
            Self {
                x: 1.0,
                y: 0.0,
                z: 0.0,
            }
        } else {
            self.scale(1.0 / length)
        }
    }

    fn reject(self, axis: Self) -> Self {
        self.subtract(axis.scale(self.dot(axis)))
    }

    fn rotate_around(self, axis: Self, angle: f64) -> Self {
        let axis = axis.normalize();
        let cos = angle.cos();
        let sin = angle.sin();
        self.scale(cos)
            .add(axis.cross(self).scale(sin))
            .add(axis.scale(axis.dot(self) * (1.0 - cos)))
            .normalize()
    }
}

fn microdegrees(degrees: f64) -> i32 {
    degrees
        .mul_add(MICRODEGREES_PER_DEGREE, 0.0)
        .round()
        .clamp(-180_000_000.0, 180_000_000.0) as i32
}

fn plate_site(_grid: Grid, seed: u32, retry_index: u32, id: u16, count: u16) -> Vec3 {
    let sequence = f64::from(id) + 0.5;
    let z = 1.0 - 2.0 * sequence / f64::from(count);
    let stage_seed = derive_subsystem_seed(seed, retry_index, SeedDomain::PlateSites);
    let jitter = (splitmix64(stage_seed ^ u64::from(id).wrapping_mul(0x9e37_79b9_7f4a_7c15))
        % 1_000_000) as f64
        / 1_000_000.0
        * 0.04
        - 0.02;
    let golden_angle = std::f64::consts::PI * (3.0 - 5.0_f64.sqrt());
    let longitude = (f64::from(id) * golden_angle + jitter).rem_euclid(std::f64::consts::TAU)
        - std::f64::consts::PI;
    Vec3::from_lon_lat(longitude, z.clamp(-1.0, 1.0).asin())
}

fn cell_vector(grid: Grid, cell: usize) -> Vec3 {
    let (row, col) = grid.row_col(cell);
    let (longitude, latitude) = grid.center_radians(row, col);
    Vec3::from_lon_lat(longitude, latitude)
}

fn lattice_hash(seed: u64, x: i32, y: i32, z: i32) -> f64 {
    let mixed = seed
        ^ (x as u64).wrapping_mul(0x9e37_79b9_7f4a_7c15)
        ^ (y as u64).wrapping_mul(0xa076_1d64_78bd_642f)
        ^ (z as u64).wrapping_mul(0x243f_6a88_85a3_08d3);
    (splitmix64(mixed) % 1_000_001) as f64 / 1_000_000.0 * 2.0 - 1.0
}

fn fade(t: f64) -> f64 {
    t * t * (3.0 - 2.0 * t)
}

fn value_noise(seed: u64, point: Vec3, frequency: f64) -> f64 {
    let x = point.x * frequency;
    let y = point.y * frequency;
    let z = point.z * frequency;
    let x0 = x.floor() as i32;
    let y0 = y.floor() as i32;
    let z0 = z.floor() as i32;
    let tx = fade(x - f64::from(x0));
    let ty = fade(y - f64::from(y0));
    let tz = fade(z - f64::from(z0));
    let lerp = |a, b, t| a + (b - a) * t;
    let c000 = lattice_hash(seed, x0, y0, z0);
    let c100 = lattice_hash(seed, x0 + 1, y0, z0);
    let c010 = lattice_hash(seed, x0, y0 + 1, z0);
    let c110 = lattice_hash(seed, x0 + 1, y0 + 1, z0);
    let c001 = lattice_hash(seed, x0, y0, z0 + 1);
    let c101 = lattice_hash(seed, x0 + 1, y0, z0 + 1);
    let c011 = lattice_hash(seed, x0, y0 + 1, z0 + 1);
    let c111 = lattice_hash(seed, x0 + 1, y0 + 1, z0 + 1);
    lerp(
        lerp(lerp(c000, c100, tx), lerp(c010, c110, tx), ty),
        lerp(lerp(c001, c101, tx), lerp(c011, c111, tx), ty),
        tz,
    )
}

fn fbm_noise(seed: u64, point: Vec3) -> f64 {
    let mut sum = 0.0;
    let mut amplitude = 1.0;
    let mut frequency = 2.4;
    let mut norm = 0.0;
    for octave in 0..4u64 {
        sum += amplitude
            * value_noise(
                seed ^ octave.wrapping_mul(0x9e37_79b9_7f4a_7c15),
                point,
                frequency,
            );
        norm += amplitude;
        amplitude *= 0.5;
        frequency *= 2.07;
    }
    if norm == 0.0 {
        0.0
    } else {
        sum / norm
    }
}

fn boundary_step_cost(grid: Grid, cost_seed: u64, cell: usize, neighbor: usize) -> i64 {
    let midpoint = cell_midpoint(grid, cell, neighbor);
    let perturbation =
        (fbm_noise(cost_seed, midpoint) * BOUNDARY_COST_PERTURBATION_PPM as f64).round() as i64;
    let wave = (value_noise(cost_seed ^ 0x51ed_a11e_5a17_0001, midpoint, 6.1)
        * BOUNDARY_WAVE_COST_PPM as f64)
        .round() as i64;
    let fault = fbm_noise(cost_seed ^ 0xfa17_15ed_0000_0002, midpoint).abs();
    let fault_follow = (((0.16 - fault).max(0.0) / 0.16) * FAULT_FOLLOW_PPM as f64).round() as i64;
    let combined = (perturbation + wave + fault_follow).clamp(
        -(BOUNDARY_COST_PERTURBATION_PPM + BOUNDARY_WAVE_COST_PPM),
        BOUNDARY_COST_PERTURBATION_PPM + BOUNDARY_WAVE_COST_PPM + FAULT_FOLLOW_PPM,
    );
    edge_length_mm(grid, cell, neighbor).saturating_mul(1_000_000 + combined) / 1_000_000
}

fn continent_layout(seed: u32, retry_index: u32, settings: TectonicSettings) -> ContinentLayout {
    let stage_seed = derive_subsystem_seed(seed, retry_index, SeedDomain::ContinentalCratons);
    let roll = splitmix64(stage_seed) % LAYOUT_WEIGHT_TOTAL;
    let kind = if roll < LAYOUT_MEGA_WEIGHT {
        ContinentLayoutKind::Mega
    } else if roll < LAYOUT_MEGA_WEIGHT + LAYOUT_GIANT_SCRAPS_WEIGHT {
        ContinentLayoutKind::GiantScraps
    } else if roll < LAYOUT_MEGA_WEIGHT + LAYOUT_GIANT_SCRAPS_WEIGHT + LAYOUT_DUAL_WEIGHT {
        ContinentLayoutKind::Dual
    } else {
        ContinentLayoutKind::Scattered
    };
    layout_for_kind(kind, stage_seed, settings)
}

fn layout_for_kind(
    kind: ContinentLayoutKind,
    stage_seed: u64,
    settings: TectonicSettings,
) -> ContinentLayout {
    let max_groups = usize::from(settings.continental_plate_count).max(1);
    match kind {
        ContinentLayoutKind::Mega => ContinentLayout {
            group_count: 1,
            companion_threshold: 4,
            base_radius: 0.96,
            companion_offset_min: 0.16,
            companion_offset_span: 0.18,
            plate_crossing_ppm: 90_000,
            same_group_attraction_ppm: 520_000,
            other_group_repulsion_ppm: 780_000,
            occupied_ppm: 950_000,
            junction_attraction_ppm: 420_000,
            crust_target_ppm: 520_000,
            terrane_budget_ppm: 8_000,
            group_share_ppm: vec![1_000_000],
        },
        ContinentLayoutKind::GiantScraps => {
            let scrap_count = (2 + (splitmix64(stage_seed ^ 0x5c4a_0001) % 2) as usize)
                .min(max_groups.saturating_sub(1).max(1));
            let group_count = (1 + scrap_count).min(max_groups.max(2));
            let mut shares = vec![0u32; group_count];
            shares[0] = 760_000;
            let leftover = 240_000;
            let scrap_share = leftover / (group_count as u32 - 1).max(1);
            for share in shares.iter_mut().skip(1) {
                *share = scrap_share;
            }
            shares[group_count - 1] += leftover - scrap_share * (group_count as u32 - 1);
            ContinentLayout {
                group_count,
                companion_threshold: 3,
                base_radius: 0.88,
                companion_offset_min: 0.18,
                companion_offset_span: 0.22,
                plate_crossing_ppm: 220_000,
                same_group_attraction_ppm: 460_000,
                other_group_repulsion_ppm: 640_000,
                occupied_ppm: 900_000,
                junction_attraction_ppm: 480_000,
                crust_target_ppm: 500_000,
                terrane_budget_ppm: 28_000,
                group_share_ppm: shares,
            }
        }
        ContinentLayoutKind::Dual => {
            let primary = 560_000 + (splitmix64(stage_seed ^ 0x2d1a_7c15) % 120_001) as u32;
            ContinentLayout {
                group_count: 2.min(max_groups).max(1),
                companion_threshold: 2,
                base_radius: 0.70,
                companion_offset_min: 0.20,
                companion_offset_span: 0.18,
                plate_crossing_ppm: 380_000,
                same_group_attraction_ppm: 320_000,
                other_group_repulsion_ppm: 760_000,
                occupied_ppm: 920_000,
                junction_attraction_ppm: 380_000,
                crust_target_ppm: CONTINENTAL_AREA_TARGET_PPM,
                terrane_budget_ppm: 18_000,
                group_share_ppm: if max_groups == 1 {
                    vec![1_000_000]
                } else {
                    vec![primary, 1_000_000 - primary]
                },
            }
        }
        ContinentLayoutKind::Scattered => {
            let extra = splitmix64(stage_seed ^ 0x5c4a_11e0) as usize % 4;
            let group_count = (5 + extra).clamp(4, max_groups.max(4)).min(8);
            let mut shares = vec![1_000_000 / group_count as u32; group_count];
            let leftover = 1_000_000 - shares.iter().sum::<u32>();
            if leftover > 0 {
                shares[0] += leftover;
            }
            ContinentLayout {
                group_count,
                companion_threshold: 1,
                base_radius: 0.36,
                companion_offset_min: 0.12,
                companion_offset_span: 0.12,
                plate_crossing_ppm: 920_000,
                same_group_attraction_ppm: 140_000,
                other_group_repulsion_ppm: 880_000,
                occupied_ppm: 980_000,
                junction_attraction_ppm: 280_000,
                crust_target_ppm: 400_000,
                terrane_budget_ppm: TERRANE_AREA_BUDGET_PPM,
                group_share_ppm: shares,
            }
        }
    }
}

fn craton_seeds(layout: &ContinentLayout, stage_seed: u64) -> Vec<CratonSeed> {
    let golden_angle = std::f64::consts::PI * (3.0 - 5.0_f64.sqrt());
    let mut cratons = Vec::new();
    for group in 0..layout.group_count {
        let sequence = group as f64 + 0.5;
        let z = 1.0 - 2.0 * sequence / layout.group_count as f64;
        let jitter = (splitmix64(stage_seed ^ (group as u64).wrapping_mul(0x9e37_79b9_7f4a_7c15))
            % 1_000_000) as f64
            / 1_000_000.0
            * 0.18
            - 0.09;
        let longitude = (group as f64 * golden_angle + jitter).rem_euclid(std::f64::consts::TAU)
            - std::f64::consts::PI;
        let latitude_jitter =
            (splitmix64(stage_seed ^ (group as u64).wrapping_mul(0xbf58_476d_1ce4_e5b9))
                % 1_000_000) as f64
                / 1_000_000.0
                * 0.22
                - 0.11;
        let primary = Vec3::from_lon_lat(
            longitude,
            (z.clamp(-1.0, 1.0).asin() + latitude_jitter).clamp(
                -std::f64::consts::FRAC_PI_2 + 0.008,
                std::f64::consts::FRAC_PI_2 - 0.008,
            ),
        );
        cratons.push(make_craton(
            stage_seed,
            group as u8,
            group as u64,
            primary,
            layout.base_radius,
        ));
        let companion_draw = splitmix64(
            stage_seed ^ 0xcafe_f00d ^ (group as u64).wrapping_mul(0xa076_1d64_78bd_642f),
        );
        if layout.companion_threshold > 0 && companion_draw % 5 < layout.companion_threshold {
            let offset_angle = layout.companion_offset_min
                + (companion_draw % 1_000_001) as f64 / 1_000_000.0 * layout.companion_offset_span;
            let axis = Vec3 {
                x: primary.y,
                y: -primary.x,
                z: 0.0,
            }
            .normalize();
            let companion = primary.rotate_around(axis, offset_angle);
            cratons.push(make_craton(
                stage_seed,
                group as u8,
                (group as u64) ^ 0x1111_2222_3333_4444,
                companion,
                (layout.base_radius * 0.62).clamp(0.22, 0.78),
            ));
        }
    }
    cratons
}

fn make_craton(stage_seed: u64, group: u8, id: u64, center: Vec3, base_radius: f64) -> CratonSeed {
    let variation_seed = splitmix64(stage_seed ^ id.wrapping_mul(0xa076_1d64_78bd_642f));
    let variation_longitude =
        (variation_seed as f64 / u64::MAX as f64) * std::f64::consts::TAU - std::f64::consts::PI;
    let variation_latitude = (((variation_seed >> 32) as f64 / u32::MAX as f64) * 2.0 - 1.0)
        .clamp(-1.0, 1.0)
        .asin();
    let radius_jitter = (splitmix64(variation_seed) % 1_000_001) as f64 / 1_000_000.0 * 0.16 - 0.08;
    CratonSeed {
        group,
        center,
        home_plate: 0,
        variation_axis: Vec3::from_lon_lat(variation_longitude, variation_latitude),
        radius_radians: (base_radius + radius_jitter).clamp(0.28, 1.05),
        warp_seed: splitmix64(variation_seed ^ 0xdead_beef_cafe_f00d),
    }
}

fn lithology_variation(craton: CratonSeed, vector: Vec3) -> f64 {
    let warp = fbm_noise(craton.warp_seed, vector);
    let anisotropy = 1.0 + vector.dot(craton.variation_axis) * CRATON_ANISOTROPY;
    (warp * CRATON_WARP_AMPLITUDE + anisotropy - 1.0).clamp(-0.85, 0.85)
}

fn bind_cratons_to_plates(cratons: &mut [CratonSeed], grid: Grid, plate_by_cell: &[u16]) {
    for craton in cratons.iter_mut() {
        let cell = nearest_cell(grid, craton.center);
        craton.home_plate = plate_by_cell[cell];
    }
    let mut primary = [u16::MAX; 16];
    for craton in cratons.iter() {
        let group = usize::from(craton.group);
        if group < primary.len() && primary[group] == u16::MAX {
            primary[group] = craton.home_plate;
        }
    }
    for craton in cratons.iter_mut() {
        let group = usize::from(craton.group);
        if group < primary.len() && primary[group] != u16::MAX {
            craton.home_plate = primary[group];
        }
    }
}

fn plate_rotation(identity: GenerationIdentity, id: u16) -> (i32, i32, i64) {
    let axis_stage_seed = derive_subsystem_seed(
        identity.seed,
        identity.retry_index,
        SeedDomain::RotationAxes,
    );
    let axis_seed = splitmix64(axis_stage_seed ^ u64::from(id).wrapping_mul(0xa076_1d64_78bd_642f));
    let axis_longitude =
        (axis_seed as f64 / u64::MAX as f64) * std::f64::consts::TAU - std::f64::consts::PI;
    let axis_latitude = (((axis_seed >> 32) as f64 / u32::MAX as f64) * 2.0 - 1.0)
        .clamp(-1.0, 1.0)
        .asin();
    let speed = 200_000 + (splitmix64(axis_seed) % 1_000_001) as i64;
    (
        microdegrees(axis_longitude.to_degrees()),
        microdegrees(axis_latitude.to_degrees()),
        speed,
    )
}

fn plate_from_site(id: u16, seed_cell: usize, site: Vec3, identity: GenerationIdentity) -> Plate {
    let (axis_longitude, axis_latitude, speed) = plate_rotation(identity, id);
    Plate {
        id,
        seed_cell,
        site_longitude_microdegrees: microdegrees(site.y.atan2(site.x).to_degrees()),
        site_latitude_microdegrees: microdegrees(site.z.asin().to_degrees()),
        rotation_axis_longitude_microdegrees: axis_longitude,
        rotation_axis_latitude_microdegrees: axis_latitude,
        angular_speed_nanoradians_per_year: speed,
        crust_type: CrustType::Oceanic,
    }
}

fn build_plates(grid: Grid, seed: u32, retry_index: u32, settings: TectonicSettings) -> Vec<Plate> {
    let identity = GenerationIdentity { seed, retry_index };
    (0..settings.plate_count)
        .map(|id| {
            let site = plate_site(grid, seed, retry_index, id, settings.plate_count);
            plate_from_site(id, nearest_cell(grid, site), site, identity)
        })
        .collect()
}

fn assign_majority_crust(plates: &mut [Plate], plate_by_cell: &[u16], crust_by_cell: &[CrustType]) {
    let mut continental = vec![0u32; plates.len()];
    let mut oceanic = vec![0u32; plates.len()];
    for (cell, plate) in plate_by_cell.iter().copied().enumerate() {
        match crust_by_cell[cell] {
            CrustType::Continental => continental[usize::from(plate)] += 1,
            CrustType::Oceanic => oceanic[usize::from(plate)] += 1,
        }
    }
    for plate in plates {
        plate.crust_type = if continental[usize::from(plate.id)] >= oceanic[usize::from(plate.id)] {
            CrustType::Continental
        } else {
            CrustType::Oceanic
        };
    }
}

fn nearest_cell(grid: Grid, site: Vec3) -> usize {
    (0..grid.sample_count())
        .max_by(|first, second| {
            cell_vector(grid, *first)
                .dot(site)
                .partial_cmp(&cell_vector(grid, *second).dot(site))
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| second.cmp(first))
        })
        .unwrap_or(0)
}

fn edge_length_mm(grid: Grid, first: usize, second: usize) -> i64 {
    let first_center = grid.row_col(first);
    let second_center = grid.row_col(second);
    grid.great_circle_distance(
        grid.center_radians(first_center.0, first_center.1),
        grid.center_radians(second_center.0, second_center.1),
    )
    .mul_add(1_000.0, 0.0)
    .round()
    .max(1.0) as i64
}

fn cell_midpoint(grid: Grid, first: usize, second: usize) -> Vec3 {
    cell_vector(grid, first)
        .add(cell_vector(grid, second))
        .normalize()
}

fn nearest_site_owner(sites: &[Vec3], vector: Vec3) -> u16 {
    sites
        .iter()
        .enumerate()
        .max_by(|(first_id, first), (second_id, second)| {
            first
                .dot(vector)
                .partial_cmp(&second.dot(vector))
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| second_id.cmp(first_id))
        })
        .map(|(plate, _)| plate as u16)
        .unwrap_or(0)
}

#[cfg(test)]
fn assign_nearest_plates(grid: Grid, sites: &[Vec3]) -> Vec<u16> {
    (0..grid.sample_count())
        .map(|cell| nearest_site_owner(sites, cell_vector(grid, cell)))
        .collect()
}

fn assign_plates(
    grid: Grid,
    sites: &[Vec3],
    identity: GenerationIdentity,
    progress: &mut dyn ProgressSink,
) -> Result<Vec<u16>, PhysicalError> {
    let cost_seed = derive_subsystem_seed(
        identity.seed,
        identity.retry_index,
        SeedDomain::PlateBoundaryCost,
    );
    let topology = grid.topology();
    let mut best_cost = vec![i64::MAX; grid.sample_count()];
    let mut owner = vec![u16::MAX; grid.sample_count()];
    let mut heap = BinaryHeap::new();
    for (id, site) in sites.iter().copied().enumerate() {
        let cell = nearest_cell(grid, site);
        heap.push(Reverse((0_i64, cell, id as u16)));
    }
    let mut visits = 0u32;
    while let Some(Reverse((cost, cell, plate))) = heap.pop() {
        visits += 1;
        if visits.is_multiple_of(512) {
            progress.check_cancelled()?;
        }
        if cost > best_cost[cell] {
            continue;
        }
        if cost == best_cost[cell] && plate >= owner[cell] {
            continue;
        }
        best_cost[cell] = cost;
        owner[cell] = plate;
        for neighbor in topology.neighbors(cell) {
            let step = boundary_step_cost(grid, cost_seed, cell, *neighbor).max(1);
            heap.push(Reverse((cost.saturating_add(step), *neighbor, plate)));
        }
    }
    for (cell, assigned) in owner.iter_mut().enumerate() {
        if *assigned == u16::MAX {
            *assigned = nearest_site_owner(sites, cell_vector(grid, cell));
        }
    }
    Ok(owner)
}

fn reshape_event_count(plate_count: usize) -> usize {
    if plate_count < 4 {
        return 0;
    }
    let proposed = (plate_count * PLATE_RESHAPE_FRACTION_PPM as usize / 1_000_000).max(1);
    proposed.min(plate_count.saturating_sub(2) / 2)
}

fn adjacent_plate_pairs(grid: Grid, plate_by_cell: &[u16]) -> Vec<(u16, u16)> {
    let mut seen = HashSet::new();
    let topology = grid.topology();
    for cell in 0..grid.sample_count() {
        let plate = plate_by_cell[cell];
        for neighbor in topology.neighbors(cell) {
            let other = plate_by_cell[*neighbor];
            if other == plate {
                continue;
            }
            let key = if plate < other {
                (plate, other)
            } else {
                (other, plate)
            };
            seen.insert(key);
        }
    }
    let mut pairs: Vec<(u16, u16)> = seen.into_iter().collect();
    pairs.sort_unstable();
    pairs
}

fn plate_cells(plate_by_cell: &[u16], plate: u16) -> Vec<usize> {
    plate_by_cell
        .iter()
        .enumerate()
        .filter(|(_, assigned)| **assigned == plate)
        .map(|(cell, _)| cell)
        .collect()
}

fn pick_shatter_seed(grid: Grid, cells: &[usize], parent_seed: usize, rng: u64) -> Option<usize> {
    if cells.len() < MIN_SHATTER_CELLS {
        return None;
    }
    let parent_vec = cell_vector(grid, parent_seed);
    let mut ranked: Vec<(i64, usize)> = cells
        .iter()
        .copied()
        .map(|cell| {
            let distance = (cell_vector(grid, cell).angular_distance(parent_vec) * 1_000_000_000.0)
                .round() as i64;
            (distance, cell)
        })
        .collect();
    ranked.sort_unstable();
    let start = ranked.len() / 3;
    let span = ranked.len() - start;
    if span == 0 {
        return None;
    }
    let chosen = ranked[start + (rng as usize % span)].1;
    if chosen == parent_seed {
        None
    } else {
        Some(chosen)
    }
}

#[allow(clippy::too_many_arguments)]
fn shatter_plate(
    grid: Grid,
    plate_by_cell: &mut [u16],
    parent: u16,
    shard: u16,
    parent_seed: usize,
    shard_seed: usize,
    identity: GenerationIdentity,
    progress: &mut dyn ProgressSink,
) -> Result<(), PhysicalError> {
    let cost_seed = derive_subsystem_seed(
        identity.seed,
        identity.retry_index,
        SeedDomain::PlateBoundaryCost,
    ) ^ u64::from(shard).wrapping_mul(0x9e37_79b9_7f4a_7c15);
    let in_parent: Vec<bool> = plate_by_cell.iter().map(|plate| *plate == parent).collect();
    let topology = grid.topology();
    let mut best_cost = vec![i64::MAX; grid.sample_count()];
    let mut owner = vec![u16::MAX; grid.sample_count()];
    let mut heap = BinaryHeap::new();
    heap.push(Reverse((0_i64, parent_seed, parent)));
    heap.push(Reverse((0_i64, shard_seed, shard)));
    let mut visits = 0u32;
    while let Some(Reverse((cost, cell, plate))) = heap.pop() {
        visits += 1;
        if visits.is_multiple_of(512) {
            progress.check_cancelled()?;
        }
        if !in_parent[cell] {
            continue;
        }
        if cost > best_cost[cell] {
            continue;
        }
        if cost == best_cost[cell] && plate >= owner[cell] {
            continue;
        }
        best_cost[cell] = cost;
        owner[cell] = plate;
        for neighbor in topology.neighbors(cell) {
            if !in_parent[*neighbor] {
                continue;
            }
            let step = boundary_step_cost(grid, cost_seed, cell, *neighbor).max(1);
            heap.push(Reverse((cost.saturating_add(step), *neighbor, plate)));
        }
    }
    for (cell, assigned) in owner.iter().enumerate() {
        if *assigned == shard {
            plate_by_cell[cell] = shard;
        }
    }
    Ok(())
}

fn merge_plate(plate_by_cell: &mut [u16], keep: u16, absorb: u16) {
    for plate in plate_by_cell.iter_mut() {
        if *plate == absorb {
            *plate = keep;
        }
    }
}

fn plate_centroid(grid: Grid, plate_by_cell: &[u16], plate: u16) -> Vec3 {
    let mut sum = Vec3 {
        x: 0.0,
        y: 0.0,
        z: 0.0,
    };
    let mut count = 0.0;
    for (cell, assigned) in plate_by_cell.iter().enumerate() {
        if *assigned == plate {
            sum = sum.add(cell_vector(grid, cell));
            count += 1.0;
        }
    }
    if count == 0.0 {
        Vec3 {
            x: 1.0,
            y: 0.0,
            z: 0.0,
        }
    } else {
        sum.scale(1.0 / count).normalize()
    }
}

fn apply_extra_plate_merges(
    grid: Grid,
    plate_by_cell: &mut [u16],
    reshape_seed: u64,
    plate_count: usize,
) {
    let roll = splitmix64(reshape_seed ^ 0xc011_e570_0000_0001) % 10;
    let extra = match roll {
        0..=4 => reshape_event_count(plate_count).max(1),
        _ => 0,
    };
    if extra == 0 {
        return;
    }
    let mut used = HashSet::new();
    let mut merged = 0usize;
    for (keep, absorb) in adjacent_plate_pairs(grid, plate_by_cell) {
        if merged >= extra {
            break;
        }
        if used.contains(&keep) || used.contains(&absorb) || keep == absorb {
            continue;
        }
        if !plate_present(plate_by_cell, keep) || !plate_present(plate_by_cell, absorb) {
            continue;
        }
        let remaining = (0..plate_count as u16)
            .filter(|id| plate_present(plate_by_cell, *id) && *id != absorb)
            .count();
        if remaining < MIN_PLATES as usize {
            break;
        }
        merge_plate(plate_by_cell, keep, absorb);
        used.insert(keep);
        used.insert(absorb);
        merged += 1;
    }
}

fn plate_present(plate_by_cell: &[u16], plate: u16) -> bool {
    plate_by_cell.contains(&plate)
}

fn compact_plates(
    grid: Grid,
    plates: &[Plate],
    plate_by_cell: &mut [u16],
    identity: GenerationIdentity,
) -> Vec<Plate> {
    let mut max_id = 0usize;
    for plate in plate_by_cell.iter() {
        max_id = max_id.max(usize::from(*plate));
    }
    let mut present = vec![false; max_id + 1];
    for plate in plate_by_cell.iter() {
        present[usize::from(*plate)] = true;
    }
    let mut remap = vec![u16::MAX; present.len()];
    let mut next_id = 0u16;
    for (old, is_present) in present.iter().copied().enumerate() {
        if is_present {
            remap[old] = next_id;
            next_id += 1;
        }
    }
    for plate in plate_by_cell.iter_mut() {
        *plate = remap[usize::from(*plate)];
    }
    let mut rebuilt = Vec::with_capacity(usize::from(next_id));
    for (old, is_present) in present.iter().copied().enumerate() {
        if !is_present {
            continue;
        }
        let id = remap[old];
        let centroid = plate_centroid(grid, plate_by_cell, id);
        let seed_cell = nearest_cell(grid, centroid);
        let site = cell_vector(grid, seed_cell);
        let (axis_longitude, axis_latitude, speed) = if old < plates.len() {
            (
                plates[old].rotation_axis_longitude_microdegrees,
                plates[old].rotation_axis_latitude_microdegrees,
                plates[old].angular_speed_nanoradians_per_year,
            )
        } else {
            plate_rotation(identity, id)
        };
        rebuilt.push(Plate {
            id,
            seed_cell,
            site_longitude_microdegrees: microdegrees(site.y.atan2(site.x).to_degrees()),
            site_latitude_microdegrees: microdegrees(site.z.asin().to_degrees()),
            rotation_axis_longitude_microdegrees: axis_longitude,
            rotation_axis_latitude_microdegrees: axis_latitude,
            angular_speed_nanoradians_per_year: speed,
            crust_type: CrustType::Oceanic,
        });
    }
    rebuilt
}

fn couple_transform_pairs(
    plates: &mut [Plate],
    grid: Grid,
    plate_by_cell: &[u16],
    identity: GenerationIdentity,
) -> Vec<(u16, u16)> {
    let seed = derive_subsystem_seed(
        identity.seed,
        identity.retry_index,
        SeedDomain::PlateTransformSlip,
    );
    let mut pairs = adjacent_plate_pairs(grid, plate_by_cell);
    pairs.sort_by_key(|(first, second)| {
        splitmix64(
            seed ^ u64::from(*first).wrapping_mul(0x9e37_79b9_7f4a_7c15) ^ u64::from(*second),
        )
    });
    let budget = (plates.len() / TRANSFORM_COUPLE_PAIR_DIVISOR).max(1);
    let mut coupled = vec![false; plates.len()];
    let mut selected = Vec::new();
    for (first, second) in pairs {
        if selected.len() >= budget {
            break;
        }
        let first_index = usize::from(first);
        let second_index = usize::from(second);
        if first_index >= plates.len()
            || second_index >= plates.len()
            || coupled[first_index]
            || coupled[second_index]
        {
            continue;
        }
        plates[second_index].rotation_axis_longitude_microdegrees =
            plates[first_index].rotation_axis_longitude_microdegrees;
        plates[second_index].rotation_axis_latitude_microdegrees =
            plates[first_index].rotation_axis_latitude_microdegrees;
        let delta = 80_000 + (splitmix64(seed ^ u64::from(second)) % 420_001) as i64;
        plates[second_index].angular_speed_nanoradians_per_year =
            (plates[first_index].angular_speed_nanoradians_per_year + delta)
                .clamp(200_000, 1_800_000);
        coupled[first_index] = true;
        coupled[second_index] = true;
        selected.push((first, second));
    }
    selected
}

fn walk_direction(grid: Grid, start: usize, direction: Vec3, hops: usize) -> usize {
    let topology = grid.topology();
    let mut cell = start;
    for _ in 0..hops {
        let origin = cell_vector(grid, cell);
        let mut best = cell;
        let mut best_dot = f64::NEG_INFINITY;
        for neighbor in topology.neighbors(cell) {
            let step = cell_vector(grid, *neighbor)
                .subtract(origin)
                .reject(origin)
                .normalize();
            let alignment = step.dot(direction);
            if alignment > best_dot {
                best_dot = alignment;
                best = *neighbor;
            }
        }
        if best == cell {
            break;
        }
        cell = best;
    }
    cell
}

fn contact_band(
    grid: Grid,
    plate_by_cell: &[u16],
    first: u16,
    second: u16,
    width: usize,
) -> Vec<usize> {
    let topology = grid.topology();
    let mut distance = vec![-1i16; grid.sample_count()];
    let mut queue = Vec::new();
    for cell in 0..grid.sample_count() {
        let plate = plate_by_cell[cell];
        if plate != first && plate != second {
            continue;
        }
        let touches = topology.neighbors(cell).iter().any(|neighbor| {
            let other = plate_by_cell[*neighbor];
            (plate == first && other == second) || (plate == second && other == first)
        });
        if touches {
            distance[cell] = 0;
            queue.push(cell);
        }
    }
    let mut cursor = 0usize;
    while cursor < queue.len() {
        let cell = queue[cursor];
        cursor += 1;
        if distance[cell] as usize >= width {
            continue;
        }
        for neighbor in topology.neighbors(cell) {
            let plate = plate_by_cell[*neighbor];
            if (plate != first && plate != second) || distance[*neighbor] != -1 {
                continue;
            }
            distance[*neighbor] = distance[cell] + 1;
            queue.push(*neighbor);
        }
    }
    queue
}

fn apply_transform_slip(
    grid: Grid,
    plates: &[Plate],
    plate_by_cell: &mut [u16],
    pairs: &[(u16, u16)],
    identity: GenerationIdentity,
    progress: &mut dyn ProgressSink,
) -> Result<(), PhysicalError> {
    let seed = derive_subsystem_seed(
        identity.seed,
        identity.retry_index,
        SeedDomain::PlateTransformSlip,
    );
    for &(first, second) in pairs {
        progress.check_cancelled()?;
        let hops = TRANSFORM_SLIP_MIN_HOPS
            + (splitmix64(
                seed ^ u64::from(first).wrapping_mul(0xa076_1d64_78bd_642f) ^ u64::from(second),
            ) % TRANSFORM_SLIP_EXTRA_HOPS) as usize;
        let axis = plate_axis(&plates[usize::from(first)]);
        let band = contact_band(grid, plate_by_cell, first, second, hops + 1);
        let previous = plate_by_cell.to_vec();
        let mut next = previous.clone();
        for cell in band {
            let flow = axis.cross(cell_vector(grid, cell));
            if flow.dot(flow) < 1e-12 {
                continue;
            }
            let source = walk_direction(grid, cell, flow.normalize().scale(-1.0), hops);
            let source_plate = previous[source];
            if source_plate == first || source_plate == second {
                next[cell] = source_plate;
            }
        }
        if plate_present(&next, first) && plate_present(&next, second) {
            plate_by_cell.copy_from_slice(&next);
        }
    }
    Ok(())
}

fn diversify_plates(
    grid: Grid,
    mut plates: Vec<Plate>,
    plate_by_cell: &mut [u16],
    identity: GenerationIdentity,
    progress: &mut dyn ProgressSink,
) -> Result<Vec<Plate>, PhysicalError> {
    let reshape_seed = derive_subsystem_seed(
        identity.seed,
        identity.retry_index,
        SeedDomain::PlateReshape,
    );
    let event_count = reshape_event_count(plates.len());
    let mut plate_order: Vec<u16> = (0..plates.len() as u16).collect();
    plate_order.sort_by_key(|id| splitmix64(reshape_seed ^ u64::from(*id)));
    let mut shatter_parents = Vec::new();
    for id in &plate_order {
        if shatter_parents.len() >= event_count {
            break;
        }
        if plate_cells(plate_by_cell, *id).len() >= MIN_SHATTER_CELLS {
            shatter_parents.push(*id);
        }
    }
    let shatter_set: HashSet<u16> = shatter_parents.iter().copied().collect();
    let mut merge_pairs = Vec::new();
    let mut used = shatter_set.clone();
    for (first, second) in adjacent_plate_pairs(grid, plate_by_cell) {
        if merge_pairs.len() >= shatter_parents.len() {
            break;
        }
        if used.contains(&first) || used.contains(&second) {
            continue;
        }
        merge_pairs.push((first, second));
        used.insert(first);
        used.insert(second);
    }
    let merge_budget = merge_pairs.len();
    let mut next_id = plates.len() as u16;
    let mut shattered = 0usize;
    for parent in shatter_parents {
        if shattered >= merge_budget {
            break;
        }
        let parent_cells = plate_cells(plate_by_cell, parent);
        let rng = splitmix64(reshape_seed ^ 0x5a11_e000 ^ u64::from(parent));
        let Some(shard_seed) = pick_shatter_seed(
            grid,
            &parent_cells,
            plates[usize::from(parent)].seed_cell,
            rng,
        ) else {
            continue;
        };
        let shard_id = next_id;
        next_id = next_id.saturating_add(1);
        let shard_site = cell_vector(grid, shard_seed);
        plates.push(plate_from_site(shard_id, shard_seed, shard_site, identity));
        shatter_plate(
            grid,
            plate_by_cell,
            parent,
            shard_id,
            plates[usize::from(parent)].seed_cell,
            shard_seed,
            identity,
            progress,
        )?;
        if plate_present(plate_by_cell, shard_id) {
            shattered += 1;
        }
    }
    for (keep, absorb) in merge_pairs.into_iter().take(shattered) {
        merge_plate(plate_by_cell, keep, absorb);
    }
    apply_extra_plate_merges(grid, plate_by_cell, reshape_seed, plates.len());
    plates = compact_plates(grid, &plates, plate_by_cell, identity);
    let transform_pairs = couple_transform_pairs(&mut plates, grid, plate_by_cell, identity);
    apply_transform_slip(
        grid,
        &plates,
        plate_by_cell,
        &transform_pairs,
        identity,
        progress,
    )?;
    Ok(compact_plates(grid, &plates, plate_by_cell, identity))
}

#[allow(clippy::too_many_arguments)]
fn crust_cost_delta(
    grid: Grid,
    from: usize,
    to: usize,
    craton: CratonSeed,
    layout: &ContinentLayout,
    plates: &[Plate],
    plate_by_cell: &[u16],
    group_by_cell: &[i16],
    crust_by_cell: &[CrustType],
) -> i64 {
    let step = edge_length_mm(grid, from, to);
    let dist = cell_vector(grid, to).angular_distance(craton.center);
    let geodesic = ((dist / craton.radius_radians.max(0.18)) * step as f64).round() as i64;
    let lithology = (lithology_variation(craton, cell_vector(grid, to)) * LITHOLOGY_COST_PPM as f64)
        .round() as i64;
    let plate_crossing = if plate_by_cell[to] == craton.home_plate {
        0
    } else {
        layout.plate_crossing_ppm
    };
    let (mut same_group, mut other_group) = (0_i64, 0_i64);
    for neighbor in grid.topology().neighbors(to) {
        if crust_by_cell[*neighbor] != CrustType::Continental || group_by_cell[*neighbor] < 0 {
            continue;
        }
        if group_by_cell[*neighbor] as u8 == craton.group {
            same_group = layout.same_group_attraction_ppm;
        } else {
            other_group = layout.other_group_repulsion_ppm;
        }
    }
    let occupied = if crust_by_cell[to] == CrustType::Continental {
        layout.occupied_ppm
    } else {
        0
    };
    let mut plates_here = vec![plate_by_cell[to]];
    plates_here.extend(
        grid.topology()
            .neighbors(to)
            .iter()
            .map(|neighbor| plate_by_cell[*neighbor]),
    );
    plates_here.sort_unstable();
    plates_here.dedup();
    let convergent = cell_has_convergent_contact(grid, plates, plate_by_cell, to);
    let junction = if plates_here.len() >= JUNCTION_NEIGHBOR_PLATES {
        if convergent {
            layout.junction_attraction_ppm.saturating_mul(3) / 2
        } else {
            layout.junction_attraction_ppm
        }
    } else if convergent {
        layout.junction_attraction_ppm / 2
    } else {
        0
    };
    step.saturating_add(geodesic)
        .saturating_add(lithology)
        .saturating_add(plate_crossing * step / 1_000_000)
        .saturating_sub(same_group * step / 1_000_000)
        .saturating_add(other_group * step / 1_000_000)
        .saturating_add(occupied * step / 1_000_000)
        .saturating_sub(junction * step / 1_000_000)
        .max(1)
}

fn cell_has_convergent_contact(
    grid: Grid,
    plates: &[Plate],
    plate_by_cell: &[u16],
    cell: usize,
) -> bool {
    let here = plate_by_cell[cell];
    if usize::from(here) >= plates.len() {
        return false;
    }
    let here_vec = cell_vector(grid, cell);
    for neighbor in grid.topology().neighbors(cell) {
        let other = plate_by_cell[*neighbor];
        if other == here || usize::from(other) >= plates.len() {
            continue;
        }
        let (kind, _) = classify_boundary_at(
            &plates[usize::from(here)],
            &plates[usize::from(other)],
            cell_midpoint(grid, cell, *neighbor),
            here_vec,
            cell_vector(grid, *neighbor),
        );
        if kind == BoundaryKind::Convergent {
            return true;
        }
    }
    false
}

fn group_cell_targets(layout: &ContinentLayout, sample_count: usize) -> Vec<usize> {
    let total =
        (u64::from(layout.crust_target_ppm) * sample_count as u64 / 1_000_000).max(1) as usize;
    let mut targets = layout
        .group_share_ppm
        .iter()
        .map(|share| ((total as u64 * u64::from(*share)) / 1_000_000).max(1) as usize)
        .collect::<Vec<_>>();
    if targets.len() < layout.group_count {
        targets.resize(layout.group_count, 1);
    }
    let assigned = targets.iter().sum::<usize>();
    if assigned > total {
        targets[0] = targets[0].saturating_sub(assigned - total).max(1);
    }
    targets
}

fn grow_continental_crust(
    grid: Grid,
    cratons: &[CratonSeed],
    layout: &ContinentLayout,
    plates: &[Plate],
    plate_by_cell: &[u16],
    identity: GenerationIdentity,
    progress: &mut dyn ProgressSink,
) -> Result<(Vec<CrustType>, Vec<i16>), PhysicalError> {
    let sample_count = grid.sample_count();
    let mut crust = vec![CrustType::Oceanic; sample_count];
    let mut group_by_cell = vec![-1_i16; sample_count];
    let mut heap = BinaryHeap::new();
    for (index, craton) in cratons.iter().copied().enumerate() {
        let cell = nearest_cell(grid, craton.center);
        heap.push(Reverse((0_i64, cell, index as u16)));
    }
    let group_targets = group_cell_targets(layout, sample_count);
    let mut group_grown = vec![0usize; layout.group_count];
    let mut visits = 0u32;
    let topology = grid.topology();
    while let Some(Reverse((cost, cell, craton_id))) = heap.pop() {
        visits += 1;
        if visits.is_multiple_of(512) {
            progress.check_cancelled()?;
        }
        if crust[cell] == CrustType::Continental {
            continue;
        }
        let craton = cratons[usize::from(craton_id)];
        let group = usize::from(craton.group);
        if group >= group_targets.len() || group_grown[group] >= group_targets[group] {
            continue;
        }
        crust[cell] = CrustType::Continental;
        group_by_cell[cell] = i16::from(craton.group);
        group_grown[group] += 1;
        for neighbor in topology.neighbors(cell) {
            if crust[*neighbor] == CrustType::Continental {
                continue;
            }
            let next = cost.saturating_add(crust_cost_delta(
                grid,
                cell,
                *neighbor,
                craton,
                layout,
                plates,
                plate_by_cell,
                &group_by_cell,
                &crust,
            ));
            heap.push(Reverse((next, *neighbor, craton_id)));
        }
    }
    grow_detached_terranes(
        grid,
        identity,
        layout,
        plate_by_cell,
        &mut crust,
        &mut group_by_cell,
        progress,
    )?;
    Ok((crust, group_by_cell))
}

fn grow_detached_terranes(
    grid: Grid,
    identity: GenerationIdentity,
    layout: &ContinentLayout,
    plate_by_cell: &[u16],
    crust: &mut [CrustType],
    group_by_cell: &mut [i16],
    progress: &mut dyn ProgressSink,
) -> Result<(), PhysicalError> {
    let terrane_seed = derive_subsystem_seed(
        identity.seed,
        identity.retry_index,
        SeedDomain::DetachedTerranes,
    );
    let budget = if layout.terrane_budget_ppm == 0 {
        0
    } else {
        (u64::from(layout.terrane_budget_ppm) * grid.sample_count() as u64 / 1_000_000).max(1)
            as usize
    };
    if budget == 0 {
        return Ok(());
    }
    let terrane_count = 1 + (splitmix64(terrane_seed) as usize % 3);
    let topology = grid.topology();
    let mut remaining = budget;
    let mut cursor = 0usize;
    for terrane in 0..terrane_count {
        if remaining == 0 {
            break;
        }
        let pick =
            splitmix64(terrane_seed ^ (terrane as u64 + 1).wrapping_mul(0x9e37_79b9_7f4a_7c15))
                as usize;
        let mut start = None;
        for offset in 0..grid.sample_count() {
            let cell = (pick + offset) % grid.sample_count();
            if crust[cell] == CrustType::Oceanic {
                start = Some(cell);
                break;
            }
        }
        let Some(start) = start else {
            break;
        };
        let home_plate = plate_by_cell[start];
        let mut heap = BinaryHeap::new();
        heap.push(Reverse((0_i64, start)));
        let mut grown = 0usize;
        let local_budget = (remaining / (terrane_count - terrane)).max(1);
        while let Some(Reverse((cost, cell))) = heap.pop() {
            cursor += 1;
            if cursor.is_multiple_of(256) {
                progress.check_cancelled()?;
            }
            if crust[cell] == CrustType::Continental || grown >= local_budget {
                continue;
            }
            crust[cell] = CrustType::Continental;
            group_by_cell[cell] = 100 + terrane as i16;
            grown += 1;
            remaining = remaining.saturating_sub(1);
            for neighbor in topology.neighbors(cell) {
                if crust[*neighbor] == CrustType::Continental {
                    continue;
                }
                let mut step = edge_length_mm(grid, cell, *neighbor);
                if plate_by_cell[*neighbor] != home_plate {
                    step = step.saturating_add(step * layout.plate_crossing_ppm / 1_000_000);
                }
                heap.push(Reverse((cost.saturating_add(step.max(1)), *neighbor)));
            }
        }
    }
    Ok(())
}

fn plate_axis(plate: &Plate) -> Vec3 {
    Vec3::from_lon_lat(
        f64::from(plate.rotation_axis_longitude_microdegrees) / MICRODEGREES_PER_DEGREE
            * std::f64::consts::PI
            / 180.0,
        f64::from(plate.rotation_axis_latitude_microdegrees) / MICRODEGREES_PER_DEGREE
            * std::f64::consts::PI
            / 180.0,
    )
}

fn plate_velocity(plate: &Plate, site: Vec3) -> Vec3 {
    plate_axis(plate)
        .cross(site)
        .scale(plate.angular_speed_nanoradians_per_year as f64)
}

fn boundary_key(first_cell: usize, second_cell: usize) -> (usize, usize) {
    if first_cell < second_cell {
        (first_cell, second_cell)
    } else {
        (second_cell, first_cell)
    }
}

fn classify_boundary_at(
    first: &Plate,
    second: &Plate,
    midpoint: Vec3,
    first_point: Vec3,
    second_point: Vec3,
) -> (BoundaryKind, u64) {
    let relative = plate_velocity(second, midpoint).subtract(plate_velocity(first, midpoint));
    let normal = second_point
        .subtract(first_point)
        .reject(midpoint)
        .normalize();
    let tangent = midpoint.cross(normal).normalize();
    let normal_speed = relative.dot(normal);
    let tangent_speed = relative.dot(tangent);
    let kind = if normal_speed.abs() < RELATIVE_MOTION_THRESHOLD_NANORADIANS_PER_YEAR
        || tangent_speed.abs() * 1_000_000.0 > normal_speed.abs() * TRANSFORM_TIE_RATIO_PPM as f64
    {
        BoundaryKind::Transform
    } else if normal_speed < 0.0 {
        BoundaryKind::Convergent
    } else {
        BoundaryKind::Divergent
    };
    let magnitude = relative.dot(relative).sqrt().round().max(1.0) as u64;
    (kind, magnitude)
}

#[cfg(test)]
fn classify_boundary(first: &Plate, second: &Plate) -> (BoundaryKind, u64) {
    let first_site = Vec3::from_lon_lat(
        f64::from(first.site_longitude_microdegrees) / MICRODEGREES_PER_DEGREE
            * std::f64::consts::PI
            / 180.0,
        f64::from(first.site_latitude_microdegrees) / MICRODEGREES_PER_DEGREE
            * std::f64::consts::PI
            / 180.0,
    );
    let second_site = Vec3::from_lon_lat(
        f64::from(second.site_longitude_microdegrees) / MICRODEGREES_PER_DEGREE
            * std::f64::consts::PI
            / 180.0,
        f64::from(second.site_latitude_microdegrees) / MICRODEGREES_PER_DEGREE
            * std::f64::consts::PI
            / 180.0,
    );
    classify_boundary_at(
        first,
        second,
        first_site.add(second_site).normalize(),
        first_site,
        second_site,
    )
}

fn build_boundaries(
    grid: Grid,
    plates: &[Plate],
    plate_by_cell: &[u16],
    progress: &mut dyn ProgressSink,
) -> Result<Vec<BoundarySegment>, PhysicalError> {
    let mut boundaries = Vec::new();
    let mut seen = HashSet::new();
    let topology = grid.topology();
    for first_cell in 0..grid.sample_count() {
        if first_cell.is_multiple_of(256) {
            progress.check_cancelled()?;
        }
        for second_cell in topology.neighbors(first_cell) {
            if plate_by_cell[first_cell] == plate_by_cell[*second_cell] {
                continue;
            }
            let (left, right) = boundary_key(first_cell, *second_cell);
            if !seen.insert((left, right)) {
                continue;
            }
            let first_plate = plate_by_cell[left];
            let second_plate = plate_by_cell[right];
            let midpoint = cell_midpoint(grid, left, right);
            let (kind, relative_speed) = classify_boundary_at(
                &plates[usize::from(first_plate)],
                &plates[usize::from(second_plate)],
                midpoint,
                cell_vector(grid, left),
                cell_vector(grid, right),
            );
            boundaries.push(BoundarySegment {
                first_cell: left,
                second_cell: right,
                first_plate,
                second_plate,
                kind,
                relative_speed_nanoradians_per_year: relative_speed,
            });
        }
    }
    Ok(boundaries)
}

fn spreading_ridge_elevation(strength: f64, age_steps: u32) -> f64 {
    strength * RIDGE_RELIEF_FRACTION / (1.0 + f64::from(age_steps))
}

fn rift_shoulder_elevation(strength: f64, age_steps: u32) -> f64 {
    strength * RIFT_SHOULDER_FRACTION / (1.0 + f64::from(age_steps))
}

fn geodesic_distance_mm(grid: Grid, sources: &[usize], max_mm: i64) -> Vec<i64> {
    geodesic_nearest(grid, sources, max_mm).0
}

pub(crate) fn geodesic_nearest(grid: Grid, sources: &[usize], max_mm: i64) -> (Vec<i64>, Vec<u32>) {
    let mut dist = vec![i64::MAX; grid.sample_count()];
    let mut nearest = vec![u32::MAX; grid.sample_count()];
    let mut heap = BinaryHeap::new();
    for (index, &source) in sources.iter().enumerate() {
        if source >= dist.len() {
            continue;
        }
        heap.push(Reverse((0_i64, source, index as u32)));
    }
    let topology = grid.topology();
    while let Some(Reverse((cost, cell, source))) = heap.pop() {
        if cost > max_mm {
            continue;
        }
        if cost >= dist[cell] {
            continue;
        }
        dist[cell] = cost;
        nearest[cell] = source;
        for neighbor in topology.neighbors(cell) {
            let next = cost.saturating_add(edge_length_mm(grid, cell, *neighbor));
            if next < dist[*neighbor] && next <= max_mm {
                heap.push(Reverse((next, *neighbor, source)));
            }
        }
    }
    (dist, nearest)
}

fn motion_scale(relative_speed: u64) -> f64 {
    (relative_speed as f64 / MOTION_SPEED_REF_NANORADIANS).clamp(0.40, 2.20)
}

fn plate_abs_speed(plates: &[Plate], id: u16) -> u64 {
    plates
        .get(usize::from(id))
        .map(|plate| plate.angular_speed_nanoradians_per_year.unsigned_abs())
        .unwrap_or(0)
}

fn collision_side_bias(own_speed: u64, other_speed: u64) -> f64 {
    match own_speed.cmp(&other_speed) {
        std::cmp::Ordering::Less => 1.28,
        std::cmp::Ordering::Greater => 0.72,
        std::cmp::Ordering::Equal => 1.0,
    }
}

struct ScaledSource {
    cell: usize,
    speed: u64,
    bias: f64,
}

impl ScaledSource {
    fn new(cell: usize, speed: u64) -> Self {
        Self {
            cell,
            speed,
            bias: 1.0,
        }
    }

    fn biased(cell: usize, speed: u64, bias: f64) -> Self {
        Self { cell, speed, bias }
    }
}

#[allow(clippy::too_many_arguments)]
fn apply_motion_kernel(
    field: &mut [f64],
    dist: &[i64],
    nearest: &[u32],
    sources: &[ScaledSource],
    mask: impl Fn(usize) -> bool,
    base_amplitude: f64,
    base_radius_mm: i64,
    tectonic_activity: f64,
) {
    for (cell, distance) in dist.iter().copied().enumerate() {
        if !mask(cell) {
            continue;
        }
        let index = nearest[cell];
        if index == u32::MAX || (index as usize) >= sources.len() {
            continue;
        }
        let scale = motion_scale(sources[index as usize].speed) * sources[index as usize].bias;
        let radius =
            ((base_radius_mm as f64) * (0.78 + 0.22 * scale.clamp(0.40, 2.20))).round() as i64;
        let weight = bell(distance, radius.max(1));
        if weight > 0.0 {
            field[cell] += base_amplitude * tectonic_activity * scale * weight;
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn apply_motion_ring_kernel(
    field: &mut [f64],
    dist: &[i64],
    nearest: &[u32],
    sources: &[ScaledSource],
    mask: impl Fn(usize) -> bool,
    base_amplitude: f64,
    offset_mm: i64,
    width_mm: i64,
    tectonic_activity: f64,
) {
    for (cell, distance) in dist.iter().copied().enumerate() {
        if !mask(cell) {
            continue;
        }
        let index = nearest[cell];
        if index == u32::MAX || (index as usize) >= sources.len() {
            continue;
        }
        let scale = motion_scale(sources[index as usize].speed) * sources[index as usize].bias;
        let width = ((width_mm as f64) * (0.78 + 0.22 * scale.clamp(0.40, 2.20))).round() as i64;
        let weight = ring(distance, offset_mm, width.max(1));
        if weight > 0.0 {
            field[cell] += base_amplitude * tectonic_activity * scale * weight;
        }
    }
}

fn bell(distance_mm: i64, radius_mm: i64) -> f64 {
    if distance_mm == i64::MAX || radius_mm <= 0 {
        0.0
    } else {
        let t = distance_mm as f64 / radius_mm as f64;
        if t >= 1.0 {
            0.0
        } else {
            let fall = 1.0 - t * t;
            fall * fall
        }
    }
}

fn ring(distance_mm: i64, offset_mm: i64, width_mm: i64) -> f64 {
    if distance_mm == i64::MAX {
        0.0
    } else {
        bell((distance_mm - offset_mm).abs(), width_mm)
    }
}

fn metres_to_mm(metres: u64) -> i64 {
    (metres as i64).saturating_mul(1_000).max(1)
}

fn build_named_relief(
    inputs: ReliefInputs<'_>,
) -> Result<(NamedRelief, Vec<VolcanicCenter>), PhysicalError> {
    let ReliefInputs {
        grid,
        identity,
        settings,
        plates,
        plate_by_cell,
        crust_by_cell,
        boundaries,
        progress,
    } = inputs;
    let tectonic_activity = f64::from(settings.tectonic_activity_ppm) / 1_000_000.0;
    let relief_stage_seed = derive_subsystem_seed(
        identity.seed,
        identity.retry_index,
        SeedDomain::ReliefDetail,
    );
    let hotspot_stage_seed =
        derive_subsystem_seed(identity.seed, identity.retry_index, SeedDomain::Hotspots);
    let sample_count = grid.sample_count();
    let mut relief = NamedRelief::new(sample_count);
    let land_cells = crust_by_cell
        .iter()
        .enumerate()
        .filter(|(_, crust)| **crust == CrustType::Continental)
        .map(|(cell, _)| cell)
        .collect::<Vec<_>>();
    let ocean_cells = crust_by_cell
        .iter()
        .enumerate()
        .filter(|(_, crust)| **crust == CrustType::Oceanic)
        .map(|(cell, _)| cell)
        .collect::<Vec<_>>();
    progress.check_cancelled()?;
    let interior_mm = metres_to_mm(3_500_000);
    let interior_dist = geodesic_distance_mm(grid, &ocean_cells, interior_mm);
    let shelf_mm = metres_to_mm(resolution::FEATURE_FIXTURES.shelf_width_metres);
    let shelf_dist = geodesic_distance_mm(grid, &land_cells, shelf_mm);
    let max_interior = interior_dist
        .iter()
        .copied()
        .filter(|value| *value != i64::MAX)
        .max()
        .unwrap_or(1)
        .max(1);
    for cell in 0..sample_count {
        if cell.is_multiple_of(256) {
            progress.check_cancelled()?;
        }
        let restrained =
            (splitmix64(relief_stage_seed ^ (cell as u64).wrapping_mul(0x9e37_79b9_7f4a_7c15))
                % 1_000_001) as f64
                - 500_000.0;
        let baseline = match crust_by_cell[cell] {
            CrustType::Continental => {
                let inland = if interior_dist[cell] == i64::MAX {
                    1.0
                } else {
                    interior_dist[cell] as f64 / max_interior as f64
                };
                350_000.0 + 1_250_000.0 * inland.powf(1.25)
            }
            CrustType::Oceanic => -3_200_000.0 + 1_400_000.0 * bell(shelf_dist[cell], shelf_mm),
        };
        relief.add(ReliefTerm::Baseline, cell, baseline);
        relief.add(ReliefTerm::Detail, cell, restrained * 0.22);
    }

    let mut collision_sources = Vec::new();
    let mut foreland_sources = Vec::new();
    for boundary in boundaries.iter().filter(|boundary| {
        boundary.kind == BoundaryKind::Convergent
            && crust_by_cell[boundary.first_cell] == CrustType::Continental
            && crust_by_cell[boundary.second_cell] == CrustType::Continental
    }) {
        let first_speed = plate_abs_speed(plates, boundary.first_plate);
        let second_speed = plate_abs_speed(plates, boundary.second_plate);
        collision_sources.push(ScaledSource::biased(
            boundary.first_cell,
            boundary.relative_speed_nanoradians_per_year,
            collision_side_bias(first_speed, second_speed),
        ));
        collision_sources.push(ScaledSource::biased(
            boundary.second_cell,
            boundary.relative_speed_nanoradians_per_year,
            collision_side_bias(second_speed, first_speed),
        ));
        let faster = if first_speed >= second_speed {
            boundary.first_cell
        } else {
            boundary.second_cell
        };
        foreland_sources.push(ScaledSource::new(
            faster,
            boundary.relative_speed_nanoradians_per_year,
        ));
    }
    let collision_mm = metres_to_mm(resolution::FEATURE_FIXTURES.collision_belt_width_metres);
    let collision_cells = collision_sources
        .iter()
        .map(|source| source.cell)
        .collect::<Vec<_>>();
    let (collision_dist, collision_nearest) = geodesic_nearest(
        grid,
        &collision_cells,
        ((collision_mm as f64) * 2.2).round() as i64,
    );
    apply_motion_kernel(
        relief.term_mut(ReliefTerm::Collision),
        &collision_dist,
        &collision_nearest,
        &collision_sources,
        |_| true,
        1_650_000.0,
        collision_mm,
        tectonic_activity,
    );
    let foreland_cells = foreland_sources
        .iter()
        .map(|source| source.cell)
        .collect::<Vec<_>>();
    let (foreland_dist, foreland_nearest) = geodesic_nearest(
        grid,
        &foreland_cells,
        ((collision_mm as f64) * 1.6).round() as i64,
    );
    apply_motion_kernel(
        relief.term_mut(ReliefTerm::Collision),
        &foreland_dist,
        &foreland_nearest,
        &foreland_sources,
        |cell| crust_by_cell[cell] == CrustType::Continental,
        -480_000.0,
        collision_mm.saturating_mul(4) / 5,
        tectonic_activity,
    );

    let rift_sources = boundaries
        .iter()
        .filter(|boundary| {
            boundary.kind == BoundaryKind::Divergent
                && crust_by_cell[boundary.first_cell] == CrustType::Continental
                && crust_by_cell[boundary.second_cell] == CrustType::Continental
        })
        .flat_map(|boundary| {
            [
                ScaledSource::new(
                    boundary.first_cell,
                    boundary.relative_speed_nanoradians_per_year,
                ),
                ScaledSource::new(
                    boundary.second_cell,
                    boundary.relative_speed_nanoradians_per_year,
                ),
            ]
        })
        .collect::<Vec<_>>();
    let rift_floor_mm = metres_to_mm(resolution::FEATURE_FIXTURES.rift_floor_width_metres);
    let rift_shoulder_mm = metres_to_mm(resolution::FEATURE_FIXTURES.rift_shoulder_width_metres);
    let rift_cells = rift_sources
        .iter()
        .map(|source| source.cell)
        .collect::<Vec<_>>();
    let (rift_dist, rift_nearest) = geodesic_nearest(
        grid,
        &rift_cells,
        ((rift_shoulder_mm as f64) * 2.2).round() as i64,
    );
    apply_motion_kernel(
        relief.term_mut(ReliefTerm::RiftFloor),
        &rift_dist,
        &rift_nearest,
        &rift_sources,
        |cell| crust_by_cell[cell] == CrustType::Continental,
        -1_850_000.0,
        rift_floor_mm,
        tectonic_activity,
    );
    apply_motion_ring_kernel(
        relief.term_mut(ReliefTerm::RiftShoulders),
        &rift_dist,
        &rift_nearest,
        &rift_sources,
        |cell| crust_by_cell[cell] == CrustType::Continental,
        rift_shoulder_elevation(1_900_000.0, 1),
        rift_floor_mm,
        rift_shoulder_mm.saturating_sub(rift_floor_mm).max(1),
        tectonic_activity,
    );

    let mut trench_sources = Vec::new();
    let mut inland_arc_sources = Vec::new();
    let mut oceanic_arc_sources = Vec::new();
    let mut ridge_sources = Vec::new();
    let mut transform_sources = Vec::new();
    for boundary in boundaries {
        let first_crust = crust_by_cell[boundary.first_cell];
        let second_crust = crust_by_cell[boundary.second_cell];
        match boundary.kind {
            BoundaryKind::Convergent if first_crust != second_crust => {
                let (ocean_cell, continent_cell) = if first_crust == CrustType::Oceanic {
                    (boundary.first_cell, boundary.second_cell)
                } else {
                    (boundary.second_cell, boundary.first_cell)
                };
                trench_sources.push(ScaledSource::new(
                    ocean_cell,
                    boundary.relative_speed_nanoradians_per_year,
                ));
                inland_arc_sources.push(ScaledSource::new(
                    continent_cell,
                    boundary.relative_speed_nanoradians_per_year,
                ));
            }
            BoundaryKind::Convergent
                if first_crust == CrustType::Oceanic && second_crust == CrustType::Oceanic =>
            {
                let first_speed = plate_abs_speed(plates, boundary.first_plate);
                let second_speed = plate_abs_speed(plates, boundary.second_plate);
                let (down_cell, up_cell) = if first_speed > second_speed
                    || (first_speed == second_speed && boundary.first_plate > boundary.second_plate)
                {
                    (boundary.first_cell, boundary.second_cell)
                } else {
                    (boundary.second_cell, boundary.first_cell)
                };
                trench_sources.push(ScaledSource::new(
                    down_cell,
                    boundary.relative_speed_nanoradians_per_year,
                ));
                oceanic_arc_sources.push(ScaledSource::new(
                    up_cell,
                    boundary.relative_speed_nanoradians_per_year,
                ));
            }
            BoundaryKind::Divergent
                if first_crust == CrustType::Oceanic && second_crust == CrustType::Oceanic =>
            {
                ridge_sources.push(ScaledSource::new(
                    boundary.first_cell,
                    boundary.relative_speed_nanoradians_per_year,
                ));
                ridge_sources.push(ScaledSource::new(
                    boundary.second_cell,
                    boundary.relative_speed_nanoradians_per_year,
                ));
            }
            BoundaryKind::Transform => {
                transform_sources.push(boundary.first_cell);
                transform_sources.push(boundary.second_cell);
            }
            _ => {}
        }
    }
    let trench_mm = metres_to_mm(resolution::FEATURE_FIXTURES.trench_half_width_metres);
    let arc_offset_mm = metres_to_mm(resolution::FEATURE_FIXTURES.trench_arc_offset_metres);
    let trench_cells = trench_sources
        .iter()
        .map(|source| source.cell)
        .collect::<Vec<_>>();
    let inland_cells = inland_arc_sources
        .iter()
        .map(|source| source.cell)
        .collect::<Vec<_>>();
    let oceanic_cells = oceanic_arc_sources
        .iter()
        .map(|source| source.cell)
        .collect::<Vec<_>>();
    let (trench_dist, trench_nearest) = geodesic_nearest(
        grid,
        &trench_cells,
        trench_mm.saturating_mul(2).saturating_mul(3) / 2,
    );
    let (inland_dist, inland_nearest) = geodesic_nearest(
        grid,
        &inland_cells,
        arc_offset_mm.saturating_mul(2).saturating_mul(3) / 2,
    );
    let (oceanic_arc_dist, oceanic_nearest) = geodesic_nearest(
        grid,
        &oceanic_cells,
        arc_offset_mm.saturating_mul(2).saturating_mul(3) / 2,
    );
    apply_motion_kernel(
        relief.term_mut(ReliefTerm::Trench),
        &trench_dist,
        &trench_nearest,
        &trench_sources,
        |cell| crust_by_cell[cell] == CrustType::Oceanic,
        -1_400_000.0,
        trench_mm,
        tectonic_activity,
    );
    apply_motion_ring_kernel(
        relief.term_mut(ReliefTerm::InlandArc),
        &inland_dist,
        &inland_nearest,
        &inland_arc_sources,
        |cell| crust_by_cell[cell] == CrustType::Continental,
        1_350_000.0,
        arc_offset_mm,
        trench_mm,
        tectonic_activity,
    );
    apply_motion_ring_kernel(
        relief.term_mut(ReliefTerm::OceanicArc),
        &oceanic_arc_dist,
        &oceanic_nearest,
        &oceanic_arc_sources,
        |cell| crust_by_cell[cell] == CrustType::Oceanic,
        2_050_000.0,
        arc_offset_mm / 2,
        trench_mm,
        tectonic_activity,
    );
    let ridge_mm = metres_to_mm(3_000_000);
    let ridge_cells = ridge_sources
        .iter()
        .map(|source| source.cell)
        .collect::<Vec<_>>();
    let (ridge_dist, ridge_nearest) = geodesic_nearest(grid, &ridge_cells, ridge_mm);
    {
        let ridge = relief.term_mut(ReliefTerm::Ridge);
        for (cell, distance) in ridge_dist.iter().copied().enumerate() {
            if crust_by_cell[cell] != CrustType::Oceanic || distance == i64::MAX {
                continue;
            }
            let index = ridge_nearest[cell];
            let scale = if index == u32::MAX || (index as usize) >= ridge_sources.len() {
                1.0
            } else {
                motion_scale(ridge_sources[index as usize].speed)
            };
            let age_steps = (distance as f64 / metres_to_mm(400_000) as f64).floor() as u32;
            ridge[cell] +=
                spreading_ridge_elevation(1_200_000.0 * tectonic_activity * scale, age_steps)
                    * bell(distance, ridge_mm);
        }
    }
    let transform_mm = metres_to_mm(resolution::FEATURE_FIXTURES.collision_belt_width_metres / 3);
    let transform_dist = geodesic_distance_mm(grid, &transform_sources, transform_mm);
    {
        let transform = relief.term_mut(ReliefTerm::Transform);
        for (cell, distance) in transform_dist.iter().copied().enumerate() {
            let weight = bell(distance, transform_mm);
            if weight <= 0.0 {
                continue;
            }
            let sign = if splitmix64(relief_stage_seed ^ 0x7777 ^ cell as u64).is_multiple_of(2) {
                1.0
            } else {
                -1.0
            };
            transform[cell] += 180_000.0 * tectonic_activity * weight * sign;
        }
    }

    progress.check_cancelled()?;
    let mut volcanic_centers = Vec::new();
    place_hotspot_chains(
        grid,
        plates,
        plate_by_cell,
        hotspot_stage_seed,
        settings,
        relief.term_mut(ReliefTerm::Hotspot),
        &mut volcanic_centers,
    );
    for (index, boundary) in boundaries.iter().enumerate() {
        if boundary.kind != BoundaryKind::Convergent
            || index % 4 != 0
            || volcanic_centers.len() >= MAX_VOLCANIC_CENTERS
        {
            continue;
        }
        let first_crust = crust_by_cell[boundary.first_cell];
        let second_crust = crust_by_cell[boundary.second_cell];
        let cell = if first_crust != second_crust {
            let continent_cell = if first_crust == CrustType::Continental {
                boundary.first_cell
            } else {
                boundary.second_cell
            };
            nearest_offset_cell(grid, continent_cell, inland_dist.as_slice(), arc_offset_mm)
        } else if first_crust == CrustType::Oceanic {
            nearest_offset_cell(
                grid,
                boundary.first_cell,
                oceanic_arc_dist.as_slice(),
                arc_offset_mm / 2,
            )
        } else {
            continue;
        };
        relief.add(ReliefTerm::InlandArc, cell, 1_100_000.0 * tectonic_activity);
        volcanic_centers.push(VolcanicCenter {
            cell,
            plate: plate_by_cell[cell],
            kind: VolcanicCenterKind::SubductionArc,
            intensity_ppm: 350_000,
        });
    }
    Ok((relief, volcanic_centers))
}

fn nearest_offset_cell(grid: Grid, origin: usize, dist: &[i64], target_mm: i64) -> usize {
    grid.topology()
        .neighbors(origin)
        .iter()
        .copied()
        .chain(std::iter::once(origin))
        .min_by_key(|cell| (dist.get(*cell).copied().unwrap_or(i64::MAX) - target_mm).abs())
        .unwrap_or(origin)
}

fn place_hotspot_chains(
    grid: Grid,
    plates: &[Plate],
    plate_by_cell: &[u16],
    hotspot_stage_seed: u64,
    settings: TectonicSettings,
    hotspot: &mut [f64],
    volcanic_centers: &mut Vec<VolcanicCenter>,
) {
    let plume_count = (2 + settings.island_activity_ppm * 5 / 1_000_000).clamp(2, 8) as usize;
    for plume in 0..plume_count {
        if volcanic_centers.len() >= MAX_VOLCANIC_CENTERS {
            break;
        }
        let pick =
            splitmix64(hotspot_stage_seed ^ (plume as u64 + 1).wrapping_mul(0xa076_1d64_78bd_642f));
        let start = (pick as usize) % grid.sample_count();
        let plate = &plates[usize::from(plate_by_cell[start])];
        let start_vec = cell_vector(grid, start);
        let axis = plate_axis(plate);
        let chain_len = HOTSPOT_CHAIN_MIN
            + (splitmix64(pick) as usize % (HOTSPOT_CHAIN_MAX - HOTSPOT_CHAIN_MIN + 1));
        let base_intensity = 1_800_000.0 + (pick % 400_001) as f64;
        for age in 0..chain_len {
            if volcanic_centers.len() >= MAX_VOLCANIC_CENTERS {
                break;
            }
            let position = start_vec.rotate_around(axis, -HOTSPOT_STEP_RADIANS * age as f64);
            let cell = nearest_cell(grid, position);
            let decay = (chain_len - age) as f64 / chain_len as f64;
            let intensity = base_intensity * decay;
            hotspot[cell] += intensity;
            volcanic_centers.push(VolcanicCenter {
                cell,
                plate: plate_by_cell[cell],
                kind: VolcanicCenterKind::Hotspot,
                intensity_ppm: ((intensity / 10.0).round() as i64).clamp(1, 1_000_000) as u32,
            });
        }
    }
}

fn build_relief(
    inputs: ReliefInputs<'_>,
) -> Result<(Vec<i32>, Vec<VolcanicCenter>, NamedRelief), PhysicalError> {
    let (relief, volcanic_centers) = build_named_relief(inputs)?;
    Ok((relief.summed_mm(), volcanic_centers, relief))
}

impl TectonicWorld {
    pub fn validate(&self) -> Result<(), PhysicalError> {
        self.settings.validate()?;
        let sample_count = self.grid.sample_count();
        if !(1..1_000_000).contains(&self.target_land_fraction_ppm) {
            return Err(PhysicalError::Validation(
                "tectonic target land fraction is outside (0, 1)".into(),
            ));
        }
        if self.plate_by_cell.len() != sample_count
            || self.crust_by_cell.len() != sample_count
            || self.elevations_mm.len() != sample_count
        {
            return Err(PhysicalError::Validation(
                "tectonic field lengths do not match the grid".into(),
            ));
        }
        if self.plates.len() != self.settings.plate_count as usize {
            return Err(PhysicalError::Validation(
                "tectonic plate metadata does not match the settings".into(),
            ));
        }
        for (index, plate) in self.plates.iter().enumerate() {
            if usize::from(plate.id) != index
                || plate.seed_cell >= sample_count
                || !(-180_000_000..=180_000_000).contains(&plate.site_longitude_microdegrees)
                || !(-90_000_000..=90_000_000).contains(&plate.site_latitude_microdegrees)
                || !(-180_000_000..=180_000_000)
                    .contains(&plate.rotation_axis_longitude_microdegrees)
                || !(-90_000_000..=90_000_000).contains(&plate.rotation_axis_latitude_microdegrees)
                || !(1..=2_000_000).contains(&plate.angular_speed_nanoradians_per_year)
            {
                return Err(PhysicalError::Validation(
                    "plate metadata is invalid or out of bounds".into(),
                ));
            }
        }
        for plate_id in &self.plate_by_cell {
            self.plates.get(*plate_id as usize).ok_or_else(|| {
                PhysicalError::Validation("cell references an unknown plate".into())
            })?;
        }
        if self.boundaries.len() > MAX_BOUNDARIES
            || self.volcanic_centers.len() > MAX_VOLCANIC_CENTERS
        {
            return Err(PhysicalError::Validation(
                "tectonic metadata exceeds the source budget".into(),
            ));
        }
        let topology = self.grid.topology();
        let mut seen_boundaries = HashSet::with_capacity(self.boundaries.len());
        for first in &self.boundaries {
            if first.first_cell >= sample_count
                || first.second_cell >= sample_count
                || first.first_cell >= first.second_cell
                || first.first_plate == first.second_plate
                || usize::from(first.first_plate) >= self.plates.len()
                || usize::from(first.second_plate) >= self.plates.len()
                || self.plate_by_cell[first.first_cell] != first.first_plate
                || self.plate_by_cell[first.second_cell] != first.second_plate
                || !topology.are_neighbors(first.first_cell, first.second_cell)
            {
                return Err(PhysicalError::Validation(
                    "tectonic boundary has invalid topology".into(),
                ));
            }
            if !seen_boundaries.insert((first.first_cell, first.second_cell)) {
                return Err(PhysicalError::Validation(
                    "tectonic boundary is duplicated".into(),
                ));
            }
        }
        for center in &self.volcanic_centers {
            if center.cell >= sample_count
                || usize::from(center.plate) >= self.plates.len()
                || self.plate_by_cell[center.cell] != center.plate
                || center.intensity_ppm == 0
                || center.intensity_ppm > 1_000_000
            {
                return Err(PhysicalError::Validation(
                    "volcanic center metadata is invalid".into(),
                ));
            }
        }
        Ok(())
    }

    pub fn continental_cell_count(&self) -> usize {
        self.crust_by_cell
            .iter()
            .filter(|crust| **crust == CrustType::Continental)
            .count()
    }

    pub fn metrics(&self) -> Result<TectonicMetrics, PhysicalError> {
        self.validate()?;
        let total_area = self.grid.total_area();
        let mut plate_areas = vec![0.0; self.plates.len()];
        let mut continental_area = 0.0;
        let mut exposed_land_area = 0.0;
        let mut continental_exposed_land_area = 0.0;
        let mut shelf_area = 0.0;
        let mut boundary_cells = vec![false; self.grid.sample_count()];
        let mut convergent_cells = vec![false; self.grid.sample_count()];
        let mut divergent_cells = vec![false; self.grid.sample_count()];
        let mut convergent_boundary_count = 0u32;
        let mut divergent_boundary_count = 0u32;
        let mut transform_boundary_count = 0u32;

        for cell in 0..self.grid.sample_count() {
            let area = self.grid.cell_area(self.grid.row_col(cell).0);
            let plate = usize::from(self.plate_by_cell[cell]);
            plate_areas[plate] += area;
            if self.crust_by_cell[cell] == CrustType::Continental {
                continental_area += area;
                if self.elevations_mm[cell] <= self.sea_level_mm {
                    shelf_area += area;
                } else {
                    continental_exposed_land_area += area;
                }
            }
            if self.elevations_mm[cell] > self.sea_level_mm {
                exposed_land_area += area;
            }
        }
        for boundary in &self.boundaries {
            boundary_cells[boundary.first_cell] = true;
            boundary_cells[boundary.second_cell] = true;
            match boundary.kind {
                BoundaryKind::Convergent => {
                    convergent_cells[boundary.first_cell] = true;
                    convergent_cells[boundary.second_cell] = true;
                    convergent_boundary_count += 1;
                }
                BoundaryKind::Divergent => {
                    divergent_cells[boundary.first_cell] = true;
                    divergent_cells[boundary.second_cell] = true;
                    divergent_boundary_count += 1;
                }
                BoundaryKind::Transform => transform_boundary_count += 1,
            }
        }
        let mut plate_area_ppm = plate_areas
            .into_iter()
            .map(|area| area_fraction_ppm(area, total_area))
            .collect::<Vec<_>>();
        plate_area_ppm.sort_unstable();
        let mut elevations = self
            .elevations_mm
            .iter()
            .enumerate()
            .map(|(cell, elevation)| (*elevation, self.grid.cell_area(self.grid.row_col(cell).0)))
            .collect::<Vec<_>>();
        elevations.sort_unstable_by_key(|(elevation, _)| *elevation);
        let (island_component_count, largest_island_component_cells) = self.land_components();

        Ok(TectonicMetrics {
            plate_area_p05_ppm: percentile_u32(&plate_area_ppm, 50_000),
            plate_area_median_ppm: percentile_u32(&plate_area_ppm, 500_000),
            plate_area_p95_ppm: percentile_u32(&plate_area_ppm, 950_000),
            boundary_coverage_ppm: area_fraction_ppm(
                masked_area(self.grid, &boundary_cells),
                total_area,
            ),
            convergent_boundary_count,
            divergent_boundary_count,
            transform_boundary_count,
            continental_crust_area_ppm: area_fraction_ppm(continental_area, total_area),
            exposed_land_area_ppm: area_fraction_ppm(exposed_land_area, total_area),
            continental_exposed_land_area_ppm: area_fraction_ppm(
                continental_exposed_land_area,
                total_area,
            ),
            continental_shelf_area_ppm: area_fraction_ppm(shelf_area, total_area),
            elevation_p05_mm: weighted_elevation_percentile(&elevations, 50_000),
            elevation_median_mm: weighted_elevation_percentile(&elevations, 500_000),
            elevation_p95_mm: weighted_elevation_percentile(&elevations, 950_000),
            trench_ridge_separation_metres: minimum_distance_between_masks(
                self.grid,
                &convergent_cells,
                &divergent_cells,
            ),
            volcanic_arc_offset_metres: volcanic_arc_offset(self),
            island_component_count: island_component_count as u32,
            largest_island_component_cells: largest_island_component_cells as u32,
        })
    }

    fn land_components(&self) -> (usize, usize) {
        let mut visited = vec![false; self.grid.sample_count()];
        let mut component_count = 0;
        let mut largest_component = 0;
        let topology = self.grid.topology();
        for start in 0..self.grid.sample_count() {
            if visited[start] || self.elevations_mm[start] <= self.sea_level_mm {
                continue;
            }
            component_count += 1;
            let mut stack = vec![start];
            visited[start] = true;
            let mut component_size = 0;
            while let Some(cell) = stack.pop() {
                component_size += 1;
                for neighbor in topology.neighbors(cell) {
                    if !visited[*neighbor] && self.elevations_mm[*neighbor] > self.sea_level_mm {
                        visited[*neighbor] = true;
                        stack.push(*neighbor);
                    }
                }
            }
            largest_component = largest_component.max(component_size);
        }
        (component_count, largest_component)
    }
}

fn area_fraction_ppm(area: f64, total_area: f64) -> u32 {
    (area / total_area * 1_000_000.0)
        .round()
        .clamp(0.0, 1_000_000.0) as u32
}

fn masked_area(grid: Grid, mask: &[bool]) -> f64 {
    mask.iter()
        .enumerate()
        .filter(|(_, included)| **included)
        .map(|(cell, _)| grid.cell_area(grid.row_col(cell).0))
        .sum()
}

fn percentile_u32(sorted: &[u32], percentile_ppm: u32) -> u32 {
    if sorted.is_empty() {
        return 0;
    }
    let index =
        (((sorted.len() - 1) as u64 * u64::from(percentile_ppm) + 500_000) / 1_000_000) as usize;
    sorted[index.min(sorted.len() - 1)]
}

fn weighted_elevation_percentile(sorted: &[(i32, f64)], percentile_ppm: u32) -> i32 {
    let total_area = sorted.iter().map(|(_, area)| *area).sum::<f64>();
    let target_area = total_area * f64::from(percentile_ppm) / 1_000_000.0;
    let mut accumulated = 0.0;
    sorted
        .iter()
        .find_map(|(elevation, area)| {
            accumulated += *area;
            (accumulated >= target_area).then_some(*elevation)
        })
        .or_else(|| sorted.last().map(|(elevation, _)| *elevation))
        .unwrap_or(0)
}

fn cell_vector_masks(grid: Grid, mask: &[bool]) -> Vec<Vec3> {
    mask.iter()
        .enumerate()
        .filter(|(_, included)| **included)
        .map(|(cell, _)| cell_vector(grid, cell))
        .collect()
}

fn minimum_distance_between_masks(grid: Grid, first: &[bool], second: &[bool]) -> u64 {
    let first_vectors = cell_vector_masks(grid, first);
    let second_vectors = cell_vector_masks(grid, second);
    if first_vectors.is_empty() || second_vectors.is_empty() {
        return 0;
    }
    let mut minimum_chord_squared = f64::INFINITY;
    for first in &first_vectors {
        for second in &second_vectors {
            let difference = first.subtract(*second);
            let chord_squared = difference.dot(difference);
            if chord_squared > 1e-12 {
                minimum_chord_squared = minimum_chord_squared.min(chord_squared);
            }
        }
    }
    if !minimum_chord_squared.is_finite() {
        return 0;
    }
    let angle = 2.0 * (minimum_chord_squared.sqrt() / 2.0).clamp(0.0, 1.0).asin();
    (angle * grid.radius_metres as f64).round() as u64
}

fn volcanic_arc_offset(world: &TectonicWorld) -> u64 {
    let convergent_cells = world
        .boundaries
        .iter()
        .filter(|boundary| boundary.kind == BoundaryKind::Convergent)
        .flat_map(|boundary| [boundary.first_cell, boundary.second_cell])
        .collect::<Vec<_>>();
    let arc_centers = world
        .volcanic_centers
        .iter()
        .filter(|center| center.kind == VolcanicCenterKind::SubductionArc)
        .map(|center| center.cell)
        .collect::<Vec<_>>();
    if convergent_cells.is_empty() || arc_centers.is_empty() {
        return 0;
    }
    let mut total = 0.0;
    for center in arc_centers {
        let center_vector = cell_vector(world.grid, center);
        let nearest = convergent_cells
            .iter()
            .map(|cell| {
                let difference = center_vector.subtract(cell_vector(world.grid, *cell));
                difference.dot(difference)
            })
            .fold(f64::INFINITY, f64::min);
        total +=
            2.0 * (nearest.sqrt() / 2.0).clamp(0.0, 1.0).asin() * world.grid.radius_metres as f64;
    }
    (total
        / world
            .volcanic_centers
            .iter()
            .filter(|center| center.kind == VolcanicCenterKind::SubductionArc)
            .count() as f64)
        .round() as u64
}

fn crust_byte(crust: CrustType) -> u8 {
    match crust {
        CrustType::Continental => 0,
        CrustType::Oceanic => 1,
    }
}

fn crust_from_byte(value: u8) -> Result<CrustType, String> {
    match value {
        0 => Ok(CrustType::Continental),
        1 => Ok(CrustType::Oceanic),
        _ => Err("tectonic source contains an unsupported crust type".into()),
    }
}

fn boundary_byte(kind: BoundaryKind) -> u8 {
    match kind {
        BoundaryKind::Convergent => 0,
        BoundaryKind::Divergent => 1,
        BoundaryKind::Transform => 2,
    }
}

fn boundary_from_byte(value: u8) -> Result<BoundaryKind, String> {
    match value {
        0 => Ok(BoundaryKind::Convergent),
        1 => Ok(BoundaryKind::Divergent),
        2 => Ok(BoundaryKind::Transform),
        _ => Err("tectonic source contains an unsupported boundary kind".into()),
    }
}

fn volcanic_byte(kind: VolcanicCenterKind) -> u8 {
    match kind {
        VolcanicCenterKind::Hotspot => 0,
        VolcanicCenterKind::SubductionArc => 1,
    }
}

fn volcanic_from_byte(value: u8) -> Result<VolcanicCenterKind, String> {
    match value {
        0 => Ok(VolcanicCenterKind::Hotspot),
        1 => Ok(VolcanicCenterKind::SubductionArc),
        _ => Err("tectonic source contains an unsupported volcanic center kind".into()),
    }
}

fn source_payload_bytes(
    sample_count: usize,
    plate_count: usize,
    boundary_count: usize,
    volcanic_count: usize,
) -> Result<usize, String> {
    let sample_bytes = sample_count
        .checked_mul(7)
        .ok_or_else(|| "tectonic source size overflow".to_string())?;
    let plate_bytes = plate_count
        .checked_mul(31)
        .ok_or_else(|| "tectonic source size overflow".to_string())?;
    let boundary_bytes = boundary_count
        .checked_mul(21)
        .ok_or_else(|| "tectonic source size overflow".to_string())?;
    let volcanic_bytes = volcanic_count
        .checked_mul(11)
        .ok_or_else(|| "tectonic source size overflow".to_string())?;
    TECTONIC_SOURCE_HEADER_BYTES
        .checked_add(sample_bytes)
        .and_then(|size| size.checked_add(plate_bytes))
        .and_then(|size| size.checked_add(boundary_bytes))
        .and_then(|size| size.checked_add(volcanic_bytes))
        .ok_or_else(|| "tectonic source size overflow".to_string())
}

/// Encode the tectonic result as the active explicitly versioned
/// `physical-world-v2` source.
pub fn encode_source_v2(world: &TectonicWorld) -> Result<Vec<u8>, String> {
    world
        .validate()
        .map_err(|error| format!("cannot encode invalid tectonic world: {error}"))?;
    let sample_count = world.grid.sample_count();
    let expected_len = source_payload_bytes(
        sample_count,
        world.plates.len(),
        world.boundaries.len(),
        world.volcanic_centers.len(),
    )?;
    if expected_len > TECTONIC_MAX_SOURCE_BYTES {
        return Err(format!(
            "tectonic source exceeds the {} byte source budget",
            TECTONIC_MAX_SOURCE_BYTES
        ));
    }
    let mut output = Vec::with_capacity(expected_len);
    output.extend_from_slice(&super::SOURCE_MAGIC);
    super::push_u16(&mut output, TECTONIC_SOURCE_VERSION);
    super::push_u16(&mut output, TECTONIC_SOURCE_HEADER_BYTES as u16);
    super::push_u32(&mut output, world.grid.width);
    super::push_u32(&mut output, world.grid.height);
    super::push_u64(&mut output, world.grid.radius_metres);
    super::push_u32(&mut output, world.seed);
    super::push_u32(&mut output, world.retry_index);
    super::push_u32(&mut output, world.target_land_fraction_ppm);
    super::push_i32(&mut output, world.sea_level_mm);
    super::push_u32(&mut output, sample_count as u32);
    super::push_u16(&mut output, world.settings.plate_count);
    super::push_u16(&mut output, world.settings.continental_plate_count);
    super::push_u32(&mut output, world.settings.tectonic_activity_ppm);
    super::push_u32(&mut output, world.settings.island_activity_ppm);
    super::push_u32(&mut output, world.boundaries.len() as u32);
    super::push_u32(&mut output, world.volcanic_centers.len() as u32);
    for elevation in &world.elevations_mm {
        super::push_i32(&mut output, *elevation);
    }
    for plate in &world.plate_by_cell {
        super::push_u16(&mut output, *plate);
    }
    for crust in &world.crust_by_cell {
        output.push(crust_byte(*crust));
    }
    for plate in &world.plates {
        super::push_u16(&mut output, plate.id);
        super::push_u32(&mut output, plate.seed_cell as u32);
        super::push_i32(&mut output, plate.site_longitude_microdegrees);
        super::push_i32(&mut output, plate.site_latitude_microdegrees);
        super::push_i32(&mut output, plate.rotation_axis_longitude_microdegrees);
        super::push_i32(&mut output, plate.rotation_axis_latitude_microdegrees);
        super::push_u64(&mut output, plate.angular_speed_nanoradians_per_year as u64);
        output.push(crust_byte(plate.crust_type));
    }
    for boundary in &world.boundaries {
        super::push_u32(&mut output, boundary.first_cell as u32);
        super::push_u32(&mut output, boundary.second_cell as u32);
        super::push_u16(&mut output, boundary.first_plate);
        super::push_u16(&mut output, boundary.second_plate);
        output.push(boundary_byte(boundary.kind));
        super::push_u64(&mut output, boundary.relative_speed_nanoradians_per_year);
    }
    for center in &world.volcanic_centers {
        super::push_u32(&mut output, center.cell as u32);
        super::push_u16(&mut output, center.plate);
        output.push(volcanic_byte(center.kind));
        super::push_u32(&mut output, center.intensity_ppm);
    }
    debug_assert_eq!(output.len(), expected_len);
    Ok(output)
}

/// Decode a strict `physical-world-v2` source. Older source versions are not
/// accepted by the hard-cut provider contract.
pub fn decode_source_v2(bytes: &[u8]) -> Result<TectonicWorld, String> {
    if bytes.len() < TECTONIC_SOURCE_HEADER_BYTES {
        return Err("tectonic source is shorter than its fixed header".into());
    }
    let mut offset = 0;
    if super::read_exact::<8>(bytes, &mut offset)? != super::SOURCE_MAGIC {
        return Err("tectonic source magic does not match DAENAPW1".into());
    }
    let version = u16::from_le_bytes(super::read_exact(bytes, &mut offset)?);
    if version != TECTONIC_SOURCE_VERSION {
        return Err("tectonic source version is unsupported".into());
    }
    let header_bytes = u16::from_le_bytes(super::read_exact(bytes, &mut offset)?);
    if header_bytes as usize != TECTONIC_SOURCE_HEADER_BYTES {
        return Err("tectonic source header length is unsupported".into());
    }
    let width = u32::from_le_bytes(super::read_exact(bytes, &mut offset)?);
    let height = u32::from_le_bytes(super::read_exact(bytes, &mut offset)?);
    let radius_metres = u64::from_le_bytes(super::read_exact(bytes, &mut offset)?);
    let grid = Grid::new(width, height, radius_metres)?;
    let seed = u32::from_le_bytes(super::read_exact(bytes, &mut offset)?);
    let retry_index = u32::from_le_bytes(super::read_exact(bytes, &mut offset)?);
    let target_land_fraction_ppm = u32::from_le_bytes(super::read_exact(bytes, &mut offset)?);
    let sea_level_mm = i32::from_le_bytes(super::read_exact(bytes, &mut offset)?);
    let sample_count = u32::from_le_bytes(super::read_exact(bytes, &mut offset)?) as usize;
    if sample_count != grid.sample_count() {
        return Err("tectonic source sample count does not match the grid".into());
    }
    let plate_count = u16::from_le_bytes(super::read_exact(bytes, &mut offset)?);
    let continental_plate_count = u16::from_le_bytes(super::read_exact(bytes, &mut offset)?);
    let tectonic_activity_ppm = u32::from_le_bytes(super::read_exact(bytes, &mut offset)?);
    let island_activity_ppm = u32::from_le_bytes(super::read_exact(bytes, &mut offset)?);
    let boundary_count = u32::from_le_bytes(super::read_exact(bytes, &mut offset)?) as usize;
    let volcanic_count = u32::from_le_bytes(super::read_exact(bytes, &mut offset)?) as usize;
    let settings = TectonicSettings {
        plate_count,
        continental_plate_count,
        tectonic_activity_ppm,
        island_activity_ppm,
    };
    settings.validate().map_err(|error| error.to_string())?;
    if boundary_count > MAX_BOUNDARIES || volcanic_count > MAX_VOLCANIC_CENTERS {
        return Err("tectonic source metadata exceeds its bounded counts".into());
    }
    let expected_len = source_payload_bytes(
        sample_count,
        plate_count as usize,
        boundary_count,
        volcanic_count,
    )?;
    if expected_len > TECTONIC_MAX_SOURCE_BYTES || bytes.len() != expected_len {
        return Err("tectonic source has trailing, missing, or over-budget bytes".into());
    }
    let mut elevations_mm = Vec::with_capacity(sample_count);
    for _ in 0..sample_count {
        elevations_mm.push(i32::from_le_bytes(super::read_exact(bytes, &mut offset)?));
    }
    let mut plate_by_cell = Vec::with_capacity(sample_count);
    for _ in 0..sample_count {
        plate_by_cell.push(u16::from_le_bytes(super::read_exact(bytes, &mut offset)?));
    }
    let mut crust_by_cell = Vec::with_capacity(sample_count);
    for _ in 0..sample_count {
        crust_by_cell.push(crust_from_byte(
            *super::read_exact::<1>(bytes, &mut offset)?.first().unwrap(),
        )?);
    }
    let mut plates = Vec::with_capacity(plate_count as usize);
    for _ in 0..plate_count {
        let id = u16::from_le_bytes(super::read_exact(bytes, &mut offset)?);
        let seed_cell = u32::from_le_bytes(super::read_exact(bytes, &mut offset)?) as usize;
        let site_longitude_microdegrees =
            i32::from_le_bytes(super::read_exact(bytes, &mut offset)?);
        let site_latitude_microdegrees = i32::from_le_bytes(super::read_exact(bytes, &mut offset)?);
        let rotation_axis_longitude_microdegrees =
            i32::from_le_bytes(super::read_exact(bytes, &mut offset)?);
        let rotation_axis_latitude_microdegrees =
            i32::from_le_bytes(super::read_exact(bytes, &mut offset)?);
        let angular_speed_nanoradians_per_year =
            u64::from_le_bytes(super::read_exact(bytes, &mut offset)?) as i64;
        let crust_type =
            crust_from_byte(*super::read_exact::<1>(bytes, &mut offset)?.first().unwrap())?;
        plates.push(Plate {
            id,
            seed_cell,
            site_longitude_microdegrees,
            site_latitude_microdegrees,
            rotation_axis_longitude_microdegrees,
            rotation_axis_latitude_microdegrees,
            angular_speed_nanoradians_per_year,
            crust_type,
        });
    }
    let mut boundaries = Vec::with_capacity(boundary_count);
    for _ in 0..boundary_count {
        let first_cell = u32::from_le_bytes(super::read_exact(bytes, &mut offset)?) as usize;
        let second_cell = u32::from_le_bytes(super::read_exact(bytes, &mut offset)?) as usize;
        let first_plate = u16::from_le_bytes(super::read_exact(bytes, &mut offset)?);
        let second_plate = u16::from_le_bytes(super::read_exact(bytes, &mut offset)?);
        let kind =
            boundary_from_byte(*super::read_exact::<1>(bytes, &mut offset)?.first().unwrap())?;
        let relative_speed_nanoradians_per_year =
            u64::from_le_bytes(super::read_exact(bytes, &mut offset)?);
        boundaries.push(BoundarySegment {
            first_cell,
            second_cell,
            first_plate,
            second_plate,
            kind,
            relative_speed_nanoradians_per_year,
        });
    }
    let mut volcanic_centers = Vec::with_capacity(volcanic_count);
    for _ in 0..volcanic_count {
        let cell = u32::from_le_bytes(super::read_exact(bytes, &mut offset)?) as usize;
        let plate = u16::from_le_bytes(super::read_exact(bytes, &mut offset)?);
        let kind =
            volcanic_from_byte(*super::read_exact::<1>(bytes, &mut offset)?.first().unwrap())?;
        let intensity_ppm = u32::from_le_bytes(super::read_exact(bytes, &mut offset)?);
        volcanic_centers.push(VolcanicCenter {
            cell,
            plate,
            kind,
            intensity_ppm,
        });
    }
    let world = TectonicWorld {
        grid,
        seed,
        retry_index,
        target_land_fraction_ppm,
        sea_level_mm,
        settings,
        plates,
        plate_by_cell,
        crust_by_cell,
        boundaries,
        volcanic_centers,
        elevations_mm,
    };
    world
        .validate()
        .map_err(|error| format!("decoded tectonic source is invalid: {error}"))?;
    Ok(world)
}

pub fn generate_tectonic_source(
    grid: Grid,
    settings: TectonicSettings,
    target_land_fraction_ppm: u32,
    seed: u32,
    retry_index: u32,
    progress: &mut dyn ProgressSink,
) -> Result<(TectonicWorld, Vec<u8>), PhysicalError> {
    let mut world = generate_tectonic_world(
        grid,
        settings,
        target_land_fraction_ppm,
        seed,
        retry_index,
        progress,
    )?;
    world.sea_level_mm =
        super::solve_sea_level(&grid, &world.elevations_mm, target_land_fraction_ppm)
            .map_err(PhysicalError::Validation)?;
    let source = encode_source_v2(&world).map_err(PhysicalError::InvalidSource)?;
    Ok((world, source))
}

fn center_microdegrees(grid: Grid, cell: usize) -> [i32; 2] {
    let (row, col) = grid.row_col(cell);
    [
        (-180_000_000i64 + 360_000_000i64 * (i64::from(col) * 2 + 1) / (i64::from(grid.width) * 2))
            as i32,
        (-90_000_000i64 + 180_000_000i64 * (i64::from(row) * 2 + 1) / (i64::from(grid.height) * 2))
            as i32,
    ]
}

fn write_cell_polygon(output: &mut String, grid: Grid, cell: usize) {
    let (row, col) = grid.row_col(cell);
    let west = super::longitude_micro(&grid, col);
    let east = super::longitude_micro(&grid, col + 1);
    let south = super::latitude_micro(&grid, row);
    let north = super::latitude_micro(&grid, row + 1);
    output.push_str("[[[");
    super::write_micro(output, west);
    output.push(',');
    super::write_micro(output, south);
    output.push_str("],[");
    super::write_micro(output, east);
    output.push(',');
    super::write_micro(output, south);
    output.push_str("],[");
    super::write_micro(output, east);
    output.push(',');
    super::write_micro(output, north);
    output.push_str("],[");
    super::write_micro(output, west);
    output.push(',');
    super::write_micro(output, north);
    output.push_str("],[");
    super::write_micro(output, west);
    output.push(',');
    super::write_micro(output, south);
    output.push_str("]]]");
}

fn point_geometry(point: [i32; 2]) -> String {
    format!(
        "[{},{}]",
        super::format_micro(point[0]),
        super::format_micro(point[1])
    )
}

fn line_geometry(first: [i32; 2], second: [i32; 2]) -> String {
    format!(
        "[[{},{}],[{},{}]]",
        super::format_micro(first[0]),
        super::format_micro(first[1]),
        super::format_micro(second[0]),
        super::format_micro(second[1])
    )
}

fn path_geometry(path: &[[i32; 2]]) -> String {
    let mut output = String::from("[");
    for (index, point) in path.iter().enumerate() {
        if index > 0 {
            output.push(',');
        }
        output.push_str(&format!(
            "[{},{}]",
            super::format_micro(point[0]),
            super::format_micro(point[1])
        ));
    }
    output.push(']');
    output
}

fn diagnostic_feature(
    id: &str,
    layer: &str,
    kind: &str,
    name: &str,
    geometry_type: &str,
    geometry: &str,
) -> String {
    format!(
        "{{\"type\":\"Feature\",\"id\":\"{id}\",\"properties\":{{\"daenaLayerId\":\"{layer}\",\"kind\":\"{kind}\",\"name\":\"{name}\"}},\"geometry\":{{\"type\":\"{geometry_type}\",\"coordinates\":{geometry}}}}}"
    )
}

fn add_diagnostic_feature(features: &mut DiagnosticWriter, feature: String) -> Result<(), String> {
    features.push(&feature)
}

struct DiagnosticWriter {
    output: String,
    count: usize,
}

impl DiagnosticWriter {
    fn new() -> Self {
        let mut output = String::with_capacity(8 * 1024 * 1024);
        output.push_str(r#"{"type":"FeatureCollection","features":["#);
        Self { output, count: 0 }
    }

    fn push(&mut self, feature: &str) -> Result<(), String> {
        if self.count >= super::MAX_GEOJSON_FEATURES {
            return Err("physical diagnostic output exceeds the feature budget".into());
        }
        if self.count > 0 {
            self.output.push(',');
        }
        self.output.push_str(feature);
        self.count += 1;
        Ok(())
    }

    fn push_polygon_feature(
        &mut self,
        id: &str,
        layer: &str,
        kind: &str,
        name: &str,
        grid: Grid,
        cell: usize,
    ) -> Result<(), String> {
        if self.count >= super::MAX_GEOJSON_FEATURES {
            return Err("physical diagnostic output exceeds the feature budget".into());
        }
        if self.count > 0 {
            self.output.push(',');
        }
        write!(
            self.output,
            r#"{{"type":"Feature","id":"{id}","properties":{{"daenaLayerId":"{layer}","kind":"{kind}","name":"{name}"}},"geometry":{{"type":"Polygon","coordinates":"#
        )
        .map_err(|_| "failed to format derived GeoJSON".to_string())?;
        write_cell_polygon(&mut self.output, grid, cell);
        self.output.push_str("}}");
        self.count += 1;
        Ok(())
    }

    fn finish(mut self) -> Result<String, String> {
        self.output.push_str("]}");
        if self.output.len() > crate::MAX_DERIVED_GEOJSON_BYTES {
            return Err(format!(
                "physical diagnostic output exceeds the {} byte budget",
                crate::MAX_DERIVED_GEOJSON_BYTES
            ));
        }
        Ok(self.output)
    }
}

fn add_boundary_features(
    features: &mut DiagnosticWriter,
    world: &TectonicWorld,
    boundary_index: usize,
    boundary: BoundarySegment,
) -> Result<(), String> {
    let first = center_microdegrees(world.grid, boundary.first_cell);
    let second = center_microdegrees(world.grid, boundary.second_cell);
    let name = match boundary.kind {
        BoundaryKind::Convergent => "Convergent boundary",
        BoundaryKind::Divergent => "Divergent boundary",
        BoundaryKind::Transform => "Transform boundary",
    };
    if (first[0] - second[0]).unsigned_abs() <= 180_000_000 {
        add_diagnostic_feature(
            features,
            diagnostic_feature(
                &format!("boundary-{boundary_index:06}"),
                "tectonic-boundaries",
                "route",
                name,
                "LineString",
                &line_geometry(first, second),
            ),
        )?;
    } else {
        let seam = if first[0] < 0 {
            -180_000_000
        } else {
            180_000_000
        };
        let opposite = -seam;
        let midpoint_latitude = (i64::from(first[1]) + i64::from(second[1])) / 2;
        add_diagnostic_feature(
            features,
            diagnostic_feature(
                &format!("boundary-{boundary_index:06}-a"),
                "tectonic-boundaries",
                "route",
                name,
                "LineString",
                &line_geometry(first, [seam, midpoint_latitude as i32]),
            ),
        )?;
        add_diagnostic_feature(
            features,
            diagnostic_feature(
                &format!("boundary-{boundary_index:06}-b"),
                "tectonic-boundaries",
                "route",
                name,
                "LineString",
                &line_geometry([opposite, midpoint_latitude as i32], second),
            ),
        )?;
    }
    Ok(())
}

fn diagnostic_cell_stride(grid: Grid) -> u32 {
    crate::derived_cell_stride(grid)
}

/// Emit all current tectonic diagnostics as derived, read-only vector layers.
/// The canonical source remains the v2 binary; this output is disposable and
/// can be regenerated from it after restart or cache deletion.
pub fn to_diagnostic_geojson(world: &TectonicWorld) -> Result<String, String> {
    world
        .validate()
        .map_err(|error| format!("invalid tectonic diagnostics: {error}"))?;
    let field = world.physical_field();
    let mut features = DiagnosticWriter::new();
    for (index, path) in super::coastline_paths(&field)
        .map_err(|error| error.to_string())?
        .iter()
        .enumerate()
    {
        if path.len() < 2 {
            continue;
        }
        add_diagnostic_feature(
            &mut features,
            diagnostic_feature(
                &format!("coastline-{index:05}"),
                "base",
                "custom",
                "physical coastline",
                "LineString",
                &path_geometry(path),
            ),
        )?;
    }
    let stride = diagnostic_cell_stride(world.grid);
    for row in (0..world.grid.height).step_by(stride as usize) {
        for col in (0..world.grid.width).step_by(stride as usize) {
            let cell = world.grid.index(row, col);
            let plate = world.plate_by_cell[cell];
            features.push_polygon_feature(
                &format!("plate-cell-{cell:05}"),
                "tectonic-plates",
                "region",
                &format!("Plate {plate} (LOD {stride})"),
                world.grid,
                cell,
            )?;
            let elevation = world.elevations_mm[cell];
            features.push_polygon_feature(
                &format!("bathymetry-cell-{cell:05}"),
                "bathymetry",
                "custom",
                &format!("Elevation {elevation} mm (LOD {stride})"),
                world.grid,
                cell,
            )?;
        }
    }
    for (index, boundary) in world.boundaries.iter().copied().enumerate() {
        add_boundary_features(&mut features, world, index, boundary)?;
    }
    for (index, center) in world.volcanic_centers.iter().enumerate() {
        let name = match center.kind {
            VolcanicCenterKind::Hotspot => "Hotspot",
            VolcanicCenterKind::SubductionArc => "Subduction arc",
        };
        add_diagnostic_feature(
            &mut features,
            diagnostic_feature(
                &format!("volcano-{index:05}"),
                "volcanic-centers",
                "marker",
                name,
                "Point",
                &point_geometry(center_microdegrees(world.grid, center.cell)),
            ),
        )?;
    }
    features.finish()
}

pub fn generate_tectonic_world(
    grid: Grid,
    settings: TectonicSettings,
    target_land_fraction_ppm: u32,
    seed: u32,
    retry_index: u32,
    progress: &mut dyn ProgressSink,
) -> Result<TectonicWorld, PhysicalError> {
    settings.validate()?;
    let mut settings = settings;
    if !(1..1_000_000).contains(&target_land_fraction_ppm) {
        return Err(PhysicalError::InvalidSettings(
            "target land fraction must be between zero and one".into(),
        ));
    }
    progress.report(ProgressPhase::BuildingTectonicStructure, 0, 3)?;
    let identity = GenerationIdentity { seed, retry_index };
    let sites = (0..settings.plate_count)
        .map(|id| plate_site(grid, seed, retry_index, id, settings.plate_count))
        .collect::<Vec<_>>();
    let mut plates = build_plates(grid, seed, retry_index, settings);
    let mut plate_by_cell = assign_plates(grid, &sites, identity, progress)?;
    plates = diversify_plates(grid, plates, &mut plate_by_cell, identity, progress)?;
    settings.plate_count = plates.len() as u16;
    if settings.continental_plate_count > settings.plate_count {
        settings.continental_plate_count = settings.plate_count.max(1);
    }
    let layout = continent_layout(seed, retry_index, settings);
    let stage_seed = derive_subsystem_seed(seed, retry_index, SeedDomain::ContinentalCratons);
    let mut cratons = craton_seeds(&layout, stage_seed);
    bind_cratons_to_plates(&mut cratons, grid, &plate_by_cell);
    let (crust_by_cell, _group_by_cell) = grow_continental_crust(
        grid,
        &cratons,
        &layout,
        &plates,
        &plate_by_cell,
        identity,
        progress,
    )?;
    assign_majority_crust(&mut plates, &plate_by_cell, &crust_by_cell);
    progress.report(ProgressPhase::BuildingTectonicStructure, 1, 3)?;
    let boundaries = build_boundaries(grid, &plates, &plate_by_cell, progress)?;
    progress.report(ProgressPhase::BuildingTectonicStructure, 2, 3)?;
    let (elevations_mm, volcanic_centers, _named_relief) = build_relief(ReliefInputs {
        grid,
        identity,
        settings,
        plates: &plates,
        plate_by_cell: &plate_by_cell,
        crust_by_cell: &crust_by_cell,
        boundaries: &boundaries,
        progress,
    })?;
    progress.report(ProgressPhase::BuildingTectonicStructure, 3, 3)?;
    progress.report(ProgressPhase::BuildingTerrain, 1, 1)?;
    let world = TectonicWorld {
        grid,
        seed,
        retry_index,
        target_land_fraction_ppm,
        sea_level_mm: 0,
        settings,
        plates,
        plate_by_cell,
        crust_by_cell,
        boundaries,
        volcanic_centers,
        elevations_mm,
    };
    world.validate()?;
    Ok(world)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        coastline_segments, solve_sea_level, Grid, NoopProgress, DEFAULT_HEIGHT,
        DEFAULT_RADIUS_METRES, DEFAULT_WIDTH,
    };
    use std::collections::HashMap;

    fn fixture() -> TectonicWorld {
        let grid = Grid::new(DEFAULT_WIDTH, DEFAULT_HEIGHT, DEFAULT_RADIUS_METRES).unwrap();
        let mut progress = NoopProgress;
        generate_tectonic_world(
            grid,
            TectonicSettings::default_for(grid),
            300_000,
            831_429,
            0,
            &mut progress,
        )
        .unwrap()
    }

    #[test]
    fn every_cell_has_one_plate_and_matching_crust() {
        let world = fixture();
        assert_eq!(world.plate_by_cell.len(), world.grid.sample_count());
        assert!(world
            .plate_by_cell
            .iter()
            .all(|plate| { usize::from(*plate) < world.plates.len() }));
        assert_eq!(
            world.continental_cell_count()
                + world
                    .crust_by_cell
                    .iter()
                    .filter(|crust| **crust == CrustType::Oceanic)
                    .count(),
            world.grid.sample_count()
        );
        assert!(world
            .plates
            .iter()
            .all(|plate| { world.plate_by_cell.contains(&plate.id) }));
        assert!((0..world.plates.len()).any(|plate_id| {
            let mut has_continental = false;
            let mut has_oceanic = false;
            for (cell, assigned) in world.plate_by_cell.iter().enumerate() {
                if usize::from(*assigned) != plate_id {
                    continue;
                }
                match world.crust_by_cell[cell] {
                    CrustType::Continental => has_continental = true,
                    CrustType::Oceanic => has_oceanic = true,
                }
            }
            has_continental && has_oceanic
        }));
    }

    #[test]
    fn boundaries_are_unique_wrapped_and_classified() {
        let world = fixture();
        assert!(!world.boundaries.is_empty());
        assert!(world.boundaries.iter().all(|boundary| {
            boundary.first_cell < boundary.second_cell
                && boundary.first_plate != boundary.second_plate
                && boundary.relative_speed_nanoradians_per_year > 0
        }));
        assert!(world
            .boundaries
            .iter()
            .any(|boundary| { boundary.kind == BoundaryKind::Convergent }));
        assert!(world
            .boundaries
            .iter()
            .any(|boundary| { boundary.kind == BoundaryKind::Divergent }));
        assert!(world
            .boundaries
            .iter()
            .any(|boundary| { boundary.kind == BoundaryKind::Transform }));
        for boundary in &world.boundaries {
            assert!(world
                .grid
                .neighbors(boundary.first_cell)
                .contains(&boundary.second_cell));
            assert!(world
                .grid
                .neighbors(boundary.second_cell)
                .contains(&boundary.first_cell));
        }
    }

    #[test]
    fn classification_is_reversal_invariant_at_the_versioned_threshold() {
        let world = fixture();
        let boundary = world.boundaries[0];
        let first = world.plates[usize::from(boundary.first_plate)];
        let second = world.plates[usize::from(boundary.second_plate)];
        assert_eq!(
            classify_boundary(&first, &second),
            classify_boundary(&second, &first)
        );
        let midpoint = cell_midpoint(world.grid, boundary.first_cell, boundary.second_cell);
        let first_point = cell_vector(world.grid, boundary.first_cell);
        let second_point = cell_vector(world.grid, boundary.second_cell);
        assert_eq!(
            classify_boundary_at(&first, &second, midpoint, first_point, second_point),
            classify_boundary_at(&second, &first, midpoint, second_point, first_point)
        );
        assert_eq!(RELATIVE_MOTION_THRESHOLD_NANORADIANS_PER_YEAR, 25_000.0);
        assert_eq!(boundary_key(9, 3), boundary_key(3, 9));
    }

    #[test]
    fn seam_and_pole_fixtures_have_complete_ownership_and_closed_topology() {
        let world = fixture();
        assert_eq!(world.plate_by_cell.len(), world.grid.sample_count());
        assert!(world
            .plate_by_cell
            .iter()
            .all(|plate| usize::from(*plate) < world.plates.len()));

        let seam_boundary = (0..world.grid.height).any(|row| {
            let first = world.grid.index(row, 0);
            let second = world.grid.index(row, world.grid.width - 1);
            world.boundaries.iter().any(|boundary| {
                (boundary.first_cell == first && boundary.second_cell == second)
                    || (boundary.first_cell == second && boundary.second_cell == first)
            })
        });
        assert!(
            seam_boundary,
            "fixture must contain a boundary across +/-180 degrees"
        );

        let polar_grid = Grid::new(DEFAULT_WIDTH, 2, DEFAULT_RADIUS_METRES).unwrap();
        let mut progress = NoopProgress;
        let polar_world = generate_tectonic_world(
            polar_grid,
            TectonicSettings::default_for(polar_grid),
            300_000,
            0,
            0,
            &mut progress,
        )
        .unwrap();
        assert_eq!(
            polar_world.plate_by_cell.len(),
            polar_world.grid.sample_count()
        );
        assert!(polar_world
            .plate_by_cell
            .iter()
            .all(|plate| usize::from(*plate) < polar_world.plates.len()));
        let polar_boundary = (0..polar_world.grid.width).any(|column| {
            let first = polar_world.grid.index(0, 0);
            let second = polar_world.grid.index(0, column);
            first != second
                && polar_world.boundaries.iter().any(|boundary| {
                    (boundary.first_cell == first && boundary.second_cell == second)
                        || (boundary.first_cell == second && boundary.second_cell == first)
                })
        });
        assert!(
            polar_boundary,
            "fixture must contain a boundary through the polar row"
        );
        for boundary in &polar_world.boundaries {
            assert!(polar_world
                .grid
                .neighbors(boundary.first_cell)
                .contains(&boundary.second_cell));
            assert!(polar_world
                .grid
                .neighbors(boundary.second_cell)
                .contains(&boundary.first_cell));
        }

        let mut solved = polar_world;
        solved.sea_level_mm = solve_sea_level(
            &solved.grid,
            &solved.elevations_mm,
            solved.target_land_fraction_ppm,
        )
        .unwrap();
        let polar_land = (0..solved.grid.width)
            .map(|column| solved.grid.index(0, column))
            .map(|cell| solved.elevations_mm[cell] > solved.sea_level_mm)
            .collect::<Vec<_>>();
        let segments = coastline_segments(&solved.physical_field());
        assert!(segments.iter().all(|segment| {
            [segment.first, segment.second].iter().all(|point| {
                (-180_000_000..=180_000_000).contains(&point[0])
                    && (-90_000_000..=90_000_000).contains(&point[1])
            })
        }));
        if polar_land.iter().any(|land| *land) && polar_land.iter().any(|land| !*land) {
            let south_center =
                (-90_000_000i64 + 180_000_000i64 / i64::from(solved.grid.height * 2)) as i32;
            assert!(
                segments.iter().any(|segment| {
                    segment.first[1] <= south_center || segment.second[1] <= south_center
                }),
                "mixed polar land must produce a polar-triangle coastline"
            );
        }
    }

    #[test]
    fn relief_has_causal_positive_and_negative_terrain() {
        let world = fixture();
        let minimum = *world.elevations_mm.iter().min().unwrap();
        let maximum = *world.elevations_mm.iter().max().unwrap();
        assert!(minimum < 0);
        assert!(maximum > 0);
        let continental = world
            .crust_by_cell
            .iter()
            .zip(&world.elevations_mm)
            .filter(|(crust, _)| **crust == CrustType::Continental)
            .map(|(_, elevation)| i64::from(*elevation))
            .sum::<i64>();
        let oceanic = world
            .crust_by_cell
            .iter()
            .zip(&world.elevations_mm)
            .filter(|(crust, _)| **crust == CrustType::Oceanic)
            .map(|(_, elevation)| i64::from(*elevation))
            .sum::<i64>();
        assert!(continental > oceanic);
        assert!(!world.volcanic_centers.is_empty());
    }

    #[test]
    fn divergent_relief_terms_have_bounded_causal_profiles() {
        let strength = 1_000_000.0;
        assert!(rift_shoulder_elevation(strength, 1) > 0.0);
        assert!(spreading_ridge_elevation(strength, 0) > spreading_ridge_elevation(strength, 1));
        assert!(spreading_ridge_elevation(strength, 1) > spreading_ridge_elevation(strength, 2));
        assert!(spreading_ridge_elevation(strength, 0) <= strength);
    }

    #[test]
    fn same_input_is_byte_stable_and_retry_changes_the_world() {
        let first = fixture();
        let settings = TectonicSettings::default_for(first.grid);
        let mut progress = NoopProgress;
        let retry = generate_tectonic_world(
            first.grid,
            settings,
            first.target_land_fraction_ppm,
            first.seed,
            first.retry_index + 1,
            &mut progress,
        )
        .unwrap();
        let mut progress = NoopProgress;
        let repeat = generate_tectonic_world(
            first.grid,
            settings,
            first.target_land_fraction_ppm,
            first.seed,
            first.retry_index,
            &mut progress,
        )
        .unwrap();
        assert_eq!(first, repeat);
        assert_ne!(first, retry);
    }

    #[test]
    fn diagnostic_geojson_contains_locked_read_only_layers_and_is_bounded() {
        let world = fixture();
        let geojson = to_diagnostic_geojson(&world).unwrap();
        assert!(geojson.starts_with(r#"{"type":"FeatureCollection","features":["#));
        for layer in [
            "\"daenaLayerId\":\"base\"",
            "\"daenaLayerId\":\"tectonic-plates\"",
            "\"daenaLayerId\":\"tectonic-boundaries\"",
            "\"daenaLayerId\":\"bathymetry\"",
            "\"daenaLayerId\":\"volcanic-centers\"",
        ] {
            assert!(geojson.contains(layer), "missing diagnostic layer: {layer}");
        }
        assert!(!geojson.contains("http://"));
        assert!(!geojson.contains("https://"));
        assert!(geojson.len() < crate::MAX_DERIVED_GEOJSON_BYTES);
    }

    #[test]
    fn metrics_cover_bounded_tectonic_fixture_invariants() {
        let mut world = fixture();
        world.sea_level_mm = solve_sea_level(
            &world.grid,
            &world.elevations_mm,
            world.target_land_fraction_ppm,
        )
        .unwrap();
        let metrics = world.metrics().unwrap();
        assert!(metrics.plate_area_p05_ppm > 0);
        assert!(metrics.plate_area_p05_ppm <= metrics.plate_area_median_ppm);
        assert!(metrics.plate_area_median_ppm <= metrics.plate_area_p95_ppm);
        assert!(metrics.plate_area_p95_ppm < 1_000_000);
        assert!(metrics.boundary_coverage_ppm > 0);
        assert_eq!(
            metrics.convergent_boundary_count
                + metrics.divergent_boundary_count
                + metrics.transform_boundary_count,
            world.boundaries.len() as u32
        );
        assert!(metrics.continental_crust_area_ppm > metrics.exposed_land_area_ppm);
        assert!(metrics.continental_shelf_area_ppm > 0);
        assert!(metrics.continental_exposed_land_area_ppm > 0);
        assert!(metrics.continental_crust_area_ppm > metrics.continental_shelf_area_ppm);
        assert!(metrics.elevation_p05_mm <= metrics.elevation_median_mm);
        assert!(metrics.elevation_median_mm <= metrics.elevation_p95_mm);
        assert!(metrics.trench_ridge_separation_metres > 0);
        assert!(metrics.volcanic_arc_offset_metres > 0);
        assert!(metrics.island_component_count > 0);
        assert!(metrics.largest_island_component_cells > 0);
        assert_eq!(metrics, world.metrics().unwrap());
    }

    fn named_relief_for(world: &TectonicWorld) -> NamedRelief {
        let mut progress = NoopProgress;
        build_relief(ReliefInputs {
            grid: world.grid,
            identity: GenerationIdentity {
                seed: world.seed,
                retry_index: world.retry_index,
            },
            settings: world.settings,
            plates: &world.plates,
            plate_by_cell: &world.plate_by_cell,
            crust_by_cell: &world.crust_by_cell,
            boundaries: &world.boundaries,
            progress: &mut progress,
        })
        .unwrap()
        .2
    }

    #[test]
    fn ownership_is_complete_and_adjacency_is_reciprocal() {
        let world = fixture();
        assert!(world
            .plate_by_cell
            .iter()
            .all(|plate| usize::from(*plate) < world.plates.len()));
        let mut adjacency: HashMap<(u16, u16), usize> = HashMap::new();
        for boundary in &world.boundaries {
            adjacency
                .entry((boundary.first_plate, boundary.second_plate))
                .and_modify(|count| *count += 1)
                .or_insert(1);
            adjacency
                .entry((boundary.second_plate, boundary.first_plate))
                .and_modify(|count| *count += 1)
                .or_insert(1);
        }
        for ((first, second), count) in &adjacency {
            assert_eq!(
                adjacency.get(&(*second, *first)),
                Some(count),
                "plate adjacency {first}->{second} is not reciprocal"
            );
        }
    }

    #[test]
    fn plate_boundaries_are_not_nearest_site_voronoi() {
        let world = fixture();
        let sites = world
            .plates
            .iter()
            .map(|plate| {
                Vec3::from_lon_lat(
                    f64::from(plate.site_longitude_microdegrees) / MICRODEGREES_PER_DEGREE
                        * std::f64::consts::PI
                        / 180.0,
                    f64::from(plate.site_latitude_microdegrees) / MICRODEGREES_PER_DEGREE
                        * std::f64::consts::PI
                        / 180.0,
                )
            })
            .collect::<Vec<_>>();
        let nearest = assign_nearest_plates(world.grid, &sites);
        let disagree = nearest
            .iter()
            .zip(&world.plate_by_cell)
            .filter(|(voronoi, assigned)| *voronoi != *assigned)
            .count();
        assert!(
            disagree * 20 > world.grid.sample_count(),
            "cost-field ownership must irregularize Voronoi sites, disagree={disagree}"
        );
    }

    #[test]
    fn plates_have_uneven_areas_after_merge_and_shatter() {
        let world = fixture();
        let metrics = world.metrics().unwrap();
        assert!(
            u64::from(metrics.plate_area_p95_ppm) * 1_000
                >= u64::from(metrics.plate_area_p05_ppm) * 2_000,
            "plate area p95={} should be at least 2x p05={}",
            metrics.plate_area_p95_ppm,
            metrics.plate_area_p05_ppm
        );
    }

    #[test]
    fn transform_slip_leaves_material_transform_boundaries() {
        let world = fixture();
        let metrics = world.metrics().unwrap();
        assert!(metrics.transform_boundary_count > 0);
        assert!(
            metrics.transform_boundary_count * 8
                >= metrics.convergent_boundary_count + metrics.divergent_boundary_count,
            "transform={} convergent={} divergent={}",
            metrics.transform_boundary_count,
            metrics.convergent_boundary_count,
            metrics.divergent_boundary_count
        );
    }

    #[test]
    fn plate_contacts_bend_instead_of_running_straight() {
        let world = fixture();
        let mut by_pair: HashMap<(u16, u16), Vec<(usize, usize)>> = HashMap::new();
        for boundary in &world.boundaries {
            by_pair
                .entry((boundary.first_plate, boundary.second_plate))
                .or_default()
                .push((boundary.first_cell, boundary.second_cell));
        }
        let mut bent = 0u32;
        let mut compared = 0u32;
        for segments in by_pair.values() {
            for (index, (first_a, second_a)) in segments.iter().enumerate() {
                let dir_a = cell_vector(world.grid, *second_a)
                    .subtract(cell_vector(world.grid, *first_a))
                    .reject(cell_midpoint(world.grid, *first_a, *second_a))
                    .normalize();
                for (first_b, second_b) in segments.iter().skip(index + 1) {
                    if *first_a != *first_b
                        && *first_a != *second_b
                        && *second_a != *first_b
                        && *second_a != *second_b
                    {
                        continue;
                    }
                    let dir_b = cell_vector(world.grid, *second_b)
                        .subtract(cell_vector(world.grid, *first_b))
                        .reject(cell_midpoint(world.grid, *first_b, *second_b))
                        .normalize();
                    compared += 1;
                    if dir_a.dot(dir_b).abs() < 0.92 {
                        bent += 1;
                    }
                }
            }
        }
        assert!(compared > 0, "plate contacts must have adjacent segments");
        assert!(
            bent * 4 >= compared,
            "plate contacts must change heading, bent={bent} compared={compared}"
        );
    }

    #[test]
    fn craton_groups_are_connected_and_terranes_are_bounded() {
        let world = fixture();
        let mut progress = NoopProgress;
        let layout = continent_layout(world.seed, world.retry_index, world.settings);
        let stage_seed = derive_subsystem_seed(
            world.seed,
            world.retry_index,
            SeedDomain::ContinentalCratons,
        );
        let mut cratons = craton_seeds(&layout, stage_seed);
        bind_cratons_to_plates(&mut cratons, world.grid, &world.plate_by_cell);
        let (crust, groups) = grow_continental_crust(
            world.grid,
            &cratons,
            &layout,
            &world.plates,
            &world.plate_by_cell,
            GenerationIdentity {
                seed: world.seed,
                retry_index: world.retry_index,
            },
            &mut progress,
        )
        .unwrap();
        assert_eq!(crust, world.crust_by_cell);
        let group_ids = groups
            .iter()
            .copied()
            .filter(|group| *group >= 0 && *group < 100)
            .collect::<HashSet<_>>();
        let topology = world.grid.topology();
        for group in group_ids {
            let cells = groups
                .iter()
                .enumerate()
                .filter(|(_, assigned)| **assigned == group)
                .map(|(cell, _)| cell)
                .collect::<Vec<_>>();
            assert!(!cells.is_empty());
            let mut seen = HashSet::new();
            let mut stack = vec![cells[0]];
            seen.insert(cells[0]);
            while let Some(cell) = stack.pop() {
                for neighbor in topology.neighbors(cell) {
                    if groups[*neighbor] == group && seen.insert(*neighbor) {
                        stack.push(*neighbor);
                    }
                }
            }
            assert_eq!(
                seen.len(),
                cells.len(),
                "craton group {group} must remain connected"
            );
        }
        let terrane_cells = groups.iter().filter(|group| **group >= 100).count();
        let budget = (u64::from(layout.terrane_budget_ppm) * world.grid.sample_count() as u64
            / 1_000_000) as usize
            + 8;
        assert!(
            terrane_cells <= budget,
            "detached terranes exceed the area budget: {terrane_cells} > {budget}"
        );
        let shelf = world
            .crust_by_cell
            .iter()
            .zip(&world.elevations_mm)
            .filter(|(crust, elevation)| **crust == CrustType::Continental && **elevation <= 0)
            .count();
        assert!(
            shelf > 0,
            "continental shelves must exist below sea level 0"
        );
    }

    #[test]
    fn named_relief_terms_account_for_final_elevation() {
        let world = fixture();
        let named = named_relief_for(&world);
        assert_eq!(named.summed_mm(), world.elevations_mm);
        let has_negative_trench = named.terms[ReliefTerm::Trench as usize]
            .iter()
            .any(|value| *value < 0.0);
        let has_positive_collision = named.terms[ReliefTerm::Collision as usize]
            .iter()
            .any(|value| *value > 0.0);
        assert!(has_negative_trench || has_positive_collision);
        assert!(named.terms[ReliefTerm::Detail as usize]
            .iter()
            .all(|value| value.abs() < 400_000.0));
    }

    #[test]
    fn tectonic_cross_sections_preserve_cause_signs() {
        let world = fixture();
        let named = named_relief_for(&world);
        let mut saw_collision = false;
        let mut saw_subduction = false;
        let mut saw_rift = false;
        let mut saw_ridge = false;
        let mut saw_transform = false;
        for boundary in &world.boundaries {
            let first = crust_of(&world, boundary.first_cell);
            let second = crust_of(&world, boundary.second_cell);
            match (boundary.kind, first, second) {
                (BoundaryKind::Convergent, CrustType::Continental, CrustType::Continental) => {
                    let uplift = named.terms[ReliefTerm::Collision as usize][boundary.first_cell]
                        + named.terms[ReliefTerm::Collision as usize][boundary.second_cell];
                    assert!(uplift > 0.0);
                    saw_collision = true;
                }
                (BoundaryKind::Convergent, CrustType::Oceanic, CrustType::Continental)
                | (BoundaryKind::Convergent, CrustType::Continental, CrustType::Oceanic) => {
                    let ocean = if first == CrustType::Oceanic {
                        boundary.first_cell
                    } else {
                        boundary.second_cell
                    };
                    let continent = if first == CrustType::Continental {
                        boundary.first_cell
                    } else {
                        boundary.second_cell
                    };
                    assert!(named.terms[ReliefTerm::Trench as usize][ocean] <= 0.0);
                    assert!(
                        named.terms[ReliefTerm::InlandArc as usize][continent] >= 0.0
                            || named.terms[ReliefTerm::Trench as usize][ocean] < 0.0
                    );
                    saw_subduction = true;
                }
                (BoundaryKind::Divergent, CrustType::Continental, CrustType::Continental) => {
                    assert!(
                        named.terms[ReliefTerm::RiftFloor as usize][boundary.first_cell] <= 0.0
                    );
                    saw_rift = true;
                }
                (BoundaryKind::Divergent, CrustType::Oceanic, CrustType::Oceanic) => {
                    assert!(named.terms[ReliefTerm::Ridge as usize][boundary.first_cell] >= 0.0);
                    saw_ridge = true;
                }
                (BoundaryKind::Transform, _, _) => {
                    let transform =
                        named.terms[ReliefTerm::Transform as usize][boundary.first_cell].abs();
                    assert!(
                        transform < 400_000.0,
                        "transform relief must stay a minor term, got {transform}"
                    );
                    saw_transform = true;
                }
                _ => {}
            }
        }
        assert!(saw_collision);
        assert!(saw_subduction);
        assert!(saw_rift);
        assert!(saw_ridge);
        assert!(saw_transform);
    }

    fn crust_of(world: &TectonicWorld, cell: usize) -> CrustType {
        world.crust_by_cell[cell]
    }

    #[test]
    fn hotspot_chains_age_along_plate_motion() {
        let world = fixture();
        let hotspots = world
            .volcanic_centers
            .iter()
            .filter(|center| center.kind == VolcanicCenterKind::Hotspot)
            .copied()
            .collect::<Vec<_>>();
        assert!(hotspots.len() >= HOTSPOT_CHAIN_MIN);
        let youngest = hotspots
            .iter()
            .max_by_key(|center| center.intensity_ppm)
            .unwrap();
        let plate = world.plates[usize::from(youngest.plate)];
        let start = cell_vector(world.grid, youngest.cell);
        let mut previous = youngest.intensity_ppm;
        let mut found_older = 0u32;
        for age in 1..=HOTSPOT_CHAIN_MAX {
            let older = start.rotate_around(plate_axis(&plate), -HOTSPOT_STEP_RADIANS * age as f64);
            let cell = nearest_cell(world.grid, older);
            if let Some(center) = hotspots.iter().find(|candidate| candidate.cell == cell) {
                assert!(center.intensity_ppm <= previous);
                previous = center.intensity_ppm;
                found_older += 1;
            }
        }
        assert!(
            found_older >= 1,
            "hotspot chain must place older centers backward along plate motion"
        );
    }

    fn world_for_retry(retry_index: u32) -> TectonicWorld {
        let grid = Grid::new(DEFAULT_WIDTH, DEFAULT_HEIGHT, DEFAULT_RADIUS_METRES).unwrap();
        let mut progress = NoopProgress;
        generate_tectonic_world(
            grid,
            TectonicSettings::default_for(grid),
            300_000,
            831_429,
            retry_index,
            &mut progress,
        )
        .unwrap()
    }

    fn crust_groups_for(world: &TectonicWorld) -> (ContinentLayout, Vec<i16>) {
        let layout = continent_layout(world.seed, world.retry_index, world.settings);
        let stage_seed = derive_subsystem_seed(
            world.seed,
            world.retry_index,
            SeedDomain::ContinentalCratons,
        );
        let mut cratons = craton_seeds(&layout, stage_seed);
        bind_cratons_to_plates(&mut cratons, world.grid, &world.plate_by_cell);
        let mut progress = NoopProgress;
        let (crust, groups) = grow_continental_crust(
            world.grid,
            &cratons,
            &layout,
            &world.plates,
            &world.plate_by_cell,
            GenerationIdentity {
                seed: world.seed,
                retry_index: world.retry_index,
            },
            &mut progress,
        )
        .unwrap();
        assert_eq!(crust, world.crust_by_cell);
        (layout, groups)
    }

    fn major_group_shares_ppm(groups: &[i16]) -> Vec<u32> {
        let mut counts = HashMap::new();
        let mut continental = 0u32;
        for group in groups {
            if *group < 0 || *group >= 100 {
                continue;
            }
            continental += 1;
            *counts.entry(*group).or_insert(0u32) += 1;
        }
        let mut shares = counts
            .into_values()
            .map(|count| (u64::from(count) * 1_000_000 / u64::from(continental.max(1))) as u32)
            .collect::<Vec<_>>();
        shares.sort_unstable_by(|left, right| right.cmp(left));
        shares
    }

    fn inferred_layout_kind(layout: &ContinentLayout) -> ContinentLayoutKind {
        if layout.group_count == 1 {
            ContinentLayoutKind::Mega
        } else if layout.group_share_ppm.first().copied().unwrap_or(0) >= 700_000 {
            ContinentLayoutKind::GiantScraps
        } else if layout.group_count == 2 {
            ContinentLayoutKind::Dual
        } else {
            ContinentLayoutKind::Scattered
        }
    }

    fn world_with_kind(kind: ContinentLayoutKind) -> TectonicWorld {
        let grid = Grid::new(DEFAULT_WIDTH, DEFAULT_HEIGHT, DEFAULT_RADIUS_METRES).unwrap();
        let settings = TectonicSettings::default_for(grid);
        for retry in 0..64 {
            if inferred_layout_kind(&continent_layout(831_429, retry, settings)) == kind {
                return world_for_retry(retry);
            }
        }
        panic!("no retry produced {kind:?} for the fixture seed");
    }

    #[test]
    fn continent_layout_draw_covers_mega_dual_and_scattered() {
        let grid = Grid::new(DEFAULT_WIDTH, DEFAULT_HEIGHT, DEFAULT_RADIUS_METRES).unwrap();
        let settings = TectonicSettings::default_for(grid);
        let kinds = (0..48)
            .map(|retry| inferred_layout_kind(&continent_layout(831_429, retry, settings)))
            .collect::<HashSet<_>>();
        assert!(kinds.contains(&ContinentLayoutKind::Mega));
        assert!(kinds.contains(&ContinentLayoutKind::GiantScraps));
        assert!(kinds.contains(&ContinentLayoutKind::Dual));
        assert!(kinds.contains(&ContinentLayoutKind::Scattered));
    }

    #[test]
    fn mega_layout_concentrates_one_continent() {
        let world = world_with_kind(ContinentLayoutKind::Mega);
        let (layout, groups) = crust_groups_for(&world);
        assert_eq!(inferred_layout_kind(&layout), ContinentLayoutKind::Mega);
        assert_eq!(layout.group_count, 1);
        let shares = major_group_shares_ppm(&groups);
        assert_eq!(shares.len(), 1);
        assert!(
            shares[0] >= 850_000,
            "mega layout must concentrate crust into one group, share={}",
            shares[0]
        );
    }

    #[test]
    fn dual_layout_keeps_two_major_continents() {
        let world = world_with_kind(ContinentLayoutKind::Dual);
        let (layout, groups) = crust_groups_for(&world);
        assert_eq!(inferred_layout_kind(&layout), ContinentLayoutKind::Dual);
        let shares = major_group_shares_ppm(&groups);
        assert_eq!(shares.len(), 2);
        assert!(
            shares[0] >= 280_000 && shares[1] >= 220_000,
            "dual layout needs two substantial continents, shares={shares:?}"
        );
        assert!(
            shares[0] <= 780_000,
            "dual layout must not collapse to a mega-continent, shares={shares:?}"
        );
    }

    #[test]
    fn scattered_layout_fragments_into_many_continents() {
        let world = world_with_kind(ContinentLayoutKind::Scattered);
        let (layout, groups) = crust_groups_for(&world);
        assert_eq!(
            inferred_layout_kind(&layout),
            ContinentLayoutKind::Scattered
        );
        assert!(layout.group_count >= 4);
        let shares = major_group_shares_ppm(&groups);
        assert!(
            shares.len() >= 4,
            "scattered layout must keep several continents, count={}",
            shares.len()
        );
        assert!(
            shares[0] <= 480_000,
            "scattered layout must not grow a mega-continent, largest={}",
            shares[0]
        );
    }

    #[test]
    fn motion_scale_is_monotonic_with_relative_speed() {
        assert!(motion_scale(200_000) < motion_scale(600_000));
        assert!(motion_scale(600_000) < motion_scale(1_200_000));
        assert!((motion_scale(80_000) - 0.40).abs() < 1e-9);
        assert!((motion_scale(5_000_000) - 2.20).abs() < 1e-9);
        assert!(collision_side_bias(200_000, 900_000) > collision_side_bias(900_000, 200_000));
    }

    #[test]
    fn giant_scraps_layout_keeps_one_dominant_continent() {
        let world = world_with_kind(ContinentLayoutKind::GiantScraps);
        let (layout, groups) = crust_groups_for(&world);
        assert_eq!(
            inferred_layout_kind(&layout),
            ContinentLayoutKind::GiantScraps
        );
        assert!(layout.group_count >= 2);
        let shares = major_group_shares_ppm(&groups);
        assert!(
            shares[0] >= 620_000,
            "giant+scraps must keep one huge landmass, shares={shares:?}"
        );
        assert!(
            shares.len() >= 2,
            "giant+scraps must retain scrap continents, shares={shares:?}"
        );
    }
}
