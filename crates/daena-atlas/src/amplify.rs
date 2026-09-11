//! Structure residual: interpolated landform grain, divide-tree ridge and
//! valley *paths*, and pit fill. Callers must pass year-0 / accepted
//! hydrology (`structure_controls` in `lib.rs`). Pit-fill and watershed
//! priors use that year-0 sea/lakes/watersheds — not epoch climate. Epoch
//! coastline grain lives in refine.

use daena_physical::Grid;
use rayon::prelude::*;

use crate::control::ControlFields;
use crate::detail::{
    cell_center_lat_micro, cell_center_lon_micro, domain_key, lattice_lat_micro, lattice_lon_micro,
    lattice_nearest_cells, lattice_sample, nearest_cell, nest_lattice_coord, sample_field_mm,
    sample_sdf_ppm, AtlasDetailModel, COASTAL_ENVELOPE_PPM,
};
use crate::erosion::{
    accumulate_flow, assign_simple_flow, lattice_index, lock_polar_rows, neighbor_at,
    priority_fill_pits, HIERARCHICAL_FILL_MM,
};
use crate::projection::{bilinear_i32, lat_to_row_ppm, lon_to_column_ppm};
use crate::request::DetailLevel;
use crate::{AtlasError, ATLAS_DETAIL_ALGORITHM_VERSION};

pub const HIERARCHICAL_RELIEF_DOMAIN: &str = "hierarchical-relief";
pub const MOUNTAIN_OROMETRY_DOMAIN: &str = "mountain-orometry";
pub const COASTLINE_SYNTHESIS_DOMAIN: &str = "coastline-synthesis";
pub const COASTAL_RAMP_MM: i32 = 72_000;
pub const COASTAL_DISPLACE_PPM: i32 = 380_000;
pub const MAX_MOUNTAIN_FEATURES: usize = 768;
pub const MAX_PEAKS_PER_SYSTEM: usize = 48;
pub const MIN_PEAK_SEPARATION: u32 = 6;
pub const RIDGE_SYNTHESIS_MM: i32 = 780_000;
pub const VALLEY_SYNTHESIS_MM: i32 = 520_000;
pub const PLAINS_VALLEY_MM: i32 = 96_000;
pub const OROMETRY_FALLOFF: u32 = 8;
/// Inspect hit `layer_id` only (same role as `landmass`). Not a paint layer.
pub const OROMETRY_LAYER: &str = "orometry";
/// High, low-influence physical masks. Edges are later refine erosion sites.
const UPLAND_HEIGHT_MM: i32 = 80_000;
const PLATEAU_RELIEF_MM: i32 = 120_000;
const MIN_UPLAND_CELLS: usize = 2;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u8)]
pub enum MountainKind {
    System,
    Peak,
    Saddle,
    Ridge,
    SecondaryRidge,
    Valley,
    Foothill,
    Plateau,
    Upland,
}

