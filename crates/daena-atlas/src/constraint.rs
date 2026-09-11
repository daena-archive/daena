//! Map-owned Atlas constraints. Authored records, not generator state.

use daena_physical::hydrology::HydrologyField;
use daena_physical::Grid;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::detail::{lattice_lat_micro, lattice_lon_micro};
use crate::erosion::lattice_index;
use crate::projection::{wrap_lon_micro, LON_MICRO_SPAN};
use crate::AtlasError;

pub const MAX_CONSTRAINTS: usize = 256;
pub const MAX_CONSTRAINT_PATH: usize = 2_048;
const NEAR_PATH_RADIUS2: i64 = 250_000_i64 * 250_000;
const LOCK_MAX_RADIUS2: i64 = 2_000_000_i64 * 2_000_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum AtlasConstraintKind {
    LockPath,
    ForceLake,
    FreezeErosion,
    PinShoreline,
}

impl AtlasConstraintKind {
    pub fn parse(value: &str) -> Result<Self, AtlasError> {
        match value {
            "lock-path" => Ok(Self::LockPath),
            "force-lake" => Ok(Self::ForceLake),
            "freeze-erosion" => Ok(Self::FreezeErosion),
            "pin-shoreline" => Ok(Self::PinShoreline),
            other => Err(AtlasError::invalid(format!(
                "unsupported atlas constraint kind: {other}"
            ))),
        }
    }

    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::LockPath => "lock-path",
            Self::ForceLake => "force-lake",
            Self::FreezeErosion => "freeze-erosion",
            Self::PinShoreline => "pin-shoreline",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AtlasConstraint {
    pub id: String,
    pub kind: AtlasConstraintKind,
    pub path: Vec<[i32; 2]>,
    #[serde(default)]
    pub closed: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub revision: Option<String>,
}

#[must_use]
pub fn fingerprint(constraints: &[AtlasConstraint]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update((constraints.len() as u32).to_le_bytes());
    for constraint in constraints {
        hasher.update((constraint.id.len() as u32).to_le_bytes());
        hasher.update(constraint.id.as_bytes());
        hasher.update(constraint.kind.as_str().as_bytes());
        hasher.update([u8::from(constraint.closed)]);
        let revision = constraint.revision.as_deref().unwrap_or("");
        hasher.update((revision.len() as u32).to_le_bytes());
        hasher.update(revision.as_bytes());
        hasher.update((constraint.path.len() as u32).to_le_bytes());
        for point in &constraint.path {
            hasher.update(point[0].to_le_bytes());
            hasher.update(point[1].to_le_bytes());
        }
    }
    hasher.finalize().into()
}

#[must_use]
pub fn contains(path: &[[i32; 2]], lon_micro: i32, lat_micro: i32) -> bool {
    if path.len() < 3 {
        return false;
    }
    let mut inside = false;
    let mut prev = path[path.len() - 1];
    for &point in path {
        let crosses = (prev[1] > lat_micro) != (point[1] > lat_micro);
        if crosses {
            let dy = i64::from(point[1]) - i64::from(prev[1]);
            if dy != 0 {
                let dlon = wrap_dlon(point[0], prev[0]);
                let at =
                    i64::from(prev[0]) + dlon * (i64::from(lat_micro) - i64::from(prev[1])) / dy;
                if wrap_dlon(wrap_lon_micro(at), lon_micro) > 0 {
                    inside = !inside;
                }
            }
        }
        prev = point;
    }
    inside
}

#[must_use]
pub fn near_path(path: &[[i32; 2]], lon_micro: i32, lat_micro: i32, radius2: i64) -> bool {
    if path.is_empty() {
        return false;
    }
    if path.len() == 1 {
        return distance2(path[0], [lon_micro, lat_micro]) <= radius2;
    }
    for window in path.windows(2) {
        if segment_distance2(window[0], window[1], [lon_micro, lat_micro]) <= radius2 {
            return true;
        }
    }
    false
}

pub fn apply_to_protected(
    constraints: &[AtlasConstraint],
    width: u32,
    height: u32,
    protected: &mut [bool],
) {
    if constraints.is_empty() {
        return;
    }
    for j in 0..height {
        for i in 0..width {
            let lon = lattice_lon_micro(i, width);
            let lat = lattice_lat_micro(j, height);
            let index = lattice_index(width, i, j);
            for constraint in constraints {
                match constraint.kind {
                    AtlasConstraintKind::FreezeErosion
                    | AtlasConstraintKind::ForceLake
                    | AtlasConstraintKind::LockPath => {
                        if constraint.closed {
                            if contains(&constraint.path, lon, lat) {
                                protected[index] = true;
                            }
                        } else if near_path(&constraint.path, lon, lat, NEAR_PATH_RADIUS2) {
                            protected[index] = true;
                        }
                    }
                    AtlasConstraintKind::PinShoreline => {}
                }
            }
        }
    }
}

#[must_use]
pub fn pins_shoreline(constraints: &[AtlasConstraint], lon_micro: i32, lat_micro: i32) -> bool {
    constraints.iter().any(|constraint| {
        constraint.kind == AtlasConstraintKind::PinShoreline
            && near_path(&constraint.path, lon_micro, lat_micro, NEAR_PATH_RADIUS2)
    })
}

pub fn apply_to_coastal(
    constraints: &[AtlasConstraint],
    width: u32,
    height: u32,
    source_mm: &[i32],
    coastal_mm: &mut [i32],
    protected: &mut [bool],
) {
    for constraint in constraints {
        if constraint.kind != AtlasConstraintKind::PinShoreline {
            continue;
        }
        for j in 0..height {
            for i in 0..width {
                let lon = lattice_lon_micro(i, width);
                let lat = lattice_lat_micro(j, height);
                if near_path(&constraint.path, lon, lat, NEAR_PATH_RADIUS2) {
                    let index = lattice_index(width, i, j);
                    coastal_mm[index] = source_mm[index];
                    protected[index] = true;
                }
            }
        }
    }
}

pub fn apply_lock_to_drainage(
    constraints: &[AtlasConstraint],
    tributaries: &mut [crate::drainage::DerivedTributary],
) {
    for constraint in constraints {
        if constraint.kind != AtlasConstraintKind::LockPath || constraint.path.len() < 2 {
            continue;
        }
        if let Some(index) = nearest_path(
            tributaries
                .iter()
                .map(|tributary| tributary.path.as_slice()),
            &constraint.path,
        ) {
            tributaries[index].path = constraint.path.clone();
        }
    }
}

pub fn apply_to_hydrology(constraints: &[AtlasConstraint], hydrology: &mut HydrologyField) {
    let grid = hydrology.grid;
    for constraint in constraints {
        match constraint.kind {
            AtlasConstraintKind::ForceLake => {
                force_lake(constraint, grid, hydrology);
            }
            AtlasConstraintKind::LockPath => {
                lock_path(constraint, hydrology);
            }
            AtlasConstraintKind::FreezeErosion | AtlasConstraintKind::PinShoreline => {}
        }
    }
}

fn force_lake(constraint: &AtlasConstraint, grid: Grid, hydrology: &mut HydrologyField) {
    let count = grid.sample_count();
    if hydrology.lake_cells.len() != count {
        return;
    }
    for cell in 0..count {
        let lon = crate::detail::cell_center_lon_micro((cell as u32) % grid.width, grid.width);
        let lat = crate::detail::cell_center_lat_micro((cell as u32) / grid.width, grid.height);
        let hit = if constraint.closed {
            contains(&constraint.path, lon, lat)
        } else {
            near_path(&constraint.path, lon, lat, NEAR_PATH_RADIUS2)
        };
        if hit {
            hydrology.lake_cells[cell] = true;
        }
    }
}

fn lock_path(constraint: &AtlasConstraint, hydrology: &mut HydrologyField) {
    if constraint.path.len() < 2 {
        return;
    }
    if let Some(index) = nearest_path(
        hydrology
            .river_coordinates
            .iter()
            .map(|path| path.as_slice()),
        &constraint.path,
    ) {
        hydrology.river_coordinates[index] = constraint.path.clone();
    } else {
        hydrology.river_coordinates.push(constraint.path.clone());
    }
}

fn nearest_path<'a, I>(paths: I, target: &[[i32; 2]]) -> Option<usize>
where
    I: IntoIterator<Item = &'a [[i32; 2]]>,
{
    let mut best = None;
    let mut best_d = LOCK_MAX_RADIUS2;
    for (index, path) in paths.into_iter().enumerate() {
        if path.is_empty() {
            continue;
        }
        let mut distance = i64::MAX;
        for &point in target {
            if path.len() == 1 {
                distance = distance.min(distance2(path[0], point));
            } else {
                for window in path.windows(2) {
                    distance = distance.min(segment_distance2(window[0], window[1], point));
                }
            }
        }
        if distance <= best_d {
            best_d = distance;
            best = Some(index);
        }
    }
    best
}

fn wrap_dlon(a: i32, b: i32) -> i64 {
    let mut delta = i64::from(a) - i64::from(b);
    if delta > LON_MICRO_SPAN / 2 {
        delta -= LON_MICRO_SPAN;
    } else if delta < -LON_MICRO_SPAN / 2 {
        delta += LON_MICRO_SPAN;
    }
    delta
}

fn distance2(a: [i32; 2], b: [i32; 2]) -> i64 {
    let dlon = wrap_dlon(a[0], b[0]);
    let dlat = i64::from(a[1]) - i64::from(b[1]);
    dlon.saturating_mul(dlon)
        .saturating_add(dlat.saturating_mul(dlat))
}

fn segment_distance2(a: [i32; 2], b: [i32; 2], p: [i32; 2]) -> i64 {
    let dx = wrap_dlon(b[0], a[0]);
    let dy = i64::from(b[1]) - i64::from(a[1]);
    let len2 = dx.saturating_mul(dx).saturating_add(dy.saturating_mul(dy));
    if len2 == 0 {
        return distance2(a, p);
    }
    let px = wrap_dlon(p[0], a[0]);
    let py = i64::from(p[1]) - i64::from(a[1]);
    let t = (px.saturating_mul(dx).saturating_add(py.saturating_mul(dy))).clamp(0, len2);
    let qx = dx.saturating_mul(t) / len2;
    let qy = dy.saturating_mul(t) / len2;
    let rx = px - qx;
    let ry = py - qy;
    rx.saturating_mul(rx).saturating_add(ry.saturating_mul(ry))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fingerprint_includes_revision() {
        let mut constraint = AtlasConstraint {
            id: "a".into(),
            kind: AtlasConstraintKind::FreezeErosion,
            path: vec![[0, 0], [1, 1]],
            closed: false,
            revision: None,
        };
        let without = fingerprint(&[constraint.clone()]);
        constraint.revision = Some("r2".into());
        assert_ne!(without, fingerprint(&[constraint]));
    }

    #[test]
    fn closed_square_contains_interior() {
        let path = [
            [0, 0],
            [10_000_000, 0],
            [10_000_000, 10_000_000],
            [0, 10_000_000],
        ];
        assert!(contains(&path, 5_000_000, 5_000_000));
        assert!(!contains(&path, 20_000_000, 5_000_000));
    }

    #[test]
    fn lock_path_does_not_replace_a_distant_tributary() {
        let original = vec![[0, 0], [1_000_000, 0]];
        let mut tributaries = [crate::drainage::DerivedTributary {
            id: "t".into(),
            source_cell: 0,
            join_cell: 1,
            parent_river_id: 0,
            ordinal: 0,
            watershed_id: 0,
            width_mm: 1,
            depth_mm: 1,
            path: original.clone(),
        }];
        apply_lock_to_drainage(
            &[AtlasConstraint {
                id: "lock".into(),
                kind: AtlasConstraintKind::LockPath,
                path: vec![[120_000_000, 40_000_000], [130_000_000, 41_000_000]],
                closed: false,
                revision: None,
            }],
            &mut tributaries,
        );
        assert_eq!(tributaries[0].path, original);
    }
}
