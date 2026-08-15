//! Detail algorithm 5: visible landform grain, divide-tree orometry in
//! hundreds of metres, coastline reconstruction, and slope-exaggerated relief.

use daena_physical::Grid;

use crate::control::{
    ControlFields, CLIMATE_CLASS_ARID, CLIMATE_CLASS_FOREST, CLIMATE_CLASS_GRASSLAND,
    CLIMATE_CLASS_ICE, CLIMATE_CLASS_TUNDRA,
};
use crate::detail::{
    domain_key, lattice_lat_micro, lattice_lon_micro, lattice_sample, nearest_cell, sample_field_mm,
    sample_sdf_ppm, AtlasDetailModel, COASTAL_ENVELOPE_PPM, MAX_RESIDUAL_MM,
};
use crate::erosion::{
    accumulate_flow, apply_scale_erosion, assign_simple_flow, lattice_index, lock_polar_rows,
    neighbor_at, HIERARCHICAL_EROSION_STEP_MM, HIERARCHICAL_SCALES,
};
use crate::projection::bilinear_i32;
use crate::request::DetailLevel;
use crate::{AtlasError, ATLAS_DETAIL_ALGORITHM_VERSION};

pub const HIERARCHICAL_RELIEF_DOMAIN: &str = "hierarchical-relief";
pub const MOUNTAIN_OROMETRY_DOMAIN: &str = "mountain-orometry";
pub const COASTLINE_SYNTHESIS_DOMAIN: &str = "coastline-synthesis";
pub const COASTAL_RAMP_MM: i32 = 72_000;
pub const COASTAL_DISPLACE_PPM: i32 = 380_000;
pub const MOUNTAIN_WINDOW_WIDTH: u32 = 16;
pub const MOUNTAIN_WINDOW_HEIGHT: u32 = 12;
pub const MAX_MOUNTAIN_FEATURES: usize = 512;
pub const MAX_PEAKS_PER_SYSTEM: usize = 24;
pub const RIDGE_SYNTHESIS_MM: i32 = 920_000;
pub const VALLEY_SYNTHESIS_MM: i32 = 680_000;
const CANCELLATION_STRIDE: usize = 4_096;

fn mountain_window_size(grid: Grid) -> (u32, u32) {
    (
        MOUNTAIN_WINDOW_WIDTH.min(grid.width),
        MOUNTAIN_WINDOW_HEIGHT.min(grid.height),
    )
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum MountainKind {
    Peak,
    Saddle,
    Ridge,
    SecondaryRidge,
    Valley,
    Foothill,
}

impl MountainKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Peak => "peak",
            Self::Saddle => "saddle",
            Self::Ridge => "ridge",
            Self::SecondaryRidge => "secondary-ridge",
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
    pub mountain_system_count: u32,
    pub features: Vec<MountainFeature>,
    pub orometry_key: [u8; 32],
}

fn signed_unit_mm(sample: u64, amplitude_mm: i32) -> i32 {
    let unit = (sample >> 11) as i64 % 2_000_001 - 1_000_000;
    ((unit * i64::from(amplitude_mm)) / 1_000_000) as i32
}

