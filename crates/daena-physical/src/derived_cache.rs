//! Compact disk cache for accepted-world physics.
//!
//! Callers supply the directory. This module does not discover `.daena` paths.
//! The cache stores the engine-backed product fields and is keyed by the
//! physical identity together with every version, view size and planetary
//! token that can change its contents.

use std::fs;
use std::path::Path;

use crate::climate::ClimateField as LegacyClimateField;
use crate::engine::{SourceHashes, SOURCE_VERSION, WORLD_ENGINE_VERSION};
use crate::evolution::EvolutionField;
use crate::hydrology::{
    ocean_volume_curve, HydrologyField as LegacyHydrologyField, OceanVolumeSample,
};
use crate::products::{
    Basin, BasinStatus, ClimateField, ClimateMetrics, HazardField, HazardMetrics, HydrologyField,
    HydrologyMetrics, RiverDestination, RiverSegment, VolcanicActivity, VolcanicOrigin,
    VolcanicSource,
};
use crate::{
    GeneratedWorld, Grid, PhysicalError, PhysicalErrorCode, PhysicalField, Segment,
    MAX_DERIVED_GEOJSON_BYTES,
};

pub const CACHE_FORMAT_VERSION: u32 = 3;
pub const CACHE_FILE_NAME: &str = "static.bin";
const MAGIC: &[u8; 8] = b"DAENAPDC";
const MAX_CACHE_BYTES: usize = MAX_DERIVED_GEOJSON_BYTES.saturating_add(192 * 1024 * 1024);

/// Engine-backed derived physics for one accepted world view.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StaticDerivedPhysics {
    pub view: PhysicalField,
    pub climate: ClimateField,
    pub hydrology: HydrologyField,
    pub hazards: HazardField,
    pub geojson: String,
    pub hashes: SourceHashes,
}

impl StaticDerivedPhysics {
    pub fn new(
        view: PhysicalField,
        climate: ClimateField,
        hydrology: HydrologyField,
        hazards: HazardField,
        geojson: String,
        hashes: SourceHashes,
    ) -> Self {
        Self {
            view,
            climate,
            hydrology,
            hazards,
            geojson,
            hashes,
        }
    }

    pub fn matches_hashes(&self, hashes: &SourceHashes) -> bool {
        self.hashes == *hashes
    }

    /// Version directory: engine, source, product derivations, view size and
    /// the planetary display/authoring token.
    pub fn version_dir(
        planetary: crate::planetary::PlanetaryConfiguration,
        view_width: u32,
        view_height: u32,
    ) -> String {
        format!(
            "e{}-s{}-c{}-h{}-z{}-v{}-view{}x{}-{}",
            WORLD_ENGINE_VERSION,
            SOURCE_VERSION,
            crate::products::CLIMATE_DERIVATION_VERSION,
            crate::products::HYDROLOGY_DERIVATION_VERSION,
            crate::hazards::HAZARD_DERIVATION_VERSION,
            crate::hazards::VOLCANIC_SOURCE_DERIVATION_VERSION,
            view_width,
            view_height,
            planetary.climate_cache_token(),
        )
    }
}

/// The old product-generation cache shape. It is kept only for the old
/// history/generator tests that E8 deletes; it is never written to disk.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LegacyDerivedPhysics {
    pub climate: LegacyClimateField,
    pub evolution: EvolutionField,
    pub hydrology: LegacyHydrologyField,
    pub ocean_curve: Vec<OceanVolumeSample>,
    pub geojson: String,
}

impl LegacyDerivedPhysics {
    pub fn from_world(world: &GeneratedWorld) -> Result<Self, PhysicalError> {
        Ok(Self {
            climate: world.climate.clone(),
            evolution: world.evolution.clone(),
            hydrology: world.hydrology.clone(),
            ocean_curve: ocean_volume_curve(&world.field)?,
            geojson: world.derived_geojson.clone(),
        })
    }
}

pub fn save(dir: &Path, physics: &StaticDerivedPhysics) -> Result<(), PhysicalError> {
    refuse_symlink(dir, true)?;
    fs::create_dir_all(dir).map_err(io_error)?;
    refuse_symlink(dir, false)?;
    let payload = encode(physics)?;
    if payload.len() > MAX_CACHE_BYTES {
        return Err(PhysicalError::coded(
            PhysicalErrorCode::LimitExceeded,
            "physical derived cache exceeds the bounded size",
        ));
    }
    let dest = dir.join(CACHE_FILE_NAME);
    refuse_symlink(&dest, true)?;
    let partial = dest.with_extension("bin.part");
    fs::write(&partial, &payload).map_err(io_error)?;
    fs::rename(&partial, &dest).map_err(io_error)?;
    Ok(())
}

pub fn load(dir: &Path) -> Result<Option<StaticDerivedPhysics>, PhysicalError> {
    let dest = dir.join(CACHE_FILE_NAME);
    if !dest.exists() {
        return Ok(None);
    }
    refuse_symlink(dir, false)?;
    refuse_symlink(&dest, false)?;
    let bytes = fs::read(&dest).map_err(io_error)?;
    if bytes.len() > MAX_CACHE_BYTES {
        return Err(PhysicalError::coded(
            PhysicalErrorCode::LimitExceeded,
            "physical derived cache exceeds the bounded size",
        ));
    }
    match decode(&bytes) {
        Ok(physics) => Ok(Some(physics)),
        Err(_) => {
            let _ = fs::remove_file(&dest);
            Ok(None)
        }
    }
}

fn refuse_symlink(path: &Path, missing_ok: bool) -> Result<(), PhysicalError> {
    match fs::symlink_metadata(path) {
        Ok(meta) if meta.file_type().is_symlink() => Err(PhysicalError::coded(
            PhysicalErrorCode::SourceInvalid,
            "physical derived cache refused a symlink",
        )),
        Ok(_) => Ok(()),
        Err(error) if missing_ok && error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(io_error(error)),
    }
}

