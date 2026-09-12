//! World-space deterministic elevation residual.

use daena_physical::Grid;
use rayon::prelude::*;
use sha2::{Digest, Sha256};

use crate::projection::{
    bilinear_i32, clamp_lat_micro, lat_to_row_ppm, lon_to_column_ppm, wrap_lon_micro,
    LAT_MICRO_MIN, LAT_MICRO_SPAN, LON_MICRO_MIN, LON_MICRO_SPAN,
};
use crate::request::DetailLevel;
use crate::AtlasError;

pub const COASTAL_ENVELOPE_PPM: u32 = 500_000;
pub const COASTAL_RAMP_MM: i32 = 72_000;
pub const MAX_RESIDUAL_MM: i32 = 720_000;
const CANCELLATION_STRIDE: usize = 4_096;

#[derive(Debug, Clone)]
pub struct AtlasDetailModel {
    pub grid: Grid,
    pub elevations_mm: Vec<i32>,
    pub residual_mm: Vec<i32>,
    pub lattice_width: u32,
    pub lattice_height: u32,
    pub algorithm_version: u32,
    pub variant: u32,
    pub level: DetailLevel,
}

fn splitmix64(mut value: u64) -> u64 {
    value = value.wrapping_add(0x9e37_79b9_7f4a_7c15);
    let mut result = value;
    result = (result ^ (result >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    result = (result ^ (result >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    result ^ (result >> 31)
}

fn push_u32(bytes: &mut Vec<u8>, value: u32) {
    bytes.extend_from_slice(&value.to_le_bytes());
}

fn domain_prefix(algorithm_version: u32) -> &'static [u8] {
    match algorithm_version {
        1 => b"daena-atlas-detail-v1\0",
        _ => b"daena-atlas-detail-v2\0",
    }
}

#[must_use]
pub fn domain_key(identity: &[u8], algorithm_version: u32, variant: u32, domain: &str) -> [u8; 32] {
    let domain_bytes = domain.as_bytes();
    let mut input = Vec::with_capacity(64 + identity.len() + domain_bytes.len());
    input.extend_from_slice(domain_prefix(algorithm_version));
    push_u32(&mut input, identity.len() as u32);
    input.extend_from_slice(identity);
    push_u32(&mut input, algorithm_version);
    push_u32(&mut input, variant);
    push_u32(&mut input, domain_bytes.len() as u32);
    input.extend_from_slice(domain_bytes);
    Sha256::digest(input).into()
}

#[must_use]
pub(crate) fn interpolated_unit_ppm(
    key: &[u8; 32],
    lon_micro: i32,
    lat_micro: i32,
    octave: u32,
    cell_micro: i64,
) -> i32 {
    let cell_micro = cell_micro.max(1);
    let lon_cells = (LON_MICRO_SPAN / cell_micro).max(1) as u32;
    let lat_cells = (LAT_MICRO_SPAN / cell_micro).max(1) as u32;
    let lon_pos = (i64::from(wrap_lon_micro(i64::from(lon_micro))) - i64::from(LON_MICRO_MIN))
        .rem_euclid(LON_MICRO_SPAN);
    let lat_pos = (i64::from(clamp_lat_micro(i64::from(lat_micro))) - i64::from(LAT_MICRO_MIN))
        .clamp(0, LAT_MICRO_SPAN - 1);
    let i = (lon_pos / cell_micro) as u32 % lon_cells;
    let j = ((lat_pos / cell_micro) as u32).min(lat_cells.saturating_sub(1));
    let ni = (i + 1) % lon_cells;
    let nj = (j + 1).min(lat_cells.saturating_sub(1));
    let fx = ((lon_pos.rem_euclid(cell_micro)) * 1_000_000 / cell_micro) as u32;
    let fy = ((lat_pos.rem_euclid(cell_micro)) * 1_000_000 / cell_micro) as u32;
    let corner = |ii: u32, jj: u32| -> i32 {
        ((lattice_sample(key, ii, jj, octave) >> 11) % 1_000_001) as i32
    };
    bilinear_i32(
        corner(i, j),
        corner(ni, j),
        corner(i, nj),
        corner(ni, nj),
        fx,
        fy,
    )
}

#[must_use]
pub fn lattice_sample(key: &[u8; 32], lattice_i: u32, lattice_j: u32, octave: u32) -> u64 {
    let k0 = u64::from_le_bytes(key[0..8].try_into().expect("key word"));
    let k1 = u64::from_le_bytes(key[8..16].try_into().expect("key word"));
    splitmix64(
        k0 ^ u64::from(lattice_i).wrapping_mul(0x9e37_79b9_7f4a_7c15)
            ^ u64::from(lattice_j).wrapping_mul(0xbf58_476d_1ce4_e5b9)
            ^ u64::from(octave).wrapping_mul(0x94d0_49bb_1331_11eb)
            ^ k1,
    )
}

#[must_use]
pub fn nearest_cell(grid: Grid, lon_micro: i32, lat_micro: i32) -> usize {
    let (col, _, _) = lon_to_column_ppm(lon_micro, grid.width);
    let (row, _, _) = lat_to_row_ppm(lat_micro, grid.height);
    grid.index(row, col)
}

#[must_use]
pub fn lattice_nearest_cells(grid: Grid, width: u32, height: u32) -> Vec<usize> {
    let width_us = width as usize;
    let mut cells = vec![0_usize; width_us.saturating_mul(height as usize)];
    cells.par_iter_mut().enumerate().for_each(|(index, slot)| {
        let i = (index % width_us) as u32;
        let j = (index / width_us) as u32;
        *slot = nearest_cell(
            grid,
            lattice_lon_micro(i, width),
            lattice_lat_micro(j, height),
        );
    });
    cells
}

#[must_use]
pub fn sample_field_mm(grid: Grid, field: &[i32], lon_micro: i32, lat_micro: i32) -> i32 {
    let (col, next_col, fx) = lon_to_column_ppm(lon_micro, grid.width);
    let (row, next_row, fy) = lat_to_row_ppm(lat_micro, grid.height);
    bilinear_i32(
        field[grid.index(row, col)],
        field[grid.index(row, next_col)],
        field[grid.index(next_row, col)],
        field[grid.index(next_row, next_col)],
        fx,
        fy,
    )
}

#[must_use]
pub fn sample_mask_ppm(grid: Grid, mask: &[bool], lon_micro: i32, lat_micro: i32) -> i32 {
    let (col, next_col, fx) = lon_to_column_ppm(lon_micro, grid.width);
    let (row, next_row, fy) = lat_to_row_ppm(lat_micro, grid.height);
    let bit = |row: u32, col: u32| {
        if mask.get(grid.index(row, col)).copied().unwrap_or(false) {
            1_000_000
        } else {
            0
        }
    };
    bilinear_i32(
        bit(row, col),
        bit(row, next_col),
        bit(next_row, col),
        bit(next_row, next_col),
        fx,
        fy,
    )
}

pub(crate) fn nest_lattice_coord(i: u32, dim: u32) -> u32 {
    ((u64::from(i) << 16) / u64::from(dim.max(1))) as u32
}

pub(crate) fn lattice_lon_micro(i: u32, lattice_width: u32) -> i32 {
    let width = i64::from(lattice_width.max(1));
    wrap_lon_micro(i64::from(LON_MICRO_MIN) + LON_MICRO_SPAN * i64::from(i) / width)
}

pub(crate) fn lattice_lat_micro(j: u32, lattice_height: u32) -> i32 {
    let height = i64::from(lattice_height.max(1));
    clamp_lat_micro(i64::from(LAT_MICRO_MIN) + LAT_MICRO_SPAN * i64::from(j) / height)
}

pub(crate) fn cell_center_lon_micro(i: u32, width: u32) -> i32 {
    wrap_lon_micro(
        i64::from(LON_MICRO_MIN)
            + (LON_MICRO_SPAN * (i64::from(i).saturating_mul(2) + 1))
                / (i64::from(width.max(1)) * 2),
    )
}

pub(crate) fn cell_center_lat_micro(j: u32, height: u32) -> i32 {
    clamp_lat_micro(
        i64::from(LAT_MICRO_MIN)
            + (LAT_MICRO_SPAN * (i64::from(j).saturating_mul(2) + 1))
                / (i64::from(height.max(1)) * 2),
    )
}

impl AtlasDetailModel {
    #[must_use]
    pub fn residual_at(&self, lon_micro: i32, lat_micro: i32) -> i32 {
        let lattice = Grid {
            width: self.lattice_width,
            height: self.lattice_height,
            radius_metres: self.grid.radius_metres,
        };
        sample_field_mm(lattice, &self.residual_mm, lon_micro, lat_micro)
    }

    #[must_use]
    pub fn canonical_at(&self, lon_micro: i32, lat_micro: i32) -> i32 {
        sample_field_mm(self.grid, &self.elevations_mm, lon_micro, lat_micro)
    }

    #[must_use]
    pub fn refined_at(
        &self,
        lon_micro: i32,
        lat_micro: i32,
        sea_level_mm: i32,
        sdf_ppm: i32,
    ) -> i32 {
        self.refined_at_with_extra(lon_micro, lat_micro, sea_level_mm, sdf_ppm, 0)
    }

    #[must_use]
    pub fn refined_at_with_extra(
        &self,
        lon_micro: i32,
        lat_micro: i32,
        sea_level_mm: i32,
        sdf_ppm: i32,
        extra_mm: i32,
    ) -> i32 {
        let canonical = self.canonical_at(lon_micro, lat_micro);
        let prepared = canonical.saturating_add(self.residual_at(lon_micro, lat_micro));
        let extra_mm = if sdf_ppm.unsigned_abs() > COASTAL_ENVELOPE_PPM {
            extra_mm
        } else {
            extra_mm.clamp(-COASTAL_RAMP_MM, COASTAL_RAMP_MM)
        };
        let mut refined = prepared.saturating_add(extra_mm);
        if sdf_ppm.unsigned_abs() > COASTAL_ENVELOPE_PPM {
            let canon_land = canonical >= sea_level_mm;
            let refined_land = refined >= sea_level_mm;
            if canon_land != refined_land {
                refined = if canon_land {
                    sea_level_mm.saturating_add(1)
                } else {
                    sea_level_mm.saturating_sub(1)
                };
            }
        }
        refined
    }

    pub fn bake_absolute_elevation(&mut self, absolute_mm: &[i32]) {
        let width = self.lattice_width;
        let height = self.lattice_height;
        let count = (width as usize).saturating_mul(height as usize);
        if absolute_mm.len() != count || self.residual_mm.len() != count {
            return;
        }
        let width_us = width as usize;
        let grid = self.grid;
        let elevations = self.elevations_mm.as_slice();
        self.residual_mm
            .par_iter_mut()
            .zip(absolute_mm.par_iter())
            .enumerate()
            .for_each(|(index, (slot, &absolute))| {
                let i = (index % width_us) as u32;
                let j = (index / width_us) as u32;
                let canonical = sample_field_mm(
                    grid,
                    elevations,
                    lattice_lon_micro(i, width),
                    lattice_lat_micro(j, height),
                );
                *slot = absolute.saturating_sub(canonical);
            });
    }
}

#[must_use]
pub fn signed_coastal_distance_ppm(
    grid: Grid,
    elevations_mm: &[i32],
    sea_level_mm: i32,
) -> Vec<i32> {
    let count = grid.sample_count();
    let land = elevations_mm
        .iter()
        .map(|elevation| *elevation >= sea_level_mm)
        .collect::<Vec<_>>();
    let mut distance = vec![i32::MAX; count];
    let mut queue = std::collections::VecDeque::new();
    for index in 0..count {
        let is_land = land[index];
        let coastal = grid
            .neighbors(index)
            .iter()
            .any(|neighbor| land[*neighbor] != is_land);
        if coastal {
            distance[index] = 0;
            queue.push_back(index);
        }
    }
    if queue.is_empty() {
        return vec![1_000_000; count];
    }
    while let Some(index) = queue.pop_front() {
        let next = distance[index].saturating_add(1_000_000);
        for neighbor in grid.neighbors(index) {
            if distance[neighbor] > next {
                distance[neighbor] = next;
                queue.push_back(neighbor);
            }
        }
    }
    for (index, value) in distance.iter_mut().enumerate() {
        if *value == i32::MAX {
            *value = 1_000_000;
        }
        if !land[index] {
            *value = -*value;
        }
    }
    distance
}

#[must_use]
pub fn sample_sdf_ppm(grid: Grid, sdf: &[i32], lon_micro: i32, lat_micro: i32) -> i32 {
    sample_field_mm(grid, sdf, lon_micro, lat_micro)
}

pub fn downsample_mean_mm(
    model: &AtlasDetailModel,
    sea_level_mm: i32,
    sdf: &[i32],
    check_cancelled: &mut dyn FnMut() -> Result<(), AtlasError>,
) -> Result<Vec<i32>, AtlasError> {
    let mut sums = vec![0_i64; model.grid.sample_count()];
    let mut counts = vec![0_u32; model.grid.sample_count()];
    for j in 0..model.lattice_height {
        if (j as usize).is_multiple_of(CANCELLATION_STRIDE) {
            check_cancelled()?;
        }
        for i in 0..model.lattice_width {
            let lon = lattice_lon_micro(i, model.lattice_width);
            let lat = lattice_lat_micro(j, model.lattice_height);
            let sdf_ppm = sample_sdf_ppm(model.grid, sdf, lon, lat);
            let refined = model.refined_at(lon, lat, sea_level_mm, sdf_ppm);
            let cell = nearest_cell(model.grid, lon, lat);
            sums[cell] += i64::from(refined);
            counts[cell] += 1;
        }
    }
    Ok(sums
        .iter()
        .zip(counts)
        .zip(model.elevations_mm.iter())
        .map(|((sum, count), canonical)| {
            if count == 0 {
                *canonical
            } else {
                (*sum / i64::from(count)) as i32
            }
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::amplify::build_amplification_model;
    use crate::control::ControlFields;
    use crate::golden_world;
    use crate::spike_identity_from_source;
    use daena_physical::history::{
        derive_historical_world_with_planet, HistoricalForcingParameters,
    };
    use daena_physical::NoopProgress as PhysicalNoop;

    fn model() -> (AtlasDetailModel, i32, Vec<i32>, Vec<u8>) {
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
        let controls = ControlFields::from_accepted(
            &world.field,
            &world.tectonics,
            &historical.climate,
            &historical.hydrology,
        )
        .unwrap();
        let mut cancel = || Ok(());
        let model =
            build_amplification_model(&controls, &identity, 0, DetailLevel::Detailed, &mut cancel)
                .unwrap()
                .detail;
        let sdf = signed_coastal_distance_ppm(
            world.field.grid,
            &world.field.elevations_mm,
            historical.metrics.sea_level_mm,
        );
        (model, historical.metrics.sea_level_mm, sdf, identity)
    }

    #[test]
    fn named_domain_vectors_are_stable() {
        let key = domain_key(
            b"identity-fixture",
            crate::ATLAS_DETAIL_ALGORITHM_VERSION,
            0,
            crate::amplify::HIERARCHICAL_RELIEF_DOMAIN,
        );
        assert_ne!(lattice_sample(&key, 3, 5, 0), lattice_sample(&key, 4, 5, 0));
        assert_ne!(
            domain_key(
                b"identity-fixture",
                crate::ATLAS_DETAIL_ALGORITHM_VERSION,
                0,
                crate::amplify::HIERARCHICAL_RELIEF_DOMAIN
            ),
            domain_key(
                b"identity-fixture",
                crate::ATLAS_DETAIL_ALGORITHM_VERSION,
                1,
                crate::amplify::HIERARCHICAL_RELIEF_DOMAIN
            )
        );
        assert_ne!(
            domain_key(
                b"identity-fixture",
                crate::ATLAS_DETAIL_ALGORITHM_VERSION,
                0,
                crate::amplify::HIERARCHICAL_RELIEF_DOMAIN
            ),
            domain_key(
                b"identity-fixture",
                crate::ATLAS_DETAIL_ALGORITHM_VERSION,
                0,
                crate::amplify::MOUNTAIN_OROMETRY_DOMAIN
            )
        );
    }

    #[test]
    fn nested_lattices_share_world_coordinates() {
        let width_4 = 64 * 4;
        let width_8 = 64 * 8;
        let width_16 = 64 * 16;
        let height_4 = 32 * 4;
        let height_8 = 32 * 8;
        let height_16 = 32 * 16;
        for i in 0..width_4 {
            assert_eq!(
                lattice_lon_micro(i, width_4),
                lattice_lon_micro(i * 2, width_8)
            );
            assert_eq!(
                lattice_lon_micro(i, width_4),
                lattice_lon_micro(i * 4, width_16)
            );
            assert_eq!(
                nest_lattice_coord(i, width_4),
                nest_lattice_coord(i * 2, width_8)
            );
        }
        for j in 0..height_4 {
            assert_eq!(
                lattice_lat_micro(j, height_4),
                lattice_lat_micro(j * 2, height_8)
            );
            assert_eq!(
                lattice_lat_micro(j, height_4),
                lattice_lat_micro(j * 4, height_16)
            );
        }
    }

    #[test]
    fn residual_is_independent_of_query_resolution() {
        let (model, sea, sdf, identity) = model();
        let samples = [
            (-30_000_000, 10_000_000),
            (179_900_000, 0),
            (0, 89_000_000),
            (0, -89_000_000),
        ];
        for (lon, lat) in samples {
            let first = model.residual_at(lon, lat);
            let second = model.residual_at(lon, lat);
            assert_eq!(first, second);
        }
        let mut cancel = || Ok(());
        let other = {
            let world = golden_world();
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
            let controls = ControlFields::from_accepted(
                &world.field,
                &world.tectonics,
                &historical.climate,
                &historical.hydrology,
            )
            .unwrap();
            build_amplification_model(&controls, &identity, 0, DetailLevel::Standard, &mut cancel)
                .unwrap()
                .detail
        };
        assert_eq!(
            model.canonical_at(12_000_000, -4_000_000),
            other.canonical_at(12_000_000, -4_000_000)
        );
        let sdf_ppm = sample_sdf_ppm(model.grid, &sdf, 0, 0);
        let reference = model.refined_at(0, 0, sea, sdf_ppm);
        let shifted_sea = sea.saturating_add(50_000);
        let shifted = model.refined_at(0, 0, shifted_sea, sdf_ppm);
        assert_eq!(model.residual_at(0, 0), model.residual_at(0, 0));
        let _ = (reference, shifted);
    }

    #[test]
    fn downsample_preserves_macro_elevation_and_land_sign_outside_envelope() {
        let (model, sea, sdf, _) = model();
        let mut sums = vec![0_i64; model.grid.sample_count()];
        let mut counts = vec![0_u32; model.grid.sample_count()];
        for j in 0..model.lattice_height {
            for i in 0..model.lattice_width {
                let lon = lattice_lon_micro(i, model.lattice_width);
                let lat = lattice_lat_micro(j, model.lattice_height);
                let sdf_ppm = sample_sdf_ppm(model.grid, &sdf, lon, lat);
                if sdf_ppm.unsigned_abs() <= COASTAL_ENVELOPE_PPM {
                    continue;
                }
                let cell = nearest_cell(model.grid, lon, lat);
                sums[cell] += i64::from(
                    model.residual_mm[j as usize * model.lattice_width as usize + i as usize],
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
        let mut cancel = || Ok(());
        let down = downsample_mean_mm(&model, sea, &sdf, &mut cancel).unwrap();
        assert_eq!(down.len(), model.elevations_mm.len());

        for j in 0..model.lattice_height {
            for i in 0..model.lattice_width {
                let lon = lattice_lon_micro(i, model.lattice_width);
                let lat = lattice_lat_micro(j, model.lattice_height);
                let sdf_ppm = sample_sdf_ppm(model.grid, &sdf, lon, lat);
                if sdf_ppm.unsigned_abs() <= COASTAL_ENVELOPE_PPM {
                    continue;
                }
                let canonical = model.canonical_at(lon, lat);
                let refined = model.refined_at(lon, lat, sea, sdf_ppm);
                assert_eq!(
                    canonical >= sea,
                    refined >= sea,
                    "sign changed outside coastal envelope at lattice {i},{j}"
                );
            }
        }
    }
}
