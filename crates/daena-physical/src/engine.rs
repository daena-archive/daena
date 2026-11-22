//! Product-facing facade over `daena-worldgen`.
//!
//! This module is the only place in the product that names engine types. It
//! maps product settings and progress, owns the `DAENAPW2` source codec, and
//! builds the transitional product views.

use std::collections::{BTreeMap, VecDeque};
use std::fmt;
use std::sync::OnceLock;

use daena_worldgen as worldgen;
use rayon::prelude::*;
use serde_json::json;

use crate::{
    planetary::PlanetaryConfiguration, products, PhysicalError, PhysicalErrorCode, PhysicalField,
    ProgressPhase, ProgressSink, ValidationReport, GENERATOR_ID, GENERATOR_VERSION,
};

pub use worldgen::WORLD_ENGINE_VERSION;

/// Magic bytes of the canonical product source.
pub const SOURCE_MAGIC: [u8; 8] = *b"DAENAPW2";
/// Canonical source format version.
pub const SOURCE_VERSION: u16 = 2;
/// Fixed part of the header for any schedule length.
pub const SOURCE_FIXED_HEADER_BYTES: usize = 252;
/// Header bytes at the default six-row schedule.
pub const SOURCE_HEADER_BYTES: usize = SOURCE_FIXED_HEADER_BYTES + 6 * 16;
/// Default product view width.
pub const PRODUCT_VIEW_WIDTH: u32 = 1024;
/// Default product view height.
pub const PRODUCT_VIEW_HEIGHT: u32 = 512;
/// Resolutions the product accepts for new worlds.
pub const PRODUCT_RESOLUTIONS: [u32; 2] = [512, 1024];

/// Centre of one canonical cube cell in microdegrees.
///
/// `resolution` is the source resolution (`Settings::resolution`), not the
/// product view size. Hazard sources and event cells use this function so the
/// frontend never has to convert a canonical cell into a position.
pub fn canonical_cell_center_microdegrees(
    resolution: u32,
    radius_metres: f64,
    cell: u32,
) -> Result<[i32; 2], PhysicalError> {
    let grid = worldgen::CubeSphere::new(resolution, radius_metres)
        .map_err(|error| invalid(error.to_string()))?;
    if cell as usize >= grid.cell_count() {
        return Err(invalid("cell is outside the canonical grid"));
    }
    let (latitude, longitude) = grid.lat_lon(cell);
    Ok([
        (longitude.to_degrees() * 1_000_000.0).round() as i32,
        (latitude.to_degrees() * 1_000_000.0).round() as i32,
    ])
}

/// Receiver roots for a canonical hydrology receiver array.
///
/// A root is the lowest cell of a receiver tree; it is also the watershed id.
pub(crate) fn receiver_roots(receiver: &[u32]) -> Vec<u32> {
    const VISITING: u32 = u32::MAX - 1;
    let mut roots = vec![TOOL_SURFACE_NONE; receiver.len()];
    for start in 0..receiver.len() {
        if roots[start] != TOOL_SURFACE_NONE {
            continue;
        }
        let mut path = Vec::new();
        let mut cursor = start as u32;
        let root = loop {
            let known = roots[cursor as usize];
            if known != TOOL_SURFACE_NONE && known != VISITING {
                break known;
            }
            if known == VISITING {
                break cursor;
            }
            roots[cursor as usize] = VISITING;
            let next = receiver
                .get(cursor as usize)
                .copied()
                .unwrap_or(TOOL_SURFACE_NONE);
            if next == TOOL_SURFACE_NONE || next == cursor || next as usize >= receiver.len() {
                break cursor;
            }
            path.push(cursor);
            cursor = next;
        };
        roots[cursor as usize] = root;
        for cell in path {
            roots[cell as usize] = root;
        }
        roots[start] = root;
    }
    roots
}

const FNV_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;

/// Named engine surface preset, with `Custom` recorded for engine settings
/// that did not come from one of the named presets.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorldPreset {
    Custom,
    Earthlike,
    Frozen,
    Hothouse,
    Desert,
    Archipelago,
    WaterWorld,
    Pangea,
    Shattered,
}

impl WorldPreset {
    pub const ALL: [WorldPreset; 9] = [
        WorldPreset::Custom,
        WorldPreset::Earthlike,
        WorldPreset::Frozen,
        WorldPreset::Hothouse,
        WorldPreset::Desert,
        WorldPreset::Archipelago,
        WorldPreset::WaterWorld,
        WorldPreset::Pangea,
        WorldPreset::Shattered,
    ];

    pub fn parse(text: &str) -> Option<Self> {
        match text {
            "custom" => Some(Self::Custom),
            _ => worldgen::Preset::parse(text).map(Self::from_engine),
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Custom => "custom",
            Self::Earthlike => "earthlike",
            Self::Frozen => "frozen",
            Self::Hothouse => "hothouse",
            Self::Desert => "desert",
            Self::Archipelago => "archipelago",
            Self::WaterWorld => "water-world",
            Self::Pangea => "pangea",
            Self::Shattered => "shattered",
        }
    }

    pub fn code(self) -> u8 {
        match self {
            Self::Custom => 0,
            Self::Earthlike => 1,
            Self::Frozen => 2,
            Self::Hothouse => 3,
            Self::Desert => 4,
            Self::Archipelago => 5,
            Self::WaterWorld => 6,
            Self::Pangea => 7,
            Self::Shattered => 8,
        }
    }

    pub fn from_code(code: u8) -> Option<Self> {
        Some(match code {
            0 => Self::Custom,
            1 => Self::Earthlike,
            2 => Self::Frozen,
            3 => Self::Hothouse,
            4 => Self::Desert,
            5 => Self::Archipelago,
            6 => Self::WaterWorld,
            7 => Self::Pangea,
            8 => Self::Shattered,
            _ => return None,
        })
    }

    fn from_engine(preset: worldgen::Preset) -> Self {
        match preset {
            worldgen::Preset::Earthlike => Self::Earthlike,
            worldgen::Preset::Frozen => Self::Frozen,
            worldgen::Preset::Hothouse => Self::Hothouse,
            worldgen::Preset::Desert => Self::Desert,
            worldgen::Preset::Archipelago => Self::Archipelago,
            worldgen::Preset::WaterWorld => Self::WaterWorld,
            worldgen::Preset::Pangea => Self::Pangea,
            worldgen::Preset::Shattered => Self::Shattered,
        }
    }

    fn engine_base(self) -> worldgen::Preset {
        match self {
            Self::Custom => worldgen::Preset::Earthlike,
            Self::Earthlike => worldgen::Preset::Earthlike,
            Self::Frozen => worldgen::Preset::Frozen,
            Self::Hothouse => worldgen::Preset::Hothouse,
            Self::Desert => worldgen::Preset::Desert,
            Self::Archipelago => worldgen::Preset::Archipelago,
            Self::WaterWorld => worldgen::Preset::WaterWorld,
            Self::Pangea => worldgen::Preset::Pangea,
            Self::Shattered => worldgen::Preset::Shattered,
        }
    }
}

/// Product generation request.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Settings {
    pub seed: u64,
    pub preset: WorldPreset,
    pub resolution: u32,
    pub radius_metres: u64,
    /// Zero uses the preset land fraction.
    pub target_land_fraction_ppm: u32,
    pub planetary: PlanetaryConfiguration,
}

impl Settings {
    pub fn from_parts(
        seed: u64,
        preset: WorldPreset,
        resolution: u32,
        radius_metres: u64,
        target_land_fraction_ppm: u32,
        planetary: PlanetaryConfiguration,
    ) -> Self {
        Self {
            seed,
            preset,
            resolution,
            radius_metres,
            target_land_fraction_ppm,
            planetary,
        }
    }

    pub fn validate(&self) -> Result<(), PhysicalError> {
        if !self.resolution.is_power_of_two() || !(32..=2048).contains(&self.resolution) {
            return Err(PhysicalError::InvalidSettings(format!(
                "resolution must be a power of two in [32, 2048], got {}",
                self.resolution
            )));
        }
        self.planetary.validate()?;
        if self.planetary.radius_metres != self.radius_metres {
            return Err(PhysicalError::InvalidSettings(
                "planetary.radiusMetres must match settings.radiusMetres".into(),
            ));
        }
        if self.target_land_fraction_ppm >= 1_000_000 {
            return Err(PhysicalError::InvalidSettings(
                "target land fraction must be in parts per million".into(),
            ));
        }
        self.snapshot().map(|_| ())
    }

    /// Product acceptance is stricter than the engine and codec: new worlds
    /// are only 512 or 1024 wide.
    pub fn validate_product(&self) -> Result<(), PhysicalError> {
        if !PRODUCT_RESOLUTIONS.contains(&self.resolution) {
            return Err(PhysicalError::InvalidSettings(format!(
                "resolution must be 512 or 1024, got {}",
                self.resolution
            )));
        }
        self.validate()
    }

    /// Resolve product fields to the engine settings that are the causes.
    pub fn world_settings(&self) -> Result<worldgen::WorldSettings, PhysicalError> {
        let mut settings = self.preset.engine_base().settings();
        settings.seed = self.seed;
        settings.resolution = self.resolution;
        settings.radius_m = self.radius_metres as f64;
        if self.target_land_fraction_ppm != 0 {
            settings.land_fraction = f64::from(self.target_land_fraction_ppm) / 1_000_000.0;
        }
        apply_planetary(&mut settings.planet, self.planetary);
        settings
            .validate()
            .map_err(|error| PhysicalError::InvalidSettings(error.to_string()))?;
        Ok(settings)
    }

    pub fn snapshot(&self) -> Result<WorldSettingsSnapshot, PhysicalError> {
        let settings = self.world_settings()?;
        Ok(WorldSettingsSnapshot::from_engine(&settings, self.preset))
    }
}

fn apply_planetary(planet: &mut worldgen::PlanetSettings, planetary: PlanetaryConfiguration) {
    planet.star_luminosity = f64::from(planetary.star_luminosity_ppm) / 1_000_000.0;
    planet.star_mass = f64::from(planetary.star_mass_ppm) / 1_000_000.0;
    planet.orbit_au = f64::from(planetary.semi_major_axis_milli_au) / 1_000_000.0;
    planet.eccentricity = f64::from(planetary.eccentricity_ppm) / 1_000_000.0;
    planet.axial_tilt_deg = f64::from(planetary.axial_tilt_milli_deg) / 1_000.0;
    planet.rotation_hours = f64::from(planetary.rotation_period_seconds) / 3_600.0;
    planet.temperature_offset_c = (f64::from(planetary.retained_heat_centi_c) - 1_400.0) / 100.0;
}

/// Exact, comparable serialization of the resolved engine settings.
#[derive(Debug, Clone)]
pub struct WorldSettingsSnapshot {
    pub seed: u64,
    pub resolution: u32,
    pub radius_m: f64,
    pub land_fraction: f64,
    pub plate_count: u32,
    pub continent_layout: u8,
    pub tectonic_activity: f64,
    pub schedule: Vec<(u32, u32, f64)>,
    pub erosion: [f64; 5],
    pub planet: [f64; 10],
    pub river_min_m3s: f64,
    pub preset: WorldPreset,
}

impl WorldSettingsSnapshot {
    fn from_engine(settings: &worldgen::WorldSettings, preset: WorldPreset) -> Self {
        Self {
            seed: settings.seed,
            resolution: settings.resolution,
            radius_m: settings.radius_m,
            land_fraction: settings.land_fraction,
            plate_count: settings.plate_count,
            continent_layout: layout_code(settings.continent_layout),
            tectonic_activity: settings.tectonic_activity,
            schedule: settings
                .schedule
                .steps
                .iter()
                .map(|step| (step.max_n, step.iterations, step.dt_yr))
                .collect(),
            erosion: [
                settings.erosion.m,
                settings.erosion.k,
                settings.erosion.s_c,
                settings.erosion.kappa,
                settings.erosion.max_elevation_m,
            ],
            planet: [
                settings.planet.star_luminosity,
                settings.planet.star_mass,
                settings.planet.orbit_au,
                settings.planet.eccentricity,
                settings.planet.perihelion_deg,
                settings.planet.axial_tilt_deg,
                settings.planet.rotation_hours,
                settings.planet.greenhouse_wm2,
                settings.planet.temperature_offset_c,
                settings.planet.precipitation_scale,
            ],
            river_min_m3s: settings.river_min_m3s,
            preset,
        }
    }

    pub fn mismatch(&self, other: &Self) -> Option<&'static str> {
        if self.seed != other.seed {
            return Some("seed");
        }
        if self.resolution != other.resolution {
            return Some("resolution");
        }
        if self.radius_m.to_bits() != other.radius_m.to_bits() {
            return Some("radiusMetres");
        }
        if self.land_fraction.to_bits() != other.land_fraction.to_bits() {
            return Some("targetLandFractionPpm");
        }
        if self.plate_count != other.plate_count {
            return Some("plateCount");
        }
        if self.continent_layout != other.continent_layout {
            return Some("continentLayout");
        }
        if self.tectonic_activity.to_bits() != other.tectonic_activity.to_bits() {
            return Some("tectonicActivity");
        }
        if self.schedule.len() != other.schedule.len() {
            return Some("schedule");
        }
        for (first, second) in self.schedule.iter().zip(other.schedule.iter()) {
            if first.0 != second.0 || first.1 != second.1 || first.2.to_bits() != second.2.to_bits()
            {
                return Some("schedule");
            }
        }
        for (first, second) in self.erosion.iter().zip(other.erosion.iter()) {
            if first.to_bits() != second.to_bits() {
                return Some("erosion");
            }
        }
        for (first, second) in self.planet.iter().zip(other.planet.iter()) {
            if first.to_bits() != second.to_bits() {
                return Some("planet");
            }
        }
        if self.river_min_m3s.to_bits() != other.river_min_m3s.to_bits() {
            return Some("riverMinM3s");
        }
        None
    }
}

/// Hashes stored in the source.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SourceHashes {
    pub elevation_mm: u64,
    pub tectonics: u64,
    pub climate: u64,
    pub hydrology: u64,
    pub biome: u64,
    pub hazards: u64,
}

impl SourceHashes {
    pub fn mismatch(&self, other: &Self) -> Option<&'static str> {
        if self.elevation_mm != other.elevation_mm {
            return Some("elevationHash");
        }
        if self.tectonics != other.tectonics {
            return Some("tectonicsHash");
        }
        if self.climate != other.climate {
            return Some("climateHash");
        }
        if self.hydrology != other.hydrology {
            return Some("hydrologyHash");
        }
        if self.biome != other.biome {
            return Some("biomeHash");
        }
        if self.hazards != other.hazards {
            return Some("hazardHash");
        }
        None
    }
}

