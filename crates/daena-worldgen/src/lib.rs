//! Cube-sphere planetary terrain: analytic tectonics, erosion and climate.
//!
//! Phase A builds a deterministic world on a nested cube-sphere, from a plate
//! model at `min(n, 256)` down to an eroded surface at the requested `n`.
//! Phase B1 reads that surface and builds a seasonal climate. Stored cell
//! fields are `f32`. Geometry and area sums use `f64`.

#![deny(unsafe_code)]

/// Engine output contract version. Any output change bumps this.
pub const WORLD_ENGINE_VERSION: u32 = 2;

mod biome;
mod climate;
mod evolution;
mod grid;
mod hazards;
mod hydrology;
mod noise;
mod par;
mod planet;
mod preset;
mod render;
mod routing;
mod sealevel;
mod tectonics;

pub use biome::Biome;
pub use climate::{
    ClimateField, ClimateStats, ClimateSummary, NH_SUMMER_SOLSTICE, NH_WINTER_SOLSTICE,
};
pub use grid::{downsample, downsample_max, resample, upsample, CubeSphere, FACE_BASIS, NONE};
pub use hazards::{
    trace_storm, HazardStats, Hazards, StormClimatology, StormStep, StormTrack, VolcanicActivity,
    VolcanicOrigin, VolcanicSource,
};
pub use hydrology::{Hydrology, Lake, LakeKind, RiverEnd, RiverSegment};
pub use noise::{derive_seed, fbm, hash3, ridged, splitmix64, warp, SeedDomain};
pub use planet::PlanetSettings;
pub use preset::Preset;
pub use render::{
    hillshade, render_equirect, render_equirect_with_shade, render_orthographic,
    render_orthographic_with_shade, write_png, Layer,
};
pub use sealevel::{
    ocean_volume_at, ocean_volume_curve, sea_level_for_ocean_volume, OceanVolumeSample,
};
pub use tectonics::{BoundaryKind, BoundaryRecord, Hotspot, Plate, SubductSide, Tectonics};

use std::time::Instant;

use rayon::prelude::*;
use routing::route_with_cancel;
use routing::RoutingScratch;

/// How continents are arranged before the area threshold is applied.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ContinentLayout {
    /// One large landmass with a few attached pieces.
    Pangea,
    /// Two to four broad masses.
    Few,
    /// Six to ten smaller masses.
    Many,
    /// Deterministic choice of the other three from the world seed.
    #[default]
    Mixed,
}

impl ContinentLayout {
    /// Stable name used by the CLI and the JSON report.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Pangea => "pangea",
            Self::Few => "few",
            Self::Many => "many",
            Self::Mixed => "mixed",
        }
    }

    /// Parse `pangea`, `few`, `many` or `mixed`.
    pub fn parse(text: &str) -> Option<Self> {
        match text {
            "pangea" => Some(Self::Pangea),
            "few" => Some(Self::Few),
            "many" => Some(Self::Many),
            "mixed" => Some(Self::Mixed),
            _ => None,
        }
    }
}

/// One row of the erosion schedule: every level with `n <= max_n` may use it.
#[derive(Clone, Copy, Debug)]
pub struct ScheduleStep {
    /// Largest grid `n` this row applies to.
    pub max_n: u32,
    /// Erosion iterations at that level.
    pub iterations: u32,
    /// Time step in years.
    pub dt_yr: f64,
}

/// Iterations and time steps from the coarse grid to the final one.
#[derive(Clone, Debug)]
pub struct Schedule {
    /// Rows in increasing `max_n`. The first row with `n <= max_n` wins.
    pub steps: Vec<ScheduleStep>,
}

impl Default for Schedule {
    fn default() -> Self {
        Self {
            steps: vec![
                ScheduleStep {
                    max_n: 64,
                    iterations: 30,
                    dt_yr: 1.0e6,
                },
                ScheduleStep {
                    max_n: 128,
                    iterations: 20,
                    dt_yr: 5.0e5,
                },
                ScheduleStep {
                    max_n: 256,
                    iterations: 12,
                    dt_yr: 2.5e5,
                },
                ScheduleStep {
                    max_n: 512,
                    iterations: 6,
                    dt_yr: 1.5e5,
                },
                ScheduleStep {
                    max_n: 1024,
                    iterations: 4,
                    dt_yr: 1.0e5,
                },
                ScheduleStep {
                    max_n: 2048,
                    iterations: 3,
                    dt_yr: 6.0e4,
                },
            ],
        }
    }
}

impl Schedule {
    /// Schedule row for grid resolution `n`.
    pub fn step_for(&self, n: u32) -> Option<ScheduleStep> {
        self.steps.iter().copied().find(|step| n <= step.max_n)
    }
}

/// Stream-power and hillslope parameters. All values are finite and positive.
#[derive(Clone, Copy, Debug)]
pub struct ErosionParams {
    /// Drainage-area exponent `m`.
    pub m: f64,
    /// Stream-power coefficient `K`, multiplied by `k_rock`.
    pub k: f64,
    /// Threshold slope `S_c`, dimensionless rise over run.
    pub s_c: f64,
    /// Jacobi smoothing fraction per iteration.
    pub kappa: f64,
    /// Elevation cap in metres.
    pub max_elevation_m: f64,
}

impl Default for ErosionParams {
    fn default() -> Self {
        Self {
            m: 0.45,
            k: 4.2e-5,
            s_c: 0.35,
            kappa: 0.02,
            max_elevation_m: 9500.0,
        }
    }
}

/// Inputs for [`generate`].
#[derive(Clone, Debug)]
pub struct WorldSettings {
    /// Master seed. Every random choice is mixed with a [`SeedDomain`].
    pub seed: u64,
    /// Final cube-sphere edge, a power of two in `32..=2048`.
    pub resolution: u32,
    /// Planet radius in metres.
    pub radius_m: f64,
    /// Target fraction of the sphere above sea level.
    pub land_fraction: f64,
    /// Number of plates, in `2..=48`.
    pub plate_count: u32,
    /// Continental arrangement.
    pub continent_layout: ContinentLayout,
    /// Scales plate speeds. `1` is the reference.
    pub tectonic_activity: f64,
    /// Per-level iteration counts and time steps.
    pub schedule: Schedule,
    /// Erosion coefficients.
    pub erosion: ErosionParams,
    /// Star, orbit and climate knobs.
    pub planet: PlanetSettings,
    /// Discharge at or above this, in m³/s, is a river. Default 100.
    pub river_min_m3s: f64,
}

impl Default for WorldSettings {
    fn default() -> Self {
        Self {
            seed: 1,
            resolution: 512,
            radius_m: 6_371_000.0,
            land_fraction: 0.29,
            plate_count: 14,
            continent_layout: ContinentLayout::Mixed,
            tectonic_activity: 1.0,
            schedule: Schedule::default(),
            erosion: ErosionParams::default(),
            planet: PlanetSettings::default(),
            river_min_m3s: 100.0,
        }
    }
}

impl WorldSettings {
    /// Reject values outside the phase A ranges.
    pub fn validate(&self) -> Result<(), WorldError> {
        if !self.resolution.is_power_of_two() || !(32..=2048).contains(&self.resolution) {
            return Err(WorldError::InvalidSettings(format!(
                "resolution must be a power of two in [32, 2048], got {}",
                self.resolution
            )));
        }
        if !self.radius_m.is_finite() || !(1.0e6..=2.0e7).contains(&self.radius_m) {
            return Err(WorldError::InvalidSettings(format!(
                "radius_m must be in [1e6, 2e7], got {}",
                self.radius_m
            )));
        }
        if !self.land_fraction.is_finite() || !(0.02..=0.9).contains(&self.land_fraction) {
            return Err(WorldError::InvalidSettings(format!(
                "land_fraction must be in [0.02, 0.9], got {}",
                self.land_fraction
            )));
        }
        if !(2..=48).contains(&self.plate_count) {
            return Err(WorldError::InvalidSettings(format!(
                "plate_count must be in [2, 48], got {}",
                self.plate_count
            )));
        }
        if !self.tectonic_activity.is_finite() || !(0.2..=2.0).contains(&self.tectonic_activity) {
            return Err(WorldError::InvalidSettings(format!(
                "tectonic_activity must be in [0.2, 2.0], got {}",
                self.tectonic_activity
            )));
        }
        if self.schedule.steps.is_empty() {
            return Err(WorldError::InvalidSettings(
                "schedule must contain at least one step".to_string(),
            ));
        }
        for step in &self.schedule.steps {
            if step.iterations < 1 {
                return Err(WorldError::InvalidSettings(
                    "schedule iterations must be >= 1".to_string(),
                ));
            }
            if !step.dt_yr.is_finite() || step.dt_yr <= 0.0 {
                return Err(WorldError::InvalidSettings(
                    "schedule dt must be finite and > 0".to_string(),
                ));
            }
        }
        if self.schedule.step_for(self.resolution).is_none() {
            return Err(WorldError::InvalidSettings(format!(
                "schedule does not cover resolution {}",
                self.resolution
            )));
        }
        for (name, value) in [
            ("erosion.m", self.erosion.m),
            ("erosion.k", self.erosion.k),
            ("erosion.s_c", self.erosion.s_c),
            ("erosion.kappa", self.erosion.kappa),
            ("erosion.max_elevation_m", self.erosion.max_elevation_m),
        ] {
            if !value.is_finite() || value <= 0.0 {
                return Err(WorldError::InvalidSettings(format!(
                    "{name} must be finite and positive, got {value}"
                )));
            }
        }
        self.planet.validate()?;
        if !self.river_min_m3s.is_finite() || self.river_min_m3s < 0.0 {
            return Err(WorldError::InvalidSettings(format!(
                "river_min_m3s must be finite and >= 0, got {}",
                self.river_min_m3s
            )));
        }
        Ok(())
    }
}