fn io_error(error: std::io::Error) -> PhysicalError {
    PhysicalError::coded(
        PhysicalErrorCode::SourceInvalid,
        format!("physical derived cache: {error}"),
    )
}

fn cache_error(message: &'static str) -> PhysicalError {
    PhysicalError::coded(PhysicalErrorCode::SourceInvalid, message)
}

fn encode(physics: &StaticDerivedPhysics) -> Result<Vec<u8>, PhysicalError> {
    let mut out = Vec::new();
    out.extend_from_slice(MAGIC);
    write_u32(&mut out, CACHE_FORMAT_VERSION);
    for hash in [
        physics.hashes.elevation_mm,
        physics.hashes.tectonics,
        physics.hashes.climate,
        physics.hashes.hydrology,
        physics.hashes.biome,
        physics.hashes.hazards,
    ] {
        write_u64(&mut out, hash);
    }
    encode_physical_field(&mut out, &physics.view)?;
    encode_climate(&mut out, &physics.climate)?;
    encode_hydrology(&mut out, &physics.hydrology)?;
    encode_hazards(&mut out, &physics.hazards)?;
    write_bytes(&mut out, physics.geojson.as_bytes())?;
    Ok(out)
}

fn decode(bytes: &[u8]) -> Result<StaticDerivedPhysics, PhysicalError> {
    let mut reader = Reader {
        data: bytes,
        pos: 0,
    };
    let mut magic = [0u8; 8];
    reader.read_exact(&mut magic)?;
    if &magic != MAGIC {
        return Err(cache_error("physical derived cache magic is invalid"));
    }
    if reader.u32()? != CACHE_FORMAT_VERSION {
        return Err(cache_error(
            "physical derived cache format version is unsupported",
        ));
    }
    let hashes = SourceHashes {
        elevation_mm: reader.u64()?,
        tectonics: reader.u64()?,
        climate: reader.u64()?,
        hydrology: reader.u64()?,
        biome: reader.u64()?,
        hazards: reader.u64()?,
    };
    let view = decode_physical_field(&mut reader)?;
    let climate = decode_climate(&mut reader)?;
    climate.validate()?;
    let hydrology = decode_hydrology(&mut reader)?;
    hydrology.validate(&view)?;
    let hazards = decode_hazards(&mut reader)?;
    hazards.validate(view.grid)?;
    let geojson = String::from_utf8(reader.bytes()?)
        .map_err(|_| cache_error("physical derived cache geojson is not UTF-8"))?;
    if reader.pos != reader.data.len() {
        return Err(cache_error("physical derived cache has trailing bytes"));
    }
    Ok(StaticDerivedPhysics {
        view,
        climate,
        hydrology,
        hazards,
        geojson,
        hashes,
    })
}

struct Reader<'a> {
    data: &'a [u8],
    pos: usize,
}

impl Reader<'_> {
    fn remaining(&self) -> usize {
        self.data.len().saturating_sub(self.pos)
    }

    fn check_remaining(&self, count: usize, element_size: usize) -> Result<(), PhysicalError> {
        let needed = count
            .checked_mul(element_size)
            .ok_or_else(|| cache_error("physical derived cache length overflow"))?;
        if self.remaining() < needed {
            return Err(cache_error("physical derived cache truncated"));
        }
        Ok(())
    }

    fn read_exact(&mut self, dest: &mut [u8]) -> Result<(), PhysicalError> {
        if self.remaining() < dest.len() {
            return Err(cache_error("physical derived cache truncated"));
        }
        dest.copy_from_slice(&self.data[self.pos..self.pos + dest.len()]);
        self.pos += dest.len();
        Ok(())
    }

    fn u8(&mut self) -> Result<u8, PhysicalError> {
        let mut buf = [0u8; 1];
        self.read_exact(&mut buf)?;
        Ok(buf[0])
    }

    fn u16(&mut self) -> Result<u16, PhysicalError> {
        let mut buf = [0u8; 2];
        self.read_exact(&mut buf)?;
        Ok(u16::from_le_bytes(buf))
    }

    fn u32(&mut self) -> Result<u32, PhysicalError> {
        let mut buf = [0u8; 4];
        self.read_exact(&mut buf)?;
        Ok(u32::from_le_bytes(buf))
    }

    fn u64(&mut self) -> Result<u64, PhysicalError> {
        let mut buf = [0u8; 8];
        self.read_exact(&mut buf)?;
        Ok(u64::from_le_bytes(buf))
    }

    fn i32(&mut self) -> Result<i32, PhysicalError> {
        Ok(self.u32()? as i32)
    }

    fn bool(&mut self) -> Result<bool, PhysicalError> {
        match self.u8()? {
            0 => Ok(false),
            1 => Ok(true),
            _ => Err(cache_error("physical derived cache bool is invalid")),
        }
    }

    fn bytes(&mut self) -> Result<Vec<u8>, PhysicalError> {
        let len = self.u32()? as usize;
        if self.remaining() < len {
            return Err(cache_error("physical derived cache truncated"));
        }
        let bytes = self.data[self.pos..self.pos + len].to_vec();
        self.pos += len;
        Ok(bytes)
    }

    fn vec_i32(&mut self) -> Result<Vec<i32>, PhysicalError> {
        let len = self.u32()? as usize;
        self.check_remaining(len, 4)?;
        let mut values = Vec::with_capacity(len);
        for _ in 0..len {
            values.push(self.i32()?);
        }
        Ok(values)
    }

    fn vec_u32(&mut self) -> Result<Vec<u32>, PhysicalError> {
        let len = self.u32()? as usize;
        self.check_remaining(len, 4)?;
        let mut values = Vec::with_capacity(len);
        for _ in 0..len {
            values.push(self.u32()?);
        }
        Ok(values)
    }

    fn vec_u64(&mut self) -> Result<Vec<u64>, PhysicalError> {
        let len = self.u32()? as usize;
        self.check_remaining(len, 8)?;
        let mut values = Vec::with_capacity(len);
        for _ in 0..len {
            values.push(self.u64()?);
        }
        Ok(values)
    }

    fn vec_bool(&mut self) -> Result<Vec<bool>, PhysicalError> {
        let len = self.u32()? as usize;
        self.check_remaining(len, 1)?;
        let mut values = Vec::with_capacity(len);
        for _ in 0..len {
            values.push(self.bool()?);
        }
        Ok(values)
    }

    fn point(&mut self) -> Result<[i32; 2], PhysicalError> {
        Ok([self.i32()?, self.i32()?])
    }

    fn path(&mut self) -> Result<Vec<[i32; 2]>, PhysicalError> {
        let len = self.u32()? as usize;
        self.check_remaining(len, 8)?;
        let mut path = Vec::with_capacity(len);
        for _ in 0..len {
            path.push(self.point()?);
        }
        Ok(path)
    }

    fn paths(&mut self) -> Result<Vec<Vec<[i32; 2]>>, PhysicalError> {
        let len = self.u32()? as usize;
        self.check_remaining(len, 4)?;
        let mut paths = Vec::with_capacity(len);
        for _ in 0..len {
            paths.push(self.path()?);
        }
        Ok(paths)
    }

    fn polygons(&mut self) -> Result<Vec<Vec<Vec<[i32; 2]>>>, PhysicalError> {
        let len = self.u32()? as usize;
        self.check_remaining(len, 4)?;
        let mut polygons = Vec::with_capacity(len);
        for _ in 0..len {
            polygons.push(self.paths()?);
        }
        Ok(polygons)
    }

    fn grid(&mut self) -> Result<Grid, PhysicalError> {
        Grid::new(self.u32()?, self.u32()?, self.u64()?)
            .map_err(|error| PhysicalError::coded(PhysicalErrorCode::GeometryInvalid, error))
    }

    fn segments(&mut self) -> Result<Vec<Segment>, PhysicalError> {
        let len = self.u32()? as usize;
        self.check_remaining(len, 16)?;
        let mut segments = Vec::with_capacity(len);
        for _ in 0..len {
            segments.push(Segment {
                first: self.point()?,
                second: self.point()?,
            });
        }
        Ok(segments)
    }
}

