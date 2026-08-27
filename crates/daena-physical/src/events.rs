//! Bounded, deterministic natural-event sampling from the generated hazard fields.
//!
//! This module only produces an explicit materialization result. It does not
//! persist anything and it does not modify the accepted physical source.
//! Event rates are events per physical model year.

use serde::{Deserialize, Serialize};

use super::{
    hazards::{self, HazardField, RATE_NANO, VOLCANIC_SOURCE_DERIVATION_VERSION},
    splitmix64,
    tectonics::TectonicWorld,
    Grid,
};

pub const EVENT_MATERIALIZATION_VERSION: u16 = 1;
pub const MAX_INTERVAL_OFFSET_YEARS: i64 = 100_000;
pub const MAX_EVENTS: u32 = 128;
const MAX_EXPECTED_EVENTS: f64 = 1_024.0;
const EARTHQUAKE_MIN_MAGNITUDE_MILLI: u32 = 4_000;
const EARTHQUAKE_MAX_MAGNITUDE_MILLI: u32 = 8_500;
const GUTENBERG_RICHTER_B_VALUE: f64 = 0.9;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum NaturalEventKind {
    Earthquake,
    Eruption,
}

impl NaturalEventKind {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Earthquake => "earthquake",
            Self::Eruption => "eruption",
        }
    }

    fn tag(self) -> u64 {
        match self {
            Self::Earthquake => 0x6561_7274_6871_0001,
            Self::Eruption => 0x6572_7570_7469_0002,
        }
    }

    pub const fn model_label(self) -> &'static str {
        match self {
            Self::Earthquake => "poisson-gutenberg-richter-v2",
            Self::Eruption => "persistent-rate-v2",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EventMaterializationRequest {
    pub event_kind: NaturalEventKind,
    pub interval_start_years: i64,
    pub interval_end_years: i64,
    pub max_events: u32,
    /// This seed is intentionally supplied separately from the geography seed.
    pub hazard_seed: u64,
}

impl EventMaterializationRequest {
    pub fn validate(&self) -> Result<(), String> {
        if self.interval_start_years < -MAX_INTERVAL_OFFSET_YEARS
            || self.interval_end_years > MAX_INTERVAL_OFFSET_YEARS
        {
            return Err(format!(
                "event interval must stay within +/-{MAX_INTERVAL_OFFSET_YEARS} years"
            ));
        }
        if self.interval_start_years > self.interval_end_years {
            return Err("event interval start cannot be after its end".into());
        }
        if !(1..=MAX_EVENTS).contains(&self.max_events) {
            return Err(format!("maxEvents must be between 1 and {MAX_EVENTS}"));
        }
        Ok(())
    }

    fn interval_length(&self) -> u64 {
        (self.interval_end_years as i128 - self.interval_start_years as i128 + 1) as u64
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct MaterializedEvent {
    pub event_kind: NaturalEventKind,
    pub ordinal: u32,
    pub year_offset: i64,
    pub cell: u32,
    pub longitude_microdegrees: i32,
    pub latitude_microdegrees: i32,
    pub magnitude_milli: u32,
    pub hazard_ppm: u32,
    pub annual_rate_nano: u64,
    pub rate_per_million_years_ppm: u32,
    pub sampled_center_id: Option<u32>,
    pub volcanic_source_derivation_version: u16,
}

fn uniform_open01(seed: u64) -> f64 {
    ((splitmix64(seed) >> 11) as f64 + 1.0) / ((1u64 << 53) as f64 + 1.0)
}

fn poisson(lambda: f64, seed: u64) -> u32 {
    let lambda = lambda.clamp(0.0, MAX_EXPECTED_EVENTS);
    if lambda == 0.0 {
        return 0;
    }
    let threshold = uniform_open01(seed);
    let mut probability = (-lambda).exp();
    let mut cumulative = probability;
    let mut count = 0u32;
    while threshold > cumulative && count < (MAX_EXPECTED_EVENTS as u32 * 2) {
        count += 1;
        probability *= lambda / f64::from(count);
        cumulative += probability;
    }
    count
}

fn center_microdegrees(grid: Grid, cell: usize) -> [i32; 2] {
    let (longitude, latitude) = grid.center_radians(grid.row_col(cell).0, grid.row_col(cell).1);
    [
        (longitude.to_degrees() * 1_000_000.0).round() as i32,
        (latitude.to_degrees() * 1_000_000.0).round() as i32,
    ]
}

fn earthquake_magnitude_milli(seed: u64) -> u32 {
    let u = uniform_open01(seed);
    let magnitude = f64::from(EARTHQUAKE_MIN_MAGNITUDE_MILLI)
        + (-u.ln() / (GUTENBERG_RICHTER_B_VALUE * 10.0_f64.ln()) * 1_000.0);
    magnitude.round().clamp(
        f64::from(EARTHQUAKE_MIN_MAGNITUDE_MILLI),
        f64::from(EARTHQUAKE_MAX_MAGNITUDE_MILLI),
    ) as u32
}

fn eruption_magnitude_milli(annual_rate_nano: u64, seed: u64) -> u32 {
    let variability = (splitmix64(seed) % 401) as u32;
    1_000 + (annual_rate_nano / 20).min(2_000) as u32 + variability
}

fn sample_cell(rates: &[u64], mass: u64, seed: u64) -> usize {
    if mass == 0 {
        return 0;
    }
    let mut remaining = (splitmix64(seed) % mass) + 1;
    for (cell, rate) in rates.iter().enumerate() {
        if *rate == 0 {
            continue;
        }
        if remaining <= *rate {
            return cell;
        }
        remaining -= *rate;
    }
    rates.len().saturating_sub(1)
}

/// Samples occurrences from the complete annual-rate field. `N ~ Poisson(Λ)`
/// with `Λ = total_annual_rate * years`, then each location is drawn from the
/// normalized rate mass. `maxEvents` only truncates the persisted list.
pub fn sample_events(
    world: &TectonicWorld,
    hazards: &HazardField,
    request: &EventMaterializationRequest,
) -> Result<Vec<MaterializedEvent>, String> {
    request.validate()?;
    hazards
        .validate(world.grid)
        .map_err(|error| error.to_string())?;
    let rates = match request.event_kind {
        NaturalEventKind::Earthquake => &hazards.earthquake_annual_rate_nano,
        NaturalEventKind::Eruption => &hazards.volcanic_annual_rate_nano,
    };
    let hazard_values = match request.event_kind {
        NaturalEventKind::Earthquake => &hazards.earthquake_hazard_ppm,
        NaturalEventKind::Eruption => &hazards.volcanic_hazard_ppm,
    };
    let million_year_rates = match request.event_kind {
        NaturalEventKind::Earthquake => &hazards.earthquake_rate_per_million_years_ppm,
        NaturalEventKind::Eruption => &hazards.eruption_rate_per_million_years_ppm,
    };
    let interval_length = request.interval_length();
    let mass = rates.iter().copied().fold(0u64, u64::saturating_add);
    let lambda = (mass as f64 / RATE_NANO as f64) * interval_length as f64;
    let count = poisson(
        lambda,
        request.hazard_seed ^ request.event_kind.tag() ^ 0x506f_6973_736f_6e31,
    )
    .min(request.max_events);
    let mut events = Vec::with_capacity(count as usize);
    for ordinal in 0..count {
        let event_seed = request.hazard_seed
            ^ request.event_kind.tag()
            ^ u64::from(ordinal).wrapping_mul(0x9e37_79b9_7f4a_7c15);
        let cell = sample_cell(rates, mass, event_seed ^ 0x6c6f_6361_7469_6f6e);
        let point = center_microdegrees(world.grid, cell);
        let year = request.interval_start_years
            + (splitmix64(event_seed ^ 0x7469_6d65) % interval_length) as i64;
        let magnitude_milli = match request.event_kind {
            NaturalEventKind::Earthquake => earthquake_magnitude_milli(event_seed ^ 0x4551),
            NaturalEventKind::Eruption => {
                eruption_magnitude_milli(rates[cell], event_seed ^ 0x4552)
            }
        };
        let sampled_center_id = match request.event_kind {
            NaturalEventKind::Eruption => {
                hazards::nearest_volcanic_source(hazards, cell).map(|source| source.stable_id)
            }
            NaturalEventKind::Earthquake => None,
        };
        events.push(MaterializedEvent {
            event_kind: request.event_kind,
            ordinal,
            year_offset: year,
            cell: cell as u32,
            longitude_microdegrees: point[0],
            latitude_microdegrees: point[1],
            magnitude_milli,
            hazard_ppm: hazard_values[cell],
            annual_rate_nano: rates[cell],
            rate_per_million_years_ppm: million_year_rates[cell],
            sampled_center_id,
            volcanic_source_derivation_version: VOLCANIC_SOURCE_DERIVATION_VERSION,
        });
    }
    Ok(events)
}

pub fn expected_lambda(hazards: &HazardField, request: &EventMaterializationRequest) -> f64 {
    let rates = match request.event_kind {
        NaturalEventKind::Earthquake => &hazards.earthquake_annual_rate_nano,
        NaturalEventKind::Eruption => &hazards.volcanic_annual_rate_nano,
    };
    let mass = rates.iter().copied().fold(0u64, u64::saturating_add);
    (mass as f64 / RATE_NANO as f64) * request.interval_length() as f64
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{generate_world, GenerationSettings, NoopProgress, DEFAULT_RADIUS_METRES};

    fn fixture() -> (TectonicWorld, HazardField) {
        let settings = GenerationSettings {
            width: 16,
            height: 8,
            radius_metres: DEFAULT_RADIUS_METRES,
            target_land_fraction_ppm: 300_000,
        };
        let mut progress = NoopProgress;
        let world = generate_world(settings, 831_429, 0, &mut progress)
            .unwrap()
            .tectonics;
        let hazards = crate::hazards::derive_hazards(&world).unwrap();
        (world, hazards)
    }

    fn request(
        kind: NaturalEventKind,
        years: i64,
        seed: u64,
        max_events: u32,
    ) -> EventMaterializationRequest {
        EventMaterializationRequest {
            event_kind: kind,
            interval_start_years: 0,
            interval_end_years: years.saturating_sub(1).max(0),
            max_events,
            hazard_seed: seed,
        }
    }

    #[test]
    fn request_bounds_are_explicit() {
        let mut request = EventMaterializationRequest {
            event_kind: NaturalEventKind::Earthquake,
            interval_start_years: -100_000,
            interval_end_years: 100_000,
            max_events: MAX_EVENTS,
            hazard_seed: 42,
        };
        assert!(request.validate().is_ok());
        request.max_events = MAX_EVENTS + 1;
        assert!(request.validate().is_err());
    }

    #[test]
    fn event_sampling_is_deterministic_bounded_and_seam_safe() {
        let (world, hazards) = fixture();
        let request = EventMaterializationRequest {
            event_kind: NaturalEventKind::Earthquake,
            interval_start_years: -100_000,
            interval_end_years: 100_000,
            max_events: MAX_EVENTS,
            hazard_seed: 7_331,
        };
        let first = sample_events(&world, &hazards, &request).unwrap();
        let second = sample_events(&world, &hazards, &request).unwrap();
        assert_eq!(first, second);
        assert!(first.len() <= MAX_EVENTS as usize);
        assert!(first.iter().all(|event| {
            (-180_000_000..=180_000_000).contains(&event.longitude_microdegrees)
                && (-90_000_000..=90_000_000).contains(&event.latitude_microdegrees)
                && (EARTHQUAKE_MIN_MAGNITUDE_MILLI..=EARTHQUAKE_MAX_MAGNITUDE_MILLI)
                    .contains(&event.magnitude_milli)
                && event.sampled_center_id.is_none()
        }));
    }

    #[test]
    fn eruptions_use_persistent_rates_and_distinct_model_label() {
        let (world, hazards) = fixture();
        let request = EventMaterializationRequest {
            event_kind: NaturalEventKind::Eruption,
            interval_start_years: -100_000,
            interval_end_years: 100_000,
            max_events: MAX_EVENTS,
            hazard_seed: 7_331,
        };
        let events = sample_events(&world, &hazards, &request).unwrap();
        assert_eq!(
            NaturalEventKind::Eruption.model_label(),
            "persistent-rate-v2"
        );
        assert!(events.iter().all(|event| {
            event.event_kind == NaturalEventKind::Eruption
                && event.magnitude_milli >= 1_000
                && event.volcanic_source_derivation_version == VOLCANIC_SOURCE_DERIVATION_VERSION
        }));
    }

    #[test]
    fn display_cap_does_not_change_event_samples() {
        let (world, hazards) = fixture();
        let request = request(NaturalEventKind::Earthquake, 80_000, 99, 32);
        let first = sample_events(&world, &hazards, &request).unwrap();
        let limited = crate::hazards::to_geojson_with_limit(&world, &hazards, 4).unwrap();
        let second = sample_events(&world, &hazards, &request).unwrap();
        assert_eq!(first, second);
        assert!(limited.contains("earthquake-hazard"));
    }

    #[test]
    fn poisson_mean_tracks_rate_mass_lambda() {
        let (world, hazards) = fixture();
        let request = request(NaturalEventKind::Earthquake, 20_000, 1, MAX_EVENTS);
        let lambda = expected_lambda(&hazards, &request);
        assert!(lambda > 0.0);
        let mut total = 0u64;
        let ensemble = 48u32;
        for seed in 0..ensemble {
            let mut seeded = request.clone();
            seeded.hazard_seed = u64::from(seed + 11);
            total += sample_events(&world, &hazards, &seeded).unwrap().len() as u64;
        }
        let mean = total as f64 / f64::from(ensemble);
        assert!((mean - lambda.min(MAX_EVENTS as f64)).abs() < lambda.max(1.0) * 0.75 + 1.5);
    }

    #[test]
    fn magnitude_frequency_prefers_smaller_earthquakes() {
        let (world, hazards) = fixture();
        let request = EventMaterializationRequest {
            event_kind: NaturalEventKind::Earthquake,
            interval_start_years: 0,
            interval_end_years: 100_000,
            max_events: MAX_EVENTS,
            hazard_seed: 17,
        };
        let mut small = 0u32;
        let mut large = 0u32;
        for seed in 0..24 {
            let mut seeded = request.clone();
            seeded.hazard_seed = 100 + seed;
            for event in sample_events(&world, &hazards, &seeded).unwrap() {
                if event.magnitude_milli < 5_500 {
                    small += 1;
                } else {
                    large += 1;
                }
            }
        }
        assert!(small + large > 0);
        assert!(small >= large);
    }

    #[test]
    fn rate_mass_used_for_sampling_matches_the_field() {
        let (_world, hazards) = fixture();
        let mass: u64 = hazards
            .earthquake_annual_rate_nano
            .iter()
            .copied()
            .fold(0u64, u64::saturating_add);
        assert_eq!(mass, hazards.metrics.earthquake_rate_mass_nano);
    }
}
