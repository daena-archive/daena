//! Long-term hazard rate and potential fields from the finished world.
//!
//! Hazards read the tectonic, climate, water and biome products and never feed
//! back into them. Rates are events per year per km². Potentials are stored as
//! `u8`, `round(clamp(value, 0, 1) * 255)`.
//!
//! Earthquake density is a line-source convolution of the boundary records on
//! the tectonic grid, sampled bilinearly to the final grid, so the belt
//! integral is resolution-independent. Volcanic
//! sources use the same jittered lattices the tectonics already places. Storm
//! genesis and tracks live on the coarse climate grid, where the monthly wind
//! and sea-level temperature are defined.

use std::collections::VecDeque;

use rayon::prelude::*;

use crate::climate::{ClimateField, ClimateSummary};
use crate::grid::{cross, dot3, normalize, CubeSphere, NONE};
use crate::hydrology::Hydrology;
use crate::noise::{derive_seed, hash3, hash_u01, splitmix64, SeedDomain};
use crate::tectonics::{volcano_site, BoundaryKind, BoundaryRecord, Tectonics};
use crate::WorldError;

/// Jittered arc-volcano lattice spacing, metres.
pub const ARC_SPACING_M: f64 = 140_000.0;
/// Jittered rift-volcano lattice spacing, metres.
pub const RIFT_SPACING_M: f64 = 140_000.0;

/// Reference relative speed for boundary and arc strengths, m/yr.
const REF_SPEED_M_YR: f64 = 0.05;
/// M ≥ 5 earthquakes per year per km of convergent boundary at 50 mm/yr.
/// Calibrated on Earthlike seed 7 so the planet total is about 1 500.
const QUAKE_CONVERGENT_RATE_KM: f64 = 0.013_880_390;
const QUAKE_TRANSFORM_SHARE: f64 = 0.45;
const QUAKE_DIVERGENT_SHARE: f64 = 0.12;
const QUAKE_WIDTH_CONVERGENT_KM: f64 = 150.0;
const QUAKE_WIDTH_COLLISION_KM: f64 = 400.0;
const QUAKE_WIDTH_TRANSFORM_KM: f64 = 60.0;
const QUAKE_WIDTH_DIVERGENT_KM: f64 = 40.0;
/// Intraplate floor as a share of the boundary rate mass on every world.
const QUAKE_FLOOR_SHARE: f64 = 0.02;
/// Hazard at the centre of a 50 mm/yr convergent boundary.
const QUAKE_CENTRE_HAZARD: f64 = 0.9;
/// Cells per fixed reduction chunk. Chunks combine in order, so sums do not
/// depend on the thread count.
const HAZARD_CHUNK: usize = 4096;

/// Distance from the arc platform centre to the volcanic front, metres.
const ARC_BAND_CENTRE_M: f64 = 150_000.0;
const ARC_BAND_WIDTH_M: f64 = 60_000.0;
/// Minimum arc-platform bump for a lattice site to be a source.
const ARC_SOURCE_BUMP: f64 = 0.3;
/// A rift site must sit this close to a divergent boundary, metres.
const RIFT_MAX_DIST_M: f64 = 50_000.0;
/// Continental rift sites become moderate with this probability.
const RIFT_MODERATE_PROBABILITY: f64 = 0.3;
/// Activity thresholds on arcs: `u < 0.06 s` vigorous, `u < 0.36 s` moderate.
const ARC_VIGOROUS_U: f64 = 0.06;
const ARC_MODERATE_U: f64 = 0.36;
/// Eruptions per year by activity class.
const ERUPTIONS_VIGOROUS: f64 = 1.5;
const ERUPTIONS_MODERATE: f64 = 0.15;
const ERUPTIONS_DORMANT: f64 = 0.009;
/// Volcanic hazard weights by activity class.
const VOLCANO_WEIGHT_VIGOROUS: f64 = 1.0;
const VOLCANO_WEIGHT_MODERATE: f64 = 0.6;
const VOLCANO_WEIGHT_DORMANT: f64 = 0.25;
const VOLCANO_DECAY_M: f64 = 30_000.0;
const VOLCANO_CUTOFF_M: f64 = 90_000.0;

/// Genesis rate, storms per year per km² per unit of summed potential.
/// Calibrated once on Earthlike seed 7 so the planet total is about 85.
const STORM_GENESIS_C: f64 = 1.111_456_5e-7;
const STORM_GENESIS_SST_START_C: f64 = 26.0;
const STORM_GENESIS_SST_FULL_C: f64 = 28.5;
const STORM_PI_SST_START_C: f64 = 26.0;
const STORM_PI_SST_FULL_C: f64 = 32.0;
/// Poleward and westward beta drift, m/s.
const STORM_BETA_POLEWARD: f64 = 2.5;
const STORM_BETA_WESTWARD: f64 = 1.0;
const STORM_STEP_M: f64 = 250_000.0;
const STORM_STEPS: usize = 20;
/// Fraction of the gap to local potential intensity closed each ocean step.
const STORM_OCEAN_RELAX: f64 = 0.3;
/// Intensity multiplier per land step.
const STORM_LAND_DECAY: f64 = 0.6;
/// A track stops below this carried intensity.
const STORM_STOP_INTENSITY: f64 = 0.1;
/// Below this coarse ocean fraction a storm step is over land.
const STORM_OCEAN_FRACTION: f64 = 0.5;

/// Where a volcanic source comes from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VolcanicOrigin {
    /// Arc volcano on the overriding plate.
    SubductionArc,
    /// Active hotspot head or the first two trail points.
    Hotspot,
    /// Continental rift or oceanic spreading ridge.
    Rift,
}

impl VolcanicOrigin {
    /// Stable name used by the CLI and tests.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::SubductionArc => "subduction-arc",
            Self::Hotspot => "hotspot",
            Self::Rift => "rift",
        }
    }
}

/// Long-term activity class of a volcanic source.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VolcanicActivity {
    /// Rare eruptions.
    Dormant,
    /// Occasional eruptions.
    Moderate,
    /// Frequent eruptions.
    Vigorous,
}

impl VolcanicActivity {
    /// Stable name used by the CLI and tests.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Dormant => "dormant",
            Self::Moderate => "moderate",
            Self::Vigorous => "vigorous",
        }
    }

    fn weight(self) -> f64 {
        match self {
            Self::Dormant => VOLCANO_WEIGHT_DORMANT,
            Self::Moderate => VOLCANO_WEIGHT_MODERATE,
            Self::Vigorous => VOLCANO_WEIGHT_VIGOROUS,
        }
    }

    fn eruptions_per_yr(self) -> f64 {
        match self {
            Self::Dormant => ERUPTIONS_DORMANT,
            Self::Moderate => ERUPTIONS_MODERATE,
            Self::Vigorous => ERUPTIONS_VIGOROUS,
        }
    }
}