/// Which part of [`generate`] is running.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stage {
    /// Plates, crust and the pre-erosion surface.
    Tectonics,
    /// One erosion iteration at resolution `n`.
    Level(u32),
    /// Sea-level solve and ocean flood.
    SeaLevel,
    /// Drainage on the shifted surface.
    FinalRouting,
    /// Seasonal climate on the finished surface.
    Climate,
    /// Lakes, rivers and ice on the finished surface.
    Hydrology,
    /// Biome classes.
    Biomes,
    /// Hazard rates and potentials on the finished world.
    Hazards,
}

/// Receives stage updates and can stop the build.
///
/// `Send + Sync` lets [`generate`] run on a rayon worker and lets both
/// branches of a `rayon::join` poll [`Self::cancelled`]. Reports stay on the
/// calling thread.
pub trait Progress: Send + Sync {
    /// `done` counts completed units of `total` for `stage`.
    fn report(&mut self, stage: Stage, done: u32, total: u32);
    /// Return true to abort with [`WorldError::Cancelled`].
    fn cancelled(&self) -> bool {
        false
    }
}

/// Progress sink that ignores reports and never cancels.
#[derive(Clone, Copy, Debug, Default)]
pub struct NoProgress;

impl Progress for NoProgress {
    fn report(&mut self, _stage: Stage, _done: u32, _total: u32) {}
}

/// Fail with [`WorldError::Cancelled`] when the sink asks to stop.
pub fn check_cancelled(progress: &dyn Progress) -> Result<(), WorldError> {
    if progress.cancelled() {
        Err(WorldError::Cancelled)
    } else {
        Ok(())
    }
}

/// Failure from settings, cancellation, or an internal invariant.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WorldError {
    /// A setting is outside its allowed range.
    InvalidSettings(String),
    /// [`Progress::cancelled`] returned true.
    Cancelled,
    /// A grid or field invariant failed.
    Internal(String),
}

impl std::fmt::Display for WorldError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidSettings(msg) => write!(f, "invalid settings: {msg}"),
            Self::Cancelled => write!(f, "cancelled"),
            Self::Internal(msg) => write!(f, "internal error: {msg}"),
        }
    }
}

impl std::error::Error for WorldError {}

/// One erosion level.
#[derive(Clone, Copy, Debug)]
pub struct LevelStats {
    /// Grid edge.
    pub n: u32,
    /// Iterations that ran.
    pub iterations: u32,
    /// Domain pits found in the last iteration, before carving.
    pub pits: u32,
    /// Domain pits in the first iteration, before carving.
    pub pits_first: u32,
    /// Basins carved in the last iteration.
    pub carved_basins: u32,
    /// Largest `edges.len()` built while carving this level.
    pub edges_peak: u32,
    /// Highest domain cell after the last iteration, metres.
    pub max_elevation_m: f32,
    /// Area-weighted mean domain elevation, metres.
    pub mean_elevation_m: f32,
    /// Cells clamped to the elevation cap in the last iteration.
    pub clamped_cells: u32,
    /// Pit basins closed by evaporation in the last iteration.
    pub closed_basins: u32,
    /// Overflowing closed basins drained by the final-level outlet cut.
    pub breached_basins: u32,
    /// Basins kept closed by active subsidence.
    pub kept_subsiding_basins: u32,
    /// Basins kept closed by glacial overdeepening.
    pub kept_glacial_basins: u32,
    /// Basins kept closed because evaporation beats inflow.
    pub kept_arid_basins: u32,
    /// Domain cells with positive ice discharge in the last iteration.
    pub glaciated_cells: u32,
    /// Wall time for the level, milliseconds. Excludes the climate pass.
    pub wall_ms: f64,
}

/// One climate integration used to drive erosion, or the final surface.
#[derive(Clone, Debug)]
pub struct ClimatePass {
    /// `level_64`, `level_256`, or `final`.
    pub label: &'static str,
    /// Edge of the surface this pass read.
    pub source_n: u32,
    /// Years integrated, including the year that converged.
    pub spinup_years: u32,
    /// True when the same-month change fell below 0.02 K.
    pub converged: bool,
    /// Largest same-month change on the last raw year, K.
    pub residual_k: f64,
    /// Area-weighted RMS same-month change on the last raw year, K.
    pub rms_k: f64,
    /// Wall time for the coarse spin-up, milliseconds.
    pub wall_ms: f64,
}

/// Summary numbers for the finished world.
#[derive(Clone, Debug)]
pub struct WorldStats {
    /// Area fraction with elevation strictly above 0.
    pub achieved_land_fraction: f64,
    /// Domain cells at or below sea level that are not ocean.
    pub dry_depressions: u32,
    /// Lowest elevation in metres.
    pub elevation_min_m: f32,
    /// Highest elevation in metres.
    pub elevation_max_m: f32,
    /// Median elevation of cells above sea level, metres.
    pub land_p50_m: f32,
    /// 90th percentile of land elevation, metres.
    pub land_p90_m: f32,
    /// 99th percentile of land elevation, metres.
    pub land_p99_m: f32,
    /// Per-level erosion stats, coarse to fine.
    pub levels: Vec<LevelStats>,
}

/// A generated planet. Elevations are metres relative to sea level.
#[derive(Clone, Debug)]
pub struct World {
    /// Settings that produced this world.
    pub settings: WorldSettings,
    /// Final cube-sphere.
    pub grid: CubeSphere,
    /// Elevation relative to sea level, metres.
    pub elevation_m: Vec<f32>,
    /// Pre-erosion surface on the final grid, not shifted, metres.
    pub base_m: Vec<f32>,
    /// Uplift on the final grid, metres per year.
    pub uplift_m_yr: Vec<f32>,
    /// Erodibility multiplier on the final grid.
    pub k_rock: Vec<f32>,
    /// Plate index per final cell.
    pub plate: Vec<u8>,
    /// `1` continental, `0` oceanic.
    pub crust: Vec<u8>,
    /// `1` if the cell is ocean at or below sea level.
    pub ocean: Vec<u8>,
    /// Downstream cell after the final routing, or self at a root.
    pub receiver: Vec<u32>,
    /// Upstream area in square metres, including the cell itself.
    pub drainage_area_m2: Vec<f32>,
    /// Tectonic model on `min(resolution, 256)`.
    pub tectonics: Tectonics,
    /// Metres added to elevation when sea level was set to 0. Negative when land was lowered.
    pub sea_level_shift_m: f32,
    /// Land fraction, percentiles and per-level stats.
    pub stats: WorldStats,
    /// Full-resolution annual climate.
    pub climate: ClimateSummary,
    /// Coarse monthly climate used to build [`Self::climate`].
    pub climate_field: ClimateField,
    /// Lakes, rivers, discharge and ice. Elevation is unchanged.
    pub hydrology: Hydrology,
    /// One [`Biome`] discriminant per cell.
    pub biome: Vec<u8>,
    /// Long-term hazard rates and potentials.
    pub hazards: Hazards,
    /// Phase timings in milliseconds. `total` is wall time and includes
    /// climate, water and biomes. Spans may overlap (the final coarse climate
    /// runs beside final routing and the lake flood), so the individual
    /// entries do not sum to `total`.
    pub timings: Vec<(&'static str, f64)>,
    /// Evolution climate passes, then the final pass. Spin-up years are per pass.
    pub climate_passes: Vec<ClimatePass>,
}

/// Hashes stored beside a canonical source.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GenerationHashes {
    /// FNV-1a over the source `i32` millimetres, not the `f32` bits.
    pub elevation_mm: u64,
    /// FNV-1a over the regenerated tectonic field.
    pub tectonics: u64,
    /// [`ClimateSummary::hash_bits`].
    pub climate: u64,
    /// [`Hydrology::hash_bits`].
    pub hydrology: u64,
    /// FNV-1a over the biome ids.
    pub biome: u64,
    /// FNV-1a over the hazard fields and sources.
    pub hazards: u64,
}

