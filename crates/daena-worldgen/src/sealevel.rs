//! Sea level, the elevation shift, and ocean connectivity.
//!
//! `S` is chosen so the area with elevation strictly above `S` matches the
//! requested land fraction. The scan walks cells from high to low. If `S` is
//! negative the surface is left unshifted and the achieved fraction is the
//! area that already stands above 0 m.

use std::collections::VecDeque;

use rayon::prelude::*;

use crate::grid::{CubeSphere, NONE};
use crate::WorldError;

/// One `(sea level, ocean volume)` sample on the cube grid.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OceanVolumeSample {
    /// Level in millimetres relative to the reference sea level.
    pub sea_level_mm: i32,
    /// Volume below the level, cubic metres.
    pub volume_m3: u64,
}

/// Water volume below `sea_level_mm`, over every cell, in cubic metres.
pub fn ocean_volume_at(
    grid: &CubeSphere,
    elevation_mm: &[i32],
    sea_level_mm: i32,
) -> Result<u64, WorldError> {
    if elevation_mm.len() != grid.cell_count() {
        return Err(WorldError::Internal(
            "ocean volume input does not match the grid".to_string(),
        ));
    }
    let mut volume = 0.0f64;
    for (cell, height) in elevation_mm.iter().enumerate() {
        let depth_mm = i64::from(sea_level_mm) - i64::from(*height);
        if depth_mm > 0 {
            volume += grid.area_m2(cell as u32) * depth_mm as f64 / 1000.0;
        }
    }
    Ok(volume.round().clamp(0.0, u64::MAX as f64) as u64)
}

/// Monotonic samples from the lowest to the highest cell elevation.
///
/// The endpoints are one millimetre outside the terrain, so a solve can clamp
/// to an empty or fully drowned planet without extrapolating.
pub fn ocean_volume_curve(
    grid: &CubeSphere,
    elevation_mm: &[i32],
) -> Result<Vec<OceanVolumeSample>, WorldError> {
    if elevation_mm.len() != grid.cell_count() {
        return Err(WorldError::Internal(
            "ocean volume input does not match the grid".to_string(),
        ));
    }
    let Some(minimum) = elevation_mm.iter().copied().min() else {
        return Ok(Vec::new());
    };
    let maximum = elevation_mm.iter().copied().max().unwrap_or(minimum);
    let mut order: Vec<u32> = (0..elevation_mm.len() as u32).collect();
    order.sort_unstable_by_key(|cell| (elevation_mm[*cell as usize], *cell));

    let mut samples = Vec::with_capacity(order.len() + 2);
    samples.push(OceanVolumeSample {
        sea_level_mm: minimum.saturating_sub(1),
        volume_m3: 0,
    });
    let mut below_area = 0.0f64;
    let mut below_area_height = 0.0f64;
    let mut index = 0usize;
    while index < order.len() {
        let level = elevation_mm[order[index] as usize];
        let volume = (below_area * f64::from(level) - below_area_height).max(0.0) / 1000.0;
        samples.push(OceanVolumeSample {
            sea_level_mm: level,
            volume_m3: volume.round().clamp(0.0, u64::MAX as f64) as u64,
        });
        while index < order.len() && elevation_mm[order[index] as usize] == level {
            let cell = order[index];
            let area = grid.area_m2(cell);
            below_area += area;
            below_area_height += area * f64::from(level);
            index += 1;
        }
    }
    let top = maximum.saturating_add(1);
    let volume = (below_area * f64::from(top) - below_area_height).max(0.0) / 1000.0;
    samples.push(OceanVolumeSample {
        sea_level_mm: top,
        volume_m3: volume.round().clamp(0.0, u64::MAX as f64) as u64,
    });
    if !samples.iter().any(|sample| sample.sea_level_mm == 0) {
        samples.push(OceanVolumeSample {
            sea_level_mm: 0,
            volume_m3: ocean_volume_at(grid, elevation_mm, 0)?,
        });
        samples.sort_unstable_by_key(|sample| sample.sea_level_mm);
    }
    Ok(samples)
}

