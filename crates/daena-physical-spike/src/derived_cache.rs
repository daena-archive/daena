//! Compact disk cache for accepted-world physics.
//!
//! Callers supply the directory. This module does not discover `.daena` paths.

use std::fs;
use std::path::Path;

use crate::climate::{ClimateField, ClimateMetrics};
use crate::evolution::{
    DrainageEdge, DrainageField, DrainageMetrics, EvolutionField, EvolutionMetrics, EvolutionPreset,
};
use crate::hydrology::{
    ocean_volume_curve, Basin, BasinDestination, BasinStatus, HydrologyField, OceanVolumeSample,
    RiverSegment, WaterBalanceMetrics,
};
use crate::{
    GeneratedWorld, Grid, PhysicalError, PhysicalErrorCode, Segment, MAX_DERIVED_GEOJSON_BYTES,
};

pub const CACHE_FORMAT_VERSION: u32 = 1;
pub const CACHE_FILE_NAME: &str = "static.bin";
const MAGIC: &[u8; 8] = b"DAENAPDC";
const MAX_CACHE_BYTES: usize = MAX_DERIVED_GEOJSON_BYTES.saturating_add(64 * 1024 * 1024);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StaticDerivedPhysics {
    pub climate: ClimateField,
    pub evolution: EvolutionField,
    pub hydrology: HydrologyField,
    pub ocean_curve: Vec<OceanVolumeSample>,
    pub geojson: String,
}

impl StaticDerivedPhysics {
    pub fn from_world(world: &GeneratedWorld) -> Result<Self, PhysicalError> {
        Ok(Self {
            climate: world.climate.clone(),
            evolution: world.evolution.clone(),
            hydrology: world.hydrology.clone(),
            ocean_curve: ocean_volume_curve(&world.field)?,
            geojson: world.derived_geojson.clone(),
        })
    }