/// Climate values at one point.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ClimatePoint {
    pub t_annual_c: f32,
    pub t_warm_c: f32,
    pub t_cold_c: f32,
    pub t_nh_summer_c: f32,
    pub t_nh_winter_c: f32,
    pub p_annual_mm: f32,
    pub p_nh_summer_mm: f32,
    pub p_nh_winter_mm: f32,
    pub p_dry_month_mm: f32,
    pub pet_annual_mm: f32,
    pub runoff_mm: f32,
    pub ice_balance_mm: f32,
    pub sea_ice_months: u8,
    pub wind_east_milli: i32,
    pub wind_north_milli: i32,
    pub aridity_ppm: u32,
    pub maritime_ppm: u32,
}

/// Hydrology values at one point.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HydrologyPoint {
    pub lake_id: u32,
    pub lake_level_m: f32,
    pub discharge_m3s: f32,
    pub ice_cover: u8,
    pub ocean: bool,
}

/// Hazard values at one point.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HazardPoint {
    pub quake_rate_km2: f32,
    pub quake_hazard: u8,
    pub volcanic_hazard: u8,
    pub drought: u8,
    pub heat_wave: u8,
    pub extreme_rain: u8,
    pub flood: u8,
}

/// Point sample of every canonical field.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PointSample {
    pub cell: u32,
    pub elevation_m: f32,
    pub ocean: bool,
    pub biome: Biome,
    pub climate: ClimatePoint,
    pub hydrology: HydrologyPoint,
    pub hazards: HazardPoint,
}

/// Hazard fields in an equirectangular view.
#[derive(Clone, Debug, PartialEq)]
pub struct HazardView {
    pub quake_rate_km2: Vec<f32>,
    pub quake_hazard: Vec<u8>,
    pub volcanic_hazard: Vec<u8>,
    pub drought: Vec<u8>,
    pub heat_wave: Vec<u8>,
    pub extreme_rain: Vec<u8>,
    pub flood: Vec<u8>,
}

type MonthRow = (Vec<f32>, Vec<f32>, Vec<f32>, Vec<f32>);

/// Equirectangular raster view of a finished world.
#[derive(Clone, Debug)]
pub struct EquirectView {
    pub width: u32,
    pub height: u32,
    pub elevation_m: Vec<f32>,
    pub ocean: Vec<u8>,
    pub climate: ClimateSummary,
    pub t_nh_summer_c: Vec<f32>,
    pub t_nh_winter_c: Vec<f32>,
    pub p_nh_summer_mm: Vec<f32>,
    pub p_nh_winter_mm: Vec<f32>,
    pub biome: Vec<u8>,
    pub lake_id: Vec<u32>,
    pub discharge_m3s: Vec<f32>,
    pub ice_cover: Vec<u8>,
    pub hazards: HazardView,
    pub storms: StormClimatology,
}

/// Land aridity and the ocean rainfall belts used to judge the moisture scheme.
#[derive(Clone, Copy, Debug)]
pub struct LandMoisture {
    /// Hot desert plus cold desert, divided by ice-free land area.
    pub desert_fraction: f64,
    /// Area-weighted annual precipitation on all land, mm.
    pub p_land_mm: f64,
    /// Ice-free land at `|lat|` 40–60°, mm.
    pub p_midlat_mm: f64,
    /// Ocean at `|lat| < 10°`, mm.
    pub ocean_equator_mm: f64,
    /// Ocean at `|lat|` 15–35°, mm.
    pub ocean_subtrop_mm: f64,
    /// Ocean at `|lat|` 40–60°, mm.
    pub ocean_midlat_mm: f64,
}

fn ratio(sum: f64, area: f64) -> f64 {
    if area > 0.0 {
        sum / area
    } else {
        0.0
    }
}

/// Convert metres to the canonical whole-millimetre representation.
#[inline]
pub fn elevation_mm_from_m(value_m: f32) -> i32 {
    let millimetres = (f64::from(value_m) * 1000.0).round();
    millimetres.clamp(f64::from(i32::MIN), f64::from(i32::MAX)) as i32
}

/// Convert whole millimetres back to the canonical `f32` metres.
#[inline]
pub fn elevation_m_from_mm(value_mm: i32) -> f32 {
    value_mm as f32 / 1000.0
}

/// Round every elevation to the canonical whole-millimetre `f32` value.
pub fn canonicalize_elevation_m(elevation: &mut [f32]) {
    for value in elevation {
        *value = elevation_m_from_mm(elevation_mm_from_m(*value));
    }
}

impl World {
    /// FNV-1a over the little-endian bits of each elevation.
    pub fn elevation_hash(&self) -> u64 {
        elevation_hash(&self.elevation_m)
    }

    /// Canonical source elevations, relative to reference sea level.
    pub fn source_elevation_mm(&self) -> Vec<i32> {
        self.elevation_m
            .iter()
            .map(|value| elevation_mm_from_m(*value))
            .collect()
    }

    /// Hashes the source stores for decode/derive validation.
    pub fn generation_hashes(&self) -> GenerationHashes {
        self.generation_hashes_with_cancel(&self.source_elevation_mm(), &|| false)
            .expect("a never-cancelled hash cannot be cancelled")
    }

    /// [`Self::generation_hashes`] with cancellation between hash chunks.
    ///
    /// The values are byte-identical to the plain path. `elevation_mm` must be
    /// [`Self::source_elevation_mm`] (or the identical decoded source slice);
    /// callers pass it because they already hold it. `cancelled` is polled
    /// between chunks of 65,536 values and once per lake, river and volcanic
    /// source.
    pub fn generation_hashes_with_cancel(
        &self,
        elevation_mm: &[i32],
        cancelled: &(dyn Fn() -> bool + Sync),
    ) -> Result<GenerationHashes, WorldError> {
        debug_assert_eq!(elevation_mm.len(), self.grid.cell_count());
        let elevation_mm = hash_i32_values_with_cancel(FNV_OFFSET, elevation_mm, cancelled)?;
        let tectonics = self.tectonics.hash_bits();
        let climate = hash_climate_with_cancel(&self.climate, cancelled)?;
        let hydrology = hash_hydrology_with_cancel(&self.hydrology, cancelled)?;
        let biome = hash_u8_values_with_cancel(FNV_OFFSET, &self.biome, cancelled)?;
        let hazards = hash_hazards_with_cancel(&self.hazards, cancelled)?;
        Ok(GenerationHashes {
            elevation_mm,
            tectonics,
            climate,
            hydrology,
            biome,
            hazards,
        })
    }

    /// Reference water inventory: all cells below sea level, in cubic metres.
    pub fn reference_water_inventory_m3(&self) -> u64 {
        let elevation_mm = self.source_elevation_mm();
        ocean_volume_at(&self.grid, &elevation_mm, 0).unwrap_or(0)
    }

    /// Hash of the tectonic field stored by the source.
    pub fn tectonics_hash(&self) -> u64 {
        self.tectonics.hash_bits()
    }

    /// Hash of every hazard field and source.
    pub fn hazard_hash(&self) -> u64 {
        self.hazards.hash_bits()
    }

    /// FNV-1a over the climate summary, in field order, then sea-ice months.
    pub fn climate_hash(&self) -> u64 {
        self.climate.hash_bits()
    }

    /// FNV-1a over lakes, rivers, discharge, ice and flood receivers.
    pub fn hydrology_hash(&self) -> u64 {
        self.hydrology.hash_bits()
    }

    /// FNV-1a over the biome ids.
    pub fn biome_hash(&self) -> u64 {
        biome::hash_bits(&self.biome)
    }

