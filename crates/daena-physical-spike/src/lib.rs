//! Pure, disposable feasibility spike for the native physical map model.
//!
//! This crate deliberately has no Daena, Tauri, SQLite, or frontend
//! dependency. Its integer source codec and deterministic field generation are
//! the parts that iteration 0 measures; the algorithms are not yet the
//! production physical generator.

use std::fmt::{Display, Write};
use std::sync::atomic::{AtomicBool, Ordering};

pub const SOURCE_MAGIC: [u8; 8] = *b"DAENAPW1";
pub const SOURCE_VERSION: u16 = 1;
pub const SOURCE_HEADER_BYTES: usize = 48;
pub const DEFAULT_WIDTH: u32 = 64;
pub const DEFAULT_HEIGHT: u32 = 32;
pub const DEFAULT_RADIUS_METRES: u64 = 6_371_000;
pub const MAX_WIDTH: u32 = 128;
pub const MAX_HEIGHT: u32 = 64;
pub const MAX_GEOJSON_FEATURES: usize = 32_768;
pub const GENERATOR_ID: &str = "daena-physical-world";
pub const GENERATOR_VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PhysicalError {
    InvalidSettings(String),
    InvalidSource(String),
    Validation(String),
    Cancelled,
}

impl Display for PhysicalError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidSettings(message) => write!(formatter, "invalid settings: {message}"),
            Self::InvalidSource(message) => write!(formatter, "invalid physical source: {message}"),
            Self::Validation(message) => write!(formatter, "physical validation failed: {message}"),
            Self::Cancelled => formatter.write_str("physical generation cancelled"),
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
        stage: &'static str,
        completed: u32,
        total: u32,
    ) -> Result<(), PhysicalError>;
}

pub struct NoopProgress;

impl ProgressSink for NoopProgress {
    fn report(
        &mut self,
        _stage: &'static str,
        _completed: u32,
        _total: u32,
    ) -> Result<(), PhysicalError> {
        Ok(())
    }
}

pub struct CancellationProgress<'a> {
    pub cancelled: &'a AtomicBool,
    pub on_progress: &'a mut dyn FnMut(&'static str, u32, u32),
}

