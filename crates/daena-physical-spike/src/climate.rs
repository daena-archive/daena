//! Deterministic current-climate and runoff derivation.
//!
//! Climate is a disposable interpretation of the accepted physical field. It
//! never changes the canonical elevation/source bytes. The model is purposely
//! bounded: a latitude/altitude temperature field feeds a fixed-point moisture
//! transport pass, and precipitation feeds runoff volumes using exact
//! spherical cell areas.

use super::{
    derive_subsystem_seed, splitmix64, Grid, PhysicalError, PhysicalErrorCode, PhysicalField,
    ProgressPhase, ProgressSink, SeedDomain,
};

pub const CLIMATE_DERIVATION_VERSION: u16 = 1;
pub const CLIMATE_WIND_BAND_COUNT: u32 = 6;
pub const CLIMATE_MAX_TRANSPORT_ITERATIONS: u32 = 96;
const CLIMATE_MIN_TRANSPORT_ITERATIONS: u32 = 8;
const CLIMATE_TRANSPORT_TOLERANCE_MM: f64 = 1.0;
const MAX_CLIMATE_PRECIPITATION_MM: u32 = 100_000;
const MAX_CLIMATE_MOISTURE_MM: u32 = 100_000;
const MAX_CLIMATE_TEMPERATURE_CENTI_C: i32 = 10_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HydrologyPreset {
    Arid,
    Balanced,
    Wet,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClimateSettings {
    pub global_temperature_centi_c: i32,
    pub latitude_cooling_centi_c: i32,
    pub altitude_lapse_centi_c_per_km: i32,
    pub maritime_moderation_centi_c: i32,
    pub maritime_scale_km: u32,
    pub ocean_moisture_mm_per_year: u32,
    pub moisture_decay_ppm: u32,
    pub moisture_decay_scale_km: u32,
    pub convergence_ppm: u32,
    pub base_precipitation_ppm: u32,
    pub orographic_precipitation_ppm: u32,
    pub hydrology_preset: HydrologyPreset,
}

impl ClimateSettings {
    pub fn default_for(_grid: Grid) -> Self {
        Self {
            global_temperature_centi_c: 1_400,
            latitude_cooling_centi_c: 3_200,
            altitude_lapse_centi_c_per_km: 650,
            maritime_moderation_centi_c: 1_000,
            maritime_scale_km: 3_000,
            ocean_moisture_mm_per_year: 1_600,
            moisture_decay_ppm: 940_000,
            moisture_decay_scale_km: 8_000,
            convergence_ppm: 120_000,
            base_precipitation_ppm: 70_000,
            orographic_precipitation_ppm: 18_000_000,
            hydrology_preset: HydrologyPreset::Balanced,
        }
    }

    pub fn validate(self) -> Result<(), PhysicalError> {
        if !(-5_000..=5_000).contains(&self.global_temperature_centi_c)
            || !(0..=10_000).contains(&self.latitude_cooling_centi_c)
            || !(0..=2_000).contains(&self.altitude_lapse_centi_c_per_km)
            || !(0..=5_000).contains(&self.maritime_moderation_centi_c)
            || !(1..=20_000).contains(&self.maritime_scale_km)
            || !(100..=100_000).contains(&self.moisture_decay_scale_km)
        {
            return Err(PhysicalError::InvalidSettings(
                "climate temperature parameters are outside the bounded range".into(),
            ));
        }
        if self.ocean_moisture_mm_per_year == 0
            || self.ocean_moisture_mm_per_year > MAX_CLIMATE_PRECIPITATION_MM
            || !(700_000..=999_000).contains(&self.moisture_decay_ppm)
            || self.convergence_ppm > 500_000
            || self.base_precipitation_ppm > 500_000
            || self.orographic_precipitation_ppm > 50_000_000
        {
            return Err(PhysicalError::InvalidSettings(
                "climate moisture parameters are outside the bounded range".into(),
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClimateMetrics {
    pub precipitation_volume_m3_per_year: u64,
    pub runoff_volume_m3_per_year: u64,
    pub mean_temperature_centi_c: i32,
    pub minimum_temperature_centi_c: i32,
    pub maximum_temperature_centi_c: i32,
    pub mean_precipitation_mm_per_year: u32,
    pub mean_runoff_mm_per_year: u32,
    pub wettest_cell_precipitation_mm_per_year: u32,
    pub driest_land_cell_precipitation_mm_per_year: u32,
    pub transport_iterations: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClimateField {
    pub grid: Grid,
    pub derivation_version: u16,
    pub temperature_centi_c: Vec<i32>,
    pub moisture_mm_per_year: Vec<u32>,
    pub precipitation_mm_per_year: Vec<u32>,
    pub runoff_mm_per_year: Vec<u32>,
    pub runoff_volume_m3_per_year: Vec<u64>,
    pub maritime_factor_ppm: Vec<u32>,
    pub metrics: ClimateMetrics,
}

impl ClimateField {
    pub fn validate(&self) -> Result<(), PhysicalError> {
        let expected = self.grid.sample_count();
        if self.derivation_version != CLIMATE_DERIVATION_VERSION
            || self.temperature_centi_c.len() != expected
            || self.moisture_mm_per_year.len() != expected
            || self.precipitation_mm_per_year.len() != expected
            || self.runoff_mm_per_year.len() != expected
            || self.runoff_volume_m3_per_year.len() != expected
            || self.maritime_factor_ppm.len() != expected
        {
            return Err(PhysicalError::coded(
                PhysicalErrorCode::NumericNonFinite,
                "climate field shape or derivation version is invalid",
            ));
        }
        if self.temperature_centi_c.iter().any(|value| {
            !(-MAX_CLIMATE_TEMPERATURE_CENTI_C..=MAX_CLIMATE_TEMPERATURE_CENTI_C).contains(value)
        }) || self
            .moisture_mm_per_year
            .iter()
            .chain(self.precipitation_mm_per_year.iter())
            .chain(self.runoff_mm_per_year.iter())
            .any(|value| *value > MAX_CLIMATE_PRECIPITATION_MM)
            || self
                .maritime_factor_ppm
                .iter()
                .any(|value| *value > 1_000_000)
        {
            return Err(PhysicalError::coded(
                PhysicalErrorCode::NumericNonFinite,
                "climate field contains a non-finite or unbounded value",
            ));
        }
        Ok(())
    }

    pub fn validate_against(&self, field: &PhysicalField) -> Result<(), PhysicalError> {
        self.validate()?;
        field.validate().map_err(PhysicalError::InvalidSource)?;
        if self.grid != field.grid {
            return Err(PhysicalError::coded(
                PhysicalErrorCode::GeometryInvalid,
                "climate field grid does not match the physical field",
            ));
        }
        let mut precipitation_volume = 0.0;
        let mut runoff_volume = 0.0;
        let mut wettest = 0;
        let mut driest_land = u32::MAX;
        for cell in 0..self.grid.sample_count() {
            let area = self.grid.cell_area(self.grid.row_col(cell).0);
            precipitation_volume +=
                area * f64::from(self.precipitation_mm_per_year[cell]) / 1_000.0;
            if field.elevations_mm[cell] <= field.sea_level_mm {
                if self.runoff_mm_per_year[cell] != 0 || self.runoff_volume_m3_per_year[cell] != 0 {
                    return Err(PhysicalError::Validation(
                        "climate runoff must be zero on ocean cells".into(),
                    ));
                }
            } else {
                if self.runoff_mm_per_year[cell] > self.precipitation_mm_per_year[cell] {
                    return Err(PhysicalError::Validation(
                        "climate runoff cannot exceed local precipitation".into(),
                    ));
                }
                driest_land = driest_land.min(self.precipitation_mm_per_year[cell]);
            }
            wettest = wettest.max(self.precipitation_mm_per_year[cell]);
            let expected_volume =
                round_volume(area * f64::from(self.runoff_mm_per_year[cell]) / 1_000.0)?;
            if expected_volume != self.runoff_volume_m3_per_year[cell] {
                return Err(PhysicalError::Validation(
                    "climate runoff volume does not match cell area".into(),
                ));
            }
            runoff_volume += f64::from(self.runoff_mm_per_year[cell]) * area / 1_000.0;
        }
        if precipitation_volume.round() as u64 != self.metrics.precipitation_volume_m3_per_year
            || runoff_volume.round() as u64 != self.metrics.runoff_volume_m3_per_year
            || wettest != self.metrics.wettest_cell_precipitation_mm_per_year
            || (if driest_land == u32::MAX {
                0
            } else {
                driest_land
            }) != self.metrics.driest_land_cell_precipitation_mm_per_year
        {
            return Err(PhysicalError::Validation(
                "climate metrics do not match derived fields".into(),
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy)]
struct UnitVector {
    x: f64,
    y: f64,
    z: f64,
}

impl UnitVector {
    fn from_lon_lat(longitude: f64, latitude: f64) -> Self {
        let latitude_cos = latitude.cos();
        Self {
            x: latitude_cos * longitude.cos(),
            y: latitude_cos * longitude.sin(),
            z: latitude.sin(),
        }
    }

    fn dot(self, other: Self) -> f64 {
        self.x * other.x + self.y * other.y + self.z * other.z
    }
}

#[derive(Debug, Clone, Copy)]
struct CellClimateGeometry {
    latitude: f64,
    maritime_factor: f64,
}

fn cell_geometry(grid: Grid, cell: usize) -> (f64, UnitVector) {
    let (row, col) = grid.row_col(cell);
    let (longitude, latitude) = grid.center_radians(row, col);
    (latitude, UnitVector::from_lon_lat(longitude, latitude))
}

fn build_geometry(
    field: &PhysicalField,
    settings: ClimateSettings,
    progress: &mut dyn ProgressSink,
) -> Result<Vec<CellClimateGeometry>, PhysicalError> {
    let ocean_vectors = (0..field.grid.sample_count())
        .filter(|cell| field.elevations_mm[*cell] <= field.sea_level_mm)
        .map(|cell| cell_geometry(field.grid, cell).1)
        .collect::<Vec<_>>();
    if ocean_vectors.is_empty() {
        return Err(PhysicalError::coded(
            PhysicalErrorCode::GeometryInvalid,
            "climate derivation requires at least one ocean cell",
        ));
    }
    let ocean_tree = KdNode::build(&ocean_vectors);
    let maritime_scale_metres = f64::from(settings.maritime_scale_km) * 1_000.0;
    let mut geometry = Vec::with_capacity(field.grid.sample_count());
    for cell in 0..field.grid.sample_count() {
        if cell % 128 == 0 {
            progress.check_cancelled()?;
        }
        let (latitude, vector) = cell_geometry(field.grid, cell);
        let distance = if field.elevations_mm[cell] <= field.sea_level_mm {
            0.0
        } else {
            nearest_ocean_distance(&ocean_vectors, ocean_tree.as_deref(), vector)
                * field.grid.radius_metres as f64
        };
        let maritime_factor = (-distance / maritime_scale_metres).exp();
        if !distance.is_finite() || !maritime_factor.is_finite() {
            return Err(PhysicalError::coded(
                PhysicalErrorCode::NumericNonFinite,
                "climate maritime distance is not finite",
            ));
        }
        geometry.push(CellClimateGeometry {
            latitude,
            maritime_factor,
        });
    }
    Ok(geometry)
}

struct KdNode {
    index: usize,
    axis: u8,
    left: Option<Box<KdNode>>,
    right: Option<Box<KdNode>>,
}

impl KdNode {
    fn build(points: &[UnitVector]) -> Option<Box<Self>> {
        if points.is_empty() {
            return None;
        }
        let mut indices = (0..points.len()).collect::<Vec<_>>();
        Self::build_from_indices(points, &mut indices, 0)
    }

    fn build_from_indices(
        points: &[UnitVector],
        indices: &mut [usize],
        depth: usize,
    ) -> Option<Box<Self>> {
        if indices.is_empty() {
            return None;
        }
        let axis = (depth % 3) as u8;
        indices.sort_unstable_by(|first, second| {
            coord(points[*first], axis)
                .partial_cmp(&coord(points[*second], axis))
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| first.cmp(second))
        });
        let mid = indices.len() / 2;
        Some(Box::new(Self {
            index: indices[mid],
            axis,
            left: Self::build_from_indices(points, &mut indices[..mid], depth + 1),
            right: Self::build_from_indices(points, &mut indices[mid + 1..], depth + 1),
        }))
    }

    fn nearest(
        &self,
        points: &[UnitVector],
        query: UnitVector,
        best_distance_sq: &mut f64,
        best_index: &mut usize,
    ) {
        let candidate = points[self.index];
        let distance_sq = chord_squared(candidate, query);
        if distance_sq < *best_distance_sq
            || ((distance_sq - *best_distance_sq).abs() <= f64::EPSILON && self.index < *best_index)
        {
            *best_distance_sq = distance_sq;
            *best_index = self.index;
        }
        let delta = coord(query, self.axis) - coord(candidate, self.axis);
        let (first, second) = if delta <= 0.0 {
            (self.left.as_deref(), self.right.as_deref())
        } else {
            (self.right.as_deref(), self.left.as_deref())
        };
        if let Some(child) = first {
            child.nearest(points, query, best_distance_sq, best_index);
        }
        if second.is_some() && delta * delta <= *best_distance_sq {
            if let Some(child) = second {
                child.nearest(points, query, best_distance_sq, best_index);
            }
        }
    }
}

fn coord(vector: UnitVector, axis: u8) -> f64 {
    match axis {
        0 => vector.x,
        1 => vector.y,
        _ => vector.z,
    }
}

fn chord_squared(first: UnitVector, second: UnitVector) -> f64 {
    let dx = first.x - second.x;
    let dy = first.y - second.y;
    let dz = first.z - second.z;
    dx * dx + dy * dy + dz * dz
}

fn nearest_ocean_distance(
    ocean_vectors: &[UnitVector],
    tree: Option<&KdNode>,
    vector: UnitVector,
) -> f64 {
    if ocean_vectors.len() <= 48 {
        return ocean_vectors
            .iter()
            .map(|ocean| ocean.dot(vector).clamp(-1.0, 1.0).acos())
            .fold(f64::INFINITY, f64::min);
    }
    let Some(tree) = tree else {
        return f64::INFINITY;
    };
    let mut best_distance_sq = f64::INFINITY;
    let mut best_index = 0usize;
    tree.nearest(
        ocean_vectors,
        vector,
        &mut best_distance_sq,
        &mut best_index,
    );
    ocean_vectors[best_index]
        .dot(vector)
        .clamp(-1.0, 1.0)
        .acos()
}

fn temperature_field(
    field: &PhysicalField,
    settings: ClimateSettings,
    geometry: &[CellClimateGeometry],
    progress: &mut dyn ProgressSink,
) -> Result<(Vec<i32>, Vec<u32>), PhysicalError> {
    let mut temperatures = Vec::with_capacity(field.grid.sample_count());
    let mut maritime_factors = Vec::with_capacity(field.grid.sample_count());
    for (cell, cell_geometry) in geometry.iter().copied().enumerate() {
        if cell % 128 == 0 {
            progress.check_cancelled()?;
        }
        let latitude_fraction =
            (cell_geometry.latitude.abs() / std::f64::consts::FRAC_PI_2).clamp(0.0, 1.0);
        let latitude_cooling =
            f64::from(settings.latitude_cooling_centi_c) * latitude_fraction.powf(1.35);
        let altitude_km = (f64::from(field.elevations_mm[cell] - field.sea_level_mm)) / 1_000_000.0;
        let altitude_cooling = altitude_km * f64::from(settings.altitude_lapse_centi_c_per_km);
        let continental_temperature =
            f64::from(settings.global_temperature_centi_c) - latitude_cooling - altitude_cooling;
        let maritime_temperature = f64::from(settings.global_temperature_centi_c)
            - latitude_cooling * 0.58
            - altitude_cooling * 0.62
            + f64::from(settings.maritime_moderation_centi_c) * 0.18;
        let temperature = continental_temperature
            + (maritime_temperature - continental_temperature) * cell_geometry.maritime_factor;
        if !temperature.is_finite() {
            return Err(PhysicalError::coded(
                PhysicalErrorCode::NumericNonFinite,
                "climate temperature is not finite",
            ));
        }
        temperatures.push(temperature.round().clamp(
            -f64::from(MAX_CLIMATE_TEMPERATURE_CENTI_C),
            f64::from(MAX_CLIMATE_TEMPERATURE_CENTI_C),
        ) as i32);
        maritime_factors.push((cell_geometry.maritime_factor * 1_000_000.0).round() as u32);
    }
    Ok((temperatures, maritime_factors))
}

fn wind_direction(row: u32, height: u32) -> i32 {
    let latitude = -std::f64::consts::FRAC_PI_2
        + std::f64::consts::PI * (f64::from(row) + 0.5) / f64::from(height);
    let absolute_degrees = latitude.abs().to_degrees();
    if !(30.0..60.0).contains(&absolute_degrees) {
        -1
    } else {
        1
    }
}

fn hydrology_parameters(preset: HydrologyPreset) -> (f64, f64, f64, f64) {
    match preset {
        HydrologyPreset::Arid => (0.75, 0.90, 0.24, 0.18),
        HydrologyPreset::Balanced => (1.0, 1.0, 0.40, 0.28),
        HydrologyPreset::Wet => (1.15, 1.015, 0.56, 0.34),
    }
}

fn band_source_multiplier(seed: u64, row: u32, height: u32) -> f64 {
    let band = row.saturating_mul(CLIMATE_WIND_BAND_COUNT) / height.max(1);
    let random = splitmix64(seed ^ u64::from(band).wrapping_mul(0x9e37_79b9_7f4a_7c15));
    0.90 + f64::from((random % 200_001) as u32) / 1_000_000.0
}

fn convergence_row(row: u32, height: u32) -> u32 {
    if row < height / 2 {
        (row + 1).min(height - 1)
    } else {
        row.saturating_sub(1)
    }
}

fn transport_moisture(
    field: &PhysicalField,
    settings: ClimateSettings,
    climate_seed: u64,
    progress: &mut dyn ProgressSink,
) -> Result<(Vec<u32>, Vec<u32>, u32), PhysicalError> {
    let (source_multiplier, decay_multiplier, _, _) =
        hydrology_parameters(settings.hydrology_preset);
    let decay_at_scale = f64::from(settings.moisture_decay_ppm) / 1_000_000.0 * decay_multiplier;
    let convergence = f64::from(settings.convergence_ppm) / 1_000_000.0;
    // Converging wind bands share moisture, but the two transport weights are
    // kept below one in total so the wrapped fixed-point pass is contractive.
    let convergence_weight = convergence * 0.5;
    let base_precipitation = f64::from(settings.base_precipitation_ppm) / 1_000_000.0;
    let sample_count = field.grid.sample_count();
    let mut previous = vec![0.0; sample_count];
    let mut next = vec![0.0; sample_count];
    let mut precipitation = vec![0.0; sample_count];
    let mut final_precipitation = vec![0.0; sample_count];
    let mut iterations = 0;
    let mut converged = false;
    let row_step_metres = (0..field.grid.height)
        .map(|row| {
            field
                .grid
                .great_circle_distance(
                    field.grid.center_radians(row, 0),
                    field.grid.center_radians(row, 1 % field.grid.width.max(1)),
                )
                .max(1.0)
        })
        .collect::<Vec<_>>();

    for iteration in 0..CLIMATE_MAX_TRANSPORT_ITERATIONS {
        progress.check_cancelled()?;
        let mut maximum_delta = 0.0_f64;
        for row in 0..field.grid.height {
            if row % 4 == 0 {
                progress.check_cancelled()?;
            }
            let direction = wind_direction(row, field.grid.height);
            let adjacent_row = convergence_row(row, field.grid.height);
            let source_factor = band_source_multiplier(climate_seed, row, field.grid.height);
            let distance = row_step_metres[row as usize];
            let physical_decay =
                (-distance / (f64::from(settings.moisture_decay_scale_km) * 1_000.0)).exp();
            let upstream_weight =
                (decay_at_scale * physical_decay).clamp(0.0, 0.995) * (1.0 - convergence_weight);
            for offset in 0..field.grid.width {
                let col = if direction > 0 {
                    offset
                } else {
                    field.grid.width - 1 - offset
                };
                let cell = field.grid.index(row, col);
                let upstream_col = if direction > 0 {
                    (col + field.grid.width - 1) % field.grid.width
                } else {
                    (col + 1) % field.grid.width
                };
                let upstream = field.grid.index(row, upstream_col);
                let adjacent = field.grid.index(adjacent_row, col);
                let ocean_source = if field.elevations_mm[cell] <= field.sea_level_mm {
                    f64::from(settings.ocean_moisture_mm_per_year)
                        * source_multiplier
                        * source_factor
                } else {
                    0.0
                };
                let incoming = (ocean_source
                    + previous[upstream] * upstream_weight
                    + previous[adjacent] * convergence_weight)
                    .clamp(0.0, f64::from(MAX_CLIMATE_MOISTURE_MM));
                let uphill_metres =
                    f64::from(field.elevations_mm[cell] - field.elevations_mm[upstream]) / 1_000.0;
                let slope = (uphill_metres / distance).max(0.0);
                let orographic =
                    slope * f64::from(settings.orographic_precipitation_ppm) / 1_000_000.0;
                let precipitation_fraction = (base_precipitation + orographic).clamp(0.0, 0.65);
                let precipitated = incoming * precipitation_fraction;
                let remaining = (incoming - precipitated).max(0.0);
                next[cell] = remaining;
                precipitation[cell] = precipitated;
                maximum_delta = maximum_delta.max((remaining - previous[cell]).abs());
            }
        }
        std::mem::swap(&mut previous, &mut next);
        final_precipitation.copy_from_slice(&precipitation);
        iterations = iteration + 1;
        if iterations >= CLIMATE_MIN_TRANSPORT_ITERATIONS
            && maximum_delta <= CLIMATE_TRANSPORT_TOLERANCE_MM
        {
            converged = true;
            break;
        }
    }
    if !converged {
        return Err(PhysicalError::coded(
            PhysicalErrorCode::NumericNonConvergent,
            format!(
                "climate moisture transport did not converge within {CLIMATE_MAX_TRANSPORT_ITERATIONS} iterations"
            ),
        ));
    }
    let moisture = previous
        .into_iter()
        .map(|value| value.round().clamp(0.0, f64::from(MAX_CLIMATE_MOISTURE_MM)) as u32)
        .collect::<Vec<_>>();
    let precipitation = final_precipitation
        .into_iter()
        .map(|value| {
            value
                .round()
                .clamp(0.0, f64::from(MAX_CLIMATE_PRECIPITATION_MM)) as u32
        })
        .collect::<Vec<_>>();
    Ok((moisture, precipitation, iterations))
}

fn runoff_fields(
    field: &PhysicalField,
    settings: ClimateSettings,
    temperatures: &[i32],
    precipitation: &[u32],
    progress: &mut dyn ProgressSink,
) -> Result<(Vec<u32>, Vec<u64>, ClimateMetrics), PhysicalError> {
    let (_, _, base_runoff, runoff_response) = hydrology_parameters(settings.hydrology_preset);
    let total_area = field.grid.total_area();
    let mut runoff = Vec::with_capacity(field.grid.sample_count());
    let mut runoff_volume = Vec::with_capacity(field.grid.sample_count());
    let mut precipitation_volume = 0.0_f64;
    let mut runoff_volume_total = 0.0_f64;
    let mut temperature_area_sum = 0.0_f64;
    let mut precipitation_area_sum = 0.0_f64;
    let mut runoff_area_sum = 0.0_f64;
    let mut minimum_temperature = i32::MAX;
    let mut maximum_temperature = i32::MIN;
    let mut wettest = 0;
    let mut driest_land = u32::MAX;
    for cell in 0..field.grid.sample_count() {
        if cell % 128 == 0 {
            progress.check_cancelled()?;
        }
        let area = field.grid.cell_area(field.grid.row_col(cell).0);
        let precip = precipitation[cell];
        let precip_volume = area * f64::from(precip) / 1_000.0;
        let is_land = field.elevations_mm[cell] > field.sea_level_mm;
        let wetness = (f64::from(precip) / 1_500.0).clamp(0.0, 1.0);
        let temperature_factor = if temperatures[cell] < 0 { 0.72 } else { 1.0 };
        let coefficient = if is_land {
            (base_runoff + wetness * runoff_response) * temperature_factor
        } else {
            0.0
        }
        .clamp(0.0, 0.95);
        let runoff_mm = (f64::from(precip) * coefficient)
            .round()
            .clamp(0.0, f64::from(MAX_CLIMATE_PRECIPITATION_MM)) as u32;
        let runoff_m3 = area * f64::from(runoff_mm) / 1_000.0;
        if !precip_volume.is_finite() || !runoff_m3.is_finite() {
            return Err(PhysicalError::coded(
                PhysicalErrorCode::NumericNonFinite,
                "climate precipitation or runoff volume is not finite",
            ));
        }
        precipitation_volume += precip_volume;
        runoff_volume_total += runoff_m3;
        temperature_area_sum += f64::from(temperatures[cell]) * area;
        precipitation_area_sum += f64::from(precip) * area;
        runoff_area_sum += f64::from(runoff_mm) * area;
        minimum_temperature = minimum_temperature.min(temperatures[cell]);
        maximum_temperature = maximum_temperature.max(temperatures[cell]);
        wettest = wettest.max(precip);
        if is_land {
            driest_land = driest_land.min(precip);
        }
        runoff.push(runoff_mm);
        runoff_volume.push(round_volume(runoff_m3)?);
    }
    if !precipitation_volume.is_finite()
        || !runoff_volume_total.is_finite()
        || precipitation_volume > u64::MAX as f64
        || runoff_volume_total > u64::MAX as f64
    {
        return Err(PhysicalError::coded(
            PhysicalErrorCode::NumericNonFinite,
            "climate total water volumes are not finite or bounded",
        ));
    }
    Ok((
        runoff,
        runoff_volume,
        ClimateMetrics {
            precipitation_volume_m3_per_year: precipitation_volume.round() as u64,
            runoff_volume_m3_per_year: runoff_volume_total.round() as u64,
            mean_temperature_centi_c: (temperature_area_sum / total_area).round() as i32,
            minimum_temperature_centi_c: minimum_temperature,
            maximum_temperature_centi_c: maximum_temperature,
            mean_precipitation_mm_per_year: (precipitation_area_sum / total_area).round() as u32,
            mean_runoff_mm_per_year: (runoff_area_sum / total_area).round() as u32,
            wettest_cell_precipitation_mm_per_year: wettest,
            driest_land_cell_precipitation_mm_per_year: if driest_land == u32::MAX {
                0
            } else {
                driest_land
            },
            transport_iterations: 0,
        },
    ))
}

fn round_volume(volume_m3: f64) -> Result<u64, PhysicalError> {
    if !volume_m3.is_finite() || !(0.0..=(u64::MAX as f64)).contains(&volume_m3) {
        return Err(PhysicalError::coded(
            PhysicalErrorCode::NumericNonFinite,
            "climate cell volume is not finite or bounded",
        ));
    }
    Ok(volume_m3.round() as u64)
}

pub fn derive_current_climate(
    field: &PhysicalField,
    settings: ClimateSettings,
    seed: u32,
    retry_index: u32,
    progress: &mut dyn ProgressSink,
) -> Result<ClimateField, PhysicalError> {
    field.validate().map_err(PhysicalError::InvalidSource)?;
    settings.validate()?;
    progress.report(ProgressPhase::CalculatingClimate, 0, 4)?;
    let geometry = build_geometry(field, settings, progress)?;
    progress.report(ProgressPhase::CalculatingClimate, 1, 4)?;
    let (temperatures, maritime_factors) = temperature_field(field, settings, &geometry, progress)?;
    progress.report(ProgressPhase::CalculatingClimate, 2, 4)?;
    let climate_seed = derive_subsystem_seed(seed, retry_index, SeedDomain::Climate);
    let (moisture, precipitation, transport_iterations) =
        transport_moisture(field, settings, climate_seed, progress)?;
    let (runoff, runoff_volume, mut metrics) =
        runoff_fields(field, settings, &temperatures, &precipitation, progress)?;
    metrics.transport_iterations = transport_iterations;
    progress.report(ProgressPhase::CalculatingClimate, 3, 4)?;
    let climate = ClimateField {
        grid: field.grid,
        derivation_version: CLIMATE_DERIVATION_VERSION,
        temperature_centi_c: temperatures,
        moisture_mm_per_year: moisture,
        precipitation_mm_per_year: precipitation,
        runoff_mm_per_year: runoff,
        runoff_volume_m3_per_year: runoff_volume,
        maritime_factor_ppm: maritime_factors,
        metrics,
    };
    climate.validate_against(field)?;
    progress.report(ProgressPhase::CalculatingClimate, 4, 4)?;
    Ok(climate)
}

/// Analytic helper used by the climate exit gate. It deliberately uses the
/// exact spherical cell areas rather than raster-cell counts.
pub fn uniform_runoff_volume_m3_per_year(
    grid: Grid,
    precipitation_mm_per_year: u32,
    runoff_coefficient_ppm: u32,
) -> Result<u64, PhysicalError> {
    if runoff_coefficient_ppm > 1_000_000 {
        return Err(PhysicalError::InvalidSettings(
            "uniform runoff coefficient must be parts per million".into(),
        ));
    }
    let volume = grid.total_area() * f64::from(precipitation_mm_per_year) / 1_000.0
        * f64::from(runoff_coefficient_ppm)
        / 1_000_000.0;
    round_volume(volume)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{NoopProgress, DEFAULT_RADIUS_METRES};

    fn field(grid: Grid, elevations_mm: Vec<i32>, sea_level_mm: i32) -> PhysicalField {
        PhysicalField {
            grid,
            seed: 831_429,
            retry_index: 0,
            target_land_fraction_ppm: 300_000,
            sea_level_mm,
            elevations_mm,
        }
    }

    #[test]
    fn uniform_runoff_uses_exact_spherical_area() {
        let expected =
            (4.0 * std::f64::consts::PI * (DEFAULT_RADIUS_METRES as f64).powi(2) * 1_000.0
                / 1_000.0
                * 400_000.0
                / 1_000_000.0)
                .round() as u64;
        for grid in [
            Grid::new(32, 16, DEFAULT_RADIUS_METRES).unwrap(),
            Grid::new(64, 32, DEFAULT_RADIUS_METRES).unwrap(),
            Grid::new(128, 64, DEFAULT_RADIUS_METRES).unwrap(),
        ] {
            let actual = uniform_runoff_volume_m3_per_year(grid, 1_000, 400_000).unwrap();
            assert_eq!(
                actual, expected,
                "row-count bias at {}x{}",
                grid.width, grid.height
            );
        }
    }

    #[test]
    fn temperature_falls_with_latitude_and_maritime_distance_is_bounded() {
        let grid = Grid::new(16, 8, DEFAULT_RADIUS_METRES).unwrap();
        let mut progress = NoopProgress;
        let physical = field(grid, vec![0; grid.sample_count()], 1);
        let climate = derive_current_climate(
            &physical,
            ClimateSettings::default_for(grid),
            physical.seed,
            physical.retry_index,
            &mut progress,
        )
        .unwrap();
        assert!(
            climate.temperature_centi_c[grid.index(3, 0)]
                > climate.temperature_centi_c[grid.index(0, 0)]
        );
        assert!(climate
            .maritime_factor_ppm
            .iter()
            .all(|value| *value == 1_000_000));
        assert!(
            climate.metrics.minimum_temperature_centi_c <= climate.metrics.mean_temperature_centi_c
        );
        assert!(
            climate.metrics.mean_temperature_centi_c <= climate.metrics.maximum_temperature_centi_c
        );
    }

    #[test]
    fn transport_is_periodic_across_the_antimeridian() {
        let grid = Grid::new(16, 8, DEFAULT_RADIUS_METRES).unwrap();
        let mut elevations = vec![-2_000; grid.sample_count()];
        for row in 4..8 {
            for col in 0..grid.width {
                elevations[grid.index(row, col)] = 1_000;
            }
        }
        let physical = field(grid, elevations, 0);
        let mut progress = NoopProgress;
        let climate = derive_current_climate(
            &physical,
            ClimateSettings::default_for(grid),
            physical.seed,
            physical.retry_index,
            &mut progress,
        )
        .unwrap();
        for row in 0..grid.height {
            assert_eq!(
                climate.precipitation_mm_per_year[grid.index(row, 0)],
                climate.precipitation_mm_per_year[grid.index(row, grid.width - 1)]
            );
        }
    }

    #[test]
    fn ridge_creates_windward_precipitation_and_leeward_shadow() {
        let grid = Grid::new(16, 8, DEFAULT_RADIUS_METRES).unwrap();
        let mut elevations = vec![-2_000; grid.sample_count()];
        let row = 3;
        for col in 1..15 {
            elevations[grid.index(row, col)] = 500;
        }
        elevations[grid.index(row, 8)] = 4_500;
        let physical = field(grid, elevations, 0);
        let mut progress = NoopProgress;
        let climate = derive_current_climate(
            &physical,
            ClimateSettings::default_for(grid),
            physical.seed,
            physical.retry_index,
            &mut progress,
        )
        .unwrap();
        let windward = climate.precipitation_mm_per_year[grid.index(row, 8)];
        let leeward_precipitation = climate.precipitation_mm_per_year[grid.index(row, 7)];
        let leeward = climate.moisture_mm_per_year[grid.index(row, 7)];
        let upwind = climate.moisture_mm_per_year[grid.index(row, 9)];
        assert!(windward > leeward_precipitation);
        assert!(upwind > leeward, "upwind={upwind} leeward={leeward}");
        assert!(climate.metrics.transport_iterations <= CLIMATE_MAX_TRANSPORT_ITERATIONS);
    }

    #[test]
    fn coastal_moisture_drives_interior_drying() {
        let grid = Grid::new(16, 8, DEFAULT_RADIUS_METRES).unwrap();
        let mut elevations = vec![-2_000; grid.sample_count()];
        let row = 3;
        for col in 1..grid.width {
            elevations[grid.index(row, col)] = 500;
        }
        let physical = field(grid, elevations, 0);
        let mut progress = NoopProgress;
        let climate = derive_current_climate(
            &physical,
            ClimateSettings::default_for(grid),
            physical.seed,
            physical.retry_index,
            &mut progress,
        )
        .unwrap();
        let coastal = climate.moisture_mm_per_year[grid.index(row, grid.width - 1)];
        let interior = climate.moisture_mm_per_year[grid.index(row, 8)];
        assert!(coastal > interior, "coastal={coastal} interior={interior}");
    }

    #[test]
    fn hydrology_presets_change_coherent_water_response() {
        let grid = Grid::new(16, 8, DEFAULT_RADIUS_METRES).unwrap();
        let mut elevations = vec![500; grid.sample_count()];
        elevations[0] = -2_000;
        let physical = field(grid, elevations, 0);
        let mut arid_settings = ClimateSettings::default_for(grid);
        arid_settings.hydrology_preset = HydrologyPreset::Arid;
        let mut wet_settings = ClimateSettings::default_for(grid);
        wet_settings.hydrology_preset = HydrologyPreset::Wet;
        let mut progress = NoopProgress;
        let arid = derive_current_climate(
            &physical,
            arid_settings,
            physical.seed,
            physical.retry_index,
            &mut progress,
        )
        .unwrap();
        let mut progress = NoopProgress;
        let wet = derive_current_climate(
            &physical,
            wet_settings,
            physical.seed,
            physical.retry_index,
            &mut progress,
        )
        .unwrap();
        assert!(
            wet.metrics.precipitation_volume_m3_per_year
                > arid.metrics.precipitation_volume_m3_per_year
        );
        assert!(wet.metrics.runoff_volume_m3_per_year > arid.metrics.runoff_volume_m3_per_year);
    }

    #[test]
    fn climate_is_deterministic_and_runoff_is_land_only() {
        let grid = Grid::new(16, 8, DEFAULT_RADIUS_METRES).unwrap();
        let mut elevations = vec![500; grid.sample_count()];
        elevations[0] = -2_000;
        let physical = field(grid, elevations, 0);
        let mut progress = NoopProgress;
        let first = derive_current_climate(
            &physical,
            ClimateSettings::default_for(grid),
            physical.seed,
            physical.retry_index,
            &mut progress,
        )
        .unwrap();
        let mut progress = NoopProgress;
        let second = derive_current_climate(
            &physical,
            ClimateSettings::default_for(grid),
            physical.seed,
            physical.retry_index,
            &mut progress,
        )
        .unwrap();
        assert_eq!(first, second);
        assert_eq!(first.runoff_mm_per_year[0], 0);
        assert_eq!(first.runoff_volume_m3_per_year[0], 0);
        assert!(first.runoff_mm_per_year[1..].iter().any(|value| *value > 0));
        assert!(first.runoff_volume_m3_per_year[1..]
            .iter()
            .any(|volume| *volume > 0));
        assert!(
            first.metrics.precipitation_volume_m3_per_year
                >= first.metrics.runoff_volume_m3_per_year
        );
        let mut invalid = first.clone();
        invalid.runoff_volume_m3_per_year[1] += 1;
        assert!(invalid.validate_against(&physical).is_err());
    }

    #[test]
    fn nearest_ocean_tree_matches_brute_force_distance() {
        let grid = Grid::new(64, 32, DEFAULT_RADIUS_METRES).unwrap();
        let mut elevations = vec![1_000; grid.sample_count()];
        for (cell, elevation) in elevations.iter_mut().enumerate() {
            if cell.is_multiple_of(7) {
                *elevation = -500;
            }
        }
        let ocean_vectors = elevations
            .iter()
            .enumerate()
            .filter(|(_, elevation)| **elevation <= 0)
            .map(|(cell, _)| cell_geometry(grid, cell).1)
            .collect::<Vec<_>>();
        assert!(ocean_vectors.len() > 48);
        let tree = KdNode::build(&ocean_vectors);
        for (cell, elevation) in elevations.iter().enumerate() {
            if *elevation <= 0 {
                continue;
            }
            let vector = cell_geometry(grid, cell).1;
            let brute = ocean_vectors
                .iter()
                .map(|ocean| ocean.dot(vector).clamp(-1.0, 1.0).acos())
                .fold(f64::INFINITY, f64::min);
            let accelerated = nearest_ocean_distance(&ocean_vectors, tree.as_deref(), vector);
            assert!(
                (brute - accelerated).abs() < 1e-12,
                "cell {cell}: brute={brute} accelerated={accelerated}"
            );
        }
    }
}
