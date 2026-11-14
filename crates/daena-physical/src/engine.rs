//! Product-facing facade over `daena-worldgen`.
//!
//! This module is the only place in the product that names engine types. It
//! maps product settings and progress, owns the `DAENAPW2` source codec, and
//! builds the transitional product views.

use std::fmt;

use daena_worldgen as worldgen;
use serde_json::json;

use crate::{
    climate::{self, ClimateField},
    planetary::PlanetaryConfiguration,
    PhysicalError, PhysicalErrorCode, PhysicalField, ProgressPhase, ProgressSink, ValidationReport,
    GENERATOR_ID, GENERATOR_VERSION,
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
    fn from_engine(hashes: worldgen::GenerationHashes) -> Self {
        Self {
            elevation_mm: hashes.elevation_mm,
            tectonics: hashes.tectonics,
            climate: hashes.climate,
            hydrology: hashes.hydrology,
            biome: hashes.biome,
            hazards: hashes.hazards,
        }
    }

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
}

/// Products built from one engine world.
pub struct Products {
    source: Vec<u8>,
    view: PhysicalField,
    climate: ClimateField,
    derived_geojson: String,
    generation: Option<serde_json::Value>,
    hashes: SourceHashes,
    reference_water_inventory_m3: u64,
    resolution: u32,
    seed: u64,
}

impl Products {
    pub fn source(&self) -> &[u8] {
        &self.source
    }

    pub fn view(&self) -> &PhysicalField {
        &self.view
    }

    pub fn climate(&self) -> &ClimateField {
        &self.climate
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
    let hashes = SourceHashes::from_engine(world.generation_hashes());
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
    build_products(
        source,
        generation,
        &world,
        settings.planetary,
        hashes,
        reference_water_inventory_m3,
        progress,
    )
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
    let computed = SourceHashes::from_engine(world.generation_hashes());
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
    build_products(
        source,
        None,
        &world,
        planetary,
        decoded.hashes,
        decoded.reference_water_inventory_m3,
        progress,
    )
}

fn build_products(
    source: Vec<u8>,
    generation: Option<serde_json::Value>,
    world: &worldgen::World,
    planetary: PlanetaryConfiguration,
    hashes: SourceHashes,
    reference_water_inventory_m3: u64,
    progress: &mut dyn ProgressSink,
) -> Result<Products, PhysicalError> {
    if source.len() > crate::MAX_SOURCE_BYTES {
        return Err(PhysicalError::coded(
            PhysicalErrorCode::LimitExceeded,
            format!("source exceeds the {} byte budget", crate::MAX_SOURCE_BYTES),
        ));
    }
    let elevation_m = world
        .equirect_elevation_with_cancel(PRODUCT_VIEW_WIDTH, PRODUCT_VIEW_HEIGHT, &|| {
            progress.check_cancelled().is_err()
        })
        .map_err(|error| map_world_error(error, false))?;
    let elevations_mm = elevation_m
        .iter()
        .map(|value| worldgen::elevation_mm_from_m(*value))
        .collect::<Vec<_>>();
    let field = PhysicalField {
        grid: crate::Grid::new(
            PRODUCT_VIEW_WIDTH,
            PRODUCT_VIEW_HEIGHT,
            world.settings.radius_m.round() as u64,
        )
        .map_err(PhysicalError::InvalidSettings)?,
        seed: (world.settings.seed & u64::from(u32::MAX)) as u32,
        retry_index: 0,
        target_land_fraction_ppm: (world.settings.land_fraction * 1_000_000.0).round() as u32,
        sea_level_mm: 0,
        elevations_mm,
    };
    field.validate().map_err(PhysicalError::InvalidSettings)?;
    let climate = transitional_climate(&field, planetary, progress)?;
    let derived_geojson = crate::to_geojson_with_progress(&field, progress)?;
    Ok(Products {
        source,
        view: field,
        climate,
        derived_geojson,
        generation,
        hashes,
        reference_water_inventory_m3,
        resolution: world.settings.resolution,
        seed: world.settings.seed,
    })
}

fn transitional_climate(
    field: &PhysicalField,
    planetary: PlanetaryConfiguration,
    progress: &mut dyn ProgressSink,
) -> Result<ClimateField, PhysicalError> {
    let mut settings = climate::ClimateSettings::default_for(field.grid);
    settings.planetary = planetary;
    climate::derive_current_climate(field, settings, field.seed, field.retry_index, progress)
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
    let forcing = crate::history::HistoricalForcingParameters::default_for(
        (settings.seed & u64::from(u32::MAX)) as u32,
        0,
    );
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
}
