//! Deterministic terrain evolution and depression-resolving drainage.
//!
//! The routing surface in this module is a temporary Priority-Flood surface;
//! fill depth and flood order are recorded, and the real signed elevation
//! field is never modified while routing. Flow uses D-infinity facets on the
//! local tangent plane with a polar stencil that never uses a zero-width
//! longitude step. Stream-power incision evaluates `K * Q^m * S^n` from a
//! versioned evolution preset.

use std::cmp::Reverse;
use std::collections::BinaryHeap;

use super::{
    climate::{derive_current_climate, ClimateField, ClimateSettings},
    Grid, PhysicalError, PhysicalErrorCode, PhysicalField, ProgressPhase, ProgressSink,
};
use crate::tectonics::{geodesic_nearest, BoundaryKind, TectonicWorld};

pub const EVOLUTION_DERIVATION_VERSION: u16 = 4;
pub const MAX_EVOLUTION_ELEVATION_MM: i32 = 9_000_000;
pub const MAX_RELIEF_LOSS_PER_STEP_MM: i32 = 25_000;
pub const ROUTING_ANISOTROPY_LIMIT_PPM: u32 = 950_000;
pub const SECONDS_PER_YEAR: u64 = 31_557_600;
pub const STREAM_POWER_M_MILLI: u16 = 500;
pub const STREAM_POWER_N_MILLI: u16 = 1_000;
const MILLION: u128 = 1_000_000;
const MIN_TANGENT_METRES: f64 = 1.0;
const COASTAL_EROSION_RADIUS_MM: i64 = 350_000_000;
const MAX_DEPOSITION_PER_STEP_MM: i32 = 8_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EvolutionPreset {
    Young,
    Mature,
    Old,
}

