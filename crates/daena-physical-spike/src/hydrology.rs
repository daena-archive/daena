//! Deterministic current-world water and geography derivation.
//!
//! Hydrology is disposable. It consumes the canonical final elevation field,
//! current climate, and the Iteration 4 routing graph, then produces basin,
//! lake, river, watershed, coastline, and bounded raster diagnostics without
//! changing the physical-world-v2 source bytes.

use std::collections::{BTreeMap, VecDeque};

use super::{
    climate::ClimateField,
    evolution::{DrainageEdge, DrainageField},
    Grid, PhysicalError, PhysicalErrorCode, PhysicalField, Segment,
};
use crate::tectonics::CrustType;

pub const HYDROLOGY_DERIVATION_VERSION: u16 = 1;
pub const MAX_HYDROLOGY_FEATURES: usize = 262_144;
pub const WATER_BALANCE_TOLERANCE_PPM: u64 = 5_000;
const MAX_FIXED_POINT_ITERATIONS: u32 = 24;
const MAX_ENDORHEIC_DEPTH_MM: i32 = 250_000;
const MIN_LAKE_DEPTH_MM: i32 = 10;
const LAKE_STORAGE_FRACTION_PPM: u64 = 250_000;
const EVAPORATION_FRACTION_PPM: u64 = 280_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BasinDestination {
    Ocean,
    Basin(usize),
    Endorheic,
    Junction(usize),
}

