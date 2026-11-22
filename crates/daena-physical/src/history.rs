//! Deterministic historical climate and water derivation.
//!
//! History is a disposable interpretation of the accepted final field. Forcing
//! is a bounded sum of three low-frequency cosines over integer physical time.
//! It is not an Earth-orbit or paleoclimate simulation.

use super::{
    derive_subsystem_seed, derive_subsystem_seed64,
    derived_cache::StaticDerivedPhysics,
    engine::{DecodedSource, DerivedProducts},
    planetary::PlanetaryConfiguration,
    PhysicalError, PhysicalErrorCode, ProgressPhase, ProgressSink, SeedDomain,
};

pub const HISTORICAL_DERIVATION_VERSION: u16 = 3;
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
        Self::from_random(random)
    }

    /// Forcing from the full 64-bit world seed. The product seed is not
    /// truncated through a `u32` on this path.
    pub fn default_for_seed(seed: u64) -> Self {
        Self::from_random(derive_subsystem_seed64(seed, SeedDomain::HistoricalClimate))
    }

    fn from_random(random: u64) -> Self {
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
    /// Signed change from epoch zero. Positive means more land ice than at
    /// epoch zero; negative means less.
    pub land_ice_m3: i64,
    pub thermal_expansion_m3: i64,
    pub effective_ocean_water_m3: u64,
    pub conserved_water_m3: u64,
    pub balance_error_m3: u64,
    pub sea_level_mm: i32,
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
    forcing_centi_c(epoch_offset_years, parameters).saturating_sub(forcing_centi_c(0, parameters))
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
    let step = requested_step
        .abs()
        .clamp(1, parameters.ice_response_years.max(1));
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
    let argument_ppm =
        ((i64::from(temperature_centi_c) - i64::from(parameters.ice_midpoint_centi_c)) * 1_000_000)
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
    const MAX_THERMAL_EXPANSION_FRACTION_PPM: u64 = 50_000;
    let raw = (i128::from(volume_m3)
        * i128::from(temperature_centi_c)
        * i128::from(parameters.thermal_expansion_ppm_per_degree_c))
        / 100_000_000;
    let bound = (u128::from(volume_m3) * u128::from(MAX_THERMAL_EXPANSION_FRACTION_PPM) / 1_000_000)
        .min(i128::from(i64::MAX) as u128) as i128;
    raw.clamp(-bound, bound) as i64
}

/// Engine-backed historical products for one epoch.
#[derive(Clone, PartialEq, Eq)]
pub struct DerivedHistoricalWorld {
    pub metrics: HistoricalMetrics,
    pub products: DerivedProducts,
}

/// One solved epoch: its forcing, water inventory and sea level.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EpochSolve {
    pub normalized_epoch: i64,
    pub temperature_offset_centi_c: i32,
    pub lagged_temperature_offset_centi_c: i32,
    pub land_ice_equilibrium_m3: u64,
    /// Signed change from epoch zero. Positive means more land ice.
    pub land_ice_m3: i64,
    pub thermal_expansion_m3: i64,
    pub target_ocean_water_m3: u64,
    pub sea_level_mm: i32,
}