    /// Desert share of ice-free land, and area-weighted precipitation.
    ///
    /// Ice-free land excludes ice-sheet and glacier cells. Mid-latitude rain
    /// is ice-free land at `|lat|` 40–60°. Ocean bands are the equator
    /// (`|lat| < 10°`), the dry belt (15–35°) and the westerlies (40–60°).
    pub fn land_moisture(&self) -> LandMoisture {
        let mut desert = 0.0;
        let mut ice_free = 0.0;
        let mut land_p = 0.0;
        let mut land_a = 0.0;
        let mut mid_p = 0.0;
        let mut mid_a = 0.0;
        let mut ocean_eq_p = 0.0;
        let mut ocean_eq_a = 0.0;
        let mut ocean_dry_p = 0.0;
        let mut ocean_dry_a = 0.0;
        let mut ocean_mid_p = 0.0;
        let mut ocean_mid_a = 0.0;
        for cell in 0..self.grid.cell_count() {
            let area = self.grid.area_m2(cell as u32);
            let precip = f64::from(self.climate.p_annual_mm.get(cell).copied().unwrap_or(0.0));
            let lat = self.grid.lat_lon(cell as u32).0.to_degrees().abs();
            if self.ocean.get(cell).copied().unwrap_or(1) == 1 {
                if lat < 10.0 {
                    ocean_eq_p += precip * area;
                    ocean_eq_a += area;
                }
                if (15.0..=35.0).contains(&lat) {
                    ocean_dry_p += precip * area;
                    ocean_dry_a += area;
                }
                if (40.0..=60.0).contains(&lat) {
                    ocean_mid_p += precip * area;
                    ocean_mid_a += area;
                }
                continue;
            }
            land_p += precip * area;
            land_a += area;
            let class = self.biome.get(cell).copied().and_then(Biome::from_u8);
            let ice = class.is_some_and(|class| matches!(class, Biome::IceSheet | Biome::Glacier));
            if ice {
                continue;
            }
            ice_free += area;
            if class.is_some_and(Biome::is_desert) {
                desert += area;
            }
            if (40.0..=60.0).contains(&lat) {
                mid_p += precip * area;
                mid_a += area;
            }
        }
        LandMoisture {
            desert_fraction: ratio(desert, ice_free),
            p_land_mm: ratio(land_p, land_a),
            p_midlat_mm: ratio(mid_p, mid_a),
            ocean_equator_mm: ratio(ocean_eq_p, ocean_eq_a),
            ocean_subtrop_mm: ratio(ocean_dry_p, ocean_dry_a),
            ocean_midlat_mm: ratio(ocean_mid_p, ocean_mid_a),
        }
    }

    /// Bilinear elevation at a geographic point. Latitude and longitude are degrees.
    pub fn sample_elevation(&self, lat_deg: f64, lon_deg: f64) -> f32 {
        self.grid
            .sample_bilinear(&self.elevation_m, direction_from_lat_lon(lat_deg, lon_deg))
    }

    /// Point sample; continuous fields are bilinear, classes and ids nearest.
    pub fn sample_point(&self, lat_deg: f64, lon_deg: f64) -> PointSample {
        let p = direction_from_lat_lon(lat_deg, lon_deg);
        let cell = self.grid.cell_at(p);
        let sample = |field: &[f32]| self.grid.sample_bilinear(field, p);
        let t_annual = sample(&self.climate.t_annual_c);
        let p_annual = sample(&self.climate.p_annual_mm);
        let pet = sample(&self.climate.pet_annual_mm);
        let (t_summer, p_summer) =
            self.climate_month_at_point(lat_deg, lon_deg, climate::NH_SUMMER_SOLSTICE);
        let (t_winter, p_winter) =
            self.climate_month_at_point(lat_deg, lon_deg, climate::NH_WINTER_SOLSTICE);
        let ocean = self.ocean.get(cell as usize).copied().unwrap_or(1) == 1;
        let (aridity, maritime) = {
            let aridity = if ocean || pet <= 0.0 {
                0
            } else {
                (1.0e6 * (1.0 - f64::from(p_annual) / f64::from(pet)).clamp(0.0, 1.0)).round()
                    as u32
            };
            let coarse = &self.climate_field;
            let coarse_cell = coarse.grid.cell_at(p) as usize;
            let maritime = (f64::from(coarse.maritime.get(coarse_cell).copied().unwrap_or(0.0))
                * 1.0e6)
                .round()
                .clamp(0.0, 1.0e6) as u32;
            (aridity, maritime)
        };
        let lake_id = self
            .hydrology
            .lake_id
            .get(cell as usize)
            .copied()
            .unwrap_or(NONE);
        let lake_level = self
            .hydrology
            .lakes
            .iter()
            .find(|lake| lake.id == lake_id)
            .map(|lake| lake.level_m as f32)
            .unwrap_or(0.0);
        let wind = {
            let field = &self.climate_field;
            let n = field.grid.cell_count();
            let coarse_cell = field.grid.cell_at(p) as usize;
            let mut east = 0.0;
            let mut north = 0.0;
            let pos = field.grid.position(coarse_cell as u32);
            let (east_dir, north_dir) = climate::point_east_north(pos);
            for month in 0..12 {
                let base = month * 3 * n + coarse_cell;
                let value = [
                    f64::from(field.wind[base]),
                    f64::from(field.wind[base + n]),
                    f64::from(field.wind[base + 2 * n]),
                ];
                let tangent = climate::point_tangent(pos, value);
                east += crate::grid::dot3(tangent, east_dir);
                north += crate::grid::dot3(tangent, north_dir);
            }
            (
                (east / 12.0 * 1000.0).round() as i32,
                (north / 12.0 * 1000.0).round() as i32,
            )
        };
        let hazard_u8 = |field: &[u8]| field.get(cell as usize).copied().unwrap_or(0);
        PointSample {
            cell,
            elevation_m: sample(&self.elevation_m),
            ocean,
            biome: Biome::from_u8(self.biome.get(cell as usize).copied().unwrap_or(0))
                .unwrap_or(Biome::DeepOcean),
            climate: ClimatePoint {
                t_annual_c: t_annual,
                t_warm_c: sample(&self.climate.t_warm_c),
                t_cold_c: sample(&self.climate.t_cold_c),
                t_nh_summer_c: t_summer,
                t_nh_winter_c: t_winter,
                p_annual_mm: p_annual,
                p_nh_summer_mm: p_summer,
                p_nh_winter_mm: p_winter,
                p_dry_month_mm: sample(&self.climate.p_dry_month_mm),
                pet_annual_mm: pet,
                runoff_mm: sample(&self.climate.runoff_mm),
                ice_balance_mm: sample(&self.climate.ice_balance_mm),
                sea_ice_months: self
                    .climate
                    .sea_ice_months
                    .get(cell as usize)
                    .copied()
                    .unwrap_or(0),
                wind_east_milli: wind.0,
                wind_north_milli: wind.1,
                aridity_ppm: aridity,
                maritime_ppm: maritime,
            },
            hydrology: HydrologyPoint {
                lake_id,
                lake_level_m: lake_level,
                discharge_m3s: self.grid.sample_f64(&self.hydrology.discharge_m3s, p) as f32,
                ice_cover: self
                    .hydrology
                    .ice_cover
                    .get(cell as usize)
                    .copied()
                    .unwrap_or(0),
                ocean,
            },
            hazards: HazardPoint {
                quake_rate_km2: sample(&self.hazards.quake_rate_km2),
                quake_hazard: hazard_u8(&self.hazards.quake_hazard),
                volcanic_hazard: hazard_u8(&self.hazards.volcanic_hazard),
                drought: hazard_u8(&self.hazards.drought),
                heat_wave: hazard_u8(&self.hazards.heat_wave),
                extreme_rain: hazard_u8(&self.hazards.extreme_rain),
                flood: hazard_u8(&self.hazards.flood),
            },
        }
    }

    /// Bilinear monthly temperature and precipitation at a point.
    ///
    /// The stencil is built on the fine grid, so each term is a fine cell with
    /// its own lapse and orographic correction.
    pub fn climate_month_at_point(&self, lat_deg: f64, lon_deg: f64, month: usize) -> (f32, f32) {
        let p = direction_from_lat_lon(lat_deg, lon_deg);
        let (face, fx, fy) = self.grid.face_coords(p);
        let temperature = self
            .grid
            .sample_chart(face, fx, fy, &|cell| {
                Some(f64::from(self.climate_month_at(cell, month).0))
            })
            .unwrap_or(0.0);
        let precipitation = self
            .grid
            .sample_chart(face, fx, fy, &|cell| {
                Some(f64::from(self.climate_month_at(cell, month).1))
            })
            .unwrap_or(0.0);
        (temperature as f32, precipitation as f32)
    }