fn write_u8(out: &mut Vec<u8>, value: u8) {
    out.push(value);
}

fn write_u16(out: &mut Vec<u8>, value: u16) {
    out.extend_from_slice(&value.to_le_bytes());
}

fn write_u32(out: &mut Vec<u8>, value: u32) {
    out.extend_from_slice(&value.to_le_bytes());
}

fn write_u64(out: &mut Vec<u8>, value: u64) {
    out.extend_from_slice(&value.to_le_bytes());
}

fn write_i32(out: &mut Vec<u8>, value: i32) {
    write_u32(out, value as u32);
}

fn write_bool(out: &mut Vec<u8>, value: bool) {
    write_u8(out, u8::from(value));
}

fn u32_len(len: usize) -> Result<u32, PhysicalError> {
    u32::try_from(len).map_err(|_| cache_error("physical derived cache length overflow"))
}

fn write_bytes(out: &mut Vec<u8>, bytes: &[u8]) -> Result<(), PhysicalError> {
    write_u32(out, u32_len(bytes.len())?);
    out.extend_from_slice(bytes);
    Ok(())
}

fn write_vec_i32(out: &mut Vec<u8>, values: &[i32]) -> Result<(), PhysicalError> {
    write_u32(out, u32_len(values.len())?);
    for value in values {
        write_i32(out, *value);
    }
    Ok(())
}

fn write_vec_u32(out: &mut Vec<u8>, values: &[u32]) -> Result<(), PhysicalError> {
    write_u32(out, u32_len(values.len())?);
    for value in values {
        write_u32(out, *value);
    }
    Ok(())
}

fn write_vec_u64(out: &mut Vec<u8>, values: &[u64]) -> Result<(), PhysicalError> {
    write_u32(out, u32_len(values.len())?);
    for value in values {
        write_u64(out, *value);
    }
    Ok(())
}

fn write_vec_bool(out: &mut Vec<u8>, values: &[bool]) -> Result<(), PhysicalError> {
    write_u32(out, u32_len(values.len())?);
    for value in values {
        write_bool(out, *value);
    }
    Ok(())
}

fn write_point(out: &mut Vec<u8>, point: [i32; 2]) {
    write_i32(out, point[0]);
    write_i32(out, point[1]);
}

fn write_path(out: &mut Vec<u8>, path: &[[i32; 2]]) -> Result<(), PhysicalError> {
    write_u32(out, u32_len(path.len())?);
    for point in path {
        write_point(out, *point);
    }
    Ok(())
}

fn write_paths(out: &mut Vec<u8>, paths: &[Vec<[i32; 2]>]) -> Result<(), PhysicalError> {
    write_u32(out, u32_len(paths.len())?);
    for path in paths {
        write_path(out, path)?;
    }
    Ok(())
}

fn write_polygons(out: &mut Vec<u8>, polygons: &[Vec<Vec<[i32; 2]>>]) -> Result<(), PhysicalError> {
    write_u32(out, u32_len(polygons.len())?);
    for polygon in polygons {
        write_paths(out, polygon)?;
    }
    Ok(())
}

fn write_segments(out: &mut Vec<u8>, segments: &[Segment]) -> Result<(), PhysicalError> {
    write_u32(out, u32_len(segments.len())?);
    for segment in segments {
        write_point(out, segment.first);
        write_point(out, segment.second);
    }
    Ok(())
}

fn encode_grid(out: &mut Vec<u8>, grid: Grid) {
    write_u32(out, grid.width);
    write_u32(out, grid.height);
    write_u64(out, grid.radius_metres);
}

