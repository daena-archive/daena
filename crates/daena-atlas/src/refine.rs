//! Derived drainage 3: bounded pit fill, watershed-constrained continuous
//! flow, atlas-only tributaries, and multi-scale erosion during and after
//! hierarchical amplification.

use std::cmp::Reverse;
use std::collections::{BTreeSet, BinaryHeap};

use daena_physical::hydrology::{BasinStatus, HydrologyField};

use crate::amplify::{AmplificationModel, MountainKind};
use crate::control::ControlFields;
use crate::detail::{
    domain_key, lattice_lat_micro, lattice_lon_micro, lattice_sample, nearest_cell, sample_sdf_ppm,
};
use crate::erosion::{
    apply_scale_erosion, lattice_index, lock_polar_rows, neighbor_at, ScaleErosion, DIRS,
    EROSION_SCALES, FAN_SLOPE_PPM, FLOODPLAIN_SLOPE_PPM, MAX_EROSION_STEP_MM,
    MULTI_SCALE_EROSION_DOMAIN, NO_FLOW,
};
use crate::request::DetailLevel;
use crate::{AtlasError, ATLAS_DERIVED_DRAINAGE_VERSION, ATLAS_DETAIL_ALGORITHM_VERSION};

pub const REFINED_DRAINAGE_DOMAIN: &str = "refined-drainage";
pub const MAX_TRIBUTARIES: usize = 128;
pub const MAX_VALLEYS: usize = 64;
pub const MAX_DEPOSITION_FEATURES: usize = 64;
pub const MAX_FILL_MM: i32 = 4_800;
const CANCELLATION_STRIDE: usize = 4_096;
const MAX_TRACE_STEPS: usize = 64;
const OCEAN: u32 = u32::MAX;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RefinedTributary {
    pub id: String,
    pub source_index: usize,
    pub join_index: usize,
    pub parent_river_id: u32,
    pub watershed_id: u32,
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
}

impl DepositionKind {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Fan => "fan",
            Self::Floodplain => "floodplain",
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
}

