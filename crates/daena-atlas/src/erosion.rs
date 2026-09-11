//! Multi-scale fluvial, thermal, arid, and glacial processes on the structure
//! lattice. Magnitudes come from climate state at `t` (and lagged ice), not `|t|`.

use std::cmp::Reverse;
use std::collections::BinaryHeap;

use daena_physical::Grid;
use rayon::prelude::*;

use crate::detail::{
    lattice_lat_micro, lattice_lon_micro, lattice_sample, nest_lattice_coord, sample_sdf_ppm,
    COASTAL_ENVELOPE_PPM,
};
use crate::AtlasError;

pub const REFINED_DRAINAGE_DOMAIN: &str = "refined-drainage";
pub const MULTI_SCALE_EROSION_DOMAIN: &str = "multi-scale-erosion";
pub const MAX_EROSION_STEP_MM: i32 = 18_000;
pub const HIERARCHICAL_EROSION_STEP_MM: i32 = 8_000;
pub const HIERARCHICAL_FILL_MM: i32 = 720_000;
pub const EROSION_SCALES: [u32; 3] = [4, 2, 1];
pub const HIERARCHICAL_SCALES: [u32; 1] = [1];
pub const THERMAL_SLOPE_PPM: i32 = 180_000;
pub const FAN_SLOPE_PPM: i32 = 40_000;
pub const FLOODPLAIN_SLOPE_PPM: i32 = 18_000;
const PRECIP_REF_MM: i32 = 4_000;
const ICE_REF_MM: i32 = 400_000;
const CANCELLATION_STRIDE: usize = 4_096;
pub const NO_FLOW: u32 = u32::MAX;
pub const DIRS: [(i32, i32); 8] = [
    (1, 0),
    (1, 1),
    (0, 1),
    (-1, 1),
    (-1, 0),
    (-1, -1),
    (0, -1),
    (1, -1),
];

#[must_use]
pub fn lattice_index(width: u32, i: u32, j: u32) -> usize {
    j as usize * width as usize + i as usize
}

pub fn priority_fill_pits(
    width: u32,
    height: u32,
    source_mm: &[i32],
    protected: &[bool],
    sea_level_mm: i32,
    max_fill_mm: i32,
    check_cancelled: &mut dyn FnMut() -> Result<(), AtlasError>,
) -> Result<Vec<i32>, AtlasError> {
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
        return Ok(routing);
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
                let capped = source_mm[neighbor].saturating_add(max_fill_mm);
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
    Ok(routing)
}

pub fn lock_polar_rows(width: u32, height: u32, field: &mut [i32]) {
    for j in [0, height.saturating_sub(1)] {
        let pole = field[lattice_index(width, 0, j)];
        for i in 1..width {
            field[lattice_index(width, i, j)] = pole;
        }
    }
}

#[must_use]
pub fn neighbor_at(
    width: u32,
    height: u32,
    i: u32,
    j: u32,
    dir: (i32, i32),
) -> Option<(u32, u32, usize)> {
    let nj = j as i32 + dir.1;
    if nj < 0 || nj >= height as i32 {
        return None;
    }
    let width_i = width as i32;
    let mut ni = i as i32 + dir.0;
    if ni < 0 {
        ni += width_i;
    } else if ni >= width_i {
        ni -= width_i;
    }
    let ni = ni as u32;
    let nj = nj as u32;
    Some((ni, nj, lattice_index(width, ni, nj)))
}

#[must_use]
pub fn vegetation_resistance_ppm(humidity_ppm: i32, precip_mm: i32, temp_centi: i32) -> i32 {
    let wet = (i64::from(precip_mm.clamp(0, PRECIP_REF_MM)) * 1_000_000) / i64::from(PRECIP_REF_MM);
    let humid = i64::from(humidity_ppm.clamp(0, 1_000_000));
    let comfort = if temp_centi <= -500 || temp_centi >= 4_000 {
        0
    } else if temp_centi < 1_000 {
        i64::from(temp_centi + 500) * 1_000_000 / 1_500
    } else if temp_centi <= 2_500 {
        1_000_000
    } else {
        i64::from(4_000 - temp_centi) * 1_000_000 / 1_500
    };
    ((wet * humid / 1_000_000) * comfort / 1_000_000) as i32
}

