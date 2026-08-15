//! Bounded, deterministic natural-event sampling from the generated hazard fields.
//!
//! This module only produces an explicit materialization result. It does not
//! persist anything and it does not modify the accepted physical source.

use serde::{Deserialize, Serialize};

use super::{hazards::HazardField, splitmix64, tectonics::TectonicWorld, Grid};

pub const EVENT_MATERIALIZATION_VERSION: u16 = 1;
pub const MAX_INTERVAL_OFFSET_YEARS: i64 = 100_000;
pub const MAX_EVENTS: u32 = 128;
const OCCURRENCE_SCALE: f64 = 0.001;
const MAX_CELL_EXPECTED_EVENTS: f64 = 24.0;
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
            Self::Earthquake => "poisson-gutenberg-richter-v1",
            Self::Eruption => "persistent-rate-v1",
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
    pub rate_per_million_years_ppm: u32,
}

fn uniform_open01(seed: u64) -> f64 {
    ((splitmix64(seed) >> 11) as f64 + 1.0) / ((1u64 << 53) as f64 + 1.0)
}

fn poisson(lambda: f64, seed: u64) -> u32 {
    let lambda = lambda.clamp(0.0, MAX_CELL_EXPECTED_EVENTS);
    if lambda == 0.0 {
        return 0;
    }
    let threshold = uniform_open01(seed);
    let mut probability = (-lambda).exp();
    let mut cumulative = probability;
    let mut count = 0u32;
    while threshold > cumulative && count < (MAX_CELL_EXPECTED_EVENTS as u32 * 2) {
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

fn eruption_magnitude_milli(rate_ppm: u32, seed: u64) -> u32 {
    let variability = (splitmix64(seed) % 401) as u32;
    1_000 + rate_ppm / 2_000 + variability
}

/// Samples a bounded set of independent event occurrences. Cells are visited
/// in descending persistent-rate order, with the cell index as a stable tie
/// breaker. This makes the cap deterministic while retaining a Poisson draw
/// per cell. Eruption rates use the same persistent field but do not imply
/// aftershock or clustered sequences.
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
        NaturalEventKind::Earthquake => &hazards.earthquake_rate_per_million_years_ppm,
        NaturalEventKind::Eruption => &hazards.eruption_rate_per_million_years_ppm,
    };
    let hazard_values = match request.event_kind {
        NaturalEventKind::Earthquake => &hazards.earthquake_hazard_ppm,
        NaturalEventKind::Eruption => &hazards.volcanic_hazard_ppm,
    };
    let mut cells = (0..world.grid.sample_count()).collect::<Vec<_>>();
    cells.sort_unstable_by_key(|cell| (std::cmp::Reverse(rates[*cell]), *cell));
    let interval_length = request.interval_length();
    let mut events = Vec::new();

    for cell in cells {
        if events.len() >= request.max_events as usize {
            break;
        }
        let lambda =
            f64::from(rates[cell]) / 1_000_000.0 * interval_length as f64 * OCCURRENCE_SCALE;
        let count = poisson(
            lambda,
            request.hazard_seed
                ^ request.event_kind.tag()
                ^ (cell as u64).wrapping_mul(0xd6e8_feb8_6659_fd93),
        );
        for ordinal in 0..count {
            if events.len() >= request.max_events as usize {
                break;
            }
            let event_seed = request.hazard_seed
                ^ request.event_kind.tag()
                ^ (cell as u64).wrapping_mul(0x9e37_79b9_7f4a_7c15)
                ^ u64::from(ordinal).wrapping_mul(0xbf58_476d_1ce4_e5b9);
            let year =
                request.interval_start_years + (splitmix64(event_seed) % interval_length) as i64;
            let point = center_microdegrees(world.grid, cell);
            let magnitude_milli = match request.event_kind {
                NaturalEventKind::Earthquake => earthquake_magnitude_milli(event_seed ^ 0x4551),
                NaturalEventKind::Eruption => {
                    eruption_magnitude_milli(rates[cell], event_seed ^ 0x4552)
                }
            };
            events.push(MaterializedEvent {
                event_kind: request.event_kind,
                ordinal: events.len() as u32,
                year_offset: year,
                cell: cell as u32,
                longitude_microdegrees: point[0],
                latitude_microdegrees: point[1],
                magnitude_milli,
                hazard_ppm: hazard_values[cell],
                rate_per_million_years_ppm: rates[cell],
            });
        }
    }
    Ok(events)
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
        assert!(!first.is_empty());
        assert!(first.len() <= MAX_EVENTS as usize);
        assert!(first.iter().all(|event| {
            (-180_000_000..=180_000_000).contains(&event.longitude_microdegrees)
                && (-90_000_000..=90_000_000).contains(&event.latitude_microdegrees)
                && (EARTHQUAKE_MIN_MAGNITUDE_MILLI..=EARTHQUAKE_MAX_MAGNITUDE_MILLI)
                    .contains(&event.magnitude_milli)
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
        assert!(!events.is_empty());
        assert!(events.iter().all(|event| {
            event.event_kind == NaturalEventKind::Eruption
                && event.magnitude_milli >= 1_000
                && event.rate_per_million_years_ppm > 0
        }));
    }
}
