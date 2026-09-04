//! Deterministic current-world water and geography derivation.
//!
//! Hydrology is disposable. It consumes the canonical final elevation field,
//! current climate, and the Iteration 4 routing graph, then produces basin,
//! lake, river, watershed, coastline, and bounded raster diagnostics without
//! changing the physical-world-v1 source bytes.

use std::collections::{BTreeMap, VecDeque};
use std::fmt::Write as _;

use super::{
    climate::ClimateField,
    contours,
    evolution::{DrainageEdge, DrainageField},
    Grid, PhysicalError, PhysicalErrorCode, PhysicalField, Segment,
};
use crate::tectonics::CrustType;

pub const HYDROLOGY_DERIVATION_VERSION: u16 = 1;
pub const MAX_HYDROLOGY_FEATURES: usize = crate::MAX_GEOJSON_FEATURES;
pub const WATER_SOLVER_UNDERRELAXATION_PPM: u32 = 1_000_000;
const MAX_FIXED_POINT_ITERATIONS: u32 = 32;
const MIN_LAKE_DEPTH_MM: i32 = 10;
const BASE_LAKE_EVAPORATION_MM: u32 = 900;
const MAX_LAKE_EVAPORATION_MM: u32 = 6_000;
const OCEAN_LABEL: u32 = u32::MAX;
const MIN_ICE_PRECIPITATION_MM: u32 = 80;
const MAX_ICE_WATER_EQUIVALENT_MM: u32 = 220_000;
const MIN_ICE_COMPONENT_CELLS: usize = 8;

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
    pub children: Vec<usize>,
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
    pub ice_cells: Vec<bool>,
    pub ice_thickness_mm: Vec<u32>,
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
    pub ice_polygons: Vec<Vec<Vec<[i32; 2]>>>,
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
            || self.ice_cells.len() != count
            || self.ice_thickness_mm.len() != count
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
        let tolerance = tolerance_m3(field, self.sea_level_mm);
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
            let owner = self
                .basin_by_cell
                .get(basin.minimum_cell)
                .copied()
                .unwrap_or(u32::MAX);
            let minimum_in_subtree = owner != u32::MAX
                && (owner == basin.id as u32
                    || basin_is_descendant(&self.basins, owner as usize, basin.id));
            if basin.id >= self.basins.len()
                || basin_counts[basin.id] != basin.cell_count
                || basin.minimum_cell >= count
                || !minimum_in_subtree
                || basin.minimum_elevation_mm != field.elevations_mm[basin.minimum_cell]
                || basin.children.iter().any(|child| {
                    *child >= self.basins.len()
                        || self.basins[*child].parent_basin != Some(basin.id)
                })
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
                    && (*level <= self.sea_level_mm || *level != self.lake_level_mm[cell])
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
                let matched = self.basins.iter().any(|basin| {
                    basin.spill_cell == Some(river.source_cell)
                        && basin.outflow_m3_per_year > 0
                        && basin.status != BasinStatus::Merged
                        && basin.destination == river.destination
                });
                if !matched {
                    let owner = self.basin_by_cell.get(river.source_cell).copied();
                    eprintln!(
                        "hydrology spill-mismatch debug: river={} source={} mouth={} dest={:?} owner={:?} owner_spill={:?} owner_outflow={:?} owner_dest={:?}",
                        river.id,
                        river.source_cell,
                        river.mouth_cell,
                        river.destination,
                        owner,
                        owner.and_then(|id| self.basins.get(id as usize).map(|basin| basin.spill_cell)),
                        owner.and_then(|id| self.basins.get(id as usize).map(|basin| basin.outflow_m3_per_year)),
                        owner.and_then(|id| self.basins.get(id as usize).map(|basin| basin.destination)),
                    );
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
            if basin.outflow_m3_per_year > 0 && basin.status != BasinStatus::Merged && outlets > 1 {
                return Err(PhysicalError::coded(
                    PhysicalErrorCode::HydrologyInvalidSink,
                    "overflowing basin has more than one spill outlet",
                ));
            }
        }
        Ok(())
    }
}

fn basin_is_descendant(basins: &[Basin], mut cursor: usize, ancestor: usize) -> bool {
    for _ in 0..=basins.len() {
        let Some(parent) = basins.get(cursor).and_then(|basin| basin.parent_basin) else {
            return false;
        };
        if parent == ancestor {
            return true;
        }
        cursor = parent;
    }
    false
}

fn basin_drains_to_ocean(basins: &[Basin], mut id: usize) -> bool {
    for _ in 0..=basins.len() {
        let Some(basin) = basins.get(id) else {
            return false;
        };
        match basin.destination {
            BasinDestination::Ocean => return true,
            BasinDestination::Basin(parent) => id = parent,
            BasinDestination::Endorheic | BasinDestination::Junction(_) => return false,
        }
    }
    false
}

fn spill_neighbor_reaches_destination(
    field: &PhysicalField,
    basins: &[Basin],
    basin_by_cell: &[u32],
    from_basin: usize,
    destination: BasinDestination,
    cell: usize,
) -> bool {
    match destination {
        BasinDestination::Ocean => {
            if field.elevations_mm[cell] <= field.sea_level_mm {
                return true;
            }
            let owner = basin_by_cell[cell];
            if owner == OCEAN_LABEL {
                return true;
            }
            let owner = owner as usize;
            if owner == from_basin || basin_is_descendant(basins, owner, from_basin) {
                return false;
            }
            basin_drains_to_ocean(basins, owner)
        }
        BasinDestination::Basin(parent) => {
            let owner = basin_by_cell[cell] as usize;
            basin_by_cell[cell] == parent as u32
                || basins.get(parent).is_some_and(|parent_basin| {
                    parent_basin.children.contains(&owner) || parent_basin.id == owner
                })
        }
        BasinDestination::Endorheic | BasinDestination::Junction(_) => false,
    }
}

#[derive(Debug, Clone)]
struct VolumeCurve {
    knots: Vec<(i32, u64, u64)>,
}

impl VolumeCurve {
    fn from_cells(grid: Grid, elevations: &[i32], cells: &[usize], spill_mm: i32) -> Self {
        let row_areas = grid.row_areas();
        let mut samples = cells
            .iter()
            .filter_map(|cell| {
                let elevation = elevations[*cell];
                (elevation < spill_mm).then(|| {
                    let (row, _) = grid.row_col(*cell);
                    (elevation, *cell, row_areas[row as usize])
                })
            })
            .collect::<Vec<_>>();
        samples.sort_unstable_by_key(|(elevation, cell, _)| (*elevation, *cell));
        let mut levels = samples
            .iter()
            .map(|(elevation, _, _)| *elevation)
            .collect::<Vec<_>>();
        levels.push(spill_mm);
        levels.dedup();

        let mut knots = Vec::with_capacity(levels.len());
        let mut sample_index = 0usize;
        let mut area_m2 = 0.0_f64;
        let mut area_elevation_mm = 0.0_f64;
        for level in levels {
            while sample_index < samples.len() && samples[sample_index].0 < level {
                let (elevation, _, cell_area_m2) = samples[sample_index];
                area_m2 += cell_area_m2;
                area_elevation_mm += cell_area_m2 * f64::from(elevation);
                sample_index += 1;
            }
            let volume_m3 = (area_m2.mul_add(f64::from(level), -area_elevation_mm) / 1_000.0)
                .round()
                .clamp(0.0, u64::MAX as f64) as u64;
            knots.push((level, area_m2.round().max(0.0) as u64, volume_m3));
        }
        Self { knots }
    }

    fn volume_at(&self, level_mm: i32) -> u64 {
        self.knots
            .iter()
            .rev()
            .find(|(level, _, _)| *level <= level_mm)
            .map(|(_, _, volume)| *volume)
            .unwrap_or(0)
    }

    fn area_at(&self, level_mm: i32) -> u64 {
        self.knots
            .iter()
            .rev()
            .find(|(level, _, _)| *level <= level_mm)
            .map(|(_, area, _)| *area)
            .unwrap_or(0)
    }

    fn level_for_volume(&self, target: u64, min_mm: i32, spill_mm: i32) -> i32 {
        self.level_for_knot(target, min_mm, spill_mm, true)
    }

    fn level_for_area(&self, target_area: u64, min_mm: i32, spill_mm: i32) -> i32 {
        self.level_for_knot(target_area, min_mm, spill_mm, false)
    }

    fn level_for_knot(&self, target: u64, min_mm: i32, spill_mm: i32, by_volume: bool) -> i32 {
        if target == 0 || self.knots.is_empty() {
            return min_mm;
        }
        let mut low = 0usize;
        let mut high = self.knots.len();
        while low < high {
            let mid = (low + high) / 2;
            let value = if by_volume {
                self.knots[mid].2
            } else {
                self.knots[mid].1
            };
            if value < target {
                low = mid + 1;
            } else {
                high = mid;
            }
        }
        self.knots
            .get(low.min(self.knots.len().saturating_sub(1)))
            .map(|(level, _, _)| (*level).clamp(min_mm, spill_mm))
            .unwrap_or(min_mm)
    }
}

#[derive(Debug, Clone)]
struct DepressionTree {
    basins: Vec<Basin>,
    exclusive_cells: Vec<Vec<usize>>,
    children: Vec<Vec<usize>>,
    basin_by_cell: Vec<u32>,
    curves: Vec<VolumeCurve>,
    subtree_cells: Vec<Vec<usize>>,
}