#[must_use]
pub fn freeze_thaw_ppm(summer_centi: i32, winter_centi: i32) -> i32 {
    if winter_centi < 0 && summer_centi > 0 {
        1_000_000
    } else if summer_centi <= 0 {
        450_000
    } else if winter_centi <= 200 {
        600_000
    } else {
        100_000
    }
}

#[must_use]
pub fn glacial_work_ppm(ice_mm: i32, summer_centi: i32) -> i32 {
    if ice_mm <= 0 {
        return 0;
    }
    let ice = (i64::from(ice_mm.clamp(0, ICE_REF_MM)) * 1_000_000) / i64::from(ICE_REF_MM);
    let cold = if summer_centi <= 0 {
        1_000_000
    } else if summer_centi >= 1_200 {
        0
    } else {
        i64::from(1_200 - summer_centi) * 1_000_000 / 1_200
    };
    (ice * cold / 1_000_000) as i32
}

#[must_use]
pub fn fluvial_gain_ppm(runoff_mm: i32, precip_mm: i32, vegetation_ppm: i32) -> i32 {
    let runoff =
        (i64::from(runoff_mm.clamp(0, PRECIP_REF_MM)) * 1_000_000) / i64::from(PRECIP_REF_MM);
    let precip =
        (i64::from(precip_mm.clamp(0, PRECIP_REF_MM)) * 1_000_000) / i64::from(PRECIP_REF_MM);
    let resist = 1_000_000 - i64::from(vegetation_ppm.clamp(0, 1_000_000)) / 2;
    let mixed = ((runoff + precip) / 2) * resist / 1_000_000;
    mixed.clamp(0, 1_250_000) as i32
}

fn dist_ppm(dir: (i32, i32)) -> i32 {
    if dir.0 == 0 || dir.1 == 0 {
        1_000
    } else {
        1_414
    }
}

#[must_use]
pub fn walk_flow(primary: &[u32], start: usize, hops: u32, count: usize) -> u32 {
    let mut dest = start as u32;
    for _ in 0..hops {
        let next = primary[dest as usize];
        if next == NO_FLOW || next as usize >= count || next == dest {
            break;
        }
        dest = next;
    }
    dest
}

pub fn mean_remove_delta(
    grid: Grid,
    cells: &[usize],
    delta: &mut [i32],
    check_cancelled: &mut dyn FnMut() -> Result<(), AtlasError>,
) -> Result<(), AtlasError> {
    let mut sums = vec![0_i64; grid.sample_count()];
    let mut counts = vec![0_u32; grid.sample_count()];
    check_cancelled()?;
    for (index, &cell) in cells.iter().enumerate() {
        sums[cell] += i64::from(delta[index]);
        counts[cell] += 1;
    }
    let means = sums
        .iter()
        .zip(counts)
        .map(|(sum, count)| {
            if count == 0 {
                0
            } else {
                sum / i64::from(count)
            }
        })
        .collect::<Vec<_>>();
    delta.par_iter_mut().enumerate().for_each(|(index, slot)| {
        *slot = (i64::from(*slot) - means[cells[index]]) as i32;
    });
    check_cancelled()?;
    Ok(())
}

