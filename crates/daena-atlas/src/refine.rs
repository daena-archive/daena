//! Derived drainage: bounded pit fill, watershed-constrained continuous
//! flow, atlas-only tributaries, climate-state erosion, and epoch coastline
//! on the structure lattice.

use std::cmp::Reverse;
use std::collections::{BTreeMap, BTreeSet, BinaryHeap};

use daena_physical::hydrology::{BasinStatus, HydrologyField};
use rayon::prelude::*;

use crate::amplify::{apply_coastline, AmplificationModel, MountainKind};
use crate::constraint::AtlasConstraint;
use crate::control::ControlFields;
use crate::detail::{
    cell_center_lat_micro, cell_center_lon_micro, domain_key, lattice_lat_micro, lattice_lon_micro,
    lattice_nearest_cells, lattice_sample, nearest_cell, nest_lattice_coord, sample_sdf_ppm,
    COASTAL_ENVELOPE_PPM,
};
use crate::erosion::{
    apply_scale_erosion, fluvial_gain_ppm, freeze_thaw_ppm, glacial_work_ppm, lattice_index,
    lock_polar_rows, neighbor_at, vegetation_resistance_ppm, ScaleErosion, DIRS, EROSION_SCALES,
    BOUNDED_SEDIMENT_DOMAIN, FAN_SLOPE_PPM, FLOODPLAIN_SLOPE_PPM, MAX_EROSION_STEP_MM,
    MULTI_SCALE_EROSION_DOMAIN, NO_FLOW,
};

use crate::request::DetailLevel;
use crate::{AtlasError, ATLAS_DERIVED_DRAINAGE_VERSION, ATLAS_DETAIL_ALGORITHM_VERSION};

pub const REFINED_DRAINAGE_DOMAIN: &str = "refined-drainage";
pub const MAX_TRIBUTARIES: usize = 256;
pub const MAX_VALLEYS: usize = 64;
pub const MAX_DEPOSITION_FEATURES: usize = 64;
pub const MAX_FILL_MM: i32 = 4_800;
const CANCELLATION_STRIDE: usize = 4_096;
const MAX_TRACE_STEPS: usize = 8_192;

fn lattice_center(i: u32, j: u32, width: u32, height: u32) -> [i32; 2] {
    [
        cell_center_lon_micro(i, width),
        cell_center_lat_micro(j, height),
    ]
}

fn max_trace_steps(width: u32, height: u32) -> usize {
    (width as usize)
        .saturating_add(height as usize)
        .saturating_mul(2)
        .max(64)
}
const REFINE_WORKING_BUFFERS: usize = 4;
const REFINE_BUDGET_BYTES: usize = 96 * 1024 * 1024;