fn find_catchment(uf: &mut [u32], mut id: u32) -> u32 {
    if id == OCEAN_LABEL {
        return OCEAN_LABEL;
    }
    let mut seen = Vec::new();
    while id != OCEAN_LABEL && uf[id as usize] != id {
        seen.push(id);
        id = uf[id as usize];
    }
    for previous in seen {
        uf[previous as usize] = id;
    }
    id
}

fn connecting_spill_cell(
    field: &PhysicalField,
    cell: usize,
    catchment: u32,
    cell_catchment: &[u32],
    uf: &mut [u32],
) -> usize {
    field
        .grid
        .topology()
        .neighbors(cell)
        .iter()
        .copied()
        .filter(|neighbor| {
            let label = cell_catchment[*neighbor];
            label != OCEAN_LABEL && find_catchment(uf, label) == catchment
        })
        .min_by_key(|neighbor| (field.elevations_mm[*neighbor], *neighbor))
        .unwrap_or(cell)
}

fn build_depression_tree(field: &PhysicalField, sea_level_mm: i32) -> DepressionTree {
    let count = field.grid.sample_count();
    let topology = field.grid.topology();
    let mut cell_catchment = vec![OCEAN_LABEL; count];
    let mut processed = vec![false; count];
    let mut land = Vec::new();
    for (cell, elevation) in field.elevations_mm.iter().enumerate() {
        if *elevation <= sea_level_mm {
            processed[cell] = true;
        } else {
            land.push(cell);
        }
    }
    land.sort_unstable_by_key(|cell| (field.elevations_mm[*cell], *cell));
    let mut uf = Vec::<u32>::new();
    let mut exclusive_cells = Vec::<Vec<usize>>::new();
    let mut children = Vec::<Vec<usize>>::new();
    let mut minima = Vec::<(usize, i32)>::new();
    let mut spills = Vec::<Option<(usize, i32, BasinDestination)>>::new();
    let mut parents = Vec::<Option<usize>>::new();
    for cell in land {
        let mut neighbor_labels = topology
            .neighbors(cell)
            .iter()
            .copied()
            .filter(|neighbor| processed[*neighbor])
            .map(|neighbor| {
                if field.elevations_mm[neighbor] <= sea_level_mm {
                    OCEAN_LABEL
                } else {
                    find_catchment(&mut uf, cell_catchment[neighbor])
                }
            })
            .collect::<Vec<_>>();
        neighbor_labels.sort_unstable();
        neighbor_labels.dedup();
        let touches_ocean = neighbor_labels.contains(&OCEAN_LABEL);
        let land_labels = neighbor_labels
            .iter()
            .copied()
            .filter(|label| *label != OCEAN_LABEL)
            .collect::<Vec<_>>();
        let assigned = if land_labels.is_empty() {
            if touches_ocean {
                OCEAN_LABEL
            } else {
                let id = exclusive_cells.len() as u32;
                uf.push(id);
                exclusive_cells.push(vec![cell]);
                children.push(Vec::new());
                minima.push((cell, field.elevations_mm[cell]));
                spills.push(None);
                parents.push(None);
                id
            }
        } else if land_labels.len() == 1 && !touches_ocean {
            let id = land_labels[0];
            exclusive_cells[id as usize].push(cell);
            id
        } else if land_labels.len() == 1 && touches_ocean {
            let id = land_labels[0];
            let already_ocean = matches!(
                spills[id as usize].map(|spill| spill.2),
                Some(BasinDestination::Ocean)
            );
            if !already_ocean {
                let spill_cell = connecting_spill_cell(field, cell, id, &cell_catchment, &mut uf);
                spills[id as usize] = Some((
                    spill_cell,
                    field.elevations_mm[cell],
                    BasinDestination::Ocean,
                ));
            }
            uf[id as usize] = OCEAN_LABEL;
            OCEAN_LABEL
        } else {
            let parent = exclusive_cells.len() as u32;
            uf.push(parent);
            exclusive_cells.push(vec![cell]);
            children.push(land_labels.iter().map(|label| *label as usize).collect());
            let (minimum_cell, minimum_elevation) = land_labels
                .iter()
                .map(|label| minima[*label as usize])
                .min_by_key(|value| (value.1, value.0))
                .unwrap_or((cell, field.elevations_mm[cell]));
            minima.push((minimum_cell, minimum_elevation));
            parents.push(None);
            let destination = if touches_ocean {
                BasinDestination::Ocean
            } else {
                BasinDestination::Endorheic
            };
            spills.push(Some((cell, field.elevations_mm[cell], destination)));
            for child in &land_labels {
                let spill_cell =
                    connecting_spill_cell(field, cell, *child, &cell_catchment, &mut uf);
                spills[*child as usize] = Some((
                    spill_cell,
                    field.elevations_mm[cell],
                    BasinDestination::Basin(parent as usize),
                ));
                parents[*child as usize] = Some(parent as usize);
                uf[*child as usize] = if touches_ocean { OCEAN_LABEL } else { parent };
            }
            if touches_ocean {
                uf[parent as usize] = OCEAN_LABEL;
                OCEAN_LABEL
            } else {
                parent
            }
        };
        cell_catchment[cell] = assigned;
        processed[cell] = true;
    }
    let mut basin_by_cell = vec![OCEAN_LABEL; count];
    for (id, cells) in exclusive_cells.iter().enumerate() {
        for cell in cells {
            basin_by_cell[*cell] = id as u32;
        }
    }
    let subtree_cells = {
        let mut subtree = vec![Vec::new(); exclusive_cells.len()];
        let mut order = (0..exclusive_cells.len()).collect::<Vec<_>>();
        order.sort_unstable_by_key(|id| (parents[*id].is_some(), *id));
        let mut remaining = exclusive_cells.len();
        let mut placed = vec![false; exclusive_cells.len()];
        let mut guard = 0usize;
        while remaining > 0 && guard <= exclusive_cells.len() + 1 {
            guard += 1;
            let mut progressed = false;
            for id in 0..exclusive_cells.len() {
                if placed[id] {
                    continue;
                }
                if children[id].iter().any(|child| !placed[*child]) {
                    continue;
                }
                let mut cells = exclusive_cells[id].clone();
                for child in &children[id] {
                    cells.extend_from_slice(&subtree[*child]);
                }
                cells.sort_unstable();
                cells.dedup();
                subtree[id] = cells;
                placed[id] = true;
                remaining -= 1;
                progressed = true;
            }
            if !progressed {
                break;
            }
        }
        subtree
    };
    let mut basins = Vec::with_capacity(exclusive_cells.len());
    let mut curves = Vec::with_capacity(exclusive_cells.len());
    for id in 0..exclusive_cells.len() {
        let spill = spills[id];
        let spill_elevation = spill.map(|value| value.1).unwrap_or_else(|| {
            subtree_cells[id]
                .iter()
                .map(|cell| field.elevations_mm[*cell])
                .max()
                .unwrap_or(minima[id].1)
        });
        let destination = spill
            .map(|value| value.2)
            .unwrap_or(BasinDestination::Endorheic);
        let curve = VolumeCurve::from_cells(
            field.grid,
            &field.elevations_mm,
            &subtree_cells[id],
            spill_elevation,
        );
        let volume_to_spill = curve.volume_at(spill_elevation);
        basins.push(Basin {
            id,
            minimum_cell: minima[id].0,
            minimum_elevation_mm: minima[id].1,
            cell_count: exclusive_cells[id].len() as u32,
            spill_cell: spill.map(|value| value.0),
            spill_elevation_mm: spill.map(|value| value.1),
            volume_to_spill_m3: volume_to_spill,
            parent_basin: parents[id],
            children: children[id].clone(),
            destination,
            water_level_mm: minima[id].1,
            water_volume_m3: 0,
            inflow_m3_per_year: 0,
            direct_precipitation_m3_per_year: 0,
            evaporation_m3_per_year: 0,
            outflow_m3_per_year: 0,
            status: if matches!(destination, BasinDestination::Endorheic) {
                BasinStatus::Endorheic
            } else {
                BasinStatus::Dry
            },
        });
        curves.push(curve);
    }
    DepressionTree {
        basins,
        exclusive_cells,
        children,
        basin_by_cell,
        curves,
        subtree_cells,
    }
}

#[cfg(test)]
fn precipitation_volume(grid: Grid, cell: usize, millimetres: u32) -> u64 {
    (grid.cell_area(grid.row_col(cell).0) * f64::from(millimetres) / 1_000.0)
        .round()
        .max(0.0) as u64
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

fn tolerance_m3(field: &PhysicalField, sea_level_mm: i32) -> u64 {
    let millimetre_step = (ocean_area(field, sea_level_mm) / 1_000.0).round().max(1.0) as u64;
    millimetre_step.max(2 * field.grid.sample_count() as u64)
}

/// Choose the bounded sea level whose ocean volume is nearest to an inventory.
///
/// This helper is intentionally independent of inland storage. Historical
/// derivation uses it only to select the immutable terrain's initial basin
/// topology before the existing inland/ocean fixed-point solve runs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OceanVolumeSample {
    pub sea_level_mm: i32,
    pub ocean_volume_m3: u64,
}

