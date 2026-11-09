//! Equiangular cube-sphere grid.
//!
//! Faces `0..6` are `+X, −X, +Y, −Y, +Z, −Z`. Each face has a fixed
//! right-handed basis `(normal, u, v)` with `u × v = normal` (see [`FACE_BASIS`]).
//! Cell id is `face * n * n + j * n + i`.
//!
//! Construction is cached per `n` (unit-sphere tables). Distances and areas
//! scale by the instance radius. [`CubeSphere::position`] returns a unit vector.

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex, OnceLock};

use rayon::prelude::*;

/// Missing neighbour.
pub const NONE: u32 = u32::MAX;

/// `(normal, u, v)` for faces `+X, −X, +Y, −Y, +Z, −Z`.
pub const FACE_BASIS: [[[f64; 3]; 3]; 6] = [
    [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
    [[-1.0, 0.0, 0.0], [0.0, 0.0, 1.0], [0.0, 1.0, 0.0]],
    [[0.0, 1.0, 0.0], [0.0, 0.0, 1.0], [1.0, 0.0, 0.0]],
    [[0.0, -1.0, 0.0], [1.0, 0.0, 0.0], [0.0, 0.0, 1.0]],
    [[0.0, 0.0, 1.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
    [[0.0, 0.0, -1.0], [0.0, 1.0, 0.0], [1.0, 0.0, 0.0]],
];

/// Neighbour steps in face coordinates: E, NE, N, NW, W, SW, S, SE.
pub const NEIGHBOR_DI: [i32; 8] = [1, 1, 0, -1, -1, -1, 0, 1];
/// Neighbour steps in face coordinates: E, NE, N, NW, W, SW, S, SE.
pub const NEIGHBOR_DJ: [i32; 8] = [0, 1, 1, 1, 0, -1, -1, -1];

const AXIS_NEIGHBORS: [usize; 4] = [0, 2, 4, 6];

struct GridTables {
    /// Steradians, one face, index `j * n + i`.
    area_sr: Vec<f64>,
    /// Radians. Index `k * n * n + j * n + i`, valid for interior cells.
    dist_rad: Vec<f32>,
    /// Global neighbour ids. Length `6 * (4n − 4)`.
    border_nb: Vec<[u32; 8]>,
    /// Radians. `0` where the neighbour is [`NONE`].
    border_dist: Vec<[f32; 8]>,
}

fn cache() -> &'static Mutex<BTreeMap<u32, Arc<GridTables>>> {
    static CACHE: OnceLock<Mutex<BTreeMap<u32, Arc<GridTables>>>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(BTreeMap::new()))
}

fn tables_for(n: u32) -> Arc<GridTables> {
    let map = cache();
    let mut guard = map.lock().unwrap_or_else(|e| e.into_inner());
    if let Some(existing) = guard.get(&n) {
        return Arc::clone(existing);
    }
    let built = Arc::new(build_tables(n));
    guard.insert(n, Arc::clone(&built));
    built
}

/// Cube-sphere cell complex of resolution `n` (cells along one face edge).
#[derive(Clone)]
pub struct CubeSphere {
    /// Cells along one edge of a face. Power of two in `8..=2048`.
    pub n: u32,
    /// Sphere radius in metres.
    pub radius_m: f64,
    nn: u32,
    tables: Arc<GridTables>,
}

impl std::fmt::Debug for CubeSphere {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CubeSphere")
            .field("n", &self.n)
            .field("radius_m", &self.radius_m)
            .finish()
    }
}

impl CubeSphere {
    /// Build or reuse the grid for `n`. `n` must be a power of two in `8..=2048`.
    /// `radius_m` must be finite and positive.
    pub fn new(n: u32, radius_m: f64) -> Result<Self, String> {
        if !n.is_power_of_two() || !(8..=2048).contains(&n) {
            return Err(format!(
                "grid n must be a power of two in [8, 2048], got {n}"
            ));
        }
        if !radius_m.is_finite() || radius_m <= 0.0 {
            return Err(format!(
                "radius_m must be finite and positive, got {radius_m}"
            ));
        }
        Ok(Self {
            n,
            radius_m,
            nn: n * n,
            tables: tables_for(n),
        })
    }

