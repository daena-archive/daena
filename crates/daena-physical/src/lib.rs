//! Pure Rust physical generator for the native physical map model.
//!
//! This crate deliberately has no Daena, Tauri, SQLite, or frontend
//! dependency. Its integer source codec and deterministic field generation are
//! the canonical physical-map production boundary.

use std::collections::HashMap;
use std::fmt::{Display, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, OnceLock};

pub mod contours;
pub mod derived_cache;
pub mod engine;
pub mod events;
pub mod find_place;
pub mod history;
pub mod hydro_claim;
pub mod landmass_claim;
pub mod planetary;
pub mod products;
pub mod region;
pub mod routing;

pub const SOURCE_MAGIC: [u8; 8] = engine::SOURCE_MAGIC;
pub const SOURCE_VERSION: u16 = engine::SOURCE_VERSION;
pub const SOURCE_HEADER_BYTES: usize = engine::SOURCE_HEADER_BYTES;
pub const DEFAULT_RADIUS_METRES: u64 = 6_371_000;
pub const SUPPORTED_PREVIEW_MAX_WIDTH: u32 = 2048;
pub const SUPPORTED_PREVIEW_MAX_HEIGHT: u32 = 1024;
/// Accepted-source size bound. The largest accepted source is an n=1024
/// world at 25,166,172 bytes, so 32 MiB leaves 33 % headroom.
pub const MAX_SOURCE_BYTES: usize = 32 * 1024 * 1024;
/// Derived GeoJSON is disposable and is not stored in the v2 source. The E8
/// sweep (eight presets, n=512 and n=1024, present and -21,000 years) measures
/// at most 242,827 bytes at 1024x512, and the E5 2048x1024 measurement is
/// 264,838 bytes; 4 MiB leaves more than 15x.
pub const MAX_DERIVED_GEOJSON_BYTES: usize = 4 * 1024 * 1024;
/// Top-level feature ceiling for derived products and region rings. Eight
/// presets at n=512 (two seeds) and n=1024, present and a -21,000-year epoch,
/// measure at most 427 features and 932 region rings; 4,096 leaves 4.4x.
pub const MAX_GEOJSON_FEATURES: usize = 4_096;
/// Coastline segment ceiling for the 1024x512 product view. Segments are not
/// features: the same 24-world sweep measures up to 8,820 (Shattered, n=1024,
/// -21,000 years), so 65,536 leaves 7.4x.
pub const MAX_COASTLINE_SEGMENTS: usize = 65_536;
pub const CANCELLATION_LATENCY_BUDGET_MS: u128 = 100;
/// Release n=1024 generation including the product adapters measures
/// 4.16-4.23 s over three runs; 8 s leaves about 1.9x.
pub const GENERATION_TIME_BUDGET_MS: u128 = 8_000;
/// Release n=1024 generation peaks at 1.33 GB RSS (0.95 GB live with
/// `MallocLargeCache=0`); 1.5 GiB leaves 1.2x over RSS. An Atlas epoch
/// prepare at n=1024 peaks higher (1.64 GB live); see the E8 notes.
pub const WORKING_MEMORY_BUDGET_BYTES: usize = 3 * 512 * 1024 * 1024;
pub const GENERATOR_ID: &str = "daena-physical-world";
pub const GENERATOR_VERSION: u32 = 2;