    /// Bilinear equirectangular elevation view, width by height, row-major.
    ///
    /// This is the elevation-only subset of [`Self::equirect_view`]: it
    /// produces exactly the same `elevation_m` values, but skips the climate
    /// and field resampling and checks `cancelled` between row batches. The
    /// product view uses this path, so a cancel request does not wait for the
    /// monthly climate view.
    pub fn equirect_elevation_with_cancel(
        &self,
        width: u32,
        height: u32,
        cancelled: &(dyn Fn() -> bool + Sync),
    ) -> Result<Vec<f32>, WorldError> {
        if width == 0 || height == 0 {
            return Err(WorldError::Internal(
                "equirect view dimensions must be positive".to_string(),
            ));
        }
        let count = width as usize * height as usize;
        let mut out = vec![0.0f32; count];
        let batch_rows = (height as usize / 16).max(1);
        for start in (0..height as usize).step_by(batch_rows) {
            if cancelled() {
                return Err(WorldError::Cancelled);
            }
            let end = (start + batch_rows).min(height as usize);
            let block = &mut out[start * width as usize..end * width as usize];
            block
                .par_chunks_mut(width as usize)
                .enumerate()
                .for_each(|(local_row, row_out)| {
                    let row = start + local_row;
                    let lat = -90.0 + 180.0 * (f64::from(row as u32) + 0.5) / f64::from(height);
                    for (col, dest) in row_out.iter_mut().enumerate() {
                        let lon = -180.0 + 360.0 * (col as f64 + 0.5) / f64::from(width);
                        let direction = direction_from_lat_lon(lat, lon);
                        *dest = self.grid.sample_bilinear(&self.elevation_m, direction);
                    }
                });
        }
        if cancelled() {
            Err(WorldError::Cancelled)
        } else {
            Ok(out)
        }
    }

    /// Equirectangular raster view; continuous fields bilinear, classes nearest.
    pub fn equirect_view(&self, width: u32, height: u32) -> Result<EquirectView, WorldError> {
        if width == 0 || height == 0 {
            return Err(WorldError::Internal(
                "equirect view dimensions must be positive".to_string(),
            ));
        }
        let count = width as usize * height as usize;
        let points: Vec<[f64; 3]> = (0..count)
            .map(|index| {
                let row = index / width as usize;
                let col = index % width as usize;
                let lon = -180.0 + 360.0 * (col as f64 + 0.5) / width as f64;
                let lat = -90.0 + 180.0 * (row as f64 + 0.5) / height as f64;
                direction_from_lat_lon(lat, lon)
            })
            .collect();
        let resample = |field: &[f32]| -> Vec<f32> {
            points
                .iter()
                .map(|p| self.grid.sample_bilinear(field, *p))
                .collect()
        };
        let nearest_u8 = |field: &[u8]| -> Vec<u8> {
            points
                .iter()
                .map(|p| field[self.grid.cell_at(*p) as usize])
                .collect()
        };
        let nearest_u32 = |field: &[u32]| -> Vec<u32> {
            points
                .iter()
                .map(|p| field[self.grid.cell_at(*p) as usize])
                .collect()
        };
        let discharge_f32: Vec<f32> = self
            .hydrology
            .discharge_m3s
            .iter()
            .map(|value| *value as f32)
            .collect();
        let elevation_m = self.equirect_elevation_with_cancel(width, height, &|| false)?;
        let climate = ClimateSummary {
            t_annual_c: resample(&self.climate.t_annual_c),
            t_warm_c: resample(&self.climate.t_warm_c),
            t_cold_c: resample(&self.climate.t_cold_c),
            p_annual_mm: resample(&self.climate.p_annual_mm),
            p_dry_month_mm: resample(&self.climate.p_dry_month_mm),
            pet_annual_mm: resample(&self.climate.pet_annual_mm),
            runoff_mm: resample(&self.climate.runoff_mm),
            ice_balance_mm: resample(&self.climate.ice_balance_mm),
            sea_ice_months: nearest_u8(&self.climate.sea_ice_months),
            stats: self.climate.stats,
        };
        let rows: Vec<MonthRow> = (0..height)
            .into_par_iter()
            .map(|row| {
                let lat = -90.0 + 180.0 * (f64::from(row) + 0.5) / f64::from(height);
                let mut summer_t = Vec::with_capacity(width as usize);
                let mut winter_t = Vec::with_capacity(width as usize);
                let mut summer_p = Vec::with_capacity(width as usize);
                let mut winter_p = Vec::with_capacity(width as usize);
                for col in 0..width {
                    let lon = -180.0 + 360.0 * (f64::from(col) + 0.5) / f64::from(width);
                    let (st, sp) =
                        self.climate_month_at_point(lat, lon, climate::NH_SUMMER_SOLSTICE);
                    let (wt, wp) =
                        self.climate_month_at_point(lat, lon, climate::NH_WINTER_SOLSTICE);
                    summer_t.push(st);
                    winter_t.push(wt);
                    summer_p.push(sp);
                    winter_p.push(wp);
                }
                (summer_t, winter_t, summer_p, winter_p)
            })
            .collect();
        let mut t_nh_summer_c = Vec::with_capacity(count);
        let mut t_nh_winter_c = Vec::with_capacity(count);
        let mut p_nh_summer_mm = Vec::with_capacity(count);
        let mut p_nh_winter_mm = Vec::with_capacity(count);
        for (summer_t, winter_t, summer_p, winter_p) in rows {
            t_nh_summer_c.extend_from_slice(&summer_t);
            t_nh_winter_c.extend_from_slice(&winter_t);
            p_nh_summer_mm.extend_from_slice(&summer_p);
            p_nh_winter_mm.extend_from_slice(&winter_p);
        }
        Ok(EquirectView {
            width,
            height,
            elevation_m,
            ocean: nearest_u8(&self.ocean),
            climate,
            t_nh_summer_c,
            t_nh_winter_c,
            p_nh_summer_mm,
            p_nh_winter_mm,
            biome: nearest_u8(&self.biome),
            lake_id: nearest_u32(&self.hydrology.lake_id),
            discharge_m3s: resample(&discharge_f32),
            ice_cover: nearest_u8(&self.hydrology.ice_cover),
            hazards: HazardView {
                quake_rate_km2: resample(&self.hazards.quake_rate_km2),
                quake_hazard: nearest_u8(&self.hazards.quake_hazard),
                volcanic_hazard: nearest_u8(&self.hazards.volcanic_hazard),
                drought: nearest_u8(&self.hazards.drought),
                heat_wave: nearest_u8(&self.hazards.heat_wave),
                extreme_rain: nearest_u8(&self.hazards.extreme_rain),
                flood: nearest_u8(&self.hazards.flood),
            },
            storms: self.hazards.storms.clone(),
        })
    }

    /// Equirectangular elevation sample, width by height, row-major.
    pub fn to_equirect(&self, width: u32, height: u32) -> Vec<f32> {
        render::sample_equirect(&self.grid, &self.elevation_m, width, height)
    }

    /// One month's downscaled temperature (°C) and precipitation (mm) at a cell.
    pub fn climate_month_at(&self, cell: u32, month: usize) -> (f32, f32) {
        climate::month_at(
            &self.grid,
            &self.climate_field,
            &self.elevation_m,
            &self.ocean,
            &self.settings.planet,
            cell as usize,
            month,
        )
    }

    /// Coarse-cell ocean fraction used by the storm tracer.
    pub fn coarse_ocean_fraction(&self) -> Vec<f64> {
        let coarse = &self.climate_field.grid;
        let scale = self.grid.n / coarse.n;
        hazards::coarse_ocean_fraction(&self.grid, &self.ocean, coarse, scale)
    }

    /// Latitude and longitude of a canonical cell centre, degrees.
    pub fn cell_lat_lon(&self, cell: u32) -> (f64, f64) {
        let (lat, lon) = self.grid.lat_lon(cell);
        (lat.to_degrees(), lon.to_degrees())
    }

    /// Exact area of a canonical cell, square metres.
    pub fn cell_area_m2(&self, cell: u32) -> f64 {
        self.grid.area_m2(cell)
    }
}

/// Pre-erosion fields on the final grid, rebuilt from settings and tectonics.
///
/// The erosion domain is `base_m > 0`; no erosion state is needed. This is the
/// input shared by generation and decode/derive.
#[derive(Clone, Debug)]
pub struct SurfaceContext {
    /// Final cube-sphere grid.
    pub grid: CubeSphere,
    /// `1` where `base_m > 0`.
    pub domain: Vec<u8>,
    /// Pre-erosion surface, unshifted, metres.
    pub base_m: Vec<f32>,
    /// Uplift, metres per year.
    pub uplift_m_yr: Vec<f32>,
    /// Erodibility multiplier.
    pub k_rock: Vec<f32>,
    /// Plate index per cell.
    pub plate: Vec<u8>,
    /// `1` continental, `0` oceanic.
    pub crust: Vec<u8>,
}

impl SurfaceContext {
    /// Clone the borrowed parts into an owned context.
    pub fn from_parts(
        grid: &CubeSphere,
        domain: &[u8],
        base_m: &[f32],
        uplift_m_yr: &[f32],
        k_rock: &[f32],
        plate: &[u8],
        crust: &[u8],
    ) -> Self {
        Self {
            grid: grid.clone(),
            domain: domain.to_vec(),
            base_m: base_m.to_vec(),
            uplift_m_yr: uplift_m_yr.to_vec(),
            k_rock: k_rock.to_vec(),
            plate: plate.to_vec(),
            crust: crust.to_vec(),
        }
    }
}

