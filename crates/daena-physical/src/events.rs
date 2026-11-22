//! Bounded, deterministic natural-event sampling from the engine world.
//!
//! This module only produces an explicit materialization result. It does not
//! persist anything and it does not modify the accepted physical source.
//! Event rates are events per physical model year.
//!
//! Every cell in a [`MaterializedEvent`] is a canonical cube cell. Cells are
//! never view or coarse-climate cells: the storm tracer works on coarse cells,
//! so each traced step is mapped to the canonical cell that contains it.
//!
//! Earthquakes draw from the engine M ≥ 4 canonical rate mass, eruptions draw
//! a volcanic source, storms draw from the engine genesis census and trace with
//! the shared tracer, and floods list engine lakes that spill. The notable
//! storm rate is a product policy applied to the physical census, not a hazard
//! field.

use daena_worldgen as worldgen;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use super::{
    products::{
        earthquake_rate_nano, fraction_ppm, hazard_ppm, rate_per_million_years_ppm,
        CLIMATE_DERIVATION_VERSION, HYDROLOGY_DERIVATION_VERSION,
        VOLCANIC_SOURCE_DERIVATION_VERSION,
    },
    splitmix64, PhysicalError,
};

pub const EVENT_MATERIALIZATION_VERSION: u16 = 3;
pub const MAX_INTERVAL_OFFSET_YEARS: i64 = 100_000;
/// Nano-units per event or per year.
const RATE_NANO: u64 = 1_000_000_000;
pub const MAX_EVENTS: u32 = 128;
const MAX_EXPECTED_EVENTS: f64 = 1_024.0;
/// Earthlike seed 831429, `n = 512`: the retired 384×192 product generator
/// reports `mean_ocean_storm_suitability_ppm = 55_708`, so its notable rate is
/// `55_708 / 1e6 · 0.00035 = 1.94978e-5` events per year. The engine genesis
/// census for the same seed and resolution is 60.219666 storms per year, so
/// `1.94978e-5 / 60.219666 = 3.23778e-7`. This constant keeps that rate within
/// 10 % of the retired product on the calibration world.
const STORM_NOTABLE_FRACTION: f64 = 3.24e-7;
const STORM_MONTH_SEED_TAG: u64 = 0x6d6f_6e74_6800_0005;
const EARTHQUAKE_MIN_MAGNITUDE_MILLI: u32 = 4_000;
const EARTHQUAKE_MAX_MAGNITUDE_MILLI: u32 = 8_500;
const GUTENBERG_RICHTER_B_VALUE: f64 = 0.9;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum NaturalEventKind {
    Earthquake,
    Eruption,
    Storm,
    Flood,
}

impl NaturalEventKind {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Earthquake => "earthquake",
            Self::Eruption => "eruption",
            Self::Storm => "storm",
            Self::Flood => "flood",
        }
    }

    fn tag(self) -> u64 {
        match self {
            Self::Earthquake => 0x6561_7274_6871_0001,
            Self::Eruption => 0x6572_7570_7469_0002,
            Self::Storm => 0x7374_6f72_6d00_0003,
            Self::Flood => 0x666c_6f6f_6400_0004,
        }
    }

    pub const fn model_label(self) -> &'static str {
        match self {
            Self::Earthquake => "poisson-gutenberg-richter-v2",
            Self::Eruption => "engine-volcanic-source-v3",
            Self::Storm => "engine-storm-tracer-v3",
            Self::Flood => "engine-lake-spill-v3",
        }
    }

    pub const fn materialization_version(self) -> u16 {
        match self {
            Self::Earthquake | Self::Eruption => 2,
            Self::Storm | Self::Flood => 3,
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

    #[must_use]
    pub fn interval_midpoint_years(&self) -> i64 {
        ((self.interval_start_years as i128 + self.interval_end_years as i128) / 2) as i64
    }
}

/// Events plus the Poisson mean used for their draw.
#[derive(Debug, Clone, PartialEq)]
pub struct EventSample {
    pub events: Vec<MaterializedEvent>,
    pub expected_lambda: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct MaterializedEvent {
    pub event_kind: NaturalEventKind,
    pub ordinal: u32,
    pub year_offset: i64,
    /// Canonical cube cell.
    pub cell: u32,
    pub longitude_microdegrees: i32,
    pub latitude_microdegrees: i32,
    pub magnitude_milli: u32,
    pub hazard_ppm: u32,
    pub annual_rate_nano: u64,
    pub rate_per_million_years_ppm: u32,
    #[serde(
        default,
        serialize_with = "serialize_optional_decimal_u64",
        deserialize_with = "deserialize_optional_decimal_u64"
    )]
    pub sampled_center_id: Option<u64>,
    pub volcanic_source_derivation_version: u16,
    #[serde(default, skip_serializing_if = "u16_is_zero")]
    pub hydrology_derivation_version: u16,
    #[serde(default)]
    pub track_cells: Vec<u32>,
}

fn u16_is_zero(value: &u16) -> bool {
    *value == 0
}

fn serialize_optional_decimal_u64<S>(value: &Option<u64>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    match value {
        Some(id) => serializer.serialize_some(&id.to_string()),
        None => serializer.serialize_none(),
    }
}

fn deserialize_optional_decimal_u64<'de, D>(deserializer: D) -> Result<Option<u64>, D::Error>
where
    D: Deserializer<'de>,
{
    let text = Option::<String>::deserialize(deserializer)?;
    text.map(|value| {
        value
            .parse::<u64>()
            .map_err(|_| serde::de::Error::custom("sampledCenterId must be a decimal u64 string"))
    })
    .transpose()
}