    /// Number of cells on the sphere (`6 n²`).
    pub fn cell_count(&self) -> usize {
        6 * self.nn as usize
    }

    /// Nominal cell width `π/2 · R / n`, in metres.
    pub fn cell_size_m(&self) -> f64 {
        std::f64::consts::FRAC_PI_2 * self.radius_m / self.n as f64
    }

    /// Cell id from face coordinates.
    #[inline]
    pub fn id(&self, face: u32, i: u32, j: u32) -> u32 {
        face * self.nn + j * self.n + i
    }

    /// `(face, i, j)` of a cell id.
    #[inline]
    pub fn decode(&self, cell: u32) -> (u32, u32, u32) {
        let face = cell / self.nn;
        let rem = cell % self.nn;
        let j = rem / self.n;
        let i = rem % self.n;
        (face, i, j)
    }

    /// Unit vector of the cell centre.
    pub fn position(&self, cell: u32) -> [f64; 3] {
        let (face, i, j) = self.decode(cell);
        unit_on_face(face, self.n, i, j)
    }

    /// `(latitude, longitude)` in radians. Latitude is `asin(z)`, longitude `atan2(y, x)`.
    pub fn lat_lon(&self, cell: u32) -> (f64, f64) {
        let p = self.position(cell);
        let lat = p[2].clamp(-1.0, 1.0).asin();
        let lon = p[1].atan2(p[0]);
        (lat, lon)
    }

    /// Cell containing the direction `p` (any non-zero scale).
    pub fn cell_at(&self, p: [f64; 3]) -> u32 {
        cell_at_n(self.n, p)
    }

    /// Face and continuous coordinates where cell centres sit at integer + 0.5.
    pub fn face_coords(&self, p: [f64; 3]) -> (u32, f64, f64) {
        let face = face_of(p);
        let pn = dot(p, FACE_BASIS[face as usize][0]);
        let pu = dot(p, FACE_BASIS[face as usize][1]);
        let pv = dot(p, FACE_BASIS[face as usize][2]);
        let inv = if pn.abs() < 1e-30 { 0.0 } else { 1.0 / pn };
        let a = (pu * inv).atan();
        let b = (pv * inv).atan();
        let nf = self.n as f64;
        let fi = (a / std::f64::consts::FRAC_PI_2 + 0.5) * nf;
        let fj = (b / std::f64::consts::FRAC_PI_2 + 0.5) * nf;
        (face, fi, fj)
    }

    /// Eight neighbours in order E, NE, N, NW, W, SW, S, SE. [`NONE`] if absent.
    pub fn neighbors(&self, cell: u32) -> [u32; 8] {
        let (face, i, j) = self.decode(cell);
        if is_interior(self.n, i, j) {
            let mut out = [NONE; 8];
            for k in 0..8 {
                let ii = (i as i32 + NEIGHBOR_DI[k]) as u32;
                let jj = (j as i32 + NEIGHBOR_DJ[k]) as u32;
                out[k] = self.id(face, ii, jj);
            }
            out
        } else {
            self.tables.border_nb[self.border_index(face, i, j)]
        }
    }

    /// Great-circle distance in metres to neighbour `k`. `0` when the neighbour is absent.
    pub fn neighbor_distance(&self, cell: u32, k: usize) -> f32 {
        let (face, i, j) = self.decode(cell);
        let radians = if is_interior(self.n, i, j) {
            let idx = k * self.nn as usize + (j * self.n + i) as usize;
            self.tables.dist_rad[idx]
        } else {
            self.tables.border_dist[self.border_index(face, i, j)][k]
        };
        (radians as f64 * self.radius_m) as f32
    }

