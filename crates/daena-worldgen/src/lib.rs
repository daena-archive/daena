//! Cube-sphere planetary terrain: analytic tectonics, erosion and climate.
//!
//! Phase A builds a deterministic world on a nested cube-sphere, from a plate
//! model at `min(n, 256)` down to an eroded surface at the requested `n`.
//! Phase B1 reads that surface and builds a seasonal climate. Stored cell
//! fields are `f32`. Geometry and area sums use `f64`.

#![deny(unsafe_code)]

mod biome;
mod climate;
mod evolution;
mod grid;
mod hydrology;
mod noise;
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
pub use hydrology::{Hydrology, Lake, LakeKind, RiverEnd, RiverSegment};
pub use noise::{derive_seed, fbm, hash3, ridged, splitmix64, warp, SeedDomain};
pub use planet::PlanetSettings;
pub use preset::Preset;
pub use render::{
    hillshade, render_equirect, render_equirect_with_shade, render_orthographic,
    render_orthographic_with_shade, write_png, Layer,
};
pub use tectonics::{BoundaryKind, BoundaryRecord, Hotspot, Plate, SubductSide, Tectonics};

use std::time::Instant;

use routing::route;
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
}

/// Receives stage updates and can stop the build.
pub trait Progress {
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
    /// Phase timings in milliseconds. `total` includes climate, water and biomes.
    pub timings: Vec<(&'static str, f64)>,
    /// Evolution climate passes, then the final pass. Spin-up years are per pass.
    pub climate_passes: Vec<ClimatePass>,
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

impl World {
    /// FNV-1a over the little-endian bits of each elevation.
    pub fn elevation_hash(&self) -> u64 {
        elevation_hash(&self.elevation_m)
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

    /// Equirectangular elevation sample, width by height, row-major.
    pub fn to_equirect(&self, width: u32, height: u32) -> Vec<f32> {
        render::sample_equirect(&self.grid, &self.elevation_m, width, height)
    }
}

/// Build a world. `progress` is notified per tectonic stage and per erosion iteration.
pub fn generate(
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
    let tectonics = tectonics::build(settings, n_t)?;
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
    let sea = sealevel::solve(
        &fields.grid,
        &mut elevation,
        &domain,
        settings.land_fraction,
    );
    drop(domain);
    timings.push(("sea_level", elapsed_ms(t_sea)));
    progress.report(Stage::SeaLevel, 1, 1);

    progress.report(Stage::FinalRouting, 0, 1);
    let t_route = Instant::now();
    let n = elevation.len();
    let route_domain: Vec<u8> = sea.ocean.iter().map(|flag| u8::from(*flag == 0)).collect();
    // Evolution already dropped its routing scratch before returning.
    let mut scratch = RoutingScratch::new(n);
    let _routed = route(
        &fields.grid,
        &elevation,
        &route_domain,
        None,
        &mut scratch,
        None,
    );
    let receiver = std::mem::take(&mut scratch.receiver);
    let area = std::mem::take(&mut scratch.area);
    drop(scratch);
    let drainage_area_m2: Vec<f32> = area.iter().map(|value| *value as f32).collect();
    drop(area);
    timings.push(("final_routing", elapsed_ms(t_route)));
    progress.report(Stage::FinalRouting, 1, 1);

    if progress.cancelled() {
        return Err(WorldError::Cancelled);
    }
    progress.report(Stage::Climate, 0, 1);
    let t_coarse = Instant::now();
    let climate_field = climate::coarse(&fields.grid, &elevation, &sea.ocean, &settings.planet)?;
    let coarse_ms = elapsed_ms(t_coarse);
    timings.push(("climate_coarse", coarse_ms));
    let mut climate_passes = evolved.passes;
    climate_passes.push(ClimatePass {
        label: "final",
        source_n: fields.grid.n,
        spinup_years: climate_field.spinup_years,
        converged: climate_field.converged,
        residual_k: climate_field.spinup_residual_k,
        wall_ms: coarse_ms,
    });
    let t_down = Instant::now();
    let climate = climate::downscale(
        &fields.grid,
        &elevation,
        &sea.ocean,
        &settings.planet,
        &climate_field,
    );
    timings.push(("climate_downscale", elapsed_ms(t_down)));
    progress.report(Stage::Climate, 1, 1);

    if progress.cancelled() {
        return Err(WorldError::Cancelled);
    }
    progress.report(Stage::Hydrology, 0, 1);
    let t_hydro = Instant::now();
    let hydrology = hydrology::build(
        &fields.grid,
        &elevation,
        &sea.ocean,
        &climate,
        settings.river_min_m3s,
    )?;
    timings.push(("hydrology", elapsed_ms(t_hydro)));
    progress.report(Stage::Hydrology, 1, 1);

    progress.report(Stage::Biomes, 0, 1);
    let t_biome = Instant::now();
    let biome = biome::classify(&fields.grid, &elevation, &sea.ocean, &climate, &hydrology)?;
    timings.push(("biomes", elapsed_ms(t_biome)));
    progress.report(Stage::Biomes, 1, 1);
    timings.push(("total", elapsed_ms(started)));

    let stats = summarize(&elevation, &sea, evolved.stats);
    Ok(World {
        settings: settings.clone(),
        grid: fields.grid,
        elevation_m: elevation,
        base_m: fields.base,
        uplift_m_yr: fields.uplift,
        k_rock: fields.k_rock,
        plate: fields.plate,
        crust: fields.crust,
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

pub(crate) fn direction_from_lat_lon(lat_deg: f64, lon_deg: f64) -> [f64; 3] {
    let lat = lat_deg.to_radians();
    let lon = lon_deg.to_radians();
    let cl = lat.cos();
    [cl * lon.cos(), cl * lon.sin(), lat.sin()]
}

fn elapsed_ms(started: Instant) -> f64 {
    started.elapsed().as_secs_f64() * 1000.0
}

#[cfg(test)]
mod tests {
    mod biome;
    mod climate;
    mod evolution;
    mod grid;
    mod hydrology;
    mod noise;
    mod pipeline;
    mod preset;
    mod routing;
    mod settings;
    mod tectonics;
}
