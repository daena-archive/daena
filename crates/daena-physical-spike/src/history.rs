//! Deterministic historical climate and water derivation.
//!
//! History is a disposable interpretation of the accepted final field. The
//! forcing is deliberately a bounded, low-frequency heuristic: it gives the
//! map a useful geographic playback axis without implying an Earth-orbit or
//! paleoclimate simulation.

use super::{
    climate::{self, ClimateField, ClimateSettings},
    derive_subsystem_seed,
    evolution::{self, DrainageField},
    hydrology::{self, HydrologyField},
    tectonics::CrustType,
    PhysicalError, PhysicalErrorCode, PhysicalField, ProgressPhase, ProgressSink, SeedDomain,
};

pub const HISTORICAL_DERIVATION_VERSION: u16 = 1;
pub const MAX_HISTORICAL_OFFSET_YEARS: i64 = 10_000_000;
const MIN_PERIOD_YEARS: i64 = 4;
const MAX_PERIOD_YEARS: i64 = 2_000_000;
const MAX_RESPONSE_YEARS: i64 = 500_000;
const MAX_THERMAL_EXPANSION_FRACTION_PPM: u64 = 50_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HistoricalForcingParameters {
    pub version: u16,
    pub temperature_amplitude_centi_c: i32,
    pub period_years: i64,
    pub phase_offset_years: i64,
    pub land_ice_amplitude_ppm: u32,
    pub ice_response_years: i64,
    pub thermal_expansion_ppm_per_degree_c: u32,
}

impl HistoricalForcingParameters {
    pub fn default_for(seed: u32, retry_index: u32) -> Self {
        let random = derive_subsystem_seed(seed, retry_index, SeedDomain::HistoricalClimate);
        Self {
            version: HISTORICAL_DERIVATION_VERSION,
            temperature_amplitude_centi_c: 700 + (random % 501) as i32,
            period_years: 80_000 + ((random >> 12) % 80_001) as i64,
            phase_offset_years: 0,
            land_ice_amplitude_ppm: 90_000 + ((random >> 24) % 70_001) as u32,
            ice_response_years: 8_000 + ((random >> 36) % 12_001) as i64,
            thermal_expansion_ppm_per_degree_c: 1_500 + ((random >> 48) % 1_001) as u32,
        }
    }