#[allow(clippy::type_complexity)]
pub fn assign_simple_flow(
    width: u32,
    height: u32,
    elevation_mm: &[i32],
    watershed_id: &[i32],
    sea_level_mm: i32,
    check_cancelled: &mut dyn FnMut() -> Result<(), AtlasError>,
) -> Result<(Vec<u32>, Vec<u32>, Vec<u32>), AtlasError> {
    let count = elevation_mm.len();
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
            if elevation_mm[index] < sea_level_mm {
                return;
            }
            let watershed = watershed_id[index];
            if watershed < 0 {
                return;
            }
            let i = (index % width_us) as u32;
            let j = (index / width_us) as u32;
            let mut best_dir = 8_usize;
            let mut best_slope = 0_i64;
            let mut best_neighbor = NO_FLOW;
            let mut candidates = [None; 8];
            for (dir_index, dir) in DIRS.iter().copied().enumerate() {
                let Some((_, _, neighbor)) = neighbor_at(width, height, i, j, dir) else {
                    continue;
                };
                let neighbor_ocean = elevation_mm[neighbor] < sea_level_mm;
                if !neighbor_ocean && watershed_id[neighbor] != watershed {
                    continue;
                }
                let slope = ((i64::from(elevation_mm[index]) - i64::from(elevation_mm[neighbor]))
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
                return;
            }
            *prim = best_neighbor;
            let left = (best_dir + 7) % 8;
            let right = (best_dir + 1) % 8;
            let side = [candidates[left], candidates[right]]
                .into_iter()
                .flatten()
                .filter(|(_, slope)| *slope > 0)
                .max_by_key(|(neighbor, slope)| (*slope, std::cmp::Reverse(*neighbor)));
            if let Some((neighbor, slope)) = side {
                let total = best_slope + slope;
                if total > 0 {
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

pub fn accumulate_flow(
    elevation_mm: &[i32],
    primary: &[u32],
    secondary: &[u32],
    weight: &[u32],
    sea_level_mm: i32,
    check_cancelled: &mut dyn FnMut() -> Result<(), AtlasError>,
) -> Result<Vec<u32>, AtlasError> {
    let count = elevation_mm.len();
    let mut order = (0..count).collect::<Vec<_>>();
    order.par_sort_unstable_by_key(|index| (std::cmp::Reverse(elevation_mm[*index]), *index));
    let mut accum = vec![1_u32; count];
    for (rank, index) in order.iter().copied().enumerate() {
        if rank.is_multiple_of(CANCELLATION_STRIDE) {
            check_cancelled()?;
        }
        if elevation_mm[index] < sea_level_mm {
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
fn restore_coastal_sign(
    grid: Grid,
    width: u32,
    height: u32,
    sdf: &[i32],
    sea_level_mm: i32,
    protected: &[bool],
    source_or_canonical_land: impl Fn(i32, i32) -> bool + Sync,
    worked: &mut [i32],
) {
    let width_us = width as usize;
    worked.par_iter_mut().enumerate().for_each(|(index, slot)| {
        if protected[index] {
            return;
        }
        let i = (index % width_us) as u32;
        let j = (index / width_us) as u32;
        let lon = lattice_lon_micro(i, width);
        let lat = lattice_lat_micro(j, height);
        let sdf_ppm = sample_sdf_ppm(grid, sdf, lon, lat);
        if sdf_ppm.unsigned_abs() <= COASTAL_ENVELOPE_PPM {
            return;
        }
        let canon_land = source_or_canonical_land(lon, lat);
        let worked_land = *slot >= sea_level_mm;
        if canon_land != worked_land {
            *slot = if canon_land {
                sea_level_mm.saturating_add(1)
            } else {
                sea_level_mm.saturating_sub(1)
            };
        }
    });
}

#[allow(clippy::too_many_arguments)]
fn thermal_delta(
    width: u32,
    height: u32,
    worked: &[i32],
    protected: &[bool],
    sea_level_mm: i32,
    mountain_ppm: &[i32],
    freeze_thaw_ppm: &[i32],
    aridity_ppm: &[i32],
    glacial_ppm: &[i32],
    max_step_mm: i32,
) -> Vec<i32> {
    let count = worked.len();
    let width_us = width as usize;
    let mut dests = vec![NO_FLOW; count];
    let mut fluxes = vec![0_i32; count];
    dests
        .par_iter_mut()
        .zip(fluxes.par_iter_mut())
        .enumerate()
        .for_each(|(index, (dest_slot, flux_slot))| {
            let j = (index / width_us) as u32;
            if j == 0 || j + 1 == height {
                return;
            }
            if protected[index] || worked[index] < sea_level_mm {
                return;
            }
            let ice = i64::from(glacial_ppm[index].clamp(0, 1_000_000));
            let mountain = mountain_ppm[index] > 500_000;
            if mountain && ice == 0 {
                return;
            }
            let i = (index % width_us) as u32;
            let mut steepest = 0_i64;
            let mut dest = NO_FLOW;
            for dir in DIRS {
                let Some((_, _, neighbor)) = neighbor_at(width, height, i, j, dir) else {
                    continue;
                };
                if protected[neighbor] {
                    continue;
                }
                let slope = ((i64::from(worked[index]) - i64::from(worked[neighbor])) * 1_000_000)
                    / i64::from(dist_ppm(dir));
                if slope > steepest || (slope == steepest && (neighbor as u32) < dest) {
                    steepest = slope;
                    dest = neighbor as u32;
                }
            }
            if dest == NO_FLOW {
                return;
            }
            let frost = i64::from(freeze_thaw_ppm[index].clamp(0, 1_000_000));
            let arid = i64::from(aridity_ppm[index].clamp(0, 1_000_000));
            let mut flux = 0_i64;
            if !mountain {
                if steepest >= i64::from(THERMAL_SLOPE_PPM) {
                    flux +=
                        ((steepest - i64::from(THERMAL_SLOPE_PPM)) / 20_000) * frost / 1_000_000;
                }
                if steepest >= i64::from(FAN_SLOPE_PPM) && arid > 350_000 {
                    flux += ((steepest - i64::from(FAN_SLOPE_PPM)) / 40_000) * arid / 1_000_000;
                }
            }
            if ice > 0 && steepest >= i64::from(THERMAL_SLOPE_PPM) / 4 {
                flux += ((steepest - i64::from(THERMAL_SLOPE_PPM) / 4) / 12_000) * ice / 1_000_000;
            }
            let flux = flux.clamp(0, i64::from(max_step_mm / 2)) as i32;
            if flux == 0 {
                return;
            }
            *dest_slot = dest;
            *flux_slot = flux;
        });
    let mut delta = vec![0_i32; count];
    for index in 0..count {
        let flux = fluxes[index];
        if flux == 0 {
            continue;
        }
        delta[index] = delta[index].saturating_sub(flux);
        let dest = dests[index] as usize;
        if dest < count {
            delta[dest] = delta[dest].saturating_add(flux);
        }
    }
    delta
}

#[allow(clippy::too_many_arguments)]
fn fluvial_and_deposition_delta(
    width: u32,
    height: u32,
    worked: &[i32],
    protected: &[bool],
    mountain_ppm: &[i32],
    runoff_ppm: &[i32],
    aridity_ppm: &[i32],
    glacial_ppm: &[i32],
    primary: &[u32],
    secondary: &[u32],
    weight: &[u32],
    accumulation: &[u32],
    erosion_key: &[u8; 32],
    sea_level_mm: i32,
    scale: u32,
    max_step_mm: i32,
) -> Vec<i32> {
    let count = worked.len();
    let width_us = width as usize;
    let mut ice_dests = vec![NO_FLOW; count];
    let mut ice_extras = vec![0_i32; count];
    let mut dests = vec![NO_FLOW; count];
    let mut shares = vec![0_i32; count];
    let mut rest_dests = vec![NO_FLOW; count];
    let mut rests = vec![0_i32; count];
    ice_dests
        .par_iter_mut()
        .zip(ice_extras.par_iter_mut())
        .zip(dests.par_iter_mut())
        .zip(shares.par_iter_mut())
        .zip(rest_dests.par_iter_mut())
        .zip(rests.par_iter_mut())
        .enumerate()
        .for_each(
            |(
                index,
                (((((ice_dest, ice_extra), dest_slot), share_slot), rest_dest), rest_slot),
            )| {
                let j = (index / width_us) as u32;
                if j == 0 || j + 1 == height {
                    return;
                }
                if protected[index] || worked[index] < sea_level_mm {
                    return;
                }
                let ice = glacial_ppm[index].clamp(0, 1_000_000);
                if ice > 0 {
                    let ice_to = walk_flow(primary, index, scale.saturating_mul(2).max(1), count);
                    if ice_to != NO_FLOW && (ice_to as usize) < count && ice_to != index as u32 {
                        let drop = (worked[index] - worked[ice_to as usize]).max(0);
                        let extra = ((i64::from(drop.min(max_step_mm / 4)) * i64::from(ice))
                            / 1_000_000)
                            .clamp(0, i64::from(max_step_mm / 4))
                            as i32;
                        if extra > 0 {
                            *ice_dest = ice_to;
                            *ice_extra = extra;
                        }
                    }
                }
                if mountain_ppm[index] > 500_000 {
                    return;
                }
                let hops = if aridity_ppm[index] > 400_000 {
                    scale.max(2) / 2
                } else {
                    scale
                }
                .max(1);
                let dest = walk_flow(primary, index, hops, count);
                if dest == NO_FLOW || dest as usize >= count || dest == index as u32 {
                    return;
                }
                if mountain_ppm[dest as usize] > 500_000 {
                    return;
                }
                let drop = (worked[index] - worked[dest as usize]).max(0);
                if drop == 0 {
                    return;
                }
                let i = (index % width_us) as u32;
                let damp = 1_000_000 - mountain_ppm[index] / 2;
                let runoff = runoff_ppm[index].clamp(0, 1_250_000);
                let prf = lattice_sample(
                    erosion_key,
                    nest_lattice_coord(i, width),
                    nest_lattice_coord(j, height),
                    scale,
                );
                let prf_damp = 1_000_000 - ((prf >> 11) % 25_000) as i32;
                let accum = accumulation[index].max(1);
                let flux = ((i64::from(drop.min(max_step_mm))
                    * i64::from(accum.min(64))
                    * i64::from(damp)
                    * i64::from(prf_damp)
                    / (64 * 1_000_000 * 1_000_000))
                    * i64::from(runoff)
                    / 1_000_000)
                    .clamp(0, i64::from(max_step_mm)) as i32;
                if flux == 0 {
                    return;
                }
                let share = ((i64::from(flux) * i64::from(weight[index])) / 1_000_000) as i32;
                let rest = flux.saturating_sub(share);
                *dest_slot = dest;
                *share_slot = share;
                *rest_slot = rest;
                if rest > 0 && secondary[index] != NO_FLOW && (secondary[index] as usize) < count {
                    *rest_dest = secondary[index];
                } else {
                    *rest_dest = dest;
                }
            },
        );
    let mut delta = vec![0_i32; count];
    for index in 0..count {
        let extra = ice_extras[index];
        if extra != 0 {
            delta[index] = delta[index].saturating_sub(extra);
            let dest = ice_dests[index] as usize;
            if dest < count {
                delta[dest] = delta[dest].saturating_add(extra);
            }
        }
        let share = shares[index];
        let rest = rests[index];
        let flux = share.saturating_add(rest);
        if flux == 0 {
            continue;
        }
        delta[index] = delta[index].saturating_sub(flux);
        let dest = dests[index] as usize;
        if dest < count {
            delta[dest] = delta[dest].saturating_add(share);
        }
        let rest_dest = rest_dests[index] as usize;
        if rest > 0 && rest_dest < count {
            delta[rest_dest] = delta[rest_dest].saturating_add(rest);
        } else if rest > 0 && dest < count {
            delta[dest] = delta[dest].saturating_add(rest);
        }
    }
    delta
}

pub struct ScaleErosion<'a> {
    pub grid: Grid,
    pub width: u32,
    pub height: u32,
    pub sea_level_mm: i32,
    pub sdf: &'a [i32],
    pub protected: &'a [bool],
    pub mountain_ppm: &'a [i32],
    pub runoff_ppm: &'a [i32],
    pub freeze_thaw_ppm: &'a [i32],
    pub aridity_ppm: &'a [i32],
    pub glacial_ppm: &'a [i32],
    pub primary: &'a [u32],
    pub secondary: &'a [u32],
    pub weight: &'a [u32],
    pub accumulation: &'a [u32],
    pub erosion_key: &'a [u8; 32],
    pub peaks: &'a [usize],
    pub filled_mm: &'a [i32],
    pub land_at: &'a (dyn Fn(i32, i32) -> bool + Sync),
    pub scales: &'a [u32],
    pub max_step_mm: i32,
    pub cells: &'a [usize],
}

fn enforce_peaks(width: u32, height: u32, peaks: &[usize], filled_mm: &[i32], worked: &mut [i32]) {
    let count = worked.len();
    for &peak in peaks {
        if peak >= count {
            continue;
        }
        let j = peak as u32 / width;
        let i = peak as u32 % width;
        if j == 0 || j + 1 == height {
            continue;
        }
        let mut required = worked[peak].max(filled_mm[peak]);
        for dir in DIRS {
            if let Some((_, nj, neighbor)) = neighbor_at(width, height, i, j, dir) {
                if nj == 0 || nj + 1 == height {
                    continue;
                }
                required = required.max(worked[neighbor].saturating_sub(MAX_EROSION_STEP_MM));
            }
        }
        worked[peak] = required;
    }
}

pub fn apply_scale_erosion(
    params: ScaleErosion<'_>,
    surface: &mut [i32],
    check_cancelled: &mut dyn FnMut() -> Result<(), AtlasError>,
) -> Result<(), AtlasError> {
    let width = params.width;
    let height = params.height;
    let count = surface.len();
    for &scale in params.scales {
        check_cancelled()?;
        let mut delta = fluvial_and_deposition_delta(
            width,
            height,
            surface,
            params.protected,
            params.mountain_ppm,
            params.runoff_ppm,
            params.aridity_ppm,
            params.glacial_ppm,
            params.primary,
            params.secondary,
            params.weight,
            params.accumulation,
            params.erosion_key,
            params.sea_level_mm,
            scale,
            params.max_step_mm,
        );
        let thermal = thermal_delta(
            width,
            height,
            surface,
            params.protected,
            params.sea_level_mm,
            params.mountain_ppm,
            params.freeze_thaw_ppm,
            params.aridity_ppm,
            params.glacial_ppm,
            params.max_step_mm,
        );
        for index in 0..count {
            delta[index] = delta[index].saturating_add(thermal[index]);
        }
        mean_remove_delta(params.grid, params.cells, &mut delta, check_cancelled)?;
        for index in 0..count {
            if params.protected[index] {
                surface[index] = params.filled_mm[index];
                continue;
            }
            surface[index] = surface[index].saturating_add(delta[index]);
        }
        restore_coastal_sign(
            params.grid,
            width,
            height,
            params.sdf,
            params.sea_level_mm,
            params.protected,
            params.land_at,
            surface,
        );
        enforce_peaks(width, height, params.peaks, params.filled_mm, surface);
        lock_polar_rows(width, height, surface);
        for (index, protected) in params.protected.iter().enumerate() {
            if *protected {
                surface[index] = params.filled_mm[index];
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use daena_physical::Grid;

    fn ramp_params(
        count: usize,
        width: u32,
        height: u32,
    ) -> (Vec<i32>, Vec<u32>, Vec<u32>, Vec<u32>) {
        let mut filled = vec![0_i32; count];
        let mut primary = vec![NO_FLOW; count];
        let mut weight = vec![0_u32; count];
        let mut accumulation = vec![1_u32; count];
        for j in 0..height {
            for i in 0..width {
                let index = lattice_index(width, i, j);
                filled[index] = 80_000 + (height as i32 - 1 - j as i32) * 40_000;
                if j + 1 < height {
                    primary[index] = lattice_index(width, i, j + 1) as u32;
                    weight[index] = 1_000_000;
                }
                accumulation[index] = 16;
            }
        }
        (filled, primary, weight, accumulation)
    }

    #[test]
    fn vegetation_and_glacial_gains_track_climate_state_not_year() {
        let lush = vegetation_resistance_ppm(800_000, 2_000, 1_800);
        let desert = vegetation_resistance_ppm(80_000, 80, 3_200);
        let frozen = vegetation_resistance_ppm(400_000, 800, -1_200);
        assert!(lush > desert);
        assert!(lush > frozen);
        assert_eq!(freeze_thaw_ppm(1_200, -800), 1_000_000);
        assert!(freeze_thaw_ppm(2_400, 1_800) < freeze_thaw_ppm(1_200, -800));
        assert!(glacial_work_ppm(200_000, -200) > glacial_work_ppm(200_000, 1_500));
        assert_eq!(glacial_work_ppm(0, -400), 0);
        let wet = fluvial_gain_ppm(2_000, 2_400, desert);
        let dry = fluvial_gain_ppm(200, 80, lush);
        assert!(wet > dry);
        assert!(dry < 80_000);
    }

    #[test]
    fn climate_state_changes_work_on_fixed_surface() {
        let grid = Grid {
            width: 4,
            height: 2,
            radius_metres: daena_physical::DEFAULT_RADIUS_METRES,
        };
        let width = 8_u32;
        let height = 4_u32;
        let count = 32;
        let (filled, primary, weight, accumulation) = ramp_params(count, width, height);
        let protected = vec![false; count];
        let mountain = vec![0_i32; count];
        let sdf = vec![1_000_000_i32; grid.sample_count()];
        let secondary = vec![NO_FLOW; count];
        let key = [7_u8; 32];
        let cells = crate::detail::lattice_nearest_cells(grid, width, height);
        let erode = |runoff: i32, frost: i32, arid: i32, ice: i32, mountains: &[i32]| {
            let mut surface = filled.clone();
            let runoff_ppm = vec![runoff; count];
            let freeze = vec![frost; count];
            let aridity = vec![arid; count];
            let glacial = vec![ice; count];
            let land_at = |_lon: i32, _lat: i32| true;
            let mut cancel = || Ok(());
            apply_scale_erosion(
                ScaleErosion {
                    grid,
                    width,
                    height,
                    sea_level_mm: 0,
                    sdf: &sdf,
                    protected: &protected,
                    mountain_ppm: mountains,
                    runoff_ppm: &runoff_ppm,
                    freeze_thaw_ppm: &freeze,
                    aridity_ppm: &aridity,
                    glacial_ppm: &glacial,
                    primary: &primary,
                    secondary: &secondary,
                    weight: &weight,
                    accumulation: &accumulation,
                    erosion_key: &key,
                    peaks: &[],
                    filled_mm: &filled,
                    land_at: &land_at,
                    scales: &[1],
                    max_step_mm: MAX_EROSION_STEP_MM,
                    cells: &cells,
                },
                &mut surface,
                &mut cancel,
            )
            .unwrap();
            surface
        };
        let wet = erode(1_000_000, 100_000, 0, 0, &mountain);
        let dry = erode(80_000, 100_000, 800_000, 0, &mountain);
        let cold = erode(200_000, 1_000_000, 0, 800_000, &mountain);
        assert_ne!(wet, dry);
        assert_ne!(cold, dry);
        let high = vec![600_000_i32; count];
        let iced = erode(0, 0, 0, 800_000, &high);
        let bare = erode(0, 0, 0, 0, &high);
        assert_eq!(bare, filled);
        assert_ne!(iced, filled);
    }
}
