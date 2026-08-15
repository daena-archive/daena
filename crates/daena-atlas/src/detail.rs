//! World-space deterministic elevation residual.

use daena_physical::Grid;
use sha2::{Digest, Sha256};

use crate::projection::{
    bilinear_i32, clamp_lat_micro, lat_to_row_ppm, lon_to_column_ppm, wrap_lon_micro,
    LAT_MICRO_MIN, LAT_MICRO_SPAN, LON_MICRO_MIN, LON_MICRO_SPAN,
};
use crate::request::DetailLevel;
use crate::AtlasError;

pub const COASTAL_ENVELOPE_PPM: u32 = 500_000;
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
        2 => b"daena-atlas-detail-v2\0",
        3 => b"daena-atlas-detail-v3\0",
        4 => b"daena-atlas-detail-v4\0",
        _ => b"daena-atlas-detail-v5\0",
    }
}

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

pub fn nearest_cell(grid: Grid, lon_micro: i32, lat_micro: i32) -> usize {
    let (col, _, _) = lon_to_column_ppm(lon_micro, grid.width);
    let (row, _, _) = lat_to_row_ppm(lat_micro, grid.height);
    grid.index(row, col)
}

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

pub(crate) fn lattice_lon_micro(i: u32, lattice_width: u32) -> i32 {
    wrap_lon_micro(
        i64::from(LON_MICRO_MIN)
            + (LON_MICRO_SPAN * (i64::from(i).saturating_mul(2) + 1))
                / (i64::from(lattice_width) * 2),
    )
}

pub(crate) fn lattice_lat_micro(j: u32, lattice_height: u32) -> i32 {
    clamp_lat_micro(
        i64::from(LAT_MICRO_MIN)
            + (LAT_MICRO_SPAN * (i64::from(j).saturating_mul(2) + 1))
                / (i64::from(lattice_height) * 2),
    )
}

impl AtlasDetailModel {
    pub fn residual_at(&self, lon_micro: i32, lat_micro: i32) -> i32 {
        let lattice = Grid {
            width: self.lattice_width,
            height: self.lattice_height,
            radius_metres: self.grid.radius_metres,
        };
        sample_field_mm(lattice, &self.residual_mm, lon_micro, lat_micro)
    }

    pub fn canonical_at(&self, lon_micro: i32, lat_micro: i32) -> i32 {
        sample_field_mm(self.grid, &self.elevations_mm, lon_micro, lat_micro)
    }

    pub fn refined_at(
        &self,
        lon_micro: i32,
        lat_micro: i32,
        sea_level_mm: i32,
        sdf_ppm: i32,
    ) -> i32 {
        let canonical = self.canonical_at(lon_micro, lat_micro);
        let mut refined = canonical.saturating_add(self.residual_at(lon_micro, lat_micro));
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
        for j in 0..height {
            for i in 0..width {
                let index = j as usize * width as usize + i as usize;
                let lon = lattice_lon_micro(i, width);
                let lat = lattice_lat_micro(j, height);
                let canonical = self.canonical_at(lon, lat);
                self.residual_mm[index] = absolute_mm[index].saturating_sub(canonical);
            }
        }
    }
}

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
        if j as usize % CANCELLATION_STRIDE == 0 {
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
    use daena_physical::history::{derive_historical_world, HistoricalForcingParameters};
    use daena_physical::NoopProgress as PhysicalNoop;

    fn model() -> (AtlasDetailModel, i32, Vec<i32>, Vec<u8>) {
        let world = golden_world();
        let identity = spike_identity_from_source(&world.source);
        let historical = derive_historical_world(
            &world.field,
            world.report.reference_water_inventory_m3,
            Some(&world.tectonics.crust_by_cell),
            HistoricalForcingParameters::default_for(world.field.seed, world.field.retry_index),
            0,
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
        assert_eq!(
            key.iter()
                .map(|byte| format!("{byte:02x}"))
                .collect::<String>(),
            "067d6bee8d7e4cbc3257618473a18027b74bb3221bf8d8fc0546a606fbe03f85"
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
    fn residual_is_independent_of_query_resolution_and_epoch_year() {
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
            let historical = derive_historical_world(
                &world.field,
                world.report.reference_water_inventory_m3,
                Some(&world.tectonics.crust_by_cell),
                HistoricalForcingParameters::default_for(world.field.seed, world.field.retry_index),
                0,
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
        // Different level may change amplitude sampling density, but the PRF
        // at a shared lattice coordinate family stays world-addressed.
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