    pub fn validate(self) -> Result<(), PhysicalError> {
        if self.version != HISTORICAL_DERIVATION_VERSION
            || !(100..=2_000).contains(&self.temperature_amplitude_centi_c)
            || !(MIN_PERIOD_YEARS..=MAX_PERIOD_YEARS).contains(&self.period_years)
            || self.phase_offset_years.unsigned_abs() > MAX_HISTORICAL_OFFSET_YEARS as u64
            || self.land_ice_amplitude_ppm > 500_000
            || !(0..=MAX_RESPONSE_YEARS).contains(&self.ice_response_years)
            || self.thermal_expansion_ppm_per_degree_c > 10_000
        {
            return Err(PhysicalError::coded(
                PhysicalErrorCode::GeneratorInvalidSettings,
                "historical climate forcing is outside its bounded ranges",
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HistoricalMetrics {
    pub derivation_version: u16,
    pub epoch_offset_years: i64,
    pub normalized_epoch: i64,
    pub temperature_offset_centi_c: i32,
    pub lagged_temperature_offset_centi_c: i32,
    pub land_ice_equilibrium_m3: u64,
    pub land_ice_m3: u64,
    pub thermal_expansion_m3: i64,
    pub effective_ocean_water_m3: u64,
    pub conserved_water_m3: u64,
    pub balance_error_m3: u64,
    pub sea_level_mm: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HistoricalWorld {
    pub metrics: HistoricalMetrics,
    pub climate: ClimateField,
    pub drainage: DrainageField,
    pub hydrology: HydrologyField,
}

pub fn normalize_epoch_offset(epoch_offset_years: i64) -> Result<i64, PhysicalError> {
    if !(-MAX_HISTORICAL_OFFSET_YEARS..=MAX_HISTORICAL_OFFSET_YEARS).contains(&epoch_offset_years) {
        return Err(PhysicalError::coded(
            PhysicalErrorCode::GeneratorInvalidSettings,
            "historical epoch is outside the bounded playback range",
        ));
    }
    Ok(epoch_offset_years)
}

fn lerp_i32(start: i32, end: i32, position: i64, length: i64) -> i32 {
    start.saturating_add((((i64::from(end) - i64::from(start)) * position) / length) as i32)
}

fn triangle_wave_centi_c(epoch: i64, parameters: HistoricalForcingParameters) -> i32 {
    let phase = (epoch + parameters.phase_offset_years).rem_euclid(parameters.period_years);
    let quarter = (parameters.period_years / 4).max(1);
    let three_quarter = quarter * 3;
    let amplitude = parameters.temperature_amplitude_centi_c;
    if phase <= quarter {
        lerp_i32(0, amplitude, phase, quarter)
    } else if phase <= three_quarter {
        lerp_i32(amplitude, -amplitude, phase - quarter, quarter * 2)
    } else {
        lerp_i32(-amplitude, 0, phase - three_quarter, quarter)
    }
}

pub fn temperature_offset_centi_c(
    epoch_offset_years: i64,
    parameters: HistoricalForcingParameters,
) -> i32 {
    let baseline = triangle_wave_centi_c(0, parameters);
    triangle_wave_centi_c(epoch_offset_years, parameters).saturating_sub(baseline)
}

fn lagged_temperature_offset_centi_c(
    epoch_offset_years: i64,
    parameters: HistoricalForcingParameters,
) -> i32 {
    let baseline = temperature_offset_centi_c(-parameters.ice_response_years, parameters);
    temperature_offset_centi_c(
        epoch_offset_years.saturating_sub(parameters.ice_response_years),
        parameters,
    )
    .saturating_sub(baseline)
}

fn scaled_volume(reference: u64, first_ppm: u64, second_ppm: u64) -> u64 {
    ((u128::from(reference) * u128::from(first_ppm) * u128::from(second_ppm))
        / 1_000_000u128
        / 1_000_000u128) as u64
}

fn land_ice_volume(
    reference: u64,
    cooling_centi_c: i32,
    parameters: HistoricalForcingParameters,
) -> u64 {
    let cooling_ppm = if cooling_centi_c <= 0 {
        0
    } else {
        (i64::from(cooling_centi_c) * 1_000_000
            / i64::from(parameters.temperature_amplitude_centi_c))
        .clamp(0, 1_000_000) as u64
    };
    scaled_volume(
        reference,
        u64::from(parameters.land_ice_amplitude_ppm),
        cooling_ppm,
    )
}

fn thermal_expansion_volume(
    reference: u64,
    temperature_centi_c: i32,
    parameters: HistoricalForcingParameters,
) -> i64 {
    let raw = (i128::from(reference)
        * i128::from(temperature_centi_c)
        * i128::from(parameters.thermal_expansion_ppm_per_degree_c))
        / 100_000_000;
    let bound = (u128::from(reference) * u128::from(MAX_THERMAL_EXPANSION_FRACTION_PPM) / 1_000_000)
        .min(i128::from(i64::MAX) as u128) as i128;
    raw.clamp(-bound, bound) as i64
}

pub fn derive_historical_world(
    field: &PhysicalField,
    reference_water_inventory_m3: u64,
    crust_by_cell: Option<&[CrustType]>,
    parameters: HistoricalForcingParameters,
    epoch_offset_years: i64,
    progress: &mut dyn ProgressSink,
) -> Result<HistoricalWorld, PhysicalError> {
    field.validate().map_err(PhysicalError::InvalidSource)?;
    parameters.validate()?;
    let normalized_epoch = normalize_epoch_offset(epoch_offset_years)?;
    if reference_water_inventory_m3 == 0 {
        return Err(PhysicalError::coded(
            PhysicalErrorCode::GeneratorInvalidSettings,
            "historical derivation requires a positive water inventory",
        ));
    }
    let temperature_offset = temperature_offset_centi_c(normalized_epoch, parameters);
    let lagged_temperature = lagged_temperature_offset_centi_c(normalized_epoch, parameters);
    let land_ice_equilibrium_m3 = land_ice_volume(
        reference_water_inventory_m3,
        -temperature_offset,
        parameters,
    );
    let land_ice_m3 = land_ice_volume(
        reference_water_inventory_m3,
        -lagged_temperature,
        parameters,
    );
    let thermal_expansion_m3 =
        thermal_expansion_volume(reference_water_inventory_m3, temperature_offset, parameters);
    let effective_ocean_water_m3 = (i128::from(reference_water_inventory_m3)
        - i128::from(land_ice_m3)
        + i128::from(thermal_expansion_m3))
    .clamp(1, i128::from(u64::MAX)) as u64;

    progress.report(ProgressPhase::CalculatingClimate, 0, 1)?;
    let initial_sea_level = if normalized_epoch == 0 {
        // The accepted current world is the persisted reference epoch. Keep
        // its initial sea level so the existing inland-storage fixed point
        // reproduces the current derived climate and hydrology exactly.
        field.sea_level_mm
    } else {
        hydrology::solve_ocean_level_for_inventory(field, effective_ocean_water_m3)?
    };
    let mut epoch_field = field.clone();
    epoch_field.sea_level_mm = initial_sea_level;
    let mut climate_settings = ClimateSettings::default_for(field.grid);
    climate_settings.global_temperature_centi_c = climate_settings
        .global_temperature_centi_c
        .saturating_add(temperature_offset);
    let climate = climate::derive_current_climate(
        &epoch_field,
        climate_settings,
        field.seed,
        field.retry_index,
        progress,
    )?;
    progress.report(ProgressPhase::CalculatingClimate, 1, 1)?;
    progress.check_cancelled()?;
    let drainage = evolution::derive_drainage(&epoch_field, &climate)?;
    progress.check_cancelled()?;
    progress.report(ProgressPhase::CalculatingWater, 0, 1)?;
    let hydrology = hydrology::derive_hydrology_with_crust(
        &epoch_field,
        &climate,
        &drainage,
        effective_ocean_water_m3,
        crust_by_cell,
    )?;
    progress.check_cancelled()?;
    progress.report(ProgressPhase::CalculatingWater, 1, 1)?;

    let conserved = i128::from(hydrology.metrics.ocean_water_m3)
        + i128::from(hydrology.metrics.inland_water_m3)
        + i128::from(land_ice_m3)
        - i128::from(thermal_expansion_m3);
    let conserved_water_m3 = conserved.clamp(0, i128::from(u64::MAX)) as u64;
    let balance_error_m3 = reference_water_inventory_m3.abs_diff(conserved_water_m3);
    Ok(HistoricalWorld {
        metrics: HistoricalMetrics {
            derivation_version: HISTORICAL_DERIVATION_VERSION,
            epoch_offset_years,
            normalized_epoch,
            temperature_offset_centi_c: temperature_offset,
            lagged_temperature_offset_centi_c: lagged_temperature,
            land_ice_equilibrium_m3,
            land_ice_m3,
            thermal_expansion_m3,
            effective_ocean_water_m3,
            conserved_water_m3,
            balance_error_m3,
            sea_level_mm: hydrology.sea_level_mm,
        },
        climate,
        drainage,
        hydrology,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{generate_world, GenerationSettings, NoopProgress, DEFAULT_RADIUS_METRES};

    fn fixture() -> crate::GeneratedWorld {
        let mut progress = NoopProgress;
        generate_world(
            GenerationSettings {
                width: 32,
                height: 16,
                radius_metres: DEFAULT_RADIUS_METRES,
                target_land_fraction_ppm: 300_000,
            },
            831_429,
            0,
            &mut progress,
        )
        .unwrap()
    }

    #[test]
    fn default_forcing_is_deterministic_and_epoch_zero_is_reference() {
        let first = HistoricalForcingParameters::default_for(831_429, 0);
        assert_eq!(first, HistoricalForcingParameters::default_for(831_429, 0));
        assert_eq!(temperature_offset_centi_c(0, first), 0);
        assert_eq!(lagged_temperature_offset_centi_c(0, first), 0);
    }

    #[test]
    fn historical_api_bound_is_explicit_and_inclusive() {
        assert_eq!(
            normalize_epoch_offset(MAX_HISTORICAL_OFFSET_YEARS),
            Ok(MAX_HISTORICAL_OFFSET_YEARS)
        );
        assert_eq!(
            normalize_epoch_offset(-MAX_HISTORICAL_OFFSET_YEARS),
            Ok(-MAX_HISTORICAL_OFFSET_YEARS)
        );
        assert!(normalize_epoch_offset(MAX_HISTORICAL_OFFSET_YEARS + 1).is_err());
        assert!(normalize_epoch_offset(-MAX_HISTORICAL_OFFSET_YEARS - 1).is_err());
    }

    #[test]
    fn cooling_and_warming_move_water_in_expected_directions() {
        let parameters = HistoricalForcingParameters::default_for(831_429, 0);
        let cold_epoch = -parameters.period_years / 4;
        let warm_epoch = parameters.period_years / 4;
        let cold_temperature = temperature_offset_centi_c(cold_epoch, parameters);
        let warm_temperature = temperature_offset_centi_c(warm_epoch, parameters);
        assert!(cold_temperature < 0);
        assert!(warm_temperature > 0);
        let reference = 1_000_000_000u64;
        let cold_ice = land_ice_volume(reference, -cold_temperature, parameters);
        let warm_ice = land_ice_volume(reference, -warm_temperature, parameters);
        assert!(cold_ice > warm_ice);
        let cold_expansion = thermal_expansion_volume(reference, cold_temperature, parameters);
        let warm_expansion = thermal_expansion_volume(reference, warm_temperature, parameters);
        assert!(cold_expansion < warm_expansion);
        let expansion_bound = reference * MAX_THERMAL_EXPANSION_FRACTION_PPM / 1_000_000;
        assert!(cold_expansion.unsigned_abs() <= expansion_bound);
        assert!(warm_expansion.unsigned_abs() <= expansion_bound);
        let cold_lagged = lagged_temperature_offset_centi_c(cold_epoch, parameters);
        let warm_lagged = lagged_temperature_offset_centi_c(warm_epoch, parameters);
        let cold_lagged_ice = land_ice_volume(reference, -cold_lagged, parameters);
        let warm_lagged_ice = land_ice_volume(reference, -warm_lagged, parameters);
        let cold_available =
            i128::from(reference) - i128::from(cold_lagged_ice) + i128::from(cold_expansion);
        let warm_available =
            i128::from(reference) - i128::from(warm_lagged_ice) + i128::from(warm_expansion);
        assert!(cold_available < warm_available);
        let transition_epoch = parameters.period_years / 2 + parameters.ice_response_years / 2;
        let transition_equilibrium = temperature_offset_centi_c(transition_epoch, parameters);
        let transition_lagged = lagged_temperature_offset_centi_c(transition_epoch, parameters);
        assert!(transition_equilibrium < 0);
        assert!(transition_lagged > transition_equilibrium);
        assert!(
            land_ice_volume(reference, -transition_equilibrium, parameters)
                > land_ice_volume(reference, -transition_lagged, parameters)
        );
    }

    #[test]
    fn historical_derivation_preserves_the_canonical_field_and_conserves_water() {
        let world = fixture();
        let before = world.field.clone();
        let source_before = crate::tectonics::encode_source_v2(&world.tectonics).unwrap();
        let parameters =
            HistoricalForcingParameters::default_for(world.field.seed, world.field.retry_index);
        let mut reference_progress = NoopProgress;
        let reference = derive_historical_world(
            &world.field,
            world.report.reference_water_inventory_m3,
            Some(&world.tectonics.crust_by_cell),
            parameters,
            0,
            &mut reference_progress,
        )
        .unwrap();
        assert_eq!(reference.climate, world.climate);
        assert_eq!(reference.drainage, world.evolution.drainage);
        assert_eq!(reference.hydrology, world.hydrology);
        let mut progress = NoopProgress;
        let derived = derive_historical_world(
            &world.field,
            world.report.reference_water_inventory_m3,
            Some(&world.tectonics.crust_by_cell),
            parameters,
            parameters.period_years / 4,
            &mut progress,
        )
        .unwrap();
        assert_eq!(world.field, before);
        assert_eq!(
            crate::tectonics::encode_source_v2(&world.tectonics).unwrap(),
            source_before
        );
        assert!(derived.hydrology.metrics.converged);
        assert!(derived.metrics.balance_error_m3 <= derived.hydrology.metrics.tolerance_m3);
        let mut cold_progress = NoopProgress;
        let cold = derive_historical_world(
            &world.field,
            world.report.reference_water_inventory_m3,
            Some(&world.tectonics.crust_by_cell),
            parameters,
            -parameters.period_years / 4,
            &mut cold_progress,
        )
        .unwrap();
        assert!(cold.hydrology.metrics.converged);
        assert!(cold.metrics.balance_error_m3 <= cold.hydrology.metrics.tolerance_m3);
        assert!(cold.metrics.land_ice_m3 > derived.metrics.land_ice_m3);
        assert!(cold.metrics.effective_ocean_water_m3 < derived.metrics.effective_ocean_water_m3);
        let cold_flooded_cells = cold
            .hydrology
            .bathymetry_mm
            .iter()
            .filter(|depth| **depth > 0)
            .count();
        let warm_flooded_cells = derived
            .hydrology
            .bathymetry_mm
            .iter()
            .filter(|depth| **depth > 0)
            .count();
        assert!(cold_flooded_cells < warm_flooded_cells);
        assert!(
            cold.hydrology
                .shelf_cells
                .iter()
                .filter(|cell| **cell)
                .count()
                <= derived
                    .hydrology
                    .shelf_cells
                    .iter()
                    .filter(|cell| **cell)
                    .count()
        );
        assert_ne!(cold.hydrology.island_id, derived.hydrology.island_id);
        let mut second_progress = NoopProgress;
        let repeated = derive_historical_world(
            &world.field,
            world.report.reference_water_inventory_m3,
            Some(&world.tectonics.crust_by_cell),
            parameters,
            parameters.period_years / 4,
            &mut second_progress,
        )
        .unwrap();
        assert_eq!(derived, repeated);
    }
}