fn landform_amplitude_mm(controls: &ControlFields, lon_micro: i32, lat_micro: i32) -> i32 {
    let elevation = controls.sample_elevation(lon_micro, lat_micro);
    let crust = controls.sample_crust_influence(lon_micro, lat_micro).clamp(0, 1_000_000);
    let mountain = controls
        .sample_mountain_influence(lon_micro, lat_micro)
        .clamp(0, 1_000_000);
    let ice = controls.sample_ice_thickness(lon_micro, lat_micro).max(0);
    let runoff = controls.sample_runoff(lon_micro, lat_micro).clamp(0, 4_000);
    let precip = controls.sample_precipitation(lon_micro, lat_micro).clamp(0, 4_000);
    let lake = controls.sample_lake_mask(lon_micro, lat_micro);
    let climate = controls.sample_climate_class(lon_micro, lat_micro);
    let sea = controls.sea_level_mm;
    if lake > 0 {
        return 16;
    }
    let magnitude = elevation.unsigned_abs().min(8_000_000);
    let scaled = (u64::from(magnitude) * 180_000 / 8_000_000) as i32;
    let mut amplitude = scaled.clamp(48_000, 180_000);
    amplitude = ((i64::from(amplitude) * i64::from(crust.max(220_000))) / 1_000_000) as i32;
    amplitude = amplitude.saturating_add(((i64::from(mountain) * 540_000) / 1_000_000) as i32);
    if elevation < sea {
        amplitude = amplitude.min(180);
    }
    if ice > 800 {
        amplitude = ((i64::from(amplitude) * 220_000) / 1_000_000) as i32;
    }
    amplitude = match climate {
        CLIMATE_CLASS_ICE => ((i64::from(amplitude) * 180_000) / 1_000_000) as i32,
        CLIMATE_CLASS_TUNDRA => ((i64::from(amplitude) * 720_000) / 1_000_000) as i32,
        CLIMATE_CLASS_ARID => ((i64::from(amplitude) * 1_180_000) / 1_000_000) as i32,
        CLIMATE_CLASS_GRASSLAND => amplitude,
        CLIMATE_CLASS_FOREST => ((i64::from(amplitude) * 900_000) / 1_000_000) as i32,
        _ => amplitude,
    };
    if runoff > 400 && mountain < 200_000 {
        amplitude = ((i64::from(amplitude) * 860_000) / 1_000_000) as i32;
    }
    if precip < 200 && climate == CLIMATE_CLASS_ARID {
        amplitude = amplitude.saturating_add(28_000);
    }
    if elevation < sea {
        return amplitude.clamp(48, 180);
    }
    amplitude.clamp(36_000, MAX_RESIDUAL_MM)
}