impl ProgressSink for CancellationProgress<'_> {
    fn report(
        &mut self,
        stage: &'static str,
        completed: u32,
        total: u32,
    ) -> Result<(), PhysicalError> {
        if self.cancelled.load(Ordering::Relaxed) {
            return Err(PhysicalError::Cancelled);
        }
        (self.on_progress)(stage, completed, total);
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GeneratedWorld {
    pub field: PhysicalField,
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
    let grid = settings.grid()?;
    progress.report("elevation", 0, grid.height)?;
    let mut elevations_mm = Vec::with_capacity(grid.sample_count());
    for row in 0..grid.height {
        for col in 0..grid.width {
            elevations_mm.push(elevation_for(seed, retry_index, grid.index(row, col)));
        }
        progress.report("elevation", row + 1, grid.height)?;
    }
    progress.report("sea-level", 0, 1)?;
    let sea_level_mm = solve_sea_level(&grid, &elevations_mm, settings.target_land_fraction_ppm)
        .map_err(PhysicalError::Validation)?;
    let field = PhysicalField {
        grid,
        seed,
        retry_index,
        target_land_fraction_ppm: settings.target_land_fraction_ppm,
        sea_level_mm,
        elevations_mm,
    };
    let report = validate_field_report(&field)?;
    progress.report("derived-coastline", 0, 1)?;
    let derived_geojson = to_geojson(&field).map_err(PhysicalError::Validation)?;
    let source = encode_source(&field).map_err(PhysicalError::InvalidSource)?;
    progress.report("complete", 1, 1)?;
    Ok(GeneratedWorld {
        field,
        source,
        derived_geojson,
        report,
    })
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

pub fn encode_source(field: &PhysicalField) -> Result<Vec<u8>, String> {
    field.validate()?;
    let mut output = Vec::with_capacity(SOURCE_HEADER_BYTES + field.elevations_mm.len() * 4);
    output.extend_from_slice(&SOURCE_MAGIC);
    push_u16(&mut output, SOURCE_VERSION);
    push_u16(&mut output, SOURCE_HEADER_BYTES as u16);
    push_u32(&mut output, field.grid.width);
    push_u32(&mut output, field.grid.height);
    push_u64(&mut output, field.grid.radius_metres);
    push_u32(&mut output, field.seed);
    push_u32(&mut output, field.retry_index);
    push_u32(&mut output, field.target_land_fraction_ppm);
    push_i32(&mut output, field.sea_level_mm);
    push_u32(&mut output, field.elevations_mm.len() as u32);
    for elevation in &field.elevations_mm {
        push_i32(&mut output, *elevation);
    }
    Ok(output)
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

pub fn decode_source(bytes: &[u8]) -> Result<PhysicalField, String> {
    if bytes.len() < SOURCE_HEADER_BYTES {
        return Err("source is shorter than its fixed header".into());
    }
    let mut offset = 0;
    if read_exact::<8>(bytes, &mut offset)? != SOURCE_MAGIC {
        return Err("source magic does not match physical-world-v1".into());
    }
    let version = u16::from_le_bytes(read_exact(bytes, &mut offset)?);
    if version != SOURCE_VERSION {
        return Err("source version is unsupported".into());
    }
    let header_bytes = u16::from_le_bytes(read_exact(bytes, &mut offset)?);
    if header_bytes as usize != SOURCE_HEADER_BYTES {
        return Err("source header length is unsupported".into());
    }
    let width = u32::from_le_bytes(read_exact(bytes, &mut offset)?);
    let height = u32::from_le_bytes(read_exact(bytes, &mut offset)?);
    let radius_metres = u64::from_le_bytes(read_exact(bytes, &mut offset)?);
    let grid = Grid::new(width, height, radius_metres)?;
    let seed = u32::from_le_bytes(read_exact(bytes, &mut offset)?);
    let retry_index = u32::from_le_bytes(read_exact(bytes, &mut offset)?);
    let target_land_fraction_ppm = u32::from_le_bytes(read_exact(bytes, &mut offset)?);
    let sea_level_mm = i32::from_le_bytes(read_exact(bytes, &mut offset)?);
    let sample_count = u32::from_le_bytes(read_exact(bytes, &mut offset)?) as usize;
    if sample_count != grid.sample_count() {
        return Err("source sample count does not match the grid".into());
    }
    let expected_len = SOURCE_HEADER_BYTES
        .checked_add(
            sample_count
                .checked_mul(4)
                .ok_or_else(|| "source size overflow".to_string())?,
        )
        .ok_or_else(|| "source size overflow".to_string())?;
    if bytes.len() != expected_len {
        return Err("source has trailing or missing sample bytes".into());
    }
    let mut elevations_mm = Vec::with_capacity(sample_count);
    for _ in 0..sample_count {
        elevations_mm.push(i32::from_le_bytes(read_exact(bytes, &mut offset)?));
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> PhysicalField {
        generate_field(
            Grid::new(DEFAULT_WIDTH, DEFAULT_HEIGHT, DEFAULT_RADIUS_METRES).unwrap(),
            831_429,
            0,
            300_000,
        )
        .unwrap()
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
        let field = fixture();
        let encoded = encode_source(&field).unwrap();
        assert_eq!(decode_source(&encoded).unwrap(), field);
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
    }

    #[test]
    fn coastline_and_geojson_are_bounded_and_seam_safe() {
        let field = fixture();
        let segments = coastline_segments(&field);
        assert!(!segments.is_empty());
        assert!(segments.len() <= MAX_GEOJSON_FEATURES);
        for segment in &segments {
            for point in [segment.first, segment.second] {
                assert!((-180_000_000..=180_000_000).contains(&point[0]));
                assert!((-90_000_000..=90_000_000).contains(&point[1]));
            }
        }
        let geojson = to_geojson(&field).unwrap();
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
        assert!(world.derived_geojson.contains("physical coastline"));

        let cancelled = AtomicBool::new(true);
        let mut updates = |_stage: &'static str, _completed: u32, _total: u32| {};
        let mut progress = CancellationProgress {
            cancelled: &cancelled,
            on_progress: &mut updates,
        };
        assert_eq!(
            generate_world(settings, 831_429, 0, &mut progress),
            Err(PhysicalError::Cancelled)
        );
    }
}
