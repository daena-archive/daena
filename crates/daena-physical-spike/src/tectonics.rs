//! Deterministic tectonic scaffold for the native physical generator.
//!
//! This module owns the active v2 source codec. The source stores the signed
//! elevation field together with the physical causes required to derive later
//! terrain, climate, hydrology, and hazard products.

use super::{
    derive_subsystem_seed, splitmix64, Grid, PhysicalError, ProgressPhase, ProgressSink, SeedDomain,
};

const MICRODEGREES_PER_DEGREE: f64 = 1_000_000.0;
const MIN_PLATES: u16 = 4;
const MAX_PLATES: u16 = 64;
const MAX_VOLCANIC_CENTERS: usize = 256;
const MAX_BOUNDARIES: usize = 2_000_000;
pub const TECTONIC_SOURCE_VERSION: u16 = 2;
pub const TECTONIC_SOURCE_HEADER_BYTES: usize = 68;
pub const TECTONIC_MAX_SOURCE_BYTES: usize = 16 * 1024 * 1024;
/// Versioned v2 physical classification parameter. Keep this value stable
/// unless the generator version changes; it is intentionally not serialized
/// because the v2 header and payload are locked.
pub const RELATIVE_MOTION_THRESHOLD_NANORADIANS_PER_YEAR: f64 = 25_000.0;
const CRATON_RADIUS_RADIANS: f64 = 0.55;
const CRATON_SCORE_THRESHOLD: f64 = 0.10;
const CRATON_VARIATION_AMPLITUDE: f64 = 0.12;
const RIFT_SHOULDER_FRACTION: f64 = 0.18;
const RIDGE_RELIEF_FRACTION: f64 = 0.55;

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