#[must_use]
pub fn refined_lattice_exceeds_budget(count: usize) -> bool {
    count
        .saturating_mul(std::mem::size_of::<i32>())
        .saturating_mul(REFINE_WORKING_BUFFERS)
        > REFINE_BUDGET_BYTES
}
const OCEAN: u32 = u32::MAX;
const NO_PARENT_RIVER: u32 = u32::MAX;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RefinedTributary {
    pub id: String,
    pub source_index: usize,
    pub join_index: usize,
    pub parent_river_id: u32,
    pub ordinal: u32,
    pub watershed_id: u32,
    pub width_mm: u32,
    pub depth_mm: u32,
    pub path: Vec<[i32; 2]>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RefinedValley {
    pub id: String,
    pub lattice_index: usize,
    pub watershed_id: u32,
    pub lon_micro: i32,
    pub lat_micro: i32,
}

impl RefinedValley {
    #[must_use]
    pub fn id_for(lattice_index: usize) -> String {
        format!("atlas:valley:v{ATLAS_DERIVED_DRAINAGE_VERSION}:{lattice_index}")
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DepositionKind {
    Fan,
    Floodplain,
    Delta,
}

impl DepositionKind {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Fan => "fan",
            Self::Floodplain => "floodplain",
            Self::Delta => "delta",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DepositionFeature {
    pub id: String,
    pub kind: DepositionKind,
    pub lattice_index: usize,
    pub watershed_id: u32,
    pub lon_micro: i32,
    pub lat_micro: i32,
}

impl DepositionFeature {
    #[must_use]
    pub fn id_for(kind: DepositionKind, lattice_index: usize) -> String {
        format!(
            "atlas:deposition:v{ATLAS_DERIVED_DRAINAGE_VERSION}:{}:{lattice_index}",
            kind.as_str()
        )
    }
}

#[derive(Debug, Clone)]
pub struct RefinedHydrology {
    pub version: u32,
    pub lattice_width: u32,
    pub lattice_height: u32,
    pub source_mm: Vec<i32>,
    pub filled_mm: Vec<i32>,
    pub worked_mm: Vec<i32>,
    pub flow_primary: Vec<u32>,
    pub flow_secondary: Vec<u32>,
    pub primary_weight_ppm: Vec<u32>,
    pub accumulation: Vec<u32>,
    pub protected: Vec<bool>,
    pub filled_pit_count: u32,
    pub tributaries: Vec<RefinedTributary>,
    pub valleys: Vec<RefinedValley>,
    pub deposition: Vec<DepositionFeature>,
    pub drainage_key: [u8; 32],
    pub erosion_key: [u8; 32],
    pub sediment_key: [u8; 32],
    pub sediment_mm: Vec<i32>,
}

impl RefinedTributary {
    #[must_use]
    pub fn id_for(parent_river_id: u32, ordinal: u32) -> String {
        format!("atlas:tributary:v{ATLAS_DERIVED_DRAINAGE_VERSION}:{parent_river_id}:{ordinal}")
    }
}

fn channel_discharge(accumulation: u32, runoff_mm: i32) -> u32 {
    accumulation.saturating_mul(runoff_mm.clamp(0, 8_000) as u32)
}

fn channel_width_mm(accumulation: u32, runoff_mm: i32) -> u32 {
    400u32
        .saturating_add(channel_discharge(accumulation, runoff_mm) / 16)
        .min(24_000)
}

fn channel_depth_mm(width_mm: u32) -> u32 {
    (width_mm / 8).clamp(80, 3_000)
}

fn wrap_lon_micro(lon: i64) -> i32 {
    let mut x = lon % 360_000_000;
    if x <= -180_000_000 {
        x += 360_000_000;
    } else if x > 180_000_000 {
        x -= 360_000_000;
    }
    x as i32
}

fn displace_meander(
    path: &mut [[i32; 2]],
    lattice_width: u32,
    lattice_height: u32,
    drainage_key: &[u8; 32],
    discharge: u32,
) {
    if path.len() < 3 {
        return;
    }
    let original = path.to_vec();
    let amp = i64::from((discharge / 32).clamp(1, 1_200)) * 80;
    for i in 1..original.len() - 1 {
        let prev = original[i - 1];
        let next = original[i + 1];
        let mut dlon = i64::from(next[0]) - i64::from(prev[0]);
        if dlon > 180_000_000 {
            dlon -= 360_000_000;
        } else if dlon < -180_000_000 {
            dlon += 360_000_000;
        }
        let dlat = i64::from(next[1]) - i64::from(prev[1]);
        let px = -dlat;
        let py = dlon;
        let scale = px.abs().max(py.abs()).max(1);
        let cell_i = ((i64::from(original[i][0]) + 180_000_000) * i64::from(lattice_width)
            / 360_000_000)
            .clamp(0, i64::from(lattice_width) - 1) as u32;
        let cell_j = ((i64::from(original[i][1]) + 90_000_000) * i64::from(lattice_height)
            / 180_000_000)
            .clamp(0, i64::from(lattice_height) - 1) as u32;
        let prf = lattice_sample(drainage_key, cell_i, cell_j, 2);
        let unit = ((prf >> 11) % 2_000_001) as i64 - 1_000_000;
        let off = unit * amp / 1_000_000;
        path[i][0] = wrap_lon_micro(i64::from(original[i][0]) + px * off / scale);
        path[i][1] =
            (i64::from(original[i][1]) + py * off / scale).clamp(-90_000_000, 90_000_000) as i32;
    }
}

fn dist_ppm(dir: (i32, i32)) -> i32 {
    if dir.0 == 0 || dir.1 == 0 {
        1_000
    } else {
        1_414
    }
}

fn is_wet_basin(hydrology: &HydrologyField, cell: usize) -> bool {
    let basin = hydrology.basin_by_cell.get(cell).copied().unwrap_or(OCEAN);
    if basin == OCEAN {
        return false;
    }
    hydrology.basins.get(basin as usize).is_some_and(|record| {
        matches!(
            record.status,
            BasinStatus::Endorheic | BasinStatus::Active | BasinStatus::Overflowing
        ) || record.water_volume_m3 > 0
    })
}

fn is_protected(hydrology: &HydrologyField, cell: usize) -> bool {
    hydrology.lake_cells.get(cell).copied() == Some(true)
        || hydrology
            .lake_level_mm
            .get(cell)
            .copied()
            .unwrap_or(i32::MIN)
            > hydrology.sea_level_mm
        || is_wet_basin(hydrology, cell)
}

fn river_occupancy(hydrology: &HydrologyField) -> (BTreeSet<usize>, Vec<u32>) {
    let count = hydrology.grid.sample_count();
    let mut cells = BTreeSet::new();
    let mut river_at = vec![NO_PARENT_RIVER; count];
    for (index, path) in hydrology.river_coordinates.iter().enumerate() {
        let source = hydrology
            .rivers
            .get(index)
            .map_or(NO_PARENT_RIVER, |river| river.source_cell as u32);
        for point in path {
            let cell = nearest_cell(hydrology.grid, point[0], point[1]);
            cells.insert(cell);
            if river_at[cell] == NO_PARENT_RIVER || source < river_at[cell] {
                river_at[cell] = source;
            }
        }
    }
    (cells, river_at)
}

fn mouth_by_watershed(hydrology: &HydrologyField) -> Vec<usize> {
    let count = hydrology.grid.sample_count();
    let mut mouth = vec![usize::MAX; count];
    let mut by_watershed = vec![usize::MAX; count];
    for river in &hydrology.rivers {
        let watershed = hydrology
            .watershed_id
            .get(river.mouth_cell)
            .copied()
            .unwrap_or(OCEAN);
        if watershed == OCEAN || river.mouth_cell >= count {
            continue;
        }
        let slot = watershed as usize;
        if slot >= by_watershed.len() {
            by_watershed.resize(slot + 1, usize::MAX);
        }
        by_watershed[slot] = by_watershed[slot].min(river.mouth_cell);
    }
    for (cell, watershed) in hydrology.watershed_id.iter().enumerate() {
        if *watershed == OCEAN {
            continue;
        }
        let slot = *watershed as usize;
        if slot < by_watershed.len() {
            mouth[cell] = by_watershed[slot];
        }
    }
    mouth
}

fn canonical_is_coastal(hydrology: &HydrologyField, elevations_mm: &[i32], cell: usize) -> bool {
    if elevations_mm.get(cell).copied().unwrap_or(i32::MIN) < hydrology.sea_level_mm {
        return true;
    }
    hydrology.grid.neighbors(cell).into_iter().any(|neighbor| {
        elevations_mm.get(neighbor).copied().unwrap_or(i32::MIN) < hydrology.sea_level_mm
            || hydrology
                .watershed_id
                .get(neighbor)
                .copied()
                .unwrap_or(OCEAN)
                == OCEAN
    })
}

fn accum_threshold(level: DetailLevel) -> u32 {
    match level {
        DetailLevel::Standard => 16,
        DetailLevel::Detailed => 32,
        DetailLevel::Print => 64,
    }
}

fn build_source_surface(
    model: &AmplificationModel,
    sea_level_mm: i32,
    sdf: &[i32],
    check_cancelled: &mut dyn FnMut() -> Result<(), AtlasError>,
) -> Result<Vec<i32>, AtlasError> {
    let width = model.detail.lattice_width;
    let height = model.detail.lattice_height;
    let width_us = width as usize;
    let mut source = vec![0_i32; width_us * height as usize];
    check_cancelled()?;
    source.par_iter_mut().enumerate().for_each(|(index, slot)| {
        let i = (index % width_us) as u32;
        let j = (index / width_us) as u32;
        let lon = lattice_lon_micro(i, width);
        let lat = lattice_lat_micro(j, height);
        let sdf_ppm = sample_sdf_ppm(model.detail.grid, sdf, lon, lat);
        *slot = model.detail.refined_at(lon, lat, sea_level_mm, sdf_ppm);
    });
    check_cancelled()?;
    lock_polar_rows(width, height, &mut source);
    Ok(source)
}

fn priority_fill(
    width: u32,
    height: u32,
    source_mm: &[i32],
    protected: &[bool],
    sea_level_mm: i32,
    check_cancelled: &mut dyn FnMut() -> Result<(), AtlasError>,
) -> Result<(Vec<i32>, u32), AtlasError> {
    let count = source_mm.len();
    let mut routing = source_mm.to_vec();
    let mut visited = vec![0_u8; count];
    let mut seeds = Vec::with_capacity(count / 2);
    for index in 0..count {
        if index.is_multiple_of(CANCELLATION_STRIDE) {
            check_cancelled()?;
        }
        if source_mm[index] < sea_level_mm || protected[index] {
            visited[index] = 1;
            seeds.push(Reverse((source_mm[index], index as u32)));
        }
    }
    if seeds.is_empty() {
        return Err(AtlasError::limit(
            "pit fill has no ocean or protected outlet",
        ));
    }
    let mut queue = BinaryHeap::from(seeds);
    let mut pops = 0_usize;
    while let Some(Reverse((level, index_u))) = queue.pop() {
        pops += 1;
        if pops.is_multiple_of(CANCELLATION_STRIDE) {
            check_cancelled()?;
        }
        let j = index_u / width;
        let i = index_u % width;
        for dir in DIRS {
            let Some((_, nj, neighbor)) = neighbor_at(width, height, i, j, dir) else {
                continue;
            };
            if visited[neighbor] != 0 {
                continue;
            }
            visited[neighbor] = 1;
            let polar = nj == 0 || nj + 1 == height;
            if !protected[neighbor] && !polar {
                let raised = routing[neighbor].max(level);
                let capped = source_mm[neighbor].saturating_add(MAX_FILL_MM);
                routing[neighbor] = raised.min(capped);
            }
            queue.push(Reverse((routing[neighbor], neighbor as u32)));
        }
    }
    lock_polar_rows(width, height, &mut routing);
    for index in 0..count {
        if protected[index] {
            routing[index] = source_mm[index];
        }
    }
    let mut filled_pit_count = 0_u32;
    for index in 0..count {
        if !protected[index] && routing[index] > source_mm[index] {
            filled_pit_count = filled_pit_count.saturating_add(1);
        }
    }
    Ok((routing, filled_pit_count))
}

#[allow(clippy::too_many_arguments)]
fn eligible_flow_neighbor(
    hydrology: &HydrologyField,
    elevations_mm: &[i32],
    mouths: &[usize],
    from_watershed: u32,
    from_canonical: usize,
    neighbor_elev: i32,
    neighbor_canonical: usize,
    sea_level_mm: i32,
) -> bool {
    if neighbor_elev < sea_level_mm {
        return from_watershed != OCEAN
            && (canonical_is_coastal(hydrology, elevations_mm, from_canonical)
                || mouths.get(from_canonical).copied() == Some(from_canonical)
                || mouths.get(from_canonical).copied() == Some(neighbor_canonical));
    }
    hydrology.watershed_id.get(neighbor_canonical).copied() == Some(from_watershed)
        && from_watershed != OCEAN
}

#[allow(clippy::too_many_arguments, clippy::type_complexity)]
fn assign_flow(
    width: u32,
    height: u32,
    filled_mm: &[i32],
    hydrology: &HydrologyField,
    elevations_mm: &[i32],
    mouths: &[usize],
    cells: &[usize],
    sea_level_mm: i32,
    check_cancelled: &mut dyn FnMut() -> Result<(), AtlasError>,
) -> Result<(Vec<u32>, Vec<u32>, Vec<u32>), AtlasError> {
    let count = filled_mm.len();
    let mut primary = vec![NO_FLOW; count];
    let mut secondary = vec![NO_FLOW; count];
    let mut weight = vec![0_u32; count];
    check_cancelled()?;
    let width_us = width as usize;
    primary
        .par_iter_mut()
        .zip(secondary.par_iter_mut())
        .zip(weight.par_iter_mut())
        .enumerate()
        .for_each(|(index, ((prim, sec), wgt))| {
            if filled_mm[index] < sea_level_mm {
                return;
            }
            let i = (index % width_us) as u32;
            let j = (index / width_us) as u32;
            let canonical = cells[index];
            let watershed = hydrology
                .watershed_id
                .get(canonical)
                .copied()
                .unwrap_or(OCEAN);
            if watershed == OCEAN {
                return;
            }
            let mut best_dir = 8_usize;
            let mut best_slope = 0_i64;
            let mut best_neighbor = NO_FLOW;
            let mut candidates = [None; 8];
            for (dir_index, dir) in DIRS.iter().copied().enumerate() {
                let Some((_, _, neighbor)) = neighbor_at(width, height, i, j, dir) else {
                    continue;
                };
                let n_canonical = cells[neighbor];
                if !eligible_flow_neighbor(
                    hydrology,
                    elevations_mm,
                    mouths,
                    watershed,
                    canonical,
                    filled_mm[neighbor],
                    n_canonical,
                    sea_level_mm,
                ) {
                    continue;
                }
                let slope = ((i64::from(filled_mm[index]) - i64::from(filled_mm[neighbor]))
                    * 1_000)
                    / i64::from(dist_ppm(dir));
                candidates[dir_index] = Some((neighbor as u32, slope));
                if (slope > best_slope
                    || (slope == best_slope && (neighbor as u32) < best_neighbor))
                    && slope > 0
                {
                    best_slope = slope;
                    best_dir = dir_index;
                    best_neighbor = neighbor as u32;
                }
            }
            if best_neighbor == NO_FLOW {
                let mut fallback = NO_FLOW;
                let mut fallback_elev = i32::MAX;
                for dir in DIRS {
                    let Some((_, _, neighbor)) = neighbor_at(width, height, i, j, dir) else {
                        continue;
                    };
                    let n_canonical = cells[neighbor];
                    if hydrology.watershed_id.get(n_canonical).copied() != Some(watershed) {
                        continue;
                    }
                    if filled_mm[neighbor] < fallback_elev
                        || (filled_mm[neighbor] == fallback_elev && (neighbor as u32) < fallback)
                    {
                        fallback_elev = filled_mm[neighbor];
                        fallback = neighbor as u32;
                    }
                }
                if fallback != NO_FLOW && fallback != index as u32 {
                    *prim = fallback;
                    *wgt = 1_000_000;
                }
                return;
            }
            *prim = best_neighbor;
            let left = (best_dir + 7) % 8;
            let right = (best_dir + 1) % 8;
            let side = [candidates[left], candidates[right]]
                .into_iter()
                .flatten()
                .filter(|(_, slope)| *slope > 0)
                .max_by_key(|(neighbor, slope)| (*slope, Reverse(*neighbor)));
            if let Some((neighbor, slope)) = side {
                let total = best_slope + slope;
                if total > 0 {
                    *prim = best_neighbor;
                    *sec = neighbor;
                    *wgt = ((best_slope * 1_000_000) / total) as u32;
                    return;
                }
            }
            *wgt = 1_000_000;
        });
    check_cancelled()?;
    Ok((primary, secondary, weight))
}

fn accumulate(
    filled_mm: &[i32],
    primary: &[u32],
    secondary: &[u32],
    weight: &[u32],
    sea_level_mm: i32,
    check_cancelled: &mut dyn FnMut() -> Result<(), AtlasError>,
) -> Result<Vec<u32>, AtlasError> {
    let count = filled_mm.len();
    let mut order = (0..count).collect::<Vec<_>>();
    order.par_sort_unstable_by_key(|index| (Reverse(filled_mm[*index]), *index));
    let mut accum = vec![1_u32; count];
    for (rank, index) in order.iter().copied().enumerate() {
        if rank.is_multiple_of(CANCELLATION_STRIDE) {
            check_cancelled()?;
        }
        if filled_mm[index] < sea_level_mm {
            continue;
        }
        let value = accum[index];
        let dest = primary[index];
        if dest != NO_FLOW && (dest as usize) < count {
            let share = ((u64::from(value) * u64::from(weight[index])) / 1_000_000) as u32;
            accum[dest as usize] = accum[dest as usize].saturating_add(share);
            let rest = value.saturating_sub(share);
            if secondary[index] != NO_FLOW && (secondary[index] as usize) < count && rest > 0 {
                accum[secondary[index] as usize] =
                    accum[secondary[index] as usize].saturating_add(rest);
            } else if rest > 0 {
                accum[dest as usize] = accum[dest as usize].saturating_add(rest);
            }
        }
    }
    Ok(accum)
}

#[allow(clippy::too_many_arguments)]
fn extract_tributaries(
    width: u32,
    height: u32,
    filled_mm: &[i32],
    primary: &[u32],
    accumulation: &[u32],
    hydrology: &HydrologyField,
    river_cells: &BTreeSet<usize>,
    river_at: &[u32],
    runoff_mm: &[i32],
    sea_level_mm: i32,
    level: DetailLevel,
    drainage_key: &[u8; 32],
    check_cancelled: &mut dyn FnMut() -> Result<(), AtlasError>,
) -> Result<Vec<RefinedTributary>, AtlasError> {
    let threshold = accum_threshold(level);
    let count = filled_mm.len();
    let mut channel = vec![0_u8; count];
    check_cancelled()?;
    channel
        .par_iter_mut()
        .enumerate()
        .for_each(|(index, slot)| {
            let jitter = (lattice_sample(
                drainage_key,
                nest_lattice_coord((index as u32) % width, width),
                nest_lattice_coord((index as u32) / width, height),
                0,
            ) >> 11) as u32
                % 3;
            if filled_mm[index] >= sea_level_mm
                && accumulation[index] >= threshold.saturating_sub(jitter)
            {
                *slot = 1;
            }
        });
    check_cancelled()?;
    let mut features = Vec::new();
    for j in 0..height {
        for i in 0..width {
            let index = lattice_index(width, i, j);
            if index.is_multiple_of(CANCELLATION_STRIDE) {
                check_cancelled()?;
            }
            if channel[index] == 0 || features.len() >= MAX_TRIBUTARIES {
                continue;
            }
            let [lon, lat] = lattice_center(i, j, width, height);
            let canonical = nearest_cell(hydrology.grid, lon, lat);
            if hydrology
                .watershed_id
                .get(canonical)
                .copied()
                .unwrap_or(OCEAN)
                == OCEAN
            {
                continue;
            }
            let mut upstream_channel = false;
            for dir in DIRS {
                let Some((_, _, neighbor)) = neighbor_at(width, height, i, j, dir) else {
                    continue;
                };
                if channel[neighbor] != 0 && primary[neighbor] == index as u32 {
                    upstream_channel = true;
                    break;
                }
            }
            if upstream_channel {
                continue;
            }
            let watershed = hydrology.watershed_id[canonical];
            let mut path = vec![[lon, lat]];
            let mut current = index;
            let mut join = index;
            let mut seen = BTreeSet::from([current]);
            let mut parent_river_id = if river_cells.contains(&canonical) {
                river_at[canonical]
            } else {
                NO_PARENT_RIVER
            };
            for _ in 0..max_trace_steps(width, height).min(MAX_TRACE_STEPS) {
                let dest = primary[current];
                if dest == NO_FLOW || dest as usize >= count || !seen.insert(dest as usize) {
                    break;
                }
                current = dest as usize;
                if filled_mm[current] < sea_level_mm {
                    break;
                }
                let cj = (current as u32) / width;
                let ci = (current as u32) % width;
                let [clon, clat] = lattice_center(ci, cj, width, height);
                let join_canonical = nearest_cell(hydrology.grid, clon, clat);
                if hydrology.watershed_id.get(join_canonical).copied() != Some(watershed) {
                    break;
                }
                join = current;
                path.push([clon, clat]);
                if parent_river_id == NO_PARENT_RIVER && river_cells.contains(&join_canonical) {
                    parent_river_id = river_at[join_canonical];
                }
                if hydrology
                    .lake_cells
                    .get(join_canonical)
                    .copied()
                    .unwrap_or(false)
                {
                    break;
                }
            }
            if path.len() < 3 {
                continue;
            }
            let gauge = if accumulation.get(join).copied().unwrap_or(0) >= accumulation[index] {
                join
            } else {
                index
            };
            let runoff = runoff_mm.get(gauge).copied().unwrap_or(0);
            let discharge = channel_discharge(accumulation[gauge], runoff);
            let lattice_path = path.clone();
            displace_meander(&mut path, width, height, drainage_key, discharge);
            for (point, origin) in path.iter_mut().zip(lattice_path.iter()) {
                let cell = nearest_cell(hydrology.grid, point[0], point[1]);
                let displaced = hydrology.watershed_id.get(cell).copied().unwrap_or(OCEAN);
                if displaced != watershed {
                    *point = *origin;
                }
            }
            let width_mm = channel_width_mm(accumulation[gauge], runoff);
            features.push(RefinedTributary {
                id: String::new(),
                source_index: index,
                join_index: join,
                parent_river_id,
                ordinal: 0,
                watershed_id: watershed,
                width_mm,
                depth_mm: channel_depth_mm(width_mm),
                path,
            });
        }
    }
    features.sort_by_key(|feature| feature.source_index);
    features.truncate(MAX_TRIBUTARIES);
    number_tributaries(&mut features);
    Ok(features)
}

fn number_tributaries(features: &mut [RefinedTributary]) {
    features.sort_by(|a, b| {
        a.parent_river_id
            .cmp(&b.parent_river_id)
            .then(a.source_index.cmp(&b.source_index))
    });
    let mut next_ordinal = BTreeMap::new();
    for feature in features {
        let ordinal = next_ordinal.entry(feature.parent_river_id).or_insert(0_u32);
        feature.ordinal = *ordinal;
        feature.id = RefinedTributary::id_for(feature.parent_river_id, feature.ordinal);
        *ordinal = ordinal.saturating_add(1);
    }
}

#[allow(clippy::too_many_arguments)]
fn extract_valleys(
    width: u32,
    height: u32,
    filled_mm: &[i32],
    primary: &[u32],
    accumulation: &[u32],
    hydrology: &HydrologyField,
    river_cells: &BTreeSet<usize>,
    protected: &[bool],
    sea_level_mm: i32,
    sdf: &[i32],
    level: DetailLevel,
    drainage_key: &[u8; 32],
    check_cancelled: &mut dyn FnMut() -> Result<(), AtlasError>,
) -> Result<Vec<RefinedValley>, AtlasError> {
    let threshold = accum_threshold(level).saturating_div(2).max(4);
    let width_us = width as usize;
    check_cancelled()?;
    let mut features = (0..filled_mm.len())
        .into_par_iter()
        .filter_map(|index| {
            let j = (index / width_us) as u32;
            if j == 0 || j + 1 == height {
                return None;
            }
            let i = (index % width_us) as u32;
            let lon = lattice_lon_micro(i, width);
            let lat = lattice_lat_micro(j, height);
            let sdf_ppm = sample_sdf_ppm(hydrology.grid, sdf, lon, lat);
            let drowned =
                filled_mm[index] < sea_level_mm && sdf_ppm.unsigned_abs() <= COASTAL_ENVELOPE_PPM;
            let jitter = (lattice_sample(
                drainage_key,
                nest_lattice_coord(i, width),
                nest_lattice_coord(j, height),
                1,
            ) >> 11) as u32
                % 3;
            if protected[index]
                || (filled_mm[index] < sea_level_mm && !drowned)
                || accumulation[index] < threshold.saturating_sub(jitter)
            {
                return None;
            }
            let canonical = nearest_cell(hydrology.grid, lon, lat);
            let watershed = hydrology
                .watershed_id
                .get(canonical)
                .copied()
                .unwrap_or(OCEAN);
            if river_cells.contains(&canonical) || (watershed == OCEAN && !drowned) {
                return None;
            }
            let down = primary[index];
            for dir in DIRS {
                let Some((_, _, neighbor)) = neighbor_at(width, height, i, j, dir) else {
                    continue;
                };
                if neighbor as u32 == down {
                    continue;
                }
                if filled_mm[neighbor] < filled_mm[index] {
                    return None;
                }
            }
            Some(RefinedValley {
                id: RefinedValley::id_for(index),
                lattice_index: index,
                watershed_id: watershed,
                lon_micro: lon,
                lat_micro: lat,
            })
        })
        .collect::<Vec<_>>();
    check_cancelled()?;
    features.sort_by(|a, b| a.id.cmp(&b.id));
    features.truncate(MAX_VALLEYS);
    Ok(features)
}

#[allow(clippy::too_many_arguments)]
fn extract_deposition(
    width: u32,
    height: u32,
    worked_mm: &[i32],
    filled_mm: &[i32],
    primary: &[u32],
    accumulation: &[u32],
    hydrology: &HydrologyField,
    protected: &[bool],
    runoff_mm: &[i32],
    sea_level_mm: i32,
    sdf: &[i32],
    check_cancelled: &mut dyn FnMut() -> Result<(), AtlasError>,
) -> Result<Vec<DepositionFeature>, AtlasError> {
    let mouths = hydrology
        .rivers
        .iter()
        .map(|river| river.mouth_cell)
        .collect::<BTreeSet<_>>();
    let width_us = width as usize;
    check_cancelled()?;
    let mut features = (0..worked_mm.len())
        .into_par_iter()
        .filter_map(|index| {
            let j = (index / width_us) as u32;
            if j == 0 || j + 1 == height {
                return None;
            }
            if protected[index] {
                return None;
            }
            let dest = primary[index];
            if dest == NO_FLOW || dest as usize >= worked_mm.len() {
                return None;
            }
            let drop = (filled_mm[index] - filled_mm[dest as usize]).max(0);
            let slope_ppm = (i64::from(drop) * 1_000) / 2;
            let deposited = worked_mm[index] > filled_mm[index];
            let discharge = channel_discharge(
                accumulation[index],
                runoff_mm.get(index).copied().unwrap_or(0),
            );
            let i = (index % width_us) as u32;
            let lon = lattice_lon_micro(i, width);
            let lat = lattice_lat_micro(j, height);
            let canonical = nearest_cell(hydrology.grid, lon, lat);
            let sdf_ppm = sample_sdf_ppm(hydrology.grid, sdf, lon, lat);
            let is_mouth = mouths.contains(&canonical)
                && sdf_ppm.unsigned_abs() <= COASTAL_ENVELOPE_PPM
                && discharge >= 12;
            if !is_mouth && (worked_mm[index] < sea_level_mm || (!deposited && discharge < 24)) {
                return None;
            }
            let watershed = hydrology
                .watershed_id
                .get(canonical)
                .copied()
                .unwrap_or(OCEAN);
            if watershed == OCEAN && !is_mouth {
                return None;
            }
            let kind = if is_mouth {
                DepositionKind::Delta
            } else if slope_ppm < i64::from(FAN_SLOPE_PPM) && discharge >= 12 && deposited {
                DepositionKind::Fan
            } else if slope_ppm < i64::from(FLOODPLAIN_SLOPE_PPM) && discharge >= 24 {
                DepositionKind::Floodplain
            } else {
                return None;
            };
            Some(DepositionFeature {
                id: DepositionFeature::id_for(kind, index),
                kind,
                lattice_index: index,
                watershed_id: watershed,
                lon_micro: lon,
                lat_micro: lat,
            })
        })
        .collect::<Vec<_>>();
    check_cancelled()?;
    features.sort_by(|a, b| a.id.cmp(&b.id));
    features.truncate(MAX_DEPOSITION_FEATURES);
    Ok(features)
}

#[allow(clippy::too_many_arguments)]
fn apply_mouth_deltas(
    width: u32,
    height: u32,
    hydrology: &HydrologyField,
    accumulation: &[u32],
    runoff_mm: &[i32],
    sea_level_mm: i32,
    structure_sea_level_mm: i32,
    sdf: &[i32],
    protected: &[bool],
    worked_mm: &mut [i32],
) {
    let mouths = hydrology
        .rivers
        .iter()
        .map(|river| river.mouth_cell)
        .collect::<BTreeSet<_>>();
    if mouths.is_empty() {
        return;
    }
    let sea_shift = sea_level_mm.abs_diff(structure_sea_level_mm);
    let width_us = width as usize;
    worked_mm
        .par_iter_mut()
        .enumerate()
        .for_each(|(index, slot)| {
            let j = (index / width_us) as u32;
            if j == 0 || j + 1 == height {
                return;
            }
            if protected.get(index).copied().unwrap_or(false) {
                return;
            }
            let i = (index % width_us) as u32;
            let lon = lattice_lon_micro(i, width);
            let lat = lattice_lat_micro(j, height);
            let cell = nearest_cell(hydrology.grid, lon, lat);
            if !mouths.contains(&cell) {
                return;
            }
            let sdf_ppm = sample_sdf_ppm(hydrology.grid, sdf, lon, lat);
            if sdf_ppm.unsigned_abs() > COASTAL_ENVELOPE_PPM {
                return;
            }
            let discharge = channel_discharge(
                accumulation.get(index).copied().unwrap_or(0),
                runoff_mm.get(index).copied().unwrap_or(0),
            );
            if discharge < 12 {
                return;
            }
            let grow = (discharge / 48)
                .saturating_add(sea_shift / 64)
                .min(MAX_EROSION_STEP_MM as u32);
            if grow == 0 {
                return;
            }
            *slot = slot.saturating_add(grow as i32);
        });
}

#[allow(clippy::too_many_arguments)]
fn erode(
    model: &AmplificationModel,
    controls: &ControlFields,
    sdf: &[i32],
    filled_mm: &[i32],
    protected: &[bool],
    primary: &[u32],
    secondary: &[u32],
    weight: &[u32],
    accumulation: &[u32],
    erosion_key: &[u8; 32],
    sediment_key: &[u8; 32],
    cells: &[usize],
    check_cancelled: &mut dyn FnMut() -> Result<(), AtlasError>,
) -> Result<(Vec<i32>, Vec<i32>), AtlasError> {
    let width = model.detail.lattice_width;
    let height = model.detail.lattice_height;
    let count = filled_mm.len();
    let mut worked = filled_mm.to_vec();
    let mut sediment_mm = vec![0_i32; count];
    let mut mountain_ppm = vec![0_i32; count];
    let mut runoff_ppm = vec![0_i32; count];
    let mut freeze_thaw = vec![0_i32; count];
    let mut aridity_ppm = vec![0_i32; count];
    let mut glacial_ppm = vec![0_i32; count];
    check_cancelled()?;
    let width_us = width as usize;
    mountain_ppm
        .par_iter_mut()
        .zip(runoff_ppm.par_iter_mut())
        .zip(freeze_thaw.par_iter_mut())
        .zip(aridity_ppm.par_iter_mut())
        .zip(glacial_ppm.par_iter_mut())
        .enumerate()
        .for_each(|(index, ((((mtn, runoff), freeze), arid), glacial))| {
            let i = (index % width_us) as u32;
            let j = (index / width_us) as u32;
            let lon = lattice_lon_micro(i, width);
            let lat = lattice_lat_micro(j, height);
            *mtn = controls.sample_mountain_influence(lon, lat);
            let nh_summer = controls.sample_nh_summer_temperature(lon, lat);
            let nh_winter = controls.sample_nh_winter_temperature(lon, lat);
            let (summer, winter) = if lat >= 0 {
                (nh_summer, nh_winter)
            } else {
                (nh_winter, nh_summer)
            };
            let vegetation = vegetation_resistance_ppm(
                controls.sample_humidity(lon, lat),
                controls.sample_precipitation(lon, lat),
                controls.sample_temperature(lon, lat),
            );
            *runoff = fluvial_gain_ppm(
                controls.sample_runoff(lon, lat),
                controls.sample_precipitation(lon, lat),
                vegetation,
            );
            *freeze = freeze_thaw_ppm(summer, winter);
            *arid = controls.sample_aridity(lon, lat).clamp(0, 1_000_000);
            *glacial = glacial_work_ppm(controls.sample_ice_thickness(lon, lat), summer);
        });
    check_cancelled()?;
    let peaks = model
        .features
        .iter()
        .filter(|feature| matches!(feature.kind, MountainKind::Peak | MountainKind::Ridge))
        .map(|feature| feature.lattice_index)
        .collect::<Vec<_>>();
    let land_at = |lon: i32, lat: i32| model.detail.canonical_at(lon, lat) >= controls.sea_level_mm;
    apply_scale_erosion(
        ScaleErosion {
            grid: model.detail.grid,
            width,
            height,
            sea_level_mm: controls.sea_level_mm,
            sdf,
            protected,
            mountain_ppm: &mountain_ppm,
            runoff_ppm: &runoff_ppm,
            freeze_thaw_ppm: &freeze_thaw,
            aridity_ppm: &aridity_ppm,
            glacial_ppm: &glacial_ppm,
            primary,
            secondary,
            weight,
            accumulation,
            erosion_key,
            sediment_key,
            sediment_mm: Some(&mut sediment_mm),
            peaks: &peaks,
            filled_mm,
            land_at: &land_at,
            scales: &EROSION_SCALES,
            max_step_mm: MAX_EROSION_STEP_MM,
            cells,
        },
        &mut worked,
        check_cancelled,
    )?;
    Ok((worked, sediment_mm))
}

pub fn build_refined_hydrology(
    model: &AmplificationModel,
    controls: &ControlFields,
    hydrology: &HydrologyField,
    sdf: &[i32],
    identity: &[u8],
    structure_sea_level_mm: i32,
    check_cancelled: &mut dyn FnMut() -> Result<(), AtlasError>,
) -> Result<RefinedHydrology, AtlasError> {
    build_refined_hydrology_constrained(
        model,
        controls,
        hydrology,
        sdf,
        identity,
        structure_sea_level_mm,
        &[],
        check_cancelled,
    )
}

#[allow(clippy::too_many_arguments)]
pub fn build_refined_hydrology_constrained(
    model: &AmplificationModel,
    controls: &ControlFields,
    hydrology: &HydrologyField,
    sdf: &[i32],
    identity: &[u8],
    structure_sea_level_mm: i32,
    constraints: &[AtlasConstraint],
    check_cancelled: &mut dyn FnMut() -> Result<(), AtlasError>,
) -> Result<RefinedHydrology, AtlasError> {
    check_cancelled()?;
    let width = model.detail.lattice_width;
    let height = model.detail.lattice_height;
    let count = (width as usize)
        .checked_mul(height as usize)
        .ok_or_else(|| AtlasError::limit("lattice count overflowed"))?;
    if refined_lattice_exceeds_budget(count) {
        return Err(AtlasError::limit(
            "refined lattice exceeded the in-process byte budget",
        ));
    }
    let drainage_key = domain_key(
        identity,
        ATLAS_DETAIL_ALGORITHM_VERSION,
        model.detail.variant,
        REFINED_DRAINAGE_DOMAIN,
    );
    let erosion_key = domain_key(
        identity,
        ATLAS_DETAIL_ALGORITHM_VERSION,
        model.detail.variant,
        MULTI_SCALE_EROSION_DOMAIN,
    );
    let sediment_key = domain_key(
        identity,
        ATLAS_DETAIL_ALGORITHM_VERSION,
        model.detail.variant,
        BOUNDED_SEDIMENT_DOMAIN,
    );
    let source_mm = build_source_surface(model, controls.sea_level_mm, sdf, check_cancelled)?;
    let cells = lattice_nearest_cells(controls.grid, width, height);
    let mut protected = vec![false; count];
    check_cancelled()?;
    let width_us = width as usize;
    protected
        .par_iter_mut()
        .enumerate()
        .for_each(|(index, slot)| {
            let cell = cells[index];
            *slot = is_protected(hydrology, cell)
                && controls
                    .mountain_influence_ppm
                    .get(cell)
                    .copied()
                    .unwrap_or(0)
                    <= 0;
        });
    check_cancelled()?;
    crate::constraint::apply_to_protected(constraints, width, height, &mut protected);
    let mut coastal_mm = source_mm.clone();
    apply_coastline(
        controls,
        sdf,
        identity,
        model.detail.variant,
        width,
        height,
        structure_sea_level_mm,
        &source_mm,
        &protected,
        &mut coastal_mm,
        check_cancelled,
    )?;
    crate::constraint::apply_to_coastal(
        constraints,
        width,
        height,
        &source_mm,
        &mut coastal_mm,
        &mut protected,
    );
    let (filled_mm, filled_pit_count) = priority_fill(
        width,
        height,
        &coastal_mm,
        &protected,
        controls.sea_level_mm,
        check_cancelled,
    )?;
    let mouths = mouth_by_watershed(hydrology);
    let (flow_primary, flow_secondary, primary_weight_ppm) = assign_flow(
        width,
        height,
        &filled_mm,
        hydrology,
        &controls.elevation_mm,
        &mouths,
        &cells,
        controls.sea_level_mm,
        check_cancelled,
    )?;
    let accumulation = accumulate(
        &filled_mm,
        &flow_primary,
        &flow_secondary,
        &primary_weight_ppm,
        controls.sea_level_mm,
        check_cancelled,
    )?;
    let (river_cells, river_at) = river_occupancy(hydrology);
    let mut runoff_mm = vec![0_i32; count];
    check_cancelled()?;
    runoff_mm
        .par_iter_mut()
        .enumerate()
        .for_each(|(index, slot)| {
            let i = (index % width_us) as u32;
            let j = (index / width_us) as u32;
            *slot =
                controls.sample_runoff(lattice_lon_micro(i, width), lattice_lat_micro(j, height));
        });
    check_cancelled()?;
    let tributaries = extract_tributaries(
        width,
        height,
        &filled_mm,
        &flow_primary,
        &accumulation,
        hydrology,
        &river_cells,
        &river_at,
        &runoff_mm,
        controls.sea_level_mm,
        model.detail.level,
        &drainage_key,
        check_cancelled,
    )?;
    let valleys = extract_valleys(
        width,
        height,
        &filled_mm,
        &flow_primary,
        &accumulation,
        hydrology,
        &river_cells,
        &protected,
        controls.sea_level_mm,
        sdf,
        model.detail.level,
        &drainage_key,
        check_cancelled,
    )?;
    let (mut worked_mm, sediment_mm) = erode(
        model,
        controls,
        sdf,
        &filled_mm,
        &protected,
        &flow_primary,
        &flow_secondary,
        &primary_weight_ppm,
        &accumulation,
        &erosion_key,
        &sediment_key,
        &cells,
        check_cancelled,
    )?;
    apply_mouth_deltas(
        width,
        height,
        hydrology,
        &accumulation,
        &runoff_mm,
        controls.sea_level_mm,
        structure_sea_level_mm,
        sdf,
        &protected,
        &mut worked_mm,
    );
    let deposition = extract_deposition(
        width,
        height,
        &worked_mm,
        &filled_mm,
        &flow_primary,
        &accumulation,
        hydrology,
        &protected,
        &runoff_mm,
        controls.sea_level_mm,
        sdf,
        check_cancelled,
    )?;
    Ok(RefinedHydrology {
        version: ATLAS_DERIVED_DRAINAGE_VERSION,
        lattice_width: width,
        lattice_height: height,
        source_mm,
        filled_mm,
        worked_mm,
        flow_primary,
        flow_secondary,
        primary_weight_ppm,
        accumulation,
        protected,
        filled_pit_count,
        tributaries,
        valleys,
        deposition,
        drainage_key,
        erosion_key,
        sediment_key,
        sediment_mm,
    })
}

impl RefinedHydrology {
    #[must_use]
    pub fn encode_identities(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&self.version.to_le_bytes());
        bytes.extend_from_slice(&(self.tributaries.len() as u32).to_le_bytes());
        for tributary in &self.tributaries {
            bytes.extend_from_slice(&(tributary.source_index as u32).to_le_bytes());
            bytes.extend_from_slice(&(tributary.join_index as u32).to_le_bytes());
            bytes.extend_from_slice(&tributary.parent_river_id.to_le_bytes());
            bytes.extend_from_slice(&tributary.ordinal.to_le_bytes());
            bytes.extend_from_slice(&tributary.watershed_id.to_le_bytes());
            bytes.extend_from_slice(&tributary.width_mm.to_le_bytes());
            bytes.extend_from_slice(&tributary.depth_mm.to_le_bytes());
            bytes.extend_from_slice(&(tributary.path.len() as u32).to_le_bytes());
            for point in &tributary.path {
                bytes.extend_from_slice(&point[0].to_le_bytes());
                bytes.extend_from_slice(&point[1].to_le_bytes());
            }
        }
        bytes.extend_from_slice(&(self.valleys.len() as u32).to_le_bytes());
        for valley in &self.valleys {
            bytes.extend_from_slice(&(valley.lattice_index as u32).to_le_bytes());
            bytes.extend_from_slice(&valley.watershed_id.to_le_bytes());
            bytes.extend_from_slice(&valley.lon_micro.to_le_bytes());
            bytes.extend_from_slice(&valley.lat_micro.to_le_bytes());
        }
        bytes
    }

    pub fn decode_identities(
        bytes: &[u8],
    ) -> Result<(Vec<RefinedTributary>, Vec<RefinedValley>), AtlasError> {
        let mut offset = 0;
        let version = read_u32(bytes, &mut offset)?;
        if version != ATLAS_DERIVED_DRAINAGE_VERSION {
            return Err(AtlasError::invalid("drainage identity version mismatch"));
        }
        let tributary_count = read_u32(bytes, &mut offset)? as usize;
        if tributary_count > MAX_TRIBUTARIES {
            return Err(AtlasError::limit("tributary cache is over budget"));
        }
        let mut tributaries = Vec::with_capacity(tributary_count);
        for _ in 0..tributary_count {
            let source_index = read_u32(bytes, &mut offset)? as usize;
            let join_index = read_u32(bytes, &mut offset)? as usize;
            let parent_river_id = read_u32(bytes, &mut offset)?;
            let ordinal = read_u32(bytes, &mut offset)?;
            let watershed_id = read_u32(bytes, &mut offset)?;
            let width_mm = read_u32(bytes, &mut offset)?;
            let depth_mm = read_u32(bytes, &mut offset)?;
            let path_len = read_u32(bytes, &mut offset)? as usize;
            if path_len > MAX_TRACE_STEPS + 1 {
                return Err(AtlasError::limit("tributary path is over budget"));
            }
            let mut path = Vec::with_capacity(path_len);
            for _ in 0..path_len {
                let lon = read_i32(bytes, &mut offset)?;
                let lat = read_i32(bytes, &mut offset)?;
                path.push([lon, lat]);
            }
            tributaries.push(RefinedTributary {
                id: RefinedTributary::id_for(parent_river_id, ordinal),
                source_index,
                join_index,
                parent_river_id,
                ordinal,
                watershed_id,
                width_mm,
                depth_mm,
                path,
            });
        }
        let valley_count = read_u32(bytes, &mut offset)? as usize;
        if valley_count > MAX_VALLEYS {
            return Err(AtlasError::limit("valley cache is over budget"));
        }
        let mut valleys = Vec::with_capacity(valley_count);
        for _ in 0..valley_count {
            let lattice_index = read_u32(bytes, &mut offset)? as usize;
            let watershed_id = read_u32(bytes, &mut offset)?;
            let lon_micro = read_i32(bytes, &mut offset)?;
            let lat_micro = read_i32(bytes, &mut offset)?;
            valleys.push(RefinedValley {
                id: RefinedValley::id_for(lattice_index),
                lattice_index,
                watershed_id,
                lon_micro,
                lat_micro,
            });
        }
        if offset != bytes.len() {
            return Err(AtlasError::invalid(
                "drainage identity cache has trailing bytes",
            ));
        }
        Ok((tributaries, valleys))
    }
}

fn read_u32(bytes: &[u8], offset: &mut usize) -> Result<u32, AtlasError> {
    let end = offset.saturating_add(4);
    let slice = bytes
        .get(*offset..end)
        .ok_or_else(|| AtlasError::invalid("drainage identity cache is truncated"))?;
    *offset = end;
    Ok(u32::from_le_bytes(slice.try_into().expect("u32")))
}

fn read_i32(bytes: &[u8], offset: &mut usize) -> Result<i32, AtlasError> {
    Ok(read_u32(bytes, offset)? as i32)
}

#[must_use]
pub fn flow_stays_in_watershed(
    model: &RefinedHydrology,
    hydrology: &HydrologyField,
    sea_level_mm: i32,
) -> bool {
    let width = model.lattice_width;
    for index in 0..model.flow_primary.len() {
        if model.filled_mm[index] < sea_level_mm {
            continue;
        }
        let lon = lattice_lon_micro((index as u32) % width, width);
        let lat = lattice_lat_micro((index as u32) / width, model.lattice_height);
        let from = nearest_cell(hydrology.grid, lon, lat);
        let watershed = hydrology.watershed_id.get(from).copied().unwrap_or(OCEAN);
        if watershed == OCEAN {
            continue;
        }
        for dest in [model.flow_primary[index], model.flow_secondary[index]] {
            if dest == NO_FLOW {
                continue;
            }
            if model
                .filled_mm
                .get(dest as usize)
                .copied()
                .unwrap_or(sea_level_mm)
                < sea_level_mm
            {
                continue;
            }
            let dlon = lattice_lon_micro(dest % width, width);
            let dlat = lattice_lat_micro(dest / width, model.lattice_height);
            let to = nearest_cell(hydrology.grid, dlon, dlat);
            let to_watershed = hydrology.watershed_id.get(to).copied().unwrap_or(OCEAN);
            if to_watershed == OCEAN {
                continue;
            }
            if to_watershed != watershed {
                return false;
            }
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::amplify::{
        build_amplification_model, interior_land_components_equivalent, land_components,
    };
    use crate::cache::{decode_residual, encode_residual};
    use crate::control::ControlFields;
    use crate::detail::{
        domain_key, downsample_mean_mm, sample_field_mm, sample_sdf_ppm,
        signed_coastal_distance_ppm, COASTAL_ENVELOPE_PPM,
    };
    use crate::drainage::DerivedDrainage;
    use crate::golden_world;
    use crate::request::AtlasRenderRequest;
    use crate::spike_identity_from_source;
    use crate::{AtlasError, ATLAS_DERIVED_DRAINAGE_VERSION};
    use daena_physical::history::{
        derive_historical_world_with_planet, HistoricalForcingParameters,
    };
    use daena_physical::Grid;
    use daena_physical::NoopProgress as PhysicalNoop;
    use std::cell::Cell;
    use std::collections::{BTreeMap, BTreeSet};
    use std::fs;
    use std::time::Instant;

    fn peak_resident_bytes() -> Option<u64> {
        #[cfg(unix)]
        {
            extern "C" {
                fn getrusage(who: i32, usage: *mut u8) -> i32;
            }
            let mut usage = [0_u8; 256];
            if unsafe { getrusage(0, usage.as_mut_ptr()) } != 0 {
                return None;
            }
            let rss = i64::from_le_bytes(usage[32..40].try_into().ok()?);
            if rss <= 0 {
                return None;
            }
            let rss = rss as u64;
            if cfg!(target_os = "macos") || cfg!(target_os = "ios") {
                Some(rss)
            } else {
                Some(rss.saturating_mul(1024))
            }
        }
        #[cfg(not(unix))]
        {
            None
        }
    }

    #[test]
    fn print_production_lattice_exceeds_refine_budget() {
        let cells = |level: DetailLevel| {
            let factor = level.lattice_factor() as usize;
            384_usize
                .saturating_mul(factor)
                .saturating_mul(192)
                .saturating_mul(factor)
        };
        assert!(!refined_lattice_exceeds_budget(cells(
            DetailLevel::Standard
        )));
        assert!(!refined_lattice_exceeds_budget(cells(
            DetailLevel::Detailed
        )));
        assert!(refined_lattice_exceeds_budget(cells(DetailLevel::Print)));
    }

    fn fixture() -> (
        AmplificationModel,
        ControlFields,
        HydrologyField,
        Vec<i32>,
        Vec<u8>,
    ) {
        let world = golden_world();
        let identity = spike_identity_from_source(&world.source);
        let historical = derive_historical_world_with_planet(
            &world.field,
            world.report.reference_water_inventory_m3,
            Some(&world.tectonics.crust_by_cell),
            HistoricalForcingParameters::default_for(world.field.seed, world.field.retry_index),
            0,
            world.climate.planetary,
            &mut PhysicalNoop,
        )
        .unwrap();
        let sdf = signed_coastal_distance_ppm(
            world.field.grid,
            &world.field.elevations_mm,
            historical.metrics.sea_level_mm,
        );
        let controls = ControlFields::from_accepted(
            &world.field,
            &world.tectonics,
            &world.climate,
            &world.hydrology,
        )
        .unwrap();
        let mut cancel = || Ok(());
        let model =
            build_amplification_model(&controls, &identity, 0, DetailLevel::Standard, &mut cancel)
                .unwrap();
        (model, controls, world.hydrology.clone(), sdf, identity)
    }

    #[test]
    fn synthetic_protected_lake_is_not_filled_and_artificial_pit_is() {
        let width = 8_u32;
        let height = 4_u32;
        let mut source = vec![100_i32; 32];
        source[lattice_index(width, 2, 2)] = 10;
        source[lattice_index(width, 5, 2)] = 10;
        let mut protected = vec![false; 32];
        protected[lattice_index(width, 5, 2)] = true;
        for i in 0..width {
            source[lattice_index(width, i, 0)] = -10;
        }
        let mut cancel = || Ok(());
        let (filled, pits) =
            priority_fill(width, height, &source, &protected, 0, &mut cancel).unwrap();
        assert!(pits >= 1);
        assert!(filled[lattice_index(width, 2, 2)] > source[lattice_index(width, 2, 2)]);
        assert_eq!(
            filled[lattice_index(width, 5, 2)],
            source[lattice_index(width, 5, 2)]
        );
    }

    #[test]
    fn experimental_domains_are_isolated_from_version_one() {
        let drainage = domain_key(
            b"identity-fixture",
            ATLAS_DETAIL_ALGORITHM_VERSION,
            0,
            REFINED_DRAINAGE_DOMAIN,
        );
        let erosion = domain_key(
            b"identity-fixture",
            ATLAS_DETAIL_ALGORITHM_VERSION,
            0,
            MULTI_SCALE_EROSION_DOMAIN,
        );
        let sediment = domain_key(
            b"identity-fixture",
            ATLAS_DETAIL_ALGORITHM_VERSION,
            0,
            BOUNDED_SEDIMENT_DOMAIN,
        );
        let next_version = ATLAS_DETAIL_ALGORITHM_VERSION.wrapping_add(1);
        let other = domain_key(
            b"identity-fixture",
            next_version,
            0,
            REFINED_DRAINAGE_DOMAIN,
        );
        assert_ne!(drainage, other);
        assert_ne!(drainage, erosion);
        assert_ne!(erosion, sediment);
    }

    #[test]
    fn refinement_conserves_basins_watersheds_mouths_and_peaks() {
        let world = golden_world();
        let (model, controls, hydrology, sdf, identity) = fixture();
        let mut cancel = || Ok(());
        let started = Instant::now();
        let refined = build_refined_hydrology(
            &model,
            &controls,
            &hydrology,
            &sdf,
            &identity,
            controls.sea_level_mm,
            &mut cancel,
        )
        .unwrap();
        assert!(
            started.elapsed().as_secs() < 5,
            "standard refinement exceeded the 5s budget"
        );
        if let Some(rss) = peak_resident_bytes() {
            assert!(
                rss <= 512 * 1024 * 1024,
                "standard refinement peak RSS {rss} exceeded 512 MiB"
            );
        }
        assert!(refined.worked_mm.len() * 4 <= 2_000_000);
        assert_eq!(refined.version, ATLAS_DERIVED_DRAINAGE_VERSION);
        assert_eq!(ATLAS_DERIVED_DRAINAGE_VERSION, 2);
        assert!(
            refined
                .tributaries
                .iter()
                .any(|tributary| tributary.path.len() >= 6),
            "lattice river spines should follow flow instead of cell hops"
        );
        for (index, protected) in refined.protected.iter().enumerate() {
            if *protected {
                assert_eq!(refined.filled_mm[index], refined.source_mm[index]);
                assert_eq!(refined.worked_mm[index], refined.filled_mm[index]);
            }
        }
        assert!(flow_stays_in_watershed(
            &refined,
            &hydrology,
            controls.sea_level_mm
        ));
        let original_mouths = world
            .hydrology
            .rivers
            .iter()
            .map(|river| (river.id, river.mouth_cell, river.source_cell))
            .collect::<Vec<_>>();
        assert_eq!(
            original_mouths,
            hydrology
                .rivers
                .iter()
                .map(|river| (river.id, river.mouth_cell, river.source_cell))
                .collect::<Vec<_>>()
        );
        for tributary in &refined.tributaries {
            assert_eq!(
                tributary.id,
                RefinedTributary::id_for(tributary.parent_river_id, tributary.ordinal)
            );
            assert!(tributary.path.len() >= 3);
            assert!(tributary.width_mm >= 400);
            assert!(tributary.depth_mm >= 80);
            for point in &tributary.path {
                let cell = nearest_cell(hydrology.grid, point[0], point[1]);
                let watershed = hydrology.watershed_id[cell];
                if watershed != OCEAN {
                    assert_eq!(watershed, tributary.watershed_id);
                }
            }
        }
        assert!(!refined.tributaries.is_empty());
        assert!(!refined.valleys.is_empty());
        let mut baked = model.detail.clone();
        baked.bake_absolute_elevation(&refined.worked_mm);
        let down = downsample_mean_mm(&baked, controls.sea_level_mm, &sdf, &mut cancel).unwrap();
        assert!(interior_land_components_equivalent(
            &land_components(controls.grid, &controls.elevation_mm, controls.sea_level_mm),
            &land_components(controls.grid, &down, controls.sea_level_mm),
            &sdf
        ));
        for river in &world.hydrology.rivers {
            for cell in [river.mouth_cell, river.source_cell] {
                if sdf[cell].unsigned_abs() <= COASTAL_ENVELOPE_PPM {
                    continue;
                }
                assert_eq!(
                    world.field.elevations_mm[cell] >= controls.sea_level_mm,
                    down[cell] >= controls.sea_level_mm,
                    "river endpoint land sign changed at cell {cell}"
                );
            }
        }
        for feature in &refined.deposition {
            assert!(feature.id.starts_with(&format!(
                "atlas:deposition:v{}:",
                ATLAS_DERIVED_DRAINAGE_VERSION
            )));
        }
        let river_sources = hydrology
            .rivers
            .iter()
            .map(|river| river.source_cell as u32)
            .collect::<BTreeSet<_>>();
        for tributary in &refined.tributaries {
            assert!(
                tributary.parent_river_id == NO_PARENT_RIVER
                    || river_sources.contains(&tributary.parent_river_id)
            );
        }
        for valley in &refined.valleys {
            assert!(valley.id.starts_with(&format!(
                "atlas:valley:v{}:",
                ATLAS_DERIVED_DRAINAGE_VERSION
            )));
            let cell = nearest_cell(hydrology.grid, valley.lon_micro, valley.lat_micro);
            if hydrology.watershed_id[cell] != OCEAN {
                assert_eq!(hydrology.watershed_id[cell], valley.watershed_id);
            }
        }
        let mouth_cells = hydrology
            .rivers
            .iter()
            .map(|river| river.mouth_cell)
            .collect::<BTreeSet<_>>();
        for index in 0..refined.flow_primary.len() {
            let dest = refined.flow_primary[index];
            if dest == NO_FLOW
                || refined.filled_mm.get(dest as usize).copied().unwrap_or(0)
                    < controls.sea_level_mm
            {
                continue;
            }
            let dlon = lattice_lon_micro(dest % refined.lattice_width, refined.lattice_width);
            let dlat = lattice_lat_micro(dest / refined.lattice_width, refined.lattice_height);
            let to = nearest_cell(hydrology.grid, dlon, dlat);
            let from_lon = lattice_lon_micro(
                (index as u32) % refined.lattice_width,
                refined.lattice_width,
            );
            let from_lat = lattice_lat_micro(
                (index as u32) / refined.lattice_width,
                refined.lattice_height,
            );
            let from = nearest_cell(hydrology.grid, from_lon, from_lat);
            if mouth_cells.contains(&to)
                && hydrology.watershed_id[to] != OCEAN
                && hydrology.watershed_id[from] != OCEAN
            {
                assert_eq!(hydrology.watershed_id[to], hydrology.watershed_id[from]);
            }
        }
        let mut peak_cells = BTreeSet::new();
        for peak in model
            .features
            .iter()
            .filter(|feature| feature.kind == MountainKind::Peak)
        {
            peak_cells.insert(peak.lattice_index);
            let width = refined.lattice_width;
            let j = peak.lattice_index as u32 / width;
            let i = peak.lattice_index as u32 % width;
            let peak_sdf = sample_sdf_ppm(controls.grid, &sdf, peak.lon_micro, peak.lat_micro);
            if peak_sdf.unsigned_abs() <= COASTAL_ENVELOPE_PPM {
                continue;
            }
            let peak_elev = refined.worked_mm[peak.lattice_index];
            for dir in DIRS {
                if let Some((ni, nj, neighbor)) =
                    neighbor_at(width, refined.lattice_height, i, j, dir)
                {
                    if j == 0
                        || j + 1 == refined.lattice_height
                        || nj == 0
                        || nj + 1 == refined.lattice_height
                    {
                        continue;
                    }
                    let nlon = lattice_lon_micro(ni, width);
                    let nlat = lattice_lat_micro(nj, refined.lattice_height);
                    let neighbor_sdf = sample_sdf_ppm(controls.grid, &sdf, nlon, nlat);
                    if neighbor_sdf.unsigned_abs() <= COASTAL_ENVELOPE_PPM {
                        continue;
                    }
                    assert!(
                        peak_elev + crate::detail::MAX_RESIDUAL_MM >= refined.worked_mm[neighbor],
                        "peak {} lost local-maxima tolerance",
                        peak.id
                    );
                }
            }
        }
        let rebuilt = build_refined_hydrology(
            &model,
            &controls,
            &hydrology,
            &sdf,
            &identity,
            controls.sea_level_mm,
            &mut cancel,
        )
        .unwrap();
        assert_eq!(refined.worked_mm, rebuilt.worked_mm);
        assert_eq!(refined.tributaries, rebuilt.tributaries);
        assert_eq!(refined.valleys, rebuilt.valleys);
        for j in [0, refined.lattice_height - 1] {
            let pole = refined.worked_mm[lattice_index(refined.lattice_width, 0, j)];
            for i in 1..refined.lattice_width {
                assert_eq!(
                    refined.worked_mm[lattice_index(refined.lattice_width, i, j)],
                    pole
                );
            }
        }
        let width = refined.lattice_width;
        let height = refined.lattice_height;
        let mut sums = vec![0_i64; controls.grid.sample_count()];
        let mut counts = vec![0_u32; controls.grid.sample_count()];
        for j in 0..height {
            for i in 0..width {
                let index = lattice_index(width, i, j);
                if refined.protected[index] || peak_cells.contains(&index) {
                    continue;
                }
                let lon = lattice_lon_micro(i, width);
                let lat = lattice_lat_micro(j, height);
                let sdf_ppm = sample_sdf_ppm(controls.grid, &sdf, lon, lat);
                if sdf_ppm.unsigned_abs() <= COASTAL_ENVELOPE_PPM {
                    continue;
                }
                let source_land = refined.source_mm[index] >= controls.sea_level_mm;
                let worked_land = refined.worked_mm[index] >= controls.sea_level_mm;
                assert_eq!(source_land, worked_land, "coastal sign changed at {index}");
                let cell = nearest_cell(controls.grid, lon, lat);
                sums[cell] += i64::from(refined.worked_mm[index] - refined.filled_mm[index]);
                counts[cell] += 1;
            }
        }
        let mut max_mean = 0_i64;
        for (sum, count) in sums.iter().zip(counts.iter()) {
            if *count == 0 {
                continue;
            }
            max_mean = max_mean.max((*sum / i64::from(*count)).abs());
        }
        assert!(
            max_mean <= i64::from(MAX_EROSION_STEP_MM) * i64::from(EROSION_SCALES.len() as u32 + 1),
            "macro elevation drifted by {max_mean} mm after erosion"
        );
        let identities = refined.encode_identities();
        let (decoded_t, decoded_v) = RefinedHydrology::decode_identities(&identities).unwrap();
        assert_eq!(decoded_t, refined.tributaries);
        assert_eq!(decoded_v, refined.valleys);
        assert!(DerivedDrainage::decode(&identities).is_err());
        let pixels = encode_residual(width, height, &refined.worked_mm);
        let (pw, ph, decoded_pixels) = decode_residual(&pixels).unwrap();
        assert_eq!(
            (pw, ph, decoded_pixels),
            (width, height, refined.worked_mm.clone())
        );
        let cache_dir =
            std::env::temp_dir().join(format!("daena-atlas-refine-iter4-{}", std::process::id()));
        let _ = fs::remove_dir_all(&cache_dir);
        fs::create_dir_all(&cache_dir).unwrap();
        fs::write(cache_dir.join("identities.bin"), &identities).unwrap();
        fs::write(cache_dir.join("worked.bin"), &pixels).unwrap();
        let disk_identities = fs::read(cache_dir.join("identities.bin")).unwrap();
        let disk_pixels = fs::read(cache_dir.join("worked.bin")).unwrap();
        let (disk_t, disk_v) = RefinedHydrology::decode_identities(&disk_identities).unwrap();
        let (_, _, disk_worked) = decode_residual(&disk_pixels).unwrap();
        assert_eq!(disk_t, refined.tributaries);
        assert_eq!(disk_v, refined.valleys);
        assert_eq!(disk_worked, refined.worked_mm);
        fs::remove_dir_all(&cache_dir).unwrap();
        assert!(!cache_dir.exists());
        let lattice = Grid {
            width,
            height,
            radius_metres: controls.grid.radius_metres,
        };
        assert_eq!(
            (
                sample_field_mm(lattice, &refined.worked_mm, 0, 0),
                sample_field_mm(lattice, &refined.worked_mm, 12_345_678, -8_000_000)
            ),
            (
                sample_field_mm(lattice, &rebuilt.worked_mm, 0, 0),
                sample_field_mm(lattice, &rebuilt.worked_mm, 12_345_678, -8_000_000)
            )
        );
        let unsupported = ATLAS_DETAIL_ALGORITHM_VERSION.wrapping_add(1);
        let rejected = AtlasRenderRequest {
            algorithm_version: unsupported,
            ..AtlasRenderRequest::spike_png(64, 32).unwrap()
        }
        .normalize();
        assert!(rejected.is_err());
    }

    #[test]
    fn refinement_honors_cancellation_before_allocating_lattices() {
        let (model, controls, hydrology, sdf, identity) = fixture();
        let mut cancel = || Err(AtlasError::cancelled());
        let err = build_refined_hydrology(
            &model,
            &controls,
            &hydrology,
            &sdf,
            &identity,
            controls.sea_level_mm,
            &mut cancel,
        )
        .unwrap_err();
        assert_eq!(err, AtlasError::cancelled());
    }

    #[test]
    fn refinement_honors_cancellation_after_mid_pipeline_checks() {
        let (model, controls, hydrology, sdf, identity) = fixture();
        let checks = Cell::new(0_u32);
        let mut cancel = || {
            let next = checks.get() + 1;
            checks.set(next);
            if next > 3 {
                Err(AtlasError::cancelled())
            } else {
                Ok(())
            }
        };
        let err = build_refined_hydrology(
            &model,
            &controls,
            &hydrology,
            &sdf,
            &identity,
            controls.sea_level_mm,
            &mut cancel,
        )
        .unwrap_err();
        assert_eq!(err, AtlasError::cancelled());
        assert!(
            checks.get() > 3,
            "cancellation returned before mid-pipeline lattice checks"
        );
    }

    #[test]
    fn climate_operators_change_worked_surface_without_moving_orometry() {
        let world = golden_world();
        let identity = spike_identity_from_source(&world.source);
        let forcing =
            HistoricalForcingParameters::default_for(world.field.seed, world.field.retry_index);
        let structure = ControlFields::from_accepted(
            &world.field,
            &world.tectonics,
            &world.climate,
            &world.hydrology,
        )
        .unwrap();
        let mut cancel = || Ok(());
        let model =
            build_amplification_model(&structure, &identity, 0, DetailLevel::Standard, &mut cancel)
                .unwrap();
        assert!(model.features.iter().any(|f| f.kind == MountainKind::Peak));
        assert!(model.features.iter().any(|f| f.kind == MountainKind::Ridge));
        let year0_sdf = signed_coastal_distance_ppm(
            world.field.grid,
            &world.field.elevations_mm,
            world.hydrology.sea_level_mm,
        );
        let mut run = |offset: i64| {
            let historical = derive_historical_world_with_planet(
                &world.field,
                world.report.reference_water_inventory_m3,
                Some(&world.tectonics.crust_by_cell),
                forcing,
                offset,
                world.climate.planetary,
                &mut PhysicalNoop,
            )
            .unwrap();
            let sdf = signed_coastal_distance_ppm(
                world.field.grid,
                &world.field.elevations_mm,
                historical.metrics.sea_level_mm,
            );
            let controls = ControlFields::from_accepted(
                &world.field,
                &world.tectonics,
                &historical.climate,
                &historical.hydrology,
            )
            .unwrap();
            let refined = build_refined_hydrology(
                &model,
                &controls,
                &historical.hydrology,
                &sdf,
                &identity,
                structure.sea_level_mm,
                &mut cancel,
            )
            .unwrap();
            (historical.metrics.land_ice_m3, refined)
        };
        let (_, present) = run(0);
        let (cold_ice, cold) = run(-8_000);
        let (warm_ice, warm) = run(8_000);
        assert_ne!(cold_ice, warm_ice);
        assert_eq!(present.erosion_key, cold.erosion_key);
        assert_eq!(cold.erosion_key, warm.erosion_key);
        assert_eq!(present.sediment_key, cold.sediment_key);
        assert_eq!(cold.sediment_key, warm.sediment_key);
        assert_ne!(present.erosion_key, present.sediment_key);
        for value in present
            .sediment_mm
            .iter()
            .chain(&cold.sediment_mm)
            .chain(&warm.sediment_mm)
        {
            assert!(value.abs() <= crate::erosion::DUNE_MAX_MM);
        }
        let crests_hold = |refined: &RefinedHydrology| {
            for crest in model
                .features
                .iter()
                .filter(|feature| matches!(feature.kind, MountainKind::Peak | MountainKind::Ridge))
            {
                let width = refined.lattice_width;
                let j = crest.lattice_index as u32 / width;
                let i = crest.lattice_index as u32 % width;
                let crest_sdf = sample_sdf_ppm(
                    world.field.grid,
                    &year0_sdf,
                    crest.lon_micro,
                    crest.lat_micro,
                );
                if crest_sdf.unsigned_abs() <= COASTAL_ENVELOPE_PPM {
                    continue;
                }
                let crest_elev = refined.worked_mm[crest.lattice_index];
                for dir in DIRS {
                    if let Some((ni, nj, neighbor)) =
                        neighbor_at(width, refined.lattice_height, i, j, dir)
                    {
                        if j == 0
                            || j + 1 == refined.lattice_height
                            || nj == 0
                            || nj + 1 == refined.lattice_height
                        {
                            continue;
                        }
                        let nlon = lattice_lon_micro(ni, width);
                        let nlat = lattice_lat_micro(nj, refined.lattice_height);
                        let neighbor_sdf = sample_sdf_ppm(world.field.grid, &year0_sdf, nlon, nlat);
                        if neighbor_sdf.unsigned_abs() <= COASTAL_ENVELOPE_PPM {
                            continue;
                        }
                        assert!(
                            crest_elev + crate::detail::MAX_RESIDUAL_MM
                                >= refined.worked_mm[neighbor],
                            "{} {} lost crest tolerance",
                            crest.kind.as_str(),
                            crest.id
                        );
                    }
                }
            }
        };
        crests_hold(&present);
        crests_hold(&cold);
        crests_hold(&warm);
        let interior = |refined: &RefinedHydrology| {
            let width = refined.lattice_width;
            let mut cut = 0_i64;
            let mut rough = 0_i64;
            for j in 1..refined.lattice_height.saturating_sub(1) {
                for i in 0..width {
                    let index = lattice_index(width, i, j);
                    if refined.protected[index] {
                        continue;
                    }
                    let lon = lattice_lon_micro(i, width);
                    let lat = lattice_lat_micro(j, refined.lattice_height);
                    let sdf_ppm = sample_sdf_ppm(world.field.grid, &year0_sdf, lon, lat);
                    if sdf_ppm.unsigned_abs() <= COASTAL_ENVELOPE_PPM {
                        continue;
                    }
                    if refined.source_mm[index] < world.hydrology.sea_level_mm {
                        continue;
                    }
                    cut += i64::from(refined.filled_mm[index] - refined.worked_mm[index]).abs();
                    let east = lattice_index(width, (i + 1) % width, j);
                    rough += i64::from(refined.worked_mm[index] - refined.worked_mm[east]).abs();
                }
            }
            (cut, rough)
        };
        let present_work = interior(&present);
        let cold_work = interior(&cold);
        let warm_work = interior(&warm);
        assert_ne!(present.worked_mm, cold.worked_mm);
        assert_ne!(cold.worked_mm, warm.worked_mm);
        assert_ne!(
            cold_work, warm_work,
            "cold vs warm interior gully/roughness"
        );
        assert_ne!(present_work, cold_work);
    }

    #[test]
    fn golden_world_dunes_fire_when_arid_and_keep_prf_across_epochs() {
        let world = golden_world();
        let identity = spike_identity_from_source(&world.source);
        let forcing =
            HistoricalForcingParameters::default_for(world.field.seed, world.field.retry_index);
        let structure = ControlFields::from_accepted(
            &world.field,
            &world.tectonics,
            &world.climate,
            &world.hydrology,
        )
        .unwrap();
        let mut cancel = || Ok(());
        let model =
            build_amplification_model(&structure, &identity, 0, DetailLevel::Standard, &mut cancel)
                .unwrap();
        let sdf = signed_coastal_distance_ppm(
            world.field.grid,
            &world.field.elevations_mm,
            world.hydrology.sea_level_mm,
        );
        let present = build_refined_hydrology(
            &model,
            &structure,
            &world.hydrology,
            &sdf,
            &identity,
            structure.sea_level_mm,
            &mut cancel,
        )
        .unwrap();
        let mut arid_controls = structure.clone();
        for value in &mut arid_controls.aridity_ppm {
            *value = 900_000;
        }
        for value in &mut arid_controls.ice_thickness_mm {
            *value = 0;
        }
        let arid = build_refined_hydrology(
            &model,
            &arid_controls,
            &world.hydrology,
            &sdf,
            &identity,
            structure.sea_level_mm,
            &mut cancel,
        )
        .unwrap();
        assert_eq!(present.sediment_key, arid.sediment_key);
        assert_ne!(
            present.sediment_mm, arid.sediment_mm,
            "raising aridity at t must change dune magnitude, not only occupancy"
        );
        assert!(
            arid.sediment_mm.iter().any(|&value| value != 0),
            "forced-arid golden lattice must emit dune grain"
        );
        assert!(arid
            .sediment_mm
            .iter()
            .all(|value| value.abs() <= crate::erosion::DUNE_MAX_MM));
        let mut run = |offset: i64| {
            let historical = derive_historical_world_with_planet(
                &world.field,
                world.report.reference_water_inventory_m3,
                Some(&world.tectonics.crust_by_cell),
                forcing,
                offset,
                world.climate.planetary,
                &mut PhysicalNoop,
            )
            .unwrap();
            let sdf = signed_coastal_distance_ppm(
                world.field.grid,
                &world.field.elevations_mm,
                historical.metrics.sea_level_mm,
            );
            let controls = ControlFields::from_accepted(
                &world.field,
                &world.tectonics,
                &historical.climate,
                &historical.hydrology,
            )
            .unwrap();
            build_refined_hydrology(
                &model,
                &controls,
                &historical.hydrology,
                &sdf,
                &identity,
                structure.sea_level_mm,
                &mut cancel,
            )
            .unwrap()
        };
        let cold = run(-8_000);
        let warm = run(8_000);
        assert_eq!(present.sediment_key, cold.sediment_key);
        assert_eq!(cold.sediment_key, warm.sediment_key);
        for value in cold.sediment_mm.iter().chain(&warm.sediment_mm) {
            assert!(value.abs() <= crate::erosion::DUNE_MAX_MM);
        }
    }

    #[test]
    fn epoch_hydrology_keeps_river_ids_while_channels_change() {
        let world = golden_world();
        let identity = spike_identity_from_source(&world.source);
        let forcing =
            HistoricalForcingParameters::default_for(world.field.seed, world.field.retry_index);
        let structure = ControlFields::from_accepted(
            &world.field,
            &world.tectonics,
            &world.climate,
            &world.hydrology,
        )
        .unwrap();
        let mut cancel = || Ok(());
        let model =
            build_amplification_model(&structure, &identity, 0, DetailLevel::Standard, &mut cancel)
                .unwrap();
        let mut run = |offset: i64| {
            let historical = derive_historical_world_with_planet(
                &world.field,
                world.report.reference_water_inventory_m3,
                Some(&world.tectonics.crust_by_cell),
                forcing,
                offset,
                world.climate.planetary,
                &mut PhysicalNoop,
            )
            .unwrap();
            let sdf = signed_coastal_distance_ppm(
                world.field.grid,
                &world.field.elevations_mm,
                historical.metrics.sea_level_mm,
            );
            let controls = ControlFields::from_accepted(
                &world.field,
                &world.tectonics,
                &historical.climate,
                &historical.hydrology,
            )
            .unwrap();
            let refined = build_refined_hydrology(
                &model,
                &controls,
                &historical.hydrology,
                &sdf,
                &identity,
                structure.sea_level_mm,
                &mut cancel,
            )
            .unwrap();
            (historical.hydrology, refined)
        };
        let (present_h, present) = run(0);
        let (cold_h, cold) = run(-8_000);
        let (warm_h, warm) = run(8_000);
        let ids = |hydrology: &HydrologyField| {
            hydrology
                .rivers
                .iter()
                .map(|river| daena_physical::hydro_claim::river_id(river.source_cell))
                .collect::<BTreeSet<_>>()
        };
        let mouths = |hydrology: &HydrologyField| {
            let mut map = BTreeMap::new();
            for river in &hydrology.rivers {
                map.entry(river.source_cell)
                    .and_modify(|mouth: &mut usize| *mouth = (*mouth).min(river.mouth_cell))
                    .or_insert(river.mouth_cell);
            }
            map
        };
        let path_for = |hydrology: &HydrologyField, source: usize| {
            hydrology
                .rivers
                .iter()
                .zip(&hydrology.river_coordinates)
                .filter(|(river, _)| river.source_cell == source)
                .min_by_key(|(river, _)| (river.mouth_cell, river.id))
                .map(|(_, path)| path.clone())
        };
        let present_ids = ids(&present_h);
        let cold_ids = ids(&cold_h);
        let warm_ids = ids(&warm_h);
        let shared_cold = present_ids
            .intersection(&cold_ids)
            .cloned()
            .collect::<Vec<_>>();
        let shared_warm = present_ids
            .intersection(&warm_ids)
            .cloned()
            .collect::<Vec<_>>();
        assert!(!shared_cold.is_empty(), "no shared river spines at cold");
        assert!(!shared_warm.is_empty(), "no shared river spines at warm");
        let present_mouths = mouths(&present_h);
        let cold_mouths = mouths(&cold_h);
        let warm_mouths = mouths(&warm_h);
        for (hydrology, shared) in [
            (&present_h, &shared_cold),
            (&cold_h, &shared_cold),
            (&warm_h, &shared_warm),
        ] {
            for id in shared {
                let claim = daena_physical::hydro_claim::resolve(
                    hydrology,
                    daena_physical::hydro_claim::KIND_RIVER,
                    id,
                );
                assert_eq!(
                    claim.as_ref().map(|claim| claim.id.as_str()),
                    Some(id.as_str())
                );
            }
        }
        let spine_moved =
            |shared: &[String], other_mouths: &BTreeMap<usize, usize>, other: &HydrologyField| {
                shared.iter().any(|id| {
                    let source: usize = id
                        .strip_prefix("river:")
                        .and_then(|rest| rest.parse().ok())
                        .expect("source spine");
                    other_mouths
                        .get(&source)
                        .is_some_and(|mouth| *mouth != present_mouths[&source])
                        || path_for(&present_h, source) != path_for(other, source)
                })
            };
        assert!(
            spine_moved(&shared_cold, &cold_mouths, &cold_h)
                || spine_moved(&shared_warm, &warm_mouths, &warm_h),
            "shared spines must change mouth or path across epochs"
        );
        assert_eq!(present.drainage_key, cold.drainage_key);
        assert_eq!(cold.drainage_key, warm.drainage_key);
        let widths = |refined: &RefinedHydrology| {
            refined
                .tributaries
                .iter()
                .map(|tributary| {
                    (
                        tributary.parent_river_id,
                        tributary.ordinal,
                        tributary.width_mm,
                    )
                })
                .collect::<BTreeSet<_>>()
        };
        let paths = |refined: &RefinedHydrology| {
            refined
                .tributaries
                .iter()
                .map(|tributary| {
                    (
                        tributary.parent_river_id,
                        tributary.ordinal,
                        tributary.path.clone(),
                    )
                })
                .collect::<Vec<_>>()
        };
        assert_ne!(widths(&present), widths(&cold));
        assert_ne!(paths(&present), paths(&cold));
        assert_ne!(paths(&cold), paths(&warm));
        assert!(!present.tributaries.is_empty());
        for tributary in present
            .tributaries
            .iter()
            .chain(&cold.tributaries)
            .chain(&warm.tributaries)
        {
            assert_eq!(
                tributary.id,
                RefinedTributary::id_for(tributary.parent_river_id, tributary.ordinal)
            );
        }
    }

    #[test]
    fn epoch_sea_moves_one_shoreline() {
        let world = golden_world();
        let identity = spike_identity_from_source(&world.source);
        let forcing =
            HistoricalForcingParameters::default_for(world.field.seed, world.field.retry_index);
        let structure = ControlFields::from_accepted(
            &world.field,
            &world.tectonics,
            &world.climate,
            &world.hydrology,
        )
        .unwrap();
        let mut cancel = || Ok(());
        let model =
            build_amplification_model(&structure, &identity, 0, DetailLevel::Standard, &mut cancel)
                .unwrap();
        let mut run = |offset: i64| {
            let historical = derive_historical_world_with_planet(
                &world.field,
                world.report.reference_water_inventory_m3,
                Some(&world.tectonics.crust_by_cell),
                forcing,
                offset,
                world.climate.planetary,
                &mut PhysicalNoop,
            )
            .unwrap();
            let sdf = signed_coastal_distance_ppm(
                world.field.grid,
                &world.field.elevations_mm,
                historical.metrics.sea_level_mm,
            );
            let controls = ControlFields::from_accepted(
                &world.field,
                &world.tectonics,
                &historical.climate,
                &historical.hydrology,
            )
            .unwrap();
            let refined = build_refined_hydrology(
                &model,
                &controls,
                &historical.hydrology,
                &sdf,
                &identity,
                structure.sea_level_mm,
                &mut cancel,
            )
            .unwrap();
            (historical.metrics.sea_level_mm, sdf, refined)
        };
        let (present_sea, present_sdf, present) = run(0);
        let (cold_sea, cold_sdf, cold) = run(-8_000);
        let (warm_sea, warm_sdf, warm) = run(8_000);
        assert_ne!(cold_sea, present_sea);
        assert_ne!(warm_sea, present_sea);
        let flips = |a: &RefinedHydrology,
                     sea_a: i32,
                     sdf_a: &[i32],
                     b: &RefinedHydrology,
                     sea_b: i32,
                     sdf_b: &[i32]| {
            let width = a.lattice_width;
            let mut n = 0_u32;
            for j in 0..a.lattice_height {
                for i in 0..width {
                    let lon = lattice_lon_micro(i, width);
                    let lat = lattice_lat_micro(j, a.lattice_height);
                    let sa = sample_sdf_ppm(world.field.grid, sdf_a, lon, lat);
                    let sb = sample_sdf_ppm(world.field.grid, sdf_b, lon, lat);
                    if sa.unsigned_abs() > COASTAL_ENVELOPE_PPM
                        && sb.unsigned_abs() > COASTAL_ENVELOPE_PPM
                    {
                        continue;
                    }
                    let index = lattice_index(width, i, j);
                    if (a.filled_mm[index] >= sea_a) != (b.filled_mm[index] >= sea_b) {
                        n += 1;
                    }
                }
            }
            n
        };
        let cold_flips = flips(
            &present,
            present_sea,
            &present_sdf,
            &cold,
            cold_sea,
            &cold_sdf,
        );
        let warm_flips = flips(
            &present,
            present_sea,
            &present_sdf,
            &warm,
            warm_sea,
            &warm_sdf,
        );
        assert!(
            cold_flips >= 1 && warm_flips >= 1 && cold_flips.max(warm_flips) >= 16,
            "shoreline did not move both ways: cold_flips={cold_flips} warm_flips={warm_flips}"
        );
        let continent_holds = |refined: &RefinedHydrology, sea: i32, sdf: &[i32]| {
            let width = refined.lattice_width;
            for j in 0..refined.lattice_height {
                for i in 0..width {
                    let lon = lattice_lon_micro(i, width);
                    let lat = lattice_lat_micro(j, refined.lattice_height);
                    let sdf_ppm = sample_sdf_ppm(world.field.grid, sdf, lon, lat);
                    if sdf_ppm.unsigned_abs() <= COASTAL_ENVELOPE_PPM {
                        continue;
                    }
                    let canonical = model.detail.canonical_at(lon, lat);
                    let worked = refined.worked_mm[lattice_index(width, i, j)];
                    assert_eq!(
                        canonical >= sea,
                        worked >= sea,
                        "continent sign flipped outside envelope"
                    );
                }
            }
        };
        continent_holds(&present, present_sea, &present_sdf);
        continent_holds(&cold, cold_sea, &cold_sdf);
        continent_holds(&warm, warm_sea, &warm_sdf);
        let warm_valley = warm
            .valleys
            .iter()
            .map(|valley| valley.lattice_index)
            .collect::<BTreeSet<_>>();
        let drowned = present
            .valleys
            .iter()
            .filter(|valley| {
                let sdf_ppm = sample_sdf_ppm(
                    world.field.grid,
                    &warm_sdf,
                    valley.lon_micro,
                    valley.lat_micro,
                );
                warm.filled_mm[valley.lattice_index] < warm_sea
                    && sdf_ppm.unsigned_abs() <= COASTAL_ENVELOPE_PPM
            })
            .collect::<Vec<_>>();
        if warm_sea > present_sea {
            assert!(!drowned.is_empty(), "rising sea drowned no valley spines");
            assert!(
                drowned
                    .iter()
                    .any(|valley| warm_valley.contains(&valley.lattice_index)),
                "drowned valley spine IDs dropped"
            );
        }
        assert!(
            present
                .deposition
                .iter()
                .chain(&cold.deposition)
                .chain(&warm.deposition)
                .any(|feature| feature.kind == DepositionKind::Delta),
            "mouth deltas missing"
        );
    }
}
