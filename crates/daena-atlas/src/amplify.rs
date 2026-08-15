//! Experimental detail algorithm 2: hierarchical amplification and bounded
//! mountain topology. Production requests continue to use algorithm 1.

use daena_physical::Grid;

use crate::control::ControlFields;
use crate::detail::{
    domain_key, lattice_lat_micro, lattice_lon_micro, lattice_sample, nearest_cell,
    sample_field_mm, sample_sdf_ppm, AtlasDetailModel, COASTAL_ENVELOPE_PPM, MAX_RESIDUAL_MM,
};
use crate::request::DetailLevel;
use crate::{AtlasError, ATLAS_DETAIL_ALGORITHM_EXPERIMENTAL_VERSION};

pub const HIERARCHICAL_RELIEF_DOMAIN: &str = "hierarchical-relief";
pub const MOUNTAIN_OROMETRY_DOMAIN: &str = "mountain-orometry";
pub const MOUNTAIN_WINDOW_WIDTH: u32 = 16;
pub const MOUNTAIN_WINDOW_HEIGHT: u32 = 12;
pub const MAX_MOUNTAIN_FEATURES: usize = 64;
const CANCELLATION_STRIDE: usize = 4_096;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum MountainKind {
    Peak,
    Saddle,
    Ridge,
    Valley,
    Foothill,
}

impl MountainKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Peak => "peak",
            Self::Saddle => "saddle",
            Self::Ridge => "ridge",
            Self::Valley => "valley",
            Self::Foothill => "foothill",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MountainFeature {
    pub id: String,
    pub kind: MountainKind,
    pub lattice_index: usize,
    pub lon_micro: i32,
    pub lat_micro: i32,
    pub elevation_mm: i32,
}

#[derive(Debug, Clone)]
pub struct AmplificationModel {
    pub detail: AtlasDetailModel,
    pub mountain_origin_row: u32,
    pub mountain_origin_col: u32,
    pub features: Vec<MountainFeature>,
    pub orometry_key: [u8; 32],
}

fn lattice_index(width: u32, i: u32, j: u32) -> usize {
    j as usize * width as usize + i as usize
}

fn signed_unit_mm(sample: u64, amplitude_mm: i32) -> i32 {
    let unit = (sample >> 11) as i64 % 2_000_001 - 1_000_000;
    ((unit * i64::from(amplitude_mm)) / 1_000_000) as i32
}

fn amplitude_mm(elevation_mm: i32, crust_ppm: i32) -> i32 {
    let magnitude = elevation_mm.unsigned_abs().min(8_000_000);
    let scaled = (u64::from(magnitude) * u64::from(MAX_RESIDUAL_MM as u32) / 8_000_000) as i32;
    let crust = crust_ppm.clamp(0, 1_000_000);
    let conditioned = (i64::from(scaled.clamp(40, MAX_RESIDUAL_MM)) * i64::from(crust)
        + 500_000 * 80)
        / 1_000_000;
    (conditioned as i32).clamp(24, MAX_RESIDUAL_MM)
}

fn octave_factor(level: DetailLevel, octave: u32) -> u32 {
    let finest = level.lattice_factor();
    match octave {
        0 => (finest / 4).max(1),
        1 => (finest / 2).max(1),
        _ => finest,
    }
}

fn octave_weight_ppm(octave: u32) -> i32 {
    match octave {
        0 => 1_000_000,
        1 => 500_000,
        _ => 250_000,
    }
}