    /// Cell area in square metres.
    pub fn area_m2(&self, cell: u32) -> f64 {
        let (_face, i, j) = self.decode(cell);
        let sr = self.tables.area_sr[(j * self.n + i) as usize];
        sr * self.radius_m * self.radius_m
    }

    /// Parent cell on the grid of resolution `n/2`. `self` is the fine grid.
    pub fn parent_id(&self, cell: u32) -> u32 {
        let (face, i, j) = self.decode(cell);
        let pn = self.n / 2;
        face * pn * pn + (j / 2) * pn + (i / 2)
    }

    /// Four children on the grid of resolution `2n`, order `(2i,2j), (2i+1,2j), (2i,2j+1), (2i+1,2j+1)`.
    pub fn child_ids(&self, cell: u32) -> [u32; 4] {
        let (face, i, j) = self.decode(cell);
        let cn = self.n * 2;
        let ci = i * 2;
        let cj = j * 2;
        let id = |ii: u32, jj: u32| face * cn * cn + jj * cn + ii;
        [
            id(ci, cj),
            id(ci + 1, cj),
            id(ci, cj + 1),
            id(ci + 1, cj + 1),
        ]
    }

    /// Bilinear sample. Face borders clamp to the border cell of the owning face.
    pub fn sample_bilinear(&self, field: &[f32], p: [f64; 3]) -> f32 {
        let (face, fx, fy) = self.face_coords(p);
        self.sample_face(field, None, face, fx, fy).unwrap_or(0.0)
    }

    /// Nearest cell sample.
    pub fn sample_nearest(&self, field: &[f32], p: [f64; 3]) -> f32 {
        field[self.cell_at(p) as usize]
    }

    /// Bilinear sample that ignores parents where `mask` is `0`.
    /// `None` when every contributing parent is masked out.
    pub fn sample_face_masked(
        &self,
        field: &[f32],
        mask: &[u8],
        face: u32,
        fx: f64,
        fy: f64,
    ) -> Option<f32> {
        self.sample_face(field, Some(mask), face, fx, fy)
    }

    fn sample_face(
        &self,
        field: &[f32],
        mask: Option<&[u8]>,
        face: u32,
        fx: f64,
        fy: f64,
    ) -> Option<f32> {
        let n = self.n as i32;
        let u = fx - 0.5;
        let v = fy - 0.5;
        let i0f = u.floor();
        let j0f = v.floor();
        let tx = u - i0f;
        let ty = v - j0f;
        let mut i0 = i0f as i32;
        let mut j0 = j0f as i32;
        let mut i1 = i0 + 1;
        let mut j1 = j0 + 1;
        i0 = i0.clamp(0, n - 1);
        j0 = j0.clamp(0, n - 1);
        i1 = i1.clamp(0, n - 1);
        j1 = j1.clamp(0, n - 1);
        let sample = |i: i32, j: i32| {
            let id = self.id(face, i as u32, j as u32) as usize;
            let w = match mask {
                Some(m) if m[id] == 0 => 0.0,
                _ => 1.0,
            };
            (field[id] as f64, w)
        };
        let (a, wa) = sample(i0, j0);
        let (b, wb) = sample(i1, j0);
        let (c, wc) = sample(i0, j1);
        let (d, wd) = sample(i1, j1);
        let w00 = (1.0 - tx) * (1.0 - ty) * wa;
        let w10 = tx * (1.0 - ty) * wb;
        let w01 = (1.0 - tx) * ty * wc;
        let w11 = tx * ty * wd;
        let wsum = w00 + w10 + w01 + w11;
        if wsum == 0.0 {
            None
        } else {
            Some(((a * w00 + b * w10 + c * w01 + d * w11) / wsum) as f32)
        }
    }

    fn border_index(&self, face: u32, i: u32, j: u32) -> usize {
        let slot = border_slot(self.n, i, j).unwrap_or(0);
        (face * border_count(self.n) + slot) as usize
    }
}