/// Solve the sea level whose ocean volume equals `inventory_m3`.
///
/// The reference inventory maps to `0` mm exactly. Otherwise the solver
/// interpolates inside the bracketing elevation segment, rounds to the nearest
/// millimetre, ties toward the lower level, and clamps to that bracket.
pub fn sea_level_for_ocean_volume(
    curve: &[OceanVolumeSample],
    inventory_m3: u64,
) -> Result<i32, WorldError> {
    if curve.is_empty() {
        return Err(WorldError::Internal(
            "ocean volume curve is empty".to_string(),
        ));
    }
    // The reference inventory is exactly `V(0)`. The curve may contain a
    // lower sampled level with the same volume, so keep the zero datum when
    // it is the exact target.
    if let Some(sample) = curve
        .iter()
        .find(|sample| sample.sea_level_mm == 0 && sample.volume_m3 == inventory_m3)
    {
        return Ok(sample.sea_level_mm);
    }
    let mut low = 0usize;
    let mut high = curve.len();
    while low < high {
        let mid = (low + high) / 2;
        if curve[mid].volume_m3 >= inventory_m3 {
            high = mid;
        } else {
            low = mid + 1;
        }
    }
    if low == 0 {
        return Ok(curve[0].sea_level_mm);
    }
    if low >= curve.len() {
        return Ok(curve[curve.len() - 1].sea_level_mm);
    }
    let left = curve[low - 1];
    let right = curve[low];
    if right.volume_m3 == left.volume_m3 {
        return Ok(left.sea_level_mm);
    }
    let fraction =
        (inventory_m3 - left.volume_m3) as f64 / (right.volume_m3 - left.volume_m3) as f64;
    let exact =
        f64::from(left.sea_level_mm) + fraction * f64::from(right.sea_level_mm - left.sea_level_mm);
    let lower = exact.floor();
    let rounded = if exact - lower <= 0.5 {
        lower
    } else {
        lower + 1.0
    };
    let clamped = rounded.clamp(f64::from(left.sea_level_mm), f64::from(right.sea_level_mm));
    Ok(clamped as i32)
}

/// Outcome of the sea-level solve. Elevations in the caller's buffer are shifted.
pub(crate) struct SeaLevel {
    /// Metres added to every cell. `0` when the solved level was below 0.
    pub shift_m: f32,
    /// Area fraction with elevation strictly above 0 after the shift.
    pub achieved_land_fraction: f64,
    /// Domain cells at or below 0 m that the ocean flood did not reach.
    pub dry_depressions: u32,
    /// `1` where the cell is ocean at or below sea level.
    pub ocean: Vec<u8>,
}

/// Find the sea-level threshold and shift `elevation` by `-S` when `S >= 0`.
///
/// The caller may rewrite `elevation` between this step and [`finish`], for
/// example to canonicalise it to whole millimetres. The shift stays valid
/// because the sea-level datum does not move.
pub(crate) fn apply_threshold(
    grid: &CubeSphere,
    elevation: &mut [f32],
    land_fraction: f64,
    cancelled: &dyn Fn() -> bool,
) -> Result<f32, WorldError> {
    if cancelled() {
        return Err(WorldError::Cancelled);
    }
    let shift = shift_for(find_threshold(grid, elevation, land_fraction));
    if cancelled() {
        return Err(WorldError::Cancelled);
    }
    if shift != 0.0 {
        for (index, value) in elevation.iter_mut().enumerate() {
            if index.is_multiple_of(4096) && cancelled() {
                return Err(WorldError::Cancelled);
            }
            *value += shift;
        }
    }
    Ok(shift)
}

/// Solve sea level, shift `elevation` by `-S` when `S >= 0`, and flood the ocean.
#[cfg(test)]
pub(crate) fn solve(
    grid: &CubeSphere,
    elevation: &mut [f32],
    domain: &[u8],
    land_fraction: f64,
) -> SeaLevel {
    let shift =
        apply_threshold(grid, elevation, land_fraction, &|| false).expect("no cancellation");
    finish(grid, elevation, domain, shift, &|| false).expect("no cancellation")
}