pub const CODE_GENERATOR_INVALID_SETTINGS: &str = "physical.generator.invalid-settings";
pub const CODE_GENERATOR_UNSUPPORTED_VERSION: &str = "physical.generator.unsupported-version";
pub const CODE_GENERATOR_CANCELLED: &str = "physical.generator.cancelled";
pub const CODE_GENERATOR_RETRY_EXHAUSTED: &str = "physical.generator.retry-exhausted";
pub const CODE_SOURCE_INVALID: &str = "physical.source.invalid";
pub const CODE_SOURCE_UNSUPPORTED_VERSION: &str = "physical.source.unsupported-version";
pub const CODE_NUMERIC_NON_FINITE: &str = "physical.numeric.non-finite";
pub const CODE_NUMERIC_NON_CONVERGENT: &str = "physical.numeric.non-convergent";
pub const CODE_WATER_NON_CONVERGENT: &str = "physical.water.non-convergent";
pub const CODE_HYDROLOGY_CYCLE: &str = "physical.hydrology.cycle";
pub const CODE_HYDROLOGY_INVALID_SINK: &str = "physical.hydrology.invalid-sink";
pub const CODE_GEOMETRY_INVALID: &str = "physical.geometry.invalid";
pub const CODE_LIMIT_EXCEEDED: &str = "physical.limit.exceeded";
pub const CODE_RENDERER_UNAVAILABLE: &str = "physical.renderer.unavailable";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PhysicalErrorCode {
    GeneratorInvalidSettings,
    GeneratorUnsupportedVersion,
    GeneratorCancelled,
    GeneratorRetryExhausted,
    SourceInvalid,
    SourceUnsupportedVersion,
    NumericNonFinite,
    NumericNonConvergent,
    WaterNonConvergent,
    HydrologyCycle,
    HydrologyInvalidSink,
    GeometryInvalid,
    LimitExceeded,
    RendererUnavailable,
}

impl PhysicalErrorCode {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::GeneratorInvalidSettings => CODE_GENERATOR_INVALID_SETTINGS,
            Self::GeneratorUnsupportedVersion => CODE_GENERATOR_UNSUPPORTED_VERSION,
            Self::GeneratorCancelled => CODE_GENERATOR_CANCELLED,
            Self::GeneratorRetryExhausted => CODE_GENERATOR_RETRY_EXHAUSTED,
            Self::SourceInvalid => CODE_SOURCE_INVALID,
            Self::SourceUnsupportedVersion => CODE_SOURCE_UNSUPPORTED_VERSION,
            Self::NumericNonFinite => CODE_NUMERIC_NON_FINITE,
            Self::NumericNonConvergent => CODE_NUMERIC_NON_CONVERGENT,
            Self::WaterNonConvergent => CODE_WATER_NON_CONVERGENT,
            Self::HydrologyCycle => CODE_HYDROLOGY_CYCLE,
            Self::HydrologyInvalidSink => CODE_HYDROLOGY_INVALID_SINK,
            Self::GeometryInvalid => CODE_GEOMETRY_INVALID,
            Self::LimitExceeded => CODE_LIMIT_EXCEEDED,
            Self::RendererUnavailable => CODE_RENDERER_UNAVAILABLE,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProgressPhase {
    BuildingTectonicStructure,
    BuildingTerrain,
    CalculatingClimate,
    ErodingLandscape,
    CalculatingWater,
    BuildingRiversAndLakes,
    PreparingGeography,
    ValidatingWorld,
}

impl ProgressPhase {
    pub const fn label(self) -> &'static str {
        match self {
            Self::BuildingTectonicStructure => "Building tectonic structure",
            Self::BuildingTerrain => "Building terrain",
            Self::CalculatingClimate => "Calculating climate",
            Self::ErodingLandscape => "Eroding landscape",
            Self::CalculatingWater => "Calculating water",
            Self::BuildingRiversAndLakes => "Building rivers and lakes",
            Self::PreparingGeography => "Preparing geography",
            Self::ValidatingWorld => "Validating world",
        }
    }
}

pub const PROGRESS_PHASES: [ProgressPhase; 8] = [
    ProgressPhase::BuildingTectonicStructure,
    ProgressPhase::BuildingTerrain,
    ProgressPhase::CalculatingClimate,
    ProgressPhase::ErodingLandscape,
    ProgressPhase::CalculatingWater,
    ProgressPhase::BuildingRiversAndLakes,
    ProgressPhase::PreparingGeography,
    ProgressPhase::ValidatingWorld,
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PhysicalError {
    InvalidSettings(String),
    InvalidSource(String),
    Validation(String),
    Coded {
        code: PhysicalErrorCode,
        message: String,
    },
    Cancelled,
}

impl Display for PhysicalError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}: {}", self.code(), self.message())
    }
}