/// Bilinear upsample from `parent` onto `child` (`child.n == 2 * parent.n`).
///
/// Weights inside a face are 0.75/0.25. Coordinates that fall outside the
/// parent face clamp to that face's border cell. Cross-face blending is
/// intentionally not done.
pub fn upsample(
    parent: &CubeSphere,
    parent_field: &[f32],
    child: &CubeSphere,
) -> Result<Vec<f32>, String> {
    if child.n != parent.n * 2 {
        return Err(format!(
            "upsample requires child.n = 2 * parent.n ({} vs {})",
            child.n, parent.n
        ));
    }
    if parent_field.len() != parent.cell_count() {
        return Err("upsample field length does not match parent grid".to_string());
    }
    let mut out = vec![0.0f32; child.cell_count()];
    out.par_iter_mut().enumerate().for_each(|(cell, dest)| {
        let (face, i, j) = child.decode(cell as u32);
        let fx = (i as f64 + 0.5) / 2.0;
        let fy = (j as f64 + 0.5) / 2.0;
        *dest = parent
            .sample_face(parent_field, None, face, fx, fy)
            .unwrap_or(0.0);
    });
    Ok(out)
}

/// Area-weighted mean of the four children. `child.n` must be `2 * parent.n`.
pub fn downsample(
    child: &CubeSphere,
    field: &[f32],
    parent: &CubeSphere,
) -> Result<Vec<f32>, String> {
    if child.n != parent.n * 2 {
        return Err(format!(
            "downsample requires child.n = 2 * parent.n ({} vs {})",
            child.n, parent.n
        ));
    }
    if field.len() != child.cell_count() {
        return Err("downsample field length does not match child grid".to_string());
    }
    let mut out = vec![0.0f32; parent.cell_count()];
    out.par_iter_mut().enumerate().for_each(|(cell, dest)| {
        let mut wsum = 0.0f64;
        let mut vsum = 0.0f64;
        for kid in parent.child_ids(cell as u32) {
            let area = child.area_m2(kid);
            wsum += area;
            vsum += area * f64::from(field[kid as usize]);
        }
        *dest = if wsum == 0.0 {
            0.0
        } else {
            (vsum / wsum) as f32
        };
    });
    Ok(out)
}

/// Resample `field` from `src` onto `dst`. Resolutions must differ by a power of two.
/// Upsampling repeats bilinear doubling. Downsampling repeats the four-child mean.
pub fn resample(src: &CubeSphere, field: &[f32], dst: &CubeSphere) -> Result<Vec<f32>, String> {
    if src.n == dst.n {
        if field.len() != src.cell_count() {
            return Err("resample field length mismatch".to_string());
        }
        return Ok(field.to_vec());
    }
    if src.n > dst.n {
        if !src.n.is_multiple_of(dst.n) {
            return Err("resample resolutions are not nested".to_string());
        }
        let mut cur_grid = src.clone();
        let mut cur = field.to_vec();
        while cur_grid.n > dst.n {
            let next = CubeSphere::new(cur_grid.n / 2, cur_grid.radius_m)?;
            cur = downsample(&cur_grid, &cur, &next)?;
            cur_grid = next;
        }
        Ok(cur)
    } else {
        if !dst.n.is_multiple_of(src.n) {
            return Err("resample resolutions are not nested".to_string());
        }
        let mut cur_grid = src.clone();
        let mut cur = field.to_vec();
        while cur_grid.n < dst.n {
            let next = CubeSphere::new(cur_grid.n * 2, cur_grid.radius_m)?;
            cur = upsample(&cur_grid, &cur, &next)?;
            cur_grid = next;
        }
        Ok(cur)
    }
}

/// Axis neighbour slots: E, N, W, S.
pub fn axis_neighbor_slots() -> [usize; 4] {
    AXIS_NEIGHBORS
}

