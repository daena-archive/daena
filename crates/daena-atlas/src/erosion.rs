//! Multi-scale fluvial, thermal, and deposition processes used during
//! hierarchical amplification and after the finest lattice exists.

use daena_physical::Grid;

use crate::detail::{
    lattice_lat_micro, lattice_lon_micro, lattice_sample, nearest_cell, sample_sdf_ppm,
    COASTAL_ENVELOPE_PPM,
};
use crate::AtlasError;

pub const REFINED_DRAINAGE_DOMAIN: &str = "refined-drainage";
pub const MULTI_SCALE_EROSION_DOMAIN: &str = "multi-scale-erosion";
pub const MAX_EROSION_STEP_MM: i32 = 18_000;
pub const HIERARCHICAL_EROSION_STEP_MM: i32 = 8_000;
pub const EROSION_SCALES: [u32; 3] = [4, 2, 1];
pub const HIERARCHICAL_SCALES: [u32; 1] = [1];
pub const THERMAL_SLOPE_PPM: i32 = 180_000;
pub const FAN_SLOPE_PPM: i32 = 40_000;
pub const FLOODPLAIN_SLOPE_PPM: i32 = 18_000;
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

pub fn lattice_index(width: u32, i: u32, j: u32) -> usize {
    j as usize * width as usize + i as usize
}

pub fn lock_polar_rows(width: u32, height: u32, field: &mut [i32]) {
    for j in [0, height.saturating_sub(1)] {
        let pole = field[lattice_index(width, 0, j)];
        for i in 1..width {
            field[lattice_index(width, i, j)] = pole;
        }
    }
}

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
    let ni = (i as i32 + dir.0).rem_euclid(width as i32) as u32;
    let nj = nj as u32;
    Some((ni, nj, lattice_index(width, ni, nj)))
}