impl MountainKind {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::System => "system",
            Self::Peak => "peak",
            Self::Saddle => "saddle",
            Self::Ridge => "ridge",
            Self::SecondaryRidge => "secondary-ridge",
            Self::Valley => "valley",
            Self::Foothill => "foothill",
            Self::Plateau => "plateau",
            Self::Upland => "upland",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MountainFeature {
    pub id: String,
    pub kind: MountainKind,
    pub lattice_index: usize,
    pub physical_cell: usize,
    pub system_id: String,
    pub parent_id: Option<String>,
    pub basin_id: Option<u32>,
    pub lon_micro: i32,
    pub lat_micro: i32,
    pub elevation_mm: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OrometryInspectHit {
    pub id: String,
    pub kind: MountainKind,
    pub label: String,
}

#[derive(Debug, Clone)]
pub struct AmplificationModel {
    pub detail: AtlasDetailModel,
    pub mountain_system_count: u32,
    pub features: Vec<MountainFeature>,
    pub orometry_key: [u8; 32],
    pub structure_sea_level_mm: i32,
}

fn signed_unit_mm(sample: u64, amplitude_mm: i32) -> i32 {
    let unit = (sample >> 11) as i64 % 2_000_001 - 1_000_000;
    ((unit * i64::from(amplitude_mm)) / 1_000_000) as i32
}

pub(crate) fn landform_amplitude_from(elevation: i32, crust: i32, mountain: i32) -> i32 {
    let crust = crust.clamp(0, 1_000_000);
    let mountain = mountain.clamp(0, 1_000_000);
    let magnitude = elevation.unsigned_abs().min(8_000_000);
    let scaled = (u64::from(magnitude) * 64_000 / 8_000_000) as i32;
    let mut amplitude = scaled.clamp(18_000, 64_000);
    amplitude = ((i64::from(amplitude) * i64::from(crust.max(220_000))) / 1_000_000) as i32;
    amplitude = amplitude.saturating_add(((i64::from(mountain) * 72_000) / 1_000_000) as i32);
    if crust < 500_000 {
        return amplitude.min(180).clamp(48, 180);
    }
    amplitude.clamp(12_000, 140_000)
}

pub(crate) fn shape_unit(mountain: i32, unit: i32) -> i32 {
    let mountain = mountain.clamp(0, 1_000_000);
    let ridged = if unit < 0 { -unit / 2 } else { unit };
    ((i64::from(unit) * i64::from(1_000_000 - mountain) + i64::from(ridged) * i64::from(mountain))
        / 1_000_000) as i32
}

/// Physical grid ≈ brief LOD 0–1. Atlas octaves (factor 1, 2, 4, … finest) ≈ LOD 2–4.
fn stack_octaves(
    controls: &ControlFields,
    identity: &[u8],
    variant: u32,
    finest: u32,
    check_cancelled: &mut dyn FnMut() -> Result<(), AtlasError>,
) -> Result<(u32, u32, Vec<i32>), AtlasError> {
    stack_octaves_from(
        controls,
        identity,
        variant,
        finest,
        None,
        check_cancelled,
        &mut |_, _, _, _| {},
    )
}

pub(crate) fn cacheable_stack_factor(factor: u32) -> bool {
    factor == 4
}

pub(crate) type OctaveStackSink<'a> = dyn FnMut(u32, u32, u32, &[i32]) + 'a;

pub(crate) fn stack_octaves_from(
    controls: &ControlFields,
    identity: &[u8],
    variant: u32,
    finest: u32,
    start: Option<(u32, u32, u32, Vec<i32>)>,
    check_cancelled: &mut dyn FnMut() -> Result<(), AtlasError>,
    on_factor: &mut OctaveStackSink<'_>,
) -> Result<(u32, u32, Vec<i32>), AtlasError> {
    let finest = finest.max(1);
    let start = start.and_then(|(factor, width, height, residual_mm)| {
        let expected_width = controls.grid.width.checked_mul(factor)?;
        let expected_height = controls.grid.height.checked_mul(factor)?;
        let count = (width as usize).checked_mul(height as usize)?;
        if factor == 0
            || factor > finest
            || !factor.is_power_of_two()
            || !finest.is_multiple_of(factor)
            || width != expected_width
            || height != expected_height
            || residual_mm.len() != count
        {
            return None;
        }
        Some((factor, width, height, residual_mm))
    });
    let (mut factor, mut width, mut height, mut residual_mm) = if let Some(start) = start {
        start
    } else {
        let (width, height, residual_mm) =
            build_octave(controls, identity, variant, 1, check_cancelled)?;
        (1_u32, width, height, residual_mm)
    };
    if factor == 0 || width == 0 || height == 0 {
        return Err(AtlasError::limit("atlas octave stack start was empty"));
    }
    if cacheable_stack_factor(factor) {
        on_factor(factor, width, height, &residual_mm);
    }
    while factor < finest {
        factor = factor.saturating_mul(2);
        if factor > finest {
            factor = finest;
        }
        let next_width = controls
            .grid
            .width
            .checked_mul(factor)
            .ok_or_else(|| AtlasError::limit("atlas lattice width overflowed"))?;
        let next_height = controls
            .grid
            .height
            .checked_mul(factor)
            .ok_or_else(|| AtlasError::limit("atlas lattice height overflowed"))?;
        residual_mm = upsample_residual(
            width,
            height,
            &residual_mm,
            next_width,
            next_height,
            controls.grid.radius_metres,
            check_cancelled,
        )?;
        let (_, _, detail) = build_octave(controls, identity, variant, factor, check_cancelled)?;
        residual_mm
            .par_iter_mut()
            .zip(detail.par_iter())
            .for_each(|(slot, value)| {
                *slot = slot.saturating_add(*value);
            });
        width = next_width;
        height = next_height;
        if cacheable_stack_factor(factor) {
            on_factor(factor, width, height, &residual_mm);
        }
    }
    Ok((width, height, residual_mm))
}

pub(crate) fn octave_id_for_factor(factor: u32) -> u32 {
    factor.max(1).trailing_zeros()
}

pub(crate) fn octave_weight_for_factor(factor: u32) -> i32 {
    (1_000_000 / i32::try_from(factor.max(1)).unwrap_or(1)).max(1)
}

pub(crate) fn octave_noise_step_for_factor(factor: u32) -> u32 {
    match factor {
        0 | 1 => 8,
        2 => 4,
        4 => 2,
        _ => 1,
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn octave_cell_mm(
    grid: Grid,
    elevation_mm: &[i32],
    crust_influence_ppm: &[i32],
    mountain_influence_ppm: &[i32],
    key: &[u8; 32],
    factor: u32,
    i: u32,
    j: u32,
    width: u32,
    height: u32,
) -> i32 {
    let polar = j == 0 || j + 1 == height;
    let sample_i = if polar { 0 } else { i };
    let sample_j = if polar { 0 } else { j };
    let lon = lattice_lon_micro(if polar { 0 } else { i }, width);
    let lat = lattice_lat_micro(j, height);
    let noise = octave_noise_ppm(
        key,
        sample_i,
        sample_j,
        width,
        height,
        octave_id_for_factor(factor),
        octave_noise_step_for_factor(factor),
    );
    let amplitude = landform_amplitude_from(
        sample_field_mm(grid, elevation_mm, lon, lat),
        sample_field_mm(grid, crust_influence_ppm, lon, lat),
        sample_field_mm(grid, mountain_influence_ppm, lon, lat),
    );
    let unit = ((i64::from(noise) * i64::from(amplitude)) / 1_000_000) as i32;
    let shaped = shape_unit(
        sample_field_mm(grid, mountain_influence_ppm, lon, lat),
        unit,
    );
    ((i64::from(shaped) * i64::from(octave_weight_for_factor(factor))) / 1_000_000) as i32
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn octave_sample_mm(
    grid: Grid,
    elevation_mm: &[i32],
    crust_influence_ppm: &[i32],
    mountain_influence_ppm: &[i32],
    key: &[u8; 32],
    factor: u32,
    lon_micro: i32,
    lat_micro: i32,
) -> i32 {
    let width = grid.width.saturating_mul(factor.max(1)).max(1);
    let height = grid.height.saturating_mul(factor.max(1)).max(1);
    let (c0, c1, fx) = lon_to_column_ppm(lon_micro, width);
    let (r0, r1, fy) = lat_to_row_ppm(lat_micro, height);
    bilinear_i32(
        octave_cell_mm(
            grid,
            elevation_mm,
            crust_influence_ppm,
            mountain_influence_ppm,
            key,
            factor,
            c0,
            r0,
            width,
            height,
        ),
        octave_cell_mm(
            grid,
            elevation_mm,
            crust_influence_ppm,
            mountain_influence_ppm,
            key,
            factor,
            c1,
            r0,
            width,
            height,
        ),
        octave_cell_mm(
            grid,
            elevation_mm,
            crust_influence_ppm,
            mountain_influence_ppm,
            key,
            factor,
            c0,
            r1,
            width,
            height,
        ),
        octave_cell_mm(
            grid,
            elevation_mm,
            crust_influence_ppm,
            mountain_influence_ppm,
            key,
            factor,
            c1,
            r1,
            width,
            height,
        ),
        fx,
        fy,
    )
}

fn build_octave(
    controls: &ControlFields,
    identity: &[u8],
    variant: u32,
    factor: u32,
    check_cancelled: &mut dyn FnMut() -> Result<(), AtlasError>,
) -> Result<(u32, u32, Vec<i32>), AtlasError> {
    let width = controls
        .grid
        .width
        .checked_mul(factor)
        .ok_or_else(|| AtlasError::limit("atlas lattice width overflowed"))?;
    let height = controls
        .grid
        .height
        .checked_mul(factor)
        .ok_or_else(|| AtlasError::limit("atlas lattice height overflowed"))?;
    let count = (width as usize)
        .checked_mul(height as usize)
        .ok_or_else(|| AtlasError::limit("atlas lattice count overflowed"))?;
    let key = domain_key(
        identity,
        ATLAS_DETAIL_ALGORITHM_VERSION,
        variant,
        HIERARCHICAL_RELIEF_DOMAIN,
    );
    let mut residual = vec![0_i32; count];
    check_cancelled()?;
    let width_us = width as usize;
    residual
        .par_iter_mut()
        .enumerate()
        .for_each(|(index, slot)| {
            let i = (index % width_us) as u32;
            let j = (index / width_us) as u32;
            *slot = octave_cell_mm(
                controls.grid,
                &controls.elevation_mm,
                &controls.crust_influence_ppm,
                &controls.mountain_influence_ppm,
                &key,
                factor,
                i,
                j,
                width,
                height,
            );
        });
    check_cancelled()?;
    Ok((width, height, residual))
}

fn mean_remove(
    grid: Grid,
    cells: &[usize],
    residual: &mut [i32],
    check_cancelled: &mut dyn FnMut() -> Result<(), AtlasError>,
) -> Result<(), AtlasError> {
    let mut sums = vec![0_i64; grid.sample_count()];
    let mut counts = vec![0_u32; grid.sample_count()];
    check_cancelled()?;
    for (index, &cell) in cells.iter().enumerate() {
        sums[cell] += i64::from(residual[index]);
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
    check_cancelled()?;
    residual
        .par_iter_mut()
        .enumerate()
        .for_each(|(index, slot)| {
            *slot = (i64::from(*slot) - means[cells[index]]) as i32;
        });
    check_cancelled()?;
    Ok(())
}

fn mean_remove_outside_envelope(
    grid: Grid,
    sdf: &[i32],
    lattice_width: u32,
    lattice_height: u32,
    cells: &[usize],
    residual: &mut [i32],
    check_cancelled: &mut dyn FnMut() -> Result<(), AtlasError>,
) -> Result<(), AtlasError> {
    let mut sums = vec![0_i64; grid.sample_count()];
    let mut counts = vec![0_u32; grid.sample_count()];
    let width_us = lattice_width as usize;
    check_cancelled()?;
    for (index, &cell) in cells.iter().enumerate() {
        let i = (index % width_us) as u32;
        let j = (index / width_us) as u32;
        let lon = lattice_lon_micro(i, lattice_width);
        let lat = lattice_lat_micro(j, lattice_height);
        let sdf_ppm = sample_sdf_ppm(grid, sdf, lon, lat);
        if sdf_ppm.unsigned_abs() <= COASTAL_ENVELOPE_PPM {
            continue;
        }
        sums[cell] += i64::from(residual[index]);
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
    check_cancelled()?;
    residual
        .par_iter_mut()
        .enumerate()
        .for_each(|(index, slot)| {
            let i = (index % width_us) as u32;
            let j = (index / width_us) as u32;
            let lon = lattice_lon_micro(i, lattice_width);
            let lat = lattice_lat_micro(j, lattice_height);
            let sdf_ppm = sample_sdf_ppm(grid, sdf, lon, lat);
            if sdf_ppm.unsigned_abs() <= COASTAL_ENVELOPE_PPM {
                return;
            }
            *slot = (i64::from(*slot) - means[cells[index]]) as i32;
        });
    check_cancelled()?;
    Ok(())
}

fn sample_octave(
    width: u32,
    height: u32,
    field: &[i32],
    lon_micro: i32,
    lat_micro: i32,
    radius_metres: u64,
) -> i32 {
    let lattice = Grid {
        width,
        height,
        radius_metres,
    };
    sample_field_mm(lattice, field, lon_micro, lat_micro)
}

fn feature_id(kind: MountainKind, index: usize) -> String {
    format!(
        "atlas:orometry:v{}:{}:{index}",
        ATLAS_DETAIL_ALGORITHM_VERSION,
        kind.as_str()
    )
}

fn physical_lonlat(grid: Grid, cell: usize) -> (i32, i32) {
    let (row, col) = grid.row_col(cell);
    (
        cell_center_lon_micro(col, grid.width),
        cell_center_lat_micro(row, grid.height),
    )
}

fn physical_basin_id(controls: &ControlFields, lon_micro: i32, lat_micro: i32) -> Option<u32> {
    let sampled = controls.sample_basin_id(lon_micro, lat_micro);
    if sampled < 0 || sampled == i32::MAX {
        None
    } else {
        u32::try_from(sampled).ok()
    }
}

fn new_feature(
    kind: MountainKind,
    lattice_index: usize,
    lon: i32,
    lat: i32,
    elevation: i32,
) -> MountainFeature {
    MountainFeature {
        id: String::new(),
        kind,
        lattice_index,
        physical_cell: 0,
        system_id: String::new(),
        parent_id: None,
        basin_id: None,
        lon_micro: lon,
        lat_micro: lat,
        elevation_mm: elevation,
    }
}

struct UnionFind {
    parent: Vec<usize>,
}

impl UnionFind {
    fn new(count: usize) -> Self {
        Self {
            parent: (0..count).collect(),
        }
    }

    fn find(&mut self, mut node: usize) -> usize {
        while self.parent[node] != node {
            self.parent[node] = self.parent[self.parent[node]];
            node = self.parent[node];
        }
        node
    }

    fn union(&mut self, a: usize, b: usize) -> bool {
        let pa = self.find(a);
        let pb = self.find(b);
        if pa == pb {
            return false;
        }
        if pa < pb {
            self.parent[pb] = pa;
        } else {
            self.parent[pa] = pb;
        }
        true
    }
}

fn lattice_neighbors(width: u32, height: u32, i: u32, j: u32) -> Vec<(u32, u32)> {
    let mut neighbors = Vec::with_capacity(8);
    for dj in [-1_i32, 0, 1] {
        for di in [-1_i32, 0, 1] {
            if di == 0 && dj == 0 {
                continue;
            }
            let nj = j as i32 + dj;
            if nj < 0 || nj >= height as i32 {
                continue;
            }
            let ni = (i as i32 + di).rem_euclid(width as i32) as u32;
            neighbors.push((ni, nj as u32));
        }
    }
    neighbors
}

fn mountain_system_labels(controls: &ControlFields) -> (Vec<i32>, Vec<usize>) {
    let count = controls.grid.sample_count();
    let mut labels = vec![-1_i32; count];
    let mut min_cells = Vec::new();
    let mut next = 0_i32;
    for cell in 0..count {
        if labels[cell] >= 0 || controls.mountain_influence_ppm[cell] <= 0 {
            continue;
        }
        let mut stack = vec![cell];
        labels[cell] = next;
        let mut min_cell = cell;
        while let Some(current) = stack.pop() {
            min_cell = min_cell.min(current);
            for neighbor in controls.grid.neighbors(current) {
                if labels[neighbor] < 0 && controls.mountain_influence_ppm[neighbor] > 0 {
                    labels[neighbor] = next;
                    stack.push(neighbor);
                }
            }
        }
        min_cells.push(min_cell);
        next += 1;
    }
    (labels, min_cells)
}

fn lattice_path(width: u32, height: u32, start: usize, end: usize) -> Vec<usize> {
    let mut i = start as u32 % width;
    let mut j = start as u32 / width;
    let ti = end as u32 % width;
    let tj = end as u32 / width;
    let mut path = vec![start];
    let mut guard = 0_u32;
    while (i != ti || j != tj) && guard < width + height {
        guard += 1;
        let mut dcol = ti as i32 - i as i32;
        let wrap = width as i32;
        if dcol > wrap / 2 {
            dcol -= wrap;
        } else if dcol < -wrap / 2 {
            dcol += wrap;
        }
        let drow = tj as i32 - j as i32;
        if dcol.abs() >= drow.abs() && dcol != 0 {
            i = (i as i32 + dcol.signum()).rem_euclid(wrap) as u32;
        } else if drow != 0 {
            j = (j as i32 + drow.signum()).clamp(0, height as i32 - 1) as u32;
        } else {
            break;
        }
        path.push(lattice_index(width, i, j));
    }
    path
}

#[allow(clippy::too_many_arguments)]
fn push_feature(
    features: &mut Vec<MountainFeature>,
    kind: MountainKind,
    width: u32,
    i: u32,
    j: u32,
    lon: i32,
    lat: i32,
    elevation: i32,
) {
    if features.len() >= MAX_MOUNTAIN_FEATURES {
        return;
    }
    let index = lattice_index(width, i, j);
    if features
        .iter()
        .any(|feature| feature.kind == kind && feature.lattice_index == index)
    {
        return;
    }
    features.push(new_feature(kind, index, lon, lat, elevation));
}

#[allow(clippy::too_many_arguments)]
fn synthesize_orometry(
    width: u32,
    height: u32,
    residual: &mut [i32],
    ridge_cells: &[usize],
    valley_cells: &[usize],
    mountain: &[bool],
    land: &[bool],
    mountain_ppm: &[i32],
) {
    let count = residual.len();
    let mut d_ridge = vec![u32::MAX; count];
    let mut d_valley = vec![u32::MAX; count];
    let mut ridge_queue = std::collections::VecDeque::new();
    let mut valley_queue = std::collections::VecDeque::new();
    for &index in ridge_cells {
        if index < count {
            d_ridge[index] = 0;
            ridge_queue.push_back(index);
        }
    }
    for &index in valley_cells {
        if index < count {
            d_valley[index] = 0;
            valley_queue.push_back(index);
        }
    }
    let flood = |queue: &mut std::collections::VecDeque<usize>,
                 dist: &mut [u32],
                 allow: &dyn Fn(usize) -> bool| {
        while let Some(index) = queue.pop_front() {
            if dist[index] >= OROMETRY_FALLOFF {
                continue;
            }
            let j = index as u32 / width;
            let i = index as u32 % width;
            for dir in crate::erosion::DIRS {
                let Some((_, _, neighbor)) = neighbor_at(width, height, i, j, dir) else {
                    continue;
                };
                if !allow(neighbor) {
                    continue;
                }
                let next = dist[index].saturating_add(1);
                if dist[neighbor] > next {
                    dist[neighbor] = next;
                    queue.push_back(neighbor);
                }
            }
        }
    };
    flood(&mut ridge_queue, &mut d_ridge, &|index| {
        mountain[index] || land[index]
    });
    flood(&mut valley_queue, &mut d_valley, &|index| land[index]);
    for index in 0..count {
        if !land[index] {
            continue;
        }
        let j = index as u32 / width;
        if j == 0 || j + 1 == height {
            continue;
        }
        let dr = d_ridge[index];
        let dv = d_valley[index];
        if dr == u32::MAX && dv == u32::MAX {
            continue;
        }
        let ridge_w = if dr == u32::MAX {
            0
        } else {
            1_000_000 / (1 + dr.min(OROMETRY_FALLOFF))
        };
        let valley_w = if dv == u32::MAX {
            0
        } else {
            1_000_000 / (1 + dv.min(OROMETRY_FALLOFF))
        };
        let mountain_w = mountain_ppm[index].clamp(0, 1_000_000);
        let valley_amp = PLAINS_VALLEY_MM
            + ((i64::from(VALLEY_SYNTHESIS_MM - PLAINS_VALLEY_MM) * i64::from(mountain_w))
                / 1_000_000) as i32;
        let ridge_amp = ((i64::from(RIDGE_SYNTHESIS_MM) * i64::from(mountain_w.max(80_000)))
            / 1_000_000) as i32;
        let total = ridge_w + valley_w;
        if total == 0 {
            continue;
        }
        let delta = (i64::from(ridge_amp) * i64::from(ridge_w)
            - i64::from(valley_amp) * i64::from(valley_w))
            / i64::from(total);
        residual[index] = residual[index].saturating_add(delta as i32);
    }
}

fn follow_ascent(parent: &[usize], start: usize) -> Vec<usize> {
    let mut path = vec![start];
    let mut current = start;
    let mut guard = 0_usize;
    while guard < parent.len() {
        let next = parent[current];
        if next == usize::MAX || next == current {
            break;
        }
        path.push(next);
        current = next;
        guard += 1;
    }
    path
}

fn descent_path(
    width: u32,
    height: u32,
    elevation: &[i32],
    sea_level_mm: i32,
    start: usize,
) -> Vec<usize> {
    let mut path = vec![start];
    let mut current = start;
    let max_hops = (width + height).max(8);
    for _ in 0..max_hops {
        let j = current as u32 / width;
        let i = current as u32 % width;
        let mut best = current;
        let mut best_elev = elevation[current];
        for dir in crate::erosion::DIRS {
            let Some((_, _, neighbor)) = neighbor_at(width, height, i, j, dir) else {
                continue;
            };
            if elevation[neighbor] < best_elev
                || (elevation[neighbor] == best_elev && neighbor < best)
            {
                best_elev = elevation[neighbor];
                best = neighbor;
            }
        }
        if best == current {
            break;
        }
        path.push(best);
        if elevation[best] < sea_level_mm {
            break;
        }
        current = best;
    }
    path
}

struct OrometryPlan {
    features: Vec<MountainFeature>,
    ridge_cells: Vec<usize>,
    valley_cells: Vec<usize>,
}

fn peak_too_close(features: &[MountainFeature], width: u32, index: usize) -> bool {
    features.iter().any(|feature| {
        feature.kind == MountainKind::Peak
            && chebyshev(width, feature.lattice_index, index) < MIN_PEAK_SEPARATION
    })
}

const OROMETRY_ID_FACTOR: u32 = 4;

fn map_orometry_index(
    index: usize,
    src_width: u32,
    scale_i: u32,
    scale_j: u32,
    dst_width: u32,
) -> usize {
    let i = (index as u32 % src_width).saturating_mul(scale_i);
    let j = (index as u32 / src_width).saturating_mul(scale_j);
    lattice_index(dst_width, i, j)
}

#[allow(clippy::too_many_arguments)]
fn extract_mountain_features(
    controls: &ControlFields,
    identity: &[u8],
    variant: u32,
    lattice_width: u32,
    lattice_height: u32,
    residual: &[i32],
    system_labels: &[i32],
    check_cancelled: &mut dyn FnMut() -> Result<(), AtlasError>,
) -> Result<OrometryPlan, AtlasError> {
    let grid_w = controls.grid.width.max(1);
    let grid_h = controls.grid.height.max(1);
    let factor = (lattice_width / grid_w).max(1);
    let id_factor = factor.min(OROMETRY_ID_FACTOR);
    let work_w = grid_w.saturating_mul(id_factor);
    let work_h = grid_h.saturating_mul(id_factor);
    let coarse = if work_w != lattice_width || work_h != lattice_height {
        Some(stack_octaves(
            controls,
            identity,
            variant,
            id_factor,
            check_cancelled,
        )?)
    } else {
        None
    };
    let (work_width, work_height, work) = if let Some((_, _, coarse)) = coarse.as_ref() {
        (work_w, work_h, coarse.as_slice())
    } else {
        (lattice_width, lattice_height, residual)
    };
    let scale_i = lattice_width / work_width.max(1);
    let scale_j = lattice_height / work_height.max(1);
    let mut plan =
        extract_mountain_features_on(controls, work_width, work_height, work, system_labels);
    if scale_i != 1 || scale_j != 1 {
        for feature in &mut plan.features {
            feature.lattice_index = map_orometry_index(
                feature.lattice_index,
                work_width,
                scale_i,
                scale_j,
                lattice_width,
            );
        }
        for cell in &mut plan.ridge_cells {
            *cell = map_orometry_index(*cell, work_width, scale_i, scale_j, lattice_width);
        }
        for cell in &mut plan.valley_cells {
            *cell = map_orometry_index(*cell, work_width, scale_i, scale_j, lattice_width);
        }
    }
    Ok(plan)
}

fn extract_mountain_features_on(
    controls: &ControlFields,
    lattice_width: u32,
    lattice_height: u32,
    residual: &[i32],
    system_labels: &[i32],
) -> OrometryPlan {
    let count = lattice_width as usize * lattice_height as usize;
    let width_us = lattice_width as usize;
    let mut elevation = vec![0_i32; count];
    elevation
        .par_iter_mut()
        .enumerate()
        .for_each(|(index, slot)| {
            let i = (index % width_us) as u32;
            let j = (index / width_us) as u32;
            *slot = controls.sample_elevation(
                lattice_lon_micro(i, lattice_width),
                lattice_lat_micro(j, lattice_height),
            ) + residual[index];
        });
    let mut window = (0..count)
        .into_par_iter()
        .filter_map(|index| {
            let i = (index % width_us) as u32;
            let j = (index / width_us) as u32;
            let lon = lattice_lon_micro(i, lattice_width);
            let lat = lattice_lat_micro(j, lattice_height);
            if controls.sample_mountain_influence(lon, lat) <= 0 {
                return None;
            }
            let system = system_labels[nearest_cell(controls.grid, lon, lat)];
            Some((elevation[index], i, j, lon, lat, system))
        })
        .collect::<Vec<_>>();
    window.sort_by(|a, b| {
        b.0.cmp(&a.0)
            .then_with(|| a.1.cmp(&b.1))
            .then_with(|| a.2.cmp(&b.2))
    });
    let mut index_of = vec![usize::MAX; count];
    for (slot, &(_, i, j, _, _, _)) in window.iter().enumerate() {
        index_of[lattice_index(lattice_width, i, j)] = slot;
    }
    let mut uf = UnionFind::new(window.len());
    let mut seen = vec![false; window.len()];
    let mut peak_of = vec![None; window.len()];
    let mut ascent_parent = vec![usize::MAX; count];
    let mut features = Vec::new();
    let mut ridge_cells = Vec::new();
    let mut valley_cells = Vec::new();
    let mut peaks_in_system = vec![0_u32; system_labels.len().max(1)];
    for (slot, &(elev, i, j, lon, lat, system)) in window.iter().enumerate() {
        seen[slot] = true;
        let index = lattice_index(lattice_width, i, j);
        let mut higher_neighbor = false;
        let mut merge_slots = Vec::new();
        let mut parent = usize::MAX;
        let mut parent_elev = i32::MIN;
        for (ni, nj) in lattice_neighbors(lattice_width, lattice_height, i, j) {
            let neighbor_index = lattice_index(lattice_width, ni, nj);
            let neighbor_slot = index_of[neighbor_index];
            if neighbor_slot == usize::MAX {
                continue;
            }
            if window[neighbor_slot].0 > elev
                || (window[neighbor_slot].0 == elev && neighbor_slot < slot)
            {
                higher_neighbor = true;
            }
            if seen[neighbor_slot] {
                merge_slots.push(neighbor_slot);
                if window[neighbor_slot].0 > parent_elev
                    || (window[neighbor_slot].0 == parent_elev && neighbor_index < parent)
                {
                    parent_elev = window[neighbor_slot].0;
                    parent = neighbor_index;
                }
            }
        }
        ascent_parent[index] = parent;
        let system_slot = if system < 0 {
            usize::MAX
        } else {
            system as usize
        };
        let under_system_cap = system_slot == usize::MAX
            || peaks_in_system
                .get(system_slot)
                .copied()
                .unwrap_or(MAX_PEAKS_PER_SYSTEM as u32)
                < MAX_PEAKS_PER_SYSTEM as u32;
        if !higher_neighbor
            && features.len() < MAX_MOUNTAIN_FEATURES
            && under_system_cap
            && j > 0
            && j + 1 < lattice_height
            && !peak_too_close(&features, lattice_width, index)
        {
            peak_of[slot] = Some(features.len());
            if system_slot < peaks_in_system.len() {
                peaks_in_system[system_slot] += 1;
            }
            ridge_cells.push(index);
            features.push(new_feature(MountainKind::Peak, index, lon, lat, elev));
        }
        for neighbor in merge_slots {
            let pa = uf.find(slot);
            let pb = uf.find(neighbor);
            if pa == pb {
                continue;
            }
            let peak_a = peak_of[pa];
            let peak_b = peak_of[pb];
            uf.union(slot, neighbor);
            let root = uf.find(slot);
            peak_of[root] = peak_a.or(peak_b);
            if peak_a.is_some() && peak_b.is_some() && peak_a != peak_b {
                let neighbor_index =
                    lattice_index(lattice_width, window[neighbor].1, window[neighbor].2);
                let path_a = follow_ascent(&ascent_parent, index);
                let path_b = follow_ascent(&ascent_parent, neighbor_index);
                ridge_cells.extend_from_slice(&path_a);
                ridge_cells.extend_from_slice(&path_b);
                push_feature(
                    &mut features,
                    MountainKind::Saddle,
                    lattice_width,
                    i,
                    j,
                    lon,
                    lat,
                    elev,
                );
                push_feature(
                    &mut features,
                    MountainKind::Ridge,
                    lattice_width,
                    i,
                    j,
                    lon,
                    lat,
                    elev,
                );
                let valley = descent_path(
                    lattice_width,
                    lattice_height,
                    &elevation,
                    controls.sea_level_mm,
                    index,
                );
                if let Some(&outlet) = valley.get(1) {
                    let oj = outlet as u32 / lattice_width;
                    let oi = outlet as u32 % lattice_width;
                    let vlon = lattice_lon_micro(oi, lattice_width);
                    let vlat = lattice_lat_micro(oj, lattice_height);
                    if controls.sample_mountain_influence(vlon, vlat) > 0 {
                        push_feature(
                            &mut features,
                            MountainKind::Valley,
                            lattice_width,
                            oi,
                            oj,
                            vlon,
                            vlat,
                            elevation[outlet],
                        );
                    }
                }
                valley_cells.extend_from_slice(&valley);
            }
        }
    }
    let peaks = features
        .iter()
        .filter(|feature| feature.kind == MountainKind::Peak)
        .cloned()
        .collect::<Vec<_>>();
    for (a, peak_a) in peaks.iter().enumerate() {
        let mut nearest = None;
        let mut second = None;
        for (b, peak_b) in peaks.iter().enumerate() {
            if a == b {
                continue;
            }
            let cell_a = nearest_cell(controls.grid, peak_a.lon_micro, peak_a.lat_micro);
            let cell_b = nearest_cell(controls.grid, peak_b.lon_micro, peak_b.lat_micro);
            if system_labels[cell_a] < 0 || system_labels[cell_a] != system_labels[cell_b] {
                continue;
            }
            let dist = chebyshev(lattice_width, peak_a.lattice_index, peak_b.lattice_index);
            let candidate = (dist, peak_b.lattice_index, b);
            if nearest.is_none_or(|(d, _, _)| candidate < (d, usize::MAX, usize::MAX)) {
                second = nearest;
                nearest = Some(candidate);
            } else if second.is_none_or(|(d, idx, _)| (dist, peak_b.lattice_index) < (d, idx)) {
                second = Some(candidate);
            }
        }
        if let Some((dist, index, _)) = second {
            if dist > 1 && dist <= 12 {
                let mid = lattice_path(lattice_width, lattice_height, peak_a.lattice_index, index);
                ridge_cells.extend_from_slice(&mid);
                if let Some(&saddle) = mid.get(mid.len() / 2) {
                    let sj = saddle as u32 / lattice_width;
                    let si = saddle as u32 % lattice_width;
                    let lon = lattice_lon_micro(si, lattice_width);
                    let lat = lattice_lat_micro(sj, lattice_height);
                    push_feature(
                        &mut features,
                        MountainKind::SecondaryRidge,
                        lattice_width,
                        si,
                        sj,
                        lon,
                        lat,
                        elevation[saddle],
                    );
                }
            }
        }
    }
    for peak in &peaks {
        let mut foothill = None;
        for &cell in &ridge_cells {
            if cell == peak.lattice_index || elevation[cell] >= peak.elevation_mm {
                continue;
            }
            let dist = chebyshev(lattice_width, peak.lattice_index, cell);
            if (2..=8).contains(&dist) {
                let candidate = (dist, cell);
                if foothill.is_none_or(|best: (u32, usize)| candidate < best) {
                    foothill = Some(candidate);
                }
            }
        }
        let Some((_, cell)) = foothill else {
            continue;
        };
        let fj = cell as u32 / lattice_width;
        let fi = cell as u32 % lattice_width;
        push_feature(
            &mut features,
            MountainKind::Foothill,
            lattice_width,
            fi,
            fj,
            lattice_lon_micro(fi, lattice_width),
            lattice_lat_micro(fj, lattice_height),
            elevation[cell],
        );
    }
    ridge_cells.sort_unstable();
    ridge_cells.dedup();
    valley_cells.sort_unstable();
    valley_cells.dedup();
    features.truncate(MAX_MOUNTAIN_FEATURES);
    OrometryPlan {
        features,
        ridge_cells,
        valley_cells,
    }
}

fn complete_orometry(
    controls: &ControlFields,
    mut features: Vec<MountainFeature>,
    system_labels: &[i32],
    system_min_cells: &[usize],
) -> Vec<MountainFeature> {
    append_system_features(&mut features, controls, system_min_cells);
    append_upland_features(&mut features, controls);
    bind_structure_graph(&mut features, controls, system_labels, system_min_cells);
    features.sort_by(|a, b| a.id.cmp(&b.id).then_with(|| a.kind.cmp(&b.kind)));
    features
}

fn append_system_features(
    features: &mut Vec<MountainFeature>,
    controls: &ControlFields,
    system_min_cells: &[usize],
) {
    for &min_cell in system_min_cells {
        let (lon, lat) = physical_lonlat(controls.grid, min_cell);
        let elevation = controls.elevation_mm[min_cell];
        features.push(new_feature(
            MountainKind::System,
            min_cell,
            lon,
            lat,
            elevation,
        ));
    }
}

fn is_upland_cell(controls: &ControlFields, cell: usize) -> bool {
    controls.elevation_mm[cell] >= controls.sea_level_mm.saturating_add(UPLAND_HEIGHT_MM)
        && controls.mountain_influence_ppm[cell] <= 0
}

fn local_relief_mm(controls: &ControlFields, cell: usize) -> i32 {
    let mut min_elev = controls.elevation_mm[cell];
    let mut max_elev = min_elev;
    for neighbor in controls.grid.neighbors(cell) {
        let elev = controls.elevation_mm[neighbor];
        min_elev = min_elev.min(elev);
        max_elev = max_elev.max(elev);
    }
    max_elev.saturating_sub(min_elev)
}

fn append_upland_features(features: &mut Vec<MountainFeature>, controls: &ControlFields) {
    let count = controls.grid.sample_count();
    let mut seen = vec![false; count];
    for cell in 0..count {
        if seen[cell] || !is_upland_cell(controls, cell) {
            continue;
        }
        let mut stack = vec![cell];
        seen[cell] = true;
        let mut min_cell = cell;
        let mut high_cell = cell;
        let mut size = 0_usize;
        let mut max_relief = local_relief_mm(controls, cell);
        while let Some(current) = stack.pop() {
            min_cell = min_cell.min(current);
            size += 1;
            if controls.elevation_mm[current] > controls.elevation_mm[high_cell]
                || (controls.elevation_mm[current] == controls.elevation_mm[high_cell]
                    && current < high_cell)
            {
                high_cell = current;
            }
            max_relief = max_relief.max(local_relief_mm(controls, current));
            for neighbor in controls.grid.neighbors(current) {
                if !seen[neighbor] && is_upland_cell(controls, neighbor) {
                    seen[neighbor] = true;
                    stack.push(neighbor);
                }
            }
        }
        if size < MIN_UPLAND_CELLS {
            continue;
        }
        let kind = if max_relief <= PLATEAU_RELIEF_MM {
            MountainKind::Plateau
        } else {
            MountainKind::Upland
        };
        let (lon, lat) = physical_lonlat(controls.grid, high_cell);
        features.push(new_feature(
            kind,
            min_cell,
            lon,
            lat,
            controls.elevation_mm[high_cell],
        ));
    }
}

fn system_label_at(grid: Grid, system_labels: &[i32], cell: usize) -> i32 {
    if let Some(&label) = system_labels.get(cell) {
        if label >= 0 {
            return label;
        }
    }
    grid.neighbors(cell)
        .into_iter()
        .filter_map(|neighbor| {
            let label = *system_labels.get(neighbor)?;
            (label >= 0).then_some(label)
        })
        .min()
        .unwrap_or(-1)
}

/// IDs are `atlas:orometry:v{ver}:{kind}:{physical_cell}` plus `:{ordinal}`
/// when two same-kind features share a cell (lon/lat rank, not insertion).
/// Parent search is world-space so nested Standard/Detailed graphs match.
fn bind_structure_graph(
    features: &mut [MountainFeature],
    controls: &ControlFields,
    system_labels: &[i32],
    system_min_cells: &[usize],
) {
    for feature in features.iter_mut() {
        feature.physical_cell = if matches!(
            feature.kind,
            MountainKind::System | MountainKind::Plateau | MountainKind::Upland
        ) {
            feature.lattice_index
        } else {
            nearest_cell(controls.grid, feature.lon_micro, feature.lat_micro)
        };
        feature.basin_id = physical_basin_id(controls, feature.lon_micro, feature.lat_micro);
        if matches!(feature.kind, MountainKind::Plateau | MountainKind::Upland) {
            continue;
        }
        let label = system_label_at(controls.grid, system_labels, feature.physical_cell);
        if label >= 0 {
            if let Some(&min_cell) = system_min_cells.get(label as usize) {
                feature.system_id = feature_id(MountainKind::System, min_cell);
            }
        }
    }
    let mut groups: std::collections::BTreeMap<(MountainKind, usize), Vec<usize>> =
        std::collections::BTreeMap::new();
    for (index, feature) in features.iter().enumerate() {
        groups
            .entry((feature.kind, feature.physical_cell))
            .or_default()
            .push(index);
    }
    for slots in groups.values_mut() {
        slots.sort_by(|&a, &b| {
            features[a]
                .lon_micro
                .cmp(&features[b].lon_micro)
                .then_with(|| features[a].lat_micro.cmp(&features[b].lat_micro))
        });
        for (ordinal, &slot) in slots.iter().enumerate() {
            let base = feature_id(features[slot].kind, features[slot].physical_cell);
            features[slot].id = if slots.len() == 1 {
                base
            } else {
                format!("{base}:{ordinal}")
            };
        }
    }
    let ridge_slots = features
        .iter()
        .enumerate()
        .filter(|(_, feature)| {
            matches!(
                feature.kind,
                MountainKind::Ridge | MountainKind::SecondaryRidge
            )
        })
        .map(|(index, _)| index)
        .collect::<Vec<_>>();
    let peak_slots = features
        .iter()
        .enumerate()
        .filter(|(_, feature)| feature.kind == MountainKind::Peak)
        .map(|(index, _)| index)
        .collect::<Vec<_>>();
    for index in 0..features.len() {
        let kind = features[index].kind;
        let system_parent =
            (!features[index].system_id.is_empty()).then(|| features[index].system_id.clone());
        features[index].parent_id = match kind {
            MountainKind::System | MountainKind::Plateau | MountainKind::Upland => None,
            MountainKind::Ridge | MountainKind::SecondaryRidge | MountainKind::Saddle => {
                system_parent
            }
            MountainKind::Peak => nearest_in_system(features, &ridge_slots, index)
                .map(|slot| features[slot].id.clone())
                .or(system_parent),
            MountainKind::Valley | MountainKind::Foothill => {
                nearest_in_system(features, &peak_slots, index)
                    .map(|slot| features[slot].id.clone())
                    .or(system_parent)
            }
        };
    }
}

fn nearest_in_system(
    features: &[MountainFeature],
    candidates: &[usize],
    index: usize,
) -> Option<usize> {
    let system_id = &features[index].system_id;
    if system_id.is_empty() {
        return None;
    }
    let origin = &features[index];
    candidates
        .iter()
        .copied()
        .filter(|&slot| slot != index && features[slot].system_id == *system_id)
        .min_by_key(|&slot| {
            (
                orometry_distance2(
                    origin.lon_micro,
                    origin.lat_micro,
                    features[slot].lon_micro,
                    features[slot].lat_micro,
                ),
                features[slot].id.clone(),
            )
        })
}

fn wrap_dlon_micro(a: i32, b: i32) -> i64 {
    let mut delta = i64::from(a) - i64::from(b);
    if delta > 180_000_000 {
        delta -= 360_000_000;
    } else if delta < -180_000_000 {
        delta += 360_000_000;
    }
    delta
}

fn orometry_distance2(lon_a: i32, lat_a: i32, lon_b: i32, lat_b: i32) -> i64 {
    let dlon = wrap_dlon_micro(lon_a, lon_b);
    let dlat = i64::from(lat_a) - i64::from(lat_b);
    dlon.saturating_mul(dlon)
        .saturating_add(dlat.saturating_mul(dlat))
}

fn is_ridge_kind(kind: MountainKind) -> bool {
    matches!(kind, MountainKind::Ridge | MountainKind::SecondaryRidge)
}

fn nearest_of_kind<'a>(
    features: &'a [MountainFeature],
    kind: MountainKind,
    system_id: &str,
    lon_micro: i32,
    lat_micro: i32,
) -> Option<&'a MountainFeature> {
    features
        .iter()
        .filter(|feature| feature.kind == kind && feature.system_id == system_id)
        .min_by(|a, b| {
            orometry_distance2(a.lon_micro, a.lat_micro, lon_micro, lat_micro)
                .cmp(&orometry_distance2(
                    b.lon_micro,
                    b.lat_micro,
                    lon_micro,
                    lat_micro,
                ))
                .then_with(|| a.id.cmp(&b.id))
        })
}

fn ridge_parent<'a>(
    features: &'a [MountainFeature],
    peak: &MountainFeature,
) -> Option<&'a MountainFeature> {
    peak.parent_id.as_deref().and_then(|parent| {
        features
            .iter()
            .find(|feature| feature.id == parent && is_ridge_kind(feature.kind))
    })
}

#[must_use]
pub fn inspect_orometry(
    features: &[MountainFeature],
    lon_micro: i32,
    lat_micro: i32,
    radius_micro: i32,
) -> Vec<OrometryInspectHit> {
    let radius2 = i64::from(radius_micro.max(1)).saturating_mul(i64::from(radius_micro.max(1)));
    let nearest = features.iter().filter_map(|feature| {
        let distance =
            orometry_distance2(feature.lon_micro, feature.lat_micro, lon_micro, lat_micro);
        (distance <= radius2).then_some((distance, feature.id.as_str(), feature))
    });
    let Some((_, _, nearest)) = nearest.min_by(|a, b| a.0.cmp(&b.0).then_with(|| a.1.cmp(b.1)))
    else {
        return Vec::new();
    };
    if matches!(nearest.kind, MountainKind::Plateau | MountainKind::Upland) {
        return vec![orometry_hit(nearest)];
    }
    let system_id = if nearest.kind == MountainKind::System {
        nearest.id.as_str()
    } else {
        nearest.system_id.as_str()
    };
    let system = features
        .iter()
        .find(|feature| feature.kind == MountainKind::System && feature.id == system_id);
    let (ridge, peak) = match nearest.kind {
        MountainKind::Peak => (ridge_parent(features, nearest), Some(nearest)),
        MountainKind::Ridge | MountainKind::SecondaryRidge => {
            let child = features
                .iter()
                .filter(|feature| {
                    feature.kind == MountainKind::Peak
                        && feature.parent_id.as_deref() == Some(nearest.id.as_str())
                })
                .min_by(|a, b| {
                    orometry_distance2(a.lon_micro, a.lat_micro, lon_micro, lat_micro)
                        .cmp(&orometry_distance2(
                            b.lon_micro,
                            b.lat_micro,
                            lon_micro,
                            lat_micro,
                        ))
                        .then_with(|| a.id.cmp(&b.id))
                })
                .or_else(|| {
                    nearest_of_kind(
                        features,
                        MountainKind::Peak,
                        system_id,
                        lon_micro,
                        lat_micro,
                    )
                });
            (Some(nearest), child)
        }
        _ => {
            let peak = nearest_of_kind(
                features,
                MountainKind::Peak,
                system_id,
                lon_micro,
                lat_micro,
            );
            let ridge = peak
                .and_then(|feature| ridge_parent(features, feature))
                .or_else(|| {
                    features
                        .iter()
                        .filter(|feature| {
                            is_ridge_kind(feature.kind) && feature.system_id == system_id
                        })
                        .min_by(|a, b| {
                            orometry_distance2(a.lon_micro, a.lat_micro, lon_micro, lat_micro)
                                .cmp(&orometry_distance2(
                                    b.lon_micro,
                                    b.lat_micro,
                                    lon_micro,
                                    lat_micro,
                                ))
                                .then_with(|| a.id.cmp(&b.id))
                        })
                });
            (ridge, peak)
        }
    };
    let mut hits = Vec::new();
    if let Some(feature) = system {
        hits.push(orometry_hit(feature));
    }
    if let Some(feature) = ridge {
        hits.push(orometry_hit(feature));
    }
    if let Some(feature) = peak {
        hits.push(orometry_hit(feature));
    }
    hits
}

fn orometry_hit(feature: &MountainFeature) -> OrometryInspectHit {
    OrometryInspectHit {
        id: feature.id.clone(),
        kind: feature.kind,
        label: feature.kind.as_str().to_string(),
    }
}

fn chebyshev(width: u32, a: usize, b: usize) -> u32 {
    let (ar, ac) = (a as u32 / width, a as u32 % width);
    let (br, bc) = (b as u32 / width, b as u32 % width);
    let drow = ar.abs_diff(br);
    let dcol = ac.abs_diff(bc).min(width.saturating_sub(ac.abs_diff(bc)));
    drow.max(dcol)
}

fn upsample_residual(
    src_width: u32,
    src_height: u32,
    src: &[i32],
    dst_width: u32,
    dst_height: u32,
    radius_metres: u64,
    check_cancelled: &mut dyn FnMut() -> Result<(), AtlasError>,
) -> Result<Vec<i32>, AtlasError> {
    let mut dst = vec![0_i32; dst_width as usize * dst_height as usize];
    check_cancelled()?;
    let width_us = dst_width as usize;
    dst.par_iter_mut().enumerate().for_each(|(index, slot)| {
        let i = (index % width_us) as u32;
        let j = (index / width_us) as u32;
        let polar = j == 0 || j + 1 == dst_height;
        if polar {
            let src_j = if j == 0 { 0 } else { src_height - 1 };
            *slot = src[lattice_index(src_width, 0, src_j)];
            return;
        }
        *slot = sample_octave(
            src_width,
            src_height,
            src,
            lattice_lon_micro(i, dst_width),
            lattice_lat_micro(j, dst_height),
            radius_metres,
        );
    });
    check_cancelled()?;
    Ok(dst)
}

fn land_mask_ppm(controls: &ControlFields) -> Vec<i32> {
    let sea = controls.sea_level_mm;
    controls
        .elevation_mm
        .iter()
        .map(|elevation| if *elevation >= sea { 1_000_000 } else { 0 })
        .collect()
}

fn octave_noise_ppm(
    key: &[u8; 32],
    i: u32,
    j: u32,
    width: u32,
    height: u32,
    octave: u32,
    step: u32,
) -> i32 {
    octave_noise_ppm_keyed(key, i, j, width, height, octave, step, false)
}

#[allow(clippy::too_many_arguments)]
fn octave_noise_ppm_keyed(
    key: &[u8; 32],
    i: u32,
    j: u32,
    width: u32,
    height: u32,
    octave: u32,
    step: u32,
    nest: bool,
) -> i32 {
    let step = step.max(1);
    let i0 = (i / step) * step;
    let j0 = (j / step) * step;
    let i1 = (i0 + step) % width;
    let j1 = (j0 + step).min(height.saturating_sub(1));
    let fx = ((i - i0) * 1_000_000) / step;
    let fy = if j1 == j0 {
        0
    } else {
        ((j - j0) * 1_000_000) / step
    };
    let sample = |ii: u32, jj: u32| {
        let polar = jj == 0 || jj + 1 == height;
        let si = if polar { 0 } else { ii };
        let sj = if polar { 0 } else { jj };
        let (si, sj) = if nest {
            (
                nest_lattice_coord(si, width),
                nest_lattice_coord(sj, height),
            )
        } else {
            (si, sj)
        };
        signed_unit_mm(lattice_sample(key, si, sj, octave), 1_000_000)
    };
    bilinear_i32(
        sample(i0, j0),
        sample(i1, j0),
        sample(i0, j1),
        sample(i1, j1),
        fx,
        fy,
    )
}

fn coastline_noise_ppm(
    key: &[u8; 32],
    i: u32,
    j: u32,
    width: u32,
    height: u32,
    grid_width: u32,
) -> i32 {
    let factor = (width / grid_width.max(1)).max(1);
    let step = |base: u32| (base.saturating_mul(factor) / 4).max(1);
    let coarse = octave_noise_ppm_keyed(key, i, j, width, height, 0, step(8), true);
    let mid = octave_noise_ppm_keyed(key, i, j, width, height, 1, step(4), true);
    let fine = octave_noise_ppm_keyed(key, i, j, width, height, 2, step(2), true);
    ((i64::from(coarse) * 520_000 + i64::from(mid) * 300_000 + i64::from(fine) * 180_000)
        / 1_000_000) as i32
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn apply_coastline(
    controls: &ControlFields,
    sdf: &[i32],
    identity: &[u8],
    variant: u32,
    width: u32,
    height: u32,
    structure_sea_level_mm: i32,
    structure_mm: &[i32],
    protected: &[bool],
    surface: &mut [i32],
    check_cancelled: &mut dyn FnMut() -> Result<(), AtlasError>,
) -> Result<(), AtlasError> {
    let key = domain_key(
        identity,
        ATLAS_DETAIL_ALGORITHM_VERSION,
        variant,
        COASTLINE_SYNTHESIS_DOMAIN,
    );
    let land_ppm = land_mask_ppm(controls);
    let sea = controls.sea_level_mm;
    let sea_rise = sea.saturating_sub(structure_sea_level_mm);
    check_cancelled()?;
    let width_us = width as usize;
    surface
        .par_iter_mut()
        .enumerate()
        .for_each(|(index, slot)| {
            if protected.get(index).copied().unwrap_or(false) {
                return;
            }
            let i = (index % width_us) as u32;
            let j = (index / width_us) as u32;
            let polar = j == 0 || j + 1 == height;
            let lon = lattice_lon_micro(if polar { 0 } else { i }, width);
            let lat = lattice_lat_micro(j, height);
            if controls.sample_lake_mask(lon, lat) > 0 {
                return;
            }
            let sdf_ppm = sample_sdf_ppm(controls.grid, sdf, lon, lat);
            if sdf_ppm.unsigned_abs() > COASTAL_ENVELOPE_PPM {
                return;
            }
            let fraction = sample_field_mm(controls.grid, &land_ppm, lon, lat);
            let proximity = 1_000_000_u32.saturating_sub(
                fraction
                    .saturating_sub(500_000)
                    .unsigned_abs()
                    .saturating_mul(2),
            );
            if proximity == 0 {
                return;
            }
            let noise = coastline_noise_ppm(
                &key,
                if polar { 0 } else { i },
                j,
                width,
                height,
                controls.grid.width,
            );
            let displaced = fraction.saturating_add(
                ((i64::from(noise) * i64::from(COASTAL_DISPLACE_PPM)) / 1_000_000) as i32,
            );
            let ramp = ((i64::from(displaced.saturating_sub(500_000)) * i64::from(COASTAL_RAMP_MM))
                / 500_000) as i32;
            let target = sea.saturating_add(ramp);
            let current = structure_mm.get(index).copied().unwrap_or(*slot);
            let continental = controls.sample_crust_class(lon, lat) > 0;
            let blended = ((i64::from(current) * i64::from(1_000_000 - proximity as i32)
                + i64::from(target) * i64::from(proximity as i32))
                / 1_000_000) as i32;
            let want_land = displaced >= 500_000 && (continental || current >= sea);
            let mut signed = if want_land == (blended >= sea) {
                blended
            } else if want_land {
                blended.max(sea.saturating_add(1)).max(target)
            } else {
                blended.min(sea.saturating_sub(1)).min(target)
            };
            let work = ((i64::from(proximity) * i64::from(sea_rise.abs().min(COASTAL_RAMP_MM)))
                / 8_000_000) as i32;
            if work != 0 {
                signed = if sea_rise > 0 {
                    signed.saturating_sub(work)
                } else {
                    signed.saturating_add(work)
                };
                if want_land != (signed >= sea) {
                    signed = if want_land {
                        signed.max(sea.saturating_add(1))
                    } else {
                        signed.min(sea.saturating_sub(1))
                    };
                }
            }
            *slot = signed;
        });
    check_cancelled()?;
    lock_polar_rows(width, height, surface);
    Ok(())
}

/// Structure residual from year-0 / accepted hydrology.
///
/// `controls` must be year-0 (`structure_controls`). Sea, lakes, and
/// watersheds on that snapshot drive pit-fill. Do not pass epoch hydrology
/// or climate; those operators belong in refine.
pub fn build_amplification_model(
    controls: &ControlFields,
    identity: &[u8],
    variant: u32,
    level: DetailLevel,
    check_cancelled: &mut dyn FnMut() -> Result<(), AtlasError>,
) -> Result<AmplificationModel, AtlasError> {
    let (width, height, residual_mm) = stack_octaves(
        controls,
        identity,
        variant,
        level.lattice_factor(),
        check_cancelled,
    )?;
    finish_amplification_model(
        controls,
        identity,
        variant,
        level,
        (width, height, residual_mm),
        check_cancelled,
    )
}

pub(crate) fn finish_amplification_model(
    controls: &ControlFields,
    identity: &[u8],
    variant: u32,
    level: DetailLevel,
    stacked: (u32, u32, Vec<i32>),
    check_cancelled: &mut dyn FnMut() -> Result<(), AtlasError>,
) -> Result<AmplificationModel, AtlasError> {
    let (width, height, mut residual_mm) = stacked;
    let sea = controls.sea_level_mm;
    let sdf =
        crate::detail::signed_coastal_distance_ppm(controls.grid, &controls.elevation_mm, sea);
    let cells = lattice_nearest_cells(controls.grid, width, height);
    let (system_labels, system_min_cells) = mountain_system_labels(controls);
    let system_count = system_min_cells.len() as u32;
    let plan = extract_mountain_features(
        controls,
        identity,
        variant,
        width,
        height,
        &residual_mm,
        &system_labels,
        check_cancelled,
    )?;
    mean_remove(controls.grid, &cells, &mut residual_mm, check_cancelled)?;
    let mut surface = vec![0_i32; residual_mm.len()];
    let mut protected = vec![false; residual_mm.len()];
    let mut land = vec![false; residual_mm.len()];
    let mut mountain = vec![false; residual_mm.len()];
    let mut mountain_ppm = vec![0_i32; residual_mm.len()];
    let mut watershed = vec![-1_i32; residual_mm.len()];
    let width_us = width as usize;
    check_cancelled()?;
    surface
        .par_iter_mut()
        .zip(protected.par_iter_mut())
        .zip(land.par_iter_mut())
        .zip(mountain.par_iter_mut())
        .zip(mountain_ppm.par_iter_mut())
        .zip(watershed.par_iter_mut())
        .zip(residual_mm.par_iter())
        .enumerate()
        .for_each(
            |(index, ((((((surf, prot), land_slot), mtn), mtn_ppm), ws), residual))| {
                let i = (index % width_us) as u32;
                let j = (index / width_us) as u32;
                let lon = lattice_lon_micro(i, width);
                let lat = lattice_lat_micro(j, height);
                let elevation = controls.sample_elevation(lon, lat);
                let mountain_inf = controls.sample_mountain_influence(lon, lat);
                *surf = elevation + *residual;
                *prot = controls.sample_lake_mask(lon, lat) > 0;
                *land_slot = elevation >= sea;
                *mtn = mountain_inf > 0;
                *mtn_ppm = mountain_inf;
                *ws = controls.sample_watershed_id(lon, lat);
            },
        );
    check_cancelled()?;
    lock_polar_rows(width, height, &mut surface);
    surface = priority_fill_pits(
        width,
        height,
        &surface,
        &protected,
        sea,
        HIERARCHICAL_FILL_MM,
        check_cancelled,
    )?;
    residual_mm
        .par_iter_mut()
        .zip(surface.par_iter())
        .enumerate()
        .for_each(|(index, (residual, surf))| {
            let i = (index % width_us) as u32;
            let j = (index / width_us) as u32;
            *residual = *surf
                - controls
                    .sample_elevation(lattice_lon_micro(i, width), lattice_lat_micro(j, height));
        });
    let (primary, secondary, weight) =
        assign_simple_flow(width, height, &surface, &watershed, sea, check_cancelled)?;
    let accumulation = accumulate_flow(
        &surface,
        &primary,
        &secondary,
        &weight,
        sea,
        check_cancelled,
    )?;
    let ridge_cells = plan.ridge_cells;
    let mut valley_cells = plan.valley_cells;
    let accum_threshold = (width / 8).max(24);
    for index in 0..surface.len() {
        if land[index] && accumulation[index] >= accum_threshold {
            valley_cells.push(index);
        }
    }
    valley_cells.sort_unstable();
    valley_cells.dedup();
    synthesize_orometry(
        width,
        height,
        &mut residual_mm,
        &ridge_cells,
        &valley_cells,
        &mountain,
        &land,
        &mountain_ppm,
    );
    mean_remove(controls.grid, &cells, &mut residual_mm, check_cancelled)?;
    mean_remove_outside_envelope(
        controls.grid,
        &sdf,
        width,
        height,
        &cells,
        &mut residual_mm,
        check_cancelled,
    )?;
    let orometry_key = domain_key(
        identity,
        ATLAS_DETAIL_ALGORITHM_VERSION,
        variant,
        MOUNTAIN_OROMETRY_DOMAIN,
    );
    Ok(AmplificationModel {
        detail: AtlasDetailModel {
            grid: controls.grid,
            elevations_mm: controls.elevation_mm.clone(),
            residual_mm,
            lattice_width: width,
            lattice_height: height,
            algorithm_version: crate::ATLAS_DETAIL_ALGORITHM_VERSION,
            variant,
            level,
        },
        mountain_system_count: system_count,
        features: complete_orometry(controls, plan.features, &system_labels, &system_min_cells),
        orometry_key,
        structure_sea_level_mm: controls.sea_level_mm,
    })
}

impl AmplificationModel {
    #[must_use]
    pub fn from_cached_detail(
        detail: AtlasDetailModel,
        controls: &ControlFields,
        identity: &[u8],
    ) -> Self {
        let (system_labels, system_min_cells) = mountain_system_labels(controls);
        let system_count = system_min_cells.len() as u32;
        let mut cancel = || Ok(());
        let features = complete_orometry(
            controls,
            extract_mountain_features(
                controls,
                identity,
                detail.variant,
                detail.lattice_width,
                detail.lattice_height,
                &detail.residual_mm,
                &system_labels,
                &mut cancel,
            )
            .map(|plan| plan.features)
            .unwrap_or_default(),
            &system_labels,
            &system_min_cells,
        );
        let orometry_key = domain_key(
            identity,
            detail.algorithm_version,
            detail.variant,
            MOUNTAIN_OROMETRY_DOMAIN,
        );
        Self {
            detail,
            mountain_system_count: system_count,
            features,
            orometry_key,
            structure_sea_level_mm: controls.sea_level_mm,
        }
    }
}

#[must_use]
pub fn land_components(grid: Grid, elevations_mm: &[i32], sea_level_mm: i32) -> Vec<i32> {
    let count = grid.sample_count();
    let mut labels = vec![-1_i32; count];
    let mut next = 0_i32;
    for cell in 0..count {
        if labels[cell] >= 0 || elevations_mm[cell] < sea_level_mm {
            continue;
        }
        let mut stack = vec![cell];
        labels[cell] = next;
        while let Some(current) = stack.pop() {
            for neighbor in grid.neighbors(current) {
                if labels[neighbor] < 0 && elevations_mm[neighbor] >= sea_level_mm {
                    labels[neighbor] = next;
                    stack.push(neighbor);
                }
            }
        }
        next += 1;
    }
    labels
}

#[must_use]
pub fn downsample_preserves_sign_outside_envelope(
    model: &AtlasDetailModel,
    sea_level_mm: i32,
    sdf: &[i32],
) -> bool {
    for j in 0..model.lattice_height {
        for i in 0..model.lattice_width {
            let lon = lattice_lon_micro(i, model.lattice_width);
            let lat = lattice_lat_micro(j, model.lattice_height);
            let sdf_ppm = sample_sdf_ppm(model.grid, sdf, lon, lat);
            if sdf_ppm.unsigned_abs() <= COASTAL_ENVELOPE_PPM {
                continue;
            }
            let canonical = model.canonical_at(lon, lat);
            let refined = model.refined_at(lon, lat, sea_level_mm, sdf_ppm);
            if (canonical >= sea_level_mm) != (refined >= sea_level_mm) {
                return false;
            }
        }
    }
    true
}

#[must_use]
pub fn interior_land_components_equivalent(
    canonical: &[i32],
    refined: &[i32],
    sdf: &[i32],
) -> bool {
    let count = canonical.len();
    for first in 0..count {
        if sdf[first].unsigned_abs() <= COASTAL_ENVELOPE_PPM || canonical[first] < 0 {
            continue;
        }
        for second in first + 1..count {
            if sdf[second].unsigned_abs() <= COASTAL_ENVELOPE_PPM || canonical[second] < 0 {
                continue;
            }
            let same_canonical = canonical[first] == canonical[second];
            let same_refined = refined[first] >= 0 && refined[first] == refined[second];
            if same_canonical != same_refined {
                return false;
            }
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::control::ControlFields;
    use crate::detail::{domain_key, signed_coastal_distance_ppm};
    use crate::golden_world;
    use crate::projection::{pixel_center_lat_micro, pixel_center_lon_micro};
    use crate::request::AtlasRenderRequest;
    use crate::spike_identity_from_source;
    use crate::studio::xyz_world_pixel_center;
    use crate::style::BIOME_STYLE_ID;
    use crate::style::{load_style, ANTIQUE_STYLE_ID, RELIEF_STYLE_ID};
    use crate::ATLAS_DETAIL_ALGORITHM_VERSION;
    use daena_physical::history::{
        derive_historical_world_with_planet, HistoricalForcingParameters,
    };
    use daena_physical::NoopProgress as PhysicalNoop;
    use std::time::Instant;

    fn controls_at(offset_years: i64) -> (ControlFields, Vec<u8>, i32, Vec<i32>) {
        let world = golden_world();
        let identity = spike_identity_from_source(&world.source);
        let historical = derive_historical_world_with_planet(
            &world.field,
            world.report.reference_water_inventory_m3,
            Some(&world.tectonics.crust_by_cell),
            HistoricalForcingParameters::default_for(world.field.seed, world.field.retry_index),
            offset_years,
            world.climate.planetary,
            &mut PhysicalNoop,
        )
        .unwrap();
        let sdf = signed_coastal_distance_ppm(
            world.field.grid,
            &world.field.elevations_mm,
            historical.metrics.sea_level_mm,
        );
        let climate = if offset_years == 0 {
            world.climate.clone()
        } else {
            historical.climate.clone()
        };
        let hydrology = if offset_years == 0 {
            world.hydrology.clone()
        } else {
            historical.hydrology.clone()
        };
        let controls =
            ControlFields::from_accepted(&world.field, &world.tectonics, &climate, &hydrology)
                .unwrap();
        (controls, identity, historical.metrics.sea_level_mm, sdf)
    }

    #[test]
    fn experimental_domain_key_is_isolated_from_version_one() {
        let v1 = domain_key(b"identity-fixture", 1, 0, HIERARCHICAL_RELIEF_DOMAIN);
        let next = ATLAS_DETAIL_ALGORITHM_VERSION.wrapping_add(1);
        let v_next = domain_key(b"identity-fixture", next, 0, HIERARCHICAL_RELIEF_DOMAIN);
        assert_ne!(v1, v_next);
    }

    #[test]
    fn small_grid_amplification_builds() {
        let settings = daena_physical::GenerationSettings {
            width: 16,
            height: 8,
            radius_metres: daena_physical::DEFAULT_RADIUS_METRES,
            target_land_fraction_ppm: 300_000,
        };
        let world =
            daena_physical::generate_world(settings, 831_429, 0, &mut PhysicalNoop).unwrap();
        let controls = ControlFields::from_accepted(
            &world.field,
            &world.tectonics,
            &world.climate,
            &world.hydrology,
        )
        .unwrap();
        let identity = spike_identity_from_source(&world.source);
        let mut cancel = || Ok(());
        build_amplification_model(&controls, &identity, 0, DetailLevel::Standard, &mut cancel)
            .unwrap();
    }

    #[test]
    fn amplification_conserves_macro_sign_topology_and_drainage() {
        let world = golden_world();
        let (controls, identity, _, _) = controls_at(0);
        let sea = controls.sea_level_mm;
        let sdf = signed_coastal_distance_ppm(controls.grid, &controls.elevation_mm, sea);
        let mut cancel = || Ok(());
        let started = Instant::now();
        let model =
            build_amplification_model(&controls, &identity, 0, DetailLevel::Standard, &mut cancel)
                .unwrap();
        assert!(
            started.elapsed().as_secs() < 5,
            "standard amplification exceeded the 5s budget"
        );
        assert!(
            model.detail.residual_mm.len() * 4 <= 2_000_000,
            "residual exceeded the in-process byte budget"
        );
        assert_eq!(
            model.detail.algorithm_version,
            ATLAS_DETAIL_ALGORITHM_VERSION
        );
        assert!(downsample_preserves_sign_outside_envelope(
            &model.detail,
            sea,
            &sdf
        ));
        let mut sums = vec![0_i64; model.detail.grid.sample_count()];
        let mut counts = vec![0_u32; model.detail.grid.sample_count()];
        for j in 0..model.detail.lattice_height {
            for i in 0..model.detail.lattice_width {
                let lon = lattice_lon_micro(i, model.detail.lattice_width);
                let lat = lattice_lat_micro(j, model.detail.lattice_height);
                let sdf_ppm = sample_sdf_ppm(model.detail.grid, &sdf, lon, lat);
                if sdf_ppm.unsigned_abs() <= COASTAL_ENVELOPE_PPM {
                    continue;
                }
                let cell = nearest_cell(model.detail.grid, lon, lat);
                sums[cell] += i64::from(
                    model.detail.residual_mm
                        [j as usize * model.detail.lattice_width as usize + i as usize],
                );
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
            max_mean <= 1,
            "per-cell residual mean drifted by {max_mean} mm"
        );

        let original_rivers = world
            .hydrology
            .rivers
            .iter()
            .map(|river| (river.id, river.mouth_cell, river.source_cell))
            .collect::<Vec<_>>();
        assert_eq!(
            original_rivers,
            world
                .hydrology
                .rivers
                .iter()
                .map(|river| (river.id, river.mouth_cell, river.source_cell))
                .collect::<Vec<_>>()
        );
        for river in &world.hydrology.rivers {
            for cell in [river.mouth_cell, river.source_cell] {
                if sdf[cell].unsigned_abs() <= COASTAL_ENVELOPE_PPM {
                    continue;
                }
                assert_eq!(
                    world.hydrology.watershed_id[cell],
                    u32::try_from(controls.watershed_id[cell]).unwrap_or(u32::MAX)
                );
            }
        }
        assert!(!model.features.is_empty());
        assert!(model.mountain_system_count >= 1);
        assert!(model.features.iter().any(|f| f.kind == MountainKind::Peak));
        assert!(model
            .features
            .iter()
            .any(|f| matches!(f.kind, MountainKind::Saddle | MountainKind::Ridge)));
        assert!(model.features.iter().all(|feature| {
            matches!(
                feature.kind,
                MountainKind::Plateau
                    | MountainKind::Upland
                    | MountainKind::Foothill
                    | MountainKind::SecondaryRidge
                    | MountainKind::Valley
                    | MountainKind::System
            ) || controls.sample_mountain_influence(feature.lon_micro, feature.lat_micro) > 0
        }));
        let mut cancel_octave = || Ok(());
        let (octave_w, octave_h, octave) =
            build_octave(&controls, &identity, 0, 4, &mut cancel_octave).unwrap();
        for j in [0, octave_h - 1] {
            let pole = octave[lattice_index(octave_w, 0, j)];
            for i in 1..octave_w {
                assert_eq!(
                    octave[lattice_index(octave_w, i, j)],
                    pole,
                    "octave polar row {j} varied at column {i}"
                );
            }
        }
        for foothill in model
            .features
            .iter()
            .filter(|feature| feature.kind == MountainKind::Foothill)
        {
            assert!(
                model.features.iter().any(|feature| {
                    feature.kind == MountainKind::Peak
                        && feature.elevation_mm > foothill.elevation_mm
                        && {
                            let distance = chebyshev(
                                model.detail.lattice_width,
                                feature.lattice_index,
                                foothill.lattice_index,
                            );
                            (2..=8).contains(&distance)
                        }
                }),
                "foothill without a taller parent peak in the classification window"
            );
        }
    }

    #[test]
    fn overlapping_octaves_are_byte_equal_across_detail_levels() {
        let (controls, identity, _, _) = controls_at(0);
        let mut cancel = || Ok(());
        for factor in [1_u32, 2, 4, 8] {
            let a = build_octave(&controls, &identity, 0, factor, &mut cancel).unwrap();
            let b = build_octave(&controls, &identity, 0, factor, &mut cancel).unwrap();
            assert_eq!(a, b);
            assert_eq!(a.0, controls.grid.width * factor);
        }
        let key = domain_key(
            &identity,
            ATLAS_DETAIL_ALGORITHM_VERSION,
            0,
            COASTLINE_SYNTHESIS_DOMAIN,
        );
        let grid_w = controls.grid.width;
        let grid_h = controls.grid.height;
        let w4 = grid_w * 4;
        let h4 = grid_h * 4;
        let w8 = grid_w * 8;
        let h8 = grid_h * 8;
        for j in 1..h4.saturating_sub(1) {
            for i in 0..w4 {
                assert_eq!(
                    coastline_noise_ppm(&key, i, j, w4, h4, grid_w),
                    coastline_noise_ppm(&key, i * 2, j * 2, w8, h8, grid_w)
                );
            }
        }
    }

    #[test]
    fn nested_upsample_preserves_coarse_samples() {
        let (controls, identity, _, _) = controls_at(0);
        let mut cancel = || Ok(());
        let (src_w, src_h, src) = stack_octaves(&controls, &identity, 0, 4, &mut cancel).unwrap();
        let stacked_again = stack_octaves(&controls, &identity, 0, 4, &mut cancel).unwrap();
        assert_eq!((src_w, src_h, src.clone()), stacked_again);
        let dst_w = src_w * 2;
        let dst_h = src_h * 2;
        let up = upsample_residual(
            src_w,
            src_h,
            &src,
            dst_w,
            dst_h,
            controls.grid.radius_metres,
            &mut cancel,
        )
        .unwrap();
        let (_, _, fine) = build_octave(&controls, &identity, 0, 8, &mut cancel).unwrap();
        let (stack_w, stack_h, stacked) =
            stack_octaves(&controls, &identity, 0, 8, &mut cancel).unwrap();
        assert_eq!((stack_w, stack_h), (dst_w, dst_h));
        for j in 0..src_h {
            for i in 0..src_w {
                assert_eq!(
                    up[lattice_index(dst_w, i * 2, j * 2)],
                    src[lattice_index(src_w, i, j)],
                    "nested sample drifted at {i},{j}"
                );
            }
        }
        for index in 0..up.len() {
            assert_eq!(stacked[index], up[index].saturating_add(fine[index]));
        }
        let continued = stack_octaves_from(
            &controls,
            &identity,
            0,
            8,
            Some((4, src_w, src_h, src.clone())),
            &mut cancel,
            &mut |_, _, _, _| {},
        )
        .unwrap();
        assert_eq!(continued, (stack_w, stack_h, stacked.clone()));
        let ignored = stack_octaves_from(
            &controls,
            &identity,
            0,
            4,
            Some((8, stack_w, stack_h, stacked)),
            &mut cancel,
            &mut |_, _, _, _| {},
        )
        .unwrap();
        assert_eq!(ignored, (src_w, src_h, src));
    }

    #[test]
    fn mountain_system_ids_match_across_standard_and_detailed() {
        let (controls, identity, _, _) = controls_at(0);
        let mut cancel = || Ok(());
        let standard =
            build_amplification_model(&controls, &identity, 0, DetailLevel::Standard, &mut cancel)
                .unwrap();
        let detailed =
            build_amplification_model(&controls, &identity, 0, DetailLevel::Detailed, &mut cancel)
                .unwrap();
        let ids = |model: &AmplificationModel, kind: MountainKind| {
            model
                .features
                .iter()
                .filter(|feature| feature.kind == kind)
                .map(|feature| feature.id.clone())
                .collect::<std::collections::BTreeSet<_>>()
        };
        assert_eq!(
            ids(&standard, MountainKind::System),
            ids(&detailed, MountainKind::System)
        );
        assert_eq!(
            ids(&standard, MountainKind::Ridge),
            ids(&detailed, MountainKind::Ridge)
        );
        assert_eq!(
            ids(&standard, MountainKind::SecondaryRidge),
            ids(&detailed, MountainKind::SecondaryRidge)
        );
        let systems = |model: &AmplificationModel| {
            model
                .features
                .iter()
                .filter(|feature| feature.kind == MountainKind::System)
                .map(|feature| {
                    (
                        feature.id.clone(),
                        feature.physical_cell,
                        feature.lon_micro,
                        feature.lat_micro,
                    )
                })
                .collect::<Vec<_>>()
        };
        assert_eq!(systems(&standard), systems(&detailed));
    }

    #[test]
    fn world_space_samples_match_across_export_tiles_workers_epochs_and_styles() {
        let (controls, identity, sea, sdf) = controls_at(0);
        let mut cancel = || Ok(());
        let model =
            build_amplification_model(&controls, &identity, 0, DetailLevel::Standard, &mut cancel)
                .unwrap();
        let rebuilt =
            build_amplification_model(&controls, &identity, 0, DetailLevel::Standard, &mut cancel)
                .unwrap();
        assert_eq!(model.detail.residual_mm, rebuilt.detail.residual_mm);
        assert_eq!(model.features, rebuilt.features);

        let (epoch_controls, _, _, _) = controls_at(42);
        let epoch_model = build_amplification_model(
            &epoch_controls,
            &identity,
            0,
            DetailLevel::Standard,
            &mut cancel,
        )
        .unwrap();
        let relief = load_style(RELIEF_STYLE_ID).unwrap();
        let antique = load_style(ANTIQUE_STYLE_ID).unwrap();
        let biome = load_style(BIOME_STYLE_ID).unwrap();
        assert_ne!(relief.0.id, antique.0.id);
        assert_eq!(biome.0.id, BIOME_STYLE_ID);
        assert_ne!(biome.0.biome_forest, biome.0.biome_arid);

        let mut samples = vec![
            (-30_000_000, 10_000_000),
            (179_900_000, 0),
            (0, 89_000_000),
            (12_345_678, -8_000_000),
            (
                pixel_center_lon_micro(40, 256),
                pixel_center_lat_micro(20, 128),
            ),
            (
                pixel_center_lon_micro(80, 512),
                pixel_center_lat_micro(40, 256),
            ),
            xyz_world_pixel_center(2, 1, 1, 16, 16, 256).unwrap(),
            xyz_world_pixel_center(3, 2, 2, 16, 16, 256).unwrap(),
        ];
        let sequential = samples
            .iter()
            .map(|(lon, lat)| model.detail.residual_at(*lon, *lat))
            .collect::<Vec<_>>();
        samples.reverse();
        let reversed = samples
            .iter()
            .rev()
            .map(|(lon, lat)| model.detail.residual_at(*lon, *lat))
            .collect::<Vec<_>>();
        assert_eq!(sequential, reversed);
        let epoch_rebuilt = build_amplification_model(
            &epoch_controls,
            &identity,
            0,
            DetailLevel::Standard,
            &mut cancel,
        )
        .unwrap();
        assert_eq!(
            epoch_model.detail.residual_mm,
            epoch_rebuilt.detail.residual_mm
        );
        for (lon, lat) in samples {
            assert_eq!(
                model.detail.residual_at(lon, lat),
                rebuilt.detail.residual_at(lon, lat)
            );
            assert_eq!(
                epoch_model.detail.residual_at(lon, lat),
                epoch_rebuilt.detail.residual_at(lon, lat)
            );
        }
        let _ = (sea, sdf, relief.0.ocean_deep, antique.0.ocean_deep);
        let unsupported_version = ATLAS_DETAIL_ALGORITHM_VERSION.wrapping_add(1);
        let rejected = AtlasRenderRequest {
            algorithm_version: unsupported_version,
            ..AtlasRenderRequest::spike_png(64, 32).unwrap()
        }
        .normalize();
        assert!(rejected.is_err());
        assert_eq!(ATLAS_DETAIL_ALGORITHM_VERSION, 2);
    }

    #[test]
    fn epoch_controls_change_structure_residual_without_year0_wrapper() {
        let (present, identity, present_sea, _) = controls_at(0);
        let (cold, _, cold_sea, _) = controls_at(-8_000);
        assert_ne!(cold_sea, present_sea);
        let mut cancel = || Ok(());
        let present_model =
            build_amplification_model(&present, &identity, 0, DetailLevel::Standard, &mut cancel)
                .unwrap();
        let cold_model =
            build_amplification_model(&cold, &identity, 0, DetailLevel::Standard, &mut cancel)
                .unwrap();
        assert_ne!(
            present_model.detail.residual_mm,
            cold_model.detail.residual_mm
        );
    }

    #[test]
    fn orometry_ids_use_physical_cell_and_inspect_lists_system_ridge_peak() {
        let (controls, identity, _, _) = controls_at(0);
        let mut cancel = || Ok(());
        let model =
            build_amplification_model(&controls, &identity, 0, DetailLevel::Standard, &mut cancel)
                .unwrap();
        assert!(model
            .features
            .iter()
            .any(|feature| feature.kind == MountainKind::System));
        assert!(model
            .features
            .iter()
            .any(|feature| matches!(feature.kind, MountainKind::Plateau | MountainKind::Upland)));
        for feature in &model.features {
            let prefix = feature_id(feature.kind, feature.physical_cell);
            assert!(
                feature.id == prefix || feature.id.starts_with(&format!("{prefix}:")),
                "orometry id missing physical cell: {}",
                feature.id
            );
            if matches!(feature.kind, MountainKind::Plateau | MountainKind::Upland) {
                assert!(feature.system_id.is_empty());
                assert!(feature.parent_id.is_none());
                continue;
            }
            assert_eq!(
                feature.physical_cell,
                nearest_cell(controls.grid, feature.lon_micro, feature.lat_micro)
            );
            if feature.kind != MountainKind::System
                && feature.lattice_index != feature.physical_cell
            {
                assert!(
                    !feature.id.ends_with(&format!(":{}", feature.lattice_index)),
                    "orometry id still keyed on finest lattice: {}",
                    feature.id
                );
            }
            if feature.kind == MountainKind::System {
                assert!(feature.parent_id.is_none());
                assert_eq!(feature.id, prefix);
            }
            if controls.sample_elevation(feature.lon_micro, feature.lat_micro)
                < controls.sea_level_mm
            {
                assert!(feature.basin_id.is_none());
            }
        }
        let peak = model
            .features
            .iter()
            .find(|feature| feature.kind == MountainKind::Peak)
            .expect("peak");
        let hits = inspect_orometry(&model.features, peak.lon_micro, peak.lat_micro, 2_000_000);
        assert!(hits.len() >= 2, "inspect chain {hits:?}");
        assert_eq!(hits[0].kind, MountainKind::System);
        assert_eq!(hits.last().map(|hit| hit.kind), Some(MountainKind::Peak));
        assert!(hits.iter().any(|hit| is_ridge_kind(hit.kind)));
        let ridge = model
            .features
            .iter()
            .find(|feature| is_ridge_kind(feature.kind))
            .expect("ridge");
        let ridge_hits =
            inspect_orometry(&model.features, ridge.lon_micro, ridge.lat_micro, 2_000_000);
        assert_eq!(ridge_hits[0].kind, MountainKind::System);
        assert!(
            ridge_hits.iter().any(|hit| hit.id == ridge.id),
            "inspect replaced the clicked ridge: {ridge_hits:?}"
        );
        let detailed =
            build_amplification_model(&controls, &identity, 0, DetailLevel::Detailed, &mut cancel)
                .unwrap();
        let systems = |model: &AmplificationModel| {
            model
                .features
                .iter()
                .filter(|feature| feature.kind == MountainKind::System)
                .map(|feature| feature.id.clone())
                .collect::<Vec<_>>()
        };
        let ridges = |model: &AmplificationModel| {
            model
                .features
                .iter()
                .filter(|feature| is_ridge_kind(feature.kind))
                .map(|feature| (feature.id.clone(), feature.parent_id.clone()))
                .collect::<Vec<_>>()
        };
        assert_eq!(systems(&model), systems(&detailed));
        assert_eq!(ridges(&model), ridges(&detailed));
    }

    #[test]
    fn production_grid_lattices_stay_inside_the_in_process_budget() {
        let width = 384_u32;
        let height = 192_u32;
        for factor in [4_u32, 8, 16] {
            let cells = (width as u64) * (height as u64) * u64::from(factor) * u64::from(factor);
            assert!(
                cells * 4 <= 96 * 1024 * 1024,
                "production lattice {width}x{height} x{factor} is {bytes} bytes",
                bytes = cells * 4
            );
        }
    }
}