fn is_interior(n: u32, i: u32, j: u32) -> bool {
    i >= 1 && j >= 1 && i + 1 < n && j + 1 < n
}

fn border_count(n: u32) -> u32 {
    4 * n - 4
}

fn border_slot(n: u32, i: u32, j: u32) -> Option<u32> {
    if is_interior(n, i, j) {
        return None;
    }
    if j == 0 {
        return Some(i);
    }
    if j + 1 == n {
        return Some(n + i);
    }
    if i == 0 {
        return Some(2 * n + (j - 1));
    }
    Some(2 * n + (n - 2) + (j - 1))
}

fn build_tables(n: u32) -> GridTables {
    let nn = (n * n) as usize;
    let mut area_sr = vec![0.0f64; nn];
    for j in 0..n {
        for i in 0..n {
            area_sr[(j * n + i) as usize] = cell_solid_angle(n, i, j);
        }
    }
    let mut dist_rad = vec![0.0f32; 8 * nn];
    if n >= 3 {
        for j in 1..n - 1 {
            for i in 1..n - 1 {
                let p = unit_on_face(0, n, i, j);
                let local = (j * n + i) as usize;
                for k in 0..8 {
                    let ii = (i as i32 + NEIGHBOR_DI[k]) as u32;
                    let jj = (j as i32 + NEIGHBOR_DJ[k]) as u32;
                    let q = unit_on_face(0, n, ii, jj);
                    dist_rad[k * nn + local] = angular_sep(p, q) as f32;
                }
            }
        }
    }
    let bcount = border_count(n) as usize;
    let mut border_nb = vec![[NONE; 8]; 6 * bcount];
    let mut border_dist = vec![[0.0f32; 8]; 6 * bcount];
    for face in 0..6u32 {
        for j in 0..n {
            for i in 0..n {
                let Some(slot) = border_slot(n, i, j) else {
                    continue;
                };
                let idx = face as usize * bcount + slot as usize;
                let self_id = face * n * n + j * n + i;
                let p = unit_on_face(face, n, i, j);
                let mut nbs = [NONE; 8];
                let mut dists = [0.0f32; 8];
                for k in 0..8 {
                    let ni = i as i32 + NEIGHBOR_DI[k];
                    let nj = j as i32 + NEIGHBOR_DJ[k];
                    let nid = if (0..n as i32).contains(&ni) && (0..n as i32).contains(&nj) {
                        face * n * n + (nj as u32) * n + (ni as u32)
                    } else {
                        extrapolated_cell(n, face, ni, nj)
                    };
                    if nid == self_id || nbs[..k].contains(&nid) {
                        nbs[k] = NONE;
                    } else {
                        nbs[k] = nid;
                        let (nf, ii, jj) = decode_n(n, nid);
                        let q = unit_on_face(nf, n, ii, jj);
                        dists[k] = angular_sep(p, q) as f32;
                    }
                }
                border_nb[idx] = nbs;
                border_dist[idx] = dists;
            }
        }
    }
    GridTables {
        area_sr,
        dist_rad,
        border_nb,
        border_dist,
    }
}

fn extrapolated_cell(n: u32, face: u32, i: i32, j: i32) -> u32 {
    let nf = n as f64;
    let a = ((i as f64 + 0.5) / nf - 0.5) * std::f64::consts::FRAC_PI_2;
    let b = ((j as f64 + 0.5) / nf - 0.5) * std::f64::consts::FRAC_PI_2;
    let basis = FACE_BASIS[face as usize];
    let ta = a.tan();
    let tb = b.tan();
    let mut q = [0.0; 3];
    for c in 0..3 {
        q[c] = basis[0][c] + ta * basis[1][c] + tb * basis[2][c];
    }
    cell_at_n(n, normalize(q))
}

