//! Deterministic historical climate and water derivation.
//!
//! History is a disposable interpretation of the accepted final field. Forcing
//! is a bounded sum of three low-frequency cosines over integer physical time.
//! It is not an Earth-orbit or paleoclimate simulation.

use super::{
    climate::{self, ClimateField, ClimateSettings},
    derive_subsystem_seed,
    evolution::{self, DrainageField},
    hydrology::{self, HydrologyField, ThermalExpansion},
    tectonics::CrustType,
    PhysicalError, PhysicalErrorCode, PhysicalField, ProgressPhase, ProgressSink, SeedDomain,
};

pub const HISTORICAL_DERIVATION_VERSION: u16 = 2;
pub const FORCING_COMPONENT_COUNT: usize = 3;
pub const MAX_HISTORICAL_OFFSET_YEARS: i64 = 10_000_000;
const MIN_PERIOD_YEARS: i64 = 4;
const MAX_PERIOD_YEARS: i64 = 2_000_000;
const MAX_RESPONSE_YEARS: i64 = 500_000;
const MAX_LAG_STEPS: i64 = 4_096;
const COS_PPM: i32 = 1_000_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ForcingComponent {
    pub amplitude_centi_c: i32,
    pub period_years: i64,
    pub phase_offset_years: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HistoricalForcingParameters {
    pub version: u16,
    pub components: [ForcingComponent; FORCING_COMPONENT_COUNT],
    pub sensitivity_ppm: u32,
    pub land_ice_amplitude_ppm: u32,
    pub ice_response_years: i64,
    pub ice_midpoint_centi_c: i32,
    pub ice_transition_width_centi_c: i32,
    pub thermal_expansion_ppm_per_degree_c: u32,
}

impl HistoricalForcingParameters {
    pub fn default_for(seed: u32, retry_index: u32) -> Self {
        let random = derive_subsystem_seed(seed, retry_index, SeedDomain::HistoricalClimate);
        let period_0 = 100_000 + ((random >> 8) % 50_001) as i64;
        let period_1 = 41_000 + ((random >> 24) % 9_001) as i64;
        let period_2 = 23_000 + ((random >> 48) % 5_001) as i64;
        Self {
            version: HISTORICAL_DERIVATION_VERSION,
            components: [
                ForcingComponent {
                    amplitude_centi_c: 420 + (random % 181) as i32,
                    period_years: period_0,
                    phase_offset_years: 0,
                },
                ForcingComponent {
                    amplitude_centi_c: 180 + ((random >> 16) % 121) as i32,
                    period_years: period_1,
                    phase_offset_years: ((random >> 32) as i64).rem_euclid(period_1),
                },
                ForcingComponent {
                    amplitude_centi_c: 70 + ((random >> 40) % 61) as i32,
                    period_years: period_2,
                    phase_offset_years: ((random >> 10) as i64).rem_euclid(period_2),
                },
            ],
            sensitivity_ppm: 1_000_000,
            land_ice_amplitude_ppm: 90_000 + ((random >> 20) % 70_001) as u32,
            ice_response_years: 8_000 + ((random >> 36) % 12_001) as i64,
            ice_midpoint_centi_c: 0,
            ice_transition_width_centi_c: 320 + ((random >> 12) % 161) as i32,
            thermal_expansion_ppm_per_degree_c: 1_500 + ((random >> 44) % 1_001) as u32,
        }
    }

    pub fn period_years(self) -> i64 {
        self.components
            .iter()
            .map(|component| component.period_years)
            .max()
            .unwrap_or(MIN_PERIOD_YEARS)
    }

    pub fn temperature_amplitude_centi_c(self) -> i32 {
        self.components
            .iter()
            .map(|component| component.amplitude_centi_c.unsigned_abs() as i32)
            .sum::<i32>()
            .saturating_mul(self.sensitivity_ppm as i32 / 1_000)
            / 1_000
    }

    pub fn validate(self) -> Result<(), PhysicalError> {
        if self.version != HISTORICAL_DERIVATION_VERSION
            || !(100_000..=2_000_000).contains(&self.sensitivity_ppm)
            || self.land_ice_amplitude_ppm > 500_000
            || !(0..=MAX_RESPONSE_YEARS).contains(&self.ice_response_years)
            || !(-1_000..=1_000).contains(&self.ice_midpoint_centi_c)
            || !(50..=2_000).contains(&self.ice_transition_width_centi_c)
            || self.thermal_expansion_ppm_per_degree_c > 10_000
            || self.temperature_amplitude_centi_c() < 100
            || self.temperature_amplitude_centi_c() > 2_500
        {
            return Err(PhysicalError::coded(
                PhysicalErrorCode::GeneratorInvalidSettings,
                "historical climate forcing is outside its bounded ranges",
            ));
        }
        for component in self.components {
            if !(20..=1_500).contains(&component.amplitude_centi_c)
                || !(MIN_PERIOD_YEARS..=MAX_PERIOD_YEARS).contains(&component.period_years)
                || component.phase_offset_years.unsigned_abs() > MAX_HISTORICAL_OFFSET_YEARS as u64
            {
                return Err(PhysicalError::coded(
                    PhysicalErrorCode::GeneratorInvalidSettings,
                    "historical climate forcing is outside its bounded ranges",
                ));
            }
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

fn cos_turns_ppm(turns_ppm: i64) -> i32 {
    let mut turn = turns_ppm.rem_euclid(1_000_000);
    let mut sign = 1i32;
    if turn >= 500_000 {
        turn -= 500_000;
        sign = -1;
    }
    if turn >= 250_000 {
        turn = 500_000 - turn;
        sign = -sign;
    }
    let radians_micro = (turn * 3_141_593) / 500_000;
    let x2 = (radians_micro * radians_micro) / 1_000_000;
    let mut term = 1_000_000i64;
    let mut sum = term;
    for denom in [2i64, 12, 30, 56, 90] {
        term = -term * x2 / 1_000_000 / denom;
        sum += term;
    }
    (sum.clamp(-i64::from(COS_PPM), i64::from(COS_PPM)) as i32) * sign
}

pub fn forcing_centi_c(epoch_offset_years: i64, parameters: HistoricalForcingParameters) -> i32 {
    let mut sum = 0i64;
    for component in parameters.components {
        let turns = ((epoch_offset_years + component.phase_offset_years) * 1_000_000)
            .div_euclid(component.period_years.max(1));
        sum += i64::from(component.amplitude_centi_c) * i64::from(cos_turns_ppm(turns));
    }
    ((sum * i64::from(parameters.sensitivity_ppm)) / 1_000_000 / 1_000_000) as i32
}

pub fn temperature_offset_centi_c(
    epoch_offset_years: i64,
    parameters: HistoricalForcingParameters,
) -> i32 {
    forcing_centi_c(epoch_offset_years, parameters)
        .saturating_sub(forcing_centi_c(0, parameters))
}

fn exp_neg_ppm(x_ppm: u64) -> u64 {
    if x_ppm == 0 {
        return 1_000_000;
    }
    if x_ppm >= 20_000_000 {
        return 0;
    }
    let mut term = 1_000_000u128;
    let mut sum = term;
    for n in 1..=16 {
        term = term * u128::from(x_ppm) / 1_000_000 / n as u128;
        sum += term;
        if term == 0 {
            break;
        }
    }
    (1_000_000u128 * 1_000_000 / sum.max(1)) as u64
}

fn mix_i32(previous: i32, target: i32, decay_ppm: u64) -> i32 {
    let keep = i64::from(previous) * decay_ppm as i64;
    let add = i64::from(target) * (1_000_000 - decay_ppm as i64);
    ((keep + add) / 1_000_000) as i32
}

pub fn lagged_temperature_offset_centi_c(
    epoch_offset_years: i64,
    parameters: HistoricalForcingParameters,
) -> i32 {
    lagged_temperature_offset_with_step(
        epoch_offset_years,
        parameters,
        (parameters.ice_response_years / 32).max(1),
    )
}

fn lagged_temperature_offset_with_step(
    epoch_offset_years: i64,
    parameters: HistoricalForcingParameters,
    requested_step: i64,
) -> i32 {
    if epoch_offset_years == 0 {
        return 0;
    }
    if parameters.ice_response_years == 0 {
        return temperature_offset_centi_c(epoch_offset_years, parameters);
    }
    let step = requested_step.abs().clamp(1, parameters.ice_response_years.max(1));
    let steps = (epoch_offset_years.abs() / step).clamp(1, MAX_LAG_STEPS);
    let signed_step = if epoch_offset_years > 0 { step } else { -step };
    let decay =
        exp_neg_ppm((step.unsigned_abs() * 1_000_000) / parameters.ice_response_years as u64);
    let mut lag = 0i32;
    let mut t = 0i64;
    for _ in 0..steps {
        t += signed_step;
        lag = mix_i32(lag, temperature_offset_centi_c(t, parameters), decay);
    }
    if t != epoch_offset_years {
        let remainder = (epoch_offset_years - t).unsigned_abs();
        let remainder_decay =
            exp_neg_ppm((remainder * 1_000_000) / parameters.ice_response_years as u64);
        lag = mix_i32(
            lag,
            temperature_offset_centi_c(epoch_offset_years, parameters),
            remainder_decay,
        );
    }
    lag
}

fn scaled_volume(reference: u64, first_ppm: u64, second_ppm: u64) -> u64 {
    ((u128::from(reference) * u128::from(first_ppm) * u128::from(second_ppm))
        / 1_000_000u128
        / 1_000_000u128) as u64
}

fn logistic_ppm(temperature_centi_c: i32, parameters: HistoricalForcingParameters) -> u64 {
    let width = i64::from(parameters.ice_transition_width_centi_c).max(1);
    let argument_ppm = ((i64::from(temperature_centi_c)
        - i64::from(parameters.ice_midpoint_centi_c))
        * 1_000_000)
        / width;
    if argument_ppm >= 20_000_000 {
        return 0;
    }
    if argument_ppm <= -20_000_000 {
        return 1_000_000;
    }
    if argument_ppm >= 0 {
        let exp = 1_000_000u128 * 1_000_000 / u128::from(exp_neg_ppm(argument_ppm as u64).max(1));
        (1_000_000u128 * 1_000_000 / (1_000_000u128 + exp)) as u64
    } else {
        let exp = u128::from(exp_neg_ppm((-argument_ppm) as u64));
        (1_000_000u128 * 1_000_000 / (1_000_000u128 + exp)) as u64
    }
}

fn land_ice_volume(
    reference: u64,
    temperature_centi_c: i32,
    parameters: HistoricalForcingParameters,
) -> u64 {
    scaled_volume(
        reference,
        u64::from(parameters.land_ice_amplitude_ppm),
        logistic_ppm(temperature_centi_c, parameters),
    )
}

fn thermal_expansion_volume(
    volume_m3: u64,
    temperature_centi_c: i32,
    parameters: HistoricalForcingParameters,
) -> i64 {
    hydrology::thermal_expansion_delta_m3(
        volume_m3,
        ThermalExpansion {
            ppm_per_degree_c: parameters.thermal_expansion_ppm_per_degree_c,
            temperature_offset_centi_c: temperature_centi_c,
        },
    )
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
    let land_ice_equilibrium_m3 =
        land_ice_volume(reference_water_inventory_m3, lagged_temperature, parameters);
    let expansion = ThermalExpansion {
        ppm_per_degree_c: parameters.thermal_expansion_ppm_per_degree_c,
        temperature_offset_centi_c: temperature_offset,
    };

    progress.report(ProgressPhase::CalculatingClimate, 0, 1)?;
    let initial_sea_level = if normalized_epoch == 0 {
        field.sea_level_mm
    } else {
        let remaining = reference_water_inventory_m3.max(1);
        hydrology::solve_ocean_level_for_inventory(
            field,
            (i128::from(remaining)
                + i128::from(thermal_expansion_volume(
                    remaining,
                    temperature_offset,
                    parameters,
                )))
            .clamp(1, i128::from(u64::MAX)) as u64,
        )?
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
    let hydrology = hydrology::derive_hydrology_with_forcing(
        &epoch_field,
        &climate,
        &drainage,
        reference_water_inventory_m3,
        crust_by_cell,
        Some(expansion),
    )?;
    progress.check_cancelled()?;
    progress.report(ProgressPhase::CalculatingWater, 1, 1)?;

    let land_ice_m3 = hydrology.metrics.land_ice_m3;
    let remaining_mass = reference_water_inventory_m3
        .saturating_sub(land_ice_m3)
        .saturating_sub(hydrology.metrics.inland_water_m3)
        .max(1);
    let thermal_expansion_m3 =
        thermal_expansion_volume(remaining_mass, temperature_offset, parameters);
    let effective_ocean_water_m3 = hydrology
        .metrics
        .ocean_water_m3
        .saturating_add(hydrology.metrics.inland_water_m3);
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

    fn extreme_epochs(parameters: HistoricalForcingParameters) -> (i64, i64) {
        let span = parameters.period_years();
        let mut cold = (-span, i32::MAX);
        let mut warm = (span, i32::MIN);
        let mut t = -span;
        while t <= span {
            let temperature = temperature_offset_centi_c(t, parameters);
            if temperature < cold.1 {
                cold = (t, temperature);
            }
            if temperature > warm.1 {
                warm = (t, temperature);
            }
            t += span / 32;
        }
        (cold.0, warm.0)
    }

    #[test]
    fn cosine_is_integer_and_bounded() {
        assert_eq!(cos_turns_ppm(0), 1_000_000);
        assert!(cos_turns_ppm(250_000).abs() <= 20_000);
        assert!(cos_turns_ppm(500_000) <= -980_000);
        assert_eq!(cos_turns_ppm(1_000_000), cos_turns_ppm(0));
    }

    #[test]
    fn forcing_is_smooth_and_bounded() {
        let parameters = HistoricalForcingParameters::default_for(831_429, 0);
        let bound = parameters.temperature_amplitude_centi_c();
        let mut previous = temperature_offset_centi_c(0, parameters);
        let mut previous_delta = 0i32;
        for year in 1..=5_000 {
            let current = temperature_offset_centi_c(year, parameters);
            assert!(current.abs() <= bound);
            let delta = current - previous;
            assert!(delta.abs() <= 8);
            if year > 1 {
                assert!((delta - previous_delta).abs() <= 4);
            }
            previous = current;
            previous_delta = delta;
        }
    }

    #[test]
    fn ice_lag_has_hysteresis_and_timestep_refinement() {
        let parameters = HistoricalForcingParameters::default_for(831_429, 0);
        let (cold_epoch, warm_epoch) = extreme_epochs(parameters);
        let coarse = lagged_temperature_offset_with_step(warm_epoch, parameters, 512);
        let fine = lagged_temperature_offset_with_step(warm_epoch, parameters, 64);
        assert!((coarse - fine).abs() <= 25);
        let rising = lagged_temperature_offset_centi_c(warm_epoch.abs(), parameters);
        let falling = lagged_temperature_offset_centi_c(-warm_epoch.abs(), parameters);
        assert_ne!(rising, falling);
        assert!(
            land_ice_volume(1_000_000_000, temperature_offset_centi_c(cold_epoch, parameters), parameters)
                > land_ice_volume(
                    1_000_000_000,
                    temperature_offset_centi_c(warm_epoch, parameters),
                    parameters
                )
        );
    }

    #[test]
    fn cooling_and_warming_move_water_in_expected_directions() {
        let parameters = HistoricalForcingParameters::default_for(831_429, 0);
        let (cold_epoch, warm_epoch) = extreme_epochs(parameters);
        let cold_temperature = temperature_offset_centi_c(cold_epoch, parameters);
        let warm_temperature = temperature_offset_centi_c(warm_epoch, parameters);
        assert!(cold_temperature < 0);
        assert!(warm_temperature > 0);
        let reference = 1_000_000_000u64;
        let cold_ice = land_ice_volume(reference, cold_temperature, parameters);
        let warm_ice = land_ice_volume(reference, warm_temperature, parameters);
        assert!(cold_ice > warm_ice);
        let cold_expansion = thermal_expansion_volume(reference, cold_temperature, parameters);
        let warm_expansion = thermal_expansion_volume(reference, warm_temperature, parameters);
        assert!(cold_expansion < warm_expansion);
        let expansion_bound = reference * 50_000 / 1_000_000;
        assert!(cold_expansion.unsigned_abs() <= expansion_bound);
        assert!(warm_expansion.unsigned_abs() <= expansion_bound);
        let cold_lagged = lagged_temperature_offset_centi_c(cold_epoch, parameters);
        let warm_lagged = lagged_temperature_offset_centi_c(warm_epoch, parameters);
        let cold_lagged_ice = land_ice_volume(reference, cold_lagged, parameters);
        let warm_lagged_ice = land_ice_volume(reference, warm_lagged, parameters);
        let cold_available =
            i128::from(reference) - i128::from(cold_lagged_ice) + i128::from(cold_expansion);
        let warm_available =
            i128::from(reference) - i128::from(warm_lagged_ice) + i128::from(warm_expansion);
        assert!(cold_available < warm_available);
    }

    #[test]
    fn historical_derivation_preserves_the_canonical_field_and_conserves_water() {
        let world = fixture();
        let before = world.field.clone();
        let source_before = crate::tectonics::encode_source_v2(&world.tectonics).unwrap();
        let parameters =
            HistoricalForcingParameters::default_for(world.field.seed, world.field.retry_index);
        let (cold_epoch, warm_epoch) = extreme_epochs(parameters);
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
            warm_epoch,
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
            cold_epoch,
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
            warm_epoch,
            &mut second_progress,
        )
        .unwrap();
        assert_eq!(derived, repeated);
    }
}
