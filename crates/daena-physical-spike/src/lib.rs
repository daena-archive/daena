//! Pure Rust physical generator for the native physical map model.
//!
//! This crate deliberately has no Daena, Tauri, SQLite, or frontend
//! dependency. Its integer source codec and deterministic field generation are
//! the canonical physical-map production boundary.

use std::fmt::{Display, Write};
use std::sync::atomic::{AtomicBool, Ordering};

pub mod climate;
pub mod evolution;
pub mod hydrology;
pub mod tectonics;

pub const SOURCE_MAGIC: [u8; 8] = *b"DAENAPW1";
pub const SOURCE_VERSION: u16 = tectonics::TECTONIC_SOURCE_VERSION;
pub const SOURCE_HEADER_BYTES: usize = tectonics::TECTONIC_SOURCE_HEADER_BYTES;
pub const DEFAULT_WIDTH: u32 = 64;
pub const DEFAULT_HEIGHT: u32 = 32;
pub const DEFAULT_RADIUS_METRES: u64 = 6_371_000;
pub const MAX_WIDTH: u32 = 128;
pub const MAX_HEIGHT: u32 = 64;
pub const MAX_GEOJSON_FEATURES: usize = 32_768;
pub const CANCELLATION_LATENCY_BUDGET_MS: u128 = 100;
pub const GENERATOR_ID: &str = "daena-physical-world";
pub const GENERATOR_VERSION: u32 = 6;

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
pub struct GenerationSettings {
    pub width: u32,
    pub height: u32,
    pub radius_metres: u64,
    pub target_land_fraction_ppm: u32,
}