/// A decoded, validated `DAENAPW2` source.
#[derive(Clone)]
pub struct DecodedSource {
    settings: worldgen::WorldSettings,
    preset: WorldPreset,
    elevation_mm: Vec<i32>,
    hashes: SourceHashes,
    reference_water_inventory_m3: u64,
    tectonics: worldgen::Tectonics,
}

impl fmt::Debug for DecodedSource {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("DecodedSource")
            .field("settings", &self.settings)
            .field("preset", &self.preset)
            .field("cells", &self.elevation_mm.len())
            .field("hashes", &self.hashes)
            .field(
                "reference_water_inventory_m3",
                &self.reference_water_inventory_m3,
            )
            .finish_non_exhaustive()
    }
}

impl DecodedSource {
    pub fn settings_snapshot(&self) -> WorldSettingsSnapshot {
        WorldSettingsSnapshot::from_engine(&self.settings, self.preset)
    }

    pub fn source_version(&self) -> u16 {
        SOURCE_VERSION
    }

    pub fn engine_version(&self) -> u32 {
        WORLD_ENGINE_VERSION
    }

    pub fn preset(&self) -> WorldPreset {
        self.preset
    }

    pub fn hashes(&self) -> SourceHashes {
        self.hashes
    }

    pub fn reference_water_inventory_m3(&self) -> u64 {
        self.reference_water_inventory_m3
    }

    pub fn elevation_mm(&self) -> &[i32] {
        &self.elevation_mm
    }

    /// Validation report on the canonical cube grid.
    pub fn validation_report(&self) -> Result<ValidationReport, PhysicalError> {
        let grid = worldgen::CubeSphere::new(self.settings.resolution, self.settings.radius_m)
            .map_err(|error| invalid(error.to_string()))?;
        let mut land = 0.0;
        let mut total = 0.0;
        for (cell, elevation) in self.elevation_mm.iter().enumerate() {
            let area = grid.area_m2(cell as u32);
            total += area;
            if *elevation > 0 {
                land += area;
            }
        }
        if !land.is_finite() || !total.is_finite() || total <= 0.0 {
            return Err(invalid("source land fraction is not finite"));
        }
        Ok(ValidationReport {
            actual_land_fraction_ppm: (land / total * 1_000_000.0).round() as u32,
            reference_water_inventory_m3: self.reference_water_inventory_m3,
            // Raster coastlines are an E4 derived product. The canonical
            // source report does not invent one.
            coastline_segments: 0,
        })
    }

    /// Canonical reference elevations in cell order.
    pub fn reference_elevation_mm(&self) -> &[i32] {
        &self.elevation_mm
    }

    /// Resolved seed stored in the source.
    pub fn seed(&self) -> u64 {
        self.settings.seed
    }

    fn cube_grid(&self) -> Result<worldgen::CubeSphere, PhysicalError> {
        worldgen::CubeSphere::new(self.settings.resolution, self.settings.radius_m)
            .map_err(|error| invalid(error.to_string()))
    }

    /// Centre of one canonical cube cell in microdegrees.
    pub fn cell_center_microdegrees(&self, cell: u32) -> Result<[i32; 2], PhysicalError> {
        canonical_cell_center_microdegrees(self.settings.resolution, self.settings.radius_m, cell)
    }

    /// Ocean volume samples on the canonical cube grid.
    pub fn ocean_volume_curve(&self) -> Result<Vec<OceanVolumePoint>, PhysicalError> {
        let grid = self.cube_grid()?;
        worldgen::ocean_volume_curve(&grid, &self.elevation_mm)
            .map(|curve| {
                curve
                    .into_iter()
                    .map(|sample| OceanVolumePoint {
                        sea_level_mm: sample.sea_level_mm,
                        volume_m3: sample.volume_m3,
                    })
                    .collect()
            })
            .map_err(|error| invalid(error.to_string()))
    }

    /// Solve a sea level from an already-built canonical curve.
    pub fn sea_level_for_curve(
        &self,
        curve: &[OceanVolumePoint],
        inventory_m3: u64,
    ) -> Result<i32, PhysicalError> {
        let samples = curve
            .iter()
            .map(|point| worldgen::OceanVolumeSample {
                sea_level_mm: point.sea_level_mm,
                volume_m3: point.volume_m3,
            })
            .collect::<Vec<_>>();
        worldgen::sea_level_for_ocean_volume(&samples, inventory_m3)
            .map_err(|error| invalid(error.to_string()))
    }

    /// Ocean volume below a sea level on the canonical cube grid.
    pub fn ocean_volume_at(&self, sea_level_mm: i32) -> Result<u64, PhysicalError> {
        let grid = self.cube_grid()?;
        worldgen::ocean_volume_at(&grid, &self.elevation_mm, sea_level_mm)
            .map_err(|error| invalid(error.to_string()))
    }

    /// Product display configuration reconstructed from the source planet.
    ///
    /// The source stores `PlanetSettings`, not a product
    /// `PlanetaryConfiguration`. Values the source does not carry keep the
    /// Earth-like defaults, and the preset is `Custom`.
    pub fn planetary_configuration(&self) -> PlanetaryConfiguration {
        let planet = self.settings.planet;
        let mut configuration = PlanetaryConfiguration::earth_like();
        configuration.preset = crate::planetary::PlanetaryPreset::Custom;
        configuration.star_luminosity_ppm =
            (planet.star_luminosity * 1_000_000.0).round().max(0.0) as u32;
        configuration.star_mass_ppm = (planet.star_mass * 1_000_000.0).round().max(0.0) as u32;
        configuration.semi_major_axis_milli_au =
            (planet.orbit_au * 1_000_000.0).round().max(0.0) as u32;
        configuration.eccentricity_ppm =
            (planet.eccentricity * 1_000_000.0).round().max(0.0) as u32;
        configuration.axial_tilt_milli_deg =
            (planet.axial_tilt_deg * 1_000.0).round().max(0.0) as u32;
        configuration.rotation_period_seconds =
            (planet.rotation_hours * 3_600.0).round().max(0.0) as u32;
        configuration.retained_heat_centi_c =
            (planet.temperature_offset_c * 100.0 + 1_400.0).round() as i32;
        configuration.radius_metres = self.settings.radius_m.round() as u64;
        configuration
    }

    /// Re-derive the present world and build the engine-backed products.
    pub fn derive_products(
        &self,
        planetary: PlanetaryConfiguration,
        progress: &mut dyn ProgressSink,
    ) -> Result<DerivedProducts, PhysicalError> {
        let mut adapter = EngineProgress::new(progress);
        let derived = worldgen::derive_world(
            &self.settings,
            &self.tectonics,
            &self.elevation_mm,
            &mut adapter,
        );
        adapter.finish()?;
        let world = derived.map_err(|error| map_world_error(error, true))?;
        let computed = source_hashes_with_cancel(&world, &world.source_elevation_mm(), progress)?;
        if let Some(field) = computed.mismatch(&self.hashes) {
            return Err(invalid(format!(
                "derived generation hash does not match the source: {field}"
            )));
        }
        let mut products = build_world_products(&world, 0, planetary, Some(computed), progress)?;
        products.hashes = computed;
        Ok(products)
    }

    /// Derive one epoch from the canonical source and build its products.
    pub fn derive_epoch_products(
        &self,
        epoch_sea_level_mm: i32,
        temperature_offset_c: f64,
        planetary: PlanetaryConfiguration,
        progress: &mut dyn ProgressSink,
    ) -> Result<DerivedProducts, PhysicalError> {
        let mut adapter = EngineProgress::new(progress);
        let derived = worldgen::derive_epoch(
            &self.settings,
            &self.tectonics,
            &self.elevation_mm,
            epoch_sea_level_mm,
            temperature_offset_c,
            &mut adapter,
        );
        adapter.finish()?;
        let world = derived.map_err(|error| map_world_error(error, true))?;
        build_world_products(
            &world,
            epoch_sea_level_mm,
            planetary,
            Some(self.hashes),
            progress,
        )
    }

    /// Sample bounded natural events from one engine world.
    ///
    /// Earthquakes and eruptions always read the present world, because
    /// tectonic hazards do not change with epoch. Storms and floods read the
    /// epoch world when `epoch` is present and its normalized epoch is not
    /// zero. A present derive verifies every source generation hash before it
    /// samples; an epoch derive is not hash-checked because its climate,
    /// hydrology, biome and hazard outputs are expected to move. The world is
    /// dropped before this returns.
    pub fn sample_events(
        &self,
        epoch: Option<&crate::history::EpochSolve>,
        request: &crate::events::EventMaterializationRequest,
        progress: &mut dyn ProgressSink,
    ) -> Result<crate::events::EventSample, PhysicalError> {
        request.validate().map_err(PhysicalError::Validation)?;
        let present = match request.event_kind {
            crate::events::NaturalEventKind::Earthquake
            | crate::events::NaturalEventKind::Eruption => true,
            crate::events::NaturalEventKind::Storm | crate::events::NaturalEventKind::Flood => {
                match epoch {
                    None => true,
                    Some(solve) => solve.normalized_epoch == 0,
                }
            }
        };
        let mut adapter = EngineProgress::new(progress);
        let derived = if present {
            worldgen::derive_world(
                &self.settings,
                &self.tectonics,
                &self.elevation_mm,
                &mut adapter,
            )
        } else {
            let solve = epoch.expect("a non-present event epoch has a solve");
            worldgen::derive_epoch(
                &self.settings,
                &self.tectonics,
                &self.elevation_mm,
                solve.sea_level_mm,
                f64::from(solve.temperature_offset_centi_c) / 100.0,
                &mut adapter,
            )
        };
        adapter.finish()?;
        let world = derived.map_err(|error| map_world_error(error, true))?;
        if present {
            let computed =
                source_hashes_with_cancel(&world, &world.source_elevation_mm(), progress)?;
            if let Some(field) = computed.mismatch(&self.hashes) {
                return Err(invalid(format!(
                    "derived generation hash does not match the source: {field}"
                )));
            }
        }
        crate::events::sample_world_events(&world, request)
    }
}

/// Sentinel cell id for "no cell" in [`ToolSurface`].
pub const TOOL_SURFACE_NONE: u32 = worldgen::NONE;

const FLAG_OCEAN: u8 = 1;
const FLAG_RIVER: u8 = 2;
const FLAG_ICE: u8 = 4;
/// Cell of a fresh lake that holds water.
const FLAG_FRESH_LAKE: u8 = 8;
/// Cell of a fresh or saline lake that holds water (not a dry salt flat).
const FLAG_WET_LAKE: u8 = 16;
/// Cell of a fresh lake that spills: the lake overflows at this surface epoch.
const FLAG_OVERFLOWING_LAKE: u8 = 32;

/// One lake at the surface epoch. Cells are canonical cube cells.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SurfaceLake {
    pub id: u32,
    pub minimum_cell: u32,
    pub spill_cell: u32,
    pub fresh: bool,
    pub overflowing: bool,
    /// Whether the lake holds water at this surface epoch. A salt flat is dry
    /// even though the engine keeps its depression record.
    pub has_water: bool,
    pub cell_count: u32,
}

/// One river segment at the surface epoch, head cell first.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SurfaceRiver {
    pub source_cell: u32,
    pub mouth_cell: u32,
    pub strahler_order: u16,
    pub cells: Vec<u32>,
}

/// Compact canonical cell surface for the physical tools (Find Place, routes,
/// claims and regions).
///
/// Built from one [`worldgen::World`]; the world is released once the surface
/// exists. Elevations are reference elevations (relative to the reference sea
/// level), and `sea_level_mm` moves them for an epoch. Every other field is the
/// value at the surface epoch.
#[derive(Debug)]
pub struct ToolSurface {
    grid: worldgen::CubeSphere,
    pub sea_level_mm: i32,
    /// Reference elevation in millimetres, canonical cell order.
    pub elevation_mm: Vec<i32>,
    /// Bit flags: ocean, river, ice.
    pub flags: Vec<u8>,
    /// Slope in parts per million (µm/m) from true cube-neighbour distances.
    pub slope_ppm: Vec<u32>,
    /// Engine lake id, or [`TOOL_SURFACE_NONE`].
    pub lake_id: Vec<u32>,
    /// Engine biome id `0..=25`.
    pub biome_class: Vec<u8>,
    pub temperature_centi_c: Vec<i32>,
    pub precipitation_mm: Vec<u32>,
    pub pet_mm: Vec<u32>,
    pub earthquake_hazard: Vec<u8>,
    pub volcanic_hazard: Vec<u8>,
    /// Storm genesis presentation index, `0..=1_000_000`.
    pub storm_genesis_ppm: Vec<u32>,
    /// Landmass component id: the lowest canonical cell of the component, or
    /// [`TOOL_SURFACE_NONE`] on ocean.
    pub landmass_id: Vec<u32>,
    /// Priority-flood receiver cell at the surface epoch.
    pub receiver: Vec<u32>,
    pub lakes: Vec<SurfaceLake>,
    pub rivers: Vec<SurfaceRiver>,
    lake_index: BTreeMap<u32, usize>,
    lake_minimum_index: BTreeMap<u32, usize>,
    /// Landmass and watershed size ranks, computed once on first use.
    region_ranks: OnceLock<RegionRanks>,
}

#[derive(Debug, Default)]
struct RegionRanks {
    landmass: BTreeMap<u32, u32>,
    watershed: BTreeMap<u32, u32>,
}

fn rank_map(sizes: BTreeMap<u32, u32>) -> BTreeMap<u32, u32> {
    let mut ordered = sizes.into_iter().collect::<Vec<_>>();
    ordered.sort_by(|left, right| right.1.cmp(&left.1).then(left.0.cmp(&right.0)));
    ordered
        .into_iter()
        .enumerate()
        .map(|(index, (id, _))| (id, index as u32 + 1))
        .collect()
}

impl ToolSurface {
    fn rebuild_lake_indexes(&mut self) {
        self.lake_index.clear();
        self.lake_minimum_index.clear();
        for (index, lake) in self.lakes.iter().enumerate() {
            self.lake_index.entry(lake.id).or_insert(index);
            self.lake_minimum_index
                .entry(lake.minimum_cell)
                .or_insert(index);
        }
    }

    pub fn cell_count(&self) -> usize {
        self.elevation_mm.len()
    }

