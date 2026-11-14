//! Product-facing facade over `daena-worldgen`.
//!
//! This module is the only place in the product that names engine types. It
//! maps product settings and progress, owns the `DAENAPW2` source codec, and
//! builds the transitional product views.

use std::fmt;

use daena_worldgen as worldgen;
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
#[derive(Clone, PartialEq, Eq)]
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
    if progress.check_cancelled().is_err() {
        return Err(PhysicalError::Cancelled);
    }
    let cancelled = || progress.check_cancelled().is_err();
    let raster =
        products::build_view_raster(world, PRODUCT_VIEW_WIDTH, PRODUCT_VIEW_HEIGHT, &cancelled)?;
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
    let derived_geojson = crate::to_geojson_with_progress(&view, progress)?;
    let hashes = match hashes {
        Some(hashes) => hashes,
        None => source_hashes_with_cancel(world, &world.source_elevation_mm(), progress)?,
    };
    Ok(DerivedProducts {
        view,
        climate,
        hydrology,
        hazards,
        derived_geojson,
        hashes,
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
            "hazardDerivationVersion": crate::hazards::HAZARD_DERIVATION_VERSION,
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

fn hash_f32_value(hash: u64, value: f32) -> u64 {
    hash_u32(hash, value.to_bits())
}

fn hash_f64_value(hash: u64, value: f64) -> u64 {
    hash_u64(hash, value.to_bits())
}

fn hash_elevation_mm(
    mut hash: u64,
    values: &[i32],
    progress: &dyn ProgressSink,
) -> Result<u64, PhysicalError> {
    for chunk in values.chunks(HASH_CHUNK) {
        progress.check_cancelled()?;
        for value in chunk {
            hash = hash_u32(hash, *value as u32);
        }
    }
    Ok(hash)
}

fn hash_u8_values(
    mut hash: u64,
    values: &[u8],
    progress: &dyn ProgressSink,
) -> Result<u64, PhysicalError> {
    for chunk in values.chunks(HASH_CHUNK) {
        progress.check_cancelled()?;
        for value in chunk {
            hash = hash_byte(hash, *value);
        }
    }
    Ok(hash)
}

fn hash_u32_values(
    mut hash: u64,
    values: &[u32],
    progress: &dyn ProgressSink,
) -> Result<u64, PhysicalError> {
    for chunk in values.chunks(HASH_CHUNK) {
        progress.check_cancelled()?;
        for value in chunk {
            hash = hash_u32(hash, *value);
        }
    }
    Ok(hash)
}

fn hash_f32_values(
    mut hash: u64,
    values: &[f32],
    progress: &dyn ProgressSink,
) -> Result<u64, PhysicalError> {
    for chunk in values.chunks(HASH_CHUNK) {
        progress.check_cancelled()?;
        for value in chunk {
            hash = hash_f32_value(hash, *value);
        }
    }
    Ok(hash)
}

fn hash_f64_values(
    mut hash: u64,
    values: &[f64],
    progress: &dyn ProgressSink,
) -> Result<u64, PhysicalError> {
    for chunk in values.chunks(HASH_CHUNK) {
        progress.check_cancelled()?;
        for value in chunk {
            hash = hash_f64_value(hash, *value);
        }
    }
    Ok(hash)
}

fn climate_hash(
    climate: &worldgen::ClimateSummary,
    progress: &dyn ProgressSink,
) -> Result<u64, PhysicalError> {
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
        hash = hash_f32_values(hash, field, progress)?;
    }
    hash_u8_values(hash, &climate.sea_ice_months, progress)
}

fn hydrology_hash(
    hydrology: &worldgen::Hydrology,
    progress: &dyn ProgressSink,
) -> Result<u64, PhysicalError> {
    let mut hash = FNV_OFFSET;
    for lake in &hydrology.lakes {
        progress.check_cancelled()?;
        hash = hash_byte(hash, lake.kind as u8);
        hash = hash_f64_value(hash, lake.level_m);
        hash = hash_f64_value(hash, lake.area_m2);
        hash = hash_f64_value(hash, lake.volume_m3);
        hash = hash_f64_value(hash, lake.evaporated_m3);
        hash = hash_f64_value(hash, lake.outflow_m3);
        hash = hash_f64_value(hash, lake.inflow_m3);
    }
    for river in &hydrology.rivers {
        hash = hash_byte(hash, river.end as u8);
        hash = hash_f64_value(hash, river.max_discharge_m3s);
        for chunk in river.cells.chunks(HASH_CHUNK) {
            progress.check_cancelled()?;
            for cell in chunk {
                hash = hash_u32(hash, *cell);
            }
        }
    }
    hash = hash_u32_values(hash, &hydrology.lake_id, progress)?;
    hash = hash_f64_values(hash, &hydrology.discharge_m3s, progress)?;
    hash = hash_u8_values(hash, &hydrology.ice_cover, progress)?;
    hash_u32_values(hash, &hydrology.receiver, progress)
}

fn hazards_hash(
    hazards: &worldgen::Hazards,
    progress: &dyn ProgressSink,
) -> Result<u64, PhysicalError> {
    let mut hash = FNV_OFFSET;
    hash = hash_f32_values(hash, &hazards.quake_rate_km2, progress)?;
    hash = hash_u8_values(hash, &hazards.quake_hazard, progress)?;
    hash = hash_u8_values(hash, &hazards.volcanic_hazard, progress)?;
    for source in &hazards.volcanic_sources {
        progress.check_cancelled()?;
        hash = hash_u64(hash, source.id);
        for component in source.pos {
            hash = hash_f64_value(hash, component);
        }
        hash = hash_u32(hash, source.cell);
        hash = hash_byte(
            hash,
            match source.origin {
                worldgen::VolcanicOrigin::SubductionArc => 0,
                worldgen::VolcanicOrigin::Hotspot => 1,
                worldgen::VolcanicOrigin::Rift => 2,
            },
        );
        hash = hash_byte(
            hash,
            match source.activity {
                worldgen::VolcanicActivity::Dormant => 0,
                worldgen::VolcanicActivity::Moderate => 1,
                worldgen::VolcanicActivity::Vigorous => 2,
            },
        );
        hash = hash_f32_value(hash, source.eruptions_per_yr);
    }
    hash = hash_f32_values(hash, &hazards.storms.genesis_km2, progress)?;
    hash = hash_f32_values(hash, &hazards.storms.track_km2, progress)?;
    hash = hash_f32_values(hash, &hazards.storms.intensity, progress)?;
    hash = hash_f32_values(hash, &hazards.storms.genesis_monthly_km2, progress)?;
    hash = hash_u8_values(hash, &hazards.drought, progress)?;
    hash = hash_u8_values(hash, &hazards.heat_wave, progress)?;
    hash = hash_u8_values(hash, &hazards.extreme_rain, progress)?;
    hash_u8_values(hash, &hazards.flood, progress)
}

/// Compute the source hashes with cancellation between chunks.
///
/// This mirrors `World::generation_hashes` exactly; the equality test at
/// `source_hashes_match_the_engine` keeps the two implementations locked.
pub fn source_hashes_with_cancel(
    world: &worldgen::World,
    elevation_mm: &[i32],
    progress: &dyn ProgressSink,
) -> Result<SourceHashes, PhysicalError> {
    progress.check_cancelled()?;
    let elevation_mm_hash = hash_elevation_mm(FNV_OFFSET, elevation_mm, progress)?;
    let tectonics = world.tectonics_hash();
    progress.check_cancelled()?;
    let climate = climate_hash(&world.climate, progress)?;
    let hydrology = hydrology_hash(&world.hydrology, progress)?;
    let biome = hash_u8_values(FNV_OFFSET, &world.biome, progress)?;
    let hazards = hazards_hash(&world.hazards, progress)?;
    Ok(SourceHashes {
        elevation_mm: elevation_mm_hash,
        tectonics,
        climate,
        hydrology,
        biome,
        hazards,
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
    fn source_hashes_match_the_engine() {
        let world_settings = settings(64).world_settings().unwrap();
        let progress = NoopProgress;
        let world = worldgen::generate(&world_settings, &mut worldgen::NoProgress).unwrap();
        let elevation = world.source_elevation_mm();
        let ours = source_hashes_with_cancel(&world, &elevation, &progress).unwrap();
        let engine = world.generation_hashes();
        assert_eq!(ours.elevation_mm, engine.elevation_mm);
        assert_eq!(ours.tectonics, engine.tectonics);
        assert_eq!(ours.climate, engine.climate);
        assert_eq!(ours.hydrology, engine.hydrology);
        assert_eq!(ours.biome, engine.biome);
        assert_eq!(ours.hazards, engine.hazards);
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