/// One discrete volcanic source with an id that does not depend on `n`.
#[derive(Clone, Debug, PartialEq)]
pub struct VolcanicSource {
    /// Hashed from origin and lattice coordinates or hotspot indices.
    pub id: u64,
    /// Unit direction of the source.
    pub pos: [f64; 3],
    /// Final-grid cell holding the source.
    pub cell: u32,
    /// Cause class.
    pub origin: VolcanicOrigin,
    /// Long-term activity class.
    pub activity: VolcanicActivity,
    /// Expected eruptions per year.
    pub eruptions_per_yr: f32,
}

/// Tropical-cyclone climatology on the coarse climate grid.
#[derive(Clone, Debug, PartialEq)]
pub struct StormClimatology {
    /// Tropical-cyclone formations per year per km².
    pub genesis_km2: Vec<f32>,
    /// Storm passages per year per km².
    pub track_km2: Vec<f32>,
    /// Potential intensity reached, `[0, 1]`.
    pub intensity: Vec<f32>,
}

/// Summary numbers for the hazard products.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HazardStats {
    /// M ≥ 5 earthquakes per year over the planet.
    pub quakes_per_yr: f64,
    /// Share of the quake rate within 300 km of a convergent boundary.
    pub quake_near_convergent_share: f64,
    /// Eruptions per year over the planet.
    pub eruptions_per_yr: f64,
    /// Subduction-arc share of the eruption rate.
    pub arc_eruption_share: f64,
    /// Tropical storms per year over the planet.
    pub storms_per_yr: f64,
    /// Source counts by origin and activity.
    pub subduction_arc_dormant: u32,
    pub subduction_arc_moderate: u32,
    pub subduction_arc_vigorous: u32,
    pub hotspot_dormant: u32,
    pub hotspot_moderate: u32,
    pub hotspot_vigorous: u32,
    pub rift_dormant: u32,
    pub rift_moderate: u32,
    pub rift_vigorous: u32,
    /// Sources that are not dormant.
    pub non_dormant_sources: u32,
    /// Area mean drought potential on land, `[0, 1]`.
    pub mean_land_drought: f64,
    /// Area mean flood potential on land, `[0, 1]`.
    pub mean_land_flood: f64,
    /// Area mean heat-wave potential on land, `[0, 1]`.
    pub mean_land_heat: f64,
    /// Area mean heat-wave potential over all cells, `[0, 1]`.
    pub mean_heat: f64,
    /// Area mean extreme-rain potential over all cells, `[0, 1]`.
    pub mean_extreme_rain: f64,
    /// FNV-1a over every hazard field, in declaration order.
    pub hazard_hash: u64,
}

/// Long-term hazard rate and potential fields.
#[derive(Clone, Debug, PartialEq)]
pub struct Hazards {
    /// Final grid: M ≥ 5 per year per km².
    pub quake_rate_km2: Vec<f32>,
    /// Final grid: seismic hazard, `u8`.
    pub quake_hazard: Vec<u8>,
    /// Final grid: volcanic hazard, `u8`.
    pub volcanic_hazard: Vec<u8>,
    /// Discrete sources, sorted by id.
    pub volcanic_sources: Vec<VolcanicSource>,
    /// Coarse climate grid: storm climatology.
    pub storms: StormClimatology,
    /// Final grid: drought potential, `0` on ocean.
    pub drought: Vec<u8>,
    /// Final grid: heat-wave potential.
    pub heat_wave: Vec<u8>,
    /// Final grid: extreme-rain potential.
    pub extreme_rain: Vec<u8>,
    /// Final grid: flood potential, `0` on ocean and under lakes.
    pub flood: Vec<u8>,
    /// Summary numbers and the hazard hash.
    pub stats: HazardStats,
}

/// Finished-world inputs of the hazard pass.
pub(crate) struct HazardIn<'a> {
    pub grid: &'a CubeSphere,
    pub ocean: &'a [u8],
    pub elevation: &'a [f32],
    pub climate: &'a ClimateSummary,
    pub climate_field: &'a ClimateField,
    pub hydrology: &'a Hydrology,
    pub tectonics: &'a Tectonics,
    pub river_min_m3s: f64,
    pub seed: u64,
}

/// Build every hazard field. Existing world products are read only.
pub(crate) fn build(input: &HazardIn<'_>) -> Result<Hazards, WorldError> {
    let n = input.grid.cell_count();
    if input.ocean.len() != n
        || input.elevation.len() != n
        || input.climate.t_warm_c.len() != n
        || input.climate.t_cold_c.len() != n
        || input.climate.p_annual_mm.len() != n
        || input.climate.p_dry_month_mm.len() != n
        || input.climate.pet_annual_mm.len() != n
        || input.hydrology.filled_m.len() != n
        || input.hydrology.receiver.len() != n
        || input.hydrology.discharge_m3s.len() != n
        || input.hydrology.lake_id.len() != n
    {
        return Err(WorldError::Internal(
            "hazard inputs do not match the terrain grid".to_string(),
        ));
    }
    let (quake_rate_km2, quake_hazard, quake) = earthquakes(input)?;
    let volcanoes = volcanoes(input)?;
    let storms = storms(input)?;
    let (prefix, extra) = rayon::join(
        || {
            hash_quake_fields(
                0xcbf2_9ce4_8422_2325u64,
                &quake_rate_km2,
                &quake_hazard,
                &volcanoes.hazard,
                &volcanoes.sources,
                &storms,
            )
        },
        || -> Result<ExtremeAndFlood, WorldError> {
            let (drought, heat_wave, extreme_rain) = extremes(input)?;
            let flood = floods(input, &extreme_rain);
            Ok((drought, heat_wave, extreme_rain, flood))
        },
    );
    let (drought, heat_wave, extreme_rain, flood) = extra?;
    let mut stats = hazard_stats(
        input,
        &quake_rate_km2,
        quake.near_convergent_share,
        &volcanoes.sources,
        &storms,
        &drought,
        &heat_wave,
        &extreme_rain,
        &flood,
    );
    stats.hazard_hash = hash_extreme_fields(prefix, &drought, &heat_wave, &extreme_rain, &flood);
    Ok(Hazards {
        quake_rate_km2,
        quake_hazard,
        volcanic_hazard: volcanoes.hazard,
        volcanic_sources: volcanoes.sources,
        storms,
        drought,
        heat_wave,
        extreme_rain,
        flood,
        stats,
    })
}

impl Hazards {
    /// FNV-1a over the hazard fields in declaration order, sources by id.
    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) fn hash_bits(&self) -> u64 {
        let hash = hash_quake_fields(
            0xcbf2_9ce4_8422_2325u64,
            &self.quake_rate_km2,
            &self.quake_hazard,
            &self.volcanic_hazard,
            &self.volcanic_sources,
            &self.storms,
        );
        hash_extreme_fields(
            hash,
            &self.drought,
            &self.heat_wave,
            &self.extreme_rain,
            &self.flood,
        )
    }
}