/// Build only the tectonic model, on `min(resolution, 256)`.
pub fn generate_tectonics(
    settings: &WorldSettings,
    progress: &mut dyn Progress,
) -> Result<Tectonics, WorldError> {
    settings.validate()?;
    tectonics::build(settings, settings.resolution.min(256), &|| {
        progress.cancelled()
    })
}

/// Rebuild the final-grid pre-erosion fields and flood domain from settings.
pub fn prepare_surface(
    settings: &WorldSettings,
    tectonics: &Tectonics,
    progress: &mut dyn Progress,
) -> Result<SurfaceContext, WorldError> {
    settings.validate()?;
    evolution::prepare_surface(settings, tectonics, &|| progress.cancelled())
}

/// Bilinear equirectangular resample of an `f32` field.
#[must_use]
pub fn resample_equirect_f32(
    grid: &CubeSphere,
    field: &[f32],
    width: u32,
    height: u32,
) -> Vec<f32> {
    resample_equirect_points(width, height)
        .into_iter()
        .map(|(lat, lon)| grid.sample_bilinear(field, direction_from_lat_lon(lat, lon)))
        .collect()
}

/// Nearest-cell equirectangular resample of a `u8` field.
#[must_use]
pub fn resample_equirect_nearest_u8(
    grid: &CubeSphere,
    field: &[u8],
    width: u32,
    height: u32,
) -> Vec<u8> {
    resample_equirect_points(width, height)
        .into_iter()
        .map(|(lat, lon)| field[grid.cell_at(direction_from_lat_lon(lat, lon)) as usize])
        .collect()
}

/// Nearest-cell equirectangular resample of a `u32` field.
#[must_use]
pub fn resample_equirect_nearest_u32(
    grid: &CubeSphere,
    field: &[u32],
    width: u32,
    height: u32,
) -> Vec<u32> {
    resample_equirect_points(width, height)
        .into_iter()
        .map(|(lat, lon)| field[grid.cell_at(direction_from_lat_lon(lat, lon)) as usize])
        .collect()
}

fn resample_equirect_points(width: u32, height: u32) -> Vec<(f64, f64)> {
    let mut points = Vec::with_capacity(width as usize * height as usize);
    for row in 0..height {
        let lat = -90.0 + 180.0 * (f64::from(row) + 0.5) / f64::from(height);
        for col in 0..width {
            let lon = -180.0 + 360.0 * (f64::from(col) + 0.5) / f64::from(width);
            points.push((lat, lon));
        }
    }
    points
}

/// Build a world. `progress` is notified per tectonic stage and per erosion iteration.
///
/// Generation runs on a rayon worker, so every per-cell loop is injected from
/// inside the pool instead of waking the workers from the main thread. A
/// caller that is already inside the pool runs inline.
pub fn generate(
    settings: &WorldSettings,
    progress: &mut dyn Progress,
) -> Result<World, WorldError> {
    rayon::scope(|_| generate_inner(settings, progress))
}

fn generate_inner(
    settings: &WorldSettings,
    progress: &mut dyn Progress,
) -> Result<World, WorldError> {
    settings.validate()?;
    let started = Instant::now();
    if progress.cancelled() {
        return Err(WorldError::Cancelled);
    }
    let mut timings = Vec::new();
    progress.report(Stage::Tectonics, 0, 1);
    let t0 = Instant::now();
    let n_t = settings.resolution.min(256);
    let tectonics = tectonics::build(settings, n_t, &|| progress.cancelled())?;
    timings.push(("tectonics", elapsed_ms(t0)));
    progress.report(Stage::Tectonics, 1, 1);

    let evolved = evolution::evolve(settings, &tectonics, progress)?;
    timings.extend(evolved.timings);

    if progress.cancelled() {
        return Err(WorldError::Cancelled);
    }
    progress.report(Stage::SeaLevel, 0, 1);
    let t_sea = Instant::now();
    let mut fields = evolved.fields;
    let mut elevation = std::mem::take(&mut fields.h);
    let domain = std::mem::take(&mut fields.domain);
    let shift = sealevel::apply_threshold(
        &fields.grid,
        &mut elevation,
        settings.land_fraction,
        &|| progress.cancelled(),
    )?;
    canonicalize_elevation_m(&mut elevation);
    let sea = sealevel::finish(&fields.grid, &elevation, &domain, shift, &|| {
        progress.cancelled()
    })?;
    timings.push(("sea_level", elapsed_ms(t_sea)));
    progress.report(Stage::SeaLevel, 1, 1);

    let surface = SurfaceContext {
        grid: fields.grid,
        domain,
        base_m: fields.base,
        uplift_m_yr: fields.uplift,
        k_rock: fields.k_rock,
        plate: fields.plate,
        crust: fields.crust,
    };
    derive_after_surface_inner(
        settings,
        tectonics,
        surface,
        elevation,
        sea,
        progress,
        timings,
        evolved.passes,
        evolved.stats,
        evolved.scratch,
        started,
    )
}

/// Decode and derive a world from its canonical surface, without erosion.
pub fn derive_world(
    settings: &WorldSettings,
    tectonics: &Tectonics,
    elevation_mm: &[i32],
    progress: &mut dyn Progress,
) -> Result<World, WorldError> {
    let started = Instant::now();
    settings.validate()?;
    let grid =
        CubeSphere::new(settings.resolution, settings.radius_m).map_err(WorldError::Internal)?;
    if elevation_mm.len() != grid.cell_count() {
        return Err(WorldError::InvalidSettings(format!(
            "elevation count {} does not match the {}-cell grid",
            elevation_mm.len(),
            grid.cell_count()
        )));
    }
    let surface = prepare_surface(settings, tectonics, progress)?;
    let elevation = elevation_mm
        .iter()
        .copied()
        .map(elevation_m_from_mm)
        .collect::<Vec<_>>();
    let sea = sealevel::finish(&surface.grid, &elevation, &surface.domain, 0.0, &|| {
        progress.cancelled()
    })?;
    derive_after_surface_inner(
        settings,
        tectonics.clone(),
        surface,
        elevation,
        sea,
        progress,
        Vec::new(),
        Vec::new(),
        Vec::new(),
        None,
        started,
    )
}

/// Derive every post-erosion product from a prepared surface.
pub fn derive_after_surface(
    settings: &WorldSettings,
    tectonics: &Tectonics,
    surface: &SurfaceContext,
    elevation_m: &[f32],
    progress: &mut dyn Progress,
) -> Result<World, WorldError> {
    let started = Instant::now();
    settings.validate()?;
    if elevation_m.len() != surface.grid.cell_count() {
        return Err(WorldError::InvalidSettings(format!(
            "elevation count {} does not match the {}-cell grid",
            elevation_m.len(),
            surface.grid.cell_count()
        )));
    }
    let elevation = elevation_m.to_vec();
    let sea = sealevel::finish(&surface.grid, &elevation, &surface.domain, 0.0, &|| {
        progress.cancelled()
    })?;
    derive_after_surface_inner(
        settings,
        tectonics.clone(),
        surface.clone(),
        elevation,
        sea,
        progress,
        Vec::new(),
        Vec::new(),
        Vec::new(),
        None,
        started,
    )
}

/// Derive an epoch surface. `reference_elevation_mm` is relative to the
/// accepted reference sea level; `epoch_sea_level_mm` is relative to it.
pub fn derive_epoch(
    settings: &WorldSettings,
    tectonics: &Tectonics,
    reference_elevation_mm: &[i32],
    epoch_sea_level_mm: i32,
    temperature_offset_c: f64,
    progress: &mut dyn Progress,
) -> Result<World, WorldError> {
    let started = Instant::now();
    let mut epoch_settings = settings.clone();
    epoch_settings.planet = epoch_settings
        .planet
        .with_temperature_offset(temperature_offset_c);
    epoch_settings.validate()?;
    let grid = CubeSphere::new(epoch_settings.resolution, epoch_settings.radius_m)
        .map_err(WorldError::Internal)?;
    if reference_elevation_mm.len() != grid.cell_count() {
        return Err(WorldError::InvalidSettings(format!(
            "elevation count {} does not match the {}-cell grid",
            reference_elevation_mm.len(),
            grid.cell_count()
        )));
    }
    let surface = prepare_surface(&epoch_settings, tectonics, progress)?;
    let elevation = reference_elevation_mm
        .iter()
        .map(|value| {
            let shifted = i64::from(*value) - i64::from(epoch_sea_level_mm);
            elevation_m_from_mm(shifted.clamp(i64::from(i32::MIN), i64::from(i32::MAX)) as i32)
        })
        .collect::<Vec<_>>();
    let sea = sealevel::finish(&surface.grid, &elevation, &surface.domain, 0.0, &|| {
        progress.cancelled()
    })?;
    derive_after_surface_inner(
        &epoch_settings,
        tectonics.clone(),
        surface,
        elevation,
        sea,
        progress,
        Vec::new(),
        Vec::new(),
        Vec::new(),
        None,
        started,
    )
}