    fn compute_region_ranks(&self) -> RegionRanks {
        let mut landmass_sizes = BTreeMap::<u32, u32>::new();
        for (cell, id) in self.landmass_id.iter().enumerate() {
            if *id != TOOL_SURFACE_NONE && !self.ocean(cell as u32) {
                *landmass_sizes.entry(*id).or_default() += 1;
            }
        }
        let roots = receiver_roots(&self.receiver);
        let mut watershed_sizes = BTreeMap::<u32, u32>::new();
        for (cell, root) in roots.iter().enumerate() {
            if *root != TOOL_SURFACE_NONE && !self.ocean(cell as u32) {
                *watershed_sizes.entry(*root).or_default() += 1;
            }
        }
        RegionRanks {
            landmass: rank_map(landmass_sizes),
            watershed: rank_map(watershed_sizes),
        }
    }

    fn region_ranks(&self) -> &RegionRanks {
        self.region_ranks
            .get_or_init(|| self.compute_region_ranks())
    }

    /// Size rank of a landmass id on this surface, 1 is the largest.
    pub fn landmass_rank(&self, region_id: u32) -> u32 {
        self.region_ranks()
            .landmass
            .get(&region_id)
            .copied()
            .unwrap_or(0)
    }

    /// Size rank of a receiver-root watershed on this surface, 1 is the largest.
    pub fn watershed_rank(&self, outlet: u32) -> u32 {
        self.region_ranks()
            .watershed
            .get(&outlet)
            .copied()
            .unwrap_or(0)
    }

    pub fn resolution(&self) -> u32 {
        self.grid.n
    }

    pub fn radius_metres(&self) -> f64 {
        self.grid.radius_m
    }

    pub fn ocean(&self, cell: u32) -> bool {
        self.flags
            .get(cell as usize)
            .is_some_and(|flags| flags & FLAG_OCEAN != 0)
    }

    pub fn ocean_count(&self) -> usize {
        self.flags
            .iter()
            .filter(|flags| *flags & FLAG_OCEAN != 0)
            .count()
    }

    pub fn river(&self, cell: u32) -> bool {
        self.flags
            .get(cell as usize)
            .is_some_and(|flags| flags & FLAG_RIVER != 0)
    }

    pub fn ice(&self, cell: u32) -> bool {
        self.flags
            .get(cell as usize)
            .is_some_and(|flags| flags & FLAG_ICE != 0)
    }

    pub fn lake_id(&self, cell: u32) -> u32 {
        self.lake_id
            .get(cell as usize)
            .copied()
            .unwrap_or(TOOL_SURFACE_NONE)
    }

    pub fn is_lake(&self, cell: u32) -> bool {
        self.lake_id(cell) != TOOL_SURFACE_NONE
    }

    /// Cell of a fresh lake holding water at the surface epoch.
    pub fn fresh_lake(&self, cell: u32) -> bool {
        self.flags
            .get(cell as usize)
            .is_some_and(|flags| flags & FLAG_FRESH_LAKE != 0)
    }

    /// Cell of a fresh or saline lake holding water; a dry salt flat is not.
    pub fn wet_lake(&self, cell: u32) -> bool {
        self.flags
            .get(cell as usize)
            .is_some_and(|flags| flags & FLAG_WET_LAKE != 0)
    }

    /// Cell of a fresh lake that spills at the surface epoch.
    pub fn overflowing_lake(&self, cell: u32) -> bool {
        self.flags
            .get(cell as usize)
            .is_some_and(|flags| flags & FLAG_OVERFLOWING_LAKE != 0)
    }

    pub fn landmass_id(&self, cell: u32) -> u32 {
        self.landmass_id
            .get(cell as usize)
            .copied()
            .unwrap_or(TOOL_SURFACE_NONE)
    }

    pub fn receiver(&self, cell: u32) -> u32 {
        self.receiver
            .get(cell as usize)
            .copied()
            .unwrap_or(TOOL_SURFACE_NONE)
    }

    pub fn lake(&self, lake_id: u32) -> Option<&SurfaceLake> {
        self.lake_index
            .get(&lake_id)
            .and_then(|index| self.lakes.get(*index))
            .or_else(|| self.lakes.iter().find(|lake| lake.id == lake_id))
    }

    pub fn lake_at_minimum_cell(&self, minimum_cell: u32) -> Option<&SurfaceLake> {
        self.lake_minimum_index
            .get(&minimum_cell)
            .and_then(|index| self.lakes.get(*index))
            .or_else(|| {
                self.lakes
                    .iter()
                    .find(|lake| lake.minimum_cell == minimum_cell)
            })
    }

    pub fn river_at_source_cell(&self, source_cell: u32) -> Option<&SurfaceRiver> {
        self.rivers
            .iter()
            .filter(|river| river.source_cell == source_cell)
            .min_by_key(|river| (river.mouth_cell, river.cells.len()))
    }

    /// Nearest canonical cell to a latitude and longitude in degrees.
    pub fn cell_at_degrees(&self, latitude_deg: f64, longitude_deg: f64) -> u32 {
        self.grid.cell_at(direction(latitude_deg, longitude_deg))
    }

    /// Nearest canonical cell to micro-degree coordinates, `[lon, lat]`.
    pub fn cell_at_microdegrees(&self, longitude_micro: i32, latitude_micro: i32) -> u32 {
        self.cell_at_degrees(
            f64::from(latitude_micro) / 1_000_000.0,
            f64::from(longitude_micro) / 1_000_000.0,
        )
    }

    /// Nearest canonical cell to a unit direction.
    pub fn cell_at_direction(&self, direction: [f64; 3]) -> u32 {
        self.grid.cell_at(direction)
    }

    /// Unit direction of a cell centre.
    pub fn direction_of(&self, cell: u32) -> [f64; 3] {
        self.grid.position(cell)
    }

    /// Cell centre in micro-degrees, `[lon, lat]`.
    pub fn center_microdegrees(&self, cell: u32) -> [i32; 2] {
        let (lat, lon) = self.grid.lat_lon(cell);
        [
            (lon.to_degrees() * 1_000_000.0).round() as i32,
            (lat.to_degrees() * 1_000_000.0).round() as i32,
        ]
    }

    pub fn cell_lat_lon_degrees(&self, cell: u32) -> (f64, f64) {
        let (lat, lon) = self.grid.lat_lon(cell);
        (lat.to_degrees(), lon.to_degrees())
    }

    /// Eight neighbours in order E, NE, N, NW, W, SW, S, SE. Absent
    /// neighbours are [`TOOL_SURFACE_NONE`].
    pub fn neighbors(&self, cell: u32) -> [u32; 8] {
        self.grid.neighbors(cell)
    }

    /// Great-circle distance to neighbour `k`, metres.
    pub fn neighbor_distance_metres(&self, cell: u32, k: usize) -> u32 {
        self.grid
            .neighbor_distance(cell, k)
            .round()
            .clamp(1.0, u32::MAX as f32) as u32
    }

    /// Distance in metres between two adjacent cells.
    pub fn edge_distance_metres(&self, from: u32, to: u32) -> Option<u32> {
        self.neighbors(from)
            .iter()
            .position(|neighbor| *neighbor == to)
            .map(|k| self.neighbor_distance_metres(from, k))
    }

    pub fn area_m2(&self, cell: u32) -> f64 {
        self.grid.area_m2(cell)
    }

    /// Receiver-tree root (watershed outlet) of a cell.
    pub fn receiver_root(&self, cell: u32) -> u32 {
        let mut cursor = cell;
        for _ in 0..=self.receiver.len() {
            let next = self.receiver(cursor);
            if next == cursor || next == TOOL_SURFACE_NONE || next as usize >= self.receiver.len() {
                return cursor;
            }
            cursor = next;
        }
        cursor
    }

    pub fn heap_bytes(&self) -> usize {
        std::mem::size_of::<Self>()
            + self.elevation_mm.capacity() * std::mem::size_of::<i32>()
            + self.flags.capacity() * std::mem::size_of::<u8>()
            + self.slope_ppm.capacity() * std::mem::size_of::<u32>()
            + self.lake_id.capacity() * std::mem::size_of::<u32>()
            + self.biome_class.capacity() * std::mem::size_of::<u8>()
            + self.temperature_centi_c.capacity() * std::mem::size_of::<i32>()
            + self.precipitation_mm.capacity() * std::mem::size_of::<u32>()
            + self.pet_mm.capacity() * std::mem::size_of::<u32>()
            + self.earthquake_hazard.capacity() * std::mem::size_of::<u8>()
            + self.volcanic_hazard.capacity() * std::mem::size_of::<u8>()
            + self.storm_genesis_ppm.capacity() * std::mem::size_of::<u32>()
            + self.landmass_id.capacity() * std::mem::size_of::<u32>()
            + self.receiver.capacity() * std::mem::size_of::<u32>()
            + self.lakes.capacity() * std::mem::size_of::<SurfaceLake>()
            + self
                .rivers
                .iter()
                .map(|river| std::mem::size_of::<SurfaceRiver>() + river.cells.capacity() * 4)
                .sum::<usize>()
    }
}

fn direction(latitude_deg: f64, longitude_deg: f64) -> [f64; 3] {
    let lat = latitude_deg.clamp(-90.0, 90.0).to_radians();
    let lon = longitude_deg.to_radians();
    let cos_lat = lat.cos();
    [cos_lat * lon.cos(), cos_lat * lon.sin(), lat.sin()]
}

fn add_saturating_i32(value: i32, delta: i32) -> i32 {
    i64::from(value)
        .saturating_add(i64::from(delta))
        .clamp(i64::from(i32::MIN), i64::from(i32::MAX)) as i32
}

fn tool_surface_slope(
    grid: &worldgen::CubeSphere,
    elevation_mm: &[i32],
    cancelled: &(dyn Fn() -> bool + Sync),
) -> Result<Vec<u32>, PhysicalError> {
    let count = elevation_mm.len();
    let mut out = vec![0u32; count];
    let chunk = 65_536usize;
    for start in (0..count).step_by(chunk) {
        if cancelled() {
            return Err(PhysicalError::Cancelled);
        }
        let end = (start + chunk).min(count);
        out[start..end]
            .par_iter_mut()
            .enumerate()
            .for_each(|(offset, slot)| {
                let cell = (start + offset) as u32;
                let neighbors = grid.neighbors(cell);
                let height = |index: usize| {
                    let neighbor = neighbors[index];
                    if neighbor == worldgen::NONE {
                        None
                    } else {
                        Some(f64::from(elevation_mm[neighbor as usize]))
                    }
                };
                let axis = |first: usize, second: usize| -> f64 {
                    let low = height(first);
                    let high = height(second);
                    let centre = f64::from(elevation_mm[cell as usize]);
                    match (low, high) {
                        (Some(low), Some(high)) => {
                            let distance = f64::from(grid.neighbor_distance(cell, first))
                                + f64::from(grid.neighbor_distance(cell, second));
                            if distance <= 0.0 {
                                0.0
                            } else {
                                (high - low) / distance
                            }
                        }
                        (Some(low), None) => {
                            let distance = f64::from(grid.neighbor_distance(cell, first));
                            if distance <= 0.0 {
                                0.0
                            } else {
                                (centre - low) / distance
                            }
                        }
                        (None, Some(high)) => {
                            let distance = f64::from(grid.neighbor_distance(cell, second));
                            if distance <= 0.0 {
                                0.0
                            } else {
                                (high - centre) / distance
                            }
                        }
                        (None, None) => 0.0,
                    }
                };
                // Neighbour order is E, NE, N, NW, W, SW, S, SE. A cube-grid
                // central difference uses the true distance across both sides.
                let east_west = axis(4, 0);
                let north_south = axis(6, 2);
                *slot = (east_west.hypot(north_south) * 1000.0)
                    .round()
                    .clamp(0.0, f64::from(u32::MAX)) as u32;
            });
    }
    Ok(out)
}

fn tool_surface_landmasses(
    world: &worldgen::World,
    cancelled: &(dyn Fn() -> bool + Sync),
) -> Result<Vec<u32>, PhysicalError> {
    let count = world.ocean.len();
    let mut ids = vec![worldgen::NONE; count];
    let mut queue = VecDeque::new();
    let mut visited = 0usize;
    for start in 0..count {
        if start.is_multiple_of(65_536) && cancelled() {
            return Err(PhysicalError::Cancelled);
        }
        if world.ocean[start] != 0 || ids[start] != worldgen::NONE {
            continue;
        }
        let id = start as u32;
        ids[start] = id;
        queue.push_back(id);
        while let Some(cell) = queue.pop_front() {
            visited += 1;
            if visited.is_multiple_of(65_536) && cancelled() {
                return Err(PhysicalError::Cancelled);
            }
            for neighbor in world.grid.neighbors(cell) {
                let neighbor = neighbor as usize;
                if neighbor < count && world.ocean[neighbor] == 0 && ids[neighbor] == worldgen::NONE
                {
                    ids[neighbor] = id;
                    queue.push_back(neighbor as u32);
                }
            }
        }
    }
    Ok(ids)
}

fn tool_surface_storm_genesis(
    world: &worldgen::World,
    cancelled: &(dyn Fn() -> bool + Sync),
) -> Result<Vec<u32>, PhysicalError> {
    let coarse = &world.climate_field.grid;
    let genesis = &world.hazards.storms.genesis_km2;
    let count = world.grid.cell_count();
    let mut values = vec![0f32; count];
    for start in (0..count).step_by(65_536) {
        if cancelled() {
            return Err(PhysicalError::Cancelled);
        }
        let end = (start + 65_536).min(count);
        values[start..end]
            .par_iter_mut()
            .enumerate()
            .for_each(|(offset, slot)| {
                let cell = (start + offset) as u32;
                *slot = coarse.sample_bilinear(genesis, world.grid.position(cell));
            });
    }
    let maximum = values.iter().copied().fold(0.0f32, f32::max);
    for value in &mut values {
        *value = if maximum > 0.0 {
            (f64::from(*value) / f64::from(maximum) * 1_000_000.0).round() as f32
        } else {
            0.0
        };
    }
    Ok(values
        .into_iter()
        .map(|value| value.clamp(0.0, 1_000_000.0) as u32)
        .collect())
}

fn build_tool_surface(world: &worldgen::World, sea_level_mm: i32) -> ToolSurface {
    build_tool_surface_with_cancel(world, sea_level_mm, &|| false)
        .expect("non-cancellable tool surface build")
}