fn encode_physical_field(out: &mut Vec<u8>, field: &PhysicalField) -> Result<(), PhysicalError> {
    encode_grid(out, field.grid);
    write_u32(out, field.seed);
    write_u32(out, field.retry_index);
    write_u32(out, field.target_land_fraction_ppm);
    write_i32(out, field.sea_level_mm);
    write_vec_i32(out, &field.elevations_mm)
}

fn decode_physical_field(reader: &mut Reader<'_>) -> Result<PhysicalField, PhysicalError> {
    let field = PhysicalField {
        grid: reader.grid()?,
        seed: reader.u32()?,
        retry_index: reader.u32()?,
        target_land_fraction_ppm: reader.u32()?,
        sea_level_mm: reader.i32()?,
        elevations_mm: reader.vec_i32()?,
    };
    field
        .validate()
        .map_err(|error| PhysicalError::coded(PhysicalErrorCode::GeometryInvalid, error))?;
    Ok(field)
}

fn encode_planetary(out: &mut Vec<u8>, planetary: crate::planetary::PlanetaryConfiguration) {
    write_u16(out, planetary.version);
    write_u8(out, encode_planetary_preset(planetary.preset));
    write_u32(out, planetary.star_luminosity_ppm);
    write_u32(out, planetary.star_mass_ppm);
    write_u32(out, planetary.semi_major_axis_milli_au);
    write_u32(out, planetary.eccentricity_ppm);
    write_u32(out, planetary.axial_tilt_milli_deg);
    write_u32(out, planetary.rotation_period_seconds);
    write_i32(out, planetary.retained_heat_centi_c);
    write_u32(out, planetary.bond_albedo_ppm);
    write_u32(out, planetary.mean_density_kg_m3);
    write_u64(out, planetary.radius_metres);
}

fn decode_planetary(
    reader: &mut Reader<'_>,
) -> Result<crate::planetary::PlanetaryConfiguration, PhysicalError> {
    Ok(crate::planetary::PlanetaryConfiguration {
        version: reader.u16()?,
        preset: decode_planetary_preset(reader.u8()?)?,
        star_luminosity_ppm: reader.u32()?,
        star_mass_ppm: reader.u32()?,
        semi_major_axis_milli_au: reader.u32()?,
        eccentricity_ppm: reader.u32()?,
        axial_tilt_milli_deg: reader.u32()?,
        rotation_period_seconds: reader.u32()?,
        retained_heat_centi_c: reader.i32()?,
        bond_albedo_ppm: reader.u32()?,
        mean_density_kg_m3: reader.u32()?,
        radius_metres: reader.u64()?,
    })
}

fn encode_planetary_preset(preset: crate::planetary::PlanetaryPreset) -> u8 {
    match preset {
        crate::planetary::PlanetaryPreset::EarthLike => 0,
        crate::planetary::PlanetaryPreset::LowTilt => 1,
        crate::planetary::PlanetaryPreset::HighTilt => 2,
        crate::planetary::PlanetaryPreset::SlowRotating => 3,
        crate::planetary::PlanetaryPreset::CloseOrbit => 4,
        crate::planetary::PlanetaryPreset::Custom => 5,
    }
}

fn decode_planetary_preset(value: u8) -> Result<crate::planetary::PlanetaryPreset, PhysicalError> {
    match value {
        0 => Ok(crate::planetary::PlanetaryPreset::EarthLike),
        1 => Ok(crate::planetary::PlanetaryPreset::LowTilt),
        2 => Ok(crate::planetary::PlanetaryPreset::HighTilt),
        3 => Ok(crate::planetary::PlanetaryPreset::SlowRotating),
        4 => Ok(crate::planetary::PlanetaryPreset::CloseOrbit),
        5 => Ok(crate::planetary::PlanetaryPreset::Custom),
        _ => Err(cache_error(
            "physical derived cache planetary preset is invalid",
        )),
    }
}

fn encode_climate(out: &mut Vec<u8>, climate: &ClimateField) -> Result<(), PhysicalError> {
    encode_grid(out, climate.grid);
    write_u16(out, climate.derivation_version);
    encode_planetary(out, climate.planetary);
    write_vec_i32(out, &climate.temperature_centi_c)?;
    write_vec_i32(out, &climate.temperature_nh_summer_centi_c)?;
    write_vec_i32(out, &climate.temperature_nh_winter_centi_c)?;
    write_vec_u32(out, &climate.precipitation_mm_per_year)?;
    write_vec_u32(out, &climate.runoff_mm_per_year)?;
    write_vec_u64(out, &climate.runoff_volume_m3_per_year)?;
    write_vec_u32(out, &climate.maritime_factor_ppm)?;
    write_vec_i32(out, &climate.wind_east_milli)?;
    write_vec_i32(out, &climate.wind_north_milli)?;
    write_vec_i32(out, &climate.wind_east_nh_summer_milli)?;
    write_vec_i32(out, &climate.wind_north_nh_summer_milli)?;
    write_vec_i32(out, &climate.wind_east_nh_winter_milli)?;
    write_vec_i32(out, &climate.wind_north_nh_winter_milli)?;
    write_vec_i32(out, &climate.wind_divergence_nano_per_s)?;
    write_vec_i32(out, &climate.wind_divergence_nh_summer_nano_per_s)?;
    write_vec_i32(out, &climate.wind_divergence_nh_winter_nano_per_s)?;
    write_vec_u32(out, &climate.wind_band)?;
    write_vec_u32(out, &climate.wind_band_nh_summer)?;
    write_vec_u32(out, &climate.wind_band_nh_winter)?;
    write_vec_u32(out, &climate.aridity_ppm)?;
    write_vec_u32(out, &climate.precipitation_nh_summer_mm)?;
    write_vec_u32(out, &climate.precipitation_nh_winter_mm)?;
    write_vec_u32(out, &climate.biome_class)?;
    write_vec_u32(out, &climate.storm_suitability_ppm)?;
    write_vec_u32(out, &climate.storm_track_ppm)?;
    write_vec_u32(out, &climate.storm_intensity_ppm)?;
    encode_climate_metrics(out, climate.metrics);
    Ok(())
}