/// Copy of `elevation` shifted onto sea level, plus the ocean flood.
///
/// The caller's buffer is not written. `S < 0` leaves the copy unshifted, matching [`solve`].
pub(crate) fn sea_frame(
    grid: &CubeSphere,
    elevation: &[f32],
    domain: &[u8],
    land_fraction: f64,
    cancelled: &dyn Fn() -> bool,
) -> Result<(Vec<f32>, Vec<u8>), WorldError> {
    if cancelled() {
        return Err(WorldError::Cancelled);
    }
    let shift = shift_for(find_threshold(grid, elevation, land_fraction));
    let mut frame = elevation.to_vec();
    if shift != 0.0 {
        for value in &mut frame {
            *value += shift;
        }
    }
    if cancelled() {
        return Err(WorldError::Cancelled);
    }
    let ocean = flood(grid, &frame, domain, cancelled)?;
    Ok((frame, ocean))
}

fn find_threshold(grid: &CubeSphere, elevation: &[f32], land_fraction: f64) -> f32 {
    let n = elevation.len();
    let mut total = 0.0f64;
    let mut order = Vec::with_capacity(n);
    for cell in 0..n {
        total += grid.area_m2(cell as u32);
        order.push(cell as u32);
    }
    order.par_sort_unstable_by(|&a, &b| {
        elevation[a as usize]
            .total_cmp(&elevation[b as usize])
            .then(a.cmp(&b))
    });
    let target = land_fraction * total;
    let mut acc = 0.0f64;
    let mut threshold = if order.is_empty() {
        0.0
    } else {
        elevation[order[0] as usize]
    };
    for &cell in order.iter().rev() {
        if acc >= target {
            threshold = elevation[cell as usize];
            break;
        }
        acc += grid.area_m2(cell);
        threshold = elevation[cell as usize];
    }
    threshold
}

fn shift_for(threshold: f32) -> f32 {
    if threshold < 0.0 {
        0.0
    } else {
        -threshold
    }
}

pub(crate) fn finish(
    grid: &CubeSphere,
    elevation: &[f32],
    domain: &[u8],
    shift: f32,
    cancelled: &dyn Fn() -> bool,
) -> Result<SeaLevel, WorldError> {
    let mut total = 0.0f64;
    let mut above = 0.0f64;
    for (cell, &height) in elevation.iter().enumerate() {
        if cell.is_multiple_of(4096) && cancelled() {
            return Err(WorldError::Cancelled);
        }
        let area = grid.area_m2(cell as u32);
        total += area;
        if height > 0.0 {
            above += area;
        }
    }
    let achieved = if total == 0.0 { 0.0 } else { above / total };
    let ocean = flood(grid, elevation, domain, cancelled)?;
    let mut dry = 0u32;
    for (i, &is_ocean) in ocean.iter().enumerate() {
        if i.is_multiple_of(4096) && cancelled() {
            return Err(WorldError::Cancelled);
        }
        if domain[i] == 1 && elevation[i] <= 0.0 && is_ocean == 0 {
            dry = dry.saturating_add(1);
        }
    }
    Ok(SeaLevel {
        shift_m: shift,
        achieved_land_fraction: achieved,
        dry_depressions: dry,
        ocean,
    })
}

fn flood(
    grid: &CubeSphere,
    elevation: &[f32],
    domain: &[u8],
    cancelled: &dyn Fn() -> bool,
) -> Result<Vec<u8>, WorldError> {
    let n = elevation.len();
    let mut ocean = vec![0u8; n];
    let mut queue = VecDeque::new();
    for i in 0..n {
        if i.is_multiple_of(4096) && cancelled() {
            return Err(WorldError::Cancelled);
        }
        if domain[i] == 0 && elevation[i] <= 0.0 {
            ocean[i] = 1;
            queue.push_back(i as u32);
        }
    }
    let mut pops = 0u64;
    while let Some(cell) = queue.pop_front() {
        pops += 1;
        if pops.is_multiple_of(4096) && cancelled() {
            return Err(WorldError::Cancelled);
        }
        for nb in grid.neighbors(cell) {
            if nb == NONE {
                continue;
            }
            let j = nb as usize;
            if ocean[j] == 0 && elevation[j] <= 0.0 {
                ocean[j] = 1;
                queue.push_back(nb);
            }
        }
    }
    Ok(ocean)
}