#[derive(Debug, Clone, Copy)]
struct CratonSeed {
    center: Vec3,
    variation_axis: Vec3,
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
    plate_by_cell: &'a [u16],
    crust_by_cell: &'a [CrustType],
    boundaries: &'a [BoundarySegment],
    progress: &'a mut dyn ProgressSink,
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

fn craton_seeds(seed: u32, retry_index: u32, settings: TectonicSettings) -> Vec<CratonSeed> {
    let stage_seed = derive_subsystem_seed(seed, retry_index, SeedDomain::ContinentalCratons);
    let count = usize::from(settings.continental_plate_count).max(2);
    let golden_angle = std::f64::consts::PI * (3.0 - 5.0_f64.sqrt());
    (0..count)
        .map(|id| {
            let sequence = id as f64 + 0.5;
            let z = 1.0 - 2.0 * sequence / count as f64;
            let jitter = (splitmix64(stage_seed ^ (id as u64).wrapping_mul(0x9e37_79b9_7f4a_7c15))
                % 1_000_000) as f64
                / 1_000_000.0
                * 0.06
                - 0.03;
            let longitude = (id as f64 * golden_angle + jitter).rem_euclid(std::f64::consts::TAU)
                - std::f64::consts::PI;
            let center = Vec3::from_lon_lat(longitude, z.clamp(-1.0, 1.0).asin());
            let variation_seed =
                splitmix64(stage_seed ^ (id as u64).wrapping_mul(0xa076_1d64_78bd_642f));
            let variation_longitude = (variation_seed as f64 / u64::MAX as f64)
                * std::f64::consts::TAU
                - std::f64::consts::PI;
            let variation_latitude = (((variation_seed >> 32) as f64 / u32::MAX as f64) * 2.0
                - 1.0)
                .clamp(-1.0, 1.0)
                .asin();
            CratonSeed {
                center,
                variation_axis: Vec3::from_lon_lat(variation_longitude, variation_latitude),
            }
        })
        .collect()
}

fn craton_score(vector: Vec3, craton: CratonSeed) -> f64 {
    let growth = 1.0 - vector.angular_distance(craton.center) / CRATON_RADIUS_RADIANS;
    growth + vector.dot(craton.variation_axis) * CRATON_VARIATION_AMPLITUDE
}

fn crust_for_vector(vector: Vec3, cratons: &[CratonSeed]) -> CrustType {
    if cratons
        .iter()
        .copied()
        .map(|craton| craton_score(vector, craton))
        .fold(f64::NEG_INFINITY, f64::max)
        >= CRATON_SCORE_THRESHOLD
    {
        CrustType::Continental
    } else {
        CrustType::Oceanic
    }
}

fn build_plates(
    grid: Grid,
    seed: u32,
    retry_index: u32,
    settings: TectonicSettings,
    cratons: &[CratonSeed],
) -> Vec<Plate> {
    (0..settings.plate_count)
        .map(|id| {
            let site = plate_site(grid, seed, retry_index, id, settings.plate_count);
            let axis_stage_seed =
                derive_subsystem_seed(seed, retry_index, SeedDomain::RotationAxes);
            let axis_seed =
                splitmix64(axis_stage_seed ^ u64::from(id).wrapping_mul(0xa076_1d64_78bd_642f));
            let axis_longitude =
                (axis_seed as f64 / u64::MAX as f64) * std::f64::consts::TAU - std::f64::consts::PI;
            let axis_latitude = (((axis_seed >> 32) as f64 / u32::MAX as f64) * 2.0 - 1.0)
                .clamp(-1.0, 1.0)
                .asin();
            let speed = 200_000 + (splitmix64(axis_seed) % 1_000_001) as i64;
            let seed_cell = nearest_cell(grid, site);
            Plate {
                id,
                seed_cell,
                site_longitude_microdegrees: microdegrees(site.y.atan2(site.x).to_degrees()),
                site_latitude_microdegrees: microdegrees(site.z.asin().to_degrees()),
                rotation_axis_longitude_microdegrees: microdegrees(axis_longitude.to_degrees()),
                rotation_axis_latitude_microdegrees: microdegrees(axis_latitude.to_degrees()),
                angular_speed_nanoradians_per_year: speed,
                crust_type: crust_for_vector(cell_vector(grid, seed_cell), cratons),
            }
        })
        .collect()
}

fn build_crust_by_cell(
    grid: Grid,
    cratons: &[CratonSeed],
    progress: &mut dyn ProgressSink,
) -> Result<Vec<CrustType>, PhysicalError> {
    let mut crust = Vec::with_capacity(grid.sample_count());
    for cell in 0..grid.sample_count() {
        if cell % 256 == 0 {
            progress.check_cancelled()?;
        }
        crust.push(crust_for_vector(cell_vector(grid, cell), cratons));
    }
    Ok(crust)
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

fn assign_plates(
    grid: Grid,
    sites: &[Vec3],
    progress: &mut dyn ProgressSink,
) -> Result<Vec<u16>, PhysicalError> {
    let mut assigned = Vec::with_capacity(grid.sample_count());
    for cell in 0..grid.sample_count() {
        if cell % 256 == 0 {
            progress.check_cancelled()?;
        }
        let vector = cell_vector(grid, cell);
        assigned.push(
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
                .unwrap_or(0),
        );
    }
    Ok(assigned)
}

fn plate_velocity(plate: &Plate, site: Vec3) -> Vec3 {
    let axis = Vec3::from_lon_lat(
        f64::from(plate.rotation_axis_longitude_microdegrees) / MICRODEGREES_PER_DEGREE
            * std::f64::consts::PI
            / 180.0,
        f64::from(plate.rotation_axis_latitude_microdegrees) / MICRODEGREES_PER_DEGREE
            * std::f64::consts::PI
            / 180.0,
    );
    axis.cross(site)
        .scale(plate.angular_speed_nanoradians_per_year as f64)
}

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
    let direction = second_site.subtract(first_site).normalize();
    let relative_velocity =
        plate_velocity(second, second_site).subtract(plate_velocity(first, first_site));
    let normal_speed = relative_velocity.dot(direction);
    let kind = if normal_speed < -RELATIVE_MOTION_THRESHOLD_NANORADIANS_PER_YEAR {
        BoundaryKind::Convergent
    } else if normal_speed > RELATIVE_MOTION_THRESHOLD_NANORADIANS_PER_YEAR {
        BoundaryKind::Divergent
    } else {
        BoundaryKind::Transform
    };
    (kind, normal_speed.abs().round() as u64)
}

fn build_boundaries(
    grid: Grid,
    plates: &[Plate],
    plate_by_cell: &[u16],
    progress: &mut dyn ProgressSink,
) -> Result<Vec<BoundarySegment>, PhysicalError> {
    let mut boundaries = Vec::new();
    for first_cell in 0..grid.sample_count() {
        if first_cell % 256 == 0 {
            progress.check_cancelled()?;
        }
        for second_cell in grid.neighbors(first_cell) {
            if first_cell >= second_cell || plate_by_cell[first_cell] == plate_by_cell[second_cell]
            {
                continue;
            }
            let first_plate = plate_by_cell[first_cell];
            let second_plate = plate_by_cell[second_cell];
            let (kind, relative_speed) = classify_boundary(
                &plates[first_plate as usize],
                &plates[second_plate as usize],
            );
            boundaries.push(BoundarySegment {
                first_cell,
                second_cell,
                first_plate,
                second_plate,
                kind,
                relative_speed_nanoradians_per_year: relative_speed,
            });
        }
    }
    Ok(boundaries)
}

fn boundary_effect(
    boundary: BoundarySegment,
    first_crust: CrustType,
    second_crust: CrustType,
    tectonic_activity: f64,
) -> (f64, f64) {
    let strength = (boundary.relative_speed_nanoradians_per_year as f64 * 2.0)
        .clamp(100_000.0, 1_800_000.0)
        * tectonic_activity;
    match boundary.kind {
        BoundaryKind::Convergent => match (first_crust, second_crust) {
            (CrustType::Continental, CrustType::Continental) => (strength * 1.35, strength * 1.35),
            (CrustType::Continental, CrustType::Oceanic) => (strength * 0.85, -strength * 1.1),
            (CrustType::Oceanic, CrustType::Continental) => (-strength * 1.1, strength * 0.85),
            (CrustType::Oceanic, CrustType::Oceanic) => (strength * 0.25, -strength * 0.65),
        },
        BoundaryKind::Divergent => match (first_crust, second_crust) {
            (CrustType::Continental, CrustType::Continental) => {
                (-strength * 0.55, -strength * 0.55)
            }
            _ => (0.0, 0.0),
        },
        BoundaryKind::Transform => (0.0, 0.0),
    }
}

fn spreading_ridge_elevation(strength: f64, age_steps: u32) -> f64 {
    strength * RIDGE_RELIEF_FRACTION / (1.0 + f64::from(age_steps))
}

fn rift_shoulder_elevation(strength: f64, age_steps: u32) -> f64 {
    strength * RIFT_SHOULDER_FRACTION / (1.0 + f64::from(age_steps))
}

fn apply_divergent_relief(
    grid: Grid,
    boundary: BoundarySegment,
    first_crust: CrustType,
    second_crust: CrustType,
    crust_by_cell: &[CrustType],
    relief: &mut [f64],
    strength: f64,
) {
    if first_crust == CrustType::Continental && second_crust == CrustType::Continental {
        for origin in [boundary.first_cell, boundary.second_cell] {
            for cell in grid.neighbors(origin) {
                if cell != boundary.first_cell
                    && cell != boundary.second_cell
                    && crust_by_cell[cell] == CrustType::Continental
                {
                    relief[cell] += rift_shoulder_elevation(strength, 1);
                }
            }
        }
    } else if first_crust == CrustType::Oceanic && second_crust == CrustType::Oceanic {
        for origin in [boundary.first_cell, boundary.second_cell] {
            if crust_by_cell[origin] == CrustType::Oceanic {
                relief[origin] += spreading_ridge_elevation(strength, 0);
            }
            for cell in grid.neighbors(origin) {
                if crust_by_cell[cell] == CrustType::Oceanic {
                    relief[cell] += spreading_ridge_elevation(strength, 1);
                }
            }
        }
    }
}

fn subduction_arc_cell(
    grid: Grid,
    boundary: BoundarySegment,
    plate_by_cell: &[u16],
    crust_by_cell: &[CrustType],
) -> usize {
    let first_is_continental = crust_by_cell[boundary.first_cell] == CrustType::Continental;
    let (anchor, other, target_plate) = if first_is_continental {
        (
            boundary.first_cell,
            boundary.second_cell,
            boundary.first_plate,
        )
    } else {
        (
            boundary.second_cell,
            boundary.first_cell,
            boundary.second_plate,
        )
    };
    grid.neighbors(anchor)
        .into_iter()
        .filter(|cell| *cell != other && plate_by_cell[*cell] == target_plate)
        .min_by(|first, second| {
            cell_vector(grid, *first)
                .dot(cell_vector(grid, other))
                .partial_cmp(&cell_vector(grid, *second).dot(cell_vector(grid, other)))
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| first.cmp(second))
        })
        .unwrap_or(anchor)
}