fn decode_climate(reader: &mut Reader<'_>) -> Result<ClimateField, PhysicalError> {
    Ok(ClimateField {
        grid: reader.grid()?,
        derivation_version: reader.u16()?,
        planetary: decode_planetary(reader)?,
        temperature_centi_c: reader.vec_i32()?,
        temperature_nh_summer_centi_c: reader.vec_i32()?,
        temperature_nh_winter_centi_c: reader.vec_i32()?,
        precipitation_mm_per_year: reader.vec_u32()?,
        runoff_mm_per_year: reader.vec_u32()?,
        runoff_volume_m3_per_year: reader.vec_u64()?,
        maritime_factor_ppm: reader.vec_u32()?,
        wind_east_milli: reader.vec_i32()?,
        wind_north_milli: reader.vec_i32()?,
        wind_east_nh_summer_milli: reader.vec_i32()?,
        wind_north_nh_summer_milli: reader.vec_i32()?,
        wind_east_nh_winter_milli: reader.vec_i32()?,
        wind_north_nh_winter_milli: reader.vec_i32()?,
        wind_divergence_nano_per_s: reader.vec_i32()?,
        wind_divergence_nh_summer_nano_per_s: reader.vec_i32()?,
        wind_divergence_nh_winter_nano_per_s: reader.vec_i32()?,
        wind_band: reader.vec_u32()?,
        wind_band_nh_summer: reader.vec_u32()?,
        wind_band_nh_winter: reader.vec_u32()?,
        aridity_ppm: reader.vec_u32()?,
        precipitation_nh_summer_mm: reader.vec_u32()?,
        precipitation_nh_winter_mm: reader.vec_u32()?,
        biome_class: reader.vec_u32()?,
        storm_suitability_ppm: reader.vec_u32()?,
        storm_track_ppm: reader.vec_u32()?,
        storm_intensity_ppm: reader.vec_u32()?,
        metrics: decode_climate_metrics(reader)?,
    })
}

fn encode_climate_metrics(out: &mut Vec<u8>, metrics: ClimateMetrics) {
    write_u64(out, metrics.precipitation_volume_m3_per_year);
    write_u64(out, metrics.runoff_volume_m3_per_year);
    write_i32(out, metrics.mean_temperature_centi_c);
    write_i32(out, metrics.minimum_temperature_centi_c);
    write_i32(out, metrics.maximum_temperature_centi_c);
    write_u32(out, metrics.mean_precipitation_mm_per_year);
    write_u32(out, metrics.mean_runoff_mm_per_year);
    write_u32(out, metrics.wettest_cell_precipitation_mm_per_year);
    write_u32(out, metrics.driest_land_cell_precipitation_mm_per_year);
    write_u32(out, metrics.climate_spinup_years);
    write_u32(out, metrics.mean_seasonal_range_centi_c);
    write_i32(out, metrics.minimum_seasonal_temperature_centi_c);
    write_i32(out, metrics.maximum_seasonal_temperature_centi_c);
    write_u32(out, metrics.permanently_frozen_land_ppm);
    write_u32(out, metrics.seasonally_frozen_land_ppm);
    write_u32(out, metrics.mean_wind_speed_milli);
    write_i32(out, metrics.itcz_latitude_milli_deg);
    write_u32(out, metrics.easterly_cell_ppm);
    write_u32(out, metrics.converging_cell_ppm);
    write_u32(out, metrics.mean_land_aridity_ppm);
    write_u32(out, metrics.mean_seasonal_precipitation_range_mm);
    write_u32(out, metrics.dominant_land_biome);
    write_u32(out, metrics.mean_ocean_storm_suitability_ppm);
    write_u32(out, metrics.storm_prone_ocean_ppm);
    write_u32(out, metrics.mean_storm_intensity_ppm);
    write_u32(out, metrics.mean_land_storm_track_ppm);
    write_u32(out, metrics.expected_storms_per_year_milli);
    write_u32(out, metrics.mean_land_drought_potential_ppm);
    write_u32(out, metrics.mean_heat_wave_potential_ppm);
    write_u32(out, metrics.mean_extreme_rainfall_potential_ppm);
    write_u32(out, metrics.mean_land_growing_season_ppm);
    write_u32(out, metrics.mean_land_dry_season_ppm);
    write_u32(out, metrics.mean_land_wet_season_ppm);
}

fn decode_climate_metrics(reader: &mut Reader<'_>) -> Result<ClimateMetrics, PhysicalError> {
    Ok(ClimateMetrics {
        precipitation_volume_m3_per_year: reader.u64()?,
        runoff_volume_m3_per_year: reader.u64()?,
        mean_temperature_centi_c: reader.i32()?,
        minimum_temperature_centi_c: reader.i32()?,
        maximum_temperature_centi_c: reader.i32()?,
        mean_precipitation_mm_per_year: reader.u32()?,
        mean_runoff_mm_per_year: reader.u32()?,
        wettest_cell_precipitation_mm_per_year: reader.u32()?,
        driest_land_cell_precipitation_mm_per_year: reader.u32()?,
        climate_spinup_years: reader.u32()?,
        mean_seasonal_range_centi_c: reader.u32()?,
        minimum_seasonal_temperature_centi_c: reader.i32()?,
        maximum_seasonal_temperature_centi_c: reader.i32()?,
        permanently_frozen_land_ppm: reader.u32()?,
        seasonally_frozen_land_ppm: reader.u32()?,
        mean_wind_speed_milli: reader.u32()?,
        itcz_latitude_milli_deg: reader.i32()?,
        easterly_cell_ppm: reader.u32()?,
        converging_cell_ppm: reader.u32()?,
        mean_land_aridity_ppm: reader.u32()?,
        mean_seasonal_precipitation_range_mm: reader.u32()?,
        dominant_land_biome: reader.u32()?,
        mean_ocean_storm_suitability_ppm: reader.u32()?,
        storm_prone_ocean_ppm: reader.u32()?,
        mean_storm_intensity_ppm: reader.u32()?,
        mean_land_storm_track_ppm: reader.u32()?,
        expected_storms_per_year_milli: reader.u32()?,
        mean_land_drought_potential_ppm: reader.u32()?,
        mean_heat_wave_potential_ppm: reader.u32()?,
        mean_extreme_rainfall_potential_ppm: reader.u32()?,
        mean_land_growing_season_ppm: reader.u32()?,
        mean_land_dry_season_ppm: reader.u32()?,
        mean_land_wet_season_ppm: reader.u32()?,
    })
}

