//! Deterministic current-climate and runoff derivation.
//!
//! Climate is a disposable interpretation of the accepted physical field. It
//! never changes the canonical elevation/source bytes. The model is purposely
//! bounded: a solar-driven latitude/altitude temperature field with two
//! solstice states, an explicit rotation-and-temperature wind field, a
//! wind-driven surface-ocean current field, moisture transport driven by those
//! winds with sea-surface and current evaporation, humidity and aridity from
//! remaining moisture versus local saturation and evaporative demand, and
//! precipitation that feeds runoff volumes using exact spherical cell areas.

use super::{
    derive_subsystem_seed, splitmix64, Grid, PhysicalError, PhysicalErrorCode, PhysicalField,
    ProgressPhase, ProgressSink, SeedDomain,
};
use crate::planetary::{
    PlanetaryConfiguration, EARTH_AXIAL_TILT_MILLI_DEG, EARTH_BOND_ALBEDO_PPM,
    EARTH_ECCENTRICITY_PPM, EARTH_RETAINED_HEAT_CENTI_C, EARTH_ROTATION_PERIOD_SECONDS,
    SOLAR_LUMINOSITY_PPM,
};

pub const CLIMATE_DERIVATION_VERSION: u16 = 4;
const EARTH_EQUATOR_BASE_CENTI_C: i32 = 1_400;
pub const CLIMATE_WIND_BAND_COUNT: u32 = 6;
pub const WIND_BAND_HADLEY: u32 = 0;
pub const WIND_BAND_FERREL: u32 = 1;
pub const WIND_BAND_POLAR: u32 = 2;
pub const MAX_WIND_MILLI: i32 = 10_000;
pub const MAX_CURRENT_MILLI: i32 = 4_000;
const WIND_GRADIENT_REF_METRES: f64 = 1_000_000.0;
const CURRENT_WIND_COUPLING: f64 = 0.30;
const CURRENT_GEOSTROPHY: f64 = 0.42;
const CURRENT_SVERDRUP: f64 = 0.22;
const MIN_CURRENT_BASIN_CELLS: usize = 8;
const CURRENT_WESTERN_RETURN: f64 = 2.35;
const CURRENT_WESTERN_SCALE: f64 = 2.1;
const CURRENT_SMOOTH_PASSES: usize = 2;
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
    pub planetary: PlanetaryConfiguration,
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
            planetary: PlanetaryConfiguration::earth_like(),
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
        self.planetary.validate()?;
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
    pub mean_seasonal_range_centi_c: u32,
    pub minimum_seasonal_temperature_centi_c: i32,
    pub maximum_seasonal_temperature_centi_c: i32,
    pub permanently_frozen_land_ppm: u32,
    pub seasonally_frozen_land_ppm: u32,
    pub mean_wind_speed_milli: u32,
    pub itcz_latitude_milli_deg: i32,
    pub easterly_cell_ppm: u32,
    pub converging_cell_ppm: u32,
    pub mean_current_speed_milli: u32,
    pub mean_humidity_ppm: u32,
    pub mean_land_aridity_ppm: u32,
    pub mean_seasonal_precipitation_range_mm: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClimateField {
    pub grid: Grid,
    pub derivation_version: u16,
    pub planetary: PlanetaryConfiguration,
    pub temperature_centi_c: Vec<i32>,
    pub temperature_nh_summer_centi_c: Vec<i32>,
    pub temperature_nh_winter_centi_c: Vec<i32>,
    pub moisture_mm_per_year: Vec<u32>,
    pub precipitation_mm_per_year: Vec<u32>,
    pub runoff_mm_per_year: Vec<u32>,
    pub runoff_volume_m3_per_year: Vec<u64>,
    pub maritime_factor_ppm: Vec<u32>,
    pub wind_east_milli: Vec<i32>,
    pub wind_north_milli: Vec<i32>,
    pub wind_east_nh_summer_milli: Vec<i32>,
    pub wind_north_nh_summer_milli: Vec<i32>,
    pub wind_east_nh_winter_milli: Vec<i32>,
    pub wind_north_nh_winter_milli: Vec<i32>,
    pub wind_divergence_ppm: Vec<i32>,
    pub wind_divergence_nh_summer_ppm: Vec<i32>,
    pub wind_divergence_nh_winter_ppm: Vec<i32>,
    pub wind_band: Vec<u32>,
    pub wind_band_nh_summer: Vec<u32>,
    pub wind_band_nh_winter: Vec<u32>,
    pub current_east_milli: Vec<i32>,
    pub current_north_milli: Vec<i32>,
    pub humidity_ppm: Vec<u32>,
    pub aridity_ppm: Vec<u32>,
    pub precipitation_nh_summer_mm: Vec<u32>,
    pub precipitation_nh_winter_mm: Vec<u32>,
    pub metrics: ClimateMetrics,
}

impl ClimateField {
    pub fn validate(&self) -> Result<(), PhysicalError> {
        let expected = self.grid.sample_count();
        if self.derivation_version != CLIMATE_DERIVATION_VERSION
            || self.temperature_centi_c.len() != expected
            || self.temperature_nh_summer_centi_c.len() != expected
            || self.temperature_nh_winter_centi_c.len() != expected
            || self.moisture_mm_per_year.len() != expected
            || self.precipitation_mm_per_year.len() != expected
            || self.runoff_mm_per_year.len() != expected
            || self.runoff_volume_m3_per_year.len() != expected
            || self.maritime_factor_ppm.len() != expected
            || self.wind_east_milli.len() != expected
            || self.wind_north_milli.len() != expected
            || self.wind_east_nh_summer_milli.len() != expected
            || self.wind_north_nh_summer_milli.len() != expected
            || self.wind_east_nh_winter_milli.len() != expected
            || self.wind_north_nh_winter_milli.len() != expected
            || self.wind_divergence_ppm.len() != expected
            || self.wind_divergence_nh_summer_ppm.len() != expected
            || self.wind_divergence_nh_winter_ppm.len() != expected
            || self.wind_band.len() != expected
            || self.wind_band_nh_summer.len() != expected
            || self.wind_band_nh_winter.len() != expected
            || self.current_east_milli.len() != expected
            || self.current_north_milli.len() != expected
            || self.humidity_ppm.len() != expected
            || self.aridity_ppm.len() != expected
            || self.precipitation_nh_summer_mm.len() != expected
            || self.precipitation_nh_winter_mm.len() != expected
        {
            return Err(PhysicalError::coded(
                PhysicalErrorCode::NumericNonFinite,
                "climate field shape or derivation version is invalid",
            ));
        }
        if self
            .temperature_centi_c
            .iter()
            .chain(self.temperature_nh_summer_centi_c.iter())
            .chain(self.temperature_nh_winter_centi_c.iter())
            .any(|value| {
                !(-MAX_CLIMATE_TEMPERATURE_CENTI_C..=MAX_CLIMATE_TEMPERATURE_CENTI_C)
                    .contains(value)
            })
            || self
                .moisture_mm_per_year
                .iter()
                .chain(self.precipitation_mm_per_year.iter())
                .chain(self.runoff_mm_per_year.iter())
                .any(|value| *value > MAX_CLIMATE_PRECIPITATION_MM)
            || self
                .maritime_factor_ppm
                .iter()
                .any(|value| *value > 1_000_000)
            || self
                .wind_east_milli
                .iter()
                .chain(self.wind_north_milli.iter())
                .chain(self.wind_east_nh_summer_milli.iter())
                .chain(self.wind_north_nh_summer_milli.iter())
                .chain(self.wind_east_nh_winter_milli.iter())
                .chain(self.wind_north_nh_winter_milli.iter())
                .any(|value| !(-MAX_WIND_MILLI..=MAX_WIND_MILLI).contains(value))
            || self
                .wind_divergence_ppm
                .iter()
                .chain(self.wind_divergence_nh_summer_ppm.iter())
                .chain(self.wind_divergence_nh_winter_ppm.iter())
                .any(|value| !(-1_000_000..=1_000_000).contains(value))
            || self
                .wind_band
                .iter()
                .chain(self.wind_band_nh_summer.iter())
                .chain(self.wind_band_nh_winter.iter())
                .any(|value| *value > WIND_BAND_POLAR)
            || self
                .current_east_milli
                .iter()
                .chain(self.current_north_milli.iter())
                .any(|value| !(-MAX_CURRENT_MILLI..=MAX_CURRENT_MILLI).contains(value))
            || self
                .humidity_ppm
                .iter()
                .chain(self.aridity_ppm.iter())
                .any(|value| *value > 1_000_000)
            || self
                .precipitation_nh_summer_mm
                .iter()
                .chain(self.precipitation_nh_winter_mm.iter())
                .any(|value| *value > MAX_CLIMATE_PRECIPITATION_MM)
        {
            return Err(PhysicalError::coded(
                PhysicalErrorCode::NumericNonFinite,
                "climate field contains a non-finite or unbounded value",
            ));
        }
        Ok(())
    }

    pub fn with_global_temperature_offset(&self, offset_centi_c: i32) -> Self {
        if offset_centi_c == 0 {
            return self.clone();
        }
        let mut shifted = self.clone();
        for temperatures in [
            &mut shifted.temperature_centi_c,
            &mut shifted.temperature_nh_summer_centi_c,
            &mut shifted.temperature_nh_winter_centi_c,
        ] {
            for temperature in temperatures {
                *temperature = temperature.saturating_add(offset_centi_c).clamp(
                    -MAX_CLIMATE_TEMPERATURE_CENTI_C,
                    MAX_CLIMATE_TEMPERATURE_CENTI_C,
                );
            }
        }
        shifted.metrics.mean_temperature_centi_c = shifted
            .metrics
            .mean_temperature_centi_c
            .saturating_add(offset_centi_c)
            .clamp(
                -MAX_CLIMATE_TEMPERATURE_CENTI_C,
                MAX_CLIMATE_TEMPERATURE_CENTI_C,
            );
        shifted.metrics.minimum_temperature_centi_c = shifted
            .temperature_centi_c
            .iter()
            .copied()
            .min()
            .unwrap_or(shifted.metrics.minimum_temperature_centi_c);
        shifted.metrics.maximum_temperature_centi_c = shifted
            .temperature_centi_c
            .iter()
            .copied()
            .max()
            .unwrap_or(shifted.metrics.maximum_temperature_centi_c);
        shifted
    }