impl PhysicalError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::InvalidSettings(_) => CODE_GENERATOR_INVALID_SETTINGS,
            Self::InvalidSource(message) if message.contains("unsupported") => {
                CODE_SOURCE_UNSUPPORTED_VERSION
            }
            Self::InvalidSource(_) => CODE_SOURCE_INVALID,
            Self::Validation(_) => CODE_NUMERIC_NON_CONVERGENT,
            Self::Coded { code, .. } => code.as_str(),
            Self::Cancelled => CODE_GENERATOR_CANCELLED,
        }
    }

    fn message(&self) -> &str {
        match self {
            Self::InvalidSettings(message)
            | Self::InvalidSource(message)
            | Self::Validation(message) => message,
            Self::Coded { message, .. } => message,
            Self::Cancelled => "generation was cancelled",
        }
    }

    pub fn coded(code: PhysicalErrorCode, message: impl Into<String>) -> Self {
        Self::Coded {
            code,
            message: message.into(),
        }
    }
}

impl std::error::Error for PhysicalError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ValidationReport {
    pub actual_land_fraction_ppm: u32,
    pub reference_water_inventory_m3: u64,
    pub coastline_segments: usize,
}

pub trait ProgressSink: Send + Sync {
    fn report(
        &mut self,
        phase: ProgressPhase,
        completed: u32,
        total: u32,
    ) -> Result<(), PhysicalError>;

    fn check_cancelled(&self) -> Result<(), PhysicalError> {
        Ok(())
    }
}

pub struct NoopProgress;

impl ProgressSink for NoopProgress {
    fn report(
        &mut self,
        _phase: ProgressPhase,
        _completed: u32,
        _total: u32,
    ) -> Result<(), PhysicalError> {
        Ok(())
    }
}

pub struct CancellationProgress<'a> {
    pub cancelled: &'a AtomicBool,
    pub on_progress: &'a (dyn Fn(ProgressPhase, u32, u32) + Sync),
}