fn encode_status(out: &mut Vec<u8>, status: BasinStatus) {
    write_u8(
        out,
        match status {
            BasinStatus::Dry => 0,
            BasinStatus::Endorheic => 1,
            BasinStatus::Active => 2,
            BasinStatus::Overflowing => 3,
        },
    );
}

fn decode_status(reader: &mut Reader<'_>) -> Result<BasinStatus, PhysicalError> {
    match reader.u8()? {
        0 => Ok(BasinStatus::Dry),
        1 => Ok(BasinStatus::Endorheic),
        2 => Ok(BasinStatus::Active),
        3 => Ok(BasinStatus::Overflowing),
        _ => Err(cache_error(
            "physical derived cache basin status is invalid",
        )),
    }
}

fn encode_destination(out: &mut Vec<u8>, destination: RiverDestination) {
    match destination {
        RiverDestination::Ocean => write_u8(out, 0),
        RiverDestination::Lake(id) => {
            write_u8(out, 1);
            write_u32(out, id);
        }
        RiverDestination::Confluence => write_u8(out, 2),
    }
}

fn decode_destination(reader: &mut Reader<'_>) -> Result<RiverDestination, PhysicalError> {
    match reader.u8()? {
        0 => Ok(RiverDestination::Ocean),
        1 => Ok(RiverDestination::Lake(reader.u32()?)),
        2 => Ok(RiverDestination::Confluence),
        _ => Err(cache_error(
            "physical derived cache river destination is invalid",
        )),
    }
}

fn encode_option_i32(out: &mut Vec<u8>, value: Option<i32>) {
    match value {
        None => write_u8(out, 0),
        Some(inner) => {
            write_u8(out, 1);
            write_i32(out, inner);
        }
    }
}

fn decode_option_i32(reader: &mut Reader<'_>) -> Result<Option<i32>, PhysicalError> {
    match reader.u8()? {
        0 => Ok(None),
        1 => Ok(Some(reader.i32()?)),
        _ => Err(cache_error(
            "physical derived cache optional i32 is invalid",
        )),
    }
}

fn encode_hydrology(out: &mut Vec<u8>, hydrology: &HydrologyField) -> Result<(), PhysicalError> {
    encode_grid(out, hydrology.grid);
    write_u16(out, hydrology.derivation_version);
    write_i32(out, hydrology.sea_level_mm);
    write_vec_i32(out, &hydrology.water_level_mm)?;
    write_vec_i32(out, &hydrology.lake_level_mm)?;
    write_vec_u32(out, &hydrology.slope_ppm)?;
    write_vec_u32(out, &hydrology.hillshade_ppm)?;
    write_vec_i32(out, &hydrology.bathymetry_mm)?;
    write_vec_u32(out, &hydrology.watershed_id)?;
    write_vec_u32(out, &hydrology.basin_by_cell)?;
    write_vec_bool(out, &hydrology.lake_cells)?;
    write_vec_bool(out, &hydrology.ice_cells)?;
    write_vec_bool(out, &hydrology.shelf_cells)?;
    write_vec_u32(out, &hydrology.island_id)?;
    write_u32(out, u32_len(hydrology.basins.len())?);
    for basin in &hydrology.basins {
        write_u32(out, basin.id);
        write_u32(out, basin.minimum_cell);
        write_i32(out, basin.minimum_elevation_mm);
        write_u32(out, basin.cell_count);
        write_u32(out, basin.spill_cell);
        encode_option_i32(out, basin.spill_elevation_mm);
        write_i32(out, basin.water_level_mm);
        write_u64(out, basin.water_volume_m3);
        write_u64(out, basin.inflow_m3_per_year);
        write_u64(out, basin.evaporation_m3_per_year);
        write_u64(out, basin.outflow_m3_per_year);
        encode_status(out, basin.status);
    }
    write_u32(out, u32_len(hydrology.rivers.len())?);
    for river in &hydrology.rivers {
        write_u32(out, river.id);
        write_u32(out, river.source_cell);
        write_u32(out, river.mouth_cell);
        write_u16(out, river.strahler_order);
        encode_destination(out, river.destination);
        write_bool(out, river.spill_outlet);
        write_u32(out, river.coordinate_count);
    }
    write_paths(out, &hydrology.river_coordinates)?;
    write_segments(out, &hydrology.coastline_segments)?;
    write_polygons(out, &hydrology.lake_polygons)?;
    write_polygons(out, &hydrology.watershed_polygons)?;
    write_polygons(out, &hydrology.land_polygons)?;
    write_polygons(out, &hydrology.ocean_polygons)?;
    write_polygons(out, &hydrology.shelf_polygons)?;
    write_polygons(out, &hydrology.island_polygons)?;
    write_polygons(out, &hydrology.ice_polygons)?;
    write_segments(out, &hydrology.bathymetry_contours)?;
    let metrics = hydrology.metrics;
    write_u32(out, metrics.lake_count);
    write_u32(out, metrics.river_count);
    write_u32(out, metrics.watershed_count);
    write_u32(out, metrics.coastline_segment_count);
    write_u32(out, metrics.land_polygon_count);
    write_u32(out, metrics.ocean_polygon_count);
    write_u32(out, metrics.shelf_cell_count);
    write_u32(out, metrics.bathymetry_contour_count);
    write_u32(out, metrics.island_count);
    Ok(())
}