impl EvolutionPreset {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Young => "young",
            Self::Mature => "mature",
            Self::Old => "old",
        }
    }

    pub fn parse(value: &str) -> Result<Self, PhysicalError> {
        match value {
            "young" => Ok(Self::Young),
            "mature" => Ok(Self::Mature),
            "old" => Ok(Self::Old),
            _ => Err(PhysicalError::InvalidSettings(
                "evolution preset must be young, mature, or old".into(),
            )),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EvolutionSettings {
    pub preset: EvolutionPreset,
}

impl EvolutionSettings {
    pub const fn default() -> Self {
        Self {
            preset: EvolutionPreset::Mature,
        }
    }

    pub const fn budget(self) -> EvolutionBudget {
        match self.preset {
            EvolutionPreset::Young => EvolutionBudget {
                steps: 2,
                timestep_years: 8_000,
                climate_cadence_steps: 1,
                erodibility_e12: 20_000_000_000,
                discharge_exponent_milli: STREAM_POWER_M_MILLI,
                slope_exponent_milli: STREAM_POWER_N_MILLI,
                uplift_mm_per_year: 10,
                diffusivity_m2_per_year: 1_200_000,
                max_incision_of_relief_ppm: 250_000,
            },
            EvolutionPreset::Mature => EvolutionBudget {
                steps: 4,
                timestep_years: 12_000,
                climate_cadence_steps: 1,
                erodibility_e12: 20_000_000_000,
                discharge_exponent_milli: STREAM_POWER_M_MILLI,
                slope_exponent_milli: STREAM_POWER_N_MILLI,
                uplift_mm_per_year: 6,
                diffusivity_m2_per_year: 900_000,
                max_incision_of_relief_ppm: 250_000,
            },
            EvolutionPreset::Old => EvolutionBudget {
                steps: 8,
                timestep_years: 16_000,
                climate_cadence_steps: 1,
                erodibility_e12: 20_000_000_000,
                discharge_exponent_milli: STREAM_POWER_M_MILLI,
                slope_exponent_milli: STREAM_POWER_N_MILLI,
                uplift_mm_per_year: 4,
                diffusivity_m2_per_year: 1_800_000,
                max_incision_of_relief_ppm: 250_000,
            },
        }
    }

    pub fn validate(self) -> Result<(), PhysicalError> {
        let budget = self.budget();
        if budget.steps == 0
            || budget.timestep_years == 0
            || budget.climate_cadence_steps == 0
            || budget.erodibility_e12 == 0
            || budget.discharge_exponent_milli > 4_000
            || budget.slope_exponent_milli > 4_000
            || budget.uplift_mm_per_year > 1_000
            || budget.max_incision_of_relief_ppm == 0
            || budget.max_incision_of_relief_ppm > 1_000_000
        {
            return Err(PhysicalError::InvalidSettings(
                "terrain evolution budget is outside the bounded range".into(),
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EvolutionBudget {
    pub steps: u32,
    pub timestep_years: u32,
    pub climate_cadence_steps: u32,
    pub erodibility_e12: u64,
    pub discharge_exponent_milli: u16,
    pub slope_exponent_milli: u16,
    pub uplift_mm_per_year: i32,
    pub diffusivity_m2_per_year: u32,
    pub max_incision_of_relief_ppm: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DrainageEdge {
    pub source_cell: usize,
    pub destination_cell: usize,
    pub weight_ppm: u32,
    pub distance_metres: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DrainageMetrics {
    pub direct_runoff_m3_per_year: u64,
    pub routed_runoff_m3_per_year: u64,
    pub routed_edge_count: u32,
    pub drainage_density_ppm: u32,
    pub grid_anisotropy_ppm: u32,
    pub convergence_ppm: u32,
    pub outlet_count: u32,
    pub routing_surface_raise_max_mm: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DrainageField {
    pub grid: Grid,
    pub derivation_version: u16,
    pub routing_elevation_mm: Vec<i32>,
    pub fill_depth_mm: Vec<i32>,
    pub routing_order: Vec<u32>,
    pub slope_ppm: Vec<u32>,
    pub accumulation_m3_per_year: Vec<u64>,
    pub edges: Vec<DrainageEdge>,
    pub outlet_cells: Vec<usize>,
    pub metrics: DrainageMetrics,
}

impl DrainageField {
    pub fn validate_against(
        &self,
        field: &PhysicalField,
        climate: &ClimateField,
    ) -> Result<(), PhysicalError> {
        let sample_count = field.grid.sample_count();
        if self.derivation_version != EVOLUTION_DERIVATION_VERSION
            || self.grid != field.grid
            || climate.grid != field.grid
            || self.routing_elevation_mm.len() != sample_count
            || self.fill_depth_mm.len() != sample_count
            || self.routing_order.len() != sample_count
            || self.slope_ppm.len() != sample_count
            || self.accumulation_m3_per_year.len() != sample_count
        {
            return Err(PhysicalError::coded(
                PhysicalErrorCode::GeometryInvalid,
                "drainage field shape or derivation version is invalid",
            ));
        }
        if self.routing_elevation_mm.iter().any(|elevation| {
            !(-MAX_EVOLUTION_ELEVATION_MM..=MAX_EVOLUTION_ELEVATION_MM).contains(elevation)
        }) {
            return Err(PhysicalError::coded(
                PhysicalErrorCode::NumericNonFinite,
                "drainage field contains an unbounded value",
            ));
        }
        if self
            .fill_depth_mm
            .iter()
            .zip(&self.routing_elevation_mm)
            .zip(&field.elevations_mm)
            .any(|((fill, routed), actual)| *fill != routed.saturating_sub(*actual) || *fill < 0)
        {
            return Err(PhysicalError::coded(
                PhysicalErrorCode::GeometryInvalid,
                "drainage fill depth does not match the temporary routing surface",
            ));
        }
        let mut outgoing_weight = vec![0u32; sample_count];
        let mut outgoing_count = vec![0u32; sample_count];
        for edge in &self.edges {
            if edge.source_cell >= sample_count
                || edge.destination_cell >= sample_count
                || edge.source_cell == edge.destination_cell
                || edge.weight_ppm == 0
                || edge.weight_ppm > 1_000_000
                || edge.distance_metres == 0
                || self.routing_elevation_mm[edge.destination_cell]
                    > self.routing_elevation_mm[edge.source_cell]
                || (self.routing_elevation_mm[edge.destination_cell]
                    == self.routing_elevation_mm[edge.source_cell]
                    && self.routing_order[edge.destination_cell]
                        >= self.routing_order[edge.source_cell])
                || !field
                    .grid
                    .topology()
                    .are_neighbors(edge.source_cell, edge.destination_cell)
            {
                return Err(PhysicalError::coded(
                    PhysicalErrorCode::HydrologyCycle,
                    "drainage edge is uphill, cyclic, or topologically invalid",
                ));
            }
            outgoing_weight[edge.source_cell] = outgoing_weight[edge.source_cell]
                .checked_add(edge.weight_ppm)
                .ok_or_else(|| {
                    PhysicalError::coded(
                        PhysicalErrorCode::HydrologyCycle,
                        "drainage edge weights overflow",
                    )
                })?;
            outgoing_count[edge.source_cell] += 1;
        }
        for cell in 0..sample_count {
            let is_ocean = field.elevations_mm[cell] <= field.sea_level_mm;
            if is_ocean {
                if outgoing_count[cell] != 0 {
                    return Err(PhysicalError::coded(
                        PhysicalErrorCode::HydrologyCycle,
                        "ocean routing cell has a downstream edge",
                    ));
                }
            } else if outgoing_count[cell] == 0 || outgoing_weight[cell] != 1_000_000 {
                return Err(PhysicalError::coded(
                    PhysicalErrorCode::HydrologyInvalidSink,
                    "land routing cell has no normalized downstream destination",
                ));
            }
        }
        for outlet in &self.outlet_cells {
            if *outlet >= sample_count || field.elevations_mm[*outlet] > field.sea_level_mm {
                return Err(PhysicalError::coded(
                    PhysicalErrorCode::HydrologyInvalidSink,
                    "drainage outlet is not an ocean cell",
                ));
            }
        }
        let expected_outlets = (0..sample_count)
            .filter(|cell| field.elevations_mm[*cell] <= field.sea_level_mm)
            .collect::<Vec<_>>();
        if self.outlet_cells != expected_outlets {
            return Err(PhysicalError::coded(
                PhysicalErrorCode::HydrologyInvalidSink,
                "drainage outlet set does not cover the complete ocean boundary",
            ));
        }
        let expected_accumulation = accumulate_runoff(
            sample_count,
            &climate.runoff_volume_m3_per_year,
            &self.edges,
            &self.routing_order,
        )?;
        if expected_accumulation != self.accumulation_m3_per_year {
            return Err(PhysicalError::coded(
                PhysicalErrorCode::HydrologyCycle,
                "drainage accumulation does not match its normalized edges",
            ));
        }
        let direct_total = climate
            .runoff_volume_m3_per_year
            .iter()
            .try_fold(0u64, |total, value| total.checked_add(*value))
            .ok_or_else(|| {
                PhysicalError::coded(
                    PhysicalErrorCode::NumericNonFinite,
                    "direct runoff total overflowed its bounded integer range",
                )
            })?;
        let outlet_total = self
            .outlet_cells
            .iter()
            .try_fold(0u64, |total, cell| {
                total.checked_add(self.accumulation_m3_per_year[*cell])
            })
            .ok_or_else(|| {
                PhysicalError::coded(
                    PhysicalErrorCode::NumericNonFinite,
                    "outlet runoff total overflowed its bounded integer range",
                )
            })?;
        if direct_total != outlet_total {
            return Err(PhysicalError::coded(
                PhysicalErrorCode::HydrologyCycle,
                "drainage outlets do not account for all direct runoff",
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EvolutionMetrics {
    pub initial_relief_span_mm: i32,
    pub final_relief_span_mm: i32,
    pub relief_change_mm: i32,
    pub mean_absolute_elevation_change_mm: u32,
    pub erosion_work_m3: u64,
    pub uplift_work_m3: u64,
    pub max_step_relief_loss_mm: i32,
    pub drainage_density_ppm: u32,
    pub grid_anisotropy_ppm: u32,
    pub convergence_ppm: u32,
    pub tectonic_range_orientation_ppm: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvolutionField {
    pub grid: Grid,
    pub derivation_version: u16,
    pub preset: EvolutionPreset,
    pub before_elevations_mm: Vec<i32>,
    pub elevations_mm: Vec<i32>,
    pub drainage: DrainageField,
    pub metrics: EvolutionMetrics,
}

impl EvolutionField {
    pub fn validate_against(
        &self,
        before_field: &PhysicalField,
        final_field: &PhysicalField,
        climate: &ClimateField,
    ) -> Result<(), PhysicalError> {
        if self.derivation_version != EVOLUTION_DERIVATION_VERSION
            || self.grid != before_field.grid
            || self.grid != final_field.grid
            || self.before_elevations_mm != before_field.elevations_mm
            || self.elevations_mm != final_field.elevations_mm
            || self.elevations_mm.len() != self.grid.sample_count()
        {
            return Err(PhysicalError::coded(
                PhysicalErrorCode::GeometryInvalid,
                "evolution field does not match its before/after fields",
            ));
        }
        self.drainage.validate_against(final_field, climate)?;
        if self.elevations_mm.iter().any(|elevation| {
            !(-MAX_EVOLUTION_ELEVATION_MM..=MAX_EVOLUTION_ELEVATION_MM).contains(elevation)
        }) {
            return Err(PhysicalError::coded(
                PhysicalErrorCode::NumericNonFinite,
                "evolved elevation is outside the bounded signed field",
            ));
        }
        Ok(())
    }

    pub fn replace_drainage(&mut self, drainage: DrainageField) {
        self.drainage = drainage;
        self.metrics.drainage_density_ppm = self.drainage.metrics.drainage_density_ppm;
        self.metrics.grid_anisotropy_ppm = self.drainage.metrics.grid_anisotropy_ppm;
        self.metrics.convergence_ppm = self.drainage.metrics.convergence_ppm;
    }
}

fn physical_distance_metres(grid: Grid, first: usize, second: usize) -> u64 {
    grid.great_circle_distance(
        grid.center_radians(grid.row_col(first).0, grid.row_col(first).1),
        grid.center_radians(grid.row_col(second).0, grid.row_col(second).1),
    )
    .round()
    .max(1.0) as u64
}

fn priority_flood(field: &PhysicalField) -> Result<(Vec<i32>, Vec<u32>), PhysicalError> {
    let sample_count = field.grid.sample_count();
    let mut routing = field.elevations_mm.clone();
    let mut visited = vec![false; sample_count];
    let mut order = vec![u32::MAX; sample_count];
    let mut queue = BinaryHeap::new();
    let topology = field.grid.topology();
    for (cell, visited_cell) in visited.iter_mut().enumerate() {
        if field.elevations_mm[cell] <= field.sea_level_mm {
            *visited_cell = true;
            queue.push((Reverse(field.elevations_mm[cell]), Reverse(cell)));
        }
    }
    if queue.is_empty() {
        return Err(PhysicalError::coded(
            PhysicalErrorCode::HydrologyInvalidSink,
            "priority flood has no ocean outlet",
        ));
    }
    let mut next_order = 0u32;
    while let Some((Reverse(level), Reverse(cell))) = queue.pop() {
        if order[cell] != u32::MAX {
            continue;
        }
        order[cell] = next_order;
        next_order = next_order.saturating_add(1);
        for neighbor in topology.neighbors(cell) {
            if visited[*neighbor] {
                continue;
            }
            visited[*neighbor] = true;
            routing[*neighbor] = routing[*neighbor].max(level);
            queue.push((Reverse(routing[*neighbor]), Reverse(*neighbor)));
        }
    }
    if order.contains(&u32::MAX) {
        return Err(PhysicalError::coded(
            PhysicalErrorCode::HydrologyInvalidSink,
            "priority flood left an unvisited routing cell",
        ));
    }
    Ok((routing, order))
}

fn cell_xyz(grid: Grid, cell: usize) -> (f64, f64, f64) {
    let (row, col) = grid.row_col(cell);
    let (longitude, latitude) = grid.center_radians(row, col);
    let cos_lat = latitude.cos();
    (
        cos_lat * longitude.cos(),
        cos_lat * longitude.sin(),
        latitude.sin(),
    )
}

fn tangent_frame(up: (f64, f64, f64)) -> ((f64, f64, f64), (f64, f64, f64)) {
    let (x, y, z) = up;
    let east = if z.abs() > 0.995 {
        let length = (x * x + y * y).sqrt();
        if length < 1e-12 {
            (1.0, 0.0, 0.0)
        } else {
            (-y / length, x / length, 0.0)
        }
    } else {
        let length = (x * x + y * y).sqrt().max(1e-12);
        (-y / length, x / length, 0.0)
    };
    let north = (
        z * east.1 - y * east.2,
        x * east.2 - z * east.0,
        y * east.0 - x * east.1,
    );
    (east, north)
}

fn tangent_offset_metres(grid: Grid, origin: usize, neighbor: usize) -> Option<(f64, f64)> {
    let radius = grid.radius_metres as f64;
    let origin_xyz = cell_xyz(grid, origin);
    let neighbor_xyz = cell_xyz(grid, neighbor);
    let (east, north) = tangent_frame(origin_xyz);
    let delta = (
        neighbor_xyz.0 - origin_xyz.0,
        neighbor_xyz.1 - origin_xyz.1,
        neighbor_xyz.2 - origin_xyz.2,
    );
    let radial = delta.0 * origin_xyz.0 + delta.1 * origin_xyz.1 + delta.2 * origin_xyz.2;
    let tangent = (
        delta.0 - origin_xyz.0 * radial,
        delta.1 - origin_xyz.1 * radial,
        delta.2 - origin_xyz.2 * radial,
    );
    let east_m = (tangent.0 * east.0 + tangent.1 * east.1 + tangent.2 * east.2) * radius;
    let north_m = (tangent.0 * north.0 + tangent.1 * north.1 + tangent.2 * north.2) * radius;
    if east_m.hypot(north_m) < MIN_TANGENT_METRES {
        None
    } else {
        Some((east_m, north_m))
    }
}

fn wrapped_delta(from: f64, to: f64) -> f64 {
    let mut delta = to - from;
    while delta <= -std::f64::consts::PI {
        delta += std::f64::consts::TAU;
    }
    while delta > std::f64::consts::PI {
        delta -= std::f64::consts::TAU;
    }
    delta
}

fn facet_split(
    z0: f64,
    first: (usize, f64, f64, f64),
    second: (usize, f64, f64, f64),
) -> Option<(f64, f64, f64)> {
    let (_, x1, y1, z1) = first;
    let (_, x2, y2, z2) = second;
    let det = x1 * y2 - x2 * y1;
    if !det.is_finite() || det.abs() < 1e-12 {
        return None;
    }
    let ax = ((z1 - z0) * y2 - (z2 - z0) * y1) / det;
    let ay = (x1 * (z2 - z0) - x2 * (z1 - z0)) / det;
    if !ax.is_finite() || !ay.is_finite() {
        return None;
    }
    let slope = ax.hypot(ay);
    if slope <= 1e-18 {
        return None;
    }
    let down_x = -ax / slope;
    let down_y = -ay / slope;
    let a1 = y1.atan2(x1);
    let a2 = y2.atan2(x2);
    let ad = down_y.atan2(down_x);
    let span = wrapped_delta(a1, a2);
    if span.abs() < 1e-12 {
        return None;
    }
    let along = wrapped_delta(a1, ad);
    if along.signum() != span.signum() || along.abs() > span.abs() {
        return None;
    }
    let toward_second = (along / span).clamp(0.0, 1.0);
    Some((slope, 1.0 - toward_second, toward_second))
}

fn quantize_split(parts: &mut Vec<(usize, f64, u64)>) -> Vec<(usize, u32, u64)> {
    parts.retain(|part| part.1 > 0.0 && part.2 > 0);
    if parts.is_empty() {
        return Vec::new();
    }
    let weight_sum = parts.iter().map(|part| part.1).sum::<f64>();
    if !(weight_sum > 0.0) {
        return Vec::new();
    }
    let mut weights = parts
        .iter()
        .map(|part| ((part.1 / weight_sum) * 1_000_000.0).floor() as u32)
        .collect::<Vec<_>>();
    let total = weights.iter().map(|weight| u64::from(*weight)).sum::<u64>();
    if total < 1_000_000 {
        let remainder = (1_000_000 - total) as u32;
        let winner = parts
            .iter()
            .enumerate()
            .max_by(|(_, first), (_, second)| {
                first
                    .1
                    .partial_cmp(&second.1)
                    .unwrap_or(std::cmp::Ordering::Equal)
                    .then_with(|| second.0.cmp(&first.0))
            })
            .map(|(index, _)| index)
            .unwrap_or(0);
        weights[winner] = weights[winner].saturating_add(remainder);
    }
    parts
        .iter()
        .zip(weights)
        .filter(|(_, weight)| *weight > 0)
        .map(|((cell, _, distance), weight)| (*cell, weight, *distance))
        .collect()
}

fn dinfinity_edges(
    field: &PhysicalField,
    routing: &[i32],
    order: &[u32],
    cell: usize,
) -> Vec<DrainageEdge> {
    let mut neighbors = field
        .grid
        .topology()
        .neighbors(cell)
        .iter()
        .copied()
        .filter_map(|destination| {
            let (east, north) = tangent_offset_metres(field.grid, cell, destination)?;
            Some((
                destination,
                east,
                north,
                f64::from(routing[destination]),
                physical_distance_metres(field.grid, cell, destination),
            ))
        })
        .collect::<Vec<_>>();
    neighbors.sort_unstable_by(|first, second| {
        first
            .2
            .atan2(first.1)
            .partial_cmp(&second.2.atan2(second.1))
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| first.0.cmp(&second.0))
    });
    let z0 = f64::from(routing[cell]);
    let mut best: Option<(f64, usize, usize, f64, f64, u64, u64)> = None;
    if neighbors.len() >= 2 {
        for index in 0..neighbors.len() {
            let first = neighbors[index];
            let second = neighbors[(index + 1) % neighbors.len()];
            if first.0 == second.0 {
                continue;
            }
            let Some((slope, w1, w2)) = facet_split(
                z0,
                (first.0, first.1, first.2, first.3),
                (second.0, second.1, second.2, second.3),
            ) else {
                continue;
            };
            if first.3 >= z0 && second.3 >= z0 {
                continue;
            }
            let better = best
                .map(|(best_slope, ..)| {
                    slope
                        .partial_cmp(&best_slope)
                        .unwrap_or(std::cmp::Ordering::Equal)
                        == std::cmp::Ordering::Greater
                })
                .unwrap_or(true);
            if better {
                best = Some((slope, first.0, second.0, w1, w2, first.4, second.4));
            }
        }
    }
    let mut parts = if let Some((_, first, second, w1, w2, d1, d2)) = best {
        let mut parts = Vec::new();
        if w1 > 0.0 && routing[first] <= routing[cell] {
            if routing[first] < routing[cell] || order[first] < order[cell] {
                parts.push((first, w1, d1));
            }
        }
        if w2 > 0.0 && routing[second] <= routing[cell] {
            if routing[second] < routing[cell] || order[second] < order[cell] {
                parts.push((second, w2, d2));
            }
        }
        parts
    } else {
        Vec::new()
    };
    if parts.is_empty() {
        parts = field
            .grid
            .topology()
            .neighbors(cell)
            .iter()
            .copied()
            .filter_map(|destination| {
                let lower = routing[destination] < routing[cell];
                let plateau =
                    routing[destination] == routing[cell] && order[destination] < order[cell];
                if !lower && !plateau {
                    return None;
                }
                Some((
                    destination,
                    1.0,
                    physical_distance_metres(field.grid, cell, destination),
                ))
            })
            .collect();
    }
    quantize_split(&mut parts)
        .into_iter()
        .map(
            |(destination_cell, weight_ppm, distance_metres)| DrainageEdge {
                source_cell: cell,
                destination_cell,
                weight_ppm,
                distance_metres,
            },
        )
        .collect()
}

fn checked_add(target: &mut u64, value: u64) -> Result<(), PhysicalError> {
    *target = target.checked_add(value).ok_or_else(|| {
        PhysicalError::coded(
            PhysicalErrorCode::NumericNonFinite,
            "drainage accumulation overflowed its bounded integer range",
        )
    })?;
    Ok(())
}

fn accumulate_runoff(
    sample_count: usize,
    direct_runoff: &[u64],
    edges: &[DrainageEdge],
    order: &[u32],
) -> Result<Vec<u64>, PhysicalError> {
    if direct_runoff.len() != sample_count || order.len() != sample_count {
        return Err(PhysicalError::coded(
            PhysicalErrorCode::GeometryInvalid,
            "drainage accumulation inputs do not match the grid",
        ));
    }
    let mut accumulation = direct_runoff.to_vec();
    let mut edges_by_source = vec![Vec::<DrainageEdge>::new(); sample_count];
    for edge in edges.iter().copied() {
        if edge.source_cell >= sample_count || edge.destination_cell >= sample_count {
            return Err(PhysicalError::coded(
                PhysicalErrorCode::HydrologyCycle,
                "drainage edge is outside the grid",
            ));
        }
        edges_by_source[edge.source_cell].push(edge);
    }
    let mut order_desc = (0..sample_count).collect::<Vec<_>>();
    order_desc.sort_unstable_by(|first, second| {
        order[*second]
            .cmp(&order[*first])
            .then_with(|| second.cmp(first))
    });
    for source in order_desc {
        if edges_by_source[source].is_empty() {
            continue;
        }
        if edges_by_source[source]
            .iter()
            .any(|edge| order[edge.destination_cell] >= order[source])
        {
            return Err(PhysicalError::coded(
                PhysicalErrorCode::HydrologyCycle,
                "drainage graph contains a cycle",
            ));
        }
        let source_volume = accumulation[source];
        let mut distributed = 0u64;
        for (index, edge) in edges_by_source[source].iter().enumerate() {
            let share = if index + 1 == edges_by_source[source].len() {
                source_volume.checked_sub(distributed).ok_or_else(|| {
                    PhysicalError::coded(
                        PhysicalErrorCode::HydrologyCycle,
                        "drainage split distributed more volume than its source",
                    )
                })?
            } else {
                source_volume
                    .checked_mul(u64::from(edge.weight_ppm))
                    .ok_or_else(|| {
                        PhysicalError::coded(
                            PhysicalErrorCode::NumericNonFinite,
                            "drainage weighted runoff overflowed its bounded integer range",
                        )
                    })?
                    / 1_000_000
            };
            distributed = distributed.checked_add(share).ok_or_else(|| {
                PhysicalError::coded(
                    PhysicalErrorCode::NumericNonFinite,
                    "drainage split total overflowed its bounded integer range",
                )
            })?;
            checked_add(&mut accumulation[edge.destination_cell], share)?;
        }
        if distributed != source_volume {
            return Err(PhysicalError::coded(
                PhysicalErrorCode::HydrologyCycle,
                "drainage accumulation did not conserve source runoff",
            ));
        }
    }
    Ok(accumulation)
}

pub fn derive_drainage(
    field: &PhysicalField,
    climate: &ClimateField,
) -> Result<DrainageField, PhysicalError> {
    field.validate().map_err(PhysicalError::InvalidSource)?;
    climate.validate()?;
    if climate.grid != field.grid
        || climate.runoff_volume_m3_per_year.len() != field.grid.sample_count()
    {
        return Err(PhysicalError::coded(
            PhysicalErrorCode::GeometryInvalid,
            "drainage climate field does not match the physical field",
        ));
    }
    let (routing, order) = priority_flood(field)?;
    let mut slope_ppm = vec![0u32; field.grid.sample_count()];
    let mut edges = Vec::new();
    for cell in 0..field.grid.sample_count() {
        if field.elevations_mm[cell] <= field.sea_level_mm {
            continue;
        }
        let cell_edges = dinfinity_edges(field, &routing, &order, cell);
        if cell_edges.is_empty() {
            return Err(PhysicalError::coded(
                PhysicalErrorCode::HydrologyInvalidSink,
                format!("land cell {cell} has no routed downstream destination"),
            ));
        }
        slope_ppm[cell] = {
            let mut weighted = 0u128;
            let mut weight_sum = 0u128;
            for edge in &cell_edges {
                let drop = i64::from(field.elevations_mm[cell])
                    - i64::from(field.elevations_mm[edge.destination_cell]);
                if drop > 0 {
                    let slope = (drop as u128).saturating_mul(1_000)
                        / u128::from(edge.distance_metres.max(1));
                    weighted += slope.saturating_mul(u128::from(edge.weight_ppm));
                    weight_sum += u128::from(edge.weight_ppm);
                }
            }
            if weight_sum == 0 {
                0
            } else {
                (weighted / weight_sum).min(u128::from(u32::MAX)) as u32
            }
        };
        edges.extend(cell_edges);
    }
    let accumulation = accumulate_runoff(
        field.grid.sample_count(),
        &climate.runoff_volume_m3_per_year,
        &edges,
        &order,
    )?;
    let direct_total = climate
        .runoff_volume_m3_per_year
        .iter()
        .try_fold(0u64, |total, value| total.checked_add(*value))
        .ok_or_else(|| {
            PhysicalError::coded(
                PhysicalErrorCode::NumericNonFinite,
                "direct runoff total overflowed its bounded integer range",
            )
        })?;
    let outlet_cells = (0..field.grid.sample_count())
        .filter(|cell| field.elevations_mm[*cell] <= field.sea_level_mm)
        .collect::<Vec<_>>();
    let routed_total = outlet_cells
        .iter()
        .try_fold(0u64, |total, cell| total.checked_add(accumulation[*cell]))
        .ok_or_else(|| {
            PhysicalError::coded(
                PhysicalErrorCode::NumericNonFinite,
                "outlet runoff total overflowed its bounded integer range",
            )
        })?;
    if routed_total != direct_total {
        return Err(PhysicalError::coded(
            PhysicalErrorCode::HydrologyCycle,
            format!(
                "drainage runoff is not accounted for: direct={direct_total} routed={routed_total}"
            ),
        ));
    }
    let mut sources = vec![false; field.grid.sample_count()];
    let mut edges_by_source = vec![Vec::<DrainageEdge>::new(); field.grid.sample_count()];
    for edge in &edges {
        sources[edge.source_cell] = true;
        edges_by_source[edge.source_cell].push(*edge);
    }
    let land_count = field
        .elevations_mm
        .iter()
        .filter(|elevation| **elevation > field.sea_level_mm)
        .count();
    let routed_source_count = sources.iter().filter(|value| **value).count();
    let drainage_density_ppm = if land_count == 0 {
        0
    } else {
        (u64::from(routed_source_count as u32) * 1_000_000 / land_count as u64) as u32
    };
    let grid_anisotropy_ppm = if routed_source_count == 0 {
        0
    } else {
        let source_max_sum = sources
            .iter()
            .enumerate()
            .filter(|(_, included)| **included)
            .map(|(cell, _)| {
                edges_by_source[cell]
                    .iter()
                    .map(|edge| edge.weight_ppm)
                    .max()
                    .unwrap_or(0)
            })
            .map(u64::from)
            .sum::<u64>();
        (source_max_sum / routed_source_count as u64) as u32
    };
    let fill_depth_mm = field
        .elevations_mm
        .iter()
        .zip(&routing)
        .map(|(actual, routed)| routed.saturating_sub(*actual))
        .collect::<Vec<_>>();
    let routing_surface_raise_max_mm = fill_depth_mm.iter().copied().max().unwrap_or(0);
    let convergence_ppm = if direct_total == 0 {
        1_000_000
    } else {
        (u128::from(routed_total) * MILLION / u128::from(direct_total)).min(MILLION) as u32
    };
    let routed_edge_count = edges.len() as u32;
    let outlet_count = outlet_cells.len() as u32;
    let drainage = DrainageField {
        grid: field.grid,
        derivation_version: EVOLUTION_DERIVATION_VERSION,
        routing_elevation_mm: routing,
        fill_depth_mm,
        routing_order: order,
        slope_ppm,
        accumulation_m3_per_year: accumulation,
        edges,
        outlet_cells,
        metrics: DrainageMetrics {
            direct_runoff_m3_per_year: direct_total,
            routed_runoff_m3_per_year: routed_total,
            routed_edge_count,
            drainage_density_ppm,
            grid_anisotropy_ppm,
            convergence_ppm,
            outlet_count,
            routing_surface_raise_max_mm,
        },
    };
    drainage.validate_against(field, climate)?;
    Ok(drainage)
}

fn uplift_signal(world: &TectonicWorld) -> Vec<u32> {
    let mut signal = vec![0u32; world.grid.sample_count()];
    let topology = world.grid.topology();
    for boundary in &world.boundaries {
        if boundary.kind != BoundaryKind::Convergent {
            continue;
        }
        for cell in [boundary.first_cell, boundary.second_cell] {
            signal[cell] = signal[cell].max(800_000);
            for neighbor in topology.neighbors(cell) {
                signal[*neighbor] = signal[*neighbor].max(450_000);
            }
        }
    }
    for center in &world.volcanic_centers {
        signal[center.cell] = signal[center.cell].max(center.intensity_ppm);
    }
    signal
}

fn relief_span(values: &[i32]) -> i32 {
    values.iter().max().unwrap_or(&0) - values.iter().min().unwrap_or(&0)
}

fn highland_mask_overlap_ppm(before: &[i32], after: &[i32]) -> u32 {
    if before.is_empty() || before.len() != after.len() {
        return 0;
    }
    let mut before_sorted = before.to_vec();
    let mut after_sorted = after.to_vec();
    before_sorted.sort_unstable();
    after_sorted.sort_unstable();
    let before_threshold = before_sorted[before_sorted.len() * 3 / 4];
    let after_threshold = after_sorted[after_sorted.len() * 3 / 4];
    let before_highland = before
        .iter()
        .map(|elevation| *elevation >= before_threshold)
        .collect::<Vec<_>>();
    let highland_count = before_highland.iter().filter(|included| **included).count();
    if highland_count == 0 {
        return 1_000_000;
    }
    let overlap = before_highland
        .iter()
        .enumerate()
        .filter(|(cell, included)| **included && after[*cell] >= after_threshold)
        .count();
    (overlap as u64 * 1_000_000 / highland_count as u64) as u32
}

fn checked_power(base: f64, exponent_milli: u16) -> Result<f64, PhysicalError> {
    if !base.is_finite() || base < 0.0 {
        return Err(PhysicalError::coded(
            PhysicalErrorCode::NumericNonFinite,
            "stream-power base is non-finite or negative",
        ));
    }
    let value = match exponent_milli {
        0 => 1.0,
        500 => base.sqrt(),
        1000 => base,
        _ => base.powf(f64::from(exponent_milli) / 1_000.0),
    };
    if !value.is_finite() {
        return Err(PhysicalError::coded(
            PhysicalErrorCode::NumericNonFinite,
            "stream-power evaluation produced a non-finite value",
        ));
    }
    Ok(value)
}

fn stream_power_incision_mm(
    budget: EvolutionBudget,
    timestep_years: u32,
    accumulation_m3_per_year: u64,
    slope_ppm: u32,
    removable_relief_mm: i32,
) -> Result<i32, PhysicalError> {
    let discharge = accumulation_m3_per_year as f64 / SECONDS_PER_YEAR as f64;
    if !discharge.is_finite() || discharge < 0.0 {
        return Err(PhysicalError::coded(
            PhysicalErrorCode::NumericNonFinite,
            "stream-power discharge is negative or non-finite",
        ));
    }
    let slope = f64::from(slope_ppm) / 1_000_000.0;
    let k = budget.erodibility_e12 as f64 / 1_000_000_000_000.0;
    let rate = k
        * checked_power(discharge, budget.discharge_exponent_milli)?
        * checked_power(slope.max(0.0), budget.slope_exponent_milli)?;
    if !rate.is_finite() || rate < 0.0 {
        return Err(PhysicalError::coded(
            PhysicalErrorCode::NumericNonFinite,
            "stream-power incision rate is invalid",
        ));
    }
    let incision_mm = (rate * f64::from(timestep_years) * 1_000.0).round() as i64;
    if incision_mm < 0 {
        return Err(PhysicalError::coded(
            PhysicalErrorCode::NumericNonFinite,
            "stream-power incision is negative",
        ));
    }
    let relief_limit = (i64::from(removable_relief_mm.max(0))
        * i64::from(budget.max_incision_of_relief_ppm)
        / 1_000_000)
        .max(0);
    let incision = incision_mm
        .min(relief_limit)
        .min(i64::from(MAX_RELIEF_LOSS_PER_STEP_MM));
    Ok(incision as i32)
}

fn coastal_incision_scale_ppm(distance_mm: i64, coast_mm: i64, exposure_ppm: u32) -> u32 {
    if distance_mm == i64::MAX || coast_mm <= 0 || distance_mm >= coast_mm {
        return 1_000_000;
    }
    let t = 1.0 - distance_mm as f64 / coast_mm as f64;
    let exposure = 0.35 + 0.65 * (f64::from(exposure_ppm) / 1_000_000.0);
    (1_000_000.0 + 1_200_000.0 * t * exposure).round() as u32
}

fn ocean_exposure_ppm(grid: Grid, is_ocean: impl Fn(usize) -> bool, cell: usize) -> u32 {
    let topology = grid.topology();
    let neighbors = topology.neighbors(cell);
    if neighbors.is_empty() {
        return 0;
    }
    let ocean = neighbors
        .iter()
        .filter(|neighbor| is_ocean(**neighbor))
        .count();
    (ocean as u32 * 1_000_000 / neighbors.len() as u32).min(1_000_000)
}

fn mouth_incision_scale_ppm(accumulation_m3_per_year: u64, distance_mm: i64, coast_mm: i64) -> u32 {
    if accumulation_m3_per_year < 40_000_000
        || distance_mm == i64::MAX
        || distance_mm >= coast_mm.saturating_mul(2)
    {
        return 1_000_000;
    }
    1_800_000
}

fn deposition_mm(
    accumulation_m3_per_year: u64,
    slope_ppm: u32,
    timestep_years: u32,
    coast_distance_mm: i64,
    coast_mm: i64,
) -> i32 {
    if slope_ppm > 85_000 || accumulation_m3_per_year < 8_000_000 {
        return 0;
    }
    let slope_f = 1.0 - f64::from(slope_ppm) / 85_000.0;
    let mouth = if coast_distance_mm != i64::MAX && coast_distance_mm < coast_mm {
        1.8
    } else {
        0.35
    };
    let millimetres = (accumulation_m3_per_year as f64).sqrt()
        * 2.2e-8
        * slope_f
        * mouth
        * f64::from(timestep_years);
    millimetres
        .round()
        .clamp(0.0, f64::from(MAX_DEPOSITION_PER_STEP_MM)) as i32
}

fn removable_relief_by_cell(
    elevations_mm: &[i32],
    sea_level_mm: i32,
    edges: &[DrainageEdge],
) -> Vec<i32> {
    let mut removable = elevations_mm
        .iter()
        .map(|elevation| elevation.saturating_sub(sea_level_mm).max(0))
        .collect::<Vec<_>>();
    for edge in edges {
        let drop = elevations_mm[edge.source_cell]
            .saturating_sub(elevations_mm[edge.destination_cell])
            .max(0);
        removable[edge.source_cell] = removable[edge.source_cell].min(drop);
    }
    removable
}

fn metrics_for(
    before: &[i32],
    after: &[i32],
    drainage: &DrainageField,
    erosion_work_m3: u64,
    uplift_work_m3: u64,
    max_step_relief_loss_mm: i32,
) -> EvolutionMetrics {
    let absolute_change = before
        .iter()
        .zip(after)
        .map(|(first, second)| u64::from(first.abs_diff(*second)))
        .sum::<u64>();
    let mean_absolute_elevation_change_mm = if before.is_empty() {
        0
    } else {
        (absolute_change / before.len() as u64) as u32
    };
    EvolutionMetrics {
        initial_relief_span_mm: relief_span(before),
        final_relief_span_mm: relief_span(after),
        relief_change_mm: relief_span(after) - relief_span(before),
        mean_absolute_elevation_change_mm,
        erosion_work_m3,
        uplift_work_m3,
        max_step_relief_loss_mm,
        drainage_density_ppm: drainage.metrics.drainage_density_ppm,
        grid_anisotropy_ppm: drainage.metrics.grid_anisotropy_ppm,
        convergence_ppm: drainage.metrics.convergence_ppm,
        tectonic_range_orientation_ppm: highland_mask_overlap_ppm(before, after),
    }
}

pub fn evolve_terrain(
    before_field: &PhysicalField,
    climate: &ClimateField,
    tectonics: &TectonicWorld,
    settings: EvolutionSettings,
    progress: &mut dyn ProgressSink,
) -> Result<EvolutionField, PhysicalError> {
    before_field
        .validate()
        .map_err(PhysicalError::InvalidSource)?;
    climate.validate_against(before_field)?;
    if tectonics.grid != before_field.grid || tectonics.elevations_mm != before_field.elevations_mm
    {
        return Err(PhysicalError::coded(
            PhysicalErrorCode::GeometryInvalid,
            "terrain evolution tectonic field does not match the initial field",
        ));
    }
    settings.validate()?;
    let budget = settings.budget();
    let signal = uplift_signal(tectonics);
    let topology = before_field.grid.topology();
    let row_areas = before_field.grid.row_areas();
    let min_spacing_m = (0..before_field.grid.sample_count())
        .flat_map(|cell| {
            let (row, _) = before_field.grid.row_col(cell);
            topology
                .neighbors(cell)
                .iter()
                .copied()
                .filter_map(move |neighbor| {
                    let (neighbor_row, _) = before_field.grid.row_col(neighbor);
                    let polar_row_pair = (row == 0 && neighbor_row == 0)
                        || (row + 1 == before_field.grid.height
                            && neighbor_row + 1 == before_field.grid.height);
                    if polar_row_pair {
                        None
                    } else {
                        Some(physical_distance_metres(before_field.grid, cell, neighbor))
                    }
                })
        })
        .min()
        .unwrap_or(1)
        .max(1);
    let cfl_years = if budget.diffusivity_m2_per_year == 0 {
        u64::from(budget.timestep_years)
    } else {
        (u128::from(min_spacing_m).saturating_mul(u128::from(min_spacing_m))
            / (4u128.saturating_mul(u128::from(budget.diffusivity_m2_per_year))))
        .max(1) as u64
    };
    let timestep_years = budget.timestep_years.min(cfl_years as u32).max(1);
    let before = before_field.elevations_mm.clone();
    let mut current = before.clone();
    let mut erosion_work_m3 = 0u64;
    let mut uplift_work_m3 = 0u64;
    let mut max_step_relief_loss_mm = 0;
    let mut active_climate = climate.clone();
    progress.report(ProgressPhase::ErodingLandscape, 0, budget.steps)?;
    for step in 0..budget.steps {
        progress.check_cancelled()?;
        let step_field = PhysicalField {
            grid: before_field.grid,
            seed: before_field.seed,
            retry_index: before_field.retry_index,
            target_land_fraction_ppm: before_field.target_land_fraction_ppm,
            sea_level_mm: before_field.sea_level_mm,
            elevations_mm: current.clone(),
        };
        if step > 0 && step % budget.climate_cadence_steps == 0 {
            active_climate = derive_current_climate(
                &step_field,
                ClimateSettings::default_for(before_field.grid),
                before_field.seed,
                before_field.retry_index,
                progress,
            )?;
        }
        let drainage = derive_drainage(&step_field, &active_climate)?;
        let outline_locked = step * 2 >= budget.steps;
        let mut coast_dist = Vec::new();
        let mut coast_exposure = Vec::new();
        if outline_locked {
            let ocean_cells = current
                .iter()
                .enumerate()
                .filter(|(_, elevation)| **elevation <= before_field.sea_level_mm)
                .map(|(cell, _)| cell)
                .collect::<Vec<_>>();
            let (dist, nearest) = geodesic_nearest(
                before_field.grid,
                &ocean_cells,
                COASTAL_EROSION_RADIUS_MM.saturating_mul(2),
            );
            let source_exposure = ocean_cells
                .iter()
                .map(|cell| {
                    ocean_exposure_ppm(
                        before_field.grid,
                        |other| current[other] <= before_field.sea_level_mm,
                        *cell,
                    )
                })
                .collect::<Vec<_>>();
            coast_exposure = nearest
                .iter()
                .map(|index| {
                    if *index == u32::MAX || (*index as usize) >= source_exposure.len() {
                        0
                    } else {
                        source_exposure[*index as usize]
                    }
                })
                .collect();
            coast_dist = dist;
        }
        let mut incised = current.clone();
        let removable_relief =
            removable_relief_by_cell(&current, before_field.sea_level_mm, &drainage.edges);
        for cell in 0..current.len() {
            if cell % 128 == 0 {
                progress.check_cancelled()?;
            }
            if current[cell] <= before_field.sea_level_mm {
                continue;
            }
            let base_incision = stream_power_incision_mm(
                budget,
                timestep_years,
                drainage.accumulation_m3_per_year[cell],
                drainage.slope_ppm[cell],
                removable_relief[cell],
            )?;
            let mut incision = base_incision;
            let mut deposit = 0i32;
            if outline_locked {
                let distance = coast_dist.get(cell).copied().unwrap_or(i64::MAX);
                let exposure = coast_exposure.get(cell).copied().unwrap_or(0);
                let coastal =
                    coastal_incision_scale_ppm(distance, COASTAL_EROSION_RADIUS_MM, exposure);
                let mouth = mouth_incision_scale_ppm(
                    drainage.accumulation_m3_per_year[cell],
                    distance,
                    COASTAL_EROSION_RADIUS_MM,
                );
                let scaled = ((i64::from(base_incision) * i64::from(coastal) / 1_000_000)
                    * i64::from(mouth)
                    / 1_000_000)
                    .min(i64::from(removable_relief[cell]))
                    .min(i64::from(MAX_RELIEF_LOSS_PER_STEP_MM))
                    .max(0) as i32;
                let extra = scaled.saturating_sub(base_incision);
                let land_floor = before_field.sea_level_mm.saturating_add(1);
                let max_keep_land = current[cell].saturating_sub(land_floor).max(0);
                incision = (base_incision.saturating_add(extra))
                    .min(max_keep_land)
                    .min(removable_relief[cell]);
                deposit = deposition_mm(
                    drainage.accumulation_m3_per_year[cell],
                    drainage.slope_ppm[cell],
                    timestep_years,
                    distance,
                    COASTAL_EROSION_RADIUS_MM,
                );
            }
            max_step_relief_loss_mm = max_step_relief_loss_mm.max(incision);
            let evolved = i64::from(current[cell]) - i64::from(incision);
            if evolved < i64::from(-MAX_EVOLUTION_ELEVATION_MM)
                || evolved > i64::from(MAX_EVOLUTION_ELEVATION_MM)
            {
                return Err(PhysicalError::coded(
                    PhysicalErrorCode::NumericNonFinite,
                    "terrain evolution exceeded the bounded signed elevation field",
                ));
            }
            let area = row_areas[before_field.grid.row_col(cell).0 as usize];
            erosion_work_m3 = erosion_work_m3
                .saturating_add((f64::from(incision) * area / 1_000.0).round().max(0.0) as u64);
            let deposited = evolved + i64::from(deposit);
            if deposited < i64::from(-MAX_EVOLUTION_ELEVATION_MM)
                || deposited > i64::from(MAX_EVOLUTION_ELEVATION_MM)
            {
                return Err(PhysicalError::coded(
                    PhysicalErrorCode::NumericNonFinite,
                    "terrain evolution exceeded the bounded signed elevation field",
                ));
            }
            incised[cell] = deposited as i32;
        }
        let mut diffused = incised.clone();
        for cell in 0..current.len() {
            if incised[cell] <= before_field.sea_level_mm {
                continue;
            }
            let mut delta = 0.0;
            let mut lambda_sum = 0.0;
            for neighbor in topology.neighbors(cell) {
                let distance = physical_distance_metres(before_field.grid, cell, *neighbor) as f64;
                let lambda = f64::from(budget.diffusivity_m2_per_year) * f64::from(timestep_years)
                    / distance.max(1.0).powi(2);
                lambda_sum += lambda;
                delta += lambda * f64::from(incised[*neighbor] - incised[cell]);
            }
            if lambda_sum > 0.5 {
                delta *= 0.5 / lambda_sum;
            }
            if !delta.is_finite() {
                return Err(PhysicalError::coded(
                    PhysicalErrorCode::NumericNonFinite,
                    "hillslope diffusion produced a non-finite update",
                ));
            }
            let evolved = (f64::from(incised[cell]) + delta).round() as i64;
            if evolved < i64::from(-MAX_EVOLUTION_ELEVATION_MM)
                || evolved > i64::from(MAX_EVOLUTION_ELEVATION_MM)
            {
                return Err(PhysicalError::coded(
                    PhysicalErrorCode::NumericNonFinite,
                    "terrain evolution exceeded the bounded signed elevation field",
                ));
            }
            diffused[cell] = evolved as i32;
        }
        let mut next = diffused;
        for cell in 0..current.len() {
            if next[cell] <= before_field.sea_level_mm {
                continue;
            }
            let uplift = (i64::from(budget.uplift_mm_per_year)
                * i64::from(timestep_years)
                * i64::from(signal[cell])
                / 1_000_000) as i32;
            let evolved = i64::from(next[cell]) + i64::from(uplift);
            if evolved < i64::from(-MAX_EVOLUTION_ELEVATION_MM)
                || evolved > i64::from(MAX_EVOLUTION_ELEVATION_MM)
            {
                return Err(PhysicalError::coded(
                    PhysicalErrorCode::NumericNonFinite,
                    "terrain evolution exceeded the bounded signed elevation field",
                ));
            }
            let area = row_areas[before_field.grid.row_col(cell).0 as usize];
            if uplift > 0 {
                uplift_work_m3 = uplift_work_m3
                    .saturating_add((f64::from(uplift) * area / 1_000.0).round().max(0.0) as u64);
            }
            next[cell] = evolved as i32;
        }
        current = next;
        progress.report(ProgressPhase::ErodingLandscape, step + 1, budget.steps)?;
    }
    let final_field = PhysicalField {
        grid: before_field.grid,
        seed: before_field.seed,
        retry_index: before_field.retry_index,
        target_land_fraction_ppm: before_field.target_land_fraction_ppm,
        sea_level_mm: before_field.sea_level_mm,
        elevations_mm: current.clone(),
    };
    let drainage = derive_drainage(&final_field, climate)?;
    let evolution = EvolutionField {
        grid: before_field.grid,
        derivation_version: EVOLUTION_DERIVATION_VERSION,
        preset: settings.preset,
        before_elevations_mm: before,
        elevations_mm: current,
        metrics: metrics_for(
            &before_field.elevations_mm,
            &final_field.elevations_mm,
            &drainage,
            erosion_work_m3,
            uplift_work_m3,
            max_step_relief_loss_mm,
        ),
        drainage,
    };
    evolution.validate_against(before_field, &final_field, climate)?;
    Ok(evolution)
}

/// Rebuild disposable routing and before/after diagnostics for an accepted
/// final source. The initial elevation is supplied by deterministic tectonic
/// regeneration because the locked source payload stores only the final field.
pub fn diagnostics_from_before_after(
    before_elevations_mm: Vec<i32>,
    final_field: &PhysicalField,
    climate: &ClimateField,
    preset: EvolutionPreset,
) -> Result<EvolutionField, PhysicalError> {
    if before_elevations_mm.len() != final_field.grid.sample_count() {
        return Err(PhysicalError::coded(
            PhysicalErrorCode::GeometryInvalid,
            "before/after evolution diagnostics do not match the grid",
        ));
    }
    let before_field = PhysicalField {
        grid: final_field.grid,
        seed: final_field.seed,
        retry_index: final_field.retry_index,
        target_land_fraction_ppm: final_field.target_land_fraction_ppm,
        sea_level_mm: final_field.sea_level_mm,
        elevations_mm: before_elevations_mm.clone(),
    };
    let drainage = derive_drainage(final_field, climate)?;
    let evolution = EvolutionField {
        grid: final_field.grid,
        derivation_version: EVOLUTION_DERIVATION_VERSION,
        preset,
        before_elevations_mm,
        elevations_mm: final_field.elevations_mm.clone(),
        metrics: metrics_for(
            &before_field.elevations_mm,
            &final_field.elevations_mm,
            &drainage,
            0,
            0,
            0,
        ),
        drainage,
    };
    evolution.validate_against(&before_field, final_field, climate)?;
    Ok(evolution)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        climate::derive_current_climate, tectonics::generate_tectonic_world, NoopProgress,
    };

    fn fixture_parts(preset: EvolutionPreset) -> (PhysicalField, ClimateField, EvolutionField) {
        let grid = Grid::new(32, 16, crate::DEFAULT_RADIUS_METRES).unwrap();
        let settings = crate::tectonics::TectonicSettings::default_for(grid);
        let mut progress = NoopProgress;
        let tectonics =
            generate_tectonic_world(grid, settings, 300_000, 831_429, 0, &mut progress).unwrap();
        let sea_level_mm =
            crate::solve_sea_level(&grid, &tectonics.elevations_mm, 300_000).unwrap();
        let field = PhysicalField {
            grid,
            seed: 831_429,
            retry_index: 0,
            target_land_fraction_ppm: 300_000,
            sea_level_mm,
            elevations_mm: tectonics.elevations_mm.clone(),
        };
        let climate = derive_current_climate(
            &field,
            crate::climate::ClimateSettings::default_for(grid),
            field.seed,
            field.retry_index,
            &mut progress,
        )
        .unwrap();
        let evolution = evolve_terrain(
            &field,
            &climate,
            &tectonics,
            EvolutionSettings { preset },
            &mut progress,
        )
        .unwrap();
        (field, climate, evolution)
    }

    fn fixture(preset: EvolutionPreset) -> EvolutionField {
        fixture_parts(preset).2
    }

    #[test]
    fn priority_flood_and_flow_are_acyclic_and_conservative() {
        let evolution = fixture(EvolutionPreset::Mature);
        assert!(evolution.drainage.metrics.convergence_ppm == 1_000_000);
        assert!(evolution
            .drainage
            .edges
            .iter()
            .all(|edge| edge.weight_ppm > 0));
        assert!(evolution.drainage.edges.iter().all(|edge| {
            evolution.drainage.routing_elevation_mm[edge.destination_cell]
                <= evolution.drainage.routing_elevation_mm[edge.source_cell]
        }));
        assert!(evolution.drainage.metrics.routing_surface_raise_max_mm >= 0);
        assert!(evolution.metrics.grid_anisotropy_ppm <= ROUTING_ANISOTROPY_LIMIT_PPM);
        assert_eq!(
            evolution.drainage.metrics.direct_runoff_m3_per_year,
            evolution.drainage.metrics.routed_runoff_m3_per_year
        );
    }

    #[test]
    fn priority_flood_raises_only_the_temporary_bowl_surface() {
        let grid = Grid::new(6, 4, crate::DEFAULT_RADIUS_METRES).unwrap();
        let mut elevations = vec![-1_000; grid.sample_count()];
        for row in 1..grid.height {
            for column in 0..grid.width {
                elevations[grid.index(row, column)] = 100;
            }
        }
        let bowl = grid.index(2, 2);
        elevations[bowl] = 10;
        let field = PhysicalField {
            grid,
            seed: 1,
            retry_index: 0,
            target_land_fraction_ppm: 300_000,
            sea_level_mm: 0,
            elevations_mm: elevations.clone(),
        };
        let (routing, _) = priority_flood(&field).unwrap();
        assert_eq!(field.elevations_mm, elevations);
        assert_eq!(routing[bowl], 100);
        assert!(routing[bowl] > field.elevations_mm[bowl]);
        let climate = synthetic_climate(&field, 100);
        let drainage = derive_drainage(&field, &climate).unwrap();
        assert_eq!(drainage.fill_depth_mm[bowl], 90);
        assert_eq!(
            drainage.fill_depth_mm[bowl],
            drainage.routing_elevation_mm[bowl] - field.elevations_mm[bowl]
        );
    }

    #[test]
    fn accumulation_validator_rejects_corrupt_edges_and_conserves_outlets() {
        let (field, climate, evolution) = fixture_parts(EvolutionPreset::Mature);
        assert_eq!(
            evolution
                .drainage
                .outlet_cells
                .iter()
                .map(|cell| evolution.drainage.accumulation_m3_per_year[*cell])
                .sum::<u64>(),
            climate.runoff_volume_m3_per_year.iter().sum::<u64>()
        );
        let mut corrupt = evolution.drainage.clone();
        corrupt.edges[0].weight_ppm = 1;
        assert!(corrupt.validate_against(&field, &climate).is_err());
    }

    #[test]
    fn stream_power_incision_increases_with_slope_and_discharge() {
        let budget = EvolutionSettings::default().budget();
        let low = stream_power_incision_mm(budget, 12_000, 1_000_000, 50_000, 2_000_000).unwrap();
        let steep =
            stream_power_incision_mm(budget, 12_000, 1_000_000, 400_000, 2_000_000).unwrap();
        let wet = stream_power_incision_mm(budget, 12_000, 25_000_000, 50_000, 2_000_000).unwrap();
        assert!(steep > low);
        assert!(wet > low);
    }

    #[test]
    fn removable_relief_is_computed_in_one_pass_over_drainage_edges() {
        let elevations = [1_000, 700, 400, -100];
        let edges = [
            DrainageEdge {
                source_cell: 0,
                destination_cell: 1,
                weight_ppm: 600_000,
                distance_metres: 1,
            },
            DrainageEdge {
                source_cell: 0,
                destination_cell: 2,
                weight_ppm: 400_000,
                distance_metres: 1,
            },
            DrainageEdge {
                source_cell: 1,
                destination_cell: 2,
                weight_ppm: 1_000_000,
                distance_metres: 1,
            },
        ];

        assert_eq!(
            removable_relief_by_cell(&elevations, 0, &edges),
            vec![300, 300, 400, 0]
        );
    }

    #[test]
    fn coastal_and_mouth_scales_raise_incision_near_the_ocean() {
        assert!(coastal_incision_scale_ppm(0, COASTAL_EROSION_RADIUS_MM, 1_000_000) > 1_000_000);
        assert!(
            coastal_incision_scale_ppm(0, COASTAL_EROSION_RADIUS_MM, 1_000_000)
                > coastal_incision_scale_ppm(0, COASTAL_EROSION_RADIUS_MM, 0)
        );
        assert_eq!(
            coastal_incision_scale_ppm(
                COASTAL_EROSION_RADIUS_MM,
                COASTAL_EROSION_RADIUS_MM,
                1_000_000
            ),
            1_000_000
        );
        assert!(
            mouth_incision_scale_ppm(80_000_000, 10_000_000, COASTAL_EROSION_RADIUS_MM) > 1_000_000
        );
        assert!(
            deposition_mm(
                80_000_000,
                20_000,
                8_000,
                1_000_000,
                COASTAL_EROSION_RADIUS_MM
            ) > deposition_mm(
                80_000_000,
                20_000,
                8_000,
                i64::MAX,
                COASTAL_EROSION_RADIUS_MM
            )
        );
    }

    #[test]
    fn evolution_is_deterministic_and_bounded() {
        let first = fixture(EvolutionPreset::Mature);
        let second = fixture(EvolutionPreset::Mature);
        assert_eq!(first, second);
        assert!(first.metrics.max_step_relief_loss_mm <= MAX_RELIEF_LOSS_PER_STEP_MM);
        assert!(first.metrics.tectonic_range_orientation_ppm >= 600_000);
        assert!(first.elevations_mm.iter().all(|elevation| {
            (-MAX_EVOLUTION_ELEVATION_MM..=MAX_EVOLUTION_ELEVATION_MM).contains(elevation)
        }));
    }

    #[test]
    fn older_budgets_accumulate_more_stream_power_work() {
        let young = fixture(EvolutionPreset::Young);
        let mature = fixture(EvolutionPreset::Mature);
        let old = fixture(EvolutionPreset::Old);
        assert!(young.metrics.erosion_work_m3 <= mature.metrics.erosion_work_m3);
        assert!(mature.metrics.erosion_work_m3 <= old.metrics.erosion_work_m3);
    }

    #[test]
    fn continuous_direction_weights_sum_per_source() {
        let evolution = fixture(EvolutionPreset::Young);
        let mut sums = vec![0u32; evolution.grid.sample_count()];
        for edge in &evolution.drainage.edges {
            sums[edge.source_cell] += edge.weight_ppm;
        }
        for (cell, _) in sums.iter().enumerate() {
            if evolution
                .drainage
                .edges
                .iter()
                .any(|edge| edge.source_cell == cell)
            {
                assert_eq!(sums[cell], 1_000_000);
            }
        }
    }

    fn synthetic_field(
        width: u32,
        height: u32,
        elevations_mm: Vec<i32>,
        sea_level_mm: i32,
    ) -> PhysicalField {
        let grid = Grid::new(width, height, crate::DEFAULT_RADIUS_METRES).unwrap();
        PhysicalField {
            grid,
            seed: 1,
            retry_index: 0,
            target_land_fraction_ppm: 300_000,
            sea_level_mm,
            elevations_mm,
        }
    }

    fn synthetic_climate(field: &PhysicalField, runoff_mm_per_year: u32) -> ClimateField {
        let sample_count = field.grid.sample_count();
        let mut runoff_mm_per_year_field = vec![0u32; sample_count];
        let mut runoff_volume_m3_per_year = vec![0u64; sample_count];
        for cell in 0..sample_count {
            if field.elevations_mm[cell] <= field.sea_level_mm {
                continue;
            }
            runoff_mm_per_year_field[cell] = runoff_mm_per_year;
            let area = field.grid.cell_area(field.grid.row_col(cell).0);
            runoff_volume_m3_per_year[cell] = (area * f64::from(runoff_mm_per_year) / 1_000.0)
                .round()
                .max(0.0) as u64;
        }
        ClimateField {
            grid: field.grid,
            derivation_version: crate::climate::CLIMATE_DERIVATION_VERSION,
            temperature_centi_c: vec![0; sample_count],
            moisture_mm_per_year: vec![0; sample_count],
            precipitation_mm_per_year: runoff_mm_per_year_field.clone(),
            runoff_mm_per_year: runoff_mm_per_year_field,
            runoff_volume_m3_per_year,
            maritime_factor_ppm: vec![0; sample_count],
            metrics: crate::climate::ClimateMetrics {
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
            },
        }
    }

    fn island_plane(width: u32, height: u32, origin_col: u32) -> PhysicalField {
        let grid = Grid::new(width, height, crate::DEFAULT_RADIUS_METRES).unwrap();
        let mut elevations = vec![-2_000; grid.sample_count()];
        for row in 2..height - 2 {
            for col in 2..width - 2 {
                let eastward = (col + width - origin_col) % width;
                elevations[grid.index(row, col)] = 4_000 - eastward as i32 * 80;
            }
        }
        synthetic_field(width, height, elevations, 0)
    }

    fn island_cone(width: u32, height: u32, peak_row: u32, peak_col: u32) -> PhysicalField {
        let grid = Grid::new(width, height, crate::DEFAULT_RADIUS_METRES).unwrap();
        let peak = grid.center_radians(peak_row, peak_col);
        let mut elevations = vec![-2_000; grid.sample_count()];
        for row in 1..height - 1 {
            for col in 0..width {
                let center = grid.center_radians(row, col);
                let distance_km = grid.great_circle_distance(peak, center) / 1_000.0;
                let elevation = 18_000 - (distance_km * 1.2).round() as i32;
                if elevation > 0 {
                    elevations[grid.index(row, col)] = elevation;
                }
            }
        }
        synthetic_field(width, height, elevations, 0)
    }

    fn island_ridge(width: u32, height: u32, ridge_col: u32) -> PhysicalField {
        let grid = Grid::new(width, height, crate::DEFAULT_RADIUS_METRES).unwrap();
        let mut elevations = vec![-2_000; grid.sample_count()];
        for row in 2..height - 2 {
            for col in 2..width - 2 {
                let delta = col.abs_diff(ridge_col).min(width - col.abs_diff(ridge_col)) as i32;
                elevations[grid.index(row, col)] = 6_000 - delta * 400;
            }
        }
        synthetic_field(width, height, elevations, 0)
    }

    fn mean_outgoing_east_ppm(drainage: &DrainageField) -> i64 {
        let mut weighted = 0i128;
        let mut weight = 0i128;
        for edge in &drainage.edges {
            let (_, source_col) = drainage.grid.row_col(edge.source_cell);
            let (_, dest_col) = drainage.grid.row_col(edge.destination_cell);
            let wrapped = i32::from(dest_col as i16 - source_col as i16);
            let width = drainage.grid.width as i32;
            let delta = if wrapped > width / 2 {
                wrapped - width
            } else if wrapped < -width / 2 {
                wrapped + width
            } else {
                wrapped
            };
            weighted += i128::from(delta) * i128::from(edge.weight_ppm);
            weight += i128::from(edge.weight_ppm);
        }
        if weight == 0 {
            0
        } else {
            (weighted * 1_000_000 / weight) as i64
        }
    }

    #[test]
    fn analytic_planes_route_eastward_at_several_longitudes() {
        for origin_col in [2, 5, 8, 11] {
            let field = island_plane(16, 8, origin_col);
            let climate = synthetic_climate(&field, 250);
            let drainage = derive_drainage(&field, &climate).unwrap();
            drainage.validate_against(&field, &climate).unwrap();
            assert!(mean_outgoing_east_ppm(&drainage) > 200_000);
            assert_eq!(
                drainage.metrics.direct_runoff_m3_per_year,
                drainage.metrics.routed_runoff_m3_per_year
            );
        }
    }

    #[test]
    fn analytic_cones_keep_compact_basins_and_radial_outlets() {
        for (row, col) in [(4, 4), (4, 10), (5, 7)] {
            let field = island_cone(16, 8, row, col);
            let climate = synthetic_climate(&field, 250);
            let drainage = derive_drainage(&field, &climate).unwrap();
            drainage.validate_against(&field, &climate).unwrap();
            let peak = field.grid.index(row, col);
            let peak_area = drainage.accumulation_m3_per_year[peak];
            let max_land = drainage
                .accumulation_m3_per_year
                .iter()
                .enumerate()
                .filter(|(cell, _)| field.elevations_mm[*cell] > field.sea_level_mm)
                .map(|(_, volume)| *volume)
                .max()
                .unwrap_or(0);
            assert!(max_land > peak_area);
            let land_cells = field
                .elevations_mm
                .iter()
                .filter(|elevation| **elevation > field.sea_level_mm)
                .count();
            assert!(drainage.metrics.outlet_count as usize > 4);
            assert!(land_cells > 8);
            assert!(drainage.metrics.grid_anisotropy_ppm < 1_000_000);
        }
    }

    #[test]
    fn analytic_ridges_split_flow_away_from_the_divide() {
        for ridge_col in [4, 7, 11] {
            let field = island_ridge(16, 8, ridge_col);
            let climate = synthetic_climate(&field, 250);
            let drainage = derive_drainage(&field, &climate).unwrap();
            drainage.validate_against(&field, &climate).unwrap();
            let mut west = 0u64;
            let mut east = 0u64;
            for edge in &drainage.edges {
                let (_, source_col) = field.grid.row_col(edge.source_cell);
                if source_col == ridge_col {
                    continue;
                }
                let (_, dest_col) = field.grid.row_col(edge.destination_cell);
                if source_col < ridge_col && dest_col <= source_col {
                    west += u64::from(edge.weight_ppm);
                }
                if source_col > ridge_col && dest_col >= source_col {
                    east += u64::from(edge.weight_ppm);
                }
            }
            assert!(west > 0 && east > 0, "ridge {ridge_col} did not split");
        }
    }

    #[test]
    fn production_preview_drainage_converges_toward_the_next_finer_grid() {
        let coarse = fixture(EvolutionPreset::Mature);
        let fine_grid = Grid::new(64, 32, crate::DEFAULT_RADIUS_METRES).unwrap();
        let settings = crate::tectonics::TectonicSettings::default_for(fine_grid);
        let mut progress = NoopProgress;
        let tectonics =
            generate_tectonic_world(fine_grid, settings, 300_000, 831_429, 0, &mut progress)
                .unwrap();
        let sea_level_mm =
            crate::solve_sea_level(&fine_grid, &tectonics.elevations_mm, 300_000).unwrap();
        let field = PhysicalField {
            grid: fine_grid,
            seed: 831_429,
            retry_index: 0,
            target_land_fraction_ppm: 300_000,
            sea_level_mm,
            elevations_mm: tectonics.elevations_mm.clone(),
        };
        let climate = derive_current_climate(
            &field,
            crate::climate::ClimateSettings::default_for(fine_grid),
            field.seed,
            field.retry_index,
            &mut progress,
        )
        .unwrap();
        let fine = evolve_terrain(
            &field,
            &climate,
            &tectonics,
            EvolutionSettings {
                preset: EvolutionPreset::Mature,
            },
            &mut progress,
        )
        .unwrap();
        let density_delta = i64::from(coarse.metrics.drainage_density_ppm)
            .abs_diff(i64::from(fine.metrics.drainage_density_ppm));
        let anisotropy_delta = i64::from(coarse.metrics.grid_anisotropy_ppm)
            .abs_diff(i64::from(fine.metrics.grid_anisotropy_ppm));
        assert!(
            density_delta < 250_000,
            "drainage density diverged: {density_delta}"
        );
        assert!(
            anisotropy_delta < 250_000,
            "grid anisotropy diverged: {anisotropy_delta}"
        );
        assert_eq!(
            fine.drainage.metrics.direct_runoff_m3_per_year,
            fine.drainage.metrics.routed_runoff_m3_per_year
        );
    }
}