fn build_relief(
    inputs: ReliefInputs<'_>,
) -> Result<(Vec<i32>, Vec<VolcanicCenter>), PhysicalError> {
    let ReliefInputs {
        grid,
        identity,
        settings,
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
    let mut relief = Vec::with_capacity(crust_by_cell.len());
    for (cell, crust) in crust_by_cell.iter().enumerate() {
        if cell % 256 == 0 {
            progress.check_cancelled()?;
        }
        let baseline = match crust {
            CrustType::Continental => 1_100_000.0,
            CrustType::Oceanic => -2_500_000.0,
        };
        let detail =
            (splitmix64(relief_stage_seed ^ (cell as u64).wrapping_mul(0x9e37_79b9_7f4a_7c15))
                % 1_000_001) as f64
                - 500_000.0;
        relief.push(baseline + detail * 0.45);
    }
    progress.check_cancelled()?;
    for boundary in boundaries {
        let first_crust = crust_by_cell[boundary.first_cell];
        let second_crust = crust_by_cell[boundary.second_cell];
        let (first_effect, second_effect) =
            boundary_effect(*boundary, first_crust, second_crust, tectonic_activity);
        relief[boundary.first_cell] += first_effect;
        relief[boundary.second_cell] += second_effect;
        if boundary.kind == BoundaryKind::Divergent {
            let strength = (boundary.relative_speed_nanoradians_per_year as f64 * 2.0)
                .clamp(100_000.0, 1_800_000.0)
                * tectonic_activity;
            apply_divergent_relief(
                grid,
                *boundary,
                first_crust,
                second_crust,
                crust_by_cell,
                &mut relief,
                strength,
            );
        }
    }

    let mut volcanic_centers = Vec::new();
    for cell in 0..plate_by_cell.len() {
        if cell % 256 == 0 {
            progress.check_cancelled()?;
        }
        let random =
            splitmix64(hotspot_stage_seed ^ (cell as u64).wrapping_mul(0xa076_1d64_78bd_642f));
        let hotspot_threshold = u64::from(settings.island_activity_ppm / 10);
        if random % 1_000_000 < hotspot_threshold && volcanic_centers.len() < MAX_VOLCANIC_CENTERS {
            let intensity = 700_000 + (random % 1_600_001) as i64;
            relief[cell] += intensity as f64;
            volcanic_centers.push(VolcanicCenter {
                cell,
                plate: plate_by_cell[cell],
                kind: VolcanicCenterKind::Hotspot,
                intensity_ppm: (intensity / 10).clamp(1, 1_000_000) as u32,
            });
        }
    }
    for (index, boundary) in boundaries.iter().enumerate() {
        if boundary.kind == BoundaryKind::Convergent
            && index % 4 == 0
            && volcanic_centers.len() < MAX_VOLCANIC_CENTERS
        {
            let cell = subduction_arc_cell(grid, *boundary, plate_by_cell, crust_by_cell);
            relief[cell] += 350_000.0 * tectonic_activity;
            volcanic_centers.push(VolcanicCenter {
                cell,
                plate: plate_by_cell[cell],
                kind: VolcanicCenterKind::SubductionArc,
                intensity_ppm: 350_000,
            });
        }
    }
    let elevations = relief
        .into_iter()
        .map(|value| value.round().clamp(-9_000_000.0, 9_000_000.0) as i32)
        .collect();
    Ok((elevations, volcanic_centers))
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
        for (index, first) in self.boundaries.iter().enumerate() {
            if first.first_cell >= sample_count
                || first.second_cell >= sample_count
                || first.first_cell >= first.second_cell
                || first.first_plate == first.second_plate
                || usize::from(first.first_plate) >= self.plates.len()
                || usize::from(first.second_plate) >= self.plates.len()
                || self.plate_by_cell[first.first_cell] != first.first_plate
                || self.plate_by_cell[first.second_cell] != first.second_plate
                || !self
                    .grid
                    .neighbors(first.first_cell)
                    .contains(&first.second_cell)
                || !self
                    .grid
                    .neighbors(first.second_cell)
                    .contains(&first.first_cell)
            {
                return Err(PhysicalError::Validation(
                    "tectonic boundary has invalid topology".into(),
                ));
            }
            for second in self.boundaries.iter().skip(index + 1) {
                if first.first_cell == second.first_cell && first.second_cell == second.second_cell
                {
                    return Err(PhysicalError::Validation(
                        "tectonic boundary is duplicated".into(),
                    ));
                }
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
                for neighbor in self.grid.neighbors(cell) {
                    if !visited[neighbor] && self.elevations_mm[neighbor] > self.sea_level_mm {
                        visited[neighbor] = true;
                        stack.push(neighbor);
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
        return Err("tectonic source exceeds the 16 MiB source budget".into());
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

fn cell_polygon(grid: Grid, cell: usize) -> String {
    let (row, col) = grid.row_col(cell);
    let west = super::longitude_micro(&grid, col);
    let east = super::longitude_micro(&grid, col + 1);
    let south = super::latitude_micro(&grid, row);
    let north = super::latitude_micro(&grid, row + 1);
    format!(
        "[[[{},{}],[{},{}],[{},{}],[{},{}],[{},{}]]]",
        super::format_micro(west),
        super::format_micro(south),
        super::format_micro(east),
        super::format_micro(south),
        super::format_micro(east),
        super::format_micro(north),
        super::format_micro(west),
        super::format_micro(north),
        super::format_micro(west),
        super::format_micro(south),
    )
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

fn add_diagnostic_feature(features: &mut Vec<String>, feature: String) -> Result<(), String> {
    if features.len() >= super::MAX_GEOJSON_FEATURES {
        return Err("physical diagnostic output exceeds the feature budget".into());
    }
    features.push(feature);
    Ok(())
}

fn add_boundary_features(
    features: &mut Vec<String>,
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
    match grid.sample_count() {
        0..=8_192 => 1,
        8_193..=131_072 => 2,
        _ => 4,
    }
}

/// Emit all current tectonic diagnostics as derived, read-only vector layers.
/// The canonical source remains the v2 binary; this output is disposable and
/// can be regenerated from it after restart or cache deletion.
pub fn to_diagnostic_geojson(world: &TectonicWorld) -> Result<String, String> {
    world
        .validate()
        .map_err(|error| format!("invalid tectonic diagnostics: {error}"))?;
    let field = world.physical_field();
    let mut features = Vec::new();
    for (index, segment) in super::coastline_segments(&field).iter().enumerate() {
        add_diagnostic_feature(
            &mut features,
            diagnostic_feature(
                &format!("coastline-{index:05}"),
                "base",
                "custom",
                "physical coastline",
                "LineString",
                &line_geometry(segment.first, segment.second),
            ),
        )?;
    }
    let stride = diagnostic_cell_stride(world.grid);
    for row in (0..world.grid.height).step_by(stride as usize) {
        for col in (0..world.grid.width).step_by(stride as usize) {
            let cell = world.grid.index(row, col);
            let plate = world.plate_by_cell[cell];
            add_diagnostic_feature(
                &mut features,
                diagnostic_feature(
                    &format!("plate-cell-{cell:05}"),
                    "tectonic-plates",
                    "region",
                    &format!("Plate {plate} (LOD {stride})"),
                    "Polygon",
                    &cell_polygon(world.grid, cell),
                ),
            )?;
            let elevation = world.elevations_mm[cell];
            add_diagnostic_feature(
                &mut features,
                diagnostic_feature(
                    &format!("bathymetry-cell-{cell:05}"),
                    "bathymetry",
                    "custom",
                    &format!("Elevation {elevation} mm (LOD {stride})"),
                    "Polygon",
                    &cell_polygon(world.grid, cell),
                ),
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
    let mut output = String::from(r#"{"type":"FeatureCollection","features":["#);
    for (index, feature) in features.iter().enumerate() {
        if index > 0 {
            output.push(',');
        }
        output.push_str(feature);
    }
    output.push_str("]}");
    if output.len() > TECTONIC_MAX_SOURCE_BYTES {
        return Err("physical diagnostic output exceeds the 16 MiB budget".into());
    }
    Ok(output)
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
    if !(1..1_000_000).contains(&target_land_fraction_ppm) {
        return Err(PhysicalError::InvalidSettings(
            "target land fraction must be between zero and one".into(),
        ));
    }
    progress.report(ProgressPhase::BuildingTectonicStructure, 0, 3)?;
    let sites = (0..settings.plate_count)
        .map(|id| plate_site(grid, seed, retry_index, id, settings.plate_count))
        .collect::<Vec<_>>();
    let cratons = craton_seeds(seed, retry_index, settings);
    let plates = build_plates(grid, seed, retry_index, settings, &cratons);
    let plate_by_cell = assign_plates(grid, &sites, progress)?;
    let crust_by_cell = build_crust_by_cell(grid, &cratons, progress)?;
    progress.report(ProgressPhase::BuildingTectonicStructure, 1, 3)?;
    let boundaries = build_boundaries(grid, &plates, &plate_by_cell, progress)?;
    progress.report(ProgressPhase::BuildingTectonicStructure, 2, 3)?;
    let (elevations_mm, volcanic_centers) = build_relief(ReliefInputs {
        grid,
        identity: GenerationIdentity { seed, retry_index },
        settings,
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
        let first = world.plates[0];
        let second = world.plates[1];
        assert_eq!(
            classify_boundary(&first, &second),
            classify_boundary(&second, &first)
        );
        assert_eq!(RELATIVE_MOTION_THRESHOLD_NANORADIANS_PER_YEAR, 25_000.0);
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
            assert!(segments.iter().any(|segment| {
                segment.first == [0, -90_000_000]
                    || segment.second == [0, -90_000_000]
                    || segment.first == [0, 90_000_000]
                    || segment.second == [0, 90_000_000]
            }));
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
        let mut progress = NoopProgress;
        let retry = generate_tectonic_world(
            first.grid,
            first.settings,
            first.target_land_fraction_ppm,
            first.seed,
            first.retry_index + 1,
            &mut progress,
        )
        .unwrap();
        let mut progress = NoopProgress;
        let repeat = generate_tectonic_world(
            first.grid,
            first.settings,
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
        assert!(geojson.len() < TECTONIC_MAX_SOURCE_BYTES);
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
}