fn build_tool_surface_with_cancel(
    world: &worldgen::World,
    sea_level_mm: i32,
    cancelled: &(dyn Fn() -> bool + Sync),
) -> Result<ToolSurface, PhysicalError> {
    let count = world.grid.cell_count();
    if cancelled() {
        return Err(PhysicalError::Cancelled);
    }
    let mut elevation_mm = world.source_elevation_mm();
    for value in &mut elevation_mm {
        *value = add_saturating_i32(*value, sea_level_mm);
    }
    let river_min = world.settings.river_min_m3s;
    let lake_kinds = world
        .hydrology
        .lakes
        .iter()
        .map(|lake| (lake.id, lake.kind))
        .collect::<BTreeMap<_, _>>();
    let overflowing_lakes = world
        .hydrology
        .lakes
        .iter()
        .filter(|lake| lake.kind == worldgen::LakeKind::Fresh && lake.outflow_m3 > 0.0)
        .map(|lake| lake.id)
        .collect::<std::collections::BTreeSet<_>>();
    let mut flags = Vec::with_capacity(count);
    for cell in 0..count {
        if cell.is_multiple_of(65_536) && cancelled() {
            return Err(PhysicalError::Cancelled);
        }
        let mut value = 0u8;
        if world.ocean[cell] != 0 {
            value |= FLAG_OCEAN;
        }
        match lake_kinds.get(&world.hydrology.lake_id[cell]) {
            Some(worldgen::LakeKind::Fresh) => {
                value |= FLAG_FRESH_LAKE | FLAG_WET_LAKE;
                if overflowing_lakes.contains(&world.hydrology.lake_id[cell]) {
                    value |= FLAG_OVERFLOWING_LAKE;
                }
            }
            Some(worldgen::LakeKind::Saline) => value |= FLAG_WET_LAKE,
            _ => {}
        }
        if world.hydrology.lake_id[cell] == worldgen::NONE
            && world.hydrology.discharge_m3s[cell] >= river_min
        {
            value |= FLAG_RIVER;
        }
        if world.hydrology.ice_cover[cell] != 0 {
            value |= FLAG_ICE;
        }
        flags.push(value);
    }
    if cancelled() {
        return Err(PhysicalError::Cancelled);
    }
    let mut lake_cell_counts = BTreeMap::<u32, u32>::new();
    for id in &world.hydrology.lake_id {
        if *id != worldgen::NONE {
            *lake_cell_counts.entry(*id).or_default() += 1;
        }
    }
    let lakes = world
        .hydrology
        .lakes
        .iter()
        .map(|lake| SurfaceLake {
            id: lake.id,
            minimum_cell: lake.minimum_cell,
            spill_cell: lake.spill_cell,
            fresh: lake.kind == worldgen::LakeKind::Fresh,
            overflowing: lake.kind == worldgen::LakeKind::Fresh && lake.outflow_m3 > 0.0,
            has_water: lake.kind != worldgen::LakeKind::SaltFlat
                && lake_cell_counts.get(&lake.id).copied().unwrap_or(0) > 0,
            cell_count: lake_cell_counts.get(&lake.id).copied().unwrap_or(0),
        })
        .collect::<Vec<_>>();
    let river_order = products::river_strahler_orders(
        &world.hydrology.receiver,
        &world.hydrology.rivers,
        cancelled,
    )?;
    let mut rivers = Vec::with_capacity(world.hydrology.rivers.len());
    for river in &world.hydrology.rivers {
        if cancelled() {
            return Err(PhysicalError::Cancelled);
        }
        let Some(source_cell) = river.cells.first().copied() else {
            continue;
        };
        let Some(mouth_cell) = river.cells.last().copied() else {
            continue;
        };
        rivers.push(SurfaceRiver {
            source_cell,
            mouth_cell,
            strahler_order: river
                .cells
                .iter()
                .map(|cell| river_order.get(*cell as usize).copied().unwrap_or(0))
                .max()
                .unwrap_or(1)
                .max(1),
            cells: river.cells.clone(),
        });
    }
    let slope_ppm = tool_surface_slope(&world.grid, &elevation_mm, cancelled)?;
    let storm_genesis_ppm = tool_surface_storm_genesis(world, cancelled)?;
    let landmass_id = tool_surface_landmasses(world, cancelled)?;
    if cancelled() {
        return Err(PhysicalError::Cancelled);
    }
    let mut surface = ToolSurface {
        grid: world.grid.clone(),
        sea_level_mm,
        elevation_mm: elevation_mm.clone(),
        flags,
        slope_ppm,
        lake_id: world.hydrology.lake_id.clone(),
        biome_class: world.biome.clone(),
        temperature_centi_c: world
            .climate
            .t_annual_c
            .iter()
            .map(|value| products::centi_celsius(*value))
            .collect(),
        precipitation_mm: world
            .climate
            .p_annual_mm
            .iter()
            .map(|value| products::millimetres(*value))
            .collect(),
        pet_mm: world
            .climate
            .pet_annual_mm
            .iter()
            .map(|value| products::millimetres(*value))
            .collect(),
        earthquake_hazard: world.hazards.quake_hazard.clone(),
        volcanic_hazard: world.hazards.volcanic_hazard.clone(),
        storm_genesis_ppm,
        landmass_id,
        receiver: world.hydrology.receiver.clone(),
        lakes,
        rivers,
        lake_index: BTreeMap::new(),
        lake_minimum_index: BTreeMap::new(),
        region_ranks: OnceLock::new(),
    };
    surface.rebuild_lake_indexes();
    Ok(surface)
}

impl DecodedSource {
    /// Derive the present world and build its compact tool surface. The world
    /// is dropped before this returns, and its generation hashes are checked
    /// against the source.
    pub fn derive_tool_surface_present(
        &self,
        progress: &mut dyn ProgressSink,
    ) -> Result<ToolSurface, PhysicalError> {
        let mut adapter = EngineProgress::new(progress);
        let derived = worldgen::derive_world(
            &self.settings,
            &self.tectonics,
            &self.elevation_mm,
            &mut adapter,
        );
        adapter.finish()?;
        let world = derived.map_err(|error| map_world_error(error, true))?;
        let computed = source_hashes_with_cancel(&world, &world.source_elevation_mm(), progress)?;
        if let Some(field) = computed.mismatch(&self.hashes) {
            return Err(invalid(format!(
                "derived generation hash does not match the source: {field}"
            )));
        }
        Ok(build_tool_surface(&world, 0))
    }

    /// Derive one epoch world and build its compact tool surface.
    pub fn derive_tool_surface_epoch(
        &self,
        epoch_sea_level_mm: i32,
        temperature_offset_c: f64,
        progress: &mut dyn ProgressSink,
    ) -> Result<ToolSurface, PhysicalError> {
        let mut adapter = EngineProgress::new(progress);
        let derived = worldgen::derive_epoch(
            &self.settings,
            &self.tectonics,
            &self.elevation_mm,
            epoch_sea_level_mm,
            temperature_offset_c,
            &mut adapter,
        );
        adapter.finish()?;
        let world = derived.map_err(|error| map_world_error(error, true))?;
        Ok(build_tool_surface(&world, epoch_sea_level_mm))
    }

    /// Derive one Atlas world, checking every hash on the present path.
    fn derive_atlas_world(
        &self,
        epoch: Option<&crate::history::EpochSolve>,
        progress: &mut dyn ProgressSink,
    ) -> Result<worldgen::World, PhysicalError> {
        let present = match epoch {
            None => true,
            Some(solve) => solve.normalized_epoch == 0,
        };
        let mut adapter = EngineProgress::new(progress);
        let derived = if present {
            worldgen::derive_world(
                &self.settings,
                &self.tectonics,
                &self.elevation_mm,
                &mut adapter,
            )
        } else {
            let solve = epoch.expect("epoch solve");
            worldgen::derive_epoch(
                &self.settings,
                &self.tectonics,
                &self.elevation_mm,
                solve.sea_level_mm,
                f64::from(solve.temperature_offset_centi_c) / 100.0,
                &mut adapter,
            )
        };
        adapter.finish()?;
        let world = derived.map_err(|error| map_world_error(error, true))?;
        if present {
            let computed =
                source_hashes_with_cancel(&world, &world.source_elevation_mm(), progress)?;
            if let Some(field) = computed.mismatch(&self.hashes) {
                return Err(invalid(format!(
                    "derived generation hash does not match the source: {field}"
                )));
            }
        }
        Ok(world)
    }

    /// Derive the world for one Atlas epoch and build its product inputs.
    ///
    /// `None`, or a solve whose normalized epoch is zero, uses `derive_world`
    /// and verifies every source hash. An epoch uses `derive_epoch` with the
    /// solve's sea level and temperature offset and is not hash-checked. The
    /// view keeps reference elevations; `hydrology.sea_level_mm` carries the
    /// epoch sea level. The world is dropped before this returns.
    pub fn derive_atlas_inputs(
        &self,
        epoch: Option<&crate::history::EpochSolve>,
        view_width: u32,
        view_height: u32,
        planetary: PlanetaryConfiguration,
        progress: &mut dyn ProgressSink,
    ) -> Result<AtlasInputs, PhysicalError> {
        let world = self.derive_atlas_world(epoch, progress)?;
        let sea_level_mm = epoch.map_or(0, |solve| solve.sea_level_mm);
        let fields = build_world_product_fields(
            &world,
            sea_level_mm,
            planetary,
            view_width,
            view_height,
            progress,
        )?;
        let tectonics = tectonic_view(&world, view_width, view_height)?;
        progress.check_cancelled()?;
        let surface = build_tool_surface_with_cancel(&world, sea_level_mm, &|| {
            progress.check_cancelled().is_err()
        })?;
        Ok(AtlasInputs {
            view: fields.view,
            climate: fields.climate,
            hydrology: fields.hydrology,
            hazards: fields.hazards,
            tectonics,
            surface,
        })
    }

    /// Derive one Atlas epoch without building a [`ToolSurface`].
    pub fn derive_atlas_control_inputs(
        &self,
        epoch: Option<&crate::history::EpochSolve>,
        view_width: u32,
        view_height: u32,
        planetary: PlanetaryConfiguration,
        progress: &mut dyn ProgressSink,
    ) -> Result<AtlasControlInputs, PhysicalError> {
        let world = self.derive_atlas_world(epoch, progress)?;
        let sea_level_mm = epoch.map_or(0, |solve| solve.sea_level_mm);
        let fields = build_world_product_fields(
            &world,
            sea_level_mm,
            planetary,
            view_width,
            view_height,
            progress,
        )?;
        let tectonics = tectonic_view(&world, view_width, view_height)?;
        let volcanic_centers = world
            .hazards
            .volcanic_sources
            .iter()
            .map(|source| {
                let (latitude, longitude) = world.grid.lat_lon(source.cell);
                [
                    (longitude.to_degrees() * 1_000_000.0).round() as i32,
                    (latitude.to_degrees() * 1_000_000.0).round() as i32,
                ]
            })
            .collect();
        progress.check_cancelled()?;
        Ok(AtlasControlInputs {
            view: fields.view,
            climate: fields.climate,
            hydrology: fields.hydrology,
            hazards: fields.hazards,
            tectonics,
            volcanic_centers,
        })
    }
}

#[cfg(test)]
impl ToolSurface {
    /// Empty all-ocean surface for tool unit tests.
    pub(crate) fn synthetic(n: u32) -> Self {
        let grid = worldgen::CubeSphere::new(n, 6_371_000.0).expect("test grid");
        let count = grid.cell_count();
        Self {
            grid,
            sea_level_mm: 0,
            elevation_mm: vec![0; count],
            flags: vec![FLAG_OCEAN; count],
            slope_ppm: vec![0; count],
            lake_id: vec![TOOL_SURFACE_NONE; count],
            biome_class: vec![0; count],
            temperature_centi_c: vec![0; count],
            precipitation_mm: vec![0; count],
            pet_mm: vec![0; count],
            earthquake_hazard: vec![0; count],
            volcanic_hazard: vec![0; count],
            storm_genesis_ppm: vec![0; count],
            landmass_id: vec![TOOL_SURFACE_NONE; count],
            receiver: (0..count as u32).collect(),
            lakes: Vec::new(),
            rivers: Vec::new(),
            lake_index: BTreeMap::new(),
            lake_minimum_index: BTreeMap::new(),
            region_ranks: OnceLock::new(),
        }
    }

    pub(crate) fn test_set_land(&mut self, cell: u32) {
        self.flags[cell as usize] &= !FLAG_OCEAN;
    }

    pub(crate) fn test_set_ocean(&mut self, cell: u32) {
        self.flags[cell as usize] |= FLAG_OCEAN;
    }

    pub(crate) fn test_set_river(&mut self, cell: u32) {
        self.flags[cell as usize] |= FLAG_RIVER;
    }

    pub(crate) fn test_set_ice(&mut self, cell: u32) {
        self.flags[cell as usize] |= FLAG_ICE;
    }

    pub(crate) fn test_set_lake(&mut self, cell: u32, lake_id: u32, fresh: bool) {
        self.lake_id[cell as usize] = lake_id;
        self.flags[cell as usize] |= FLAG_WET_LAKE;
        if fresh {
            self.flags[cell as usize] |= FLAG_FRESH_LAKE;
        }
    }
}

/// Boundary kind in an [`TectonicView`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TectonicBoundaryKind {
    Convergent,
    Divergent,
    Transform,
}

/// One plate boundary as two micro-degree points on the product view.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TectonicBoundarySegment {
    pub kind: TectonicBoundaryKind,
    /// `[longitude, latitude]` in micro-degrees of the boundary cell.
    pub first: [i32; 2],
    /// `[longitude, latitude]` in micro-degrees of the neighbour cell.
    pub second: [i32; 2],
    pub first_continental: bool,
    pub second_continental: bool,
}

/// Product tectonic data for Atlas: continental cells and boundary segments.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TectonicView {
    /// Continental crust per view cell, nearest canonical sampling.
    pub continental: Vec<bool>,
    pub boundaries: Vec<TectonicBoundarySegment>,
}

/// One engine derive shaped for Atlas at an explicit equirectangular view size.
///
/// Elevations are the reference datum; `hydrology.sea_level_mm` is the epoch
/// sea level. The world is dropped before this returns.
pub struct AtlasInputs {
    pub view: PhysicalField,
    pub climate: products::ClimateField,
    pub hydrology: products::HydrologyField,
    pub hazards: products::HazardField,
    pub tectonics: TectonicView,
    pub surface: ToolSurface,
}

/// Atlas inputs that do not need a [`ToolSurface`].
///
/// The year-zero structure pass builds this instead of a full [`AtlasInputs`],
/// so an epoch prepare holds one tool surface, not two. Volcanic source centres
/// are resolved while the canonical grid is still available.
pub struct AtlasControlInputs {
    pub view: PhysicalField,
    pub climate: products::ClimateField,
    pub hydrology: products::HydrologyField,
    pub hazards: products::HazardField,
    pub tectonics: TectonicView,
    pub volcanic_centers: Vec<[i32; 2]>,
}