fn build_octave(
    controls: &ControlFields,
    identity: &[u8],
    variant: u32,
    factor: u32,
    weight_ppm: i32,
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
        ATLAS_DETAIL_ALGORITHM_EXPERIMENTAL_VERSION,
        variant,
        HIERARCHICAL_RELIEF_DOMAIN,
    );
    let mut residual = vec![0_i32; count];
    for j in 0..height {
        if j as usize % CANCELLATION_STRIDE == 0 {
            check_cancelled()?;
        }
        let polar = j == 0 || j + 1 == height;
        let sample_j = if polar { 0 } else { j };
        for i in 0..width {
            let sample_i = if polar { 0 } else { i };
            let lon = lattice_lon_micro(if polar { 0 } else { i }, width);
            let lat = lattice_lat_micro(j, height);
            let elevation = controls.sample_elevation(lon, lat);
            let crust = controls.sample_crust_influence(lon, lat);
            let sample = lattice_sample(&key, sample_i, sample_j, factor);
            let unit = signed_unit_mm(sample, amplitude_mm(elevation, crust));
            residual[lattice_index(width, i, j)] =
                ((i64::from(unit) * i64::from(weight_ppm)) / 1_000_000) as i32;
        }
    }
    Ok((width, height, residual))
}

fn mean_remove(
    grid: Grid,
    lattice_width: u32,
    lattice_height: u32,
    residual: &mut [i32],
    check_cancelled: &mut dyn FnMut() -> Result<(), AtlasError>,
) -> Result<(), AtlasError> {
    let mut sums = vec![0_i64; grid.sample_count()];
    let mut counts = vec![0_u32; grid.sample_count()];
    for j in 0..lattice_height {
        for i in 0..lattice_width {
            let lon = lattice_lon_micro(i, lattice_width);
            let lat = lattice_lat_micro(j, lattice_height);
            let cell = nearest_cell(grid, lon, lat);
            sums[cell] += i64::from(residual[lattice_index(lattice_width, i, j)]);
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
    for j in 0..lattice_height {
        if j % 8 == 0 {
            check_cancelled()?;
        }
        for i in 0..lattice_width {
            let lon = lattice_lon_micro(i, lattice_width);
            let lat = lattice_lat_micro(j, lattice_height);
            let cell = nearest_cell(grid, lon, lat);
            let index = lattice_index(lattice_width, i, j);
            residual[index] = (i64::from(residual[index]) - means[cell]) as i32;
        }
    }
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

pub fn mountain_window_origin(controls: &ControlFields) -> (u32, u32) {
    let mut best_sum = i64::MIN;
    let mut best = (0, 0);
    let max_row = controls.grid.height.saturating_sub(MOUNTAIN_WINDOW_HEIGHT);
    let max_col = controls.grid.width.saturating_sub(MOUNTAIN_WINDOW_WIDTH);
    for row in 0..=max_row {
        for col in 0..=max_col {
            let mut sum = 0_i64;
            for dr in 0..MOUNTAIN_WINDOW_HEIGHT {
                for dc in 0..MOUNTAIN_WINDOW_WIDTH {
                    let cell = controls.grid.index(row + dr, col + dc);
                    sum += i64::from(controls.mountain_influence_ppm[cell]);
                }
            }
            if sum > best_sum {
                best_sum = sum;
                best = (row, col);
            }
        }
    }
    best
}

fn in_window(grid: Grid, origin_row: u32, origin_col: u32, lon_micro: i32, lat_micro: i32) -> bool {
    let cell = nearest_cell(grid, lon_micro, lat_micro);
    let (row, col) = grid.row_col(cell);
    row >= origin_row
        && row < origin_row + MOUNTAIN_WINDOW_HEIGHT
        && col >= origin_col
        && col < origin_col + MOUNTAIN_WINDOW_WIDTH
}

fn feature_id(kind: MountainKind, index: usize) -> String {
    format!(
        "atlas:orometry:v{}:{}:{index}",
        ATLAS_DETAIL_ALGORITHM_EXPERIMENTAL_VERSION,
        kind.as_str()
    )
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

fn extract_mountain_features(
    controls: &ControlFields,
    lattice_width: u32,
    lattice_height: u32,
    residual: &[i32],
    origin_row: u32,
    origin_col: u32,
) -> Vec<MountainFeature> {
    let mut window = Vec::new();
    for j in 0..lattice_height {
        for i in 0..lattice_width {
            let lon = lattice_lon_micro(i, lattice_width);
            let lat = lattice_lat_micro(j, lattice_height);
            if !in_window(controls.grid, origin_row, origin_col, lon, lat) {
                continue;
            }
            if controls.sample_mountain_influence(lon, lat) <= 0 {
                continue;
            }
            let elevation =
                controls.sample_elevation(lon, lat) + residual[lattice_index(lattice_width, i, j)];
            window.push((elevation, i, j, lon, lat));
        }
    }
    window.sort_by(|a, b| {
        b.0.cmp(&a.0)
            .then_with(|| a.1.cmp(&b.1))
            .then_with(|| a.2.cmp(&b.2))
    });
    let mut index_of = vec![usize::MAX; lattice_width as usize * lattice_height as usize];
    for (slot, &(_, i, j, _, _)) in window.iter().enumerate() {
        index_of[lattice_index(lattice_width, i, j)] = slot;
    }
    let mut uf = UnionFind::new(window.len());
    let mut seen = vec![false; window.len()];
    let mut peak_of = vec![None; window.len()];
    let mut features = Vec::new();
    for (slot, &(elevation, i, j, lon, lat)) in window.iter().enumerate() {
        seen[slot] = true;
        let mut higher_neighbor = false;
        let mut merge_slots = Vec::new();
        for (ni, nj) in lattice_neighbors(lattice_width, lattice_height, i, j) {
            let neighbor_index = lattice_index(lattice_width, ni, nj);
            let neighbor_slot = index_of[neighbor_index];
            if neighbor_slot == usize::MAX {
                continue;
            }
            if window[neighbor_slot].0 > elevation
                || (window[neighbor_slot].0 == elevation && neighbor_slot < slot)
            {
                higher_neighbor = true;
            }
            if seen[neighbor_slot] {
                merge_slots.push(neighbor_slot);
            }
        }
        if !higher_neighbor && features.len() < MAX_MOUNTAIN_FEATURES {
            peak_of[slot] = Some(features.len());
            features.push(MountainFeature {
                id: feature_id(MountainKind::Peak, lattice_index(lattice_width, i, j)),
                kind: MountainKind::Peak,
                lattice_index: lattice_index(lattice_width, i, j),
                lon_micro: lon,
                lat_micro: lat,
                elevation_mm: elevation,
            });
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
                if features.len() < MAX_MOUNTAIN_FEATURES {
                    features.push(MountainFeature {
                        id: feature_id(MountainKind::Saddle, lattice_index(lattice_width, i, j)),
                        kind: MountainKind::Saddle,
                        lattice_index: lattice_index(lattice_width, i, j),
                        lon_micro: lon,
                        lat_micro: lat,
                        elevation_mm: elevation,
                    });
                }
                if features.len() < MAX_MOUNTAIN_FEATURES {
                    features.push(MountainFeature {
                        id: feature_id(MountainKind::Ridge, lattice_index(lattice_width, i, j)),
                        kind: MountainKind::Ridge,
                        lattice_index: lattice_index(lattice_width, i, j),
                        lon_micro: lon,
                        lat_micro: lat,
                        elevation_mm: elevation,
                    });
                }
            }
        }
    }
    for j in 1..lattice_height.saturating_sub(1) {
        for i in 0..lattice_width {
            let lon = lattice_lon_micro(i, lattice_width);
            let lat = lattice_lat_micro(j, lattice_height);
            if !in_window(controls.grid, origin_row, origin_col, lon, lat) {
                continue;
            }
            if controls.sample_mountain_influence(lon, lat) <= 0 {
                continue;
            }
            let elevation =
                controls.sample_elevation(lon, lat) + residual[lattice_index(lattice_width, i, j)];
            if elevation < controls.sea_level_mm {
                continue;
            }
            let mut local_min = true;
            let mut near_ridge = false;
            for (ni, nj) in lattice_neighbors(lattice_width, lattice_height, i, j) {
                let neighbor_elev = controls.sample_elevation(
                    lattice_lon_micro(ni, lattice_width),
                    lattice_lat_micro(nj, lattice_height),
                ) + residual[lattice_index(lattice_width, ni, nj)];
                if neighbor_elev < elevation {
                    local_min = false;
                }
            }
            let index = lattice_index(lattice_width, i, j);
            if features.iter().any(|feature| {
                matches!(feature.kind, MountainKind::Ridge | MountainKind::Peak)
                    && chebyshev(lattice_width, feature.lattice_index, index) <= 3
            }) {
                near_ridge = true;
            }
            if local_min && features.len() < MAX_MOUNTAIN_FEATURES {
                features.push(MountainFeature {
                    id: feature_id(MountainKind::Valley, index),
                    kind: MountainKind::Valley,
                    lattice_index: index,
                    lon_micro: lon,
                    lat_micro: lat,
                    elevation_mm: elevation,
                });
            } else if near_ridge
                && !local_min
                && features.len() < MAX_MOUNTAIN_FEATURES
                && features
                    .iter()
                    .all(|feature| feature.lattice_index != index)
            {
                let Some(peak_elevation) = features
                    .iter()
                    .filter(|feature| feature.kind == MountainKind::Peak)
                    .map(|feature| {
                        (
                            chebyshev(lattice_width, feature.lattice_index, index),
                            feature.elevation_mm,
                        )
                    })
                    .min_by_key(|(distance, elevation)| (*distance, -*elevation))
                    .map(|(_, elevation)| elevation)
                else {
                    continue;
                };
                if elevation >= peak_elevation {
                    continue;
                }
                features.push(MountainFeature {
                    id: feature_id(MountainKind::Foothill, index),
                    kind: MountainKind::Foothill,
                    lattice_index: index,
                    lon_micro: lon,
                    lat_micro: lat,
                    elevation_mm: elevation,
                });
            }
        }
    }
    features.truncate(MAX_MOUNTAIN_FEATURES);
    features.sort_by(|a, b| a.id.cmp(&b.id));
    features
}

fn chebyshev(width: u32, a: usize, b: usize) -> u32 {
    let (ar, ac) = (a as u32 / width, a as u32 % width);
    let (br, bc) = (b as u32 / width, b as u32 % width);
    let drow = ar.abs_diff(br);
    let dcol = ac.abs_diff(bc).min(width.saturating_sub(ac.abs_diff(bc)));
    drow.max(dcol)
}

pub fn build_amplification_model(
    controls: &ControlFields,
    identity: &[u8],
    variant: u32,
    level: DetailLevel,
    check_cancelled: &mut dyn FnMut() -> Result<(), AtlasError>,
) -> Result<AmplificationModel, AtlasError> {
    let finest = octave_factor(level, 2);
    let lattice_width = controls
        .grid
        .width
        .checked_mul(finest)
        .ok_or_else(|| AtlasError::limit("atlas lattice width overflowed"))?;
    let lattice_height = controls
        .grid
        .height
        .checked_mul(finest)
        .ok_or_else(|| AtlasError::limit("atlas lattice height overflowed"))?;
    let octaves = [
        build_octave(
            controls,
            identity,
            variant,
            octave_factor(level, 0),
            octave_weight_ppm(0),
            check_cancelled,
        )?,
        build_octave(
            controls,
            identity,
            variant,
            octave_factor(level, 1),
            octave_weight_ppm(1),
            check_cancelled,
        )?,
        build_octave(
            controls,
            identity,
            variant,
            octave_factor(level, 2),
            octave_weight_ppm(2),
            check_cancelled,
        )?,
    ];
    let count = (lattice_width as usize)
        .checked_mul(lattice_height as usize)
        .ok_or_else(|| AtlasError::limit("atlas lattice count overflowed"))?;
    let mut residual_mm = vec![0_i32; count];
    for j in 0..lattice_height {
        if j as usize % CANCELLATION_STRIDE == 0 {
            check_cancelled()?;
        }
        let polar = j == 0 || j + 1 == lattice_height;
        if polar {
            let mut sum = 0_i32;
            for (width, height, field) in &octaves {
                let octave_j = if j == 0 { 0 } else { *height - 1 };
                sum = sum.saturating_add(field[lattice_index(*width, 0, octave_j)]);
            }
            for i in 0..lattice_width {
                residual_mm[lattice_index(lattice_width, i, j)] = sum;
            }
            continue;
        }
        for i in 0..lattice_width {
            let lon = lattice_lon_micro(i, lattice_width);
            let lat = lattice_lat_micro(j, lattice_height);
            let mut sum = 0_i32;
            for (width, height, field) in &octaves {
                sum = sum.saturating_add(sample_octave(
                    *width,
                    *height,
                    field,
                    lon,
                    lat,
                    controls.grid.radius_metres,
                ));
            }
            residual_mm[lattice_index(lattice_width, i, j)] = sum;
        }
    }
    mean_remove(
        controls.grid,
        lattice_width,
        lattice_height,
        &mut residual_mm,
        check_cancelled,
    )?;
    let origin = mountain_window_origin(controls);
    let features = extract_mountain_features(
        controls,
        lattice_width,
        lattice_height,
        &residual_mm,
        origin.0,
        origin.1,
    );
    let orometry_key = domain_key(
        identity,
        ATLAS_DETAIL_ALGORITHM_EXPERIMENTAL_VERSION,
        variant,
        MOUNTAIN_OROMETRY_DOMAIN,
    );
    Ok(AmplificationModel {
        detail: AtlasDetailModel {
            grid: controls.grid,
            elevations_mm: controls.elevation_mm.clone(),
            residual_mm,
            lattice_width,
            lattice_height,
            algorithm_version: ATLAS_DETAIL_ALGORITHM_EXPERIMENTAL_VERSION,
            variant,
            level,
        },
        mountain_origin_row: origin.0,
        mountain_origin_col: origin.1,
        features,
        orometry_key,
    })
}

pub fn topology_fingerprint(model: &AmplificationModel) -> [u8; 32] {
    use sha2::{Digest, Sha256};
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&model.orometry_key);
    bytes.extend_from_slice(&model.mountain_origin_row.to_le_bytes());
    bytes.extend_from_slice(&model.mountain_origin_col.to_le_bytes());
    bytes.extend_from_slice(&(model.features.len() as u32).to_le_bytes());
    for feature in &model.features {
        bytes.extend_from_slice(feature.id.as_bytes());
        bytes.push(0);
        bytes.extend_from_slice(&(feature.kind as u8).to_le_bytes());
        bytes.extend_from_slice(&(feature.lattice_index as u32).to_le_bytes());
        bytes.extend_from_slice(&feature.lon_micro.to_le_bytes());
        bytes.extend_from_slice(&feature.lat_micro.to_le_bytes());
        bytes.extend_from_slice(&feature.elevation_mm.to_le_bytes());
    }
    Sha256::digest(bytes).into()
}

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
    use crate::style::{load_style, ANTIQUE_STYLE_ID, RELIEF_STYLE_ID};
    use crate::ATLAS_DETAIL_ALGORITHM_VERSION;
    use daena_physical::history::{derive_historical_world, HistoricalForcingParameters};
    use daena_physical::NoopProgress as PhysicalNoop;
    use std::time::Instant;

    fn hex(bytes: &[u8]) -> String {
        bytes.iter().map(|byte| format!("{byte:02x}")).collect()
    }

    fn controls_at(offset_years: i64) -> (ControlFields, Vec<u8>, i32, Vec<i32>) {
        let world = golden_world();
        let identity = spike_identity_from_source(&world.source);
        let historical = derive_historical_world(
            &world.field,
            world.report.reference_water_inventory_m3,
            Some(&world.tectonics.crust_by_cell),
            HistoricalForcingParameters::default_for(world.field.seed, world.field.retry_index),
            offset_years,
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
        let v2 = domain_key(
            b"identity-fixture",
            ATLAS_DETAIL_ALGORITHM_EXPERIMENTAL_VERSION,
            0,
            HIERARCHICAL_RELIEF_DOMAIN,
        );
        assert_eq!(
            hex(&v2),
            "1501efd4684e5f49c9983a78f7f167b9d87c673f0fb3379739545b19e9c3f9ff"
        );
        assert_ne!(v1, v2);
    }

    #[test]
    fn amplification_conserves_macro_sign_topology_and_drainage() {
        let world = golden_world();
        let (controls, identity, sea, sdf) = controls_at(0);
        let mut cancel = || Ok(());
        let started = Instant::now();
        let model =
            build_amplification_model(&controls, &identity, 0, DetailLevel::Standard, &mut cancel)
                .unwrap();
        assert!(
            started.elapsed().as_secs() < 5,
            "standard amplification exceeded the experimental budget"
        );
        assert!(
            model.detail.residual_mm.len() * 4 <= 2_000_000,
            "experimental residual exceeded the in-process byte budget"
        );
        assert_eq!(
            model.detail.algorithm_version,
            ATLAS_DETAIL_ALGORITHM_EXPERIMENTAL_VERSION
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

        let down =
            crate::detail::downsample_mean_mm(&model.detail, sea, &sdf, &mut cancel).unwrap();
        let canonical_components = land_components(controls.grid, &controls.elevation_mm, sea);
        let refined_components = land_components(controls.grid, &down, sea);
        assert!(interior_land_components_equivalent(
            &canonical_components,
            &refined_components,
            &sdf
        ));
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
                    world.field.elevations_mm[cell] >= sea,
                    down[cell] >= sea,
                    "river endpoint land sign changed at cell {cell}"
                );
                assert_eq!(
                    world.hydrology.watershed_id[cell],
                    u32::try_from(controls.watershed_id[cell]).unwrap_or(u32::MAX)
                );
            }
        }
        assert!(!model.features.is_empty());
        assert!(model.features.iter().any(|f| f.kind == MountainKind::Peak));
        assert!(model
            .features
            .iter()
            .any(|f| matches!(f.kind, MountainKind::Saddle | MountainKind::Ridge)));
        assert!(model.features.iter().all(|feature| {
            controls.sample_mountain_influence(feature.lon_micro, feature.lat_micro) > 0
        }));
        let mut cancel_octave = || Ok(());
        let (octave_w, octave_h, octave) = build_octave(
            &controls,
            &identity,
            0,
            octave_factor(DetailLevel::Standard, 2),
            octave_weight_ppm(2),
            &mut cancel_octave,
        )
        .unwrap();
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
            let Some((_, peak_elevation)) = model
                .features
                .iter()
                .filter(|feature| feature.kind == MountainKind::Peak)
                .map(|feature| {
                    (
                        chebyshev(
                            model.detail.lattice_width,
                            feature.lattice_index,
                            foothill.lattice_index,
                        ),
                        feature.elevation_mm,
                    )
                })
                .min_by_key(|(distance, elevation)| (*distance, -*elevation))
            else {
                panic!("foothill without a peak");
            };
            assert!(foothill.elevation_mm < peak_elevation);
        }
        assert_eq!(
            hex(&topology_fingerprint(&model)),
            "6e0cb3f3d5255894465f72c4ced6a65298fcf762d1179f0c5d07de86f0a8d348"
        );
        assert_eq!(
            (
                model.detail.residual_at(0, 0),
                model.detail.residual_at(12_345_678, -8_000_000)
            ),
            (-25, 4)
        );
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
        assert_ne!(relief.0.id, antique.0.id);

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
        for (lon, lat) in samples {
            assert_eq!(
                model.detail.residual_at(lon, lat),
                epoch_model.detail.residual_at(lon, lat)
            );
            assert_eq!(
                model.detail.residual_at(lon, lat),
                rebuilt.detail.residual_at(lon, lat)
            );
        }
        let _ = (sea, sdf, relief.0.ocean_deep, antique.0.ocean_deep);
        let rejected = AtlasRenderRequest {
            algorithm_version: ATLAS_DETAIL_ALGORITHM_EXPERIMENTAL_VERSION,
            ..AtlasRenderRequest::spike_png(64, 32).unwrap()
        }
        .normalize();
        assert!(rejected.is_err());
        assert_eq!(ATLAS_DETAIL_ALGORITHM_VERSION, 1);
    }
}