#[allow(clippy::too_many_arguments)]
fn derive_after_surface_inner(
    settings: &WorldSettings,
    tectonics: Tectonics,
    mut surface: SurfaceContext,
    elevation: Vec<f32>,
    sea: sealevel::SeaLevel,
    progress: &mut dyn Progress,
    mut timings: Vec<(&'static str, f64)>,
    mut climate_passes: Vec<ClimatePass>,
    levels: Vec<LevelStats>,
    mut reusable_scratch: Option<RoutingScratch>,
    started: Instant,
) -> Result<World, WorldError> {
    // The ocean flood is already done; nothing downstream reads the domain.
    drop(std::mem::take(&mut surface.domain));
    if progress.cancelled() {
        return Err(WorldError::Cancelled);
    }
    progress.report(Stage::FinalRouting, 0, 1);
    let t_join = Instant::now();
    let n = elevation.len();
    // The coarse climate is one long, mostly serial solve. Routing and the
    // lake flood read only the surface, so they run beside it. The two spans
    // overlap; `total` is wall time, not their sum.
    let (surface_side, climate_side) = rayon::join(
        || -> Result<_, WorldError> {
            if progress.cancelled() {
                return Err(WorldError::Cancelled);
            }
            let route_domain: Vec<u8> = sea.ocean.iter().map(|flag| u8::from(*flag == 0)).collect();
            // Land cells plus the ocean ring they drain into. Non-ring ocean
            // cells are their own roots, so the expanded fields match the
            // full-grid route exactly.
            let route_index = routing::DomainIndex::active(&surface.grid, &route_domain);
            let route_elev: Vec<f32> = route_index
                .cells()
                .iter()
                .map(|&cell| elevation[cell as usize])
                .collect();
            let mut scratch = match reusable_scratch.take() {
                Some(mut scratch) => {
                    scratch.rebase(route_index.len());
                    scratch
                }
                None => RoutingScratch::with_index(&route_index),
            };
            let _routed = route_with_cancel(
                &surface.grid,
                &route_elev,
                &route_domain,
                &route_index,
                None,
                &mut scratch,
                None,
                &|| progress.cancelled(),
            )?;
            if progress.cancelled() {
                return Err(WorldError::Cancelled);
            }
            let mut receiver = vec![0u32; n];
            let mut drainage_area_m2 = vec![0.0f32; n];
            for (slot, &cell) in route_index.cells().iter().enumerate() {
                receiver[cell as usize] = route_index.cell(scratch.receiver[slot] as usize);
                drainage_area_m2[cell as usize] = scratch.area[slot] as f32;
            }
            let mut active = vec![0u8; n];
            for &cell in route_index.cells() {
                active[cell as usize] = 1;
            }
            for (cell, flag) in active.iter().enumerate() {
                if *flag == 0 {
                    receiver[cell] = cell as u32;
                    drainage_area_m2[cell] = surface.grid.area_m2(cell as u32) as f32;
                }
            }
            let flood = hydrology::priority_flood(&surface.grid, &elevation, &sea.ocean, &|| {
                progress.cancelled()
            })?;
            if progress.cancelled() {
                return Err(WorldError::Cancelled);
            }
            Ok((receiver, drainage_area_m2, flood, elapsed_ms(t_join)))
        },
        || {
            climate::coarse_with(
                &surface.grid,
                &elevation,
                &sea.ocean,
                &settings.planet,
                &climate::ClimateOptions::published(),
                &|| progress.cancelled(),
            )
            .map(|field| (field, elapsed_ms(t_join)))
        },
    );
    let (receiver, drainage_area_m2, flood, route_ms) = surface_side?;
    let (climate_field, coarse_ms) = climate_side?;
    timings.push(("final_routing", route_ms));
    progress.report(Stage::FinalRouting, 1, 1);
    progress.report(Stage::Climate, 0, 1);
    timings.push(("climate_coarse", coarse_ms));
    climate_passes.push(ClimatePass {
        label: "final",
        source_n: surface.grid.n,
        spinup_years: climate_field.spinup_years,
        converged: climate_field.converged,
        residual_k: climate_field.spinup_residual_k,
        rms_k: climate_field.spinup_rms_k,
        wall_ms: coarse_ms,
    });
    let t_down = Instant::now();
    let climate = climate::downscale(
        &surface.grid,
        &elevation,
        &sea.ocean,
        &settings.planet,
        &climate_field,
        &|| progress.cancelled(),
    )?;
    timings.push(("climate_downscale", elapsed_ms(t_down)));
    progress.report(Stage::Climate, 1, 1);

    if progress.cancelled() {
        return Err(WorldError::Cancelled);
    }
    progress.report(Stage::Hydrology, 0, 1);
    let t_hydro = Instant::now();
    let hydrology = hydrology::build(
        &surface.grid,
        &elevation,
        &sea.ocean,
        &climate,
        settings.river_min_m3s,
        Some(flood),
        &|| progress.cancelled(),
    )?;
    timings.push(("hydrology", elapsed_ms(t_hydro)));
    progress.report(Stage::Hydrology, 1, 1);

    progress.report(Stage::Biomes, 0, 1);
    let t_biome = Instant::now();
    let biome = biome::classify(
        &surface.grid,
        &elevation,
        &sea.ocean,
        &climate,
        &hydrology,
        &|| progress.cancelled(),
    )?;
    timings.push(("biomes", elapsed_ms(t_biome)));
    progress.report(Stage::Biomes, 1, 1);

    if progress.cancelled() {
        return Err(WorldError::Cancelled);
    }
    progress.report(Stage::Hazards, 0, 1);
    let t_hazard = Instant::now();
    let hazards = hazards::build(
        &hazards::HazardIn {
            grid: &surface.grid,
            ocean: &sea.ocean,
            elevation: &elevation,
            climate: &climate,
            climate_field: &climate_field,
            hydrology: &hydrology,
            tectonics: &tectonics,
            river_min_m3s: settings.river_min_m3s,
            seed: settings.seed,
        },
        &|| progress.cancelled(),
    )?;
    timings.push(("hazards", elapsed_ms(t_hazard)));
    progress.report(Stage::Hazards, 1, 1);
    timings.push(("total", elapsed_ms(started)));

    let stats = summarize(&elevation, &sea, levels);
    Ok(World {
        settings: settings.clone(),
        grid: surface.grid,
        elevation_m: elevation,
        base_m: surface.base_m,
        uplift_m_yr: surface.uplift_m_yr,
        k_rock: surface.k_rock,
        plate: surface.plate,
        crust: surface.crust,
        ocean: sea.ocean,
        receiver,
        drainage_area_m2,
        tectonics,
        sea_level_shift_m: sea.shift_m,
        stats,
        climate,
        climate_field,
        hydrology,
        biome,
        hazards,
        timings,
        climate_passes,
    })
}

fn summarize(elevation: &[f32], sea: &sealevel::SeaLevel, levels: Vec<LevelStats>) -> WorldStats {
    let mut min_h = f32::MAX;
    let mut max_h = f32::MIN;
    let mut land = Vec::new();
    for &value in elevation {
        min_h = min_h.min(value);
        max_h = max_h.max(value);
        if value > 0.0 {
            land.push(value);
        }
    }
    if elevation.is_empty() {
        min_h = 0.0;
        max_h = 0.0;
    }
    land.sort_by(|a, b| a.total_cmp(b));
    WorldStats {
        achieved_land_fraction: sea.achieved_land_fraction,
        dry_depressions: sea.dry_depressions,
        elevation_min_m: min_h,
        elevation_max_m: max_h,
        land_p50_m: percentile(&land, 0.50),
        land_p90_m: percentile(&land, 0.90),
        land_p99_m: percentile(&land, 0.99),
        levels,
    }
}

fn percentile(sorted_asc: &[f32], p: f64) -> f32 {
    if sorted_asc.is_empty() {
        return 0.0;
    }
    let last = sorted_asc.len() - 1;
    let index = ((last as f64) * p).round() as usize;
    sorted_asc[index.min(last)]
}

pub(crate) fn elevation_hash(values: &[f32]) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325u64;
    for value in values {
        for byte in value.to_bits().to_le_bytes() {
            hash ^= u64::from(byte);
            hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
        }
    }
    hash
}

