//! Deterministic terrain evolution and depression-resolving drainage.
//!
//! The routing surface in this module is a temporary Priority-Flood surface;
//! the real signed elevation field is never modified while routing. Flow uses
//! a reviewed continuous-direction blend: up to four downhill or flood-order
//! neighbors receive normalized, integer weights derived from physical
//! distance and routing-surface drop. This avoids treating a raster diagonal
//! as a special physical direction while keeping every edge deterministic.

use std::cmp::Reverse;
use std::collections::BinaryHeap;

use super::{
    climate::ClimateField, Grid, PhysicalError, PhysicalErrorCode, PhysicalField, ProgressPhase,
    ProgressSink,
};
use crate::tectonics::{BoundaryKind, TectonicWorld};

pub const EVOLUTION_DERIVATION_VERSION: u16 = 1;
pub const MAX_EVOLUTION_ELEVATION_MM: i32 = 9_000_000;
pub const MAX_RELIEF_LOSS_PER_STEP_MM: i32 = 25_000;
pub const ROUTING_ANISOTROPY_LIMIT_PPM: u32 = 950_000;
const MAX_ROUTING_SPLIT: usize = 4;
const MILLION: u128 = 1_000_000;

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
                stream_power_ppm: 650_000,
                uplift_mm_per_step: 80,
                hillslope_relaxation_ppm: 80_000,
            },
            EvolutionPreset::Mature => EvolutionBudget {
                steps: 4,
                stream_power_ppm: 800_000,
                uplift_mm_per_step: 70,
                hillslope_relaxation_ppm: 55_000,
            },
            EvolutionPreset::Old => EvolutionBudget {
                steps: 8,
                stream_power_ppm: 900_000,
                uplift_mm_per_step: 60,
                hillslope_relaxation_ppm: 110_000,
            },
        }
    }

    pub fn validate(self) -> Result<(), PhysicalError> {
        let budget = self.budget();
        if budget.steps == 0
            || budget.stream_power_ppm > 1_000_000
            || budget.uplift_mm_per_step > MAX_RELIEF_LOSS_PER_STEP_MM
            || budget.hillslope_relaxation_ppm > 500_000
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
    pub stream_power_ppm: u32,
    pub uplift_mm_per_step: i32,
    pub hillslope_relaxation_ppm: u32,
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

fn normalized_edges(
    field: &PhysicalField,
    routing: &[i32],
    order: &[u32],
    cell: usize,
) -> Vec<DrainageEdge> {
    let mut candidates = field
        .grid
        .topology()
        .neighbors(cell)
        .iter()
        .copied()
        .filter_map(|destination| {
            let lower = routing[destination] < routing[cell];
            let plateau = routing[destination] == routing[cell] && order[destination] < order[cell];
            if !lower && !plateau {
                return None;
            }
            let distance = physical_distance_metres(field.grid, cell, destination);
            let drop_mm = i64::from(routing[cell]) - i64::from(routing[destination]);
            let raw_weight = if drop_mm > 0 {
                ((drop_mm as u128).saturating_mul(MILLION) / u128::from(distance)).max(1)
            } else {
                (MILLION / u128::from(distance)).max(1)
            };
            Some((destination, raw_weight, distance))
        })
        .collect::<Vec<_>>();
    candidates.sort_unstable_by(|first, second| {
        second.1.cmp(&first.1).then_with(|| first.0.cmp(&second.0))
    });
    candidates.truncate(MAX_ROUTING_SPLIT);
    if candidates.is_empty() {
        return Vec::new();
    }
    let weight_sum = candidates.iter().map(|candidate| candidate.1).sum::<u128>();
    let mut weights = candidates
        .iter()
        .map(|candidate| ((candidate.1 * MILLION) / weight_sum) as u32)
        .collect::<Vec<_>>();
    let total = weights.iter().map(|weight| u64::from(*weight)).sum::<u64>();
    if total < 1_000_000 {
        weights[0] = weights[0].saturating_add((1_000_000 - total) as u32);
    }
    candidates
        .into_iter()
        .zip(weights)
        .filter(|(_, weight)| *weight > 0)
        .map(
            |((destination_cell, _, distance_metres), weight_ppm)| DrainageEdge {
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
        let cell_edges = normalized_edges(field, &routing, &order, cell);
        if cell_edges.is_empty() {
            return Err(PhysicalError::coded(
                PhysicalErrorCode::HydrologyInvalidSink,
                format!("land cell {cell} has no routed downstream destination"),
            ));
        }
        slope_ppm[cell] = cell_edges
            .iter()
            .map(|edge| {
                let drop = i64::from(routing[cell]) - i64::from(routing[edge.destination_cell]);
                if drop <= 0 {
                    0
                } else {
                    (u128::try_from(drop).unwrap_or(0) * MILLION / u128::from(edge.distance_metres))
                        .min(u128::from(u32::MAX)) as u32
                }
            })
            .max()
            .unwrap_or(0);
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
    let routing_surface_raise_max_mm = field
        .elevations_mm
        .iter()
        .zip(&routing)
        .map(|(actual, routed)| routed.saturating_sub(*actual))
        .max()
        .unwrap_or(0);
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

fn stream_power_incision_mm(
    budget: EvolutionBudget,
    accumulation_m3_per_year: u64,
    slope_ppm: u32,
) -> i32 {
    let discharge_scale =
        ((accumulation_m3_per_year as f64).ln_1p() / 1_000_000_000f64.ln_1p()).clamp(0.0, 1.0);
    let slope_scale = f64::from(slope_ppm) / 1_000_000.0;
    (f64::from(budget.stream_power_ppm) / 1_000_000.0 * 30_000.0 * discharge_scale * slope_scale)
        .round() as i32
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
    let before = before_field.elevations_mm.clone();
    let mut current = before.clone();
    let mut erosion_work_m3 = 0u64;
    let mut uplift_work_m3 = 0u64;
    let mut max_step_relief_loss_mm = 0;
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
        let drainage = derive_drainage(&step_field, climate)?;
        let mut next = current.clone();
        for cell in 0..current.len() {
            if cell % 128 == 0 {
                progress.check_cancelled()?;
            }
            if current[cell] <= before_field.sea_level_mm {
                continue;
            }
            let incision = stream_power_incision_mm(
                budget,
                drainage.accumulation_m3_per_year[cell],
                drainage.slope_ppm[cell],
            );
            let uplift =
                (i64::from(budget.uplift_mm_per_step) * i64::from(signal[cell]) / 1_000_000) as i32;
            let neighbors = topology.neighbors(cell);
            let average_neighbor = neighbors
                .iter()
                .map(|neighbor| f64::from(current[*neighbor]))
                .sum::<f64>()
                / neighbors.len().max(1) as f64;
            let relaxation = ((average_neighbor - f64::from(current[cell]))
                * f64::from(budget.hillslope_relaxation_ppm)
                / 1_000_000.0)
                .round() as i32;
            let unclamped_delta = uplift.saturating_add(relaxation).saturating_sub(incision);
            let delta =
                unclamped_delta.clamp(-MAX_RELIEF_LOSS_PER_STEP_MM, MAX_RELIEF_LOSS_PER_STEP_MM);
            max_step_relief_loss_mm = max_step_relief_loss_mm.max(-delta.min(0));
            let area = row_areas[before_field.grid.row_col(cell).0 as usize];
            if delta < 0 {
                let volume = f64::from(-delta) * area / 1_000.0;
                erosion_work_m3 = erosion_work_m3.saturating_add(volume.round().max(0.0) as u64);
            } else {
                let volume = f64::from(delta) * area / 1_000.0;
                uplift_work_m3 = uplift_work_m3.saturating_add(volume.round().max(0.0) as u64);
            }
            let evolved = i64::from(current[cell]) + i64::from(delta);
            if evolved < i64::from(-MAX_EVOLUTION_ELEVATION_MM)
                || evolved > i64::from(MAX_EVOLUTION_ELEVATION_MM)
            {
                return Err(PhysicalError::coded(
                    PhysicalErrorCode::NumericNonFinite,
                    "terrain evolution exceeded the bounded signed elevation field",
                ));
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
        let low = stream_power_incision_mm(budget, 1_000, 100_000);
        let steep = stream_power_incision_mm(budget, 1_000, 800_000);
        let wet = stream_power_incision_mm(budget, 1_000_000_000, 100_000);
        assert!(steep > low);
        assert!(wet > low);
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
}