fn hash_quake_fields(
    mut hash: u64,
    quake_rate_km2: &[f32],
    quake_hazard: &[u8],
    volcanic_hazard: &[u8],
    sources: &[VolcanicSource],
    storms: &StormClimatology,
) -> u64 {
    for value in quake_rate_km2 {
        hash = fnv_f32(hash, *value);
    }
    for value in quake_hazard {
        hash = fnv_byte(hash, *value);
    }
    for value in volcanic_hazard {
        hash = fnv_byte(hash, *value);
    }
    for source in sources {
        hash = fnv_u64(hash, source.id);
        for component in source.pos {
            hash = fnv_f64(hash, component);
        }
        hash = fnv_u32(hash, source.cell);
        hash = fnv_byte(hash, origin_code(source.origin));
        hash = fnv_byte(hash, activity_code(source.activity));
        hash = fnv_f32(hash, source.eruptions_per_yr);
    }
    for value in &storms.genesis_km2 {
        hash = fnv_f32(hash, *value);
    }
    for value in &storms.track_km2 {
        hash = fnv_f32(hash, *value);
    }
    for value in &storms.intensity {
        hash = fnv_f32(hash, *value);
    }
    hash
}

fn hash_extreme_fields(
    mut hash: u64,
    drought: &[u8],
    heat_wave: &[u8],
    extreme_rain: &[u8],
    flood: &[u8],
) -> u64 {
    for value in drought {
        hash = fnv_byte(hash, *value);
    }
    for value in heat_wave {
        hash = fnv_byte(hash, *value);
    }
    for value in extreme_rain {
        hash = fnv_byte(hash, *value);
    }
    for value in flood {
        hash = fnv_byte(hash, *value);
    }
    hash
}

struct QuakeOut {
    near_convergent_share: f64,
}

fn earthquakes(input: &HazardIn<'_>) -> Result<(Vec<f32>, Vec<u8>, QuakeOut), WorldError> {
    let grid = input.grid;
    let tectonics = input.tectonics;
    let n = grid.cell_count();
    let tgrid = CubeSphere::new(tectonics.n, grid.radius_m).map_err(WorldError::Internal)?;
    let n_t = tgrid.cell_count();
    let positions: Vec<[f64; 3]> = (0..n_t)
        .into_par_iter()
        .map(|cell| tgrid.position(cell as u32))
        .collect();
    let edge_km = tgrid.cell_size_m() / 1000.0;
    let kernels = [
        QuakeKernel::new(QUAKE_WIDTH_DIVERGENT_KM, grid.radius_m),
        QuakeKernel::new(QUAKE_WIDTH_TRANSFORM_KM, grid.radius_m),
        QuakeKernel::new(QUAKE_WIDTH_CONVERGENT_KM, grid.radius_m),
        QuakeKernel::new(QUAKE_WIDTH_COLLISION_KM, grid.radius_m),
    ];
    let chunk_count = 12.min(tectonics.boundaries.len().max(1));
    let chunk_size = tectonics.boundaries.len().div_ceil(chunk_count);
    let partials: Vec<Vec<f32>> = tectonics
        .boundaries
        .par_chunks(chunk_size)
        .map(|records| {
            let mut density = vec![0.0f32; n_t];
            let mut visited = vec![0u32; n_t];
            let mut queue = VecDeque::new();
            let mut generation = 0u32;
            for rec in records {
                generation = generation.wrapping_add(1);
                if generation == 0 {
                    visited.fill(0);
                    generation = 1;
                }
                let width_km = quake_width(rec);
                let kernel = &kernels[quake_kernel_index(width_km)];
                // Both sides of an interface carry records, so each side carries
                // half the per-km rate. The belt integral is then the record rate
                // itself.
                let strength = 0.5 * quake_unit_rate(rec) * edge_km;
                splat_quake(
                    &tgrid,
                    &positions,
                    rec.cell,
                    positions[rec.cell as usize],
                    3.0 * width_km * 1000.0,
                    width_km,
                    kernel,
                    strength,
                    &mut density,
                    &mut visited,
                    &mut queue,
                    generation,
                );
            }
            density
        })
        .collect();
    let mut combined = vec![0.0f64; n_t];
    for part in partials {
        for (total, value) in combined.iter_mut().zip(part) {
            *total += f64::from(value);
        }
    }
    let density_t: Vec<f32> = combined.into_iter().map(|value| value as f32).collect();
    let scale = grid.n / tgrid.n;
    let mut unit = vec![0.0f32; n];
    unit.par_iter_mut().enumerate().for_each(|(cell, slot)| {
        let (face, i, j) = grid.decode(cell as u32);
        let fx = (f64::from(i) + 0.5) / f64::from(scale);
        let fy = (f64::from(j) + 0.5) / f64::from(scale);
        *slot = tgrid.sample_at(&density_t, face, fx, fy);
    });
    let chunks = n.div_ceil(HAZARD_CHUNK);
    let sums: Vec<(f64, f64)> = (0..chunks)
        .into_par_iter()
        .map(|chunk| {
            let start = chunk * HAZARD_CHUNK;
            let end = (start + HAZARD_CHUNK).min(n);
            let mut boundary_mass = 0.0;
            let mut planet_area = 0.0;
            for (offset, value) in unit[start..end].iter().enumerate() {
                let area = grid.area_m2((start + offset) as u32) / 1.0e6;
                boundary_mass += f64::from(*value) * area;
                planet_area += area;
            }
            (boundary_mass, planet_area)
        })
        .collect();
    let boundary_mass: f64 = sums.iter().map(|(mass, _)| *mass).sum();
    let planet_area_km2: f64 = sums.iter().map(|(_, area)| *area).sum();
    let floor = if planet_area_km2 > 0.0 {
        QUAKE_FLOOR_SHARE * QUAKE_CONVERGENT_RATE_KM * boundary_mass / planet_area_km2
    } else {
        0.0
    };
    let convergent_distance = convergent_distance(&tgrid, tectonics);
    let mut rate = vec![0.0f32; n];
    let mut hazard = vec![0u8; n];
    let centre =
        QUAKE_CONVERGENT_RATE_KM / (QUAKE_WIDTH_CONVERGENT_KM * std::f64::consts::PI.sqrt());
    let saturation = (centre / (1.0 / (1.0 - QUAKE_CENTRE_HAZARD)).ln()).max(f64::MIN_POSITIVE);
    let partials: Vec<(f64, f64)> = rate
        .par_chunks_mut(HAZARD_CHUNK)
        .zip(hazard.par_chunks_mut(HAZARD_CHUNK))
        .enumerate()
        .map(|(chunk, (rates, hazards))| {
            let start = chunk * HAZARD_CHUNK;
            let mut total = 0.0;
            let mut near = 0.0;
            let values = &unit[start..start + rates.len()];
            for (offset, ((rate, hazard), value)) in
                rates.iter_mut().zip(hazards).zip(values).enumerate()
            {
                let cell = start + offset;
                let density = QUAKE_CONVERGENT_RATE_KM * f64::from(*value) + floor;
                *rate = density as f32;
                *hazard = quantize(1.0 - (-density / saturation).exp());
                let mass = density * grid.area_m2(cell as u32) / 1.0e6;
                total += mass;
                let tcell = tectonic_cell(grid, &tgrid, cell);
                if convergent_distance
                    .get(tcell)
                    .is_some_and(|distance| *distance <= 300)
                {
                    near += mass;
                }
            }
            (total, near)
        })
        .collect();
    let total: f64 = partials.iter().map(|(total, _)| *total).sum();
    let near_mass: f64 = partials.iter().map(|(_, near)| *near).sum();
    let near_convergent_share = if total > 0.0 { near_mass / total } else { 0.0 };
    Ok((
        rate,
        hazard,
        QuakeOut {
            near_convergent_share,
        },
    ))
}