fn control_shaped_unit(
    controls: &ControlFields,
    lon_micro: i32,
    lat_micro: i32,
    sample: u64,
    amplitude_mm: i32,
) -> i32 {
    let unit = signed_unit_mm(sample, amplitude_mm);
    let mountain = controls
        .sample_mountain_influence(lon_micro, lat_micro)
        .clamp(0, 1_000_000);
    let runoff = controls.sample_runoff(lon_micro, lat_micro).clamp(0, 4_000);
    let ridged = if unit < 0 { -unit / 2 } else { unit };
    let valley = if unit > 0 { unit / 2 } else { unit };
    let mountain_mix =
        (i64::from(unit) * i64::from(1_000_000 - mountain) + i64::from(ridged) * i64::from(mountain))
            / 1_000_000;
    let runoff_ppm = (i64::from(runoff) * 1_000_000 / 4_000).clamp(0, 700_000);
    ((mountain_mix * (1_000_000 - runoff_ppm) + i64::from(valley) * runoff_ppm) / 1_000_000) as i32
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
        ATLAS_DETAIL_ALGORITHM_VERSION,
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
            let sample = lattice_sample(&key, sample_i, sample_j, factor);
            let amplitude = landform_amplitude_mm(controls, lon, lat);
            let unit = control_shaped_unit(controls, lon, lat, sample, amplitude);
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

fn mean_remove_outside_envelope(
    grid: Grid,
    sdf: &[i32],
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
            let sdf_ppm = sample_sdf_ppm(grid, sdf, lon, lat);
            if sdf_ppm.unsigned_abs() <= COASTAL_ENVELOPE_PPM {
                continue;
            }
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
            let sdf_ppm = sample_sdf_ppm(grid, sdf, lon, lat);
            if sdf_ppm.unsigned_abs() <= COASTAL_ENVELOPE_PPM {
                continue;
            }
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
    let (window_w, window_h) = mountain_window_size(controls.grid);
    let max_row = controls.grid.height.saturating_sub(window_h);
    let max_col = controls.grid.width.saturating_sub(window_w);
    for row in 0..=max_row {
        for col in 0..=max_col {
            let mut sum = 0_i64;
            for dr in 0..window_h {
                for dc in 0..window_w {
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

fn feature_id(kind: MountainKind, index: usize) -> String {
    format!(
        "atlas:orometry:v{}:{}:{index}",
        ATLAS_DETAIL_ALGORITHM_VERSION,
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

fn mountain_system_labels(controls: &ControlFields) -> (Vec<i32>, u32, (u32, u32)) {
    let count = controls.grid.sample_count();
    let mut labels = vec![-1_i32; count];
    let mut next = 0_i32;
    let mut best_size = 0_u32;
    let mut best_origin = (0, 0);
    for cell in 0..count {
        if labels[cell] >= 0 || controls.mountain_influence_ppm[cell] <= 0 {
            continue;
        }
        let mut stack = vec![cell];
        labels[cell] = next;
        let mut size = 1_u32;
        let (mut origin_row, mut origin_col) = controls.grid.row_col(cell);
        while let Some(current) = stack.pop() {
            let (row, col) = controls.grid.row_col(current);
            if (row, col) < (origin_row, origin_col) {
                origin_row = row;
                origin_col = col;
            }
            for neighbor in controls.grid.neighbors(current) {
                if labels[neighbor] < 0 && controls.mountain_influence_ppm[neighbor] > 0 {
                    labels[neighbor] = next;
                    size += 1;
                    stack.push(neighbor);
                }
            }
        }
        if size > best_size || (size == best_size && (origin_row, origin_col) < best_origin) {
            best_size = size;
            best_origin = (origin_row, origin_col);
        }
        next += 1;
    }
    (labels, next.max(0) as u32, best_origin)
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
    features.push(MountainFeature {
        id: feature_id(kind, index),
        kind,
        lattice_index: index,
        lon_micro: lon,
        lat_micro: lat,
        elevation_mm: elevation,
    });
}

fn synthesize_orometry(
    width: u32,
    height: u32,
    residual: &mut [i32],
    features: &[MountainFeature],
    mountain: &[bool],
) {
    let count = residual.len();
    let mut d_ridge = vec![u32::MAX; count];
    let mut d_valley = vec![u32::MAX; count];
    let mut ridge_queue = std::collections::VecDeque::new();
    let mut valley_queue = std::collections::VecDeque::new();
    for feature in features {
        if feature.lattice_index >= count {
            continue;
        }
        match feature.kind {
            MountainKind::Peak | MountainKind::Ridge | MountainKind::Saddle => {
                d_ridge[feature.lattice_index] = 0;
                ridge_queue.push_back(feature.lattice_index);
            }
            MountainKind::SecondaryRidge => {
                d_ridge[feature.lattice_index] = d_ridge[feature.lattice_index].min(1);
                ridge_queue.push_back(feature.lattice_index);
            }
            MountainKind::Valley => {
                d_valley[feature.lattice_index] = 0;
                valley_queue.push_back(feature.lattice_index);
            }
            MountainKind::Foothill => {}
        }
    }
    let flood = |queue: &mut std::collections::VecDeque<usize>, dist: &mut [u32]| {
        while let Some(index) = queue.pop_front() {
            let j = index as u32 / width;
            let i = index as u32 % width;
            for dir in crate::erosion::DIRS {
                let Some((_, _, neighbor)) = neighbor_at(width, height, i, j, dir) else {
                    continue;
                };
                if !mountain[neighbor] {
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
    flood(&mut ridge_queue, &mut d_ridge);
    flood(&mut valley_queue, &mut d_valley);
    for index in 0..count {
        if !mountain[index] {
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
            1_000_000 / (1 + dr.min(24))
        };
        let valley_w = if dv == u32::MAX {
            0
        } else {
            1_000_000 / (1 + dv.min(24))
        };
        let total = ridge_w + valley_w;
        if total == 0 {
            continue;
        }
        let delta = (i64::from(RIDGE_SYNTHESIS_MM) * i64::from(ridge_w)
            - i64::from(VALLEY_SYNTHESIS_MM) * i64::from(valley_w))
            / i64::from(total);
        residual[index] = residual[index].saturating_add(delta as i32);
    }
    for feature in features {
        if feature.kind == MountainKind::Foothill && feature.lattice_index < count {
            residual[feature.lattice_index] =
                residual[feature.lattice_index].saturating_add(RIDGE_SYNTHESIS_MM / 5);
        }
    }
}

fn extract_mountain_features(
    controls: &ControlFields,
    lattice_width: u32,
    lattice_height: u32,
    residual: &[i32],
    system_labels: &[i32],
) -> Vec<MountainFeature> {
    let mut window = Vec::new();
    for j in 0..lattice_height {
        for i in 0..lattice_width {
            let lon = lattice_lon_micro(i, lattice_width);
            let lat = lattice_lat_micro(j, lattice_height);
            if controls.sample_mountain_influence(lon, lat) <= 0 {
                continue;
            }
            let elevation =
                controls.sample_elevation(lon, lat) + residual[lattice_index(lattice_width, i, j)];
            let system = system_labels[nearest_cell(controls.grid, lon, lat)];
            window.push((elevation, i, j, lon, lat, system));
        }
    }
    window.sort_by(|a, b| {
        b.0.cmp(&a.0)
            .then_with(|| a.1.cmp(&b.1))
            .then_with(|| a.2.cmp(&b.2))
    });
    let mut index_of = vec![usize::MAX; lattice_width as usize * lattice_height as usize];
    for (slot, &(_, i, j, _, _, _)) in window.iter().enumerate() {
        index_of[lattice_index(lattice_width, i, j)] = slot;
    }
    let mut uf = UnionFind::new(window.len());
    let mut seen = vec![false; window.len()];
    let mut peak_of = vec![None; window.len()];
    let mut features = Vec::new();
    let mut peaks_in_system = vec![0_u32; system_labels.len().max(1)];
    for (slot, &(elevation, i, j, lon, lat, system)) in window.iter().enumerate() {
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
        let system_slot = if system < 0 { usize::MAX } else { system as usize };
        let under_system_cap = system_slot == usize::MAX
            || peaks_in_system
                .get(system_slot)
                .copied()
                .unwrap_or(MAX_PEAKS_PER_SYSTEM as u32)
                < MAX_PEAKS_PER_SYSTEM as u32;
        if !higher_neighbor && features.len() < MAX_MOUNTAIN_FEATURES && under_system_cap {
            if j == 0 || j + 1 == lattice_height {
                // Polar rows stay longitude-constant; they cannot host peaks.
            } else {
            peak_of[slot] = Some(features.len());
            if system_slot < peaks_in_system.len() {
                peaks_in_system[system_slot] += 1;
            }
            features.push(MountainFeature {
                id: feature_id(MountainKind::Peak, lattice_index(lattice_width, i, j)),
                kind: MountainKind::Peak,
                lattice_index: lattice_index(lattice_width, i, j),
                lon_micro: lon,
                lat_micro: lat,
                elevation_mm: elevation,
            });
            }
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
                push_feature(
                    &mut features,
                    MountainKind::Saddle,
                    lattice_width,
                    i,
                    j,
                    lon,
                    lat,
                    elevation,
                );
                push_feature(
                    &mut features,
                    MountainKind::Ridge,
                    lattice_width,
                    i,
                    j,
                    lon,
                    lat,
                    elevation,
                );
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
            if nearest.map(|(d, _, _)| candidate < (d, usize::MAX, usize::MAX)).unwrap_or(true) {
                second = nearest;
                nearest = Some(candidate);
            } else if second
                .map(|(d, idx, _)| (dist, peak_b.lattice_index) < (d, idx))
                .unwrap_or(true)
            {
                second = Some(candidate);
            }
        }
        if let Some((dist, index, _)) = second {
            if dist > 1 && dist <= 8 {
                let j = index as u32 / lattice_width;
                let i = index as u32 % lattice_width;
                let mid = lattice_path(
                    lattice_width,
                    lattice_height,
                    peak_a.lattice_index,
                    index,
                );
                if let Some(&saddle) = mid.get(mid.len() / 2) {
                    let sj = saddle as u32 / lattice_width;
                    let si = saddle as u32 % lattice_width;
                    let lon = lattice_lon_micro(si, lattice_width);
                    let lat = lattice_lat_micro(sj, lattice_height);
                    let elevation = controls.sample_elevation(lon, lat)
                        + residual[saddle];
                    push_feature(
                        &mut features,
                        MountainKind::SecondaryRidge,
                        lattice_width,
                        si,
                        sj,
                        lon,
                        lat,
                        elevation,
                    );
                }
                let _ = (i, j);
            }
        }
    }
    for j in 1..lattice_height.saturating_sub(1) {
        for i in 0..lattice_width {
            let lon = lattice_lon_micro(i, lattice_width);
            let lat = lattice_lat_micro(j, lattice_height);
            if controls.sample_mountain_influence(lon, lat) <= 0 {
                continue;
            }
            let elevation =
                controls.sample_elevation(lon, lat) + residual[lattice_index(lattice_width, i, j)];
            if elevation < controls.sea_level_mm {
                continue;
            }
            let mut local_min = true;
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
            let near_ridge = features.iter().any(|feature| {
                matches!(
                    feature.kind,
                    MountainKind::Ridge | MountainKind::Peak | MountainKind::SecondaryRidge
                ) && chebyshev(lattice_width, feature.lattice_index, index) <= 3
            });
            if local_min {
                push_feature(
                    &mut features,
                    MountainKind::Valley,
                    lattice_width,
                    i,
                    j,
                    lon,
                    lat,
                    elevation,
                );
            } else if near_ridge
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
                push_feature(
                    &mut features,
                    MountainKind::Foothill,
                    lattice_width,
                    i,
                    j,
                    lon,
                    lat,
                    elevation,
                );
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
    for j in 0..dst_height {
        if j as usize % CANCELLATION_STRIDE == 0 {
            check_cancelled()?;
        }
        let polar = j == 0 || j + 1 == dst_height;
        if polar {
            let src_j = if j == 0 { 0 } else { src_height - 1 };
            let value = src[lattice_index(src_width, 0, src_j)];
            for i in 0..dst_width {
                dst[lattice_index(dst_width, i, j)] = value;
            }
            continue;
        }
        for i in 0..dst_width {
            dst[lattice_index(dst_width, i, j)] = sample_octave(
                src_width,
                src_height,
                src,
                lattice_lon_micro(i, dst_width),
                lattice_lat_micro(j, dst_height),
                radius_metres,
            );
        }
    }
    Ok(dst)
}

fn land_mask_ppm(controls: &ControlFields) -> Vec<i32> {
    controls
        .elevation_mm
        .iter()
        .map(|elevation| {
            if *elevation >= controls.sea_level_mm {
                1_000_000
            } else {
                0
            }
        })
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

fn coastline_noise_ppm(key: &[u8; 32], i: u32, j: u32, width: u32, height: u32) -> i32 {
    let coarse = octave_noise_ppm(key, i, j, width, height, 0, 8);
    let mid = octave_noise_ppm(key, i, j, width, height, 1, 4);
    let fine = octave_noise_ppm(key, i, j, width, height, 2, 2);
    ((i64::from(coarse) * 520_000 + i64::from(mid) * 300_000 + i64::from(fine) * 180_000)
        / 1_000_000) as i32
}

fn synthesize_coastline(
    controls: &ControlFields,
    sdf: &[i32],
    identity: &[u8],
    variant: u32,
    width: u32,
    height: u32,
    residual: &mut [i32],
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
    for j in 0..height {
        if j as usize % CANCELLATION_STRIDE == 0 {
            check_cancelled()?;
        }
        let polar = j == 0 || j + 1 == height;
        for i in 0..width {
            let index = lattice_index(width, i, j);
            let lon = lattice_lon_micro(if polar { 0 } else { i }, width);
            let lat = lattice_lat_micro(j, height);
            if controls.sample_lake_mask(lon, lat) > 0 {
                continue;
            }
            let sdf_ppm = sample_sdf_ppm(controls.grid, sdf, lon, lat);
            if sdf_ppm.unsigned_abs() > COASTAL_ENVELOPE_PPM {
                continue;
            }
            let fraction = sample_field_mm(controls.grid, &land_ppm, lon, lat);
            let proximity = 1_000_000_u32.saturating_sub(
                fraction
                    .saturating_sub(500_000)
                    .unsigned_abs()
                    .saturating_mul(2),
            );
            if proximity == 0 {
                continue;
            }
            let noise = coastline_noise_ppm(&key, if polar { 0 } else { i }, j, width, height);
            let displaced = fraction.saturating_add(
                ((i64::from(noise) * i64::from(COASTAL_DISPLACE_PPM)) / 1_000_000) as i32,
            );
            let ramp = ((i64::from(displaced.saturating_sub(500_000)) * i64::from(COASTAL_RAMP_MM))
                / 500_000) as i32;
            let target = sea.saturating_add(ramp);
            let canonical = controls.sample_elevation(lon, lat);
            let current = canonical.saturating_add(residual[index]);
            let blended = ((i64::from(current) * i64::from(1_000_000 - proximity as i32)
                + i64::from(target) * i64::from(proximity as i32))
                / 1_000_000) as i32;
            let want_land = displaced >= 500_000;
            let signed = if want_land != (blended >= sea) {
                if want_land {
                    blended.max(sea.saturating_add(1)).max(target)
                } else {
                    blended.min(sea.saturating_sub(1)).min(target)
                }
            } else {
                blended
            };
            residual[index] = signed.saturating_sub(canonical);
        }
    }
    lock_polar_rows(width, height, residual);
    Ok(())
}

fn erode_residual_octave(
    controls: &ControlFields,
    sdf: &[i32],
    identity: &[u8],
    variant: u32,
    width: u32,
    height: u32,
    residual: &mut [i32],
    check_cancelled: &mut dyn FnMut() -> Result<(), AtlasError>,
) -> Result<(), AtlasError> {
    let count = residual.len();
    let mut surface = vec![0_i32; count];
    let mut protected = vec![false; count];
    let mut mountain_ppm = vec![0_i32; count];
    let mut runoff_ppm = vec![0_i32; count];
    let mut watershed = vec![-1_i32; count];
    for j in 0..height {
        if j as usize % CANCELLATION_STRIDE == 0 {
            check_cancelled()?;
        }
        for i in 0..width {
            let index = lattice_index(width, i, j);
            let lon = lattice_lon_micro(i, width);
            let lat = lattice_lat_micro(j, height);
            surface[index] = controls.sample_elevation(lon, lat) + residual[index];
            protected[index] = controls.sample_lake_mask(lon, lat) > 0
                && controls.sample_mountain_influence(lon, lat) <= 0;
            mountain_ppm[index] = controls.sample_mountain_influence(lon, lat);
            runoff_ppm[index] = (i64::from(controls.sample_runoff(lon, lat).clamp(0, 4_000))
                * 1_000_000
                / 4_000) as i32;
            watershed[index] = controls.sample_watershed_id(lon, lat);
        }
    }
    lock_polar_rows(width, height, &mut surface);
    let (primary, secondary, weight) = assign_simple_flow(
        width,
        height,
        &surface,
        &watershed,
        controls.sea_level_mm,
        check_cancelled,
    )?;
    let accumulation = accumulate_flow(
        &surface,
        &primary,
        &secondary,
        &weight,
        controls.sea_level_mm,
        check_cancelled,
    )?;
    let erosion_key = domain_key(
        identity,
        ATLAS_DETAIL_ALGORITHM_VERSION,
        variant,
        crate::erosion::MULTI_SCALE_EROSION_DOMAIN,
    );
    let filled = surface.clone();
    let land_at = |lon: i32, lat: i32| controls.sample_elevation(lon, lat) >= controls.sea_level_mm;
    apply_scale_erosion(
        crate::erosion::ScaleErosion {
            grid: controls.grid,
            width,
            height,
            sea_level_mm: controls.sea_level_mm,
            sdf,
            protected: &protected,
            mountain_ppm: &mountain_ppm,
            runoff_ppm: &runoff_ppm,
            primary: &primary,
            secondary: &secondary,
            weight: &weight,
            accumulation: &accumulation,
            erosion_key: &erosion_key,
            peaks: &[],
            filled_mm: &filled,
            land_at: &land_at,
            scales: &HIERARCHICAL_SCALES,
            max_step_mm: HIERARCHICAL_EROSION_STEP_MM,
        },
        &mut surface,
        check_cancelled,
    )?;
    for j in 0..height {
        for i in 0..width {
            let index = lattice_index(width, i, j);
            let lon = lattice_lon_micro(i, width);
            let lat = lattice_lat_micro(j, height);
            residual[index] = surface[index] - controls.sample_elevation(lon, lat);
        }
    }
    Ok(())
}

pub fn build_amplification_model(
    controls: &ControlFields,
    identity: &[u8],
    variant: u32,
    level: DetailLevel,
    check_cancelled: &mut dyn FnMut() -> Result<(), AtlasError>,
) -> Result<AmplificationModel, AtlasError> {
    let sdf = crate::detail::signed_coastal_distance_ppm(
        controls.grid,
        &controls.elevation_mm,
        controls.sea_level_mm,
    );
    let (mut width, mut height, mut residual_mm) = build_octave(
        controls,
        identity,
        variant,
        octave_factor(level, 0),
        octave_weight_ppm(0),
        check_cancelled,
    )?;
    erode_residual_octave(
        controls,
        &sdf,
        identity,
        variant,
        width,
        height,
        &mut residual_mm,
        check_cancelled,
    )?;
    for octave in 1..=2 {
        let factor = octave_factor(level, octave);
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
        let (_, _, detail) = build_octave(
            controls,
            identity,
            variant,
            factor,
            octave_weight_ppm(octave),
            check_cancelled,
        )?;
        for (index, value) in residual_mm.iter_mut().zip(detail.iter()) {
            *index = index.saturating_add(*value);
        }
        width = next_width;
        height = next_height;
        if octave < 2 {
            erode_residual_octave(
                controls,
                &sdf,
                identity,
                variant,
                width,
                height,
                &mut residual_mm,
                check_cancelled,
            )?;
        }
    }
    mean_remove(
        controls.grid,
        width,
        height,
        &mut residual_mm,
        check_cancelled,
    )?;
    let (system_labels, system_count, origin) = mountain_system_labels(controls);
    let features = extract_mountain_features(
        controls,
        width,
        height,
        &residual_mm,
        &system_labels,
    );
    let mut mountain = vec![false; residual_mm.len()];
    for j in 0..height {
        for i in 0..width {
            let lon = lattice_lon_micro(i, width);
            let lat = lattice_lat_micro(j, height);
            mountain[lattice_index(width, i, j)] = controls.sample_mountain_influence(lon, lat) > 0;
        }
    }
    synthesize_orometry(width, height, &mut residual_mm, &features, &mountain);
    mean_remove(
        controls.grid,
        width,
        height,
        &mut residual_mm,
        check_cancelled,
    )?;
    synthesize_coastline(
        controls,
        &sdf,
        identity,
        variant,
        width,
        height,
        &mut residual_mm,
        check_cancelled,
    )?;
    mean_remove_outside_envelope(
        controls.grid,
        &sdf,
        width,
        height,
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
        mountain_origin_row: origin.0,
        mountain_origin_col: origin.1,
        mountain_system_count: system_count,
        features,
        orometry_key,
    })
}

impl AmplificationModel {
    pub fn from_cached_detail(
        detail: AtlasDetailModel,
        controls: &ControlFields,
        identity: &[u8],
    ) -> Self {
        let (system_labels, system_count, origin) = mountain_system_labels(controls);
        let features = extract_mountain_features(
            controls,
            detail.lattice_width,
            detail.lattice_height,
            &detail.residual_mm,
            &system_labels,
        );
        let orometry_key = domain_key(
            identity,
            detail.algorithm_version,
            detail.variant,
            MOUNTAIN_OROMETRY_DOMAIN,
        );
        Self {
            detail,
            mountain_origin_row: origin.0,
            mountain_origin_col: origin.1,
            mountain_system_count: system_count,
            features,
            orometry_key,
        }
    }
}

pub fn topology_fingerprint(model: &AmplificationModel) -> [u8; 32] {
    use sha2::{Digest, Sha256};
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&model.orometry_key);
    bytes.extend_from_slice(&model.mountain_origin_row.to_le_bytes());
    bytes.extend_from_slice(&model.mountain_origin_col.to_le_bytes());
    bytes.extend_from_slice(&model.mountain_system_count.to_le_bytes());
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
            ATLAS_DETAIL_ALGORITHM_VERSION,
            0,
            HIERARCHICAL_RELIEF_DOMAIN,
        );
        assert_eq!(
            hex(&v2),
            "067d6bee8d7e4cbc3257618473a18027b74bb3221bf8d8fc0546a606fbe03f85"
        );
        assert_ne!(v1, v2);
    }

    #[test]
    fn mountain_window_clamps_to_grids_smaller_than_nominal_window() {
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
        assert_eq!(mountain_window_origin(&controls), (0, 0));
        let identity = spike_identity_from_source(&world.source);
        let mut cancel = || Ok(());
        build_amplification_model(&controls, &identity, 0, DetailLevel::Standard, &mut cancel)
            .unwrap();
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
        assert!(model.mountain_system_count >= 1);
        assert!(model.features.iter().any(|f| f.kind == MountainKind::Peak));
        assert!(model
            .features
            .iter()
            .any(|f| matches!(f.kind, MountainKind::Saddle | MountainKind::Ridge)));
        let mut flipped = 0_u32;
        let mut coastal = 0_u32;
        for j in 0..model.detail.lattice_height {
            for i in 0..model.detail.lattice_width {
                let lon = lattice_lon_micro(i, model.detail.lattice_width);
                let lat = lattice_lat_micro(j, model.detail.lattice_height);
                let sdf_ppm = sample_sdf_ppm(model.detail.grid, &sdf, lon, lat);
                if sdf_ppm.unsigned_abs() > COASTAL_ENVELOPE_PPM {
                    continue;
                }
                coastal += 1;
                let canonical = model.detail.canonical_at(lon, lat);
                let refined = model.detail.refined_at(lon, lat, sea, sdf_ppm);
                if (canonical >= sea) != (refined >= sea) {
                    flipped += 1;
                }
            }
        }
        assert!(coastal > 0);
        assert!(
            flipped >= 16,
            "coastline synthesis did not reconstruct the shore ({flipped} sign flips)"
        );
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
            "65ca2a0bf35762e6e840b36f9c5bf8ed014a6194b388be75988b17b08bd42b2d"
        );
        assert_eq!(
            (
                model.detail.residual_at(0, 0),
                model.detail.residual_at(12_345_678, -8_000_000)
            ),
            (-17, 7)
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
        let rejected = AtlasRenderRequest {
            algorithm_version: 1,
            ..AtlasRenderRequest::spike_png(64, 32).unwrap()
        }
        .normalize();
        assert!(rejected.is_err());
        assert_eq!(ATLAS_DETAIL_ALGORITHM_VERSION, 5);
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