fn view_cell_direction(row: u32, col: u32, width: u32, height: u32) -> [f64; 3] {
    let lat = -90.0 + 180.0 * (f64::from(row) + 0.5) / f64::from(height);
    let lon = -180.0 + 360.0 * (f64::from(col) + 0.5) / f64::from(width);
    direction(lat, lon)
}

fn tectonic_view(
    world: &worldgen::World,
    width: u32,
    height: u32,
) -> Result<TectonicView, PhysicalError> {
    let count = (width as usize)
        .checked_mul(height as usize)
        .ok_or_else(|| invalid("atlas view cell count overflowed"))?;
    let mut continental = vec![false; count];
    continental
        .par_iter_mut()
        .enumerate()
        .for_each(|(index, slot)| {
            let row = (index / width as usize) as u32;
            let col = (index % width as usize) as u32;
            let cell = world
                .grid
                .cell_at(view_cell_direction(row, col, width, height));
            *slot = world.crust.get(cell as usize).copied().unwrap_or(0) == 1;
        });
    let boundaries = world
        .tectonics
        .boundaries
        .iter()
        .map(|boundary| {
            let (first_lat, first_lon) = world.grid.lat_lon(boundary.cell);
            let (second_lat, second_lon) = world.grid.lat_lon(boundary.neighbor);
            TectonicBoundarySegment {
                kind: match boundary.kind {
                    worldgen::BoundaryKind::Convergent => TectonicBoundaryKind::Convergent,
                    worldgen::BoundaryKind::Divergent => TectonicBoundaryKind::Divergent,
                    worldgen::BoundaryKind::Transform => TectonicBoundaryKind::Transform,
                },
                first: [
                    (first_lon.to_degrees() * 1_000_000.0).round() as i32,
                    (first_lat.to_degrees() * 1_000_000.0).round() as i32,
                ],
                second: [
                    (second_lon.to_degrees() * 1_000_000.0).round() as i32,
                    (second_lat.to_degrees() * 1_000_000.0).round() as i32,
                ],
                first_continental: boundary.crust_a == 1,
                second_continental: boundary.crust_b == 1,
            }
        })
        .collect::<Vec<_>>();
    Ok(TectonicView {
        continental,
        boundaries,
    })
}

/// Products built from one engine world.
pub struct Products {
    source: Vec<u8>,
    view: PhysicalField,
    climate: products::ClimateField,
    hydrology: products::HydrologyField,
    hazards: products::HazardField,
    derived_geojson: String,
    generation: Option<serde_json::Value>,
    hashes: SourceHashes,
    reference_water_inventory_m3: u64,
    resolution: u32,
    seed: u64,
}

/// One canonical cube-grid ocean volume sample.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OceanVolumePoint {
    pub sea_level_mm: i32,
    pub volume_m3: u64,
}

/// Derive the engine-backed product fields without a source envelope.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct DerivedProducts {
    pub view: PhysicalField,
    pub climate: products::ClimateField,
    pub hydrology: products::HydrologyField,
    pub hazards: products::HazardField,
    pub derived_geojson: String,
    pub hashes: SourceHashes,
}

impl Products {
    pub fn source(&self) -> &[u8] {
        &self.source
    }

    pub fn view(&self) -> &PhysicalField {
        &self.view
    }

    pub fn climate(&self) -> &products::ClimateField {
        &self.climate
    }

    pub fn hydrology(&self) -> &products::HydrologyField {
        &self.hydrology
    }

    pub fn hazards(&self) -> &products::HazardField {
        &self.hazards
    }

    pub fn derived_geojson(&self) -> &str {
        &self.derived_geojson
    }

    pub fn generation(&self) -> Option<&serde_json::Value> {
        self.generation.as_ref()
    }

    pub fn hashes(&self) -> SourceHashes {
        self.hashes
    }

    pub fn reference_water_inventory_m3(&self) -> u64 {
        self.reference_water_inventory_m3
    }

    pub fn resolution(&self) -> u32 {
        self.resolution
    }

    pub fn seed(&self) -> u64 {
        self.seed
    }
}

/// Generate a world with the engine and encode its canonical source.
pub fn generate(
    settings: Settings,
    progress: &mut dyn ProgressSink,
) -> Result<Products, PhysicalError> {
    settings.validate()?;
    let world_settings = settings.world_settings()?;
    let mut adapter = EngineProgress::new(progress);
    let generated = worldgen::generate(&world_settings, &mut adapter);
    adapter.finish()?;
    let world = generated.map_err(|error| map_world_error(error, false))?;
    let elevation_mm = world.source_elevation_mm();
    let hashes = source_hashes_with_cancel(&world, &elevation_mm, progress)?;
    let reference_water_inventory_m3 = world.reference_water_inventory_m3();
    let source = encode_parts(
        &world.settings,
        settings.preset,
        &elevation_mm,
        hashes,
        reference_water_inventory_m3,
    )?;
    let generation = Some(generation_json(
        &settings,
        &world.settings,
        reference_water_inventory_m3,
    ));
    let derived = build_world_products(&world, 0, settings.planetary, Some(hashes), progress)?;
    Ok(Products {
        source,
        view: derived.view,
        climate: derived.climate,
        hydrology: derived.hydrology,
        hazards: derived.hazards,
        derived_geojson: derived.derived_geojson,
        generation,
        hashes,
        reference_water_inventory_m3,
        resolution: world.settings.resolution,
        seed: world.settings.seed,
    })
}

/// Decode and validate a canonical source. This rebuilds tectonics and checks
/// the elevation and tectonic hashes, but does not run a full derive.
pub fn decode_source(bytes: &[u8]) -> Result<DecodedSource, PhysicalError> {
    let mut progress = crate::NoopProgress;
    decode_source_with_progress(bytes, &mut progress)
}

/// Decode with cancellation and progress.
pub fn decode_source_with_progress(
    bytes: &[u8],
    progress: &mut dyn ProgressSink,
) -> Result<DecodedSource, PhysicalError> {
    if bytes.len() < 68 {
        return Err(invalid("source is shorter than the fixed header"));
    }
    if bytes[0..8] != SOURCE_MAGIC {
        return Err(unsupported("source magic is not DAENAPW2"));
    }
    let format_version = read_u16(bytes, 8)?;
    if format_version != SOURCE_VERSION {
        return Err(unsupported(format!(
            "source version {format_version} is unsupported"
        )));
    }
    let header_bytes = read_u16(bytes, 10)? as usize;
    let engine_version = read_u32(bytes, 12)?;
    if engine_version != WORLD_ENGINE_VERSION {
        return Err(unsupported(format!(
            "engine version {engine_version} is unsupported"
        )));
    }
    let preset_code = bytes[16];
    let preset =
        WorldPreset::from_code(preset_code).ok_or_else(|| invalid("preset code is invalid"))?;
    let layout = bytes[17];
    let flags = read_u16(bytes, 18)?;
    if flags != 0 {
        return Err(invalid("reserved source flags are not zero"));
    }
    let resolution = read_u32(bytes, 20)?;
    let cell_count = read_u32(bytes, 24)?;
    let seed = read_u64(bytes, 28)?;
    let radius_m = read_f64(bytes, 36)?;
    let land_fraction = read_f64(bytes, 44)?;
    let plate_count = read_u32(bytes, 52)?;
    let tectonic_activity = read_f64(bytes, 56)?;
    let schedule_count = read_u32(bytes, 64)?;
    if !resolution.is_power_of_two() || !(32..=2048).contains(&resolution) {
        return Err(invalid(format!(
            "resolution must be a power of two in [32, 2048], got {resolution}"
        )));
    }
    let expected_cells = 6u64 * u64::from(resolution) * u64::from(resolution);
    if u64::from(cell_count) != expected_cells {
        return Err(invalid("cell count does not match the resolution"));
    }
    if schedule_count == 0 || schedule_count > 64 {
        return Err(invalid("schedule row count is out of range"));
    }
    let expected_header = SOURCE_FIXED_HEADER_BYTES
        .checked_add(
            (schedule_count as usize)
                .checked_mul(16)
                .ok_or_else(|| invalid("schedule length overflow"))?,
        )
        .ok_or_else(|| invalid("header length overflow"))?;
    if header_bytes != expected_header {
        return Err(invalid("header byte count does not match the schedule"));
    }
    let record_bytes = header_bytes
        .checked_add(
            (cell_count as usize)
                .checked_mul(4)
                .ok_or_else(|| invalid("elevation length overflow"))?,
        )
        .ok_or_else(|| invalid("source length overflow"))?;
    if bytes.len() != record_bytes {
        return Err(invalid(format!(
            "source length {} does not match the record length {record_bytes}",
            bytes.len()
        )));
    }

    let mut schedule_steps = Vec::with_capacity(schedule_count as usize);
    for index in 0..schedule_count as usize {
        let offset = 68 + index * 16;
        schedule_steps.push(worldgen::ScheduleStep {
            max_n: read_u32(bytes, offset)?,
            iterations: read_u32(bytes, offset + 4)?,
            dt_yr: read_f64(bytes, offset + 8)?,
        });
    }
    let base = 68 + schedule_count as usize * 16;
    let erosion = worldgen::ErosionParams {
        m: read_f64(bytes, base)?,
        k: read_f64(bytes, base + 8)?,
        s_c: read_f64(bytes, base + 16)?,
        kappa: read_f64(bytes, base + 24)?,
        max_elevation_m: read_f64(bytes, base + 32)?,
    };
    let planet = worldgen::PlanetSettings {
        star_luminosity: read_f64(bytes, base + 40)?,
        star_mass: read_f64(bytes, base + 48)?,
        orbit_au: read_f64(bytes, base + 56)?,
        eccentricity: read_f64(bytes, base + 64)?,
        perihelion_deg: read_f64(bytes, base + 72)?,
        axial_tilt_deg: read_f64(bytes, base + 80)?,
        rotation_hours: read_f64(bytes, base + 88)?,
        greenhouse_wm2: read_f64(bytes, base + 96)?,
        temperature_offset_c: read_f64(bytes, base + 104)?,
        precipitation_scale: read_f64(bytes, base + 112)?,
    };
    let river_min_m3s = read_f64(bytes, base + 120)?;
    let reference_water_inventory_m3 = read_u64(bytes, base + 128)?;
    let hashes = SourceHashes {
        elevation_mm: read_u64(bytes, base + 136)?,
        tectonics: read_u64(bytes, base + 144)?,
        climate: read_u64(bytes, base + 152)?,
        hydrology: read_u64(bytes, base + 160)?,
        biome: read_u64(bytes, base + 168)?,
        hazards: read_u64(bytes, base + 176)?,
    };

    let settings = worldgen::WorldSettings {
        seed,
        resolution,
        radius_m,
        land_fraction,
        plate_count,
        continent_layout: layout_from_code(layout)?,
        tectonic_activity,
        schedule: worldgen::Schedule {
            steps: schedule_steps,
        },
        erosion,
        planet,
        river_min_m3s,
    };
    settings
        .validate()
        .map_err(|error| invalid(error.to_string()))?;

    let mut elevation_mm = Vec::with_capacity(cell_count as usize);
    for index in 0..cell_count as usize {
        let offset = header_bytes + index * 4;
        elevation_mm.push(read_i32(bytes, offset)?);
    }
    if elevation_mm_hash(&elevation_mm) != hashes.elevation_mm {
        return Err(invalid("elevation hash does not match the source"));
    }

    let mut adapter = EngineProgress::new(progress);
    let tectonics = worldgen::generate_tectonics(&settings, &mut adapter);
    adapter.finish()?;
    let tectonics = tectonics.map_err(|error| map_world_error(error, true))?;
    if tectonics.hash_bits() != hashes.tectonics {
        return Err(invalid("tectonic hash does not match the source"));
    }

    let grid = worldgen::CubeSphere::new(resolution, radius_m)
        .map_err(|error| invalid(error.to_string()))?;
    let inventory = worldgen::ocean_volume_at(&grid, &elevation_mm, 0)
        .map_err(|error| invalid(error.to_string()))?;
    if inventory != reference_water_inventory_m3 {
        return Err(invalid(
            "reference water inventory does not match the canonical surface",
        ));
    }

    Ok(DecodedSource {
        settings,
        preset,
        elevation_mm,
        hashes,
        reference_water_inventory_m3,
        tectonics,
    })
}

/// Re-encode a decoded source byte-exactly.
pub fn encode_source(decoded: &DecodedSource) -> Result<Vec<u8>, PhysicalError> {
    encode_parts(
        &decoded.settings,
        decoded.preset,
        &decoded.elevation_mm,
        decoded.hashes,
        decoded.reference_water_inventory_m3,
    )
}

/// Decode, derive and verify every generation hash, then build product views.
pub fn reload(
    bytes: &[u8],
    planetary: PlanetaryConfiguration,
    progress: &mut dyn ProgressSink,
) -> Result<Products, PhysicalError> {
    let decoded = decode_source_with_progress(bytes, progress)?;
    let mut adapter = EngineProgress::new(progress);
    let derived = worldgen::derive_world(
        &decoded.settings,
        &decoded.tectonics,
        &decoded.elevation_mm,
        &mut adapter,
    );
    adapter.finish()?;
    let world = derived.map_err(|error| map_world_error(error, true))?;
    let computed = source_hashes_with_cancel(&world, &decoded.elevation_mm, progress)?;
    if let Some(field) = computed.mismatch(&decoded.hashes) {
        return Err(invalid(format!(
            "derived generation hash does not match the source: {field}"
        )));
    }
    let elevation_mm = world.source_elevation_mm();
    let source = encode_parts(
        &world.settings,
        decoded.preset,
        &elevation_mm,
        decoded.hashes,
        decoded.reference_water_inventory_m3,
    )?;
    if source != bytes {
        return Err(invalid("source is not in canonical form"));
    }
    let derived = build_world_products(&world, 0, planetary, Some(decoded.hashes), progress)?;
    Ok(Products {
        source,
        view: derived.view,
        climate: derived.climate,
        hydrology: derived.hydrology,
        hazards: derived.hazards,
        derived_geojson: derived.derived_geojson,
        generation: None,
        hashes: decoded.hashes,
        reference_water_inventory_m3: decoded.reference_water_inventory_m3,
        resolution: decoded.settings.resolution,
        seed: decoded.settings.seed,
    })
}