pub(crate) const FNV_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;
const HASH_CHUNK: usize = 65_536;

fn hash_byte(mut hash: u64, byte: u8) -> u64 {
    hash ^= u64::from(byte);
    hash.wrapping_mul(FNV_PRIME)
}

fn hash_u32(mut hash: u64, value: u32) -> u64 {
    for byte in value.to_le_bytes() {
        hash = hash_byte(hash, byte);
    }
    hash
}

fn hash_u64(mut hash: u64, value: u64) -> u64 {
    for byte in value.to_le_bytes() {
        hash = hash_byte(hash, byte);
    }
    hash
}

fn hash_f32(hash: u64, value: f32) -> u64 {
    hash_u32(hash, value.to_bits())
}

fn hash_f64(hash: u64, value: f64) -> u64 {
    hash_u64(hash, value.to_bits())
}

fn hash_i32_values_with_cancel(
    mut hash: u64,
    values: &[i32],
    cancelled: &(dyn Fn() -> bool + Sync),
) -> Result<u64, WorldError> {
    for chunk in values.chunks(HASH_CHUNK) {
        if cancelled() {
            return Err(WorldError::Cancelled);
        }
        for value in chunk {
            hash = hash_u32(hash, *value as u32);
        }
    }
    Ok(hash)
}

pub(crate) fn hash_u8_values_with_cancel(
    mut hash: u64,
    values: &[u8],
    cancelled: &(dyn Fn() -> bool + Sync),
) -> Result<u64, WorldError> {
    for chunk in values.chunks(HASH_CHUNK) {
        if cancelled() {
            return Err(WorldError::Cancelled);
        }
        for value in chunk {
            hash = hash_byte(hash, *value);
        }
    }
    Ok(hash)
}

fn hash_u32_values_with_cancel(
    mut hash: u64,
    values: &[u32],
    cancelled: &(dyn Fn() -> bool + Sync),
) -> Result<u64, WorldError> {
    for chunk in values.chunks(HASH_CHUNK) {
        if cancelled() {
            return Err(WorldError::Cancelled);
        }
        for value in chunk {
            hash = hash_u32(hash, *value);
        }
    }
    Ok(hash)
}

fn hash_f32_values_with_cancel(
    mut hash: u64,
    values: &[f32],
    cancelled: &(dyn Fn() -> bool + Sync),
) -> Result<u64, WorldError> {
    for chunk in values.chunks(HASH_CHUNK) {
        if cancelled() {
            return Err(WorldError::Cancelled);
        }
        for value in chunk {
            hash = hash_f32(hash, *value);
        }
    }
    Ok(hash)
}

fn hash_f64_values_with_cancel(
    mut hash: u64,
    values: &[f64],
    cancelled: &(dyn Fn() -> bool + Sync),
) -> Result<u64, WorldError> {
    for chunk in values.chunks(HASH_CHUNK) {
        if cancelled() {
            return Err(WorldError::Cancelled);
        }
        for value in chunk {
            hash = hash_f64(hash, *value);
        }
    }
    Ok(hash)
}

pub(crate) fn hash_climate_with_cancel(
    climate: &ClimateSummary,
    cancelled: &(dyn Fn() -> bool + Sync),
) -> Result<u64, WorldError> {
    let mut hash = FNV_OFFSET;
    for field in [
        climate.t_annual_c.as_slice(),
        climate.t_warm_c.as_slice(),
        climate.t_cold_c.as_slice(),
        climate.p_annual_mm.as_slice(),
        climate.p_dry_month_mm.as_slice(),
        climate.pet_annual_mm.as_slice(),
        climate.runoff_mm.as_slice(),
        climate.ice_balance_mm.as_slice(),
    ] {
        hash = hash_f32_values_with_cancel(hash, field, cancelled)?;
    }
    hash_u8_values_with_cancel(hash, &climate.sea_ice_months, cancelled)
}

pub(crate) fn hash_hydrology_with_cancel(
    hydrology: &Hydrology,
    cancelled: &(dyn Fn() -> bool + Sync),
) -> Result<u64, WorldError> {
    let mut hash = FNV_OFFSET;
    for lake in &hydrology.lakes {
        if cancelled() {
            return Err(WorldError::Cancelled);
        }
        hash = hash_byte(hash, lake.kind as u8);
        hash = hash_f64(hash, lake.level_m);
        hash = hash_f64(hash, lake.area_m2);
        hash = hash_f64(hash, lake.volume_m3);
        hash = hash_f64(hash, lake.evaporated_m3);
        hash = hash_f64(hash, lake.outflow_m3);
        hash = hash_f64(hash, lake.inflow_m3);
    }
    for river in &hydrology.rivers {
        hash = hash_byte(hash, river.end as u8);
        hash = hash_f64(hash, river.max_discharge_m3s);
        for chunk in river.cells.chunks(HASH_CHUNK) {
            if cancelled() {
                return Err(WorldError::Cancelled);
            }
            for cell in chunk {
                hash = hash_u32(hash, *cell);
            }
        }
    }
    hash = hash_u32_values_with_cancel(hash, &hydrology.lake_id, cancelled)?;
    hash = hash_f64_values_with_cancel(hash, &hydrology.discharge_m3s, cancelled)?;
    hash = hash_u8_values_with_cancel(hash, &hydrology.ice_cover, cancelled)?;
    hash_u32_values_with_cancel(hash, &hydrology.receiver, cancelled)
}

pub(crate) fn hash_hazards_with_cancel(
    hazards: &Hazards,
    cancelled: &(dyn Fn() -> bool + Sync),
) -> Result<u64, WorldError> {
    let mut hash = FNV_OFFSET;
    hash = hash_f32_values_with_cancel(hash, &hazards.quake_rate_km2, cancelled)?;
    hash = hash_u8_values_with_cancel(hash, &hazards.quake_hazard, cancelled)?;
    hash = hash_u8_values_with_cancel(hash, &hazards.volcanic_hazard, cancelled)?;
    for source in &hazards.volcanic_sources {
        if cancelled() {
            return Err(WorldError::Cancelled);
        }
        hash = hash_u64(hash, source.id);
        for component in source.pos {
            hash = hash_f64(hash, component);
        }
        hash = hash_u32(hash, source.cell);
        hash = hash_byte(
            hash,
            match source.origin {
                VolcanicOrigin::SubductionArc => 0,
                VolcanicOrigin::Hotspot => 1,
                VolcanicOrigin::Rift => 2,
            },
        );
        hash = hash_byte(
            hash,
            match source.activity {
                VolcanicActivity::Dormant => 0,
                VolcanicActivity::Moderate => 1,
                VolcanicActivity::Vigorous => 2,
            },
        );
        hash = hash_f32(hash, source.eruptions_per_yr);
    }
    hash = hash_f32_values_with_cancel(hash, &hazards.storms.genesis_km2, cancelled)?;
    hash = hash_f32_values_with_cancel(hash, &hazards.storms.track_km2, cancelled)?;
    hash = hash_f32_values_with_cancel(hash, &hazards.storms.intensity, cancelled)?;
    hash = hash_f32_values_with_cancel(hash, &hazards.storms.genesis_monthly_km2, cancelled)?;
    hash = hash_u8_values_with_cancel(hash, &hazards.drought, cancelled)?;
    hash = hash_u8_values_with_cancel(hash, &hazards.heat_wave, cancelled)?;
    hash = hash_u8_values_with_cancel(hash, &hazards.extreme_rain, cancelled)?;
    hash_u8_values_with_cancel(hash, &hazards.flood, cancelled)
}

pub(crate) fn direction_from_lat_lon(lat_deg: f64, lon_deg: f64) -> [f64; 3] {
    let lat = lat_deg.to_radians();
    let lon = lon_deg.to_radians();
    let cl = lat.cos();
    [cl * lon.cos(), cl * lon.sin(), lat.sin()]
}

/// Cell count of one cancellation-batch chunk, tuned for 12 threads.
///
/// A batch runs on the whole pool, so its wall time grows when the pool
/// shrinks. Keeping cells per thread constant keeps one batch at roughly the
/// same wall time on any core count.
pub(crate) fn scaled_chunk(base_cells: usize) -> usize {
    let threads = rayon::current_num_threads().max(1);
    (base_cells * threads).div_ceil(12).max(1)
}

fn elapsed_ms(started: Instant) -> f64 {
    started.elapsed().as_secs_f64() * 1000.0
}

#[cfg(test)]
mod tests {
    mod biome;
    mod climate;
    mod e2;
    mod evolution;
    mod grid;
    mod hazards;
    mod hydrology;
    mod noise;
    mod pipeline;
    mod preset;
    mod routing;
    mod sampling;
    mod settings;
    mod tectonics;
}