impl GenerationSettings {
    pub fn grid(self) -> Result<Grid, PhysicalError> {
        Grid::new(self.width, self.height, self.radius_metres)
            .map_err(PhysicalError::InvalidSettings)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ValidationReport {
    pub actual_land_fraction_ppm: u32,
    pub reference_water_inventory_m3: u64,
    pub coastline_segments: usize,
}

pub trait ProgressSink {
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
    pub on_progress: &'a mut dyn FnMut(ProgressPhase, u32, u32),
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GeneratedWorld {
    pub field: PhysicalField,
    pub tectonics: tectonics::TectonicWorld,
    pub climate: climate::ClimateField,
    pub evolution: evolution::EvolutionField,
    pub hydrology: hydrology::HydrologyField,
    pub source: Vec<u8>,
    pub derived_geojson: String,
    pub report: ValidationReport,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Grid {
    pub width: u32,
    pub height: u32,
    pub radius_metres: u64,
}

impl Grid {
    pub fn new(width: u32, height: u32, radius_metres: u64) -> Result<Self, String> {
        if !(4..=MAX_WIDTH).contains(&width) || !(2..=MAX_HEIGHT).contains(&height) {
            return Err("grid dimensions exceed the iteration-0 bounds".into());
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

    /// Great-circle distance using wrapped longitude and haversine arithmetic.
    pub fn great_circle_distance(self, first: (f64, f64), second: (f64, f64)) -> f64 {
        let mut delta_lon = second.0 - first.0;
        while delta_lon > std::f64::consts::PI {
            delta_lon -= std::f64::consts::TAU;
        }
        while delta_lon < -std::f64::consts::PI {
            delta_lon += std::f64::consts::TAU;
        }
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
        }
    }
}

pub fn derive_subsystem_seed(seed: u32, retry_index: u32, domain: SeedDomain) -> u64 {
    splitmix64(
        u64::from(seed) ^ u64::from(retry_index).wrapping_mul(0xd6e8_feb8_6659_fd93) ^ domain.tag(),
    )
}

fn elevation_for(seed: u32, retry_index: u32, index: usize) -> i32 {
    let state = u64::from(seed)
        ^ (u64::from(retry_index).wrapping_mul(0xd6e8_feb8_6659_fd93))
        ^ (index as u64).wrapping_mul(0x9e37_79b9_7f4a_7c15);
    let random = splitmix64(state);
    let broad = ((random >> 16) % 7_001) as i32 - 3_500;
    let detail = ((random >> 40) % 401) as i32 - 200;
    broad * 100 + detail
}

pub fn generate_field(
    grid: Grid,
    seed: u32,
    retry_index: u32,
    target_land_fraction_ppm: u32,
) -> Result<PhysicalField, String> {
    if target_land_fraction_ppm == 0 || target_land_fraction_ppm >= 1_000_000 {
        return Err("iteration-0 target land fraction must be between 0 and 1".into());
    }
    let elevations_mm = (0..grid.sample_count())
        .map(|index| elevation_for(seed, retry_index, index))
        .collect::<Vec<_>>();
    let sea_level_mm = solve_sea_level(&grid, &elevations_mm, target_land_fraction_ppm)?;
    let field = PhysicalField {
        grid,
        seed,
        retry_index,
        target_land_fraction_ppm,
        sea_level_mm,
        elevations_mm,
    };
    field.validate()?;
    Ok(field)
}

fn reference_water_inventory_m3(field: &PhysicalField) -> Result<u64, PhysicalError> {
    let mut inventory = 0.0_f64;
    for (index, elevation) in field.elevations_mm.iter().enumerate() {
        let depth_mm = i64::from(field.sea_level_mm) - i64::from(*elevation);
        if depth_mm <= 0 {
            continue;
        }
        let row = field.grid.row_col(index).0;
        inventory += field.grid.cell_area(row) * depth_mm as f64 / 1_000.0;
    }
    if !inventory.is_finite() || inventory < 0.0 || inventory > u64::MAX as f64 {
        return Err(PhysicalError::Validation(
            "reference water inventory is not finite or bounded".into(),
        ));
    }
    Ok(inventory.round() as u64)
}

pub fn validate_field_report(field: &PhysicalField) -> Result<ValidationReport, PhysicalError> {
    field.validate().map_err(PhysicalError::InvalidSource)?;
    let actual = land_fraction(&field.grid, &field.elevations_mm, field.sea_level_mm);
    if !actual.is_finite() {
        return Err(PhysicalError::Validation(
            "land fraction is not finite".into(),
        ));
    }
    let target = f64::from(field.target_land_fraction_ppm) / 1_000_000.0;
    if (actual - target).abs() > 0.08 {
        return Err(PhysicalError::Validation(format!(
            "land fraction is outside the target tolerance: actual={actual:.6} target={target:.6}"
        )));
    }
    let segments = coastline_segments(field);
    if segments.len() > MAX_GEOJSON_FEATURES {
        return Err(PhysicalError::Validation(
            "coastline exceeds the feature budget".into(),
        ));
    }
    for segment in &segments {
        for point in [segment.first, segment.second] {
            if !(-180_000_000..=180_000_000).contains(&point[0])
                || !(-90_000_000..=90_000_000).contains(&point[1])
            {
                return Err(PhysicalError::Validation(
                    "coastline contains an out-of-range coordinate".into(),
                ));
            }
        }
    }
    Ok(ValidationReport {
        actual_land_fraction_ppm: (actual * 1_000_000.0).round() as u32,
        reference_water_inventory_m3: reference_water_inventory_m3(field)?,
        coastline_segments: segments.len(),
    })
}

pub fn generate_world(
    settings: GenerationSettings,
    seed: u32,
    retry_index: u32,
    progress: &mut dyn ProgressSink,
) -> Result<GeneratedWorld, PhysicalError> {
    generate_world_with_evolution(
        settings,
        seed,
        retry_index,
        evolution::EvolutionSettings::default(),
        progress,
    )
}

pub fn generate_world_with_evolution(
    settings: GenerationSettings,
    seed: u32,
    retry_index: u32,
    evolution_settings: evolution::EvolutionSettings,
    progress: &mut dyn ProgressSink,
) -> Result<GeneratedWorld, PhysicalError> {
    let grid = settings.grid()?;
    let tectonic_settings = tectonics::TectonicSettings::default_for(grid);
    let mut tectonics = tectonics::generate_tectonic_world(
        grid,
        tectonic_settings,
        settings.target_land_fraction_ppm,
        seed,
        retry_index,
        progress,
    )?;
    let initial_elevations_mm = tectonics.elevations_mm.clone();
    let initial_sea_level_mm = solve_sea_level(
        &grid,
        &initial_elevations_mm,
        settings.target_land_fraction_ppm,
    )
    .map_err(|error| PhysicalError::coded(PhysicalErrorCode::WaterNonConvergent, error))?;
    let initial_field = PhysicalField {
        grid,
        seed,
        retry_index,
        target_land_fraction_ppm: settings.target_land_fraction_ppm,
        sea_level_mm: initial_sea_level_mm,
        elevations_mm: initial_elevations_mm,
    };
    let initial_climate = climate::derive_current_climate(
        &initial_field,
        climate::ClimateSettings::default_for(grid),
        seed,
        retry_index,
        progress,
    )?;
    let mut evolution = evolution::evolve_terrain(
        &initial_field,
        &initial_climate,
        &tectonics,
        evolution_settings,
        progress,
    )?;
    let elevations_mm = evolution.elevations_mm.clone();
    let sea_level_mm = solve_sea_level(&grid, &elevations_mm, settings.target_land_fraction_ppm)
        .map_err(|error| PhysicalError::coded(PhysicalErrorCode::WaterNonConvergent, error))?;
    tectonics.target_land_fraction_ppm = settings.target_land_fraction_ppm;
    tectonics.sea_level_mm = sea_level_mm;
    tectonics.elevations_mm = elevations_mm.clone();
    let field = PhysicalField {
        grid,
        seed,
        retry_index,
        target_land_fraction_ppm: settings.target_land_fraction_ppm,
        sea_level_mm,
        elevations_mm,
    };
    let climate = climate::derive_current_climate(
        &field,
        climate::ClimateSettings::default_for(grid),
        seed,
        retry_index,
        progress,
    )?;
    let final_drainage = evolution::derive_drainage(&field, &climate)?;
    evolution.replace_drainage(final_drainage);
    evolution.validate_against(&initial_field, &field, &climate)?;
    let report = validate_field_report(&field)?;
    progress.report(ProgressPhase::CalculatingWater, 0, 1)?;
    let hydrology = hydrology::derive_hydrology_with_crust(
        &field,
        &climate,
        &evolution.drainage,
        report.reference_water_inventory_m3,
        Some(&tectonics.crust_by_cell),
    )?;
    progress.report(ProgressPhase::CalculatingWater, 1, 1)?;
    progress.report(ProgressPhase::BuildingRiversAndLakes, 0, 1)?;
    progress.report(ProgressPhase::BuildingRiversAndLakes, 1, 1)?;
    progress.report(ProgressPhase::PreparingGeography, 0, 1)?;
    let diagnostic_geojson = tectonics::to_diagnostic_geojson(&tectonics)
        .map_err(|error| PhysicalError::coded(PhysicalErrorCode::GeometryInvalid, error))?;
    let hydrology_geojson = hydrology::to_geojson(&hydrology)
        .map_err(|error| PhysicalError::coded(PhysicalErrorCode::GeometryInvalid, error))?;
    let derived_geojson = merge_geojson_features_for_host(&diagnostic_geojson, &hydrology_geojson)
        .map_err(|error| PhysicalError::coded(PhysicalErrorCode::GeometryInvalid, error))?;
    progress.report(ProgressPhase::PreparingGeography, 1, 1)?;
    progress.report(ProgressPhase::ValidatingWorld, 0, 1)?;
    let source = tectonics::encode_source_v2(&tectonics).map_err(PhysicalError::InvalidSource)?;
    progress.report(ProgressPhase::ValidatingWorld, 1, 1)?;
    Ok(GeneratedWorld {
        field,
        tectonics,
        climate,
        evolution,
        hydrology,
        source,
        derived_geojson,
        report,
    })
}

pub fn merge_geojson_features_for_host(first: &str, second: &str) -> Result<String, String> {
    let first_prefix = r#"{"type":"FeatureCollection","features":["#;
    let second_prefix = r#"{"type":"FeatureCollection","features":["#;
    let first_features = first
        .strip_prefix(first_prefix)
        .and_then(|value| value.strip_suffix("]}"))
        .ok_or_else(|| "diagnostic GeoJSON has an invalid feature collection shape".to_string())?;
    let second_features = second
        .strip_prefix(second_prefix)
        .and_then(|value| value.strip_suffix("]}"))
        .ok_or_else(|| "hydrology GeoJSON has an invalid feature collection shape".to_string())?;
    let joined = match (first_features.is_empty(), second_features.is_empty()) {
        (true, true) => String::new(),
        (true, false) => second_features.to_string(),
        (false, true) => first_features.to_string(),
        (false, false) => format!("{first_features},{second_features}"),
    };
    Ok(format!(
        r#"{{"type":"FeatureCollection","features":[{joined}]}}"#
    ))
}

pub fn land_fraction(grid: &Grid, elevations_mm: &[i32], sea_level_mm: i32) -> f64 {
    let total = grid.total_area();
    elevations_mm
        .iter()
        .enumerate()
        .filter(|(_, elevation)| **elevation > sea_level_mm)
        .map(|(index, _)| grid.cell_area(grid.row_col(index).0))
        .sum::<f64>()
        / total
}

pub fn solve_sea_level(
    grid: &Grid,
    elevations_mm: &[i32],
    target_land_fraction_ppm: u32,
) -> Result<i32, String> {
    if elevations_mm.len() != grid.sample_count() {
        return Err("sea-level input does not match the grid".into());
    }
    let mut candidates = elevations_mm.to_vec();
    candidates.sort_unstable();
    candidates.dedup();
    let minimum = candidates
        .first()
        .copied()
        .ok_or_else(|| "sea-level input is empty".to_string())?;
    candidates.insert(0, minimum.saturating_sub(1));
    candidates.push(
        candidates
            .last()
            .copied()
            .unwrap_or(minimum)
            .saturating_add(1),
    );
    let target = f64::from(target_land_fraction_ppm) / 1_000_000.0;
    candidates
        .into_iter()
        .min_by(|first, second| {
            let first_error = (land_fraction(grid, elevations_mm, *first) - target).abs();
            let second_error = (land_fraction(grid, elevations_mm, *second) - target).abs();
            first_error
                .partial_cmp(&second_error)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| first.cmp(second))
        })
        .ok_or_else(|| "sea-level candidates are empty".into())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Segment {
    pub first: [i32; 2],
    pub second: [i32; 2],
}

fn longitude_micro(grid: &Grid, column: u32) -> i32 {
    (-180_000_000i64 + 360_000_000i64 * i64::from(column) / i64::from(grid.width)) as i32
}

fn latitude_micro(grid: &Grid, row: u32) -> i32 {
    (-90_000_000i64 + 180_000_000i64 * i64::from(row) / i64::from(grid.height)) as i32
}

fn edge_segments_for_cell(grid: &Grid, row: u32, col: u32) -> [[[i32; 2]; 2]; 4] {
    let west = longitude_micro(grid, col);
    let east = longitude_micro(grid, col + 1);
    let south = latitude_micro(grid, row);
    let north = latitude_micro(grid, row + 1);
    [
        [[west, south], [west, north]],
        [[east, north], [east, south]],
        [[west, south], [east, south]],
        [[east, north], [west, north]],
    ]
}

pub fn coastline_segments(field: &PhysicalField) -> Vec<Segment> {
    let grid = field.grid;
    let mut segments = Vec::new();
    let is_land = |index: usize| field.elevations_mm[index] > field.sea_level_mm;
    for row in 0..grid.height {
        for col in 0..grid.width {
            let index = grid.index(row, col);
            if !is_land(index) {
                continue;
            }
            let edges = edge_segments_for_cell(&grid, row, col);
            let west = grid.index(row, (col + grid.width - 1) % grid.width);
            let east = grid.index(row, (col + 1) % grid.width);
            if !is_land(west) {
                segments.push(Segment {
                    first: edges[0][0],
                    second: edges[0][1],
                });
            }
            if !is_land(east) {
                segments.push(Segment {
                    first: edges[1][0],
                    second: edges[1][1],
                });
            }
            if row == 0 {
                let polar_land = (0..grid.width).all(|polar_col| is_land(grid.index(0, polar_col)));
                if !polar_land {
                    segments.push(Segment {
                        first: edges[2][0],
                        second: [0, -90_000_000],
                    });
                }
            } else if !is_land(grid.index(row - 1, col)) {
                segments.push(Segment {
                    first: edges[2][0],
                    second: edges[2][1],
                });
            }
            if row + 1 == grid.height {
                let polar_land = (0..grid.width)
                    .all(|polar_col| is_land(grid.index(grid.height - 1, polar_col)));
                if !polar_land {
                    segments.push(Segment {
                        first: edges[3][0],
                        second: [0, 90_000_000],
                    });
                }
            } else if !is_land(grid.index(row + 1, col)) {
                segments.push(Segment {
                    first: edges[3][0],
                    second: edges[3][1],
                });
            }
        }
    }
    segments
}

pub fn to_geojson(field: &PhysicalField) -> Result<String, String> {
    let segments = coastline_segments(field);
    if segments.len() > MAX_GEOJSON_FEATURES {
        return Err("derived coastline exceeds the iteration-0 feature budget".into());
    }
    let mut output = String::from(r#"{"type":"FeatureCollection","features":["#);
    for (index, segment) in segments.iter().enumerate() {
        if index > 0 {
            output.push(',');
        }
        write!(
            output,
            r#"{{"type":"Feature","id":"coastline-{index:05}","properties":{{"daenaLayerId":"base","kind":"custom","name":"physical coastline"}},"geometry":{{"type":"LineString","coordinates":[[{},{}],[{},{}]]}}}}"#,
            format_micro(segment.first[0]),
            format_micro(segment.first[1]),
            format_micro(segment.second[0]),
            format_micro(segment.second[1]),
        )
        .map_err(|_| "failed to format derived GeoJSON".to_string())?;
    }
    output.push_str("]}");
    Ok(output)
}

fn format_micro(value: i32) -> String {
    let sign = if value < 0 { "-" } else { "" };
    let absolute = value.unsigned_abs();
    let whole = absolute / 1_000_000;
    let fraction = absolute % 1_000_000;
    if fraction == 0 {
        format!("{sign}{whole}")
    } else {
        let mut digits = format!("{fraction:06}");
        while digits.ends_with('0') {
            digits.pop();
        }
        format!("{sign}{whole}.{digits}")
    }
}

fn push_u16(output: &mut Vec<u8>, value: u16) {
    output.extend_from_slice(&value.to_le_bytes());
}

fn push_u32(output: &mut Vec<u8>, value: u32) {
    output.extend_from_slice(&value.to_le_bytes());
}

fn push_u64(output: &mut Vec<u8>, value: u64) {
    output.extend_from_slice(&value.to_le_bytes());
}

fn push_i32(output: &mut Vec<u8>, value: i32) {
    output.extend_from_slice(&value.to_le_bytes());
}

fn read_exact<const N: usize>(bytes: &[u8], offset: &mut usize) -> Result<[u8; N], String> {
    let end = offset
        .checked_add(N)
        .ok_or_else(|| "source offset overflow".to_string())?;
    let value = bytes
        .get(*offset..end)
        .ok_or_else(|| "source is truncated".to_string())?;
    *offset = end;
    value
        .try_into()
        .map_err(|_| "source field has an invalid width".to_string())
}

pub fn encode_source(world: &tectonics::TectonicWorld) -> Result<Vec<u8>, String> {
    tectonics::encode_source_v2(world)
}

pub fn decode_source(bytes: &[u8]) -> Result<tectonics::TectonicWorld, String> {
    tectonics::decode_source_v2(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> GeneratedWorld {
        let settings = GenerationSettings {
            width: DEFAULT_WIDTH,
            height: DEFAULT_HEIGHT,
            radius_metres: DEFAULT_RADIUS_METRES,
            target_land_fraction_ppm: 300_000,
        };
        let mut progress = NoopProgress;
        generate_world(settings, 831_429, 0, &mut progress).unwrap()
    }

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
    fn sea_level_is_monotonic_and_hits_the_target_band() {
        let grid = Grid::new(32, 16, DEFAULT_RADIUS_METRES).unwrap();
        let field = generate_field(grid, 831_429, 0, 300_000).unwrap();
        let lower = generate_field(grid, 831_429, 0, 200_000).unwrap();
        let higher = generate_field(grid, 831_429, 0, 500_000).unwrap();
        assert!(lower.sea_level_mm > field.sea_level_mm);
        assert!(higher.sea_level_mm < field.sea_level_mm);
        let actual = land_fraction(&grid, &field.elevations_mm, field.sea_level_mm);
        assert!((actual - 0.3).abs() < 0.04, "land fraction: {actual}");
    }

    #[test]
    fn source_round_trip_is_byte_exact_and_strict() {
        let world = fixture();
        let encoded = encode_source(&world.tectonics).unwrap();
        assert_eq!(encoded, world.source);
        assert_eq!(
            u16::from_le_bytes(encoded[8..10].try_into().unwrap()),
            SOURCE_VERSION
        );
        assert_eq!(
            u16::from_le_bytes(encoded[10..12].try_into().unwrap()) as usize,
            SOURCE_HEADER_BYTES
        );
        assert_eq!(decode_source(&encoded).unwrap(), world.tectonics);
        assert_eq!(
            encode_source(&decode_source(&encoded).unwrap()).unwrap(),
            encoded
        );
        assert!(decode_source(&encoded[..encoded.len() - 1]).is_err());
        let mut trailing = encoded.clone();
        trailing.push(0);
        assert!(decode_source(&trailing).is_err());
        let mut duplicate_header = encoded.clone();
        duplicate_header[0] = b'X';
        assert!(decode_source(&duplicate_header).is_err());
        let mut old_version = encoded.clone();
        old_version[8..10].copy_from_slice(&1u16.to_le_bytes());
        assert!(decode_source(&old_version).is_err());
    }

    #[test]
    fn coastline_and_geojson_are_bounded_and_seam_safe() {
        let world = fixture();
        let segments = coastline_segments(&world.field);
        assert!(!segments.is_empty());
        assert!(segments.len() <= MAX_GEOJSON_FEATURES);
        for segment in &segments {
            for point in [segment.first, segment.second] {
                assert!((-180_000_000..=180_000_000).contains(&point[0]));
                assert!((-90_000_000..=90_000_000).contains(&point[1]));
            }
        }
        let geojson = to_geojson(&world.field).unwrap();
        assert!(geojson.starts_with(r#"{"type":"FeatureCollection","features":["#));
        assert!(!geojson.contains("http://"));
        assert!(!geojson.contains("https://"));
        assert!(geojson.len() < 4 * 1024 * 1024);
    }

    #[test]
    fn world_pipeline_reports_inventory_and_honors_cancellation() {
        let settings = GenerationSettings {
            width: 16,
            height: 8,
            radius_metres: DEFAULT_RADIUS_METRES,
            target_land_fraction_ppm: 300_000,
        };
        let mut progress = NoopProgress;
        let world = generate_world(settings, 831_429, 0, &mut progress).unwrap();
        assert!(world.report.reference_water_inventory_m3 > 0);
        assert!(world.report.coastline_segments > 0);
        assert_eq!(world.climate.grid, world.field.grid);
        assert!(world.climate.metrics.precipitation_volume_m3_per_year > 0);
        assert!(world.climate.metrics.runoff_volume_m3_per_year > 0);
        assert!(world.climate.metrics.transport_iterations > 0);
        assert_eq!(world.evolution.elevations_mm, world.field.elevations_mm);
        assert_eq!(world.tectonics.elevations_mm, world.field.elevations_mm);
        assert_ne!(
            world.evolution.before_elevations_mm,
            world.evolution.elevations_mm
        );
        assert_eq!(
            world.evolution.drainage.metrics.direct_runoff_m3_per_year,
            world.evolution.drainage.metrics.routed_runoff_m3_per_year
        );
        assert_eq!(encode_source(&world.tectonics).unwrap(), world.source);
        let source_before_derived_disposal = world.source.clone();
        let mut climate_progress = NoopProgress;
        let disposable_climate = climate::derive_current_climate(
            &world.field,
            climate::ClimateSettings::default_for(world.field.grid),
            world.field.seed,
            world.field.retry_index,
            &mut climate_progress,
        )
        .unwrap();
        drop(disposable_climate);
        assert_eq!(
            encode_source(&world.tectonics).unwrap(),
            source_before_derived_disposal
        );
        assert!(world.derived_geojson.contains("physical coastline"));

        let cancelled = AtomicBool::new(true);
        let mut updates = |_phase: ProgressPhase, _completed: u32, _total: u32| {};
        let mut progress = CancellationProgress {
            cancelled: &cancelled,
            on_progress: &mut updates,
        };
        assert_eq!(
            generate_world(settings, 831_429, 0, &mut progress),
            Err(PhysicalError::Cancelled)
        );

        let settings = GenerationSettings {
            width: MAX_WIDTH,
            height: MAX_HEIGHT,
            radius_metres: DEFAULT_RADIUS_METRES,
            target_land_fraction_ppm: 300_000,
        };
        let cancelled = AtomicBool::new(false);
        let mut updates = |_: ProgressPhase, _, _| cancelled.store(true, Ordering::Relaxed);
        let mut progress = CancellationProgress {
            cancelled: &cancelled,
            on_progress: &mut updates,
        };
        let started = std::time::Instant::now();
        assert_eq!(
            generate_world(settings, 831_429, 0, &mut progress),
            Err(PhysicalError::Cancelled)
        );
        assert!(started.elapsed().as_millis() < CANCELLATION_LATENCY_BUDGET_MS);
    }
}