fn decode_hydrology(reader: &mut Reader<'_>) -> Result<HydrologyField, PhysicalError> {
    let grid = reader.grid()?;
    let derivation_version = reader.u16()?;
    let sea_level_mm = reader.i32()?;
    let water_level_mm = reader.vec_i32()?;
    let lake_level_mm = reader.vec_i32()?;
    let slope_ppm = reader.vec_u32()?;
    let hillshade_ppm = reader.vec_u32()?;
    let bathymetry_mm = reader.vec_i32()?;
    let watershed_id = reader.vec_u32()?;
    let basin_by_cell = reader.vec_u32()?;
    let lake_cells = reader.vec_bool()?;
    let ice_cells = reader.vec_bool()?;
    let shelf_cells = reader.vec_bool()?;
    let island_id = reader.vec_u32()?;
    let basin_len = reader.u32()? as usize;
    let mut basins = Vec::with_capacity(basin_len);
    for _ in 0..basin_len {
        basins.push(Basin {
            id: reader.u32()?,
            minimum_cell: reader.u32()?,
            minimum_elevation_mm: reader.i32()?,
            cell_count: reader.u32()?,
            spill_cell: reader.u32()?,
            spill_elevation_mm: decode_option_i32(reader)?,
            water_level_mm: reader.i32()?,
            water_volume_m3: reader.u64()?,
            inflow_m3_per_year: reader.u64()?,
            evaporation_m3_per_year: reader.u64()?,
            outflow_m3_per_year: reader.u64()?,
            status: decode_status(reader)?,
        });
    }
    let river_len = reader.u32()? as usize;
    let mut rivers = Vec::with_capacity(river_len);
    for _ in 0..river_len {
        rivers.push(RiverSegment {
            id: reader.u32()?,
            source_cell: reader.u32()?,
            mouth_cell: reader.u32()?,
            strahler_order: reader.u16()?,
            destination: decode_destination(reader)?,
            spill_outlet: reader.bool()?,
            coordinate_count: reader.u32()?,
        });
    }
    let river_coordinates = reader.paths()?;
    let coastline_segments = reader.segments()?;
    let lake_polygons = reader.polygons()?;
    let watershed_polygons = reader.polygons()?;
    let land_polygons = reader.polygons()?;
    let ocean_polygons = reader.polygons()?;
    let shelf_polygons = reader.polygons()?;
    let island_polygons = reader.polygons()?;
    let ice_polygons = reader.polygons()?;
    let bathymetry_contours = reader.segments()?;
    Ok(HydrologyField {
        grid,
        derivation_version,
        sea_level_mm,
        water_level_mm,
        lake_level_mm,
        slope_ppm,
        hillshade_ppm,
        bathymetry_mm,
        watershed_id,
        basin_by_cell,
        lake_cells,
        ice_cells,
        shelf_cells,
        island_id,
        basins,
        rivers,
        river_coordinates,
        coastline_segments,
        lake_polygons,
        watershed_polygons,
        land_polygons,
        ocean_polygons,
        shelf_polygons,
        island_polygons,
        ice_polygons,
        bathymetry_contours,
        metrics: HydrologyMetrics {
            lake_count: reader.u32()?,
            river_count: reader.u32()?,
            watershed_count: reader.u32()?,
            coastline_segment_count: reader.u32()?,
            land_polygon_count: reader.u32()?,
            ocean_polygon_count: reader.u32()?,
            shelf_cell_count: reader.u32()?,
            bathymetry_contour_count: reader.u32()?,
            island_count: reader.u32()?,
        },
    })
}

fn encode_hazards(out: &mut Vec<u8>, hazards: &HazardField) -> Result<(), PhysicalError> {
    write_vec_u32(out, &hazards.earthquake_hazard_ppm)?;
    write_vec_u32(out, &hazards.volcanic_hazard_ppm)?;
    write_vec_u64(out, &hazards.earthquake_annual_rate_nano)?;
    write_vec_u64(out, &hazards.volcanic_annual_rate_nano)?;
    write_vec_u32(out, &hazards.earthquake_rate_per_million_years_ppm)?;
    write_vec_u32(out, &hazards.eruption_rate_per_million_years_ppm)?;
    write_u32(out, u32_len(hazards.volcanic_sources.len())?);
    for source in &hazards.volcanic_sources {
        write_u64(out, source.stable_id);
        write_u32(out, source.cell);
        write_u8(
            out,
            match source.origin {
                VolcanicOrigin::Hotspot => 0,
                VolcanicOrigin::SubductionArc => 1,
                VolcanicOrigin::Rift => 2,
            },
        );
        write_u8(
            out,
            match source.activity_class {
                VolcanicActivity::Dormant => 0,
                VolcanicActivity::Moderate => 1,
                VolcanicActivity::Vigorous => 2,
            },
        );
        write_u64(out, source.eruptions_per_model_year_nano);
    }
    let metrics = hazards.metrics;
    write_u16(out, metrics.derivation_version);
    write_u16(out, metrics.volcanic_source_derivation_version);
    write_u32(out, metrics.volcanic_source_count);
    write_u32(out, metrics.non_dormant_source_count);
    write_u32(out, metrics.earthquake_mean_ppm);
    write_u32(out, metrics.earthquake_max_ppm);
    write_u32(out, metrics.volcanic_mean_ppm);
    write_u32(out, metrics.volcanic_max_ppm);
    write_u64(out, metrics.earthquake_rate_mass_nano);
    write_u64(out, metrics.volcanic_rate_mass_nano);
    Ok(())
}