impl RefinedTributary {
    #[must_use]
    pub fn id_for(lattice_index: usize) -> String {
        format!("atlas:tributary:v{ATLAS_DERIVED_DRAINAGE_VERSION}:{lattice_index}")
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
    let mut river_at = vec![0_u32; count];
    for (index, path) in hydrology.river_coordinates.iter().enumerate() {
        let id = hydrology.rivers.get(index).map_or(0, |river| river.id);
        for point in path {
            let cell = nearest_cell(hydrology.grid, point[0], point[1]);
            cells.insert(cell);
            if river_at[cell] == 0 {
                river_at[cell] = id;
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
    let mut source = vec![0_i32; width as usize * height as usize];
    for j in 0..height {
        if (j as usize).is_multiple_of(CANCELLATION_STRIDE) {
            check_cancelled()?;
        }
        for i in 0..width {
            let lon = lattice_lon_micro(i, width);
            let lat = lattice_lat_micro(j, height);
            let sdf_ppm = sample_sdf_ppm(model.detail.grid, sdf, lon, lat);
            source[lattice_index(width, i, j)] =
                model.detail.refined_at(lon, lat, sea_level_mm, sdf_ppm);
        }
    }
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
    let mut visited = vec![false; count];
    let mut queue = BinaryHeap::new();
    for index in 0..count {
        if index.is_multiple_of(CANCELLATION_STRIDE) {
            check_cancelled()?;
        }
        if source_mm[index] < sea_level_mm || protected[index] {
            visited[index] = true;
            queue.push((Reverse(source_mm[index]), Reverse(index)));
        }
    }
    if queue.is_empty() {
        return Err(AtlasError::limit(
            "pit fill has no ocean or protected outlet",
        ));
    }
    while let Some((Reverse(level), Reverse(index))) = queue.pop() {
        let j = (index as u32) / width;
        let i = (index as u32) % width;
        if (j as usize).is_multiple_of(CANCELLATION_STRIDE) {
            check_cancelled()?;
        }
        for dir in DIRS {
            let Some((_, _, neighbor)) = neighbor_at(width, height, i, j, dir) else {
                continue;
            };
            if visited[neighbor] {
                continue;
            }
            visited[neighbor] = true;
            let nj = (neighbor as u32) / width;
            let polar = nj == 0 || nj + 1 == height;
            if !protected[neighbor] && !polar {
                let raised = routing[neighbor].max(level);
                let capped = source_mm[neighbor].saturating_add(MAX_FILL_MM);
                routing[neighbor] = raised.min(capped);
            }
            queue.push((Reverse(routing[neighbor]), Reverse(neighbor)));
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
    sea_level_mm: i32,
    check_cancelled: &mut dyn FnMut() -> Result<(), AtlasError>,
) -> Result<(Vec<u32>, Vec<u32>, Vec<u32>), AtlasError> {
    let count = filled_mm.len();
    let mut primary = vec![NO_FLOW; count];
    let mut secondary = vec![NO_FLOW; count];
    let mut weight = vec![0_u32; count];
    for j in 0..height {
        if (j as usize).is_multiple_of(CANCELLATION_STRIDE) {
            check_cancelled()?;
        }
        for i in 0..width {
            let index = lattice_index(width, i, j);
            if filled_mm[index] < sea_level_mm {
                continue;
            }
            let lon = lattice_lon_micro(i, width);
            let lat = lattice_lat_micro(j, height);
            let canonical = nearest_cell(hydrology.grid, lon, lat);
            let watershed = hydrology
                .watershed_id
                .get(canonical)
                .copied()
                .unwrap_or(OCEAN);
            if watershed == OCEAN {
                continue;
            }
            let mut best_dir = 8_usize;
            let mut best_slope = 0_i64;
            let mut best_neighbor = NO_FLOW;
            let mut candidates = [None; 8];
            for (dir_index, dir) in DIRS.iter().copied().enumerate() {
                let Some((_, _, neighbor)) = neighbor_at(width, height, i, j, dir) else {
                    continue;
                };
                let n_lon = lattice_lon_micro(neighbor as u32 % width, width);
                let n_lat = lattice_lat_micro(neighbor as u32 / width, height);
                let n_canonical = nearest_cell(hydrology.grid, n_lon, n_lat);
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
                    let n_lon = lattice_lon_micro(neighbor as u32 % width, width);
                    let n_lat = lattice_lat_micro(neighbor as u32 / width, height);
                    let n_canonical = nearest_cell(hydrology.grid, n_lon, n_lat);
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
                    primary[index] = fallback;
                    weight[index] = 1_000_000;
                }
                continue;
            }
            primary[index] = best_neighbor;
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
                    primary[index] = best_neighbor;
                    secondary[index] = neighbor;
                    weight[index] = ((best_slope * 1_000_000) / total) as u32;
                    continue;
                }
            }
            weight[index] = 1_000_000;
        }
    }
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
    order.sort_by_key(|index| (Reverse(filled_mm[*index]), *index));
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
    sea_level_mm: i32,
    level: DetailLevel,
    drainage_key: &[u8; 32],
    check_cancelled: &mut dyn FnMut() -> Result<(), AtlasError>,
) -> Result<Vec<RefinedTributary>, AtlasError> {
    let threshold = accum_threshold(level);
    let count = filled_mm.len();
    let mut channel = vec![false; count];
    for index in 0..count {
        if index.is_multiple_of(CANCELLATION_STRIDE) {
            check_cancelled()?;
        }
        let jitter = (lattice_sample(
            drainage_key,
            (index as u32) % width,
            (index as u32) / width,
            0,
        ) >> 11) as u32
            % 3;
        if filled_mm[index] >= sea_level_mm
            && accumulation[index] >= threshold.saturating_sub(jitter)
        {
            channel[index] = true;
        }
    }
    let mut features = Vec::new();
    for j in 0..height {
        for i in 0..width {
            let index = lattice_index(width, i, j);
            if index.is_multiple_of(CANCELLATION_STRIDE) {
                check_cancelled()?;
            }
            if !channel[index] || features.len() >= MAX_TRIBUTARIES {
                continue;
            }
            let lon = lattice_lon_micro(i, width);
            let lat = lattice_lat_micro(j, height);
            let canonical = nearest_cell(hydrology.grid, lon, lat);
            if river_cells.contains(&canonical)
                || hydrology
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
                if channel[neighbor] && primary[neighbor] == index as u32 {
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
            let mut parent_river_id = 0_u32;
            for _ in 0..MAX_TRACE_STEPS {
                let dest = primary[current];
                if dest == NO_FLOW || dest as usize >= count || !seen.insert(dest as usize) {
                    break;
                }
                current = dest as usize;
                join = current;
                let cj = (current as u32) / width;
                let ci = (current as u32) % width;
                let clon = lattice_lon_micro(ci, width);
                let clat = lattice_lat_micro(cj, height);
                path.push([clon, clat]);
                let join_canonical = nearest_cell(hydrology.grid, clon, clat);
                if hydrology.watershed_id.get(join_canonical).copied() != Some(watershed) {
                    path.pop();
                    break;
                }
                if river_cells.contains(&join_canonical) {
                    parent_river_id = river_at[join_canonical];
                    break;
                }
                if filled_mm[current] < sea_level_mm {
                    break;
                }
            }
            if path.len() < 3 {
                continue;
            }
            features.push(RefinedTributary {
                id: RefinedTributary::id_for(index),
                source_index: index,
                join_index: join,
                parent_river_id,
                watershed_id: watershed,
                path,
            });
        }
    }
    features.sort_by(|a, b| a.id.cmp(&b.id));
    features.truncate(MAX_TRIBUTARIES);
    Ok(features)
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
    level: DetailLevel,
    drainage_key: &[u8; 32],
    check_cancelled: &mut dyn FnMut() -> Result<(), AtlasError>,
) -> Result<Vec<RefinedValley>, AtlasError> {
    let threshold = accum_threshold(level).saturating_div(2).max(4);
    let mut features = Vec::new();
    for j in 1..height.saturating_sub(1) {
        for i in 0..width {
            let index = lattice_index(width, i, j);
            if index.is_multiple_of(CANCELLATION_STRIDE) {
                check_cancelled()?;
            }
            let jitter = (lattice_sample(drainage_key, i, j, 1) >> 11) as u32 % 3;
            if protected[index]
                || filled_mm[index] < sea_level_mm
                || accumulation[index] < threshold.saturating_sub(jitter)
                || features.len() >= MAX_VALLEYS
            {
                continue;
            }
            let lon = lattice_lon_micro(i, width);
            let lat = lattice_lat_micro(j, height);
            let canonical = nearest_cell(hydrology.grid, lon, lat);
            let watershed = hydrology
                .watershed_id
                .get(canonical)
                .copied()
                .unwrap_or(OCEAN);
            if watershed == OCEAN || river_cells.contains(&canonical) {
                continue;
            }
            let down = primary[index];
            let mut valley_bottom = true;
            for dir in DIRS {
                let Some((_, _, neighbor)) = neighbor_at(width, height, i, j, dir) else {
                    continue;
                };
                if neighbor as u32 == down {
                    continue;
                }
                if filled_mm[neighbor] < filled_mm[index] {
                    valley_bottom = false;
                    break;
                }
            }
            if !valley_bottom {
                continue;
            }
            features.push(RefinedValley {
                id: RefinedValley::id_for(index),
                lattice_index: index,
                watershed_id: watershed,
                lon_micro: lon,
                lat_micro: lat,
            });
        }
    }
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
    sea_level_mm: i32,
    check_cancelled: &mut dyn FnMut() -> Result<(), AtlasError>,
) -> Result<Vec<DepositionFeature>, AtlasError> {
    let mut features = Vec::new();
    for j in 1..height.saturating_sub(1) {
        if (j as usize).is_multiple_of(CANCELLATION_STRIDE) {
            check_cancelled()?;
        }
        for i in 0..width {
            let index = lattice_index(width, i, j);
            if protected[index]
                || worked_mm[index] < sea_level_mm
                || features.len() >= MAX_DEPOSITION_FEATURES
            {
                continue;
            }
            let dest = primary[index];
            if dest == NO_FLOW || dest as usize >= worked_mm.len() {
                continue;
            }
            let drop = (filled_mm[index] - filled_mm[dest as usize]).max(0);
            let slope_ppm = (i64::from(drop) * 1_000) / 2;
            let deposited = worked_mm[index] > filled_mm[index];
            if !deposited && accumulation[index] < 24 {
                continue;
            }
            let lon = lattice_lon_micro(i, width);
            let lat = lattice_lat_micro(j, height);
            let canonical = nearest_cell(hydrology.grid, lon, lat);
            let watershed = hydrology
                .watershed_id
                .get(canonical)
                .copied()
                .unwrap_or(OCEAN);
            if watershed == OCEAN {
                continue;
            }
            let kind =
                if slope_ppm < i64::from(FAN_SLOPE_PPM) && accumulation[index] >= 12 && deposited {
                    DepositionKind::Fan
                } else if slope_ppm < i64::from(FLOODPLAIN_SLOPE_PPM) && accumulation[index] >= 24 {
                    DepositionKind::Floodplain
                } else {
                    continue;
                };
            if features.iter().any(|feature: &DepositionFeature| {
                feature.kind == kind && feature.lattice_index == index
            }) {
                continue;
            }
            features.push(DepositionFeature {
                id: DepositionFeature::id_for(kind, index),
                kind,
                lattice_index: index,
                watershed_id: watershed,
                lon_micro: lon,
                lat_micro: lat,
            });
        }
    }
    features.sort_by(|a, b| a.id.cmp(&b.id));
    features.truncate(MAX_DEPOSITION_FEATURES);
    Ok(features)
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
    check_cancelled: &mut dyn FnMut() -> Result<(), AtlasError>,
) -> Result<Vec<i32>, AtlasError> {
    let width = model.detail.lattice_width;
    let height = model.detail.lattice_height;
    let count = filled_mm.len();
    let mut worked = filled_mm.to_vec();
    let mut mountain_ppm = vec![0_i32; count];
    let mut runoff_ppm = vec![0_i32; count];
    for j in 0..height {
        for i in 0..width {
            let index = lattice_index(width, i, j);
            let lon = lattice_lon_micro(i, width);
            let lat = lattice_lat_micro(j, height);
            mountain_ppm[index] = controls.sample_mountain_influence(lon, lat);
            runoff_ppm[index] = (i64::from(controls.sample_runoff(lon, lat).clamp(0, 4_000))
                * 1_000_000
                / 4_000) as i32;
        }
    }
    let peaks = model
        .features
        .iter()
        .filter(|feature| feature.kind == MountainKind::Peak)
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
            primary,
            secondary,
            weight,
            accumulation,
            erosion_key,
            peaks: &peaks,
            filled_mm,
            land_at: &land_at,
            scales: &EROSION_SCALES,
            max_step_mm: MAX_EROSION_STEP_MM,
        },
        &mut worked,
        check_cancelled,
    )?;
    Ok(worked)
}

pub fn build_refined_hydrology(
    model: &AmplificationModel,
    controls: &ControlFields,
    hydrology: &HydrologyField,
    sdf: &[i32],
    identity: &[u8],
    check_cancelled: &mut dyn FnMut() -> Result<(), AtlasError>,
) -> Result<RefinedHydrology, AtlasError> {
    check_cancelled()?;
    let width = model.detail.lattice_width;
    let height = model.detail.lattice_height;
    let count = (width as usize)
        .checked_mul(height as usize)
        .ok_or_else(|| AtlasError::limit("lattice count overflowed"))?;
    if count * 4 > 96 * 1024 * 1024 {
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
    let source_mm = build_source_surface(model, controls.sea_level_mm, sdf, check_cancelled)?;
    let mut protected = vec![false; count];
    for j in 0..height {
        if (j as usize).is_multiple_of(CANCELLATION_STRIDE) {
            check_cancelled()?;
        }
        for i in 0..width {
            let lon = lattice_lon_micro(i, width);
            let lat = lattice_lat_micro(j, height);
            let cell = nearest_cell(controls.grid, lon, lat);
            protected[lattice_index(width, i, j)] = is_protected(hydrology, cell)
                && controls
                    .mountain_influence_ppm
                    .get(cell)
                    .copied()
                    .unwrap_or(0)
                    <= 0;
        }
    }
    let (filled_mm, filled_pit_count) = priority_fill(
        width,
        height,
        &source_mm,
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
    let tributaries = extract_tributaries(
        width,
        height,
        &filled_mm,
        &flow_primary,
        &accumulation,
        hydrology,
        &river_cells,
        &river_at,
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
        model.detail.level,
        &drainage_key,
        check_cancelled,
    )?;
    let worked_mm = erode(
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
        check_cancelled,
    )?;
    let deposition = extract_deposition(
        width,
        height,
        &worked_mm,
        &filled_mm,
        &flow_primary,
        &accumulation,
        hydrology,
        &protected,
        controls.sea_level_mm,
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
            bytes.extend_from_slice(&tributary.watershed_id.to_le_bytes());
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
            let watershed_id = read_u32(bytes, &mut offset)?;
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
                id: RefinedTributary::id_for(source_index),
                source_index,
                join_index,
                parent_river_id,
                watershed_id,
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
    use crate::amplify::build_amplification_model;
    use crate::cache::{decode_residual, encode_residual};
    use crate::control::ControlFields;
    use crate::detail::{
        domain_key, sample_field_mm, sample_sdf_ppm, signed_coastal_distance_ppm,
        COASTAL_ENVELOPE_PPM,
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
    use std::collections::BTreeSet;
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
        let next_version = ATLAS_DETAIL_ALGORITHM_VERSION.wrapping_add(1);
        let other = domain_key(
            b"identity-fixture",
            next_version,
            0,
            REFINED_DRAINAGE_DOMAIN,
        );
        assert_ne!(drainage, other);
        assert_ne!(drainage, erosion);
    }

    #[test]
    fn refinement_conserves_basins_watersheds_mouths_and_peaks() {
        let world = golden_world();
        let (model, controls, hydrology, sdf, identity) = fixture();
        let mut cancel = || Ok(());
        let started = Instant::now();
        let refined =
            build_refined_hydrology(&model, &controls, &hydrology, &sdf, &identity, &mut cancel)
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
        assert_eq!(ATLAS_DERIVED_DRAINAGE_VERSION, 1);
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
            assert!(tributary.id.starts_with(&format!(
                "atlas:tributary:v{}:",
                ATLAS_DERIVED_DRAINAGE_VERSION
            )));
            assert!(tributary.path.len() >= 3);
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
        for feature in &refined.deposition {
            assert!(feature.id.starts_with(&format!(
                "atlas:deposition:v{}:",
                ATLAS_DERIVED_DRAINAGE_VERSION
            )));
        }
        let river_ids = hydrology
            .rivers
            .iter()
            .map(|river| river.id)
            .collect::<BTreeSet<_>>();
        for tributary in &refined.tributaries {
            assert!(
                tributary.parent_river_id == 0 || river_ids.contains(&tributary.parent_river_id)
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
        let rebuilt =
            build_refined_hydrology(&model, &controls, &hydrology, &sdf, &identity, &mut cancel)
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
            max_mean <= i64::from(MAX_EROSION_STEP_MM) * i64::from(EROSION_SCALES.len() as u32),
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
        let err =
            build_refined_hydrology(&model, &controls, &hydrology, &sdf, &identity, &mut cancel)
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
        let err =
            build_refined_hydrology(&model, &controls, &hydrology, &sdf, &identity, &mut cancel)
                .unwrap_err();
        assert_eq!(err, AtlasError::cancelled());
        assert!(
            checks.get() > 3,
            "cancellation returned before mid-pipeline lattice checks"
        );
    }
}