fn cell_solid_angle(n: u32, i: u32, j: u32) -> f64 {
    let nf = n as f64;
    let a0 = (i as f64 / nf - 0.5) * std::f64::consts::FRAC_PI_2;
    let a1 = ((i as f64 + 1.0) / nf - 0.5) * std::f64::consts::FRAC_PI_2;
    let b0 = (j as f64 / nf - 0.5) * std::f64::consts::FRAC_PI_2;
    let b1 = ((j as f64 + 1.0) / nf - 0.5) * std::f64::consts::FRAC_PI_2;
    let x0 = a0.tan();
    let x1 = a1.tan();
    let y0 = b0.tan();
    let y1 = b1.tan();
    omega(x1, y1) - omega(x0, y1) - omega(x1, y0) + omega(x0, y0)
}

fn omega(x: f64, y: f64) -> f64 {
    (x * y / (1.0 + x * x + y * y).sqrt()).atan()
}

fn unit_on_face(face: u32, n: u32, i: u32, j: u32) -> [f64; 3] {
    let nf = n as f64;
    let a = ((i as f64 + 0.5) / nf - 0.5) * std::f64::consts::FRAC_PI_2;
    let b = ((j as f64 + 0.5) / nf - 0.5) * std::f64::consts::FRAC_PI_2;
    let basis = FACE_BASIS[face as usize];
    let ta = a.tan();
    let tb = b.tan();
    let mut p = [0.0; 3];
    for c in 0..3 {
        p[c] = basis[0][c] + ta * basis[1][c] + tb * basis[2][c];
    }
    normalize(p)
}

fn cell_at_n(n: u32, p: [f64; 3]) -> u32 {
    let face = face_of(p);
    let pn = dot(p, FACE_BASIS[face as usize][0]);
    let pu = dot(p, FACE_BASIS[face as usize][1]);
    let pv = dot(p, FACE_BASIS[face as usize][2]);
    let inv = if pn.abs() < 1e-30 { 0.0 } else { 1.0 / pn };
    let a = (pu * inv).atan();
    let b = (pv * inv).atan();
    let nf = n as f64;
    let mut i = ((a / std::f64::consts::FRAC_PI_2 + 0.5) * nf).floor() as i32;
    let mut j = ((b / std::f64::consts::FRAC_PI_2 + 0.5) * nf).floor() as i32;
    let last = n as i32 - 1;
    i = i.clamp(0, last);
    j = j.clamp(0, last);
    face * n * n + (j as u32) * n + (i as u32)
}

fn face_of(p: [f64; 3]) -> u32 {
    let ax = p[0].abs();
    let ay = p[1].abs();
    let az = p[2].abs();
    if ax >= ay && ax >= az {
        if p[0] >= 0.0 {
            0
        } else {
            1
        }
    } else if ay >= az {
        if p[1] >= 0.0 {
            2
        } else {
            3
        }
    } else if p[2] >= 0.0 {
        4
    } else {
        5
    }
}

fn decode_n(n: u32, cell: u32) -> (u32, u32, u32) {
    let nn = n * n;
    let face = cell / nn;
    let rem = cell % nn;
    (face, rem % n, rem / n)
}

#[inline]
fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

pub(crate) fn normalize(mut p: [f64; 3]) -> [f64; 3] {
    let m = (p[0] * p[0] + p[1] * p[1] + p[2] * p[2]).sqrt();
    if m == 0.0 {
        return [1.0, 0.0, 0.0];
    }
    p[0] /= m;
    p[1] /= m;
    p[2] /= m;
    p
}

fn angular_sep(a: [f64; 3], b: [f64; 3]) -> f64 {
    let c = cross(a, b);
    let s = (c[0] * c[0] + c[1] * c[1] + c[2] * c[2]).sqrt();
    let d = dot(a, b).clamp(-1.0, 1.0);
    s.atan2(d)
}

pub(crate) fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

pub(crate) fn dot3(a: [f64; 3], b: [f64; 3]) -> f64 {
    dot(a, b)
}