fn decode_hazards(reader: &mut Reader<'_>) -> Result<HazardField, PhysicalError> {
    let earthquake_hazard_ppm = reader.vec_u32()?;
    let volcanic_hazard_ppm = reader.vec_u32()?;
    let earthquake_annual_rate_nano = reader.vec_u64()?;
    let volcanic_annual_rate_nano = reader.vec_u64()?;
    let earthquake_rate_per_million_years_ppm = reader.vec_u32()?;
    let eruption_rate_per_million_years_ppm = reader.vec_u32()?;
    let source_count = reader.u32()? as usize;
    let mut volcanic_sources = Vec::with_capacity(source_count);
    for _ in 0..source_count {
        let stable_id = reader.u64()?;
        let cell = reader.u32()?;
        let origin = match reader.u8()? {
            0 => VolcanicOrigin::Hotspot,
            1 => VolcanicOrigin::SubductionArc,
            2 => VolcanicOrigin::Rift,
            _ => {
                return Err(cache_error(
                    "physical derived cache volcano origin is invalid",
                ))
            }
        };
        let activity_class = match reader.u8()? {
            0 => VolcanicActivity::Dormant,
            1 => VolcanicActivity::Moderate,
            2 => VolcanicActivity::Vigorous,
            _ => {
                return Err(cache_error(
                    "physical derived cache volcano activity is invalid",
                ))
            }
        };
        volcanic_sources.push(VolcanicSource {
            stable_id,
            cell,
            origin,
            activity_class,
            eruptions_per_model_year_nano: reader.u64()?,
        });
    }
    Ok(HazardField {
        earthquake_hazard_ppm,
        volcanic_hazard_ppm,
        earthquake_annual_rate_nano,
        volcanic_annual_rate_nano,
        earthquake_rate_per_million_years_ppm,
        eruption_rate_per_million_years_ppm,
        volcanic_sources,
        metrics: HazardMetrics {
            derivation_version: reader.u16()?,
            volcanic_source_derivation_version: reader.u16()?,
            volcanic_source_count: reader.u32()?,
            non_dormant_source_count: reader.u32()?,
            earthquake_mean_ppm: reader.u32()?,
            earthquake_max_ppm: reader.u32()?,
            volcanic_mean_ppm: reader.u32()?,
            volcanic_max_ppm: reader.u32()?,
            earthquake_rate_mass_nano: reader.u64()?,
            volcanic_rate_mass_nano: reader.u64()?,
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{engine, planetary::PlanetaryConfiguration, NoopProgress};

    fn products(resolution: u32) -> engine::Products {
        let settings = engine::Settings::from_parts(
            831_429,
            engine::WorldPreset::Earthlike,
            resolution,
            6_371_000,
            290_000,
            PlanetaryConfiguration::earth_like(),
        );
        engine::generate(settings, &mut NoopProgress).unwrap()
    }

    #[test]
    fn static_derived_round_trips_through_the_cache_file() {
        let products = products(32);
        let physics = StaticDerivedPhysics::new(
            products.view().clone(),
            products.climate().clone(),
            products.hydrology().clone(),
            products.hazards().clone(),
            products.derived_geojson().to_owned(),
            products.hashes(),
        );
        let encoded = encode(&physics).unwrap();
        let decoded = decode(&encoded).unwrap();
        assert_eq!(decoded, physics);
    }

    #[test]
    fn deleting_the_cache_rebuilds_identical_products() {
        let products = products(32);
        let physics = StaticDerivedPhysics::new(
            products.view().clone(),
            products.climate().clone(),
            products.hydrology().clone(),
            products.hazards().clone(),
            products.derived_geojson().to_owned(),
            products.hashes(),
        );
        let dir = std::env::temp_dir().join(format!(
            "daena-e4-cache-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        save(&dir, &physics).unwrap();
        assert_eq!(load(&dir).unwrap().unwrap(), physics);
        let first = std::fs::read(dir.join(CACHE_FILE_NAME)).unwrap();
        std::fs::remove_dir_all(&dir).unwrap();

        let decoded = engine::decode_source(products.source()).unwrap();
        let mut progress = NoopProgress;
        let rebuilt = decoded
            .derive_products(PlanetaryConfiguration::earth_like(), &mut progress)
            .unwrap();
        let rebuilt = StaticDerivedPhysics::new(
            rebuilt.view,
            rebuilt.climate,
            rebuilt.hydrology,
            rebuilt.hazards,
            rebuilt.derived_geojson,
            rebuilt.hashes,
        );
        assert_eq!(rebuilt, physics);
        save(&dir, &rebuilt).unwrap();
        assert_eq!(std::fs::read(dir.join(CACHE_FILE_NAME)).unwrap(), first);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn version_dir_names_engine_source_view_and_planet() {
        let earth = PlanetaryConfiguration::earth_like();
        let first = StaticDerivedPhysics::version_dir(earth, 1024, 512);
        assert!(first.contains(&format!("e{}", WORLD_ENGINE_VERSION)));
        assert!(first.contains(&format!("s{}", SOURCE_VERSION)));
        assert!(first.contains("view1024x512"));
        assert!(first.contains(&earth.climate_cache_token()));
        assert_ne!(first, StaticDerivedPhysics::version_dir(earth, 2048, 1024));
        let mut close = earth;
        close.retained_heat_centi_c += 1;
        assert_ne!(first, StaticDerivedPhysics::version_dir(close, 1024, 512));
    }

    #[test]
    fn decode_rejects_a_stale_format_version() {
        let products = products(32);
        let physics = StaticDerivedPhysics::new(
            products.view().clone(),
            products.climate().clone(),
            products.hydrology().clone(),
            products.hazards().clone(),
            products.derived_geojson().to_owned(),
            products.hashes(),
        );
        let mut encoded = encode(&physics).unwrap();
        encoded[8..12].copy_from_slice(&(CACHE_FORMAT_VERSION + 1).to_le_bytes());
        assert!(decode(&encoded).is_err());
    }
}
