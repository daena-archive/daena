//! Sea level, the elevation shift, and ocean connectivity.
//!
//! `S` is chosen so the area with elevation strictly above `S` matches the
//! requested land fraction. The scan walks cells from high to low. If `S` is
//! negative the surface is left unshifted and the achieved fraction is the
//! area that already stands above 0 m.

use std::collections::VecDeque;

use rayon::prelude::*;

use crate::grid::{CubeSphere, NONE};

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

/// Solve sea level, shift `elevation` by `-S` when `S >= 0`, and flood the ocean.
pub(crate) fn solve(
    grid: &CubeSphere,
    elevation: &mut [f32],
    domain: &[u8],
    land_fraction: f64,
) -> SeaLevel {
    let shift = shift_for(find_threshold(grid, elevation, land_fraction));
    if shift != 0.0 {
        for value in elevation.iter_mut() {
            *value += shift;
        }
    }
    finish(grid, elevation, domain, shift)
}

/// Copy of `elevation` shifted onto sea level, plus the ocean flood.
///
/// The caller's buffer is not written. `S < 0` leaves the copy unshifted, matching [`solve`].
pub(crate) fn sea_frame(
    grid: &CubeSphere,
    elevation: &[f32],
    domain: &[u8],
    land_fraction: f64,
) -> (Vec<f32>, Vec<u8>) {
    let shift = shift_for(find_threshold(grid, elevation, land_fraction));
    let mut frame = elevation.to_vec();
    if shift != 0.0 {
        for value in &mut frame {
            *value += shift;
        }
    }
    let ocean = flood(grid, &frame, domain);
    (frame, ocean)
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

fn finish(grid: &CubeSphere, elevation: &[f32], domain: &[u8], shift: f32) -> SeaLevel {
    let n = elevation.len();
    let mut total = 0.0f64;
    let mut above = 0.0f64;
    for (cell, &height) in elevation.iter().enumerate() {
        let area = grid.area_m2(cell as u32);
        total += area;
        if height > 0.0 {
            above += area;
        }
    }
    let achieved = if total == 0.0 { 0.0 } else { above / total };
    let ocean = flood(grid, elevation, domain);
    let mut dry = 0u32;
    for i in 0..n {
        if domain[i] == 1 && elevation[i] <= 0.0 && ocean[i] == 0 {
            dry = dry.saturating_add(1);
        }
    }
    SeaLevel {
        shift_m: shift,
        achieved_land_fraction: achieved,
        dry_depressions: dry,
        ocean,
    }
}

fn flood(grid: &CubeSphere, elevation: &[f32], domain: &[u8]) -> Vec<u8> {
    let n = elevation.len();
    let mut ocean = vec![0u8; n];
    let mut queue = VecDeque::new();
    for i in 0..n {
        if domain[i] == 0 && elevation[i] <= 0.0 {
            ocean[i] = 1;
            queue.push_back(i as u32);
        }
    }
    while let Some(cell) = queue.pop_front() {
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
    ocean
}