/// Monotonic `(sea level → ocean volume)` samples for the canonical elevation field.
///
/// Built once from the accepted terrain. Historical epochs look up a waterline
/// here instead of bisecting `ocean_volume` on the grid.
pub fn ocean_volume_curve(field: &PhysicalField) -> Result<Vec<OceanVolumeSample>, PhysicalError> {
    field.validate().map_err(PhysicalError::InvalidSource)?;
    let mut order: Vec<usize> = (0..field.elevations_mm.len()).collect();
    order.sort_unstable_by_key(|&cell| (field.elevations_mm[cell], cell));
    let Some((&first, &last)) = order.first().zip(order.last()) else {
        return Err(PhysicalError::coded(
            PhysicalErrorCode::GeometryInvalid,
            "ocean-volume curve requires at least one elevation sample",
        ));
    };
    let minimum = field.elevations_mm[first];
    let maximum = field.elevations_mm[last];
    let mut samples = Vec::with_capacity(order.len().saturating_add(2));
    samples.push(OceanVolumeSample {
        sea_level_mm: minimum.saturating_sub(1),
        ocean_volume_m3: 0,
    });
    let mut index = 0;
    let mut sum_area = 0.0;
    let mut sum_area_elev = 0.0;
    while index < order.len() {
        let level = field.elevations_mm[order[index]];
        while index < order.len() && field.elevations_mm[order[index]] == level {
            let cell = order[index];
            let area = field.grid.cell_area(field.grid.row_col(cell).0);
            sum_area += area;
            sum_area_elev += area * f64::from(level);
            index += 1;
        }
        let volume = ((sum_area * f64::from(level) - sum_area_elev) / 1_000.0)
            .round()
            .max(0.0) as u64;
        samples.push(OceanVolumeSample {
            sea_level_mm: level,
            ocean_volume_m3: volume,
        });
    }
    let top = maximum.saturating_add(1);
    let top_volume = ((sum_area * f64::from(top) - sum_area_elev) / 1_000.0)
        .round()
        .max(0.0) as u64;
    samples.push(OceanVolumeSample {
        sea_level_mm: top,
        ocean_volume_m3: top_volume,
    });
    Ok(samples)
}

pub fn lookup_ocean_level(
    curve: &[OceanVolumeSample],
    inventory_m3: u64,
) -> Result<i32, PhysicalError> {
    if curve.is_empty() {
        return Err(PhysicalError::coded(
            PhysicalErrorCode::GeometryInvalid,
            "ocean-volume curve is empty",
        ));
    }
    let mut low = 0usize;
    let mut high = curve.len();
    while low < high {
        let mid = (low + high) / 2;
        if curve[mid].ocean_volume_m3 >= inventory_m3 {
            high = mid;
        } else {
            low = mid + 1;
        }
    }
    let best = if low == 0 {
        curve[0]
    } else if low >= curve.len() {
        curve[curve.len() - 1]
    } else {
        let left = curve[low - 1];
        let right = curve[low];
        if right.ocean_volume_m3.abs_diff(inventory_m3)
            < left.ocean_volume_m3.abs_diff(inventory_m3)
        {
            right
        } else {
            left
        }
    };
    Ok(best.sea_level_mm)
}

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
    let mut low = 0usize;
    let mut high = candidates.len();
    while low < high {
        let mid = (low + high) / 2;
        if ocean_volume(field, candidates[mid]) >= inventory_m3 {
            high = mid;
        } else {
            low = mid + 1;
        }
    }
    let best = if low == 0 {
        candidates[0]
    } else if low >= candidates.len() {
        candidates[candidates.len() - 1]
    } else {
        let left = candidates[low - 1];
        let right = candidates[low];
        let left_error = ocean_volume(field, left).abs_diff(inventory_m3);
        let right_error = ocean_volume(field, right).abs_diff(inventory_m3);
        if right_error < left_error {
            right
        } else {
            left
        }
    };
    Ok(best)
}

fn solve_ocean_level_mm(field: &PhysicalField, inventory_m3: u64) -> Result<i32, PhysicalError> {
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
    let mut low = minimum.saturating_sub(1);
    let mut high = maximum.saturating_add(1);
    while low < high {
        let mid = low + (high - low) / 2;
        if ocean_volume(field, mid) >= inventory_m3 {
            high = mid;
        } else {
            low = mid.saturating_add(1);
        }
    }
    let left = low.saturating_sub(1);
    let right = low;
    let left_error = ocean_volume(field, left).abs_diff(inventory_m3);
    let right_error = ocean_volume(field, right).abs_diff(inventory_m3);
    Ok(if right_error < left_error {
        right
    } else {
        left
    })
}

fn ice_water_equivalent_mm(temperature_centi_c: i32, precipitation_mm: u32) -> u32 {
    if temperature_centi_c >= 0 || precipitation_mm < MIN_ICE_PRECIPITATION_MM {
        return 0;
    }
    let cold = u64::from((-temperature_centi_c).clamp(0, 4_000) as u32);
    let wet = u64::from(precipitation_mm.min(2_500));
    ((u64::from(MAX_ICE_WATER_EQUIVALENT_MM) * cold * wet) / (4_000 * 2_500)) as u32
}

fn prune_small_ice(grid: Grid, ice_cells: &mut [bool], ice_thickness_mm: &mut [u32]) {
    let min_cells = if grid.width >= 128 {
        MIN_ICE_COMPONENT_CELLS
    } else {
        1
    };
    let topology = grid.topology();
    let mut seen = vec![false; grid.sample_count()];
    for start in 0..grid.sample_count() {
        if !ice_cells[start] || seen[start] {
            continue;
        }
        let mut stack = vec![start];
        seen[start] = true;
        let mut cells = Vec::new();
        while let Some(cell) = stack.pop() {
            cells.push(cell);
            for neighbor in topology.neighbors(cell) {
                if ice_cells[*neighbor] && !seen[*neighbor] {
                    seen[*neighbor] = true;
                    stack.push(*neighbor);
                }
            }
        }
        if cells.len() < min_cells {
            for cell in cells {
                ice_cells[cell] = false;
                ice_thickness_mm[cell] = 0;
            }
        }
    }
}

fn derive_land_ice(
    field: &PhysicalField,
    climate: &ClimateField,
    sea_level_mm: i32,
    inventory_m3: u64,
) -> Result<(Vec<bool>, Vec<u32>, u64), PhysicalError> {
    let count = field.grid.sample_count();
    let mut ice_cells = vec![false; count];
    let mut ice_thickness_mm = vec![0u32; count];
    for cell in 0..count {
        if field.elevations_mm[cell] <= sea_level_mm {
            continue;
        }
        let thickness = ice_water_equivalent_mm(
            climate.temperature_centi_c[cell],
            climate.precipitation_mm_per_year[cell],
        );
        if thickness == 0 {
            continue;
        }
        ice_cells[cell] = true;
        ice_thickness_mm[cell] = thickness;
    }
    prune_small_ice(field.grid, &mut ice_cells, &mut ice_thickness_mm);
    let mut volume = 0u64;
    for cell in 0..count {
        if !ice_cells[cell] {
            continue;
        }
        let area = field.grid.cell_area(field.grid.row_col(cell).0);
        let cell_volume = (area * f64::from(ice_thickness_mm[cell]) / 1_000.0)
            .round()
            .max(0.0) as u64;
        volume = checked_add(volume, cell_volume, "land ice overflowed its bounded range")?;
    }
    if volume > inventory_m3 && volume > 0 {
        let keep_ppm = (inventory_m3.saturating_mul(1_000_000) / volume).min(1_000_000);
        volume = 0;
        for cell in 0..count {
            if !ice_cells[cell] {
                continue;
            }
            ice_thickness_mm[cell] =
                ((u64::from(ice_thickness_mm[cell]) * keep_ppm) / 1_000_000) as u32;
            if ice_thickness_mm[cell] == 0 {
                ice_cells[cell] = false;
                continue;
            }
            let area = field.grid.cell_area(field.grid.row_col(cell).0);
            volume = checked_add(
                volume,
                (area * f64::from(ice_thickness_mm[cell]) / 1_000.0)
                    .round()
                    .max(0.0) as u64,
                "scaled land ice overflowed its bounded range",
            )?;
        }
    }
    Ok((ice_cells, ice_thickness_mm, volume.min(inventory_m3)))
}

fn lake_evaporation_mm(temperature_centi_c: i32, maritime_ppm: u32) -> u32 {
    let warmth = i64::from((1_400 + temperature_centi_c).clamp(0, 5_400));
    let dryness = i64::from(1_000_000u32.saturating_sub(maritime_ppm.min(1_000_000)));
    let millimetres = i64::from(BASE_LAKE_EVAPORATION_MM) * warmth / 1_400 * dryness / 1_000_000;
    millimetres.clamp(0, i64::from(MAX_LAKE_EVAPORATION_MM)) as u32
}

fn mean_climate(cells: &[usize], climate: &ClimateField) -> (i32, u32, u32) {
    if cells.is_empty() {
        return (0, 0, 0);
    }
    let mut temperature = 0i64;
    let mut precipitation = 0u64;
    let mut maritime = 0u64;
    for cell in cells {
        temperature += i64::from(climate.temperature_centi_c[*cell]);
        precipitation += u64::from(climate.precipitation_mm_per_year[*cell]);
        maritime += u64::from(climate.maritime_factor_ppm[*cell]);
    }
    let count = cells.len() as i64;
    (
        (temperature / count) as i32,
        (precipitation / cells.len() as u64) as u32,
        (maritime / cells.len() as u64) as u32,
    )
}

fn postorder(children: &[Vec<usize>]) -> Vec<usize> {
    let mut order = Vec::with_capacity(children.len());
    let mut seen = vec![false; children.len()];
    fn visit(id: usize, children: &[Vec<usize>], seen: &mut [bool], order: &mut Vec<usize>) {
        if seen[id] {
            return;
        }
        seen[id] = true;
        for child in &children[id] {
            visit(*child, children, seen, order);
        }
        order.push(id);
    }
    for id in 0..children.len() {
        visit(id, children, &mut seen, &mut order);
    }
    order
}