/// Splat one record's 2-D Gaussian on the tectonic grid, in true distance.
#[allow(clippy::too_many_arguments)]
fn splat_quake(
    grid: &CubeSphere,
    positions: &[[f64; 3]],
    start: u32,
    centre: [f64; 3],
    cutoff: f64,
    width_km: f64,
    kernel: &QuakeKernel,
    strength: f64,
    density: &mut [f32],
    visited: &mut [u32],
    queue: &mut VecDeque<(u32, f64)>,
    generation: u32,
) {
    queue.clear();
    queue.push_back((start, 0.0f64));
    visited[start as usize] = generation;
    let scale = strength / (std::f64::consts::PI * width_km * width_km);
    while let Some((cell, path)) = queue.pop_front() {
        let pos = positions[cell as usize];
        let dot = (centre[0] * pos[0] + centre[1] * pos[1] + centre[2] * pos[2]).clamp(-1.0, 1.0);
        if dot.acos() * grid.radius_m <= cutoff {
            let u = (2.0 * (1.0 - dot)).max(0.0).sqrt();
            let index = ((u * kernel.inv_step) as usize).min(kernel.table.len() - 1);
            density[cell as usize] += (scale * kernel.table[index]) as f32;
        }
        if path >= cutoff {
            continue;
        }
        for (slot, &neighbor) in grid.neighbors(cell).iter().enumerate() {
            if neighbor == NONE || visited[neighbor as usize] == generation {
                continue;
            }
            let step = f64::from(grid.neighbor_distance(cell, slot));
            if path + step > cutoff {
                continue;
            }
            visited[neighbor as usize] = generation;
            queue.push_back((neighbor, path + step));
        }
    }
}

const QUAKE_KERNEL_SAMPLES: usize = 65_536;

struct QuakeKernel {
    table: Vec<f64>,
    inv_step: f64,
}

impl QuakeKernel {
    fn new(width_km: f64, radius_m: f64) -> Self {
        let mut table = Vec::with_capacity(QUAKE_KERNEL_SAMPLES);
        let step = 2.0 / QUAKE_KERNEL_SAMPLES as f64;
        for index in 0..QUAKE_KERNEL_SAMPLES {
            let u = index as f64 * step;
            let theta = 2.0 * (u * 0.5).asin();
            let distance_km = theta * radius_m / 1000.0;
            let value = if distance_km <= 3.0 * width_km {
                (-(distance_km / width_km).powi(2)).exp()
            } else {
                0.0
            };
            table.push(value);
        }
        Self {
            table,
            inv_step: 1.0 / step,
        }
    }
}

fn quake_kernel_index(width_km: f64) -> usize {
    if width_km == QUAKE_WIDTH_DIVERGENT_KM {
        0
    } else if width_km == QUAKE_WIDTH_TRANSFORM_KM {
        1
    } else if width_km == QUAKE_WIDTH_CONVERGENT_KM {
        2
    } else {
        3
    }
}

fn tectonic_cell(grid: &CubeSphere, tectonic: &CubeSphere, cell: usize) -> usize {
    if grid.n.is_multiple_of(tectonic.n) {
        let scale = grid.n / tectonic.n;
        let (face, i, j) = grid.decode(cell as u32);
        tectonic.id(face, i / scale, j / scale) as usize
    } else {
        tectonic.cell_at(grid.position(cell as u32)) as usize
    }
}

fn convergent_distance(grid: &CubeSphere, tectonics: &Tectonics) -> Vec<u32> {
    let n = grid.cell_count();
    let mut distance = vec![u32::MAX; n];
    let mut buckets: Vec<Vec<u32>> = vec![Vec::new(); 301];
    for rec in &tectonics.boundaries {
        if rec.kind == BoundaryKind::Convergent {
            distance[rec.cell as usize] = 0;
            buckets[0].push(rec.cell);
        }
    }
    for current in 0..=300u32 {
        while let Some(cell) = buckets[current as usize].pop() {
            if distance[cell as usize] != current {
                continue;
            }
            for (slot, &neighbor) in grid.neighbors(cell).iter().enumerate() {
                if neighbor == NONE {
                    continue;
                }
                let step = (grid.neighbor_distance(cell, slot) / 1000.0).round() as u32;
                let next = current + step.max(1);
                if next <= 300 && next < distance[neighbor as usize] {
                    distance[neighbor as usize] = next;
                    buckets[next as usize].push(neighbor);
                }
            }
        }
    }
    distance
}

pub(crate) fn quake_width(rec: &BoundaryRecord) -> f64 {
    match rec.kind {
        BoundaryKind::Convergent => {
            if rec.crust_a == 1 && rec.crust_b == 1 {
                QUAKE_WIDTH_COLLISION_KM
            } else {
                QUAKE_WIDTH_CONVERGENT_KM
            }
        }
        BoundaryKind::Transform => QUAKE_WIDTH_TRANSFORM_KM,
        BoundaryKind::Divergent => QUAKE_WIDTH_DIVERGENT_KM,
    }
}

/// Density of one boundary record at `d_km`, per km², before calibration.
#[cfg_attr(not(test), allow(dead_code))]
pub(crate) fn quake_belt(rec: &BoundaryRecord, d_km: f64) -> f64 {
    let width = quake_width(rec);
    quake_unit_rate(rec) * (-(d_km / width).powi(2)).exp() / (width * std::f64::consts::PI.sqrt())
}

pub(crate) fn quake_unit_rate(rec: &BoundaryRecord) -> f64 {
    let speed = (f64::from(rec.convergence_m_yr).powi(2) + f64::from(rec.shear_m_yr).powi(2))
        .sqrt()
        / REF_SPEED_M_YR;
    let share = match rec.kind {
        BoundaryKind::Convergent => 1.0,
        BoundaryKind::Transform => QUAKE_TRANSFORM_SHARE,
        BoundaryKind::Divergent => QUAKE_DIVERGENT_SHARE,
    };
    share * speed.clamp(0.2, 2.0)
}

struct VolcanoOut {
    sources: Vec<VolcanicSource>,
    hazard: Vec<u8>,
}