/// Solve the epoch forcing, land-ice change, expansion and sea level once.
///
/// This is the shared path of [`derive_historical_products`] and the physical
/// tools. Epoch zero takes no hypsometric solve and keeps the reference
/// inventory and sea level.
pub fn solve_epoch(
    decoded: &DecodedSource,
    parameters: HistoricalForcingParameters,
    epoch_offset_years: i64,
    progress: &dyn ProgressSink,
) -> Result<EpochSolve, PhysicalError> {
    progress.check_cancelled()?;
    parameters.validate()?;
    let normalized_epoch = normalize_epoch_offset(epoch_offset_years)?;
    let reference_inventory = decoded.reference_water_inventory_m3();
    if reference_inventory == 0 {
        return Err(PhysicalError::coded(
            PhysicalErrorCode::GeneratorInvalidSettings,
            "historical derivation requires a positive water inventory",
        ));
    }
    let temperature = temperature_offset_centi_c(normalized_epoch, parameters);
    let lagged = lagged_temperature_offset_centi_c(normalized_epoch, parameters);
    let equilibrium = land_ice_volume(reference_inventory, lagged, parameters);
    let equilibrium_at_zero = land_ice_volume(reference_inventory, 0, parameters);
    let land_ice_change = i128::from(equilibrium) - i128::from(equilibrium_at_zero);
    let expansion_at = thermal_expansion_volume(reference_inventory, temperature, parameters);
    let expansion_at_zero = thermal_expansion_volume(reference_inventory, 0, parameters);
    let expansion_change = i128::from(expansion_at) - i128::from(expansion_at_zero);
    if normalized_epoch == 0 {
        return Ok(EpochSolve {
            normalized_epoch,
            temperature_offset_centi_c: 0,
            lagged_temperature_offset_centi_c: 0,
            land_ice_equilibrium_m3: equilibrium_at_zero,
            land_ice_m3: 0,
            thermal_expansion_m3: 0,
            target_ocean_water_m3: reference_inventory,
            sea_level_mm: 0,
        });
    }
    let curve = decoded.ocean_volume_curve()?;
    progress.check_cancelled()?;
    let maximum = i128::from(curve.last().map(|point| point.volume_m3).unwrap_or(0));
    let target =
        (i128::from(reference_inventory) - land_ice_change + expansion_change).clamp(0, maximum);
    let target = target as u64;
    let sea_level_mm = decoded.sea_level_for_curve(&curve, target)?;
    progress.check_cancelled()?;
    Ok(EpochSolve {
        normalized_epoch,
        temperature_offset_centi_c: temperature,
        lagged_temperature_offset_centi_c: lagged,
        land_ice_equilibrium_m3: equilibrium,
        land_ice_m3: land_ice_change.clamp(i128::from(i64::MIN), i128::from(i64::MAX)) as i64,
        thermal_expansion_m3: expansion_change.clamp(i128::from(i64::MIN), i128::from(i64::MAX))
            as i64,
        target_ocean_water_m3: target,
        sea_level_mm,
    })
}