/// Build every engine-backed product field from one finished world.
///
/// `sea_level_mm` is zero for the present world. For an epoch it is the solved
/// epoch sea level; the product view keeps reference elevations and stores the
/// epoch level in `PhysicalField::sea_level_mm`.
pub fn build_world_products(
    world: &worldgen::World,
    sea_level_mm: i32,
    planetary: PlanetaryConfiguration,
    hashes: Option<SourceHashes>,
    progress: &mut dyn ProgressSink,
) -> Result<DerivedProducts, PhysicalError> {
    build_world_products_with_view(
        world,
        sea_level_mm,
        planetary,
        hashes,
        PRODUCT_VIEW_WIDTH,
        PRODUCT_VIEW_HEIGHT,
        progress,
    )
}

/// Same as [`build_world_products`] with an explicit view size. E5 uses this
/// only to measure the GeoJSON limits at 2048×1024; production stays on the
/// recorded view constants.
pub fn build_world_products_with_view(
    world: &worldgen::World,
    sea_level_mm: i32,
    planetary: PlanetaryConfiguration,
    hashes: Option<SourceHashes>,
    width: u32,
    height: u32,
    progress: &mut dyn ProgressSink,
) -> Result<DerivedProducts, PhysicalError> {
    let fields =
        build_world_product_fields(world, sea_level_mm, planetary, width, height, progress)?;
    let derived_geojson = crate::to_geojson_with_progress(&fields.view, progress)?;
    let hashes = match hashes {
        Some(hashes) => hashes,
        None => source_hashes_with_cancel(world, &world.source_elevation_mm(), progress)?,
    };
    Ok(DerivedProducts {
        view: fields.view,
        climate: fields.climate,
        hydrology: fields.hydrology,
        hazards: fields.hazards,
        derived_geojson,
        hashes,
    })
}

struct WorldProductFields {
    view: PhysicalField,
    climate: products::ClimateField,
    hydrology: products::HydrologyField,
    hazards: products::HazardField,
}

/// The view, climate, hydrology and hazard products of one world, without
/// GeoJSON. Shared by [`build_world_products_with_view`] and the Atlas inputs
/// so the two derivations cannot drift.
fn build_world_product_fields(
    world: &worldgen::World,
    sea_level_mm: i32,
    planetary: PlanetaryConfiguration,
    width: u32,
    height: u32,
    progress: &mut dyn ProgressSink,
) -> Result<WorldProductFields, PhysicalError> {
    if progress.check_cancelled().is_err() {
        return Err(PhysicalError::Cancelled);
    }
    let cancelled = || progress.check_cancelled().is_err();
    let raster = products::build_view_raster(world, width, height, &cancelled)?;
    let view = PhysicalField {
        grid: raster.grid,
        seed: (world.settings.seed & u64::from(u32::MAX)) as u32,
        retry_index: 0,
        target_land_fraction_ppm: (world.settings.land_fraction * 1_000_000.0)
            .round()
            .clamp(1.0, 999_999.0) as u32,
        sea_level_mm,
        elevations_mm: raster
            .elevations_mm
            .iter()
            .map(|value| value.saturating_add(sea_level_mm))
            .collect(),
    };
    view.validate().map_err(PhysicalError::InvalidSettings)?;
    progress.check_cancelled()?;
    let climate = products::build_climate(world, &raster, planetary)?;
    progress.check_cancelled()?;
    let hydrology = products::build_hydrology(world, &raster, &view, &cancelled)?;
    progress.check_cancelled()?;
    let hazards = products::build_hazards(world, &raster, &cancelled)?;
    progress.check_cancelled()?;
    Ok(WorldProductFields {
        view,
        climate,
        hydrology,
        hazards,
    })
}

fn encode_parts(
    settings: &worldgen::WorldSettings,
    preset: WorldPreset,
    elevation_mm: &[i32],
    hashes: SourceHashes,
    reference_water_inventory_m3: u64,
) -> Result<Vec<u8>, PhysicalError> {
    settings
        .validate()
        .map_err(|error| invalid(error.to_string()))?;
    let cell_count = 6u64
        .checked_mul(u64::from(settings.resolution))
        .and_then(|value| value.checked_mul(u64::from(settings.resolution)))
        .ok_or_else(|| invalid("cell count overflow"))?;
    if elevation_mm.len() as u64 != cell_count {
        return Err(invalid("elevation count does not match the resolution"));
    }
    if elevation_mm_hash(elevation_mm) != hashes.elevation_mm {
        return Err(invalid("elevation hash does not match the encoded surface"));
    }
    let schedule_count = settings.schedule.steps.len();
    if schedule_count == 0 || schedule_count > 64 {
        return Err(invalid("schedule row count is out of range"));
    }
    let header_bytes = SOURCE_FIXED_HEADER_BYTES + schedule_count * 16;
    let record_bytes = header_bytes
        .checked_add(elevation_mm.len() * 4)
        .ok_or_else(|| invalid("source length overflow"))?;
    if record_bytes > crate::MAX_SOURCE_BYTES {
        return Err(PhysicalError::coded(
            PhysicalErrorCode::LimitExceeded,
            format!("source exceeds the {} byte budget", crate::MAX_SOURCE_BYTES),
        ));
    }
    let mut bytes = Vec::with_capacity(record_bytes);
    bytes.extend_from_slice(&SOURCE_MAGIC);
    bytes.extend_from_slice(&SOURCE_VERSION.to_le_bytes());
    bytes.extend_from_slice(&(header_bytes as u16).to_le_bytes());
    bytes.extend_from_slice(&WORLD_ENGINE_VERSION.to_le_bytes());
    bytes.push(preset.code());
    bytes.push(layout_code(settings.continent_layout));
    bytes.extend_from_slice(&0u16.to_le_bytes());
    bytes.extend_from_slice(&settings.resolution.to_le_bytes());
    bytes.extend_from_slice(&(cell_count as u32).to_le_bytes());
    bytes.extend_from_slice(&settings.seed.to_le_bytes());
    bytes.extend_from_slice(&settings.radius_m.to_bits().to_le_bytes());
    bytes.extend_from_slice(&settings.land_fraction.to_bits().to_le_bytes());
    bytes.extend_from_slice(&settings.plate_count.to_le_bytes());
    bytes.extend_from_slice(&settings.tectonic_activity.to_bits().to_le_bytes());
    bytes.extend_from_slice(&(schedule_count as u32).to_le_bytes());
    for step in &settings.schedule.steps {
        bytes.extend_from_slice(&step.max_n.to_le_bytes());
        bytes.extend_from_slice(&step.iterations.to_le_bytes());
        bytes.extend_from_slice(&step.dt_yr.to_bits().to_le_bytes());
    }
    for value in [
        settings.erosion.m,
        settings.erosion.k,
        settings.erosion.s_c,
        settings.erosion.kappa,
        settings.erosion.max_elevation_m,
        settings.planet.star_luminosity,
        settings.planet.star_mass,
        settings.planet.orbit_au,
        settings.planet.eccentricity,
        settings.planet.perihelion_deg,
        settings.planet.axial_tilt_deg,
        settings.planet.rotation_hours,
        settings.planet.greenhouse_wm2,
        settings.planet.temperature_offset_c,
        settings.planet.precipitation_scale,
        settings.river_min_m3s,
    ] {
        bytes.extend_from_slice(&value.to_bits().to_le_bytes());
    }
    bytes.extend_from_slice(&reference_water_inventory_m3.to_le_bytes());
    for hash in [
        hashes.elevation_mm,
        hashes.tectonics,
        hashes.climate,
        hashes.hydrology,
        hashes.biome,
        hashes.hazards,
    ] {
        bytes.extend_from_slice(&hash.to_le_bytes());
    }
    if bytes.len() != header_bytes {
        return Err(invalid(
            "encoded header length does not match its declaration",
        ));
    }
    for elevation in elevation_mm {
        bytes.extend_from_slice(&elevation.to_le_bytes());
    }
    Ok(bytes)
}

fn generation_json(
    settings: &Settings,
    resolved: &worldgen::WorldSettings,
    reference_water_inventory_m3: u64,
) -> serde_json::Value {
    let forcing = crate::history::HistoricalForcingParameters::default_for_seed(settings.seed);
    json!({
        "id": GENERATOR_ID,
        "version": GENERATOR_VERSION,
        "seed": settings.seed,
        "settings": {
            "resolution": resolved.resolution,
            "radiusMetres": settings.radius_metres,
            "targetLandFractionPpm": (resolved.land_fraction * 1_000_000.0).round() as u32,
            "referenceWaterInventoryM3": reference_water_inventory_m3,
            "preset": settings.preset.as_str(),
            "engineVersion": WORLD_ENGINE_VERSION,
            "sourceVersion": SOURCE_VERSION,
            "planetary": settings.planetary,
            "historicalForcing": forcing_json(forcing),
            "hazardDerivationVersion": crate::products::HAZARD_DERIVATION_VERSION,
        }
    })
}

fn forcing_json(parameters: crate::history::HistoricalForcingParameters) -> serde_json::Value {
    json!({
        "version": parameters.version,
        "components": parameters.components.iter().map(|component| json!({
            "amplitudeCentiC": component.amplitude_centi_c,
            "periodYears": component.period_years,
            "phaseOffsetYears": component.phase_offset_years,
        })).collect::<Vec<_>>(),
        "sensitivityPpm": parameters.sensitivity_ppm,
        "landIceAmplitudePpm": parameters.land_ice_amplitude_ppm,
        "iceResponseYears": parameters.ice_response_years,
        "iceMidpointCentiC": parameters.ice_midpoint_centi_c,
        "iceTransitionWidthCentiC": parameters.ice_transition_width_centi_c,
        "thermalExpansionPpmPerDegreeC": parameters.thermal_expansion_ppm_per_degree_c,
    })
}

struct EngineProgress<'a> {
    sink: &'a mut dyn ProgressSink,
    failure: Option<PhysicalError>,
}

impl<'a> EngineProgress<'a> {
    fn new(sink: &'a mut dyn ProgressSink) -> Self {
        Self {
            sink,
            failure: None,
        }
    }

    fn finish(self) -> Result<(), PhysicalError> {
        if let Some(error) = self.failure {
            return Err(error);
        }
        self.sink.check_cancelled()
    }
}

impl worldgen::Progress for EngineProgress<'_> {
    fn report(&mut self, stage: worldgen::Stage, done: u32, total: u32) {
        if self.failure.is_some() {
            return;
        }
        let phase = phase_for(stage);
        if let Err(error) = self.sink.report(phase, done, total) {
            self.failure = Some(error);
        }
    }

    fn cancelled(&self) -> bool {
        self.failure.is_some() || self.sink.check_cancelled().is_err()
    }
}

fn phase_for(stage: worldgen::Stage) -> ProgressPhase {
    match stage {
        worldgen::Stage::Tectonics => ProgressPhase::BuildingTectonicStructure,
        worldgen::Stage::Level(_) => ProgressPhase::ErodingLandscape,
        worldgen::Stage::SeaLevel => ProgressPhase::BuildingTerrain,
        worldgen::Stage::FinalRouting => ProgressPhase::CalculatingWater,
        worldgen::Stage::Climate => ProgressPhase::CalculatingClimate,
        worldgen::Stage::Hydrology => ProgressPhase::BuildingRiversAndLakes,
        worldgen::Stage::Biomes => ProgressPhase::PreparingGeography,
        worldgen::Stage::Hazards => ProgressPhase::ValidatingWorld,
    }
}

fn map_world_error(error: worldgen::WorldError, source: bool) -> PhysicalError {
    match error {
        worldgen::WorldError::Cancelled => PhysicalError::Cancelled,
        worldgen::WorldError::InvalidSettings(message) => {
            if source {
                invalid(message)
            } else {
                PhysicalError::InvalidSettings(message)
            }
        }
        worldgen::WorldError::Internal(message) => {
            if source {
                invalid(message)
            } else {
                PhysicalError::Validation(message)
            }
        }
    }
}

fn layout_code(layout: worldgen::ContinentLayout) -> u8 {
    match layout {
        worldgen::ContinentLayout::Mixed => 0,
        worldgen::ContinentLayout::Pangea => 1,
        worldgen::ContinentLayout::Few => 2,
        worldgen::ContinentLayout::Many => 3,
    }
}

fn layout_from_code(code: u8) -> Result<worldgen::ContinentLayout, PhysicalError> {
    match code {
        0 => Ok(worldgen::ContinentLayout::Mixed),
        1 => Ok(worldgen::ContinentLayout::Pangea),
        2 => Ok(worldgen::ContinentLayout::Few),
        3 => Ok(worldgen::ContinentLayout::Many),
        _ => Err(invalid("continent layout code is invalid")),
    }
}

fn elevation_mm_hash(values: &[i32]) -> u64 {
    let mut hash = FNV_OFFSET;
    for value in values {
        for byte in value.to_le_bytes() {
            hash ^= u64::from(byte);
            hash = hash.wrapping_mul(FNV_PRIME);
        }
    }
    hash
}

/// Chunked, cancellable source hashing from the engine.
///
/// The elevation argument must be the world's canonical source elevation or
/// the identical decoded source slice; it is hashed as given.
pub fn source_hashes_with_cancel(
    world: &worldgen::World,
    elevation_mm: &[i32],
    progress: &dyn ProgressSink,
) -> Result<SourceHashes, PhysicalError> {
    progress.check_cancelled()?;
    debug_assert_eq!(elevation_mm, world.source_elevation_mm());
    let hashes = world
        .generation_hashes_with_cancel(elevation_mm, &|| progress.check_cancelled().is_err())
        .map_err(|error| match error {
            worldgen::WorldError::Cancelled => PhysicalError::Cancelled,
            other => PhysicalError::Validation(other.to_string()),
        })?;
    Ok(SourceHashes {
        elevation_mm: hashes.elevation_mm,
        tectonics: hashes.tectonics,
        climate: hashes.climate,
        hydrology: hashes.hydrology,
        biome: hashes.biome,
        hazards: hashes.hazards,
    })
}

fn invalid(message: impl Into<String>) -> PhysicalError {
    PhysicalError::coded(PhysicalErrorCode::SourceInvalid, message)
}

fn unsupported(message: impl Into<String>) -> PhysicalError {
    PhysicalError::coded(PhysicalErrorCode::SourceUnsupportedVersion, message)
}

fn read_u16(bytes: &[u8], offset: usize) -> Result<u16, PhysicalError> {
    let value = bytes
        .get(offset..offset + 2)
        .ok_or_else(|| invalid("source is truncated"))?;
    Ok(u16::from_le_bytes(value.try_into().map_err(|_| {
        invalid("source field has an invalid width")
    })?))
}