fn solve_depression_water(
    tree: &mut DepressionTree,
    climate: &ClimateField,
) -> Result<u64, PhysicalError> {
    let order = postorder(&tree.children);
    for id in order {
        let exclusive_runoff = tree.exclusive_cells[id]
            .iter()
            .map(|cell| climate.runoff_volume_m3_per_year[*cell])
            .fold(0u64, u64::saturating_add);
        let child_outflow = tree.children[id]
            .iter()
            .map(|child| tree.basins[*child].outflow_m3_per_year)
            .fold(0u64, u64::saturating_add);
        let inflow = exclusive_runoff.saturating_add(child_outflow);
        let (temperature, precipitation_mm, maritime) =
            mean_climate(&tree.subtree_cells[id], climate);
        let evap_mm = lake_evaporation_mm(temperature, maritime);
        let min_mm = tree.basins[id].minimum_elevation_mm;
        let spill_mm = tree.basins[id].spill_elevation_mm.unwrap_or_else(|| {
            tree.curves[id]
                .knots
                .last()
                .map(|(level, _, _)| *level)
                .unwrap_or(min_mm)
        });
        let area_spill = tree.curves[id].area_at(spill_mm);
        let precip_spill = precipitation_volume_from_area(area_spill, precipitation_mm);
        let evap_spill = precipitation_volume_from_area(area_spill, evap_mm);
        let net_spill = inflow
            .saturating_add(precip_spill)
            .saturating_sub(evap_spill);
        let can_overflow = !matches!(tree.basins[id].destination, BasinDestination::Endorheic);
        let curve = tree.curves[id].clone();
        let basin = &mut tree.basins[id];
        if can_overflow && net_spill > 0 {
            basin.water_level_mm = spill_mm;
            basin.water_volume_m3 = curve.volume_at(spill_mm);
            basin.inflow_m3_per_year = inflow;
            basin.direct_precipitation_m3_per_year = precip_spill;
            basin.evaporation_m3_per_year = evap_spill;
            basin.outflow_m3_per_year = net_spill;
            basin.status = if matches!(basin.destination, BasinDestination::Basin(_)) {
                BasinStatus::Merged
            } else {
                BasinStatus::Overflowing
            };
            continue;
        }
        let closed_net = |level: i32| -> i128 {
            let area = curve.area_at(level);
            i128::from(inflow) + i128::from(precipitation_volume_from_area(area, precipitation_mm))
                - i128::from(precipitation_volume_from_area(area, evap_mm))
        };
        if closed_net(min_mm) <= 0 && evap_mm >= precipitation_mm && inflow == 0 {
            basin.water_level_mm = min_mm;
            basin.water_volume_m3 = 0;
            basin.inflow_m3_per_year = inflow;
            basin.direct_precipitation_m3_per_year = 0;
            basin.evaporation_m3_per_year = 0;
            basin.outflow_m3_per_year = 0;
            basin.status = if matches!(basin.destination, BasinDestination::Endorheic) {
                BasinStatus::Endorheic
            } else {
                BasinStatus::Dry
            };
            continue;
        }
        let surplus = evap_mm.saturating_sub(precipitation_mm);
        let level = if surplus == 0 {
            spill_mm
        } else {
            let target_area = ((inflow as u128 * 1_000) / u128::from(surplus).max(1)) as u64;
            curve.level_for_area(target_area, min_mm, spill_mm)
        };
        let area = curve.area_at(level);
        let precip = precipitation_volume_from_area(area, precipitation_mm);
        let evap = precipitation_volume_from_area(area, evap_mm);
        basin.water_level_mm = level;
        basin.water_volume_m3 = curve.volume_at(level);
        basin.inflow_m3_per_year = inflow;
        basin.direct_precipitation_m3_per_year = precip;
        basin.evaporation_m3_per_year = evap;
        basin.outflow_m3_per_year = 0;
        basin.status = if basin.water_volume_m3 == 0 {
            if matches!(basin.destination, BasinDestination::Endorheic) {
                BasinStatus::Endorheic
            } else {
                BasinStatus::Dry
            }
        } else if matches!(basin.destination, BasinDestination::Endorheic) {
            BasinStatus::Endorheic
        } else {
            BasinStatus::Active
        };
    }
    let mut inland = 0u64;
    for basin in &tree.basins {
        if basin.status == BasinStatus::Merged {
            continue;
        }
        inland = checked_add(
            inland,
            basin.water_volume_m3,
            "inland water overflowed its bounded range",
        )?;
    }
    Ok(inland)
}

fn inland_budget(field: &PhysicalField, inventory_m3: u64, land_ice_m3: u64) -> u64 {
    let minimum = field.elevations_mm.iter().copied().min().unwrap_or(0);
    let min_ocean = ocean_volume(field, minimum);
    inventory_m3
        .saturating_sub(land_ice_m3)
        .saturating_sub(min_ocean)
}

fn scale_inland_to_budget(tree: &mut DepressionTree, inland: u64, max_inland: u64) -> u64 {
    if inland == 0 || inland <= max_inland {
        return inland;
    }
    let ppm = ((u128::from(max_inland) * 1_000_000) / u128::from(inland).max(1)) as u64;
    let mut volumes = Vec::with_capacity(tree.basins.len());
    for (id, basin) in tree.basins.iter().enumerate() {
        if basin.status == BasinStatus::Merged || basin.water_volume_m3 == 0 {
            volumes.push(None);
            continue;
        }
        let volume = ((u128::from(basin.water_volume_m3) * u128::from(ppm)) / 1_000_000) as u64;
        let spill = basin
            .spill_elevation_mm
            .unwrap_or(basin.minimum_elevation_mm);
        let level = tree.curves[id].level_for_volume(volume, basin.minimum_elevation_mm, spill);
        volumes.push(Some((volume, level)));
    }
    for (id, basin) in tree.basins.iter_mut().enumerate() {
        let Some((volume, level)) = volumes[id] else {
            continue;
        };
        basin.water_volume_m3 = volume;
        basin.water_level_mm = level;
        if basin.water_volume_m3 == 0 {
            basin.status = if matches!(basin.destination, BasinDestination::Endorheic) {
                BasinStatus::Endorheic
            } else {
                BasinStatus::Dry
            };
        }
    }
    tree.basins
        .iter()
        .filter(|basin| basin.status != BasinStatus::Merged)
        .map(|basin| basin.water_volume_m3)
        .fold(0u64, u64::saturating_add)
}

fn precipitation_volume_from_area(area_m2: u64, millimetres: u32) -> u64 {
    (area_m2 as f64 * f64::from(millimetres) / 1_000.0)
        .round()
        .max(0.0) as u64
}

fn hillshade(field: &PhysicalField, cell: usize) -> u32 {
    let (row, col) = field.grid.row_col(cell);
    let width = field.grid.width as i32;
    let height = field.grid.height as i32;
    let sample = |row_offset: i32, col_offset: i32| {
        let sample_row = (row as i32 + row_offset).clamp(0, height - 1) as u32;
        let sample_col = ((col as i32 + col_offset).rem_euclid(width)) as u32;
        f64::from(field.elevations_mm[field.grid.index(sample_row, sample_col)])
    };
    let z1 = sample(-1, -1);
    let z2 = sample(-1, 0);
    let z3 = sample(-1, 1);
    let z4 = sample(0, -1);
    let z6 = sample(0, 1);
    let z7 = sample(1, -1);
    let z8 = sample(1, 0);
    let z9 = sample(1, 1);
    let cell_metres = (std::f64::consts::PI * field.grid.radius_metres as f64
        / f64::from(field.grid.height.max(1)))
    .max(1.0);
    let dzdx = ((z3 + 2.0 * z6 + z9) - (z1 + 2.0 * z4 + z7)) / (8.0 * cell_metres);
    let dzdy = ((z7 + 2.0 * z8 + z9) - (z1 + 2.0 * z2 + z3)) / (8.0 * cell_metres);
    let slope = dzdx.hypot(dzdy).atan();
    let aspect = (-dzdx).atan2(dzdy);
    let zenith = 45.0_f64.to_radians();
    let azimuth = 315.0_f64.to_radians();
    let illumination = (zenith.cos() * slope.cos()
        + zenith.sin() * slope.sin() * (azimuth - aspect).cos())
    .clamp(0.0, 1.0);
    (illumination * 1_000_000.0).round().clamp(0.0, 1_000_000.0) as u32
}

fn water_boundary_segments(
    field: &PhysicalField,
    water: &[bool],
) -> Result<Vec<Segment>, PhysicalError> {
    let paths = contours::polygons_from_mask(field.grid, water)?
        .into_iter()
        .flatten()
        .collect::<Vec<_>>();
    Ok(contours::segments_from_paths(&paths))
}