fn volcanoes(input: &HazardIn<'_>) -> Result<VolcanoOut, WorldError> {
    let grid = input.grid;
    let tectonics = input.tectonics;
    let tgrid = CubeSphere::new(tectonics.n, grid.radius_m).map_err(WorldError::Internal)?;
    let radius = grid.radius_m;
    let mut sources = Vec::new();

    let arc_seed = derive_seed(input.seed, SeedDomain::ArcVolcano);
    let arc_c = ARC_SPACING_M / radius;
    let mut arc_sites = Vec::new();
    for_each_site_lattice(arc_c, |lattice| {
        if let Some(pos) = volcano_site(arc_seed, lattice, arc_c) {
            arc_sites.push((lattice, pos));
        }
    });
    for (lattice, pos) in arc_sites {
        let tcell = tgrid.cell_at(pos) as usize;
        let record = tectonics.arc_record[tcell];
        if record == NONE {
            continue;
        }
        let distance = f64::from(tectonics.d_arc[tcell]);
        let bump = (-((distance - ARC_BAND_CENTRE_M) / ARC_BAND_WIDTH_M).powi(2)).exp();
        if bump < ARC_SOURCE_BUMP {
            continue;
        }
        let rec = &tectonics.boundaries[record as usize];
        let id = hash3(arc_seed ^ 0xA7C0_0001, lattice[0], lattice[1], lattice[2]);
        let u = hash_u01(id, 0, 0);
        let s = (f64::from(rec.convergence_m_yr) / REF_SPEED_M_YR).clamp(0.4, 1.5);
        let activity = if u < ARC_VIGOROUS_U * s {
            VolcanicActivity::Vigorous
        } else if u < ARC_MODERATE_U * s {
            VolcanicActivity::Moderate
        } else {
            VolcanicActivity::Dormant
        };
        sources.push(VolcanicSource {
            id,
            pos,
            cell: grid.cell_at(pos),
            origin: VolcanicOrigin::SubductionArc,
            activity,
            eruptions_per_yr: activity.eruptions_per_yr() as f32,
        });
    }

    let rift_seed = derive_seed(input.seed, SeedDomain::RiftVolcano);
    let rift_c = RIFT_SPACING_M / radius;
    let mut rift_sites = Vec::new();
    for_each_site_lattice(rift_c, |lattice| {
        if let Some(pos) = volcano_site(rift_seed, lattice, rift_c) {
            rift_sites.push((lattice, pos));
        }
    });
    for (lattice, pos) in rift_sites {
        let tcell = tgrid.cell_at(pos) as usize;
        let record = tectonics.boundary_record[tcell];
        if record == NONE {
            continue;
        }
        if tectonics.d_boundary[tcell] > RIFT_MAX_DIST_M as f32 {
            continue;
        }
        let rec = &tectonics.boundaries[record as usize];
        if rec.kind != BoundaryKind::Divergent {
            continue;
        }
        let id = hash3(rift_seed ^ 0xA7C0_0002, lattice[0], lattice[1], lattice[2]);
        let continental = rec.crust_a == 1 && rec.crust_b == 1;
        let activity = if continental && hash_u01(id, 0, 0) < RIFT_MODERATE_PROBABILITY {
            VolcanicActivity::Moderate
        } else {
            VolcanicActivity::Dormant
        };
        sources.push(VolcanicSource {
            id,
            pos,
            cell: grid.cell_at(pos),
            origin: VolcanicOrigin::Rift,
            activity,
            eruptions_per_yr: activity.eruptions_per_yr() as f32,
        });
    }

    let hotspot_seed = derive_seed(input.seed, SeedDomain::Hotspots);
    for (index, hotspot) in tectonics.hotspots.iter().enumerate() {
        for (trail, point) in hotspot.points.iter().copied().take(3).enumerate() {
            let activity = match trail {
                0 => VolcanicActivity::Vigorous,
                1 => VolcanicActivity::Moderate,
                _ => VolcanicActivity::Dormant,
            };
            let id = splitmix64(
                hotspot_seed
                    ^ (index as u64 + 1).wrapping_mul(0x9E37_79B9_7F4A_7C15)
                    ^ (trail as u64).wrapping_mul(0xBF58_476D_1CE4_E5B9),
            );
            sources.push(VolcanicSource {
                id,
                pos: point,
                cell: grid.cell_at(point),
                origin: VolcanicOrigin::Hotspot,
                activity,
                eruptions_per_yr: activity.eruptions_per_yr() as f32,
            });
        }
    }

    sources.sort_by_key(|source| source.id);
    let mut value = vec![0.0f32; grid.cell_count()];
    let mut visited = vec![0u32; grid.cell_count()];
    let mut generation = 0u32;
    for source in &sources {
        generation = generation.wrapping_add(1);
        if generation == 0 {
            visited.fill(0);
            generation = 1;
        }
        splat(
            grid,
            source.pos,
            source.cell,
            VOLCANO_CUTOFF_M,
            VOLCANO_DECAY_M,
            source.activity.weight(),
            &mut value,
            &mut visited,
            generation,
        );
    }
    let hazard = value
        .into_iter()
        .map(|value| quantize(f64::from(value)))
        .collect();
    Ok(VolcanoOut { sources, hazard })
}