    pub fn with_winds_and_moisture_for_field(
        &self,
        field: &PhysicalField,
        seed: u32,
        retry_index: u32,
        progress: &mut dyn ProgressSink,
    ) -> Result<Self, PhysicalError> {
        field.validate().map_err(PhysicalError::InvalidSource)?;
        self.validate()?;
        if self.grid != field.grid {
            return Err(PhysicalError::coded(
                PhysicalErrorCode::GeometryInvalid,
                "climate field grid does not match the physical field",
            ));
        }
        let mut next = self.clone();
        let winds = derive_winds(
            field,
            self.planetary,
            &next.temperature_centi_c,
            &next.temperature_nh_summer_centi_c,
            &next.temperature_nh_winter_centi_c,
            seed,
            retry_index,
        );
        next.wind_east_milli = winds.east;
        next.wind_north_milli = winds.north;
        next.wind_east_nh_summer_milli = winds.east_summer;
        next.wind_north_nh_summer_milli = winds.north_summer;
        next.wind_east_nh_winter_milli = winds.east_winter;
        next.wind_north_nh_winter_milli = winds.north_winter;
        next.wind_divergence_ppm = winds.divergence_ppm;
        next.wind_divergence_nh_summer_ppm = winds.divergence_summer_ppm;
        next.wind_divergence_nh_winter_ppm = winds.divergence_winter_ppm;
        next.wind_band = winds.band;
        next.wind_band_nh_summer = winds.band_summer;
        next.wind_band_nh_winter = winds.band_winter;
        let mut settings = ClimateSettings::default_for(field.grid);
        settings.planetary = self.planetary;
        settings.validate()?;
        let climate_seed = derive_subsystem_seed(seed, retry_index, SeedDomain::Climate);
        stamp_currents(&mut next, field);
        let moisture = derive_moisture_fields(field, settings, climate_seed, &next, progress)?;
        let (runoff, runoff_volume, mut metrics) = runoff_fields(
            field,
            settings,
            &next.temperature_centi_c,
            &moisture.precipitation,
            progress,
        )?;
        metrics.transport_iterations = moisture.iterations;
        apply_seasonal_metrics(
            field,
            &next.temperature_nh_summer_centi_c,
            &next.temperature_nh_winter_centi_c,
            &mut metrics,
        );
        apply_wind_metrics(
            field,
            &next.wind_east_milli,
            &next.wind_north_milli,
            &next.wind_divergence_ppm,
            winds.itcz_latitude,
            &mut metrics,
        );
        apply_current_metrics(
            field,
            &next.current_east_milli,
            &next.current_north_milli,
            &mut metrics,
        );
        apply_humidity_metrics(
            field,
            &moisture.humidity_ppm,
            &moisture.aridity_ppm,
            &moisture.precipitation_summer,
            &moisture.precipitation_winter,
            &mut metrics,
        );
        next.moisture_mm_per_year = moisture.moisture;
        next.precipitation_mm_per_year = moisture.precipitation;
        next.precipitation_nh_summer_mm = moisture.precipitation_summer;
        next.precipitation_nh_winter_mm = moisture.precipitation_winter;
        next.humidity_ppm = moisture.humidity_ppm;
        next.aridity_ppm = moisture.aridity_ppm;
        next.runoff_mm_per_year = runoff;
        next.runoff_volume_m3_per_year = runoff_volume;
        next.metrics = metrics;
        next.validate_against(field)?;
        Ok(next)
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
        let mid = indices.len() / 2;
        let (left_indices, median, right_indices) =
            indices.select_nth_unstable_by(mid, |first, second| {
                coord(points[*first], axis)
                    .partial_cmp(&coord(points[*second], axis))
                    .unwrap_or(std::cmp::Ordering::Equal)
                    .then_with(|| first.cmp(second))
            });
        let index = *median;
        Some(Box::new(Self {
            index,
            axis,
            left: Self::build_from_indices(points, left_indices, depth + 1),
            right: Self::build_from_indices(points, right_indices, depth + 1),
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

fn solar_base_centi_c(settings: ClimateSettings) -> Result<f64, PhysicalError> {
    let planetary = settings.planetary;
    let insolation = f64::from(planetary.insolation_ppm()?);
    let eccentricity = f64::from(planetary.eccentricity_ppm) / 1_000_000.0;
    let mean_factor = 1.0 / (1.0 - eccentricity * eccentricity).sqrt();
    let absorbed =
        insolation * mean_factor * f64::from(1_000_000 - planetary.bond_albedo_ppm) / 1_000_000.0;
    let earth_eccentricity = f64::from(EARTH_ECCENTRICITY_PPM) / 1_000_000.0;
    let earth_absorbed = f64::from(SOLAR_LUMINOSITY_PPM)
        / (1.0 - earth_eccentricity * earth_eccentricity).sqrt()
        * f64::from(1_000_000 - EARTH_BOND_ALBEDO_PPM)
        / 1_000_000.0;
    let scale = (absorbed / earth_absorbed).powf(0.25);
    if !scale.is_finite() {
        return Err(PhysicalError::coded(
            PhysicalErrorCode::NumericNonFinite,
            "solar temperature scale is not finite",
        ));
    }
    Ok(f64::from(EARTH_EQUATOR_BASE_CENTI_C) * scale
        + f64::from(planetary.retained_heat_centi_c - EARTH_RETAINED_HEAT_CENTI_C)
        + f64::from(settings.global_temperature_centi_c - EARTH_EQUATOR_BASE_CENTI_C))
}

fn seasonal_amplitude_centi_c(
    latitude: f64,
    maritime_factor: f64,
    planetary: PlanetaryConfiguration,
) -> f64 {
    let tilt_scale =
        f64::from(planetary.axial_tilt_milli_deg) / f64::from(EARTH_AXIAL_TILT_MILLI_DEG);
    let latitude_term = (latitude.abs() / std::f64::consts::FRAC_PI_2)
        .clamp(0.0, 1.0)
        .powf(0.9);
    let continentality = (1.0 - maritime_factor).clamp(0.0, 1.0);
    // Earth-like mid-latitude continental swing is about 22 °C at 23.44° tilt.
    // Oceans keep 28% of that swing; interiors keep the rest.
    tilt_scale * 2_200.0 * latitude_term * (0.28 + 0.72 * continentality)
}

fn orbital_season_centi_c(maritime_factor: f64, planetary: PlanetaryConfiguration) -> f64 {
    let eccentricity = f64::from(planetary.eccentricity_ppm) / 1_000_000.0;
    // Perihelion–aphelion contrast ≈ (T/4)·2e/(1−e²); Earth is about 2 °C.
    let contrast = 12_000.0 * eccentricity / (1.0 - eccentricity * eccentricity).max(0.2);
    let continentality = (1.0 - maritime_factor).clamp(0.0, 1.0);
    contrast * (0.55 + 0.45 * continentality)
}

fn clamp_temperature(value: f64) -> Result<i32, PhysicalError> {
    if !value.is_finite() {
        return Err(PhysicalError::coded(
            PhysicalErrorCode::NumericNonFinite,
            "climate temperature is not finite",
        ));
    }
    Ok(value.round().clamp(
        -f64::from(MAX_CLIMATE_TEMPERATURE_CENTI_C),
        f64::from(MAX_CLIMATE_TEMPERATURE_CENTI_C),
    ) as i32)
}

fn temperature_field(
    field: &PhysicalField,
    settings: ClimateSettings,
    geometry: &[CellClimateGeometry],
    progress: &mut dyn ProgressSink,
) -> Result<(Vec<i32>, Vec<i32>, Vec<i32>, Vec<u32>), PhysicalError> {
    let base = solar_base_centi_c(settings)?;
    let mut temperatures = Vec::with_capacity(field.grid.sample_count());
    let mut summers = Vec::with_capacity(field.grid.sample_count());
    let mut winters = Vec::with_capacity(field.grid.sample_count());
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
        let continental_temperature = base - latitude_cooling - altitude_cooling;
        let maritime_temperature = base - latitude_cooling * 0.58 - altitude_cooling * 0.62
            + f64::from(settings.maritime_moderation_centi_c) * 0.18;
        let temperature = continental_temperature
            + (maritime_temperature - continental_temperature) * cell_geometry.maritime_factor;
        let tilt_amplitude = seasonal_amplitude_centi_c(
            cell_geometry.latitude,
            cell_geometry.maritime_factor,
            settings.planetary,
        );
        let orbit_amplitude =
            orbital_season_centi_c(cell_geometry.maritime_factor, settings.planetary);
        let hemisphere = if cell_geometry.latitude >= 0.0 {
            1.0
        } else {
            -1.0
        };
        // Northern-summer solstice also carries the warmer-orbit half of
        // eccentricity; perihelion longitude is not authored.
        let summer = temperature + tilt_amplitude * hemisphere + orbit_amplitude;
        let winter = temperature - tilt_amplitude * hemisphere - orbit_amplitude;
        temperatures.push(clamp_temperature(temperature)?);
        summers.push(clamp_temperature(summer)?);
        winters.push(clamp_temperature(winter)?);
        maritime_factors.push((cell_geometry.maritime_factor * 1_000_000.0).round() as u32);
    }
    Ok((temperatures, summers, winters, maritime_factors))
}

fn clamp_wind(value: f64) -> i32 {
    value
        .round()
        .clamp(-f64::from(MAX_WIND_MILLI), f64::from(MAX_WIND_MILLI)) as i32
}

fn clamp_current(value: f64) -> i32 {
    value
        .round()
        .clamp(-f64::from(MAX_CURRENT_MILLI), f64::from(MAX_CURRENT_MILLI)) as i32
}

fn omega_ratio(planetary: PlanetaryConfiguration) -> f64 {
    f64::from(EARTH_ROTATION_PERIOD_SECONDS) / f64::from(planetary.rotation_period_seconds.max(1))
}

fn hadley_edge_radians(omega: f64) -> f64 {
    (30.0 / omega.max(0.04).sqrt())
        .clamp(12.0, 80.0)
        .to_radians()
}

fn ferrel_edge_radians(hadley: f64) -> f64 {
    let pole = std::f64::consts::FRAC_PI_2;
    (hadley + (pole - hadley) * 0.55).clamp(hadley + 0.08, pole - 0.04)
}

fn thermal_equator_latitude(grid: Grid, temperatures: &[i32]) -> f64 {
    let mut best = i64::MIN;
    let mut latitude = 0.0;
    for row in 0..grid.height {
        let mut sum = 0i64;
        for col in 0..grid.width {
            sum += i64::from(temperatures[grid.index(row, col)]);
        }
        if sum > best {
            best = sum;
            latitude = grid.center_radians(row, 0).1;
        }
    }
    latitude
}

fn circulation_at(latitude: f64, itcz: f64, hadley: f64, ferrel: f64) -> (u32, f64, f64) {
    let phi = latitude - itcz;
    let abs_phi = phi.abs();
    let toward_itcz = if phi >= 0.0 { -1.0 } else { 1.0 };
    if abs_phi <= hadley {
        let gap = (abs_phi / hadley.max(1e-6)).clamp(0.0, 1.0);
        (WIND_BAND_HADLEY, -1.0, toward_itcz * gap)
    } else if abs_phi <= ferrel {
        (WIND_BAND_FERREL, 1.0, -toward_itcz)
    } else {
        (WIND_BAND_POLAR, -1.0, toward_itcz)
    }
}

fn smoothstep(edge0: f64, edge1: f64, x: f64) -> f64 {
    if edge1 <= edge0 {
        return if x >= edge1 { 1.0 } else { 0.0 };
    }
    let t = ((x - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

fn circulation_flow(latitude: f64, itcz: f64, hadley: f64, ferrel: f64) -> (f64, f64) {
    let phi = latitude - itcz;
    let abs_phi = phi.abs();
    let toward_itcz = if phi >= 0.0 { -1.0 } else { 1.0 };
    let width = 0.16;
    let hadley_w = 1.0 - smoothstep(hadley - width, hadley + width, abs_phi);
    let polar_w = smoothstep(ferrel - width, ferrel + width, abs_phi);
    let ferrel_w = (1.0 - hadley_w - polar_w).max(0.0);
    let gap = (abs_phi / hadley.max(1e-6)).clamp(0.0, 1.0);
    (
        -hadley_w + ferrel_w - polar_w,
        hadley_w * toward_itcz * gap + ferrel_w * (-toward_itcz) + polar_w * toward_itcz,
    )
}

fn wrapped_col(grid: Grid, col: u32, delta: i32) -> u32 {
    (col as i32 + delta).rem_euclid(grid.width as i32) as u32
}

fn clamped_row(grid: Grid, row: u32, delta: i32) -> u32 {
    (row as i32 + delta).clamp(0, grid.height as i32 - 1) as u32
}

fn neighbor_metres(grid: Grid, row: u32, col: u32, drow: i32, dcol: i32) -> f64 {
    let other_row = clamped_row(grid, row, drow);
    let other_col = wrapped_col(grid, col, dcol);
    grid.great_circle_distance(
        grid.center_radians(row, col),
        grid.center_radians(other_row, other_col),
    )
    .max(1.0)
}

fn spacing_scale(metres: f64) -> f64 {
    (WIND_GRADIENT_REF_METRES / metres.max(1.0)).clamp(0.25, 2.5)
}

fn is_ocean(field: &PhysicalField, cell: usize) -> bool {
    field.elevations_mm[cell] <= field.sea_level_mm
}

fn ocean_mask(field: &PhysicalField) -> Vec<bool> {
    let count = field.grid.sample_count();
    let wet: Vec<bool> = (0..count)
        .map(|cell| field.elevations_mm[cell] <= field.sea_level_mm)
        .collect();
    let mut seen = vec![false; count];
    let mut components = Vec::new();
    for start in 0..count {
        if !wet[start] || seen[start] {
            continue;
        }
        let mut stack = vec![start];
        let mut component = Vec::new();
        seen[start] = true;
        while let Some(cell) = stack.pop() {
            component.push(cell);
            let (row, col) = field.grid.row_col(cell);
            let neighbors = [
                field.grid.index(row, wrapped_col(field.grid, col, -1)),
                field.grid.index(row, wrapped_col(field.grid, col, 1)),
                field.grid.index(clamped_row(field.grid, row, -1), col),
                field.grid.index(clamped_row(field.grid, row, 1), col),
            ];
            for neighbor in neighbors {
                if wet[neighbor] && !seen[neighbor] {
                    seen[neighbor] = true;
                    stack.push(neighbor);
                }
            }
        }
        components.push(component);
    }
    let mut ocean = vec![false; count];
    for component in components {
        if component.len() >= MIN_CURRENT_BASIN_CELLS {
            for cell in component {
                ocean[cell] = true;
            }
        }
    }
    ocean
}

fn ocean_temperature(temperatures: &[i32], ocean: &[bool], cell: usize, fallback: i32) -> i32 {
    if ocean[cell] {
        temperatures[cell]
    } else {
        fallback
    }
}

fn wind_components(
    field: &PhysicalField,
    temperatures: &[i32],
    planetary: PlanetaryConfiguration,
    itcz: f64,
    hadley: f64,
    ferrel: f64,
    wind_seed: u64,
) -> (Vec<i32>, Vec<i32>) {
    let omega = omega_ratio(planetary);
    let zonal_scale = 1_050.0 * omega.clamp(0.15, 2.8).powf(0.45);
    let meridional_scale = 520.0 / omega.clamp(0.25, 2.5).powf(0.35);
    let waves = (2.0 + 3.2 * omega.clamp(0.2, 2.4)).clamp(2.0, 8.0);
    let phase = (wind_seed as f64) * (std::f64::consts::TAU / (u64::MAX as f64));
    let wave_amp = 340.0 * omega.clamp(0.3, 2.0).sqrt();
    let mut east = Vec::with_capacity(field.grid.sample_count());
    let mut north = Vec::with_capacity(field.grid.sample_count());
    for cell in 0..field.grid.sample_count() {
        let (row, col) = field.grid.row_col(cell);
        let (longitude, latitude) = field.grid.center_radians(row, col);
        let (zonal_sign, meridional_sign) = circulation_flow(latitude, itcz, hadley, ferrel);
        let west_cell = field.grid.index(row, wrapped_col(field.grid, col, -1));
        let east_cell = field.grid.index(row, wrapped_col(field.grid, col, 1));
        let south_cell = field.grid.index(clamped_row(field.grid, row, -1), col);
        let north_cell = field.grid.index(clamped_row(field.grid, row, 1), col);
        let dx = neighbor_metres(field.grid, row, col, 0, 1);
        let dy = neighbor_metres(field.grid, row, col, 1, 0);
        let scale_x = spacing_scale(dx);
        let scale_y = spacing_scale(dy);
        let mut u = zonal_sign * zonal_scale;
        let mut v = meridional_sign * meridional_scale;
        let envelope = zonal_sign.max(0.0) + (-zonal_sign).max(0.0) * 0.22;
        let theta = waves * longitude + phase;
        u += envelope * wave_amp * theta.sin();
        v += envelope * wave_amp * 1.2 * theta.cos();
        let theta2 = (waves * 0.5 + 1.0) * longitude + phase * 1.73;
        u += envelope * wave_amp * 0.38 * theta2.cos();
        v += envelope * wave_amp * 0.45 * theta2.sin();
        u += 0.72 * f64::from(temperatures[east_cell] - temperatures[west_cell]) * scale_x;
        v += 0.72 * f64::from(temperatures[north_cell] - temperatures[south_cell]) * scale_y;
        if is_ocean(field, east_cell) != is_ocean(field, cell) {
            u += 0.48 * f64::from(temperatures[east_cell] - temperatures[cell]) * scale_x;
        }
        if is_ocean(field, west_cell) != is_ocean(field, cell) {
            u += 0.48 * f64::from(temperatures[cell] - temperatures[west_cell]) * scale_x;
        }
        if is_ocean(field, north_cell) != is_ocean(field, cell) {
            v += 0.48 * f64::from(temperatures[north_cell] - temperatures[cell]) * scale_y;
        }
        if is_ocean(field, south_cell) != is_ocean(field, cell) {
            v += 0.48 * f64::from(temperatures[cell] - temperatures[south_cell]) * scale_y;
        }
        let elevation_km =
            (f64::from(field.elevations_mm[cell] - field.sea_level_mm) / 1_000_000.0).max(0.0);
        let blocking = 1.0 / (1.0 + elevation_km * 0.6);
        let roughness = if is_ocean(field, cell) { 1.0 } else { 0.86 };
        u *= blocking * roughness;
        v *= roughness;
        v += -0.000_4
            * f64::from(field.elevations_mm[east_cell] - field.elevations_mm[west_cell])
            * blocking
            * scale_x;
        east.push(clamp_wind(u));
        north.push(clamp_wind(v));
    }
    (east, north)
}

fn wind_divergence_ppm(grid: Grid, east: &[i32], north: &[i32]) -> Vec<i32> {
    let mut divergence = Vec::with_capacity(grid.sample_count());
    for cell in 0..grid.sample_count() {
        let (row, col) = grid.row_col(cell);
        let west = grid.index(row, wrapped_col(grid, col, -1));
        let east_cell = grid.index(row, wrapped_col(grid, col, 1));
        let south = grid.index(clamped_row(grid, row, -1), col);
        let north_cell = grid.index(clamped_row(grid, row, 1), col);
        let scale_x = spacing_scale(neighbor_metres(grid, row, col, 0, 1));
        let scale_y = spacing_scale(neighbor_metres(grid, row, col, 1, 0));
        let value = f64::from(east[east_cell] - east[west]) * scale_x
            + f64::from(north[north_cell] - north[south]) * scale_y;
        divergence.push((value * 80.0).round().clamp(-1_000_000.0, 1_000_000.0) as i32);
    }
    divergence
}

fn wind_band_field(grid: Grid, itcz: f64, hadley: f64, ferrel: f64) -> Vec<u32> {
    (0..grid.sample_count())
        .map(|cell| {
            let (row, col) = grid.row_col(cell);
            circulation_at(grid.center_radians(row, col).1, itcz, hadley, ferrel).0
        })
        .collect()
}

struct DerivedWinds {
    east: Vec<i32>,
    north: Vec<i32>,
    east_summer: Vec<i32>,
    north_summer: Vec<i32>,
    east_winter: Vec<i32>,
    north_winter: Vec<i32>,
    divergence_ppm: Vec<i32>,
    divergence_summer_ppm: Vec<i32>,
    divergence_winter_ppm: Vec<i32>,
    band: Vec<u32>,
    band_summer: Vec<u32>,
    band_winter: Vec<u32>,
    itcz_latitude: f64,
}

fn derive_winds(
    field: &PhysicalField,
    planetary: PlanetaryConfiguration,
    annual: &[i32],
    summer: &[i32],
    winter: &[i32],
    seed: u32,
    retry_index: u32,
) -> DerivedWinds {
    let omega = omega_ratio(planetary);
    let hadley = hadley_edge_radians(omega);
    let ferrel = ferrel_edge_radians(hadley);
    let itcz = thermal_equator_latitude(field.grid, annual);
    let summer_itcz = thermal_equator_latitude(field.grid, summer);
    let winter_itcz = thermal_equator_latitude(field.grid, winter);
    let wind_seed = derive_subsystem_seed(seed, retry_index, SeedDomain::Climate);
    let (east, north) = wind_components(field, annual, planetary, itcz, hadley, ferrel, wind_seed);
    let (east_summer, north_summer) = wind_components(
        field,
        summer,
        planetary,
        summer_itcz,
        hadley,
        ferrel,
        wind_seed ^ 0x9e37_79b9_7f4a_7c15,
    );
    let (east_winter, north_winter) = wind_components(
        field,
        winter,
        planetary,
        winter_itcz,
        hadley,
        ferrel,
        wind_seed ^ 0xbf58_476d_1ce4_e5b9,
    );
    let divergence_ppm = wind_divergence_ppm(field.grid, &east, &north);
    let divergence_summer_ppm = wind_divergence_ppm(field.grid, &east_summer, &north_summer);
    let divergence_winter_ppm = wind_divergence_ppm(field.grid, &east_winter, &north_winter);
    let band = wind_band_field(field.grid, itcz, hadley, ferrel);
    let band_summer = wind_band_field(field.grid, summer_itcz, hadley, ferrel);
    let band_winter = wind_band_field(field.grid, winter_itcz, hadley, ferrel);
    DerivedWinds {
        east,
        north,
        east_summer,
        north_summer,
        east_winter,
        north_winter,
        divergence_ppm,
        divergence_summer_ppm,
        divergence_winter_ppm,
        band,
        band_summer,
        band_winter,
        itcz_latitude: itcz,
    }
}

fn stamp_currents(climate: &mut ClimateField, field: &PhysicalField) {
    let (east, north) = derive_currents(
        field,
        climate.planetary,
        &climate.temperature_centi_c,
        &climate.wind_east_milli,
        &climate.wind_north_milli,
    );
    climate.current_east_milli = east;
    climate.current_north_milli = north;
    apply_current_metrics(
        field,
        &climate.current_east_milli,
        &climate.current_north_milli,
        &mut climate.metrics,
    );
}

fn steps_along_row(field: &PhysicalField, ocean: &[bool], col_delta: i32) -> Vec<u32> {
    let mut distance = vec![0u32; field.grid.sample_count()];
    for cell in 0..field.grid.sample_count() {
        if !ocean[cell] {
            continue;
        }
        let (row, col) = field.grid.row_col(cell);
        let mut steps = 0u32;
        loop {
            steps += 1;
            if steps >= field.grid.width {
                break;
            }
            let other = field
                .grid
                .index(row, wrapped_col(field.grid, col, col_delta * steps as i32));
            if !ocean[other] {
                break;
            }
        }
        distance[cell] = steps.saturating_sub(1);
    }
    distance
}

fn derive_currents(
    field: &PhysicalField,
    planetary: PlanetaryConfiguration,
    temperatures: &[i32],
    wind_east: &[i32],
    wind_north: &[i32],
) -> (Vec<i32>, Vec<i32>) {
    let omega = omega_ratio(planetary);
    let count = field.grid.sample_count();
    let ocean = ocean_mask(field);
    let mut east = vec![0.0; count];
    let mut north = vec![0.0; count];
    let west_distance = steps_along_row(field, &ocean, -1);
    let east_distance = steps_along_row(field, &ocean, 1);
    let wind_scale = CURRENT_WIND_COUPLING * omega.clamp(0.2, 2.2).sqrt();
    for cell in 0..count {
        if !ocean[cell] {
            continue;
        }
        let (row, col) = field.grid.row_col(cell);
        let latitude = field.grid.center_radians(row, col).1;
        let coriolis = (latitude.sin() * omega).clamp(-2.4, 2.4);
        let turn = 1.08 * coriolis.tanh();
        let (sin, cos) = turn.sin_cos();
        let wind_u = f64::from(wind_east[cell]);
        let wind_v = f64::from(wind_north[cell]);
        let mut u = wind_scale * (wind_u * cos + wind_v * sin);
        let mut v = wind_scale * (-wind_u * sin + wind_v * cos);
        let west_cell = field.grid.index(row, wrapped_col(field.grid, col, -1));
        let east_cell = field.grid.index(row, wrapped_col(field.grid, col, 1));
        let south_cell = field.grid.index(clamped_row(field.grid, row, -1), col);
        let north_cell = field.grid.index(clamped_row(field.grid, row, 1), col);
        let scale_x = spacing_scale(neighbor_metres(field.grid, row, col, 0, 1));
        let scale_y = spacing_scale(neighbor_metres(field.grid, row, col, 1, 0));
        let here = temperatures[cell];
        let dtx = f64::from(
            ocean_temperature(temperatures, &ocean, east_cell, here)
                - ocean_temperature(temperatures, &ocean, west_cell, here),
        ) * scale_x;
        let dty = f64::from(
            ocean_temperature(temperatures, &ocean, north_cell, here)
                - ocean_temperature(temperatures, &ocean, south_cell, here),
        ) * scale_y;
        let geostrophy = CURRENT_GEOSTROPHY * (2.2 * coriolis.abs()).clamp(0.0, 1.0);
        let sign = if coriolis == 0.0 {
            0.0
        } else {
            coriolis.signum()
        };
        u += geostrophy * (-dty) * sign;
        v += geostrophy * dtx * sign;
        let d_u_dy = f64::from(wind_east[north_cell] - wind_east[south_cell]) * scale_y;
        let wind_curl = -d_u_dy;
        let beta = latitude.cos().abs().max(0.2);
        let sverdrup = CURRENT_SVERDRUP * omega.clamp(0.2, 2.2).sqrt() * wind_curl / beta;
        v += sverdrup;
        let dist_west = west_distance[cell];
        let dist_east = east_distance[cell];
        if dist_west <= dist_east && dist_west + dist_east + 1 < field.grid.width {
            let boost = 1.0
                + CURRENT_WESTERN_RETURN * (-f64::from(dist_west) / CURRENT_WESTERN_SCALE).exp();
            v -= sverdrup * boost;
            u *= 0.62 + 0.38 / boost;
        }
        east[cell] = u;
        north[cell] = v;
    }
    for _ in 0..CURRENT_SMOOTH_PASSES {
        let previous_east = east.clone();
        let previous_north = north.clone();
        for cell in 0..count {
            if !ocean[cell] {
                continue;
            }
            let (row, col) = field.grid.row_col(cell);
            let neighbors = [
                field.grid.index(row, wrapped_col(field.grid, col, -1)),
                field.grid.index(row, wrapped_col(field.grid, col, 1)),
                field.grid.index(clamped_row(field.grid, row, -1), col),
                field.grid.index(clamped_row(field.grid, row, 1), col),
            ];
            let mut sum_u = previous_east[cell] * 2.0;
            let mut sum_v = previous_north[cell] * 2.0;
            let mut weight = 2.0;
            for neighbor in neighbors {
                if ocean[neighbor] {
                    sum_u += previous_east[neighbor];
                    sum_v += previous_north[neighbor];
                    weight += 1.0;
                }
            }
            east[cell] = sum_u / weight;
            north[cell] = sum_v / weight;
        }
    }
    for cell in 0..count {
        if !ocean[cell] {
            east[cell] = 0.0;
            north[cell] = 0.0;
            continue;
        }
        let (row, col) = field.grid.row_col(cell);
        let west_cell = field.grid.index(row, wrapped_col(field.grid, col, -1));
        let east_cell = field.grid.index(row, wrapped_col(field.grid, col, 1));
        let south_cell = field.grid.index(clamped_row(field.grid, row, -1), col);
        let north_cell = field.grid.index(clamped_row(field.grid, row, 1), col);
        if !ocean[east_cell] {
            east[cell] = east[cell].min(0.0);
        }
        if !ocean[west_cell] {
            east[cell] = east[cell].max(0.0);
        }
        if !ocean[north_cell] {
            north[cell] = north[cell].min(0.0);
        }
        if !ocean[south_cell] {
            north[cell] = north[cell].max(0.0);
        }
    }
    (
        east.into_iter().map(clamp_current).collect(),
        north.into_iter().map(clamp_current).collect(),
    )
}

fn hydrology_parameters(preset: HydrologyPreset) -> (f64, f64, f64, f64) {
    match preset {
        HydrologyPreset::Arid => (0.75, 0.90, 0.24, 0.18),
        HydrologyPreset::Balanced => (1.0, 1.0, 0.40, 0.28),
        HydrologyPreset::Wet => (1.15, 1.015, 0.56, 0.34),
    }
}

fn band_source_multiplier(seed: u64, band: u32, southern: bool) -> f64 {
    let token = band.saturating_add(if southern { CLIMATE_WIND_BAND_COUNT } else { 0 });
    let random = splitmix64(seed ^ u64::from(token).wrapping_mul(0x9e37_79b9_7f4a_7c15));
    0.90 + f64::from((random % 200_001) as u32) / 1_000_000.0
}

struct MoistureBundle {
    moisture: Vec<u32>,
    precipitation: Vec<u32>,
    precipitation_summer: Vec<u32>,
    precipitation_winter: Vec<u32>,
    humidity_ppm: Vec<u32>,
    aridity_ppm: Vec<u32>,
    iterations: u32,
}

fn saturation_moisture_mm(temperature_centi_c: i32) -> f64 {
    let t = f64::from(temperature_centi_c) / 100.0;
    let es = 6.112 * (17.67 * t / (t + 243.5)).exp();
    (180.0 * es).clamp(80.0, f64::from(MAX_CLIMATE_MOISTURE_MM))
}

fn potential_evapotranspiration_mm(temperature_centi_c: i32) -> f64 {
    let t = f64::from(temperature_centi_c) / 100.0;
    if t <= 0.0 {
        40.0
    } else {
        (40.0 + 70.0 * t).clamp(40.0, 4_000.0)
    }
}

fn ocean_evaporation_mm(
    settings: ClimateSettings,
    source_multiplier: f64,
    source_factor: f64,
    temperature_centi_c: i32,
    current_east: i32,
    current_north: i32,
) -> f64 {
    let frozen = if temperature_centi_c < 0 { 0.18 } else { 1.0 };
    let sst = (1.0 + f64::from(temperature_centi_c - 1_400) / 4_500.0).clamp(0.35, 1.75);
    let speed =
        f64::from(current_east).hypot(f64::from(current_north)) / f64::from(MAX_CURRENT_MILLI);
    let current = 1.0 + 0.20 * speed.clamp(0.0, 1.0);
    f64::from(settings.ocean_moisture_mm_per_year)
        * source_multiplier
        * source_factor
        * frozen
        * sst
        * current
}

fn humidity_and_aridity(
    temperatures: &[i32],
    moisture: &[u32],
    precipitation: &[u32],
) -> (Vec<u32>, Vec<u32>) {
    let humidity = moisture
        .iter()
        .zip(temperatures)
        .map(|(value, temperature)| {
            let sat = saturation_moisture_mm(*temperature).max(1.0);
            ((f64::from(*value) / sat) * 1_000_000.0)
                .round()
                .clamp(0.0, 1_000_000.0) as u32
        })
        .collect::<Vec<_>>();
    let aridity = precipitation
        .iter()
        .zip(temperatures)
        .map(|(value, temperature)| {
            let pet = potential_evapotranspiration_mm(*temperature).max(1.0);
            let wetness = (f64::from(*value) / pet).clamp(0.0, 1.0);
            ((1.0 - wetness) * 1_000_000.0)
                .round()
                .clamp(0.0, 1_000_000.0) as u32
        })
        .collect::<Vec<_>>();
    (humidity, aridity)
}

fn derive_moisture_fields(
    field: &PhysicalField,
    settings: ClimateSettings,
    climate_seed: u64,
    climate: &ClimateField,
    progress: &mut dyn ProgressSink,
) -> Result<MoistureBundle, PhysicalError> {
    let (moisture, precipitation, it_annual) = transport_moisture(
        field,
        settings,
        climate_seed,
        &climate.temperature_centi_c,
        &climate.current_east_milli,
        &climate.current_north_milli,
        &climate.wind_east_milli,
        &climate.wind_north_milli,
        &climate.wind_divergence_ppm,
        &climate.wind_band,
        progress,
    )?;
    let summer_matches_annual = climate.temperature_nh_summer_centi_c
        == climate.temperature_centi_c
        && climate.wind_east_nh_summer_milli == climate.wind_east_milli
        && climate.wind_north_nh_summer_milli == climate.wind_north_milli
        && climate.wind_divergence_nh_summer_ppm == climate.wind_divergence_ppm
        && climate.wind_band_nh_summer == climate.wind_band;
    let winter_matches_annual = climate.temperature_nh_winter_centi_c
        == climate.temperature_centi_c
        && climate.wind_east_nh_winter_milli == climate.wind_east_milli
        && climate.wind_north_nh_winter_milli == climate.wind_north_milli
        && climate.wind_divergence_nh_winter_ppm == climate.wind_divergence_ppm
        && climate.wind_band_nh_winter == climate.wind_band;
    let (precipitation_summer, it_summer) = if summer_matches_annual {
        (precipitation.clone(), it_annual)
    } else {
        let (_, precip, iterations) = transport_moisture(
            field,
            settings,
            climate_seed,
            &climate.temperature_nh_summer_centi_c,
            &climate.current_east_milli,
            &climate.current_north_milli,
            &climate.wind_east_nh_summer_milli,
            &climate.wind_north_nh_summer_milli,
            &climate.wind_divergence_nh_summer_ppm,
            &climate.wind_band_nh_summer,
            progress,
        )?;
        (precip, iterations)
    };
    let (precipitation_winter, it_winter) = if winter_matches_annual {
        (precipitation.clone(), it_annual)
    } else {
        let (_, precip, iterations) = transport_moisture(
            field,
            settings,
            climate_seed,
            &climate.temperature_nh_winter_centi_c,
            &climate.current_east_milli,
            &climate.current_north_milli,
            &climate.wind_east_nh_winter_milli,
            &climate.wind_north_nh_winter_milli,
            &climate.wind_divergence_nh_winter_ppm,
            &climate.wind_band_nh_winter,
            progress,
        )?;
        (precip, iterations)
    };
    let (humidity_ppm, aridity_ppm) =
        humidity_and_aridity(&climate.temperature_centi_c, &moisture, &precipitation);
    Ok(MoistureBundle {
        moisture,
        precipitation,
        precipitation_summer,
        precipitation_winter,
        humidity_ppm,
        aridity_ppm,
        iterations: it_annual.max(it_summer).max(it_winter),
    })
}

fn transport_moisture(
    field: &PhysicalField,
    settings: ClimateSettings,
    climate_seed: u64,
    temperatures: &[i32],
    current_east: &[i32],
    current_north: &[i32],
    wind_east: &[i32],
    wind_north: &[i32],
    wind_divergence_ppm: &[i32],
    wind_band: &[u32],
    progress: &mut dyn ProgressSink,
) -> Result<(Vec<u32>, Vec<u32>, u32), PhysicalError> {
    let (source_multiplier, decay_multiplier, _, _) =
        hydrology_parameters(settings.hydrology_preset);
    let decay_at_scale = f64::from(settings.moisture_decay_ppm) / 1_000_000.0 * decay_multiplier;
    let convergence = f64::from(settings.convergence_ppm) / 1_000_000.0;
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
    let meridional_step_metres = (0..field.grid.height)
        .map(|row| {
            let neighbor = if row + 1 < field.grid.height {
                row + 1
            } else {
                row.saturating_sub(1)
            };
            field
                .grid
                .great_circle_distance(
                    field.grid.center_radians(row, 0),
                    field.grid.center_radians(neighbor, 0),
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
            let source_factor = band_source_multiplier(
                climate_seed,
                wind_band[field.grid.index(row, 0)],
                row < field.grid.height / 2,
            );
            let distance = row_step_metres[row as usize];
            let physical_decay =
                (-distance / (f64::from(settings.moisture_decay_scale_km) * 1_000.0)).exp();
            let upstream_weight = (decay_at_scale * physical_decay).clamp(0.0, 0.995);
            for col in 0..field.grid.width {
                let cell = field.grid.index(row, col);
                let east = f64::from(wind_east[cell]);
                let north = f64::from(wind_north[cell]);
                let axis_sum = east.abs() + north.abs();
                let zonal_frac = if axis_sum <= 0.0 {
                    0.0
                } else {
                    east.abs() / axis_sum
                };
                let meridional_frac = if axis_sum <= 0.0 {
                    0.0
                } else {
                    1.0 - zonal_frac
                };
                let upstream_col = wrapped_col(field.grid, col, if east >= 0.0 { -1 } else { 1 });
                let upstream_row = clamped_row(field.grid, row, if north >= 0.0 { -1 } else { 1 });
                let zonal_upstream = field.grid.index(row, upstream_col);
                let meridional_upstream = field.grid.index(upstream_row, col);
                let ocean_source = if field.elevations_mm[cell] <= field.sea_level_mm {
                    ocean_evaporation_mm(
                        settings,
                        source_multiplier,
                        source_factor,
                        temperatures[cell],
                        current_east[cell],
                        current_north[cell],
                    )
                } else {
                    0.0
                };
                let incoming = (ocean_source
                    + previous[zonal_upstream] * upstream_weight * zonal_frac
                    + previous[meridional_upstream] * upstream_weight * meridional_frac)
                    .clamp(0.0, f64::from(MAX_CLIMATE_MOISTURE_MM));
                let zonal_uphill =
                    (f64::from(field.elevations_mm[cell] - field.elevations_mm[zonal_upstream])
                        / 1_000.0)
                        .max(0.0)
                        / distance;
                let meridional_distance = meridional_step_metres[row as usize];
                let meridional_uphill = (f64::from(
                    field.elevations_mm[cell] - field.elevations_mm[meridional_upstream],
                ) / 1_000.0)
                    .max(0.0)
                    / meridional_distance;
                let slope = zonal_frac * zonal_uphill + meridional_frac * meridional_uphill;
                let orographic =
                    slope * f64::from(settings.orographic_precipitation_ppm) / 1_000_000.0;
                let convergence_boost = if wind_divergence_ppm[cell] < 0 {
                    convergence
                        * (-f64::from(wind_divergence_ppm[cell]) / 1_000_000.0).clamp(0.0, 1.0)
                } else {
                    0.0
                };
                let precipitation_fraction =
                    (base_precipitation + orographic + convergence_boost).clamp(0.0, 0.65);
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
            mean_seasonal_range_centi_c: 0,
            minimum_seasonal_temperature_centi_c: 0,
            maximum_seasonal_temperature_centi_c: 0,
            permanently_frozen_land_ppm: 0,
            seasonally_frozen_land_ppm: 0,
            mean_wind_speed_milli: 0,
            itcz_latitude_milli_deg: 0,
            easterly_cell_ppm: 0,
            converging_cell_ppm: 0,
            mean_current_speed_milli: 0,
            mean_humidity_ppm: 0,
            mean_land_aridity_ppm: 0,
            mean_seasonal_precipitation_range_mm: 0,
        },
    ))
}

fn apply_seasonal_metrics(
    field: &PhysicalField,
    summers: &[i32],
    winters: &[i32],
    metrics: &mut ClimateMetrics,
) {
    let total_area = field.grid.total_area();
    let mut range_area_sum = 0.0;
    let mut land_area = 0.0;
    let mut permanent_area = 0.0;
    let mut seasonal_area = 0.0;
    let mut minimum_seasonal = i32::MAX;
    let mut maximum_seasonal = i32::MIN;
    for cell in 0..field.grid.sample_count() {
        let area = field.grid.cell_area(field.grid.row_col(cell).0);
        let cold = summers[cell].min(winters[cell]);
        let warm = summers[cell].max(winters[cell]);
        range_area_sum += f64::from((summers[cell] - winters[cell]).unsigned_abs()) * area;
        minimum_seasonal = minimum_seasonal.min(cold);
        maximum_seasonal = maximum_seasonal.max(warm);
        if field.elevations_mm[cell] > field.sea_level_mm {
            land_area += area;
            if warm < 0 {
                permanent_area += area;
            } else if cold < 0 {
                seasonal_area += area;
            }
        }
    }
    metrics.mean_seasonal_range_centi_c = (range_area_sum / total_area).round() as u32;
    metrics.minimum_seasonal_temperature_centi_c = minimum_seasonal;
    metrics.maximum_seasonal_temperature_centi_c = maximum_seasonal;
    metrics.permanently_frozen_land_ppm = if land_area <= 0.0 {
        0
    } else {
        (permanent_area / land_area * 1_000_000.0).round() as u32
    };
    metrics.seasonally_frozen_land_ppm = if land_area <= 0.0 {
        0
    } else {
        (seasonal_area / land_area * 1_000_000.0).round() as u32
    };
}

fn apply_wind_metrics(
    field: &PhysicalField,
    east: &[i32],
    north: &[i32],
    divergence_ppm: &[i32],
    itcz_latitude: f64,
    metrics: &mut ClimateMetrics,
) {
    let total_area = field.grid.total_area();
    let mut speed_area_sum = 0.0;
    let mut easterly_area = 0.0;
    let mut converging_area = 0.0;
    for cell in 0..field.grid.sample_count() {
        let area = field.grid.cell_area(field.grid.row_col(cell).0);
        speed_area_sum += f64::from(east[cell]).hypot(f64::from(north[cell])) * area;
        if east[cell] < 0 {
            easterly_area += area;
        }
        if divergence_ppm[cell] < 0 {
            converging_area += area;
        }
    }
    metrics.mean_wind_speed_milli = (speed_area_sum / total_area).round() as u32;
    metrics.itcz_latitude_milli_deg = (itcz_latitude.to_degrees() * 1_000.0).round() as i32;
    metrics.easterly_cell_ppm = (easterly_area / total_area * 1_000_000.0).round() as u32;
    metrics.converging_cell_ppm = (converging_area / total_area * 1_000_000.0).round() as u32;
}

fn apply_humidity_metrics(
    field: &PhysicalField,
    humidity_ppm: &[u32],
    aridity_ppm: &[u32],
    precipitation_summer: &[u32],
    precipitation_winter: &[u32],
    metrics: &mut ClimateMetrics,
) {
    let total_area = field.grid.total_area();
    let mut humidity_area_sum = 0.0;
    let mut aridity_area_sum = 0.0;
    let mut land_area = 0.0;
    let mut range_area_sum = 0.0;
    for cell in 0..field.grid.sample_count() {
        let area = field.grid.cell_area(field.grid.row_col(cell).0);
        humidity_area_sum += f64::from(humidity_ppm[cell]) * area;
        range_area_sum +=
            f64::from(precipitation_summer[cell].abs_diff(precipitation_winter[cell])) * area;
        if field.elevations_mm[cell] > field.sea_level_mm {
            land_area += area;
            aridity_area_sum += f64::from(aridity_ppm[cell]) * area;
        }
    }
    metrics.mean_humidity_ppm = (humidity_area_sum / total_area).round() as u32;
    metrics.mean_land_aridity_ppm = if land_area <= 0.0 {
        0
    } else {
        (aridity_area_sum / land_area).round() as u32
    };
    metrics.mean_seasonal_precipitation_range_mm = (range_area_sum / total_area).round() as u32;
}

fn apply_current_metrics(
    field: &PhysicalField,
    east: &[i32],
    north: &[i32],
    metrics: &mut ClimateMetrics,
) {
    let ocean = ocean_mask(field);
    let mut speed_area_sum = 0.0;
    let mut ocean_area = 0.0;
    for cell in 0..field.grid.sample_count() {
        if !ocean[cell] {
            continue;
        }
        let area = field.grid.cell_area(field.grid.row_col(cell).0);
        ocean_area += area;
        speed_area_sum += f64::from(east[cell]).hypot(f64::from(north[cell])) * area;
    }
    metrics.mean_current_speed_milli = if ocean_area <= 0.0 {
        0
    } else {
        (speed_area_sum / ocean_area).round() as u32
    };
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
    let (temperatures, summers, winters, maritime_factors) =
        temperature_field(field, settings, &geometry, progress)?;
    let winds = derive_winds(
        field,
        settings.planetary,
        &temperatures,
        &summers,
        &winters,
        seed,
        retry_index,
    );
    progress.report(ProgressPhase::CalculatingClimate, 2, 4)?;
    let climate_seed = derive_subsystem_seed(seed, retry_index, SeedDomain::Climate);
    let (current_east, current_north) = derive_currents(
        field,
        settings.planetary,
        &temperatures,
        &winds.east,
        &winds.north,
    );
    let mut climate = ClimateField {
        grid: field.grid,
        derivation_version: CLIMATE_DERIVATION_VERSION,
        planetary: settings.planetary,
        temperature_centi_c: temperatures,
        temperature_nh_summer_centi_c: summers,
        temperature_nh_winter_centi_c: winters,
        moisture_mm_per_year: Vec::new(),
        precipitation_mm_per_year: Vec::new(),
        runoff_mm_per_year: Vec::new(),
        runoff_volume_m3_per_year: Vec::new(),
        maritime_factor_ppm: maritime_factors,
        wind_east_milli: winds.east,
        wind_north_milli: winds.north,
        wind_east_nh_summer_milli: winds.east_summer,
        wind_north_nh_summer_milli: winds.north_summer,
        wind_east_nh_winter_milli: winds.east_winter,
        wind_north_nh_winter_milli: winds.north_winter,
        wind_divergence_ppm: winds.divergence_ppm,
        wind_divergence_nh_summer_ppm: winds.divergence_summer_ppm,
        wind_divergence_nh_winter_ppm: winds.divergence_winter_ppm,
        wind_band: winds.band,
        wind_band_nh_summer: winds.band_summer,
        wind_band_nh_winter: winds.band_winter,
        current_east_milli: current_east,
        current_north_milli: current_north,
        humidity_ppm: Vec::new(),
        aridity_ppm: Vec::new(),
        precipitation_nh_summer_mm: Vec::new(),
        precipitation_nh_winter_mm: Vec::new(),
        metrics: ClimateMetrics {
            precipitation_volume_m3_per_year: 0,
            runoff_volume_m3_per_year: 0,
            mean_temperature_centi_c: 0,
            minimum_temperature_centi_c: 0,
            maximum_temperature_centi_c: 0,
            mean_precipitation_mm_per_year: 0,
            mean_runoff_mm_per_year: 0,
            wettest_cell_precipitation_mm_per_year: 0,
            driest_land_cell_precipitation_mm_per_year: 0,
            transport_iterations: 0,
            mean_seasonal_range_centi_c: 0,
            minimum_seasonal_temperature_centi_c: 0,
            maximum_seasonal_temperature_centi_c: 0,
            permanently_frozen_land_ppm: 0,
            seasonally_frozen_land_ppm: 0,
            mean_wind_speed_milli: 0,
            itcz_latitude_milli_deg: 0,
            easterly_cell_ppm: 0,
            converging_cell_ppm: 0,
            mean_current_speed_milli: 0,
            mean_humidity_ppm: 0,
            mean_land_aridity_ppm: 0,
            mean_seasonal_precipitation_range_mm: 0,
        },
    };
    let moisture = derive_moisture_fields(field, settings, climate_seed, &climate, progress)?;
    let (runoff, runoff_volume, mut metrics) = runoff_fields(
        field,
        settings,
        &climate.temperature_centi_c,
        &moisture.precipitation,
        progress,
    )?;
    metrics.transport_iterations = moisture.iterations;
    apply_seasonal_metrics(
        field,
        &climate.temperature_nh_summer_centi_c,
        &climate.temperature_nh_winter_centi_c,
        &mut metrics,
    );
    apply_wind_metrics(
        field,
        &climate.wind_east_milli,
        &climate.wind_north_milli,
        &climate.wind_divergence_ppm,
        winds.itcz_latitude,
        &mut metrics,
    );
    apply_current_metrics(
        field,
        &climate.current_east_milli,
        &climate.current_north_milli,
        &mut metrics,
    );
    apply_humidity_metrics(
        field,
        &moisture.humidity_ppm,
        &moisture.aridity_ppm,
        &moisture.precipitation_summer,
        &moisture.precipitation_winter,
        &mut metrics,
    );
    progress.report(ProgressPhase::CalculatingClimate, 3, 4)?;
    climate.moisture_mm_per_year = moisture.moisture;
    climate.precipitation_mm_per_year = moisture.precipitation;
    climate.precipitation_nh_summer_mm = moisture.precipitation_summer;
    climate.precipitation_nh_winter_mm = moisture.precipitation_winter;
    climate.humidity_ppm = moisture.humidity_ppm;
    climate.aridity_ppm = moisture.aridity_ppm;
    climate.runoff_mm_per_year = runoff;
    climate.runoff_volume_m3_per_year = runoff_volume;
    climate.metrics = metrics;
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
            let west = climate.precipitation_mm_per_year[grid.index(row, 0)] as i32;
            let east = climate.precipitation_mm_per_year[grid.index(row, grid.width - 1)] as i32;
            let interior = climate.precipitation_mm_per_year[grid.index(row, 1)] as i32;
            let wrap_gap = (west - east).unsigned_abs();
            let neighbor_gap = (west - interior).unsigned_abs();
            assert!(
                wrap_gap <= neighbor_gap.saturating_add(80),
                "antimeridian seam at row {row}: wrap {wrap_gap} vs neighbor {neighbor_gap}"
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
        elevations[grid.index(row, 8)] = 2_000_000;
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
        let ridge = grid.index(row, 8);
        let from_west = climate.wind_east_milli[ridge] >= 0;
        let upwind_col = if from_west { 7 } else { 9 };
        let leeward_col = if from_west { 9 } else { 7 };
        let windward = climate.precipitation_mm_per_year[ridge];
        let leeward_precipitation = climate.precipitation_mm_per_year[grid.index(row, leeward_col)];
        let leeward = climate.moisture_mm_per_year[grid.index(row, leeward_col)];
        let upwind = climate.moisture_mm_per_year[grid.index(row, upwind_col)];
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
        assert!(
            climate.aridity_ppm[grid.index(row, 8)]
                > climate.aridity_ppm[grid.index(row, grid.width - 1)]
        );
    }

    fn mean_u32(values: &[u32]) -> u64 {
        values.iter().map(|value| u64::from(*value)).sum::<u64>() / values.len() as u64
    }

    #[test]
    fn warmer_sea_surface_increases_ocean_moisture() {
        let grid = Grid::new(16, 8, DEFAULT_RADIUS_METRES).unwrap();
        let elevations = vec![-2_000; grid.sample_count()];
        let physical = field(grid, elevations, 0);
        let climate = derive_current_climate(
            &physical,
            ClimateSettings::default_for(grid),
            physical.seed,
            physical.retry_index,
            &mut NoopProgress,
        )
        .unwrap();
        let zeros = vec![0i32; grid.sample_count()];
        let cold = vec![-800; grid.sample_count()];
        let warm = vec![2_800; grid.sample_count()];
        let settings = ClimateSettings::default_for(grid);
        let seed = derive_subsystem_seed(physical.seed, physical.retry_index, SeedDomain::Climate);
        let (_, cold_precip, _) = transport_moisture(
            &physical,
            settings,
            seed,
            &cold,
            &zeros,
            &zeros,
            &climate.wind_east_milli,
            &climate.wind_north_milli,
            &climate.wind_divergence_ppm,
            &climate.wind_band,
            &mut NoopProgress,
        )
        .unwrap();
        let (_, warm_precip, _) = transport_moisture(
            &physical,
            settings,
            seed,
            &warm,
            &zeros,
            &zeros,
            &climate.wind_east_milli,
            &climate.wind_north_milli,
            &climate.wind_divergence_ppm,
            &climate.wind_band,
            &mut NoopProgress,
        )
        .unwrap();
        assert!(
            mean_u32(&warm_precip) > mean_u32(&cold_precip),
            "warm {} cold {}",
            mean_u32(&warm_precip),
            mean_u32(&cold_precip)
        );
    }

    #[test]
    fn surface_currents_increase_ocean_evaporation() {
        let grid = Grid::new(16, 8, DEFAULT_RADIUS_METRES).unwrap();
        let elevations = vec![-2_000; grid.sample_count()];
        let physical = field(grid, elevations, 0);
        let climate = derive_current_climate(
            &physical,
            ClimateSettings::default_for(grid),
            physical.seed,
            physical.retry_index,
            &mut NoopProgress,
        )
        .unwrap();
        let zeros = vec![0i32; grid.sample_count()];
        let settings = ClimateSettings::default_for(grid);
        let seed = derive_subsystem_seed(physical.seed, physical.retry_index, SeedDomain::Climate);
        let (_, still, _) = transport_moisture(
            &physical,
            settings,
            seed,
            &climate.temperature_centi_c,
            &zeros,
            &zeros,
            &climate.wind_east_milli,
            &climate.wind_north_milli,
            &climate.wind_divergence_ppm,
            &climate.wind_band,
            &mut NoopProgress,
        )
        .unwrap();
        let (_, moving, _) = transport_moisture(
            &physical,
            settings,
            seed,
            &climate.temperature_centi_c,
            &climate.current_east_milli,
            &climate.current_north_milli,
            &climate.wind_east_milli,
            &climate.wind_north_milli,
            &climate.wind_divergence_ppm,
            &climate.wind_band,
            &mut NoopProgress,
        )
        .unwrap();
        assert!(
            mean_u32(&moving) >= mean_u32(&still),
            "moving {} still {}",
            mean_u32(&moving),
            mean_u32(&still)
        );
        assert_ne!(moving, still);
    }

    #[test]
    fn seasonal_precipitation_differs_when_tilt_is_large() {
        let grid = Grid::new(16, 8, DEFAULT_RADIUS_METRES).unwrap();
        let mut elevations = vec![500; grid.sample_count()];
        elevations[0] = -2_000;
        let physical = field(grid, elevations, 0);
        let mut settings = ClimateSettings::default_for(grid);
        settings.planetary.axial_tilt_milli_deg = 35_000;
        let climate = derive_current_climate(
            &physical,
            settings,
            physical.seed,
            physical.retry_index,
            &mut NoopProgress,
        )
        .unwrap();
        assert_ne!(
            climate.precipitation_nh_summer_mm,
            climate.precipitation_nh_winter_mm
        );
        assert!(climate.metrics.mean_seasonal_precipitation_range_mm > 0);
        assert!(climate.metrics.mean_humidity_ppm > 0);
        assert!(climate.metrics.mean_humidity_ppm <= 1_000_000);
    }

    #[test]
    fn humidity_is_remaining_moisture_versus_saturation() {
        let grid = Grid::new(16, 8, DEFAULT_RADIUS_METRES).unwrap();
        let mut elevations = vec![-2_000; grid.sample_count()];
        let row = 3;
        for col in 1..grid.width {
            elevations[grid.index(row, col)] = 500;
        }
        let physical = field(grid, elevations, 0);
        let climate = derive_current_climate(
            &physical,
            ClimateSettings::default_for(grid),
            physical.seed,
            physical.retry_index,
            &mut NoopProgress,
        )
        .unwrap();
        let coastal = grid.index(row, grid.width - 1);
        let interior = grid.index(row, 8);
        assert!(climate.humidity_ppm[coastal] <= 1_000_000);
        assert!(
            climate.humidity_ppm[coastal] > climate.humidity_ppm[interior],
            "coastal humidity {} interior {}",
            climate.humidity_ppm[coastal],
            climate.humidity_ppm[interior]
        );
    }

    #[test]
    fn frozen_seas_evaporate_less_than_open_water() {
        let grid = Grid::new(16, 8, DEFAULT_RADIUS_METRES).unwrap();
        let elevations = vec![-2_000; grid.sample_count()];
        let physical = field(grid, elevations, 0);
        let climate = derive_current_climate(
            &physical,
            ClimateSettings::default_for(grid),
            physical.seed,
            physical.retry_index,
            &mut NoopProgress,
        )
        .unwrap();
        let zeros = vec![0i32; grid.sample_count()];
        let frozen = vec![-400; grid.sample_count()];
        let open = vec![200; grid.sample_count()];
        let settings = ClimateSettings::default_for(grid);
        let seed = derive_subsystem_seed(physical.seed, physical.retry_index, SeedDomain::Climate);
        let (_, frozen_precip, _) = transport_moisture(
            &physical,
            settings,
            seed,
            &frozen,
            &zeros,
            &zeros,
            &climate.wind_east_milli,
            &climate.wind_north_milli,
            &climate.wind_divergence_ppm,
            &climate.wind_band,
            &mut NoopProgress,
        )
        .unwrap();
        let (_, open_precip, _) = transport_moisture(
            &physical,
            settings,
            seed,
            &open,
            &zeros,
            &zeros,
            &climate.wind_east_milli,
            &climate.wind_north_milli,
            &climate.wind_divergence_ppm,
            &climate.wind_band,
            &mut NoopProgress,
        )
        .unwrap();
        assert!(
            mean_u32(&frozen_precip) < mean_u32(&open_precip),
            "frozen {} open {}",
            mean_u32(&frozen_precip),
            mean_u32(&open_precip)
        );
    }

    #[test]
    fn climate_derivation_does_not_modify_terrain() {
        let grid = Grid::new(16, 8, DEFAULT_RADIUS_METRES).unwrap();
        let mut elevations = vec![500; grid.sample_count()];
        elevations[0] = -2_000;
        let physical = field(grid, elevations.clone(), 0);
        let _ = derive_current_climate(
            &physical,
            ClimateSettings::default_for(grid),
            physical.seed,
            physical.retry_index,
            &mut NoopProgress,
        )
        .unwrap();
        assert_eq!(physical.elevations_mm, elevations);
        assert_eq!(physical.sea_level_mm, 0);
    }

    #[test]
    fn solstice_winds_change_seasonal_precipitation() {
        let grid = Grid::new(16, 8, DEFAULT_RADIUS_METRES).unwrap();
        let mut elevations = vec![500; grid.sample_count()];
        elevations[0] = -2_000;
        let physical = field(grid, elevations, 0);
        let climate = derive_current_climate(
            &physical,
            ClimateSettings::default_for(grid),
            physical.seed,
            physical.retry_index,
            &mut NoopProgress,
        )
        .unwrap();
        let settings = ClimateSettings::default_for(grid);
        let seed = derive_subsystem_seed(physical.seed, physical.retry_index, SeedDomain::Climate);
        let (_, from_summer_winds, _) = transport_moisture(
            &physical,
            settings,
            seed,
            &climate.temperature_centi_c,
            &climate.current_east_milli,
            &climate.current_north_milli,
            &climate.wind_east_nh_summer_milli,
            &climate.wind_north_nh_summer_milli,
            &climate.wind_divergence_nh_summer_ppm,
            &climate.wind_band_nh_summer,
            &mut NoopProgress,
        )
        .unwrap();
        let (_, from_winter_winds, _) = transport_moisture(
            &physical,
            settings,
            seed,
            &climate.temperature_centi_c,
            &climate.current_east_milli,
            &climate.current_north_milli,
            &climate.wind_east_nh_winter_milli,
            &climate.wind_north_nh_winter_milli,
            &climate.wind_divergence_nh_winter_ppm,
            &climate.wind_band_nh_winter,
            &mut NoopProgress,
        )
        .unwrap();
        assert_ne!(from_summer_winds, from_winter_winds);
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

    #[test]
    fn closer_orbit_warms_the_annual_field() {
        let grid = Grid::new(16, 8, DEFAULT_RADIUS_METRES).unwrap();
        let physical = field(grid, vec![0; grid.sample_count()], 1);
        let earth = derive_current_climate(
            &physical,
            ClimateSettings::default_for(grid),
            physical.seed,
            physical.retry_index,
            &mut NoopProgress,
        )
        .unwrap();
        let mut close = ClimateSettings::default_for(grid);
        close.planetary =
            PlanetaryConfiguration::from_preset(crate::planetary::PlanetaryPreset::CloseOrbit);
        let close_climate = derive_current_climate(
            &physical,
            close,
            physical.seed,
            physical.retry_index,
            &mut NoopProgress,
        )
        .unwrap();
        assert!(
            close_climate.metrics.mean_temperature_centi_c > earth.metrics.mean_temperature_centi_c
        );
    }

    #[test]
    fn axial_tilt_creates_solstice_contrast() {
        let grid = Grid::new(16, 8, DEFAULT_RADIUS_METRES).unwrap();
        let mut elevations = vec![500; grid.sample_count()];
        elevations[grid.index(3, 0)] = -2_000;
        let physical = field(grid, elevations, 0);
        let mut untilted = ClimateSettings::default_for(grid);
        untilted.planetary.axial_tilt_milli_deg = 0;
        untilted.planetary.eccentricity_ppm = 0;
        untilted.planetary.preset = crate::planetary::PlanetaryPreset::Custom;
        let none = derive_current_climate(
            &physical,
            untilted,
            physical.seed,
            physical.retry_index,
            &mut NoopProgress,
        )
        .unwrap();
        assert_eq!(
            none.temperature_nh_summer_centi_c,
            none.temperature_nh_winter_centi_c
        );
        assert_eq!(none.metrics.mean_seasonal_range_centi_c, 0);

        let mut tilted = ClimateSettings::default_for(grid);
        tilted.planetary =
            PlanetaryConfiguration::from_preset(crate::planetary::PlanetaryPreset::HighTilt);
        let seasons = derive_current_climate(
            &physical,
            tilted,
            physical.seed,
            physical.retry_index,
            &mut NoopProgress,
        )
        .unwrap();
        let north = grid.index(grid.height - 1, 0);
        let south = grid.index(0, 0);
        assert!(
            seasons.temperature_nh_summer_centi_c[north]
                > seasons.temperature_nh_winter_centi_c[north]
        );
        assert!(
            seasons.temperature_nh_summer_centi_c[south]
                < seasons.temperature_nh_winter_centi_c[south]
        );
        assert!(seasons.metrics.mean_seasonal_range_centi_c > 0);
    }

    #[test]
    fn eccentricity_creates_global_solstice_contrast_without_tilt() {
        let grid = Grid::new(16, 8, DEFAULT_RADIUS_METRES).unwrap();
        let mut elevations = vec![500; grid.sample_count()];
        elevations[0] = -2_000;
        let physical = field(grid, elevations, 0);
        let mut settings = ClimateSettings::default_for(grid);
        settings.planetary.axial_tilt_milli_deg = 0;
        settings.planetary.eccentricity_ppm = 400_000;
        settings.planetary.preset = crate::planetary::PlanetaryPreset::Custom;
        let climate = derive_current_climate(
            &physical,
            settings,
            physical.seed,
            physical.retry_index,
            &mut NoopProgress,
        )
        .unwrap();
        assert!(climate.metrics.mean_seasonal_range_centi_c > 0);
        assert!(
            climate.temperature_nh_summer_centi_c[grid.index(3, 4)]
                > climate.temperature_nh_winter_centi_c[grid.index(3, 4)]
        );
    }

    #[test]
    fn far_orbit_marks_permanent_freeze_on_land() {
        let grid = Grid::new(16, 8, DEFAULT_RADIUS_METRES).unwrap();
        let mut elevations = vec![500; grid.sample_count()];
        elevations[grid.index(3, 0)] = -2_000;
        let physical = field(grid, elevations, 0);
        let mut settings = ClimateSettings::default_for(grid);
        settings.planetary.semi_major_axis_milli_au = 5_000_000;
        settings.planetary.preset = crate::planetary::PlanetaryPreset::Custom;
        let climate = derive_current_climate(
            &physical,
            settings,
            physical.seed,
            physical.retry_index,
            &mut NoopProgress,
        )
        .unwrap();
        assert!(climate.metrics.permanently_frozen_land_ppm > 0);
        assert!(climate.metrics.minimum_seasonal_temperature_centi_c < 0);
    }

    #[test]
    fn earth_like_winds_have_tropical_easterlies_and_midlatitude_westerlies() {
        let grid = Grid::new(16, 8, DEFAULT_RADIUS_METRES).unwrap();
        let mut elevations = vec![500; grid.sample_count()];
        elevations[0] = -2_000;
        let physical = field(grid, elevations, 0);
        let climate = derive_current_climate(
            &physical,
            ClimateSettings::default_for(grid),
            physical.seed,
            physical.retry_index,
            &mut NoopProgress,
        )
        .unwrap();
        let tropics = grid.index(4, 4);
        let midlatitude = grid.index(5, 4);
        assert!(climate.wind_east_milli[tropics] < 0);
        assert!(climate.wind_east_milli[midlatitude] > 0);
        assert_eq!(climate.wind_band[tropics], WIND_BAND_HADLEY);
        assert_eq!(climate.wind_band[midlatitude], WIND_BAND_FERREL);
        assert!(climate.metrics.mean_wind_speed_milli > 0);
        assert!(climate.metrics.easterly_cell_ppm > 0);
        assert!(climate.metrics.easterly_cell_ppm < 1_000_000);
    }

    #[test]
    fn slow_rotation_expands_hadley_easterlies() {
        let grid = Grid::new(16, 8, DEFAULT_RADIUS_METRES).unwrap();
        let mut elevations = vec![500; grid.sample_count()];
        elevations[0] = -2_000;
        let physical = field(grid, elevations, 0);
        let earth = derive_current_climate(
            &physical,
            ClimateSettings::default_for(grid),
            physical.seed,
            physical.retry_index,
            &mut NoopProgress,
        )
        .unwrap();
        let mut slow = ClimateSettings::default_for(grid);
        slow.planetary =
            PlanetaryConfiguration::from_preset(crate::planetary::PlanetaryPreset::SlowRotating);
        let slow_climate = derive_current_climate(
            &physical,
            slow,
            physical.seed,
            physical.retry_index,
            &mut NoopProgress,
        )
        .unwrap();
        let midlatitude = grid.index(5, 4);
        assert!(earth.wind_east_milli[midlatitude] > 0);
        assert!(slow_climate.wind_east_milli[midlatitude] < 0);
        assert_eq!(slow_climate.wind_band[midlatitude], WIND_BAND_HADLEY);
    }

    #[test]
    fn northern_summer_shifts_itcz_north() {
        let grid = Grid::new(16, 8, DEFAULT_RADIUS_METRES).unwrap();
        let mut elevations = vec![500; grid.sample_count()];
        elevations[0] = -2_000;
        let physical = field(grid, elevations, 0);
        let mut tilted = ClimateSettings::default_for(grid);
        tilted.planetary =
            PlanetaryConfiguration::from_preset(crate::planetary::PlanetaryPreset::HighTilt);
        let climate = derive_current_climate(
            &physical,
            tilted,
            physical.seed,
            physical.retry_index,
            &mut NoopProgress,
        )
        .unwrap();
        let summer_itcz = thermal_equator_latitude(grid, &climate.temperature_nh_summer_centi_c);
        let winter_itcz = thermal_equator_latitude(grid, &climate.temperature_nh_winter_centi_c);
        assert!(summer_itcz > winter_itcz);
        let tropics = grid.index(4, 4);
        assert_ne!(
            climate.wind_north_nh_summer_milli[tropics],
            climate.wind_north_nh_winter_milli[tropics]
        );
        assert!(climate
            .wind_band_nh_summer
            .iter()
            .zip(climate.wind_band_nh_winter.iter())
            .any(|(summer, winter)| summer != winter));
        assert_ne!(
            climate.wind_divergence_nh_summer_ppm[tropics],
            climate.wind_divergence_nh_winter_ppm[tropics]
        );
    }

    #[test]
    fn mountains_block_zonal_wind() {
        let grid = Grid::new(16, 8, DEFAULT_RADIUS_METRES).unwrap();
        let mut elevations = vec![500; grid.sample_count()];
        elevations[0] = -2_000;
        let peak = grid.index(4, 8);
        elevations[peak] = 4_000_000;
        let physical = field(grid, elevations, 0);
        let climate = derive_current_climate(
            &physical,
            ClimateSettings::default_for(grid),
            physical.seed,
            physical.retry_index,
            &mut NoopProgress,
        )
        .unwrap();
        let neighbor = grid.index(4, 4);
        assert!(
            climate.wind_east_milli[peak].unsigned_abs()
                < climate.wind_east_milli[neighbor].unsigned_abs()
        );
    }

    #[test]
    fn uniform_temperature_offset_keeps_winds_until_sea_level_changes() {
        let grid = Grid::new(16, 8, DEFAULT_RADIUS_METRES).unwrap();
        let mut elevations = vec![500; grid.sample_count()];
        elevations[0] = -2_000;
        let physical = field(grid, elevations, 0);
        let climate = derive_current_climate(
            &physical,
            ClimateSettings::default_for(grid),
            physical.seed,
            physical.retry_index,
            &mut NoopProgress,
        )
        .unwrap();
        let shifted = climate
            .with_global_temperature_offset(-500)
            .with_winds_and_moisture_for_field(
                &physical,
                physical.seed,
                physical.retry_index,
                &mut NoopProgress,
            )
            .unwrap();
        assert_eq!(shifted.wind_east_milli, climate.wind_east_milli);
        assert_eq!(shifted.wind_north_milli, climate.wind_north_milli);
        let mut flooded = physical.clone();
        flooded.sea_level_mm = 2_000;
        let epoch = climate
            .with_winds_and_moisture_for_field(
                &flooded,
                physical.seed,
                physical.retry_index,
                &mut NoopProgress,
            )
            .unwrap();
        assert_ne!(epoch.wind_east_milli, climate.wind_east_milli);
        assert_ne!(
            epoch.precipitation_mm_per_year,
            climate.precipitation_mm_per_year
        );
        assert_ne!(epoch.current_east_milli, climate.current_east_milli);
        assert_ne!(epoch.current_north_milli, climate.current_north_milli);
    }

    #[test]
    fn ocean_temperature_gradient_changes_currents() {
        let grid = Grid::new(16, 8, DEFAULT_RADIUS_METRES).unwrap();
        let elevations = vec![-2_000; grid.sample_count()];
        let physical = field(grid, elevations, 0);
        let climate = derive_current_climate(
            &physical,
            ClimateSettings::default_for(grid),
            physical.seed,
            physical.retry_index,
            &mut NoopProgress,
        )
        .unwrap();
        let flat = vec![1_500; grid.sample_count()];
        let mut front = flat.clone();
        for cell in 0..grid.sample_count() {
            let (row, _) = grid.row_col(cell);
            front[cell] = if row + 1 > grid.height / 2 {
                4_000
            } else {
                -2_000
            };
        }
        let (front_east, _) = derive_currents(
            &physical,
            climate.planetary,
            &front,
            &climate.wind_east_milli,
            &climate.wind_north_milli,
        );
        let (flat_east, _) = derive_currents(
            &physical,
            climate.planetary,
            &flat,
            &climate.wind_east_milli,
            &climate.wind_north_milli,
        );
        let mean_abs_delta = front_east
            .iter()
            .zip(&flat_east)
            .map(|(a, b)| i64::from(*a - *b).unsigned_abs())
            .sum::<u64>()
            / u64::from(grid.sample_count() as u32);
        assert!(
            mean_abs_delta > 40,
            "geostrophy should move currents, mean abs delta {mean_abs_delta}"
        );
    }

    #[test]
    fn circulation_signs_hold_on_a_finer_grid() {
        let grid = Grid::new(32, 16, DEFAULT_RADIUS_METRES).unwrap();
        let mut elevations = vec![500; grid.sample_count()];
        elevations[0] = -2_000;
        let physical = field(grid, elevations, 0);
        let climate = derive_current_climate(
            &physical,
            ClimateSettings::default_for(grid),
            physical.seed,
            physical.retry_index,
            &mut NoopProgress,
        )
        .unwrap();
        let tropics = grid.index(8, 8);
        let midlatitude = grid.index(11, 8);
        assert!(climate.wind_east_milli[tropics] < 0);
        assert!(climate.wind_east_milli[midlatitude] > 0);
    }

    #[test]
    fn wind_meanders_depend_on_seed() {
        let grid = Grid::new(16, 8, DEFAULT_RADIUS_METRES).unwrap();
        let mut elevations = vec![500; grid.sample_count()];
        elevations[0] = -2_000;
        let first = field(grid, elevations.clone(), 0);
        let mut second = first.clone();
        second.seed = 42;
        let climate_first = derive_current_climate(
            &first,
            ClimateSettings::default_for(grid),
            first.seed,
            first.retry_index,
            &mut NoopProgress,
        )
        .unwrap();
        let climate_second = derive_current_climate(
            &second,
            ClimateSettings::default_for(grid),
            second.seed,
            second.retry_index,
            &mut NoopProgress,
        )
        .unwrap();
        assert_ne!(
            climate_first.wind_east_milli,
            climate_second.wind_east_milli
        );
        assert_ne!(
            climate_first.wind_north_milli,
            climate_second.wind_north_milli
        );
    }

    #[test]
    fn ocean_currents_are_zero_on_land() {
        let grid = Grid::new(16, 8, DEFAULT_RADIUS_METRES).unwrap();
        let mut elevations = vec![-2_000; grid.sample_count()];
        for row in 0..grid.height {
            elevations[grid.index(row, 5)] = 2_000;
        }
        let physical = field(grid, elevations, 0);
        let climate = derive_current_climate(
            &physical,
            ClimateSettings::default_for(grid),
            physical.seed,
            physical.retry_index,
            &mut NoopProgress,
        )
        .unwrap();
        for cell in 0..grid.sample_count() {
            if physical.elevations_mm[cell] > physical.sea_level_mm {
                assert_eq!(climate.current_east_milli[cell], 0);
                assert_eq!(climate.current_north_milli[cell], 0);
            }
        }
        assert!(climate.metrics.mean_current_speed_milli > 0);
    }

    #[test]
    fn inland_sinks_do_not_get_ocean_currents() {
        let grid = Grid::new(16, 8, DEFAULT_RADIUS_METRES).unwrap();
        let mut elevations = vec![2_000; grid.sample_count()];
        for row in 0..grid.height {
            elevations[grid.index(row, 0)] = -2_000;
            elevations[grid.index(row, 1)] = -2_000;
        }
        elevations[grid.index(3, 10)] = -2_000;
        elevations[grid.index(3, 11)] = -2_000;
        elevations[grid.index(4, 10)] = -2_000;
        elevations[grid.index(4, 11)] = -2_000;
        let physical = field(grid, elevations, 0);
        let climate = derive_current_climate(
            &physical,
            ClimateSettings::default_for(grid),
            physical.seed,
            physical.retry_index,
            &mut NoopProgress,
        )
        .unwrap();
        for cell in [
            grid.index(3, 10),
            grid.index(3, 11),
            grid.index(4, 10),
            grid.index(4, 11),
        ] {
            assert_eq!(climate.current_east_milli[cell], 0);
            assert_eq!(climate.current_north_milli[cell], 0);
        }
        assert!(climate.metrics.mean_current_speed_milli > 0);
    }

    #[test]
    fn enclosed_seas_get_wind_driven_currents() {
        let grid = Grid::new(16, 8, DEFAULT_RADIUS_METRES).unwrap();
        let mut elevations = vec![2_000; grid.sample_count()];
        for row in 0..grid.height {
            elevations[grid.index(row, 0)] = -2_000;
            elevations[grid.index(row, 1)] = -2_000;
        }
        for row in 2..6 {
            for col in 8..12 {
                elevations[grid.index(row, col)] = -2_000;
            }
        }
        let physical = field(grid, elevations, 0);
        let climate = derive_current_climate(
            &physical,
            ClimateSettings::default_for(grid),
            physical.seed,
            physical.retry_index,
            &mut NoopProgress,
        )
        .unwrap();
        let mut enclosed_speed = 0.0;
        for row in 2..6 {
            for col in 8..12 {
                let cell = grid.index(row, col);
                enclosed_speed += f64::from(climate.current_east_milli[cell])
                    .hypot(f64::from(climate.current_north_milli[cell]));
            }
        }
        assert!(
            enclosed_speed > 0.0,
            "enclosed sea should circulate: {enclosed_speed}"
        );
    }

    #[test]
    fn western_boundary_current_is_stronger_than_the_basin_interior() {
        let grid = Grid::new(32, 16, DEFAULT_RADIUS_METRES).unwrap();
        let mut elevations = vec![-2_000; grid.sample_count()];
        for row in 0..grid.height {
            elevations[grid.index(row, 6)] = 2_000;
        }
        let physical = field(grid, elevations, 0);
        let climate = derive_current_climate(
            &physical,
            ClimateSettings::default_for(grid),
            physical.seed,
            physical.retry_index,
            &mut NoopProgress,
        )
        .unwrap();
        let mut west_speed = 0.0;
        let mut interior_speed = 0.0;
        let mut samples = 0.0;
        for row in 9..12 {
            let west = grid.index(row, 7);
            let interior = grid.index(row, 18);
            west_speed += f64::from(climate.current_east_milli[west])
                .hypot(f64::from(climate.current_north_milli[west]));
            interior_speed += f64::from(climate.current_east_milli[interior])
                .hypot(f64::from(climate.current_north_milli[interior]));
            samples += 1.0;
        }
        assert!(
            west_speed / samples > interior_speed / samples * 1.15,
            "western {west_speed} vs interior {interior_speed}"
        );
        let mut nh_north = 0i64;
        let mut sh_north = 0i64;
        let mut eq_east = 0i64;
        for col in 7..10 {
            nh_north += i64::from(climate.current_north_milli[grid.index(10, col)]);
            sh_north += i64::from(climate.current_north_milli[grid.index(5, col)]);
            eq_east += i64::from(climate.current_east_milli[grid.index(8, col)]);
        }
        assert!(
            nh_north > 0,
            "NH western boundary should run north: {nh_north}"
        );
        assert!(
            sh_north < 0,
            "SH western boundary should run south: {sh_north}"
        );
        assert!(
            eq_east < 0,
            "equator should have a westward current: {eq_east}"
        );
    }

    #[test]
    fn slower_rotation_weakens_surface_currents() {
        let grid = Grid::new(16, 8, DEFAULT_RADIUS_METRES).unwrap();
        let mut elevations = vec![-2_000; grid.sample_count()];
        for row in 0..grid.height {
            elevations[grid.index(row, 4)] = 2_000;
        }
        let physical = field(grid, elevations, 0);
        let earth = derive_current_climate(
            &physical,
            ClimateSettings::default_for(grid),
            physical.seed,
            physical.retry_index,
            &mut NoopProgress,
        )
        .unwrap();
        let mut settings = ClimateSettings::default_for(grid);
        settings.planetary.rotation_period_seconds =
            EARTH_ROTATION_PERIOD_SECONDS.saturating_mul(4);
        settings.planetary.preset = crate::planetary::PlanetaryPreset::Custom;
        let slow = derive_current_climate(
            &physical,
            settings,
            physical.seed,
            physical.retry_index,
            &mut NoopProgress,
        )
        .unwrap();
        assert!(slow.metrics.mean_current_speed_milli < earth.metrics.mean_current_speed_milli);
    }
}