impl BasinDestination {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Ocean => "ocean",
            Self::Basin(_) => "basin",
            Self::Endorheic => "endorheic",
            Self::Junction(_) => "junction",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BasinStatus {
    Dry,
    Endorheic,
    Active,
    Overflowing,
    Merged,
}

impl BasinStatus {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Dry => "dry",
            Self::Endorheic => "endorheic",
            Self::Active => "active",
            Self::Overflowing => "overflowing",
            Self::Merged => "merged",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Basin {
    pub id: usize,
    pub minimum_cell: usize,
    pub minimum_elevation_mm: i32,
    pub cell_count: u32,
    pub spill_cell: Option<usize>,
    pub spill_elevation_mm: Option<i32>,
    pub volume_to_spill_m3: u64,
    pub parent_basin: Option<usize>,
    pub destination: BasinDestination,
    pub water_level_mm: i32,
    pub water_volume_m3: u64,
    pub inflow_m3_per_year: u64,
    pub direct_precipitation_m3_per_year: u64,
    pub evaporation_m3_per_year: u64,
    pub outflow_m3_per_year: u64,
    pub status: BasinStatus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RiverSegment {
    pub id: u32,
    pub source_cell: usize,
    pub mouth_cell: usize,
    pub strahler_order: u16,
    pub destination: BasinDestination,
    pub spill_outlet: bool,
    pub coordinate_count: u32,
}

type RiverProducts = (Vec<RiverSegment>, Vec<Vec<[i32; 2]>>);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WaterBalanceMetrics {
    pub total_water_m3: u64,
    pub ocean_water_m3: u64,
    pub inland_water_m3: u64,
    pub land_ice_m3: u64,
    pub balance_error_m3: u64,
    pub tolerance_m3: u64,
    pub fixed_point_iterations: u32,
    pub converged: bool,
    pub lake_count: u32,
    pub river_count: u32,
    pub watershed_count: u32,
    pub coastline_segment_count: u32,
    pub land_polygon_count: u32,
    pub ocean_polygon_count: u32,
    pub shelf_cell_count: u32,
    pub bathymetry_contour_count: u32,
    pub island_count: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HydrologyField {
    pub grid: Grid,
    pub derivation_version: u16,
    pub sea_level_mm: i32,
    pub water_level_mm: Vec<i32>,
    pub lake_level_mm: Vec<i32>,
    pub slope_ppm: Vec<u32>,
    pub hillshade_ppm: Vec<u32>,
    pub bathymetry_mm: Vec<i32>,
    pub watershed_id: Vec<u32>,
    pub basin_by_cell: Vec<u32>,
    pub lake_cells: Vec<bool>,
    pub shelf_cells: Vec<bool>,
    pub island_id: Vec<u32>,
    pub basins: Vec<Basin>,
    pub rivers: Vec<RiverSegment>,
    pub river_coordinates: Vec<Vec<[i32; 2]>>,
    pub coastline_segments: Vec<Segment>,
    pub lake_polygons: Vec<Vec<Vec<[i32; 2]>>>,
    pub watershed_polygons: Vec<Vec<Vec<[i32; 2]>>>,
    pub land_polygons: Vec<Vec<Vec<[i32; 2]>>>,
    pub ocean_polygons: Vec<Vec<Vec<[i32; 2]>>>,
    pub shelf_polygons: Vec<Vec<Vec<[i32; 2]>>>,
    pub island_polygons: Vec<Vec<Vec<[i32; 2]>>>,
    pub bathymetry_contours: Vec<Segment>,
    pub metrics: WaterBalanceMetrics,
}

impl HydrologyField {
    pub fn validate_against(
        &self,
        field: &PhysicalField,
        climate: &ClimateField,
        drainage: &DrainageField,
        reference_water_inventory_m3: u64,
    ) -> Result<(), PhysicalError> {
        let count = field.grid.sample_count();
        if self.derivation_version != HYDROLOGY_DERIVATION_VERSION
            || self.grid != field.grid
            || climate.grid != field.grid
            || drainage.grid != field.grid
            || self.water_level_mm.len() != count
            || self.lake_level_mm.len() != count
            || self.slope_ppm.len() != count
            || self.hillshade_ppm.len() != count
            || self.bathymetry_mm.len() != count
            || self.watershed_id.len() != count
            || self.basin_by_cell.len() != count
            || self.lake_cells.len() != count
            || self.shelf_cells.len() != count
            || self.island_id.len() != count
        {
            return Err(PhysicalError::coded(
                PhysicalErrorCode::GeometryInvalid,
                "hydrology field shape or derivation version is invalid",
            ));
        }
        if !self.metrics.converged {
            return Err(PhysicalError::coded(
                PhysicalErrorCode::WaterNonConvergent,
                "hydrology water balance did not converge",
            ));
        }
        let tolerance = tolerance_m3(reference_water_inventory_m3);
        if self.metrics.total_water_m3 != reference_water_inventory_m3
            || self.metrics.balance_error_m3 > tolerance
            || self.metrics.tolerance_m3 != tolerance
        {
            return Err(PhysicalError::coded(
                PhysicalErrorCode::WaterNonConvergent,
                "hydrology water balance exceeds its declared tolerance",
            ));
        }
        if self.watershed_polygons.len() != self.metrics.watershed_count as usize
            || self.rivers.len() != self.river_coordinates.len()
            || self.coastline_segments.len() > MAX_HYDROLOGY_FEATURES
        {
            return Err(PhysicalError::coded(
                PhysicalErrorCode::GeometryInvalid,
                "hydrology geometry counts do not match its products",
            ));
        }
        let mut basin_counts = vec![0u32; self.basins.len()];
        for (cell, basin) in self.basin_by_cell.iter().copied().enumerate() {
            if basin == u32::MAX {
                if self.lake_cells[cell] {
                    return Err(PhysicalError::coded(
                        PhysicalErrorCode::HydrologyInvalidSink,
                        "lake cell is not assigned to a basin",
                    ));
                }
                continue;
            }
            let Some(count) = basin_counts.get_mut(basin as usize) else {
                return Err(PhysicalError::coded(
                    PhysicalErrorCode::HydrologyInvalidSink,
                    "hydrology cell points to a missing basin",
                ));
            };
            *count = count.saturating_add(1);
            if self.lake_cells[cell]
                && (self.water_level_mm[cell] <= self.sea_level_mm
                    || self.lake_level_mm[cell] != self.water_level_mm[cell]
                    || field.elevations_mm[cell] >= self.water_level_mm[cell])
            {
                return Err(PhysicalError::coded(
                    PhysicalErrorCode::GeometryInvalid,
                    "lake cell has an invalid water level",
                ));
            }
        }
        for basin in &self.basins {
            if basin.id >= self.basins.len()
                || basin_counts[basin.id] != basin.cell_count
                || basin.minimum_cell >= count
                || self.basin_by_cell[basin.minimum_cell] != basin.id as u32
                || basin.minimum_elevation_mm != field.elevations_mm[basin.minimum_cell]
            {
                return Err(PhysicalError::coded(
                    PhysicalErrorCode::HydrologyInvalidSink,
                    "basin hierarchy has an invalid minimum or membership count",
                ));
            }
            if basin.water_volume_m3 > 0
                && basin.water_level_mm > self.sea_level_mm
                && basin
                    .spill_elevation_mm
                    .is_some_and(|spill| basin.water_level_mm > spill)
            {
                return Err(PhysicalError::coded(
                    PhysicalErrorCode::HydrologyInvalidSink,
                    "lake water level exceeds its spill elevation",
                ));
            }
            if let Some(parent) = basin.parent_basin {
                if parent >= self.basins.len() || parent == basin.id {
                    return Err(PhysicalError::coded(
                        PhysicalErrorCode::HydrologyCycle,
                        "basin hierarchy contains an invalid parent",
                    ));
                }
            }
        }
        for basin in &self.basins {
            let mut cursor = Some(basin.id);
            for _ in 0..=self.basins.len() {
                cursor = cursor.and_then(|id| self.basins[id].parent_basin);
                if cursor.is_none() {
                    break;
                }
            }
            if cursor.is_some() {
                return Err(PhysicalError::coded(
                    PhysicalErrorCode::HydrologyCycle,
                    "basin hierarchy contains a parent cycle",
                ));
            }
        }
        for (cell, level) in self.water_level_mm.iter().enumerate() {
            if !(-9_000_000..=9_000_000).contains(level)
                || self.lake_cells[cell]
                    && (*level <= field.sea_level_mm || *level != self.lake_level_mm[cell])
            {
                return Err(PhysicalError::coded(
                    PhysicalErrorCode::NumericNonFinite,
                    "hydrology water level is outside its bounded field",
                ));
            }
        }
        for (index, (river, coordinates)) in
            self.rivers.iter().zip(&self.river_coordinates).enumerate()
        {
            if river.id != index as u32
                || river.source_cell >= count
                || river.mouth_cell >= count
                || river.coordinate_count != coordinates.len() as u32
                || coordinates.len() < 2
                || coordinates.iter().any(|coordinate| {
                    !(-180_000_000..=180_000_000).contains(&coordinate[0])
                        || !(-90_000_000..=90_000_000).contains(&coordinate[1])
                })
            {
                return Err(PhysicalError::coded(
                    PhysicalErrorCode::GeometryInvalid,
                    "river geometry is disconnected or out of range",
                ));
            }
            if river.spill_outlet {
                let Some(basin_id) = self.basin_by_cell.get(river.source_cell).copied() else {
                    return Err(PhysicalError::coded(
                        PhysicalErrorCode::HydrologyInvalidSink,
                        "spill outlet source is outside a basin",
                    ));
                };
                let Some(basin) = self.basins.get(basin_id as usize) else {
                    return Err(PhysicalError::coded(
                        PhysicalErrorCode::HydrologyInvalidSink,
                        "spill outlet source points to a missing basin",
                    ));
                };
                if basin.spill_cell != Some(river.source_cell)
                    || basin.outflow_m3_per_year == 0
                    || basin.destination != river.destination
                {
                    return Err(PhysicalError::coded(
                        PhysicalErrorCode::HydrologyInvalidSink,
                        "spill outlet does not match its overflowing basin",
                    ));
                }
            }
        }
        for basin in &self.basins {
            let outlets = self
                .rivers
                .iter()
                .filter(|river| {
                    river.spill_outlet
                        && river.source_cell == basin.spill_cell.unwrap_or(usize::MAX)
                })
                .count();
            if basin.outflow_m3_per_year > 0 && outlets != 1 {
                return Err(PhysicalError::coded(
                    PhysicalErrorCode::HydrologyInvalidSink,
                    "overflowing basin does not have exactly one spill outlet",
                ));
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy)]
struct Spill {
    elevation_mm: i32,
    cell: usize,
    destination: BasinDestination,
}

fn checked_add(first: u64, second: u64, message: &'static str) -> Result<u64, PhysicalError> {
    first
        .checked_add(second)
        .ok_or_else(|| PhysicalError::coded(PhysicalErrorCode::NumericNonFinite, message))
}

fn volume_for_depth(grid: Grid, cell: usize, depth_mm: i32) -> u64 {
    if depth_mm <= 0 {
        return 0;
    }
    (grid.cell_area(grid.row_col(cell).0) * f64::from(depth_mm) / 1_000.0)
        .round()
        .max(0.0) as u64
}

fn volume_for_depths(grid: Grid, field: &PhysicalField, cells: &[usize], level_mm: i32) -> u64 {
    cells
        .iter()
        .map(|cell| {
            volume_for_depth(
                grid,
                *cell,
                level_mm.saturating_sub(field.elevations_mm[*cell]),
            )
        })
        .fold(0u64, u64::saturating_add)
}

pub fn ocean_volume(field: &PhysicalField, sea_level_mm: i32) -> u64 {
    field
        .elevations_mm
        .iter()
        .enumerate()
        .filter(|(_, elevation)| **elevation <= sea_level_mm)
        .map(|(cell, elevation)| {
            volume_for_depth(field.grid, cell, sea_level_mm.saturating_sub(*elevation))
        })
        .fold(0u64, u64::saturating_add)
}

pub fn ocean_area(field: &PhysicalField, sea_level_mm: i32) -> f64 {
    field
        .elevations_mm
        .iter()
        .enumerate()
        .filter(|(_, elevation)| **elevation <= sea_level_mm)
        .map(|(cell, _)| field.grid.cell_area(field.grid.row_col(cell).0))
        .sum()
}

fn tolerance_m3(total: u64) -> u64 {
    ((u128::from(total) * u128::from(WATER_BALANCE_TOLERANCE_PPM) / 1_000_000).max(1)) as u64
}

/// Choose the bounded sea level whose ocean volume is nearest to an inventory.
///
/// This helper is intentionally independent of inland storage. Historical
/// derivation uses it only to select the immutable terrain's initial basin
/// topology before the existing inland/ocean fixed-point solve runs.
pub fn solve_ocean_level_for_inventory(
    field: &PhysicalField,
    inventory_m3: u64,
) -> Result<i32, PhysicalError> {
    field.validate().map_err(PhysicalError::InvalidSource)?;
    let mut candidates = field.elevations_mm.clone();
    let minimum = field.elevations_mm.iter().copied().min().ok_or_else(|| {
        PhysicalError::coded(
            PhysicalErrorCode::GeometryInvalid,
            "ocean-level solve requires at least one elevation sample",
        )
    })?;
    let maximum = field.elevations_mm.iter().copied().max().ok_or_else(|| {
        PhysicalError::coded(
            PhysicalErrorCode::GeometryInvalid,
            "ocean-level solve requires at least one elevation sample",
        )
    })?;
    candidates.push(minimum.saturating_sub(1));
    candidates.push(maximum.saturating_add(1));
    candidates.sort_unstable();
    candidates.dedup();
    candidates
        .into_iter()
        .min_by_key(|level| (ocean_volume(field, *level).abs_diff(inventory_m3), *level))
        .ok_or_else(|| {
            PhysicalError::coded(
                PhysicalErrorCode::GeometryInvalid,
                "ocean-level solve produced no candidates",
            )
        })
}

fn downhill_destination(field: &PhysicalField, cell: usize) -> Option<usize> {
    if field.elevations_mm[cell] <= field.sea_level_mm {
        return None;
    }
    let elevation = field.elevations_mm[cell];
    field
        .grid
        .neighbors(cell)
        .into_iter()
        .filter(|neighbor| {
            field.elevations_mm[*neighbor] < elevation
                || (field.elevations_mm[*neighbor] == elevation && *neighbor < cell)
        })
        .min_by_key(|neighbor| (field.elevations_mm[*neighbor], *neighbor))
}

fn basin_assignments(field: &PhysicalField) -> (Vec<u32>, Vec<usize>) {
    let count = field.grid.sample_count();
    let mut basin_by_cell = vec![u32::MAX; count];
    let mut sink_by_cell = vec![usize::MAX; count];
    for (start, sink_slot) in sink_by_cell.iter_mut().enumerate() {
        if field.elevations_mm[start] <= field.sea_level_mm {
            continue;
        }
        let mut cell = start;
        let mut steps = 0usize;
        while let Some(next) = downhill_destination(field, cell) {
            if field.elevations_mm[next] <= field.sea_level_mm {
                cell = usize::MAX;
                break;
            }
            cell = next;
            steps += 1;
            if steps > count {
                cell = usize::MAX;
                break;
            }
        }
        if cell != usize::MAX && field.elevations_mm[cell] > field.sea_level_mm {
            *sink_slot = cell;
        }
    }
    let mut sinks = sink_by_cell
        .iter()
        .copied()
        .filter(|sink| *sink != usize::MAX)
        .collect::<Vec<_>>();
    sinks.sort_unstable();
    sinks.dedup();
    let ids = sinks
        .iter()
        .enumerate()
        .map(|(id, sink)| (*sink, id as u32))
        .collect::<BTreeMap<_, _>>();
    for cell in 0..count {
        if sink_by_cell[cell] != usize::MAX {
            basin_by_cell[cell] = ids[&sink_by_cell[cell]];
        }
    }
    (basin_by_cell, sinks)
}

fn basin_cells(basin_by_cell: &[u32], basin_count: usize) -> Vec<Vec<usize>> {
    let mut cells = vec![Vec::new(); basin_count];
    for (cell, basin) in basin_by_cell.iter().copied().enumerate() {
        if basin != u32::MAX {
            cells[basin as usize].push(cell);
        }
    }
    cells
}

fn find_spill(
    field: &PhysicalField,
    basin_by_cell: &[u32],
    basin: usize,
    cells: &[usize],
) -> Option<Spill> {
    cells
        .iter()
        .flat_map(|cell| {
            field
                .grid
                .neighbors(*cell)
                .into_iter()
                .map(move |neighbor| (*cell, neighbor))
        })
        .filter(|(_, neighbor)| basin_by_cell[*neighbor] != basin as u32)
        .map(|(cell, neighbor)| {
            let destination = if field.elevations_mm[neighbor] <= field.sea_level_mm {
                BasinDestination::Ocean
            } else if basin_by_cell[neighbor] != u32::MAX {
                BasinDestination::Basin(basin_by_cell[neighbor] as usize)
            } else {
                BasinDestination::Endorheic
            };
            Spill {
                elevation_mm: field.elevations_mm[cell].max(field.elevations_mm[neighbor]),
                cell,
                destination,
            }
        })
        .min_by_key(|spill| (spill.elevation_mm, spill.cell, spill.destination.label()))
}

fn precipitation_volume(grid: Grid, cell: usize, millimetres: u32) -> u64 {
    (grid.cell_area(grid.row_col(cell).0) * f64::from(millimetres) / 1_000.0)
        .round()
        .max(0.0) as u64
}

fn derive_basins(
    field: &PhysicalField,
    climate: &ClimateField,
) -> Result<(Vec<Basin>, Vec<u32>), PhysicalError> {
    let (basin_by_cell, sinks) = basin_assignments(field);
    let cells_by_basin = basin_cells(&basin_by_cell, sinks.len());
    let mut basins = Vec::with_capacity(sinks.len());
    for (id, minimum_cell) in sinks.iter().copied().enumerate() {
        let cells = &cells_by_basin[id];
        let spill = find_spill(field, &basin_by_cell, id, cells);
        let (spill_cell, spill_elevation_mm, volume_to_spill_m3, mut destination) = match spill {
            Some(spill) => (
                Some(spill.cell),
                Some(spill.elevation_mm),
                volume_for_depths(field.grid, field, cells, spill.elevation_mm),
                spill.destination,
            ),
            None => (None, None, 0, BasinDestination::Endorheic),
        };
        if let BasinDestination::Basin(parent) = destination {
            if parent == id
                || field.elevations_mm[sinks[parent]] >= field.elevations_mm[minimum_cell]
            {
                destination = BasinDestination::Endorheic;
            }
        }
        let mut direct_precipitation = 0u64;
        let mut inflow = 0u64;
        let mut evaporation = 0u64;
        for cell in cells {
            let precipitation =
                precipitation_volume(field.grid, *cell, climate.precipitation_mm_per_year[*cell]);
            direct_precipitation = checked_add(
                direct_precipitation,
                precipitation,
                "basin precipitation overflowed its bounded range",
            )?;
            inflow = checked_add(
                inflow,
                climate.runoff_volume_m3_per_year[*cell],
                "basin inflow overflowed its bounded range",
            )?;
            evaporation = checked_add(
                evaporation,
                precipitation.saturating_mul(EVAPORATION_FRACTION_PPM) / 1_000_000,
                "basin evaporation overflowed its bounded range",
            )?;
        }
        basins.push(Basin {
            id,
            minimum_cell,
            minimum_elevation_mm: field.elevations_mm[minimum_cell],
            cell_count: cells.len() as u32,
            spill_cell,
            spill_elevation_mm,
            volume_to_spill_m3,
            parent_basin: match destination {
                BasinDestination::Basin(parent) => Some(parent),
                _ => None,
            },
            destination,
            water_level_mm: field.elevations_mm[minimum_cell],
            water_volume_m3: 0,
            inflow_m3_per_year: inflow,
            direct_precipitation_m3_per_year: direct_precipitation,
            evaporation_m3_per_year: evaporation,
            outflow_m3_per_year: 0,
            status: if matches!(destination, BasinDestination::Endorheic) {
                BasinStatus::Endorheic
            } else {
                BasinStatus::Dry
            },
        });
    }
    Ok((basins, basin_by_cell))
}

fn solve_basin_water(basins: &mut [Basin], field: &PhysicalField) {
    let mut order = (0..basins.len()).collect::<Vec<_>>();
    order.sort_unstable_by_key(|id| (basins[*id].spill_elevation_mm.unwrap_or(i32::MAX), *id));
    let mut carried_inflow = vec![0u64; basins.len()];
    for id in order {
        let basin = &mut basins[id];
        basin.inflow_m3_per_year = basin.inflow_m3_per_year.saturating_add(carried_inflow[id]);
        let net = basin
            .inflow_m3_per_year
            .saturating_add(basin.direct_precipitation_m3_per_year)
            .saturating_sub(basin.evaporation_m3_per_year);
        let desired = net.saturating_mul(LAKE_STORAGE_FRACTION_PPM) / 1_000_000;
        let endorheic = matches!(basin.destination, BasinDestination::Endorheic);
        let capacity = if endorheic {
            let cells = basin.cell_count.max(1) as u64;
            let area = field.grid.total_area() / field.grid.sample_count() as f64 * cells as f64;
            (area * f64::from(MAX_ENDORHEIC_DEPTH_MM) / 1_000.0).round() as u64
        } else {
            basin.volume_to_spill_m3
        };
        basin.water_volume_m3 = desired.min(capacity);
        basin.outflow_m3_per_year = desired.saturating_sub(basin.water_volume_m3);
        if basin.water_volume_m3 > 0 {
            let spill = basin.spill_elevation_mm.unwrap_or(
                basin
                    .minimum_elevation_mm
                    .saturating_add(MAX_ENDORHEIC_DEPTH_MM),
            );
            let area = field.grid.total_area() / field.grid.sample_count() as f64
                * basin.cell_count.max(1) as f64;
            let depth = (basin.water_volume_m3 as f64 * 1_000.0 / area).round() as i32;
            basin.water_level_mm = basin.minimum_elevation_mm.saturating_add(depth).min(spill);
            basin.status = if endorheic {
                BasinStatus::Endorheic
            } else if basin.outflow_m3_per_year > 0 {
                BasinStatus::Overflowing
            } else {
                BasinStatus::Active
            };
        }
        if basin.outflow_m3_per_year > 0 {
            if let BasinDestination::Basin(parent) = basin.destination {
                if parent < carried_inflow.len() {
                    carried_inflow[parent] =
                        carried_inflow[parent].saturating_add(basin.outflow_m3_per_year);
                    basin.status = BasinStatus::Merged;
                }
            }
        }
    }
}

fn hillshade(field: &PhysicalField, cell: usize) -> u32 {
    let elevation = field.elevations_mm[cell];
    let (row, _) = field.grid.row_col(cell);
    let neighbors = field.grid.neighbors(cell);
    let mean = if neighbors.is_empty() {
        elevation
    } else {
        neighbors
            .iter()
            .map(|neighbor| i64::from(field.elevations_mm[*neighbor]))
            .sum::<i64>()
            .saturating_div(neighbors.len() as i64) as i32
    };
    let gradient = i64::from(elevation)
        .saturating_sub(i64::from(mean))
        .unsigned_abs();
    let latitude_light = (f64::from(row) / f64::from(field.grid.height.max(1))
        * std::f64::consts::PI)
        .sin()
        .abs();
    (500_000i64
        .saturating_add((gradient.min(5_000_000) * 400_000 / 5_000_000) as i64)
        .saturating_add((latitude_light * 100_000.0).round() as i64))
    .clamp(0, 1_000_000) as u32
}

fn longitude_micro(grid: Grid, column: u32) -> i32 {
    (-180_000_000i64 + 360_000_000i64 * i64::from(column) / i64::from(grid.width)) as i32
}

fn latitude_micro(grid: Grid, row: u32) -> i32 {
    (-90_000_000i64 + 180_000_000i64 * i64::from(row) / i64::from(grid.height)) as i32
}

fn cell_ring(grid: Grid, cell: usize) -> Vec<[i32; 2]> {
    let (row, col) = grid.row_col(cell);
    let west = longitude_micro(grid, col);
    let east = longitude_micro(grid, col + 1);
    let south = latitude_micro(grid, row);
    let north = latitude_micro(grid, row + 1);
    vec![
        [west, south],
        [east, south],
        [east, north],
        [west, north],
        [west, south],
    ]
}

fn water_boundary_segments(field: &PhysicalField, water: &[bool]) -> Vec<Segment> {
    let mut segments = Vec::new();
    for cell in 0..field.grid.sample_count() {
        if !water[cell] {
            continue;
        }
        let (row, col) = field.grid.row_col(cell);
        let west = longitude_micro(field.grid, col);
        let east = longitude_micro(field.grid, col + 1);
        let south = latitude_micro(field.grid, row);
        let north = latitude_micro(field.grid, row + 1);
        let edges = [
            [[west, south], [west, north]],
            [[east, north], [east, south]],
            [[west, south], [east, south]],
            [[east, north], [west, north]],
        ];
        let neighbors = [
            field
                .grid
                .index(row, (col + field.grid.width - 1) % field.grid.width),
            field.grid.index(row, (col + 1) % field.grid.width),
            if row == 0 {
                cell
            } else {
                field.grid.index(row - 1, col)
            },
            if row + 1 == field.grid.height {
                cell
            } else {
                field.grid.index(row + 1, col)
            },
        ];
        for (edge, neighbor) in edges.into_iter().zip(neighbors) {
            if neighbor == cell || !water[neighbor] {
                segments.push(Segment {
                    first: edge[0],
                    second: edge[1],
                });
            }
        }
    }
    segments
}

fn primary_edges(field: &PhysicalField, drainage: &DrainageField) -> Vec<Option<DrainageEdge>> {
    let mut primary: Vec<Option<DrainageEdge>> = vec![None; drainage.grid.sample_count()];
    for edge in drainage.edges.iter().copied() {
        if edge.destination_cell >= field.grid.sample_count()
            || field.elevations_mm[edge.destination_cell] > field.elevations_mm[edge.source_cell]
        {
            continue;
        }
        let slot = &mut primary[edge.source_cell];
        let should_replace = match *slot {
            None => true,
            Some(current) => {
                edge.weight_ppm > current.weight_ppm
                    || (edge.weight_ppm == current.weight_ppm
                        && edge.destination_cell < current.destination_cell)
            }
        };
        if should_replace {
            *slot = Some(edge);
        }
    }
    primary
}

fn coordinate_for_cell(grid: Grid, cell: usize) -> [i32; 2] {
    let (row, col) = grid.row_col(cell);
    [
        (-180_000_000i64 + 360_000_000i64 * (i64::from(col) * 2 + 1) / i64::from(grid.width * 2))
            as i32,
        (-90_000_000i64 + 180_000_000i64 * (i64::from(row) * 2 + 1) / i64::from(grid.height * 2))
            as i32,
    ]
}

fn destination_for_cell(
    field: &PhysicalField,
    basins: &[Basin],
    basin_by_cell: &[u32],
    cell: usize,
) -> BasinDestination {
    if field.elevations_mm[cell] <= field.sea_level_mm {
        return BasinDestination::Ocean;
    }
    basin_by_cell
        .get(cell)
        .copied()
        .filter(|basin| *basin != u32::MAX)
        .and_then(|basin| basins.get(basin as usize))
        .and_then(|basin| basin.parent_basin.map(BasinDestination::Basin))
        .unwrap_or(BasinDestination::Endorheic)
}

fn derive_rivers(
    field: &PhysicalField,
    drainage: &DrainageField,
    basins: &[Basin],
    basin_by_cell: &[u32],
    lake_cells: &[bool],
) -> Result<RiverProducts, PhysicalError> {
    let primary = primary_edges(field, drainage);
    let count = field.grid.sample_count();
    let total = drainage.metrics.direct_runoff_m3_per_year.max(1);
    let threshold = (total / 2_000).max(1);
    let active = (0..count)
        .map(|cell| {
            field.elevations_mm[cell] > field.sea_level_mm
                && !lake_cells[cell]
                && drainage.accumulation_m3_per_year[cell] >= threshold
        })
        .collect::<Vec<_>>();
    let mut incoming = vec![0u32; count];
    for cell in 0..count {
        if active[cell]
            && primary[cell]
                .map(|edge| edge.destination_cell < count && active[edge.destination_cell])
                .unwrap_or(false)
        {
            incoming[primary[cell].unwrap().destination_cell] += 1;
        }
    }
    let mut order_cache = vec![0u16; count];
    fn stream_order(
        cell: usize,
        active: &[bool],
        primary: &[Option<DrainageEdge>],
        cache: &mut [u16],
    ) -> u16 {
        if cache[cell] > 0 {
            return cache[cell];
        }
        let parents = primary
            .iter()
            .enumerate()
            .filter(|(parent, edge)| {
                active[*parent]
                    && edge
                        .map(|edge| edge.destination_cell == cell && active[cell])
                        .unwrap_or(false)
            })
            .map(|(parent, _)| stream_order(parent, active, primary, cache))
            .collect::<Vec<_>>();
        let result = if parents.is_empty() {
            1
        } else {
            let max = parents.iter().copied().max().unwrap_or(1);
            if parents.iter().filter(|order| **order == max).count() > 1 {
                max.saturating_add(1)
            } else {
                max
            }
        };
        cache[cell] = result;
        result
    }
    let mut segments = Vec::new();
    let mut coordinates = Vec::new();
    for start in 0..count {
        if !active[start] || incoming[start] == 1 {
            continue;
        }
        let mut path = Vec::new();
        let mut cell = start;
        let mut steps = 0usize;
        let mut ended_at_junction = false;
        loop {
            path.push(coordinate_for_cell(field.grid, cell));
            steps += 1;
            let Some(edge) = primary[cell] else {
                if field.elevations_mm[cell] > field.sea_level_mm && basin_by_cell[cell] == u32::MAX
                {
                    if let Some(outlet) = field
                        .grid
                        .neighbors(cell)
                        .into_iter()
                        .filter(|neighbor| field.elevations_mm[*neighbor] <= field.sea_level_mm)
                        .min()
                    {
                        path.push(coordinate_for_cell(field.grid, outlet));
                        cell = outlet;
                    }
                }
                break;
            };
            let next = edge.destination_cell;
            if next >= count {
                return Err(PhysicalError::coded(
                    PhysicalErrorCode::HydrologyInvalidSink,
                    "river edge points outside the physical grid",
                ));
            }
            if field.elevations_mm[next] > field.elevations_mm[cell] {
                return Err(PhysicalError::coded(
                    PhysicalErrorCode::HydrologyCycle,
                    "river edge moves uphill on the final terrain",
                ));
            }
            if field.elevations_mm[next] <= field.sea_level_mm || lake_cells[next] {
                path.push(coordinate_for_cell(field.grid, next));
                cell = next;
                break;
            }
            if active[next] && incoming[next] > 1 {
                path.push(coordinate_for_cell(field.grid, next));
                cell = next;
                ended_at_junction = true;
                break;
            }
            if steps > count {
                return Err(PhysicalError::coded(
                    PhysicalErrorCode::HydrologyCycle,
                    "river channel contains a flow cycle",
                ));
            }
            cell = next;
        }
        if path.len() < 2 {
            continue;
        }
        let mouth = cell.min(count - 1);
        let destination = if ended_at_junction {
            BasinDestination::Junction(mouth)
        } else if lake_cells[mouth] {
            if basin_by_cell[mouth] == u32::MAX {
                return Err(PhysicalError::coded(
                    PhysicalErrorCode::HydrologyInvalidSink,
                    "river enters a lake without a basin",
                ));
            }
            BasinDestination::Basin(basin_by_cell[mouth] as usize)
        } else if field.elevations_mm[mouth] <= field.sea_level_mm {
            BasinDestination::Ocean
        } else {
            destination_for_cell(field, basins, basin_by_cell, mouth)
        };
        if matches!(destination, BasinDestination::Endorheic) && basin_by_cell[mouth] == u32::MAX {
            return Err(PhysicalError::coded(
                PhysicalErrorCode::HydrologyInvalidSink,
                "river terminates without an ocean or valid endorheic basin",
            ));
        }
        let simplified = simplify_path(path);
        let id = segments.len() as u32;
        segments.push(RiverSegment {
            id,
            source_cell: start,
            mouth_cell: mouth,
            strahler_order: stream_order(start, &active, &primary, &mut order_cache),
            destination,
            spill_outlet: false,
            coordinate_count: simplified.len() as u32,
        });
        coordinates.push(simplified);
    }
    for basin in basins {
        if basin.outflow_m3_per_year == 0 {
            continue;
        }
        let destination = basin.destination;
        if matches!(
            destination,
            BasinDestination::Endorheic | BasinDestination::Junction(_)
        ) {
            return Err(PhysicalError::coded(
                PhysicalErrorCode::HydrologyInvalidSink,
                "overflowing basin has no valid spill destination",
            ));
        }
        let Some(spill_cell) = basin.spill_cell else {
            return Err(PhysicalError::coded(
                PhysicalErrorCode::HydrologyInvalidSink,
                "overflowing basin has no spill point",
            ));
        };
        if let Some(existing) = segments
            .iter_mut()
            .find(|river| river.source_cell == spill_cell)
        {
            if existing.destination == destination {
                existing.spill_outlet = true;
                continue;
            }
            return Err(PhysicalError::coded(
                PhysicalErrorCode::HydrologyInvalidSink,
                "overflowing basin spill point would create a river bifurcation",
            ));
        }
        let Some(outlet_cell) = field
            .grid
            .neighbors(spill_cell)
            .into_iter()
            .filter(|neighbor| match destination {
                BasinDestination::Ocean => field.elevations_mm[*neighbor] <= field.sea_level_mm,
                BasinDestination::Basin(parent) => basin_by_cell[*neighbor] == parent as u32,
                BasinDestination::Endorheic | BasinDestination::Junction(_) => false,
            })
            .min_by_key(|neighbor| (field.elevations_mm[*neighbor], *neighbor))
        else {
            return Err(PhysicalError::coded(
                PhysicalErrorCode::HydrologyInvalidSink,
                "overflowing basin spill point has no adjacent destination",
            ));
        };
        let path = vec![
            coordinate_for_cell(field.grid, spill_cell),
            coordinate_for_cell(field.grid, outlet_cell),
        ];
        let id = segments.len() as u32;
        segments.push(RiverSegment {
            id,
            source_cell: spill_cell,
            mouth_cell: outlet_cell,
            strahler_order: 1,
            destination,
            spill_outlet: true,
            coordinate_count: path.len() as u32,
        });
        coordinates.push(path);
    }
    Ok((segments, coordinates))
}

fn simplify_path(mut path: Vec<[i32; 2]>) -> Vec<[i32; 2]> {
    if path.len() <= 2 {
        return path;
    }
    let mut simplified = vec![path[0]];
    for index in 1..path.len() - 1 {
        let previous = *simplified.last().unwrap_or(&path[index - 1]);
        let current = path[index];
        let next = path[index + 1];
        let first = [
            i64::from(current[0]) - i64::from(previous[0]),
            i64::from(current[1]) - i64::from(previous[1]),
        ];
        let second = [
            i64::from(next[0]) - i64::from(current[0]),
            i64::from(next[1]) - i64::from(current[1]),
        ];
        if first[0] * second[1] != first[1] * second[0] {
            simplified.push(current);
        }
    }
    simplified.push(*path.last().unwrap_or(&path[0]));
    path.clear();
    simplified
}

fn polygon_for_cells(grid: Grid, cells: &[usize]) -> Vec<Vec<[i32; 2]>> {
    cells.iter().map(|cell| cell_ring(grid, *cell)).collect()
}

fn polygon_for_mask(grid: Grid, mask: &[bool]) -> Vec<Vec<Vec<[i32; 2]>>> {
    mask.iter()
        .enumerate()
        .filter(|(_, included)| **included)
        .map(|(cell, _)| vec![cell_ring(grid, cell)])
        .collect()
}

fn island_groups(grid: Grid, land_mask: &[bool]) -> (Vec<u32>, Vec<Vec<usize>>) {
    let mut island_id = vec![u32::MAX; grid.sample_count()];
    let mut groups = Vec::new();
    for start in 0..grid.sample_count() {
        if !land_mask[start] || island_id[start] != u32::MAX {
            continue;
        }
        let id = groups.len() as u32;
        let mut queue = VecDeque::from([start]);
        island_id[start] = id;
        let mut cells = Vec::new();
        while let Some(cell) = queue.pop_front() {
            cells.push(cell);
            for neighbor in grid.neighbors(cell) {
                if land_mask[neighbor] && island_id[neighbor] == u32::MAX {
                    island_id[neighbor] = id;
                    queue.push_back(neighbor);
                }
            }
        }
        groups.push(cells);
    }
    (island_id, groups)
}

fn bathymetry_contour_segments(
    field: &PhysicalField,
    ocean_mask: &[bool],
    sea_level_mm: i32,
) -> Vec<Segment> {
    let mut contours = Vec::new();
    for threshold in [100_000, 250_000, 500_000] {
        for cell in 0..field.grid.sample_count() {
            if !ocean_mask[cell]
                || sea_level_mm.saturating_sub(field.elevations_mm[cell]) < threshold
            {
                continue;
            }
            let (row, col) = field.grid.row_col(cell);
            let west = longitude_micro(field.grid, col);
            let east = longitude_micro(field.grid, col + 1);
            let south = latitude_micro(field.grid, row);
            let north = latitude_micro(field.grid, row + 1);
            let edges = [
                [[west, south], [west, north]],
                [[east, north], [east, south]],
                [[west, south], [east, south]],
                [[east, north], [west, north]],
            ];
            let neighbors = [
                field
                    .grid
                    .index(row, (col + field.grid.width - 1) % field.grid.width),
                field.grid.index(row, (col + 1) % field.grid.width),
                if row == 0 {
                    cell
                } else {
                    field.grid.index(row - 1, col)
                },
                if row + 1 == field.grid.height {
                    cell
                } else {
                    field.grid.index(row + 1, col)
                },
            ];
            for (edge, neighbor) in edges.into_iter().zip(neighbors) {
                if neighbor == cell
                    || !ocean_mask[neighbor]
                    || sea_level_mm.saturating_sub(field.elevations_mm[neighbor]) < threshold
                {
                    contours.push(Segment {
                        first: edge[0],
                        second: edge[1],
                    });
                }
            }
        }
    }
    contours
}

fn basin_cells_from_ids(basin_by_cell: &[u32], basin_count: usize) -> Vec<Vec<usize>> {
    basin_cells(basin_by_cell, basin_count)
}

fn watershed_groups(
    field: &PhysicalField,
    drainage: &DrainageField,
    lake_cells: &[bool],
) -> (Vec<u32>, Vec<Vec<usize>>) {
    let primary = primary_edges(field, drainage);
    let count = field.grid.sample_count();
    let mut ids = vec![u32::MAX; count];
    for (start, watershed_slot) in ids.iter_mut().enumerate() {
        if field.elevations_mm[start] <= field.sea_level_mm {
            continue;
        }
        let mut cell = start;
        let mut steps = 0usize;
        while field.elevations_mm[cell] > field.sea_level_mm && !lake_cells[cell] {
            let Some(edge) = primary[cell] else { break };
            cell = edge.destination_cell;
            steps += 1;
            if steps > count {
                break;
            }
        }
        *watershed_slot = cell.min(count - 1) as u32;
    }
    let mut groups = BTreeMap::<u32, Vec<usize>>::new();
    for (cell, watershed) in ids.iter().copied().enumerate() {
        if watershed != u32::MAX {
            groups.entry(watershed).or_default().push(cell);
        }
    }
    (ids, groups.into_values().collect())
}

pub fn derive_hydrology(
    field: &PhysicalField,
    climate: &ClimateField,
    drainage: &DrainageField,
    reference_water_inventory_m3: u64,
) -> Result<HydrologyField, PhysicalError> {
    derive_hydrology_with_crust(field, climate, drainage, reference_water_inventory_m3, None)
}

pub fn derive_hydrology_with_crust(
    field: &PhysicalField,
    climate: &ClimateField,
    drainage: &DrainageField,
    reference_water_inventory_m3: u64,
    crust_by_cell: Option<&[CrustType]>,
) -> Result<HydrologyField, PhysicalError> {
    field.validate().map_err(PhysicalError::InvalidSource)?;
    climate.validate()?;
    if climate.grid != field.grid || drainage.grid != field.grid {
        return Err(PhysicalError::coded(
            PhysicalErrorCode::GeometryInvalid,
            "hydrology inputs do not match the physical grid",
        ));
    }
    let (mut basins, basin_by_cell) = derive_basins(field, climate)?;
    solve_basin_water(&mut basins, field);
    let basin_cells = basin_cells_from_ids(&basin_by_cell, basins.len());
    let mut lake_cells = vec![false; field.grid.sample_count()];
    let mut lake_level_mm = vec![field.sea_level_mm; field.grid.sample_count()];
    for basin in &basins {
        if basin.water_level_mm <= field.sea_level_mm || basin.water_volume_m3 == 0 {
            continue;
        }
        for cell in &basin_cells[basin.id] {
            if field.elevations_mm[*cell] < basin.water_level_mm {
                lake_cells[*cell] = true;
                lake_level_mm[*cell] = basin.water_level_mm;
            }
        }
    }
    let inland_water = basins.iter().try_fold(0u64, |sum, basin| {
        checked_add(
            sum,
            basin.water_volume_m3,
            "inland water overflowed its bounded range",
        )
    })?;
    let tolerance = tolerance_m3(reference_water_inventory_m3);
    let initial_level = field.sea_level_mm;
    let mut sea_level = initial_level;
    let mut iterations = 0;
    let mut converged = false;
    for iteration in 1..=MAX_FIXED_POINT_ITERATIONS {
        iterations = iteration;
        let ocean = ocean_volume(field, sea_level);
        let total = ocean.saturating_add(inland_water);
        let error = reference_water_inventory_m3.abs_diff(total);
        if error <= tolerance {
            converged = true;
            break;
        }
        let area = ocean_area(field, sea_level).max(1.0);
        let delta = ((reference_water_inventory_m3 as f64 - total as f64) * 1_000.0 / area)
            .round()
            .clamp(-25_000.0, 25_000.0) as i32;
        if delta == 0 {
            break;
        }
        sea_level = sea_level.saturating_add(delta);
    }
    if !converged {
        return Err(PhysicalError::coded(
            PhysicalErrorCode::WaterNonConvergent,
            "ocean and inland water fixed-point solve did not converge",
        ));
    }
    let ocean_water = ocean_volume(field, sea_level);
    let total_water = ocean_water.saturating_add(inland_water);
    let balance_error = reference_water_inventory_m3.abs_diff(total_water);
    if balance_error > tolerance {
        return Err(PhysicalError::coded(
            PhysicalErrorCode::WaterNonConvergent,
            "final water balance exceeds tolerance",
        ));
    }
    let mut water_level_mm = vec![sea_level; field.grid.sample_count()];
    let mut bathymetry_mm = vec![0; field.grid.sample_count()];
    let mut hillshade_ppm = vec![0; field.grid.sample_count()];
    for cell in 0..field.grid.sample_count() {
        water_level_mm[cell] = if lake_cells[cell] {
            lake_level_mm[cell]
        } else {
            field.elevations_mm[cell].max(sea_level)
        };
        bathymetry_mm[cell] = sea_level.saturating_sub(field.elevations_mm[cell]).max(0);
        hillshade_ppm[cell] = hillshade(field, cell);
    }
    let (rivers, river_coordinates) =
        derive_rivers(field, drainage, &basins, &basin_by_cell, &lake_cells)?;
    let ocean_mask = (0..field.grid.sample_count())
        .map(|cell| field.elevations_mm[cell] <= sea_level)
        .collect::<Vec<_>>();
    let water_mask = ocean_mask
        .iter()
        .zip(lake_cells.iter())
        .map(|(ocean, lake)| *ocean || *lake)
        .collect::<Vec<_>>();
    let coastline_segments = water_boundary_segments(field, &water_mask);
    let lake_polygons = basins
        .iter()
        .filter(|basin| {
            basin.water_volume_m3 > 0
                && basin
                    .water_level_mm
                    .saturating_sub(basin.minimum_elevation_mm)
                    >= MIN_LAKE_DEPTH_MM
                && basin.water_level_mm > sea_level
        })
        .map(|basin| polygon_for_cells(field.grid, &basin_cells[basin.id]))
        .collect::<Vec<_>>();
    let (watershed_id, watershed_cells) = watershed_groups(field, drainage, &lake_cells);
    let watershed_polygons = watershed_cells
        .iter()
        .map(|cells| polygon_for_cells(field.grid, cells))
        .collect::<Vec<_>>();
    let shelf_cells = (0..field.grid.sample_count())
        .map(|cell| {
            if !ocean_mask[cell] {
                return false;
            }
            match crust_by_cell.and_then(|crust| crust.get(cell)) {
                Some(CrustType::Continental) => true,
                Some(CrustType::Oceanic) => false,
                None => bathymetry_mm[cell] <= 200_000,
            }
        })
        .collect::<Vec<_>>();
    let land_mask = (0..field.grid.sample_count())
        .map(|cell| field.elevations_mm[cell] > sea_level && !lake_cells[cell])
        .collect::<Vec<_>>();
    let land_polygons = polygon_for_mask(field.grid, &land_mask);
    let (island_id, island_cells) = island_groups(field.grid, &land_mask);
    let island_polygons = island_cells
        .iter()
        .map(|cells| polygon_for_cells(field.grid, cells))
        .collect::<Vec<_>>();
    let ocean_polygons = polygon_for_mask(field.grid, &ocean_mask);
    let shelf_polygons = polygon_for_mask(field.grid, &shelf_cells);
    let bathymetry_contours = bathymetry_contour_segments(field, &ocean_mask, sea_level);
    let metrics = WaterBalanceMetrics {
        total_water_m3: reference_water_inventory_m3,
        ocean_water_m3: ocean_water,
        inland_water_m3: inland_water,
        land_ice_m3: 0,
        balance_error_m3: balance_error,
        tolerance_m3: tolerance,
        fixed_point_iterations: iterations,
        converged,
        lake_count: lake_polygons.len() as u32,
        river_count: rivers.len() as u32,
        watershed_count: watershed_polygons.len() as u32,
        coastline_segment_count: coastline_segments.len() as u32,
        land_polygon_count: land_polygons.len() as u32,
        ocean_polygon_count: ocean_polygons.len() as u32,
        shelf_cell_count: shelf_cells.iter().filter(|cell| **cell).count() as u32,
        bathymetry_contour_count: bathymetry_contours.len() as u32,
        island_count: island_polygons.len() as u32,
    };
    let hydrology = HydrologyField {
        grid: field.grid,
        derivation_version: HYDROLOGY_DERIVATION_VERSION,
        sea_level_mm: sea_level,
        water_level_mm,
        lake_level_mm,
        slope_ppm: drainage.slope_ppm.clone(),
        hillshade_ppm,
        bathymetry_mm,
        watershed_id,
        basin_by_cell,
        lake_cells,
        shelf_cells,
        island_id,
        basins,
        rivers,
        river_coordinates,
        coastline_segments,
        lake_polygons,
        watershed_polygons,
        land_polygons,
        ocean_polygons,
        shelf_polygons,
        island_polygons,
        bathymetry_contours,
        metrics,
    };
    hydrology.validate_against(field, climate, drainage, reference_water_inventory_m3)?;
    Ok(hydrology)
}

fn geometry_string(coordinates: &[[i32; 2]]) -> String {
    let values = coordinates
        .iter()
        .map(|coordinate| {
            format!(
                "[{},{}]",
                coordinate[0] as f64 / 1_000_000.0,
                coordinate[1] as f64 / 1_000_000.0
            )
        })
        .collect::<Vec<_>>();
    format!("[{}]", values.join(","))
}

fn multi_polygon_from_rings(polygons: &[Vec<[i32; 2]>]) -> String {
    format!(
        "[{}]",
        polygons
            .iter()
            .map(|ring| format!("[{}]", geometry_string(ring)))
            .collect::<Vec<_>>()
            .join(",")
    )
}

fn multi_polygon_from_polygons(polygons: &[Vec<Vec<[i32; 2]>>]) -> String {
    format!(
        "[{}]",
        polygons
            .iter()
            .map(|polygon| {
                format!(
                    "[{}]",
                    polygon
                        .iter()
                        .map(|ring| geometry_string(ring))
                        .collect::<Vec<_>>()
                        .join(",")
                )
            })
            .collect::<Vec<_>>()
            .join(",")
    )
}

fn feature(
    id: &str,
    layer: &str,
    kind: &str,
    name: &str,
    geometry_type: &str,
    geometry: &str,
) -> String {
    format!(
        "{{\"type\":\"Feature\",\"id\":\"{id}\",\"properties\":{{\"daenaLayerId\":\"{layer}\",\"kind\":\"{kind}\",\"name\":\"{name}\"}},\"geometry\":{{\"type\":\"{geometry_type}\",\"coordinates\":{geometry}}}}}"
    )
}

pub fn to_geojson(hydrology: &HydrologyField) -> Result<String, String> {
    let mut features = Vec::new();
    for (index, segment) in hydrology.coastline_segments.iter().enumerate() {
        features.push(feature(
            &format!("current-coastline-{index:06}"),
            "base",
            "custom",
            "current coastline",
            "LineString",
            &geometry_string(&[segment.first, segment.second]),
        ));
    }
    for (id, name, polygons) in [
        ("land", "Exposed land", &hydrology.land_polygons),
        ("ocean", "Ocean", &hydrology.ocean_polygons),
        ("shelves", "Continental shelf", &hydrology.shelf_polygons),
        ("islands", "Islands", &hydrology.island_polygons),
    ] {
        if !polygons.is_empty() {
            features.push(feature(
                id,
                id,
                if id == "ocean" { "custom" } else { "land" },
                name,
                "MultiPolygon",
                &multi_polygon_from_polygons(polygons),
            ));
        }
    }
    for (index, segment) in hydrology.bathymetry_contours.iter().enumerate() {
        features.push(feature(
            &format!("bathymetry-contour-{index:06}"),
            "bathymetric-contours",
            "route",
            "Bathymetric contour",
            "LineString",
            &geometry_string(&[segment.first, segment.second]),
        ));
    }
    for (index, polygons) in hydrology.lake_polygons.iter().enumerate() {
        features.push(feature(
            &format!("lake-{index:06}"),
            "lakes",
            "lake",
            &format!("Lake {index}"),
            "MultiPolygon",
            &multi_polygon_from_rings(polygons),
        ));
    }
    for (index, polygons) in hydrology.watershed_polygons.iter().enumerate() {
        features.push(feature(
            &format!("watershed-{index:06}"),
            "watersheds",
            "region",
            &format!("Watershed {index}"),
            "MultiPolygon",
            &multi_polygon_from_rings(polygons),
        ));
    }
    for (river, coordinates) in hydrology.rivers.iter().zip(&hydrology.river_coordinates) {
        features.push(feature(
            &format!("river-{:06}", river.id),
            "rivers",
            "route",
            &format!("River order {}", river.strahler_order),
            "LineString",
            &geometry_string(coordinates),
        ));
    }
    if features.len() > MAX_HYDROLOGY_FEATURES {
        return Err("hydrology derived output exceeds the feature budget".into());
    }
    Ok(format!(
        "{{\"type\":\"FeatureCollection\",\"features\":[{}]}}",
        features.join(",")
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        climate::derive_current_climate, evolution::evolve_terrain,
        tectonics::generate_tectonic_world, NoopProgress,
    };

    fn fixture() -> HydrologyField {
        let grid = Grid::new(16, 8, crate::DEFAULT_RADIUS_METRES).unwrap();
        let tectonic_settings = crate::tectonics::TectonicSettings::default_for(grid);
        let mut progress = NoopProgress;
        let tectonics =
            generate_tectonic_world(grid, tectonic_settings, 300_000, 831_429, 0, &mut progress)
                .unwrap();
        let sea_level = crate::solve_sea_level(&grid, &tectonics.elevations_mm, 300_000).unwrap();
        let field = PhysicalField {
            grid,
            seed: 831_429,
            retry_index: 0,
            target_land_fraction_ppm: 300_000,
            sea_level_mm: sea_level,
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
            crate::evolution::EvolutionSettings::default(),
            &mut progress,
        )
        .unwrap();
        let final_field = PhysicalField {
            sea_level_mm: crate::solve_sea_level(&grid, &evolution.elevations_mm, 300_000).unwrap(),
            elevations_mm: evolution.elevations_mm.clone(),
            ..field
        };
        let final_climate = derive_current_climate(
            &final_field,
            crate::climate::ClimateSettings::default_for(grid),
            final_field.seed,
            final_field.retry_index,
            &mut progress,
        )
        .unwrap();
        let drainage = crate::evolution::derive_drainage(&final_field, &final_climate).unwrap();
        let total = crate::validate_field_report(&final_field)
            .unwrap()
            .reference_water_inventory_m3;
        derive_hydrology(&final_field, &final_climate, &drainage, total).unwrap()
    }

    #[test]
    fn water_balance_and_geometry_are_bounded() {
        let hydrology = fixture();
        assert!(hydrology.metrics.converged);
        assert!(hydrology.metrics.balance_error_m3 <= hydrology.metrics.tolerance_m3);
        assert!(
            hydrology
                .metrics
                .total_water_m3
                .abs_diff(hydrology.metrics.ocean_water_m3 + hydrology.metrics.inland_water_m3)
                <= hydrology.metrics.tolerance_m3
        );
        assert_eq!(hydrology.rivers.len(), hydrology.river_coordinates.len());
        assert!(hydrology
            .river_coordinates
            .iter()
            .all(|path| path.len() >= 2));
        assert!(hydrology
            .rivers
            .iter()
            .zip(&hydrology.river_coordinates)
            .all(|(river, path)| river.coordinate_count as usize == path.len()));
        assert!(hydrology
            .rivers
            .iter()
            .all(|river| river.strahler_order > 0));
        assert!(hydrology.rivers.iter().all(|river| {
            !river.spill_outlet
                || hydrology.basins.iter().any(|basin| {
                    basin.spill_cell == Some(river.source_cell)
                        && basin.outflow_m3_per_year > 0
                        && basin.destination == river.destination
                })
        }));
        for basin in &hydrology.basins {
            assert_ne!(basin.parent_basin, Some(basin.id));
            assert_eq!(
                basin.cell_count,
                hydrology
                    .basin_by_cell
                    .iter()
                    .filter(|id| **id == basin.id as u32)
                    .count() as u32
            );
        }
    }

    #[test]
    fn repeated_derivation_is_byte_stable_and_preserves_immutable_water_products() {
        let first = fixture();
        let second = fixture();
        assert_eq!(first, second);
        assert!(
            first
                .metrics
                .total_water_m3
                .abs_diff(first.metrics.ocean_water_m3 + first.metrics.inland_water_m3)
                <= first.metrics.tolerance_m3
        );
        assert!(first
            .lake_cells
            .iter()
            .enumerate()
            .all(|(cell, lake)| !*lake || first.water_level_mm[cell] > first.sea_level_mm));
    }

    #[test]
    fn hydrology_geojson_has_current_world_layers() {
        let hydrology = fixture();
        let json = to_geojson(&hydrology).unwrap();
        assert!(json.contains("watersheds"));
        assert!(json.contains("rivers") || hydrology.rivers.is_empty());
    }
}