/// Lattice cells whose jittered raw point can land on the unit-sphere shell.
///
/// The raw point is `(lattice + jitter) * c` with jitter in `[0.15, 0.85]`,
/// so a site exists only when the centre norm is within `0.5 + 0.35√3`
/// lattice units of `1/c`. Enumerating that shell instead of the full cube cuts
/// the scan about thirtyfold without changing which sites `volcano_site`
/// accepts.
fn for_each_site_lattice(c: f64, mut visit: impl FnMut([i64; 3])) {
    let radius = 1.0 / c;
    let slack = 0.5 + 0.35 * 3.0f64.sqrt();
    let outer = radius + slack;
    let inner = (radius - slack).max(0.0);
    let reach = outer.ceil() as i64;
    for x in -reach..=reach {
        let dx = x as f64 + 0.5;
        for y in -reach..=reach {
            let dy = y as f64 + 0.5;
            let d2 = dx * dx + dy * dy;
            if d2 > outer * outer {
                continue;
            }
            let z_hi = (outer * outer - d2).max(0.0).sqrt();
            let z_lo = (inner * inner - d2).max(0.0).sqrt();
            if z_lo == 0.0 {
                let low = (-z_hi - 0.5).ceil() as i64;
                let high = (z_hi - 0.5).floor() as i64;
                for z in low..=high {
                    visit([x, y, z]);
                }
            } else {
                let low = (z_lo - 0.5).ceil() as i64;
                let high = (z_hi - 0.5).floor() as i64;
                for z in low..=high {
                    visit([x, y, z]);
                }
                let low = (-z_hi - 0.5).ceil() as i64;
                let high = (-z_lo - 0.5).floor() as i64;
                for z in low..=high {
                    visit([x, y, z]);
                }
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn splat(
    grid: &CubeSphere,
    centre: [f64; 3],
    start: u32,
    cutoff: f64,
    decay: f64,
    weight: f64,
    value: &mut [f32],
    visited: &mut [u32],
    generation: u32,
) {
    let mut queue = VecDeque::new();
    queue.push_back((start, 0.0f64));
    visited[start as usize] = generation;
    while let Some((cell, path)) = queue.pop_front() {
        let pos = grid.position(cell);
        let distance = angle_between(centre, pos) * grid.radius_m;
        if distance <= cutoff {
            let contribution = (weight * (-(distance / decay).powi(2)).exp()) as f32;
            if contribution > value[cell as usize] {
                value[cell as usize] = contribution;
            }
        }
        if path >= cutoff {
            continue;
        }
        for (slot, &neighbor) in grid.neighbors(cell).iter().enumerate() {
            if neighbor == NONE || visited[neighbor as usize] == generation {
                continue;
            }
            let step = f64::from(grid.neighbor_distance(cell, slot));
            if path + step > cutoff {
                continue;
            }
            visited[neighbor as usize] = generation;
            queue.push_back((neighbor, path + step));
        }
    }
}

fn storms(input: &HazardIn<'_>) -> Result<StormClimatology, WorldError> {
    let fine = input.grid;
    let coarse = &input.climate_field.grid;
    if !fine.n.is_multiple_of(coarse.n) {
        return Err(WorldError::Internal(
            "climate grid does not nest in the hazard grid".to_string(),
        ));
    }
    let scale = fine.n / coarse.n;
    let n_c = coarse.cell_count();
    let ocean_frac = coarse_ocean_fraction(fine, input.ocean, coarse, scale);
    let inverse_area: Vec<f64> = (0..n_c)
        .map(|cell| {
            let area_km2 = coarse.area_m2(cell as u32) / 1.0e6;
            if area_km2 > 0.0 {
                1.0 / area_km2
            } else {
                0.0
            }
        })
        .collect();
    let mut ocean_columns = Vec::with_capacity(n_c);
    for cell in 0..n_c {
        let pos = coarse.position(cell as u32);
        let lat = pos[2].clamp(-1.0, 1.0).asin();
        ocean_columns.push((pos, lat));
    }
    let hadley = input.climate_field.phi_h_deg;
    let months: Vec<MonthStorm> = (0..12)
        .into_par_iter()
        .map(|month| {
            let base = month * n_c;
            let plane = month * 3 * n_c;
            let mut genesis = vec![0.0f64; n_c];
            let mut track = vec![0.0f64; n_c];
            let mut intensity = vec![0.0f64; n_c];
            for (cell, value) in genesis.iter_mut().enumerate() {
                *value = STORM_GENESIS_C
                    * genesis_potential(
                        ocean_frac[cell],
                        f64::from(input.climate_field.t_c[base + cell]),
                        ocean_columns[cell].1,
                        hadley,
                    );
            }
            for origin in 0..n_c {
                let weight = genesis[origin];
                if weight <= 0.0 {
                    continue;
                }
                let mut pos = ocean_columns[origin].0;
                let mut cell = origin;
                let mut strength = smoothstep(
                    STORM_PI_SST_START_C,
                    STORM_PI_SST_FULL_C,
                    f64::from(input.climate_field.t_c[base + origin]),
                );
                raise(&mut intensity, cell, strength);
                add_track(&mut track, cell, weight, &inverse_area);
                let mut seen = vec![cell];
                for _ in 0..STORM_STEPS {
                    let wind = [
                        f64::from(input.climate_field.wind[plane + cell]),
                        f64::from(input.climate_field.wind[plane + n_c + cell]),
                        f64::from(input.climate_field.wind[plane + 2 * n_c + cell]),
                    ];
                    let (east, north) = local_east_north(pos);
                    let poleward = if pos[2] >= 0.0 { 1.0 } else { -1.0 };
                    // The deep-layer flow is the zonal wind; the surface trades
                    // carry an equatorward boundary-layer component that would
                    // cancel the beta drift.
                    let zonal = wind[0] * east[0] + wind[1] * east[1] + wind[2] * east[2];
                    let direction = normalize([
                        (zonal - STORM_BETA_WESTWARD) * east[0]
                            + poleward * STORM_BETA_POLEWARD * north[0],
                        (zonal - STORM_BETA_WESTWARD) * east[1]
                            + poleward * STORM_BETA_POLEWARD * north[1],
                        (zonal - STORM_BETA_WESTWARD) * east[2]
                            + poleward * STORM_BETA_POLEWARD * north[2],
                    ]);
                    pos = step_gc(pos, direction, STORM_STEP_M / coarse.radius_m);
                    cell = coarse.cell_at(pos) as usize;
                    let local_pi = smoothstep(
                        STORM_PI_SST_START_C,
                        STORM_PI_SST_FULL_C,
                        f64::from(input.climate_field.t_c[base + cell]),
                    );
                    let ocean = ocean_frac[cell] >= STORM_OCEAN_FRACTION;
                    strength = if ocean {
                        strength + STORM_OCEAN_RELAX * (local_pi - strength)
                    } else {
                        strength * STORM_LAND_DECAY
                    };
                    if !seen.contains(&cell) {
                        seen.push(cell);
                        add_track(&mut track, cell, weight, &inverse_area);
                    }
                    raise(&mut intensity, cell, strength);
                    // A storm over cool water can fall below the potential
                    // intensity threshold and still keep a tracked circulation;
                    // the stop is the land-decay cutoff.
                    if !ocean && strength < STORM_STOP_INTENSITY {
                        break;
                    }
                }
            }
            MonthStorm {
                genesis,
                track,
                intensity,
            }
        })
        .collect();

    let mut genesis = vec![0.0f64; n_c];
    let mut track = vec![0.0f64; n_c];
    let mut intensity = vec![0.0f64; n_c];
    for month in months {
        for cell in 0..n_c {
            genesis[cell] += month.genesis[cell];
            track[cell] += month.track[cell];
            if month.intensity[cell] > intensity[cell] {
                intensity[cell] = month.intensity[cell];
            }
        }
    }

    let to_f32 =
        |values: Vec<f64>| -> Vec<f32> { values.into_iter().map(|value| value as f32).collect() };
    Ok(StormClimatology {
        genesis_km2: to_f32(genesis),
        track_km2: to_f32(track),
        intensity: to_f32(intensity),
    })
}

struct MonthStorm {
    genesis: Vec<f64>,
    track: Vec<f64>,
    intensity: Vec<f64>,
}

fn coarse_ocean_fraction(
    fine: &CubeSphere,
    fine_ocean: &[u8],
    coarse: &CubeSphere,
    scale: u32,
) -> Vec<f64> {
    let n_c = coarse.cell_count();
    (0..n_c)
        .into_par_iter()
        .map(|cell| {
            let (face, i, j) = coarse.decode(cell as u32);
            let mut ocean = 0u32;
            for dj in 0..scale {
                for di in 0..scale {
                    let id = fine.id(face, i * scale + di, j * scale + dj);
                    if id != NONE && fine_ocean[id as usize] == 1 {
                        ocean += 1;
                    }
                }
            }
            ocean as f64 / (scale * scale) as f64
        })
        .collect()
}

fn genesis_potential(ocean_frac: f64, sst_c: f64, lat: f64, hadley_deg: f64) -> f64 {
    let warm = smoothstep(STORM_GENESIS_SST_START_C, STORM_GENESIS_SST_FULL_C, sst_c);
    let abs_lat = lat.to_degrees().abs();
    let latitude = smoothstep(4.0, 8.0, abs_lat)
        * (1.0 - smoothstep(hadley_deg - 5.0, hadley_deg + 5.0, abs_lat));
    ocean_frac * warm * latitude
}

fn add_track(track: &mut [f64], cell: usize, weight: f64, inverse_area: &[f64]) {
    track[cell] += weight * inverse_area[cell];
}

fn raise(field: &mut [f64], cell: usize, value: f64) {
    if value > field[cell] {
        field[cell] = value;
    }
}

type ExtremeFields = (Vec<u8>, Vec<u8>, Vec<u8>);
type ExtremeAndFlood = (Vec<u8>, Vec<u8>, Vec<u8>, Vec<u8>);

fn extremes(input: &HazardIn<'_>) -> Result<ExtremeFields, WorldError> {
    let fine = input.grid;
    let coarse = &input.climate_field.grid;
    if !fine.n.is_multiple_of(coarse.n) {
        return Err(WorldError::Internal(
            "climate grid does not nest in the hazard grid".to_string(),
        ));
    }
    let n_c = coarse.cell_count();
    let scale = fine.n / coarse.n;
    let mut annual = vec![0.0f32; n_c];
    let mut wet_ratio = vec![0.0f32; n_c];
    for month in 0..12 {
        for (cell, value) in annual.iter_mut().enumerate() {
            *value += input.climate_field.p_mm[month * n_c + cell];
        }
    }
    for (cell, value) in wet_ratio.iter_mut().enumerate() {
        let total = annual[cell];
        if total > 0.0 {
            let mut peak = 0.0f32;
            for month in 0..12 {
                peak = peak.max(input.climate_field.p_mm[month * n_c + cell]);
            }
            *value = peak / total;
        }
    }
    let n = fine.cell_count();
    let rows: Vec<_> = (0..n)
        .into_par_iter()
        .map(|cell| {
            let (face, i, j) = fine.decode(cell as u32);
            let fx = (f64::from(i) + 0.5) / f64::from(scale);
            let fy = (f64::from(j) + 0.5) / f64::from(scale);
            let p_annual = f64::from(input.climate.p_annual_mm[cell]);
            let peak_estimate = p_annual * f64::from(coarse.sample_at(&wet_ratio, face, fx, fy));
            let t_warm = f64::from(input.climate.t_warm_c[cell]);
            let t_cold = f64::from(input.climate.t_cold_c[cell]);
            let p_dry = f64::from(input.climate.p_dry_month_mm[cell]);
            let pet = f64::from(input.climate.pet_annual_mm[cell]);
            let ocean = input.ocean[cell] == 1;
            let drought = if ocean || t_warm < 0.0 {
                0.0
            } else {
                let aridity = if pet > 0.0 { p_annual / pet } else { 2.0 };
                let dry_months = 1.0 - smoothstep(10.0, 60.0, p_dry);
                let wet_fade = 1.0 - smoothstep(0.65, 1.5, aridity);
                // Weights sum to 1 so hyper-arid land with a dry season reads 1.
                0.75 * (1.0 - smoothstep(0.1, 0.65, aridity)) + 0.25 * dry_months * wet_fade
            };
            let mut heat =
                0.7 * smoothstep(22.0, 38.0, t_warm) + 0.3 * smoothstep(4.0, 25.0, t_warm - t_cold);
            if ocean {
                heat *= 0.25;
            }
            let rain = if t_warm < 0.0 && peak_estimate < 30.0 {
                0.0
            } else {
                let monsoon = if p_annual > 0.0 {
                    peak_estimate / p_annual
                } else {
                    0.0
                };
                smoothstep(80.0, 450.0, peak_estimate)
                    * (0.7 + 0.3 * smoothstep(0.15, 0.35, monsoon))
            };
            (quantize(drought), quantize(heat), quantize(rain))
        })
        .collect();
    let mut drought = Vec::with_capacity(n);
    let mut heat_wave = Vec::with_capacity(n);
    let mut extreme_rain = Vec::with_capacity(n);
    for (dry, heat, rain) in rows {
        drought.push(dry);
        heat_wave.push(heat);
        extreme_rain.push(rain);
    }
    Ok((drought, heat_wave, extreme_rain))
}

fn floods(input: &HazardIn<'_>, extreme_rain: &[u8]) -> Vec<u8> {
    let grid = input.grid;
    let hydro = input.hydrology;
    let n = grid.cell_count();
    let mut river = vec![0u32; n];
    for &cell in &hydro.pop_order {
        let c = cell as usize;
        if c >= n {
            continue;
        }
        if input.ocean[c] == 1 {
            river[c] = cell;
            continue;
        }
        let discharge = hydro.discharge_m3s[c];
        let is_river =
            hydro.lake_id[c] == NONE && discharge.is_finite() && discharge >= input.river_min_m3s;
        river[c] = if is_river {
            cell
        } else {
            let receiver = hydro.receiver[c] as usize;
            if receiver < n && receiver != c {
                river[receiver]
            } else {
                cell
            }
        };
    }
    let mut flood = vec![0u8; n];
    flood.par_iter_mut().enumerate().for_each(|(cell, slot)| {
        if input.ocean[cell] == 1 || hydro.lake_id[cell] != NONE {
            return;
        }
        let source = river[cell] as usize;
        let delta = f64::from(input.elevation[cell]) - f64::from(input.elevation[source]);
        if delta < 0.0 {
            // Below the inherited drainage is a depression floor, not a river
            // floodplain. Clamping to zero would flood the whole basin.
            return;
        }
        let rain = f64::from(extreme_rain[cell]) / 255.0;
        *slot = quantize(flood_potential(delta, hydro.discharge_m3s[source], rain));
    });
    flood
}

#[allow(clippy::too_many_arguments)]
fn hazard_stats(
    input: &HazardIn<'_>,
    quake_rate_km2: &[f32],
    quake_near_convergent_share: f64,
    sources: &[VolcanicSource],
    storms: &StormClimatology,
    drought: &[u8],
    heat_wave: &[u8],
    extreme_rain: &[u8],
    flood: &[u8],
) -> HazardStats {
    let n = input.grid.cell_count();
    let chunks = n.div_ceil(HAZARD_CHUNK);
    let partials: Vec<[f64; 8]> = (0..chunks)
        .into_par_iter()
        .map(|chunk| {
            let start = chunk * HAZARD_CHUNK;
            let end = (start + HAZARD_CHUNK).min(n);
            let mut sum = [0.0f64; 8];
            for cell in start..end {
                let area = input.grid.area_m2(cell as u32);
                sum[0] += f64::from(quake_rate_km2[cell]) * area / 1.0e6;
                sum[1] += area;
                sum[5] += f64::from(heat_wave[cell]) / 255.0 * area;
                sum[6] += f64::from(extreme_rain[cell]) / 255.0 * area;
                if input.ocean[cell] == 0 {
                    sum[2] += area;
                    sum[3] += f64::from(drought[cell]) / 255.0 * area;
                    sum[4] += f64::from(flood[cell]) / 255.0 * area;
                    sum[7] += f64::from(heat_wave[cell]) / 255.0 * area;
                }
            }
            sum
        })
        .collect();
    let mut totals = [0.0f64; 8];
    for part in partials {
        for (total, value) in totals.iter_mut().zip(part) {
            *total += value;
        }
    }
    let quakes = totals[0];
    let all_area = totals[1];
    let land_area = totals[2];
    let land_drought = totals[3];
    let land_flood = totals[4];
    let all_heat = totals[5];
    let all_rain = totals[6];
    let land_heat = totals[7];
    let storms_per_yr = storms
        .genesis_km2
        .iter()
        .enumerate()
        .map(|(cell, value)| {
            f64::from(*value) * input.climate_field.grid.area_m2(cell as u32) / 1.0e6
        })
        .sum();
    let mut stats = HazardStats {
        quakes_per_yr: quakes,
        quake_near_convergent_share,
        eruptions_per_yr: 0.0,
        arc_eruption_share: 0.0,
        storms_per_yr,
        subduction_arc_dormant: 0,
        subduction_arc_moderate: 0,
        subduction_arc_vigorous: 0,
        hotspot_dormant: 0,
        hotspot_moderate: 0,
        hotspot_vigorous: 0,
        rift_dormant: 0,
        rift_moderate: 0,
        rift_vigorous: 0,
        non_dormant_sources: 0,
        mean_land_drought: mean(land_drought, land_area),
        mean_land_flood: mean(land_flood, land_area),
        mean_land_heat: mean(land_heat, land_area),
        mean_heat: mean(all_heat, all_area),
        mean_extreme_rain: mean(all_rain, all_area),
        hazard_hash: 0,
    };
    let mut arc_rate = 0.0;
    for source in sources {
        stats.eruptions_per_yr += f64::from(source.eruptions_per_yr);
        if source.origin == VolcanicOrigin::SubductionArc {
            arc_rate += f64::from(source.eruptions_per_yr);
        }
        if source.activity != VolcanicActivity::Dormant {
            stats.non_dormant_sources += 1;
        }
        let counter = match (source.origin, source.activity) {
            (VolcanicOrigin::SubductionArc, VolcanicActivity::Dormant) => {
                &mut stats.subduction_arc_dormant
            }
            (VolcanicOrigin::SubductionArc, VolcanicActivity::Moderate) => {
                &mut stats.subduction_arc_moderate
            }
            (VolcanicOrigin::SubductionArc, VolcanicActivity::Vigorous) => {
                &mut stats.subduction_arc_vigorous
            }
            (VolcanicOrigin::Hotspot, VolcanicActivity::Dormant) => &mut stats.hotspot_dormant,
            (VolcanicOrigin::Hotspot, VolcanicActivity::Moderate) => &mut stats.hotspot_moderate,
            (VolcanicOrigin::Hotspot, VolcanicActivity::Vigorous) => &mut stats.hotspot_vigorous,
            (VolcanicOrigin::Rift, VolcanicActivity::Dormant) => &mut stats.rift_dormant,
            (VolcanicOrigin::Rift, VolcanicActivity::Moderate) => &mut stats.rift_moderate,
            (VolcanicOrigin::Rift, VolcanicActivity::Vigorous) => &mut stats.rift_vigorous,
        };
        *counter += 1;
    }
    stats.arc_eruption_share = if stats.eruptions_per_yr > 0.0 {
        arc_rate / stats.eruptions_per_yr
    } else {
        0.0
    };
    stats
}

pub(crate) fn flood_potential(hand_m: f64, discharge_m3s: f64, extreme_rain: f64) -> f64 {
    let discharge = discharge_m3s.max(1.0).log10();
    (1.0 - smoothstep(0.0, 12.0, hand_m))
        * smoothstep(1.0, 4.0, discharge)
        * (0.6 + 0.4 * extreme_rain.clamp(0.0, 1.0))
}

fn mean(sum: f64, area: f64) -> f64 {
    if area > 0.0 {
        sum / area
    } else {
        0.0
    }
}

fn quantize(value: f64) -> u8 {
    (value.clamp(0.0, 1.0) * 255.0).round() as u8
}

fn smoothstep(edge0: f64, edge1: f64, value: f64) -> f64 {
    if edge0 == edge1 {
        return if value < edge0 { 0.0 } else { 1.0 };
    }
    let t = ((value - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

fn angle_between(a: [f64; 3], b: [f64; 3]) -> f64 {
    let c = cross(a, b);
    let s = (c[0] * c[0] + c[1] * c[1] + c[2] * c[2]).sqrt();
    s.atan2(dot3(a, b).clamp(-1.0, 1.0))
}

fn step_gc(p: [f64; 3], tangent: [f64; 3], angle: f64) -> [f64; 3] {
    let (s, c) = angle.sin_cos();
    normalize([
        p[0] * c + tangent[0] * s,
        p[1] * c + tangent[1] * s,
        p[2] * c + tangent[2] * s,
    ])
}

fn local_east_north(up: [f64; 3]) -> ([f64; 3], [f64; 3]) {
    let mut east = cross([0.0, 0.0, 1.0], up);
    let mut len = norm(east);
    if len < 1e-8 {
        east = cross([0.0, 1.0, 0.0], up);
        len = norm(east);
    }
    if len < 1e-12 {
        return ([1.0, 0.0, 0.0], [0.0, 1.0, 0.0]);
    }
    east = [east[0] / len, east[1] / len, east[2] / len];
    (east, cross(up, east))
}

fn norm(v: [f64; 3]) -> f64 {
    (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt()
}

fn origin_code(origin: VolcanicOrigin) -> u8 {
    match origin {
        VolcanicOrigin::SubductionArc => 0,
        VolcanicOrigin::Hotspot => 1,
        VolcanicOrigin::Rift => 2,
    }
}

fn activity_code(activity: VolcanicActivity) -> u8 {
    match activity {
        VolcanicActivity::Dormant => 0,
        VolcanicActivity::Moderate => 1,
        VolcanicActivity::Vigorous => 2,
    }
}

fn fnv_byte(mut hash: u64, byte: u8) -> u64 {
    hash ^= u64::from(byte);
    hash.wrapping_mul(0x0000_0100_0000_01b3)
}

fn fnv_u32(mut hash: u64, value: u32) -> u64 {
    for byte in value.to_le_bytes() {
        hash = fnv_byte(hash, byte);
    }
    hash
}

fn fnv_u64(mut hash: u64, value: u64) -> u64 {
    for byte in value.to_le_bytes() {
        hash = fnv_byte(hash, byte);
    }
    hash
}

fn fnv_f32(hash: u64, value: f32) -> u64 {
    fnv_u32(hash, value.to_bits())
}

fn fnv_f64(hash: u64, value: f64) -> u64 {
    fnv_u64(hash, value.to_bits())
}