/// Derive one epoch from the canonical source.
///
/// Land ice and thermal expansion are changes relative to epoch zero. Epoch
/// zero reuses the present products byte for byte and takes no sea-level solve
/// or climate run.
pub fn derive_historical_products(
    decoded: &DecodedSource,
    present: &StaticDerivedPhysics,
    parameters: HistoricalForcingParameters,
    epoch_offset_years: i64,
    planetary: PlanetaryConfiguration,
    progress: &mut dyn ProgressSink,
) -> Result<DerivedHistoricalWorld, PhysicalError> {
    let solved = solve_epoch(decoded, parameters, epoch_offset_years, progress)?;
    if solved.normalized_epoch == 0 {
        let products = DerivedProducts {
            view: present.view.clone(),
            climate: present.climate.clone(),
            hydrology: present.hydrology.clone(),
            hazards: present.hazards.clone(),
            derived_geojson: present.geojson.clone(),
            hashes: present.hashes,
        };
        return Ok(DerivedHistoricalWorld {
            metrics: HistoricalMetrics {
                derivation_version: HISTORICAL_DERIVATION_VERSION,
                epoch_offset_years,
                normalized_epoch: 0,
                temperature_offset_centi_c: 0,
                lagged_temperature_offset_centi_c: 0,
                land_ice_equilibrium_m3: solved.land_ice_equilibrium_m3,
                land_ice_m3: 0,
                thermal_expansion_m3: 0,
                effective_ocean_water_m3: decoded.reference_water_inventory_m3(),
                conserved_water_m3: solved.target_ocean_water_m3,
                balance_error_m3: 0,
                sea_level_mm: 0,
            },
            products,
        });
    }

    progress.report(ProgressPhase::CalculatingClimate, 0, 1)?;
    let products = decoded.derive_epoch_products(
        solved.sea_level_mm,
        f64::from(solved.temperature_offset_centi_c) / 100.0,
        planetary,
        progress,
    )?;
    progress.report(ProgressPhase::CalculatingClimate, 1, 1)?;
    progress.check_cancelled()?;
    let effective_ocean_water_m3 = decoded.ocean_volume_at(solved.sea_level_mm)?;
    Ok(DerivedHistoricalWorld {
        metrics: HistoricalMetrics {
            derivation_version: HISTORICAL_DERIVATION_VERSION,
            epoch_offset_years,
            normalized_epoch: solved.normalized_epoch,
            temperature_offset_centi_c: solved.temperature_offset_centi_c,
            lagged_temperature_offset_centi_c: solved.lagged_temperature_offset_centi_c,
            land_ice_equilibrium_m3: solved.land_ice_equilibrium_m3,
            land_ice_m3: solved.land_ice_m3,
            thermal_expansion_m3: solved.thermal_expansion_m3,
            effective_ocean_water_m3,
            conserved_water_m3: solved.target_ocean_water_m3,
            balance_error_m3: effective_ocean_water_m3.abs_diff(solved.target_ocean_water_m3),
            sea_level_mm: solved.sea_level_mm,
        },
        products,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

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

    pub(super) fn extreme_epochs(parameters: HistoricalForcingParameters) -> (i64, i64) {
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
            land_ice_volume(
                1_000_000_000,
                temperature_offset_centi_c(cold_epoch, parameters),
                parameters
            ) > land_ice_volume(
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
}

#[cfg(test)]
mod e4_tests {
    use super::*;
    use crate::derived_cache::StaticDerivedPhysics;
    use crate::engine::{self, WorldPreset};
    use crate::planetary::PlanetaryConfiguration;
    use crate::{NoopProgress, DEFAULT_RADIUS_METRES};

    fn engine_products(resolution: u32) -> engine::Products {
        let settings = engine::Settings::from_parts(
            831_429,
            WorldPreset::Earthlike,
            resolution,
            DEFAULT_RADIUS_METRES,
            290_000,
            PlanetaryConfiguration::earth_like(),
        );
        engine::generate(settings, &mut NoopProgress).unwrap()
    }

    fn present(source: &[u8]) -> StaticDerivedPhysics {
        let decoded = engine::decode_source(source).unwrap();
        let mut progress = NoopProgress;
        let derived = decoded
            .derive_products(PlanetaryConfiguration::earth_like(), &mut progress)
            .unwrap();
        StaticDerivedPhysics::new(
            derived.view,
            derived.climate,
            derived.hydrology,
            derived.hazards,
            derived.derived_geojson,
            derived.hashes,
        )
    }

    #[test]
    fn epoch_zero_equals_the_present_products_byte_for_byte() {
        let products = engine_products(32);
        let decoded = engine::decode_source(products.source()).unwrap();
        let physics = present(products.source());
        let forcing = HistoricalForcingParameters::default_for_seed(831_429);
        let mut progress = NoopProgress;
        let historical = derive_historical_products(
            &decoded,
            &physics,
            forcing,
            0,
            PlanetaryConfiguration::earth_like(),
            &mut progress,
        )
        .unwrap();
        assert_eq!(historical.products.view, physics.view);
        assert_eq!(historical.products.climate, physics.climate);
        assert_eq!(historical.products.hydrology, physics.hydrology);
        assert_eq!(historical.products.hazards, physics.hazards);
        assert_eq!(historical.products.derived_geojson, physics.geojson);
        assert_eq!(historical.metrics.sea_level_mm, 0);
        assert_eq!(historical.metrics.land_ice_m3, 0);
        assert_eq!(historical.metrics.thermal_expansion_m3, 0);
        assert_eq!(historical.metrics.balance_error_m3, 0);
    }

    #[test]
    fn epoch_sea_level_is_conserved_within_one_millimetre_of_ocean_volume() {
        let products = engine_products(32);
        let decoded = engine::decode_source(products.source()).unwrap();
        let physics = present(products.source());
        let forcing = HistoricalForcingParameters::default_for_seed(831_429);
        let (cold_epoch, warm_epoch) = super::tests::extreme_epochs(forcing);
        for epoch in [cold_epoch, warm_epoch] {
            let mut progress = NoopProgress;
            let historical = derive_historical_products(
                &decoded,
                &physics,
                forcing,
                epoch,
                PlanetaryConfiguration::earth_like(),
                &mut progress,
            )
            .unwrap();
            let volume = decoded
                .ocean_volume_at(historical.metrics.sea_level_mm)
                .unwrap();
            assert_eq!(historical.metrics.effective_ocean_water_m3, volume);
            let target = historical.metrics.conserved_water_m3 as i128;
            let one_mm_volume = (4.0
                * std::f64::consts::PI
                * (DEFAULT_RADIUS_METRES as f64).powi(2)
                * 0.001)
                .ceil() as u128;
            assert!(
                (volume as i128 - target).unsigned_abs() <= one_mm_volume,
                "epoch {epoch}: volume {volume}, target {target}"
            );
            let expected_target = i128::from(decoded.reference_water_inventory_m3())
                - i128::from(historical.metrics.land_ice_m3)
                + i128::from(historical.metrics.thermal_expansion_m3);
            assert_eq!(target, expected_target);
            assert_eq!(
                historical.metrics.sea_level_mm,
                historical.products.view.sea_level_mm
            );
            assert_eq!(
                historical.metrics.temperature_offset_centi_c,
                temperature_offset_centi_c(epoch, forcing)
            );
        }
    }

    #[test]
    fn land_ice_change_is_signed_relative_to_epoch_zero() {
        let products = engine_products(32);
        let decoded = engine::decode_source(products.source()).unwrap();
        let physics = present(products.source());
        let forcing = HistoricalForcingParameters::default_for_seed(831_429);
        let (cold_epoch, warm_epoch) = super::tests::extreme_epochs(forcing);
        let mut progress = NoopProgress;
        let cold = derive_historical_products(
            &decoded,
            &physics,
            forcing,
            cold_epoch,
            PlanetaryConfiguration::earth_like(),
            &mut progress,
        )
        .unwrap();
        let warm = derive_historical_products(
            &decoded,
            &physics,
            forcing,
            warm_epoch,
            PlanetaryConfiguration::earth_like(),
            &mut progress,
        )
        .unwrap();
        assert!(cold.metrics.land_ice_m3 > 0);
        assert!(warm.metrics.land_ice_m3 < 0);
        assert!(cold.metrics.sea_level_mm < warm.metrics.sea_level_mm);
    }

    #[test]
    #[ignore = "wall-clock: release-only epoch cancellation sweep"]
    fn epoch_cancellation_stays_under_the_deadline_budget() {
        struct DeadlineSink {
            deadline: std::time::Instant,
            armed: bool,
        }
        impl crate::ProgressSink for DeadlineSink {
            fn report(
                &mut self,
                _phase: crate::ProgressPhase,
                _completed: u32,
                _total: u32,
            ) -> Result<(), PhysicalError> {
                if std::time::Instant::now() >= self.deadline {
                    self.armed = true;
                }
                Ok(())
            }

            fn check_cancelled(&self) -> Result<(), PhysicalError> {
                if self.armed || std::time::Instant::now() >= self.deadline {
                    Err(PhysicalError::Cancelled)
                } else {
                    Ok(())
                }
            }
        }
        let products = engine_products(512);
        let decoded = engine::decode_source(products.source()).unwrap();
        let physics = present(products.source());
        let forcing = HistoricalForcingParameters::default_for_seed(831_429);
        let epoch = forcing.period_years() / 4;
        let planet = PlanetaryConfiguration::earth_like();
        let started = std::time::Instant::now();
        derive_historical_products(
            &decoded,
            &physics,
            forcing,
            epoch,
            planet,
            &mut NoopProgress,
        )
        .unwrap();
        let total = started.elapsed();
        let mut worst = std::time::Duration::ZERO;
        let mut deadline_ms = 25;
        while std::time::Duration::from_millis(deadline_ms) < total {
            let started = std::time::Instant::now();
            let mut sink = DeadlineSink {
                deadline: started + std::time::Duration::from_millis(deadline_ms),
                armed: false,
            };
            let result =
                derive_historical_products(&decoded, &physics, forcing, epoch, planet, &mut sink);
            if !matches!(result, Err(PhysicalError::Cancelled)) {
                deadline_ms += 100;
                continue;
            }
            let late = started
                .elapsed()
                .saturating_sub(std::time::Duration::from_millis(deadline_ms));
            worst = worst.max(late);
            deadline_ms += 100;
        }
        println!("epoch total {total:?}; worst late {worst:?}");
        assert!(
            worst.as_millis() < crate::CANCELLATION_LATENCY_BUDGET_MS,
            "epoch cancellation was {worst:?} late"
        );
    }
}