fn bathymetry_contour_segments(
    field: &PhysicalField,
    ocean_mask: &[bool],
    sea_level_mm: i32,
) -> Result<Vec<Segment>, PhysicalError> {
    let mut scalar = vec![-1; field.grid.sample_count()];
    for (cell, ocean) in ocean_mask.iter().copied().enumerate() {
        if ocean {
            scalar[cell] = sea_level_mm
                .saturating_sub(field.elevations_mm[cell])
                .max(0);
        }
    }
    let mut contours_out = Vec::new();
    for threshold in [100_000, 250_000, 500_000] {
        let paths = contours::isolines_at(field.grid, &scalar, threshold)?;
        contours_out.extend(contours::segments_from_paths(&paths));
    }
    Ok(contours_out)
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

fn nearest_sea_cell(field: &PhysicalField, basin_by_cell: &[u32], start: usize) -> Option<usize> {
    if field.elevations_mm[start] <= field.sea_level_mm {
        return Some(start);
    }
    let topology = field.grid.topology();
    let count = field.grid.sample_count();
    let mut seen = vec![false; count];
    let mut queue = VecDeque::from([start]);
    seen[start] = true;
    let mut found = Vec::new();
    while !queue.is_empty() {
        let layer = queue.len();
        for _ in 0..layer {
            let cell = queue.pop_front()?;
            for neighbor in topology.neighbors(cell) {
                if seen[*neighbor] {
                    continue;
                }
                seen[*neighbor] = true;
                if field.elevations_mm[*neighbor] <= field.sea_level_mm {
                    found.push(*neighbor);
                    continue;
                }
                if basin_by_cell[*neighbor] == OCEAN_LABEL {
                    queue.push_back(*neighbor);
                }
            }
        }
        if !found.is_empty() {
            return found
                .into_iter()
                .min_by_key(|cell| (field.elevations_mm[*cell], *cell));
        }
    }
    None
}

fn destination_for_cell(
    field: &PhysicalField,
    basins: &[Basin],
    basin_by_cell: &[u32],
    cell: usize,
) -> BasinDestination {
    if field.elevations_mm[cell] <= field.sea_level_mm
        || basin_by_cell.get(cell).copied() == Some(OCEAN_LABEL)
    {
        return BasinDestination::Ocean;
    }
    basin_by_cell
        .get(cell)
        .copied()
        .filter(|basin| *basin != OCEAN_LABEL)
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
    let mut parents_of = vec![Vec::<usize>::new(); count];
    for (parent, edge) in primary.iter().enumerate() {
        if !active[parent] {
            continue;
        }
        if let Some(edge) = edge {
            if edge.destination_cell < count && active[edge.destination_cell] {
                parents_of[edge.destination_cell].push(parent);
            }
        }
    }
    fn stream_order(cell: usize, parents_of: &[Vec<usize>], cache: &mut [u16]) -> u16 {
        if cache[cell] > 0 {
            return cache[cell];
        }
        let parents = parents_of[cell]
            .iter()
            .map(|parent| stream_order(*parent, parents_of, cache))
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
            path.push(cell);
            steps += 1;
            let Some(edge) = primary[cell] else {
                if field.elevations_mm[cell] > field.sea_level_mm
                    && basin_by_cell[cell] == OCEAN_LABEL
                {
                    if let Some(outlet) = nearest_sea_cell(field, basin_by_cell, cell) {
                        if outlet != cell {
                            path.push(outlet);
                            cell = outlet;
                        }
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
                path.push(next);
                cell = next;
                break;
            }
            if active[next] && incoming[next] > 1 {
                path.push(next);
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
            let (row, col) = field.grid.row_col(mouth);
            eprintln!(
                "hydrology river-terminal debug: start={} mouth={} row={} col={} elev={} sea={} lake={} primary={:?} dest={:?} path_len={} ended_at_junction={}",
                start,
                mouth,
                row,
                col,
                field.elevations_mm[mouth],
                field.sea_level_mm,
                lake_cells[mouth],
                primary[mouth].map(|edge| edge.destination_cell),
                destination,
                path.len(),
                ended_at_junction,
            );
            for neighbor in field.grid.topology().neighbors(mouth) {
                let (nrow, ncol) = field.grid.row_col(*neighbor);
                eprintln!(
                    "  neighbor={} row={} col={} elev={} owner={} sea={} lake={} active={}",
                    neighbor,
                    nrow,
                    ncol,
                    field.elevations_mm[*neighbor],
                    basin_by_cell[*neighbor],
                    field.elevations_mm[*neighbor] <= field.sea_level_mm,
                    lake_cells[*neighbor],
                    active[*neighbor],
                );
            }
            return Err(PhysicalError::coded(
                PhysicalErrorCode::HydrologyInvalidSink,
                "river terminates without an ocean or valid endorheic basin",
            ));
        }
        let coords = river_coordinates_from_cells(field, &path, destination, lake_cells, basins)?;
        let simplified = simplify_path(coords);
        let id = segments.len() as u32;
        segments.push(RiverSegment {
            id,
            source_cell: start,
            mouth_cell: mouth,
            strahler_order: stream_order(start, &parents_of, &mut order_cache),
            destination,
            spill_outlet: false,
            coordinate_count: simplified.len() as u32,
        });
        coordinates.push(simplified);
    }
    for basin in basins {
        if basin.outflow_m3_per_year == 0 || basin.status == BasinStatus::Merged {
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
        }
        let Some(outlet_cell) = field
            .grid
            .topology()
            .neighbors(spill_cell)
            .iter()
            .copied()
            .filter(|neighbor| {
                spill_neighbor_reaches_destination(
                    field,
                    basins,
                    basin_by_cell,
                    basin.id,
                    destination,
                    *neighbor,
                )
            })
            .min_by_key(|neighbor| (field.elevations_mm[*neighbor], *neighbor))
        else {
            let (row, col) = field.grid.row_col(spill_cell);
            eprintln!(
                "hydrology invalid-sink debug: basin={} status={:?} dest={:?} parent={:?} children={:?} spill_cell={} row={} col={} spill_elev={:?} water_mm={} sea_mm={} basin_label={}",
                basin.id,
                basin.status,
                basin.destination,
                basin.parent_basin,
                basin.children,
                spill_cell,
                row,
                col,
                basin.spill_elevation_mm,
                basin.water_level_mm,
                field.sea_level_mm,
                basin_by_cell[spill_cell],
            );
            for neighbor in field.grid.topology().neighbors(spill_cell) {
                let (nrow, ncol) = field.grid.row_col(*neighbor);
                let owner = basin_by_cell[*neighbor];
                let owner_dest = basins.get(owner as usize).map(|item| item.destination);
                let descendant =
                    owner != OCEAN_LABEL && basin_is_descendant(basins, owner as usize, basin.id);
                let drains = owner != OCEAN_LABEL && basin_drains_to_ocean(basins, owner as usize);
                eprintln!(
                    "  neighbor={} row={} col={} elev={} owner={} owner_dest={:?} descendant={} drains_ocean={} sea={}",
                    neighbor,
                    nrow,
                    ncol,
                    field.elevations_mm[*neighbor],
                    owner,
                    owner_dest,
                    descendant,
                    drains,
                    field.elevations_mm[*neighbor] <= field.sea_level_mm,
                );
            }
            return Err(PhysicalError::coded(
                PhysicalErrorCode::HydrologyInvalidSink,
                "overflowing basin spill point has no adjacent destination",
            ));
        };
        let path = river_coordinates_from_cells(
            field,
            &[spill_cell, outlet_cell],
            destination,
            lake_cells,
            basins,
        )?;
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

fn river_coordinates_from_cells(
    field: &PhysicalField,
    cells: &[usize],
    destination: BasinDestination,
    lake_cells: &[bool],
    basins: &[Basin],
) -> Result<Vec<[i32; 2]>, PhysicalError> {
    let mut coordinates = cells
        .iter()
        .map(|cell| coordinate_for_cell(field.grid, *cell))
        .collect::<Vec<_>>();
    if cells.len() < 2 {
        return Ok(coordinates);
    }
    let from = cells[cells.len() - 2];
    let to = cells[cells.len() - 1];
    let should_snap = matches!(destination, BasinDestination::Ocean)
        || (matches!(destination, BasinDestination::Basin(_))
            && lake_cells.get(to).copied().unwrap_or(false));
    if !should_snap {
        return Ok(coordinates);
    }
    let threshold = match destination {
        BasinDestination::Basin(id) => basins
            .get(id)
            .map(|basin| basin.water_level_mm)
            .unwrap_or(field.sea_level_mm),
        _ => field.sea_level_mm,
    };
    let mut candidates = vec![to];
    candidates.extend(field.grid.topology().neighbors(from).iter().copied());
    let snapped = candidates.into_iter().find_map(|neighbor| {
        contours::interpolate_edge(field.grid, &field.elevations_mm, threshold, from, neighbor).ok()
    });
    if let Some(point) = snapped {
        if let Some(last) = coordinates.last_mut() {
            *last = point;
        }
    }
    Ok(coordinates)
}

fn polygon_for_cells(grid: Grid, cells: &[usize]) -> Result<Vec<Vec<[i32; 2]>>, PhysicalError> {
    let polygons = contours::polygons_for_sparse_cells(grid, cells)?;
    if polygons.is_empty() {
        Ok(Vec::new())
    } else if polygons.len() == 1 {
        Ok(polygons.into_iter().next().unwrap_or_default())
    } else {
        Ok(polygons.into_iter().flatten().collect())
    }
}

fn polygon_for_mask(grid: Grid, mask: &[bool]) -> Result<Vec<Vec<Vec<[i32; 2]>>>, PhysicalError> {
    contours::polygons_from_mask(grid, mask)
}

fn island_groups(grid: Grid, land_mask: &[bool]) -> (Vec<u32>, Vec<Vec<usize>>) {
    let topology = grid.topology();
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
            for neighbor in topology.neighbors(cell) {
                if land_mask[*neighbor] && island_id[*neighbor] == u32::MAX {
                    island_id[*neighbor] = id;
                    queue.push_back(*neighbor);
                }
            }
        }
        groups.push(cells);
    }
    (island_id, groups)
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
    derive_hydrology_with_forcing(
        field,
        climate,
        drainage,
        reference_water_inventory_m3,
        None,
        None,
    )
}

pub fn derive_hydrology_with_crust(
    field: &PhysicalField,
    climate: &ClimateField,
    drainage: &DrainageField,
    reference_water_inventory_m3: u64,
    crust_by_cell: Option<&[CrustType]>,
) -> Result<HydrologyField, PhysicalError> {
    derive_hydrology_with_forcing(
        field,
        climate,
        drainage,
        reference_water_inventory_m3,
        crust_by_cell,
        None,
    )
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ThermalExpansion {
    pub ppm_per_degree_c: u32,
    pub temperature_offset_centi_c: i32,
}

pub fn thermal_expansion_delta_m3(volume_m3: u64, expansion: ThermalExpansion) -> i64 {
    const MAX_THERMAL_EXPANSION_FRACTION_PPM: u64 = 50_000;
    let raw = (i128::from(volume_m3)
        * i128::from(expansion.temperature_offset_centi_c)
        * i128::from(expansion.ppm_per_degree_c))
        / 100_000_000;
    let bound = (u128::from(volume_m3) * u128::from(MAX_THERMAL_EXPANSION_FRACTION_PPM) / 1_000_000)
        .min(i128::from(i64::MAX) as u128) as i128;
    raw.clamp(-bound, bound) as i64
}

fn geometric_ocean_target(mass_m3: u64, expansion: Option<ThermalExpansion>) -> u64 {
    let Some(expansion) = expansion else {
        return mass_m3.max(1);
    };
    let delta = thermal_expansion_delta_m3(mass_m3, expansion);
    (i128::from(mass_m3) + i128::from(delta)).clamp(1, i128::from(u64::MAX)) as u64
}

pub fn derive_hydrology_with_forcing(
    field: &PhysicalField,
    climate: &ClimateField,
    drainage: &DrainageField,
    reference_water_inventory_m3: u64,
    crust_by_cell: Option<&[CrustType]>,
    thermal_expansion: Option<ThermalExpansion>,
) -> Result<HydrologyField, PhysicalError> {
    field.validate().map_err(PhysicalError::InvalidSource)?;
    climate.validate()?;
    if climate.grid != field.grid || drainage.grid != field.grid {
        return Err(PhysicalError::coded(
            PhysicalErrorCode::GeometryInvalid,
            "hydrology inputs do not match the physical grid",
        ));
    }
    let mut sea_level = field.sea_level_mm;
    let mut iterations = 0;
    let mut converged = false;
    let mut seen = Vec::<(i32, u64, u64)>::new();
    for iteration in 1..=MAX_FIXED_POINT_ITERATIONS {
        iterations = iteration;
        let mut work = field.clone();
        work.sea_level_mm = sea_level;
        let mut tree = build_depression_tree(&work, sea_level);
        let mut inland_water = solve_depression_water(&mut tree, climate)?;
        let ice = derive_land_ice(&work, climate, sea_level, reference_water_inventory_m3)?;
        let ice_cells = ice.0;
        let land_ice_m3 = ice.2;
        for basin in &mut tree.basins {
            if ice_cells[basin.minimum_cell] && basin.water_volume_m3 > 0 {
                inland_water = inland_water.saturating_sub(basin.water_volume_m3);
                basin.water_volume_m3 = 0;
                basin.water_level_mm = basin.minimum_elevation_mm;
                basin.outflow_m3_per_year = 0;
                if !matches!(basin.destination, BasinDestination::Endorheic) {
                    basin.status = BasinStatus::Dry;
                }
            }
        }
        inland_water = scale_inland_to_budget(
            &mut tree,
            inland_water,
            inland_budget(field, reference_water_inventory_m3, land_ice_m3),
        );
        let remaining = reference_water_inventory_m3
            .saturating_sub(land_ice_m3)
            .saturating_sub(inland_water)
            .max(1);
        let sea_target =
            solve_ocean_level_mm(field, geometric_ocean_target(remaining, thermal_expansion))?;
        let mixed = ((i64::from(sea_level)
            * i64::from(1_000_000 - WATER_SOLVER_UNDERRELAXATION_PPM)
            + i64::from(sea_target) * i64::from(WATER_SOLVER_UNDERRELAXATION_PPM))
            / 1_000_000) as i32;
        let state = (sea_level, inland_water, land_ice_m3);
        if seen.iter().any(|previous| previous.0 == mixed) {
            sea_level = sea_level.min(mixed);
            converged = true;
            break;
        }
        seen.push(state);
        let tolerance = tolerance_m3(field, sea_level);
        let ocean = ocean_volume(field, sea_level);
        let expansion = thermal_expansion
            .map(|expansion| thermal_expansion_delta_m3(remaining, expansion))
            .unwrap_or(0);
        let total = (i128::from(ocean) - i128::from(expansion)
            + i128::from(inland_water)
            + i128::from(land_ice_m3))
        .clamp(0, i128::from(u64::MAX)) as u64;
        if mixed == sea_level && reference_water_inventory_m3.abs_diff(total) <= tolerance {
            converged = true;
            break;
        }
        sea_level = mixed;
    }
    let mut work = field.clone();
    work.sea_level_mm = sea_level;
    let mut tree = build_depression_tree(&work, sea_level);
    let mut inland_water = solve_depression_water(&mut tree, climate)?;
    let ice = derive_land_ice(&work, climate, sea_level, reference_water_inventory_m3)?;
    let ice_cells = ice.0;
    let ice_thickness_mm = ice.1;
    let land_ice_m3 = ice.2;
    for basin in &mut tree.basins {
        if ice_cells[basin.minimum_cell] && basin.water_volume_m3 > 0 {
            inland_water = inland_water.saturating_sub(basin.water_volume_m3);
            basin.water_volume_m3 = 0;
            basin.water_level_mm = basin.minimum_elevation_mm;
            basin.outflow_m3_per_year = 0;
        }
    }
    inland_water = scale_inland_to_budget(
        &mut tree,
        inland_water,
        inland_budget(field, reference_water_inventory_m3, land_ice_m3),
    );
    let remaining = reference_water_inventory_m3
        .saturating_sub(land_ice_m3)
        .saturating_sub(inland_water)
        .max(1);
    sea_level = solve_ocean_level_mm(field, geometric_ocean_target(remaining, thermal_expansion))?;
    work.sea_level_mm = sea_level;
    let tolerance = tolerance_m3(field, sea_level);
    let ocean_water = ocean_volume(field, sea_level);
    let expansion = thermal_expansion
        .map(|expansion| thermal_expansion_delta_m3(remaining, expansion))
        .unwrap_or(0);
    let total_water = (i128::from(ocean_water) - i128::from(expansion)
        + i128::from(inland_water)
        + i128::from(land_ice_m3))
    .clamp(0, i128::from(u64::MAX)) as u64;
    let balance_error = reference_water_inventory_m3.abs_diff(total_water);
    if balance_error <= tolerance {
        converged = true;
    }
    if !converged || balance_error > tolerance {
        return Err(PhysicalError::coded(
            PhysicalErrorCode::WaterNonConvergent,
            "ocean and inland water coupled solve did not converge",
        ));
    }
    let basins = tree.basins.clone();
    let basin_by_cell = tree.basin_by_cell.clone();
    let mut lake_cells = vec![false; field.grid.sample_count()];
    let mut lake_level_mm = vec![sea_level; field.grid.sample_count()];
    for basin in &basins {
        if ice_cells[basin.minimum_cell]
            || basin.status == BasinStatus::Merged
            || basin.water_level_mm <= sea_level
            || basin.water_volume_m3 == 0
        {
            continue;
        }
        for cell in &tree.subtree_cells[basin.id] {
            if ice_cells[*cell] {
                continue;
            }
            if field.elevations_mm[*cell] < basin.water_level_mm
                && field.elevations_mm[*cell] > sea_level
            {
                lake_cells[*cell] = true;
                lake_level_mm[*cell] = basin.water_level_mm;
            }
        }
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
        derive_rivers(&work, drainage, &basins, &basin_by_cell, &lake_cells)?;
    let ocean_mask = (0..field.grid.sample_count())
        .map(|cell| field.elevations_mm[cell] <= sea_level)
        .collect::<Vec<_>>();
    let water_mask = ocean_mask
        .iter()
        .zip(lake_cells.iter())
        .map(|(ocean, lake)| *ocean || *lake)
        .collect::<Vec<_>>();
    let coastline_segments = water_boundary_segments(field, &water_mask)?;
    let mut lake_polygons = Vec::new();
    for basin in &basins {
        if ice_cells[basin.minimum_cell]
            || basin.status == BasinStatus::Merged
            || basin.water_volume_m3 == 0
            || basin
                .water_level_mm
                .saturating_sub(basin.minimum_elevation_mm)
                < MIN_LAKE_DEPTH_MM
            || basin.water_level_mm <= sea_level
        {
            continue;
        }
        let cells = tree.subtree_cells[basin.id]
            .iter()
            .copied()
            .filter(|cell| lake_cells[*cell])
            .collect::<Vec<_>>();
        let rings = polygon_for_cells(field.grid, &cells)?;
        if !rings.is_empty() {
            lake_polygons.push(rings);
        }
    }
    let (watershed_id, watershed_cells) = watershed_groups(field, drainage, &lake_cells);
    let mut watershed_polygons = Vec::new();
    for cells in &watershed_cells {
        watershed_polygons.push(polygon_for_cells(field.grid, cells)?);
    }
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
    let land_polygons = polygon_for_mask(field.grid, &land_mask)?;
    let ice_polygons = polygon_for_mask(field.grid, &ice_cells)?;
    let (island_id, island_cells) = island_groups(field.grid, &land_mask);
    let mut island_polygons = Vec::new();
    for cells in &island_cells {
        island_polygons.push(polygon_for_cells(field.grid, cells)?);
    }
    let ocean_polygons =
        contours::extract(field.grid, &field.elevations_mm, sea_level, false)?.polygons;
    let shelf_polygons = polygon_for_mask(field.grid, &shelf_cells)?;
    let bathymetry_contours = bathymetry_contour_segments(field, &ocean_mask, sea_level)?;
    let metrics = WaterBalanceMetrics {
        total_water_m3: reference_water_inventory_m3,
        ocean_water_m3: ocean_water,
        inland_water_m3: inland_water,
        land_ice_m3,
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
        ice_cells,
        ice_thickness_mm,
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
        ice_polygons,
        bathymetry_contours,
        metrics,
    };
    hydrology.validate_against(field, climate, drainage, reference_water_inventory_m3)?;
    Ok(hydrology)
}

fn geometry_string(coordinates: &[[i32; 2]]) -> String {
    let mut output = String::new();
    write_geometry(&mut output, coordinates);
    output
}

fn write_geometry(output: &mut String, coordinates: &[[i32; 2]]) {
    output.push('[');
    for (index, coordinate) in coordinates.iter().enumerate() {
        if index > 0 {
            output.push(',');
        }
        let _ = write!(
            output,
            "[{},{}]",
            coordinate[0] as f64 / 1_000_000.0,
            coordinate[1] as f64 / 1_000_000.0
        );
    }
    output.push(']');
}

fn multi_polygon_from_rings(polygons: &[Vec<[i32; 2]>]) -> String {
    let mut output = String::from("[");
    for (index, ring) in polygons.iter().enumerate() {
        if index > 0 {
            output.push(',');
        }
        output.push('[');
        write_geometry(&mut output, ring);
        output.push(']');
    }
    output.push(']');
    output
}

fn multi_polygon_from_polygons(polygons: &[Vec<Vec<[i32; 2]>>]) -> String {
    let mut output = String::with_capacity(polygons.len().saturating_mul(96).max(2));
    output.push('[');
    for (polygon_index, polygon) in polygons.iter().enumerate() {
        if polygon_index > 0 {
            output.push(',');
        }
        output.push('[');
        for (ring_index, ring) in polygon.iter().enumerate() {
            if ring_index > 0 {
                output.push(',');
            }
            write_geometry(&mut output, ring);
        }
        output.push(']');
    }
    output.push(']');
    output
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
    let mut output = String::with_capacity(8 * 1024 * 1024);
    output.push_str(r#"{"type":"FeatureCollection","features":["#);
    let mut feature_count = 0usize;
    let mut push_feature = |feature: String| {
        if feature_count > 0 {
            output.push(',');
        }
        output.push_str(&feature);
        feature_count += 1;
    };
    for (index, segment) in hydrology.coastline_segments.iter().enumerate() {
        push_feature(feature(
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
        ("ice", "Ice", &hydrology.ice_polygons),
    ] {
        if !polygons.is_empty() {
            push_feature(feature(
                id,
                id,
                if id == "land" || id == "islands" || id == "shelves" {
                    "land"
                } else {
                    "custom"
                },
                name,
                "MultiPolygon",
                &multi_polygon_from_polygons(polygons),
            ));
        }
    }
    for (index, segment) in hydrology.bathymetry_contours.iter().enumerate() {
        push_feature(feature(
            &format!("bathymetry-contour-{index:06}"),
            "bathymetric-contours",
            "route",
            "Bathymetric contour",
            "LineString",
            &geometry_string(&[segment.first, segment.second]),
        ));
    }
    for (index, polygons) in hydrology.lake_polygons.iter().enumerate() {
        push_feature(feature(
            &format!("lake-{index:06}"),
            "lakes",
            "lake",
            &format!("Lake {index}"),
            "MultiPolygon",
            &multi_polygon_from_rings(polygons),
        ));
    }
    for (index, polygons) in hydrology.watershed_polygons.iter().enumerate() {
        push_feature(feature(
            &format!("watershed-{index:06}"),
            "watersheds",
            "region",
            &format!("Watershed {index}"),
            "MultiPolygon",
            &multi_polygon_from_rings(polygons),
        ));
    }
    for (river, coordinates) in hydrology.rivers.iter().zip(&hydrology.river_coordinates) {
        push_feature(feature(
            &format!("river-{:06}", river.id),
            "rivers",
            "route",
            &format!("River order {}", river.strahler_order),
            "LineString",
            &geometry_string(coordinates),
        ));
    }
    if feature_count > MAX_HYDROLOGY_FEATURES {
        return Err("hydrology derived output exceeds the feature budget".into());
    }
    output.push_str("]}");
    if output.len() > crate::MAX_DERIVED_GEOJSON_BYTES {
        return Err(format!(
            "hydrology derived output exceeds the {} byte budget",
            crate::MAX_DERIVED_GEOJSON_BYTES
        ));
    }
    Ok(output)
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
    fn cumulative_volume_curve_stays_within_per_cell_rounding_error() {
        let grid = Grid::new(8, 6, crate::DEFAULT_RADIUS_METRES).unwrap();
        let mut elevations = vec![5_000; grid.sample_count()];
        let cells = [
            grid.index(1, 1),
            grid.index(1, 2),
            grid.index(2, 2),
            grid.index(3, 3),
            grid.index(4, 3),
        ];
        for (index, cell) in cells.iter().enumerate() {
            elevations[*cell] = -2_000 + index as i32 * 700;
        }
        let curve = VolumeCurve::from_cells(grid, &elevations, &cells, 2_000);

        for (level, _, volume) in &curve.knots {
            let per_cell = cells
                .iter()
                .filter(|cell| elevations[**cell] < *level)
                .map(|cell| volume_for_depth(grid, *cell, level.saturating_sub(elevations[*cell])))
                .sum::<u64>();
            assert!(volume.abs_diff(per_cell) <= cells.len() as u64);
        }
    }

    #[test]
    fn water_balance_and_geometry_are_bounded() {
        let hydrology = fixture();
        assert!(hydrology.metrics.converged);
        assert!(hydrology.metrics.balance_error_m3 <= hydrology.metrics.tolerance_m3);
        assert!(
            hydrology.metrics.total_water_m3.abs_diff(
                hydrology.metrics.ocean_water_m3
                    + hydrology.metrics.inland_water_m3
                    + hydrology.metrics.land_ice_m3,
            ) <= hydrology.metrics.tolerance_m3
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
            first.metrics.total_water_m3.abs_diff(
                first.metrics.ocean_water_m3
                    + first.metrics.inland_water_m3
                    + first.metrics.land_ice_m3
            ) <= first.metrics.tolerance_m3
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
        assert!(json.contains("\"ice\"") || hydrology.ice_polygons.is_empty());
        for polygon in hydrology
            .land_polygons
            .iter()
            .chain(hydrology.ocean_polygons.iter())
            .chain(hydrology.island_polygons.iter())
            .chain(hydrology.lake_polygons.iter())
        {
            for ring in polygon {
                assert!(ring.len() >= 4);
                assert_eq!(ring.first(), ring.last());
                for point in ring {
                    assert!((-180_000_000..=180_000_000).contains(&point[0]));
                    assert!((-90_000_000..=90_000_000).contains(&point[1]));
                }
            }
        }
        assert_eq!(hydrology.derivation_version, HYDROLOGY_DERIVATION_VERSION);
    }

    #[test]
    fn land_ice_requires_freezing_and_moisture() {
        assert_eq!(ice_water_equivalent_mm(0, 400), 0);
        assert_eq!(ice_water_equivalent_mm(-1_200, 40), 0);
        assert!(ice_water_equivalent_mm(-1_200, 400) > 0);
        let hydrology = fixture();
        assert_eq!(hydrology.ice_cells.len(), hydrology.ice_thickness_mm.len());
        assert!(hydrology
            .ice_cells
            .iter()
            .zip(&hydrology.ice_thickness_mm)
            .all(|(ice, thickness)| (*ice && *thickness > 0) || (!*ice && *thickness == 0)));
    }

    fn synthetic_field(
        width: u32,
        height: u32,
        elevations: Vec<i32>,
        sea_level_mm: i32,
    ) -> PhysicalField {
        let grid = Grid::new(width, height, crate::DEFAULT_RADIUS_METRES).unwrap();
        PhysicalField {
            grid,
            seed: 7,
            retry_index: 0,
            target_land_fraction_ppm: 300_000,
            sea_level_mm,
            elevations_mm: elevations,
        }
    }

    fn synthetic_climate(field: &PhysicalField, runoff_mm: u32) -> crate::climate::ClimateField {
        let count = field.grid.sample_count();
        let mut runoff_mm_per_year = vec![0u32; count];
        let mut runoff_volume_m3_per_year = vec![0u64; count];
        let mut precipitation_mm_per_year = vec![0u32; count];
        for cell in 0..count {
            if field.elevations_mm[cell] <= field.sea_level_mm {
                continue;
            }
            runoff_mm_per_year[cell] = runoff_mm;
            precipitation_mm_per_year[cell] = runoff_mm.saturating_add(200);
            runoff_volume_m3_per_year[cell] = precipitation_volume(field.grid, cell, runoff_mm);
        }
        crate::climate::ClimateField {
            grid: field.grid,
            derivation_version: crate::climate::CLIMATE_DERIVATION_VERSION,
            planetary: crate::planetary::PlanetaryConfiguration::earth_like(),
            temperature_centi_c: vec![1_200; count],
            temperature_nh_summer_centi_c: vec![1_200; count],
            temperature_nh_winter_centi_c: vec![1_200; count],
            moisture_mm_per_year: precipitation_mm_per_year.clone(),
            precipitation_mm_per_year,
            runoff_mm_per_year,
            runoff_volume_m3_per_year,
            maritime_factor_ppm: vec![400_000; count],
            metrics: crate::climate::ClimateMetrics {
                precipitation_volume_m3_per_year: 0,
                runoff_volume_m3_per_year: 0,
                mean_temperature_centi_c: 1_200,
                minimum_temperature_centi_c: 1_200,
                maximum_temperature_centi_c: 1_200,
                mean_precipitation_mm_per_year: 0,
                mean_runoff_mm_per_year: 0,
                wettest_cell_precipitation_mm_per_year: 0,
                driest_land_cell_precipitation_mm_per_year: 0,
                transport_iterations: 0,
                mean_seasonal_range_centi_c: 0,
                minimum_seasonal_temperature_centi_c: 0,
                maximum_seasonal_temperature_centi_c: 0,
                permanently_frozen_land_ppm: 0,
                seasonally_frozen_land_ppm: 0,
            },
        }
    }

    #[test]
    fn nearest_sea_cell_walks_ocean_labeled_land() {
        let width = 8;
        let height = 6;
        let grid = Grid::new(width, height, crate::DEFAULT_RADIUS_METRES).unwrap();
        let mut elevations = vec![400; grid.sample_count()];
        for col in 0..width {
            elevations[grid.index(0, col)] = -200;
        }
        elevations[grid.index(2, 3)] = 80;
        elevations[grid.index(1, 3)] = 60;
        let field = synthetic_field(width, height, elevations, 0);
        let mut basin_by_cell = vec![0; field.grid.sample_count()];
        basin_by_cell[grid.index(2, 3)] = OCEAN_LABEL;
        basin_by_cell[grid.index(1, 3)] = OCEAN_LABEL;
        let start = grid.index(2, 3);
        let sea = nearest_sea_cell(&field, &basin_by_cell, start).unwrap();
        assert!(field.elevations_mm[sea] <= field.sea_level_mm);
        assert_eq!(sea, grid.index(0, 3));
    }

    #[test]
    fn nested_bowls_form_a_parent_child_depression_tree() {
        let grid = Grid::new(12, 8, crate::DEFAULT_RADIUS_METRES).unwrap();
        let mut elevations = vec![-4_000; grid.sample_count()];
        for row in 2..6 {
            for col in 2..10 {
                elevations[grid.index(row, col)] = 800;
            }
        }
        for row in 3..5 {
            for col in 3..7 {
                elevations[grid.index(row, col)] = 400;
            }
        }
        elevations[grid.index(4, 4)] = 80;
        elevations[grid.index(4, 8)] = 120;
        let field = synthetic_field(12, 8, elevations, 0);
        let tree = build_depression_tree(&field, 0);
        assert!(
            tree.basins.iter().any(|basin| !basin.children.is_empty())
                || tree.basins.iter().any(|basin| basin.parent_basin.is_some())
        );
        let climate = synthetic_climate(&field, 400);
        let mut wet = tree;
        let _inland = solve_depression_water(&mut wet, &climate).unwrap();
        assert!(wet.basins.iter().any(|basin| basin.outflow_m3_per_year > 0
            || basin.water_volume_m3 > 0
            || basin.parent_basin.is_some()
            || !basin.children.is_empty()));
    }

    #[test]
    fn chained_overflow_and_endorheic_basins_are_represented() {
        let hydrology = fixture();
        assert!(hydrology.basins.iter().all(|basin| {
            basin.parent_basin != Some(basin.id)
                && basin
                    .children
                    .iter()
                    .all(|child| hydrology.basins[*child].parent_basin == Some(basin.id))
        }));
        let endorheic = hydrology
            .basins
            .iter()
            .any(|basin| basin.destination == BasinDestination::Endorheic);
        let overflowing = hydrology
            .basins
            .iter()
            .any(|basin| basin.status == BasinStatus::Overflowing || basin.outflow_m3_per_year > 0);
        let ocean = hydrology
            .basins
            .iter()
            .any(|basin| basin.destination == BasinDestination::Ocean);
        assert!(endorheic || overflowing || ocean || hydrology.basins.is_empty());
    }

    fn empty_drainage(field: &PhysicalField) -> crate::evolution::DrainageField {
        let count = field.grid.sample_count();
        crate::evolution::DrainageField {
            grid: field.grid,
            derivation_version: crate::evolution::EVOLUTION_DERIVATION_VERSION,
            routing_elevation_mm: field.elevations_mm.clone(),
            fill_depth_mm: vec![0; count],
            routing_order: (0..count as u32).collect(),
            slope_ppm: vec![0; count],
            accumulation_m3_per_year: vec![0; count],
            edges: Vec::new(),
            outlet_cells: Vec::new(),
            metrics: crate::evolution::DrainageMetrics {
                direct_runoff_m3_per_year: 1,
                routed_runoff_m3_per_year: 0,
                routed_edge_count: 0,
                drainage_density_ppm: 0,
                grid_anisotropy_ppm: 0,
                convergence_ppm: 0,
                outlet_count: 0,
                routing_surface_raise_max_mm: 0,
            },
        }
    }

    #[test]
    fn endorheic_parent_is_marked_ocean_when_it_later_reaches_the_sea() {
        let width = 12;
        let height = 12;
        let grid = Grid::new(width, height, crate::DEFAULT_RADIUS_METRES).unwrap();
        let mut elevations = vec![-500; grid.sample_count()];
        for row in 3..10 {
            for col in 2..11 {
                elevations[grid.index(row, col)] = 800;
            }
        }
        elevations[grid.index(6, 4)] = 40;
        elevations[grid.index(6, 6)] = 50;
        elevations[grid.index(6, 5)] = 200;
        elevations[grid.index(5, 5)] = 250;
        elevations[grid.index(4, 5)] = 300;
        elevations[grid.index(3, 5)] = 350;
        let field = synthetic_field(width, height, elevations, 0);
        let tree = build_depression_tree(&field, field.sea_level_mm);
        let parent = tree
            .basins
            .iter()
            .find(|basin| basin.children.len() >= 2)
            .expect("two inland pits should merge into a parent");
        assert_eq!(parent.destination, BasinDestination::Ocean);
        assert!(parent.spill_cell.is_some());
    }

    #[test]
    fn ocean_spill_may_enter_an_already_drained_catchment() {
        let width = 12;
        let height = 12;
        let grid = Grid::new(width, height, crate::DEFAULT_RADIUS_METRES).unwrap();
        let mut elevations = vec![-500; grid.sample_count()];
        for row in 3..10 {
            for col in 2..11 {
                elevations[grid.index(row, col)] = 800;
            }
        }
        elevations[grid.index(4, 3)] = 40;
        elevations[grid.index(4, 4)] = 90;
        elevations[grid.index(4, 5)] = 120;
        elevations[grid.index(4, 6)] = 150;
        elevations[grid.index(5, 6)] = 180;
        elevations[grid.index(3, 3)] = 200;
        elevations[grid.index(6, 5)] = 50;
        elevations[grid.index(6, 4)] = 140;
        elevations[grid.index(7, 5)] = 140;
        elevations[grid.index(6, 7)] = 55;
        elevations[grid.index(6, 8)] = 140;
        elevations[grid.index(7, 7)] = 140;
        elevations[grid.index(6, 6)] = 350;
        let field = synthetic_field(width, height, elevations, 0);
        let mut tree = build_depression_tree(&field, field.sea_level_mm);
        let climate = synthetic_climate(&field, 800);
        solve_depression_water(&mut tree, &climate).unwrap();
        assert!(
            tree.basins.iter().any(|basin| {
                basin.status == BasinStatus::Overflowing
                    && matches!(basin.destination, BasinDestination::Ocean)
                    && basin.outflow_m3_per_year > 0
                    && basin.spill_cell == Some(field.grid.index(6, 6))
            }),
            "expected an overflowing parent whose spill is the inland ocean-touching saddle"
        );
        let drainage = empty_drainage(&field);
        let lake_cells = vec![false; field.grid.sample_count()];
        let (rivers, _) = derive_rivers(
            &field,
            &drainage,
            &tree.basins,
            &tree.basin_by_cell,
            &lake_cells,
        )
        .expect("spill outlet should accept an already ocean-drained neighbor");
        assert!(rivers.iter().any(|river| {
            river.spill_outlet
                && river.source_cell == field.grid.index(6, 6)
                && river.destination == BasinDestination::Ocean
        }));
    }

    #[test]
    fn coastal_capture_removes_a_lake_when_sea_level_rises() {
        let grid = Grid::new(10, 6, crate::DEFAULT_RADIUS_METRES).unwrap();
        let mut elevations = vec![-2_000; grid.sample_count()];
        for row in 1..5 {
            for col in 2..8 {
                elevations[grid.index(row, col)] = 600;
            }
        }
        elevations[grid.index(3, 4)] = 40;
        elevations[grid.index(3, 7)] = 80;
        let low = synthetic_field(10, 6, elevations.clone(), -1_500);
        let high = synthetic_field(10, 6, elevations, 100);
        let low_tree = build_depression_tree(&low, low.sea_level_mm);
        let high_tree = build_depression_tree(&high, high.sea_level_mm);
        assert!(low_tree.basins.len() >= high_tree.basins.len());
    }
}