    pub fn version_dir() -> String {
        format!(
            "c{}-d{}-h{}-z{}",
            crate::climate::CLIMATE_DERIVATION_VERSION,
            crate::evolution::EVOLUTION_DERIVATION_VERSION,
            crate::hydrology::HYDROLOGY_DERIVATION_VERSION,
            crate::hazards::HAZARD_DERIVATION_VERSION,
        )
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
    encode_climate(&mut out, &physics.climate)?;
    encode_evolution(&mut out, &physics.evolution)?;
    encode_hydrology(&mut out, &physics.hydrology)?;
    write_u32(&mut out, u32_len(physics.ocean_curve.len())?);
    for sample in &physics.ocean_curve {
        write_i32(&mut out, sample.sea_level_mm);
        write_u64(&mut out, sample.ocean_volume_m3);
    }
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
    let climate = decode_climate(&mut reader)?;
    let evolution = decode_evolution(&mut reader)?;
    let hydrology = decode_hydrology(&mut reader)?;
    let curve_len = reader.u32()? as usize;
    let mut ocean_curve = Vec::with_capacity(curve_len);
    for _ in 0..curve_len {
        ocean_curve.push(OceanVolumeSample {
            sea_level_mm: reader.i32()?,
            ocean_volume_m3: reader.u64()?,
        });
    }
    let geojson = String::from_utf8(reader.bytes()?)
        .map_err(|_| cache_error("physical derived cache geojson is not UTF-8"))?;
    if reader.pos != reader.data.len() {
        return Err(cache_error("physical derived cache has trailing bytes"));
    }
    Ok(StaticDerivedPhysics {
        climate,
        evolution,
        hydrology,
        ocean_curve,
        geojson,
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

    fn usize(&mut self) -> Result<usize, PhysicalError> {
        usize::try_from(self.u32()?)
            .map_err(|_| cache_error("physical derived cache index overflow"))
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
        let mut values = Vec::with_capacity(len);
        for _ in 0..len {
            values.push(self.i32()?);
        }
        Ok(values)
    }

    fn vec_u32(&mut self) -> Result<Vec<u32>, PhysicalError> {
        let len = self.u32()? as usize;
        let mut values = Vec::with_capacity(len);
        for _ in 0..len {
            values.push(self.u32()?);
        }
        Ok(values)
    }

    fn vec_u64(&mut self) -> Result<Vec<u64>, PhysicalError> {
        let len = self.u32()? as usize;
        let mut values = Vec::with_capacity(len);
        for _ in 0..len {
            values.push(self.u64()?);
        }
        Ok(values)
    }

    fn vec_bool(&mut self) -> Result<Vec<bool>, PhysicalError> {
        let len = self.u32()? as usize;
        let mut values = Vec::with_capacity(len);
        for _ in 0..len {
            values.push(self.bool()?);
        }
        Ok(values)
    }

    fn vec_usize(&mut self) -> Result<Vec<usize>, PhysicalError> {
        let len = self.u32()? as usize;
        let mut values = Vec::with_capacity(len);
        for _ in 0..len {
            values.push(self.usize()?);
        }
        Ok(values)
    }

    fn point(&mut self) -> Result<[i32; 2], PhysicalError> {
        Ok([self.i32()?, self.i32()?])
    }

    fn path(&mut self) -> Result<Vec<[i32; 2]>, PhysicalError> {
        let len = self.u32()? as usize;
        let mut path = Vec::with_capacity(len);
        for _ in 0..len {
            path.push(self.point()?);
        }
        Ok(path)
    }

    fn paths(&mut self) -> Result<Vec<Vec<[i32; 2]>>, PhysicalError> {
        let len = self.u32()? as usize;
        let mut paths = Vec::with_capacity(len);
        for _ in 0..len {
            paths.push(self.path()?);
        }
        Ok(paths)
    }

    fn polygons(&mut self) -> Result<Vec<Vec<Vec<[i32; 2]>>>, PhysicalError> {
        let len = self.u32()? as usize;
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

fn write_usize(out: &mut Vec<u8>, value: usize) -> Result<(), PhysicalError> {
    write_u32(out, u32_len(value)?);
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

fn write_vec_usize(out: &mut Vec<u8>, values: &[usize]) -> Result<(), PhysicalError> {
    write_u32(out, u32_len(values.len())?);
    for value in values {
        write_usize(out, *value)?;
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

fn encode_grid(out: &mut Vec<u8>, grid: Grid) {
    write_u32(out, grid.width);
    write_u32(out, grid.height);
    write_u64(out, grid.radius_metres);
}

fn encode_climate(out: &mut Vec<u8>, climate: &ClimateField) -> Result<(), PhysicalError> {
    encode_grid(out, climate.grid);
    write_u16(out, climate.derivation_version);
    write_vec_i32(out, &climate.temperature_centi_c)?;
    write_vec_u32(out, &climate.moisture_mm_per_year)?;
    write_vec_u32(out, &climate.precipitation_mm_per_year)?;
    write_vec_u32(out, &climate.runoff_mm_per_year)?;
    write_vec_u64(out, &climate.runoff_volume_m3_per_year)?;
    write_vec_u32(out, &climate.maritime_factor_ppm)?;
    write_u64(out, climate.metrics.precipitation_volume_m3_per_year);
    write_u64(out, climate.metrics.runoff_volume_m3_per_year);
    write_i32(out, climate.metrics.mean_temperature_centi_c);
    write_i32(out, climate.metrics.minimum_temperature_centi_c);
    write_i32(out, climate.metrics.maximum_temperature_centi_c);
    write_u32(out, climate.metrics.mean_precipitation_mm_per_year);
    write_u32(out, climate.metrics.mean_runoff_mm_per_year);
    write_u32(out, climate.metrics.wettest_cell_precipitation_mm_per_year);
    write_u32(
        out,
        climate.metrics.driest_land_cell_precipitation_mm_per_year,
    );
    write_u32(out, climate.metrics.transport_iterations);
    Ok(())
}

fn decode_climate(reader: &mut Reader<'_>) -> Result<ClimateField, PhysicalError> {
    Ok(ClimateField {
        grid: reader.grid()?,
        derivation_version: reader.u16()?,
        temperature_centi_c: reader.vec_i32()?,
        moisture_mm_per_year: reader.vec_u32()?,
        precipitation_mm_per_year: reader.vec_u32()?,
        runoff_mm_per_year: reader.vec_u32()?,
        runoff_volume_m3_per_year: reader.vec_u64()?,
        maritime_factor_ppm: reader.vec_u32()?,
        metrics: ClimateMetrics {
            precipitation_volume_m3_per_year: reader.u64()?,
            runoff_volume_m3_per_year: reader.u64()?,
            mean_temperature_centi_c: reader.i32()?,
            minimum_temperature_centi_c: reader.i32()?,
            maximum_temperature_centi_c: reader.i32()?,
            mean_precipitation_mm_per_year: reader.u32()?,
            mean_runoff_mm_per_year: reader.u32()?,
            wettest_cell_precipitation_mm_per_year: reader.u32()?,
            driest_land_cell_precipitation_mm_per_year: reader.u32()?,
            transport_iterations: reader.u32()?,
        },
    })
}

fn encode_preset(preset: EvolutionPreset) -> u8 {
    match preset {
        EvolutionPreset::Young => 0,
        EvolutionPreset::Mature => 1,
        EvolutionPreset::Old => 2,
    }
}

fn decode_preset(value: u8) -> Result<EvolutionPreset, PhysicalError> {
    match value {
        0 => Ok(EvolutionPreset::Young),
        1 => Ok(EvolutionPreset::Mature),
        2 => Ok(EvolutionPreset::Old),
        _ => Err(cache_error(
            "physical derived cache evolution preset is invalid",
        )),
    }
}

fn encode_drainage(out: &mut Vec<u8>, drainage: &DrainageField) -> Result<(), PhysicalError> {
    encode_grid(out, drainage.grid);
    write_u16(out, drainage.derivation_version);
    write_vec_i32(out, &drainage.routing_elevation_mm)?;
    write_vec_i32(out, &drainage.fill_depth_mm)?;
    write_vec_u32(out, &drainage.routing_order)?;
    write_vec_u32(out, &drainage.slope_ppm)?;
    write_vec_u64(out, &drainage.accumulation_m3_per_year)?;
    write_u32(out, u32_len(drainage.edges.len())?);
    for edge in &drainage.edges {
        write_usize(out, edge.source_cell)?;
        write_usize(out, edge.destination_cell)?;
        write_u32(out, edge.weight_ppm);
        write_u64(out, edge.distance_metres);
    }
    write_vec_usize(out, &drainage.outlet_cells)?;
    write_u64(out, drainage.metrics.direct_runoff_m3_per_year);
    write_u64(out, drainage.metrics.routed_runoff_m3_per_year);
    write_u32(out, drainage.metrics.routed_edge_count);
    write_u32(out, drainage.metrics.drainage_density_ppm);
    write_u32(out, drainage.metrics.grid_anisotropy_ppm);
    write_u32(out, drainage.metrics.convergence_ppm);
    write_u32(out, drainage.metrics.outlet_count);
    write_i32(out, drainage.metrics.routing_surface_raise_max_mm);
    Ok(())
}

fn decode_drainage(reader: &mut Reader<'_>) -> Result<DrainageField, PhysicalError> {
    let grid = reader.grid()?;
    let derivation_version = reader.u16()?;
    let routing_elevation_mm = reader.vec_i32()?;
    let fill_depth_mm = reader.vec_i32()?;
    let routing_order = reader.vec_u32()?;
    let slope_ppm = reader.vec_u32()?;
    let accumulation_m3_per_year = reader.vec_u64()?;
    let edge_len = reader.u32()? as usize;
    let mut edges = Vec::with_capacity(edge_len);
    for _ in 0..edge_len {
        edges.push(DrainageEdge {
            source_cell: reader.usize()?,
            destination_cell: reader.usize()?,
            weight_ppm: reader.u32()?,
            distance_metres: reader.u64()?,
        });
    }
    Ok(DrainageField {
        grid,
        derivation_version,
        routing_elevation_mm,
        fill_depth_mm,
        routing_order,
        slope_ppm,
        accumulation_m3_per_year,
        edges,
        outlet_cells: reader.vec_usize()?,
        metrics: DrainageMetrics {
            direct_runoff_m3_per_year: reader.u64()?,
            routed_runoff_m3_per_year: reader.u64()?,
            routed_edge_count: reader.u32()?,
            drainage_density_ppm: reader.u32()?,
            grid_anisotropy_ppm: reader.u32()?,
            convergence_ppm: reader.u32()?,
            outlet_count: reader.u32()?,
            routing_surface_raise_max_mm: reader.i32()?,
        },
    })
}

fn encode_evolution(out: &mut Vec<u8>, evolution: &EvolutionField) -> Result<(), PhysicalError> {
    encode_grid(out, evolution.grid);
    write_u16(out, evolution.derivation_version);
    write_u8(out, encode_preset(evolution.preset));
    write_vec_i32(out, &evolution.before_elevations_mm)?;
    write_vec_i32(out, &evolution.elevations_mm)?;
    encode_drainage(out, &evolution.drainage)?;
    write_i32(out, evolution.metrics.initial_relief_span_mm);
    write_i32(out, evolution.metrics.final_relief_span_mm);
    write_i32(out, evolution.metrics.relief_change_mm);
    write_u32(out, evolution.metrics.mean_absolute_elevation_change_mm);
    write_u64(out, evolution.metrics.erosion_work_m3);
    write_u64(out, evolution.metrics.uplift_work_m3);
    write_i32(out, evolution.metrics.max_step_relief_loss_mm);
    write_u32(out, evolution.metrics.drainage_density_ppm);
    write_u32(out, evolution.metrics.grid_anisotropy_ppm);
    write_u32(out, evolution.metrics.convergence_ppm);
    write_u32(out, evolution.metrics.tectonic_range_orientation_ppm);
    Ok(())
}

fn decode_evolution(reader: &mut Reader<'_>) -> Result<EvolutionField, PhysicalError> {
    Ok(EvolutionField {
        grid: reader.grid()?,
        derivation_version: reader.u16()?,
        preset: decode_preset(reader.u8()?)?,
        before_elevations_mm: reader.vec_i32()?,
        elevations_mm: reader.vec_i32()?,
        drainage: decode_drainage(reader)?,
        metrics: EvolutionMetrics {
            initial_relief_span_mm: reader.i32()?,
            final_relief_span_mm: reader.i32()?,
            relief_change_mm: reader.i32()?,
            mean_absolute_elevation_change_mm: reader.u32()?,
            erosion_work_m3: reader.u64()?,
            uplift_work_m3: reader.u64()?,
            max_step_relief_loss_mm: reader.i32()?,
            drainage_density_ppm: reader.u32()?,
            grid_anisotropy_ppm: reader.u32()?,
            convergence_ppm: reader.u32()?,
            tectonic_range_orientation_ppm: reader.u32()?,
        },
    })
}

fn encode_destination(
    out: &mut Vec<u8>,
    destination: BasinDestination,
) -> Result<(), PhysicalError> {
    match destination {
        BasinDestination::Ocean => write_u8(out, 0),
        BasinDestination::Basin(id) => {
            write_u8(out, 1);
            write_usize(out, id)?;
        }
        BasinDestination::Endorheic => write_u8(out, 2),
        BasinDestination::Junction(id) => {
            write_u8(out, 3);
            write_usize(out, id)?;
        }
    }
    Ok(())
}

fn decode_destination(reader: &mut Reader<'_>) -> Result<BasinDestination, PhysicalError> {
    match reader.u8()? {
        0 => Ok(BasinDestination::Ocean),
        1 => Ok(BasinDestination::Basin(reader.usize()?)),
        2 => Ok(BasinDestination::Endorheic),
        3 => Ok(BasinDestination::Junction(reader.usize()?)),
        _ => Err(cache_error(
            "physical derived cache basin destination is invalid",
        )),
    }
}

fn encode_status(status: BasinStatus) -> u8 {
    match status {
        BasinStatus::Dry => 0,
        BasinStatus::Endorheic => 1,
        BasinStatus::Active => 2,
        BasinStatus::Overflowing => 3,
        BasinStatus::Merged => 4,
    }
}

fn decode_status(value: u8) -> Result<BasinStatus, PhysicalError> {
    match value {
        0 => Ok(BasinStatus::Dry),
        1 => Ok(BasinStatus::Endorheic),
        2 => Ok(BasinStatus::Active),
        3 => Ok(BasinStatus::Overflowing),
        4 => Ok(BasinStatus::Merged),
        _ => Err(cache_error(
            "physical derived cache basin status is invalid",
        )),
    }
}

fn encode_option_usize(out: &mut Vec<u8>, value: Option<usize>) -> Result<(), PhysicalError> {
    match value {
        None => write_u8(out, 0),
        Some(inner) => {
            write_u8(out, 1);
            write_usize(out, inner)?;
        }
    }
    Ok(())
}

fn decode_option_usize(reader: &mut Reader<'_>) -> Result<Option<usize>, PhysicalError> {
    match reader.u8()? {
        0 => Ok(None),
        1 => Ok(Some(reader.usize()?)),
        _ => Err(cache_error(
            "physical derived cache optional index is invalid",
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
    write_vec_u32(out, &hydrology.ice_thickness_mm)?;
    write_vec_bool(out, &hydrology.shelf_cells)?;
    write_vec_u32(out, &hydrology.island_id)?;
    write_u32(out, u32_len(hydrology.basins.len())?);
    for basin in &hydrology.basins {
        write_usize(out, basin.id)?;
        write_usize(out, basin.minimum_cell)?;
        write_i32(out, basin.minimum_elevation_mm);
        write_u32(out, basin.cell_count);
        encode_option_usize(out, basin.spill_cell)?;
        encode_option_i32(out, basin.spill_elevation_mm);
        write_u64(out, basin.volume_to_spill_m3);
        encode_option_usize(out, basin.parent_basin)?;
        write_vec_usize(out, &basin.children)?;
        encode_destination(out, basin.destination)?;
        write_i32(out, basin.water_level_mm);
        write_u64(out, basin.water_volume_m3);
        write_u64(out, basin.inflow_m3_per_year);
        write_u64(out, basin.direct_precipitation_m3_per_year);
        write_u64(out, basin.evaporation_m3_per_year);
        write_u64(out, basin.outflow_m3_per_year);
        write_u8(out, encode_status(basin.status));
    }
    write_u32(out, u32_len(hydrology.rivers.len())?);
    for river in &hydrology.rivers {
        write_u32(out, river.id);
        write_usize(out, river.source_cell)?;
        write_usize(out, river.mouth_cell)?;
        write_u16(out, river.strahler_order);
        encode_destination(out, river.destination)?;
        write_bool(out, river.spill_outlet);
        write_u32(out, river.coordinate_count);
    }
    write_paths(out, &hydrology.river_coordinates)?;
    write_u32(out, u32_len(hydrology.coastline_segments.len())?);
    for segment in &hydrology.coastline_segments {
        write_point(out, segment.first);
        write_point(out, segment.second);
    }
    write_polygons(out, &hydrology.lake_polygons)?;
    write_polygons(out, &hydrology.watershed_polygons)?;
    write_polygons(out, &hydrology.land_polygons)?;
    write_polygons(out, &hydrology.ocean_polygons)?;
    write_polygons(out, &hydrology.shelf_polygons)?;
    write_polygons(out, &hydrology.island_polygons)?;
    write_polygons(out, &hydrology.ice_polygons)?;
    write_u32(out, u32_len(hydrology.bathymetry_contours.len())?);
    for segment in &hydrology.bathymetry_contours {
        write_point(out, segment.first);
        write_point(out, segment.second);
    }
    write_u64(out, hydrology.metrics.total_water_m3);
    write_u64(out, hydrology.metrics.ocean_water_m3);
    write_u64(out, hydrology.metrics.inland_water_m3);
    write_u64(out, hydrology.metrics.land_ice_m3);
    write_u64(out, hydrology.metrics.balance_error_m3);
    write_u64(out, hydrology.metrics.tolerance_m3);
    write_u32(out, hydrology.metrics.fixed_point_iterations);
    write_bool(out, hydrology.metrics.converged);
    write_u32(out, hydrology.metrics.lake_count);
    write_u32(out, hydrology.metrics.river_count);
    write_u32(out, hydrology.metrics.watershed_count);
    write_u32(out, hydrology.metrics.coastline_segment_count);
    write_u32(out, hydrology.metrics.land_polygon_count);
    write_u32(out, hydrology.metrics.ocean_polygon_count);
    write_u32(out, hydrology.metrics.shelf_cell_count);
    write_u32(out, hydrology.metrics.bathymetry_contour_count);
    write_u32(out, hydrology.metrics.island_count);
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
    let ice_thickness_mm = reader.vec_u32()?;
    let shelf_cells = reader.vec_bool()?;
    let island_id = reader.vec_u32()?;
    let basin_len = reader.u32()? as usize;
    let mut basins = Vec::with_capacity(basin_len);
    for _ in 0..basin_len {
        basins.push(Basin {
            id: reader.usize()?,
            minimum_cell: reader.usize()?,
            minimum_elevation_mm: reader.i32()?,
            cell_count: reader.u32()?,
            spill_cell: decode_option_usize(reader)?,
            spill_elevation_mm: decode_option_i32(reader)?,
            volume_to_spill_m3: reader.u64()?,
            parent_basin: decode_option_usize(reader)?,
            children: reader.vec_usize()?,
            destination: decode_destination(reader)?,
            water_level_mm: reader.i32()?,
            water_volume_m3: reader.u64()?,
            inflow_m3_per_year: reader.u64()?,
            direct_precipitation_m3_per_year: reader.u64()?,
            evaporation_m3_per_year: reader.u64()?,
            outflow_m3_per_year: reader.u64()?,
            status: decode_status(reader.u8()?)?,
        });
    }
    let river_len = reader.u32()? as usize;
    let mut rivers = Vec::with_capacity(river_len);
    for _ in 0..river_len {
        rivers.push(RiverSegment {
            id: reader.u32()?,
            source_cell: reader.usize()?,
            mouth_cell: reader.usize()?,
            strahler_order: reader.u16()?,
            destination: decode_destination(reader)?,
            spill_outlet: reader.bool()?,
            coordinate_count: reader.u32()?,
        });
    }
    let river_coordinates = reader.paths()?;
    let coast_len = reader.u32()? as usize;
    let mut coastline_segments = Vec::with_capacity(coast_len);
    for _ in 0..coast_len {
        coastline_segments.push(Segment {
            first: reader.point()?,
            second: reader.point()?,
        });
    }
    let lake_polygons = reader.polygons()?;
    let watershed_polygons = reader.polygons()?;
    let land_polygons = reader.polygons()?;
    let ocean_polygons = reader.polygons()?;
    let shelf_polygons = reader.polygons()?;
    let island_polygons = reader.polygons()?;
    let ice_polygons = reader.polygons()?;
    let bathymetry_len = reader.u32()? as usize;
    let mut bathymetry_contours = Vec::with_capacity(bathymetry_len);
    for _ in 0..bathymetry_len {
        bathymetry_contours.push(Segment {
            first: reader.point()?,
            second: reader.point()?,
        });
    }
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
        ice_thickness_mm,
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
        metrics: WaterBalanceMetrics {
            total_water_m3: reader.u64()?,
            ocean_water_m3: reader.u64()?,
            inland_water_m3: reader.u64()?,
            land_ice_m3: reader.u64()?,
            balance_error_m3: reader.u64()?,
            tolerance_m3: reader.u64()?,
            fixed_point_iterations: reader.u32()?,
            converged: reader.bool()?,
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{generate_world, GenerationSettings, NoopProgress, DEFAULT_RADIUS_METRES};

    #[test]
    fn static_derived_round_trips_through_the_cache_file() {
        let settings = GenerationSettings {
            width: 8,
            height: 4,
            radius_metres: DEFAULT_RADIUS_METRES,
            target_land_fraction_ppm: 300_000,
        };
        let mut progress = NoopProgress;
        let world = generate_world(settings, 831_429, 0, &mut progress).unwrap();
        let physics = StaticDerivedPhysics::from_world(&world).unwrap();
        let encoded = encode(&physics).unwrap();
        let decoded = decode(&encoded).unwrap();
        assert_eq!(decoded.climate, physics.climate);
        assert_eq!(decoded.evolution, physics.evolution);
        assert_eq!(decoded.hydrology, physics.hydrology);
        assert_eq!(decoded.ocean_curve, physics.ocean_curve);
        assert_eq!(decoded.geojson, physics.geojson);
        assert!(!decoded.ocean_curve.is_empty());
        assert_eq!(
            StaticDerivedPhysics::version_dir(),
            format!(
                "c{}-d{}-h{}-z{}",
                crate::climate::CLIMATE_DERIVATION_VERSION,
                crate::evolution::EVOLUTION_DERIVATION_VERSION,
                crate::hydrology::HYDROLOGY_DERIVATION_VERSION,
                crate::hazards::HAZARD_DERIVATION_VERSION,
            )
        );
    }
}