#[must_use]
pub fn flood_materialization_key(
    map_entity_id: &str,
    identity: &str,
    hydrology_derivation_version: u16,
    minimum_cell: u32,
    year_offset: i64,
) -> String {
    format!(
        "flood:{map_entity_id}:{identity}:{hydrology_derivation_version}:{minimum_cell}:{year_offset}"
    )
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

fn center_microdegrees(grid: &worldgen::CubeSphere, cell: u32) -> [i32; 2] {
    let (latitude, longitude) = grid.lat_lon(cell);
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

fn storm_magnitude_milli(intensity_ppm: u32, seed: u64) -> u32 {
    let variability = (splitmix64(seed) % 251) as u32;
    1_000 + intensity_ppm / 500 + variability
}

fn flood_magnitude_milli(outflow_m3_per_year: u64) -> u32 {
    let log = (outflow_m3_per_year as f64 + 1.0).log10();
    (1_000.0 + log * 500.0).round().clamp(1_000.0, 8_000.0) as u32
}

fn prefix_sums(weights: &[u64]) -> (Vec<u64>, u64) {
    let mut prefix = Vec::with_capacity(weights.len());
    let mut mass = 0u64;
    for weight in weights {
        mass = mass.saturating_add(*weight);
        prefix.push(mass);
    }
    (prefix, mass)
}

fn sample_prefix(prefix: &[u64], mass: u64, seed: u64) -> Option<usize> {
    if mass == 0 {
        return None;
    }
    let remaining = splitmix64(seed) % mass;
    let index = prefix.partition_point(|value| *value <= remaining);
    (index < prefix.len()).then_some(index)
}

fn sample_weighted(weights: &[u64], seed: u64) -> Option<usize> {
    let (prefix, mass) = prefix_sums(weights);
    sample_prefix(&prefix, mass, seed)
}

fn event_year(request: &EventMaterializationRequest, event_seed: u64) -> i64 {
    request.interval_start_years
        + (splitmix64(event_seed ^ 0x7469_6d65) % request.interval_length()) as i64
}

fn event_seed(request: &EventMaterializationRequest, ordinal: u32) -> u64 {
    request.hazard_seed
        ^ request.event_kind.tag()
        ^ u64::from(ordinal).wrapping_mul(0x9e37_79b9_7f4a_7c15)
}

/// Draw every event for one engine world.
///
/// The world is read only. The caller decides whether it is the present world
/// or an epoch world. `max_events` only truncates the returned list;
/// `expected_lambda` is the draw mean before that cap.
pub fn sample_world_events(
    world: &worldgen::World,
    request: &EventMaterializationRequest,
) -> Result<EventSample, PhysicalError> {
    request.validate().map_err(PhysicalError::Validation)?;
    match request.event_kind {
        NaturalEventKind::Earthquake => Ok(sample_earthquakes(world, request)),
        NaturalEventKind::Eruption => Ok(sample_eruptions(world, request)),
        NaturalEventKind::Storm => sample_storms(world, request),
        NaturalEventKind::Flood => Ok(sample_floods(world, request)),
    }
}

fn sample_earthquakes(
    world: &worldgen::World,
    request: &EventMaterializationRequest,
) -> EventSample {
    let mut rates = Vec::with_capacity(world.hazards.quake_rate_km2.len());
    let mut mass = 0u64;
    for (cell, rate) in world.hazards.quake_rate_km2.iter().enumerate() {
        let annual = earthquake_rate_nano(*rate, world.cell_area_m2(cell as u32));
        mass = mass.saturating_add(annual);
        rates.push(annual);
    }
    let (prefix, prefix_mass) = prefix_sums(&rates);
    debug_assert_eq!(mass, prefix_mass);
    let lambda = (mass as f64 / RATE_NANO as f64) * request.interval_length() as f64;
    let count = poisson(
        lambda,
        request.hazard_seed ^ request.event_kind.tag() ^ 0x506f_6973_736f_6e31,
    )
    .min(request.max_events);
    let mut events = Vec::with_capacity(count as usize);
    if mass == 0 {
        return EventSample {
            events,
            expected_lambda: lambda,
        };
    }
    for ordinal in 0..count {
        let event_seed = event_seed(request, ordinal);
        let Some(genesis) = sample_prefix(&prefix, mass, event_seed ^ 0x6c6f_6361_7469_6f6e) else {
            break;
        };
        let cell = u32::try_from(genesis).expect("canonical cell index fits in u32");
        debug_assert!((cell as usize) < world.grid.cell_count());
        let point = center_microdegrees(&world.grid, cell);
        events.push(MaterializedEvent {
            event_kind: request.event_kind,
            ordinal,
            year_offset: event_year(request, event_seed),
            cell,
            longitude_microdegrees: point[0],
            latitude_microdegrees: point[1],
            magnitude_milli: earthquake_magnitude_milli(event_seed ^ 0x4551),
            hazard_ppm: hazard_ppm(world.hazards.quake_hazard[genesis]),
            annual_rate_nano: rates[genesis],
            rate_per_million_years_ppm: rate_per_million_years_ppm(rates[genesis]),
            sampled_center_id: None,
            volcanic_source_derivation_version: VOLCANIC_SOURCE_DERIVATION_VERSION,
            hydrology_derivation_version: 0,
            track_cells: vec![cell],
        });
    }
    EventSample {
        events,
        expected_lambda: lambda,
    }
}

fn sample_eruptions(world: &worldgen::World, request: &EventMaterializationRequest) -> EventSample {
    let mut sources = world.hazards.volcanic_sources.iter().collect::<Vec<_>>();
    sources.sort_by_key(|source| source.id);
    let rates = sources
        .iter()
        .map(|source| {
            (f64::from(source.eruptions_per_yr) * RATE_NANO as f64)
                .round()
                .clamp(0.0, u64::MAX as f64) as u64
        })
        .collect::<Vec<_>>();
    let (prefix, mass) = prefix_sums(&rates);
    let lambda = (mass as f64 / RATE_NANO as f64) * request.interval_length() as f64;
    let count = poisson(
        lambda,
        request.hazard_seed ^ request.event_kind.tag() ^ 0x506f_6973_736f_6e31,
    )
    .min(request.max_events);
    let mut events = Vec::with_capacity(count as usize);
    if mass == 0 {
        return EventSample {
            events,
            expected_lambda: lambda,
        };
    }
    for ordinal in 0..count {
        let event_seed = event_seed(request, ordinal);
        let Some(source_index) = sample_prefix(&prefix, mass, event_seed ^ 0x6c6f_6361_7469_6f6e)
        else {
            break;
        };
        let source = sources[source_index];
        debug_assert!((source.cell as usize) < world.grid.cell_count());
        let point = center_microdegrees(&world.grid, source.cell);
        events.push(MaterializedEvent {
            event_kind: request.event_kind,
            ordinal,
            year_offset: event_year(request, event_seed),
            cell: source.cell,
            longitude_microdegrees: point[0],
            latitude_microdegrees: point[1],
            magnitude_milli: eruption_magnitude_milli(rates[source_index], event_seed ^ 0x4552),
            hazard_ppm: hazard_ppm(world.hazards.volcanic_hazard[source.cell as usize]),
            annual_rate_nano: rates[source_index],
            rate_per_million_years_ppm: rate_per_million_years_ppm(rates[source_index]),
            sampled_center_id: Some(source.id),
            volcanic_source_derivation_version: VOLCANIC_SOURCE_DERIVATION_VERSION,
            hydrology_derivation_version: 0,
            track_cells: vec![source.cell],
        });
    }
    EventSample {
        events,
        expected_lambda: lambda,
    }
}

fn storm_census(world: &worldgen::World, areas_km2: &[f64]) -> f64 {
    world
        .hazards
        .storms
        .genesis_km2
        .iter()
        .zip(areas_km2)
        .map(|(genesis, area)| f64::from(*genesis) * area)
        .sum()
}

fn sample_storms(
    world: &worldgen::World,
    request: &EventMaterializationRequest,
) -> Result<EventSample, PhysicalError> {
    let coarse = &world.climate_field.grid;
    let coarse_cells = coarse.cell_count();
    let storms = &world.hazards.storms;
    if storms.genesis_km2.len() != coarse_cells
        || storms.genesis_monthly_km2.len() != 12 * coarse_cells
    {
        return Err(PhysicalError::Validation(
            "storm climatology does not match the coarse climate grid".into(),
        ));
    }
    let areas_km2 = (0..coarse_cells)
        .map(|cell| coarse.area_m2(cell as u32) / 1.0e6)
        .collect::<Vec<_>>();
    let weights = storms
        .genesis_km2
        .iter()
        .zip(&areas_km2)
        .map(|(genesis, area)| {
            (f64::from(*genesis) * area * RATE_NANO as f64)
                .round()
                .clamp(0.0, u64::MAX as f64) as u64
        })
        .collect::<Vec<_>>();
    let (prefix, mass) = prefix_sums(&weights);
    let census = storm_census(world, &areas_km2);
    let lambda = (census * STORM_NOTABLE_FRACTION * request.interval_length() as f64)
        .clamp(0.0, MAX_EXPECTED_EVENTS);
    let count = poisson(
        lambda,
        request.hazard_seed ^ request.event_kind.tag() ^ 0x506f_6973_736f_6e31,
    )
    .min(request.max_events);
    let notable_nano = (census * STORM_NOTABLE_FRACTION * RATE_NANO as f64)
        .round()
        .clamp(0.0, u64::MAX as f64) as u64;
    let ocean_fraction = world.coarse_ocean_fraction();
    let mut events = Vec::with_capacity(count as usize);
    if mass == 0 {
        return Ok(EventSample {
            events,
            expected_lambda: lambda,
        });
    }
    for ordinal in 0..count {
        let event_seed = event_seed(request, ordinal);
        let Some(genesis) = sample_prefix(&prefix, mass, event_seed ^ 0x6c6f_6361_7469_6f6e) else {
            break;
        };
        let month_weights = (0..12)
            .map(|month| {
                let value = f64::from(storms.genesis_monthly_km2[month * coarse_cells + genesis]);
                (value * RATE_NANO as f64)
                    .round()
                    .clamp(0.0, u64::MAX as f64) as u64
            })
            .collect::<Vec<_>>();
        let month_mass = month_weights
            .iter()
            .copied()
            .fold(0u64, u64::saturating_add);
        if month_mass == 0 {
            return Err(PhysicalError::Validation(format!(
                "storm genesis cell {genesis} has no monthly genesis weights"
            )));
        }
        let Some(month) = sample_weighted(&month_weights, event_seed ^ STORM_MONTH_SEED_TAG) else {
            return Err(PhysicalError::Validation(
                "storm genesis month could not be sampled".into(),
            ));
        };
        let track = worldgen::trace_storm(
            &world.climate_field,
            &ocean_fraction,
            u32::try_from(genesis).expect("coarse cell index fits in u32"),
            u32::try_from(month).expect("month fits in u32"),
        )
        .map_err(|error| PhysicalError::Validation(error.to_string()))?;
        let mut track_cells = Vec::new();
        let mut maximum_intensity = 0.0f64;
        for step in &track.steps {
            maximum_intensity = maximum_intensity.max(step.intensity);
            let cell = world.grid.cell_at(coarse.position(step.cell));
            debug_assert!((cell as usize) < world.grid.cell_count());
            if track_cells.last() != Some(&cell) {
                track_cells.push(cell);
            }
        }
        let Some(&cell) = track_cells.last() else {
            return Err(PhysicalError::Validation(
                "storm tracer returned an empty track".into(),
            ));
        };
        let canonical_genesis = world.grid.cell_at(
            coarse.position(u32::try_from(genesis).expect("coarse cell index fits in u32")),
        );
        debug_assert!((canonical_genesis as usize) < world.grid.cell_count());
        let point = center_microdegrees(&world.grid, cell);
        let hazard_ppm = fraction_ppm(maximum_intensity);
        let annual_rate_nano = notable_nano.saturating_mul(weights[genesis]) / mass;
        events.push(MaterializedEvent {
            event_kind: request.event_kind,
            ordinal,
            year_offset: event_year(request, event_seed),
            cell,
            longitude_microdegrees: point[0],
            latitude_microdegrees: point[1],
            magnitude_milli: storm_magnitude_milli(hazard_ppm, event_seed ^ 0x4553),
            hazard_ppm,
            annual_rate_nano,
            rate_per_million_years_ppm: rate_per_million_years_ppm(annual_rate_nano),
            sampled_center_id: Some(u64::from(canonical_genesis)),
            volcanic_source_derivation_version: CLIMATE_DERIVATION_VERSION,
            hydrology_derivation_version: 0,
            track_cells,
        });
    }
    Ok(EventSample {
        events,
        expected_lambda: lambda,
    })
}

fn sample_floods(world: &worldgen::World, request: &EventMaterializationRequest) -> EventSample {
    let mut lakes = world
        .hydrology
        .lakes
        .iter()
        .filter(|lake| lake.kind == worldgen::LakeKind::Fresh && lake.outflow_m3 > 0.0)
        .collect::<Vec<_>>();
    lakes.sort_by_key(|lake| lake.minimum_cell);
    let expected_lambda = lakes.len() as f64;
    let year = request.interval_midpoint_years();
    let events = lakes
        .into_iter()
        .take(request.max_events as usize)
        .enumerate()
        .map(|(ordinal, lake)| {
            let cell = lake.spill_cell;
            debug_assert!((cell as usize) < world.grid.cell_count());
            debug_assert!((lake.minimum_cell as usize) < world.grid.cell_count());
            let point = center_microdegrees(&world.grid, cell);
            let outflow = lake.outflow_m3.round().clamp(0.0, u64::MAX as f64) as u64;
            MaterializedEvent {
                event_kind: request.event_kind,
                ordinal: ordinal as u32,
                year_offset: year,
                cell,
                longitude_microdegrees: point[0],
                latitude_microdegrees: point[1],
                magnitude_milli: flood_magnitude_milli(outflow),
                hazard_ppm: 1_000_000,
                annual_rate_nano: 0,
                rate_per_million_years_ppm: 0,
                sampled_center_id: Some(u64::from(lake.minimum_cell)),
                volcanic_source_derivation_version: 0,
                hydrology_derivation_version: HYDROLOGY_DERIVATION_VERSION,
                track_cells: if cell == lake.minimum_cell {
                    vec![cell]
                } else {
                    vec![lake.minimum_cell, cell]
                },
            }
        })
        .collect();
    EventSample {
        events,
        expected_lambda,
    }
}

#[cfg(test)]
mod tests {
    use std::sync::OnceLock;

    use super::*;
    use crate::NoopProgress;

    fn engine_world(n: u32, preset: worldgen::Preset) -> worldgen::World {
        let mut settings = preset.settings();
        settings.seed = 831_429;
        settings.resolution = n;
        worldgen::generate(&settings, &mut worldgen::NoProgress).unwrap()
    }

    fn earthlike_64() -> &'static worldgen::World {
        static WORLD: OnceLock<worldgen::World> = OnceLock::new();
        WORLD.get_or_init(|| engine_world(64, worldgen::Preset::Earthlike))
    }

    fn earthlike_128() -> &'static worldgen::World {
        static WORLD: OnceLock<worldgen::World> = OnceLock::new();
        WORLD.get_or_init(|| engine_world(128, worldgen::Preset::Earthlike))
    }

    fn request(
        kind: NaturalEventKind,
        start_years: i64,
        end_years: i64,
        hazard_seed: u64,
        max_events: u32,
    ) -> EventMaterializationRequest {
        EventMaterializationRequest {
            event_kind: kind,
            interval_start_years: start_years,
            interval_end_years: end_years,
            max_events,
            hazard_seed,
        }
    }

    fn event_center(world: &worldgen::World, cell: u32) -> (f64, f64) {
        let (latitude, longitude) = world.grid.lat_lon(cell);
        (latitude.to_degrees(), longitude.to_degrees())
    }

    fn angular_distance(world: &worldgen::World, a: u32, b: [f64; 3]) -> f64 {
        let first = world.grid.position(a);
        let dot = (first[0] * b[0] + first[1] * b[1] + first[2] * b[2]).clamp(-1.0, 1.0);
        dot.acos()
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
    fn earthquakes_are_deterministic_bounded_and_rate_massed() {
        let world = earthlike_64();
        let request = request(
            NaturalEventKind::Earthquake,
            -100_000,
            100_000,
            7_331,
            MAX_EVENTS,
        );
        let first = sample_world_events(world, &request).unwrap();
        let second = sample_world_events(world, &request).unwrap();
        assert_eq!(first, second);
        assert_eq!(
            serde_json::to_string(&first.events).unwrap(),
            serde_json::to_string(&second.events).unwrap()
        );
        assert!(first.events.len() <= MAX_EVENTS as usize);
        let mass = world
            .hazards
            .quake_rate_km2
            .iter()
            .enumerate()
            .map(|(cell, rate)| earthquake_rate_nano(*rate, world.cell_area_m2(cell as u32)))
            .fold(0u64, u64::saturating_add);
        let years = request.interval_length() as f64;
        assert!((first.expected_lambda - mass as f64 / RATE_NANO as f64 * years).abs() < 1e-9);
        assert!(first.expected_lambda > 0.0);
        for event in &first.events {
            assert_eq!(event.event_kind, NaturalEventKind::Earthquake);
            assert!(event.sampled_center_id.is_none());
            assert_eq!(event.track_cells, vec![event.cell]);
            assert!((event.cell as usize) < world.grid.cell_count());
            assert!((-180_000_000..=180_000_000).contains(&event.longitude_microdegrees));
            assert!((-90_000_000..=90_000_000).contains(&event.latitude_microdegrees));
            assert!(
                (EARTHQUAKE_MIN_MAGNITUDE_MILLI..=EARTHQUAKE_MAX_MAGNITUDE_MILLI)
                    .contains(&event.magnitude_milli)
            );
            assert!(event.annual_rate_nano > 0);
            assert_eq!(
                event.rate_per_million_years_ppm,
                rate_per_million_years_ppm(event.annual_rate_nano)
            );
            assert_eq!(
                event.hazard_ppm,
                hazard_ppm(world.hazards.quake_hazard[event.cell as usize])
            );
            let (latitude, longitude) = event_center(world, event.cell);
            assert_eq!(
                event.latitude_microdegrees,
                (latitude * 1_000_000.0).round() as i32
            );
            assert_eq!(
                event.longitude_microdegrees,
                (longitude * 1_000_000.0).round() as i32
            );
        }
    }

    #[test]
    fn poisson_mean_tracks_the_eruption_rate_mass() {
        let world = earthlike_64();
        let base = request(NaturalEventKind::Eruption, 0, 0, 1, MAX_EVENTS);
        let lambda = sample_world_events(world, &base).unwrap().expected_lambda;
        assert!(
            lambda > 0.5 && lambda < f64::from(MAX_EVENTS) / 2.0,
            "{lambda}"
        );
        let ensemble = 200u64;
        let total = (0..ensemble)
            .map(|seed| {
                let mut seeded = base.clone();
                seeded.hazard_seed = seed + 11;
                sample_world_events(world, &seeded).unwrap().events.len() as u64
            })
            .sum::<u64>();
        let mean = total as f64 / ensemble as f64;
        // Standard error of the mean is sqrt(lambda / ensemble).
        let tolerance = 4.0 * (lambda / ensemble as f64).sqrt() + 0.05;
        assert!(
            (mean - lambda).abs() <= tolerance,
            "mean {mean} vs lambda {lambda}"
        );
    }

    #[test]
    fn earthquake_magnitudes_follow_gutenberg_richter() {
        // b = 0.9: P(M >= 5 | M >= 4) = 10^-0.9, about 0.126.
        let draws = 20_000u64;
        let large = (0..draws)
            .filter(|seed| earthquake_magnitude_milli(seed.wrapping_mul(0x9e37_79b9)) >= 5_000)
            .count() as f64;
        let share = large / draws as f64;
        assert!((share - 10f64.powf(-0.9)).abs() < 0.01, "{share}");
    }

    #[test]
    fn earthquake_rate_converts_magnitude_five_to_four_through_the_product_helper() {
        let rate_m5_km2 = 0.013_880_39f32;
        let area_m2 = 1.2e10f64;
        let got = earthquake_rate_nano(rate_m5_km2, area_m2);
        let expected = (f64::from(rate_m5_km2)
            * (area_m2 / 1_000_000.0)
            * 10.0f64.powf(0.9)
            * RATE_NANO as f64)
            .round() as u64;
        assert_eq!(got, expected);
        let m5 = (f64::from(rate_m5_km2) * (area_m2 / 1_000_000.0) * RATE_NANO as f64).round();
        assert!((got as f64 / m5 - 7.943_282_347_242_816).abs() < 1e-9);
    }

    #[test]
    fn eruptions_sample_engine_sources_and_versions() {
        let world = earthlike_64();
        let request = request(
            NaturalEventKind::Eruption,
            -100_000,
            100_000,
            7_331,
            MAX_EVENTS,
        );
        let sample = sample_world_events(world, &request).unwrap();
        let second = sample_world_events(world, &request).unwrap();
        assert_eq!(
            serde_json::to_string(&sample.events).unwrap(),
            serde_json::to_string(&second.events).unwrap()
        );
        assert!(world
            .hazards
            .volcanic_sources
            .windows(2)
            .all(|pair| pair[0].id < pair[1].id));
        assert!(!sample.events.is_empty());
        let mass = world
            .hazards
            .volcanic_sources
            .iter()
            .map(|source| {
                (f64::from(source.eruptions_per_yr) * RATE_NANO as f64)
                    .round()
                    .clamp(0.0, u64::MAX as f64) as u64
            })
            .fold(0u64, u64::saturating_add);
        assert!(
            (sample.expected_lambda
                - mass as f64 / RATE_NANO as f64 * request.interval_length() as f64)
                .abs()
                < 1e-9
        );
        for event in &sample.events {
            let source = world
                .hazards
                .volcanic_sources
                .iter()
                .find(|source| Some(source.id) == event.sampled_center_id)
                .expect("eruption samples a volcanic source");
            assert_eq!(event.cell, source.cell);
            assert_eq!(event.track_cells, vec![source.cell]);
            assert_eq!(
                event.annual_rate_nano,
                (f64::from(source.eruptions_per_yr) * RATE_NANO as f64).round() as u64
            );
            assert_eq!(
                event.hazard_ppm,
                hazard_ppm(world.hazards.volcanic_hazard[source.cell as usize])
            );
            assert!(event.magnitude_milli >= 1_000);
            assert_eq!(
                event.volcanic_source_derivation_version,
                VOLCANIC_SOURCE_DERIVATION_VERSION
            );
        }
    }

    #[test]
    fn storms_use_canonical_cells_and_match_the_engine_tracer() {
        let world = earthlike_128();
        let request = request(NaturalEventKind::Storm, 0, 100_000, 42, MAX_EVENTS);
        let sample = sample_world_events(world, &request).unwrap();
        let second = sample_world_events(world, &request).unwrap();
        assert_eq!(
            serde_json::to_string(&sample.events).unwrap(),
            serde_json::to_string(&second.events).unwrap()
        );
        assert!(!sample.events.is_empty());
        let coarse = &world.climate_field.grid;
        assert_ne!(world.grid.n, coarse.n);
        let ocean_fraction = world.coarse_ocean_fraction();
        let coarse_spacing = std::f64::consts::FRAC_PI_2 / f64::from(coarse.n);
        let n_c = coarse.cell_count();
        for event in &sample.events {
            assert!(!event.track_cells.is_empty());
            assert_eq!(event.cell, *event.track_cells.last().unwrap());
            for cell in &event.track_cells {
                assert!((*cell as usize) < world.grid.cell_count());
            }
            let canonical_genesis =
                u32::try_from(event.sampled_center_id.unwrap()).expect("canonical cell id");
            assert!((canonical_genesis as usize) < world.grid.cell_count());
            let genesis = coarse.cell_at(world.grid.position(canonical_genesis));
            let genesis_canonical = world.grid.cell_at(coarse.position(genesis));
            assert_eq!(canonical_genesis, genesis_canonical);
            let month_weights = (0..12)
                .map(|month| {
                    let value = f64::from(
                        world.hazards.storms.genesis_monthly_km2[month * n_c + genesis as usize],
                    );
                    (value * RATE_NANO as f64)
                        .round()
                        .clamp(0.0, u64::MAX as f64) as u64
                })
                .collect::<Vec<_>>();
            let ordinal_seed = event_seed(&request, event.ordinal);
            let month = sample_weighted(&month_weights, ordinal_seed ^ STORM_MONTH_SEED_TAG)
                .expect("storm month");
            let trace =
                worldgen::trace_storm(&world.climate_field, &ocean_fraction, genesis, month as u32)
                    .unwrap();
            let mut expected = Vec::new();
            let mut maximum = 0.0f64;
            for step in &trace.steps {
                maximum = maximum.max(step.intensity);
                let cell = world.grid.cell_at(coarse.position(step.cell));
                if expected.last() != Some(&cell) {
                    expected.push(cell);
                }
            }
            assert_eq!(event.track_cells, expected);
            assert_eq!(event.hazard_ppm, fraction_ppm(maximum));
            assert_eq!(
                event.magnitude_milli,
                storm_magnitude_milli(event.hazard_ppm, ordinal_seed ^ 0x4553)
            );
            assert_eq!(
                event.volcanic_source_derivation_version,
                CLIMATE_DERIVATION_VERSION
            );
            // Dropping consecutive duplicates shifts indices, so each stored
            // cell is checked against its nearest traced coarse position. A
            // coarse id stored as a canonical cell lands far from every step.
            for track_cell in &event.track_cells {
                let distance = trace
                    .steps
                    .iter()
                    .map(|step| angular_distance(world, *track_cell, coarse.position(step.cell)))
                    .fold(f64::INFINITY, f64::min);
                assert!(
                    distance <= coarse_spacing,
                    "canonical cell {track_cell} is {distance} rad from every traced coarse cell"
                );
            }
        }
        let areas = (0..n_c)
            .map(|cell| coarse.area_m2(cell as u32) / 1.0e6)
            .collect::<Vec<_>>();
        let census = world
            .hazards
            .storms
            .genesis_km2
            .iter()
            .zip(areas)
            .map(|(genesis, area)| f64::from(*genesis) * area)
            .sum::<f64>();
        let expected_lambda = (census * STORM_NOTABLE_FRACTION * request.interval_length() as f64)
            .clamp(0.0, MAX_EXPECTED_EVENTS);
        assert!((sample.expected_lambda - expected_lambda).abs() < 1e-9);
        let one = sample_world_events(
            world,
            &self::request(NaturalEventKind::Storm, 0, 100_000, 42, 1),
        )
        .unwrap();
        assert_eq!(one.expected_lambda, sample.expected_lambda);
        assert!(one.events.len() <= 1);
    }

    #[test]
    fn floods_follow_fresh_spilling_lakes_in_minimum_cell_order() {
        let world = earthlike_64();
        let request = request(NaturalEventKind::Flood, -8_000, 8_000, 1, MAX_EVENTS);
        let sample = sample_world_events(world, &request).unwrap();
        let second = sample_world_events(world, &request).unwrap();
        assert_eq!(
            serde_json::to_string(&sample.events).unwrap(),
            serde_json::to_string(&second.events).unwrap()
        );
        let mut lakes = world
            .hydrology
            .lakes
            .iter()
            .filter(|lake| lake.kind == worldgen::LakeKind::Fresh && lake.outflow_m3 > 0.0)
            .collect::<Vec<_>>();
        lakes.sort_by_key(|lake| lake.minimum_cell);
        assert_eq!(sample.expected_lambda, lakes.len() as f64);
        assert_eq!(sample.events.len(), lakes.len().min(MAX_EVENTS as usize));
        for (event, lake) in sample.events.iter().zip(lakes) {
            assert_eq!(event.event_kind, NaturalEventKind::Flood);
            assert_eq!(event.year_offset, request.interval_midpoint_years());
            assert_eq!(event.cell, lake.spill_cell);
            assert_eq!(event.sampled_center_id, Some(u64::from(lake.minimum_cell)));
            assert_eq!(event.hazard_ppm, 1_000_000);
            assert_eq!(event.annual_rate_nano, 0);
            assert_eq!(event.rate_per_million_years_ppm, 0);
            assert_eq!(
                event.track_cells,
                if lake.spill_cell == lake.minimum_cell {
                    vec![lake.minimum_cell]
                } else {
                    vec![lake.minimum_cell, lake.spill_cell]
                }
            );
            assert_eq!(
                event.magnitude_milli,
                flood_magnitude_milli(lake.outflow_m3.round() as u64)
            );
            assert_eq!(
                event.hydrology_derivation_version,
                HYDROLOGY_DERIVATION_VERSION
            );
        }
    }

    #[test]
    fn zero_mass_worlds_return_no_events() {
        let mut world = earthlike_64().clone();
        world.hazards.quake_rate_km2.fill(0.0);
        let earthquakes = sample_world_events(
            &world,
            &request(NaturalEventKind::Earthquake, 0, 100_000, 3, MAX_EVENTS),
        )
        .unwrap();
        assert!(earthquakes.events.is_empty());
        assert_eq!(earthquakes.expected_lambda, 0.0);

        world.hazards.volcanic_sources.clear();
        let eruptions = sample_world_events(
            &world,
            &request(NaturalEventKind::Eruption, 0, 100_000, 3, MAX_EVENTS),
        )
        .unwrap();
        assert!(eruptions.events.is_empty());
        assert_eq!(eruptions.expected_lambda, 0.0);

        let mut dry = earthlike_64().clone();
        for lake in &mut dry.hydrology.lakes {
            lake.outflow_m3 = 0.0;
        }
        let floods = sample_world_events(
            &dry,
            &request(NaturalEventKind::Flood, 0, 1_000, 3, MAX_EVENTS),
        )
        .unwrap();
        assert!(floods.events.is_empty());
        assert_eq!(floods.expected_lambda, 0.0);
    }

    #[test]
    fn storm_genesis_with_all_zero_months_is_an_error() {
        let mut world = earthlike_64().clone();
        let n_c = world.climate_field.grid.cell_count();
        world.hazards.storms.genesis_monthly_km2.fill(0.0);
        world.hazards.storms.genesis_km2.fill(0.0);
        world.hazards.storms.genesis_km2[0] = 1.0e6;
        let error = sample_world_events(
            &world,
            &request(NaturalEventKind::Storm, 0, 100, 9, MAX_EVENTS),
        )
        .unwrap_err();
        assert!(matches!(error, PhysicalError::Validation(_)));
        assert_eq!(world.hazards.storms.genesis_monthly_km2.len(), 12 * n_c);
    }

    #[test]
    fn sampled_center_id_round_trips_beyond_two_to_the_53_and_rejects_numbers() {
        let id = (1u64 << 53) + 1;
        let event = MaterializedEvent {
            event_kind: NaturalEventKind::Eruption,
            ordinal: 0,
            year_offset: 0,
            cell: 1,
            longitude_microdegrees: 0,
            latitude_microdegrees: 0,
            magnitude_milli: 1_000,
            hazard_ppm: 0,
            annual_rate_nano: 1,
            rate_per_million_years_ppm: 1,
            sampled_center_id: Some(id),
            volcanic_source_derivation_version: VOLCANIC_SOURCE_DERIVATION_VERSION,
            hydrology_derivation_version: 0,
            track_cells: vec![1],
        };
        let json = serde_json::to_value(&event).unwrap();
        assert_eq!(json["sampledCenterId"], serde_json::json!(id.to_string()));
        let decoded: MaterializedEvent = serde_json::from_value(json.clone()).unwrap();
        assert_eq!(decoded.sampled_center_id, Some(id));

        let mut numeric = json;
        numeric["sampledCenterId"] = serde_json::json!(id);
        assert!(serde_json::from_value::<MaterializedEvent>(numeric).is_err());
    }

    #[test]
    fn present_derive_rejects_a_tampered_source_before_sampling() {
        let settings = crate::engine::Settings::from_parts(
            831_429,
            crate::engine::WorldPreset::Earthlike,
            32,
            crate::DEFAULT_RADIUS_METRES,
            290_000,
            crate::planetary::PlanetaryConfiguration::earth_like(),
        );
        let products = crate::engine::generate(settings, &mut NoopProgress).unwrap();
        let mut source = products.source().to_vec();
        let schedule_count = u32::from_le_bytes(source[64..68].try_into().unwrap()) as usize;
        let base = 68 + schedule_count * 16;
        // Climate hash. Decoding does not check it; a present derive does.
        source[base + 152] ^= 0x01;
        let decoded = crate::engine::decode_source(&source).unwrap();
        let epoch_zero = crate::history::EpochSolve {
            normalized_epoch: 0,
            temperature_offset_centi_c: 0,
            lagged_temperature_offset_centi_c: 0,
            land_ice_equilibrium_m3: 0,
            land_ice_m3: 0,
            thermal_expansion_m3: 0,
            target_ocean_water_m3: decoded.reference_water_inventory_m3(),
            sea_level_mm: 0,
        };
        let error = decoded
            .sample_events(
                Some(&epoch_zero),
                &request(NaturalEventKind::Storm, 0, 1_000, 5, 8),
                &mut NoopProgress,
            )
            .unwrap_err();
        assert_eq!(error.code(), crate::CODE_SOURCE_INVALID);
    }

    #[test]
    #[ignore = "wall-clock: release-only E6 event timings at n=512 and n=1024"]
    fn e6_release_event_timings() {
        for n in [512u32, 1024] {
            let settings = crate::engine::Settings::from_parts(
                831_429,
                crate::engine::WorldPreset::Earthlike,
                n,
                crate::DEFAULT_RADIUS_METRES,
                290_000,
                crate::planetary::PlanetaryConfiguration::earth_like(),
            );
            let source = crate::engine::generate(settings, &mut NoopProgress)
                .unwrap()
                .source()
                .to_vec();
            let decoded = crate::engine::decode_source(&source).unwrap();
            let forcing =
                crate::history::HistoricalForcingParameters::default_for_seed(decoded.seed());
            let solve =
                crate::history::solve_epoch(&decoded, forcing, -8_000, &NoopProgress).unwrap();
            for (kind, epoch) in [
                (NaturalEventKind::Earthquake, None),
                (NaturalEventKind::Storm, Some(&solve)),
                (NaturalEventKind::Flood, Some(&solve)),
            ] {
                let request = request(kind, -100_000, 100_000, 7, MAX_EVENTS);
                let started = std::time::Instant::now();
                let sample = decoded
                    .sample_events(epoch, &request, &mut NoopProgress)
                    .unwrap();
                println!(
                    "E6 n={n} {}: {:?}, {} events",
                    kind.label(),
                    started.elapsed(),
                    sample.events.len()
                );
            }
        }
    }

    #[test]
    fn changed_models_have_versioned_labels_and_materialization_versions() {
        assert_eq!(EVENT_MATERIALIZATION_VERSION, 3);
        assert_eq!(NaturalEventKind::Earthquake.materialization_version(), 2);
        assert_eq!(NaturalEventKind::Eruption.materialization_version(), 2);
        assert_eq!(NaturalEventKind::Storm.materialization_version(), 3);
        assert_eq!(NaturalEventKind::Flood.materialization_version(), 3);
        assert_eq!(
            NaturalEventKind::Earthquake.model_label(),
            "poisson-gutenberg-richter-v2"
        );
        assert_eq!(
            NaturalEventKind::Eruption.model_label(),
            "engine-volcanic-source-v3"
        );
        assert_eq!(
            NaturalEventKind::Storm.model_label(),
            "engine-storm-tracer-v3"
        );
        assert_eq!(
            NaturalEventKind::Flood.model_label(),
            "engine-lake-spill-v3"
        );
    }

    #[test]
    fn flood_materialization_key_is_scoped_to_map_world_version_cell_and_year() {
        let base = flood_materialization_key("map-a", "world-1", 3, 12, -8000);
        assert_eq!(
            base,
            flood_materialization_key("map-a", "world-1", 3, 12, -8000)
        );
        assert_ne!(
            base,
            flood_materialization_key("map-b", "world-1", 3, 12, -8000)
        );
        assert_ne!(
            base,
            flood_materialization_key("map-a", "world-2", 3, 12, -8000)
        );
        assert_ne!(
            base,
            flood_materialization_key("map-a", "world-1", 4, 12, -8000)
        );
        assert_ne!(
            base,
            flood_materialization_key("map-a", "world-1", 3, 13, -8000)
        );
        assert_ne!(
            base,
            flood_materialization_key("map-a", "world-1", 3, 12, 8000)
        );
    }
}