fn read_u32(bytes: &[u8], offset: usize) -> Result<u32, PhysicalError> {
    let value = bytes
        .get(offset..offset + 4)
        .ok_or_else(|| invalid("source is truncated"))?;
    Ok(u32::from_le_bytes(value.try_into().map_err(|_| {
        invalid("source field has an invalid width")
    })?))
}

fn read_i32(bytes: &[u8], offset: usize) -> Result<i32, PhysicalError> {
    let value = bytes
        .get(offset..offset + 4)
        .ok_or_else(|| invalid("source is truncated"))?;
    Ok(i32::from_le_bytes(value.try_into().map_err(|_| {
        invalid("source field has an invalid width")
    })?))
}

fn read_u64(bytes: &[u8], offset: usize) -> Result<u64, PhysicalError> {
    let value = bytes
        .get(offset..offset + 8)
        .ok_or_else(|| invalid("source is truncated"))?;
    Ok(u64::from_le_bytes(value.try_into().map_err(|_| {
        invalid("source field has an invalid width")
    })?))
}

fn read_f64(bytes: &[u8], offset: usize) -> Result<f64, PhysicalError> {
    Ok(f64::from_bits(read_u64(bytes, offset)?))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::NoopProgress;
    use std::time::Instant;

    fn settings(resolution: u32) -> Settings {
        Settings::from_parts(
            831_429,
            WorldPreset::Earthlike,
            resolution,
            6_371_000,
            290_000,
            PlanetaryConfiguration::earth_like(),
        )
    }

    fn record_measurement(line: &str) {
        use std::io::Write;
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../target/e5a-measurements.txt");
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .expect("measurement file");
        writeln!(file, "{line}").expect("measurement line");
    }

    #[test]
    fn build_world_products_stays_byte_identical() {
        fn digest(products: &DerivedProducts) -> u64 {
            let mut hash = FNV_OFFSET;
            for byte in format!("{products:?}").bytes() {
                hash ^= u64::from(byte);
                hash = hash.wrapping_mul(FNV_PRIME);
            }
            hash
        }
        let generated = generate(settings(64), &mut NoopProgress).unwrap();
        let derived = DerivedProducts {
            view: generated.view().clone(),
            climate: generated.climate().clone(),
            hydrology: generated.hydrology().clone(),
            hazards: generated.hazards().clone(),
            derived_geojson: generated.derived_geojson().to_string(),
            hashes: generated.hashes(),
        };
        // E6 changed `HAZARD_DERIVATION_VERSION` and
        // `VOLCANIC_SOURCE_DERIVATION_VERSION` from 1 to 2; the hazard metrics
        // carry both values, so the pinned digest moved with them.
        assert_eq!(
            digest(&derived),
            12_582_050_379_977_442_768,
            "build_world_products output changed; regenerate only with a written reason"
        );
    }

    #[test]
    fn source_round_trip_is_byte_exact_and_length_strict() {
        let generated = generate(settings(64), &mut NoopProgress).unwrap();
        let source = generated.source().to_vec();
        let decoded = decode_source(&source).unwrap();
        assert_eq!(encode_source(&decoded).unwrap(), source);
        assert_eq!(decoded.source_version(), SOURCE_VERSION);
        assert_eq!(decoded.engine_version(), WORLD_ENGINE_VERSION);
        assert_eq!(
            u16::from_le_bytes(source[10..12].try_into().unwrap()) as usize,
            SOURCE_HEADER_BYTES
        );
        assert_eq!(source.len(), SOURCE_HEADER_BYTES + 4 * 64 * 64 * 6);

        let mut trailing = source.clone();
        trailing.push(0);
        assert_eq!(
            decode_source(&trailing).unwrap_err().code(),
            crate::CODE_SOURCE_INVALID
        );
        assert_eq!(
            decode_source(&source[..source.len() - 1])
                .unwrap_err()
                .code(),
            crate::CODE_SOURCE_INVALID
        );
    }

    #[test]
    fn cancellable_source_hashes_match_the_plain_engine_path() {
        let world_settings = settings(64).world_settings().unwrap();
        let progress = NoopProgress;
        let world = worldgen::generate(&world_settings, &mut worldgen::NoProgress).unwrap();
        let elevation = world.source_elevation_mm();
        let cancelled = source_hashes_with_cancel(&world, &elevation, &progress).unwrap();
        let plain = world.generation_hashes();
        assert_eq!(cancelled.elevation_mm, plain.elevation_mm);
        assert_eq!(cancelled.tectonics, plain.tectonics);
        assert_eq!(cancelled.climate, plain.climate);
        assert_eq!(cancelled.hydrology, plain.hydrology);
        assert_eq!(cancelled.biome, plain.biome);
        assert_eq!(cancelled.hazards, plain.hazards);
    }

    #[test]
    #[ignore = "wall-clock: release-only generation budget"]
    fn e8_generation_meets_the_time_budget_at_1024() {
        let started = Instant::now();
        let products = generate(settings(1024), &mut NoopProgress).unwrap();
        let elapsed = started.elapsed();
        println!(
            "E8 n=1024 generation {:?}, source {} bytes, geojson {} bytes, {} hashes",
            elapsed,
            products.source().len(),
            products.derived_geojson().len(),
            6
        );
        assert!(
            elapsed.as_millis() < crate::GENERATION_TIME_BUDGET_MS,
            "generation took {elapsed:?}, budget {} ms",
            crate::GENERATION_TIME_BUDGET_MS
        );
    }

    #[test]
    #[ignore = "wall-clock: release-only hash cancellation deadline sweep"]
    fn e8_hash_cancellation_stays_under_the_deadline_budget() {
        let world_settings = settings(1024).world_settings().unwrap();
        let world = worldgen::generate(&world_settings, &mut worldgen::NoProgress).unwrap();
        let elevation = world.source_elevation_mm();
        let started = Instant::now();
        world
            .generation_hashes_with_cancel(&elevation, &|| false)
            .unwrap();
        let hash_duration = started.elapsed();
        let mut worst = std::time::Duration::ZERO;
        let mut worst_deadline = std::time::Duration::ZERO;
        // Deadlines up to 90 % of the uncancelled time; a deadline at 100 %
        // races the hash finishing and proves nothing about latency.
        for step in 0..10 {
            let deadline = hash_duration.mul_f64(f64::from(step) / 10.0);
            let started = Instant::now();
            let result =
                world.generation_hashes_with_cancel(&elevation, &|| started.elapsed() >= deadline);
            let elapsed = started.elapsed();
            assert_eq!(result, Err(worldgen::WorldError::Cancelled));
            let late = elapsed.saturating_sub(deadline);
            if late > worst {
                worst = late;
                worst_deadline = deadline;
            }
        }
        println!(
            "E8 hash sweep: plain {hash_duration:?}, worst late {worst:?} at deadline {worst_deadline:?}"
        );
        assert!(
            worst.as_millis() < crate::CANCELLATION_LATENCY_BUDGET_MS,
            "hash cancellation was {worst:?} late at {worst_deadline:?}"
        );
    }

    #[test]
    fn coastline_segments_have_their_own_limit_above_the_feature_cap() {
        // Coastline segments are counted on the fixed 1024x512 view, so even a
        // small world can carry more of them than the top-level feature cap.
        // In the E8 review the shared 4,096 cap made seed 4 (n=512) fail to
        // generate; the sweep's worst world has 8,820 segments.
        let mut most = 0;
        for preset in [WorldPreset::Shattered, WorldPreset::Archipelago] {
            let settings = Settings::from_parts(
                4,
                preset,
                128,
                6_371_000,
                0,
                PlanetaryConfiguration::earth_like(),
            );
            let products = generate(settings, &mut NoopProgress).unwrap();
            most = most.max(products.hydrology().coastline_segments.len());
        }
        assert!(
            most > crate::MAX_GEOJSON_FEATURES,
            "expected a world with more coastline segments than features, got {most}"
        );
        assert!(most <= crate::MAX_COASTLINE_SEGMENTS);
    }

    #[test]
    fn generation_hash_values_are_pinned() {
        let world_settings = settings(32).world_settings().unwrap();
        let world = worldgen::generate(&world_settings, &mut worldgen::NoProgress).unwrap();
        let hashes = world.generation_hashes();
        assert_eq!(hashes.elevation_mm, 14_057_032_883_735_868_487);
        assert_eq!(hashes.tectonics, 2_178_534_385_459_862_412);
        assert_eq!(hashes.climate, 17_281_621_976_253_921_028);
        assert_eq!(hashes.hydrology, 5_046_656_125_177_288_775);
        assert_eq!(hashes.biome, 12_713_969_144_926_352_957);
        assert_eq!(hashes.hazards, 5_626_183_643_324_330_689);
    }

    #[test]
    fn decode_and_derive_reproduce_every_generation_hash() {
        let generated = generate(settings(64), &mut NoopProgress).unwrap();
        let reloaded = reload(
            generated.source(),
            PlanetaryConfiguration::earth_like(),
            &mut NoopProgress,
        )
        .unwrap();
        assert_eq!(reloaded.hashes(), generated.hashes());
    }

    #[test]
    fn source_only_rebuild_reproduces_every_product_field() {
        let generated = generate(settings(64), &mut NoopProgress).unwrap();
        let reloaded = reload(
            generated.source(),
            PlanetaryConfiguration::earth_like(),
            &mut NoopProgress,
        )
        .unwrap();
        assert_eq!(reloaded.view(), generated.view());
        assert_eq!(reloaded.climate(), generated.climate());
        assert_eq!(reloaded.hydrology(), generated.hydrology());
        assert_eq!(reloaded.hazards(), generated.hazards());
        assert_eq!(reloaded.derived_geojson(), generated.derived_geojson());
    }

    #[test]
    fn corrupt_and_unsupported_sources_are_rejected_with_the_right_codes() {
        let generated = generate(settings(64), &mut NoopProgress).unwrap();
        let source = generated.source().to_vec();

        let mut elevation = source.clone();
        let last = elevation.len() - 1;
        elevation[last] ^= 1;
        assert_eq!(
            decode_source(&elevation).unwrap_err().code(),
            crate::CODE_SOURCE_INVALID
        );

        let mut tectonic = source.clone();
        let offset = SOURCE_HEADER_BYTES - 40;
        tectonic[offset] ^= 1;
        assert_eq!(
            decode_source(&tectonic).unwrap_err().code(),
            crate::CODE_SOURCE_INVALID
        );

        let mut setting = source.clone();
        let offset = 44;
        setting[offset..offset + 8].copy_from_slice(&1.5f64.to_bits().to_le_bytes());
        assert_eq!(
            decode_source(&setting).unwrap_err().code(),
            crate::CODE_SOURCE_INVALID
        );

        let mut old_magic = source.clone();
        old_magic[0..8].copy_from_slice(b"DAENAPW1");
        assert_eq!(
            decode_source(&old_magic).unwrap_err().code(),
            crate::CODE_SOURCE_UNSUPPORTED_VERSION
        );

        let mut old_version = source.clone();
        old_version[8..10].copy_from_slice(&1u16.to_le_bytes());
        assert_eq!(
            decode_source(&old_version).unwrap_err().code(),
            crate::CODE_SOURCE_UNSUPPORTED_VERSION
        );

        let mut old_engine = source.clone();
        old_engine[12..16].copy_from_slice(&(WORLD_ENGINE_VERSION + 1).to_le_bytes());
        assert_eq!(
            decode_source(&old_engine).unwrap_err().code(),
            crate::CODE_SOURCE_UNSUPPORTED_VERSION
        );
    }

    #[test]
    fn zero_land_fraction_uses_the_preset_and_an_explicit_value_overrides_it() {
        let water = Settings::from_parts(
            1,
            WorldPreset::WaterWorld,
            512,
            6_371_000,
            0,
            PlanetaryConfiguration::earth_like(),
        );
        assert_eq!(water.snapshot().unwrap().land_fraction, 0.02);
        let pangea = Settings::from_parts(
            1,
            WorldPreset::Pangea,
            512,
            6_371_000,
            0,
            PlanetaryConfiguration::earth_like(),
        );
        assert_eq!(pangea.snapshot().unwrap().land_fraction, 0.30);
        let overridden = Settings::from_parts(
            1,
            WorldPreset::WaterWorld,
            512,
            6_371_000,
            450_000,
            PlanetaryConfiguration::earth_like(),
        );
        assert_eq!(overridden.snapshot().unwrap().land_fraction, 0.45);
    }

    #[test]
    fn rejected_resolutions_are_invalid_settings_for_the_product() {
        let mut request = settings(512);
        request.resolution = 256;
        assert!(request.validate_product().is_err());
        request.resolution = 64;
        assert!(request.validate().is_ok());
        assert!(request.validate_product().is_err());
    }

    #[test]
    fn tool_surface_matches_the_derived_world() {
        let generated = generate(settings(64), &mut NoopProgress).unwrap();
        let decoded = decode_source(generated.source()).unwrap();
        let world = worldgen::derive_world(
            &decoded.settings,
            &decoded.tectonics,
            &decoded.elevation_mm,
            &mut worldgen::NoProgress,
        )
        .unwrap();
        let surface = build_tool_surface(&world, 0);
        assert_eq!(surface.cell_count(), world.grid.cell_count());
        assert_eq!(surface.elevation_mm, world.source_elevation_mm());
        assert_eq!(surface.lake_id, world.hydrology.lake_id);
        assert_eq!(surface.biome_class, world.biome);
        assert_eq!(surface.earthquake_hazard, world.hazards.quake_hazard);
        assert_eq!(surface.volcanic_hazard, world.hazards.volcanic_hazard);
        assert_eq!(surface.receiver, world.hydrology.receiver);
        for cell in 0..world.grid.cell_count() {
            assert_eq!(surface.ocean(cell as u32), world.ocean[cell] != 0);
            assert_eq!(
                surface.river(cell as u32),
                world.hydrology.lake_id[cell] == worldgen::NONE
                    && world.hydrology.discharge_m3s[cell] >= world.settings.river_min_m3s
            );
            assert_eq!(
                surface.ice(cell as u32),
                world.hydrology.ice_cover[cell] != 0
            );
            assert_eq!(
                surface.temperature_centi_c[cell],
                products::centi_celsius(world.climate.t_annual_c[cell])
            );
            assert_eq!(
                surface.precipitation_mm[cell],
                products::millimetres(world.climate.p_annual_mm[cell])
            );
            assert_eq!(
                surface.pet_mm[cell],
                products::millimetres(world.climate.pet_annual_mm[cell])
            );
            if world.ocean[cell] == 0 {
                let id = surface.landmass_id(cell as u32);
                assert_ne!(id, TOOL_SURFACE_NONE);
                assert!(id as usize <= cell);
            }
        }
        assert!(surface.heap_bytes() > surface.cell_count() * 30);
    }

    #[test]
    fn canonical_tools_match_a_generated_world() {
        let generated = generate(settings(64), &mut NoopProgress).unwrap();
        let decoded = decode_source(generated.source()).unwrap();
        let world = worldgen::derive_world(
            &decoded.settings,
            &decoded.tectonics,
            &decoded.elevation_mm,
            &mut worldgen::NoProgress,
        )
        .unwrap();
        let surface = build_tool_surface(&world, 0);
        let query = crate::find_place::FindPlaceQuery {
            temperature_centi_c_min: Some(-10_000),
            ..crate::find_place::FindPlaceQuery::default()
        };
        let found = crate::find_place::find_place_canonical(&surface, &query).unwrap();
        assert!(found.candidate_count >= 1);
        for candidate in &found.candidates {
            let mut altitude = 0i64;
            let mut temperature = 0i64;
            let mut precipitation = 0u64;
            let mut slope = 0u64;
            let mut islands = std::collections::BTreeMap::<u32, u32>::new();
            for cell in &candidate.cells {
                altitude += i64::from(
                    (surface.elevation_mm[*cell as usize] - surface.sea_level_mm) / 1_000,
                );
                temperature += i64::from(surface.temperature_centi_c[*cell as usize]);
                precipitation += u64::from(surface.precipitation_mm[*cell as usize]);
                slope += u64::from(surface.slope_ppm[*cell as usize]);
                *islands.entry(surface.landmass_id(*cell)).or_default() += 1;
            }
            let count = candidate.cells.len() as i64;
            assert_eq!(candidate.means.altitude_m, (altitude / count) as i32);
            assert_eq!(
                candidate.means.temperature_centi_c,
                (temperature / count) as i32
            );
            assert_eq!(
                candidate.means.precipitation_mm,
                (precipitation / count as u64) as u32
            );
            assert_eq!(candidate.means.slope_ppm, (slope / count as u64) as u32);
            assert_eq!(
                candidate.means.island_id,
                islands
                    .into_iter()
                    .max_by_key(|(_, count)| *count)
                    .map(|(id, _)| id)
                    .unwrap_or(TOOL_SURFACE_NONE)
            );
            assert!(candidate.cells.iter().all(|cell| !surface.ocean(*cell)));
        }

        let mut landmass_cells = std::collections::BTreeMap::<u32, Vec<u32>>::new();
        for cell in 0..surface.cell_count() as u32 {
            if !surface.ocean(cell) {
                landmass_cells
                    .entry(surface.landmass_id(cell))
                    .or_default()
                    .push(cell);
            }
        }
        let landmass = landmass_cells
            .iter()
            .max_by_key(|(_, cells)| cells.len())
            .map(|(id, cells)| (*id, cells))
            .expect("generated land");
        let latitude_key =
            |cell: &&u32| (surface.cell_lat_lon_degrees(**cell).0 * 1_000.0).round() as i64;
        let start = *landmass.1.iter().min_by_key(latitude_key).unwrap();
        let end = *landmass.1.iter().max_by_key(latitude_key).unwrap();
        let start_point = surface.center_microdegrees(start);
        let end_point = surface.center_microdegrees(end);
        let routes = crate::routing::suggest_routes_canonical(
            &surface,
            &crate::routing::RouteSuggestQuery {
                start_longitude_microdegrees: start_point[0],
                start_latitude_microdegrees: start_point[1],
                end_longitude_microdegrees: end_point[0],
                end_latitude_microdegrees: end_point[1],
                existing_routes: Vec::new(),
            },
        )
        .unwrap();
        assert!(routes.suggestion_count >= 1);
        assert!(routes
            .suggestions
            .iter()
            .all(|suggestion| suggestion.cells.iter().all(|cell| !surface.ocean(*cell))));

        let lake = surface.lakes.iter().find(|lake| lake.has_water);
        if let Some(lake) = lake {
            let cell = (0..surface.cell_count() as u32)
                .find(|cell| surface.lake_id(*cell) == lake.id)
                .expect("a lake has cells");
            let point = surface.center_microdegrees(cell);
            let claim =
                crate::hydro_claim::claim_at_canonical(&surface, point[0], point[1], 50_000)
                    .expect("lake claim");
            assert_eq!(claim.id, format!("lake:c{}", lake.minimum_cell));
            assert_eq!(
                crate::hydro_claim::resolve_canonical(&surface, claim.kind, &claim.id)
                    .expect("lake resolves")
                    .id,
                claim.id
            );
        }

        let click = surface.center_microdegrees(landmass.1[0]);
        let region = crate::region::propose_region_canonical(
            &surface,
            click[0],
            click[1],
            crate::region::DETECTOR_LANDMASS,
            64,
            32,
        )
        .unwrap();
        assert_eq!(region.region_id, landmass.0);
        assert!(region.cell_count >= landmass.1.len() as u32);
        assert!(!region.rings.is_empty());
        assert!(region.rings.iter().all(|ring| ring.first() == ring.last()));
    }

    #[test]
    fn tool_surface_epoch_uses_the_shared_epoch_solve() {
        use crate::history::{solve_epoch, HistoricalForcingParameters};
        let generated = generate(settings(64), &mut NoopProgress).unwrap();
        let decoded = decode_source(generated.source()).unwrap();
        let forcing = HistoricalForcingParameters::default_for_seed(831_429);
        let epoch = forcing.period_years() / 2;
        let solved = solve_epoch(&decoded, forcing, epoch, &NoopProgress).unwrap();
        assert_eq!(solved.normalized_epoch, epoch);
        let surface = decoded
            .derive_tool_surface_epoch(
                solved.sea_level_mm,
                f64::from(solved.temperature_offset_centi_c) / 100.0,
                &mut NoopProgress,
            )
            .unwrap();
        assert_eq!(surface.sea_level_mm, solved.sea_level_mm);
        assert_eq!(surface.cell_count(), decoded.elevation_mm().len());
        let reference = decoded
            .derive_tool_surface_present(&mut NoopProgress)
            .unwrap();
        assert_ne!(
            (surface.ocean_count(), surface.sea_level_mm),
            (reference.ocean_count(), reference.sea_level_mm)
        );
    }

    #[test]
    #[ignore = "wall-clock: release-only E5a surface sizes"]
    fn tool_surface_sizes_at_product_resolutions() {
        for resolution in [512, 1024] {
            let generated = generate(settings(resolution), &mut NoopProgress).unwrap();
            let decoded = decode_source(generated.source()).unwrap();
            let started = Instant::now();
            let surface = decoded
                .derive_tool_surface_present(&mut NoopProgress)
                .unwrap();
            let elapsed = started.elapsed();
            record_measurement(&format!(
                "n={resolution}: surface {} bytes ({:.2} bytes/cell), present build {elapsed:?}",
                surface.heap_bytes(),
                surface.heap_bytes() as f64 / surface.cell_count() as f64
            ));
        }
    }

    #[test]
    #[ignore = "wall-clock: release-only E5a GeoJSON limit measurement at 2048x1024"]
    fn e5a_geojson_limits_at_1024x512_and_2048x1024() {
        let generated = generate(settings(1024), &mut NoopProgress).unwrap();
        let decoded = decode_source(generated.source()).unwrap();
        let world = worldgen::derive_world(
            &decoded.settings,
            &decoded.tectonics,
            &decoded.elevation_mm,
            &mut worldgen::NoProgress,
        )
        .unwrap();
        for (width, height) in [(1024, 512), (2048, 1024)] {
            let started = Instant::now();
            let derived = build_world_products_with_view(
                &world,
                0,
                PlanetaryConfiguration::earth_like(),
                None,
                width,
                height,
                &mut NoopProgress,
            )
            .unwrap();
            let elapsed = started.elapsed();
            let features = derived
                .derived_geojson
                .matches("\"type\":\"Feature\"")
                .count();
            let bytes = derived.derived_geojson.len();
            assert!(features <= crate::MAX_GEOJSON_FEATURES);
            assert!(bytes <= crate::MAX_DERIVED_GEOJSON_BYTES);
            record_measurement(&format!(
                "view {width}x{height}: GeoJSON {features} features, {bytes} bytes, {elapsed:?}; coastline segments {}; lake records {}; watershed polygons {}; land polygons {}; ocean polygons {}; island components {}; shelf cells {}; bathymetry contours {}; rivers {}",
                derived.hydrology.metrics.coastline_segment_count,
                derived.hydrology.metrics.lake_count,
                derived.hydrology.metrics.watershed_count,
                derived.hydrology.metrics.land_polygon_count,
                derived.hydrology.metrics.ocean_polygon_count,
                derived.hydrology.metrics.island_count,
                derived.hydrology.metrics.shelf_cell_count,
                derived.hydrology.metrics.bathymetry_contour_count,
                derived.hydrology.metrics.river_count,
            ));
            assert_eq!(derived.view.grid.width, width);
            assert_eq!(derived.view.grid.height, height);
        }
    }

    #[test]
    #[ignore = "wall-clock: release-only timing record"]
    fn generation_and_reload_time_at_512() {
        let started = Instant::now();
        let generated = generate(settings(512), &mut NoopProgress).unwrap();
        let generation = started.elapsed();
        let started = Instant::now();
        let _ = reload(
            generated.source(),
            PlanetaryConfiguration::earth_like(),
            &mut NoopProgress,
        )
        .unwrap();
        let reload = started.elapsed();
        println!(
            "n=512 generation {:?} reload {:?} source {} bytes",
            generation,
            reload,
            generated.source().len()
        );
    }

    #[test]
    #[ignore = "wall-clock: release-only E4 timing table"]
    fn e4_release_timing_table() {
        use crate::derived_cache::StaticDerivedPhysics;
        use crate::history::{derive_historical_products, HistoricalForcingParameters};
        use std::fs;

        let mut rows = Vec::new();
        for resolution in [512, 1024] {
            let started = Instant::now();
            let generated = generate(settings(resolution), &mut NoopProgress).unwrap();
            let generation = started.elapsed();
            let started = Instant::now();
            let decoded = decode_source(generated.source()).unwrap();
            let decode = started.elapsed();
            let started = Instant::now();
            let derived = decoded
                .derive_products(PlanetaryConfiguration::earth_like(), &mut NoopProgress)
                .unwrap();
            let derive = started.elapsed();
            let cold_reload = decode + derive;
            let max_strahler = derived
                .hydrology
                .rivers
                .iter()
                .map(|river| river.strahler_order)
                .max()
                .unwrap_or(0);
            println!(
                "n={resolution}: decode {decode:?}; derive+views {derive:?}; cold reload {cold_reload:?}; max Strahler {max_strahler}"
            );
            let physics = StaticDerivedPhysics::new(
                derived.view,
                derived.climate,
                derived.hydrology,
                derived.hazards,
                derived.derived_geojson,
                derived.hashes,
            );
            let dir = std::env::temp_dir().join(format!(
                "daena-e4-timing-{}-{}",
                std::process::id(),
                resolution
            ));
            crate::derived_cache::save(&dir, &physics).unwrap();
            let started = Instant::now();
            let cache = crate::derived_cache::load(&dir).unwrap().unwrap();
            let warm_cache = started.elapsed();
            let started = Instant::now();
            let forcing = HistoricalForcingParameters::default_for_seed(831_429);
            let _ = derive_historical_products(
                &decoded,
                &cache,
                forcing,
                forcing.period_years() / 4,
                PlanetaryConfiguration::earth_like(),
                &mut NoopProgress,
            )
            .unwrap();
            let epoch = started.elapsed();
            rows.push((resolution, generation, cold_reload, warm_cache, epoch));
            let _ = fs::remove_dir_all(&dir);
        }
        for (resolution, generation, cold_reload, warm_cache, epoch) in rows {
            println!(
                "n={resolution}: generation {generation:?}; cold reload {cold_reload:?}; warm cache {warm_cache:?}; epoch {epoch:?}"
            );
        }
    }

    #[test]
    #[ignore = "wall-clock: release-only E4 epoch breakdown"]
    fn e4_epoch_breakdown_at_512() {
        use crate::history::HistoricalForcingParameters;
        let generated = generate(settings(512), &mut NoopProgress).unwrap();
        let decoded = decode_source(generated.source()).unwrap();
        let forcing = HistoricalForcingParameters::default_for_seed(831_429);
        let sea_level = 0;
        let mut progress = NoopProgress;
        let started = Instant::now();
        let mut adapter = EngineProgress::new(&mut progress);
        let epoch_world = worldgen::derive_epoch(
            &decoded.settings,
            &decoded.tectonics,
            &decoded.elevation_mm,
            sea_level,
            f64::from(crate::history::temperature_offset_centi_c(
                forcing.period_years() / 4,
                forcing,
            )) / 100.0,
            &mut adapter,
        )
        .unwrap();
        adapter.finish().unwrap();
        let engine_epoch = started.elapsed();
        let started = Instant::now();
        let raster = products::build_view_raster(
            &epoch_world,
            PRODUCT_VIEW_WIDTH,
            PRODUCT_VIEW_HEIGHT,
            &|| false,
        )
        .unwrap();
        let view = started.elapsed();
        let field = PhysicalField {
            grid: raster.grid,
            seed: 831_429,
            retry_index: 0,
            target_land_fraction_ppm: 290_000,
            sea_level_mm: sea_level,
            elevations_mm: raster.elevations_mm.clone(),
        };
        let started = Instant::now();
        let climate =
            products::build_climate(&epoch_world, &raster, PlanetaryConfiguration::earth_like())
                .unwrap();
        let climate_time = started.elapsed();
        let started = Instant::now();
        let hydrology =
            products::build_hydrology(&epoch_world, &raster, &field, &|| false).unwrap();
        let hydrology_time = started.elapsed();
        let started = Instant::now();
        let _ = products::build_hazards(&epoch_world, &raster, &|| false).unwrap();
        let hazards_time = started.elapsed();
        let started = Instant::now();
        let _ = crate::to_geojson_with_progress(&field, &NoopProgress).unwrap();
        let geojson = started.elapsed();
        println!(
            "epoch breakdown: engine {engine_epoch:?}; view {view:?}; climate {climate_time:?}; hydrology {hydrology_time:?}; hazards {hazards_time:?}; geojson {geojson:?}; spinup {}",
            climate.metrics.climate_spinup_years
        );
        let _ = hydrology;
    }
}