impl ProgressSink for CancellationProgress<'_> {
    fn report(
        &mut self,
        phase: ProgressPhase,
        completed: u32,
        total: u32,
    ) -> Result<(), PhysicalError> {
        self.check_cancelled()?;
        (self.on_progress)(phase, completed, total);
        Ok(())
    }

    fn check_cancelled(&self) -> Result<(), PhysicalError> {
        if self.cancelled.load(Ordering::Relaxed) {
            Err(PhysicalError::Cancelled)
        } else {
            Ok(())
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Grid {
    pub width: u32,
    pub height: u32,
    pub radius_metres: u64,
}

impl Grid {
    pub fn new(width: u32, height: u32, radius_metres: u64) -> Result<Self, String> {
        if !(4..=SUPPORTED_PREVIEW_MAX_WIDTH).contains(&width)
            || !(2..=SUPPORTED_PREVIEW_MAX_HEIGHT).contains(&height)
        {
            return Err("grid dimensions exceed the supported preview bounds".into());
        }
        if radius_metres == 0 {
            return Err("planet radius must be positive".into());
        }
        Ok(Self {
            width,
            height,
            radius_metres,
        })
    }

    pub fn sample_count(self) -> usize {
        self.width as usize * self.height as usize
    }

    pub fn row_col(self, index: usize) -> (u32, u32) {
        (index as u32 / self.width, index as u32 % self.width)
    }

    pub fn index(self, row: u32, col: u32) -> usize {
        row as usize * self.width as usize + col as usize
    }

    pub fn cell_area(self, row: u32) -> f64 {
        let delta_lon = std::f64::consts::TAU / f64::from(self.width);
        let south = -std::f64::consts::FRAC_PI_2
            + std::f64::consts::PI * f64::from(row) / f64::from(self.height);
        let north = -std::f64::consts::FRAC_PI_2
            + std::f64::consts::PI * f64::from(row + 1) / f64::from(self.height);
        (self.radius_metres as f64).powi(2) * delta_lon * (north.sin() - south.sin())
    }

    pub fn total_area(self) -> f64 {
        f64::from(self.width) * (0..self.height).map(|row| self.cell_area(row)).sum::<f64>()
    }

    pub fn center_radians(self, row: u32, col: u32) -> (f64, f64) {
        let longitude = -std::f64::consts::PI
            + std::f64::consts::TAU * (f64::from(col) + 0.5) / f64::from(self.width);
        let latitude = -std::f64::consts::FRAC_PI_2
            + std::f64::consts::PI * (f64::from(row) + 0.5) / f64::from(self.height);
        (longitude, latitude)
    }
}

pub fn wrap_radians(delta: f64) -> f64 {
    if !delta.is_finite() {
        return 0.0;
    }
    let tau = std::f64::consts::TAU;
    let pi = std::f64::consts::PI;
    (delta + pi).rem_euclid(tau) - pi
}

impl Grid {
    /// Great-circle distance using wrapped longitude and haversine arithmetic.
    pub fn great_circle_distance(self, first: (f64, f64), second: (f64, f64)) -> f64 {
        let delta_lon = wrap_radians(second.0 - first.0);
        let delta_lat = second.1 - first.1;
        let haversine = (delta_lat / 2.0).sin().powi(2)
            + first.1.cos() * second.1.cos() * (delta_lon / 2.0).sin().powi(2);
        2.0 * (haversine.sqrt()).atan2((1.0 - haversine).max(0.0).sqrt())
            * self.radius_metres as f64
    }

    /// Neighbor policy for topology checks:
    ///
    /// * columns wrap modulo `width`;
    /// * the first and last rows terminate at a single pole;
    /// * all cells in a polar row are point-adjacent through that pole; and
    /// * returned indexes are sorted and deduplicated.
    pub fn neighbors(self, index: usize) -> Vec<usize> {
        self.topology().neighbors(index).to_vec()
    }

    pub(crate) fn topology(self) -> Arc<GridTopology> {
        cached_topology(self)
    }

    fn collect_neighbors(self, index: usize) -> Vec<usize> {
        let (row, col) = self.row_col(index);
        let mut neighbors = Vec::with_capacity(self.width as usize + 4);
        neighbors.push(self.index(row, (col + self.width - 1) % self.width));
        neighbors.push(self.index(row, (col + 1) % self.width));
        if row > 0 {
            neighbors.push(self.index(row - 1, col));
        } else {
            neighbors.extend((0..self.width).map(|polar_col| self.index(0, polar_col)));
        }
        if row + 1 < self.height {
            neighbors.push(self.index(row + 1, col));
        } else {
            neighbors
                .extend((0..self.width).map(|polar_col| self.index(self.height - 1, polar_col)));
        }
        neighbors.retain(|neighbor| *neighbor != index);
        neighbors.sort_unstable();
        neighbors.dedup();
        neighbors
    }

    pub(crate) fn row_areas(self) -> Vec<f64> {
        (0..self.height).map(|row| self.cell_area(row)).collect()
    }
}

#[derive(Debug, Clone)]
pub(crate) struct GridTopology {
    offsets: Vec<u32>,
    neighbors: Vec<usize>,
}

impl GridTopology {
    fn from_grid(grid: Grid) -> Self {
        let sample_count = grid.sample_count();
        let mut offsets = Vec::with_capacity(sample_count + 1);
        let mut neighbors = Vec::new();
        offsets.push(0);
        for index in 0..sample_count {
            neighbors.extend(grid.collect_neighbors(index));
            offsets.push(neighbors.len() as u32);
        }
        Self { offsets, neighbors }
    }

    pub(crate) fn neighbors(&self, index: usize) -> &[usize] {
        let start = self.offsets[index] as usize;
        let end = self.offsets[index + 1] as usize;
        &self.neighbors[start..end]
    }
}

type TopologyCache = Mutex<HashMap<(u32, u32), Arc<GridTopology>>>;

fn cached_topology(grid: Grid) -> Arc<GridTopology> {
    static CACHE: OnceLock<TopologyCache> = OnceLock::new();
    let cache = CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    let mut cache = cache
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    cache
        .entry((grid.width, grid.height))
        .or_insert_with(|| Arc::new(GridTopology::from_grid(grid)))
        .clone()
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PhysicalField {
    pub grid: Grid,
    pub seed: u32,
    pub retry_index: u32,
    pub target_land_fraction_ppm: u32,
    pub sea_level_mm: i32,
    pub elevations_mm: Vec<i32>,
}

impl PhysicalField {
    pub fn validate(&self) -> Result<(), String> {
        if self.elevations_mm.len() != self.grid.sample_count() {
            return Err("elevation sample count does not match the grid".into());
        }
        if self.target_land_fraction_ppm == 0 || self.target_land_fraction_ppm >= 1_000_000 {
            return Err("target land fraction must be in parts per million".into());
        }
        Ok(())
    }
}

fn splitmix64(mut value: u64) -> u64 {
    value = value.wrapping_add(0x9e37_79b9_7f4a_7c15);
    let mut result = value;
    result = (result ^ (result >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    result = (result ^ (result >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    result ^ (result >> 31)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SeedDomain {
    PlateSites,
    ContinentalCratons,
    RotationAxes,
    ReliefDetail,
    Hotspots,
    Climate,
    Erosion,
    Hydrology,
    Hazards,
    HistoricalClimate,
    PlateBoundaryCost,
    DetachedTerranes,
    PlateReshape,
    PlateTransformSlip,
}

impl SeedDomain {
    const fn tag(self) -> u64 {
        match self {
            Self::PlateSites => 0x706c_6174_6573_0001,
            Self::ContinentalCratons => 0x6372_6174_6f6e_0002,
            Self::RotationAxes => 0x726f_7461_7465_0003,
            Self::ReliefDetail => 0x7265_6c69_6566_0004,
            Self::Hotspots => 0x686f_7473_706f_0005,
            Self::Climate => 0x636c_696d_6174_0006,
            Self::Erosion => 0x6572_6f73_696f_0007,
            Self::Hydrology => 0x6879_6472_6f00_0008,
            Self::Hazards => 0x6861_7a61_7264_0009,
            Self::HistoricalClimate => 0x6869_7374_6f72_0010,
            Self::PlateBoundaryCost => 0x706c_6163_6f73_0011,
            Self::DetachedTerranes => 0x7465_7272_616e_0012,
            Self::PlateReshape => 0x7265_7368_6170_0013,
            Self::PlateTransformSlip => 0x7472_616e_736c_0014,
        }
    }
}

pub fn derive_subsystem_seed(seed: u32, retry_index: u32, domain: SeedDomain) -> u64 {
    splitmix64(
        u64::from(seed) ^ u64::from(retry_index).wrapping_mul(0xd6e8_feb8_6659_fd93) ^ domain.tag(),
    )
}

/// Derive a subsystem value from the full 64-bit world seed.
pub fn derive_subsystem_seed64(seed: u64, domain: SeedDomain) -> u64 {
    splitmix64(seed ^ domain.tag())
}

pub fn land_fraction(grid: &Grid, elevations_mm: &[i32], sea_level_mm: i32) -> f64 {
    let row_areas = grid.row_areas();
    let total = f64::from(grid.width) * row_areas.iter().sum::<f64>();
    land_fraction_from_row_areas(grid, elevations_mm, sea_level_mm, &row_areas, total)
}

fn land_fraction_from_row_areas(
    grid: &Grid,
    elevations_mm: &[i32],
    sea_level_mm: i32,
    row_areas: &[f64],
    total: f64,
) -> f64 {
    let width = grid.width as usize;
    elevations_mm
        .iter()
        .enumerate()
        .filter(|(_, elevation)| **elevation > sea_level_mm)
        .map(|(index, _)| row_areas[index / width])
        .sum::<f64>()
        / total
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Segment {
    pub first: [i32; 2],
    pub second: [i32; 2],
}

pub fn coastline_paths(field: &PhysicalField) -> Result<Vec<Vec<[i32; 2]>>, PhysicalError> {
    contours::isolines_at(field.grid, &field.elevations_mm, field.sea_level_mm)
}

pub fn coastline_paths_with_progress(
    field: &PhysicalField,
    progress: &dyn ProgressSink,
) -> Result<Vec<Vec<[i32; 2]>>, PhysicalError> {
    field.validate().map_err(PhysicalError::InvalidSource)?;
    contours::isolines_at_with_cancel(
        field.grid,
        &field.elevations_mm,
        field.sea_level_mm,
        &|| progress.check_cancelled().is_err(),
    )
}

pub fn coastline_segments(field: &PhysicalField) -> Vec<Segment> {
    coastline_paths(field)
        .map(|paths| contours::segments_from_paths(&paths))
        .unwrap_or_default()
}

pub fn to_geojson(field: &PhysicalField) -> Result<String, String> {
    let paths = coastline_paths(field).map_err(|error| error.to_string())?;
    format_coastline_geojson(&paths, &|| false).map_err(|error| error.to_string())
}

pub fn to_geojson_with_progress(
    field: &PhysicalField,
    progress: &dyn ProgressSink,
) -> Result<String, PhysicalError> {
    let paths = coastline_paths_with_progress(field, progress)?;
    format_coastline_geojson(&paths, &|| progress.check_cancelled().is_err())
}

fn format_coastline_geojson(
    paths: &[Vec<[i32; 2]>],
    cancelled: &(dyn Fn() -> bool + Sync),
) -> Result<String, PhysicalError> {
    if paths.len() > MAX_GEOJSON_FEATURES {
        return Err(PhysicalError::coded(
            PhysicalErrorCode::LimitExceeded,
            "derived coastline exceeds the iteration-0 feature budget",
        ));
    }
    let mut output = String::from(r#"{"type":"FeatureCollection","features":["#);
    for (index, path) in paths.iter().enumerate() {
        if index.is_multiple_of(64) && cancelled() {
            return Err(PhysicalError::Cancelled);
        }
        if path.len() < 2 {
            continue;
        }
        if index > 0 {
            output.push(',');
        }
        output.push_str(&format!(
            r#"{{"type":"Feature","id":"coastline-{index:05}","properties":{{"daenaLayerId":"base","kind":"custom","name":"physical coastline"}},"geometry":{{"type":"LineString","coordinates":"#
        ));
        output.push('[');
        for (point_index, point) in path.iter().enumerate() {
            if point_index > 0 {
                output.push(',');
            }
            write!(
                output,
                "[{},{}]",
                format_micro(point[0]),
                format_micro(point[1])
            )
            .map_err(|_| {
                PhysicalError::coded(
                    PhysicalErrorCode::GeometryInvalid,
                    "failed to format derived GeoJSON",
                )
            })?;
        }
        output.push_str("]}}");
    }
    output.push_str("]}");
    Ok(output)
}

pub(crate) fn format_micro(value: i32) -> String {
    let mut output = String::new();
    write_micro(&mut output, value);
    output
}

fn write_micro(output: &mut String, value: i32) {
    if value < 0 {
        output.push('-');
    }
    let absolute = value.unsigned_abs();
    let whole = absolute / 1_000_000;
    let fraction = absolute % 1_000_000;
    let _ = write!(output, "{whole}");
    if fraction != 0 {
        output.push('.');
        let mut digits = [b'0'; 6];
        let mut rest = fraction;
        for index in (0..6).rev() {
            digits[index] = b'0' + (rest % 10) as u8;
            rest /= 10;
        }
        let mut end = 6;
        while end > 1 && digits[end - 1] == b'0' {
            end -= 1;
        }
        output.push_str(std::str::from_utf8(&digits[..end]).unwrap_or("0"));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stable_error_and_progress_vocabularies_are_complete() {
        let labels = PROGRESS_PHASES.map(ProgressPhase::label);
        assert_eq!(
            labels,
            [
                "Building tectonic structure",
                "Building terrain",
                "Calculating climate",
                "Eroding landscape",
                "Calculating water",
                "Building rivers and lakes",
                "Preparing geography",
                "Validating world",
            ]
        );
        assert_eq!(
            PhysicalError::coded(PhysicalErrorCode::WaterNonConvergent, "test").code(),
            CODE_WATER_NON_CONVERGENT
        );
        assert_eq!(PhysicalError::Cancelled.code(), CODE_GENERATOR_CANCELLED);
    }

    #[test]
    fn named_subsystem_seeds_are_distinct_and_retry_scoped() {
        let domains = [
            SeedDomain::PlateSites,
            SeedDomain::ContinentalCratons,
            SeedDomain::RotationAxes,
            SeedDomain::ReliefDetail,
            SeedDomain::Hotspots,
            SeedDomain::Climate,
            SeedDomain::Erosion,
            SeedDomain::Hydrology,
            SeedDomain::Hazards,
            SeedDomain::HistoricalClimate,
            SeedDomain::PlateBoundaryCost,
            SeedDomain::DetachedTerranes,
            SeedDomain::PlateReshape,
            SeedDomain::PlateTransformSlip,
        ];
        for (index, domain) in domains.iter().copied().enumerate() {
            assert!(domains
                .iter()
                .skip(index + 1)
                .all(|other| derive_subsystem_seed(831_429, 0, domain)
                    != derive_subsystem_seed(831_429, 0, *other)));
            assert_ne!(
                derive_subsystem_seed(831_429, 0, domain),
                derive_subsystem_seed(831_429, 1, domain)
            );
        }
    }

    #[test]
    fn spherical_cell_areas_cover_the_planet() {
        let grid = Grid::new(64, 32, DEFAULT_RADIUS_METRES).unwrap();
        let expected = 4.0 * std::f64::consts::PI * (DEFAULT_RADIUS_METRES as f64).powi(2);
        let relative_error = (grid.total_area() - expected).abs() / expected;
        assert!(relative_error < 1e-12, "relative error: {relative_error}");
    }

    #[test]
    fn wrapped_distance_and_pole_adjacency_are_explicit() {
        let grid = Grid::new(8, 4, DEFAULT_RADIUS_METRES).unwrap();
        let first = grid.center_radians(1, 0);
        let last = grid.center_radians(1, 7);
        let wrapped = grid.great_circle_distance(first, last);
        let direct = grid.great_circle_distance(first, grid.center_radians(1, 1));
        assert!((wrapped - direct).abs() < 1e-6);
        assert_eq!(grid.neighbors(grid.index(0, 0)).len(), 8);
        assert_eq!(grid.neighbors(grid.index(3, 0)).len(), 8);
        assert_eq!(grid.neighbors(grid.index(1, 0)), vec![0, 9, 15, 16]);
    }

    #[test]
    fn source_version_one_is_rejected_as_unsupported() {
        let mut bytes = vec![0u8; 68];
        bytes[0..8].copy_from_slice(b"DAENAPW1");
        bytes[8..10].copy_from_slice(&1u16.to_le_bytes());
        let error = engine::decode_source(&bytes).unwrap_err();
        assert_eq!(error.code(), CODE_SOURCE_UNSUPPORTED_VERSION);
    }
}