fn dist_ppm(dir: (i32, i32)) -> i32 {
    if dir.0 == 0 || dir.1 == 0 {
        1_000
    } else {
        1_414
    }
}

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
    width: u32,
    height: u32,
    delta: &mut [i32],
    check_cancelled: &mut dyn FnMut() -> Result<(), AtlasError>,
) -> Result<(), AtlasError> {
    let mut sums = vec![0_i64; grid.sample_count()];
    let mut counts = vec![0_u32; grid.sample_count()];
    for j in 0..height {
        if j as usize % CANCELLATION_STRIDE == 0 {
            check_cancelled()?;
        }
        for i in 0..width {
            let cell = nearest_cell(
                grid,
                lattice_lon_micro(i, width),
                lattice_lat_micro(j, height),
            );
            sums[cell] += i64::from(delta[lattice_index(width, i, j)]);
            counts[cell] += 1;
        }
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
    for j in 0..height {
        for i in 0..width {
            let cell = nearest_cell(
                grid,
                lattice_lon_micro(i, width),
                lattice_lat_micro(j, height),
            );
            delta[lattice_index(width, i, j)] =
                (i64::from(delta[lattice_index(width, i, j)]) - means[cell]) as i32;
        }
    }
    Ok(())
}

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
    for j in 0..height {
        if j as usize % CANCELLATION_STRIDE == 0 {
            check_cancelled()?;
        }
        for i in 0..width {
            let index = lattice_index(width, i, j);
            if elevation_mm[index] < sea_level_mm {
                continue;
            }
            let watershed = watershed_id[index];
            if watershed < 0 {
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
                let neighbor_ocean = elevation_mm[neighbor] < sea_level_mm;
                if !neighbor_ocean && watershed_id[neighbor] != watershed {
                    continue;
                }
                let slope = ((i64::from(elevation_mm[index]) - i64::from(elevation_mm[neighbor]))
                    * 1_000)
                    / i64::from(dist_ppm(dir));
                candidates[dir_index] = Some((neighbor as u32, slope));
                if slope > best_slope || (slope == best_slope && (neighbor as u32) < best_neighbor)
                {
                    if slope > 0 {
                        best_slope = slope;
                        best_dir = dir_index;
                        best_neighbor = neighbor as u32;
                    }
                }
            }
            if best_neighbor == NO_FLOW {
                continue;
            }
            primary[index] = best_neighbor;
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
    order.sort_by_key(|index| (std::cmp::Reverse(elevation_mm[*index]), *index));
    let mut accum = vec![1_u32; count];
    for (rank, index) in order.iter().copied().enumerate() {
        if rank % CANCELLATION_STRIDE == 0 {
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

fn restore_coastal_sign(
    grid: Grid,
    width: u32,
    height: u32,
    sdf: &[i32],
    sea_level_mm: i32,
    protected: &[bool],
    source_or_canonical_land: impl Fn(i32, i32) -> bool,
    worked: &mut [i32],
) {
    for j in 0..height {
        for i in 0..width {
            let index = lattice_index(width, i, j);
            if protected[index] {
                continue;
            }
            let lon = lattice_lon_micro(i, width);
            let lat = lattice_lat_micro(j, height);
            let sdf_ppm = sample_sdf_ppm(grid, sdf, lon, lat);
            if sdf_ppm.unsigned_abs() <= COASTAL_ENVELOPE_PPM {
                continue;
            }
            let canon_land = source_or_canonical_land(lon, lat);
            let worked_land = worked[index] >= sea_level_mm;
            if canon_land != worked_land {
                worked[index] = if canon_land {
                    sea_level_mm.saturating_add(1)
                } else {
                    sea_level_mm.saturating_sub(1)
                };
            }
        }
    }
}

fn thermal_delta(
    width: u32,
    height: u32,
    worked: &[i32],
    protected: &[bool],
    sea_level_mm: i32,
    mountain_ppm: &[i32],
    max_step_mm: i32,
) -> Vec<i32> {
    let count = worked.len();
    let mut delta = vec![0_i32; count];
    for j in 1..height.saturating_sub(1) {
        for i in 0..width {
            let index = lattice_index(width, i, j);
            if protected[index] || worked[index] < sea_level_mm || mountain_ppm[index] > 500_000 {
                continue;
            }
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
            if dest == NO_FLOW || steepest < i64::from(THERMAL_SLOPE_PPM) {
                continue;
            }
            let flux = ((steepest - i64::from(THERMAL_SLOPE_PPM)) / 20_000)
                .clamp(0, i64::from(max_step_mm / 2)) as i32;
            if flux == 0 {
                continue;
            }
            delta[index] = delta[index].saturating_sub(flux);
            delta[dest as usize] = delta[dest as usize].saturating_add(flux);
        }
    }
    delta
}

fn fluvial_and_deposition_delta(
    width: u32,
    height: u32,
    worked: &[i32],
    protected: &[bool],
    mountain_ppm: &[i32],
    runoff_ppm: &[i32],
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
    let mut delta = vec![0_i32; count];
    for j in 1..height.saturating_sub(1) {
        for i in 0..width {
            let index = lattice_index(width, i, j);
            if protected[index] || worked[index] < sea_level_mm || mountain_ppm[index] > 500_000 {
                continue;
            }
            let dest = walk_flow(primary, index, scale, count);
            if dest == NO_FLOW || dest as usize >= count || dest == index as u32 {
                continue;
            }
            if mountain_ppm[dest as usize] > 500_000 {
                continue;
            }
            let drop = (worked[index] - worked[dest as usize]).max(0);
            if drop == 0 {
                continue;
            }
            let damp = 1_000_000 - mountain_ppm[index] / 2;
            let runoff = runoff_ppm[index].clamp(250_000, 1_250_000);
            let prf = lattice_sample(erosion_key, i, j, scale);
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
                continue;
            }
            delta[index] = delta[index].saturating_sub(flux);
            let hops = scale.max(1) as i64;
            let slope_ppm = (i64::from(drop) * 1_000) / hops;
            let mut deposit = flux;
            if slope_ppm < i64::from(FAN_SLOPE_PPM) && mountain_ppm[index] > 80_000 {
                deposit = flux;
            }
            if slope_ppm < i64::from(FLOODPLAIN_SLOPE_PPM) {
                deposit = flux;
            }
            let share = ((i64::from(deposit) * i64::from(weight[index])) / 1_000_000) as i32;
            delta[dest as usize] = delta[dest as usize].saturating_add(share);
            let rest = deposit.saturating_sub(share);
            if rest > 0 && secondary[index] != NO_FLOW && (secondary[index] as usize) < count {
                delta[secondary[index] as usize] =
                    delta[secondary[index] as usize].saturating_add(rest);
            } else {
                delta[dest as usize] = delta[dest as usize].saturating_add(rest);
            }
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
    pub primary: &'a [u32],
    pub secondary: &'a [u32],
    pub weight: &'a [u32],
    pub accumulation: &'a [u32],
    pub erosion_key: &'a [u8; 32],
    pub peaks: &'a [usize],
    pub filled_mm: &'a [i32],
    pub land_at: &'a dyn Fn(i32, i32) -> bool,
    pub scales: &'a [u32],
    pub max_step_mm: i32,
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
            params.max_step_mm,
        );
        for index in 0..count {
            delta[index] = delta[index].saturating_add(thermal[index]);
        }
        mean_remove_delta(params.grid, width, height, &mut delta, check_cancelled)?;
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
        for index in 0..count {
            if params.protected[index] {
                surface[index] = params.filled_mm[index];
            }
        }
    }
    Ok(())
}
